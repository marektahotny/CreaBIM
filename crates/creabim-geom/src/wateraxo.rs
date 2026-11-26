//! The axonometry of the water supply (axonometria vodovodu), as the
//! samples George gave (Vodovod – Axonometrie 3.NP, Axonometrie vodovodu
//! A3): every pipe of cold water, hot water and circulation drawn in an
//! oblique projection, the plan's x across, its y slanting up at 45° at
//! its full length, heights upright; each system in its colour, cold
//! water solid, hot water and circulation dashed, each pipe with its size
//! (`20x2,8`), the risers with their marks (V1), the fixtures with their
//! marks and the height of their outlets above the floor, the valves on
//! the pipes. Drawn on paper (1 unit = 1 mm) at [`SCALE`].

use crate::paper::Anchor;
use crate::plan::{Pen, PlanItem, PlanView};
use creabim_core::mep::{Pipe, SystemKind};
use creabim_core::words::tr;
use creabim_core::{ElementId, ElementKind, Model};
use glam::{DVec2, DVec3};

/// The scale it is drawn at, as the samples: 1:50.
pub const SCALE: f64 = 50.0;
const K: f64 = 1000.0 / SCALE;
const HEAD: f64 = 3.2;
const TEXT: f64 = 2.2;
const SMALL: f64 = 1.8;
/// A pipe shorter than this gets no size written, m.
const SHORT: f64 = 0.4;
/// How near a pipe's end must be to a fixture's connector, m.
const REACH: f64 = 0.03;

/// Is a pipe of this system drawn in it?
pub fn is_water(k: SystemKind) -> bool {
    matches!(
        k,
        SystemKind::ColdWater | SystemKind::HotWater | SystemKind::Circulation
    )
}

/// The water pipes of the model, with their ends in the model, m.
pub fn pipes(model: &Model) -> Vec<(ElementId, &Pipe, DVec3, DVec3)> {
    model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Pipe(p) if is_water(p.kind()) && p.length() > 1e-6 => {
                let z = DVec3::Z * model.level(p.level).map_or(0.0, |l| l.elevation);
                Some((e.id, p, p.start + z, p.end + z))
            }
            _ => None,
        })
        .collect()
}

/// Where a point of the model falls in the drawing, m: x across, y
/// slanting at 45° at its full length, z up.
pub fn project(p: DVec3) -> DVec2 {
    let c = std::f64::consts::FRAC_1_SQRT_2;
    DVec2::new(p.x + p.y * c, p.z + p.y * c)
}

fn rgb_of(c: u32) -> [f32; 3] {
    [
        ((c >> 16) & 0xff) as f32 / 255.0,
        ((c >> 8) & 0xff) as f32 / 255.0,
        (c & 0xff) as f32 / 255.0,
    ]
}

/// A pipe's size as the samples write it: outside diameter by wall,
/// `20x2,8`.
pub fn size_text(p: &Pipe) -> String {
    let n = |v: f64| {
        if (v - v.round()).abs() < 0.05 {
            format!("{v:.0}")
        } else {
            format!("{v:.1}").replace('.', ",")
        }
    };
    format!(
        "{}x{}",
        n(p.outer_mm),
        n((p.outer_mm - p.inner_mm).max(0.0) / 2.0)
    )
}

/// The fixtures the water pipes reach: each with its mark, where its
/// water connector is in the model and its floor's height, m.
pub fn fixtures(model: &Model) -> Vec<(ElementId, String, DVec3, f64)> {
    use creabim_core::fixture::{self, ConnKind};
    let ends: Vec<DVec3> = pipes(model).iter().flat_map(|p| [p.2, p.3]).collect();
    let marks: Vec<(ElementId, String)> = creabim_services::fixturespec::legend(model, None)
        .into_iter()
        .flat_map(|r| {
            let m = r.spec.mark.clone();
            r.fixtures.into_iter().map(move |f| (f, m.clone()))
        })
        .collect();
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Object(o) = &e.kind else {
            continue;
        };
        if fixture::family_of(model, e).is_none() {
            continue;
        }
        let floor = model.level(o.level).map_or(0.0, |l| l.elevation);
        let reached = fixture::connectors(model, e)
            .into_iter()
            .filter(|c| matches!(c.kind, ConnKind::SV | ConnKind::TV))
            .map(|c| c.at + DVec3::Z * floor)
            .find(|at| ends.iter().any(|q| q.distance(*at) <= REACH));
        let Some(at) = reached else {
            continue;
        };
        let mark = marks
            .iter()
            .find(|m| m.0 == e.id)
            .map_or(String::new(), |m| m.1.clone());
        out.push((e.id, mark, at, floor));
    }
    out
}

