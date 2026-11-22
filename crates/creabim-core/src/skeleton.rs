//! The weighted straight skeleton of a polygon: what a roof over any
//! outline looks like when each eave rises at its own pitch. Every edge's
//! line moves inward at its own speed (a gable's not at all); where the
//! moving lines meet they leave the ridges, hips and valleys.
//!
//! Event by event, exact between events: an edge shrinking away, a reflex
//! corner running into an edge across (which splits the outline in two),
//! two opposite edges meeting along a ridge. Made for roofs: tens of
//! corners, so each step simply looks at every pair.

use glam::DVec2;

const EPS: f64 = 1e-9;

/// One edge's line at time `t`: `n · p = c + s t`, `n` pointing inward.
#[derive(Clone, Copy)]
struct Line {
    n: DVec2,
    c: f64,
    s: f64,
}

/// A corner of the moving outline, where edge `edges[k - 1]` meets
/// `edges[k]`: the node it started from and when.
#[derive(Clone, Copy)]
struct Corner {
    node: usize,
    t0: f64,
}

struct Wave {
    edges: Vec<usize>,
    corners: Vec<Corner>,
}

/// One face of the skeleton: the part of the outline's inside that edge
/// `edge` reaches first, as a polygon in plan (counter-clockwise).
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    pub edge: usize,
    pub outline: Vec<DVec2>,
}

struct Skeleton {
    lines: Vec<Line>,
    nodes: Vec<DVec2>,
    /// (from node, to node, the edges on either side)
    arcs: Vec<(usize, usize, usize, usize)>,
}

impl Skeleton {
    /// Where the corner between edges `a` and `b` is at time `t`, and how
    /// fast it moves.
    fn corner(&self, a: usize, b: usize, k: Corner, t: f64) -> (DVec2, DVec2) {
        let (la, lb) = (self.lines[a], self.lines[b]);
        let det = la.n.perp_dot(lb.n);
        if det.abs() < 1e-9 {
            let start = self.nodes[k.node];
            if la.n.dot(lb.n) > 0.0 {
                // one straight line on: the corner runs straight in
                let v = la.n * (la.s + lb.s) / 2.0;
                return (start + v * (t - k.t0), v);
            }
            // two edges facing each other: a ridge, standing still
            return (start, DVec2::ZERO);
        }
        let solve = |ra: f64, rb: f64| {
            DVec2::new(
                (ra * lb.n.y - rb * la.n.y) / det,
                (la.n.x * rb - lb.n.x * ra) / det,
            )
        };
        (solve(la.c + la.s * t, lb.c + lb.s * t), solve(la.s, lb.s))
    }

    fn node(&mut self, p: DVec2) -> usize {
        self.nodes.push(p);
        self.nodes.len() - 1
    }
}

enum Event {
    /// Wave `w`'s edge at index `k` shrinks away.
    Edge { w: usize, k: usize },
    /// Wave `w`'s corner `k` runs into its edge at index `e`.
    Split { w: usize, k: usize, e: usize },
}

