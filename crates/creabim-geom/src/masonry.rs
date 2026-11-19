//! What a masonry plan writes over the building: by each opening its
//! system lintels (`3× NP 1800/100`), and over each stretch of masonry
//! wall that runs too far between braces, or stands too high, a red band
//! with what is wrong (see [`creabim_core::masonry`]).

use crate::paper::Anchor;
use crate::plan::PlanItem;
use creabim_core::masonry::{self, WallFault};
use creabim_core::{ElementId, ElementKind, Model};
use glam::DVec2;

/// The warning colour.
pub const WARN: [f32; 3] = [0.85, 0.1, 0.1];

/// Labels and warnings of the masonry plan of `level`, in a drawing of
/// `mm` metres per paper millimetre.
pub fn plan_marks(model: &Model, level: ElementId, mm: f64) -> Vec<PlanItem> {
    let mut out = vec![];
    // the lintels, by the opening they span
    let mut groups: Vec<(Option<ElementId>, Vec<ElementId>)> = vec![];
    for e in model.iter() {
        let ElementKind::Beam(b) = &e.kind else {
            continue;
        };
        let Some(l) = &b.lintel else { continue };
        if b.level != level {
            continue;
        }
        let key = l.opening.or(Some(e.id));
        match groups.iter_mut().find(|g| g.0 == key) {
            Some(g) => g.1.push(e.id),
            None => groups.push((key, vec![e.id])),
        }
    }
    // each piece's outline, dashed: it lies over the cut
    for (_, ids) in &groups {
        for id in ids {
            let Some(ElementKind::Beam(b)) = model.get(*id).map(|e| &e.kind) else {
                continue;
            };
            let pts = b.outline();
            for k in 0..pts.len() {
                out.push(PlanItem::Line {
                    id: *id,
                    pen: crate::plan::Pen::Hidden,
                    a: pts[k],
                    b: pts[(k + 1) % pts.len()],
                });
            }
        }
    }
    for (_, ids) in &groups {
        let beams: Vec<&creabim_core::Beam> = ids
            .iter()
            .filter_map(|id| match model.get(*id).map(|e| &e.kind) {
                Some(ElementKind::Beam(b)) => Some(b),
                _ => None,
            })
            .collect();
        let Some(first) = beams.first() else { continue };
        let mid = beams
            .iter()
            .map(|b| (b.start + b.end) / 2.0)
            .fold(DVec2::ZERO, |a, p| a + p)
            / beams.len() as f64;
        let d = (first.end - first.start).normalize_or_zero();
        let text = masonry::label(model, ids);
        // on the wall's inside, clear of it: a text along a wall across
        // the page stands off by half its length
        let (side, half_wall) = first
            .lintel
            .as_ref()
            .and_then(|l| l.opening)
            .and_then(|o| match model.get(o).map(|e| &e.kind) {
                Some(ElementKind::Opening(op)) => match model.get(op.host).map(|e| &e.kind) {
                    Some(ElementKind::Wall(w)) => {
                        let left = w.direction().perp();
                        // the exterior is on the right unless flipped
                        let inside = if w.flip { -left } else { left };
                        let centre =
                            (w.start + w.direction() * (op.offset + op.width / 2.0)).dot(inside);
                        Some((inside, centre + w.thickness / 2.0 - mid.dot(inside)))
                    }
                    _ => None,
                },
                _ => None,
            })
            .unwrap_or((d.perp(), first.width));
        let size = 2.0;
        let text_half = text.chars().count() as f64 * size * 0.3 * mm;
        let off = half_wall + 2.5 * mm + side.x.abs() * text_half + side.y.abs() * size * 0.6 * mm;
        let at = mid + side * off;
        let n = side;
        let issue = beams.iter().find_map(|b| masonry::lintel_issue(model, b));
        out.push(PlanItem::Text {
            id: ids[0],
            at,
            text,
            size,
            align: Anchor::Middle,
            mm,
            rgb: issue.is_some().then_some(WARN),
            bold: false,
        });
        if let Some(i) = issue {
            out.push(PlanItem::Text {
                id: ids[0],
                at: at + n * 3.0 * mm,
                text: format!("! {i}"),
                size: 1.8,
                align: Anchor::Middle,
                mm,
                rgb: Some(WARN),
                bold: false,
            });
        }
    }
    // the walls that run too far, or stand too high
    for issue in masonry::check_walls(model, Some(level)) {
        let thickness = issue
            .walls
            .iter()
            .filter_map(|id| match model.get(*id).map(|e| &e.kind) {
                Some(ElementKind::Wall(w)) => Some(w.thickness),
                _ => None,
            })
            .fold(0.0, f64::max);
        let d = (issue.b - issue.a).normalize_or_zero();
        let n = d.perp() * (thickness / 2.0 + 0.6 * mm);
        out.push(PlanItem::Tint {
            id: issue.walls[0],
            points: vec![issue.a - n, issue.b - n, issue.b + n, issue.a + n],
            rgb: WARN,
        });
        let text = match issue.fault {
            WallFault::TooLong { length, limit } => {
                format!("! L = {} m > {} m", comma(length), comma(limit))
            }
            WallFault::TooHigh { height, limit } => {
                format!("! h = {} m > {} m", comma(height), comma(limit))
            }
        };
        let side = d.perp();
        let half = text.chars().count() as f64 * 2.0 * 0.3 * mm;
        let off = thickness / 2.0 + 2.5 * mm + side.x.abs() * half + side.y.abs() * 1.2 * mm;
        out.push(PlanItem::Text {
            id: issue.walls[0],
            at: (issue.a + issue.b) / 2.0 + side * off,
            text,
            size: 2.0,
            align: Anchor::Middle,
            mm,
            rgb: Some(WARN),
            bold: true,
        });
    }
    out
}

fn comma(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}
