//! Masonry as the makers of its systems design it: system lintels placed
//! by hand over doors and windows, and how long a masonry wall may run
//! between the walls and columns that brace it.
//!
//! **Lintels.** Nothing is placed by itself: the designer puts lintels over
//! the openings they pick, in a plan set for it (a view's `masonry`), and
//! sections show them as they cut them. A lintel is a [`Beam`] carrying a
//! [`Lintel`]: the product and the opening it spans. It follows its
//! opening (moved, widened, its wall moved) through [`settle`], which the
//! [`crate::Document`] runs after every edit, and it goes with the opening.
//!
//! The products and their data:
//! - PORFIX load-bearing lintels (NP), 100 and 125 wide, 250 high: lengths,
//!   bearing and clear opening from the *Odborná príručka PORFIX* (2023),
//!   table *Nosné preklady*, and the pieces a wall takes from the table
//!   *Skladanie prekladov v nosnom preklade* (at least two load-bearing
//!   pieces on the inner side);
//! - PORFIX non-load-bearing lintels (P) for partitions, 75 to 150 wide,
//!   table *Nenosné preklady*;
//! - PORFIX U-profiles (250, 300, 375 wide) as permanent shuttering of an
//!   RC lintel up to 3 m clear, reinforced as the structural engineer says;
//! - Porotherm KP 7 (70 × 238), KP Vario (125 × 238, 1,00 to 1,75 m) and
//!   the flat KP 11,5 and KP 14,5 (115 / 145 × 71, made up with masonry
//!   over them): Wienerberger's technical sheets (bearing 125 mm up to
//!   1,75 m long, 200 mm for 2,00 and 2,25 m, 250 mm from 2,50 m; the
//!   flat ones 120 mm).
//! - Porotherm KP XL 30 and 38 (300 / 380 wide, 3,75 to 6,50 m): KP Vario
//!   pieces with added reinforcement cast with the ring beam over them,
//!   bearing 250 mm, from Wienerberger's technical sheet *Porotherm KP XL*,
//!   with MRd, VRd and gRd for a ring beam 210 to 250 mm high.
//!
//! **Wall lengths.** A masonry wall is braced by the walls across it and by
//! RC columns in it. Its stretches between them are checked against:
//! - load-bearing walls: [`MasonryRules::bearing_max`], 6 m by default
//!   (PORFIX: stiffening walls *4 to 6 m apart in smaller buildings, 6 to
//!   8 m in larger ones*); with [`MasonryRules::seismic`], at most 5 m
//!   between confining elements (STN EN 1998-1, 9.5.3: confined masonry);
//! - partitions: PORFIX's table *Maximálna výška samonosných stien* by
//!   thickness and whether the top is propped against the floor above
//!   (length and height).
//!
//! A wall across braces only when it is thick and long enough (STN EN
//! 1996-1-1, 5.5.1.3: at least 0,3 times the braced wall's thickness and
//! 85 mm, and at least a fifth of its height long).

use crate::members::{storey_height, Profile};
use crate::{Beam, Column, Element, ElementId, ElementKind, Model, Op, Opening, Transaction, Wall};
use glam::DVec2;
use serde::{Deserialize, Serialize};

/// A system lintel's product.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LintelProduct {
    /// Its key, kept in the model: `porfix-np-1500-125`.
    pub id: &'static str,
    /// The family it is picked from in the tool.
    pub family: Family,
    /// Its name as the maker writes it, in the plan's label: `NP 1500/125`.
    pub name: &'static str,
    pub length: f64,
    pub width: f64,
    pub height: f64,
    /// The least bearing at each end.
    pub bearing: f64,
    /// The widest clear opening it spans.
    pub max_span: f64,
    /// Kilograms a piece.
    pub mass: f64,
}

/// A family of lintels: one maker's kind, picked by name in the tool.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    /// PORFIX load-bearing lintels (NP).
    PorfixNp,
    /// PORFIX non-load-bearing lintels for partitions (P).
    PorfixP,
    /// PORFIX U-profiles: an RC lintel cast in them.
    PorfixU,
    /// Porotherm KP 7: 70 × 238, in load-bearing walls.
    Kp7,
    /// Porotherm KP Vario 100 to 175: 125 × 238.
    KpVario,
    /// Porotherm KP 11,5 and KP 14,5: flat, made up with masonry on top.
    Kp115,
    Kp145,
    /// Porotherm KP XL: KP Vario pieces with added reinforcement, cast
    /// together with the ring beam, for 3 to 6 m openings (300 and 380
    /// wide).
    KpXl,
}

impl Family {
    pub const ALL: [Family; 8] = [
        Family::PorfixNp,
        Family::PorfixP,
        Family::PorfixU,
        Family::Kp7,
        Family::KpVario,
        Family::Kp115,
        Family::Kp145,
        Family::KpXl,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Family::PorfixNp => "PORFIX load-bearing lintel (NP)",
            Family::PorfixP => "PORFIX partition lintel (P)",
            Family::PorfixU => "PORFIX U-profile (RC lintel)",
            Family::Kp7 => "Porotherm KP 7",
            Family::KpVario => "Porotherm KP Vario",
            Family::Kp115 => "Porotherm KP 11,5 (flat)",
            Family::Kp145 => "Porotherm KP 14,5 (flat)",
            Family::KpXl => "Porotherm KP XL (3 to 6 m)",
        }
    }

    pub fn maker(self) -> &'static str {
        match self {
            Family::PorfixNp | Family::PorfixP | Family::PorfixU => "PORFIX",
            _ => "Wienerberger",
        }
    }

    /// For load-bearing walls (else for partitions).
    pub fn bearing_wall(self) -> bool {
        !matches!(self, Family::PorfixP)
    }

    pub fn id(self) -> &'static str {
        match self {
            Family::PorfixNp => "porfix-np",
            Family::PorfixP => "porfix-p",
            Family::PorfixU => "porfix-u",
            Family::Kp7 => "kp7",
            Family::KpVario => "kp-vario",
            Family::Kp115 => "kp115",
            Family::Kp145 => "kp145",
            Family::KpXl => "kp-xl",
        }
    }

    pub fn from_id(s: &str) -> Option<Family> {
        Family::ALL.into_iter().find(|f| f.id() == s)
    }

    /// The material the pieces are drawn in: its name, colour and hatch.
    pub fn material(self) -> (&'static str, [f32; 3], crate::Hatch) {
        match self {
            Family::PorfixNp | Family::PorfixP => (
                "Pórobetón – preklad",
                [0.87, 0.85, 0.80],
                crate::Hatch::Diagonal,
            ),
            Family::PorfixU => ("Železobetón", [0.60, 0.64, 0.68], crate::Hatch::Reinforced),
            _ => (
                "Keramobetón – preklad",
                [0.78, 0.45, 0.32],
                crate::Hatch::Brick,
            ),
        }
    }
}

macro_rules! p {
    ($id:literal, $f:ident, $name:literal, $l:literal, $w:literal, $h:literal, $b:literal, $s:literal, $m:literal) => {
        LintelProduct {
            id: $id,
            family: Family::$f,
            name: $name,
            length: $l,
            width: $w,
            height: $h,
            bearing: $b,
            max_span: $s,
            mass: $m,
        }
    };
}

