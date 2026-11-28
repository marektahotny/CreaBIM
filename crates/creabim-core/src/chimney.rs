//! Schiedel chimney systems for family houses (George 2026-11-27): one
//! element, a chimney, with its system and type picked from the maker's
//! range: one or two flues, with or without a ventilation (multipurpose)
//! shaft. The chimney stays a part pulled up from a plan outline, so the
//! flashing round it and the gap timber keeps from it work as they did;
//! the outline, the flues and the shafts come from the catalogue.
//!
//! Every size here is the maker's, as printed:
//! - Blocks (ABSOLUT, UNI SMART, UNI ADVANCED PLUS, STABIL): Schiedel
//!   Slovensko, *Typy tvárnic v keramických systémoch Schiedel* 10/2024
//!   (sent by George 2026-11-28): flue, outer size, shaft, kg/bm, for the
//!   block without and with the shaft.
//! - ABSOLUT 25, 30, 40 and the ABSOLUT liner and insulation: *Informačný
//!   dátový list Schiedel ABSOLUT* 10/2024, p. 1 and 3.
//! - KERASTAR: *Informačný dátový list Schiedel KERASTAR* 10/2024: inner
//!   Ø, outer Ø, liner wall, kg/m; mineral wool 60 mm.
//!
//! Where the flues and shafts lie in a block is not dimensioned in these
//! sheets, only drawn: each flue sits in the middle of a square cell as
//! wide as the block at its end (the single-flue block), and the shaft
//! sits in the middle of what is left (between the two cells of a
//! two-flue block); two shafts stand side by side with equal walls.

use glam::DVec2;
use serde::{Deserialize, Serialize};

/// A Schiedel system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum System {
    /// Ceramic liner, mineral wool and a stainless jacket: round.
    Kerastar,
    /// Lightweight concrete block with foam-concrete insulation.
    Absolut,
    UniSmart,
    /// Block, insulating mat and ceramic liner.
    UniAdvancedPlus,
    Stabil,
}

/// The block sheet George sent.
const BLOCKS: &str = "Schiedel SK, Typy tvárnic v keramických systémoch Schiedel 10/2024";

impl System {
    pub const ALL: [System; 5] = [
        System::Kerastar,
        System::Absolut,
        System::UniSmart,
        System::UniAdvancedPlus,
        System::Stabil,
    ];

    /// The maker's name.
    pub fn name(self) -> &'static str {
        match self {
            System::Kerastar => "Schiedel KERASTAR",
            System::Absolut => "Schiedel ABSOLUT",
            System::UniSmart => "Schiedel UNI SMART",
            System::UniAdvancedPlus => "Schiedel UNI ADVANCED PLUS",
            System::Stabil => "Schiedel STABIL",
        }
    }

    /// Where its sizes are printed.
    pub fn source(self) -> &'static str {
        match self {
            System::Kerastar => "Schiedel SK, Informačný dátový list KERASTAR 10/2024",
            System::Absolut => {
                "Schiedel SK, Typy tvárnic v keramických systémoch Schiedel 10/2024; ABSOLUT 25–40, vložka a izolácia: Informačný dátový list ABSOLUT 10/2024"
            }
            _ => BLOCKS,
        }
    }

    /// Its types: single flues smallest first, then two flues; each
    /// without and with its shaft where the maker makes one.
    pub fn types(self) -> &'static [Type] {
        static ALL: std::sync::LazyLock<Vec<Vec<Type>>> = std::sync::LazyLock::new(|| {
            System::ALL
                .iter()
                .map(|s| match s {
                    System::Kerastar => KERASTAR.to_vec(),
                    System::Absolut => {
                        let mut v = expand(ABSOLUT);
                        // the larger single flues of the data sheet, no shaft
                        let at = v.iter().position(|t| t.flue2 > 0).unwrap_or(v.len());
                        v.splice(at..at, ABSOLUT_LARGE.iter().copied());
                        v
                    }
                    System::UniSmart => expand(UNI_SMART),
                    System::UniAdvancedPlus => expand(UNI_ADVANCED),
                    System::Stabil => expand(STABIL),
                })
                .collect()
        });
        let i = System::ALL.iter().position(|s| *s == self).unwrap_or(0);
        &ALL[i]
    }

    /// The type of flues `flue` (and `flue2`, 0 for none), with or without
    /// a shaft, or the nearest the system makes.
    pub fn of(self, flue: u16, flue2: u16, shaft: bool) -> &'static Type {
        let t = self.types();
        let cost = |x: &Type| {
            let mut c = x.flue.abs_diff(flue) as u32 + x.flue2.abs_diff(flue2) as u32;
            if (x.flue2 > 0) != (flue2 > 0) {
                c += 10_000;
            }
            if x.shaft.is_some() != shaft {
                c += 1_000;
            }
            c
        };
        t.iter().min_by_key(|x| cost(x)).unwrap()
    }

    /// Whether it makes blocks with a shaft.
    pub fn has_shafts(self) -> bool {
        self.types().iter().any(|t| t.shaft.is_some())
    }

    /// Whether it makes two-flue blocks.
    pub fn has_two_flues(self) -> bool {
        self.types().iter().any(|t| t.flue2 > 0)
    }
}

