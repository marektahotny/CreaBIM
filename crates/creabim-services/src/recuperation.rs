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
//! loss of the GP ducts is read from ATREA's graph (element catalogue
//! A2328 26/06), of the Spiro mains from their friction (Darcy–Weisbach,
//! Swamee–Jain), plus their bends; the unit's figures come from its
//! technical sheet.
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
    /// "Pro-V" and "Slim" stand at a wall; "Pro" and "Easy2" hang under the ceiling.
    #[serde(rename = "rada")]
    pub series: String,
    /// Its largest flow, m³/h.
    #[serde(rename = "qMax")]
    pub q_max: f64,
    #[serde(rename = "cena")]
    pub price: Option<f64>,
    /// Sound power into the supply duct, dB(A), where the maker's data
    /// are given.
    #[serde(rename = "lw", default)]
    pub lw: Option<f64>,
    /// Width, depth, height, m (as the catalogue gives them).
    #[serde(rename = "rozmer")]
    pub size: [f64; 3],
    /// Its duct spigots, mm.
    #[serde(rename = "hrdlo", default = "spigot")]
    pub spigot: f64,
    /// Sound power radiated by its casing at Qref, dB(A).
    #[serde(rename = "lwOkolie", default)]
    pub lw_case: Option<f64>,
    #[serde(rename = "zdroj", default)]
    pub source: String,
}

impl Unit {
    /// Whether the unit hangs under the ceiling (Pro, Easy2) instead of standing at a wall (Pro-V, Slim).
    pub fn on_ceiling(&self) -> bool {
        !matches!(self.series.as_str(), "Pro-V" | "Slim")
    }
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
    /// Its sound pressure level at the most it is run at, dB(A), as
    /// the maker states it.
    #[serde(rename = "hluk", default)]
    pub noise: Option<f64>,
    #[serde(rename = "cena")]
    pub price: Option<f64>,
    #[serde(rename = "zdroj", default)]
    pub source: String,
    #[serde(rename = "pozn", default)]
    pub note: String,
}

fn spigot() -> f64 {
    160.0
}

/// The external pressure the catalogue gives the units' largest flow
/// at, Pa: a fan that reaches `qMax` there has at least this much at
/// any smaller flow.
pub const UNIT_PRESSURE_PA: f64 = 100.0;

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
    /// The unit's series: "Pro-V" or "Slim" (at a wall), "Pro" or "Easy2" (under the ceiling).
    pub series: String,
    /// The most air one duct Ø75 carries, m³/h (GP: 15–50 recommended
    /// by ATREA; quieter the less).
    pub per_duct: f64,
    /// GP runs into one plate valve: 1 (PKS Ø100/Ø100 GP, valve Ø100) or
    /// 2 (PKS 2×Ø100/Ø125 GP, valve Ø125). George 2026-11-27: one, at
    /// 45 m³/h a run.
    pub runs_per_valve: u32,
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
            series: "Pro-V".into(),
            per_duct: 45.0,
            runs_per_valve: 1,
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

/// The noise in a protected room against the decree's limit.
#[derive(Clone, Debug, PartialEq)]
pub struct Noise {
    pub room: usize,
    /// Category of the indoor space (A, B) and its limits LAmax,p for
    /// day, evening and night, dB.
    pub category: char,
    pub limits: [f64; 3],
    /// The level the equipment makes there, where it is known.
    pub level: Option<f64>,
    pub ok: Option<bool>,
    pub note: String,
}

/// Decree 549/2007 Z. z. (as amended by 237/2009 Z. z.), table 3:
/// LAmax,p from inside sources (the building's equipment), day,
/// evening, night. A: hospital wards; B: living rooms of dwellings.
pub const NOISE_A: [f64; 3] = [35.0, 30.0, 25.0];
pub const NOISE_B: [f64; 3] = [40.0, 40.0, 30.0];
pub const NOISE_SOURCE: &str = "vyhláška MZ SR 549/2007 Z. z. v znení 237/2009 Z. z., tab. 3";

