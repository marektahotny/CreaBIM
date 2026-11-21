//! The single-line diagram of the distribution boards, as
//! PilonDesigner's `boardSchemaSvg`: the feed and main switch, the
//! busbar, and each final circuit leaving it with its breaker, its
//! residual current device, its cable, the power installed, its phase
//! and what it feeds. Symbols simplified after STN EN 60617. Drawn on
//! paper (1 unit = 1 mm), a board under the other.

use crate::paper::Anchor;
use crate::plan::{Pen, PlanItem, PlanView};
use creabim_core::electrical::{self as el, DeviceKind};
use creabim_core::words::tr;
use creabim_core::{ElementId, Model};
use glam::DVec2;

const HEAD: f64 = 3.2;
const TEXT: f64 = 2.2;
/// A circuit's row, mm.
const ROW: f64 = 8.0;
/// Where the busbar runs, and the columns of the texts, mm.
const BUS: f64 = 12.0;
const COLS: [f64; 7] = [46.0, 62.0, 78.0, 110.0, 128.0, 142.0, 164.0];

/// One circuit as the diagram writes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub circuit: ElementId,
    pub name: String,
    pub breaker: String,
    pub cable: String,
    pub power_w: f64,
    pub phase: String,
    pub rcd: String,
    pub three_phase: bool,
    /// What it feeds: "4× Socket, 1× Appliance", in English kinds.
    pub feeds: Vec<(usize, &'static str)>,
}

/// The boards of the model and their circuits, by name.
pub fn boards(model: &Model) -> Vec<(ElementId, String, Vec<Row>)> {
    let devs = el::devices(model);
    let mut boards: Vec<(ElementId, String, Vec<Row>)> = devs
        .iter()
        .filter(|d| d.2 == DeviceKind::Board)
        .map(|(e, _, _)| (e.id, e.name.clone(), vec![]))
        .collect();
    let mut loose = vec![];
    for (ce, c) in el::circuits(model) {
        let mut feeds: Vec<(usize, &'static str)> = vec![];
        let mut power = 0.0;
        for (e, _, kind) in devs.iter() {
            if *kind == DeviceKind::Board || el::circuit_of(model, e.id) != Some(ce.id) {
                continue;
            }
            match feeds.iter_mut().find(|f| f.1 == kind.label()) {
                Some(f) => f.0 += 1,
                None => feeds.push((1, kind.label())),
            }
        }
        for l in &c.loads {
            power += l.power_w;
        }
        let row = Row {
            circuit: ce.id,
            name: ce.name.clone(),
            breaker: c.breaker_a.map_or("–".into(), |a| format!("B{}", trim(a))),
            cable: c.cable().unwrap_or_else(|| "–".into()),
            power_w: power,
            phase: if c.three_phase {
                "3f".into()
            } else {
                c.phase.map_or("1f".into(), |p| format!("L{p}"))
            },
            rcd: c
                .rcd_ma
                .map_or(String::new(), |m| format!("{} mA", trim(m))),
            three_phase: c.three_phase,
            feeds,
        };
        match boards.iter_mut().find(|b| Some(b.0) == c.board) {
            Some(b) => b.2.push(row),
            None => loose.push(row),
        }
    }
    if !loose.is_empty() {
        boards.push((ElementId::default(), String::new(), loose));
    }
    let key = |s: &str| {
        let digits: String = s.chars().filter(char::is_ascii_digit).collect();
        (digits.parse::<u64>().unwrap_or(u64::MAX), s.to_string())
    };
    for b in &mut boards {
        b.2.sort_by_key(|r| key(&r.name));
    }
    boards
}

fn trim(v: f64) -> String {
    if v.fract().abs() < 1e-9 {
        format!("{v:.0}")
    } else {
        format!("{v}").replace('.', ",")
    }
}

/// The diagram as a table, for the export to a spreadsheet: a row for
/// each circuit under its board's.
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
        col("Board", 22.0, false),
        col("Circuit", 16.0, false),
        col("Breaker", 16.0, false),
        col("Cable", 30.0, false),
        col("Power W", 18.0, true),
        col("Phase", 12.0, false),
        col("RCD", 16.0, false),
        col("Feeds", 60.0, false),
    ];
    let mut rows = vec![];
    for (id, name, circuits) in boards(model) {
        for r in circuits {
            let feeds = r
                .feeds
                .iter()
                .map(|(n, k)| format!("{n}× {}", tr(k)))
                .collect::<Vec<_>>()
                .join(", ");
            rows.push(SRow {
                id: r.circuit,
                cells: vec![
                    name.clone(),
                    r.name,
                    r.breaker,
                    r.cable,
                    format!("{:.0}", r.power_w),
                    r.phase,
                    r.rcd,
                    feeds,
                ],
                group: false,
            });
        }
        let _ = id;
    }
    let listed = rows.len();
    Schedule {
        category: creabim_core::ScheduleCategory::BoardDiagram,
        columns,
        rows,
        totals: vec![],
        listed,
        above_totals: vec![],
    }
}

