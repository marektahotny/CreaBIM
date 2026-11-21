//! A HERZ differential pressure regulator (regulátor tlakovej
//! diferencie, RTD 4002 / 4007 / F 4007) with its partner valve
//! STRÖMAX, worked out as HERZ's own design tool does it ("Návrh
//! regulátora tlakovej diferencie", version 1.0, IX. 2025):
//!
//! - the regulator's setting is read off its characteristic lines: each
//!   setting holds a pressure difference that falls along a line from
//!   the least to the greatest flow; the setting is interpolated between
//!   the two lines that hold the guarded circuit's loss at the flow;
//! - a circuit losing less than the lowest setting holds leaves a
//!   residual pressure that shows as overflow;
//! - the regulator's body loses (Q/kvs)²; the partner valve takes the
//!   setting that throttles what is left, and the regulator's membrane
//!   the rest, which must not be below nothing;
//! - the pump must give the circuit, the regulator, the partner valve and
//!   the system up to the regulator.

#[path = "herzdp_data.rs"]
mod data;
pub use data::{CURVES, DN_LIMITS, FLOWS, PARTNERS, PARTNER_CODES, REGULATORS};

/// What is put in.
#[derive(Clone, Debug, PartialEq)]
pub struct DpInput {
    /// The guarded circuit's flow, l/h, and its pressure loss, kPa.
    pub flow_lh: f64,
    pub circuit_kpa: f64,
    /// Flanged rather than threaded.
    pub flanged: bool,
    /// The regulator's type and DN; none proposes them.
    pub kind: Option<String>,
    pub dn: Option<String>,
    /// The partner valve and its DN; none takes the regulator's.
    pub partner: Option<String>,
    pub partner_dn: Option<String>,
    /// The system's loss up to the regulator and the pump's head, kPa.
    pub system_kpa: f64,
    pub pump_kpa: f64,
}

impl Default for DpInput {
    fn default() -> Self {
        DpInput {
            flow_lh: 600.0,
            circuit_kpa: 13.0,
            flanged: false,
            kind: None,
            dn: None,
            partner: None,
            partner_dn: None,
            system_kpa: 10.0,
            pump_kpa: 80.0,
        }
    }
}

/// The partner valve's part.
#[derive(Clone, Debug, PartialEq)]
pub struct Partner {
    pub valve: String,
    pub dn: String,
    pub setting: f64,
    pub kv: f64,
    pub kpa: f64,
    pub order: String,
}

/// What comes out.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct DpResult {
    pub kind: String,
    pub dn: String,
    pub order: String,
    pub kvs: f64,
    /// The regulator's setting; none where no line holds the loss.
    pub setting: Option<f64>,
    /// The regulator body's loss, kPa.
    pub body_kpa: f64,
    /// The circuit's loss as the regulator holds it, kPa: the lowest
    /// setting's where the circuit loses less; the residual pressure.
    pub held_kpa: f64,
    pub residual_kpa: f64,
    pub partner: Option<Partner>,
    /// What the membrane throttles, kPa; below nothing the pump is short.
    pub membrane_kpa: f64,
    /// The least pump head, kPa.
    pub pump_min_kpa: f64,
    /// The DN a pipe carrying the flow has, for comparison.
    pub dn_estimate: Option<String>,
    /// The types and DNs that would do.
    pub suitable: Vec<(String, String)>,
    /// Warnings, in Slovak as HERZ's tool words them.
    pub notes: Vec<String>,
}

impl DpResult {
    /// Is it a sound design?
    pub fn ok(&self) -> bool {
        self.setting.is_some() && self.membrane_kpa >= 0.0
    }
}

/// The regulator types for threaded or flanged ends, in HERZ's order.
pub fn kinds(flanged: bool) -> Vec<&'static str> {
    let mut out: Vec<&str> = vec![];
    for c in CURVES {
        if c.0 == flanged && !out.contains(&c.1) {
            out.push(c.1);
        }
    }
    out
}

/// A type's DNs.
pub fn dns(kind: &str) -> Vec<&'static str> {
    let mut out: Vec<&str> = vec![];
    for c in CURVES {
        if c.1 == kind && !out.contains(&c.2) {
            out.push(c.2);
        }
    }
    out
}

/// The partner valves for threaded or flanged ends.
pub fn partners(flanged: bool) -> Vec<&'static str> {
    let mut out: Vec<&str> = vec![];
    for p in PARTNERS {
        if p.0 == flanged && !out.contains(&p.1) {
            out.push(p.1);
        }
    }
    out
}

/// A loss through kv at a flow: (Q/kv)², kPa.
fn loss(flow_lh: f64, kv: f64) -> f64 {
    if kv <= 0.0 {
        return f64::INFINITY;
    }
    (flow_lh / 1000.0 / kv).powi(2) * 100.0
}

