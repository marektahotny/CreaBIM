//! Pipes and ducts in 3D and in plan: a pipe a cylinder (a rectangular
//! duct a box) along its axis, in its system's colour; in plan its
//! outline, washed in that colour, cut or seen below as other members.

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

/// The pipe's solid, insulation included.
pub fn pipe_mesh(model: &Model, p: &Pipe) -> TriMesh {
    let (a, b) = ends(model, p);
    let mut m = TriMesh::default();
    let d = (b - a).normalize_or_zero();
    if d == DVec3::ZERO {
        return m;
    }
    let (u, v) = across(d);
    let ins = p.insulation_mm / 1000.0;
    let (w, h) = p.section_m();
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
/// washed in its system's colour, cut where it crosses the plane, seen
/// below it, or dashed above.
pub fn pipe_plan(model: &Model, e: &Element, p: &Pipe, cut: f64) -> Vec<PlanItem> {
    let mut items = Vec::new();
    let (a, b) = (p.start, p.end);
    let (w, _) = p.section_m();
    let half = w / 2.0 + p.insulation_mm / 1000.0;
    let (a2, b2) = (a.truncate(), b.truncate());
    let run = b2 - a2;
    // a riser shows as its section: a square or a circle round its axis
    let pts: Vec<DVec2> = if run.length() < 1e-6 {
        (0..SIDES)
            .map(|i| {
                let t = i as f64 / SIDES as f64 * std::f64::consts::TAU;
                a2 + DVec2::new(t.cos(), t.sin()) * half
            })
            .collect()
    } else {
        let n = run.normalize().perp() * half;
        vec![a2 - n, b2 - n, b2 + n, a2 + n]
    };
    let lo = a.z.min(b.z) - half;
    let hi = a.z.max(b.z) + half;
    let pen = if lo < cut && cut < hi {
        Pen::Cut
    } else if hi <= cut {
        Pen::Projection
    } else {
        Pen::Hidden
    };
    let _ = model;
    items.push(PlanItem::Shade {
        id: e.id,
        points: pts.clone(),
        rgb: color(p),
    });
    for i in 0..pts.len() {
        items.push(PlanItem::Line {
            id: e.id,
            pen,
            a: pts[i],
            b: pts[(i + 1) % pts.len()],
        });
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
        assert_eq!(pen(0.3), Pen::Cut);
        assert_eq!(pen(1.2), Pen::Projection);
        assert_eq!(pen(0.1), Pen::Hidden);
        assert!(matches!(
            pipe_plan(&m, &e, p, 1.2)[0],
            PlanItem::Shade { .. }
        ));
    }
}
