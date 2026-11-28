//! The tables of the site plan, written as the office writes them beside
//! the drawing: the area data (ÚDAJE O PLOCHÁCH) with the indices worked
//! out from the plot's area, the other data (OSTATNÉ: the roof's height
//! over the finished terrain), and the identification of the
//! construction (IDENTIFIKAČNÉ ÚDAJE) from the project information.

use creabim_core::words::tr;
use creabim_core::{ElementId, ElementKind, Model};
use glam::DVec2;

use crate::paper::Anchor;
use crate::plan::{PlanItem, PlanView};
use crate::schedule::{Column, Row, Schedule};

/// The site plan's figures: the areas, the indices and the rest, each a
/// label and its value as written.
pub struct AreaData {
    pub areas: Vec<(String, String)>,
    pub indices: Vec<(String, String)>,
    pub other: Vec<(String, String)>,
}

/// A number with `places` decimals, with a decimal comma where the
/// sheets are not in English.
fn num(v: f64, places: usize) -> String {
    let s = format!("{v:.places$}");
    if tr("Built-up area") == "Built-up area" {
        s
    } else {
        s.replace('.', ",")
    }
}

const DOTS: &str = "........";

/// The roofs' highest point over the project's zero, m; without a roof
/// the highest wall crown.
pub fn roof_top(model: &Model) -> Option<f64> {
    let roofs: Vec<ElementId> = model
        .iter()
        .filter(|e| matches!(e.kind, ElementKind::Roof(_)))
        .map(|e| e.id)
        .collect();
    if roofs.is_empty() {
        return None;
    }
    let mut top: Option<f64> = None;
    for it in crate::scene::build_scene(model) {
        if !roofs.contains(&it.id) || it.ghost {
            continue;
        }
        for p in &it.mesh.positions {
            let z = p[2] as f64;
            top = Some(top.map_or(z, |t: f64| t.max(z)));
        }
    }
    top
}

pub fn area_data(model: &Model) -> AreaData {
    let f = crate::summary::figures(model);
    let info = model
        .project_info()
        .map(|p| p.1.clone())
        .unwrap_or_default();
    let usable = crate::summary::usable_area(model);
    let m2 = |v: f64| format!("{} m²", num(v, 2));
    let plot = f.plot.filter(|p| *p > 0.0).map_or(DOTS.to_string(), |p| {
        // the cadastre gives whole square metres
        if (p - p.round()).abs() < 1e-6 {
            format!("{p:.0} m²")
        } else {
            m2(p)
        }
    });
    let areas = vec![
        (tr("Area of the plots owned"), plot),
        (tr("Total usable area"), m2(usable)),
        (tr("Built-up area"), m2(f.built_up)),
        (
            tr("Built-up area incl. paved areas"),
            m2(f.built_up + f.paved),
        ),
        (tr("Enclosed volume"), format!("{} m³", num(f.volume, 2))),
    ];
    let index = |v: Option<f64>| v.map_or(DOTS.to_string(), |v| num(v, 2));
    let indices = vec![
        (tr("Built-up index"), index(f.coverage())),
        (tr("Max. built-up index"), index(info.max_coverage)),
        (tr("Green coefficient"), index(f.green())),
    ];
    let top = roof_top(model).unwrap_or(f.height);
    // the finished terrain at the building, its lowest; ±0,000 without one
    let ground = f.ground.map_or(0.0, |g| g.0);
    let other = vec![(
        tr("Max. height of the roof above the finished terrain is"),
        format!("+ {}", num(top - ground, 3)),
    )];
    AreaData {
        areas,
        indices,
        other,
    }
}

/// The identification of the construction: a label and a value a row,
/// from the project information.
pub fn identification(model: &Model) -> Vec<(String, String)> {
    let i = model
        .project_info()
        .map(|p| p.1.clone())
        .unwrap_or_default();
    let kind = if i.building_name.trim().is_empty() {
        i.name.clone()
    } else {
        i.building_name.clone()
    };
    [
        ("Kind of construction", kind),
        ("Cadastral district", i.cadastre),
        ("Plot number", i.parcels),
        ("Place of the construction", i.address),
        ("Town", i.town),
        ("District", i.district),
        ("Region", i.region),
        ("Investor", i.client),
        ("Seat of the investor", i.client_address),
        ("Owner of the plot", i.land_owner),
        ("Sheet of ownership no.", i.ownership_sheet),
    ]
    .into_iter()
    .map(|(l, v)| (tr(l), v.trim().to_string()))
    .collect()
}

fn two_columns(category: creabim_core::ScheduleCategory, rows: Vec<Row>) -> Schedule {
    let col = |title: &'static str, width: f64, numeric: bool| Column {
        title,
        field: None,
        width,
        numeric,
        align: None,
    };
    Schedule {
        category,
        columns: vec![col("Figure", 80.0, false), col("Value", 40.0, true)],
        listed: rows.len(),
        rows,
        totals: vec![],
        above_totals: vec![],
    }
}

fn row(label: String, value: String, group: bool) -> Row {
    Row {
        id: ElementId::NIL,
        group,
        cells: vec![label, value],
    }
}

