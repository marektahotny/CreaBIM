//! The rooms' cooling load, as PilonDesigner's `coolingload.ts`: a
//! simplified calculation after STN 73 0548. Sun through the windows by
//! where they face (21 July, 50° north) with the glazing's g and the
//! shading, heat through walls and roof with an added temperature
//! difference for the sun on them, through windows and doors, the gains
//! inside (people, lighting, appliances) and ventilation. The geometry
//! and the U values are those of the heat loss
//! ([`crate::heatloss`]). The result sizes fan-coil units and the
//! source of cold.

use crate::coolnorm;
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

/// How the load is calculated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Method {
    /// PilonDesigner's: the sun on walls and roofs as an added Δθ, the
    /// windows' sun cut by a storage factor.
    #[default]
    Simplified,
    /// STN 73 0548 itself ([`crate::coolnorm`]): sol-air temperatures
    /// with ε, damped and delayed by the wall's thickness, the shading s
    /// of table 11 and the room's storing mass.
    Norm,
}

impl Method {
    pub const ALL: [Method; 2] = [Method::Simplified, Method::Norm];

    pub fn name(self) -> &'static str {
        match self {
            Method::Simplified => "Simplified",
            Method::Norm => "By STN 73 0548",
        }
    }
}

/// The ε of table 12 taken when none is picked: a light render, a fired
/// roof tile (keys of the norm's parameter file).
pub const EPS_WALL_KEY: &str = "omietka_svetla";
pub const EPS_ROOF_KEY: &str = "palena_krytina";
/// The rows of table 6 for people in a home or an office, and cooking.
pub const PEOPLE_SEATED: &str = "mierne_aktivny";
pub const PEOPLE_COOKING: &str = "lahka_praca";

/// How clear the air is where the building stands (STN 73 0548, art.
/// 64): the sun of table 10 is for the average.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Region {
    #[default]
    Average,
    Rural,
    Urban,
}

impl Region {
    pub const ALL: [Region; 3] = [Region::Average, Region::Rural, Region::Urban];

    pub fn name(self) -> &'static str {
        match self {
            Region::Average => "Average (c_o 1)",
            Region::Rural => "Countryside",
            Region::Urban => "Industrial area, large town",
        }
    }

    /// Its row of the norm's factors.
    fn key(self) -> Option<&'static str> {
        match self {
            Region::Average => None,
            Region::Rural => Some("c_o_vidiek"),
            Region::Urban => Some("c_o_mesto"),
        }
    }
}

/// What the load is calculated with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    pub method: Method,
    /// [`Method::Norm`]: the rows of table 12 (ε) picked for the outside
    /// walls and the roof (none: [`EPS_WALL_KEY`], [`EPS_ROOF_KEY`]), the
    /// row of table 11 for the windows' shading (none: no shading), and
    /// the room's allowed rise Δt of formula 27, K (none: the table's).
    pub eps_wall: Option<usize>,
    pub eps_roof: Option<usize>,
    pub shade: Option<usize>,
    pub dt_allow: Option<f64>,
    /// [`Method::Norm`]: the month (3 to 10) of the sun (tables 5, 10)
    /// and of the outside air (table 1, formula 1), the clearness of the
    /// air where the building stands (art. 64), and the windows' reveals
    /// shading their glass (formulas 24 to 26).
    pub month: u8,
    pub region: Region,
    pub reveal: bool,
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
            method: Method::Simplified,
            eps_wall: None,
            eps_roof: None,
            shade: None,
            dt_allow: None,
            month: 7,
            region: Region::Average,
            reveal: true,
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
    /// The day's mean gains with the outside and the ground at their
    /// temperatures, W, over the loss H, W/K: `t_mean` = gain / H before
    /// the rooms next door join in.
    pub(crate) gain: f64,
    pub(crate) h: f64,
    /// `t_max` − `t_mean`, K.
    pub(crate) rise: f64,
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

