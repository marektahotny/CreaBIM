//! A fire main with hose systems, as PilonDesigner's `firewater.ts`
//! (STN EN 671, STN 92 0400; losses as STN EN 806-3): the two hose
//! systems worst placed work at once, so each pipe carries the two
//! largest flows beyond it. A pipe is sized for at most 2.5 m/s and no
//! smaller than its hose systems join; losses are Darcy-Weisbach with
//! Colebrook-White, with a surcharge for fittings; the pressure needed
//! at the inlet is the nozzle's, the losses on the way and the height.

use crate::tree::{self, Tree};
use creabim_core::{ElementId, Model};

/// Hose systems working at once.
pub const SIMULTANEOUS: usize = 2;
/// The greatest velocity, m/s.
pub const V_MAX: f64 = 2.5;
/// The surcharge for local losses (fittings, valves).
pub const LOCAL_FACTOR: f64 = 1.3;
const NU: f64 = 1.31e-6;
const RHO: f64 = 999.7;

/// The loss, Pa/m, and velocity, m/s, of a full pipe (Colebrook-White).
pub fn pipe_loss(q_ls: f64, di_mm: f64, kb_mm: f64) -> (f64, f64) {
    let d = di_mm / 1000.0;
    let a = std::f64::consts::PI * d * d / 4.0;
    let v = q_ls / 1000.0 / a;
    if v <= 0.0 {
        return (0.0, 0.0);
    }
    let re = v * d / NU;
    let mut lam: f64 = 0.02;
    for _ in 0..30 {
        lam = (-2.0 * (2.51 / (re * lam.sqrt()) + kb_mm / 1000.0 / (3.71 * d)).log10()).powi(-2);
    }
    if re < 2300.0 {
        lam = 64.0 / re;
    }
    (lam / d * RHO * v * v / 2.0, v)
}

