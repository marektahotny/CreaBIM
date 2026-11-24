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

/// A number as a table writes it, also in the forms "2e-10" and
/// "1,8824·10^-10".
pub fn sci(text: &str) -> Option<f64> {
    let t = text.trim().replace(',', ".").replace(' ', "");
    if let Ok(v) = t.parse::<f64>() {
        return Some(v);
    }
    if let Some((m, e)) = t.split_once("·10^").or_else(|| t.split_once("*10^")) {
        let e = e.replace(['−', '–'], "-");
        return Some(m.parse::<f64>().ok()? * 10f64.powi(e.parse().ok()?));
    }
    number(text)
}

impl Constants {
    pub fn of(tables: &[ParamTable]) -> Option<Constants> {
        let t = tables.iter().find(|t| t.id == CONSTANTS)?;
        let v = |k: &str| sci(t.cell(k, "value")).filter(|x| *x > 0.0);
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

/// STN 73 0540-3 formula (14) constant: `key`, `value` (`delta_a`, δa
/// in s).
pub const SK_CONSTANTS: &str = "difuzia_konstanty";
/// STN 73 0540-3 table 4: `theta_e`, `phi_e`, `p_e_Pa`, `A_tz`, …,
/// `B_tz`, … (durations in 10³ s).
pub const SK_ANNUAL: &str = "rocna_bilancia_vlhkosti";
/// Every layer is cut into this many slices to find the edges of the
/// condensation zone on the continuous psat curve.
const SLICES: usize = 400;

/// One row of the annual balance (STN 73 0540-2 annex B).
#[derive(Clone, Debug, PartialEq)]
pub struct AnnualRow {
    pub theta_e: f64,
    pub phi_e: f64,
    /// pde as table 4 prints it, Pa.
    pub pe: f64,
    /// Duration under an overcast sky tz, s.
    pub t: f64,
    /// Condensation happens in this row (the line touches psat).
    pub condenses: bool,
    /// The zone's edges A (inner) and B (outer): depth from the outside
    /// surface, m; none where nothing condenses all year.
    pub zone: Option<(f64, f64)>,
    pub psat_a: f64,
    pub psat_b: f64,
    /// RdA (inside to A) and RdB (B to outside), m/s⁻¹ (s'd / δa).
    pub rd_a: f64,
    pub rd_b: f64,
    /// ΔMd (B.1), kg/(m²·s).
    pub dmd: f64,
}

impl AnnualRow {
    /// ΔMd·t, kg/m².
    pub fn m(&self) -> f64 {
        self.dmd * self.t
    }
}

/// The annual moisture balance of STN 73 0540-2 annex B.
#[derive(Clone, Debug, PartialEq)]
pub struct Annual {
    /// Column A (θe ≤ −15 °C) or B of table 4.
    pub column: char,
    pub delta_a: f64,
    pub pi: f64,
    pub rows: Vec<AnnualRow>,
    /// Mc (B.2) and Mev (B.3), kg/(m²·a).
    pub mc: f64,
    pub mev: f64,
}

fn slices(layers: &[Layer], n: usize) -> Vec<Layer> {
    let mut out = vec![];
    for l in layers {
        // a layer without thermal resistance has one temperature: its
        // psat is flat over its sd, no slices needed
        let k = if l.r > 0.0 { n } else { 1 };
        for _ in 0..k {
            out.push(Layer {
                name: l.name.clone(),
                d: l.d / k as f64,
                r: l.r / k as f64,
                sd: l.sd / k as f64,
            });
        }
    }
    out
}

/// The annual balance of `layers` (from the outside in) for a site of
/// design temperature `theta_e_site`, inside `theta_i`, `phi_i` (0..1).
/// The zone of each row with condensation is where the tight line
/// touches psat; rows without keep the zone of the coldest row that has
/// one, at their own temperatures.
pub fn annual_sk(
    layers: &[Layer],
    theta_i: f64,
    phi_i: f64,
    rsi: f64,
    rse: f64,
    theta_e_site: f64,
    tables: &[ParamTable],
    psat: &Psat,
) -> Result<Annual, String> {
    let delta_a = tables
        .iter()
        .find(|t| t.id == SK_CONSTANTS)
        .and_then(|t| sci(t.cell("delta_a", "value")))
        .filter(|x| *x > 0.0)
        .ok_or_else(|| format!("{SK_CONSTANTS}: delta_a"))?;
    let tab = tables
        .iter()
        .find(|t| t.id == SK_ANNUAL)
        .ok_or_else(|| SK_ANNUAL.to_string())?;
    // STN 73 0540-3 table 4: column A for θe ≤ −15 °C
    let column = if theta_e_site <= -15.0 + 1e-9 { 'A' } else { 'B' };
    let tz = tab
        .column(&format!("{column}_tz"))
        .ok_or_else(|| format!("{SK_ANNUAL}: {column}_tz"))?;
    let (cphi, cpe) = (
        tab.column("phi_e").ok_or("phi_e")?,
        tab.column("p_e_Pa").ok_or("p_e_Pa")?,
    );
    let fine = slices(layers, SLICES);
    let k = Constants {
        delta0: delta_a,
        mu_metal: f64::INFINITY,
        r_max: f64::INFINITY,
    };
    let mut rows: Vec<(f64, f64, f64, f64, Profile)> = vec![];
    for r in &tab.rows {
        let (Some(te), Some(phi), Some(pe)) = (
            sci(&r[0]),
            r.get(cphi).and_then(|x| sci(x)),
            r.get(cpe).and_then(|x| sci(x)),
        ) else {
            continue;
        };
        let Some(t) = r.get(tz).and_then(|x| sci(x)).filter(|t| *t > 0.0) else {
            continue;
        };
        // pde as printed; φe only for the record
        let c = Conditions {
            theta_i,
            phi_i,
            theta_e: te,
            phi_e: pe / psat.p(te),
            rsi,
            rse,
        };
        rows.push((te, phi, pe, t * 1000.0, profile(&fine, &c, psat, &k)?));
    }
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    let zone_of = |p: &Profile| -> Option<(usize, usize)> {
        let b = p.condensation.first()?.0;
        let a = p.condensation.last()?.0;
        Some((a, b))
    };
    let first = rows.iter().find_map(|r| zone_of(&r.4));
    let pi = phi_i * psat.p(theta_i);
    let mut out = vec![];
    for (te, phi, pe, t, p) in rows {
        let own = zone_of(&p);
        let Some((a, b)) = own.or(first) else {
            out.push(AnnualRow {
                theta_e: te,
                phi_e: phi,
                pe,
                t,
                condenses: false,
                zone: None,
                psat_a: f64::NAN,
                psat_b: f64::NAN,
                rd_a: f64::NAN,
                rd_b: f64::NAN,
                dmd: 0.0,
            });
            continue;
        };
        let (fa, fb) = (&p.interfaces[a], &p.interfaces[b]);
        let rd_a = (p.sd_total - fa.sd) / delta_a;
        let rd_b = fb.sd / delta_a;
        let dmd = (pi - fa.psat) / rd_a - (fb.psat - pe) / rd_b;
        out.push(AnnualRow {
            theta_e: te,
            phi_e: phi,
            pe,
            t,
            condenses: own.is_some(),
            zone: Some((fa.x, fb.x)),
            psat_a: fa.psat,
            psat_b: fb.psat,
            rd_a,
            rd_b,
            dmd,
        });
    }
    let mc = out.iter().map(|r| r.m()).filter(|m| *m > 0.0).sum();
    let mev = -out.iter().map(|r| r.m()).filter(|m| *m < 0.0).sum::<f64>();
    Ok(Annual {
        column,
        delta_a,
        pi,
        rows: out,
        mc,
        mev,
    })
}

/// The limit of STN 73 0540-2 čl. 6.1 for a build-up of kind `key`
/// (`bez_kondenzacie`, `jednoplastova_strecha`, `ostatne`), kg/(m²·a).
pub fn mc_limit(tables: &[ParamTable], key: &str) -> Option<f64> {
    sci(tables
        .iter()
        .find(|t| t.id == crate::thermcheck::CONDENSATION_LIMITS)?
        .cell(key, "mc_max_kg_m2a"))
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
    fn reads_scientific_numbers() {
        assert_eq!(sci("2e-10"), Some(2e-10));
        assert!((sci("1,8824·10^-10").unwrap() - 1.8824e-10).abs() < 1e-24);
        assert_eq!(sci("0,5"), Some(0.5));
    }

    fn table4() -> Vec<ParamTable> {
        // the shape of table 4 with made-up durations (the real ones are
        // read from the user's file)
        creabim_core::fireparams::parse(
            "#table rocna_bilancia_vlhkosti\n\
             #columns theta_e\tphi_e\tp_e_Pa\tA_tz\tA_tj_a\tA_tj_b\tA_spolu\tB_tz\n\
             -15\t84\t139\t500\t\t\t\t400\n\
             -5\t82\t329\t2000\t\t\t\t3000\n\
             15\t73\t1244\t5000\t\t\t\t5000\n\
             #table difuzia_konstanty\n#columns key\tvalue\n\
             delta_a\t0,00000000018824\n",
        )
    }

    #[test]
    fn annual_balance_by_annex_b() {
        let foil = Layer {
            name: "PE".into(),
            d: 0.0002,
            r: 0.0,
            sd: 20.0,
        };
        let layers = [foil, l("wool", 0.16, 0.04, 1.0), l("board", 0.0125, 0.21, 10.0)];
        let a = annual_sk(&layers, 20.0, 0.5, 0.13, 0.04, -15.0, &table4(), &psat()).unwrap();
        assert_eq!(a.column, 'A');
        assert_eq!(a.rows.len(), 3);
        let r0 = &a.rows[0];
        assert!(r0.condenses && r0.dmd > 0.0);
        // B.1 by hand from the row's values
        let want = (a.pi - r0.psat_a) / r0.rd_a - (r0.psat_b - r0.pe) / r0.rd_b;
        assert!((r0.dmd - want).abs() < 1e-18);
        // the zone starts at the foil's inner face and reaches into the
        // wool where psat stays under the line to pi
        let (za, zb) = r0.zone.unwrap();
        eprintln!("zone {zb}..{za}, dmd {}", r0.dmd);
        assert!((zb - 0.0002).abs() < 1e-12 && za > zb && za < 0.0002 + 0.16);
        assert!((r0.rd_b - 20.0 / 1.8824e-10).abs() / r0.rd_b < 1e-12);
        // at 15 °C the zone dries out to the inside
        let warm = a.rows.last().unwrap();
        assert!(!warm.condenses && warm.dmd < 0.0);
        let mc: f64 = a.rows.iter().map(|r| r.m()).filter(|m| *m > 0.0).sum();
        assert!((a.mc - mc).abs() < 1e-15 && a.mev > 0.0);
        // the column follows the site's θe
        let b = annual_sk(&layers, 20.0, 0.5, 0.13, 0.04, -14.0, &table4(), &psat()).unwrap();
        assert_eq!(b.column, 'B');
        assert!((b.rows[0].t - 400_000.0).abs() < 1e-9);
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
