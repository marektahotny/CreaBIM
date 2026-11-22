//! Plane polygon helpers shared by meshes, slabs and drawings.

use glam::DVec2;

/// Twice the signed area; positive for counter-clockwise polygons.
pub fn signed_area2(poly: &[DVec2]) -> f64 {
    (0..poly.len())
        .map(|i| poly[i].perp_dot(poly[(i + 1) % poly.len()]))
        .sum()
}

/// Ear-clipping triangulation of a simple polygon (either winding).
/// Returns triangles as indices into `poly`, counter-clockwise.
pub fn triangulate(poly: &[DVec2]) -> Vec<[usize; 3]> {
    let n = poly.len();
    if n < 3 {
        return vec![];
    }
    let rings = pinch_split(poly);
    if rings.len() <= 1 {
        return ear_clip(poly, (0..n).collect());
    }
    // an outline pinched at a corner it passes twice (a section through
    // a window frame and its sill, run round as one; a hole's bridge):
    // each loop by itself, a loop lying inside a bigger one being a hole
    let tris: Vec<Vec<[usize; 3]>> = rings.iter().map(|r| ear_clip(poly, r.clone())).collect();
    let area = |r: &[usize]| {
        let pts: Vec<DVec2> = r.iter().map(|&i| poly[i]).collect();
        signed_area2(&pts).abs()
    };
    let hole = rings.iter().enumerate().any(|(k, r)| {
        let Some(t) = tris[k].first() else {
            return false;
        };
        let inner = (poly[t[0]] + poly[t[1]] + poly[t[2]]) / 3.0;
        rings.iter().enumerate().any(|(j, o)| {
            j != k && area(o) > area(r) && {
                let pts: Vec<DVec2> = o.iter().map(|&i| poly[i]).collect();
                contains(&pts, inner)
            }
        })
    });
    if hole {
        // an outline round its holes, joined by bridges: clipped whole,
        // the bridges' corners sitting on its ears
        return ear_clip(poly, (0..n).collect());
    }
    tris.concat()
}

/// A ring of `poly`'s corners split where it passes the same point twice,
/// into loops that each pass every point once; loops with no area left
/// out.
fn pinch_split(poly: &[DVec2]) -> Vec<Vec<usize>> {
    let mut todo: Vec<Vec<usize>> = vec![(0..poly.len()).collect()];
    let mut done = vec![];
    while let Some(r) = todo.pop() {
        let m = r.len();
        let mut cut = None;
        'find: for i in 0..m {
            for j in i + 2..m {
                if i == 0 && j == m - 1 {
                    continue;
                }
                if poly[r[i]].distance(poly[r[j]]) < 1e-9 {
                    cut = Some((i, j));
                    break 'find;
                }
            }
        }
        match cut {
            Some((i, j)) => {
                todo.push(r[i..j].to_vec());
                let mut rest = r[j..].to_vec();
                rest.extend_from_slice(&r[..i]);
                todo.push(rest);
            }
            None => {
                let pts: Vec<DVec2> = r.iter().map(|&i| poly[i]).collect();
                if m >= 3 && signed_area2(&pts).abs() > 1e-14 {
                    done.push(r);
                }
            }
        }
        if done.len() + todo.len() > poly.len() {
            // never more loops than corners; give up splitting
            return vec![(0..poly.len()).collect()];
        }
    }
    done
}

/// Ear clipping of the loop of `poly`'s corners `idx`.
fn ear_clip(poly: &[DVec2], mut idx: Vec<usize>) -> Vec<[usize; 3]> {
    let n = idx.len();
    if n < 3 {
        return vec![];
    }
    let pts: Vec<DVec2> = idx.iter().map(|&i| poly[i]).collect();
    if signed_area2(&pts) < 0.0 {
        idx.reverse();
    }
    let mut out = Vec::with_capacity(n - 2);
    let mut guard = 0;
    while idx.len() > 3 && guard < n * n {
        guard += 1;
        let m = idx.len();
        let mut clipped = false;
        for i in 0..m {
            let (a, b, c) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            let (pa, pb, pc) = (poly[a], poly[b], poly[c]);
            if (pb - pa).perp_dot(pc - pb) <= 1e-12 {
                continue; // reflex or degenerate corner
            }
            // corners repeated by a hole's bridge sit on the ear, not in it
            let same = |q: DVec2| q == pa || q == pb || q == pc;
            let inside = idx.iter().any(|&j| {
                j != a
                    && j != b
                    && j != c
                    && !same(poly[j])
                    && point_in_triangle(poly[j], pa, pb, pc)
            });
            if !inside {
                out.push([a, b, c]);
                idx.remove(i);
                clipped = true;
                break;
            }
        }
        if !clipped {
            break; // self-intersecting input; return what we have
        }
    }
    if idx.len() == 3 {
        out.push([idx[0], idx[1], idx[2]]);
    }
    out
}

