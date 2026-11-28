//! KORADO KORALUX LINEAR CLASSIC-M bathroom towel radiators (KLCM):
//! horizontal steel tubes Ø 20 mm between two upright steel profiles
//! 40 × 30 mm, connected from below in the middle at 50 mm spacing,
//! G 1/2" inside, 110 °C and 1.0 MPa at most. The sizes, outputs,
//! exponents, mass, water content and the most an electric heating
//! element may give are KORADO's (technical price list KORALUX, p. 10
//! and p. 22, saved in the project files as korado/koralux-cennik-str10
//! and -str22).
//!
//! Each is fitted as George asked: on the underfloor heating's water as
//! a supplement, with KORADO's ARMATURA HM (valve, lockshield and
//! thermostatic head in one, Z-D040 straight, white) under its middle
//! connection, and an EL07 electric heating element with its own
//! temperature regulator (Z-KT7R-…-10, white, 230 V, IP44) of the most
//! the size allows, for drying towels in summer, off a socket.
//!
//! KORADO's sheets give no count of the tubes; they are drawn evenly
//! spaced.

use super::{appliance_white, cuboid, cylinder, metal, rrect, soft_box, v2, v3, Build};
use crate::objects::{CatalogueObject, Mount, ObjectCategory, Supply};

/// One size: H, L mm; output at 75/65/20 °C W; exponent n; mass kg;
/// water l; the most an electric element may give, W.
struct Size {
    h: u32,
    l: u32,
    qn: f64,
    n: f64,
    mass: f64,
    water: f64,
    el_max: u32,
}

const fn s(h: u32, l: u32, qn: f64, n: f64, mass: f64, water: f64, el_max: u32) -> Size {
    Size {
        h,
        l,
        qn,
        n,
        mass,
        water,
        el_max,
    }
}

/// KORADO, KORALUX LINEAR CLASSIC (-M), technical price list p. 22.
const SIZES: [Size; 20] = [
    s(700, 450, 255.0, 1.2309, 4.4, 2.5, 200),
    s(700, 500, 280.0, 1.2293, 4.7, 2.7, 200),
    s(700, 600, 329.0, 1.2260, 5.4, 3.0, 300),
    s(700, 750, 400.0, 1.2211, 6.3, 3.5, 300),
    s(900, 450, 333.0, 1.2392, 5.9, 3.4, 300),
    s(900, 500, 365.0, 1.2374, 6.3, 3.6, 300),
    s(900, 600, 429.0, 1.2340, 7.2, 4.0, 400),
    s(900, 750, 522.0, 1.2288, 8.5, 4.7, 500),
    s(1220, 450, 462.0, 1.2524, 7.9, 4.5, 400),
    s(1220, 500, 507.0, 1.2505, 8.4, 4.8, 500),
    s(1220, 600, 596.0, 1.2468, 9.6, 5.4, 500),
    s(1220, 750, 726.0, 1.2412, 11.3, 6.3, 700),
    s(1500, 450, 581.0, 1.2514, 9.9, 5.7, 500),
    s(1500, 500, 638.0, 1.2501, 10.6, 6.1, 600),
    s(1500, 600, 750.0, 1.2474, 12.1, 6.9, 700),
    s(1500, 750, 913.0, 1.2433, 14.3, 8.0, 800),
    s(1820, 450, 725.0, 1.2503, 11.9, 6.8, 600),
    s(1820, 500, 795.0, 1.2496, 12.8, 7.3, 700),
    s(1820, 600, 934.0, 1.2481, 14.5, 8.2, 800),
    s(1820, 750, 1138.0, 1.2458, 17.2, 9.7, 1000),
];

/// The heights made, mm.
pub const HEIGHTS: [u32; 5] = [700, 900, 1220, 1500, 1820];
/// The lengths made, mm.
pub const LENGTHS: [u32; 4] = [450, 500, 600, 750];

/// Bottom of a towel radiator above the floor, m.
pub const OFF_FLOOR: f64 = 0.15;

/// Its connection stubs under its bottom, m.
const STUB: f64 = 0.03;

/// One KORALUX LINEAR CLASSIC-M.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TowelRail {
    /// mm.
    pub height: u32,
    /// mm.
    pub length: u32,
}

