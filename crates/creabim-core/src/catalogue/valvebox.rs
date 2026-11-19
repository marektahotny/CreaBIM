//! A medical gas valve box (zone shut-off box) made to order, as the Pilon
//! Hydraulika plugin builds its Revit family (`ValveBoxFactory`,
//! `ValveBoxSpec`): one module per gas, 1 to 4 of O2, AIR, VAC and N2O
//! side by side, flush in the wall or on it, the gas coming in on the
//! right and leaving on the left seen at the door, both on top, or with a
//! through box in on top and out at the bottom. Each module has its
//! shut-off valves, the NIST inlet for maintenance and a reserve supply,
//! the pressure switch for the alarm with its gauge, and the gas's
//! colour label (STN EN ISO 7396-1, STN EN 13348). Sizes after the Flídr
//! medical UP range: check them with the maker.

use super::{cuboid, cylinder, look, v2, Build, Look};
use crate::objects::{CatalogueObject, Mount, ObjectCategory};
use crate::{EditMesh, Hatch};
use glam::DVec3;

/// The gases a box can shut, in the order they stand in it.
pub const BOX_GASES: [&str; 4] = ["O2", "AIR", "VAC", "N2O"];

/// The copper sizes it is made for, mm.
pub const BOX_PIPES: [f64; 5] = [18.0, 22.0, 28.0, 35.0, 42.0];

/// What a box is made with.
#[derive(Clone, Debug, PartialEq)]
pub struct ValveBoxConfig {
    /// Which of [`BOX_GASES`] it shuts.
    pub gases: [bool; 4],
    /// On the wall, not flush in it.
    pub wall: bool,
    /// The gas leaves at the bottom (a box on a riser), not on top.
    pub through: bool,
    /// The copper pipe's outside diameter, mm (one of [`BOX_PIPES`]).
    pub pipe_mm: f64,
    /// Its underside above the floor, m (1.0 to 1.8, HTM 02-01).
    pub height: f64,
    /// The distance of a module's inlet and outlet, mm.
    pub spacing_mm: f64,
}

impl Default for ValveBoxConfig {
    /// The plugin's usual box: O2, AIR and VAC, flush, Cu 18, 1 m up.
    fn default() -> Self {
        Self {
            gases: [true, true, true, false],
            wall: false,
            through: false,
            pipe_mm: 18.0,
            height: 1.0,
            spacing_mm: 70.0,
        }
    }
}

impl ValveBoxConfig {
    /// Its gases, in their order in it; O2 alone when none is chosen.
    pub fn gas_list(&self) -> Vec<&'static str> {
        let g: Vec<_> = BOX_GASES
            .iter()
            .zip(self.gases)
            .filter(|(_, on)| *on)
            .map(|(g, _)| *g)
            .collect();
        if g.is_empty() {
            vec!["O2"]
        } else {
            g
        }
    }

    /// The pipe size the box is made for nearest to the one asked.
    pub fn pipe(&self) -> f64 {
        BOX_PIPES
            .iter()
            .copied()
            .min_by(|a, b| {
                (a - self.pipe_mm)
                    .abs()
                    .partial_cmp(&(b - self.pipe_mm).abs())
                    .unwrap()
            })
            .unwrap_or(18.0)
    }

    /// Width, height and depth, mm (`ValveBoxSpec.Size`).
    pub fn size(&self) -> (f64, f64, f64) {
        let n = self.gas_list().len();
        let i = if n <= 2 {
            0
        } else if n == 3 {
            1
        } else {
            2
        };
        if self.wall {
            ([352.0, 472.0, 592.0][i], 432.0, 110.0)
        } else {
            ([300.0, 420.0, 540.0][i], 380.0, 95.0)
        }
    }

    /// Its code as the plugin names its type: `UP 3 – O2-AIR-VAC`.
    pub fn code(&self) -> String {
        let g = self.gas_list();
        format!("UP {} – {}", g.len(), g.join("-"))
    }
}

/// A gas's colour label (STN EN ISO 7396-1, ISO 32).
pub fn gas_rgb(gas: &str) -> [u8; 3] {
    match gas {
        "N2O" => [47, 111, 214],
        "VAC" => [242, 195, 24],
        "AIR" => [40, 40, 40],
        _ => [255, 255, 255],
    }
}

