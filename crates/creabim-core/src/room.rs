//! Rooms (IFC's IfcSpace), the room separation lines that close them off
//! where there is no wall, and the ceilings hung in them (IfcCovering
//! CEILING).
//!
//! A room is placed by a point inside it. Its outline is never stored: it
//! is found again from the walls and room separators on its level each
//! time it is asked for, to the walls' inner faces, so moving a wall
//! changes the room's area, perimeter and tag at once. A room can also be
//! drawn with an outline of its own, for a space no wall closes off (a
//! terrace, a balcony); then the outline is kept and edited like a slab's.
//!
//! A ceiling hangs `height` above its level, `thickness` thick upwards,
//! either over a room (following the room's outline as walls move) or over
//! an outline of its own. Fixtures hung from the ceiling hang from it, and
//! walls can attach their tops to its underside (see [`crate::walls`]).

use crate::{Element, ElementId, ElementKind, Model, Transaction};
use glam::DVec2;
use serde::{Deserialize, Serialize};

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_blank(s: &str) -> bool {
    s.is_empty()
}

/// A room: a space on a level, named by its element's name.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Room {
    pub level: ElementId,
    /// A point inside it. With no outline of its own, the room is the
    /// space the walls and room separators of its level close off round
    /// this point; its tag sits here too.
    pub at: DVec2,
    /// An outline drawn for it, counter-clockwise; empty when the walls
    /// round `at` bound it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boundary: Vec<DVec2>,
    /// Its number, as schedules and tags show it ("0.01").
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub number: String,
    /// Floor, wall and ceiling finishes, as free text.
    #[serde(default, skip_serializing_if = "is_blank")]
    pub floor_finish: String,
    #[serde(default, skip_serializing_if = "is_blank")]
    pub wall_finish: String,
    #[serde(default, skip_serializing_if = "is_blank")]
    pub ceiling_finish: String,
    /// What it is used for ("Bedroom", "Office"), IFC's occupancy type.
    #[serde(default, skip_serializing_if = "is_blank")]
    pub occupancy: String,
    /// The department or dwelling it belongs to.
    #[serde(default, skip_serializing_if = "is_blank")]
    pub department: String,
    /// Height of its top above its level; none reaches up to the ceiling
    /// over it, or to the level's floor-to-floor height.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    /// The fire zone it belongs to ("N1.01"); rooms with one code make
    /// a zone.
    #[serde(default, skip_serializing_if = "is_blank")]
    pub fire_zone: String,
    /// What it is counted as for its persons: an item of the project's
    /// occupancy table ("9.1").
    #[serde(default, skip_serializing_if = "is_blank")]
    pub fire_use: String,
    /// The count the item's factor multiplies: the designed persons,
    /// beds, seats or parking places.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design_count: Option<f64>,
    /// What it is for its fire load: an item of the project's fire load
    /// table ("1.1"); empty takes it as a corridor's remaining area.
    #[serde(default, skip_serializing_if = "is_blank")]
    pub fire_load: String,
    /// Its fixed fire load ps, kg/m², where the designer gives it; none
    /// takes windows, doors and floors from the table by its area.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fire_ps: Option<f64>,
    /// The design fire load pvm of a concentrated fire load in it,
    /// kg/m², where the designer gives one: its openings have their own
    /// heat flux (STN 92 0201-4 4.4.5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fire_pvm: Option<f64>,
    /// The tiling of its walls, where it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tiling: Option<Tiling>,
    /// Tiling on part of its walls only, each its own stretch (a
    /// kitchen's splashback over the base cabinets).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tiling_parts: Vec<Tiling>,
    /// Its underfloor heating, where it has one: the loops drawn in it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ufh: Option<crate::underfloor::Underfloor>,
    /// Aired by its own room heat-recovery units in its outer wall, not
    /// by the house's central unit (the recuperation design).
    #[serde(default, skip_serializing_if = "is_false")]
    pub decentral_hrv: bool,
    /// Counted in the living area or not, where the designer says; none
    /// goes by its type and area ([`counts_as_living`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub living: Option<bool>,
    /// The temperature it is heated to, °C, where the designer sets it
    /// (a corridor inside the house heated as the rooms, George
    /// 2026-11-24); none goes by its type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heated_to: Option<f64>,
}

/// Tiles on a room's walls: `height` high from `bottom` above the floor,
/// on every wall of the room (or the stretch `span` of them), the
/// openings left out.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tiling {
    /// How high the tiles are, m (from the floor, unless `bottom`).
    pub height: f64,
    /// How far above the drawn storey's floor the tiles start, m.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub bottom: f64,
    /// What it is ("Ceramic tiles 300x600").
    #[serde(default, skip_serializing_if = "is_blank")]
    pub name: String,
    /// The walls of the room left without tiles.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skip: Vec<ElementId>,
    /// Only the stretch of the room's walls from the first point round
    /// to the second, the shorter way: a kitchen's splashback.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<[DVec2; 2]>,
    /// Where its mark "(height)(bottom)" was dragged to: it sits on the
    /// tiling's line nearest this point; none is the middle of its
    /// longest stretch.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<DVec2>,
    /// Drawn without its mark.
    #[serde(default, skip_serializing_if = "is_false")]
    pub no_mark: bool,
}

impl Tiling {
    /// Its top above the floor, m.
    pub fn top(&self) -> f64 {
        self.bottom + self.height
    }

    /// Its mark as in the drawings: "(600)(900)", the tiles' height and
    /// how far above the floor they start, mm (George 2026-11-24).
    pub fn mark_text(&self) -> String {
        format!("({:.0})({:.0})", self.height * 1000.0, self.bottom * 1000.0)
    }
}

/// The height new tiling reaches.
pub const TILING_HEIGHT: f64 = 2.0;

impl Default for Tiling {
    fn default() -> Self {
        Self {
            height: TILING_HEIGHT,
            bottom: 0.0,
            name: String::new(),
            skip: vec![],
            span: None,
            mark: None,
            no_mark: false,
        }
    }
}

/// The stretches of the edge `a` to `b` of a room that the openings of
/// `kind` in `wall` take, each with its opening.
fn opening_spans<'a>(
    model: &'a Model,
    wall: &Element,
    w: &crate::Wall,
    kind: crate::OpeningKind,
    a: DVec2,
    b: DVec2,
) -> Vec<(f64, f64, &'a crate::Opening)> {
    let len = a.distance(b);
    let along = (w.end - w.start).normalize_or_zero();
    if len < 1e-6 || along == DVec2::ZERO {
        return vec![];
    }
    let d = (b - a) / len;
    let mut out: Vec<(f64, f64, &crate::Opening)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Opening(o) if o.host == wall.id && o.kind == kind => {
                let p = (w.start + along * o.offset - a).dot(d);
                let q = (w.start + along * (o.offset + o.width) - a).dot(d);
                Some((p.min(q).max(0.0), p.max(q).min(len), o))
            }
            _ => None,
        })
        .filter(|g| g.1 - g.0 > 1e-6)
        .collect();
    out.sort_by(|x, y| x.0.total_cmp(&y.0));
    out
}

/// The walls round a room, each once, in the outline's order.
pub fn bounding_walls(model: &Model, shape: &RoomShape) -> Vec<ElementId> {
    let mut out = vec![];
    for id in shape.bounded_by.iter().flatten() {
        if !out.contains(id)
            && matches!(model.get(*id).map(|e| &e.kind), Some(ElementKind::Wall(_)))
        {
            out.push(*id);
        }
    }
    out
}

/// Where a room's tiling runs: the stretches of its outline along
/// walls, less the doors in them and the walls in `skip`.
pub fn tiling_runs(model: &Model, shape: &RoomShape, skip: &[ElementId]) -> Vec<(DVec2, DVec2)> {
    let n = shape.outline.len();
    let mut out = vec![];
    for (i, by) in shape.bounded_by.iter().enumerate() {
        let Some(wall) = by.and_then(|id| model.get(id)) else {
            continue;
        };
        if skip.contains(&wall.id) {
            continue;
        }
        let ElementKind::Wall(w) = &wall.kind else {
            continue;
        };
        let (a, b) = (shape.outline[i], shape.outline[(i + 1) % n]);
        let len = a.distance(b);
        let along = (w.end - w.start).normalize_or_zero();
        if len < 1e-6 || along == DVec2::ZERO {
            continue;
        }
        let d = (b - a) / len;
        // the doors of the wall, as stretches of the edge
        let gaps = opening_spans(model, wall, w, crate::OpeningKind::Door, a, b);
        let mut from = 0.0;
        for (p, q, _) in gaps {
            if p - from > 1e-6 {
                out.push((a + d * from, a + d * p));
            }
            from = from.max(q);
        }
        if len - from > 1e-6 {
            out.push((a + d * from, b));
        }
    }
    out
}

/// One rectangle of tiles on a wall's face: from `a` to `b` along the
/// room's outline, from `z0` to `z1` above the room's floor, m.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TilePanel {
    pub a: DVec2,
    pub b: DVec2,
    pub z0: f64,
    pub z1: f64,
}

/// Every opening of `wall` (doors, windows and holes) as a stretch of
/// the edge `a` to `b` with how high it runs above the room's floor.
fn opening_boxes(
    model: &Model,
    wall: &Element,
    w: &crate::Wall,
    a: DVec2,
    b: DVec2,
) -> Vec<(f64, f64, f64, f64)> {
    let mut out = vec![];
    for kind in [
        crate::OpeningKind::Door,
        crate::OpeningKind::Window,
        crate::OpeningKind::Hole,
    ] {
        for (p, q, o) in opening_spans(model, wall, w, kind, a, b) {
            let bottom = w.base_offset + o.sill;
            out.push((p, q, bottom, bottom + o.height));
        }
    }
    out
}

/// The tiles of a room's walls as rectangles: on every wall not in
/// `skip`, from the floor up to `height`, less every door, window and
/// hole in the wall, so that nothing is tiled over an opening.
pub fn tiling_panels(
    model: &Model,
    shape: &RoomShape,
    skip: &[ElementId],
    height: f64,
) -> Vec<TilePanel> {
    tiling_panels_of(
        model,
        shape,
        &Tiling {
            height,
            skip: skip.to_vec(),
            ..Tiling::default()
        },
    )
}

