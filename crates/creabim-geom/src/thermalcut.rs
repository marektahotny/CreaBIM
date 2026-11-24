//! What a section cuts, as the 2D model of a construction detail for its
//! temperature field (STN EN ISO 10211): the cut pieces as the section
//! draws them, each with its material, and where the rooms' air stands in
//! the cut, inside a box (a detail's callout, or one drawn on the section).
//! Lengths are in metres along the section line and up (the elevation).

use crate::plan::CutPiece;
use crate::scene::SceneItem;
use crate::section::SectionFrame;
use creabim_core::{ElementId, ElementKind, Model, ViewKind};
use glam::DVec2;

/// One piece of cut material.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    /// The element it is cut from.
    pub element: ElementId,
    /// Its material (a layer's, else the element's); none for an air gap
    /// or an element without one.
    pub material: Option<ElementId>,
    /// An air gap layer of an assembly.
    pub air_gap: bool,
    pub points: Vec<DVec2>,
}

/// A room's air in the cut: a rectangle along the line and up.
#[derive(Clone, Debug, PartialEq)]
pub struct RoomAir {
    pub room: ElementId,
    pub lo: DVec2,
    pub hi: DVec2,
}

/// The cut of a detail.
#[derive(Clone, Debug, PartialEq)]
pub struct ThermalCut {
    pub pieces: Vec<Piece>,
    pub rooms: Vec<RoomAir>,
    /// The box the detail is cut to.
    pub lo: DVec2,
    pub hi: DVec2,
}

/// Keep what lies inside the box `lo`..`hi` (Sutherland–Hodgman).
pub fn clip_box(poly: &[DVec2], lo: DVec2, hi: DVec2) -> Vec<DVec2> {
    let mut out = poly.to_vec();
    let planes: [(DVec2, f64); 4] = [
        (DVec2::X, lo.x),
        (-DVec2::X, -hi.x),
        (DVec2::Y, lo.y),
        (-DVec2::Y, -hi.y),
    ];
    for (n, c) in planes {
        if out.len() < 3 {
            return vec![];
        }
        let inside = |p: DVec2| p.dot(n) >= c - 1e-12;
        let mut next = Vec::with_capacity(out.len() + 2);
        for i in 0..out.len() {
            let (a, b) = (out[i], out[(i + 1) % out.len()]);
            let (ia, ib) = (inside(a), inside(b));
            if ia {
                next.push(a);
            }
            if ia != ib {
                let (da, db) = (a.dot(n) - c, b.dot(n) - c);
                next.push(a + (b - a) * (da / (da - db)));
            }
        }
        out = next;
    }
    if out.len() < 3 {
        vec![]
    } else {
        out
    }
}

/// The section a view is cut along: its own line, or for a detail called
/// out of a section that section's, with the callout's box.
/// A box: its lower and upper corner.
type Box2 = (DVec2, DVec2);

fn section_of(model: &Model, view: ElementId) -> Option<(DVec2, DVec2, Option<Box2>)> {
    let ElementKind::View(v) = &model.get(view)?.kind else {
        return None;
    };
    let (kind, boxed) = match v.callout {
        Some(c) => match &model.get(c.parent)?.kind {
            ElementKind::View(p) => (&p.kind, Some((c.lo, c.hi))),
            _ => return None,
        },
        None => (&v.kind, None),
    };
    match kind {
        ViewKind::Section { start, end, cut: true, .. } => Some((*start, *end, boxed)),
        _ => None,
    }
}

/// Is `view` a section, or a detail called out of one, that a thermal
/// detail can be cut from?
pub fn is_cuttable(model: &Model, view: ElementId) -> bool {
    section_of(model, view).is_some()
}