/// The outside of a chimney in plan, mm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Outer {
    Round(f64),
    /// Along the block (its long side), then across, mm.
    Block(f64, f64),
}

/// A ventilation shaft in a block, mm: across the block's length, along
/// it, and how many stand side by side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shaft {
    pub across: f64,
    pub along: f64,
    pub count: u8,
}

/// One type of a system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Type {
    /// Flue, inner Ø, mm.
    pub flue: u16,
    /// The second flue of a two-flue block, mm (0 for one flue).
    pub flue2: u16,
    pub outer: Outer,
    pub shaft: Option<Shaft>,
    /// The liner's wall, mm (0 when not printed).
    pub liner: f64,
    /// The insulation's thickness, mm (0 when not printed).
    pub insulation: f64,
    /// Weight per running metre, kg/m.
    pub kg_m: f64,
}

const fn kera(flue: u16, d: f64, liner: f64, kg_m: f64) -> Type {
    Type {
        flue,
        flue2: 0,
        outer: Outer::Round(d),
        shaft: None,
        liner,
        insulation: 60.0,
        kg_m,
    }
}

/// KERASTAR, IDL 10/2024: inner Ø / outer Ø / liner wall / kg/m.
const KERASTAR: &[Type] = &[
    kera(140, 276.0, 6.5, 20.0),
    kera(160, 298.0, 7.0, 22.0),
    kera(180, 318.0, 7.0, 24.0),
    kera(200, 342.0, 8.5, 28.0),
    kera(250, 395.0, 10.0, 30.0),
    kera(300, 451.0, 11.0, 41.0),
    kera(400, 551.0, 12.0, 55.0),
];

/// ABSOLUT 25, 30 and 40 (IDL 10/2024 p. 1 and 3; not in the block
/// sheet): block, liner, insulation, kg/m.
const ABSOLUT_LARGE: &[Type] = &[
    abs_large(250, 480.0, 10.0, 47.5, 130.0),
    abs_large(300, 550.0, 11.0, 45.0, 169.0),
    abs_large(400, 670.0, 12.0, 45.0, 222.0),
];

const fn abs_large(flue: u16, a: f64, liner: f64, insulation: f64, kg_m: f64) -> Type {
    Type {
        flue,
        flue2: 0,
        outer: Outer::Block(a, a),
        shaft: None,
        liner,
        insulation,
        kg_m,
    }
}

/// The ABSOLUT liner and insulation of a flue, mm (IDL 10/2024 p. 1).
fn absolut_layers(flue: u16) -> (f64, f64) {
    match flue {
        120 | 140 => (6.5, 38.0),
        160 | 180 => (7.0, 38.0),
        200 => (8.5, 35.5),
        _ => (0.0, 0.0),
    }
}

/// A row of the block sheet, cm and kg/bm as printed: the flues; the
/// block without a shaft (its two sides, kg/bm); the shaft (across ×
/// along, how many; 0 when there is none) and the block with it.
type Row = (u16, u16, f64, f64, f64, f64, f64, u8, f64, f64, f64);

/// The block's sides as printed ("36/50"): the shorter first.
fn block(x: f64, y: f64) -> Outer {
    Outer::Block(x.max(y) * 10.0, x.min(y) * 10.0)
}

