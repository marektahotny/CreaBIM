//! The rooms' cooling load, as PilonDesigner's `coolingload.ts`: a
//! simplified calculation after STN 73 0548. Sun through the windows by
//! where they face (21 July, 50° north) with the glazing's g and the
//! shading, heat through walls and roof with an added temperature
//! difference for the sun on them, through windows and doors, the gains
//! inside (people, lighting, appliances) and ventilation. The geometry
//! and the U values are those of the heat loss
//! ([`crate::heatloss`]). The result sizes fan-coil units and the
//! source of cold.

use crate::heatloss::{self, RoomLoss};
use creabim_core::{ElementId, Model};

/// How the windows are shaded.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Shading {
    /// Outside blinds or shutters.
    External,
    /// Inside blinds or curtains.
    #[default]
    Internal,
    None,
}

impl Shading {
    pub const ALL: [Shading; 3] = [Shading::External, Shading::Internal, Shading::None];

    pub fn name(self) -> &'static str {
        match self {
            Shading::External => "Outside blinds or shutters",
            Shading::Internal => "Inside blinds or curtains",
            Shading::None => "No shading",
        }
    }

    /// The shading factor Fc.
    pub fn fc(self) -> f64 {
        match self {
            Shading::External => 0.25,
            Shading::Internal => 0.6,
            Shading::None => 1.0,
        }
    }
}

/// What the load is calculated with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// Design outside temperature in summer, °C.
    pub te: f64,
    /// Wanted inside, °C.
    pub ti: f64,
    /// Solar factor of the glazing.
    pub g: f64,
    pub shading: Shading,
    /// The glazed share of a window's area.
    pub glazed: f64,
    /// Storage factor of the sun's load (1 = none taken off).
    pub storage: f64,
    /// Sensible heat of a person, W.
    pub w_person: f64,
    /// Lighting, W/m².
    pub light: f64,
    /// Appliances, W/m²: dwelling, kitchen, office.
    pub eq_home: f64,
    pub eq_kitchen: f64,
    pub eq_office: f64,
    /// Floor area to a person, m²: dwelling, office.
    pub m2_home: f64,
    pub m2_office: f64,
    /// Added to the temperature difference for the sun on a wall and
    /// on a roof, K.
    pub dt_wall: f64,
    pub dt_roof: f64,
    /// The outside air's daily range in summer, max − min, K: the hours
    /// before and after the warmest (15 h) are cooler by its half-cosine.
    pub swing: f64,
    /// The hours the building is used, from and to: the load is the
    /// highest of these hours.
    pub from_h: u32,
    pub to_h: u32,
    /// Chilled water, °C.
    pub supply_c: f64,
    pub return_c: f64,
    /// Summer stability: the highest room air temperature allowed, °C
    /// (STN 73 0540-2: 27 °C for rooms people stay in).
    pub summer_tmax: f64,
    /// Air change by opening windows in summer, 1/h.
    pub summer_ach: f64,
    /// The ground under a floor on the ground, °C, and its loss,
    /// W/(m²K).
    pub t_ground: f64,
    pub ground_h: f64,
    /// How heavy the inner layers are, for the surfaces' admittance.
    pub mass: Mass,
}

/// How much heat the room's inner surfaces store: the mass of the
/// layers from the inside to the insulation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mass {
    /// Plasterboard, timber frame: under 60 kg/m².
    Light,
    /// 60 to 250 kg/m².
    Medium,
    /// Masonry or concrete inside the insulation, 250 kg/m² or more.
    #[default]
    Heavy,
}

impl Mass {
    pub const ALL: [Mass; 3] = [Mass::Light, Mass::Medium, Mass::Heavy];

    pub fn name(self) -> &'static str {
        match self {
            Mass::Light => "Light (plasterboard, timber frame)",
            Mass::Medium => "Medium",
            Mass::Heavy => "Heavy (masonry, concrete)",
        }
    }

    /// The inner surfaces' admittance Y, W/(m²K).
    pub fn admittance(self) -> f64 {
        match self {
            Mass::Light => 1.5,
            Mass::Medium => 3.0,
            Mass::Heavy => 4.0,
        }
    }
}

