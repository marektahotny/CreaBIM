//! KORADO RADIK VK steel panel radiators (STN EN 442): valve built in,
//! connected from below on the right at 50 mm spacing, G 1/2" inside,
//! 110 °C and 1.0 MPa at most. Types 10, 11, 20, 21, 22 and 33 in the
//! heights and lengths KORADO makes, each with the output KORADO's own
//! table gives at 75/65/20 °C (korado.cz, RADIK VK, "Výkony a technické
//! informace", read 22 Nov 2026). A size KORADO does not make gives no
//! output.
//!
//! The exponent of each type and height is taken from the same table:
//! the output of the longest radiator made at 75/65/20 °C against
//! 45/35/20 °C, the temperature difference per STN EN 442-2. Against
//! KORADO's calculator at thirteen other temperatures it is within
//! 0.3 %.
//!
//! A radiator is a catalogue object whose key says its size
//! (`radiator-22-600-1000`), so one standing in a model can be read back
//! ([`Panel::of_key`]).

use super::{appliance_white, cuboid, rrect, soft_box, v2, v3, Build};
use crate::objects::{CatalogueObject, Mount, ObjectCategory};

/// The panels and convector fins a radiator has.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PanelKind {
    /// One panel, no fins.
    K10,
    /// One panel, one row of fins.
    K11,
    /// Two panels, no fins.
    K20,
    /// Two panels, one row of fins.
    K21,
    /// Two panels, two rows of fins.
    K22,
    /// Three panels, three rows of fins.
    K33,
}

/// The heights KORADO makes, mm.
pub const HEIGHTS: [u32; 6] = [300, 400, 500, 600, 700, 900];

/// The lengths KORADO makes, mm.
const LENGTHS: [u32; 16] = [
    400, 500, 600, 700, 800, 900, 1000, 1100, 1200, 1400, 1600, 1800, 2000, 2300, 2600, 3000,
];

/// Bottom of a radiator above the floor, m.
pub const OFF_FLOOR: f64 = 0.15;

