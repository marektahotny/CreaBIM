//! What the structural drawing (Statika) writes over its plan and its
//! lists: each lintel's outline over its opening with its position and
//! underside (`P1`, `SH=+2,400`), each ring beam's outline with its
//! position (`V1`), as the office's "výkres tvaru" (George 2026-11-23);
//! and the lists of lintels and ring beams by position.

use crate::paper::Anchor;
use crate::plan::{Pen, PlanItem};
use creabim_core::structural::{self, LintelPosition, RingPosition};
use creabim_core::{ElementId, ElementKind, Model};
use glam::DVec2;

fn comma(v: f64, places: usize) -> String {
    format!("{v:.places$}").replace('.', ",")
}

/// A height above ±0,000 as a drawing writes it: `+2,400`.
fn level_text(z: f64) -> String {
    let s = comma(z.abs(), 3);
    if z < -5e-4 {
        format!("-{s}")
    } else {
        format!("+{s}")
    }
}

fn outline(id: ElementId, pts: &[DVec2], out: &mut Vec<PlanItem>) {
    for k in 0..pts.len() {
        out.push(PlanItem::Line {
            id,
            pen: Pen::Overhead,
            a: pts[k],
            b: pts[(k + 1) % pts.len()],
        });
    }
}

/// The side of a lintel's wall its text goes on: the inside, and how far
/// from the lintel's middle the wall's face is.
fn inside_of(model: &Model, b: &creabim_core::Beam, mid: DVec2) -> (DVec2, f64) {
    let d = (b.end - b.start).normalize_or_zero();
    b.lintel
        .as_ref()
        .and_then(|l| l.opening)
        .and_then(|o| match model.get(o).map(|e| &e.kind) {
            Some(ElementKind::Opening(op)) => match model.get(op.host).map(|e| &e.kind) {
                Some(ElementKind::Wall(w)) => {
                    let left = w.direction().perp();
                    let inside = if w.flip { -left } else { left };
                    let centre =
                        (w.start + w.direction() * (op.offset + op.width / 2.0)).dot(inside);
                    Some((inside, centre + w.thickness / 2.0 - mid.dot(inside)))
                }
                _ => None,
            },
            _ => None,
        })
        .unwrap_or((d.perp(), b.width / 2.0))
}

/// The marks of the structural plan of `level`, `mm` drawing units a
/// paper millimetre.
pub fn plan_marks(model: &Model, level: ElementId, mm: f64) -> Vec<PlanItem> {
    let mut out = vec![];
    let beam = |id: &ElementId| match model.get(*id).map(|e| &e.kind) {
        Some(ElementKind::Beam(b)) => Some(b),
        _ => None,
    };
    for p in structural::lintel_positions(model) {
        for ids in &p.groups {
            let beams: Vec<&creabim_core::Beam> = ids.iter().filter_map(beam).collect();
            let Some(first) = beams.first() else { continue };
            if first.level != level {
                continue;
            }
            for (id, b) in ids.iter().zip(&beams) {
                outline(*id, &b.outline(), &mut out);
            }
            let mid = beams
                .iter()
                .map(|b| (b.start + b.end) / 2.0)
                .fold(DVec2::ZERO, |a, x| a + x)
                / beams.len() as f64;
            let (side, half_wall) = inside_of(model, first, mid);
            let at = mid + side * (half_wall + 5.0 * mm);
            out.push(PlanItem::Text {
                id: ids[0],
                at,
                text: p.mark.clone(),
                size: 2.5,
                align: Anchor::Middle,
                mm,
                rgb: None,
                bold: true,
            });
            out.push(PlanItem::Text {
                id: ids[0],
                at: at + side * 3.5 * mm,
                text: format!("SH={}", level_text(p.underside)),
                size: 1.8,
                align: Anchor::Middle,
                mm,
                rgb: None,
                bold: false,
            });
        }
    }
    for p in structural::ring_positions(model) {
        for id in &p.beams {
            let Some(b) = beam(id) else { continue };
            if b.level != level {
                continue;
            }
            // its position is written on the chain along its wall; the
            // beam drawn whole, seen from above on the core of its wall,
            // as the office's "výkres tvaru" draws it
            let pts = b.outline();
            for (k, a) in pts.iter().enumerate() {
                out.push(PlanItem::Line {
                    id: *id,
                    pen: Pen::Cut,
                    a: *a,
                    b: pts[(k + 1) % pts.len()],
                });
            }
        }
    }
    out
}

