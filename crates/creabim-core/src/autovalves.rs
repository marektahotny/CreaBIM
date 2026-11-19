//! The valves and accessories a pipe system needs, proposed from the
//! pipes drawn, as PilonDesigner's `autovalves.ts` (and the Pilon
//! Hydraulika plugin before it).
//!
//! Water (STN EN 806-2, STN 73 6660):
//! - the meter assembly on the cold water connection: shut-off, water
//!   meter, check valve, strainer and a pressure reducer; a shut-off
//!   where the hot water sets out;
//! - a shut-off and a check valve on the cold water into a water heater
//!   (STN 06 0830);
//! - a shut-off at the foot of every riser and on a branch that feeds
//!   more than one fixture;
//! - an angle valve at a fixture that joins through one (basin, WC,
//!   sink, bidet, washing machine, dishwasher, urinal).
//!
//! Waste (STN EN 12056-2, STN 75 6760):
//! - a vent head on the free top of every stack;
//! - a cleanout at the foot of every stack, 1.0 m above the floor of
//!   the lowest storey it passes;
//! - on a drain laid flat (DN 100 and more) a cleanout where it turns
//!   by more than 45° and every 20 m at most.
//!
//! What is already on the pipes is not added a second time.

use crate::fixture::{self, ConnKind};
use crate::mep::{Pipe, SystemKind, Valve};
use crate::{ElementId, ElementKind, Model};
use glam::DVec3;

/// How near two ends must be to join, m.
const JOIN: f64 = 0.02;
/// How near a pipe's end must be to a connector, m.
const REACH: f64 = 0.03;

/// What to propose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FillOpts {
    /// The meter assembly, a shut-off at the start of hot water and the
    /// valves into a water heater.
    pub assembly: bool,
    /// A pressure reducer in the assembly.
    pub reducer: bool,
    /// Shut-offs on risers and branches.
    pub branches: bool,
    /// Angle valves at fixtures.
    pub angle: bool,
    /// Vent heads on stacks.
    pub vents: bool,
    /// Cleanouts.
    pub cleanouts: bool,
}

impl Default for FillOpts {
    fn default() -> Self {
        Self {
            assembly: true,
            reducer: true,
            branches: true,
            angle: true,
            vents: true,
            cleanouts: true,
        }
    }
}

/// A valve proposed.
#[derive(Clone, Debug, PartialEq)]
pub struct Proposed {
    pub valve: Valve,
    /// The size a vent head or a cleanout is made in, DN.
    pub dn: Option<u32>,
}

/// What [`fill`] proposes, and how many of each group.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fill {
    pub valves: Vec<Proposed>,
    /// The kinds added at the start of the system, in their order.
    pub assembly: Vec<&'static str>,
    /// Water heaters whose cold inlet got its valves.
    pub heaters: usize,
    pub shut_offs: usize,
    pub angle_valves: usize,
    pub vents: usize,
    pub cleanouts: usize,
    /// The system takes no proposal, or has no pipes.
    pub unsupported: bool,
}

/// A fixture of this family joins through an angle valve: a shower and
/// a bath have their taps on the wall.
pub fn uses_angle_valve(family: &str) -> bool {
    let f = family.to_lowercase();
    [
        "umýv", "wc", "záchod", "drez", "bidet", "prač", "práč", "riad", "pisoár",
    ]
    .iter()
    .any(|k| f.contains(k))
}

/// The size a vent head or a cleanout is made in for a pipe of this
/// outside diameter, mm.
pub fn vent_dn(outer_mm: f64) -> u32 {
    if outer_mm <= 90.0 {
        75
    } else if outer_mm <= 125.0 {
        100
    } else {
        125
    }
}

/// A stretch of a pipe between two points where pipes meet.
struct Seg {
    pipe: usize,
    /// The pipe's `t` at its ends.
    t: [f64; 2],
    /// Its ends, m, and the points they are.
    p: [DVec3; 2],
    at: [usize; 2],
}

impl Seg {
    fn len(&self) -> f64 {
        self.p[0].distance(self.p[1])
    }
    /// The end that is not point `i`.
    fn other(&self, i: usize) -> usize {
        if self.at[0] == i {
            self.at[1]
        } else {
            self.at[0]
        }
    }
    /// The pipe's `t` at `d` metres from its end at point `from`.
    fn t_at(&self, from: usize, d: f64) -> f64 {
        let l = self.len().max(1e-9);
        let f = (d.clamp(0.0, l)) / l;
        let (a, b) = if self.at[0] == from {
            (self.t[0], self.t[1])
        } else {
            (self.t[1], self.t[0])
        };
        r4(a + (b - a) * f)
    }
    fn run(&self) -> f64 {
        (self.p[1] - self.p[0]).truncate().length()
    }
    fn rise(&self) -> f64 {
        (self.p[1].z - self.p[0].z).abs()
    }
}

fn r4(v: f64) -> f64 {
    (v * 10000.0).round() / 10000.0
}

