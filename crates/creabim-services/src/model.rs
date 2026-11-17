//! A network from the model: the pipes of one system drawn in it, joined
//! where their ends meet (or one ends on another, a tee), and the
//! fixtures whose connectors they reach, as the tree the calculations
//! take. Its root is the free end no fixture stands at: the lowest, where
//! a supply comes up into the building.

use crate::fixtures::match_fixture;
use crate::network::{FittingKind, Network, Node, NodeId, NodeKind, PipeSize};
use crate::pipe_types;
use crate::settings::Settings;
use creabim_core::fixture::{self, ConnKind};
use creabim_core::mep::Pipe;
use creabim_core::{ElementId, ElementKind, Model};
use glam::DVec3;

/// How near two ends must be to join, m.
const JOIN: f64 = 0.02;
/// How near a pipe's end must be to a fixture's connector, m.
const REACH: f64 = 0.03;

/// A network built from the model, and what of the model each node is.
#[derive(Clone, Debug, Default)]
pub struct ModelNetwork {
    pub net: Network,
    /// The pipe element a pipe node is (part of), by node.
    pub pipes: Vec<Option<ElementId>>,
    /// The fixture a fixture node is, by node.
    pub fixtures: Vec<Option<ElementId>>,
    /// Pipes of the system not reached from the root.
    pub unreached: Vec<ElementId>,
    /// Free ends with no fixture, besides the root.
    pub open_ends: Vec<DVec3>,
    /// Where pipes close a loop: the calculation takes a tree, so one
    /// pipe of each loop is left out.
    pub loops: usize,
}

struct Seg {
    id: ElementId,
    a: DVec3,
    b: DVec3,
    pipe: Pipe,
}

/// The ends of `p` in the model, m.
fn ends(model: &Model, p: &Pipe) -> (DVec3, DVec3) {
    let z = DVec3::Z * model.level(p.level).map_or(0.0, |l| l.elevation);
    (p.start + z, p.end + z)
}

/// Where `p` lies inside `a`–`b` (not at its ends), as the fraction along.
fn inside(a: DVec3, b: DVec3, p: DVec3) -> Option<f64> {
    let d = b - a;
    let l2 = d.length_squared();
    if l2 < 1e-12 {
        return None;
    }
    let t = (p - a).dot(d) / l2;
    let len = l2.sqrt();
    if t * len <= JOIN || (1.0 - t) * len <= JOIN {
        return None;
    }
    ((a + d * t).distance(p) <= JOIN).then_some(t)
}

