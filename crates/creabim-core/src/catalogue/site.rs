//! What stands on the plot where the networks come in: the water meter
//! shaft with its whole meter assembly, sewer inspection chambers (Wavin
//! Tegra 425 and 600), the electricity meter pillar and the gas meter box
//! with its main valve and regulator. Their connections are `Connection
//! <code> DN<n>` parts ([`crate::fixture::part_connectors`]).
//!
//! Object axes as every catalogue object's: x to the right, the front
//! towards −y, the back at y = 0. A shaft's top is at the object's top:
//! it is placed its depth below the level (`offset`), so its lid lies on
//! the ground at ±0.

use super::valvebox::rod;
use super::{cuboid, cylinder, look, metal, prism, squircle, v2, v3, Build, Look};
use crate::objects::{CatalogueObject, Mount, ObjectCategory};
use crate::Hatch;
use glam::{DVec2, DVec3};
use std::f64::consts::TAU;

fn plastic(name: &str, rgb: [u8; 3]) -> Look {
    look(name, "plastic", Hatch::Empty, rgb, 0.6)
}

/// A connection stub at `at` (its top), `r` round.
fn connection(b: &mut Build, code: &str, dn: u32, at: DVec3) {
    b.add(
        &format!("Connection {code} DN{dn}"),
        metal("Brass", [196, 160, 92], 0.3),
        cylinder(at.truncate(), 0.012, at.z - 0.02, at.z),
        false,
    );
}

/// A round wall from `r0` to `r1` about `c`, `z0` to `z1`, in `n` staves.
fn ring(
    b: &mut Build,
    name: &str,
    l: &Look,
    (c, r0, r1): (DVec2, f64, f64),
    z: (f64, f64),
    n: usize,
) {
    for i in 0..n {
        let (a0, a1) = (TAU * i as f64 / n as f64, TAU * (i + 1) as f64 / n as f64);
        let p = |r: f64, a: f64| c + DVec2::from_angle(a) * r;
        b.add(
            name,
            l.clone(),
            prism(vec![p(r0, a0), p(r1, a0), p(r1, a1), p(r0, a1)], z.0, z.1),
            false,
        );
    }
}

