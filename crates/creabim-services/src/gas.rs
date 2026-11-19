//! A gas installation in a building, as PilonDesigner's `FuelGasAnalyzer`
//! (STN EN 1775, TPP 704 01, TPP 704 03, the SPP-distribúcia technical
//! conditions): the appliances' flows from their heat input, reduced for
//! diversity Vr = Σ Kᵢ·Vᵢ (K1 = n^-0,5, K2 = n^-0,15, K3 = n^-0,1,
//! K4 = 1); Darcy-Weisbach losses at the gas's density at its pressure,
//! with half again for fittings and valves; the gas's lift or weight in
//! risers; a low pressure installation may lose 100 Pa from the meter
//! to the farthest appliance, and gas may run at 10 m/s (20 m/s at
//! medium pressure). The pipes on the worst appliance's way are enlarged
//! until it is met. The gas meter and the cabinet at the boundary follow.

use crate::fluid::Fluid;
use crate::hydraulics;
use crate::tree::{self, Tree};
use creabim_core::{ElementId, Model};

/// The gases a building is supplied with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GasKind {
    #[default]
    Natural,
    Propane,
    PropaneButane,
    /// Natural gas with 20 % hydrogen.
    Hydrogen20,
}

/// A gas's properties at normal conditions (0 °C, 101,325 kPa).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GasProps {
    pub name: &'static str,
    /// Density, kg/m³.
    pub rho_n: f64,
    /// Net calorific value, kWh/m³.
    pub hi_kwh: f64,
    /// Dynamic viscosity, Pa·s.
    pub mu: f64,
    /// The appliances' nominal and least supply pressure, kPa (STN EN 437).
    pub nominal_kpa: f64,
    pub min_kpa: f64,
}

pub const RHO_AIR_N: f64 = 1.293;

impl GasKind {
    pub const ALL: [GasKind; 4] = [
        GasKind::Natural,
        GasKind::Propane,
        GasKind::PropaneButane,
        GasKind::Hydrogen20,
    ];

    pub fn props(self) -> GasProps {
        match self {
            GasKind::Natural => GasProps {
                name: "Natural gas",
                rho_n: 0.73,
                hi_kwh: 9.5,
                mu: 1.08e-5,
                nominal_kpa: 2.0,
                min_kpa: 1.8,
            },
            GasKind::Propane => GasProps {
                name: "Propane (LPG)",
                rho_n: 2.019,
                hi_kwh: 25.9,
                mu: 7.5e-6,
                nominal_kpa: 3.7,
                min_kpa: 2.5,
            },
            GasKind::PropaneButane => GasProps {
                name: "Propane-butane (LPG 60/40)",
                rho_n: 2.293,
                hi_kwh: 29.3,
                mu: 7.4e-6,
                nominal_kpa: 3.0,
                min_kpa: 2.5,
            },
            GasKind::Hydrogen20 => GasProps {
                name: "Natural gas with 20 % H₂",
                rho_n: 0.602,
                hi_kwh: 8.2,
                mu: 1.04e-5,
                nominal_kpa: 2.0,
                min_kpa: 1.8,
            },
        }
    }

    pub fn is_lpg(self) -> bool {
        matches!(self, GasKind::Propane | GasKind::PropaneButane)
    }
}

impl GasProps {
    /// Relative density to air.
    pub fn d(&self) -> f64 {
        self.rho_n / RHO_AIR_N
    }
    /// Molar mass of the ideal gas, kg/mol.
    pub fn molar_mass(&self) -> f64 {
        self.rho_n * 0.022414
    }
}

/// The settings of a gas calculation.
#[derive(Clone, Debug, PartialEq)]
pub struct GasSettings {
    pub gas: GasKind,
    /// The installation runs at medium pressure (5–400 kPa), not low.
    pub medium_pressure: bool,
    /// The service pipe is at medium pressure: the cabinet has a regulator.
    pub service_medium: bool,
    /// Low pressure at the start, natural gas / propane / propane-butane, kPa.
    pub ntl_natural_kpa: f64,
    pub ntl_propane_kpa: f64,
    pub ntl_pb_kpa: f64,
    /// Medium pressure past the cabinet, kPa.
    pub stl_kpa: f64,
    /// The service pipe's pressure before the regulator, kPa.
    pub service_kpa: f64,
    /// The loss allowed at low pressure, Pa.
    pub allowed_ntl_pa: f64,
    /// The loss allowed at medium pressure, % of its pressure.
    pub allowed_stl_pct: f64,
    pub vmax_ntl: f64,
    pub vmax_stl: f64,
    /// The gas meter's loss, Pa.
    pub meter_pa: f64,
    pub temp_c: f64,
    pub atm_kpa: f64,
    /// A gas meter in the cabinet.
    pub metering: bool,
    /// The least meter (G4 for homes).
    pub min_meter: &'static str,
    /// Enlarge the worst appliance's way until its loss is met.
    pub enlarge_critical: bool,
    pub min_inner_mm: f64,
    /// The most a house regulator set passes, m³/h.
    pub max_house_set_m3h: f64,
    /// Local losses as a share of friction (TPP 704 01: 50 %).
    pub local_share: f64,
}

