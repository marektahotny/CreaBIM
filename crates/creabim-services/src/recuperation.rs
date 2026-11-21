//! Ventilation with heat recovery for a family house, designed in one
//! step and then edited by hand: a central unit (ATREA DUPLEX) with a
//! radial system of flexible ducts Ø75 from two distribution boxes to a
//! plate valve in every room, and room units in the outer wall
//! (decentral recuperation) for the rooms marked so.
//!
//! The air each room needs is the heat loss's (`v_min`, from the room
//! types of [`creabim_core::roomtype`]); supply and extract are balanced
//! by raising the smaller side. Every duct is routed from its room
//! through the doors to the unit's room, so no duct is led through a
//! wall, a lintel or a beam (the routes check still looks). Pressure
//! loss is the ducts' friction (Darcy–Weisbach, Swamee–Jain) and their
//! bends; the plate valves', boxes' and the unit's own figures come from
//! the catalogue where it gives them.
//!
//! The bill of materials ([`bill`]) is read from the model, so whatever
//! is moved, added or deleted by hand is priced as it is drawn.

use crate::heatloss;
use creabim_core::catalogue::services as cat_objects;
use creabim_core::mep::{Pipe, Valve};
use creabim_core::objects::CatalogueObject;
use creabim_core::roomtype::RoomType;
use creabim_core::{Element, ElementId, ElementKind, Model, OpeningKind, Transaction};
use glam::{DVec2, DVec3};
use serde::Deserialize;
use std::sync::OnceLock;

// ---------------------------------------------------------------- catalogue

/// A central unit.
#[derive(Clone, Debug, Deserialize)]
pub struct Unit {
    #[serde(rename = "kod")]
    pub code: String,
    #[serde(rename = "nazov")]
    pub name: String,
    /// "ECV5" stands at a wall, "EC5" hangs under the ceiling.
    #[serde(rename = "rada")]
    pub series: String,
    /// Its largest flow, m³/h.
    #[serde(rename = "qMax")]
    pub q_max: f64,
    #[serde(rename = "cena")]
    pub price: Option<f64>,
    /// Width, depth, height, m (as the catalogue gives them).
    #[serde(rename = "rozmer")]
    pub size: [f64; 3],
    #[serde(rename = "zdroj", default)]
    pub source: String,
}

/// A room unit in the outer wall.
#[derive(Clone, Debug, Deserialize)]
pub struct RoomUnit {
    #[serde(rename = "kod")]
    pub code: String,
    #[serde(rename = "nazov")]
    pub name: String,
    /// Its flow airing straight through, and with heat recovery
    /// (reversing), m³/h.
    #[serde(rename = "qVetranie")]
    pub q_vent: f64,
    #[serde(rename = "qRekuperacia")]
    pub q_hr: f64,
    /// The walls its sleeve fits, m.
    #[serde(rename = "stenaMin")]
    pub wall_min: f64,
    #[serde(rename = "stenaMax")]
    pub wall_max: f64,
    /// The core hole, m.
    #[serde(rename = "otvor")]
    pub hole: f64,
    #[serde(rename = "cena")]
    pub price: Option<f64>,
    #[serde(rename = "zdroj", default)]
    pub source: String,
    #[serde(rename = "pozn", default)]
    pub note: String,
}

/// Any other item of the bill.
#[derive(Clone, Debug, Deserialize)]
pub struct Item {
    #[serde(rename = "kluc")]
    pub key: String,
    #[serde(rename = "kod")]
    pub code: String,
    #[serde(rename = "nazov")]
    pub name: String,
    #[serde(rename = "mj")]
    pub unit: String,
    #[serde(rename = "cena")]
    pub price: Option<f64>,
    #[serde(rename = "zdroj", default)]
    pub source: String,
    /// A distribution box's outlets.
    #[serde(rename = "vystupy", default)]
    pub outlets: u32,
}

/// The catalogue and its prices (without VAT).
#[derive(Clone, Debug, Deserialize)]
pub struct Catalogue {
    #[serde(rename = "dph")]
    pub vat: f64,
    #[serde(rename = "poznamka")]
    pub note: String,
    #[serde(rename = "jednotky")]
    pub units: Vec<Unit>,
    #[serde(rename = "decentralne")]
    pub room_units: Vec<RoomUnit>,
    #[serde(rename = "polozky")]
    pub items: Vec<Item>,
}

impl Catalogue {
    pub fn item(&self, key: &str) -> Option<&Item> {
        self.items.iter().find(|i| i.key == key)
    }
}

/// The built-in catalogue (data/vzt-rekuperacia.json).
pub fn catalogue() -> &'static Catalogue {
    static C: OnceLock<Catalogue> = OnceLock::new();
    C.get_or_init(|| {
        serde_json::from_str(include_str!("../data/vzt-rekuperacia.json"))
            .expect("data/vzt-rekuperacia.json is the recuperation catalogue")
    })
}

/// The catalogue keys of the objects placed.
pub const UNIT_KEY: &str = "vzt/duplex/";
pub const BOX_KEY: &str = "vzt/box/";
pub const ROOM_UNIT_KEY: &str = "vzt/dec/";
/// The names the design gives what it places, so designing again
/// replaces them and nothing drawn by hand.
pub const TAG: &str = "Rekuperácia";

// ---------------------------------------------------------------- options

/// Where the flexible ducts are laid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// In the floor's insulation, under the screed.
    Floor,
    /// Above the ceiling (attic insulation, a suspended ceiling, the
    /// slab).
    Ceiling,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Opts {
    pub route: Route,
    /// The unit's series: "ECV5" (at a wall) or "EC5" (under the ceiling).
    pub series: String,
    /// The most air one duct Ø75 carries, m³/h (GP: 15–50 recommended
    /// by ATREA; quieter the less).
    pub per_duct: f64,
    /// The share of the unit's largest flow it is designed to run at.
    pub use_of_unit: f64,
    /// Air velocity through a door's undercut, m/s (the transfer).
    pub transfer_v: f64,
    /// The pressure lost by a bend of a flexible duct, ζ.
    pub zeta_bend: f64,
    /// The room unit's model and how high it sits, m.
    pub room_unit: usize,
    pub room_unit_at: f64,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            route: Route::Floor,
            series: "ECV5".into(),
            per_duct: 25.0,
            use_of_unit: 0.7,
            transfer_v: 1.0,
            zeta_bend: 0.5,
            room_unit: 0,
            room_unit_at: 2.0,
        }
    }
}

// ---------------------------------------------------------------- design

