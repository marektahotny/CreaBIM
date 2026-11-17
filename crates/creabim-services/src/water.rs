//! Cold and hot water supply (STN EN 806-3, STN 73 6655, DIN 1988-300):
//! the design flow in every pipe, each pipe's size, the losses along the
//! way, and the pressure the network needs at its root against what the
//! connection gives. Ported from PilonHydraulika's `WaterAnalyzer`; the
//! proposals of valves along the network come in a later step.

use crate::hydraulics::{self, G};
use crate::network::{FittingKind, Network, NodeId, NodeKind, SizeNote};
use crate::peak_flow::{self, Sums};
use crate::settings::Settings;
use crate::sizing;
use crate::Fluid;

/// Which water a network carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemKind {
    ColdWater,
    HotWater,
}

/// What the calculation found, besides what it wrote on the nodes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WaterResult {
    pub fluid_temp_c: f64,
    pub root_flow_ls: f64,
    /// Pressure needed at the root with the sizes as they are, Pa.
    pub required_pa_current: f64,
    /// Pressure needed at the root with the proposed sizes, Pa.
    pub required_pa: f64,
    /// Pressure there is, Pa (behind a pressure reducer, its setting).
    pub available_pa: f64,
    /// Reserve kept on top of what is needed, Pa.
    pub reserve_pa: f64,
    /// The outlet hardest to reach.
    pub critical_leaf: Option<NodeId>,
    pub critical_path: Vec<NodeId>,
    /// Steps the critical branch was enlarged by.
    pub enlarged_steps: usize,
    /// The pressure reducer's recommended setting, kPa.
    pub reducer_setting_kpa: Option<f64>,
    /// The static pressure at the lowest outlet without a reducer, Pa.
    pub static_pa_without_reducer: f64,
    pub notes: Vec<Note>,
}

impl WaterResult {
    /// Pressure missing at the root, Pa; zero when there is enough.
    pub fn shortfall_pa(&self) -> f64 {
        (self.required_pa + self.reserve_pa - self.available_pa).max(0.0)
    }
}

/// Something the user should know about the result.
#[derive(Clone, Debug, PartialEq)]
pub enum Note {
    /// No fixtures are connected to the system.
    NoFixtures,
    /// Fixtures matched by none of the rules, by name.
    UnmatchedFixtures(Vec<String>),
    /// Equipment with no demand, counted with no flow.
    EquipmentWithoutDemand(Vec<NodeId>),
    /// The LU table has too few rows: DIN 1988-300 was used.
    LuTableMissing,
    /// The meter's loss was added at the root (it isn't in the model).
    MeterLossAdded { kpa: f64 },
    /// The static pressure exceeds the limit (STN EN 806-2): a pressure
    /// reducer is proposed, or needed.
    StaticPressureHigh {
        pa: f64,
        limit_pa: f64,
        reducer_proposed: bool,
    },
    /// The pressure falls short by this much, Pa; `stuck` are critical
    /// pipes with no larger size in their series.
    PressureShort { pa: f64, stuck: Vec<NodeId> },
    /// No size meets the limits for these pipes.
    NoSizeFits(Vec<NodeId>),
}

/// Sum each node's fixtures up the tree.
fn accumulate_fixtures(net: &mut Network, use_connector_size: bool) {
    for i in net.post_order() {
        let n = &net.nodes[i];
        if n.children.is_empty() {
            if let Some(f) = n.fixture.clone() {
                let n = &mut net.nodes[i];
                n.sum_lu = f.lu;
                n.max_lu = f.lu;
                n.sum_qa = f.qa_ls();
                n.max_qa = f.qa_ls();
                n.sum_qa_stn_sq = f.qa_stn * f.qa_stn;
                n.sum_qa_phi = f.qa_stn * f.phi;
                n.max_qa_stn = f.qa_stn;
                n.min_nominal_mm = if use_connector_size {
                    f.connector_nominal_mm
                } else {
                    0.0
                };
                n.fixture_count = 1;
            }
            continue;
        }
        let kids: Vec<_> = n
            .children
            .iter()
            .map(|c| net.nodes[*c].clone_sums())
            .collect();
        let n = &mut net.nodes[i];
        let max = |f: fn(&Sum) -> f64| kids.iter().map(f).fold(f64::MIN, f64::max);
        n.sum_lu = kids.iter().map(|k| k.lu).sum();
        n.max_lu = max(|k| k.max_lu);
        n.sum_qa = kids.iter().map(|k| k.qa).sum();
        n.max_qa = max(|k| k.max_qa);
        n.sum_qa_stn_sq = kids.iter().map(|k| k.qa_stn_sq).sum();
        n.sum_qa_phi = kids.iter().map(|k| k.qa_phi).sum();
        n.max_qa_stn = max(|k| k.max_qa_stn);
        n.min_nominal_mm = max(|k| k.min_nominal);
        n.fixture_count = kids.iter().map(|k| k.count).sum();
    }
}

