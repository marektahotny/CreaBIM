//! The plinth (sokel) of an outer wall: below a line some way over the
//! finished ground, the facade insulation gives way to extruded
//! polystyrene (XPS) on a waterproofing membrane turned up the masonry,
//! flush with the facade or set back from it, its render the plinth's own
//! (George 2026-11-22, after his details of a recessed and a flush
//! plinth, without a maker's names).
//!
//! It is set on the wall's build-up, so every wall of that type has it;
//! a wall may set its own top, in steps along it where the ground can't
//! be levelled.

use crate::{Element, ElementId, ElementKind, Model, Wall};
use glam::DVec2;
use serde::{Deserialize, Serialize};

/// A wall build-up's plinth.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plinth {
    /// Its insulation (XPS); none keeps the facade's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub insulation: Option<ElementId>,
    /// Its insulation's thickness, m: less than the facade insulation's
    /// sets the plinth back (uskočený sokel), as much is flush with it.
    pub thickness: f64,
    /// The waterproofing turned up the masonry behind it; none, no
    /// membrane.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub membrane: Option<ElementId>,
    /// How far the membrane runs up past the plinth's top, m.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub lap: f64,
    /// The plinth's render; none keeps the facade's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render: Option<ElementId>,
    /// Its top over the highest finished ground along the wall, m.
    pub above: f64,
    /// Where it ends at the bottom.
    #[serde(default)]
    pub bottom: PlinthBottom,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// Where a plinth ends at the bottom.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum PlinthBottom {
    /// At the wall's base: the masonry under it (DT blocks) carries its
    /// own insulation and membrane on down to the footing, butted to it.
    #[default]
    WallBase,
    /// Down over the side of the slab the wall stands on (a foundation
    /// slab), and this much further, m.
    SlabSide(f64),
}

impl PlinthBottom {
    pub const KINDS: [&'static str; 2] = ["At the wall's base", "Over the slab's side"];
    pub fn kind(self) -> usize {
        match self {
            PlinthBottom::WallBase => 0,
            PlinthBottom::SlabSide(_) => 1,
        }
    }
}

/// The usual plinth: XPS as thick as the facade insulation, 300 mm over
/// the ground.
pub const ABOVE_GROUND: f64 = 0.3;

impl Plinth {
    pub fn new(thickness: f64) -> Self {
        Self {
            insulation: None,
            thickness,
            membrane: None,
            lap: 0.0,
            render: None,
            above: ABOVE_GROUND,
            bottom: PlinthBottom::WallBase,
        }
    }
}

/// A wall's own plinth top from `from` metres along it on, a height
/// against the project's zero.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlinthStep {
    pub from: f64,
    pub top: f64,
}

/// The plinth of a wall's build-up, if it has one.
pub fn of_wall<'a>(model: &'a Model, w: &Wall) -> Option<&'a Plinth> {
    let a = w.assembly.and_then(|id| model.get(id))?;
    match &a.kind {
        ElementKind::Assembly(a) => a.plinth.as_ref(),
        _ => None,
    }
}

/// The highest finished ground along the segment `a`–`b`, against the
/// project's zero: the terrains and paved areas over it, sampled every
/// 0.25 m. None when no ground lies under it.
pub fn ground_along(model: &Model, a: DVec2, b: DVec2) -> Option<f64> {
    let n = ((a.distance(b) / 0.25).ceil() as usize).max(1);
    let pts: Vec<DVec2> = (0..=n).map(|i| a.lerp(b, i as f64 / n as f64)).collect();
    let mut best: Option<f64> = None;
    for e in model.iter() {
        let ElementKind::Site(s) = &e.kind else {
            continue;
        };
        let ground = matches!(
            s.form,
            crate::site::SiteForm::Terrain { .. } | crate::site::SiteForm::Area { .. }
        );
        if !ground {
            continue;
        }
        let Some(boundary) = s.boundary() else {
            continue;
        };
        let base = model.level(s.level).map_or(0.0, |l| l.elevation);
        let inside: Vec<DVec2> = pts
            .iter()
            .copied()
            .filter(|p| crate::polygon::contains(boundary, *p))
            .collect();
        if inside.is_empty() {
            continue;
        }
        for z in s.ground_heights(&inside) {
            let z = base + z;
            best = Some(best.map_or(z, |b: f64| b.max(z)));
        }
    }
    best
}

/// The wall's base against the project's zero.
pub fn base_z(model: &Model, w: &Wall) -> f64 {
    model.level(w.level).map_or(0.0, |l| l.elevation) + w.base_offset
}

/// The plinth's top along the wall: pieces (from, to, height) in metres
/// along it from its start, against the project's zero; the wall's own
/// steps, or the highest ground along its outer face plus `above`. With
/// no ground at all, `above` over the wall's base.
pub fn tops(model: &Model, w: &Wall, p: &Plinth) -> Vec<(f64, f64, f64)> {
    let len = w.length();
    if !w.plinth_steps.is_empty() {
        let mut steps = w.plinth_steps.clone();
        steps.sort_by(|a, b| a.from.total_cmp(&b.from));
        let mut out = vec![];
        for (i, s) in steps.iter().enumerate() {
            let from = if i == 0 { 0.0 } else { s.from.clamp(0.0, len) };
            let to = steps.get(i + 1).map_or(len, |n| n.from.clamp(0.0, len));
            if to > from + 1e-6 {
                out.push((from, to, s.top));
            }
        }
        return out;
    }
    // along the outer face
    let d = w.direction();
    let n = DVec2::new(d.y, -d.x) * if w.flip { -1.0 } else { 1.0 };
    let off = n * (w.thickness / 2.0);
    let ground = ground_along(model, w.start + off, w.end + off);
    let top = ground.unwrap_or_else(|| base_z(model, w)) + p.above;
    vec![(0.0, len, top)]
}

/// How far below the wall's base the plinth reaches, m.
pub fn below_base(model: &Model, e: &Element, w: &Wall, p: &Plinth) -> f64 {
    match p.bottom {
        PlinthBottom::WallBase => 0.0,
        PlinthBottom::SlabSide(more) => slab_under(model, e.id, w).unwrap_or(0.0) + more.max(0.0),
    }
}

/// The thickness of the slab the wall stands on: one whose top is at the
/// wall's base and whose outline holds the wall's middle.
pub fn slab_under(model: &Model, _id: ElementId, w: &Wall) -> Option<f64> {
    let z = base_z(model, w);
    let mid = (w.start + w.end) / 2.0;
    model.iter().find_map(|e| match &e.kind {
        ElementKind::Slab(s) => {
            let level = model.level(s.level)?.elevation;
            let top = s.top(level);
            ((top - z).abs() < 0.05 && crate::polygon::contains(&s.boundary, mid))
                .then_some(s.thickness)
        }
        _ => None,
    })
}
