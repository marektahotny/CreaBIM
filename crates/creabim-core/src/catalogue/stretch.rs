//! Objects drawn out to any length: a built-in wardrobe whose sliding
//! doors follow its length, and a kitchen island. A placed one keeps its
//! own shape at its length (see [`crate::objects::stretch`]).

use super::{
    cuboid, kitchen_front, laminate, metal, plinth, quartz, stainless, v2, v3, Build, Look,
};
use crate::objects::{CatalogueObject, Mount, ObjectCategory};

/// A built-in wardrobe's depth with its doors, and height.
pub const WARDROBE_DEPTH: f64 = 0.6;
pub const WARDROBE_HEIGHT: f64 = 2.4;
/// Sliding doors are this wide at most (over it, one more door); two at
/// least; neighbours overlap by [`DOOR_OVERLAP`].
pub const DOOR_MAX: f64 = 1.0;
pub const DOOR_OVERLAP: f64 = 0.03;

/// A stretchable object: its family's key (after `starter/`), the
/// lengths it can take and its usual one, m, and how it is made.
pub struct Stretch {
    pub family: &'static str,
    pub min: f64,
    pub max: f64,
    pub usual: f64,
    pub make: fn(f64) -> CatalogueObject,
}

pub const STRETCHES: [Stretch; 4] = [
    Stretch {
        family: "built-in-wardrobe",
        min: 1.0,
        max: 6.0,
        usual: 2.4,
        make: built_in_wardrobe,
    },
    Stretch {
        family: "kitchen-island",
        min: 1.2,
        max: 4.2,
        usual: 1.8,
        make: kitchen_island,
    },
    Stretch {
        family: "metod-base-cabinet",
        min: 0.15,
        max: 1.2,
        usual: 0.6,
        make: metod_base_cabinet,
    },
    Stretch {
        family: "metod-wall-cabinet",
        min: 0.15,
        max: 1.2,
        usual: 0.6,
        make: metod_wall_cabinet,
    },
];

/// Fronts on a cabinet: one up to 60 cm wide, two over it (as IKEA fits
/// two 40 cm doors on its 80 cm frame); 21 mm proud, 2 mm gaps.
fn metod_fronts(b: &mut Build, w: f64, y: f64, z0: f64, z1: f64, handle_low: bool) {
    let n = if w > 0.6 + 1e-6 { 2 } else { 1 };
    let fw = w / n as f64;
    let f = 0.021;
    for i in 0..n {
        let a = -w / 2.0 + i as f64 * fw;
        b.add(
            "Door",
            kitchen_front(),
            cuboid(
                v3(a + 0.002, y - f, z0 + 0.002),
                v3(a + fw - 0.002, y, z1 - 0.002),
            ),
            false,
        );
        // the handle on the stile away from the hinges, near the
        // worktop or the wall unit's bottom
        let hx = if n == 2 && i == 0 {
            a + fw - 0.04
        } else {
            a + 0.04
        };
        let (h0, h1) = if handle_low {
            (z0 + 0.03, z0 + 0.2)
        } else {
            (z1 - 0.2, z1 - 0.03)
        };
        b.add(
            "Handle",
            stainless(),
            cuboid(v3(hx - 0.006, y - f - 0.02, h0), v3(hx + 0.006, y - f, h1)),
            false,
        );
    }
    b.line(v2(-w / 2.0, y - f), v2(w / 2.0, y - f));
}

/// A made-to-measure base cabinet `len` wide (George 2026-11-23: "na
/// mieru"), deep and high as IKEA's METOD: frame 60 deep, 80 high on 8 cm legs (88 cm to the worktop,
/// as the corner base cabinet), the legs' plinth set back 10 cm; the
/// worktop is left to the kitchen's own.
pub fn metod_base_cabinet(len: f64) -> CatalogueObject {
    let w = len;
    let (d, legs) = (0.6, 0.08);
    let mut b = Build::new();
    b.add(
        "Plinth",
        plinth(),
        cuboid(v3(-w / 2.0, -d + 0.1, 0.0), v3(w / 2.0, 0.0, legs)),
        false,
    );
    b.add(
        "Carcass",
        laminate(),
        cuboid(v3(-w / 2.0, -d, legs), v3(w / 2.0, 0.0, legs + 0.8)),
        true,
    );
    metod_fronts(&mut b, w, -d, legs, legs + 0.8, false);
    let mm = (w * 1000.0).round();
    b.finish(
        "metod-base-cabinet",
        "Kitchen base cabinet",
        &format!(
            "Base cabinet made to measure {mm:.0} × 600 × 800 on 80 mm legs (880 to the worktop), depth and height as IKEA METOD"
        ),
        ObjectCategory::Kitchen,
        ("IfcFurniture", "USERDEFINED"),
        Mount::Wall,
    )
}