/// A round bar of radius `r` from `a` to `b`, closed and facing out.
pub(super) fn rod(a: DVec3, b: DVec3, r: f64) -> EditMesh {
    let d = b - a;
    let len = d.length();
    let w = d / len;
    let u = w.any_orthonormal_vector();
    let v = w.cross(u);
    let mut m = cylinder(v2(0.0, 0.0), r, 0.0, len);
    for p in &mut m.vertices {
        *p = a + u * p.x + v * p.y + w * p.z;
    }
    m
}

fn mat(name: &str, rgb: [u8; 3], rough: f32) -> Look {
    look(name, "metal", Hatch::Empty, rgb, rough)
}

/// The box as a catalogue object, to place on (or in) a wall.
pub fn valve_box(c: &ValveBoxConfig) -> CatalogueObject {
    let gases = c.gas_list();
    let n = gases.len();
    let d = c.pipe();
    let (w, h, dp) = c.size();
    let mm = 0.001;
    let th = 2.0;
    let r = d / 2.0;
    let wm = (w - 2.0 * th) / n as f64;
    let a = (r + 8.0).max((c.spacing_mm / 2.0).min(wm / 2.0 - 1.8 * r - 4.0));
    // out from the wall (−y) on it, into it (+y) flush
    let (yf, yb) = if c.wall { (-dp, 0.0) } else { (0.0, dp) };
    let yc = (yf + yb) / 2.0;
    let p = |x: f64, y: f64, z: f64| DVec3::new(x * mm, y * mm, z * mm);
    let bx = |x0: f64, x1: f64, y0: f64, y1: f64, z0: f64, hh: f64| {
        cuboid(p(x0, y0, z0), p(x1, y1, z0 + hh))
    };
    let cy = |x: f64, y: f64, z: f64, axis: DVec3, len: f64, rr: f64| {
        let a = p(x, y, z);
        rod(a, a + axis * len * mm, rr * mm)
    };
    let sheet = mat("Sheet steel, white", [240, 241, 242], 0.5);
    let door = mat("Door, white", [250, 250, 250], 0.4);
    let cu = mat("Copper", [184, 115, 51], 0.3);
    let brass = mat("Brass", [196, 160, 60], 0.3);
    let black = mat("Black", [35, 35, 35], 0.5);
    let gauge = mat("Gauge face", [245, 245, 245], 0.3);
    let red = mat("Emergency opening, red", [210, 50, 40], 0.4);
    let mut b = Build::new();
    b.at(c.height);
    let hw = w / 2.0;
    for (name, m) in [
        ("Back", bx(-hw, hw, yb - th, yb, 0.0, h)),
        ("Side", bx(-hw, -hw + th, yf, yb, 0.0, h)),
        ("Side", bx(hw - th, hw, yf, yb, 0.0, h)),
        ("Bottom", bx(-hw, hw, yf, yb, 0.0, th)),
        ("Top", bx(-hw, hw, yf, yb, h - th, th)),
    ] {
        b.add(name, sheet.clone(), m, false);
    }
    for j in 1..n {
        let x = -hw + th + j as f64 * wm;
        b.add(
            "Divider",
            sheet.clone(),
            bx(x - 1.0, x + 1.0, yf + 6.0, yb - th, th, h - 2.0 * th),
            false,
        );
    }
    if !c.wall {
        b.add(
            "Frame",
            sheet.clone(),
            bx(-hw - 15.0, hw + 15.0, yf - 3.0, yf, -15.0, h + 30.0),
            false,
        );
    }
    b.add(
        "Door",
        door,
        bx(-hw + 2.0, hw - 2.0, yf - 5.0, yf - 3.0, 2.0, h - 4.0),
        false,
    );
    b.add(
        "Emergency opening",
        red.clone(),
        bx(
            hw - 40.0,
            hw - 14.0,
            yf - 9.0,
            yf - 5.0,
            h / 2.0 - 20.0,
            40.0,
        ),
        false,
    );
    b.add(
        "Lock",
        black.clone(),
        cy(hw - 27.0, yf - 5.0, h / 2.0 + 35.0, -DVec3::Y, 4.0, 5.0),
        false,
    );
    let up = DVec3::Z;
    for (k, g) in gases.iter().enumerate() {
        let xc = -hw + th + wm * (k as f64 + 0.5);
        let lw = 22f64.min(wm / 2.0 - 6.0);
        b.add(
            &format!("Label {g}"),
            mat(&format!("Label {g}"), gas_rgb(g), 0.5),
            bx(xc - lw, xc + lw, yf - 6.0, yf - 5.0, h - 42.0, 22.0),
            false,
        );
        // in on the right, on top
        b.add(
            &format!("Inlet {g}"),
            cu.clone(),
            cy(xc + a, yc, h, up, 30.0, r),
            false,
        );
        b.add(
            &format!("Connection {g} DN{d:.0}"),
            cu.clone(),
            cy(xc + a, yc, h + 29.0, up, 1.0, r),
            false,
        );
        let v2z = h - 70.0;
        let n3 = h - 125.0;
        let (zx, v4, s5) = if !c.through {
            b.add(
                &format!("Outlet {g}"),
                cu.clone(),
                cy(xc - a, yc, h, up, 30.0, r),
                false,
            );
            b.add(
                &format!("Connection {g} DN{d:.0}"),
                cu.clone(),
                cy(xc - a, yc, h + 29.0, up, 1.0, r),
                false,
            );
            let zx = 45.0;
            b.add("Pipe", cu.clone(), cy(xc + a, yc, zx, up, h - zx, r), false);
            b.add("Pipe", cu.clone(), cy(xc - a, yc, zx, up, h - zx, r), false);
            (zx, h - 125.0, zx + 60.0)
        } else {
            b.add(
                &format!("Outlet {g}"),
                cu.clone(),
                cy(xc - a, yc, -30.0, up, 30.0, r),
                false,
            );
            b.add(
                &format!("Connection {g} DN{d:.0}"),
                cu.clone(),
                cy(xc - a, yc, -30.0, up, 1.0, r),
                false,
            );
            let zx = h * 0.55;
            b.add("Pipe", cu.clone(), cy(xc + a, yc, zx, up, h - zx, r), false);
            b.add("Pipe", cu.clone(), cy(xc - a, yc, 0.0, up, zx, r), false);
            (zx, zx - 70.0, zx - 140.0)
        };
        b.add(
            "Pipe",
            cu.clone(),
            cy(xc - a - r, yc, zx, DVec3::X, 2.0 * a + 2.0 * r, r),
            false,
        );
        // the shut-off valve on the inlet, its red lever
        b.add(
            "Shut-off valve",
            brass.clone(),
            cy(xc + a, yc, v2z - 18.0, up, 36.0, r * 1.7),
            false,
        );
        b.add(
            "Valve lever",
            red.clone(),
            bx(xc + a - 3.0, xc + a + 3.0, yf + 8.0, yc, v2z - 3.0, 6.0),
            false,
        );
        // the NIST inlet towards the door
        b.add(
            "NIST inlet",
            cu.clone(),
            cy(xc + a, yc, n3, -DVec3::Y, dp / 2.0 - 14.0, r * 0.8),
            false,
        );
        b.add(
            "NIST socket",
            brass.clone(),
            cy(xc + a, yf + 22.0, n3, -DVec3::Y, 10.0, r * 1.4),
            false,
        );
        // the outlet valve, the pressure switch and its gauge
        b.add(
            "Outlet valve",
            brass.clone(),
            cy(xc - a, yc, v4 - 18.0, up, 36.0, r * 1.7),
            false,
        );
        b.add(
            "Valve lever",
            red.clone(),
            bx(xc - a - 3.0, xc - a + 3.0, yf + 8.0, yc, v4 - 3.0, 6.0),
            false,
        );
        b.add(
            "Sensor line",
            cu.clone(),
            cy(
                xc - a,
                yc,
                s5,
                DVec3::X,
                22f64.min(wm / 2.0 - 10.0),
                r * 0.7,
            ),
            false,
        );
        b.add(
            "Pressure switch",
            black.clone(),
            bx(
                xc - a + r + 2.0,
                xc - a + r + 14.0,
                yc - 9.0,
                yc + 9.0,
                s5 - 22.0,
                14.0,
            ),
            false,
        );
        let gr = if wm / 2.0 - 30.0 > 10.0 { 16.0 } else { 10.0 };
        b.add(
            "Gauge",
            gauge.clone(),
            cy(
                (xc - a + 38.0).min(xc + wm / 2.0 - 20.0),
                yc - 6.0,
                s5 + 26.0,
                -DVec3::Y,
                12.0,
                gr,
            ),
            false,
        );
        for x in [xc - wm / 2.0 + 14.0, xc - wm / 2.0 + 26.0] {
            b.add(
                "Cable gland",
                black.clone(),
                cy(x, yb - 14.0, h, up, 10.0, 4.0),
                false,
            );
        }
        // the plan: a valve (two triangles point to point) in each module
        let s = (wm / 2.0 - 8.0).min(30.0) * mm;
        let (x, y) = (xc * mm, yc * mm);
        let q = |dx: f64, dy: f64| glam::DVec2::new(x + dx, y + dy);
        b.outline(vec![
            q(-s, -0.7 * s),
            q(-s, 0.7 * s),
            q(s, -0.7 * s),
            q(s, 0.7 * s),
        ]);
    }
    // the plan: the box, and a flush one's frame on the wall's face
    let pr = |x0: f64, y0: f64, x1: f64, y1: f64| {
        vec![
            glam::DVec2::new(x0 * mm, y0 * mm),
            glam::DVec2::new(x1 * mm, y0 * mm),
            glam::DVec2::new(x1 * mm, y1 * mm),
            glam::DVec2::new(x0 * mm, y1 * mm),
        ]
    };
    b.outline(pr(-hw, yf, hw, yb));
    if !c.wall {
        b.outline(pr(-hw - 15.0, yf - 3.0, hw + 15.0, yf));
    }
    let mounting = if c.wall {
        "nástenná"
    } else {
        "podomietková"
    };
    let through = if c.through { " priebežná" } else { "" };
    let key = format!(
        "med-box-{}-{d:.0}-{}{}",
        gases.join("-").to_lowercase(),
        if c.wall { "wall" } else { "flush" },
        if c.through { "-through" } else { "" }
    );
    b.finish(
        &key,
        &format!("Ventilová skrinka {} {mounting}{through}", c.code()),
        &format!(
            "{}: medical gas zone valve box {:.0} × {:.0} × {:.0} mm, {}; for each gas a shut-off valve PN 20, NIST inlet for maintenance and reserve supply, outlet valve, pressure switch for the alarm with its gauge; in on the right, out on the left {} seen at the door, Cu {d:.0} (STN EN ISO 7396-1, STN EN 13348)",
            c.code(),
            w,
            h,
            dp,
            if c.wall { "on the wall" } else { "flush in the wall" },
            if c.through { "at the bottom" } else { "on top" },
        ),
        ObjectCategory::Medical,
        ("IfcValve", "ISOLATING"),
        Mount::Wall,
    )
}