/// What a room gets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Supply,
    Extract,
    /// Air passes through it (a hall): no terminal.
    Transfer,
    /// Its own units in the outer wall.
    Decentral,
    /// Left out (a garage: never on the house's air).
    Out,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RoomAir {
    pub id: ElementId,
    pub number: String,
    pub name: String,
    pub role: Role,
    /// What it needs, and what it gets after the balance, m³/h.
    pub need: f64,
    pub design: f64,
    pub ducts: u32,
    pub terminals: Vec<Terminal>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Terminal {
    pub at: DVec2,
    pub flow: f64,
    pub ducts: u32,
}

/// One flexible duct from a box to a terminal.
#[derive(Clone, Debug, PartialEq)]
pub struct Branch {
    pub room: usize,
    pub terminal: usize,
    pub supply: bool,
    pub path: Vec<DVec2>,
    pub length: f64,
    pub flow: f64,
    pub velocity: f64,
    pub bends: u32,
    /// Friction and bends, and with the terminal and box, Pa.
    pub dp: f64,
    /// What its throttle ring takes up so it matches the worst, Pa.
    pub throttle: f64,
}

/// A door the extract air passes under.
#[derive(Clone, Debug, PartialEq)]
pub struct Transfer {
    pub door: ElementId,
    pub flow: f64,
    /// The undercut it needs, mm.
    pub gap_mm: f64,
}

/// Room units placed in a room's outer wall.
#[derive(Clone, Debug, PartialEq)]
pub struct RoomUnits {
    pub room: usize,
    pub places: Vec<(DVec2, f64)>,
    pub wall: f64,
    pub wall_id: ElementId,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Design {
    pub level: Option<ElementId>,
    pub rooms: Vec<RoomAir>,
    pub need_supply: f64,
    pub need_extract: f64,
    /// The balanced flow the unit runs at, m³/h.
    pub flow: f64,
    pub unit: Option<usize>,
    /// The unit's place and turn (back to the wall), and the boxes'.
    pub unit_at: Option<(DVec2, f64)>,
    pub boxes: [(DVec2, f64); 2],
    /// The outer wall the outdoor and exhaust ducts reach.
    pub outside: Vec<DVec2>,
    /// The outer wall the unit stands at, its ducts through it.
    pub unit_wall: Option<ElementId>,
    pub branches: Vec<Branch>,
    pub dp_supply: f64,
    pub dp_extract: f64,
    pub transfers: Vec<Transfer>,
    pub room_units: Vec<RoomUnits>,
    pub warnings: Vec<String>,
    /// Height of the flexible ducts above the level, m.
    pub z: f64,
}

struct RoomGeo {
    id: ElementId,
    outline: Vec<DVec2>,
    kind: RoomType,
    decentral: bool,
    number: String,
    name: String,
    height: f64,
    /// What bounds each edge (corner `i` to `i + 1`).
    bounded_by: Vec<Option<ElementId>>,
}

/// A door: its middle, which way it faces (its wall's normal), its
/// width and thickness, and the rooms each side.
struct Door {
    id: ElementId,
    mid: DVec2,
    n: DVec2,
    width: f64,
    thick: f64,
    a: Option<usize>,
    b: Option<usize>,
    /// An open passage (a room separator, a hole), not a door leaf.
    open: bool,
}

fn room_at(rooms: &[RoomGeo], p: DVec2) -> Option<usize> {
    rooms
        .iter()
        .position(|r| creabim_core::polygon::contains(&r.outline, p))
}

fn centroid(poly: &[DVec2]) -> DVec2 {
    let (mut a, mut c) = (0.0, DVec2::ZERO);
    for i in 0..poly.len() {
        let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
        let k = p.perp_dot(q);
        a += k;
        c += (p + q) * k;
    }
    if a.abs() < 1e-9 {
        poly.iter().copied().sum::<DVec2>() / poly.len().max(1) as f64
    } else {
        c / (3.0 * a)
    }
}

fn doors(model: &Model, level: ElementId, rooms: &[RoomGeo]) -> Vec<Door> {
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Opening(o) = &e.kind else {
            continue;
        };
        if o.kind == OpeningKind::Window || (o.kind == OpeningKind::Hole && o.sill > 0.1) {
            continue;
        }
        let Some(ElementKind::Wall(w)) = model.get(o.host).map(|h| &h.kind) else {
            continue;
        };
        if w.level != level {
            continue;
        }
        let along = (w.end - w.start).normalize_or_zero();
        let mid = w.start + along * (o.offset + o.width / 2.0);
        let n = along.perp();
        let d = w.thickness / 2.0 + 0.15;
        out.push(Door {
            id: e.id,
            mid,
            n,
            width: o.width,
            thick: w.thickness,
            a: room_at(rooms, mid + n * d),
            b: room_at(rooms, mid - n * d),
            open: o.kind == OpeningKind::Hole,
        });
    }
    // rooms open to each other: an edge with no wall along it
    for (i, r) in rooms.iter().enumerate() {
        let n = r.outline.len();
        for k in 0..n {
            let walled = r.bounded_by.get(k).copied().flatten().is_some_and(|id| {
                matches!(model.get(id).map(|e| &e.kind), Some(ElementKind::Wall(_)))
            });
            let (a, b) = (r.outline[k], r.outline[(k + 1) % n]);
            if walled || a.distance(b) < 0.6 {
                continue;
            }
            let mid = (a + b) / 2.0;
            let out_n = (b - a).normalize().perp() * -1.0;
            let Some(j) = room_at(rooms, mid + out_n * 0.3) else {
                continue;
            };
            if j <= i {
                continue;
            }
            out.push(Door {
                id: r.id,
                mid,
                n: out_n,
                width: a.distance(b),
                thick: 0.0,
                a: Some(j),
                b: Some(i),
                open: true,
            });
        }
    }
    out
}

/// The room's edges with no room beyond them (its outer walls), with
/// the wall's thickness.
fn outer_edges(model: &Model, rooms: &[RoomGeo], i: usize) -> Vec<(DVec2, DVec2, f64, ElementId)> {
    let r = &rooms[i];
    let Some(ElementKind::Room(room)) = model.get(r.id).map(|e| &e.kind) else {
        return vec![];
    };
    let Some(shape) = creabim_core::room::shape_of(model, room) else {
        return vec![];
    };
    let mut out = vec![];
    let n = shape.outline.len();
    for k in 0..n {
        let (a, b) = (shape.outline[k], shape.outline[(k + 1) % n]);
        let Some(Some(wid)) = shape.bounded_by.get(k) else {
            continue;
        };
        let Some(ElementKind::Wall(w)) = model.get(*wid).map(|e| &e.kind) else {
            continue;
        };
        if a.distance(b) < 0.8 {
            continue;
        }
        // outward: the outline runs counter-clockwise
        let out_n = (b - a).normalize().perp() * -1.0;
        let probe = (a + b) / 2.0 + out_n * (w.thickness + 0.3);
        if room_at(rooms, probe).is_none() {
            out.push((a, b, w.thickness, *wid));
        }
    }
    out
}

/// The turn that sets an object's back (+y) along `n`.
fn turn_to(n: DVec2) -> f64 {
    (-n.x).atan2(n.y).to_degrees()
}

/// The terminals' places in a room: as far from its doors as it goes,
/// 0.6 m in from its corners.
fn terminal_places(r: &RoomGeo, doors: &[DVec2], count: usize) -> Vec<DVec2> {
    let c = centroid(&r.outline);
    let mut cand: Vec<DVec2> = r
        .outline
        .iter()
        .map(|v| *v + (c - *v).normalize_or_zero() * 0.6)
        .filter(|p| creabim_core::polygon::contains(&r.outline, *p))
        .collect();
    if cand.is_empty() {
        cand.push(c);
    }
    let mut out: Vec<DVec2> = vec![];
    for _ in 0..count {
        let best = cand
            .iter()
            .copied()
            .max_by(|p, q| {
                let s = |x: DVec2| {
                    let d = doors
                        .iter()
                        .map(|d| d.distance(x))
                        .fold(f64::INFINITY, f64::min);
                    let d = if d.is_finite() { d } else { x.distance(c) };
                    let o = out
                        .iter()
                        .map(|o| o.distance(x))
                        .fold(f64::INFINITY, f64::min);
                    d.min(o * 1.5)
                };
                s(*p).total_cmp(&s(*q))
            })
            .unwrap_or(c);
        out.push(best);
    }
    out
}

/// An L between `a` and `b` that stays in the room, else straight.
fn ell(a: DVec2, b: DVec2, outline: Option<&[DVec2]>) -> Option<DVec2> {
    if (a.x - b.x).abs() < 0.05 || (a.y - b.y).abs() < 0.05 {
        return None;
    }
    for c in [DVec2::new(b.x, a.y), DVec2::new(a.x, b.y)] {
        let inside = outline.is_none_or(|o| {
            creabim_core::polygon::contains(o, c)
                && creabim_core::polygon::contains(o, (a + c) / 2.0)
                && creabim_core::polygon::contains(o, (c + b) / 2.0)
        });
        if inside {
            return Some(c);
        }
    }
    None
}

fn path_length(p: &[DVec2]) -> f64 {
    p.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// Pressure lost per metre in a round duct, Pa/m.
fn friction(q_m3h: f64, d: f64, rough: f64) -> (f64, f64) {
    let a = std::f64::consts::PI * d * d / 4.0;
    let v = q_m3h / 3600.0 / a;
    let re = v * d / 1.5e-5;
    if re < 1.0 {
        return (v, 0.0);
    }
    let lambda = if re < 2300.0 {
        64.0 / re
    } else {
        0.25 / ((rough / (3.7 * d) + 5.74 / re.powf(0.9)).log10()).powi(2)
    };
    (v, lambda / d * 1.2 * v * v / 2.0)
}

fn ceil_to(v: f64, step: f64) -> f64 {
    (v / step).ceil() * step
}

/// A loggia, a terrace, a balcony: outdoors, not aired.
fn is_outdoor(text: &str) -> bool {
    let t = text.to_lowercase();
    [
        "lodži", "lodzi", "loggia", "teras", "terrace", "balkón", "balkon", "balcony", "zápraž",
        "zapraz", "porch",
    ]
    .iter()
    .any(|w| t.contains(w))
}

/// The level the design is for: the one with the most rooms.
fn level_of(model: &Model) -> Option<ElementId> {
    let mut best: Option<(ElementId, usize)> = None;
    for (e, _) in model.levels() {
        let n = model
            .iter()
            .filter(|x| matches!(&x.kind, ElementKind::Room(r) if r.level == e.id))
            .count();
        if n > 0 && best.is_none_or(|b| n > b.1) {
            best = Some((e.id, n));
        }
    }
    best.map(|b| b.0)
}

/// Design the ventilation of the house's main storey.
pub fn design(model: &Model, ho: &heatloss::Opts, o: &Opts) -> Design {
    let cat = catalogue();
    let mut d = Design::default();
    let Some(level) = level_of(model) else {
        d.warnings
            .push("V modeli nie sú miestnosti: návrh potrebuje miestnosti s typom.".into());
        return d;
    };
    d.level = Some(level);
    let losses = heatloss::building(model, ho);
    let mut geo: Vec<RoomGeo> = vec![];
    let mut need: Vec<f64> = vec![];
    for hl in &losses.rooms {
        let Some(e) = model.get(hl.id) else { continue };
        let ElementKind::Room(r) = &e.kind else {
            continue;
        };
        if r.level != level {
            continue;
        }
        let Some(shape) = creabim_core::room::shape_of(model, r) else {
            continue;
        };
        if is_outdoor(&e.name) || is_outdoor(&r.occupancy) {
            continue;
        }
        geo.push(RoomGeo {
            id: e.id,
            outline: shape.outline,
            kind: hl.kind,
            decentral: r.decentral_hrv,
            number: hl.number.clone(),
            name: hl.name.clone(),
            height: creabim_core::room::height(model, r),
            bounded_by: shape.bounded_by.clone(),
        });
        need.push(hl.v_min.round());
    }
    if geo.is_empty() {
        d.warnings.push("Miestnosti podlažia nemajú obrys.".into());
        return d;
    }
    let ds = doors(model, level, &geo);

    // what each room gets
    for (i, g) in geo.iter().enumerate() {
        let n = g.kind.needs();
        let role = if g.kind == RoomType::Garage {
            Role::Out
        } else if g.decentral {
            Role::Decentral
        } else if n.exhaust && need[i] > 0.0 {
            Role::Extract
        } else if !n.exhaust
            && need[i] > 0.0
            && !matches!(g.kind, RoomType::Hall | RoomType::Staircase)
        {
            Role::Supply
        } else {
            Role::Transfer
        };
        d.rooms.push(RoomAir {
            id: g.id,
            number: g.number.clone(),
            name: g.name.clone(),
            role,
            need: need[i],
            design: 0.0,
            ducts: 0,
            terminals: vec![],
        });
    }
    if d.rooms.iter().any(|r| r.role == Role::Out) {
        d.warnings.push(
            "Garáž sa na vzduch domu nepripája (iný druh prevádzky): vetrá sa samostatne.".into(),
        );
    }
    d.need_supply = d
        .rooms
        .iter()
        .filter(|r| r.role == Role::Supply)
        .map(|r| r.need)
        .sum();
    d.need_extract = d
        .rooms
        .iter()
        .filter(|r| r.role == Role::Extract)
        .map(|r| r.need)
        .sum();

    // the balance: the smaller side raised to the larger
    let q = d.need_supply.max(d.need_extract);
    for r in &mut d.rooms {
        let side = match r.role {
            Role::Supply => d.need_supply,
            Role::Extract => d.need_extract,
            _ => continue,
        };
        r.design = if side > 0.0 {
            ceil_to(r.need * q / side, 5.0)
        } else {
            0.0
        };
    }
    let sum = |role| -> f64 {
        d.rooms
            .iter()
            .filter(|r| r.role == role)
            .map(|r| r.design)
            .sum()
    };
    d.flow = sum(Role::Supply).max(sum(Role::Extract));
    let central = d.flow > 0.0;
    if central && (d.need_supply == 0.0 || d.need_extract == 0.0) {
        d.warnings.push(
            "Chýba prívodná alebo odvodná strana (typy miestností): vyváženie nie je možné.".into(),
        );
    }

    // the unit
    if central {
        let mut fits: Vec<(usize, &Unit)> = cat
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.series == o.series)
            .collect();
        fits.sort_by(|a, b| a.1.q_max.total_cmp(&b.1.q_max));
        d.unit = fits
            .iter()
            .find(|(_, u)| u.q_max * o.use_of_unit >= d.flow)
            .or(fits.last())
            .map(|(i, _)| *i);
        if let Some(u) = d.unit.map(|i| &cat.units[i]) {
            if u.q_max * o.use_of_unit < d.flow {
                d.warnings.push(format!(
                    "Prietok {:.0} m³/h je nad {:.0} % najväčšej jednotky {}.",
                    d.flow,
                    o.use_of_unit * 100.0,
                    u.name
                ));
            }
        }
    }

    // where the unit goes: a technical room, else a laundry, a store,
    // a wardrobe, a hall; on an outer wall for short outdoor ducts
    let order = [
        RoomType::Utility,
        RoomType::Laundry,
        RoomType::Pantry,
        RoomType::Wardrobe,
        RoomType::Hall,
    ];
    let unit_room = order.iter().find_map(|k| {
        geo.iter().enumerate().find_map(|(i, g)| {
            (g.kind == *k && d.rooms[i].role != Role::Decentral && d.rooms[i].role != Role::Out)
                .then_some(i)
        })
    });
    let z = match o.route {
        Route::Floor => -0.06,
        Route::Ceiling => unit_room.map_or(2.6, |i| geo[i].height) + 0.08,
    };
    d.z = z;
    if central {
        match unit_room {
            None => d.warnings.push(
                "Nenašla sa technická miestnosť, práčovňa, sklad ani chodba pre jednotku: umiestni ju ručne.".into(),
            ),
            Some(ui) => {
                let g = &geo[ui];
                let edges = outer_edges(model, &geo, ui);
                let (a, b, n, th) = match edges
                    .iter()
                    .max_by(|x, y| x.0.distance(x.1).total_cmp(&y.0.distance(y.1)))
                {
                    Some(&(a, b, th, wid)) => {
                        d.unit_wall = Some(wid);
                        (a, b, (b - a).normalize().perp() * -1.0, th)
                    }
                    None => {
                        d.warnings.push(format!(
                            "Miestnosť {} nemá obvodovú stenu: nasávanie a výfuk treba viesť ručne.",
                            g.name
                        ));
                        let (a, b) = (g.outline[0], g.outline[1 % g.outline.len()]);
                        (a, b, (b - a).normalize().perp() * -1.0, 0.4)
                    }
                };
                let along = (b - a).normalize();
                let w = d.unit.map_or(0.6, |i| cat.units[i].size[0]);
                let at = a + along * (0.15 + w / 2.0).min(a.distance(b) / 2.0);
                d.unit_at = Some((at, turn_to(n)));
                // through the outer wall to its face and a little out,
                // for the louvre
                d.outside = vec![at + n * (th + 0.08)];
                // the boxes beside it, in the floor or above the ceiling
                let s = at + along * (w / 2.0 + 0.45) - n * 0.35;
                let e = at + along * (w / 2.0 + 1.15) - n * 0.35;
                let e = if creabim_core::polygon::contains(&g.outline, e) {
                    e
                } else {
                    at - n * 1.0
                };
                d.boxes = [(s, turn_to(n)), (e, turn_to(n))];
            }
        }
    }

    // terminals in every central room
    let door_pts = |i: usize| -> Vec<DVec2> {
        ds.iter()
            .filter(|x| x.a == Some(i) || x.b == Some(i))
            .map(|x| x.mid)
            .collect()
    };
    for (i, r) in d.rooms.iter_mut().enumerate() {
        if !matches!(r.role, Role::Supply | Role::Extract) {
            continue;
        }
        let ducts = (r.design / o.per_duct).ceil().max(1.0) as u32;
        let terms = ducts.div_ceil(2) as usize;
        r.ducts = ducts;
        let places = terminal_places(&geo[i], &door_pts(i), terms);
        let mut left = ducts;
        for p in places {
            let k = left.min(2);
            left -= k;
            r.terminals.push(Terminal {
                at: p,
                flow: (r.design * k as f64 / ducts as f64).round(),
                ducts: k,
            });
        }
    }

    // routes: room to room through the doors, to the unit's room
    if let (Some(ui), Some(_)) = (unit_room, d.unit_at) {
        let n = geo.len();
        let mut prev: Vec<Option<(usize, usize)>> = vec![None; n];
        let mut seen = vec![false; n];
        seen[ui] = true;
        let mut queue = std::collections::VecDeque::from([ui]);
        while let Some(i) = queue.pop_front() {
            for (k, dr) in ds.iter().enumerate() {
                let other = if dr.a == Some(i) {
                    dr.b
                } else if dr.b == Some(i) {
                    dr.a
                } else {
                    None
                };
                if let Some(j) = other {
                    if !seen[j] {
                        seen[j] = true;
                        prev[j] = Some((i, k));
                        queue.push_back(j);
                    }
                }
            }
        }
        let rough = crate::pipe_types::by_id("GP").map_or(5e-5, |t| t.roughness_mm / 1000.0);
        for (ri, r) in d.rooms.iter().enumerate() {
            if !matches!(r.role, Role::Supply | Role::Extract) {
                continue;
            }
            let supply = r.role == Role::Supply;
            let target = d.boxes[if supply { 0 } else { 1 }].0;
            if !seen[ri] {
                d.warnings.push(format!(
                    "{} {}: nevedie k nej žiadne dvere z miestnosti jednotky, potrubie treba viesť ručne.",
                    r.number, r.name
                ));
                continue;
            }
            // the doors from this room back to the unit's room
            let mut chain: Vec<(usize, usize)> = vec![]; // (room left, door)
            let mut at = ri;
            while let Some((p, k)) = prev[at] {
                chain.push((at, k));
                at = p;
            }
            for (ti, t) in r.terminals.iter().enumerate() {
                for dn in 0..t.ducts {
                    // a duct of its own, a little aside of its neighbours
                    let off = (dn as f64 - (t.ducts as f64 - 1.0) / 2.0) * 0.1;
                    let mut pts = vec![t.at];
                    let mut room = ri;
                    for &(_, k) in &chain {
                        let dr = &ds[k];
                        let side = if room_at(&geo, dr.mid + dr.n * (dr.thick / 2.0 + 0.15))
                            == Some(room)
                        {
                            1.0
                        } else {
                            -1.0
                        };
                        let along = dr.n.perp();
                        let lane = along * off.clamp(-dr.width / 2.0 + 0.1, dr.width / 2.0 - 0.1);
                        let here = dr.mid + lane + dr.n * side * (dr.thick / 2.0 + 0.3);
                        let there = dr.mid + lane - dr.n * side * (dr.thick / 2.0 + 0.3);
                        let last = *pts.last().unwrap();
                        if let Some(c) = ell(last, here, Some(&geo[room].outline)) {
                            pts.push(c);
                        }
                        pts.push(here);
                        pts.push(there);
                        room = room_at(&geo, there).unwrap_or(room);
                    }
                    let last = *pts.last().unwrap();
                    let end = target + DVec2::new(off, 0.0);
                    if let Some(c) = ell(last, end, Some(&geo[room].outline)) {
                        pts.push(c);
                    }
                    pts.push(end);
                    pts.dedup_by(|a, b| a.distance(*b) < 0.02);
                    let length = path_length(&pts) + 2.0 * 0.3;
                    let flow = t.flow / t.ducts as f64;
                    let (v, rpm) = friction(flow, 0.075, rough);
                    let bends = pts.len().saturating_sub(2) as u32 + 2;
                    let dyn_p = 1.2 * v * v / 2.0;
                    let dp = rpm * length + o.zeta_bend * bends as f64 * dyn_p;
                    d.branches.push(Branch {
                        room: ri,
                        terminal: ti,
                        supply,
                        path: pts,
                        length,
                        flow,
                        velocity: v,
                        bends,
                        dp,
                        throttle: 0.0,
                    });
                }
            }
        }
        for supply in [true, false] {
            let worst = d
                .branches
                .iter()
                .filter(|b| b.supply == supply)
                .map(|b| b.dp)
                .fold(0.0, f64::max);
            for b in d.branches.iter_mut().filter(|b| b.supply == supply) {
                b.throttle = worst - b.dp;
            }
            // the main from the unit to its box, and outdoors, Spiro Ø160
            let main_q = d.flow;
            let (_, rpm) = friction(main_q, 0.16, 1.5e-4);
            let main = rpm * 3.0;
            if supply {
                d.dp_supply = worst + main;
            } else {
                d.dp_extract = worst + main;
            }
        }
        d.warnings.push(
            "Tlakové straty sú len trenie a ohyby potrubia: straty tanierových ventilov, boxov, tlmičov a žalúzií doplniť z katalógu výrobcu; dispozičný tlak jednotky overiť podľa jej krivky.".into(),
        );
    }

    // the transfer: under the doors of the rooms air is taken from
    for (i, r) in d.rooms.iter().enumerate() {
        if r.role != Role::Extract {
            continue;
        }
        let mine: Vec<&Door> = ds
            .iter()
            .filter(|x| !x.open && (x.a == Some(i) || x.b == Some(i)))
            .collect();
        let share = r.design / mine.len().max(1) as f64;
        for dr in mine {
            let gap = share / 3600.0 / o.transfer_v / dr.width.max(0.6) * 1000.0;
            d.transfers.push(Transfer {
                door: dr.id,
                flow: share.round(),
                gap_mm: gap.ceil(),
            });
        }
    }

    // room units in the outer walls of the rooms marked so
    if let Some(ru) = cat.room_units.get(o.room_unit) {
        for (i, r) in d.rooms.iter().enumerate() {
            if r.role != Role::Decentral {
                continue;
            }
            let count = (r.need / ru.q_hr).ceil().max(1.0) as usize;
            let edges = outer_edges(model, &geo, i);
            let Some(&(a, b, thick, wall_id)) = edges
                .iter()
                .max_by(|x, y| x.0.distance(x.1).total_cmp(&y.0.distance(y.1)))
            else {
                d.warnings.push(format!(
                    "{} {}: nemá obvodovú stenu, decentrálnu jednotku nie je kam osadiť.",
                    r.number, r.name
                ));
                continue;
            };
            if thick < ru.wall_min - 1e-6 || thick > ru.wall_max + 1e-6 {
                d.warnings.push(format!(
                    "{} {}: stena {:.0} mm je mimo rozsahu puzdra {} ({:.0}–{:.0} mm): treba predĺženie.",
                    r.number,
                    r.name,
                    thick * 1000.0,
                    ru.name,
                    ru.wall_min * 1000.0,
                    ru.wall_max * 1000.0
                ));
            }
            let n = (b - a).normalize().perp() * -1.0;
            let windows = window_spans(model, a, b);
            let len = a.distance(b);
            let mut places = vec![];
            for k in 0..count {
                let mut t = len * (k as f64 + 1.0) / (count as f64 + 1.0);
                // off the windows, and 0.3 m clear of the corners
                for _ in 0..8 {
                    match windows.iter().find(|(p, q)| t > p - 0.25 && t < q + 0.25) {
                        Some((_, q)) => t = q + 0.35,
                        None => break,
                    }
                }
                let t = t.clamp(0.3, (len - 0.3).max(0.3));
                places.push((a + (b - a) / len * t, turn_to(n)));
            }
            d.room_units.push(RoomUnits {
                room: i,
                places,
                wall: thick,
                wall_id,
            });
        }
    }
    d.warnings.push(
        "Potreby vzduchu sú z typov miestností (0,5 1/h podľa STN EN 12831-1 tab. B.7; odvod kuchyňa 100, kúpeľňa 90, WC 50 m³/h): zdroj odvodných hodnôt treba potvrdiť normou.".into(),
    );
    d
}

/// The windows along the edge `a`–`b`, as distances from `a`.
fn window_spans(model: &Model, a: DVec2, b: DVec2) -> Vec<(f64, f64)> {
    let len = a.distance(b);
    let dir = (b - a) / len.max(1e-9);
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Opening(o) = &e.kind else {
            continue;
        };
        if o.kind == OpeningKind::Door && o.sill > 0.5 {
            continue;
        }
        let Some(ElementKind::Wall(w)) = model.get(o.host).map(|h| &h.kind) else {
            continue;
        };
        let along = (w.end - w.start).normalize_or_zero();
        let p = w.start + along * o.offset;
        let q = w.start + along * (o.offset + o.width);
        // on this edge: near its line
        let off = |x: DVec2| (x - a).perp_dot(dir).abs();
        if off(p) > w.thickness + 0.05 || off(q) > w.thickness + 0.05 {
            continue;
        }
        let (s, t) = ((p - a).dot(dir), (q - a).dot(dir));
        let (s, t) = (s.min(t), s.max(t));
        if t > 0.0 && s < len {
            out.push((s, t));
        }
    }
    out
}

