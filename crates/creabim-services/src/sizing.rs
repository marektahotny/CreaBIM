//! Choosing pipe sizes: the smallest in a pipe's series within the limits
//! of velocity and friction, then the rules that keep a network sensible
//! (no pipe smaller than one after it, one size between branches).

use crate::fixtures::normalize;
use crate::hydraulics;
use crate::network::{Network, NodeId, PipeSize, SizeNote};
use crate::Fluid;
use std::collections::{HashMap, HashSet};

/// Sorted by inner diameter, stable as LINQ's OrderBy.
fn by_inner(sizes: impl Iterator<Item = PipeSize>) -> Vec<PipeSize> {
    let mut v: Vec<PipeSize> = sizes.collect();
    v.sort_by(|a, b| a.inner_mm.total_cmp(&b.inner_mm));
    v
}

/// The size in the series matching the pipe's own.
fn current_size_entry(net: &Network, p: NodeId) -> Option<PipeSize> {
    let n = &net.nodes[p];
    n.sizes
        .iter()
        .copied()
        .find(|s| (s.nominal_mm - n.nominal_mm).abs() < 0.6)
}

/// Choose the pipe's size, or take the one chosen by hand.
pub(crate) fn choose_size(
    net: &mut Network,
    p: NodeId,
    fluid: &Fluid,
    vmax: f64,
    rmax: f64,
    min_inner_mm: f64,
    only_upsize: bool,
) -> Option<PipeSize> {
    net.nodes[p].vmax_used = vmax;
    let auto = choose_size_auto(net, p, fluid, vmax, rmax, min_inner_mm, only_upsize);
    net.nodes[p].auto_size = auto;
    match net.nodes[p].manual_size {
        Some(m) => {
            net.nodes[p].size_notes = vec![SizeNote::Manual];
            Some(m)
        }
        None => auto,
    }
}

fn choose_size_auto(
    net: &mut Network,
    p: NodeId,
    fluid: &Fluid,
    vmax: f64,
    rmax: f64,
    min_inner_mm: f64,
    only_upsize: bool,
) -> Option<PipeSize> {
    net.nodes[p].size_notes.clear();
    let sizes = by_inner(
        net.nodes[p]
            .sizes
            .iter()
            .copied()
            .filter(|s| s.inner_mm > 0.0),
    );
    if sizes.is_empty() {
        return None;
    }
    if net.nodes[p].flow_ls <= 0.0 {
        net.nodes[p].size_notes.push(SizeNote::NoFlow);
        return current_size_entry(net, p);
    }
    let n = &mut net.nodes[p];
    let cur_inner = n.inner_diameter_m * 1000.0;
    n.min_ok_size = None;
    n.kept_oversized = false;
    let any_reaches_connector = sizes.iter().any(|x| x.nominal_mm + 0.6 >= n.min_nominal_mm);
    for s in &sizes {
        let d = s.inner_mm / 1000.0;
        let v = hydraulics::velocity(n.flow_ls, d);
        let r = hydraulics::r(n.flow_ls, d, n.roughness_m, fluid);
        if s.inner_mm + 0.01 < min_inner_mm {
            continue;
        }
        if n.min_nominal_mm > 0.0 && s.nominal_mm + 0.6 < n.min_nominal_mm && any_reaches_connector
        {
            continue;
        }
        let ok = v <= vmax + 1e-9 && (rmax <= 0.0 || r <= rmax + 1e-9);
        if ok && n.min_ok_size.is_none() {
            n.min_ok_size = Some(*s);
        }
        if only_upsize && s.inner_mm + 0.01 < cur_inner {
            continue;
        }
        if ok {
            if only_upsize
                && n.min_ok_size
                    .is_some_and(|m| m.inner_mm + 0.05 < s.inner_mm)
            {
                n.kept_oversized = true;
                n.size_notes.push(SizeNote::KeptOversized);
            }
            return Some(*s);
        }
    }
    n.size_notes.push(SizeNote::NoneFits);
    sizes.last().copied()
}

