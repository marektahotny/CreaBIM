//! A heating drawing's underfloor heating, as the office draws it: the
//! loops' supply a full red line and their return a dashed blue one, in
//! every heated room its circuits' table (the manifold and circuit, the
//! supply temperature, the spacing, a loop's length, the pipe, the loss
//! and flow, the floor laid and the room's temperature and loss), and
//! the manifold's mark with the number of its circuits.

use crate::annotation::{Annotation, Shape, TextLook};
use crate::plan::PlanItem;
use creabim_core::{fixture, room, ElementId, ElementKind, Model};
use creabim_services::{floorheat, heatloss};
use glam::DVec2;

/// The supply's and the return's colour.
pub const SUPPLY: [f32; 3] = [0.84, 0.1, 0.1];
pub const RETURN: [f32; 3] = [0.1, 0.3, 0.82];
/// The edge of a heated field.
pub const FIELD: [f32; 3] = [0.98, 0.82, 0.05];
/// The return's dash and gap, paper mm.
pub const DASH: (f64, f64) = (4.0, 1.5);
/// A loop's line, paper mm.
const WIDTH: f64 = 0.25;
/// The tables' letters, paper mm.
const TEXT: f64 = 2.0;
/// The area of transit pipes' wash.
pub const TRANSIT: [f32; 3] = [1.0, 0.96, 0.72];
/// The sleeves' dots, and how far a sleeve runs past a doorway's side, m.
pub const SLEEVE: [f32; 3] = [0.15, 0.15, 0.15];
const SLEEVE_HALF: f64 = 0.1;
/// A movement joint between loops, and its dashes, paper mm.
pub const JOINT: [f32; 3] = [0.0, 0.0, 0.0];
pub const JOINT_DASH: (f64, f64) = (6.0, 1.0);
/// Half a sleeve where a connecting pipe crosses a joint, m (STN EN
/// 1264-4 4.2.2.9.4: about 0.3 m).
const JOINT_SLEEVE_HALF: f64 = 0.15;
/// The pipes of a bundle apart, m.
const PITCH: f64 = 0.05;

/// The way through `pts` to `end` square to the plan: from each point
/// it goes on as it came, then turns onto the next.
pub fn square(pts: &[DVec2], end: DVec2) -> Vec<DVec2> {
    let mut all = pts.to_vec();
    all.push(end);
    let mut out: Vec<DVec2> = vec![];
    for (i, q) in all.iter().enumerate() {
        if let (Some(p), true) = (out.last().copied(), i > 0) {
            let d = q - p;
            if d.x.abs() > 1e-6 && d.y.abs() > 1e-6 {
                let came = if out.len() > 1 {
                    p - out[out.len() - 2]
                } else {
                    d
                };
                let elbow = if came.x.abs() >= came.y.abs() {
                    DVec2::new(q.x, p.y)
                } else {
                    DVec2::new(p.x, q.y)
                };
                out.push(elbow);
            }
        }
        out.push(*q);
    }
    out
}

/// `pts` without repeated points, points on a straight run and points
/// where it turns back.
pub fn tidy(pts: &[DVec2]) -> Vec<DVec2> {
    let mut out: Vec<DVec2> = vec![];
    for p in pts {
        if out.last().is_some_and(|q| q.distance(*p) < 1e-4) {
            continue;
        }
        while out.len() >= 2 {
            let (a, b) = (out[out.len() - 2], out[out.len() - 1]);
            let (u, v) = ((b - a).normalize_or_zero(), (*p - b).normalize_or_zero());
            // straight on, or back the way it came
            if u.perp_dot(v).abs() < 1e-6 {
                out.pop();
            } else {
                break;
            }
        }
        out.push(*p);
    }
    out
}

/// The point of the polyline `pts` nearest `p`, and how far along it, m.
fn near_on(pts: &[DVec2], p: DVec2) -> (DVec2, f64) {
    let mut best = (pts.first().copied().unwrap_or(p), 0.0, f64::MAX);
    let mut run = 0.0;
    for w in pts.windows(2) {
        let d = w[1] - w[0];
        let l = d.length();
        let t = if l < 1e-12 {
            0.0
        } else {
            ((p - w[0]).dot(d) / (l * l)).clamp(0.0, 1.0)
        };
        let q = w[0] + d * t;
        if q.distance(p) < best.2 {
            best = (q, run + l * t, q.distance(p));
        }
        run += l;
    }
    (best.0, best.1)
}