/// The faces of `poly` (counter-clockwise), edge `i` (corner `i` to
/// `i + 1`) moving inward at `speed[i]` (the run per unit of rise: a
/// roof's `1 / tan(pitch)`; 0 for a gable, which never moves). Gables
/// get no face. Empty if the polygon is degenerate or nothing moves.
pub fn faces(poly: &[DVec2], speed: &[f64]) -> Vec<Face> {
    let n = poly.len();
    if n < 3 || speed.len() != n || speed.iter().all(|s| *s <= EPS) {
        return vec![];
    }
    let mut sk = Skeleton {
        lines: (0..n)
            .map(|i| {
                let d = (poly[(i + 1) % n] - poly[i]).normalize_or_zero();
                let nrm = d.perp();
                Line {
                    n: nrm,
                    c: nrm.dot(poly[i]),
                    s: speed[i].max(0.0),
                }
            })
            .collect(),
        nodes: poly.to_vec(),
        arcs: vec![],
    };
    if sk.lines.iter().any(|l| l.n == DVec2::ZERO) {
        return vec![];
    }
    let mut waves = vec![Wave {
        edges: (0..n).collect(),
        corners: (0..n).map(|i| Corner { node: i, t0: 0.0 }).collect(),
    }];
    let mut now = 0.0;
    let mut guard = 0;
    while !waves.is_empty() && guard < 40 * n * n + 100 {
        guard += 1;
        // close the waves that are done, and meet the ridges now formed
        tidy(&mut sk, &mut waves, now);
        if waves.is_empty() {
            break;
        }
        let Some((dt, ev)) = next_event(&sk, &waves, now) else {
            break;
        };
        now += dt.max(0.0);
        match ev {
            Event::Edge { w, k } => {
                let wave = &mut waves[w];
                let m = wave.edges.len();
                let (a, b, c) = (
                    wave.edges[(k + m - 1) % m],
                    wave.edges[k],
                    wave.edges[(k + 1) % m],
                );
                let (k0, k1) = (wave.corners[k], wave.corners[(k + 1) % m]);
                let p = (sk.corner(a, b, k0, now).0 + sk.corner(b, c, k1, now).0) / 2.0;
                let node = sk.node(p);
                sk.arcs.push((k0.node, node, a, b));
                sk.arcs.push((k1.node, node, b, c));
                let wave = &mut waves[w];
                wave.edges.remove(k);
                wave.corners.remove(k);
                let k1i = k % wave.corners.len();
                wave.corners[k1i] = Corner { node, t0: now };
            }
            Event::Split { w, k, e } => {
                let wave = waves.swap_remove(w);
                let m = wave.edges.len();
                let (a, b) = (wave.edges[(k + m - 1) % m], wave.edges[k]);
                let kc = wave.corners[k];
                let p = sk.corner(a, b, kc, now).0;
                let node = sk.node(p);
                sk.arcs.push((kc.node, node, a, b));
                let here = Corner { node, t0: now };
                // one side: from b round to the edge hit, closing on it
                let mut w1 = Wave {
                    edges: vec![],
                    corners: vec![],
                };
                let mut i = k;
                loop {
                    w1.edges.push(wave.edges[i]);
                    w1.corners.push(if i == k { here } else { wave.corners[i] });
                    if i == e {
                        break;
                    }
                    i = (i + 1) % m;
                }
                // the other: from the edge hit round to a
                let mut w2 = Wave {
                    edges: vec![],
                    corners: vec![],
                };
                let mut i = e;
                loop {
                    w2.edges.push(wave.edges[i]);
                    w2.corners.push(if i == e { here } else { wave.corners[i] });
                    if (i + 1) % m == k {
                        break;
                    }
                    i = (i + 1) % m;
                }
                waves.push(w1);
                waves.push(w2);
            }
        }
    }
    assemble(&sk, poly)
}