/// The network of system `code` (SV, TV...) in `model`: none when it has
/// no pipes.
pub fn build(model: &Model, code: &str, s: &Settings) -> Option<ModelNetwork> {
    let pipes: Vec<(ElementId, Pipe)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Pipe(p) if p.system == code && p.length() > 1e-6 => {
                Some((e.id, p.clone()))
            }
            _ => None,
        })
        .collect();
    if pipes.is_empty() {
        return None;
    }
    let ends_all: Vec<DVec3> = pipes
        .iter()
        .flat_map(|(_, p)| {
            let (a, b) = ends(model, p);
            [a, b]
        })
        .collect();
    // pipes split where another ends on them
    let mut segs: Vec<Seg> = vec![];
    for (id, p) in &pipes {
        let (a, b) = ends(model, p);
        let mut ts: Vec<f64> = ends_all.iter().filter_map(|&q| inside(a, b, q)).collect();
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|x, y| (*x - *y).abs() * a.distance(b) < JOIN);
        let mut at = a;
        for t in ts.into_iter().chain([1.0]) {
            let to = a.lerp(b, t);
            segs.push(Seg {
                id: *id,
                a: at,
                b: to,
                pipe: p.clone(),
            });
            at = to;
        }
    }
    // the points they meet at
    let mut points: Vec<DVec3> = vec![];
    let mut point = |p: DVec3| -> usize {
        if let Some(i) = points.iter().position(|q| q.distance(p) <= JOIN) {
            return i;
        }
        points.push(p);
        points.len() - 1
    };
    let edges: Vec<(usize, usize)> = segs.iter().map(|s| (point(s.a), point(s.b))).collect();
    let mut adj: Vec<Vec<usize>> = vec![vec![]; points.len()];
    for (k, &(i, j)) in edges.iter().enumerate() {
        adj[i].push(k);
        adj[j].push(k);
    }
    // fixtures at their connectors of this system
    let mut at_point: Vec<Option<(ElementId, &'static str, String, f64)>> =
        vec![None; points.len()];
    for e in model.iter() {
        let Some(family) = fixture::family_of(model, e) else {
            continue;
        };
        let ElementKind::Object(o) = &e.kind else {
            continue;
        };
        let floor = model.level(o.level).map_or(0.0, |l| l.elevation);
        for c in fixture::connectors(model, e) {
            if c.kind.code() != code || !matches!(c.kind, ConnKind::SV | ConnKind::TV) {
                continue;
            }
            let at = c.at + DVec3::Z * floor;
            if let Some(i) = (0..points.len())
                .filter(|&i| adj[i].len() == 1 && points[i].distance(at) <= REACH)
                .min_by(|&i, &j| points[i].distance(at).total_cmp(&points[j].distance(at)))
            {
                at_point[i] = Some((e.id, family, e.name.clone(), c.dn));
            }
        }
    }
    // the root: the lowest free end with no fixture, else the lowest end
    let free: Vec<usize> = (0..points.len())
        .filter(|&i| adj[i].len() == 1 && at_point[i].is_none())
        .collect();
    let lowest = |v: &[usize]| {
        v.iter()
            .copied()
            .min_by(|&i, &j| points[i].z.total_cmp(&points[j].z))
    };
    let all: Vec<usize> = (0..points.len()).filter(|&i| adj[i].len() == 1).collect();
    let root = lowest(&free).or_else(|| lowest(&all)).unwrap_or(0);

    let mut out = ModelNetwork::default();
    let mut used = vec![false; segs.len()];
    let mut seen = vec![false; points.len()];
    seen[root] = true;
    // depth first: (point reached, the segment it came by, the node to hang on)
    let &first = adj[root].first()?;
    let mut stack: Vec<(usize, usize, Option<NodeId>, bool)> = vec![(root, first, None, false)];
    while let Some((from, k, parent, branch)) = stack.pop() {
        if used[k] {
            continue;
        }
        used[k] = true;
        let (i, j) = edges[k];
        let to = if i == from { j } else { i };
        let sg = &segs[k];
        let (a, b) = if i == from {
            (sg.a, sg.b)
        } else {
            (sg.b, sg.a)
        };
        if seen[to] {
            out.loops += 1;
            continue;
        }
        seen[to] = true;
        let mut node = pipe_node(&sg.pipe, a, b, out.net.nodes.len());
        node.is_branch = branch;
        let mut here = add(&mut out, parent, node, Some(sg.id), None);
        let dir = (b - a).normalize_or_zero();
        let next: Vec<usize> = adj[to].iter().copied().filter(|&n| !used[n]).collect();
        let fx = at_point[to].clone();
        let outs = next.len() + usize::from(fx.is_some());
        let p = [b.x, b.y, b.z];
        if outs == 0 {
            if to != root {
                out.open_ends.push(b);
                let n = point_node(NodeKind::OpenEnd, p, out.net.nodes.len());
                add(&mut out, Some(here), n, None, None);
            }
            continue;
        }
        // the run goes on along the straightest way; the others branch
        let along = |n: usize| {
            let (x, y) = edges[n];
            let o = if x == to { y } else { x };
            (points[o] - points[to]).normalize_or_zero().dot(dir)
        };
        let mut next = next;
        next.sort_by(|&x, &y| along(y).total_cmp(&along(x)));
        if outs >= 2 {
            let mut t = point_node(NodeKind::Fitting, p, out.net.nodes.len());
            t.fitting = if outs == 2 {
                FittingKind::Tee
            } else {
                FittingKind::Cross
            };
            here = add(&mut out, Some(here), t, None, None);
        } else if let Some(&n) = next.first() {
            let turn = along(n).clamp(-1.0, 1.0).acos().to_degrees();
            if turn > 1.0 {
                let mut el = point_node(NodeKind::Fitting, p, out.net.nodes.len());
                el.fitting = FittingKind::Elbow;
                el.elbow_angle_deg = turn.min(180.0);
                here = add(&mut out, Some(here), el, None, None);
            }
        }
        if let Some((id, family, name, dn)) = fx {
            let mut n = point_node(NodeKind::Fixture, p, out.net.nodes.len());
            n.name = if name.is_empty() {
                family.to_string()
            } else {
                name.clone()
            };
            let mut data = match_fixture(&s.fixtures, &s.unknown_fixture, family, &name);
            data.connector_nominal_mm = dn;
            n.fixture = Some(data);
            add(&mut out, Some(here), n, None, Some(id));
        }
        // pushed in reverse, so the run is taken first
        for (m, &n) in next.iter().enumerate().rev() {
            stack.push((to, n, Some(here), m > 0));
        }
    }
    let mut reached: Vec<ElementId> = out.pipes.iter().flatten().copied().collect();
    reached.sort();
    reached.dedup();
    out.unreached = pipes
        .iter()
        .map(|(id, _)| *id)
        .filter(|id| reached.binary_search(id).is_err())
        .collect();
    Some(out)
}

