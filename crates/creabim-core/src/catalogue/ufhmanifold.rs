//! An underfloor heating manifold made to order from a maker's range: the
//! number of circuits (the loops and any extra outlets, for radiators fed
//! through the floor's insulation layer) picks the size, the range its
//! length, the bars' distance and the cabinet that takes it.
//!
//! Sources (read on the makers' sites, 27. 11. 2026, copied in the
//! project files under vyrobcovia/rozdelovace):
//! - HERZ 1 8532 xx brass manifold set DN 25 (technical sheet 853X,
//!   CZ 0909): 3 to 16 circuits, L = 221 + 50·(n − 3) mm, bars 200 mm
//!   apart, brackets 313 mm high, inlet 1" female, outlets G 3/4
//!   Eurocone; flowmeters on the supply, thermostatic valves M28×1.5 on
//!   the return; 120 °C, 10 bar. Cabinet table 18532xx and sheet
//!   1 85XX XX-000 (flush cabinets 1 8572 10…30, 534…1134 mm wide).
//! - HERZ PUSH 2010/16 composite manifold QF2327xx: 2 to 12 circuits,
//!   lengths from its sheet, bars 220 mm apart, 365 mm high with the
//!   hand wheels, 96 mm deep; for 16 mm plastic pipe without aluminium
//!   only (PE-RT, PB, PEX, PP); 70 °C, 6 bar test.
//! - Wavin stainless (XF1565xxW) and composite (XF1570xxW) manifolds:
//!   2 to 12 circuits, lengths from the Wavin technical handbook for
//!   surface heating (p. 48), 50 mm outlet pitch, bars 200 / 210 mm
//!   apart, flowmeter 0 to 6 l/min; cabinets XF1580/XF1582 for 7 to 14
//!   circuits (wavin.com/cz).

use super::{cuboid, cylinder, metal, v2, v3, Build};
use crate::objects::{CatalogueObject, Mount, ObjectCategory};
use serde::{Deserialize, Serialize};

/// A maker's range of manifolds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Range {
    /// HERZ 1 8532 xx, brass, with flowmeters.
    HerzBrass,
    /// HERZ PUSH 2010/16, composite, PE-RT 16 only.
    HerzPush,
    /// Wavin stainless steel.
    WavinSteel,
    /// Wavin composite.
    WavinComposite,
}

impl Range {
    pub const ALL: [Range; 4] = [
        Range::HerzBrass,
        Range::HerzPush,
        Range::WavinSteel,
        Range::WavinComposite,
    ];

    pub fn maker(self) -> &'static str {
        match self {
            Range::HerzBrass | Range::HerzPush => "HERZ",
            Range::WavinSteel | Range::WavinComposite => "Wavin",
        }
    }

    /// Its name in the catalogue and the legend.
    pub fn label(self) -> &'static str {
        match self {
            Range::HerzBrass => "HERZ 1 8532 (mosadz, DN 25)",
            Range::HerzPush => "HERZ PUSH 2010/16 (kompozit)",
            Range::WavinSteel => "Wavin nerez",
            Range::WavinComposite => "Wavin kompozit",
        }
    }

    /// The fewest and most circuits it is made with.
    pub fn circuits(self) -> (u32, u32) {
        match self {
            Range::HerzBrass => (3, 16),
            _ => (2, 12),
        }
    }

    /// Its length with `n` circuits, mm.
    pub fn length_mm(self, n: u32) -> f64 {
        let n = n.clamp(self.circuits().0, self.circuits().1);
        match self {
            Range::HerzBrass => 221.0 + 50.0 * (n as f64 - 3.0),
            Range::HerzPush => {
                const L: [f64; 11] = [
                    193.0, 248.0, 303.0, 359.0, 415.0, 470.0, 525.0, 580.0, 636.0, 692.0, 747.0,
                ];
                L[(n - 2) as usize]
            }
            Range::WavinSteel => 175.0 + 50.0 * (n as f64 - 2.0),
            Range::WavinComposite => 245.0 + 50.0 * (n as f64 - 2.0),
        }
    }

    /// The supply and return bars' axes apart, mm.
    pub fn bars_mm(self) -> f64 {
        match self {
            Range::HerzBrass | Range::WavinSteel => 200.0,
            Range::HerzPush => 220.0,
            Range::WavinComposite => 210.0,
        }
    }

    /// The outlets' pitch along a bar, mm (HERZ PUSH: its length step).
    pub fn pitch_mm(self) -> f64 {
        match self {
            Range::HerzPush => (747.0 - 193.0) / 10.0,
            _ => 50.0,
        }
    }

    /// Its order code with `n` circuits.
    pub fn code(self, n: u32) -> String {
        match self {
            Range::HerzBrass => format!("1 8532 {n:02}"),
            Range::HerzPush => format!("QF2327{:02}", 30 + n),
            Range::WavinSteel => format!("XF1565{n:02}W"),
            Range::WavinComposite => format!("XF1570{n:02}W"),
        }
    }

    /// Whether it takes pipe of `outer_mm` with (`aluminium`) or without
    /// an aluminium layer.
    pub fn takes(self, outer_mm: f64, aluminium: bool) -> bool {
        match self {
            Range::HerzPush => (outer_mm - 16.0).abs() < 0.5 && !aluminium,
            _ => true,
        }
    }

    /// The range for a maker's pipe: Wavin's stainless one for Wavin,
    /// else HERZ's brass one, which takes every pipe (HERZ PUSH only
    /// 16 mm plastic pipe without aluminium, so it is picked by hand).
    pub fn for_pipe(maker: &str) -> Range {
        if maker.to_lowercase().contains("wavin") {
            Range::WavinSteel
        } else {
            Range::HerzBrass
        }
    }
}

