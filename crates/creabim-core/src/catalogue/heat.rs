//! Heat sources and hot water heating, as PilonDesigner's `equiplib.ts`:
//! an indirectly heated hot water cylinder, a wall-hung condensing gas
//! boiler (system or combination) and a split air-to-water heat pump:
//! its outdoor unit and its indoor one, a wall hydromodule or one with a
//! hot water cylinder built in. The heat pump is Panasonic's Aquarea
//! (bi-bloc J and H, all-in-one J) with its catalogue's sizes and
//! outputs; the cylinder and the boilers keep the ranges usual on the
//! Slovak and Czech market, to be replaced by a maker's own.
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
            // Panasonic Aquarea all-in-one J: the 185 l unit with the J
            // outdoor units only
            HeatKind::HpIndoorCylinder => &[3, 5, 7, 9],
            // Panasonic Aquarea bi-bloc J (R32) and H (R410A) kits
            _ => &[3, 5, 7, 9, 12, 16],
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

/// One Panasonic Aquarea bi-bloc kit, as its catalogue prints it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aquarea {
    /// Rated kW (A7/W35).
    pub size: u32,
    pub kit: &'static str,
    pub outdoor: &'static str,
    pub indoor: &'static str,
    /// Outdoor unit width, height, depth, m.
    pub out_whd: [f64; 3],
    pub out_kg: f64,
    pub in_kg: f64,
    /// Heating kW and COP at A7, A2, A-7 (W35, then W55); none where
    /// the catalogue read prints only A7 (the H kits).
    pub w35: [(f64, f64); 3],
    pub w55: [(f64, f64); 3],
    pub full_curve: bool,
    /// SCOP, average climate, W35 / W55; 0 where not read.
    pub scop: (f64, f64),
    /// Outdoor sound power at full load, heating, dB; 0 where not read.
    pub lw: u32,
    /// Integrated electric backup heater, kW.
    pub backup: f64,
    /// Refrigerant pipes, liquid and gas, inch (mm).
    pub pipes: (&'static str, &'static str),
    /// Refrigerant pipe length range, m, and height difference, m.
    pub length: (&'static str, u32),
    /// R32 charge, kg; 0 for the R410A kits (not read).
    pub charge: f64,
}

