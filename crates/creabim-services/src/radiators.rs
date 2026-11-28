//! Radiators sized to each room's heat loss (STN EN 12831-1) with the
//! characteristic of STN EN 442: Φ = Φn · (Δθ / 50)ⁿ, Δθ the mean
//! temperature difference between the water and the room (arithmetic,
//! or logarithmic where the water cools a lot). A room gets a radiator
//! under each of its windows, the slimmest type that covers its share
//! within the longest length allowed; what underfloor heating already
//! gives the room is taken off first. Radiators already standing in a
//! room are counted against its loss: a towel radiator on the floor's
//! water is taken off what is needed first, and [`resize`] makes each
//! panel radiator standing in a room, unless it is locked, the size the
//! room needs.

use crate::floorheat;
use crate::heatloss::{self, RoomLoss};
use creabim_core::catalogue::radiators::{lengths, Heater, Panel, PanelKind};
use creabim_core::{ElementId, ElementKind, Model, Transaction};

/// What the system is designed with.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    pub supply_c: f64,
    pub return_c: f64,
    /// Height of the radiators, mm.
    pub height: u32,
    /// The longest radiator to propose, mm.
    pub max_len: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            supply_c: 55.0,
            return_c: 45.0,
            height: 600,
            max_len: 1600,
        }
    }
}

/// The most radiators a room is given.
pub const MAX_COUNT: u32 = 6;

/// Mean temperature difference between the water and a room at `ti`, K
/// (STN EN 442-2): arithmetic, logarithmic where the return's excess is
/// under 0.7 of the supply's.
pub fn delta_t(supply: f64, ret: f64, ti: f64) -> f64 {
    let (a, b) = (supply - ti, ret - ti);
    if a <= 0.0 || b <= 0.0 {
        0.0
    } else if b / a >= 0.7 || (a - b).abs() < 1e-9 {
        (a + b) / 2.0
    } else {
        (a - b) / (a / b).ln()
    }
}

/// The slimmest type, and of it the shortest radiator, that gives
/// `need` W at `dt`, no longer than the settings allow.
pub fn pick(need: f64, dt: f64, s: &Settings) -> Option<Panel> {
    PanelKind::ALL.into_iter().find_map(|kind| {
        lengths()
            .into_iter()
            .filter(|l| *l <= s.max_len)
            .map(|length| Panel {
                kind,
                height: s.height,
                length,
            })
            .find(|p| p.output_w(dt) >= need)
    })
}

/// One room's radiators.
#[derive(Clone, Debug, PartialEq)]
pub struct RoomDesign {
    pub id: ElementId,
    pub number: String,
    pub name: String,
    pub ti: f64,
    /// Design heat loss, W.
    pub phi: f64,
    /// What the room's underfloor heating gives, W.
    pub floor_w: f64,
    /// What towel radiators standing in the room give, W.
    pub towels_w: f64,
    /// Left for radiators, W.
    pub need: f64,
    /// Windows in the room's outside walls.
    pub windows: u32,
    /// The proposal: how many of which (none where the floor covers
    /// the loss).
    pub count: u32,
    pub panel: Option<Panel>,
    /// The proposal's output at the design temperatures, W.
    pub output: f64,
    /// Its flow, kg/h.
    pub flow: f64,
    /// Radiators standing in the room, and their output, W.
    pub installed: Vec<Heater>,
    pub installed_w: f64,
    /// The proposal covers what is needed.
    pub ok: bool,
}

/// Every room's radiators.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Design {
    pub rooms: Vec<RoomDesign>,
    /// Of the proposals, kW and kg/h.
    pub output_kw: f64,
    pub flow: f64,
    pub count: u32,
    pub installed_kw: f64,
}

/// A heater standing in the model.
#[derive(Clone, Debug)]
pub struct Standing {
    pub id: ElementId,
    pub level: ElementId,
    /// A point in front of the wall it hangs on.
    pub at: glam::DVec2,
    pub heater: Heater,
    pub locked: bool,
}

