//! The building in a site plan as the custom has it: only its outer walls
//! (and the columns standing outside its rooms), cut through and filled
//! solid dark grey, without plaster, cleanly joined; no openings, no
//! inner walls, no pipes, no section or elevation marks. What lies about
//! it on the site (terrain, networks, shafts, marks) stays as drawn.

use crate::plan::{Pen, PlanItem};
use creabim_core::assembly::{wall_bands, LayerFunction};
use creabim_core::{ElementId, ElementKind, Model};
use glam::DVec2;
use std::collections::HashSet;

/// The grey the building is filled with.
pub const GREY: [f32; 3] = [0.35, 0.35, 0.35];

/// Does the segment a–b cross the ray from `p` along `d`?
fn ray_hits(p: DVec2, d: DVec2, a: DVec2, b: DVec2) -> bool {
    let e = b - a;
    let den = d.perp_dot(e);
    if den.abs() < 1e-12 {
        return false;
    }
    let w = a - p;
    let t = w.perp_dot(e) / den;
    let u = w.perp_dot(d) / den;
    t > 1e-6 && (-1e-9..=1.0 + 1e-9).contains(&u)
}

/// The outer walls of `level`: those from which, somewhere along them,
/// one can look away from the building past every other wall of the
/// level (rooms or none).
pub fn outer_walls(model: &Model, level: ElementId) -> Vec<ElementId> {
    let walls: Vec<(ElementId, &creabim_core::Wall)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Wall(w) if w.level == level && w.start.distance(w.end) > 1e-6 => {
                Some((e.id, w))
            }
            _ => None,
        })
        .collect();
    // each wall as its two faces, so a ray from inside one is stopped by
    // the walls round it but not by itself
    let mut out = vec![];
    for (id, w) in &walls {
        let dir = (w.end - w.start).normalize();
        let n = DVec2::new(-dir.y, dir.x);
        let open = [0.1, 0.3, 0.5, 0.7, 0.9].iter().any(|k| {
            let mid = w.start.lerp(w.end, *k);
            [1.0, -1.0].iter().any(|side| {
                let p = mid + n * side * (w.thickness / 2.0 + 0.01);
                [-60.0f64, -30.0, 0.0, 30.0, 60.0].iter().any(|deg| {
                    let d = DVec2::from_angle(deg.to_radians()).rotate(n * *side);
                    !walls.iter().filter(|(o, _)| o != id).any(|(_, o)| {
                        // a little past its ends, so a ray doesn't slip
                        // through the corner where two walls meet
                        let along = (o.end - o.start).normalize() * o.thickness;
                        ray_hits(p, d, o.start - along, o.end + along)
                    })
                })
            })
        });
        if open {
            out.push(*id);
        }
    }
    out
}

/// The level whose outer walls make the footprint: the view's own, else
/// the one nearest ±0,000 that has walls.
fn footprint_level(model: &Model, own: ElementId) -> Option<ElementId> {
    let has = |l: ElementId| {
        model
            .iter()
            .any(|e| matches!(&e.kind, ElementKind::Wall(w) if w.level == l))
    };
    if has(own) {
        return Some(own);
    }
    let mut levels: Vec<(f64, ElementId)> = model
        .levels()
        .map(|(e, l)| (l.elevation.abs(), e.id))
        .filter(|(_, id)| has(*id))
        .collect();
    levels.sort_by(|a, b| a.0.total_cmp(&b.0));
    levels.first().map(|l| l.1)
}

/// A wall without its finishes: its thickness and the offset of its
/// middle from its centreline (left of start → end positive).
fn bare(model: &Model, w: &creabim_core::Wall) -> (f64, f64) {
    let asm = w.assembly.and_then(|a| match &model.get(a)?.kind {
        ElementKind::Assembly(a) if !a.layers.is_empty() && a.total() > 1e-9 => Some(a),
        _ => None,
    });
    let Some(a) = asm else {
        return (w.thickness, 0.0);
    };
    let bands: Vec<_> = wall_bands(a, w)
        .into_iter()
        .filter(|b| b.function != LayerFunction::Finish)
        .collect();
    if bands.is_empty() {
        return (w.thickness, 0.0);
    }
    let lo = bands
        .iter()
        .map(|b| b.s0.min(b.s1))
        .fold(f64::MAX, f64::min);
    let hi = bands
        .iter()
        .map(|b| b.s0.max(b.s1))
        .fold(f64::MIN, f64::max);
    (hi - lo, (lo + hi) / 2.0)
}