/// Every type of a sheet's rows: without the shaft, then with it.
fn expand(rows: &[Row]) -> Vec<Type> {
    let mut out = Vec::new();
    for with in [false, true] {
        for &(f1, f2, a, b, kg, sa, sb, n, sa_, sb_, kg_s) in rows {
            let (liner, insulation) = if rows == ABSOLUT && f2 == 0 {
                absolut_layers(f1 * 10)
            } else {
                (0.0, 0.0)
            };
            let t = Type {
                flue: f1 * 10,
                flue2: f2 * 10,
                outer: block(a, b),
                shaft: None,
                liner,
                insulation,
                kg_m: kg,
            };
            if !with {
                out.push(t);
            } else if n > 0 {
                out.push(Type {
                    outer: block(sa_, sb_),
                    shaft: Some(Shaft {
                        across: sa.min(sb) * 10.0,
                        along: sa.max(sb) * 10.0,
                        count: n,
                    }),
                    kg_m: kg_s,
                    ..t
                });
            }
        }
    }
    // single flues first, then two, each without the shaft first
    out.sort_by_key(|t| (t.flue2 > 0, t.shaft.is_some(), t.flue, t.flue2));
    out
}

/// ABSOLUT, block sheet 10/2024.
const ABSOLUT: &[Row] = &[
    (12, 0, 36.0, 36.0, 86.9, 10.0, 23.0, 1, 36.0, 50.0, 124.4),
    (14, 0, 36.0, 36.0, 87.1, 10.0, 23.0, 1, 36.0, 50.0, 124.6),
    (16, 0, 36.0, 36.0, 89.3, 10.0, 23.0, 1, 36.0, 50.0, 126.8),
    (18, 0, 36.0, 36.0, 90.0, 10.0, 23.0, 1, 36.0, 50.0, 127.5),
    (20, 0, 38.0, 38.0, 87.1, 12.0, 25.0, 1, 38.0, 54.0, 124.6),
    (12, 12, 36.0, 65.0, 152.8, 13.0, 20.0, 1, 36.0, 83.0, 196.0),
    (12, 16, 36.0, 65.0, 155.2, 13.0, 20.0, 1, 36.0, 83.0, 198.4),
    (12, 18, 36.0, 65.0, 155.9, 13.0, 20.0, 1, 36.0, 83.0, 199.1),
    (12, 20, 38.0, 71.0, 157.5, 14.0, 22.0, 1, 38.0, 88.0, 193.5),
    (14, 14, 36.0, 65.0, 153.2, 13.0, 20.0, 1, 36.0, 83.0, 196.4),
    (14, 16, 36.0, 65.0, 155.4, 13.0, 20.0, 1, 36.0, 83.0, 198.6),
    (14, 18, 36.0, 65.0, 155.9, 13.0, 20.0, 1, 36.0, 83.0, 199.1),
    (14, 20, 38.0, 71.0, 157.7, 14.0, 22.0, 1, 38.0, 88.0, 193.7),
    (16, 16, 36.0, 65.0, 157.6, 13.0, 20.0, 1, 36.0, 83.0, 200.8),
    (16, 20, 38.0, 71.0, 159.9, 14.0, 22.0, 1, 38.0, 88.0, 201.3),
    (18, 18, 36.0, 65.0, 159.0, 13.0, 20.0, 1, 36.0, 83.0, 202.2),
    (18, 20, 38.0, 71.0, 160.6, 14.0, 22.0, 1, 38.0, 88.0, 202.0),
];

/// UNI SMART, block sheet 10/2024. The two-flue 16 18 and 18 18 with a
/// shaft are printed 32/72, narrower than the same blocks without one
/// (36/67): left out until George confirms them.
const UNI_SMART: &[Row] = &[
    (16, 0, 36.0, 36.0, 91.0, 10.0, 26.0, 1, 36.0, 50.0, 112.3),
    (18, 0, 36.0, 36.0, 92.0, 10.0, 26.0, 1, 36.0, 50.0, 114.4),
    (16, 16, 36.0, 67.0, 118.8, 10.0, 22.0, 1, 36.0, 83.0, 143.4),
    (16, 18, 36.0, 67.0, 118.8, 0.0, 0.0, 0, 0.0, 0.0, 0.0),
    (18, 18, 36.0, 67.0, 118.8, 0.0, 0.0, 0, 0.0, 0.0, 0.0),
];