/// The detail cut by section view `view` (or a detail called out of a
/// section) inside the box `lo`..`hi`, in the section's coordinates;
/// without a box the callout's box.
pub fn cut(model: &Model, view: ElementId, boxed: Option<(DVec2, DVec2)>) -> Option<ThermalCut> {
    let (start, end, callout) = section_of(model, view)?;
    let (lo, hi) = boxed.or(callout)?;
    let (lo, hi) = (lo.min(hi), lo.max(hi));
    let frame = SectionFrame::new(start, end, 0.0);
    let shared = crate::scene::shared_scene(model);
    // the construction only: no fixtures, furniture, pipes or railings
    let built = |id: ElementId| {
        !matches!(
            model.get(id).map(|e| &e.kind),
            Some(
                ElementKind::Object(_)
                    | ElementKind::Pipe(_)
                    | ElementKind::Valve(_)
                    | ElementKind::Railing(_)
                    | ElementKind::Circuit(_)
            ) | None
        )
    };
    let items: Vec<&SceneItem> = shared.iter().filter(|i| !i.ghost && !i.is_lining() && built(i.id)).collect();
    let mut cuts = vec![];
    for s in items {
        for loop_ in crate::section::cut_loops(s, &frame) {
            let pts: Vec<DVec2> = loop_.iter().map(|p| DVec2::new(p.x, p.y)).collect();
            if pts.iter().all(|p| p.x < lo.x) || pts.iter().all(|p| p.x > hi.x) {
                continue;
            }
            if pts.iter().all(|p| p.y < lo.y) || pts.iter().all(|p| p.y > hi.y) {
                continue;
            }
            cuts.push(CutPiece { id: s.id, points: pts, layer: s.layer });
        }
    }
    let resolved = crate::plan::resolve_cuts(model, cuts, true);
    let mut pieces = vec![];
    for c in resolved {
        let pts = clip_box(&c.points, lo, hi);
        if pts.len() < 3 {
            continue;
        }
        let (material, air_gap) = match c.layer {
            Some(l) => (
                l.material.filter(|m| model.get(*m).is_some()),
                l.function == creabim_core::LayerFunction::AirGap,
            ),
            None => (
                model.get(c.id).and_then(|e| creabim_core::material_of(model, e)).map(|(m, _)| m.id),
                false,
            ),
        };
        pieces.push(Piece { element: c.id, material, air_gap, points: pts });
    }
    // the rooms the line runs through, from their floor to their ceiling
    let mut rooms = vec![];
    let along = (end - start).normalize_or_zero();
    for e in model.iter() {
        let ElementKind::Room(r) = &e.kind else {
            continue;
        };
        let Some(level) = model.level(r.level) else {
            continue;
        };
        let Some(outline) = creabim_core::room::outline(model, e.id) else {
            continue;
        };
        let z0 = level.elevation;
        let z1 = z0 + creabim_core::room::height(model, r);
        for (a, b) in line_in(&outline, start, end) {
            let (x0, x1) = ((a - start).dot(along), (b - start).dot(along));
            rooms.push(RoomAir {
                room: e.id,
                lo: DVec2::new(x0.min(x1), z0 - 0.01),
                hi: DVec2::new(x0.max(x1), z1 + 0.01),
            });
        }
    }
    Some(ThermalCut { pieces, rooms, lo, hi })
}

/// The stretches of the line `a`–`b` inside polygon `poly`.
fn line_in(poly: &[DVec2], a: DVec2, b: DVec2) -> Vec<(DVec2, DVec2)> {
    let d = b - a;
    let mut ts = vec![0.0, 1.0];
    for i in 0..poly.len() {
        let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
        let e = q - p;
        let den = d.perp_dot(e);
        if den.abs() < 1e-12 {
            continue;
        }
        let t = (p - a).perp_dot(e) / den;
        let s = (p - a).perp_dot(d) / den;
        if (0.0..=1.0).contains(&s) && (0.0..=1.0).contains(&t) {
            ts.push(t);
        }
    }
    ts.sort_by(|x, y| x.total_cmp(y));
    ts.dedup_by(|x, y| (*x - *y).abs() < 1e-9);
    ts.windows(2)
        .filter(|w| creabim_core::polygon::contains(poly, a + d * (0.5 * (w[0] + w[1]))))
        .map(|w| (a + d * w[0], a + d * w[1]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clips_to_box() {
        let sq = vec![DVec2::new(-1.0, -1.0), DVec2::new(2.0, -1.0), DVec2::new(2.0, 2.0), DVec2::new(-1.0, 2.0)];
        let c = clip_box(&sq, DVec2::ZERO, DVec2::ONE);
        let area: f64 = (0..c.len()).map(|i| c[i].perp_dot(c[(i + 1) % c.len()])).sum::<f64>() / 2.0;
        assert!((area.abs() - 1.0).abs() < 1e-12);
        assert!(clip_box(&sq, DVec2::splat(5.0), DVec2::splat(6.0)).is_empty());
    }

    #[test]
    fn line_through_square() {
        let sq = vec![DVec2::new(0.0, 0.0), DVec2::new(2.0, 0.0), DVec2::new(2.0, 2.0), DVec2::new(0.0, 2.0)];
        let s = line_in(&sq, DVec2::new(-1.0, 1.0), DVec2::new(3.0, 1.0));
        assert_eq!(s.len(), 1);
        assert!((s[0].0.x - 0.0).abs() < 1e-9 && (s[0].1.x - 2.0).abs() < 1e-9);
    }
}