fn add(
    out: &mut ModelNetwork,
    parent: Option<NodeId>,
    node: Node,
    pipe: Option<ElementId>,
    fixture: Option<ElementId>,
) -> NodeId {
    let i = out.net.add(parent, node);
    out.pipes.push(pipe);
    out.fixtures.push(fixture);
    i
}

fn point_node(kind: NodeKind, p: [f64; 3], n: usize) -> Node {
    Node {
        id: (n + 1).to_string(),
        kind,
        p1: p,
        p2: p,
        elbow_angle_deg: 90.0,
        ..Default::default()
    }
}

fn pipe_node(p: &Pipe, a: DVec3, b: DVec3, n: usize) -> Node {
    let t = pipe_types::by_id(&p.pipe_type);
    let size = PipeSize {
        nominal_mm: p.nominal_mm,
        inner_mm: p.inner_mm,
    };
    let sizes = t.map(|t| t.calc_sizes()).unwrap_or_default();
    let mut node = Node::pipe(
        &(n + 1).to_string(),
        [a.x, a.y, a.z],
        [b.x, b.y, b.z],
        size,
        sizes,
    );
    if let Some(t) = t {
        node.roughness_m = t.roughness_m();
        node.material = t.id.clone();
    }
    node
}

/// What the calculation found for one pipe of the model: the largest flow
/// along it, and the size proposed there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PipeResult {
    pub flow_ls: f64,
    pub velocity_ms: f64,
    pub r_pa_per_m: f64,
    pub proposed: Option<PipeSize>,
    pub on_critical_path: bool,
}