/// A child's sums, copied out for its parent.
struct Sum {
    lu: f64,
    max_lu: f64,
    qa: f64,
    max_qa: f64,
    qa_stn_sq: f64,
    qa_phi: f64,
    max_qa_stn: f64,
    min_nominal: f64,
    count: usize,
}

impl crate::network::Node {
    fn clone_sums(&self) -> Sum {
        Sum {
            lu: self.sum_lu,
            max_lu: self.max_lu,
            qa: self.sum_qa,
            max_qa: self.max_qa,
            qa_stn_sq: self.sum_qa_stn_sq,
            qa_phi: self.sum_qa_phi,
            max_qa_stn: self.max_qa_stn,
            min_nominal: self.min_nominal_mm,
            count: self.fixture_count,
        }
    }
}

fn zeta_for_fitting(s: &Settings, net: &Network, fitting: NodeId, child: NodeId) -> f64 {
    let f = &net.nodes[fitting];
    if let Some(z) = f.zeta_override {
        return z;
    }
    let z = &s.zeta;
    match f.fitting {
        FittingKind::Elbow => {
            if f.elbow_angle_deg < 60.0 {
                z.elbow45
            } else {
                z.elbow90
            }
        }
        FittingKind::Tee => {
            if net.nodes[child].is_branch {
                z.tee_branch
            } else {
                z.tee_through
            }
        }
        FittingKind::Cross => z.cross,
        FittingKind::Transition => z.reducer,
        FittingKind::Coupling => z.coupling,
        FittingKind::Cap => 0.0,
        FittingKind::None | FittingKind::Other => z.other,
    }
}

/// Velocities and losses on every node, with the sizes as they are or as
/// proposed.
pub(crate) fn compute_hydraulics(s: &Settings, net: &mut Network, fluid: &Fluid, proposed: bool) {
    use crate::network::AccessoryRole as R;
    let order = net.pre_order();
    for &i in &order {
        let n = &mut net.nodes[i];
        n.velocity_ms = 0.0;
        n.r_pa_per_m = 0.0;
        n.linear_loss_pa = 0.0;
        n.local_loss_pa = 0.0;
        n.zeta = 0.0;
        if n.is_pipe() {
            let d = n.active_inner_m(proposed);
            n.velocity_ms = hydraulics::velocity(n.flow_ls, d);
            n.r_pa_per_m = hydraulics::r(n.flow_ls, d, n.roughness_m, fluid);
            n.linear_loss_pa = n.r_pa_per_m * n.length_m;
        }
    }
    for &i in &order {
        if net.nodes[i].kind == NodeKind::Accessory {
            let up = net
                .nearest_upstream_pipe(i)
                .or_else(|| net.nearest_downstream_pipe(i));
            let v = up.map_or(0.0, |u| {
                hydraulics::velocity(net.nodes[i].flow_ls, net.nodes[u].active_inner_m(proposed))
            });
            let role = net.nodes[i].role;
            let fam = s.family_for(role);
            let z = net.nodes[i].zeta_override.unwrap_or(fam.zeta);
            let n = &mut net.nodes[i];
            n.velocity_ms = v;
            n.zeta += z;
            n.local_loss_pa += z * hydraulics::dynamic(v, fluid.rho);
            n.fixed_loss_pa = if n.flow_ls > 0.0 && !matches!(role, R::Other | R::None) {
                fam.fixed_loss_kpa * 1000.0
            } else {
                0.0
            };
        }
        if let Some(p) = net.nodes[i].parent {
            if net.nodes[p].kind == NodeKind::Fitting && net.nodes[i].flow_ls > 0.0 {
                let v_up = net
                    .nearest_upstream_pipe(p)
                    .map_or(0.0, |u| net.nodes[u].velocity_ms);
                let v_down = if net.nodes[i].is_pipe() {
                    net.nodes[i].velocity_ms
                } else {
                    net.nearest_downstream_pipe(i)
                        .map_or(0.0, |d| net.nodes[d].velocity_ms)
                };
                let z = zeta_for_fitting(s, net, p, i);
                let n = &mut net.nodes[i];
                n.zeta += z;
                n.local_loss_pa += z * hydraulics::dynamic(v_up.max(v_down), fluid.rho);
            }
        }
    }
}