/// The drawing of the view `view`, named `title`.
pub fn drawing(view: ElementId, title: &str, model: &Model) -> PlanView {
    let mut items: Vec<PlanItem> = vec![];
    let green = Some(crate::mep::LABEL_GREEN);
    let text = |items: &mut Vec<PlanItem>, at: DVec2, t: &str, size: f64, align: Anchor| {
        items.push(PlanItem::Text {
            id: view,
            at,
            text: t.to_string(),
            size,
            align,
            mm: 1.0,
            rgb: green,
            bold: false,
        })
    };
    let p = DVec2::new;
    items.push(PlanItem::Text {
        id: view,
        at: p(0.0, 6.0),
        text: title.to_uppercase(),
        size: HEAD + 0.8,
        align: Anchor::Start,
        mm: 1.0,
        rgb: None,
        bold: true,
    });
    let list = pipes(model);
    if list.is_empty() {
        items.push(PlanItem::Text {
            id: view,
            at: p(0.0, -4.0),
            text: tr("No water pipe in the model yet."),
            size: TEXT,
            align: Anchor::Start,
            mm: 1.0,
            rgb: None,
            bold: false,
        });
        return PlanView { items };
    }
    let fx = fixtures(model);
    // the drawing's corner, under the title
    let mut lo = DVec2::splat(f64::MAX);
    let mut hi = DVec2::splat(f64::MIN);
    for (_, _, a, b) in &list {
        for q in [project(*a), project(*b)] {
            lo = lo.min(q);
            hi = hi.max(q);
        }
    }
    for (_, _, at, floor) in &fx {
        let q = project(DVec3::new(at.x, at.y, *floor));
        lo = lo.min(q);
    }
    let at = |q: DVec3| {
        let v = project(q);
        p(4.0 + (v.x - lo.x) * K, -14.0 - (hi.y - v.y) * K)
    };
    // the pipes, each system its colour, and their sizes
    for (id, pipe, a, b) in &list {
        let (pa, pb) = (at(*a), at(*b));
        let rgb = pipe
            .system_def()
            .map_or([0.2, 0.2, 0.2], |s| rgb_of(s.color));
        items.push(PlanItem::Curve {
            id: *id,
            points: vec![pa, pb],
            rgb,
            dashed: pipe.kind() != SystemKind::ColdWater,
        });
        if a.distance(*b) < SHORT || (pipe.is_vertical() && a.distance(*b) < 0.6) {
            continue;
        }
        let mid = (pa + pb) / 2.0;
        let d = (pb - pa).normalize_or_zero();
        let side = if d.x.abs() < 1e-3 {
            p(-1.2, 0.0)
        } else {
            d.perp() * if d.x > 0.0 { 1.0 } else { -1.0 } * 1.0
        };
        let align = if d.x.abs() < 1e-3 {
            Anchor::End
        } else {
            Anchor::Middle
        };
        text(&mut items, mid + side, &size_text(pipe), SMALL, align);
    }
    // the risers' marks, once a stack
    let mut marked: Vec<String> = vec![];
    for (_, pipe, a, b) in &list {
        if !pipe.is_vertical() {
            continue;
        }
        let Some(m) = crate::mep::stack_mark(model, pipe) else {
            continue;
        };
        if marked.contains(&m) {
            continue;
        }
        marked.push(m.clone());
        let top = if a.z > b.z { *a } else { *b };
        let c = at(top) + p(0.0, 6.0);
        items.push(PlanItem::Arc {
            id: view,
            pen: Pen::Projection,
            center: c,
            radius: 3.0,
            start: 0.0,
            end: std::f64::consts::TAU,
        });
        items.push(PlanItem::Line {
            id: view,
            pen: Pen::Fine,
            a: at(top),
            b: c - p(0.0, 3.0),
        });
        text(
            &mut items,
            c + p(0.0, -HEAD * 0.36),
            &m,
            HEAD - 0.4,
            Anchor::Middle,
        );
    }
    // the fixtures: their marks and their outlets' heights over the floor
    for (id, mark, q, floor) in &fx {
        let top = at(*q);
        let foot = at(DVec3::new(q.x, q.y, *floor));
        // the height of the outlet over the floor, beside it
        let off = p(3.0, 0.0);
        let tick = |items: &mut Vec<PlanItem>, c: DVec2| {
            items.push(PlanItem::Line {
                id: *id,
                pen: Pen::Fine,
                a: c - p(1.0, 0.0),
                b: c + p(1.0, 0.0),
            })
        };
        items.push(PlanItem::Line {
            id: *id,
            pen: Pen::Fine,
            a: foot + off,
            b: top + off,
        });
        tick(&mut items, foot + off);
        tick(&mut items, top + off);
        let h = ((q.z - floor) * 1000.0).round();
        if h > 0.0 {
            text(
                &mut items,
                (foot + top) / 2.0 + off + p(0.8, -0.8),
                &format!("{h:.0}"),
                SMALL,
                Anchor::Start,
            );
        }
        if !mark.is_empty() {
            text(
                &mut items,
                top + p(-1.2, 0.6),
                mark,
                TEXT + 0.4,
                Anchor::End,
            );
        }
    }
    // the valves on the pipes: a bow tie across the pipe
    for e in model.iter() {
        let ElementKind::Valve(v) = &e.kind else {
            continue;
        };
        let Some((_, pipe, a, b)) = list.iter().find(|l| l.0 == v.pipe) else {
            continue;
        };
        let c = at(a.lerp(*b, v.t));
        let d = (at(*b) - at(*a)).normalize_or_zero();
        let n = d.perp();
        let r = 1.0;
        let pts = vec![
            c - d * r + n * r,
            c + d * r - n * r,
            c + d * r + n * r,
            c - d * r - n * r,
        ];
        items.push(PlanItem::Polygon {
            id: e.id,
            pen: Pen::Projection,
            points: pts,
        });
        let _ = pipe;
    }
    PlanView { items }
}