impl Settings {
    /// The outside temperature at hour `h`: the highest at 15 h, lower
    /// by the daily range's half-cosine either side.
    pub fn te_at(&self, h: f64) -> f64 {
        let a = (h - 15.0) / 24.0 * std::f64::consts::TAU;
        self.te - self.swing / 2.0 * (1.0 - a.cos())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            te: 32.0,
            ti: 26.0,
            g: 0.5,
            shading: Shading::Internal,
            glazed: 0.7,
            storage: 1.0,
            w_person: 70.0,
            light: 6.0,
            eq_home: 4.0,
            eq_kitchen: 12.0,
            eq_office: 15.0,
            m2_home: 20.0,
            m2_office: 10.0,
            dt_wall: 5.0,
            dt_roof: 15.0,
            swing: 15.0,
            from_h: 8,
            to_h: 16,
            supply_c: 7.0,
            return_c: 12.0,
            summer_tmax: 27.0,
            summer_ach: 1.5,
            t_ground: 15.0,
            ground_h: 0.3,
            mass: Mass::Heavy,
        }
    }
}

/// The hours of [`SOLAR_JULY`].
pub const SOLAR_HOURS: [u32; 15] = [5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19];

/// Solar radiation passing a single-glazed window I₀, W/m², 21 July,
/// 50° north, atmospheric turbidity z = 4, for the hours 5 to 19 and the
/// faces N, NE, E, SE, S, SW, W, NW (Chyský, Hemzal et al.: Technický
/// průvodce – Větrání a klimatizace, 1993; the base of STN 73 0548).
/// Other glazing: the shading coefficient SC = g / 0.87.
pub const SOLAR_JULY: [[f64; 8]; 15] = [
    [45.0, 85.0, 83.0, 41.0, 24.0, 24.0, 24.0, 24.0],
    [87.0, 287.0, 322.0, 180.0, 53.0, 53.0, 53.0, 53.0],
    [80.0, 361.0, 481.0, 335.0, 78.0, 78.0, 78.0, 78.0],
    [100.0, 321.0, 539.0, 452.0, 128.0, 100.0, 100.0, 100.0],
    [117.0, 217.0, 505.0, 511.0, 230.0, 117.0, 117.0, 117.0],
    [130.0, 135.0, 389.0, 506.0, 335.0, 130.0, 130.0, 130.0],
    [139.0, 139.0, 232.0, 437.0, 409.0, 185.0, 139.0, 139.0],
    [141.0, 141.0, 141.0, 316.0, 435.0, 316.0, 141.0, 141.0],
    [139.0, 139.0, 139.0, 185.0, 409.0, 437.0, 232.0, 139.0],
    [130.0, 130.0, 130.0, 130.0, 335.0, 506.0, 389.0, 135.0],
    [117.0, 117.0, 117.0, 117.0, 230.0, 511.0, 505.0, 217.0],
    [100.0, 100.0, 100.0, 100.0, 128.0, 452.0, 539.0, 321.0],
    [80.0, 78.0, 78.0, 78.0, 78.0, 335.0, 481.0, 361.0],
    [87.0, 53.0, 53.0, 53.0, 53.0, 180.0, 322.0, 287.0],
    [46.0, 24.0, 24.0, 24.0, 24.0, 41.0, 83.0, 85.0],
];

/// g of single glazing, which I₀ is for.
pub const G_SINGLE: f64 = 0.87;

/// I₀ for the azimuth `az` (0 = north, clockwise; between the faces in
/// a straight line) at the hour of index `k`.
pub fn solar_at(az: f64, k: usize) -> f64 {
    let a = az.rem_euclid(360.0);
    let i = (a / 45.0).floor() as usize % 8;
    let f = a / 45.0 - (a / 45.0).floor();
    let row = &SOLAR_JULY[k];
    row[i] + (row[(i + 1) % 8] - row[i]) * f
}

/// The largest hourly I₀ for the azimuth.
pub fn solar_max(az: f64) -> f64 {
    (0..SOLAR_HOURS.len())
        .map(|k| solar_at(az, k))
        .fold(0.0, f64::max)
}