/// Type 10, W at 75/65/20 °C.
const Q10: [[u16; 6]; 16] = [
    [0, 0, 206, 242, 278, 350],
    [165, 212, 257, 302, 347, 438],
    [198, 254, 308, 362, 416, 525],
    [0, 0, 360, 423, 486, 613],
    [0, 0, 411, 483, 555, 700],
    [0, 0, 463, 544, 625, 788],
    [0, 0, 514, 604, 694, 875],
    [0, 0, 565, 664, 763, 963],
    [0, 0, 617, 725, 833, 1050],
    [0, 0, 720, 846, 972, 1225],
    [0, 0, 822, 966, 1110, 1400],
    [0, 0, 925, 1087, 1249, 0],
    [0, 0, 1028, 1208, 1388, 0],
    [0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0],
    [0, 0, 0, 0, 0, 0],
];
/// Type 11, W at 75/65/20 °C.
const Q11: [[u16; 6]; 16] = [
    [220, 283, 343, 401, 456, 558],
    [275, 354, 429, 501, 570, 697],
    [329, 425, 515, 601, 683, 836],
    [384, 496, 601, 701, 797, 976],
    [439, 566, 686, 802, 911, 1115],
    [494, 637, 772, 902, 1025, 1255],
    [549, 708, 858, 1002, 1139, 1394],
    [604, 779, 944, 1102, 1253, 1533],
    [659, 850, 1030, 1202, 1367, 1673],
    [769, 991, 1201, 1403, 1595, 1952],
    [878, 1133, 1373, 1603, 1822, 2230],
    [988, 1274, 1544, 1804, 2050, 0],
    [1098, 1416, 1716, 2004, 2278, 0],
    [0, 0, 1973, 2305, 2620, 0],
    [0, 0, 2231, 2605, 2961, 0],
    [0, 0, 2574, 3006, 3417, 0],
];
/// Type 20, W at 75/65/20 °C.
const Q20: [[u16; 6]; 16] = [
    [0, 0, 335, 391, 447, 0],
    [0, 0, 419, 489, 559, 0],
    [0, 0, 503, 587, 670, 0],
    [0, 0, 587, 685, 782, 0],
    [0, 0, 670, 782, 894, 0],
    [0, 0, 754, 880, 1005, 0],
    [0, 0, 838, 978, 1117, 0],
    [0, 0, 922, 1076, 1229, 0],
    [0, 0, 1006, 1174, 1340, 0],
    [0, 0, 1173, 1369, 1564, 0],
    [0, 0, 1341, 1565, 1787, 0],
    [0, 0, 1508, 1760, 2011, 0],
    [0, 0, 1676, 1956, 2234, 0],
    [0, 0, 1927, 2249, 2569, 0],
    [0, 0, 2179, 2543, 2904, 0],
    [0, 0, 2514, 2934, 3351, 0],
];
/// Type 21, W at 75/65/20 °C.
const Q21: [[u16; 6]; 16] = [
    [298, 375, 447, 515, 580, 702],
    [373, 469, 559, 644, 725, 877],
    [447, 562, 670, 773, 870, 1052],
    [522, 656, 782, 902, 1015, 1228],
    [596, 750, 894, 1030, 1160, 1403],
    [671, 843, 1005, 1159, 1305, 1579],
    [745, 937, 1117, 1288, 1450, 1754],
    [820, 1031, 1229, 1417, 1595, 1929],
    [894, 1124, 1340, 1546, 1740, 2105],
    [1043, 1312, 1564, 1803, 2030, 2456],
    [1192, 1499, 1787, 2061, 2320, 2806],
    [1341, 1687, 2011, 2318, 2610, 3157],
    [1490, 1874, 2234, 2576, 2900, 3508],
    [0, 0, 2569, 2962, 3335, 0],
    [0, 0, 2904, 3349, 3770, 0],
    [0, 0, 3351, 3864, 4350, 0],
];
/// Type 22, W at 75/65/20 °C.
const Q22: [[u16; 6]; 16] = [
    [386, 486, 581, 672, 759, 925],
    [483, 608, 726, 840, 949, 1157],
    [580, 730, 871, 1007, 1138, 1388],
    [676, 851, 1016, 1175, 1328, 1619],
    [773, 973, 1162, 1343, 1518, 1850],
    [869, 1094, 1307, 1511, 1707, 2082],
    [966, 1216, 1452, 1679, 1897, 2313],
    [1063, 1338, 1597, 1847, 2087, 2544],
    [1159, 1459, 1742, 2015, 2276, 2776],
    [1352, 1702, 2033, 2351, 2656, 3238],
    [1546, 1946, 2323, 2686, 3035, 3701],
    [1739, 2189, 2614, 3022, 3415, 4163],
    [1932, 2432, 2904, 3358, 3794, 4626],
    [2222, 2797, 3340, 3862, 4363, 0],
    [2512, 3162, 3775, 4365, 4932, 0],
    [2898, 3648, 4356, 5037, 5691, 0],
];
/// Type 33, W at 75/65/20 °C.
const Q33: [[u16; 6]; 16] = [
    [552, 695, 832, 962, 1089, 1331],
    [690, 869, 1040, 1203, 1362, 1664],
    [827, 1043, 1247, 1444, 1634, 1997],
    [965, 1217, 1455, 1684, 1906, 2330],
    [1103, 1390, 1663, 1925, 2178, 2662],
    [1241, 1564, 1871, 2165, 2451, 2995],
    [1379, 1738, 2079, 2406, 2723, 3328],
    [1517, 1912, 2287, 2647, 2995, 3661],
    [1655, 2086, 2495, 2887, 3268, 3994],
    [1931, 2433, 2911, 3368, 3812, 4659],
    [2206, 2781, 3326, 3850, 4357, 5325],
    [2482, 3128, 3742, 4331, 4901, 5990],
    [2758, 3476, 4158, 4812, 5446, 6656],
    [3172, 3997, 4782, 5534, 6263, 0],
    [3585, 4519, 5405, 6256, 7080, 0],
    [4137, 5214, 6237, 7218, 8169, 0],
];
/// The exponents, by type and height (0: not made).
const N10: [f64; 6] = [1.3279, 1.3152, 1.3028, 1.2910, 1.2953, 1.3045];
const N11: [f64; 6] = [1.3115, 1.3088, 1.3080, 1.3064, 1.3093, 1.3163];
const N20: [f64; 6] = [0.0, 0.0, 1.2956, 1.2964, 1.3144, 0.0];
const N21: [f64; 6] = [1.3154, 1.3190, 1.3232, 1.3273, 1.3359, 1.3535];
const N22: [f64; 6] = [1.3245, 1.3271, 1.3288, 1.3302, 1.3379, 1.3528];
const N33: [f64; 6] = [1.2927, 1.3082, 1.3235, 1.3387, 1.3452, 1.3579];

