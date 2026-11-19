//! Foul drainage, as PilonDesigner's `SanitaryAnalyzer` (STN EN 12056-2,
//! system I, STN 75 6760): the waste flow Qww = K·√ΣDU, no less than the
//! largest fixture's DU; a horizontal pipe carries it half full on its
//! fall, a connecting pipe is its fixture's DN, a stack is sized by the
//! capacity of its DN; nothing downstream of a WC under DN 100, and no
//! pipe smaller than one draining into it.

use crate::model::{self, ModelNetwork};
use crate::network::{NodeKind, PipeSize};
use crate::settings::Settings;
use crate::storm::{fill_for, partial_flow};
use creabim_core::{ElementId, Model};

/// The design of a drainage system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrainSettings {
    /// The frequency factor K (0.5 homes, 0.7 schools and hospitals,
    /// 1.0 public toilets, 1.2 laboratories).
    pub k: f64,
    /// The filling of a horizontal pipe, h/d.
    pub fill: f64,
    pub min_slope: f64,
    pub min_slope_branch: f64,
    /// The fall taken where none is drawn.
    pub slope_if_none: f64,
    pub kb_mm: f64,
    /// Below it, solids may settle, m/s.
    pub min_velocity: f64,
    /// The least DN downstream of a WC.
    pub min_dn_wc: f64,
    /// Branches swept into the stack (its greater capacity).
    pub swept: bool,
}

impl Default for DrainSettings {
    fn default() -> Self {
        DrainSettings {
            k: 0.5,
            fill: 0.5,
            min_slope: 0.01,
            min_slope_branch: 0.02,
            slope_if_none: 0.02,
            kb_mm: 1.0,
            min_velocity: 0.7,
            min_dn_wc: 100.0,
            swept: false,
        }
    }
}

/// A pipe's DN from its inner diameter (STN EN 12056-2, table 2 after
/// PilonDesigner's).
pub fn dn_of_inner(inner_mm: f64) -> f64 {
    const T: [(f64, f64); 15] = [
        (30.0, 26.0),
        (40.0, 34.0),
        (50.0, 44.0),
        (56.0, 49.0),
        (60.0, 56.0),
        (70.0, 68.0),
        (80.0, 75.0),
        (90.0, 79.0),
        (100.0, 96.0),
        (125.0, 113.0),
        (150.0, 146.0),
        (200.0, 184.0),
        (225.0, 207.0),
        (250.0, 230.0),
        (300.0, 290.0),
    ];
    T.iter()
        .filter(|t| inner_mm + 0.5 >= t.1)
        .map(|t| t.0)
        .fold(0.0, f64::max)
}

/// A stack's capacity, l/s, by its DN (STN EN 12056-2, table 11): with
/// branches entering squarely or swept.
pub fn stack_capacity(dn: f64, swept: bool) -> f64 {
    const T: [(f64, f64, f64); 8] = [
        (60.0, 0.5, 0.7),
        (70.0, 1.5, 2.0),
        (80.0, 2.0, 2.6),
        (90.0, 2.7, 3.5),
        (100.0, 4.0, 5.2),
        (125.0, 5.8, 7.6),
        (150.0, 9.5, 12.4),
        (200.0, 16.0, 21.0),
    ];
    T.iter()
        .filter(|t| dn >= t.0)
        .map(|t| if swept { t.2 } else { t.1 })
        .fold(0.0, f64::max)
}

#[derive(Clone, Debug, PartialEq)]
pub struct PipeResult {
    pub id: ElementId,
    pub flow_ls: f64,
    pub sum_du: f64,
    pub fixtures: usize,
    pub stack: bool,
    pub size: String,
    pub proposed: String,
    pub proposed_mm: f64,
    /// The DN its fixtures or a WC ask for.
    pub required_dn: f64,
    /// Its fall, as drawn or as taken.
    pub slope: f64,
    pub no_slope: bool,
    pub too_flat: bool,
    /// h/d it runs at, and its velocity.
    pub fill: f64,
    pub velocity_ms: f64,
    /// What it carries: half full, or a stack's capacity, l/s.
    pub capacity_ls: f64,
    /// Too slow: solids may settle.
    pub slow: bool,
    /// Even the largest size is too small.
    pub over: bool,
    /// A Wavin AS+ or SiTech+ stack's installation noise at its flow,
    /// dB(A), with the sound-insulating clip and with the double one
    /// (Wavin's tests at 2 and 4 l/s, P-BA 62 to 64/2019, 9/2020).
    pub noise_db: Option<(f64, f64)>,
}