/// The walls of a structural plan as outlines, without their hatch, as
/// the office's "výkres tvaru" draws them; columns keep theirs.
pub fn outline_walls(model: &Model, items: &mut Vec<PlanItem>) {
    items.retain(|it| match it {
        PlanItem::Fill { id, .. } => {
            !matches!(model.get(*id).map(|e| &e.kind), Some(ElementKind::Wall(_)))
        }
        _ => true,
    });
}

/// Each column's mark by it: its position in a box and its top and
/// bottom (`S1`, `HH=+2,400`, `SH=-0,650`).
pub fn column_marks(model: &Model, level: ElementId, mm: f64) -> Vec<PlanItem> {
    let mut out = vec![];
    for p in structural::column_positions(model) {
        for id in &p.columns {
            let Some(ElementKind::Column(c)) = model.get(*id).map(|e| &e.kind) else {
                continue;
            };
            // its own storey's, and one standing on a level below that
            // runs up through this one (porch posts on the footings)
            if c.level != level && !c.crosses(model, level, 1.0) {
                continue;
            }
            let half = c
                .profile
                .outline(c.position, c.rotation)
                .iter()
                .map(|q| (*q - c.position).abs())
                .fold(DVec2::ZERO, |a, q| a.max(q));
            // up and to the left of it, clear of it
            // or where it was dragged to
            let at = c.position
                + DVec2::new(-half.x - 6.0 * mm, half.y + 6.0 * mm)
                + c.mark_at.unwrap_or(DVec2::ZERO);
            let w = (p.mark.chars().count() as f64 * 0.62 + 1.4) * 2.5 * mm;
            let h = 4.0 * mm;
            out.push(PlanItem::Polygon {
                id: *id,
                pen: Pen::Symbol,
                points: vec![
                    at + DVec2::new(-w / 2.0, -h / 2.0),
                    at + DVec2::new(w / 2.0, -h / 2.0),
                    at + DVec2::new(w / 2.0, h / 2.0),
                    at + DVec2::new(-w / 2.0, h / 2.0),
                ],
            });
            out.push(PlanItem::Text {
                id: *id,
                at,
                text: p.mark.clone(),
                size: 2.5,
                align: Anchor::Middle,
                mm,
                rgb: None,
                bold: true,
            });
            for (k, t) in [
                format!("HH={}", level_text(p.top)),
                format!("SH={}", level_text(p.bottom)),
            ]
            .into_iter()
            .enumerate()
            {
                out.push(PlanItem::Text {
                    id: *id,
                    at: at + DVec2::new(-w / 2.0 - 1.0 * mm, (0.9 - 2.2 * k as f64) * mm),
                    text: t,
                    size: 1.8,
                    align: Anchor::End,
                    mm,
                    rgb: None,
                    bold: false,
                });
            }
            // a leader to the column's corner
            out.push(PlanItem::Line {
                id: *id,
                pen: Pen::Symbol,
                a: at + DVec2::new(w / 2.0, -h / 2.0),
                b: c.position + DVec2::new(-half.x, half.y) * 0.5,
            });
        }
    }
    out
}

