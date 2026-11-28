//! The living room's larger pieces: corner sofas and sofas with a chaise
//! longue (each with its corner or chaise on the left or the right, as
//! seen from in front), TV units and wall-hung LED televisions.

use super::{black_glass, cylinder, fabric, laminate, soft_box, v2, v3, walnut, Build, Look};
use crate::objects::{CatalogueObject, Mount, ObjectCategory};
use glam::DVec3;

/// The corner sofas: overall width and the depth of the return, mm.
pub const CORNER_SOFAS: [(u32, u32); 2] = [(2500, 1700), (2900, 2000)];
/// The sofas with a chaise longue: overall width, mm (the chaise 1600 deep).
pub const CHAISE_SOFAS: [u32; 2] = [2400, 2800];
/// TV units: width, mm.
pub const TV_UNITS: [u32; 3] = [1600, 1800, 2000];
/// LED televisions: diagonal in inches, and the set's width and height
/// (16:9 screens of the size, with a slim frame), mm.
pub const TVS: [(u32, u32, u32); 3] = [(55, 1230, 715), (65, 1450, 835), (75, 1680, 965)];

/// Seat and back as the three-seat sofa's: seat depth, back and arm
/// thickness, heights.
const D: f64 = 0.9;
const BACK: f64 = 0.2;
const ARM: f64 = 0.2;
const SEAT: f64 = 0.44;
const BASE: f64 = 0.08;

/// A soft box between any two opposite corners.
fn block(b: &mut Build, name: &str, cover: &Look, a: DVec3, c: DVec3, r: f64, plan: bool) {
    b.add(
        name,
        cover.clone(),
        soft_box(a.min(c), a.max(c), r, r * 0.8),
        plan,
    );
}

fn feet(b: &mut Build, pts: &[(f64, f64)]) {
    for &(x, y) in pts {
        b.add(
            "Foot",
            walnut(),
            cylinder(v2(x, y), 0.025, 0.0, BASE),
            false,
        );
    }
}

fn grey() -> Look {
    fabric("Fabric, grey", [118, 122, 128])
}

fn side_word(left: bool) -> &'static str {
    if left {
        "left"
    } else {
        "right"
    }
}

/// An L-shaped corner sofa `w` wide whose return, `l` deep, is on the
/// left (seen from in front) or the right. Its back runs along the wall
/// and down the return's outer side.
fn corner_sofa(w_mm: u32, l_mm: u32, left: bool) -> CatalogueObject {
    let (w, l) = (w_mm as f64 / 1000.0, l_mm as f64 / 1000.0);
    // seen from in front (looking towards +y) left is −x
    let s = if left { -1.0 } else { 1.0 };
    let (xe, xf) = (s * w / 2.0, -s * w / 2.0);
    let cover = grey();
    let mut b = Build::new();
    // the backs: along the wall, and down the return's outer side
    block(
        &mut b,
        "Backrest",
        &cover,
        v3(xf, -BACK, BASE),
        v3(xe, 0.0, 0.82),
        0.03,
        true,
    );
    block(
        &mut b,
        "Backrest",
        &cover,
        v3(xe - s * BACK, -l + ARM, BASE),
        v3(xe, -BACK, 0.82),
        0.03,
        true,
    );
    // an arm at each free end
    block(
        &mut b,
        "Arm",
        &cover,
        v3(xf, -D, BASE),
        v3(xf + s * ARM, -BACK, 0.62),
        0.05,
        true,
    );
    block(
        &mut b,
        "Arm",
        &cover,
        v3(xe, -l, BASE),
        v3(xe - s * D, -l + ARM, 0.62),
        0.05,
        true,
    );
    // the seats
    block(
        &mut b,
        "Seat",
        &cover,
        v3(xf + s * ARM, -D, BASE),
        v3(xe - s * BACK, -BACK, SEAT),
        0.05,
        true,
    );
    block(
        &mut b,
        "Seat",
        &cover,
        v3(xe - s * D, -l + ARM, BASE),
        v3(xe - s * BACK, -D, SEAT),
        0.05,
        true,
    );
    // back cushions along both backs
    block(
        &mut b,
        "Back cushion",
        &cover,
        v3(xf + s * ARM, -BACK - 0.18, SEAT),
        v3(xe - s * BACK, -BACK, 0.78),
        0.06,
        false,
    );
    block(
        &mut b,
        "Back cushion",
        &cover,
        v3(xe - s * (BACK + 0.18), -l + ARM, SEAT),
        v3(xe - s * BACK, -BACK - 0.18, 0.78),
        0.06,
        false,
    );
    feet(
        &mut b,
        &[
            (xf + s * 0.06, -D + 0.06),
            (xf + s * 0.06, -0.06),
            (xe - s * 0.06, -0.06),
            (xe - s * 0.06, -l + 0.06),
            (xe - s * (D - 0.06), -l + 0.06),
        ],
    );
    let side = side_word(left);
    b.finish(
        &format!("corner-sofa-{side}-{w_mm}x{l_mm}"),
        &format!("Corner sofa {w_mm} × {l_mm}, corner {side}"),
        &format!(
            "L-shaped sofa {w_mm} wide with its return {l_mm} deep on the {side}, seat height 440 mm"
        ),
        ObjectCategory::Living,
        ("IfcFurniture", "SOFA"),
        Mount::Wall,
    )
}