/// The water meter shaft of the ZTI.006 sample ("Vodomerná šachta
/// 900/1200/1800 vnútorné, vodovzdorná, monolitická"): inside 1200 × 900
/// × 1800, walls 100, floor 150, roof 120 with a 600 × 600 opening, the
/// concrete riser and the 600 × 600 cover on the ground (its roof 500
/// under the ground), five plastic-coated steps; on a 30 mm sand bed,
/// 150 mm of concrete and 120 mm of gravel. The service pipe D32 comes
/// in and goes out through its side walls 400 above its floor, 150 from
/// its front; inside, the meter assembly of the drawing: transition
/// plastic-steel, ball valve DN 25, reducer 25/20, the wet-dial meter DN
/// 20, reducer 20/25, ball valve DN 25, transition steel-plastic. Every
/// size is the drawing's; where it gives none (the steps' places, the
/// layers' overhang), it is read off the drawing at its scale.
pub fn water_meter_shaft() -> CatalogueObject {
    let mut b = Build::new();
    let concrete = look(
        "Concrete C30/37, watertight",
        "concrete",
        Hatch::Concrete,
        [176, 176, 170],
        0.9,
    );
    let lean = look(
        "Lean concrete",
        "concrete",
        Hatch::Concrete,
        [196, 194, 186],
        0.9,
    );
    let sand = look("Sand bed", "stone", Hatch::Empty, [214, 200, 160], 0.95);
    let gravel = look("Gravel", "stone", Hatch::Diagonal, [190, 180, 160], 0.95);
    let steel = metal("Steel, plastic-coated", [60, 64, 70], 0.5);
    let iron = metal("Cast iron", [70, 72, 74], 0.6);
    // heights from the bottom of the gravel
    let (gv, ln, sd) = (0.12, 0.15, 0.03);
    let z0 = gv + ln + sd; // the shaft's floor underside
    let (floor, inner_h, roof) = (0.15, 1.8, 0.12);
    let zf = z0 + floor; // its floor
    let zr = zf + inner_h; // its roof's underside
    let zt = zr + roof; // its roof's top
    let ground = zt + 0.5;
    let (cover_h, frame) = (0.11, 0.74);
    // the shaft: x along the pipe, the back at y = 0
    let (xo, yo, t) = (0.7, -1.1, 0.1);
    let (xi, yi0, yi1) = (xo - t, yo + t, -t);
    for (name, l, grow, lo, hi) in [
        ("Gravel", &gravel, 0.5, 0.0, gv),
        ("Lean concrete", &lean, 0.39, gv, gv + ln),
        ("Sand bed", &sand, 0.27, gv + ln, z0),
    ] {
        b.add(
            name,
            l.clone(),
            cuboid(v3(-xo - grow, yo - grow, lo), v3(xo + grow, grow, hi)),
            false,
        );
    }
    b.add(
        "Shaft floor",
        concrete.clone(),
        cuboid(v3(-xo, yo, z0), v3(xo, 0.0, zf)),
        false,
    );
    for (lo, hi) in [
        (v3(-xo, yo, zf), v3(xo, yi0, zr)),
        (v3(-xo, yi1, zf), v3(xo, 0.0, zr)),
        (v3(-xo, yi0, zf), v3(-xi, yi1, zr)),
        (v3(xi, yi0, zf), v3(xo, yi1, zr)),
    ] {
        b.add("Shaft wall", concrete.clone(), cuboid(lo, hi), false);
    }
    // the roof round its 600 × 600 opening
    let (ox0, ox1, oy0, oy1) = (-xi + 0.45, -xi + 1.05, yi1 - 0.6, yi1);
    for (lo, hi) in [
        (v3(-xo, yo, zr), v3(ox0, 0.0, zt)),
        (v3(ox1, yo, zr), v3(xo, 0.0, zt)),
        (v3(ox0, yo, zr), v3(ox1, oy0, zt)),
        (v3(ox0, oy1, zr), v3(ox1, 0.0, zt)),
    ] {
        b.add("Shaft roof", concrete.clone(), cuboid(lo, hi), false);
    }
    // the riser up to the cover, 100 thick
    let zc = ground - cover_h;
    for (lo, hi) in [
        (v3(ox0 - t, oy0 - t, zt), v3(ox1 + t, oy0, zc)),
        (v3(ox0 - t, oy1, zt), v3(ox1 + t, oy1 + t, zc)),
        (v3(ox0 - t, oy0, zt), v3(ox0, oy1, zc)),
        (v3(ox1, oy0, zt), v3(ox1 + t, oy1, zc)),
    ] {
        b.add("Concrete riser", concrete.clone(), cuboid(lo, hi), false);
    }
    let m = (frame - 0.6) / 2.0;
    for (lo, hi) in [
        (v3(ox0 - m, oy0 - m, zc), v3(ox1 + m, oy0, ground)),
        (v3(ox0 - m, oy1, zc), v3(ox1 + m, oy1 + m, ground)),
        (v3(ox0 - m, oy0, zc), v3(ox0, oy1, ground)),
        (v3(ox1, oy0, zc), v3(ox1 + m, oy1, ground)),
    ] {
        b.add("Cover frame", iron.clone(), cuboid(lo, hi), false);
    }
    b.add(
        "Cover 600 × 600",
        iron.clone(),
        cuboid(v3(ox0, oy0, ground - 0.04), v3(ox1, oy1, ground)),
        false,
    );
    // the steps on the wall away from the pipe's way in
    for z in [0.33, 0.65, 0.98, 1.29, 1.59] {
        let z = z0 + z;
        b.add(
            "Step",
            steel.clone(),
            cuboid(v3(xi - 0.15, -0.48, z), v3(xi - 0.13, -0.18, z + 0.025)),
            false,
        );
        for y in [-0.48, -0.2] {
            b.add(
                "Step",
                steel.clone(),
                cuboid(v3(xi - 0.15, y, z), v3(xi, y + 0.02, z + 0.025)),
                false,
            );
        }
    }
    // the service pipe and the meter assembly, 400 over the floor, 150
    // from the front
    let (ya, za) = (yi0 + 0.15, zf + 0.4);
    let at = |x: f64| v3(x, ya, za);
    let pe = plastic("PE 100, blue stripe", [24, 30, 38]);
    let brass = metal("Brass", [196, 160, 92], 0.3);
    let pipe = metal("Steel pipe 32 × 3,0, galvanised", [170, 174, 178], 0.5);
    let red = plastic("Handle, red", [190, 40, 36]);
    let out = xo + 0.3;
    b.add(
        "Service pipe in, PE D32",
        pe.clone(),
        rod(at(-out), at(-xi), 0.016),
        false,
    );
    b.add(
        "Service pipe out, PE D32",
        pe,
        rod(at(xi), at(out), 0.016),
        false,
    );
    let x = |d: f64| -xi + d; // from the inner face of the wall it comes in by
    for (name, l, a, len, r) in [
        ("Transition plastic-steel", &brass, 0.0, 0.1, 0.022),
        ("Steel pipe DN 25", &pipe, 0.1, 0.14, 0.016),
        ("Ball valve DN 25, PN 16", &brass, 0.24, 0.08, 0.026),
        ("Steel pipe DN 25", &pipe, 0.32, 0.06, 0.016),
        ("Reducer 25/20", &brass, 0.38, 0.04, 0.02),
        (
            "Water meter DN 20, wet dial, Qn 2,5 m³/h",
            &brass,
            0.42,
            0.19,
            0.04,
        ),
        ("Reducer 20/25", &brass, 0.61, 0.04, 0.02),
        ("Steel pipe DN 25", &pipe, 0.65, 0.32, 0.016),
        ("Ball valve DN 25, PN 16", &brass, 0.97, 0.08, 0.026),
        ("Steel pipe DN 25", &pipe, 1.05, 0.05, 0.016),
        ("Transition steel-plastic", &brass, 1.1, 0.1, 0.022),
    ] {
        b.add(name, l.clone(), rod(at(x(a)), at(x(a + len)), r), false);
    }
    b.add(
        "Water meter register",
        plastic("Register, blue", [30, 80, 170]),
        cylinder(v2(x(0.515), ya), 0.045, za + 0.035, za + 0.08),
        false,
    );
    for hx in [x(0.28), x(1.01)] {
        b.add(
            "Valve handle",
            red.clone(),
            cuboid(
                v3(hx - 0.01, ya - 0.06, za + 0.03),
                v3(hx + 0.01, ya + 0.01, za + 0.045),
            ),
            false,
        );
    }
    connection(&mut b, "SV", 32, v3(-out, ya, za + 0.016));
    connection(&mut b, "SV", 32, v3(out, ya, za + 0.016));
    // plan, as a site plan draws it: the walls dashed, the cover thin and
    // solid, in the water main's colour
    let rect =
        |x0: f64, y0: f64, x1: f64, y1: f64| vec![v2(x0, y0), v2(x1, y0), v2(x1, y1), v2(x0, y1)];
    b.net(rect(-xo, yo, xo, 0.0), "V", true);
    b.net(rect(-xi, yi0, xi, yi1), "V", true);
    b.net(rect(ox0, oy0, ox1, oy1), "V", false);
    // the gravel's overhang behind it to y = 0
    b.shift(v3(0.0, -0.5, 0.0));
    b.at(-ground);
    b.finish(
        "site-water-meter-shaft",
        "Water meter shaft 1200 × 900 × 1800",
        "Water meter shaft, watertight monolithic concrete, inside 1200 × 900 × 1800 (walls 100, floor 150, roof 120, its top 500 under the ground), concrete riser and cover 600 × 600 on the ground, five plastic-coated steps; on a 30 mm sand bed, 150 mm of lean concrete and 120 mm of gravel. The meter assembly 400 above its floor: transition plastic-steel, ball valve DN 25 PN 16, reducer 25/20, wet-dial water meter DN 20 (Qn 2,5 m³/h, Qmax 5 m³/h, 30 °C, PN 16), reducer 20/25, ball valve DN 25 PN 16, transition steel-plastic; the service pipe PE D32 in and out through its side walls",
        ObjectCategory::Site,
        ("IfcDistributionChamberElement", "METERCHAMBER"),
        Mount::Floor,
    )
}

