//! Pipe insulation of heating pipes as the makers make it (George
//! 2026-11-27: MIRELON, TUBOLIT): the tubes' inner diameters and wall
//! thicknesses, their order codes, λ at the mean temperature; and the
//! thickness of a product that insulates as well as a thickness asked
//! for at λ 0,035 W/(m·K) (the services' rule, [`creabim_core::mep::
//! heat_insulation_mm`]): the same resistance per metre of pipe,
//! ln(D/d) / (2πλ), the tube's own from its inner diameter (the gap round
//! the pipe left out) at λ of the mean temperature.
//!
//! Sources (copied in the project files, vyrobcovia/izolacia-potrubi.md):
//! - Armacell Tubolit DG Plus, brochure cs-CZ 112026: λd 0,033 at 0 °C,
//!   0,038 at 40 °C for 6–20 mm, λ(θm) = [32,8 + 0,13·θm +
//!   0,0008·(θm − 40)²] / 1000; over 20 mm λ(θm) = (35 + 0,111·θm) /
//!   1000; tubes 2 m, TL-{Ø}/{s}-DGPLUS; B_L-s1,d0; at most 100 °C.
//! - Mirel Vratimov MIRELON PRO, technical sheet of 2. 1. 2026: λD
//!   −20 °C 0,039, 0 °C 0,044, 10 °C 0,046, 20 °C 0,049, 50 °C 0,057,
//!   90 °C 0,069; tubes 2 m; E_L-s3,d2; at most 90 °C.

use serde::{Deserialize, Serialize};

/// A maker's insulation tube.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Product {
    #[default]
    TubolitDgPlus,
    MirelonPro,
}

/// Tubolit DG Plus, standard tubes 2,0 m: (wall, inner diameters; * made
/// to order marked by a negative diameter).
const TUBOLIT: &[(u32, &[i32])] = &[
    (6, &[15, 18, 22, 28, 35]),
    (9, &[-12, 15, 18, 22, 28, 35, 42, 48, 54, 60, 64, 76, 89]),
    (
        13,
        &[-12, 15, 18, 22, 28, 35, 42, 48, 54, 60, 64, 76, 89, 114],
    ),
    (
        20,
        &[15, 18, 22, 28, 35, 42, 48, 54, 60, 64, 76, 89, 110, 114],
    ),
    (25, &[-15, 18, 22, 28, 35, 42, 48, 54, -60, 64, 76, 89, 114]),
    (26, &[-18, -22]),
    (27, &[-15]),
];

/// MIRELON PRO: (inner diameter, walls made).
const MIRELON: &[(u32, &[u32])] = &[
    (6, &[6, 9]),
    (8, &[6, 9]),
    (10, &[6, 9]),
    (12, &[6, 9, 13]),
    (15, &[6, 9, 13, 20]),
    (18, &[6, 9, 13, 20]),
    (20, &[6, 9, 13, 20]),
    (22, &[6, 9, 13, 20, 25]),
    (25, &[6, 9, 13, 20, 25]),
    (28, &[6, 9, 13, 20, 25]),
    (32, &[6, 9, 13, 20, 25]),
    (35, &[6, 9, 13, 20, 25]),
    (40, &[6, 9, 13, 20, 25]),
];

/// λ the services' thicknesses are given at, W/(m·K).
pub const LAMBDA_RULE: f64 = 0.035;

/// A tube of a product picked for a pipe.
#[derive(Clone, Debug, PartialEq)]
pub struct Tube {
    pub product: Product,
    /// Inner diameter and wall, mm.
    pub inner_mm: u32,
    pub wall_mm: u32,
    /// Its name with the size, and its order code.
    pub name: String,
    pub code: String,
    /// Made to order only.
    pub to_order: bool,
    /// λ at the mean temperature, W/(m·K).
    pub lambda: f64,
    /// It insulates as well as the thickness asked for; else it is the
    /// thickest made for the pipe.
    pub enough: bool,
}

impl Product {
    pub const ALL: [Product; 2] = [Product::TubolitDgPlus, Product::MirelonPro];