fn point_in_triangle(p: DVec2, a: DVec2, b: DVec2, c: DVec2) -> bool {
    let d1 = (b - a).perp_dot(p - a);
    let d2 = (c - b).perp_dot(p - b);
    let d3 = (a - c).perp_dot(p - c);
    d1 >= 0.0 && d2 >= 0.0 && d3 >= 0.0
}

/// Is `p` inside the polygon (even-odd)?
pub fn contains(poly: &[DVec2], p: DVec2) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            inside = !inside;
        }
    }
    inside
}

/// One simple polygon for an outline with holes: the outline made
/// counter-clockwise and each hole, clockwise, joined to it by a zero-width
/// bridge through the closest pair of corners. Triangulating it fills the
/// outline and leaves the holes empty.
pub fn bridged(outer: &[DVec2], holes: &[Vec<DVec2>]) -> Vec<DVec2> {
    let mut out = outer.to_vec();
    if signed_area2(&out) < 0.0 {
        out.reverse();
    }
    for h in holes.iter().filter(|h| h.len() >= 3) {
        let mut h = h.clone();
        if signed_area2(&h) > 0.0 {
            h.reverse();
        }
        let mut best = (f64::MAX, 0, 0);
        for (i, a) in out.iter().enumerate() {
            for (j, b) in h.iter().enumerate() {
                let d = a.distance_squared(*b);
                if d < best.0 {
                    best = (d, i, j);
                }
            }
        }
        let (_, i, j) = best;
        let mut next = out[..=i].to_vec();
        next.extend(h[j..].iter().copied());
        next.extend(h[..=j].iter().copied());
        next.extend(out[i..].iter().copied());
        out = next;
    }
    out
}

/// Do segments `a`–`b` and `c`–`d` cross or overlap? Touching only at a
/// shared end does not count.
pub fn segments_cross(a: DVec2, b: DVec2, c: DVec2, d: DVec2) -> bool {
    let eps = 1e-9;
    let orient = |p: DVec2, q: DVec2, r: DVec2| (q - p).perp_dot(r - p);
    let (o1, o2) = (orient(a, b, c), orient(a, b, d));
    let (o3, o4) = (orient(c, d, a), orient(c, d, b));
    if ((o1 > eps && o2 < -eps) || (o1 < -eps && o2 > eps))
        && ((o3 > eps && o4 < -eps) || (o3 < -eps && o4 > eps))
    {
        return true;
    }
    // collinear or touching: a point of one lying on the other, other than
    // an end they share
    let on = |p: DVec2, q: DVec2, r: DVec2| {
        orient(p, q, r).abs() <= eps * (q - p).length().max(1.0)
            && r.x >= p.x.min(q.x) - eps
            && r.x <= p.x.max(q.x) + eps
            && r.y >= p.y.min(q.y) - eps
            && r.y <= p.y.max(q.y) + eps
    };
    (on(a, b, c) && c.distance(a) > 1e-9 && c.distance(b) > 1e-9)
        || (on(a, b, d) && d.distance(a) > 1e-9 && d.distance(b) > 1e-9)
        || (on(c, d, a) && a.distance(c) > 1e-9 && a.distance(d) > 1e-9)
        || (on(c, d, b) && b.distance(c) > 1e-9 && b.distance(d) > 1e-9)
}

/// The edges of a ring of corners: each corner to the next, and the last
/// back to the first when it is `closed`.
fn edges(poly: &[DVec2], closed: bool) -> Vec<(DVec2, DVec2)> {
    let n = poly.len();
    let count = if closed { n } else { n.saturating_sub(1) };
    (0..count).map(|i| (poly[i], poly[(i + 1) % n])).collect()
}

/// Does an outline cross itself? Edges next to each other may share their
/// corner; any other contact counts.
pub fn self_intersects(poly: &[DVec2], closed: bool) -> bool {
    let e = edges(poly, closed);
    let n = e.len();
    for i in 0..n {
        for j in i + 1..n {
            let next = j == i + 1 || (closed && i == 0 && j == n - 1);
            let (a, b) = e[i];
            let (c, d) = e[j];
            if next {
                // neighbours meet at one corner; folding back onto each
                // other overlaps them
                let (p, q, r) = if j == i + 1 { (a, b, d) } else { (c, d, b) };
                let back = (q - p).normalize_or_zero().dot((r - q).normalize_or_zero());
                if (q - p).perp_dot(r - q).abs() < 1e-12 && back < 0.0 {
                    return true;
                }
                continue;
            }
            if segments_cross(a, b, c, d) {
                return true;
            }
        }
    }
    false
}

/// Do the edges of two closed outlines cross or touch?
pub fn outlines_cross(a: &[DVec2], b: &[DVec2]) -> bool {
    let (ea, eb) = (edges(a, true), edges(b, true));
    ea.iter()
        .any(|(p, q)| eb.iter().any(|(r, s)| segments_cross(*p, *q, *r, *s)))
}

/// Is closed outline `inner` inside `outer`, without touching its edges?
pub fn inside(inner: &[DVec2], outer: &[DVec2]) -> bool {
    !inner.is_empty() && inner.iter().all(|p| contains(outer, *p)) && !outlines_cross(inner, outer)
}