/// Every product, by family and length.
pub const PRODUCTS: &[LintelProduct] = &[
    // PORFIX nosné preklady (Odborná príručka, s. 9)
    p!(
        "porfix-np-1200-100",
        PorfixNp,
        "NP 1200/100",
        1.2,
        0.1,
        0.25,
        0.3,
        0.6,
        55.7
    ),
    p!(
        "porfix-np-1200-125",
        PorfixNp,
        "NP 1200/125",
        1.2,
        0.125,
        0.25,
        0.3,
        0.6,
        71.2
    ),
    p!(
        "porfix-np-1500-100",
        PorfixNp,
        "NP 1500/100",
        1.5,
        0.1,
        0.25,
        0.3,
        0.9,
        69.2
    ),
    p!(
        "porfix-np-1500-125",
        PorfixNp,
        "NP 1500/125",
        1.5,
        0.125,
        0.25,
        0.3,
        0.9,
        89.0
    ),
    p!(
        "porfix-np-1800-100",
        PorfixNp,
        "NP 1800/100",
        1.8,
        0.1,
        0.25,
        0.3,
        1.2,
        83.0
    ),
    p!(
        "porfix-np-1800-125",
        PorfixNp,
        "NP 1800/125",
        1.8,
        0.125,
        0.25,
        0.3,
        1.2,
        106.8
    ),
    p!(
        "porfix-np-2100-100",
        PorfixNp,
        "NP 2100/100",
        2.1,
        0.1,
        0.25,
        0.3,
        1.5,
        96.8
    ),
    p!(
        "porfix-np-2100-125",
        PorfixNp,
        "NP 2100/125",
        2.1,
        0.125,
        0.25,
        0.3,
        1.5,
        125.0
    ),
    p!(
        "porfix-np-2400-100",
        PorfixNp,
        "NP 2400/100",
        2.4,
        0.1,
        0.25,
        0.3,
        1.8,
        110.6
    ),
    p!(
        "porfix-np-2400-125",
        PorfixNp,
        "NP 2400/125",
        2.4,
        0.125,
        0.25,
        0.3,
        1.8,
        142.3
    ),
    p!(
        "porfix-np-2700-100",
        PorfixNp,
        "NP 2700/100",
        2.7,
        0.1,
        0.25,
        0.3,
        2.1,
        124.5
    ),
    p!(
        "porfix-np-2700-125",
        PorfixNp,
        "NP 2700/125",
        2.7,
        0.125,
        0.25,
        0.3,
        2.1,
        160.3
    ),
    // PORFIX nenosné preklady (s. 12)
    p!(
        "porfix-p-1000-75",
        PorfixP,
        "P 1000/75",
        1.0,
        0.075,
        0.25,
        0.1,
        0.8,
        13.0
    ),
    p!(
        "porfix-p-1000-100",
        PorfixP,
        "P 1000/100",
        1.0,
        0.1,
        0.25,
        0.15,
        0.7,
        17.1
    ),
    p!(
        "porfix-p-1000-125",
        PorfixP,
        "P 1000/125",
        1.0,
        0.125,
        0.25,
        0.15,
        0.7,
        21.1
    ),
    p!(
        "porfix-p-1000-150",
        PorfixP,
        "P 1000/150",
        1.0,
        0.15,
        0.25,
        0.15,
        0.7,
        25.2
    ),
    p!(
        "porfix-p-1250-100",
        PorfixP,
        "P 1250/100",
        1.25,
        0.1,
        0.25,
        0.15,
        0.95,
        21.1
    ),
    p!(
        "porfix-p-1250-125",
        PorfixP,
        "P 1250/125",
        1.25,
        0.125,
        0.25,
        0.15,
        0.95,
        26.2
    ),
    p!(
        "porfix-p-1250-150",
        PorfixP,
        "P 1250/150",
        1.25,
        0.15,
        0.25,
        0.15,
        0.95,
        31.3
    ),
    p!(
        "porfix-p-1500-100",
        PorfixP,
        "P 1500/100",
        1.5,
        0.1,
        0.25,
        0.15,
        1.2,
        25.2
    ),
    p!(
        "porfix-p-1500-125",
        PorfixP,
        "P 1500/125",
        1.5,
        0.125,
        0.25,
        0.15,
        1.2,
        31.3
    ),
    p!(
        "porfix-p-1500-150",
        PorfixP,
        "P 1500/150",
        1.5,
        0.15,
        0.25,
        0.15,
        1.2,
        37.4
    ),
    p!(
        "porfix-p-2000-100",
        PorfixP,
        "P 2000/100",
        2.0,
        0.1,
        0.25,
        0.1,
        1.8,
        42.8
    ),
    p!(
        "porfix-p-2000-125",
        PorfixP,
        "P 2000/125",
        2.0,
        0.125,
        0.25,
        0.1,
        1.8,
        45.6
    ),
    p!(
        "porfix-p-2000-150",
        PorfixP,
        "P 2000/150",
        2.0,
        0.15,
        0.25,
        0.15,
        1.7,
        53.0
    ),
    p!(
        "porfix-p-2500-100",
        PorfixP,
        "P 2500/100",
        2.5,
        0.1,
        0.25,
        0.1,
        2.3,
        53.5
    ),
    p!(
        "porfix-p-2500-125",
        PorfixP,
        "P 2500/125",
        2.5,
        0.125,
        0.25,
        0.1,
        2.3,
        57.0
    ),
    p!(
        "porfix-p-2500-150",
        PorfixP,
        "P 2500/150",
        2.5,
        0.15,
        0.25,
        0.15,
        2.2,
        66.3
    ),
    // PORFIX U-profily: a cast lintel up to 3 m clear, in 500 long blocks
    p!(
        "porfix-u-1500-250",
        PorfixU,
        "U 250 – 1500",
        1.5,
        0.25,
        0.25,
        0.25,
        1.0,
        0.0
    ),
    p!(
        "porfix-u-2000-250",
        PorfixU,
        "U 250 – 2000",
        2.0,
        0.25,
        0.25,
        0.25,
        1.5,
        0.0
    ),
    p!(
        "porfix-u-2500-250",
        PorfixU,
        "U 250 – 2500",
        2.5,
        0.25,
        0.25,
        0.25,
        2.0,
        0.0
    ),
    p!(
        "porfix-u-3000-250",
        PorfixU,
        "U 250 – 3000",
        3.0,
        0.25,
        0.25,
        0.25,
        2.5,
        0.0
    ),
    p!(
        "porfix-u-3500-250",
        PorfixU,
        "U 250 – 3500",
        3.5,
        0.25,
        0.25,
        0.25,
        3.0,
        0.0
    ),
    p!(
        "porfix-u-1500-300",
        PorfixU,
        "U 300 – 1500",
        1.5,
        0.3,
        0.25,
        0.25,
        1.0,
        0.0
    ),
    p!(
        "porfix-u-2000-300",
        PorfixU,
        "U 300 – 2000",
        2.0,
        0.3,
        0.25,
        0.25,
        1.5,
        0.0
    ),
    p!(
        "porfix-u-2500-300",
        PorfixU,
        "U 300 – 2500",
        2.5,
        0.3,
        0.25,
        0.25,
        2.0,
        0.0
    ),
    p!(
        "porfix-u-3000-300",
        PorfixU,
        "U 300 – 3000",
        3.0,
        0.3,
        0.25,
        0.25,
        2.5,
        0.0
    ),
    p!(
        "porfix-u-3500-300",
        PorfixU,
        "U 300 – 3500",
        3.5,
        0.3,
        0.25,
        0.25,
        3.0,
        0.0
    ),
    p!(
        "porfix-u-1500-375",
        PorfixU,
        "U 375 – 1500",
        1.5,
        0.375,
        0.25,
        0.25,
        1.0,
        0.0
    ),
    p!(
        "porfix-u-2000-375",
        PorfixU,
        "U 375 – 2000",
        2.0,
        0.375,
        0.25,
        0.25,
        1.5,
        0.0
    ),
    p!(
        "porfix-u-2500-375",
        PorfixU,
        "U 375 – 2500",
        2.5,
        0.375,
        0.25,
        0.25,
        2.0,
        0.0
    ),
    p!(
        "porfix-u-3000-375",
        PorfixU,
        "U 375 – 3000",
        3.0,
        0.375,
        0.25,
        0.25,
        2.5,
        0.0
    ),
    p!(
        "porfix-u-3500-375",
        PorfixU,
        "U 375 – 3500",
        3.5,
        0.375,
        0.25,
        0.25,
        3.0,
        0.0
    ),
    // Porotherm KP 7 (70 × 238)
    p!(
        "kp7-100",
        Kp7,
        "KP 7 – 100",
        1.0,
        0.07,
        0.238,
        0.125,
        0.75,
        35.0
    ),
    p!(
        "kp7-125",
        Kp7,
        "KP 7 – 125",
        1.25,
        0.07,
        0.238,
        0.125,
        1.0,
        43.8
    ),
    p!(
        "kp7-150",
        Kp7,
        "KP 7 – 150",
        1.5,
        0.07,
        0.238,
        0.125,
        1.25,
        52.5
    ),
    p!(
        "kp7-175",
        Kp7,
        "KP 7 – 175",
        1.75,
        0.07,
        0.238,
        0.125,
        1.5,
        61.3
    ),
    p!(
        "kp7-200",
        Kp7,
        "KP 7 – 200",
        2.0,
        0.07,
        0.238,
        0.2,
        1.6,
        70.0
    ),
    p!(
        "kp7-225",
        Kp7,
        "KP 7 – 225",
        2.25,
        0.07,
        0.238,
        0.2,
        1.85,
        78.8
    ),
    p!(
        "kp7-250",
        Kp7,
        "KP 7 – 250",
        2.5,
        0.07,
        0.238,
        0.25,
        2.0,
        87.5
    ),
    p!(
        "kp7-275",
        Kp7,
        "KP 7 – 275",
        2.75,
        0.07,
        0.238,
        0.25,
        2.25,
        96.3
    ),
    p!(
        "kp7-300",
        Kp7,
        "KP 7 – 300",
        3.0,
        0.07,
        0.238,
        0.25,
        2.5,
        105.0
    ),
    p!(
        "kp7-325",
        Kp7,
        "KP 7 – 325",
        3.25,
        0.07,
        0.238,
        0.25,
        2.75,
        113.8
    ),
    p!(
        "kp7-350",
        Kp7,
        "KP 7 – 350",
        3.5,
        0.07,
        0.238,
        0.25,
        3.0,
        122.5
    ),
    // Porotherm KP Vario 100 to 175 (125 × 238)
    p!(
        "kp-vario-100",
        KpVario,
        "KP Vario 100",
        1.0,
        0.125,
        0.238,
        0.125,
        0.75,
        59.0
    ),
    p!(
        "kp-vario-125",
        KpVario,
        "KP Vario 125",
        1.25,
        0.125,
        0.238,
        0.125,
        1.0,
        74.3
    ),
    p!(
        "kp-vario-150",
        KpVario,
        "KP Vario 150",
        1.5,
        0.125,
        0.238,
        0.125,
        1.25,
        89.9
    ),
    p!(
        "kp-vario-175",
        KpVario,
        "KP Vario 175",
        1.75,
        0.125,
        0.238,
        0.125,
        1.5,
        106.6
    ),
    // Porotherm KP 11,5 and KP 14,5 (flat, 71 high)
    p!(
        "kp115-100",
        Kp115,
        "KP 11,5 – 100",
        1.0,
        0.115,
        0.071,
        0.12,
        0.75,
        17.0
    ),
    p!(
        "kp115-125",
        Kp115,
        "KP 11,5 – 125",
        1.25,
        0.115,
        0.071,
        0.12,
        1.0,
        21.3
    ),
    p!(
        "kp115-150",
        Kp115,
        "KP 11,5 – 150",
        1.5,
        0.115,
        0.071,
        0.12,
        1.25,
        25.5
    ),
    p!(
        "kp115-175",
        Kp115,
        "KP 11,5 – 175",
        1.75,
        0.115,
        0.071,
        0.12,
        1.5,
        29.8
    ),
    p!(
        "kp115-200",
        Kp115,
        "KP 11,5 – 200",
        2.0,
        0.115,
        0.071,
        0.12,
        1.75,
        34.0
    ),
    p!(
        "kp115-225",
        Kp115,
        "KP 11,5 – 225",
        2.25,
        0.115,
        0.071,
        0.12,
        2.0,
        38.3
    ),
    p!(
        "kp115-250",
        Kp115,
        "KP 11,5 – 250",
        2.5,
        0.115,
        0.071,
        0.12,
        2.25,
        42.5
    ),
    p!(
        "kp115-275",
        Kp115,
        "KP 11,5 – 275",
        2.75,
        0.115,
        0.071,
        0.12,
        2.5,
        46.8
    ),
    p!(
        "kp145-100",
        Kp145,
        "KP 14,5 – 100",
        1.0,
        0.145,
        0.071,
        0.12,
        0.75,
        20.0
    ),
    p!(
        "kp145-125",
        Kp145,
        "KP 14,5 – 125",
        1.25,
        0.145,
        0.071,
        0.12,
        1.0,
        25.0
    ),
    p!(
        "kp145-150",
        Kp145,
        "KP 14,5 – 150",
        1.5,
        0.145,
        0.071,
        0.12,
        1.25,
        30.0
    ),
    p!(
        "kp145-175",
        Kp145,
        "KP 14,5 – 175",
        1.75,
        0.145,
        0.071,
        0.12,
        1.5,
        35.0
    ),
    p!(
        "kp145-200",
        Kp145,
        "KP 14,5 – 200",
        2.0,
        0.145,
        0.071,
        0.12,
        1.75,
        40.0
    ),
    p!(
        "kp145-225",
        Kp145,
        "KP 14,5 – 225",
        2.25,
        0.145,
        0.071,
        0.12,
        2.0,
        45.0
    ),
    p!(
        "kp145-250",
        Kp145,
        "KP 14,5 – 250",
        2.5,
        0.145,
        0.071,
        0.12,
        2.25,
        50.0
    ),
    p!(
        "kp145-275",
        Kp145,
        "KP 14,5 – 275",
        2.75,
        0.145,
        0.071,
        0.12,
        2.5,
        55.0
    ),
    // Porotherm KP XL (technical sheet KP XL, Wienerberger): composite,
    // cast with the ring beam; bearing 250, mass of the precast pieces
    p!(
        "kp-xl-30-375",
        KpXl,
        "KP XL 30 - 375",
        3.75,
        0.3,
        0.238,
        0.25,
        3.25,
        279.2
    ),
    p!(
        "kp-xl-30-400",
        KpXl,
        "KP XL 30 - 400",
        4.0,
        0.3,
        0.238,
        0.25,
        3.5,
        297.8
    ),
    p!(
        "kp-xl-30-425",
        KpXl,
        "KP XL 30 - 425",
        4.25,
        0.3,
        0.238,
        0.25,
        3.75,
        316.6
    ),
    p!(
        "kp-xl-30-450",
        KpXl,
        "KP XL 30 - 450",
        4.5,
        0.3,
        0.238,
        0.25,
        4.0,
        335.4
    ),
    p!(
        "kp-xl-30-475",
        KpXl,
        "KP XL 30 - 475",
        4.75,
        0.3,
        0.238,
        0.25,
        4.25,
        352.2
    ),
    p!(
        "kp-xl-30-500",
        KpXl,
        "KP XL 30 - 500",
        5.0,
        0.3,
        0.238,
        0.25,
        4.5,
        370.8
    ),
    p!(
        "kp-xl-30-525",
        KpXl,
        "KP XL 30 - 525",
        5.25,
        0.3,
        0.238,
        0.25,
        4.75,
        389.6
    ),
    p!(
        "kp-xl-30-550",
        KpXl,
        "KP XL 30 - 550",
        5.5,
        0.3,
        0.238,
        0.25,
        5.0,
        408.4
    ),
    p!(
        "kp-xl-38-575",
        KpXl,
        "KP XL 38 - 575",
        5.75,
        0.38,
        0.238,
        0.25,
        5.25,
        428.6
    ),
    p!(
        "kp-xl-38-600",
        KpXl,
        "KP XL 38 - 600",
        6.0,
        0.38,
        0.238,
        0.25,
        5.5,
        447.0
    ),
    p!(
        "kp-xl-38-625",
        KpXl,
        "KP XL 38 - 625",
        6.25,
        0.38,
        0.238,
        0.25,
        5.75,
        462.6
    ),
    p!(
        "kp-xl-38-650",
        KpXl,
        "KP XL 38 - 650",
        6.5,
        0.38,
        0.238,
        0.25,
        6.0,
        481.4
    ),
];

/// Porotherm KP XL after casting, from its sheet: for a ring beam (or
/// slab) over it 210 mm and 250 mm high, MRd kNm, VRd kN and gRd kN/m
/// (the most uniform design load, its own weight left out).
const KP_XL_CAPACITY: &[(&str, [[f64; 3]; 2])] = &[
    (
        "kp-xl-30-375",
        [[53.23, 125.18, 30.23], [59.05, 138.09, 33.62]],
    ),
    (
        "kp-xl-30-400",
        [[58.39, 122.72, 28.68], [64.81, 135.63, 31.93]],
    ),
    (
        "kp-xl-30-425",
        [[65.00, 122.72, 27.96], [72.17, 135.63, 31.14]],
    ),
    (
        "kp-xl-30-450",
        [[75.00, 120.72, 28.68], [83.36, 133.63, 31.98]],
    ),
    (
        "kp-xl-30-475",
        [[77.04, 118.56, 25.90], [85.72, 131.47, 28.92]],
    ),
    (
        "kp-xl-30-500",
        [[90.12, 116.24, 27.42], [100.41, 129.15, 30.66]],
    ),
    (
        "kp-xl-30-525",
        [[104.36, 113.75, 28.86], [126.40, 129.07, 35.51]],
    ),
    (
        "kp-xl-30-550",
        [[128.03, 111.07, 32.62], [143.28, 123.98, 36.65]],
    ),
    (
        "kp-xl-38-575",
        [[151.36, 112.62, 33.26], [163.73, 125.13, 35.94]],
    ),
    (
        "kp-xl-38-600",
        [[146.55, 112.62, 28.80], [163.73, 125.53, 32.35]],
    ),
    (
        "kp-xl-38-625",
        [[173.50, 110.41, 30.45], [194.34, 123.32, 34.18]],
    ),
    (
        "kp-xl-38-650",
        [[165.11, 110.41, 27.23], [194.34, 123.32, 32.56]],
    ),
];

/// A KP XL lintel's MRd (kNm), VRd (kN) and gRd (kN/m) under a ring
/// beam `ring` m high: between the sheet's 210 and 250 mm by straight
/// interpolation, as the sheet allows; outside them the nearer.
pub fn kp_xl_capacity(id: &str, ring: f64) -> Option<[f64; 3]> {
    let (_, [a, b]) = KP_XL_CAPACITY.iter().find(|x| x.0 == id)?;
    let t = ((ring - 0.21) / 0.04).clamp(0.0, 1.0);
    Some([0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t))
}

pub fn product(id: &str) -> Option<&'static LintelProduct> {
    PRODUCTS.iter().find(|p| p.id == id)
}

/// The widths a family comes in, narrowest first.
pub fn widths(family: Family) -> Vec<f64> {
    let mut w: Vec<f64> = PRODUCTS
        .iter()
        .filter(|p| p.family == family)
        .map(|p| p.width)
        .collect();
    w.sort_by(f64::total_cmp);
    w.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    w
}

/// The shortest piece of `family`, `width` wide, that spans a clear
/// opening `span` wide.
pub fn fit(family: Family, width: f64, span: f64) -> Option<&'static LintelProduct> {
    PRODUCTS
        .iter()
        // a KP XL 30 serves a wider wall too (300 is its least width)
        .filter(|p| {
            p.family == family
                && ((p.width - width).abs() < 1e-6
                    || (family == Family::KpXl && p.width < width + 1e-6))
        })
        .filter(|p| p.max_span + 1e-6 >= span)
        .min_by(|a, b| a.length.total_cmp(&b.length))
}

/// A system lintel's piece, on the beam it is drawn as.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lintel {
    /// The product, by [`LintelProduct::id`].
    pub product: String,
    /// The door, window or hole it spans; none is a lintel standing on
    /// its own (its opening deleted, or placed so).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening: Option<ElementId>,
    /// How far its inner side is from the wall core's inner face, across
    /// the wall: pieces lie side by side from the inside out.
    #[serde(default)]
    pub across: f64,
    /// How far its position mark in a structural plan (P1 over SH=…)
    /// was dragged from where the drawing puts it, m (George 2026-11-24);
    /// kept on the opening's first piece.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark_at: Option<glam::DVec2>,
    /// A monolithic RC lintel, its reinforcement tied in the formwork and
    /// cast on site (George 2026-11-25), instead of a product: its bearing
    /// and depth. It fills the core across, less the insulation laid into
    /// the formwork on its exterior side and under it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cast: Option<CastLintel>,
}

/// A monolithic RC lintel's own sizes, m.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CastLintel {
    /// How far it rests on the wall at each end.
    pub bearing: f64,
    /// The concrete's depth (without the insulation under it).
    pub depth: f64,
}

/// The name of a monolithic RC lintel.
pub const RC_LINTEL: &str = "ŽB monolitický preklad";