/// The installation noise of a Wavin stack at a flow, dB(A): with the
/// sound-insulating clip and with the double clip; between Wavin's
/// figures at 2 and 4 l/s along a line, held at 2 l/s below.
pub fn stack_noise(pipe_type: &str, flow_ls: f64) -> Option<(f64, f64)> {
    let ((a2, a4), (b2, b4)) = match pipe_type {
        "WAS" => ((14.0, 19.0), (10.0, 13.0)),
        "WSI" => ((21.0, 25.0), (13.0, 16.0)),
        _ => return None,
    };
    let f = ((flow_ls - 2.0) / 2.0).max(0.0);
    Some((a2 + (a4 - a2) * f, b2 + (b4 - b2) * f))
}

/// The limit Wavin's figures are held against: DIN 4109, 30 dB(A).
pub const NOISE_LIMIT_DB: f64 = 30.0;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrainResult {
    pub pipes: Vec<PipeResult>,
    pub sum_du: f64,
    pub fixtures: usize,
    /// Qww at the outlet, l/s.
    pub flow_ls: f64,
    pub outlet_dn: f64,
    pub outlet_fill: f64,
    pub outlet_velocity: f64,
    pub stacks: usize,
    pub unmatched: Vec<String>,
    pub unreached: usize,
    pub open_ends: usize,
}

/// Foul drainage `code` of the model, calculated.
pub fn run(model: &Model, code: &str, fx: &Settings, s: &DrainSettings) -> Option<DrainResult> {
    let m = model::build(model, code, fx)?;
    Some(calc(&m, s))
}

fn slope_of(p1: [f64; 3], p2: [f64; 3]) -> f64 {
    let run = ((p2[0] - p1[0]).powi(2) + (p2[1] - p1[1]).powi(2)).sqrt();
    if run < 1e-6 {
        0.0
    } else {
        (p2[2] - p1[2]).abs() / run
    }
}

