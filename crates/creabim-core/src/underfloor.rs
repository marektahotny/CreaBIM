//! Underfloor heating loops in a room, as PilonDesigner's
//! `floorheat.ts`: the pipe laid as a double spiral, the supply going
//! in along every other inset of the room's outline and the return
//! coming back along those between, so the two lie side by side at the
//! spacing; a room with more loops is cut into strips across its longer
//! side, each with its own spiral. A room can be laid as a meander or
//! a double meander instead, or not drawn; an edge zone (by a window or a
//! door) is laid denser, and a place of reduced output (under a bed with
//! no legs) gives less by its own share (George 2026-11-27).

use glam::DVec2;
use serde::{Deserialize, Serialize};

/// A room's underfloor heating, as designed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Underfloor {
    /// The pipes' spacing, m.
    pub spacing: f64,
    /// How many loops the room is laid in.
    pub loops: u32,
    /// The floor finish's thermal resistance RλB, m²K/W; none takes it
    /// from the room's floor finish.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finish_r: Option<f64>,
    /// How the pipe is laid in the plan.
    #[serde(default, skip_serializing_if = "Layout::is_default")]
    pub layout: Layout,
    /// Its edge zones and places of reduced output.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub zones: Vec<Zone>,
    /// Where its loops are split: across which way, and at which lines
    /// (x for a vertical split, y for a horizontal one, in the plan); none
    /// splits evenly across the longer side.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub split: Option<Split>,
    /// The doorway its loops come in by from the manifold; none takes
    /// the one on the shortest way.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub door: Option<crate::ElementId>,
}

/// How a room's loops are split, set by hand.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Split {
    /// Vertical lines (x), else horizontal ones (y).
    pub vertical: bool,
    /// The lines, as many as loops less one; fewer are spread evenly.
    #[serde(default)]
    pub at: Vec<f64>,
}

impl Default for Underfloor {
    fn default() -> Self {
        Self {
            spacing: 0.15,
            loops: 1,
            finish_r: None,
            layout: Layout::default(),
            zones: vec![],
            split: None,
            door: None,
        }
    }
}

/// How a room's pipe is laid.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layout {
    /// A double (bifilar) spiral, an edge zone laid in it.
    #[default]
    Spiral,
    /// A double spiral, the edge zone laid first, before the room.
    SpiralLeading,
    /// One pipe there and back across the room, the return along its edge.
    Meander,
    /// The supply and the return side by side, there and back.
    DoubleMeander,
    /// Designed, but not drawn in plans.
    Hidden,
}

impl Layout {
    pub const ALL: [Layout; 5] = [
        Layout::Hidden,
        Layout::Spiral,
        Layout::SpiralLeading,
        Layout::Meander,
        Layout::DoubleMeander,
    ];

    pub fn is_default(&self) -> bool {
        *self == Layout::Spiral
    }

    pub fn label(self) -> &'static str {
        match self {
            Layout::Hidden => "Not drawn",
            Layout::Spiral => "Spiral with an integrated edge zone",
            Layout::SpiralLeading => "Spiral with a leading edge zone",
            Layout::Meander => "Meander",
            Layout::DoubleMeander => "Double meander",
        }
    }
}

/// What a zone of a room's floor is.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ZoneKind {
    /// An edge zone laid at its own, closer spacing, m.
    Edge { spacing: f64 },
    /// A place of reduced output: what it gives less, %.
    Reduced { percent: f64 },
    /// A hole: the system board, but no pipe in it.
    Hole,
}

/// An edge zone or a place of reduced output, drawn in the plan.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    pub kind: ZoneKind,
    pub outline: Vec<DVec2>,
    /// Off, it is kept but neither drawn nor counted.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub on: bool,
    /// An edge zone set off from an edge drawn along the heated field
    /// (George 2026-11-28): the dashed line's ends at its width from the
    /// edge, and the point of the edge its arrow points at.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<[DVec2; 3]>,
}

fn yes() -> bool {
    true
}

fn is_true(v: &bool) -> bool {
    *v
}

impl Zone {
    pub fn area(&self) -> f64 {
        crate::polygon::signed_area2(&self.outline).abs() / 2.0
    }
}

impl Underfloor {
    /// Its zones that are on, of either kind.
    pub fn edges(&self) -> impl Iterator<Item = (&Zone, f64)> {
        self.zones
            .iter()
            .filter(|z| z.on)
            .filter_map(|z| match z.kind {
                ZoneKind::Edge { spacing } => Some((z, spacing)),
                _ => None,
            })
    }

    pub fn holes(&self) -> impl Iterator<Item = &Zone> {
        self.zones
            .iter()
            .filter(|z| z.on && matches!(z.kind, ZoneKind::Hole))
    }

    pub fn reduced(&self) -> impl Iterator<Item = (&Zone, f64)> {
        self.zones
            .iter()
            .filter(|z| z.on)
            .filter_map(|z| match z.kind {
                ZoneKind::Reduced { percent } => Some((z, percent)),
                _ => None,
            })
    }

    pub fn validate(&self) -> Result<(), String> {
        for z in &self.zones {
            if z.outline.len() < 3 {
                return Err("an underfloor heating zone needs three corners".into());
            }
            match z.kind {
                ZoneKind::Edge { spacing } if !(0.03..=0.5).contains(&spacing) => {
                    return Err("an edge zone's pipes are spaced 30 to 500 mm apart".into())
                }
                ZoneKind::Reduced { percent } if !(0.0..=100.0).contains(&percent) => {
                    return Err("a place of reduced output gives 0 to 100 % less".into())
                }
                _ => {}
            }
        }
        if !(self.spacing.is_finite() && (0.05..=0.5).contains(&self.spacing)) {
            return Err("underfloor pipes are spaced 50 to 500 mm apart".into());
        }
        if self.loops == 0 || self.loops > 50 {
            return Err("a room is laid in 1 to 50 loops".into());
        }
        Ok(())
    }
}

/// The edge zones `width` wide along the edge `line` drawn in the room
/// `room` (its outline), on the side the room lies (George 2026-11-28):
/// a piece a leg of the line, the line's corners mitred, each cut to
/// the room, with its dashed line and arrow ([`Zone::mark`]). `closed`
/// runs it round from the last point to the first too.
pub fn edge_zones(
    room: &[DVec2],
    line: &[DVec2],
    closed: bool,
    width: f64,
    spacing: f64,
) -> Vec<Zone> {
    let mut pts: Vec<DVec2> = line.to_vec();
    pts.dedup_by(|a, b| a.distance(*b) < 1e-6);
    if closed && pts.len() > 2 {
        pts.push(pts[0]);
    }
    let n = pts.len();
    if n < 2 || width <= 0.0 {
        return vec![];
    }
    // the side the room lies on, from the first leg's middle
    let leg = |i: usize| (pts[i + 1] - pts[i]).normalize_or_zero();
    let mid = (pts[0] + pts[1]) / 2.0;
    let left = leg(0).perp();
    let probe = 0.05_f64.min(width / 2.0);
    let side = if crate::polygon::contains(room, mid + left * probe) {
        1.0
    } else {
        -1.0
    };
    let normal = |i: usize| leg(i).perp() * side;
    // each point set off, mitred at the corners
    let off: Vec<DVec2> = (0..n)
        .map(|i| {
            let (a, b) = match (i, closed) {
                (0, true) => (normal(n - 2), normal(0)),
                (0, false) => (normal(0), normal(0)),
                (i, true) if i == n - 1 => (normal(n - 2), normal(0)),
                (i, false) if i == n - 1 => (normal(n - 2), normal(n - 2)),
                (i, _) => (normal(i - 1), normal(i)),
            };
            let m = a + b;
            let k = 1.0 + a.dot(b);
            let m = if k > 1e-6 { m / k } else { a };
            pts[i] + m * width
        })
        .collect();
    let mut out = vec![];
    for i in 0..n - 1 {
        let mut quad = vec![pts[i], pts[i + 1], off[i + 1], off[i]];
        if crate::polygon::signed_area2(&quad) < 0.0 {
            quad.reverse();
        }
        let cut = clip_convex(room, &quad);
        if cut.len() < 3 || crate::polygon::signed_area2(&cut).abs() < 1e-6 {
            continue;
        }
        out.push(Zone {
            kind: ZoneKind::Edge { spacing },
            outline: cut,
            on: true,
            mark: Some([off[i], off[i + 1], (pts[i] + pts[i + 1]) / 2.0]),
        });
    }
    out
}

