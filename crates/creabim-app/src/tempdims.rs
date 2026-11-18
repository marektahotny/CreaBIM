//! Temporary dimensions (brief 02, 2.10 part 3): selecting a wall, a grid
//! line, a door or window, a column, or any other element on a level
//! (furniture, fixtures, equipment, a stair…) in a plan shows blue
//! dimensions to its nearest neighbours. As in PilonDesigner they measure
//! walls from the faces of their load-bearing core by default, never from
//! plaster, insulation or the centreline (the view strip's *Measure from*). Click a value and type a new one: the element
//! moves to make it true, and what it carries follows (the doors in a wall,
//! the walls on a grid line, the walls joined to a moved wall). The padlock
//! beside a value keeps it as a dimension that holds the spacing (see
//! [`creabim_core::holds`]); clicking a closed padlock lets it go.
//!
//! The typing is the tracker's (2.10 part 1): digits and length marks go in,
//! Backspace takes back, Enter sets it, Esc stops, and Tab moves on to the
//! next value.

use creabim_core::holds::{self, Place};
use creabim_core::walls::{self, WallRef};
use creabim_core::{Dimension, Element, ElementId, ElementKind, Model, Transaction, ViewKind};
use eframe::egui::{self, Color32, Painter, Pos2, Rect, Stroke, Vec2};
use glam::DVec2;
use std::collections::HashSet;

/// How a temporary dimension's value moves its element.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum How {
    /// The whole element moves along `n` (unit, from the neighbour toward
    /// the element): a wall, grid line or column.
    Spacing { n: DVec2 },
    /// A door or window slides along its wall; the value is the gap on its
    /// start side or its end side.
    Along { start_side: bool },
    /// A wall's own length (A4-7): typing it moves its `end` (else its
    /// start), the end nearer the middle of the view.
    Length { end: bool },
}

/// One temporary dimension, in the plan's drawing.
#[derive(Clone, Debug, PartialEq)]
pub struct TempDim {
    /// The selected element, which moves.
    pub owner: ElementId,
    /// The neighbour measured to; none for a wall's end.
    pub other: Option<ElementId>,
    /// The point on the element, and the one on the neighbour.
    pub a: DVec2,
    pub b: DVec2,
    pub value: f64,
    pub how: How,
    /// The held dimension between the two already, if there is one.
    pub held: Option<ElementId>,
    /// What the neighbour is called in the status bar: "grid A", "W03".
    pub to: String,
}

/// The temporary dimension being typed into: in which view, for which
/// element, which of its values, and what has been typed.
#[derive(Clone, Debug, Default)]
pub struct TempState {
    pub editing: Option<(ElementId, ElementId, usize)>,
    pub typed: crate::tracker::Tracker,
    /// Where the pane was last clicked (drawing units): a dimension between
    /// two levels moves the level at the end nearer it (A5-2).
    pub click: Option<DVec2>,
}

/// A placed dimension whose two ends are on levels (in a section or
/// elevation): each end's level and its point in the drawing.
pub fn dim_levels(model: &Model, id: ElementId) -> Option<[(ElementId, DVec2); 2]> {
    let ElementKind::Dimension(d) = &model.get(id)?.kind else {
        return None;
    };
    let end = |p: &creabim_core::HostPoint| {
        let h = p.host?;
        let ElementKind::Level(l) = &model.get(h)?.kind else {
            return None;
        };
        Some((h, DVec2::new(p.local.x, l.elevation + p.local.y)))
    };
    let (a, b) = (end(&d.a)?, end(&d.b)?);
    (a.0 != b.0).then_some([a, b])
}

/// The value box of a placed dimension between two levels: typing moves
/// the level at the end nearer `click`, the other stays (A5-2).
fn dim_level_temp(model: &Model, id: ElementId, click: DVec2) -> Option<TempDim> {
    let [p, q] = dim_levels(model, id)?;
    let (mine, other) = if p.1.distance(click) <= q.1.distance(click) {
        (p, q)
    } else {
        (q, p)
    };
    let z = |l: ElementId| model.level(l).map_or(0.0, |l| l.elevation);
    let (zm, zo) = (z(mine.0), z(other.0));
    Some(TempDim {
        owner: mine.0,
        other: Some(other.0),
        a: mine.1,
        b: other.1,
        value: (zm - zo).abs(),
        how: How::Spacing {
            n: DVec2::new(0.0, if zm >= zo { 1.0 } else { -1.0 }),
        },
        held: matches!(&model.get(id)?.kind, ElementKind::Dimension(d) if d.held.is_some())
            .then_some(id),
        to: name_of(model, other.0),
    })
}

fn name_of(model: &Model, id: ElementId) -> String {
    match model.get(id).map(|e| &e.kind) {
        Some(ElementKind::Grid(g)) => {
            format!("grid {}", creabim_core::grids::display_label(model, g))
        }
        Some(_) => model.get(id).map(|e| e.name.clone()).unwrap_or_default(),
        None => String::new(),
    }
}

fn level_of(model: &Model, id: ElementId) -> Option<ElementId> {
    match &model.get(id)?.kind {
        ElementKind::Wall(w) => Some(w.level),
        ElementKind::Column(c) => Some(c.level),
        ElementKind::Opening(o) => level_of(model, o.host),
        ElementKind::Object(o) => Some(o.level),
        ElementKind::Stair(s) => Some(s.level),
        ElementKind::Slab(s) => Some(s.level),
        ElementKind::Beam(b) => Some(b.level),
        ElementKind::Footing(f) => Some(f.level),
        ElementKind::Railing(r) => Some(r.level),
        ElementKind::Pipe(p) => Some(p.level),
        _ => None,
    }
}

/// The lines a selected element measures to, in a plan of `level`: grid
/// lines, and the walls on that level (but not `id` itself), each wall by
/// the lines `wref` names on it (its core's faces unless set otherwise):
/// (owner, a, b).
fn ref_lines(
    model: &Model,
    level: ElementId,
    id: ElementId,
    wref: WallRef,
) -> Vec<(ElementId, DVec2, DVec2)> {
    let mut out = vec![];
    for e in model.iter().filter(|e| e.id != id) {
        match &e.kind {
            ElementKind::Grid(g) if g.start.distance(g.end) > 1e-6 => {
                out.push((e.id, g.start, g.end))
            }
            ElementKind::Wall(w) if w.level == level && w.length() > 1e-6 => {
                let n = w.direction().perp();
                for o in walls::ref_offsets(model, w, wref) {
                    out.push((e.id, w.start + n * o, w.end + n * o));
                }
            }
            ElementKind::Column(c) if c.level == level => {
                let pts = c.outline();
                for i in 0..pts.len() {
                    out.push((e.id, pts[i], pts[(i + 1) % pts.len()]));
                }
            }
            _ => {}
        }
    }
    out
}

/// Where the ray from `p` along unit `d` first crosses the segment `a`–`b`:
/// the distance along the ray.
fn ray_hit(p: DVec2, d: DVec2, a: DVec2, b: DVec2) -> Option<f64> {
    let e = b - a;
    let den = d.perp_dot(e);
    if den.abs() < 1e-9 {
        return None;
    }
    let w = a - p;
    let t = w.perp_dot(e) / den;
    let s = w.perp_dot(d) / den;
    (t > 1e-6 && (-1e-9..=1.0 + 1e-9).contains(&s)).then_some(t)
}