fn calc(m: &ModelNetwork, s: &DrainSettings) -> DrainResult {
    let net = &m.net;
    let n = net.nodes.len();
    let mut out = DrainResult {
        unreached: m.unreached.len(),
        open_ends: m.open_ends.len(),
        ..DrainResult::default()
    };
    // what each node drains: ΣDU, the largest DU, the largest fixture
    // DN, a WC, fixtures
    let mut du = vec![0.0; n];
    let mut max_du = vec![0.0f64; n];
    let mut max_dn = vec![0.0f64; n];
    let mut wc = vec![false; n];
    let mut count = vec![0usize; n];
    for i in net.post_order() {
        let node = &net.nodes[i];
        if let Some(f) = &node.fixture {
            du[i] = f.du;
            max_du[i] = f.du;
            max_dn[i] = f.drain_dn.max(f.connector_nominal_mm);
            wc[i] = f.is_wc;
            count[i] = 1;
            if !f.matched {
                out.unmatched.push(node.name.clone());
            }
        }
        for &c in &node.children {
            du[i] += du[c];
            max_du[i] = max_du[i].max(max_du[c]);
            max_dn[i] = max_dn[i].max(max_dn[c]);
            wc[i] |= wc[c];
            count[i] += count[c];
        }
    }
    let q: Vec<f64> = (0..n)
        .map(|i| {
            let qww = if du[i] > 0.0 { s.k * du[i].sqrt() } else { 0.0 };
            qww.max(max_du[i])
        })
        .collect();
    // sizes, upstream first
    let mut pick: Vec<Option<PipeSize>> = vec![None; n];
    let mut over = vec![false; n];
    let mut slope = vec![0.0; n];
    let mut no_slope = vec![false; n];
    let mut too_flat = vec![false; n];
    let mut need = vec![0.0; n];
    for i in 0..n {
        let node = &net.nodes[i];
        if node.kind != NodeKind::Pipe {
            continue;
        }
        let mut sizes: Vec<PipeSize> = node
            .sizes
            .iter()
            .copied()
            .filter(|z| z.inner_mm > 0.0)
            .collect();
        sizes.sort_by(|a, b| a.inner_mm.total_cmp(&b.inner_mm));
        let rule = max_dn[i].max(if wc[i] { s.min_dn_wc } else { 0.0 });
        let fits_rule = |z: &PipeSize| dn_of_inner(z.inner_mm) + 1e-6 >= rule;
        if node.is_vertical() {
            slope[i] = 1.0;
            let cap_dn = if count[i] <= 1 {
                0.0
            } else {
                [60.0, 70.0, 80.0, 90.0, 100.0, 125.0, 150.0, 200.0]
                    .into_iter()
                    .find(|&d| stack_capacity(d, s.swept) >= q[i])
                    .unwrap_or_else(|| {
                        over[i] = true;
                        200.0
                    })
            };
            need[i] = rule.max(cap_dn);
            pick[i] = sizes
                .iter()
                .copied()
                .find(|z| dn_of_inner(z.inner_mm) + 1e-6 >= need[i])
                .or(sizes.last().copied());
            continue;
        }
        let connecting = count[i] <= 1;
        let mut sl = slope_of(node.p1, node.p2);
        let least = if connecting {
            s.min_slope_branch
        } else {
            s.min_slope
        };
        if sl < 1e-4 {
            no_slope[i] = true;
            sl = s.slope_if_none;
        } else if sl + 1e-6 < least {
            too_flat[i] = true;
        }
        slope[i] = sl;
        need[i] = rule;
        pick[i] = sizes
            .iter()
            .copied()
            .find(|z| {
                fits_rule(z)
                    && (connecting || partial_flow(z.inner_mm, s.fill, sl, s.kb_mm).0 >= q[i])
            })
            .or_else(|| {
                over[i] = !sizes.is_empty();
                sizes.last().copied()
            });
    }
    // no pipe smaller than one draining into it
    for i in net.post_order() {
        if net.nodes[i].kind != NodeKind::Pipe {
            continue;
        }
        // the nearest pipes upstream of it
        let mut stack: Vec<usize> = net.nodes[i].children.clone();
        let mut biggest: Option<PipeSize> = None;
        while let Some(c) = stack.pop() {
            if net.nodes[c].kind == NodeKind::Pipe {
                if let Some(z) = pick[c] {
                    if biggest.is_none_or(|b| z.inner_mm > b.inner_mm) {
                        biggest = Some(z);
                    }
                }
            } else {
                stack.extend(net.nodes[c].children.iter().copied());
            }
        }
        if let (Some(b), Some(me)) = (biggest, pick[i]) {
            if me.inner_mm + 1e-6 < b.inner_mm {
                let up = net.nodes[i]
                    .sizes
                    .iter()
                    .copied()
                    .filter(|z| z.inner_mm + 1e-6 >= b.inner_mm)
                    .min_by(|x, y| x.inner_mm.total_cmp(&y.inner_mm));
                pick[i] = up.or(Some(b));
            }
        }
    }
    // how each runs
    let mut res: Vec<Option<(f64, f64, f64)>> = vec![None; n];
    for i in 0..n {
        let node = &net.nodes[i];
        if node.kind != NodeKind::Pipe {
            continue;
        }
        let di = pick[i].map_or(node.inner_diameter_m * 1000.0, |z| z.inner_mm);
        if node.is_vertical() {
            let cap = stack_capacity(dn_of_inner(di), s.swept);
            let v = crate::hydraulics::velocity(q[i], di / 1000.0);
            res[i] = Some((cap, 0.0, v));
        } else {
            let cap = partial_flow(di, s.fill, slope[i], s.kb_mm).0;
            let f = fill_for(q[i], di, slope[i], s.kb_mm);
            let v = if q[i] > 0.0 {
                partial_flow(di, f.min(0.94), slope[i], s.kb_mm).1
            } else {
                0.0
            };
            res[i] = Some((cap, f, v));
        }
    }
    // by pipe element: its stretch carrying the most
    let mut by: Vec<(ElementId, Vec<usize>)> = vec![];
    for (i, p) in m.pipes.iter().enumerate() {
        if let Some(id) = p {
            match by.iter_mut().find(|(x, _)| x == id) {
                Some((_, v)) => v.push(i),
                None => by.push((*id, vec![i])),
            }
        }
    }
    for (id, parts) in by {
        let big = parts
            .iter()
            .copied()
            .max_by(|&a, &b| q[a].total_cmp(&q[b]))
            .unwrap_or(parts[0]);
        let node = &net.nodes[big];
        let ty = crate::pipe_types::by_id(&node.material);
        let label = |nom: f64| ty.map_or_else(|| format!("DN {nom}"), |t| t.size_label(nom, None));
        let size = label(node.nominal_mm);
        let (proposed, proposed_mm) = pick[big].map_or((size.clone(), node.nominal_mm), |z| {
            (label(z.nominal_mm), z.nominal_mm)
        });
        let (cap, fill, v) = res[big].unwrap_or_default();
        let stack = node.is_vertical();
        out.pipes.push(PipeResult {
            id,
            flow_ls: q[big],
            sum_du: du[big],
            fixtures: count[big],
            stack,
            size,
            proposed,
            proposed_mm,
            required_dn: need[big],
            slope: slope[big],
            no_slope: parts.iter().any(|&p| no_slope[p]),
            too_flat: parts.iter().any(|&p| too_flat[p]),
            fill,
            velocity_ms: v,
            capacity_ls: cap,
            slow: !stack && count[big] > 1 && q[big] > 0.0 && v < s.min_velocity,
            over: parts.iter().any(|&p| over[p]),
            noise_db: stack.then(|| stack_noise(&node.material, q[big])).flatten(),
        });
    }
    out.stacks = out.pipes.iter().filter(|p| p.stack).count();
    let root = 0;
    out.sum_du = du[root];
    out.fixtures = count[root];
    out.flow_ls = q[root];
    let outlet = if net.nodes.first().is_some_and(|r| r.kind == NodeKind::Pipe) {
        Some(root)
    } else {
        None
    };
    if let Some(o) = outlet {
        let di = pick[o].map_or(net.nodes[o].inner_diameter_m * 1000.0, |z| z.inner_mm);
        out.outlet_dn = dn_of_inner(di);
        if let Some((_, f, v)) = res[o] {
            out.outlet_fill = f;
            out.outlet_velocity = v;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wavin_stacks_are_as_quiet_as_wavin_measured_them() {
        assert_eq!(stack_noise("WAS", 2.0), Some((14.0, 10.0)));
        assert_eq!(stack_noise("WAS", 4.0), Some((19.0, 13.0)));
        assert_eq!(stack_noise("WSI", 3.0), Some((23.0, 14.5)));
        assert_eq!(stack_noise("WSI", 1.0), Some((21.0, 13.0)));
        assert_eq!(stack_noise("PPHT", 3.0), None);
    }
    use creabim_core::mep::Pipe;
    use creabim_core::{Document, Element, ElementKind, Transaction};
    use glam::{DVec2, DVec3};

    #[test]
    fn stacks_and_dns_follow_the_tables() {
        assert_eq!(dn_of_inner(96.8), 100.0);
        assert_eq!(dn_of_inner(44.0), 50.0);
        assert_eq!(stack_capacity(100.0, false), 4.0);
        assert_eq!(stack_capacity(100.0, true), 5.2);
        assert_eq!(stack_capacity(50.0, false), 0.0);
    }

    #[test]
    fn a_wc_and_a_basin_drain_through_a_dn_100_at_qww() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let mut ids = vec![];
        for (key, x) in [("starter/washbasin", 1.0), ("starter/wc", 3.0)] {
            let o = creabim_core::catalogue::starter(key).unwrap();
            let (tx, id) =
                creabim_core::objects::place(doc.model(), &o, level, DVec2::new(x, 3.0), 0.0);
            doc.commit(tx).unwrap();
            ids.push(id);
        }
        let conn = |id| {
            let e = doc.model().get(id).unwrap();
            creabim_core::fixture::connectors(doc.model(), e)
                .into_iter()
                .find(|c| c.kind == creabim_core::fixture::ConnKind::K)
                .unwrap()
                .at
        };
        let (b, w) = (conn(ids[0]), conn(ids[1]));
        let ty = crate::pipe_types::by_id("PPHT").unwrap();
        let z = ty.sizes[0].clone();
        let pipe = |a: DVec3, b: DVec3| {
            Element::new(
                "pipe",
                ElementKind::Pipe(Pipe {
                    level,
                    system: "K".into(),
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
        // a collector at 2 % from the basin past the WC to an outlet
        let y = 3.6;
        let z0 = -0.3;
        let at = |x: f64| DVec3::new(x, y, z0 - 0.02 * (x - b.x));
        let mut tx = Transaction::new("waste")
            .insert(pipe(b, DVec3::new(b.x, y, b.z)))
            .insert(pipe(DVec3::new(b.x, y, b.z), at(b.x)))
            .insert(pipe(at(b.x), at(w.x)))
            .insert(pipe(w, DVec3::new(w.x, y, w.z)))
            .insert(pipe(DVec3::new(w.x, y, w.z), at(w.x)));
        let outlet = pipe(at(w.x), at(w.x + 3.0));
        let out_id = outlet.id;
        tx = tx.insert(outlet);
        doc.commit(tx).unwrap();
        let r = run(
            doc.model(),
            "K",
            &Settings::default(),
            &DrainSettings::default(),
        )
        .expect("waste");
        assert_eq!(r.fixtures, 2, "{r:?}");
        assert!(r.unmatched.is_empty(), "{:?}", r.unmatched);
        // Qww = 0,5·√ΣDU = 0,79 l/s, no less than the WC's DU 2,0
        assert!((r.sum_du - 2.5).abs() < 1e-9, "{r:#?}");
        assert!((r.flow_ls - 2.0).abs() < 1e-9, "{r:#?}");
        let o = r.pipes.iter().find(|p| p.id == out_id).unwrap();
        assert!(o.proposed_mm >= 100.0, "{o:?}");
        assert!(!o.no_slope && !o.too_flat && o.fill <= 0.5 + 1e-6, "{o:?}");
        assert_eq!(r.outlet_dn, 100.0);
    }
}