/// Do two closed outlines share any area (cross, or one inside the other)?
pub fn overlap(a: &[DVec2], b: &[DVec2]) -> bool {
    outlines_cross(a, b)
        || a.first().is_some_and(|p| contains(b, *p))
        || b.first().is_some_and(|p| contains(a, *p))
}

/// A rectangle's corners, counter-clockwise, from two opposite corners.
pub fn rectangle(a: DVec2, b: DVec2) -> Vec<DVec2> {
    let (lo, hi) = (a.min(b), a.max(b));
    vec![lo, DVec2::new(hi.x, lo.y), hi, DVec2::new(lo.x, hi.y)]
}

#[cfg(test)]
mod hole_tests {
    use super::*;

    #[test]
    fn crossings_and_containment_are_found() {
        let sq = rectangle(DVec2::ZERO, DVec2::new(4.0, 4.0));
        assert!(!self_intersects(&sq, true));
        // a bow tie crosses itself
        let bow = vec![
            DVec2::ZERO,
            DVec2::new(2.0, 2.0),
            DVec2::new(2.0, 0.0),
            DVec2::new(0.0, 2.0),
        ];
        assert!(self_intersects(&bow, true));
        // an open path doubling back on itself
        let back = vec![DVec2::ZERO, DVec2::new(2.0, 0.0), DVec2::new(1.0, 0.0)];
        assert!(self_intersects(&back, false));
        let small = rectangle(DVec2::new(1.0, 1.0), DVec2::new(2.0, 2.0));
        assert!(inside(&small, &sq));
        let across = rectangle(DVec2::new(3.0, 1.0), DVec2::new(5.0, 2.0));
        assert!(!inside(&across, &sq) && outlines_cross(&across, &sq));
        assert!(overlap(&small, &sq) && overlap(&across, &sq));
        let apart = rectangle(DVec2::new(6.0, 6.0), DVec2::new(7.0, 7.0));
        assert!(!overlap(&apart, &sq));
        // corners shared by neighbouring edges are no crossing
        assert!(!segments_cross(
            DVec2::ZERO,
            DVec2::X,
            DVec2::X,
            DVec2::new(1.0, 1.0)
        ));
    }

    #[test]
    fn a_bridged_outline_triangulates_round_its_hole() {
        let outer = rectangle(DVec2::ZERO, DVec2::new(4.0, 4.0));
        let hole = rectangle(DVec2::new(1.0, 1.0), DVec2::new(2.0, 3.0));
        let p = bridged(&outer, &[hole]);
        assert_eq!(p.len(), 4 + 4 + 2);
        let area: f64 = triangulate(&p)
            .iter()
            .map(|t| signed_area2(&[p[t[0]], p[t[1]], p[t[2]]]) / 2.0)
            .sum();
        assert!((area - 14.0).abs() < 1e-9, "{area}");
        assert!(contains(&outer, DVec2::new(1.5, 2.0)));
    }

    /// George: grey wedges by a window's frame in a section. Its cut,
    /// frame and sill pieces run round as one outline pinched where they
    /// meet, was filled with triangles reaching out of it; each loop is
    /// filled by itself now, all of it and nothing more.
    #[test]
    fn a_pinched_outline_is_filled_loop_by_loop() {
        let p: Vec<DVec2> = [
            (3.262, 1.675),
            (3.262, 1.650),
            (3.023, 1.650),
            (3.023, 2.400),
            (2.922, 2.400),
            (2.922, 1.650),
            (2.727, 1.650),
            (2.727, 1.680),
            (2.922, 1.680),
            (2.922, 1.650),
            (3.023, 1.650),
            (3.023, 1.675),
        ]
        .iter()
        .map(|(x, y)| DVec2::new(*x, *y))
        .collect();
        let t = triangulate(&p);
        let area: f64 = t
            .iter()
            .map(|t| signed_area2(&[p[t[0]], p[t[1]], p[t[2]]]).abs() / 2.0)
            .sum();
        let pieces = 0.239 * 0.025 + 0.101 * 0.75 + 0.195 * 0.03;
        assert!((area - pieces).abs() < 1e-9, "{area} {pieces}");
        // a hole joined by a bridge stays empty
        let outer = [
            DVec2::new(0.0, 0.0),
            DVec2::new(4.0, 0.0),
            DVec2::new(4.0, 4.0),
            DVec2::new(0.0, 4.0),
        ];
        let hole = vec![
            DVec2::new(1.0, 1.0),
            DVec2::new(3.0, 1.0),
            DVec2::new(3.0, 3.0),
            DVec2::new(1.0, 3.0),
        ];
        let b = bridged(&outer, &[hole]);
        let t = triangulate(&b);
        let area: f64 = t
            .iter()
            .map(|t| signed_area2(&[b[t[0]], b[t[1]], b[t[2]]]).abs() / 2.0)
            .sum();
        assert!((area - 12.0).abs() < 1e-9, "{area}");
    }
}