/// What [`Method::Norm`] works with: the norm's values picked and the
/// thickness of the walls and roofs by their build-up's name.
pub(crate) struct Norm {
    pub tables: Vec<creabim_core::fireparams::ParamTable>,
    pub eps_wall: Option<f64>,
    pub eps_roof: Option<f64>,
    /// s of table 11, 1 with no shading.
    pub shade: f64,
    pub alpha_e: Option<f64>,
    pub k_acc: Option<f64>,
    pub dt_allow: Option<f64>,
    pub thick: Vec<(String, f64)>,
    pub month: u8,
    /// t_e,max of the month (table 1) and A (formula 1).
    pub te_max: Option<f64>,
    pub amplitude: Option<f64>,
    /// c_o of art. 64 (1 for the average air).
    pub c_o: f64,
    pub lat: Option<f64>,
    pub reveal: bool,
    /// Table 10 holds the month.
    pub glass: bool,
    pub windows: Vec<(ElementId, coolnorm::Win)>,
}

impl Norm {
    pub(crate) fn of(model: &Model, s: &Settings) -> Norm {
        let tables = coolnorm::tables_of(model);
        let eps = coolnorm::rows(&tables, coolnorm::EPS, "eps");
        let shades = coolnorm::rows(&tables, coolnorm::SHADE, "s");
        Norm {
            eps_wall: coolnorm::picked(&eps, s.eps_wall, EPS_WALL_KEY).map(|r| r.value),
            eps_roof: coolnorm::picked(&eps, s.eps_roof, EPS_ROOF_KEY).map(|r| r.value),
            shade: s.shade.and_then(|i| shades.get(i)).map_or(1.0, |r| r.value),
            alpha_e: coolnorm::factor(&tables, "alpha_e"),
            k_acc: coolnorm::factor(&tables, "k_acc"),
            dt_allow: s.dt_allow.or_else(|| coolnorm::factor(&tables, "dt_allow")),
            thick: coolnorm::thickness_by_name(model),
            month: s.month,
            te_max: coolnorm::te_max(&tables, s.month),
            amplitude: coolnorm::factor(&tables, "amplitude"),
            c_o: s
                .region
                .key()
                .and_then(|k| coolnorm::factor(&tables, k))
                .unwrap_or(1.0),
            lat: coolnorm::factor(&tables, "latitude"),
            reveal: s.reveal,
            glass: coolnorm::sun_of(&tables, coolnorm::SUN_GLASS, s.month, Some(180.0), 12)
                .is_some(),
            windows: coolnorm::windows(model),
            tables,
        }
    }

    /// The outside air at hour `h` by formula 1, if table 1 and A are
    /// read.
    fn te(&self, h: f64) -> Option<f64> {
        Some(coolnorm::te_at(self.te_max?, self.amplitude?, h))
    }

    fn delta(&self, name: &str) -> Option<f64> {
        // a ceiling's name carries the roof space's Ru after its build-up's
        let name = name.split(" + Ru ").next().unwrap_or(name);
        self.thick.iter().find(|x| x.0 == name).map(|x| x.1)
    }
}

