//! Radiators sized to each room's heat loss (STN EN 12831-1) with the
//! characteristic of STN EN 442: Φ = Φn · (Δθ / 50)ⁿ, Δθ the mean
//! temperature difference between the water and the room (arithmetic,
//! or logarithmic where the water cools a lot). A room gets a radiator
//! under each of its windows, the slimmest type that covers its share
//! within the longest length allowed; what underfloor heating already
//! gives the room is taken off first. Radiators already standing in a
//! room are counted against its loss.

use crate::floorheat;
use crate::heatloss::{self, RoomLoss};
use creabim_core::catalogue::radiators::{lengths, Panel, PanelKind};
use creabim_core::{ElementId, ElementKind, Model};

/// What the system is designed with.
#[derive(Clone, Copy, Debug, PartialEq)]
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
    pub installed: Vec<Panel>,
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

/// The panel radiators standing in the model: level and place in plan.
fn standing(model: &Model) -> Vec<(ElementId, glam::DVec2, Panel)> {
    model
        .iter()
        .filter_map(|e| {
            let ElementKind::Object(ob) = &e.kind else {
                return None;
            };
            let t = creabim_core::objects::type_of(model, e)?;
            let p = Panel::of_key(t.catalogue.as_deref()?)?;
            // a point in front of the wall it hangs on
            Some((ob.level, ob.to_plan(glam::DVec2::new(0.0, -0.15)), p))
        })
        .collect()
}

fn one(
    model: &Model,
    s: &Settings,
    hl: &RoomLoss,
    floor: Option<&floorheat::RoomDesign>,
    standing: &[(ElementId, glam::DVec2, Panel)],
) -> Option<RoomDesign> {
    let e = model.get(hl.id)?;
    let ElementKind::Room(r) = &e.kind else {
        return None;
    };
    let dt = delta_t(s.supply_c, s.return_c, hl.ti);
    let floor_w = floor.filter(|f| f.on).map_or(0.0, |f| f.covered);
    let need = (hl.phi - floor_w).max(0.0);
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
                .rfind(|l| *l <= s.max_len)
                .unwrap_or(400),
        }));
    }
    let output = panel.map_or(0.0, |p| p.output_w(dt) * count as f64);
    let drop = (s.supply_c - s.return_c).max(1.0);
    let outline = creabim_core::room::shape_of(model, r).map(|x| x.outline);
    let installed: Vec<Panel> = standing
        .iter()
        .filter(|(level, at, _)| {
            *level == r.level
                && outline
                    .as_ref()
                    .is_some_and(|o| creabim_core::polygon::contains(o, *at))
        })
        .map(|x| x.2)
        .collect();
    let installed_w = installed.iter().map(|p| p.output_w(dt)).sum();
    Some(RoomDesign {
        id: hl.id,
        number: hl.number.clone(),
        name: hl.name.clone(),
        ti: hl.ti,
        phi: hl.phi,
        floor_w,
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
}
