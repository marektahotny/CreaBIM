//! Plan drawings generated from the model: walls cut at a height, openings
//! drawn with architectural symbols, everything below shown in projection.

use crate::wall::Cut;
use creabim_core::{Curve, ElementId, ElementKind, Model, OpeningKind};
use glam::{DVec2, DVec3};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pen {
    /// Material cut by the plan plane: heavy line, filled.
    Cut,
    /// Visible below the cut plane: light line.
    Projection,
    /// Symbols such as door swings: light line.
    Symbol,
    /// Small things seen below the cut, furniture and fixtures: the
    /// lightest line.
    Fine,
    /// Grid lines: dashed.
    Grid,
    /// Construction guides: shown while working, never printed.
    Guide,
    /// Above the cut plane (beams, the stair past its break): dashed.
    Hidden,
    /// The wall carrying on over a door, above the cut plane: a thin
    /// line of a dash and two dots.
    Overhead,
    /// A stair above the cut plane (STN 01 3420, 5.9.3 d): its seen edges
    /// in the projection's weight, dash-dot; with `hidden`, the edges it
    /// hides, dash and two dots.
    StairAbove { hidden: bool },
    /// Where a footing's underside steps down: a heavy dashed line.
    HiddenCut,
    /// Another level traced under a plan: halftone, never printed, not
    /// picked, but snapped to.
    Underlay,
    /// A drafted line in a pen and line type of its own (the Line and
    /// Polyline tools).
    Drafted(creabim_core::LineLook),
}

pub use creabim_core::Hatch;

/// The hatch for an element's cut faces: its material's, or its type's default.
fn is_footing(model: &Model, id: ElementId) -> bool {
    matches!(
        model.get(id).map(|e| &e.kind),
        Some(ElementKind::Footing(_))
    )
}

/// Ground, paving or a plant: what everything built cuts through.
fn is_site(model: &Model, id: ElementId) -> bool {
    matches!(model.get(id).map(|e| &e.kind), Some(ElementKind::Site(_)))
}

pub fn hatch_of(model: &Model, id: ElementId) -> Hatch {
    let Some(e) = model.get(id) else {
        return Hatch::Solid;
    };
    if let Some((_, m)) = creabim_core::material_of(model, e) {
        return m.hatch;
    }
    match &e.kind {
        ElementKind::Wall(_) => Hatch::Diagonal,
        ElementKind::Slab(_) => Hatch::Cross,
        _ => Hatch::Solid,
    }
}

/// Hatch line segments clipped to `polygon`, with a base spacing of
/// `spacing` drawing units. Stipple dots come back as very short segments.
pub fn hatch_lines(hatch: Hatch, polygon: &[DVec2], spacing: f64) -> Vec<(DVec2, DVec2)> {
    // the built-in pattern, so the screen draws what paper prints
    if spacing <= 0.0 {
        return vec![];
    }
    crate::looks::pattern_lines(
        &creabim_core::styles::builtin_pattern(hatch),
        polygon,
        creabim_core::styles::BASE_SPACING / spacing,
    )
}

fn point_in(p: DVec2, poly: &[DVec2]) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    inside
}

/// Which cut wins where two overlap: higher numbers cut through lower ones.
fn cut_priority(model: &Model, id: ElementId) -> u8 {
    model
        .get(id)
        .map_or(0, |e| creabim_core::joins::category_rank(&e.kind))
}

/// Elements of the same material (or, without materials, the same type)
/// merge into one mass where they touch; different ones keep a line between.
fn material_key(model: &Model, id: ElementId) -> (Option<ElementId>, u8) {
    match model.get(id) {
        Some(e) => match creabim_core::material_of(model, e) {
            Some((m, _)) => (Some(m.id), 0),
            None => (None, 1 + cut_priority(model, id)),
        },
        None => (None, 0),
    }
}

/// A piece of cut material: an element's cut, or one layer's of a layered
/// element. [`clean_up_pieces`] turns them into fills and outlines.
#[derive(Clone, Debug, PartialEq)]
pub struct CutPiece {
    pub id: ElementId,
    pub points: Vec<DVec2>,
    pub layer: Option<crate::scene::LayerPart>,
}

/// What decides which cut wins where two overlap: the junction rules set
/// by hand, and the order elements were drawn in.
struct Joining {
    rules: Vec<creabim_core::JoinRule>,
    order: std::collections::HashMap<ElementId, usize>,
}

impl Joining {
    fn of(model: &Model) -> Self {
        Self {
            rules: creabim_core::joins::rules(model),
            order: model.iter().enumerate().map(|(i, e)| (e.id, i)).collect(),
        }
    }

    /// Does `a` cut through `b` where they overlap? By the junction's own
    /// rule if it has one, else layer by layer: the layer with the lower
    /// join priority runs through (cores join cores, finishes stop at the
    /// structure), then columns through walls through slabs, then the
    /// element drawn first. An element without layers is all structure.
    fn cuts(&self, model: &Model, a: &CutPiece, b: &CutPiece) -> bool {
        if a.id == b.id {
            return false;
        }
        if let Some(r) = self.rules.iter().find(|r| r.joins(a.id, b.id)) {
            if !r.cleanup {
                return false;
            }
            if let Some(w) = r.wins(a.id, b.id) {
                return w;
            }
        }
        let strength = |p: &CutPiece| {
            (
                std::cmp::Reverse(p.layer.map_or(1, |l| l.priority)),
                cut_priority(model, p.id),
            )
        };
        let (sa, sb) = (strength(a), strength(b));
        if sa != sb {
            return sa > sb;
        }
        // as strong: the same material merges into one mass where they
        // overlap (see the outlines below); different ones are told
        // apart by the order they were drawn in
        if a.key(model) == b.key(model) {
            return false;
        }
        let order = |p: &CutPiece| self.order.get(&p.id).copied().unwrap_or(usize::MAX);
        order(a) < order(b)
    }
}

impl CutPiece {
    /// What it merges with: the same material, or for a layer without
    /// one, layers of the same function (air gaps run on into each other).
    fn key(&self, model: &Model) -> (Option<ElementId>, u8) {
        match self.layer {
            None => material_key(model, self.id),
            Some(l) => match l.material {
                Some(m) if model.get(m).is_some() => (Some(m), 0),
                _ if l.function == creabim_core::LayerFunction::AirGap => AIR,
                _ => (None, 1 + cut_priority(model, self.id)),
            },
        }
    }

    /// The colour of the material this piece cuts: its layer's, else the
    /// element's own (or its core layer's); none for an air gap.
    fn colour(&self, model: &Model, section: bool) -> Option<[f32; 3]> {
        if is_footing(model, self.id) && !section {
            return None;
        }
        let mat = |id: ElementId| match &model.get(id)?.kind {
            ElementKind::Material(m) => Some(m.color),
            _ => None,
        };
        match self.layer {
            Some(l) => l.material.and_then(mat),
            None => {
                let e = model.get(self.id)?;
                creabim_core::material_of(model, e)
                    .map(|(_, m)| m.color)
                    .or_else(|| {
                        let (_, a) = creabim_core::assembly::assembly_of(model, e)?;
                        let i = (0..a.layers.len()).find(|i| a.is_core(*i))?;
                        mat(a.layers[i].material?)
                    })
            }
        }
    }

    fn hatch(&self, model: &Model, section: bool) -> Hatch {
        // footings are drawn in outline only in plans (STN practice); a
        // section fills them with their material like any cut, concrete
        // when none is chosen
        if is_footing(model, self.id) {
            if !section {
                return Hatch::Empty;
            }
            if self.layer.is_none() {
                return match model.get(self.id) {
                    Some(e) => creabim_core::material_of(model, e)
                        .map_or(Hatch::Concrete, |(_, m)| m.hatch),
                    None => Hatch::Concrete,
                };
            }
        }
        match self.layer {
            None => hatch_of(model, self.id),
            Some(l) => match l.material.and_then(|m| model.get(m)).map(|e| &e.kind) {
                Some(ElementKind::Material(m)) => m.hatch,
                _ if l.function == creabim_core::LayerFunction::AirGap => Hatch::Empty,
                _ => match model.get(self.id).map(|e| &e.kind) {
                    Some(ElementKind::Wall(_)) => Hatch::Diagonal,
                    Some(ElementKind::Slab(_)) => Hatch::Cross,
                    _ => Hatch::Solid,
                },
            },
        }
    }
}

fn is_convex(poly: &[DVec2]) -> bool {
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut sign = 0.0;
    for i in 0..n {
        let (a, b, c) = (poly[i], poly[(i + 1) % n], poly[(i + 2) % n]);
        let z = (b - a).perp_dot(c - b);
        if z.abs() < 1e-12 {
            continue;
        }
        if sign == 0.0 {
            sign = z.signum();
        } else if z.signum() != sign {
            return false;
        }
    }
    sign != 0.0
}

fn signed_area(poly: &[DVec2]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| poly[i].perp_dot(poly[(i + 1) % n]))
        .sum::<f64>()
        / 2.0
}

/// Keep the part of `poly` on the side of a → b where `keep(side)` holds,
/// side being the signed distance (left positive).
fn clip_half(poly: &[DVec2], a: DVec2, b: DVec2, left: bool) -> Vec<DVec2> {
    let d = (b - a).normalize_or_zero();
    let side = |p: DVec2| {
        let s = d.perp_dot(p - a);
        if left {
            s
        } else {
            -s
        }
    };
    let mut out = Vec::new();
    for i in 0..poly.len() {
        let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
        let (sp, sq) = (side(p), side(q));
        if sp >= 0.0 {
            out.push(p);
        }
        if (sp >= 0.0) != (sq >= 0.0) {
            out.push(p + (q - p) * (sp / (sp - sq)));
        }
    }
    out
}

/// `subject` minus the convex polygon `hole`, as convex-ish pieces.
pub fn subtract_convex(subject: &[DVec2], hole: &[DVec2]) -> Vec<Vec<DVec2>> {
    let ccw = signed_area(hole) > 0.0;
    let n = hole.len();
    let mut rest = subject.to_vec();
    let mut pieces = Vec::new();
    for i in 0..n {
        let (a, b) = (hole[i], hole[(i + 1) % n]);
        // outside this edge of the hole is kept; inside goes on to the next edge
        let outside = clip_half(&rest, a, b, !ccw);
        if outside.len() >= 3 && signed_area(&outside).abs() > 1e-9 {
            pieces.push(outside);
        }
        rest = clip_half(&rest, a, b, ccw);
        if rest.len() < 3 {
            break;
        }
    }
    pieces
}

/// [`subtract_convex`] for a long, not convex subject such as the ground
/// in a section: it is first cut into the part left of the hole, the part
/// right of it and the band across it, so that the hole's top and bottom
/// edges only clip that band (cut along the whole ground, a dip in its
/// surface would leave a sliver joining its two sides).
pub fn subtract_convex_banded(subject: &[DVec2], hole: &[DVec2]) -> Vec<Vec<DVec2>> {
    let (lo, hi) = hole.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
        (lo.min(p.x), hi.max(p.x))
    });
    let up = DVec2::Y;
    let (l0, l1) = (DVec2::new(lo, 0.0), DVec2::new(hi, 0.0));
    // left of the line going up is x < lo
    let left = clip_half(subject, l0, l0 + up, true);
    let rest = clip_half(subject, l0, l0 + up, false);
    let right = clip_half(&rest, l1, l1 + up, false);
    let band = clip_half(&rest, l1, l1 + up, true);
    let mut out = vec![];
    for p in [left, right] {
        if p.len() >= 3 && signed_area(&p).abs() > 1e-9 {
            out.push(p);
        }
    }
    if band.len() >= 3 && signed_area(&band).abs() > 1e-9 {
        out.extend(subtract_convex(&band, hole));
    }
    out
}

/// Do two polygons' bounding boxes overlap by more than a sliver?
fn boxes_overlap(a: &[DVec2], b: &[DVec2]) -> bool {
    let bb = |p: &[DVec2]| {
        p.iter().fold((DVec2::MAX, DVec2::MIN), |(lo, hi), q| {
            (lo.min(*q), hi.max(*q))
        })
    };
    let ((alo, ahi), (blo, bhi)) = (bb(a), bb(b));
    let eps = 1e-6;
    alo.x < bhi.x - eps && blo.x < ahi.x - eps && alo.y < bhi.y - eps && blo.y < ahi.y - eps
}

/// The stretches of segment `a` → `b` running strictly inside `poly`, as
/// distances along it.
fn inside_spans(a: DVec2, b: DVec2, poly: &[DVec2], eps: f64) -> Vec<(f64, f64)> {
    let len = a.distance(b);
    if len < eps || poly.len() < 3 {
        return vec![];
    }
    let d = (b - a) / len;
    let mut ts = vec![0.0, len];
    let n = poly.len();
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let e = q - p;
        let den = d.perp_dot(e);
        if den.abs() < 1e-12 {
            continue;
        }
        // a + d t = p + e s
        let t = (p - a).perp_dot(e) / den;
        let s = (p - a).perp_dot(d) / den;
        if (-1e-9..=1.0 + 1e-9).contains(&s) && t > 0.0 && t < len {
            ts.push(t);
        }
    }
    ts.sort_by(|x, y| x.total_cmp(y));
    ts.dedup_by(|x, y| (*x - *y).abs() < eps);
    ts.windows(2)
        .filter(|w| w[1] > w[0] + eps)
        .filter(|w| {
            let m = a + d * ((w[0] + w[1]) / 2.0);
            point_in(m, poly) && edge_distance(m, poly) > eps
        })
        .map(|w| (w[0], w[1]))
        .collect()
}

/// The cut pieces as drawn: where two overlap the stronger cuts through
/// the other (see `Joining::cuts`), and in a section the ground gives way
/// to everything built in it.
pub fn resolve_cuts(model: &Model, cuts: Vec<CutPiece>, section: bool) -> Vec<CutPiece> {
    // where cuts overlap, the stronger cuts through the other (see
    // `Joining::cuts`): a wall runs through the end of the slab that sits
    // in it, and the slab's structure through the wall's plaster
    let joining = Joining::of(model);
    // (only cuts whose boxes meet can touch: a tower's floors have
    // thousands of pieces)
    let bb = |p: &[DVec2]| {
        p.iter().fold((DVec2::MAX, DVec2::MIN), |(lo, hi), q| {
            (lo.min(*q), hi.max(*q))
        })
    };
    let near = |a: (DVec2, DVec2), b: (DVec2, DVec2), eps: f64| {
        a.0.x <= b.1.x + eps && b.0.x <= a.1.x + eps && a.0.y <= b.1.y + eps && b.0.y <= a.1.y + eps
    };
    let cut_boxes: Vec<_> = cuts.iter().map(|c| bb(&c.points)).collect();
    let mut polys: Vec<CutPiece> = Vec::new();
    for (ci, c) in cuts.iter().enumerate() {
        let mut pieces = vec![c.points.clone()];
        // the ground is cut by everything built in it (footings, walls,
        // slabs), never the other way round, so the line where they meet
        // is a thin one and its thick line stays only where it meets air
        // (in sections: plans keep their own rules)
        let ground = section && is_site(model, c.id);
        for (oi, other) in cuts.iter().enumerate() {
            let built = !is_site(model, other.id);
            let cuts_it = if ground && built {
                true
            } else if section && !ground && !built {
                false
            } else {
                joining.cuts(model, other, c)
            };
            if !near(cut_boxes[ci], cut_boxes[oi], 1e-6) || !cuts_it || !is_convex(&other.points) {
                continue;
            }
            let hole = &other.points;
            pieces = pieces
                .into_iter()
                .flat_map(|p| {
                    if !boxes_overlap(&p, hole) {
                        vec![p]
                    } else if ground {
                        subtract_convex_banded(&p, hole)
                    } else {
                        subtract_convex(&p, hole)
                    }
                })
                .collect();
        }
        polys.extend(pieces.into_iter().map(|p| CutPiece {
            points: p,
            ..c.clone()
        }));
    }
    polys
}

/// The key of an air gap layer: air, for where the thick line goes.
const AIR: (Option<ElementId>, u8) = (None, 250);

/// Turn cut polygons into fills plus outlines, leaving out outline segments
/// shared with a neighbouring cut so joined elements read as one mass.
pub fn clean_up_cuts(model: &Model, items: &mut Vec<PlanItem>) {
    clean_up_pieces(model, items, vec![]);
}

/// [`clean_up_cuts`], with layer pieces of layered elements besides the
/// `Pen::Cut` polygons in `items`. Each layer is filled with its own
/// material's hatch; the line between two layers of one element is a thin
/// one, and layers of the same material merge across joined walls.
pub fn clean_up_pieces(model: &Model, items: &mut Vec<PlanItem>, layers: Vec<CutPiece>) {
    clean_up_pieces_in(model, items, layers, false)
}

/// [`clean_up_pieces`] for a plan, or with `section` for a section: there
/// footings are filled with their material.
fn boxes_of(p: &[DVec2]) -> (DVec2, DVec2) {
    p.iter().fold((DVec2::MAX, DVec2::MIN), |(lo, hi), q| {
        (lo.min(*q), hi.max(*q))
    })
}

/// Do polygons `a` and `b` share a stretch of their outlines?
fn shares_edge(a: &[DVec2], b: &[DVec2], eps: f64) -> bool {
    let edges = |p: &[DVec2]| -> Vec<(DVec2, DVec2)> {
        (0..p.len()).map(|i| (p[i], p[(i + 1) % p.len()])).collect()
    };
    for (p, q) in edges(a) {
        let len = p.distance(q);
        if len < eps {
            continue;
        }
        let d = (q - p) / len;
        for (r, t) in edges(b) {
            if (r - p).perp_dot(d).abs() < eps && (t - p).perp_dot(d).abs() < eps {
                let (u, v) = ((r - p).dot(d), (t - p).dot(d));
                if u.max(v).min(len) - u.min(v).max(0.0) > eps {
                    return true;
                }
            }
        }
    }
    false
}

/// A point snapped to a grid.
type Grid = (i64, i64);

/// The outline round polygons that touch along their edges and do not
/// overlap, when it is one loop: each edge split where another's corner
/// lies on it, and the edges two of them share (run opposite ways)
/// dropped. None when the parts do not make one simple outline.
pub(crate) fn union_touching(parts: &[&[DVec2]], eps: f64) -> Option<Vec<DVec2>> {
    let area = |p: &[DVec2]| -> f64 {
        (0..p.len())
            .map(|i| p[i].perp_dot(p[(i + 1) % p.len()]))
            .sum::<f64>()
            / 2.0
    };
    let rings: Vec<Vec<DVec2>> = parts
        .iter()
        .map(|p| {
            let mut r = p.to_vec();
            if area(&r) < 0.0 {
                r.reverse();
            }
            r
        })
        .collect();
    let corners: Vec<DVec2> = rings.iter().flatten().copied().collect();
    let key = |p: DVec2| ((p.x / eps).round() as i64, (p.y / eps).round() as i64);
    let mut edges: Vec<(DVec2, DVec2)> = vec![];
    for r in &rings {
        for i in 0..r.len() {
            let (a, b) = (r[i], r[(i + 1) % r.len()]);
            let len = a.distance(b);
            if len < eps {
                continue;
            }
            let d = (b - a) / len;
            let mut cuts: Vec<f64> = corners
                .iter()
                .filter(|c| (**c - a).perp_dot(d).abs() < eps)
                .map(|c| (*c - a).dot(d))
                .filter(|t| *t > eps && *t < len - eps)
                .collect();
            cuts.sort_by(f64::total_cmp);
            let mut from = a;
            for t in cuts {
                let p = a + d * t;
                if p.distance(from) > eps {
                    edges.push((from, p));
                    from = p;
                }
            }
            edges.push((from, b));
        }
    }
    // shared edges run both ways: both go
    let mut count: std::collections::HashMap<(Grid, Grid), i32> = std::collections::HashMap::new();
    for (a, b) in &edges {
        *count.entry((key(*a), key(*b))).or_default() += 1;
    }
    let outer: Vec<(DVec2, DVec2)> = edges
        .into_iter()
        .filter(|(a, b)| !count.contains_key(&(key(*b), key(*a))))
        .collect();
    if outer.is_empty() {
        return None;
    }
    let mut next: std::collections::HashMap<(i64, i64), DVec2> = std::collections::HashMap::new();
    for (a, b) in &outer {
        if next.insert(key(*a), *b).is_some() {
            return None;
        }
    }
    let start = outer[0].0;
    let mut ring = vec![start];
    let mut at = outer[0].1;
    while key(at) != key(start) {
        if ring.len() > outer.len() {
            return None;
        }
        ring.push(at);
        at = *next.get(&key(at))?;
    }
    (ring.len() == outer.len()).then_some(ring)
}