/// A Wavin Tegra inspection chamber `dia` round (0.425 or 0.6), `depth`
/// deep: the base with its channel in and out (DN 160), the riser pipe
/// and the telescopic cover.
pub fn tegra(dia: f64, depth: f64) -> CatalogueObject {
    let mut b = Build::new();
    let mm = (dia * 1000.0).round() as u32;
    let r = dia / 2.0;
    let wall = 0.02;
    let c = v2(0.0, -r - 0.12);
    let pp = plastic("PP, Wavin orange-brown", [176, 92, 46]);
    // the base: a solid bottom with the channel's benching, in and out
    b.add(
        "Chamber base",
        pp.clone(),
        cylinder(c, r + 0.04, 0.0, 0.32),
        false,
    );
    ring(
        &mut b,
        "Riser pipe",
        &pp,
        (c, r - wall, r),
        (0.32, depth - 0.1),
        24,
    );
    b.add(
        "Telescopic cover",
        metal("Cast iron, cover", [50, 50, 54], 0.7),
        cylinder(c, r.min(0.33), depth - 0.12, depth),
        false,
    );
    let ka = plastic("PVC-U KG, orange", [196, 104, 48]);
    let z = 0.16;
    b.add(
        "Sewer pipe in, DN 160",
        ka.clone(),
        rod(v3(c.x - r - 0.3, c.y, z), v3(c.x - r, c.y, z), 0.08),
        false,
    );
    b.add(
        "Sewer pipe out, DN 160",
        ka,
        rod(v3(c.x + r, c.y, z), v3(c.x + r + 0.3, c.y, z), 0.08),
        false,
    );
    connection(&mut b, "K", 150, v3(c.x - r - 0.3, c.y, z + 0.08));
    connection(&mut b, "K", 150, v3(c.x + r + 0.3, c.y, z + 0.08));
    b.outline(squircle(c, r + 0.04, r + 0.04, 2.0, 40));
    b.outline(squircle(c, r - wall, r - wall, 2.0, 40));
    b.line(v2(c.x - r - 0.3, c.y), v2(c.x + r + 0.3, c.y));
    b.at(-depth);
    b.finish(
        &format!("site-tegra-{mm}"),
        &format!("Wavin Tegra {mm} inspection chamber"),
        &format!("Wavin Tegra {mm} plastic inspection chamber, Ø {mm} mm, {depth:.1} m deep: base with the channel in and out DN 160, riser pipe, telescopic cover on the ground; for a building's sewer connection"),
        ObjectCategory::Site,
        ("IfcDistributionChamberElement", "INSPECTIONCHAMBER"),
        Mount::Floor,
    )
}

