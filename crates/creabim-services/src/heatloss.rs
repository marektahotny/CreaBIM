//! The design heat loss of the rooms, simplified after STN EN 12831-1,
//! as PilonDesigner's `heatloss.ts`: by transmission through outside
//! walls, windows and doors, the floor on the ground and the roof
//! (U with a surcharge ΔU for thermal bridges), into colder rooms next
//! door, and by ventilation 0.34 · V̇ · Δθ, where V̇ is the larger of the
//! infiltration and the least air change: V̇inf = 2 · V · n50 · e · ε
//! (n50 by the envelope's airtightness, e by the façades with openings,
//! ε by the height above the ground: STN EN 12831 tables D.5, D.8, D.9).
//!
//! A wall is an outside one where no room lies on its other side. U of
//! a build-up comes from its layers' thermal conductivities; where one
//! is missing the loss is left out and the room says what it lacks.

use crate::groundloss::{fg2, u_equiv_floor, FG1};
use creabim_core::assembly::assembly_of;
use creabim_core::room::{self, RoomShape};
use creabim_core::roomtype::RoomType;
use creabim_core::{polygon, Element, ElementId, ElementKind, Model, OpeningKind};
use glam::DVec2;

/// How sheltered the building stands (STN EN 12831 table D.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shielding {
    /// In the wind; tall buildings in towns.
    None,
    /// In the country with trees, suburbs.
    Moderate,
    /// In a town centre, in a forest.
    Heavy,
}

impl Shielding {
    pub const ALL: [Shielding; 3] = [Shielding::None, Shielding::Moderate, Shielding::Heavy];
    pub fn name(self) -> &'static str {
        match self {
            Shielding::None => "No shielding (in the wind, tall buildings in towns)",
            Shielding::Moderate => "Moderate shielding (country with trees, suburbs)",
            Shielding::Heavy => "Heavy shielding (town centre, forest)",
        }
    }
}

/// The shielding coefficient e for the façades with openings a room has.
pub fn shield_e(facades: usize, s: Shielding) -> f64 {
    let k = match s {
        Shielding::None => 0,
        Shielding::Moderate => 1,
        Shielding::Heavy => 2,
    };
    match facades {
        0 => 0.0,
        1 => [0.03, 0.02, 0.01][k],
        _ => [0.05, 0.03, 0.02][k],
    }
}

/// The height correction ε for the middle of a room `z` m above ground.
pub fn height_eps(z: f64) -> f64 {
    if z <= 10.0 {
        1.0
    } else if z <= 30.0 {
        1.2
    } else {
        1.5
    }
}

/// What the calculation is given.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opts {
    /// Design outside temperature θe, °C.
    pub te: f64,
    /// Mean annual outside temperature θm,e, °C; none takes f_g2 = 1.
    pub te_mean: Option<f64>,
    /// Mean outside temperature of the heating season, °C, and its
    /// days: the locality's, for the season's degree days.
    pub te_heating: Option<f64>,
    pub heating_days: Option<f64>,
    /// The surcharge for thermal bridges ΔU, W/(m²K).
    pub d_u: f64,
    pub u_window: f64,
    pub u_door: f64,
    /// Air changes at 50 Pa, 1/h.
    pub n50: f64,
    pub shielding: Shielding,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            te: -11.0,
            te_mean: None,
            te_heating: None,
            heating_days: None,
            d_u: 0.05,
            u_window: 0.6,
            u_door: 0.65,
            n50: 2.0,
            shielding: Shielding::Moderate,
        }
    }
}

impl Opts {
    /// The heating season's degree days for an inside 20 °C, where the
    /// locality gives them.
    pub fn degree_days(&self) -> Option<f64> {
        Some(self.heating_days? * (20.0 - self.te_heating?))
    }

    /// A rough yearly heat need, kWh, of a design load `phi` W: the load
    /// scaled by the degree days (no night setback, no gains).
    pub fn yearly_kwh(&self, phi: f64) -> Option<f64> {
        let dt = 20.0 - self.te;
        (dt > 0.0).then_some(phi * 24.0 * self.degree_days()? / dt / 1000.0)
    }
}

