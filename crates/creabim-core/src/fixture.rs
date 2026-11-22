//! Sanitary fixtures, heaters, manifolds and fan-coils as PilonDesigner
//! models them: a family ("Umývadlo", "Vaňa", "Doskové vykurovacie
//! teleso"...), its plan symbol (STN 01 3450), its size, and its
//! connectors: where its cold and hot water, waste, heating or cooling
//! pipes join it, at the usual mounting heights (STN 73 6660, STN EN 806,
//! STN EN 12056).
//!
//! A placed object (a basin from the catalogue, or one imported) is a
//! fixture of the family its type says (its catalogue entry, else its IFC
//! class), so it keeps its own 3D model and gains the connectors.
//!
//! PilonDesigner's local coordinates: x across (0 in the middle), y from
//! the wall (0) forwards; heights above the floor. An object's own axes
//! have its back on the wall too but its front towards −y, so a point
//! turns half round between them.

use crate::mep::SystemKind;
use crate::objects::{Mount, ObjectType};
use crate::{Element, ElementKind, Model};
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};

/// The families PilonDesigner offers, in its order (their names are what
/// the calculations recognise them by).
pub const FAMILIES: &[&str] = &[
    "Umývadlo",
    "WC s nádržkou",
    "Závesné WC s podomietkovou nádržkou",
    "Sprcha",
    "Vaňa",
    "Kuchynský drez",
    "Práčka",
    "Umývačka riadu",
    "Bidet",
    "Pisoár",
    "Výtokový ventil",
    "Doskové vykurovacie teleso",
    "Kúpeľňový rebrík",
    "Rozdeľovač podlahového vykurovania",
    "Fan-coil nástenný",
    "Fan-coil parapetný",
    "Fan-coil kazetový (strop)",
    "Podlahová vpusť",
];

/// A fixture's size and plan symbol.
#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub width: f64,
    pub depth: f64,
    /// Height of its top above the floor.
    pub height: f64,
    /// Height of its underside (a wall-hung basin).
    pub bottom: f64,
    /// Its plan symbol, as polylines in local coordinates.
    pub curves: Vec<Vec<DVec2>>,
    /// A label written in the symbol ("PM", "R").
    pub label: Option<&'static str>,
}

/// What a connector takes: the pipe of one system joins it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConnKind {
    /// Cold water.
    SV,
    /// Hot water.
    TV,
    /// Waste.
    K,
    /// Heating supply.
    UKP,
    /// Heating return.
    UKS,
    /// Chilled water supply.
    CHP,
    /// Chilled water return.
    CHS,
    /// Oxygen.
    O2,
    /// Medical air.
    AIR,
    /// Vacuum.
    VAC,
    /// Nitrous oxide.
    N2O,
    /// Hot water circulation.
    C,
    /// Fuel gas.
    P,
}

impl ConnKind {
    /// The system a pipe joining it is in.
    pub fn system(self) -> SystemKind {
        match self {
            ConnKind::SV => SystemKind::ColdWater,
            ConnKind::TV => SystemKind::HotWater,
            ConnKind::K => SystemKind::Sanitary,
            ConnKind::UKP => SystemKind::HeatingSupply,
            ConnKind::UKS => SystemKind::HeatingReturn,
            ConnKind::CHP => SystemKind::CoolingSupply,
            ConnKind::CHS => SystemKind::CoolingReturn,
            ConnKind::O2 | ConnKind::AIR | ConnKind::VAC | ConnKind::N2O => SystemKind::MedGas,
            ConnKind::C => SystemKind::Circulation,
            ConnKind::P => SystemKind::Gas,
        }
    }

