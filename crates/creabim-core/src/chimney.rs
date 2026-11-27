//! Schiedel chimney systems for family houses (George 2026-11-27): one
//! element, a chimney, with its system and flue picked from the maker's
//! range. The chimney stays a part pulled up from a plan outline, so the
//! flashing round it and the gap timber keeps from it work as they did;
//! the outline and the flue in it come from the catalogue.
//!
//! Every size here is the maker's, as printed:
//! - KERASTAR: Schiedel Slovensko, *Informačný dátový list Schiedel
//!   KERASTAR* 10/2024 (and the Czech *Technický list KERASTAR* 2019-04,
//!   the same values): inner Ø, outer Ø, liner wall, kg/m; mineral wool
//!   60 mm; jacket stainless 1.4301.
//! - ABSOLUT: Schiedel Slovensko, *Informačný dátový list Schiedel
//!   ABSOLUT* 10/2024, p. 1 and 3: block size, liner, insulation, kg/m.
//! - UNI ADVANCED PLUS: Schiedel Slovensko, *Sortimentná ponuka*
//!   06/2025, p. 6–7: block size, kg per block, module 33 cm.
//!
//! Blocks with a ventilation shaft and two-flue blocks are not here yet:
//! where the flues and the shaft lie in the block is on the maker's
//! drawings, which are still to be had.

use glam::DVec2;
use serde::{Deserialize, Serialize};

/// A Schiedel system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum System {
    /// Ceramic liner, mineral wool and a stainless jacket: round.
    Kerastar,
    /// Lightweight concrete block with foam-concrete insulation.
    Absolut,
    /// Block, insulating mat and ceramic liner.
    UniAdvancedPlus,
}

impl System {
    pub const ALL: [System; 3] = [System::Kerastar, System::Absolut, System::UniAdvancedPlus];

    /// The maker's name.
    pub fn name(self) -> &'static str {
        match self {
            System::Kerastar => "Schiedel KERASTAR",
            System::Absolut => "Schiedel ABSOLUT",
            System::UniAdvancedPlus => "Schiedel UNI ADVANCED PLUS",
        }
    }

    /// Where its sizes are printed.
    pub fn source(self) -> &'static str {
        match self {
            System::Kerastar => "Schiedel SK, Informačný dátový list KERASTAR 10/2024",
            System::Absolut => "Schiedel SK, Informačný dátový list ABSOLUT 10/2024, s. 1 a 3",
            System::UniAdvancedPlus => "Schiedel SK, Sortimentná ponuka 06/2025, s. 6–7",
        }
    }

    /// Its types, smallest flue first.
    pub fn types(self) -> &'static [Type] {
        match self {
            System::Kerastar => KERASTAR,
            System::Absolut => ABSOLUT,
            System::UniAdvancedPlus => UNI,
        }
    }

    /// The type of flue `flue` mm, or the nearest.
    pub fn of(self, flue: u16) -> &'static Type {
        let t = self.types();
        t.iter()
            .find(|x| x.flue == flue)
            .unwrap_or_else(|| t.iter().min_by_key(|x| x.flue.abs_diff(flue)).unwrap())
    }
}

/// The outside of a chimney in plan, mm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outer {
    Round(f64),
    Block(f64, f64),
}

/// One type of a system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Type {
    /// The maker's type name.
    pub name: &'static str,
    /// Flue, inner Ø, mm.
    pub flue: u16,
    pub outer: Outer,
    /// The liner's wall, mm (0 when not printed).
    pub liner: f64,
    /// The insulation's thickness, mm (0 when not printed).
    pub insulation: f64,
    /// Weight per running metre, kg/m (0 when the maker gives it per
    /// block).
    pub kg_m: f64,
    /// Weight of one block, kg (0 when the maker gives it per metre).
    pub kg_block: f64,
}

const fn kera(name: &'static str, flue: u16, d: f64, liner: f64, kg_m: f64) -> Type {
    Type {
        name,
        flue,
        outer: Outer::Round(d),
        liner,
        insulation: 60.0,
        kg_m,
        kg_block: 0.0,
    }
}