/// UNI ADVANCED PLUS, block sheet 10/2024.
const UNI_ADVANCED: &[Row] = &[
    (14, 0, 32.0, 32.0, 68.7, 10.0, 22.0, 1, 32.0, 46.0, 99.0),
    (16, 0, 32.0, 32.0, 69.9, 10.0, 22.0, 1, 32.0, 46.0, 100.2),
    (18, 0, 36.0, 36.0, 87.7, 10.0, 26.0, 1, 36.0, 50.0, 112.3),
    (20, 0, 36.0, 36.0, 89.8, 10.0, 26.0, 1, 36.0, 50.0, 114.4),
    (14, 14, 32.0, 59.0, 116.7, 10.0, 22.0, 1, 32.0, 72.0, 142.2),
    (14, 16, 32.0, 59.0, 117.9, 10.0, 22.0, 1, 32.0, 72.0, 143.4),
    (14, 18, 36.0, 64.0, 138.7, 14.0, 26.0, 1, 36.0, 81.0, 168.4),
    (14, 20, 36.0, 64.0, 141.4, 14.0, 26.0, 1, 36.0, 81.0, 171.1),
    (16, 16, 32.0, 59.0, 119.1, 10.0, 22.0, 1, 32.0, 72.0, 144.6),
    (16, 18, 36.0, 64.0, 139.9, 14.0, 26.0, 1, 36.0, 81.0, 169.6),
    (16, 20, 36.0, 64.0, 142.6, 14.0, 26.0, 1, 36.0, 81.0, 172.3),
    (18, 18, 36.0, 67.0, 135.2, 13.0, 26.0, 1, 36.0, 83.0, 166.4),
    (20, 20, 36.0, 67.0, 140.6, 13.0, 26.0, 1, 36.0, 83.0, 171.8),
];

/// STABIL, block sheet 10/2024.
const STABIL: &[Row] = &[
    (14, 0, 32.0, 32.0, 76.2, 10.0, 22.0, 1, 32.0, 46.0, 106.5),
    (16, 0, 32.0, 32.0, 77.7, 10.0, 22.0, 1, 32.0, 46.0, 108.0),
    (18, 0, 36.0, 36.0, 96.9, 10.0, 26.0, 1, 36.0, 50.0, 121.5),
    (20, 0, 36.0, 36.0, 98.7, 10.0, 26.0, 1, 36.0, 50.0, 123.3),
    (25, 0, 48.0, 48.0, 179.1, 10.5, 17.0, 2, 48.0, 62.0, 226.8),
    (30, 0, 55.0, 55.0, 216.9, 12.0, 20.0, 2, 55.0, 71.0, 279.0),
    (35, 0, 60.0, 60.0, 375.0, 14.0, 22.5, 2, 60.0, 78.0, 423.0),
    (40, 0, 67.0, 67.0, 456.0, 15.5, 26.0, 2, 67.0, 86.0, 543.0),
    (14, 14, 32.0, 59.0, 135.3, 10.0, 22.0, 1, 32.0, 72.0, 160.8),
    (14, 16, 32.0, 59.0, 136.8, 10.0, 22.0, 1, 32.0, 72.0, 162.3),
    (14, 18, 36.0, 64.0, 159.6, 14.0, 26.0, 1, 36.0, 81.0, 189.3),
    (14, 20, 36.0, 64.0, 161.4, 14.0, 26.0, 1, 36.0, 81.0, 191.1),
    (16, 16, 32.0, 59.0, 138.3, 10.0, 22.0, 1, 32.0, 72.0, 163.8),
    (16, 18, 36.0, 64.0, 161.1, 14.0, 26.0, 1, 36.0, 81.0, 190.8),
    (16, 20, 36.0, 64.0, 162.9, 14.0, 26.0, 1, 36.0, 81.0, 192.6),
    (18, 18, 36.0, 67.0, 158.4, 13.0, 26.0, 1, 36.0, 83.0, 189.6),
    (20, 20, 36.0, 67.0, 162.0, 13.0, 26.0, 1, 36.0, 83.0, 193.2),
];

impl Type {
    /// "Ø200", "14 16", "14 16 + šachta 13/20".
    pub fn short(&self) -> String {
        let mm = |v: f64| {
            let s = format!("{}", v / 10.0);
            s.replace('.', ",")
        };
        let mut s = if self.flue2 > 0 {
            format!("{} {}", self.flue / 10, self.flue2 / 10)
        } else if matches!(self.outer, Outer::Round(_)) {
            format!("Ø{}", self.flue)
        } else {
            format!("{}", self.flue / 10)
        };
        if let Some(sh) = self.shaft {
            let n = if sh.count > 1 {
                format!("{} × ", sh.count)
            } else {
                String::new()
            };
            s += &format!(
                " + {} {n}{}/{}",
                crate::words::tr("shaft"),
                mm(sh.across),
                mm(sh.along)
            );
        }
        s
    }
}