/// The electricity meter pillar at the plot's edge: a plastic pillar
/// with the meter box (RE) at eye height and the main fuse box under it,
/// its foot in the ground; the cable comes in from below.
pub fn electricity_pillar() -> CatalogueObject {
    let mut b = Build::new();
    let (w, d, h, foot) = (0.6, 0.32, 1.3, 0.6);
    let grey = plastic("Laminate, grey", [196, 198, 196]);
    b.add(
        "Pillar foot",
        plastic("Plastic, black", [36, 36, 38]),
        cuboid(
            v3(-w / 2.0 + 0.02, -d + 0.02, 0.0),
            v3(w / 2.0 - 0.02, -0.02, foot),
        ),
        false,
    );
    b.add(
        "Pillar",
        grey.clone(),
        cuboid(v3(-w / 2.0, -d, foot), v3(w / 2.0, 0.0, foot + h)),
        true,
    );
    b.add(
        "Roof",
        grey,
        cuboid(
            v3(-w / 2.0 - 0.02, -d - 0.02, foot + h),
            v3(w / 2.0 + 0.02, 0.0, foot + h + 0.03),
        ),
        false,
    );
    let door = plastic("Door, grey", [176, 178, 176]);
    b.add(
        "Meter box door",
        door.clone(),
        cuboid(
            v3(-0.26, -d - 0.008, foot + 0.65),
            v3(0.26, -d, foot + 1.22),
        ),
        false,
    );
    b.add(
        "Fuse box door",
        door,
        cuboid(v3(-0.26, -d - 0.008, foot + 0.08), v3(0.26, -d, foot + 0.6)),
        false,
    );
    b.add(
        "Meter window",
        look("Glass", "glass", Hatch::Empty, [150, 190, 210], 0.05),
        cuboid(
            v3(-0.08, -d - 0.012, foot + 0.95),
            v3(0.08, -d - 0.008, foot + 1.1),
        ),
        false,
    );
    b.line(v2(-w / 2.0, -d), v2(w / 2.0, 0.0));
    b.line(v2(-w / 2.0, 0.0), v2(w / 2.0, -d));
    b.at(-foot);
    b.finish(
        "site-electricity-pillar",
        "Electricity meter pillar",
        "Electricity meter pillar (pilierový rozvádzač) 600 × 320, 1.3 m above the ground: the meter box with its window at eye height, the main fuse box under it, the foot 0.6 m in the ground where the cable comes in",
        ObjectCategory::Site,
        ("IfcElectricDistributionBoard", "DISTRIBUTIONBOARD"),
        Mount::Floor,
    )
}