impl PanelKind {
    /// Slimmest first.
    pub const ALL: [PanelKind; 6] = [
        PanelKind::K10,
        PanelKind::K11,
        PanelKind::K20,
        PanelKind::K21,
        PanelKind::K22,
        PanelKind::K33,
    ];

    /// Its number: 10, 11, 20, 21, 22 or 33.
    pub fn number(self) -> u32 {
        match self {
            PanelKind::K10 => 10,
            PanelKind::K11 => 11,
            PanelKind::K20 => 20,
            PanelKind::K21 => 21,
            PanelKind::K22 => 22,
            PanelKind::K33 => 33,
        }
    }

    pub fn of_number(n: u32) -> Option<PanelKind> {
        PanelKind::ALL.into_iter().find(|k| k.number() == n)
    }

    /// How many panels.
    pub fn panels(self) -> u32 {
        self.number() / 10
    }

    /// Its depth, m (KORADO: 47, 63, 66, 66, 100 and 155 mm).
    pub fn depth(self) -> f64 {
        match self {
            PanelKind::K10 => 0.047,
            PanelKind::K11 => 0.063,
            PanelKind::K20 | PanelKind::K21 => 0.066,
            PanelKind::K22 => 0.100,
            PanelKind::K33 => 0.155,
        }
    }

    fn table(self) -> (&'static [[u16; 6]; 16], &'static [f64; 6]) {
        match self {
            PanelKind::K10 => (&Q10, &N10),
            PanelKind::K11 => (&Q11, &N11),
            PanelKind::K20 => (&Q20, &N20),
            PanelKind::K21 => (&Q21, &N21),
            PanelKind::K22 => (&Q22, &N22),
            PanelKind::K33 => (&Q33, &N33),
        }
    }

    /// The exponent of its characteristic at `height` mm,
    /// Φ = Φn · (Δθ / 50)ⁿ; none for a height not made.
    pub fn exponent(self, height: u32) -> Option<f64> {
        let j = HEIGHTS.iter().position(|h| *h == height)?;
        let n = self.table().1[j];
        (n > 0.0).then_some(n)
    }
}

/// The lengths KORADO makes, mm.
pub fn lengths() -> Vec<u32> {
    LENGTHS.to_vec()
}

/// One radiator of the range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Panel {
    pub kind: PanelKind,
    /// mm.
    pub height: u32,
    /// mm.
    pub length: u32,
}

impl Panel {
    /// KORADO makes it.
    pub fn made(&self) -> bool {
        self.nominal_w() > 0.0
    }

    /// Its output at 75/65/20 °C, W, from KORADO's table; 0 for a size
    /// not made.
    pub fn nominal_w(&self) -> f64 {
        let (Some(i), Some(j)) = (
            LENGTHS.iter().position(|l| *l == self.length),
            HEIGHTS.iter().position(|h| *h == self.height),
        ) else {
            return 0.0;
        };
        self.kind.table().0[i][j] as f64
    }

    /// Its output at the mean temperature difference `dt` K to the
    /// room (STN EN 442-2), W.
    pub fn output_w(&self, dt: f64) -> f64 {
        match self.kind.exponent(self.height) {
            Some(n) if dt > 0.0 => self.nominal_w() * (dt / 50.0).powf(n),
            _ => 0.0,
        }
    }

    /// As makers write it: 22-600×1000.
    pub fn label(&self) -> String {
        format!("{}-{}×{}", self.kind.number(), self.height, self.length)
    }

    /// Its catalogue key.
    pub fn key(&self) -> String {
        format!(
            "radiator-{}-{}-{}",
            self.kind.number(),
            self.height,
            self.length
        )
    }

    /// The radiator a catalogue key names; the starter catalogue's plain
    /// `radiator` is a 22-600×800.
    pub fn of_key(key: &str) -> Option<Panel> {
        let key = key.rsplit('/').next().unwrap_or(key);
        if key == "radiator" {
            return Some(Panel {
                kind: PanelKind::K22,
                height: 600,
                length: 800,
            });
        }
        let mut it = key.strip_prefix("radiator-")?.split('-');
        let kind = PanelKind::of_number(it.next()?.parse().ok()?)?;
        let height = it.next()?.parse().ok()?;
        let length = it.next()?.parse().ok()?;
        (it.next().is_none() && height > 0 && length > 0).then_some(Panel {
            kind,
            height,
            length,
        })
    }

