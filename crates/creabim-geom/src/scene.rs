use crate::{triangulate::triangulate, wall::wall_mesh, TriMesh};
use creabim_core::{assembly, csg, BoolOp, Element, ElementId, ElementKind, Model, Opening};
use glam::{DVec2, DVec3};

/// Roughness, metallic and opacity of an element without a material.
pub const PLAIN_SURFACE: [f32; 3] = [creabim_core::element::DEFAULT_ROUGHNESS, 0.0, 1.0];

/// One element's renderable geometry.
#[derive(Clone, Debug)]
pub struct SceneItem {
    pub id: ElementId,
    pub mesh: TriMesh,
    pub color: [f32; 3],
    /// Roughness, metallic and opacity, from the element's material.
    pub surface: [f32; 3],
    /// A cutter: drawn as a see-through outline in 3D, left out of
    /// drawings and exports, but still clickable.
    pub ghost: bool,
    /// For one layer of a layered wall, slab or roof: which layer. A
    /// layered element is one item per layer, all with its id.
    pub layer: Option<LayerPart>,
    /// A look of its own over the element's material, by name: a
    /// window's glass, a door's handle.
    pub finish: Option<&'static str>,
}

/// Which layer of its element's assembly a scene item is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerPart {
    pub index: usize,
    pub material: Option<ElementId>,
    pub function: creabim_core::LayerFunction,
    pub core: bool,
    /// Its join priority: lower runs through higher where elements meet.
    pub priority: u8,
}

impl LayerPart {
    pub fn of(b: &assembly::Band) -> Self {
        Self {
            index: b.index,
            material: b.material,
            function: b.function,
            core: b.core,
            priority: b.priority,
        }
    }
}

/// Default colours for layers with no material, by function.
fn layer_color(f: creabim_core::LayerFunction, plain: [f32; 3]) -> [f32; 3] {
    use creabim_core::LayerFunction as F;
    match f {
        F::Insulation => [0.93, 0.86, 0.45],
        F::Membrane => [0.25, 0.27, 0.30],
        F::Finish => [0.92, 0.91, 0.88],
        _ => plain,
    }
}

/// The solid of each layer of a layered element, in model coordinates,
/// air gaps left out: what an export writes as the element's parts.
/// Empty for an element that isn't layered.
pub fn layer_parts(
    model: &Model,
    e: &Element,
    joins: &std::collections::HashMap<ElementId, creabim_core::WallEnds>,
) -> Vec<(LayerPart, TriMesh)> {
    layer_items(model, e, joins, [0.8; 3])
        .unwrap_or_default()
        .into_iter()
        .filter(|i| !i.is_lining())
        .filter_map(|i| Some((i.layer?, i.mesh)))
        .collect()
}

/// The solid of each layer of a layered element, with its look; air gaps
/// are left empty. `None` for an element that isn't layered.
fn layer_items(
    model: &Model,
    e: &Element,
    joins: &std::collections::HashMap<ElementId, creabim_core::WallEnds>,
    plain: [f32; 3],
) -> Option<Vec<SceneItem>> {
    let (_, a) = assembly::assembly_of(model, e)?;
    let mut linings: Vec<SceneItem> = Vec::new();
    let parts: Vec<(assembly::Band, TriMesh)> = match &e.kind {
        ElementKind::Wall(w) => {
            let level = model.level(w.level)?;
            // its doors and windows, and the curtain walls set in it
            let owned = creabim_core::curtain::wall_openings(model, e.id);
            let openings: Vec<&Opening> = owned.iter().collect();
            let ends = joins.get(&e.id).cloned().unwrap_or_default();
            let top = creabim_core::walls::top_profile(model, w);
            linings = reveal_linings(
                model,
                e.id,
                w,
                &openings,
                &assembly::wall_bands(a, w),
                level.elevation,
                plain,
            );
            let mut parts: Vec<(assembly::Band, TriMesh)> = assembly::wall_bands(a, w)
                .into_iter()
                .map(|b| {
                    let m = crate::wall::wall_layer_mesh_v(
                        w,
                        level.elevation,
                        &openings,
                        &ends,
                        top.as_ref(),
                        (b.s0, b.s1),
                        b.core,
                        b.ext,
                        b.vext,
                    );
                    (b, m)
                })
                .collect();
            // a round penetration: its square's corners filled back, each
            // layer's a solid of its own beside the layer's (sections
            // slice each solid on its own)
            let fillers: Vec<(assembly::Band, TriMesh)> = parts
                .iter()
                .map(|(b, _)| {
                    let mut m = TriMesh::default();
                    for o in &owned {
                        m.append(&crate::opening::round_fillers(
                            o,
                            w,
                            level.elevation,
                            b.s0,
                            b.s1,
                        ));
                    }
                    (*b, m)
                })
                .filter(|(_, m)| !m.is_empty())
                .collect();
            parts.extend(fillers);
            // each layer's reveals are part of its solid: one solid a layer
            for (b, m) in reveal_wrap_parts(w, a, &openings, level.elevation) {
                if let Some((_, mesh)) = parts.iter_mut().find(|(p, _)| p.index == b.index) {
                    mesh.append(&m);
                }
            }
            // the ETICS fire barriers: bands of mineral wool cut out of its
            // burning insulation, a part of their own in that layer
            let barriers = crate::etics::barriers(model);
            if let Some(wb) = barriers.walls.get(&e.id) {
                if let Some(k) = parts.iter().position(|(p, _)| p.index == wb.layer) {
                    let (band, eps) = parts[k].clone();
                    let d = w.direction();
                    let n = DVec2::new(-d.y, d.x);
                    let mut cutter = TriMesh::default();
                    for &[u0, u1, z0, z1] in &wb.boxes {
                        let p = |j: usize| {
                            let u = if j & 1 == 0 { u0 } else { u1 };
                            let z = if j & 2 == 0 { z0 } else { z1 };
                            let s = if j & 4 == 0 {
                                band.s0 - 0.01
                            } else {
                                band.s1 + 0.01
                            };
                            (w.start + d * u + n * s).extend(z)
                        };
                        cuboid(
                            &mut cutter,
                            [p(0), p(1), p(2), p(3), p(4), p(5), p(6), p(7)],
                        );
                    }
                    // a layer wholly in one box (a storey over 22.5 m) is
                    // all wool: the boolean gives nothing when no faces meet
                    let inside = |p: &[f32; 3]| {
                        let q = DVec2::new(p[0] as f64, p[1] as f64) - w.start;
                        let (u, z) = (q.dot(d), p[2] as f64);
                        wb.boxes.iter().any(|b| {
                            u >= b[0] - 1e-3
                                && u <= b[1] + 1e-3
                                && z >= b[2] - 1e-3
                                && z <= b[3] + 1e-3
                        })
                    };
                    if !eps.positions.is_empty() && eps.positions.iter().all(inside) {
                        parts[k].0.material = barriers.material;
                    } else {
                        let wool = boolean_mesh(&eps, &cutter, BoolOp::Intersection);
                        let rest = boolean_mesh(&eps, &cutter, BoolOp::Difference);
                        // a boolean that came out broken (the pieces not
                        // adding up to the layer, or reaching out of it)
                        // leaves the layer whole rather than wrong
                        let within = |m: &TriMesh| match (m.bounds(), eps.bounds()) {
                            (Some((lo, hi)), Some((a, b))) => {
                                (lo - a).min_element() > -0.01 && (b - hi).min_element() > -0.01
                            }
                            _ => true,
                        };
                        let whole = closed_volume(&eps);
                        let sound = within(&wool)
                            && within(&rest)
                            && (closed_volume(&wool) + closed_volume(&rest) - whole).abs()
                                <= 0.01 * whole.abs().max(1e-6);
                        if !wool.indices.is_empty() && sound {
                            parts[k].1 = rest;
                            let mut b = band;
                            b.material = barriers.material;
                            parts.push((b, wool));
                        }
                    }
                }
            }
            plinth_parts(
                model,
                e,
                w,
                &openings,
                &ends,
                top.as_ref(),
                level.elevation,
                &mut parts,
            );
            parts
        }
        ElementKind::Slab(s) => {
            let level = model.level(s.level)?;
            let top = s.top(level.elevation);
            let holes = creabim_core::slab::holes_of(model, e);
            assembly::depth_bands(a, s.thickness)
                .into_iter()
                .map(|b| {
                    let outline = s.layer_outline(b.index);
                    let m = slab_mesh_with_holes(&outline, &holes, top - b.s0, b.s1 - b.s0);
                    (b, m)
                })
                .collect()
        }
        // a ceiling's build-up (podhľad), layers from the top down
        ElementKind::Ceiling(c) => {
            let outline = c.outline(model)?;
            let (_, top) = c.span(model);
            assembly::depth_bands(a, c.thickness)
                .into_iter()
                .map(|b| (b, slab_mesh(&outline, top - b.s0, b.s1 - b.s0)))
                .collect()
        }
        ElementKind::Roof(r) => {
            let base = r.base(model);
            // layers are square to the slope; upright they are thicker
            let k = r.vertical_thickness() / r.thickness.max(1e-9);
            let tv = r.vertical_thickness();
            let holes = creabim_core::slab::holes_of(model, e);
            assembly::depth_bands(a, r.thickness)
                .into_iter()
                .map(|b| {
                    let m = crate::members::roof_layer_mesh(
                        r,
                        base,
                        tv - b.s1 * k,
                        tv - b.s0 * k,
                        Some(b.index),
                    );
                    (b, crate::members::cut_holes(m, &holes))
                })
                .collect()
        }
        _ => return None,
    };
    Some(
        parts
            .into_iter()
            .filter(|(b, _)| b.function != creabim_core::LayerFunction::AirGap)
            .map(|(b, mesh)| {
                let look = b.material.and_then(|m| match &model.get(m)?.kind {
                    ElementKind::Material(m) => Some((m.color, m.surface())),
                    _ => None,
                });
                let (color, surface) =
                    look.unwrap_or((layer_color(b.function, plain), PLAIN_SURFACE));
                SceneItem {
                    id: e.id,
                    mesh,
                    color,
                    surface,
                    ghost: false,
                    layer: Some(LayerPart::of(&b)),
                    finish: None,
                }
            })
            .chain(linings)
            .collect(),
    )
}