/// The box an element covers in plan, from its own shape where that is
/// quick to know, else from `foot` (its 3D body's box, from the pane).
fn footprint(model: &Model, id: ElementId, foot: Option<(DVec2, DVec2)>) -> Option<(DVec2, DVec2)> {
    let bbox = |pts: &[DVec2]| {
        let first = *pts.first()?;
        Some(
            pts.iter()
                .fold((first, first), |(lo, hi), p| (lo.min(*p), hi.max(*p))),
        )
    };
    match &model.get(id)?.kind {
        ElementKind::Column(c) => bbox(&c.outline()),
        ElementKind::Object(o) => {
            let t = creabim_core::objects::type_of_id(model, id)?;
            let (lo, hi) = t.bounds();
            bbox(&[
                o.to_plan(DVec2::new(lo.x, lo.y)),
                o.to_plan(DVec2::new(hi.x, lo.y)),
                o.to_plan(DVec2::new(hi.x, hi.y)),
                o.to_plan(DVec2::new(lo.x, hi.y)),
            ])
        }
        ElementKind::Slab(s) => bbox(&s.boundary),
        _ => foot,
    }
}

/// The temporary dimensions of `id` in a plan of `level`. `near` (a drawing
/// point, the middle of the pane) places a grid line's dimensions where they
/// can be seen. Walls are measured by `wref` (PilonDesigner: the faces of
/// their load-bearing core, never plaster or the centreline); `foot` is the
/// box the element's body covers, for elements measured by their sides.
pub fn find(
    model: &Model,
    level: ElementId,
    id: ElementId,
    near: DVec2,
    wref: WallRef,
    foot: Option<(DVec2, DVec2)>,
) -> Vec<TempDim> {
    let Some(e) = model.get(id) else {
        return vec![];
    };
    // a level in a section or elevation (A5-2): its height above the level
    // below and below the level above, at the middle of the pane
    if let ElementKind::Level(l) = &e.kind {
        let z = l.elevation;
        let others: Vec<(ElementId, f64)> = model
            .iter()
            .filter(|o| o.id != id)
            .filter_map(|o| match &o.kind {
                ElementKind::Level(m) => Some((o.id, m.elevation)),
                _ => None,
            })
            .collect();
        let below = others
            .iter()
            .filter(|o| o.1 < z - 1e-9)
            .max_by(|a, b| a.1.total_cmp(&b.1));
        let above = others
            .iter()
            .filter(|o| o.1 > z + 1e-9)
            .min_by(|a, b| a.1.total_cmp(&b.1));
        return [below, above]
            .into_iter()
            .flatten()
            .map(|&(o, oz)| TempDim {
                owner: id,
                other: Some(o),
                a: DVec2::new(near.x, z),
                b: DVec2::new(near.x, oz),
                value: (z - oz).abs(),
                // from the neighbour toward the level
                how: How::Spacing {
                    n: DVec2::new(0.0, if z > oz { 1.0 } else { -1.0 }),
                },
                held: holds::holding(model, id, o),
                to: name_of(model, o),
            })
            .collect();
    }
    // a grid line is in every plan; the rest must be on this level
    if !matches!(e.kind, ElementKind::Grid(_)) && level_of(model, id) != Some(level) {
        return vec![];
    }
    let held = |other: ElementId| holds::holding(model, id, other);
    match &e.kind {
        ElementKind::Wall(_) | ElementKind::Grid(_) => {
            let Some(Place::Line(p, q)) = holds::place_of(model, id) else {
                return vec![];
            };
            let u = (q - p).normalize_or_zero();
            let n0 = u.perp();
            let len = p.distance(q);
            let is_grid = matches!(e.kind, ElementKind::Grid(_));
            // the element's own lines across it: a wall's by `wref`
            let mine: Vec<f64> = match &e.kind {
                ElementKind::Wall(w) => walls::ref_offsets(model, w, wref),
                _ => vec![0.0],
            };
            // a grid measures to grids; a wall to walls and grids
            let mut best: [Option<(f64, TempDim)>; 2] = [None, None];
            for o in model.iter().filter(|o| o.id != id) {
                let (r, s, theirs) = match &o.kind {
                    ElementKind::Grid(g) if g.start.distance(g.end) > 1e-6 => {
                        (g.start, g.end, vec![0.0])
                    }
                    ElementKind::Wall(w) if !is_grid && w.level == level && w.length() > 1e-6 => {
                        (w.start, w.end, walls::ref_offsets(model, w, wref))
                    }
                    _ => continue,
                };
                let oid = o.id;
                let v = (s - r).normalize_or_zero();
                if u.perp_dot(v).abs() > 2e-3 {
                    continue;
                }
                let off = (r - p).dot(n0);
                // a neighbour more than 10 m off measures nothing useful
                // (A4-7)
                if off.abs() < 0.001 || off.abs() > 10.0 {
                    continue;
                }
                let other_is_grid = matches!(o.kind, ElementKind::Grid(_));
                // where along the element the dimension sits: the middle of
                // where the two run side by side; a grid line's, nearest
                // the middle of the pane
                let (r0, r1) = {
                    let (x, y) = ((r - p).dot(u), (s - p).dot(u));
                    (x.min(y), x.max(y))
                };
                let (lo, hi) = (r0.max(0.0), r1.min(len));
                let t = if is_grid {
                    (near - p).dot(u).clamp(lo.min(hi), hi.max(lo))
                } else if lo < hi {
                    (lo + hi) / 2.0
                } else if other_is_grid {
                    len / 2.0
                } else {
                    // walls side by side only
                    continue;
                };
                // the neighbour's lines in this element's frame (it may run
                // the other way)
                let flip = if v.perp().dot(n0) < 0.0 { -1.0 } else { 1.0 };
                let theirs: Vec<f64> = theirs.iter().map(|x| off + x * flip).collect();
                let max = |v: &[f64]| v.iter().copied().fold(f64::MIN, f64::max);
                let min = |v: &[f64]| v.iter().copied().fold(f64::MAX, f64::min);
                // from this element's line on the neighbour's side to the
                // neighbour's line facing it
                let (from, to) = if off > 0.0 {
                    (max(&mine), min(&theirs))
                } else {
                    (min(&mine), max(&theirs))
                };
                let gap = (to - from) * off.signum();
                if gap < 0.001 {
                    continue;
                }
                let side = usize::from(off < 0.0);
                if best[side].as_ref().is_some_and(|(d, _)| *d <= gap) {
                    continue;
                }
                let a = p + u * t + n0 * from;
                best[side] = Some((
                    gap,
                    TempDim {
                        owner: id,
                        other: Some(oid),
                        a,
                        b: p + u * t + n0 * to,
                        value: gap,
                        how: How::Spacing {
                            n: -n0 * off.signum(),
                        },
                        held: held(oid),
                        to: name_of(model, oid),
                    },
                ));
            }
            let mut out: Vec<TempDim> = best.iter().flatten().map(|(_, d)| d.clone()).collect();
            // a wall always shows its own length, beside it on the side
            // with no gap shown (A4-7)
            if let ElementKind::Wall(w) = &e.kind {
                let side = if best[0].is_some() && best[1].is_none() {
                    -1.0
                } else {
                    1.0
                };
                let off = n0 * side * (w.thickness / 2.0 + 0.6);
                let end = (near - q).length() < (near - p).length();
                out.push(TempDim {
                    owner: id,
                    other: None,
                    a: p + off,
                    b: q + off,
                    value: len,
                    how: How::Length { end },
                    held: None,
                    to: "its length".into(),
                });
            }
            out
        }
        ElementKind::Opening(o) => {
            let Some(ElementKind::Wall(w)) = model.get(o.host).map(|h| &h.kind) else {
                return vec![];
            };
            let len = w.length();
            let d = w.direction();
            let n = d.perp();
            let (u0, u1) = (o.offset, o.offset + o.width);
            // what a jamb measures to along its wall: the wall's ends,
            // other openings' jambs, and where the walls meeting this one
            // have their reference lines (PilonDesigner: the core faces of
            // the adjoining walls, not their centrelines)
            let mut cands: Vec<(f64, Option<ElementId>)> = vec![(0.0, None), (len, None)];
            for x in model.iter().filter(|x| x.id != id) {
                match &x.kind {
                    ElementKind::Opening(p) if p.host == o.host => {
                        cands.push((p.offset, Some(x.id)));
                        cands.push((p.offset + p.width, Some(x.id)));
                    }
                    ElementKind::Wall(ow)
                        if x.id != o.host && ow.level == w.level && ow.length() > 1e-6 =>
                    {
                        let ou = ow.direction();
                        if d.perp_dot(ou).abs() < 0.2 {
                            continue;
                        }
                        // it must meet this wall: its centreline reaches it
                        let Some(tw) = ray_hit(w.start - d * 1e3, d, ow.start, ow.end)
                            .map(|t| t - 1e3)
                            .or_else(|| {
                                // or stops within this wall's thickness of it
                                let near_end = [ow.start, ow.end].into_iter().find(|e| {
                                    ((*e - w.start).dot(n)).abs() <= w.thickness / 2.0 + 0.05
                                })?;
                                Some((near_end - w.start).dot(d))
                            })
                        else {
                            continue;
                        };
                        if tw < -ow.thickness || tw > len + ow.thickness {
                            continue;
                        }
                        let on = ou.perp();
                        for off in walls::ref_offsets(model, ow, wref) {
                            if let Some(t) = ray_hit(
                                w.start - d * 1e3,
                                d,
                                ow.start + on * off - ou * 1e3,
                                ow.end + on * off + ou * 1e3,
                            ) {
                                cands.push((t - 1e3, Some(x.id)));
                            }
                        }
                    }
                    _ => {}
                }
            }
            let s = w.thickness / 2.0 + 0.35;
            let on = |u: f64| w.start + d * u + n * s;
            let to = |o: Option<ElementId>| match o {
                Some(x) => name_of(model, x),
                None => "the wall's end".to_string(),
            };
            let mut out = vec![];
            // the nearest on each side of the opening
            let left = cands
                .iter()
                .filter(|c| c.0 < u0 - 1e-4)
                .max_by(|a, b| a.0.total_cmp(&b.0));
            if let Some(&(at, other)) = left {
                out.push(TempDim {
                    owner: id,
                    other,
                    a: on(u0),
                    b: on(at),
                    value: u0 - at,
                    how: How::Along { start_side: true },
                    held: None,
                    to: to(other),
                });
            }
            let right = cands
                .iter()
                .filter(|c| c.0 > u1 + 1e-4)
                .min_by(|a, b| a.0.total_cmp(&b.0));
            if let Some(&(at, other)) = right {
                out.push(TempDim {
                    owner: id,
                    other,
                    a: on(u1),
                    b: on(at),
                    value: at - u1,
                    how: How::Along { start_side: false },
                    held: None,
                    to: to(other),
                });
            }
            out
        }
        // a column, furniture, a fixture, equipment, a stair, a footing…:
        // from the middle of each side of its box straight out to the
        // nearest wall core, column or grid line (PilonDesigner: every
        // element has its controlling dimensions, equipment too)
        _ => {
            let Some((lo, hi)) = footprint(model, id, foot) else {
                return vec![];
            };
            let c = (lo + hi) / 2.0;
            let inside = |p: DVec2| {
                p.x > lo.x - 1e-3 && p.x < hi.x + 1e-3 && p.y > lo.y - 1e-3 && p.y < hi.y + 1e-3
            };
            let lines: Vec<(ElementId, DVec2, DVec2)> = ref_lines(model, level, id, wref)
                .into_iter()
                .filter(|(_, a, b)| !(inside(*a) && inside(*b)))
                .collect();
            let mut out = vec![];
            for (from, dir) in [
                (DVec2::new(hi.x, c.y), DVec2::X),
                (DVec2::new(lo.x, c.y), -DVec2::X),
                (DVec2::new(c.x, hi.y), DVec2::Y),
                (DVec2::new(c.x, lo.y), -DVec2::Y),
            ] {
                let hit = lines
                    .iter()
                    .filter_map(|(o, a, b)| Some((ray_hit(from, dir, *a, *b)?, *o)))
                    .filter(|(t, _)| *t > 0.001 && *t <= 10.0)
                    .min_by(|x, y| x.0.total_cmp(&y.0));
                let Some((t, oid)) = hit else { continue };
                out.push(TempDim {
                    owner: id,
                    other: Some(oid),
                    a: from,
                    b: from + dir * t,
                    value: t,
                    how: How::Spacing { n: -dir },
                    held: held(oid),
                    to: name_of(model, oid),
                });
            }
            out
        }
    }
}

