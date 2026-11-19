//! Heat sources and hot water heating, as PilonDesigner's `equiplib.ts`:
//! an indirectly heated hot water cylinder, a wall-hung condensing gas
//! boiler (system or combination) and a split air-to-water heat pump:
//! its outdoor unit and its indoor one, a wall hydromodule or one with a
//! hot water cylinder built in. The ranges, sizes and connections are
//! those usual on the Slovak and Czech market: values to design with,
//! to be replaced by the maker's own once the product is chosen.
//!
//! Each connection is a small `Connection <code> DN<n>` part where its
//! pipe joins ([`crate::fixture::part_connectors`]).

use super::{appliance_white, cuboid, cylinder, look, metal, v2, v3, Build, Look};
use crate::objects::{CatalogueObject, Mount, ObjectCategory};
use crate::Hatch;

/// Which piece of plant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeatKind {
    /// Indirectly heated hot water cylinder, standing.
    Cylinder,
    /// Condensing gas boiler heating a cylinder.
    Boiler,
    /// Condensing gas boiler with instantaneous hot water.
    BoilerCombi,
    /// Air-to-water heat pump, outdoor unit.
    HpOutdoor,
    /// Heat pump indoor unit: a wall hydromodule.
    HpIndoor,
    /// Heat pump indoor unit with a hot water cylinder built in.
    HpIndoorCylinder,
}

impl HeatKind {
    pub const ALL: [HeatKind; 6] = [
        HeatKind::Cylinder,
        HeatKind::Boiler,
        HeatKind::BoilerCombi,
        HeatKind::HpOutdoor,
        HeatKind::HpIndoor,
        HeatKind::HpIndoorCylinder,
    ];

    /// PilonDesigner's name of the kind.
    pub fn id(self) -> &'static str {
        match self {
            HeatKind::Cylinder => "zasobnik",
            HeatKind::Boiler => "kotol",
            HeatKind::BoilerCombi => "kotolKombi",
            HeatKind::HpOutdoor => "tcVonk",
            HeatKind::HpIndoor => "tcVnut",
            HeatKind::HpIndoorCylinder => "tcVnutZas",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            HeatKind::Cylinder => "Zásobník TV nepriamo ohrievaný (stojatý)",
            HeatKind::Boiler => "Plynový kondenzačný kotol – systémový (s ohrevom zásobníka)",
            HeatKind::BoilerCombi => {
                "Plynový kondenzačný kotol – kombinovaný (prietokový ohrev TV)"
            }
            HeatKind::HpOutdoor => "Tepelné čerpadlo vzduch–voda split – vonkajšia jednotka",
            HeatKind::HpIndoor => "Tepelné čerpadlo – vnútorná jednotka (hydromodul nástenný)",
            HeatKind::HpIndoorCylinder => {
                "Tepelné čerpadlo – vnútorná jednotka s integrovaným zásobníkom TV"
            }
        }
    }

    /// The sizes of its range: litres for a cylinder, else rated kW.
    pub fn sizes(self) -> &'static [u32] {
        match self {
            HeatKind::Cylinder => &[100, 125, 160, 200, 300, 400, 500],
            HeatKind::Boiler => &[12, 16, 20, 25, 30, 35],
            HeatKind::BoilerCombi => &[25, 30, 35],
            _ => &[4, 6, 8, 10, 12, 14, 16],
        }
    }

    /// The unit its sizes are in.
    pub fn unit(self) -> &'static str {
        if self == HeatKind::Cylinder {
            "l"
        } else {
            "kW"
        }
    }

    /// The smallest of the range that covers `need` (the largest when
    /// none does).
    pub fn pick(self, need: f64) -> u32 {
        let s = self.sizes();
        s.iter()
            .copied()
            .find(|x| *x as f64 >= need - 1e-9)
            .unwrap_or(s[s.len() - 1])
    }

    /// `size` if the range has it, else the nearest that covers it.
    pub fn size(self, size: u32) -> u32 {
        if self.sizes().contains(&size) {
            size
        } else {
            self.pick(size as f64)
        }
    }
}

