//! Stairs laid out tread by tread: an L or a U turning on winders (kosé
//! stupne), and spirals. A straight flight, or one turning on a landing,
//! is laid out in flights by [`Stair::layout`].
//!
//! The walking line carries the treads at equal goings. The nosing lines
//! run through it and through the inner edge; across a turn their inner
//! ends are spread evenly along the inner edge (balanced winders), as far
//! either side of the turn as it takes to give each winder its narrowest
//! allowed width there.

use glam::{DVec2, DVec3};

use crate::members::{riser_count, Flight, Stair, StairLayout, StairShape};

/// A stair laid out tread by tread.
#[derive(Clone, Debug, PartialEq)]
pub struct StepPlan {
    pub risers: u32,
    pub riser: f64,
    /// The nosing lines (risers), first to last: inner end, outer end.
    pub nosings: Vec<[DVec2; 2]>,
    /// Tread `k` (1 ..= risers − 1) is `treads[k − 1]`, between nosings
    /// `k − 1` and `k`, its top `k · riser` above the base;
    /// counter-clockwise.
    pub treads: Vec<Vec<DVec2>>,
    /// Each tread's width at its inner end, along the inner edge.
    pub narrow: Vec<f64>,
    /// Each tread's going on the walking line.
    pub going: f64,
    pub inner: Vec<DVec2>,
    pub outer: Vec<DVec2>,
    pub walk: Vec<DVec2>,
    /// The turn goes left: the inner edge is the left one, looking up.
    pub left: bool,
}

/// A polyline measured along its length.
#[derive(Clone, Debug)]
pub(crate) struct Path {
    pts: Vec<DVec2>,
    acc: Vec<f64>,
}

impl Path {
    fn new(mut pts: Vec<DVec2>) -> Self {
        pts.dedup_by(|a, b| a.distance(*b) < 1e-9);
        let mut acc = vec![0.0];
        for w in pts.windows(2) {
            acc.push(acc.last().copied().unwrap_or(0.0) + w[0].distance(w[1]));
        }
        Self { pts, acc }
    }

    fn len(&self) -> f64 {
        self.acc.last().copied().unwrap_or(0.0)
    }

    fn seg(&self, t: f64) -> usize {
        let n = self.pts.len();
        if n < 2 {
            return 0;
        }
        match self.acc.iter().position(|a| *a > t) {
            Some(0) => 0,
            Some(i) => (i - 1).min(n - 2),
            None => n - 2,
        }
    }

    fn at(&self, t: f64) -> DVec2 {
        if self.pts.len() < 2 {
            return self.pts.first().copied().unwrap_or(DVec2::ZERO);
        }
        let i = self.seg(t);
        let (a, b) = (self.pts[i], self.pts[i + 1]);
        let l = (self.acc[i + 1] - self.acc[i]).max(1e-12);
        a.lerp(b, ((t - self.acc[i]) / l).clamp(0.0, 1.0))
    }

    fn tangent(&self, t: f64) -> DVec2 {
        if self.pts.len() < 2 {
            return DVec2::X;
        }
        let i = self.seg(t);
        (self.pts[i + 1] - self.pts[i]).normalize_or_zero()
    }

    /// The length along to the point nearest `p`.
    fn project(&self, p: DVec2) -> f64 {
        let mut best = (f64::INFINITY, 0.0);
        for i in 0..self.pts.len().saturating_sub(1) {
            let (a, b) = (self.pts[i], self.pts[i + 1]);
            let ab = b - a;
            let l2 = ab.length_squared().max(1e-18);
            let u = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
            let d = p.distance(a + ab * u);
            if d < best.0 - 1e-12 {
                best = (d, self.acc[i] + u * l2.sqrt());
            }
        }
        best.1
    }

    /// The corners strictly between lengths `a` and `b` (a < b).
    fn between(&self, a: f64, b: f64) -> Vec<DVec2> {
        self.between_at(a, b).into_iter().map(|(p, _)| p).collect()
    }

