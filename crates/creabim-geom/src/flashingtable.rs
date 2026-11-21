//! The table of sheet-metal items (tabuľka klampiarskych prvkov) as
//! George's roof drawings carry it: a row for each item with its mark in
//! a circle, its drawn sign, its description, its developed width, its
//! length and its accessories counted; under the table the roof planes'
//! area, the covering, and the ridges' and hips' lengths. Drawn on paper
//! (1 unit = 1 mm), y up.

use crate::joinerylist::wrap;
use crate::paper::Anchor;
use crate::plan::{Pen, PlanItem, PlanView};
use creabim_core::flashing::{num, roof_figures, rows, Row, RowKind};
use creabim_core::{ElementId, ElementKind, Model};
use glam::DVec2;
use std::f64::consts::PI;

const TITLE: f64 = 5.0;
const TEXT: f64 = 2.5;
const LINE: f64 = 3.6;
const HEAD_ROW: f64 = 10.0;
/// The columns' widths, mm: mark, sign, description, r. š., length, note.
const COLS: [f64; 6] = [15.0, 38.0, 46.0, 12.0, 16.0, 52.0];
/// The least a row is tall: room for its sign.
const MIN_ROW: f64 = 30.0;

fn text(items: &mut Vec<PlanItem>, id: ElementId, at: DVec2, t: &str, size: f64, align: Anchor) {
    items.push(PlanItem::Text {
        id,
        at,
        text: t.to_string(),
        size,
        align,
        mm: 1.0,
        rgb: None,
        bold: false,
    });
}

fn line(items: &mut Vec<PlanItem>, id: ElementId, pen: Pen, a: DVec2, b: DVec2) {
    items.push(PlanItem::Line { id, pen, a, b });
}

/// The words of a row's note column: each accessory with its count,
/// then the row's own note.
pub fn note_lines(r: &Row) -> Vec<String> {
    let mut out: Vec<String> = r
        .extras
        .iter()
        .map(|(what, n)| format!("{what} {n} ks"))
        .collect();
    if !r.note.is_empty() {
        out.push(r.note.clone());
    }
    out
}