/// `subject` cut to the convex, counter-clockwise `clip`
/// (Sutherland–Hodgman).
fn clip_convex(subject: &[DVec2], clip: &[DVec2]) -> Vec<DVec2> {
    let mut out: Vec<DVec2> = subject.to_vec();
    if crate::polygon::signed_area2(&out) < 0.0 {
        out.reverse();
    }
    for i in 0..clip.len() {
        let (a, b) = (clip[i], clip[(i + 1) % clip.len()]);
        let inside = |p: DVec2| (b - a).perp_dot(p - a) >= -1e-12;
        let input = std::mem::take(&mut out);
        let Some(&last) = input.last() else {
            return vec![];
        };
        let mut s = last;
        for &e in &input {
            let cross = |p: DVec2, q: DVec2| {
                let d = q - p;
                let den = (b - a).perp_dot(d);
                if den.abs() < 1e-15 {
                    q
                } else {
                    p + d * ((b - a).perp_dot(a - p) / den).clamp(0.0, 1.0)
                }
            };
            match (inside(e), inside(s)) {
                (true, true) => out.push(e),
                (true, false) => {
                    out.push(cross(s, e));
                    out.push(e);
                }
                (false, true) => out.push(cross(s, e)),
                _ => {}
            }
            s = e;
        }
    }
    out.dedup_by(|a, b| a.distance(*b) < 1e-9);
    out
}

/// The edge strip left free along the walls, m.
pub const EDGE: f64 = 0.1;

/// The outline moved in by `d` (its edges' crossings); none once an
/// edge is lost on the way.
pub fn inset_ring(o: &[DVec2], d: f64) -> Option<Vec<DVec2>> {
    let n = o.len();
    if n < 3 {
        return None;
    }
    let sg = if crate::polygon::signed_area2(o) >= 0.0 {
        1.0
    } else {
        -1.0
    };
    let es: Vec<(DVec2, DVec2)> = (0..n)
        .map(|i| {
            let (a, b) = (o[i], o[(i + 1) % n]);
            let u = (b - a).normalize_or(DVec2::X);
            (u, a + DVec2::new(-u.y, u.x) * sg * d)
        })
        .collect();
    let vs: Vec<DVec2> = (0..n)
        .map(|i| {
            let (pu, pq) = es[(i + n - 1) % n];
            let (u, q) = es[i];
            let den = pu.perp_dot(u);
            if den.abs() < 1e-9 {
                return q;
            }
            pq + pu * ((q - pq).perp_dot(u) / den)
        })
        .collect();
    for i in 0..n {
        if (vs[(i + 1) % n] - vs[i]).dot(es[i].0) < 0.02 {
            return None;
        }
    }
    Some(vs)
}

/// A loop laid: the supply and the return as polylines, and the pipe's
/// length, m.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Spiral {
    pub supply: Vec<DVec2>,
    pub ret: Vec<DVec2>,
    pub length: f64,
    /// More pieces of it: where an edge zone cuts it, and the edge zone's
    /// own; `true` for the supply.
    pub more: Vec<(Vec<DVec2>, bool)>,
}

impl Spiral {
    /// Every piece of it, `true` for the supply.
    pub fn pieces(&self) -> impl Iterator<Item = (&[DVec2], bool)> {
        [(&self.supply[..], true), (&self.ret[..], false)]
            .into_iter()
            .chain(self.more.iter().map(|(p, s)| (&p[..], *s)))
            .filter(|(p, _)| p.len() > 1)
    }
}

/// The double spiral in `outline` at spacing `t`, `edge` off the walls,
/// setting out from corner `start`. In a room that is not convex (an L)
/// the rings follow its whole outline until its narrower part is filled,
/// then go on round its largest convex part: one coil (George
/// 2026-11-28: as many coils drawn as there are loops).
pub fn spiral(outline: &[DVec2], t: f64, edge: f64, start: usize) -> Spiral {
    let n = outline.len();
    if n < 3 || t <= 0.0 {
        return Spiral::default();
    }
    let o: Vec<DVec2> = (0..n).map(|i| outline[(i + start) % n]).collect();
    let mut rings: Vec<Vec<DVec2>> = vec![];
    let mut current = o.clone();
    for j in 0..400 {
        let d = edge + j as f64 * t;
        let ring = match inset_ring(&current, d) {
            Some(r) => Some(r),
            None => {
                // the narrower part filled: on round the largest convex
                // part it still has room in
                largest_convex(&current, 0)
                    .filter(|p| p.len() != current.len() || p != &current)
                    .and_then(|p| inset_ring(&p, d).map(|r| (p, r)))
                    .map(|(p, r)| {
                        current = p;
                        r
                    })
            }
        };
        let Some(mut r) = ring else { break };
        // each ring set out from the corner nearest the last one's
        if let Some(prev) = rings.last() {
            let k = (0..r.len())
                .min_by(|a, b| r[*a].distance(prev[0]).total_cmp(&r[*b].distance(prev[0])))
                .unwrap_or(0);
            r.rotate_left(k);
        }
        rings.push(r);
    }
    let path = |seq: Vec<&Vec<DVec2>>| -> Vec<DVec2> {
        let mut pts: Vec<DVec2> = vec![];
        for (k, r) in seq.iter().enumerate() {
            let m = r.len();
            let (last, v0) = (r[m - 1], r[0]);
            let l = last.distance(v0).max(1e-9);
            let u = (v0 - last) / l;
            if k == 0 {
                pts.push(v0);
            }
            pts.extend(r.iter().skip(1).copied());
            match seq.get(k + 1) {
                Some(nx) => {
                    // the last edge ends on the line of the next ring's first
                    let (a, b) = (nx[0], nx[1]);
                    let w = b - a;
                    let den = u.perp_dot(w);
                    let d = if den.abs() < 1e-9 {
                        l - 2.0 * t
                    } else {
                        (a - last).perp_dot(w) / den
                    };
                    let s = last + u * d;
                    pts.push(s);
                    if (s - a).dot(w) / w.length_squared().max(1e-12) < -1e-6 {
                        pts.push(a);
                    }
                }
                None => pts.push(last + u * (l - 2.0 * t).max(l / 2.0)),
            }
        }
        pts
    };
    let mut supply = path(rings.iter().step_by(2).collect());
    let ret = path(rings.iter().skip(1).step_by(2).collect());
    if let (false, Some(&end)) = (supply.is_empty(), ret.last()) {
        supply.push(end);
    }
    let len = |q: &[DVec2]| q.windows(2).map(|w| w[0].distance(w[1])).sum::<f64>();
    Spiral {
        length: len(&supply) + len(&ret),
        supply,
        ret,
        more: vec![],
    }
}

