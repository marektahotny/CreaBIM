//! Underfloor heating loops in a room, as PilonDesigner's
//! `floorheat.ts`: the pipe laid as a double spiral, the supply going
//! in along every other inset of the room's outline and the return
//! coming back along those between, so the two lie side by side at the
//! spacing; a room with more loops is cut into strips across its longer
//! side, each with its own spiral.

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
}

impl Underfloor {
    pub fn validate(&self) -> Result<(), String> {
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
    }
}

/// The room cut into `n` strips across its longer side, a loop each.
pub fn zones(outline: &[DVec2], n: usize) -> Vec<Vec<DVec2>> {
    if n <= 1 || outline.len() < 3 {
        return vec![outline.to_vec()];
    }
    let (lo, hi) = outline.iter().fold(
        (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN)),
        |(lo, hi), p| (lo.min(*p), hi.max(*p)),
    );
    let ax = usize::from(hi.x - lo.x < hi.y - lo.y);
    let step = (hi[ax] - lo[ax]) / n as f64;
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
            z = clip(&z, lo[ax] + i as f64 * step, 1.0);
        }
        if i < n - 1 {
            z = clip(&z, lo[ax] + (i + 1) as f64 * step, -1.0);
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

/// The loops of a room with outline `outline`, laid as designed.
pub fn loops(outline: &[DVec2], u: &Underfloor) -> Vec<Spiral> {
    zones(outline, u.loops.max(1) as usize)
        .iter()
        .map(|z| spiral(z, u.spacing, EDGE, 0))
        .filter(|s| s.supply.len() > 1)
        .collect()
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
            finish_r: None,
        };
        assert_eq!(loops(&rect(6.0, 4.0), &u).len(), 2);
        assert!(u.validate().is_ok());
        assert!(Underfloor { loops: 0, ..u }.validate().is_err());
    }
}