/// Where along each edge of a room's outline a tiling runs, as stretches
/// from the edge's start, m: the whole edge, or the part of it in the
/// tiling's span.
fn edge_clips(shape: &RoomShape, span: Option<[DVec2; 2]>) -> Vec<Vec<(f64, f64)>> {
    let n = shape.outline.len();
    let lens: Vec<f64> = (0..n)
        .map(|i| shape.outline[i].distance(shape.outline[(i + 1) % n]))
        .collect();
    let Some([p, q]) = span else {
        return lens.iter().map(|l| vec![(0.0, *l)]).collect();
    };
    let total: f64 = lens.iter().sum();
    let mut out = vec![vec![]; n];
    if total < 1e-9 {
        return out;
    }
    // where a point falls round the outline, m from its first corner
    let at = |p: DVec2| {
        let mut best = (f64::INFINITY, 0.0);
        let mut s0 = 0.0;
        for (i, &l) in lens.iter().enumerate() {
            let (a, b) = (shape.outline[i], shape.outline[(i + 1) % n]);
            let t = if l > 1e-9 {
                ((p - a).dot(b - a) / (l * l)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let d = p.distance(a + (b - a) * t);
            if d < best.0 {
                best = (d, s0 + t * l);
            }
            s0 += l;
        }
        best.1
    };
    let (mut s, mut e) = (at(p), at(q));
    if s > e {
        std::mem::swap(&mut s, &mut e);
    }
    // the shorter way round
    let ranges: Vec<(f64, f64)> = if e - s <= total - (e - s) {
        vec![(s, e)]
    } else {
        vec![(e, total), (0.0, s)]
    };
    let mut s0 = 0.0;
    for i in 0..n {
        for (r0, r1) in &ranges {
            let (x0, x1) = ((r0 - s0).max(0.0), (r1 - s0).min(lens[i]));
            if x1 - x0 > 1e-6 {
                out[i].push((x0, x1));
            }
        }
        s0 += lens[i];
    }
    out
}

/// The tiles of one tiling as rectangles: on its walls (and its stretch
/// of them), `height` high from `bottom`, less every door, window and
/// hole in the wall, so that nothing is tiled over an opening.
pub fn tiling_panels_of(model: &Model, shape: &RoomShape, t: &Tiling) -> Vec<TilePanel> {
    const EPS: f64 = 1e-6;
    let n = shape.outline.len();
    let mut out = vec![];
    let (bottom, top) = (t.bottom, t.top());
    if t.height <= EPS {
        return out;
    }
    let clips = edge_clips(shape, t.span);
    for (i, by) in shape.bounded_by.iter().enumerate() {
        let Some(wall) = by.and_then(|id| model.get(id)) else {
            continue;
        };
        if t.skip.contains(&wall.id) || clips[i].is_empty() {
            continue;
        }
        let ElementKind::Wall(w) = &wall.kind else {
            continue;
        };
        let (a, b) = (shape.outline[i], shape.outline[(i + 1) % n]);
        let len = a.distance(b);
        if len < EPS || (w.end - w.start).length() < EPS {
            continue;
        }
        let d = (b - a) / len;
        // the openings that reach into the tiled height
        let holes: Vec<_> = opening_boxes(model, wall, w, a, b)
            .into_iter()
            .filter(|h| h.2 < top - EPS && h.3 > bottom + EPS)
            .collect();
        let mut cuts = vec![0.0, len];
        for h in &holes {
            cuts.push(h.0);
            cuts.push(h.1);
        }
        for c in &clips[i] {
            cuts.push(c.0);
            cuts.push(c.1);
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup_by(|x, y| (*x - *y).abs() < EPS);
        // each stretch between the openings' edges: the height less the
        // openings over the whole stretch
        let mut row: Vec<(f64, f64, f64, f64)> = vec![];
        for s in cuts.windows(2) {
            let (x0, x1) = (s[0], s[1]);
            if x1 - x0 < EPS {
                continue;
            }
            let m = (x0 + x1) / 2.0;
            if !clips[i].iter().any(|c| c.0 <= m && m <= c.1) {
                continue;
            }
            let mut over: Vec<(f64, f64)> = holes
                .iter()
                .filter(|h| h.0 <= x0 + EPS && h.1 >= x1 - EPS)
                .map(|h| (h.2.max(bottom), h.3.min(top)))
                .collect();
            over.sort_by(|p, q| p.0.total_cmp(&q.0));
            let mut z = bottom;
            for (lo, hi) in over {
                if lo - z > EPS {
                    row.push((x0, x1, z, lo));
                }
                z = f64::max(z, hi);
            }
            if top - z > EPS {
                row.push((x0, x1, z, top));
            }
        }
        // neighbours of the same height join into one rectangle
        let mut joined: Vec<(f64, f64, f64, f64)> = vec![];
        for r in row {
            match joined.iter_mut().find(|j| {
                (j.1 - r.0).abs() < EPS && (j.2 - r.2).abs() < EPS && (j.3 - r.3).abs() < EPS
            }) {
                Some(j) => j.1 = r.1,
                None => joined.push(r),
            }
        }
        out.extend(joined.into_iter().map(|(x0, x1, z0, z1)| TilePanel {
            a: a + d * x0,
            b: a + d * x1,
            z0,
            z1,
        }));
    }
    out
}

/// Where a room's tiling is drawn in a plan: its runs less every
/// window and hole that reaches into the tiled height, as the doors
/// are left out already, so the line never runs across an opening.
pub fn tiling_plan_runs(
    model: &Model,
    shape: &RoomShape,
    skip: &[ElementId],
    height: f64,
) -> Vec<(DVec2, DVec2)> {
    tiling_plan_runs_of(
        model,
        shape,
        &Tiling {
            height,
            skip: skip.to_vec(),
            ..Tiling::default()
        },
    )
}

/// Where one tiling is drawn in a plan: along its walls (and its stretch
/// of them), less every door, window and hole that reaches into its
/// height, so the line never runs across an opening.
pub fn tiling_plan_runs_of(model: &Model, shape: &RoomShape, t: &Tiling) -> Vec<(DVec2, DVec2)> {
    const EPS: f64 = 1e-6;
    let n = shape.outline.len();
    let (bottom, top) = (t.bottom, t.top());
    let clips = edge_clips(shape, t.span);
    let mut out = vec![];
    for (i, by) in shape.bounded_by.iter().enumerate() {
        let Some(wall) = by.and_then(|id| model.get(id)) else {
            continue;
        };
        if t.skip.contains(&wall.id) {
            continue;
        }
        let ElementKind::Wall(w) = &wall.kind else {
            continue;
        };
        let (a, b) = (shape.outline[i], shape.outline[(i + 1) % n]);
        let len = a.distance(b);
        if len < EPS || (w.end - w.start).length() < EPS {
            continue;
        }
        let d = (b - a) / len;
        let mut gaps: Vec<(f64, f64)> = opening_boxes(model, wall, w, a, b)
            .into_iter()
            .filter(|h| h.2 < top - EPS && h.3 > bottom + EPS)
            .map(|h| (h.0, h.1))
            .collect();
        gaps.sort_by(|x, y| x.0.total_cmp(&y.0));
        for &(c0, c1) in &clips[i] {
            let mut from = c0;
            for &(p, q) in &gaps {
                let (p, q) = (p.max(c0), q.min(c1));
                if q - p <= EPS {
                    continue;
                }
                if p - from > EPS {
                    out.push((a + d * from, a + d * p));
                }
                from = f64::max(from, q);
            }
            if c1 - from > EPS {
                out.push((a + d * from, a + d * c1));
            }
        }
    }
    // and on into the side reveals of the windows it runs across
    for r in tiling_reveals(model, shape, t) {
        if r.normal.z.abs() < 0.5 {
            // the tiles on the left of the run, as along the walls
            let (a, b) = (r.corners[0].truncate(), r.corners[1].truncate());
            if (b - a).perp().dot(r.normal.truncate()) >= 0.0 {
                out.push((a, b));
            } else {
                out.push((b, a));
            }
        }
    }
    out
}

/// A tile in a window's reveal: the quad `corners` (plan point, height
/// above the room's floor), its tiles facing `normal`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RevealTile {
    pub corners: [glam::DVec3; 4],
    pub normal: glam::DVec3,
}

impl RevealTile {
    pub fn area(&self) -> f64 {
        let [a, b, c, _] = self.corners;
        a.distance(b) * b.distance(c)
    }
}

/// The tiles one tiling puts in the reveals of the windows it runs
/// across (George 2026-11-24): the side reveals where the tiles run
/// across the window's height, the head only where they run on above
/// the window, the bottom only where the window has no inner sill and
/// the tiles run across its bottom; from the wall's face to the frame.
pub fn tiling_reveals(model: &Model, shape: &RoomShape, t: &Tiling) -> Vec<RevealTile> {
    use glam::DVec3;
    const EPS: f64 = 1e-6;
    let n = shape.outline.len();
    let (bottom, top) = (t.bottom, t.top());
    let clips = edge_clips(shape, t.span);
    let mut out = vec![];
    if t.height <= EPS {
        return out;
    }
    for (i, by) in shape.bounded_by.iter().enumerate() {
        let Some(wall) = by.and_then(|id| model.get(id)) else {
            continue;
        };
        if t.skip.contains(&wall.id) || clips[i].is_empty() {
            continue;
        }
        let ElementKind::Wall(w) = &wall.kind else {
            continue;
        };
        let (a, b) = (shape.outline[i], shape.outline[(i + 1) % n]);
        let len = a.distance(b);
        let along = (w.end - w.start).normalize_or_zero();
        if len < EPS || along == DVec2::ZERO {
            continue;
        }
        let d = (b - a) / len;
        // into the room, and where the room's face lies across the wall
        let inward = DVec2::new(-d.y, d.x);
        let face = (a - w.start).dot(along.perp());
        let asm = crate::assembly_of(model, wall).map(|(_, a)| a);
        let inside = |x: f64| clips[i].iter().any(|c| c.0 - EPS <= x && x <= c.1 + EPS);
        for (p, q, o) in opening_spans(model, wall, w, crate::OpeningKind::Window, a, b) {
            let frame = o.frame.fitted(o.width, o.height, w.thickness);
            let span = o.frame_span(w, asm, frame.depth);
            let depth = if face >= 0.0 {
                face - span.1
            } else {
                span.0 - face
            };
            if depth < 0.005 {
                continue;
            }
            let z0 = w.base_offset + o.sill;
            let z1 = z0 + o.height;
            let at = |x: f64, back: f64, z: f64| {
                let pt = a + d * x - inward * back;
                DVec3::new(pt.x, pt.y, z)
            };
            // the side reveals, where the tiles run across the window
            let (lo, hi) = (z0.max(bottom), z1.min(top));
            if hi - lo > EPS {
                for (x, facing) in [(p, d), (q, -d)] {
                    if inside(x) {
                        out.push(RevealTile {
                            corners: [
                                at(x, 0.0, lo),
                                at(x, depth, lo),
                                at(x, depth, hi),
                                at(x, 0.0, hi),
                            ],
                            normal: DVec3::new(facing.x, facing.y, 0.0),
                        });
                    }
                }
            }
            let across = inside((p + q) / 2.0);
            // the head, where the tiles run on above the window
            if across && top > z1 + EPS && bottom < z1 - EPS {
                out.push(RevealTile {
                    corners: [
                        at(p, 0.0, z1),
                        at(q, 0.0, z1),
                        at(q, depth, z1),
                        at(p, depth, z1),
                    ],
                    normal: DVec3::NEG_Z,
                });
            }
            // the bottom, with no inner sill, where the tiles run across it
            if across && o.no_inner_sill && bottom < z0 + EPS && top > z0 + EPS {
                out.push(RevealTile {
                    corners: [
                        at(p, 0.0, z0),
                        at(q, 0.0, z0),
                        at(q, depth, z0),
                        at(p, depth, z0),
                    ],
                    normal: DVec3::Z,
                });
            }
        }
    }
    out
}

/// The area of one tiling, m²: on the walls and in the reveals.
pub fn tiling_area_of(model: &Model, shape: &RoomShape, t: &Tiling) -> f64 {
    let walls: f64 = tiling_panels_of(model, shape, t)
        .iter()
        .map(|p| p.a.distance(p.b) * (p.z1 - p.z0))
        .sum();
    let reveals: f64 = tiling_reveals(model, shape, t)
        .iter()
        .map(|r| r.area())
        .sum();
    walls + reveals
}

/// Where a tiling's mark sits on its line in a plan: the point on its
/// runs nearest where it was dragged (or the middle of the longest run)
/// and the run's direction.
pub fn tiling_mark_at(model: &Model, shape: &RoomShape, t: &Tiling) -> Option<(DVec2, DVec2)> {
    let runs = tiling_plan_runs_of(model, shape, t);
    let dir = |a: DVec2, b: DVec2| (b - a).normalize_or_zero();
    match t.mark {
        Some(p) => runs
            .iter()
            .map(|(a, b)| {
                let l2 = a.distance_squared(*b).max(1e-12);
                let s = ((p - *a).dot(*b - *a) / l2).clamp(0.0, 1.0);
                (*a + (*b - *a) * s, dir(*a, *b))
            })
            .min_by(|x, y| x.0.distance(p).total_cmp(&y.0.distance(p))),
        None => runs
            .iter()
            .max_by(|x, y| x.0.distance(x.1).total_cmp(&y.0.distance(y.1)))
            .map(|(a, b)| ((*a + *b) / 2.0, dir(*a, *b))),
    }
}

/// The area of a room's tiling, m²: its rectangles, so every door,
/// window and hole in the tiled height is left out; the tiling on part
/// of its walls too.
pub fn tiling_area(model: &Model, r: &Room, shape: &RoomShape) -> f64 {
    r.tilings().map(|t| tiling_area_of(model, shape, t)).sum()
}

impl Room {
    /// A room round `at` on `level`, bounded by the walls there.
    pub fn new(level: ElementId, at: DVec2) -> Self {
        Self {
            level,
            at,
            ..Default::default()
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.at.is_finite() || self.boundary.iter().any(|p| !p.is_finite()) {
            return Err("a room's point and outline must be finite".into());
        }
        if !self.boundary.is_empty() && self.boundary.len() < 3 {
            return Err("a drawn room needs at least 3 corners".into());
        }
        if self.height.is_some_and(|h| !(h.is_finite() && h > 0.0)) {
            return Err("a room's height must be positive".into());
        }
        if let Some(u) = &self.ufh {
            u.validate()?;
        }
        for t in self.tilings() {
            if !(t.height.is_finite() && t.bottom.is_finite())
                || t.span
                    .is_some_and(|s| !(s[0].is_finite() && s[1].is_finite()))
            {
                return Err("a room's tiling must be finite".into());
            }
        }
        Ok(())
    }

    /// Is its outline drawn rather than found from the walls?
    pub fn is_drawn(&self) -> bool {
        !self.boundary.is_empty()
    }

    /// Its tilings: the one on all its walls, then those on part of them.
    pub fn tilings(&self) -> impl Iterator<Item = &Tiling> {
        self.tiling.iter().chain(self.tiling_parts.iter())
    }

    /// Its `i`th tiling, counted as [`Room::tilings`] does.
    pub fn tiling_mut(&mut self, i: usize) -> Option<&mut Tiling> {
        match (&mut self.tiling, i) {
            (Some(t), 0) => Some(t),
            (Some(_), i) => self.tiling_parts.get_mut(i - 1),
            (None, i) => self.tiling_parts.get_mut(i),
        }
    }
}

/// A room separation line: closes rooms off where there is no wall, as
/// between a kitchen and the living room it opens into. Drawn in plans
/// only; it has no body.
/// One thing a room tag can show. A tag lists the fields it shows; one
/// that lists none shows the room's name, number and area.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoomField {
    Name,
    Number,
    Area,
    Perimeter,
    Height,
    Level,
    FloorFinish,
    WallFinish,
    CeilingFinish,
    Occupancy,
    Department,
    FireZone,
}

impl RoomField {
    pub const ALL: [RoomField; 12] = [
        RoomField::Name,
        RoomField::Number,
        RoomField::Area,
        RoomField::Perimeter,
        RoomField::Height,
        RoomField::Level,
        RoomField::FloorFinish,
        RoomField::WallFinish,
        RoomField::CeilingFinish,
        RoomField::Occupancy,
        RoomField::Department,
        RoomField::FireZone,
    ];
    /// What a tag shows when it lists no fields.
    pub const DEFAULT: [RoomField; 3] = [RoomField::Name, RoomField::Number, RoomField::Area];

    pub fn label(self) -> &'static str {
        match self {
            RoomField::Name => "Name",
            RoomField::Number => "Number",
            RoomField::Area => "Area",
            RoomField::Perimeter => "Perimeter",
            RoomField::Height => "Height",
            RoomField::Level => "Level",
            RoomField::FloorFinish => "Floor finish",
            RoomField::WallFinish => "Wall finish",
            RoomField::CeilingFinish => "Ceiling finish",
            RoomField::Occupancy => "Occupancy",
            RoomField::Department => "Department",
            RoomField::FireZone => "Fire zone",
        }
    }

    /// This field of a room as its tag writes it; empty when the room has
    /// none, and "Not enclosed" for the area or perimeter of a room its
    /// walls don't close off.
    pub fn value(self, model: &Model, room: ElementId) -> String {
        let Some(e) = model.get(room) else {
            return String::new();
        };
        let ElementKind::Room(r) = &e.kind else {
            return String::new();
        };
        let units = model.units();
        match self {
            RoomField::Name => e.name.clone(),
            RoomField::Number => r.number.clone(),
            RoomField::Area => shape_of(model, r).map_or("Not enclosed".into(), |s| {
                units.area(bare_area(model, r, &s))
            }),
            RoomField::Perimeter => {
                shape_of(model, r).map_or("Not enclosed".into(), |s| units.dimension(s.perimeter()))
            }
            RoomField::Height => units.dimension(height(model, r)),
            RoomField::Level => model
                .get(r.level)
                .map(|l| l.name.clone())
                .unwrap_or_default(),
            RoomField::FloorFinish => r.floor_finish.clone(),
            RoomField::WallFinish => r.wall_finish.clone(),
            RoomField::CeilingFinish => r.ceiling_finish.clone(),
            RoomField::Occupancy => r.occupancy.clone(),
            RoomField::Department => r.department.clone(),
            RoomField::FireZone => r.fire_zone.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Separator {
    pub level: ElementId,
    pub a: DVec2,
    pub b: DVec2,
    /// How it is drawn: a pen and a line type (George 2026-11-22);
    /// none, the plain thin line it always had.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<crate::LineLook>,
    /// A slab filling a room stops at it too, as at a wall's centre line,
    /// so two floor build-ups can meet on it (George 2026-11-22).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub bounds_slabs: bool,
}

impl Separator {
    pub fn validate(&self) -> Result<(), String> {
        if self.a.is_finite() && self.b.is_finite() && self.a.distance(self.b) > 1e-6 {
            Ok(())
        } else {
            Err("a room separator needs two different points".into())
        }
    }
}

/// A ceiling: a flat layer hung `height` above its level, `thickness`
/// thick upwards from its underside.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ceiling {
    pub level: ElementId,
    /// Its outline, counter-clockwise; empty when it follows `room`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boundary: Vec<DVec2>,
    /// The room whose outline it follows, when it has none of its own.
    /// Soft: when the room goes the ceiling keeps nothing to stand on and
    /// is left out until it is given an outline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room: Option<ElementId>,
    /// Height of its underside above the level.
    pub height: f64,
    pub thickness: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<ElementId>,
    /// The tile size of a suspended ceiling's grid, drawn in ceiling
    /// plans; 0 for a plain (plastered) one.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub grid: f64,
    /// The build-up it is made of (a ceiling's [`crate::AssemblyUse::Ceiling`]
    /// assembly, layers from the top down): with one its thickness is the
    /// build-up's, and the heat loss counts it over the room (George
    /// 2026-11-23).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assembly: Option<ElementId>,
    /// The roof space over it, a row of STN EN ISO 6946 table 9 (its key in
    /// the norm's parameter table [`ROOF_SPACE`]): its thermal resistance
    /// Ru is added to the build-up's. None: no roof space counted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roof_space: Option<String>,
}

/// STN EN ISO 6946:2019 table 9, thermal resistance of roof spaces: the
/// parameter table read from the norm (columns `key`, `ru` m²K/W, `text`).
/// The values come from the user's copy of the norm, never from the code.
pub const ROOF_SPACE: &str = "roof_space";

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// The rows of STN EN ISO 6946 table 9 read ([`ROOF_SPACE`]): key, Ru
/// m²K/W, what roof it is.
pub fn roof_spaces(tables: &[crate::fireparams::ParamTable]) -> Vec<(String, f64, String)> {
    let Some(t) = tables.iter().find(|t| t.id == ROOF_SPACE) else {
        return vec![];
    };
    let (Some(k), Some(r), x) = (t.column("key"), t.column("ru"), t.column("text")) else {
        return vec![];
    };
    t.rows
        .iter()
        .filter_map(|row| {
            Some((
                row.get(k)?.clone(),
                crate::fireparams::number(row.get(r)?)?,
                x.and_then(|x| row.get(x)).cloned().unwrap_or_default(),
            ))
        })
        .collect()
}

/// The norm tables of the project (its own over the application's).
pub fn project_tables(model: &Model) -> Vec<crate::fireparams::ParamTable> {
    let own = model
        .project_info()
        .map(|(_, p)| p.fire_params.clone())
        .unwrap_or_default();
    crate::fireparams::effective(&own)
}

/// The roof space's Ru over ceiling `c`, m²K/W, by its row of table 9;
/// none when it has none or the table isn't read.
pub fn roof_space_ru(tables: &[crate::fireparams::ParamTable], c: &Ceiling) -> Option<f64> {
    let key = c.roof_space.as_deref()?;
    roof_spaces(tables)
        .into_iter()
        .find(|(k, _, _)| k == key)
        .map(|(_, ru, _)| ru)
}

/// The ceiling with a build-up over room `room` (on `level`, its point
/// `at`): one that follows the room, else one whose outline on the room's
/// level holds the point.
pub fn built_ceiling_over(
    model: &Model,
    room: ElementId,
    level: ElementId,
    at: DVec2,
) -> Option<&Element> {
    let built = |c: &Ceiling| {
        c.assembly
            .and_then(|a| model.get(a))
            .is_some_and(|a| matches!(&a.kind, ElementKind::Assembly(x) if !x.layers.is_empty()))
    };
    model
        .iter()
        .find(|e| matches!(&e.kind, ElementKind::Ceiling(c) if c.room == Some(room) && c.boundary.is_empty() && built(c)))
        .or_else(|| {
            model.iter().find(|e| {
                matches!(&e.kind, ElementKind::Ceiling(c) if c.level == level
                    && built(c)
                    && c.boundary.len() > 2
                    && crate::polygon::contains(&c.boundary, at))
            })
        })
}

/// Ceilings (podhľady) drawn before ceilings had build-ups of their own,
/// as slabs of a ceiling's build-up (George 2026-11-23): each such slab a
/// ceiling of that build-up, its underside where the slab's was, and the
/// build-up a ceiling's. The slab types of those build-ups go, none being
/// left of them. None when there is nothing to change.
pub fn mend_ceilings(model: &Model) -> Option<Transaction> {
    use crate::{AssemblyUse, ElementKind as K};
    let ceiling_asm = |id: ElementId| {
        model.get(id).is_some_and(|a| match &a.kind {
            K::Assembly(x) => {
                x.usage == AssemblyUse::Ceiling
                    || (x.usage == AssemblyUse::Floor && a.name.starts_with("Podhľad"))
            }
            _ => false,
        })
    };
    let mut tx = Transaction::new("Ceilings of their own");
    let mut any = false;
    for e in model.iter() {
        match &e.kind {
            K::Slab(s) if s.assembly.is_some_and(ceiling_asm) => {
                let Some(asm) = s.assembly else { continue };
                let mut c = Ceiling::drawn(s.level, s.boundary.clone(), s.top_offset - s.thickness);
                c.thickness = s.thickness;
                c.material = s.material;
                c.assembly = Some(asm);
                let mut after = e.clone();
                after.kind = K::Ceiling(c);
                after.of_type = None;
                if after.name == "Slab" || after.name.is_empty() {
                    after.name = "Ceiling".into();
                }
                tx = tx.replace(e.clone(), after);
                any = true;
            }
            K::Assembly(a) if a.usage == AssemblyUse::Floor && e.name.starts_with("Podhľad") => {
                let mut after = e.clone();
                if let K::Assembly(x) = &mut after.kind {
                    x.usage = AssemblyUse::Ceiling;
                }
                tx = tx.replace(e.clone(), after);
                any = true;
            }
            _ => {}
        }
    }
    // the slab types of the build-ups now a ceiling's, with no slab left
    for e in model.iter() {
        if let K::ElementType(t) = &e.kind {
            if let crate::types::TypeParams::Slab(st) = &t.params {
                if st.assembly.is_some_and(ceiling_asm) {
                    let used = model.iter().any(|x| {
                        x.of_type.as_ref().is_some_and(|r| r.id == e.id)
                            && !matches!(&x.kind, K::Slab(s) if s.assembly.is_some_and(ceiling_asm))
                    });
                    if !used {
                        tx.ops.push(crate::ops::Op::Remove(e.clone()));
                        any = true;
                    }
                }
            }
        }
    }
    any.then_some(tx)
}

/// The height new ceilings hang at, and their thickness.
pub const CEILING_HEIGHT: f64 = 2.5;
pub const CEILING_THICKNESS: f64 = 0.0125;

impl Ceiling {
    /// A ceiling over `room`, following its outline.
    pub fn in_room(level: ElementId, room: ElementId, height: f64) -> Self {
        Self {
            level,
            boundary: vec![],
            room: Some(room),
            height,
            thickness: CEILING_THICKNESS,
            material: None,
            grid: 0.0,
            assembly: None,
            roof_space: None,
        }
    }

    /// A ceiling with an outline of its own.
    pub fn drawn(level: ElementId, mut boundary: Vec<DVec2>, height: f64) -> Self {
        if crate::polygon::signed_area2(&boundary) < 0.0 {
            boundary.reverse();
        }
        Self {
            level,
            boundary,
            room: None,
            height,
            thickness: CEILING_THICKNESS,
            material: None,
            grid: 0.0,
            assembly: None,
            roof_space: None,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if !(self.thickness.is_finite() && self.thickness > 0.0) {
            return Err(format!(
                "ceiling thickness must be positive, got {}",
                self.thickness
            ));
        }
        if !self.height.is_finite() {
            return Err("a ceiling's height must be a number".into());
        }
        if !(self.grid.is_finite() && (self.grid == 0.0 || self.grid >= 0.1)) {
            return Err("a ceiling's grid must be 0 (none) or at least 100 mm".into());
        }
        if self.boundary.iter().any(|p| !p.is_finite()) {
            return Err("a ceiling's outline must be finite".into());
        }
        if self.boundary.is_empty() && self.room.is_none() {
            return Err("a ceiling needs an outline or a room".into());
        }
        if !self.boundary.is_empty() && self.boundary.len() < 3 {
            return Err("a ceiling's outline needs at least 3 corners".into());
        }
        Ok(())
    }

    /// Absolute heights of its underside and its top.
    pub fn span(&self, model: &Model) -> (f64, f64) {
        let z = model.level(self.level).map_or(0.0, |l| l.elevation) + self.height;
        (z, z + self.thickness)
    }

    /// Its outline in plan: its own, or its room's as the walls stand now.
    pub fn outline(&self, model: &Model) -> Option<Vec<DVec2>> {
        if !self.boundary.is_empty() {
            return Some(self.boundary.clone());
        }
        outline(model, self.room?)
    }
}

// ------------------------------------------------------------ boundaries

/// Where a room's outline comes from, edge by edge.
#[derive(Clone, Debug, PartialEq)]
pub struct RoomShape {
    /// The outline, counter-clockwise.
    pub outline: Vec<DVec2>,
    /// The wall or room separator each edge (corner `i` to `i + 1`) runs
    /// along, if any.
    pub bounded_by: Vec<Option<ElementId>>,
}

impl RoomShape {
    pub fn area(&self) -> f64 {
        crate::polygon::signed_area2(&self.outline).abs() / 2.0
    }

    pub fn perimeter(&self) -> f64 {
        perimeter(&self.outline)
    }
}

/// The length round a closed outline.
pub fn perimeter(poly: &[DVec2]) -> f64 {
    let n = poly.len();
    (0..n).map(|i| poly[i].distance(poly[(i + 1) % n])).sum()
}

/// The lines that bound rooms on `level`: both faces of every wall, run on
/// at their ends far enough to meet the faces of the walls they stop
/// against, and every room separator, run on as far as the walls are so
/// that one drawn to a face, to a wall's centre line or a snap short of
/// the face still crosses it.
fn bounding_lines(model: &Model, level: ElementId) -> Vec<(DVec2, DVec2, ElementId)> {
    let walls: Vec<(ElementId, &crate::Wall)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Wall(w) if w.level == level && w.length() > 1e-3 => Some((e.id, w)),
            _ => None,
        })
        .collect();
    let reach = walls
        .iter()
        .map(|w| w.1.thickness / 2.0)
        .fold(0.0, f64::max)
        + 0.01;
    let mut out = Vec::new();
    for (id, w) in &walls {
        if w.is_arc() {
            // a curved wall's faces round its arc, piece by piece, run on
            // along its tangent at each end as a straight one's are
            let t = w.thickness / 2.0;
            let mut us = vec![0.0];
            us.extend(w.bends());
            us.push(w.length());
            for side in [t, -t] {
                let mut pts: Vec<DVec2> = us.iter().map(|u| w.place(*u, side)).collect();
                let last = pts.len() - 1;
                pts[0] -= w.tangent_at(0.0) * reach;
                pts[last] += w.tangent_at(w.length()) * reach;
                for k in 0..last {
                    out.push((pts[k], pts[k + 1], *id));
                }
            }
            continue;
        }
        let d = w.direction();
        let n = d.perp() * w.thickness / 2.0;
        let (a, b) = (w.start - d * reach, w.end + d * reach);
        out.push((a + n, b + n, *id));
        out.push((a - n, b - n, *id));
    }
    for e in model.iter() {
        if let ElementKind::RoomSeparator(s) = &e.kind {
            if s.level == level && s.a.distance(s.b) > 1e-6 {
                let d = (s.b - s.a).normalize();
                out.push((s.a - d * reach.max(0.1), s.b + d * reach.max(0.1), e.id));
            }
        }
    }
    out
}

/// The space the walls and room separators of `level` close off round
/// `p`, to the walls' faces; `None` when nothing closes it off.
pub fn enclosed(model: &Model, level: ElementId, p: DVec2) -> Option<RoomShape> {
    let lines = bounding_lines(model, level);
    let segs: Vec<(DVec2, DVec2)> = lines.iter().map(|l| (l.0, l.1)).collect();
    let outline = crate::slab::room_in_lines(&segs, p)?;
    // a point inside a wall finds the wall itself: that is no room
    let thin = crate::polygon::signed_area2(&outline).abs() / 2.0 < 0.05;
    if thin {
        return None;
    }
    let n = outline.len();
    let bounded_by = (0..n)
        .map(|i| {
            let (a, b) = (outline[i], outline[(i + 1) % n]);
            let mid = (a + b) / 2.0;
            lines
                .iter()
                .find(|l| point_on_segment(mid, l.0, l.1) && point_on_segment(a, l.0, l.1))
                .or_else(|| lines.iter().find(|l| point_on_segment(mid, l.0, l.1)))
                .map(|l| l.2)
        })
        .collect();
    Some(RoomShape {
        outline,
        bounded_by,
    })
}

/// Where the walls round `p` leave its space open, when they don't close
/// it: a line across each gap between two walls that stop short of each
/// other, in sight of `p`, or from a wall that stops in the open to the
/// nearest face. Empty when nothing is near enough to say.
pub fn gaps(model: &Model, level: ElementId, p: DVec2) -> Vec<(DVec2, DVec2)> {
    let lines = bounding_lines(model, level);
    let segs: Vec<(DVec2, DVec2)> = lines.iter().map(|l| (l.0, l.1)).collect();
    let reach = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Wall(w) if w.level == level => Some(w.thickness / 2.0),
            _ => None,
        })
        .fold(0.0, f64::max)
        + 0.01;
    // a line crossing nothing on its way from p
    let clear = |q: DVec2, own: Option<usize>| {
        segs.iter()
            .enumerate()
            .all(|(k, &(a, b))| Some(k) == own || !crosses_before(p, q, a, b))
    };
    // the loose ends, back at the walls' real ends
    let mut ends: Vec<(DVec2, usize)> = crate::slab::loose_ends(&segs, reach * 1.5)
        .into_iter()
        .map(|(q, k)| {
            let (a, b) = segs[k];
            let out = if q.distance(a) < q.distance(b) {
                a - b
            } else {
                b - a
            };
            let back = reach.min(out.length() / 2.0);
            (q - out.normalize_or_zero() * back, k)
        })
        .filter(|(q, k)| q.distance(p) < 30.0 && clear(*q, Some(*k)))
        .collect();
    ends.sort_by(|x, y| x.0.distance(p).total_cmp(&y.0.distance(p)));
    let mut used = vec![false; ends.len()];
    let mut out = vec![];
    // the nearest pairs first
    let mut pairs: Vec<(f64, usize, usize)> = vec![];
    for i in 0..ends.len() {
        for j in i + 1..ends.len() {
            if lines[ends[i].1].2 != lines[ends[j].1].2 {
                pairs.push((ends[i].0.distance(ends[j].0), i, j));
            }
        }
    }
    pairs.sort_by(|x, y| x.0.total_cmp(&y.0));
    for (d, i, j) in pairs {
        if d < 6.0 && !used[i] && !used[j] {
            used[i] = true;
            used[j] = true;
            out.push((ends[i].0, ends[j].0));
        }
    }
    // one left over runs to the nearest face
    for (i, &(q, k)) in ends.iter().enumerate() {
        if used[i] {
            continue;
        }
        let near = segs
            .iter()
            .enumerate()
            .filter(|(m, _)| *m != k && lines[*m].2 != lines[k].2)
            .map(|(_, &(a, b))| closest_on_segment(q, a, b))
            .filter(|c| c.distance(q) > 1e-3)
            .min_by(|x, y| x.distance(q).total_cmp(&y.distance(q)));
        if let Some(c) = near.filter(|c| c.distance(q) < 6.0) {
            out.push((q, c));
        }
    }
    out
}

