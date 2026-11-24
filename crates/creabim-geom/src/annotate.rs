//! Dimensions and tags: where they sit in a view's drawing, what they
//! read, and the linework they are drawn with. Both follow their hosts,
//! because their positions are worked out from the model every time.

use crate::annotation::{end_marks, measure, runs_past, Annotation, Ends, Shape, TextLook};
use crate::plan::PlanItem;
use crate::section::{AxoFrame, SectionFrame};
use creabim_core::{
    DimensionKind, Element, ElementId, ElementKind, HostPoint, Model, Note, OpeningKind, RoomField,
    TagShape, Terminator, View, ViewKind, Wall,
};
use glam::{DAffine3, DMat3, DVec2, DVec3};

/// Maps world points into a view's drawing coordinates.
pub enum Projector {
    Plan,
    Section(SectionFrame),
    Axo(AxoFrame),
}

impl Projector {
    pub fn of(model: &Model, view: &View) -> Option<Self> {
        Some(match view.kind {
            ViewKind::Plan { .. } => Projector::Plan,
            ViewKind::Section {
                start, end, depth, ..
            } => Projector::Section(SectionFrame::new(start, end, depth)),
            ViewKind::Axonometric { azimuth, altitude } => {
                Projector::Axo(AxoFrame::new(model, azimuth, altitude))
            }
            ViewKind::Schedule { .. } => return None,
        })
    }

    pub fn project(&self, p: DVec3) -> DVec2 {
        match self {
            Projector::Plan => p.truncate(),
            Projector::Section(f) => f.project(p).truncate(),
            Projector::Axo(f) => f.project(p).truncate(),
        }
    }
}

/// A wall's frame: a point `u` along it and `s` to its left.
fn on_wall(w: &Wall, u: f64, s: f64) -> DVec2 {
    w.start + w.direction() * u + w.direction().perp() * s
}

fn host_wall(model: &Model, id: ElementId) -> Option<(&Wall, f64)> {
    match &model.get(id)?.kind {
        ElementKind::Wall(w) => Some((w, 0.0)),
        ElementKind::Opening(o) => match &model.get(o.host)?.kind {
            ElementKind::Wall(w) => Some((w, o.offset)),
            _ => None,
        },
        _ => None,
    }
}

/// Where a dimension point sits in a view's drawing, if it can be drawn there.
pub fn resolve(model: &Model, view: &View, p: &HostPoint) -> Option<DVec2> {
    let Some(host) = p.host else {
        return Some(p.local);
    };
    match (&model.get(host)?.kind, &view.kind) {
        (ElementKind::Level(l), ViewKind::Section { .. }) => {
            Some(DVec2::new(p.local.x, l.elevation + p.local.y))
        }
        // on a stair's nosing: it follows the steps
        (ElementKind::Stair(s), ViewKind::Plan { .. }) if p.step.is_some() => {
            let k = p.step?;
            let lines = s.nosing_lines(s.rise(model));
            let n = lines.len() as u32;
            let i = if p.from_end {
                n.checked_sub(1 + k)?
            } else {
                k.min(n.checked_sub(1)?)
            };
            let [a, b] = lines.get(i as usize)?;
            Some(a.lerp(*b, p.local.x))
        }
        (ElementKind::Wall(_) | ElementKind::Opening(_), ViewKind::Plan { .. }) => {
            let (w, base) = host_wall(model, host)?;
            // a door's or window's far jamb is kept from that jamb, so a
            // dimension to it follows when it is made wider
            let u = match (&model.get(host)?.kind, p.from_end) {
                (ElementKind::Opening(o), true) => base + o.width + p.local.x,
                (_, true) => w.length() + p.local.x,
                (_, false) => base + p.local.x,
            };
            Some(on_wall(w, u, p.local.y))
        }
        // a plan's spot level on an element without its own axes (a roof,
        // the terrain, a stair) stays where it was put
        (_, ViewKind::Plan { .. }) if p.body.is_none() => Some(p.local),
        _ => {
            let body = p.body?;
            Some(
                Projector::of(model, view)?
                    .project(host_frame(model, host)?.transform_point3(body)),
            )
        }
    }
}

/// An element's own 3D axes in the model, for dimension points kept on it:
/// a mass's origin, turn and size; a slab's first corner and edge; a grid,
/// wall or guide's start and direction; a sketch's plane.
pub fn host_frame(model: &Model, id: ElementId) -> Option<DAffine3> {
    let elevation = |level: Option<ElementId>| {
        level
            .and_then(|l| model.level(l))
            .map_or(0.0, |l| l.elevation)
    };
    // axes along `x` (made level), across it and up, at `o`
    let along = |o: DVec3, x: DVec3| {
        let x = DVec3::new(x.x, x.y, 0.0)
            .try_normalize()
            .unwrap_or(DVec3::X);
        DAffine3::from_mat3_translation(DMat3::from_cols(x, DVec3::Z.cross(x), DVec3::Z), o)
    };
    Some(match &model.get(id)?.kind {
        ElementKind::Wall(w) => along(
            w.start.extend(elevation(Some(w.level)) + w.base_offset),
            (w.end - w.start).extend(0.0),
        ),
        ElementKind::Opening(o) => {
            let ElementKind::Wall(w) = &model.get(o.host)?.kind else {
                return None;
            };
            let z = elevation(Some(w.level)) + w.base_offset + o.sill;
            along(
                on_wall(w, o.offset, 0.0).extend(z),
                (w.end - w.start).extend(0.0),
            )
        }
        // a beam's top along its axis, so a spot level on it follows it
        // up or down (a ring beam with its wall's top, George 2026-11-24)
        ElementKind::Beam(b) => along(b.start.extend(b.top(model)), (b.end - b.start).extend(0.0)),
        ElementKind::Slab(s) => {
            let a = *s.boundary.first()?;
            let b = s.boundary.get(1).copied().unwrap_or(a + DVec2::X);
            // with its top's offset, so a dimension follows it up or down
            along(
                a.extend(elevation(Some(s.level)) + s.top_offset),
                (b - a).extend(0.0),
            )
        }
        // a footing's top along its line (a pad's along its turn), so a
        // dimension to it in a section follows it when it is moved or set
        // deeper
        ElementKind::Footing(f) => match f.kind {
            creabim_core::footing::FootingKind::Strip => {
                let (a, b) = f.line(model);
                along(a.extend(f.top(model)), (b - a).extend(0.0))
            }
            _ => {
                let (c, turn) = f.place(model);
                along(
                    c.extend(f.top(model)),
                    DVec2::from_angle(turn.to_radians()).extend(0.0),
                )
            }
        },
        // a ceiling's underside along its outline's first edge, so a note
        // to it in a section follows it (George 2026-11-23)
        ElementKind::Ceiling(c) => {
            let o = c.outline(model)?;
            let a = *o.first()?;
            let b = o.get(1).copied().unwrap_or(a + DVec2::X);
            along(a.extend(c.span(model).0), (b - a).extend(0.0))
        }
        ElementKind::Grid(g) => along(g.start.extend(0.0), (g.end - g.start).extend(0.0)),
        // a column or an object (furniture, a fixture, equipment): its
        // position and turn, so dimensions to it follow it (PilonDesigner's
        // dimensions bound to every element)
        ElementKind::Column(c) => DAffine3::from_mat3_translation(
            DMat3::from_rotation_z(c.rotation.to_radians()),
            c.position.extend(elevation(Some(c.level)) + c.base_offset),
        ),
        ElementKind::Object(o) => DAffine3::from_mat3_translation(
            DMat3::from_rotation_z(o.rotation.to_radians()),
            o.position.extend(elevation(Some(o.level))),
        ),
        ElementKind::Guide(g) => along(g.a + DVec3::Z * elevation(g.level), g.b - g.a),
        // a pipe's axis: a dimension to it follows the pipe
        ElementKind::Pipe(p) => along(
            p.start + DVec3::Z * elevation(Some(p.level)),
            p.end - p.start,
        ),
        ElementKind::FreeMesh(m) => DAffine3::from_mat3_translation(
            DMat3::from_rotation_z(m.rotation.to_radians()) * m.scale,
            m.origin + DVec3::Z * elevation(m.level),
        ),
        ElementKind::Sketch(sk) => {
            let p = &sk.plane;
            DAffine3::from_mat3_translation(
                DMat3::from_cols(p.x, p.y, p.x.cross(p.y)),
                p.origin + DVec3::Z * elevation(sk.level),
            )
        }
        _ => return None,
    })
}

/// A drawing point of `view` back in the model: on the level in a plan, on
/// the cut plane in a section.
fn unproject(model: &Model, view: &View, at: DVec2) -> Option<DVec3> {
    Some(match view.kind {
        ViewKind::Plan { level, .. } => at.extend(model.level(level).map_or(0.0, |l| l.elevation)),
        ViewKind::Section {
            start, end, depth, ..
        } => {
            let f = SectionFrame::new(start, end, depth);
            (f.origin + f.along * at.x).extend(at.y)
        }
        _ => return None,
    })
}

/// A dimension point attached to `host`, expressed in the host's frame so
/// it follows the host. `at` is a drawing point in `view`.
pub fn attach(model: &Model, view: &View, host: Option<ElementId>, at: DVec2) -> HostPoint {
    let attached = host.and_then(|h| match (&model.get(h)?.kind, &view.kind) {
        (ElementKind::Level(l), ViewKind::Section { .. }) => Some(HostPoint {
            host: Some(h),
            local: DVec2::new(at.x, at.y - l.elevation),
            from_end: false,
            body: None,
            step: None,
        }),
        (ElementKind::Wall(_) | ElementKind::Opening(_), ViewKind::Plan { .. }) => {
            let (w, base) = host_wall(model, h)?;
            let d = w.direction();
            let u = (at - w.start).dot(d);
            let s = (at - w.start).dot(d.perp());
            // points nearer a wall's end are kept relative to the end, so
            // they follow it when the wall is lengthened; points nearer a
            // door's or window's far jamb, from that jamb
            let (from_end, end) = match &model.get(h)?.kind {
                ElementKind::Opening(o) => {
                    let far = u - base > o.width / 2.0;
                    (far, base + o.width)
                }
                _ => (u > w.length() / 2.0, w.length()),
            };
            Some(HostPoint {
                host: Some(h),
                local: DVec2::new(if from_end { u - end } else { u - base }, s),
                from_end,
                body: None,
                step: None,
            })
        }
        (ElementKind::Level(_), _) => None,
        (ElementKind::Stair(s), ViewKind::Plan { .. }) => stair_point(model, h, s, at),
        _ => {
            let frame = host_frame(model, h)?;
            let world = unproject(model, view, at)?;
            Some(HostPoint {
                host: Some(h),
                local: at,
                from_end: false,
                body: Some(frame.inverse().transform_point3(world)),
                step: None,
            })
        }
    });
    attached.unwrap_or(HostPoint::free(at))
}

/// A plan point on one of a stair's nosings, kept on that nosing (from the
/// last when it is in the upper half), so a dimension to it follows the
/// steps (George 2026-11-24); none off every nosing.
fn stair_point(
    model: &Model,
    id: ElementId,
    s: &creabim_core::Stair,
    at: DVec2,
) -> Option<HostPoint> {
    let lines = s.nosing_lines(s.rise(model));
    let n = lines.len();
    let (i, t, d) = lines
        .iter()
        .enumerate()
        .map(|(i, [a, b])| {
            let ab = *b - *a;
            let t = ((at - *a).dot(ab) / ab.length_squared().max(1e-12)).clamp(0.0, 1.0);
            (i, t, a.lerp(*b, t).distance(at))
        })
        .min_by(|x, y| x.2.total_cmp(&y.2))?;
    if d > 2e-3 {
        return None;
    }
    let from_end = n > 1 && i >= n / 2 + n % 2;
    Some(HostPoint {
        host: Some(id),
        local: DVec2::new(t, 0.0),
        from_end,
        body: None,
        step: Some(if from_end {
            (n - 1 - i) as u32
        } else {
            i as u32
        }),
    })
}

/// A dimension between two of a stair's nosing points set to `value`
/// (George 2026-11-24): across one nosing it sets the flight's width;
/// between two nosings, the going, which the norm then keeps its risers
/// to. None when the points are not on its nosings.
fn stair_driven(
    model: &Model,
    id: ElementId,
    ha: &HostPoint,
    hb: &HostPoint,
    now: f64,
    value: f64,
) -> Option<Result<creabim_core::Transaction, String>> {
    let e = model.get(id)?;
    let ElementKind::Stair(s) = &e.kind else {
        return None;
    };
    let n = s.nosing_lines(s.rise(model)).len() as u32;
    let index = |h: &HostPoint| {
        let k = h.step?;
        Some(if h.from_end { n.checked_sub(1 + k)? } else { k })
    };
    let (ia, ib) = (index(ha)?, index(hb)?);
    let mut st = s.clone();
    if ia == ib {
        let across = (ha.local.x - hb.local.x).abs();
        if across < 1e-3 {
            return Some(Err("Both ends are at one point of the stair.".into()));
        }
        st.width = (value / across).max(0.3);
        creabim_core::staircheck::keep_to_norm(
            model,
            &mut st,
            creabim_core::staircheck::Fit::Width,
        );
    } else {
        if now < 1e-6 {
            return None;
        }
        st.tread = (s.tread * value / now).clamp(0.1, 1.0);
        creabim_core::staircheck::keep_to_norm(
            model,
            &mut st,
            creabim_core::staircheck::Fit::Going,
        );
    }
    let mut after = e.clone();
    after.kind = ElementKind::Stair(st);
    Some(Ok(
        creabim_core::Transaction::new("Set dimension").replace(e.clone(), after)
    ))
}

