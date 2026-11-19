//! JIKA products from their technical sheets: two small basins with a
//! single-lever tap, a bottle trap and angle valves, and a wall-hung pan
//! on its concealed frame behind a plasterboard pre-wall, all as one
//! object each. Their connections are `Connection <code> DN<n>` parts
//! where the maker's drawings put them ([`crate::fixture::part_connectors`]).
//!
//! Object axes: the back on the wall at y = 0, the front towards −y, x to
//! the right seen from the front, heights above the finished floor.

use super::{
    ceramic, chrome, cuboid, cylinder, look, metal, rrect, soft_box, squircle, stack, v2, v3, Build,
};
use crate::objects::{CatalogueObject, Mount, ObjectCategory};
use crate::Hatch;
use glam::DVec2;

/// A connection stub `r` round: in the wall behind `x`, its axis at `z`.
fn connection(b: &mut Build, code: &str, dn: u32, x: f64, y: f64, z: f64) {
    b.add(
        &format!("Connection {code} DN{dn}"),
        metal("Brass", [196, 160, 92], 0.3),
        cylinder(v2(x, y), 0.011, z - 0.02, z),
        false,
    );
}

/// A single-lever basin mixer standing in its hole at `at` on a rim `z`
/// high, its spout reaching forward over the bowl.
fn mixer(b: &mut Build, at: DVec2, z: f64, reach: f64) {
    b.add(
        "Mixer body",
        chrome(),
        cylinder(at, 0.024, z, z + 0.15),
        true,
    );
    b.add(
        "Mixer spout",
        chrome(),
        soft_box(
            v3(at.x - 0.011, at.y - reach, z + 0.11),
            v3(at.x + 0.011, at.y, z + 0.135),
            0.008,
            0.003,
        ),
        true,
    );
    b.add(
        "Mixer lever",
        chrome(),
        soft_box(
            v3(at.x - 0.008, at.y - 0.01, z + 0.15),
            v3(at.x + 0.008, at.y + 0.045, z + 0.165),
            0.005,
            0.002,
        ),
        false,
    );
}

/// The bottle trap under the drain at `drain` (the basin's underside
/// `under` high) to the waste in the wall at `waste` high, and the two
/// angle valves at the water connections `water` high, `half` either side
/// of `x`, with their hoses up to the mixer at `tap`.
#[allow(clippy::too_many_arguments)]
fn trap_and_valves(
    b: &mut Build,
    drain: DVec2,
    under: f64,
    waste: f64,
    x: f64,
    half: f64,
    water: f64,
    tap: DVec2,
) {
    let trap = chrome();
    b.add(
        "Trap tailpiece",
        trap.clone(),
        cylinder(drain, 0.016, waste - 0.02, under),
        false,
    );
    b.add(
        "Trap bottle",
        trap.clone(),
        cylinder(drain, 0.03, waste - 0.13, waste - 0.01),
        false,
    );
    b.add(
        "Trap arm",
        trap.clone(),
        cuboid(
            v3(drain.x - 0.016, drain.y, waste - 0.016),
            v3(drain.x + 0.016, -0.0, waste + 0.016),
        ),
        false,
    );
    b.add(
        "Wall rose",
        trap,
        cuboid(
            v3(drain.x - 0.035, -0.008, waste - 0.035),
            v3(drain.x + 0.035, 0.0, waste + 0.035),
        ),
        false,
    );
    for (side, code) in [(-1.0, "TV"), (1.0, "SV")] {
        let vx = x + side * half;
        b.add(
            &format!("Angle valve {code}"),
            chrome(),
            cuboid(
                v3(vx - 0.012, -0.05, water - 0.012),
                v3(vx + 0.012, 0.0, water + 0.012),
            ),
            false,
        );
        b.add(
            &format!("Hose {code}"),
            metal("Braided steel", [170, 172, 176], 0.5),
            cuboid(v3(vx - 0.004, -0.05, water), v3(vx + 0.004, -0.042, under)),
            false,
        );
        b.add(
            &format!("Hose to mixer {code}"),
            metal("Braided steel", [170, 172, 176], 0.5),
            cuboid(
                v3(
                    vx.min(tap.x) - 0.004,
                    tap.y.min(-0.042) - 0.004,
                    under - 0.008,
                ),
                v3(vx.max(tap.x) + 0.004, tap.y.max(-0.042) + 0.004, under),
            ),
            false,
        );
    }
}