pub fn drawing(view: ElementId, title: &str, model: &Model) -> PlanView {
    let mut items = vec![];
    let id = view;
    let width: f64 = COLS.iter().sum();
    let xs: Vec<f64> = COLS
        .iter()
        .scan(0.0, |x, w| {
            let at = *x;
            *x += w;
            Some(at)
        })
        .collect();
    let mid = |c: usize| xs[c] + COLS[c] / 2.0;
    text(
        &mut items,
        id,
        DVec2::new(0.0, 3.0),
        title,
        TITLE,
        Anchor::Start,
    );
    let top = 0.0;
    let mut y = top;
    // the heading
    let heads = [
        ("Ozn.\nna výk.", 0),
        ("Grafická značka", 1),
        ("Popis", 2),
        ("R.š.\n(mm)", 3),
        ("Dĺžka\n(m)", 4),
        ("Poznámka", 5),
    ];
    for (h, c) in heads {
        let lines: Vec<&str> = h.split('\n').collect();
        let mut ty = y - HEAD_ROW / 2.0 + (lines.len() as f64 - 1.0) * LINE / 2.0 - TEXT * 0.35;
        for l in lines {
            text(
                &mut items,
                id,
                DVec2::new(mid(c), ty),
                l,
                TEXT,
                Anchor::Middle,
            );
            ty -= LINE;
        }
    }
    y -= HEAD_ROW;
    let mut rule_at = vec![y];
    let rows = rows(model);
    for r in &rows {
        let desc = wrap(&r.description, COLS[2] - 3.0, TEXT);
        let notes: Vec<String> = note_lines(r)
            .iter()
            .flat_map(|n| wrap(n, COLS[5] - 4.0, TEXT))
            .collect();
        let tall = ((desc.len().max(notes.len()) as f64) * LINE + 6.0).max(MIN_ROW);
        let cy = y - tall / 2.0;
        // the mark in its circle
        items.push(PlanItem::Arc {
            id,
            pen: Pen::Projection,
            center: DVec2::new(mid(0), cy),
            radius: 5.0,
            start: 0.0,
            end: 2.0 * PI,
        });
        text(
            &mut items,
            id,
            DVec2::new(mid(0), cy - TEXT * 0.35),
            &r.mark,
            TEXT,
            Anchor::Middle,
        );
        sign(
            &mut items,
            id,
            r.kind,
            DVec2::new(mid(1), cy),
            COLS[1] - 6.0,
            tall - 6.0,
        );
        let mut ty = cy + (desc.len() as f64 - 1.0) * LINE / 2.0 - TEXT * 0.35;
        for l in &desc {
            text(
                &mut items,
                id,
                DVec2::new(mid(2), ty),
                l,
                TEXT,
                Anchor::Middle,
            );
            ty -= LINE;
        }
        text(
            &mut items,
            id,
            DVec2::new(mid(3), cy - TEXT * 0.35),
            &num(r.developed_mm),
            TEXT,
            Anchor::Middle,
        );
        text(
            &mut items,
            id,
            DVec2::new(mid(4), cy - TEXT * 0.35),
            &format!("{:.3}", r.length).replace('.', ","),
            TEXT,
            Anchor::Middle,
        );
        let mut ty = cy + (notes.len() as f64 - 1.0) * LINE / 2.0 - TEXT * 0.35;
        for l in &notes {
            text(
                &mut items,
                id,
                DVec2::new(xs[5] + 3.0, ty),
                l,
                TEXT,
                Anchor::Start,
            );
            ty -= LINE;
        }
        y -= tall;
        rule_at.push(y);
    }
    if rows.is_empty() {
        y -= MIN_ROW / 2.0;
        text(
            &mut items,
            id,
            DVec2::new(width / 2.0, y),
            "Na streche zatiaľ nie sú žľaby ani zvody.",
            TEXT,
            Anchor::Middle,
        );
        y -= MIN_ROW / 2.0;
        rule_at.push(y);
    }
    // the frame: heavy outside, thin between rows and columns
    let bottom = y;
    for (a, b) in [
        (DVec2::new(0.0, top), DVec2::new(width, top)),
        (DVec2::new(0.0, bottom), DVec2::new(width, bottom)),
        (DVec2::new(0.0, top), DVec2::new(0.0, bottom)),
        (DVec2::new(width, top), DVec2::new(width, bottom)),
    ] {
        line(&mut items, id, Pen::Cut, a, b);
    }
    for (i, x) in xs.iter().enumerate().skip(1) {
        let _ = i;
        line(
            &mut items,
            id,
            Pen::Projection,
            DVec2::new(*x, top),
            DVec2::new(*x, bottom),
        );
    }
    for ry in rule_at.iter().filter(|r| (**r - bottom).abs() > 1e-9) {
        let pen = if (*ry - (top - HEAD_ROW)).abs() < 1e-9 {
            Pen::Cut
        } else {
            Pen::Projection
        };
        line(
            &mut items,
            id,
            pen,
            DVec2::new(0.0, *ry),
            DVec2::new(width, *ry),
        );
    }
    // the roof's figures under it
    y -= 6.0;
    let fig = roof_figures(model);
    let dot = |items: &mut Vec<PlanItem>, y: f64| {
        items.push(PlanItem::Arc {
            id,
            pen: Pen::Cut,
            center: DVec2::new(1.2, y + TEXT * 0.35),
            radius: 0.8,
            start: 0.0,
            end: 2.0 * PI,
        });
    };
    let m2 = |v: f64| format!("{v:.2}").replace('.', ",");
    if fig.area > 0.0 {
        dot(&mut items, y);
        text(
            &mut items,
            id,
            DVec2::new(4.0, y),
            &format!("Plocha strešných rovín S = {} m²", m2(fig.area)),
            TEXT + 0.5,
            Anchor::Start,
        );
        y -= LINE + 1.5;
    }
    for c in coverings(model) {
        dot(&mut items, y);
        text(
            &mut items,
            id,
            DVec2::new(4.0, y),
            &format!("Strešná krytina {c}"),
            TEXT + 0.5,
            Anchor::Start,
        );
        y -= LINE + 1.5;
    }
    for (what, l) in [
        ("hrebeňa", fig.ridge),
        ("nárožia", fig.hip),
        ("úžľabia", fig.valley),
    ] {
        if l <= 0.0 {
            continue;
        }
        text(
            &mut items,
            id,
            DVec2::new(4.0, y),
            &format!("Celková dĺžka {what}"),
            TEXT + 0.5,
            Anchor::Start,
        );
        text(
            &mut items,
            id,
            DVec2::new(60.0, y),
            &format!("l = {} bm", m2(l)),
            TEXT + 0.5,
            Anchor::Start,
        );
        y -= LINE + 1.5;
    }
    PlanView { items }
}