impl Default for GasSettings {
    fn default() -> Self {
        GasSettings {
            gas: GasKind::Natural,
            medium_pressure: false,
            service_medium: true,
            ntl_natural_kpa: 2.1,
            ntl_propane_kpa: 3.7,
            ntl_pb_kpa: 3.0,
            stl_kpa: 100.0,
            service_kpa: 300.0,
            allowed_ntl_pa: 100.0,
            allowed_stl_pct: 10.0,
            vmax_ntl: 10.0,
            vmax_stl: 20.0,
            meter_pa: 50.0,
            temp_c: 15.0,
            atm_kpa: 101.325,
            metering: true,
            min_meter: "G4",
            enlarge_critical: true,
            min_inner_mm: 10.0,
            max_house_set_m3h: 25.0,
            local_share: 0.5,
        }
    }
}

impl GasSettings {
    /// The overpressure where the installation starts, kPa.
    pub fn start_kpa(&self) -> f64 {
        if self.medium_pressure {
            return self.stl_kpa;
        }
        match self.gas {
            GasKind::Propane => self.ntl_propane_kpa,
            GasKind::PropaneButane => self.ntl_pb_kpa,
            _ => self.ntl_natural_kpa,
        }
    }
    /// The loss allowed, Pa.
    pub fn allowed_pa(&self) -> f64 {
        if self.medium_pressure {
            self.start_kpa() * 1000.0 * self.allowed_stl_pct / 100.0
        } else {
            self.allowed_ntl_pa
        }
    }
    pub fn vmax(&self) -> f64 {
        if self.medium_pressure {
            self.vmax_stl
        } else {
            self.vmax_ntl
        }
    }
    /// The absolute pressure the density is taken at: halfway down the
    /// loss allowed, kPa.
    pub fn mean_abs_kpa(&self) -> f64 {
        self.atm_kpa + self.start_kpa() - self.allowed_pa() / 2000.0
    }
    /// The gas at its working pressure and temperature.
    pub fn fluid(&self) -> Fluid {
        let p = self.gas.props();
        let rho =
            self.mean_abs_kpa() * 1000.0 * p.molar_mass() / (8.314462 * (self.temp_c + 273.15));
        Fluid {
            temp_c: self.temp_c,
            rho,
            nu: p.mu / rho,
            cp: 2200.0,
        }
    }
    /// The working flow, l/s, of `vr` normal m³/h.
    pub fn actual_ls(&self, vr_m3h: f64) -> f64 {
        vr_m3h / 3.6 * (self.atm_kpa / self.mean_abs_kpa()) * ((273.15 + self.temp_c) / 273.15)
    }
    /// An appliance's flow, normal m³/h, of its heat input.
    pub fn flow_for(&self, kw: f64) -> f64 {
        kw / self.gas.props().hi_kwh
    }
}

/// An appliance kind, known by words in its name.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ApplianceRule {
    pub name: &'static str,
    /// 1–4, as [`k`].
    pub cat: u8,
    pub kw: f64,
    pub words: &'static [&'static str],
}

