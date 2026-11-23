//! The structural engineer's drawings (Statika, George 2026-11-23): what
//! carries the building and the positions its lintels and ring beams are
//! listed by, as the office's "výkres tvaru" marks them: P1, P2… for the
//! lintels over the openings (one position for openings with the same
//! lintels), V1, V2… for the ring beams by their section.

use crate::masonry::{self, RING_BEAM};
use crate::{Element, ElementId, ElementKind, Model};

/// One lintel position: the openings it lies over, alike.
#[derive(Clone, Debug, PartialEq)]
pub struct LintelPosition {
    /// `P1`.
    pub mark: String,
    /// What lies over each opening: `3× NP 1800/100`.
    pub label: String,
    /// The pieces' length, m.
    pub length: f64,
    /// The underside's height above ±0,000, m.
    pub underside: f64,
    /// The storey's name.
    pub storey: String,
    /// Each opening's lintel pieces (a piece with no opening its own).
    pub groups: Vec<Vec<ElementId>>,
}

/// One ring beam position: ring beams of one section.
#[derive(Clone, Debug, PartialEq)]
pub struct RingPosition {
    /// `V1`.
    pub mark: String,
    /// Width and depth, m.
    pub width: f64,
    pub depth: f64,
    /// Running length along the walls' axes, m.
    pub length: f64,
    pub beams: Vec<ElementId>,
}

/// Is `e` a ring beam (stužujúci veniec)?
pub fn is_ring_beam(e: &Element) -> bool {
    matches!(e.kind, ElementKind::Beam(_)) && e.name == RING_BEAM
}

fn mm(v: f64) -> i64 {
    (v * 1000.0).round() as i64
}

/// The lintel positions of the model, lowest storey first, then by what
/// lies over the opening and its length.
pub fn lintel_positions(model: &Model) -> Vec<LintelPosition> {
    // pieces by the opening they span
    let mut groups: Vec<(Option<ElementId>, Vec<ElementId>)> = vec![];
    for e in model.iter() {
        let ElementKind::Beam(b) = &e.kind else {
            continue;
        };
        let Some(l) = &b.lintel else { continue };
        let key = l.opening.or(Some(e.id));
        match groups.iter_mut().find(|g| g.0 == key) {
            Some(g) => g.1.push(e.id),
            None => groups.push((key, vec![e.id])),
        }
    }
    // (elevation, label, length mm, storey, underside, ids)
    let mut out: Vec<(f64, LintelPosition)> = vec![];
    for (_, ids) in groups {
        let Some(ElementKind::Beam(b)) = model.get(ids[0]).map(|e| &e.kind) else {
            continue;
        };
        let label = masonry::label(model, &ids);
        let length = ids
            .iter()
            .filter_map(|id| match model.get(*id).map(|e| &e.kind) {
                Some(ElementKind::Beam(b)) => Some(b.length()),
                _ => None,
            })
            .fold(0.0, f64::max);
        let underside = ids
            .iter()
            .filter_map(|id| match model.get(*id).map(|e| &e.kind) {
                Some(ElementKind::Beam(b)) => Some(b.top(model) - b.depth),
                _ => None,
            })
            .fold(f64::MAX, f64::min);
        let (z, storey) = model
            .get(b.level)
            .map_or((0.0, String::new()), |lv| match &lv.kind {
                ElementKind::Level(x) => (x.elevation, lv.name.clone()),
                _ => (0.0, lv.name.clone()),
            });
        let same = |p: &LintelPosition| {
            p.storey == storey
                && p.label == label
                && mm(p.length) == mm(length)
                && mm(p.underside) == mm(underside)
        };
        match out.iter_mut().find(|(_, p)| same(p)) {
            Some((_, p)) => p.groups.push(ids),
            None => out.push((
                z,
                LintelPosition {
                    mark: String::new(),
                    label,
                    length,
                    underside,
                    storey,
                    groups: vec![ids],
                },
            )),
        }
    }
    out.sort_by(|a, b| {
        a.0.total_cmp(&b.0)
            .then_with(|| mm(a.1.length).cmp(&mm(b.1.length)))
            .then_with(|| a.1.label.cmp(&b.1.label))
    });
    out.into_iter()
        .enumerate()
        .map(|(i, (_, mut p))| {
            p.mark = format!("P{}", i + 1);
            p
        })
        .collect()
}

