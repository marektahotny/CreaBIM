//! The material take-off counts every piece of material once: where two
//! elements overlap in 3D, the one the drawings let run through keeps the
//! overlap and the other gives it up (footing under a wall, a beam in a
//! wall, a roof over a slab, a ceiling running into a wall, walls meeting
//! at corners and T-junctions).

use creabim_core::{
    document::Document, Beam, Ceiling, Element, ElementId, ElementKind, Footing, Level, Model,
    Roof, RoofShape, Slab, Transaction, Wall,
};
use creabim_geom::scene::{boolean_mesh, build_scene, closed_volume, SceneItem};
use glam::DVec2;

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<DVec2> {
    vec![
        DVec2::new(x0, y0),
        DVec2::new(x1, y0),
        DVec2::new(x1, y1),
        DVec2::new(x0, y1),
    ]
}

struct Built {
    model: Model,
    wall: ElementId,
    footing: ElementId,
    beam: ElementId,
    slab: ElementId,
    roof: ElementId,
    ceiling: ElementId,
}

fn built() -> Built {
    let l0 = Element::new(
        "L0",
        ElementKind::Level(Level {
            elevation: 0.0,
            height: 3.0,
        }),
    );
    let l1 = Element::new(
        "L1",
        ElementKind::Level(Level {
            elevation: 3.0,
            height: 3.0,
        }),
    );
    // of one material each, not layered: measured all the same
    let brick = Element::new(
        "Brick",
        ElementKind::Material(creabim_core::Material::new(
            creabim_core::Hatch::Diagonal,
            [0.8, 0.45, 0.35],
        )),
    );
    let mut w = Wall::new(l0.id, DVec2::ZERO, DVec2::new(6.0, 0.0), 0.3, 3.0);
    w.material = Some(brick.id);
    let wall = Element::new("Wall", ElementKind::Wall(w));
    // a strip footing reaching 0.1 up into the wall's foot
    let mut f = Footing::strip(l0.id, DVec2::new(0.0, 0.0), DVec2::new(6.0, 0.0));
    f.base_offset = Some(-0.2);
    f.material = Some(brick.id);
    let footing = Element::new("Footing", ElementKind::Footing(f));
    // a beam lying in the wall, its top 0.5 under the storey's top
    let mut b = Beam::new(l0.id, DVec2::new(1.0, 0.0), DVec2::new(4.0, 0.0));
    b.width = 0.2;
    b.depth = 0.3;
    b.top_offset = 0.5;
    let beam = Element::new("Beam", ElementKind::Beam(b));
    // a ceiling drawn across the wall
    let ceiling = Element::new(
        "Ceiling",
        ElementKind::Ceiling(Ceiling::drawn(l0.id, rect(0.5, -1.0, 5.5, 1.0), 2.0)),
    );
    let slab = Element::new(
        "Slab",
        ElementKind::Slab(Slab::new(l1.id, rect(0.0, -2.0, 6.0, 2.0), 0.25)),
    );
    // a flat roof sunk into the slab
    let mut r = Roof::new(l1.id, rect(1.0, -1.0, 5.0, 1.0), RoofShape::Flat);
    r.overhang = 0.0;
    r.base_offset = -0.1;
    let roof = Element::new("Roof", ElementKind::Roof(r));
    let ids = (wall.id, footing.id, beam.id, slab.id, roof.id, ceiling.id);
    let mut doc = Document::new();
    let mut tx = Transaction::new("t");
    for e in [l0, l1, brick, wall, footing, beam, ceiling, slab, roof] {
        tx = tx.insert(e);
    }
    doc.commit(tx).unwrap();
    Built {
        model: doc.model().clone(),
        wall: ids.0,
        footing: ids.1,
        beam: ids.2,
        slab: ids.3,
        roof: ids.4,
        ceiling: ids.5,
    }
}

fn volume_of(items: &[SceneItem], id: ElementId) -> f64 {
    items
        .iter()
        .filter(|i| i.id == id && !i.ghost)
        .map(|i| closed_volume(&i.mesh))
        .sum()
}

