//! A network as a tree: its nodes in one list, each knowing its parent and
//! its children in order, from the root (the connection, a heater) out to
//! the fixtures. Besides what the model gives (where a pipe runs, its size,
//! the fixture at an end) a node keeps what the calculation finds on it.

use crate::fixtures::FixtureData;
use serde::{Deserialize, Serialize};

pub type NodeId = usize;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeKind {
    #[default]
    Pipe,
    Fitting,
    Accessory,
    Fixture,
    Equipment,
    Terminal,
    OpenEnd,
    Other,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FittingKind {
    #[default]
    None,
    Elbow,
    Tee,
    Cross,
    Transition,
    Coupling,
    Cap,
    Other,
}

/// What an accessory is for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessoryRole {
    #[default]
    None,
    ShutOff,
    CheckValve,
    Strainer,
    PressureReducer,
    Balancing,
    WaterMeter,
    Other,
    ValveBox,
    GasValve,
    GasCabinet,
    GasRegulator,
    GasThermal,
    /// An air vent or a drain cock on a tee the water runs straight
    /// through: the tee's ζ straight through.
    TeeTap,
}

/// A size a pipe can take: nominal (DN, or the outside diameter of
/// plastic pipe) and inner diameter, mm.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PipeSize {
    pub nominal_mm: f64,
    pub inner_mm: f64,
}

impl PipeSize {
    /// The size as a drawing labels it: "15", "16.2".
    pub fn label(&self) -> String {
        label(self.nominal_mm)
    }
}

/// A nominal size to one decimal, as PilonHydraulika compares sizes.
fn label(nominal_mm: f64) -> String {
    let r = (nominal_mm * 10.0).round() / 10.0;
    if r.fract() == 0.0 {
        format!("{r:.0}")
    } else {
        format!("{r:.1}")
    }
}

/// Why a pipe got its size, beyond the plain fit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SizeNote {
    /// Its size is kept: it fits, though a smaller one would.
    KeptOversized,
    /// No size in its series meets the limits; the largest is proposed.
    NoneFits,
    /// Made no smaller than the pipe after it.
    Monotonic,
    /// One size along a run between branches.
    UniformRun,
    /// Enlarged for want of pressure on the critical branch.
    CriticalBranch,
    /// Its size was chosen by hand.
    Manual,
    /// Without flow: kept as it is.
    NoFlow,
}

#[derive(Clone, Debug, Default)]
pub struct Node {
    /// The model's id for it.
    pub id: String,
    pub name: String,
    pub kind: NodeKind,
    pub fitting: FittingKind,
    pub role: AccessoryRole,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    /// Where it starts and ends, m (a fitting or a fixture: one point).
    pub p1: [f64; 3],
    pub p2: [f64; 3],
    pub length_m: f64,
    pub inner_diameter_m: f64,
    pub nominal_mm: f64,
    pub roughness_m: f64,
    pub material: String,
    /// The sizes its series offers.
    pub sizes: Vec<PipeSize>,
    /// A ζ given in the model, over the settings'.
    pub zeta_override: Option<f64>,
    pub elbow_angle_deg: f64,
    /// It leaves its parent tee by the branch, not straight through.
    pub is_branch: bool,
    pub fixture: Option<FixtureData>,
    /// Equipment with its own demand, linked in (not a fixture).
    pub is_linked_equipment: bool,
    pub leaf_required_pa: f64,
    pub setpoint_kpa: Option<f64>,
    pub manual_size: Option<PipeSize>,

    // what the calculation finds
    pub sum_lu: f64,
    pub max_lu: f64,
    pub sum_qa: f64,
    pub max_qa: f64,
    pub sum_qa_stn_sq: f64,
    pub sum_qa_phi: f64,
    pub max_qa_stn: f64,
    pub min_nominal_mm: f64,
    pub fixture_count: usize,
    pub flow_ls: f64,
    pub velocity_ms: f64,
    pub r_pa_per_m: f64,
    pub linear_loss_pa: f64,
    pub local_loss_pa: f64,
    pub fixed_loss_pa: f64,
    pub zeta: f64,
    pub vmax_used: f64,
    pub proposed: Option<PipeSize>,
    pub auto_size: Option<PipeSize>,
    pub min_ok_size: Option<PipeSize>,
    pub kept_oversized: bool,
    pub size_notes: Vec<SizeNote>,
    pub on_critical_path: bool,
}

impl Node {
    pub fn pipe(
        id: &str,
        p1: [f64; 3],
        p2: [f64; 3],
        size: PipeSize,
        sizes: Vec<PipeSize>,
    ) -> Node {
        Node {
            id: id.into(),
            kind: NodeKind::Pipe,
            p1,
            p2,
            length_m: dist(p1, p2),
            nominal_mm: size.nominal_mm,
            inner_diameter_m: size.inner_mm / 1000.0,
            roughness_m: 7e-6,
            sizes,
            elbow_angle_deg: 90.0,
            ..Default::default()
        }
    }