/// What a room is heated to and aired by, from what it is used for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoomUse {
    /// θint, °C.
    pub temp: f64,
    /// Least air changes, 1/h.
    pub ach: Option<f64>,
    /// Least air flow, m³/h.
    pub flow: Option<f64>,
}

/// The use of a room by a text naming it (its occupancy, its name):
/// the values [`RoomType`]'s table gives its type.
pub fn room_use(text: &str) -> RoomUse {
    RoomType::find(text).unwrap_or(RoomType::Other).into()
}

impl From<RoomType> for RoomUse {
    fn from(t: RoomType) -> Self {
        let n = t.needs();
        RoomUse {
            temp: n.temp,
            ach: n.ach,
            flow: n.flow,
        }
    }
}

/// One way heat leaves a room.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// What it is, in English: the app translates it.
    pub kind: &'static str,
    /// The build-up's name, or the room next door.
    pub detail: String,
    pub area: f64,
    pub u: f64,
    /// The temperature factor (1 to the outside).
    pub f: f64,
    /// W.
    pub phi: f64,
    /// Where an outside wall, window or door faces: degrees clockwise
    /// from true north.
    pub azimuth: Option<f64>,
}

/// What a room lacks for a full answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Missing {
    WallBuildUp,
    InnerWallBuildUp,
    FloorBuildUp,
    RoofBuildUp,
    NoRoofOrCeiling,
}

