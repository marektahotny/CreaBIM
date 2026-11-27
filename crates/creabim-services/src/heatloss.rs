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

/// How the losses are worked out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Method {
    /// One surcharge ΔU for every outside wall, floors and ceilings
    /// between storeys left out.
    #[default]
    Simplified,
    /// STN EN 12831-1:2019 chapter 6: areas by the outside dimensions,
    /// ΔU_TB of table B.1, heat to colder rooms beside, over and under,
    /// the ground by annex E and the air by 6.3.3.
    Detailed,
}

impl Method {
    pub const ALL: [Method; 2] = [Method::Simplified, Method::Detailed];
    pub fn name(self) -> &'static str {
        match self {
            Method::Simplified => "Simplified",
            Method::Detailed => "Detailed",
        }
    }
}

/// What the calculation is given.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opts {
    pub method: Method,
    /// The detailed method's row of table B.1 (ΔU_TB by the kind of
    /// building), counted from 0; the values are read from the norm's
    /// parameter file, never built in.
    pub tb_row: usize,
    /// Ground water within 1 m under the floor slab (f_GW, B.2.3).
    pub gw_near: bool,
    /// The detailed method's thermal bridges as Σ l·Ψ (STN EN 12831-1
    /// annex C, Ψ of STN EN ISO 14683 table C.2, [`crate::psi`]) instead of
    /// ΔU_TB of table B.1; and the detail picked for each kind of junction
    /// ([`crate::psi::slot`]), an index into the table's rows (none: by
    /// the build-up).
    pub psi: bool,
    pub psi_pick: [Option<u16>; 7],
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
    /// The rooms whose type takes air out (bathroom, WC, kitchen…) have
    /// fans running at their extract flow (q_v,exh of STN EN 12831-1
    /// 6.3.3.3); without them the house airs naturally and only the
    /// infiltration and the least air change count (formula 14).
    pub exhaust_fans: bool,
}

