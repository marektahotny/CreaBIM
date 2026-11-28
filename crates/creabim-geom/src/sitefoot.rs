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
    // the columns outside the rooms with their insulation, not its render
    let outside = |c: &creabim_core::Column| {
        c.level == level && !crate::firezones::room_at(model, level, c.position)
    };
    let columns: HashSet<ElementId> = model
        .iter()
        .filter(|e| match &e.kind {
            ElementKind::Column(c) => match &c.piece {
                None => outside(c),
                Some(p) => {
                    !p.plaster
                        && matches!(model.get(p.host).map(|h| &h.kind),
                            Some(ElementKind::Column(h)) if outside(h))
                }
            },
            _ => false,
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
        // start and end are the centreline whatever the location line
        let shift = n * mid;
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
    // the roof over it: its outline, a very thin line of a dash and two dots
    let look = creabim_core::LineLook {
        pen: creabim_core::LinePen::VeryThin,
        kind: creabim_core::LineType::DashDotDot,
    };
    for e in model.iter() {
        if let ElementKind::Roof(r) = &e.kind {
            let pts = r.eave_outline();
            for i in 0..pts.len() {
                fills.push(PlanItem::Line {
                    id: e.id,
                    pen: Pen::Drafted(look),
                    a: pts[i],
                    b: pts[(i + 1) % pts.len()],
                });
            }
        }
    }
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

/// The corners and ends of level `level`'s bare outer walls, where their
/// faces meet (every pairing of the two walls' faces, in one order), with
/// each wall's bare middle laid `extra(w)` off where it truly is (zero:
/// as drawn now).
fn bare_corners(
    model: &Model,
    level: ElementId,
    extra: impl Fn(&creabim_core::Wall) -> f64,
) -> Vec<DVec2> {
    let walls: Vec<&creabim_core::Wall> = outer_walls(model, level)
        .into_iter()
        .filter_map(|id| match &model.get(id)?.kind {
            ElementKind::Wall(w) => Some(w),
            _ => None,
        })
        .collect();
    // each wall's two bare faces
    let faces: Vec<[[DVec2; 2]; 2]> = walls
        .iter()
        .map(|w| {
            let (t, mid) = bare(model, w);
            let n = (w.end - w.start).normalize_or_zero().perp();
            [1.0, -1.0].map(|side| {
                let s = n * (mid + extra(w) + side * t / 2.0);
                [w.start + s, w.end + s]
            })
        })
        .collect();
    let cross = |a: [DVec2; 2], b: [DVec2; 2]| {
        let (d1, d2) = (a[1] - a[0], b[1] - b[0]);
        let den = d1.perp_dot(d2);
        (den.abs() > 1e-9).then(|| a[0] + d1 * ((b[0] - a[0]).perp_dot(d2) / den))
    };
    let mut out = vec![];
    for i in 0..walls.len() {
        for end in 0..2 {
            let p = if end == 0 {
                walls[i].start
            } else {
                walls[i].end
            };
            // the wall it meets: the one whose end is nearest, within a
            // wall's thickness (walls drawn to each other's faces too)
            let near = |j: usize| walls[j].start.distance(p).min(walls[j].end.distance(p));
            let joined = (0..walls.len())
                .filter(|&j| j != i && near(j) < walls[i].thickness.max(walls[j].thickness))
                .min_by(|&a, &b| near(a).total_cmp(&near(b)));
            for fi in 0..2 {
                for fj in 0..2 {
                    let at = joined
                        .and_then(|j| cross(faces[i][fi], faces[j][fj]))
                        .unwrap_or(faces[i][fi][end]);
                    out.push(at);
                }
            }
        }
    }
    out
}

/// The site plans' dimensions and S-JTSK marks put on the building's
/// corners while its footprint was drawn off by the walls' location line
/// (before 0.6-8), moved onto the corners where they truly are. The
/// changed dimensions; none once they sit right.
pub fn mend_marks(model: &Model) -> Vec<creabim_core::Element> {
    use creabim_core::ViewKind;
    let mut moves: Vec<(DVec2, DVec2)> = vec![];
    let mut site_views = HashSet::new();
    let mut levels = HashSet::new();
    for e in model.iter() {
        if let ElementKind::View(v) = &e.kind {
            if let ViewKind::Plan { level, .. } = &v.kind {
                if v.graphics.range.is_site() {
                    site_views.insert(e.id);
                    if let Some(l) = footprint_level(model, *level) {
                        levels.insert(l);
                    }
                }
            }
        }
    }
    for l in levels {
        let now = bare_corners(model, l, |_| 0.0);
        let was = bare_corners(model, l, |w| {
            -creabim_core::walls::location_offset(model, w)
        });
        for (a, b) in was.into_iter().zip(now) {
            if a.distance(b) > 1e-4 {
                moves.push((a, b));
            }
        }
    }
    if moves.is_empty() {
        return vec![];
    }
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Dimension(d) = &e.kind else {
            continue;
        };
        if !site_views.contains(&d.view) {
            continue;
        }
        let Some(ElementKind::View(view)) = model.get(d.view).map(|v| &v.kind) else {
            continue;
        };
        let mut nd = d.clone();
        let mut changed = false;
        let mut fix = |p: &mut creabim_core::HostPoint| {
            let Some(at) = crate::annotate::resolve(model, view, p) else {
                return;
            };
            let Some((_, to)) = moves.iter().find(|(from, _)| from.distance(at) < 2e-3) else {
                return;
            };
            let delta = *to - at;
            match p.host.and_then(|h| model.get(h)).map(|h| &h.kind) {
                None => p.local += delta,
                Some(ElementKind::Wall(w)) => {
                    let dir = w.direction();
                    p.local += DVec2::new(delta.dot(dir), delta.dot(dir.perp()));
                }
                Some(ElementKind::Opening(o)) => {
                    let Some(ElementKind::Wall(w)) = model.get(o.host).map(|h| &h.kind) else {
                        return;
                    };
                    let dir = w.direction();
                    p.local += DVec2::new(delta.dot(dir), delta.dot(dir.perp()));
                }
                _ => return,
            }
            changed = true;
        };
        fix(&mut nd.a);
        fix(&mut nd.b);
        for m in nd.mids.iter_mut() {
            fix(m);
        }
        if changed {
            let mut ne = e.clone();
            ne.kind = ElementKind::Dimension(nd);
            out.push(ne);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::{Transaction, View};

    /// The starter house's site plan: its outer walls filled grey, no
    /// window or door, no inner wall.
    fn bbox(items: &[PlanItem]) -> (glam::DVec2, glam::DVec2) {
        let (mut lo, mut hi) = (glam::DVec2::splat(1e9), glam::DVec2::splat(-1e9));
        for it in items {
            if let PlanItem::Shade { points, .. } = it {
                for p in points {
                    lo = lo.min(*p);
                    hi = hi.max(*p);
                }
            }
        }
        (lo, hi)
    }

    /// George's site plan drew the house 77,5 mm in on every side: its
    /// walls kept on the outer face of their masonry, the footprint took
    /// that line for their middle. Where a wall's location line is must
    /// not move it; marks put on the old corners go to the true ones.
    #[test]
    fn the_footprint_is_where_the_walls_are_whatever_their_location_line() {
        let mut m = creabim_core::starter::starter_house();
        let ground = m
            .levels()
            .min_by(|a, b| a.1.elevation.abs().total_cmp(&b.1.elevation.abs()))
            .map(|(e, _)| e.id)
            .unwrap();
        let want = bbox(&footprint(&m, ground, 1.2));
        let outer = outer_walls(&m, ground);
        let mut tx = Transaction::new("location");
        for id in &outer {
            let before = m.get(*id).unwrap().clone();
            let mut after = before.clone();
            if let ElementKind::Wall(w) = &mut after.kind {
                w.location = creabim_core::LocationLine::CoreExterior;
            }
            tx = tx.replace(before, after);
        }
        tx.apply(&mut m).unwrap();
        let got = bbox(&footprint(&m, ground, 1.2));
        assert!(
            got.0.distance(want.0) < 1e-6 && got.1.distance(want.1) < 1e-6,
            "{got:?} vs {want:?}"
        );
        // a site plan with a dimension between two corners as drawn before
        let mut v = View::plan(ground);
        v.graphics.range.from_above = true;
        v.graphics.range.footprint = true;
        let ve = creabim_core::Element::new("Site plan", ElementKind::View(v));
        let vid = ve.id;
        Transaction::new("site").insert(ve).apply(&mut m).unwrap();
        let was = bare_corners(&m, ground, |w| -creabim_core::walls::location_offset(&m, w));
        let now = bare_corners(&m, ground, |_| 0.0);
        let k = (0..was.len())
            .find(|&k| was[k].distance(now[k]) > 0.01)
            .expect("a corner drawn off");
        let dim = creabim_core::Element::new(
            "Dimension",
            ElementKind::Dimension(creabim_core::Dimension::linear(
                vid,
                creabim_core::HostPoint::free(was[k]),
                creabim_core::HostPoint::free(was[k] + glam::DVec2::new(5.0, 0.0)),
                1.0,
            )),
        );
        let did = dim.id;
        Transaction::new("dim").insert(dim).apply(&mut m).unwrap();
        let moved = mend_marks(&m);
        assert_eq!(moved.len(), 1);
        let ElementKind::Dimension(d) = &moved[0].kind else {
            panic!()
        };
        assert_eq!(moved[0].id, did);
        assert!(d.a.local.distance(now[k]) < 1e-9);
        // the true corner is a corner of the grey footprint
        let items = footprint(&m, ground, 1.2);
        assert!(items.iter().any(|it| matches!(it,
            PlanItem::Shade { points, .. } if points.iter().any(|p| p.distance(now[k]) < 1e-6))));
        // and once there it stays
        let before = m.get(did).unwrap().clone();
        Transaction::new("mend")
            .replace(before, moved[0].clone())
            .apply(&mut m)
            .unwrap();
        assert!(mend_marks(&m).is_empty());
    }

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
            outer.contains(&id)
                || !hides(&m, id)
                || matches!(m.get(id).map(|e| &e.kind), Some(ElementKind::Roof(_)))
        }));
        if let Ok(dir) = std::env::var("CREABIM_SITE_SVG") {
            std::fs::write(dir, d.to_svg(40.0)).unwrap();
        }
    }
}