/// KERASTAR, IDL 10/2024: inner Ø / outer Ø / liner wall / kg/m.
const KERASTAR: &[Type] = &[
    kera("KERASTAR Ø140", 140, 276.0, 6.5, 20.0),
    kera("KERASTAR Ø160", 160, 298.0, 7.0, 22.0),
    kera("KERASTAR Ø180", 180, 318.0, 7.0, 24.0),
    kera("KERASTAR Ø200", 200, 342.0, 8.5, 28.0),
    kera("KERASTAR Ø250", 250, 395.0, 10.0, 30.0),
    kera("KERASTAR Ø300", 300, 451.0, 11.0, 41.0),
    kera("KERASTAR Ø400", 400, 551.0, 12.0, 55.0),
];

const fn abs(name: &'static str, flue: u16, a: f64, liner: f64, ins: f64, kg_m: f64) -> Type {
    Type {
        name,
        flue,
        outer: Outer::Block(a, a),
        liner,
        insulation: ins,
        kg_m,
        kg_block: 0.0,
    }
}

/// ABSOLUT single flue, IDL 10/2024 p. 1 and 3.
const ABSOLUT: &[Type] = &[
    abs("ABS 12", 120, 360.0, 6.5, 38.0, 71.0),
    abs("ABS 14", 140, 360.0, 6.5, 38.0, 71.0),
    abs("ABS 16", 160, 360.0, 7.0, 38.0, 73.0),
    abs("ABS 18", 180, 360.0, 7.0, 38.0, 74.0),
    abs("ABS 20", 200, 380.0, 8.5, 35.5, 80.0),
    abs("ABS 25", 250, 480.0, 10.0, 47.5, 130.0),
    abs("ABS 30", 300, 550.0, 11.0, 45.0, 169.0),
    abs("ABS 40", 400, 670.0, 12.0, 45.0, 222.0),
];

const fn uni(name: &'static str, flue: u16, a: f64, kg: f64) -> Type {
    Type {
        name,
        flue,
        outer: Outer::Block(a, a),
        liner: 0.0,
        insulation: 0.0,
        kg_m: 0.0,
        kg_block: kg,
    }
}

/// UNI ADVANCED PLUS single flue, sortiment 06/2025 p. 6–7 (32/32 and
/// 36/36 cm).
const UNI: &[Type] = &[
    uni("UNI ADVANCED PLUS 14", 140, 320.0, 20.1),
    uni("UNI ADVANCED PLUS 16", 160, 320.0, 20.1),
    uni("UNI ADVANCED PLUS 18", 180, 360.0, 23.0),
    uni("UNI ADVANCED PLUS 20", 200, 360.0, 23.0),
];

/// The module of UNI ADVANCED PLUS blocks, m ("Skladobná výška všetkých
/// tvárnic 33 cm", sortiment 06/2025 p. 7).
pub const UNI_MODULE: f64 = 0.33;

/// A chimney's system and flue.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chimney {
    pub system: System,
    /// Flue, inner Ø, mm.
    pub flue: u16,
}

impl Default for Chimney {
    fn default() -> Self {
        Self {
            system: System::Kerastar,
            flue: 200,
        }
    }
}

/// Points on a circle, counter-clockwise.
fn circle(c: DVec2, r: f64, n: usize) -> Vec<DVec2> {
    (0..n)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            c + DVec2::new(a.cos(), a.sin()) * r
        })
        .collect()
}

/// Sides of a round chimney or flue drawn as a polygon.
const SIDES: usize = 48;