impl Default for Opts {
    fn default() -> Self {
        Self {
            method: Method::Simplified,
            tb_row: 1,
            gw_near: false,
            psi: false,
            psi_pick: [None; 7],
            te: -11.0,
            te_mean: None,
            te_heating: None,
            heating_days: None,
            d_u: 0.05,
            u_window: 0.6,
            u_door: 0.65,
            n50: 2.0,
            exhaust_fans: false,
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
    /// The detailed method lacks a table of STN EN 12831-1 (its
    /// parameter file is not read): that part is left out.
    NormTable,
    /// The detailed method needs the mean annual outside temperature
    /// θe,m for the ground: the floor on the ground is left out.
    MeanTemperature,
    /// A floor or ceiling to a colder room with no build-up.
    InnerSlabBuildUp,
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
            Missing::NormTable => {
                "a table of STN EN 12831-1 is not read (Read the table…): that part is left out"
            }
            Missing::MeanTemperature => {
                "no mean annual outside temperature θe,m: the floor on the ground is left out"
            }
            Missing::InnerSlabBuildUp => {
                "a floor or ceiling to a colder room with no build-up: its loss is left out"
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
    /// Of the air taken out (a bathroom's 90 m³/h), what comes in through
    /// the door from the rooms next to it, m³/h, and their temperature,
    /// °C: heated by `ti − θ` only (STN EN 12831-1, (15), q_v,transfer).
    pub v_transfer: f64,
    pub t_transfer: Option<f64>,
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
pub(crate) const R_WALL: f64 = 0.13 + 0.04;
pub(crate) const R_ROOF: f64 = 0.10 + 0.04;
pub(crate) const R_FLOOR: f64 = 0.17;
pub(crate) const R_INNER: f64 = 0.13 + 0.13;
/// A floor or ceiling between rooms, heat flowing up and down.
pub(crate) const R_CEILING_UP: f64 = 0.10 + 0.10;
pub(crate) const R_FLOOR_DOWN: f64 = 0.17 + 0.17;

/// U of ceiling `e` with the roof space over it, W/(m²K), and what it is:
/// its build-up between Rsi and Rse, and the roof space's Ru (STN EN ISO
/// 6946 6.10.2, table 9) added when one is picked and the table is read.
pub(crate) fn ceiling_u(
    model: &Model,
    e: &Element,
    tables: &[creabim_core::fireparams::ParamTable],
) -> Option<(f64, String)> {
    let ElementKind::Ceiling(c) = &e.kind else {
        return None;
    };
    let (u, name) = u_of(model, e, R_ROOF)?;
    match creabim_core::room::roof_space_ru(tables, c) {
        Some(ru) => Some((
            1.0 / (1.0 / u + ru),
            format!("{name} + Ru {}", format!("{ru}").replace('.', ",")),
        )),
        None => Some((u, name)),
    }
}

/// U of the build-up (or the one material) element `e` is made of,
/// W/(m²K), and its name: none where it has neither or a layer has no λ.
pub(crate) fn u_of(model: &Model, e: &Element, r_surfaces: f64) -> Option<(f64, String)> {
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

/// The U of build-up `asm` with surface resistances `r_surfaces` (an
/// unventilated gap with no material 0,18), none while a layer has no λ.
pub(crate) fn u_of_assembly(
    model: &Model,
    asm: &creabim_core::assembly::Assembly,
    r_surfaces: f64,
) -> Option<f64> {
    let mut r = r_surfaces;
    for l in &asm.layers {
        if l.thickness <= 0.0 {
            continue;
        }
        let Some(m) = l.material else {
            r += 0.18;
            continue;
        };
        let ElementKind::Material(mat) = &model.get(m)?.kind else {
            return None;
        };
        r += l.thickness / mat.lambda.filter(|x| *x > 0.0)?;
    }
    Some(1.0 / r)
}

/// The face of outside wall `we` between `s` along it (from its start)
/// and heights `z` that concrete members in its core take: ring beams,
/// RC lintels, RC columns, with the insulation or VELOX boards in their
/// formwork (George 2026-11-27): each one's detail, area and U (none
/// when a layer has no λ). Where two overlap, the first keeps it.
pub(crate) fn insert_parts(
    model: &Model,
    we: &Element,
    wall_name: &str,
    s: (f64, f64),
    z: (f64, f64),
    r_surfaces: f64,
) -> Vec<(String, f64, Option<f64>)> {
    type Rect = ((f64, f64), (f64, f64));
    let clip = |a: (f64, f64), b: (f64, f64)| (a.0.max(b.0), a.1.min(b.1));
    let area = |r: Rect| (r.0 .1 - r.0 .0).max(0.0) * (r.1 .1 - r.1 .0).max(0.0);
    let mut taken: Vec<Rect> = vec![];
    let mut out = vec![];
    for ins in creabim_core::thermal::inserts(model, we.id) {
        let r = (clip(ins.along, s), clip(ins.z, z));
        let mut a = area(r);
        if a < 1e-6 {
            continue;
        }
        for t in &taken {
            a -= area((clip(r.0, t.0), clip(r.1, t.1)));
        }
        taken.push(r);
        if a < 1e-6 {
            continue;
        }
        let u = u_of_assembly(model, &ins.assembly, r_surfaces);
        out.push((format!("{wall_name} – {}", ins.name), a, u));
    }
    out
}

pub(crate) struct RoomAt<'a> {
    pub e: &'a Element,
    pub level: ElementId,
    pub shape: RoomShape,
    pub ti: f64,
    pub kind: RoomType,
    pub usage: RoomUse,
}

/// The model's rooms with a shape, what they are used for and heated to.
pub(crate) fn rooms_of(model: &Model) -> Vec<RoomAt<'_>> {
    model
        .iter()
        .filter_map(|e| {
            let ElementKind::Room(r) = &e.kind else {
                return None;
            };
            let shape = room::shape_of(model, r).filter(|s| s.outline.len() > 2)?;
            let kind = RoomType::of(&r.occupancy, &e.name);
            let mut usage = RoomUse::from(kind);
            if let Some(t) = r.heated_to {
                usage.temp = t;
            }
            Some(RoomAt {
                e,
                level: r.level,
                shape,
                ti: usage.temp,
                kind,
                usage,
            })
        })
        .collect()
}

/// The heat loss of every room of the model.
pub fn building(model: &Model, o: &Opts) -> BuildingLoss {
    if o.method == Method::Detailed {
        return crate::heatdetail::building(model, o);
    }
    let rooms = rooms_of(model);
    let tables = creabim_core::room::project_tables(model);
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
    let mut airs: Vec<(usize, usize, f64, f64, f64, f64, f64)> = vec![];
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
                        Some((uw, name)) => {
                            // ring beams, RC lintels and columns in the
                            // core: their own build-up over their face
                            let z0 = elev(r.level);
                            for (detail, ia, iu) in
                                insert_parts(model, we, &name, (s0, s1), (z0, z0 + h), R_WALL)
                            {
                                match iu {
                                    Some(iu) => {
                                        let ia = ia.min(net.max(0.0));
                                        net -= ia;
                                        add("Outside wall", &detail, ia, iu + o.d_u, 1.0, Some(az));
                                    }
                                    None => miss(Missing::WallBuildUp),
                                }
                            }
                            add(
                                "Outside wall",
                                &name,
                                net.max(0.0),
                                uw + o.d_u,
                                1.0,
                                Some(az),
                            )
                        }
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
        // a ceiling with a build-up over the room (podhľad), the roof space
        // over it as a layer of STN EN ISO 6946 table 9 (George 2026-11-23)
        let ceiling = if above {
            None
        } else {
            creabim_core::room::built_ceiling_over(model, r.e.id, r.level, c)
        };
        if let Some(ce) = ceiling {
            match ceiling_u(model, ce, &tables) {
                Some((u, name)) => add("Ceiling", &name, area, u + o.d_u, 1.0, None),
                None => miss(Missing::RoofBuildUp),
            }
        } else if !above {
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
        // ventilation after all rooms are known (the air the fans take
        // out comes in through the whole envelope)
        let e = shield_e(facade_dirs.len(), o.shielding);
        let eps = height_eps(elev(r.level) - ground + h / 2.0);
        let v_inf = 2.0 * volume * o.n50 * e * eps;
        // A_env,i: the room's envelope to the outside air (6.3.3.6)
        let a_env: f64 = items
            .iter()
            .filter(|x| {
                matches!(
                    x.kind,
                    "Outside wall" | "Window" | "Outside door" | "Ceiling" | "Roof"
                )
            })
            .map(|x| x.area)
            .sum();
        let phi_t: f64 = items.iter().map(|x| x.phi).sum();
        airs.push((out.rooms.len(), k, volume, v_inf, a_env, dt, ti));
        out.phi += phi_t;
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
            phi_v: 0.0,
            phi: phi_t,
            v_min: 0.0,
            v_inf,
            v_transfer: 0.0,
            t_transfer: None,
            facades: facade_dirs.len(),
            missing,
        });
    }
    // the fans' extract air q_v,exh,z comes in through the envelope and is
    // shared out by the rooms' envelope areas (STN EN 12831-1 (24), (19))
    let q_exh_z: f64 = if o.exhaust_fans {
        airs.iter().map(|a| q_exh_of(&rooms[a.1], true)).sum()
    } else {
        0.0
    };
    let a_env_z: f64 = airs.iter().map(|a| a.4).sum();
    for (i, k, volume, v_inf, a_env, dt, ti) in airs {
        let q_env = if a_env_z > 1e-9 {
            v_inf + q_exh_z * a_env / a_env_z
        } else {
            v_inf
        };
        let (v_min, v_out, v_transfer, t_transfer) =
            air_of(model, &rooms, k, volume, q_env, o.exhaust_fans);
        let phi_v = 0.34 * (v_out * dt + v_transfer * (ti - t_transfer.unwrap_or(ti)));
        let x = &mut out.rooms[i];
        x.phi_v = phi_v;
        x.phi += phi_v;
        x.v_min = v_min;
        x.v_inf = q_env;
        x.v_transfer = v_transfer;
        x.t_transfer = t_transfer;
        out.phi += phi_v;
    }
    out.rooms.sort_by(|a, b| a.number.cmp(&b.number));
    out
}

/// The conductance between every two rooms that touch, W/K: through
/// the inner walls between them (each room's own edges, the two sides
/// averaged) and through the floor between a room and the one right
/// over it (the shared area by the upper room's floor build-up). Pairs
/// `(a, b, H)` with `a < b`; for the summer, where rooms of one
/// temperature still pass heat to each other.
pub(crate) fn partitions(model: &Model) -> Vec<(ElementId, ElementId, f64)> {
    let rooms = rooms_of(model);
    let elev = |id: ElementId| model.level(id).map_or(0.0, |l| l.elevation);
    let mut out: Vec<(ElementId, ElementId, f64)> = vec![];
    let mut put = |x: ElementId, y: ElementId, h: f64| {
        if x == y || h <= 0.0 {
            return;
        }
        let (a, b) = if x < y { (x, y) } else { (y, x) };
        match out.iter_mut().find(|p| p.0 == a && p.1 == b) {
            Some(p) => p.2 += h,
            None => out.push((a, b, h)),
        }
    };
    for r in &rooms {
        let ElementKind::Room(room) = &r.e.kind else {
            continue;
        };
        let h = room::height(model, room);
        let n = r.shape.outline.len();
        for i in 0..n {
            let (a, b) = (r.shape.outline[i], r.shape.outline[(i + 1) % n]);
            let Some((we, wall)) = wall_of(model, r.shape.bounded_by.get(i).copied().flatten())
            else {
                continue;
            };
            let p = beyond(a, b, wall.thickness);
            let Some(j) = rooms.iter().find(|x| {
                x.level == r.level && x.e.id != r.e.id && polygon::contains(&x.shape.outline, p)
            }) else {
                continue;
            };
            if let Some((u, _)) = u_of(model, we, R_INNER) {
                // each side counts its own edges: half of it from each
                put(r.e.id, j.e.id, 0.5 * a.distance(b) * h * u);
            }
        }
        // the room right over it, through that room's floor
        let up = rooms
            .iter()
            .filter(|x| elev(x.level) > elev(r.level) + 1e-6)
            .map(|x| elev(x.level))
            .fold(f64::MAX, f64::min);
        for x in rooms.iter().filter(|x| (elev(x.level) - up).abs() < 1e-6) {
            let shared = overlap_area(&r.shape.outline, &x.shape.outline);
            if shared < 0.01 {
                continue;
            }
            let ElementKind::Room(xr) = &x.e.kind else {
                continue;
            };
            let c = xr.at;
            let slab = model.iter().find(|e| {
                matches!(&e.kind, ElementKind::Slab(s)
                    if s.level == x.level && s.boundary.len() > 2 && polygon::contains(&s.boundary, c))
            });
            if let Some((u, _)) = slab.and_then(|s| u_of(model, s, R_FLOOR_DOWN)) {
                put(r.e.id, x.e.id, shared * u);
            }
        }
    }
    out
}

/// The area two outlines share, m², by their triangles.
pub fn overlap_area(a: &[DVec2], b: &[DVec2]) -> f64 {
    let tri = |p: &[DVec2]| -> Vec<[DVec2; 3]> {
        polygon::triangulate(p)
            .into_iter()
            .map(|t| [p[t[0]], p[t[1]], p[t[2]]])
            .collect()
    };
    let (ta, tb) = (tri(a), tri(b));
    let mut sum = 0.0;
    for x in &ta {
        for y in &tb {
            let mut poly = x.to_vec();
            for i in 0..3 {
                let (p, q) = (y[i], y[(i + 1) % 3]);
                // keep the left of p → q (the triangles run anticlockwise)
                let n = DVec2::new((q - p).y, -(q - p).x);
                poly = creabim_core::members::clip_half_plane(&poly, n, n.dot(p));
                if poly.len() < 3 {
                    break;
                }
            }
            if poly.len() > 2 {
                sum += polygon::signed_area2(&poly).abs() / 2.0;
            }
        }
    }
    sum
}

pub(crate) fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

pub(crate) fn room_number(e: &Element) -> &str {
    match &e.kind {
        ElementKind::Room(r) => &r.number,
        _ => "",
    }
}

pub(crate) fn wall_of(
    model: &Model,
    id: Option<ElementId>,
) -> Option<(&Element, &creabim_core::Wall)> {
    let e = model.get(id?)?;
    match &e.kind {
        ElementKind::Wall(w) => Some((e, w)),
        _ => None,
    }
}

/// q_v,exh,i of a room, m³/h: the extract flow of a room its type takes
/// air out of (bathroom, WC, kitchen, laundry), when the house has fans.
pub(crate) fn q_exh_of(r: &RoomAt, fans: bool) -> f64 {
    if fans && r.kind.needs().exhaust {
        r.usage.flow.unwrap_or(0.0)
    } else {
        0.0
    }
}

/// The air of room `k`, m³/h, by STN EN 12831-1 6.3.3.3: what it needs
/// (the larger of its least air change and its extract flow, for the air
/// balance), what is heated from outside, what comes through its door
/// from the heated rooms next to it and their (lowest) temperature.
/// `q_env` is the room's share of the air coming in through the envelope
/// (q_v,env,i, formula 18). With `fans` a room with an extract flow has
/// q_v,techn,i = q_v,exh,i (23) and its air is made up by transfer air
/// q_v,transfer,ij = q_v,exh,i (15), heated from the neighbours'
/// temperature; from outside only max(q_v,env,i; q_v,min,i − q_v,techn,i).
/// Without fans max(q_v,env,i; q_v,min,i) (14). With no door to a heated
/// room the transfer air is taken as outside air.
pub(crate) fn air_of(
    model: &Model,
    rooms: &[RoomAt],
    k: usize,
    volume: f64,
    q_env: f64,
    fans: bool,
) -> (f64, f64, f64, Option<f64>) {
    let r = &rooms[k];
    // q_v,min,i = n_min,i·V_i (6.3.3.5)
    let q_min = r
        .usage
        .ach
        .unwrap_or(if r.usage.flow.is_some() { 0.0 } else { 0.5 })
        * volume;
    let need = q_min.max(r.usage.flow.unwrap_or(0.0));
    let q_exh = q_exh_of(r, fans);
    if q_exh <= 1e-9 {
        // no fan: max(q_v,env,i; q_v,min,i) from outside (formula 14/15)
        return (need, q_env.max(q_min), 0.0, None);
    }
    // a fan: q_v,techn,i = q_v,exh,i (23), the air it takes out comes in
    // through the door as q_v,transfer,ij = q_v,exh,i; from outside only
    // max(q_v,env,i; q_v,min,i − q_v,techn,i) (15)
    let outside = q_env.max(q_min - q_exh);
    match door_temp(model, rooms, k) {
        Some(t) => (need, outside, q_exh, Some(t.min(r.ti))),
        // no door to a heated room: the air is taken as outside air
        None => (need, outside + q_exh, 0.0, None),
    }
}

/// The lowest temperature of the heated rooms room `k`'s doors open to,
/// or it lies open to (an edge with no wall).
fn door_temp(model: &Model, rooms: &[RoomAt], k: usize) -> Option<f64> {
    let r = &rooms[k];
    let n = r.shape.outline.len();
    let mut best: Option<f64> = None;
    let room_past = |p: DVec2| {
        rooms
            .iter()
            .enumerate()
            .find(|(j, x)| *j != k && x.level == r.level && polygon::contains(&x.shape.outline, p))
            .map(|(_, x)| x.ti)
    };
    for i in 0..n {
        let (a, b) = (r.shape.outline[i], r.shape.outline[(i + 1) % n]);
        let len = a.distance(b);
        if len < 1e-6 {
            continue;
        }
        let d = (b - a) / len;
        let Some((we, w)) = wall_of(model, r.shape.bounded_by.get(i).copied().flatten()) else {
            // no wall: open to the room past it (a kitchen to the living
            // room, by a room separator)
            if let Some(t) = room_past((a + b) / 2.0 + DVec2::new(d.y, -d.x) * 0.05) {
                best = Some(best.map_or(t, |m: f64| m.min(t)));
            }
            continue;
        };
        if w.length() < 1e-6 {
            continue;
        }
        for e in model.dependents(we.id) {
            let Some(ElementKind::Opening(o)) = model.get(e).map(|x| &x.kind) else {
                continue;
            };
            if o.kind != creabim_core::OpeningKind::Door {
                continue;
            }
            let c = w.start + w.direction() * (o.offset + o.width / 2.0);
            let t = (c - a).dot(d);
            if t < -1e-6 || t > len + 1e-6 || (c - a).perp_dot(d).abs() > w.thickness + 0.05 {
                continue;
            }
            let p = a + d * t + DVec2::new(d.y, -d.x) * (w.thickness + 0.05);
            if let Some(t) = room_past(p) {
                best = Some(best.map_or(t, |m: f64| m.min(t)));
            }
        }
    }
    best
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

pub(crate) fn beyond(a: DVec2, b: DVec2, thickness: f64) -> DVec2 {
    let d = (b - a).normalize_or_zero();
    (a + b) / 2.0 + DVec2::new(d.y, -d.x) * (thickness + 0.05)
}

/// The sample house with a bathroom and a living room in it, every
/// material given λ = 0.5 W/(m·K).
#[cfg(test)]
pub(crate) fn test_house() -> Model {
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

    fn house() -> Model {
        test_house()
    }

    #[test]
    fn the_fans_air_comes_in_through_the_envelope() {
        let m = house();
        let off = building(
            &m,
            &Opts {
                exhaust_fans: false,
                ..Opts::default()
            },
        );
        let on = building(
            &m,
            &Opts {
                exhaust_fans: true,
                ..Opts::default()
            },
        );
        // without fans the bathroom has max(q_v,env,i; q_v,min,i) (14)
        let bath = |b: &BuildingLoss| b.rooms.iter().find(|r| r.number == "1.01").unwrap().clone();
        let (b0, b1) = (bath(&off), bath(&on));
        let q_min = 0.5 * b0.volume;
        assert!((b0.phi_v - 0.34 * b0.v_inf.max(q_min) * (24.0 + 11.0)).abs() < 1e-9);
        // with them their 90 m³/h is shared out by the envelope (24, 19)
        let sum = |b: &BuildingLoss| b.rooms.iter().map(|r| r.v_inf).sum::<f64>();
        assert!((sum(&on) - sum(&off) - 90.0).abs() < 1e-9);
        for (r0, r1) in off.rooms.iter().zip(&on.rooms) {
            assert!(r1.v_inf > r0.v_inf);
        }
        // its door opens to the living room (20 °C): 90 m³/h transfer air
        // (15), from outside only max(q_v,env,i; q_v,min,i − q_v,exh,i)
        assert_eq!((b1.v_transfer, b1.t_transfer), (90.0, Some(20.0)));
        let out = b1.v_inf.max(q_min - 90.0);
        let phi = 0.34 * (out * (24.0 + 11.0) + 90.0 * (24.0 - 20.0));
        assert!((b1.phi_v - phi).abs() < 1e-9);
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

    /// A ceiling with a build-up (podhľad) over a room takes the roof's
    /// place, with the roof space's Ru of STN EN ISO 6946 table 9 added
    /// when one is picked and the table read (the value here is a test's,
    /// not the norm's).
    #[test]
    fn a_ceiling_with_a_build_up_counts_with_its_roof_space() {
        use creabim_core::{Assembly, AssemblyUse, Layer, LayerFunction, Transaction};
        let mut model = house();
        let mat = model
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Material(_)))
            .unwrap()
            .id;
        let asm = Element::new(
            "Podhľad",
            ElementKind::Assembly(Assembly::new(
                AssemblyUse::Ceiling,
                vec![Layer::new(LayerFunction::Insulation, Some(mat), 0.2)],
            )),
        );
        let aid = asm.id;
        let mut tx = Transaction::new("t").insert(asm);
        let rooms: Vec<(ElementId, ElementId)> = model
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Room(r) => Some((e.id, r.level)),
                _ => None,
            })
            .collect();
        for (id, level) in rooms {
            let mut c = creabim_core::room::Ceiling::in_room(level, id, 2.5);
            c.assembly = Some(aid);
            c.thickness = 0.2;
            c.roof_space = Some("2".into());
            tx = tx.insert(Element::new("Ceiling", ElementKind::Ceiling(c)));
        }
        tx.apply(&mut model).unwrap();
        let o = Opts::default();
        let u0 = 1.0 / (R_ROOF + 0.2 / 0.5);
        let ceiling = |m: &Model| {
            building(m, &o).rooms[1]
                .items
                .iter()
                .find(|i| i.kind == "Ceiling")
                .cloned()
                .expect("the ceiling counted")
        };
        let c = ceiling(&model);
        assert!((c.u - o.d_u - u0).abs() < 1e-9, "{c:?}");
        assert!(building(&model, &o).rooms[1]
            .items
            .iter()
            .all(|i| i.kind != "Roof"));
        // table 9 read: Ru added
        let mut info = model
            .project_info()
            .map(|(_, p)| p.clone())
            .unwrap_or_default();
        info.fire_params = creabim_core::fireparams::parse(
            "#table roof_space\n#columns key\tru\ttext\n2\t0,5\ttest\n",
        );
        creabim_core::project::set_project(&model, info)
            .apply(&mut model)
            .unwrap();
        let c = ceiling(&model);
        assert!((c.u - o.d_u - 1.0 / (1.0 / u0 + 0.5)).abs() < 1e-9, "{c:?}");
        assert!(c.detail.ends_with("+ Ru 0,5"), "{c:?}");
    }

    #[test]
    fn shared_area_of_two_outlines() {
        let sq =
            |x: f64, y: f64, s: f64| polygon::rectangle(DVec2::new(x, y), DVec2::new(x + s, y + s));
        assert!((overlap_area(&sq(0.0, 0.0, 4.0), &sq(2.0, 2.0, 4.0)) - 4.0).abs() < 1e-9);
        assert!(overlap_area(&sq(0.0, 0.0, 1.0), &sq(5.0, 5.0, 1.0)) < 1e-12);
        let l = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(4.0, 0.0),
            DVec2::new(4.0, 2.0),
            DVec2::new(2.0, 2.0),
            DVec2::new(2.0, 4.0),
            DVec2::new(0.0, 4.0),
        ];
        assert!((overlap_area(&l, &sq(0.0, 0.0, 4.0)) - 12.0).abs() < 1e-9);
    }
}