    /// The fittings it is connected with (George: Heimeier): the
    /// Vekolux H-fitting under its bottom right connection, two-pipe,
    /// straight for pipes from the floor (IMI Heimeier, Vekolux
    /// catalogue sheet: 0530-50.000, angle 0531-50.000) and the
    /// thermostatic head K, M30 × 1.5, white (6000-09.500).
    pub const FITTINGS: [(&'static str, &'static str); 2] = [
        (
            "0530-50.000",
            "Heimeier Vekolux, priame, dvojrúrkové, R1/2 – G3/4, rozteč 50 mm",
        ),
        (
            "6000-09.500",
            "Heimeier termostatická hlavica K, M30 × 1,5, biela",
        ),
    ];

    /// As a catalogue object: its back to the wall on y = 0, its bottom
    /// 150 mm off the floor, its body as deep as KORADO makes the type.
    pub fn object(&self) -> CatalogueObject {
        let (w, h) = (self.length as f64 / 1000.0, self.height as f64 / 1000.0);
        let d = self.kind.depth();
        let n = self.kind.panels();
        // the body 30 mm clear of the wall, its panels at its faces
        let gap = 0.03;
        let t = 0.011_f64.min(d / n as f64 / 2.0);
        let step = if n > 1 { (d - t) / (n - 1) as f64 } else { 0.0 };
        let mut b = Build::new();
        for i in 0..n {
            let y1 = -gap - step * i as f64;
            b.add(
                "Panel",
                appliance_white(),
                soft_box(v3(-w / 2.0, y1 - t, 0.0), v3(w / 2.0, y1, h), 0.004, 0.003),
                false,
            );
        }
        if n > 1 || self.kind != PanelKind::K10 {
            // the fins, covers and sides between the panels
            b.add(
                "Fins",
                appliance_white(),
                cuboid(
                    v3(-w / 2.0 + 0.002, -gap - d + 0.002, 0.004),
                    v3(w / 2.0 - 0.002, -gap - 0.002, h - 0.002),
                ),
                false,
            );
        }
        let bx = (w / 2.0 - 0.1).max(0.05);
        for x in [-bx, bx] {
            b.add(
                "Bracket",
                appliance_white(),
                cuboid(v3(x - 0.02, -gap, h - 0.1), v3(x + 0.02, 0.0, h - 0.06)),
                false,
            );
        }
        // its outline, hatched across
        b.outline(rrect(v2(0.0, -gap - d / 2.0), w, d, 0.0, 0));
        let lines = ((w / 0.1).round() as usize).max(4);
        for i in 0..lines {
            let x = -w / 2.0 + w * (i as f64 + 0.5) / lines as f64;
            b.line(v2(x - 0.03, -gap - d), v2(x + 0.03, -gap));
        }
        b.at(OFF_FLOOR);
        b.finish(
            &self.key(),
            &format!("KORADO RADIK VK {}", self.label()),
            &format!(
                "KORADO RADIK VK steel panel radiator type {}, {} × {} mm, {} mm deep, {} W at 75/65/20 °C (KORADO), n = {}; valve built in, connected from below on the right at 50 mm; Heimeier Vekolux 0530-50.000 and thermostatic head K 6000-09.500",
                self.kind.number(),
                self.height,
                self.length,
                (d * 1000.0).round(),
                self.nominal_w().round(),
                self.kind.exponent(self.height).map_or_else(|| "–".into(), |n| format!("{n:.4}")),
            ),
            ObjectCategory::Hvac,
            ("IfcSpaceHeater", "RADIATOR"),
            Mount::Wall,
        )
    }
}

/// A heater standing in a model: a panel radiator or a towel radiator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Heater {
    Panel(Panel),
    Towel(super::towels::TowelRail),
}

impl Heater {
    pub fn of_key(key: &str) -> Option<Heater> {
        Panel::of_key(key)
            .map(Heater::Panel)
            .or_else(|| super::towels::TowelRail::of_key(key).map(Heater::Towel))
    }

    pub fn label(&self) -> String {
        match self {
            Heater::Panel(p) => format!("RADIK VK {}", p.label()),
            Heater::Towel(t) => t.label(),
        }
    }

    /// Output at 75/65/20 °C, W.
    pub fn nominal_w(&self) -> f64 {
        match self {
            Heater::Panel(p) => p.nominal_w(),
            Heater::Towel(t) => t.nominal_w(),
        }
    }