/// The gas meter box on its pillar: the main valve (HUP), the pressure
/// regulator and the meter G4 inside; the service pipe in from below, the
/// pipe to the house out at the side.
pub fn gas_meter_box() -> CatalogueObject {
    let mut b = Build::new();
    let (w, d) = (0.6, 0.32);
    let (z0, z1) = (0.4, 1.15);
    let grey = plastic("Laminate, grey", [196, 198, 196]);
    b.add(
        "Pillar",
        plastic("Concrete, pillar", [150, 150, 146]),
        cuboid(v3(-w / 2.0, -d, 0.0), v3(w / 2.0, 0.0, z0)),
        false,
    );
    b.add(
        "Box",
        grey.clone(),
        cuboid(v3(-w / 2.0, -d, z0), v3(w / 2.0, -0.02, z1)),
        true,
    );
    b.add(
        "Box back",
        grey,
        cuboid(v3(-w / 2.0, -0.02, z0), v3(w / 2.0, 0.0, z1)),
        false,
    );
    b.add(
        "Door, yellow mark",
        plastic("Door, yellow", [226, 190, 40]),
        cuboid(v3(-0.06, -d - 0.006, z1 - 0.12), v3(0.06, -d, z1 - 0.06)),
        false,
    );
    let steel = metal("Steel, yellow", [214, 176, 40], 0.4);
    let brass = metal("Brass", [196, 160, 92], 0.3);
    let y = -d / 2.0;
    b.add(
        "Service pipe in, PE 32",
        plastic("PE 100, yellow", [220, 180, 30]),
        rod(v3(-0.18, y, 0.0), v3(-0.18, y, z0 + 0.12), 0.016),
        false,
    );
    b.add(
        "Main valve (HUP)",
        brass.clone(),
        rod(v3(-0.18, y, z0 + 0.12), v3(-0.18, y, z0 + 0.22), 0.025),
        false,
    );
    b.add(
        "Pressure regulator",
        metal("Regulator, grey", [120, 124, 130], 0.5),
        cylinder(v2(-0.18, y), 0.06, z0 + 0.22, z0 + 0.34),
        false,
    );
    b.add(
        "Pipe to the meter",
        steel.clone(),
        rod(v3(-0.18, y, z0 + 0.34), v3(-0.18, y, z0 + 0.52), 0.014),
        false,
    );
    b.add(
        "Gas meter G4",
        metal("Meter, grey", [200, 200, 196], 0.5),
        cuboid(v3(-0.1, y - 0.08, z0 + 0.25), v3(0.12, y + 0.08, z0 + 0.6)),
        false,
    );
    b.add(
        "Pipe out",
        steel,
        rod(v3(0.12, y, z0 + 0.5), v3(w / 2.0 + 0.1, y, z0 + 0.5), 0.014),
        false,
    );
    connection(&mut b, "P", 25, v3(-0.18, y, 0.02));
    connection(&mut b, "P", 25, v3(w / 2.0 + 0.1, y, z0 + 0.514));
    b.line(v2(-w / 2.0, -d), v2(w / 2.0, 0.0));
    b.finish(
        "site-gas-meter-box",
        "Gas meter box with main valve",
        "Gas meter box 600 × 320 × 750 on a pillar at the plot's edge: the main valve (HUP), the pressure regulator and the meter G4 inside, the service pipe PE 32 in from below, the pipe to the house out at the side",
        ObjectCategory::Site,
        ("IfcFlowMeter", "GASMETER"),
        Mount::Floor,
    )
}

