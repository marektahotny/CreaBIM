//! Water vapour diffusion through a build-up by the Glaser method
//! (STN EN ISO 13788, 6.2 and 6.4.1 to 6.4.6): steady one-dimensional
//! heat flow and diffusion, the temperatures at the interfaces (12),
//! their saturation pressures, the line of the partial vapour pressure
//! drawn tight under them on the scale of sd (6.4.5) and the rate of
//! condensation at each interface where it touches (14 to 16). The
//! constants (δ0, R of a sublayer, μ of metal) come from the norm's
//! parameter table; the saturation pressure from STN 73 0540-3 (8), (9).

use crate::thermcheck::Psat;
use creabim_core::fireparams::{number, ParamTable};

/// STN EN ISO 13788 constants: `key`, `value`, `unit`, `text`
/// (`delta0`, `mu_nepriepustne`, `R_max_vrstva`).
pub const CONSTANTS: &str = "kondenzacia_konstanty";

/// The constants of 6.2 and 6.4.1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Constants {
    /// δ0, the diffusion coefficient of water vapour in air,
    /// kg/(m·s·Pa) (6).
    pub delta0: f64,
    /// μ taken for metal sheets and the like.
    pub mu_metal: f64,
    /// A layer of larger R is split into sublayers of at most this,
    /// m²·K/W.
    pub r_max: f64,
}

impl Constants {
    pub fn of(tables: &[ParamTable]) -> Option<Constants> {
        let t = tables.iter().find(|t| t.id == CONSTANTS)?;
        let v = |k: &str| number(t.cell(k, "value")).filter(|x| *x > 0.0);
        Some(Constants {
            delta0: v("delta0")?,
            mu_metal: v("mu_nepriepustne")?,
            r_max: v("R_max_vrstva")?,
        })
    }
}

/// One layer, from the outside in.
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    pub name: String,
    /// Thickness, m.
    pub d: f64,
    /// Thermal resistance, m²·K/W.
    pub r: f64,
    /// Equivalent air layer thickness sd = μ·d, m.
    pub sd: f64,
}

/// The air on both sides.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Conditions {
    pub theta_i: f64,
    /// Relative humidity inside, 0..1.
    pub phi_i: f64,
    pub theta_e: f64,
    /// Relative humidity outside, 0..1.
    pub phi_e: f64,
    pub rsi: f64,
    pub rse: f64,
}

/// An interface between (sub)layers, or a surface.
#[derive(Clone, Debug, PartialEq)]
pub struct Interface {
    /// The layer on its inner side (the layer it is the outer face of);
    /// none for the inner surface.
    pub layer: Option<usize>,
    /// R'n from the outside air to here, Rse included (8), m²·K/W.
    pub r: f64,
    /// s'd,n from the outside surface to here (9), m.
    pub sd: f64,
    /// Depth from the outside surface, m.
    pub x: f64,
    /// θ'n (12), °C.
    pub theta: f64,
    pub psat: f64,
    /// The partial vapour pressure, Pa.
    pub p: f64,
}

/// The vapour flow through a build-up under one set of conditions.
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    /// From the outside surface (first) to the inside surface (last).
    pub interfaces: Vec<Interface>,
    /// R'T (10), m²·K/W, and s'd,T (11), m.
    pub r_total: f64,
    pub sd_total: f64,
    pub pi: f64,
    pub pe: f64,
    /// The vapour flow were nothing to condense (13), kg/(m²·s),
    /// positive outwards.
    pub g: f64,
    /// The interfaces where vapour condenses and its rate there, gc
    /// (14 to 16), kg/(m²·s).
    pub condensation: Vec<(usize, f64)>,
    /// The inner surface's saturation pressure is below pi: water
    /// condenses on the surface itself.
    pub surface: bool,
}

impl Profile {
    /// All the condensation, kg/(m²·s).
    pub fn gc(&self) -> f64 {
        self.condensation.iter().map(|c| c.1).sum()
    }
}

