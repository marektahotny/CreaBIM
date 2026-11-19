//! The pipes of one system as a tree from where it leaves (or enters)
//! the building to the terminals on it: roof drains on storm water,
//! hydrants on a fire main. Pipes are split where another ends on them
//! and at the terminals standing along them.

use creabim_core::fixture::{self, ConnKind};
use creabim_core::mep::{Pipe, Valve};
use creabim_core::{ElementId, ElementKind, Model};
use glam::DVec3;

/// How near two ends must be to join, m.
const JOIN: f64 = 0.02;
/// How near a pipe's end must be to an object's connector, m.
const REACH: f64 = 0.03;

/// A terminal on the pipes: a valve of a terminal kind.
#[derive(Clone, Debug)]
pub struct Terminal {
    pub id: ElementId,
    pub name: String,
    pub valve: Valve,
    /// Where it is in the model, m.
    pub at: DVec3,
}

/// A stretch of a pipe, from its end nearer the root to the other.
#[derive(Clone, Debug)]
pub struct Stretch {
    /// The pipe element it is (part of).
    pub id: ElementId,
    pub pipe: Pipe,
    pub a: DVec3,
    pub b: DVec3,
    /// The stretch it hangs on: always before it in the tree.
    pub parent: Option<usize>,
    /// The terminals at its far end.
    pub terminals: Vec<usize>,
}

impl Stretch {
    pub fn length(&self) -> f64 {
        self.a.distance(self.b)
    }
    /// Its fall or rise, and its run in plan, m.
    pub fn rise_run(&self) -> (f64, f64) {
        (
            (self.b.z - self.a.z).abs(),
            (self.b - self.a).truncate().length(),
        )
    }
    /// Rises more than it runs: a stack or a riser.
    pub fn is_vertical(&self) -> bool {
        let (dz, dh) = self.rise_run();
        dz > dh
    }
}

#[derive(Clone, Debug, Default)]
pub struct Tree {
    /// Parents before children; the first starts at the root.
    pub stretches: Vec<Stretch>,
    pub terminals: Vec<Terminal>,
    pub root: DVec3,
    /// Pipes of the system the root does not reach.
    pub unreached: Vec<ElementId>,
}

impl Tree {
    /// The stretches hanging on each.
    pub fn children(&self) -> Vec<Vec<usize>> {
        let mut kids = vec![vec![]; self.stretches.len()];
        for (i, s) in self.stretches.iter().enumerate() {
            if let Some(p) = s.parent {
                kids[p].push(i);
            }
        }
        kids
    }

    /// The terminals at and beyond each stretch.
    pub fn terminals_below(&self) -> Vec<Vec<usize>> {
        let mut below: Vec<Vec<usize>> =
            self.stretches.iter().map(|s| s.terminals.clone()).collect();
        for i in (0..self.stretches.len()).rev() {
            if let Some(p) = self.stretches[i].parent {
                let mine = below[i].clone();
                below[p].extend(mine);
            }
        }
        below
    }

    /// The stretches of each pipe element, in tree order.
    pub fn by_pipe(&self) -> Vec<(ElementId, Vec<usize>)> {
        let mut out: Vec<(ElementId, Vec<usize>)> = vec![];
        for (i, s) in self.stretches.iter().enumerate() {
            match out.iter_mut().find(|(id, _)| *id == s.id) {
                Some((_, v)) => v.push(i),
                None => out.push((s.id, vec![i])),
            }
        }
        out
    }
}

/// The tree of system `code`: none when it has no pipes. Its root is
/// the lowest free end no terminal stands at.
pub fn build(model: &Model, code: &str) -> Option<Tree> {
    build_inner(model, code, None)
}

/// The tree of system `code` with the objects whose connectors of
/// `kind` a pipe ends at as terminals too (radiators on heating, a boiler
/// on gas), by their name; its root is where plant (an object that is no
/// fixture: a boiler) joins it when `plant`, else the lowest free end;
/// without `plant` every object is a terminal.
pub fn build_objects(model: &Model, code: &str, kind: ConnKind, plant: bool) -> Option<Tree> {
    build_inner(model, code, Some((kind, plant)))
}

/// The connectors of `kind` in the model: where, the object, its name,
/// and whether it is plant rather than a fixture.
fn object_connectors(model: &Model, kind: ConnKind) -> Vec<(DVec3, ElementId, String, bool)> {
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Object(o) = &e.kind else {
            continue;
        };
        let floor = model.level(o.level).map_or(0.0, |l| l.elevation);
        let family = fixture::family_of(model, e);
        for c in fixture::connectors(model, e) {
            if c.kind == kind {
                let name = if e.name.is_empty() {
                    family.unwrap_or("").to_string()
                } else {
                    e.name.clone()
                };
                out.push((c.at + DVec3::Z * floor, e.id, name, family.is_none()));
            }
        }
    }
    out
}

