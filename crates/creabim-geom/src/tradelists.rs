//! The take-offs of the services' trades, as tables: the pipes and ducts
//! by system, type and size with their lengths and the valves on them
//! by kind (ZTI, heating, ventilation), and the electrical devices by
//! kind and type with the cables and breakers of the circuits.

use crate::schedule::{Column, Row, Schedule};
use creabim_core::mep::SYSTEMS;
use creabim_core::words::tr;
use creabim_core::{ElementId, ElementKind, Model, ScheduleCategory};

fn col(title: &'static str, width: f64, numeric: bool) -> Column {
    Column {
        title,
        field: None,
        width,
        numeric,
        align: None,
    }
}

fn columns(first: &'static str) -> Vec<Column> {
    vec![
        col(first, 46.0, false),
        col("Item", 70.0, false),
        col("Size", 30.0, false),
        col("Quantity", 22.0, true),
        col("Unit", 14.0, false),
    ]
}

fn metres(v: f64) -> String {
    format!("{v:.1}").replace('.', ",")
}

fn row(id: ElementId, cells: [String; 5]) -> Row {
    Row {
        id,
        cells: cells.into(),
        group: false,
    }
}

/// The pipes and ducts: a row for each system, type and size with the
/// length laid, in the systems' order, the smallest size first; under a
/// system's pipes its valves by kind, counted.
pub fn pipes(model: &Model, trade: Option<creabim_core::viewmodes::Trade>) -> Schedule {
    let mut rows = vec![];
    let mut total = 0.0;
    let fittings = if trade == Some(creabim_core::viewmodes::Trade::Heating) {
        creabim_services::fittings::derive(model)
    } else {
        vec![]
    };
    for s in SYSTEMS {
        if trade.is_some_and(|t| !t.shows(s.kind) || s.kind == creabim_core::mep::SystemKind::Other)
        {
            continue;
        }
        // (type's name, size's label, nominal, length, first pipe)
        let mut sizes: Vec<(String, String, f64, f64, ElementId)> = vec![];
        // (kind's name, count, first valve)
        let mut valves: Vec<(String, usize, ElementId)> = vec![];
        for e in model.iter() {
            match &e.kind {
                ElementKind::Pipe(p) if p.system == s.code => {
                    let t = creabim_services::pipe_types::by_id(&p.pipe_type);
                    let name = t.map_or(p.pipe_type.clone(), |t| t.name.clone());
                    let size = t.map_or_else(
                        || format!("{:.0}", p.nominal_mm),
                        |t| t.size_label(p.nominal_mm, p.height_mm),
                    );
                    match sizes.iter_mut().find(|x| x.0 == name && x.1 == size) {
                        Some(x) => x.3 += p.length(),
                        None => sizes.push((name, size, p.nominal_mm, p.length(), e.id)),
                    }
                }
                ElementKind::Valve(v) => {
                    let on = matches!(model.get(v.pipe).map(|h| &h.kind),
                        Some(ElementKind::Pipe(p)) if p.system == s.code);
                    if !on {
                        continue;
                    }
                    let name = creabim_core::mep::valve_kind(&v.kind)
                        .map_or(v.kind.clone(), |k| k.name.to_string());
                    match valves.iter_mut().find(|x| x.0 == name) {
                        Some(x) => x.1 += 1,
                        None => valves.push((name, 1, e.id)),
                    }
                }
                _ => {}
            }
        }
        sizes.sort_by(|a, b| a.0.cmp(&b.0).then(a.2.total_cmp(&b.2)));
        let system = format!("{} - {}", s.code, tr(s.name));
        for (name, size, _, length, id) in sizes {
            total += length;
            rows.push(row(
                id,
                [system.clone(), name, size, metres(length), "m".into()],
            ));
        }
        // the fittings where its pipes meet (heating: George 2026-11-27)
        if trade == Some(creabim_core::viewmodes::Trade::Heating) {
            let found: Vec<creabim_services::fittings::Fitting> = fittings
                .iter()
                .filter(|f| f.system == s.code)
                .cloned()
                .collect();
            for l in creabim_services::fittings::summary(&found) {
                let id = found
                    .iter()
                    .find(|f| f.label == l.label)
                    .and_then(|f| f.pipes.first().copied())
                    .unwrap_or_else(ElementId::new);
                let name = if l.code.is_empty() {
                    l.label
                } else {
                    format!("{} ({})", l.label, l.code)
                };
                rows.push(row(
                    id,
                    [
                        system.clone(),
                        name,
                        String::new(),
                        l.count.to_string(),
                        tr("pcs"),
                    ],
                ));
            }
        }
        for (name, n, id) in valves {
            rows.push(row(
                id,
                [
                    system.clone(),
                    name,
                    String::new(),
                    n.to_string(),
                    tr("pcs"),
                ],
            ));
        }
    }
    if trade == Some(creabim_core::viewmodes::Trade::Heating) {
        heating_rows(model, &mut rows);
    }
    let listed = rows.len();
    Schedule {
        category: ScheduleCategory::PipeList,
        columns: columns("System"),
        rows,
        totals: vec![
            tr("Total"),
            String::new(),
            String::new(),
            metres(total),
            "m".into(),
        ],
        listed,
        above_totals: vec![],
    }
}