/// A chimney's system and type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chimney {
    pub system: System,
    /// Flue, inner Ø, mm.
    pub flue: u16,
    /// The second flue of a two-flue block, mm; 0 for one flue.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub flue2: u16,
    /// With the block's ventilation shaft.
    #[serde(default, skip_serializing_if = "is_false")]
    pub shaft: bool,
}

fn is_zero(v: &u16) -> bool {
    *v == 0
}
fn is_false(v: &bool) -> bool {
    !*v
}

impl Default for Chimney {
    fn default() -> Self {
        Self {
            system: System::Kerastar,
            flue: 200,
            flue2: 0,
            shaft: false,
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
        self.system.of(self.flue, self.flue2, self.shaft)
    }

    /// The same chimney set to type `t`.
    pub fn with(self, t: &Type) -> Self {
        Self {
            flue: t.flue,
            flue2: t.flue2,
            shaft: t.shaft.is_some(),
            ..self
        }
    }

    /// The nearest type of system `s`.
    pub fn in_system(self, s: System) -> Self {
        let t = s.of(self.flue, self.flue2, self.shaft);
        Self { system: s, ..self }.with(t)
    }

    /// "Schiedel KERASTAR Ø200", "Schiedel ABSOLUT 14 16 + šachta 13/20".
    pub fn name(&self) -> String {
        format!("{} {}", self.system.name(), self.kind().short())
    }

    /// Outer size in words: "Ø342 mm", "380 × 380 mm".
    pub fn size(&self) -> String {
        match self.kind().outer {
            Outer::Round(d) => format!("Ø{d:.0} mm"),
            Outer::Block(a, b) => format!("{b:.0} × {a:.0} mm"),
        }
    }

    /// Its outline and holes (flues, shafts) in plan, centred on `c`, the
    /// block's length along `dir`, m.
    pub fn shape(&self, c: DVec2, dir: DVec2) -> (Vec<DVec2>, Vec<Vec<DVec2>>) {
        let t = self.kind();
        let u = dir.try_normalize().unwrap_or(DVec2::X);
        let v = u.perp();
        let at = |x: f64, y: f64| c + u * (x / 1000.0) + v * (y / 1000.0);
        let rect = |x: f64, y: f64, w: f64, h: f64| {
            vec![
                at(x - w / 2.0, y - h / 2.0),
                at(x + w / 2.0, y - h / 2.0),
                at(x + w / 2.0, y + h / 2.0),
                at(x - w / 2.0, y + h / 2.0),
            ]
        };
        let flue = |x: f64, d: u16| circle(at(x, 0.0), d as f64 / 2000.0, SIDES);
        match t.outer {
            Outer::Round(d) => (circle(c, d / 2000.0, SIDES), vec![flue(0.0, t.flue)]),
            Outer::Block(a, b) => {
                let outline = rect(0.0, 0.0, a, b);
                let mut holes = vec![];
                // flue cells as wide as the block, at its ends
                let end = (a - b) / 2.0;
                let (shaft_x, flues) = if t.flue2 > 0 {
                    (0.0, vec![(-end, t.flue), (end, t.flue2)])
                } else if t.shaft.is_some() {
                    // the cell at one end, the shaft in the strip past it
                    (b / 2.0, vec![(-end, t.flue)])
                } else {
                    (0.0, vec![(0.0, t.flue)])
                };
                for (x, d) in flues {
                    holes.push(flue(x, d));
                }
                if let Some(s) = t.shaft {
                    let n = s.count.max(1) as f64;
                    let gap = (b - n * s.along) / (n + 1.0);
                    for i in 0..s.count.max(1) {
                        let y = -b / 2.0 + gap * (i as f64 + 1.0) + s.along * (i as f64 + 0.5);
                        holes.push(rect(shaft_x, y, s.across, s.along));
                    }
                }
                (outline, holes)
            }
        }
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

    /// The take-off's line: name, size and weight.
    pub fn label(&self) -> String {
        format!(
            "{} – {} ({} kg/m)",
            self.name(),
            self.size(),
            fmt_dec(self.kind().kg_m)
        )
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
            ..Default::default()
        };
        assert_eq!(c.name(), "Schiedel ABSOLUT 20");
        let dir = DVec2::new(1.0, 1.0).normalize();
        let (o, _) = c.shape(DVec2::new(5.0, 2.0), dir);
        assert!((o[0].distance(o[1]) - 0.38).abs() < 1e-9);
        let (p, d) = Chimney::placement(&o);
        assert!(p.distance(DVec2::new(5.0, 2.0)) < 1e-9);
        assert!(d.distance(dir) < 1e-9);
        assert_eq!(c.label(), "Schiedel ABSOLUT 20 – 380 × 380 mm (87,1 kg/m)");
        assert_eq!(c.kind().liner, 8.5);
    }

    #[test]
    fn a_shaft_block_has_its_flue_cell_and_shaft() {
        let c = Chimney {
            system: System::Absolut,
            flue: 180,
            flue2: 0,
            shaft: true,
        };
        assert_eq!(c.size(), "360 × 500 mm");
        assert_eq!(
            c.label(),
            "Schiedel ABSOLUT 18 + shaft 10/23 – 360 × 500 mm (127,5 kg/m)"
        );
        let (o, h) = c.shape(DVec2::ZERO, DVec2::X);
        assert!((o[1].x - 0.25).abs() < 1e-9);
        assert_eq!(h.len(), 2);
        // the flue in the 360 cell at the block's end
        let fc = h[0].iter().copied().sum::<DVec2>() / h[0].len() as f64;
        assert!((fc.x + 0.07).abs() < 1e-9);
        // the shaft 100 × 230 in the 140 strip, 20 mm each side
        let xs: Vec<f64> = h[1].iter().map(|p| p.x).collect();
        assert!((xs[0] - 0.13).abs() < 1e-9 && (xs[1] - 0.23).abs() < 1e-9);
        assert!((h[1][2].y - 0.115).abs() < 1e-9);
    }

    #[test]
    fn a_two_flue_block_with_its_shaft_between() {
        let c = Chimney {
            system: System::UniAdvancedPlus,
            flue: 140,
            flue2: 180,
            shaft: true,
        };
        assert_eq!(c.size(), "360 × 810 mm");
        assert_eq!(c.kind().kg_m, 168.4);
        let (_, h) = c.shape(DVec2::ZERO, DVec2::X);
        assert_eq!(h.len(), 3);
        let centre = |p: &Vec<DVec2>| p.iter().copied().sum::<DVec2>() / p.len() as f64;
        assert!((centre(&h[0]).x + 0.225).abs() < 1e-9);
        assert!((centre(&h[1]).x - 0.225).abs() < 1e-9);
        assert!(centre(&h[2]).length() < 1e-9);
        // nothing overlaps: the shaft clear of both flues
        let r2 = 0.09;
        assert!(0.225 - r2 > 0.07);
    }

    #[test]
    fn stabil_large_has_two_shafts_side_by_side() {
        let c = Chimney {
            system: System::Stabil,
            flue: 250,
            flue2: 0,
            shaft: true,
        };
        assert_eq!(c.size(), "480 × 620 mm");
        let (_, h) = c.shape(DVec2::ZERO, DVec2::X);
        assert_eq!(h.len(), 3);
        let gap = (480.0 - 2.0 * 170.0) / 3.0 / 1000.0;
        assert!((h[1][0].y - (-0.24 + gap)).abs() < 1e-9);
        assert!((h[2][3].y - (0.24 - gap)).abs() < 1e-9);
    }

    #[test]
    fn every_type_fits_its_block() {
        for s in System::ALL {
            for t in s.types() {
                let c = Chimney {
                    system: s,
                    ..Default::default()
                }
                .with(t);
                assert_eq!(c.kind(), t, "{s:?} {t:?} is found again");
                let (o, holes) = c.shape(DVec2::ZERO, DVec2::X);
                for h in &holes {
                    assert!(crate::polygon::inside(h, &o), "{s:?} {t:?}");
                    for g in &holes {
                        if !std::ptr::eq(h, g) {
                            assert!(!crate::polygon::overlap(h, g), "{s:?} {t:?}");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_missing_type_takes_the_nearest() {
        assert_eq!(System::UniAdvancedPlus.of(250, 0, false).flue, 200);
        // UNI SMART 18 18 has no shaft block: the nearest two-flue with one
        let t = System::UniSmart.of(180, 180, true);
        assert_eq!((t.flue, t.flue2), (160, 160));
    }
}
