//! The circuits of a model's electrical installation (after
//! PilonDesigner's `elektro.ts`): each circuit's installed power, design
//! current, length and check ([`crate::electrical`], STN 33 2000-5-52),
//! where a 30 mA residual current device is asked for (STN 33 2000-4-41
//! art. 411.3.3 and 411.3.4, STN 33 2000-7-701), the balance of power
//! over the phases, and a proposal of circuits for devices that have
//! none.

use crate::electrical::{self as el, Check, Method, BREAKER_IN, IZ_S, UZ};
use creabim_core::electrical::{self as core_el, Circuit, DeviceKind, Load};
use creabim_core::{Element, ElementId, ElementKind, Model};
use glam::DVec2;

/// The power factor taken where none is given (STN 33 2000-5-52 annex G).
pub const COS_PHI: f64 = 0.8;

/// One circuit, reckoned.
#[derive(Clone, Debug, PartialEq)]
pub struct CircuitResult {
    pub id: ElementId,
    pub name: String,
    pub three_phase: bool,
    pub phase: Option<u8>,
    /// How many devices it feeds, and how many of them draw power.
    pub devices: usize,
    pub loads: usize,
    /// Installed power, W, and how many loads have none given.
    pub power_w: f64,
    pub missing_power: usize,
    /// Design current, A.
    pub ib: f64,
    /// The cable's length, m, and whether it was measured from the plan.
    pub length: f64,
    pub measured: bool,
    /// The check, once the breaker and the cable are given.
    pub check: Option<Check>,
    /// Why a 30 mA RCD is asked for, and whether it has one.
    pub rcd_required: Option<&'static str>,
    pub ok_rcd: Option<bool>,
    pub issues: Vec<String>,
}

impl CircuitResult {
    pub fn ok(&self) -> bool {
        self.issues.is_empty()
    }
}

fn room_named(model: &Model, level: ElementId, at: DVec2, words: &[&str]) -> bool {
    model.iter().any(|e| match &e.kind {
        ElementKind::Room(r) if r.level == level => {
            let name = e.name.to_lowercase();
            words.iter().any(|w| name.contains(w))
                && creabim_core::room::shape_of(model, r)
                    .is_some_and(|s| creabim_core::polygon::contains(&s.outline, at))
        }
        _ => false,
    })
}

const BATH: [&str; 4] = ["kúpe", "kupe", "bath", "sprch"];
const KITCHEN: [&str; 2] = ["kuch", "kitchen"];

/// The devices a circuit feeds that are still in the model.
fn fed<'a>(
    model: &'a Model,
    c: &Circuit,
) -> Vec<(&'a Element, &'a creabim_core::Object, DeviceKind, f64)> {
    c.loads
        .iter()
        .filter_map(|l| {
            let e = model.get(l.device)?;
            let ElementKind::Object(o) = &e.kind else {
                return None;
            };
            Some((e, o, core_el::device_kind(model, e)?, l.power_w))
        })
        .collect()
}

/// Why circuit `c` needs a 30 mA RCD, if it does.
pub fn rcd_requirement(model: &Model, c: &Circuit) -> Option<&'static str> {
    let devs = fed(model, c);
    if devs
        .iter()
        .any(|(_, o, ..)| room_named(model, o.level, o.position, &BATH))
    {
        return Some("bathroom - STN 33 2000-7-701");
    }
    if devs.iter().any(|d| d.2 == DeviceKind::Socket) && c.breaker_a.is_none_or(|a| a <= 32.0) {
        return Some("sockets up to 32 A - STN 33 2000-4-41 art. 411.3.3");
    }
    if devs.iter().any(|d| d.2 == DeviceKind::Light) {
        return Some("lights in a dwelling - STN 33 2000-4-41 art. 411.3.4");
    }
    None
}

/// The cable's length measured square (along x and y, and up) from the
/// board to the farthest device.
pub fn measured_length(model: &Model, c: &Circuit) -> f64 {
    let Some(board) = c.board.and_then(|b| model.get(b)) else {
        return 0.0;
    };
    let ElementKind::Object(b) = &board.kind else {
        return 0.0;
    };
    let zb = creabim_core::objects::base_z(model, board).unwrap_or(0.0);
    fed(model, c)
        .iter()
        .map(|(e, o, ..)| {
            let z = creabim_core::objects::base_z(model, e).unwrap_or(0.0);
            let d = o.position - b.position;
            d.x.abs() + d.y.abs() + (z - zb).abs()
        })
        .fold(0.0, f64::max)
}