/// A made-to-measure wall cabinet `len` wide, deep and high as IKEA's
/// METOD: frame 37 deep, 80 high, its top level with the corner wall cabinet's (2170 mm).
pub fn metod_wall_cabinet(len: f64) -> CatalogueObject {
    let w = len;
    let (d, top) = (0.37, 2.17);
    let mut b = Build::new();
    b.add(
        "Carcass",
        laminate(),
        cuboid(v3(-w / 2.0, -d, top - 0.8), v3(w / 2.0, 0.0, top)),
        true,
    );
    metod_fronts(&mut b, w, -d, top - 0.8, top, true);
    let mm = (w * 1000.0).round();
    b.finish(
        "metod-wall-cabinet",
        "Kitchen wall cabinet",
        &format!(
            "Wall cabinet made to measure {mm:.0} × 370 × 800, its top at 2170 mm, depth and height as IKEA METOD"
        ),
        ObjectCategory::Kitchen,
        ("IfcFurniture", "USERDEFINED"),
        Mount::Wall,
    )
}

/// The stretch of the catalogue object `key` (`starter/built-in-wardrobe`
/// or a stretched one's).
pub fn stretch_of(key: &str) -> Option<&'static Stretch> {
    let k = key.rsplit('/').next().unwrap_or(key);
    STRETCHES.iter().find(|s| k == s.family)
}

/// How many sliding doors a wardrobe `len` long takes, and each one's width.
pub fn sliding_doors(len: f64) -> (usize, f64) {
    let n = ((len / DOOR_MAX - 1e-9).ceil() as usize).max(2);
    let w = (len + (n - 1) as f64 * DOOR_OVERLAP) / n as f64;
    (n, w)
}

fn mirror_grey() -> Look {
    super::look(
        "Sliding door, white gloss",
        "plastic",
        crate::Hatch::Empty,
        [236, 236, 232],
        0.2,
    )
}

/// A built-in wardrobe `len` long, 600 deep, 2400 high, behind sliding
/// doors on two tracks: as many as keep each 1000 wide or less.
pub fn built_in_wardrobe(len: f64) -> CatalogueObject {
    let (d, h) = (WARDROBE_DEPTH, WARDROBE_HEIGHT);
    let x0 = -len / 2.0;
    let mut b = Build::new();
    // the body, back to the wall, its front behind the tracks
    let body = -d + 0.08;
    b.add(
        "Carcass",
        laminate(),
        cuboid(v3(x0, body, 0.0), v3(-x0, 0.0, h)),
        false,
    );
    let rail = metal("Aluminium, anodised", [190, 192, 196], 0.35);
    b.add(
        "Track",
        rail.clone(),
        cuboid(v3(x0, -d, h - 0.06), v3(-x0, body, h)),
        false,
    );
    b.add(
        "Track",
        rail.clone(),
        cuboid(v3(x0, -d, 0.0), v3(-x0, body, 0.02)),
        false,
    );
    b.outline(vec![v2(x0, body), v2(-x0, body), v2(-x0, 0.0), v2(x0, 0.0)]);
    // the doors, front and back track in turn
    let (n, w) = sliding_doors(len);
    for i in 0..n {
        let a = x0 + i as f64 * (w - DOOR_OVERLAP);
        let (y0, y1) = if i % 2 == 0 {
            (-d + 0.035, -d + 0.06)
        } else {
            (-d, -d + 0.025)
        };
        b.add(
            "Sliding door",
            mirror_grey(),
            cuboid(v3(a, y0 + 0.003, 0.02), v3(a + w, y1 - 0.003, h - 0.06)),
            true,
        );
        b.add(
            "Door profile",
            rail.clone(),
            cuboid(v3(a, y0, 0.02), v3(a + 0.025, y1, h - 0.06)),
            false,
        );
    }
    // the hanging rail
    b.line(v2(x0 + 0.05, body / 2.0), v2(-x0 - 0.05, body / 2.0));
    let mm = (len * 1000.0).round();
    b.finish(
        "built-in-wardrobe",
        "Built-in wardrobe",
        &format!(
            "Built-in wardrobe {mm:.0} × 600, 2400 high, {n} sliding doors (draw it out to any length: the doors follow)"
        ),
        ObjectCategory::Bedroom,
        ("IfcFurniture", "USERDEFINED"),
        Mount::Wall,
    )
}