/// `d` made `value` long: the transaction that moves its element, with the
/// status bar's message; else why it can't.
pub fn set(model: &Model, d: &TempDim, value: f64) -> Result<(Transaction, String), String> {
    if value <= 0.0 {
        return Err("Type a distance more than zero.".into());
    }
    let e = model.get(d.owner).ok_or("It is gone.")?;
    let delta = value - d.value;
    if delta.abs() < 1e-7 {
        return Err("Already that far.".into());
    }
    let label = format!("Set {} from {}", e.name, d.to);
    let mut tx = Transaction::new(label);
    match d.how {
        How::Spacing { n } => {
            let keep: HashSet<ElementId> = d.other.into_iter().collect();
            for (o, now) in holds::shift(model, d.owner, n * delta, &keep) {
                tx = tx.replace(o, now);
            }
            // a held spacing typed anew is held at the new value
            if let Some(h) = d.held.and_then(|h| model.get(h)) {
                let mut now = h.clone();
                if let ElementKind::Dimension(dim) = &mut now.kind {
                    dim.held = Some(value);
                }
                tx = tx.replace(h.clone(), now);
            }
        }
        How::Length { end } => {
            // the end moves as its grip would, walls joined to it following
            let kind = crate::handles::HandleKind::WallEnd { end };
            let ElementKind::Wall(w) = &e.kind else {
                return Err("It is not a wall.".into());
            };
            let (from, dir) = if end {
                (w.end, w.direction())
            } else {
                (w.start, -w.direction())
            };
            let z = model.level(w.level).map_or(0.0, |l| l.elevation);
            let (orig, joined) = crate::handles::originals(model, d.owner, kind);
            let out = crate::handles::apply(
                model,
                &orig,
                &joined,
                kind,
                from.extend(z),
                (from + dir * delta).extend(z),
                0.0,
            )?;
            for (o, now) in orig.into_iter().zip(out) {
                tx = tx.replace(o, now);
            }
        }
        How::Along { start_side } => {
            let ElementKind::Opening(o) = &e.kind else {
                return Err("It is not in a wall.".into());
            };
            let len = match model.get(o.host).map(|h| &h.kind) {
                Some(ElementKind::Wall(w)) => w.length(),
                _ => return Err("It is not in a wall.".into()),
            };
            let offset = o.offset + if start_side { delta } else { -delta };
            if offset < -1e-9 || offset + o.width > len + 1e-9 {
                return Err(format!("{} would run past the end of its wall.", e.name));
            }
            let mut now = e.clone();
            if let ElementKind::Opening(o) = &mut now.kind {
                o.offset = offset.max(0.0);
            }
            tx = tx.replace(e.clone(), now);
        }
    }
    let way = if delta > 0.0 { "+" } else { "−" };
    let msg = format!(
        "Moved {} {way}{}: {} → {} from {}. Ctrl+Z to undo.",
        e.name,
        crate::units::len(delta.abs()),
        crate::units::len(d.value),
        crate::units::len(value),
        d.to
    );
    Ok((tx, msg))
}

