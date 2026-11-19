//! A rainwater soakaway of EKODREN DRENBLOK blocks (DB40-NEW 600 × 600 ×
//! 400, DB60-NEW 600 × 600 × 600, over 95 % void) in one to five layers,
//! with its filter and sedimentation shaft (FŠ 300, 400, 600 or 800) ahead
//! of it and the vent and safety overflow at its far end, as EKODREN's
//! sample solutions draw it; and its size worked out after DWA-A 138
//! (ATV-DVWK-A 138) for a block trench: the length that stores the
//! critical rain less what seeps away meanwhile.
//!
//! The same blocks wrapped in a welded HDPE / PVC foil make a retention
//! tank (EKODREN RN, "opláštený hydroizoláciou"): nothing seeps away, a
//! throttled outlet at the bottom of the far end lets the water out to a
//! recipient (a sewer or a stream) at the flow the authority allows, and
//! the vent and safety overflow stay. Its volume is worked out with
//! DWA-A 117's simple method: the most any rain brings in more than the
//! outlet lets out.

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
    /// Wrapped in foil with a throttled outlet: a retention tank, not a
    /// soakaway.
    #[serde(default)]
    pub retention: bool,
    /// The retention tank's outflow to the recipient, l/s.
    #[serde(default = "default_outflow")]
    pub outflow: f64,
    /// Its outlet pipe, DN.
    #[serde(default = "default_outlet")]
    pub outlet_dn: u32,
}

fn default_outflow() -> f64 {
    1.0
}