    /// Its system's code, as drawings label it.
    pub fn code(self) -> &'static str {
        match self {
            ConnKind::SV => "SV",
            ConnKind::TV => "TV",
            ConnKind::K => "K",
            ConnKind::UKP => "UKP",
            ConnKind::UKS => "UKS",
            ConnKind::CHP => "CHP",
            ConnKind::CHS => "CHS",
            ConnKind::O2 => "O2",
            ConnKind::AIR => "AIR",
            ConnKind::VAC => "VAC",
            ConnKind::N2O => "N2O",
            ConnKind::C => "C",
            ConnKind::P => "P",
        }
    }

    /// Its name, in English: the app translates it.
    pub fn name(self) -> &'static str {
        match self {
            ConnKind::SV => "cold water",
            ConnKind::TV => "hot water",
            ConnKind::K => "waste",
            ConnKind::UKP => "heating supply",
            ConnKind::UKS => "heating return",
            ConnKind::CHP => "chilled water supply",
            ConnKind::CHS => "chilled water return",
            ConnKind::O2 => "oxygen",
            ConnKind::AIR => "medical air",
            ConnKind::VAC => "vacuum",
            ConnKind::N2O => "nitrous oxide",
            ConnKind::C => "circulation",
            ConnKind::P => "gas",
        }
    }
}

/// A connector in local coordinates: across, from the wall, and height.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalConn {
    pub kind: ConnKind,
    pub at: DVec3,
    /// Its nominal size, DN.
    pub dn: f64,
}

/// A connector where it is in the model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Connector {
    pub kind: ConnKind,
    /// In plan, and the height above the fixture's level.
    pub at: DVec3,
    pub dn: f64,
}

/// A heater, towel rail or convector: on heating, not water.
pub fn is_heating(family: &str) -> bool {
    let f = family.to_lowercase();
    [
        "teleso",
        "radiátor",
        "radiator",
        "rebrík",
        "rebrik",
        "konvektor",
    ]
    .iter()
    .any(|k| f.contains(k))
}

/// A fan-coil: on chilled water, with a condensate drain.
pub fn is_cooling(family: &str) -> bool {
    let f = family.to_lowercase();
    f.contains("fan-coil") || f.contains("fancoil") || f.contains("fan coil")
}

/// An underfloor heating manifold.
pub fn is_manifold(family: &str) -> bool {
    let f = family.to_lowercase();
    ["rozdeľovač", "rozdelovac", "manifold"]
        .iter()
        .any(|k| f.contains(k))
}

/// A sanitary fixture: marked and listed in the plumbing legend.
pub fn is_sanitary(family: &str) -> bool {
    !is_heating(family) && !is_manifold(family) && !is_cooling(family)
}

/// The usual length of a heater, m.
pub const HEATER_LEN: f64 = 1.0;

fn v(x: f64, y: f64) -> DVec2 {
    DVec2::new(x, y)
}

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<DVec2> {
    vec![v(x0, y0), v(x1, y0), v(x1, y1), v(x0, y1), v(x0, y0)]
}

fn ellipse(cx: f64, cy: f64, rx: f64, ry: f64, n: usize) -> Vec<DVec2> {
    (0..=n)
        .map(|i| {
            let a = i as f64 / n as f64 * std::f64::consts::TAU;
            v(cx + rx * a.cos(), cy + ry * a.sin())
        })
        .collect()
}

fn rounded(x0: f64, y0: f64, x1: f64, y1: f64, r: f64) -> Vec<DVec2> {
    let mut pts = vec![];
    let mut arc = |cx: f64, cy: f64, a0: f64| {
        for i in 0..=6 {
            let a = a0 + i as f64 / 6.0 * std::f64::consts::FRAC_PI_2;
            pts.push(v(cx + r * a.cos(), cy + r * a.sin()));
        }
    };
    use std::f64::consts::PI;
    arc(x1 - r, y0 + r, -PI / 2.0);
    arc(x1 - r, y1 - r, 0.0);
    arc(x0 + r, y1 - r, PI / 2.0);
    arc(x0 + r, y0 + r, PI);
    pts.push(pts[0]);
    pts
}

fn line(a: (f64, f64), b: (f64, f64)) -> Vec<DVec2> {
    vec![v(a.0, a.1), v(b.0, b.1)]
}