/// The padlock: keep `d` as a dimension in `view` that holds its spacing,
/// or, when it is held already, let it go.
pub fn padlock(model: &Model, view: ElementId, d: &TempDim) -> Option<(Transaction, String)> {
    if let Some(h) = d.held.and_then(|h| model.get(h)) {
        let mut now = h.clone();
        if let ElementKind::Dimension(dim) = &mut now.kind {
            dim.held = None;
        }
        return Some((
            Transaction::new("Unlock dimension").replace(h.clone(), now),
            format!(
                "Unlocked: {} no longer holds its spacing to {}.",
                name_of(model, d.owner),
                d.to
            ),
        ));
    }
    let other = d.other?;
    let How::Spacing { .. } = d.how else {
        return None;
    };
    let ElementKind::View(v) = &model.get(view)?.kind else {
        return None;
    };
    // levels are locked in a section or elevation, the rest in plans
    let level = matches!(model.get(d.owner)?.kind, ElementKind::Level(_));
    if level != matches!(v.kind, ViewKind::Section { .. }) {
        return None;
    }
    // a dimension already placed between two levels here holds itself
    // rather than gaining a twin (A5-2)
    if level {
        let placed = model.iter().find(|e| match &e.kind {
            ElementKind::Dimension(dim) => {
                dim.view == view
                    && dim_levels(model, e.id).is_some_and(|[p, q]| {
                        (p.0 == d.owner && q.0 == other) || (p.0 == other && q.0 == d.owner)
                    })
            }
            _ => false,
        });
        if let Some(e) = placed {
            let mut now = e.clone();
            if let ElementKind::Dimension(dim) = &mut now.kind {
                dim.held = Some(d.value);
            }
            return Some((
                Transaction::new("Lock dimension").replace(e.clone(), now),
                format!(
                    "Locked: {} keeps {} from {}. Moving either moves the other.",
                    name_of(model, d.owner),
                    crate::units::len(d.value),
                    d.to
                ),
            ));
        }
    }
    let a = creabim_geom::annotate::attach(model, v, Some(d.owner), d.a);
    let b = creabim_geom::annotate::attach(model, v, Some(other), d.b);
    let mut dim = Dimension::linear(view, a, b, 0.0);
    dim.held = Some(d.value);
    let e = Element::new("Dimension", ElementKind::Dimension(dim));
    Some((
        Transaction::new("Lock dimension").insert(e),
        format!(
            "Locked: {} keeps {} from {}. Moving either moves the other.",
            name_of(model, d.owner),
            crate::units::len(d.value),
            d.to
        ),
    ))
}

/// The blue of temporary dimensions, for the theme.
fn blue(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(120, 170, 245)
    } else {
        Color32::from_rgb(30, 100, 200)
    }
}

/// A length's number alone, in the project's units.
fn bare(metres: f64) -> String {
    let s = crate::units::len(metres);
    let unit = format!(" {}", crate::units::symbol());
    s.strip_suffix(&unit).map(str::to_string).unwrap_or(s)
}

/// Where a dimension's value and padlock are drawn on screen.
struct Spot {
    value: Rect,
    lock: Option<Rect>,
}

fn layout(
    painter: &Painter,
    a: Pos2,
    b: Pos2,
    text: &str,
    lock: bool,
) -> (Spot, std::sync::Arc<egui::Galley>) {
    let g = painter.layout_no_wrap(
        text.to_string(),
        egui::FontId::proportional(12.0),
        Color32::WHITE,
    );
    let mid = a + (b - a) / 2.0;
    let size = g.size() + Vec2::new(12.0, 6.0);
    let lock_w = if lock { 18.0 } else { 0.0 };
    let whole = Rect::from_center_size(mid, Vec2::new(size.x + lock_w, size.y));
    let value = Rect::from_min_size(whole.min, size);
    let lock = lock.then(|| {
        Rect::from_min_size(
            Pos2::new(value.right() + 2.0, whole.top()),
            Vec2::new(16.0, size.y),
        )
    });
    (Spot { value, lock }, g)
}

/// A small padlock, closed or open.
/// The mark by a dimension that holds its spacing: a small padlock,
/// `size` pixels high, centred on `c`. `current` (the dimension or one of
/// its ends selected) draws it strong and filled in the temporary
/// dimensions' blue; otherwise it is a faint grey outline, a ghost that
/// stays out of the way of the drawing.
pub fn held_mark(painter: &Painter, c: Pos2, size: f32, current: bool) {
    let k = size / 14.0 * if current { 1.0 } else { 0.85 };
    let colour = if current {
        Color32::from_rgb(40, 110, 220)
    } else {
        Color32::from_rgba_unmultiplied(110, 116, 128, 90)
    };
    let body = Rect::from_center_size(c + Vec2::new(0.0, 2.0 * k), Vec2::new(9.0 * k, 7.0 * k));
    let top = c.y - 5.5 * k;
    let pts = vec![
        Pos2::new(body.left() + 1.8 * k, body.top()),
        Pos2::new(body.left() + 1.8 * k, top + 2.0 * k),
        Pos2::new(c.x - 1.0 * k, top),
        Pos2::new(c.x + 1.0 * k, top),
        Pos2::new(body.right() - 1.8 * k, top + 2.0 * k),
        Pos2::new(body.right() - 1.8 * k, body.top()),
    ];
    let width = if current { 1.3 * k.max(0.8) } else { 1.0 };
    painter.add(egui::Shape::line(pts, Stroke::new(width, colour)));
    if current {
        painter.rect_filled(body, 1.5 * k, colour);
    } else {
        painter.rect_stroke(
            body,
            1.5 * k,
            Stroke::new(1.0, colour),
            egui::StrokeKind::Inside,
        );
    }
}

fn draw_lock(painter: &Painter, r: Rect, closed: bool, color: Color32, ground: Color32) {
    let c = r.center();
    let body = Rect::from_center_size(c + Vec2::new(0.0, 2.5), Vec2::new(10.0, 7.5));
    let shackle_x = if closed { 0.0 } else { 3.0 };
    let top = c.y - 6.0;
    let pts = [
        Pos2::new(body.left() + 2.0 + shackle_x, body.top()),
        Pos2::new(body.left() + 2.0 + shackle_x, top + 2.0),
        Pos2::new(c.x - 1.0 + shackle_x, top),
        Pos2::new(c.x + 1.0 + shackle_x, top),
        Pos2::new(body.right() - 2.0 + shackle_x, top + 2.0),
        Pos2::new(
            body.right() - 2.0 + shackle_x,
            body.top() - if closed { 0.0 } else { 2.5 },
        ),
    ];
    painter.rect_filled(r, 4.0, ground);
    painter.add(egui::Shape::line(pts.to_vec(), Stroke::new(1.4, color)));
    if closed {
        painter.rect_filled(body, 1.5, color);
    } else {
        painter.rect_stroke(body, 1.5, Stroke::new(1.4, color), egui::StrokeKind::Inside);
    }
}