/// A basin of `w` × `d`, `h` deep with its rim at `top`: its outside
/// rounded `r`, its bowl `bowl` (centre, width, depth) and its drain.
fn basin_body(b: &mut Build, w: f64, d: f64, h: f64, top: f64, bowl: (DVec2, f64, f64)) {
    let k = 6;
    let c = v2(0.0, -d / 2.0);
    let (bc, bw, bd) = bowl;
    b.add(
        "Basin",
        ceramic(),
        stack(&[
            (
                rrect(c + v2(0.0, 0.02), w - 0.06, d - 0.06, 0.03, k),
                top - h,
            ),
            (rrect(c, w, d, 0.02, k), top - h + 0.04),
            (rrect(c, w, d, 0.02, k), top),
            (rrect(bc, bw, bd, 0.05, k), top),
            (rrect(bc, bw - 0.04, bd - 0.04, 0.045, k), top - h * 0.6),
            (rrect(bc, bw * 0.45, bd * 0.45, 0.03, k), top - h + 0.03),
        ]),
        true,
    );
    b.outline(rrect(bc, bw, bd, 0.05, k));
    b.add(
        "Drain",
        chrome(),
        cylinder(bc, 0.02, top - h + 0.028, top - h + 0.032),
        false,
    );
}

/// JIKA Lyra Plus 400 (H8153800001041): 400 × 340 × 170, rim 850,
/// tap hole in the middle 55 from the wall, drain 190 from the wall;
/// water 580 high 100 apart, waste 530, on the middle.
fn lyra_plus() -> CatalogueObject {
    let (w, d, h, top) = (0.4, 0.34, 0.17, 0.85);
    let mut b = Build::new();
    basin_body(&mut b, w, d, h, top, (v2(0.0, -0.205), 0.35, 0.22));
    let tap = v2(0.0, -0.055);
    mixer(&mut b, tap, top, 0.11);
    let drain = v2(0.0, -0.19);
    trap_and_valves(&mut b, drain, top - h, 0.53, 0.0, 0.05, 0.58, tap);
    connection(&mut b, "TV", 15, -0.05, -0.011, 0.58);
    connection(&mut b, "SV", 15, 0.05, -0.011, 0.58);
    connection(&mut b, "K", 40, 0.0, -0.011, 0.53);
    b.finish(
        "jika-basin-lyra-plus-400",
        "JIKA Lyra Plus basin 400",
        "JIKA Lyra Plus small basin H8153800001041, 400 × 340 × 170, wall-hung, rim at 850 mm, with a single-lever mixer, bottle trap and two angle valves; water DN 15 at 580 mm 100 apart, waste DN 40 at 530 mm",
        ObjectCategory::Bathroom,
        ("IfcSanitaryTerminal", "WASHHANDBASIN"),
        Mount::Wall,
    )
}

/// JIKA Cubito 450, tap on the right (H8114240001061): 450 × 250 × 160,
/// rim 850, the bowl on the left (drain 185 from its left edge, 90 from
/// the wall), a shelf 120 wide on the right with the tap hole; water 630
/// high 100 apart, waste 580, under the drain.
fn cubito() -> CatalogueObject {
    let (w, d, h, top) = (0.45, 0.25, 0.16, 0.85);
    let mut b = Build::new();
    let x = -w / 2.0 + 0.185;
    basin_body(&mut b, w, d, h, top, (v2(x, -0.13), 0.29, 0.18));
    let tap = v2(w / 2.0 - 0.06, -0.072);
    mixer(&mut b, tap, top, 0.08);
    let drain = v2(x, -0.09);
    trap_and_valves(&mut b, drain, top - h, 0.58, x, 0.05, 0.63, tap);
    connection(&mut b, "TV", 15, x - 0.05, -0.011, 0.63);
    connection(&mut b, "SV", 15, x + 0.05, -0.011, 0.63);
    connection(&mut b, "K", 40, x, -0.011, 0.58);
    b.finish(
        "jika-basin-cubito-450",
        "JIKA Cubito basin 450",
        "JIKA Cubito asymmetric small basin H8114240001061, 450 × 250 × 160, tap on the right, wall-hung, rim at 850 mm, with a single-lever mixer, bottle trap and two angle valves; water DN 15 at 630 mm 100 apart, waste DN 40 at 580 mm",
        ObjectCategory::Bathroom,
        ("IfcSanitaryTerminal", "WASHHANDBASIN"),
        Mount::Wall,
    )
}