/// The centre of an element's body, in world coordinates.
pub fn centre(model: &Model, id: ElementId) -> Option<DVec3> {
    let e = model.get(id)?;
    let elevation = |level: ElementId| model.level(level).map_or(0.0, |l| l.elevation);
    Some(match &e.kind {
        ElementKind::Wall(w) => on_wall(w, w.length() / 2.0, 0.0)
            .extend(elevation(w.level) + w.base_offset + w.height / 2.0),
        ElementKind::Opening(o) => {
            let ElementKind::Wall(w) = &model.get(o.host)?.kind else {
                return None;
            };
            on_wall(w, o.offset + o.width / 2.0, 0.0)
                .extend(elevation(w.level) + w.base_offset + o.sill + o.height / 2.0)
        }
        ElementKind::Slab(s) => {
            let c = s.boundary.iter().copied().sum::<DVec2>() / s.boundary.len().max(1) as f64;
            c.extend(s.top(elevation(s.level)) - s.thickness / 2.0)
        }
        ElementKind::FreeMesh(m) => {
            let mesh = m.evaluated().mesh;
            let n = mesh.vertices.len().max(1) as f64;
            let c = mesh.vertices.iter().copied().sum::<DVec3>() / n;
            c + m.origin + DVec3::Z * m.level.map_or(0.0, elevation)
        }
        // furniture and fixtures: the middle of their parts
        ElementKind::Object(_) => {
            let mut pts = creabim_core::objects::model_parts(model, e)
                .into_iter()
                .flat_map(|(m, _)| m.vertices);
            let first = pts.next()?;
            let (lo, hi) = pts.fold((first, first), |(lo, hi), v| (lo.min(v), hi.max(v)));
            (lo + hi) / 2.0
        }
        // a room: its point on its floor
        ElementKind::Room(r) => r.at.extend(elevation(r.level)),
        // a ceiling: the middle of its outline, on its underside
        ElementKind::Ceiling(c) => {
            let o = c.outline(model)?;
            let middle = o.iter().copied().sum::<DVec2>() / o.len().max(1) as f64;
            middle.extend(c.span(model).0)
        }
        // columns, beams, roofs, stairs and railings: the middle of their shape
        k if creabim_core::members::is_member(k) => {
            let (mesh, _) = crate::members::member_mesh(model, e)?;
            let (lo, hi) = mesh.bounds()?;
            ((lo + hi) / 2.0).as_dvec3()
        }
        _ => return None,
    })
}

/// What a material tag on `id` reads: its layers, one line each with
/// their thickness, outside to inside, or its one material's name.
pub fn material_text(model: &Model, id: ElementId) -> String {
    let Some(e) = model.get(id) else {
        return String::new();
    };
    let tag = |m: f64| creabim_core::write_number(model, creabim_core::NumberKind::Tags, None, m);
    if let Some((_, a)) = creabim_core::assembly_of(model, e) {
        if !a.layers.is_empty() {
            return a
                .layers
                .iter()
                .map(|l| {
                    let name = l
                        .material
                        .and_then(|m| model.get(m))
                        .map_or("(no material)".to_string(), |m| m.name.clone());
                    format!("{} {name}", tag(l.thickness))
                })
                .collect::<Vec<_>>()
                .join("\n");
        }
    }
    creabim_core::material_of(model, e).map_or("(no material)".into(), |(m, _)| m.name.clone())
}

/// What a tag on `id` reads, and whether it sits in a circle (doors) or a box.
/// A door's hand as STN 01 3420 names it: "L" when its hinges are on the
/// left seen from the side it opens to, else "P" (pravé); none for
/// anything but a door with a leaf.
pub fn door_hand(model: &Model, id: ElementId) -> Option<&'static str> {
    let ElementKind::Opening(o) = &model.get(id)?.kind else {
        return None;
    };
    if o.kind != OpeningKind::Door {
        return None;
    }
    let ElementKind::Wall(w) = &model.get(o.host)?.kind else {
        return None;
    };
    let u = w.direction();
    // the side it swings to, and the jamb its hinges are on
    let swing = if o.faces_left(w) { u.perp() } else { -u.perp() };
    let hinge = if o.flip_hand { u } else { -u };
    // seen from the swing side, looking at the wall, right is swing.perp()
    Some(if hinge.dot(swing.perp()) < 0.0 {
        "L"
    } else {
        "P"
    })
}

/// The way a door's or window's tag may stand off its wall in a plan (a
/// unit vector square to the wall, in the view); none for other tags.
pub fn tag_normal(model: &Model, view: &View, tag: &creabim_core::Tag) -> Option<DVec2> {
    if !matches!(view.kind, ViewKind::Plan { .. }) {
        return None;
    }
    let ElementKind::Opening(o) = &model.get(tag.host)?.kind else {
        return None;
    };
    if !matches!(o.kind, OpeningKind::Door | OpeningKind::Window) {
        return None;
    }
    let ElementKind::Wall(w) = &model.get(o.host)?.kind else {
        return None;
    };
    let project = Projector::of(model, view)?;
    let n = w.direction().perp().extend(0.0);
    let d = project.project(n) - project.project(DVec3::ZERO);
    (d.length() > 0.5).then(|| d.normalize())
}

/// A door's widest leaf, and how far from its wall's middle the arcs its
/// leaves sweep reach, as the plan draws them; none without a leaf.
fn leaf_reach(
    model: &Model,
    o: &creabim_core::Opening,
    w: &creabim_core::Wall,
) -> Option<(f64, f64)> {
    let asm = model
        .get(o.host)
        .and_then(|h| creabim_core::assembly_of(model, h))
        .map(|x| x.1);
    let left = o.faces_left(w);
    crate::opening::leaf_boxes(o, w, asm)
        .iter()
        .map(|b| {
            let r = b.hi.x - b.lo.x;
            let s = if left { b.hi.y } else { -b.lo.y };
            (r, s + r)
        })
        .reduce(|a, b| (a.0.max(b.0), a.1.max(b.1)))
}

pub fn tag_text(model: &Model, id: ElementId) -> (String, bool) {
    let Some(e) = model.get(id) else {
        return (String::new(), false);
    };
    let material = creabim_core::material_of(model, e).map(|(m, _)| m.name.clone());
    let units = creabim_core::units::number_format(model, creabim_core::NumberKind::Tags, None)
        .units(&model.units());
    let tag = |m: f64| creabim_core::write_number(model, creabim_core::NumberKind::Tags, None, m);
    let with_material = |t: f64| match &material {
        Some(m) => format!("{m} {}", tag(t)),
        None => tag(t),
    };
    // a lintel or ring beam: its position, P1 or V1, in a circle, as the
    // office's výkres tvaru marks them (George 2026-11-24)
    if matches!(e.kind, ElementKind::Beam(_)) {
        if let Some(m) = crate::structural::beam_mark(model, id) {
            return (m, true);
        }
    }
    // a mark, typed or numbered by the project's rules, is what it reads
    if !matches!(e.kind, ElementKind::Object(_)) {
        if let Some(m) = creabim_core::marks::mark_of(model, id) {
            // doors and windows in a circle, as the office marks them
            let round = matches!(&e.kind, ElementKind::Opening(o)
                if matches!(o.kind, OpeningKind::Door | OpeningKind::Window));
            // a door's mark (STN 01 3420): left or right over its number
            if let Some(hand) = door_hand(model, id) {
                return (format!("{hand}\n{m}"), true);
            }
            return (m, round);
        }
    }
    if let Some(hand) = door_hand(model, id) {
        return (format!("{hand}\n{}", e.name), true);
    }
    match &e.kind {
        ElementKind::Opening(o) => (e.name.clone(), o.kind == OpeningKind::Door),
        ElementKind::Wall(w) => (with_material(w.thickness), false),
        ElementKind::Slab(s) => (with_material(s.thickness), false),
        ElementKind::Column(c) => (c.profile.label_in(&units), false),
        ElementKind::Beam(b) => (format!("{} × {}", tag(b.width), tag(b.depth)), false),
        ElementKind::Roof(r) => (
            format!("{} {:.0}°", with_material(r.thickness), r.pitch),
            false,
        ),
        // furniture and fixtures: their type's mark, in a circle like doors
        ElementKind::Object(_) => match creabim_core::marks::object_mark(model, e) {
            Some(m) => (m, true),
            None => (e.name.clone(), false),
        },
        ElementKind::Footing(f) => (
            match f.kind {
                creabim_core::FootingKind::Strip => {
                    format!("{} × {}", tag(f.width), tag(f.thickness))
                }
                creabim_core::FootingKind::Pad => format!(
                    "{} × {} × {}",
                    tag(f.width),
                    tag(f.length),
                    tag(f.thickness)
                ),
                creabim_core::FootingKind::Pile => {
                    format!("Ø {} L {}", tag(f.width), tag(f.thickness))
                }
            },
            false,
        ),
        ElementKind::Room(_) => (room_tag_text(model, id, &[]), false),
        // a ceiling: its height above the floor, as reflected ceiling plans write it
        ElementKind::Ceiling(c) => (format!("CLG +{}", tag(c.height)), false),
        _ => (e.name.clone(), false),
    }
}

/// What a room's tag reads: each of `fields` on its own line (the room's
/// name, number and area when there are none), its name in capitals and
/// its number and area side by side; fields the room has no value for are
/// left out.
pub fn room_tag_text(model: &Model, id: ElementId, fields: &[RoomField]) -> String {
    let Some(e) = model.get(id) else {
        return String::new();
    };
    if !matches!(e.kind, ElementKind::Room(_)) {
        return e.name.clone();
    }
    let fields = if fields.is_empty() {
        &RoomField::DEFAULT[..]
    } else {
        fields
    };
    let mut lines: Vec<String> = Vec::new();
    let mut joined = false;
    for (i, f) in fields.iter().enumerate() {
        let value = f.value(model, id);
        if value.is_empty() {
            continue;
        }
        let value = if *f == RoomField::Name {
            value.to_uppercase()
        } else {
            value
        };
        // the area goes beside the number written just before it
        if *f == RoomField::Area && joined && i > 0 && fields[i - 1] == RoomField::Number {
            if let Some(last) = lines.last_mut() {
                last.push_str("  ");
                last.push_str(&value);
                continue;
            }
        }
        joined = *f == RoomField::Number;
        lines.push(value);
    }
    lines.join("\n")
}

/// A room's tag as drawing text: its lines stacked and centred on `at`.
pub fn room_tag_item(id: ElementId, text: String, at: DVec2, look: TextLook, mm: f64) -> PlanItem {
    // its lines centred by their real widths (MU29)
    let mut look = look;
    if look.align == creabim_core::TextAlign::Usual {
        look.align = creabim_core::TextAlign::Centre;
    }
    let lines: Vec<&str> = text.split('\n').collect();
    let width = lines.iter().map(|l| look.width(l)).fold(0.0, f64::max) * mm;
    let step = look.size * 1.5 * mm;
    let start = at + DVec2::new(-width / 2.0, step * (lines.len() - 1) as f64 / 2.0);
    PlanItem::Annotation {
        id,
        item: Box::new(Annotation {
            shape: Shape::Note {
                at: start,
                leader: None,
                elbow: None,
                angle: 0.0,
            },
            text,
            look,
            mm,
        }),
    }
}

/// A room's tag laid out as `style` says, centred on `at`: each line its
/// own drawing text. Stacked writes the name in capitals and semibold, the
/// area below it and the number small and last; Inline is
/// [`room_tag_text`] in one block. A tag moved out of its room (`room` is
/// its outline in the drawing) gets a leader back to `centre`.
#[allow(clippy::too_many_arguments)]
pub fn room_tag_items(
    model: &Model,
    id: ElementId,
    room: ElementId,
    fields: &[RoomField],
    at: DVec2,
    centre: DVec2,
    outline: Option<&[DVec2]>,
    look: TextLook,
    mm: f64,
    style: creabim_core::styles::RoomTagStyle,
) -> Vec<PlanItem> {
    use creabim_core::styles::RoomTagStyle;
    let outside = outline.is_some_and(|o| o.len() > 2 && !creabim_core::polygon::contains(o, at));
    let mut out = match style {
        RoomTagStyle::Inline => {
            vec![room_tag_item(
                id,
                room_tag_text(model, room, fields),
                at,
                look,
                mm,
            )]
        }
        RoomTagStyle::Stacked => {
            let order: &[RoomField] = if fields.is_empty() {
                &[RoomField::Name, RoomField::Area, RoomField::Number]
            } else {
                fields
            };
            let lines: Vec<(String, TextLook)> = order
                .iter()
                .filter_map(|f| {
                    let v = f.value(model, room);
                    if v.is_empty() {
                        return None;
                    }
                    let mut l = look;
                    Some(match f {
                        RoomField::Name => {
                            l.bold = true;
                            (v.to_uppercase(), l)
                        }
                        RoomField::Number => {
                            l.size *= 0.72;
                            (v, l)
                        }
                        _ => {
                            l.size *= 0.9;
                            (v, l)
                        }
                    })
                })
                .collect();
            let steps: Vec<f64> = lines.iter().map(|(_, l)| l.size * 1.45 * mm).collect();
            let total: f64 = steps.iter().sum::<f64>() - steps.last().copied().unwrap_or(0.0);
            let mut y = at.y + total / 2.0;
            let mut items = vec![];
            // left or right aligned: every line against the block's edge
            let block = lines
                .iter()
                .map(|(t, l)| l.width(t) * mm)
                .fold(0.0, f64::max);
            for ((text, l), step) in lines.into_iter().zip(steps) {
                let w = l.width(&text) * mm;
                let x = match look.align {
                    creabim_core::TextAlign::Left => at.x - block / 2.0 + w / 2.0,
                    creabim_core::TextAlign::Right => at.x + block / 2.0 - w / 2.0,
                    _ => at.x,
                };
                items.push(room_tag_item(id, text, DVec2::new(x, y), l, mm));
                y -= step;
            }
            items
        }
        RoomTagStyle::NumberBadge | RoomTagStyle::Boxed => {
            let value = |f: RoomField| {
                let shown = fields.is_empty() || fields.contains(&f);
                Some(f.value(model, room)).filter(|v| shown && !v.is_empty())
            };
            let (name, number, area) = (
                value(RoomField::Name),
                value(RoomField::Number),
                value(RoomField::Area),
            );
            let mut bold = look;
            bold.bold = true;
            let mut small = look;
            small.size *= 0.72;
            let mut mid = look;
            mid.size *= 0.9;
            let pen = crate::plan::Pen::Symbol;
            let rect = |lo: DVec2, hi: DVec2| PlanItem::Polygon {
                id,
                pen,
                points: vec![lo, DVec2::new(hi.x, lo.y), hi, DVec2::new(lo.x, hi.y)],
            };
            let mut items = vec![];
            if style == RoomTagStyle::NumberBadge {
                // the number in a small outline on top, the name, the area
                let step = look.size * 1.45 * mm;
                let rows = [number.is_some(), name.is_some(), area.is_some()]
                    .iter()
                    .filter(|r| **r)
                    .count();
                let mut y = at.y + step * (rows.saturating_sub(1)) as f64 / 2.0;
                if let Some(n) = number {
                    let w = small.width(&n) * mm + 1.6 * mm;
                    let h = small.size * 1.5 * mm;
                    let c = DVec2::new(at.x, y);
                    items.push(PlanItem::Polygon {
                        id,
                        pen,
                        points: (0..20)
                            .map(|i| {
                                // a stadium round the number
                                let t = i as f64 / 20.0 * std::f64::consts::TAU;
                                let (cx, cy) = (t.cos(), t.sin());
                                let half = (w - h).max(0.0) / 2.0;
                                c + DVec2::new(cx * h / 2.0 + half * cx.signum(), cy * h / 2.0)
                            })
                            .collect(),
                    });
                    items.push(room_tag_item(id, n, c, small, mm));
                    y -= step;
                }
                if let Some(n) = name {
                    items.push(room_tag_item(
                        id,
                        n.to_uppercase(),
                        DVec2::new(at.x, y),
                        bold,
                        mm,
                    ));
                    y -= step;
                }
                if let Some(a) = area {
                    items.push(room_tag_item(id, a, DVec2::new(at.x, y), mid, mm));
                }
            } else {
                // a box: the name, and a footer with the number and area
                let pad = 1.2 * mm;
                let top = name
                    .as_ref()
                    .map_or(0.0, |n| bold.width(&n.to_uppercase()) * mm);
                // text drawn on screen runs a little wider than the
                // estimate, so the footer leaves room and a divider
                let foot_gap = 2.0 * pad;
                let nw = number.as_ref().map_or(0.0, |n| small.width(n) * mm * 1.15);
                let aw = area.as_ref().map_or(0.0, |a| mid.width(a) * mm * 1.15);
                let foot = nw + aw + foot_gap;
                let w = top.max(foot) + 2.0 * pad;
                let h_top = look.size * 1.6 * mm;
                let h_foot = look.size * 1.4 * mm;
                let lo = DVec2::new(at.x - w / 2.0, at.y - (h_top + h_foot) / 2.0);
                let hi = DVec2::new(at.x + w / 2.0, at.y + (h_top + h_foot) / 2.0);
                let split = hi.y - h_top;
                items.push(rect(lo, hi));
                items.push(PlanItem::Line {
                    id,
                    pen,
                    a: DVec2::new(lo.x, split),
                    b: DVec2::new(hi.x, split),
                });
                if let Some(n) = name {
                    items.push(room_tag_item(
                        id,
                        n.to_uppercase(),
                        DVec2::new(at.x, split + h_top / 2.0),
                        bold,
                        mm,
                    ));
                }
                let fy = lo.y + h_foot / 2.0;
                if number.is_some() && area.is_some() {
                    let x = lo.x + 2.0 * pad + nw;
                    items.push(PlanItem::Line {
                        id,
                        pen,
                        a: DVec2::new(x, lo.y),
                        b: DVec2::new(x, split),
                    });
                }
                if let Some(n) = number {
                    items.push(room_tag_item(
                        id,
                        n,
                        DVec2::new(lo.x + pad + nw / 2.0, fy),
                        small,
                        mm,
                    ));
                }
                if let Some(a) = area {
                    items.push(room_tag_item(
                        id,
                        a,
                        DVec2::new(hi.x - pad - aw / 2.0, fy),
                        mid,
                        mm,
                    ));
                }
            }
            items
        }
    };
    if outside {
        // a leader from the tag back to its room
        let gap = look.size * mm;
        let d = centre - at;
        if d.length() > gap * 2.0 {
            let from = at + d.normalize() * gap * 1.5;
            out.push(PlanItem::Line {
                id,
                pen: crate::plan::Pen::Symbol,
                a: from,
                b: centre,
            });
            out.push(PlanItem::Polygon {
                id,
                pen: crate::plan::Pen::Symbol,
                points: (0..12)
                    .map(|i| {
                        let t = i as f64 / 12.0 * std::f64::consts::TAU;
                        centre + DVec2::new(t.cos(), t.sin()) * 0.6 * mm
                    })
                    .collect(),
            });
        }
    }
    out
}