/// A sofa `w` wide with a chaise longue 1600 deep at its left or right
/// end: an arm at the other end only.
fn chaise_sofa(w_mm: u32, left: bool) -> CatalogueObject {
    let w = w_mm as f64 / 1000.0;
    let (l, cw) = (1.6, 0.85);
    let s = if left { -1.0 } else { 1.0 };
    let (xe, xf) = (s * w / 2.0, -s * w / 2.0);
    let cover = fabric("Fabric, sand", [176, 160, 136]);
    let mut b = Build::new();
    block(
        &mut b,
        "Backrest",
        &cover,
        v3(xf, -BACK, BASE),
        v3(xe, 0.0, 0.82),
        0.03,
        true,
    );
    block(
        &mut b,
        "Arm",
        &cover,
        v3(xf, -D, BASE),
        v3(xf + s * ARM, -BACK, 0.62),
        0.05,
        true,
    );
    // the seats, and the chaise reaching forward
    block(
        &mut b,
        "Seat",
        &cover,
        v3(xf + s * ARM, -D, BASE),
        v3(xe - s * cw, -BACK, SEAT),
        0.05,
        true,
    );
    block(
        &mut b,
        "Chaise",
        &cover,
        v3(xe - s * cw, -l, BASE),
        v3(xe, -BACK, SEAT),
        0.05,
        true,
    );
    block(
        &mut b,
        "Back cushion",
        &cover,
        v3(xf + s * ARM, -BACK - 0.18, SEAT),
        v3(xe, -BACK, 0.78),
        0.06,
        false,
    );
    feet(
        &mut b,
        &[
            (xf + s * 0.06, -D + 0.06),
            (xf + s * 0.06, -0.06),
            (xe - s * 0.06, -0.06),
            (xe - s * 0.06, -l + 0.06),
            (xe - s * (cw - 0.06), -l + 0.06),
        ],
    );
    let side = side_word(left);
    b.finish(
        &format!("chaise-sofa-{side}-{w_mm}"),
        &format!("Sofa with chaise {w_mm}, chaise {side}"),
        &format!(
            "Sofa {w_mm} wide with a chaise longue 1600 deep on the {side}, seat height 440 mm"
        ),
        ObjectCategory::Living,
        ("IfcFurniture", "SOFA"),
        Mount::Wall,
    )
}

/// A low TV unit `w` wide, 400 deep, 450 high, its fronts 600 wide or so.
fn tv_unit(w_mm: u32) -> CatalogueObject {
    let w = w_mm as f64 / 1000.0;
    let (d, h) = (0.4, 0.45);
    let mut b = Build::new();
    b.add(
        "Carcass",
        laminate(),
        super::cuboid(v3(-w / 2.0, -d + 0.02, 0.06), v3(w / 2.0, 0.0, h - 0.02)),
        true,
    );
    b.add(
        "Plinth",
        super::plinth(),
        super::cuboid(
            v3(-w / 2.0 + 0.03, -d + 0.05, 0.0),
            v3(w / 2.0 - 0.03, -0.03, 0.06),
        ),
        false,
    );
    b.add(
        "Top",
        walnut(),
        super::cuboid(v3(-w / 2.0, -d, h - 0.02), v3(w / 2.0, 0.0, h)),
        true,
    );
    let n = ((w / 0.6).round() as usize).max(2);
    let fw = w / n as f64;
    for i in 0..n {
        let x0 = -w / 2.0 + i as f64 * fw;
        b.add(
            "Drawer",
            laminate(),
            super::cuboid(
                v3(x0 + 0.002, -d, 0.062),
                v3(x0 + fw - 0.002, -d + 0.02, h - 0.022),
            ),
            false,
        );
    }
    b.line(v2(-w / 2.0, -d), v2(w / 2.0, -d));
    b.finish(
        &format!("tv-unit-{w_mm}"),
        &format!("TV unit {w_mm}"),
        &format!("Low TV unit {w_mm} × 400 × 450 with {n} drawers and a walnut top"),
        ObjectCategory::Living,
        ("IfcFurniture", "SHELF"),
        Mount::Wall,
    )
}