/// How far the pre-wall stands out from the wall: the frame 150 (130 to
/// 165) and two layers of 12.5 mm plasterboard.
pub const PRE_WALL: f64 = 0.175;

/// JIKA Deep wall-hung rimless pan (H8206160000001), 360 × 510, top 430,
/// on the JIKA Modul WC system frame (H8956520000001): 450 × 1120, 130 to
/// 165 from the wall, studs 180/230 apart 330 high, the pan's waste
/// 230 high into the frame's DN 90/110 bend 85 to 95 from the wall,
/// water R1/2 at the top of the cistern; a plasterboard pre-wall 600
/// wide and 1200 high in front of it with the flush plate.
fn deep_on_modul() -> CatalogueObject {
    let mut b = Build::new();
    let y0 = -PRE_WALL;
    let steel = metal("Steel, galvanised", [150, 156, 160], 0.5);
    // the frame: two posts on adjustable feet, cross bars, the cistern
    let fy = (-0.15, -0.053);
    for x in [-0.205, 0.185] {
        b.add(
            "Frame post",
            steel.clone(),
            cuboid(v3(x, fy.0, 0.0), v3(x + 0.02, fy.1, 1.12)),
            false,
        );
    }
    for (z, t) in [(1.1, 0.02), (0.47, 0.02), (0.18, 0.02)] {
        b.add(
            "Frame bar",
            steel.clone(),
            cuboid(v3(-0.205, fy.0, z), v3(0.205, fy.0 + 0.03, z + t)),
            false,
        );
    }
    b.add(
        "Cistern",
        look(
            "Plastic, cistern",
            "plastic",
            Hatch::Empty,
            [70, 90, 140],
            0.6,
        ),
        cuboid(v3(-0.18, -0.14, 0.6), v3(0.18, -0.06, 1.08)),
        false,
    );
    b.add(
        "Waste bend DN 90/110",
        look("PE, black", "plastic", Hatch::Empty, [30, 30, 32], 0.6),
        stack(&[
            (squircle(v2(0.0, -0.09), 0.055, 0.055, 2.0, 20), 0.04),
            (squircle(v2(0.0, -0.11), 0.055, 0.055, 2.0, 20), 0.2),
            (squircle(v2(0.0, -0.15), 0.055, 0.055, 2.0, 20), 0.26),
        ]),
        false,
    );
    b.add(
        "Pan connection",
        look("PE, white", "plastic", Hatch::Empty, [235, 235, 235], 0.6),
        cuboid(v3(-0.045, y0 - 0.002, 0.185), v3(0.045, -0.15, 0.275)),
        false,
    );
    for x in [-0.09, 0.09] {
        b.add(
            "Stud",
            steel.clone(),
            cylinder(v2(x, y0 - 0.06), 0.006, 0.324, 0.336),
            false,
        );
    }
    // the plasterboard pre-wall and the flush plate on it
    b.add(
        "Pre-wall",
        look(
            "Plasterboard",
            "plaster",
            Hatch::Empty,
            [236, 235, 230],
            0.9,
        ),
        cuboid(v3(-0.3, y0, 0.0), v3(0.3, -0.0, 1.2)),
        true,
    );
    b.add(
        "Flush plate",
        chrome(),
        soft_box(
            v3(-0.123, y0 - 0.012, 0.92),
            v3(0.123, y0, 1.084),
            0.004,
            0.002,
        ),
        false,
    );
    // the pan: 510 out from the pre-wall, 360 wide, top 430
    let n = 32;
    let cy = y0 - 0.27;
    let rim = 0.415;
    b.add(
        "Pan",
        ceramic(),
        stack(&[
            (squircle(v2(0.0, y0 - 0.13), 0.12, 0.13, 3.0, n), 0.2),
            (squircle(v2(0.0, y0 - 0.2), 0.16, 0.2, 3.0, n), 0.3),
            (
                squircle(v2(0.0, cy + 0.015), 0.18, 0.255, 3.2, n),
                rim - 0.03,
            ),
            (squircle(v2(0.0, cy + 0.015), 0.18, 0.255, 3.2, n), rim),
            (
                squircle(v2(0.0, cy - 0.01), 0.13, 0.19, 2.8, n),
                rim - 0.005,
            ),
            (squircle(v2(0.0, cy - 0.02), 0.06, 0.08, 2.6, n), 0.24),
        ]),
        true,
    );
    b.add(
        "Seat and lid",
        look(
            "Duroplast, white",
            "plastic",
            Hatch::Empty,
            [246, 246, 244],
            0.4,
        ),
        stack(&[
            (squircle(v2(0.0, cy + 0.01), 0.178, 0.25, 3.2, n), rim),
            (squircle(v2(0.0, cy + 0.01), 0.178, 0.25, 3.2, n), 0.43),
        ]),
        false,
    );
    b.outline(squircle(v2(0.0, cy - 0.01), 0.13, 0.19, 2.8, n));
    connection(&mut b, "SV", 15, -0.12, -0.08, 1.04);
    connection(&mut b, "K", 100, 0.0, -0.09, 0.06);
    b.finish(
        "jika-wc-deep-modul",
        "JIKA Deep WC on Modul frame",
        "JIKA Deep wall-hung rimless pan H8206160000001 (360 × 510, top 430 mm) on the JIKA Modul WC system frame H8956520000001 (450 × 1120, concealed cistern 3/6 l, bend DN 90/110) in a plasterboard pre-wall 600 × 175 × 1200 with the flush plate; water DN 15 (R1/2) at the top of the frame, waste DN 100 from the bend",
        ObjectCategory::Toilet,
        ("IfcSanitaryTerminal", "TOILETPAN"),
        Mount::Wall,
    )
}