/// A linear dimension's items, one per stretch: a chain (PilonDesigner's
/// reťazová kóta) is one dimension line through all its points, each
/// stretch between neighbours along it measured. A stretch across a whole
/// door or window reads its width over the line and its height (and a
/// window's sill) under it, STN 01 3420.
/// One witness line of a linear dimension (see [`witness_ends`]).
#[derive(Clone, Copy, Debug)]
pub struct WitnessEnd {
    /// The point's place in [`creabim_core::Dimension::points`].
    pub index: usize,
    /// The point measured, in the view's drawing.
    pub point: DVec2,
    /// Where its witness line meets the dimension line.
    pub foot: DVec2,
    /// Where its witness line starts: dragged, or at the point.
    pub start: DVec2,
}

impl WitnessEnd {
    /// The way from the dimension line out to the point (along the
    /// witness line); square to the line when the point is on it.
    pub fn out(&self, normal: DVec2) -> DVec2 {
        let d = self.point - self.foot;
        if d.length() > 1e-9 {
            d.normalize()
        } else {
            normal
        }
    }

    /// How far it starts from the dimension line.
    pub fn reach(&self) -> f64 {
        self.start.distance(self.foot)
    }
}

/// The witness lines of a linear dimension, each with where it starts:
/// its own reach where it was dragged, else the measured point. With the
/// dimension line's normal (to the left of its run).
pub fn witness_ends(
    model: &Model,
    view: &View,
    d: &creabim_core::Dimension,
) -> Option<(Vec<WitnessEnd>, DVec2)> {
    let a = resolve(model, view, &d.a)?;
    let b = resolve(model, view, &d.b)?;
    let (b2, _) = squared(d.axis, a, b)?;
    let u = (b2 - a).normalize_or_zero();
    let n = u.perp();
    let ends = d
        .points()
        .enumerate()
        .filter_map(|(index, p)| {
            let point = resolve(model, view, p)?;
            let foot = a + u * (point - a).dot(u) + n * d.offset;
            let mut w = WitnessEnd {
                index,
                point,
                foot,
                start: point,
            };
            if let Some(r) = d.reach.get(index).copied().flatten() {
                w.start = foot + w.out(-n * d.offset.signum()) * r.max(0.0);
            }
            Some(w)
        })
        .collect();
    Some((ends, n))
}

pub fn linear_items(
    model: &Model,
    view: &View,
    id: ElementId,
    d: &creabim_core::Dimension,
) -> Vec<PlanItem> {
    let (Some(a), Some(b)) = (resolve(model, view, &d.a), resolve(model, view, &d.b)) else {
        return vec![];
    };
    let Some((b2, witness)) = squared(d.axis, a, b) else {
        return vec![];
    };
    let num = |v: f64| {
        spaced(&creabim_core::write_number(
            model,
            creabim_core::NumberKind::Dimensions,
            d.format,
            v,
        ))
    };
    let below_text = |h: f64, sill: Option<f64>| match sill {
        Some(s) => format!("{} ({})", num(h), num(s)),
        None => num(h),
    };
    let look = look_of(model, d.style, d.color);
    // where each witness line starts, by the point's place
    let ends = if d.reach.iter().any(|r| r.is_some()) {
        witness_ends(model, view, d)
            .map(|e| e.0)
            .unwrap_or_default()
    } else {
        vec![]
    };
    let start = |i: usize, q: DVec2| ends.iter().find(|e| e.index == i).map_or(q, |e| e.start);
    let place = |i: usize| {
        let shift = d.texts.get(i).copied().flatten();
        crate::plan::DimPlace {
            shift: shift.unwrap_or_default(),
            manual: shift.is_some(),
            leader: shift.is_some(),
            short: d.short,
            back: d.back || look.back,
        }
    };
    if !d.is_chain() {
        let len = a.distance(b2);
        let below = opening_below(model, d, len)
            .or_else(|| opening_of_span(model, view, a, b))
            .map(|(h, s)| below_text(h, s));
        let witness = if ends.is_empty() {
            witness
        } else {
            Some([start(0, a), start(1, b)])
        };
        return vec![PlanItem::Dimension {
            id,
            a,
            b: b2,
            offset: d.offset,
            text: num(len),
            look,
            witness,
            breaks: vec![],
            below,
            place: place(0),
        }];
    }
    let u = (b2 - a).normalize_or_zero();
    let mut marks: Vec<(f64, DVec2, &HostPoint, usize)> = d
        .points()
        .enumerate()
        .filter_map(|(k, p)| {
            let q = resolve(model, view, p)?;
            Some(((q - a).dot(u), q, p, k))
        })
        .collect();
    marks.sort_by(|x, y| x.0.total_cmp(&y.0));
    marks.dedup_by(|y, x| (y.0 - x.0).abs() < 1e-4);
    marks
        .windows(2)
        .enumerate()
        .map(|(i, w)| {
            let ((sa, pa, ha, ka), (sb, pb, hb, kb)) = (w[0], w[1]);
            let len = sb - sa;
            let below = opening_between(model, ha, hb, len)
                .or_else(|| opening_of_span(model, view, pa, pb))
                .map(|(h, s)| below_text(h, s));
            PlanItem::Dimension {
                id,
                a: a + u * sa,
                b: a + u * sb,
                offset: d.offset,
                text: if d.eq { "EQ".to_string() } else { num(len) },
                look,
                witness: Some([start(ka, pa), start(kb, pb)]),
                breaks: vec![],
                below,
                place: place(i),
            }
        })
        .collect()
}

/// A number with its thousands apart, as the drawings write millimetres:
/// `1 970`. Text that is not a whole number stays as it is.
pub fn spaced(text: &str) -> String {
    if text.len() < 4 || !text.bytes().all(|b| b.is_ascii_digit()) {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + text.len() / 3);
    for (i, c) in text.chars().enumerate() {
        if i > 0 && (text.len() - i) % 3 == 0 {
            out.push('\u{a0}');
        }
        out.push(c);
    }
    out
}

/// Put the text of each stretch too short for it beside the stretch, as
/// PilonDesigner does: a chain's first stretch before its start, its last
/// past its end, one inside past the mark towards the neighbour with room
/// beside its own text, and where neither has room a row higher with a
/// leader. Texts moved by hand stay. `mm` is drawing units per paper mm.
pub fn place_dimension_texts(items: &mut [PlanItem], mm: f64) {
    // what each dimension item needs along its line, and its length
    struct Run {
        k: usize,
        a: DVec2,
        b: DVec2,
        w: f64,
        up: DVec2,
        lm: f64,
        manual: bool,
        below: bool,
    }
    let runs: Vec<Run> = items
        .iter()
        .enumerate()
        .filter_map(|(k, it)| {
            let PlanItem::Dimension {
                a,
                b,
                offset,
                text,
                look,
                witness,
                below,
                place,
                ..
            } = it
            else {
                return None;
            };
            let g = dimension_geometry(*a, *b, *offset, *witness, look, mm);
            let (pa, pb) = g.ends_at;
            let w = below
                .iter()
                .map(|t| look.width(t))
                .fold(look.width(text), f64::max)
                * mm;
            Some(Run {
                k,
                a: pa,
                b: pb,
                w,
                up: (g.text_at - (pa + pb) / 2.0).normalize_or_zero(),
                lm: look.size * mm,
                manual: place.manual,
                below: below.is_some(),
            })
        })
        .collect();
    for (i, r) in runs.iter().enumerate() {
        let len = r.a.distance(r.b);
        if r.manual || len < 1e-9 || len >= r.w + 0.4 * r.lm {
            continue;
        }
        let u = (r.b - r.a) / len;
        // the stretch that carries on along the line from `p`, be it of
        // this dimension or of another: how much room it has beside its
        // own text
        let room = |p: DVec2| {
            runs.iter()
                .enumerate()
                .filter(|(j, o)| {
                    *j != i
                        && (o.a.distance(p) < 1e-6 || o.b.distance(p) < 1e-6)
                        && (o.b - o.a).perp_dot(u).abs() < 1e-6 * o.a.distance(o.b).max(1.0)
                })
                .map(|(_, o)| o.a.distance(o.b) / 2.0 - o.w / 2.0)
                .reduce(f64::min)
        };
        let gap = 0.5 * r.lm;
        // from the middle of the stretch to beside it
        let before = -u * (len / 2.0 + r.w / 2.0 + gap);
        let after = u * (len / 2.0 + r.w / 2.0 + gap);
        let need = r.w + 2.0 * gap;
        let (shift, leader) = match (room(r.a), room(r.b)) {
            // the line ends here: past its end
            (_, None) => (after, false),
            (None, _) => (before, false),
            (Some(left), Some(right)) if left.max(right) >= need => {
                (if left >= right { before } else { after }, false)
            }
            // nowhere beside: a row higher, every other one higher still
            _ => {
                let rows = if r.below { 2.6 } else { 1.5 } + if i % 2 == 1 { 0.0 } else { 1.4 };
                (r.up * r.lm * rows, true)
            }
        };
        if let PlanItem::Dimension { place, .. } = &mut items[r.k] {
            place.shift = shift;
            place.leader = leader;
        }
    }
}

/// A chain without its point `i` (0 is `a`, then the further points, then
/// `b`), keeping its line where it is: an end taken out is replaced by the
/// next point in along the line. None when only two points are left.
pub fn without_point(
    model: &Model,
    view: &View,
    d: &creabim_core::Dimension,
    i: usize,
) -> Option<creabim_core::Dimension> {
    if !d.is_chain() || i > d.mids.len() + 1 {
        return None;
    }
    let (a, b) = (resolve(model, view, &d.a)?, resolve(model, view, &d.b)?);
    let (b2, _) = squared(d.axis, a, b)?;
    let u = (b2 - a).normalize_or_zero();
    let mut out = d.clone();
    let along = |p: &HostPoint| resolve(model, view, p).map_or(0.0, |q| (q - a).dot(u));
    // what is left, in order along the line: its ends are the outermost,
    // so the line keeps its direction and its side
    let mut left: Vec<HostPoint> = d
        .points()
        .enumerate()
        .filter(|(k, _)| *k != i)
        .map(|(_, p)| *p)
        .collect();
    left.sort_by(|x, y| along(x).total_cmp(&along(y)));
    out.b = left.pop()?;
    out.a = left.remove(0);
    out.mids = left;
    // the line keeps its place: offset is measured from `a`
    let q = resolve(model, view, &out.a)?;
    out.offset += (a - q).dot(u.perp());
    // the numbers dragged aside belonged to the old stretches
    out.texts.clear();
    Some(out)
}