/// Closes waves of fewer than three edges, drops edges shrunk to nothing,
/// and folds away two edges facing each other once they meet.
fn tidy(sk: &mut Skeleton, waves: &mut Vec<Wave>, now: f64) {
    let mut w = 0;
    while w < waves.len() {
        let mut changed = true;
        while changed {
            changed = false;
            let m = waves[w].edges.len();
            if m < 3 {
                break;
            }
            let pos: Vec<DVec2> = (0..m)
                .map(|k| {
                    let wave = &waves[w];
                    let (a, b) = (wave.edges[(k + m - 1) % m], wave.edges[k]);
                    sk.corner(a, b, wave.corners[k], now).0
                })
                .collect();
            for k in 0..m {
                let (a, b) = (waves[w].edges[k], waves[w].edges[(k + 1) % m]);
                let (la, lb) = (sk.lines[a], sk.lines[b]);
                let facing = la.n.dot(lb.n) < -1.0 + 1e-9
                    && (la.c + la.s * now + lb.c + lb.s * now).abs() < 1e-7;
                // an edge between corners k and k+1 shrunk to nothing
                let gone = pos[k].distance(pos[(k + 1) % m]) < 1e-7;
                if gone {
                    let wave = &waves[w];
                    let prev = wave.edges[(k + m - 1) % m];
                    let next = wave.edges[(k + 1) % m];
                    let (k0, k1) = (wave.corners[k], wave.corners[(k + 1) % m]);
                    let node = sk.node(pos[k]);
                    sk.arcs.push((k0.node, node, prev, a));
                    sk.arcs.push((k1.node, node, a, next));
                    let wave = &mut waves[w];
                    wave.edges.remove(k);
                    wave.corners.remove(k);
                    let i = k % wave.corners.len();
                    wave.corners[i] = Corner { node, t0: now };
                    changed = true;
                    break;
                }
                if facing {
                    // edges a and b lie on one line now: the shorter one is
                    // gone, leaving the ridge from their corner to its end
                    let k1 = (k + 1) % m;
                    let (pa, pb) = (pos[k], pos[(k + 2) % m]);
                    let ridge = pos[k1];
                    let (da, db) = (ridge.distance(pa), ridge.distance(pb));
                    let wave = &waves[w];
                    let mid = wave.corners[k1];
                    if da <= db + 1e-7 {
                        // a ends first: its far corner is where the ridge ends
                        let prev = wave.edges[(k + m - 1) % m];
                        let ka = wave.corners[k];
                        let node = sk.node(pa);
                        sk.arcs.push((mid.node, node, a, b));
                        sk.arcs.push((ka.node, node, prev, a));
                        let wave = &mut waves[w];
                        wave.edges.remove(k);
                        wave.corners.remove(k);
                        // the corner now between prev and b
                        let i = k % wave.corners.len();
                        wave.corners[i] = Corner { node, t0: now };
                    } else {
                        let next = wave.edges[(k + 2) % m];
                        let kb = wave.corners[(k + 2) % m];
                        let node = sk.node(pb);
                        sk.arcs.push((mid.node, node, a, b));
                        sk.arcs.push((kb.node, node, b, next));
                        let wave = &mut waves[w];
                        wave.edges.remove(k1);
                        wave.corners.remove(k1);
                        // the corner now between a and next
                        let i = k1 % wave.corners.len();
                        wave.corners[i] = Corner { node, t0: now };
                    }
                    changed = true;
                    break;
                }
            }
        }
        let m = waves[w].edges.len();
        if m < 3 {
            let wave = waves.swap_remove(w);
            if m == 2 {
                let (a, b) = (wave.edges[0], wave.edges[1]);
                let p0 = sk.corner(b, a, wave.corners[0], now).0;
                let p1 = sk.corner(a, b, wave.corners[1], now).0;
                let (n0, n1) = (sk.node(p0), sk.node(p1));
                sk.arcs.push((wave.corners[0].node, n0, b, a));
                sk.arcs.push((wave.corners[1].node, n1, a, b));
                sk.arcs.push((n0, n1, a, b));
            }
            continue;
        }
        w += 1;
    }
}