/// The inner diameter the pipe needs, mm: what it was given or proposed,
/// or for one kept oversized, the least that would do.
fn needed_inner_mm(net: &Network, i: NodeId) -> f64 {
    let n = &net.nodes[i];
    if let Some(m) = n.manual_size {
        return m.inner_mm;
    }
    match n.proposed {
        None => n.inner_diameter_m * 1000.0,
        Some(p) => {
            if n.kept_oversized && !n.is_size_changed() {
                if let Some(m) = n.min_ok_size {
                    return m.inner_mm;
                }
            }
            p.inner_mm
        }
    }
}

/// No pipe smaller than any after it.
pub(crate) fn enforce_monotonic(net: &mut Network) {
    let mut max_down: HashMap<NodeId, f64> = HashMap::new();
    for i in net.post_order() {
        let mut m = net.nodes[i]
            .children
            .iter()
            .map(|c| max_down[c])
            .fold(None, |a: Option<f64>, b| Some(a.map_or(b, |a| a.max(b))))
            .unwrap_or(0.0);
        if net.nodes[i].is_pipe() {
            let n = &net.nodes[i];
            let mut inner = n
                .proposed
                .map_or(n.inner_diameter_m * 1000.0, |p| p.inner_mm);
            if inner + 0.05 < m && !n.sizes.is_empty() && n.manual_size.is_none() {
                let s = by_inner(n.sizes.iter().copied().filter(|x| x.inner_mm + 0.05 >= m))
                    .first()
                    .copied();
                if let Some(s) = s {
                    let n = &mut net.nodes[i];
                    n.proposed = Some(s);
                    n.size_notes.push(SizeNote::Monotonic);
                    inner = s.inner_mm;
                }
            }
            m = m.max(inner.min(needed_inner_mm(net, i)));
        }
        max_down.insert(i, m);
    }
}

/// The runs between branches: each start and the pipes along it.
fn runs(net: &Network, with: impl Fn(&Network, NodeId) -> bool) -> Vec<Vec<NodeId>> {
    let mut visited = HashSet::new();
    let mut out = vec![];
    for start in net.pre_order() {
        let parent_single = net.nodes[start]
            .parent
            .is_some_and(|p| net.nodes[p].children.len() == 1);
        if visited.contains(&start) || parent_single {
            continue;
        }
        let mut run = vec![];
        let mut n = Some(start);
        while let Some(i) = n {
            visited.insert(i);
            if with(net, i) {
                run.push(i);
            }
            if net.nodes[i].children.len() != 1 {
                break;
            }
            n = Some(net.nodes[i].children[0]);
        }
        out.push(run);
    }
    out
}

/// A material's key: a run is made one size only within one material.
fn material_key(name: &str) -> String {
    let n = normalize(name);
    if n.contains("pex")
        || n.contains("alpex")
        || n.contains("viacvrstv")
        || n.contains("multilayer")
        || n.contains("pe-al")
        || n.contains("pe al")
    {
        return "PexAlPex".into();
    }
    let sp = format!(" {} ", n.replace(['-', '_'], " "));
    if sp.contains(" pe 100")
        || sp.contains(" pe100")
        || sp.contains(" sdr")
        || sp.contains("hdpe")
        || (sp.contains(" pe ") && !sp.contains(" al "))
    {
        return "PE".into();
    }
    if n.contains("med")
        || n.contains("cu ")
        || n.starts_with("cu")
        || n.contains("copper")
        || n.contains("en 1057")
    {
        return "Med".into();
    }
    if n.contains("ocel")
        || n.contains("steel")
        || n.contains("10255")
        || n.contains("zavit")
        || n.contains("cierna")
    {
        return "Ocel".into();
    }
    name.to_string()
}

/// One size along each run between branches (per material).
pub(crate) fn uniform_runs(net: &mut Network) {
    let all = runs(net, |net, i| {
        let n = &net.nodes[i];
        n.is_pipe() && n.proposed.is_some() && !n.sizes.is_empty() && n.manual_size.is_none()
    });
    for run in all {
        let mut parts: Vec<Vec<NodeId>> = vec![];
        for i in run {
            let key = material_key(&net.nodes[i].material);
            match parts.last_mut() {
                Some(p) if material_key(&net.nodes[p[0]].material) == key => p.push(i),
                _ => parts.push(vec![i]),
            }
        }
        for part in parts {
            uniform_part(net, &part);
        }
    }
}

