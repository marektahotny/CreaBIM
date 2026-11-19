//! The list of the documentation: part A of the documentation of a
//! construction (decree 60/2025 Z. z., annex 17). Over a table of every
//! sheet by its part (number as annex 24 names the files, name, scale,
//! paper, who designed it) stand the construction's name and place, the
//! date of issue and the revision; under the sheets, the text of part B
//! and the annexes of part E the decree always asks for.

use crate::paper::Anchor;
use crate::plan::{PlanItem, PlanView};
use crate::schedule::{Column, Row, Schedule};
use creabim_core::words::tr;
use creabim_core::{ElementId, ElementKind, Model, ScheduleCategory, ViewKind};
use glam::DVec2;

/// The number the summary report (part B) is filed under.
pub const SUMMARY_NUMBER: &str = "B00_TXT_001";

/// The scale of a sheet's drawings: "1:50", several as "1:50, 1:100",
/// "-" with none to scale (lists and legends).
fn scale_of(model: &Model, sheet: ElementId) -> String {
    let mut scales: Vec<i64> = vec![];
    for e in model.iter() {
        let ElementKind::Viewport(vp) = &e.kind else {
            continue;
        };
        if vp.sheet != sheet {
            continue;
        }
        let Some(ElementKind::View(v)) = model.get(vp.view).map(|e| &e.kind) else {
            continue;
        };
        if matches!(v.kind, ViewKind::Schedule { .. }) {
            continue;
        }
        let s = v.scale.round() as i64;
        if !scales.contains(&s) {
            scales.push(s);
        }
    }
    if scales.is_empty() {
        return "-".into();
    }
    scales.sort();
    scales
        .iter()
        .map(|s| format!("1:{s}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The list as a table: a heading for each part, under it its sheets in
/// the order of their numbers.
pub fn schedule(model: &Model) -> Schedule {
    let col = |title: &'static str, width: f64| Column {
        title,
        field: None,
        width,
        numeric: false,
    };
    let columns = vec![
        col("Number", 46.0),
        col("Name", 70.0),
        col("Scale", 24.0),
        col("Paper size", 18.0),
        col("Designer", 42.0),
    ];
    let info = model
        .project_info()
        .map(|p| p.1.clone())
        .unwrap_or_default();
    let designer = if info.designer.is_empty() {
        info.architect.clone()
    } else {
        info.designer.clone()
    };
    // (the part's place in the documentation, its heading, number, row)
    let mut sheets: Vec<(usize, String, String, Row)> = vec![];
    for e in model.iter() {
        let ElementKind::Sheet(s) = &e.kind else {
            continue;
        };
        // title pages are no drawings
        if crate::coverpage::cover_of(s).is_some() {
            continue;
        }
        let (order, part) = crate::docset::part_order(&s.number)
            .map_or((usize::MAX, tr("Other sheets")), |(i, p)| (i, tr(p)));
        sheets.push((
            order,
            part,
            s.number.clone(),
            Row {
                id: e.id,
                group: false,
                cells: vec![
                    s.number.clone(),
                    e.name.clone(),
                    scale_of(model, e.id),
                    s.size_name().unwrap_or("-").to_string(),
                    designer.clone(),
                ],
            },
        ));
    }
    let drawings = sheets.len();
    // the text of part B, and what part E always holds
    let text = |order: usize, part: &str, number: &str, name: &str| {
        (
            order,
            tr(part),
            number.to_string(),
            Row {
                id: ElementId::NIL,
                group: false,
                cells: vec![
                    number.to_string(),
                    tr(name),
                    "-".into(),
                    "A4".into(),
                    designer.clone(),
                ],
            },
        )
    };
    let part = |key: &str| crate::docset::part_order(key).expect("a part of the set");
    let (b, e) = (part("B00_"), part("E00_"));
    sheets.push(text(b.0, b.1, SUMMARY_NUMBER, "Summary report"));
    sheets.push(text(e.0, e.1, "E00", "Geodetic survey of the site"));
    sheets.push(text(e.0, e.1, "E00", "Fire safety design"));
    sheets.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.2.cmp(&b.2)));
    let mut rows = vec![];
    let mut last = None;
    for (order, part, _, row) in sheets {
        if last != Some(order) {
            let mut cells = vec![String::new(); columns.len()];
            cells[0] = part;
            rows.push(Row {
                id: ElementId::NIL,
                group: true,
                cells,
            });
            last = Some(order);
        }
        rows.push(row);
    }
    let mut totals = vec![String::new(); columns.len()];
    totals[0] = tr("Number of drawings");
    totals[1] = drawings.to_string();
    Schedule {
        category: ScheduleCategory::Documents,
        columns,
        rows,
        totals,
        listed: drawings,
    }
}

/// The list as a drawing in paper millimetres: the table with its top
/// left corner at the origin, the construction's data over it.
pub fn drawing(view: ElementId, title: &str, model: &Model) -> PlanView {
    let mut table = schedule(model);
    crate::schedule::apply_widths(model, view, &mut table);
    let mut d = crate::schedule::schedule_drawing(view, title, &table);
    let info = model
        .project_info()
        .map(|p| p.1.clone())
        .unwrap_or_default();
    let or_dots = |s: &str| {
        if s.trim().is_empty() {
            "........................".to_string()
        } else {
            s.trim().to_string()
        }
    };
    let place = [info.address.trim(), info.cadastre.trim()]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(", ");
    let name = if info.name.trim().is_empty() {
        &info.building_name
    } else {
        &info.name
    };
    let lines = [
        (tr("Name of the construction"), or_dots(name)),
        (tr("Place of the construction"), or_dots(&place)),
        (tr("Plots"), or_dots(&info.parcels)),
        (tr("Client"), or_dots(&info.client)),
        (tr("Stage"), or_dots(&info.status)),
        (tr("Date"), or_dots(&info.issue_date)),
        (tr("Revision"), info.revision_or_first()),
    ];
    let n = lines.len() as f64;
    for (i, (label, value)) in lines.iter().enumerate() {
        let y = 6.0 + (n - 1.0 - i as f64) * 5.0;
        for (x, t) in [(0.0, format!("{label}:")), (32.0, value.clone())] {
            d.items.push(PlanItem::Text {
                id: view,
                at: DVec2::new(x, y),
                text: t,
                size: 2.8,
                align: Anchor::Start,
                mm: 1.0,
                rgb: None,
                bold: x > 0.0,
            });
        }
    }
    d
}