/// The dimension `d` with its points kept on the walls they stand on, so
/// that it follows them when they move (PilonDesigner's kóty zviazané s
/// prvkami): a point clicked where lines cross, on a layer's face or on a
/// corner belongs to no element by itself. Of the walls a point is on, it
/// goes to the one whose face the dimension measures to, else to one
/// whose end it is at, else to any. Points on doors and windows stay.
pub fn rehosted(
    model: &Model,
    view: &View,
    d: &creabim_core::Dimension,
) -> creabim_core::Dimension {
    let mut out = d.clone();
    let ViewKind::Plan { level, .. } = view.kind else {
        return out;
    };
    let (Some(a), Some(b)) = (resolve(model, view, &d.a), resolve(model, view, &d.b)) else {
        return out;
    };
    let Some((b2, _)) = squared(d.axis, a, b) else {
        return out;
    };
    let u = (b2 - a).normalize_or_zero();
    const NEAR: f64 = 2e-3;
    for p in out.points_mut() {
        let on_opening = p.host.is_some_and(|h| {
            matches!(
                model.get(h).map(|e| &e.kind),
                Some(ElementKind::Opening(_) | ElementKind::Pipe(_))
            )
        });
        if on_opening {
            continue;
        }
        let Some(q) = resolve(model, view, p) else {
            continue;
        };
        // a point at a door's or window's jamb is the opening's, as
        // PilonDesigner binds it: the dimension goes with the opening
        if let Some(o) = jamb_at(model, level, q, NEAR) {
            *p = attach(model, view, Some(o), q);
            continue;
        }
        // a point on a pipe (its axis or its outline) is the pipe's: the
        // dimension then goes with the pipe and moves it when typed
        let on_pipe = model.iter().find_map(|e| {
            let ElementKind::Pipe(pp) = &e.kind else {
                return None;
            };
            if pp.level != level {
                return None;
            }
            let (a, b) = (pp.start.truncate(), pp.end.truncate());
            let ab = b - a;
            let t = if ab.length_squared() < 1e-12 {
                0.0
            } else {
                ((q - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
            };
            let r = (pp.outer_mm / 2.0 + pp.insulation_mm) / 1000.0;
            (q.distance(a + ab * t) <= r + NEAR).then_some(e.id)
        });
        if let Some(pipe) = on_pipe {
            *p = attach(model, view, Some(pipe), q);
            continue;
        }
        let mut best: Option<(u8, ElementId)> = None;
        for e in model.iter() {
            let ElementKind::Wall(w) = &e.kind else {
                continue;
            };
            if w.level != level || w.is_arc() {
                continue;
            }
            let dir = w.direction();
            let (x, s) = ((q - w.start).dot(dir), (q - w.start).dot(dir.perp()));
            // its body, and where a joined corner carries it on
            let reach = w.thickness;
            if s.abs() > w.thickness / 2.0 + NEAR || x < -reach || x > w.length() + reach {
                continue;
            }
            let at_end = x.abs() < NEAR || (x - w.length()).abs() < NEAR;
            let inside = x > -NEAR && x < w.length() + NEAR;
            let score = if dir.perp().dot(u).abs() > 0.7 {
                3
            } else if at_end {
                2
            } else if inside {
                1
            } else {
                0
            };
            // what it was on already wins a tie
            let better = match best {
                None => true,
                Some((b, _)) => score > b || (score == b && p.host == Some(e.id)),
            };
            if better {
                best = Some((score, e.id));
            }
        }
        if let Some((_, wall)) = best {
            *p = attach(model, view, Some(wall), q);
        }
    }
    out
}

/// The door or window on `level` whose jamb plan point `q` stands at:
/// within its wall's thickness, at one of its sides to `near`.
pub fn jamb_at(model: &Model, level: ElementId, q: DVec2, near: f64) -> Option<ElementId> {
    model.iter().find_map(|e| {
        let ElementKind::Opening(o) = &e.kind else {
            return None;
        };
        let ElementKind::Wall(w) = &model.get(o.host)?.kind else {
            return None;
        };
        if w.level != level || w.is_arc() {
            return None;
        }
        let d = w.direction();
        let (x, s) = ((q - w.start).dot(d), (q - w.start).dot(d.perp()));
        let at_jamb = (x - o.offset).abs() < near || (x - o.offset - o.width).abs() < near;
        (at_jamb && s.abs() <= w.thickness / 2.0 + near).then_some(e.id)
    })
}

/// Every linear dimension on a plan with points at a door's or window's
/// jamb bound to the wall (or to nothing), bound to the opening instead:
/// older drawings, so their dimensions follow the opening when it moves.
pub fn bind_jambs(model: &Model) -> Vec<(Element, Element)> {
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Dimension(d) = &e.kind else {
            continue;
        };
        if d.kind != DimensionKind::Linear {
            continue;
        }
        let Some(ElementKind::View(v)) = model.get(d.view).map(|e| &e.kind) else {
            continue;
        };
        let ViewKind::Plan { level, .. } = v.kind else {
            continue;
        };
        let mut now = d.clone();
        for p in now.points_mut() {
            let on_wall = p.host.is_none_or(|h| {
                matches!(model.get(h).map(|e| &e.kind), Some(ElementKind::Wall(_)))
            });
            if !on_wall {
                continue;
            }
            let Some(q) = resolve(model, v, p) else {
                continue;
            };
            if let Some(o) = jamb_at(model, level, q, 2e-3) {
                *p = attach(model, v, Some(o), q);
            } else if p.host.is_none() {
                // drawn to a stair's nosing before they bound to it
                let on_stair = model.iter().find_map(|e| match &e.kind {
                    ElementKind::Stair(s) if s.level == level => stair_point(model, e.id, s, q),
                    _ => None,
                });
                if let Some(h) = on_stair {
                    *p = h;
                }
            }
        }
        if now != *d {
            let mut after = e.clone();
            after.kind = ElementKind::Dimension(now);
            out.push((e.clone(), after));
        }
    }
    out
}

/// A linear dimension's stretches in order along its line: the two points
/// each measures between and where they stand on the measured line.
pub fn stretches(
    model: &Model,
    view: &View,
    d: &creabim_core::Dimension,
) -> Vec<(HostPoint, HostPoint, DVec2, DVec2)> {
    let (Some(a), Some(b)) = (resolve(model, view, &d.a), resolve(model, view, &d.b)) else {
        return vec![];
    };
    let Some((b2, _)) = squared(d.axis, a, b) else {
        return vec![];
    };
    if !d.is_chain() {
        return vec![(d.a, d.b, a, b2)];
    }
    let u = (b2 - a).normalize_or_zero();
    let mut marks: Vec<(f64, HostPoint)> = d
        .points()
        .filter_map(|p| Some(((resolve(model, view, p)? - a).dot(u), *p)))
        .collect();
    marks.sort_by(|x, y| x.0.total_cmp(&y.0));
    marks.dedup_by(|y, x| (y.0 - x.0).abs() < 1e-4);
    marks
        .windows(2)
        .map(|w| (w[0].1, w[1].1, a + u * w[0].0, a + u * w[1].0))
        .collect()
}

/// The stretches of chain `d` made equal (an EQ dimension): its two ends
/// stay, every point between moves what it stands on to an even share of
/// the whole. One transaction; an error where a point between stands on
/// nothing that can move.
pub fn equalised(
    model: &Model,
    view: &View,
    d: &creabim_core::Dimension,
) -> Result<creabim_core::Transaction, String> {
    let all = stretches(model, view, d);
    let n = all.len();
    if n < 2 {
        return Err("An EQ dimension is a chain of two stretches or more.".into());
    }
    let total: f64 = all.iter().map(|s| s.2.distance(s.3)).sum();
    let each = total / n as f64;
    let mut now = model.clone();
    let mut tx = creabim_core::Transaction::new("Equal stretches");
    for i in 0..n - 1 {
        let s = stretches(&now, view, d);
        let Some(&(_, hb, pa, pb)) = s.get(i) else {
            break;
        };
        if (pa.distance(pb) - each).abs() < 1e-6 {
            continue;
        }
        let prefer: Vec<ElementId> = hb.host.into_iter().collect();
        if prefer.is_empty() {
            return Err("A point between the ends stands on no element: nothing to move.".into());
        }
        let step = driven(&now, view, d, i, each, &prefer)?;
        step.apply(&mut now).map_err(|e| e.to_string())?;
        tx.ops.extend(step.ops);
    }
    Ok(tx)
}

/// Stretch `i` of the dimension `d` made `value` long (PilonDesigner's
/// riadiaca kóta): the element one of its ends is on moves, the one in
/// `prefer` (what is selected) if either is, else the one at its far end.
/// A door or window slides in its wall; a wall measured along itself is
/// made longer or shorter at that end; anything else moves bodily, with
/// what is joined to it.
/// The pipes in one straight line with pipe `id`, joined end to end, on
/// its level and in its system: what moves when it is dimensioned.
pub fn straight_run(model: &Model, id: ElementId) -> Vec<ElementId> {
    let pipes: Vec<(ElementId, &creabim_core::mep::Pipe)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Pipe(p) => Some((e.id, p)),
            _ => None,
        })
        .collect();
    let Some(first) = pipes.iter().find(|(i, _)| *i == id).map(|x| x.1) else {
        return vec![];
    };
    let dir = (first.end - first.start).normalize_or_zero();
    let mut run = vec![id];
    let mut i = 0;
    while i < run.len() {
        let Some(p) = pipes.iter().find(|(x, _)| *x == run[i]).map(|x| x.1) else {
            break;
        };
        for (oid, o) in &pipes {
            if run.contains(oid) || o.level != first.level || o.system != first.system {
                continue;
            }
            let od = (o.end - o.start).normalize_or_zero();
            if od.cross(dir).length() > 1e-3 {
                continue;
            }
            let touches = [p.start, p.end]
                .iter()
                .any(|a| a.distance(o.start) < 0.002 || a.distance(o.end) < 0.002);
            if touches {
                run.push(*oid);
            }
        }
        i += 1;
    }
    run
}

pub fn driven(
    model: &Model,
    view: &View,
    d: &creabim_core::Dimension,
    i: usize,
    value: f64,
    prefer: &[ElementId],
) -> Result<creabim_core::Transaction, String> {
    if !matches!(view.kind, ViewKind::Plan { .. } | ViewKind::Section { .. }) {
        return Err("A dimension moves what it measures in plans and sections only.".into());
    }
    if value <= 0.0 {
        return Err("Type a distance more than zero.".into());
    }
    let all = stretches(model, view, d);
    let (ha, hb, pa, pb) = *all.get(i).ok_or("It is gone.")?;
    let u = (pb - pa).normalize_or_zero();
    let delta = value - pa.distance(pb);
    if delta.abs() < 1e-7 {
        return Err("Already that far.".into());
    }
    if let (Some(id), true) = (ha.host, ha.host == hb.host) {
        if let Some(tx) = stair_driven(model, id, &ha, &hb, pa.distance(pb), value) {
            return tx;
        }
        return Err("Both ends are on the same element: change its size instead.".into());
    }
    let chosen = |h: &HostPoint| h.host.is_some_and(|id| prefer.contains(&id));
    // a pipe dimensioned from the building moves, never the wall
    let is_pipe = |h: &HostPoint| {
        h.host
            .and_then(|id| model.get(id))
            .is_some_and(|e| matches!(e.kind, ElementKind::Pipe(_)))
    };
    let (ha_pipe, hb_pipe) = (is_pipe(&ha), is_pipe(&hb));
    let take_b = if ha_pipe != hb_pipe {
        hb_pipe
    } else {
        chosen(&hb) || (!chosen(&ha) && hb.host.is_some())
    };
    let (p, other, by) = if take_b {
        (hb, ha, u * delta)
    } else if ha.host.is_some() {
        (ha, hb, -u * delta)
    } else {
        return Err("Neither end is on an element: nothing to move.".into());
    };
    let id = p.host.ok_or("It is gone.")?;
    let e = model.get(id).ok_or("It is gone.")?;
    let mut tx = creabim_core::Transaction::new("Set dimension");
    if let ViewKind::Section {
        start, end, depth, ..
    } = view.kind
    {
        return section_driven(
            model,
            e,
            other.host,
            by,
            SectionFrame::new(start, end, depth),
        );
    }
    match &e.kind {
        ElementKind::Opening(o) => {
            let ElementKind::Wall(w) = &model.get(o.host).ok_or("It is not in a wall.")?.kind
            else {
                return Err("It is not in a wall.".into());
            };
            let offset = o.offset + by.dot(w.direction());
            if offset < -1e-9 || offset + o.width > w.length() + 1e-9 {
                return Err(format!("{} would run past the end of its wall.", e.name));
            }
            let mut now = e.clone();
            if let ElementKind::Opening(o) = &mut now.kind {
                o.offset = offset.max(0.0);
            }
            tx = tx.replace(e.clone(), now);
        }
        ElementKind::Wall(w) if w.direction().dot(u).abs() > 0.7 => {
            // measured along the wall: that end of it moves
            let dir = w.direction();
            let end = p.from_end || p.local.x > w.length() / 2.0;
            let mut now = e.clone();
            if let ElementKind::Wall(w) = &mut now.kind {
                if end {
                    w.end += dir * by.dot(dir);
                } else {
                    w.start += dir * by.dot(dir);
                }
                if w.start.distance(w.end) < 0.05 {
                    return Err(format!("{} would be left with no length.", e.name));
                }
            }
            let mut moved = vec![now];
            let follow = creabim_core::joins::follow(model, &mut moved);
            for n in moved {
                tx = tx.replace(e.clone(), n);
            }
            for (o, n) in follow {
                if Some(o.id) != other.host && o != n {
                    tx = tx.replace(o, n);
                }
            }
        }
        ElementKind::Pipe(_) => {
            // the pipe's straight run moves; what joins it stretches to
            // stay joined
            let run = straight_run(model, id);
            let shift = by.extend(0.0);
            let mut moved: Vec<creabim_core::Element> = run
                .iter()
                .filter_map(|r| {
                    let mut n = model.get(*r)?.clone();
                    if let ElementKind::Pipe(p) = &mut n.kind {
                        p.start += shift;
                        p.end += shift;
                    }
                    Some(n)
                })
                .collect();
            let follow = creabim_core::joins::follow(model, &mut moved);
            for n in moved {
                if let Some(o) = model.get(n.id) {
                    tx = tx.replace(o.clone(), n);
                }
            }
            for (o, n) in follow {
                if !run.contains(&o.id) && o != n {
                    tx = tx.replace(o, n);
                }
            }
        }
        _ => {
            let keep: std::collections::HashSet<ElementId> = other.host.into_iter().collect();
            let moved = creabim_core::holds::shift(model, id, by, &keep);
            if moved.is_empty() {
                return Err(format!("{} can not be moved by a dimension.", e.name));
            }
            for (o, n) in moved {
                tx = tx.replace(o, n);
            }
        }
    }
    Ok(tx)
}

