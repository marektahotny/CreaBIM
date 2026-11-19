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

/// A plastic water meter shaft 1200 × 900, 1.6 m deep, with the meter
/// assembly the water companies ask for: a ball valve, the strainer, the
/// meter, the check valve and a ball valve with drain, on supports; the
/// PE service pipe in and out through its walls, a ladder, the lid.
pub fn water_meter_shaft() -> CatalogueObject {
    let mut b = Build::new();
    let (w, d, h, t) = (1.2, 0.9, 1.6, 0.06);
    let pe = plastic("PE, shaft", [40, 44, 50]);
    let (x0, x1) = (-w / 2.0 - t, w / 2.0 + t);
    let (y0, y1) = (-d - 2.0 * t, 0.0);
    // walls and floor
    b.add(
        "Shaft floor",
        pe.clone(),
        cuboid(v3(x0, y0, 0.0), v3(x1, y1, t)),
        false,
    );
    b.add(
        "Shaft wall",
        pe.clone(),
        cuboid(v3(x0, y0, t), v3(x1, y0 + t, h)),
        false,
    );
    b.add(
        "Shaft wall",
        pe.clone(),
        cuboid(v3(x0, y1 - t, t), v3(x1, y1, h)),
        false,
    );
    b.add(
        "Shaft wall",
        pe.clone(),
        cuboid(v3(x0, y0 + t, t), v3(x0 + t, y1 - t, h)),
        false,
    );
    b.add(
        "Shaft wall",
        pe.clone(),
        cuboid(v3(x1 - t, y0 + t, t), v3(x1, y1 - t, h)),
        false,
    );
    // the neck and its lid, 600 round
    let neck = v2(x0 + t + 0.4, (y0 + y1) / 2.0);
    b.add(
        "Shaft roof",
        pe.clone(),
        cuboid(v3(x0, y0, h - 0.04), v3(neck.x - 0.33, y1, h)),
        false,
    );
    b.add(
        "Shaft roof",
        pe.clone(),
        cuboid(v3(neck.x + 0.33, y0, h - 0.04), v3(x1, y1, h)),
        false,
    );
    b.add(
        "Lid",
        plastic("Lid, green", [46, 92, 52]),
        cylinder(neck, 0.32, h - 0.03, h + 0.02),
        false,
    );
    // the ladder under the neck
    let steel = metal("Steel, galvanised", [170, 174, 178], 0.5);
    let ly = y1 - t - 0.03;
    for x in [neck.x - 0.2, neck.x + 0.2] {
        b.add(
            "Ladder",
            steel.clone(),
            cuboid(v3(x - 0.015, ly - 0.03, t), v3(x + 0.015, ly, h - 0.05)),
            false,
        );
    }
    let mut z = 0.3;
    while z < h - 0.1 {
        b.add(
            "Ladder rung",
            steel.clone(),
            cuboid(
                v3(neck.x - 0.2, ly - 0.03, z),
                v3(neck.x + 0.2, ly - 0.01, z + 0.02),
            ),
            false,
        );
        z += 0.28;
    }
    // the assembly along the shaft, 0.45 above its floor
    let (ya, za) = ((y0 + y1) / 2.0, 0.45);
    let blue = plastic("PE 100, blue stripe", [24, 30, 38]);
    let brass = metal("Brass", [196, 160, 92], 0.3);
    let red = plastic("Handle, red", [190, 40, 36]);
    let at = |x: f64| v3(x, ya, za);
    b.add(
        "Service pipe in, PE 32",
        blue.clone(),
        rod(at(x0 - 0.05), at(-0.42), 0.016),
        false,
    );
    b.add(
        "Service pipe out, PE 32",
        blue.clone(),
        rod(at(0.42), at(x1 + 0.05), 0.016),
        false,
    );
    let mut x = -0.42;
    let mut piece = |b: &mut Build, name: &str, l: &Look, len: f64, r: f64| {
        b.add(name, l.clone(), rod(at(x), at(x + len), r), false);
        x += len;
    };
    piece(&mut b, "Coupling", &brass, 0.05, 0.022);
    piece(&mut b, "Ball valve before the meter", &brass, 0.08, 0.026);
    piece(&mut b, "Strainer", &brass, 0.08, 0.024);
    piece(&mut b, "Water meter", &brass, 0.19, 0.04);
    piece(&mut b, "Check valve", &brass, 0.08, 0.024);
    piece(
        &mut b,
        "Ball valve with drain after the meter",
        &brass,
        0.09,
        0.026,
    );
    piece(&mut b, "Coupling", &brass, 0.05, 0.022);
    let _ = x;
    // the meter's register and the valves' handles
    b.add(
        "Water meter register",
        plastic("Register, blue", [30, 80, 170]),
        cylinder(v2(-0.42 + 0.21 + 0.095, ya), 0.045, za + 0.035, za + 0.08),
        false,
    );
    for hx in [-0.42 + 0.09, -0.42 + 0.54] {
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
    for sx in [-0.3, 0.3] {
        b.add(
            "Pipe support",
            steel.clone(),
            cuboid(
                v3(sx - 0.015, ya - 0.015, t),
                v3(sx + 0.015, ya + 0.015, za - 0.02),
            ),
            false,
        );
    }
    connection(&mut b, "SV", 32, v3(x0 - 0.05, ya, za + 0.016));
    connection(&mut b, "SV", 32, v3(x1 + 0.05, ya, za + 0.016));
    // plan: the shaft, its inside, the neck, the line of the assembly
    b.outline(vec![v2(x0, y0), v2(x1, y0), v2(x1, y1), v2(x0, y1)]);
    b.outline(vec![
        v2(x0 + t, y0 + t),
        v2(x1 - t, y0 + t),
        v2(x1 - t, y1 - t),
        v2(x0 + t, y1 - t),
    ]);
    b.outline(squircle(neck, 0.3, 0.3, 2.0, 32));
    b.line(v2(x0 - 0.05, ya), v2(x1 + 0.05, ya));
    b.outline(squircle(v2(-0.42 + 0.305, ya), 0.06, 0.06, 2.0, 20));
    b.at(-h);
    b.finish(
        "site-water-meter-shaft",
        "Water meter shaft 1200 × 900",
        "Plastic water meter shaft, inside 1200 × 900, 1.6 m deep (frost-free), lid Ø 600 on the ground, ladder; the meter assembly as the water companies ask for it: ball valve, strainer, water meter, check valve, ball valve with drain, on supports; the service pipe PE 32 in and out through its walls 0.45 above its floor",
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

pub fn starters() -> Vec<CatalogueObject> {
    vec![
        water_meter_shaft(),
        tegra(0.425, 1.2),
        tegra(0.6, 1.5),
        electricity_pillar(),
        gas_meter_box(),
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
        assert!((shaft.object.offset + 1.6).abs() < 1e-9);
        let t = tegra(0.6, 1.5);
        assert!(t.name.contains("600"));
        assert!(part_connectors(&t.object)
            .iter()
            .all(|c| c.kind == ConnKind::K));
        let gas = gas_meter_box();
        assert!(part_connectors(&gas.object)
            .iter()
            .all(|c| c.kind == ConnKind::P));
    }
}