/// The setting that holds `dp` at `flow` on a type's lines: the
/// setting, and the lowest line's pressure where `dp` is below it. None
/// where `dp` is above every line, or no line reaches the flow.
pub fn setting(kind: &str, dn: &str, flow: f64, dp: f64) -> Option<(f64, f64)> {
    // each setting's held pressure at the flow, within 3 % past its end
    let mut lines: Vec<(f64, f64)> = CURVES
        .iter()
        .filter(|c| c.1 == kind && c.2 == dn && flow <= c.6 * 1.03)
        .map(|c| {
            let (x1, y1, x2, y2) = (c.4, c.5, c.6, c.7);
            let v = if flow <= x1 || x2 <= x1 {
                y1
            } else {
                y1 + (y2 - y1) * (flow.min(x2) - x1) / (x2 - x1)
            };
            (c.3, v)
        })
        .collect();
    if lines.is_empty() {
        return None;
    }
    lines.sort_by(|a, b| a.1.total_cmp(&b.1));
    let (low, high) = (lines[0], lines[lines.len() - 1]);
    if dp > high.1 + 1e-9 {
        return None;
    }
    if dp <= low.1 {
        return Some((low.0, low.1));
    }
    for w in lines.windows(2) {
        let ((s0, v0), (s1, v1)) = (w[0], w[1]);
        if dp >= v0 && dp <= v1 {
            let s = if v1 > v0 {
                s0 + (dp - v0) * (s1 - s0) / (v1 - v0)
            } else {
                s0
            };
            return Some(((s * 10.0).round() / 10.0, dp));
        }
    }
    None
}

/// The DN a pipe carrying the flow has.
pub fn dn_estimate(flow: f64) -> Option<&'static str> {
    DN_LIMITS.iter().find(|d| flow <= d.1).map(|d| d.0)
}

fn dn_index(dn: &str) -> Option<usize> {
    DN_LIMITS.iter().position(|d| d.0 == dn)
}

/// Does the type take the flow?
fn takes(kind: &str, flow: f64) -> bool {
    FLOWS
        .iter()
        .find(|f| f.0 == kind)
        .is_none_or(|f| flow >= f.1 && flow <= f.2)
}

/// The regulator, its partner valve and the check of the pump.
pub fn design(i: &DpInput) -> DpResult {
    let mut r = DpResult {
        dn_estimate: dn_estimate(i.flow_lh).map(str::to_string),
        ..Default::default()
    };
    for kind in kinds(i.flanged) {
        if !takes(kind, i.flow_lh) {
            continue;
        }
        for dn in dns(kind) {
            if setting(kind, dn, i.flow_lh, i.circuit_kpa).is_some() {
                r.suitable.push((kind.to_string(), dn.to_string()));
            }
        }
    }
    // the type and DN asked for, else the suitable one nearest the
    // pipe's DN, the lower pressure series first
    let est = r.dn_estimate.as_deref().and_then(dn_index);
    let pick = match (&i.kind, &i.dn) {
        (Some(k), Some(d)) => Some((k.clone(), d.clone())),
        (Some(k), None) => r
            .suitable
            .iter()
            .filter(|s| &s.0 == k)
            .min_by_key(|s| dn_distance(&s.1, est))
            .cloned()
            .or_else(|| dns(k).first().map(|d| (k.clone(), d.to_string()))),
        _ => r
            .suitable
            .iter()
            .min_by_key(|s| dn_distance(&s.1, est))
            .cloned(),
    };
    let Some((kind, dn)) = pick else {
        r.notes
            .push("Pre zadaný prietok a tlakovú stratu nevyhovuje žiaden ventil RTD.".into());
        return r;
    };
    r.kind = kind.clone();
    r.dn = dn.clone();
    if let Some(s) = REGULATORS.iter().find(|s| s.0 == kind && s.1 == dn) {
        r.kvs = s.2;
        r.order = s.4.to_string();
    }
    if !takes(&kind, i.flow_lh) {
        r.notes.push(match FLOWS.iter().find(|f| f.0 == kind) {
            Some(f) if i.flow_lh > f.2 => {
                "Potrebný typ ventilu pre vyššie prietoky (F 4007).".into()
            }
            _ => "Potrebný ventil pre nižšie prietoky (4002, 4007).".into(),
        });
    }
    match setting(&kind, &dn, i.flow_lh, i.circuit_kpa) {
        Some((s, held)) => {
            r.setting = Some(s);
            r.held_kpa = held.max(i.circuit_kpa);
            r.residual_kpa = (held - i.circuit_kpa).max(0.0);
            if r.residual_kpa > 0.35 {
                r.notes.push(format!(
                    "Tlaková strata sústavy je nižšia ako nastaviteľné minimum, vzniká zvyškový dispozičný tlak {} kPa, ktorý sa môže prejaviť ako nadprietok. Zvýšte tlakovú stratu sústavy alebo použite RTD pre nižšie tlakové rady.",
                    comma(r.residual_kpa)
                ));
            }
        }
        None => {
            r.held_kpa = i.circuit_kpa;
            let reaches = CURVES
                .iter()
                .any(|c| c.1 == kind && c.2 == dn && i.flow_lh <= c.6 * 1.03);
            r.notes.push(if reaches {
                "Strážený okruh má tlakovú stratu vyššiu ako je maximálna nastaviteľná tlaková diferencia pre navrhovaný ventil, vyberte iný typ ventilov RTD s vyššou tlakovou radou.".into()
            } else {
                "Pracovný bod sústavy sa nachádza mimo použiteľného rozsahu ventilu, skúste zvoliť väčšiu DN alebo zvýšiť tlakovú stratu sústavy, prípadne RTD s vyšším tlakovým radom.".into()
            });
        }
    }
    r.body_kpa = loss(i.flow_lh, r.kvs);
    // the partner valve: the one asked for, else STRÖMAX GM threaded,
    // GMF flanged up to DN 150, GF above
    let valve = i.partner.clone().unwrap_or_else(|| {
        if !i.flanged {
            "STROMAX GM".into()
        } else if dn_index(&dn).is_some_and(|x| dn_index("DN150").is_some_and(|y| x > y)) {
            "STROMAX GF".into()
        } else {
            "STROMAX GMF".into()
        }
    });
    let pdn = i.partner_dn.clone().unwrap_or_else(|| dn.clone());
    let available = i.pump_kpa - i.system_kpa;
    let throttle = available - r.body_kpa - r.held_kpa;
    let options: Vec<(f64, f64, f64)> = PARTNERS
        .iter()
        .filter(|p| p.1 == valve && p.2 == pdn)
        .map(|p| (p.3, p.4, loss(i.flow_lh, p.4)))
        .filter(|o| o.2 > 0.0 && o.2 <= 50.0)
        .collect();
    let best = options
        .iter()
        .filter(|o| o.2 <= throttle)
        .max_by(|a, b| a.2.total_cmp(&b.2))
        .or_else(|| {
            options
                .iter()
                .filter(|o| o.2 > throttle)
                .min_by(|a, b| a.2.total_cmp(&b.2))
        });
    match best {
        Some(&(setting, kv, kpa)) => {
            r.partner = Some(Partner {
                order: PARTNER_CODES
                    .iter()
                    .find(|c| c.0 == valve && c.1 == pdn)
                    .map_or(String::new(), |c| c.2.to_string()),
                valve,
                dn: pdn,
                setting,
                kv,
                kpa,
            })
        }
        None => r
            .notes
            .push("Partnerský ventil tejto DN pre daný prietok nevyhovuje.".into()),
    }
    let partner_kpa = r.partner.as_ref().map_or(0.0, |p| p.kpa);
    r.membrane_kpa = available - r.body_kpa - partner_kpa - r.held_kpa;
    r.pump_min_kpa = ((r.body_kpa + partner_kpa + r.held_kpa) * 10.0).round() / 10.0 + i.system_kpa;
    if r.membrane_kpa < 0.0 {
        r.notes.push(format!(
            "Nedostatočný výtlak čerpadla: potrebných je najmenej {} kPa.",
            comma(r.pump_min_kpa)
        ));
    }
    if let (Some(a), Some(b)) = (dn_index(&dn), est) {
        if a.abs_diff(b) > 1 {
            r.notes
                .push("Veľký rozdiel medzi DN RTD a DN sústavy.".into());
        }
    }
    r
}

