//! A rainwater soakaway of EKODREN DRENBLOK blocks (DB40-NEW 600 × 600 ×
//! 400, DB60-NEW 600 × 600 × 600, over 95 % void) in one to five layers,
//! with its filter and sedimentation shaft (FŠ 300, 400, 600 or 800) ahead
//! of it and the vent and safety overflow at its far end, as EKODREN's
//! sample solutions draw it; and its size worked out after DWA-A 138
//! (ATV-DVWK-A 138) for a block trench: the length that stores the
//! critical rain less what seeps away meanwhile.

use super::valvebox::rod;
use super::{cuboid, cylinder, look, prism, squircle, v2, v3, Build, Look};
use crate::objects::{CatalogueObject, Mount, ObjectCategory};
use crate::Hatch;
use glam::DVec2;
use serde::{Deserialize, Serialize};
use std::f64::consts::TAU;

/// A DRENBLOK block.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Block {
    /// DB40-NEW: 600 × 600 × 400, 144 l.
    Db40,
    /// DB60-NEW: 600 × 600 × 600, 216 l.
    #[default]
    Db60,
}

impl Block {
    pub const ALL: [Block; 2] = [Block::Db40, Block::Db60];

    pub fn name(self) -> &'static str {
        match self {
            Block::Db40 => "DRENBLOK DB40-NEW",
            Block::Db60 => "DRENBLOK DB60-NEW",
        }
    }

    /// Its height, m; every block is 0.6 × 0.6 in plan.
    pub fn height(self) -> f64 {
        match self {
            Block::Db40 => 0.4,
            Block::Db60 => 0.6,
        }
    }
}

/// The side of a block in plan, m.
pub const BLOCK: f64 = 0.6;
/// The share of a block's volume that stores water.
pub const VOID: f64 = 0.95;

/// The filter and sedimentation shaft ahead of the blocks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Filter {
    None,
    /// FŠ 300: up to 350 m² of roof, DN 110 / 160, 12 l/s.
    #[default]
    Fs300,
    /// FŠ 400: up to 500 m², DN 160 / 200, 17 l/s.
    Fs400,
    /// FŠ 600: up to 1000 m², DN 160 to 250, 34 l/s.
    Fs600,
    /// FŠ 800: up to 1500 m², DN 200 to 315.
    Fs800,
}

impl Filter {
    pub const ALL: [Filter; 5] = [
        Filter::None,
        Filter::Fs300,
        Filter::Fs400,
        Filter::Fs600,
        Filter::Fs800,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Filter::None => "No filter shaft",
            Filter::Fs300 => "EKODREN FŠ 300",
            Filter::Fs400 => "EKODREN FŠ 400",
            Filter::Fs600 => "EKODREN FŠ 600",
            Filter::Fs800 => "EKODREN FŠ 800",
        }
    }

    /// Its diameter and height, m, and the area it serves, m².
    pub fn size(self) -> Option<(f64, f64, f64)> {
        match self {
            Filter::None => None,
            Filter::Fs300 => Some((0.3, 1.7, 350.0)),
            Filter::Fs400 => Some((0.4, 1.8, 500.0)),
            Filter::Fs600 => Some((0.6, 2.0, 1000.0)),
            Filter::Fs800 => Some((0.8, 2.2, 1500.0)),
        }
    }

    /// The smallest shaft for `area` m² of roof.
    pub fn for_area(area: f64) -> Filter {
        Filter::ALL[1..]
            .iter()
            .copied()
            .find(|f| f.size().is_some_and(|s| s.2 >= area))
            .unwrap_or(Filter::Fs800)
    }
}

/// A soakaway as it is ordered.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SoakawayConfig {
    pub block: Block,
    /// Blocks along the trench and across it.
    pub along: u32,
    pub across: u32,
    /// Layers, 1 to 5.
    pub layers: u32,
    pub filter: Filter,
    /// Earth over the blocks, m (pedestrian 0.5, cars 0.7, SLW 30 0.8).
    pub cover: f64,
    /// The inlet pipe, DN.
    pub inlet_dn: u32,
}