/// The pipes of a system as stretches joined at points.
struct Graph {
    ids: Vec<ElementId>,
    pipes: Vec<Pipe>,
    segs: Vec<Seg>,
    points: Vec<DVec3>,
    adj: Vec<Vec<usize>>,
    /// The valves on the pipes, those proposed with them: pipe, t, kind.
    valves: Vec<(usize, f64, String)>,
    added: Vec<Proposed>,
}

impl Graph {
    fn new(model: &Model, code: &str) -> Option<Self> {
        let (ids, pipes): (Vec<ElementId>, Vec<Pipe>) = model
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Pipe(p) if p.system == code && p.length() > 0.005 => {
                    Some((e.id, p.clone()))
                }
                _ => None,
            })
            .unzip();
        if pipes.is_empty() {
            return None;
        }
        let ends: Vec<(DVec3, DVec3)> = pipes
            .iter()
            .map(|p| {
                let z = DVec3::Z * model.level(p.level).map_or(0.0, |l| l.elevation);
                (p.start + z, p.end + z)
            })
            .collect();
        let all: Vec<DVec3> = ends.iter().flat_map(|(a, b)| [*a, *b]).collect();
        let mut points: Vec<DVec3> = vec![];
        let mut point = |p: DVec3| -> usize {
            if let Some(i) = points.iter().position(|q| q.distance(p) <= JOIN) {
                return i;
            }
            points.push(p);
            points.len() - 1
        };
        let mut segs = vec![];
        for (k, (a, b)) in ends.iter().enumerate() {
            let d = *b - *a;
            let len = d.length();
            // split where another pipe ends on it
            let mut ts: Vec<f64> = all
                .iter()
                .filter_map(|q| {
                    let t = (*q - *a).dot(d) / (len * len);
                    (t * len > JOIN && (1.0 - t) * len > JOIN && (*a + d * t).distance(*q) <= JOIN)
                        .then_some(t)
                })
                .collect();
            ts.sort_by(f64::total_cmp);
            ts.dedup_by(|x, y| (*x - *y).abs() * len < JOIN);
            let mut t0 = 0.0;
            for t1 in ts.into_iter().chain([1.0]) {
                let p = [a.lerp(*b, t0), a.lerp(*b, t1)];
                segs.push(Seg {
                    pipe: k,
                    t: [t0, t1],
                    p,
                    at: [point(p[0]), point(p[1])],
                });
                t0 = t1;
            }
        }
        let mut adj = vec![vec![]; points.len()];
        for (k, s) in segs.iter().enumerate() {
            adj[s.at[0]].push(k);
            adj[s.at[1]].push(k);
        }
        let valves = model
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Valve(v) => {
                    let k = ids.iter().position(|id| *id == v.pipe)?;
                    Some((k, v.t, v.kind.clone()))
                }
                _ => None,
            })
            .collect();
        Some(Self {
            ids,
            pipes,
            segs,
            points,
            adj,
            valves,
            added: vec![],
        })
    }

    fn deg(&self, i: usize) -> usize {
        self.adj[i].len()
    }

    /// The point a connector at `p` joins: a free end within reach.
    fn end_at(&self, p: DVec3) -> Option<usize> {
        (0..self.points.len())
            .filter(|&i| self.deg(i) == 1 && self.points[i].distance(p) <= REACH)
            .min_by(|&i, &j| {
                self.points[i]
                    .distance(p)
                    .total_cmp(&self.points[j].distance(p))
            })
    }

    fn on_pipe(&self, pipe: usize, kind: &str) -> bool {
        self.valves.iter().any(|v| v.0 == pipe && v.2 == kind)
    }

    /// A valve of `kind` on this stretch of its pipe.
    fn on_seg(&self, s: usize, kind: &str) -> bool {
        let sg = &self.segs[s];
        self.valves.iter().any(|v| {
            v.0 == sg.pipe && v.2 == kind && v.1 >= sg.t[0] - 1e-9 && v.1 <= sg.t[1] + 1e-9
        })
    }

    fn has(&self, kind: &str) -> bool {
        self.valves.iter().any(|v| v.2 == kind)
    }

    fn put(&mut self, pipe: usize, t: f64, kind: &str, dn: Option<u32>) {
        let t = r4(t.clamp(0.0, 1.0));
        self.valves.push((pipe, t, kind.to_string()));
        self.added.push(Proposed {
            valve: Valve::new(self.ids[pipe], t, kind),
            dn,
        });
    }

    /// A valve `d` metres along stretch `s` from its end at point `from`.
    fn put_on(&mut self, s: usize, from: usize, d: f64, kind: &str) {
        let (pipe, t) = (self.segs[s].pipe, self.segs[s].t_at(from, d));
        self.put(pipe, t, kind, None);
    }
}

/// A stretch in the tree from the root.
struct N {
    seg: usize,
    from: usize,
    to: usize,
    /// It leaves the run it comes off.
    branch: bool,
    /// Fixtures it feeds.
    fixtures: usize,
}