/// The Panasonic Aquarea bi-bloc kits (catalogue "Aquarea Bi-bloc J
/// R32", panasonicproclub.com, and Panasonic Aquarea 2023–2024, p. 17
/// and 18; read 2026-11-22). The H kits' A7 values and sizes are from
/// the 2023–2024 catalogue's p. 18 as read; their A2/A-7 points are not
/// printed in what could be read, so their curve holds A7 only and the
/// sizing does not take them.
pub const AQUAREA: [Aquarea; 6] = [
    Aquarea {
        size: 3,
        kit: "KIT-WC03J3E5",
        outdoor: "WH-UD03JE5",
        indoor: "WH-SDC0305J3E5",
        out_whd: [0.824, 0.622, 0.298],
        out_kg: 37.0,
        in_kg: 42.0,
        w35: [(3.20, 5.33), (3.20, 3.64), (3.30, 2.80)],
        w55: [(3.20, 2.81), (3.20, 2.19), (3.20, 1.79)],
        full_curve: true,
        scop: (5.07, 3.47),
        lw: 60,
        backup: 3.0,
        pipes: ("1/4\" (6,35)", "1/2\" (12,70)"),
        length: ("3–25", 20),
        charge: 0.9,
    },
    Aquarea {
        size: 5,
        kit: "KIT-WC05J3E5",
        outdoor: "WH-UD05JE5",
        indoor: "WH-SDC0305J3E5",
        out_whd: [0.824, 0.622, 0.298],
        out_kg: 37.0,
        in_kg: 42.0,
        w35: [(5.00, 5.00), (4.20, 3.18), (4.20, 2.59)],
        w55: [(5.00, 2.72), (4.10, 1.99), (3.55, 1.71)],
        full_curve: true,
        scop: (5.07, 3.47),
        lw: 64,
        backup: 3.0,
        pipes: ("1/4\" (6,35)", "1/2\" (12,70)"),
        length: ("3–25", 20),
        charge: 0.9,
    },
    Aquarea {
        size: 7,
        kit: "KIT-WC07J3E5",
        outdoor: "WH-UD07JE5",
        indoor: "WH-SDC0709J3E5",
        out_whd: [0.875, 0.795, 0.320],
        out_kg: 61.0,
        in_kg: 42.0,
        w35: [(7.00, 4.76), (6.85, 3.41), (5.60, 2.87)],
        w55: [(7.00, 2.82), (6.20, 2.21), (5.25, 1.94)],
        full_curve: true,
        scop: (4.90, 3.32),
        lw: 68,
        backup: 3.0,
        pipes: ("1/4\" (6,35)", "5/8\" (15,88)"),
        length: ("3–50", 30),
        charge: 1.27,
    },
    Aquarea {
        size: 9,
        kit: "KIT-WC09J3E5",
        outdoor: "WH-UD09JE5-1",
        indoor: "WH-SDC0709J3E5",
        out_whd: [0.875, 0.795, 0.320],
        out_kg: 61.0,
        in_kg: 42.0,
        w35: [(9.00, 4.48), (7.00, 3.40), (6.12, 2.78)],
        w55: [(8.95, 2.78), (6.30, 2.16), (5.90, 1.93)],
        full_curve: true,
        scop: (4.90, 3.32),
        lw: 69,
        backup: 3.0,
        pipes: ("1/4\" (6,35)", "5/8\" (15,88)"),
        length: ("3–50", 30),
        charge: 1.27,
    },
    Aquarea {
        size: 12,
        kit: "KIT-WC12H6E5",
        outdoor: "WH-UD12HE5",
        indoor: "WH-SDC12H6E5",
        out_whd: [0.900, 1.340, 0.320],
        out_kg: 101.0,
        in_kg: 43.0,
        w35: [(12.00, 4.74), (0.0, 0.0), (0.0, 0.0)],
        w55: [(12.00, 0.0), (0.0, 0.0), (0.0, 0.0)],
        full_curve: false,
        scop: (0.0, 0.0),
        lw: 0,
        backup: 6.0,
        pipes: ("", ""),
        length: ("", 0),
        charge: 0.0,
    },
    Aquarea {
        size: 16,
        kit: "KIT-WC16H6E5",
        outdoor: "WH-UD16HE5",
        indoor: "WH-SDC16H6E5",
        out_whd: [0.900, 1.340, 0.320],
        out_kg: 101.0,
        in_kg: 44.0,
        w35: [(16.00, 4.28), (0.0, 0.0), (0.0, 0.0)],
        w55: [(14.50, 0.0), (0.0, 0.0), (0.0, 0.0)],
        full_curve: false,
        scop: (0.0, 0.0),
        lw: 0,
        backup: 6.0,
        pipes: ("", ""),
        length: ("", 0),
        charge: 0.0,
    },
];

/// The kit of `size` kW (the nearest of the range).
pub fn aquarea(size: u32) -> &'static Aquarea {
    let size = HeatKind::HpOutdoor.size(size);
    AQUAREA
        .iter()
        .find(|a| a.size == size)
        .unwrap_or(&AQUAREA[3])
}

/// The Panasonic Aquarea all-in-one J indoor unit with its 185 l
/// stainless tank: width, height, depth, m (catalogue 2023–2024, p. 16).
pub const ALL_IN_ONE_WHD: [f64; 3] = [0.599, 1.642, 0.602];