/// All losses from the root down to `n`, Pa.
pub(crate) fn path_loss_pa(net: &Network, n: NodeId) -> f64 {
    let mut s = 0.0;
    let mut x = Some(n);
    while let Some(i) = x {
        let k = &net.nodes[i];
        s += k.linear_loss_pa + k.local_loss_pa + k.fixed_loss_pa;
        x = k.parent;
    }
    s
}

fn leaf_pmin(net: &Network, l: NodeId) -> f64 {
    let n = &net.nodes[l];
    if n.is_linked_equipment {
        n.leaf_required_pa
    } else {
        n.fixture.as_ref().map_or(0.0, |f| f.min_pressure_kpa) * 1000.0
    }
}

/// The pressure the root needs for its hardest outlet, and that outlet.
fn required(net: &Network, leaves: &[NodeId], fluid: &Fluid) -> (f64, Option<NodeId>) {
    let root_z = net.root().p1[2];
    let mut best = 0.0;
    let mut bl = None;
    for &l in leaves {
        let p = path_loss_pa(net, l)
            + fluid.rho * G * (net.nodes[l].p1[2] - root_z)
            + leaf_pmin(net, l);
        if bl.is_none() || p > best {
            best = p;
            bl = Some(l);
        }
    }
    (best, bl)
}