fn comma(v: f64) -> String {
    format!("{v:.1}").replace('.', ",")
}

fn dn_distance(dn: &str, est: Option<usize>) -> usize {
    match (dn_index(dn), est) {
        (Some(a), Some(b)) => a.abs_diff(b),
        _ => usize::MAX / 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tool's own example: 600 l/h, 13 kPa, threaded 4007 DN 20,
    /// system 10 kPa, pump 80 kPa.
    #[test]
    fn the_tools_example_designs_a_4007_dn20() {
        let i = DpInput {
            kind: Some("4007 (5-30kPa)".into()),
            dn: Some("DN20".into()),
            ..Default::default()
        };
        let r = design(&i);
        assert_eq!(r.order, "1400702");
        let s = r.setting.expect("a setting");
        assert!(s > 50.0 && s < 300.0, "{s}");
        // body loss (0,6 / 5,9)² · 100
        assert!((r.body_kpa - 1.0342).abs() < 1e-3, "{}", r.body_kpa);
        let p = r.partner.as_ref().unwrap();
        assert_eq!(p.valve, "STROMAX GM");
        assert!(r.membrane_kpa >= 0.0 && r.ok(), "{r:?}");
        assert!((r.pump_min_kpa - (r.body_kpa + p.kpa + r.held_kpa + 10.0)).abs() < 0.06);
    }

    #[test]
    fn it_proposes_a_regulator_and_says_when_none_will_do() {
        let r = design(&DpInput::default());
        assert!(!r.kind.is_empty() && !r.suitable.is_empty());
        assert!(r.setting.is_some());
        let r = design(&DpInput {
            circuit_kpa: 190.0,
            ..Default::default()
        });
        assert!(r.kind.is_empty() && !r.notes.is_empty());
    }

    #[test]
    fn a_low_loss_leaves_a_residual_pressure() {
        let r = design(&DpInput {
            kind: Some("4002 (25-60kPa)".into()),
            dn: Some("DN20".into()),
            circuit_kpa: 5.0,
            ..Default::default()
        });
        assert!(r.residual_kpa > 5.0, "{r:?}");
        assert!(r.notes.iter().any(|n| n.contains("zvyškový")));
    }
}