/// A family's size and plan symbol; `len` the length of a heater,
/// manifold or fan-coil.
pub fn shape(family: &str, len: Option<f64>) -> Shape {
    let f = family.to_lowercase();
    let s = |width, depth, height, bottom, curves, label| Shape {
        width,
        depth,
        height,
        bottom,
        curves,
        label,
    };
    if is_manifold(family) {
        let w = len.unwrap_or(0.6);
        return s(
            w,
            0.11,
            1.0,
            0.4,
            vec![
                rect(-w / 2.0, 0.01, w / 2.0, 0.12),
                line((-w / 2.0, 0.01), (w / 2.0, 0.12)),
            ],
            Some("RZ"),
        );
    }
    if is_cooling(family) {
        if f.contains("kazet") {
            return s(
                0.6,
                0.6,
                2.6,
                2.35,
                vec![
                    rect(-0.3, 0.0, 0.3, 0.6),
                    rect(-0.2, 0.1, 0.2, 0.5),
                    line((-0.3, 0.0), (0.3, 0.6)),
                    line((0.3, 0.0), (-0.3, 0.6)),
                ],
                Some("FCK"),
            );
        }
        if f.contains("parapet") {
            let w = len.unwrap_or(1.0);
            return s(
                w,
                0.22,
                0.65,
                0.1,
                vec![
                    rect(-w / 2.0, 0.01, w / 2.0, 0.23),
                    line((-w / 2.0, 0.12), (w / 2.0, 0.12)),
                ],
                Some("FC"),
            );
        }
        let w = len.unwrap_or(0.9);
        return s(
            w,
            0.22,
            2.3,
            2.0,
            vec![
                rect(-w / 2.0, 0.01, w / 2.0, 0.23),
                line((-w / 2.0, 0.16), (w / 2.0, 0.16)),
            ],
            Some("FC"),
        );
    }
    if is_heating(family) {
        if f.contains("rebrík") || f.contains("rebrik") {
            let w = len.unwrap_or(0.5);
            return s(
                w,
                0.08,
                1.4,
                0.2,
                vec![
                    rect(-w / 2.0, 0.03, w / 2.0, 0.11),
                    line((-w / 2.0, 0.07), (w / 2.0, 0.07)),
                ],
                Some("R"),
            );
        }
        let w = len.unwrap_or(HEATER_LEN);
        return s(
            w,
            0.13,
            0.75,
            0.15,
            vec![
                rect(-w / 2.0, 0.04, w / 2.0, 0.14),
                line((-w / 2.0, 0.09), (w / 2.0, 0.09)),
            ],
            None,
        );
    }
    // hung on the wall: the pan alone, its cistern and frame in the wall
    // behind the flush plate (the usual 360 × 530 pan, rim 400 mm)
    if f.contains("závesné wc") {
        return s(
            0.36,
            0.53,
            0.42,
            0.07,
            vec![
                rect(-0.123, 0.0, 0.123, 0.012),
                rounded(-0.18, 0.012, 0.18, 0.53, 0.17),
                ellipse(0.0, 0.31, 0.12, 0.17, 28),
            ],
            None,
        );
    }
    if f.contains("wc") || f.contains("záchod") {
        return s(
            0.4,
            0.68,
            0.42,
            0.0,
            vec![
                rect(-0.2, 0.0, 0.2, 0.17),
                ellipse(0.0, 0.43, 0.18, 0.24, 28),
                ellipse(0.0, 0.45, 0.12, 0.17, 28),
            ],
            None,
        );
    }
    if f.contains("umýv") && !f.contains("riad") && !f.contains("umývačka") {
        return s(
            0.6,
            0.46,
            0.85,
            0.7,
            vec![
                rounded(-0.3, 0.0, 0.3, 0.46, 0.06),
                ellipse(0.0, 0.24, 0.22, 0.15, 28),
                ellipse(0.0, 0.24, 0.02, 0.02, 10),
            ],
            None,
        );
    }
    if f.contains("sprch") {
        return s(
            0.9,
            0.9,
            0.05,
            0.0,
            vec![
                rect(-0.45, 0.0, 0.45, 0.9),
                line((-0.45, 0.0), (0.45, 0.9)),
                line((0.45, 0.0), (-0.45, 0.9)),
                ellipse(0.0, 0.45, 0.05, 0.05, 12),
            ],
            None,
        );
    }
    if f.contains("vaň") {
        return s(
            1.7,
            0.75,
            0.55,
            0.0,
            vec![
                rect(-0.85, 0.0, 0.85, 0.75),
                rounded(-0.78, 0.07, 0.78, 0.68, 0.25),
                ellipse(-0.62, 0.375, 0.03, 0.03, 10),
            ],
            None,
        );
    }
    if f.contains("drez") {
        let mut curves = vec![
            rect(-0.4, 0.0, 0.4, 0.6),
            rounded(-0.36, 0.08, 0.04, 0.52, 0.04),
        ];
        for y in [0.14, 0.22, 0.3, 0.38, 0.46] {
            curves.push(line((0.1, y), (0.36, y)));
        }
        return s(0.8, 0.6, 0.9, 0.0, curves, None);
    }
    if f.contains("prač") || f.contains("práč") {
        return s(
            0.6,
            0.6,
            0.85,
            0.0,
            vec![rect(-0.3, 0.0, 0.3, 0.6), ellipse(0.0, 0.32, 0.2, 0.2, 28)],
            Some("PM"),
        );
    }
    if f.contains("umývačka") || f.contains("riad") {
        return s(
            0.6,
            0.6,
            0.85,
            0.0,
            vec![rect(-0.3, 0.0, 0.3, 0.6), line((-0.3, 0.6), (0.3, 0.0))],
            Some("UR"),
        );
    }
    if f.contains("bidet") {
        return s(
            0.38,
            0.55,
            0.4,
            0.0,
            vec![
                ellipse(0.0, 0.3, 0.18, 0.25, 28),
                ellipse(0.0, 0.3, 0.1, 0.16, 28),
            ],
            None,
        );
    }
    if f.contains("pisoár") {
        return s(
            0.36,
            0.32,
            0.65,
            0.4,
            vec![rounded(-0.18, 0.0, 0.18, 0.32, 0.12)],
            None,
        );
    }
    if f.contains("vpus") {
        return s(
            0.15,
            0.15,
            0.01,
            0.0,
            vec![
                rect(-0.075, 0.0, 0.075, 0.15),
                ellipse(0.0, 0.075, 0.05, 0.05, 14),
            ],
            None,
        );
    }
    // a draw-off tap and the rest
    s(
        0.15,
        0.12,
        0.6,
        0.5,
        vec![
            ellipse(0.0, 0.06, 0.06, 0.06, 14),
            line((0.0, 0.0), (0.0, 0.12)),
        ],
        None,
    )
}