/// The heating's bill beyond its pipes and valves (George 2026-11-27):
/// the drawn pipes' insulation as the maker's tubes, the heat sources,
/// manifolds and radiators by type, and the underfloor heating as
/// designed (system boards, loops' pipe and coils, manifolds and their
/// cabinets, the pipes to the radiators on them and their insulation).
fn heating_rows(model: &Model, rows: &mut Vec<Row>) {
    use creabim_core::fixture::{self, ConnKind};
    use creabim_core::mep::SystemKind;
    use creabim_services::floorheat;
    let s: floorheat::Settings =
        creabim_core::results::settings_of(model, "floorheat").unwrap_or_default();
    let qty = |v: f64, unit: &str| {
        if unit == "pcs" {
            format!("{v:.0}")
        } else {
            metres(v)
        }
    };
    // the drawn pipes' insulation, by tube
    let insulation = tr("Insulation");
    let mut tubes: Vec<(String, String, f64, ElementId)> = vec![];
    for e in model.iter() {
        let ElementKind::Pipe(p) = &e.kind else {
            continue;
        };
        let medium = match p.kind() {
            SystemKind::HeatingSupply => s.radiator_supply_c,
            SystemKind::HeatingReturn => s.radiator_return_c,
            _ => continue,
        };
        let Some(t) = s.insulation.tube(p.outer_mm, p.insulation(), medium) else {
            continue;
        };
        let label = if t.code.is_empty() {
            t.name.clone()
        } else {
            format!("{} ({})", t.name, t.code)
        };
        match tubes.iter_mut().find(|x| x.0 == label) {
            Some(x) => x.2 += p.length(),
            None => tubes.push((label, format!("{} mm", t.wall_mm), p.length(), e.id)),
        }
    }
    for (name, size, len, id) in tubes {
        rows.push(row(
            id,
            [insulation.clone(), name, size, metres(len), "m".into()],
        ));
    }
    // the heat sources, manifolds and radiators: objects on heating
    let devices = tr("Heating devices");
    let mut objects: Vec<(String, usize, ElementId)> = vec![];
    for e in model.iter() {
        if !matches!(e.kind, ElementKind::Object(_)) {
            continue;
        }
        let on_heating = fixture::connectors(model, e)
            .iter()
            .any(|c| matches!(c.kind, ConnKind::UKP | ConnKind::UKS));
        let family = fixture::family_of(model, e);
        if !on_heating && !family.is_some_and(|f| fixture::is_heating(f) || fixture::is_manifold(f))
        {
            continue;
        }
        let name = tr(&e.name);
        match objects.iter_mut().find(|x| x.0 == name) {
            Some(x) => x.1 += 1,
            None => objects.push((name, 1, e.id)),
        }
    }
    objects.sort_by(|a, b| a.0.cmp(&b.0));
    for (name, n, id) in objects {
        rows.push(row(
            id,
            [
                devices.clone(),
                name,
                String::new(),
                n.to_string(),
                tr("pcs"),
            ],
        ));
    }
    // the underfloor heating as designed
    let ufh = tr("Underfloor heating");
    for l in floorheat::model_bill(model) {
        let name = if l.code.is_empty() {
            l.item
        } else {
            format!("{} ({})", l.item, l.code)
        };
        let unit = if l.unit == "pcs" {
            tr("pcs")
        } else {
            l.unit.to_string()
        };
        rows.push(row(
            l.id,
            [ufh.clone(), name, l.size, qty(l.quantity, l.unit), unit],
        ));
    }
}