pub const APPLIANCES: [ApplianceRule; 6] = [
    ApplianceRule {
        name: "Boiler",
        cat: 3,
        kw: 25.0,
        words: &["kotol", "kotel", "boiler", "kondenz", "combi", "turbo"],
    },
    ApplianceRule {
        name: "Gas cooker / hob / oven",
        cat: 1,
        kw: 8.0,
        words: &["sporak", "varna", "hob", "cooker", "rura", "oven", "doska"],
    },
    ApplianceRule {
        name: "Instantaneous water heater",
        cat: 1,
        kw: 20.0,
        words: &["prietokov", "prutokov", "karma"],
    },
    ApplianceRule {
        name: "Storage water heater",
        cat: 2,
        kw: 6.0,
        words: &["zasobnik", "bojler", "ohrievac"],
    },
    ApplianceRule {
        name: "Local heater (gas fire, stove)",
        cat: 2,
        kw: 5.0,
        words: &[
            "gamat",
            "krb",
            "kachl",
            "lokal",
            "waw",
            "konvektor",
            "infra",
        ],
    },
    ApplianceRule {
        name: "Industrial / catering appliance",
        cat: 4,
        kw: 20.0,
        words: &["technolog", "velkokuch", "kotlina", "konvektomat", "horak"],
    },
];

/// An appliance not known by its name: taken as a boiler.
pub const UNKNOWN: ApplianceRule = ApplianceRule {
    name: "Unknown appliance (taken as a 25 kW boiler)",
    cat: 3,
    kw: 25.0,
    words: &[],
};

/// The rule an appliance named `name` follows, and whether it was known.
pub fn match_appliance(name: &str) -> (&'static ApplianceRule, bool) {
    let n = crate::fixtures::normalize(name);
    APPLIANCES
        .iter()
        .find(|r| r.words.iter().any(|w| n.contains(w)))
        .map_or((&UNKNOWN, false), |r| (r, true))
}

/// The diversity factor of `n` appliances of category `cat`.
pub fn k(cat: u8, n: usize) -> f64 {
    if n == 0 {
        return 0.0;
    }
    let n = n as f64;
    match cat {
        1 => n.powf(-0.5),
        2 => n.powf(-0.15),
        3 => n.powf(-0.1),
        _ => 1.0,
    }
}

/// Vr = Σ Kᵢ(nᵢ)·Vᵢ, m³/h, of the counts and flows by category.
pub fn reduced(n: [usize; 4], v: [f64; 4]) -> f64 {
    (0..4).map(|i| k(i as u8 + 1, n[i]) * v[i]).sum()
}

/// Gas meters (STN EN 1359): size, Qmax m³/h.
pub const METERS: [(&str, f64); 8] = [
    ("G2,5", 4.0),
    ("G4", 6.0),
    ("G6", 10.0),
    ("G10", 16.0),
    ("G16", 25.0),
    ("G25", 40.0),
    ("G40", 65.0),
    ("G65", 100.0),
];