/// A pipe's result (its worst stretch).
#[derive(Clone, Debug, PartialEq)]
pub struct PipeResult {
    pub id: ElementId,
    pub flow_ls: f64,
    /// Hose systems beyond it.
    pub hydrants: usize,
    pub nominal_mm: f64,
    pub proposed_mm: f64,
    pub velocity_ms: f64,
    /// Pa/m at the proposed size.
    pub r_pa_m: f64,
    /// The loss along it with the surcharge, Pa.
    pub loss_pa: f64,
    pub critical: bool,
    /// The size is the least its hose systems join, not the velocity's.
    pub by_min_dn: bool,
    /// Even the type's largest size runs faster than allowed.
    pub too_fast: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HydrantResult {
    pub id: ElementId,
    pub name: String,
    pub kind: &'static str,
    pub flow_ls: f64,
    /// The pressure it needs at the inlet, kPa.
    pub inlet_kpa: f64,
    pub critical: bool,
}

#[derive(Clone, Debug, Default)]
pub struct FireResult {
    pub hydrants: Vec<HydrantResult>,
    pub pipes: Vec<PipeResult>,
    /// The flow at the inlet, l/s.
    pub flow_ls: f64,
    /// The pressure needed at the inlet, kPa.
    pub required_kpa: f64,
    pub unreached: usize,
}

/// The fire main `code` of the model, calculated.
pub fn run(model: &Model, code: &str) -> Option<FireResult> {
    let t = tree::build(model, code)?;
    Some(calc(&t))
}

fn calc(t: &Tree) -> FireResult {
    let mut out = FireResult {
        unreached: t.unreached.len(),
        ..FireResult::default()
    };
    let hyd: Vec<(f64, f64, f64)> = t
        .terminals
        .iter()
        .map(|h| {
            let d = h.valve.hydrant_def();
            (
                h.valve.flow_lmin.map_or(d.q_ls, |q| q / 60.0),
                d.p_kpa,
                d.min_dn,
            )
        })
        .collect();
    let below = t.terminals_below();
    let flow_of = |b: &[usize]| -> f64 {
        let mut q: Vec<f64> = b.iter().map(|&k| hyd[k].0).collect();
        q.sort_by(|a, b| b.total_cmp(a));
        q.into_iter().take(SIMULTANEOUS).sum()
    };
    let n = t.stretches.len();
    struct R {
        q: f64,
        nom: f64,
        v: f64,
        r: f64,
        dp: f64,
        by_dn: bool,
        too_fast: bool,
    }
    let mut res: Vec<R> = Vec::with_capacity(n);
    for (i, s) in t.stretches.iter().enumerate() {
        let ty = crate::pipe_types::by_id(&s.pipe.pipe_type);
        let kb = ty.map_or(0.0015, |t| t.roughness_mm);
        let q = flow_of(&below[i]);
        let min_dn = below[i].iter().map(|&k| hyd[k].2).fold(0.0, f64::max);
        let sizes: Vec<(f64, f64)> = ty
            .map(|t| t.sizes.iter().map(|z| (z.nominal_mm, z.inner_mm)).collect())
            .unwrap_or_default();
        let fits = |z: &(f64, f64)| pipe_loss(q, z.1, kb).1 <= V_MAX;
        let k = sizes
            .iter()
            .position(|z| z.1 >= min_dn - 3.0 && fits(z))
            .or(sizes.len().checked_sub(1));
        let (nom, di) = k.map_or((s.pipe.nominal_mm, s.pipe.inner_mm), |k| sizes[k]);
        let (r, v) = pipe_loss(q, di, kb);
        res.push(R {
            q,
            nom,
            v,
            r,
            dp: r * s.length() * LOCAL_FACTOR,
            by_dn: min_dn > 0.0 && k.is_some_and(|k| k > 0 && fits(&sizes[k - 1])),
            too_fast: v > V_MAX,
        });
    }
    // the pressure each hose system needs at the inlet
    let z0 = t.root.z;
    let mut crit = vec![false; n];
    let mut worst: Option<(usize, f64)> = None;
    let mut inlet = vec![0.0; hyd.len()];
    let host = |h: usize| t.stretches.iter().position(|s| s.terminals.contains(&h));
    for (h, d) in hyd.iter().enumerate() {
        let mut dp = 0.0;
        let mut at = host(h);
        while let Some(i) = at {
            dp += res[i].dp;
            at = t.stretches[i].parent;
        }
        let kpa = d.1 + dp / 1000.0 + RHO * 9.81 * (t.terminals[h].at.z - z0) / 1000.0;
        inlet[h] = kpa;
        if worst.is_none_or(|w| kpa > w.1) {
            worst = Some((h, kpa));
        }
    }
    if let Some((h, kpa)) = worst {
        out.required_kpa = kpa;
        let mut at = host(h);
        while let Some(i) = at {
            crit[i] = true;
            at = t.stretches[i].parent;
        }
    }
    for (h, term) in t.terminals.iter().enumerate() {
        out.hydrants.push(HydrantResult {
            id: term.id,
            name: term.name.clone(),
            kind: term.valve.hydrant_def().code,
            flow_ls: hyd[h].0,
            inlet_kpa: inlet[h],
            critical: worst.is_some_and(|w| w.0 == h),
        });
    }
    for (id, parts) in t.by_pipe() {
        let big = parts
            .iter()
            .copied()
            .max_by(|&a, &b| {
                res[a]
                    .nom
                    .total_cmp(&res[b].nom)
                    .then(res[a].q.total_cmp(&res[b].q))
            })
            .unwrap_or(parts[0]);
        let x = &res[big];
        out.pipes.push(PipeResult {
            id,
            flow_ls: x.q,
            hydrants: below[big].len(),
            nominal_mm: t.stretches[big].pipe.nominal_mm,
            proposed_mm: x.nom,
            velocity_ms: x.v,
            r_pa_m: x.r,
            loss_pa: parts.iter().map(|&p| res[p].dp).sum(),
            critical: parts.iter().any(|&p| crit[p]),
            by_min_dn: x.by_dn,
            too_fast: parts.iter().any(|&p| res[p].too_fast),
        });
    }
    out.flow_ls = t
        .stretches
        .iter()
        .enumerate()
        .filter(|(_, s)| s.parent.is_none())
        .map(|(i, _)| res[i].q)
        .sum();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::mep::{Pipe, Valve};
    use creabim_core::{Document, Element, ElementKind, Transaction};
    use glam::DVec3;

    #[test]
    fn a_full_pipes_loss_is_colebrook_whites() {
        // 2 l/s in di 32.8 mm steel (k 0.15 mm): about 2.37 m/s
        let (r, v) = pipe_loss(2.0, 32.8, 0.15);
        assert!((v - 2.367).abs() < 0.01, "{v}");
        assert!(r > 1800.0 && r < 2800.0, "{r}");
        assert_eq!(pipe_loss(0.0, 32.8, 0.15), (0.0, 0.0));
    }

    #[test]
    fn the_two_worst_hose_reels_work_at_once() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let sys = creabim_core::mep::system("PV").unwrap();
        let ty = crate::pipe_types::by_id(sys.pipe_type).expect("the fire main's pipe type");
        let pipe = |a: [f64; 3], b: [f64; 3]| {
            let z = &ty.sizes[0];
            Element::new(
                "pipe",
                ElementKind::Pipe(Pipe {
                    level,
                    system: "PV".into(),
                    start: DVec3::from(a),
                    end: DVec3::from(b),
                    pipe_type: ty.id.clone(),
                    nominal_mm: z.nominal_mm,
                    outer_mm: z.outer_mm,
                    inner_mm: z.inner_mm,
                    height_mm: None,
                    insulation_mm: 0.0,
                    insulation_set: false,
                    mark: None,
                }),
            )
        };
        // a riser with a hose reel on each of three floors
        let main = pipe([20.0, 20.0, -1.0], [20.0, 20.0, 0.5]);
        let main_id = main.id;
        let mut tx = Transaction::new("fire").insert(main);
        let mut tops = vec![];
        for k in 0..3 {
            let z = 0.5 + 3.0 * k as f64;
            if k > 0 {
                tx = tx.insert(pipe([20.0, 20.0, z - 3.0], [20.0, 20.0, z]));
            }
            let br = pipe([20.0, 20.0, z], [22.0, 20.0, z + 0.8]);
            let id = br.id;
            tops.push(id);
            tx = tx.insert(br).insert(Element::new(
                format!("H{k}"),
                ElementKind::Valve(Valve::new(id, 1.0, "hydrant")),
            ));
        }
        doc.commit(tx).unwrap();
        let r = run(doc.model(), "PV").expect("fire pipes");
        assert_eq!((r.hydrants.len(), r.unreached), (3, 0));
        // two D25 reels of 1.0 l/s at once
        assert!((r.flow_ls - 2.0).abs() < 1e-9);
        let of = |id| r.pipes.iter().find(|p| p.id == id).unwrap();
        assert!((of(main_id).flow_ls - 2.0).abs() < 1e-9 && of(main_id).hydrants == 3);
        assert!((of(tops[2]).flow_ls - 1.0).abs() < 1e-9);
        assert!(r
            .pipes
            .iter()
            .all(|p| p.velocity_ms <= V_MAX && !p.too_fast));
        // the top reel is the critical one: 200 kPa at the nozzle, the
        // height (8.3 m over the inlet) and the losses
        let top = r.hydrants.iter().find(|h| h.name == "H2").unwrap();
        assert!(top.critical && (r.required_kpa - top.inlet_kpa).abs() < 1e-9);
        let head = RHO * 9.81 * 8.3 / 1000.0;
        assert!(r.required_kpa > 200.0 + head && r.required_kpa < 200.0 + head + 60.0);
        assert!(of(main_id).critical && of(tops[2]).critical && !of(tops[0]).critical);
    }
}