/// Outdoor unit: width, height, depth and its fans. The catalogue does
/// not say how many fans a unit has: the 1340 mm tall H units are drawn
/// with two, the lower ones with one.
fn outdoor_row(size: u32) -> [f64; 4] {
    let a = aquarea(size);
    let [w, h, d] = a.out_whd;
    [w, h, d, if h > 1.0 { 2.0 } else { 1.0 }]
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

/// The electric backup heater of the indoor unit, kW (Panasonic: 3 kW
/// in the J kits, 6 kW in the H kits).
pub fn backup_kw(size: u32) -> f64 {
    aquarea(size).backup
}

/// The heat pump's output, kW, by outside temperature (°C) at a flow
/// temperature of 35 or 55 °C, as the catalogue prints it: A-7, A2 and
/// A7. The H kits have their A7 point only.
pub fn hp_points(size: u32, w55: bool) -> Vec<(f64, f64)> {
    let a = aquarea(size);
    let row = if w55 { a.w55 } else { a.w35 };
    if !a.full_curve {
        return vec![(7.0, row[0].0)];
    }
    vec![(-7.0, row[2].0), (2.0, row[1].0), (7.0, row[0].0)]
}

/// Whether the kit has the catalogue's whole curve (A-7 to A7), so it
/// can be sized by its bivalence point.
pub fn has_curve(size: u32) -> bool {
    aquarea(size).full_curve
}

/// The curve at flow temperature `t_flow`: between the catalogue's W35
/// and W55 points in a line (held at W35 under 35 °C, at W55 over 55 °C).
/// Under A-7 the catalogue prints nothing: the output is held at its
/// A-7 value there, down to the unit's −20 °C limit.
pub fn hp_curve(size: u32, t_flow: f64) -> Vec<(f64, f64)> {
    let (a, b) = (hp_points(size, false), hp_points(size, true));
    let k = ((t_flow - 35.0) / 20.0).clamp(0.0, 1.0);
    let mut pts: Vec<(f64, f64)> = a
        .iter()
        .zip(&b)
        .map(|((t, qa), (_, qb))| (*t, r2(qa + (qb - qa) * k)))
        .collect();
    if pts.len() > 1 {
        pts.insert(0, (-20.0, pts[0].1));
    }
    pts
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
            let a = aquarea(size);
            let [w, h, d, _] = outdoor_row(size);
            dims = [w, d, h];
            name = format!(
                "Panasonic Aquarea {} – vonkajšia jednotka {}",
                a.outdoor, a.kit
            );
            ifc = "IfcUnitaryEquipment";
            let x = w / 2.0 - 0.04;
            port("Chladivo – kvapalina", "CH", [x, 0.05, 0.25], 10);
            port("Chladivo – plyn", "CH", [x, 0.12, 0.25], 16);
            port("Napájanie", "E", [x, -0.05, 0.4], 0);
            port("Odvod kondenzátu", "K", [0.0, 0.0, 0.06], 32);
            p(
                "Zostava",
                format!("{} ({} + {})", a.kit, a.indoor, a.outdoor),
            );
            for (label, row) in [("W35", a.w35), ("W55", a.w55)] {
                for (t, (q, cop)) in [7, 2, -7].iter().zip(row) {
                    if q <= 0.0 {
                        continue;
                    }
                    let cop = if cop > 0.0 {
                        format!(", COP {}", n(cop, 2))
                    } else {
                        String::new()
                    };
                    p(
                        &format!("Tepelný výkon A{t}/{label}"),
                        format!("{} kW{cop}", n(q, 2)),
                    );
                }
            }
            if a.scop.0 > 0.0 {
                p(
                    "SCOP priemerná klíma W35 / W55",
                    format!("{} / {}", n(a.scop.0, 2), n(a.scop.1, 2)),
                );
            }
            if a.lw > 0 {
                p(
                    "Akustický výkon (plný výkon, vykurovanie)",
                    format!("{} dB", a.lw),
                );
            }
            p("Prevádzková teplota (vykurovanie)", "−20 až +35 °C".into());
            p(
                "Chladivo",
                if a.charge > 0.0 {
                    format!("R32, {} kg", n(a.charge, 2))
                } else {
                    "R410A".into()
                },
            );
            if !a.pipes.0.is_empty() {
                p(
                    "Rúrky chladiva kvapalina / plyn",
                    format!("{} / {} mm", a.pipes.0, a.pipes.1),
                );
                p(
                    "Dĺžka potrubia chladiva / prevýšenie",
                    format!("{} m / {} m", a.length.0, a.length.1),
                );
            }
            p(
                "Napájanie",
                "1 × 230 V / 50 Hz (cez vnútornú jednotku)".into(),
            );
            p("Hmotnosť", format!("{:.0} kg", a.out_kg));
            p(
                "Rozmer v × š × h",
                format!("{} × {} × {} mm", mm(h), mm(w), mm(d)),
            );
            if !a.full_curve {
                p(
                    "Pozn.",
                    "výkony pri A2 a A-7 neboli z katalógu načítané".into(),
                );
            }
        }
        HeatKind::HpIndoor | HeatKind::HpIndoorCylinder => {
            let tank = kind == HeatKind::HpIndoorCylinder;
            let a = aquarea(size);
            // Panasonic: bi-bloc hydromodule 892 × 500 × 340 (v × š × h);
            // all-in-one 1642 × 599 × 602 with its 185 l tank
            let (w, d, h) = if tank {
                (ALL_IN_ONE_WHD[0], ALL_IN_ONE_WHD[2], ALL_IN_ONE_WHD[1])
            } else {
                (0.5, 0.34, 0.892)
            };
            dims = [w, d, h];
            name = if tank {
                format!(
                    "Panasonic Aquarea All-in-One WH-ADC0309J3E5 so zásobníkom TV 185 l – vnútorná jednotka k {}",
                    a.outdoor
                )
            } else {
                format!(
                    "Panasonic Aquarea {} – vnútorná jednotka (hydromodul) {}",
                    a.indoor, a.kit
                )
            };
            ifc = "IfcUnitaryEquipment";
            let (z, y) = (if tank { h } else { 0.0 }, 0.05);
            port("Vykurovanie – prívod", "UKP", [-0.15, y, z], 32);
            port("Vykurovanie – spiatočka", "UKS", [-0.07, y, z], 32);
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
                    32,
                );
                port(
                    "Ohrev zásobníka TV – spiatočka",
                    "UKS",
                    [-0.07, y + 0.08, z],
                    32,
                );
            }
            p(
                "Vonkajšia jednotka",
                format!("{} ({size} kW pri A7/W35)", a.outdoor),
            );
            p(
                "Elektrický dohrev (vykurovacia tyč)",
                format!("{} kW", n(a.backup, 0)),
            );
            p("Výstupná voda (vykurovanie)", "20 až 60 °C".into());
            if tank {
                p("Objem zásobníka TV", "185 l (nerez)".into());
            } else {
                p("Hmotnosť", format!("{:.0} kg", a.in_kg));
                p(
                    "Prípojka zásobníka TV",
                    "externý zásobník cez 3-cestný ventil".into(),
                );
            }
            p("Prípojky vody", "R 1¼\"".into());
            p(
                "Rozmer v × š × h",
                format!("{} × {} × {} mm", mm(h), mm(w), mm(d)),
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
            let [_, _, _, fans] = outdoor_row(u.size);
            // the grille drawn over most of the unit's height or half of
            // it (its size is not in the catalogue)
            let fan_d = ((h - 0.06) / fans - 0.1).min(w * 0.6);
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
            // in front of the unit: it is too shallow to hold its name
            b.text(v2(0.0, -d - 0.3), "T.Č. vonkajšia");
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
            // a wall hydromodule is too shallow to hold its name
            let at = if kind == HeatKind::HpIndoor {
                -d - 0.2
            } else {
                -d / 2.0
            };
            b.text(v2(0.0, at), "T.Č. vnútorná");
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
        unit(HeatKind::HpOutdoor, 9),
        unit(HeatKind::HpIndoor, 9),
        unit(HeatKind::HpIndoorCylinder, 9),
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
        assert_eq!(HeatKind::HpOutdoor.size(8), 9);
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
        let hp = describe(HeatKind::HpOutdoor, 9);
        assert!(hp
            .params
            .contains(&("Tepelný výkon A-7/W35".into(), "6,12 kW, COP 2,78".into())));
        assert_eq!(hp.dims, [0.875, 0.32, 0.795]);
        assert!(hp.name.contains("WH-UD09JE5-1"));
    }

    #[test]
    fn the_curve_is_panasonics() {
        // KIT-WC07J3E5 at W35: A-7 5,60, A2 6,85, A7 7,00 kW
        assert_eq!(
            hp_curve(7, 35.0),
            vec![(-20.0, 5.6), (-7.0, 5.6), (2.0, 6.85), (7.0, 7.0)]
        );
        // 45 °C: halfway to W55 (A-7 5,25)
        assert!((hp_curve(7, 45.0)[1].1 - 5.43).abs() < 1e-9);
        // the H kits: A7 only, not sized by their curve
        assert_eq!(hp_curve(12, 35.0), vec![(7.0, 12.0)]);
        assert!(!has_curve(16) && has_curve(3));
        assert_eq!(backup_kw(9), 3.0);
        assert_eq!(backup_kw(16), 6.0);
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