/// `poly` with the small steps of its outline (a wall's plaster or a
/// centimetre's jog, shorter than `tol`) taken out, so they neither make
/// a convex part of their own nor stop a ring.
pub fn tidy(poly: &[DVec2], tol: f64) -> Vec<DVec2> {
    let mut out = clean(poly.to_vec());
    for _ in 0..64 {
        let m = out.len();
        if m <= 4 {
            break;
        }
        // a short edge: its ends become one point
        if let Some(k) = (0..m).find(|&k| out[k].distance(out[(k + 1) % m]) < tol) {
            let mid = (out[k] + out[(k + 1) % m]) / 2.0;
            out[k] = mid;
            out.remove((k + 1) % m);
            continue;
        }
        // a corner hardly off the line of its neighbours
        let off = (0..m).find(|&k| {
            let (p, c, q) = (out[(k + m - 1) % m], out[k], out[(k + 1) % m]);
            let w = q - p;
            w.length() > 1e-9 && (c - p).perp_dot(w).abs() / w.length() < tol / 4.0
        });
        match off {
            Some(k) => {
                out.remove(k);
            }
            None => break,
        }
    }
    out
}

fn plen(q: &[DVec2]) -> f64 {
    q.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// Where line y = `y` (along axis `ax`'s other) crosses `poly`: the
/// first and the last crossing along `ax`.
fn span(poly: &[DVec2], ax: usize, y: f64) -> Option<(f64, f64)> {
    let o = 1 - ax;
    let mut xs: Vec<f64> = vec![];
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        if (a[o] - y) * (b[o] - y) <= 0.0 && (a[o] - b[o]).abs() > 1e-12 {
            xs.push(a[ax] + (b[ax] - a[ax]) * (y - a[o]) / (b[o] - a[o]));
        }
    }
    let lo = xs.iter().copied().fold(f64::MAX, f64::min);
    let hi = xs.iter().copied().fold(f64::MIN, f64::max);
    (hi - lo > 1e-6).then_some((lo, hi))
}

fn pt(ax: usize, x: f64, y: f64) -> DVec2 {
    if ax == 0 {
        DVec2::new(x, y)
    } else {
        DVec2::new(y, x)
    }
}

/// The lines across the inset outline at `t` apart, along its longer
/// side: (y, x from, x to), and the axis they run along.
/// The rows' count and their (position, start, end).
type Rows = (usize, Vec<(f64, f64, f64)>);

fn rows(outline: &[DVec2], t: f64, edge: f64, first: f64) -> Option<Rows> {
    let o = inset_ring(outline, edge)?;
    let (lo, hi) = o.iter().fold(
        (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN)),
        |(lo, hi), p| (lo.min(*p), hi.max(*p)),
    );
    let ax = usize::from(hi.x - lo.x < hi.y - lo.y);
    let oy = 1 - ax;
    let mut out = vec![];
    let mut y = lo[oy] + first;
    while y <= hi[oy] - 1e-6 && out.len() < 2000 {
        if let Some((a, b)) = span(&o, ax, y) {
            out.push((y, a, b));
        }
        y += t;
    }
    Some((ax, out))
}

/// One pipe there and back across `outline` at spacing `t`, `edge` off
/// the walls; the return runs back along the edge.
pub fn meander(outline: &[DVec2], t: f64, edge: f64) -> Spiral {
    meander_at(outline, t, edge, 0.0)
}

/// [`meander`] with its first row `first` in from the edge.
fn meander_at(outline: &[DVec2], t: f64, edge: f64, first: f64) -> Spiral {
    if outline.len() < 3 || t <= 0.0 {
        return Spiral::default();
    }
    let Some((ax, mut rs)) = rows(outline, t, edge, first) else {
        return Spiral::default();
    };
    // an even number of rows: it ends on the side it set out from
    if rs.len() % 2 == 1 && rs.len() > 1 {
        rs.pop();
    }
    if rs.is_empty() {
        return Spiral::default();
    }
    // the serpentine keeps a spacing clear of the return along the side
    let mut supply = vec![];
    for (k, (y, a, b)) in rs.iter().enumerate() {
        let (a, b) = (a + t, *b);
        if k % 2 == 0 {
            supply.extend([pt(ax, a, *y), pt(ax, b, *y)]);
        } else {
            supply.extend([pt(ax, b, *y), pt(ax, a, *y)]);
        }
    }
    let (y0, a0, _) = rs[0];
    let (y1, a1, _) = rs[rs.len() - 1];
    let last = *supply.last().unwrap();
    let ret = vec![last, pt(ax, a1.min(a0), y1), pt(ax, a1.min(a0), y0)];
    let ret = if (last - ret[1]).length() < 1e-9 {
        ret[1..].to_vec()
    } else {
        ret
    };
    Spiral {
        length: plen(&supply) + plen(&ret),
        supply,
        ret,
        more: vec![],
    }
}

/// The open polyline `p` moved sideways by `d` (left of its way), its
/// corners mitred.
fn offset(p: &[DVec2], d: f64) -> Vec<DVec2> {
    let n = p.len();
    if n < 2 {
        return p.to_vec();
    }
    let nrm = |a: DVec2, b: DVec2| {
        let u = (b - a).normalize_or(DVec2::X);
        DVec2::new(-u.y, u.x)
    };
    (0..n)
        .map(|i| {
            if i == 0 {
                p[0] + nrm(p[0], p[1]) * d
            } else if i == n - 1 {
                p[n - 1] + nrm(p[n - 2], p[n - 1]) * d
            } else {
                let (n0, n1) = (nrm(p[i - 1], p[i]), nrm(p[i], p[i + 1]));
                let m = (n0 + n1).normalize_or(n0);
                p[i] + m * (d / m.dot(n0).max(0.2))
            }
        })
        .collect()
}

/// The supply and the return side by side at `t`, there and back across
/// `outline` as one meander of pairs, `edge` off the walls.
pub fn double_meander(outline: &[DVec2], t: f64, edge: f64) -> Spiral {
    if outline.len() < 3 || t <= 0.0 {
        return Spiral::default();
    }
    // the pairs' middle lines, 2t apart, half a spacing off the edge
    let Some((ax, rs)) = rows(outline, 2.0 * t, edge + t / 2.0, 0.0) else {
        return Spiral::default();
    };
    if rs.is_empty() {
        return Spiral::default();
    }
    let mut mid = vec![];
    for (k, (y, a, b)) in rs.iter().enumerate() {
        if k % 2 == 0 {
            mid.extend([pt(ax, *a, *y), pt(ax, *b, *y)]);
        } else {
            mid.extend([pt(ax, *b, *y), pt(ax, *a, *y)]);
        }
    }
    let mut supply = offset(&mid, t / 2.0);
    let mut ret = offset(&mid, -t / 2.0);
    // the far end: the supply turns into the return
    if let Some(&end) = ret.last() {
        supply.push(end);
    }
    ret.reverse();
    Spiral {
        length: plen(&supply) + plen(&ret),
        supply,
        ret,
        more: vec![],
    }
}