/// The ring beam positions: one for each section, widest and deepest
/// first as they come.
pub fn ring_positions(model: &Model) -> Vec<RingPosition> {
    let mut out: Vec<RingPosition> = vec![];
    for e in model.iter() {
        if !is_ring_beam(e) {
            continue;
        }
        let ElementKind::Beam(b) = &e.kind else {
            continue;
        };
        match out
            .iter_mut()
            .find(|p| mm(p.width) == mm(b.width) && mm(p.depth) == mm(b.depth))
        {
            Some(p) => {
                p.length += b.length();
                p.beams.push(e.id);
            }
            None => out.push(RingPosition {
                mark: String::new(),
                width: b.width,
                depth: b.depth,
                length: b.length(),
                beams: vec![e.id],
            }),
        }
    }
    out.sort_by(|a, b| {
        mm(b.width)
            .cmp(&mm(a.width))
            .then_with(|| mm(b.depth).cmp(&mm(a.depth)))
    });
    for (i, p) in out.iter_mut().enumerate() {
        p.mark = format!("V{}", i + 1);
    }
    out
}

/// Does the structural drawing draw `e`: what carries (load-bearing
/// walls, columns, beams, slabs, footings, stairs) and what is written
/// on a drawing; partitions, doors and windows, furniture, fixtures,
/// rooms, pipes and the site are left out.
pub fn draws(model: &Model, e: &Element, rules: &masonry::MasonryRules) -> bool {
    match &e.kind {
        ElementKind::Wall(w) => w.curtain.is_none() && masonry::is_bearing(model, w, rules),
        ElementKind::Column(_)
        | ElementKind::Beam(_)
        | ElementKind::Slab(_)
        | ElementKind::Footing(_)
        | ElementKind::Stair(_)
        | ElementKind::Grid(_)
        | ElementKind::Level(_)
        | ElementKind::Dimension(_)
        | ElementKind::Note(_)
        | ElementKind::Guide(_) => true,
        // a tag on what is drawn
        ElementKind::Tag(t) => model.get(t.host).is_some_and(|h| draws(model, h, rules)),
        // the marks of the structural sections only
        ElementKind::View(v) => v.structural,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Beam, Level, Transaction};
    use glam::DVec2;

    #[test]
    fn ring_beams_of_one_section_are_one_position_with_their_length_summed() {
        let mut m = Model::new();
        let lv = Element::new(
            "1. NP",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let l = lv.id;
        let ring = |a: DVec2, b: DVec2, w: f64| {
            let mut x = Beam::new(l, a, b);
            x.width = w;
            x.depth = 0.25;
            Element::new(RING_BEAM, ElementKind::Beam(x))
        };
        Transaction::new("t")
            .insert(lv)
            .insert(ring(DVec2::ZERO, DVec2::new(10.0, 0.0), 0.3))
            .insert(ring(DVec2::new(10.0, 0.0), DVec2::new(10.0, 6.0), 0.3))
            .insert(ring(DVec2::new(5.0, 0.0), DVec2::new(5.0, 6.0), 0.2))
            .apply(&mut m)
            .unwrap();
        let p = ring_positions(&m);
        assert_eq!(p.len(), 2);
        assert_eq!((p[0].mark.as_str(), p[0].width), ("V1", 0.3));
        assert!((p[0].length - 16.0).abs() < 1e-9);
        assert_eq!((p[1].mark.as_str(), p[1].beams.len()), ("V2", 1));
    }
}