// ---------------------------------------------------------------- model

/// A core hole through `wall` round a duct crossing it at `p`, its
/// middle `z` above the level, `size` square.
fn hole(model: &Model, wall: ElementId, p: DVec2, z: f64, size: f64) -> Option<Element> {
    let Some(ElementKind::Wall(w)) = model.get(wall).map(|e| &e.kind) else {
        return None;
    };
    let along = (w.end - w.start).normalize_or_zero();
    let t = (p - w.start).dot(along);
    let len = w.start.distance(w.end);
    if t < size / 2.0 || t > len - size / 2.0 {
        return None;
    }
    Some(Element::new(
        format!("{TAG} – prestup"),
        ElementKind::Opening(creabim_core::Opening {
            host: wall,
            kind: OpeningKind::Hole,
            offset: t - size / 2.0,
            sill: (z - size / 2.0 - w.base_offset).max(0.0),
            width: size,
            height: size,
            material: None,
            frame: Default::default(),
            flip_hand: false,
            flip_facing: false,
            no_outer_sill: false,
            no_inner_sill: false,
            threshold: false,
            threshold_material: None,
            setback: None,
            reveal: None,
            division: None,
            door: None,
            texts: None,
            performance: None,
        }),
    ))
}

fn gp_duct(level: ElementId, system: &str, a: DVec2, b: DVec2, z: f64) -> Pipe {
    Pipe {
        level,
        system: system.into(),
        start: a.extend(z),
        end: b.extend(z),
        pipe_type: "GP".into(),
        nominal_mm: 75.0,
        outer_mm: 95.0,
        inner_mm: 75.0,
        height_mm: None,
        insulation_mm: 0.0,
        insulation_set: false,
        mark: None,
    }
}