pub fn clean_up_pieces_in(
    model: &Model,
    items: &mut Vec<PlanItem>,
    layers: Vec<CutPiece>,
    section: bool,
) {
    clean_up_pieces_with(model, items, layers, section, false)
}

/// [`clean_up_pieces_in`], with `one_wall` for a structural plan: every
/// wall's cut is one material there, so joined walls merge into one
/// outline with no line across where they meet.
fn clean_up_pieces_with(
    model: &Model,
    items: &mut Vec<PlanItem>,
    layers: Vec<CutPiece>,
    section: bool,
    one_wall: bool,
) {
    let mut cuts: Vec<CutPiece> = Vec::new();
    items.retain(|it| match it {
        PlanItem::Polygon {
            id,
            pen: Pen::Cut,
            points,
        } => {
            cuts.push(CutPiece {
                id: *id,
                points: points.clone(),
                layer: None,
            });
            false
        }
        _ => true,
    });
    cuts.extend(layers);
    let polys = resolve_cuts(model, cuts, section);
    let bb = |p: &[DVec2]| {
        p.iter().fold((DVec2::MAX, DVec2::MIN), |(lo, hi), q| {
            (lo.min(*q), hi.max(*q))
        })
    };
    let near = |a: (DVec2, DVec2), b: (DVec2, DVec2), eps: f64| {
        a.0.x <= b.1.x + eps && b.0.x <= a.1.x + eps && a.0.y <= b.1.y + eps && b.0.y <= a.1.y + eps
    };
    let keys: Vec<(Option<ElementId>, u8)> = polys
        .iter()
        .map(|p| {
            let k = p.key(model);
            let wall = matches!(
                model.get(p.id).map(|e| &e.kind),
                Some(creabim_core::ElementKind::Wall(_))
            );
            if one_wall && wall && k != AIR {
                (None, u8::MAX)
            } else {
                k
            }
        })
        .collect();
    // batt insulation of one layer cut in pieces (two ceilings meeting
    // over a wall, the wall's top running into the wool) filled as one,
    // so its loops run on over the joints (George 2026-11-25)
    let mut filled = vec![false; polys.len()];
    for (i, p) in polys.iter().enumerate() {
        if filled[i] {
            continue;
        }
        let hatch = p.hatch(model, section);
        let mut points = p.points.clone();
        if hatch == Hatch::Insulation {
            let layer = |q: &CutPiece| q.layer.map(|l| l.index);
            let mut group = vec![i];
            let mut k = 0;
            while k < group.len() {
                let g = group[k];
                for j in i + 1..polys.len() {
                    if filled[j]
                        || group.contains(&j)
                        || keys[j] != keys[i]
                        || layer(&polys[j]) != layer(p)
                        || polys[j].hatch(model, section) != hatch
                        || !near(boxes_of(&polys[g].points), boxes_of(&polys[j].points), 1e-4)
                    {
                        continue;
                    }
                    if shares_edge(&polys[g].points, &polys[j].points, 1e-4) {
                        group.push(j);
                    }
                }
                k += 1;
            }
            if group.len() > 1 {
                let parts: Vec<&[DVec2]> = group.iter().map(|&g| &polys[g].points[..]).collect();
                if let Some(one) = union_touching(&parts, 1e-5) {
                    points = one;
                    for &g in &group {
                        filled[g] = true;
                    }
                }
            }
        }
        filled[i] = true;
        items.push(PlanItem::Fill {
            id: p.id,
            points,
            hatch,
            rgb: p.colour(model, section),
        });
    }
    let edges = |pts: &Vec<DVec2>| -> Vec<(DVec2, DVec2)> {
        (0..pts.len())
            .map(|i| (pts[i], pts[(i + 1) % pts.len()]))
            .collect()
    };
    let eps = 1e-4;
    let boxes: Vec<_> = polys.iter().map(|p| bb(&p.points)).collect();
    for (i, piece) in polys.iter().enumerate() {
        let touching: Vec<usize> = (0..polys.len())
            .filter(|&j| near(boxes[i], boxes[j], 2.0 * eps))
            .collect();
        for (a, b) in edges(&piece.points) {
            let len = a.distance(b);
            if len < eps {
                continue;
            }
            let d = (b - a) / len;
            let on_line = |p: DVec2| (p - a).perp_dot(d).abs() < eps;
            // parts of this edge covered by another cut's edge: merged away
            // where it is the same material, a thin line between two layers
            // of one element (drawn by the first of the two)
            let mut covered: Vec<(f64, f64)> = Vec::new();
            let mut thin: Vec<(f64, f64, bool)> = Vec::new();
            for &j in &touching {
                let other = &polys[j];
                let same = keys[i] == keys[j];
                // a thick line only where cut material meets air
                // (PilonDesigner): an air gap counts as air, and where two
                // different materials meet, of one element or of two, the
                // line between them is thin
                if i == j || (!same && keys[j] == AIR) {
                    continue;
                }
                let merge = same || keys[i] == AIR;
                for (p, q) in edges(&other.points) {
                    if on_line(p) && on_line(q) {
                        let (tp, tq) = ((p - a).dot(d), (q - a).dot(d));
                        let (lo, hi) = (tp.min(tq).max(0.0), tp.max(tq).min(len));
                        if hi > lo + eps {
                            if merge {
                                covered.push((lo, hi));
                            } else {
                                thin.push((lo, hi, i < j));
                            }
                        }
                    }
                }
                // where two of the same overlap (footings lapped at a
                // corner), the edges running inside the other are merged
                // away too, leaving one outline round both
                if same {
                    covered.extend(inside_spans(a, b, &other.points, eps));
                }
            }
            // split the edge where any of that changes, and draw each part
            let mut ts: Vec<f64> = vec![0.0, len];
            for (lo, hi) in &covered {
                ts.extend([*lo, *hi]);
            }
            for (lo, hi, _) in &thin {
                ts.extend([*lo, *hi]);
            }
            ts.sort_by(|x, y| x.partial_cmp(y).unwrap());
            ts.dedup_by(|x, y| (*x - *y).abs() < eps);
            let mut run: Option<(f64, f64, Pen)> = None;
            let flush = |run: &mut Option<(f64, f64, Pen)>, items: &mut Vec<PlanItem>| {
                if let Some((t0, t1, pen)) = run.take() {
                    items.push(PlanItem::Line {
                        id: piece.id,
                        pen,
                        a: a + d * t0,
                        b: a + d * t1,
                    });
                }
            };
            for w in ts.windows(2) {
                let (t0, t1) = (w[0], w[1]);
                let m = (t0 + t1) / 2.0;
                let inside = |lo: f64, hi: f64| lo <= m && m <= hi;
                let pen = if covered.iter().any(|(lo, hi)| inside(*lo, *hi)) {
                    None
                } else {
                    let mine: Vec<bool> = thin
                        .iter()
                        .filter(|(lo, hi, _)| inside(*lo, *hi))
                        .map(|t| t.2)
                        .collect();
                    if mine.is_empty() {
                        Some(Pen::Cut)
                    } else if mine.iter().any(|f| *f) {
                        Some(Pen::Projection)
                    } else {
                        None
                    }
                };
                match (pen, &mut run) {
                    (Some(p), Some((_, end, rp))) if *rp == p && (*end - t0).abs() < eps => {
                        *end = t1
                    }
                    (Some(p), _) => {
                        flush(&mut run, items);
                        run = Some((t0, t1, p));
                    }
                    (None, _) => flush(&mut run, items),
                }
            }
            flush(&mut run, items);
        }
    }
}

/// Where a dimension's text sits and how its witness lines are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DimPlace {
    /// The text moved from over the middle of its line, drawing units.
    pub shift: DVec2,
    /// Moved by hand (a dragged text): the automatic placing of short
    /// stretches leaves it alone.
    pub manual: bool,
    /// A thin leader from the middle of the line to the moved text.
    pub leader: bool,
    /// Witness lines only by the dimension line, not back to what is
    /// measured.
    pub short: bool,
    /// The text on a patch of paper, covering what is drawn under it.
    pub back: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlanItem {
    Polygon {
        id: ElementId,
        pen: Pen,
        points: Vec<DVec2>,
    },
    /// Fill of cut material, with no outline (outlines are `Pen::Cut` lines).
    Fill {
        id: ElementId,
        points: Vec<DVec2>,
        hatch: Hatch,
        /// The colour of the material cut here (its layer's, for layered
        /// elements), for the colour drawing styles (MU32).
        rgb: Option<[f32; 3]>,
    },
    Line {
        id: ElementId,
        pen: Pen,
        a: DVec2,
        b: DVec2,
    },
    /// Counter-clockwise arc from `start` to `end` angle, radians.
    Arc {
        id: ElementId,
        pen: Pen,
        center: DVec2,
        radius: f64,
        start: f64,
        end: f64,
    },
    Label {
        id: ElementId,
        at: DVec2,
        text: String,
    },
    /// Text at a fixed paper size, `size` mm tall, in a drawing of `mm`
    /// units per paper millimetre (1 for paper itself, as on a schedule).
    Text {
        id: ElementId,
        at: DVec2,
        text: String,
        size: f64,
        align: crate::paper::Anchor,
        mm: f64,
        /// sRGB 0..1; none is the view's ink.
        rgb: Option<[f32; 3]>,
        /// Semibold.
        bold: bool,
    },
    /// A dimension from `a` to `b`, its line `offset` to the left. A
    /// horizontal or vertical one measures from `a` to `b` square across,
    /// so `b` is where the true second point `witness[1]` lands on that
    /// line, and its witness lines start from `witness`.
    Dimension {
        id: ElementId,
        a: DVec2,
        b: DVec2,
        offset: f64,
        text: String,
        look: crate::annotation::TextLook,
        witness: Option<[DVec2; 2]>,
        /// Stretches of the dimension line left out where tags and notes
        /// sit across it, as fractions of the way from `a` to `b`.
        breaks: Vec<[f64; 2]>,
        /// Text under the line (STN 01 3420: a dimension across a whole
        /// door or window has its width above and its height, and a
        /// window's sill in brackets, below).
        below: Option<String>,
        /// Where its text sits and how its witness lines are drawn.
        place: DimPlace,
    },
    /// A tag: `text` in a bubble at `at`, with a leader back to `anchor`.
    Tag {
        id: ElementId,
        anchor: DVec2,
        at: DVec2,
        /// A bend in the leader.
        elbow: Option<DVec2>,
        text: String,
        round: bool,
        look: crate::annotation::TextLook,
    },
    /// An angular, radial, diameter or arc-length dimension, or a note.
    Annotation {
        id: ElementId,
        item: Box<crate::annotation::Annotation>,
    },
    /// An element hidden inside another, washed over what hides it in its
    /// material colour (sRGB, 0..1), see-through: for detailing.
    Tint {
        id: ElementId,
        points: Vec<DVec2>,
        rgb: [f32; 3],
    },
    /// A room's floor: washed in `rgb` when the view colours its rooms,
    /// otherwise not drawn; picked by a click inside it that finds nothing
    /// else.
    Room {
        id: ElementId,
        points: Vec<DVec2>,
        rgb: Option<[f32; 3]>,
    },
    /// The footprint of something below the cut, filled flat in a light
    /// tint (sRGB, 0..1) under the linework, with no outline.
    Shade {
        id: ElementId,
        points: Vec<DVec2>,
        rgb: [f32; 3],
    },
    /// A polyline in a colour of its own, dashed or solid, over what it
    /// lies on: the fire-dangerous space in front of an opening.
    Curve {
        id: ElementId,
        points: Vec<DVec2>,
        rgb: [f32; 3],
        dashed: bool,
    },
    /// Shaded surfaces as an image spanning `lo`..`hi`, drawn only inside `clip`.
    Image {
        id: ElementId,
        raster: std::sync::Arc<crate::raster::Raster>,
        lo: DVec2,
        hi: DVec2,
        clip: Option<(DVec2, DVec2)>,
    },
}

/// Is a label short enough to sit in a bubble, as grid and section marks
/// do (level datums and longer text are plain)?
pub fn is_bubble(text: &str) -> bool {
    // a roof pitch (35°, 1:60) is short too, but plain
    !text.contains([' ', '°', ':']) && (1..=5).contains(&text.chars().count())
}

/// A bubble's radius for `text`, in units of the bubble for one or two
/// characters.
pub fn bubble_scale(text: &str) -> f64 {
    (text.chars().count() as f64 * 0.38).max(1.0)
}

impl PlanItem {
    pub fn id(&self) -> ElementId {
        match self {
            PlanItem::Polygon { id, .. }
            | PlanItem::Fill { id, .. }
            | PlanItem::Line { id, .. }
            | PlanItem::Arc { id, .. }
            | PlanItem::Label { id, .. }
            | PlanItem::Text { id, .. }
            | PlanItem::Dimension { id, .. }
            | PlanItem::Tag { id, .. }
            | PlanItem::Annotation { id, .. }
            | PlanItem::Tint { id, .. }
            | PlanItem::Room { id, .. }
            | PlanItem::Shade { id, .. }
            | PlanItem::Curve { id, .. }
            | PlanItem::Image { id, .. } => *id,
        }
    }

    /// Corners of what the item covers, in drawing units (none for guides,
    /// which run on without end).
    pub fn extent(&self) -> Vec<DVec2> {
        let mut pts = Vec::new();
        match self {
            PlanItem::Polygon { points, .. }
            | PlanItem::Fill { points, .. }
            | PlanItem::Tint { points, .. }
            | PlanItem::Room { points, .. }
            | PlanItem::Shade { points, .. }
            | PlanItem::Curve { points, .. } => pts.extend(points.iter().copied()),
            PlanItem::Line {
                pen: Pen::Guide, ..
            } => {}
            PlanItem::Line { a, b, .. } => pts.extend([*a, *b]),
            PlanItem::Arc { center, radius, .. } => pts.extend([
                *center - DVec2::splat(*radius),
                *center + DVec2::splat(*radius),
            ]),
            PlanItem::Label { at, text, .. } => {
                // rough text extent at 1:100 (3 mm type)
                let half = DVec2::new(text.chars().count() as f64 * 0.09 + 0.2, 0.25);
                pts.extend([*at - half, *at + half]);
            }
            PlanItem::Text {
                at,
                text,
                size,
                align,
                mm,
                ..
            } => {
                // the text's own box, at its size on paper
                let w = crate::paper::text_width(text, *size) * mm;
                let h = size * mm;
                let x0 = match align {
                    crate::paper::Anchor::Start => at.x,
                    crate::paper::Anchor::Middle => at.x - w / 2.0,
                    crate::paper::Anchor::End => at.x - w,
                };
                pts.extend([
                    DVec2::new(x0, at.y - h / 2.0),
                    DVec2::new(x0 + w, at.y + h / 2.0),
                ]);
            }
            PlanItem::Dimension { a, b, offset, .. } => {
                let n = (*b - *a).normalize_or_zero().perp() * *offset;
                pts.extend([*a, *b, *a + n, *b + n]);
            }
            PlanItem::Tag {
                anchor, at, elbow, ..
            } => {
                pts.extend([*anchor, *at - DVec2::splat(0.3), *at + DVec2::splat(0.3)]);
                pts.extend(*elbow);
            }
            PlanItem::Annotation { item, .. } => pts.extend(item.extent()),
            PlanItem::Image { lo, hi, clip, .. } => match clip {
                Some((clo, chi)) => pts.extend([lo.max(*clo), hi.min(*chi)]),
                None => pts.extend([*lo, *hi]),
            },
        }
        pts
    }

    pub fn bounds(&self) -> Option<(DVec2, DVec2)> {
        let pts = self.extent();
        let first = *pts.first()?;
        Some(
            pts.iter()
                .fold((first, first), |(lo, hi), p| (lo.min(*p), hi.max(*p))),
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlanView {
    pub items: Vec<PlanItem>,
}

impl PlanView {
    pub fn bounds(&self) -> Option<(DVec2, DVec2)> {
        let pts: Vec<DVec2> = self.items.iter().flat_map(|it| it.extent()).collect();
        let first = *pts.first()?;
        Some(
            pts.iter()
                .fold((first, first), |(lo, hi), p| (lo.min(*p), hi.max(*p))),
        )
    }

    /// Render as SVG at a drawing scale, e.g. `100.0` for 1:100. Units are millimetres on paper.
    pub fn to_svg(&self, scale: f64) -> String {
        let (lo, hi) = self.bounds().unwrap_or((DVec2::ZERO, DVec2::ONE));
        let margin = DVec2::splat(1.0);
        let (lo, hi) = (lo - margin, hi + margin);
        let map = crate::paper::PaperMap {
            model: lo,
            paper: DVec2::ZERO,
            scale,
        };
        let mut svg = crate::paper::SvgCanvas::new((hi - lo) * map.k());
        crate::paper::draw(&mut svg, self, &map);
        svg.finish()
    }

    /// [`PlanView::to_svg`] with a view's looks: its pens, styles and
    /// overrides, as a sheet prints it.
    pub fn to_svg_looks(&self, scale: f64, looks: &crate::looks::Looks) -> String {
        let (lo, hi) = self.bounds().unwrap_or((DVec2::ZERO, DVec2::ONE));
        let margin = DVec2::splat(1.0);
        let (lo, hi) = (lo - margin, hi + margin);
        let map = crate::paper::PaperMap {
            model: lo,
            paper: DVec2::ZERO,
            scale,
        };
        let mut svg = crate::paper::SvgCanvas::new((hi - lo) * map.k());
        crate::paper::draw_looks(&mut svg, self, &map, looks);
        svg.finish()
    }
}

/// Generate the plan of one level, cut `cut_height` metres above it.
/// A sketch's lines and arcs in a drawing. `project` takes a point in the
/// sketch's model coordinates (relative to its level) to drawing x and y,
/// plus depth. Arcs on a plane facing the drawing stay arcs; seen at an
/// angle they are drawn in short pieces.
pub fn sketch_items(
    id: ElementId,
    sk: &creabim_core::Sketch,
    project: &dyn Fn(DVec3) -> DVec3,
) -> Vec<PlanItem> {
    let at = |p: DVec2| project(sk.plane.to_model(p)).truncate();
    let o = at(DVec2::ZERO);
    let (x, y) = (at(DVec2::X) - o, at(DVec2::Y) - o);
    let facing =
        (x.length() - 1.0).abs() < 1e-6 && (y.length() - 1.0).abs() < 1e-6 && x.dot(y).abs() < 1e-6;
    let mut out = Vec::new();
    // a closed sketch is a face: a light wash inside it (A7-1)
    let outline = sk.outline();
    if sk.closed
        && sk.net.is_none()
        && outline.len() >= 3
        && !creabim_core::polygon::self_intersects(&outline, true)
    {
        out.push(PlanItem::Tint {
            id,
            points: outline.iter().map(|p| at(*p)).collect(),
            rgb: [0.16, 0.27, 0.55],
        });
    }
    for c in sk.curves() {
        match c {
            Curve::Arc {
                center,
                radius,
                start,
                sweep,
            } if facing => {
                // seen from behind, the arc turns the other way
                let flip = x.perp_dot(y) < 0.0;
                let a0 = if flip {
                    x.to_angle() - start
                } else {
                    x.to_angle() + start
                };
                let sweep = if flip { -sweep } else { sweep };
                let (start, end) = if sweep < 0.0 {
                    (a0 + sweep, a0)
                } else {
                    (a0, a0 + sweep)
                };
                out.push(PlanItem::Arc {
                    id,
                    pen: Pen::Symbol,
                    center: at(center),
                    radius,
                    start,
                    end,
                });
            }
            _ => {
                for w in c.points().windows(2) {
                    let (a, b) = (at(w[0]), at(w[1]));
                    if a.distance(b) > 1e-6 {
                        out.push(PlanItem::Line {
                            id,
                            pen: Pen::Symbol,
                            a,
                            b,
                        });
                    }
                }
            }
        }
    }
    out
}

/// Does the plan of `level`, cut `cut_height` above it, pass through
/// footing `f`? A foundation level under the ground floor shows the
/// footings its plane cuts, whatever level their walls stand on.
pub(crate) fn footing_cut_at(
    model: &Model,
    f: &creabim_core::Footing,
    level: ElementId,
    cut_height: f64,
) -> bool {
    let Some(l) = model.level(level) else {
        return false;
    };
    let z = l.elevation + cut_height;
    let top = f.top(model);
    top - f.thickness < z && z < top
}

/// Is the plan of `level` cut `cut_height` above it a foundation plan:
/// does its plane pass through a footing?
pub fn is_foundation_plan(model: &Model, level: ElementId, cut_height: f64) -> bool {
    model.iter().any(|e| match &e.kind {
        ElementKind::Footing(f) => footing_cut_at(model, f, level, cut_height),
        _ => false,
    })
}

/// What a foundation plan adds over the footings its plane cuts: a heavy
/// dashed line where their undersides step. The masonry standing on them
/// is drawn by [`above_cut_walls`], as all above the cut.
fn foundation_lines(model: &Model, level: ElementId, cut_height: f64) -> Vec<PlanItem> {
    let mut out = vec![];
    let cut: Vec<(ElementId, &creabim_core::Footing)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Footing(f) if footing_cut_at(model, f, level, cut_height) => {
                Some((e.id, f))
            }
            _ => None,
        })
        .collect();
    if cut.is_empty() {
        return out;
    }
    for (id, a, b) in creabim_core::footing::steps(model) {
        if cut.iter().any(|(c, _)| *c == id) {
            out.push(PlanItem::Line {
                id,
                pen: Pen::HiddenCut,
                a,
                b,
            });
        }
    }
    out
}

/// The elevation of the first level over `z`; none above is infinitely high.
pub fn next_level_above(model: &Model, z: f64) -> f64 {
    model
        .levels()
        .map(|(_, l)| l.elevation)
        .filter(|e| *e > z + 1e-6)
        .fold(f64::INFINITY, f64::min)
}

/// The roofs of other levels whose eaves lie between the cut and the top
/// of the range (George 2026-11-21: a storey's plan with its top raised
/// to the roof shows its outline): eaves, ridges and hips as a thin line
/// of a dash and two dots, as all above the cut. A roof of the plan's own
/// level is drawn with its other members.
fn above_cut_roofs(model: &Model, level: ElementId, cut_z: f64, top_z: f64) -> Vec<PlanItem> {
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Roof(r) = &e.kind else {
            continue;
        };
        if r.level == level {
            continue;
        }
        let low = crate::members::roof_low(model, r);
        if !(low > cut_z + 1e-3 && low < top_z - 1e-3) {
            continue;
        }
        for it in crate::members::member_plan(model, e, -1e4) {
            match it {
                PlanItem::Line { id, a, b, .. } => out.push(PlanItem::Line {
                    id,
                    pen: Pen::Overhead,
                    a,
                    b,
                }),
                PlanItem::Polygon { id, points, .. } => out.push(PlanItem::Polygon {
                    id,
                    pen: Pen::Overhead,
                    points,
                }),
                _ => {}
            }
        }
    }
    out
}

