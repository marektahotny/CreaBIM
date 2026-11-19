//! Pipe hydraulics: velocity, the friction factor (Colebrook–White) and
//! the pressure loss per metre; partly filled gravity pipes (drainage).

use crate::Fluid;
use std::f64::consts::PI;

pub const G: f64 = 9.81;

/// A circle's area, m², from its diameter in m.
pub fn area(d: f64) -> f64 {
    PI * d * d / 4.0
}

/// Mean velocity, m/s, of `flow_ls` l/s in a pipe of inner diameter `d` m.
pub fn velocity(flow_ls: f64, d: f64) -> f64 {
    if d <= 0.0 {
        0.0
    } else {
        flow_ls / 1000.0 / area(d)
    }
}

pub fn reynolds(v: f64, d: f64, nu: f64) -> f64 {
    if nu <= 0.0 {
        0.0
    } else {
        v * d / nu
    }
}

/// The Darcy friction factor: 64/Re in laminar flow, else Colebrook–White
/// solved by iteration from the Swamee–Jain estimate. `k` and `d` in m.
pub fn lambda(re: f64, k: f64, d: f64) -> f64 {
    if re < 1e-6 || d <= 0.0 {
        return 0.0;
    }
    if re < 2320.0 {
        return 64.0 / re;
    }
    let rel = k.max(0.0) / d;
    let mut l = 0.25 / (rel / 3.7 + 5.74 / re.powf(0.9)).log10().powi(2);
    for _ in 0..60 {
        let rhs = -2.0 * (rel / 3.71 + 2.51 / (re * l.sqrt())).log10();
        let ln = 1.0 / (rhs * rhs);
        if (ln - l).abs() < 1e-12 {
            l = ln;
            break;
        }
        l = ln;
    }
    l
}

/// The pressure loss by friction, Pa/m.
pub fn r(flow_ls: f64, d: f64, k: f64, f: &Fluid) -> f64 {
    let v = velocity(flow_ls, d);
    if v <= 0.0 {
        return 0.0;
    }
    lambda(reynolds(v, d, f.nu), k, d) / d * f.rho * v * v / 2.0
}

/// The dynamic pressure, Pa.
pub fn dynamic(v: f64, rho: f64) -> f64 {
    rho * v * v / 2.0
}

/// A gravity pipe of diameter `d` m filled to `fill` (0..1) on a `slope`:
/// its flow, l/s, and velocity, m/s (Prandtl–Colebrook).
pub fn partial_fill(d: f64, fill: f64, slope: f64, k: f64, nu: f64) -> (f64, f64) {
    if d <= 0.0 || slope <= 0.0 {
        return (0.0, 0.0);
    }
    let fill = fill.clamp(0.05, 1.0);
    let (a, perim) = if fill >= 0.999 {
        (area(d), PI * d)
    } else {
        let theta = 2.0 * (1.0 - 2.0 * fill).acos();
        (d * d / 8.0 * (theta - theta.sin()), d * theta / 2.0)
    };
    let dh = 4.0 * a / perim;
    let s = (19.62 * dh * slope).sqrt();
    let v = (-2.0 * s * (k / (3.71 * dh) + 2.51 * nu / (dh * s)).log10()).max(0.0);
    (v * a * 1000.0, v)
}

/// The fill at which a gravity pipe carries `flow_ls` (1 when it can't).
pub fn fill_for_flow(d: f64, flow_ls: f64, slope: f64, k: f64, nu: f64) -> f64 {
    let full = partial_fill(d, 0.999, slope, k, nu).0;
    if flow_ls >= full {
        return 1.0;
    }
    let (mut lo, mut hi) = (0.01, 0.999);
    for _ in 0..60 {
        let mid = (lo + hi) / 2.0;
        if partial_fill(d, mid, slope, k, nu).0 < flow_ls {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn laminar_and_turbulent_friction() {
        assert_eq!(lambda(1000.0, 1e-6, 0.02), 0.064);
        // a smooth pipe at Re 1e5: Blasius gives about 0.018
        let l = lambda(1e5, 0.0, 0.02);
        assert!((l - 0.018).abs() < 0.001, "{l}");
    }

    #[test]
    fn copper_15_at_a_quarter_litre() {
        // 0.25 l/s in 13 mm: about 1.9 m/s and some 3–4 kPa/m
        let w = Fluid::water(10.0);
        let v = velocity(0.25, 0.013);
        assert!((v - 1.883).abs() < 0.01, "{v}");
        let r = r(0.25, 0.013, 1.5e-6, &w);
        assert!(r > 3000.0 && r < 4000.0, "{r}");
    }

    #[test]
    fn a_full_pipe_carries_its_capacity() {
        let (q, _) = partial_fill(0.1, 1.0, 0.02, 1e-4, 1.3e-6);
        let f = fill_for_flow(0.1, q * 0.5, 0.02, 1e-4, 1.3e-6);
        assert!(f > 0.4 && f < 0.6, "{f}");
        assert_eq!(fill_for_flow(0.1, q * 2.0, 0.02, 1e-4, 1.3e-6), 1.0);
    }
}