/// [`driven`] in a section: `by` in the drawing, x along the section and
/// y up. A height moves the element up or down (a footing deeper, a
/// slab's top, an object, a column's base, a pipe's run); a distance
/// across moves it along the section line.
fn section_driven(
    model: &Model,
    e: &creabim_core::Element,
    other: Option<ElementId>,
    by: DVec2,
    f: SectionFrame,
) -> Result<creabim_core::Transaction, String> {
    let mut tx = creabim_core::Transaction::new("Set dimension");
    let mut now = e.clone();
    if by.y.abs() >= by.x.abs() {
        let dz = by.y;
        match &mut now.kind {
            ElementKind::Footing(x) => x.lift(dz),
            ElementKind::Slab(x) => x.top_offset += dz,
            ElementKind::Object(x) => x.offset += dz,
            ElementKind::Column(x) => x.base_offset += dz,
            ElementKind::Wall(x) => x.base_offset += dz,
            ElementKind::Pipe(_) => {
                let run = straight_run(model, e.id);
                let mut moved: Vec<creabim_core::Element> = run
                    .iter()
                    .filter_map(|r| {
                        let mut n = model.get(*r)?.clone();
                        if let ElementKind::Pipe(p) = &mut n.kind {
                            p.start.z += dz;
                            p.end.z += dz;
                        }
                        Some(n)
                    })
                    .collect();
                let follow = creabim_core::joins::follow(model, &mut moved);
                for n in moved {
                    if let Some(o) = model.get(n.id) {
                        tx = tx.replace(o.clone(), n);
                    }
                }
                for (o, n) in follow {
                    if !run.contains(&o.id) && o != n {
                        tx = tx.replace(o, n);
                    }
                }
                return Ok(tx);
            }
            _ => {
                return Err(format!(
                    "{} can not be moved up or down by a dimension.",
                    e.name
                ))
            }
        }
        return Ok(tx.replace(e.clone(), now));
    }
    let world = f.along * by.x;
    let keep: std::collections::HashSet<ElementId> = other.into_iter().collect();
    let moved = creabim_core::holds::shift(model, e.id, world, &keep);
    if moved.is_empty() {
        return Err(format!("{} can not be moved by a dimension.", e.name));
    }
    for (o, n) in moved {
        tx = tx.replace(o, n);
    }
    Ok(tx)
}

/// The door or window a stretch between two points on its jambs spans.
fn opening_between(
    model: &Model,
    a: &HostPoint,
    b: &HostPoint,
    len: f64,
) -> Option<(f64, Option<f64>)> {
    let h = a.host?;
    if b.host != Some(h) {
        return None;
    }
    let ElementKind::Opening(o) = &model.get(h)?.kind else {
        return None;
    };
    if (len - o.width).abs() > 1e-3 || o.kind == OpeningKind::Hole {
        return None;
    }
    Some((
        o.height,
        (o.kind == OpeningKind::Window).then_some(o.sill_from_floor(model)),
    ))
}

/// The door or window in a plan whose jambs a stretch from `a` to `b`
/// runs between, whatever the points are bound to (a wall's face, the
/// opening, nothing), as PilonDesigner finds it: both ends within the
/// wall's thickness, at its jambs to 15 mm.
pub fn opening_of_span(
    model: &Model,
    view: &View,
    a: DVec2,
    b: DVec2,
) -> Option<(f64, Option<f64>)> {
    let ViewKind::Plan { level, .. } = view.kind else {
        return None;
    };
    model.iter().find_map(|e| {
        let ElementKind::Opening(o) = &e.kind else {
            return None;
        };
        if o.kind == OpeningKind::Hole {
            return None;
        }
        let ElementKind::Wall(w) = &model.get(o.host)?.kind else {
            return None;
        };
        if w.level != level {
            return None;
        }
        let u = (w.end - w.start).normalize_or_zero();
        let reach = w.thickness / 2.0 + 0.06;
        let (sa, sb) = ((a - w.start).dot(u), (b - w.start).dot(u));
        if (a - w.start).perp_dot(u).abs() > reach || (b - w.start).perp_dot(u).abs() > reach {
            return None;
        }
        let (lo, hi) = (sa.min(sb), sa.max(sb));
        ((lo - o.offset).abs() < 0.015 && (hi - o.offset - o.width).abs() < 0.015).then_some((
            o.height,
            (o.kind == OpeningKind::Window).then_some(o.sill_from_floor(model)),
        ))
    })
}

/// A plan dimension from jamb to jamb of one door or window (both ends on
/// it, as long as it is wide): its height, and a window's sill, to write
/// under the line (STN 01 3420).
pub fn opening_below(
    model: &Model,
    d: &creabim_core::Dimension,
    len: f64,
) -> Option<(f64, Option<f64>)> {
    let h = d.a.host?;
    if d.b.host != Some(h) {
        return None;
    }
    let ElementKind::Opening(o) = &model.get(h)?.kind else {
        return None;
    };
    if (len - o.width).abs() > 1e-3 || o.kind == OpeningKind::Hole {
        return None;
    }
    Some((
        o.height,
        (o.kind == OpeningKind::Window).then_some(o.sill_from_floor(model)),
    ))
}

/// A dimension's value as it is written in `units`.
pub fn dimension_text(units: &creabim_core::Units, a: DVec2, b: DVec2) -> String {
    units.dimension(a.distance(b))
}

/// A linear dimension from `a` to `b` measured along `axis`: where `b`
/// lands on the line through `a` that way, and the true points its
/// witness lines start from (none when aligned). None when the points
/// coincide along it, so there is nothing to measure.
pub fn squared(
    axis: creabim_core::DimAxis,
    a: DVec2,
    b: DVec2,
) -> Option<(DVec2, Option<[DVec2; 2]>)> {
    let u = axis.direction(a, b)?;
    if axis.is_aligned() {
        return (a.distance(b) > 1e-6).then_some((b, None));
    }
    let b2 = a + u * (b - a).dot(u);
    (a.distance(b2) > 1e-6).then_some((b2, Some([a, b])))
}

/// How an annotation with `style` and its own `color` looks.
pub fn look_of(model: &Model, style: Option<ElementId>, color: Option<[f32; 3]>) -> TextLook {
    creabim_core::text_style(model, style, color).into()
}

/// A dimension's look: its style's, its colour, and its own background.
fn dim_look(model: &Model, d: &creabim_core::Dimension) -> TextLook {
    let mut look = look_of(model, d.style, d.color);
    look.back |= d.back;
    look
}

/// A dimension about a centre (angle, radius, diameter, arc length) as a
/// drawing item, if its points can be drawn in the view.
fn measured(
    model: &Model,
    view: &View,
    id: ElementId,
    d: &creabim_core::Dimension,
) -> Option<PlanItem> {
    let c = resolve(model, view, d.center.as_ref()?)?;
    let a = resolve(model, view, &d.a)?;
    let b = resolve(model, view, &d.b)?;
    if c.distance(a) < 1e-6 {
        return None;
    }
    let shape = match d.kind {
        DimensionKind::Linear | DimensionKind::Spot | DimensionKind::Slope => return None,
        DimensionKind::Angular => {
            if c.distance(b) < 1e-6 {
                return None;
            }
            Shape::Angle {
                vertex: c,
                a,
                b,
                radius: d.offset,
            }
        }
        DimensionKind::Radius | DimensionKind::Diameter => Shape::Radius {
            center: c,
            at: a,
            extra: d.offset,
            diameter: d.kind == DimensionKind::Diameter,
        },
        DimensionKind::ArcLength => Shape::ArcLength {
            center: c,
            a,
            b,
            offset: d.offset,
        },
    };
    Some(PlanItem::Annotation {
        id,
        item: Box::new(Annotation {
            shape,
            text: measure(
                &creabim_core::units::number_format(
                    model,
                    creabim_core::NumberKind::Dimensions,
                    d.format,
                )
                .units(&model.units()),
                d.kind,
                c,
                a,
                b,
            ),
            look: dim_look(model, d),
            mm: view.scale / 1000.0,
        }),
    })
}

/// A slope dimension as a drawing item, if its line can be drawn in the
/// view.
fn slope(
    model: &Model,
    view: &View,
    id: ElementId,
    d: &creabim_core::Dimension,
) -> Option<PlanItem> {
    let a = resolve(model, view, &d.a)?;
    let b = resolve(model, view, &d.b)?;
    if a.distance(b) < 1e-6 {
        return None;
    }
    let plan = matches!(view.kind, ViewKind::Plan { .. });
    Some(PlanItem::Annotation {
        id,
        item: Box::new(Annotation {
            shape: Shape::Slope {
                a,
                b,
                offset: d.offset,
                plan,
            },
            text: slope_reading(d.pattern.as_deref(), b - a, plan),
            look: dim_look(model, d),
            mm: view.scale / 1000.0,
        }),
    })
}

/// What a slope over the run `d` reads with `pattern`: in a plan, where
/// lines are level, only typed text (a format there reads `? %`).
pub fn slope_reading(pattern: Option<&str>, d: DVec2, plan: bool) -> String {
    let p = pattern.map(str::trim).unwrap_or("%");
    if plan && (p.is_empty() || creabim_core::annotation::SLOPE_FORMATS.contains(&p)) {
        return "? %".into();
    }
    creabim_core::annotation::slope_text(d, Some(p))
}

/// The height a spot level reads at `at`: in a section or elevation the
/// point's own height; in a plan the height of what it was put on (a
/// roof's face, a slab's top, the terrain, a stair's tread) at that very
/// point, or else the plan's level (a level marker).
pub fn spot_height(
    model: &Model,
    view: &View,
    d: &creabim_core::Dimension,
    at: DVec2,
) -> Option<f64> {
    match view.kind {
        ViewKind::Section { .. } => Some(at.y),
        ViewKind::Plan { level, .. } => {
            let elevation = |l: ElementId| model.level(l).map(|l| l.elevation);
            let host = d.a.host.filter(|h| model.get(*h).is_some());
            if let Some(z) = host.and_then(|h| surface_height(model, h, at)) {
                return Some(z);
            }
            match host.and_then(|h| model.get(h)).map(|e| &e.kind) {
                Some(ElementKind::Slab(s)) => Some(s.top(elevation(s.level)?)),
                _ => elevation(level),
            }
        }
        _ => None,
    }
}

/// Whether a spot level in a plan reads the surface of an element of this
/// kind: what is walked or built on, and seen from above also the tops
/// of walls, columns and the like. A floor plan's cut walls, columns,
/// doors and furniture are looked past, to the floor.
pub fn spot_reads(kind: &ElementKind, from_above: bool) -> bool {
    match kind {
        ElementKind::Slab(_)
        | ElementKind::Roof(_)
        | ElementKind::Stair(_)
        | ElementKind::Site(_)
        | ElementKind::Footing(_)
        | ElementKind::FreeMesh(_)
        | ElementKind::Beam(_) => true,
        ElementKind::Wall(_)
        | ElementKind::Column(_)
        | ElementKind::Object(_)
        | ElementKind::Railing(_)
        | ElementKind::Shaft(_) => from_above,
        _ => false,
    }
}

/// The height of `host`'s upper surface at plan point `p`: where a line
/// straight down from above it first meets it. None off its outline.
pub fn surface_height(model: &Model, host: ElementId, p: DVec2) -> Option<f64> {
    let scene = crate::scene::shared_scene(model);
    let top = scene
        .iter()
        .filter(|i| i.id == host)
        .filter_map(|i| i.mesh.bounds())
        .map(|(_, b)| b.z as f64)
        .reduce(f64::max)?;
    around(p)
        .filter_map(|q| hit_down(&scene, q, top + 1.0, &|i| i.id == host))
        .map(|(_, z)| z)
        .reduce(f64::max)
}

/// `p` and four points half a millimetre about it: a point snapped right
/// onto an edge (a ridge, an eave, a slab's rim) reads the top surface
/// there, though a line straight down its edge may slip past the faces.
fn around(p: DVec2) -> impl Iterator<Item = DVec2> {
    const R: f64 = 5e-4;
    [(0.0, 0.0), (R, 0.0), (-R, 0.0), (0.0, R), (0.0, -R)]
        .into_iter()
        .map(move |(x, y)| p + DVec2::new(x, y))
}

