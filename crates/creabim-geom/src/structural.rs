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
            let sh = format!("SH={}", level_text(p.underside));
            // the mark over its underside, upright on the sheet whichever
            // way the wall runs, the pair clear of the wall's face on its
            // inside (George 2026-11-24: on a wall up the sheet the two
            // were written over each other), or where it was dragged to
            let wide = |t: &str, size: f64| t.chars().count() as f64 * size * 0.72;
            let half = DVec2::new(
                wide(&p.mark, 2.5).max(wide(&sh, 1.8)) / 2.0,
                (1.25 + 3.5 + 0.9) / 2.0,
            ) * mm;
            let reach = half.x * side.x.abs() + half.y * side.y.abs();
            let dragged = model.get(ids[0]).and_then(|e| match &e.kind {
                ElementKind::Beam(b) => b.lintel.as_ref().and_then(|l| l.mark_at),
                _ => None,
            });
            let block = mid + side * (half_wall + 2.5 * mm + reach);
            let at = block + DVec2::new(0.0, half.y - 1.25 * mm) + dragged.unwrap_or(DVec2::ZERO);
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
                at: at - DVec2::new(0.0, 3.5 * mm),
                text: sh,
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

/// The position beam `id` is listed by: its lintel position (`P1`) or
/// its ring beam position (`V1`).
pub fn beam_mark(model: &Model, id: ElementId) -> Option<String> {
    if let Some(p) = structural::lintel_positions(model)
        .into_iter()
        .find(|p| p.groups.iter().any(|g| g.contains(&id)))
    {
        return Some(p.mark);
    }
    structural::ring_positions(model)
        .into_iter()
        .find(|p| p.beams.contains(&id))
        .map(|p| p.mark)
}

/// How far above a structural section's highest construction its marks
/// stand in a row, m (as STA_002 has them over its sections).
const MARK_ROW: f64 = 0.8;
/// How far apart marks in that row keep at the least, m.
const MARK_GAP: f64 = 0.9;
/// How far from the construction's outer face a spot level's symbol
/// stands, m.
const SPOT_OUT: f64 = 1.2;

/// The structural sections not yet marked, each given its marks once:
/// a P or V mark in a circle on a leader over each lintel and ring beam
/// it draws, spot levels of the ring beams' top and underside and the
/// base slab's top on the left outer face, of each lintel's underside and
/// ±0,000 (George 2026-11-24, as the sections of STA_002). They are
/// ordinary tags and spot levels: each can be moved, its leader too, or
/// deleted, and they are not made again.
pub fn section_marks(model: &Model) -> Option<creabim_core::Transaction> {
    let mut tx = creabim_core::Transaction::new("Structural section marks");
    for e in model.iter() {
        let ElementKind::View(v) = &e.kind else {
            continue;
        };
        if !v.structural || v.structural_marked {
            continue;
        }
        let creabim_core::ViewKind::Section {
            start,
            end,
            depth,
            cut: true,
        } = v.kind
        else {
            continue;
        };
        let frame = crate::section::SectionFrame::new(start, end, depth);
        for m in marks_of_section(model, e.id, &frame) {
            tx = tx.insert(m);
        }
        let mut after = e.clone();
        if let ElementKind::View(x) = &mut after.kind {
            x.structural_marked = true;
        }
        tx = tx.replace(e.clone(), after);
    }
    (!tx.ops.is_empty()).then_some(tx)
}

