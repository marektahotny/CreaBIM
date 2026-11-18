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
pub fn pipes(model: &Model) -> Schedule {
    let mut rows = vec![];
    let mut total = 0.0;
    for s in SYSTEMS {
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
        assert!(pipes(doc.model()).rows.is_empty());
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
                }),
            ));
        }
        doc.commit(tx).unwrap();
        let s = pipes(doc.model());
        assert_eq!(s.rows.len(), 2);
        // cold water first, its two pipes together
        assert!(s.rows[0].cells[0].starts_with("SV"));
        assert_eq!(s.rows[0].cells[3], "4,0");
        assert_eq!(s.rows[1].cells[3], "2,0");
        assert_eq!(s.totals[3], "6,0");
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