/// The element a plan's spot level clicked at `p` stands on, and its
/// height there: the first surface under `below` that a line down from
/// above meets, of the elements [`spot_reads`] takes. `below` is a floor
/// plan's cut, so it reads the floor, not a roof or the floor over it;
/// or the top of a roof or site plan's range.
pub fn surface_under(
    model: &Model,
    p: DVec2,
    below: f64,
    from_above: bool,
) -> Option<(ElementId, f64)> {
    let scene = crate::scene::shared_scene(model);
    let keep = |i: &crate::SceneItem| {
        !i.ghost
            && !i.is_lining()
            && model
                .get(i.id)
                .is_some_and(|e| spot_reads(&e.kind, from_above))
    };
    let top = scene
        .iter()
        .filter(|i| keep(i))
        .filter_map(|i| i.mesh.bounds())
        .map(|(_, b)| b.z as f64)
        .reduce(f64::max)?
        .max(below)
        + 1.0;
    around(p)
        .filter_map(|q| {
            let origin = glam::Vec3::new(q.x as f32, q.y as f32, top as f32);
            crate::pick::pick_all_items(&scene, origin, -glam::Vec3::Z, &keep)
                .into_iter()
                .map(|h| (h.id, top - h.t as f64))
                .find(|(_, z)| *z < below + 1e-4)
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

/// Where a line down from height `from` at `p` first meets a drawn item
/// that `keep` takes (cutters and reveal linings are not drawn).
fn hit_down(
    scene: &[crate::SceneItem],
    p: DVec2,
    from: f64,
    keep: &dyn Fn(&crate::SceneItem) -> bool,
) -> Option<(ElementId, f64)> {
    let origin = glam::Vec3::new(p.x as f32, p.y as f32, from as f32);
    let hit = crate::pick::pick_items(scene, origin, -glam::Vec3::Z, &|i| {
        !i.ghost && !i.is_lining() && keep(i)
    })?;
    Some((hit.id, from - hit.t as f64))
}

/// What a plan's spot levels read under: the top of its range seen from
/// above (a roof or site plan, and whether it is one), else its cut plane,
/// so a roof over a floor plan is not what its spot levels read.
pub fn spot_from(model: &Model, view: &View) -> Option<(f64, bool)> {
    let ViewKind::Plan { level, cut_height } = view.kind else {
        return None;
    };
    if view.graphics.range.from_above {
        let (top, _) = crate::ranges::from_above_span(model, view, level, cut_height);
        return Some((top, true));
    }
    Some((model.level(level)?.elevation + cut_height, false))
}

/// A spot level as a drawing item, if it can be drawn in the view.
fn spot(
    model: &Model,
    view: &View,
    id: ElementId,
    d: &creabim_core::Dimension,
) -> Option<PlanItem> {
    let at = resolve(model, view, &d.a)?;
    let text_at = resolve(model, view, &d.b)?;
    let z = spot_height(model, view, d, at)?;
    let pattern = d
        .pattern
        .as_deref()
        .unwrap_or(creabim_core::annotation::SPOT_PATTERN);
    let plan = matches!(view.kind, ViewKind::Plan { .. });
    // a coordinate mark: the point's S-JTSK Y and X, with a cross on it
    let cross = plan && creabim_core::annotation::writes_coordinates(pattern);
    let text = creabim_core::annotation::spot_text(model, pattern, z);
    let text = if cross {
        creabim_core::annotation::coordinate_text(model, &text, at, z)
    } else {
        text
    };
    Some(PlanItem::Annotation {
        id,
        item: Box::new(Annotation {
            shape: Shape::Spot {
                at,
                text_at,
                plan,
                cross,
                short: d.short,
            },
            text,
            look: dim_look(model, d),
            mm: view.scale / 1000.0,
        }),
    })
}

/// A note as a drawing item; `leader` resolves its leader point.
pub fn note_item(
    model: &Model,
    id: ElementId,
    n: &Note,
    leader: Option<DVec2>,
    mm: f64,
) -> PlanItem {
    let mut look = look_of(model, n.style, n.color);
    look.back |= n.back;
    look.leader_end = n.leader_end.or(look.leader_end);
    if let Some(size) = n.size.filter(|s| *s > 0.0) {
        look.size = size;
    }
    PlanItem::Annotation {
        id,
        item: Box::new(Annotation {
            shape: Shape::Note {
                at: n.at,
                leader,
                elbow: leader.and(n.elbow),
                angle: n.angle.to_radians(),
            },
            text: crate::annotation::lay_out_note(&n.text, &look, n.width, n.markdown),
            look,
            mm,
        }),
    }
}

/// The notes written straight on a sheet, in paper millimetres.
pub fn sheet_notes(model: &Model, sheet: ElementId) -> Vec<PlanItem> {
    model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Note(n) if n.view == sheet => {
                Some(note_item(model, e.id, n, n.leader.map(|p| p.local), 1.0))
            }
            _ => None,
        })
        .collect()
}

/// The dimensions, tags, notes and detail components placed in a view,
/// and the boxes of the detail views called out from it, as drawing items.
pub fn annotations(model: &Model, view_id: ElementId, view: &View) -> Vec<PlanItem> {
    let Some(project) = Projector::of(model, view) else {
        return vec![];
    };
    let mut out = Vec::new();
    for e in model.iter() {
        match &e.kind {
            ElementKind::Detail(d) if d.view == view_id => {
                let look = creabim_core::styles::active_pack(model).membranes;
                out.extend(crate::detail::detail_items(
                    e.id,
                    d,
                    view.scale / 1000.0,
                    look,
                ));
            }
            ElementKind::View(v) => {
                if let Some(c) = v.callout.filter(|c| c.parent == view_id) {
                    let mm = view.scale / 1000.0;
                    match creabim_core::styles::active_pack(model).marker_parts() {
                        Some(parts) => {
                            // a detail called out of a section is ringed
                            let round = matches!(view.kind, ViewKind::Section { .. });
                            out.extend(crate::markers::callout(
                                e.id,
                                &crate::markers::number_of(model, e.id),
                                &crate::markers::sheet_ref(model, e.id),
                                c.lo,
                                c.hi,
                                round,
                                &parts,
                                mm,
                            ));
                        }
                        None => {
                            out.extend(crate::detail::callout_items(e.id, &e.name, c.lo, c.hi, mm))
                        }
                    }
                }
            }
            ElementKind::Dimension(d) if d.view == view_id && d.kind == DimensionKind::Spot => {
                out.extend(spot(model, view, e.id, d));
            }
            ElementKind::Dimension(d) if d.view == view_id && d.kind == DimensionKind::Slope => {
                out.extend(slope(model, view, e.id, d));
            }
            ElementKind::Dimension(d) if d.view == view_id && !d.kind.is_linear() => {
                out.extend(measured(model, view, e.id, d));
            }
            ElementKind::Note(n) if n.view == view_id => {
                let leader = n.leader.and_then(|p| resolve(model, view, &p));
                out.push(note_item(model, e.id, n, leader, view.scale / 1000.0));
            }
            ElementKind::Dimension(d) if d.view == view_id => {
                out.extend(linear_items(model, view, e.id, d));
            }
            ElementKind::Tag(t) if t.view == view_id => {
                let Some(c) = centre(model, t.host) else {
                    continue;
                };
                let centre = project.project(c);
                // a room's tag is its name, number and area, without a bubble
                if matches!(
                    model.get(t.host).map(|h| &h.kind),
                    Some(ElementKind::Room(_))
                ) {
                    // nor in a site or roof plan, which shows no rooms
                    if !view.shows_room_tags() || view.graphics.range.from_above {
                        continue;
                    }
                    let outline = match model.get(t.host).map(|h| &h.kind) {
                        Some(ElementKind::Room(r)) => {
                            creabim_core::room::shape_of(model, r).map(|s| {
                                s.outline
                                    .iter()
                                    .map(|p| project.project(p.extend(c.z)))
                                    .collect::<Vec<_>>()
                            })
                        }
                        _ => None,
                    };
                    // a trade's drawing tags its rooms as the
                    // architecture does (George 2026-11-23)
                    out.extend(room_tag_items(
                        model,
                        e.id,
                        t.host,
                        &t.fields,
                        centre + t.offset,
                        centre,
                        outline.as_deref(),
                        look_of(model, t.style, t.color),
                        view.scale / 1000.0,
                        creabim_core::styles::active_pack(model).room_tags,
                    ));
                    continue;
                }
                // a material tag reads the build-up (Annotate step 5)
                let (text, round) = if t.material {
                    (material_text(model, t.host), false)
                } else {
                    tag_text(model, t.host)
                };
                let mut look = look_of(model, t.style, t.color);
                look.leader_end = t.leader_end.or(look.leader_end);
                let mm = view.scale / 1000.0;
                // a door's or window's mark stands square to its wall, on
                // a straight leader
                let square = if t.material {
                    None
                } else {
                    tag_normal(model, view, t)
                };
                let (mut offset, elbow) = match square {
                    Some(n) => (n * t.offset.dot(n), None),
                    None => (t.offset, t.elbow.map(|p| centre + p)),
                };
                // a door's clear passage, written along its leader
                let passage = square.and_then(|_| match &model.get(t.host)?.kind {
                    ElementKind::Opening(o) => {
                        let (w, h) = creabim_core::joinery::clear_passage(o)?;
                        let ElementKind::Wall(wall) = &model.get(o.host)?.kind else {
                            return None;
                        };
                        let write = |m: f64| spaced(&format!("{:.0}", m * 1000.0));
                        // a fire door's rating goes under its height
                        let fire = o.door.as_ref().and_then(|d| d.fire.as_ref());
                        let under = match fire {
                            Some(f) => format!("{}\n{}", write(h), f.label()),
                            None => write(h),
                        };
                        // the way its leaf swings, and how far
                        let left = if o.faces_left(wall) { 1.0 } else { -1.0 };
                        // its widest leaf, and how far from the wall's
                        // middle the arcs reach, as the plan draws them
                        let (leaf, reach) =
                            leaf_reach(model, o, wall).unwrap_or((w, wall.thickness / 2.0 + w));
                        Some((write(w), under, wall.thickness / 2.0, left, leaf, reach))
                    }
                    _ => None,
                });
                let at = centre + offset;
                let radius = |text: &str| {
                    let o = tag_geometry(at, at, None, text, true, &look, mm).outline;
                    let (lo, hi) = o.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
                        (lo.min(p.x), hi.max(p.x))
                    });
                    (hi - lo).max(0.0) / 2.0
                };
                // do numbers `long` on the leader fit between the wall
                // and the arc a leaf `leaf` wide sweeps, at its middle?
                let fits = |long: f64, leaf: f64| long + 3.0 * mm <= leaf * 0.85;
                // the leader is as long as the numbers on it need
                // and on the side the leaf swings to, the bubble stands
                // clear of the leaf and the arc it sweeps
                // unless it was dragged where it stands
                if let (Some(n), Some((tw, th, face, left, leaf, reach)), false) =
                    (square, &passage, t.placed)
                {
                    let long = th
                        .split('\n')
                        .chain([tw.as_str()])
                        .map(|t| look.width(t))
                        .fold(0.0, f64::max)
                        * mm;
                    let side = if t.offset.dot(n) < 0.0 { -n } else { n };
                    let mut need = face + long + 3.0 * mm + radius(&text);
                    if side.dot(n * *left) > 0.0 {
                        // the numbers inside the arc where they fit,
                        // else past the leaf
                        need = if fits(long, *leaf) {
                            need.max(reach + 1.5 * mm + radius(&text))
                        } else {
                            need + reach - face
                        };
                    }
                    if offset.length() < need {
                        offset = side * need;
                    }
                }
                let at = centre + offset;
                let anchor = match square {
                    Some(_) => centre,
                    None => centre + t.end.unwrap_or(DVec2::ZERO),
                };
                let radius = |text: &str| {
                    let o = tag_geometry(at, at, None, text, true, &look, mm).outline;
                    let (lo, hi) = o.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
                        (lo.min(p.x), hi.max(p.x))
                    });
                    (hi - lo).max(0.0) / 2.0
                };
                // beside a window's mark and touching it, its outer sill's
                // for the list of sheet-metal work: K over its number in a
                // split circle
                if let (Some(n), Some(sill)) =
                    (square, creabim_core::joinery::outer_sill_of(model, t.host))
                {
                    let mark = format!("{}\n{}", creabim_core::joinery::SILL_LETTER, sill.number);
                    let mut u = n.perp();
                    if u.x < -1e-6 || (u.x.abs() <= 1e-6 && u.y < 0.0) {
                        u = -u;
                    }
                    let beside = at + u * (radius(&text) + radius(&mark));
                    out.push(PlanItem::Tag {
                        id: e.id,
                        anchor: beside,
                        at: beside,
                        elbow: None,
                        text: mark,
                        round: true,
                        look,
                    });
                }
                // the width over the line, the height under it, midway
                // from the bubble to the wall's face
                if let Some((tw, th, face, left, leaf, reach)) = passage {
                    let d = anchor - at;
                    // a bubble dragged onto the door still points off
                    // its wall the side it was dragged to
                    let dir = match square {
                        Some(n) if d.length() < 1e-6 => {
                            if t.offset.dot(n) < 0.0 {
                                n
                            } else {
                                -n
                            }
                        }
                        _ => d.normalize_or_zero(),
                    };
                    let (angle, up) = crate::annotation::upright(dir.to_angle());
                    // beside the wall's face, as the office writes them
                    let long = th
                        .split('\n')
                        .chain([tw.as_str()])
                        .map(|t| look.width(t))
                        .fold(0.0, f64::max)
                        * mm;
                    // past the leaf where it swings to this side
                    let swung = square.is_some_and(|n| dir.dot(n * left) < 0.0);
                    let mut clear = if swung && !fits(long, leaf) {
                        reach - face
                    } else {
                        0.0
                    };
                    // a leader dragged short: the numbers between the wall
                    // and the bubble where they fit, else past the bubble
                    let gap = d.length() - radius(&text) - face;
                    if gap < clear + long + 3.0 * mm {
                        clear = 0.0;
                    }
                    let mid = if gap < long + 3.0 * mm {
                        at - dir * (radius(&text) + 1.5 * mm + long / 2.0)
                    } else {
                        anchor - dir * (face + clear + 1.5 * mm + long / 2.0)
                    };
                    let mut rows = vec![(tw, 1.0)];
                    rows.extend(
                        th.split('\n')
                            .enumerate()
                            .map(|(k, t)| (t.to_string(), -1.0 - 1.4 * k as f64)),
                    );
                    for (text, side) in rows {
                        out.push(PlanItem::Annotation {
                            id: e.id,
                            item: Box::new(Annotation {
                                shape: Shape::Along {
                                    at: mid + up * side * look.size * 0.75 * mm,
                                    angle,
                                },
                                text,
                                look,
                                mm,
                            }),
                        });
                    }
                }
                out.push(PlanItem::Tag {
                    id: e.id,
                    anchor,
                    at,
                    elbow,
                    text,
                    round,
                    look,
                });
            }
            _ => {}
        }
    }
    out
}

/// Where a dimension line `p` → `q` crosses the box `corners` (grown by
/// `pad`), as fractions of the way along it.
fn crossing(p: DVec2, q: DVec2, corners: &[DVec2], pad: f64) -> Option<[f64; 2]> {
    let lo = corners.iter().copied().reduce(DVec2::min)? - DVec2::splat(pad);
    let hi = corners.iter().copied().reduce(DVec2::max)? + DVec2::splat(pad);
    let d = q - p;
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for (o, v, a, b) in [(p.x, d.x, lo.x, hi.x), (p.y, d.y, lo.y, hi.y)] {
        if v.abs() < 1e-12 {
            if o < a || o > b {
                return None;
            }
            continue;
        }
        let (mut s0, mut s1) = ((a - o) / v, (b - o) / v);
        if s0 > s1 {
            std::mem::swap(&mut s0, &mut s1);
        }
        t0 = t0.max(s0);
        t1 = t1.min(s1);
    }
    (t1 - t0 > 1e-9).then_some([t0, t1])
}

/// Give each dimension whose style breaks at tags the gaps where the
/// tags' bubbles and the notes' text sit across its line; `mm` is drawing
/// units per paper mm.
pub fn break_dimensions(items: &mut [PlanItem], mm: f64) {
    let mut boxes: Vec<Vec<DVec2>> = Vec::new();
    for it in items.iter() {
        match it {
            PlanItem::Tag {
                anchor,
                at,
                elbow,
                text,
                round,
                look,
                ..
            } => boxes.push(tag_geometry(*anchor, *at, *elbow, text, *round, look, mm).outline),
            PlanItem::Annotation { item, .. } if matches!(item.shape, Shape::Note { .. }) => {
                boxes.extend(item.text_boxes().iter().map(|b| b.to_vec()))
            }
            _ => {}
        }
    }
    boxes.retain(|b| !b.is_empty());
    for it in items.iter_mut() {
        let PlanItem::Dimension {
            a,
            b,
            offset,
            look,
            breaks,
            ..
        } = it
        else {
            continue;
        };
        breaks.clear();
        if !look.breaks {
            continue;
        }
        let n = (*b - *a).normalize_or_zero().perp() * *offset;
        let (p, q) = (*a + n, *b + n);
        let mut gaps: Vec<[f64; 2]> = boxes
            .iter()
            .filter_map(|c| crossing(p, q, c, 0.8 * mm))
            .collect();
        gaps.sort_by(|x, y| x[0].total_cmp(&y[0]));
        for g in gaps {
            match breaks.last_mut() {
                Some(last) if g[0] <= last[1] => last[1] = last[1].max(g[1]),
                _ => breaks.push(g),
            }
        }
    }
}

