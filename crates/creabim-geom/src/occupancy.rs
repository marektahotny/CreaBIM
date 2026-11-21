//! The persons a building is counted for in fire design (obsadenie
//! stavby osobami): each room by the item of the project's occupancy
//! table it is given, summed by fire zone and for the building. The
//! table's values are not the program's: they are read from the
//! designer's parameter file (`creabim_core::fireparams`).
//!
//! A room's item gives square metres a person, a factor on the designed
//! count (persons, beds, seats), or both, when the higher counts. Both
//! may change by steps of the area or the count, taken part by part. A
//! room's persons are rounded up to whole ones.

use crate::schedule::{Column, Row, Schedule};
use creabim_core::fireparams::{self, ParamTable, OCCUPANCY};
use creabim_core::words::tr;
use creabim_core::{ElementId, ElementKind, Model, ScheduleCategory};

/// The table's columns the count reads.
pub const AREA: &str = "area";
pub const FACTOR: &str = "factor";

/// A room's count.
#[derive(Clone, Debug, PartialEq)]
pub struct RoomPersons {
    pub room: ElementId,
    pub zone: String,
    /// The table's item and its name; empty where the room has none.
    pub item: String,
    pub name: String,
    pub area: f64,
    pub design_count: Option<f64>,
    /// The item's cells as the table writes them.
    pub per_person: String,
    pub factor: String,
    /// Whole persons; none where the item gives no value to count by.
    pub persons: Option<u32>,
}

/// The project's occupancy table.
pub fn table(model: &Model) -> Option<&ParamTable> {
    creabim_core::fireparams::find(&model.project_info()?.1.fire_params, OCCUPANCY)
}

/// The persons of an area and a designed count by an item's cells:
/// the higher of the area over the square metres a person and the
/// count times the factor; none when the cells give neither.
pub fn persons(per_person: &str, factor: &str, area: f64, count: Option<f64>) -> Option<u32> {
    let by_area = Some(fireparams::steps(per_person))
        .filter(|s| !s.is_empty())
        .map(|s| fireparams::divided(area, &s));
    let by_count = count.and_then(|n| {
        Some(fireparams::steps(factor))
            .filter(|s| !s.is_empty())
            .map(|s| fireparams::multiplied(n, &s))
    });
    let p = match (by_area, by_count) {
        (Some(a), Some(c)) => a.max(c),
        (a, c) => a.or(c)?,
    };
    Some((p - 1e-9).ceil().max(0.0) as u32)
}

/// Every room with an item or a designed count, by zone and number.
pub fn rooms(model: &Model) -> Vec<RoomPersons> {
    let table = table(model);
    let auto = crate::firerisk::AutoUses::new(model);
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Room(r) = &e.kind else {
            continue;
        };
        let item = auto.occupancy(r, &e.name);
        let item = item.as_str();
        if item.is_empty() && r.design_count.is_none() {
            continue;
        }
        let area = creabim_core::room::shape(model, e.id)
            .map_or(0.0, |s| creabim_core::room::net_area(model, r, &s));
        let cell = |column: &str| table.map_or("", |t| t.cell(item, column)).to_string();
        let (per_person, factor) = (cell(AREA), cell(FACTOR));
        out.push(RoomPersons {
            room: e.id,
            zone: r.fire_zone.trim().to_string(),
            item: item.to_string(),
            name: cell("name"),
            area,
            design_count: r.design_count,
            persons: persons(&per_person, &factor, area, r.design_count),
            per_person,
            factor,
        });
    }
    let number = |id: ElementId| match model.get(id).map(|e| &e.kind) {
        Some(ElementKind::Room(r)) => r.number.clone(),
        _ => String::new(),
    };
    out.sort_by(|a, b| (&a.zone, number(a.room)).cmp(&(&b.zone, number(b.room))));
    out
}

/// The persons of each fire zone ("" for rooms in none) and of the
/// building.
pub fn by_zone(rooms: &[RoomPersons]) -> (Vec<(String, u32)>, u32) {
    let mut zones: Vec<(String, u32)> = vec![];
    for r in rooms {
        let p = r.persons.unwrap_or(0);
        match zones.iter_mut().find(|z| z.0 == r.zone) {
            Some(z) => z.1 += p,
            None => zones.push((r.zone.clone(), p)),
        }
    }
    let total = zones.iter().map(|z| z.1).sum();
    (zones, total)
}

/// The norm the count is by, as the report states it: its name, its
/// edition, and whether values were changed by hand.
pub fn basis(model: &Model) -> String {
    match table(model) {
        Some(t) => {
            let mut s = format!("{} ({})", t.norm, t.edition);
            if !t.changed.is_empty() {
                s += &format!(", {}", tr("values changed by hand"));
            }
            s
        }
        None => tr("no occupancy table in the project"),
    }
}