/// Reckon one circuit.
pub fn result(model: &Model, e: &Element, c: &Circuit, method: Method) -> CircuitResult {
    let devs = fed(model, c);
    let loads: Vec<_> = devs.iter().filter(|d| d.2.is_load()).collect();
    let power_w: f64 = loads.iter().map(|d| d.3).sum();
    let missing_power = loads.iter().filter(|d| d.3 <= 0.0).count();
    let ib = el::design_current(power_w, COS_PHI, c.three_phase);
    let measured = c.length.is_none();
    let length = c.length.unwrap_or_else(|| measured_length(model, c));
    let mut issues = vec![];
    if missing_power > 0 {
        issues.push(format!("without power: {missing_power}"));
    }
    if c.board.and_then(|b| model.get(b)).is_none() {
        issues.push("without a board".to_string());
    }
    let check = match (c.breaker_a, c.cable_mm2) {
        (Some(in_a), Some(s)) => {
            let k = el::check(&el::Circuit {
                power_w,
                cos_phi: COS_PHI,
                three_phase: c.three_phase,
                length_m: length,
                s_mm2: s,
                in_a,
                curve: el::Curve::B,
                aluminium: false,
                method,
                lighting: c.lighting,
                supply: el::Supply::Public,
                ze: None,
            });
            if !k.ok_du {
                issues.push(format!("voltage drop {:.2} % > {} %", k.du, k.du_limit));
            }
            if k.ok_prot == Some(false) {
                issues.push(if ib > in_a {
                    format!("IB {ib:.1} A > In {in_a} A")
                } else {
                    format!("In {in_a} A > Iz {} A", k.iz.unwrap_or(0.0))
                });
            }
            Some(k)
        }
        (None, _) => {
            issues.push("give the breaker In".to_string());
            None
        }
        (_, None) => {
            issues.push("give the cable's cross-section".to_string());
            None
        }
    };
    let rcd_required = rcd_requirement(model, c);
    let ok_rcd = rcd_required.map(|_| c.rcd_ma.is_some_and(|ma| ma <= 30.0));
    if ok_rcd == Some(false) {
        issues.push("a 30 mA RCD is missing".to_string());
    }
    CircuitResult {
        id: e.id,
        name: e.name.clone(),
        three_phase: c.three_phase,
        phase: c.phase,
        devices: devs.len(),
        loads: loads.len(),
        power_w,
        missing_power,
        ib,
        length,
        measured,
        check,
        rcd_required,
        ok_rcd,
        issues,
    }
}

/// Every circuit of the model, reckoned.
pub fn results(model: &Model, method: Method) -> Vec<CircuitResult> {
    core_el::circuits(model)
        .into_iter()
        .map(|(e, c)| result(model, e, c, method))
        .collect()
}

/// The balance of the building's power.
#[derive(Clone, Debug, PartialEq)]
pub struct Balance {
    /// Installed power Pi, W, and the power at once Pp = β·Pi.
    pub pi: f64,
    pub pp: f64,
    /// The design current of a three-phase supply, A.
    pub ip: f64,
    /// Installed power and current on each phase.
    pub phase_p: [f64; 3],
    pub phase_i: [f64; 3],
    /// Single-phase circuits with power and no phase yet.
    pub unphased: usize,
    /// The largest departure of a phase's current from the mean, %.
    pub unbalance: Option<f64>,
    /// The next preferred breaker at or over Ip.
    pub main_breaker: Option<f64>,
}

/// The balance of `results` with the simultaneity `beta` (the designer's).
pub fn balance(results: &[CircuitResult], beta: f64) -> Balance {
    let pi: f64 = results.iter().map(|r| r.power_w).sum();
    let pp = beta.clamp(0.0, 1.0) * pi;
    let ip = pp / (3.0_f64.sqrt() * UZ * COS_PHI);
    let (mut phase_p, mut phase_i, mut unphased) = ([0.0; 3], [0.0; 3], 0);
    for r in results {
        if r.three_phase {
            for k in 0..3 {
                phase_p[k] += r.power_w / 3.0;
                phase_i[k] += r.ib;
            }
        } else if let Some(p) = r.phase.filter(|p| (1..=3).contains(p)) {
            phase_p[p as usize - 1] += r.power_w;
            phase_i[p as usize - 1] += r.ib;
        } else if r.power_w > 0.0 {
            unphased += 1;
        }
    }
    let mean = phase_i.iter().sum::<f64>() / 3.0;
    let unbalance = (mean > 0.0)
        .then(|| 100.0 * phase_i.iter().map(|i| (i - mean).abs()).fold(0.0, f64::max) / mean);
    Balance {
        pi,
        pp,
        ip,
        phase_p,
        phase_i,
        unphased,
        unbalance,
        main_breaker: (pp > 0.0).then(|| el::next_std(&BREAKER_IN, ip)).flatten(),
    }
}

