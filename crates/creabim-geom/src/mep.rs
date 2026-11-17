//! Pipes and ducts in 3D and in plan: a pipe a cylinder (a rectangular
//! duct a box) along its axis, in its system's colour; in plan its
//! outline, washed in that colour, cut or seen below as other members.
//!
//! Where pipes meet end to end they are drawn joined: two rigid pipes at
//! a corner mitre into each other (an elbow), a pipe that is bent rather
//! than fitted (multilayer pipe from a coil, up to the size its maker
//! allows) turns in an arc of the maker's bending radius, and at a tee or
//! a cross no end is closed. The model stays straight pipes sharing a
//! point; the joints are how they are drawn (PilonDesigner's
//! `pipebends.ts`).

use crate::mesh::TriMesh;
use crate::plan::{Pen, PlanItem};
use creabim_core::mep::Pipe;
use creabim_core::{Element, Model};
use glam::{DVec2, DVec3};

/// Sides of a pipe's cylinder.
const SIDES: usize = 16;

/// A system's colour, sRGB 0..1; grey when the system is unknown.
pub fn color(p: &Pipe) -> [f32; 3] {
    let c = p.system_def().map_or(0x8c8c8c, |s| s.color);
    [
        ((c >> 16) & 0xff) as f32 / 255.0,
        ((c >> 8) & 0xff) as f32 / 255.0,
        (c & 0xff) as f32 / 255.0,
    ]
}

/// Its ends in the model, m.
pub fn ends(model: &Model, p: &Pipe) -> (DVec3, DVec3) {
    let z = DVec3::Z * model.level(p.level).map_or(0.0, |l| l.elevation);
    (p.start + z, p.end + z)
}

/// Two axes square to `d` (a unit vector): the first level, unless `d` is
/// upright.
fn across(d: DVec3) -> (DVec3, DVec3) {
    let side = if d.truncate().length() > 1e-9 {
        DVec3::Z.cross(d).normalize()
    } else {
        DVec3::X
    };
    (side, d.cross(side).normalize())
}

/// How one end of a pipe meets other pipes.
#[derive(Clone, Debug, PartialEq)]
pub enum Joint {
    /// Nothing there: the end is closed.
    Free,
    /// One other pipe, bent into: the pipe stops at `at`, where the arc
    /// `arc` (from this pipe's stop to the other's) begins; one of the
    /// two draws it.
    Bend {
        at: DVec3,
        arc: Vec<DVec3>,
        owner: bool,
    },
    /// One other rigid pipe at an angle: `to` is the way it leaves.
    Elbow { to: DVec3, owner: bool },
    /// Two or more others (a tee, a cross), or one straight on.
    Node { owner: bool },
}

use creabim_services::fittings::{bend, key};

/// How the pipe's start and end meet the model's other pipes.
pub fn joints(model: &Model, p: &Pipe) -> [Joint; 2] {
    let (a, b) = ends(model, p);
    let mut out = [Joint::Free, Joint::Free];
    if a.distance(b) < 1e-6 {
        return out;
    }
    // the others at each end: (their pipe, their far end)
    let mut at: [Vec<(&Pipe, DVec3)>; 2] = [vec![], vec![]];
    let keys = [key(a), key(b)];
    for e in model.iter() {
        let creabim_core::ElementKind::Pipe(q) = &e.kind else {
            continue;
        };
        let (qa, qb) = ends(model, q);
        if (key(qa) == keys[0] && key(qb) == keys[1]) || (key(qa) == keys[1] && key(qb) == keys[0])
        {
            // itself (or one laid over it)
            continue;
        }
        for (i, k) in keys.iter().enumerate() {
            if key(qa) == *k {
                at[i].push((q, qb));
            } else if key(qb) == *k {
                at[i].push((q, qa));
            }
        }
    }
    // of those at a node, the one whose far end comes first draws what is shared
    let first = |x: DVec3, y: DVec3| (x.x, x.y, x.z) < (y.x, y.y, y.z);
    for (i, others) in at.iter().enumerate() {
        let (node, far) = if i == 0 { (a, b) } else { (b, a) };
        let owner = others.iter().all(|(_, f)| first(far, *f));
        out[i] = match others.as_slice() {
            [] => Joint::Free,
            [(q, qfar)] => {
                let (ua, la) = ((far - node).normalize(), far.distance(node));
                let lb = qfar.distance(node);
                if lb < 1e-6 {
                    Joint::Node { owner }
                } else {
                    let ub = (*qfar - node) / lb;
                    // the angle between the two arms, and how far the run turns
                    let th = ua.dot(ub).clamp(-1.0, 1.0).acos();
                    let dev = std::f64::consts::PI - th;
                    let least = 5.0_f64.to_radians();
                    if dev < least || th < least {
                        Joint::Node { owner }
                    } else {
                        let bent = bend(p, q, la, lb, th).map(|r| {
                            let t = r / (th / 2.0).tan();
                            {
                                let (t1, t2) = (node + ua * t, node + ub * t);
                                let c = node + (ua + ub).normalize() * (r / (th / 2.0).sin());
                                let (r1, r2) = (t1 - c, t2 - c);
                                let n =
                                    ((dev / (std::f64::consts::PI / 16.0)).ceil() as usize).max(4);
                                let s = dev.sin();
                                let arc = (0..=n)
                                    .map(|k| {
                                        let f = k as f64 / n as f64;
                                        c + r1 * (((1.0 - f) * dev).sin() / s)
                                            + r2 * ((f * dev).sin() / s)
                                    })
                                    .collect();
                                (t1, arc)
                            }
                        });
                        match bent {
                            Some((at, arc)) => Joint::Bend { at, arc, owner },
                            None => Joint::Elbow { to: ub, owner },
                        }
                    }
                }
            }
            _ => Joint::Node { owner },
        };
    }
    out
}