/// How a manifold stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Cabinet {
    /// Flush in the wall.
    #[default]
    Flush,
    /// On the wall.
    Wall,
    /// On its brackets, without a cabinet.
    None,
}

/// A cabinet from the maker's range: its code, width, height and depth,
/// mm (a range of heights or depths as printed: the larger figure).
#[derive(Clone, Debug, PartialEq)]
pub struct CabinetSpec {
    pub code: String,
    pub width: f64,
    pub height: f64,
    pub depth: f64,
    /// As the maker prints it ("705–885 × 80–110").
    pub note: String,
}

/// What a manifold is made with.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ManifoldConfig {
    pub range: Range,
    /// Underfloor loops.
    pub loops: u32,
    /// Extra outlets: radiators fed through the floor's insulation layer.
    pub extra: u32,
    pub cabinet: Cabinet,
    /// With the maker's mixing set (a larger HERZ cabinet).
    pub mixing: bool,
}

impl Default for ManifoldConfig {
    fn default() -> Self {
        Self {
            range: Range::HerzBrass,
            loops: 4,
            extra: 0,
            cabinet: Cabinet::Flush,
            mixing: false,
        }
    }
}

impl ManifoldConfig {
    /// The circuits it needs: its loops and extra outlets.
    pub fn needed(&self) -> u32 {
        self.loops + self.extra
    }

    /// The size made: the circuits needed, at least the range's smallest;
    /// none past its largest (a second manifold is needed).
    pub fn circuits(&self) -> Option<u32> {
        let (lo, hi) = self.range.circuits();
        let n = self.needed().max(lo);
        (n <= hi).then_some(n)
    }

    /// Its code, by the circuits made.
    pub fn code(&self) -> Option<String> {
        self.circuits().map(|n| self.range.code(n))
    }

    /// The cabinet that takes it, from the maker's table.
    pub fn cabinet_spec(&self) -> Option<CabinetSpec> {
        let n = self.circuits()?;
        match (self.range, self.cabinet) {
            (_, Cabinet::None) => None,
            // HERZ makes its cabinets to go under the plaster only
            (Range::HerzBrass | Range::HerzPush, Cabinet::Wall) => None,
            (Range::HerzBrass | Range::HerzPush, Cabinet::Flush) => {
                // table 18532xx: column A manifold + shut-off valves,
                // column B with the mixing set as well
                let a = [(5, 10), (8, 15), (10, 20), (14, 25), (16, 30)];
                let b = [(4, 15), (6, 20), (9, 25), (14, 30)];
                let size = if self.mixing { &b[..] } else { &a[..] }
                    .iter()
                    .find(|(m, _)| n <= *m)
                    .map(|(_, s)| *s)?;
                // sheet 1 85XX XX-000: widths C by size; flush 8569 80–110
                // deep, the frame 8572 110–150; 705–885 high on its feet
                let width = match size {
                    10 => 534.0,
                    15 => 684.0,
                    20 => 834.0,
                    25 => 984.0,
                    _ => 1134.0,
                };
                Some(CabinetSpec {
                    code: format!("1 8572 {size:02}"),
                    width,
                    height: 885.0,
                    depth: 110.0,
                    note: "705–885 × 80–110 mm".into(),
                })
            }
            (Range::WavinSteel | Range::WavinComposite, c) => {
                let wall = c == Cabinet::Wall;
                let (k, w) = match n {
                    0..=6 => return None,
                    7..=9 => (4, 680.0),
                    10..=12 => (5, 830.0),
                    _ => (6, 1030.0),
                };
                Some(CabinetSpec {
                    code: format!("XF158{}0{k}W", if wall { 0 } else { 2 }),
                    width: w,
                    height: if wall { 600.0 } else { 605.0 },
                    depth: if wall { 120.0 } else { 160.0 },
                    note: String::new(),
                })
            }
        }
    }

