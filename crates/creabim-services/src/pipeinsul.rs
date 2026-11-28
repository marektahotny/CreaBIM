//! Pipe insulation of heating pipes as the makers make it (George
//! 2026-11-27: MIRELON, TUBOLIT): the tubes' inner diameters and wall
//! thicknesses, their order codes, λ; and the least thickness of a
//! product by vyhláška MH SR č. 14/2016 Z. z. (Slov-Lex, in force since
//! the first of January 2016): príloha č. 1 gives it at λ 0,035 W/(m·K) at 0 °C by the
//! pipe's inner diameter ([`creabim_core::mep::heat_insulation_mm`]);
//! príloha č. 2 reckons it for any other insulation from the same
//! resistance per metre of pipe, the surface's included (α2 =
//! 10 W/(m²·K), α1 and the pipe's own wall cancel out):
//!
//! ln(dI/dR1′)/(2·λI) + 1/(α2·dI) = ln(dIZ/dR1′)/(2·λIZ) + 1/(α2·dIZ),
//! dI = dR1′ + 2·s(príloha 1), sIZ = (dIZ − dR1′)/2,
//!
//! both λ at 0 °C as the vyhláška gives them. Its 50 % less at
//! manifolds, crossings, joints and penetrations of walls and ceilings
//! (príloha 1 bod 2, príloha 2 bod 2) is not taken: the pipes are whole.
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

/// λ the services' thicknesses are given at, W/(m·K) at 0 °C (vyhláška
/// č. 14/2016 Z. z., § 1).
pub const LAMBDA_RULE: f64 = 0.035;

/// α2 on the insulation's surface, W/(m²·K) (vyhláška č. 14/2016 Z. z.,
/// príloha č. 2 bod 1).
pub const ALPHA_SURFACE: f64 = 10.0;

/// The least thickness, mm, of insulation of `lambda` (at 0 °C) on a
/// pipe of `outer_mm` that insulates as `rule_mm` at [`LAMBDA_RULE`]
/// does (vyhláška č. 14/2016 Z. z., príloha č. 2).
pub fn least_mm(outer_mm: f64, rule_mm: f64, lambda: f64) -> f64 {
    if rule_mm <= 0.0 {
        return 0.0;
    }
    let d = outer_mm / 1000.0;
    let r = |dx: f64, l: f64| (dx / d).ln() / (2.0 * l) + 1.0 / (ALPHA_SURFACE * dx);
    let need = r(d + 2.0 * rule_mm / 1000.0, LAMBDA_RULE);
    // r rises with the diameter beyond 2λ/α2 (some 7 mm): halve between
    let (mut lo, mut hi) = (d.max(2.0 * lambda / ALPHA_SURFACE), d + 2.0);
    if r(lo, lambda) >= need {
        return (lo - d) / 2.0 * 1000.0;
    }
    for _ in 0..100 {
        let m = (lo + hi) / 2.0;
        if r(m, lambda) >= need {
            hi = m;
        } else {
            lo = m;
        }
    }
    (hi - d) / 2.0 * 1000.0
}

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
    /// λ at 0 °C, W/(m·K), and the least thickness it takes, mm
    /// ([`least_mm`]).
    pub lambda: f64,
    pub least_mm: f64,
    /// Its wall is that thick at least; else it is the thickest made for
    /// the pipe.
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
    /// as `rule_mm` at [`LAMBDA_RULE`] (vyhláška č. 14/2016 Z. z.): the
    /// smallest inner diameter the pipe goes into, the thinnest wall of
    /// the least thickness at λ of 0 °C; the thickest made when none is.
    /// None for no insulation, no tube the pipe goes into, or a medium
    /// hotter than the product takes.
    pub fn tube(self, outer_mm: f64, rule_mm: f64, medium_c: f64) -> Option<Tube> {
        if rule_mm <= 0.0 {
            return None;
        }
        let inner = self
            .inners()
            .into_iter()
            .find(|d| *d as f64 + 1e-6 >= outer_mm)?;
        if medium_c > self.max_c() {
            return None;
        }
        let walls = self.walls(inner);
        let least = |w: u32| least_mm(outer_mm, rule_mm, self.lambda(0.0, w));
        let found = walls
            .iter()
            .copied()
            .find(|(w, _)| *w as f64 + 1e-6 >= least(*w));
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
            lambda: self.lambda(0.0, wall),
            least_mm: least(wall),
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
    fn the_least_thickness_of_the_vyhlaska() {
        // λ 0,035: the table's own
        assert!((least_mm(16.0, 20.0, 0.035) - 20.0).abs() < 1e-6);
        // poorer insulation wants more, better less
        let more = least_mm(16.0, 20.0, 0.044);
        let less = least_mm(16.0, 20.0, 0.030);
        assert!(more > 26.0 && more < 40.0, "{more}");
        assert!(less < 20.0 && less > 14.0, "{less}");
        // the resistance is the same
        let r = |s: f64, l: f64| {
            let (d, dx) = (0.016, 0.016 + 2.0 * s / 1000.0);
            (dx / d).ln() / (2.0 * l) + 1.0 / (ALPHA_SURFACE * dx)
        };
        assert!((r(more, 0.044) - r(20.0, 0.035)).abs() < 1e-9);
        assert_eq!(least_mm(16.0, 0.0, 0.044), 0.0);
    }

    #[test]
    fn a_tube_insulates_as_well_as_the_rule() {
        // 16 × 2 pipe (12 inside): 20 mm at λ 0,035; Tubolit 0,0341 at
        // 0 °C wants a little less, its 20 mm wall does
        let t = Product::TubolitDgPlus.tube(16.0, 20.0, 35.0).unwrap();
        assert_eq!(
            (t.inner_mm, t.wall_mm, t.code.as_str(), t.enough),
            (18, 20, "TL-18/20-DGPLUS", true)
        );
        assert!(t.least_mm < 20.0);
        // 32 mm pipe, 30 mm asked: Tubolit's thickest for 35 is 25
        let t = Product::TubolitDgPlus.tube(32.0, 30.0, 55.0).unwrap();
        assert_eq!((t.inner_mm, t.wall_mm, t.enough), (35, 25, false));
        // MIRELON (0,044 at 0 °C) needs more than its 20 mm for 18
        let m = Product::MirelonPro.tube(16.0, 20.0, 55.0).unwrap();
        assert_eq!((m.inner_mm, m.wall_mm, m.enough), (18, 20, false));
        assert!(m.least_mm > 25.0);
        // 40 mm pipe into 42
        let t = Product::TubolitDgPlus.tube(40.0, 34.0, 55.0).unwrap();
        assert_eq!(t.inner_mm, 42);
        assert!(Product::TubolitDgPlus.tube(16.0, 0.0, 55.0).is_none());
        // too hot for MIRELON
        assert!(Product::MirelonPro.tube(16.0, 20.0, 95.0).is_none());
    }
}