/// The rooms the decree protects, and their category.
fn noise_category(k: RoomType) -> Option<char> {
    match k {
        RoomType::Ward => Some('A'),
        RoomType::Living
        | RoomType::Bedroom
        | RoomType::Children
        | RoomType::Dining
        | RoomType::Office
        | RoomType::Other => Some('B'),
        _ => None,
    }
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
    /// The direction the manifolds' branches line up along (the wall).
    pub box_along: DVec2,
    /// No outer wall at the unit: outdoor air and exhaust go straight up
    /// through the ceiling and roof to this height, m above the floor.
    pub through_roof: Option<f64>,
    /// The points the unit's mains pass on their way to the manifolds
    /// (a door, when the manifolds sit outside the unit's room).
    pub main_via: Vec<DVec2>,
    /// Air velocity in the unit's mains, m/s.
    pub main_velocity: f64,
    /// Length of the mains on the supply and the extract side, m.
    pub main_length: [f64; 2],
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
    pub noise: Vec<Noise>,
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
    /// Under or through a partition, not a door: needs a penetration.
    partition: bool,
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
            partition: false,
        });
    }
    // through a partition (not load-bearing, at most 0.2 m thick) the
    // rooms share: a passage under it, as wide as their common edge less
    // 0.2 m at each corner (needs a penetration in the partition)
    for (i, r) in rooms.iter().enumerate() {
        let n = r.outline.len();
        for k in 0..n {
            let Some(wid) = r.bounded_by.get(k).copied().flatten() else {
                continue;
            };
            let Some(ElementKind::Wall(w)) = model.get(wid).map(|e| &e.kind) else {
                continue;
            };
            if w.bearing == Some(true) || w.thickness > 0.2 + 1e-6 {
                continue;
            }
            let (a, b) = (r.outline[k], r.outline[(k + 1) % n]);
            let out_n = outward(&r.outline, a, b);
            let dir = (b - a).normalize_or_zero();
            // the stretch of this edge the other room also lies along
            let probe = |t: f64| room_at(rooms, a + dir * t + out_n * (w.thickness + 0.15));
            let len = a.distance(b);
            let Some(j) = probe(len / 2.0) else {
                continue;
            };
            if j <= i {
                continue;
            }
            let step = 0.05;
            let ts: Vec<f64> = (0..=((len / step) as usize))
                .map(|q| q as f64 * step)
                .filter(|t| probe(*t) == Some(j))
                .collect();
            let (Some(&t0), Some(&t1)) = (ts.first(), ts.last()) else {
                continue;
            };
            let width = t1 - t0 - 0.4;
            if width < 0.3 {
                continue;
            }
            out.push(Door {
                id: wid,
                mid: a + dir * ((t0 + t1) / 2.0) + out_n * (w.thickness / 2.0),
                n: out_n,
                width,
                thick: w.thickness,
                a: Some(j),
                b: Some(i),
                open: false,
                partition: true,
            });
        }
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
            let out_n = outward(&r.outline, a, b);
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
                partition: false,
            });
        }
    }
    out
}