/// What [`section_marks`] gives the section `view`.
fn marks_of_section(
    model: &Model,
    view: ElementId,
    frame: &crate::section::SectionFrame,
) -> Vec<creabim_core::Element> {
    use creabim_core::{Dimension, Element, HostPoint, Tag};
    use std::collections::HashMap;
    let Some(d) = crate::section::view_drawing(model, view) else {
        return vec![];
    };
    // what the section draws, each element's extent in it
    let mut seen: HashMap<ElementId, (DVec2, DVec2)> = HashMap::new();
    for it in &d.items {
        if matches!(it, PlanItem::Annotation { .. } | PlanItem::Text { .. }) {
            continue;
        }
        if let Some((lo, hi)) = it.bounds() {
            seen.entry(it.id())
                .and_modify(|b| *b = (b.0.min(lo), b.1.max(hi)))
                .or_insert((lo, hi));
        }
    }
    let kind = |id: &ElementId| model.get(*id).map(|e| &e.kind);
    // the construction's extent: its walls and columns
    let (lo, hi) = seen
        .iter()
        .filter(|(id, _)| {
            matches!(
                kind(id),
                Some(ElementKind::Wall(_) | ElementKind::Column(_))
            )
        })
        .map(|(_, b)| *b)
        .reduce(|a, b| (a.0.min(b.0), a.1.max(b.1)))
        .unwrap_or((DVec2::ZERO, DVec2::ZERO));
    if hi.x - lo.x < 1e-6 {
        return vec![];
    }
    let mut out = vec![];
    // a point of the section's plane in the model
    let world = |u: f64, z: f64| (frame.origin + frame.along * u).extend(z);
    // a point kept on `host`, so the spot level follows it
    let on = |host: ElementId, p: glam::DVec3| -> HostPoint {
        match crate::annotate::host_frame(model, host) {
            Some(f) => HostPoint {
                host: Some(host),
                body: Some(f.inverse().transform_point3(p)),
                ..HostPoint::free(frame.project(p).truncate())
            },
            None => HostPoint::free(frame.project(p).truncate()),
        }
    };
    let spot = |a: HostPoint, b: DVec2, short: bool| {
        let mut d = Dimension::spot(view, a, HostPoint::free(b), None);
        d.short = short;
        Element::new("Spot level", ElementKind::Dimension(d))
    };
    let beam = |id: &ElementId| match kind(id) {
        Some(ElementKind::Beam(b)) => Some(b),
        _ => None,
    };
    // the marks in a row over the construction
    let mut row: Vec<f64> = vec![];
    let mut tag = |host: ElementId, out: &mut Vec<Element>| {
        let Some(c) = crate::annotate::centre(model, host) else {
            return;
        };
        let c = frame.project(c).truncate();
        let mut x = c.x;
        while row.iter().any(|r| (r - x).abs() < MARK_GAP) {
            x += MARK_GAP;
        }
        row.push(x);
        let t = Tag::new(view, host, DVec2::new(x, hi.y + MARK_ROW) - c);
        out.push(Element::new("Tag", ElementKind::Tag(t)));
    };
    // each lintel over an opening once, with its underside; one of the
    // same position seen behind another (the openings of one wall, one
    // behind the other) not again
    let mut done: Vec<(String, DVec2)> = vec![];
    for p in structural::lintel_positions(model) {
        for g in &p.groups {
            let Some(id) = g.iter().find(|id| seen.contains_key(id)) else {
                continue;
            };
            let Some(b) = beam(id) else { continue };
            let mid = (b.start + b.end) / 2.0;
            let under = mid.extend(b.top(model) - b.depth);
            let at = frame.project(under).truncate();
            if done
                .iter()
                .any(|(m, q)| *m == p.mark && q.distance(at) < 0.6)
            {
                continue;
            }
            done.push((p.mark.clone(), at));
            tag(*id, &mut out);
            out.push(spot(on(*id, under), at + DVec2::new(0.35, -0.3), true));
        }
    }
    // the ring beams: each marked; the outermost cut ones on each side
    // with their top and underside
    let mut cut: Vec<(f64, ElementId, glam::DVec3)> = vec![];
    for p in structural::ring_positions(model) {
        for id in &p.beams {
            if !seen.contains_key(id) {
                continue;
            }
            tag(*id, &mut out);
            let Some(b) = beam(id) else { continue };
            let (s0, s1) = (
                frame.project(b.start.extend(0.0)).z,
                frame.project(b.end.extend(0.0)).z,
            );
            if s0.signum() == s1.signum() || (s1 - s0).abs() < 1e-9 {
                continue;
            }
            let q = b.start.lerp(b.end, s0 / (s0 - s1));
            let n = (b.end - b.start).normalize_or_zero().perp() * b.width / 2.0;
            for f in [q + n, q - n] {
                let p = f.extend(b.top(model));
                cut.push((frame.project(p).x, *id, p));
            }
        }
    }
    // on the left: the storeys' heads stand on the right
    let mut sides = vec![];
    if let Some(l) = cut.iter().min_by(|a, b| a.0.total_cmp(&b.0)) {
        sides.push((*l, -1.0));
    }
    for ((u, id, p), side) in &sides {
        let depth = beam(id).map_or(0.0, |b| b.depth);
        for z in [p.z, p.z - depth] {
            let at = p.truncate().extend(z);
            out.push(spot(on(*id, at), DVec2::new(u + side * SPOT_OUT, z), false));
        }
    }
    // the base slab's top on both outer faces
    let slabs: Vec<(ElementId, (DVec2, DVec2))> = seen
        .iter()
        .filter(|(id, _)| matches!(kind(id), Some(ElementKind::Slab(_))))
        .map(|(id, b)| (*id, *b))
        .collect();
    let low = slabs.iter().map(|s| s.1 .1.y).fold(f64::INFINITY, f64::min);
    let base: Vec<_> = slabs
        .iter()
        .filter(|s| (s.1 .1.y - low).abs() < 0.02)
        .collect();
    if let Some((id, (slo, shi))) = base.iter().min_by(|a, b| a.1 .0.x.total_cmp(&b.1 .0.x)) {
        let p = world(slo.x, shi.y);
        out.push(spot(on(*id, p), DVec2::new(slo.x - SPOT_OUT, shi.y), false));
    }
    // ±0,000 inside, by the storey at it
    if let Some((lid, l)) = model.iter().find_map(|e| match &e.kind {
        ElementKind::Level(l) if l.elevation.abs() < 1e-6 => Some((e.id, l)),
        _ => None,
    }) {
        let u = lo.x + (hi.x - lo.x) * 0.62;
        let a = HostPoint {
            host: Some(lid),
            ..HostPoint::free(DVec2::new(u, 0.0))
        };
        out.push(spot(a, DVec2::new(u, l.elevation + 0.35), true));
    }
    out
}