/// The calculation's findings, by pipe element.
pub fn pipe_results(m: &ModelNetwork) -> Vec<(ElementId, PipeResult)> {
    let mut out: Vec<(ElementId, PipeResult)> = vec![];
    for (i, n) in m.net.nodes.iter().enumerate() {
        let Some(id) = m.pipes[i] else { continue };
        let r = PipeResult {
            flow_ls: n.flow_ls,
            velocity_ms: n.velocity_ms,
            r_pa_per_m: n.r_pa_per_m,
            proposed: n.proposed,
            on_critical_path: n.on_critical_path,
        };
        match out.iter_mut().find(|(x, _)| *x == id) {
            Some((_, old)) => {
                if r.flow_ls > old.flow_ls {
                    *old = PipeResult {
                        on_critical_path: old.on_critical_path || r.on_critical_path,
                        ..r
                    };
                } else {
                    old.on_critical_path |= r.on_critical_path;
                }
            }
            None => out.push((id, r)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::{Document, Element, Transaction};
    use glam::DVec2;

    fn pipe(level: ElementId, code: &str, a: [f64; 3], b: [f64; 3]) -> Element {
        let t = pipe_types::by_id("HERZ").unwrap();
        let s = t.size(16.0).unwrap();
        Element::new(
            "pipe",
            ElementKind::Pipe(Pipe {
                level,
                system: code.into(),
                start: DVec3::from(a),
                end: DVec3::from(b),
                pipe_type: t.id.clone(),
                nominal_mm: s.nominal_mm,
                outer_mm: s.outer_mm,
                inner_mm: s.inner_mm,
                height_mm: None,
                insulation_mm: 0.0,
            }),
        )
    }

    /// Two basins on a cold water main coming up through the floor, the
    /// first on a tee part way along it.
    fn two_basins() -> (Document, ElementId) {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let basin = creabim_core::catalogue::starter("starter/washbasin").unwrap();
        for x in [1.0, 4.0] {
            let (tx, _) =
                creabim_core::objects::place(doc.model(), &basin, level, DVec2::new(x, 3.0), 0.0);
            doc.commit(tx).unwrap();
        }
        let mut tx = Transaction::new("pipes");
        for (a, b) in [
            ([0.0, 2.5, -0.5], [0.0, 2.5, 0.3]),
            ([0.0, 2.5, 0.3], [4.08, 2.5, 0.3]),
            ([1.08, 2.5, 0.3], [1.08, 2.98, 0.3]),
            ([1.08, 2.98, 0.3], [1.08, 2.98, 0.55]),
            ([4.08, 2.5, 0.3], [4.08, 2.98, 0.3]),
            ([4.08, 2.98, 0.3], [4.08, 2.98, 0.55]),
        ] {
            tx = tx.insert(pipe(level, "SV", a, b));
        }
        // hot water elsewhere is another network
        tx = tx.insert(pipe(level, "TV", [9.0, 9.0, 0.3], [9.0, 8.0, 0.3]));
        doc.commit(tx).unwrap();
        (doc, level)
    }

    #[test]
    fn pipes_and_basins_make_a_tree_from_where_the_main_comes_in() {
        let (doc, _) = two_basins();
        let s = Settings::default();
        let mut m = build(doc.model(), "SV", &s).unwrap();
        let net = &m.net;
        assert_eq!(net.root().p1, [0.0, 2.5, -0.5], "the lowest free end");
        let kinds = |k: NodeKind| net.nodes.iter().filter(|n| n.kind == k).count();
        // the main is split at the tee: 7 pipes from 6
        assert_eq!(kinds(NodeKind::Pipe), 7);
        assert_eq!(kinds(NodeKind::Fixture), 2);
        assert_eq!(kinds(NodeKind::OpenEnd), 0);
        let tees = net
            .nodes
            .iter()
            .filter(|n| n.fitting == FittingKind::Tee)
            .count();
        assert_eq!(tees, 1);
        assert!(net.nodes.iter().any(|n| n.fitting == FittingKind::Elbow));
        let fx = net.nodes.iter().find_map(|n| n.fixture.as_ref()).unwrap();
        assert!(fx.matched, "{}", fx.rule);
        assert_eq!(fx.connector_nominal_mm, 15.0);
        assert!(m.unreached.is_empty() && m.open_ends.is_empty() && m.loops == 0);

        let r = crate::water(&s, &mut m.net, crate::SystemKind::ColdWater, true, None);
        assert!(r.root_flow_ls > 0.0 && r.required_pa > 0.0, "{r:?}");
        let res = pipe_results(&m);
        assert_eq!(res.len(), 6, "a result for every pipe drawn");
        assert!(res
            .iter()
            .all(|(_, p)| p.flow_ls > 0.0 && p.proposed.is_some()));
    }

    #[test]
    fn a_system_with_no_pipes_has_no_network() {
        let (doc, _) = two_basins();
        assert!(build(doc.model(), "K", &Settings::default()).is_none());
        let tv = build(doc.model(), "TV", &Settings::default()).unwrap();
        // a lone pipe: one end is its root, the other open
        assert_eq!(tv.open_ends.len(), 1);
    }
}