/// The point `s` m along the polyline `pts`.
fn point_at(pts: &[DVec2], s: f64) -> DVec2 {
    let mut run = 0.0;
    for w in pts.windows(2) {
        let l = w[0].distance(w[1]);
        if run + l >= s {
            return w[0].lerp(w[1], ((s - run) / l.max(1e-12)).clamp(0.0, 1.0));
        }
        run += l;
    }
    pts.last().copied().unwrap_or_default()
}

/// The polyline `pts` up to `a` m along it and on from `b`.
fn cut_at(pts: &[DVec2], a: f64, b: f64) -> (Vec<DVec2>, Vec<DVec2>) {
    let (mut first, mut last) = (vec![], vec![]);
    let mut run = 0.0;
    for (i, w) in pts.windows(2).enumerate() {
        let l = w[0].distance(w[1]);
        if i == 0 && a > 0.0 {
            first.push(w[0]);
        }
        if run < a && run + l >= a {
            first.push(point_at(pts, a));
        } else if run + l < a {
            first.push(w[1]);
        }
        if run <= b && run + l > b {
            last.push(point_at(pts, b));
            last.push(w[1]);
        } else if run > b {
            last.push(w[1]);
        }
        run += l;
    }
    (first, last)
}

/// `pts` moved `d` to its left, each corner mitred.
pub fn offset(pts: &[DVec2], d: f64) -> Vec<DVec2> {
    let n = pts.len();
    if n < 2 {
        return pts.to_vec();
    }
    let dir = |i: usize| (pts[i + 1] - pts[i]).normalize_or_zero();
    (0..n)
        .map(|i| {
            let a = if i > 0 { dir(i - 1) } else { dir(0) };
            let b = if i + 1 < n { dir(i) } else { dir(n - 2) };
            let m = (a.perp() + b.perp()).normalize_or_zero();
            let c = m.dot(b.perp()).max(0.3);
            pts[i] + m * d / c
        })
        .collect()
}

/// The outline `o` with its corners rounded off by `r` (an arc of a few
/// chords at each), closed.
pub fn rounded(o: &[DVec2], r: f64) -> Vec<DVec2> {
    let n = o.len();
    let mut out = vec![];
    for i in 0..n {
        let (p, c, q) = (o[(i + n - 1) % n], o[i], o[(i + 1) % n]);
        let (u, v) = ((p - c).normalize_or_zero(), (q - c).normalize_or_zero());
        let k = r.min(c.distance(p) / 2.0).min(c.distance(q) / 2.0);
        let (a, b) = (c + u * k, c + v * k);
        // a quadratic curve from a to b about c
        for j in 0..=4 {
            let t = j as f64 / 4.0;
            out.push(a * (1.0 - t) * (1.0 - t) + c * 2.0 * t * (1.0 - t) + b * t * t);
        }
    }
    if let Some(f) = out.first().copied() {
        out.push(f);
    }
    out
}