/// A connection: where a pipe joins, in PilonDesigner's axes (x to the
/// right, y to the back, z up; metres from the middle of the footprint).
#[derive(Clone, Debug, PartialEq)]
pub struct HeatPort {
    pub name: &'static str,
    /// The system's code: SV, TV, C, K, UKP, UKS, P (gas); CH
    /// (refrigerant), E (power) and INE (flue) have no pipe to join yet.
    pub code: &'static str,
    pub at: [f64; 3],
    pub dn: u32,
}

/// One piece of plant worked out: its size, connections and data.
#[derive(Clone, Debug, PartialEq)]
pub struct HeatUnit {
    pub kind: HeatKind,
    pub size: u32,
    pub name: String,
    pub ifc: &'static str,
    /// Width, depth and height, m.
    pub dims: [f64; 3],
    pub ports: Vec<HeatPort>,
    /// Its data sheet: name and value with its unit.
    pub params: Vec<(String, String)>,
}

/// Cylinder: diameter, height, coil area m², coil output kW.
fn cylinder_row(size: u32) -> [f64; 4] {
    match size {
        100 => [0.5, 0.9, 0.7, 20.0],
        125 => [0.5, 1.06, 0.8, 23.0],
        160 => [0.6, 1.1, 0.9, 26.0],
        200 => [0.6, 1.3, 1.0, 30.0],
        300 => [0.66, 1.7, 1.4, 37.0],
        400 => [0.75, 1.75, 1.8, 46.0],
        _ => [0.75, 2.05, 2.0, 50.0],
    }
}

/// Outdoor unit: width, height, depth, fans, fan diameter.
fn outdoor_row(size: u32) -> [f64; 5] {
    match size {
        4 | 6 => [0.89, 0.67, 0.34, 1.0, 0.46],
        8 | 10 => [1.01, 0.86, 0.4, 1.0, 0.53],
        _ => [1.01, 1.36, 0.4, 2.0, 0.53],
    }
}

/// A number as Slovak drawings write it: a decimal comma, no trailing
/// zeros.
fn n(v: f64, decimals: usize) -> String {
    let s = format!("{v:.decimals$}");
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    };
    s.replace('.', ",")
}

fn r1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}
fn r2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
fn mm(v: f64) -> String {
    format!("{:.0}", v * 1000.0)
}

/// The electric backup heater of an indoor unit, kW.
pub fn backup_kw(size: u32) -> f64 {
    if size >= 12 {
        9.0
    } else {
        6.0
    }
}

/// The heat pump's greatest output, kW, by outside temperature (°C) at
/// a flow temperature of 35 or 55 °C: the usual curve of an R32 inverter
/// unit rated `size` kW at A7/W35.
pub fn hp_points(size: u32, w55: bool) -> [(f64, f64); 4] {
    let s = size as f64;
    let f = if w55 {
        [0.62, 0.76, 0.85, 0.95]
    } else {
        [0.72, 0.85, 0.92, 1.0]
    };
    let q = |k: f64| if k == 1.0 { s } else { r1(s * k) };
    [
        (-15.0, q(f[0])),
        (-7.0, q(f[1])),
        (2.0, q(f[2])),
        (7.0, q(f[3])),
    ]
}

/// The curve at flow temperature `t_flow`: the nearer of W35 and W55,
/// and where it is further than 2 K off, about 0.5 % of output for each
/// kelvin of flow temperature.
pub fn hp_curve(size: u32, t_flow: f64) -> Vec<(f64, f64)> {
    let near: f64 = if (55.0 - t_flow).abs() < (35.0 - t_flow).abs() {
        55.0
    } else {
        35.0
    };
    let pts = hp_points(size, near == 55.0);
    let f = if (near - t_flow).abs() > 2.0 {
        1.0 - 0.005 * (t_flow - near)
    } else {
        1.0
    };
    pts.iter().map(|(t, q)| (*t, q * f)).collect()
}