/// How thick the plinth's membrane is drawn.
pub const PLINTH_MEMBRANE: f64 = 0.002;

/// A wall's plinth (sokel, [`creabim_core::plinth`]): below its top the
/// facade insulation gives way to the plinth's XPS (flush, or set back
/// when thinner) and the facade render to the plinth's, both run down
/// past the wall's base when the plinth covers the slab's side; the
/// membrane turned up the masonry behind them. Each a part of the
/// facade insulation's layer, in its own material.
#[allow(clippy::too_many_arguments)]
fn plinth_parts(
    model: &Model,
    e: &Element,
    w: &creabim_core::Wall,
    openings: &[&Opening],
    ends: &creabim_core::WallEnds,
    top: Option<&creabim_core::walls::TopProfile>,
    level_z: f64,
    parts: &mut Vec<(assembly::Band, TriMesh)>,
) {
    use creabim_core::LayerFunction as F;
    let Some(p) = creabim_core::plinth::of_wall(model, w) else {
        return;
    };
    let bands: Vec<assembly::Band> = {
        let mut seen: Vec<usize> = vec![];
        parts
            .iter()
            .filter(|(b, _)| {
                let first = !seen.contains(&b.index);
                seen.push(b.index);
                first
            })
            .map(|(b, _)| *b)
            .collect()
    };
    let (Some(first), Some(last)) = (bands.first(), bands.last()) else {
        return;
    };
    // which way across the wall is out
    let out = if first.s0 + first.s1 < last.s0 + last.s1 {
        -1.0
    } else {
        1.0
    };
    let Some(ins) = bands.iter().find(|b| b.function == F::Insulation).copied() else {
        return;
    };
    let inner = if out < 0.0 { ins.s1 } else { ins.s0 };
    let thick = p.thickness.clamp(0.005, ins.s1 - ins.s0 + 0.2);
    let recess = (ins.s1 - ins.s0) - thick;
    let span = |a: f64, b: f64| (a.min(b), a.max(b));
    let below = creabim_core::plinth::below_base(model, e, w, p);
    let base = level_z + w.base_offset;
    let tops = creabim_core::plinth::tops(model, w, p);
    let d = w.direction();
    let n = DVec2::new(-d.y, d.x);
    let len = w.length();
    // boxes over the plinth's height, a metre past the wall's ends
    let cutter = |lift: f64, s: (f64, f64)| {
        let mut m = TriMesh::default();
        let k = tops.len();
        for (i, &(u0, u1, z)) in tops.iter().enumerate() {
            let u0 = if i == 0 { -1.0 } else { u0 };
            let u1 = if i + 1 == k { len + 1.0 } else { u1 };
            let z1 = (z + lift).max(base - below);
            let pt = |j: usize| {
                let u = if j & 1 == 0 { u0 } else { u1 };
                let z = if j & 2 == 0 { base - below - 1.0 } else { z1 };
                let s = if j & 4 == 0 { s.0 - 0.05 } else { s.1 + 0.05 };
                (w.start + d * u + n * s).extend(z)
            };
            cuboid(
                &mut m,
                [pt(0), pt(1), pt(2), pt(3), pt(4), pt(5), pt(6), pt(7)],
            );
        }
        m
    };
    let whole = span(
        bands.iter().map(|b| b.s0).fold(f64::MAX, f64::min),
        bands.iter().map(|b| b.s1).fold(f64::MIN, f64::max),
    );
    let boxes = cutter(0.0, whole);
    let layer = |b: (f64, f64), vext: [f64; 2], ext: [f64; 2]| {
        crate::wall::wall_layer_mesh_v(w, level_z, openings, ends, top, b, false, ext, vext)
    };
    let mut add: Vec<(assembly::Band, TriMesh)> = vec![];
    // the facade's insulation and render above it; the plinth's below
    for (b, mesh) in parts.iter_mut() {
        let outer = if out < 0.0 {
            b.s1 <= inner + 1e-9
        } else {
            b.s0 >= inner - 1e-9
        };
        if !outer || !(b.function == F::Insulation || b.function == F::Finish) {
            continue;
        }
        let first_of_layer = !add.iter().any(|(a, _)| a.index == b.index);
        *mesh = boolean_mesh(mesh, &boxes, BoolOp::Difference);
        if !first_of_layer {
            continue;
        }
        let (band, material) = if b.index == ins.index {
            (
                span(inner, inner + out * thick),
                p.insulation.or(b.material),
            )
        } else {
            (
                span(b.s0 - out * recess, b.s1 - out * recess),
                p.render.or(b.material),
            )
        };
        let piece = layer(band, [b.vext[0].max(0.0) + below, b.vext[1]], b.ext);
        let piece = boolean_mesh(&piece, &boxes, BoolOp::Intersection);
        if !piece.indices.is_empty() {
            let mut nb = *b;
            nb.s0 = band.0;
            nb.s1 = band.1;
            nb.material = material;
            add.push((nb, piece));
        }
    }
    // the membrane up the masonry, past the top by its lap
    if let Some(m) = p.membrane {
        let band = span(inner, inner + out * PLINTH_MEMBRANE);
        let piece = layer(band, [ins.vext[0].max(0.0) + below, ins.vext[1]], ins.ext);
        let piece = boolean_mesh(&piece, &cutter(p.lap.max(0.0), whole), BoolOp::Intersection);
        if !piece.indices.is_empty() {
            let mut nb = ins;
            nb.s0 = band.0;
            nb.s1 = band.1;
            nb.material = Some(m);
            nb.function = F::Membrane;
            add.push((nb, piece));
        }
    }
    parts.extend(add);
}

/// The [`SceneItem::finish`] of a reveal lining.
pub const REVEAL: &str = "Reveal";

/// How thick a reveal lining is: thin enough to leave the opening its size.
const LINING: f64 = 0.004;

impl SceneItem {
    /// A reveal lining, seen in 3D only: drawings and exports leave it out.
    pub fn is_lining(&self) -> bool {
        self.finish == Some(REVEAL)
    }
}

/// The reveals of a layered wall's doors and windows lined, for the 3D
/// view: each layer's part of the jambs, head and sill in the finish on
/// its side of the wall (render outside, plaster inside), so an opening
/// doesn't show the insulation or the membrane cut open.
fn reveal_linings(
    model: &Model,
    id: ElementId,
    w: &creabim_core::Wall,
    openings: &[&Opening],
    bands: &[assembly::Band],
    level_z: f64,
    plain: [f32; 3],
) -> Vec<SceneItem> {
    use creabim_core::LayerFunction as F;
    let look = |b: &assembly::Band| {
        b.material
            .and_then(|m| match &model.get(m)?.kind {
                ElementKind::Material(m) => Some((m.color, m.surface())),
                _ => None,
            })
            .unwrap_or((layer_color(b.function, plain), PLAIN_SURFACE))
    };
    let finishes: Vec<&assembly::Band> = bands.iter().filter(|b| b.function == F::Finish).collect();
    let d = w.direction();
    let n = DVec2::new(-d.y, d.x);
    let z0 = level_z + w.base_offset;
    let at = |u: f64, v: f64, s: f64| (w.start + d * u + n * s).extend(z0 + v);
    let mut out = Vec::new();
    for o in openings {
        if o.kind == creabim_core::OpeningKind::Hole {
            continue;
        }
        let Some(c) = crate::wall::Cut::of(o, w) else {
            continue;
        };
        for b in bands.iter().filter(|b| b.function != F::AirGap) {
            // a finish shows itself; the rest take the finish nearest them
            let mid = (b.s0 + b.s1) / 2.0;
            let from = if b.function == F::Finish {
                Some(b)
            } else {
                finishes.iter().copied().min_by(|x, y| {
                    ((x.s0 + x.s1) / 2.0 - mid)
                        .abs()
                        .total_cmp(&((y.s0 + y.s1) / 2.0 - mid).abs())
                })
            };
            let (color, surface) = from.map_or((plain, PLAIN_SURFACE), look);
            let mut mesh = TriMesh::default();
            let mut slab = |u0: f64, u1: f64, v0: f64, v1: f64| {
                let p = |i: usize| {
                    at(
                        if i & 1 == 0 { u0 } else { u1 },
                        if i & 2 == 0 { v0 } else { v1 },
                        if i & 4 == 0 { b.s0 } else { b.s1 },
                    )
                };
                cuboid(&mut mesh, [p(0), p(1), p(2), p(3), p(4), p(5), p(6), p(7)]);
            };
            slab(c.u0, c.u0 + LINING, c.v0, c.v1);
            slab(c.u1 - LINING, c.u1, c.v0, c.v1);
            slab(c.u0, c.u1, c.v1 - LINING, c.v1);
            if c.v0 > 1e-6 {
                slab(c.u0, c.u1, c.v0, c.v0 + LINING);
            }
            // one item per finish, for all the wall's openings
            match out.iter_mut().find(|i: &&mut SceneItem| i.color == color) {
                Some(i) => i.mesh.append(&mesh),
                None => out.push(SceneItem {
                    id,
                    mesh,
                    color,
                    surface,
                    ghost: false,
                    layer: None,
                    finish: Some(REVEAL),
                }),
            }
        }
    }
    out
}