/// Draw `id`'s temporary dimensions in a plan and take clicks and typing on
/// them. `quiet` while something else is going on (a drag, a modal edit):
/// they then stay hidden. Some(result) when this frame's input was theirs,
/// so the pane does nothing else with it.
#[allow(clippy::too_many_arguments)]
pub fn interact(
    ui: &egui::Ui,
    doc: &mut creabim_core::Document,
    view: ElementId,
    level: ElementId,
    xf: &crate::planview::Xf,
    painter: &Painter,
    response: &egui::Response,
    temp: &mut TempState,
    selection: &crate::selection::Selection,
    status: &mut Option<String>,
    hint: &mut Option<&'static str>,
    quiet: bool,
    wref: WallRef,
    foot: Option<(DVec2, DVec2)>,
) -> Option<Option<String>> {
    let id = match (selection.active, selection.others.is_empty()) {
        (Some(id), true) if !quiet => id,
        _ => {
            if temp.editing.is_some_and(|e| e.0 == view) {
                temp.editing = None;
                temp.typed.clear();
            }
            return None;
        }
    };
    let near = xf.to_world(painter.clip_rect().center());
    let mut dims = find(doc.model(), level, id, near, wref, foot);
    if dims.is_empty() {
        dims.extend(dim_level_temp(doc.model(), id, temp.click.unwrap_or(near)));
    }
    let editing = temp
        .editing
        .filter(|(v, o, i)| *v == view && *o == id && *i < dims.len());
    if temp.editing.is_some_and(|e| e.0 == view) && editing.is_none() {
        temp.editing = None;
        temp.typed.clear();
    }
    if dims.is_empty() {
        return None;
    }
    let pal = crate::theme::Palette::of(ui.ctx());
    let ink = blue(pal.dark);
    let look = crate::handles::PillLook::of(&pal);
    let pointer = ui
        .input(|i| i.pointer.hover_pos())
        .filter(|p| response.rect.contains(*p));
    let mut clicked: Option<(usize, bool)> = None;
    // grips win over values (A4-7): where a value box would come within
    // 12 px of a grip, the dimension line steps further out
    let grips: Vec<Pos2> = crate::handles::handles_in(doc.model(), id, None, false)
        .iter()
        .map(|h| xf.to_screen(h.at.truncate()))
        .collect();
    for (i, d) in dims.iter().enumerate() {
        let (mut a, mut b) = (xf.to_screen(d.a), xf.to_screen(d.b));
        let out = {
            let from = a;
            let n = Vec2::new(-(b - a).y, (b - a).x).normalized();
            // away from the element's middle
            let mid =
                grips.iter().fold(Vec2::ZERO, |s, g| s + g.to_vec2()) / grips.len().max(1) as f32;
            if (from.to_vec2() - mid).dot(n) < 0.0 {
                -n
            } else {
                n
            }
        };
        let text_w = bare(d.value).len() as f32 * 7.0 + 30.0;
        for _ in 0..4 {
            let mid = a + (b - a) / 2.0;
            let boxed = Rect::from_center_size(mid, Vec2::new(text_w, 20.0)).expand(12.0);
            if !grips.iter().any(|g| boxed.contains(*g)) {
                break;
            }
            a += out * 20.0;
            b += out * 20.0;
        }
        let (a, b) = (a, b);
        let along = (b - a).normalized();
        let tick = Vec2::new(-along.y, along.x) * 5.0;
        let stroke = Stroke::new(1.2, ink);
        painter.line_segment([a, b], stroke);
        for p in [a, b] {
            painter.line_segment([p - tick - along * 3.0, p + tick + along * 3.0], stroke);
        }
        let on = editing.is_some_and(|e| e.2 == i);
        let text = if on && temp.typed.typing() {
            format!("{}|", temp.typed.fields[0])
        } else if on {
            "|".to_string()
        } else {
            bare(d.value)
        };
        let lockable = d.held.is_some()
            || (d.other.is_some()
                && matches!(d.how, How::Spacing { .. })
                && holds::place_of(doc.model(), d.owner).is_some()
                && d.other
                    .and_then(|o| holds::place_of(doc.model(), o))
                    .is_some());
        let (spot, galley) = layout(painter, a, b, &text, lockable);
        let over_value = pointer.is_some_and(|p| spot.value.expand(2.0).contains(p));
        let over_lock =
            pointer.is_some_and(|p| spot.lock.is_some_and(|l| l.expand(2.0).contains(p)));
        let ground = if on { look.ground } else { pal.raise };
        painter.rect(
            spot.value,
            5.0,
            ground,
            Stroke::new(if on || over_value { 1.6 } else { 1.0 }, ink),
            egui::StrokeKind::Inside,
        );
        let color = if on { look.number } else { ink };
        painter.galley_with_override_text_color(
            spot.value.min + Vec2::new(6.0, 3.0),
            galley,
            color,
        );
        if let Some(l) = spot.lock {
            draw_lock(
                painter,
                l,
                d.held.is_some(),
                if over_lock { look.number } else { ink },
                pal.raise,
            );
        }
        if over_value {
            *hint = Some("Click the value and type a new one: the element moves to make it true");
        } else if over_lock {
            *hint = Some(if d.held.is_some() {
                "Click the padlock to let the spacing go"
            } else {
                "Click the padlock to keep this spacing: moving either then moves the other"
            });
        }
        if response.clicked() && (over_value || over_lock) {
            clicked = Some((i, over_lock));
        }
    }
    if let Some((i, lock)) = clicked {
        ui.ctx().request_repaint();
        if lock {
            if let Some((tx, msg)) = padlock(doc.model(), view, &dims[i]) {
                if let Err(e) = doc.commit(tx) {
                    return Some(Some(e.to_string()));
                }
                *status = Some(msg);
            }
        } else {
            temp.editing = Some((view, id, i));
            temp.typed.clear();
            *status = Some(format!(
                "Type the distance to {}, then Enter. Tab: the next value; Esc: stop.",
                dims[i].to
            ));
        }
        return Some(None);
    }
    // typing into the value being edited
    if let Some((_, _, i)) = editing {
        if !ui.ctx().egui_wants_keyboard_input() {
            let typing = temp.typed.typing();
            // Backspace and Delete stay here, not deleting the element; Esc
            // with nothing typed stops
            let stop = ui.input_mut(|inp| {
                if !typing {
                    inp.consume_key(egui::Modifiers::NONE, egui::Key::Backspace);
                }
                inp.consume_key(egui::Modifiers::NONE, egui::Key::Delete);
                !typing && inp.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
            });
            if stop {
                temp.editing = None;
                temp.typed.clear();
                *status = None;
                return Some(None);
            }
            let keys = temp.typed.keys(ui.ctx());
            if keys != crate::tracker::Keys::None {
                ui.ctx().request_repaint();
            }
            if temp.typed.active != 0 {
                // Tab: on to the next value
                temp.typed.clear();
                temp.editing = Some((view, id, (i + 1) % dims.len()));
                return Some(None);
            }
            if keys == crate::tracker::Keys::Enter {
                let typed = temp.typed.length();
                temp.typed.clear();
                temp.editing = None;
                let Some(v) = typed else {
                    *status = Some("That doesn't read as a length.".into());
                    return Some(None);
                };
                match set(doc.model(), &dims[i], v) {
                    Ok((tx, msg)) => {
                        if let Err(e) = doc.commit(tx) {
                            return Some(Some(e.to_string()));
                        }
                        *status = Some(msg);
                    }
                    Err(m) => *status = Some(m),
                }
                return Some(None);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::{Document, Grid, Level, Wall};

    fn setup() -> (Document, ElementId, [ElementId; 3], ElementId) {
        crate::units::set(creabim_core::Units::metric());
        let level = Element::new(
            "Level 0",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 2.7,
            }),
        );
        let lid = level.id;
        let grid = |l: &str, x: f64| {
            Element::new(
                l,
                ElementKind::Grid(Grid {
                    label: l.into(),
                    start: DVec2::new(x, -2.0),
                    end: DVec2::new(x, 8.0),
                    group: None,
                }),
            )
        };
        let (a, b, c) = (grid("A", 0.0), grid("B", 3.4), grid("C", 7.0));
        let ids = [a.id, b.id, c.id];
        let wall = Element::new(
            "W01",
            ElementKind::Wall(Wall::new(
                lid,
                DVec2::new(3.4, 0.0),
                DVec2::new(3.4, 6.0),
                0.2,
                2.7,
            )),
        );
        let wid = wall.id;
        let plan = Element::new("Plan", ElementKind::View(creabim_core::View::plan(lid)));
        let mut doc = Document::new();
        doc.commit(
            Transaction::new("set up")
                .insert(level)
                .insert(plan)
                .insert(a)
                .insert(b)
                .insert(c)
                .insert(wall),
        )
        .unwrap();
        (doc, lid, ids, wid)
    }

    fn x(doc: &Document, id: ElementId) -> f64 {
        match &doc.model().get(id).unwrap().kind {
            ElementKind::Grid(g) => g.start.x,
            ElementKind::Wall(w) => w.start.x,
            _ => unreachable!(),
        }
    }

    #[test]
    fn a_wall_shows_its_length_and_typing_it_lengthens_the_wall() {
        let m = creabim_core::sample::sample_house();
        let (id, level, len) = m
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::Wall(w) => Some((e.id, w.level, w.length())),
                _ => None,
            })
            .unwrap();
        let dims = find(&m, level, id, DVec2::new(1e3, 1e3), WallRef::CoreFace, None);
        let d = dims
            .iter()
            .find(|d| matches!(d.how, How::Length { .. }))
            .expect("a wall's own length");
        assert!((d.value - len).abs() < 1e-9);
        // no gap to anything more than 10 m away
        assert!(dims
            .iter()
            .all(|d| d.value <= 10.0 || matches!(d.how, How::Length { .. })));
        let (tx, _) = set(&m, d, len + 0.5).unwrap();
        let mut doc = Document::from_model(m);
        doc.commit(tx).unwrap();
        let ElementKind::Wall(w) = &doc.model().get(id).unwrap().kind else {
            panic!()
        };
        assert!((w.length() - (len + 0.5)).abs() < 1e-6, "{}", w.length());
    }

    #[test]
    fn typing_a_grid_spacing_moves_the_grid_and_the_walls_on_it() {
        let (mut doc, lid, [a, b, c], wall) = setup();
        let dims = find(
            doc.model(),
            lid,
            b,
            DVec2::new(3.0, 3.0),
            WallRef::CoreFace,
            None,
        );
        assert_eq!(dims.len(), 2);
        let to_a = dims.iter().find(|d| d.other == Some(a)).unwrap();
        assert!((to_a.value - 3.4).abs() < 1e-9);
        let (tx, msg) = set(doc.model(), to_a, 3.6).unwrap();
        doc.commit(tx).unwrap();
        assert!((x(&doc, b) - 3.6).abs() < 1e-9, "{msg}");
        assert!((x(&doc, wall) - 3.6).abs() < 1e-9);
        assert!((x(&doc, a)).abs() < 1e-9 && (x(&doc, c) - 7.0).abs() < 1e-9);
    }

    #[test]
    fn a_locked_spacing_holds_when_grid_a_moves() {
        let (mut doc, lid, [a, b, _], _) = setup();
        let view = doc
            .model()
            .iter()
            .find(|e| matches!(e.kind, ElementKind::View(_)))
            .unwrap()
            .id;
        let dims = find(
            doc.model(),
            lid,
            b,
            DVec2::new(3.0, 3.0),
            WallRef::CoreFace,
            None,
        );
        let to_a = dims.iter().find(|d| d.other == Some(a)).unwrap();
        let (tx, _) = padlock(doc.model(), view, to_a).unwrap();
        doc.commit(tx).unwrap();
        // grid A moves 0.3 left: B keeps 3400 from it
        let dims_a = find(
            doc.model(),
            lid,
            a,
            DVec2::new(3.0, 3.0),
            WallRef::CoreFace,
            None,
        );
        let a_to_b = dims_a.iter().find(|d| d.other == Some(b)).unwrap();
        assert!(a_to_b.held.is_some());
        let before = doc.model().get(a).unwrap().clone();
        let mut after = before.clone();
        if let ElementKind::Grid(g) = &mut after.kind {
            g.start.x -= 0.3;
            g.end.x -= 0.3;
        }
        doc.commit(Transaction::new("Move A").replace(before, after))
            .unwrap();
        assert!((x(&doc, b) - 3.1).abs() < 1e-9);
        // and typing the held spacing anew keeps it held at the new value
        let dims = find(
            doc.model(),
            lid,
            b,
            DVec2::new(3.0, 3.0),
            WallRef::CoreFace,
            None,
        );
        let to_a = dims.iter().find(|d| d.other == Some(a)).unwrap();
        let (tx, _) = set(doc.model(), to_a, 3.0).unwrap();
        doc.commit(tx).unwrap();
        assert!((x(&doc, b) - 2.7).abs() < 1e-9);
        assert!((x(&doc, a) + 0.3).abs() < 1e-9);
    }

    #[test]
    fn a_door_slides_to_the_gap_typed_from_the_wall_end_or_its_neighbour() {
        let (mut doc, lid, _, wall) = setup();
        let door = |offset: f64| {
            Element::new(
                "D01",
                ElementKind::Opening(creabim_core::Opening {
                    host: wall,
                    kind: creabim_core::OpeningKind::Door,
                    offset,
                    sill: 0.0,
                    width: 0.9,
                    height: 2.1,
                    material: None,
                    frame: Default::default(),
                    flip_hand: false,
                    flip_facing: false,
                    setback: None,
                    reveal: None,
                    division: None,
                    door: None,
                }),
            )
        };
        let (d1, d2) = (door(1.0), door(4.0));
        let (i1, i2) = (d1.id, d2.id);
        doc.commit(Transaction::new("doors").insert(d1).insert(d2))
            .unwrap();
        let dims = find(doc.model(), lid, i1, DVec2::ZERO, WallRef::CoreFace, None);
        // to the wall's start (1000) and to the next door (4000 − 1900)
        let start = dims
            .iter()
            .find(|d| d.how == How::Along { start_side: true })
            .unwrap();
        let end = dims
            .iter()
            .find(|d| d.how == How::Along { start_side: false })
            .unwrap();
        assert!((start.value - 1.0).abs() < 1e-9 && start.other.is_none());
        assert!((end.value - 2.1).abs() < 1e-9 && end.other == Some(i2));
        let (tx, _) = set(doc.model(), end, 1.5).unwrap();
        doc.commit(tx).unwrap();
        let ElementKind::Opening(o) = &doc.model().get(i1).unwrap().kind else {
            unreachable!()
        };
        assert!((o.offset - 1.6).abs() < 1e-9);
        // past the wall's end is refused
        let dims = find(doc.model(), lid, i1, DVec2::ZERO, WallRef::CoreFace, None);
        let start = dims
            .iter()
            .find(|d| d.how == How::Along { start_side: true })
            .unwrap();
        assert!(set(doc.model(), start, 9.0).is_err());
    }
}

