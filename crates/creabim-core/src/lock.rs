//! Locking elements against moving (George 2026-11-22, PilonDesigner's
//! lock): a locked wall, column, beam, door, object… keeps where it is.
//! Its other values (type, height, material…) still change; only its place
//! in plan (and a level's height) is held. [`crate::Document::commit`]
//! refuses any edit that would move a locked element, whatever moved it:
//! a drag, a temporary dimension, Move, a held dimension, a wall joined to
//! it.

use crate::{Element, ElementId, ElementKind, Model};

/// Can this kind of element be locked (has a place to keep)?
pub fn lockable(kind: &ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Wall(_)
            | ElementKind::Column(_)
            | ElementKind::Beam(_)
            | ElementKind::Opening(_)
            | ElementKind::Object(_)
            | ElementKind::Slab(_)
            | ElementKind::Roof(_)
            | ElementKind::Stair(_)
            | ElementKind::Railing(_)
            | ElementKind::Footing(_)
            | ElementKind::Pipe(_)
            | ElementKind::Grid(_)
            | ElementKind::Level(_)
            | ElementKind::Ceiling(_)
    )
}

/// Where `e` is, as numbers that change when it moves or turns: none for
/// kinds that can't be locked.
pub fn placement(model: &Model, e: &Element) -> Option<Vec<f64>> {
    let xy = |v: &[glam::DVec2]| v.iter().flat_map(|p| [p.x, p.y]).collect::<Vec<f64>>();
    Some(match &e.kind {
        ElementKind::Wall(w) => {
            // its location line: a thicker wall or another type keeps it
            let (a, b) = crate::walls::location_line(model, w);
            xy(&[a, b])
        }
        ElementKind::Column(c) => vec![c.position.x, c.position.y, c.rotation],
        ElementKind::Beam(b) => xy(&[b.start, b.end]),
        ElementKind::Opening(o) => vec![o.offset, o.host.0.as_u128() as f64],
        ElementKind::Object(o) => vec![o.position.x, o.position.y, o.rotation],
        ElementKind::Slab(s) => xy(&s.boundary),
        ElementKind::Roof(r) => xy(&r.boundary),
        ElementKind::Stair(s) => vec![s.start.x, s.start.y, s.direction],
        ElementKind::Railing(r) => r.path.iter().flat_map(|p| [p.x, p.y]).collect(),
        ElementKind::Footing(f) => vec![f.start.x, f.start.y, f.end.x, f.end.y, f.rotation],
        ElementKind::Pipe(p) => vec![p.start.x, p.start.y, p.start.z, p.end.x, p.end.y, p.end.z],
        ElementKind::Grid(g) => xy(&[g.start, g.end]),
        ElementKind::Level(l) => vec![l.elevation],
        ElementKind::Ceiling(c) => xy(&c.boundary),
        _ => return None,
    })
}

/// The places of the locked elements in `model`, read before an edit.
pub fn before(model: &Model) -> Vec<(ElementId, Vec<f64>)> {
    model
        .iter()
        .filter(|e| e.locked)
        .filter_map(|e| Some((e.id, placement(model, e)?)))
        .collect()
}

/// The first locked element that `model` has moved from where `before`
/// had it (and that is still locked).
pub fn moved(model: &Model, before: &[(ElementId, Vec<f64>)]) -> Option<ElementId> {
    before.iter().find_map(|(id, was)| {
        let e = model.get(*id).filter(|e| e.locked)?;
        let now = placement(model, e)?;
        let off = now.len() != was.len() || now.iter().zip(was).any(|(a, b)| (a - b).abs() > 1e-7);
        off.then_some(*id)
    })
}

#[cfg(test)]
mod tests {
    use crate::{Document, ElementKind, Transaction};

    #[test]
    fn a_locked_wall_keeps_its_place_and_its_other_values_change() {
        let mut doc = Document::from_model(crate::sample::sample_house());
        let wall = doc
            .model()
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Wall(_)))
            .unwrap()
            .clone();
        doc.edit(wall.id, "Lock", |e| e.locked = true).unwrap();
        let locked = doc.model().get(wall.id).unwrap().clone();
        // moving it is refused, and nothing changes
        let mut moved = locked.clone();
        if let ElementKind::Wall(w) = &mut moved.kind {
            w.start.x += 0.5;
            w.end.x += 0.5;
        }
        let err = doc.commit(Transaction::new("Move").replace(locked.clone(), moved));
        assert!(matches!(err, Err(crate::ModelError::Locked(id)) if id == wall.id));
        assert_eq!(doc.model().get(wall.id), Some(&locked));
        // its height still changes
        doc.edit(wall.id, "Height", |e| {
            if let ElementKind::Wall(w) = &mut e.kind {
                w.height += 0.3;
            }
        })
        .unwrap();
        // unlocked, it moves
        doc.edit(wall.id, "Unlock", |e| e.locked = false).unwrap();
        doc.edit(wall.id, "Move", |e| {
            if let ElementKind::Wall(w) = &mut e.kind {
                w.start.x += 0.5;
                w.end.x += 0.5;
            }
        })
        .unwrap();
    }
}