/// The volume shared by any two items of different elements.
fn overlaps(model: &Model, items: &[SceneItem]) -> Vec<(String, String, f64)> {
    let live: Vec<&SceneItem> = items
        .iter()
        .filter(|i| !i.ghost && !i.is_lining() && !i.mesh.is_empty())
        .filter(|i| {
            model
                .get(i.id)
                .is_some_and(|e| creabim_core::joins::joinable(&e.kind))
        })
        .collect();
    let mut out = vec![];
    for (n, a) in live.iter().enumerate() {
        for b in &live[n + 1..] {
            if a.id == b.id {
                continue;
            }
            let (Some(ba), Some(bb)) = (a.mesh.bounds(), b.mesh.bounds()) else {
                continue;
            };
            if (ba.1.min(bb.1) - ba.0.max(bb.0)).min_element() <= 1e-4 {
                continue;
            }
            let v = closed_volume(&boolean_mesh(
                &a.mesh,
                &b.mesh,
                creabim_core::BoolOp::Intersection,
            ));
            // a litre is below what the take-off shows (m³ to 3 places)
            if v > 1e-3 {
                let name = |id| model.get(id).map_or(String::new(), |e| e.name.clone());
                out.push((
                    format!("{} {:?}", name(a.id), a.layer.map(|l| l.function)),
                    format!("{} {:?}", name(b.id), b.layer.map(|l| l.function)),
                    v,
                ));
            }
        }
    }
    out
}

#[test]
fn footing_beam_roof_and_ceiling_give_up_their_overlaps_once() {
    let b = built();
    let raw = creabim_geom::scene::build_scene_for_exchange(&b.model);
    let joined = build_scene(&b.model);
    let v = |items: &[SceneItem], id| volume_of(items, id);
    let near = |a: f64, b: f64| (a - b).abs() < 1e-3;
    // the wall keeps all of itself but the beam lying in it
    assert!(near(v(&raw, b.wall), 5.4));
    assert!(
        near(v(&joined, b.wall), 5.4 - 3.0 * 0.2 * 0.3),
        "{}",
        v(&joined, b.wall)
    );
    assert!(near(v(&joined, b.beam), v(&raw, b.beam)));
    // the footing gives the wall its top 0.1
    let f = v(&raw, b.footing);
    assert!(
        near(v(&joined, b.footing), f - 6.0 * 0.3 * 0.1),
        "{}",
        v(&joined, b.footing)
    );
    // the slab gives the wall its 0.25, the roof gives the slab its 0.1
    assert!(near(v(&joined, b.slab), v(&raw, b.slab) - 6.0 * 0.3 * 0.25));
    assert!(near(v(&joined, b.roof), v(&raw, b.roof) - 4.0 * 2.0 * 0.1));
    // the ceiling stops at the wall's faces
    assert!(near(
        v(&joined, b.ceiling),
        v(&raw, b.ceiling) - 5.0 * 0.3 * 0.0125
    ));
    // and the take-off says the same
    let lines = creabim_geom::takeoff::lines(&b.model);
    for (id, want) in [
        (b.wall, v(&joined, b.wall)),
        (b.footing, v(&joined, b.footing)),
        (b.slab, v(&joined, b.slab)),
        (b.roof, v(&joined, b.roof)),
        (b.ceiling, v(&joined, b.ceiling)),
        (b.beam, v(&joined, b.beam)),
    ] {
        let got: f64 = lines
            .iter()
            .filter(|l| l.element == id)
            .map(|l| l.quantity.volume)
            .sum();
        assert!(
            near(got, want),
            "{:?}: take-off {got}, solid {want}",
            b.model.get(id).map(|e| &e.name)
        );
    }
    let left = overlaps(&b.model, &joined);
    assert!(left.is_empty(), "overlaps left: {left:?}");
}

#[test]
fn the_starter_house_has_no_overlapping_solids() {
    let m = creabim_core::starter::starter_house();
    let left = overlaps(&m, &build_scene(&m));
    assert!(left.is_empty(), "overlaps left: {left:?}");
}