fn build_inner(model: &Model, code: &str, kind: Option<(ConnKind, bool)>) -> Option<Tree> {
    let pipes: Vec<(ElementId, Pipe, DVec3, DVec3)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Pipe(p) if p.system == code && p.length() > 1e-6 => {
                let z = DVec3::Z * model.level(p.level).map_or(0.0, |l| l.elevation);
                Some((e.id, p.clone(), p.start + z, p.end + z))
            }
            _ => None,
        })
        .collect();
    if pipes.is_empty() {
        return None;
    }
    let terminals: Vec<Terminal> = model
        .iter()
        .filter_map(|e| {
            let ElementKind::Valve(v) = &e.kind else {
                return None;
            };
            let (_, _, a, b) = pipes.iter().find(|p| p.0 == v.pipe)?;
            v.kind_def().filter(|k| k.terminal)?;
            Some(Terminal {
                id: e.id,
                name: e.name.clone(),
                valve: v.clone(),
                at: a.lerp(*b, v.t),
            })
        })
        .collect();
    let objects = kind.map_or_else(Vec::new, |k| object_connectors(model, k.0));
    let plant_roots = kind.is_some_and(|k| k.1);
    let mut plant: Vec<DVec3> = vec![];
    let mut terminals = terminals;
    for (at, id, name, is_plant) in objects {
        // only where a pipe ends
        let Some(pipe) = pipes
            .iter()
            .find(|p| p.2.distance(at) <= REACH || p.3.distance(at) <= REACH)
        else {
            continue;
        };
        let end = if pipe.2.distance(at) <= REACH {
            pipe.2
        } else {
            pipe.3
        };
        if is_plant && plant_roots {
            plant.push(end);
            continue;
        }
        let t = if end == pipe.2 { 0.0 } else { 1.0 };
        terminals.push(Terminal {
            id,
            name,
            valve: Valve::new(pipe.0, t, "teleso"),
            at: end,
        });
    }
    let mut cuts: Vec<DVec3> = pipes.iter().flat_map(|p| [p.2, p.3]).collect();
    cuts.extend(terminals.iter().map(|t| t.at));
    struct Seg {
        pipe: usize,
        a: DVec3,
        b: DVec3,
    }
    let mut segs: Vec<Seg> = vec![];
    for (k, (_, _, a, b)) in pipes.iter().enumerate() {
        let d = *b - *a;
        let len = d.length();
        let mut ts: Vec<f64> = cuts
            .iter()
            .filter_map(|q| {
                let t = (*q - *a).dot(d) / (len * len);
                (t * len > JOIN && (1.0 - t) * len > JOIN && (*a + d * t).distance(*q) <= JOIN)
                    .then_some(t)
            })
            .collect();
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|x, y| (*x - *y).abs() * len < JOIN);
        let mut at = *a;
        for t in ts.into_iter().chain([1.0]) {
            let to = a.lerp(*b, t);
            segs.push(Seg {
                pipe: k,
                a: at,
                b: to,
            });
            at = to;
        }
    }
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
    let mut at_point: Vec<Vec<usize>> = vec![vec![]; points.len()];
    for (k, t) in terminals.iter().enumerate() {
        if let Some(i) = points.iter().position(|q| q.distance(t.at) <= JOIN) {
            at_point[i].push(k);
        }
    }
    let ends = || (0..points.len()).filter(|&i| adj[i].len() == 1);
    let lowest = |v: Vec<usize>| {
        v.into_iter()
            .min_by(|&i, &j| points[i].z.total_cmp(&points[j].z))
    };
    let at_plant = |i: usize| plant.iter().any(|p| p.distance(points[i]) <= JOIN);
    let root = lowest(ends().filter(|&i| at_plant(i)).collect())
        .or_else(|| lowest(ends().filter(|&i| at_point[i].is_empty()).collect()))
        .or_else(|| lowest(ends().collect()))
        .unwrap_or(0);
    let mut out = Tree {
        terminals,
        root: points[root],
        ..Tree::default()
    };
    let mut used = vec![false; segs.len()];
    let mut seen = vec![false; points.len()];
    seen[root] = true;
    let mut stack: Vec<(usize, usize, Option<usize>)> =
        adj[root].iter().map(|&k| (root, k, None)).collect();
    while let Some((from, k, parent)) = stack.pop() {
        if used[k] {
            continue;
        }
        used[k] = true;
        let (i, j) = edges[k];
        let to = if i == from { j } else { i };
        if seen[to] {
            // a loop: the tree takes one way round
            continue;
        }
        seen[to] = true;
        let (a, b) = if i == from {
            (segs[k].a, segs[k].b)
        } else {
            (segs[k].b, segs[k].a)
        };
        let me = out.stretches.len();
        out.stretches.push(Stretch {
            id: pipes[segs[k].pipe].0,
            pipe: pipes[segs[k].pipe].1.clone(),
            a,
            b,
            parent,
            terminals: at_point[to].clone(),
        });
        for &n in &adj[to] {
            if !used[n] {
                stack.push((to, n, Some(me)));
            }
        }
    }
    out.unreached = pipes
        .iter()
        .map(|p| p.0)
        .filter(|id| !out.stretches.iter().any(|s| s.id == *id))
        .collect();
    Some(out)
}