impl TowelRail {
    fn size(&self) -> Option<&'static Size> {
        SIZES
            .iter()
            .find(|s| s.h == self.height && s.l == self.length)
    }

    /// Every size KORADO makes, smallest output first.
    pub fn all() -> Vec<TowelRail> {
        let mut v: Vec<TowelRail> = SIZES
            .iter()
            .map(|s| TowelRail {
                height: s.h,
                length: s.l,
            })
            .collect();
        v.sort_by(|a, b| a.nominal_w().total_cmp(&b.nominal_w()));
        v
    }

    /// Its output at 75/65/20 °C, W; 0 for a size not made.
    pub fn nominal_w(&self) -> f64 {
        self.size().map_or(0.0, |s| s.qn)
    }

    /// The exponent of its characteristic.
    pub fn exponent(&self) -> Option<f64> {
        self.size().map(|s| s.n)
    }

    /// Its output at the mean temperature difference `dt` K to the room
    /// (STN EN 442-2), W.
    pub fn output_w(&self, dt: f64) -> f64 {
        match self.size() {
            Some(s) if dt > 0.0 => s.qn * (dt / 50.0).powf(s.n),
            _ => 0.0,
        }
    }

    /// Mass kg and water content l.
    pub fn mass_water(&self) -> Option<(f64, f64)> {
        self.size().map(|s| (s.mass, s.water))
    }

    /// Power of its electric heating element: the most KORADO allows for
    /// the size, W.
    pub fn electric_w(&self) -> u32 {
        self.size().map_or(0, |s| s.el_max)
    }

    /// As KORADO writes it: KLCM 1220.600.
    pub fn label(&self) -> String {
        format!("KLCM {}.{}", self.height, self.length)
    }

    /// KORADO's order code: KLC-122060-00M10.
    pub fn code(&self) -> String {
        format!("KLC-{:03}{:03}-00M10", self.height / 10, self.length / 10)
    }

    /// What it is fitted with: code and name.
    pub fn fittings(&self) -> Vec<(String, String)> {
        vec![
            (
                "Z-D040".into(),
                "KORADO ARMATURA HM, priama, biela, s termostatickou hlavicou".into(),
            ),
            (
                format!("Z-KT7R-{:04}-10", self.electric_w()),
                format!(
                    "KORADO elektrické vykurovacie teleso EL07 {} W s regulátorom teploty, biele, 230 V, IP44",
                    self.electric_w()
                ),
            ),
        ]
    }

    /// Its catalogue key.
    pub fn key(&self) -> String {
        format!("koralux-klcm-{}-{}", self.height, self.length)
    }

    /// The towel radiator a catalogue key names; the starter's
    /// `pd-kupelnovy-rebrik` is a KLCM 1220.500.
    pub fn of_key(key: &str) -> Option<TowelRail> {
        let key = key.rsplit('/').next().unwrap_or(key);
        if key == "pd-kupelnovy-rebrik" {
            return Some(TowelRail {
                height: 1220,
                length: 500,
            });
        }
        let mut it = key.strip_prefix("koralux-klcm-")?.split('-');
        let height = it.next()?.parse().ok()?;
        let length = it.next()?.parse().ok()?;
        let t = TowelRail { height, length };
        (it.next().is_none() && t.size().is_some()).then_some(t)
    }

    /// As a catalogue object: its back to the wall on y = 0, 30 mm off
    /// it, its bottom 150 mm off the floor; the water connections in the
    /// middle underneath, 50 mm apart.
    pub fn object(&self) -> CatalogueObject {
        let (w, h) = (self.length as f64 / 1000.0, self.height as f64 / 1000.0);
        let gap = 0.03;
        // the upright profiles 40 wide, 30 deep
        let (pw, pd) = (0.04, 0.03);
        let yc = -gap - pd / 2.0;
        let z0 = STUB;
        let mut b = Build::new();
        for (name, x) in [
            ("Left profile", -w / 2.0 + pw / 2.0),
            ("Right profile", w / 2.0 - pw / 2.0),
        ] {
            b.add(
                name,
                appliance_white(),
                soft_box(
                    v3(x - pw / 2.0, -gap - pd, z0),
                    v3(x + pw / 2.0, -gap, z0 + h),
                    0.012,
                    0.004,
                ),
                false,
            );
        }
        // tubes Ø 20, evenly spaced about 60 mm apart
        let r = 0.01;
        let span = h - 0.08;
        let count = ((span / 0.06).round() as usize).max(2);
        for i in 0..count {
            let z = z0 + 0.04 + span * i as f64 / (count - 1) as f64;
            b.add(
                "Tube",
                appliance_white(),
                cuboid(
                    v3(-w / 2.0 + pw, yc - r, z - r),
                    v3(w / 2.0 - pw, yc + r, z + r),
                ),
                false,
            );
        }
        let brass = metal("Brass", [196, 160, 92], 0.3);
        b.add(
            "Connection UKP DN15",
            brass.clone(),
            cylinder(v2(-0.025, yc), 0.008, 0.0, z0),
            false,
        );
        b.add(
            "Connection UKS DN15",
            brass,
            cylinder(v2(0.025, yc), 0.008, 0.0, z0),
            false,
        );
        for z in [z0 + 0.1_f64.min(h / 3.0), z0 + h - 0.1] {
            for x in [-w / 2.0 + pw / 2.0, w / 2.0 - pw / 2.0] {
                b.add(
                    "Bracket",
                    appliance_white(),
                    cuboid(v3(x - 0.01, -gap, z - 0.01), v3(x + 0.01, 0.0, z + 0.01)),
                    false,
                );
            }
        }
        // in plan: its outline and the line of its tubes
        b.outline(rrect(v2(0.0, -gap - pd / 2.0), w, pd, 0.0, 0));
        b.line(v2(-w / 2.0 + pw, yc), v2(w / 2.0 - pw, yc));
        b.at(OFF_FLOOR - STUB);
        let (mass, water) = self.mass_water().unwrap_or((0.0, 0.0));
        let mut c = b.finish(
            &self.key(),
            &format!("KORADO KORALUX LINEAR CLASSIC-M {}.{}", self.height, self.length),
            &format!(
                "KORADO KORALUX LINEAR CLASSIC-M towel radiator {} × {} mm ({}), {} W at 75/65/20 °C, n = {:.4}, {} kg, {} l; middle bottom connection 50 mm with ARMATURA HM Z-D040; EL07 electric element {} W with regulator (Z-KT7R-{:04}-10)",
                self.height,
                self.length,
                self.code(),
                self.nominal_w().round(),
                self.exponent().unwrap_or(0.0),
                mass,
                water,
                self.electric_w(),
                self.electric_w(),
            ),
            ObjectCategory::Hvac,
            ("IfcSpaceHeater", "RADIATOR"),
            Mount::Wall,
        );
        c.object.supply = Some(Supply::single(self.electric_w() as f64, false));
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_towel_radiator_is_korado_s_size() {
        let t = TowelRail {
            height: 1220,
            length: 600,
        };
        assert_eq!(t.nominal_w(), 596.0);
        assert_eq!(t.electric_w(), 500);
        assert_eq!(t.code(), "KLC-122060-00M10");
        assert_eq!(t.label(), "KLCM 1220.600");
        assert_eq!(TowelRail::of_key(&t.key()), Some(t));
        assert_eq!(TowelRail::of_key("koralux-klcm-1220-550"), None);
        assert_eq!(
            TowelRail::of_key("starter/pd-kupelnovy-rebrik").map(|x| x.length),
            Some(500)
        );
        // KORADO's p. 22: 700.450 at 55/45/20 gives 137 W, 1820.750 609 W
        let small = TowelRail {
            height: 700,
            length: 450,
        };
        assert!((small.output_w(30.0) - 137.0).abs() < 1.5);
        let big = TowelRail {
            height: 1820,
            length: 750,
        };
        // KORADO's own table runs up to 1.2 % above its exponent here
        assert!((big.output_w(30.0) / 609.0 - 1.0).abs() < 0.015);
        assert_eq!(big.fittings()[1].0, "Z-KT7R-1000-10");
        assert_eq!(TowelRail::all().len(), 20);
        let o = t.object();
        let (lo, hi) = o.object.bounds();
        assert!((hi.x - lo.x - 0.6).abs() < 1e-9);
        assert_eq!(o.object.supply.as_ref().unwrap().power_w, Some(500.0));
        let conns = crate::fixture::part_connectors(&o.object);
        assert_eq!(conns.len(), 2);
        assert!((conns[0].at.x - conns[1].at.x).abs() - 0.05 < 1e-9);
    }
}