    /// The corners strictly between lengths `a` and `b`, with their
    /// lengths along.
    fn between_at(&self, a: f64, b: f64) -> Vec<(DVec2, f64)> {
        self.pts
            .iter()
            .zip(&self.acc)
            .filter(|(_, t)| **t > a + 1e-9 && **t < b - 1e-9)
            .map(|(p, t)| (*p, *t))
            .collect()
    }

    /// Where the ray from `o` along `dir` first meets the path past `o`:
    /// the length along. A path lying over itself in plan (a spiral of
    /// more than a turn) is met where its length is nearest `hint`.
    fn hit(&self, o: DVec2, dir: DVec2, hint: f64) -> Option<f64> {
        let mut hits: Vec<(f64, f64)> = Vec::new();
        for i in 0..self.pts.len().saturating_sub(1) {
            let (a, b) = (self.pts[i], self.pts[i + 1]);
            let e = b - a;
            let den = dir.perp_dot(e);
            if den.abs() < 1e-12 {
                continue;
            }
            let w = a - o;
            let lam = w.perp_dot(e) / den;
            let u = w.perp_dot(dir) / den;
            if lam > 1e-9 && (-1e-7..=1.0 + 1e-7).contains(&u) {
                hits.push((lam, self.acc[i] + u.clamp(0.0, 1.0) * e.length()));
            }
        }
        let near = hits.iter().map(|h| h.0).fold(f64::INFINITY, f64::min);
        hits.into_iter()
            .filter(|h| h.0 < near + 1e-6)
            .min_by(|a, b| (a.1 - hint).abs().total_cmp(&(b.1 - hint).abs()))
            .map(|h| h.1)
    }
}

/// Points along an arc about `c` of radius `r`, from `c + r·a` turning
/// toward `b` (unit, square to `a`) through `sweep` radians.
fn arc(c: DVec2, a: DVec2, b: DVec2, r: f64, sweep: f64) -> Vec<DVec2> {
    let n = ((sweep.abs() / (std::f64::consts::FRAC_PI_2 / 96.0)).ceil() as usize).max(1);
    (0..=n)
        .map(|i| {
            let th = sweep * i as f64 / n as f64;
            c + (a * th.cos() + b * th.sin()) * r
        })
        .collect()
}

/// The stair's guide lines: an edge `off` from the axis toward the turn
/// (negative: away from it) and the walking line, and where the turns
/// lie along the walking line.
struct Guide {
    walk: Path,
    turns: Vec<(f64, f64)>,
}

/// The line `off` from the axis (positive toward the turn) of a stair
/// whose walking line is `rw` from its inner edge, the walking line being
/// `total` long.
fn guide_line(s: &Stair, n: u32, off: f64, total: f64) -> (Vec<DVec2>, Vec<(f64, f64)>) {
    let w = s.width.max(0.1);
    let b = s.tread;
    let (after, left) = s.turn(n);
    let d = s.dir();
    let p = if left { d.perp() } else { -d.perp() };
    let rw = s.walk_from_inner();
    // distance of this line from the inner edge
    let q = w / 2.0 - off;
    let quarter = std::f64::consts::FRAC_PI_2;
    match s.form() {
        StairShape::Spiral => {
            let ri = s.well.max(0.0);
            let c = s.start + p * (ri + w / 2.0);
            let rwalk = (ri + rw).max(1e-3);
            let sweep = total / rwalk;
            (arc(c, -p, d, ri + q, sweep), vec![])
        }
        StairShape::L | StairShape::U => {
            let u_turn = s.form() == StairShape::U;
            let g = if u_turn { s.well.max(0.0) } else { 0.0 };
            let arcs = if u_turn { 2.0 } else { 1.0 } * quarter * rw + g;
            let l1 = ((after.max(1) - 1) as f64 * b).min((total - arcs).max(0.0));
            let l2 = (total - l1 - arcs).max(0.0);
            let c1 = s.start + d * l1 + p * (w / 2.0);
            let mut pts = vec![s.start + p * off];
            // the corner: rounded on the inner side of the walking line
            // (it is the walking line, or between it and the inner edge),
            // square outside it
            let round = q <= rw + 1e-9;
            let corner = |pts: &mut Vec<DVec2>, c: DVec2, a: DVec2, bb: DVec2| {
                if round {
                    pts.extend(arc(c, a, bb, q, quarter));
                } else {
                    pts.push(c + (a + bb) * q);
                }
            };
            corner(&mut pts, c1, -p, d);
            let (end_dir, last) = if u_turn {
                let c2 = c1 + p * g;
                corner(&mut pts, c2, d, p);
                (-d, c2 + p * q)
            } else {
                (p, c1 + d * q)
            };
            // past the turn, as far as the walking line goes on
            pts.push(last + end_dir * l2);
            let t0 = l1;
            (pts, vec![(t0, t0 + arcs)])
        }
        StairShape::Straight => {
            let pts = vec![s.start + p * off, s.start + p * off + d * total];
            (pts, vec![])
        }
    }
}