/// The connectors of code `code` in the model: where, the fixture's
/// family (none for plant: a water heater, a boiler), DN.
fn connectors_of(model: &Model, code: &str) -> Vec<(DVec3, Option<&'static str>)> {
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Object(o) = &e.kind else {
            continue;
        };
        let floor = model.level(o.level).map_or(0.0, |l| l.elevation);
        let family = fixture::family_of(model, e);
        for c in fixture::connectors(model, e) {
            if c.kind.code() == code {
                out.push((c.at + DVec3::Z * floor, family));
            }
        }
    }
    out
}

/// Where the plant of the model takes or gives system `code`: the
/// connectors of objects that are no fixture (a water heater's cold
/// inlet and hot outlet, a boiler's), m in the model.
pub fn plant_connectors(model: &Model, kind: ConnKind) -> Vec<DVec3> {
    connectors_of(model, kind.code())
        .into_iter()
        .filter(|(_, f)| f.is_none())
        .map(|(p, _)| p)
        .collect()
}

/// The valves system `code` of the model still needs.
pub fn fill(model: &Model, code: &str, opts: &FillOpts) -> Fill {
    let kind = crate::mep::system(code).map(|s| s.kind);
    let unsupported = Fill {
        unsupported: true,
        ..Fill::default()
    };
    let Some(g) = Graph::new(model, code) else {
        return unsupported;
    };
    match kind {
        Some(SystemKind::ColdWater) => fill_water(model, g, code, true, opts),
        Some(SystemKind::HotWater) => fill_water(model, g, code, false, opts),
        Some(SystemKind::Sanitary) => fill_sanitary(model, g, code, opts),
        Some(k @ (SystemKind::HeatingSupply | SystemKind::HeatingReturn | SystemKind::Gas)) => {
            fill_ends(model, g, k)
        }
        _ => unsupported,
    }
}

/// Heating and gas: the valves at the ends of the pipes, where they
/// reach what they feed.
///
/// Heating (STN EN 12828): a thermostatic valve on the supply into a
/// radiator and a lockshield on its return; a shut-off on both pipes at
/// a manifold and at the heat source, and a strainer on the return into
/// the source.
///
/// Gas (STN EN 1775, TPP 704 01): the main shut-off where the pipe
/// comes in (its lowest free end) and a gas cock before every appliance.
fn fill_ends(model: &Model, mut g: Graph, kind: SystemKind) -> Fill {
    let mut out = Fill::default();
    let conn = match kind {
        SystemKind::HeatingSupply => ConnKind::UKP,
        SystemKind::HeatingReturn => ConnKind::UKS,
        _ => ConnKind::P,
    };
    let conns = connectors_of(model, conn.code());
    let mut joined = vec![false; g.points.len()];
    for (p, family) in &conns {
        let Some(i) = g.end_at(*p) else { continue };
        joined[i] = true;
        let radiator = family.is_some_and(|f| fixture::is_heating(f) && !fixture::is_manifold(f));
        let want: &[&str] = match (kind, radiator, family.is_none()) {
            (SystemKind::HeatingSupply, true, _) => &["termostaticky"],
            (SystemKind::HeatingReturn, true, _) => &["spiatocka"],
            (SystemKind::HeatingReturn, _, true) => &["uzaver", "filter"],
            (SystemKind::Gas, ..) => &["kohut"],
            _ => &["uzaver"],
        };
        let s = g.adj[i][0];
        let l = g.segs[s].len();
        for (n, k) in want.iter().enumerate() {
            if g.on_seg(s, k) {
                continue;
            }
            g.put_on(s, i, (0.1 + 0.15 * n as f64).min(l / 2.0), k);
            if radiator {
                out.angle_valves += 1;
            } else {
                out.shut_offs += 1;
            }
        }
    }
    if kind == SystemKind::Gas {
        // where the gas comes in: the lowest free end nothing stands at
        let root = (0..g.points.len())
            .filter(|&i| g.deg(i) == 1 && !joined[i])
            .min_by(|&i, &j| g.points[i].z.total_cmp(&g.points[j].z));
        if let Some(i) = root {
            if !g.has("hup") {
                let s = g.adj[i][0];
                let l = g.segs[s].len();
                g.put_on(s, i, 0.1_f64.min(l / 2.0), "hup");
                out.assembly.push("hup");
            }
        }
    }
    out.valves = g.added;
    out
}