impl Chimney {
    pub fn kind(&self) -> &'static Type {
        self.system.of(self.flue)
    }

    /// "Schiedel KERASTAR Ø200", "Schiedel ABSOLUT ABS 20".
    pub fn name(&self) -> String {
        match self.system {
            System::Kerastar => format!("Schiedel {}", self.kind().name),
            _ => format!(
                "{} {}",
                self.system.name(),
                self.kind().name.rsplit(' ').next().unwrap_or("")
            ),
        }
    }

    /// Outer size in words: "Ø342 mm", "380 × 380 mm".
    pub fn size(&self) -> String {
        match self.kind().outer {
            Outer::Round(d) => format!("Ø{d:.0} mm"),
            Outer::Block(a, b) => format!("{a:.0} × {b:.0} mm"),
        }
    }

    /// Its outline and flue in plan, centred on `c`, the block's sides
    /// along `dir`, m.
    pub fn shape(&self, c: DVec2, dir: DVec2) -> (Vec<DVec2>, Vec<Vec<DVec2>>) {
        let t = self.kind();
        let flue = circle(c, t.flue as f64 / 2000.0, SIDES);
        let outline = match t.outer {
            Outer::Round(d) => circle(c, d / 2000.0, SIDES),
            Outer::Block(a, b) => {
                let u = dir.try_normalize().unwrap_or(DVec2::X);
                let (x, y) = (u * a / 2000.0, u.perp() * b / 2000.0);
                vec![c - x - y, c + x - y, c + x + y, c - x + y]
            }
        };
        (outline, vec![flue])
    }

    /// Where an outline made by [`Self::shape`] sits: its centre and the
    /// direction of its first side.
    pub fn placement(outline: &[DVec2]) -> (DVec2, DVec2) {
        let n = outline.len().max(1) as f64;
        let c = outline.iter().copied().sum::<DVec2>() / n;
        let dir = if outline.len() == 4 {
            (outline[1] - outline[0])
                .try_normalize()
                .unwrap_or(DVec2::X)
        } else {
            DVec2::X
        };
        (c, dir)
    }

    /// The take-off's line for `length` m of it: name, size and weight.
    pub fn label(&self, length: f64) -> String {
        let t = self.kind();
        let weight = if t.kg_m > 0.0 {
            format!("{:.0} kg/m", t.kg_m)
        } else {
            let blocks = (length / UNI_MODULE - 1e-6).ceil().max(0.0);
            format!(
                "{blocks:.0} {} × {} kg",
                crate::words::tr("chimney blocks"),
                fmt_dec(t.kg_block)
            )
        };
        format!("{} – {} ({weight})", self.name(), self.size())
    }
}

fn fmt_dec(v: f64) -> String {
    let s = format!("{v:.1}");
    s.trim_end_matches(".0").replace('.', ",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kerastar_sizes_are_the_makers() {
        let c = Chimney::default();
        assert_eq!(c.kind().outer, Outer::Round(342.0));
        assert_eq!(c.name(), "Schiedel KERASTAR Ø200");
        assert_eq!(c.size(), "Ø342 mm");
        let (o, h) = c.shape(DVec2::ZERO, DVec2::X);
        assert_eq!(o.len(), SIDES);
        assert!((o[0].length() - 0.171).abs() < 1e-9);
        assert!((h[0][0].length() - 0.1).abs() < 1e-9);
    }

    #[test]
    fn absolut_block_keeps_its_place_and_turn() {
        let c = Chimney {
            system: System::Absolut,
            flue: 200,
        };
        assert_eq!(c.name(), "Schiedel ABSOLUT 20");
        let dir = DVec2::new(1.0, 1.0).normalize();
        let (o, _) = c.shape(DVec2::new(5.0, 2.0), dir);
        assert!((o[0].distance(o[1]) - 0.38).abs() < 1e-9);
        let (p, d) = Chimney::placement(&o);
        assert!(p.distance(DVec2::new(5.0, 2.0)) < 1e-9);
        assert!(d.distance(dir) < 1e-9);
        assert_eq!(c.label(6.0), "Schiedel ABSOLUT 20 – 380 × 380 mm (80 kg/m)");
    }

    #[test]
    fn uni_counts_its_33_cm_blocks() {
        let c = Chimney {
            system: System::UniAdvancedPlus,
            flue: 160,
        };
        assert_eq!(
            c.label(3.3),
            "Schiedel UNI ADVANCED PLUS 16 – 320 × 320 mm (10 chimney blocks × 20,1 kg)"
        );
        // a flue the system lacks takes the nearest
        assert_eq!(System::UniAdvancedPlus.of(250).flue, 200);
    }
}
