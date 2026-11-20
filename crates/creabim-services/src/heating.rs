//! A two-pipe heating system, as PilonDesigner's `HeatingAnalyzer` with
//! its HERZ presetting (`herzpreset.ts`): each radiator's flow from its
//! output at the design temperatures (STN EN 442), the supply and the
//! return sized for at most 0.8 m/s along the mains, 0.5 m/s to one
//! radiator and 150 Pa/m; every radiator's circuit is the supply to it,
//! its own radiator and valve (10 kPa) and the return from it. The worst
//! circuit sets the pump; the others are throttled by presetting their
//! valves, upstream ones first, with kv = Q / √(Δp / 100) as HERZ's
//! programs for STRÖMAX MS, TS-98-V and the differential pressure
//! regulators.

use crate::fluid::Fluid;
use crate::herzvalves::{self, HerzValve};
use crate::hydraulics;
use crate::tree::{self, Tree};
use creabim_core::catalogue::radiators::Panel;
use creabim_core::fixture::ConnKind;
use creabim_core::mep::Valve;
use creabim_core::{ElementId, ElementKind, Model};
use glam::DVec3;

/// The design of a heating system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeatSettings {
    pub supply_c: f64,
    pub return_c: f64,
    pub room_c: f64,
    /// The greatest velocity along the mains and to one radiator, m/s.
    pub vmax_main: f64,
    pub vmax_branch: f64,
    /// The greatest loss per metre, Pa/m.
    pub rmax: f64,
    pub min_inner_mm: f64,
    /// A radiator and its valve, kPa.
    pub terminal_kpa: f64,
    /// The heat source, kPa.
    pub source_kpa: f64,
    /// The pump's reserve, %.
    pub reserve_pct: f64,
    /// Local losses (fittings) as a share of friction.
    pub local_share: f64,
}

impl Default for HeatSettings {
    fn default() -> Self {
        HeatSettings {
            supply_c: 55.0,
            return_c: 45.0,
            room_c: 20.0,
            vmax_main: 0.8,
            vmax_branch: 0.5,
            rmax: 150.0,
            min_inner_mm: 10.0,
            terminal_kpa: 10.0,
            source_kpa: 15.0,
            reserve_pct: 10.0,
            local_share: 0.3,
        }
    }
}