/// `poly` with no repeated points, nor corners on a straight line (the
/// spikes a cut leaves along itself).
fn clean(mut out: Vec<DVec2>) -> Vec<DVec2> {
    loop {
        let m = out.len();
        if m < 3 {
            return out;
        }
        let k = (0..m).find(|&k| {
            let (p, c, q) = (out[(k + m - 1) % m], out[k], out[(k + 1) % m]);
            c.distance(q) < 1e-6 || (c - p).perp_dot(q - c).abs() < 1e-9
        });
        match k {
            Some(k) => {
                out.remove(k);
            }
            None => return out,
        }
    }
}

/// `poly` cut by the line through `a` along `d`: the parts left and
/// right of it.
fn halves(poly: &[DVec2], a: DVec2, d: DVec2) -> (Vec<DVec2>, Vec<DVec2>) {
    let side = |p: DVec2| d.perp_dot(p - a);
    let clip = |keep: f64| -> Vec<DVec2> {
        let mut out: Vec<DVec2> = vec![];
        for i in 0..poly.len() {
            let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
            let (sp, sq) = (side(p) * keep, side(q) * keep);
            if sp >= -1e-9 {
                out.push(p);
            }
            if (sp > 1e-9 && sq < -1e-9) || (sp < -1e-9 && sq > 1e-9) {
                out.push(p + (q - p) * (sp / (sp - sq)));
            }
        }
        clean(out)
    };
    (clip(1.0), clip(-1.0))
}

/// `poly` cut into convex parts at its inner corners (an L-shaped room
/// into two), so each can be laid by itself.
pub fn convex_parts(poly: &[DVec2]) -> Vec<Vec<DVec2>> {
    fn go(poly: Vec<DVec2>, depth: u32, out: &mut Vec<Vec<DVec2>>) {
        let n = poly.len();
        let area = crate::polygon::signed_area2(&poly);
        if n < 3 || area.abs() < 1e-6 {
            return;
        }
        let sg = area.signum();
        // an inner (reflex) corner
        let reflex = (0..n).find(|&i| {
            let (p, c, q) = (poly[(i + n - 1) % n], poly[i], poly[(i + 1) % n]);
            (c - p).perp_dot(q - c) * sg < -1e-9
        });
        let Some(i) = reflex.filter(|_| depth < 12) else {
            out.push(poly);
            return;
        };
        let (p, c) = (poly[(i + n - 1) % n], poly[i]);
        let (l, r) = halves(&poly, c, c - p);
        let big = |q: &Vec<DVec2>| q.len() >= 3 && crate::polygon::signed_area2(q).abs() > 1e-4;
        if !big(&l) || !big(&r) {
            out.push(poly);
            return;
        }
        go(l, depth + 1, out);
        go(r, depth + 1, out);
    }
    let mut out = vec![];
    go(clean(poly.to_vec()), 0, &mut out);
    if out.is_empty() {
        out.push(poly.to_vec());
    }
    out
}

/// The largest convex piece of `poly` cut off at its inner corners,
/// along either of their sides (of an L, its larger arm whole).
fn largest_convex(poly: &[DVec2], depth: u32) -> Option<Vec<DVec2>> {
    let poly = clean(poly.to_vec());
    let n = poly.len();
    let area = crate::polygon::signed_area2(&poly);
    if n < 3 || area.abs() < 1e-4 {
        return None;
    }
    let sg = area.signum();
    let reflex: Vec<usize> = (0..n)
        .filter(|&i| {
            let (p, c, q) = (poly[(i + n - 1) % n], poly[i], poly[(i + 1) % n]);
            (c - p).perp_dot(q - c) * sg < -1e-9
        })
        .collect();
    if reflex.is_empty() {
        return Some(poly);
    }
    if depth > 3 {
        return None;
    }
    let mut best: Option<Vec<DVec2>> = None;
    for &i in reflex.iter().take(4) {
        let (p, c, q) = (poly[(i + n - 1) % n], poly[i], poly[(i + 1) % n]);
        for dir in [c - p, q - c] {
            let (l, r) = halves(&poly, c, dir);
            for h in [l, r] {
                if h.len() < 3 || h.len() == n && h == poly {
                    continue;
                }
                if let Some(x) = largest_convex(&h, depth + 1) {
                    let a = crate::polygon::signed_area2(&x).abs();
                    if best
                        .as_ref()
                        .is_none_or(|b| a > crate::polygon::signed_area2(b).abs())
                    {
                        best = Some(x);
                    }
                }
            }
        }
    }
    best
}

/// The pieces of polyline `p` outside polygon `poly`.
pub fn outside(p: &[DVec2], poly: &[DVec2]) -> Vec<Vec<DVec2>> {
    let n = poly.len();
    let mut out: Vec<Vec<DVec2>> = vec![];
    let mut cur: Vec<DVec2> = vec![];
    for w in p.windows(2) {
        let (a, b) = (w[0], w[1]);
        let mut ts = vec![0.0, 1.0];
        for i in 0..n {
            let (c, d) = (poly[i], poly[(i + 1) % n]);
            let (r, s) = (b - a, d - c);
            let den = r.perp_dot(s);
            if den.abs() < 1e-12 {
                continue;
            }
            let u = (c - a).perp_dot(s) / den;
            let v = (c - a).perp_dot(r) / den;
            if (0.0..=1.0).contains(&u) && (0.0..=1.0).contains(&v) {
                ts.push(u);
            }
        }
        ts.sort_by(f64::total_cmp);
        for k in 0..ts.len() - 1 {
            let (t0, t1) = (ts[k], ts[k + 1]);
            if t1 - t0 < 1e-9 {
                continue;
            }
            let (x, y) = (a + (b - a) * t0, a + (b - a) * t1);
            if crate::polygon::contains(poly, (x + y) / 2.0) {
                if cur.len() > 1 {
                    out.push(std::mem::take(&mut cur));
                }
                cur.clear();
            } else {
                if cur.last().is_none_or(|l| l.distance(x) > 1e-9) {
                    if cur.len() > 1 {
                        out.push(std::mem::take(&mut cur));
                    }
                    cur = vec![x];
                }
                cur.push(y);
            }
        }
    }
    if cur.len() > 1 {
        out.push(cur);
    }
    out
}

/// The room cut into `n` strips across its longer side, a loop each.
pub fn zones(outline: &[DVec2], n: usize) -> Vec<Vec<DVec2>> {
    split_zones(outline, n, None)
}