/// A wall-hung LED television, its screen's middle 1100 mm above the
/// floor (seen from a sofa).
fn tv(inch: u32, w_mm: u32, h_mm: u32) -> CatalogueObject {
    let (w, h) = (w_mm as f64 / 1000.0, h_mm as f64 / 1000.0);
    let mut b = Build::new();
    b.add(
        "Backrest",
        super::black_metal(),
        super::cuboid(
            v3(-w / 2.0 + 0.1, -0.07, 0.1),
            v3(w / 2.0 - 0.1, 0.0, h - 0.1),
        ),
        false,
    );
    b.add(
        "Screen",
        black_glass(),
        soft_box(
            v3(-w / 2.0, -0.105, 0.0),
            v3(w / 2.0, -0.07, h),
            0.004,
            0.002,
        ),
        true,
    );
    b.at(1.1 - h / 2.0);
    b.finish(
        &format!("tv-led-{inch}"),
        &format!("LED TV {inch}\""),
        &format!(
            "Wall-hung LED television {inch}\" ({w_mm} × {h_mm}), on its own socket behind it"
        ),
        ObjectCategory::Living,
        ("IfcAudioVisualAppliance", "DISPLAY"),
        Mount::Wall,
    )
}

pub fn starters() -> Vec<CatalogueObject> {
    let mut out = vec![];
    for (w, l) in CORNER_SOFAS {
        for left in [true, false] {
            out.push(corner_sofa(w, l, left));
        }
    }
    for w in CHAISE_SOFAS {
        for left in [true, false] {
            out.push(chaise_sofa(w, left));
        }
    }
    out.extend(TV_UNITS.into_iter().map(tv_unit));
    out.extend(TVS.into_iter().map(|(i, w, h)| tv(i, w, h)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A corner sofa's return is on its side and reaches its depth; the
    /// chaise reaches 1600; the television hangs with its middle at 1100.
    #[test]
    fn sofas_turn_their_corner_to_their_side() {
        let left = corner_sofa(2500, 1700, true).object;
        let (lo, hi) = left.bounds();
        assert!((hi.x - lo.x - 2.5).abs() < 1e-6 && (hi.y - lo.y - 1.7).abs() < 1e-6);
        // the return's arm, at the far front, is on the left (−x)
        let arm = left
            .parts
            .iter()
            .filter(|p| p.name == "Arm")
            .map(|p| p.mesh.vertices.iter().map(|v| v.y).fold(0.0, f64::min))
            .fold(0.0, f64::min);
        assert!((arm + 1.7).abs() < 1e-6);
        let front_x: Vec<f64> = left
            .parts
            .iter()
            .flat_map(|p| p.mesh.vertices.iter())
            .filter(|v| v.y < -1.6)
            .map(|v| v.x)
            .collect();
        assert!(front_x.iter().all(|x| *x < 0.0), "{front_x:?}");
        let right = corner_sofa(2500, 1700, false).object;
        assert!(right
            .parts
            .iter()
            .flat_map(|p| p.mesh.vertices.iter())
            .filter(|v| v.y < -1.6)
            .all(|v| v.x > 0.0));
        let chaise = chaise_sofa(2400, true).object;
        assert!((chaise.size().y - 1.6).abs() < 1e-6);
        let t = tv(65, 1450, 835).object;
        assert!((t.offset + t.size().z / 2.0 - 1.1).abs() < 1e-6);
        assert_eq!(starters().len(), 4 + 4 + 3 + 3);
    }
}
