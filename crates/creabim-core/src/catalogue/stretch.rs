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

pub const STRETCHES: [Stretch; 2] = [
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
];

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
}
