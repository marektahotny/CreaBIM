//! In sections, footings are filled with their material, and the ground
//! is cut by what is built in it: a thin line where the two meet, the
//! thick one only where the ground meets air.

use creabim_core::{Element, ElementKind, Hatch, Model, Site};
use creabim_geom::plan::{Pen, PlanItem};
use glam::{DVec2, DVec3};

fn with_ground() -> (Model, creabim_core::ElementId) {
    let m = creabim_core::starter::starter_house();
    let level = m.iter().find(|e| e.name == "Ground Floor").unwrap().id;
    let rect = vec![
        DVec2::new(-20.0, -20.0),
        DVec2::new(30.0, -20.0),
        DVec2::new(30.0, 30.0),
        DVec2::new(-20.0, 30.0),
    ];
    // sloping, so the walls stand in it on one side
    let mut ground = Site::terrain(level, rect, -0.2, 2.0);
    if let creabim_core::SiteForm::Terrain { points, .. } = &mut ground.form {
        points.push(DVec3::new(6.6, 12.0, 0.4));
    }
    let g = Element::new("Ground", ElementKind::Site(ground));
    let gid = g.id;
    (
        Model::from_elements(m.iter().cloned().chain([g])).unwrap(),
        gid,
    )
}

fn point_in(p: DVec2, poly: &[DVec2]) -> bool {
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

fn on_edge(p: DVec2, poly: &[DVec2]) -> bool {
    let n = poly.len();
    (0..n).any(|i| {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let d = b - a;
        let t = (p - a).dot(d) / d.length_squared();
        (0.0..=1.0).contains(&t) && (a + d * t).distance(p) < 1e-5
    })
}

#[test]
fn footings_are_filled_and_the_ground_meets_them_with_a_thin_line() {
    let (m, ground) = with_ground();
    let view = m.iter().find(|e| e.name == "Section A-A").unwrap().id;
    let d = creabim_geom::section::view_drawing(&m, view).unwrap();
    let footing = |id| matches!(m.get(id).map(|e| &e.kind), Some(ElementKind::Footing(_)));
    let footings: Vec<&Vec<DVec2>> = d
        .items
        .iter()
        .filter_map(|i| match i {
            PlanItem::Fill {
                id, points, hatch, ..
            } if footing(*id) => {
                assert_ne!(*hatch, Hatch::Empty, "a cut footing is filled");
                Some(points)
            }
            _ => None,
        })
        .collect();
    assert!(!footings.is_empty(), "the section cuts footings");
    let grounds: Vec<&Vec<DVec2>> = d
        .items
        .iter()
        .filter_map(|i| match i {
            PlanItem::Fill { id, points, .. } if *id == ground => Some(points),
            _ => None,
        })
        .collect();
    assert!(!grounds.is_empty(), "the ground is cut");
    // the ground leaves the footings out
    for f in &footings {
        let c = f.iter().copied().sum::<DVec2>() / f.len() as f64;
        assert!(
            !grounds.iter().any(|g| point_in(c, g)),
            "ground over a footing at {c}"
        );
    }
    // no thick line where the ground meets a footing
    for i in &d.items {
        if let PlanItem::Line {
            pen: Pen::Cut,
            a,
            b,
            ..
        } = i
        {
            let mid = (*a + *b) / 2.0;
            let in_ground = grounds.iter().any(|g| on_edge(mid, g));
            let in_footing = footings.iter().any(|f| on_edge(mid, f));
            assert!(
                !(in_ground && in_footing),
                "thick line between ground and footing at {mid}"
            );
        }
    }
}

#[test]
#[ignore]
fn dump_ground_section() {
    let (m, _) = with_ground();
    let view = m.iter().find(|e| e.name == "Section A-A").unwrap().id;
    let d = creabim_geom::section::view_drawing(&m, view).unwrap();
    std::fs::write(std::env::var("OUT").unwrap(), d.to_svg(100.0)).unwrap();
}