/// A kitchen island `len` long: base units 600 deep with their fronts,
/// a worktop 900 deep reaching 300 over the back for stools.
pub fn kitchen_island(len: f64) -> CatalogueObject {
    let x0 = -len / 2.0;
    let mut b = Build::new();
    b.add(
        "Plinth",
        plinth(),
        cuboid(v3(x0 + 0.05, -0.84, 0.0), v3(-x0 - 0.05, -0.32, 0.1)),
        false,
    );
    b.add(
        "Carcass",
        laminate(),
        cuboid(v3(x0, -0.88, 0.1), v3(-x0, -0.3, 0.87)),
        false,
    );
    let n = ((len / 0.6).round() as usize).max(2);
    let fw = len / n as f64;
    for i in 0..n {
        let a = x0 + i as f64 * fw;
        b.add(
            "Door",
            kitchen_front(),
            cuboid(v3(a + 0.002, -0.9, 0.102), v3(a + fw - 0.002, -0.88, 0.868)),
            false,
        );
        b.add(
            "Handle",
            stainless(),
            cuboid(
                v3(a + fw / 2.0 - 0.1, -0.92, 0.8),
                v3(a + fw / 2.0 + 0.1, -0.9, 0.812),
            ),
            false,
        );
    }
    b.add(
        "Worktop",
        quartz(),
        cuboid(v3(x0, -0.92, 0.87), v3(-x0, 0.0, 0.91)),
        true,
    );
    // where the units stop under the worktop
    b.line(v2(x0, -0.3), v2(-x0, -0.3));
    let mm = (len * 1000.0).round();
    b.finish(
        "kitchen-island",
        "Kitchen island",
        &format!(
            "Kitchen island {mm:.0} × 900, 910 high, base units with {n} doors and a breakfast bar (draw it out to any length)"
        ),
        ObjectCategory::Kitchen,
        ("IfcFurniture", "USERDEFINED"),
        Mount::Floor,
    )
}

pub fn starters() -> Vec<CatalogueObject> {
    STRETCHES.iter().map(|s| (s.make)(s.usual)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// George: a built-in wardrobe drawn out to any length, its sliding
    /// doors as many as its length takes.
    #[test]
    fn a_wardrobes_sliding_doors_follow_its_length() {
        assert_eq!(sliding_doors(1.2).0, 2);
        assert_eq!(sliding_doors(2.0).0, 2);
        assert_eq!(sliding_doors(2.4).0, 3);
        assert_eq!(sliding_doors(3.6).0, 4);
        for len in [1.0, 1.7, 2.4, 3.3, 6.0] {
            let (n, w) = sliding_doors(len);
            assert!(w <= DOOR_MAX + DOOR_OVERLAP && w > 0.5, "{len}: {n} × {w}");
            let t = built_in_wardrobe(len).object;
            assert!((t.size().x - len).abs() < 1e-9 && (t.size().y - 0.6).abs() < 1e-9);
            let doors = t.parts.iter().filter(|p| p.name == "Sliding door").count();
            assert_eq!(doors, n);
        }
        let island = kitchen_island(2.4).object;
        assert!((island.size().x - 2.4).abs() < 1e-9);
        assert!(stretch_of("starter/built-in-wardrobe").is_some());
        assert!(stretch_of("starter/wardrobe").is_none());
    }

    /// George 2026-11-23: kitchen base and wall cabinets made to measure,
    /// of any width.
    #[test]
    fn kitchen_cabinets_are_made_to_measure() {
        for (len, w, doors) in [
            (0.15, 0.15, 1),
            (0.45, 0.45, 1),
            (0.6, 0.6, 1),
            (0.75, 0.75, 2),
        ] {
            let base = metod_base_cabinet(len).object;
            assert!((base.size().x - w).abs() < 1e-9, "{len}");
            assert!((base.size().z - 0.88).abs() < 1e-9);
            assert_eq!(
                base.parts.iter().filter(|p| p.name == "Door").count(),
                doors
            );
            let wall = metod_wall_cabinet(len).object;
            assert!((wall.size().x - w).abs() < 1e-9);
            let (lo, hi) = wall.bounds();
            assert!((hi.z - 2.17).abs() < 1e-9 && (lo.z - 1.37).abs() < 1e-9);
        }
        assert!(stretch_of("starter/metod-base-cabinet").is_some());
    }
}