fn guide(s: &Stair, n: u32) -> Guide {
    let total = n.saturating_sub(1) as f64 * s.tread;
    let w = s.width.max(0.1);
    let rw = s.walk_from_inner();
    let (pts, turns) = guide_line(s, n, w / 2.0 - rw, total);
    Guide {
        walk: Path::new(pts),
        turns,
    }
}

/// An edge of the stair, `inset` in from it: the inner (toward the turn)
/// or the outer one.
fn edge(s: &Stair, n: u32, inner: bool, inset: f64) -> Path {
    let total = n.saturating_sub(1) as f64 * s.tread;
    let w = s.width.max(0.1);
    let off = if inner {
        w / 2.0 - inset
    } else {
        -(w / 2.0 - inset)
    };
    Path::new(guide_line(s, n, off, total).0)
}

impl Stair {
    /// The tread-by-tread layout, for a stair that [`Stair::by_steps`].
    pub fn step_plan(&self, rise: f64) -> StepPlan {
        let n = riser_count(rise, self.max_riser).max(2);
        let r = rise / n as f64;
        let b = self.tread.max(0.05);
        let (_, left) = self.turn(n);
        let g = guide(self, n);
        let inner = edge(self, n, true, 0.0);
        let outer = edge(self, n, false, 0.0);
        let total = g.walk.len();
        let s_k: Vec<f64> = (0..n).map(|k| (k as f64 * b).min(total)).collect();
        // a spiral's inner edge goes round with the walking line (and
        // may lie over itself): along it in proportion
        let spiral = self.form() == StairShape::Spiral;
        let k_in = inner.len() / total.max(1e-9);
        let k_out = outer.len() / total.max(1e-9);
        let perp: Vec<f64> = s_k
            .iter()
            .map(|s| {
                if spiral {
                    s * k_in
                } else {
                    inner.project(g.walk.at(*s))
                }
            })
            .collect();
        let mut t_k = perp.clone();
        // the winders: spread their inner ends evenly over the turn and as
        // many treads either side as the narrowest width needs
        let want = self.narrow.unwrap_or(b / 2.0).min(b);
        for &(za, zb) in &g.turns {
            let tp = |s: f64| inner.project(g.walk.at(s.clamp(0.0, total)));
            for e in 0..n {
                let a = (za - e as f64 * b).max(0.0);
                let z = (zb + e as f64 * b).min(total);
                let (ta, tb) = (tp(a), tp(z));
                let inside: Vec<usize> = (0..n as usize)
                    .filter(|k| s_k[*k] > a - 1e-9 && s_k[*k] < z + 1e-9)
                    .collect();
                for &k in &inside {
                    t_k[k] = ta + (s_k[k] - a) / (z - a).max(1e-9) * (tb - ta);
                }
                let step = (tb - ta) * b / (z - a).max(1e-9);
                if step >= want - 1e-9 || (a <= 1e-9 && z >= total - 1e-9) {
                    break;
                }
                // undo for the next, wider try
                for &k in &inside {
                    t_k[k] = perp[k];
                }
            }
        }
        let mut nosings = Vec::with_capacity(n as usize);
        let mut u_k = Vec::with_capacity(n as usize);
        for k in 0..n as usize {
            let i = inner.at(t_k[k]);
            let wp = g.walk.at(s_k[k]);
            let tan = g.walk.tangent(s_k[k]);
            let out_dir = if left { -tan.perp() } else { tan.perp() };
            let dir = if wp.distance(i) > 1e-6 {
                (wp - i).normalize()
            } else {
                out_dir
            };
            let u = outer
                .hit(i, dir, s_k[k] * k_out)
                .unwrap_or_else(|| outer.project(i + dir * self.width));
            nosings.push([i, outer.at(u)]);
            u_k.push(u);
        }
        let mut treads = Vec::new();
        let mut narrow = Vec::new();
        for k in 1..n as usize {
            let (ta, tb) = (t_k[k - 1], t_k[k]);
            let (ua, ub) = (u_k[k - 1], u_k[k]);
            let mut poly = vec![inner.at(ta)];
            poly.extend(inner.between(ta, tb));
            poly.push(inner.at(tb));
            poly.push(outer.at(ub));
            let mut back = outer.between(ua, ub);
            back.reverse();
            poly.extend(back);
            poly.push(outer.at(ua));
            poly.dedup_by(|a, b| a.distance(*b) < 1e-9);
            if crate::polygon::signed_area2(&poly) < 0.0 {
                poly.reverse();
            }
            treads.push(poly);
            narrow.push(tb - ta);
        }
        let t_end = t_k.last().copied().unwrap_or(0.0);
        let u_end = u_k.last().copied().unwrap_or(0.0);
        let cut = |p: &Path, t: f64| {
            let mut v = vec![p.at(0.0)];
            v.extend(p.between(0.0, t));
            v.push(p.at(t));
            v
        };
        StepPlan {
            risers: n,
            riser: r,
            nosings,
            treads,
            narrow,
            going: b,
            inner: cut(&inner, t_end),
            outer: cut(&outer, u_end),
            walk: g.walk.pts.clone(),
            left,
        }
    }

