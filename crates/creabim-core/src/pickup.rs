//! Picking up an element's parameters (George 2026-11-20, as ArchiCAD's
//! Alt+click): the element clicked becomes a template, and every new
//! element of the same kind its tool draws takes all of the template's
//! parameters (type, assembly, material, heights, offsets, sizes, pens,
//! styles...), keeping only where and on what it was drawn.
//!
//! What counts as "where" is listed per kind in [`placement_keys`]; all
//! the rest is copied, so a parameter added to a kind later is picked up
//! without touching this file.

use crate::{Element, ElementId, ElementKind, Model};
use serde_json::Value;

/// The fields of each kind (by its serialised name) that say where an
/// element is and what it sits on: a new element keeps its own. A path
/// through an enum's variant names it (`form.Extrusion.outline`).
fn placement_keys(tag: &str) -> &'static [&'static str] {
    match tag {
        "Wall" => &[
            "level",
            "start",
            "end",
            "bulge",
            "disallow_join",
            "attach_top",
            "layer_ends",
        ],
        "Slab" => &["level", "boundary", "bulges", "holes", "layer_edges"],
        "Roof" => &["level", "boundary", "edge", "holes", "layer_edges"],
        "Opening" => &["host", "offset", "flip_hand", "flip_facing"],
        "Column" => &["level", "position", "rotation", "in_wall"],
        "Beam" => &[
            "level", "start", "end", "rise", "up", "extend", "cuts", "lintel",
        ],
        "Stair" => &["level", "start", "direction"],
        "Railing" => &["level", "path", "stair"],
        "Footing" => &["level", "start", "end", "rotation", "host"],
        "Shaft" => &["level", "boundary"],
        "Object" => &["level", "position", "rotation", "mark"],
        "Part" => &[
            "level",
            "form.Extrusion.outline",
            "form.Extrusion.holes",
            "form.Extrusion.plane",
            "form.Sweep.path",
        ],
        "Site" => &[
            "level",
            "form.Terrain.boundary",
            "form.Terrain.points",
            "form.Area.boundary",
            "form.Area.holes",
            "form.Earthwork.boundary",
            "form.Plant.position",
        ],
        "Room" => &["level", "at", "boundary", "number", "ufh"],
        "RoomSeparator" => &["level", "a", "b"],
        "Ceiling" => &["level", "boundary", "room"],
        "Detail" => &["view", "points", "flip"],
        "Pipe" => &["level", "start", "end", "mark"],
        "Valve" => &["pipe", "t"],
        "Dimension" => &[
            "view", "a", "b", "offset", "center", "mids", "texts", "held", "axis", "eq",
        ],
        "Tag" => &["view", "host", "offset", "elbow", "end"],
        "Note" => &["view", "at", "text", "leader", "elbow"],
        "Sketch" => &[
            "level",
            "plane",
            "points",
            "closed",
            "kinds",
            "circle",
            "constraints",
        ],
        "Grid" => &["label", "start", "end"],
        _ => &[],
    }
}

/// The fields that make two elements of a kind different things, drawn
/// by different tools or settings: a door is not a window, a strip
/// footing not a pad, a void not an extrusion.
fn family_keys(tag: &str) -> &'static [&'static str] {
    match tag {
        "Opening" => &["kind"],
        "Footing" => &["kind"],
        "Part" => &["form", "void"],
        "Site" => &["form"],
        "Detail" => &["kind"],
        "Dimension" => &["kind"],
        "Sketch" => &["net", "mark"],
        _ => &[],
    }
}

/// Kinds a template may be made from: elements drawn by a tool, not
/// levels, views, sheets, materials or types.
pub fn can_template(kind: &ElementKind) -> bool {
    tagged(kind).is_some_and(|(tag, _)| !placement_keys(&tag).is_empty())
}

fn tagged(kind: &ElementKind) -> Option<(String, Value)> {
    match serde_json::to_value(kind).ok()? {
        Value::Object(m) if m.len() == 1 => m.into_iter().next(),
        _ => None,
    }
}