/// The columns of other levels reaching into the plan's range (George
/// 2026-11-24: a column of the ground storey running down into the
/// footings): one standing above the cut with its foot under the top of
/// the range as all above the cut, a thin line of a dash and two dots;
/// one the plane passes through cut, as the plan's own. A foundation
/// plan draws them all as above its cut, and only their load-bearing
/// part (not the insulation boards of a wrap).
fn other_level_columns(
    model: &Model,
    level: ElementId,
    cut_z: f64,
    top_z: f64,
    core_only: bool,
) -> Vec<PlanItem> {
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Column(c) = &e.kind else {
            continue;
        };
        if core_only && c.piece.is_some() {
            continue;
        }
        let (z0, z1) = c.span(model);
        if z0 >= top_z - 1e-3 || z1 <= cut_z + 1e-3 {
            continue;
        }
        // one the plane cuts is drawn cut with the plan's own (its pieces
        // of insulation and plaster joined, hatched); drawing it again
        // here put a grey box with the pieces' edges over it (George
        // 2026-11-24). Only a foundation plan draws another storey's
        // column through its cut as above it, its core.
        if z0 < cut_z && (!core_only || c.level == level) {
            continue;
        }
        // its own level's columns standing above the cut (porch posts on
        // the foundation level, their base over the footings) are seen
        // from above too
        let points = c.outline();
        for i in 0..points.len() {
            out.push(PlanItem::Line {
                id: e.id,
                pen: Pen::Overhead,
                a: points[i],
                b: points[(i + 1) % points.len()],
            });
        }
    }
    out
}

/// Every wall standing above the cut whose base is under the top of the
/// range (a plan's "Horná hrana rezu"), of whatever level: its outline as
/// a thin line of a dash and two dots, as all above the cut (George
/// 2026-11-20). A foundation plan draws only the load-bearing layer of
/// each (the DT blocks of the masonry on the footings, without its
/// membrane and insulation); an end meeting no other such wall is closed.
fn above_cut_walls(
    model: &Model,
    cut_z: f64,
    top_z: f64,
    core_only: bool,
    joins: &std::collections::HashMap<ElementId, creabim_core::joins::WallEnds>,
) -> Vec<PlanItem> {
    let mut out = vec![];
    let walls: Vec<(ElementId, &creabim_core::Wall)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Wall(w) => {
                let base = creabim_core::walls::base_z(model, w);
                (base > cut_z + 1e-3 && base < top_z - 1e-3 && w.length() > 1e-6)
                    .then_some((e.id, w))
            }
            _ => None,
        })
        .collect();
    for (wid, w) in &walls {
        let ends = joins.get(wid).cloned().unwrap_or_default();
        let (s0, s1) = if core_only {
            creabim_core::walls::core_span(model, w)
        } else {
            (-w.thickness / 2.0, w.thickness / 2.0)
        };
        if w.is_arc() {
            let len = w.length();
            let at = |u: f64, s: f64| {
                if u <= 1e-9 || u >= len - 1e-9 {
                    w.place_end(u > 1e-9, ends.place(w, u, s, false), s)
                } else {
                    w.place(u, s)
                }
            };
            out.push(PlanItem::Polygon {
                id: *wid,
                pen: Pen::Overhead,
                points: wall_band(w, &at, 0.0, len, s0.min(s1), s0.max(s1)),
            });
            continue;
        }
        let len = w.length();
        let d = (w.end - w.start) / len;
        let n = d.perp();
        let at = |u: f64, s: f64| w.start + d * ends.place(w, u, s, true) + n * s;
        let (p0, p1, q0, q1) = (at(0.0, s0), at(len, s0), at(0.0, s1), at(len, s1));
        // where another wall ends against this one's side (a T), that
        // side's line stops for it, so the two join as one outline
        // (George 2026-11-23: masonry of DT blocks only against the
        // masonry with its insulation)
        let gaps = |side: f64| -> Vec<(f64, f64)> {
            let a0 = w.start + n * side;
            let mut g = vec![];
            for (o, ow) in &walls {
                if o == wid || ow.is_arc() {
                    continue;
                }
                let ol = ow.length();
                let d2 = (ow.end - ow.start) / ol;
                if d.perp_dot(d2).abs() < 0.2 {
                    continue;
                }
                for (end, away) in [(ow.start, d2), (ow.end, -d2)] {
                    let rel = end - w.start;
                    let (u, v) = (rel.dot(d), rel.dot(n));
                    if u < 1e-3 || u > len - 1e-3 || v.abs() > w.thickness / 2.0 + 0.02 {
                        continue;
                    }
                    // only the side the other wall stands on
                    if away.dot(n) * (side - (s0 + s1) / 2.0) <= 0.0 {
                        continue;
                    }
                    let (c0, c1) = if core_only {
                        creabim_core::walls::core_span(model, ow)
                    } else {
                        (-ow.thickness / 2.0, ow.thickness / 2.0)
                    };
                    let n2 = d2.perp();
                    let cut = |c: f64| {
                        let p = ow.start + n2 * c;
                        (p - a0).perp_dot(d2) / d.perp_dot(d2)
                    };
                    let (x0, x1) = (cut(c0), cut(c1));
                    g.push((x0.min(x1), x0.max(x1)));
                }
            }
            g
        };
        let mut line = |a: DVec2, b: DVec2, side: f64| {
            let (ua, ub) = ((a - w.start).dot(d), (b - w.start).dot(d));
            let mut from = ua;
            let mut gs = gaps(side);
            gs.sort_by(|x, y| x.0.total_cmp(&y.0));
            let on = |u: f64| a + (b - a) * ((u - ua) / (ub - ua));
            for (g0, g1) in gs {
                if g1 <= from || g0 >= ub {
                    continue;
                }
                if g0 > from + 1e-6 {
                    out.push(PlanItem::Line {
                        id: *wid,
                        pen: Pen::Overhead,
                        a: on(from),
                        b: on(g0),
                    });
                }
                from = from.max(g1);
            }
            if ub > from + 1e-6 {
                out.push(PlanItem::Line {
                    id: *wid,
                    pen: Pen::Overhead,
                    a: on(from),
                    b: on(ub),
                });
            }
        };
        line(p0, p1, s0);
        line(q0, q1, s1);
        let mut line = |a: DVec2, b: DVec2| {
            out.push(PlanItem::Line {
                id: *wid,
                pen: Pen::Overhead,
                a,
                b,
            })
        };
        for (p, a, b) in [(w.start, p0, q0), (w.end, p1, q1)] {
            let joined = walls.iter().any(|(o, ow)| {
                o != wid
                    && creabim_core::footing::segment_distance(p, ow.start, ow.end)
                        <= (ow.thickness + w.thickness) / 2.0
            });
            if !joined {
                line(a, b);
            }
        }
    }
    out
}

pub fn plan_view(model: &Model, level: ElementId, cut_height: f64) -> PlanView {
    plan_view_with(
        model,
        level,
        cut_height,
        &creabim_core::PlanGraphics::default(),
    )
}