/// The usual box on the wall: O2, AIR and VAC.
pub fn starters() -> Vec<CatalogueObject> {
    vec![valve_box(&ValveBoxConfig {
        wall: true,
        ..Default::default()
    })]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{part_connectors, ConnKind};

    #[test]
    fn a_box_has_a_module_and_an_inlet_and_outlet_for_each_gas() {
        let c = ValveBoxConfig::default();
        assert_eq!(c.code(), "UP 3 – O2-AIR-VAC");
        assert_eq!(c.size(), (420.0, 380.0, 95.0));
        let o = valve_box(&c);
        o.object.validate().unwrap();
        let k: Vec<_> = part_connectors(&o.object).iter().map(|c| c.kind).collect();
        use ConnKind::*;
        assert_eq!(k, vec![O2, O2, AIR, AIR, VAC, VAC]);
        let cs = part_connectors(&o.object);
        // in on the right, out on the left, both 30 mm over the top
        assert!(cs[0].at.x > cs[1].at.x);
        assert!((cs[0].at.z - 0.41).abs() < 1e-9, "{:?}", cs[0]);
        assert_eq!(cs[0].dn, 18.0);
        // flush: in the wall behind its face
        let (lo, hi) = o.object.bounds();
        assert!(lo.y < 0.0 && hi.y > 0.09, "{lo} {hi}");
        assert!((o.object.offset - 1.0).abs() < 1e-12);
        // four gases on the wall, a through box: out at the bottom
        let four = ValveBoxConfig {
            gases: [true; 4],
            wall: true,
            through: true,
            pipe_mm: 21.0,
            ..Default::default()
        };
        assert_eq!(four.size(), (592.0, 432.0, 110.0));
        assert_eq!(four.pipe(), 22.0);
        let cs = part_connectors(&valve_box(&four).object);
        assert_eq!(cs.len(), 8);
        assert!(cs[1].at.z < 0.0 && cs[0].at.z > 0.4);
        assert_eq!(gas_rgb("VAC"), [242, 195, 24]);
    }
}