/// The footprint of level `own` (see [`footprint_level`]) as plan items:
/// solid grey fills under clean cut outlines.
pub fn footprint(model: &Model, own: ElementId, cut_height: f64) -> Vec<PlanItem> {
    let Some(level) = footprint_level(model, own) else {
        return vec![];
    };
    let walls: HashSet<ElementId> = outer_walls(model, level).into_iter().collect();
    let columns: HashSet<ElementId> = model
        .iter()
        .filter(|e| {
            matches!(&e.kind, ElementKind::Column(c)
                if c.level == level && !crate::firezones::room_at(model, level, c.position))
        })
        .map(|e| e.id)
        .collect();
    // a copy with only them, the walls bare, of one stuff and drawn on
    // their centrelines, so they join cleanly without openings
    let mut m = model.clone();
    m.retain(|e| match &e.kind {
        ElementKind::Wall(_) => walls.contains(&e.id),
        ElementKind::Column(_) => columns.contains(&e.id),
        ElementKind::Opening(_)
        | ElementKind::Slab(_)
        | ElementKind::Roof(_)
        | ElementKind::Beam(_)
        | ElementKind::Stair(_)
        | ElementKind::Railing(_)
        | ElementKind::Object(_)
        | ElementKind::Footing(_)
        | ElementKind::Room(_)
        | ElementKind::RoomSeparator(_)
        | ElementKind::Ceiling(_)
        | ElementKind::Pipe(_)
        | ElementKind::Valve(_)
        | ElementKind::FreeMesh(_)
        | ElementKind::Part(_)
        | ElementKind::Site(_)
        | ElementKind::Sketch(_)
        | ElementKind::Detail(_)
        | ElementKind::Shaft(_)
        | ElementKind::Grid(_)
        | ElementKind::GridGroup(_)
        | ElementKind::Dimension(_)
        | ElementKind::Tag(_)
        | ElementKind::Note(_)
        | ElementKind::Reference(_)
        | ElementKind::View(_) => false,
        _ => true,
    });
    let ids: Vec<ElementId> = walls.iter().copied().collect();
    let mut bare_walls = vec![];
    for id in ids {
        let Some(e) = model.get(id) else { continue };
        let ElementKind::Wall(w) = &e.kind else {
            continue;
        };
        let (t, mid) = bare(model, w);
        let dir = (w.end - w.start).normalize_or_zero();
        let n = DVec2::new(-dir.y, dir.x);
        let centre = creabim_core::walls::location_offset(model, w);
        let shift = n * (mid - centre);
        let mut e = e.clone();
        if let ElementKind::Wall(x) = &mut e.kind {
            x.start += shift;
            x.end += shift;
            x.thickness = t;
            x.assembly = None;
            x.material = None;
            x.location = creabim_core::LocationLine::Centre;
            x.layer_ends.clear();
            x.curtain = None;
        }
        bare_walls.push(e);
    }
    // walls that met still meet: each end moved to where the moved
    // lines cross
    let orig: Vec<[DVec2; 2]> = bare_walls
        .iter()
        .map(|e| match model.get(e.id).map(|x| &x.kind) {
            Some(ElementKind::Wall(w)) => [w.start, w.end],
            _ => [DVec2::ZERO; 2],
        })
        .collect();
    let lines: Vec<[DVec2; 2]> = bare_walls
        .iter()
        .map(|e| match &e.kind {
            ElementKind::Wall(w) => [w.start, w.end],
            _ => [DVec2::ZERO; 2],
        })
        .collect();
    for i in 0..bare_walls.len() {
        for end in 0..2 {
            let p = orig[i][end];
            let Some(j) =
                (0..orig.len()).find(|&j| j != i && orig[j].iter().any(|q| q.distance(p) < 1e-3))
            else {
                continue;
            };
            let (a, b) = (lines[i], lines[j]);
            let (d1, d2) = (a[1] - a[0], b[1] - b[0]);
            let den = d1.perp_dot(d2);
            if den.abs() < 1e-9 {
                continue;
            }
            let t = (b[0] - a[0]).perp_dot(d2) / den;
            let x = a[0] + d1 * t;
            if let ElementKind::Wall(w) = &mut bare_walls[i].kind {
                if end == 0 {
                    w.start = x;
                } else {
                    w.end = x;
                }
            }
        }
    }
    m.retain(|e| !matches!(e.kind, ElementKind::Wall(_)));
    let mut elements: Vec<_> = m.iter().cloned().collect();
    elements.extend(bare_walls);
    // what pointed at the elements left out goes too (a tag on a door)
    let m = loop {
        match Model::from_elements(elements.clone()) {
            Ok(m) => break m,
            Err(creabim_core::ModelError::DanglingReference { element, .. })
                if elements.len() > 1 =>
            {
                elements.retain(|e| e.id != element);
            }
            Err(_) => return vec![],
        }
    };
    let d = crate::plan::plan_view(&m, level, cut_height);
    let mut fills = vec![];
    let mut lines = vec![];
    for it in d.items {
        match it {
            PlanItem::Fill { id, points, .. } => fills.push(PlanItem::Shade {
                id,
                points,
                rgb: GREY,
            }),
            PlanItem::Line {
                pen: Pen::Cut,
                id,
                a,
                b,
            } => lines.push(PlanItem::Line {
                id,
                pen: Pen::Cut,
                a,
                b,
            }),
            PlanItem::Polygon {
                pen: Pen::Cut,
                id,
                points,
            } => {
                fills.push(PlanItem::Shade {
                    id,
                    points: points.clone(),
                    rgb: GREY,
                });
                lines.push(PlanItem::Polygon {
                    id,
                    pen: Pen::Cut,
                    points,
                });
            }
            _ => {}
        }
    }
    fills.extend(lines);
    fills
}

