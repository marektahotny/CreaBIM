//! Sizing of the ventilation ducts drawn, by the velocity of the air:
//! every stretch carries the air of the terminals beyond it
//! ([`crate::airbalance::terminal_flow`]); a round duct takes the
//! smallest diameter of the series (STN EN 1506) that keeps the air at
//! or under the velocity allowed (the designer's: the main run and the
//! branches to the terminals), a rectangular one is checked as it is.
//! The friction loss of each stretch is Darcy's with Colebrook's λ for
//! galvanised sheet (k = 0.15 mm) and air of 1.2 kg/m³.

use crate::{airbalance, heatloss, tree};
use creabim_core::{ElementId, Model};

/// The diameters round ducts are made in, mm (STN EN 1506).
pub const ROUND_MM: [f64; 12] = [
    63.0, 80.0, 100.0, 125.0, 160.0, 200.0, 250.0, 315.0, 400.0, 500.0, 630.0, 800.0,
];
const RHO: f64 = 1.2;
const NU: f64 = 15.0e-6;
const ROUGH: f64 = 0.15e-3;

/// The velocities allowed, m/s.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Limits {
    /// In the run from the unit to the last branching.
    pub main: f64,
    /// In a branch that ends at a terminal.
    pub branch: f64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            main: 4.0,
            branch: 2.5,
        }
    }
}

/// One duct (a pipe element) sized.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub id: ElementId,
    /// The most air any of its stretches carries, m³/h.
    pub flow_m3h: f64,
    pub length: f64,
    /// It ends at a terminal: the branch's velocity applies.
    pub branch: bool,
    /// A rectangular duct: checked as drawn.
    pub rect: Option<(f64, f64)>,
    /// The diameter drawn (a rectangular duct's equivalent), mm, and
    /// the air's velocity in it, m/s.
    pub drawn_mm: f64,
    pub v_drawn: f64,
    /// The diameter proposed, mm, the velocity and the friction loss in
    /// it, Pa/m; none for a rectangular duct.
    pub proposed_mm: Option<f64>,
    pub v: f64,
    pub r_pa_m: f64,
    pub limit: f64,
}

impl Row {
    /// The duct as drawn keeps the air under the velocity allowed.
    pub fn ok(&self) -> bool {
        self.v_drawn <= self.limit + 1e-9
    }
}

/// The air's velocity in a round duct `d_mm` across, m/s.
pub fn velocity(flow_m3h: f64, d_mm: f64) -> f64 {
    let d = d_mm / 1000.0;
    flow_m3h / 3600.0 / (std::f64::consts::PI * d * d / 4.0)
}

/// Friction loss in a round duct, Pa/m.
pub fn friction(flow_m3h: f64, d_mm: f64) -> f64 {
    let d = d_mm / 1000.0;
    let v = velocity(flow_m3h, d_mm);
    if v <= 0.0 {
        return 0.0;
    }
    let re = v * d / NU;
    if re < 2300.0 {
        return 64.0 / re / d * RHO * v * v / 2.0;
    }
    // Colebrook, a few rounds from Swamee-Jain's start
    let mut l = 0.25 / (ROUGH / (3.7 * d) + 5.74 / re.powf(0.9)).log10().powi(2);
    for _ in 0..8 {
        l = 1.0 / (-2.0 * (ROUGH / (3.7 * d) + 2.51 / (re * l.sqrt())).log10()).powi(2);
    }
    l / d * RHO * v * v / 2.0
}

/// The smallest diameter of the series that keeps `flow_m3h` at or under
/// `v_max`.
pub fn round_for(flow_m3h: f64, v_max: f64) -> f64 {
    ROUND_MM
        .iter()
        .copied()
        .find(|d| velocity(flow_m3h, *d) <= v_max + 1e-9)
        .unwrap_or(ROUND_MM[ROUND_MM.len() - 1])
}

/// The ducts of air system `code`, sized; none when it has no ducts.
pub fn size(model: &Model, code: &str, o: &heatloss::Opts, limits: Limits) -> Option<Vec<Row>> {
    let t = tree::build(model, code)?;
    let below = t.terminals_below();
    let kids = t.children();
    let flow_of: Vec<f64> = t
        .terminals
        .iter()
        .map(|x| airbalance::terminal_flow(model, o, x.id))
        .collect();
    let mut rows = vec![];
    for (id, stretches) in t.by_pipe() {
        let flow = stretches
            .iter()
            .map(|&i| below[i].iter().map(|&k| flow_of[k]).sum::<f64>())
            .fold(0.0, f64::max);
        let length: f64 = stretches.iter().map(|&i| t.stretches[i].length()).sum();
        // a branch: nothing branches beyond its last stretch
        let branch = stretches
            .last()
            .is_some_and(|&i| kids[i].len() <= 1 && below[i].len() <= 1);
        let pipe = &t.stretches[stretches[0]].pipe;
        let limit = if branch { limits.branch } else { limits.main };
        let rect = pipe.height_mm.map(|h| (pipe.nominal_mm, h));
        let (drawn_mm, v_drawn) = match rect {
            Some((w, h)) => {
                // the equivalent diameter for the same friction
                let de = 1.3 * (w * h).powf(0.625) / (w + h).powf(0.25);
                (de, flow / 3600.0 / (w * h / 1.0e6))
            }
            None => (pipe.nominal_mm, velocity(flow, pipe.nominal_mm.max(1.0))),
        };
        let proposed_mm = rect.is_none().then(|| round_for(flow, limit));
        let d = proposed_mm.unwrap_or(drawn_mm);
        rows.push(Row {
            id,
            flow_m3h: flow,
            length,
            branch,
            rect,
            drawn_mm,
            v_drawn,
            proposed_mm,
            v: if rect.is_some() {
                v_drawn
            } else {
                velocity(flow, d)
            },
            r_pa_m: friction(flow, d),
            limit,
        });
    }
    Some(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_duct_takes_the_smallest_diameter_under_the_velocity() {
        // 150 m³/h at 2.5 m/s needs 146 mm: the series gives 160
        assert_eq!(round_for(150.0, 2.5), 160.0);
        assert!((velocity(150.0, 160.0) - 2.07).abs() < 0.01);
        // 300 m³/h at 4 m/s needs 163 mm: 200
        assert_eq!(round_for(300.0, 4.0), 200.0);
        assert_eq!(round_for(0.0, 2.5), 63.0);
    }

    #[test]
    fn friction_is_about_what_the_charts_give() {
        // 300 m³/h in a 200 mm spiral duct: 2.65 m/s, about 0.45 Pa/m
        let r = friction(300.0, 200.0);
        assert!(r > 0.35 && r < 0.6, "{r}");
        assert_eq!(friction(0.0, 200.0), 0.0);
    }
}
