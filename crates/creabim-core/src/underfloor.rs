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
/// setting out from corner `start`.
pub fn spiral(outline: &[DVec2], t: f64, edge: f64, start: usize) -> Spiral {
    let n = outline.len();
    if n < 3 || t <= 0.0 {
        return Spiral::default();
    }
    let o: Vec<DVec2> = (0..n).map(|i| outline[(i + start) % n]).collect();
    let mut rings: Vec<Vec<DVec2>> = vec![];
    for j in 0..400 {
        match inset_ring(&o, edge + j as f64 * t) {
            Some(r) => rings.push(r),
            None => break,
        }
    }
    let path = |seq: Vec<&Vec<DVec2>>| -> Vec<DVec2> {
        let mut pts: Vec<DVec2> = vec![];
        for (k, r) in seq.iter().enumerate() {
            let (last, v0) = (r[n - 1], r[0]);
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
fn rows(outline: &[DVec2], t: f64, edge: f64, first: f64) -> Option<(usize, Vec<(f64, f64, f64)>)> {
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
    if outline.len() < 3 || t <= 0.0 {
        return Spiral::default();
    }
    let Some((ax, mut rs)) = rows(outline, t, edge, 0.0) else {
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
    let strips = split_zones(outline, u.loops.max(1) as usize, u.split.as_ref());
    // each strip laid part by convex part, so an L-shaped room is filled
    // all over (George 2026-11-27)
    let mut out: Vec<Spiral> = strips
        .iter()
        .map(|z| {
            let mut all = Spiral::default();
            for (k, part) in convex_parts(z).iter().enumerate() {
                let sp = match u.layout {
                    Layout::Spiral | Layout::SpiralLeading => spiral(part, u.spacing, EDGE, 0),
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
        })
        .collect();
    if u.layout == Layout::Hidden {
        return vec![];
    }
    // no pipe in a hole, nor the room's in an edge zone
    let cut: Vec<&Zone> = u.holes().chain(u.edges().map(|e| e.0)).collect();
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
    for (z, t) in u.edges() {
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
}