    pub fn label(self) -> &'static str {
        match self {
            Product::TubolitDgPlus => "Armacell Tubolit DG Plus",
            Product::MirelonPro => "MIRELON PRO",
        }
    }

    /// The hottest medium it takes, °C.
    pub fn max_c(self) -> f64 {
        match self {
            Product::TubolitDgPlus => 100.0,
            Product::MirelonPro => 90.0,
        }
    }

    /// λ at the mean temperature `tm` °C of a tube of `wall_mm`.
    pub fn lambda(self, tm: f64, wall_mm: u32) -> f64 {
        match self {
            Product::TubolitDgPlus if wall_mm <= 20 => {
                (32.8 + 0.13 * tm + 0.0008 * (tm - 40.0).powi(2)) / 1000.0
            }
            Product::TubolitDgPlus => (35.0 + 0.111 * tm) / 1000.0,
            Product::MirelonPro => {
                const T: [(f64, f64); 6] = [
                    (-20.0, 0.039),
                    (0.0, 0.044),
                    (10.0, 0.046),
                    (20.0, 0.049),
                    (50.0, 0.057),
                    (90.0, 0.069),
                ];
                let t = tm.clamp(T[0].0, T[5].0);
                let k = T.windows(2).position(|w| t <= w[1].0).unwrap_or(4);
                let (a, b) = (T[k], T[k + 1]);
                a.1 + (b.1 - a.1) * (t - a.0) / (b.0 - a.0)
            }
        }
    }

    /// The walls it is made in for a pipe of `inner` mm across, thinnest
    /// first, with made to order.
    fn walls(self, inner: u32) -> Vec<(u32, bool)> {
        match self {
            Product::TubolitDgPlus => TUBOLIT
                .iter()
                .filter_map(|(w, ds)| {
                    ds.iter()
                        .find(|d| d.unsigned_abs() == inner)
                        .map(|d| (*w, *d < 0))
                })
                .collect(),
            Product::MirelonPro => MIRELON
                .iter()
                .find(|(d, _)| *d == inner)
                .map_or(vec![], |(_, ws)| ws.iter().map(|w| (*w, false)).collect()),
        }
    }

    /// The inner diameters it is made in, smallest first.
    fn inners(self) -> Vec<u32> {
        let mut v: Vec<u32> = match self {
            Product::TubolitDgPlus => TUBOLIT
                .iter()
                .flat_map(|(_, ds)| ds.iter().map(|d| d.unsigned_abs()))
                .collect(),
            Product::MirelonPro => MIRELON.iter().map(|(d, _)| *d).collect(),
        };
        v.sort_unstable();
        v.dedup();
        v
    }

    /// The tube for a pipe of `outer_mm` that insulates at least as well
    /// as `rule_mm` at [`LAMBDA_RULE`], the medium at `medium_c` in air
    /// at `air_c`: the smallest inner diameter the pipe goes into, the
    /// thinnest wall that keeps the resistance; the thickest made when
    /// none does. None for no insulation or no tube the pipe goes into.
    pub fn tube(self, outer_mm: f64, rule_mm: f64, medium_c: f64, air_c: f64) -> Option<Tube> {
        if rule_mm <= 0.0 {
            return None;
        }
        let inner = self
            .inners()
            .into_iter()
            .find(|d| *d as f64 + 1e-6 >= outer_mm)?;
        let need = ((outer_mm + 2.0 * rule_mm) / outer_mm).ln() / LAMBDA_RULE;
        let tm = (medium_c + air_c) / 2.0;
        let walls = self.walls(inner);
        let d = inner as f64;
        let fits = |w: u32| ((d + 2.0 * w as f64) / d).ln() / self.lambda(tm, w) >= need - 1e-9;
        let found = walls.iter().copied().find(|(w, _)| fits(*w));
        let enough = found.is_some();
        let (wall, to_order) = found.or_else(|| walls.last().copied())?;
        let (name, code) = match self {
            Product::TubolitDgPlus => (
                format!("Tubolit DG Plus {inner} × {wall}"),
                format!("TL-{inner}/{wall}-DGPLUS"),
            ),
            Product::MirelonPro => (format!("MIRELON PRO {inner} × {wall}"), String::new()),
        };
        Some(Tube {
            product: self,
            inner_mm: inner,
            wall_mm: wall,
            name,
            code,
            to_order,
            lambda: self.lambda(tm, wall),
            enough,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_makers_lambdas() {
        let t = Product::TubolitDgPlus;
        assert!(
            (t.lambda(0.0, 9) - 0.0341).abs() < 2e-4,
            "{}",
            t.lambda(0.0, 9)
        );
        assert!((t.lambda(40.0, 9) - 0.038).abs() < 1e-4);
        assert!((t.lambda(40.0, 25) - 0.0394).abs() < 1e-4);
        let m = Product::MirelonPro;
        assert!((m.lambda(50.0, 9) - 0.057).abs() < 1e-9);
        assert!((m.lambda(35.0, 9) - 0.053).abs() < 1e-9);
    }

    #[test]
    fn a_tube_insulates_as_well_as_the_rule() {
        // 16 mm pipe, 13 mm at λ 0,035 asked, 35 °C water in a 20 °C room
        let t = Product::TubolitDgPlus.tube(16.0, 13.0, 35.0, 20.0).unwrap();
        assert_eq!(
            (t.inner_mm, t.code.as_str(), t.enough),
            (18, "TL-18/20-DGPLUS", true)
        );
        // 20 mm at 55 °C: even its thickest, 26 mm to order, does not do it
        let t = Product::TubolitDgPlus.tube(16.0, 20.0, 55.0, 20.0).unwrap();
        assert_eq!((t.wall_mm, t.to_order, t.enough), (26, true, false));
        // MIRELON conducts worse: its thickest for 18 mm is 20 mm
        let m = Product::MirelonPro.tube(16.0, 20.0, 55.0, 20.0).unwrap();
        assert_eq!((m.inner_mm, m.wall_mm, m.enough), (18, 20, false));
        // 40 mm pipe into 42
        let t = Product::TubolitDgPlus.tube(40.0, 40.0, 55.0, 20.0).unwrap();
        assert_eq!(t.inner_mm, 42);
        assert!(Product::TubolitDgPlus.tube(16.0, 0.0, 55.0, 20.0).is_none());
    }
}