/// Along each load-bearing wall of `level` with lintels or a ring beam,
/// on its outer side, a chain of dimensions: each lintel's span written
/// with its position (`P1 800`), the stretches between with the ring
/// beam's (`V1 2720`), as the office's "výkres tvaru" does.
pub fn chains(model: &Model, level: ElementId, mm: f64) -> Vec<PlanItem> {
    let rules = creabim_core::masonry::rules(model);
    let lintels = structural::lintel_positions(model);
    let rings = structural::ring_positions(model);
    let look = crate::annotate::look_of(model, None, None);
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Wall(w) = &e.kind else {
            continue;
        };
        if w.level != level
            || w.is_arc()
            || w.curtain.is_some()
            || !creabim_core::masonry::is_bearing(model, w, &rules)
        {
            continue;
        }
        let d = w.direction();
        let len = w.length();
        let left = d.perp();
        let outward = if w.flip { left } else { -left };
        // the ring beam along it
        let ring = rings.iter().find_map(|p| {
            p.beams
                .iter()
                .find_map(|id| match model.get(*id).map(|x| &x.kind) {
                    Some(ElementKind::Beam(b)) if b.level == w.level => {
                        let off = |q: DVec2| (q - w.start).dot(left).abs();
                        let along = (b.end - b.start).normalize_or_zero().dot(d).abs();
                        (along > 0.999
                            && off(b.start) < w.thickness / 2.0 + 1e-6
                            && off(b.end) < w.thickness / 2.0 + 1e-6)
                            .then_some(p.mark.clone())
                    }
                    _ => None,
                })
        });
        // the lintels over its openings: their span along it
        let mut spans: Vec<(f64, f64, String)> = vec![];
        for p in &lintels {
            for g in &p.groups {
                let host = g
                    .iter()
                    .find_map(|id| match model.get(*id).map(|x| &x.kind) {
                        Some(ElementKind::Beam(b)) => {
                            b.lintel.as_ref().and_then(|l| l.opening).and_then(|o| {
                                match model.get(o).map(|x| &x.kind) {
                                    Some(ElementKind::Opening(op)) => Some(op.host),
                                    _ => None,
                                }
                            })
                        }
                        _ => None,
                    });
                if host != Some(e.id) {
                    continue;
                }
                let (lo, hi) = g
                    .iter()
                    .filter_map(|id| match model.get(*id).map(|x| &x.kind) {
                        Some(ElementKind::Beam(b)) => Some(b),
                        _ => None,
                    })
                    .flat_map(|b| [b.start, b.end])
                    .map(|q| (q - w.start).dot(d))
                    .fold((f64::MAX, f64::MIN), |(a, b), u| (a.min(u), b.max(u)));
                spans.push((lo.max(0.0), hi.min(len), p.mark.clone()));
            }
        }
        if spans.is_empty() && ring.is_none() {
            continue;
        }
        spans.sort_by(|a, b| a.0.total_cmp(&b.0));
        // the stretches: between the lintels, the ring beam's
        let mut cuts: Vec<(f64, f64, Option<String>)> = vec![];
        let mut u = 0.0;
        for (lo, hi, mark) in spans {
            if lo > u + 1e-3 {
                cuts.push((u, lo, ring.clone()));
            }
            cuts.push((lo.max(u), hi, Some(mark)));
            u = hi;
        }
        if len > u + 1e-3 {
            cuts.push((u, len, ring.clone()));
        }
        let face = outward * (w.thickness / 2.0);
        let at = |u: f64| w.start + d * u + face;
        for (lo, hi, mark) in cuts {
            let (mut a, mut b) = (at(lo), at(hi));
            // its line to the left of a → b, on the outer side
            if (b - a).perp().dot(outward) < 0.0 {
                std::mem::swap(&mut a, &mut b);
            }
            let n = format!("{:.0}", (hi - lo) * 1000.0);
            out.push(PlanItem::Dimension {
                id: e.id,
                a,
                b,
                offset: 8.0 * mm,
                text: match mark {
                    Some(m) => format!("{m} {n}"),
                    None => n,
                },
                look,
                witness: Some([a, b]),
                breaks: vec![],
                below: None,
                place: crate::plan::DimPlace::default(),
            });
        }
    }
    out
}