impl DimensionGeometry {
    /// Leave the stretches `breaks` (fractions of the way from `pa` to
    /// `pb`, the dimension line's measured ends) out of its line.
    pub fn break_line(&mut self, breaks: &[[f64; 2]]) {
        if breaks.is_empty() || self.lines.is_empty() {
            return;
        }
        let (s, e) = self.lines[0];
        let (pa, pb) = self.ends_at;
        let len = pa.distance(pb);
        if len < 1e-12 {
            return;
        }
        let u = (pb - pa) / len;
        let t = |p: DVec2| (p - pa).dot(u) / len;
        let at = |f: f64| pa + (pb - pa) * f;
        let mut pieces = vec![];
        let mut from = t(s);
        for br in breaks {
            if br[0] > from {
                pieces.push((at(from), at(br[0].min(t(e)))));
            }
            from = from.max(br[1]);
        }
        if t(e) > from {
            pieces.push((at(from), e));
        }
        pieces.retain(|(p, q)| p.distance(*q) > 1e-9);
        self.lines.splice(0..1, pieces);
    }
}

/// Linework of a dimension, in drawing units; `mm` is drawing units per
/// paper millimetre, so ticks and gaps are a fixed size on paper.
pub struct DimensionGeometry {
    pub lines: Vec<(DVec2, DVec2)>,
    /// The ends of the dimension line: ticks (drawn heavier than the
    /// line), arrows, dots, as its style says.
    pub ends: Ends,
    pub text_at: DVec2,
    /// Text angle in radians, kept readable (never upside down).
    pub angle: f64,
    /// Where the dimension line meets its witness lines.
    pub ends_at: (DVec2, DVec2),
    /// Where text under the line goes, mirroring `text_at`.
    pub below_at: DVec2,
    /// The text covers what is under it.
    pub back: bool,
}

impl DimensionGeometry {
    /// The geometry with its text where `place` says and its witness
    /// lines as it says. Call before [`DimensionGeometry::break_line`].
    pub fn placed(mut self, place: &crate::plan::DimPlace, mm: f64) -> Self {
        if place.short {
            // 4.5 mm of witness line before the dimension line
            for (from, to) in self.lines.iter_mut().skip(1) {
                let d = *to - *from;
                let len = d.length();
                let keep = 6.0 * mm;
                if len > keep {
                    *from = *to - d / len * keep;
                }
            }
        }
        if place.shift != DVec2::ZERO {
            let (pa, pb) = self.ends_at;
            let mid = (pa + pb) / 2.0;
            let up = (self.text_at - mid).normalize_or_zero();
            let off = place.shift.dot(up);
            if place.leader && off.abs() > 1.0 * mm {
                // from the line to under the text
                let foot = self.text_at + place.shift - up * off.signum() * 1.2 * mm;
                self.lines.push((mid, foot));
            }
            self.text_at += place.shift;
            self.below_at += place.shift;
        }
        self.back = place.back;
        self
    }

    /// Corners of the patch behind `text` at `at` (see
    /// [`crate::plan::DimPlace::back`]).
    pub fn patch(&self, at: DVec2, text: &str, look: &TextLook, mm: f64) -> [DVec2; 4] {
        let dir = DVec2::from_angle(self.angle);
        let up = dir.perp();
        let w = (look.width(text) / 2.0 + 0.4) * mm;
        let h = (look.size * 0.5 + 0.25) * mm;
        [
            at - dir * w - up * h,
            at + dir * w - up * h,
            at + dir * w + up * h,
            at - dir * w + up * h,
        ]
    }
}

/// A linear dimension's linework: its line `offset` to the left of `a` →
/// `b`, witness lines back to the measured points (`witness` when they are
/// not `a` and `b`, as for a horizontal or vertical dimension), and its ends.
pub fn dimension_geometry(
    a: DVec2,
    b: DVec2,
    offset: f64,
    witness: Option<[DVec2; 2]>,
    look: &TextLook,
    mm: f64,
) -> DimensionGeometry {
    let u = (b - a).normalize_or_zero();
    let n = u.perp();
    let off = n * offset;
    let (pa, pb) = (a + off, b + off);
    let end = look.end.unwrap_or(Terminator::Tick);
    let run = if runs_past(end) { 1.5 * mm } else { 0.0 };
    let mut lines = vec![(pa - u * run, pb + u * run)];
    let [wa, wb] = witness.unwrap_or([a, b]);
    // witness lines from just off the measured points to just past the line
    for (from, to) in [(wa, pa), (wb, pb)] {
        let reach = (to - from).dot(n);
        if reach.abs() > 2.0 * mm {
            let s = n * reach.signum();
            lines.push((from + s * 1.0 * mm, to + s * 1.5 * mm));
        }
    }
    let mut ends = end_marks(end, pa, -u, look.end_size, mm);
    ends.extend(end_marks(end, pb, u, look.end_size, mm));
    let mut angle = u.to_angle();
    let mut up = n;
    if angle > std::f64::consts::FRAC_PI_2 + 1e-6 || angle <= -std::f64::consts::FRAC_PI_2 + 1e-6 {
        angle += std::f64::consts::PI;
        up = -n;
    }
    if angle > std::f64::consts::PI {
        angle -= std::f64::consts::TAU;
    }
    DimensionGeometry {
        lines,
        ends,
        // 1.8 mm off the line for the default 2.5 mm text
        text_at: (pa + pb) / 2.0 + up * (0.55 + look.size / 2.0) * mm,
        below_at: (pa + pb) / 2.0 - up * (0.55 + look.size / 2.0) * mm,
        angle,
        ends_at: (pa, pb),
        back: false,
    }
}

/// A tag's bubble and leader, in drawing units.
pub struct TagGeometry {
    pub outline: Vec<DVec2>,
    /// The outline is a closed shape, filled white under the text (else an
    /// underline, or nothing for text alone).
    pub closed: bool,
    /// The leader from the bubble's edge (through its elbow) to the host,
    /// as a polyline; empty when the tag sits on its host.
    pub leader: Vec<DVec2>,
    /// The mark at the leader's end: an arrow, a dot, or nothing.
    pub ends: Ends,
    /// Its text, a line at a time, where each is centred.
    pub texts: Vec<(DVec2, String)>,
    /// The line across a split circle, between its two lines of text.
    pub divider: Option<(DVec2, DVec2)>,
}

/// Text height of tags and dimensions, paper mm.
pub const TEXT: f64 = 2.5;

/// A tag's outline of `shape` about `at`, its text half `half` wide and high.
fn tag_outline(shape: TagShape, at: DVec2, half: DVec2) -> (Vec<DVec2>, bool) {
    use std::f64::consts::{FRAC_PI_2, PI, TAU};
    let ring = |n: usize, r: DVec2, turn: f64| -> Vec<DVec2> {
        (0..n)
            .map(|i| {
                let t = turn + i as f64 / n as f64 * TAU;
                at + DVec2::new(t.cos() * r.x, t.sin() * r.y)
            })
            .collect()
    };
    // rounded corners: quarter circles of `r` inside a box `h`
    let rounded = |h: DVec2, r: f64| -> Vec<DVec2> {
        let r = r.min(h.x).min(h.y);
        let mut pts = vec![];
        for (k, c) in [
            DVec2::new(h.x - r, h.y - r),
            DVec2::new(-h.x + r, h.y - r),
            DVec2::new(-h.x + r, -h.y + r),
            DVec2::new(h.x - r, -h.y + r),
        ]
        .into_iter()
        .enumerate()
        {
            for i in 0..=6 {
                let t = k as f64 * FRAC_PI_2 + FRAC_PI_2 * i as f64 / 6.0;
                pts.push(at + c + DVec2::from_angle(t) * r);
            }
        }
        pts
    };
    let boxed = |h: DVec2| {
        vec![
            at - h,
            at + DVec2::new(h.x, -h.y),
            at + h,
            at + DVec2::new(-h.x, h.y),
        ]
    };
    match shape {
        TagShape::Auto | TagShape::Box => (boxed(half), true),
        TagShape::Circle => (ring(24, DVec2::splat(half.x.max(half.y)), 0.0), true),
        TagShape::RoundedBox => (rounded(half, half.y * 0.45), true),
        TagShape::Pill => (rounded(half + DVec2::X * half.y * 0.4, half.y), true),
        TagShape::Hexagon => {
            // flat top and bottom, points left and right
            let w = half.x + half.y * 0.6;
            (
                vec![
                    at + DVec2::new(w, 0.0),
                    at + DVec2::new(half.x, half.y),
                    at + DVec2::new(-half.x, half.y),
                    at + DVec2::new(-w, 0.0),
                    at + DVec2::new(-half.x, -half.y),
                    at + DVec2::new(half.x, -half.y),
                ],
                true,
            )
        }
        TagShape::Octagon => {
            let r = half.x.max(half.y) / (PI / 8.0).cos();
            (ring(8, DVec2::splat(r), PI / 8.0), true)
        }
        TagShape::Diamond => {
            let h = half * 1.45;
            (
                vec![
                    at + DVec2::new(h.x, 0.0),
                    at + DVec2::new(0.0, h.y),
                    at + DVec2::new(-h.x, 0.0),
                    at + DVec2::new(0.0, -h.y),
                ],
                true,
            )
        }
        TagShape::Underline => (
            vec![
                at + DVec2::new(-half.x, -half.y * 0.75),
                at + DVec2::new(half.x, -half.y * 0.75),
            ],
            false,
        ),
        TagShape::TextOnly => (vec![], false),
    }
}