/// The lines the room's `n` loops are split at, and which way they run
/// (`true`: vertical, x = c): those set by hand, the rest spread evenly.
pub fn split_lines(outline: &[DVec2], n: usize, split: Option<&Split>) -> (bool, Vec<f64>) {
    let (lo, hi) = outline.iter().fold(
        (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN)),
        |(lo, hi), p| (lo.min(*p), hi.max(*p)),
    );
    let vertical = split.map_or(hi.x - lo.x >= hi.y - lo.y, |s| s.vertical);
    let ax = usize::from(!vertical);
    if n <= 1 {
        return (vertical, vec![]);
    }
    let mut at: Vec<f64> = split
        .map(|s| {
            s.at.iter()
                .copied()
                .filter(|c| *c > lo[ax] + 0.05 && *c < hi[ax] - 0.05)
                .collect()
        })
        .unwrap_or_default();
    at.truncate(n - 1);
    at.sort_by(f64::total_cmp);
    if at.len() < n - 1 {
        // spread the rest over what is left after the last set
        let from = at.last().copied().unwrap_or(lo[ax]);
        let left = n - 1 - at.len();
        let step = (hi[ax] - from) / (left + 1) as f64;
        for k in 1..=left {
            at.push(from + k as f64 * step);
        }
    }
    (vertical, at)
}

/// The room cut into `n` strips at the split's lines, a loop each.
pub fn split_zones(outline: &[DVec2], n: usize, split: Option<&Split>) -> Vec<Vec<DVec2>> {
    if n <= 1 || outline.len() < 3 {
        return vec![outline.to_vec()];
    }
    let (vertical, lines) = split_lines(outline, n, split);
    let ax = usize::from(!vertical);
    // the half plane q[ax] ≥ c (keep 1) or ≤ c (keep −1)
    let clip = |poly: &[DVec2], c: f64, keep: f64| -> Vec<DVec2> {
        let mut out = vec![];
        for i in 0..poly.len() {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            let (ia, ib) = ((a[ax] - c) * keep >= -1e-9, (b[ax] - c) * keep >= -1e-9);
            if ia {
                out.push(a);
            }
            if ia != ib {
                out.push(a + (b - a) * ((c - a[ax]) / (b[ax] - a[ax])));
            }
        }
        out
    };
    let mut out: Vec<Vec<DVec2>> = vec![];
    for i in 0..n {
        let mut z = outline.to_vec();
        if i > 0 {
            z = clip(&z, lines[i - 1], 1.0);
        }
        if i < n - 1 {
            z = clip(&z, lines[i], -1.0);
        }
        let m = z.len();
        let z: Vec<DVec2> = (0..m)
            .filter(|&k| z[k].distance(z[(k + 1) % m]) > 1e-6)
            .map(|k| z[k])
            .collect();
        if z.len() >= 3 {
            out.push(z);
        }
    }
    if out.is_empty() {
        vec![outline.to_vec()]
    } else {
        out
    }
}

/// The loops of a room with outline `outline`, laid as designed: none
/// when it is not drawn; an edge zone's pipe laid at its own spacing, the
/// room's kept out of it.
pub fn loops(outline: &[DVec2], u: &Underfloor) -> Vec<Spiral> {
    loops_from(outline, u, None)
}

/// The loops, each spiral setting out from its corner nearest `from`
/// (where the pipes come in from the manifold).
pub fn loops_from(outline: &[DVec2], u: &Underfloor, from: Option<DVec2>) -> Vec<Spiral> {
    let tidied = tidy(outline, 0.05);
    let strips = split_zones(&tidied, u.loops.max(1) as usize, u.split.as_ref());
    // the holes along the walls (under kitchen units) taken off the field
    // the coil is laid in, so it goes round them as one pipe
    let holes: Vec<&[DVec2]> = u.holes().map(|z| &z.outline[..]).collect();
    let strips: Vec<Vec<DVec2>> = strips
        .into_iter()
        .map(|z| {
            if holes.is_empty() {
                return z;
            }
            square_less(&z, &holes)
                .filter(|r| r.len() >= 3)
                .unwrap_or(z)
        })
        .collect();
    let corner = |part: &[DVec2]| {
        from.map_or(0, |f| {
            (0..part.len())
                .min_by(|a, b| part[*a].distance(f).total_cmp(&part[*b].distance(f)))
                .unwrap_or(0)
        })
    };
    // a loop is one coil (George 2026-11-28); an L-shaped strip spiralled
    // whole, and only one that cannot be (a U) laid part by convex part,
    // so it is filled all over (George 2026-11-27)
    let lay = |z: &[DVec2], start: usize| -> Spiral {
        if matches!(u.layout, Layout::Spiral | Layout::SpiralLeading) {
            let sp = spiral(z, u.spacing, EDGE, start);
            let area = crate::polygon::signed_area2(z).abs() / 2.0;
            if sp.length >= 0.8 * area / u.spacing || convex_parts(z).len() < 2 {
                return sp;
            }
            // the rings cannot fill it (round a notch): its smaller parts
            // a serpentine each, then on into the coil over the largest,
            // still one pipe
            let mut parts = convex_parts(z);
            parts.sort_by(|a, b| {
                crate::polygon::signed_area2(b)
                    .abs()
                    .total_cmp(&crate::polygon::signed_area2(a).abs())
            });
            let main = parts.remove(0);
            let mut supply: Vec<DVec2> = vec![];
            let mut length = 0.0;
            let mut at = from;
            while !parts.is_empty() {
                // the nearest part next
                let k = (0..parts.len())
                    .min_by(|a, b| {
                        let d = |q: &Vec<DVec2>| {
                            at.map_or(0.0, |f| {
                                q.iter().map(|p| p.distance(f)).fold(f64::MAX, f64::min)
                            })
                        };
                        d(&parts[*a]).total_cmp(&d(&parts[*b]))
                    })
                    .unwrap_or(0);
                let p = parts.remove(k);
                let mut s = meander_at(&p, u.spacing, EDGE, u.spacing / 2.0).supply;
                if s.len() < 2 {
                    continue;
                }
                if let Some(f) = at {
                    if s[s.len() - 1].distance(f) < s[0].distance(f) {
                        s.reverse();
                    }
                }
                if let Some(&l) = supply.last() {
                    let c = square_corner(z, l, s[0]);
                    length += plen(&[l, c, s[0]]);
                    supply.push(c);
                }
                length += plen(&s);
                at = s.last().copied();
                supply.extend(s);
            }
            let coil = spiral(&main, u.spacing, EDGE, nearest(&main, at));
            if let (Some(&l), Some(&f)) = (supply.last(), coil.supply.first()) {
                let c = square_corner(z, l, f);
                length += plen(&[l, c, f]);
                supply.push(c);
            }
            length += coil.length;
            supply.extend(coil.supply);
            return Spiral {
                supply,
                ret: coil.ret,
                length,
                more: vec![],
            };
        }
        let mut all = Spiral::default();
        for (k, part) in convex_parts(z).iter().enumerate() {
            let sp = match u.layout {
                Layout::Spiral | Layout::SpiralLeading => {
                    spiral(part, u.spacing, EDGE, corner(part))
                }
                Layout::Meander => meander(part, u.spacing, EDGE),
                Layout::DoubleMeander => double_meander(part, u.spacing, EDGE),
                Layout::Hidden => Spiral::default(),
            };
            all.length += sp.length;
            if k == 0 {
                all.supply = sp.supply;
                all.ret = sp.ret;
            } else {
                all.more.push((sp.supply, true));
                all.more.push((sp.ret, false));
            }
        }
        all
    };
    // an edge zone set off from an edge (George 2026-11-28) runs through
    // every loop it lies in: each loop's coil laid closer there, the
    // loops as many as before
    let marked: Vec<(&Zone, f64)> = u.edges().filter(|(z, _)| z.mark.is_some()).collect();
    let through: Option<Vec<Spiral>> = (!marked.is_empty())
        .then(|| {
            strips
                .iter()
                .map(|z| loop_with_edges(z, &marked, u.spacing, from, &lay))
                .collect()
        })
        .flatten();
    let edges_in_loops = through.is_some();
    let mut out: Vec<Spiral> = match through {
        Some(v) => v,
        None => strips.iter().map(|z| lay(z, corner(z))).collect(),
    };
    // no pipe in a hole, nor the room's in an edge zone
    let cut: Vec<&Zone> = u
        .holes()
        .chain(
            u.edges()
                .filter(|e| !(edges_in_loops && e.0.mark.is_some()))
                .map(|e| e.0),
        )
        .collect();
    for z in cut {
        // the room's pipe stays out of the zone
        for sp in out.iter_mut() {
            let mut parts: Vec<(Vec<DVec2>, bool)> = vec![];
            for (p, s) in sp.pieces() {
                for q in outside(p, &z.outline) {
                    parts.push((q, s));
                }
            }
            let first = |s: bool, parts: &mut Vec<(Vec<DVec2>, bool)>| {
                parts
                    .iter()
                    .position(|x| x.1 == s)
                    .map(|i| parts.remove(i).0)
                    .unwrap_or_default()
            };
            sp.supply = first(true, &mut parts);
            sp.ret = first(false, &mut parts);
            sp.more = parts;
        }
    }
    for (z, t) in u
        .edges()
        .filter(|e| !(edges_in_loops && e.0.mark.is_some()))
    {
        // the zone's own, a meander at its spacing, in the loop it is in
        let m = meander(&z.outline, t, 0.0);
        let c = z.outline.iter().copied().sum::<DVec2>() / z.outline.len() as f64;
        let k = strips
            .iter()
            .position(|s| crate::polygon::contains(s, c))
            .unwrap_or(0);
        if let Some(sp) = out.get_mut(k) {
            sp.length += m.length;
            sp.more.push((m.supply, true));
            sp.more.push((m.ret, false));
        }
    }
    out.retain(|s| s.pieces().next().is_some());
    out
}