/// Rings of a round pipe of radius `r` along `path`, each square to the
/// way the path runs there, as a tube without ends.
fn tube(m: &mut TriMesh, path: &[DVec3], r: f64) {
    let path: Vec<(DVec3, f64)> = path.iter().map(|p| (*p, r)).collect();
    taper(m, &path)
}

/// As [`tube`], each ring of its own radius: a reducer is a cone.
pub(crate) fn taper(m: &mut TriMesh, path: &[(DVec3, f64)]) {
    if path.len() < 2 {
        return;
    }
    let (path, radii): (Vec<DVec3>, Vec<f64>) = path.iter().copied().unzip();
    let path = path.as_slice();
    let dir = |i: usize| {
        let (lo, hi) = (i.saturating_sub(1), (i + 1).min(path.len() - 1));
        (path[hi] - path[lo]).normalize_or_zero()
    };
    let (mut u, _) = across(dir(0));
    let base = m.positions.len() as u32;
    for (i, c) in path.iter().enumerate() {
        let d = dir(i);
        let r = radii[i];
        // carried along the path, so the rings don't twist
        u = (u - d * u.dot(d)).normalize_or(across(d).0);
        let v = d.cross(u);
        for k in 0..SIDES {
            let t = k as f64 / SIDES as f64 * std::f64::consts::TAU;
            let n = u * t.cos() + v * t.sin();
            m.positions.push((*c + n * r).as_vec3().to_array());
            m.normals.push(n.as_vec3().to_array());
        }
    }
    let s = SIDES as u32;
    for i in 0..path.len() as u32 - 1 {
        for k in 0..s {
            let j = (k + 1) % s;
            let (a0, a1, b0, b1) = (
                base + i * s + k,
                base + i * s + j,
                base + (i + 1) * s + k,
                base + (i + 1) * s + j,
            );
            m.indices.extend([a0, a1, b1, a0, b1, b0]);
        }
    }
}

/// A ball of radius `r` at `c`: what an elbow or a tee is in 3D.
pub(crate) fn ball(m: &mut TriMesh, c: DVec3, r: f64) {
    const RINGS: usize = 8;
    let base = m.positions.len() as u32;
    for i in 0..=RINGS {
        let lat = i as f64 / RINGS as f64 * std::f64::consts::PI;
        for k in 0..SIDES {
            let lon = k as f64 / SIDES as f64 * std::f64::consts::TAU;
            let n = DVec3::new(lat.sin() * lon.cos(), lat.sin() * lon.sin(), lat.cos());
            m.positions.push((c + n * r).as_vec3().to_array());
            m.normals.push(n.as_vec3().to_array());
        }
    }
    let s = SIDES as u32;
    for i in 0..RINGS as u32 {
        for k in 0..s {
            let j = (k + 1) % s;
            let (a0, a1, b0, b1) = (
                base + i * s + k,
                base + i * s + j,
                base + (i + 1) * s + k,
                base + (i + 1) * s + j,
            );
            m.indices.extend([a0, b1, a1, a0, b0, b1]);
        }
    }
}