impl Default for SoakawayConfig {
    fn default() -> Self {
        Self {
            block: Block::Db60,
            along: 4,
            across: 2,
            layers: 2,
            filter: Filter::Fs300,
            cover: 0.6,
            inlet_dn: 110,
        }
    }
}

impl SoakawayConfig {
    pub fn blocks(&self) -> u32 {
        self.along * self.across * self.layers
    }

    /// Length, width and height of the blocks, m.
    pub fn size(&self) -> (f64, f64, f64) {
        (
            self.along as f64 * BLOCK,
            self.across as f64 * BLOCK,
            self.layers as f64 * self.block.height(),
        )
    }

    /// The water it stores, m³.
    pub fn storage(&self) -> f64 {
        let (l, b, h) = self.size();
        l * b * h * VOID
    }

    /// The deepest its bottom may lie with this many layers, m (the
    /// maker's table for SLW 60; less earth over more layers).
    pub fn max_cover(&self) -> f64 {
        let n = self.layers.clamp(1, 5) as usize - 1;
        match self.block {
            Block::Db40 => [4.6, 4.4, 4.2, 4.0, 3.8][n],
            Block::Db60 => [4.4, 4.0, 3.7, 3.4, 3.0][n],
        }
    }

    pub fn code(&self) -> String {
        format!(
            "{} {} × {} × {} ({} pcs)",
            self.block.name(),
            self.along,
            self.across,
            self.layers,
            self.blocks()
        )
    }
}

/// One rain: its duration, min, and intensity, l/(s·ha), for the chosen
/// return period (from the local rain series).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rain {
    pub minutes: f64,
    pub intensity: f64,
}

/// What DWA-A 138 asks of a block trench `width` × `height` with `void`
/// storage, under a connected area `area_u` m² (each surface times its
/// runoff coefficient), in ground of permeability `kf` m/s, with the
/// surcharge factor `fz` (1.2 usually): for each rain the length it
/// needs, m. The longest is the critical rain's.
pub fn trench_length(
    area_u: f64,
    rain: Rain,
    width: f64,
    height: f64,
    void: f64,
    kf: f64,
    fz: f64,
) -> f64 {
    // L = A_u · 10⁻⁷ · r / ( b·h·s / (D·60·f_z) + (b + h/2) · k_f / 2 )
    let stored = width * height * void / (rain.minutes * 60.0 * fz);
    let seeps = (width + height / 2.0) * kf / 2.0;
    area_u * 1e-7 * rain.intensity / (stored + seeps)
}