/// Does segment pq cross segment ab before reaching q?
fn crosses_before(p: DVec2, q: DVec2, a: DVec2, b: DVec2) -> bool {
    let (r, s) = (q - p, b - a);
    let den = r.perp_dot(s);
    if den.abs() < 1e-12 {
        return false;
    }
    let t = (a - p).perp_dot(s) / den;
    let u = (a - p).perp_dot(r) / den;
    (1e-9..0.98).contains(&t) && (-1e-9..=1.0 + 1e-9).contains(&u)
}

fn closest_on_segment(q: DVec2, a: DVec2, b: DVec2) -> DVec2 {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 < 1e-18 {
        return a;
    }
    a + ab * ((q - a).dot(ab) / len2).clamp(0.0, 1.0)
}

fn point_on_segment(p: DVec2, a: DVec2, b: DVec2) -> bool {
    let ab = b - a;
    let len2 = ab.length_squared();
    if len2 < 1e-18 {
        return false;
    }
    let t = (p - a).dot(ab) / len2;
    (-1e-9..=1.0 + 1e-9).contains(&t) && (a + ab * t).distance(p) < 1e-6
}

/// A room's shape as the model stands: its drawn outline, or the space its
/// walls close off. `None` for a room that is not enclosed (or not a room).
pub fn shape(model: &Model, room: ElementId) -> Option<RoomShape> {
    let ElementKind::Room(r) = &model.get(room)?.kind else {
        return None;
    };
    shape_of(model, r)
}