/// One loop in strip `z` with the edge zones `marked` in it: a closer
/// serpentine in each zone's piece of the strip, then on into the coil
/// over the rest, as one pipe. `None` when the zones cannot be cut off the
/// strip along their dashed lines (the loop is then laid as before).
fn loop_with_edges(
    z: &[DVec2],
    marked: &[(&Zone, f64)],
    t: f64,
    from: Option<DVec2>,
    lay: &dyn Fn(&[DVec2], usize) -> Spiral,
) -> Option<Spiral> {
    let area = |q: &[DVec2]| crate::polygon::signed_area2(q).abs() / 2.0;
    let mut rest = z.to_vec();
    let mut pieces: Vec<(Vec<DVec2>, f64)> = vec![];
    for (zone, tz) in marked {
        let [a, b, e] = zone.mark?;
        let d = b - a;
        if d.length() < 1e-6 {
            return None;
        }
        for mut part in convex_parts(z) {
            if crate::polygon::signed_area2(&part) < 0.0 {
                part.reverse();
            }
            let pc = clip_convex(&zone.outline, &part);
            if pc.len() >= 3 && area(&pc) > 0.05 {
                pieces.push((pc, *tz));
            }
        }
        // what is left on the far side of the dashed line
        let (left, right) = halves(&rest, a, d);
        rest = if d.perp_dot(e - a) > 0.0 { right } else { left };
    }
    let cut: f64 = pieces.iter().map(|(p, _)| area(p)).sum();
    if pieces.is_empty() {
        return Some(lay(z, nearest(z, from)));
    }
    if ((area(z) - area(&rest)) - cut).abs() > 0.15 * cut + 0.1 {
        return None;
    }
    // the zones' serpentines first, from the manifold's side
    let mut supply: Vec<DVec2> = vec![];
    let mut length = 0.0;
    let mut at = from;
    for (p, tz) in &pieces {
        let mut s = meander_at(p, *tz, EDGE, tz / 2.0).supply;
        if s.len() < 2 {
            continue;
        }
        if let Some(f) = at {
            if s[s.len() - 1].distance(f) < s[0].distance(f) {
                s.reverse();
            }
        }
        length += plen(&s);
        if let Some(&l) = supply.last() {
            let c = square_corner(z, l, s[0]);
            length += plen(&[l, c, s[0]]);
            supply.push(c);
        }
        at = s.last().copied();
        supply.extend(s);
    }
    if rest.len() < 3 || area(&rest) < 0.3 {
        // no room left: the serpentine comes back along itself
        let mut ret = supply.clone();
        ret.reverse();
        let ret = offset(&ret, t / 2.0);
        length += plen(&ret);
        return Some(Spiral {
            supply,
            ret,
            length,
            more: vec![],
        });
    }
    let coil = lay(&rest, nearest(&rest, at));
    if let (Some(&l), Some(&f)) = (supply.last(), coil.supply.first()) {
        let c = square_corner(z, l, f);
        length += plen(&[l, c, f]);
        supply.push(c);
    }
    length += coil.length;
    supply.extend(coil.supply);
    Some(Spiral {
        supply,
        ret: coil.ret,
        length,
        more: coil.more,
    })
}

/// The movement joints between a room's loops: along the lines its
/// loops are split at, inside the room (STN EN 1264-4 4.2.2.9.4: a joint
/// never crosses a loop, only its connecting pipes, in a sleeve).
pub fn joints(outline: &[DVec2], u: &Underfloor) -> Vec<[DVec2; 2]> {
    let n = u.loops.max(1) as usize;
    if n < 2 || u.layout == Layout::Hidden {
        return vec![];
    }
    let tidied = tidy(outline, 0.05);
    let (vertical, lines) = split_lines(&tidied, n, u.split.as_ref());
    let ax = usize::from(!vertical);
    let m = tidied.len();
    let mut out = vec![];
    for c in lines {
        // where the line crosses the outline, in pairs along it
        let mut xs: Vec<f64> = vec![];
        for i in 0..m {
            let (a, b) = (tidied[i], tidied[(i + 1) % m]);
            if (a[ax] - c) * (b[ax] - c) < 0.0
                || ((a[ax] - c).abs() < 1e-9 && (b[ax] - c).abs() > 1e-9)
            {
                let t = (c - a[ax]) / (b[ax] - a[ax]);
                xs.push(a[1 - ax] + (b[1 - ax] - a[1 - ax]) * t);
            }
        }
        xs.sort_by(f64::total_cmp);
        xs.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        for w in xs.chunks(2) {
            if let [lo, hi] = w {
                let at = |v: f64| {
                    if ax == 0 {
                        DVec2::new(c, v)
                    } else {
                        DVec2::new(v, c)
                    }
                };
                if hi - lo > 0.05 {
                    out.push([at(*lo), at(*hi)]);
                }
            }
        }
    }
    out
}

/// The corner a pipe from `a` to `b` turns at, square to the plan, the
/// way that keeps it in `z`.
fn square_corner(z: &[DVec2], a: DVec2, b: DVec2) -> DVec2 {
    let (p, q) = (DVec2::new(b.x, a.y), DVec2::new(a.x, b.y));
    let inside = |c: DVec2| {
        crate::polygon::contains(z, (a + c) / 2.0) && crate::polygon::contains(z, (c + b) / 2.0)
    };
    if inside(p) || !inside(q) {
        p
    } else {
        q
    }
}