/// The doors and windows of the load-bearing walls of `level`, which the
/// structural drawing leaves as gaps: each a pick area over its gap, not
/// drawn, so a click in the gap takes the opening (to put its lintels,
/// George 2026-11-24).
pub fn opening_picks(model: &Model, level: ElementId) -> Vec<PlanItem> {
    let rules = creabim_core::masonry::rules(model);
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Opening(o) = &e.kind else {
            continue;
        };
        let Some(host) = model.get(o.host) else {
            continue;
        };
        let ElementKind::Wall(w) = &host.kind else {
            continue;
        };
        if w.level != level || w.is_arc() || !structural::draws(model, host, &rules) {
            continue;
        }
        let d = w.direction();
        let n = d.perp() * (w.thickness / 2.0);
        let a = w.start + d * o.offset;
        let b = a + d * o.width;
        out.push(PlanItem::Room {
            id: e.id,
            points: vec![a - n, b - n, b + n, a + n],
            rgb: None,
        });
    }
    out
}

/// Keep in a structural plan only what the structural drawing draws
/// (see [`structural::draws`]): openings stay as the gaps they leave.
pub fn keep_structure(model: &Model, items: &mut Vec<PlanItem>) {
    let rules = creabim_core::masonry::rules(model);
    let mut keep: std::collections::HashMap<ElementId, bool> = Default::default();
    items.retain(|it| {
        let id = it.id();
        *keep.entry(id).or_insert_with(|| match model.get(id) {
            // what has no element of its own (view marks, titles) stays
            None => true,
            // the insulation and render put round a column or lintel, and
            // the floors and ceilings, are not drawn: only what carries, as
            // the office's "výkres tvaru" (George 2026-11-24)
            Some(e) if creabim_core::wrap::piece_of(e).is_some() => false,
            Some(e) if matches!(e.kind, ElementKind::Slab(_) | ElementKind::Ceiling(_)) => false,
            Some(e) => structural::draws(model, e, &rules),
        })
    });
}

/// Is scene item `i` a part a structural section draws: a load-bearing
/// layer (or an element of one material), not the insulation, render,
/// plaster or floor build-up, not a window's glass, not the boards
/// wrapped round a column or lintel (George 2026-11-24, as the sections
/// of his výkres tvaru STA_002).
pub fn bearing_part(model: &Model, i: &crate::scene::SceneItem) -> bool {
    i.layer
        .is_none_or(|l| l.function == creabim_core::LayerFunction::Structure)
        && i.finish.is_none()
        && model
            .get(i.id)
            .is_none_or(|e| creabim_core::wrap::piece_of(e).is_none())
}

/// Is the section mark `id` one a plan of this kind shows: a structural
/// plan the structural sections' marks, every other plan the others.
pub fn shows_mark(model: &Model, id: ElementId, structural_plan: bool) -> bool {
    match model.get(id).map(|e| &e.kind) {
        Some(ElementKind::View(v)) if matches!(v.kind, creabim_core::ViewKind::Section { .. }) => {
            v.structural == structural_plan
        }
        _ => true,
    }
}

fn col(title: &'static str, width: f64, numeric: bool) -> crate::schedule::Column {
    crate::schedule::Column {
        title,
        field: None,
        width,
        numeric,
        align: None,
    }
}

/// The list of lintels by position: its mark, what lies over each
/// opening, how many openings, the pieces' length, the underside's
/// height and the storey.
pub fn lintel_schedule(model: &Model) -> crate::schedule::Schedule {
    use crate::schedule::{Row, Schedule};
    use creabim_core::words::tr;
    let columns = vec![
        col("Mark", 12.0, false),
        col("Lintel", 46.0, false),
        // how many openings the position is over
        col("Count", 12.0, true),
        col("Length m", 14.0, true),
        col("Underside", 18.0, false),
        col("Storey", 18.0, false),
    ];
    let positions: Vec<LintelPosition> = structural::lintel_positions(model);
    let rows: Vec<Row> = positions
        .iter()
        .map(|p| Row {
            id: p.groups[0][0],
            cells: vec![
                p.mark.clone(),
                p.label.clone(),
                p.groups.len().to_string(),
                comma(p.length, 3),
                format!("SH={}", level_text(p.underside)),
                p.storey.clone(),
            ],
            group: false,
        })
        .collect();
    let openings: usize = positions.iter().map(|p| p.groups.len()).sum();
    Schedule {
        category: creabim_core::ScheduleCategory::LintelPositions,
        columns,
        listed: rows.len(),
        rows,
        totals: vec![
            tr("Total"),
            String::new(),
            openings.to_string(),
            String::new(),
            String::new(),
            String::new(),
        ],
        above_totals: vec![],
    }
}

