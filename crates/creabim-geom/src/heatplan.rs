//! A heating drawing's underfloor heating, as the office draws it: the
//! loops' supply a full red line and their return a dashed blue one, in
//! every heated room its circuits' table (the manifold and circuit, the
//! supply temperature, the spacing, a loop's length, the pipe, the loss
//! and flow, the floor laid and the room's temperature and loss), and
//! the manifold's mark with the number of its circuits.

use crate::annotation::{Annotation, Shape, TextLook};
use crate::plan::PlanItem;
use creabim_core::{fixture, room, ElementId, ElementKind, Model};
use creabim_services::{floorheat, heatloss};
use glam::DVec2;

/// The supply's and the return's colour.
pub const SUPPLY: [f32; 3] = [0.84, 0.1, 0.1];
pub const RETURN: [f32; 3] = [0.1, 0.3, 0.82];
/// The return's dash and gap, paper mm.
pub const DASH: (f64, f64) = (4.0, 1.5);
/// A loop's line, paper mm.
const WIDTH: f64 = 0.25;
/// The tables' letters, paper mm.
const TEXT: f64 = 2.0;

/// The line through `pts` as thin strips in `rgb`, `dashes` (dash and
/// gap, paper mm) running on round its corners; `mm` drawing units make
/// a paper millimetre.
pub fn strips(
    items: &mut Vec<PlanItem>,
    id: ElementId,
    pts: &[DVec2],
    rgb: [f32; 3],
    dashes: Option<(f64, f64)>,
    mm: f64,
) {
    let half = WIDTH / 2.0 * mm;
    let mut strip = |a: DVec2, b: DVec2| {
        let n = (b - a).perp().normalize_or_zero() * half;
        items.push(PlanItem::Shade {
            id,
            points: vec![a - n, b - n, b + n, a + n],
            rgb,
        })
    };
    // how far into the dash-and-gap the line is
    let mut phase = 0.0;
    for w in pts.windows(2) {
        let len = w[0].distance(w[1]);
        if len < 1e-9 {
            continue;
        }
        let u = (w[1] - w[0]) / len;
        let Some((dash, gap)) = dashes.map(|(d, g)| (d * mm, g * mm)) else {
            strip(w[0], w[1]);
            continue;
        };
        let mut x = -phase;
        while x < len {
            let (s, e) = (x.max(0.0), (x + dash).min(len));
            if e - s > 1e-9 {
                strip(w[0] + u * s, w[0] + u * e);
            }
            x += dash + gap;
        }
        phase = (phase + len) % (dash + gap);
    }
}

fn num(x: f64, places: usize) -> String {
    format!("{x:.places$}")
}