    pub fn is_pipe(&self) -> bool {
        self.kind == NodeKind::Pipe
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// A pipe that rises or falls more than it runs.
    pub fn is_vertical(&self) -> bool {
        if !self.is_pipe() {
            return false;
        }
        let d = sub(self.p2, self.p1);
        let h = (d[0] * d[0] + d[1] * d[1]).sqrt();
        d[2].abs() > h && d[2].abs() > 0.05
    }

    /// Its size in the model.
    pub fn current_size(&self) -> PipeSize {
        PipeSize {
            nominal_mm: self.nominal_mm,
            inner_mm: self.inner_diameter_m * 1000.0,
        }
    }

    pub fn is_size_changed(&self) -> bool {
        self.proposed
            .is_some_and(|p| p.label() != self.current_size().label())
    }

    /// The inner diameter, m, as it is or as proposed.
    pub fn active_inner_m(&self, proposed: bool) -> f64 {
        match self.proposed {
            Some(p) if proposed => p.inner_mm / 1000.0,
            _ => self.inner_diameter_m,
        }
    }

    pub fn active_nominal_mm(&self, proposed: bool) -> f64 {
        match self.proposed {
            Some(p) if proposed => p.nominal_mm,
            _ => self.nominal_mm,
        }
    }
}

pub(crate) fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub(crate) fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    let d = sub(a, b);
    (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
}

/// A network: its nodes, the first the root.
#[derive(Clone, Debug, Default)]
pub struct Network {
    pub nodes: Vec<Node>,
}

impl Network {
    pub const ROOT: NodeId = 0;

    /// Add `node` under `parent` (none for the root, which must come
    /// first), after its other children.
    pub fn add(&mut self, parent: Option<NodeId>, mut node: Node) -> NodeId {
        let i = self.nodes.len();
        assert!(
            parent.is_some() || i == 0,
            "only the first node is the root"
        );
        node.parent = parent;
        if let Some(p) = parent {
            self.nodes[p].children.push(i);
        }
        self.nodes.push(node);
        i
    }

    pub fn root(&self) -> &Node {
        &self.nodes[Self::ROOT]
    }

    /// Every node, parents before children, children in order.
    pub fn pre_order(&self) -> Vec<NodeId> {
        let mut out = Vec::with_capacity(self.nodes.len());
        if self.nodes.is_empty() {
            return out;
        }
        let mut st = vec![Self::ROOT];
        while let Some(n) = st.pop() {
            out.push(n);
            st.extend(self.nodes[n].children.iter().rev());
        }
        out
    }

    /// Every node, children before their parents.
    pub fn post_order(&self) -> Vec<NodeId> {
        let mut v = self.pre_order();
        v.reverse();
        v
    }

    /// From the root down to `n`.
    pub fn path_from_root(&self, n: NodeId) -> Vec<NodeId> {
        let mut p = vec![];
        let mut x = Some(n);
        while let Some(i) = x {
            p.push(i);
            x = self.nodes[i].parent;
        }
        p.reverse();
        p
    }

    pub fn nearest_upstream_pipe(&self, n: NodeId) -> Option<NodeId> {
        let mut x = self.nodes[n].parent;
        while let Some(i) = x {
            if self.nodes[i].is_pipe() {
                return Some(i);
            }
            x = self.nodes[i].parent;
        }
        None
    }

    /// Down the child with the most flow to the first pipe.
    pub fn nearest_downstream_pipe(&self, n: NodeId) -> Option<NodeId> {
        let mut x = n;
        while !self.nodes[x].children.is_empty() {
            // the first of the largest, as a stable sort would give
            let c = &self.nodes[x].children;
            let mut best = c[0];
            for &k in &c[1..] {
                if self.nodes[k].flow_ls > self.nodes[best].flow_ls {
                    best = k;
                }
            }
            x = best;
            if self.nodes[x].is_pipe() {
                return Some(x);
            }
        }
        None
    }

    /// The node with model id `id`.
    pub fn find(&self, id: &str) -> Option<NodeId> {
        self.nodes.iter().position(|n| n.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_compare_by_their_labels() {
        let a = PipeSize {
            nominal_mm: 15.0,
            inner_mm: 13.0,
        };
        assert_eq!(a.label(), "15");
        assert_eq!(
            PipeSize {
                nominal_mm: 16.2,
                inner_mm: 12.0
            }
            .label(),
            "16.2"
        );
    }

    #[test]
    fn orders_walk_the_tree() {
        let mut n = Network::default();
        let s = PipeSize::default();
        let r = n.add(None, Node::pipe("r", [0.0; 3], [1.0, 0.0, 0.0], s, vec![]));
        let a = n.add(
            Some(r),
            Node::pipe("a", [1.0, 0.0, 0.0], [2.0, 0.0, 0.0], s, vec![]),
        );
        let b = n.add(
            Some(r),
            Node::pipe("b", [1.0, 0.0, 0.0], [1.0, 0.0, 3.0], s, vec![]),
        );
        let c = n.add(
            Some(a),
            Node::pipe("c", [2.0, 0.0, 0.0], [3.0, 0.0, 0.0], s, vec![]),
        );
        assert_eq!(n.pre_order(), vec![r, a, c, b]);
        assert_eq!(n.post_order(), vec![b, c, a, r]);
        assert_eq!(n.path_from_root(c), vec![r, a, c]);
        assert!(n.nodes[b].is_vertical() && !n.nodes[a].is_vertical());
    }
}
