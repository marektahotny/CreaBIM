//! Ventilation, as PilonDesigner's `vzt.ts`: the air each room needs
//! (its air change × volume or the least flow of its kind, STN EN
//! 16798-1 and STN EN 15665, as the heat loss takes it), supply into the
//! living rooms and exhaust from bathrooms, WCs and kitchens, and how
//! the air terminals standing in a room share it.

use crate::heatloss::{self, RoomLoss};
use creabim_core::mep::SystemKind;
use creabim_core::{ElementId, ElementKind, Model};
use glam::DVec2;

/// A room air is taken from (a bathroom, a WC, a kitchen, a store, a
/// plant room): the others get supply air.
pub fn is_exhaust(text: &str) -> bool {
    creabim_core::roomtype::RoomType::find(text).is_some_and(|t| t.needs().exhaust)
}

/// An air terminal standing in the model.
#[derive(Clone, Debug, PartialEq)]
pub struct Terminal {
    pub id: ElementId,
    pub level: ElementId,
    pub at: DVec2,
    pub supply: bool,
    /// Its own flow, m³/h, where it was given one.
    pub given: Option<f64>,
}

/// The model's air terminals: diffusers and grilles on the ends of
/// ducts, and those placed from the catalogue (IfcAirTerminal: a
/// diffuser supplies, a grille takes air out).
pub fn terminals(model: &Model) -> Vec<Terminal> {
    let mut out = vec![];
    for e in model.iter() {
        match &e.kind {
            ElementKind::Valve(v) if v.is_air_terminal() => {
                let Some(ElementKind::Pipe(p)) = model.get(v.pipe).map(|x| &x.kind) else {
                    continue;
                };
                let end = if v.t < 0.5 { p.start } else { p.end };
                // by its duct's system, else by its kind
                let supply = match creabim_core::mep::system(&p.system).map(|s| s.kind) {
                    Some(SystemKind::AirSupply) => true,
                    Some(SystemKind::AirExhaust) => false,
                    _ => v.kind == "vyustP",
                };
                out.push(Terminal {
                    id: e.id,
                    level: p.level,
                    at: end.truncate(),
                    supply,
                    given: v.flow_m3h.filter(|q| *q > 0.0),
                });
            }
            ElementKind::Object(ob) => {
                let Some(t) = creabim_core::objects::type_of(model, e) else {
                    continue;
                };
                if t.ifc.class != "IfcAirTerminal" {
                    continue;
                }
                out.push(Terminal {
                    id: e.id,
                    level: ob.level,
                    at: ob.to_plan(DVec2::ZERO),
                    supply: t.ifc.predefined != "GRILLE",
                    given: None,
                });
            }
            _ => {}
        }
    }
    out
}

/// One room's air.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub id: ElementId,
    pub number: String,
    pub name: String,
    pub volume: f64,
    /// Air is taken out of it; else it gets supply air.
    pub exhaust_room: bool,
    /// What it needs, m³/h.
    pub need: f64,
    /// What its terminals bring and take, m³/h, and how many they are.
    pub supply: f64,
    pub exhaust: f64,
    pub terminals: u32,
    /// Its need is met by terminals of its kind.
    pub ok: bool,
}

/// The building's air balance.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Balance {
    pub rooms: Vec<Row>,
    /// Needed by the rooms supplied and by those exhausted, m³/h.
    pub need_supply: f64,
    pub need_exhaust: f64,
    /// Through the terminals, m³/h.
    pub supply: f64,
    pub exhaust: f64,
    /// Terminals that stand in no room.
    pub stray: u32,
}

impl Balance {
    /// What the air handling unit is sized for: the larger of the two
    /// needs, so supply and exhaust balance.
    pub fn unit_flow(&self) -> f64 {
        self.need_supply.max(self.need_exhaust)
    }
}

fn row(model: &Model, hl: &RoomLoss, terms: &[Terminal], used: &mut [bool]) -> Option<Row> {
    let e = model.get(hl.id)?;
    let ElementKind::Room(r) = &e.kind else {
        return None;
    };
    let outline = creabim_core::room::shape_of(model, r)?.outline;
    let exhaust_room = creabim_core::roomtype::RoomType::of(&r.occupancy, &e.name)
        .needs()
        .exhaust;
    let need = hl.v_min.round();
    let mine: Vec<&Terminal> = terms
        .iter()
        .enumerate()
        .filter(|(_, t)| t.level == r.level && creabim_core::polygon::contains(&outline, t.at))
        .map(|(i, t)| {
            used[i] = true;
            t
        })
        .collect();
    // each side: the given flows, the rest of the need shared evenly
    // among the terminals that have none
    let side = |supply: bool| -> f64 {
        let of: Vec<&&Terminal> = mine.iter().filter(|t| t.supply == supply).collect();
        let given: f64 = of.iter().filter_map(|t| t.given).sum();
        let free = of.iter().filter(|t| t.given.is_none()).count();
        if free > 0 {
            given + (need - given).max(0.0)
        } else {
            given
        }
    };
    let (supply, exhaust) = (side(true), side(false));
    let have = if exhaust_room { exhaust } else { supply };
    Some(Row {
        id: hl.id,
        number: hl.number.clone(),
        name: hl.name.clone(),
        volume: hl.volume,
        exhaust_room,
        need,
        supply,
        exhaust,
        terminals: mine.len() as u32,
        ok: have >= need - 0.5,
    })
}