/// The underfloor heating of the rooms of `level` in a plan where `mm`
/// drawing units make a paper millimetre.
pub fn items(model: &Model, level: ElementId, mm: f64) -> Vec<PlanItem> {
    let mut rooms: Vec<_> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Room(r) if r.level == level => {
                Some((e, r, r.ufh.as_ref()?, room::shape_of(model, r)?))
            }
            _ => None,
        })
        .collect();
    if rooms.is_empty() {
        return vec![];
    }
    rooms.sort_by(|a, b| (&a.1.number, &a.0.name).cmp(&(&b.1.number, &b.0.name)));
    let settings = floorheat::Settings::default();
    let design = floorheat::design(model, &heatloss::Opts::default(), &settings);
    let storey = model.get(level).map_or(String::new(), |e| e.name.clone());
    let pipe = format!(
        "{}x{}",
        floorheat::PIPE_OUTER_MM,
        (floorheat::PIPE_OUTER_MM - floorheat::PIPE_INNER_MM) / 2.0
    );
    let mut out = vec![];
    let mut tables = vec![];
    let mut circuits = 0;
    for (e, r, u, shape) in &rooms {
        let loops = creabim_core::underfloor::loops(&shape.outline, u);
        for sp in &loops {
            strips(&mut out, e.id, &sp.supply, SUPPLY, None, mm);
            strips(&mut out, e.id, &sp.ret, RETURN, Some(DASH), mm);
        }
        let n = loops.len().max(1);
        let which = if n > 1 {
            format!("{}-{}", circuits + 1, circuits + n)
        } else {
            format!("{}", circuits + 1)
        };
        circuits += n;
        tables.push((
            e.id,
            r.at,
            which,
            design.rooms.iter().find(|x| x.id == e.id),
        ));
    }
    let look = TextLook {
        size: TEXT,
        ..Default::default()
    };
    let note = |id, at, text: String, look| PlanItem::Annotation {
        id,
        item: Box::new(Annotation {
            shape: Shape::Along { at, angle: 0.0 },
            text,
            look,
            mm,
        }),
    };
    for (id, at, which, d) in tables {
        let mut lines = vec![
            format!("RZ 1 - {storey} ({which}/{circuits})"),
            format!("tp = {} °C", num(settings.supply_c, 0)),
        ];
        if let Some(d) = d {
            lines.extend([
                format!("Lpz = {} mm", num(d.spacing * 1000.0, 0)),
                format!("L = {} m", num(d.loop_len, 1)),
                pipe.clone(),
                format!("Δp = {} kPa", num(d.dp_kpa, 1)),
                format!("{} l/min", num(d.flow_ls * 60.0, 2)),
                format!("A = {} m²", num(d.area, 1)),
                format!("{} °C, {} W", num(d.ti, 0), num(d.phi, 0)),
            ]);
        }
        // under the room's name
        let half = lines.len() as f64 * TEXT * 1.25 / 2.0;
        let at = at - DVec2::Y * (9.0 + half) * mm;
        out.push(note(id, at, lines.join("\n"), look));
    }
    // the manifold's mark: its number and how many circuits leave it
    for e in model.iter() {
        let ElementKind::Object(ob) = &e.kind else {
            continue;
        };
        if ob.level == level && fixture::family_of(model, e).is_some_and(fixture::is_manifold) {
            let look = TextLook {
                size: 3.0,
                rgb: Some(crate::mep::LABEL_GREEN),
                ..Default::default()
            };
            let at = ob.to_plan(DVec2::ZERO) + DVec2::Y * 5.0 * mm;
            out.push(note(e.id, at, format!("RZ 1/{circuits}"), look));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::underfloor::Underfloor;
    use creabim_core::{Document, Element, Room, Transaction};

    #[test]
    fn dashes_run_on_round_corners() {
        let mut items = vec![];
        let id = ElementId::new();
        let pts = [DVec2::ZERO, DVec2::X * 5.0, DVec2::new(5.0, 6.0)];
        strips(&mut items, id, &pts, RETURN, Some((4.0, 1.5)), 1.0);
        // 11 mm of 4 + 1.5: dashes at 0-4, 5.5-9.5 (round the corner:
        // one strip a leg), 11
        let len: f64 = items
            .iter()
            .map(|i| match i {
                PlanItem::Shade { points, .. } => points[0].distance(points[1]),
                _ => 0.0,
            })
            .sum();
        assert!((len - 8.0).abs() < 1e-6, "{len}");
        let mut full = vec![];
        strips(&mut full, id, &pts, SUPPLY, None, 1.0);
        assert_eq!(full.len(), 2);
    }

    #[test]
    fn a_heated_room_gets_coloured_loops_and_its_table() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        assert!(items(doc.model(), level, 0.1).is_empty());
        let mut r = Room::new(level, DVec2::new(2.5, 3.0));
        r.number = "1.01".into();
        r.ufh = Some(Underfloor {
            spacing: 0.15,
            loops: 2,
            finish_r: None,
        });
        let room = Element::new("Kúpeľňa", ElementKind::Room(r));
        let id = room.id;
        doc.commit(Transaction::new("room").insert(room)).unwrap();
        let out = items(doc.model(), level, 0.1);
        let count = |c: [f32; 3]| {
            out.iter()
                .filter(|i| matches!(i, PlanItem::Shade { rgb, .. } if *rgb == c))
                .count()
        };
        // the return dashed: many more strips than the supply's
        assert!(count(SUPPLY) > 10 && count(RETURN) > 3 * count(SUPPLY));
        let table = out
            .iter()
            .find_map(|i| match i {
                PlanItem::Annotation { id: i, item } if *i == id => Some(item.text.clone()),
                _ => None,
            })
            .unwrap();
        assert!(table.contains("(1-2/2)"), "{table}");
        assert!(
            table.contains("Lpz = 150 mm") && table.contains("16x2"),
            "{table}"
        );
        assert!(table.contains("l/min") && table.contains("kPa"), "{table}");
    }
}