/// A room's edge zones, places of reduced output and holes as the
/// office draws them (George 2026-11-27): an edge zone a dashed line with
/// rounded corners and a small triangle; a place of reduced output
/// stippled with a folded corner and what it gives less; a hole outlined
/// and crossed.
pub fn zone_items(
    items: &mut Vec<PlanItem>,
    id: ElementId,
    u: &creabim_core::underfloor::Underfloor,
    mm: f64,
) {
    use crate::plan::Pen;
    use creabim_core::underfloor::ZoneKind;
    let line = |items: &mut Vec<PlanItem>, pen: Pen, pts: &[DVec2]| {
        for w in pts.windows(2) {
            items.push(PlanItem::Line {
                id,
                pen,
                a: w[0],
                b: w[1],
            });
        }
    };
    for z in u.zones.iter().filter(|z| z.on && z.outline.len() > 2) {
        let o = &z.outline;
        let (lo, hi) = o.iter().fold(
            (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN)),
            |(lo, hi), p| (lo.min(*p), hi.max(*p)),
        );
        let mid = o.iter().copied().sum::<DVec2>() / o.len() as f64;
        match z.kind {
            ZoneKind::Edge { .. } if z.mark.is_some() => {
                // set off from an edge drawn along the field (George
                // 2026-11-28, as VYK.001): a dashed line at the zone's
                // width, an arrow on it pointing at the edge
                let [a, b, edge] = z.mark.unwrap_or_default();
                line(items, Pen::Hidden, &[a, b]);
                let m = (a + b) / 2.0;
                let dir = (edge - m).normalize_or_zero();
                if dir != DVec2::ZERO {
                    let along = dir.perp();
                    let (len, half) = (4.0 * mm, 1.2 * mm);
                    let tip = m + dir * len;
                    line(items, Pen::Symbol, &[m, tip]);
                    let head = [tip + dir * 1.5 * mm, tip + along * half, tip - along * half];
                    items.push(PlanItem::Shade {
                        id,
                        points: head.to_vec(),
                        rgb: [0.0, 0.0, 0.0],
                    });
                    line(items, Pen::Symbol, &[head[0], head[1], head[2], head[0]]);
                }
            }
            ZoneKind::Edge { .. } => {
                line(items, Pen::Hidden, &rounded(o, 0.12));
                // the triangle, pointing down
                let s = 2.5 * mm;
                let tri = [
                    mid + DVec2::new(-s, s * 0.6),
                    mid + DVec2::new(s, s * 0.6),
                    mid + DVec2::new(0.0, -s),
                    mid + DVec2::new(-s, s * 0.6),
                ];
                line(items, Pen::Symbol, &tri);
            }
            ZoneKind::Reduced { percent } => {
                items.push(PlanItem::Fill {
                    id,
                    points: o.clone(),
                    hatch: creabim_core::Hatch::Concrete,
                    rgb: None,
                });
                let mut ring = o.clone();
                ring.push(o[0]);
                line(items, Pen::Fine, &ring);
                // the folded corner at the top right
                let c = *o
                    .iter()
                    .max_by(|a, b| (a.x + a.y).total_cmp(&(b.x + b.y)))
                    .unwrap_or(&hi);
                let f = (0.25f64).min((hi - lo).min_element() / 3.0);
                line(
                    items,
                    Pen::Symbol,
                    &[
                        c - DVec2::new(f, 0.0),
                        c - DVec2::new(f, f),
                        c - DVec2::new(0.0, f),
                        c - DVec2::new(f, 0.0),
                    ],
                );
                items.push(PlanItem::Annotation {
                    id,
                    item: Box::new(Annotation {
                        shape: Shape::Along {
                            at: mid,
                            angle: 0.0,
                        },
                        text: format!("−{} %", num(percent, 0)),
                        look: TextLook {
                            size: TEXT,
                            ..Default::default()
                        },
                        mm,
                    }),
                });
            }
            ZoneKind::Hole => {
                let mut ring = o.clone();
                ring.push(o[0]);
                line(items, Pen::Projection, &ring);
                // each convex part crossed by itself, not the whole
                // bounding box (an L along the kitchen units)
                let _ = (lo, hi);
                for part in creabim_core::underfloor::convex_parts(o) {
                    let (a, b) = part.iter().fold(
                        (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN)),
                        |(lo, hi), p| (lo.min(*p), hi.max(*p)),
                    );
                    line(items, Pen::Fine, &[a, b]);
                    line(
                        items,
                        Pen::Fine,
                        &[DVec2::new(a.x, b.y), DVec2::new(b.x, a.y)],
                    );
                }
            }
        }
    }
}

/// The line through `pts` as thin strips in `rgb`, `dashes` (dash and
/// gap, paper mm) running on round its corners; `mm` drawing units make
/// a paper millimetre.
pub fn strips(
    items: &mut Vec<PlanItem>,
    id: ElementId,
    pts: &[DVec2],
    rgb: [f32; 3],
    dashes: Option<(f64, f64)>,
    mm: f64,
) {
    let half = WIDTH / 2.0 * mm;
    let mut strip = |a: DVec2, b: DVec2| {
        let n = (b - a).perp().normalize_or_zero() * half;
        items.push(PlanItem::Shade {
            id,
            points: vec![a - n, b - n, b + n, a + n],
            rgb,
        })
    };
    // how far into the dash-and-gap the line is
    let mut phase = 0.0;
    for w in pts.windows(2) {
        let len = w[0].distance(w[1]);
        if len < 1e-9 {
            continue;
        }
        let u = (w[1] - w[0]) / len;
        let Some((dash, gap)) = dashes.map(|(d, g)| (d * mm, g * mm)) else {
            strip(w[0], w[1]);
            continue;
        };
        let mut x = -phase;
        while x < len {
            let (s, e) = (x.max(0.0), (x + dash).min(len));
            if e - s > 1e-9 {
                strip(w[0] + u * s, w[0] + u * e);
            }
            x += dash + gap;
        }
        phase = (phase + len) % (dash + gap);
    }
}

fn num(x: f64, places: usize) -> String {
    format!("{x:.places$}")
}

