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
    // each piece's outline, a dash and two dots: it lies over the cut
    for (_, ids) in &groups {
        for id in ids {
            let Some(ElementKind::Beam(b)) = model.get(*id).map(|e| &e.kind) else {
                continue;
            };
            let pts = b.outline();
            for k in 0..pts.len() {
                out.push(PlanItem::Line {
                    id: *id,
                    pen: crate::plan::Pen::Overhead,
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
/// The lintel list: a row for each product on each storey with its
/// pieces, length, section and mass, and the openings they span (their
/// marks, else their sizes); the total mass under it.
pub fn lintel_schedule(model: &Model) -> crate::schedule::Schedule {
    use crate::schedule::{Column, Row, Schedule};
    use creabim_core::words::tr;
    let col = |title: &'static str, width: f64, numeric: bool| Column {
        title,
        field: None,
        width,
        numeric,
    };
    let columns = vec![
        col("Storey", 18.0, false),
        col("Lintel", 34.0, false),
        col("Maker", 22.0, false),
        col("Length mm", 16.0, true),
        col("Section mm", 18.0, false),
        col("Pieces", 12.0, true),
        col("Mass kg", 14.0, true),
        col("Over openings", 50.0, false),
    ];
    // (elevation, storey, product, first piece, pieces, openings)
    type Group = (
        f64,
        String,
        &'static masonry::LintelProduct,
        ElementId,
        usize,
        Vec<String>,
    );
    let mut groups: Vec<Group> = vec![];
    let marks = creabim_core::marks::marks(model);
    // a KP XL carries what its sheet gives under the project's ring beam
    let ring = masonry::rules(model).ring_depth;
    for e in model.iter() {
        let ElementKind::Beam(b) = &e.kind else {
            continue;
        };
        let Some(l) = &b.lintel else { continue };
        let Some(p) = masonry::product(&l.product) else {
            continue;
        };
        let (z, storey) = model
            .get(b.level)
            .map_or((0.0, String::new()), |lv| match &lv.kind {
                ElementKind::Level(x) => (x.elevation, lv.name.clone()),
                _ => (0.0, lv.name.clone()),
            });
        let marks = &marks;
        let opening = l.opening.and_then(|o| model.get(o)).map(|o| {
            marks.get(&o.id).cloned().unwrap_or_else(|| match &o.kind {
                ElementKind::Opening(op) => {
                    format!("{:.0}×{:.0}", op.width * 1000.0, op.height * 1000.0)
                }
                _ => o.name.clone(),
            })
        });
        match groups.iter_mut().find(|g| g.1 == storey && g.2.id == p.id) {
            Some(g) => {
                g.4 += 1;
                if let Some(o) = opening {
                    if !g.5.contains(&o) {
                        g.5.push(o);
                    }
                }
            }
            None => groups.push((z, storey, p, e.id, 1, opening.into_iter().collect())),
        }
    }
    groups.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.2.name.cmp(b.2.name)));
    let mut total = 0.0;
    let rows: Vec<Row> = groups
        .into_iter()
        .map(|(_, storey, p, id, n, mut ops)| {
            ops.sort();
            let mass = p.mass * n as f64;
            total += mass;
            Row {
                id,
                cells: vec![
                    storey,
                    match masonry::kp_xl_capacity(p.id, ring) {
                        Some([_, _, g]) => {
                            format!("{} (gRd {g:.1} kN/m)", p.name).replace('.', ",")
                        }
                        None => p.name.to_string(),
                    },
                    p.family.maker().to_string(),
                    format!("{:.0}", p.length * 1000.0),
                    format!("{:.0}×{:.0}", p.width * 1000.0, p.height * 1000.0),
                    n.to_string(),
                    format!("{mass:.0}"),
                    ops.join(", "),
                ],
                group: false,
            }
        })
        .collect();
    let listed = rows.len();
    let pieces: usize = rows
        .iter()
        .map(|r| r.cells[5].parse::<usize>().unwrap_or(0))
        .sum();
    Schedule {
        category: creabim_core::ScheduleCategory::LintelList,
        columns,
        rows,
        totals: vec![
            tr("Total"),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            pieces.to_string(),
            format!("{total:.0}"),
            String::new(),
        ],
        listed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::Transaction;

    #[test]
    fn the_lintel_list_counts_pieces_by_product_and_storey() {
        let mut m = creabim_core::starter::starter_house();
        let windows: Vec<ElementId> = m
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Opening(o) if o.width < 1.5 => Some(e.id),
                _ => None,
            })
            .take(2)
            .collect();
        assert_eq!(windows.len(), 2);
        let mut tx = Transaction::new("lintels");
        let mut n = 0;
        for w in &windows {
            let set = masonry::LintelSet {
                family: masonry::Family::PorfixNp,
                width: 0.125,
                count: 2,
            };
            for e in masonry::lintels_for(&m, *w, set).unwrap() {
                n += 1;
                tx = tx.insert(e);
            }
        }
        tx.apply(&mut m).unwrap();
        let s = lintel_schedule(&m);
        assert!(!s.rows.is_empty());
        let pieces: usize = s
            .rows
            .iter()
            .map(|r| r.cells[5].parse::<usize>().unwrap())
            .sum();
        assert_eq!(pieces, n);
        assert_eq!(s.totals[5], n.to_string());
        assert!(s.rows.iter().all(|r| r.cells[1].starts_with("NP")));
    }
}