    /// The catalogue name: maker, code, circuits.
    pub fn name(&self) -> String {
        match self.circuits() {
            Some(n) => format!(
                "Rozdeľovač {} {} – {n} okr.",
                self.range.maker(),
                self.range.code(n)
            ),
            None => format!("Rozdeľovač {}", self.range.label()),
        }
    }
}

/// The family the fixtures know it by.
pub const FAMILY: &str = "Rozdeľovač podlahového vykurovania";

/// The manifold made to `c`: its bars with an outlet for every circuit,
/// in its cabinet; its heating supply and return on the left, 1".
pub fn manifold(c: &ManifoldConfig) -> CatalogueObject {
    let r = c.range;
    let n = c.circuits().unwrap_or(r.circuits().1);
    let len = r.length_mm(n) / 1000.0;
    let bars = r.bars_mm() / 1000.0;
    let cab = c.cabinet_spec();
    let mut b = Build::new();
    let brass = metal("Brass", [181, 148, 82], 0.3);
    let steel = metal("Stainless steel", [196, 199, 202], 0.25);
    let composite = super::look(
        "Plastic, black",
        "plastic",
        crate::Hatch::Empty,
        [40, 40, 42],
        0.5,
    );
    let bar_look = match r {
        Range::HerzBrass => brass.clone(),
        Range::WavinSteel => steel.clone(),
        _ => composite.clone(),
    };
    // its width: the cabinet's, else its own length
    let w = cab.as_ref().map_or(len + 0.06, |s| s.width / 1000.0);
    let depth = cab.as_ref().map_or(0.08, |s| s.depth / 1000.0);
    let flush = c.cabinet == Cabinet::Flush && cab.is_some();
    // the room is −y; a flush cabinet goes into the wall (+y) save its frame
    let (y_front, y_back) = if flush {
        (-0.015, depth - 0.015)
    } else {
        (-depth, 0.0)
    };
    let y_bar = y_front + 0.045;
    // bars in the cabinet's middle, the return one low
    let z_mid = cab.as_ref().map_or(0.3, |s| s.height / 2000.0 + 0.1);
    let (z_ret, z_sup) = (z_mid - bars / 2.0, z_mid + bars / 2.0);
    if let Some(s) = &cab {
        let white = super::appliance_white();
        b.add(
            "Cabinet",
            white,
            cuboid(
                v3(-w / 2.0, y_front, 0.0),
                v3(w / 2.0, y_back, s.height / 1000.0),
            ),
            false,
        );
    }
    let x0 = -len / 2.0;
    for (i, z) in [z_ret, z_sup].into_iter().enumerate() {
        b.add(
            if i == 0 { "Return bar" } else { "Supply bar" },
            bar_look.clone(),
            cuboid(
                v3(x0, y_bar - 0.015, z - 0.015),
                v3(x0 + len, y_bar + 0.015, z + 0.015),
            ),
            false,
        );
        // an outlet for every circuit, the return's set off by half a pitch
        let p = r.pitch_mm() / 1000.0;
        let first = x0 + 0.06 + if i == 0 { p / 2.0 } else { 0.0 };
        for k in 0..n {
            let x = first + k as f64 * p;
            if x > x0 + len - 0.02 {
                break;
            }
            b.add(
                "Outlet",
                brass.clone(),
                cylinder(v2(x, y_bar), 0.0105, z - 0.075, z - 0.015),
                false,
            );
        }
    }
    // brackets at its ends
    for x in [x0 + 0.025, x0 + len - 0.025] {
        b.add(
            "Bracket",
            steel.clone(),
            cuboid(
                v3(x - 0.012, y_bar + 0.015, z_ret - 0.06),
                v3(x + 0.012, y_bar + 0.02, z_sup + 0.06),
            ),
            false,
        );
    }
    // the heating supply and return in on the left, 1"
    for (name, z) in [
        ("Connection UKP DN25", z_sup),
        ("Connection UKS DN25", z_ret),
    ] {
        b.add(
            name,
            brass.clone(),
            cylinder(v2(x0 - 0.03, y_bar), 0.0165, z - 0.02, z),
            false,
        );
    }
    // plan: its outline with a diagonal and "RZ"
    b.outline(vec![
        v2(-w / 2.0, y_front),
        v2(w / 2.0, y_front),
        v2(w / 2.0, y_back),
        v2(-w / 2.0, y_back),
    ]);
    b.line(v2(-w / 2.0, y_front), v2(w / 2.0, y_back));
    b.text(v2(0.0, y_front - 0.12), "RZ");
    let key = format!(
        "ufh-manifold-{}-{n}{}",
        match r {
            Range::HerzBrass => "herz-8532",
            Range::HerzPush => "herz-push",
            Range::WavinSteel => "wavin-nerez",
            Range::WavinComposite => "wavin-kompozit",
        },
        match c.cabinet {
            Cabinet::Flush => "",
            Cabinet::Wall => "-wall",
            Cabinet::None => "-bare",
        }
    );
    let mut desc = format!("{}, {n} okruhov", r.label());
    if c.extra > 0 {
        desc += &format!(" (z toho {} vývod. pre radiátory)", c.extra);
    }
    if let Some(s) = &cab {
        desc += &format!(", skriňa {} {:.0} mm", s.code, s.width);
    }
    let mut o = b.finish(
        &key,
        &c.name(),
        &desc,
        ObjectCategory::Hvac,
        ("IfcSpaceHeater", "USERDEFINED"),
        Mount::Wall,
    );
    o.object.offset = 0.0;
    o
}