impl CastLintel {
    /// The usual sizes over a clear opening `span` wide: resting at least
    /// 250 mm on each side (PORFIX odborná príručka, U-profile lintel
    /// detail; eStav.cz "minimálně 250 mm na každé straně"), 400 mm over
    /// spans above 4 m (PORFIX: "namiesto 300 mm ... 400 – 500 mm" for
    /// 4 to 6 m); deep a twelfth of the span (VUT Brno FAST, předběžné
    /// rozměry: průvlak "střešní a méně zatížené h = L/15 ~ L/12"; George
    /// chose L/12 on 2026-11-25), rounded up to 50 mm, never less than
    /// the 250 mm of a PORFIX U-profile.
    pub fn usual(span: f64) -> CastLintel {
        let bearing = if span > 4.0 + 1e-9 { 0.4 } else { 0.25 };
        let depth = ((span / 12.0 / 0.05 - 1e-9).ceil() * 0.05).max(0.25);
        CastLintel {
            bearing,
            depth: (depth * 1000.0).round() / 1000.0,
        }
    }
}

/// What [`lintels_for`] lays over an opening.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LintelSet {
    pub family: Family,
    pub width: f64,
    pub count: usize,
}

/// The usual set for an opening in `wall`: the makers' tables by the
/// thickness of its core. PORFIX: 250 takes 2 × NP 125, 300 and 375 take
/// 3 × NP 100, 500 takes 4 × NP 100; a partition one P lintel its own
/// width (200: two of 100). Porotherm KP 7: as many as fit across.
pub fn usual_set(model: &Model, wall: &Wall, family: Family) -> LintelSet {
    let core = core_thickness(model, wall);
    let widths = widths(family);
    let near = |t: f64| {
        widths
            .iter()
            .copied()
            .min_by(|a, b| (a - t).abs().total_cmp(&(b - t).abs()))
            .unwrap_or(t)
    };
    let (width, count) = match family {
        Family::PorfixNp => {
            if core < 0.27 {
                (0.125, 2)
            } else if core < 0.45 {
                (0.1, 3)
            } else {
                (0.1, 4)
            }
        }
        Family::PorfixP => {
            if core > 0.175 {
                (0.1, ((core / 0.1).round() as usize).max(1))
            } else {
                (near(core), 1)
            }
        }
        Family::PorfixU => (near(core), 1),
        // one composite lintel the wall's width: 380 from a 380 core
        Family::KpXl => (if core + 1e-6 >= 0.38 { 0.38 } else { 0.3 }, 1),
        _ => {
            let w = widths.first().copied().unwrap_or(0.07);
            // pieces with a joint of mortar between, at least one
            (w, ((core + 0.01) / (w + 0.03)).floor().max(1.0) as usize)
        }
    };
    LintelSet {
        family,
        width,
        count,
    }
}

/// How thick the wall's core is.
pub fn core_thickness(model: &Model, wall: &Wall) -> f64 {
    let (a, b) = crate::walls::core_span(model, wall);
    (b - a).abs()
}

/// The lintels `set` lays over the opening `opening_id`, side by side
/// from the core's inner face out; an error when the opening is too wide
/// for the family.
pub fn lintels_for(
    model: &Model,
    opening_id: ElementId,
    set: LintelSet,
) -> Result<Vec<Element>, String> {
    let Some(ElementKind::Opening(o)) = model.get(opening_id).map(|e| &e.kind) else {
        return Err("Pick a door, window or opening.".into());
    };
    let Some(ElementKind::Wall(w)) = model.get(o.host).map(|e| &e.kind) else {
        return Err("The opening has no wall.".into());
    };
    let Some(p) = fit(set.family, set.width, o.width) else {
        return Err(format!(
            "The opening is {} mm wide: no {} {} mm wide spans it. Use an RC lintel or a U-profile.",
            (o.width * 1000.0).round(),
            set.family.label(),
            (set.width * 1000.0).round()
        ));
    };
    let mut out = vec![];
    for k in 0..set.count.max(1) {
        let lintel = Lintel {
            product: p.id.to_string(),
            opening: Some(opening_id),
            across: k as f64 * p.width,
            mark_at: None,
            cast: None,
        };
        let mut b = Beam::new(w.level, DVec2::ZERO, DVec2::X);
        b.lintel = Some(lintel);
        let Some(b) = fitted(model, &b) else {
            continue;
        };
        out.push(Element::new(
            format!("{} {}", set.family.maker(), p.name),
            ElementKind::Beam(b),
        ));
    }
    Ok(out)
}

/// `b` laid where its lintel's opening has it: over the opening, centred,
/// its underside on the opening's head; `None` when it is no lintel or its
/// opening is gone.
pub fn fitted(model: &Model, b: &Beam) -> Option<Beam> {
    let l = b.lintel.as_ref()?;
    let o_id = l.opening?;
    let ElementKind::Opening(o) = &model.get(o_id)?.kind else {
        return None;
    };
    let ElementKind::Wall(w) = &model.get(o.host)?.kind else {
        return None;
    };
    if let Some(c) = l.cast {
        return Some(place_cast(model, b, c, o, w));
    }
    let p = product(&l.product)?;
    Some(place(model, b, l, p, o, w))
}

/// A monolithic RC lintel `b` (sizes `c`) over `o`: across the whole core
/// less the insulation on its exterior side, flush with the core's inner
/// face, its insulation's underside (or its own) on the opening's head,
/// resting `c.bearing` on each side. Insulation only outside and under,
/// each its own thickness, no render: it is laid into the formwork.
fn place_cast(model: &Model, b: &Beam, c: CastLintel, o: &Opening, w: &Wall) -> Beam {
    let d = w.direction();
    let n = d.perp();
    let (ext, int) = crate::walls::core_span(model, w);
    let core = (int - ext).abs();
    let mut nb = b.clone();
    // along the wall, so its right is the wall's right
    nb.start = w.start;
    nb.end = w.start + d;
    let ext_side = if ext < int { 1 } else { 0 };
    if let Some(wr) = &mut nb.wrap {
        for i in [0, 1, 3] {
            if i != ext_side {
                wr.sides[i] = 0.0;
            }
        }
        wr.plaster = None;
        wr.sides[ext_side] = wr.side(ext_side).min((core - 0.05).max(0.0));
        // VELOX boards outside (and inside): their own thicknesses
        crate::wrap::Formwork::lay(wr, ext_side);
    }
    if nb.wrap.as_ref().is_some_and(|w| w.is_empty()) {
        nb.wrap = None;
    }
    let (t_ext, t_under) = nb.wrap.as_ref().map_or((0.0, 0.0), |w| {
        (w.through(ext_side) + w.through(1 - ext_side), w.side(2))
    });
    let width = (core - t_ext).max(0.05);
    let out = if ext < int { -1.0 } else { 1.0 };
    // flush with the core's inner face, or the inner board's face
    let t_int = nb.wrap.as_ref().map_or(0.0, |w| w.through(1 - ext_side));
    let across = int + out * (t_int + width / 2.0);
    let mid = o.offset + o.width / 2.0;
    let length = o.width + 2.0 * c.bearing.max(0.0);
    let at = |u: f64| w.start + d * u + n * across;
    let head = crate::walls::base_z(model, w) + o.sill + o.height;
    let top = crate::members::elevation(model, w.level) + storey_height(model, w.level);
    let mut depth = c.depth.max(0.05);
    // reaching into the wall's ring beam: one RC lintel up to its top,
    // the ring beam stopping at its ends (George 2026-11-25)
    if let Some(ring) = ring_depth_on(model, o.host) {
        let wtop = wall_top(model, w);
        if head + t_under + depth > wtop - ring + 1e-6 {
            depth = (wtop - head - t_under).max(depth);
        }
    }
    nb.level = w.level;
    nb.start = at(mid - length / 2.0);
    nb.end = at(mid + length / 2.0);
    nb.width = width;
    nb.depth = depth;
    nb.top_offset = top - (head + t_under + depth);
    nb.section = None;
    nb.rise = [0.0, 0.0];
    nb.up = None;
    nb.extend = [0.0, 0.0];
    nb.cuts = [None, None];
    nb
}

/// The depth of the ring beam on wall `wall`, if it has one.
fn ring_depth_on(model: &Model, wall: ElementId) -> Option<f64> {
    model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Beam(b)
                if e.name == RING_BEAM
                    && b.lintel.is_none()
                    && b.piece.is_none()
                    && ring_wall(model, b) == Some(wall) =>
            {
                Some(b.depth)
            }
            _ => None,
        })
        .reduce(f64::max)
}

/// Where along wall `wall` (from its start) RC lintels merged with its
/// ring beam stand: the ring beam is not cast there.
fn ring_gaps(model: &Model, wall: ElementId, w: &Wall) -> Vec<(f64, f64)> {
    let wtop = wall_top(model, w);
    let d = w.direction();
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Beam(b) = &e.kind else {
            continue;
        };
        let Some(l) = b.lintel.as_ref().filter(|l| l.cast.is_some()) else {
            continue;
        };
        let host = l.opening.and_then(|o| match model.get(o).map(|x| &x.kind) {
            Some(ElementKind::Opening(o)) => Some(o.host),
            _ => None,
        });
        if host != Some(wall) {
            continue;
        }
        let Some(f) = fitted(model, b) else { continue };
        if (f.top(model) - wtop).abs() > 1e-6 {
            continue;
        }
        let (a, c) = ((f.start - w.start).dot(d), (f.end - w.start).dot(d));
        out.push((a.min(c), a.max(c)));
    }
    out
}

/// A monolithic RC lintel over opening `opening`, of the usual sizes
/// ([`CastLintel::usual`]), in the project's reinforced concrete.
pub fn cast_lintel(model: &Model, opening: ElementId) -> Option<Element> {
    let ElementKind::Opening(o) = &model.get(opening)?.kind else {
        return None;
    };
    let ElementKind::Wall(w) = &model.get(o.host)?.kind else {
        return None;
    };
    let mut b = Beam::new(w.level, DVec2::ZERO, DVec2::X);
    b.material = concrete(model);
    b.lintel = Some(Lintel {
        product: String::new(),
        opening: Some(opening),
        across: 0.0,
        mark_at: None,
        cast: Some(CastLintel::usual(o.width)),
    });
    let b = fitted(model, &b)?;
    Some(Element::new(RC_LINTEL, ElementKind::Beam(b)))
}

fn place(model: &Model, b: &Beam, l: &Lintel, p: &LintelProduct, o: &Opening, w: &Wall) -> Beam {
    let d = w.direction();
    let n = d.perp();
    let (ext, int) = crate::walls::core_span(model, w);
    // from the inner face outwards
    let out = if ext < int { -1.0 } else { 1.0 };
    let across = int + out * (l.across + p.width / 2.0);
    let mid = o.offset + o.width / 2.0;
    let at = |u: f64| w.start + d * u + n * across;
    let head = crate::walls::base_z(model, w) + o.sill + o.height;
    let top = crate::members::elevation(model, w.level) + storey_height(model, w.level);
    let mut nb = b.clone();
    nb.level = w.level;
    nb.start = at(mid - p.length / 2.0);
    nb.end = at(mid + p.length / 2.0);
    nb.width = p.width;
    nb.depth = p.height;
    nb.top_offset = top - (head + p.height);
    nb.section = None;
    nb.rise = [0.0, 0.0];
    nb.up = None;
    nb.extend = [0.0, 0.0];
    nb.cuts = [None, None];
    nb
}

/// Is `e` a system lintel?
pub fn is_lintel(e: &Element) -> bool {
    matches!(&e.kind, ElementKind::Beam(b) if b.lintel.is_some())
}

/// The lintels over the opening `opening`.
pub fn lintels_over(model: &Model, opening: ElementId) -> Vec<ElementId> {
    model
        .iter()
        .filter(|e| matches!(&e.kind, ElementKind::Beam(b) if b.lintel.as_ref().is_some_and(|l| l.opening == Some(opening))))
        .map(|e| e.id)
        .collect()
}

/// What a plan writes by an opening's lintels: `2× NP 1500/125`, the
/// pieces of one product counted.
pub fn label(model: &Model, ids: &[ElementId]) -> String {
    let mut counts: Vec<(&'static str, usize)> = vec![];
    let mut cast: Vec<String> = vec![];
    for id in ids {
        let Some(ElementKind::Beam(b)) = model.get(*id).map(|e| &e.kind) else {
            continue;
        };
        if b.lintel.as_ref().is_some_and(|l| l.cast.is_some()) {
            cast.push(format!(
                "ŽB {:.0}×{:.0}",
                b.width * 1000.0,
                b.depth * 1000.0
            ));
            continue;
        }
        let Some(p) = b.lintel.as_ref().and_then(|l| product(&l.product)) else {
            continue;
        };
        match counts.iter_mut().find(|(n, _)| *n == p.name) {
            Some(c) => c.1 += 1,
            None => counts.push((p.name, 1)),
        }
    }
    let mut parts: Vec<String> = counts.iter().map(|(n, c)| format!("{c}× {n}")).collect();
    for c in cast {
        if !parts.contains(&c) {
            parts.push(c);
        }
    }
    let mut out = parts.join(" + ");
    // the insulation laid into an RC lintel's formwork
    if let Some(id) = ids.iter().find(|id| {
        matches!(model.get(**id).map(|e| &e.kind),
            Some(ElementKind::Beam(b)) if b.lintel.as_ref().is_some_and(|l| l.cast.is_some()))
    }) {
        out.push_str(&insulation_note(model, *id));
    }
    out
}

/// The insulation laid into a concrete member's formwork, as a list
/// says it: ` + XPS 50 mm zvonka, 30 mm zospodu` (George 2026-11-25:
/// the size and the insulation's thickness in the legend and lists).
pub fn insulation_note(model: &Model, id: ElementId) -> String {
    let Some(e) = model.get(id) else {
        return String::new();
    };
    let Some(ElementKind::Beam(b)) = Some(&e.kind) else {
        return String::new();
    };
    let Some(w) = &b.wrap else {
        return String::new();
    };
    let Some(ext) = crate::wrap::exterior_side(model, e) else {
        return String::new();
    };
    if let Some(f) = &w.formwork {
        // VELOX boards: by the maker's name (George 2026-11-27)
        let mut parts = vec![format!(
            "{} {} {}",
            crate::velox::MAKER,
            f.product().short(),
            crate::words::tr("on the outside")
        )];
        if f.two_sided {
            parts.push(format!(
                "{} {}",
                crate::velox::WSC.short(),
                crate::words::tr("on the inside")
            ));
        }
        if w.side(2) > 1e-6 {
            let under = w
                .insulation
                .and_then(|m| model.get(m))
                .map(|m| {
                    m.name
                        .split([' ', '–', ','])
                        .next()
                        .unwrap_or("")
                        .to_string()
                })
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| crate::words::tr("insulation"));
            parts.push(format!(
                "{under} {:.0} mm {}",
                w.side(2) * 1000.0,
                crate::words::tr("underneath")
            ));
        }
        return format!(" + {}", parts.join(", "));
    }
    let name = w
        .insulation
        .and_then(|m| model.get(m))
        .map(|m| {
            m.name
                .split([' ', '–', ','])
                .next()
                .unwrap_or("")
                .to_string()
        })
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| crate::words::tr("insulation"));
    let mut parts = vec![];
    if w.side(ext) > 1e-6 {
        parts.push(format!(
            "{:.0} mm {}",
            w.side(ext) * 1000.0,
            crate::words::tr("outside")
        ));
    }
    if w.side(2) > 1e-6 {
        parts.push(format!(
            "{:.0} mm {}",
            w.side(2) * 1000.0,
            crate::words::tr("underneath")
        ));
    }
    if parts.is_empty() {
        return String::new();
    }
    format!(" + {name} {}", parts.join(", "))
}