/// The least meter, no smaller than `min`, for `q` m³/h: none over G65.
pub fn meter_for(q: f64, min: &str) -> Option<(&'static str, f64)> {
    let floor = METERS.iter().find(|m| m.0 == min).map_or(0.0, |m| m.1);
    METERS
        .iter()
        .copied()
        .find(|m| m.1 + 1e-9 >= floor && q <= m.1 + 1e-9)
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApplianceResult {
    pub id: ElementId,
    pub name: String,
    /// The kind it was taken for.
    pub kind: &'static str,
    pub cat: u8,
    pub kw: f64,
    pub flow_m3h: f64,
    /// Known by its name or given its input.
    pub matched: bool,
    /// The loss to it as compared with the loss allowed, Pa.
    pub loss_pa: f64,
    /// The pressure before it, kPa.
    pub pressure_kpa: f64,
    pub critical: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PipeResult {
    pub id: ElementId,
    pub vr_m3h: f64,
    pub flow_ls: f64,
    pub appliances: usize,
    pub size: String,
    pub proposed: String,
    pub proposed_mm: f64,
    pub velocity_ms: f64,
    pub r_pa_m: f64,
    pub loss_pa: f64,
    pub critical: bool,
    pub too_fast: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GasResult {
    pub appliances: Vec<ApplianceResult>,
    pub pipes: Vec<PipeResult>,
    /// Installed ΣV, m³/h, and kW.
    pub installed_m3h: f64,
    pub installed_kw: f64,
    /// The reduced flow Vr at the start, m³/h.
    pub vr_m3h: f64,
    /// The worst appliance's loss, Pa, and the loss allowed.
    pub loss_pa: f64,
    pub allowed_pa: f64,
    /// The worst way's losses: along horizontal pipes, in risers, and
    /// the gas's lift (+) or weight (−), Pa; its rise, m.
    pub horizontal_pa: f64,
    pub vertical_pa: f64,
    pub lift_pa: f64,
    pub rise_m: f64,
    /// The pressure before the worst appliance, kPa.
    pub pressure_kpa: f64,
    pub start_kpa: f64,
    pub rho: f64,
    pub vmax_found: f64,
    pub meter: Option<(&'static str, f64)>,
    /// Steps the worst way was enlarged by.
    pub enlarged: usize,
    /// The flow is more than a house regulator set passes.
    pub over_house_set: bool,
    pub unreached: usize,
}

/// The gas installation `code` of the model, calculated.
pub fn run(model: &Model, code: &str, s: &GasSettings) -> Option<GasResult> {
    let t = tree::build_objects(model, code, creabim_core::fixture::ConnKind::P, false)?;
    Some(calc(&t, s))
}

struct Size {
    nominal: f64,
    inner: f64,
    label: String,
}

fn calc(t: &Tree, s: &GasSettings) -> GasResult {
    let fluid = s.fluid();
    let allowed = s.allowed_pa();
    let vmax = s.vmax();
    let mut out = GasResult {
        unreached: t.unreached.len(),
        allowed_pa: allowed,
        start_kpa: s.start_kpa(),
        rho: fluid.rho,
        ..GasResult::default()
    };
    // the appliances
    let apps: Vec<(&'static ApplianceRule, bool, u8, f64)> = t
        .terminals
        .iter()
        .map(|a| {
            let (rule, known) = match_appliance(&a.name);
            let cat = a
                .valve
                .gas_cat
                .filter(|c| (1..=4).contains(c))
                .unwrap_or(rule.cat);
            let kw = a.valve.power_kw.filter(|k| *k > 0.0).unwrap_or(rule.kw);
            (rule, known || a.valve.power_kw.is_some(), cat, kw)
        })
        .collect();
    let below = t.terminals_below();
    let n = t.stretches.len();
    let vr: Vec<f64> = below
        .iter()
        .map(|b| {
            let mut cn = [0usize; 4];
            let mut cv = [0.0; 4];
            for &h in b {
                let i = (apps[h].2 - 1) as usize;
                cn[i] += 1;
                cv[i] += s.flow_for(apps[h].3);
            }
            reduced(cn, cv)
        })
        .collect();
    let q: Vec<f64> = vr.iter().map(|&v| s.actual_ls(v)).collect();
    let types: Vec<Option<&crate::pipe_types::PipeType>> = t
        .stretches
        .iter()
        .map(|st| crate::pipe_types::by_id(&st.pipe.pipe_type))
        .collect();
    let sizes: Vec<Vec<Size>> = (0..n)
        .map(|i| {
            types[i].map_or_else(Vec::new, |ty| {
                ty.sizes
                    .iter()
                    .filter(|z| z.inner_mm >= s.min_inner_mm)
                    .map(|z| Size {
                        nominal: z.nominal_mm,
                        inner: z.inner_mm,
                        label: ty.size_label(z.nominal_mm, None),
                    })
                    .collect()
            })
        })
        .collect();
    let kb: Vec<f64> = types
        .iter()
        .map(|t| t.map_or(0.0015, |t| t.roughness_mm) / 1000.0)
        .collect();
    let r_of = |i: usize, inner_mm: f64| hydraulics::r(q[i], inner_mm / 1000.0, kb[i], &fluid);
    let v_of = |i: usize, inner_mm: f64| hydraulics::velocity(q[i], inner_mm / 1000.0);
    // the longest horizontal way: the loss per metre to aim at
    let host = |h: usize| t.stretches.iter().position(|st| st.terminals.contains(&h));
    let way = |h: usize| {
        let mut v = vec![];
        let mut at = host(h);
        while let Some(i) = at {
            v.push(i);
            at = t.stretches[i].parent;
        }
        v
    };
    let longest = (0..apps.len())
        .map(|h| {
            way(h)
                .iter()
                .filter(|&&i| !t.stretches[i].is_vertical())
                .map(|&i| t.stretches[i].length())
                .sum::<f64>()
        })
        .fold(0.0, f64::max);
    let rmax = if longest > 0.5 {
        allowed / ((1.0 + s.local_share) * longest)
    } else {
        f64::INFINITY
    };
    // each stretch's size, as an index into its sizes
    let mut pick: Vec<Option<usize>> = (0..n)
        .map(|i| {
            let z = &sizes[i];
            let rm = if t.stretches[i].is_vertical() && !s.medium_pressure {
                rmax.max(2.0)
            } else {
                rmax
            };
            z.iter()
                .position(|z| v_of(i, z.inner) <= vmax && r_of(i, z.inner) <= rm)
                .or(z.len().checked_sub(1))
        })
        .collect();
    let inner = |i: usize, pick: &[Option<usize>]| {
        pick[i].map_or(t.stretches[i].pipe.inner_mm, |k| sizes[i][k].inner)
    };
    // no smaller upstream, one size along a pipe
    let even = |pick: &mut Vec<Option<usize>>| {
        for _ in 0..4 {
            for i in (0..n).rev() {
                let Some(p) = t.stretches[i].parent else {
                    continue;
                };
                let need = inner(i, pick);
                if inner(p, pick) + 1e-6 < need {
                    if let Some(k) = sizes[p].iter().position(|z| z.inner + 1e-6 >= need) {
                        pick[p] = Some(k);
                    }
                }
            }
            for (_, parts) in t.by_pipe() {
                let big = parts
                    .iter()
                    .copied()
                    .max_by(|&a, &b| inner(a, pick).total_cmp(&inner(b, pick)));
                if let Some(big) = big {
                    for &p in &parts {
                        pick[p] = pick[big];
                    }
                }
            }
        }
    };
    even(&mut pick);
    let losses = |pick: &[Option<usize>]| -> Vec<(f64, f64, f64)> {
        (0..n)
            .map(|i| {
                let d = inner(i, pick);
                let r = r_of(i, d);
                (
                    r,
                    v_of(i, d),
                    r * t.stretches[i].length() * (1.0 + s.local_share),
                )
            })
            .collect()
    };
    let rho_air = RHO_AIR_N * (s.mean_abs_kpa() / 101.325) * (273.15 / (273.15 + s.temp_c));
    // an appliance's way: horizontal, vertical, lift, rise; and the loss
    // compared with the allowed: a lift makes up for risers' losses only
    let parts = |h: usize, ls: &[(f64, f64, f64)]| {
        let (mut hz, mut vt) = (0.0, 0.0);
        for i in way(h) {
            if t.stretches[i].is_vertical() {
                vt += ls[i].2;
            } else {
                hz += ls[i].2;
            }
        }
        let dz = t.terminals[h].at.z - t.root.z;
        let gain = hydraulics::G * dz * (rho_air - fluid.rho);
        let req = hz
            + if gain > 0.0 {
                (vt - gain).max(0.0)
            } else {
                vt - gain
            };
        (hz, vt, gain, dz, req)
    };
    let worst = |ls: &[(f64, f64, f64)]| {
        (0..apps.len())
            .map(|h| (h, parts(h, ls).4))
            .max_by(|a, b| a.1.total_cmp(&b.1))
    };
    let mut ls = losses(&pick);
    let mut w = worst(&ls);
    if s.enlarge_critical {
        while let Some((h, req)) = w {
            if req <= allowed || out.enlarged >= 300 {
                break;
            }
            let path: Vec<usize> = way(h)
                .into_iter()
                .filter(|&i| pick[i].is_some_and(|k| k + 1 < sizes[i].len()))
                .collect();
            if path.is_empty() {
                break;
            }
            let top = path.iter().map(|&i| ls[i].0).fold(0.0, f64::max);
            for &i in &path {
                if ls[i].0 >= 0.8 * top {
                    pick[i] = pick[i].map(|k| k + 1);
                }
            }
            even(&mut pick);
            ls = losses(&pick);
            w = worst(&ls);
            out.enlarged += 1;
        }
    }
    let meter_pa = if !s.medium_pressure && s.metering {
        s.meter_pa
    } else {
        0.0
    };
    let mut crit = vec![false; n];
    if let Some((h, req)) = w {
        let (hz, vt, gain, dz, _) = parts(h, &ls);
        out.loss_pa = req;
        out.horizontal_pa = hz;
        out.vertical_pa = vt;
        out.lift_pa = gain;
        out.rise_m = dz;
        out.pressure_kpa = s.start_kpa() - (meter_pa + hz + vt - gain) / 1000.0;
        for i in way(h) {
            crit[i] = true;
        }
    }
    for (h, a) in t.terminals.iter().enumerate() {
        let (hz, vt, gain, _, req) = parts(h, &ls);
        let (rule, matched, cat, kw) = apps[h];
        out.appliances.push(ApplianceResult {
            id: a.id,
            name: a.name.clone(),
            kind: rule.name,
            cat,
            kw,
            flow_m3h: s.flow_for(kw),
            matched,
            loss_pa: req,
            pressure_kpa: s.start_kpa() - (meter_pa + hz + vt - gain) / 1000.0,
            critical: w.is_some_and(|w| w.0 == h),
        });
    }
    for (id, parts) in t.by_pipe() {
        let big = parts
            .iter()
            .copied()
            .max_by(|&a, &b| q[a].total_cmp(&q[b]))
            .unwrap_or(parts[0]);
        let st = &t.stretches[big];
        let size = types[big].map_or_else(
            || format!("{}", st.pipe.nominal_mm),
            |ty| ty.size_label(st.pipe.nominal_mm, None),
        );
        let (proposed, proposed_mm) = pick[big].map_or((size.clone(), st.pipe.nominal_mm), |k| {
            (sizes[big][k].label.clone(), sizes[big][k].nominal)
        });
        out.pipes.push(PipeResult {
            id,
            vr_m3h: vr[big],
            flow_ls: q[big],
            appliances: below[big].len(),
            size,
            proposed,
            proposed_mm,
            velocity_ms: ls[big].1,
            r_pa_m: ls[big].0,
            loss_pa: parts.iter().map(|&p| ls[p].2).sum(),
            critical: parts.iter().any(|&p| crit[p]),
            too_fast: parts.iter().any(|&p| ls[p].1 > vmax + 1e-6),
        });
    }
    out.vmax_found = ls.iter().map(|l| l.1).fold(0.0, f64::max);
    out.installed_kw = apps.iter().map(|a| a.3).sum();
    out.installed_m3h = s.flow_for(out.installed_kw);
    out.vr_m3h = (0..n)
        .filter(|&i| t.stretches[i].parent.is_none())
        .map(|i| vr[i])
        .sum();
    if s.metering {
        out.meter = meter_for(out.vr_m3h, s.min_meter);
    }
    out.over_house_set = s.service_medium && out.vr_m3h > s.max_house_set_m3h;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::mep::{Pipe, Valve};
    use creabim_core::{Document, Element, ElementKind, Transaction};
    use glam::DVec3;

    #[test]
    fn diversity_reduces_the_flow_by_category() {
        // two boilers of 1 m³/h: 2·2^-0,1
        let vr = reduced([0, 0, 2, 0], [0.0, 0.0, 2.0, 0.0]);
        assert!((vr - 2.0 * 2f64.powf(-0.1)).abs() < 1e-12);
        // a cooker alone counts whole
        assert_eq!(reduced([1, 0, 0, 0], [0.84, 0.0, 0.0, 0.0]), 0.84);
        assert_eq!(k(4, 7), 1.0);
        assert_eq!(meter_for(2.6, "G4"), Some(("G4", 6.0)));
        assert_eq!(meter_for(7.0, "G4"), Some(("G6", 10.0)));
        assert_eq!(meter_for(120.0, "G4"), None);
    }

    #[test]
    fn the_gas_is_taken_at_its_pressure() {
        let s = GasSettings::default();
        let f = s.fluid();
        // natural gas at about 103,4 kPa abs. and 15 °C: ρ ≈ 0,707 kg/m³
        assert!((f.rho - 0.707).abs() < 0.01, "{}", f.rho);
        assert!((s.flow_for(25.0) - 25.0 / 9.5).abs() < 1e-12);
        assert_eq!(match_appliance("Kondenzačný kotol").0.cat, 3);
        assert_eq!(match_appliance("Plynový sporák").0.cat, 1);
        assert!(!match_appliance("spotrebic").1);
    }

    fn house(cu_size: f64) -> (Document, ElementId, ElementId) {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let ty = crate::pipe_types::by_id("Cu").unwrap();
        let z = ty.size(cu_size).unwrap().clone();
        let pipe = |a: [f64; 3], b: [f64; 3]| {
            Element::new(
                "pipe",
                ElementKind::Pipe(Pipe {
                    level,
                    system: "ZP".into(),
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
        // in from the boundary, 15 m along, up 3 m to a boiler upstairs,
        // a cooker on the ground floor
        let main = pipe([0.0, 0.0, 0.0], [15.0, 0.0, 0.0]);
        let main_id = main.id;
        let up = pipe([15.0, 0.0, 0.0], [15.0, 0.0, 3.0]);
        let to_boiler = pipe([15.0, 0.0, 3.0], [18.0, 0.0, 3.0]);
        let boiler_pipe = to_boiler.id;
        let to_cooker = pipe([10.0, 0.0, 0.0], [10.0, 4.0, 0.0]);
        let cooker_pipe = to_cooker.id;
        let mut boiler = Valve::new(boiler_pipe, 1.0, "spotrebic");
        boiler.power_kw = Some(24.0);
        let tx = Transaction::new("gas")
            .insert(main)
            .insert(up)
            .insert(to_boiler)
            .insert(to_cooker)
            .insert(Element::new("Kotol", ElementKind::Valve(boiler)))
            .insert(Element::new(
                "Plynový sporák",
                ElementKind::Valve(Valve::new(cooker_pipe, 1.0, "spotrebic")),
            ));
        doc.commit(tx).unwrap();
        (doc, main_id, boiler_pipe)
    }

    #[test]
    fn a_house_with_a_boiler_and_a_cooker_is_sized_to_100_pa() {
        let (doc, main, boiler_pipe) = house(15.0);
        let s = GasSettings::default();
        let r = run(doc.model(), "ZP", &s).expect("gas pipes");
        assert_eq!((r.appliances.len(), r.unreached), (2, 0));
        // 24 kW boiler and an 8 kW cooker: ΣV 32/9,5 m³/h, each alone
        assert!((r.installed_m3h - 32.0 / 9.5).abs() < 1e-9);
        assert!((r.vr_m3h - 32.0 / 9.5).abs() < 1e-9, "{}", r.vr_m3h);
        assert_eq!(r.meter.map(|m| m.0), Some("G4"));
        assert!(r.loss_pa <= 100.0 + 1e-9, "{r:?}");
        assert!(r.vmax_found <= 10.0);
        // natural gas is lighter than air: it gains going up 3 m
        let boiler = r.appliances.iter().find(|a| a.name == "Kotol").unwrap();
        assert!(boiler.matched && boiler.cat == 3 && boiler.kw == 24.0);
        let cooker = r.appliances.iter().find(|a| a.name != "Kotol").unwrap();
        assert!(cooker.matched && cooker.cat == 1 && cooker.kw == 8.0);
        assert!(
            r.pressure_kpa > 1.8 && r.pressure_kpa < 2.1,
            "{}",
            r.pressure_kpa
        );
        let of = |id| r.pipes.iter().find(|p| p.id == id).unwrap();
        // the main carries both, sized up from 15x1, no smaller than the
        // pipe to the boiler
        assert_eq!(of(main).appliances, 2);
        assert!(of(main).proposed_mm >= of(boiler_pipe).proposed_mm);
        assert!(of(main).proposed_mm > 15.0, "{:?}", of(main));
        assert!(of(main).critical);
    }

    #[test]
    fn a_boiler_with_a_gas_connector_is_an_appliance_by_its_name() {
        use creabim_core::catalogue::heat::{unit, HeatKind};
        use creabim_core::fixture::{self, ConnKind};
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let (tx, id) = creabim_core::objects::place(
            doc.model(),
            &unit(HeatKind::Boiler, 24),
            level,
            glam::DVec2::new(8.0, 2.0),
            0.0,
        );
        doc.commit(tx).unwrap();
        let e = doc.model().get(id).unwrap();
        let p = fixture::connectors(doc.model(), e)
            .into_iter()
            .find(|c| c.kind == ConnKind::P)
            .unwrap()
            .at;
        let ty = crate::pipe_types::by_id("Cu").unwrap();
        let z = ty.size(22.0).unwrap().clone();
        let mut tx = Transaction::new("gas");
        for (a, b) in [
            ([p.x, p.y, -0.5], [p.x, p.y, 0.0]),
            ([p.x, p.y, 0.0], [p.x, p.y, p.z]),
        ] {
            tx = tx.insert(Element::new(
                "pipe",
                ElementKind::Pipe(Pipe {
                    level,
                    system: "ZP".into(),
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
            ));
        }
        doc.commit(tx).unwrap();
        let r = run(doc.model(), "ZP", &GasSettings::default()).unwrap();
        assert_eq!(r.appliances.len(), 1, "{r:?}");
        let a = &r.appliances[0];
        assert_eq!((a.id, a.cat, a.matched), (id, 3, true), "{a:?}");
    }
}