    /// For a stair laid out by steps: a stand-in layout whose first flight
    /// starts it and whose last ends it, for what needs only its ends.
    pub(crate) fn step_layout(&self, rise: f64) -> StairLayout {
        let sp = self.step_plan(rise);
        let walk = Path::new(sp.walk.clone());
        let mid = |k: usize| {
            let [a, b] = sp.nosings[k];
            (a + b) / 2.0
        };
        let n = sp.risers;
        let first = Flight {
            start: mid(0),
            dir: walk.tangent(0.0),
            risers: 1,
            z0: 0.0,
            length: None,
        };
        let last = Flight {
            start: mid(n as usize - 1),
            dir: walk.tangent(walk.len()),
            risers: 1,
            z0: (n - 1) as f64 * sp.riser,
            length: None,
        };
        StairLayout {
            risers: n,
            riser: sp.riser,
            flights: vec![first, last],
            landing: None,
        }
    }

    /// A stair laid out by steps: the path along one side over the
    /// nosings, `inset` in from the edge (heights above the base).
    pub(crate) fn step_side_path(&self, rise: f64, left: bool, inset: f64) -> Vec<DVec3> {
        let sp = self.step_plan(rise);
        let n = sp.risers as usize;
        let inner_side = left == sp.left;
        let e = edge(self, sp.risers, inner_side, inset);
        let k_e = e.len() / Path::new(sp.walk.clone()).len().max(1e-9);
        // where each nosing line meets this edge
        let mut at: Vec<(f64, f64)> = Vec::new();
        for k in 0..n {
            let [i, o] = sp.nosings[k];
            let (from, dir) = if inner_side {
                (o, (i - o).normalize_or_zero())
            } else {
                (i, (o - i).normalize_or_zero())
            };
            if let Some(t) = e.hit(from, dir, k as f64 * sp.going * k_e) {
                at.push((t, (k + 1) as f64 * sp.riser));
            }
        }
        let mut out = Vec::new();
        for w in at.windows(2) {
            let ((ta, za), (tb, zb)) = (w[0], w[1]);
            if out.is_empty() {
                out.push(e.at(ta).extend(za));
            }
            if tb > ta {
                for (p, t) in e.between_at(ta, tb) {
                    out.push(p.extend(za + (t - ta) / (tb - ta) * (zb - za)));
                }
            }
            out.push(e.at(tb).extend(zb));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::ElementId;
    use crate::members::Landing;

    fn stair(shape: StairShape, winders: bool) -> Stair {
        let mut s = Stair::new(ElementId::new(), DVec2::ZERO, 0.0);
        s.width = 1.0;
        s.tread = 0.25;
        s.max_riser = 0.2;
        s.height = Some(2.8);
        s.shape = shape;
        s.winders = winders;
        s.landing = Some(Landing {
            after: 5,
            left: true,
        });
        s
    }

    #[test]
    fn an_l_on_winders_keeps_the_going_on_its_walking_line_and_turns_a_quarter() {
        let s = stair(StairShape::L, true);
        assert!(s.by_steps());
        let sp = s.step_plan(2.8);
        assert_eq!(sp.risers, 14);
        assert_eq!(sp.nosings.len(), 14);
        assert_eq!(sp.treads.len(), 13);
        // the walking line is as long as the goings
        let walk = Path::new(sp.walk.clone());
        assert!((walk.len() - 13.0 * 0.25).abs() < 1e-4, "{}", walk.len());
        // the first nosing is square across, the last turned a quarter
        let [a, b] = sp.nosings[0];
        assert!((b - a).normalize().dot(DVec2::Y).abs() > 0.999);
        let [a, b] = sp.nosings[13];
        assert!(
            (b - a).normalize().dot(DVec2::X).abs() > 0.999,
            "{:?}",
            b - a
        );
        // every winder is at least half a going wide at its inner end
        for w in &sp.narrow {
            assert!(*w >= 0.125 - 1e-6, "{:?}", sp.narrow);
        }
        // each tread is a proper polygon
        for t in &sp.treads {
            assert!(crate::polygon::signed_area2(t) > 0.0);
        }
    }

    #[test]
    fn a_set_narrowest_width_spreads_the_winders_further() {
        let mut s = stair(StairShape::L, true);
        let wound = |sp: &StepPlan| {
            sp.narrow
                .iter()
                .filter(|w| (*w - 0.25).abs() > 1e-6)
                .count()
        };
        let before = wound(&s.step_plan(2.8));
        s.narrow = Some(0.15);
        let sp = s.step_plan(2.8);
        for w in &sp.narrow {
            assert!(*w >= 0.15 - 1e-6, "{:?}", sp.narrow);
        }
        assert!(wound(&sp) > before, "{before} {:?}", sp.narrow);
    }

    #[test]
    fn a_u_on_winders_comes_back_the_other_way_past_its_well() {
        let mut s = stair(StairShape::U, true);
        s.well = 0.2;
        let sp = s.step_plan(2.8);
        let [a, b] = sp.nosings[13];
        assert!((b - a).normalize().dot(DVec2::Y).abs() > 0.999);
        // the top is beside the bottom, past the well
        let mid = (a + b) / 2.0;
        assert!(mid.y > 1.0 + 0.2 - 1e-6, "{mid:?}");
    }

    #[test]
    fn a_spiral_turns_about_its_eye_with_radial_nosings() {
        let mut s = stair(StairShape::Spiral, false);
        s.well = 0.1;
        let sp = s.step_plan(2.8);
        let c = DVec2::new(0.0, 0.1 + 0.5);
        for [a, b] in &sp.nosings {
            assert!(
                (a.distance(c) - 0.1).abs() < 1e-4,
                "{a:?} {}",
                a.distance(c)
            );
            assert!((b.distance(c) - 1.1).abs() < 1e-4);
        }
    }

    #[test]
    fn the_side_path_climbs_over_every_nosing() {
        let s = stair(StairShape::L, true);
        let path = s.step_side_path(2.8, false, 0.05);
        assert!(path.len() >= 14);
        assert!(path.windows(2).all(|w| w[1].z >= w[0].z - 1e-9));
        assert!((path.last().unwrap().z - 2.8).abs() < 1e-6);
    }
}