/// What is wrong with the lintel `b` as it lies: bearing shorter than its
/// product needs, or an opening wider than it spans.
pub fn lintel_issue(model: &Model, b: &Beam) -> Option<String> {
    let l = b.lintel.as_ref()?;
    let p = product(&l.product)?;
    let ElementKind::Opening(o) = &model.get(l.opening?)?.kind else {
        return None;
    };
    if o.width > p.max_span + 1e-6 {
        return Some(format!(
            "{}: the opening is {:.0} mm, the lintel spans {:.0} mm at most",
            p.name,
            o.width * 1000.0,
            p.max_span * 1000.0
        ));
    }
    let bearing = (p.length - o.width) / 2.0;
    (bearing + 1e-6 < p.bearing).then(|| {
        format!(
            "{}: bearing {:.0} mm, it needs {:.0} mm",
            p.name,
            bearing * 1000.0,
            p.bearing * 1000.0
        )
    })
}

/// After `tx`: lintels over openings it moved, resized or took the walls
/// of laid over them again; lintels whose opening went, left standing
/// on their own. Run by the [`crate::Document`] after every edit.
pub fn settle(model: &Model, tx: &Transaction) -> Transaction {
    let out = settle_columns(model, tx, Transaction::new(tx.label.clone()));
    let touches = tx.ops.iter().any(|o| match o {
        Op::Insert(e) | Op::Remove(e) => match &e.kind {
            ElementKind::Opening(_) | ElementKind::Wall(_) | ElementKind::Level(_) => true,
            // an RC lintel put or taken away: the ring beam it meets
            ElementKind::Beam(b) => b.lintel.as_ref().is_some_and(|l| l.cast.is_some()),
            _ => false,
        },
        Op::Replace { after, .. } => matches!(
            after.kind,
            ElementKind::Opening(_)
                | ElementKind::Wall(_)
                | ElementKind::Level(_)
                | ElementKind::Beam(_)
        ),
    });
    if !touches {
        return out;
    }
    let mut out = refit_rings(model, out);
    for e in model.iter() {
        let ElementKind::Beam(b) = &e.kind else {
            continue;
        };
        let Some(l) = &b.lintel else { continue };
        let next = match fitted(model, b) {
            Some(nb) => nb,
            // its opening gone: it stays where it is, on its own
            None if l.opening.is_some_and(|o| model.get(o).is_none()) => {
                let mut nb = b.clone();
                if let Some(nl) = &mut nb.lintel {
                    nl.opening = None;
                }
                nb
            }
            None => continue,
        };
        if next != *b {
            let mut after = e.clone();
            after.kind = ElementKind::Beam(next);
            out = out.replace(e.clone(), after);
        }
    }
    out
}

/// The project's rules for how long masonry walls may run.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MasonryRules {
    /// The longest stretch of a load-bearing wall between the walls and
    /// columns that brace it, m.
    pub bearing_max: f64,
    /// A wall whose core is at least this thick is load-bearing, unless
    /// it says otherwise.
    pub bearing_from: f64,
    /// A seismic area (STN EN 1998-1): confining elements at most
    /// [`SEISMIC_MAX`] apart.
    pub seismic: bool,
    /// The width of the RC columns put into walls, along the wall, m.
    pub column_width: f64,
    /// The RC ring beam's height, m, and what of the core's thickness
    /// its shuttering on the outer side takes (a ring beam block and
    /// insulation, Porotherm VT 8 + 50 mm: 0,13), m.
    pub ring_depth: f64,
    pub ring_shutter: f64,
    /// Walls whose load-bearing core is no thicker than this, m, are
    /// neither checked nor count as bracing (thin partitions).
    #[serde(default = "default_unchecked")]
    pub unchecked_upto: f64,
}

/// Partitions up to 160 mm of core are left out of the length check.
pub const UNCHECKED_UPTO: f64 = 0.16;

fn default_unchecked() -> f64 {
    UNCHECKED_UPTO
}

/// STN EN 1998-1, 9.5.3 (2): confining elements no more than 5 m apart.
pub const SEISMIC_MAX: f64 = 5.0;

impl Default for MasonryRules {
    fn default() -> Self {
        Self {
            bearing_max: 6.0,
            bearing_from: 0.24,
            seismic: false,
            column_width: 0.25,
            ring_depth: 0.25,
            ring_shutter: 0.0,
            unchecked_upto: UNCHECKED_UPTO,
        }
    }
}

impl MasonryRules {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// The same, as the project keeps them (boxed, to keep elements small).
    #[allow(clippy::borrowed_box)]
    pub fn boxed_default(b: &Box<Self>) -> bool {
        b.is_default()
    }

    /// The longest stretch of a load-bearing wall.
    pub fn bearing_limit(&self) -> f64 {
        if self.seismic {
            self.bearing_max.min(SEISMIC_MAX)
        } else {
            self.bearing_max
        }
    }
}

/// The project's masonry rules.
pub fn rules(model: &Model) -> MasonryRules {
    model
        .project_info()
        .map(|(_, p)| p.masonry.as_ref().clone())
        .unwrap_or_default()
}

/// Is `wall` load-bearing: as it is set, else by its core's thickness,
/// or because it stands on a foundation or a wall of a storey below
/// along most of its length (a 200 mm inner wall on its own foundation
/// wall carries, George 2026-11-24).
pub fn is_bearing(model: &Model, wall: &Wall, rules: &MasonryRules) -> bool {
    wall.bearing.unwrap_or_else(|| {
        core_thickness(model, wall) + 1e-6 >= rules.bearing_from || stands_on_support(model, wall)
    })
}

/// Does a wall of a lower storey or a strip footing run under `wall`
/// along at least half its length, on its line (within 0,2 m)?
pub fn stands_on_support(model: &Model, wall: &Wall) -> bool {
    use crate::ElementKind;
    let Some(own) = model.level(wall.level).map(|l| l.elevation) else {
        return false;
    };
    let len = wall.start.distance(wall.end);
    if len < 1e-6 {
        return false;
    }
    let d = (wall.end - wall.start) / len;
    let mut covered: Vec<(f64, f64)> = vec![];
    for e in model.iter() {
        let (level, a, b) = match &e.kind {
            ElementKind::Wall(w) => (w.level, w.start, w.end),
            ElementKind::Footing(f) if f.start.distance(f.end) > 1e-6 => (f.level, f.start, f.end),
            _ => continue,
        };
        if model.level(level).is_none_or(|l| l.elevation >= own - 1e-6) {
            continue;
        }
        let u = b - a;
        if u.length() < 1e-6 || d.perp_dot(u.normalize()).abs() > 0.02 {
            continue;
        }
        let off = |p: DVec2| d.perp_dot(p - wall.start).abs();
        if off(a) > 0.2 || off(b) > 0.2 {
            continue;
        }
        let (s, t) = ((a - wall.start).dot(d), (b - wall.start).dot(d));
        let (s, t) = (s.min(t).max(0.0), s.max(t).min(len));
        if t > s {
            covered.push((s, t));
        }
    }
    covered.sort_by(|x, y| x.0.total_cmp(&y.0));
    let (mut sum, mut end) = (0.0, 0.0f64);
    for (s, t) in covered {
        let s = s.max(end);
        if t > s {
            sum += t - s;
            end = t;
        }
    }
    sum >= len * 0.5
}

/// PORFIX's table of free-standing partitions (Odborná príručka, 04-1,
/// s. 5): thickness, mm; for a length under twice the height, the
/// highest wall propped at its top and free at its top, mm; for a length
/// under 3,5 times the height, the same, and the longest wall propped
/// and free, m.
/// A row of [`PARTITIONS`].
type PartitionRow = (f64, [f64; 2], [f64; 2], [f64; 2]);

const PARTITIONS: [PartitionRow; 4] = [
    (100.0, [2580.0, 1810.0], [2064.0, 1448.0], [7.22, 5.07]),
    (125.0, [3230.0, 2260.0], [2584.0, 1808.0], [9.04, 6.33]),
    (150.0, [3870.0, 2710.0], [3096.0, 2168.0], [10.84, 7.59]),
    (200.0, [5150.0, 3610.0], [4120.0, 2888.0], [14.42, 10.11]),
];

/// For a partition `t` thick (m), its top propped or not: the highest it
/// may be for a length under twice its height, and under 3,5 times, and
/// the longest it may be (all m), from [`PARTITIONS`] (between rows in
/// proportion, below and above them in proportion to the thickness).
pub fn partition_limits(t: f64, propped: bool) -> (f64, f64, f64) {
    let k = usize::from(!propped);
    let mm = t * 1000.0;
    let row = |r: &PartitionRow| (r.1[k] / 1000.0, r.2[k] / 1000.0, r.3[k]);
    let first = &PARTITIONS[0];
    let last = &PARTITIONS[PARTITIONS.len() - 1];
    if mm <= first.0 {
        let (a, b, c) = row(first);
        let s = mm / first.0;
        return (a * s, b * s, c * s);
    }
    if mm >= last.0 {
        let (a, b, c) = row(last);
        let s = mm / last.0;
        return (a * s, b * s, c * s);
    }
    for w in PARTITIONS.windows(2) {
        if mm <= w[1].0 {
            let s = (mm - w[0].0) / (w[1].0 - w[0].0);
            let (a0, b0, c0) = row(&w[0]);
            let (a1, b1, c1) = row(&w[1]);
            return (a0 + (a1 - a0) * s, b0 + (b1 - b0) * s, c0 + (c1 - c0) * s);
        }
    }
    row(last)
}

/// What is wrong with a stretch of masonry wall.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WallFault {
    /// Longer between braces than `limit`.
    TooLong { length: f64, limit: f64 },
    /// A partition higher than it may stand, `limit`.
    TooHigh { height: f64, limit: f64 },
}

/// A stretch of masonry wall between braces that breaks a rule.
#[derive(Clone, Debug, PartialEq)]
pub struct WallIssue {
    pub level: ElementId,
    /// The walls of the stretch (a wall drawn in pieces is one).
    pub walls: Vec<ElementId>,
    /// Its ends, on the walls' centreline.
    pub a: DVec2,
    pub b: DVec2,
    pub bearing: bool,
    pub fault: WallFault,
}

impl WallIssue {
    /// "Load-bearing wall 7,40 m between braces, 6,00 m at most".
    pub fn message(&self) -> String {
        let what = if self.bearing {
            "Load-bearing wall"
        } else {
            "Partition"
        };
        match self.fault {
            WallFault::TooLong { length, limit } => format!(
                "{what} {} m between cross walls or columns, {} m at most",
                comma(length),
                comma(limit)
            ),
            WallFault::TooHigh { height, limit } => format!(
                "{what} {} m high, {} m at most for its length",
                comma(height),
                comma(limit)
            ),
        }
    }
}

fn comma(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}

/// A straight line of masonry walls of one level: the walls drawn along it
/// end to end, as one.
struct Run {
    level: ElementId,
    walls: Vec<ElementId>,
    origin: DVec2,
    dir: DVec2,
    t0: f64,
    t1: f64,
    thickness: f64,
    core: f64,
    height: f64,
    bearing: bool,
    propped: bool,
}

fn masonry(w: &Wall) -> bool {
    w.curtain.is_none() && w.bulge.abs() < 1e-9 && w.length() > 0.1 && w.thickness > 0.04
}

/// Is `w` thick enough to be checked and to brace others: its core over
/// [`MasonryRules::unchecked_upto`].
fn checked(model: &Model, w: &Wall, rules: &MasonryRules) -> bool {
    core_thickness(model, w) > rules.unchecked_upto + 1e-6
}

fn runs(model: &Model, rules: &MasonryRules) -> Vec<Run> {
    let walls: Vec<(ElementId, &Wall)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Wall(w) if masonry(w) && checked(model, w, rules) => Some((e.id, w)),
            _ => None,
        })
        .collect();
    let mut out: Vec<Run> = vec![];
    for (id, w) in &walls {
        let mut d = w.direction();
        if d.x < -1e-9 || (d.x.abs() <= 1e-9 && d.y < 0.0) {
            d = -d;
        }
        let bearing = is_bearing(model, w, rules);
        let (a, b) = (w.start.dot(d), w.end.dot(d));
        let (t0, t1) = (a.min(b), a.max(b));
        let propped = !w.top.is_unconnected() || !w.attach_top.is_empty();
        let joined = out.iter_mut().find(|r| {
            r.level == w.level
                && r.bearing == bearing
                && r.dir.perp_dot(d).abs() < 1e-3
                && (w.start - r.origin).perp_dot(r.dir).abs() < 0.02
                && t0 <= r.t1 + 0.05
                && t1 >= r.t0 - 0.05
        });
        match joined {
            Some(r) => {
                r.walls.push(*id);
                r.t0 = r.t0.min(t0);
                r.t1 = r.t1.max(t1);
                r.thickness = r.thickness.max(w.thickness);
                r.core = r.core.max(core_thickness(model, w));
                r.height = r.height.max(w.height);
                r.propped &= propped;
            }
            None => out.push(Run {
                level: w.level,
                walls: vec![*id],
                origin: w.start - d * w.start.dot(d),
                dir: d,
                t0,
                t1,
                thickness: w.thickness,
                core: core_thickness(model, w),
                height: w.height,
                bearing,
                propped,
            }),
        }
    }
    // runs that grew into each other
    let mut merged: Vec<Run> = vec![];
    for r in out {
        match merged.iter_mut().find(|m| {
            m.level == r.level
                && m.bearing == r.bearing
                && m.dir.perp_dot(r.dir).abs() < 1e-3
                && (r.origin - m.origin).perp_dot(m.dir).abs() < 0.02
                && r.t0 <= m.t1 + 0.05
                && r.t1 >= m.t0 - 0.05
        }) {
            Some(m) => {
                m.walls.extend(r.walls);
                m.t0 = m.t0.min(r.t0);
                m.t1 = m.t1.max(r.t1);
                m.thickness = m.thickness.max(r.thickness);
                m.core = m.core.max(r.core);
                m.height = m.height.max(r.height);
                m.propped &= r.propped;
            }
            None => merged.push(r),
        }
    }
    merged
}