/// The JIKA products George designs with.
pub fn starters() -> Vec<CatalogueObject> {
    vec![lyra_plus(), cubito(), deep_on_modul()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{family_of_type, part_connectors, ConnKind};

    #[test]
    fn the_jika_fixtures_join_where_their_sheets_say() {
        let all = starters();
        let lyra = &all[0].object;
        let c = part_connectors(lyra);
        let kinds: Vec<_> = c.iter().map(|c| c.kind).collect();
        assert_eq!(kinds, vec![ConnKind::TV, ConnKind::SV, ConnKind::K]);
        assert!((c[0].at.z - 0.58).abs() < 1e-9 && (c[2].at.z - 0.53).abs() < 1e-9);
        assert_eq!(family_of_type(lyra), Some("Umývadlo"));
        let cubito = part_connectors(&all[1].object);
        // under the bowl, 185 from the left edge of the 450 basin
        assert!((cubito[2].at.x + 0.04).abs() < 1e-9, "{:?}", cubito[2]);
        assert!((cubito[0].at.z - 0.63).abs() < 1e-9);
        let wc = &all[2].object;
        assert_eq!(
            family_of_type(wc),
            Some("Závesné WC s podomietkovou nádržkou")
        );
        let w = part_connectors(wc);
        assert_eq!((w[0].kind, w[0].dn), (ConnKind::SV, 15.0));
        assert_eq!((w[1].kind, w[1].dn), (ConnKind::K, 100.0));
        // the pan reaches 510 out from the pre-wall
        let (lo, hi) = wc.bounds();
        assert!((lo.y + PRE_WALL + 0.51).abs() < 0.03, "{lo}");
        assert!(hi.z >= 1.2 - 1e-9);
    }
}