/// The catalogue's: HERZ brass without a cabinet (one under the
/// plaster stands in the wall, made to order in its window).
pub fn starters() -> Vec<CatalogueObject> {
    vec![manifold(&ManifoldConfig {
        cabinet: Cabinet::None,
        ..Default::default()
    })]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{family_of_type, part_connectors, ConnKind};

    #[test]
    fn sizes_and_codes_follow_the_makers_tables() {
        assert_eq!(Range::HerzBrass.length_mm(3), 221.0);
        assert_eq!(Range::HerzBrass.length_mm(16), 871.0);
        assert_eq!(Range::HerzPush.length_mm(12), 747.0);
        assert_eq!(Range::WavinSteel.length_mm(12), 675.0);
        assert_eq!(Range::WavinComposite.length_mm(5), 395.0);
        assert_eq!(Range::HerzBrass.code(8), "1 8532 08");
        assert_eq!(Range::HerzPush.code(2), "QF232732");
        assert_eq!(Range::WavinComposite.code(10), "XF157010W");
        // loops and radiator outlets together pick the size
        let c = ManifoldConfig {
            loops: 7,
            extra: 2,
            ..Default::default()
        };
        assert_eq!(c.circuits(), Some(9));
        assert_eq!(c.cabinet_spec().unwrap().code, "1 8572 20");
        // too many for one Wavin manifold
        let w = ManifoldConfig {
            range: Range::WavinSteel,
            loops: 13,
            ..Default::default()
        };
        assert_eq!(w.circuits(), None);
        // two loops on HERZ brass: its smallest, 3
        let s = ManifoldConfig {
            loops: 2,
            ..Default::default()
        };
        assert_eq!(s.code().as_deref(), Some("1 8532 03"));
    }

    #[test]
    fn it_is_a_manifold_with_its_heating_connections() {
        let o = manifold(&ManifoldConfig::default());
        o.object.validate().unwrap();
        assert_eq!(family_of_type(&o.object), Some(FAMILY));
        let k: Vec<_> = part_connectors(&o.object).iter().map(|c| c.kind).collect();
        assert_eq!(k, vec![ConnKind::UKP, ConnKind::UKS]);
        assert!(o.name.contains("1 8532 04"), "{}", o.name);
    }
}