/// The layers of a wall turned into its windows' reveals
/// ([`assembly::reveal_wraps`]) as solids, one per layer: rings round
/// the opening, the jambs with the head and (above the floor) the sill
/// between them. A section through a window shows the insulation and the
/// render lapped onto the frame, as the plan does at the jambs.
fn reveal_wrap_parts(
    w: &creabim_core::Wall,
    a: &assembly::Assembly,
    openings: &[&Opening],
    level_z: f64,
) -> Vec<(assembly::Band, TriMesh)> {
    if w.is_arc() {
        return vec![];
    }
    let bands = assembly::wall_bands(a, w);
    let d = w.direction();
    let n = DVec2::new(-d.y, d.x);
    let z0 = level_z + w.base_offset;
    let at = |u: f64, v: f64, s: f64| (w.start + d * u + n * s).extend(z0 + v);
    let mut out: Vec<(assembly::Band, TriMesh)> = vec![];
    for o in openings {
        if !creabim_core::joinery::wraps_reveal(o) {
            continue;
        }
        let Some(c) = crate::wall::Cut::of(o, w) else {
            continue;
        };
        let frame = o.frame.fitted(c.u1 - c.u0, c.v1 - c.v0, w.thickness);
        let span = o.frame_span(w, Some(a), frame.depth);
        let lap = o.reveal.unwrap_or(assembly::REVEAL_INSULATION);
        let wraps = assembly::reveal_wraps(a, w, span, lap);
        // how far the reveals come into the opening, all layers together
        let total = wraps.iter().map(|l| l.under + l.width).fold(0.0, f64::max);
        if total > (c.u1 - c.u0) / 2.0 || total > (c.v1 - c.v0) / 2.0 {
            continue;
        }
        for l in &wraps {
            let Some(band) = bands.get(l.index) else {
                continue;
            };
            let s = |depth: f64| l.face + l.inward * depth;
            // its outline as boxes: (from, to into the opening; from, to
            // across the wall)
            let mut boxes = vec![(0.0, l.under + l.width, s(l.d), s(l.d_in))];
            if l.frame > l.d_in + 1e-6 {
                boxes.push((l.under, l.under + l.width, s(l.d_in), s(l.frame)));
            }
            let mut mesh = TriMesh::default();
            let mut slab = |u: (f64, f64), v: (f64, f64), s: (f64, f64)| {
                let p = |i: usize| {
                    at(
                        if i & 1 == 0 { u.0 } else { u.1 },
                        if i & 2 == 0 { v.0 } else { v.1 },
                        if i & 4 == 0 { s.0 } else { s.1 },
                    )
                };
                cuboid(&mut mesh, [p(0), p(1), p(2), p(3), p(4), p(5), p(6), p(7)]);
            };
            for &(x0, x1, s0, s1) in &boxes {
                let across = (s0.min(s1), s0.max(s1));
                if across.1 - across.0 < 1e-6 || x1 - x0 < 1e-6 {
                    continue;
                }
                // a ring round the opening: its jambs as tall as the ring,
                // its head and sill between them, so an elevation shows
                // each layer's edge as one rectangle and no line of one
                // runs on across another's
                let sill = c.v0 > 1e-6;
                let tall = (if sill { c.v0 + x0 } else { c.v0 }, c.v1 - x0);
                slab((c.u0 + x0, c.u0 + x1), tall, across);
                slab((c.u1 - x1, c.u1 - x0), tall, across);
                let between = (c.u0 + x1, c.u1 - x1);
                slab(between, (c.v1 - x1, c.v1 - x0), across);
                if sill {
                    slab(between, (c.v0 + x0, c.v0 + x1), across);
                }
            }
            match out.iter_mut().find(|(b, _)| b.index == band.index) {
                Some((_, m)) => m.append(&mesh),
                None => out.push((*band, mesh)),
            }
        }
    }
    out
}

/// How thick a room's tiling is drawn in 3D, m.
const TILING_THICKNESS: f64 = 0.01;

/// A six-sided box from its corners, corner `i` having bit 0 for the far
/// end along the first axis, bit 1 along the second and bit 2 along the
/// third.
fn cuboid(m: &mut TriMesh, c: [DVec3; 8]) {
    let centre = c.iter().copied().sum::<DVec3>() / 8.0;
    for face in [
        [0, 1, 3, 2],
        [4, 5, 7, 6],
        [0, 1, 5, 4],
        [2, 3, 7, 6],
        [0, 2, 6, 4],
        [1, 3, 7, 5],
    ] {
        let q = face.map(|i| c[i]);
        let mid = q.iter().copied().sum::<DVec3>() / 4.0;
        let normal = (q[1] - q[0]).cross(q[3] - q[0]);
        let normal = if normal.dot(mid - centre) < 0.0 {
            -normal
        } else {
            normal
        };
        m.add_quad(q, normal);
    }
}

/// The lines of every sketch, in model coordinates: (sketch, from, to).
pub fn sketch_lines(model: &Model) -> Vec<(ElementId, DVec3, DVec3)> {
    let mut out = Vec::new();
    for e in model.iter() {
        if let ElementKind::Sketch(s) = &e.kind {
            let z = s
                .level
                .and_then(|l| model.level(l))
                .map_or(0.0, |l| l.elevation);
            for (a, b) in s.segments() {
                out.push((e.id, a + DVec3::Z * z, b + DVec3::Z * z));
            }
        }
    }
    out
}

/// Build viewport geometry for every element with a body.
pub fn build_scene(model: &Model) -> Vec<SceneItem> {
    build_scene_of(model, None)
}

/// [`build_scene`] for `only` these elements (with what cuts them, which
/// the caller includes): a plan slicing a few elements needn't mesh the
/// whole model (30-40 ms for the starter house, every drag frame).
pub fn build_scene_of(
    model: &Model,
    only: Option<&std::collections::HashSet<ElementId>>,
) -> Vec<SceneItem> {
    let mut out = build_scene_unjoined(model, only);
    apply_joins(model, &mut out);
    out
}

/// The scene for exchange (IFC, IFCX): each element's own body, its
/// explicit cuts applied but not the wall and slab joins, so a slab
/// stays the plain extrusion other tools read and re-import as a slab.
pub fn build_scene_for_exchange(model: &Model) -> Vec<SceneItem> {
    build_scene_unjoined(model, None)
}