/// [`plan_view`] with a view's graphics: what lies wholly below the cut
/// tinted, and the stair from the level below seen through its well, as
/// far down as the view depth.
pub fn plan_view_with(
    model: &Model,
    level: ElementId,
    cut_height: f64,
    graphics: &creabim_core::PlanGraphics,
) -> PlanView {
    let own = level;
    let (level, cut_height) = storey_cut(model, level, cut_height);
    // an empty storey drawn as the one under it, cut higher: the ground
    // (terrain, paving, plants) stays on its own level's plan and the
    // site plan (a downloaded terrain showed on 2. NP)
    let borrowed = level != own;
    let mut items = Vec::new();
    // tinted footprints, drawn under everything else
    let mut shades: Vec<PlanItem> = Vec::new();
    // dashed over the cut fills: openings above the cut
    let mut overhead: Vec<PlanItem> = Vec::new();
    let joins = creabim_core::wall_joins(model);
    // the top of the range: what starts above it is not drawn
    let range_top = {
        let own_z = model.level(own).map_or(0.0, |l| l.elevation);
        graphics
            .range
            .top
            .map_or_else(|| next_level_above(model, own_z), |t| t.z(model, own_z))
    };
    let cuts = crate::scene::all_cuts(model);
    let cutters: std::collections::HashSet<ElementId> = cuts.iter().map(|c| c.1).collect();
    let targets: std::collections::HashSet<ElementId> = cuts.iter().map(|c| c.0).collect();
    // elements whose plan comes from slicing their final 3D shape
    let mut sliced: Vec<ElementId> = Vec::new();
    // windows and doors drawn as cut off the cut plane: the wall, its
    // start, direction and the opening's span along it and the wall's
    // half thickness (a sliced wall's outline is cut back there too)
    let mut forced: Vec<(ElementId, DVec2, DVec2, f64, f64, f64)> = Vec::new();
    // the door openings cut: where two floors meet under a door
    let mut doors: Vec<Vec<DVec2>> = Vec::new();
    // the layers of layered walls, cut
    let mut layers: Vec<CutPiece> = Vec::new();
    for e in model.iter() {
        if cutters.contains(&e.id) || (borrowed && matches!(e.kind, ElementKind::Site(_))) {
            continue;
        }
        let on_level = match &e.kind {
            ElementKind::Wall(w) => wall_rise(model, w, level, cut_height).is_some(),
            ElementKind::Slab(s) => s.level == level,
            // an imported mesh with no level of its own is on the level its
            // base stands on (MU33: a Blender import showed in no plan)
            ElementKind::FreeMesh(f) => match f.level {
                Some(l) => l == level,
                // no level of its own (an import): in every plan whose cut
                // or range below it reaches it (MU36: it was on only one)
                None => {
                    let z0 = model.level(level).map_or(0.0, |l| l.elevation);
                    let zs = f
                        .mesh
                        .vertices
                        .iter()
                        .map(|v| f.to_model(*v).z + f.origin.z);
                    let (lo, hi) = zs.fold((f64::MAX, f64::MIN), |(a, b), z| (a.min(z), b.max(z)));
                    // under the cut and not wholly below the floor's slab
                    lo < z0 + cut_height && hi > z0 - 0.5
                }
            },
            ElementKind::Footing(f) => f.plan_level(model) == level,
            k => creabim_core::members::level_of(k) == Some(level),
        };
        if on_level && (targets.contains(&e.id) || matches!(e.kind, ElementKind::FreeMesh(_))) {
            sliced.push(e.id);
            if !matches!(e.kind, ElementKind::Wall(_)) {
                continue;
            }
        }
        match &e.kind {
            ElementKind::Grid(g) => {
                items.push(PlanItem::Line {
                    id: e.id,
                    pen: Pen::Grid,
                    a: g.start,
                    b: g.end,
                });
                let dir = (g.end - g.start).normalize_or_zero();
                items.push(PlanItem::Label {
                    id: e.id,
                    at: g.end + dir * 0.6,
                    text: creabim_core::grids::display_label(model, g),
                });
            }
            ElementKind::Wall(w) if wall_rise(model, w, level, cut_height).is_some() => {
                // how far above the wall's own level this plan's lies
                let rise = wall_rise(model, w, level, cut_height).unwrap_or(0.0);
                let d = w.direction();
                let n = DVec2::new(-d.y, d.x);
                let t = w.thickness / 2.0;
                let ends = joins.get(&e.id).cloned().unwrap_or_default();
                // wall-local point; the wall's own ends and the sides of the
                // gaps crossing walls leave follow its joins
                // a curved wall is laid round its arc, its ends square
                let at = |u: f64, s: f64| {
                    if w.is_arc() {
                        // its ends as its joins put them (see Wall::place_end)
                        if u <= 1e-9 || u >= w.length() - 1e-9 {
                            w.place_end(u > 1e-9, ends.place(w, u, s, false), s)
                        } else {
                            w.place(u, s)
                        }
                    } else {
                        w.start + d * ends.place(w, u, s, false) + n * s
                    }
                };
                let cut_v = cut_height + rise - w.base_offset;
                let top_v = wall_top_height(model, w);
                let asm_of = creabim_core::assembly_of(model, e).map(|x| x.1);
                if cut_v <= 0.0 || cut_v >= top_v {
                    // wall entirely below or above the cut: show its outline in projection
                    if cut_v >= top_v {
                        items.push(PlanItem::Polygon {
                            id: e.id,
                            pen: Pen::Projection,
                            points: if w.is_arc() {
                                wall_band(w, &at, 0.0, w.length(), -t, t)
                            } else {
                                ends.footprint(w).to_vec()
                            },
                        });
                    }
                    continue;
                }
                if let Some(g) = &w.curtain {
                    // mullions and panels, and the doors in them
                    let mut hosted: Vec<(ElementId, &creabim_core::Opening)> = vec![];
                    for oid in model.dependents(e.id) {
                        if let Some(ElementKind::Opening(o)) = model.get(oid).map(|x| &x.kind) {
                            hosted.push((oid, o));
                        }
                    }
                    let openings: Vec<&creabim_core::Opening> =
                        hosted.iter().map(|h| h.1).collect();
                    items.extend(crate::curtain::curtain_plan(e.id, w, g, &openings, cut_v));
                    let d = w.direction();
                    let n = DVec2::new(-d.y, d.x);
                    let plain = |u: f64, s: f64| w.start + d * u + n * s;
                    for (oid, o) in hosted {
                        if let Some(c) = Cut::of(o, w) {
                            if cut_v > c.v0 && cut_v < c.v1 {
                                items.extend(opening_symbol(oid, o, w, asm_of, &c, cut_v, &plain));
                            }
                        }
                    }
                    continue;
                }
                let mut gaps: Vec<(f64, f64)> = Vec::new();
                // the windows cut: their edges and their frames' depths
                let mut windows: Vec<(f64, f64, (f64, f64), f64)> = Vec::new();
                for oid in model.dependents(e.id) {
                    let Some(oe) = model.get(oid) else { continue };
                    let ElementKind::Opening(o) = &oe.kind else {
                        continue;
                    };
                    let Some(c) = Cut::of(o, w) else { continue };
                    // a window or door is always drawn as cut, its sill
                    // over the cut plane or its head under it (George
                    // 2026-11-21): through its middle then
                    let joinery = matches!(o.kind, OpeningKind::Window | OpeningKind::Door);
                    let cut_v = if joinery && !(cut_v > c.v0 && cut_v < c.v1) && c.v1 > c.v0 {
                        forced.push((e.id, w.start, w.direction(), c.u0, c.u1, t));
                        (c.v0 + c.v1) / 2.0
                    } else {
                        cut_v
                    };
                    if cut_v > c.v0 && cut_v < c.v1 {
                        gaps.push((c.u0, c.u1));
                        if o.kind == OpeningKind::Door {
                            doors.push(vec![at(c.u0, -t), at(c.u1, -t), at(c.u1, t), at(c.u0, t)]);
                        }
                        if creabim_core::joinery::wraps_reveal(o) {
                            let frame = o.frame.fitted(c.u1 - c.u0, c.v1 - c.v0, w.thickness);
                            windows.push((
                                c.u0,
                                c.u1,
                                o.frame_span(w, asm_of, frame.depth),
                                o.reveal
                                    .unwrap_or(creabim_core::assembly::REVEAL_INSULATION),
                            ));
                        }
                        items.extend(opening_symbol(oid, o, w, asm_of, &c, cut_v, &at));
                    } else if c.v0 >= cut_v || o.kind == OpeningKind::Hole {
                        // wholly above the cut (a duct hole, a high
                        // window): its outline in the wall, a thin line of
                        // a dash and two dots, as all above the cut; a
                        // penetration wholly below it, thin dashed, as all
                        // hidden under the cut; a penetration's cross with it
                        let pen = if c.v0 >= cut_v {
                            Pen::Overhead
                        } else {
                            Pen::Hidden
                        };
                        let corners = [at(c.u0, -t), at(c.u1, -t), at(c.u1, t), at(c.u0, t)];
                        for i in 0..4 {
                            overhead.push(PlanItem::Line {
                                id: oid,
                                pen,
                                a: corners[i],
                                b: corners[(i + 1) % 4],
                            });
                        }
                        if o.is_penetration() {
                            for (a, b) in [(0, 2), (1, 3)] {
                                overhead.push(PlanItem::Line {
                                    id: oid,
                                    pen,
                                    a: corners[a],
                                    b: corners[b],
                                });
                            }
                        }
                    }
                }
                // a curtain wall set in it leaves its own gap, and draws
                // itself; a screen wall's holes leave theirs
                let screen = creabim_core::screen::screen_openings(model, e.id);
                let voids = creabim_core::curtain::voids_in(model, e.id);
                for o in voids.into_iter().map(|v| v.1).chain(screen) {
                    if let Some(c) = Cut::of(&o, w) {
                        if cut_v > c.v0 && cut_v < c.v1 {
                            gaps.push((c.u0, c.u1));
                        }
                    }
                }
                gaps.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                // the stretches between crossing walls, less the openings
                let mut pieces = Vec::new();
                for (lo, hi) in ends.solid_spans(w) {
                    let mut u = lo;
                    for &(g0, g1) in &gaps {
                        if g1 <= u || g0 >= hi {
                            continue;
                        }
                        if g0 > u + 1e-9 {
                            pieces.push((u, g0));
                        }
                        u = u.max(g1);
                    }
                    if hi > u + 1e-9 {
                        pieces.push((u, hi));
                    }
                }
                if let Some((_, asm)) = creabim_core::assembly_of(model, e) {
                    // each layer between its own ends: a core runs on at a T
                    for band in creabim_core::assembly::wall_bands(asm, w) {
                        // a structural plan: the masonry or concrete only,
                        // no insulation, render or plaster (George
                        // 2026-11-24)
                        if graphics.structure_only
                            && band.function != creabim_core::LayerFunction::Structure
                        {
                            continue;
                        }
                        let at = |u: f64, s: f64| {
                            if w.is_arc() {
                                w.place(u, s)
                            } else {
                                w.start + d * ends.place_layer(w, u, s, band.core, band.ext) + n * s
                            }
                        };
                        for &(a, b) in &pieces {
                            layers.push(CutPiece {
                                id: e.id,
                                points: wall_band(w, &at, a, b, band.s0, band.s1),
                                layer: Some(crate::scene::LayerPart::of(&band)),
                            });
                        }
                    }
                    // the layers turned into the windows' reveals
                    let bands = creabim_core::assembly::wall_bands(asm, w);
                    for &(u0, u1, span, lap) in &windows {
                        if graphics.structure_only {
                            break;
                        }
                        for l in creabim_core::assembly::reveal_wraps(asm, w, span, lap) {
                            let Some(band) = bands.get(l.index) else {
                                continue;
                            };
                            // not wider than half the window
                            if l.under + l.width > (u1 - u0) / 2.0 {
                                continue;
                            }
                            for (edge, into) in [(u0, 1.0), (u1, -1.0)] {
                                layers.push(CutPiece {
                                    id: e.id,
                                    points: l
                                        .shape()
                                        .into_iter()
                                        .map(|(x, s)| w.place(edge + into * x, s))
                                        .collect(),
                                    layer: Some(crate::scene::LayerPart::of(band)),
                                });
                            }
                        }
                    }
                    continue;
                }
                for (a, b) in pieces {
                    items.push(PlanItem::Polygon {
                        id: e.id,
                        pen: Pen::Cut,
                        points: wall_band(w, &at, a, b, -t, t),
                    });
                }
            }
            ElementKind::Slab(s) if s.level == level => {
                // cut when the plan plane passes through it, else seen below
                let bottom = s.top_offset - s.thickness;
                let cut = bottom < cut_height && cut_height < s.top_offset;
                let holes = creabim_core::slab::holes_of(model, e);
                // layers whose edges were moved by hand (A7-10): counted
                // from the top, each with its depth below the slab's top
                let layers: Vec<f64> = creabim_core::assembly::assembly_of(model, e)
                    .map(|(_, a)| a.layers.iter().map(|l| l.thickness).collect())
                    .unwrap_or_default();
                let edited: std::collections::BTreeSet<usize> = s
                    .layer_edges
                    .iter()
                    .map(|l| l.layer)
                    .filter(|l| *l < layers.len())
                    .collect();
                if cut {
                    // the layer the plan plane passes through
                    let mut top = s.top_offset;
                    let mut at = None;
                    for (k, t) in layers.iter().enumerate() {
                        if cut_height <= top && cut_height > top - t {
                            at = Some(k);
                        }
                        top -= t;
                    }
                    let outline = match at {
                        Some(k) if edited.contains(&k) => s.layer_outline(k),
                        _ => s.outline(),
                    };
                    // one outline with its holes bridged in, so the fill leaves them open
                    items.insert(
                        0,
                        PlanItem::Polygon {
                            id: e.id,
                            pen: Pen::Cut,
                            points: creabim_core::polygon::bridged(&outline, &holes),
                        },
                    );
                } else {
                    // seen from above: the slab's own edge where any layer
                    // still follows it, and each edited layer's edge
                    if edited.len() < layers.len().max(1) {
                        items.insert(
                            0,
                            PlanItem::Polygon {
                                id: e.id,
                                pen: Pen::Projection,
                                points: s.outline(),
                            },
                        );
                    }
                    for k in edited.iter().rev() {
                        items.insert(
                            0,
                            PlanItem::Polygon {
                                id: e.id,
                                pen: Pen::Projection,
                                points: s.layer_outline(*k),
                            },
                        );
                    }
                    let here = model.level(level).map_or(0.0, |l| l.elevation);
                    // a penetration draws its own mark
                    let pens: Vec<&Vec<DVec2>> = model
                        .iter()
                        .filter_map(|x| match &x.kind {
                            ElementKind::Shaft(sh) if sh.penetration.is_some() => {
                                Some(&sh.boundary)
                            }
                            _ => None,
                        })
                        .collect();
                    for h in &holes {
                        if pens.contains(&h) {
                            continue;
                        }
                        if stair_arrives_in(model, here, h) {
                            items.push(PlanItem::Polygon {
                                id: e.id,
                                pen: Pen::Projection,
                                points: h.clone(),
                            });
                        } else {
                            items.extend(hole_marks(e.id, h, Pen::Projection));
                        }
                    }
                }
            }
            ElementKind::Object(o) if o.level == level => {
                items.extend(crate::objects::object_plan(model, e, cut_height));
                shades.extend(crate::objects::object_shade(model, e, cut_height, graphics));
            }
            ElementKind::Valve(v) if v.host(model).is_some_and(|p| p.level == level) => {
                items.extend(crate::valves::valve_plan(model, e, v));
            }
            // gutters and downpipes, from whatever level: a downpipe runs
            // down past the storeys under its roof
            ElementKind::Flashing(f) => {
                let z = model.level(level).map_or(0.0, |l| l.elevation) + cut_height;
                items.extend(crate::flashing::plan_items(model, e, f, z));
            }
            // a roof over the top of the range is not seen (George
            // 2026-11-21: the plan's "Horná hrana rezu" decides)
            ElementKind::Roof(r)
                if on_level && crate::members::roof_low(model, r) >= range_top - 1e-3 => {}
            k if on_level && creabim_core::members::is_member(k) => {
                items.extend(crate::members::member_plan(model, e, cut_height));
                // a mass or sweep below the cut: its top seen tinted, in
                // its material under the colour styles (MU36: they read as
                // wireframe)
                // and a beam, in its colour as other members' (MU40: a beam
                // on the roof read as an empty outline)
                // every member: sweeps and extrusions, beams, roofs,
                // stairs, railings… (MU40: roofs and sweeps read empty)
                let solid: Option<(crate::TriMesh, [f32; 3])> = match k {
                    ElementKind::Part(p) if !p.void => {
                        crate::parts::part_mesh(model, p).map(|m| (m, [0.78, 0.76, 0.72]))
                    }
                    ElementKind::Part(_) => None,
                    _ => crate::members::member_mesh(model, e),
                };
                if let Some((mesh, base)) = solid {
                    let color = creabim_core::material_of(model, e).map_or(base, |(_, m)| m.color);
                    let item = crate::scene::SceneItem {
                        id: e.id,
                        mesh,
                        color,
                        surface: crate::scene::PLAIN_SURFACE,
                        ghost: false,
                        layer: None,
                        finish: None,
                    };
                    let z = model.level(level).map_or(0.0, |l| l.elevation) + cut_height;
                    shades.extend(mesh_shade(&item, z, graphics));
                }
            }
            // a column standing on a level below, running up through this one
            ElementKind::Column(c) if c.crosses(model, level, cut_height) => {
                items.push(PlanItem::Polygon {
                    id: e.id,
                    pen: Pen::Cut,
                    points: c.outline(),
                });
            }
            // one from a level below that rises into this level but stops
            // under the cut: seen from above (MU40: porch posts reaching
            // the roof were missing from the roof plan)
            ElementKind::Column(c)
                if !on_level && {
                    let (_, z1) = c.span(model);
                    let floor = model.level(level).map_or(0.0, |l| l.elevation);
                    z1 > floor + 0.05 && z1 <= floor + cut_height
                } =>
            {
                let pts = c.outline();
                let mut ring = pts.clone();
                if let Some(f) = pts.first() {
                    ring.push(*f);
                }
                for w in ring.windows(2) {
                    items.push(PlanItem::Line {
                        id: e.id,
                        pen: Pen::Projection,
                        a: w[0],
                        b: w[1],
                    });
                }
            }
            // a footing under a wall or column of a level above, which this
            // plan's plane cuts through: the plan of a foundation level
            ElementKind::Footing(f) if footing_cut_at(model, f, level, cut_height) => {
                for piece in creabim_core::footing::outlines(model, e, None) {
                    items.push(PlanItem::Polygon {
                        id: e.id,
                        pen: Pen::Cut,
                        points: piece,
                    });
                }
            }
            ElementKind::Shaft(sh) if sh.penetration.is_some() => {
                // a penetration through a slab (PilonDesigner's prestup):
                // in its own floor, thin solid; in the slab over this
                // storey (the ceiling), over the cut: dash and two dots
                let Some(pen) = slab_penetration_pen(model, sh, level) else {
                    continue;
                };
                let p = sh.penetration.as_ref().unwrap();
                let n = sh.boundary.len();
                for i in 0..n {
                    items.push(PlanItem::Line {
                        id: e.id,
                        pen,
                        a: sh.boundary[i],
                        b: sh.boundary[(i + 1) % n],
                    });
                }
                if !p.hole_only && !p.secondary {
                    let m = p.slab_mark(sh.centre());
                    for w in m.windows(2) {
                        items.push(PlanItem::Line {
                            id: e.id,
                            pen,
                            a: w[0],
                            b: w[1],
                        });
                    }
                }
            }
            ElementKind::Shaft(sh) => {
                // on every plan it passes through, crossed, to pick it by:
                // the level its top reaches too, whose floor it cuts
                let z = model.level(level).map_or(0.0, |l| l.elevation);
                let (z0, z1) = sh.span(model);
                if (z - z0).abs() < 1e-6 && stair_in(model, level, &sh.boundary) {
                    // a stair well where its stair starts: no floor is
                    // open here, the edge of the floor above is dashed
                    let n = sh.boundary.len();
                    for i in 0..n {
                        items.push(PlanItem::Line {
                            id: e.id,
                            pen: Pen::Hidden,
                            a: sh.boundary[i],
                            b: sh.boundary[(i + 1) % n],
                        });
                    }
                } else if z0 - 1e-6 <= z && z <= z1 + 1e-6 {
                    if z > z0 + 1e-6 && stair_arrives_in(model, z, &sh.boundary) {
                        // the stair drawn through it says what it is: its
                        // outline, with no cross over the stair
                        items.push(PlanItem::Polygon {
                            id: e.id,
                            pen: Pen::Symbol,
                            points: sh.boundary.clone(),
                        });
                    } else {
                        items.extend(hole_marks(e.id, &sh.boundary, Pen::Symbol));
                    }
                }
            }
            ElementKind::Sketch(sk) if sk.level == Some(level) && sk.view.is_none() => {
                items.extend(sketch_items(e.id, sk, &|p| p));
            }
            // guides lying flat; one standing on a wall face shows in sections
            ElementKind::Guide(g)
                if g.level.is_none_or(|l| l == level) && (g.a.z - g.b.z).abs() < 1e-3 =>
            {
                if let Some((a, b)) = guide_span(g.a.truncate(), g.b.truncate()) {
                    items.push(PlanItem::Line {
                        id: e.id,
                        pen: Pen::Guide,
                        a,
                        b,
                    });
                }
            }
            // a callout's detail view is marked by its box, not its line
            ElementKind::View(v) if v.callout.is_none() => {
                if let creabim_core::ViewKind::Section {
                    start, end, cut, ..
                } = &v.kind
                {
                    section_marker(&mut items, e.id, &e.name, *start, *end, *cut);
                }
            }
            _ => {}
        }
    }
    // a cut wall keeps its opening symbols but takes its outline from the slice
    items.retain(|it| !sliced.contains(&it.id()) || matches!(it, PlanItem::Label { .. }));
    layers.retain(|p| !sliced.contains(&p.id));
    // a structural plan: the walls one outline where they join, whatever
    // they are built of (George 2026-11-24)
    clean_up_pieces_with(model, &mut items, layers, false, graphics.structure_only);
    items.append(&mut overhead);
    items.extend(foundation_lines(model, level, cut_height));
    {
        // the walls standing between the cut and the top of the range
        let own_z = model.level(own).map_or(0.0, |l| l.elevation);
        let top_z = graphics
            .range
            .top
            .map_or_else(|| next_level_above(model, own_z), |t| t.z(model, own_z));
        let cut_z = model.level(level).map_or(0.0, |l| l.elevation) + cut_height;
        let core_only = is_foundation_plan(model, level, cut_height);
        items.extend(above_cut_walls(model, cut_z, top_z, core_only, &joins));
        items.extend(other_level_columns(model, level, cut_z, top_z, core_only));
        if !core_only {
            items.extend(above_cut_roofs(model, level, cut_z, top_z));
        }
    }
    if !sliced.is_empty() {
        let z = model.level(level).map_or(0.0, |l| l.elevation) + cut_height;
        // only what is sliced, and what cuts it
        let mut needed: std::collections::HashSet<ElementId> = sliced.iter().copied().collect();
        needed.extend(cuts.iter().filter(|c| sliced.contains(&c.0)).map(|c| c.1));
        let scene = crate::scene::build_scene_of(model, Some(&needed));
        for item in scene.iter().filter(|i| sliced.contains(&i.id)) {
            let mine: Vec<_> = forced.iter().filter(|f| f.0 == item.id).collect();
            if mine.is_empty() {
                sliced_plan(model, item, z, &mut items);
            } else {
                let mut got = Vec::new();
                sliced_plan(model, item, z, &mut got);
                for f in mine {
                    got = gap_out(got, item.id, f.1, f.2, f.3, f.4, f.5);
                }
                items.extend(got);
            }
            // an imported mesh wholly below the cut: its footprint tinted
            if matches!(
                model.get(item.id).map(|e| &e.kind),
                Some(ElementKind::FreeMesh(_))
            ) {
                shades.extend(mesh_shade(item, z, graphics));
            }
        }
    }
    items.extend(stairs_beyond(model, level, graphics));
    category_pens(model, &mut items, graphics);
    floor_change_lines(model, &mut items, &doors);
    shades.append(&mut items);
    PlanView { items: shades }
}

/// Where two floors meet in a door: a very thin dashed line (George
/// 2026-11-23), the slabs' edges in projection crossing a door opening
/// drawn so; elsewhere they stay as they were.
pub const FLOOR_CHANGE: creabim_core::LineLook = creabim_core::LineLook {
    pen: creabim_core::LinePen::VeryThin,
    kind: creabim_core::LineType::Dashed,
};

fn floor_change_lines(model: &Model, items: &mut Vec<PlanItem>, doors: &[Vec<DVec2>]) {
    if doors.is_empty() {
        return;
    }
    let mut out = Vec::with_capacity(items.len());
    // two floors meeting share the edge: one line, laid one way (two
    // dashed lines run opposite ways read as one solid)
    let mut drawn: Vec<(DVec2, DVec2)> = Vec::new();
    for it in items.drain(..) {
        let PlanItem::Polygon {
            id,
            pen: Pen::Projection,
            points,
        } = &it
        else {
            out.push(it);
            continue;
        };
        if !matches!(model.get(*id).map(|e| &e.kind), Some(ElementKind::Slab(_))) {
            out.push(it);
            continue;
        }
        let n = points.len();
        type Edge = (DVec2, DVec2, Vec<(DVec2, DVec2)>);
        let edges: Vec<Edge> = (0..n)
            .map(|i| {
                let (a, b) = (points[i], points[(i + 1) % n]);
                (a, b, clip_to_holes(a, b, doors))
            })
            .collect();
        if edges.iter().all(|e| e.2.is_empty()) {
            out.push(it);
            continue;
        }
        let id = *id;
        for (a, b, inside) in edges {
            // the edge's pieces outside the doors as before, inside dashed
            let mut from = a;
            for (p, q) in inside {
                if from.distance(p) > 1e-9 {
                    out.push(PlanItem::Line {
                        id,
                        pen: Pen::Projection,
                        a: from,
                        b: p,
                    });
                }
                let (p0, q0) = if (p.x, p.y) <= (q.x, q.y) {
                    (p, q)
                } else {
                    (q, p)
                };
                let twin = drawn
                    .iter()
                    .any(|(x, y)| x.distance(p0) < 1e-6 && y.distance(q0) < 1e-6);
                if !twin {
                    drawn.push((p0, q0));
                    out.push(PlanItem::Line {
                        id,
                        pen: Pen::Drafted(FLOOR_CHANGE),
                        a: p0,
                        b: q0,
                    });
                }
                from = q;
            }
            if from.distance(b) > 1e-9 {
                out.push(PlanItem::Line {
                    id,
                    pen: Pen::Projection,
                    a: from,
                    b,
                });
            }
        }
    }
    *items = out;
}

/// The lines a category draws in projection given the pen its view's
/// table names for it: furniture and fixtures in the fine pen.
pub(crate) fn category_pens(
    model: &Model,
    items: &mut [PlanItem],
    graphics: &creabim_core::PlanGraphics,
) {
    if graphics.category_pens.is_empty() {
        return;
    }
    let cutters = std::collections::HashSet::new();
    let mut of: std::collections::HashMap<ElementId, Option<Pen>> = Default::default();
    for it in items.iter_mut() {
        let (PlanItem::Line { id, pen, .. }
        | PlanItem::Polygon { id, pen, .. }
        | PlanItem::Arc { id, pen, .. }) = it
        else {
            continue;
        };
        if !matches!(pen, Pen::Projection | Pen::Symbol | Pen::Fine) {
            continue;
        }
        let to = *of.entry(*id).or_insert_with(|| {
            let e = model.get(*id)?;
            let c = creabim_core::visibility::Category::of(e, &cutters)?;
            graphics.pen_of(c).map(|p| match p {
                creabim_core::graphics::PenClass::Projection => Pen::Projection,
                creabim_core::graphics::PenClass::Symbol => Pen::Symbol,
                creabim_core::graphics::PenClass::Fine => Pen::Fine,
            })
        });
        if let Some(to) = to {
            *pen = to;
        }
    }
}

/// Does a stair standing on `level` start inside `outline`?
fn stair_in(model: &Model, level: ElementId, outline: &[DVec2]) -> bool {
    model.iter().any(|e| match &e.kind {
        ElementKind::Stair(s) => s.level == level && point_in(s.start, outline),
        _ => false,
    })
}

/// Does a stair from below arrive at the floor at height `z` through
/// `outline`: does its last flight end inside it?
fn stair_arrives_in(model: &Model, z: f64, outline: &[DVec2]) -> bool {
    model.iter().any(|e| {
        let ElementKind::Stair(s) = &e.kind else {
            return false;
        };
        let (base, rise) = (s.base(model), s.rise(model));
        base < z - 1e-6
            && (base + rise - z).abs() <= 0.05
            && s.layout(rise)
                .flights
                .last()
                .is_some_and(|f| point_in(f.end(s.tread) - f.dir * (s.tread / 2.0), outline))
    })
}