/// The phases of single-phase circuits that have none: the largest
/// current first, each onto the phase loaded least. Returns the circuits
/// changed, with their phase.
pub fn balance_phases(results: &[CircuitResult]) -> Vec<(ElementId, u8)> {
    let mut i = [0.0; 3];
    for r in results {
        if r.three_phase {
            i.iter_mut().for_each(|x| *x += r.ib);
        } else if let Some(p) = r.phase.filter(|p| (1..=3).contains(p)) {
            i[p as usize - 1] += r.ib;
        }
    }
    let mut one: Vec<&CircuitResult> = results
        .iter()
        .filter(|r| !r.three_phase && r.phase.is_none())
        .collect();
    one.sort_by(|a, b| b.ib.total_cmp(&a.ib).then(a.name.cmp(&b.name)));
    let mut out = vec![];
    for r in one {
        let k = (0..3).min_by(|a, b| i[*a].total_cmp(&i[*b])).unwrap_or(0);
        i[k] += r.ib;
        out.push((r.id, k as u8 + 1));
    }
    out
}

/// The smallest preferred breaker at or over `ib` and `min_in`, and the
/// smallest cable whose Iz carries it.
fn sized(ib: f64, min_in: f64, three_phase: bool, method: Method) -> (f64, f64) {
    let in_a = el::next_std(&BREAKER_IN, ib.max(min_in)).unwrap_or(125.0);
    let s = IZ_S
        .iter()
        .copied()
        .find(|s| el::iz_table(method, three_phase, *s).is_some_and(|iz| iz >= in_a))
        .unwrap_or(25.0);
    (in_a, s)
}