    /// Output at the mean temperature difference `dt` K, W.
    pub fn output_w(&self, dt: f64) -> f64 {
        match self {
            Heater::Panel(p) => p.output_w(dt),
            Heater::Towel(t) => t.output_w(dt),
        }
    }

    /// Its exponent n.
    pub fn exponent(&self) -> Option<f64> {
        match self {
            Heater::Panel(p) => p.kind.exponent(p.height),
            Heater::Towel(t) => t.exponent(),
        }
    }

    /// What it is fitted with: order code and name.
    pub fn fittings(&self) -> Vec<(String, String)> {
        match self {
            Heater::Panel(_) => Panel::FITTINGS
                .iter()
                .map(|(c, n)| (c.to_string(), n.to_string()))
                .collect(),
            Heater::Towel(t) => t.fittings(),
        }
    }

    /// Its own order code, where known.
    pub fn code(&self) -> Option<String> {
        match self {
            Heater::Panel(_) => None,
            Heater::Towel(t) => Some(t.code()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_radiator_reads_back_from_its_key() {
        let p = Panel {
            kind: PanelKind::K22,
            height: 600,
            length: 1000,
        };
        assert_eq!(p.key(), "radiator-22-600-1000");
        assert_eq!(Panel::of_key(&p.key()), Some(p));
        assert_eq!(Panel::of_key("starter/radiator").unwrap().length, 800);
        assert_eq!(Panel::of_key("radiator-12-600-1000"), None);
        assert_eq!(Panel::of_key("towel-radiator"), None);
        assert_eq!(p.label(), "22-600×1000");
    }

    #[test]
    fn output_is_korado_s_table() {
        let p = Panel {
            kind: PanelKind::K22,
            height: 600,
            length: 1000,
        };
        assert_eq!(p.nominal_w(), 1679.0);
        assert!((p.output_w(50.0) - 1679.0).abs() < 1e-6);
        // KORADO's calculator, 22-600×3000: 45/35/20 gives 1447 W,
        // 55/45/20 2546 W, 75/45/20 3513 W, 90/70/20 6425 W
        let long = Panel { length: 3000, ..p };
        let lm = |a: f64, b: f64, i: f64| (a - b) / ((a - i) / (b - i)).ln();
        for (dt, korado) in [
            (lm(45.0, 35.0, 20.0), 1447.0),
            (30.0, 2546.0),
            (lm(75.0, 45.0, 20.0), 3513.0),
            (60.0, 6425.0),
        ] {
            let w = long.output_w(dt);
            assert!((w / korado - 1.0).abs() < 0.004, "{dt}: {w} vs {korado}");
        }
        assert_eq!(p.output_w(-1.0), 0.0);
        // sizes not made give nothing
        let no = Panel {
            kind: PanelKind::K10,
            height: 300,
            length: 1000,
        };
        assert!(!no.made() && no.output_w(50.0) == 0.0);
        assert!(!Panel { height: 550, ..p }.made());
        // every value of KORADO's six tables, summed
        let mut sum = 0.0;
        let mut count = 0;
        for k in PanelKind::ALL {
            for h in HEIGHTS {
                for l in lengths() {
                    let w = Panel {
                        kind: k,
                        height: h,
                        length: l,
                    }
                    .nominal_w();
                    if w > 0.0 {
                        sum += w;
                        count += 1;
                        assert!(k.exponent(h).is_some());
                    }
                }
            }
        }
        assert_eq!((count, sum), (460, 792_377.0));
        // more panels give more at a height every type has
        let w: Vec<f64> = [
            PanelKind::K11,
            PanelKind::K21,
            PanelKind::K22,
            PanelKind::K33,
        ]
        .iter()
        .map(|k| {
            Panel {
                kind: *k,
                height: 600,
                length: 1000,
            }
            .nominal_w()
        })
        .collect();
        assert!(w.windows(2).all(|x| x[0] < x[1]), "{w:?}");
        assert_eq!(lengths().first(), Some(&400));
        assert_eq!(lengths().last(), Some(&3000));
    }

    #[test]
    fn the_object_is_a_heater_as_long_as_its_panels() {
        let p = Panel {
            kind: PanelKind::K33,
            height: 500,
            length: 1400,
        };
        let o = p.object();
        assert_eq!(Panel::of_key(&o.key), Some(p));
        let panels = o.object.parts.iter().filter(|x| x.name == "Panel").count();
        assert_eq!(panels, 3);
    }
}