/// The list of ring beams by position: its mark, its section, running
/// metres and the concrete in it.
pub fn ring_schedule(model: &Model) -> crate::schedule::Schedule {
    use crate::schedule::{Row, Schedule};
    use creabim_core::words::tr;
    let columns = vec![
        col("Mark", 12.0, false),
        col("Ring beam", 50.0, false),
        col("Length m", 16.0, true),
        col("Concrete m³", 16.0, true),
    ];
    let positions: Vec<RingPosition> = structural::ring_positions(model);
    let mut concrete = 0.0;
    let rows: Vec<Row> = positions
        .iter()
        .map(|p| {
            let v = p.width * p.depth * p.length;
            concrete += v;
            Row {
                id: p.beams[0],
                cells: vec![
                    p.mark.clone(),
                    format!(
                        "{} {:.0}×{:.0} mm",
                        creabim_core::masonry::RING_BEAM,
                        p.width * 1000.0,
                        p.depth * 1000.0
                    ),
                    comma(p.length, 2),
                    comma(v, 2),
                ],
                group: false,
            }
        })
        .collect();
    let length: f64 = positions.iter().map(|p| p.length).sum();
    Schedule {
        category: creabim_core::ScheduleCategory::RingBeamList,
        columns,
        listed: rows.len(),
        rows,
        totals: vec![
            tr("Total"),
            String::new(),
            comma(length, 2),
            comma(concrete, 2),
        ],
        above_totals: vec![],
    }
}