/// The flow of one terminal, m³/h: its own, else its share of its
/// room's need (0 where it stands in no room).
pub fn terminal_flow(model: &Model, o: &heatloss::Opts, id: ElementId) -> f64 {
    let terms = terminals(model);
    let Some(t) = terms.iter().find(|t| t.id == id) else {
        return 0.0;
    };
    if let Some(q) = t.given {
        return q;
    }
    for hl in &heatloss::building(model, o).rooms {
        let Some(ElementKind::Room(r)) = model.get(hl.id).map(|e| &e.kind) else {
            continue;
        };
        let Some(shape) = creabim_core::room::shape_of(model, r) else {
            continue;
        };
        if r.level != t.level || !creabim_core::polygon::contains(&shape.outline, t.at) {
            continue;
        }
        let same = terms.iter().filter(|x| {
            x.supply == t.supply
                && x.level == t.level
                && creabim_core::polygon::contains(&shape.outline, x.at)
        });
        let (mut given, mut free) = (0.0, 0);
        for x in same {
            match x.given {
                Some(q) => given += q,
                None => free += 1,
            }
        }
        return ((hl.v_min.round() - given).max(0.0) / free.max(1) as f64).round();
    }
    0.0
}

/// Every room's air and the building's balance.
pub fn balance(model: &Model, o: &heatloss::Opts) -> Balance {
    let terms = terminals(model);
    let mut used = vec![false; terms.len()];
    let losses = heatloss::building(model, o);
    let mut out = Balance {
        rooms: losses
            .rooms
            .iter()
            .filter_map(|hl| row(model, hl, &terms, &mut used))
            .collect(),
        ..Balance::default()
    };
    for r in &out.rooms {
        if r.exhaust_room {
            out.need_exhaust += r.need;
        } else {
            out.need_supply += r.need;
        }
        out.supply += r.supply;
        out.exhaust += r.exhaust;
    }
    out.stray = used.iter().filter(|u| !**u).count() as u32;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::mep::{Pipe, Valve};
    use creabim_core::{Document, Element, Room, Transaction};
    use glam::DVec3;

    #[test]
    fn rooms_air_is_taken_from() {
        for x in ["WC", "Kúpeľňa", "Kuchyňa", "Technická miestnosť", "Garáž"] {
            assert!(is_exhaust(x), "{x}");
        }
        for x in ["Obývacia izba", "Spálňa", "Bedroom", "Kancelária"] {
            assert!(!is_exhaust(x), "{x}");
        }
    }

    fn duct(level: ElementId, system: &str, a: DVec3, b: DVec3) -> Pipe {
        Pipe {
            level,
            system: system.into(),
            start: a,
            end: b,
            pipe_type: "VZK".into(),
            nominal_mm: 160.0,
            outer_mm: 160.0,
            inner_mm: 159.0,
            height_mm: None,
            insulation_mm: 0.0,
            insulation_set: false,
            mark: None,
        }
    }

    #[test]
    fn terminals_share_their_rooms_need() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let mut tx = Transaction::new("rooms");
        for (name, at) in [
            ("Kúpeľňa", DVec2::new(2.5, 3.0)),
            ("Obývacia izba", DVec2::new(8.0, 3.0)),
        ] {
            tx = tx.insert(Element::new(name, ElementKind::Room(Room::new(level, at))));
        }
        doc.commit(tx).unwrap();
        let o = heatloss::Opts::default();
        let b = balance(doc.model(), &o);
        assert_eq!(b.rooms.len(), 2);
        let (bath, living) = (b.rooms[0].clone(), b.rooms[1].clone());
        assert!(bath.exhaust_room && !living.exhaust_room);
        assert!(bath.need > 0.0 && living.need > 0.0);
        assert!(!bath.ok && !living.ok && bath.terminals == 0);
        assert_eq!(b.need_supply, living.need);
        assert_eq!(b.need_exhaust, bath.need);
        assert_eq!(b.unit_flow(), bath.need.max(living.need));

        // two supply diffusers in the living room, one with its own flow
        let mut ids = vec![];
        for (x, given) in [(7.5, None), (9.0, Some(20.0))] {
            let p = duct(
                level,
                "VZP",
                DVec3::new(x, 1.0, 2.7),
                DVec3::new(x, 3.0, 2.7),
            );
            let pe = Element::new("Duct", ElementKind::Pipe(p));
            let pid = pe.id;
            doc.commit(Transaction::new("duct").insert(pe)).unwrap();
            let mut v = Valve::new(pid, 1.0, "vyustP");
            v.flow_m3h = given;
            let ve = Element::new("Diffuser", ElementKind::Valve(v));
            ids.push(ve.id);
            doc.commit(Transaction::new("terminal").insert(ve)).unwrap();
        }
        let b = balance(doc.model(), &o);
        let living = &b.rooms[1];
        assert_eq!(living.terminals, 2);
        assert_eq!(living.supply, living.need.max(20.0));
        assert!(living.ok && living.exhaust == 0.0);
        assert_eq!(b.stray, 0);
        assert_eq!(terminal_flow(doc.model(), &o, ids[1]), 20.0);
        assert_eq!(
            terminal_flow(doc.model(), &o, ids[0]),
            (living.need - 20.0).max(0.0).round()
        );
    }
}