/// Where along `run` it is braced: by the walls meeting or crossing it
/// (thick and long enough) and by the columns standing in it, each with
/// half its width along the run.
fn braces(model: &Model, run: &Run, rules: &MasonryRules) -> Vec<(f64, f64)> {
    let mut out = vec![];
    let n = run.dir.perp();
    let line_off = run.origin.dot(n);
    let min_t = (0.3 * run.core).max(0.085);
    let min_len = run.height / 5.0;
    for e in model.iter() {
        match &e.kind {
            ElementKind::Wall(w)
                if w.level == run.level
                    && masonry(w)
                    && checked(model, w, rules)
                    && !run.walls.contains(&e.id) =>
            {
                let d = w.direction();
                // across it, not along
                if d.perp_dot(run.dir).abs() < 0.5 {
                    continue;
                }
                if core_thickness(model, w) + 1e-6 < min_t || w.length() + 1e-6 < min_len {
                    continue;
                }
                // where its line meets the run's
                let (s0, s1) = (w.start.dot(n) - line_off, w.end.dot(n) - line_off);
                if (s0 - s1).abs() < 1e-9 {
                    continue;
                }
                let k = s0 / (s0 - s1);
                let reach = (run.thickness / 2.0 + 0.05) / (s0 - s1).abs();
                if k < -reach || k > 1.0 + reach {
                    continue;
                }
                let p = w.start + (w.end - w.start) * k;
                let t = p.dot(run.dir);
                let half = w.thickness / 2.0 / d.perp_dot(run.dir).abs();
                if t < run.t0 - half - 0.02 || t > run.t1 + half + 0.02 {
                    continue;
                }
                out.push((t, half));
            }
            ElementKind::Column(c) if c.level == run.level => {
                let off = c.position.dot(n) - line_off;
                if off.abs() > run.thickness / 2.0 + 0.02 {
                    continue;
                }
                let t = c.position.dot(run.dir);
                if t < run.t0 - 0.05 || t > run.t1 + 0.05 {
                    continue;
                }
                let half = c
                    .outline()
                    .iter()
                    .map(|q| (q.dot(run.dir) - t).abs())
                    .fold(0.0, f64::max);
                out.push((t, half));
            }
            _ => {}
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

/// The stretches of masonry wall on `level` (every level with none) that
/// break the project's rules.
pub fn check_walls(model: &Model, level: Option<ElementId>) -> Vec<WallIssue> {
    let rules = rules(model);
    let mut out = vec![];
    for run in runs(model, &rules) {
        if level.is_some_and(|l| l != run.level) {
            continue;
        }
        let braces = braces(model, &run, &rules);
        // the stretches between, clear of the braces' widths
        let mut stops: Vec<(f64, f64)> = vec![(run.t0, 0.0)];
        stops.extend(
            braces
                .iter()
                .copied()
                .filter(|(t, _)| *t > run.t0 + 1e-6 && *t < run.t1 - 1e-6),
        );
        stops.push((run.t1, 0.0));
        let at = |t: f64| run.origin + run.dir * t;
        let (hi_short, hi_long, len_max) = partition_limits(run.core, run.propped);
        for w in stops.windows(2) {
            let (a, ha) = w[0];
            let (b, hb) = w[1];
            let clear = (b - a) - ha - hb;
            if clear <= 0.05 {
                continue;
            }
            let issue = |fault| WallIssue {
                level: run.level,
                walls: run.walls.clone(),
                a: at(a + ha),
                b: at(b - hb),
                bearing: run.bearing,
                fault,
            };
            if run.bearing {
                let limit = rules.bearing_limit();
                if clear > limit + 1e-6 {
                    out.push(issue(WallFault::TooLong {
                        length: clear,
                        limit,
                    }));
                }
                continue;
            }
            if clear > len_max + 1e-6 {
                out.push(issue(WallFault::TooLong {
                    length: clear,
                    limit: len_max,
                }));
            } else {
                let limit = if clear < 2.0 * run.height {
                    hi_short
                } else {
                    hi_long
                };
                if run.height > limit + 1e-6 {
                    out.push(issue(WallFault::TooHigh {
                        height: run.height,
                        limit,
                    }));
                }
            }
        }
    }
    out
}

/// What ties an RC column to the masonry wall it is built into.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct InWall {
    /// The wall: the column sits in its load-bearing core, as deep as the
    /// core, and slides only along it.
    pub wall: ElementId,
    /// XPS (Styrodur) on the exterior side of the column, inside the
    /// core, against the thermal bridge.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub xps: bool,
    /// Its thickness, m.
    #[serde(default = "default_xps")]
    pub xps_thickness: f64,
    /// Set on the XPS board itself: the column it insulates (the board
    /// is kept by [`settle`], never edited on its own).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub of: Option<ElementId>,
}

/// The office's usual XPS on a wall column, m.
pub const XPS_THICKNESS: f64 = 0.05;

fn default_xps() -> f64 {
    XPS_THICKNESS
}

impl InWall {
    pub fn new(wall: ElementId) -> Self {
        Self {
            wall,
            xps: false,
            xps_thickness: XPS_THICKNESS,
            of: None,
        }
    }

    /// The XPS thickness in use (0 without it), leaving the concrete at
    /// least 50 mm of a core `core` thick.
    pub fn insulation(&self, core: f64) -> f64 {
        if self.xps {
            self.xps_thickness.clamp(0.0, (core - 0.05).max(0.0))
        } else {
            0.0
        }
    }
}

/// The material the XPS boards are made of.
pub const XPS_MATERIAL: &str = "Insulation, XPS";

/// An RC column in the wall `wall_id` at `p` (clamped onto the wall),
/// filling the wall's load-bearing core across, [`MasonryRules::column_width`]
/// along it, up to the next level. It stays bound to the wall: see
/// [`fit_wall_column`].
pub fn wall_column(model: &Model, wall_id: ElementId, p: DVec2) -> Option<Element> {
    let ElementKind::Wall(w) = &model.get(wall_id)?.kind else {
        return None;
    };
    let rules = rules(model);
    let mut c = Column::new(
        w.level,
        p,
        Profile::Rectangle {
            width: rules.column_width,
            depth: 0.1,
        },
    );
    c.base_offset = w.base_offset;
    c.material = concrete(model);
    c.in_wall = Some(InWall::new(wall_id));
    let c = fit_wall_column(model, &c)?;
    Some(Element::new("ŽB stĺp v stene", ElementKind::Column(c)))
}

/// The column's width along its wall.
fn along(c: &Column, model: &Model) -> f64 {
    match c.profile {
        Profile::Rectangle { width, .. } if width > 0.01 => width,
        _ => rules(model).column_width,
    }
}

/// The column `c` put back where its wall allows: on the wall's line
/// (moved only along it, kept inside its length), as high as the wall
/// (its base and top levels and offsets), turned with it, as deep
/// as the core less the XPS, which takes the core's exterior side. `None`
/// when it is not in a wall or the wall is gone.
pub fn fit_wall_column(model: &Model, c: &Column) -> Option<Column> {
    let iw = c.in_wall.as_ref()?;
    if iw.of.is_some() {
        return None;
    }
    let ElementKind::Wall(w) = &model.get(iw.wall)?.kind else {
        return None;
    };
    if w.is_arc() || w.length() < 1e-6 {
        return None;
    }
    let d = w.direction();
    let n = d.perp();
    let (ext, int) = crate::walls::core_span(model, w);
    let core = (int - ext).abs();
    let s = if int >= ext { 1.0 } else { -1.0 };
    let ins = iw.insulation(core);
    let width = along(c, model).min(w.length());
    let half = width / 2.0;
    let u = (c.position - w.start)
        .dot(d)
        .clamp(half, (w.length() - half).max(half));
    let inner = ext + s * ins;
    let mut out = c.clone();
    out.position = w.start + d * u + n * ((inner + int) / 2.0);
    out.rotation = d.y.atan2(d.x).to_degrees();
    out.profile = Profile::Rectangle {
        width,
        depth: (core - ins).max(0.05),
    };
    out.section = None;
    // its heights are the wall's: base level and offset, top level and
    // offset, or the wall's own height
    out.level = w.level;
    out.base_offset = w.base_offset;
    out.top = w.top;
    out.height = w.top.is_unconnected().then_some(w.height);
    Some(out)
}

/// The XPS board on the exterior side of the wall column `col` (fitted),
/// `id` being the column's: `None` without XPS.
pub fn wall_column_xps(
    model: &Model,
    id: ElementId,
    col: &Column,
    material: Option<ElementId>,
) -> Option<Column> {
    let iw = col.in_wall.as_ref()?;
    let ElementKind::Wall(w) = &model.get(iw.wall)?.kind else {
        return None;
    };
    let (ext, int) = crate::walls::core_span(model, w);
    let s = if int >= ext { 1.0 } else { -1.0 };
    let ins = iw.insulation((int - ext).abs());
    if ins < 1e-4 {
        return None;
    }
    let d = w.direction();
    let n = d.perp();
    let u = (col.position - w.start).dot(d);
    let mut x = col.clone();
    x.position = w.start + d * u + n * (ext + s * ins / 2.0);
    x.profile = Profile::Rectangle {
        width: along(col, model),
        depth: ins,
    };
    x.material = material;
    x.in_wall = Some(InWall {
        of: Some(id),
        xps: false,
        ..iw.clone()
    });
    Some(x)
}

/// The wall columns kept to their walls after `tx`, their XPS boards
/// added, moved or taken away to match.
fn settle_columns(model: &Model, tx: &Transaction, mut out: Transaction) -> Transaction {
    let touches = tx.ops.iter().any(|o| match o {
        Op::Insert(e) | Op::Remove(e) => {
            matches!(
                &e.kind,
                ElementKind::Wall(_) | ElementKind::Level(_) | ElementKind::Assembly(_)
            ) || matches!(&e.kind, ElementKind::Column(c) if c.in_wall.is_some())
        }
        Op::Replace { after, .. } => {
            matches!(
                &after.kind,
                ElementKind::Wall(_) | ElementKind::Level(_) | ElementKind::Assembly(_)
            ) || matches!(&after.kind, ElementKind::Column(c) if c.in_wall.is_some())
        }
    });
    if !touches {
        return out;
    }
    let mut boards: std::collections::HashMap<ElementId, Vec<&Element>> = Default::default();
    for e in model.iter() {
        if let ElementKind::Column(c) = &e.kind {
            if let Some(of) = c.in_wall.as_ref().and_then(|w| w.of) {
                boards.entry(of).or_default().push(e);
            }
        }
    }
    let mut material: Option<Option<ElementId>> = None;
    let mut seen = vec![];
    for e in model.iter() {
        let ElementKind::Column(c) = &e.kind else {
            continue;
        };
        if c.in_wall.as_ref().is_none_or(|w| w.of.is_some()) {
            continue;
        }
        seen.push(e.id);
        let Some(next) = fit_wall_column(model, c) else {
            continue;
        };
        if next != *c {
            let mut after = e.clone();
            after.kind = ElementKind::Column(next.clone());
            out = out.replace(e.clone(), after);
        }
        let has = boards.get(&e.id).cloned().unwrap_or_default();
        let mat = *material.get_or_insert_with(|| {
            let (id, new) = crate::library::find_or_new(model, XPS_MATERIAL)?;
            if let Some(m) = new {
                out = std::mem::replace(&mut out, Transaction::new("")).insert(m);
            }
            Some(id)
        });
        match wall_column_xps(model, e.id, &next, mat) {
            Some(mut want) => {
                if let Some(first) = has.first() {
                    if let ElementKind::Column(b) = &first.kind {
                        want.material = b.material.or(mat);
                        if *b != want {
                            let mut after = (*first).clone();
                            after.kind = ElementKind::Column(want);
                            out = out.replace((*first).clone(), after);
                        }
                    }
                    for extra in &has[1..] {
                        out = out.remove_cascade(model, extra.id);
                    }
                } else {
                    out = out.insert(Element::new("XPS pri ŽB stĺpe", ElementKind::Column(want)));
                }
            }
            None => {
                for b in has {
                    out = out.remove_cascade(model, b.id);
                }
            }
        }
    }
    // boards whose column is gone or no longer in a wall
    for (of, list) in &boards {
        if !seen.contains(of) {
            for b in list {
                out = out.remove_cascade(model, b.id);
            }
        }
    }
    out
}

/// The name of a ring beam, as the office writes it.
pub const RING_BEAM: &str = "ŽB stužujúci veniec";

/// An RC ring beam (stužujúci veniec) along the top of the wall
/// `wall_id`: as wide as the wall's core less its shuttering on the outer
/// side, [`MasonryRules::ring_depth`] high, its top at the wall's top.
/// It lies on the load-bearing core only, flush with its inner face,
/// with no insulation or render of its own: the wall's insulation runs
/// on over it (George 2026-11-24). Its ends stop in the cores of the
/// walls it meets (see [`ring_ends`]).
pub fn ring_beam(model: &Model, wall_id: ElementId) -> Option<Element> {
    let ElementKind::Wall(w) = &model.get(wall_id)?.kind else {
        return None;
    };
    if w.is_arc() || w.length() < 1e-6 {
        return None;
    }
    let rules = rules(model);
    let (ext, int) = crate::walls::core_span(model, w);
    let core = (int - ext).abs();
    let shutter = rules.ring_shutter.clamp(0.0, (core - 0.1).max(0.0));
    let width = (core - shutter).max(0.05);
    let mut b = Beam::new(w.level, w.start, w.end);
    b.width = width;
    b.depth = rules.ring_depth.max(0.05);
    b.material = concrete(model);
    let top = wall_top(model, w);
    let level_top = b.top(model) + b.top_offset;
    b.top_offset = level_top - top;
    let (lo, hi) = ring_ends(model, wall_id, w);
    place_on_core(model, w, &mut b, lo, hi);
    Some(Element::new(RING_BEAM, ElementKind::Beam(b)))
}

/// Lay `b` along the core of `w` from `lo` to `hi` (along the wall from
/// its start): no wider than the core, flush with its inner face, no
/// ends run past; insulation, if any, only on its exterior side, the two
/// filling the core.
fn place_on_core(model: &Model, w: &Wall, b: &mut Beam, lo: f64, hi: f64) {
    let (ext, int) = crate::walls::core_span(model, w);
    let core = (int - ext).abs();
    b.width = b.width.clamp(0.05_f64.min(core), core.max(0.05));
    // insulation only on the exterior side, inside the core (most often
    // XPS: George 2026-11-25); the wall's own insulation and render run
    // on over it, so none of its own
    let ext_side = ring_exterior(model, w, b);
    if let Some(wr) = &mut b.wrap {
        for (i, t) in wr.sides.iter_mut().enumerate() {
            if i != ext_side {
                *t = 0.0;
            }
        }
        wr.plaster = None;
        // VELOX boards outside (and inside): their own thicknesses
        crate::wrap::Formwork::lay(wr, ext_side);
    }
    if b.wrap.as_ref().is_some_and(|w| w.is_empty()) {
        b.wrap = None;
    }
    let mut t_int = 0.0;
    if let Some(w) = b.wrap.clone() {
        // the beam and its insulation (boards) fill the core, flush with
        // its faces
        if w.formwork.is_some() {
            t_int = w.through(1 - ext_side);
            b.width = (core - w.through(ext_side) - t_int).max(0.05);
        } else {
            let t = w.side(ext_side).min(core - 0.05).max(0.0);
            if let Some(wr) = &mut b.wrap {
                wr.sides[ext_side] = t;
            }
            b.width = (core - t).max(0.05);
        }
    }
    // the shuttering, if any, on the outer side
    let sign = if int >= ext { 1.0 } else { -1.0 };
    let mid = int - sign * (t_int + b.width / 2.0);
    let d = w.direction();
    let n = d.perp();
    let (s, e) = (w.start + d * lo + n * mid, w.start + d * hi + n * mid);
    // kept the way it was drawn
    if (b.end - b.start).dot(d) < 0.0 {
        b.start = e;
        b.end = s;
    } else {
        b.start = s;
        b.end = e;
    }
    b.extend = [0.0, 0.0];
    b.cuts = [None, None];
}

/// Which side of ring beam `b` on wall `w` faces the exterior: 0 its
/// left, 1 its right, looking from its start to its end (as
/// [`crate::wrap`] numbers a beam's sides).
fn ring_exterior(model: &Model, w: &Wall, b: &Beam) -> usize {
    let (ext, int) = crate::walls::core_span(model, w);
    let out = w.direction().perp() * (ext - int).signum();
    let left = (b.end - b.start).normalize_or_zero().perp();
    if left.dot(out) >= 0.0 {
        0
    } else {
        1
    }
}

/// The exterior side of a ring beam (see [`ring_exterior`]), when it
/// lies on a wall.
pub fn ring_exterior_side(model: &Model, b: &Beam) -> Option<usize> {
    let ElementKind::Wall(w) = &model.get(ring_wall(model, b)?)?.kind else {
        return None;
    };
    Some(ring_exterior(model, w, b))
}

/// The width of the masonry core of the wall ring beam `b` lies on.
pub fn ring_core_width(model: &Model, b: &Beam) -> Option<f64> {
    let ElementKind::Wall(w) = &model.get(ring_wall(model, b)?)?.kind else {
        return None;
    };
    let (ext, int) = crate::walls::core_span(model, w);
    Some((int - ext).abs())
}

/// The width of the masonry core of the wall lintel `b` lies in.
pub fn lintel_core_width(model: &Model, b: &Beam) -> Option<f64> {
    let o = b.lintel.as_ref()?.opening?;
    let ElementKind::Opening(o) = &model.get(o)?.kind else {
        return None;
    };
    let ElementKind::Wall(w) = &model.get(o.host)?.kind else {
        return None;
    };
    let (ext, int) = crate::walls::core_span(model, w);
    Some((int - ext).abs())
}

/// The width a ring beam on wall `w` has with no insulation of its own:
/// the core less the shuttering on its outer side.
pub fn ring_plain_width(model: &Model, b: &Beam) -> Option<f64> {
    let ElementKind::Wall(w) = &model.get(ring_wall(model, b)?)?.kind else {
        return None;
    };
    let (ext, int) = crate::walls::core_span(model, w);
    let core = (int - ext).abs();
    let shutter = rules(model).ring_shutter.clamp(0.0, (core - 0.1).max(0.0));
    Some((core - shutter).max(0.05))
}

/// Where a ring beam along the wall `id` ends, along it from its start:
/// at each end, in the core of the load-bearing wall it meets. At a
/// corner the one running more along x goes through to the other's
/// outer face, the other stops at its inner face; meeting a wall's side
/// (a T) it stops at that core's near face. An end meeting nothing stays
/// at the wall's end.
pub fn ring_ends(model: &Model, id: ElementId, w: &Wall) -> (f64, f64) {
    let rules = rules(model);
    let len = w.length();
    let d = w.direction();
    let (ext, int) = crate::walls::core_span(model, w);
    let c0 = w.start + d.perp() * ((ext + int) / 2.0);
    let mut ends = [0.0, len];
    let mut best = [f64::MAX, f64::MAX];
    for e in model.iter() {
        let ElementKind::Wall(o) = &e.kind else {
            continue;
        };
        if e.id == id
            || o.level != w.level
            || o.is_arc()
            || o.curtain.is_some()
            || o.length() < 1e-6
            || !is_bearing(model, o, &rules)
        {
            continue;
        }
        let od = o.direction();
        let cross = d.perp_dot(od);
        if cross.abs() < 0.5 {
            continue;
        }
        let (oe, oi) = crate::walls::core_span(model, o);
        let on = od.perp();
        // where the beam's centre line crosses a face of o's core
        let hit = |off: f64| {
            let p = o.start + on * off;
            // c0 + d u on the line p + od t: (c0 + d u - p) · on = 0
            let u = (p - c0).dot(on) / d.dot(on);
            let t = (c0 + d * u - o.start).dot(od);
            (u, t)
        };
        let ((ue, te), (ui, _)) = (hit(oe), hit(oi));
        let reach = o.thickness + 0.05;
        if te < -reach || te > o.length() + reach {
            continue;
        }
        let mid = (ue + ui) / 2.0;
        let corner = te < o.thickness || te > o.length() - o.thickness;
        // o's outer face is the one farther from w's middle
        let (near_lo, far_lo) = (ue.max(ui), ue.min(ui));
        let (near_hi, far_hi) = (ue.min(ui), ue.max(ui));
        let through = corner
            && (d.x.abs() > od.x.abs() + 1e-9
                || ((d.x.abs() - od.x.abs()).abs() <= 1e-9 && id < e.id));
        for (k, end) in [0.0, len].into_iter().enumerate() {
            let gap = (mid - end).abs();
            if gap > o.thickness / 2.0 + 0.05 || gap >= best[k] {
                continue;
            }
            best[k] = gap;
            ends[k] = match (k, through) {
                (0, true) => far_lo,
                (0, false) => near_lo,
                (_, true) => far_hi,
                (_, false) => near_hi,
            };
        }
    }
    (ends[0], ends[1])
}

/// The wall a ring beam lies on: parallel, its middle line in the core,
/// the most of it along the wall.
pub fn ring_wall(model: &Model, b: &Beam) -> Option<ElementId> {
    let bd = (b.end - b.start).normalize_or_zero();
    let blen = b.start.distance(b.end);
    if blen < 1e-6 {
        return None;
    }
    let mut best: Option<(f64, ElementId)> = None;
    for e in model.iter() {
        let ElementKind::Wall(w) = &e.kind else {
            continue;
        };
        if w.level != b.level || w.is_arc() || w.length() < 1e-6 {
            continue;
        }
        let d = w.direction();
        if bd.perp_dot(d).abs() > 0.02 {
            continue;
        }
        let n = d.perp();
        let (ext, int) = crate::walls::core_span(model, w);
        let (lo, hi) = (ext.min(int) - 0.01, ext.max(int) + 0.01);
        let off = ((b.start + b.end) / 2.0 - w.start).dot(n);
        // a wall a bit wider than the beam had it run past its core
        let half = (b.width / 2.0).min(w.thickness / 2.0);
        if off < lo - half || off > hi + half {
            continue;
        }
        let (u0, u1) = {
            let a = (b.start - w.start).dot(d);
            let c = (b.end - w.start).dot(d);
            (a.min(c), a.max(c))
        };
        let along = (u1.min(w.length()) - u0.max(0.0)).max(0.0);
        if along < blen * 0.5 {
            continue;
        }
        if best.is_none_or(|x| along > x.0) {
            best = Some((along, e.id));
        }
    }
    best.map(|x| x.1)
}

/// A ring beam `b` laid back on the core of its wall after an edit: no
/// wider than the core and inside it, its ends in the cores of the walls
/// it meets where it reached the wall's ends, nothing round it. `None`
/// when it lies on no wall.
pub fn fitted_ring(model: &Model, b: &Beam) -> Option<Beam> {
    let wid = ring_wall(model, b)?;
    let ElementKind::Wall(w) = &model.get(wid)?.kind else {
        return None;
    };
    let d = w.direction();
    let (lo, hi) = ring_ends(model, wid, w);
    let (mut u0, mut u1) = {
        let a =
            (b.start - w.start).dot(d) - b.extend[0].max(0.0) * (b.end - b.start).dot(d).signum();
        let c = (b.end - w.start).dot(d) + b.extend[1].max(0.0) * (b.end - b.start).dot(d).signum();
        (a.min(c), a.max(c))
    };
    // an end at the wall's end goes to where the cores meet; one along
    // the wall stays, inside them
    let near = w.thickness.max(0.3);
    if u0 < near {
        u0 = lo;
    }
    if u1 > w.length() - near {
        u1 = hi;
    }
    let (u0, u1) = (u0.max(lo), u1.min(hi));
    if u1 - u0 < 0.05 {
        return None;
    }
    let mut nb = b.clone();
    place_on_core(model, w, &mut nb, u0, u1);
    // its top at the top of its wall, wherever that goes (George
    // 2026-11-24)
    nb.top_offset += nb.top(model) - wall_top(model, w);
    if (nb.top_offset - b.top_offset).abs() < 1e-9 {
        nb.top_offset = b.top_offset;
    }
    Some(nb)
}

/// Ring beams laid back on the load-bearing core of their walls, onto
/// `out` (see [`fitted_ring`]).
fn refit_rings(model: &Model, mut out: Transaction) -> Transaction {
    for e in model.iter() {
        let ElementKind::Beam(b) = &e.kind else {
            continue;
        };
        if e.name != RING_BEAM || b.lintel.is_some() || b.piece.is_some() {
            continue;
        }
        let Some(next) = fitted_ring(model, b) else {
            continue;
        };
        // cut where an RC lintel merged with it stands
        let pieces = ring_wall(model, &next)
            .and_then(|wid| match &model.get(wid)?.kind {
                ElementKind::Wall(w) => Some(cut_ring(&next, w, &ring_gaps(model, wid, w))),
                _ => None,
            })
            .unwrap_or_else(|| vec![next.clone()]);
        if pieces.len() == 1 {
            if pieces[0] != *b {
                let mut after = e.clone();
                after.kind = ElementKind::Beam(pieces[0].clone());
                out = out.replace(e.clone(), after);
            }
            continue;
        }
        out.ops.push(crate::Op::Remove(e.clone()));
        for p in pieces {
            let n = Element::new(RING_BEAM, ElementKind::Beam(p));
            out = out.insert(n);
        }
    }
    out
}

/// Ring beam `b` on wall `w` less the stretches `gaps` (along the wall):
/// the pieces left, itself when nothing is taken out.
fn cut_ring(b: &Beam, w: &Wall, gaps: &[(f64, f64)]) -> Vec<Beam> {
    let d = w.direction();
    let (a, c) = ((b.start - w.start).dot(d), (b.end - w.start).dot(d));
    let mut runs = vec![(a.min(c), a.max(c))];
    for &(g0, g1) in gaps {
        let mut next = vec![];
        for (r0, r1) in runs {
            if g1 <= r0 + 1e-6 || g0 >= r1 - 1e-6 {
                next.push((r0, r1));
                continue;
            }
            if g0 - r0 > 0.05 {
                next.push((r0, g0));
            }
            if r1 - g1 > 0.05 {
                next.push((g1, r1));
            }
        }
        runs = next;
    }
    if runs.len() == 1 && (runs[0].0 - a.min(c)).abs() < 1e-9 && (runs[0].1 - a.max(c)).abs() < 1e-9
    {
        return vec![b.clone()];
    }
    let n = (b.start - w.start).dot(d.perp());
    let forward = c >= a;
    runs.into_iter()
        .map(|(r0, r1)| {
            let mut p = b.clone();
            let (s, e) = (
                w.start + d * r0 + d.perp() * n,
                w.start + d * r1 + d.perp() * n,
            );
            if forward {
                p.start = s;
                p.end = e;
            } else {
                p.start = e;
                p.end = s;
            }
            p
        })
        .collect()
}

/// Ring beams of a project from before they kept to the core (George
/// 2026-11-24): laid back on it, their insulation and render taken off.
pub fn mend_ring_beams(model: &Model) -> Option<Transaction> {
    let tx = refit_rings(model, Transaction::new("Ring beams on the core"));
    (!tx.ops.is_empty()).then_some(tx)
}

/// The height of a wall's top: its level, base offset and height (kept
/// in step with a top tied to a level).
fn wall_top(model: &Model, w: &Wall) -> f64 {
    model.level(w.level).map_or(0.0, |l| l.elevation) + w.base_offset + w.height
}

/// The ring beams still missing on the load-bearing walls of `level`
/// (or every level): one along each wall that has none at its top.
pub fn ring_beams(model: &Model, level: Option<ElementId>) -> Vec<Element> {
    let rules = rules(model);
    let have: Vec<&Beam> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Beam(b) if e.name == RING_BEAM => Some(b),
            _ => None,
        })
        .collect();
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Wall(w) = &e.kind else {
            continue;
        };
        if level.is_some_and(|l| l != w.level) || !is_bearing(model, w, &rules) {
            continue;
        }
        // one already on it, or its pieces either side of an RC lintel
        if ring_depth_on(model, e.id).is_some() {
            continue;
        }
        let Some(r) = ring_beam(model, e.id) else {
            continue;
        };
        let ElementKind::Beam(nb) = &r.kind else {
            continue;
        };
        let same = |b: &&Beam| {
            b.level == nb.level
                && ((b.start.distance(nb.start) < 0.02 && b.end.distance(nb.end) < 0.02)
                    || (b.start.distance(nb.end) < 0.02 && b.end.distance(nb.start) < 0.02))
        };
        if !have.iter().any(same) {
            out.push(r);
        }
    }
    out
}