fn spiro(level: ElementId, system: &str, a: DVec3, b: DVec3, insulation: f64) -> Pipe {
    Pipe {
        level,
        system: system.into(),
        start: a,
        end: b,
        pipe_type: "VZK".into(),
        nominal_mm: 160.0,
        outer_mm: 161.0,
        inner_mm: 160.0,
        height_mm: None,
        insulation_mm: insulation,
        insulation_set: true,
        mark: None,
    }
}

/// The objects a design places, for their types to be added first.
pub fn objects(d: &Design, o: &Opts) -> Vec<CatalogueObject> {
    let cat = catalogue();
    let mut out = vec![];
    if let (Some(u), Some(_)) = (d.unit.map(|i| &cat.units[i]), d.unit_at) {
        let [w, dd, h] = u.size;
        out.push(cat_objects::hrv_unit(
            &format!("{UNIT_KEY}{}", u.code),
            &u.name,
            w,
            dd,
            h,
            u.series == "EC5",
        ));
        let b = box_item(d);
        if let Some(b) = b {
            out.push(cat_objects::duct_box(
                &format!("{BOX_KEY}{}", b.code),
                &b.name,
                0.6,
                0.25,
                0.12,
            ));
        }
    }
    if let Some(ru) = cat.room_units.get(o.room_unit) {
        for r in &d.room_units {
            out.push(cat_objects::room_hrv(
                &format!("{ROOM_UNIT_KEY}{}", ru.code),
                &ru.name,
                ru.hole,
                r.wall,
                o.room_unit_at,
            ));
        }
    }
    out.dedup_by(|a, b| a.key == b.key);
    out
}