/// The compass point an azimuth is nearest to, as Slovak drawings write
/// it.
pub fn compass(az: f64) -> &'static str {
    ["S", "SV", "V", "JV", "J", "JZ", "Z", "SZ"][(az.rem_euclid(360.0) / 45.0).round() as usize % 8]
}

/// What a room is, for its gains inside: [`RoomType`]'s table.
pub use creabim_core::roomtype::Gains as Use;
use creabim_core::roomtype::RoomType;

/// A room's use from a text naming it (its name or occupancy).
pub fn use_of(text: &str) -> Use {
    RoomType::find(text)
        .unwrap_or(RoomType::Other)
        .needs()
        .gains
}

/// One part of a room's load.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// What it is, in English: the app translates it.
    pub kind: &'static str,
    pub note: String,
    /// W.
    pub phi: f64,
}

/// One room's cooling load.
#[derive(Clone, Debug, PartialEq)]
pub struct RoomLoad {
    pub id: ElementId,
    pub number: String,
    pub name: String,
    pub area: f64,
    /// Living rooms, kitchens and offices are cooled.
    pub cooled: bool,
    pub items: Vec<Item>,
    /// W.
    pub phi: f64,
    /// The hour the sun's load through its windows is largest.
    pub hour: Option<u32>,
    /// The fan-coil of the range that covers it, kW.
    pub fancoil_kw: f64,
    /// The room uncooled on a summer day.
    pub summer: Summer,
}

/// Summer stability of a room without cooling, as PilonDesigner's
/// `roomSummer`: roughly, by the admittance method (CIBSE Guide A, the
/// principle of STN EN ISO 13792). The day's mean from the balance of
/// the mean gains and the loss H, the swing from the peaks damped by the
/// heat the inner surfaces store.
#[derive(Clone, Debug, PartialEq)]
pub struct Summer {
    /// The day's highest and mean room air temperature, °C.
    pub t_max: f64,
    pub t_mean: f64,
    /// `t_max` within [`Settings::summer_tmax`].
    pub ok: bool,
    /// What the figures came from.
    pub note: String,
}

/// Every room's cooling load.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuildingLoad {
    pub rooms: Vec<RoomLoad>,
    /// Of the cooled rooms, W: each at its own worst hour.
    pub phi: f64,
    pub area: f64,
    /// Chilled water for it, l/s.
    pub flow_ls: f64,
}

/// Fan-coil units' sensible cooling output at 7/12 °C and middle
/// speed, kW.
pub const FANCOIL_SIZES: [f64; 7] = [1.2, 1.8, 2.5, 3.2, 4.0, 5.0, 6.5];

/// The smallest fan-coil that gives `kw` (the largest when none does).
pub fn fancoil_pick(kw: f64) -> f64 {
    FANCOIL_SIZES
        .iter()
        .copied()
        .find(|x| *x >= kw - 1e-9)
        .unwrap_or(FANCOIL_SIZES[FANCOIL_SIZES.len() - 1])
}