/// What the roofs are covered with: each roof's top layer's material
/// (or the roof's own material), each name once.
pub fn coverings(model: &Model) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for e in model.iter() {
        let ElementKind::Roof(r) = &e.kind else {
            continue;
        };
        let top = r
            .assembly
            .and_then(|a| match &model.get(a)?.kind {
                ElementKind::Assembly(a) => a.layers.iter().find_map(|l| l.material),
                _ => None,
            })
            .or(r.material);
        let Some(name) = top.and_then(|m| model.get(m)).map(|m| m.name.clone()) else {
            continue;
        };
        if !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

/// The drawn sign of a row's item, fitted in a `w` × `h` box round `c`.
fn sign(items: &mut Vec<PlanItem>, id: ElementId, kind: RowKind, c: DVec2, w: f64, h: f64) {
    let pen = Pen::Projection;
    let mut l = |a: DVec2, b: DVec2| line(items, id, pen, a, b);
    match kind {
        RowKind::GutterHalfRound | RowKind::GutterSquare => {
            // the section across the gutter, its bead at the front and
            // the hook's back
            let r = (w.min(h) * 0.3).min(8.0);
            let o = c + DVec2::new(0.0, r * 0.4);
            if kind == RowKind::GutterHalfRound {
                let n = 16;
                let pts: Vec<DVec2> = (0..=n)
                    .map(|i| o + DVec2::from_angle(PI + PI * i as f64 / n as f64) * r)
                    .collect();
                for s in pts.windows(2) {
                    l(s[0], s[1]);
                }
            } else {
                let pts = [
                    o + DVec2::new(-r, 0.0),
                    o + DVec2::new(-r, -1.6 * r),
                    o + DVec2::new(r, -1.6 * r),
                    o + DVec2::new(r, 0.0),
                ];
                for s in pts.windows(2) {
                    l(s[0], s[1]);
                }
            }
            // the back edge turned up and the hook over it
            l(o + DVec2::new(r, 0.0), o + DVec2::new(r, r * 0.5));
            l(
                o + DVec2::new(r, r * 0.5),
                o + DVec2::new(r + r * 0.4, r * 0.5),
            );
            items.push(PlanItem::Arc {
                id,
                pen,
                center: o + DVec2::new(-r - 0.8, 0.0),
                radius: 0.8,
                start: 0.0,
                end: 2.0 * PI,
            });
        }
        RowKind::DownpipeRound | RowKind::DownpipeSquare => {
            // a downpipe with its offset (two elbows) under the outlet
            let t = (w * 0.12).clamp(2.5, 4.5);
            let top = c.y + h / 2.0;
            let bot = c.y - h / 2.0;
            let x0 = c.x - w * 0.2;
            let x1 = c.x + w * 0.2;
            let y1 = top - h * 0.2;
            let y2 = y1 - (x1 - x0);
            for side in [-t / 2.0, t / 2.0] {
                l(DVec2::new(x0 + side, top), DVec2::new(x0 + side, y1));
                l(DVec2::new(x0 + side, y1), DVec2::new(x1 + side, y2));
                l(DVec2::new(x1 + side, y2), DVec2::new(x1 + side, bot));
            }
            if kind == RowKind::DownpipeRound {
                // the round top seen as an ellipse, flattened to a ring
                let n = 12;
                let pts: Vec<DVec2> = (0..=n)
                    .map(|i| {
                        let a = 2.0 * PI * i as f64 / n as f64;
                        DVec2::new(x0 + a.cos() * t / 2.0, top + a.sin() * t / 5.0)
                    })
                    .collect();
                for s in pts.windows(2) {
                    l(s[0], s[1]);
                }
            } else {
                l(DVec2::new(x0 - t / 2.0, top), DVec2::new(x0 + t / 2.0, top));
            }
            // a clamp on the pipe
            l(
                DVec2::new(x1 - t / 2.0 - 0.8, bot + h * 0.25),
                DVec2::new(x1 + t / 2.0 + 0.8, bot + h * 0.25),
            );
        }
    }
}

/// The table as a spreadsheet, for the schedule pane: a row an item.
pub fn schedule(model: &Model) -> crate::schedule::Schedule {
    use crate::schedule::{Column, Row as SRow, Schedule};
    let col = |title: &'static str, width: f64, numeric: bool| Column {
        title,
        field: None,
        width,
        numeric,
        align: None,
    };
    let columns = vec![
        col("Mark", 14.0, false),
        col("Description", 70.0, false),
        col("Developed width mm", 18.0, true),
        col("Length m", 16.0, true),
        col("Note", 60.0, false),
    ];
    let rows: Vec<SRow> = rows(model)
        .into_iter()
        .map(|r| SRow {
            id: r.members.first().copied().unwrap_or(ElementId::NIL),
            cells: vec![
                r.mark.clone(),
                r.description.clone(),
                num(r.developed_mm),
                format!("{:.3}", r.length).replace('.', ","),
                note_lines(&r).join(", "),
            ],
            group: false,
        })
        .collect();
    let listed = rows.len();
    Schedule {
        category: creabim_core::ScheduleCategory::FlashingTable,
        columns,
        rows,
        totals: vec![],
        listed,
        above_totals: vec![],
    }
}