#[cfg(test)]
mod level_tests {
    use super::*;
    use creabim_core::{Dimension, Document, HostPoint, Level};

    fn levels() -> (Document, [ElementId; 3]) {
        crate::units::set(creabim_core::Units::metric());
        let mk = |n: &str, z: f64| {
            Element::new(
                n,
                ElementKind::Level(Level {
                    elevation: z,
                    height: 3.0,
                }),
            )
        };
        let (a, b, c) = (mk("Ground", 0.0), mk("First", 3.0), mk("Roof", 6.0));
        let ids = [a.id, b.id, c.id];
        let mut doc = Document::new();
        doc.commit(Transaction::new("levels").insert(a).insert(b).insert(c))
            .unwrap();
        (doc, ids)
    }

    #[test]
    fn a_level_measures_to_the_levels_below_and_above_and_typing_moves_it() {
        let (mut doc, [g, f, r]) = levels();
        let dims = find(
            doc.model(),
            ElementId::new(),
            f,
            DVec2::new(5.0, 0.0),
            WallRef::CoreFace,
            None,
        );
        assert_eq!(dims.len(), 2);
        assert_eq!(dims[0].other, Some(g));
        assert_eq!(dims[1].other, Some(r));
        assert!((dims[0].value - 3.0).abs() < 1e-9 && (dims[1].value - 3.0).abs() < 1e-9);
        // 2.5 m below the roof: the first floor goes up half a metre
        let (tx, _) = set(doc.model(), &dims[1], 2.5).unwrap();
        doc.commit(tx).unwrap();
        assert!((doc.model().level(f).unwrap().elevation - 3.5).abs() < 1e-9);
        assert!((doc.model().level(r).unwrap().elevation - 6.0).abs() < 1e-9);
        assert!((doc.model().level(g).unwrap().elevation).abs() < 1e-9);
    }