/// `poly` less `holes` when all of them are square to the plan: the
/// largest piece left, as one outline; `None` otherwise.
pub fn square_less(poly: &[DVec2], holes: &[&[DVec2]]) -> Option<Vec<DVec2>> {
    let square = |q: &[DVec2]| {
        (0..q.len()).all(|i| {
            let d = q[(i + 1) % q.len()] - q[i];
            // as drawn: a few millimetres off is still square
            d.x.abs() < 0.005 || d.y.abs() < 0.005
        })
    };
    if !square(poly) || !holes.iter().all(|h| square(h)) {
        return None;
    }
    let mut xs: Vec<f64> = poly
        .iter()
        .chain(holes.iter().flat_map(|h| h.iter()))
        .map(|p| p.x)
        .collect();
    let mut ys: Vec<f64> = poly
        .iter()
        .chain(holes.iter().flat_map(|h| h.iter()))
        .map(|p| p.y)
        .collect();
    for v in [&mut xs, &mut ys] {
        v.sort_by(f64::total_cmp);
        v.dedup_by(|a, b| (*a - *b).abs() < 0.005);
    }
    let (nx, ny) = (xs.len(), ys.len());
    if nx < 2 || ny < 2 || nx * ny > 40_000 {
        return None;
    }
    let mut on = vec![false; (nx - 1) * (ny - 1)];
    for i in 0..nx - 1 {
        for j in 0..ny - 1 {
            let c = DVec2::new((xs[i] + xs[i + 1]) / 2.0, (ys[j] + ys[j + 1]) / 2.0);
            on[i * (ny - 1) + j] = crate::polygon::contains(poly, c)
                && !holes.iter().any(|h| crate::polygon::contains(h, c));
        }
    }
    let cell = |i: isize, j: isize| {
        i >= 0
            && j >= 0
            && (i as usize) < nx - 1
            && (j as usize) < ny - 1
            && on[i as usize * (ny - 1) + j as usize]
    };
    // the cells' outline: an edge where a cell meets one that is off,
    // counter-clockwise round the cells, keyed by grid corners
    let mut next: std::collections::HashMap<(usize, usize), Vec<(usize, usize)>> =
        Default::default();
    for i in 0..nx - 1 {
        for j in 0..ny - 1 {
            if !on[i * (ny - 1) + j] {
                continue;
            }
            let (ii, jj) = (i as isize, j as isize);
            if !cell(ii, jj - 1) {
                next.entry((i, j)).or_default().push((i + 1, j));
            }
            if !cell(ii + 1, jj) {
                next.entry((i + 1, j)).or_default().push((i + 1, j + 1));
            }
            if !cell(ii, jj + 1) {
                next.entry((i + 1, j + 1)).or_default().push((i, j + 1));
            }
            if !cell(ii - 1, jj) {
                next.entry((i, j + 1)).or_default().push((i, j));
            }
        }
    }
    let mut best: Option<Vec<DVec2>> = None;
    while let Some(&start) = next.keys().next() {
        let mut ring = vec![];
        let mut at = start;
        while let Some(v) = next.get_mut(&at) {
            let Some(to) = v.pop() else { break };
            if v.is_empty() {
                next.remove(&at);
            }
            ring.push(DVec2::new(xs[at.0], ys[at.1]));
            at = to;
            if at == start {
                break;
            }
        }
        let ring = clean(ring);
        if ring.len() >= 3 && crate::polygon::signed_area2(&ring) > 0.0 {
            let a = crate::polygon::signed_area2(&ring);
            if best
                .as_ref()
                .is_none_or(|b| a > crate::polygon::signed_area2(b))
            {
                best = Some(ring);
            }
        }
    }
    best
}