fn one(hl: &RoomLoss, d_u: f64, s: &Settings, perimeter: f64) -> RoomLoad {
    let sc = s.g / G_SINGLE;
    let fc = s.shading.fc();
    let usage = hl.kind.needs().gains;
    let mut hourly = [0.0; 15];
    let (mut glazed, mut faces): (f64, Vec<&'static str>) = (0.0, vec![]);
    let (mut win_k, mut win_a, mut wall_k, mut wall_a, mut roof_k, mut roof_a) =
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for i in &hl.items {
        // the heat loss adds the thermal bridges' ΔU; summer does not
        let u = (i.u - d_u).max(0.0);
        match i.kind {
            "Window" | "Outside door" => {
                win_k += u * i.area;
                win_a += i.area;
                if let ("Window", Some(az)) = (i.kind, i.azimuth) {
                    let ag = i.area * s.glazed;
                    glazed += ag;
                    for (k, h) in hourly.iter_mut().enumerate() {
                        *h += ag * solar_at(az, k);
                    }
                    if !faces.contains(&compass(az)) {
                        faces.push(compass(az));
                    }
                }
            }
            "Outside wall" => {
                wall_k += u * i.area;
                wall_a += i.area;
            }
            "Roof" => {
                roof_k += u * i.area;
                roof_a += i.area;
            }
            _ => {}
        }
    }
    // the load is the highest of the hours the building is used: the sun
    // of that hour and the outside air as warm as it is then
    let gains = |k: usize| {
        let d = s.te_at(SOLAR_HOURS[k] as f64) - s.ti;
        hourly[k] * sc * fc * s.storage
            + (win_k + 0.34 * hl.v_min) * d
            + wall_k * (d + s.dt_wall)
            + roof_k * (d + s.dt_roof)
    };
    let used: Vec<usize> = (0..15)
        .filter(|k| (s.from_h..=s.to_h).contains(&SOLAR_HOURS[*k]))
        .collect();
    let used = if used.is_empty() {
        (0..15).collect()
    } else {
        used
    };
    let k = used
        .iter()
        .copied()
        .max_by(|a, b| gains(*a).total_cmp(&gains(*b)))
        .unwrap_or(7);
    let dt = s.te_at(SOLAR_HOURS[k] as f64) - s.ti;
    let win = win_k * dt;
    let wall = wall_k * (dt + s.dt_wall);
    let roof = roof_k * (dt + s.dt_roof);
    let sun = hourly[k] * sc * fc * s.storage;
    let mut items = vec![];
    let mut add = |kind: &'static str, phi: f64, note: String| {
        if phi > 0.5 {
            items.push(Item { kind, note, phi });
        }
    };
    add(
        "Sun through the windows",
        sun,
        format!(
            "{glazed:.2} m² ({}) · {} h · ΣA·I₀ {:.0} W · SC {sc:.2} · Fc {fc:.2}",
            faces.join(", "),
            SOLAR_HOURS[k],
            hourly[k]
        ),
    );
    add(
        "Through windows and doors",
        win,
        format!("{win_a:.2} m² · Δθ {dt:.0} K"),
    );
    add(
        "Through outside walls",
        wall,
        format!("{wall_a:.2} m² · Δθ {dt:.0} + {:.0} K", s.dt_wall),
    );
    add(
        "Through the roof",
        roof,
        format!("{roof_a:.2} m² · Δθ {dt:.0} + {:.0} K", s.dt_roof),
    );
    let office = usage == Use::Office;
    let per = if office { s.m2_office } else { s.m2_home };
    let persons = (hl.area / per.max(1.0)).round().max(1.0);
    add(
        "People",
        persons * s.w_person,
        format!("{persons:.0} × {:.0} W", s.w_person),
    );
    add("Lighting", hl.area * s.light, format!("{} W/m²", s.light));
    let eq = match usage {
        Use::Office => s.eq_office,
        Use::Kitchen => s.eq_kitchen,
        _ => s.eq_home,
    };
    add("Appliances", hl.area * eq, format!("{eq} W/m²"));
    add(
        "Ventilation air",
        0.34 * hl.v_min * dt,
        format!("{:.0} m³/h · Δθ {dt:.0} K", hl.v_min),
    );
    let phi: f64 = items.iter().map(|x| x.phi).sum();
    let summer = summer(hl, d_u, s, perimeter, &hourly, persons, eq, office);
    RoomLoad {
        id: hl.id,
        number: hl.number.clone(),
        name: hl.name.clone(),
        area: hl.area,
        cooled: usage != Use::Service,
        items,
        phi,
        hour: (glazed > 0.0).then_some(SOLAR_HOURS[k]),
        fancoil_kw: fancoil_pick(phi / 1000.0),
        summer,
    }
}

/// The 24 hours' mean of the sun through the windows, W, from the hours
/// of [`SOLAR_JULY`] (the night has none).
fn day_mean(hourly: &[f64; 15]) -> f64 {
    hourly.iter().sum::<f64>() / 24.0
}