    #[test]
    fn a_dimension_between_two_levels_moves_the_level_nearer_the_click() {
        let (mut doc, [g, f, _]) = levels();
        let v = Element::new(
            "Section A",
            ElementKind::View(creabim_core::View::section(
                DVec2::ZERO,
                DVec2::new(10.0, 0.0),
                5.0,
                true,
            )),
        );
        let view = v.id;
        doc.commit(Transaction::new("view").insert(v)).unwrap();
        let on = |h: ElementId, x: f64| HostPoint {
            host: Some(h),
            local: DVec2::new(x, 0.0),
            from_end: false,
            body: None,
        };
        let dim = Element::new(
            "Dimension",
            ElementKind::Dimension(Dimension::linear(view, on(g, 2.0), on(f, 2.0), 0.5)),
        );
        let id = dim.id;
        doc.commit(Transaction::new("dim").insert(dim)).unwrap();
        // clicked near its top end: the first floor moves, the ground stays
        let d = dim_level_temp(doc.model(), id, DVec2::new(2.0, 2.8)).unwrap();
        assert_eq!(d.owner, f);
        let (tx, _) = set(doc.model(), &d, 2.7).unwrap();
        doc.commit(tx).unwrap();
        assert!((doc.model().level(f).unwrap().elevation - 2.7).abs() < 1e-9);
        assert!(doc.model().level(g).unwrap().elevation.abs() < 1e-9);
        // clicked near the bottom: the ground moves instead
        let d = dim_level_temp(doc.model(), id, DVec2::new(2.0, 0.1)).unwrap();
        assert_eq!(d.owner, g);
    }

    #[test]
    fn a_locked_level_spacing_carries_the_level_above() {
        let (mut doc, [g, f, r]) = levels();
        let v = Element::new(
            "Section A",
            ElementKind::View(creabim_core::View::section(
                DVec2::ZERO,
                DVec2::new(10.0, 0.0),
                5.0,
                true,
            )),
        );
        let view = v.id;
        doc.commit(Transaction::new("view").insert(v)).unwrap();
        // lock First to Roof, then type First 3.4 m above the ground
        let dims = find(
            doc.model(),
            view,
            f,
            DVec2::new(5.0, 0.0),
            WallRef::CoreFace,
            None,
        );
        let (tx, _) = padlock(doc.model(), view, &dims[1]).unwrap();
        doc.commit(tx).unwrap();
        let dims = find(
            doc.model(),
            view,
            f,
            DVec2::new(5.0, 0.0),
            WallRef::CoreFace,
            None,
        );
        assert_eq!(dims[0].other, Some(g));
        let (tx, _) = set(doc.model(), &dims[0], 3.4).unwrap();
        doc.commit(tx).unwrap();
        assert!((doc.model().level(f).unwrap().elevation - 3.4).abs() < 1e-9);
        assert!(
            (doc.model().level(r).unwrap().elevation - 6.4).abs() < 1e-9,
            "the roof keeps 3 m above"
        );
    }
}

#[cfg(test)]
mod core_tests {
    use super::*;
    use creabim_core::{Document, Level, Wall};

    fn level() -> Element {
        Element::new(
            "Ground",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        )
    }

    fn wall(level: ElementId, a: (f64, f64), b: (f64, f64), t: f64) -> Element {
        Element::new(
            "W",
            ElementKind::Wall(Wall::new(
                level,
                DVec2::new(a.0, a.1),
                DVec2::new(b.0, b.1),
                t,
                2.7,
            )),
        )
    }

    #[test]
    fn parallel_walls_measure_face_to_face_of_their_cores() {
        let l = level();
        let lid = l.id;
        // centrelines 4 m apart, 300 and 200 thick, no layers: all core
        let (a, b) = (
            wall(lid, (0.0, 0.0), (0.0, 5.0), 0.3),
            wall(lid, (4.0, 0.0), (4.0, 5.0), 0.2),
        );
        let (ia, ib) = (a.id, b.id);
        let mut doc = Document::new();
        doc.commit(Transaction::new("set up").insert(l).insert(a).insert(b))
            .unwrap();
        let dims = find(doc.model(), lid, ia, DVec2::ZERO, WallRef::CoreFace, None);
        let to_b = dims.iter().find(|d| d.other == Some(ib)).unwrap();
        assert!((to_b.value - 3.75).abs() < 1e-9, "{}", to_b.value);
        // the centre lines as before, when asked
        let dims = find(doc.model(), lid, ia, DVec2::ZERO, WallRef::Centre, None);
        let to_b = dims.iter().find(|d| d.other == Some(ib)).unwrap();
        assert!((to_b.value - 4.0).abs() < 1e-9);
        // typing the core gap moves the wall by the difference
        let dims = find(doc.model(), lid, ib, DVec2::ZERO, WallRef::CoreFace, None);
        let to_a = dims.iter().find(|d| d.other == Some(ia)).unwrap();
        let (tx, _) = set(doc.model(), to_a, 3.5).unwrap();
        doc.commit(tx).unwrap();
        let ElementKind::Wall(w) = &doc.model().get(ib).unwrap().kind else {
            unreachable!()
        };
        assert!((w.start.x - 3.75).abs() < 1e-9, "{}", w.start.x);
    }