/// A round wall like [`ring`] that narrows from `(r0, r1)` at `z.0` to
/// `(s0, s1)` at `z.1`: a cone.
fn cone(
    b: &mut Build,
    name: &str,
    l: &Look,
    c: DVec2,
    (r0, r1, s0, s1): (f64, f64, f64, f64),
    z: (f64, f64),
    n: usize,
) {
    for i in 0..n {
        let (a0, a1) = (TAU * i as f64 / n as f64, TAU * (i + 1) as f64 / n as f64);
        let p = |r: f64, a: f64| c + DVec2::from_angle(a) * r;
        b.add(
            name,
            l.clone(),
            super::stack(&[
                (vec![p(r0, a0), p(r1, a0), p(r1, a1), p(r0, a1)], z.0),
                (vec![p(s0, a0), p(s1, a0), p(s1, a1), p(s0, a1)], z.1),
            ]),
            false,
        );
    }
}

/// The domestic pumping shaft of the ZTI-06 sample, for a house on a
/// pressure sewer: the plastic sump LAURA "L" (inside Ø 1000, outside
/// Ø 1060), 2700 from the ground to its foot, with the pump NORIA LUCA
/// (with a cutting device) at the bottom, its riser with the check valve,
/// the safety valve and the ball valve, the level sensor; the gravity
/// sewer from the house DN 160 in 1630 under the ground (2 %), the
/// pressure pipe HDPE100 40 × 3.7 out to a ball valve with its trench set
/// in the ground beside it. On 150 of lean concrete and a 200 gravel
/// drainage layer, the pit 400 wider each side. The sizes the drawing
/// does not give (the cone, the collar, the lid, the pump) are read off
/// it by proportion; its control unit hangs on the house and is not here.
pub fn pumping_shaft() -> CatalogueObject {
    let mut b = Build::new();
    let pe = plastic("PE, LAURA black", [44, 46, 48]);
    let lid = plastic("PE, lid, green", [70, 96, 64]);
    let lean = look(
        "Lean concrete",
        "concrete",
        Hatch::Concrete,
        [196, 194, 186],
        0.9,
    );
    let gravel = look("Gravel", "stone", Hatch::Diagonal, [190, 180, 160], 0.95);
    let steel = metal("Stainless steel", [170, 172, 176], 0.4);
    let brass = metal("Brass", [196, 160, 92], 0.3);
    let ka = plastic("PVC-U KG, orange", [196, 104, 48]);
    let hdpe = plastic("HDPE100, black", [30, 30, 32]);
    // heights from the bottom of the gravel
    let (gv, ln) = (0.2, 0.15);
    let zb = gv + ln; // the sump's foot
    let zg = zb + 2.7; // the ground
    let (ri, ro) = (0.5, 0.53);
    let pit = ro + 0.4;
    let c = v2(0.0, -pit);
    let sq = |h: f64| (v3(c.x - h, c.y - h, 0.0), v3(c.x + h, c.y + h, 0.0));
    let (lo, hi) = sq(pit);
    b.add(
        "Gravel drainage layer",
        gravel,
        cuboid(lo, v3(hi.x, hi.y, gv)),
        false,
    );
    let (lo, hi) = sq(ro);
    b.add(
        "Lean concrete",
        lean,
        cuboid(v3(lo.x, lo.y, gv), v3(hi.x, hi.y, zb)),
        false,
    );
    // the sump: its foot, the dished bottom, the ribbed body, the cone,
    // the collar and the lid
    ring(
        &mut b,
        "Sump foot",
        &pe,
        (c, 0.32, 0.35),
        (zb, zb + 0.1),
        32,
    );
    b.add(
        "Sump bottom",
        pe.clone(),
        cylinder(c, 0.35, zb + 0.1, zb + 0.13),
        false,
    );
    cone(
        &mut b,
        "Sump bottom",
        &pe,
        c,
        (0.32, 0.35, ri, ro),
        (zb + 0.1, zb + 0.44),
        32,
    );
    ring(
        &mut b,
        "Sump flange",
        &pe,
        (c, ro, ro + 0.07),
        (zb + 0.41, zb + 0.47),
        32,
    );
    let zc = zg - 1.3; // the cone's foot
    ring(&mut b, "Sump body", &pe, (c, ri, ro), (zb + 0.44, zc), 32);
    cone(
        &mut b,
        "Sump cone",
        &pe,
        c,
        (ri, ro, 0.335, 0.365),
        (zc, zg - 0.55),
        32,
    );
    ring(
        &mut b,
        "Collar flange",
        &pe,
        (c, 0.3, 0.425),
        (zg - 0.55, zg - 0.5),
        32,
    );
    ring(
        &mut b,
        "Collar",
        &pe,
        (c, 0.39, 0.42),
        (zg - 0.5, zg + 0.05),
        32,
    );
    b.add("Lid", lid, cylinder(c, 0.46, zg + 0.05, zg + 0.1), false);
    // the gravity sewer from the house, from the left
    let zi = zg - 1.63;
    b.add(
        "Sewer pipe in, DN 160",
        ka,
        rod(v3(-pit, c.y, zi), v3(c.x - ri, c.y, zi), 0.08),
        false,
    );
    connection(&mut b, "K", 150, v3(-pit, c.y, zi + 0.08));
    // the pump at the bottom, its riser and the pressure pipe to the right
    let zp = zb + 0.16;
    b.add(
        "Pump NORIA LUCA with cutting device",
        steel.clone(),
        cylinder(v2(c.x + 0.04, c.y), 0.09, zp, zp + 0.37),
        false,
    );
    let zo = zg - 1.53;
    b.add(
        "Riser",
        steel.clone(),
        rod(v3(c.x, c.y, zp + 0.37), v3(c.x, c.y, zo + 0.1), 0.02),
        false,
    );
    b.add(
        "Safety valve",
        brass.clone(),
        cylinder(c, 0.025, zo + 0.1, zo + 0.2),
        false,
    );
    b.add(
        "Pressure pipe inside",
        steel.clone(),
        rod(v3(c.x, c.y, zo), v3(c.x + ri, c.y, zo), 0.02),
        false,
    );
    for (name, x) in [("Check valve", 0.12), ("Ball valve", 0.3)] {
        b.add(
            name,
            brass.clone(),
            cuboid(
                v3(c.x + x - 0.04, c.y - 0.035, zo - 0.035),
                v3(c.x + x + 0.04, c.y + 0.035, zo + 0.035),
            ),
            false,
        );
    }
    b.add(
        "Level sensor",
        steel,
        rod(
            v3(c.x - 0.28, c.y, zb + 0.52),
            v3(c.x - 0.28, c.y, zg - 0.78),
            0.01,
        ),
        false,
    );
    // outside: the pressure pipe to a ball valve 0.5 off the sump, its
    // trench set's key rod up to the ground
    let xv = c.x + ro + 0.5;
    let out = xv + 0.3;
    b.add(
        "Pressure pipe HDPE100 40 × 3.7",
        hdpe,
        rod(v3(c.x + ri, c.y, zo), v3(out, c.y, zo), 0.02),
        false,
    );
    b.add(
        "Ball valve DN 40",
        brass,
        cuboid(
            v3(xv - 0.06, c.y - 0.04, zo - 0.04),
            v3(xv + 0.06, c.y + 0.04, zo + 0.04),
        ),
        false,
    );
    b.add(
        "Trench set",
        metal("Steel, black", [40, 40, 42], 0.6),
        rod(v3(xv, c.y, zo + 0.04), v3(xv, c.y, zg), 0.015),
        false,
    );
    connection(&mut b, "K", 40, v3(out, c.y, zo + 0.02));
    // plan, as a site plan draws it: a circle in the sewer's colour, its
    // name beside it
    b.net(squircle(c, ro, ro, 2.0, 40), "K", false);
    b.text(c + v2(ro + 0.15, 0.25), "PS PRŠ");
    b.at(-zg);
    b.finish(
        "site-pumping-shaft-laura-l",
        "Domestic pumping shaft LAURA \"L\" with pump NORIA LUCA",
        "Domestic pumping shaft for a pressure sewer: plastic sump LAURA \"L\", inside Ø 1000, outside Ø 1060, 2700 from the ground to its foot, with lid; pump NORIA LUCA with a cutting device, its riser with the check valve, safety valve and ball valve, level sensor; the gravity sewer from the house DN 160 in 1630 under the ground (at least 2 %), the pressure pipe HDPE100 40 × 3.7 out to a ball valve DN 40 with its trench set; on 150 of lean concrete and a 200 gravel drainage layer. The control unit on the house is not part of it",
        ObjectCategory::Site,
        ("IfcDistributionChamberElement", "SUMP"),
        Mount::Floor,
    )
}