/// The dashed ±0,000 line across a structural section, between the
/// faces of what it draws (STA_002).
pub fn zero_line(model: &Model, view: ElementId, items: &[PlanItem]) -> Option<PlanItem> {
    let (lo, hi) = items
        .iter()
        .filter(|it| {
            matches!(
                model.get(it.id()).map(|e| &e.kind),
                Some(ElementKind::Wall(_) | ElementKind::Column(_))
            )
        })
        .filter_map(|it| it.bounds())
        .reduce(|a, b| (a.0.min(b.0), a.1.max(b.1)))?;
    (hi.x > lo.x).then_some(PlanItem::Line {
        id: view,
        pen: Pen::Hidden,
        a: DVec2::new(lo.x, 0.0),
        b: DVec2::new(hi.x, 0.0),
    })
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
    fn a_structural_section_gets_its_marks_and_spot_levels_once() {
        let mut m = creabim_core::sample::sample_house();
        sample(&mut m);
        let mut sec = View::section(
            glam::DVec2::new(-1.0, 2.0),
            glam::DVec2::new(20.0, 2.0),
            30.0,
            true,
        );
        creabim_core::viewmodes::Purpose::Structural.apply(&mut sec);
        let s = Element::new("Rez S", ElementKind::View(sec));
        let sid = s.id;
        Transaction::new("v").insert(s).apply(&mut m).unwrap();
        section_marks(&m).unwrap().apply(&mut m).unwrap();
        // P and V marks in circles over its lintels and ring beams
        let marks: Vec<(String, bool)> = m
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Tag(t) if t.view == sid => Some(crate::annotate::tag_text(&m, t.host)),
                _ => None,
            })
            .collect();
        assert!(
            marks.iter().any(|(t, r)| t.starts_with('V') && *r),
            "{marks:?}"
        );
        assert!(marks
            .iter()
            .all(|(t, r)| (t.starts_with('V') || t.starts_with('P')) && *r));
        // spot levels, ±0,000 among them, drawn
        let spots = m
            .iter()
            .filter(|e| {
                matches!(&e.kind, ElementKind::Dimension(d)
                    if d.view == sid && d.kind == creabim_core::DimensionKind::Spot)
            })
            .count();
        assert!(spots >= 3, "{spots}");
        let d = crate::section::view_drawing(&m, sid).unwrap();
        assert!(d.items.iter().any(
            |it| matches!(it, PlanItem::Annotation { item, .. } if item.text.contains("0,000"))
        ));
        // and not again: they are the user's now
        assert!(section_marks(&m).is_none());
    }

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
        // each lintel's mark over its SH, upright, on walls either way
        // (George 2026-11-24: on a wall up the sheet they overlapped)
        let lintel_texts: Vec<(&str, DVec2)> = d
            .items
            .iter()
            .filter_map(|it| match it {
                PlanItem::Text { id, text, at, .. }
                    if matches!(m.get(*id).map(|e| &e.kind),
                        Some(ElementKind::Beam(b)) if b.lintel.is_some()) =>
                {
                    Some((text.as_str(), *at))
                }
                _ => None,
            })
            .collect();
        let mm = m.get(sid).map_or(0.05, |e| match &e.kind {
            ElementKind::View(v) => v.scale / 1000.0,
            _ => 0.05,
        });
        for pair in lintel_texts.chunks(2) {
            let [(p, a), (sh, b)] = pair else {
                panic!("{pair:?}")
            };
            assert!(p.starts_with('P') && sh.starts_with("SH="), "{pair:?}");
            assert!((a.x - b.x).abs() < 1e-9 && (a.y - b.y - 3.5 * mm).abs() < 1e-9);
        }
        if let Ok(out) = std::env::var("CREABIM_TEST_SVG") {
            std::fs::write(
                out,
                d.to_svg(m.get(sid).map_or(50.0, |e| match &e.kind {
                    ElementKind::View(v) => v.scale,
                    _ => 50.0,
                })),
            )
            .unwrap();
        }
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