/// The footprint of a mesh lying wholly below `z`, as the faces it shows
/// from above, tinted. Very large meshes are tinted as their hull.
fn mesh_shade(
    item: &crate::scene::SceneItem,
    z: f64,
    graphics: &creabim_core::PlanGraphics,
) -> Vec<PlanItem> {
    let Some(rgb) = graphics.below_tint(item.color) else {
        return vec![];
    };
    let m = &item.mesh;
    let p = |i: u32| glam::Vec3::from(m.positions[i as usize]).as_dvec3();
    if m.positions.is_empty() {
        return vec![];
    }
    // what lies below the cut, seen from above: a solid the cut passes
    // through (a roof, a sloped beam) is tinted where it is below it
    // (MU40: they were left empty)
    let tris = m.indices.chunks(3).filter(|t| t.len() == 3);
    if m.indices.len() / 3 > 4000 {
        let pts: Vec<DVec2> = m
            .positions
            .iter()
            .filter(|q| (q[2] as f64) < z - 1e-6)
            .map(|q| DVec2::new(q[0] as f64, q[1] as f64))
            .collect();
        if pts.len() < 3 {
            return vec![];
        }
        let hull = creabim_core::objects::hull(&pts);
        return vec![PlanItem::Shade {
            id: item.id,
            points: hull,
            rgb,
        }];
    }
    tris.filter_map(|t| {
        let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
        // faces turned up, seen from above
        if (b - a).truncate().perp_dot((c - a).truncate()) <= 1e-9 {
            return None;
        }
        // the part of it below the cut (MU40: a face the cut crosses, as a
        // sloped beam's or a roof's top, was left empty)
        let mut poly: Vec<DVec2> = vec![];
        for (u, v) in [(a, b), (b, c), (c, a)] {
            let (iu, iv) = (u.z < z - 1e-6, v.z < z - 1e-6);
            if iu {
                poly.push(u.truncate());
            }
            if iu != iv {
                let k = (z - u.z) / (v.z - u.z);
                poly.push((u + (v - u) * k).truncate());
            }
        }
        (poly.len() >= 3).then_some(PlanItem::Shade {
            id: item.id,
            points: poly,
            rgb,
        })
    })
    .collect()
}

/// The band of wall `w` from `a` to `b` along it and `s0` to `s1` across,
/// as an outline: four corners on a straight wall, bending round a
/// curved one (MU40).
fn wall_band(
    w: &creabim_core::Wall,
    at: &dyn Fn(f64, f64) -> DVec2,
    a: f64,
    b: f64,
    s0: f64,
    s1: f64,
) -> Vec<DVec2> {
    let mut us = vec![a];
    us.extend(
        w.bends()
            .into_iter()
            .filter(|u| *u > a + 1e-9 && *u < b - 1e-9),
    );
    us.push(b);
    let mut pts: Vec<DVec2> = us.iter().map(|u| at(*u, s0)).collect();
    pts.extend(us.iter().rev().map(|u| at(*u, s1)));
    pts
}

/// The height of a wall's top above its base where it is highest. A top
/// attached to a roof or slab follows their undersides where they are
/// over it, and keeps the wall's own height elsewhere.
/// Is wall `w` in the plan of `level` cut `cut_height` above it? A wall of
/// the level itself is; so is a wall of another level that stands in
/// this storey: its base under the cut and its top above this level's
/// floor (a level put half way up a storey cuts the walls there, MU: a
/// plan of a level added at +2.584 was empty). Gives how far this level
/// lies above the wall's own.
/// The level and cut a plan is drawn from. A level put inside a storey
/// (to cut it there) has nothing built on it: its plan is the plan of the
/// storey it lies in, cut that much higher, so the floor, the stairs, the
/// furniture and what else stands below show as in any plan.
pub fn storey_cut(model: &Model, level: ElementId, cut_height: f64) -> (ElementId, f64) {
    let hosts = |l: ElementId| {
        model.iter().any(|e| {
            !matches!(e.kind, ElementKind::View(_))
                && creabim_core::levels::level_of(&e.kind) == Some(l)
        })
    };
    let Some(z) = model.level(level).map(|l| l.elevation) else {
        return (level, cut_height);
    };
    if hosts(level) {
        return (level, cut_height);
    }
    model
        .levels()
        .filter(|(e, l)| l.elevation < z - 1e-6 && hosts(e.id))
        .max_by(|a, b| a.1.elevation.total_cmp(&b.1.elevation))
        .map_or((level, cut_height), |(e, l)| {
            (e.id, cut_height + z - l.elevation)
        })
}

/// How a plan of `level` draws a penetration through a slab: thin solid
/// in its own level's floor, a dash and two dots in the slab of the level
/// over it (the ceiling, above the cut); none in the other plans.
pub fn slab_penetration_pen(
    model: &Model,
    sh: &creabim_core::Shaft,
    level: ElementId,
) -> Option<Pen> {
    if sh.level == level {
        return Some(Pen::Symbol);
    }
    let here = model.level(level)?.elevation;
    let above = model
        .levels()
        .map(|(e, l)| (e.id, l.elevation))
        .filter(|(_, z)| *z > here + 1e-6)
        .min_by(|a, b| a.1.total_cmp(&b.1))?;
    (above.0 == sh.level).then_some(Pen::Overhead)
}

/// The labels of the penetrations through the walls a plan of `level`
/// cuts ("P 300×200 sp. 900"), beside the wall's left face, at the
/// paper size of a drawing of `mm` metres per paper millimetre.
pub fn penetration_labels(
    model: &Model,
    level: ElementId,
    cut_height: f64,
    mm: f64,
) -> Vec<PlanItem> {
    use crate::paper::Anchor;
    const SIZE: f64 = 2.0;
    let mut out = vec![];
    for e in model.iter() {
        // through a slab: under its outline, "(strop)" when in the ceiling
        if let ElementKind::Shaft(sh) = &e.kind {
            let Some(p) = sh.penetration.as_ref().filter(|p| !p.hole_only) else {
                continue;
            };
            let Some(pen) = slab_penetration_pen(model, sh, level) else {
                continue;
            };
            let mut text = p.slab_label();
            if pen == Pen::Overhead {
                text = format!("{text} (strop)").trim().to_string();
            }
            if text.is_empty() {
                continue;
            }
            let c = sh.centre();
            let low = sh.boundary.iter().map(|q| q.y).fold(f64::MAX, f64::min);
            out.push(PlanItem::Text {
                id: e.id,
                at: DVec2::new(c.x, low - 1.5 * mm - SIZE * 0.6 * mm),
                text,
                size: SIZE,
                align: Anchor::Middle,
                mm,
                rgb: None,
                bold: false,
            });
            continue;
        }
        let ElementKind::Opening(o) = &e.kind else {
            continue;
        };
        if !o.is_penetration() {
            continue;
        }
        let Some(ElementKind::Wall(w)) = model.get(o.host).map(|h| &h.kind) else {
            continue;
        };
        if w.curtain.is_some() || wall_rise(model, w, level, cut_height).is_none() {
            continue;
        }
        let text = o.penetration_label(o.sill + w.base_offset);
        if text.is_empty() {
            continue;
        }
        let mid = o.offset + o.width / 2.0;
        let n = w.tangent_at(mid).normalize_or_zero().perp();
        let gap = 1.5 * mm;
        let reach = w.thickness / 2.0 + gap;
        let (at, align) = if n.x.abs() > 0.7 {
            // beside a wall running up the sheet: from its face outwards
            let a = if n.x > 0.0 {
                Anchor::Start
            } else {
                Anchor::End
            };
            (w.place(mid, reach), a)
        } else {
            (w.place(mid, reach + SIZE * 0.6 * mm), Anchor::Middle)
        };
        out.push(PlanItem::Text {
            id: e.id,
            at,
            text,
            size: SIZE,
            align,
            mm,
            rgb: None,
            bold: false,
        });
    }
    out
}

/// Whether a plan of `level` cut at `cut_height` cuts wall `w`, and how
/// far below the level its own level is.
pub(crate) fn wall_rise(
    model: &Model,
    w: &creabim_core::Wall,
    level: ElementId,
    cut_height: f64,
) -> Option<f64> {
    if w.level == level {
        return Some(0.0);
    }
    let z = |l| model.level(l).map(|x| x.elevation);
    let rise = z(level)? - z(w.level)?;
    let floor = rise - w.base_offset;
    (cut_height + floor > 1e-3 && wall_top_height(model, w) > floor + 1e-3).then_some(rise)
}

fn wall_top_height(model: &Model, w: &creabim_core::Wall) -> f64 {
    let Some(top) = creabim_core::walls::top_profile(model, w) else {
        return w.height;
    };
    let base = creabim_core::walls::base_z(model, w);
    let mut ts = vec![0.0, 1.0];
    ts.extend(top.breaks(w.start, w.end));
    ts.iter()
        .flat_map(|t| [t - 1e-4, t + 1e-4])
        .filter(|t| (0.0..=1.0).contains(t))
        .map(|t| {
            top.z_at(w.start.lerp(w.end, t))
                .map_or(w.height, |z| z - base)
        })
        .fold(0.0, f64::max)
}

/// Stairs climbing from a level below up to `level`, seen through the
/// holes in its floor (a stair well, a void): their treads, sides and
/// landing in projection, clipped to the holes, with the walking line, an
/// arrow and DN where it reaches this floor. What lies deeper than the
/// view depth is left out.
fn stairs_beyond(
    model: &Model,
    level: ElementId,
    graphics: &creabim_core::PlanGraphics,
) -> Vec<PlanItem> {
    let Some(here) = model.level(level).map(|l| l.elevation) else {
        return vec![];
    };
    let below = model
        .levels()
        .map(|(_, l)| l.elevation)
        .filter(|z| *z < here - 1e-6)
        .reduce(f64::max);
    let bottom = here - graphics.depth(here, below);
    // the floor's holes: shafts reaching up to it and holes in its slabs
    let mut holes: Vec<Vec<DVec2>> = Vec::new();
    for e in model.iter() {
        match &e.kind {
            ElementKind::Shaft(sh) => {
                let (z0, z1) = sh.span(model);
                if z0 < here - 1e-6 && here <= z1 + 1e-6 {
                    holes.push(sh.boundary.clone());
                }
            }
            ElementKind::Slab(s) if s.level == level => {
                holes.extend(creabim_core::slab::holes_of(model, e));
            }
            _ => {}
        }
    }
    if holes.is_empty() {
        return vec![];
    }
    let mut out = Vec::new();
    for e in model.iter() {
        let ElementKind::Stair(s) = &e.kind else {
            continue;
        };
        let base = s.base(model);
        let rise = s.rise(model);
        // it arrives at this floor from below
        if base >= here - 1e-6 || (base + rise - here).abs() > 0.05 {
            continue;
        }
        let mut lines: Vec<(Pen, DVec2, DVec2)> = Vec::new();
        crate::members::stair_lines(s, rise, &mut |pen, a, b, z| {
            if base + z >= bottom - 1e-6 {
                lines.push((pen, a, b));
            }
        });
        for (pen, a, b) in lines {
            for (p, q) in clip_to_holes(a, b, &holes) {
                out.push(PlanItem::Line {
                    id: e.id,
                    pen,
                    a: p,
                    b: q,
                });
            }
        }
        let lay = s.layout(rise);
        if let Some(f) = lay.flights.last() {
            let top = f.end(s.tread);
            if holes.iter().any(|h| point_in(top, h)) {
                out.push(PlanItem::Label {
                    id: e.id,
                    at: top + f.dir * 0.35,
                    text: "DN".into(),
                });
            }
        }
    }
    out
}

/// The parts of `a` → `b` inside any of `holes`.
fn clip_to_holes(a: DVec2, b: DVec2, holes: &[Vec<DVec2>]) -> Vec<(DVec2, DVec2)> {
    let len = a.distance(b);
    if len < 1e-9 {
        return vec![];
    }
    let d = (b - a) / len;
    let mut out: Vec<(f64, f64)> = Vec::new();
    for h in holes {
        let mut ts = vec![0.0, len];
        let n = h.len();
        for i in 0..n {
            let (p, q) = (h[i], h[(i + 1) % n]);
            let e = q - p;
            let den = d.perp_dot(e);
            if den.abs() < 1e-12 {
                continue;
            }
            let t = (p - a).perp_dot(e) / den;
            let s = (p - a).perp_dot(d) / den;
            if (0.0..=1.0).contains(&s) && t > 0.0 && t < len {
                ts.push(t);
            }
        }
        ts.sort_by(|x, y| x.total_cmp(y));
        for w in ts.windows(2) {
            // on or inside the hole's edge
            let m = a + d * ((w[0] + w[1]) / 2.0);
            if w[1] > w[0] + 1e-9 && (point_in(m, h) || edge_distance(m, h) < 1e-6) {
                out.push((w[0], w[1]));
            }
        }
    }
    out.sort_by(|x, y| x.0.total_cmp(&y.0));
    // join what touches, so overlapping holes give one line
    let mut joined: Vec<(f64, f64)> = Vec::new();
    for (lo, hi) in out {
        match joined.last_mut() {
            Some(last) if lo <= last.1 + 1e-9 => last.1 = last.1.max(hi),
            _ => joined.push((lo, hi)),
        }
    }
    joined
        .into_iter()
        .map(|(lo, hi)| (a + d * lo, a + d * hi))
        .collect()
}

/// The plan of `level` as an underlay: its linework (cut outlines,
/// projection, symbols) as halftone lines and arcs, without fills, text,
/// grids, guides or view marks.
pub fn underlay_items(model: &Model, level: ElementId, cut_height: f64) -> Vec<PlanItem> {
    as_underlay(model, plan_view(model, level, cut_height).items)
}

/// A drawing's `items` as an underlay: halftone lines and arcs, without
/// fills, text, grids, guides or view marks.
pub fn as_underlay(model: &Model, items: Vec<PlanItem>) -> Vec<PlanItem> {
    let skip = |id: ElementId| {
        matches!(
            model.get(id).map(|e| &e.kind),
            Some(ElementKind::View(_) | ElementKind::Grid(_) | ElementKind::GridGroup(_))
        )
    };
    let mut out = Vec::new();
    for it in items {
        match it {
            PlanItem::Line { id, pen, a, b } if pen != Pen::Guide && pen != Pen::Grid => {
                if !skip(id) {
                    out.push(PlanItem::Line {
                        id,
                        pen: Pen::Underlay,
                        a,
                        b,
                    });
                }
            }
            PlanItem::Polygon { id, points, .. } if !skip(id) => {
                let n = points.len();
                for i in 0..n {
                    out.push(PlanItem::Line {
                        id,
                        pen: Pen::Underlay,
                        a: points[i],
                        b: points[(i + 1) % n],
                    });
                }
            }
            PlanItem::Arc {
                id,
                center,
                radius,
                start,
                end,
                ..
            } if !skip(id) => out.push(PlanItem::Arc {
                id,
                pen: Pen::Underlay,
                center,
                radius,
                start,
                end,
            }),
            _ => {}
        }
    }
    out
}

/// A hole through a slab, roof or shaft as plans draw it: its outline
/// and a cross corner to corner.
pub fn hole_marks(id: ElementId, hole: &[DVec2], pen: Pen) -> Vec<PlanItem> {
    let mut out = vec![PlanItem::Polygon {
        id,
        pen,
        points: hole.to_vec(),
    }];
    for (a, b) in hole_cross(hole) {
        out.push(PlanItem::Line {
            id,
            pen: Pen::Fine,
            a,
            b,
        });
    }
    out
}

/// The cross drawn in a hole: corner to corner of each convex piece of
/// it, so an L-shaped stair well has a cross in each leg and no line
/// leaves the opening.
pub fn hole_cross(hole: &[DVec2]) -> Vec<(DVec2, DVec2)> {
    let mut out = Vec::new();
    for piece in convex_pieces(hole) {
        if piece.len() < 4 {
            continue;
        }
        // the corners nearest the corners of its box: in a convex piece
        // the lines between them stay inside it
        let lo = piece.iter().copied().reduce(DVec2::min).unwrap_or_default();
        let hi = piece.iter().copied().reduce(DVec2::max).unwrap_or_default();
        let near = |p: DVec2| {
            piece
                .iter()
                .copied()
                .min_by(|a, b| a.distance(p).total_cmp(&b.distance(p)))
                .unwrap_or(p)
        };
        for (a, b) in [(lo, hi), (DVec2::new(lo.x, hi.y), DVec2::new(hi.x, lo.y))] {
            let (a, b) = (near(a), near(b));
            if a.distance(b) > 1e-6 {
                out.push((a, b));
            }
        }
    }
    out
}

/// A polygon split into convex pieces: itself when convex, rectangles
/// when its sides all run along x and y, otherwise its triangles merged
/// while they stay convex (Hertel and Mehlhorn).
pub fn convex_pieces(poly: &[DVec2]) -> Vec<Vec<DVec2>> {
    let mut poly = poly.to_vec();
    poly.dedup_by(|a, b| a.distance(*b) < 1e-9);
    if poly.len() > 1 && poly[0].distance(poly[poly.len() - 1]) < 1e-9 {
        poly.pop();
    }
    if poly.len() < 3 || is_convex(&poly) {
        return vec![poly];
    }
    let n = poly.len();
    let square = (0..n).all(|i| {
        let e = poly[(i + 1) % n] - poly[i];
        e.x.abs() < 1e-9 || e.y.abs() < 1e-9
    });
    if square {
        return rectangles(&poly);
    }
    let mut pieces: Vec<Vec<DVec2>> = creabim_core::polygon::triangulate(&poly)
        .into_iter()
        .map(|t| t.iter().map(|i| poly[*i]).collect())
        .collect();
    // merge neighbours across a shared side while the result is convex
    let mut merged = true;
    while merged {
        merged = false;
        'outer: for i in 0..pieces.len() {
            for j in i + 1..pieces.len() {
                if let Some(m) = join_pieces(&pieces[i], &pieces[j]) {
                    if is_convex(&m) {
                        pieces[i] = m;
                        pieces.remove(j);
                        merged = true;
                        break 'outer;
                    }
                }
            }
        }
    }
    pieces
}

/// Two polygons sharing a side, as one (none when they share none).
fn join_pieces(a: &[DVec2], b: &[DVec2]) -> Option<Vec<DVec2>> {
    let close = |p: DVec2, q: DVec2| p.distance(q) < 1e-9;
    let (na, nb) = (a.len(), b.len());
    for i in 0..na {
        let (p, q) = (a[i], a[(i + 1) % na]);
        for j in 0..nb {
            if close(b[j], q) && close(b[(j + 1) % nb], p) {
                // a from q round to p, then b from p round to q
                let mut out: Vec<DVec2> = (0..na).map(|k| a[(i + 1 + k) % na]).collect();
                out.extend((1..nb - 1).map(|k| b[(j + 1 + k) % nb]));
                // drop corners left in a straight line
                let m = out.len();
                let keep: Vec<DVec2> = (0..m)
                    .filter(|k| {
                        let (u, v, w) = (out[(k + m - 1) % m], out[*k], out[(k + 1) % m]);
                        (v - u).perp_dot(w - v).abs() > 1e-12
                    })
                    .map(|k| out[k])
                    .collect();
                return Some(keep);
            }
        }
    }
    None
}