impl Missing {
    pub fn text(self) -> &'static str {
        match self {
            Missing::WallBuildUp => {
                "an outside wall with no build-up or a layer with no λ: its loss is left out"
            }
            Missing::InnerWallBuildUp => {
                "a wall to a colder room with no build-up: its loss is left out"
            }
            Missing::FloorBuildUp => {
                "the floor on the ground with no slab build-up: its loss is left out"
            }
            Missing::RoofBuildUp => "the roof with no build-up: its loss is left out",
            Missing::NoRoofOrCeiling => {
                "nothing over the room (no room, no roof): the loss upwards is left out"
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RoomLoss {
    pub id: ElementId,
    /// What it is used for, from its occupancy and name: the cooling
    /// load and underfloor heating take it from here.
    pub kind: RoomType,
    pub number: String,
    pub name: String,
    pub level: String,
    pub ti: f64,
    pub area: f64,
    pub volume: f64,
    pub items: Vec<Item>,
    /// Transmission, ventilation and total, W.
    pub phi_t: f64,
    pub phi_v: f64,
    pub phi: f64,
    /// Least air change and infiltration, m³/h.
    pub v_min: f64,
    pub v_inf: f64,
    /// Outside walls with openings, by direction.
    pub facades: usize,
    pub missing: Vec<Missing>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuildingLoss {
    pub rooms: Vec<RoomLoss>,
    /// W.
    pub phi: f64,
    pub area: f64,
}

/// Surface resistances, m²K/W (STN EN ISO 6946).
const R_WALL: f64 = 0.13 + 0.04;
const R_ROOF: f64 = 0.10 + 0.04;
const R_FLOOR: f64 = 0.17;
const R_INNER: f64 = 0.13 + 0.13;

/// U of the build-up (or the one material) element `e` is made of,
/// W/(m²K), and its name: none where it has neither or a layer has no λ.
fn u_of(model: &Model, e: &Element, r_surfaces: f64) -> Option<(f64, String)> {
    let Some((ae, asm)) = assembly_of(model, e) else {
        // of one material through and through
        let (material, d) = match &e.kind {
            ElementKind::Wall(w) => (w.material, w.thickness),
            ElementKind::Slab(s) => (s.material, s.thickness),
            ElementKind::Roof(r) => (r.material, r.thickness),
            _ => return None,
        };
        let me = model.get(material?)?;
        let ElementKind::Material(mat) = &me.kind else {
            return None;
        };
        let r = r_surfaces + d / mat.lambda.filter(|x| *x > 0.0)?;
        return Some((1.0 / r, me.name.clone()));
    };
    let mut r = r_surfaces;
    for l in &asm.layers {
        if l.thickness <= 0.0 {
            continue;
        }
        let Some(m) = l.material else {
            // an unventilated air gap
            r += 0.18;
            continue;
        };
        let ElementKind::Material(mat) = &model.get(m)?.kind else {
            return None;
        };
        r += l.thickness / mat.lambda.filter(|x| *x > 0.0)?;
    }
    Some((1.0 / r, ae.name.clone()))
}

struct RoomAt<'a> {
    e: &'a Element,
    level: ElementId,
    shape: RoomShape,
    ti: f64,
    kind: RoomType,
    usage: RoomUse,
}

/// The heat loss of every room of the model.
pub fn building(model: &Model, o: &Opts) -> BuildingLoss {
    let rooms: Vec<RoomAt> = model
        .iter()
        .filter_map(|e| {
            let ElementKind::Room(r) = &e.kind else {
                return None;
            };
            let shape = room::shape_of(model, r).filter(|s| s.outline.len() > 2)?;
            let kind = RoomType::of(&r.occupancy, &e.name);
            let usage = RoomUse::from(kind);
            Some(RoomAt {
                e,
                level: r.level,
                shape,
                ti: usage.temp,
                kind,
                usage,
            })
        })
        .collect();
    let elev = |id: ElementId| model.level(id).map_or(0.0, |l| l.elevation);
    let ground = model
        .levels()
        .map(|(_, l)| l.elevation)
        .fold(f64::MAX, f64::min);
    // B′ of each level: its rooms' area over half their outside perimeter
    let room_at = |level: ElementId, p: DVec2, not: ElementId| {
        rooms
            .iter()
            .find(|x| x.level == level && x.e.id != not && polygon::contains(&x.shape.outline, p))
    };
    let mut out = BuildingLoss::default();
    let north = model.project_info().map_or(0.0, |(_, p)| p.true_north);
    // (room index, outside perimeter) first, for B′
    let outside_len = |r: &RoomAt| -> f64 {
        let n = r.shape.outline.len();
        (0..n)
            .filter_map(|i| {
                let (a, b) = (r.shape.outline[i], r.shape.outline[(i + 1) % n]);
                let wall = wall_of(model, r.shape.bounded_by.get(i).copied().flatten())?;
                let probe = beyond(a, b, wall.1.thickness);
                room_at(r.level, probe, r.e.id)
                    .is_none()
                    .then_some(a.distance(b))
            })
            .sum()
    };
    let perims: Vec<f64> = rooms.iter().map(outside_len).collect();
    let bp_level = |level: ElementId| -> f64 {
        let (mut a, mut p) = (0.0, 0.0);
        for (k, r) in rooms.iter().enumerate() {
            if r.level == level {
                a += r.shape.area();
                p += perims[k];
            }
        }
        if p > 0.0 {
            a / (0.5 * p)
        } else {
            20.0
        }
    };
    for (k, r) in rooms.iter().enumerate() {
        let ElementKind::Room(room) = &r.e.kind else {
            continue;
        };
        let (ti, dt) = (r.ti, r.ti - o.te);
        let area = r.shape.area();
        let h = room::height(model, room);
        let volume = area * h;
        let mut items: Vec<Item> = vec![];
        let mut missing: Vec<Missing> = vec![];
        let mut miss = |m: Missing| {
            if !missing.contains(&m) {
                missing.push(m);
            }
        };
        let mut add =
            |kind: &'static str, detail: &str, a: f64, u: f64, f: f64, az: Option<f64>| {
                if a <= 1e-4 || u <= 0.0 {
                    return;
                }
                let phi = a * u * f * dt;
                match items.iter_mut().find(|x| {
                    x.kind == kind
                        && x.detail == detail
                        && (x.u - u).abs() < 1e-9
                        && x.f == f
                        && x.azimuth == az
                }) {
                    Some(x) => {
                        x.area += a;
                        x.phi += phi;
                    }
                    None => items.push(Item {
                        kind,
                        detail: detail.into(),
                        area: a,
                        u,
                        f,
                        phi,
                        azimuth: az,
                    }),
                }
            };
        let mut facade_dirs: Vec<f64> = vec![];
        let mut seen: Vec<ElementId> = vec![];
        let n = r.shape.outline.len();
        for i in 0..n {
            let (a, b) = (r.shape.outline[i], r.shape.outline[(i + 1) % n]);
            let len = a.distance(b);
            if len < 1e-3 {
                continue;
            }
            let Some((we, wall)) = wall_of(model, r.shape.bounded_by.get(i).copied().flatten())
            else {
                continue;
            };
            let gross = len * h;
            match room_at(r.level, beyond(a, b, wall.thickness), r.e.id) {
                None => {
                    // an outside wall: its openings along this edge first
                    let u = (wall.end - wall.start).normalize_or_zero();
                    let (s0, s1) = {
                        let (x, y) = ((a - wall.start).dot(u), (b - wall.start).dot(u));
                        (x.min(y), x.max(y))
                    };
                    let mut net = gross;
                    let az = azimuth(DVec2::new((b - a).y, -(b - a).x), north);
                    for e in model.iter() {
                        let ElementKind::Opening(op) = &e.kind else {
                            continue;
                        };
                        let mid = op.offset + op.width / 2.0;
                        if op.host != we.id
                            || op.kind == OpeningKind::Hole
                            || seen.contains(&e.id)
                            || mid < s0 - 1e-3
                            || mid > s1 + 1e-3
                        {
                            continue;
                        }
                        seen.push(e.id);
                        let ang = u.y.atan2(u.x);
                        if !facade_dirs
                            .iter()
                            .any(|x: &f64| ((x - ang).sin().atan2((x - ang).cos())).abs() < 0.35)
                        {
                            facade_dirs.push(ang);
                        }
                        let oa = (op.width * op.height).min(net.max(0.0));
                        net -= oa;
                        let (kind, uo) = if op.kind == OpeningKind::Window {
                            ("Window", o.u_window)
                        } else {
                            ("Outside door", o.u_door)
                        };
                        add(kind, "", oa, uo + o.d_u, 1.0, Some(az));
                    }
                    match u_of(model, we, R_WALL) {
                        Some((uw, name)) => add(
                            "Outside wall",
                            &name,
                            net.max(0.0),
                            uw + o.d_u,
                            1.0,
                            Some(az),
                        ),
                        None => miss(Missing::WallBuildUp),
                    }
                }
                Some(j) if ti - j.ti > 0.05 => {
                    // a colder room next door: f = (θi − θj) / (θi − θe);
                    // gains from warmer ones are not taken off
                    let label = format!("{} {} ({} °C)", room_number(j.e), j.e.name, j.ti);
                    match u_of(model, we, R_INNER) {
                        Some((uw, _)) => add(
                            "To the room",
                            label.trim(),
                            gross,
                            uw,
                            round3((ti - j.ti) / dt),
                            None,
                        ),
                        None => miss(Missing::InnerWallBuildUp),
                    }
                }
                Some(_) => {}
            }
        }
        // the floor: on the ground where no room is under it
        let c = room.at;
        let below = rooms.iter().any(|x| {
            elev(x.level) < elev(r.level) - 1e-6 && polygon::contains(&x.shape.outline, c)
        });
        let f_ground = FG1 * fg2(ti, o.te, o.te_mean);
        if !below {
            let slab = model.iter().find(|e| {
                matches!(&e.kind, ElementKind::Slab(s)
                    if s.level == r.level && s.boundary.len() > 2 && polygon::contains(&s.boundary, c))
            });
            match slab.and_then(|s| u_of(model, s, R_FLOOR)) {
                Some((u, name)) => {
                    // B′ of the whole building for a well insulated floor
                    // or a room with no outside wall, else the room's own
                    let bp = if u < 0.5 || perims[k] < 1e-3 {
                        bp_level(r.level)
                    } else {
                        area / (0.5 * perims[k])
                    };
                    // how deep the floor is under ±0.000, about the ground
                    let z = (-elev(r.level)).max(0.0);
                    add(
                        "Floor on the ground",
                        &name,
                        area,
                        round3(u_equiv_floor(bp, u, z)),
                        round3(f_ground),
                        None,
                    );
                }
                None => miss(Missing::FloorBuildUp),
            }
        }
        // upwards: a roof where no room is over it
        let above = rooms.iter().any(|x| {
            elev(x.level) > elev(r.level) + 1e-6 && polygon::contains(&x.shape.outline, c)
        });
        if !above {
            let roof = model.iter().find(|e| {
                matches!(&e.kind, ElementKind::Roof(rf)
                    if rf.boundary.len() > 2 && polygon::contains(&rf.boundary, c))
            });
            match roof {
                Some(e) => {
                    let ElementKind::Roof(rf) = &e.kind else {
                        unreachable!()
                    };
                    let slope = if matches!(rf.shape, creabim_core::RoofShape::Flat) {
                        1.0
                    } else {
                        1.0 / rf.pitch.clamp(0.0, 80.0).to_radians().cos()
                    };
                    match u_of(model, e, R_ROOF) {
                        Some((u, name)) => add("Roof", &name, area * slope, u + o.d_u, 1.0, None),
                        None => miss(Missing::RoofBuildUp),
                    }
                }
                None => miss(Missing::NoRoofOrCeiling),
            }
        }
        // ventilation: the larger of infiltration and the least change
        let v_min = (r
            .usage
            .ach
            .unwrap_or(if r.usage.flow.is_some() { 0.0 } else { 0.5 })
            * volume)
            .max(r.usage.flow.unwrap_or(0.0));
        let e = shield_e(facade_dirs.len(), o.shielding);
        let eps = height_eps(elev(r.level) - ground + h / 2.0);
        let v_inf = 2.0 * volume * o.n50 * e * eps;
        let phi_t: f64 = items.iter().map(|x| x.phi).sum();
        let phi_v = 0.34 * v_min.max(v_inf) * dt;
        out.phi += phi_t + phi_v;
        out.area += area;
        out.rooms.push(RoomLoss {
            id: r.e.id,
            kind: r.kind,
            number: room.number.clone(),
            name: r.e.name.clone(),
            level: model.get(r.level).map_or(String::new(), |l| l.name.clone()),
            ti,
            area,
            volume,
            items,
            phi_t,
            phi_v,
            phi: phi_t + phi_v,
            v_min,
            v_inf,
            facades: facade_dirs.len(),
            missing,
        });
    }
    out.rooms.sort_by(|a, b| a.number.cmp(&b.number));
    out
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

fn room_number(e: &Element) -> &str {
    match &e.kind {
        ElementKind::Room(r) => &r.number,
        _ => "",
    }
}

fn wall_of(model: &Model, id: Option<ElementId>) -> Option<(&Element, &creabim_core::Wall)> {
    let e = model.get(id?)?;
    match &e.kind {
        ElementKind::Wall(w) => Some((e, w)),
        _ => None,
    }
}

/// A point past the wall an edge `a`–`b` of a room runs along (the
/// outline is counter-clockwise, so out is to the right).
/// Where the direction `n` of the drawing points: degrees clockwise
/// from true north, which is `true_north` degrees anticlockwise of the
/// drawing's up; to the whole degree.
pub fn azimuth(n: DVec2, true_north: f64) -> f64 {
    let t = true_north.to_radians();
    let (north, east) = (DVec2::new(-t.sin(), t.cos()), DVec2::new(t.cos(), t.sin()));
    let a = n.dot(east).atan2(n.dot(north)).to_degrees().round();
    (a + 360.0) % 360.0
}

fn beyond(a: DVec2, b: DVec2, thickness: f64) -> DVec2 {
    let d = (b - a).normalize_or_zero();
    (a + b) / 2.0 + DVec2::new(d.y, -d.x) * (thickness + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tables_of_the_standard() {
        assert_eq!(shield_e(0, Shielding::Moderate), 0.0);
        assert_eq!(shield_e(1, Shielding::Moderate), 0.02);
        assert_eq!(shield_e(3, Shielding::None), 0.05);
        assert_eq!(
            (height_eps(5.0), height_eps(20.0), height_eps(40.0)),
            (1.0, 1.2, 1.5)
        );
        assert_eq!(room_use("Kúpeľňa").temp, 24.0);
        assert_eq!(room_use("Bathroom").flow, Some(90.0));
        assert_eq!(room_use("Chodba").temp, 15.0);
        assert_eq!(room_use("Garage").temp, 5.0);
        assert_eq!(room_use("Obývacia izba"), room_use("Bedroom"));
        assert_eq!(room_use("").ach, Some(0.5));
    }

    /// The sample house with a bathroom and a living room in it, every
    /// material given λ = 0.5 W/(m·K).
    fn house() -> Model {
        use creabim_core::{Document, Room, Transaction};
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc
            .model()
            .levels()
            .min_by(|a, b| a.1.elevation.total_cmp(&b.1.elevation))
            .unwrap()
            .0
            .id;
        let mut tx = Transaction::new("rooms");
        for (name, number, at) in [
            ("Kúpeľňa", "1.01", DVec2::new(2.5, 3.0)),
            ("Obývacia izba", "1.02", DVec2::new(8.0, 3.0)),
        ] {
            let mut r = Room::new(level, at);
            r.number = number.into();
            tx = tx.insert(Element::new(name, ElementKind::Room(r)));
        }
        let mats: Vec<Element> = doc
            .model()
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Material(_)))
            .cloned()
            .collect();
        for before in mats {
            let mut after = before.clone();
            if let ElementKind::Material(m) = &mut after.kind {
                m.lambda = Some(0.5);
            }
            tx = tx.replace(before, after);
        }
        doc.commit(tx).unwrap();
        doc.model().clone()
    }

    #[test]
    fn the_sample_house_loses_heat_through_what_it_is_built_of() {
        let model = house();
        let o = Opts::default();
        let b = building(&model, &o);
        assert_eq!(b.rooms.len(), 2, "{b:#?}");
        for r in &b.rooms {
            assert!(r.area > 1.0 && r.volume > r.area, "{r:?}");
            assert!(r.phi_v > 0.0 && r.phi >= r.phi_v);
            // what is counted adds up
            let sum: f64 = r.items.iter().map(|i| i.phi).sum();
            assert!((sum - r.phi_t).abs() < 1e-6);
            for i in &r.items {
                let dt = r.ti - o.te;
                assert!((i.phi - i.area * i.u * i.f * dt).abs() < 1e-6, "{i:?}");
            }
        }
        assert!((b.phi - b.rooms.iter().map(|r| r.phi).sum::<f64>()).abs() < 1e-6);
        let (bath, living) = (&b.rooms[0], &b.rooms[1]);
        assert_eq!((bath.ti, living.ti), (24.0, 20.0));
        // the bathroom loses to the colder living room: f = 4 / 35
        let to = bath
            .items
            .iter()
            .find(|i| i.kind == "To the room")
            .expect("a colder room next door");
        assert!((to.f - 0.114).abs() < 1e-9 && to.detail.starts_with("1.02 Obývacia izba"));
        assert!(living.items.iter().all(|i| i.kind != "To the room"));
        assert!(bath.items.iter().any(|i| i.kind == "Outside wall"));
        assert!(living.items.iter().any(|i| i.kind == "Window"));
        // a bathroom is aired by 90 m³/h at least
        assert!(bath.v_min >= 90.0);
        // a colder design day loses more
        let colder = building(&model, &Opts { te: -15.0, ..o });
        assert!(colder.phi > b.phi);
    }
}