    #[test]
    fn a_held_core_gap_holds_between_the_faces() {
        let l = level();
        let lid = l.id;
        let plan = Element::new("Plan", ElementKind::View(creabim_core::View::plan(lid)));
        let view = plan.id;
        let (a, b) = (
            wall(lid, (0.0, 0.0), (0.0, 5.0), 0.3),
            wall(lid, (4.0, 0.0), (4.0, 5.0), 0.2),
        );
        let (ia, ib) = (a.id, b.id);
        let mut doc = Document::new();
        doc.commit(
            Transaction::new("set up")
                .insert(l)
                .insert(plan)
                .insert(a)
                .insert(b),
        )
        .unwrap();
        let dims = find(doc.model(), lid, ib, DVec2::ZERO, WallRef::CoreFace, None);
        let to_a = dims.iter().find(|d| d.other == Some(ia)).unwrap();
        let (tx, _) = padlock(doc.model(), view, to_a).unwrap();
        doc.commit(tx).unwrap();
        // wall A moves 0.5 right: B follows and keeps 3750 between faces
        let before = doc.model().get(ia).unwrap().clone();
        let mut after = before.clone();
        if let ElementKind::Wall(w) = &mut after.kind {
            w.start.x += 0.5;
            w.end.x += 0.5;
        }
        doc.commit(Transaction::new("Move A").replace(before, after))
            .unwrap();
        let ElementKind::Wall(w) = &doc.model().get(ib).unwrap().kind else {
            unreachable!()
        };
        assert!((w.start.x - 4.5).abs() < 1e-9, "{}", w.start.x);
    }

    #[test]
    fn a_door_measures_to_the_core_face_of_the_wall_it_meets() {
        let l = level();
        let lid = l.id;
        // host along x; a 300 wall meets it at x = 0 (an L) and one at x = 6
        let host = wall(lid, (0.0, 0.0), (6.0, 0.0), 0.3);
        let left = wall(lid, (0.0, 0.0), (0.0, 4.0), 0.3);
        let right = wall(lid, (6.0, 0.0), (6.0, 4.0), 0.3);
        let (hid, lid_w) = (host.id, left.id);
        let door = Element::new(
            "D01",
            ElementKind::Opening(creabim_core::Opening {
                host: hid,
                kind: creabim_core::OpeningKind::Door,
                offset: 1.0,
                sill: 0.0,
                width: 0.9,
                height: 2.1,
                material: None,
                frame: Default::default(),
                flip_hand: false,
                flip_facing: false,
                setback: None,
                reveal: None,
                division: None,
                door: None,
            }),
        );
        let did = door.id;
        let mut doc = Document::new();
        doc.commit(
            Transaction::new("set up")
                .insert(l)
                .insert(host)
                .insert(left)
                .insert(right)
                .insert(door),
        )
        .unwrap();
        let dims = find(doc.model(), lid, did, DVec2::ZERO, WallRef::CoreFace, None);
        let start = dims
            .iter()
            .find(|d| d.how == How::Along { start_side: true })
            .unwrap();
        // from the inner face of the left wall (x = 0.15) to the jamb at 1.0
        assert!((start.value - 0.85).abs() < 1e-9, "{}", start.value);
        assert_eq!(start.other, Some(lid_w));
    }

    #[test]
    fn a_column_measures_from_its_sides_to_the_wall_cores() {
        let l = level();
        let lid = l.id;
        let w = wall(lid, (0.0, 0.0), (0.0, 5.0), 0.3);
        let wid = w.id;
        let col = Element::new(
            "C01",
            ElementKind::Column(creabim_core::Column::new(
                lid,
                DVec2::new(2.0, 2.0),
                creabim_core::Profile::Rectangle {
                    width: 0.4,
                    depth: 0.4,
                },
            )),
        );
        let cid = col.id;
        let mut doc = Document::new();
        doc.commit(Transaction::new("set up").insert(l).insert(w).insert(col))
            .unwrap();
        let dims = find(doc.model(), lid, cid, DVec2::ZERO, WallRef::CoreFace, None);
        let to_w = dims.iter().find(|d| d.other == Some(wid)).unwrap();
        // from the column's left side (1.8) to the wall's face (0.15)
        assert!((to_w.value - 1.65).abs() < 1e-9, "{}", to_w.value);
        let (tx, _) = set(doc.model(), to_w, 1.0).unwrap();
        doc.commit(tx).unwrap();
        let ElementKind::Column(c) = &doc.model().get(cid).unwrap().kind else {
            unreachable!()
        };
        assert!((c.position.x - 1.35).abs() < 1e-9, "{}", c.position.x);
    }

    #[test]
    fn plaster_and_insulation_are_left_out_of_the_gap() {
        use creabim_core::{Assembly, Layer, LayerFunction};
        let l = level();
        let lid = l.id;
        // 20 plaster | 300 masonry | 150 insulation | 10 render = 480;
        // exterior on the right of start → end
        let asm = Element::new(
            "Ext",
            ElementKind::Assembly(Assembly {
                usage: Default::default(),
                layers: vec![
                    Layer::new(LayerFunction::Finish, None, 0.01),
                    Layer::new(LayerFunction::Insulation, None, 0.15),
                    Layer::new(LayerFunction::Structure, None, 0.3),
                    Layer::new(LayerFunction::Finish, None, 0.02),
                ],
                description: String::new(),
                wrap: Default::default(),
            }),
        );
        let mut a = wall(lid, (0.0, 0.0), (0.0, 5.0), 0.48);
        if let ElementKind::Wall(w) = &mut a.kind {
            w.assembly = Some(asm.id);
        }
        let b = wall(lid, (4.0, 0.0), (4.0, 5.0), 0.2);
        let (ia, ib) = (a.id, b.id);
        let mut doc = Document::new();
        doc.commit(
            Transaction::new("set up")
                .insert(l)
                .insert(asm)
                .insert(a)
                .insert(b),
        )
        .unwrap();
        let core = walls::ref_offsets(
            doc.model(),
            match &doc.model().get(ia).unwrap().kind {
                ElementKind::Wall(w) => w,
                _ => unreachable!(),
            },
            WallRef::CoreFace,
        );
        let dims = find(doc.model(), lid, ia, DVec2::ZERO, WallRef::CoreFace, None);
        let to_b = dims.iter().find(|d| d.other == Some(ib)).unwrap();
        // the wall runs up y, so its exterior (render, insulation) faces
        // B at +x: from the masonry's face (x = 0.08) to B's face (3.9)
        assert!(
            (to_b.value - 3.82).abs() < 1e-9,
            "{} {:?}",
            to_b.value,
            core
        );
        let dims = find(doc.model(), lid, ia, DVec2::ZERO, WallRef::Face, None);
        let to_b2 = dims.iter().find(|d| d.other == Some(ib)).unwrap();
        assert!((to_b.value - to_b2.value - 0.16).abs() < 1e-9);
    }
}