/// [`shape`], for a room that need not be in the model yet.
pub fn shape_of(model: &Model, r: &Room) -> Option<RoomShape> {
    if r.is_drawn() {
        let mut outline = r.boundary.clone();
        if crate::polygon::signed_area2(&outline) < 0.0 {
            outline.reverse();
        }
        let n = outline.len();
        return Some(RoomShape {
            outline,
            bounded_by: vec![None; n],
        });
    }
    enclosed(model, r.level, r.at)
}

/// A room's outline, counter-clockwise.
pub fn outline(model: &Model, room: ElementId) -> Option<Vec<DVec2>> {
    shape(model, room).map(|s| s.outline)
}

/// A room's floor area, 0 when it is not enclosed: inside its outline,
/// less the shafts (a stair well, a lift) that open its floor to the
/// storey below.
pub fn area(model: &Model, room: ElementId) -> f64 {
    match model.get(room).map(|e| &e.kind) {
        Some(ElementKind::Room(r)) => shape_of(model, r).map_or(0.0, |s| net_area(model, r, &s)),
        _ => 0.0,
    }
}

/// How thick the finishes (plaster, render) are on the face of `wall`
/// that edge `a`–`b` of a room runs along: the room's floor area is
/// counted to the bare wall behind them (George, as STN 73 4301 drawings
/// state areas without plaster).
pub fn finish_on_face(model: &Model, wall: ElementId, a: DVec2, b: DVec2) -> f64 {
    let Some(ElementKind::Wall(w)) = model.get(wall).map(|e| &e.kind) else {
        return 0.0;
    };
    let Some(ElementKind::Assembly(asm)) = w.assembly.and_then(|id| model.get(id)).map(|e| &e.kind)
    else {
        return 0.0;
    };
    let mut bands = crate::assembly::wall_bands(asm, w);
    if bands.is_empty() {
        return 0.0;
    }
    let n = w.direction().perp();
    let side = n.dot((a + b) / 2.0 - w.start);
    // from the face the room is on, inwards
    bands.sort_by(|x, y| x.s0.min(x.s1).total_cmp(&y.s0.min(y.s1)));
    if side > 0.0 {
        bands.reverse();
    }
    bands
        .iter()
        .take_while(|b| b.function == crate::assembly::LayerFunction::Finish)
        .map(|b| (b.s1 - b.s0).abs())
        .sum()
}

