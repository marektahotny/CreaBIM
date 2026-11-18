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
    /// Chilled water, °C.
    pub supply_c: f64,
    pub return_c: f64,
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
            supply_c: 7.0,
            return_c: 12.0,
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

/// What a room is, for its gains inside.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Use {
    Home,
    Kitchen,
    Office,
    /// A bathroom, a corridor, a store: not cooled.
    Service,
}

/// A room's use from its name and occupancy.
pub fn use_of(text: &str) -> Use {
    let t = text.to_lowercase();
    let has = |keys: &[&str]| keys.iter().any(|k| t.contains(k));
    if has(&[
        "kúpeľ", "kupel", "wc", "toalet", "sprch", "bath", "shower", "chodb", "hala", "corridor",
        "hall", "zádver", "zadver", "garáž", "garaz", "garage", "sklad", "store", "komor", "techn",
        "kotol", "boiler", "plant", "schod", "stair", "pivni", "cellar", "práč", "laundry", "šatn",
    ]) {
        Use::Service
    } else if has(&["kancel", "office", "zasada", "meeting"]) {
        Use::Office
    } else if has(&["kuchy", "kitchen"]) {
        Use::Kitchen
    } else {
        Use::Home
    }
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

fn one(hl: &RoomLoss, d_u: f64, s: &Settings) -> RoomLoad {
    let dt = s.te - s.ti;
    let sc = s.g / G_SINGLE;
    let fc = s.shading.fc();
    let usage = use_of(&hl.name);
    let mut hourly = [0.0; 15];
    let (mut glazed, mut faces): (f64, Vec<&'static str>) = (0.0, vec![]);
    let (mut win, mut win_a, mut wall, mut wall_a, mut roof, mut roof_a) =
        (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for i in &hl.items {
        // the heat loss adds the thermal bridges' ΔU; summer does not
        let u = (i.u - d_u).max(0.0);
        match i.kind {
            "Window" | "Outside door" => {
                win += u * i.area * dt;
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
                wall += u * i.area * (dt + s.dt_wall);
                wall_a += i.area;
            }
            "Roof" => {
                roof += u * i.area * (dt + s.dt_roof);
                roof_a += i.area;
            }
            _ => {}
        }
    }
    let k = (0..15)
        .max_by(|a, b| hourly[*a].total_cmp(&hourly[*b]))
        .unwrap_or(7);
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
    }
}

/// The cooling load of every room of the model.
pub fn building(model: &Model, o: &heatloss::Opts, s: &Settings) -> BuildingLoad {
    let losses = heatloss::building(model, o);
    let mut out = BuildingLoad {
        rooms: losses.rooms.iter().map(|hl| one(hl, o.d_u, s)).collect(),
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