/// How long a reducer from `big` to `small` (outside diameters, mm) is
/// drawn, m.
fn taper_len(big: f64, small: f64) -> f64 {
    ((big - small) * 1.5).max(30.0) / 1000.0
}

/// The pipe's solid, insulation included, with what joins it to the
/// pipes at its ends: the arc it is bent in, or a ball where it is
/// fitted to them.
pub fn pipe_mesh(model: &Model, p: &Pipe) -> TriMesh {
    let (mut a, mut b) = ends(model, p);
    let mut m = TriMesh::default();
    if a.distance(b) < 1e-9 {
        return m;
    }
    let ins = p.insulation_mm / 1000.0;
    let (w, h) = p.section_m();
    let round = p.height_mm.is_none();
    let js = joints(model, p);
    if round {
        let r = w / 2.0 + ins;
        // narrower than the largest pipe at an end: a cone from its size
        let red = creabim_services::fittings::reductions_of(model, p);
        let along = (b - a).normalize_or_zero();
        for (i, big) in red.iter().enumerate() {
            let (Some(big), false) = (big, matches!(js[i], Joint::Bend { .. })) else {
                continue;
            };
            let (node, out) = if i == 0 { (a, along) } else { (b, -along) };
            let rb = big / 2000.0 + ins;
            let l = taper_len(*big, p.outer_mm).min(a.distance(b) / 2.0);
            taper(&mut m, &[(node, rb), (node + out * l, r)]);
        }
        for (i, j) in js.iter().enumerate() {
            let node = if i == 0 { a } else { b };
            match j {
                Joint::Bend { at, arc, owner } => {
                    if *owner {
                        tube(&mut m, arc, r);
                    }
                    if i == 0 {
                        a = *at;
                    } else {
                        b = *at;
                    }
                }
                Joint::Elbow { owner: true, .. } | Joint::Node { owner: true } => {
                    ball(&mut m, node, r)
                }
                _ => {}
            }
        }
    }
    let d = (b - a).normalize_or_zero();
    if d == DVec3::ZERO {
        return m;
    }
    let (u, v) = across(d);
    match p.height_mm {
        Some(_) => {
            let (hw, hh) = (w / 2.0 + ins, h / 2.0 + ins);
            let corner = |s: f64, t: f64| u * s * hw + v * t * hh;
            let ring = [
                corner(-1.0, -1.0),
                corner(1.0, -1.0),
                corner(1.0, 1.0),
                corner(-1.0, 1.0),
            ];
            for i in 0..4 {
                let (c0, c1) = (ring[i], ring[(i + 1) % 4]);
                let n = (c0 + c1).normalize();
                m.add_quad([a + c0, a + c1, b + c1, b + c0], n);
            }
            m.add_polygon(&ring.map(|c| a + c), &[[0, 1, 2], [0, 2, 3]], -d);
            m.add_polygon(&ring.map(|c| b + c), &[[0, 1, 2], [0, 2, 3]], d);
        }
        None => {
            let r = w / 2.0 + ins;
            let ring: Vec<DVec3> = (0..SIDES)
                .map(|i| {
                    let t = i as f64 / SIDES as f64 * std::f64::consts::TAU;
                    (u * t.cos() + v * t.sin()) * r
                })
                .collect();
            // smooth sides: a normal at each vertex
            let base = m.positions.len() as u32;
            for c in &ring {
                let n = c.normalize().as_vec3().to_array();
                for end in [a, b] {
                    m.positions.push((end + *c).as_vec3().to_array());
                    m.normals.push(n);
                }
            }
            for i in 0..SIDES as u32 {
                let j = (i + 1) % SIDES as u32;
                let (a0, b0, a1, b1) = (
                    base + 2 * i,
                    base + 2 * i + 1,
                    base + 2 * j,
                    base + 2 * j + 1,
                );
                m.indices.extend([a0, a1, b1, a0, b1, b0]);
            }
            let fan: Vec<[usize; 3]> = (1..SIDES - 1).map(|i| [0, i, i + 1]).collect();
            let cap = |end: DVec3| ring.iter().map(|c| end + *c).collect::<Vec<_>>();
            m.add_polygon(&cap(a), &fan, -d);
            m.add_polygon(&cap(b), &fan, d);
        }
    }
    m
}