/// The line a site plan's legend gives an object from the catalogue
/// (`starter/<key>`), if it has one: what the symbol means.
pub fn legend(catalogue: &str) -> Option<&'static str> {
    Some(match catalogue.trim_start_matches("starter/") {
        "site-water-meter-shaft" => "Proposed water meter shaft with the water meter assembly",
        "site-pumping-shaft-laura-l" => {
            "Proposed domestic pumping shaft fitted with LAURA \"L\" with pump NORIA LUCA"
        }
        _ => return None,
    })
}

pub fn starters() -> Vec<CatalogueObject> {
    vec![
        water_meter_shaft(),
        tegra(0.425, 1.2),
        tegra(0.6, 1.5),
        electricity_pillar(),
        gas_meter_box(),
        pumping_shaft(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{part_connectors, ConnKind};

    #[test]
    fn the_site_objects_join_their_networks() {
        let shaft = water_meter_shaft();
        let c = part_connectors(&shaft.object);
        assert_eq!(c.len(), 2);
        assert!(c.iter().all(|c| c.kind == ConnKind::SV));
        // its lid on the ground: placed its depth below
        assert!((shaft.object.offset + 2.87).abs() < 1e-9);
        let t = tegra(0.6, 1.5);
        assert!(t.name.contains("600"));
        assert!(part_connectors(&t.object)
            .iter()
            .all(|c| c.kind == ConnKind::K));
        let ps = pumping_shaft();
        let c = part_connectors(&ps.object);
        assert_eq!(c.len(), 2);
        assert!(c.iter().all(|c| c.kind == ConnKind::K));
        // the ground 2700 over its foot, on 150 + 200
        assert!((ps.object.offset + 3.05).abs() < 1e-9);
        assert!(legend("starter/site-pumping-shaft-laura-l").is_some());
        assert!(
            crate::dss::object_class("IfcDistributionChamberElement", "SUMP", &ps.key).is_none()
        );
        let wm =
            crate::dss::object_class("IfcDistributionChamberElement", "METERCHAMBER", &shaft.key);
        assert_eq!(wm, Some("XMC001"));
        let gas = gas_meter_box();
        assert!(part_connectors(&gas.object)
            .iter()
            .all(|c| c.kind == ConnKind::P));
    }
}