fn fill_water(model: &Model, mut g: Graph, code: &str, cold: bool, opts: &FillOpts) -> Fill {
    let mut out = Fill::default();
    let conns = connectors_of(model, code);
    // fixtures at the ends, and the ends plant stands at
    let mut fix_at: Vec<Vec<&'static str>> = vec![vec![]; g.points.len()];
    let mut plant: Vec<usize> = vec![];
    for (p, family) in &conns {
        let Some(i) = g.end_at(*p) else { continue };
        match family {
            Some(f) => fix_at[i].push(f),
            None => plant.push(i),
        }
    }
    // the root as the calculation takes it: hot water sets out of its
    // heater; else the lowest free end no fixture stands at
    let lowest = |v: Vec<usize>| {
        v.into_iter()
            .min_by(|&i, &j| g.points[i].z.total_cmp(&g.points[j].z))
    };
    let ends = || (0..g.points.len()).filter(|&i| g.deg(i) == 1);
    let free = lowest(
        ends()
            .filter(|&i| fix_at[i].is_empty() && !plant.contains(&i))
            .collect(),
    );
    let root = if cold { None } else { plant.first().copied() }
        .or(free)
        .or_else(|| lowest(ends().collect()))
        .unwrap_or(0);
    let Some(&first) = g.adj[root].first() else {
        return out;
    };
    // the tree from the root
    let mut tree: Vec<N> = vec![];
    let mut parent: Vec<Option<usize>> = vec![];
    let mut used = vec![false; g.segs.len()];
    let mut stack = vec![(first, root, false, None::<usize>)];
    while let Some((s, from, branch, par)) = stack.pop() {
        if used[s] {
            continue;
        }
        used[s] = true;
        let to = g.segs[s].other(from);
        let me = tree.len();
        tree.push(N {
            seg: s,
            from,
            to,
            branch,
            fixtures: fix_at[to].len(),
        });
        parent.push(par);
        let next: Vec<usize> = g.adj[to].iter().copied().filter(|&n| !used[n]).collect();
        let dir = (g.points[to] - g.points[from]).normalize_or_zero();
        let along = |n: usize| {
            let o = g.segs[n].other(to);
            (g.points[o] - g.points[to]).normalize_or_zero().dot(dir)
        };
        // the run goes on along the straightest way; the others branch
        let straight = next
            .iter()
            .copied()
            .max_by(|&x, &y| along(x).total_cmp(&along(y)));
        for n in next.iter().copied() {
            stack.push((n, to, next.len() > 1 && Some(n) != straight, Some(me)));
        }
    }
    for i in (1..tree.len()).rev() {
        if let Some(p) = parent[i] {
            tree[p].fixtures += tree[i].fixtures;
        }
    }

    // 1) the meter assembly (cold) or a shut-off where hot water sets out
    if opts.assembly || opts.reducer {
        let mut want: Vec<&'static str> = vec![];
        if cold {
            if opts.assembly {
                want.extend(["uzaver", "vodomer", "spatna", "filter"]);
            }
            if opts.reducer {
                want.push("redukcny");
            }
        } else if opts.assembly {
            want.push("uzaver");
        }
        let pipe = g.segs[first].pipe;
        let missing: Vec<&'static str> = want
            .into_iter()
            .filter(|k| {
                if *k == "uzaver" {
                    !g.on_pipe(pipe, "uzaver")
                } else {
                    !g.has(k)
                }
            })
            .collect();
        if !missing.is_empty() {
            let sg = &g.segs[first];
            let l = sg.len();
            // how far the valves already on it reach from the root
            let t_root = if sg.at[0] == root { sg.t[0] } else { sg.t[1] };
            let whole = g.pipes[pipe].length();
            let last = g
                .valves
                .iter()
                .filter(|v| v.0 == pipe)
                .map(|v| (v.1 - t_root).abs() * whole)
                .fold(0.0, f64::max);
            let step = ((l - last - 0.05) / (missing.len() as f64 + 1.0)).clamp(0.04, 0.25);
            let start = if last > 0.0 { last } else { 0.1 - step };
            for (i, k) in missing.iter().enumerate() {
                g.put_on(first, root, start + step * (i as f64 + 1.0), k);
            }
            out.assembly = missing;
        }
    }

    // 1b) cold water into a water heater: a shut-off and a check valve
    if opts.assembly && cold {
        for &w in &plant {
            let Some(&end) = g.adj[w].first() else {
                continue;
            };
            // on the first flat stretch before the inlet: the drop to it
            // is short
            let flat = |s: usize| g.segs[s].run() > 0.2;
            let (mut s, mut from) = (end, w);
            if !flat(s) {
                let o = g.segs[s].other(w);
                if let Some(&nx) = g.adj[o].iter().find(|&&x| x != s && flat(x)) {
                    (s, from) = (nx, o);
                }
            }
            let l = g.segs[s].len();
            let miss: Vec<&str> = ["spatna", "uzaver"]
                .into_iter()
                .filter(|k| !g.on_seg(s, k))
                .collect();
            for (i, k) in miss.iter().enumerate() {
                let i = i as f64;
                let alone = if *k == "uzaver" && miss.len() == 1 {
                    0.15
                } else {
                    0.0
                };
                let d = (l * (0.3 + 0.25 * i)).min(0.15 + 0.15 * i + alone);
                g.put_on(s, from, d, k);
            }
            out.heaters += usize::from(!miss.is_empty());
        }
    }

    // 2) shut-offs on risers and on branches to more than one fixture
    if opts.branches {
        for x in tree.iter().skip(1) {
            let sg = &g.segs[x.seg];
            let riser = sg.rise() > 1.0
                && sg.run() < 0.05
                && x.fixtures >= 1
                && (x.branch || g.deg(x.from) > 2);
            let group = x.branch && x.fixtures >= 2;
            if !riser && !group {
                continue;
            }
            if g.on_seg(x.seg, "uzaver") || g.on_seg(x.seg, "rohovy") {
                continue;
            }
            let d = (sg.len() / 3.0).min(0.15);
            g.put_on(x.seg, x.from, d, "uzaver");
            out.shut_offs += 1;
        }
    }

    // 3) angle valves at the fixtures, 8 cm off them
    if opts.angle {
        for x in &tree {
            if !fix_at[x.to].iter().any(|f| uses_angle_valve(f)) {
                continue;
            }
            if g.on_seg(x.seg, "rohovy") {
                continue;
            }
            let d = (g.segs[x.seg].len() / 2.0).min(0.08);
            g.put_on(x.seg, x.to, d, "rohovy");
            out.angle_valves += 1;
        }
    }
    out.valves = g.added;
    out
}