fn uniform_part(net: &mut Network, run: &[NodeId]) {
    if run.len() < 2 {
        return;
    }
    // the first with the most need, as a stable descending sort gives
    let mut max_node = run[0];
    for &i in &run[1..] {
        if needed_inner_mm(net, i) > needed_inner_mm(net, max_node) {
            max_node = i;
        }
    }
    let need = needed_inner_mm(net, max_node);
    let mn = &net.nodes[max_node];
    let max = match mn.proposed {
        Some(p) if p.inner_mm <= need + 0.05 => p,
        _ => match by_inner(
            mn.sizes
                .iter()
                .copied()
                .filter(|x| x.inner_mm + 0.05 >= need),
        )
        .first()
        {
            Some(s) => *s,
            None => return,
        },
    };
    for &p in run {
        let n = &net.nodes[p];
        if n.proposed
            .is_some_and(|s| s.inner_mm + 0.05 >= max.inner_mm)
        {
            continue;
        }
        let same = by_inner(
            n.sizes
                .iter()
                .copied()
                .filter(|x| x.inner_mm + 0.05 >= max.inner_mm),
        )
        .first()
        .copied();
        if let Some(s) = same {
            let n = &mut net.nodes[p];
            n.proposed = Some(s);
            n.size_notes.push(SizeNote::UniformRun);
        }
    }
}

/// Where a run keeps one pipe oversized, take it to its run's size, so the
/// change of size falls at a tee rather than mid-run.
pub(crate) fn unify_kept_oversized(net: &mut Network) {
    let all = runs(net, |net, i| {
        let n = &net.nodes[i];
        n.is_pipe() && n.proposed.is_some() && !n.sizes.is_empty()
    });
    for run in all {
        let kept: Vec<NodeId> = run
            .iter()
            .copied()
            .filter(|&i| {
                let n = &net.nodes[i];
                n.kept_oversized && !n.is_size_changed() && n.manual_size.is_none()
            })
            .collect();
        if kept.is_empty() || run.len() < 2 {
            continue;
        }
        let need = run
            .iter()
            .map(|&i| needed_inner_mm(net, i))
            .fold(f64::MIN, f64::max);
        let target = by_inner(
            run.iter()
                .filter(|i| !kept.contains(i))
                .filter_map(|&i| net.nodes[i].proposed)
                .filter(|p| p.inner_mm + 0.05 >= need),
        )
        .first()
        .copied();
        let Some(target) = target else {
            continue;
        };
        for p in kept {
            let n = &net.nodes[p];
            if n.proposed
                .is_some_and(|s| s.inner_mm <= target.inner_mm + 0.05)
            {
                continue;
            }
            let to = n
                .sizes
                .iter()
                .copied()
                .find(|x| (x.nominal_mm - target.nominal_mm).abs() < 0.6);
            if let Some(to) = to {
                if to.inner_mm + 0.05 >= need {
                    let n = &mut net.nodes[p];
                    n.proposed = Some(to);
                    n.kept_oversized = false;
                    n.size_notes.retain(|s| *s != SizeNote::KeptOversized);
                    n.size_notes.push(SizeNote::UniformRun);
                }
            }
        }
    }
}

/// The next size up in the pipe's series, none for one chosen by hand.
pub(crate) fn next_larger(net: &Network, p: NodeId) -> Option<PipeSize> {
    let n = &net.nodes[p];
    if n.manual_size.is_some() {
        return None;
    }
    let cur = n
        .proposed
        .map_or(n.inner_diameter_m * 1000.0, |s| s.inner_mm);
    by_inner(n.sizes.iter().copied().filter(|s| s.inner_mm > cur + 0.05))
        .first()
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn materials_are_told_apart_by_name() {
        assert_eq!(material_key("Meď"), "Med");
        assert_eq!(material_key("PE 100 SDR 11"), "PE");
        assert_eq!(material_key("Alpex 16"), "PexAlPex");
        assert_eq!(material_key("Oceľ závitová"), "Ocel");
        assert_eq!(material_key("PPR"), "PPR");
    }
}