fn build_scene_unjoined(
    model: &Model,
    only: Option<&std::collections::HashSet<ElementId>>,
) -> Vec<SceneItem> {
    let mut out = Vec::new();
    let joins = creabim_core::wall_joins(model);
    for e in model.iter() {
        if only.is_some_and(|o| !o.contains(&e.id)) {
            continue;
        }
        let plain = match &e.kind {
            ElementKind::Wall(_) => [0.86, 0.84, 0.80],
            ElementKind::Slab(_) => [0.70, 0.70, 0.72],
            _ => [0.66, 0.34, 0.26],
        };
        if let ElementKind::Wall(
            w @ creabim_core::Wall {
                curtain: Some(_), ..
            },
        ) = &e.kind
        {
            out.extend(curtain_items(model, e, w));
            continue;
        }
        if let Some(layers) = layer_items(model, e, &joins, plain) {
            out.extend(layers);
            continue;
        }
        let (mesh, color) = match &e.kind {
            ElementKind::Wall(w) => {
                let Some(level) = model.level(w.level) else {
                    continue;
                };
                // its doors and windows, and the curtain walls set in it
                let owned = creabim_core::curtain::wall_openings(model, e.id);
                let openings: Vec<&Opening> = owned.iter().collect();
                let ends = joins.get(&e.id).cloned().unwrap_or_default();
                let top = creabim_core::walls::top_profile(model, w);
                let mesh = wall_mesh(w, level.elevation, &openings, &ends, top.as_ref());
                // a round penetration: its square's corners filled back, a
                // solid of its own (sections slice each solid on its own)
                let t = w.thickness / 2.0;
                for o in &owned {
                    let fill = crate::opening::round_fillers(o, w, level.elevation, -t, t);
                    if !fill.is_empty() {
                        out.push(SceneItem {
                            id: e.id,
                            mesh: fill,
                            color: [0.86, 0.84, 0.80],
                            surface: PLAIN_SURFACE,
                            ghost: false,
                            layer: None,
                            finish: None,
                        });
                    }
                }
                (mesh, [0.86, 0.84, 0.80])
            }
            ElementKind::Slab(s) => {
                let Some(level) = model.level(s.level) else {
                    continue;
                };
                let holes = creabim_core::slab::holes_of(model, e);
                (
                    slab_mesh_with_holes(&s.outline(), &holes, s.top(level.elevation), s.thickness),
                    [0.70, 0.70, 0.72],
                )
            }
            ElementKind::Shaft(s) => {
                // a see-through stand-in, to see and pick the shaft by
                let (z0, z1) = s.span(model);
                out.push(SceneItem {
                    id: e.id,
                    mesh: slab_mesh(&s.boundary, z1, z1 - z0),
                    color: [0.85, 0.45, 0.25],
                    surface: PLAIN_SURFACE,
                    ghost: true,
                    layer: None,
                    finish: None,
                });
                continue;
            }
            ElementKind::Room(r) => {
                // its tiling: thin slabs on the walls' faces
                if r.tilings().next().is_none() {
                    continue;
                }
                let (Some(s), Some(level)) =
                    (creabim_core::room::shape_of(model, r), model.level(r.level))
                else {
                    continue;
                };
                let mut mesh = TriMesh::default();
                // the doors, windows and holes left out (George 2026-11-22)
                for p in r
                    .tilings()
                    .flat_map(|t| creabim_core::room::tiling_panels_of(model, &s, t))
                {
                    let (a, b) = (p.a, p.b);
                    let (z0, z1) = (level.elevation + p.z0, level.elevation + p.z1);
                    let d = (b - a).normalize_or_zero();
                    let n = glam::DVec2::new(-d.y, d.x) * TILING_THICKNESS;
                    let c = |p: glam::DVec2, z: f64| DVec3::new(p.x, p.y, z);
                    cuboid(
                        &mut mesh,
                        [
                            c(a, z0),
                            c(b, z0),
                            c(a + n, z0),
                            c(b + n, z0),
                            c(a, z1),
                            c(b, z1),
                            c(a + n, z1),
                            c(b + n, z1),
                        ],
                    );
                }
                // and in the reveals of the windows it runs across
                for t in r.tilings() {
                    for rt in creabim_core::room::tiling_reveals(model, &s, t) {
                        let up = DVec3::new(0.0, 0.0, level.elevation);
                        let [mut c0, mut c1, mut c2, mut c3] = rt.corners.map(|c| c + up);
                        // kept right-handed, so the faces look out
                        if (c1 - c0).cross(rt.normal).dot(c3 - c0) < 0.0 {
                            std::mem::swap(&mut c0, &mut c1);
                            std::mem::swap(&mut c3, &mut c2);
                        }
                        let n = rt.normal * TILING_THICKNESS;
                        cuboid(&mut mesh, [c0, c1, c0 + n, c1 + n, c3, c2, c3 + n, c2 + n]);
                    }
                }
                (mesh, [0.86, 0.9, 0.92])
            }
            ElementKind::Ceiling(c) => {
                let Some(outline) = c.outline(model) else {
                    continue;
                };
                let (_, top) = c.span(model);
                (slab_mesh(&outline, top, c.thickness), [0.95, 0.95, 0.93])
            }
            ElementKind::Opening(o) => {
                let Some(ElementKind::Wall(w)) = model.get(o.host).map(|h| &h.kind) else {
                    continue;
                };
                let Some(level) = model.level(w.level) else {
                    continue;
                };
                out.extend(opening_items(model, e, o, w, level.elevation));
                // a bare hole has nothing in it: a see-through box shows
                // it in 3D and lets it be clicked (MU18)
                if o.kind == creabim_core::OpeningKind::Hole {
                    let shown = crate::opening::round_cutter(o, w, level.elevation)
                        .or_else(|| crate::opening::cut_box_mesh(o, w, level.elevation));
                    if let Some(mesh) = shown {
                        out.push(SceneItem {
                            id: e.id,
                            mesh,
                            color: [0.85, 0.45, 0.25],
                            surface: PLAIN_SURFACE,
                            ghost: true,
                            layer: None,
                            finish: None,
                        });
                    }
                }
                continue;
            }
            ElementKind::Part(_) | ElementKind::Site(_) => {
                out.extend(crate::parts::scene_items(model, e));
                continue;
            }
            ElementKind::Object(_) => {
                // one item per part, each in its own material
                for (mesh, material) in creabim_core::objects::model_parts(model, e) {
                    let look = material.and_then(|m| match &model.get(m)?.kind {
                        ElementKind::Material(m) => Some((m.color, m.surface())),
                        _ => None,
                    });
                    let (color, surface) = look.unwrap_or(([0.8, 0.78, 0.74], PLAIN_SURFACE));
                    out.push(SceneItem {
                        id: e.id,
                        mesh: edit_mesh_to_tri(&mesh, DVec3::ZERO),
                        color,
                        surface,
                        ghost: false,
                        layer: None,
                        finish: None,
                    });
                }
                continue;
            }
            ElementKind::FreeMesh(f) => {
                let z = f
                    .level
                    .and_then(|l| model.level(l))
                    .map_or(0.0, |l| l.elevation);
                (
                    edit_mesh_to_tri(&f.evaluated().mesh, f.origin + DVec3::Z * z),
                    [0.62, 0.75, 0.90],
                )
            }
            ElementKind::Level(_)
            | ElementKind::Grid(_)
            | ElementKind::View(_)
            | ElementKind::Sheet(_)
            | ElementKind::Viewport(_)
            | ElementKind::Material(_)
            | ElementKind::Assembly(_)
            | ElementKind::Dimension(_)
            | ElementKind::Tag(_)
            | ElementKind::Note(_)
            | ElementKind::TextStyle(_)
            | ElementKind::SheetTemplate(_)
            | ElementKind::Units(_)
            | ElementKind::Project(_)
            | ElementKind::Reference(_)
            | ElementKind::PaperMark(_)
            | ElementKind::MarkRules(_)
            | ElementKind::Results(_)
            | ElementKind::Dss(_)
            | ElementKind::StylePack(_)
            | ElementKind::ElementType(_)
            | ElementKind::Sketch(_)
            | ElementKind::Cut(_)
            | ElementKind::Guide(_)
            | ElementKind::CropBox(_)
            | ElementKind::GridGroup(_)
            | ElementKind::RoomSeparator(_)
            | ElementKind::Join(_)
            | ElementKind::Detail(_) => continue,
            ElementKind::Column(_)
            | ElementKind::Beam(_)
            | ElementKind::Roof(_)
            | ElementKind::Footing(_)
            | ElementKind::Flashing(_)
            | ElementKind::Pipe(_) => match crate::members::member_mesh(model, e) {
                Some(m) => m,
                None => continue,
            },
            ElementKind::Valve(v) => match crate::valves::valve_mesh(model, v) {
                Some(m) => m,
                None => continue,
            },
            // its carrier, treads and risers apart, each of its material
            ElementKind::Stair(st) => {
                let parts = crate::members::stair_parts(st, st.base(model), st.rise(model));
                for (mesh, finish) in [
                    (parts.treads, st.tread_finish),
                    (parts.risers, st.riser_finish),
                ] {
                    if mesh.indices.is_empty() {
                        continue;
                    }
                    let (color, surface) = finish
                        .and_then(|f| f.material)
                        .and_then(|m| match &model.get(m)?.kind {
                            ElementKind::Material(m) => Some((m.color, m.surface())),
                            _ => None,
                        })
                        .unwrap_or(([0.62, 0.48, 0.32], PLAIN_SURFACE));
                    out.push(SceneItem {
                        id: e.id,
                        mesh,
                        color,
                        surface,
                        ghost: false,
                        layer: None,
                        finish: None,
                    });
                }
                let color = if st.carrier.is_concrete() || st.ramp.is_some() {
                    [0.82, 0.80, 0.76]
                } else {
                    match st.stringer.as_ref().map(|x| x.kind) {
                        Some(creabim_core::members::StringerKind::Timber) => [0.62, 0.48, 0.32],
                        _ => [0.35, 0.37, 0.40],
                    }
                };
                (parts.carrier, color)
            }
            // its metal and its glass apart, the glass see-through
            ElementKind::Railing(r) => {
                let (mesh, glass) = crate::members::railing_meshes(model, r);
                if !glass.indices.is_empty() {
                    out.push(SceneItem {
                        id: e.id,
                        mesh: glass,
                        color: GLASS_COLOR,
                        surface: GLASS_SURFACE,
                        ghost: false,
                        layer: None,
                        finish: Some("Glass"),
                    });
                }
                (mesh, crate::members::railing_color(r))
            }
            ElementKind::Circuit(_) => continue,
        };
        // a material's look wins over the element type's default; a
        // railing's stainless or aluminium shines
        let plain = match &e.kind {
            ElementKind::Railing(r) if r.metal.is_some() => METAL_SURFACE,
            _ => PLAIN_SURFACE,
        };
        let (color, surface) = creabim_core::material_of(model, e)
            .map_or((color, plain), |(_, m)| (m.color, m.surface()));
        out.push(SceneItem {
            id: e.id,
            mesh,
            color,
            surface,
            ghost: false,
            layer: None,
            finish: None,
        });
        // a parapet's capping lies on XPS in a fall and an OSB board
        if let ElementKind::Flashing(creabim_core::flashing::Flashing {
            item: creabim_core::flashing::FlashingItem::Capping(c),
            ..
        }) = &e.kind
        {
            for (i, (mesh, color, material)) in crate::flashing::capping_substrate(model, c)
                .into_iter()
                .enumerate()
            {
                let (color, surface) = material
                    .and_then(|m| match &model.get(m)?.kind {
                        ElementKind::Material(m) => Some((m.color, m.surface())),
                        _ => None,
                    })
                    .unwrap_or((color, PLAIN_SURFACE));
                out.push(SceneItem {
                    id: e.id,
                    mesh,
                    color,
                    surface,
                    ghost: false,
                    layer: Some(LayerPart {
                        index: i + 1,
                        material,
                        function: if i == 0 && c.xps + c.fall > 1e-6 {
                            creabim_core::LayerFunction::Insulation
                        } else {
                            creabim_core::LayerFunction::Substrate
                        },
                        core: false,
                        priority: 5,
                    }),
                    finish: None,
                });
            }
        }
    }
    apply_cuts(model, &mut out);
    out
}

/// The scene as drawings show it: cutters left out.
pub fn drawn_scene(model: &Model) -> Vec<SceneItem> {
    shared_scene(model)
        .iter()
        .filter(|i| !i.ghost && !i.is_lining())
        .cloned()
        .collect()
}

/// How many scenes [`shared_scene`] keeps: the model as it is, and a few
/// others (a view's filtered copy, the state before an edit).
const SHARED_SCENES: usize = 4;