/// The distribution box the most ducts on one side fit.
fn box_item(d: &Design) -> Option<&'static Item> {
    let cat = catalogue();
    let most = [true, false]
        .iter()
        .map(|s| d.branches.iter().filter(|b| b.supply == *s).count() as u32)
        .max()
        .unwrap_or(0);
    let mut boxes: Vec<&Item> = cat.items.iter().filter(|i| i.outlets > 0).collect();
    boxes.sort_by_key(|i| i.outlets);
    boxes
        .iter()
        .find(|i| i.outlets >= most)
        .or(boxes.last())
        .copied()
}

/// What the design places: the elements it put before (by their name)
/// are removed. The objects' types must be in the model already
/// ([`objects`]).
pub fn apply(model: &Model, d: &Design, o: &Opts) -> Transaction {
    let cat = catalogue();
    let mut tx = Transaction::new("Návrh rekuperácie");
    let old: Vec<ElementId> = model
        .iter()
        .filter(|e| e.name.starts_with(TAG))
        .map(|e| e.id)
        .collect();
    tx = tx.remove_all_cascade(model, &old);
    let Some(level) = d.level else {
        return tx;
    };
    let type_for = |key: &str| creabim_core::objects::type_for_key(model, key);
    let place = |key: &str, name: &str, at: DVec2, rot: f64| -> Option<Element> {
        let t = type_for(key)?;
        let mut e = creabim_core::objects::instance(t, name, level, at, rot);
        if let (ElementKind::Object(ob), Some(ty)) =
            (&mut e.kind, creabim_core::objects::type_of_id(model, t))
        {
            ob.offset = ty.offset;
        }
        Some(e)
    };
    let z = d.z;
    if let (Some(u), Some((at, rot))) = (d.unit.map(|i| &cat.units[i]), d.unit_at) {
        if let Some(e) = place(
            &format!("{UNIT_KEY}{}", u.code),
            &format!("{TAG} – {}", u.name),
            at,
            rot,
        ) {
            tx = tx.insert(e);
        }
        if let Some(b) = box_item(d) {
            for (k, (p, r)) in d.boxes.iter().enumerate() {
                let what = if k == 0 { "prívod" } else { "odvod" };
                if let Some(mut e) = place(
                    &format!("{BOX_KEY}{}", b.code),
                    &format!("{TAG} – box {what}"),
                    *p,
                    *r,
                ) {
                    if let ElementKind::Object(ob) = &mut e.kind {
                        ob.offset = if o.route == Route::Floor {
                            -0.12
                        } else {
                            z - 0.06
                        };
                    }
                    tx = tx.insert(e);
                }
            }
        }
        // mains: unit to boxes (with a silencer), outdoor air and exhaust
        let uh = if u.series == "EC5" {
            z
        } else {
            u.size[2] + 0.15
        };
        let n = DVec2::from_angle((rot + 90.0).to_radians());
        let out = d.outside.first().copied().unwrap_or(at + n * 0.5);
        for (sys, to, sil) in [
            ("VZP", Some(d.boxes[0].0), true),
            ("VZO", Some(d.boxes[1].0), true),
            ("VZC", None, false),
            ("VZE", None, false),
        ] {
            let (a, b, ins) = match to {
                Some(p) => (at.extend(uh), p.extend(z), 0.0),
                None => {
                    let side = if sys == "VZC" { -0.2 } else { 0.2 };
                    let s = at + n.perp() * side;
                    // its core hole through the outer wall (Ø160 with
                    // 30 mm of insulation: 250 mm)
                    if let Some(h) = d.unit_wall.and_then(|w| hole(model, w, s, uh, 0.25)) {
                        tx = tx.insert(h);
                    }
                    (s.extend(uh), (out + n.perp() * side).extend(uh), 0.03)
                }
            };
            // up and over, then down to the box
            let mid = DVec3::new(b.x, b.y, a.z);
            let pe = Element::new(
                format!("{TAG} – {sys}"),
                ElementKind::Pipe(spiro(level, sys, a, mid, ins)),
            );
            let pid = pe.id;
            tx = tx.insert(pe);
            if (mid.z - b.z).abs() > 0.01 {
                tx = tx.insert(Element::new(
                    format!("{TAG} – {sys}"),
                    ElementKind::Pipe(spiro(level, sys, mid, b, ins)),
                ));
            }
            if sil {
                tx = tx.insert(Element::new(
                    format!("{TAG} – tlmič {sys}"),
                    ElementKind::Valve(Valve::new(pid, 0.5, "tlmic")),
                ));
            }
        }
    }
    // flexible ducts and their plate valves
    for (bi, b) in d.branches.iter().enumerate() {
        let r = &d.rooms[b.room];
        let t = &r.terminals[b.terminal];
        let sys = if b.supply { "VZP" } else { "VZO" };
        let mut first = None;
        for w in b.path.windows(2) {
            let e = Element::new(
                format!("{TAG} – {sys} {}", r.number),
                ElementKind::Pipe(gp_duct(level, sys, w[1], w[0], z)),
            );
            if first.is_none() {
                first = Some(e.id);
            }
            tx = tx.insert(e);
        }
        // the valve once a terminal, on its first duct's room end
        let lead = d.branches[..bi]
            .iter()
            .all(|x| !(x.room == b.room && x.terminal == b.terminal));
        if let (Some(pid), true) = (first, lead) {
            let mut v = Valve::new(pid, 1.0, if b.supply { "vyustP" } else { "vyustO" });
            v.flow_m3h = Some(t.flow);
            tx = tx.insert(Element::new(
                format!("{TAG} – ventil {}", r.number),
                ElementKind::Valve(v),
            ));
        }
    }
    // room units
    if let Some(ru) = cat.room_units.get(o.room_unit) {
        for r in &d.room_units {
            for (p, rot) in &r.places {
                if let Some(e) = place(
                    &format!("{ROOM_UNIT_KEY}{}", ru.code),
                    &format!("{TAG} – {} {}", ru.name, d.rooms[r.room].number),
                    *p,
                    *rot,
                ) {
                    tx = tx.insert(e);
                }
                // the core hole for its sleeve
                if let Some(h) = hole(
                    model,
                    r.wall_id,
                    *p,
                    o.room_unit_at + ru.hole / 2.0,
                    ru.hole + 0.02,
                ) {
                    tx = tx.insert(h);
                }
            }
        }
    }
    tx
}