#[allow(clippy::too_many_arguments)]
fn summer(
    hl: &RoomLoss,
    d_u: f64,
    s: &Settings,
    perimeter: f64,
    hourly: &[f64; 15],
    persons: f64,
    eq: f64,
    office: bool,
) -> Summer {
    let sc = s.g / G_SINGLE;
    let fc = s.shading.fc();
    let (mut win_ua, mut opaque_ua, mut roof_ua, mut ground_a, mut windows) =
        (0.0, 0.0, 0.0, 0.0, false);
    for i in &hl.items {
        let u = (i.u - d_u).max(0.0);
        match i.kind {
            "Window" | "Outside door" => {
                win_ua += u * i.area;
                windows |= i.kind == "Window";
            }
            "Outside wall" => opaque_ua += u * i.area,
            "Roof" => roof_ua += u * i.area,
            "Floor on the ground" => ground_a += i.area,
            _ => {}
        }
    }
    let sun: Vec<f64> = hourly.iter().map(|x| x * sc * fc).collect();
    let sun_peak = sun.iter().copied().fold(0.0, f64::max);
    let sun_mean = sun.iter().sum::<f64>() / 24.0;
    debug_assert!((day_mean(hourly) * sc * fc - sun_mean).abs() < 1e-6);
    let a = hl.area.max(0.01);
    let h = if hl.area > 0.0 {
        hl.volume / hl.area
    } else {
        2.6
    };
    let v = hl.volume.max(1.0);
    let n = (if windows { s.summer_ach } else { 0.0 }).max(hl.v_min / v);
    let hv = 0.34 * n * v;
    let ground = s.ground_h * ground_a;
    let hh = (win_ua + opaque_ua + roof_ua + hv + ground).max(0.01);
    let amp = s.swing / 2.0;
    let te_mean = s.te - amp;
    let roof_sol = roof_ua * s.dt_roof;
    let int_peak = persons * s.w_person + a * eq;
    let int_mean = if office {
        (persons * s.w_person + a * (s.light + eq)) * 0.35
    } else {
        a * eq
    };
    let t_mean =
        (te_mean * (hh - ground) + s.t_ground * ground + sun_mean + int_mean + roof_sol * 0.5) / hh;
    let y = s.mass.admittance();
    let ya = y * (2.0 * a + perimeter.max(0.0) * h);
    let swing =
        (sun_peak - sun_mean) + (int_peak - int_mean) + roof_sol * 0.5 + (win_ua + hv) * amp;
    let t_max = t_mean + swing / (ya + win_ua + hv).max(0.01);
    Summer {
        t_max,
        t_mean,
        ok: t_max <= s.summer_tmax + 1e-9,
        note: format!(
            "θe {:.1} °C (mean {te_mean:.1}) · sun peak {sun_peak:.0} W · inside peak {int_peak:.0} W · H {hh:.0} W/K (air {:.1} 1/h{}) · Y {y:.1} W/(m²K) · YA {ya:.0} W/K",
            s.te,
            n,
            if ground > 0.0 { ", floor on the ground" } else { "" },
        ),
    }
}

/// What would bring a room within the summer limit: outside shading,
/// then also solar glass g 0.3; else it needs cooling or night
/// ventilation. In English: the app translates it.
pub fn summer_remedy(
    model: &Model,
    o: &heatloss::Opts,
    s: &Settings,
    room: ElementId,
) -> &'static str {
    let with = |x: Settings| {
        building(model, o, &x)
            .rooms
            .into_iter()
            .find(|r| r.id == room)
            .map(|r| r.summer)
    };
    if s.shading != Shading::External {
        if let Some(a) = with(Settings {
            shading: Shading::External,
            ..*s
        }) {
            if a.ok {
                return "Outside shading is enough";
            }
        }
    }
    if s.shading != Shading::External || s.g > 0.3 {
        if let Some(b) = with(Settings {
            shading: Shading::External,
            g: s.g.min(0.3),
            ..*s
        }) {
            if b.ok {
                return "Outside shading and solar glass g 0.3";
            }
        }
    }
    "Needs cooling or night ventilation"
}