static SHARED: std::sync::Mutex<Vec<(u64, std::sync::Arc<Vec<SceneItem>>)>> =
    std::sync::Mutex::new(Vec::new());

/// The shared scene of `model`'s revision if it is built already, without
/// waiting: `None` while it isn't, or while another thread holds the
/// cache (building one).
pub fn shared_scene_ready(model: &Model) -> Option<std::sync::Arc<Vec<SceneItem>>> {
    let rev = model.revision();
    let cache = SHARED.try_lock().ok()?;
    cache
        .iter()
        .find(|(r, _)| *r == rev && rev != 0)
        .map(|(_, s)| s.clone())
}

/// [`build_scene`] built once per model revision and shared: the 3D view,
/// every section, elevation and 3D drawing of the same revision use the
/// one scene instead of each building its own. Revisions are unique
/// across models (see [`Model::revision`]); a model at revision 0 is
/// built afresh every time.
pub fn shared_scene(model: &Model) -> std::sync::Arc<Vec<SceneItem>> {
    let rev = model.revision();
    if rev == 0 {
        return std::sync::Arc::new(build_scene(model));
    }
    // held while building, so two threads after the same revision build it once
    let mut cache = SHARED.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(i) = cache.iter().position(|(r, _)| *r == rev) {
        let hit = cache.remove(i);
        let scene = hit.1.clone();
        cache.push(hit);
        return scene;
    }
    let scene = std::sync::Arc::new(build_scene(model));
    if cache.len() >= SHARED_SCENES {
        cache.remove(0);
    }
    cache.push((rev, scene.clone()));
    scene
}

/// One item per element, a layered element's layers in one mesh with its
/// core's look: for exports that take one shape per element.
pub fn merged(items: Vec<SceneItem>) -> Vec<SceneItem> {
    let mut out: Vec<SceneItem> = Vec::with_capacity(items.len());
    let mut at: std::collections::HashMap<ElementId, usize> = std::collections::HashMap::new();
    for item in items.into_iter().filter(|i| !i.is_lining()) {
        match at.get(&item.id) {
            Some(&i) => {
                let o = &mut out[i];
                o.mesh.append(&item.mesh);
                // the first core layer gives the look
                if item.layer.is_some_and(|l| l.core) && !o.layer.is_some_and(|l| l.core) {
                    (o.color, o.surface, o.layer) = (item.color, item.surface, item.layer);
                }
            }
            None => {
                at.insert(item.id, out.len());
                out.push(item);
            }
        }
    }
    for o in &mut out {
        o.layer = None;
    }
    out
}

/// [`build_scene`] with one item per element.
pub fn element_scene(model: &Model) -> Vec<SceneItem> {
    merged(build_scene(model))
}

/// [`drawn_scene`] with one item per element.
pub fn drawn_elements(model: &Model) -> Vec<SceneItem> {
    merged(drawn_scene(model))
}

/// Every cut in the model, in order: (target, cutter, operation).
pub fn cuts(model: &Model) -> Vec<(ElementId, ElementId, BoolOp)> {
    model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Cut(c) => Some((c.target, c.cutter, c.op)),
            _ => None,
        })
        .collect()
}

/// [`cuts`], and what each void cuts: everything that is cut in the model.
pub fn all_cuts(model: &Model) -> Vec<(ElementId, ElementId, BoolOp)> {
    let mut out = cuts(model);
    out.extend(
        creabim_core::parts::void_cuts(model)
            .into_iter()
            .map(|(t, v)| (t, v, BoolOp::Difference)),
    );
    out
}

/// Is `id` used to cut something? Cutters are hidden from drawings.
pub fn is_cutter(model: &Model, id: ElementId) -> bool {
    model
        .iter()
        .any(|e| matches!(&e.kind, ElementKind::Cut(c) if c.cutter == id))
}

/// Apply every cut to the meshes of its target, using the cutters' own
/// shapes (a cutter that is itself cut cuts with its uncut shape, so cuts
/// can never loop), and turn the cutters into ghosts.
fn apply_cuts(model: &Model, items: &mut [SceneItem]) {
    let cuts = all_cuts(model);
    if cuts.is_empty() {
        return;
    }
    // a layered cutter cuts with all its layers
    let mut original: std::collections::HashMap<ElementId, TriMesh> =
        std::collections::HashMap::new();
    for (_, c, _) in &cuts {
        if original.contains_key(c) {
            continue;
        }
        let mut tool = TriMesh::default();
        for i in items.iter().filter(|i| i.id == *c) {
            tool.append(&i.mesh);
        }
        if !tool.is_empty() {
            original.insert(*c, tool);
        }
    }
    for (target, cutter, op) in &cuts {
        let Some(tool) = original.get(cutter) else {
            continue;
        };
        for item in items.iter_mut().filter(|i| i.id == *target) {
            item.mesh = boolean_mesh(&item.mesh, tool, *op);
        }
    }
    for item in items.iter_mut() {
        if original.contains_key(&item.id) {
            item.ghost = true;
        }
    }
}

/// Where two built elements overlap in 3D, the one the drawings let run
/// through cuts the other (A4-6), so 3D, sections, plans and the material
/// take-off agree and no volume is counted twice: by the junction's own
/// rule if it has one, else layer by layer (the lower join priority runs
/// through: cores through finishes), then by kind (columns, beams, walls,
/// slabs, footings, roofs, ceilings: see `joins::category_rank`), then
/// the element drawn first. Walls, slabs, roofs, footings, columns, beams
/// and ceilings all take part. Each result is kept, keyed by the two
/// shapes, so editing one wall redoes only its own joins.
fn apply_joins(model: &Model, items: &mut [SceneItem]) {
    let rules = creabim_core::joins::rules(model);
    let order: std::collections::HashMap<ElementId, usize> =
        model.iter().enumerate().map(|(i, e)| (e.id, i)).collect();
    let built = |id: ElementId| match model.get(id).map(|e| &e.kind) {
        Some(ElementKind::Wall(w)) => w.curtain.is_none(),
        Some(k) => creabim_core::joins::joinable(k),
        None => false,
    };
    let strength = |i: &SceneItem| {
        (
            std::cmp::Reverse(i.layer.map_or(1, |l| l.priority)),
            model
                .get(i.id)
                .map_or(0, |e| creabim_core::joins::category_rank(&e.kind)),
        )
    };
    let boxes: Vec<Option<(glam::Vec3, glam::Vec3)>> = items
        .iter()
        .map(|i| {
            if i.ghost || i.is_lining() {
                None
            } else {
                i.mesh.bounds()
            }
        })
        .collect();
    let overlap = |a: (glam::Vec3, glam::Vec3), b: (glam::Vec3, glam::Vec3)| {
        let lo = a.0.max(b.0);
        let hi = a.1.min(b.1);
        (hi - lo).min_element() > 1e-4
    };
    let solids: Vec<usize> = (0..items.len())
        .filter(|k| boxes[*k].is_some() && built(items[*k].id))
        .collect();
    // (target, cutter) pairs, in item indices
    let mut pairs: Vec<(usize, usize)> = vec![];
    for (n, &i) in solids.iter().enumerate() {
        for &j in &solids[n + 1..] {
            let (a, b) = (&items[i], &items[j]);
            if a.id == b.id {
                continue;
            }
            let (Some(ba), Some(bb)) = (boxes[i], boxes[j]) else {
                continue;
            };
            if !overlap(ba, bb) {
                continue;
            }
            let rule = rules.iter().find(|r| r.joins(a.id, b.id));
            if rule.is_some_and(|r| !r.cleanup) {
                continue;
            }
            let a_wins = match rule.and_then(|r| r.wins(a.id, b.id)) {
                Some(w) => w,
                None => {
                    let (sa, sb) = (strength(a), strength(b));
                    if sa != sb {
                        sa > sb
                    } else {
                        // as strong: the one drawn first (the same
                        // material reads as one mass either way)
                        let at = |id| order.get(&id).copied().unwrap_or(usize::MAX);
                        at(a.id) < at(b.id)
                    }
                }
            };
            pairs.push(if a_wins { (j, i) } else { (i, j) });
        }
    }
    if pairs.is_empty() {
        return;
    }
    // the cutters' shapes before any join cuts them
    let originals: Vec<TriMesh> = items.iter().map(|i| i.mesh.clone()).collect();
    for (t, c) in pairs {
        let key = (mesh_key(&items[t].mesh), mesh_key(&originals[c]));
        let done = JOIN_CACHE.with(|m| m.borrow().get(&key).cloned());
        let mesh = match done {
            Some(m) => m,
            None => {
                let before = &items[t].mesh;
                let cut = boolean_mesh(before, &originals[c], BoolOp::Difference);
                // where the boxes meet but the solids don't (a mitred
                // corner), the shape stays as it was, not split up for
                // nothing; a cut that came out broken (more than it was,
                // or less than nothing) is left undone, and an element
                // wholly inside the one that wins (a beam set to stay
                // buried in its wall) stays as it is, for X-ray to show
                let (v0, v1) = (closed_volume(before), closed_volume(&cut));
                let m = if v1 >= v0 - 1e-7 || v1 < 1e-6 || v1 > v0 + 1e-7 {
                    before.clone()
                } else {
                    cut
                };
                JOIN_CACHE.with(|cache| {
                    let mut cache = cache.borrow_mut();
                    if cache.len() > 4000 {
                        cache.clear();
                    }
                    cache.insert(key, m.clone());
                });
                m
            }
        };
        items[t].mesh = mesh;
    }
}