/// The area data as a table: its heading rows, the areas and indices,
/// then the other data.
pub fn area_schedule(model: &Model) -> Schedule {
    let d = area_data(model);
    let mut rows = vec![row(tr("AREA DATA"), String::new(), true)];
    rows.extend(
        d.areas
            .into_iter()
            .chain(d.indices)
            .map(|(l, v)| row(l, v, false)),
    );
    rows.push(row(tr("OTHER"), String::new(), true));
    rows.extend(d.other.into_iter().map(|(l, v)| row(l, v, false)));
    two_columns(creabim_core::ScheduleCategory::BuildingFigures, rows)
}

pub fn identification_schedule(model: &Model) -> Schedule {
    let rows = identification(model)
        .into_iter()
        .map(|(l, v)| row(l, v, false))
        .collect();
    two_columns(creabim_core::ScheduleCategory::Identification, rows)
}

const SIZE: f64 = 2.5;
const HEAD: f64 = 4.0;
const LEAD: f64 = 4.2;

fn text(d: &mut PlanView, id: ElementId, at: DVec2, t: &str, size: f64) {
    d.items.push(PlanItem::Text {
        id,
        at,
        text: t.to_string(),
        size,
        align: Anchor::Start,
        mm: 1.0,
        rgb: None,
        bold: false,
    });
}

/// Label, colon and value rows from `top` down at `x`, the colons at
/// `x + gap`; where the last row's baseline went.
fn label_rows(
    d: &mut PlanView,
    id: ElementId,
    x: f64,
    gap: f64,
    top: f64,
    rows: &[(String, String)],
) -> f64 {
    let mut y = top;
    for (l, v) in rows {
        y -= LEAD;
        text(d, id, DVec2::new(x, y), l, SIZE);
        let v = if v.is_empty() { DOTS } else { v.as_str() };
        text(d, id, DVec2::new(x + gap, y), &format!(": {v}"), SIZE);
    }
    y
}

fn widest(rows: &[(String, String)]) -> f64 {
    rows.iter()
        .map(|(l, _)| crate::paper::text_width(l, SIZE))
        .fold(0.0, f64::max)
}

/// ÚDAJE O PLOCHÁCH and OSTATNÉ as the sample site plans write them: the
/// areas in a column, the indices in one beside them, a colon before
/// each value, then the other data as sentences.
pub fn area_drawing(view: ElementId, model: &Model) -> PlanView {
    let d0 = area_data(model);
    let mut d = PlanView::default();
    text(
        &mut d,
        view,
        DVec2::new(0.0, -HEAD),
        &format!("{} :", tr("AREA DATA")),
        HEAD,
    );
    let top = -HEAD - 1.5;
    let gap = widest(&d0.areas) + 1.5;
    let value_w = d0
        .areas
        .iter()
        .map(|(_, v)| crate::paper::text_width(&format!(": {v}"), SIZE))
        .fold(0.0, f64::max);
    let left = label_rows(&mut d, view, 0.0, gap, top, &d0.areas);
    let x2 = gap + value_w + 4.0;
    let right = label_rows(
        &mut d,
        view,
        x2,
        widest(&d0.indices) + 6.0,
        top,
        &d0.indices,
    );
    let mut y = left.min(right) - LEAD - HEAD;
    text(
        &mut d,
        view,
        DVec2::new(0.0, y),
        &format!("{} :", tr("OTHER")),
        HEAD,
    );
    y -= 1.5;
    for (l, v) in &d0.other {
        y -= LEAD;
        text(&mut d, view, DVec2::new(0.0, y), &format!("{l} {v}"), SIZE);
    }
    d
}

/// IDENTIFIKAČNÉ ÚDAJE: a label, a colon and the value a row.
pub fn identification_drawing(view: ElementId, model: &Model) -> PlanView {
    let rows = identification(model);
    let mut d = PlanView::default();
    text(
        &mut d,
        view,
        DVec2::new(0.0, -HEAD),
        &tr("IDENTIFICATION DATA"),
        HEAD,
    );
    label_rows(&mut d, view, 0.0, widest(&rows) + 2.0, -HEAD - 1.5, &rows);
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    /// With the plot's area the indices are worked out; the plot's own
    /// maximum and the identification come from the project information.
    #[test]
    fn the_site_tables_read_the_project_and_the_plot() {
        let m = creabim_core::starter::starter_house();
        let info = creabim_core::ProjectInfo {
            plot_area: Some(617.0),
            max_coverage: Some(0.4),
            town: "Voderady".into(),
            ownership_sheet: "2482".into(),
            ..Default::default()
        };
        let m = Model::from_elements(
            m.iter()
                .filter(|e| !matches!(e.kind, ElementKind::Project(_)))
                .cloned()
                .chain([creabim_core::Element::new(
                    "Project",
                    ElementKind::Project(info),
                )]),
        )
        .unwrap();
        let id = m.project_info().unwrap().0;
        let d = area_data(&m);
        assert_eq!(d.areas[0].1, "617 m²");
        assert_eq!(d.indices[1].1, "0.40");
        assert_ne!(d.indices[0].1, DOTS);
        assert!(d.other[0].1.starts_with("+ "), "{:?}", d.other);
        let ids = identification(&m);
        assert!(ids.iter().any(|r| r.1 == "Voderady"));
        assert_eq!(ids.last().unwrap().1, "2482");
        assert!(!area_drawing(id, &m).items.is_empty());
        assert_eq!(identification_drawing(id, &m).items.len(), 1 + 2 * 11);
    }
}