/// The layers split so that none has R above `r_max` (6.4.1), each
/// sublayer with an equal share of R, d and sd; with the index of the
/// layer it is part of.
pub fn split(layers: &[Layer], r_max: f64) -> Vec<(usize, Layer)> {
    let mut out = vec![];
    for (i, l) in layers.iter().enumerate() {
        let n = if r_max > 0.0 && l.r > r_max {
            (l.r / r_max - 1e-9).ceil().max(1.0) as usize
        } else {
            1
        };
        for _ in 0..n {
            out.push((
                i,
                Layer {
                    name: l.name.clone(),
                    d: l.d / n as f64,
                    r: l.r / n as f64,
                    sd: l.sd / n as f64,
                },
            ));
        }
    }
    out
}

/// The profile of `layers` (from the outside in) under `c`.
pub fn profile(
    layers: &[Layer],
    c: &Conditions,
    psat: &Psat,
    k: &Constants,
) -> Result<Profile, String> {
    if layers.is_empty() {
        return Err("no layers".into());
    }
    if let Some(l) = layers
        .iter()
        .find(|l| !(l.r >= 0.0 && l.sd >= 0.0 && l.d >= 0.0))
    {
        return Err(format!("layer {}: R or sd unknown", l.name));
    }
    let subs = split(layers, k.r_max);
    let r_total = c.rse + subs.iter().map(|s| s.1.r).sum::<f64>() + c.rsi;
    let sd_total: f64 = subs.iter().map(|s| s.1.sd).sum();
    if sd_total <= 0.0 {
        return Err("s'd,T = 0".into());
    }
    let (pi, pe) = (c.phi_i * psat.p(c.theta_i), c.phi_e * psat.p(c.theta_e));
    let theta = |r: f64| c.theta_e + r / r_total * (c.theta_i - c.theta_e);
    let mut faces = vec![];
    let (mut r, mut sd, mut x) = (c.rse, 0.0, 0.0);
    let mut push = |layer, r: f64, sd: f64, x: f64| {
        let t = theta(r);
        faces.push(Interface {
            layer,
            r,
            sd,
            x,
            theta: t,
            psat: psat.p(t),
            p: 0.0,
        })
    };
    // the outside surface is the outer face of the outermost layer
    for (i, s) in &subs {
        push(Some(*i), r, sd, x);
        r += s.r;
        sd += s.sd;
        x += s.d;
    }
    push(None, r, sd_total, x);
    // the line tight under psat from (0, pe) to (s'd,T, pi): the lower
    // convex hull of the ends and the inner interfaces (6.4.5)
    let mut pts: Vec<(f64, f64, Option<usize>)> = vec![(0.0, pe, None)];
    for (n, f) in faces.iter().enumerate().take(faces.len() - 1).skip(1) {
        pts.push((f.sd, f.psat, Some(n)));
    }
    pts.push((sd_total, pi, None));
    let mut hull: Vec<(f64, f64, Option<usize>)> = vec![];
    for p in pts {
        // interfaces at one sd (layers without diffusion resistance) act
        // as one: the lowest saturation pressure holds
        if let Some(last) = hull.last_mut() {
            if (p.0 - last.0).abs() < 1e-12 {
                if p.1 < last.1 || (p.2.is_none() && last.2.is_some()) {
                    if hull.len() == 1 {
                        // never move the outside end
                        continue;
                    }
                    hull.pop();
                } else {
                    continue;
                }
            }
        }
        while hull.len() >= 2 {
            let (a, b) = (hull[hull.len() - 2], hull[hull.len() - 1]);
            // b must lie strictly below the line a–p
            let cross = (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
            if cross <= 0.0 {
                hull.pop();
            } else {
                break;
            }
        }
        hull.push(p);
    }
    for f in &mut faces {
        let j = hull
            .windows(2)
            .position(|w| f.sd <= w[1].0 + 1e-12)
            .unwrap_or(hull.len() - 2);
        let (a, b) = (hull[j], hull[j + 1]);
        f.p = if b.0 - a.0 > 1e-12 {
            a.1 + (f.sd - a.0) / (b.0 - a.0) * (b.1 - a.1)
        } else {
            a.1.min(b.1)
        };
    }
    let mut condensation = vec![];
    for w in hull.windows(3) {
        let (a, b, cc) = (w[0], w[1], w[2]);
        let Some(n) = b.2 else { continue };
        let g = k.delta0 * ((cc.1 - b.1) / (cc.0 - b.0) - (b.1 - a.1) / (b.0 - a.0));
        if g > 0.0 {
            condensation.push((n, g));
        }
    }
    let surface = faces.last().is_some_and(|f| f.psat < pi);
    Ok(Profile {
        interfaces: faces,
        r_total,
        sd_total,
        pi,
        pe,
        g: k.delta0 * (pi - pe) / sd_total,
        condensation,
        surface,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // the Magnus forms of STN 73 0540-3 (8), (9), as the norm's table
    // writes them; in use they are read from the project's table
    fn psat() -> Psat {
        let t = creabim_core::fireparams::parse(
            "#table vzorce\n#columns key\tvzorec\n\
             p_sat_plus\t610,5·exp(17,269·θ/(237,3+θ))\n\
             p_sat_minus\t610,5·exp(21,875·θ/(265,5+θ))\n",
        );
        Psat::of(&t).unwrap()
    }

    const K: Constants = Constants {
        delta0: 2e-10,
        mu_metal: 100_000.0,
        r_max: 0.25,
    };

    fn c() -> Conditions {
        Conditions {
            theta_i: 20.0,
            phi_i: 0.5,
            theta_e: -15.0,
            phi_e: 0.84,
            rsi: 0.13,
            rse: 0.04,
        }
    }

    fn l(name: &str, d: f64, lambda: f64, mu: f64) -> Layer {
        Layer {
            name: name.into(),
            d,
            r: d / lambda,
            sd: mu * d,
        }
    }

    #[test]
    fn splits_by_r() {
        let s = split(&[l("eps", 0.1, 0.04, 40.0)], 0.25);
        assert_eq!(s.len(), 10);
        assert!((s.iter().map(|x| x.1.r).sum::<f64>() - 2.5).abs() < 1e-12);
    }

    #[test]
    fn open_outside_does_not_condense() {
        // outside to inside: mineral wool, concrete
        let p = profile(
            &[l("wool", 0.16, 0.04, 1.0), l("rc", 0.2, 1.58, 29.0)],
            &c(),
            &psat(),
            &K,
        )
        .unwrap();
        assert!(p.condensation.is_empty());
        for f in &p.interfaces {
            assert!(f.p <= f.psat + 1e-9);
        }
        assert!((p.interfaces.last().unwrap().p - p.pi).abs() < 1e-9);
        assert!((p.interfaces[0].p - p.pe).abs() < 1e-9);
    }

    #[test]
    fn tight_outside_condenses_behind_it() {
        // a foil outside of wool on a light board: vapour stops at the foil
        let foil = Layer {
            name: "PE".into(),
            d: 0.0002,
            r: 0.0,
            sd: 20.0,
        };
        let p = profile(
            &[
                foil,
                l("wool", 0.16, 0.04, 1.0),
                l("board", 0.0125, 0.21, 10.0),
            ],
            &c(),
            &psat(),
            &K,
        )
        .unwrap();
        assert_eq!(p.condensation.len(), 1);
        let (n, gc) = p.condensation[0];
        // at the foil's inner face, where the wool starts
        assert_eq!(p.interfaces[n].layer, Some(1));
        let f = &p.interfaces[n];
        assert!((f.p - f.psat).abs() < 1e-9);
        // formula (14) by hand
        let want = K.delta0 * ((p.pi - f.p) / (p.sd_total - f.sd) - (f.p - p.pe) / f.sd);
        assert!((gc - want).abs() < 1e-18);
        // what comes in minus what goes out
        let first = &p.interfaces[n];
        let out = K.delta0 * (first.p - p.pe) / first.sd;
        let inn = K.delta0 * (p.pi - first.p) / (p.sd_total - first.sd);
        assert!((inn - out - p.gc()).abs() < 1e-18);
        for f in &p.interfaces {
            assert!(f.p <= f.psat + 1e-9);
        }
    }

    #[test]
    fn temperatures_follow_r() {
        let p = profile(&[l("rc", 0.2, 1.58, 29.0)], &c(), &psat(), &K).unwrap();
        let rt = 0.04 + 0.2 / 1.58 + 0.13;
        assert!((p.r_total - rt).abs() < 1e-12);
        let se = &p.interfaces[0];
        assert!((se.theta - (-15.0 + 0.04 / rt * 35.0)).abs() < 1e-12);
        let si = p.interfaces.last().unwrap();
        assert!((si.theta - (20.0 - 0.13 / rt * 35.0)).abs() < 1e-12);
    }
}