/// The project's reinforced concrete, by its name.
pub fn concrete(model: &Model) -> Option<ElementId> {
    model.iter().find_map(|e| match &e.kind {
        ElementKind::Material(_) if e.name.to_lowercase().contains("železobetón") => Some(e.id),
        _ => None,
    })
}

/// The columns that break the stretch of `issue` into pieces no longer
/// than its limit, evenly, each moved off the doors and windows of its
/// walls onto the nearest pier.
pub fn columns_for(model: &Model, issue: &WallIssue) -> Vec<Element> {
    let WallFault::TooLong { length, limit } = issue.fault else {
        return vec![];
    };
    let rules = rules(model);
    let pieces = (length / limit).ceil().max(1.0) as usize;
    let dir = (issue.b - issue.a).normalize_or_zero();
    let half = rules.column_width / 2.0;
    // the openings along it, as stretches along `dir` from `a`
    let mut gaps: Vec<(f64, f64)> = vec![];
    for wid in &issue.walls {
        let Some(ElementKind::Wall(w)) = model.get(*wid).map(|e| &e.kind) else {
            continue;
        };
        for dep in model.dependents(*wid) {
            if let Some(ElementKind::Opening(o)) = model.get(dep).map(|e| &e.kind) {
                let a = (w.start + w.direction() * o.offset - issue.a).dot(dir);
                let b = (w.start + w.direction() * (o.offset + o.width) - issue.a).dot(dir);
                gaps.push((a.min(b) - half - 0.05, a.max(b) + half + 0.05));
            }
        }
    }
    let mut out = vec![];
    for k in 1..pieces {
        let mut t = length * k as f64 / pieces as f64;
        if let Some((a, b)) = gaps.iter().find(|(a, b)| t > *a && t < *b) {
            t = if t - a < b - t { *a } else { *b };
        }
        let p = issue.a + dir * t;
        // the wall it falls in
        let host = issue.walls.iter().copied().find(|id| {
            matches!(model.get(*id).map(|e| &e.kind), Some(ElementKind::Wall(w))
                if { let u = (p - w.start).dot(w.direction()); u >= -1e-6 && u <= w.length() + 1e-6 })
        });
        if let Some(c) = host.and_then(|h| wall_column(model, h, p)) {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Document, Level, OpeningKind};

    fn house() -> (Document, ElementId, ElementId) {
        let mut doc = Document::new();
        let lv = Element::new(
            "1NP",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let l = lv.id;
        let mut w = Wall::new(l, DVec2::ZERO, DVec2::new(9.0, 0.0), 0.3, 2.8);
        w.base_offset = 0.0;
        let we = Element::new("Wall", ElementKind::Wall(w));
        let wid = we.id;
        doc.commit(Transaction::new("t").insert(lv).insert(we))
            .unwrap();
        (doc, l, wid)
    }

    fn opening(doc: &mut Document, wall: ElementId, offset: f64, width: f64) -> ElementId {
        let o = Opening {
            host: wall,
            kind: OpeningKind::Window,
            offset,
            sill: 0.9,
            width,
            height: 1.5,
            material: None,
            frame: Default::default(),
            flip_hand: false,
            flip_facing: false,
            no_outer_sill: false,
            no_inner_sill: false,
            threshold: false,
            threshold_material: None,
            penetration: None,
            setback: None,
            reveal: None,
            division: None,
            door: None,
            texts: None,
            performance: None,
        };
        let e = Element::new("Okno", ElementKind::Opening(o));
        let id = e.id;
        doc.commit(Transaction::new("o").insert(e)).unwrap();
        id
    }

    #[test]
    fn kp_xl_spans_three_to_six_metres_and_its_capacity_interpolates() {
        // a 4,1 m opening in a 300 wall: KP XL 30 - 475 (clear 4,25)
        assert_eq!(fit(Family::KpXl, 0.3, 4.1).unwrap().id, "kp-xl-30-475");
        // in a 380 wall the 30 series serves up to 5 m, then the 38
        assert_eq!(fit(Family::KpXl, 0.38, 4.1).unwrap().id, "kp-xl-30-475");
        assert_eq!(fit(Family::KpXl, 0.38, 5.4).unwrap().id, "kp-xl-38-600");
        assert!(fit(Family::KpXl, 0.3, 5.4).is_none());
        assert!(fit(Family::KpXl, 0.38, 6.1).is_none());
        let p = product("kp-xl-30-450").unwrap();
        assert_eq!((p.length, p.bearing, p.mass), (4.5, 0.25, 335.4));
        let lo = kp_xl_capacity("kp-xl-30-450", 0.21).unwrap();
        let hi = kp_xl_capacity("kp-xl-30-450", 0.25).unwrap();
        let mid = kp_xl_capacity("kp-xl-30-450", 0.23).unwrap();
        assert_eq!(lo[2], 28.68);
        assert_eq!(hi[2], 31.98);
        assert!((mid[2] - (28.68 + 31.98) / 2.0).abs() < 1e-9);
        assert_eq!(kp_xl_capacity("kp-xl-30-450", 0.4).unwrap(), hi);
    }

    #[test]
    fn the_makers_tables_are_kept() {
        // PORFIX NP: bearing 300 mm, the clear opening 600 mm shorter
        for p in PRODUCTS.iter().filter(|p| p.family == Family::PorfixNp) {
            assert!((p.length - p.max_span - 0.6).abs() < 1e-9, "{}", p.name);
        }
        // Porotherm KP 7: the clear opening is the length less two bearings
        for p in PRODUCTS.iter().filter(|p| p.family == Family::Kp7) {
            assert!(
                (p.length - 2.0 * p.bearing - p.max_span).abs() < 1e-9,
                "{}",
                p.name
            );
        }
        assert_eq!(
            fit(Family::PorfixNp, 0.125, 1.0).unwrap().name,
            "NP 1800/125"
        );
        assert_eq!(fit(Family::Kp7, 0.07, 1.25).unwrap().name, "KP 7 – 150");
        assert!(fit(Family::PorfixNp, 0.1, 2.2).is_none());
        let ids: std::collections::HashSet<_> = PRODUCTS.iter().map(|p| p.id).collect();
        assert_eq!(ids.len(), PRODUCTS.len());
    }

    #[test]
    fn lintels_lie_over_their_opening_and_follow_it() {
        let (mut doc, _, wall) = house();
        let o = opening(&mut doc, wall, 2.0, 1.2);
        let model = doc.model();
        let ElementKind::Wall(w) = &model.get(wall).unwrap().kind else {
            unreachable!()
        };
        let set = usual_set(model, w, Family::PorfixNp);
        assert_eq!((set.width, set.count), (0.1, 3));
        let pieces = lintels_for(model, o, set).unwrap();
        assert_eq!(pieces.len(), 3);
        let mut tx = Transaction::new("lintels");
        for p in pieces {
            tx = tx.insert(p);
        }
        doc.commit(tx).unwrap();
        let ids = lintels_over(doc.model(), o);
        assert_eq!(label(doc.model(), &ids), "3× NP 1800/100");
        let beam = |doc: &Document, id| match &doc.model().get(id).unwrap().kind {
            ElementKind::Beam(b) => b.clone(),
            _ => unreachable!(),
        };
        let b = beam(&doc, ids[0]);
        // centred over it, its underside on the head (0,9 + 1,5)
        assert!(((b.start.x + b.end.x) / 2.0 - 2.6).abs() < 1e-9);
        assert!((b.top(doc.model()) - b.depth - 2.4).abs() < 1e-9);
        assert!(lintel_issue(doc.model(), &b).is_none());
        // the window moves: its lintels go with it
        doc.edit(o, "move", |e| {
            if let ElementKind::Opening(op) = &mut e.kind {
                op.offset = 5.0;
                op.sill = 1.0;
            }
        })
        .unwrap();
        let b = beam(&doc, ids[0]);
        assert!(((b.start.x + b.end.x) / 2.0 - 5.6).abs() < 1e-9);
        assert!((b.top(doc.model()) - b.depth - 2.5).abs() < 1e-9);
        // widened past what it spans: said so
        doc.edit(o, "widen", |e| {
            if let ElementKind::Opening(op) = &mut e.kind {
                op.width = 1.5;
            }
        })
        .unwrap();
        assert!(lintel_issue(doc.model(), &beam(&doc, ids[0])).is_some());
        // the window deleted: its lintels go with it
        let tx = Transaction::new("del").remove_cascade(doc.model(), o);
        doc.commit(tx).unwrap();
        assert!(lintels_over(doc.model(), o).is_empty());
        assert!(!doc.model().iter().any(is_lintel));
    }

    #[test]
    fn a_long_wall_needs_a_cross_wall_or_a_column() {
        let (mut doc, l, _) = house();
        // 9 m of load-bearing wall with nothing across it
        let issues = check_walls(doc.model(), Some(l));
        assert_eq!(issues.len(), 1);
        assert!(issues[0].bearing);
        assert!(
            matches!(issues[0].fault, WallFault::TooLong { length, .. } if (length - 9.0).abs() < 1e-9)
        );
        assert!(issues[0].message().contains("9,00 m"));
        // a short stub across does not brace it
        let stub = Wall::new(l, DVec2::new(4.5, 0.15), DVec2::new(4.5, 0.4), 0.3, 2.8);
        doc.commit(Transaction::new("stub").insert(Element::new("w", ElementKind::Wall(stub))))
            .unwrap();
        assert_eq!(check_walls(doc.model(), Some(l)).len(), 1);
        // the columns it proposes break it into pieces short enough
        let issue = check_walls(doc.model(), Some(l)).remove(0);
        let cols = columns_for(doc.model(), &issue);
        assert_eq!(cols.len(), 1);
        let mut tx = Transaction::new("cols");
        for c in cols {
            tx = tx.insert(c);
        }
        doc.commit(tx).unwrap();
        assert!(check_walls(doc.model(), Some(l)).is_empty());
        // a seismic area allows 5 m: 4.5 m pieces still pass
        let pid = {
            let e = Element::new(
                "Projekt",
                ElementKind::Project(crate::ProjectInfo {
                    masonry: Box::new(MasonryRules {
                        seismic: true,
                        bearing_max: 4.0,
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
            );
            let id = e.id;
            doc.commit(Transaction::new("p").insert(e)).unwrap();
            id
        };
        assert_eq!(rules(doc.model()).bearing_limit(), 4.0);
        assert_eq!(check_walls(doc.model(), Some(l)).len(), 2);
        let _ = pid;
    }

    #[test]
    fn a_wall_across_braces_it_and_a_column_goes_beside_a_window() {
        let (mut doc, l, _) = house();
        let cross = Wall::new(l, DVec2::new(4.5, 0.15), DVec2::new(4.5, 4.0), 0.25, 2.8);
        doc.commit(Transaction::new("x").insert(Element::new("w", ElementKind::Wall(cross))))
            .unwrap();
        assert!(check_walls(doc.model(), Some(l)).is_empty());
        // a column in the middle of a window moves off it
        let (mut doc, l, wall2) = house();
        opening(&mut doc, wall2, 4.0, 1.0);
        let issue = check_walls(doc.model(), Some(l)).remove(0);
        let cols = columns_for(doc.model(), &issue);
        let ElementKind::Column(c) = &cols[0].kind else {
            unreachable!()
        };
        let x = c.position.x;
        assert!(
            !(3.85..=5.15).contains(&x),
            "column at {x} stands in the window"
        );
        assert!((c.position.y).abs() < 1e-9);
    }

    #[test]
    fn a_wall_column_keeps_to_the_core_and_moves_only_along_the_wall() {
        let (mut doc, _, wall) = house();
        let c = wall_column(doc.model(), wall, DVec2::new(3.0, 0.4)).unwrap();
        let id = c.id;
        doc.commit(Transaction::new("c").insert(c)).unwrap();
        let col = |doc: &Document| match &doc.model().get(id).unwrap().kind {
            ElementKind::Column(c) => c.clone(),
            _ => unreachable!(),
        };
        let c = col(&doc);
        assert!((c.position - DVec2::new(3.0, 0.0)).length() < 1e-9);
        assert_eq!(c.height, Some(2.8));
        // the wall raised and lifted: the column goes with it
        doc.edit(wall, "taller", |e| {
            if let ElementKind::Wall(w) = &mut e.kind {
                w.height = 3.1;
                w.base_offset = 0.2;
            }
        })
        .unwrap();
        let c = col(&doc);
        assert_eq!((c.height, c.base_offset), (Some(3.1), 0.2));
        // its own height edited: back to the wall's
        doc.edit(id, "h", |e| {
            if let ElementKind::Column(c) = &mut e.kind {
                c.height = Some(1.0);
            }
        })
        .unwrap();
        assert_eq!(col(&doc).height, Some(3.1));
        let Profile::Rectangle { width, depth } = c.profile else {
            panic!()
        };
        assert!((width - 0.25).abs() < 1e-9 && (depth - 0.3).abs() < 1e-9);
        // dragged off the wall and past its end: back on its line, inside it
        doc.edit(id, "move", |e| {
            if let ElementKind::Column(c) = &mut e.kind {
                c.position = DVec2::new(12.0, 1.0);
                c.rotation = 30.0;
            }
        })
        .unwrap();
        let c = col(&doc);
        assert!(
            (c.position - DVec2::new(8.875, 0.0)).length() < 1e-9,
            "{:?}",
            c.position
        );
        assert!(c.rotation.abs() < 1e-9);
        // XPS on: the board on the exterior side, the concrete behind it
        doc.edit(id, "xps", |e| {
            if let ElementKind::Column(c) = &mut e.kind {
                c.in_wall.as_mut().unwrap().xps = true;
            }
        })
        .unwrap();
        let c = col(&doc);
        let boards: Vec<Column> = doc
            .model()
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Column(b) if b.in_wall.as_ref().and_then(|w| w.of) == Some(id) => {
                    Some(b.clone())
                }
                _ => None,
            })
            .collect();
        assert_eq!(boards.len(), 1);
        let b = &boards[0];
        let Profile::Rectangle { depth, .. } = c.profile else {
            panic!()
        };
        assert!((depth - 0.25).abs() < 1e-9);
        let Profile::Rectangle { depth: bd, .. } = b.profile else {
            panic!()
        };
        assert!((bd - XPS_THICKNESS).abs() < 1e-9);
        // the exterior is on the right of start → end: y < 0 here
        assert!((b.position.y + 0.125).abs() < 1e-9, "{:?}", b.position);
        assert!((c.position.y - 0.025).abs() < 1e-9, "{:?}", c.position);
        assert!(b.material.is_some());
        // off again: the board goes
        doc.edit(id, "xps", |e| {
            if let ElementKind::Column(c) = &mut e.kind {
                c.in_wall.as_mut().unwrap().xps = false;
            }
        })
        .unwrap();
        assert!(!doc.model().iter().any(|e| matches!(&e.kind,
            ElementKind::Column(b) if b.in_wall.as_ref().is_some_and(|w| w.of.is_some()))));
    }

    #[test]
    fn thin_partitions_are_neither_checked_nor_bracing() {
        let (mut doc, l, _) = house();
        // a 150 mm partition across the long wall does not brace it
        let cross = Wall::new(l, DVec2::new(4.5, 0.15), DVec2::new(4.5, 4.0), 0.15, 2.8);
        // and a long 115 mm partition is not checked at all
        let long = Wall::new(l, DVec2::new(0.0, 6.0), DVec2::new(12.0, 6.0), 0.115, 2.8);
        doc.commit(
            Transaction::new("x")
                .insert(Element::new("w", ElementKind::Wall(cross)))
                .insert(Element::new("p", ElementKind::Wall(long))),
        )
        .unwrap();
        let issues = check_walls(doc.model(), Some(l));
        assert_eq!(issues.len(), 1);
        assert!(issues[0].bearing);
    }

    #[test]
    fn partitions_follow_porfix_table() {
        let (a, b, c) = partition_limits(0.1, true);
        assert_eq!((a, b, c), (2.58, 2.064, 7.22));
        let (_, _, c) = partition_limits(0.1, false);
        assert_eq!(c, 5.07);
        let (_, _, c) = partition_limits(0.1125, true);
        assert!((c - (7.22 + 9.04) / 2.0).abs() < 1e-9);
    }

    #[test]
    fn ring_beams_run_on_the_bearing_walls_at_their_tops_once() {
        let mut m = crate::starter::starter_house();
        let mut tx = Transaction::new("bearing");
        for e in m.iter() {
            if let ElementKind::Wall(w) = &e.kind {
                let mut n = e.clone();
                let mut w = w.clone();
                w.bearing = Some(true);
                n.kind = ElementKind::Wall(w);
                tx = tx.replace(e.clone(), n);
            }
        }
        tx.apply(&mut m).unwrap();
        let rings = ring_beams(&m, None);
        assert!(!rings.is_empty());
        let rules = rules(&m);
        for r in &rings {
            let ElementKind::Beam(b) = &r.kind else {
                panic!()
            };
            assert!((b.depth - rules.ring_depth).abs() < 1e-9);
            // its top at the top of a wall of its level
            let top = b.top(&m);
            assert!(
                m.iter().any(|e| matches!(&e.kind, ElementKind::Wall(w)
                    if w.level == b.level && (wall_top(&m, w) - top).abs() < 1e-6)),
                "{top}"
            );
        }
        let mut m2 = m.clone();
        let mut tx = Transaction::new("rings");
        for r in rings {
            tx = tx.insert(r);
        }
        tx.apply(&mut m2).unwrap();
        assert!(ring_beams(&m2, None).is_empty());
    }

    /// A ring beam lies on the load-bearing core only (George
    /// 2026-11-24): at a corner the beam along x runs through to the
    /// other wall's outer face, the other stops at its inner face, so
    /// nothing runs past the core; a beam made wider or given a wrap is
    /// laid back on the core, flush with its inner face, bare.
    #[test]
    fn ring_beams_keep_to_the_core_and_meet_at_the_corners() {
        let (mut doc, l, south) = house();
        // a 9 x 6 m box of 300 mm walls on their centre lines
        let mut tx = Transaction::new("walls");
        let mut ids = vec![south];
        for (a, b) in [
            (DVec2::new(9.0, 0.0), DVec2::new(9.0, 6.0)),
            (DVec2::new(9.0, 6.0), DVec2::new(0.0, 6.0)),
            (DVec2::new(0.0, 6.0), DVec2::ZERO),
        ] {
            let mut w = Wall::new(l, a, b, 0.3, 2.8);
            w.base_offset = 0.0;
            w.bearing = Some(true);
            let e = Element::new("Wall", ElementKind::Wall(w));
            ids.push(e.id);
            tx = tx.insert(e);
        }
        let mut se = doc.model().get(south).unwrap().clone();
        if let ElementKind::Wall(w) = &mut se.kind {
            w.bearing = Some(true);
        }
        tx = tx.replace(doc.model().get(south).unwrap().clone(), se);
        doc.commit(tx).unwrap();
        let m = doc.model();
        let beam = |id| match ring_beam(m, id).unwrap().kind {
            ElementKind::Beam(b) => b,
            _ => unreachable!(),
        };
        // along x: from outer face to outer face
        let b = beam(south);
        let xs = (b.start.x.min(b.end.x), b.start.x.max(b.end.x));
        assert!(
            (xs.0 + 0.15).abs() < 1e-9 && (xs.1 - 9.15).abs() < 1e-9,
            "{xs:?}"
        );
        assert_eq!(b.extend, [0.0, 0.0]);
        assert!(b.wrap.is_none());
        // along y: between the inner faces
        let b = beam(ids[1]);
        let ys = (b.start.y.min(b.end.y), b.start.y.max(b.end.y));
        assert!(
            (ys.0 - 0.15).abs() < 1e-9 && (ys.1 - 5.85).abs() < 1e-9,
            "{ys:?}"
        );
        assert!((b.start.x - 9.0).abs() < 1e-9 && (b.width - 0.3).abs() < 1e-9);
        // made 400 mm wide: back on the 300 mm core
        let mut wide = beam(south);
        wide.width = 0.4;
        wide.extend = [0.2, 0.2];
        let f = fitted_ring(m, &wide).unwrap();
        assert!((f.width - 0.3).abs() < 1e-9 && f.wrap.is_none());
        assert!(f.start.y.abs() < 1e-9 && f.extend == [0.0, 0.0]);
        // wrapped all round: only its exterior side (−y, its right) keeps
        // insulation, no render, the two filling the core (George
        // 2026-11-25: XPS on the outer side of a ring beam)
        let mut wrapped = beam(south);
        wrapped.wrap = Some(crate::wrap::new_wrap(m, [0.05; 4]).0);
        let f = fitted_ring(m, &wrapped).unwrap();
        let w = f.wrap.as_ref().unwrap();
        assert_eq!(w.sides, [0.0, 0.05, 0.0, 0.0]);
        assert!(w.plaster.is_none());
        assert!((f.width - 0.25).abs() < 1e-9, "{}", f.width);
        assert!((f.start.y - 0.025).abs() < 1e-9, "{}", f.start.y);
        let boards = crate::wrap::beam_boards(south, &f);
        assert_eq!(boards.len(), 1);
        let y = boards[0].start.y;
        assert!(
            (y + 0.125).abs() < 1e-9 && (boards[0].width - 0.05).abs() < 1e-9,
            "{y}"
        );
        // turned off again: as wide as the core less its shuttering
        assert!((ring_plain_width(m, &f).unwrap() - 0.3).abs() < 1e-9);
        // narrower: flush with the core's inner face (the room's side)
        let mut narrow = beam(south);
        narrow.width = 0.25;
        let f = fitted_ring(m, &narrow).unwrap();
        assert!((f.start.y - 0.025).abs() < 1e-9, "{}", f.start.y);
        // nothing to do for a beam already on its core
        let mut tx = Transaction::new("rings");
        for id in &ids {
            tx = tx.insert(ring_beam(m, *id).unwrap());
        }
        doc.commit(tx).unwrap();
        assert!(mend_ring_beams(doc.model()).is_none());
        // the wall made lower, or the beam moved down: its top goes back
        // to the wall's top (George 2026-11-24)
        let rid = doc
            .model()
            .iter()
            .find(|e| {
                matches!(&e.kind, ElementKind::Beam(b) if ring_wall(doc.model(), b) == Some(south))
            })
            .unwrap()
            .id;
        let we = doc.model().get(south).unwrap().clone();
        let mut lower = we.clone();
        if let ElementKind::Wall(w) = &mut lower.kind {
            w.height = 2.5;
        }
        doc.commit(Transaction::new("lower").replace(we, lower))
            .unwrap();
        let top = |doc: &crate::Document| match &doc.model().get(rid).unwrap().kind {
            ElementKind::Beam(b) => b.top(doc.model()),
            _ => unreachable!(),
        };
        let ElementKind::Wall(w) = &doc.model().get(south).unwrap().kind else {
            unreachable!()
        };
        let want = wall_top(doc.model(), w);
        assert!((top(&doc) - want).abs() < 1e-9, "{} {want}", top(&doc));
        let be = doc.model().get(rid).unwrap().clone();
        let mut down = be.clone();
        if let ElementKind::Beam(b) = &mut down.kind {
            b.top_offset += 0.3;
        }
        doc.commit(Transaction::new("down").replace(be, down))
            .unwrap();
        assert!((top(&doc) - want).abs() < 1e-9);
    }

    /// A monolithic RC lintel (George 2026-11-25): resting 250 mm each
    /// side (400 mm over 4 m), a twelfth of the span deep, at least 250 mm;
    /// across the whole core less the insulation laid into its formwork
    /// outside, its insulation under it on the opening's head.
    #[test]
    fn cast_lintels_fill_the_core_with_their_insulation() {
        assert_eq!(
            CastLintel::usual(1.2),
            CastLintel {
                bearing: 0.25,
                depth: 0.25
            }
        );
        assert_eq!(
            CastLintel::usual(3.6),
            CastLintel {
                bearing: 0.25,
                depth: 0.3
            }
        );
        assert_eq!(
            CastLintel::usual(4.5),
            CastLintel {
                bearing: 0.4,
                depth: 0.4
            }
        );
        let m = crate::starter::starter_house();
        let (oid, o) = m
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::Opening(o) => Some((e.id, o.clone())),
                _ => None,
            })
            .unwrap();
        let ElementKind::Wall(w) = &m.get(o.host).unwrap().kind else {
            panic!()
        };
        let (ext, int) = crate::walls::core_span(&m, w);
        let core = (int - ext).abs();
        let head = crate::walls::base_z(&m, w) + o.sill + o.height;
        let e = cast_lintel(&m, oid).unwrap();
        assert_eq!(e.name, RC_LINTEL);
        let ElementKind::Beam(b) = &e.kind else {
            panic!()
        };
        let c = CastLintel::usual(o.width);
        assert!((b.length() - (o.width + 2.0 * c.bearing)).abs() < 1e-9);
        assert!((b.width - core).abs() < 1e-9 && (b.depth - c.depth).abs() < 1e-9);
        assert!((b.top(&m) - b.depth - head).abs() < 1e-9);
        // XPS 50 outside, 30 under, and what it may not have
        let mut wrapped = b.clone();
        let ext_side = crate::wrap::exterior_side(&m, &e).unwrap();
        let mut sides = [0.1; 4];
        sides[ext_side] = 0.05;
        sides[2] = 0.03;
        wrapped.wrap = Some(crate::wrap::new_wrap(&m, sides).0);
        let f = fitted(&m, &wrapped).unwrap();
        let wr = f.wrap.as_ref().unwrap();
        assert!(wr.plaster.is_none());
        for i in 0..4 {
            let want = if i == ext_side {
                0.05
            } else if i == 2 {
                0.03
            } else {
                0.0
            };
            assert!((wr.side(i) - want).abs() < 1e-9, "{i} {}", wr.side(i));
        }
        assert!((f.width - (core - 0.05)).abs() < 1e-9);
        assert!((f.top(&m) - f.depth - 0.03 - head).abs() < 1e-9);
        // its inner face stays on the core's inner face
        let n = w.direction().perp();
        let inner = (f.start - w.start).dot(n) + (int - ext).signum() * f.width / 2.0;
        assert!((inner - int).abs() < 1e-9, "{inner} {int}");
        assert_eq!(crate::wrap::beam_boards(e.id, &f).len(), 2);
    }

    /// An RC lintel reaching into its wall's ring beam is one element up
    /// to the ring beam's top, the ring beam stopping at its ends (George
    /// 2026-11-25: "spoj do jedného prvku – železobetónový preklad").
    #[test]
    fn cast_lintels_merge_with_the_ring_beam() {
        let mut doc = crate::Document::from_model(crate::starter::starter_house());
        let m = doc.model();
        let (oid, host) = m
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::Opening(o) => Some((e.id, o.host)),
                _ => None,
            })
            .unwrap();
        let ring = ring_beam(m, host).unwrap();
        let rid = ring.id;
        let lintel = cast_lintel(m, oid).unwrap();
        let lid = lintel.id;
        doc.commit(Transaction::new("r").insert(ring)).unwrap();
        doc.commit(Transaction::new("l").insert(lintel)).unwrap();
        // an edit fits them
        let e = doc.model().get(lid).unwrap().clone();
        let mut e2 = e.clone();
        if let ElementKind::Beam(b) = &mut e2.kind {
            if let Some(c) = b.lintel.as_mut().and_then(|l| l.cast.as_mut()) {
                c.depth = 0.5;
            }
        }
        doc.commit(Transaction::new("deep").replace(e, e2)).unwrap();
        let m = doc.model();
        let ElementKind::Wall(w) = &m.get(host).unwrap().kind else {
            panic!()
        };
        let ElementKind::Beam(l) = &m.get(lid).unwrap().kind else {
            panic!()
        };
        assert!((l.top(m) - wall_top(m, w)).abs() < 1e-9);
        assert!(m.get(rid).is_none(), "the ring beam was cut");
        let d = w.direction();
        let (l0, l1) = {
            let (a, c) = ((l.start - w.start).dot(d), (l.end - w.start).dot(d));
            (a.min(c), a.max(c))
        };
        for e in m.iter() {
            let ElementKind::Beam(b) = &e.kind else {
                continue;
            };
            if e.name != RING_BEAM || ring_wall(m, b) != Some(host) {
                continue;
            }
            let (a, c) = ((b.start - w.start).dot(d), (b.end - w.start).dot(d));
            let (r0, r1) = (a.min(c), a.max(c));
            assert!(r1 <= l0 + 1e-6 || r0 >= l1 - 1e-6, "{r0} {r1} / {l0} {l1}");
        }
    }
}