fn fill_sanitary(model: &Model, mut g: Graph, code: &str, opts: &FillOpts) -> Fill {
    let mut out = Fill::default();
    let vertical = |s: &Seg| s.run() < 0.05 && s.rise() > 0.3;
    let outer = |g: &Graph, s: usize| g.pipes[g.segs[s].pipe].outer_mm;
    // ends a fixture stands at are no free tops of a stack
    let mut fix_end = vec![false; g.points.len()];
    for (c, _) in connectors_of(model, code) {
        for (i, p) in g.points.iter().enumerate() {
            if g.deg(i) == 1
                && (p.truncate() - c.truncate()).length() < 0.1
                && (p.z - c.z).abs() < 1.5
            {
                fix_end[i] = true;
            }
        }
    }
    // stacks: vertical stretches joined one over another
    let mut seen = vec![false; g.segs.len()];
    let mut chains: Vec<Vec<usize>> = vec![];
    for s0 in 0..g.segs.len() {
        if seen[s0] || !vertical(&g.segs[s0]) {
            continue;
        }
        let mut chain = vec![];
        let mut stack = vec![s0];
        while let Some(s) = stack.pop() {
            if seen[s] {
                continue;
            }
            seen[s] = true;
            chain.push(s);
            for i in g.segs[s].at {
                for &y in &g.adj[i] {
                    if !seen[y] && vertical(&g.segs[y]) {
                        stack.push(y);
                    }
                }
            }
        }
        chains.push(chain);
    }
    for chain in &chains {
        // its ends: (stretch, which end)
        let ends: Vec<(usize, usize)> = chain.iter().flat_map(|&s| [(s, 0), (s, 1)]).collect();
        let z = |e: &(usize, usize)| g.segs[e.0].p[e.1].z;
        let top = *ends.iter().max_by(|a, b| z(a).total_cmp(&z(b))).unwrap();
        let bottom = *ends.iter().min_by(|a, b| z(a).total_cmp(&z(b))).unwrap();
        let (zt, zb) = (z(&top), z(&bottom));
        // a fixture's short drop is no stack
        if zt - zb < 1.5 {
            continue;
        }
        let top_pt = g.segs[top.0].at[top.1];
        let top_pipe = g.segs[top.0].pipe;
        if opts.vents && g.deg(top_pt) == 1 && !fix_end[top_pt] && !g.on_pipe(top_pipe, "hlavica") {
            let dn = vent_dn(outer(&g, top.0));
            let t = g.segs[top.0].t[top.1];
            g.put(top_pipe, t, "hlavica", Some(dn));
            out.vents += 1;
        }
        if opts.cleanouts && !chain.iter().any(|&s| g.on_seg(s, "cistiaci")) {
            // the foot's storey: the highest whose floor is at most 1 m
            // over the foot (the foot is often under the floor)
            let floor = model
                .levels()
                .map(|(_, l)| l.elevation)
                .filter(|e| *e <= zb + 1.0)
                .fold(f64::MIN, f64::max);
            let zc = if floor > f64::MIN { floor } else { zb } + 1.0;
            let host = chain
                .iter()
                .copied()
                .find(|&s| {
                    let (a, b) = (g.segs[s].p[0].z, g.segs[s].p[1].z);
                    a.min(b) <= zc && a.max(b) >= zc
                })
                .unwrap_or(bottom.0);
            let sg = &g.segs[host];
            let dz = sg.p[1].z - sg.p[0].z;
            let f = ((zc - sg.p[0].z) / if dz == 0.0 { 1.0 } else { dz }).clamp(0.05, 0.95);
            let (pipe, t) = (sg.pipe, sg.t[0] + (sg.t[1] - sg.t[0]) * f);
            let dn = vent_dn(outer(&g, host));
            g.put(pipe, t, "cistiaci", Some(dn));
            out.cleanouts += 1;
        }
    }
    if opts.cleanouts {
        // drains laid flat, DN 100 and more
        let flat: Vec<bool> = (0..g.segs.len())
            .map(|s| !vertical(&g.segs[s]) && outer(&g, s) >= 100.0)
            .collect();
        // a turn by more than 45°
        for i in 0..g.points.len() {
            let here = g.adj[i].clone();
            if here.len() != 2 || !flat[here[0]] || !flat[here[1]] {
                continue;
            }
            let away = |s: usize| {
                (g.points[g.segs[s].other(i)] - g.points[i])
                    .truncate()
                    .normalize_or_zero()
            };
            let turn = 180.0
                - away(here[0])
                    .dot(away(here[1]))
                    .clamp(-1.0, 1.0)
                    .acos()
                    .to_degrees();
            if turn <= 45.0 {
                continue;
            }
            if here.iter().any(|&s| g.on_seg(s, "cistiaci")) {
                continue;
            }
            // on the lower of the two, downstream, just past the turn
            let low = |s: usize| g.segs[s].p[0].z.min(g.segs[s].p[1].z);
            let s = if low(here[1]) < low(here[0]) {
                here[1]
            } else {
                here[0]
            };
            let d = (g.segs[s].len() / 2.0).min(0.3);
            let (pipe, t) = (g.segs[s].pipe, g.segs[s].t_at(i, d));
            let dn = vent_dn(outer(&g, s));
            g.put(pipe, t, "cistiaci", Some(dn));
            out.cleanouts += 1;
        }
        // every 20 m along a run of flat stretches joined two by two
        let mut done = vec![false; g.segs.len()];
        for s0 in 0..g.segs.len() {
            if done[s0] || !flat[s0] {
                continue;
            }
            done[s0] = true;
            // the run, each stretch with the point it is entered at
            let mut run: std::collections::VecDeque<usize> = [s0].into();
            for dir in 0..2 {
                let (mut cur, mut e) = (s0, g.segs[s0].at[dir]);
                loop {
                    let nx: Vec<usize> = g.adj[e]
                        .iter()
                        .copied()
                        .filter(|&x| x != cur && flat[x] && !done[x])
                        .collect();
                    if g.adj[e].len() != 2 || nx.len() != 1 {
                        break;
                    }
                    cur = nx[0];
                    done[cur] = true;
                    if dir == 1 {
                        run.push_back(cur);
                    } else {
                        run.push_front(cur);
                    }
                    e = g.segs[cur].other(e);
                }
            }
            // walk it from its first end
            let mut from = {
                let s = run[0];
                match run.get(1) {
                    Some(&n) => {
                        let shared = g.segs[s].at.iter().find(|i| g.segs[n].at.contains(i));
                        shared.map_or(g.segs[s].at[0], |&i| g.segs[s].other(i))
                    }
                    None => g.segs[s].at[0],
                }
            };
            let mut acc = 0.0;
            for &s in &run {
                let l = g.segs[s].len();
                let to = g.segs[s].other(from);
                if g.on_seg(s, "cistiaci") || g.on_seg(s, "hlavica") {
                    acc = 0.0;
                    from = to;
                    continue;
                }
                let mut s0 = 0.0;
                while acc + (l - s0) > 20.0 {
                    s0 += 20.0 - acc;
                    acc = 0.0;
                    let (pipe, t) = (g.segs[s].pipe, g.segs[s].t_at(from, s0));
                    let dn = vent_dn(outer(&g, s));
                    g.put(pipe, t, "cistiaci", Some(dn));
                    out.cleanouts += 1;
                }
                acc += l - s0;
                from = to;
            }
        }
    }
    out.valves = g.added;
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue::heat::{unit, HeatKind};
    use crate::{Document, Element, Transaction};
    use glam::DVec2;

    fn pipe(level: ElementId, system: &str, a: [f64; 3], b: [f64; 3], outer: f64) -> Element {
        Element::new(
            "pipe",
            ElementKind::Pipe(Pipe {
                level,
                system: system.into(),
                start: DVec3::from(a),
                end: DVec3::from(b),
                pipe_type: "Cu".into(),
                nominal_mm: outer,
                outer_mm: outer,
                inner_mm: outer - 2.0,
                height_mm: None,
                insulation_mm: 0.0,
                insulation_set: false,
                mark: None,
            }),
        )
    }

    fn doc() -> (Document, ElementId) {
        let doc = Document::from_model(crate::sample::sample_house());
        let level = doc
            .model()
            .levels()
            .min_by(|a, b| a.1.elevation.total_cmp(&b.1.elevation))
            .unwrap()
            .0
            .id;
        (doc, level)
    }

    #[test]
    fn heating_and_gas_get_the_valves_at_their_ends() {
        let (mut doc, level) = doc();
        // a radiator fed by a supply and a return, a gas pipe coming in
        let sup = place(
            &mut doc,
            level,
            "Doskové vykurovacie teleso",
            DVec2::new(3.0, 2.0),
            ConnKind::UKP,
        );
        let ret = fixture::connectors(
            doc.model(),
            doc.model()
                .iter()
                .find(|e| fixture::family_of(doc.model(), e) == Some("Doskové vykurovacie teleso"))
                .unwrap(),
        )
        .into_iter()
        .find(|c| c.kind == ConnKind::UKS)
        .unwrap()
        .at;
        let far = |p: DVec3| [p.x - 2.0, p.y, p.z];
        doc.commit(
            Transaction::new("pipes")
                .insert(pipe(level, "UKP", far(sup), sup.into(), 16.0))
                .insert(pipe(level, "UKS", far(ret), ret.into(), 16.0))
                .insert(pipe(level, "ZP", [0.0, 0.0, 0.5], [0.0, 3.0, 1.0], 22.0)),
        )
        .unwrap();
        let kinds = |code: &str| -> Vec<String> {
            fill(doc.model(), code, &FillOpts::default())
                .valves
                .into_iter()
                .map(|p| p.valve.kind)
                .collect()
        };
        assert_eq!(kinds("UKP"), ["termostaticky"]);
        assert_eq!(kinds("UKS"), ["spiatocka"]);
        // the main shut-off at the lower end, 0.1 m in
        let gas = fill(doc.model(), "ZP", &FillOpts::default());
        assert_eq!(gas.assembly, ["hup"]);
        assert!(gas.valves[0].valve.t < 0.1);
    }

    /// A fixture of `family` from the catalogue at `at`, and where its
    /// connector of `kind` is.
    fn place(
        doc: &mut Document,
        level: ElementId,
        family: &str,
        at: DVec2,
        kind: ConnKind,
    ) -> DVec3 {
        let obj = crate::catalogue::starters()
            .into_iter()
            .find(|c| crate::fixture::family_of_type(&c.object) == Some(family))
            .unwrap_or_else(|| panic!("no {family} in the catalogue"));
        let (tx, id) = crate::objects::place(doc.model(), &obj, level, at, 0.0);
        doc.commit(tx).unwrap();
        let e = doc.model().get(id).unwrap();
        fixture::connectors(doc.model(), e)
            .into_iter()
            .find(|c| c.kind == kind)
            .unwrap()
            .at
    }

    fn kinds_on(f: &Fill, pipe: ElementId) -> Vec<&str> {
        let mut v: Vec<&Proposed> = f.valves.iter().filter(|p| p.valve.pipe == pipe).collect();
        v.sort_by(|a, b| a.valve.t.total_cmp(&b.valve.t));
        v.into_iter().map(|p| p.valve.kind.as_str()).collect()
    }

    #[test]
    fn the_connection_gets_its_meter_assembly_and_the_fixtures_their_angle_valves() {
        let (mut doc, level) = doc();
        let wc = place(
            &mut doc,
            level,
            "WC s nádržkou",
            DVec2::new(2.0, 1.2),
            ConnKind::SV,
        );
        let basin = place(
            &mut doc,
            level,
            "Umývadlo",
            DVec2::new(4.0, 1.2),
            ConnKind::SV,
        );
        // the main under the fixtures, a branch up to each
        let main = pipe(level, "SV", [-3.0, 0.0, 0.3], [6.0, 0.0, 0.3], 22.0);
        let main_id = main.id;
        let mut tx = Transaction::new("pipes").insert(main);
        for c in [wc, basin] {
            tx = tx
                .insert(pipe(level, "SV", [c.x, 0.0, 0.3], [c.x, c.y, 0.3], 15.0))
                .insert(pipe(level, "SV", [c.x, c.y, 0.3], [c.x, c.y, c.z], 15.0));
        }
        doc.commit(tx).unwrap();
        let f = fill(doc.model(), "SV", &FillOpts::default());
        assert_eq!(
            f.assembly,
            vec!["uzaver", "vodomer", "spatna", "filter", "redukcny"]
        );
        // in their order from the connection, before the first branch
        assert_eq!(
            kinds_on(&f, main_id),
            vec!["uzaver", "vodomer", "spatna", "filter", "redukcny"]
        );
        let first = f.valves.iter().find(|p| p.valve.kind == "uzaver").unwrap();
        assert!((first.valve.t * 9.0 - 0.1).abs() < 1e-3, "{first:?}");
        assert!(f
            .valves
            .iter()
            .all(|p| p.valve.pipe != main_id || p.valve.t * 9.0 < 3.0 + wc.x));
        assert_eq!(f.angle_valves, 2);
        // an angle valve is 8 cm off its fixture (in the middle of a
        // shorter drop)
        let rv = f.valves.iter().find(|p| p.valve.kind == "rohovy").unwrap();
        let ElementKind::Pipe(host) = &doc.model().get(rv.valve.pipe).unwrap().kind else {
            panic!()
        };
        let off = rv.valve.t.min(1.0 - rv.valve.t) * host.length();
        assert!(
            (off - (host.length() / 2.0).min(0.08)).abs() < 1e-3,
            "{off}"
        );
        assert_eq!((f.shut_offs, f.heaters), (0, 0));
        // put in the model, nothing is proposed again
        let mut tx = Transaction::new("valves");
        for p in &f.valves {
            tx = tx.insert(Element::new("v", ElementKind::Valve(p.valve.clone())));
        }
        doc.commit(tx).unwrap();
        let again = fill(doc.model(), "SV", &FillOpts::default());
        assert!(again.valves.is_empty(), "{again:?}");
    }

    #[test]
    fn a_branch_to_two_fixtures_and_a_water_heater_get_their_valves() {
        let (mut doc, level) = doc();
        let a = place(
            &mut doc,
            level,
            "Umývadlo",
            DVec2::new(2.0, 2.0),
            ConnKind::SV,
        );
        let b = place(
            &mut doc,
            level,
            "Umývadlo",
            DVec2::new(3.0, 2.0),
            ConnKind::SV,
        );
        let tank = unit(HeatKind::Cylinder, 200);
        let (tx, id) = crate::objects::place(doc.model(), &tank, level, DVec2::new(8.0, 2.0), 0.0);
        doc.commit(tx).unwrap();
        let e = doc.model().get(id).unwrap();
        let sv = fixture::connectors(doc.model(), e)
            .into_iter()
            .find(|c| c.kind == ConnKind::SV)
            .unwrap()
            .at;
        assert_eq!(plant_connectors(doc.model(), ConnKind::SV), vec![sv]);
        let main = pipe(level, "SV", [-3.0, 0.0, 0.05], [sv.x, 0.0, 0.05], 22.0);
        let to_tank = pipe(level, "SV", [sv.x, 0.0, 0.05], [sv.x, sv.y, 0.05], 22.0);
        let to_tank_id = to_tank.id;
        let branch = pipe(level, "SV", [2.0, 0.0, 0.05], [2.0, a.y, 0.05], 18.0);
        let branch_id = branch.id;
        let tx = Transaction::new("pipes")
            .insert(main)
            .insert(to_tank)
            .insert(pipe(
                level,
                "SV",
                [sv.x, sv.y, 0.05],
                [sv.x, sv.y, sv.z],
                22.0,
            ))
            .insert(branch)
            .insert(pipe(level, "SV", [2.0, a.y, 0.05], [a.x, a.y, a.z], 15.0))
            .insert(pipe(level, "SV", [2.0, a.y, 0.05], [b.x, b.y, 0.05], 15.0))
            .insert(pipe(level, "SV", [b.x, b.y, 0.05], [b.x, b.y, b.z], 15.0));
        doc.commit(tx).unwrap();
        let f = fill(doc.model(), "SV", &FillOpts::default());
        // the root is the connection, not the heater's inlet near the floor
        assert_eq!(f.assembly.len(), 5);
        assert_eq!((f.heaters, f.shut_offs, f.angle_valves), (1, 1, 2));
        // the check valve nearer the heater, the shut-off before it
        assert_eq!(kinds_on(&f, to_tank_id), vec!["uzaver", "spatna"]);
        assert_eq!(kinds_on(&f, branch_id), vec!["uzaver"]);
    }

    #[test]
    fn a_stack_gets_its_vent_head_and_cleanout_and_a_long_drain_its_cleanouts() {
        let (mut doc, level) = doc();
        let stack = pipe(level, "K", [1.0, 1.0, -0.8], [1.0, 1.0, 6.5], 110.0);
        let stack_id = stack.id;
        let drain = pipe(level, "K", [1.0, 1.0, -0.8], [1.0, 30.0, -1.4], 110.0);
        let drain_id = drain.id;
        let out = pipe(level, "K", [1.0, 30.0, -1.4], [9.0, 30.0, -1.6], 110.0);
        let out_id = out.id;
        doc.commit(
            Transaction::new("pipes")
                .insert(stack)
                .insert(drain)
                .insert(out),
        )
        .unwrap();
        let f = fill(doc.model(), "K", &FillOpts::default());
        assert_eq!(f.vents, 1);
        let on =
            |id| -> Vec<&Proposed> { f.valves.iter().filter(|p| p.valve.pipe == id).collect() };
        let s = on(stack_id);
        let head = s.iter().find(|p| p.valve.kind == "hlavica").unwrap();
        assert_eq!((head.valve.t, head.dn), (1.0, Some(100)));
        // the cleanout 1 m over the ground floor
        let c = s.iter().find(|p| p.valve.kind == "cistiaci").unwrap();
        assert!((c.valve.t * 7.3 - 0.8 - 1.0).abs() < 1e-3, "{c:?}");
        // 29 m of drain: one after 20 m; past the 90° turn another
        assert_eq!(on(drain_id).len(), 1);
        assert_eq!(on(out_id).len(), 1);
        assert_eq!(f.cleanouts, 3);
        let none = fill(
            doc.model(),
            "K",
            &FillOpts {
                vents: false,
                cleanouts: false,
                ..FillOpts::default()
            },
        );
        assert!(none.valves.is_empty());
        assert!(fill(doc.model(), "UKP", &FillOpts::default()).unsupported);
    }

    #[test]
    fn which_fixtures_take_an_angle_valve() {
        for f in ["Umývadlo", "WC", "Drez", "Bidet", "Práčka", "Pisoár"] {
            assert!(uses_angle_valve(f), "{f}");
        }
        assert!(!uses_angle_valve("Sprcha") && !uses_angle_valve("Vaňa"));
        assert_eq!(
            (vent_dn(75.0), vent_dn(110.0), vent_dn(160.0)),
            (75, 100, 125)
        );
    }
}