/// Does the site plan leave element `id` out: the building's parts, its
/// pipes, its openings, the section and elevation marks, objects inside
/// its rooms and the terrain's mesh? Site objects, paving, earthworks,
/// networks and notes stay.
pub fn hides(model: &Model, id: ElementId) -> bool {
    let Some(e) = model.get(id) else {
        return false;
    };
    match &e.kind {
        ElementKind::Wall(_)
        | ElementKind::Opening(_)
        | ElementKind::Slab(_)
        | ElementKind::Roof(_)
        | ElementKind::Column(_)
        | ElementKind::Beam(_)
        | ElementKind::Stair(_)
        | ElementKind::Railing(_)
        | ElementKind::Footing(_)
        | ElementKind::Room(_)
        | ElementKind::RoomSeparator(_)
        | ElementKind::Ceiling(_)
        | ElementKind::Pipe(_)
        | ElementKind::Valve(_)
        | ElementKind::Part(_)
        | ElementKind::Shaft(_)
        | ElementKind::View(_) => true,
        ElementKind::Object(o) => crate::firezones::room_at(model, o.level, o.position),
        // the terrain's mesh: it is modelled in its own plan (3D terén),
        // the site plan has the map's cadastre and contours
        ElementKind::Site(_) => is_terrain(model, id),
        _ => false,
    }
}

/// Is `id` the terrain's mesh (a map's 3D terrain)?
pub fn is_terrain(model: &Model, id: ElementId) -> bool {
    matches!(model.get(id).map(|e| &e.kind),
        Some(ElementKind::Site(s)) if matches!(s.form, creabim_core::site::SiteForm::Terrain { .. }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::{Transaction, View};

    /// The starter house's site plan: its outer walls filled grey, no
    /// window or door, no inner wall.
    #[test]
    fn a_site_plan_shows_the_outer_walls_only() {
        let mut m = creabim_core::starter::starter_house();
        let ground = m
            .levels()
            .min_by(|a, b| a.1.elevation.abs().total_cmp(&b.1.elevation.abs()))
            .map(|(e, _)| e.id)
            .unwrap();
        let mut v = View::plan(ground);
        v.graphics.range.from_above = true;
        v.graphics.range.footprint = true;
        v.scale = 200.0;
        let e = creabim_core::Element::new("Site plan", ElementKind::View(v));
        let id = e.id;
        Transaction::new("site").insert(e).apply(&mut m).unwrap();
        // a room's tag in it (rooms showed their names and areas there)
        let room = m
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Room(_)))
            .unwrap()
            .id;
        let tag = creabim_core::Element::new(
            "Room tag",
            ElementKind::Tag(creabim_core::Tag::new(id, room, glam::DVec2::ZERO)),
        );
        let tag_id = tag.id;
        Transaction::new("tag").insert(tag).apply(&mut m).unwrap();
        let d = crate::section::view_drawing(&m, id).unwrap();
        assert!(
            d.items.iter().all(|it| it.id() != tag_id),
            "a room's tag shows"
        );
        let outer: HashSet<ElementId> = outer_walls(&m, ground).into_iter().collect();
        let all = m
            .iter()
            .filter(|e| matches!(&e.kind, ElementKind::Wall(w) if w.level == ground))
            .count();
        assert!(
            !outer.is_empty() && outer.len() < all,
            "{} of {all}",
            outer.len()
        );
        let grey: HashSet<ElementId> = d
            .items
            .iter()
            .filter_map(|it| match it {
                PlanItem::Shade { id, rgb, .. } if *rgb == GREY => Some(*id),
                _ => None,
            })
            .collect();
        assert_eq!(grey, outer);
        // nothing of the building is drawn but them
        assert!(d.items.iter().all(|it| {
            let id = it.id();
            outer.contains(&id) || !hides(&m, id)
        }));
        if let Ok(dir) = std::env::var("CREABIM_SITE_SVG") {
            std::fs::write(dir, d.to_svg(40.0)).unwrap();
        }
    }
}