/// The soonest thing to happen, `dt` from now; edges shrinking away
/// before corners splitting an edge when both happen at once.
fn next_event(sk: &Skeleton, waves: &[Wave], now: f64) -> Option<(f64, Event)> {
    let mut best: Option<(f64, u8, Event)> = None;
    let mut offer = |dt: f64, rank: u8, ev: Event| {
        if dt < -1e-7 || !dt.is_finite() {
            return;
        }
        let better = match &best {
            None => true,
            Some((t, r, _)) => dt < *t - 1e-10 || (dt < *t + 1e-10 && rank < *r),
        };
        if better {
            best = Some((dt, rank, ev));
        }
    };
    for (w, wave) in waves.iter().enumerate() {
        let m = wave.edges.len();
        let motion: Vec<(DVec2, DVec2)> = (0..m)
            .map(|k| {
                let (a, b) = (wave.edges[(k + m - 1) % m], wave.edges[k]);
                sk.corner(a, b, wave.corners[k], now)
            })
            .collect();
        for k in 0..m {
            let b = wave.edges[k];
            let d = sk.lines[b].n.perp() * -1.0;
            let (p0, v0) = motion[k];
            let (p1, v1) = motion[(k + 1) % m];
            let len = (p1 - p0).dot(d);
            let shrink = (v1 - v0).dot(d);
            if shrink < -1e-12 {
                offer(-len / shrink, 0, Event::Edge { w, k });
            }
        }
        for k in 0..m {
            let (a, b) = (wave.edges[(k + m - 1) % m], wave.edges[k]);
            let (la, lb) = (sk.lines[a], sk.lines[b]);
            // reflex: the outline turns right here
            if la.n.perp_dot(lb.n) > -1e-9 {
                continue;
            }
            let (p, v) = motion[k];
            for e in 0..m {
                if e == k || (e + 1) % m == k || e == (k + m - 1) % m {
                    continue;
                }
                let le = sk.lines[wave.edges[e]];
                let gap = le.n.dot(p) - le.c - le.s * now;
                let close = le.n.dot(v) - le.s;
                if close >= -1e-12 || gap < -1e-7 {
                    continue;
                }
                let dt = gap.max(0.0) / -close;
                let hit = p + v * dt;
                let (q0, u0) = motion[e];
                let (q1, u1) = motion[(e + 1) % m];
                let (e0, e1) = (q0 + u0 * dt, q1 + u1 * dt);
                let along = e1 - e0;
                let l2 = along.length_squared();
                if l2 < 1e-14 {
                    continue;
                }
                let s = (hit - e0).dot(along) / l2;
                if (-1e-7..=1.0 + 1e-7).contains(&s) {
                    offer(dt, 1, Event::Split { w, k, e });
                }
            }
        }
    }
    best.map(|(dt, _, ev)| (dt, ev))
}