/// The heaters standing in the model.
pub fn standing(model: &Model) -> Vec<Standing> {
    model
        .iter()
        .filter_map(|e| {
            let ElementKind::Object(ob) = &e.kind else {
                return None;
            };
            let t = creabim_core::objects::type_of(model, e)?;
            let heater = Heater::of_key(t.catalogue.as_deref()?)?;
            Some(Standing {
                id: e.id,
                level: ob.level,
                at: ob.to_plan(glam::DVec2::new(0.0, -0.15)),
                heater,
                locked: e.locked,
            })
        })
        .collect()
}

/// The heaters standing in room `r`.
fn in_room<'a>(model: &Model, r: &creabim_core::Room, all: &'a [Standing]) -> Vec<&'a Standing> {
    let outline = creabim_core::room::shape_of(model, r).map(|x| x.outline);
    all.iter()
        .filter(|x| {
            x.level == r.level
                && outline
                    .as_ref()
                    .is_some_and(|o| creabim_core::polygon::contains(o, x.at))
        })
        .collect()
}

fn one(
    model: &Model,
    s: &Settings,
    hl: &RoomLoss,
    floor: Option<&floorheat::RoomDesign>,
    standing: &[Standing],
) -> Option<RoomDesign> {
    let e = model.get(hl.id)?;
    let ElementKind::Room(r) = &e.kind else {
        return None;
    };
    let dt = delta_t(s.supply_c, s.return_c, hl.ti);
    let floor_w = floor.filter(|f| f.on).map_or(0.0, |f| f.covered);
    let here = in_room(model, r, standing);
    let towels_w: f64 = here
        .iter()
        .filter(|x| matches!(x.heater, Heater::Towel(_)))
        .map(|x| x.heater.output_w(dt))
        .sum();
    let need = (hl.phi - floor_w - towels_w).max(0.0);
    let windows = hl.items.iter().filter(|i| i.kind == "Window").count() as u32;
    let (mut count, mut panel) = (0, None);
    if need > hl.phi * 0.02 && need > 1.0 {
        count = windows.max(1);
        loop {
            panel = pick(need / count as f64, dt, s);
            if panel.is_some() || count >= MAX_COUNT {
                break;
            }
            count += 1;
        }
        // none covers it: the largest there is
        panel = panel.or(Some(Panel {
            kind: PanelKind::K33,
            height: s.height,
            length: lengths()
                .into_iter()
                .filter(|l| *l <= s.max_len)
                .rfind(|l| {
                    Panel {
                        kind: PanelKind::K33,
                        height: s.height,
                        length: *l,
                    }
                    .made()
                })
                .unwrap_or(400),
        }));
    }
    let output = panel.map_or(0.0, |p| p.output_w(dt) * count as f64);
    let drop = (s.supply_c - s.return_c).max(1.0);
    let installed: Vec<Heater> = here
        .iter()
        .filter(|x| matches!(x.heater, Heater::Panel(_)))
        .map(|x| x.heater)
        .collect();
    let installed_w = installed.iter().map(|p| p.output_w(dt)).sum();
    Some(RoomDesign {
        id: hl.id,
        number: hl.number.clone(),
        name: hl.name.clone(),
        ti: hl.ti,
        phi: hl.phi,
        floor_w,
        towels_w,
        need,
        windows,
        count,
        panel,
        output,
        flow: output / (4186.0 * drop) * 3600.0,
        installed,
        installed_w,
        ok: output >= need * 0.98,
    })
}

/// Radiators for every room of the model, from its heat loss and what
/// its underfloor heating gives.
pub fn design(
    model: &Model,
    o: &heatloss::Opts,
    s: &Settings,
    floor: &floorheat::Settings,
) -> Design {
    let losses = heatloss::building(model, o);
    let floors = floorheat::design(model, o, floor);
    let standing = standing(model);
    let mut out = Design {
        rooms: losses
            .rooms
            .iter()
            .filter_map(|hl| {
                let f = floors.rooms.iter().find(|f| f.id == hl.id);
                one(model, s, hl, f, &standing)
            })
            .collect(),
        ..Design::default()
    };
    for x in &out.rooms {
        out.output_kw += x.output / 1000.0;
        out.flow += x.flow;
        out.count += x.count;
        out.installed_kw += x.installed_w / 1000.0;
    }
    out
}