fn default_outlet() -> u32 {
    160
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
            retention: false,
            outflow: default_outflow(),
            outlet_dn: default_outlet(),
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
            Block::Db40 => [4.6, 4.3, 4.2, 3.9, 3.7][n],
            Block::Db60 => [4.4, 4.0, 3.7, 3.4, 3.0][n],
        }
    }

    /// EKODREN's retention tank RN20: 8 × 6 DB60-NEW in two layers,
    /// 4.8 × 3.6 × 1.2 m, 20 736 l, behind a filter shaft (FDN 400), its
    /// outlet DN 160 to the recipient.
    pub fn rn20() -> Self {
        Self {
            block: Block::Db60,
            along: 8,
            across: 6,
            layers: 2,
            filter: Filter::Fs400,
            cover: 0.6,
            inlet_dn: 160,
            retention: true,
            outflow: default_outflow(),
            outlet_dn: 160,
        }
    }

    /// The blocks' gross volume, m³ (EKODREN names its tanks by it).
    pub fn gross(&self) -> f64 {
        let (l, b, h) = self.size();
        l * b * h
    }

    pub fn code(&self) -> String {
        if self.retention {
            return format!(
                "EKODREN RN{}: {} {} × {} × {} ({} pcs)",
                // its nominal volume: RN20 holds 20 736 l
                self.gross().floor().max(1.0),
                self.block.name(),
                self.along,
                self.across,
                self.layers,
                self.blocks()
            );
        }
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

/// How long a full trench takes to empty, h, as EKODREN's sheet works it
/// out: its height soaking away at kf (DWA-A 138 wants a day at most).
pub fn emptying_hours(height: f64, kf: f64) -> f64 {
    if kf <= 0.0 {
        f64::INFINITY
    } else {
        height / kf / 3600.0
    }
}

/// The volume a retention tank must hold for one rain, m³, after DWA-A
/// 117's simple method: what `area_u` m² bring in at the rain's intensity
/// less what `outflow` l/s let out over its duration, times the surcharge
/// factor `fz`.
pub fn retention_volume(area_u: f64, rain: Rain, outflow: f64, fz: f64) -> f64 {
    let inflow = area_u * 1e-7 * rain.intensity; // m³/s
    ((inflow - outflow / 1000.0) * rain.minutes * 60.0 * fz).max(0.0)
}

/// The rain of `rains` that needs the most volume, and that volume, m³.
pub fn critical_volume(area_u: f64, rains: &[Rain], outflow: f64, fz: f64) -> Option<(Rain, f64)> {
    rains
        .iter()
        .filter(|r| r.minutes > 0.0 && r.intensity > 0.0)
        .map(|r| (*r, retention_volume(area_u, *r, outflow, fz)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

/// How long a full retention tank of `volume` m³ takes to empty through
/// `outflow` l/s, h.
pub fn draining_hours(volume: f64, outflow: f64) -> f64 {
    if outflow <= 0.0 {
        f64::INFINITY
    } else {
        volume / (outflow / 1000.0) / 3600.0
    }
}

/// The blocks along a tank `across` × `layers` blocks that holds
/// `volume` m³.
pub fn blocks_along(c: &SoakawayConfig, volume: f64) -> u32 {
    let row = BLOCK * BLOCK * c.block.height() * VOID * (c.across * c.layers).max(1) as f64;
    (volume / row).ceil().max(1.0) as u32
}

/// One line of the parts list: what, how many, in what.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub name: String,
    pub qty: f64,
    pub unit: &'static str,
}

/// What to order and build for `c`, after EKODREN's laying instructions:
/// the blocks and their connectors (about 1.5 a block), covers (blends)
/// DN 315 at the corners, the blends the pipes go in by, the geotextile
/// (PP 180, the bottom over 200 mm past the edges, top and sides twice),
/// for a retention tank the 2 mm HDPE / PVC foil (welds 10 %) and a
/// protective geotextile on it, the shaft, a 100 mm gravel bed 16/32 and
/// the pit, 0.5 m wider than the blocks on every side.
pub fn parts(c: &SoakawayConfig) -> Vec<Part> {
    let (l, w, h) = c.size();
    let p = |name: String, qty: f64, unit| Part { name, qty, unit };
    let mut out = vec![
        p(c.block.name().to_string(), c.blocks() as f64, "pcs"),
        p(
            "DRENBLOK connector".into(),
            (c.blocks() as f64 * 1.5).ceil(),
            "pcs",
        ),
        p(
            "DRENBLOK cover (blend) DN 315".into(),
            (4 * c.layers) as f64,
            "pcs",
        ),
    ];
    if c.inlet_dn < 315 {
        out.push(p(format!("Inlet blend Ø{}", c.inlet_dn), 1.0, "pcs"));
    }
    if c.retention && c.outlet_dn < 315 {
        out.push(p(format!("Outlet blend Ø{}", c.outlet_dn), 1.0, "pcs"));
    }
    out.push(p("Vent and overflow blend Ø110".into(), 1.0, "pcs"));
    let bottom = (l + 0.4) * (w + 0.4);
    let wrap = 2.0 * (l * w + 2.0 * (l + w) * h) * 1.1;
    let fabric = if c.retention {
        2.0 * bottom + wrap
    } else {
        bottom + wrap
    };
    out.push(p(
        "Geotextile PP 180, calendered on both sides".into(),
        fabric.ceil(),
        "m²",
    ));
    if c.retention {
        let skin = 2.0 * (l * w + l * h + w * h) * 1.1;
        out.push(p("HDPE / PVC foil 2 mm, welded".into(), skin.ceil(), "m²"));
    }
    if c.filter != Filter::None {
        out.push(p(c.filter.name().to_string(), 1.0, "pcs"));
    }
    let (pl, pw) = (l + 1.0, w + 1.0);
    let bed = 0.1;
    out.push(p("Gravel 16/32, bed 100 mm".into(), pl * pw * bed, "m³"));
    out.push(p("Excavation".into(), pl * pw * (bed + h + c.cover), "m³"));
    out
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
    if c.retention {
        // the welded foil all round, 3 mm thick, just off the blocks
        let foil = plastic("HDPE foil, black", [28, 30, 32]);
        let (t, e) = (0.003, 0.008);
        let (xa, xb, ya, yb) = (-e, l + e, y0 - e, y1 + e);
        let (za, zb) = (bed - t, bed + h + e + t);
        for (name, lo, hi) in [
            ("Foil, bottom", v3(xa, ya, za), v3(xb, yb, za + t)),
            ("Foil, top", v3(xa, ya, zb - t), v3(xb, yb, zb)),
            ("Foil, side", v3(xa, ya, za + t), v3(xa + t, yb, zb - t)),
            ("Foil, side", v3(xb - t, ya, za + t), v3(xb, yb, zb - t)),
            (
                "Foil, side",
                v3(xa + t, ya, za + t),
                v3(xb - t, ya + t, zb - t),
            ),
            (
                "Foil, side",
                v3(xa + t, yb - t, za + t),
                v3(xb - t, yb, zb - t),
            ),
        ] {
            b.add(name, foil.clone(), cuboid(lo, hi), false);
        }
        // the throttled outlet at the bottom of the far end, out to the
        // recipient at 1 % or more
        let ro = c.outlet_dn as f64 / 2000.0;
        let oz = bed + ro + 0.02;
        let ox = l + 1.5;
        b.add(
            &format!("Outlet blend Ø{} (throttle)", c.outlet_dn),
            plastic("PP, blend", [40, 90, 170]),
            rod(v3(l + e, ym, oz), v3(l + e + 0.02, ym, oz), ro + 0.03),
            false,
        );
        b.add(
            &format!("Outlet to the recipient DN {}", c.outlet_dn),
            plastic("PVC-U KG, orange", [196, 104, 48]),
            rod(v3(l + e + 0.02, ym, oz), v3(ox, ym, oz - 0.015), ro),
            false,
        );
        b.line(v2(l, ym - 0.12), v2(ox, ym - 0.12));
        b.line(v2(ox - 0.15, ym - 0.2), v2(ox, ym - 0.12));
        b.line(v2(ox - 0.15, ym - 0.04), v2(ox, ym - 0.12));
    }
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
    if c.retention {
        return b.finish(
            &format!(
                "site-retention-{}-{}x{}x{}-{:?}-dn{}",
                c.block.height() * 100.0,
                c.along,
                c.across,
                c.layers,
                c.filter,
                c.outlet_dn
            )
            .to_lowercase(),
            &format!("Retention tank {}", c.code()),
            &format!(
                "Rainwater retention tank {}: {:.1} × {:.1} × {:.1} m, {:.2} m³ gross, {:.2} m³ stored (95 % void), wrapped in welded 2 mm HDPE / PVC foil between geotextiles on a gravel bed, {:.1} m of earth over it; {} ahead with the inlet DN {}; throttled outlet DN {} ({:.1} l/s) at the bottom of the far end to the recipient at 1 % or more; vent and safety overflow DN 110 (EKODREN)",
                c.code(),
                l,
                w,
                h,
                c.gross(),
                c.storage(),
                c.cover,
                filter,
                c.inlet_dn,
                c.outlet_dn,
                c.outflow
            ),
            ObjectCategory::Site,
            ("IfcTank", "STORAGE"),
            Mount::Floor,
        );
    }
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
    vec![
        soakaway(&SoakawayConfig::default()),
        soakaway(&SoakawayConfig::rn20()),
    ]
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
        assert!((emptying_hours(1.2, 1e-5) - 33.33).abs() < 0.01);
        assert_eq!(Filter::for_area(400.0), Filter::Fs400);
    }

    #[test]
    fn it_matches_ekodrens_sheet() {
        // EKODREN's VZOR: 1000 m², Piešťany 5-year rain 180 min at 31
        // l/(s·ha), kf 1e-5, 2.4 m wide: 13.710 m in 2 layers, 26.038 in 1
        let rain = Rain {
            minutes: 180.0,
            intensity: 31.0,
        };
        let two = trench_length(1000.0, rain, 2.4, 1.2, VOID, 1e-5, 1.2);
        let one = trench_length(1000.0, rain, 2.4, 0.6, VOID, 1e-5, 1.2);
        assert!((two - 13.710).abs() < 5e-4 && (one - 26.038).abs() < 5e-4);
        // its Bratislava sheet: 100 m², 50-year rain 120 min raised to
        // 80.6 l/(s·ha), chosen by hand: 2.430 m
        let ba = Rain {
            minutes: 120.0,
            intensity: 80.6,
        };
        let len = trench_length(100.0, ba, 2.4, 1.2, VOID, 1e-5, 1.2);
        assert!((len - 2.43015).abs() < 1e-4, "{len}");
        let c = SoakawayConfig {
            block: Block::Db40,
            layers: 4,
            ..Default::default()
        };
        assert_eq!(c.max_cover(), 3.9);
    }

    #[test]
    fn a_retention_tank_is_ekodrens_rn20() {
        let c = SoakawayConfig::rn20();
        // 4.8 × 3.6 × 1.2 m, 20 736 l gross, 96 blocks
        assert_eq!(c.blocks(), 96);
        assert!((c.gross() - 20.736).abs() < 1e-9);
        assert!(c.code().starts_with("EKODREN RN20: "), "{}", c.code());
        let o = soakaway(&c);
        assert!(o.name.starts_with("Retention tank"));
        assert!(o
            .object
            .parts
            .iter()
            .any(|p| p.name.contains("Outlet to the recipient")));
        assert!(o.object.parts.iter().any(|p| p.name == "Foil, top"));
        // 300 m² of roof, a 15 min rain of 200 l/(s·ha), 1 l/s out:
        // (0.006 − 0.001) m³/s × 900 s × 1.2 = 5.4 m³
        let r = Rain {
            minutes: 15.0,
            intensity: 200.0,
        };
        assert!((retention_volume(300.0, r, 1.0, 1.2) - 5.4).abs() < 1e-9);
        // a long weak rain needs more once the outlet is small
        let long = Rain {
            minutes: 120.0,
            intensity: 50.0,
        };
        let (crit, v) = critical_volume(300.0, &[r, long], 0.5, 1.2).unwrap();
        assert_eq!(crit.minutes, 120.0);
        assert!((v - (300.0 * 1e-7 * 50.0 - 0.0005) * 7200.0 * 1.2).abs() < 1e-9);
        // nothing to hold when the outlet takes it all
        assert_eq!(retention_volume(10.0, r, 5.0, 1.2), 0.0);
        assert!((draining_hours(19.7, 1.0) - 19.7 / 3.6).abs() < 1e-9);
        // 5.4 m³ in 6 × 2 DB60 rows (2.4624 m³ a row): 3 rows
        assert_eq!(blocks_along(&c, 5.4), 3);
        let parts = parts(&c);
        let qty = |n: &str| parts.iter().find(|p| p.name.starts_with(n)).unwrap().qty;
        assert_eq!(qty("DRENBLOK DB60"), 96.0);
        assert_eq!(qty("DRENBLOK connector"), 144.0);
        assert!(qty("HDPE") > 2.0 * (4.8 * 3.6 + 4.8 * 1.2 + 3.6 * 1.2));
        assert!(qty("Outlet blend") == 1.0);
        // a soakaway has no foil
        assert!(!super::parts(&SoakawayConfig::default())
            .iter()
            .any(|p| p.name.starts_with("HDPE")));
    }
}