/// A house with system lintels over each opening of its
/// load-bearing walls and ring beams along them, for tests and shots.
pub fn sample(model: &mut Model) {
    use creabim_core::masonry::{self as m, Family};
    let rules = m::rules(model);
    let mut tx = creabim_core::Transaction::new("lintels");
    for e in model.iter() {
        let ElementKind::Opening(o) = &e.kind else {
            continue;
        };
        let Some(ElementKind::Wall(w)) = model.get(o.host).map(|h| &h.kind) else {
            continue;
        };
        if !m::is_bearing(model, w, &rules) || !m::lintels_over(model, e.id).is_empty() {
            continue;
        }
        let set = m::usual_set(model, w, Family::PorfixNp);
        if let Ok(ls) = m::lintels_for(model, e.id, set) {
            for l in ls {
                tx = tx.insert(l);
            }
        }
    }
    for r in m::ring_beams(model, None) {
        tx = tx.insert(r);
    }
    // an RC column out on the terrace
    if let Some(level) = model.iter().find_map(|e| match &e.kind {
        ElementKind::Level(l) if l.elevation.abs() < 1e-9 => Some(e.id),
        _ => None,
    }) {
        let c = creabim_core::Column::new(
            level,
            DVec2::new(12.0, -1.5),
            creabim_core::Profile::Rectangle {
                width: 0.25,
                depth: 0.25,
            },
        );
        tx = tx.insert(creabim_core::Element::new("Stĺp", ElementKind::Column(c)));
    }
    tx.apply(model).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::{Element, Transaction, View, ViewKind};

    #[test]
    fn a_structural_section_draws_the_load_bearing_layers_only() {
        let m = creabim_core::starter::starter_house();
        let scene = crate::scene::shared_scene(&m);
        let layered: Vec<_> = scene.iter().filter(|i| i.layer.is_some()).collect();
        // the sample's walls and slabs have insulation, plaster, screed…
        assert!(layered.iter().any(|i| !bearing_part(&m, i)));
        // …and a core that stays
        assert!(layered.iter().any(|i| bearing_part(&m, i)));
        assert!(scene.iter().filter(|i| bearing_part(&m, i)).all(|i| i
            .layer
            .is_none_or(|l| l.function == creabim_core::LayerFunction::Structure)));
    }

    #[test]
    fn the_structural_plan_draws_what_carries_with_its_positions_and_lists() {
        let mut m = creabim_core::sample::sample_house();
        sample(&mut m);
        let lintels = structural::lintel_positions(&m);
        let rings = structural::ring_positions(&m);
        assert!(!lintels.is_empty() && !rings.is_empty());
        assert_eq!(lintels[0].mark, "P1");
        // a plan of the ground floor, the structural engineer's
        let (pid, _) = m
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::View(v) => match v.kind {
                    ViewKind::Plan { level, .. } if !v.graphics.range.from_above => {
                        Some((e.id, level))
                    }
                    _ => None,
                },
                _ => None,
            })
            .unwrap();
        let mut v = match &m.get(pid).unwrap().kind {
            ElementKind::View(v) => v.clone(),
            _ => unreachable!(),
        };
        creabim_core::viewmodes::Purpose::Structural.apply(&mut v);
        let sta = Element::new("Statika", ElementKind::View(v));
        let sid = sta.id;
        // a structural section and an architect's one
        let mut sec = View::section(
            glam::DVec2::new(-1.0, 2.0),
            glam::DVec2::new(20.0, 2.0),
            30.0,
            true,
        );
        creabim_core::viewmodes::Purpose::Structural.apply(&mut sec);
        let s1 = Element::new("Rez S", ElementKind::View(sec));
        let s1id = s1.id;
        Transaction::new("v")
            .insert(sta)
            .insert(s1)
            .apply(&mut m)
            .unwrap();
        let d = crate::section::view_drawing(&m, sid).unwrap();
        let rules = creabim_core::masonry::rules(&m);
        let of = |it: &PlanItem| m.get(it.id());
        // no partition, door or window, furniture drawn; a door or window
        // only as the pick area over its gap, not drawn
        assert!(d
            .items
            .iter()
            .all(|it| of(it).is_none_or(|e| match &e.kind {
                ElementKind::Wall(w) => creabim_core::masonry::is_bearing(&m, w, &rules),
                ElementKind::Opening(_) => matches!(it, PlanItem::Room { rgb: None, .. }),
                ElementKind::Object(_) | ElementKind::Room(_) => false,
                _ => true,
            })));
        // each door and window of a load-bearing wall can be picked in
        // its gap (for its lintels)
        let picks = d
            .items
            .iter()
            .filter(|it| {
                matches!(it, PlanItem::Room { .. })
                    && matches!(of(it).map(|e| &e.kind), Some(ElementKind::Opening(_)))
            })
            .count();
        assert!(picks > 0);
        let texts: Vec<&str> = d
            .items
            .iter()
            .filter_map(|it| match it {
                PlanItem::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"P1"), "{texts:?}");
        assert!(texts.iter().any(|t| t.starts_with("SH=+")), "{texts:?}");
        assert!(texts.contains(&"S1"), "{texts:?}");
        assert!(texts.iter().any(|t| t.starts_with("HH=+")), "{texts:?}");
        // walls as outlines, a chain with the positions along them
        assert!(!d.items.iter().any(|it| matches!(it,
            PlanItem::Fill { id, .. } if matches!(m.get(*id).map(|e| &e.kind), Some(ElementKind::Wall(_))))));
        let dims: Vec<&str> = d
            .items
            .iter()
            .filter_map(|it| match it {
                PlanItem::Dimension { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(dims.iter().any(|t| t.starts_with("P1 ")), "{dims:?}");
        assert!(dims.iter().any(|t| t.starts_with("V1 ")), "{dims:?}");
        // its own section's mark there, not in the architect's plan
        assert!(d.items.iter().any(|it| it.id() == s1id));
        let a = crate::section::view_drawing(&m, pid).unwrap();
        assert!(!a.items.iter().any(|it| it.id() == s1id));
        // the lists
        let l = lintel_schedule(&m);
        assert_eq!(l.rows.len(), lintels.len());
        assert_eq!(l.rows[0].cells[0], "P1");
        let r = ring_schedule(&m);
        assert_eq!(r.rows[0].cells[0], "V1");
    }
}