/// A family's connectors in local coordinates.
pub fn local_connectors(family: &str, len: Option<f64>) -> Vec<LocalConn> {
    use ConnKind::*;
    let f = family.to_lowercase();
    let s = shape(family, len);
    let c = |kind, x: f64, y: f64, h: f64, dn: f64| LocalConn {
        kind,
        at: DVec3::new(x, y, h),
        dn,
    };
    if is_manifold(family) {
        return vec![
            c(UKP, -s.width / 2.0 + 0.05, 0.06, s.bottom - 0.05, 25.0),
            c(UKS, -s.width / 2.0 + 0.11, 0.06, s.bottom - 0.05, 25.0),
        ];
    }
    if is_cooling(family) {
        // supply and return on the right seen from the front, condensate
        // on the left; a cassette's up in the ceiling
        let cassette = f.contains("kazet");
        let x = s.width / 2.0 - 0.06;
        let h = if cassette {
            s.bottom + 0.15
        } else {
            s.bottom + 0.05
        };
        let y = if cassette { s.depth / 2.0 } else { 0.08 };
        return vec![
            c(CHP, x - 0.025, y, h, 15.0),
            c(CHS, x + 0.025, y, h, 15.0),
            c(K, -s.width / 2.0 + 0.06, y, h - 0.03, 20.0),
        ];
    }
    if is_heating(family) {
        // bottom connection (valve compact) on the right, 50 mm apart
        let x = s.width / 2.0 - 0.05;
        let h = (s.bottom - 0.07).max(0.05);
        return vec![
            c(UKP, x - 0.025, 0.06, h, 15.0),
            c(UKS, x + 0.025, 0.06, h, 15.0),
        ];
    }
    // in the frame: water to the cistern 1 m up, the waste bend at the
    // wall 230 mm above the floor
    if f.contains("závesné wc") {
        return vec![c(SV, -0.17, 0.0, 1.0, 15.0), c(K, 0.0, 0.0, 0.23, 100.0)];
    }
    if f.contains("wc") || f.contains("záchod") {
        return vec![c(SV, -0.12, 0.03, 0.15, 15.0), c(K, 0.0, 0.05, 0.22, 100.0)];
    }
    if f.contains("umýv") && !f.contains("riad") && !f.contains("umývačka") {
        return vec![
            c(TV, 0.08, 0.02, 0.55, 15.0),
            c(SV, -0.08, 0.02, 0.55, 15.0),
            c(K, 0.0, 0.02, 0.5, 40.0),
        ];
    }
    if f.contains("sprch") {
        return vec![
            c(TV, 0.08, 0.02, 1.1, 15.0),
            c(SV, -0.08, 0.02, 1.1, 15.0),
            c(K, 0.0, 0.45, 0.0, 50.0),
        ];
    }
    if f.contains("vaň") {
        return vec![
            c(TV, -0.55, 0.02, 0.7, 15.0),
            c(SV, -0.69, 0.02, 0.7, 15.0),
            c(K, -0.62, 0.375, 0.0, 40.0),
        ];
    }
    if f.contains("drez") {
        return vec![
            c(TV, 0.08, 0.02, 0.5, 15.0),
            c(SV, -0.08, 0.02, 0.5, 15.0),
            c(K, 0.0, 0.02, 0.45, 50.0),
        ];
    }
    if f.contains("prač") || f.contains("práč") {
        return vec![c(SV, 0.2, 0.02, 0.9, 15.0), c(K, -0.2, 0.02, 0.6, 40.0)];
    }
    if f.contains("umývačka") || f.contains("riad") {
        return vec![c(SV, 0.2, 0.02, 0.4, 15.0), c(K, -0.2, 0.02, 0.4, 40.0)];
    }
    if f.contains("bidet") {
        return vec![
            c(TV, 0.06, 0.02, 0.15, 15.0),
            c(SV, -0.06, 0.02, 0.15, 15.0),
            c(K, 0.0, 0.05, 0.18, 40.0),
        ];
    }
    if f.contains("pisoár") {
        return vec![c(SV, 0.0, 0.02, 1.2, 15.0), c(K, 0.0, 0.02, 0.55, 50.0)];
    }
    if f.contains("vpus") {
        return vec![c(K, 0.0, s.depth / 2.0, 0.0, 50.0)];
    }
    vec![c(SV, 0.0, 0.02, 0.6, 15.0)]
}