fn one(
    hl: &RoomLoss,
    d_u: f64,
    s: &Settings,
    perimeter: f64,
    norm: Option<(&Norm, &coolnorm::Storing)>,
) -> RoomLoad {
    let sc = s.g / G_SINGLE;
    let usage = hl.kind.needs().gains;
    let mut hourly = [0.0; 15];
    let (mut glazed, mut faces): (f64, Vec<&'static str>) = (0.0, vec![]);
    let (mut win_k, mut win_a, mut wall_k, mut wall_a, mut roof_k, mut roof_a) =
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    // the outside walls and roofs one by one: k·S, where they face, their
    // build-up
    let mut opaque: Vec<(f64, Option<f64>, &str)> = vec![];
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
                opaque.push((u * i.area, i.azimuth, &i.detail));
            }
            // a ceiling under a roof space gains as a roof does
            "Roof" | "Ceiling" => {
                roof_k += u * i.area;
                roof_a += i.area;
                opaque.push((u * i.area, None, &i.detail));
            }
            _ => {}
        }
    }
    let used: Vec<usize> = (0..15)
        .filter(|k| (s.from_h..=s.to_h).contains(&SOLAR_HOURS[*k]))
        .collect();
    let used = if used.is_empty() {
        (0..15).collect()
    } else {
        used
    };
    let te_at = |h: f64| {
        norm.and_then(|(n, _)| n.te(h))
            .unwrap_or_else(|| s.te_at(h))
    };
    let air = |k: usize| te_at(SOLAR_HOURS[k] as f64) - s.ti;
    // STN 73 0548: the sun through each window of the month (table 10),
    // its sunlit glass in full and the rest by the diffuse (formula 23)
    let by_norm = norm.filter(|(n, _)| n.glass).map(|(n, _)| {
        let mut out = [0.0; 15];
        let mut area = 0.0;
        for (_, w) in n.windows.iter().filter(|x| x.0 == hl.id) {
            let so = w.la * w.lb;
            area += so;
            for (k, q) in out.iter_mut().enumerate() {
                let h = SOLAR_HOURS[k] as i32;
                let i = coolnorm::sun_of(&n.tables, coolnorm::SUN_GLASS, n.month, Some(w.az), h)
                    .unwrap_or(0.0);
                let dif = coolnorm::sun_of(&n.tables, coolnorm::SUN_GLASS, n.month, Some(0.0), h)
                    .unwrap_or(0.0)
                    .min(i);
                let sos = match (n.reveal, n.lat) {
                    (true, Some(lat)) => {
                        let (sh, sa) = coolnorm::sun_position(n.month, h as f64, lat);
                        coolnorm::sunlit(w.la, w.lb, w.depth, w.az, sh, sa)
                    }
                    _ => so,
                };
                *q += sos * i * n.c_o + (so - sos) * dif;
            }
        }
        (out, area)
    });
    let (hourly_load, glazed) = match by_norm {
        Some((h, a)) => (h, a),
        None => (hourly, glazed),
    };
    // the sun through the windows of each hour, W
    let fc = match norm {
        Some((n, _)) => n.shade,
        None => s.shading.fc(),
    };
    let sun_raw: Vec<f64> = (0..15).map(|k| hourly_load[k] * sc * fc).collect();
    // what the room stores when it may get warmer (formula 27), not below
    // the mean over the hours of use (28)
    let stored = norm.and_then(|(n, m)| Some(n.k_acc? * m.kg * n.dt_allow?));
    let sun_mean = used.iter().map(|k| sun_raw[*k]).sum::<f64>() / used.len() as f64;
    let sun_at = |k: usize| match (norm, stored) {
        (Some(_), Some(dq)) => (sun_raw[k] - dq).max(sun_mean).min(sun_raw[k]),
        (Some(_), None) => sun_raw[k],
        _ => sun_raw[k] * s.storage,
    };
    // walls and roofs: the simplified added Δθ, or formula 30 for each
    let sol = norm.map(|(n, _)| {
        opaque
            .iter()
            .map(|(ks, az, name)| {
                let eps = if az.is_some() { n.eps_wall } else { n.eps_roof };
                let tr = coolnorm::sol_air(
                    te_at,
                    |h| {
                        coolnorm::sun_of(&n.tables, coolnorm::SUN_WALL, n.month, *az, h)
                            .unwrap_or(0.0)
                    },
                    eps.unwrap_or(0.0),
                    n.alpha_e.unwrap_or(f64::INFINITY),
                );
                // a build-up of unknown thickness as a light one (formula 29')
                (*ks, az.is_none(), tr, n.delta(name).unwrap_or(0.0))
            })
            .collect::<Vec<_>>()
    });
    let opaque_at = |k: usize| -> (f64, f64) {
        match &sol {
            Some(list) => {
                let h = SOLAR_HOURS[k] as f64;
                list.iter().fold((0.0, 0.0), |(w, r), (ks, roof, tr, d)| {
                    let q = ks * coolnorm::wall_dt(tr, s.ti, *d, h);
                    if *roof {
                        (w, r + q)
                    } else {
                        (w + q, r)
                    }
                })
            }
            None => (wall_k * (air(k) + s.dt_wall), roof_k * (air(k) + s.dt_roof)),
        }
    };
    // the load is the highest of the hours the building is used: the sun
    // of that hour and the outside air as warm as it is then
    let gains = |k: usize| {
        let (w, r) = opaque_at(k);
        sun_at(k) + (win_k + 0.34 * hl.v_min) * air(k) + w + r
    };
    let k = used
        .iter()
        .copied()
        .max_by(|a, b| gains(*a).total_cmp(&gains(*b)))
        .unwrap_or(7);
    let dt = air(k);
    let win = win_k * dt;
    let (wall, roof) = opaque_at(k);
    let sun = sun_at(k);
    let mut items = vec![];
    let mut add = |kind: &'static str, phi: f64, note: String| {
        if phi > 0.5 {
            items.push(Item { kind, note, phi });
        }
    };
    let sun_note = match (norm, stored) {
        (Some((_, m)), Some(dq)) => format!(
            " · s {fc:.2} · M {:.0} kg → ΔQ {dq:.0} W{} · Q {:.0} W",
            m.kg,
            if m.unknown > 0 {
                format!(" ({} ?)", m.unknown)
            } else {
                String::new()
            },
            sun_raw[k],
        ),
        (Some(_), None) => format!(" · s {fc:.2}"),
        _ => format!(" · Fc {fc:.2}"),
    };
    add(
        "Sun through the windows",
        sun,
        format!(
            "{glazed:.2} m² ({}) · {} h · ΣA·I₀ {:.0} W · SC {sc:.2}{sun_note}",
            faces.join(", "),
            SOLAR_HOURS[k],
            hourly_load[k]
        ),
    );
    add(
        "Through windows and doors",
        win,
        format!("{win_a:.2} m² · Δθ {dt:.0} K"),
    );
    let opaque_note = |a: f64, roof: bool, extra: f64| match (&sol, norm) {
        (Some(list), Some((n, _))) => {
            let ds: Vec<String> = list
                .iter()
                .filter(|x| x.1 == roof)
                .map(|x| {
                    format!(
                        "δ {:.2} m ψ {:.1} h m {:.2}",
                        x.3,
                        coolnorm::psi(x.3),
                        coolnorm::damping(x.3)
                    )
                })
                .fold(vec![], |mut v, x| {
                    if !v.contains(&x) {
                        v.push(x);
                    }
                    v
                });
            let eps = if roof { n.eps_roof } else { n.eps_wall };
            format!(
                "{a:.2} m² · ε {} · {}",
                eps.map_or("?".into(), |e| format!("{e:.2}")),
                ds.join(", ")
            )
        }
        _ => format!("{a:.2} m² · Δθ {dt:.0} + {extra:.0} K"),
    };
    add(
        "Through outside walls",
        wall,
        opaque_note(wall_a, false, s.dt_wall),
    );
    add(
        "Through the roof",
        roof,
        opaque_note(roof_a, true, s.dt_roof),
    );
    let office = usage == Use::Office;
    let per = if office { s.m2_office } else { s.m2_home };
    let persons = (hl.area / per.max(1.0)).round().max(1.0);
    // STN 73 0548: a man's sensible heat of table 6 at the room's air
    let activity = if usage == Use::Kitchen {
        PEOPLE_COOKING
    } else {
        PEOPLE_SEATED
    };
    let w_person = norm
        .and_then(|(n, _)| coolnorm::person_w(&n.tables, activity, s.ti))
        .unwrap_or(s.w_person);
    add(
        "People",
        persons * w_person,
        format!("{persons:.0} × {w_person:.0} W"),
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
            "Roof" | "Ceiling" => roof_ua += u * i.area,
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
    let gain = te_mean * (hh - ground) + s.t_ground * ground + sun_mean + int_mean + roof_sol * 0.5;
    let t_mean = gain / hh;
    let y = s.mass.admittance();
    let ya = y * (2.0 * a + perimeter.max(0.0) * h);
    let swing =
        (sun_peak - sun_mean) + (int_peak - int_mean) + roof_sol * 0.5 + (win_ua + hv) * amp;
    let rise = swing / (ya + win_ua + hv).max(0.01);
    let t_max = t_mean + rise;
    Summer {
        t_max,
        t_mean,
        gain,
        h: hh,
        rise,
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

/// The summer's mean temperatures with the rooms passing heat to each
/// other through the inner walls and floors between them: each room
/// balances its gains against the outside and the rooms next door
/// (solved over and over until it settles). A room with no window
/// would else keep its inner gains to itself and run hot.
fn neighbours(model: &Model, s: &Settings, rooms: &mut [RoomLoad]) {
    let at = |id: ElementId| rooms.iter().position(|r| r.id == id);
    let links: Vec<(usize, usize, f64)> = heatloss::partitions(model)
        .into_iter()
        .filter_map(|(a, b, h)| Some((at(a)?, at(b)?, h)))
        .collect();
    if links.is_empty() {
        return;
    }
    let mut t: Vec<f64> = rooms.iter().map(|r| r.summer.t_mean).collect();
    for _ in 0..500 {
        let mut most: f64 = 0.0;
        for i in 0..rooms.len() {
            let (mut g, mut q) = (rooms[i].summer.h, rooms[i].summer.gain);
            for &(a, b, h) in &links {
                let j = if a == i {
                    b
                } else if b == i {
                    a
                } else {
                    continue;
                };
                g += h;
                q += h * t[j];
            }
            let x = q / g.max(0.01);
            most = most.max((x - t[i]).abs());
            t[i] = x;
        }
        if most < 1e-4 {
            break;
        }
    }
    for (i, r) in rooms.iter_mut().enumerate() {
        let next: f64 = links
            .iter()
            .filter(|l| l.0 == i || l.1 == i)
            .map(|l| l.2)
            .sum();
        if next <= 0.0 {
            continue;
        }
        let m = &mut r.summer;
        m.t_mean = t[i];
        m.t_max = t[i] + m.rise;
        m.ok = m.t_max <= s.summer_tmax + 1e-9;
        m.note
            .push_str(&format!(" · rooms next door {next:.0} W/K"));
    }
}

/// STN 73 0548 formula 20: the heat from a room next door that is not
/// cooled, Q = k·S·(t_is − t_i), its t_is the highest it reaches in
/// summer on its own ([`Summer::t_max`]). A cooler one gives nothing.
fn next_door(model: &Model, s: &Settings, rooms: &mut [RoomLoad]) {
    let links = heatloss::partitions(model);
    let warm: Vec<(ElementId, String, f64)> = rooms
        .iter()
        .filter(|r| !r.cooled)
        .map(|r| {
            (
                r.id,
                format!("{} {}", r.number, r.name).trim().to_string(),
                r.summer.t_max,
            )
        })
        .collect();
    for r in rooms.iter_mut().filter(|r| r.cooled) {
        for (a, b, h) in &links {
            let other = if *a == r.id {
                b
            } else if *b == r.id {
                a
            } else {
                continue;
            };
            let Some((_, name, t)) = warm.iter().find(|w| w.0 == *other) else {
                continue;
            };
            let q = h * (t - s.ti);
            if q > 0.5 {
                r.items.push(Item {
                    kind: "From the room next door",
                    note: format!("{name} · θ {t:.1} °C · k·S {h:.1} W/K"),
                    phi: q,
                });
                r.phi += q;
            }
        }
        r.fancoil_kw = fancoil_pick(r.phi / 1000.0);
    }
}

/// The cooling load of every room of the model.
pub fn building(model: &Model, o: &heatloss::Opts, s: &Settings) -> BuildingLoad {
    let losses = heatloss::building(model, o);
    let norm = (s.method == Method::Norm).then(|| Norm::of(model, s));
    let mut out = BuildingLoad {
        rooms: losses
            .rooms
            .iter()
            .map(|hl| {
                let per = creabim_core::room::outline(model, hl.id)
                    .map_or(4.0 * hl.area.max(0.0).sqrt(), |p| {
                        creabim_core::room::perimeter(&p)
                    });
                let h = if hl.area > 0.0 {
                    hl.volume / hl.area
                } else {
                    2.6
                };
                let m = norm
                    .as_ref()
                    .map(|_| coolnorm::storing(model, hl.id, h))
                    .unwrap_or_default();
                one(hl, o.d_u, s, per, norm.as_ref().map(|n| (n, &m)))
            })
            .collect(),
        ..BuildingLoad::default()
    };
    neighbours(model, s, &mut out.rooms);
    if norm.is_some() {
        next_door(model, s, &mut out.rooms);
    }
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
    fn the_norm_method_reads_its_tables_and_stores_the_sun() {
        let mut doc = creabim_core::Document::from_model(heatloss::test_house());
        // made-up numbers in the norm's tables' shape, not the norm's
        let mut sun = String::from("#table cool_sun_wall\n#columns key");
        for h in 5..=19 {
            sun += &format!("\th{h}");
        }
        sun += "\n";
        for f in ["S", "SV", "V", "JV", "J", "JZ", "Z", "SZ", "HOR"] {
            sun += f;
            for h in 5..=19 {
                sun += &format!("\t{}", if (9..=15).contains(&h) { 400 } else { 100 });
            }
            sun += "\n";
        }
        let text = format!(
            "{sun}#table cool_eps\n#columns key\teps\ttext\nomietka_svetla\t0,5\tsvetlá\npalena_krytina\t0,8\tkrytina\n\
#table cool_shade\n#columns key\ts\ttext\nvonk\t0,2\tvonkajšie\n\
#table cool_factors\n#columns key\tvalue\ttext\nalpha_e\t15\tα\nk_acc\t0,05\tk\ndt_allow\t2\tΔt\n"
        );
        let mut info = doc
            .model()
            .project_info()
            .map(|(_, p)| p.clone())
            .unwrap_or_default();
        info.fire_params = creabim_core::fireparams::parse(&text);
        let tx = creabim_core::set_project(doc.model(), info);
        doc.commit(tx).unwrap();
        let o = heatloss::Opts::default();
        let norm = Settings {
            method: Method::Norm,
            ..Settings::default()
        };
        let b = building(doc.model(), &o, &norm);
        let r = &b.rooms[0];
        let wall = r
            .items
            .iter()
            .find(|i| i.kind == "Through outside walls")
            .unwrap_or_else(|| panic!("{r:#?}"));
        assert!(wall.note.contains("ε 0.50"), "{}", wall.note);
        // a brighter wall takes less sun
        let dark = building(
            doc.model(),
            &o,
            &Settings {
                eps_wall: Some(1),
                ..norm
            },
        );
        let w2 = dark.rooms[0]
            .items
            .iter()
            .find(|i| i.kind == "Through outside walls")
            .unwrap();
        assert!(w2.phi > wall.phi, "{} {}", w2.phi, wall.phi);
        // shading s of table 11 lowers the sun through the windows
        let shaded = building(
            doc.model(),
            &o,
            &Settings {
                shade: Some(0),
                ..norm
            },
        );
        let sun = |b: &BuildingLoad| {
            b.rooms[0]
                .items
                .iter()
                .find(|i| i.kind == "Sun through the windows")
                .map_or(0.0, |i| i.phi)
        };
        assert!(sun(&shaded) < sun(&b), "{} {}", sun(&shaded), sun(&b));
        // no allowed rise: nothing stored, more sun
        let tight = building(
            doc.model(),
            &o,
            &Settings {
                dt_allow: Some(0.0),
                ..norm
            },
        );
        assert!(sun(&tight) >= sun(&b));
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

    #[test]
    fn rooms_next_door_share_the_summer_heat() {
        let m = heatloss::test_house();
        let links = heatloss::partitions(&m);
        assert!(!links.is_empty(), "the two rooms share a wall");
        let s = Settings::default();
        let b = building(&m, &heatloss::Opts::default(), &s);
        // each room's mean now balances against the other: they come
        // closer than on their own, and none runs away
        for r in &b.rooms {
            assert!(r.summer.t_mean < 40.0 && r.summer.t_max < 45.0, "{r:?}");
            assert!(r.summer.note.contains("rooms next door"));
            let own = r.summer.gain / r.summer.h;
            let lo = b
                .rooms
                .iter()
                .map(|x| x.summer.gain / x.summer.h)
                .fold(f64::MAX, f64::min);
            let hi = b
                .rooms
                .iter()
                .map(|x| x.summer.gain / x.summer.h)
                .fold(f64::MIN, f64::max);
            assert!(
                r.summer.t_mean >= lo - 1e-6 && r.summer.t_mean <= hi + 1e-6,
                "{own}"
            );
        }
    }
}