/// The water pipes as a table, for the export to a spreadsheet: their
/// length by system and size.
pub fn schedule(model: &Model) -> crate::schedule::Schedule {
    use crate::schedule::{Column, Row, Schedule};
    let col = |title: &'static str, width: f64, numeric: bool| Column {
        title,
        field: None,
        width,
        numeric,
        align: None,
    };
    let columns = vec![
        col("System", 40.0, false),
        col("Size", 20.0, false),
        col("Length m", 20.0, true),
    ];
    let mut sums: Vec<(String, String, f64)> = vec![];
    for (_, pipe, a, b) in pipes(model) {
        let sys = pipe
            .system_def()
            .map_or(pipe.system.clone(), |s| tr(s.name));
        let size = size_text(pipe);
        match sums.iter_mut().find(|s| s.0 == sys && s.1 == size) {
            Some(s) => s.2 += a.distance(b),
            None => sums.push((sys, size, a.distance(b))),
        }
    }
    let rows: Vec<Row> = sums
        .into_iter()
        .map(|(s, z, l)| Row {
            id: ElementId::default(),
            cells: vec![s, z, format!("{l:.2}").replace('.', ",")],
            group: false,
        })
        .collect();
    let listed = rows.len();
    Schedule {
        category: creabim_core::ScheduleCategory::WaterAxonometry,
        columns,
        rows,
        totals: vec![],
        listed,
        above_totals: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::{Document, Element, Transaction};

    #[test]
    fn a_basin_fed_by_cold_and_hot_water_is_drawn_with_sizes_and_its_outlet() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let o = creabim_core::catalogue::starter("starter/washbasin").unwrap();
        let (tx, id) =
            creabim_core::objects::place(doc.model(), &o, level, DVec2::new(1.0, 3.0), 0.0);
        doc.commit(tx).unwrap();
        let e = doc.model().get(id).unwrap();
        let conns: Vec<_> = creabim_core::fixture::connectors(doc.model(), e)
            .into_iter()
            .filter(|c| {
                matches!(
                    c.kind,
                    creabim_core::fixture::ConnKind::SV | creabim_core::fixture::ConnKind::TV
                )
            })
            .collect();
        assert!(!conns.is_empty());
        let pipe = |sys: &str, a: DVec3, b: DVec3| {
            Element::new(
                "",
                ElementKind::Pipe(Pipe {
                    level,
                    system: sys.into(),
                    start: a,
                    end: b,
                    pipe_type: "HERZ".into(),
                    nominal_mm: 20.0,
                    outer_mm: 20.0,
                    inner_mm: 14.4,
                    height_mm: None,
                    insulation_mm: 0.0,
                    insulation_set: false,
                    mark: None,
                }),
            )
        };
        let mut tx = Transaction::new("water");
        for c in &conns {
            let sys = if c.kind == creabim_core::fixture::ConnKind::SV {
                "SV"
            } else {
                "TV"
            };
            let low = DVec3::new(c.at.x, c.at.y, 0.3);
            tx = tx
                .insert(pipe(sys, c.at, low))
                .insert(pipe(sys, low, low + DVec3::X * 2.0));
        }
        doc.commit(tx).unwrap();
        let fx = fixtures(doc.model());
        assert_eq!(fx.len(), 1);
        let d = drawing(ElementId::default(), "Axonometria", doc.model());
        let texts: Vec<&str> = d
            .items
            .iter()
            .filter_map(|it| match it {
                PlanItem::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"20x2,8"), "{texts:?}");
        let h = format!("{:.0}", (fx[0].2.z - fx[0].3) * 1000.0);
        assert!(texts.contains(&h.as_str()), "{texts:?}");
        // y slants at 45° at its full length
        let q = project(DVec3::new(0.0, 1.0, 0.0));
        assert!((q.length() - 1.0).abs() < 1e-9);
    }
}