/// A room's shape with its edges along walls pushed out past their
/// plaster to the bare wall: what its floor area is counted over.
pub fn bare_shape(model: &Model, s: &RoomShape) -> RoomShape {
    let n = s.outline.len();
    let by: Vec<f64> = (0..n)
        .map(|i| match s.bounded_by.get(i).copied().flatten() {
            Some(w) => finish_on_face(model, w, s.outline[i], s.outline[(i + 1) % n]),
            None => 0.0,
        })
        .collect();
    RoomShape {
        outline: crate::members::offset_edges(&s.outline, &by),
        bounded_by: s.bounded_by.clone(),
    }
}

/// A room's floor area without plaster (see [`bare_shape`]), less the
/// shafts through its floor: what the room legend and the building's
/// figures count.
pub fn bare_area(model: &Model, r: &Room, s: &RoomShape) -> f64 {
    net_area(model, r, &bare_shape(model, s))
}

/// The floor area of `r`, shaped `s`, less the shafts through its floor.
pub fn net_area(model: &Model, r: &Room, s: &RoomShape) -> f64 {
    let floor = model.level(r.level).map_or(0.0, |l| l.elevation);
    let voids: f64 = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Shaft(sh) => {
                let (z0, z1) = sh.span(model);
                let n = sh.boundary.len().max(1) as f64;
                let middle = sh.boundary.iter().copied().sum::<DVec2>() / n;
                (z0 < floor - 1e-6
                    && z1 >= floor - 1e-6
                    && crate::polygon::contains(&s.outline, middle))
                .then(|| crate::polygon::signed_area2(&sh.boundary).abs() / 2.0)
            }
            _ => None,
        })
        .sum();
    (s.area() - voids).max(0.0)
}

/// How tall a room is: its own height, else up to the underside of the
/// lowest ceiling over its point, else its level's floor-to-floor height.
pub fn height(model: &Model, r: &Room) -> f64 {
    if let Some(h) = r.height {
        return h;
    }
    let Some(l) = model.level(r.level) else {
        return 3.0;
    };
    ceiling_over(model, r.level, r.at, l.elevation + l.height + 1e-6)
        .map_or(l.height, |(_, z)| z - l.elevation)
}

/// The lowest ceiling over `p` among those on `level` whose underside is
/// below `below`: its id and the height of its underside.
pub fn ceiling_over(
    model: &Model,
    level: ElementId,
    p: DVec2,
    below: f64,
) -> Option<(ElementId, f64)> {
    let mut best: Option<(ElementId, f64)> = None;
    for e in model.iter() {
        let ElementKind::Ceiling(c) = &e.kind else {
            continue;
        };
        if c.level != level {
            continue;
        }
        let (z, _) = c.span(model);
        if z >= below || best.is_some_and(|b| b.1 <= z) {
            continue;
        }
        if c.outline(model)
            .is_some_and(|o| crate::polygon::contains(&o, p))
        {
            best = Some((e.id, z));
        }
    }
    best
}

/// Whether room `r` named `name` is a balcony, loggia or terrace (or
/// marked outdoor): STN 73 4301 2.14 leaves these out of the floor
/// (usable) area of a dwelling.
pub fn is_open_air(name: &str, r: &Room) -> bool {
    const WORDS: [&str; 10] = [
        "balk", "balc", "lodži", "lodzi", "loggia", "lodžie", "teras", "terrace", "outdoor",
        "vonkaj",
    ];
    [r.occupancy.as_str(), r.department.as_str(), name]
        .iter()
        .map(|t| t.to_lowercase())
        .any(|t| WORDS.iter().any(|w| t.contains(w)))
}