/// The pipe in a plan cut `cut` above its level: its outline in plan,
/// washed in its system's colour, in the lightest line where it is cut
/// or seen below, dashed above. Its ends are closed only where it meets no
/// other pipe: at a corner its sides mitre into the other's, a bent pipe
/// turns in its arc.
pub fn pipe_plan(model: &Model, e: &Element, p: &Pipe, cut: f64) -> Vec<PlanItem> {
    let mut items = Vec::new();
    let (a, b) = (p.start, p.end);
    let (w, _) = p.section_m();
    let half = w / 2.0 + p.insulation_mm / 1000.0;
    let (a2, b2) = (a.truncate(), b.truncate());
    let run = b2 - a2;
    let lo = a.z.min(b.z) - half;
    let hi = a.z.max(b.z) + half;
    // the lightest line, cut or seen below: a heavier one would blot a
    // pipe a few millimetres wide at a drawing's scale
    let pen = if hi <= cut || lo < cut {
        Pen::Fine
    } else {
        Pen::Hidden
    };
    let line = |items: &mut Vec<PlanItem>, a: DVec2, b: DVec2| {
        items.push(PlanItem::Line {
            id: e.id,
            pen,
            a,
            b,
        })
    };
    // a riser shows as its section: a square or a circle round its axis
    if run.length() < 1e-6 {
        let pts: Vec<DVec2> = (0..SIDES)
            .map(|i| {
                let t = i as f64 / SIDES as f64 * std::f64::consts::TAU;
                a2 + DVec2::new(t.cos(), t.sin()) * half
            })
            .collect();
        items.push(PlanItem::Shade {
            id: e.id,
            points: pts.clone(),
            rgb: color(p),
        });
        for i in 0..pts.len() {
            line(&mut items, pts[i], pts[(i + 1) % pts.len()]);
        }
        return items;
    }
    // joints are found in the model's heights; the plan is level
    let z = model.level(p.level).map_or(0.0, |l| l.elevation);
    let flat = |q: DVec3| (q - DVec3::Z * z).truncate();
    let js = joints(model, p);
    let d = run.normalize();
    let n = d.perp() * half;
    // each end's two corners (right of the run, left of it), whether it
    // is closed, and the arc drawn from it
    let mut corners = [[a2 - n, a2 + n], [b2 - n, b2 + n]];
    let mut closed = [true, true];
    let mut arcs: Vec<Vec<DVec2>> = vec![];
    for (i, j) in js.iter().enumerate() {
        // out of the pipe, at this end
        let out = if i == 0 { -d } else { d };
        match j {
            Joint::Free => {}
            Joint::Node { .. } => closed[i] = false,
            Joint::Bend { at, arc, owner } => {
                let c = flat(*at);
                corners[i] = [c - n, c + n];
                closed[i] = false;
                if *owner {
                    arcs.push(arc.iter().map(|q| flat(*q)).collect());
                }
            }
            Joint::Elbow { to, .. } => {
                let to = to.truncate();
                // a riser there, or nearly back on itself: closed as it is
                let Some(to) = to.try_normalize() else {
                    continue;
                };
                let turn = out.perp_dot(to);
                let cos = out.dot(to);
                if cos < -0.94 {
                    continue;
                }
                // the sides meet the other pipe's on the line that halves
                // the corner: the inner side shorter, the outer longer
                let k = half * turn / (1.0 + cos) * if i == 0 { -1.0 } else { 1.0 };
                let node = if i == 0 { a2 } else { b2 };
                corners[i] = [node - n + out * k, node + n - out * k];
                closed[i] = false;
            }
        }
    }
    // narrower than the largest pipe at an end: a solid triangle there,
    // pointing into this pipe (STN EN ISO 6412)
    let red = creabim_services::fittings::reductions_of(model, p);
    for (i, big) in red.iter().enumerate() {
        let (Some(big), false) = (big, matches!(js[i], Joint::Bend { .. })) else {
            continue;
        };
        let (node, into) = if i == 0 { (a2, d) } else { (b2, -d) };
        let nb = d.perp() * (big / 2000.0 + p.insulation_mm / 1000.0);
        let tip = node + into * taper_len(*big, p.outer_mm).min(run.length() / 2.0);
        // the pipe's own corners are square again: the triangle covers them
        corners[i] = [node - n, node + n];
        closed[i] = false;
        items.push(PlanItem::Shade {
            id: e.id,
            points: vec![node - nb, tip - n, tip + n, node + nb],
            rgb: color(p),
        });
        line(&mut items, node - nb, tip - n);
        line(&mut items, node + nb, tip + n);
        line(&mut items, node - nb, node + nb);
    }
    let [[ar, al], [br, bl]] = corners;
    items.push(PlanItem::Shade {
        id: e.id,
        points: vec![ar, br, bl, al],
        rgb: color(p),
    });
    line(&mut items, ar, br);
    line(&mut items, al, bl);
    if closed[0] {
        line(&mut items, ar, al);
    }
    if closed[1] {
        line(&mut items, br, bl);
    }
    for arc in arcs {
        // the arc's two sides, each point set off square to the way it runs
        let side = |s: f64| -> Vec<DVec2> {
            (0..arc.len())
                .map(|i| {
                    let (lo, hi) = (i.saturating_sub(1), (i + 1).min(arc.len() - 1));
                    let t = (arc[hi] - arc[lo]).normalize_or_zero();
                    arc[i] + t.perp() * half * s
                })
                .collect()
        };
        let (r, l) = (side(-1.0), side(1.0));
        let mut points = r.clone();
        points.extend(l.iter().rev());
        items.push(PlanItem::Shade {
            id: e.id,
            points,
            rgb: color(p),
        });
        for s in [&r, &l] {
            for q in s.windows(2) {
                line(&mut items, q[0], q[1]);
            }
        }
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::{ElementKind, Level, Transaction};

    fn model_with(p: impl FnOnce(creabim_core::ElementId) -> Pipe) -> (Model, Element) {
        let mut m = Model::new();
        let l = Element::new(
            "Ground",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let e = Element::new("Pipe", ElementKind::Pipe(p(l.id)));
        Transaction::new("pipe")
            .insert(l)
            .insert(e.clone())
            .apply(&mut m)
            .unwrap();
        (m, e)
    }

    fn pipe(level: creabim_core::ElementId, height_mm: Option<f64>) -> Pipe {
        Pipe {
            level,
            system: "SV".into(),
            start: DVec3::new(0.0, 0.0, 0.3),
            end: DVec3::new(2.0, 0.0, 0.3),
            pipe_type: "HERZ".into(),
            nominal_mm: 16.0,
            outer_mm: 20.0,
            inner_mm: 16.0,
            height_mm,
            insulation_mm: 0.0,
        }
    }

    #[test]
    fn a_pipe_is_a_closed_cylinder_in_its_systems_colour() {
        let (m, e) = model_with(|l| pipe(l, None));
        let ElementKind::Pipe(p) = &e.kind else {
            unreachable!()
        };
        let mesh = pipe_mesh(&m, p);
        assert!(mesh.is_closed());
        let (lo, hi) = mesh.bounds().unwrap();
        assert!((hi.z - lo.z - 0.02).abs() < 1e-3 && (hi.x - 2.0).abs() < 1e-6);
        assert!((mesh.volume() - std::f64::consts::PI * 0.01 * 0.01 * 2.0).abs() < 2e-5);
        assert_eq!(
            color(p),
            [
                0x2f as f32 / 255.0,
                0x6f as f32 / 255.0,
                0xde as f32 / 255.0
            ]
        );
    }

    #[test]
    fn a_duct_is_a_box() {
        let (m, e) = model_with(|l| {
            let mut p = pipe(l, Some(200.0));
            p.nominal_mm = 400.0;
            p
        });
        let ElementKind::Pipe(p) = &e.kind else {
            unreachable!()
        };
        let mesh = pipe_mesh(&m, p);
        assert!(mesh.is_closed());
        assert!((mesh.volume() - 0.4 * 0.2 * 2.0).abs() < 1e-6);
    }

    /// Two pipes of `kind` meeting at (2, 0) at a right angle, and the
    /// first of them.
    fn corner(kind: &str) -> (Model, Element) {
        let (mut m, e) = model_with(|l| {
            let mut p = pipe(l, None);
            p.pipe_type = kind.into();
            p
        });
        let ElementKind::Pipe(p) = &e.kind else {
            unreachable!()
        };
        let mut q = p.clone();
        q.start = p.end;
        q.end = DVec3::new(2.0, 3.0, 0.3);
        Transaction::new("second")
            .insert(Element::new("Pipe", ElementKind::Pipe(q)))
            .apply(&mut m)
            .unwrap();
        (m, e)
    }

    fn lines(items: &[PlanItem]) -> Vec<(DVec2, DVec2)> {
        items
            .iter()
            .filter_map(|i| match i {
                PlanItem::Line { a, b, .. } => Some((*a, *b)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn rigid_pipes_mitre_at_a_corner() {
        let (m, e) = corner("PPR");
        let ElementKind::Pipe(p) = &e.kind else {
            unreachable!()
        };
        let js = joints(&m, p);
        assert_eq!(js[0], Joint::Free);
        assert!(matches!(js[1], Joint::Elbow { .. }));
        // closed at its free start only; at the corner the outer side
        // runs on by half the width and the inner stops as short
        let l = lines(&pipe_plan(&m, &e, p, 1.2));
        assert_eq!(l.len(), 3);
        let ends: Vec<DVec2> = l.iter().take(2).map(|(_, b)| *b).collect();
        assert!(
            (ends[0] - DVec2::new(2.01, -0.01)).length() < 1e-9,
            "{ends:?}"
        );
        assert!(
            (ends[1] - DVec2::new(1.99, 0.01)).length() < 1e-9,
            "{ends:?}"
        );
        // in 3D one of the two has the elbow's ball
        let other = m
            .iter()
            .find_map(|x| match &x.kind {
                ElementKind::Pipe(q) if x.id != e.id => Some(q.clone()),
                _ => None,
            })
            .unwrap();
        let balls = [p, &other]
            .iter()
            .filter(|q| pipe_mesh(&m, q).positions.len() > 200)
            .count();
        assert_eq!(balls, 1);
    }

    #[test]
    fn a_pipe_from_a_coil_turns_in_an_arc() {
        // multilayer pipe, 20 mm outside: bent at 5 d = 100 mm
        let (m, e) = corner("HERZ");
        let ElementKind::Pipe(p) = &e.kind else {
            unreachable!()
        };
        let js = joints(&m, p);
        let Joint::Bend { at, arc, .. } = &js[1] else {
            panic!("{js:?}")
        };
        assert!((*at - DVec3::new(1.9, 0.0, 0.3)).length() < 1e-9);
        assert!((*arc.last().unwrap() - DVec3::new(2.0, 0.1, 0.3)).length() < 1e-9);
        // every point of the arc is 100 mm from its centre
        let c = DVec3::new(1.9, 0.1, 0.3);
        assert!(arc.iter().all(|q| (q.distance(c) - 0.1).abs() < 1e-9));
        // the straight part stops where the arc begins, and is not closed there
        let l = lines(&pipe_plan(&m, &e, p, 1.2));
        assert!((l[0].1.x - 1.9).abs() < 1e-9 && (l[1].1.x - 1.9).abs() < 1e-9);
        // too short an arm for the arc: a fitting, as rigid pipe
        let (mut m, e) = corner("HERZ");
        let other = m
            .iter()
            .find(|x| x.id != e.id && matches!(x.kind, ElementKind::Pipe(_)))
            .unwrap()
            .clone();
        let mut short = other.clone();
        if let ElementKind::Pipe(q) = &mut short.kind {
            q.end = DVec3::new(2.0, 0.15, 0.3);
        }
        Transaction::new("short")
            .replace(other, short)
            .apply(&mut m)
            .unwrap();
        assert!(matches!(joints(&m, p)[1], Joint::Elbow { .. }));
    }

    #[test]
    fn a_tee_leaves_the_ends_open() {
        let (mut m, e) = corner("PPR");
        let ElementKind::Pipe(p) = &e.kind else {
            unreachable!()
        };
        let mut on = p.clone();
        on.start = p.end;
        on.end = DVec3::new(4.0, 0.0, 0.3);
        Transaction::new("on")
            .insert(Element::new("Pipe", ElementKind::Pipe(on)))
            .apply(&mut m)
            .unwrap();
        assert!(matches!(joints(&m, p)[1], Joint::Node { .. }));
        // two sides and the cap of the free start
        assert_eq!(lines(&pipe_plan(&m, &e, p, 1.2)).len(), 3);
    }

    #[test]
    fn a_pipe_is_cut_or_seen_in_plan() {
        let (m, e) = model_with(|l| pipe(l, None));
        let ElementKind::Pipe(p) = &e.kind else {
            unreachable!()
        };
        let pen = |cut| match pipe_plan(&m, &e, p, cut)[1] {
            PlanItem::Line { pen, .. } => pen,
            _ => unreachable!(),
        };
        assert_eq!(pen(0.3), Pen::Fine);
        assert_eq!(pen(1.2), Pen::Fine);
        assert_eq!(pen(0.1), Pen::Hidden);
        assert!(matches!(
            pipe_plan(&m, &e, p, 1.2)[0],
            PlanItem::Shade { .. }
        ));
    }
}