/// The corner of `part` nearest `f` (its first without one).
fn nearest(part: &[DVec2], f: Option<DVec2>) -> usize {
    f.map_or(0, |f| {
        (0..part.len())
            .min_by(|a, b| part[*a].distance(f).total_cmp(&part[*b].distance(f)))
            .unwrap_or(0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(w: f64, h: f64) -> Vec<DVec2> {
        vec![
            DVec2::ZERO,
            DVec2::new(w, 0.0),
            DVec2::new(w, h),
            DVec2::new(0.0, h),
        ]
    }

    #[test]
    fn supply_and_return_lie_side_by_side_at_the_spacing() {
        let sp = spiral(&rect(4.0, 3.0), 0.15, 0.1, 0);
        assert!(sp.supply.len() > 10 && sp.ret.len() > 10);
        assert!((sp.supply[0] - DVec2::new(0.1, 0.1)).length() < 1e-9);
        assert!((sp.ret[0].x - 0.25).abs() < 1e-9);
        // the inside 3.8 × 2.8 m over 0.15 m
        let ideal = 3.8 * 2.8 / 0.15;
        assert!(
            sp.length > ideal * 0.85 && sp.length < ideal * 1.1,
            "{}",
            sp.length
        );
        // every stretch along an axis in a rectangular room
        for q in [&sp.supply[..sp.supply.len() - 1], &sp.ret[..]] {
            for w in q.windows(2) {
                let d = (w[1] - w[0]).abs();
                assert!(d.x.min(d.y) < 1e-6, "{w:?}");
            }
        }
        // setting out from another corner
        assert!((spiral(&rect(4.0, 3.0), 0.15, 0.1, 2).supply[0].x - 3.9).abs() < 1e-9);
    }

    #[test]
    fn loops_take_strips_across_the_longer_side() {
        let l = vec![
            DVec2::ZERO,
            DVec2::new(6.0, 0.0),
            DVec2::new(6.0, 2.0),
            DVec2::new(3.0, 2.0),
            DVec2::new(3.0, 4.0),
            DVec2::new(0.0, 4.0),
        ];
        let z = zones(&l, 3);
        assert_eq!(z.len(), 3);
        let area = |q: &[DVec2]| crate::polygon::signed_area2(q).abs() / 2.0;
        assert!((z.iter().map(|q| area(q)).sum::<f64>() - area(&l)).abs() < 1e-9);
        assert_eq!(zones(&l, 1), vec![l.clone()]);
        let u = Underfloor {
            spacing: 0.2,
            loops: 2,
            ..Underfloor::default()
        };
        assert_eq!(loops(&rect(6.0, 4.0), &u).len(), 2);
        assert!(u.validate().is_ok());
        assert!(Underfloor { loops: 0, ..u }.validate().is_err());
    }

    #[test]
    fn a_loop_is_one_coil_in_an_l_shaped_room() {
        // a 5 × 8 room with a 2.3 × 1.4 bay and a centimetre's jog
        let l = vec![
            DVec2::new(5.0, 8.0),
            DVec2::new(0.0, 8.0),
            DVec2::new(0.0, 0.0),
            DVec2::new(4.0, 0.0),
            DVec2::new(4.0, 0.01),
            DVec2::new(7.3, 0.01),
            DVec2::new(7.3, 1.4),
            DVec2::new(5.0, 1.4),
        ];
        assert_eq!(tidy(&l, 0.05).len(), 6);
        for n in 1..=3 {
            let u = Underfloor {
                spacing: 0.2,
                loops: n,
                ..Underfloor::default()
            };
            let ls = loops(&l, &u);
            assert_eq!(ls.len(), n as usize);
            let area = 5.0 * 8.0 + 2.3 * 1.39;
            let len: f64 = ls.iter().map(|s| s.length).sum();
            assert!(
                len > area / 0.2 * 0.8 && len < area / 0.2 * 1.15,
                "{n} {len}"
            );
            for s in &ls {
                assert!(s.more.is_empty(), "{n}: {} pieces", s.more.len() + 2);
            }
        }
    }

    #[test]
    fn an_edge_zone_runs_through_every_loop() {
        let room = rect(6.0, 4.0);
        let edge = [DVec2::new(0.0, 0.0), DVec2::new(6.0, 0.0)];
        let mut u = Underfloor {
            spacing: 0.2,
            loops: 3,
            ..Underfloor::default()
        };
        let plain: f64 = loops(&room, &u).iter().map(|s| s.length).sum();
        u.zones = edge_zones(&room, &edge, false, 1.0, 0.1);
        let ls = loops(&room, &u);
        assert_eq!(ls.len(), 3);
        for s in &ls {
            assert!(s.more.is_empty());
            // each loop closer along the edge: some of its pipe below 1 m
            assert!(s.supply.iter().filter(|p| p.y < 0.9).count() > 4);
        }
        // the rows in the zone 0.1 m apart
        let mut ys: Vec<f64> = ls[0]
            .supply
            .iter()
            .map(|p| p.y)
            .filter(|y| *y < 0.95)
            .collect();
        ys.sort_by(f64::total_cmp);
        ys.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        assert!(ys.len() >= 8, "{ys:?}");
        for w in ys.windows(2) {
            assert!((w[1] - w[0] - 0.1).abs() < 1e-6, "{ys:?}");
        }
        let with: f64 = ls.iter().map(|s| s.length).sum();
        assert!(with > plain, "{plain} {with}");
    }

    #[test]
    fn an_l_shaped_room_is_laid_all_over() {
        // 6 × 6 less a 3 × 3 corner: 27 m²
        let l = vec![
            DVec2::ZERO,
            DVec2::new(6.0, 0.0),
            DVec2::new(6.0, 3.0),
            DVec2::new(3.0, 3.0),
            DVec2::new(3.0, 6.0),
            DVec2::new(0.0, 6.0),
        ];
        assert_eq!(convex_parts(&l).len(), 2, "{:?}", convex_parts(&l));

        for layout in [Layout::Spiral, Layout::Meander, Layout::DoubleMeander] {
            let u = Underfloor {
                spacing: 0.2,
                loops: 1,
                layout,
                ..Underfloor::default()
            };
            let len: f64 = loops(&l, &u).iter().map(|s| s.length).sum();
            assert!(
                len > 27.0 / 0.2 * 0.7 && len < 27.0 / 0.2 * 1.2,
                "{layout:?} {len}"
            );
        }
        // a hole takes its pipe away; not drawn, nothing
        let mut u = Underfloor {
            spacing: 0.2,
            ..Underfloor::default()
        };
        let full: f64 = loops(&rect(6.0, 4.0), &u).iter().map(|s| s.length).sum();
        u.zones.push(Zone {
            kind: ZoneKind::Hole,
            outline: vec![
                DVec2::new(1.0, 1.0),
                DVec2::new(3.0, 1.0),
                DVec2::new(3.0, 3.0),
                DVec2::new(1.0, 3.0),
            ],
            on: true,
            mark: None,
        });
        let holed = loops(&rect(6.0, 4.0), &u);
        for s in &holed {
            for (p, _) in s.pieces() {
                for w in p.windows(2) {
                    let m = (w[0] + w[1]) / 2.0;
                    assert!(!crate::polygon::contains(&u.zones[0].outline, m));
                }
            }
        }
        assert!(full > 0.0);
        u.layout = Layout::Hidden;
        assert!(loops(&rect(6.0, 4.0), &u).is_empty());
        // split by hand, vertically at x = 1
        let u = Underfloor {
            loops: 2,
            split: Some(Split {
                vertical: true,
                at: vec![1.0],
            }),
            ..Underfloor::default()
        };
        let z = split_zones(&rect(6.0, 4.0), 2, u.split.as_ref());
        let area = |q: &[DVec2]| crate::polygon::signed_area2(q).abs() / 2.0;
        assert!((area(&z[0]) - 4.0).abs() < 1e-9 && (area(&z[1]) - 20.0).abs() < 1e-9);
    }

    #[test]
    fn an_edge_zone_set_off_from_an_edge() {
        let room = rect(6.0, 4.0);
        // along the bottom wall, 1 m wide, into the room
        let z = edge_zones(&room, &[DVec2::ZERO, DVec2::new(6.0, 0.0)], false, 1.0, 0.1);
        assert_eq!(z.len(), 1);
        assert!((z[0].area() - 6.0).abs() < 1e-9, "{}", z[0].area());
        let m = z[0].mark.unwrap();
        assert!((m[0].y - 1.0).abs() < 1e-9 && (m[1].y - 1.0).abs() < 1e-9);
        // drawn the other way round: still into the room
        let z = edge_zones(&room, &[DVec2::new(6.0, 0.0), DVec2::ZERO], false, 1.0, 0.1);
        assert!((z[0].area() - 6.0).abs() < 1e-9);
        // round a corner: two pieces, mitred, no overlap
        let l = [DVec2::new(0.0, 4.0), DVec2::ZERO, DVec2::new(6.0, 0.0)];
        let z = edge_zones(&room, &l, false, 1.0, 0.1);
        let a: f64 = z.iter().map(Zone::area).sum();
        assert_eq!(z.len(), 2);
        assert!((a - (4.0 + 6.0 - 1.0)).abs() < 1e-9, "{a}");
        // wider than the room: cut to it
        let z = edge_zones(&room, &[DVec2::ZERO, DVec2::new(6.0, 0.0)], false, 9.0, 0.1);
        assert!((z[0].area() - 24.0).abs() < 1e-9);
    }
}

#[cfg(test)]
mod hole_tests {
    use super::*;

    #[test]
    fn a_field_goes_round_its_holes_along_the_walls() {
        let room = vec![
            DVec2::new(0.31, 0.31),
            DVec2::new(2.3, 0.31),
            DVec2::new(2.3, 3.3),
            DVec2::new(0.31, 3.3),
        ];
        let units = vec![
            DVec2::new(0.31, 0.31),
            DVec2::new(4.29, 0.31),
            DVec2::new(4.29, 3.3),
            DVec2::new(3.69, 3.3),
            DVec2::new(3.69, 0.91),
            DVec2::new(0.31, 0.91),
        ];
        let fridge = vec![
            DVec2::new(0.31, 2.2),
            DVec2::new(2.11, 2.2),
            DVec2::new(2.11, 2.82),
            DVec2::new(0.31, 2.82),
        ];
        let left = square_less(&room, &[&units, &fridge]).unwrap();
        let area = crate::polygon::signed_area2(&left) / 2.0;
        let want = 1.99 * 2.39 - 1.8 * 0.62;
        assert!((area - want).abs() < 1e-6, "{area} {want} {left:?}");
    }
}