/// From PilonDesigner's local axes to an object's own.
pub fn to_object_axes(p: DVec3) -> DVec3 {
    DVec3::new(-p.x, -p.y, p.z)
}

/// The family an object type is a fixture of: by its catalogue entry,
/// else its IFC class and type; none for what takes no pipe.
pub fn family_of_type(t: &ObjectType) -> Option<&'static str> {
    let key = t.catalogue.as_deref().unwrap_or("");
    let key = key.rsplit('/').next().unwrap_or(key);
    let by_key = match key {
        "washbasin" | "pd-basin" => Some("Umývadlo"),
        "wc" | "pd-wc" => Some("WC s nádržkou"),
        "bathtub" | "bath" => Some("Vaňa"),
        k if k.starts_with("bathtub-") => Some("Vaňa"),
        "shower" => Some("Sprcha"),
        "sink-unit" | "sink-unit-1000" | "sink" => Some("Kuchynský drez"),
        "floor-drain" => Some("Podlahová vpusť"),
        "outside-tap" => Some("Výtokový ventil"),
        "radiator" => Some("Doskové vykurovacie teleso"),
        k if k.starts_with("jika-basin") => Some("Umývadlo"),
        k if k.starts_with("jika-wc") => Some("Závesné WC s podomietkovou nádržkou"),
        _ => None,
    };
    if by_key.is_some() {
        return by_key;
    }
    if let Some(f) = key.strip_prefix("pd-") {
        return FAMILIES.iter().copied().find(|x| slug(x) == f);
    }
    Some(match (t.ifc.class.as_str(), t.ifc.predefined.as_str()) {
        ("IfcSanitaryTerminal", "WASHHANDBASIN") => "Umývadlo",
        ("IfcSanitaryTerminal", "TOILETPAN" | "WCSEAT" | "CISTERN") => "WC s nádržkou",
        ("IfcSanitaryTerminal", "BATH") => "Vaňa",
        ("IfcSanitaryTerminal", "SHOWER") => "Sprcha",
        ("IfcSanitaryTerminal", "SINK") => "Kuchynský drez",
        ("IfcSanitaryTerminal", "BIDET") => "Bidet",
        ("IfcSanitaryTerminal", "URINAL") => "Pisoár",
        ("IfcWasteTerminal", "FLOORWASTE" | "FLOORTRAP" | "GULLYTRAP") => "Podlahová vpusť",
        ("IfcValve", "FAUCET" | "DRAWOFFCOCK") => "Výtokový ventil",
        ("IfcElectricAppliance", "WASHINGMACHINE") => "Práčka",
        ("IfcElectricAppliance", "DISHWASHER") => "Umývačka riadu",
        ("IfcSpaceHeater", "RADIATOR" | "CONVECTOR" | "USERDEFINED" | "NOTDEFINED") => {
            "Doskové vykurovacie teleso"
        }
        _ => return None,
    })
}