/// Size the network and find the pressure it needs. `root_is_connection`:
/// the root is the water main's connection (the meter's loss and the
/// mains pressure apply); `available_kpa` overrides the pressure there.
pub fn run(
    s: &Settings,
    net: &mut Network,
    kind: SystemKind,
    root_is_connection: bool,
    available_kpa: Option<f64>,
) -> WaterResult {
    let w = &s.water;
    let hot = kind == SystemKind::HotWater;
    let fluid = Fluid::water(if hot { w.hot_c } else { w.cold_c });
    let mut r = WaterResult {
        fluid_temp_c: fluid.temp_c,
        ..Default::default()
    };
    if net.nodes.is_empty() {
        return r;
    }
    accumulate_fixtures(net, w.by_connector);
    let all = net.pre_order();
    let unmatched: Vec<String> = all
        .iter()
        .filter_map(|&i| {
            net.nodes[i]
                .fixture
                .as_ref()
                .filter(|f| !f.matched)
                .map(|_| net.nodes[i].name.clone())
        })
        .collect();
    if !unmatched.is_empty() {
        r.notes.push(Note::UnmatchedFixtures(unmatched));
    }
    let idle: Vec<NodeId> = all
        .iter()
        .copied()
        .filter(|&i| {
            let n = &net.nodes[i];
            n.is_leaf() && n.kind == NodeKind::Equipment && !n.is_linked_equipment
        })
        .collect();
    if !idle.is_empty() {
        r.notes.push(Note::EquipmentWithoutDemand(idle));
    }
    if w.method == peak_flow::PeakFlowMethod::TabulkaLU && w.lu_table.len() < 2 {
        r.notes.push(Note::LuTableMissing);
    }
    for &i in &all {
        let n = &net.nodes[i];
        let sums = Sums {
            sum_qa: n.sum_qa,
            max_qa: n.max_qa,
            sum_qa_stn_sq: n.sum_qa_stn_sq,
            sum_lu: n.sum_lu,
            sum_qa_phi: n.sum_qa_phi,
            max_qa_stn: n.max_qa_stn,
        };
        net.nodes[i].flow_ls = peak_flow::compute(w.method, w.building, sums, &w.lu_table);
    }
    r.root_flow_ls = net.root().flow_ls;
    if net.root().fixture_count == 0 && !all.iter().any(|&i| net.nodes[i].is_linked_equipment) {
        r.notes.push(Note::NoFixtures);
    }

    // sizes
    for &p in &all {
        if !net.nodes[p].is_pipe() {
            continue;
        }
        let n = &net.nodes[p];
        let conn = n.fixture_count <= 1
            && !n.is_linked_equipment
            && !n.children.iter().any(|&c| net.nodes[c].is_linked_equipment);
        let vmax = if conn {
            w.v_max_connection
        } else {
            w.v_max_distribution
        };
        let size =
            sizing::choose_size(net, p, &fluid, vmax, w.r_max, w.min_inner_mm, w.only_upsize);
        net.nodes[p].proposed = size;
    }
    if w.only_upsize {
        sizing::unify_kept_oversized(net);
    }
    if w.monotonic {
        sizing::enforce_monotonic(net);
    }
    sizing::uniform_runs(net);

    // the meter, when it isn't in the model
    let root = Network::ROOT;
    net.nodes[root].fixed_loss_pa = 0.0;
    let has_role = |net: &Network, role| {
        all.iter()
            .any(|&i| net.nodes[i].kind == NodeKind::Accessory && net.nodes[i].role == role)
    };
    use crate::network::AccessoryRole as R;
    let meter_in_model = has_role(net, R::WaterMeter);
    if !hot && root_is_connection && w.meter_outside_model && !meter_in_model {
        net.nodes[root].fixed_loss_pa += w.meter_loss_kpa * 1000.0;
        r.notes.push(Note::MeterLossAdded {
            kpa: w.meter_loss_kpa,
        });
    }
    let mut avail = available_kpa.unwrap_or(if hot {
        w.hot_alone_kpa
    } else {
        w.mains_min_kpa
    }) * 1000.0;
    let reserve = w.reserve_kpa * 1000.0;
    r.reserve_pa = reserve;
    let leaves: Vec<NodeId> = all
        .iter()
        .copied()
        .filter(|&i| {
            let n = &net.nodes[i];
            n.is_leaf() && (n.fixture.is_some() || n.is_linked_equipment)
        })
        .collect();

    // the static pressure at the lowest outlet
    let reducer_in_model = has_role(net, R::PressureReducer);
    let mut need_reducer = false;
    let mut propose_reducer = false;
    if !hot && root_is_connection && !leaves.is_empty() {
        let z_min = leaves
            .iter()
            .map(|&l| net.nodes[l].p1[2])
            .fold(f64::MAX, f64::min);
        r.static_pa_without_reducer =
            w.mains_max_kpa * 1000.0 + fluid.rho * G * (net.root().p1[2] - z_min);
        if r.static_pa_without_reducer > w.max_static_kpa * 1000.0 {
            need_reducer = true;
            propose_reducer = s.components.propose_reducer && !reducer_in_model;
        }
    }

    // the pressure needed, and the critical branch enlarged until it is there
    compute_hydraulics(s, net, &fluid, false);
    r.required_pa_current = required(net, &leaves, &fluid).0;
    compute_hydraulics(s, net, &fluid, true);
    let (mut req, mut leaf) = required(net, &leaves, &fluid);
    if w.enlarge_critical && leaf.is_some() && req + reserve > avail {
        let mut iter: usize = 0;
        while req + reserve > avail && iter < 300 {
            iter += 1;
            let Some(l) = leaf else { break };
            let path: Vec<NodeId> = net
                .path_from_root(l)
                .into_iter()
                .filter(|&i| net.nodes[i].is_pipe() && sizing::next_larger(net, i).is_some())
                .collect();
            if path.is_empty() {
                break;
            }
            let max_r = path
                .iter()
                .map(|&i| net.nodes[i].r_pa_per_m)
                .fold(f64::MIN, f64::max);
            for &p in &path {
                if net.nodes[p].r_pa_per_m >= 0.8 * max_r {
                    let next = sizing::next_larger(net, p);
                    let n = &mut net.nodes[p];
                    n.proposed = next;
                    if !n.size_notes.contains(&SizeNote::CriticalBranch) {
                        n.size_notes.push(SizeNote::CriticalBranch);
                    }
                }
            }
            if w.monotonic {
                sizing::enforce_monotonic(net);
            }
            sizing::uniform_runs(net);
            compute_hydraulics(s, net, &fluid, true);
            (req, leaf) = required(net, &leaves, &fluid);
        }
        r.enlarged_steps = iter;
    }
    r.required_pa = req;

    // a pressure reducer's setting
    if propose_reducer || reducer_in_model {
        let set = ((req + reserve) / 10000.0).ceil() * 10.0;
        let set = set.min(w.max_static_kpa);
        avail = avail.min(set * 1000.0);
        r.reducer_setting_kpa = Some(set);
    }
    if need_reducer && !reducer_in_model {
        r.notes.push(Note::StaticPressureHigh {
            pa: r.static_pa_without_reducer,
            limit_pa: w.max_static_kpa * 1000.0,
            reducer_proposed: propose_reducer,
        });
    }
    r.available_pa = avail;

    if let Some(l) = leaf {
        for n in net.nodes.iter_mut() {
            n.on_critical_path = false;
        }
        r.critical_path = net.path_from_root(l);
        for &i in &r.critical_path {
            net.nodes[i].on_critical_path = true;
        }
        r.critical_leaf = Some(l);
        if req + reserve > avail {
            let stuck: Vec<NodeId> = r
                .critical_path
                .iter()
                .copied()
                .filter(|&i| net.nodes[i].is_pipe() && sizing::next_larger(net, i).is_none())
                .collect();
            r.notes.push(Note::PressureShort {
                pa: req + reserve - avail,
                stuck,
            });
        }
    }
    let none_fits: Vec<NodeId> = all
        .iter()
        .copied()
        .filter(|&i| net.nodes[i].is_pipe() && net.nodes[i].proposed.is_some())
        .filter(|&i| net.nodes[i].size_notes.contains(&SizeNote::NoneFits))
        .collect();
    if !none_fits.is_empty() {
        r.notes.push(Note::NoSizeFits(none_fits));
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::match_fixture;
    use crate::network::{Node, PipeSize};

    fn copper() -> Vec<PipeSize> {
        [
            (12.0, 10.0),
            (15.0, 13.0),
            (18.0, 16.0),
            (22.0, 20.0),
            (28.0, 25.0),
            (35.0, 32.0),
        ]
        .iter()
        .map(|&(nominal_mm, inner_mm)| PipeSize {
            nominal_mm,
            inner_mm,
        })
        .collect()
    }

    /// A flat: 10 m in, a tee, a basin on 3 m and a bath on 5 m.
    fn flat(s: &Settings) -> Network {
        let cu = copper();
        let s15 = cu[1];
        let mut net = Network::default();
        let pipe = |id: &str, a: [f64; 3], b: [f64; 3]| {
            let mut n = Node::pipe(id, a, b, s15, cu.clone());
            n.roughness_m = 1.5e-6;
            n.material = "Meď".into();
            n
        };
        let root = net.add(None, pipe("1", [0.0, 0.0, 0.0], [10.0, 0.0, 0.0]));
        let tee = net.add(
            Some(root),
            Node {
                id: "2".into(),
                kind: NodeKind::Fitting,
                fitting: FittingKind::Tee,
                p1: [10.0, 0.0, 0.0],
                p2: [10.0, 0.0, 0.0],
                ..Default::default()
            },
        );
        let a = net.add(Some(tee), pipe("3", [10.0, 0.0, 0.0], [13.0, 0.0, 0.0]));
        let mut b = pipe("5", [10.0, 0.0, 0.0], [10.0, 5.0, 1.0]);
        b.is_branch = true;
        let b = net.add(Some(tee), b);
        let fixture = |id: &str, name: &str, at: [f64; 3]| Node {
            id: id.into(),
            name: name.into(),
            kind: NodeKind::Fixture,
            p1: at,
            p2: at,
            fixture: Some(match_fixture(&s.fixtures, &s.unknown_fixture, name, "")),
            ..Default::default()
        };
        net.add(Some(a), fixture("4", "Umývadlo", [13.0, 0.0, 0.0]));
        net.add(Some(b), fixture("6", "Vaňa", [10.0, 5.0, 1.0]));
        net
    }

    #[test]
    fn a_flat_is_sized_and_has_pressure_to_spare() {
        let s = Settings::default();
        let mut net = flat(&s);
        let r = run(&s, &mut net, SystemKind::ColdWater, true, None);
        // ΣqA 0.1 + 0.4 = 0.5 l/s: DIN gives 1.48·0.5^0.19 − 0.94, never
        // less than the largest fixture's 0.4 l/s
        let q = (1.48 * 0.5f64.powf(0.19) - 0.94).max(0.4);
        assert!((r.root_flow_ls - q).abs() < 1e-12);
        // the bath's branch is the critical one: higher and farther
        assert_eq!(net.nodes[r.critical_leaf.unwrap()].id, "6");
        assert!(r.shortfall_pa() == 0.0, "{r:?}");
        assert!(r.notes.contains(&Note::MeterLossAdded { kpa: 20.0 }));
        // every pipe has a size within 1.5 m/s (2 m/s to one fixture)
        for n in net.nodes.iter().filter(|n| n.is_pipe()) {
            let p = n.proposed.unwrap();
            let v = hydraulics::velocity(n.flow_ls, p.inner_mm / 1000.0);
            assert!(v <= 2.0 + 1e-9, "{} {v}", n.id);
        }
    }

    #[test]
    fn low_mains_pressure_enlarges_the_critical_branch() {
        let s = Settings::default();
        let mut net = flat(&s);
        let r0 = run(&s, &mut net, SystemKind::ColdWater, true, None);
        let mut net = flat(&s);
        // just above what the unenlarged network needs
        let tight = (r0.required_pa + r0.reserve_pa) / 1000.0 - 5.0;
        let r = run(&s, &mut net, SystemKind::ColdWater, true, Some(tight));
        assert!(r.enlarged_steps >= 1 || r.shortfall_pa() > 0.0, "{r:?}");
        assert!(r.required_pa < r0.required_pa);
    }
}