thread_local! {
    static JOIN_CACHE: std::cell::RefCell<std::collections::HashMap<(u64, u64), TriMesh>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

/// The volume a closed mesh holds, measured about its first corner so
/// that coordinates far from the origin lose no precision.
pub fn closed_volume(m: &TriMesh) -> f64 {
    let Some(o) = m.positions.first() else {
        return 0.0;
    };
    let o = glam::Vec3::from(*o).as_dvec3();
    m.indices
        .chunks_exact(3)
        .map(|t| {
            let p = |i: u32| glam::Vec3::from(m.positions[i as usize]).as_dvec3() - o;
            p(t[0]).dot(p(t[1]).cross(p(t[2]))) / 6.0
        })
        .sum()
}

/// A fingerprint of a mesh's shape.
pub(crate) fn mesh_key(m: &TriMesh) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    m.indices.hash(&mut h);
    for p in &m.positions {
        for v in p {
            v.to_bits().hash(&mut h);
        }
    }
    h.finish()
}

/// `a op b` for two closed triangle meshes.
pub fn boolean_mesh(a: &TriMesh, b: &TriMesh, op: BoolOp) -> TriMesh {
    let polys = |m: &TriMesh| -> Vec<csg::Polygon> {
        m.indices
            .chunks_exact(3)
            .filter_map(|t| {
                let p = |i: u32| {
                    let v = m.positions[i as usize];
                    DVec3::new(v[0] as f64, v[1] as f64, v[2] as f64)
                };
                csg::Polygon::new(vec![p(t[0]), p(t[1]), p(t[2])])
            })
            .collect()
    };
    let result = csg::boolean(polys(a), polys(b), op);
    let mut out = TriMesh::default();
    for p in &result {
        let n = (p.points[1] - p.points[0])
            .cross(p.points[2] - p.points[0])
            .normalize_or_zero();
        let fan: Vec<[usize; 3]> = (1..p.points.len() - 1).map(|i| [0, i, i + 1]).collect();
        // the fan's winding already follows the polygon's
        out.add_polygon(&p.points, &fan, newell(&p.points).normalize_or(n));
    }
    out
}

fn newell(p: &[DVec3]) -> DVec3 {
    let mut n = DVec3::ZERO;
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        n += DVec3::new(
            (a.y - b.y) * (a.z + b.z),
            (a.z - b.z) * (a.x + b.x),
            (a.x - b.x) * (a.y + b.y),
        );
    }
    n
}

/// A door's or window's parts, each in its own colour: the frame and leaf
/// in the opening's material (or a painted default), glass see-through.
/// An element can so have several items, all with its id.
fn opening_items(
    model: &Model,
    e: &creabim_core::Element,
    o: &Opening,
    w: &creabim_core::Wall,
    level_z: f64,
) -> Vec<SceneItem> {
    use crate::opening::Part;
    use creabim_core::OpeningKind;
    let material = creabim_core::material_of(model, e).map(|(_, m)| (m.color, m.surface()));
    let asm = model
        .get(o.host)
        .and_then(|h| creabim_core::assembly_of(model, h))
        .map(|x| x.1);
    crate::opening::opening_meshes(o, w, asm, level_z)
        .into_iter()
        .map(|(part, mesh)| {
            let (color, surface) = match (part, o.kind, material) {
                (Part::Glass | Part::LeafGlass, ..) => (GLASS_COLOR, GLASS_SURFACE),
                (Part::Handle, ..) => ([0.72, 0.73, 0.75], [0.3, 1.0, 1.0]),
                (Part::Panel, _, None) => ([0.80, 0.80, 0.78], PLAIN_SURFACE),
                // a door as joinery: its frame and leaf in their own colours
                (Part::Frame, OpeningKind::Door, None) if o.door.is_some() => (
                    o.door.as_ref().map_or([0.5; 3], |d| d.frame.color()),
                    PLAIN_SURFACE,
                ),
                (Part::Leaf | Part::LeafRail | Part::LeafOutline, _, None) if o.door.is_some() => (
                    o.door.as_ref().map_or([0.5; 3], |d| d.leaf.color()),
                    PLAIN_SURFACE,
                ),
                (_, _, Some(m)) => m,
                (Part::Frame, OpeningKind::Window | OpeningKind::Hole, None) => {
                    ([0.93, 0.93, 0.91], PLAIN_SURFACE)
                }
                (Part::Frame, OpeningKind::Door, None) => ([0.50, 0.35, 0.22], PLAIN_SURFACE),
                (Part::Leaf | Part::LeafRail | Part::LeafOutline, _, None) => {
                    ([0.62, 0.45, 0.30], PLAIN_SURFACE)
                }
            };
            SceneItem {
                id: e.id,
                mesh,
                color,
                surface,
                ghost: false,
                layer: None,
                finish: match part {
                    Part::Glass | Part::LeafGlass => Some("Glass"),
                    Part::Handle => Some("Metal"),
                    Part::Frame | Part::Leaf | Part::Panel | Part::LeafRail | Part::LeafOutline => {
                        None
                    }
                },
            }
        })
        .collect()
}

/// Bare metal: a door handle, a stainless or aluminium railing.
pub const METAL_SURFACE: [f32; 3] = [0.3, 1.0, 1.0];

/// Glass in doors and windows: pale blue, smooth and mostly see-through.
pub const GLASS_COLOR: [f32; 3] = [0.62, 0.78, 0.88];
pub const GLASS_SURFACE: [f32; 3] = [0.05, 0.0, 0.3];

/// Each element's whole mesh: the items of one element (a window's frame
/// and glass) merged.
pub fn element_meshes(scene: &[SceneItem]) -> std::collections::HashMap<ElementId, TriMesh> {
    let mut out: std::collections::HashMap<ElementId, TriMesh> = Default::default();
    for i in scene.iter().filter(|i| !i.is_lining()) {
        out.entry(i.id).or_default().append(&i.mesh);
    }
    out
}

/// A curtain wall's mullions and panels, each kind in its own look: the
/// mullions and solid panels in the wall's material (or anodised
/// aluminium and a pale grey), glass see-through.
fn curtain_items(model: &Model, e: &Element, w: &creabim_core::Wall) -> Vec<SceneItem> {
    use crate::curtain::CurtainPart;
    let (Some(level), Some(grid)) = (model.level(w.level), &w.curtain) else {
        return vec![];
    };
    let openings: Vec<&Opening> = model
        .dependents(e.id)
        .into_iter()
        .filter_map(|d| match &model.get(d)?.kind {
            ElementKind::Opening(o) => Some(o),
            _ => None,
        })
        .collect();
    let material = creabim_core::material_of(model, e).map(|(_, m)| (m.color, m.surface()));
    crate::curtain::curtain_meshes(w, grid, &openings, level.elevation)
        .into_iter()
        .map(|(part, mesh)| {
            let (color, surface) = match (part, material) {
                (CurtainPart::Glass, _) => (GLASS_COLOR, GLASS_SURFACE),
                (_, Some(m)) => m,
                (CurtainPart::Mullion, None) => ([0.62, 0.64, 0.67], [0.35, 0.9, 1.0]),
                (CurtainPart::Panel, None) => ([0.80, 0.80, 0.78], PLAIN_SURFACE),
            };
            SceneItem {
                id: e.id,
                mesh,
                color,
                surface,
                ghost: false,
                layer: None,
                finish: match part {
                    CurtainPart::Glass => Some("Glass"),
                    CurtainPart::Mullion => Some("Metal"),
                    CurtainPart::Panel => None,
                },
            }
        })
        .collect()
}

/// An extruded slab with holes through it: top face at `top_z`,
/// `thickness` downwards, each hole (a polygon inside the outline) cut
/// straight through.
pub fn slab_mesh_with_holes(
    boundary: &[DVec2],
    holes: &[Vec<DVec2>],
    top_z: f64,
    thickness: f64,
) -> TriMesh {
    if holes.is_empty() {
        return slab_mesh(boundary, top_z, thickness);
    }
    let face = creabim_core::polygon::bridged(boundary, holes);
    let mut m = TriMesh::default();
    let tris = triangulate(&face);
    let top: Vec<DVec3> = face.iter().map(|p| p.extend(top_z)).collect();
    let bottom: Vec<DVec3> = face.iter().map(|p| p.extend(top_z - thickness)).collect();
    m.add_polygon(&top, &tris, DVec3::Z);
    m.add_polygon(&bottom, &tris, -DVec3::Z);
    // the outline counter-clockwise and the holes clockwise: material is
    // on the left of every edge, so the sides face right
    let mut loops = vec![boundary.to_vec()];
    loops.extend(holes.iter().cloned());
    for (k, lp) in loops.iter_mut().enumerate() {
        let ccw = crate::triangulate::signed_area2(lp) > 0.0;
        if ccw != (k == 0) {
            lp.reverse();
        }
        let n = lp.len();
        for i in 0..n {
            let (a, b) = (lp[i], lp[(i + 1) % n]);
            let e = (b - a).normalize_or_zero();
            m.add_quad(
                [
                    a.extend(top_z - thickness),
                    b.extend(top_z - thickness),
                    b.extend(top_z),
                    a.extend(top_z),
                ],
                DVec3::new(e.y, -e.x, 0.0),
            );
        }
    }
    m
}