/// A family's catalogue key, after `pd-`: ASCII, lower case, hyphens.
pub fn slug(family: &str) -> String {
    let mut out = String::new();
    for c in family.to_lowercase().chars() {
        let c = match c {
            'á' | 'ä' => 'a',
            'č' => 'c',
            'ď' => 'd',
            'é' => 'e',
            'í' => 'i',
            'ĺ' | 'ľ' => 'l',
            'ň' => 'n',
            'ó' | 'ô' => 'o',
            'ŕ' => 'r',
            'š' => 's',
            'ť' => 't',
            'ú' => 'u',
            'ý' => 'y',
            'ž' => 'z',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        };
        if c != '-' || !out.ends_with('-') {
            out.push(c);
        }
    }
    out.trim_matches('-').to_string()
}

/// The family a placed object is a fixture of.
pub fn family_of(model: &Model, e: &Element) -> Option<&'static str> {
    family_of_type(crate::objects::type_of(model, e)?)
}

/// The length a heater, manifold or fan-coil of type `t` has: its width.
fn length_of(family: &str, t: &ObjectType) -> Option<f64> {
    if is_heating(family) || is_manifold(family) || is_cooling(family) {
        // a radiator's panels, not the valves standing out past them
        let panels = t.parts.iter().filter(|p| p.name == "Panel");
        let xs = panels.flat_map(|p| p.mesh.vertices.iter().map(|v| v.x));
        let (lo, hi) = xs.fold((f64::MAX, f64::MIN), |(a, b), x| (a.min(x), b.max(x)));
        if hi > lo {
            return Some(hi - lo);
        }
        let (lo, hi) = t.bounds();
        Some(hi.x - lo.x)
    } else {
        None
    }
}