/// An enum's variant name in serialised form: a string for a unit
/// variant, the one key of an object otherwise; none when absent.
fn variant(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::String(s) => Some(s.clone()),
        Value::Object(m) if m.len() == 1 => m.keys().next().cloned(),
        Value::Null => None,
        other => Some(other.to_string()),
    }
}

/// Are the two the same kind of thing, drawn by the same tool?
pub fn same_family(a: &ElementKind, b: &ElementKind) -> bool {
    let (Some((ta, va)), Some((tb, vb))) = (tagged(a), tagged(b)) else {
        return false;
    };
    ta == tb
        && family_keys(&ta).iter().all(|k| {
            // a sketch's network or site mark: present on both or neither,
            // of the same kind
            let (x, y) = (va.get(k), vb.get(k));
            match (x, y) {
                (Some(Value::Object(x)), Some(Value::Object(y))) if *k == "net" || *k == "mark" => {
                    x.get("kind") == y.get("kind") || x == y
                }
                _ => variant(x) == variant(y),
            }
        })
}

fn get_path<'a>(v: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(v, |v, k| v.get(*k))
}

fn set_path(v: &mut Value, path: &[&str], value: Option<Value>) {
    let Some((last, head)) = path.split_last() else {
        return;
    };
    let mut at = v;
    for k in head {
        match at.get_mut(*k) {
            Some(next) => at = next,
            None => return,
        }
    }
    if let Value::Object(m) = at {
        match value {
            Some(x) => {
                m.insert(last.to_string(), x);
            }
            None => {
                m.remove(*last);
            }
        }
    }
}

/// `new` (just drawn) made with every parameter of `template` but its
/// place. False, and `new` left as it was, when the two are not the same
/// kind of thing.
pub fn adopt(model: &Model, template: &Element, new: &mut Element) -> bool {
    if template.id == new.id || !same_family(&template.kind, &new.kind) {
        return false;
    }
    let (Some((tag, mut out)), Some((_, drawn))) = (tagged(&template.kind), tagged(&new.kind))
    else {
        return false;
    };
    for key in placement_keys(&tag) {
        let path: Vec<&str> = key.split('.').collect();
        // a path through a variant the template is not: nothing to keep
        if path.len() > 1 && get_path(&out, &path[..path.len() - 1]).is_none() {
            continue;
        }
        set_path(&mut out, &path, get_path(&drawn, &path).cloned());
    }
    // a strip's length is its run between start and end, a pad's its size
    if tag == "Footing" && variant(out.get("kind")).as_deref() == Some("Strip") {
        set_path(&mut out, &["length"], drawn.get("length").cloned());
    }
    let mut obj = serde_json::Map::new();
    obj.insert(tag, out);
    let Ok(mut kind) = serde_json::from_value::<ElementKind>(Value::Object(obj)) else {
        return false;
    };
    relevel(model, template, &mut kind);
    new.kind = kind;
    new.of_type = template.of_type.clone();
    new.reno = template.reno;
    true
}

/// Levels ordered from the lowest, without the site's.
fn level_order(model: &Model) -> Vec<ElementId> {
    let mut l: Vec<(ElementId, f64)> = model.levels().map(|(e, l)| (e.id, l.elevation)).collect();
    l.sort_by(|a, b| a.1.total_cmp(&b.1));
    l.into_iter().map(|(id, _)| id).collect()
}

/// The level as far above (or below) `to` as `level` is above `from`:
/// a wall picked up on the ground floor whose top is tied to the floor
/// above is tied, drawn on the first floor, to the one above that.
fn shifted(model: &Model, from: ElementId, to: ElementId, level: ElementId) -> Option<ElementId> {
    if from == to {
        return Some(level);
    }
    let order = level_order(model);
    let pos = |id| order.iter().position(|l| *l == id);
    let i = pos(level)? as isize + pos(to)? as isize - pos(from)? as isize;
    (i >= 0).then(|| order.get(i as usize).copied()).flatten()
}

fn template_level(template: &Element) -> Option<ElementId> {
    match &template.kind {
        ElementKind::Wall(w) => Some(w.level),
        ElementKind::Column(c) => Some(c.level),
        ElementKind::Shaft(s) => Some(s.level),
        _ => None,
    }
}