/// The underfloor heating of the rooms of `level` in a plan where `mm`
/// drawing units make a paper millimetre; `view`'s tables of the rooms
/// (their tags) where they were moved to.
/// The model's underfloor heating design as the window last wrote it,
/// worked out once per revision (every heating plan of every storey and
/// sheet draws from it).
fn design_of(model: &Model) -> std::sync::Arc<floorheat::Design> {
    use std::sync::{Arc, Mutex};
    static CACHE: Mutex<Option<(u64, u64, Arc<floorheat::Design>)>> = Mutex::new(None);
    let key = (model as *const Model as u64, model.revision());
    if key.1 != 0 {
        if let Some((p, r, d)) = CACHE.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            if (*p, *r) == key {
                return d.clone();
            }
        }
    }
    let settings: floorheat::Settings =
        creabim_core::results::settings_of(model, "floorheat").unwrap_or_default();
    let d = Arc::new(floorheat::design(
        model,
        &heatloss::Opts::default(),
        &settings,
    ));
    if key.1 != 0 {
        *CACHE.lock().unwrap_or_else(|e| e.into_inner()) = Some((key.0, key.1, d.clone()));
    }
    d
}

pub fn items(model: &Model, level: ElementId, view: Option<ElementId>, mm: f64) -> Vec<PlanItem> {
    let mut rooms: Vec<_> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Room(r) if r.level == level => {
                Some((e, r, r.ufh.clone()?, room::shape_of(model, r)?))
            }
            _ => None,
        })
        .collect();
    if rooms.is_empty() {
        return vec![];
    }
    rooms.sort_by(|a, b| (&a.1.number, &a.0.name).cmp(&(&b.1.number, &b.0.name)));
    // as the window last wrote it
    let design = design_of(model);
    let settings: floorheat::Settings =
        creabim_core::results::settings_of(model, "floorheat").unwrap_or_default();
    // as many loops drawn as the design lays (George 2026-11-28)
    for rm in rooms.iter_mut() {
        if let Some(x) = design.rooms.iter().find(|x| x.id == rm.0.id && x.on) {
            rm.2.loops = x.loops.max(1);
            rm.2.wall = x.wall;
        }
    }
    let storey = model.get(level).map_or(String::new(), |e| e.name.clone());
    let pipe = design
        .pipe
        .as_ref()
        .map_or_else(|| "16x2".to_string(), |p| p.segment.replace(' ', ""));
    let mut out = vec![];
    let mut tables = vec![];
    let mut circuits = 0;
    // the rooms the pipes pass through with no loops of their own: the
    // area of transit pipes, washed yellow under a dashed edge
    let mut transit: Vec<ElementId> = vec![];
    for x in design.rooms.iter().filter(|x| x.on) {
        for t in x.route.iter().flat_map(|w| w.transit.iter()) {
            if !transit.contains(t) && !rooms.iter().any(|r| r.0.id == *t) {
                transit.push(*t);
            }
        }
    }
    for t in &transit {
        let Some(e) = model.get(*t) else { continue };
        let ElementKind::Room(r) = &e.kind else {
            continue;
        };
        if r.level != level {
            continue;
        }
        let Some(shape) = room::shape_of(model, r) else {
            continue;
        };
        if let Some(mut ring) = creabim_core::underfloor::inset_ring(
            &shape.outline,
            creabim_core::underfloor::EDGE / 2.0,
        ) {
            out.push(PlanItem::Shade {
                id: *t,
                points: ring.clone(),
                rgb: TRANSIT,
            });
            if let Some(first) = ring.first().copied() {
                ring.push(first);
            }
            strips(&mut out, *t, &ring, FIELD, Some((1.5, 1.0)), mm);
        }
        // the pipes passing give it more than its loss: insulated there
        // (George 2026-11-28)
        if design.rooms.iter().any(|x| x.id == *t && x.insulate) {
            out.push(PlanItem::Annotation {
                id: *t,
                item: Box::new(Annotation {
                    shape: Shape::Along {
                        at: r.at + DVec2::Y * -5.0 * mm,
                        angle: 0.0,
                    },
                    text: creabim_core::words::tr("Pipes passing through insulated"),
                    look: TextLook {
                        size: TEXT,
                        ..Default::default()
                    },
                    mm,
                }),
            });
        }
    }
    // each room's coils laid once, for its feeds and its loops
    let coils: std::collections::HashMap<ElementId, Vec<creabim_core::underfloor::Spiral>> = rooms
        .iter()
        .map(|(e, _, u, shape)| {
            let from = design
                .rooms
                .iter()
                .find(|x| x.id == e.id)
                .and_then(|x| x.route.as_ref())
                .and_then(|w| w.points.last().copied());
            (
                e.id,
                creabim_core::underfloor::loops_from(&shape.outline, u, from),
            )
        })
        .collect();
    // each loop's place in the bundle side by side along the way: those
    // peeling off to the left on the left, the first off outermost, those
    // to the right likewise on the right, those running on to its end in
    // the middle, so no pair crosses another as it peels off; the
    // manifold's outlets numbered as they come out, 1 on the left as one
    // faces it (George 2026-11-28)
    let mut order: Vec<(ElementId, usize, f64, f64)> = vec![];
    for (e, _, _, _) in &rooms {
        let Some(w) = design
            .rooms
            .iter()
            .find(|x| x.id == e.id)
            .and_then(|x| x.route.as_ref())
        else {
            continue;
        };
        for j in 0..coils.get(&e.id).map_or(0, |l| l.len()) {
            order.push((e.id, j, w.peel.0, w.peel.1));
        }
    }
    let rank = |o: &(ElementId, usize, f64, f64)| match o.3 {
        x if x > 0.0 => (0, o.2),
        x if x < 0.0 => (2, -o.2),
        _ => (1, o.2),
    };
    order.sort_by(|a, b| {
        let (ra, rb) = (rank(a), rank(b));
        ra.0.cmp(&rb.0).then(ra.1.total_cmp(&rb.1))
    });
    let slots = order.len() * 2;
    let slot_of = |room: ElementId, j: usize| {
        order
            .iter()
            .position(|o| o.0 == room && o.1 == j)
            .unwrap_or(0)
    };
    // the outlet of each loop: facing the manifold its left is the pipes'
    // right as they leave it
    let outlet = |room: ElementId, j: usize| order.len() - slot_of(room, j);
    // each loop's supply and return from the manifold, side by side
    let mut outlets: Vec<(ElementId, DVec2, usize)> = vec![];
    let mut rets: Vec<(ElementId, usize, Vec<DVec2>)> = vec![];
    for (e, _, u, shape) in &rooms {
        let Some(w) = design
            .rooms
            .iter()
            .find(|x| x.id == e.id)
            .and_then(|x| x.route.as_ref())
        else {
            continue;
        };
        let Some(loops) = coils.get(&e.id) else {
            continue;
        };
        for (j, sp) in loops.iter().enumerate() {
            let (Some(a), Some(b)) = (sp.supply.first(), sp.ret.last()) else {
                continue;
            };
            let idx = slot_of(e.id, j) * 2;
            let off = ((slots as f64 - 1.0) / 2.0 - idx as f64 - 1.0) * PITCH;
            // to the loop's ends, the last step straight onto them
            // along the way side by side, the last steps square onto
            // the loop's end
            let along = |end: DVec2, d: f64| {
                let c = tidy(&square(&w.points, end));
                let body = &c[..c.len().saturating_sub(1)];
                let o = offset(body, d);
                square(&o, end)
            };
            // the return outermost on the side it peels off to, so that a
            // spiral off it crosses no other pipe
            let right = w.peel.1 < 0.0;
            let (so, ro) = if right {
                (off + PITCH, off)
            } else {
                (off, off + PITCH)
            };
            let sup = along(*a, so);
            let ret = along(*b, ro);
            // the outlet's number at the manifold, between its two pipes
            if let (Some(p), Some(q)) = (sup.first(), ret.first()) {
                let u = sup
                    .get(1)
                    .map_or(DVec2::Y, |n| (*n - *p).normalize_or_zero());
                outlets.push((e.id, (*p + *q) / 2.0 - u * 1.8 * mm, outlet(e.id, j)));
            }
            // the pipes in sleeves where they cross a doorway's joint
            for [p, q] in &w.sides {
                let u = (*q - *p).normalize_or_zero();
                let ext = SLEEVE_HALF;
                for d in [off, off + PITCH] {
                    let o = offset(&[*p - u * ext, *q + u * ext], d);
                    strips(&mut out, e.id, &o, SLEEVE, Some((0.3, 0.5)), mm);
                }
            }
            // and where they cross a joint between the room's loops
            for j in creabim_core::underfloor::joints(&shape.outline, u) {
                for pipe in [&sup, &ret] {
                    for w in pipe.windows(2) {
                        let (a, b) = (w[0], w[1]);
                        let (r, q) = (b - a, j[1] - j[0]);
                        let den = r.perp_dot(q);
                        if den.abs() < 1e-12 {
                            continue;
                        }
                        let t = (j[0] - a).perp_dot(q) / den;
                        let v = (j[0] - a).perp_dot(r) / den;
                        if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&v) {
                            let x = a + r * t;
                            let d = r.normalize_or_zero() * JOINT_SLEEVE_HALF;
                            strips(
                                &mut out,
                                e.id,
                                &[x - d, x + d],
                                SLEEVE,
                                Some((0.3, 0.5)),
                                mm,
                            );
                        }
                    }
                }
            }
            strips(&mut out, e.id, &sup, SUPPLY, None, mm);
            rets.push((e.id, j, ret));
        }
    }
    // the spirals off a loop's return into rooms with no loop of their
    // own (George 2026-11-28): the return cut where it passes the room's
    // way off the route, two leads in to the spiral and back out
    for x in design.rooms.iter().filter(|x| !x.on) {
        let (Some(sp), Some(w)) = (x.spiral.as_ref(), x.route.as_ref()) else {
            continue;
        };
        let Some(e) = model.get(x.id) else { continue };
        let ElementKind::Room(r) = &e.kind else {
            continue;
        };
        if r.level != level {
            continue;
        }
        let Some(shape) = room::shape_of(model, r) else {
            continue;
        };
        // the host room's loop outermost on the room's side, and of its
        // return the piece passing the way off (it may be cut already)
        let side = w.peel.1;
        let js: Vec<usize> = order
            .iter()
            .filter(|o| o.0 == sp.host)
            .map(|o| o.1)
            .collect();
        let Some(j) = (if side < 0.0 {
            js.iter().copied().max_by_key(|j| slot_of(sp.host, *j))
        } else {
            js.iter().copied().min_by_key(|j| slot_of(sp.host, *j))
        }) else {
            continue;
        };
        let Some(k) = rets
            .iter()
            .enumerate()
            .filter(|(_, h)| h.0 == sp.host && h.1 == j && h.2.len() > 1)
            .min_by(|a, b| {
                near_on(&a.1 .2, w.peel_at)
                    .0
                    .distance(w.peel_at)
                    .total_cmp(&near_on(&b.1 .2, w.peel_at).0.distance(w.peel_at))
            })
            .map(|(k, _)| k)
        else {
            continue;
        };
        let u = creabim_core::underfloor::Underfloor {
            spacing: sp.spacing,
            loops: 1,
            layout: creabim_core::underfloor::Layout::Spiral,
            ..Default::default()
        };
        let Some(coil) =
            creabim_core::underfloor::loops_from(&shape.outline, &u, w.points.last().copied())
                .into_iter()
                .next()
        else {
            continue;
        };
        let (Some(a), Some(b)) = (coil.supply.first().copied(), coil.ret.last().copied()) else {
            continue;
        };
        // where the way to the room leaves the main route, the return
        // nearest it
        let peel = w.peel_at;
        let (at, run) = near_on(&rets[k].2, peel);
        let _ = at;
        let (p0, p1) = (
            point_at(&rets[k].2, run - PITCH),
            point_at(&rets[k].2, run + PITCH),
        );
        // the way in after the route, from the return's two cut ends
        let tail: Vec<DVec2> = {
            let i = w
                .points
                .iter()
                .position(|p| p.distance(peel) < 1e-6)
                .unwrap_or(0);
            w.points[i..].to_vec()
        };
        let lead = |from: DVec2, end: DVec2, d: f64| {
            let mut c = vec![from];
            c.extend(offset(&tail, d).into_iter().skip(1));
            tidy(&square(&c[..c.len().saturating_sub(1)], end))
        };
        let l_in = lead(p0, a, PITCH / 2.0);
        let l_out = lead(p1, b, -PITCH / 2.0);
        let cut = cut_at(&rets[k].2, run - PITCH, run + PITCH);
        rets[k].2 = cut.0;
        rets.push((sp.host, j, cut.1));
        for piece in [&l_in, &l_out] {
            strips(&mut out, x.id, piece, RETURN, Some(DASH), mm);
        }
        for (pts, _) in coil.pieces() {
            strips(&mut out, x.id, pts, RETURN, Some(DASH), mm);
        }
    }
    for (id, _, ret) in &rets {
        strips(&mut out, *id, ret, RETURN, Some(DASH), mm);
    }
    for (id, at, n) in &outlets {
        out.push(PlanItem::Annotation {
            id: *id,
            item: Box::new(Annotation {
                shape: Shape::Along {
                    at: *at,
                    angle: 0.0,
                },
                text: n.to_string(),
                look: TextLook {
                    size: TEXT * 0.8,
                    ..Default::default()
                },
                mm,
            }),
        });
    }
    for (e, r, u, shape) in &rooms {
        let Some(loops) = coils.get(&e.id) else {
            continue;
        };
        for sp in loops {
            for (pts, supply) in sp.pieces() {
                if supply {
                    strips(&mut out, e.id, pts, SUPPLY, None, mm);
                } else {
                    strips(&mut out, e.id, pts, RETURN, Some(DASH), mm);
                }
            }
        }
        zone_items(&mut out, e.id, u, mm);
        // the movement joints between its loops
        for j in creabim_core::underfloor::joints(&shape.outline, u) {
            strips(&mut out, e.id, &j, JOINT, Some(JOINT_DASH), mm);
        }
        let n = loops.len().max(1);
        let mut nums: Vec<usize> = (0..loops.len())
            .filter(|j| order.iter().any(|o| o.0 == e.id && o.1 == *j))
            .map(|j| outlet(e.id, j))
            .collect();
        nums.sort();
        let which = match nums.as_slice() {
            [] if n > 1 => format!("{}-{}", circuits + 1, circuits + n),
            [] => format!("{}", circuits + 1),
            [one] => format!("{one}"),
            many => many
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(","),
        };
        circuits += n;
        tables.push((
            e.id,
            r.at,
            which,
            format!("{} - {}", r.number, e.name),
            design.rooms.iter().find(|x| x.id == e.id),
            shape.outline.clone(),
            u.edges().next().map(|z| z.1),
        ));
        // the heated field's edge, a yellow dashed line
        if let Some(mut ring) = creabim_core::underfloor::inset_ring(
            &shape.outline,
            creabim_core::underfloor::EDGE / 2.0,
        ) {
            if let Some(first) = ring.first().copied() {
                ring.push(first);
            }
            strips(&mut out, e.id, &ring, FIELD, Some((1.5, 1.0)), mm);
        }
    }
    let look = TextLook {
        size: TEXT,
        ..Default::default()
    };
    let note = |id, at, text: String, look| PlanItem::Annotation {
        id,
        item: Box::new(Annotation {
            shape: Shape::Along { at, angle: 0.0 },
            text,
            look,
            mm,
        }),
    };
    for (room_id, at, which, name, d, outline, edge) in tables {
        // the room's table tag in this view: the table where it was moved
        // to, picked and dragged as the tag (George 2026-11-28)
        let tag = view.and_then(|v| {
            model.iter().find_map(|t| match &t.kind {
                ElementKind::Tag(x) if x.ufh && x.view == v && x.host == room_id => {
                    Some((t.id, x.offset))
                }
                _ => None,
            })
        });
        let id = tag.map_or(room_id, |t| t.0);
        // two columns, as the office's table of a circuit
        let mut rows = vec![(
            format!("RZ 1 - {storey} ({circuits}/{which})"),
            format!("tp = {} °C", num(settings.supply_c, 0)),
        )];
        if let Some(d) = d {
            // the edge zone's spacing beside the room's (VYK.001: Lpz/Loz)
            let spacing = match edge {
                Some(oz) => format!(
                    "Lpz/Loz = {}/{} mm",
                    num(d.spacing * 1000.0, 0),
                    num(oz * 1000.0, 0)
                ),
                None => format!("Lpz = {} mm", num(d.spacing * 1000.0, 0)),
            };
            rows.extend([
                (spacing, format!("L = {} m", num(d.loop_len, 1))),
                (name, format!("{} {pipe}", creabim_core::words::tr("Pipe"))),
                (
                    format!("Δp = {} kPa", num(d.dp_kpa, 1)),
                    format!("{} l/min", num(d.flow_ls * 60.0, 2)),
                ),
                (
                    format!("S = {} m²", num(d.area, 1)),
                    format!("{} °C, {} W", num(d.ti, 0), num(d.phi, 0)),
                ),
            ]);
        }
        let pad = 0.8;
        let row = TEXT * 1.5;
        let col = |pick: &dyn Fn(&(String, String)) -> &String| {
            rows.iter().map(|r| look.width(pick(r))).fold(0.0, f64::max) + 2.0 * pad
        };
        let (w0, w1) = (col(&|r| &r.0), col(&|r| &r.1));
        let h = rows.len() as f64 * row;
        // under the room's name, or where its tag was moved to
        let offset = tag.map_or(DVec2::Y * -9.0 * mm, |t| t.1);
        let top = at + offset - DVec2::X * (w0 + w1) / 2.0 * mm;
        let p = |x: f64, y: f64| top + DVec2::new(x, -y) * mm;
        // on a leader to the room's label point once out of the room
        let box_ = [p(0.0, 0.0), p(w0 + w1, 0.0), p(w0 + w1, h), p(0.0, h)];
        let middle = p((w0 + w1) / 2.0, h / 2.0);
        if !creabim_core::polygon::contains(&outline, middle) {
            let (lo, hi) = (box_[3], box_[1]);
            let from = at.clamp(lo, hi);
            out.push(PlanItem::Line {
                id,
                pen: crate::plan::Pen::Fine,
                a: from,
                b: at,
            });
            let r = 0.6 * mm;
            out.push(PlanItem::Shade {
                id,
                points: (0..12)
                    .map(|k| {
                        let a = k as f64 / 12.0 * std::f64::consts::TAU;
                        at + DVec2::new(a.cos(), a.sin()) * r
                    })
                    .collect(),
                rgb: [0.0, 0.0, 0.0],
            });
        }
        // the table on white, over the loops
        out.push(PlanItem::Shade {
            id,
            points: box_.to_vec(),
            rgb: [1.0, 1.0, 1.0],
        });
        let mut rule = |a: DVec2, b: DVec2| {
            out.push(PlanItem::Line {
                id,
                pen: crate::plan::Pen::Fine,
                a,
                b,
            })
        };
        for k in 0..=rows.len() {
            let y = k as f64 * row;
            rule(p(0.0, y), p(w0 + w1, y));
        }
        for x in [0.0, w0, w0 + w1] {
            rule(p(x, 0.0), p(x, h));
        }
        for (k, (l, r)) in rows.into_iter().enumerate() {
            let y = (k as f64 + 0.5) * row;
            out.push(note(id, p(w0 / 2.0, y), l, look));
            out.push(note(id, p(w0 + w1 / 2.0, y), r, look));
        }
    }
    // the manifold's mark: its number and how many circuits leave it
    for e in model.iter() {
        let ElementKind::Object(ob) = &e.kind else {
            continue;
        };
        if ob.level == level && fixture::family_of(model, e).is_some_and(fixture::is_manifold) {
            let look = TextLook {
                size: 3.0,
                rgb: Some(crate::mep::LABEL_GREEN),
                ..Default::default()
            };
            let at = ob.to_plan(DVec2::ZERO) + DVec2::Y * 5.0 * mm;
            out.push(note(e.id, at, format!("RZ 1/{circuits}"), look));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::underfloor::Underfloor;
    use creabim_core::{Document, Element, Room, Transaction};

    #[test]
    fn dashes_run_on_round_corners() {
        let mut items = vec![];
        let id = ElementId::new();
        let pts = [DVec2::ZERO, DVec2::X * 5.0, DVec2::new(5.0, 6.0)];
        strips(&mut items, id, &pts, RETURN, Some((4.0, 1.5)), 1.0);
        // 11 mm of 4 + 1.5: dashes at 0-4, 5.5-9.5 (round the corner:
        // one strip a leg), 11
        let len: f64 = items
            .iter()
            .map(|i| match i {
                PlanItem::Shade { points, .. } => points[0].distance(points[1]),
                _ => 0.0,
            })
            .sum();
        assert!((len - 8.0).abs() < 1e-6, "{len}");
        let mut full = vec![];
        strips(&mut full, id, &pts, SUPPLY, None, 1.0);
        assert_eq!(full.len(), 2);
    }

    #[test]
    fn a_heated_room_gets_coloured_loops_and_its_table() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        assert!(items(doc.model(), level, None, 0.1).is_empty());
        let mut r = Room::new(level, DVec2::new(2.5, 3.0));
        r.number = "1.01".into();
        r.ufh = Some(Underfloor {
            spacing: 0.15,
            loops: 2,
            finish_r: None,
            ..Underfloor::default()
        });
        let room = Element::new("Kúpeľňa", ElementKind::Room(r));
        let id = room.id;
        doc.commit(Transaction::new("room").insert(room)).unwrap();
        let out = items(doc.model(), level, None, 0.1);
        let count = |c: [f32; 3]| {
            out.iter()
                .filter(|i| matches!(i, PlanItem::Shade { rgb, .. } if *rgb == c))
                .count()
        };
        // the return dashed: many more strips than the supply's
        assert!(count(SUPPLY) > 10 && count(RETURN) > 3 * count(SUPPLY));
        let table = out
            .iter()
            .filter_map(|i| match i {
                PlanItem::Annotation { id: i, item } if *i == id => Some(item.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(table.contains("(2/1-2)"), "{table}");
        assert!(
            table.contains("Lpz = 150 mm") && table.contains("16x2"),
            "{table}"
        );
        // framed: its rules, and the field's yellow edge
        assert!(out
            .iter()
            .any(|i| matches!(i, PlanItem::Line { id: i, .. } if *i == id)));
        assert!(count(FIELD) > 4);
        assert!(table.contains("l/min") && table.contains("kPa"), "{table}");
    }
}