// ---------------------------------------------------------------- bill

/// A line of the bill.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub group: &'static str,
    pub code: String,
    pub name: String,
    pub qty: f64,
    pub unit: String,
    pub price: Option<f64>,
    pub source: String,
}

impl Line {
    pub fn total(&self) -> Option<f64> {
        self.price.map(|p| p * self.qty)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Bill {
    pub lines: Vec<Line>,
    /// Without VAT, the VAT and with it, EUR.
    pub net: f64,
    pub vat: f64,
    pub gross: f64,
    /// Lines without a price.
    pub unpriced: usize,
}

fn line(group: &'static str, it: Option<&Item>, qty: f64) -> Option<Line> {
    let it = it?;
    (qty > 0.0).then(|| Line {
        group,
        code: it.code.clone(),
        name: it.name.clone(),
        qty,
        unit: it.unit.clone(),
        price: it.price,
        source: it.source.clone(),
    })
}

/// The bill of the ventilation with heat recovery, as the model has it.
pub fn bill(model: &Model) -> Bill {
    let cat = catalogue();
    let mut lines: Vec<Line> = vec![];
    let mut count = |key: &str| -> Vec<(DVec2, String)> {
        let _ = &mut lines;
        model
            .iter()
            .filter_map(|e| {
                let ElementKind::Object(ob) = &e.kind else {
                    return None;
                };
                let k = creabim_core::objects::type_of(model, e)?
                    .catalogue
                    .clone()?;
                k.strip_prefix(key).map(|c| (ob.position, c.to_string()))
            })
            .collect()
    };
    let units = count(UNIT_KEY);
    let boxes = count(BOX_KEY);
    let rooms = count(ROOM_UNIT_KEY);
    let mut lines: Vec<Line> = vec![];
    let mut tally =
        |group: &'static str, codes: &[(DVec2, String)], find: &dyn Fn(&str) -> Option<Line>| {
            let mut seen: Vec<(String, f64)> = vec![];
            for (_, c) in codes {
                match seen.iter_mut().find(|x| x.0 == *c) {
                    Some(x) => x.1 += 1.0,
                    None => seen.push((c.clone(), 1.0)),
                }
            }
            for (c, n) in seen {
                if let Some(mut l) = find(&c) {
                    l.group = group;
                    l.qty = n;
                    lines.push(l);
                }
            }
        };
    tally("Jednotka", &units, &|c| {
        cat.units.iter().find(|u| u.code == c).map(|u| Line {
            group: "",
            code: u.code.clone(),
            name: u.name.clone(),
            qty: 0.0,
            unit: "ks".into(),
            price: u.price,
            source: u.source.clone(),
        })
    });
    tally("Decentrálna rekuperácia", &rooms, &|c| {
        cat.room_units.iter().find(|u| u.code == c).map(|u| Line {
            group: "",
            code: u.code.clone(),
            name: u.name.clone(),
            qty: 0.0,
            unit: "ks".into(),
            price: u.price,
            source: u.source.clone(),
        })
    });
    tally("Rozvody", &boxes, &|c| {
        line("", cat.items.iter().find(|i| i.code == c), 1.0)
    });
    if !units.is_empty() {
        lines.extend(line("Jednotka", cat.item("ovladac"), units.len() as f64));
    }

    // ducts
    let (mut gp, mut spiro, mut insul) = (0.0, 0.0, 0.0);
    let mut gp_ends: Vec<DVec2> = vec![];
    let mut outdoor = 0;
    for e in model.iter() {
        let ElementKind::Pipe(p) = &e.kind else {
            continue;
        };
        let air = matches!(p.system.as_str(), "VZP" | "VZO" | "VZC" | "VZE");
        if !air {
            continue;
        }
        let len = p.start.distance(p.end);
        if p.pipe_type == "GP" {
            gp += len;
            gp_ends.push(p.start.truncate());
            gp_ends.push(p.end.truncate());
        } else if p.pipe_type == "VZK" && (units.len() + boxes.len()) > 0 {
            spiro += len;
            if matches!(p.system.as_str(), "VZC" | "VZE") {
                let ins = p.insulation_mm.max(30.0) / 1000.0;
                insul += len * std::f64::consts::PI * (p.outer_mm / 1000.0 + 2.0 * ins);
            }
        }
    }
    for e in model.iter() {
        if let ElementKind::Pipe(p) = &e.kind {
            if p.pipe_type == "VZK"
                && matches!(p.system.as_str(), "VZC" | "VZE")
                && e.name.starts_with(TAG)
            {
                outdoor += 1;
            }
        }
    }
    let _ = outdoor;
    lines.extend(line("Rozvody", cat.item("gp75"), (gp * 1.05).ceil()));
    // terminals: their box by the ducts ending there
    let (mut t1, mut t2, mut vp, mut vo, mut sil) = (0.0, 0.0, 0.0, 0.0, 0.0);
    let mut ducts_at_boxes = 0usize;
    for e in model.iter() {
        let ElementKind::Valve(v) = &e.kind else {
            continue;
        };
        let Some(ElementKind::Pipe(p)) = model.get(v.pipe).map(|x| &x.kind) else {
            continue;
        };
        match v.kind.as_str() {
            "vyustP" | "vyustO" if p.pipe_type == "GP" => {
                let end = if v.t < 0.5 { p.start } else { p.end }.truncate();
                let n = gp_ends.iter().filter(|x| x.distance(end) < 0.2).count();
                if n >= 2 {
                    t2 += 1.0;
                } else {
                    t1 += 1.0;
                }
                if v.kind == "vyustP" {
                    vp += 1.0;
                } else {
                    vo += 1.0;
                }
            }
            "tlmic" => sil += 1.0,
            _ => {}
        }
    }
    for (b, _) in &boxes {
        ducts_at_boxes += gp_ends.iter().filter(|x| x.distance(*b) < 0.5).count();
    }
    lines.extend(line("Koncové prvky", cat.item("tbox1"), t1));
    lines.extend(line("Koncové prvky", cat.item("tbox2"), t2));
    lines.extend(line("Koncové prvky", cat.item("ventilP"), vp));
    lines.extend(line("Koncové prvky", cat.item("ventilO"), vo));
    let outlets: u32 = boxes
        .iter()
        .filter_map(|(_, c)| cat.items.iter().find(|i| i.code == *c))
        .map(|i| i.outlets)
        .sum();
    lines.extend(line(
        "Rozvody",
        cat.item("zaslepka"),
        (outlets as f64 - ducts_at_boxes as f64).max(0.0),
    ));
    lines.extend(line("Rozvody", cat.item("spiro160"), (spiro * 1.05).ceil()));
    lines.extend(line(
        "Rozvody",
        cat.item("izolacia"),
        (insul * 10.0).ceil() / 10.0,
    ));
    lines.extend(line("Rozvody", cat.item("tlmic"), sil));
    lines.extend(line(
        "Rozvody",
        cat.item("zaluzia"),
        2.0 * units.len() as f64,
    ));
    lines.extend(line("Rozvody", cat.item("paska"), units.len() as f64));

    let rank = |g: &str| {
        [
            "Jednotka",
            "Decentrálna rekuperácia",
            "Rozvody",
            "Koncové prvky",
        ]
        .iter()
        .position(|x| *x == g)
        .unwrap_or(9)
    };
    lines.sort_by_key(|l| rank(l.group));
    let net: f64 = lines.iter().filter_map(|l| l.total()).sum();
    let unpriced = lines.iter().filter(|l| l.price.is_none()).count();
    Bill {
        net,
        vat: net * cat.vat,
        gross: net * (1.0 + cat.vat),
        unpriced,
        lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogue_reads() {
        let c = catalogue();
        assert!(c.units.len() >= 3 && !c.room_units.is_empty());
        assert!(c.item("gp75").is_some() && c.item("tbox2").is_some());
        assert!(crate::pipe_types::by_id("GP").is_some());
    }

    #[test]
    fn friction_of_a_gp_duct_is_small() {
        let (v, r) = friction(25.0, 0.075, 5e-5);
        assert!((v - 1.57).abs() < 0.01, "{v}");
        assert!(r > 0.3 && r < 1.0, "{r}");
    }

    #[test]
    fn the_sample_house_is_designed() {
        let model = creabim_core::starter::starter_house();
        let d = design(&model, &heatloss::Opts::default(), &Opts::default());
        assert!(d.level.is_some());
        for r in &d.rooms {
            eprintln!(
                "{} {} {:?} need {} design {} ducts {} terms {}",
                r.number,
                r.name,
                r.role,
                r.need,
                r.design,
                r.ducts,
                r.terminals.len()
            );
        }
        eprintln!(
            "flow {} unit {:?} at {:?} branches {} dp {:.1}/{:.1}",
            d.flow,
            d.unit.map(|i| &catalogue().units[i].name),
            d.unit_at,
            d.branches.len(),
            d.dp_supply,
            d.dp_extract
        );
        for w in &d.warnings {
            eprintln!("W: {w}");
        }
        for t in &d.transfers {
            eprintln!("T: {:?}", t);
        }
        let objs = objects(&d, &Opts::default());
        let mut doc = creabim_core::Document::from_model(model.clone());
        for c in &objs {
            let (tx, _) = creabim_core::objects::add_to_model(doc.model(), c);
            doc.commit(tx).unwrap();
        }
        let tx = apply(doc.model(), &d, &Opts::default());
        doc.commit(tx).unwrap();
        let b = bill(doc.model());
        for l in &b.lines {
            eprintln!(
                "B: {} | {} | {} | {} {} | {:?}",
                l.group, l.code, l.name, l.qty, l.unit, l.price
            );
        }
        eprintln!(
            "net {:.2} gross {:.2} unpriced {}",
            b.net, b.gross, b.unpriced
        );
        assert!(b.net > 0.0);
        // the design never panics; with rooms it balances
        if d.flow > 0.0 {
            let s: f64 = d
                .rooms
                .iter()
                .filter(|r| r.role == Role::Supply)
                .map(|r| r.design)
                .sum();
            let e: f64 = d
                .rooms
                .iter()
                .filter(|r| r.role == Role::Extract)
                .map(|r| r.design)
                .sum();
            assert!(s >= d.need_supply && e >= d.need_extract);
        }
    }

    #[test]
    fn room_units_and_outdoor_ducts_get_core_holes() {
        let mut model = creabim_core::starter::starter_house();
        let study = model
            .iter()
            .find(|e| matches!(&e.kind, ElementKind::Room(_)) && e.name.contains("Study"))
            .map(|e| e.id)
            .unwrap();
        let mut doc = creabim_core::Document::from_model(model.clone());
        let before = doc.model().get(study).cloned().unwrap();
        let mut after = before.clone();
        if let ElementKind::Room(r) = &mut after.kind {
            r.decentral_hrv = true;
        }
        doc.commit(Transaction::new("dec").replace(before, after))
            .unwrap();
        model = doc.model().clone();
        let o = Opts::default();
        let d = design(&model, &heatloss::Opts::default(), &o);
        assert_eq!(d.room_units.len(), 1);
        assert!(d.unit_wall.is_some());
        for c in &objects(&d, &o) {
            let (tx, _) = creabim_core::objects::add_to_model(doc.model(), c);
            doc.commit(tx).unwrap();
        }
        doc.commit(apply(doc.model(), &d, &o)).unwrap();
        let holes = doc
            .model()
            .iter()
            .filter(|e| e.name == format!("{TAG} – prestup"))
            .count();
        assert!(holes >= 3, "{holes}");
        // designing again replaces, not doubles
        let d2 = design(doc.model(), &heatloss::Opts::default(), &o);
        doc.commit(apply(doc.model(), &d2, &o)).unwrap();
        let again = doc
            .model()
            .iter()
            .filter(|e| e.name == format!("{TAG} – prestup"))
            .count();
        assert_eq!(holes, again);
        let b = bill(doc.model());
        assert!(b.lines.iter().any(|l| l.group == "Decentrálna rekuperácia"));
    }
}