/// The rooms' persons as a table: a heading and a sum for each zone,
/// the building's sum and the norm under them.
pub fn schedule(model: &Model) -> Schedule {
    let col = |title: &'static str, width: f64, numeric: bool| Column {
        title,
        field: None,
        width,
        numeric,
        align: None,
    };
    let columns = vec![
        col("Room", 45.0, false),
        col("Use by the norm", 65.0, false),
        col("Area", 18.0, true),
        col("m² a person", 22.0, true),
        col("Designed count", 20.0, true),
        col("Factor", 18.0, true),
        col("Persons", 18.0, true),
    ];
    let units = model.units();
    let list = rooms(model);
    let (zones, total) = by_zone(&list);
    let mut rows = vec![];
    for (zone, sum) in &zones {
        let first = list.iter().find(|r| &r.zone == zone).map(|r| r.room);
        let mut head = vec![String::new(); columns.len()];
        head[0] = if zone.is_empty() {
            tr("No fire zone")
        } else {
            format!("{} {zone}", tr("Fire zone"))
        };
        head[6] = sum.to_string();
        rows.push(Row {
            id: first.unwrap_or_default(),
            group: true,
            cells: head,
        });
        for r in list.iter().filter(|r| &r.zone == zone) {
            let label = match model.get(r.room) {
                Some(e) => match &e.kind {
                    ElementKind::Room(room) => format!("{} {}", room.number, tr(&e.name))
                        .trim()
                        .to_string(),
                    _ => String::new(),
                },
                None => String::new(),
            };
            let number = |v: f64| format!("{v}").replace('.', ",");
            rows.push(Row {
                id: r.room,
                group: false,
                cells: vec![
                    label,
                    format!("{} {}", r.item, r.name).trim().to_string(),
                    units.area(r.area),
                    r.per_person.clone(),
                    r.design_count.map(number).unwrap_or_default(),
                    r.factor.clone(),
                    r.persons.map_or_else(|| "?".to_string(), |p| p.to_string()),
                ],
            });
        }
    }
    let listed = list.len();
    let mut totals = vec![String::new(); columns.len()];
    totals[0] = tr("Total");
    totals[1] = basis(model);
    totals[6] = total.to_string();
    Schedule {
        category: ScheduleCategory::Occupancy,
        columns,
        rows,
        totals,
        listed,
        above_totals: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::starter::starter_house;

    const FILE: &str = "#table occupancy\n#norm N 1\n#edition 2012\n#columns item\tname\tarea\tfactor\tnote\nA\tDwelling\t20,0\t1,5\t\nB\tHall\t0-10 m2: 1,0; nad 10 m2: 2,0\t\t\nC\tSee elsewhere\t\t\t\n";

    #[test]
    fn the_higher_of_area_and_count_rounds_up() {
        // 45 m² at 20 m² a person is 2,25; four designed persons
        // times 1,5 are six.
        assert_eq!(persons("20,0", "1,5", 45.0, None), Some(3));
        assert_eq!(persons("20,0", "1,5", 45.0, Some(4.0)), Some(6));
        assert_eq!(persons("20,0", "1,5", 200.0, Some(4.0)), Some(10));
        // Stepped by area: 10 / 1 + 6 / 2.
        assert_eq!(
            persons("0-10 m2: 1,0; nad 10 m2: 2,0", "", 16.0, None),
            Some(13)
        );
        // A factor alone needs the count.
        assert_eq!(persons("", "1,3", 30.0, None), None);
        assert_eq!(persons("", "1,3", 30.0, Some(10.0)), Some(13));
        assert_eq!(persons("", "", 30.0, Some(10.0)), None);
    }

    #[test]
    fn rooms_count_by_their_items_and_sum_by_zone() {
        let mut m = starter_house();
        let mut info = m.project_info().map(|p| p.1.clone()).unwrap_or_default();
        info.fire_params = fireparams::parse(FILE);
        let tx = creabim_core::set_project(&m, info);
        tx.apply(&mut m).unwrap();
        let ids: Vec<ElementId> = m
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Room(_)))
            .map(|e| e.id)
            .take(3)
            .collect();
        for (i, id) in ids.iter().enumerate() {
            let before = m.get(*id).unwrap().clone();
            let mut after = before.clone();
            if let ElementKind::Room(r) = &mut after.kind {
                r.fire_zone = if i < 2 { "N1.01" } else { "N1.02" }.into();
                r.fire_use = ["A", "B", "C"][i].into();
                r.design_count = (i == 0).then_some(4.0);
            }
            let tx = creabim_core::Transaction::new("room").replace(before, after);
            tx.apply(&mut m).unwrap();
        }
        let list = rooms(&m);
        assert_eq!(list.len(), 3);
        let a = list.iter().find(|r| r.item == "A").unwrap();
        assert_eq!(
            (a.name.as_str(), a.persons.unwrap() >= 6),
            ("Dwelling", true)
        );
        // An item that gives no value leaves the room uncounted.
        assert_eq!(list.iter().find(|r| r.item == "C").unwrap().persons, None);
        let (zones, total) = by_zone(&list);
        assert_eq!(zones.len(), 2);
        assert_eq!(total, zones.iter().map(|z| z.1).sum::<u32>());
        let s = schedule(&m);
        assert_eq!(s.rows.iter().filter(|r| r.group).count(), 2);
        assert_eq!(s.totals[6], total.to_string());
        assert!(s.totals[1].starts_with("N 1 (2012)"));
    }
}