/// Each panel radiator standing in a room the size the room needs at
/// the design temperatures: what the floor and the towel radiators do
/// not give, shared among those not locked; a locked one keeps its size
/// and its output is counted. The slimmest type that covers its share
/// within the longest length allowed, at the settings' height.
pub fn resize(model: &Model, d: &Design, s: &Settings) -> Transaction {
    let mut tx = Transaction::new("Radiators sized to the rooms");
    let all = standing(model);
    let mut work = model.clone();
    for x in &d.rooms {
        let Some(e) = model.get(x.id) else { continue };
        let ElementKind::Room(r) = &e.kind else {
            continue;
        };
        let dt = delta_t(s.supply_c, s.return_c, x.ti);
        let here = in_room(model, r, &all);
        let panels: Vec<&&Standing> = here
            .iter()
            .filter(|h| matches!(h.heater, Heater::Panel(_)))
            .collect();
        let locked_w: f64 = panels
            .iter()
            .filter(|h| h.locked)
            .map(|h| h.heater.output_w(dt))
            .sum();
        let free: Vec<&&&Standing> = panels.iter().filter(|h| !h.locked).collect();
        if free.is_empty() || x.need <= 1.0 {
            continue;
        }
        let share = (x.need - locked_w).max(0.0) / free.len() as f64;
        let Some(p) = pick(share, dt, s).or(x.panel) else {
            continue;
        };
        let c = p.object();
        for h in free {
            if h.heater == Heater::Panel(p) {
                continue;
            }
            let (add, type_id) = creabim_core::objects::add_to_model(&work, &c);
            for op in &add.ops {
                tx.ops.push(op.clone());
            }
            let _ = add.apply(&mut work);
            let Some(before) = work.get(h.id).cloned() else {
                continue;
            };
            let mut after = before.clone();
            if let Some(t) = after.of_type.as_mut() {
                t.id = type_id;
            }
            after.name = c.name.clone();
            let step = Transaction::new("").replace(before, after);
            tx.ops.extend(step.ops.iter().cloned());
            let _ = step.apply(&mut work);
        }
    }
    tx
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mean_temperature_difference() {
        assert_eq!(delta_t(75.0, 65.0, 20.0), 50.0);
        assert_eq!(delta_t(55.0, 45.0, 20.0), 30.0);
        // 70/40/20: the water cools a lot, the logarithmic mean
        let d = delta_t(70.0, 40.0, 20.0);
        assert!((d - 32.74).abs() < 0.02, "{d}");
        assert_eq!(delta_t(40.0, 20.0, 24.0), 0.0);
    }

    #[test]
    fn the_slimmest_type_that_covers_the_need() {
        let s = Settings::default();
        let p = pick(500.0, 30.0, &s).unwrap();
        assert_eq!(p.kind, PanelKind::K11);
        assert!(p.output_w(30.0) >= 500.0 && p.length <= s.max_len);
        // a shorter one of the type would not do
        let shorter = Panel {
            length: p.length - 100,
            ..p
        };
        assert!(shorter.output_w(30.0) < 500.0);
        // more than a type 11 of 1.6 m gives: the next type
        assert!(pick(1200.0, 30.0, &s).unwrap().kind > PanelKind::K11);
        assert_eq!(pick(9000.0, 30.0, &s), None);
    }

    #[test]
    fn rooms_get_radiators_for_what_the_floor_leaves() {
        use creabim_core::underfloor::Underfloor;
        use creabim_core::{Document, Element, Room, Transaction};
        use glam::DVec2;
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let mut tx = Transaction::new("rooms");
        for (name, number, at, on) in [
            ("Kúpeľňa", "1.01", DVec2::new(2.5, 3.0), true),
            ("Obývacia izba", "1.02", DVec2::new(8.0, 3.0), false),
        ] {
            let mut r = Room::new(level, at);
            r.number = number.into();
            if on {
                r.ufh = Some(Underfloor {
                    spacing: 0.1,
                    loops: 3,
                    finish_r: None,
                    ..Underfloor::default()
                });
            }
            tx = tx.insert(Element::new(name, ElementKind::Room(r)));
        }
        doc.commit(tx).unwrap();
        let s = Settings::default();
        let d = design(
            doc.model(),
            &heatloss::Opts::default(),
            &s,
            &floorheat::Settings::default(),
        );
        assert_eq!(d.rooms.len(), 2);
        let (bath, living) = (&d.rooms[0], &d.rooms[1]);
        assert!(bath.floor_w > 0.0 && living.floor_w == 0.0);
        assert!((bath.need - (bath.phi - bath.floor_w).max(0.0)).abs() < 1e-9);
        assert_eq!(living.need, living.phi);
        assert!(living.windows >= 1 && living.count >= living.windows);
        for x in &d.rooms {
            if x.need > 1.0 {
                let p = x.panel.unwrap();
                assert!(p.length <= s.max_len && p.height == s.height);
                let dt = delta_t(s.supply_c, s.return_c, x.ti);
                assert!((x.output - p.output_w(dt) * x.count as f64).abs() < 1e-9);
                assert!(x.flow > 0.0);
            }
        }
        assert!((d.output_kw - (bath.output + living.output) / 1000.0).abs() < 1e-9);
        assert_eq!(d.count, bath.count + living.count);
    }

    #[test]
    fn standing_radiators_take_the_size_the_room_needs_unless_locked() {
        use creabim_core::catalogue::towels::TowelRail;
        use creabim_core::{Document, Element, Room, Transaction};
        use glam::DVec2;
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let mut r = Room::new(level, DVec2::new(8.0, 3.0));
        r.number = "1.02".into();
        doc.commit(Transaction::new("room").insert(Element::new("Izba", ElementKind::Room(r))))
            .unwrap();
        let small = Panel {
            kind: PanelKind::K10,
            height: 600,
            length: 400,
        };
        let mut ids = vec![];
        for x in [7.0, 9.0] {
            let (tx, id) = creabim_core::objects::place(
                doc.model(),
                &small.object(),
                level,
                DVec2::new(x, 2.0),
                0.0,
            );
            doc.commit(tx).unwrap();
            ids.push(id);
        }
        let towel = TowelRail {
            height: 700,
            length: 450,
        };
        let (tx, _) = creabim_core::objects::place(
            doc.model(),
            &towel.object(),
            level,
            DVec2::new(8.0, 2.0),
            0.0,
        );
        doc.commit(tx).unwrap();
        // the second keeps its size
        let mut locked = doc.model().get(ids[1]).unwrap().clone();
        let before = locked.clone();
        locked.locked = true;
        doc.commit(Transaction::new("lock").replace(before, locked))
            .unwrap();
        let s = Settings::default();
        let o = heatloss::Opts::default();
        let fl = floorheat::Settings::default();
        let d = design(doc.model(), &o, &s, &fl);
        let x = d.rooms.iter().find(|x| x.number == "1.02").unwrap();
        let dt = delta_t(s.supply_c, s.return_c, x.ti);
        assert!((x.towels_w - towel.output_w(dt)).abs() < 1e-9);
        assert!(x.need > small.output_w(dt) * 2.0, "{}", x.need);
        assert!((x.need - (x.phi - x.towels_w).max(0.0)).abs() < 1e-9);
        let tx = resize(doc.model(), &d, &s);
        doc.commit(tx).unwrap();
        let after = standing(doc.model());
        let of = |id| after.iter().find(|h| h.id == id).unwrap().heater;
        assert_eq!(of(ids[1]), Heater::Panel(small));
        let Heater::Panel(p) = of(ids[0]) else {
            panic!()
        };
        assert!(p.output_w(dt) + small.output_w(dt) >= x.need - 1e-6);
        assert!(p.length <= s.max_len && p.height == s.height);
        // a slimmer type or a shorter one would not do
        let shorter = Panel {
            length: p.length - 100,
            ..p
        };
        assert!(shorter.output_w(dt) + small.output_w(dt) < x.need || p.length == 400);
    }
}