/// The unit of `kind` in `size` (the nearest of its range).
pub fn describe(kind: HeatKind, size: u32) -> HeatUnit {
    let size = kind.size(size);
    let s = size as f64;
    let mut ports: Vec<HeatPort> = vec![];
    let mut params: Vec<(String, String)> = vec![];
    let mut port = |name: &'static str, code: &'static str, at: [f64; 3], dn: u32| {
        ports.push(HeatPort { name, code, at, dn })
    };
    let mut p = |k: &str, v: String| params.push((k.to_string(), v));
    let (name, ifc, dims);
    match kind {
        HeatKind::Cylinder => {
            let [d, h, a, q] = cylinder_row(size);
            dims = [d, d, h];
            name = format!("Zásobník TV {size} l nepriamo ohrievaný");
            ifc = "IfcTank";
            let y = d / 2.0;
            port("Studená voda – vstup", "SV", [-0.08, y, 0.12], 20);
            port("Teplá voda – výstup", "TV", [-0.08, y, h - 0.12], 20);
            port("Cirkulácia TV", "C", [-0.08, y, h * 0.62], 20);
            port(
                "Výmenník – vstup vykurovacej vody",
                "UKP",
                [0.08, y, h * 0.55],
                25,
            );
            port(
                "Výmenník – výstup vykurovacej vody",
                "UKS",
                [0.08, y, 0.22],
                25,
            );
            port(
                "Poistný ventil – odtok do kanalizácie",
                "K",
                [0.16, y, 0.12],
                20,
            );
            p("Objem", format!("{size} l"));
            p("Plocha výmenníka", format!("{} m²", n(a, 2)));
            p(
                "Výkon výmenníka (80/60 °C, 10/45 °C)",
                format!("{} kW", n(q, 1)),
            );
            p(
                "Trvalý výkon TV (10/45 °C)",
                format!("{:.0} l/h", (q * 860.0 / 35.0).round()),
            );
            p(
                "Pohotovostné tepelné straty",
                format!("{} kWh/24 h", n(r1(0.9 + s / 250.0), 1)),
            );
            p("Najvyšší prevádzkový tlak", "1 MPa".into());
            p("Prípojky vody / cirkulácie", "G 3/4\"".into());
            p("Prípojky výmenníka", "G 1\"".into());
            p("Rozmer Ø × výška", format!("{} × {} mm", mm(d), mm(h)));
        }
        HeatKind::Boiler | HeatKind::BoilerCombi => {
            let big = size > 30;
            let combi = kind == HeatKind::BoilerCombi;
            let (w, d, h) = (0.44, if big { 0.372 } else { 0.338 }, 0.72);
            dims = [w, d, h];
            name = format!(
                "Kondenzačný kotol {}{size} kW",
                if combi { "kombinovaný " } else { "" }
            );
            ifc = "IfcBoiler";
            let y = 0.02;
            if combi {
                port("Vykurovanie – prívod", "UKP", [-0.16, y, 0.0], 20);
                port("Teplá voda – výstup", "TV", [-0.08, y, 0.0], 15);
                port("Plyn", "P", [0.0, y, 0.0], 15);
                port("Studená voda – vstup", "SV", [0.08, y, 0.0], 15);
                port("Vykurovanie – spiatočka", "UKS", [0.16, y, 0.0], 20);
            } else {
                port("Vykurovanie – prívod", "UKP", [-0.15, y, 0.0], 20);
                port("Plyn", "P", [-0.05, y, 0.0], 15);
                port("Vykurovanie – spiatočka", "UKS", [0.05, y, 0.0], 20);
                port("Ohrev zásobníka – prívod", "UKP", [0.15, y, 0.0], 20);
                port("Ohrev zásobníka – spiatočka", "UKS", [0.21, y, 0.0], 20);
            }
            port("Odvod kondenzátu", "K", [0.19, 0.05, 0.0], 20);
            port(
                "Odvod spalín / prívod vzduchu 60/100",
                "INE",
                [0.0, 0.02, h + 0.12],
                100,
            );
            let dhw = s + 4.0;
            p("Menovitý tepelný výkon (80/60 °C)", format!("{size} kW"));
            p(
                "Tepelný výkon (50/30 °C)",
                format!("{} kW", n(r1(s * 1.08), 1)),
            );
            p(
                "Najmenší výkon",
                format!("{} kW", n(r1(s * 0.15).max(2.5), 1)),
            );
            if combi {
                p("Výkon ohrevu TV", format!("{} kW", n(dhw, 1)));
                p(
                    "Prietok TV (ΔT 30 K)",
                    format!("{} l/min", n(r1(dhw * 60.0 / (4.186 * 30.0)), 1)),
                );
            }
            p(
                "Spotreba zemného plynu G20",
                format!(
                    "{} m³/h",
                    n(r2(if combi { dhw } else { s } / (9.45 * 0.98)), 2)
                ),
            );
            p("Sezónna účinnosť vykurovania ηs", "93 %".into());
            p("Trieda NOx", "6".into());
            p(
                "Elektrický príkon",
                format!("{:.0} W", (60.0 + s * 2.0).round()),
            );
            p(
                "Odvod spalín",
                "koaxiálny 60/100 (80/125), typ C13/C33/C43".into(),
            );
            p(
                "Prípojky",
                format!(
                    "ÚK G 3/4\", plyn G 1/2\"{}, kondenzát hadica Ø 21",
                    if combi { ", SV/TV G 1/2\"" } else { "" }
                ),
            );
            p(
                "Rozmer š × v × h",
                format!("{} × {} × {} mm", mm(w), mm(h), mm(d)),
            );
            p("Expanzná nádoba", format!("{} l", if big { 10 } else { 8 }));
        }
        HeatKind::HpOutdoor => {
            let [w, h, d, _, _] = outdoor_row(size);
            dims = [w, d, h];
            name = format!("Tepelné čerpadlo vzduch–voda {size} kW – vonkajšia jednotka");
            ifc = "IfcUnitaryEquipment";
            let x = w / 2.0 - 0.04;
            port("Chladivo – kvapalina", "CH", [x, 0.05, 0.25], 10);
            port("Chladivo – plyn", "CH", [x, 0.12, 0.25], 16);
            port("Napájanie", "E", [x, -0.05, 0.4], 0);
            port("Odvod kondenzátu", "K", [0.0, 0.0, 0.06], 32);
            for (w55, label) in [(false, "W35"), (true, "W55")] {
                let pts = hp_points(size, w55);
                for (t, q) in pts.iter().rev() {
                    p(
                        &format!("Tepelný výkon A{t:.0}/{label}"),
                        format!("{} kW", n(*q, 1)),
                    );
                }
            }
            let cop = 5.1 - s * 0.02;
            p("COP A7/W35", n(r2(cop), 2));
            p("COP A2/W35", n(r2(4.0 - s * 0.015), 2));
            p("COP A-7/W35", n(r2(2.9 - s * 0.01), 2));
            p("COP A-7/W55", n(r2(2.1 - s * 0.008), 2));
            p("Prevádzková teplota (vykurovanie)", "−25 až +35 °C".into());
            p("SCOP (35 °C, priemerná klíma)", "4,6".into());
            p(
                "Chladiaci výkon A35/W18",
                format!("{} kW", n(r1(s * 0.95), 1)),
            );
            p(
                "Elektrický príkon (A7/W35)",
                format!("{} kW", n(r2(s / cop), 2)),
            );
            p(
                "Hladina akustického výkonu",
                format!(
                    "{} dB(A)",
                    if size <= 6 {
                        56
                    } else if size <= 10 {
                        59
                    } else {
                        63
                    }
                ),
            );
            p("Chladivo", "R32".into());
            p(
                "Rúrky chladiva",
                format!(
                    "kvapalina 1/4\" (6,35 mm), plyn {}",
                    if size >= 12 {
                        "5/8\" (15,88 mm)"
                    } else {
                        "1/2\" (12,7 mm)"
                    }
                ),
            );
            p(
                "Napájanie",
                if size >= 12 {
                    "3 × 400 V / 50 Hz"
                } else {
                    "1 × 230 V / 50 Hz"
                }
                .into(),
            );
            p("Max. dĺžka potrubia chladiva", "30 m".into());
            p("Hmotnosť", format!("{:.0} kg", (40.0 + s * 4.5).round()));
            p(
                "Rozmer š × v × h",
                format!("{} × {} × {} mm", mm(w), mm(h), mm(d)),
            );
            p(
                "Osadenie",
                "na konzole / sokli min. 300 mm nad terénom, voľný priestor 300 mm vzadu, 1 m vpredu"
                    .into(),
            );
        }
        HeatKind::HpIndoor | HeatKind::HpIndoorCylinder => {
            let tank = kind == HeatKind::HpIndoorCylinder;
            let (w, d, h) = if tank {
                (0.6, 0.65, 1.75)
            } else {
                (0.45, 0.33, 0.8)
            };
            dims = [w, d, h];
            name = format!(
                "Tepelné čerpadlo {size} kW – vnútorná jednotka{}",
                if tank {
                    " so zásobníkom TV 190 l"
                } else {
                    " (hydromodul)"
                }
            );
            ifc = "IfcUnitaryEquipment";
            let (z, y) = (if tank { h } else { 0.0 }, 0.05);
            port("Vykurovanie – prívod", "UKP", [-0.15, y, z], 25);
            port("Vykurovanie – spiatočka", "UKS", [-0.07, y, z], 25);
            port("Chladivo – kvapalina", "CH", [0.05, y, z], 10);
            port("Chladivo – plyn", "CH", [0.11, y, z], 16);
            port(
                "Poistný ventil – odtok",
                "K",
                [0.17, y, if tank { 0.1 } else { 0.0 }],
                20,
            );
            port("Napájanie a ovládanie", "E", [0.17, y + 0.08, z], 0);
            if tank {
                port("Studená voda – vstup", "SV", [-0.15, y + 0.12, z], 20);
                port("Teplá voda – výstup", "TV", [-0.07, y + 0.12, z], 20);
                port("Cirkulácia TV", "C", [0.02, y + 0.12, z], 20);
            } else {
                port(
                    "Ohrev zásobníka TV – prívod (3-cestný ventil)",
                    "UKP",
                    [-0.15, y + 0.08, z],
                    25,
                );
                port(
                    "Ohrev zásobníka TV – spiatočka",
                    "UKS",
                    [-0.07, y + 0.08, z],
                    25,
                );
            }
            p(
                "Vonkajšia jednotka (zostava)",
                format!("{size} kW (A7/W35)"),
            );
            p(
                "Elektrický dohrev (vykurovacia tyč)",
                format!("{:.0} kW", backup_kw(size)),
            );
            p("Expanzná nádoba", "8 l".into());
            p(
                "Obehové čerpadlo",
                "elektronické, dopravná výška 7,5 m".into(),
            );
            p("Najvyššia teplota výstupnej vody", "60 °C".into());
            if tank {
                p("Objem zásobníka TV", "190 l".into());
                p("Prípojky SV / TV / C", "G 3/4\"".into());
            } else {
                p(
                    "Prípojka zásobníka TV",
                    "externý zásobník cez 3-cestný ventil".into(),
                );
            }
            p("Prípojky ÚK", "G 1\"".into());
            p(
                "Rozmer š × v × h",
                format!("{} × {} × {} mm", mm(w), mm(h), mm(d)),
            );
        }
    }
    HeatUnit {
        kind,
        size,
        name,
        ifc,
        dims,
        ports,
        params,
    }
}