/// A tag's outline and leader. `round` is the tag's usual shape (doors
/// and windows) when its style leaves the shape to the tag.
pub fn tag_geometry(
    anchor: DVec2,
    at: DVec2,
    elbow: Option<DVec2>,
    text: &str,
    round: bool,
    look: &TextLook,
    mm: f64,
) -> TagGeometry {
    // the default 2.5 mm text sits in a bubble 4.4 mm tall
    let k = look.size / TEXT;
    let shape = match (look.shape, round) {
        (TagShape::Auto, true) => TagShape::Circle,
        (s, _) => s,
    };
    // two lines in a circle (STN 01 3420's door mark: L or P over the
    // number) split by a line across its middle
    let rows: Vec<&str> = text.split('\n').collect();
    let split = rows.len() == 2 && shape == TagShape::Circle;
    let (half, texts, divider) = if split {
        let w = rows.iter().map(|r| look.width(r)).fold(0.0, f64::max);
        // the text rows sit where the circle is 0.88 of its width
        let r = ((w / 2.0 + 0.6 * k) / 0.88).max(3.4 * k) * mm;
        let dy = DVec2::new(0.0, 0.48 * r);
        (
            DVec2::splat(r),
            vec![
                (at + dy, rows[0].to_string()),
                (at - dy, rows[1].to_string()),
            ],
            Some((at - DVec2::new(r, 0.0), at + DVec2::new(r, 0.0))),
        )
    } else {
        (
            DVec2::new((look.width(text) / 2.0 + 1.2 * k).max(2.2 * k), 2.2 * k) * mm,
            vec![(at, text.to_string())],
            None,
        )
    };
    let (outline, closed) = tag_outline(shape, at, half);
    // a leader back to the host, from the edge of the bubble (or of the
    // text's box when there is no bubble), towards its elbow if it has one
    let first = elbow.unwrap_or(anchor);
    let d = first - at;
    let edge = if closed {
        outline.clone()
    } else {
        tag_outline(TagShape::Box, at, half).0
    };
    let mut t = 1.0f64;
    for i in 0..edge.len() {
        let (p, q) = (edge[i], edge[(i + 1) % edge.len()]);
        let e = q - p;
        let den = d.perp_dot(e);
        if den.abs() < 1e-18 {
            continue;
        }
        let s = (p - at).perp_dot(e) / den;
        let v = (p - at).perp_dot(d) / den;
        if s > 0.0 && (-1e-9..=1.0 + 1e-9).contains(&v) {
            t = t.min(s);
        }
    }
    let mut leader = vec![];
    if t < 1.0 && d.length() * (1.0 - t) > 1.0 * mm {
        leader.push(at + d * t);
        leader.extend(elbow);
        leader.push(anchor);
    } else if elbow.is_some() {
        // the elbow is inside the bubble: run straight on from it
        leader = vec![first, anchor];
    }
    leader.dedup_by(|a, b| a.distance(*b) < 1e-9);
    let mut ends = Ends::default();
    if let (Some(end), [.., from, tip]) = (look.leader_end, leader.as_slice()) {
        ends = end_marks(end, *tip, *tip - *from, look.end_size, mm);
    }
    if leader.len() < 2 {
        leader.clear();
    }
    TagGeometry {
        outline,
        closed,
        leader,
        ends,
        texts,
        divider,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_stand_apart() {
        assert_eq!(spaced("900"), "900");
        assert_eq!(spaced("1970"), "1\u{a0}970");
        assert_eq!(spaced("12500"), "12\u{a0}500");
        assert_eq!(spaced("1.25"), "1.25");
        assert_eq!(spaced("+3.000"), "+3.000");
    }

    #[test]
    fn a_short_stretch_has_its_text_beside_it() {
        // a chain at 1:50 (0.05 m per paper mm): 2 m, 0.1 m, 2 m, 0.1 m
        let mm = 0.05;
        let id = ElementId::new();
        let stops = [0.0, 2.0, 2.1, 4.1, 4.2];
        let mut items: Vec<PlanItem> = stops
            .windows(2)
            .map(|w| PlanItem::Dimension {
                id,
                a: DVec2::new(w[0], 0.0),
                b: DVec2::new(w[1], 0.0),
                offset: 1.0,
                text: format!("{:.0}", (w[1] - w[0]) * 1000.0),
                look: TextLook::default(),
                witness: None,
                breaks: vec![],
                below: None,
                place: Default::default(),
            })
            .collect();
        place_dimension_texts(&mut items, mm);
        let place = |i: usize| match &items[i] {
            PlanItem::Dimension { place, .. } => *place,
            _ => unreachable!(),
        };
        // the long ones stay over their middle
        assert_eq!(place(0).shift, DVec2::ZERO);
        assert_eq!(place(2).shift, DVec2::ZERO);
        // the one inside goes beside it, over a neighbour with room
        let inner = place(1);
        assert!(inner.shift.x.abs() > 0.05 && inner.shift.y == 0.0 && !inner.leader);
        // the last past the chain's end
        assert!(place(3).shift.x > 0.05);
        // a text dragged by hand stays where it was put
        if let PlanItem::Dimension { place, .. } = &mut items[1] {
            *place = crate::plan::DimPlace {
                shift: DVec2::new(0.0, 0.3),
                manual: true,
                ..Default::default()
            };
        }
        place_dimension_texts(&mut items, mm);
        assert_eq!(place_of(&items[1]).shift, DVec2::new(0.0, 0.3));
        // three short ones in a row: the middle one has no room beside
        let stops = [0.0, 2.0, 2.1, 2.2, 2.3, 4.3];
        let mut items: Vec<PlanItem> = stops
            .windows(2)
            .map(|w| PlanItem::Dimension {
                id,
                a: DVec2::new(w[0], 0.0),
                b: DVec2::new(w[1], 0.0),
                offset: 1.0,
                text: "100".into(),
                look: TextLook::default(),
                witness: None,
                breaks: vec![],
                below: None,
                place: Default::default(),
            })
            .collect();
        place_dimension_texts(&mut items, mm);
        let mid = place_of(&items[2]);
        assert!(mid.leader && mid.shift.y > 0.0 && mid.shift.x == 0.0);
        // moved texts move with the geometry
        let g = dimension_geometry(DVec2::ZERO, DVec2::X, 1.0, None, &TextLook::default(), mm);
        let at = g.text_at;
        let g = g.placed(&mid, mm);
        assert_eq!(g.text_at, at + mid.shift);
        assert_eq!(g.lines.len(), 4, "the line, two witness lines, the leader");
    }

    fn place_of(it: &PlanItem) -> crate::plan::DimPlace {
        match it {
            PlanItem::Dimension { place, .. } => *place,
            _ => unreachable!(),
        }
    }
    use crate::section::view_drawing;
    use creabim_core::sample::sample_house;

    fn named(m: &Model, name: &str) -> ElementId {
        m.iter().find(|e| e.name == name).unwrap().id
    }

    #[test]
    fn a_material_tag_reads_the_layers_outside_to_inside() {
        // Annotate step 5: a layered wall reads one line a layer
        let m = creabim_core::starter::starter_house();
        let wall = m
            .iter()
            .find(|e| {
                matches!(e.kind, ElementKind::Wall(_))
                    && creabim_core::assembly_of(&m, e).is_some_and(|(_, a)| a.layers.len() > 1)
            })
            .expect("a layered wall");
        let (_, a) = creabim_core::assembly_of(&m, wall).unwrap();
        let text = material_text(&m, wall.id);
        assert_eq!(text.lines().count(), a.layers.len(), "{text}");
    }

    #[test]
    fn room_tags_show_the_fields_they_list() {
        let m = creabim_core::starter::starter_house();
        let study = named(&m, "Study");
        let text = |f: &[RoomField]| room_tag_text(&m, study, f);
        let area = RoomField::Area.value(&m, study);
        assert_eq!(text(&[]), format!("STUDY\n1.01  {area}"));
        assert_eq!(
            text(&[
                RoomField::Name,
                RoomField::Area,
                RoomField::FloorFinish,
                RoomField::Level
            ]),
            format!("STUDY\n{area}\nOak boards\nGround Floor")
        );
        // a field the room has nothing for is left out
        assert_eq!(text(&[RoomField::Department, RoomField::Number]), "1.01");
        // and the tag's own list is what its drawing writes
        let mut doc = creabim_core::Document::from_model(m.clone());
        let tag = m
            .iter()
            .find(|e| matches!(&e.kind, ElementKind::Tag(t) if t.host == study))
            .unwrap()
            .id;
        doc.edit(tag, "Fields", |e| {
            if let ElementKind::Tag(t) = &mut e.kind {
                t.fields = vec![RoomField::Name, RoomField::CeilingFinish];
            }
        })
        .unwrap();
        let ElementKind::Tag(t) = doc.model().get(tag).unwrap().kind.clone() else {
            unreachable!()
        };
        let ElementKind::View(view) = doc.model().get(t.view).unwrap().kind.clone() else {
            unreachable!()
        };
        let items = annotations(doc.model(), t.view, &view);
        let shown: Vec<String> = items
            .iter()
            .filter_map(|i| match i {
                PlanItem::Annotation { id, item } if *id == tag => Some(item.text.clone()),
                _ => None,
            })
            .collect();
        let ceiling = RoomField::CeilingFinish.value(doc.model(), study);
        let want: Vec<String> = ["STUDY".to_string(), ceiling]
            .into_iter()
            .filter(|l| !l.is_empty())
            .collect();
        assert_eq!(shown, want);
        // with no list, Stacked: the name in semibold, the area, the number small
        doc.edit(tag, "Fields", |e| {
            if let ElementKind::Tag(t) = &mut e.kind {
                t.fields = vec![];
            }
        })
        .unwrap();
        let items = annotations(doc.model(), t.view, &view);
        let lines: Vec<(String, f64, bool)> = items
            .iter()
            .filter_map(|i| match i {
                PlanItem::Annotation { id, item } if *id == tag => {
                    Some((item.text.clone(), item.look.size, item.look.bold))
                }
                _ => None,
            })
            .collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].0, "STUDY");
        assert!(lines[0].2);
        assert_eq!(lines[1].0, area);
        assert_eq!(lines[2].0, "1.01");
        assert!(lines[2].1 < lines[1].1 && lines[1].1 < lines[0].1);
    }

    #[test]
    fn dimensions_follow_their_walls() {
        let mut doc = creabim_core::Document::from_model(sample_house());
        let plan = named(doc.model(), "Ground Floor plan");
        let overall = |m: &Model| {
            view_drawing(m, plan)
                .unwrap()
                .items
                .into_iter()
                .filter_map(|i| match i {
                    PlanItem::Dimension { text, .. } => {
                        text.replace('\u{a0}', "").parse::<f64>().ok()
                    }
                    _ => None,
                })
                .fold(0.0, f64::max)
        };
        assert_eq!(overall(doc.model()), 10300.0);
        // stretch the house: move the east walls out by a metre
        for name in ["GF Wall 1", "GF Wall 2", "GF Wall 3"] {
            let id = named(doc.model(), name);
            doc.edit(id, "Stretch", |e| {
                if let ElementKind::Wall(w) = &mut e.kind {
                    for p in [&mut w.start, &mut w.end] {
                        if p.x > 9.0 {
                            p.x += 1.0;
                        }
                    }
                }
            })
            .unwrap();
        }
        assert_eq!(overall(doc.model()), 11300.0);
    }

    #[test]
    fn dimension_points_follow_the_masses_they_sit_on() {
        use creabim_core::transform::transformed;
        use creabim_core::{EditMesh, Element, FreeMesh, Transaction, Xform};
        let mut model = sample_house();
        let level = model.levels().next().unwrap().0.id;
        let mass = Element::new(
            "Mass",
            ElementKind::FreeMesh(FreeMesh::new(
                Some(level),
                DVec3::new(20.0, 5.0, 0.0),
                EditMesh::cuboid(DVec3::new(2.0, 4.0, 3.0)),
            )),
        );
        let id = mass.id;
        Transaction::new("Add mass")
            .insert(mass)
            .apply(&mut model)
            .unwrap();
        let plan = View::plan(level);
        let section = View::section(DVec2::new(15.0, 0.0), DVec2::new(25.0, 0.0), 20.0, true);
        // a plan corner and a section corner (top of the near face)
        let corner = DVec2::new(21.0, 7.0);
        let top = DVec2::new(6.0, 3.0);
        let in_plan = attach(&model, &plan, Some(id), corner);
        let in_section = attach(&model, &section, Some(id), top);
        assert_eq!(in_plan.host, Some(id));
        assert!(in_plan.body.is_some() && in_section.body.is_some());
        assert!(resolve(&model, &plan, &in_plan).unwrap().distance(corner) < 1e-9);
        assert!(
            resolve(&model, &section, &in_section)
                .unwrap()
                .distance(top)
                < 1e-9
        );
        // move the mass, then turn it a quarter about a point
        let moves = [
            Xform::Move(DVec3::new(1.0, -2.0, 0.5)),
            Xform::Rotate {
                center: DVec2::new(22.0, 5.0),
                degrees: 90.0,
            },
        ];
        let mut e = model.get(id).unwrap().clone();
        for t in &moves {
            e = transformed(&model, &e, t).unwrap();
        }
        let before = model.get(id).unwrap().clone();
        Transaction::new("Move mass")
            .replace(before, e)
            .apply(&mut model)
            .unwrap();
        let expect = moves.iter().fold(corner, |p, t| t.point2(p));
        let now = resolve(&model, &plan, &in_plan).unwrap();
        assert!(now.distance(expect) < 1e-9, "{now} vs {expect}");
        // the section point rose with the mass
        let now = resolve(&model, &section, &in_section).unwrap();
        assert!((now.y - 3.5).abs() < 1e-9, "{now}");
    }

    #[test]
    fn tags_read_marks_and_follow_doors() {
        let mut doc = creabim_core::Document::from_model(sample_house());
        let plan = named(doc.model(), "Ground Floor plan");
        let door = named(doc.model(), "Front Door");
        let tag_at = |m: &Model| {
            view_drawing(m, plan)
                .unwrap()
                .items
                .into_iter()
                .find_map(|i| match i {
                    // its leader's end, which follows the door (the text
                    // may be nudged clear of other lines)
                    PlanItem::Tag { anchor, text, .. } if text.ends_with("\nD01") => Some(anchor),
                    _ => None,
                })
                .expect("front door is tagged D01")
        };
        let before = tag_at(doc.model());
        doc.edit(door, "Move door", |e| {
            if let ElementKind::Opening(o) = &mut e.kind {
                o.offset += 2.0;
            }
        })
        .unwrap();
        let after = tag_at(doc.model());
        assert!((after.x - before.x - 2.0).abs() < 1e-9);
    }

    #[test]
    fn dimension_text_reads_upright() {
        let g = dimension_geometry(
            DVec2::new(0.0, 5.0),
            DVec2::ZERO,
            1.0,
            None,
            &TextLook::default(),
            0.1,
        );
        assert!(g.angle.abs() <= std::f64::consts::FRAC_PI_2 + 1e-9);
    }

    #[test]
    fn a_horizontal_dimension_measures_square_across() {
        use creabim_core::{DimAxis, Dimension, Element, Transaction};
        let mut doc = creabim_core::Document::from_model(sample_house());
        let plan = named(doc.model(), "Ground Floor plan");
        let (a, b) = (DVec2::new(1.0, 1.0), DVec2::new(1.47, 0.4));
        let mut d = Dimension::linear(plan, HostPoint::free(a), HostPoint::free(b), 0.5);
        d.axis = DimAxis::Horizontal;
        let e = Element::new("Dimension", ElementKind::Dimension(d));
        let id = e.id;
        doc.commit(Transaction::new("Add").insert(e)).unwrap();
        let item = view_drawing(doc.model(), plan)
            .unwrap()
            .items
            .into_iter()
            .find(|i| i.id() == id)
            .unwrap();
        let PlanItem::Dimension {
            a: pa,
            b: pb,
            text,
            witness,
            offset,
            look,
            ..
        } = item
        else {
            panic!("a linear dimension")
        };
        // 470 across, not the 760 along the diagonal
        assert_eq!(text, "470");
        assert!((pb - DVec2::new(1.47, 1.0)).length() < 1e-9 && pa == a);
        assert_eq!(witness, Some([a, b]));
        // the line is level, and the witness from the lower point reaches it
        let g = dimension_geometry(pa, pb, offset, witness, &look, 0.01);
        let (p, q) = g.lines[0];
        assert!((p.y - q.y).abs() < 1e-9);
        assert!(g.lines.iter().any(|(s, t)| (s.x - 1.47).abs() < 1e-9
            && (t.x - 1.47).abs() < 1e-9
            && s.y < 0.45
            && t.y > 1.5));
        // points level with each other measure nothing vertically
        assert!(squared(DimAxis::Vertical, a, DVec2::new(3.0, 1.0)).is_none());
    }

    #[test]
    fn styles_change_dimension_ends_and_tag_shapes() {
        let mut look = TextLook::default();
        let tick = dimension_geometry(DVec2::ZERO, DVec2::X * 3.0, 1.0, None, &look, 0.01);
        assert_eq!(tick.ends.heavy.len(), 2, "two ticks by default");
        look.end = Some(Terminator::Dot);
        let dots = dimension_geometry(DVec2::ZERO, DVec2::X * 3.0, 1.0, None, &look, 0.01);
        assert!(dots.ends.heavy.is_empty() && dots.ends.fills.len() == 2);
        // a tick.s line runs on past it; a dot.s or an arrow.s stops on its point
        assert!(tick.lines[0].0.x < 0.0);
        assert_eq!(dots.lines[0].0.x, 0.0);
        look.end = Some(Terminator::ArrowOpen);
        let open = dimension_geometry(DVec2::ZERO, DVec2::X * 3.0, 1.0, None, &look, 0.01);
        assert_eq!(open.lines[0].0.x, 0.0);
        assert_eq!(open.ends.thin.len(), 2);
        // every end draws something but None
        for t in Terminator::ALL {
            let e = end_marks(t, DVec2::ZERO, DVec2::X, 2.5, 0.01);
            let n = e.thin.len() + e.heavy.len() + e.fills.len();
            assert_eq!(n == 0, t == Terminator::None, "{t:?}");
        }
        // tags: shapes, and the leader leaving each from its edge
        let anchor = DVec2::new(0.0, -1.0);
        for shape in TagShape::ALL {
            look.shape = shape;
            let g = tag_geometry(anchor, DVec2::ZERO, None, "D01", false, &look, 0.01);
            assert_eq!(
                g.closed,
                !matches!(shape, TagShape::Underline | TagShape::TextOnly)
            );
            let [from, to] = g.leader[..] else {
                panic!("a leader to the far anchor")
            };
            assert_eq!(to, anchor);
            assert!(from.y < 0.0 && from.y > -0.05, "{shape:?} {from}");
        }
        look.shape = TagShape::Auto;
        let round = tag_geometry(anchor, DVec2::ZERO, None, "D01", true, &look, 0.01);
        assert_eq!(round.outline.len(), 24, "openings stay round");
    }
}