/// Circuits for the devices that have none (the designer then adjusts
/// them): a level's lights one lighting circuit (split where IB would
/// pass 10 A), its sockets by rooms at most `max_sockets` to a circuit
/// (bathrooms and kitchens their own), each fixed appliance its own;
/// breakers and cables the smallest that carry them, a 30 mA RCD where
/// one is asked for, switches with the nearest light's circuit. None
/// when there is no board or nothing left to assign.
pub fn propose(model: &Model, max_sockets: usize, method: Method) -> Result<Vec<Element>, String> {
    let all = core_el::devices(model);
    let boards: Vec<_> = all.iter().filter(|d| d.2 == DeviceKind::Board).collect();
    if boards.is_empty() {
        return Err("Place a distribution board first.".into());
    }
    let taken: std::collections::HashSet<ElementId> = core_el::circuits(model)
        .iter()
        .flat_map(|(_, c)| c.loads.iter().map(|l| l.device))
        .collect();
    let free: Vec<_> = all
        .iter()
        .filter(|d| d.2 != DeviceKind::Board && d.2 != DeviceKind::Weak && !taken.contains(&d.0.id))
        .collect();
    if !free.iter().any(|d| d.2.is_load()) {
        return Err("Every device has a circuit.".into());
    }
    let mut n = core_el::circuits(model).len();
    let mut out: Vec<Element> = vec![];
    let mut make = |devs: &[&(&Element, &creabim_core::Object, DeviceKind)],
                    lighting: bool,
                    out: &mut Vec<Element>| {
        let level = devs[0].1.level;
        let board = boards
            .iter()
            .find(|b| b.1.level == level)
            .unwrap_or(&boards[0]);
        let (in_a, s) = sized(0.0, if lighting { 10.0 } else { 16.0 }, false, method);
        let mut c = Circuit {
            board: Some(board.0.id),
            lighting,
            breaker_a: Some(in_a),
            cable_mm2: Some(s),
            loads: devs
                .iter()
                .map(|d| Load {
                    device: d.0.id,
                    power_w: 0.0,
                })
                .collect(),
            ..Default::default()
        };
        if rcd_requirement(model, &c).is_some() {
            c.rcd_ma = Some(30.0);
        }
        n += 1;
        out.push(Element::new(format!("{n}"), ElementKind::Circuit(c)));
    };
    let mut levels: Vec<ElementId> = vec![];
    for d in &free {
        if !levels.contains(&d.1.level) {
            levels.push(d.1.level);
        }
    }
    for level in levels {
        let here: Vec<_> = free
            .iter()
            .filter(|d| d.1.level == level)
            .copied()
            .collect();
        let lights: Vec<_> = here
            .iter()
            .filter(|d| d.2 == DeviceKind::Light)
            .copied()
            .collect();
        if !lights.is_empty() {
            make(&lights, true, &mut out);
        }
        for d in here.iter().filter(|d| d.2 == DeviceKind::Appliance) {
            make(&[*d], false, &mut out);
        }
        // sockets by rooms: 0 a bathroom, 1 a kitchen, 2 the rest
        let special = |d: &(&Element, &creabim_core::Object, DeviceKind)| {
            if room_named(model, level, d.1.position, &BATH) {
                0
            } else if room_named(model, level, d.1.position, &KITCHEN) {
                1
            } else {
                2
            }
        };
        let mut socks: Vec<_> = here
            .iter()
            .filter(|d| d.2 == DeviceKind::Socket)
            .map(|d| (special(d), *d))
            .collect();
        socks.sort_by(|a, b| {
            a.0.cmp(&b.0)
                .then(a.1 .1.position.x.total_cmp(&b.1 .1.position.x))
        });
        let mut acc: Vec<&(&Element, &creabim_core::Object, DeviceKind)> = vec![];
        let mut last = None;
        for (k, d) in socks {
            if !acc.is_empty() && (acc.len() >= max_sockets.max(1) || last != Some(k)) {
                make(&acc, false, &mut out);
                acc.clear();
            }
            acc.push(d);
            last = Some(k);
        }
        if !acc.is_empty() {
            make(&acc, false, &mut out);
        }
    }
    // switches go with the nearest light's new circuit on their level
    for sw in free.iter().filter(|d| d.2 == DeviceKind::Switch) {
        let mut best: Option<(f64, usize)> = None;
        for (i, e) in out.iter().enumerate() {
            let ElementKind::Circuit(c) = &e.kind else {
                continue;
            };
            if !c.lighting {
                continue;
            }
            for l in &c.loads {
                if let Some(ElementKind::Object(o)) = model.get(l.device).map(|e| &e.kind) {
                    if o.level == sw.1.level {
                        let d = o.position.distance(sw.1.position);
                        if best.is_none_or(|b| d < b.0) {
                            best = Some((d, i));
                        }
                    }
                }
            }
        }
        if let Some((_, i)) = best {
            if let ElementKind::Circuit(c) = &mut out[i].kind {
                c.loads.push(Load {
                    device: sw.0.id,
                    power_w: 0.0,
                });
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(name: &str, p: f64, three: bool, phase: Option<u8>) -> CircuitResult {
        CircuitResult {
            id: ElementId::new(),
            name: name.into(),
            three_phase: three,
            phase,
            devices: 1,
            loads: 1,
            power_w: p,
            missing_power: 0,
            ib: el::design_current(p, COS_PHI, three),
            length: 10.0,
            measured: true,
            check: None,
            rcd_required: None,
            ok_rcd: None,
            issues: vec![],
        }
    }

    #[test]
    fn phases_take_the_largest_circuits_first() {
        let rs = [
            r("1", 2000.0, false, None),
            r("2", 1000.0, false, None),
            r("3", 900.0, false, None),
            r("4", 100.0, false, None),
        ];
        let set = balance_phases(&rs);
        assert_eq!(set.len(), 4);
        // 2000 on L1, 1000 on L2, 900 on L3, then 100 on the least: L3
        assert_eq!(set[0], (rs[0].id, 1));
        assert_eq!(set[1], (rs[1].id, 2));
        assert_eq!(set[2], (rs[2].id, 3));
        assert_eq!(set[3], (rs[3].id, 3));
    }

    #[test]
    fn the_balance_sums_power_and_proposes_the_main_breaker() {
        let rs = [
            r("1", 3680.0, false, Some(1)),
            r("2", 6000.0, true, None),
            r("3", 500.0, false, None),
        ];
        let b = balance(&rs, 0.5);
        assert!((b.pi - 10180.0).abs() < 1e-9);
        assert!((b.pp - 5090.0).abs() < 1e-9);
        assert_eq!(b.unphased, 1);
        // Ip = 5090 / (1.732 * 400 * 0.8) = 9.18 A -> 10 A
        assert_eq!(b.main_breaker, Some(10.0));
        assert!((b.phase_p[0] - 5680.0).abs() < 1e-9 && (b.phase_p[1] - 2000.0).abs() < 1e-9);
        assert!(b.unbalance.unwrap() > 0.0);
    }

    #[test]
    fn the_sample_house_gets_circuits_once_it_has_a_board() {
        let model = creabim_core::sample::sample_house();
        // nothing electrical in it, or no board: no proposal
        assert!(propose(&model, 10, Method::A2).is_err());
        let (in_a, s) = sized(0.0, 16.0, false, Method::A2);
        assert_eq!((in_a, s), (16.0, 2.5));
        let (in_a, s) = sized(0.0, 10.0, false, Method::A2);
        assert_eq!((in_a, s), (10.0, 1.5));
    }
}