/// Whether room `r` named `name`, of `area` m², counts in the living area (obytná
/// plocha), by its own word, else by its type: a living room, bedroom,
/// children's room, dining room or study of at least 8 m², a kitchen of
/// a living room with a kitchen in it. STN 73 4301: obytné miestnosti
/// are the living room, dining room, bedroom, study and other rooms
/// (guest, children's), each of at least 8 m² (2005 7.2.1 a, 7.2.7).
/// By its amendment Z1:2026 a kitchen is príslušenstvo bytu (8.4.1 d),
/// a separate room with or without dining (8.4.6 a, b); a cooking space
/// in a living room leaves that room's living area as it is (3.12), so
/// a living room with a kitchen counts whole and a kitchen on its own
/// does not.
pub fn counts_as_living(name: &str, r: &Room, area: f64) -> bool {
    use crate::roomtype::RoomType as T;
    if let Some(own) = r.living {
        return own;
    }
    match T::of(&r.occupancy, name) {
        T::Living | T::Bedroom | T::Children | T::Dining | T::Office => area >= 8.0 - 1e-9,
        T::Kitchen => {
            // "Obývacia izba s kuchyňou", "Living / kitchen"
            let t = format!("{} {}", r.occupancy, name).to_lowercase();
            ["obýv", "obyv", "living", "pobyt"]
                .iter()
                .any(|w| t.contains(w))
                && area >= 8.0 - 1e-9
        }
        _ => false,
    }
}

/// The number for a new room on `level`: one more than the highest number
/// there ("1.03" after "1.02"), else the first of its storey by its
/// designation: "1.01" on 1. NP, "2.01" on 2. NP, "-1.01" on 1. PP.
pub fn next_number(model: &Model, level: ElementId) -> String {
    let mut best: Option<(String, u32, usize)> = None;
    for e in model.iter() {
        let ElementKind::Room(r) = &e.kind else {
            continue;
        };
        if r.level != level {
            continue;
        }
        let digits = r
            .number
            .chars()
            .rev()
            .take_while(char::is_ascii_digit)
            .count();
        if digits == 0 {
            continue;
        }
        let (head, tail) = r.number.split_at(r.number.len() - digits);
        let Ok(n) = tail.parse::<u32>() else { continue };
        if best.as_ref().is_none_or(|b| n > b.1) {
            best = Some((head.to_string(), n, digits));
        }
    }
    if let Some((head, n, width)) = best {
        return format!("{head}{:0width$}", n + 1);
    }
    format!("{}.01", storey_number(model, level))
}

/// The number a storey gives its rooms: that of its designation (1 for
/// "1. NP", 2 for "2. NP", -1 for "1. PP"). Read from the level's name where it
/// names one ("2. NP", "1.PP", "Storey 2", "Basement 1"), else counted
/// from the ground as storeys are named.
pub fn storey_number(model: &Model, level: ElementId) -> i64 {
    if let Some(n) = model.get(level).and_then(|e| storey_designation(&e.name)) {
        return n;
    }
    let Some(here) = model.level(level) else {
        return 1;
    };
    // storeys above ground count from the ground floor up; those under
    // it (a basement) from the ground down, the footings' level between
    // left out
    let below = here.elevation < -1e-6;
    let between = model
        .levels()
        .filter(|(e, l)| {
            e.id != level
                && if below {
                    l.elevation < crate::levels::UNDERGROUND && l.elevation > here.elevation + 1e-6
                } else {
                    l.elevation >= -1e-6 && l.elevation < here.elevation - 1e-6
                }
        })
        .count() as i64;
    if below {
        -(between + 1)
    } else {
        between + 1
    }
}

/// A storey's designation in its name: "1. NP" 1, "2.NP" 2, "1. PP" -1,
/// "Storey 2" 2, "Basement 1" -1 (also Czech "1. PP", "2. NP").
fn storey_designation(name: &str) -> Option<i64> {
    let t = name.trim().to_lowercase();
    let digits = |s: &str| -> Option<i64> {
        let d: String = s
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '.')
            .collect();
        (!d.is_empty() && d.chars().all(|c| c.is_ascii_digit()))
            .then(|| d.parse().ok())
            .flatten()
    };
    for (suffix, sign) in [("np", 1), ("pp", -1)] {
        if let Some(head) = t.strip_suffix(suffix) {
            return digits(head).filter(|n| *n > 0).map(|n| n * sign);
        }
    }
    for (prefix, sign) in [("storey", 1), ("basement", -1)] {
        if let Some(tail) = t.strip_prefix(prefix) {
            return digits(tail).filter(|n| *n > 0).map(|n| n * sign);
        }
    }
    None
}

/// Whether a note reads like a room label: a name, then its area on the
/// next line ("LIVING\n37.2 m²").
fn room_label(text: &str) -> Option<&str> {
    let mut lines = text.lines().map(str::trim);
    let name = lines.next().filter(|l| !l.is_empty())?;
    let area = lines.next()?;
    let number = area
        .strip_suffix("m²")
        .or_else(|| area.strip_suffix("m2"))?;
    number.trim().replace(',', ".").parse::<f64>().ok()?;
    lines.next().is_none().then_some(name)
}

/// "PRIMARY BEDROOM" as a name: "Primary Bedroom".
fn title_case(name: &str) -> String {
    if name.chars().any(char::is_lowercase) {
        return name.to_string();
    }
    let mut out = String::with_capacity(name.len());
    let mut start = true;
    for c in name.chars() {
        if start {
            out.extend(c.to_uppercase());
        } else {
            out.extend(c.to_lowercase());
        }
        start = !c.is_alphanumeric();
    }
    out
}

/// Turns the room labels written as notes in plans (a name over an area,
/// the way a project made before rooms existed wrote them) into rooms with
/// tags in their place, wherever the walls close the label off. Labels the
/// walls don't close off are left as notes and named in the second list.
pub fn from_notes(model: &Model) -> (crate::Transaction, Vec<String>) {
    let mut tx = crate::Transaction::new("Rooms from room notes");
    let mut skipped = Vec::new();
    let mut numbers: std::collections::HashMap<ElementId, String> = Default::default();
    let mut placed: Vec<(ElementId, Vec<DVec2>)> = Vec::new();
    for e in model.iter() {
        let ElementKind::Note(n) = &e.kind else {
            continue;
        };
        let Some(name) = room_label(&n.text) else {
            continue;
        };
        let Some(ElementKind::View(v)) = model.get(n.view).map(|v| &v.kind) else {
            continue;
        };
        let crate::ViewKind::Plan { level, .. } = v.kind else {
            continue;
        };
        // where the label's lines are centred: across its widest line, half
        // a line below its first
        let look = crate::text_style(model, n.style, n.color);
        let mm = v.scale / 1000.0;
        let widest = n.text.lines().map(|l| l.chars().count()).max().unwrap_or(0) as f64;
        let middle = n.at + DVec2::new(widest * 0.6 * look.size * mm / 2.0, -0.75 * look.size * mm);
        let Some((at, shape)) = [middle, n.at]
            .into_iter()
            .find_map(|p| enclosed(model, level, p).map(|s| (p, s)))
        else {
            skipped.push(name.to_string());
            continue;
        };
        // a room already there (or made from another label) keeps it
        let taken = model.iter().any(|x| match &x.kind {
            ElementKind::Room(r) => {
                r.level == level
                    && outline(model, x.id).is_some_and(|o| crate::polygon::contains(&o, at))
            }
            _ => false,
        }) || placed
            .iter()
            .any(|(l, o)| *l == level && crate::polygon::contains(o, at));
        if taken {
            skipped.push(name.to_string());
            continue;
        }
        let number = match numbers.get(&level) {
            Some(last) => bump(last),
            None => next_number(model, level),
        };
        numbers.insert(level, number.clone());
        let mut r = Room::new(level, at);
        r.number = number;
        let room = Element::new(title_case(name), ElementKind::Room(r));
        let mut tag = crate::Tag::new(n.view, room.id, middle - at);
        tag.style = n.style;
        tag.color = n.color;
        placed.push((level, shape.outline));
        tx = tx
            .insert(room.clone())
            .insert(Element::new("Room tag", ElementKind::Tag(tag)))
            .remove_cascade(model, e.id);
    }
    (tx, skipped)
}

/// The number after `number`: its trailing digits counted up.
fn bump(number: &str) -> String {
    let digits = number
        .chars()
        .rev()
        .take_while(char::is_ascii_digit)
        .count();
    let (head, tail) = number.split_at(number.len() - digits);
    match tail.parse::<u32>() {
        Ok(n) => format!("{head}{:0digits$}", n + 1),
        Err(_) => format!("{number}.1"),
    }
}