/// A circuit's cable as the list names it: "CYKY-J 3x2,5".
fn cable(three_phase: bool, mm2: f64) -> String {
    let s = if mm2.fract().abs() < 1e-9 {
        format!("{mm2:.0}")
    } else {
        format!("{mm2}").replace('.', ",")
    };
    format!("CYKY-J {}x{s}", if three_phase { 5 } else { 3 })
}

/// The electrical take-off: the devices by kind and type, counted; the
/// cables of the circuits by their cross-section with the length; the
/// breakers and the residual current devices by their rating.
pub fn electrical(model: &Model) -> Schedule {
    use creabim_core::electrical as el;
    let mut rows = vec![];
    // (kind, type's name, count, first)
    let mut devices: Vec<(el::DeviceKind, String, usize, ElementId)> = vec![];
    for (e, _, kind) in el::devices(model) {
        let name = e
            .of_type
            .as_ref()
            .and_then(|r| model.get(r.id))
            .map_or(e.name.clone(), |t| t.name.clone());
        match devices.iter_mut().find(|d| d.0 == kind && d.1 == name) {
            Some(d) => d.2 += 1,
            None => devices.push((kind, name, 1, e.id)),
        }
    }
    devices.sort_by_key(|d| d.0 as u8);
    for (kind, name, n, id) in devices {
        rows.push(row(
            id,
            [
                tr(kind.label()),
                name,
                String::new(),
                n.to_string(),
                tr("pcs"),
            ],
        ));
    }
    // (cable, length, first circuit)
    let mut cables: Vec<(String, f64, ElementId)> = vec![];
    // (what, rating, count, first circuit)
    let mut gear: Vec<(&'static str, String, usize, ElementId)> = vec![];
    for (e, c) in el::circuits(model) {
        if let Some(mm2) = c.cable_mm2 {
            let name = cable(c.three_phase, mm2);
            let length = c
                .length
                .unwrap_or_else(|| creabim_services::circuits::measured_length(model, c));
            match cables.iter_mut().find(|x| x.0 == name) {
                Some(x) => x.1 += length,
                None => cables.push((name, length, e.id)),
            }
        }
        let mut count = |what: &'static str, rating: String| match gear
            .iter_mut()
            .find(|g| g.0 == what && g.1 == rating)
        {
            Some(g) => g.2 += 1,
            None => gear.push((what, rating, 1, e.id)),
        };
        if let Some(a) = c.breaker_a {
            count(
                "Circuit breaker",
                format!("B{a:.0}/{}", if c.three_phase { 3 } else { 1 }),
            );
        }
        if let Some(ma) = c.rcd_ma {
            count("Residual current device", format!("{ma:.0} mA"));
        }
    }
    for (name, length, id) in cables {
        rows.push(row(
            id,
            [tr("Cable"), name, String::new(), metres(length), "m".into()],
        ));
    }
    for (what, rating, n, id) in gear {
        rows.push(row(
            id,
            [tr(what), String::new(), rating, n.to_string(), tr("pcs")],
        ));
    }
    let listed = rows.len();
    Schedule {
        category: ScheduleCategory::ElectricalList,
        columns: columns("Kind"),
        rows,
        totals: vec![],
        listed,
        above_totals: vec![],
    }
}

/// The take-off VZT – recuperation: the units, boxes, ducts and
/// terminals of the ventilation with heat recovery as the model has
/// them, priced from the catalogue (without VAT, the VAT under it).
pub fn recuperation(model: &Model) -> Schedule {
    let bill = creabim_services::recuperation::bill(model);
    let money = |v: f64| format!("{v:.2}").replace('.', ",");
    let qty = |v: f64| {
        if v.fract() == 0.0 {
            format!("{v:.0}")
        } else {
            format!("{v:.1}").replace('.', ",")
        }
    };
    let rows: Vec<Row> = bill
        .lines
        .iter()
        .map(|l| Row {
            id: ElementId::NIL,
            cells: vec![
                l.group.to_string(),
                l.code.clone(),
                l.name.clone(),
                qty(l.qty),
                l.unit.clone(),
                l.price.map_or_else(|| tr("on request"), money),
                l.total().map_or(String::new(), money),
            ],
            group: false,
        })
        .collect();
    let listed = rows.len();
    let mut rows = rows;
    let cat = creabim_services::recuperation::catalogue();
    for (what, v) in [
        (tr("Without VAT"), bill.net),
        (format!("{} {:.0} %", tr("VAT"), cat.vat * 100.0), bill.vat),
    ] {
        rows.push(Row {
            id: ElementId::NIL,
            cells: vec![
                what,
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                money(v),
            ],
            group: true,
        });
    }
    Schedule {
        category: ScheduleCategory::VentilationHrv,
        columns: vec![
            col("Group", 30.0, false),
            col("Code", 26.0, false),
            col("Item", 90.0, false),
            col("Qty", 16.0, true),
            col("Unit of measure", 10.0, false),
            col("€/unit", 18.0, true),
            col("€", 22.0, true),
        ],
        rows,
        totals: vec![
            tr("with VAT"),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            money(bill.gross),
        ],
        listed,
        above_totals: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::mep::Pipe;
    use creabim_core::{Document, Element, Transaction};
    use glam::DVec3;

    #[test]
    fn pipes_of_one_system_type_and_size_add_up_to_one_row() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        assert!(pipes(doc.model(), None).rows.is_empty());
        let mut tx = Transaction::new("pipes");
        for (system, len) in [("TV", 2.0), ("SV", 1.5), ("SV", 2.5)] {
            tx = tx.insert(Element::new(
                "Pipe",
                ElementKind::Pipe(Pipe {
                    level,
                    system: system.into(),
                    start: DVec3::ZERO,
                    end: DVec3::X * len,
                    pipe_type: "HERZ".into(),
                    nominal_mm: 16.0,
                    outer_mm: 20.0,
                    inner_mm: 16.0,
                    height_mm: None,
                    insulation_mm: 0.0,
                    insulation_set: false,
                    mark: None,
                }),
            ));
        }
        doc.commit(tx).unwrap();
        let s = pipes(doc.model(), None);
        assert_eq!(s.rows.len(), 2);
        // cold water first, its two pipes together
        assert!(s.rows[0].cells[0].starts_with("SV"));
        assert_eq!(s.rows[0].cells[3], "4,0");
        assert_eq!(s.rows[1].cells[3], "2,0");
        assert_eq!(s.totals[3], "6,0");
        // one trade's list leaves the others' pipes out
        use creabim_core::viewmodes::Trade;
        assert_eq!(pipes(doc.model(), Some(Trade::Plumbing)).rows.len(), 2);
        assert!(pipes(doc.model(), Some(Trade::Heating)).rows.is_empty());
        // through the schedules' door too
        let by = crate::schedule::schedule(doc.model(), ScheduleCategory::PipeList);
        assert_eq!(by.rows.len(), 2);
    }

    #[test]
    fn a_cable_is_named_by_its_wires_and_cross_section() {
        assert_eq!(cable(false, 2.5), "CYKY-J 3x2,5");
        assert_eq!(cable(true, 4.0), "CYKY-J 5x4");
        let m = creabim_core::sample::sample_house();
        // no devices, no rows; the table still has its columns
        let s = electrical(&m);
        assert_eq!(s.columns.len(), 5);
        assert_eq!(s.listed, s.rows.len());
    }
}

/// The heaters standing in the model, KORADO radiators and towel
/// radiators by size, with their output at 75/65/20 °C, and under them
/// the fittings each is connected with, by order code, counted.
pub fn heaters(model: &Model) -> Schedule {
    use creabim_core::catalogue::radiators::Heater;
    let mut by_size: Vec<(Heater, Vec<ElementId>)> = vec![];
    for e in model.iter() {
        if !matches!(e.kind, ElementKind::Object(_)) {
            continue;
        }
        let Some(h) = creabim_core::objects::type_of(model, e)
            .and_then(|t| t.catalogue.as_deref())
            .and_then(Heater::of_key)
        else {
            continue;
        };
        match by_size.iter_mut().find(|x| x.0 == h) {
            Some(x) => x.1.push(e.id),
            None => by_size.push((h, vec![e.id])),
        }
    }
    by_size.sort_by_key(|x| x.0.label());
    let mut rows = vec![];
    let mut fittings: Vec<(String, String, usize)> = vec![];
    for (h, ids) in &by_size {
        let what = match h {
            Heater::Panel(p) => format!(
                "KORADO RADIK VK {} × {} mm, typ {}",
                p.height,
                p.length,
                p.kind.number()
            ),
            Heater::Towel(t) => format!(
                "KORADO KORALUX LINEAR CLASSIC-M {} × {} mm",
                t.height, t.length
            ),
        };
        let n = h
            .exponent()
            .map_or_else(String::new, |n| format!("{n:.4}").replace('.', ","));
        rows.push(Row {
            id: ids[0],
            cells: vec![
                h.label(),
                h.code().unwrap_or_default(),
                what,
                format!("{:.0}", h.nominal_w()),
                n,
                ids.len().to_string(),
                tr("pcs"),
            ],
            group: false,
        });
        for (code, name) in h.fittings() {
            match fittings.iter_mut().find(|f| f.0 == code) {
                Some(f) => f.2 += ids.len(),
                None => fittings.push((code, name, ids.len())),
            }
        }
    }
    let listed = rows.len();
    if !fittings.is_empty() {
        rows.push(Row {
            id: ElementId::NIL,
            cells: vec![
                tr("Fittings"),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ],
            group: true,
        });
    }
    for (code, name, n) in fittings {
        rows.push(Row {
            id: ElementId::NIL,
            cells: vec![
                String::new(),
                code,
                name,
                String::new(),
                String::new(),
                n.to_string(),
                tr("pcs"),
            ],
            group: false,
        });
    }
    Schedule {
        category: ScheduleCategory::HeaterList,
        columns: vec![
            col("Mark", 24.0, false),
            col("Code", 30.0, false),
            col("Item", 84.0, false),
            col("Qn 75/65/20 W", 20.0, true),
            col("n", 14.0, true),
            col("Qty", 12.0, true),
            col("Unit of measure", 10.0, false),
        ],
        rows,
        totals: vec![],
        listed,
        above_totals: vec![],
    }
}

#[cfg(test)]
mod heater_tests {
    use super::*;

    #[test]
    fn heaters_are_listed_with_their_fittings() {
        use creabim_core::catalogue::radiators::{Panel, PanelKind};
        use creabim_core::catalogue::towels::TowelRail;
        let mut doc = creabim_core::Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let p = Panel {
            kind: PanelKind::K22,
            height: 600,
            length: 1000,
        };
        let t = TowelRail {
            height: 1220,
            length: 600,
        };
        for (c, x) in [(p.object(), 1.0), (p.object(), 3.0), (t.object(), 5.0)] {
            let (tx, _) =
                creabim_core::objects::place(doc.model(), &c, level, glam::DVec2::new(x, 1.0), 0.0);
            doc.commit(tx).unwrap();
        }
        let s = heaters(doc.model());
        assert_eq!(s.listed, 2);
        let cell = |code: &str| {
            s.rows
                .iter()
                .find(|r| r.cells[1] == code)
                .map(|r| r.cells[5].clone())
        };
        assert_eq!(cell("KLC-122060-00M10").as_deref(), Some("1"));
        assert_eq!(cell("0530-50.000").as_deref(), Some("2"));
        assert_eq!(cell("6000-09.500").as_deref(), Some("2"));
        assert_eq!(cell("Z-D040").as_deref(), Some("1"));
        assert_eq!(cell("Z-KT7R-0500-10").as_deref(), Some("1"));
        let radik = s
            .rows
            .iter()
            .find(|r| r.cells[0] == "RADIK VK 22-600×1000")
            .unwrap();
        assert_eq!(radik.cells[3], "1679");
    }
}