/// The cooling load of every room of the model.
pub fn building(model: &Model, o: &heatloss::Opts, s: &Settings) -> BuildingLoad {
    let losses = heatloss::building(model, o);
    let mut out = BuildingLoad {
        rooms: losses
            .rooms
            .iter()
            .map(|hl| {
                let per = creabim_core::room::outline(model, hl.id)
                    .map_or(4.0 * hl.area.max(0.0).sqrt(), |p| {
                        creabim_core::room::perimeter(&p)
                    });
                one(hl, o.d_u, s, per)
            })
            .collect(),
        ..BuildingLoad::default()
    };
    for r in out.rooms.iter().filter(|r| r.cooled) {
        out.phi += r.phi;
        out.area += r.area;
    }
    out.flow_ls = out.phi / (4186.0 * (s.return_c - s.supply_c).max(1.0));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sun_by_where_a_window_faces() {
        assert_eq!(solar_max(0.0), 141.0);
        assert_eq!(solar_max(45.0), 361.0);
        assert_eq!(solar_max(90.0), 539.0);
        assert_eq!(solar_max(135.0), 511.0);
        assert_eq!(solar_max(180.0), 435.0);
        assert_eq!(solar_max(270.0), 539.0);
        assert_eq!(solar_max(360.0), 141.0);
        // south at noon
        assert_eq!(solar_at(180.0, 7), 435.0);
        assert!((solar_at(67.5, 3) - (321.0 + 539.0) / 2.0).abs() < 1e-9);
        assert_eq!(compass(0.0), "S");
        assert_eq!(compass(180.0), "J");
        assert_eq!(compass(275.0), "Z");
        assert_eq!(compass(350.0), "S");
    }

    #[test]
    fn rooms_by_their_use_and_fan_coils() {
        assert_eq!(use_of("Obývacia izba"), Use::Home);
        assert_eq!(use_of("Kuchyňa"), Use::Kitchen);
        assert_eq!(use_of("Kancelária 2"), Use::Office);
        assert_eq!(use_of("Kúpeľňa"), Use::Service);
        assert_eq!(use_of("Chodba"), Use::Service);
        assert_eq!(fancoil_pick(1.9), 2.5);
        assert_eq!(fancoil_pick(9.0), 6.5);
    }

    #[test]
    fn the_sample_house_has_a_sensible_load_and_shading_lowers_it() {
        use creabim_core::{Document, Element, ElementKind, Room, Transaction};
        use glam::DVec2;
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
        let b = building(doc.model(), &o, &Settings::default());
        assert_eq!(b.rooms.len(), 2);
        let (bath, living) = (&b.rooms[0], &b.rooms[1]);
        assert!(!bath.cooled && living.cooled);
        let per_m2 = living.phi / living.area;
        assert!(per_m2 > 20.0 && per_m2 < 150.0, "{per_m2} {living:#?}");
        assert!(living.hour.is_some());
        assert!(living
            .items
            .iter()
            .any(|i| i.kind == "Sun through the windows"));
        assert_eq!(b.phi, living.phi);
        assert!(b.flow_ls > 0.0);
        let shaded = building(
            doc.model(),
            &o,
            &Settings {
                shading: Shading::External,
                ..Settings::default()
            },
        );
        assert!(shaded.rooms[1].phi < living.phi);
        // summer stability: a sensible day, shading cools it, a light
        // building swings more
        let su = &living.summer;
        assert!(su.t_mean > 20.0 && su.t_mean < 35.0, "{su:?}");
        assert!(su.t_max > su.t_mean && su.t_max < 45.0, "{su:?}");
        assert!(shaded.rooms[1].summer.t_max < su.t_max);
        let light = building(
            doc.model(),
            &o,
            &Settings {
                mass: Mass::Light,
                ..Settings::default()
            },
        );
        assert!(light.rooms[1].summer.t_max > su.t_max);
        let _ = summer_remedy(doc.model(), &o, &Settings::default(), living.id);
    }

    #[test]
    fn a_facade_faces_where_its_normal_points() {
        use glam::DVec2;
        assert_eq!(heatloss::azimuth(DVec2::Y, 0.0), 0.0);
        assert_eq!(heatloss::azimuth(DVec2::X, 0.0), 90.0);
        assert_eq!(heatloss::azimuth(-DVec2::Y, 0.0), 180.0);
        assert_eq!(heatloss::azimuth(-DVec2::X, 0.0), 270.0);
        // north turned 90° anticlockwise: the drawing's up looks east
        assert_eq!(heatloss::azimuth(DVec2::Y, 90.0), 90.0);
    }
}