fn dark() -> Look {
    look(
        "Plastic, charcoal",
        "plastic",
        Hatch::Empty,
        [55, 65, 81],
        0.5,
    )
}
fn grey() -> Look {
    metal("Steel, painted grey", [107, 114, 128], 0.5)
}
fn casing() -> Look {
    metal("Steel, painted light grey", [229, 231, 235], 0.4)
}

/// The height a wall-hung unit's underside is above the floor.
const WALL_HUNG: f64 = 1.1;

/// The unit as a catalogue object, its back on y = 0 and its front
/// towards −y, with a `Connection` part at every pipe it takes.
pub fn unit(kind: HeatKind, size: u32) -> CatalogueObject {
    let u = describe(kind, size);
    let [w, d, h] = u.dims;
    // PilonDesigner's y runs to the back from the middle of the depth
    let back = |y: f64| y - d / 2.0;
    let mut b = Build::new();
    let mount = match kind {
        HeatKind::Boiler | HeatKind::BoilerCombi | HeatKind::HpIndoor => {
            b.at(WALL_HUNG);
            Mount::Wall
        }
        _ => Mount::Floor,
    };
    match kind {
        HeatKind::Cylinder => {
            let c = v2(0.0, back(0.0));
            b.add(
                "Cylinder",
                appliance_white(),
                cylinder(c, w / 2.0 - 0.003, 0.05, h - 0.02),
                true,
            );
            b.add("Lid", grey(), cylinder(c, w / 2.0, h - 0.02, h), false);
            for a in [0.0_f64, 2.1, 4.2] {
                let at = c + v2(a.cos(), a.sin()) * w * 0.35;
                b.add("Foot", dark(), cylinder(at, 0.02, 0.0, 0.05), false);
            }
            // the inspection flange on the front
            b.add(
                "Flange",
                dark(),
                cuboid(v3(-0.06, -d, h * 0.3), v3(0.06, -d + 0.02, h * 0.38)),
                false,
            );
        }
        HeatKind::Boiler | HeatKind::BoilerCombi => {
            b.add(
                "Casing",
                appliance_white(),
                cuboid(v3(-w / 2.0, -d, 0.0), v3(w / 2.0, 0.0, h)),
                true,
            );
            b.add(
                "Display",
                dark(),
                cuboid(v3(-0.09, -d - 0.004, 0.09), v3(0.09, -d, 0.15)),
                false,
            );
            b.add(
                "Flue",
                appliance_white(),
                cylinder(v2(0.0, back(0.02)), 0.05, h, h + 0.12),
                false,
            );
            b.outline(super::squircle(v2(0.0, back(0.02)), 0.05, 0.05, 2.0, 24));
            b.line(v2(-w / 2.0, -d), v2(w / 2.0, 0.0));
        }
        HeatKind::HpOutdoor => {
            let [_, _, _, fans, fan_d] = outdoor_row(u.size);
            b.add(
                "Casing",
                casing(),
                cuboid(v3(-w / 2.0, -d + 0.03, 0.06), v3(w / 2.0, 0.0, h)),
                true,
            );
            for x in [-w / 2.0 + 0.06, w / 2.0 - 0.06] {
                b.add(
                    "Foot",
                    dark(),
                    cuboid(v3(x - 0.03, -d, 0.0), v3(x + 0.03, 0.0, 0.06)),
                    false,
                );
            }
            let fans = fans as usize;
            for i in 0..fans {
                let zc = if fans == 1 {
                    h / 2.0 + 0.03
                } else {
                    0.06 + (h - 0.06) / 4.0 * (1.0 + 2.0 * i as f64)
                };
                let r = fan_d / 2.0;
                let x = -w * 0.12;
                b.add(
                    "Fan grille",
                    grey(),
                    cuboid(v3(x - r, -d + 0.01, zc - r), v3(x + r, -d + 0.03, zc + r)),
                    false,
                );
            }
            // the fan's side in plan: the air blows out of the front
            b.line(v2(-w / 2.0, -d + 0.03), v2(0.0, -d - 0.12));
            b.line(v2(0.0, -d - 0.12), v2(w / 2.0, -d + 0.03));
        }
        HeatKind::HpIndoor | HeatKind::HpIndoorCylinder => {
            b.add(
                "Casing",
                appliance_white(),
                cuboid(v3(-w / 2.0, -d, 0.0), v3(w / 2.0, 0.0, h)),
                true,
            );
            b.add(
                "Display",
                dark(),
                cuboid(v3(-0.08, -d - 0.004, h - 0.3), v3(0.08, -d, h - 0.2)),
                false,
            );
            b.line(v2(-w / 2.0, -d), v2(w / 2.0, 0.0));
            b.line(v2(-w / 2.0, 0.0), v2(w / 2.0, -d));
        }
    }
    // a stub where each pipe joins: inside the back face and the floor,
    // so the unit still stands against its wall
    for p in &u.ports {
        if !matches!(p.code, "SV" | "TV" | "C" | "K" | "UKP" | "UKS" | "P") {
            continue;
        }
        let r = 0.012;
        let y = back(p.at[1]).min(-r);
        let z = p.at[2];
        // connections under a unit hang below it, others end at their axis
        let (z0, z1) = if z < 1e-9 { (0.0, 0.02) } else { (z - 0.02, z) };
        b.add(
            &format!("Connection {} DN{}", p.code, p.dn),
            metal("Brass", [196, 160, 92], 0.3),
            cuboid(v3(p.at[0] - r, y - r, z0), v3(p.at[0] + r, y + r, z1)),
            false,
        );
    }
    let description = format!(
        "{}: {}",
        kind.name(),
        u.params
            .iter()
            .take(4)
            .map(|(k, v)| format!("{k} {v}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    b.finish(
        &format!("heat-{}-{}", kind.id().to_lowercase(), u.size),
        &u.name,
        &description,
        ObjectCategory::Hvac,
        (
            u.ifc,
            match kind {
                HeatKind::Cylinder => "STORAGE",
                HeatKind::Boiler | HeatKind::BoilerCombi => "WATER",
                HeatKind::HpOutdoor => "SPLITSYSTEM",
                _ => "SPLITSYSTEM",
            },
        ),
        mount,
    )
}

/// One of each kind in its usual size for a house.
pub fn starters() -> Vec<CatalogueObject> {
    vec![
        unit(HeatKind::Cylinder, 200),
        unit(HeatKind::Boiler, 20),
        unit(HeatKind::BoilerCombi, 25),
        unit(HeatKind::HpOutdoor, 8),
        unit(HeatKind::HpIndoor, 8),
        unit(HeatKind::HpIndoorCylinder, 8),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{part_connectors, ConnKind};

    #[test]
    fn the_ranges_are_pilon_designers() {
        assert_eq!(HeatKind::Cylinder.pick(180.0), 200);
        assert_eq!(HeatKind::Boiler.pick(17.2), 20);
        assert_eq!(HeatKind::Boiler.pick(80.0), 35);
        assert_eq!(HeatKind::HpOutdoor.size(9), 10);
        let c = describe(HeatKind::Cylinder, 300);
        assert_eq!(c.dims, [0.66, 0.66, 1.7]);
        assert!(c
            .params
            .contains(&("Trvalý výkon TV (10/45 °C)".into(), "909 l/h".into())));
        let k = describe(HeatKind::BoilerCombi, 25);
        assert!(k
            .params
            .contains(&("Prietok TV (ΔT 30 K)".into(), "13,9 l/min".into())));
        assert!(k
            .params
            .contains(&("Spotreba zemného plynu G20".into(), "3,13 m³/h".into())));
        let hp = describe(HeatKind::HpOutdoor, 8);
        assert!(hp
            .params
            .contains(&("Tepelný výkon A-7/W35".into(), "6,8 kW".into())));
        assert!(hp.params.contains(&("COP A7/W35".into(), "4,94".into())));
    }

    #[test]
    fn the_curve_follows_the_flow_temperature() {
        assert_eq!(
            hp_curve(10, 35.0),
            vec![(-15.0, 7.2), (-7.0, 8.5), (2.0, 9.2), (7.0, 10.0)]
        );
        // 45 °C is 10 K over W35: 5 % less
        let c = hp_curve(10, 45.0);
        assert!((c[1].1 - 8.5 * 0.95).abs() < 1e-9);
        assert_eq!(hp_curve(10, 54.0)[1], (-7.0, 7.6));
    }

    #[test]
    fn pipes_join_where_the_connections_are() {
        let boiler = unit(HeatKind::Boiler, 20);
        let c = part_connectors(&boiler.object);
        let kinds: Vec<_> = c.iter().map(|c| c.kind).collect();
        assert_eq!(
            kinds,
            vec![
                ConnKind::UKP,
                ConnKind::P,
                ConnKind::UKS,
                ConnKind::UKP,
                ConnKind::UKS,
                ConnKind::K
            ]
        );
        // heating supply: 150 mm left of the middle, DN 20, under the boiler
        assert!((c[0].at.x + 0.15).abs() < 1e-9 && c[0].dn == 20.0);
        assert!((c[1].dn - 15.0).abs() < 1e-9);
        let tank = unit(HeatKind::Cylinder, 200);
        let c = part_connectors(&tank.object);
        assert_eq!(c.len(), 6);
        // hot water leaves 120 mm under the top, at the back
        let tv = c.iter().find(|c| c.kind == ConnKind::TV).unwrap();
        assert!((tv.at.z - 1.18).abs() < 1e-9 && tv.at.y > -0.03);
        assert!(c.iter().any(|c| c.kind == ConnKind::C));
    }
}
