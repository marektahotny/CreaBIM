//! Hot water circulation, as PilonDesigner's `CirculationAnalyzer`
//! (STN EN 806-2, STN 06 0320): the hot water pipes a circulation loop
//! keeps hot lose heat (7 W/m in risers, 10 W/m along), which the
//! circulation must carry back at a 5 K drop: Qc = ΣQloss / (ρ·c·Δt),
//! shared among the branches by their losses. Each loop's end (a
//! circulation pipe ending by a hot water pipe) takes what its branch
//! needs; the circulation is sized for 0.5 m/s and 100 Pa/m, the worst
//! loop and a balancing valve's least loss set the pump, and every other
//! loop's balancing valve takes up the difference.

use crate::fluid::Fluid;
use crate::hydraulics;
use crate::tree::{self, Tree};
use creabim_core::fixture::ConnKind;
use creabim_core::{ElementId, Model};
use glam::DVec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CircSettings {
    pub hot_c: f64,
    pub dt: f64,
    /// Heat lost by insulated pipes, W/m: risers, along.
    pub loss_riser: f64,
    pub loss_along: f64,
    pub vmax: f64,
    pub rmax: f64,
    pub min_inner_mm: f64,
    /// The least loss across a balancing valve, kPa.
    pub valve_min_kpa: f64,
    /// The heater and the check valve, kPa.
    pub heater_kpa: f64,
    /// How far a circulation end may be from the hot water pipe, m.
    pub reach_m: f64,
    pub reserve_pct: f64,
    pub local_share: f64,
}