/// A polygon with sides along x and y as rectangles: the cells between
/// its corner lines, merged into bands across and then down.
fn rectangles(poly: &[DVec2]) -> Vec<Vec<DVec2>> {
    let sorted = |mut v: Vec<f64>| {
        v.sort_by(|a, b| a.total_cmp(b));
        v.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        v
    };
    let xs = sorted(poly.iter().map(|p| p.x).collect());
    let ys = sorted(poly.iter().map(|p| p.y).collect());
    // each row of cells as runs [x0, x1) inside the polygon
    let mut rows: Vec<Vec<(usize, usize)>> = Vec::new();
    for j in 0..ys.len().saturating_sub(1) {
        let y = (ys[j] + ys[j + 1]) / 2.0;
        let mut runs = Vec::new();
        let mut start: Option<usize> = None;
        for i in 0..xs.len() - 1 {
            let inside = point_in(DVec2::new((xs[i] + xs[i + 1]) / 2.0, y), poly);
            match (inside, start) {
                (true, None) => start = Some(i),
                (false, Some(s)) => {
                    runs.push((s, i));
                    start = None;
                }
                _ => {}
            }
        }
        if let Some(s) = start {
            runs.push((s, xs.len() - 1));
        }
        rows.push(runs);
    }
    // runs repeated row on row make one rectangle
    let mut out = Vec::new();
    let mut open: Vec<((usize, usize), usize)> = Vec::new();
    for (j, runs) in rows.iter().enumerate() {
        let mut next = Vec::new();
        for (run, from) in open.drain(..) {
            if runs.contains(&run) {
                next.push((run, from));
            } else {
                out.push((run, from, j));
            }
        }
        for run in runs {
            if !next.iter().any(|(r, _)| r == run) {
                next.push((*run, j));
            }
        }
        open = next;
    }
    out.extend(open.into_iter().map(|(run, from)| (run, from, rows.len())));
    out.into_iter()
        .map(|((i0, i1), j0, j1)| {
            let (x0, x1, y0, y1) = (xs[i0], xs[i1], ys[j0], ys[j1]);
            vec![
                DVec2::new(x0, y0),
                DVec2::new(x1, y0),
                DVec2::new(x1, y1),
                DVec2::new(x0, y1),
            ]
        })
        .collect()
}

/// Is this the linework of an underlay (drawn faintly, never picked)?
/// How far inside another element's cut a line must run to count as
/// hidden by it, so lines along its edge (a slab under a wall) stay as
/// they are.
const COVER_MARGIN: f64 = 0.005;

/// Lines of elements lying under another element's cut fill, drawn again
/// dashed on top: what a view showing hidden elements adds, e.g. a beam
/// inside a wall or a stud wall in a cavity. Doors and windows, which sit
/// in their walls' holes, are left as they are.
pub fn reveal_covered(model: &Model, items: &mut Vec<PlanItem>) {
    let fills: Vec<(ElementId, &[DVec2], DVec2, DVec2)> = items
        .iter()
        .filter_map(|it| match it {
            PlanItem::Fill { id, points, .. } if points.len() >= 3 => {
                let lo = points.iter().copied().reduce(DVec2::min)?;
                let hi = points.iter().copied().reduce(DVec2::max)?;
                Some((*id, points.as_slice(), lo, hi))
            }
            _ => None,
        })
        .collect();
    if fills.is_empty() {
        return;
    }
    let mut segments: Vec<(ElementId, DVec2, DVec2)> = Vec::new();
    for it in items.iter() {
        match it {
            PlanItem::Line {
                id,
                pen: Pen::Projection | Pen::Hidden,
                a,
                b,
            } => segments.push((*id, *a, *b)),
            PlanItem::Polygon {
                id,
                pen: Pen::Projection,
                points,
            } => {
                let n = points.len();
                segments.extend((0..n).map(|i| (*id, points[i], points[(i + 1) % n])));
            }
            _ => {}
        }
    }
    let opening = |id: ElementId| {
        matches!(
            model.get(id).map(|e| &e.kind),
            Some(ElementKind::Opening(_))
        )
    };
    let covered = |own: ElementId, p: DVec2| {
        fills.iter().any(|(id, poly, lo, hi)| {
            *id != own
                && p.cmpge(*lo).all()
                && p.cmple(*hi).all()
                && point_in(p, poly)
                && edge_distance(p, poly) > COVER_MARGIN
        })
    };
    let mut out = Vec::new();
    for (id, a, b) in segments {
        if opening(id) {
            continue;
        }
        let steps = (a.distance(b) / 0.01).ceil().clamp(4.0, 400.0) as usize;
        let mut start: Option<f64> = None;
        let mut last = 0.0;
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let inside = covered(id, a + (b - a) * t);
            match (inside, start) {
                (true, None) => start = Some(t),
                (false, Some(s)) => {
                    out.push((id, s, last, a, b));
                    start = None;
                }
                _ => {}
            }
            if inside {
                last = t;
            }
        }
        if let Some(s) = start {
            out.push((id, s, 1.0, a, b));
        }
    }
    items.extend(
        out.into_iter()
            .filter(|(_, s, e, ..)| e > s)
            .map(|(id, s, e, a, b)| PlanItem::Line {
                id,
                pen: Pen::Hidden,
                a: a + (b - a) * s,
                b: a + (b - a) * e,
            }),
    );
}

/// How far `p` is from the nearest edge of `poly`.
fn edge_distance(p: DVec2, poly: &[DVec2]) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let ab = b - a;
            let t = ((p - a).dot(ab) / ab.length_squared().max(1e-12)).clamp(0.0, 1.0);
            p.distance(a + ab * t)
        })
        .fold(f64::INFINITY, f64::min)
}

pub fn is_underlay(item: &PlanItem) -> bool {
    matches!(
        item,
        PlanItem::Line {
            pen: Pen::Underlay,
            ..
        } | PlanItem::Arc {
            pen: Pen::Underlay,
            ..
        }
    )
}

/// How far guides reach each way from their points: endless for any
/// building, short enough to stay well inside float precision.
pub const GUIDE_REACH: f64 = 200.0;

/// A guide through `a` and `b` as a long segment.
pub fn guide_span(a: DVec2, b: DVec2) -> Option<(DVec2, DVec2)> {
    let d = (b - a).try_normalize()?;
    let m = (a + b) / 2.0;
    Some((m - d * GUIDE_REACH, m + d * GUIDE_REACH))
}

/// The plan of a shape with no simple footprint: its section at `z`,
/// filled, and its edges below `z` in projection.
/// A door or window in plan, where the plan cuts the wall at `cut_v` above
/// its base: the frame members the cut passes through, and for a window
/// the sill member below, the wall faces and the glass line; for a door
/// its leaf drawn open and the swing it sweeps. `at(u, s)` is a point
/// `u` along the wall and `s` across it.
#[allow(clippy::too_many_arguments)]
/// A lift-and-slide leaf's mark in plan: a dashed arrow inside the
/// frame, along the leaf the way it slides, a foot towards the frame
/// where it starts (the lift) and a half head where it ends.
fn lift_slide_marks(
    out: &mut Vec<PlanItem>,
    id: ElementId,
    o: &creabim_core::Opening,
    w: &creabim_core::Wall,
    asm: Option<&creabim_core::assembly::Assembly>,
    c: &Cut,
    at: &dyn Fn(f64, f64) -> DVec2,
) {
    for m in crate::opening::lift_slides(o, w, asm) {
        let len = m.x1 - m.x0;
        if len < 0.2 {
            continue;
        }
        let mid = c.u0 + (m.x0 + m.x1) / 2.0;
        let dir = if m.to_high { 1.0 } else { -1.0 };
        let (u0, u1) = (mid + dir * len * 0.1, mid + dir * len * 0.42);
        let s = m.y_in + m.inward * 0.06;
        let tick = 0.035;
        let mut seg = |a: DVec2, b: DVec2| {
            out.push(PlanItem::Line {
                id,
                pen: Pen::Hidden,
                a,
                b,
            })
        };
        seg(at(u0, s), at(u1, s));
        seg(at(u0, s), at(u0, s - m.inward * tick));
        seg(at(u1, s), at(u1 - dir * tick * 0.7, s + m.inward * tick));
    }
}

fn opening_symbol(
    id: ElementId,
    o: &creabim_core::Opening,
    w: &creabim_core::Wall,
    asm: Option<&creabim_core::assembly::Assembly>,
    c: &Cut,
    cut_v: f64,
    at: &dyn Fn(f64, f64) -> DVec2,
) -> Vec<PlanItem> {
    use crate::opening::{opening_boxes, Fitted, Part};
    let mut out = Vec::new();
    if Fitted::of(o, w).is_none() {
        return out;
    }
    let z = cut_v - c.v0;
    let line = |out: &mut Vec<PlanItem>, a: DVec2, b: DVec2| {
        out.push(PlanItem::Line {
            id,
            pen: Pen::Symbol,
            a,
            b,
        })
    };
    let rect = |out: &mut Vec<PlanItem>, u0: f64, u1: f64, s0: f64, s1: f64| {
        out.push(PlanItem::Polygon {
            id,
            pen: Pen::Symbol,
            points: vec![at(u0, s0), at(u1, s0), at(u1, s1), at(u0, s1)],
        })
    };
    let boxes = opening_boxes(o, w, asm);
    for b in &boxes {
        let (u0, u1) = (c.u0 + b.lo.x, c.u0 + b.hi.x);
        let cut = b.lo.z < z && z < b.hi.z;
        match b.part {
            // the sill member shows below the cut
            // and so does a door's threshold
            Part::Frame if cut || (o.kind != OpeningKind::Hole && b.lo.z <= 1e-9) => {
                rect(&mut out, u0, u1, b.lo.y, b.hi.y)
            }
            Part::Glass if cut => {
                let s = (b.lo.y + b.hi.y) / 2.0;
                line(&mut out, at(u0, s), at(u1, s));
            }
            _ => {}
        }
    }
    match o.kind {
        // a hole that starts above the floor shows the wall's faces across,
        // over its sill, as a window does; one down to the floor is just
        // the gap (MU18)
        // a penetration cut by the plane: its outline across the wall and
        // a cross (PilonDesigner's prestup); one set to show just the
        // hole, its outline
        OpeningKind::Hole if o.penetration.is_some() || o.is_penetration() => {
            let t = w.thickness / 2.0;
            let corners = [at(c.u0, -t), at(c.u1, -t), at(c.u1, t), at(c.u0, t)];
            out.push(PlanItem::Polygon {
                id,
                pen: Pen::Symbol,
                points: corners.to_vec(),
            });
            if o.is_penetration() {
                line(&mut out, corners[0], corners[2]);
                line(&mut out, corners[1], corners[3]);
            }
        }
        OpeningKind::Hole => {
            if c.v0 > 1e-9 {
                let t = w.thickness / 2.0;
                for s in [-t, t] {
                    line(&mut out, at(c.u0, s), at(c.u1, s));
                }
            }
        }
        OpeningKind::Window => {
            // the wall faces carry on across, over the sill
            let t = w.thickness / 2.0;
            for s in [-t, t] {
                line(&mut out, at(c.u0, s), at(c.u1, s));
            }
        }
        OpeningKind::Door => {}
    }
    if matches!(o.kind, OpeningKind::Window | OpeningKind::Door) {
        lift_slide_marks(&mut out, id, o, w, asm, c, at);
    }
    if o.kind == OpeningKind::Door {
        {
            // each leaf turned open about its hinge on the face of the
            // frame on the side it opens to, and the quarter circle it
            // sweeps
            // the wall carries on over the door, above the cut: its
            // faces across the opening, a dash and two dots
            let t = w.thickness / 2.0;
            for s in [-t, t] {
                out.push(PlanItem::Line {
                    id,
                    pen: Pen::Overhead,
                    a: at(c.u0, s),
                    b: at(c.u1, s),
                });
            }
            let left = o.faces_left(w);
            let side = if left { 1.0 } else { -1.0 };
            let leaves = crate::opening::leaf_boxes(o, w, asm);
            for leaf in &leaves {
                let s = if left { leaf.hi.y } else { leaf.lo.y };
                let r = leaf.hi.x - leaf.lo.x;
                // hinged on the side away from its handle
                let mid = (leaf.lo.x + leaf.hi.x) / 2.0;
                let at_end = boxes
                    .iter()
                    .find(|b| b.part == Part::Handle && b.lo.x >= leaf.lo.x && b.hi.x <= leaf.hi.x)
                    .map_or(o.flip_hand, |h| (h.lo.x + h.hi.x) / 2.0 < mid);
                let (u, towards) = if at_end {
                    (c.u0 + leaf.hi.x, -1.0)
                } else {
                    (c.u0 + leaf.lo.x, 1.0)
                };
                let lt = (leaf.hi.y - leaf.lo.y).max(0.02);
                rect(&mut out, u, u + towards * lt, s, s + side * r);
                let hinge = at(u, s);
                let closed = at(u + towards, s) - hinge;
                let open = at(u, s + side) - hinge;
                // arcs run anticlockwise: from whichever of the two comes first
                let a0 = if closed.perp_dot(open) > 0.0 {
                    closed.to_angle()
                } else {
                    open.to_angle()
                };
                out.push(PlanItem::Arc {
                    id,
                    pen: Pen::Symbol,
                    center: hinge,
                    radius: r,
                    start: a0,
                    end: a0 + std::f64::consts::FRAC_PI_2,
                });
            }
        }
    }
    out
}

/// A sliced wall's drawing with a window or door drawn as cut off the
/// cut plane: its fills and outline cut back from the opening's span
/// `u0..u1` along the wall (from `start`, along `d`), the jambs closed.
fn gap_out(
    got: Vec<PlanItem>,
    wall: ElementId,
    start: DVec2,
    d: DVec2,
    u0: f64,
    u1: f64,
    t: f64,
) -> Vec<PlanItem> {
    let n = d.perp();
    let local = |p: DVec2| DVec2::new((p - start).dot(d), (p - start).dot(n));
    let world = |q: DVec2| start + d * q.x + n * q.y;
    let s = t + 0.05;
    let (lo, hi) = (DVec2::new(u0, -s), DVec2::new(u1, s));
    let rect = [
        world(lo),
        world(DVec2::new(u1, -s)),
        world(hi),
        world(DVec2::new(u0, s)),
    ];
    let on_jamb = |a: DVec2, b: DVec2| {
        let (a, b) = (local(a), local(b));
        [u0, u1]
            .iter()
            .any(|u| (a.x - u).abs() < 1e-6 && (b.x - u).abs() < 1e-6)
    };
    let mut out = Vec::new();
    for it in got {
        match it {
            PlanItem::Fill {
                id,
                points,
                hatch,
                rgb,
            } if id == wall => {
                for piece in subtract_convex(&points, &rect) {
                    let k = piece.len();
                    for i in 0..k {
                        let (a, b) = (piece[i], piece[(i + 1) % k]);
                        if on_jamb(a, b) {
                            out.push(PlanItem::Line {
                                id,
                                pen: Pen::Cut,
                                a,
                                b,
                            });
                        }
                    }
                    out.push(PlanItem::Fill {
                        id,
                        points: piece,
                        hatch,
                        rgb,
                    });
                }
            }
            PlanItem::Line { id, pen, a, b } if id == wall => {
                let (la, lb) = (local(a), local(b));
                match crate::crop::clip_segment(la, lb, lo, hi) {
                    None => out.push(PlanItem::Line { id, pen, a, b }),
                    Some((p, q)) => {
                        for (x, y) in [(la, p), (q, lb)] {
                            if x.distance(y) > 1e-6 {
                                out.push(PlanItem::Line {
                                    id,
                                    pen,
                                    a: world(x),
                                    b: world(y),
                                });
                            }
                        }
                    }
                }
            }
            other => out.push(other),
        }
    }
    out
}

pub(crate) fn sliced_plan(
    model: &Model,
    item: &crate::scene::SceneItem,
    z: f64,
    items: &mut Vec<PlanItem>,
) {
    sliced_plan_creased(model, item, z, items, 0.995);
}

/// [`sliced_plan`] with its own crease for the edges seen below the cut
/// (see [`crate::section::feature_edges_over`]).
pub(crate) fn sliced_plan_creased(
    model: &Model,
    item: &crate::scene::SceneItem,
    z: f64,
    items: &mut Vec<PlanItem>,
    crease: f32,
) {
    let mut cut: Vec<(DVec2, DVec2)> = Vec::new();
    for lp in crate::slice::slice(&item.mesh, z) {
        let n = lp.len();
        for i in 0..n {
            cut.push((lp[i], lp[(i + 1) % n]));
            items.push(PlanItem::Line {
                id: item.id,
                pen: Pen::Cut,
                a: lp[i],
                b: lp[(i + 1) % n],
            });
        }
        let piece = CutPiece {
            id: item.id,
            points: vec![],
            layer: item.layer,
        };
        items.push(PlanItem::Fill {
            id: item.id,
            points: lp,
            hatch: piece.hatch(model, false),
            rgb: piece.colour(model, false),
        });
    }
    for (a, b) in crate::section::feature_edges_over(item, crease) {
        let (a, b) = if a.z <= b.z { (a, b) } else { (b, a) };
        if a.z >= z - 1e-6 {
            continue;
        }
        // keep the part below the cut
        let b = if b.z > z {
            a + (b - a) * ((z - a.z) / (b.z - a.z))
        } else {
            b
        };
        // edges seen end-on, and those lying in the cut, add nothing
        let (a, b) = (a.truncate(), b.truncate());
        if a.distance(b) < 1e-6 {
            continue;
        }
        // edges hidden under the heavy cut line
        let on = |p: DVec2, (s, e): (DVec2, DVec2)| {
            let d = e - s;
            let t = ((p - s).dot(d) / d.length_squared().max(1e-12)).clamp(0.0, 1.0);
            p.distance(s + d * t) < 1e-4
        };
        if cut.iter().any(|&c| on(a, c) && on(b, c)) {
            continue;
        }
        items.push(PlanItem::Line {
            id: item.id,
            pen: Pen::Projection,
            a,
            b,
        });
    }
}

/// Section line with view arrows, or an elevation marker, as drawn in plan.
fn section_marker(
    items: &mut Vec<PlanItem>,
    id: ElementId,
    name: &str,
    a: DVec2,
    b: DVec2,
    cut: bool,
) {
    let along = (b - a).normalize_or_zero();
    let look = along.perp();
    let arrow = |at: DVec2| PlanItem::Polygon {
        id,
        pen: Pen::Symbol,
        points: vec![at - along * 0.35, at + along * 0.35, at + look * 0.45],
    };
    let tag = name.rsplit(' ').next().unwrap_or(name).to_string();
    if cut {
        items.push(PlanItem::Line {
            id,
            pen: Pen::Symbol,
            a,
            b,
        });
        for end in [a, b] {
            items.push(arrow(end));
        }
        items.push(PlanItem::Label {
            id,
            at: a - along * 0.7,
            text: tag.clone(),
        });
        items.push(PlanItem::Label {
            id,
            at: b + along * 0.7,
            text: tag,
        });
    } else {
        let mid = (a + b) / 2.0;
        items.push(arrow(mid));
        items.push(PlanItem::Label {
            id,
            at: mid - look * 0.6,
            text: tag,
        });
    }
}

/// The level a height stands on: the highest at or below it, else the
/// lowest.
pub fn level_under(model: &Model, z: f64) -> Option<ElementId> {
    let mut all: Vec<(ElementId, f64)> = model.levels().map(|(e, l)| (e.id, l.elevation)).collect();
    all.sort_by(|a, b| a.1.total_cmp(&b.1));
    all.iter()
        .rev()
        .find(|l| l.1 <= z + 0.05)
        .or(all.first())
        .map(|l| l.0)
}