/// The connections a type carries as parts named `Connection <system>`
/// or `Connection <system> DN<size>` (a medical supply unit's gases, a
/// boiler's or a cylinder's pipes): each at the middle of its part's
/// top, in the object's own axes; Cu 12 × 1 where no size is said.
pub fn part_connectors(t: &ObjectType) -> Vec<LocalConn> {
    t.parts
        .iter()
        .filter_map(|p| {
            let mut words = p.name.strip_prefix("Connection ")?.split(' ');
            let kind = match words.next()? {
                "O2" => ConnKind::O2,
                "AIR" => ConnKind::AIR,
                "VAC" => ConnKind::VAC,
                "N2O" => ConnKind::N2O,
                "SV" => ConnKind::SV,
                "TV" => ConnKind::TV,
                "C" => ConnKind::C,
                "K" => ConnKind::K,
                "UKP" => ConnKind::UKP,
                "UKS" => ConnKind::UKS,
                "P" => ConnKind::P,
                _ => return None,
            };
            // medical gases join in Cu 12 × 1; others say their size
            let dn = words
                .next()
                .and_then(|w| w.strip_prefix("DN"))
                .and_then(|w| w.parse().ok())
                .unwrap_or(12.0);
            let first = *p.mesh.vertices.first()?;
            let (lo, hi) = p
                .mesh
                .vertices
                .iter()
                .fold((first, first), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
            Some(LocalConn {
                kind,
                at: ((lo + hi) / 2.0).truncate().extend(hi.z),
                dn,
            })
        })
        .collect()
}

/// A placed object's connectors: in plan, and the height above its level.
/// Heights are PilonDesigner's above the floor, moved with an object
/// raised or lowered from its type's height; for one hung from the
/// ceiling, as far below its top as the family's are below its top.
pub fn connectors(model: &Model, e: &Element) -> Vec<Connector> {
    let ElementKind::Object(o) = &e.kind else {
        return vec![];
    };
    let Some(t) = crate::objects::type_of(model, e) else {
        return vec![];
    };
    let floor = model.level(o.level).map_or(0.0, |l| l.elevation);
    // a type that carries its own connections (a medical supply unit)
    let own = part_connectors(t);
    if !own.is_empty() {
        let base = crate::objects::base_z_of(model, o, t) - floor;
        return own
            .into_iter()
            .map(|c| Connector {
                kind: c.kind,
                at: o.to_plan(c.at.truncate()).extend(base + c.at.z),
                dn: c.dn,
            })
            .collect();
    }
    let Some(family) = family_of_type(t) else {
        // a gas cooker: its gas tap behind it, at the side, under the
        // worktop
        if crate::objects::Supply::of(t).is_some_and(|s| s.gas_kw.is_some()) {
            let (lo, hi) = t.bounds();
            let base = crate::objects::base_z_of(model, o, t) - floor;
            let at = DVec2::new(hi.x - 0.1, hi.y - 0.03);
            return vec![Connector {
                kind: ConnKind::P,
                at: o.to_plan(at).extend(base + (lo.z + 0.5).min(hi.z)),
                dn: 15.0,
            }];
        }
        return vec![];
    };
    let len = length_of(family, t);
    let s = shape(family, len);
    let floor = model.level(o.level).map_or(0.0, |l| l.elevation);
    let base = crate::objects::base_z_of(model, o, t) - floor;
    let top = base + t.bounds().1.z;
    local_connectors(family, len)
        .into_iter()
        .map(|c| {
            let p = to_object_axes(c.at);
            let z = if t.mount == Mount::Ceiling {
                top - (s.height - c.at.z)
            } else {
                c.at.z + o.offset - t.offset
            };
            Connector {
                kind: c.kind,
                at: o.to_plan(p.truncate()).extend(z),
                dn: c.dn,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_basin_has_hot_and_cold_water_and_a_waste() {
        let c = local_connectors("Umývadlo", None);
        let kinds: Vec<_> = c.iter().map(|c| c.kind).collect();
        assert_eq!(kinds, vec![ConnKind::TV, ConnKind::SV, ConnKind::K]);
        assert_eq!(c[0].at, DVec3::new(0.08, 0.02, 0.55));
        assert_eq!(c[2].dn, 40.0);
        let s = shape("Umývadlo", None);
        assert_eq!((s.width, s.depth, s.bottom), (0.6, 0.46, 0.7));
        assert_eq!(slug("Kúpeľňový rebrík"), "kupelnovy-rebrik");
        assert_eq!(slug("Fan-coil kazetový (strop)"), "fan-coil-kazetovy-strop");
    }

    #[test]
    fn heaters_manifolds_and_fan_coils_join_their_own_systems() {
        let h = "Doskové vykurovacie teleso";
        assert!(is_heating(h) && !is_sanitary(h));
        let kinds: Vec<_> = local_connectors(h, None)
            .iter()
            .map(|c| c.kind.system())
            .collect();
        assert_eq!(
            kinds,
            vec![SystemKind::HeatingSupply, SystemKind::HeatingReturn]
        );
        assert_eq!(shape(h, Some(1.6)).width, 1.6);
        assert!(is_manifold("Rozdeľovač podlahového vykurovania"));
        let fc = local_connectors("Fan-coil kazetový (strop)", None);
        assert_eq!(fc[0].kind, ConnKind::CHP);
        assert!((fc[0].at.z - 2.5).abs() < 1e-12);
        assert_eq!(shape("Fan-coil kazetový (strop)", None).label, Some("FCK"));
    }

    #[test]
    fn every_family_has_a_shape_and_a_connector() {
        for f in FAMILIES {
            let s = shape(f, None);
            assert!(
                s.width > 0.0 && s.depth > 0.0 && s.height >= s.bottom,
                "{f}"
            );
            assert!(!s.curves.is_empty(), "{f}");
            assert!(!local_connectors(f, None).is_empty(), "{f}");
        }
        assert_eq!(shape("Práčka", None).label, Some("PM"));
        assert_eq!(shape("Umývačka riadu", None).label, Some("UR"));
    }

    #[test]
    fn catalogue_objects_are_fixtures_of_their_families() {
        let cat = crate::catalogue::starters();
        let family = |key: &str| {
            let o = cat
                .iter()
                .find(|o| o.key == format!("starter/{key}"))
                .unwrap();
            family_of_type(&o.object)
        };
        assert_eq!(family("washbasin"), Some("Umývadlo"));
        assert_eq!(family("wc"), Some("WC s nádržkou"));
        assert_eq!(family("radiator"), Some("Doskové vykurovacie teleso"));
        assert_eq!(family("floor-drain"), Some("Podlahová vpusť"));
        assert_eq!(family("sink-unit-1000"), Some("Kuchynský drez"));
        assert_eq!(family("bathtub"), Some("Vaňa"));
        assert_eq!(family("socket-double"), None);
        for f in FAMILIES {
            assert!(
                cat.iter().any(|o| family_of_type(&o.object) == Some(f)),
                "no catalogue object for {f}"
            );
        }
    }

    #[test]
    fn a_placed_basin_turns_and_raises_its_connectors() {
        use crate::Document;
        let mut doc = Document::from_model(crate::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let basin = crate::catalogue::starter("starter/washbasin").unwrap();
        let (tx, id) =
            crate::objects::place(doc.model(), &basin, level, DVec2::new(2.0, 3.0), 90.0);
        doc.commit(tx).unwrap();
        let e = doc.model().get(id).unwrap();
        assert_eq!(family_of(doc.model(), e), Some("Umývadlo"));
        let c = connectors(doc.model(), e);
        assert_eq!(c.len(), 3);
        // TV 80 mm across and 20 mm out from the wall: turned a quarter,
        // the wall's along y and out is +x.
        let tv = c[0];
        assert_eq!(tv.kind, ConnKind::TV);
        assert!((tv.at.x - 2.02).abs() < 1e-9, "{:?}", tv.at);
        assert!((tv.at.y - 2.92).abs() < 1e-9, "{:?}", tv.at);
        assert!((tv.at.z - 0.55).abs() < 1e-9, "{:?}", tv.at);
    }

    #[test]
    fn a_medical_rail_takes_its_gases_on_top() {
        use crate::catalogue::medical::{unit, MedUnitConfig, MedUnitKind};
        use crate::Document;
        let mut doc = Document::from_model(crate::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let cfg = MedUnitConfig::usual(MedUnitKind::Rail);
        let rail = unit(&cfg);
        let (tx, id) = crate::objects::place(doc.model(), &rail, level, DVec2::new(3.0, 5.85), 0.0);
        doc.commit(tx).unwrap();
        let e = doc.model().get(id).unwrap();
        let c = connectors(doc.model(), e);
        let kinds: Vec<_> = c.iter().map(|c| c.kind).collect();
        assert_eq!(kinds, vec![ConnKind::O2, ConnKind::AIR, ConnKind::VAC]);
        // oxygen first from the left seen from the front, 25 mm off the
        // wall, on top of the profile: its axis 1.45 m up
        let o2 = c[0];
        assert!((o2.at.x - (3.0 - 0.622)).abs() < 1e-9, "{}", o2.at);
        assert!((o2.at.y - (5.85 - 0.025)).abs() < 1e-9, "{}", o2.at);
        assert!((o2.at.z - (1.45 + 0.1165 + 0.03)).abs() < 1e-9, "{}", o2.at);
        assert_eq!((o2.dn, o2.kind.code()), (12.0, "O2"));
    }
}