/// The drawing of the view `view`, named `title`.
pub fn drawing(view: ElementId, title: &str, model: &Model) -> PlanView {
    let mut items = vec![];
    let text = |items: &mut Vec<PlanItem>, at: DVec2, t: &str, size: f64, bold: bool| {
        items.push(PlanItem::Text {
            id: view,
            at,
            text: t.to_string(),
            size,
            align: Anchor::Start,
            mm: 1.0,
            rgb: None,
            bold,
        })
    };
    let line = |items: &mut Vec<PlanItem>, a: DVec2, b: DVec2, pen: Pen| {
        items.push(PlanItem::Line {
            id: view,
            pen,
            a,
            b,
        })
    };
    let p = DVec2::new;
    text(
        &mut items,
        p(0.0, 5.0 - HEAD / 2.0),
        &title.to_uppercase(),
        HEAD + 0.8,
        true,
    );
    let mut y = -2.0;
    let boards = boards(model);
    if boards.is_empty() {
        text(
            &mut items,
            p(0.0, y - TEXT),
            &tr("No distribution board in the model yet."),
            TEXT,
            false,
        );
    }
    for (_, name, rows) in &boards {
        let name = if name.is_empty() {
            tr("Circuits without a board")
        } else {
            name.clone()
        };
        let three = rows.iter().any(|r| r.three_phase);
        let feed = if three {
            "3/N/PE AC 400/230 V TN-C-S"
        } else {
            "1/N/PE AC 230 V TN-C-S"
        };
        text(&mut items, p(0.0, y - HEAD / 2.0), &name, HEAD, true);
        y -= HEAD + 2.0;
        // the feed down to the main switch, the switch, on to the busbar
        let top = y;
        line(&mut items, p(BUS, top), p(BUS, top - 4.0), Pen::Symbol);
        line(
            &mut items,
            p(BUS, top - 4.0),
            p(BUS + 2.5, top - 8.0),
            Pen::Symbol,
        );
        line(
            &mut items,
            p(BUS - 1.0, top - 4.0),
            p(BUS + 1.0, top - 4.0),
            Pen::Symbol,
        );
        line(
            &mut items,
            p(BUS, top - 8.5),
            p(BUS, top - 11.0),
            Pen::Symbol,
        );
        text(
            &mut items,
            p(BUS + 4.0, top - 1.5),
            &format!("{} {feed}", tr("Feed")),
            TEXT,
            false,
        );
        text(
            &mut items,
            p(BUS + 4.0, top - 6.5),
            &tr("Main switch"),
            TEXT,
            false,
        );
        y = top - 11.0;
        // the columns' heads
        for (x, h) in COLS.iter().zip([
            "Circuit", "Breaker", "Cable", "Power W", "Phase", "RCD", "Feeds",
        ]) {
            text(&mut items, p(*x, y - 2.5), &tr(h), TEXT, true);
        }
        let bus_top = y;
        y -= ROW / 2.0;
        for r in rows {
            let yc = y - ROW / 2.0;
            // the outgoing line, the breaker as a switch with its trip
            line(&mut items, p(BUS, yc), p(BUS + 6.0, yc), Pen::Symbol);
            line(
                &mut items,
                p(BUS + 6.0, yc),
                p(BUS + 11.0, yc + 2.5),
                Pen::Symbol,
            );
            for (a, b) in [
                (p(BUS + 7.5, yc - 0.6), p(BUS + 8.7, yc - 0.6)),
                (p(BUS + 8.7, yc - 0.6), p(BUS + 8.7, yc + 0.6)),
                (p(BUS + 8.7, yc + 0.6), p(BUS + 7.5, yc + 0.6)),
                (p(BUS + 7.5, yc + 0.6), p(BUS + 7.5, yc - 0.6)),
            ] {
                line(&mut items, a, b, Pen::Fine);
            }
            line(
                &mut items,
                p(BUS + 11.5, yc),
                p(COLS[0] - 2.0, yc),
                Pen::Symbol,
            );
            // an RCD: a ring on the line
            if !r.rcd.is_empty() {
                items.push(PlanItem::Arc {
                    id: view,
                    pen: Pen::Symbol,
                    center: p(BUS + 16.0, yc),
                    radius: 1.6,
                    start: 0.0,
                    end: std::f64::consts::TAU,
                });
            }
            // the phases: a tick each
            let ticks = if r.three_phase { 3 } else { 1 };
            for k in 0..ticks {
                let x = BUS + 24.0 + k as f64 * 1.4;
                line(
                    &mut items,
                    p(x - 0.9, yc - 1.4),
                    p(x + 0.9, yc + 1.4),
                    Pen::Fine,
                );
            }
            let feeds = r
                .feeds
                .iter()
                .map(|(n, k)| format!("{n}× {}", tr(k)))
                .collect::<Vec<_>>()
                .join(", ");
            let power = if r.power_w > 0.0 {
                format!("{:.0}", r.power_w)
            } else {
                "–".into()
            };
            for (x, t) in COLS.iter().zip([
                &r.name, &r.breaker, &r.cable, &power, &r.phase, &r.rcd, &feeds,
            ]) {
                text(&mut items, p(*x, yc - TEXT / 2.5), t, TEXT, false);
            }
            y -= ROW;
        }
        if rows.is_empty() {
            text(
                &mut items,
                p(COLS[0], y - TEXT),
                &tr("The board has no circuits."),
                TEXT,
                false,
            );
            y -= ROW;
        }
        // the busbar, heavy
        line(&mut items, p(BUS, bus_top), p(BUS, y + ROW / 2.0), Pen::Cut);
        y -= 8.0;
    }
    PlanView { items }
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::Transaction;

    #[test]
    fn the_starter_board_lists_its_proposed_circuits() {
        let mut m = creabim_core::starter::starter_house();
        let made = creabim_services::circuits::propose(
            &m,
            8,
            creabim_services::electrical::Method::default(),
        )
        .unwrap();
        let n = made.len();
        let mut tx = Transaction::new("circuits");
        for e in made {
            tx = tx.insert(e);
        }
        tx.apply(&mut m).unwrap();
        let b = boards(&m);
        let rows: usize = b.iter().map(|x| x.2.len()).sum();
        assert_eq!(rows, n);
        assert!(b.iter().flat_map(|x| &x.2).any(|r| !r.feeds.is_empty()));
        let d = drawing(ElementId::default(), "Board", &m);
        let texts = d
            .items
            .iter()
            .filter(|i| matches!(i, PlanItem::Text { .. }))
            .count();
        assert!(texts >= n * 7);
        assert_eq!(trim(2.5), "2,5");
        assert_eq!(trim(16.0), "16");
    }
}