#[cfg(test)]
mod tests {
    /// George 2026-11-23: in a foundation plan a wall of DT blocks only
    /// ending against the masonry with its insulation joins it: the
    /// latter's line on that side stops for it.
    #[test]
    fn a_wall_ending_against_another_above_the_cut_opens_its_line() {
        use creabim_core::{Element, Level, Transaction, Wall};
        let mut m = Model::new();
        let lv = Element::new(
            "L",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let l = lv.id;
        let main = Element::new(
            "main",
            ElementKind::Wall(Wall::new(l, DVec2::ZERO, DVec2::new(10.0, 0.0), 0.445, 1.0)),
        );
        let mid = main.id;
        let branch = Element::new(
            "branch",
            ElementKind::Wall(Wall::new(
                l,
                DVec2::new(5.0, -4.0),
                DVec2::new(5.0, 0.0),
                0.3,
                1.0,
            )),
        );
        Transaction::new("t")
            .insert(lv)
            .insert(main)
            .insert(branch)
            .apply(&mut m)
            .unwrap();
        let joins = creabim_core::wall_joins(&m);
        let items = above_cut_walls(&m, -1.0, 1.0, true, &joins);
        let low: Vec<(DVec2, DVec2)> = items
            .iter()
            .filter_map(|it| match it {
                PlanItem::Line { id, a, b, .. } if *id == mid && a.y < -0.1 && b.y < -0.1 => {
                    Some((*a, *b))
                }
                _ => None,
            })
            .collect();
        assert_eq!(low.len(), 2, "{low:?}");
        assert!(
            low.iter().all(|(a, b)| {
                let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
                x1 <= 4.86 || x0 >= 5.14
            }),
            "{low:?}"
        );
        // the far side runs on whole
        assert_eq!(
            items
                .iter()
                .filter(|it| matches!(it, PlanItem::Line { id, a, .. } if *id == mid && a.y > 0.1))
                .count(),
            1
        );
    }

    use super::*;

    fn cut_footings(model: &Model, level: &str, cut: f64) -> usize {
        let level = model
            .iter()
            .find(|e| e.name == level && matches!(e.kind, ElementKind::Level(_)))
            .unwrap()
            .id;
        let footings: std::collections::HashSet<ElementId> = plan_view(model, level, cut)
            .items
            .iter()
            .map(|it| it.id())
            .filter(|id| {
                matches!(
                    model.get(*id).map(|e| &e.kind),
                    Some(ElementKind::Footing(_))
                )
            })
            .collect();
        footings.len()
    }

    #[test]
    fn where_two_floors_meet_in_a_door_the_line_is_very_thin_dashed() {
        use creabim_core::{Element, OpeningKind, Transaction};
        let mut m = creabim_core::sample::sample_house();
        // the inner wall with a door in it, running along y
        let (w, door) = m
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::Wall(w) if w.curtain.is_none() => m
                    .dependents(e.id)
                    .into_iter()
                    .find(|o| {
                        matches!(m.get(*o).map(|x| &x.kind),
                            Some(ElementKind::Opening(o)) if o.kind == OpeningKind::Door)
                    })
                    .filter(|_| {
                        (w.start.x - w.end.x).abs() < 1e-9 && w.start.x > 0.5 && w.start.x < 9.5
                    })
                    .map(|d| (w.clone(), d)),
                _ => None,
            })
            .expect("the sample's inner wall has a door");
        let x = w.start.x;
        let floor = |x0: f64, x1: f64| {
            let mut s = creabim_core::Slab::new(
                w.level,
                vec![
                    DVec2::new(x0, 0.0),
                    DVec2::new(x1, 0.0),
                    DVec2::new(x1, 6.0),
                    DVec2::new(x0, 6.0),
                ],
                0.1,
            );
            s.top_offset = 0.0;
            Element::new("Floor", ElementKind::Slab(s))
        };
        Transaction::new("floors")
            .insert(floor(0.0, x))
            .insert(floor(x, 10.0))
            .apply(&mut m)
            .unwrap();
        let items = plan_view(&m, w.level, 1.2).items;
        let dashed: Vec<_> = items
            .iter()
            .filter_map(|it| match it {
                PlanItem::Line {
                    pen: Pen::Drafted(l),
                    a,
                    b,
                    ..
                } if *l == FLOOR_CHANGE => Some((*a, *b)),
                _ => None,
            })
            .collect();
        // (the entrance door's own, on the outer wall's axis, aside)
        let dashed: Vec<_> = dashed
            .into_iter()
            .filter(|(a, b)| (a.x - x).abs() < 1e-6 && (b.x - x).abs() < 1e-6)
            .collect();
        assert_eq!(dashed.len(), 1, "one floor change line at {door:?}");
        // nothing solid left along the axis inside the door
        let solid = items.iter().any(|it| match it {
            PlanItem::Line {
                pen: Pen::Projection,
                a,
                b,
                ..
            } => {
                let m = (*a + *b) / 2.0;
                (a.x - x).abs() < 1e-6
                    && (b.x - x).abs() < 1e-6
                    && dashed
                        .iter()
                        .any(|(p, q)| m.y > p.y.min(q.y) + 1e-6 && m.y < p.y.max(q.y) - 1e-6)
            }
            _ => false,
        });
        assert!(!solid);
    }

    #[test]
    fn a_wall_attached_under_the_cut_is_drawn_in_projection() {
        use creabim_core::{Document, Element, Transaction};
        let m = creabim_core::sample::sample_house();
        let (wid, w) = m
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::Wall(w) if w.curtain.is_none() => Some((e.id, w.clone())),
                _ => None,
            })
            .unwrap();
        let cut = |m: &Model| {
            plan_view(m, w.level, 1.2).items.iter().any(|it| {
                it.id() == wid
                    && matches!(
                        it,
                        PlanItem::Polygon { pen: Pen::Cut, .. } | PlanItem::Fill { .. }
                    )
            })
        };
        assert!(cut(&m), "a full-height wall is cut at 1.2 m");
        // a slab over the whole wall with its underside at 1 m, and the
        // wall's top attached to it
        let (lo, hi) = (w.start.min(w.end), w.start.max(w.end));
        let pad = DVec2::splat(1.0);
        let mut slab = creabim_core::Slab::new(
            w.level,
            vec![
                lo - pad,
                DVec2::new(hi.x + 1.0, lo.y - 1.0),
                hi + pad,
                DVec2::new(lo.x - 1.0, hi.y + 1.0),
            ],
            0.2,
        );
        slab.top_offset = 1.2;
        let se = Element::new("Low slab", ElementKind::Slab(slab));
        let sid = se.id;
        let mut d = Document::from_model(m);
        d.commit(Transaction::new("slab").insert(se)).unwrap();
        d.edit(wid, "attach", |e| {
            if let ElementKind::Wall(w) = &mut e.kind {
                w.attach_top = vec![sid];
            }
        })
        .unwrap();
        assert!(!cut(d.model()), "its top is under the cut");
        assert!(plan_view(d.model(), w.level, 1.2).items.iter().any(|it| {
            it.id() == wid
                && matches!(
                    it,
                    PlanItem::Polygon {
                        pen: Pen::Projection,
                        ..
                    }
                )
        }));
    }

    #[test]
    fn a_level_put_in_a_storey_cuts_the_walls_standing_there() {
        use creabim_core::{Document, Element, Level, Transaction};
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let mid = Element::new(
            "Level 2",
            ElementKind::Level(Level {
                elevation: 2.584,
                height: 3.0,
            }),
        );
        let id = mid.id;
        doc.commit(Transaction::new("level").insert(mid)).unwrap();
        let m = doc.model();
        let ground: Vec<ElementId> = m
            .iter()
            .filter(|e| {
                matches!(&e.kind, ElementKind::Wall(w)
                    if m.level(w.level).is_some_and(|l| l.elevation == 0.0))
            })
            .map(|e| e.id)
            .collect();
        assert!(!ground.is_empty());
        let cut = |v: &PlanView| {
            v.items
                .iter()
                .any(|i| ground.contains(&i.id()) && matches!(i, PlanItem::Fill { .. }))
        };
        // 200 mm above it the ground floor's walls (3 m tall) are cut
        assert!(cut(&plan_view(m, id, 0.2)));
        // 1.2 m above it the cut passes over them: seen from above
        let high = plan_view(m, id, 1.2);
        assert!(!cut(&high));
        assert!(high.items.iter().any(|i| ground.contains(&i.id())
            && matches!(
                i,
                PlanItem::Polygon {
                    pen: Pen::Projection,
                    ..
                }
            )));
        // the level above keeps its own walls only: those below end at it
        let top = m.levels().find(|l| l.1.elevation == 3.0).unwrap().0.id;
        assert!(!plan_view(m, top, 1.2)
            .items
            .iter()
            .any(|i| ground.contains(&i.id())));
    }

    #[test]
    fn a_foundation_level_plan_cuts_the_footings_under_the_floor_above() {
        let m = creabim_core::starter::starter_house();
        // 0.25 above the foundation level is 0.55 below the ground floor:
        // inside every footing (top -0.35, 0.45 deep)
        assert_eq!(cut_footings(&m, "Foundation", 0.25), 16);
        // a cut above their tops passes none of them
        assert_eq!(cut_footings(&m, "Foundation", 0.6), 0);
    }

    #[test]
    fn a_ground_storey_column_reaching_into_the_footings_shows_above_the_cut() {
        let mut m = creabim_core::starter::starter_house();
        let found = m
            .iter()
            .find(|e| e.name == "Foundation" && matches!(e.kind, ElementKind::Level(_)))
            .unwrap()
            .id;
        let fz = m.level(found).unwrap().elevation;
        let ground = m
            .levels()
            .filter(|(_, l)| l.elevation > fz + 1e-6)
            .min_by(|a, b| a.1.elevation.total_cmp(&b.1.elevation))
            .unwrap()
            .0
            .id;
        let gz = m.level(ground).unwrap().elevation;
        // its foot down at the foundation level, through the plan's cut
        let mut c = creabim_core::Column::new(
            ground,
            DVec2::new(-3.0, -3.0),
            creabim_core::Profile::Rectangle {
                width: 0.25,
                depth: 0.25,
            },
        );
        c.base_offset = fz - gz;
        let ce = creabim_core::Element::new("C", ElementKind::Column(c));
        let cid = ce.id;
        creabim_core::Transaction::new("c")
            .insert(ce)
            .apply(&mut m)
            .unwrap();
        let items = plan_view(&m, found, 0.25).items;
        let lines = items
            .iter()
            .filter(|it| matches!(it, PlanItem::Line { pen: Pen::Overhead, id, .. } if *id == cid))
            .count();
        assert_eq!(lines, 4);
        assert!(!items.iter().any(|it| matches!(it,
            PlanItem::Polygon { pen: Pen::Cut, id, .. } if *id == cid)));
    }

    #[test]
    fn a_foundation_plan_draws_footings_bare_their_steps_and_the_wall_cores() {
        let mut m = creabim_core::starter::starter_house();
        let level = m
            .iter()
            .find(|e| e.name == "Foundation" && matches!(e.kind, ElementKind::Level(_)))
            .unwrap()
            .id;
        let footing = |m: &Model, id: ElementId| {
            matches!(m.get(id).map(|e| &e.kind), Some(ElementKind::Footing(_)))
        };
        let items = plan_view(&m, level, 0.25).items;
        // in outline only: no hatch, no colour
        assert!(items.iter().all(|it| !matches!(it,
            PlanItem::Fill { id, hatch, rgb, .. }
                if footing(&m, *id) && (*hatch != Hatch::Empty || rgb.is_some()))));
        // the walls on them, their cores a dash and two dots
        let cores = items
            .iter()
            .filter(|it| {
                matches!(it, PlanItem::Line { pen: Pen::Overhead, id, .. }
                if matches!(m.get(*id).map(|e| &e.kind), Some(ElementKind::Wall(_))))
            })
            .count();
        // none drawn by itself (George 2026-11-20)
        assert_eq!(cores, 0);
        // the masonry drawn on the footings with the Wall tool, standing
        // above the cut under the floor: its cores a dash and two dots
        let fz = m.level(level).unwrap().elevation;
        let on: Vec<(f64, DVec2, DVec2)> = m
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Footing(f)
                    if f.host.is_some() && f.kind == creabim_core::FootingKind::Strip =>
                {
                    let (a, b) = f.line(&m);
                    Some((f.top(&m), a, b))
                }
                _ => None,
            })
            .collect();
        let mut tx = creabim_core::Transaction::new("dt");
        for (top, a, b) in on {
            let mut w = creabim_core::Wall::new(level, a, b, 0.3, 0.3);
            w.base_offset = top - fz;
            w.height = -top;
            tx = tx.insert(creabim_core::Element::new("DT", ElementKind::Wall(w)));
        }
        tx.apply(&mut m).unwrap();
        let items = plan_view(&m, level, 0.25).items;
        let cores = items
            .iter()
            .filter(|it| {
                matches!(it, PlanItem::Line { pen: Pen::Overhead, id, .. }
                if matches!(m.get(*id).map(|e| &e.kind), Some(ElementKind::Wall(_))))
            })
            .count();
        assert!(cores >= 8, "{cores}");
        assert!(!items.iter().any(|it| matches!(
            it,
            PlanItem::Line {
                pen: Pen::HiddenCut,
                ..
            }
        )));
        // one footing set deeper: a heavy dashed line where it steps
        let one = m
            .iter()
            .find(|e| {
                matches!(&e.kind, ElementKind::Footing(f) if f.host.is_some()
                && f.kind == creabim_core::FootingKind::Strip)
            })
            .unwrap()
            .clone();
        let mut deeper = one.clone();
        if let ElementKind::Footing(f) = &mut deeper.kind {
            f.thickness += 0.2;
        }
        creabim_core::Transaction::new("deeper")
            .replace(one, deeper)
            .apply(&mut m)
            .unwrap();
        let steps = plan_view(&m, level, 0.25)
            .items
            .iter()
            .filter(|it| {
                matches!(
                    it,
                    PlanItem::Line {
                        pen: Pen::HiddenCut,
                        ..
                    }
                )
            })
            .count();
        assert!(steps >= 1, "{steps}");
        if let Ok(f) = std::env::var("CREABIM_FOUNDATION_SVG") {
            std::fs::write(f, plan_view(&m, level, 0.25).to_svg(40.0)).unwrap();
        }
    }

    fn named(m: &Model, name: &str) -> ElementId {
        m.iter().find(|e| e.name == name).expect(name).id
    }

    fn view_items(m: &Model, view: &str) -> Vec<PlanItem> {
        crate::section::view_drawing(m, named(m, view))
            .unwrap()
            .items
    }

    /// How many closed outlines a set of segments makes, joined end to
    /// end; every corner must be met by an even number of them.
    fn loops(segs: &[(DVec2, DVec2)]) -> usize {
        type Key = (i64, i64);
        let key = |p: DVec2| ((p.x * 1e4).round() as i64, (p.y * 1e4).round() as i64);
        fn find(p: &mut std::collections::HashMap<Key, Key>, k: Key) -> Key {
            let up = *p.entry(k).or_insert(k);
            if up == k {
                k
            } else {
                let r = find(p, up);
                p.insert(k, r);
                r
            }
        }
        let mut parent: std::collections::HashMap<Key, Key> = Default::default();
        let mut degree: std::collections::HashMap<Key, usize> = Default::default();
        for (a, b) in segs {
            let (ka, kb) = (key(*a), key(*b));
            *degree.entry(ka).or_default() += 1;
            *degree.entry(kb).or_default() += 1;
            let (ra, rb) = (find(&mut parent, ka), find(&mut parent, kb));
            parent.insert(ra, rb);
        }
        assert!(degree.values().all(|d| d % 2 == 0), "an open outline");
        let keys: Vec<Key> = parent.keys().copied().collect();
        let roots: std::collections::HashSet<Key> =
            keys.into_iter().map(|k| find(&mut parent, k)).collect();
        roots.len()
    }

    #[test]
    fn a_thick_line_only_where_the_cut_meets_air() {
        use creabim_core::{Element, Level, Material, Transaction, Wall};
        let mut m = Model::new();
        let lvl = Element::new(
            "L",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let brick = Element::new(
            "Brick",
            ElementKind::Material(Material::new(Default::default(), [0.8, 0.4, 0.3])),
        );
        let concrete = Element::new(
            "Concrete",
            ElementKind::Material(Material::new(Default::default(), [0.6, 0.6, 0.6])),
        );
        let mut w1 = Wall::new(lvl.id, DVec2::ZERO, DVec2::new(2.0, 0.0), 0.3, 2.7);
        w1.material = Some(brick.id);
        let mut w2 = Wall::new(lvl.id, DVec2::new(2.0, 0.0), DVec2::new(4.0, 0.0), 0.3, 2.7);
        w2.material = Some(concrete.id);
        let (e1, e2) = (
            Element::new("W1", ElementKind::Wall(w1)),
            Element::new("W2", ElementKind::Wall(w2)),
        );
        let (i1, i2) = (e1.id, e2.id);
        Transaction::new("set up")
            .insert(lvl)
            .insert(brick)
            .insert(concrete)
            .insert(e1)
            .insert(e2)
            .apply(&mut m)
            .unwrap();
        let rect = |x0: f64, x1: f64| {
            vec![
                DVec2::new(x0, -0.15),
                DVec2::new(x1, -0.15),
                DVec2::new(x1, 0.15),
                DVec2::new(x0, 0.15),
            ]
        };
        let mut items = vec![
            PlanItem::Polygon {
                id: i1,
                pen: Pen::Cut,
                points: rect(0.0, 2.0),
            },
            PlanItem::Polygon {
                id: i2,
                pen: Pen::Cut,
                points: rect(2.0, 4.0),
            },
        ];
        clean_up_cuts(&m, &mut items);
        // the joint between brick and concrete at x = 2: one thin line
        let at_joint = |pen: Pen| {
            items
                .iter()
                .filter(|it| {
                    matches!(it, PlanItem::Line { pen: p, a, b, .. }
                        if *p == pen && (a.x - 2.0).abs() < 1e-9 && (b.x - 2.0).abs() < 1e-9)
                })
                .count()
        };
        assert_eq!(at_joint(Pen::Cut), 0, "no thick line inside the cut");
        assert_eq!(
            at_joint(Pen::Projection),
            1,
            "a thin line between materials"
        );
        // the faces against air stay thick
        assert!(items
            .iter()
            .any(|it| matches!(it, PlanItem::Line { pen: Pen::Cut, a, b, .. }
            if (a.y + 0.15).abs() < 1e-9 && (b.y + 0.15).abs() < 1e-9)));
    }

    #[test]
    fn footings_meeting_at_corners_merge_into_one_outline() {
        let m = creabim_core::starter::starter_house();
        let items = view_items(&m, "Foundation plan");
        let footing =
            |id: &ElementId| matches!(m.get(*id).map(|e| &e.kind), Some(ElementKind::Footing(_)));
        let fills: Vec<&Vec<DVec2>> = items
            .iter()
            .filter_map(|it| match it {
                PlanItem::Fill { id, points, .. } if footing(id) => Some(points),
                _ => None,
            })
            .collect();
        let cut: Vec<(DVec2, DVec2)> = items
            .iter()
            .filter_map(|it| match it {
                PlanItem::Line {
                    id,
                    pen: Pen::Cut,
                    a,
                    b,
                } if footing(id) => Some((*a, *b)),
                _ => None,
            })
            .collect();
        // no outline runs inside another footing: no overlap squares
        for (a, b) in &cut {
            let mid = (*a + *b) / 2.0;
            for f in &fills {
                assert!(
                    !(point_in(mid, f) && edge_distance(mid, f) > 1e-3),
                    "a footing edge {a} {b} runs inside another"
                );
            }
        }
        // the strips make one network: an outline round its outside and
        // one round each bay inside it; each pad stands alone
        let pads = m
            .iter()
            .filter(|e| {
                matches!(&e.kind, ElementKind::Footing(f)
                    if f.kind == creabim_core::FootingKind::Pad)
            })
            .count();
        let n = loops(&cut);
        // (the starter house has seven bays between its strips)
        assert_eq!(n, 1 + 7 + pads, "{n} outlines, {pads} pads");
    }

    #[test]
    fn an_l_shaped_hole_is_crossed_in_each_leg_inside_its_outline() {
        let l = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(1.0, 0.0),
            DVec2::new(1.0, 3.0),
            DVec2::new(3.0, 3.0),
            DVec2::new(3.0, 4.0),
            DVec2::new(0.0, 4.0),
        ];
        let pieces = convex_pieces(&l);
        assert_eq!(pieces.len(), 2);
        let area: f64 = pieces.iter().map(|p| signed_area(p).abs()).sum();
        assert!((area - signed_area(&l).abs()).abs() < 1e-9);
        let cross = hole_cross(&l);
        assert_eq!(cross.len(), 4);
        for (a, b) in cross {
            for t in [0.25, 0.5, 0.75] {
                let p = a + (b - a) * t;
                assert!(point_in(p, &l), "{p} is outside the hole");
            }
        }
        // a hole that isn't square splits into convex pieces too
        let tilted: Vec<DVec2> = l
            .iter()
            .map(|p| DVec2::from_angle(0.3).rotate(*p))
            .collect();
        let pieces = convex_pieces(&tilted);
        assert!(pieces.len() >= 2);
        for piece in pieces {
            assert!(is_convex(&piece));
        }
    }

    #[test]
    fn the_first_floor_shows_the_stair_through_its_well() {
        let m = creabim_core::starter::starter_house();
        let stair = named(&m, "Stair");
        let shaft = named(&m, "Stair Well");
        let well = match &m.get(shaft).unwrap().kind {
            ElementKind::Shaft(s) => s.boundary.clone(),
            _ => unreachable!(),
        };
        let items = view_items(&m, "First Floor plan");
        let treads = items
            .iter()
            .filter(
                |it| matches!(it, PlanItem::Line { id, pen: Pen::Projection, .. } if *id == stair),
            )
            .count();
        assert!(treads >= 15, "{treads} lines of the stair");
        // all of it inside the well
        for it in items.iter().filter(|it| it.id() == stair) {
            if let PlanItem::Line { a, b, .. } = it {
                let mid = (*a + *b) / 2.0;
                assert!(point_in(mid, &well) || edge_distance(mid, &well) < 1e-6);
            }
        }
        assert!(items.iter().any(
            |it| matches!(it, PlanItem::Label { id, text, .. } if *id == stair && text == "DN")
        ));
        // the well keeps its outline, with no cross over the stair
        assert!(!items
            .iter()
            .any(|it| matches!(it, PlanItem::Line { id, .. } if *id == shaft)));
        assert!(items
            .iter()
            .any(|it| matches!(it, PlanItem::Polygon { id, .. } if *id == shaft)));
        // a view depth too shallow to reach the stair's foot shows less
        let f = named(&m, "First Floor");
        let shallow = creabim_core::PlanGraphics {
            range: creabim_core::graphics::PlanRange {
                view_depth: Some(1.0),
                ..Default::default()
            },
            ..Default::default()
        };
        let count = |g: &creabim_core::PlanGraphics| {
            plan_view_with(&m, f, 1.2, g)
                .items
                .iter()
                .filter(|it| it.id() == stair)
                .count()
        };
        let (some, all) = (count(&shallow), count(&Default::default()));
        assert!(some > 0 && some < all, "{some} of {all}");
        // on the ground floor the well is only the dashed edge above
        let g = view_items(&m, "Ground Floor plan");
        assert!(g.iter().filter(|it| it.id() == shaft).all(|it| matches!(
            it,
            PlanItem::Line {
                pen: Pen::Hidden,
                ..
            }
        )));
    }

    #[test]
    fn what_lies_below_the_cut_is_tinted() {
        let m = creabim_core::starter::starter_house();
        let g = named(&m, "Ground Floor");
        let shades = |graphics: &creabim_core::PlanGraphics| {
            plan_view_with(&m, g, 1.2, graphics)
                .items
                .iter()
                .filter(|it| matches!(it, PlanItem::Shade { .. }))
                .count()
        };
        let on = shades(&Default::default());
        assert!(on >= 10, "{on}");
        let off = creabim_core::PlanGraphics {
            below_fill: false,
            ..Default::default()
        };
        assert_eq!(shades(&off), 0);
        // tints come first, under every line, and are light
        let items = plan_view(&m, g, 1.2).items;
        let last = items
            .iter()
            .rposition(|it| matches!(it, PlanItem::Shade { .. }))
            .unwrap();
        for it in &items[..=last] {
            let PlanItem::Shade { rgb, .. } = it else {
                panic!("{it:?} under a tint");
            };
            assert!(rgb.iter().all(|c| *c > 0.6), "{rgb:?}");
        }
    }

    #[test]
    fn furniture_is_drawn_lighter_than_walls_and_slabs() {
        let m = creabim_core::starter::starter_house();
        let items = view_items(&m, "Ground Floor plan");
        let objects: std::collections::HashSet<String> = items
            .iter()
            .filter(|it| {
                m.get(it.id())
                    .is_some_and(|e| matches!(e.kind, ElementKind::Object(_)))
            })
            .filter_map(|it| match it {
                PlanItem::Line { pen, .. } | PlanItem::Polygon { pen, .. } => {
                    Some(format!("{pen:?}"))
                }
                _ => None,
            })
            .collect();
        assert!(objects.contains("Fine"));
        assert!(!objects.contains("Projection") && !objects.contains("Cut"));
        let w = creabim_core::LineWeights::for_scale(50.0);
        let width = |p| crate::paper::pen_stroke_with(p, &w).width;
        assert!(width(Pen::Cut) > width(Pen::Projection));
        assert!(width(Pen::Projection) > width(Pen::Symbol));
        assert!(width(Pen::Symbol) > width(Pen::Fine));
        // a view's own weights reach its sheet
        let mut m2 = m.clone();
        let v = named(&m2, "Ground Floor plan");
        let before = m2.get(v).unwrap().clone();
        let mut after = before.clone();
        if let ElementKind::View(view) = &mut after.kind {
            let mut heavy = creabim_core::LineWeights::for_scale(50.0);
            heavy.cut = 1.4;
            view.graphics.weights = Some(heavy);
        }
        creabim_core::Transaction::new("Line weights")
            .replace(before, after)
            .apply(&mut m2)
            .unwrap();
        let sheet = named(&m2, "Ground Floor Plan");
        let svg = crate::sheet::sheet_svg(&m2, sheet, &crate::sheet::SheetInfo::default()).unwrap();
        assert!(svg.contains("stroke-width=\"1.4\""));
    }

    #[test]
    fn an_opening_above_the_cut_is_drawn_dashed_in_its_wall() {
        use creabim_core::{Element, Level, Opening, Transaction, Wall};
        let mut m = Model::new();
        let l = Element::new(
            "Ground",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let level = l.id;
        let w = Element::new(
            "W",
            ElementKind::Wall(Wall::new(
                level,
                DVec2::ZERO,
                DVec2::new(4.0, 0.0),
                0.2,
                3.0,
            )),
        );
        let wall = w.id;
        let duct = Element::new(
            "Duct",
            ElementKind::Opening(Opening {
                host: wall,
                kind: OpeningKind::Hole,
                offset: 1.0,
                sill: 2.2,
                width: 0.4,
                height: 0.3,
                material: None,
                frame: Default::default(),
                flip_hand: false,
                flip_facing: false,
                no_outer_sill: false,
                no_inner_sill: false,
                threshold: false,
                threshold_material: None,
                penetration: None,
                setback: None,
                reveal: None,
                division: None,
                door: None,
                texts: None,
                performance: None,
            }),
        );
        let hole = duct.id;
        Transaction::new("t")
            .insert(l)
            .insert(w)
            .insert(duct)
            .apply(&mut m)
            .unwrap();
        let items = plan_view(&m, level, 1.2).items;
        let dashed: Vec<&PlanItem> = items
            .iter()
            .filter(|it| matches!(it, PlanItem::Line { id, pen: Pen::Overhead, .. } if *id == hole))
            .collect();
        // its outline and, as a penetration, its cross
        assert_eq!(dashed.len(), 6);
        // drawn over the wall's fill, so it shows
        let fill = items
            .iter()
            .position(|it| matches!(it, PlanItem::Fill { id, .. } if *id == wall))
            .unwrap();
        let first = items.iter().position(|it| it.id() == hole).unwrap();
        assert!(first > fill);
        // the wall stays whole at the cut
        assert!(!items
            .iter()
            .any(|it| matches!(it, PlanItem::Line { id, pen: Pen::Symbol, .. } if *id == hole)));
    }

    /// George 2026-11-21: a window whose sill is over the cut plane is
    /// still drawn as cut: a gap in the wall and its frame and glass.
    #[test]
    fn a_lift_and_slide_leaf_is_marked_with_a_dashed_arrow() {
        use creabim_core::joinery::FrameNode;
        use creabim_core::{Element, LeafOp, Level, Opening, Transaction, Wall};
        let marks = |op: LeafOp| {
            let mut m = Model::new();
            let l = Element::new(
                "Ground",
                ElementKind::Level(Level {
                    elevation: 0.0,
                    height: 3.0,
                }),
            );
            let level = l.id;
            let w = Element::new(
                "W",
                ElementKind::Wall(Wall::new(
                    level,
                    DVec2::ZERO,
                    DVec2::new(5.0, 0.0),
                    0.3,
                    3.0,
                )),
            );
            let win = Element::new(
                "Win",
                ElementKind::Opening(Opening {
                    host: w.id,
                    kind: OpeningKind::Window,
                    offset: 1.0,
                    sill: 0.0,
                    width: 2.4,
                    height: 2.2,
                    material: None,
                    frame: Default::default(),
                    flip_hand: false,
                    flip_facing: false,
                    no_outer_sill: false,
                    no_inner_sill: false,
                    threshold: false,
                    threshold_material: None,
                    penetration: None,
                    setback: None,
                    reveal: None,
                    division: Some(FrameNode::Split {
                        across: true,
                        sizes: vec![1.0, 1.0],
                        parts: vec![FrameNode::leaf(op), FrameNode::leaf(LeafOp::Fixed)],
                    }),
                    door: None,
                    texts: None,
                    performance: None,
                }),
            );
            let wid = win.id;
            Transaction::new("t")
                .insert(l)
                .insert(w)
                .insert(win)
                .apply(&mut m)
                .unwrap();
            plan_view(&m, level, 1.0)
                .items
                .into_iter()
                .filter_map(|it| match it {
                    PlanItem::Line {
                        id,
                        pen: Pen::Hidden,
                        a,
                        b,
                    } if id == wid => Some((a, b)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert!(marks(LeafOp::TurnLeft).is_empty());
        let left = marks(LeafOp::LiftSlideLeft);
        assert_eq!(left.len(), 3, "{left:?}");
        // the arrow along the wall, over one half of the window, off
        // the wall's middle
        let (a, b) = left[0];
        assert!((a.y - b.y).abs() < 1e-9 && a.y.abs() > 0.05, "{a} {b}");
        assert!((b.x - a.x).abs() > 0.3);
        // the other hand slides the other way
        let (c, d) = marks(LeafOp::LiftSlideRight)[0];
        assert!((b.x - a.x).signum() != (d.x - c.x).signum());
    }

    #[test]
    fn a_high_window_is_drawn_as_cut() {
        use creabim_core::{Element, Level, Opening, Transaction, Wall};
        let mut m = Model::new();
        let l = Element::new(
            "Ground",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let level = l.id;
        let w = Element::new(
            "W",
            ElementKind::Wall(Wall::new(
                level,
                DVec2::ZERO,
                DVec2::new(4.0, 0.0),
                0.3,
                3.0,
            )),
        );
        let wall = w.id;
        let win = Element::new(
            "Win",
            ElementKind::Opening(Opening {
                host: wall,
                kind: OpeningKind::Window,
                offset: 1.0,
                sill: 1.5,
                width: 1.2,
                height: 0.9,
                material: None,
                frame: Default::default(),
                flip_hand: false,
                flip_facing: false,
                no_outer_sill: false,
                no_inner_sill: false,
                threshold: false,
                threshold_material: None,
                penetration: None,
                setback: None,
                reveal: None,
                division: None,
                door: None,
                texts: None,
                performance: None,
            }),
        );
        let wid = win.id;
        Transaction::new("t")
            .insert(l)
            .insert(w)
            .insert(win)
            .apply(&mut m)
            .unwrap();
        let items = plan_view(&m, level, 1.2).items;
        // its frame and glass, not a dashed outline
        assert!(items.iter().any(|it| it.id() == wid
            && !matches!(
                it,
                PlanItem::Line {
                    pen: Pen::Overhead,
                    ..
                }
            )));
        assert!(!items
            .iter()
            .any(|it| matches!(it, PlanItem::Line { id, pen: Pen::Overhead, .. } if *id == wid)));
        // the wall's cut fill stops at the window
        let span = |pts: &[DVec2]| {
            let xs = pts.iter().map(|p| p.x);
            (
                xs.clone().fold(f64::MAX, f64::min),
                xs.fold(f64::MIN, f64::max),
            )
        };
        let fills: Vec<(f64, f64)> = items
            .iter()
            .filter_map(|it| match it {
                PlanItem::Fill { id, points, .. } if *id == wall => Some(span(points)),
                _ => None,
            })
            .collect();
        assert!(!fills.is_empty());
        assert!(
            fills
                .iter()
                .all(|(a, b)| *b <= 1.0 + 1e-6 || *a >= 2.2 - 1e-6),
            "{fills:?}"
        );
    }

    /// A slab layer's edges moved by hand show in plan (A7-10): the top
    /// layer pushed out 500 mm reaches 500 mm past the slab's own edge.
    #[test]
    fn slab_layer_edits_show_in_plan() {
        let m = creabim_core::starter::starter_house();
        let (id, level, n) = m
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::Slab(s) => {
                    let (_, a) = creabim_core::assembly::assembly_of(&m, e)?;
                    (a.layers.len() > 1).then_some((e.id, s.level, s.boundary.len()))
                }
                _ => None,
            })
            .expect("a layered slab");
        let far = |m: &Model| {
            plan_view(m, level, 1.2)
                .items
                .iter()
                .filter(|it| it.id() == id)
                .filter_map(|it| match it {
                    PlanItem::Polygon { points, .. } => {
                        Some(points.iter().map(|p| p.x).fold(f64::MIN, f64::max))
                    }
                    // edges crossing a door come as lines
                    PlanItem::Line { a, b, .. } => Some(a.x.max(b.x)),
                    _ => None,
                })
                .fold(f64::MIN, f64::max)
        };
        let before = far(&m);
        let mut e = m.get(id).unwrap().clone();
        if let ElementKind::Slab(s) = &mut e.kind {
            for edge in 0..n {
                creabim_core::set_layer_edge(&mut s.layer_edges, 0, edge, 0.5);
            }
        }
        let mut doc = creabim_core::Document::from_model(m.clone());
        let old = m.get(id).unwrap().clone();
        doc.commit(creabim_core::Transaction::new("push").replace(old, e))
            .unwrap();
        let after = far(doc.model());
        assert!(
            (after - before - 0.5).abs() < 1e-6,
            "the plan's furthest slab edge moved {:.3} m, wanted 0.5",
            after - before
        );
    }

    /// The starter house's Ground Floor plan hides its footings; shown
    /// again (the "Show footings" button under its view depth, A7-3) they
    /// draw dashed under the floor.
    #[test]
    fn shown_footings_draw_dashed_under_the_floor() {
        let m = creabim_core::starter::starter_house();
        let footings: std::collections::HashSet<ElementId> = m
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Footing(_)))
            .map(|e| e.id)
            .collect();
        let plan = m
            .iter()
            .find(|e| e.name == "Ground Floor plan" && matches!(e.kind, ElementKind::View(_)))
            .unwrap()
            .clone();
        let dashed = |m: &Model| {
            crate::section::view_drawing_with(m, plan.id, false)
                .unwrap()
                .items
                .iter()
                .filter(|it| {
                    footings.contains(&it.id())
                        && matches!(
                            it,
                            PlanItem::Line {
                                pen: Pen::Hidden,
                                ..
                            }
                        )
                })
                .count()
        };
        assert_eq!(dashed(&m), 0, "hidden in the view");
        let mut shown = plan.clone();
        if let ElementKind::View(v) = &mut shown.kind {
            v.hidden
                .categories
                .retain(|c| *c != creabim_core::visibility::Category::Footings);
        }
        let mut doc = creabim_core::Document::from_model(m.clone());
        doc.commit(creabim_core::Transaction::new("show").replace(plan.clone(), shown.clone()))
            .unwrap();
        // under the bottom of the plan's range (its floor): still left out
        // (George 2026-11-20)
        assert_eq!(dashed(doc.model()), 0, "under the range's bottom");
        // the bottom lowered under them: dashed under the floor
        let mut deeper = shown.clone();
        if let ElementKind::View(v) = &mut deeper.kind {
            let ElementKind::View(p) = &plan.kind else {
                unreachable!()
            };
            let creabim_core::ViewKind::Plan { level, .. } = p.kind else {
                unreachable!()
            };
            v.graphics.range.bottom = Some(creabim_core::graphics::RangeLimit {
                level,
                offset: -2.0,
            });
        }
        doc.commit(creabim_core::Transaction::new("deeper").replace(shown, deeper))
            .unwrap();
        assert!(dashed(doc.model()) > 20, "shown, dashed under the floor");
    }

    /// A closed sketch draws as a face (a wash inside its outline); an
    /// open one, or a closed one that crosses itself, as lines only (A7-1).
    #[test]
    fn a_closed_sketch_is_washed_as_a_face() {
        let plane = creabim_core::Workplane::ground(0.0);
        let square = vec![
            DVec2::ZERO,
            DVec2::new(2.0, 0.0),
            DVec2::new(2.0, 2.0),
            DVec2::new(0.0, 2.0),
        ];
        let bow = vec![
            DVec2::ZERO,
            DVec2::new(2.0, 2.0),
            DVec2::new(2.0, 0.0),
            DVec2::new(0.0, 2.0),
        ];
        let washed = |pts: Vec<DVec2>, closed: bool| {
            let sk = creabim_core::Sketch::polyline(None, plane, pts, closed);
            sketch_items(ElementId::NIL, &sk, &|p| p)
                .iter()
                .any(|it| matches!(it, PlanItem::Tint { .. }))
        };
        assert!(washed(square.clone(), true));
        assert!(!washed(square, false));
        assert!(!washed(bow, true));
    }

    #[test]
    fn a_beam_on_a_roof_slope_is_filled_in_its_colour_in_the_plan() {
        let lv = creabim_core::Element::new(
            "L1",
            ElementKind::Level(creabim_core::Level {
                elevation: 3.0,
                height: 3.0,
            }),
        );
        let mut b = creabim_core::Beam::new(lv.id, DVec2::ZERO, DVec2::new(4.0, 0.0));
        // rising through the cut (6 m) from 5.6 to 7.6
        b.top_offset = 0.4;
        b.rise = [0.0, 2.0];
        let be = creabim_core::Element::new("Beam", ElementKind::Beam(b));
        let id = be.id;
        let m = Model::from_elements([lv.clone(), be]).unwrap();
        let g = creabim_core::PlanGraphics {
            below_fill: true,
            below_strength: 1.0,
            ..Default::default()
        };
        let v = plan_view_with(&m, lv.id, 3.0, &g);
        assert!(
            v.items
                .iter()
                .any(|it| matches!(it, PlanItem::Shade { id: i, .. } if *i == id)),
            "the beam has its fill"
        );
    }

    #[test]
    fn pieces_of_one_batt_layer_fill_as_one_outline() {
        let p = |x: f64, y: f64| DVec2::new(x, y);
        // a layer either side of a wall's top, and the two bits over it
        let left = vec![
            p(0.0, 0.0),
            p(1.0, 0.0),
            p(1.0, 0.02),
            p(1.0, 0.05),
            p(0.0, 0.05),
        ];
        let over = vec![p(1.0, 0.02), p(1.2, 0.02), p(1.2, 0.05), p(1.0, 0.05)];
        let right = vec![
            p(1.2, 0.0),
            p(2.0, 0.0),
            p(2.0, 0.05),
            p(1.2, 0.05),
            p(1.2, 0.02),
        ];
        let one = super::union_touching(&[&left, &over, &right], 1e-5).expect("one outline");
        let area: f64 = (0..one.len())
            .map(|i| one[i].perp_dot(one[(i + 1) % one.len()]))
            .sum::<f64>()
            / 2.0;
        assert!((area - (0.1 - 0.2 * 0.02)).abs() < 1e-9, "{area} {one:?}");
        // two apart make no one outline
        let far = vec![p(5.0, 0.0), p(6.0, 0.0), p(6.0, 0.05), p(5.0, 0.05)];
        assert!(super::union_touching(&[&left, &far], 1e-5).is_none());
    }
}