/// Rooms on `level` whose point another room's outline already holds:
/// two rooms placed in the same space.
pub fn overlapping(model: &Model) -> Vec<(ElementId, ElementId)> {
    let rooms: Vec<(ElementId, &Room, Option<RoomShape>)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Room(r) => Some((e.id, r, shape_of(model, r))),
            _ => None,
        })
        .collect();
    let mut out = vec![];
    for (i, a) in rooms.iter().enumerate() {
        for b in &rooms[i + 1..] {
            if a.1.level != b.1.level {
                continue;
            }
            let holds = |s: &Option<RoomShape>, p: DVec2| {
                s.as_ref()
                    .is_some_and(|s| crate::polygon::contains(&s.outline, p))
            };
            if holds(&a.2, b.1.at) || holds(&b.2, a.1.at) {
                out.push((a.0, b.0));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Document, Element, Level, Transaction, Wall};

    fn house() -> (Document, ElementId, Vec<ElementId>) {
        let lv = Element::new(
            "Ground",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let l = lv.id;
        let mut tx = Transaction::new("walls").insert(lv);
        let mut ids = vec![];
        for (a, b) in [
            ((0.0, 0.0), (6.0, 0.0)),
            ((6.0, 0.0), (6.0, 4.0)),
            ((6.0, 4.0), (0.0, 4.0)),
            ((0.0, 4.0), (0.0, 0.0)),
        ] {
            let w = Element::new(
                "Wall",
                ElementKind::Wall(Wall::new(
                    l,
                    DVec2::new(a.0, a.1),
                    DVec2::new(b.0, b.1),
                    0.2,
                    3.0,
                )),
            );
            ids.push(w.id);
            tx = tx.insert(w);
        }
        let mut doc = Document::new();
        doc.commit(tx).unwrap();
        (doc, l, ids)
    }

    /// Tiles never run over a window: the panels leave the window out in
    /// 3D, the plan line breaks at it and the area loses it.
    #[test]
    fn tiling_leaves_the_windows_out() {
        let (mut doc, l, walls) = house();
        let room = Element::new(
            "Bath",
            ElementKind::Room(Room::new(l, DVec2::new(2.0, 2.0))),
        );
        let rid = room.id;
        // a window 1.2 x 1.3 m on a 0.9 m sill in the first wall
        let window = Element::new(
            "Window",
            ElementKind::Opening(crate::Opening {
                host: walls[0],
                kind: crate::OpeningKind::Window,
                offset: 2.0,
                sill: 0.9,
                width: 1.2,
                height: 1.3,
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
        doc.commit(Transaction::new("room").insert(room).insert(window))
            .unwrap();
        let s = shape(doc.model(), rid).expect("enclosed");
        let m = doc.model();
        let panels = tiling_panels(m, &s, &[], 2.0);
        // no panel overlaps the window's box (x 2.0..3.2 along y = 0.1)
        for p in &panels {
            let (x0, x1) = (p.a.x.min(p.b.x), p.a.x.max(p.b.x));
            let on_wall = p.a.y.abs() < 0.2 && p.b.y.abs() < 0.2;
            let across = x0 < 3.2 - 1e-6 && x1 > 2.0 + 1e-6;
            let up = p.z0 < 2.0 - 1e-6 && p.z1 > 0.9 + 1e-6;
            assert!(!(on_wall && across && up), "{p:?} runs over the window");
        }
        // under the window the tiles stay, up to its sill
        assert!(panels.iter().any(|p| p.a.y.abs() < 0.2
            && (p.z1 - 0.9).abs() < 1e-9
            && p.a.x.min(p.b.x) <= 2.0 + 1e-6
            && p.a.x.max(p.b.x) >= 3.2 - 1e-6));
        // the area: the perimeter by 2 m less the window's 1.2 x 1.1 m
        let mut r = match &m.get(rid).unwrap().kind {
            ElementKind::Room(r) => r.clone(),
            _ => unreachable!(),
        };
        r.tiling = Some(Tiling::default());
        let full = s.perimeter() * 2.0;
        // the tiles run past the window: both its side reveals tiled from
        // the sill to the tiles' top, no head (the tiles stop under it),
        // no bottom (it has its inner sill)
        let reveals = tiling_reveals(m, &s, &Tiling::default());
        assert_eq!(reveals.len(), 2, "{}", reveals.len());
        let depth = reveals[0].area() / 1.1;
        assert!(depth > 0.01 && depth < 0.3, "{depth}");
        assert!(reveals.iter().all(|v| v.normal.z.abs() < 1e-9));
        let side = 2.0 * depth * 1.1;
        assert!((tiling_area(m, &r, &s) - (full - 1.2 * 1.1 + side)).abs() < 1e-6);
        // tiles over the window: the head is tiled too, as wide as the
        // window and as deep as the sides
        let high = Tiling {
            height: 2.5,
            ..Tiling::default()
        };
        let reveals = tiling_reveals(m, &s, &high);
        assert_eq!(reveals.len(), 3);
        let head = reveals.iter().find(|v| v.normal.z < -0.5).expect("head");
        assert!((head.area() - 1.2 * depth).abs() < 1e-6, "{}", head.area());
        // tiles only to 0.8 m: under the sill, no reveal at all
        let low = Tiling {
            height: 0.8,
            ..Tiling::default()
        };
        assert!(tiling_reveals(m, &s, &low).is_empty());
        // the plan line breaks at the window and turns into its reveals
        let runs: f64 = tiling_plan_runs(m, &s, &[], 2.0)
            .iter()
            .map(|(a, b)| a.distance(*b))
            .sum();
        assert!(
            (runs - (s.perimeter() - 1.2 + 2.0 * depth)).abs() < 1e-6,
            "{runs}"
        );
        // tiles only up to 0.8 m: the window is above them, nothing breaks
        let low: f64 = tiling_plan_runs(m, &s, &[], 0.8)
            .iter()
            .map(|(a, b)| a.distance(*b))
            .sum();
        assert!((low - s.perimeter()).abs() < 1e-6);
        // a splashback (600)(900) round one corner: 1 m on either side
        let c = s.outline[1];
        let (u, v) = (
            (s.outline[0] - c).normalize(),
            (s.outline[2] - c).normalize(),
        );
        let part = Tiling {
            height: 0.6,
            bottom: 0.9,
            span: Some([c + u * 1.0, c + v * 1.0]),
            ..Tiling::default()
        };
        assert_eq!(part.mark_text(), "(600)(900)");
        let runs = tiling_plan_runs_of(m, &s, &part);
        let len: f64 = runs.iter().map(|(a, b)| a.distance(*b)).sum();
        let panels = tiling_panels_of(m, &s, &part);
        assert!(panels
            .iter()
            .all(|p| (p.z0 - 0.9).abs() < 1e-9 || p.z0 > 0.9));
        assert!(panels.iter().all(|p| p.z1 <= 1.5 + 1e-9));
        assert!(len <= 2.0 + 1e-6 && len > 0.5, "{len}");
        // its mark on its line, where it was dragged to
        let (at, _) = tiling_mark_at(m, &s, &part).unwrap();
        assert!(runs
            .iter()
            .any(|(a, b)| { (at - *a).perp_dot(*b - *a).abs() < 1e-6 }));
        let mut moved = part.clone();
        moved.mark = Some(c + v * 0.3 + DVec2::new(0.4, 0.4));
        let (to, _) = tiling_mark_at(m, &s, &moved).unwrap();
        assert!(runs
            .iter()
            .any(|(a, b)| (to - *a).perp_dot(*b - *a).abs() < 1e-6));
        // the room's area counts both
        r.tiling_parts.push(part.clone());
        let more = tiling_area(m, &r, &s) - (full - 1.2 * 1.1 + side);
        let own: f64 = panels
            .iter()
            .map(|p| p.a.distance(p.b) * (p.z1 - p.z0))
            .sum();
        assert!((more - own).abs() < 1e-9 && own > 0.0);
        // without an inner sill the bottom reveal is tiled as well
        let before = doc
            .model()
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Opening(_)))
            .unwrap()
            .clone();
        let mut after = before.clone();
        if let ElementKind::Opening(o) = &mut after.kind {
            o.no_inner_sill = true;
        }
        doc.commit(Transaction::new("no sill").replace(before, after))
            .unwrap();
        let reveals = tiling_reveals(doc.model(), &s, &Tiling::default());
        assert_eq!(reveals.len(), 3);
        let bottom = reveals.iter().find(|v| v.normal.z > 0.5).expect("bottom");
        assert!((bottom.area() - 1.2 * depth).abs() < 1e-6);
    }

    #[test]
    fn a_circle_of_two_half_walls_closes_a_round_room() {
        let lv = Element::new(
            "Ground",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let l = lv.id;
        let mut tx = Transaction::new("walls").insert(lv);
        let (a, b) = (DVec2::new(-3.0, 0.0), DVec2::new(3.0, 0.0));
        for (s, e) in [(a, b), (b, a)] {
            let mut w = Wall::new(l, s, e, 0.2, 3.0);
            w.bulge = 1.0;
            tx = tx.insert(Element::new("Wall", ElementKind::Wall(w)));
        }
        let mut doc = Document::new();
        doc.commit(tx).unwrap();
        let s = enclosed(doc.model(), l, DVec2::new(0.5, 0.5)).expect("closed");
        // and a slab filling it goes to the centre line, radius 3
        let fill = crate::slab::room_outline(doc.model(), l, DVec2::new(0.5, 0.5)).expect("fills");
        let c = std::f64::consts::PI * 9.0;
        let got = crate::polygon::signed_area2(&fill).abs() / 2.0;
        assert!((got - c).abs() / c < 0.01, "{got} vs {c}");
        // to the inner face, radius 2.9, as finely as the arcs bend
        let disc = std::f64::consts::PI * 2.9 * 2.9;
        assert!(
            (s.area() - disc).abs() / disc < 0.01,
            "{} vs {disc}",
            s.area()
        );
        assert!(s.bounded_by.iter().all(|b| b.is_some()));
    }

    #[test]
    fn a_room_fills_to_the_inner_faces_and_follows_its_walls() {
        let (mut doc, l, walls) = house();
        let room = Element::new(
            "Living",
            ElementKind::Room(Room::new(l, DVec2::new(2.0, 2.0))),
        );
        let rid = room.id;
        doc.commit(Transaction::new("room").insert(room)).unwrap();
        let s = shape(doc.model(), rid).expect("enclosed");
        // inner faces: 5.8 × 3.8
        assert!((s.area() - 5.8 * 3.8).abs() < 1e-9, "{}", s.area());
        assert!((s.perimeter() - 2.0 * (5.8 + 3.8)).abs() < 1e-9);
        // every edge runs along a wall
        assert!(s
            .bounded_by
            .iter()
            .all(|b| b.is_some_and(|b| walls.contains(&b))));
        // move the east wall out a metre (and stretch the two it meets)
        for (i, id) in walls.iter().enumerate() {
            doc.edit(*id, "Move east wall", |e| {
                if let ElementKind::Wall(w) = &mut e.kind {
                    for p in [&mut w.start, &mut w.end] {
                        if p.x > 5.0 {
                            p.x += 1.0;
                        }
                    }
                }
            })
            .unwrap_or_else(|_| panic!("wall {i}"));
        }
        assert!((area(doc.model(), rid) - 6.8 * 3.8).abs() < 1e-9);
        // a separator splits it
        let sep = Element::new(
            "Separator",
            ElementKind::RoomSeparator(Separator {
                bounds_slabs: false,
                level: l,
                a: DVec2::new(3.0, 0.1),
                b: DVec2::new(3.0, 3.9),
                look: None,
            }),
        );
        doc.commit(Transaction::new("sep").insert(sep)).unwrap();
        let s = shape(doc.model(), rid).unwrap();
        assert!((s.area() - 2.9 * 3.8).abs() < 1e-9, "{}", s.area());
        // a room outside the walls isn't enclosed
        let out = Room::new(l, DVec2::new(-3.0, 1.0));
        assert!(shape_of(doc.model(), &out).is_none());
    }

    #[test]
    fn a_separator_drawn_a_little_short_of_the_walls_still_splits_a_room() {
        let (mut doc, l, _) = house();
        let room = Element::new(
            "Room",
            ElementKind::Room(Room::new(l, DVec2::new(1.0, 2.0))),
        );
        let rid = room.id;
        doc.commit(Transaction::new("room").insert(room)).unwrap();
        let whole = area(doc.model(), rid);
        // a snap 50 mm short of each face
        let sep = Element::new(
            "Separator",
            ElementKind::RoomSeparator(Separator {
                bounds_slabs: false,
                level: l,
                a: DVec2::new(3.0, 0.15),
                b: DVec2::new(3.02, 3.85),
                look: None,
            }),
        );
        doc.commit(Transaction::new("sep").insert(sep)).unwrap();
        let split = area(doc.model(), rid);
        assert!(split < whole * 0.6, "{split} of {whole}");
    }

    #[test]
    fn a_room_is_as_tall_as_the_ceiling_over_it() {
        let (mut doc, l, _) = house();
        let room = Element::new(
            "Room",
            ElementKind::Room(Room::new(l, DVec2::new(2.0, 2.0))),
        );
        let rid = room.id;
        let r = match &room.kind {
            ElementKind::Room(r) => r.clone(),
            _ => unreachable!(),
        };
        doc.commit(Transaction::new("room").insert(room)).unwrap();
        assert_eq!(height(doc.model(), &r), 3.0);
        let c = Element::new(
            "Ceiling",
            ElementKind::Ceiling(Ceiling::in_room(l, rid, 2.4)),
        );
        doc.commit(Transaction::new("ceiling").insert(c)).unwrap();
        assert!((height(doc.model(), &r) - 2.4).abs() < 1e-12);
        let o = match &doc
            .model()
            .iter()
            .find(|e| e.name == "Ceiling")
            .unwrap()
            .kind
        {
            ElementKind::Ceiling(c) => c.outline(doc.model()).unwrap(),
            _ => unreachable!(),
        };
        assert!((crate::polygon::signed_area2(&o) / 2.0 - 5.8 * 3.8).abs() < 1e-9);
    }

    /// Rooms are numbered by their storey's designation: 1. NP 1.01,
    /// 2. NP 2.01, 1. PP -1.01.
    #[test]
    fn rooms_are_numbered_by_the_storey() {
        assert_eq!(storey_designation("1. NP"), Some(1));
        assert_eq!(storey_designation("2.NP"), Some(2));
        assert_eq!(storey_designation("1. PP"), Some(-1));
        assert_eq!(storey_designation("Basement 2"), Some(-2));
        assert_eq!(storey_designation("Prízemie"), None);
        let mut m = Model::new();
        let mk = |name: &str, z: f64| {
            Element::new(
                name,
                ElementKind::Level(Level {
                    elevation: z,
                    height: 3.0,
                }),
            )
        };
        // unnamed storeys counted from the ground
        let pp = mk("Suterén", -3.0);
        let np1 = mk("Prízemie", 0.0);
        let np2 = mk("Poschodie", 3.0);
        let ids = [pp.id, np1.id, np2.id];
        Transaction::new("t")
            .insert(pp)
            .insert(np1)
            .insert(np2)
            .apply(&mut m)
            .unwrap();
        let n: Vec<String> = ids.iter().map(|l| next_number(&m, *l)).collect();
        assert_eq!(n, ["-1.01", "1.01", "2.01"]);
        // a name with a designation wins
        let named = mk("2. NP", 0.0);
        let id = named.id;
        Transaction::new("t").insert(named).apply(&mut m).unwrap();
        assert_eq!(next_number(&m, id), "2.01");
    }

    #[test]
    fn room_numbers_count_on() {
        let (mut doc, l, _) = house();
        assert_eq!(next_number(doc.model(), l), "1.01");
        let mut r = Room::new(l, DVec2::new(2.0, 2.0));
        r.number = "0.09".into();
        doc.commit(Transaction::new("room").insert(Element::new("A", ElementKind::Room(r))))
            .unwrap();
        assert_eq!(next_number(doc.model(), l), "0.10");
    }

    #[test]
    fn room_notes_become_rooms_with_tags() {
        let m = crate::starter::starter_house();
        let rooms: Vec<ElementId> = m
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Room(_)))
            .map(|e| e.id)
            .collect();
        let mut doc = crate::Document::from_model(m.clone());
        doc.commit(crate::Transaction::new("No rooms").remove_all_cascade(&m, &rooms))
            .unwrap();
        let plan = doc
            .model()
            .iter()
            .find(|e| e.name == "Ground Floor plan")
            .unwrap()
            .id;
        let note = |at: DVec2, text: &str| {
            Element::new("Note", ElementKind::Note(crate::Note::new(plan, at, text)))
        };
        doc.commit(
            crate::Transaction::new("Labels")
                .insert(note(DVec2::new(1.5, 3.5), "STUDY\n19.9 m²"))
                .insert(note(DVec2::new(1.2, 3.0), "SECOND STUDY\n19.9 m²"))
                .insert(note(DVec2::new(9.7, 1.3), "STORE\n4.0 m²"))
                .insert(note(DVec2::new(-20.0, -20.0), "GARDEN\n100 m²"))
                .insert(note(DVec2::new(5.0, 1.7), "just a note")),
        )
        .unwrap();
        let (tx, skipped) = from_notes(doc.model());
        doc.commit(tx).unwrap();
        let m = doc.model();
        let made: Vec<(&str, &Room)> = m
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Room(r) => Some((e.name.as_str(), r)),
                _ => None,
            })
            .collect();
        // the study once; a second label in it, and one outside, stay notes
        assert_eq!(made.len(), 2, "{made:?}");
        assert_eq!(made[0].0, "Study");
        assert_eq!(
            (made[0].1.number.as_str(), made[1].1.number.as_str()),
            ("1.01", "1.02")
        );
        assert_eq!(
            skipped,
            vec!["SECOND STUDY".to_string(), "GARDEN".to_string()]
        );
        let tags = m
            .iter()
            .filter(|e| matches!(&e.kind, ElementKind::Tag(t) if model_room(m, t.host)))
            .count();
        assert_eq!(tags, 2);
        assert!(m
            .iter()
            .any(|e| matches!(&e.kind, ElementKind::Note(n) if n.text == "just a note")));
        assert!(!m
            .iter()
            .any(|e| matches!(&e.kind, ElementKind::Note(n) if n.text.starts_with("STUDY"))));
    }

    fn model_room(m: &Model, id: ElementId) -> bool {
        matches!(m.get(id).map(|e| &e.kind), Some(ElementKind::Room(_)))
    }

    #[test]
    fn tags_from_before_room_fields_still_read() {
        let id = serde_json::to_string(&ElementId::new()).unwrap();
        let t: crate::Tag = serde_json::from_str(&format!(
            r#"{{"view":{id},"host":{id},"offset":[0.0,0.0]}}"#
        ))
        .unwrap();
        assert!(t.fields.is_empty());
        assert!(!serde_json::to_string(&t).unwrap().contains("fields"));
    }

    /// A ceiling drawn as a slab of a podhľad build-up, before ceilings had
    /// their own (George 2026-11-23), opens as a ceiling of that build-up
    /// with its underside where the slab's was.
    #[test]
    fn ceilings_drawn_as_slabs_open_as_ceilings() {
        use crate::{Assembly, AssemblyUse, Document, Layer, LayerFunction, Level, Slab};
        let mut doc = Document::new();
        let lv = Element::new(
            "1. NP",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let l = lv.id;
        let asm = Element::new(
            "Podhľad pod väzníkovou strechou",
            ElementKind::Assembly(Assembly::new(
                AssemblyUse::Floor,
                vec![Layer::new(LayerFunction::Insulation, None, 0.27)],
            )),
        );
        let a = asm.id;
        let mut s = Slab::new(
            l,
            crate::polygon::rectangle(DVec2::ZERO, DVec2::new(4.0, 3.0)),
            0.285,
        );
        s.assembly = Some(a);
        s.top_offset = 2.785;
        let slab = Element::new("Slab", ElementKind::Slab(s));
        let sid = slab.id;
        doc.commit(Transaction::new("t").insert(lv).insert(asm).insert(slab))
            .unwrap();
        let tx = mend_ceilings(doc.model()).expect("something to mend");
        doc.commit(tx).unwrap();
        let m = doc.model();
        let ElementKind::Ceiling(c) = &m.get(sid).unwrap().kind else {
            panic!("a ceiling now");
        };
        assert_eq!(c.assembly, Some(a));
        assert!((c.height - 2.5).abs() < 1e-9 && (c.thickness - 0.285).abs() < 1e-9);
        assert!(
            matches!(&m.get(a).unwrap().kind, ElementKind::Assembly(x) if x.usage == AssemblyUse::Ceiling)
        );
        assert!(mend_ceilings(m).is_none());
    }

    #[test]
    fn rooms_and_ceilings_read_with_only_what_they_need() {
        let level = serde_json::to_string(&ElementId::new()).unwrap();
        let r: Room =
            serde_json::from_str(&format!(r#"{{"level":{level},"at":[1.0,2.0]}}"#)).unwrap();
        assert!(r.number.is_empty() && r.height.is_none() && !r.is_drawn());
        let text = serde_json::to_string(&r).unwrap();
        assert!(!text.contains("finish") && !text.contains("boundary"));
        let c: Ceiling = serde_json::from_str(&format!(
            r#"{{"level":{level},"boundary":[[0,0],[1,0],[1,1]],"height":2.4,"thickness":0.02}}"#
        ))
        .unwrap();
        assert!(c.room.is_none() && c.material.is_none());
        c.validate().unwrap();
    }

    #[test]
    fn an_open_room_shows_where_its_walls_stop_short() {
        let (mut doc, l, ids) = house();
        let p = DVec2::new(3.0, 2.0);
        assert!(enclosed(doc.model(), l, p).is_some());
        // the north wall stops 1 m short of the west wall
        let before = doc.model().get(ids[2]).unwrap().clone();
        let mut north = before.clone();
        if let ElementKind::Wall(w) = &mut north.kind {
            w.end = DVec2::new(1.0, 4.0);
        }
        doc.commit(Transaction::new("shorten").replace(before, north))
            .unwrap();
        assert!(enclosed(doc.model(), l, p).is_none());
        let g = gaps(doc.model(), l, p);
        assert_eq!(g.len(), 1, "{g:?}");
        let (a, b) = g[0];
        // across the gap, from the north wall's end to the west wall's end
        let xs = (a.x.min(b.x), a.x.max(b.x));
        assert!(
            (xs.0 - 0.1).abs() < 0.02 && (xs.1 - 1.0).abs() < 0.02,
            "{g:?}"
        );
        assert!(
            [a.y, b.y].iter().all(|y| (3.89..=4.01).contains(y)),
            "{g:?}"
        );
    }
}