/// Extruded slab: top face at `top_z`, `thickness` downwards.
pub fn slab_mesh(boundary: &[DVec2], top_z: f64, thickness: f64) -> TriMesh {
    let mut m = TriMesh::default();
    let tris = triangulate(boundary);
    let top: Vec<DVec3> = boundary.iter().map(|p| p.extend(top_z)).collect();
    let bottom: Vec<DVec3> = boundary
        .iter()
        .map(|p| p.extend(top_z - thickness))
        .collect();
    m.add_polygon(&top, &tris, DVec3::Z);
    m.add_polygon(&bottom, &tris, -DVec3::Z);
    let ccw = crate::triangulate::signed_area2(boundary) > 0.0;
    for i in 0..boundary.len() {
        let (a, b) = (boundary[i], boundary[(i + 1) % boundary.len()]);
        let e = (b - a).normalize_or_zero();
        let out = if ccw {
            DVec2::new(e.y, -e.x)
        } else {
            DVec2::new(-e.y, e.x)
        };
        m.add_quad(
            [
                bottom[i],
                bottom[(i + 1) % boundary.len()],
                top[(i + 1) % boundary.len()],
                top[i],
            ],
            out.extend(0.0),
        );
    }
    m
}

/// Flat-shaded triangles from a polygon mesh, offset to `origin`, face by
/// face (so triangle k still maps back to its face).
///
/// Coincident corners are welded: corners closer than a micron become one
/// position, and faces lying in one plane share the vertex itself. A
/// corner on a crease is still one vertex per face direction, so every
/// face keeps its own flat normal, but they all sit at exactly the same
/// point, so the mesh is closed wherever the edit mesh is (see
/// [`TriMesh::is_closed`]).
pub fn edit_mesh_to_tri(mesh: &creabim_core::EditMesh, origin: DVec3) -> TriMesh {
    use std::collections::HashMap;
    const TOL: f64 = 1e-6;
    let key = |p: DVec3| {
        (
            (p.x / TOL).round() as i64,
            (p.y / TOL).round() as i64,
            (p.z / TOL).round() as i64,
        )
    };
    let mut first: HashMap<(i64, i64, i64), usize> = HashMap::new();
    let corner: Vec<usize> = (0..mesh.vertices.len())
        .map(|v| *first.entry(key(mesh.vertices[v])).or_insert(v))
        .collect();
    let mut m = TriMesh::default();
    let mut shared: HashMap<(usize, [i64; 3]), u32> = HashMap::new();
    for f in 0..mesh.faces.len() {
        let face = &mesh.faces[f];
        let n = mesh.face_normal(f);
        let nk = (n * 1e5).round().as_i64vec3().to_array();
        let mut index = |k: usize| -> (u32, DVec3) {
            let c = corner[face[k] as usize];
            let p = mesh.vertices[c] + origin;
            let i = *shared.entry((c, nk)).or_insert_with(|| {
                m.positions.push(p.as_vec3().to_array());
                m.normals.push(n.as_vec3().to_array());
                (m.positions.len() - 1) as u32
            });
            (i, p)
        };
        let tris: Vec<[(u32, DVec3); 3]> = mesh
            .face_corners(f)
            .into_iter()
            .map(|t| t.map(&mut index))
            .collect();
        for [(a, pa), (b, pb), (c, pc)] in tris {
            // keep the winding consistent with the normal
            if (pb - pa).cross(pc - pa).dot(n) >= 0.0 {
                m.indices.extend([a, b, c]);
            } else {
                m.indices.extend([a, c, b]);
            }
        }
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insulation_and_render_turn_into_a_windows_head_and_sill() {
        use creabim_core::{AssemblyUse, Document, Layer, LayerFunction as F};
        let mut doc = Document::from_model(creabim_core::sample::layered_house());
        let ids: Vec<ElementId> = doc.model().iter().map(|e| e.id).collect();
        for id in ids {
            match doc.model().get(id).unwrap().kind.clone() {
                ElementKind::Assembly(a) if a.usage == AssemblyUse::Wall => doc
                    .edit(id, "layers", |e| {
                        if let ElementKind::Assembly(a) = &mut e.kind {
                            a.layers = vec![
                                Layer::new(F::Finish, None, 0.005),
                                Layer::new(F::Insulation, None, 0.15),
                                Layer::new(F::Structure, None, 0.25),
                                Layer::new(F::Finish, None, 0.015),
                            ];
                        }
                    })
                    .unwrap(),
                ElementKind::Wall(_) => {
                    let _ = doc.edit(id, "thickness", |e| {
                        if let ElementKind::Wall(w) = &mut e.kind {
                            w.thickness = 0.42;
                        }
                    });
                }
                _ => {}
            }
        }
        let m = doc.model();
        let (wall, w, o) = m
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::Opening(o) if o.kind == creabim_core::OpeningKind::Window => {
                    let host = m.get(o.host)?;
                    let ElementKind::Wall(w) = &host.kind else {
                        return None;
                    };
                    assembly::assembly_of(m, host)?;
                    Some((host.id, w.clone(), o.clone()))
                }
                _ => None,
            })
            .unwrap();
        let z0 = m.level(w.level).unwrap().elevation + w.base_offset;
        let scene = build_scene_of(m, None);
        // the insulation: the wall's layer with its reveals
        let parts: Vec<_> = scene
            .iter()
            .filter(|i| i.id == wall && i.layer.is_some_and(|l| l.index == 1))
            .collect();
        assert_eq!(parts.len(), 1, "one solid a layer");
        // lapped 30 mm over the frame at the head and the sill
        let lap = assembly::REVEAL_INSULATION;
        let v = parts[0].mesh.positions.iter().map(|p| p[2] as f64);
        assert!(v
            .clone()
            .any(|z| (z - (z0 + o.sill + o.height - lap)).abs() < 1e-4));
        assert!(v.clone().any(|z| (z - (z0 + o.sill + lap)).abs() < 1e-4));
    }

    #[test]
    fn a_partition_held_to_the_ceilings_has_no_sliver_at_its_ends() {
        // MU20: FF Spine runs a few centimetres past the ceiling's edge into
        // the wall it joins; its top used to rise there to its own height
        let m = creabim_core::starter::starter_house();
        let id = m.iter().find(|e| e.name == "FF Spine").unwrap().id;
        let top = build_scene_of(&m, None)
            .iter()
            .filter(|i| i.id == id)
            .filter_map(|i| i.mesh.bounds())
            .map(|b| b.1.z)
            .fold(f32::MIN, f32::max);
        assert!((top - 5.5).abs() < 1e-3, "its top reaches {top}");
    }

    #[test]
    fn a_slab_edge_in_a_wall_is_cut_by_it_in_3d_as_in_the_drawings() {
        // A4-6: a floor slab whose edge runs into the wall below it, with
        // no junction rule: the wall runs through, so nothing of the slab
        // is left inside the wall in 3D, as in plans and sections
        use creabim_core::{Element, Level, Slab, Transaction, Wall};
        let l0 = Element::new(
            "L0",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let l1 = Element::new(
            "L1",
            ElementKind::Level(Level {
                elevation: 3.0,
                height: 3.0,
            }),
        );
        let wall = Element::new(
            "Wall",
            ElementKind::Wall(Wall::new(
                l0.id,
                DVec2::ZERO,
                DVec2::new(6.0, 0.0),
                0.3,
                3.0,
            )),
        );
        let slab = Element::new(
            "Slab",
            ElementKind::Slab(Slab::new(
                l1.id,
                vec![
                    DVec2::new(0.0, 0.0),
                    DVec2::new(6.0, 0.0),
                    DVec2::new(6.0, 4.0),
                    DVec2::new(0.0, 4.0),
                ],
                0.25,
            )),
        );
        let (wid, sid) = (wall.id, slab.id);
        let mut doc = creabim_core::document::Document::new();
        doc.commit(
            Transaction::new("setup")
                .insert(l0)
                .insert(l1)
                .insert(wall)
                .insert(slab),
        )
        .unwrap();
        let items = build_scene_of(doc.model(), None);
        let (lo, hi) = items
            .iter()
            .filter(|i| i.id == wid)
            .filter_map(|i| i.mesh.bounds())
            .next()
            .unwrap();
        let slab_mesh = &items.iter().find(|i| i.id == sid).unwrap().mesh;
        let (slo, _) = slab_mesh.bounds().unwrap();
        // its edge now stops at the wall's face, not its middle
        assert!(
            (slo.y - hi.y).abs() < 1e-3,
            "slab starts at y {} (wall face {})",
            slo.y,
            hi.y
        );
        for t in slab_mesh.indices.chunks_exact(3) {
            let c = t
                .iter()
                .map(|k| glam::Vec3::from(slab_mesh.positions[*k as usize]))
                .sum::<glam::Vec3>()
                / 3.0;
            let inside = (c - lo).min_element() > 0.002 && (hi - c).min_element() > 0.002;
            assert!(
                !inside,
                "a slab face at {c} is inside the wall ({lo}..{hi})"
            );
        }
    }

    #[test]
    fn slab_volume_matches_area_times_thickness() {
        let l = [
            DVec2::new(0.0, 0.0),
            DVec2::new(4.0, 0.0),
            DVec2::new(4.0, 2.0),
            DVec2::new(2.0, 2.0),
            DVec2::new(2.0, 4.0),
            DVec2::new(0.0, 4.0),
        ];
        let m = slab_mesh(&l, 0.0, 0.25);
        assert!((m.volume() - 12.0 * 0.25).abs() < 1e-5);
        let mut cw = l.to_vec();
        cw.reverse();
        assert!((slab_mesh(&cw, 0.0, 0.25).volume() - 3.0).abs() < 1e-5);
    }

    #[test]
    fn a_slab_top_offset_lifts_its_mesh_and_arcs_round_it() {
        let mut model = creabim_core::sample::sample_house();
        let (id, elevation, t) = model
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::Slab(s) => Some((e.id, model.level(s.level)?.elevation, s.thickness)),
                _ => None,
            })
            .unwrap();
        let mut e = model.get(id).unwrap().clone();
        let ElementKind::Slab(s) = &mut e.kind else {
            panic!()
        };
        // a 4 x 2 m slab whose far side is a half circle, its top 0.6 m up,
        // far from any wall (walls cut what of a slab runs into them, A4-6)
        let o = DVec2::splat(100.0);
        s.boundary = vec![
            o + DVec2::new(0.0, 0.0),
            o + DVec2::new(4.0, 0.0),
            o + DVec2::new(4.0, 2.0),
            o + DVec2::new(0.0, 2.0),
        ];
        s.bulges = vec![0.0, 1.0, 0.0, 0.0];
        s.top_offset = 0.6;
        creabim_core::Transaction::new("edit")
            .replace(model.get(id).unwrap().clone(), e)
            .apply(&mut model)
            .unwrap();
        let scene = build_scene(&model);
        let item = scene.iter().find(|i| i.id == id).unwrap();
        let (lo, hi) = item.mesh.bounds().unwrap();
        assert!((hi.z as f64 - (elevation + 0.6)).abs() < 1e-5);
        assert!((lo.z as f64 - (elevation + 0.6 - t)).abs() < 1e-5);
        // the arc bulges a metre past the straight side
        assert!((hi.x - 105.0).abs() < 1e-4);
        let area = 8.0 + std::f64::consts::PI / 2.0;
        assert!((item.mesh.volume() - area * t).abs() < 0.01);
    }

    #[test]
    fn a_slab_with_a_hole_loses_its_volume() {
        let outer = creabim_core::polygon::rectangle(DVec2::ZERO, DVec2::new(4.0, 4.0));
        let hole = creabim_core::polygon::rectangle(DVec2::new(1.0, 1.0), DVec2::new(2.0, 3.0));
        let m = slab_mesh_with_holes(&outer, std::slice::from_ref(&hole), 0.0, 0.25);
        assert!((m.volume() - 14.0 * 0.25).abs() < 1e-5, "{}", m.volume());
        // the plan cut at mid-depth has the hole in it
        let cut = crate::slice::slice(&m, -0.1);
        assert_eq!(cut.len(), 1);
        let area = crate::triangulate::signed_area2(&cut[0]) / 2.0;
        assert!((area - 14.0).abs() < 1e-5, "{area}");
    }

    #[test]
    fn edit_mesh_corners_are_welded_and_faces_stay_flat() {
        // a box whose faces each carry their own copies of the corners, as
        // an imported mesh does
        let cube = creabim_core::EditMesh::cuboid(DVec3::new(2.0, 2.0, 2.0));
        let mut split = creabim_core::EditMesh::default();
        for f in &cube.faces {
            let base = split.vertices.len() as u32;
            split
                .vertices
                .extend(f.iter().map(|&v| cube.vertices[v as usize]));
            split
                .faces
                .push((0..f.len() as u32).map(|i| base + i).collect());
        }
        let m = edit_mesh_to_tri(&split, DVec3::ZERO);
        assert!(m.is_closed());
        // eight corners, three face directions each
        assert_eq!(m.positions.len(), 24);
        let mut corners: Vec<[u32; 3]> = m.positions.iter().map(|p| p.map(f32::to_bits)).collect();
        corners.sort_unstable();
        corners.dedup();
        assert_eq!(corners.len(), 8);
        for t in m.indices.chunks(3) {
            let n = |i: u32| m.normals[i as usize];
            assert!(n(t[0]) == n(t[1]) && n(t[1]) == n(t[2]), "flat faces");
        }
        // a face split in two shares the corners along the split
        let mut halves = cube.clone();
        let top = halves
            .faces
            .iter()
            .position(|f| f.iter().all(|&v| cube.vertices[v as usize].z > 1.0))
            .unwrap();
        let q = halves.faces.remove(top);
        halves.faces.push(vec![q[0], q[1], q[2]]);
        halves.faces.push(vec![q[0], q[2], q[3]]);
        let m = edit_mesh_to_tri(&halves, DVec3::ZERO);
        assert_eq!(m.positions.len(), 24);
        assert!(m.is_closed());
        // and the plan cut of the closed box is one loop
        let cut = crate::slice::slice(&m, 1.0);
        assert_eq!(cut.len(), 1);
    }

    #[test]
    fn edit_mesh_volume_is_preserved() {
        let mut e = creabim_core::EditMesh::cuboid(DVec3::new(2.0, 2.0, 2.0));
        e.extrude_face(1, 1.0);
        let m = edit_mesh_to_tri(&e, DVec3::new(10.0, 0.0, 0.0));
        assert!((m.volume() - 12.0).abs() < 1e-4);
    }

    #[test]
    fn window_reveals_show_the_finish_not_the_insulation() {
        let m = creabim_core::starter::starter_house();
        let scene = build_scene(&m);
        let linings: Vec<&SceneItem> = scene.iter().filter(|i| i.is_lining()).collect();
        assert!(!linings.is_empty());
        let insulation = layer_color(creabim_core::LayerFunction::Insulation, [0.0; 3]);
        let insulation_materials: Vec<[f32; 3]> = scene
            .iter()
            .filter(|i| {
                i.layer
                    .is_some_and(|l| l.function == creabim_core::LayerFunction::Insulation)
            })
            .map(|i| i.color)
            .collect();
        for l in &linings {
            assert_ne!(l.color, insulation);
            assert!(!insulation_materials.contains(&l.color), "{:?}", l.color);
            // thin, closed boxes
            assert!(l.mesh.volume() > 0.0);
        }
        // drawings and exports leave them out
        assert!(!drawn_scene(&m).iter().any(|i| i.is_lining()));
        let walls: f64 = element_meshes(&scene).values().map(|m| m.volume()).sum();
        let without: Vec<SceneItem> = scene.iter().filter(|i| !i.is_lining()).cloned().collect();
        let plain: f64 = element_meshes(&without).values().map(|m| m.volume()).sum();
        assert!((walls - plain).abs() < 1e-9);
    }

    /// George 2026-11-22: a plinth of XPS, set back from the facade, on a
    /// membrane up the masonry, 300 mm over the ground (here the walls'
    /// base, with no terrain); the facade insulation and render above it.
    #[test]
    fn a_plinth_swaps_the_facade_insulation_for_xps_below_its_top() {
        use creabim_core::plinth::Plinth;
        use creabim_core::{
            AssemblyUse, Document, Layer, LayerFunction as F, Material, Transaction,
        };
        let mut doc = Document::from_model(creabim_core::sample::layered_house());
        let mat = |n: &str| Element::new(n, ElementKind::Material(Material::default()));
        let (xps, foil, sokel, eps) =
            (mat("XPS"), mat("Fólia"), mat("Soklová omietka"), mat("EPS"));
        let (xid, fid, sid, eid) = (xps.id, foil.id, sokel.id, eps.id);
        doc.commit(
            Transaction::new("m")
                .insert(xps)
                .insert(foil)
                .insert(sokel)
                .insert(eps),
        )
        .unwrap();
        let ids: Vec<ElementId> = doc.model().iter().map(|e| e.id).collect();
        for id in ids {
            if let ElementKind::Assembly(a) = doc.model().get(id).unwrap().kind.clone() {
                if a.usage != AssemblyUse::Wall {
                    continue;
                }
                doc.edit(id, "plinth", |e| {
                    if let ElementKind::Assembly(a) = &mut e.kind {
                        a.layers = vec![
                            Layer::new(F::Finish, None, 0.005),
                            Layer::new(F::Insulation, Some(eid), 0.15),
                            Layer::new(F::Structure, None, 0.25),
                            Layer::new(F::Finish, None, 0.015),
                        ];
                        let mut p = Plinth::new(0.12);
                        p.insulation = Some(xid);
                        p.membrane = Some(fid);
                        p.render = Some(sid);
                        a.plinth = Some(p);
                    }
                })
                .unwrap();
            }
        }
        let m = doc.model();
        let joins = creabim_core::wall_joins(m);
        let (e, w) = m
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::Wall(w) if w.assembly.is_some() => Some((e, w)),
                _ => None,
            })
            .unwrap();
        let base = creabim_core::plinth::base_z(m, w);
        let parts = layer_parts(m, e, &joins);
        let zs = |mat: ElementId| -> Option<(f64, f64)> {
            let ps: Vec<f64> = parts
                .iter()
                .filter(|(l, _)| l.material == Some(mat))
                .flat_map(|(_, mesh)| mesh.positions.iter().map(|p| p[2] as f64))
                .collect();
            (!ps.is_empty()).then(|| {
                (
                    ps.iter().copied().fold(f64::MAX, f64::min),
                    ps.iter().copied().fold(f64::MIN, f64::max),
                )
            })
        };
        let (x0, x1) = zs(xid).expect("XPS");
        assert!(
            (x0 - base).abs() < 1e-3 && (x1 - (base + 0.3)).abs() < 1e-3,
            "{x0} {x1}"
        );
        let (e0, _) = zs(eid).expect("EPS above");
        assert!(e0 > base + 0.3 - 1e-3, "{e0}");
        let (f0, f1) = zs(fid).expect("membrane");
        assert!((f0 - base).abs() < 1e-3 && (f1 - (base + 0.3)).abs() < 1e-3);
        assert!(zs(sid).is_some(), "the plinth's render");
        // the XPS is 120 thick, set back 30 behind the facade insulation
        let xps_vol: f64 = parts
            .iter()
            .filter(|(l, _)| l.material == Some(xid))
            .map(|(_, mesh)| closed_volume(mesh))
            .sum();
        assert!(xps_vol > 0.0);
    }
}