/// The critical rain of `rains` and the length it needs, m.
pub fn critical(
    area_u: f64,
    rains: &[Rain],
    width: f64,
    height: f64,
    kf: f64,
    fz: f64,
) -> Option<(Rain, f64)> {
    rains
        .iter()
        .filter(|r| r.minutes > 0.0 && r.intensity > 0.0)
        .map(|r| (*r, trench_length(area_u, *r, width, height, VOID, kf, fz)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

/// How long a full trench takes to empty, h (DWA-A 138 wants a day at
/// most).
pub fn emptying_hours(width: f64, height: f64, kf: f64) -> f64 {
    let s = width * height * VOID;
    let q = (width + height / 2.0) * kf / 2.0;
    if q <= 0.0 {
        f64::INFINITY
    } else {
        s / q / 3600.0
    }
}

fn plastic(name: &str, rgb: [u8; 3]) -> Look {
    look(name, "plastic", Hatch::Empty, rgb, 0.6)
}

/// The soakaway as a catalogue object: the blocks wrapped in geotextile on
/// a gravel bed, the filter shaft ahead with its pipe in, the vent and
/// overflow at the far end. Its top is the blocks' top plus the cover: it
/// is placed so far below the level that the shafts' lids lie on the
/// ground.
pub fn soakaway(c: &SoakawayConfig) -> CatalogueObject {
    let mut b = Build::new();
    let (l, w, h) = c.size();
    let bed = 0.1;
    let top = bed + h + c.cover;
    let gap = 0.3;
    // the blocks from x = 0, centred across, behind y = 0
    let (y0, y1) = (-w - gap, -gap);
    let blue = plastic("PP, DRENBLOK blue", [40, 150, 210]);
    let gravel = look(
        "Gravel 8/16",
        "stone",
        Hatch::Diagonal,
        [200, 186, 150],
        0.9,
    );
    b.add(
        "Gravel bed",
        gravel,
        cuboid(
            v3(-gap, y0 - gap, 0.0),
            v3(l + gap, y1 + gap.min(0.3) - 1e-3, bed),
        ),
        false,
    );
    let lh = c.block.height();
    for k in 0..c.layers {
        let z = bed + k as f64 * lh;
        b.add(
            &format!("{} layer {}", c.block.name(), k + 1),
            blue.clone(),
            cuboid(v3(0.0, y0, z), v3(l, y1, z + lh - 0.005)),
            false,
        );
    }
    b.add(
        "Geotextile",
        plastic("Geotextile, white", [236, 236, 230]),
        cuboid(
            v3(-0.004, y0 - 0.004, bed + h),
            v3(l + 0.004, y1 + 0.004, bed + h + 0.004),
        ),
        false,
    );
    let ym = (y0 + y1) / 2.0;
    let pipe_z = bed + h - 0.15;
    let kg = plastic("PVC-U KG, orange", [196, 104, 48]);
    let r = c.inlet_dn as f64 / 2000.0;
    // the filter shaft ahead
    let mut x_in = 0.0;
    if let Some((d, sh, _)) = c.filter.size() {
        let cx = -1.2 - d / 2.0;
        let pe = plastic("PE, filter shaft", [60, 64, 70]);
        let n = 24;
        let base = top - sh;
        b.add(
            "Filter shaft bottom",
            pe.clone(),
            cylinder(
                v2(cx, ym),
                d / 2.0 + 0.02,
                base.max(0.0),
                base.max(0.0) + 0.03,
            ),
            false,
        );
        for i in 0..n {
            let (a0, a1) = (TAU * i as f64 / n as f64, TAU * (i + 1) as f64 / n as f64);
            let p = |rr: f64, a: f64| v2(cx, ym) + DVec2::from_angle(a) * rr;
            b.add(
                "Filter shaft",
                pe.clone(),
                prism(
                    vec![
                        p(d / 2.0, a0),
                        p(d / 2.0 + 0.02, a0),
                        p(d / 2.0 + 0.02, a1),
                        p(d / 2.0, a1),
                    ],
                    base.max(0.0) + 0.03,
                    top - 0.02,
                ),
                false,
            );
        }
        b.add(
            "Filter shaft lid",
            plastic("Lid, green", [46, 92, 52]),
            cylinder(v2(cx, ym), d / 2.0 + 0.03, top - 0.02, top + 0.02),
            false,
        );
        b.add(
            "Sludge basket",
            plastic("Basket, grey", [150, 150, 150]),
            cylinder(v2(cx, ym), d / 2.0 - 0.04, pipe_z - 0.25, pipe_z - 0.02),
            false,
        );
        b.add(
            "Pipe to the blocks",
            kg.clone(),
            rod(v3(cx + d / 2.0, ym, pipe_z), v3(0.0, ym, pipe_z), r),
            false,
        );
        b.add(
            "Inlet pipe",
            kg.clone(),
            rod(
                v3(cx - d / 2.0 - 0.6, ym, pipe_z + 0.05),
                v3(cx - d / 2.0, ym, pipe_z + 0.05),
                r,
            ),
            false,
        );
        x_in = cx - d / 2.0 - 0.6;
        b.outline(squircle(
            v2(cx, ym),
            d / 2.0 + 0.02,
            d / 2.0 + 0.02,
            2.0,
            32,
        ));
        b.line(v2(cx + d / 2.0, ym), v2(0.0, ym));
        b.line(v2(x_in, ym), v2(cx - d / 2.0, ym));
    } else {
        b.add(
            "Inlet pipe",
            kg.clone(),
            rod(v3(-0.6, ym, pipe_z), v3(0.0, ym, pipe_z), r),
            false,
        );
        x_in = x_in.min(-0.6);
        b.line(v2(-0.6, ym), v2(0.0, ym));
    }
    b.add(
        &format!("Connection K DN{}", c.inlet_dn.min(150)),
        super::metal("Brass", [196, 160, 92], 0.3),
        cylinder(v2(x_in, ym), 0.012, pipe_z, pipe_z + 0.02 + r),
        false,
    );
    // the vent and safety overflow at the far end, up to the ground
    let vx = l + 0.5;
    b.add(
        "Overflow pipe",
        kg.clone(),
        rod(v3(l, ym, pipe_z), v3(vx, ym, pipe_z), 0.055),
        false,
    );
    b.add(
        "Vent and overflow",
        kg,
        cylinder(v2(vx, ym), 0.055, pipe_z - 0.055, top + 0.25),
        false,
    );
    b.add(
        "Vent cap",
        plastic("Cap, black", [30, 30, 32]),
        cylinder(v2(vx, ym), 0.08, top + 0.25, top + 0.32),
        false,
    );
    // plan: the blocks, their grid, the vent
    b.outline(vec![v2(0.0, y0), v2(l, y0), v2(l, y1), v2(0.0, y1)]);
    for i in 1..c.along {
        let x = i as f64 * BLOCK;
        b.line(v2(x, y0), v2(x, y1));
    }
    for j in 1..c.across {
        let y = y0 + j as f64 * BLOCK;
        b.line(v2(0.0, y), v2(l, y));
    }
    b.outline(squircle(v2(vx, ym), 0.08, 0.08, 2.0, 16));
    b.line(v2(l, ym), v2(vx, ym));
    b.at(-top);
    let filter = match c.filter.size() {
        Some((_, _, a)) => format!("{} (up to {a:.0} m²)", c.filter.name()),
        None => c.filter.name().to_string(),
    };
    b.finish(
        &format!(
            "site-soakaway-{}-{}x{}x{}-{:?}",
            c.block.height() * 100.0,
            c.along,
            c.across,
            c.layers,
            c.filter
        )
        .to_lowercase(),
        &format!("Soakaway {}", c.code()),
        &format!(
            "Rainwater soakaway of {}: {:.1} × {:.1} × {:.1} m, {:.2} m³ stored (95 % void), wrapped in geotextile on a gravel bed, {:.1} m of earth over it; {} ahead with the inlet DN {}; vent and safety overflow DN 110 at the far end (EKODREN)",
            c.code(),
            l,
            w,
            h,
            c.storage(),
            c.cover,
            filter,
            c.inlet_dn
        ),
        ObjectCategory::Site,
        ("IfcDistributionChamberElement", "SUMP"),
        Mount::Floor,
    )
}

pub fn starters() -> Vec<CatalogueObject> {
    vec![soakaway(&SoakawayConfig::default())]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_soakaway_stores_its_blocks_and_is_sized_after_a138() {
        let c = SoakawayConfig::default();
        assert_eq!(c.blocks(), 16);
        // 16 DB60 blocks: 16 × 0.216 × 0.95
        assert!((c.storage() - 16.0 * 0.216 * 0.95).abs() < 1e-9);
        let o = soakaway(&c);
        assert!(o.object.offset < -1.5);
        // 150 m² of roof (Ψ 1), kf 1e-5, blocks 1.2 wide and 1.2 high:
        // a 15 min rain of 200 l/(s·ha) needs
        let rain = Rain {
            minutes: 15.0,
            intensity: 200.0,
        };
        let len = trench_length(150.0, rain, 1.2, 1.2, VOID, 1e-5, 1.2);
        let expect = 150.0 * 1e-7 * 200.0 / (1.2 * 1.2 * 0.95 / (900.0 * 1.2) + 1.8 * 1e-5 / 2.0);
        assert!(
            (len - expect).abs() < 1e-12 && len > 1.0 && len < 3.0,
            "{len}"
        );
        // a longer, lighter rain can need more when the ground is tight
        let rains = [
            rain,
            Rain {
                minutes: 120.0,
                intensity: 45.0,
            },
        ];
        let (crit, _) = critical(150.0, &rains, 1.2, 1.2, 1e-6, 1.2).unwrap();
        assert_eq!(crit.minutes, 120.0);
        assert!(emptying_hours(1.2, 1.2, 1e-5) < 24.0 * 3.0);
        assert_eq!(Filter::for_area(400.0), Filter::Fs400);
    }
}