impl Default for CircSettings {
    fn default() -> Self {
        CircSettings {
            hot_c: 55.0,
            dt: 5.0,
            loss_riser: 7.0,
            loss_along: 10.0,
            vmax: 0.5,
            rmax: 100.0,
            min_inner_mm: 10.0,
            valve_min_kpa: 5.0,
            heater_kpa: 3.0,
            reach_m: 1.0,
            reserve_pct: 10.0,
            local_share: 0.3,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LoopResult {
    /// The circulation pipe it ends on, and the hot water pipe it joins.
    pub circ_pipe: ElementId,
    pub hot_pipe: ElementId,
    pub flow_ls: f64,
    /// Losses along the hot water and back along the circulation, Pa.
    pub hot_pa: f64,
    pub circ_pa: f64,
    /// What its balancing valve takes, kPa, and the kv for it.
    pub valve_kpa: f64,
    pub kv: f64,
    pub critical: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PipeResult {
    pub id: ElementId,
    pub flow_ls: f64,
    pub size: String,
    pub proposed: String,
    pub proposed_mm: f64,
    pub velocity_ms: f64,
    pub r_pa_m: f64,
    pub too_fast: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CircResult {
    pub loops: Vec<LoopResult>,
    pub pipes: Vec<PipeResult>,
    /// The heat the circulated hot water pipes lose, W.
    pub loss_w: f64,
    /// Hot water pipes kept hot, m.
    pub circulated_m: f64,
    pub flow_ls: f64,
    pub pump_kpa: f64,
    pub pump_m: f64,
    /// Circulation ends near no hot water pipe.
    pub loose_ends: usize,
    pub unreached: usize,
}

/// The circulation (`C`) of the hot water (`TV`) in the model.
pub fn run(model: &Model, s: &CircSettings) -> Option<CircResult> {
    let tv = tree::build_objects(model, "TV", ConnKind::TV, true)?;
    let ci = tree::build_objects(model, "C", ConnKind::C, true)?;
    Some(calc(&tv, &ci, s))
}

/// The nearest point of `a`–`b` to `p`, and how far.
fn near(a: DVec3, b: DVec3, p: DVec3) -> f64 {
    let d = b - a;
    let t = ((p - a).dot(d) / d.length_squared().max(1e-12)).clamp(0.0, 1.0);
    (a + d * t).distance(p)
}

fn way(t: &Tree, i: usize) -> Vec<usize> {
    let mut v = vec![];
    let mut at = Some(i);
    while let Some(k) = at {
        v.push(k);
        at = t.stretches[k].parent;
    }
    v
}

fn calc(tv: &Tree, ci: &Tree, s: &CircSettings) -> CircResult {
    let fluid = Fluid::water(s.hot_c);
    let mut out = CircResult {
        unreached: ci.unreached.len(),
        ..CircResult::default()
    };
    let ckids = ci.children();
    let tkids = tv.children();
    // each circulation end and the hot water stretch it joins
    let mut ends: Vec<(usize, usize)> = vec![];
    for (i, st) in ci.stretches.iter().enumerate() {
        if !ckids[i].is_empty() {
            continue;
        }
        let best = (0..tv.stretches.len())
            .map(|k| (k, near(tv.stretches[k].a, tv.stretches[k].b, st.b)))
            .filter(|x| x.1 <= s.reach_m)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        match best {
            Some((k, _)) => ends.push((i, k)),
            None => out.loose_ends += 1,
        }
    }
    if ends.is_empty() {
        return out;
    }
    let n = tv.stretches.len();
    let mut circulated = vec![false; n];
    for &(_, k) in &ends {
        for i in way(tv, k) {
            circulated[i] = true;
        }
    }
    let own: Vec<f64> = (0..n)
        .map(|i| {
            if !circulated[i] {
                return 0.0;
            }
            let st = &tv.stretches[i];
            st.length()
                * if st.is_vertical() {
                    s.loss_riser
                } else {
                    s.loss_along
                }
        })
        .collect();
    out.circulated_m = (0..n)
        .filter(|&i| circulated[i])
        .map(|i| tv.stretches[i].length())
        .sum();
    let mut sub = own.clone();
    for i in (0..n).rev() {
        if let Some(p) = tv.stretches[i].parent {
            if circulated[i] {
                sub[p] += sub[i];
            }
        }
    }
    let roots: Vec<usize> = (0..n)
        .filter(|&i| circulated[i] && tv.stretches[i].parent.is_none())
        .collect();
    out.loss_w = roots.iter().map(|&i| sub[i]).sum();
    let q_tot = fluid.flow_for_power(out.loss_w, s.dt);
    out.flow_ls = q_tot;
    // shared by the branches' losses
    let mut q = vec![0.0; n];
    let rsum: f64 = roots.iter().map(|&i| sub[i]).sum();
    for &r in &roots {
        q[r] = if rsum > 0.0 {
            q_tot * sub[r] / rsum
        } else {
            q_tot / roots.len() as f64
        };
    }
    let joined = |i: usize| ends.iter().any(|e| e.1 == i);
    for i in 0..n {
        if !circulated[i] {
            continue;
        }
        let cs: Vec<usize> = tkids[i].iter().copied().filter(|&c| circulated[c]).collect();
        if cs.is_empty() {
            continue;
        }
        let mine = if joined(i) { own[i].max(1e-6) } else { 0.0 };
        let sum: f64 = cs.iter().map(|&c| sub[c]).sum::<f64>() + mine;
        for &c in &cs {
            q[c] = if sum > 0.0 {
                q[i] * sub[c] / sum
            } else {
                q[i] / (cs.len() + usize::from(mine > 0.0)) as f64
            };
        }
    }
    // each loop takes what its branch needs, beyond where it joins
    let mut cq = vec![0.0; ci.stretches.len()];
    for &(e, k) in &ends {
        let down: f64 = tkids[k].iter().filter(|&&c| circulated[c]).map(|&c| q[c]).sum();
        cq[e] += (q[k] - down).max(0.02 * q[k]);
    }
    let loop_q: Vec<f64> = ends.iter().map(|&(e, _)| cq[e]).collect();
    for i in (0..ci.stretches.len()).rev() {
        if let Some(p) = ci.stretches[i].parent {
            let x = cq[i];
            cq[p] += x;
        }
    }
    // the circulation's sizes
    let types: Vec<_> = ci
        .stretches
        .iter()
        .map(|st| crate::pipe_types::by_id(&st.pipe.pipe_type))
        .collect();
    let sizes: Vec<Vec<(f64, f64, String)>> = types
        .iter()
        .map(|ty| {
            ty.map_or_else(Vec::new, |ty| {
                ty.sizes
                    .iter()
                    .filter(|z| z.inner_mm >= s.min_inner_mm)
                    .map(|z| (z.nominal_mm, z.inner_mm, ty.size_label(z.nominal_mm, None)))
                    .collect()
            })
        })
        .collect();
    let kb = |ty: Option<&crate::pipe_types::PipeType>| ty.map_or(0.007, |t| t.roughness_mm) / 1000.0;
    let m = ci.stretches.len();
    let mut pick: Vec<Option<usize>> = (0..m)
        .map(|i| {
            sizes[i]
                .iter()
                .position(|z| {
                    hydraulics::velocity(cq[i], z.1 / 1000.0) <= s.vmax
                        && hydraulics::r(cq[i], z.1 / 1000.0, kb(types[i]), &fluid) <= s.rmax
                })
                .or(sizes[i].len().checked_sub(1))
        })
        .collect();
    let inner = |i: usize, pick: &[Option<usize>]| pick[i].map_or(ci.stretches[i].pipe.inner_mm, |k| sizes[i][k].1);
    for _ in 0..4 {
        for i in (0..m).rev() {
            let Some(p) = ci.stretches[i].parent else {
                continue;
            };
            let need = inner(i, &pick);
            if inner(p, &pick) + 1e-6 < need {
                if let Some(k) = sizes[p].iter().position(|z| z.1 + 1e-6 >= need) {
                    pick[p] = Some(k);
                }
            }
        }
        for (_, parts) in ci.by_pipe() {
            if let Some(big) = parts
                .iter()
                .copied()
                .max_by(|&a, &b| inner(a, &pick).total_cmp(&inner(b, &pick)))
            {
                for &p in &parts {
                    pick[p] = pick[big];
                }
            }
        }
    }
    let closs: Vec<(f64, f64, f64)> = (0..m)
        .map(|i| {
            let d = inner(i, &pick) / 1000.0;
            let r = hydraulics::r(cq[i], d, kb(types[i]), &fluid);
            (r, hydraulics::velocity(cq[i], d), r * ci.stretches[i].length() * (1.0 + s.local_share))
        })
        .collect();
    let tloss: Vec<f64> = (0..n)
        .map(|i| {
            let st = &tv.stretches[i];
            let ty = crate::pipe_types::by_id(&st.pipe.pipe_type);
            hydraulics::r(q[i], st.pipe.inner_mm / 1000.0, kb(ty), &fluid) * st.length() * (1.0 + s.local_share)
        })
        .collect();
    let totals: Vec<(f64, f64)> = ends
        .iter()
        .map(|&(e, k)| {
            (
                way(tv, k).iter().map(|&i| tloss[i]).sum(),
                way(ci, e).iter().map(|&i| closs[i].2).sum(),
            )
        })
        .collect();
    let crit = (0..ends.len())
        .max_by(|&a, &b| (totals[a].0 + totals[a].1).total_cmp(&(totals[b].0 + totals[b].1)))
        .unwrap_or(0);
    let top = totals[crit].0 + totals[crit].1 + s.valve_min_kpa * 1000.0;
    for (j, &(e, k)) in ends.iter().enumerate() {
        let valve = (top - totals[j].0 - totals[j].1) / 1000.0;
        let qh = loop_q[j] * 3.6;
        out.loops.push(LoopResult {
            circ_pipe: ci.stretches[e].id,
            hot_pipe: tv.stretches[k].id,
            flow_ls: loop_q[j],
            hot_pa: totals[j].0,
            circ_pa: totals[j].1,
            valve_kpa: valve,
            kv: if valve > 0.0 {
                qh / (valve / 100.0).sqrt()
            } else {
                0.0
            },
            critical: j == crit,
        });
    }
    out.pump_kpa = (top / 1000.0 + s.heater_kpa) * (1.0 + s.reserve_pct / 100.0);
    out.pump_m = out.pump_kpa * 1000.0 / (fluid.rho * hydraulics::G);
    for (id, parts) in ci.by_pipe() {
        let big = parts
            .iter()
            .copied()
            .max_by(|&a, &b| cq[a].total_cmp(&cq[b]))
            .unwrap_or(parts[0]);
        let st = &ci.stretches[big];
        let size = types[big].map_or_else(|| format!("{}", st.pipe.nominal_mm), |t| t.size_label(st.pipe.nominal_mm, None));
        let (proposed, proposed_mm) = pick[big].map_or((size.clone(), st.pipe.nominal_mm), |k| (sizes[big][k].2.clone(), sizes[big][k].0));
        out.pipes.push(PipeResult {
            id,
            flow_ls: cq[big],
            size,
            proposed,
            proposed_mm,
            velocity_ms: closs[big].1,
            r_pa_m: closs[big].0,
            too_fast: closs[big].1 > s.vmax + 1e-6,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::mep::Pipe;
    use creabim_core::{Document, Element, ElementKind, Transaction};

    #[test]
    fn two_risers_share_the_circulation_by_their_losses() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let ty = crate::pipe_types::by_id("HERZ").unwrap();
        let pipe = |code: &str, a: [f64; 3], b: [f64; 3], d: f64| {
            let z = ty.size(d).unwrap();
            Element::new(
                "pipe",
                ElementKind::Pipe(Pipe {
                    level,
                    system: code.into(),
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
        // hot water along 10 m, a 6 m riser at 4 m and another at 10 m;
        // circulation back from each riser's top
        let mut tx = Transaction::new("tv");
        for (a, b) in [
            ([0.0, 0.0, 0.0], [4.0, 0.0, 0.0]),
            ([4.0, 0.0, 0.0], [10.0, 0.0, 0.0]),
            ([4.0, 0.0, 0.0], [4.0, 0.0, 6.0]),
            ([10.0, 0.0, 0.0], [10.0, 0.0, 6.0]),
        ] {
            tx = tx.insert(pipe("TV", a, b, 20.0));
        }
        let near_end = pipe("C", [4.0, 0.2, 0.2], [4.0, 0.2, 5.8], 16.0);
        let far_end = pipe("C", [10.0, 0.2, 0.2], [10.0, 0.2, 5.8], 16.0);
        let (near_id, far_id) = (near_end.id, far_end.id);
        for (a, b) in [
            ([0.0, 0.2, -0.3], [0.0, 0.2, 0.2]),
            ([0.0, 0.2, 0.2], [4.0, 0.2, 0.2]),
            ([4.0, 0.2, 0.2], [10.0, 0.2, 0.2]),
        ] {
            tx = tx.insert(pipe("C", a, b, 16.0));
        }
        tx = tx.insert(near_end).insert(far_end);
        doc.commit(tx).unwrap();
        let s = CircSettings::default();
        let r = run(doc.model(), &s).expect("circulation");
        assert_eq!((r.loops.len(), r.loose_ends), (2, 0), "{r:?}");
        // 4 + 6 m along at 10 W/m, two 6 m risers at 7 W/m
        assert!((r.loss_w - (100.0 + 84.0)).abs() < 1e-6, "{}", r.loss_w);
        let want = Fluid::water(55.0).flow_for_power(184.0, 5.0);
        assert!((r.flow_ls - want).abs() < 1e-12);
        let lp = |id| r.loops.iter().find(|l| l.circ_pipe == id).unwrap();
        let (a, b) = (lp(near_id), lp(far_id));
        // the near riser loses 42 W, the far branch 60 + 42 W
        assert!((a.flow_ls / b.flow_ls - 42.0 / 102.0).abs() < 1e-9, "{a:?} {b:?}");
        assert!((a.flow_ls + b.flow_ls - want).abs() < 1e-12);
        assert!(b.critical && !a.critical);
        assert!((b.valve_kpa - s.valve_min_kpa).abs() < 1e-9 && a.valve_kpa > b.valve_kpa);
        assert!(r.pipes.iter().all(|p| !p.too_fast));
        assert!(r.pump_kpa > s.valve_min_kpa + s.heater_kpa);
    }
}