impl HeatSettings {
    pub fn dt(&self) -> f64 {
        (self.supply_c - self.return_c).max(0.1)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RadiatorResult {
    pub id: ElementId,
    pub name: String,
    pub power_w: f64,
    pub flow_ls: f64,
    /// Losses along the supply to it and the return from it, Pa.
    pub supply_pa: f64,
    pub return_pa: f64,
    /// Its whole circuit, with its radiator and valve and the valves'
    /// presettings, Pa.
    pub circuit_pa: f64,
    /// What is still left to throttle, Pa.
    pub throttle_pa: f64,
    pub critical: bool,
    /// On the return too.
    pub returned: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PipeResult {
    pub id: ElementId,
    pub code: String,
    pub flow_ls: f64,
    pub radiators: usize,
    pub size: String,
    pub proposed: String,
    pub proposed_mm: f64,
    pub velocity_ms: f64,
    pub r_pa_m: f64,
    pub loss_pa: f64,
    pub critical: bool,
    pub too_fast: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PresetNote {
    #[default]
    None,
    /// Even the smallest presetting throttles too little.
    Under,
    /// Wide open: its circuit is the critical one.
    Open,
}

/// A valve's presetting.
#[derive(Clone, Debug, PartialEq)]
pub struct PresetRow {
    pub valve: ElementId,
    pub product: &'static str,
    pub dn: u32,
    pub code: &'static str,
    pub flow_m3h: f64,
    /// The loss it should take, kPa.
    pub target_kpa: f64,
    pub kv_req: f64,
    pub preset: &'static str,
    pub kv: f64,
    pub dp_kpa: f64,
    pub note: PresetNote,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HeatResult {
    pub radiators: Vec<RadiatorResult>,
    pub pipes: Vec<PipeResult>,
    pub presets: Vec<PresetRow>,
    pub flow_ls: f64,
    pub power_w: f64,
    /// The critical circuit, Pa.
    pub critical_pa: f64,
    /// The pump: its head, kPa and m.
    pub pump_kpa: f64,
    pub pump_m: f64,
    /// Radiators of no known output.
    pub no_power: usize,
    pub no_return: bool,
    pub unreached: usize,
}

/// A valve on the system's pipes, and its HERZ product.
struct OnPipe {
    id: ElementId,
    stretch: usize,
    product: Option<(&'static HerzValve, &'static herzvalves::HerzSize)>,
    preset: Option<String>,
    /// Its local loss coefficient where it has no HERZ product: its
    /// kind's, or for an air vent or a drain cock on a tee the maker's ζ
    /// of the tee straight through (none on other pipes: the share of
    /// local losses takes it).
    zeta: Option<f64>,
}

/// One tree, sized.
struct Side {
    tree: Tree,
    q: Vec<f64>,
    below: Vec<Vec<usize>>,
    sizes: Vec<Vec<(f64, f64, String)>>,
    pick: Vec<Option<usize>>,
    loss: Vec<(f64, f64, f64)>,
    valves: Vec<OnPipe>,
    /// The water's density, kg/m³.
    rho: f64,
}

/// The heating system of the model (supply `UKP`, return `UKS`).
pub fn run(model: &Model, s: &HeatSettings) -> Option<HeatResult> {
    let sup = tree::build_objects(model, "UKP", ConnKind::UKP, true)?;
    let ret = tree::build_objects(model, "UKS", ConnKind::UKS, true);
    Some(calc(model, sup, ret, s))
}

/// A radiator's output at the design temperatures, W: none when it is
/// not a panel of the catalogue.
pub fn radiator_w(model: &Model, id: ElementId, s: &HeatSettings) -> Option<f64> {
    let e = model.get(id)?;
    let t = creabim_core::objects::type_of(model, e)?;
    let p = Panel::of_key(t.catalogue.as_deref()?)?;
    Some(p.output_w(crate::radiators::delta_t(s.supply_c, s.return_c, s.room_c)))
}

fn valves_on(model: &Model, t: &Tree, medium: &str) -> Vec<OnPipe> {
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Valve(v) = &e.kind else {
            continue;
        };
        if v.kind_def().is_none_or(|k| k.terminal || k.nocalc) {
            continue;
        }
        let Some(ElementKind::Pipe(p)) = model.get(v.pipe).map(|e| &e.kind) else {
            continue;
        };
        let z = DVec3::Z * model.level(p.level).map_or(0.0, |l| l.elevation);
        let at = (p.start + z).lerp(p.end + z, v.t);
        let Some(i) = t.stretches.iter().position(|st| {
            st.id == v.pipe && {
                let d = st.b - st.a;
                let k = (at - st.a).dot(d) / d.length_squared().max(1e-12);
                (-1e-6..=1.0 + 1e-6).contains(&k)
            }
        }) else {
            continue;
        };
        let product = product_of(v, p, medium);
        let zeta = match v.kind.as_str() {
            "odvzdusnenie" | "vypustenie" => crate::makerzeta::zeta(
                &p.pipe_type,
                crate::makerzeta::Part::TeeThrough,
                p.nominal_mm,
                false,
            ),
            _ if product.is_none() => v.kind_def().and_then(|k| k.zeta),
            _ => None,
        };
        out.push(OnPipe {
            id: e.id,
            stretch: i,
            product,
            preset: v.preset.clone(),
            zeta,
        });
    }
    out
}

/// A valve's HERZ product and size: chosen, or HERZ's default on HERZ
/// pipes.
fn product_of(
    v: &Valve,
    p: &creabim_core::mep::Pipe,
    medium: &str,
) -> Option<(&'static HerzValve, &'static herzvalves::HerzSize)> {
    let ty = crate::pipe_types::by_id(&p.pipe_type);
    let dn = herzvalves::dn_for(p.outer_mm, p.inner_mm);
    let h = match v.product.as_deref() {
        Some("") => return None,
        Some(id) => herzvalves::by_id(id)?,
        None => {
            let herz = ty
                .and_then(|t| t.manufacturer.as_deref())
                .is_some_and(|m| m.starts_with("HERZ"));
            if !herz {
                return None;
            }
            herzvalves::default_for(&v.kind, medium, dn)?
        }
    };
    Some((h, h.size_for(dn)))
}

/// A valve's kv: at its presetting, else its widest presetting, else kvs.
fn kv_of(v: &OnPipe, open: bool) -> Option<f64> {
    let (h, s) = v.product?;
    let pre = h.presets_of(s);
    if !open {
        if let Some(k) = v
            .preset
            .as_deref()
            .and_then(|p| pre.iter().find(|x| x.0 == p))
        {
            return Some(k.1);
        }
    }
    pre.iter()
        .map(|x| x.1)
        .reduce(f64::max)
        .or((s.kvs > 0.0).then_some(s.kvs))
}

fn side(
    model: &Model,
    tree: Tree,
    flow_of: &dyn Fn(ElementId) -> f64,
    fluid: &Fluid,
    s: &HeatSettings,
) -> Side {
    let below = tree.terminals_below();
    let n = tree.stretches.len();
    let q: Vec<f64> = below
        .iter()
        .map(|b| b.iter().map(|&h| flow_of(tree.terminals[h].id)).sum())
        .collect();
    let types: Vec<_> = tree
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
    let kb: Vec<f64> = types
        .iter()
        .map(|t| t.map_or(0.007, |t| t.roughness_mm) / 1000.0)
        .collect();
    let mut pick: Vec<Option<usize>> = (0..n)
        .map(|i| {
            let vmax = if below[i].len() <= 1 {
                s.vmax_branch
            } else {
                s.vmax_main
            };
            let z = &sizes[i];
            z.iter()
                .position(|z| {
                    hydraulics::velocity(q[i], z.1 / 1000.0) <= vmax
                        && hydraulics::r(q[i], z.1 / 1000.0, kb[i], fluid) <= s.rmax
                })
                .or(z.len().checked_sub(1))
        })
        .collect();
    let inner = |i: usize, pick: &[Option<usize>]| {
        pick[i].map_or(tree.stretches[i].pipe.inner_mm, |k| sizes[i][k].1)
    };
    for _ in 0..4 {
        for i in (0..n).rev() {
            let Some(p) = tree.stretches[i].parent else {
                continue;
            };
            let need = inner(i, &pick);
            if inner(p, &pick) + 1e-6 < need {
                if let Some(k) = sizes[p].iter().position(|z| z.1 + 1e-6 >= need) {
                    pick[p] = Some(k);
                }
            }
        }
        for (_, parts) in tree.by_pipe() {
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
    // the maker's ζ of its fittings where it gives them, else the share
    let joints = crate::makerzeta::joints(&tree);
    let nominal = |i: usize| pick[i].map_or(tree.stretches[i].pipe.nominal_mm, |k| sizes[i][k].0);
    let loss = (0..n)
        .map(|i| {
            let d = inner(i, &pick) / 1000.0;
            let r = hydraulics::r(q[i], d, kb[i], fluid);
            let v = hydraulics::velocity(q[i], d);
            let st = &tree.stretches[i];
            let local = crate::makerzeta::stretch_zeta(
                &st.pipe.pipe_type,
                joints[i],
                nominal(i),
                st.parent.map(nominal),
            );
            let z = match local {
                Some(z) => r * st.length() + z * fluid.rho * v * v / 2.0,
                None => r * st.length() * (1.0 + s.local_share),
            };
            (r, v, z)
        })
        .collect();
    let medium = "ukur";
    let valves = valves_on(model, &tree, medium);
    Side {
        tree,
        q,
        below,
        sizes,
        pick,
        loss,
        valves,
        rho: fluid.rho,
    }
}

impl Side {
    /// The stretches from a terminal back to the root.
    fn way(&self, h: usize) -> Vec<usize> {
        let mut v = vec![];
        let mut at = self
            .tree
            .stretches
            .iter()
            .position(|st| st.terminals.contains(&h));
        while let Some(i) = at {
            v.push(i);
            at = self.tree.stretches[i].parent;
        }
        v
    }
    /// The loss along a terminal's way, Pa: friction and the valves (wide
    /// open, or at their presettings).
    fn path_pa(&self, h: usize, open: bool) -> f64 {
        let way = self.way(h);
        let mut p: f64 = way.iter().map(|&i| self.loss[i].2).sum();
        for v in &self.valves {
            if way.contains(&v.stretch) {
                if let Some(kv) = kv_of(v, open) {
                    p += herzvalves::dp_kpa(self.q[v.stretch] * 3.6, kv) * 1000.0;
                } else if let Some(z) = v.zeta {
                    let w = self.loss[v.stretch].1;
                    p += z * self.rho * w * w / 2.0;
                }
            }
        }
        p
    }
    fn terminal_of(&self, id: ElementId) -> Option<usize> {
        self.tree.terminals.iter().position(|t| t.id == id)
    }
}

fn calc(model: &Model, sup: Tree, ret: Option<Tree>, s: &HeatSettings) -> HeatResult {
    let mean = Fluid::water((s.supply_c + s.return_c) / 2.0);
    let dt = s.dt();
    let mut out = HeatResult {
        no_return: ret.is_none(),
        unreached: sup.unreached.len() + ret.as_ref().map_or(0, |r| r.unreached.len()),
        ..HeatResult::default()
    };
    // the radiators, by the supply
    let rads: Vec<(ElementId, String, f64)> = sup
        .terminals
        .iter()
        .map(|t| {
            let w = radiator_w(model, t.id, s).unwrap_or(0.0);
            (t.id, t.name.clone(), w)
        })
        .collect();
    out.no_power = rads.iter().filter(|r| r.2 <= 0.0).count();
    let flow_of = |id: ElementId| {
        rads.iter()
            .find(|r| r.0 == id)
            .map_or(0.0, |r| mean.flow_for_power(r.2, dt))
    };
    let sup = side(model, sup, &flow_of, &Fluid::water(s.supply_c), s);
    let ret = ret.map(|t| side(model, t, &flow_of, &Fluid::water(s.return_c), s));
    let sides: Vec<&Side> = std::iter::once(&sup).chain(ret.as_ref()).collect();
    let term = s.terminal_kpa * 1000.0;
    // each circuit with the valves wide open
    let circuit = |k: usize, open: bool| -> (f64, f64, bool) {
        let id = rads[k].0;
        let a = sup.terminal_of(id).map_or(0.0, |h| sup.path_pa(h, open));
        let rt = ret.as_ref().and_then(|r| r.terminal_of(id).map(|h| (r, h)));
        let b = rt.map_or(0.0, |(r, h)| r.path_pa(h, open));
        (a, b, rt.is_some())
    };
    let open: Vec<f64> = (0..rads.len())
        .map(|k| {
            let (a, b, _) = circuit(k, true);
            a + b + term
        })
        .collect();
    let m = open.iter().copied().fold(0.0, f64::max);
    // presettings, upstream valves first
    let mut applied = vec![0.0; rads.len()];
    let mut order: Vec<(usize, usize)> = vec![];
    for (si, sd) in sides.iter().enumerate() {
        for (vi, v) in sd.valves.iter().enumerate() {
            if v.product.is_some_and(|(h, z)| !h.presets_of(z).is_empty()) && sd.q[v.stretch] > 0.0
            {
                order.push((si, vi));
            }
        }
    }
    order.sort_by_key(|&(si, vi)| {
        std::cmp::Reverse(sides[si].below[sides[si].valves[vi].stretch].len())
    });
    for (si, vi) in order {
        let sd = sides[si];
        let v = &sd.valves[vi];
        let Some((h, z)) = v.product else { continue };
        let ks: Vec<usize> = sd.below[v.stretch]
            .iter()
            .filter_map(|&t| rads.iter().position(|r| r.0 == sd.tree.terminals[t].id))
            .collect();
        if ks.is_empty() {
            continue;
        }
        let worst = ks.iter().map(|&k| open[k] + applied[k]).fold(0.0, f64::max);
        let excess = (m - worst).max(0.0) / 1000.0;
        let q = sd.q[v.stretch] * 3.6;
        let pre = h.presets_of(z);
        let kv_max = pre.iter().map(|x| x.1).fold(0.0, f64::max);
        let base = herzvalves::dp_kpa(q, kv_max);
        let target = base + excess;
        let Some(p) = herzvalves::pick(pre, q, target) else {
            continue;
        };
        for &k in &ks {
            applied[k] += (p.dp_kpa - base).max(0.0) * 1000.0;
        }
        out.presets.push(PresetRow {
            valve: v.id,
            product: h.name,
            dn: z.dn,
            code: z.code,
            flow_m3h: q,
            target_kpa: target,
            kv_req: p.kv_req,
            preset: p.preset,
            kv: p.kv,
            dp_kpa: p.dp_kpa,
            note: if p.under {
                PresetNote::Under
            } else if p.open {
                PresetNote::Open
            } else {
                PresetNote::None
            },
        });
    }
    // the critical circuit and what is left to throttle
    // the critical circuit is the worst before throttling; a presetting
    // throttles at least what is asked, so the pump takes the most after
    let crit = (0..rads.len()).max_by(|&a, &b| open[a].total_cmp(&open[b]));
    let top = (0..rads.len())
        .map(|k| open[k] + applied[k])
        .fold(0.0, f64::max);
    for (k, r) in rads.iter().enumerate() {
        let (a, b, returned) = circuit(k, true);
        out.radiators.push(RadiatorResult {
            id: r.0,
            name: r.1.clone(),
            power_w: r.2,
            flow_ls: flow_of(r.0),
            supply_pa: a,
            return_pa: b,
            circuit_pa: open[k] + applied[k],
            throttle_pa: top - (open[k] + applied[k]),
            critical: crit == Some(k),
            returned,
        });
    }
    out.critical_pa = top;
    out.pump_kpa = (top / 1000.0 + s.source_kpa) * (1.0 + s.reserve_pct / 100.0);
    out.pump_m = out.pump_kpa * 1000.0 / (mean.rho * hydraulics::G);
    out.power_w = rads.iter().map(|r| r.2).sum();
    out.flow_ls = rads.iter().map(|r| flow_of(r.0)).sum();
    // the pipes
    let crit_id = crit.map(|k| rads[k].0);
    for (si, sd) in sides.iter().enumerate() {
        let code = if si == 0 { "UKP" } else { "UKS" };
        let crit_way = crit_id
            .and_then(|id| sd.terminal_of(id))
            .map_or_else(Vec::new, |h| sd.way(h));
        for (id, parts) in sd.tree.by_pipe() {
            let big = parts
                .iter()
                .copied()
                .max_by(|&a, &b| sd.q[a].total_cmp(&sd.q[b]))
                .unwrap_or(parts[0]);
            let st = &sd.tree.stretches[big];
            let ty = crate::pipe_types::by_id(&st.pipe.pipe_type);
            let size = ty.map_or_else(
                || format!("{}", st.pipe.nominal_mm),
                |t| t.size_label(st.pipe.nominal_mm, None),
            );
            let (proposed, proposed_mm) = sd.pick[big]
                .map_or((size.clone(), st.pipe.nominal_mm), |k| {
                    (sd.sizes[big][k].2.clone(), sd.sizes[big][k].0)
                });
            let vmax = if sd.below[big].len() <= 1 {
                s.vmax_branch
            } else {
                s.vmax_main
            };
            out.pipes.push(PipeResult {
                id,
                code: code.into(),
                flow_ls: sd.q[big],
                radiators: sd.below[big].len(),
                size,
                proposed,
                proposed_mm,
                velocity_ms: sd.loss[big].1,
                r_pa_m: sd.loss[big].0,
                loss_pa: parts.iter().map(|&p| sd.loss[p].2).sum(),
                critical: parts.iter().any(|p| crit_way.contains(p)),
                too_fast: sd.loss[big].1 > vmax + 1e-6,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::catalogue::radiators::PanelKind;
    use creabim_core::fixture;
    use creabim_core::mep::Pipe;
    use creabim_core::{Document, Element, Transaction};
    use glam::DVec2;

    /// Two radiators on a supply and a return from a boiler's corner, the
    /// far one larger; each supply branch with a TS-98-V on HERZ pipes.
    fn house() -> (Document, Vec<ElementId>, Vec<ElementId>) {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let mut rads = vec![];
        for (x, len) in [(2.0, 800), (8.0, 1600)] {
            let p = Panel {
                kind: PanelKind::ALL[2],
                height: 600,
                length: len,
            };
            let (tx, id) = creabim_core::objects::place(
                doc.model(),
                &p.object(),
                level,
                DVec2::new(x, 1.0),
                0.0,
            );
            doc.commit(tx).unwrap();
            rads.push(id);
        }
        let ty = crate::pipe_types::by_id("HERZ").unwrap();
        let z = ty.size(20.0).unwrap().clone();
        let pipe = |code: &str, a: DVec3, b: DVec3| {
            Element::new(
                "pipe",
                ElementKind::Pipe(Pipe {
                    level,
                    system: code.into(),
                    start: a,
                    end: b,
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
        let mut tx = Transaction::new("heating");
        let mut valves = vec![];
        for (kind, code, y) in [(ConnKind::UKP, "UKP", 0.2), (ConnKind::UKS, "UKS", 0.3)] {
            // a main along y from x = 0 (the boiler's side, lowest end)
            let start = DVec3::new(0.0, y, -0.2);
            tx = tx.insert(pipe(code, start, DVec3::new(0.0, y, 0.1)));
            let mut at = DVec3::new(0.0, y, 0.1);
            for &r in &rads {
                let e = doc.model().get(r).unwrap();
                let c = fixture::connectors(doc.model(), e)
                    .into_iter()
                    .find(|c| c.kind == kind)
                    .unwrap()
                    .at;
                let tee = DVec3::new(c.x, y, 0.1);
                tx = tx.insert(pipe(code, at, tee));
                let up = pipe(code, tee, DVec3::new(c.x, y, c.z));
                let up_id = up.id;
                tx = tx.insert(up).insert(pipe(code, DVec3::new(c.x, y, c.z), c));
                if code == "UKP" {
                    let v = Valve::new(up_id, 0.5, "termostaticky");
                    let ve = Element::new("TS", ElementKind::Valve(v));
                    valves.push(ve.id);
                    tx = tx.insert(ve);
                }
                at = tee;
            }
        }
        doc.commit(tx).unwrap();
        (doc, rads, valves)
    }

    #[test]
    fn the_far_circuit_is_critical_and_the_near_valve_throttles_the_rest() {
        let (doc, rads, valves) = house();
        let s = HeatSettings::default();
        let r = run(doc.model(), &s).expect("heating");
        assert_eq!(r.radiators.len(), 2, "{r:?}");
        assert_eq!((r.no_power, r.no_return, r.unreached), (0, false, 0));
        assert!(r.radiators.iter().all(|x| x.returned && x.power_w > 0.0));
        let of = |id| r.radiators.iter().find(|x| x.id == id).unwrap();
        let (near, far) = (of(rads[0]), of(rads[1]));
        assert!(far.critical && !near.critical, "{r:#?}");
        assert!(far.supply_pa > near.supply_pa);
        // 55/45 °C: the flow carries the output at 10 K
        let want = Fluid::water(50.0).flow_for_power(far.power_w, 10.0);
        assert!((far.flow_ls - want).abs() < 1e-12);
        // both thermostatic valves preset; the near one throttles more
        assert_eq!(r.presets.len(), 2, "{:?}", r.presets);
        let pv = |id| r.presets.iter().find(|p| p.valve == id).unwrap();
        let (pn, pf) = (pv(valves[0]), pv(valves[1]));
        assert_eq!(pn.product, herzvalves::by_id("HZ-TS98V").unwrap().name);
        assert!(
            pn.kv < pf.kv || pf.note == PresetNote::Open,
            "{pn:?} {pf:?}"
        );
        // what is left to throttle is within a presetting step
        assert!(
            near.throttle_pa >= -1e-6 && near.throttle_pa < 3000.0,
            "{near:?}"
        );
        assert!(r.pump_kpa > s.source_kpa + s.terminal_kpa);
        assert!(r.pipes.iter().all(|p| !p.too_fast));
    }

    #[test]
    fn an_air_vent_on_the_main_adds_its_tee_to_the_circuits_beyond_it() {
        let (mut doc, rads, _) = house();
        let s = HeatSettings::default();
        let far = |doc: &Document| {
            let r = run(doc.model(), &s).unwrap();
            r.radiators
                .iter()
                .find(|x| x.id == rads[1])
                .unwrap()
                .supply_pa
        };
        let before = far(&doc);
        // the supply main between the two radiators' tees
        let main = doc
            .model()
            .iter()
            .find(|e| match &e.kind {
                ElementKind::Pipe(p) => p.system == "UKP" && p.start.x > 1.0 && p.length() > 3.0,
                _ => false,
            })
            .unwrap()
            .id;
        let v = Element::new(
            "O",
            ElementKind::Valve(Valve::new(main, 0.5, "odvzdusnenie")),
        );
        doc.commit(Transaction::new("vent").insert(v)).unwrap();
        let after = far(&doc);
        // HERZ gives the tee straight through its ζ
        assert!(after > before + 1.0, "{before} {after}");
    }
}