/// Each moving edge's face: its edge, then the arcs on its side from the
/// edge's end back round to its start.
fn assemble(sk: &Skeleton, poly: &[DVec2]) -> Vec<Face> {
    let n = poly.len();
    // nodes at one place are one node
    let mut same: Vec<usize> = (0..sk.nodes.len()).collect();
    for i in 0..sk.nodes.len() {
        for j in 0..i {
            if same[j] == j && sk.nodes[i].distance(sk.nodes[j]) < 1e-6 {
                same[i] = j;
                break;
            }
        }
    }
    let mut out = Vec::new();
    for edge in 0..n {
        if sk.lines[edge].s <= EPS {
            continue;
        }
        let mut segs: Vec<(usize, usize)> = sk
            .arcs
            .iter()
            .filter(|a| a.2 == edge || a.3 == edge)
            .map(|a| (same[a.0], same[a.1]))
            .filter(|(p, q)| p != q)
            .collect();
        segs.sort();
        segs.dedup_by(|x, y| (x.0 == y.0 && x.1 == y.1) || (x.0 == y.1 && x.1 == y.0));
        let (start, end) = (same[edge], same[(edge + 1) % n]);
        let mut path = vec![start, end];
        let mut at = end;
        let mut ok = false;
        for _ in 0..segs.len() + 1 {
            let Some(i) = segs.iter().position(|s| s.0 == at || s.1 == at) else {
                break;
            };
            let s = segs.swap_remove(i);
            at = if s.0 == at { s.1 } else { s.0 };
            if at == start {
                ok = true;
                break;
            }
            path.push(at);
        }
        if !ok {
            continue;
        }
        let mut outline: Vec<DVec2> = path.iter().map(|i| sk.nodes[*i]).collect();
        outline.dedup_by(|a, b| a.distance(*b) < 1e-9);
        if outline.len() >= 3 && crate::polygon::signed_area2(&outline).abs() > 1e-9 {
            if crate::polygon::signed_area2(&outline) < 0.0 {
                outline.reverse();
            }
            out.push(Face { edge, outline });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area(f: &[Face]) -> f64 {
        f.iter()
            .map(|f| crate::polygon::signed_area2(&f.outline) / 2.0)
            .sum()
    }

    fn poly(p: &[(f64, f64)]) -> Vec<DVec2> {
        p.iter().map(|(x, y)| DVec2::new(*x, *y)).collect()
    }

    /// Every corner of every face stands at one height, whichever face
    /// it is measured on; the faces cover the outline once.
    fn check(p: &[DVec2], sp: &[f64]) -> Vec<Face> {
        let f = faces(p, sp);
        let n = p.len();
        let z = |e: usize, q: DVec2| {
            let d = (p[(e + 1) % n] - p[e]).normalize();
            d.perp().dot(q - p[e]) / sp[e]
        };
        for a in &f {
            assert!(crate::polygon::signed_area2(&a.outline) > 0.0);
            for q in &a.outline {
                let h = z(a.edge, *q);
                assert!(h > -1e-6, "below the eaves: {h}");
                // a gable stands upright: the faces either side may differ
                let on_gable = (0..n).any(|g| {
                    sp[g] == 0.0 && {
                        let d = (p[(g + 1) % n] - p[g]).normalize();
                        d.perp().dot(*q - p[g]).abs() < 1e-6
                    }
                });
                for b in f.iter().filter(|_| !on_gable) {
                    if b.outline.iter().any(|r| r.distance(*q) < 1e-6) {
                        let hb = z(b.edge, *q);
                        assert!((h - hb).abs() < 1e-6, "{q}: {h} vs {hb}");
                    }
                }
            }
        }
        let want = crate::polygon::signed_area2(p) / 2.0;
        assert!((area(&f) - want).abs() < 1e-6, "{} vs {want}", area(&f));
        f
    }

    /// The highest point: the most any point of a face is from its edge,
    /// over the edge's speed.
    fn peak(p: &[DVec2], sp: &[f64], f: &[Face]) -> f64 {
        let n = p.len();
        f.iter()
            .flat_map(|f| {
                let d = (p[(f.edge + 1) % n] - p[f.edge]).normalize();
                f.outline
                    .iter()
                    .map(move |q| d.perp().dot(*q - p[f.edge]) / sp[f.edge])
            })
            .fold(0.0, f64::max)
    }

    #[test]
    fn a_rectangle_hips_to_a_ridge() {
        let p = poly(&[(0.0, 0.0), (10.0, 0.0), (10.0, 6.0), (0.0, 6.0)]);
        let f = check(&p, &[1.0; 4]);
        assert_eq!(f.len(), 4);
        assert!((area(&f) - 60.0).abs() < 1e-6);
        assert!((peak(&p, &[1.0; 4], &f) - 3.0).abs() < 1e-6);
        // the long sides are trapezoids, the short ones triangles
        assert_eq!(f.iter().filter(|f| f.outline.len() == 4).count(), 2);
    }

    #[test]
    fn gables_stand_still_and_the_slopes_meet_over_them() {
        let p = poly(&[(0.0, 0.0), (10.0, 0.0), (10.0, 6.0), (0.0, 6.0)]);
        let f = check(&p, &[1.0, 0.0, 1.0, 0.0]);
        assert_eq!(f.len(), 2);
        assert!((area(&f) - 60.0).abs() < 1e-6);
        for face in &f {
            assert!((crate::polygon::signed_area2(&face.outline) / 2.0 - 30.0).abs() < 1e-6);
        }
    }

    #[test]
    fn an_l_shape_gets_its_valley() {
        let p = poly(&[
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 4.0),
            (4.0, 4.0),
            (4.0, 10.0),
            (0.0, 10.0),
        ]);
        let sp = [1.0; 6];
        let f = check(&p, &sp);
        assert_eq!(f.len(), 6);
        assert!((area(&f) - 64.0).abs() < 1e-6, "{}", area(&f));
        assert!((peak(&p, &sp, &f) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn a_house_with_a_bay_in_front() {
        // George's outline: a rectangle with a small bump at the front
        let p = poly(&[
            (0.0, 0.0),
            (4.0, 0.0),
            (4.0, -1.5),
            (8.0, -1.5),
            (8.0, 0.0),
            (12.0, 0.0),
            (12.0, 9.0),
            (0.0, 9.0),
        ]);
        let sp = [1.0; 8];
        let f = check(&p, &sp);
        assert_eq!(f.len(), 8);
        assert!((area(&f) - 114.0).abs() < 1e-6, "{}", area(&f));
        assert!((peak(&p, &sp, &f) - 4.5).abs() < 1e-6);
    }

    #[test]
    fn a_t_shape_with_a_steeper_wing() {
        let p = poly(&[
            (0.0, 0.0),
            (12.0, 0.0),
            (12.0, 6.0),
            (8.0, 6.0),
            (8.0, 12.0),
            (4.0, 12.0),
            (4.0, 6.0),
            (0.0, 6.0),
        ]);
        let mut sp = [1.0; 8];
        sp[3] = 0.5;
        sp[5] = 0.0;
        let f = check(&p, &sp);
        assert_eq!(f.len(), 7);
        assert!((area(&f) - 96.0).abs() < 1e-6, "{}", area(&f));
        // every face is a simple polygon of its own edge's side
        for face in &f {
            assert!(crate::polygon::signed_area2(&face.outline) > 0.0);
        }
    }

    #[test]
    fn a_u_shape_splits_and_keeps_its_arms() {
        let p = poly(&[
            (0.0, 0.0),
            (12.0, 0.0),
            (12.0, 10.0),
            (8.0, 10.0),
            (8.0, 4.0),
            (4.0, 4.0),
            (4.0, 10.0),
            (0.0, 10.0),
        ]);
        let sp = [1.0; 8];
        let f = check(&p, &sp);
        assert_eq!(f.len(), 8);
        assert!((area(&f) - 96.0).abs() < 1e-6, "{}", area(&f));
        assert!((peak(&p, &sp, &f) - 2.0).abs() < 1e-6);
    }

    #[test]
    fn odd_outlines_close_with_every_pitch() {
        let shapes = [
            // an L with a gable on its wing
            vec![
                (0.0, 0.0),
                (10.0, 0.0),
                (10.0, 4.0),
                (4.0, 4.0),
                (4.0, 10.0),
                (0.0, 10.0),
            ],
            // a Z
            vec![
                (0.0, 0.0),
                (8.0, 0.0),
                (8.0, 4.0),
                (14.0, 4.0),
                (14.0, 9.0),
                (6.0, 9.0),
                (6.0, 5.0),
                (0.0, 5.0),
            ],
            // a slanted end
            vec![(0.0, 0.0), (9.0, 0.0), (11.0, 5.0), (0.0, 5.0)],
            // a cross
            vec![
                (4.0, 0.0),
                (8.0, 0.0),
                (8.0, 4.0),
                (12.0, 4.0),
                (12.0, 8.0),
                (8.0, 8.0),
                (8.0, 12.0),
                (4.0, 12.0),
                (4.0, 8.0),
                (0.0, 8.0),
                (0.0, 4.0),
                (4.0, 4.0),
            ],
        ];
        for s in shapes {
            let p = poly(&s);
            let n = p.len();
            for k in 0..4 {
                let sp: Vec<f64> = (0..n)
                    .map(|i| match (i + k) % 5 {
                        0 if k > 0 => 0.6,
                        3 if k > 1 => 1.4,
                        _ => 1.0,
                    })
                    .collect();
                check(&p, &sp);
            }
        }
    }
}