/// Tops tied to a level follow the floor the new element is drawn on.
fn relevel(model: &Model, template: &Element, kind: &mut ElementKind) {
    let Some(from) = template_level(template) else {
        return;
    };
    let fix = |top: &mut crate::WallTop, to: ElementId| {
        if let crate::WallTop::UpToLevel { level, .. } = top {
            match shifted(model, from, to, *level) {
                Some(l) => *level = l,
                // no level that high: as tall as the template, unconnected
                None => *top = crate::WallTop::Unconnected,
            }
        }
    };
    match kind {
        ElementKind::Wall(w) => fix(&mut w.top, w.level),
        ElementKind::Column(c) => fix(&mut c.top, c.level),
        ElementKind::Shaft(s) => {
            if let Some(t) = s.top_level {
                s.top_level = shifted(model, from, s.level, t);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Level, Wall};
    use glam::DVec2;

    fn two_floors() -> (Model, ElementId, ElementId, ElementId) {
        let mut m = Model::default();
        let ids: Vec<ElementId> = [0.0, 3.0, 6.0]
            .iter()
            .enumerate()
            .map(|(i, z)| {
                let e = Element::new(
                    format!("{i}.NP"),
                    ElementKind::Level(Level {
                        elevation: *z,
                        height: 3.0,
                    }),
                );
                let id = e.id;
                m.insert(e).unwrap();
                id
            })
            .collect();
        (m, ids[0], ids[1], ids[2])
    }

    #[test]
    fn a_new_wall_takes_everything_but_its_line_and_floor() {
        let (m, g, f1, f2) = two_floors();
        let mut w = Wall::new(g, DVec2::ZERO, DVec2::new(5.0, 0.0), 0.44, 2.5);
        w.base_offset = -0.3;
        w.flip = true;
        w.top = crate::WallTop::UpToLevel {
            level: f1,
            offset: -0.25,
        };
        w.bearing = Some(true);
        let t = Element::new("Obvodová", ElementKind::Wall(w));
        let mut n = Element::new(
            "Wall",
            ElementKind::Wall(Wall::new(
                f1,
                DVec2::new(0.0, 4.0),
                DVec2::new(3.0, 4.0),
                0.2,
                3.0,
            )),
        );
        assert!(adopt(&m, &t, &mut n));
        let ElementKind::Wall(nw) = &n.kind else {
            panic!()
        };
        assert_eq!(
            (nw.level, nw.start, nw.end),
            (f1, DVec2::new(0.0, 4.0), DVec2::new(3.0, 4.0))
        );
        assert_eq!((nw.thickness, nw.height, nw.base_offset), (0.44, 2.5, -0.3));
        assert!(nw.flip && nw.bearing == Some(true));
        // its top tied a floor higher, as the template's was
        assert_eq!(
            nw.top,
            crate::WallTop::UpToLevel {
                level: f2,
                offset: -0.25
            }
        );
    }

    #[test]
    fn a_door_is_not_a_window_and_a_line_keeps_its_points() {
        let (m, ..) = two_floors();
        let host = ElementId::new();
        let opening = |host, kind, offset, width, height, sill| crate::Opening {
            host,
            kind,
            offset,
            sill,
            width,
            height,
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
        };
        use crate::OpeningKind::{Door, Window};
        let door = Element::new(
            "D",
            ElementKind::Opening(opening(host, Door, 1.0, 0.9, 2.1, 0.05)),
        );
        let mut win = Element::new(
            "W",
            ElementKind::Opening(opening(host, Window, 2.0, 1.2, 1.5, 0.9)),
        );
        assert!(!adopt(&m, &door, &mut win));
        let other = ElementId::new();
        let mut d2 = Element::new(
            "D",
            ElementKind::Opening(opening(other, Door, 3.0, 0.8, 1.97, 0.0)),
        );
        assert!(adopt(&m, &door, &mut d2));
        let ElementKind::Opening(o) = &d2.kind else {
            panic!()
        };
        assert_eq!((o.host, o.offset), (other, 3.0));
        assert_eq!((o.width, o.height, o.sill), (0.9, 2.1, 0.05));
    }
}