/// The room's edges with no room beyond them (its outer walls), with
/// the wall's thickness.
/// The normal of the edge `a`–`b` of `outline` pointing out of it,
/// whichever way the outline runs.
fn outward(outline: &[DVec2], a: DVec2, b: DVec2) -> DVec2 {
    let n = (b - a).normalize_or_zero().perp();
    let mid = (a + b) / 2.0;
    if creabim_core::polygon::contains(outline, mid + n * 0.05) {
        -n
    } else {
        n
    }
}

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
        let out_n = outward(&shape.outline, a, b);
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
fn ell(a: DVec2, b: DVec2, outline: Option<&[DVec2]>, vertical_first: bool) -> Option<DVec2> {
    if (a.x - b.x).abs() < 1e-4 || (a.y - b.y).abs() < 1e-4 {
        return None;
    }
    let cs = if vertical_first {
        [DVec2::new(a.x, b.y), DVec2::new(b.x, a.y)]
    } else {
        [DVec2::new(b.x, a.y), DVec2::new(a.x, b.y)]
    };
    for c in cs {
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

/// The corner of an L from the path's last point to `b`, going on the
/// way the path last went first (so runs side by side stay apart).
fn ell_from(pts: &[DVec2], b: DVec2, outline: Option<&[DVec2]>) -> Option<DVec2> {
    let a = *pts.last()?;
    let vertical = pts.len() >= 2 && {
        let d = a - pts[pts.len() - 2];
        d.y.abs() > d.x.abs()
    };
    if vertical {
        ell(a, b, outline, true)
    } else {
        ell(a, b, outline, false)
    }
}

/// The path without the points it runs straight through.
fn straighten(pts: Vec<DVec2>) -> Vec<DVec2> {
    let mut out: Vec<DVec2> = vec![];
    for p in pts {
        // a point on the line on, or where it turns back on itself (a
        // stub), goes
        while out.len() >= 2 {
            let (a, b) = (out[out.len() - 2], out[out.len() - 1]);
            if (b - a).perp_dot(p - b).abs() < 1e-6 {
                out.pop();
            } else {
                break;
            }
        }
        out.push(p);
    }
    out
}

fn path_length(p: &[DVec2]) -> f64 {
    p.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// ATREA GP Ø90/75: pressure loss against flow, read from the graph
/// "Green Pipe" of the ATREA element catalogue A2328 26/06, p. 53
/// (m³/h, Pa; taken as per metre of duct, the graph gives no length).
const GP_LOSS: [(f64, f64); 11] = [
    (0.0, 0.0),
    (5.0, 0.20),
    (10.0, 0.42),
    (15.0, 0.68),
    (20.0, 0.99),
    (25.0, 1.42),
    (30.0, 2.02),
    (35.0, 2.74),
    (40.0, 3.57),
    (45.0, 4.41),
    (50.0, 5.17),
];

/// Velocity and pressure loss per metre of an ATREA GP duct at `q` m³/h,
/// from the maker's graph (linear between its points; beyond 50 m³/h,
/// which ATREA does not allow, the last step goes on).
fn gp_friction(q: f64) -> (f64, f64) {
    let a = std::f64::consts::PI * 0.075 * 0.075 / 4.0;
    let v = q / 3600.0 / a;
    let k = GP_LOSS
        .windows(2)
        .position(|w| q <= w[1].0)
        .unwrap_or(GP_LOSS.len() - 2);
    let (p, n) = (GP_LOSS[k], GP_LOSS[k + 1]);
    (v, p.1 + (n.1 - p.1) * (q - p.0) / (n.0 - p.0))
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

/// A number with one decimal and a decimal comma, for Slovak text.
fn comma(v: f64) -> String {
    format!("{v:.1}").replace('.', ",")
}

fn ceil_to(v: f64, step: f64) -> f64 {
    (v / step).ceil() * step
}

/// Share `total` among rooms: each room first gets its need rounded up
/// to a whole `step`, then the steps left to reach `total` go to the
/// rooms in proportion to their needs (largest remainders first), so
/// every room gets at least its need and the shares sum to `total`.
fn apportion(needs: &[f64], total: f64, step: f64) -> Vec<f64> {
    let base: Vec<i64> = needs
        .iter()
        .map(|n| (n / step - 1e-9).ceil().max(0.0) as i64)
        .collect();
    let sum: f64 = needs.iter().sum();
    let mut left = (total / step).round() as i64 - base.iter().sum::<i64>();
    let mut got = base.clone();
    if left > 0 && sum > 0.0 {
        let extra = left as f64;
        let exact: Vec<f64> = needs.iter().map(|n| n / sum * extra).collect();
        for (k, e) in exact.iter().enumerate() {
            got[k] += e.floor() as i64;
        }
        left -= exact.iter().map(|e| e.floor() as i64).sum::<i64>();
        let mut order: Vec<usize> = (0..needs.len()).collect();
        order.sort_by(|&a, &b| {
            (exact[b] - exact[b].floor())
                .total_cmp(&(exact[a] - exact[a].floor()))
                .then(needs[b].total_cmp(&needs[a]))
        });
        for &k in order.iter().take(left.max(0) as usize) {
            got[k] += 1;
        }
    }
    got.into_iter().map(|g| g as f64 * step).collect()
}

/// Decree 259/2008 Z. z., annex 2, table 4: the least air changes of
/// short-stay rooms, 1/h (the lower end where it gives a range). Used
/// for buildings other than dwellings; a dwelling's rooms are not in it.
pub fn table4(k: RoomType) -> Option<f64> {
    match k {
        RoomType::Hall | RoomType::Staircase => Some(2.0),
        RoomType::Bathroom => Some(10.0),
        RoomType::Toilet => Some(5.0),
        RoomType::Pantry => Some(10.0),
        RoomType::Wardrobe => Some(10.0),
        _ => None,
    }
}

/// Is the building for dwelling (its use in the project information
/// says so, or says nothing)?
pub fn is_residential(model: &Model) -> bool {
    let u = model
        .project_info()
        .map(|(_, p)| p.building_use.to_lowercase())
        .unwrap_or_default();
    u.trim().is_empty()
        || [
            "dwelling",
            "residential",
            "family house",
            "bytov",
            "byt ",
            "obyt",
            "rodinn",
        ]
        .iter()
        .any(|w| format!("{u} ").contains(w))
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
    let residential = is_residential(model);
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
        // a building other than dwellings: the decree's least air
        // changes for the rooms its table lists
        let n4 = if residential { None } else { table4(hl.kind) };
        need.push(match n4 {
            Some(n) => hl.v_min.max(n * hl.volume).round(),
            None => hl.v_min.round(),
        });
    }
    if geo.is_empty() {
        d.warnings.push("Miestnosti podlažia nemajú obrys.".into());
        return d;
    }
    let ds = doors(model, level, &geo);
    for g in &geo {
        if g.height > 3.0 + 1e-6 && g.kind.needs().ach.unwrap_or(0.0) > 0.0 {
            d.warnings.push(format!(
                "{} {}: svetlá výška {:.2} m je nad 3 m, pre ktoré STN EN 12831-1 tab. B.7 najmenšiu výmenu vzduchu neodporúča: prietok overiť inak.",
                g.number, g.name, g.height
            ));
        }
    }

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

    // the balance: the smaller side raised to the larger, both sides
    // summing to the same flow in steps of 5 m³/h
    let base = |role| -> f64 {
        d.rooms
            .iter()
            .filter(|r| r.role == role)
            .map(|r| ceil_to(r.need - 1e-9, 5.0).max(0.0))
            .sum()
    };
    let q = base(Role::Supply).max(base(Role::Extract));
    for role in [Role::Supply, Role::Extract] {
        let idx: Vec<usize> = (0..d.rooms.len())
            .filter(|&i| d.rooms[i].role == role)
            .collect();
        let needs: Vec<f64> = idx.iter().map(|&i| d.rooms[i].need).collect();
        for (k, v) in apportion(&needs, q, 5.0).into_iter().enumerate() {
            d.rooms[idx[k]].design = v;
        }
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
                        (a, b, outward(&g.outline, a, b), th)
                    }
                    None => {
                        d.warnings.push(format!(
                            "Miestnosť {} nemá obvodovú stenu: nasávanie a výfuk idú zvislo cez strop a strechu, doplniť strešné prestupy a hlavice.",
                            g.name
                        ));
                        d.through_roof = Some(g.height + 0.6);
                        let (a, b) = (g.outline[0], g.outline[1 % g.outline.len()]);
                        (a, b, outward(&g.outline, a, b), 0.4)
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
                d.box_along = along;
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
        // ATREA: a plate valve takes one GP run (transition chamber PKS
        // Ø100/Ø100 GP, valve Ø100) or two (PKS 2×Ø100/Ø125 GP, valve
        // Ø125), each run by the GP connection set
        let per_valve = o.runs_per_valve.clamp(1, 2);
        let terms = ducts.div_ceil(per_valve) as usize;
        r.ducts = ducts;
        let places = terminal_places(&geo[i], &door_pts(i), terms);
        let mut left = ducts;
        for p in places {
            let k = left.min(per_valve);
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
        // every GP run: its room, terminal, and the doors back to the
        // unit's room
        struct Run {
            room: usize,
            terminal: usize,
            /// Which of the terminal's runs (its PKS spigot).
            dn: u32,
            chain: Vec<usize>,
        }
        let mut runs: Vec<Run> = vec![];
        for (ri, r) in d.rooms.iter().enumerate() {
            if !matches!(r.role, Role::Supply | Role::Extract) {
                continue;
            }
            if !seen[ri] {
                d.warnings.push(format!(
                    "{} {}: nevedie k nej žiadne dvere z miestnosti jednotky, potrubie treba viesť ručne.",
                    r.number, r.name
                ));
                continue;
            }
            let mut chain = vec![];
            let mut at = ri;
            while let Some((p, k)) = prev[at] {
                chain.push(k);
                at = p;
            }
            for (ti, t) in r.terminals.iter().enumerate() {
                for dn in 0..t.ducts {
                    runs.push(Run {
                        room: ri,
                        terminal: ti,
                        dn,
                        chain: chain.clone(),
                    });
                }
            }
        }
        // too many runs for the door into the unit's room: the manifolds
        // go just past that door, the unit's Spiro mains through it
        let mut via_door: std::collections::HashMap<usize, usize> = Default::default();
        for r in &runs {
            if let Some(&k) = r.chain.last() {
                *via_door.entry(k).or_default() += 1;
            }
        }
        if let Some((&k0, &n0)) = via_door.iter().max_by_key(|(_, n)| **n) {
            let dr = &ds[k0];
            if !dr.open && n0 as f64 * 0.095 > dr.width - 0.2 {
                let into = if dr.a == Some(ui) { dr.b } else { dr.a };
                if let Some(x) = into {
                    // the side of the door the room past it is on
                    let side = if room_at(&geo, dr.mid + dr.n * (dr.thick / 2.0 + 0.15)) == Some(x)
                    {
                        1.0
                    } else {
                        -1.0
                    };
                    let out = dr.n * side;
                    let along = dr.n.perp();
                    let line = dr.mid + out * (dr.thick / 2.0 + 0.5);
                    let count = |sup: bool| {
                        runs.iter()
                            .filter(|r| (d.rooms[r.room].role == Role::Supply) == sup)
                            .count() as f64
                    };
                    let (ns, ne) = (count(true), count(false));
                    // supply on one side of the door, extract on the other
                    let s = line - along * (0.25 + ns * 0.05);
                    let e = line + along * (0.25 + ne * 0.05);
                    if creabim_core::polygon::contains(&geo[x].outline, s)
                        && creabim_core::polygon::contains(&geo[x].outline, e)
                    {
                        d.boxes = [(s, turn_to(out)), (e, turn_to(out))];
                        d.box_along = along;
                        d.main_via = vec![
                            dr.mid - out * (dr.thick / 2.0 + 0.3),
                            dr.mid + out * (dr.thick / 2.0 + 0.3),
                        ];
                        for r in runs.iter_mut() {
                            // the rest reach the unit's room some other
                            // way (a partition) and leave it by that door
                            if r.chain.last() == Some(&k0) {
                                r.chain.pop();
                            } else {
                                r.chain.push(k0);
                            }
                        }
                        d.warnings.push(format!(
                            "Dverami {:.2} m do miestnosti jednotky by išlo {} potrubí GP: rozdeľovače sú za dverami, k jednotke vedú potrubia Spiro.",
                            dr.width, n0
                        ));
                    }
                }
            }
        }
        // each door's runs side by side, 0.1 m apart (GP is Ø90), packed
        // closer where the door is too narrow, which is warned
        let mut lane: Vec<std::collections::HashMap<usize, f64>> =
            vec![Default::default(); runs.len()];
        for (k, dr) in ds.iter().enumerate() {
            let through: Vec<usize> = (0..runs.len())
                .filter(|&j| runs[j].chain.contains(&k))
                .collect();
            let n = through.len();
            if n == 0 {
                continue;
            }
            let room = dr.width - 0.2;
            let step = if n > 1 {
                (room / (n - 1) as f64).min(0.1)
            } else {
                0.0
            };
            if dr.partition {
                d.warnings.push(format!(
                    "Prestup cez priečku pre {} potrubí GP Ø90 (šírka zväzku {:.2} m): zakresliť prestup v priečke.",
                    n,
                    (n as f64 * 0.1).min(dr.width + 0.1)
                ));
            }
            if !dr.open && n as f64 * 0.095 > dr.width {
                d.warnings.push(format!(
                    "Dverami {:.2} m ide {} potrubí GP Ø90, vedľa seba sa zmestí {}: viesť časť pod priečkou, alebo zvýšiť prietok na potrubie.",
                    dr.width,
                    n,
                    (dr.width / 0.095).floor()
                ));
            }
            for (pos, &j) in through.iter().enumerate() {
                lane[j].insert(k, (pos as f64 - (n as f64 - 1.0) / 2.0) * step);
            }
        }
        // the runs on each side line up along the manifold, 0.1 m apart
        let mut at_box = [0usize; 2];
        let side_count = |sup: bool| {
            runs.iter()
                .filter(|x| (d.rooms[x.room].role == Role::Supply) == sup)
                .count()
        };
        let counts = [side_count(true), side_count(false)];
        for (j, run) in runs.iter().enumerate() {
            let ri = run.room;
            let ti = run.terminal;
            let t = d.rooms[ri].terminals[ti].clone();
            let supply = d.rooms[ri].role == Role::Supply;
            let s = if supply { 0 } else { 1 };
            let target = d.boxes[s].0
                + d.box_along * ((at_box[s] as f64 - (counts[s] as f64 - 1.0) / 2.0) * 0.1);
            at_box[s] += 1;
            {
                {
                    // the PKS's spigots 0.1 m apart
                    let spig = (run.dn as f64 - (t.ducts as f64 - 1.0) / 2.0) * 0.1;
                    let mut pts = vec![t.at + DVec2::new(spig, spig)];
                    let mut room = ri;
                    for &k in &run.chain {
                        let dr = &ds[k];
                        let side = if room_at(&geo, dr.mid + dr.n * (dr.thick / 2.0 + 0.15))
                            == Some(room)
                        {
                            1.0
                        } else {
                            -1.0
                        };
                        let along = dr.n.perp();
                        let lane = along * lane[j].get(&k).copied().unwrap_or(0.0);
                        let here = dr.mid + lane + dr.n * side * (dr.thick / 2.0 + 0.3);
                        let there = dr.mid + lane - dr.n * side * (dr.thick / 2.0 + 0.3);
                        let last = *pts.last().unwrap();
                        if let Some(c) = ell_from(&pts, here, Some(&geo[room].outline)) {
                            pts.push(c);
                        }
                        let _ = last;
                        pts.push(here);
                        pts.push(there);
                        room = room_at(&geo, there).unwrap_or(room);
                    }
                    let end = target;
                    if let Some(c) = ell_from(&pts, end, Some(&geo[room].outline)) {
                        pts.push(c);
                    }
                    pts.push(end);
                    pts.dedup_by(|a, b| a.distance(*b) < 0.02);
                    let pts = straighten(pts);
                    let length = path_length(&pts) + 2.0 * 0.3;
                    let flow = t.flow / t.ducts as f64;
                    let (v, rpm) = gp_friction(flow);
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
            // the mains as drawn by `apply`: unit to its box (along, then
            // up or down) and through the outer wall, Spiro of the
            // unit's spigot; each fan pushes through both of its side's
            // mains (supply: outdoor air + supply, extract: extract + exhaust)
            let main_q = d.flow;
            let u = d.unit.map(|i| &cat.units[i]);
            let dm = u.map_or(0.16, |u| u.spigot / 1000.0);
            let uh = u.map_or(z, |u| if u.on_ceiling() { z } else { u.size[2] + 0.15 });
            let (vm, rpm) = friction(main_q, dm, 1.5e-4);
            let main_len = match d.unit_at {
                Some((at, _)) => {
                    // square in plan: through the door when past it
                    let mut stops = vec![at];
                    stops.extend(d.main_via.iter().copied());
                    stops.push(d.boxes[if supply { 0 } else { 1 }].0);
                    let to_box = stops
                        .windows(2)
                        .map(|w| (w[1] - w[0]).abs().element_sum())
                        .sum::<f64>()
                        + (uh - z).abs();
                    let out = match d.through_roof {
                        Some(top) => top - uh,
                        None => d.outside.first().map_or(0.5, |o| at.distance(*o)),
                    };
                    to_box + out
                }
                None => 3.0,
            };
            // ζ: two bends on the way to the box, one through the wall
            let main = rpm * main_len + o.zeta_bend * 3.0 * 1.2 * vm * vm / 2.0;
            d.main_velocity = vm;
            d.main_length[if supply { 0 } else { 1 }] = main_len;
            if supply {
                d.dp_supply = worst + main;
            } else {
                d.dp_extract = worst + main;
            }
        }
        if let Some(u) = d.unit.map(|i| &cat.units[i]) {
            let worst = d.dp_supply.max(d.dp_extract);
            if d.flow <= u.q_max && worst <= UNIT_PRESSURE_PA {
                d.warnings.push(format!(
                    "Jednotka {} dáva {:.0} m³/h pri {:.0} Pa (katalóg): pri {:.0} m³/h má rezervu aspoň {:.0} Pa na straty tanierových ventilov, odbočiek, tlmičov a žalúzií.",
                    u.name, u.q_max, UNIT_PRESSURE_PA, d.flow, UNIT_PRESSURE_PA - worst
                ));
            } else {
                d.warnings.push(format!(
                    "Jednotka {}: {:.0} m³/h a {:.0} Pa sú mimo katalógového bodu {:.0} m³/h pri {:.0} Pa, overiť podľa krivky ventilátora.",
                    u.name, d.flow, worst, u.q_max, UNIT_PRESSURE_PA
                ));
            }
        }
        d.warnings.push(
            format!("Tlakové straty: potrubie GP podľa grafu ATREA, ohyby GP ζ {} na ohyb (predpoklad: ATREA hodnotu neudáva), hlavné rozvody Spiro trením (Darcy-Weisbach, λ podľa Swameeho-Jaina) Ø{:.0} (prívodná strana {} m, odvodná {} m, {} m/s). Straty tanierových ventilov, odbočiek, tlmičov a žalúzie katalóg ATREA neudáva: doplniť od výrobcu.",
                comma(o.zeta_bend),
                d.unit.map_or(160.0, |i| cat.units[i].spigot),
                comma(d.main_length[0]),
                comma(d.main_length[1]),
                comma(d.main_velocity)),
        );
    }

    // the transfer: under the doors of the rooms air is blown into (it
    // leaves them) and of the rooms it is taken from (it comes in); a
    // door both sides use carries the larger flow
    let mut by_door: Vec<(ElementId, f64, f64)> = vec![];
    for (i, r) in d.rooms.iter().enumerate() {
        if !matches!(r.role, Role::Supply | Role::Extract) {
            continue;
        }
        let mine: Vec<&Door> = ds
            .iter()
            .filter(|x| {
                !x.open
                    && !x.partition
                    && x.a.is_some()
                    && x.b.is_some()
                    && (x.a == Some(i) || x.b == Some(i))
            })
            .collect();
        if mine.is_empty() {
            continue;
        }
        let share = r.design / mine.len() as f64;
        for dr in mine {
            match by_door.iter_mut().find(|x| x.0 == dr.id) {
                Some(x) => x.1 = x.1.max(share),
                None => by_door.push((dr.id, share, dr.width)),
            }
        }
    }
    for (door, flow, width) in by_door {
        let gap = flow / 3600.0 / o.transfer_v / width.max(0.6) * 1000.0;
        d.transfers.push(Transfer {
            door,
            flow: flow.round(),
            gap_mm: gap.ceil(),
        });
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
            let n = outward(&geo[i].outline, a, b);
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
    // noise in the protected rooms: the ventilation runs at night too,
    // so the night limit governs
    let ru = cat.room_units.get(o.room_unit);
    let unit = d.unit.map(|i| &cat.units[i]);
    for (i, r) in d.rooms.iter().enumerate() {
        let Some(c) = noise_category(geo[i].kind) else {
            continue;
        };
        let limits = if c == 'A' { NOISE_A } else { NOISE_B };
        let (level, note) = match r.role {
            Role::Decentral => match ru.and_then(|u| u.noise) {
                Some(l) => (
                    Some(l),
                    "údaj výrobcu pri najvyššom výkone".to_string(),
                ),
                None => (None, "chýba hladina hluku jednotky od výrobcu".to_string()),
            },
            Role::Supply | Role::Extract => match unit.and_then(|u| u.lw) {
                Some(_) => (None, "akustický výpočet rozvodu: doplniť útlm tlmiča a ventilu z katalógu".to_string()),
                None => (
                    None,
                    "technický list ATREA udáva len akustický výkon do okolia; Lw do potrubia, útlm tlmiča a ventilu chýba (návrhový program ATREA)".to_string(),
                ),
            },
            _ => continue,
        };
        d.noise.push(Noise {
            room: i,
            category: c,
            limits,
            level,
            ok: level.map(|l| l <= limits[2] + 1e-9),
            note,
        });
    }
    if d.flow > 0.0 {
        d.warnings.push(
            "Rozvod podľa ATREA Katalógu prvkov A2328 26/06: potrubie GP Ø90/75 (najviac 50 m³/h na trasu) sa napája spojovacou sadou R316303 na tvarovky DN100 označené GP: na rozdeľovači odbočky OBJ-T 90° z hrdla jednotky na Ø100, v miestnosti tanierový ventil KI/KO Ø100. Strata potrubia GP podľa grafu výrobcu (str. 53, berie sa na 1 m).".into(),
        );
    }
    d.warnings.push(if residential {
        "Bytová budova: 0,5 1/h podľa STN EN 12831-1 tab. B.7, odvod kuchyňa 100, kúpeľňa 90, WC 50 m³/h (zaužívané hodnoty, vyhláška 259/2008 ich pre byty neuvádza).".into()
    } else {
        "Nebytová budova: najmenšia výmena vzduchu podľa vyhlášky 259/2008 Z. z., príl. 2, tab. 4 (chodby, schodiská 2, kúpeľne 10, WC 5, sklady a šatne 10 1/h; dolná hranica rozsahu).".into()
    });
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
            penetration: None,
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
        outer_mm: 90.0,
        inner_mm: 75.0,
        height_mm: None,
        insulation_mm: 0.0,
        insulation_set: false,
        mark: None,
    }
}

fn spiro(level: ElementId, system: &str, a: DVec3, b: DVec3, insulation: f64, dn: f64) -> Pipe {
    Pipe {
        level,
        system: system.into(),
        start: a,
        end: b,
        pipe_type: "VZK".into(),
        nominal_mm: dn,
        outer_mm: dn + 1.0,
        inner_mm: dn,
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
            u.on_ceiling(),
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
        // the manifolds: a Spiro of the unit's spigot along the wall, the
        // GP runs branching off it (OBJ-T) where they end, so every run is
        // one joined route from the unit to its valve
        let mut near = [d.boxes[0].0, d.boxes[1].0];
        for (k, sys) in [(0usize, "VZP"), (1, "VZO")] {
            let along = if d.box_along.length() > 0.5 {
                d.box_along
            } else {
                DVec2::X
            };
            let mut ends: Vec<DVec2> = d
                .branches
                .iter()
                .filter(|b| b.supply == (k == 0))
                .filter_map(|b| b.path.last().copied())
                .collect();
            if ends.is_empty() {
                continue;
            }
            ends.sort_by(|p, q| p.dot(along).total_cmp(&q.dot(along)));
            ends.dedup_by(|p, q| p.distance(*q) < 0.02);
            let first = ends[0] - along * 0.15;
            let last = ends[ends.len() - 1] + along * 0.15;
            near[k] = if first.distance(at) <= last.distance(at) {
                first
            } else {
                last
            };
            let mut line = vec![first];
            line.extend(ends.iter().copied());
            line.push(last);
            for w in line.windows(2) {
                tx = tx.insert(Element::new(
                    format!("{TAG} – {sys} rozdeľovač"),
                    ElementKind::Pipe(spiro(
                        level,
                        sys,
                        w[0].extend(z),
                        w[1].extend(z),
                        0.0,
                        u.spigot,
                    )),
                ));
            }
        }
        // mains: unit to the manifolds (with a silencer), outdoor air and
        // exhaust
        let uh = if u.on_ceiling() { z } else { u.size[2] + 0.15 };
        let n = DVec2::from_angle((rot + 90.0).to_radians());
        let out = d.outside.first().copied().unwrap_or(at + n * 0.5);
        for (sys, to, sil) in [
            ("VZP", Some(near[0]), true),
            ("VZO", Some(near[1]), true),
            ("VZC", None, false),
            ("VZE", None, false),
        ] {
            let (a, b, ins) = match to {
                Some(p) => (at.extend(uh), p.extend(z), 0.0),
                None if d.through_roof.is_some() => {
                    let side = if sys == "VZC" { -0.2 } else { 0.2 };
                    let s = at + n.perp() * side;
                    let top = d.through_roof.unwrap_or(3.0);
                    (s.extend(uh), s.extend(top), 0.03)
                }
                None => {
                    let side = if sys == "VZC" { -0.2 } else { 0.2 };
                    let s = at + n.perp() * side;
                    // its core hole through the outer wall (Ø160 with
                    // 30 mm of insulation: 250 mm)
                    if let Some(h) = d
                        .unit_wall
                        .and_then(|w| hole(model, w, s, uh, u.spigot / 1000.0 + 0.09))
                    {
                        tx = tx.insert(h);
                    }
                    (s.extend(uh), (out + n.perp() * side).extend(uh), 0.03)
                }
            };
            // at the unit's height, square in plan (through the door when
            // the manifolds are past it), then down to the manifold
            let mut plan = vec![a.truncate()];
            if to.is_some() {
                for &v in &d.main_via {
                    if let Some(c) = ell_from(&plan, v, None) {
                        plan.push(c);
                    }
                    plan.push(v);
                }
            }
            if let Some(c) = ell_from(&plan, b.truncate(), None) {
                plan.push(c);
            }
            plan.push(b.truncate());
            plan.dedup_by(|p, q| p.distance(*q) < 0.01);
            let plan = straighten(plan);
            let mut pid = None;
            for w in plan.windows(2) {
                let pe = Element::new(
                    format!("{TAG} – {sys}"),
                    ElementKind::Pipe(spiro(
                        level,
                        sys,
                        w[0].extend(a.z),
                        w[1].extend(a.z),
                        ins,
                        u.spigot,
                    )),
                );
                pid.get_or_insert(pe.id);
                tx = tx.insert(pe);
            }
            let end = plan.last().copied().unwrap_or(b.truncate()).extend(a.z);
            if (end.z - b.z).abs() > 0.01 {
                let pe = Element::new(
                    format!("{TAG} – {sys}"),
                    ElementKind::Pipe(spiro(level, sys, end, b, ins, u.spigot)),
                );
                pid.get_or_insert(pe.id);
                tx = tx.insert(pe);
            }
            let Some(pid) = pid else { continue };
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
    // terminals: a plate valve on one GP run (PKS Ø100/Ø100 GP, valve
    // Ø100) or two (PKS 2×Ø100/Ø125 GP, valve Ø125); every run joined at
    // both ends by the GP connection set and leaving the manifold by a
    // branch OBJ-T
    let (mut sil, mut runs) = (0.0, 0.0);
    let mut v = [[0.0; 2]; 2]; // [supply, extract][one run, two]
    for e in model.iter() {
        let ElementKind::Valve(x) = &e.kind else {
            continue;
        };
        let Some(ElementKind::Pipe(p)) = model.get(x.pipe).map(|y| &y.kind) else {
            continue;
        };
        match x.kind.as_str() {
            "vyustP" | "vyustO" if p.pipe_type == "GP" => {
                let end = if x.t < 0.5 { p.start } else { p.end }.truncate();
                let n = gp_ends.iter().filter(|g| g.distance(end) < 0.2).count();
                let two = usize::from(n >= 2);
                let side = usize::from(x.kind == "vyustO");
                v[side][two] += 1.0;
                runs += if n >= 2 { 2.0 } else { 1.0 };
            }
            "tlmic" => sil += 1.0,
            _ => {}
        }
    }
    let manifolds = model
        .iter()
        .filter(|e| e.name.starts_with(TAG) && e.name.ends_with("rozdeľovač"))
        .filter_map(|e| match &e.kind {
            ElementKind::Pipe(p) => Some(p.system.clone()),
            _ => None,
        })
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let spigot = units
        .first()
        .and_then(|(_, c)| cat.units.iter().find(|u| u.code == *c))
        .map_or(160.0, |u| u.spigot);
    let branch = match spigot.round() as i64 {
        125 => "odbocka125",
        200 => "odbocka200",
        _ => "odbocka160",
    };
    lines.extend(line("Rozvody", cat.item(branch), runs));
    lines.extend(line("Rozvody", cat.item("kryt"), 2.0 * manifolds as f64));
    lines.extend(line("Rozvody", cat.item("sadaGP"), 2.0 * runs));
    lines.extend(line("Koncové prvky", cat.item("pks1"), v[0][0] + v[1][0]));
    lines.extend(line("Koncové prvky", cat.item("pks2"), v[0][1] + v[1][1]));
    lines.extend(line("Koncové prvky", cat.item("ventilKI"), v[0][0]));
    lines.extend(line("Koncové prvky", cat.item("ventilKI125"), v[0][1]));
    lines.extend(line("Koncové prvky", cat.item("ventilKO"), v[1][0]));
    lines.extend(line("Koncové prvky", cat.item("ventilKO125"), v[1][1]));
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
    fn shares_meet_every_need_and_sum_to_the_total() {
        // 30, 12, 38 m³/h of supply raised to 150 of extract
        let v = apportion(&[30.0, 12.0, 38.0], 150.0, 5.0);
        assert_eq!(v.iter().sum::<f64>(), 150.0);
        assert!(v[0] >= 30.0 && v[1] >= 12.0 && v[2] >= 38.0);
        assert_eq!(v, vec![55.0, 25.0, 70.0]);
        // the larger side: each need rounded up, nothing more
        assert_eq!(apportion(&[100.0, 50.0], 150.0, 5.0), vec![100.0, 50.0]);
        assert_eq!(apportion(&[52.0, 90.0], 145.0, 5.0), vec![55.0, 90.0]);
    }

    #[test]
    fn the_catalogue_reads() {
        let c = catalogue();
        assert!(c.units.len() >= 3 && !c.room_units.is_empty());
        assert!(c.item("gp75").is_some() && c.item("sadaGP").is_some());
        assert!(crate::pipe_types::by_id("GP").is_some());
    }

    #[test]
    fn friction_of_a_gp_duct_is_small() {
        let (v, r) = friction(25.0, 0.075, 5e-5);
        assert!((v - 1.57).abs() < 0.01, "{v}");
        assert!(r > 0.3 && r < 1.0, "{r}");
    }

    #[test]
    fn gp_loss_follows_the_makers_graph() {
        // ATREA catalogue A2328 26/06, p. 53: 1.42 Pa at 25, 5.17 at 50 m³/h
        let (v, r) = gp_friction(25.0);
        assert!((v - 1.57).abs() < 0.01 && (r - 1.42).abs() < 1e-9);
        assert!((gp_friction(50.0).1 - 5.17).abs() < 1e-9);
        assert!((gp_friction(27.5).1 - 1.72).abs() < 1e-9);
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
            // balanced exactly: both sides carry the unit's flow
            assert_eq!(s, e);
            assert_eq!(s, d.flow);
            for r in d
                .rooms
                .iter()
                .filter(|r| matches!(r.role, Role::Supply | Role::Extract))
            {
                assert!(
                    r.design + 1e-9 >= r.need,
                    "{} gets less than it needs",
                    r.name
                );
                assert_eq!(r.design % 5.0, 0.0);
            }
            // every branch within ATREA's 50 m³/h per GP run
            assert!(d.branches.iter().all(|b| b.flow <= 50.0 + 1e-9));
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
        let n = d
            .noise
            .iter()
            .find(|n| d.rooms[n.room].name.contains("Study"))
            .unwrap();
        assert_eq!(n.limits, NOISE_B);
        assert_eq!(n.ok, Some(true));
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

    #[test]
    fn other_buildings_take_the_decrees_table_4() {
        assert_eq!(table4(RoomType::Bathroom), Some(10.0));
        assert_eq!(table4(RoomType::Living), None);
        let mut model = creabim_core::starter::starter_house();
        assert!(is_residential(&model));
        let o = Opts::default();
        let ho = heatloss::Opts::default();
        let home = design(&model, &ho, &o);
        let (id, mut p) = model
            .project_info()
            .map(|(e, p)| (e, p.clone()))
            .unwrap_or_default();
        p.building_use = "Office".into();
        let mut doc = creabim_core::Document::from_model(model.clone());
        if let Some(before) = doc.model().get(id).cloned() {
            let mut after = before.clone();
            if let ElementKind::Project(x) = &mut after.kind {
                *x = p;
            }
            doc.commit(Transaction::new("use").replace(before, after))
                .unwrap();
            model = doc.model().clone();
            assert!(!is_residential(&model));
            let office = design(&model, &ho, &o);
            let hall = |d: &Design| {
                d.rooms
                    .iter()
                    .find(|r| r.name.contains("Hall"))
                    .unwrap()
                    .need
            };
            assert!(hall(&office) > hall(&home));
        }
    }
}
