//! Renovation (rekonštrukcia), as PilonDesigner does it in one project:
//! each building element is existing (it stays), demolished, or new, and a
//! view shows one stage of the work:
//!
//! - *Existing state*: what stands now, the existing and the demolished
//!   elements, plain;
//! - *Demolition*: the same, the demolished ones in yellow, with the holes
//!   for new doors and windows in existing walls marked as demolition too;
//! - *New state*: the existing and the new elements, the new in red;
//! - *All*: everything, coloured, to compare.
//!
//! The colours are the usual ones on Slovak drawings (yellow demolished,
//! red new). An element with no state of its own is new: a project with no
//! renovation views draws as it always did.

use crate::{Element, ElementId, ElementKind, Model};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// What happens to an element in a renovation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Reno {
    /// Stands now and stays.
    Existing,
    /// Stands now and is taken down.
    Demolished,
    /// Built by the project.
    #[default]
    New,
}

impl Reno {
    pub const ALL: [Reno; 3] = [Reno::Existing, Reno::Demolished, Reno::New];

    pub fn label(self) -> &'static str {
        match self {
            Reno::Existing => "Existing (stays)",
            Reno::Demolished => "Demolished",
            Reno::New => "New",
        }
    }

    pub fn is_new(&self) -> bool {
        *self == Reno::New
    }
}

/// Which stage of a renovation a view shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RenoView {
    Existing,
    Demolition,
    New,
    All,
}

impl RenoView {
    pub const ALL: [RenoView; 4] = [
        RenoView::Existing,
        RenoView::Demolition,
        RenoView::New,
        RenoView::All,
    ];

    pub fn label(self) -> &'static str {
        match self {
            RenoView::Existing => "Existing state",
            RenoView::Demolition => "Demolition",
            RenoView::New => "New state",
            RenoView::All => "All (compare)",
        }
    }

    /// Does an element in state `r` show in this stage?
    pub fn shows(self, r: Reno) -> bool {
        match self {
            RenoView::All => true,
            RenoView::Existing | RenoView::Demolition => r != Reno::New,
            RenoView::New => r != Reno::Demolished,
        }
    }

    /// The colour an element in state `r` is washed in (sRGB, 0..1); none
    /// is drawn as usual.
    pub fn colour(self, r: Reno) -> Option<[f32; 3]> {
        match (r, self) {
            (Reno::Demolished, RenoView::Demolition | RenoView::All) => Some(DEMOLISHED),
            (Reno::New, RenoView::New | RenoView::All) => Some(NEW),
            _ => None,
        }
    }
}

/// Yellow, for what is taken down.
pub const DEMOLISHED: [f32; 3] = [0.949, 0.769, 0.059];
/// Red, for what is built.
pub const NEW: [f32; 3] = [0.851, 0.169, 0.169];

/// Does an element of this kind take part in a renovation?
pub fn has_state(kind: &ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Wall(_)
            | ElementKind::Opening(_)
            | ElementKind::Column(_)
            | ElementKind::Beam(_)
            | ElementKind::Slab(_)
            | ElementKind::Roof(_)
            | ElementKind::Stair(_)
            | ElementKind::Railing(_)
            | ElementKind::Object(_)
            | ElementKind::Footing(_)
            | ElementKind::Pipe(_)
            | ElementKind::Ceiling(_)
            | ElementKind::Part(_)
            | ElementKind::FreeMesh(_)
    )
}

/// An element's state: its own, new when it has none.
pub fn state(e: &Element) -> Reno {
    e.reno.unwrap_or_default()
}

/// New doors and windows in walls that stand now: their holes are
/// demolition work.
pub fn new_holes(model: &Model) -> Vec<ElementId> {
    model
        .iter()
        .filter(|e| match &e.kind {
            ElementKind::Opening(o) => {
                state(e).is_new() && model.get(o.host).is_some_and(|h| !state(h).is_new())
            }
            _ => false,
        })
        .map(|e| e.id)
        .collect()
}

/// The elements stage `v` leaves out, with the doors and windows of walls
/// it leaves out. In the demolition stage, new openings in existing walls
/// stay, to show their holes (see [`new_holes`]).
pub fn hidden(model: &Model, v: RenoView) -> HashSet<ElementId> {
    let mut out: HashSet<ElementId> = model
        .iter()
        .filter(|e| has_state(&e.kind) && !v.shows(state(e)))
        .map(|e| e.id)
        .collect();
    if v == RenoView::Demolition {
        for id in new_holes(model) {
            out.remove(&id);
        }
    }
    let gone: Vec<ElementId> = model
        .iter()
        .filter(|e| matches!(&e.kind, ElementKind::Opening(o) if out.contains(&o.host)))
        .map(|e| e.id)
        .collect();
    out.extend(gone);
    out
}

/// `model` without what stage `v` leaves out, openings and all, so a wall
/// shows no hole for a door that is not there yet.
pub fn staged(model: &Model, v: RenoView) -> Model {
    let gone = hidden(model, v);
    let mut m = model.clone();
    if !gone.is_empty() {
        // marks numbered from the whole project, so a window is W02 in
        // every stage, not renumbered when the one before it is not there
        let marks = crate::marks::marks(model);
        let fixed: Vec<Element> = model
            .iter()
            .filter(|e| crate::marks::manual(e).is_none())
            .filter_map(|e| {
                let mut e = e.clone();
                e.mark = Some(marks.get(&e.id)?.clone());
                Some(e)
            })
            .collect();
        for e in fixed {
            let _ = m.replace(e);
        }
        m.retain(|e| match &e.kind {
            ElementKind::Cut(c) => !gone.contains(&c.target) && !gone.contains(&c.cutter),
            _ => !gone.contains(&e.id),
        });
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Level, Opening, OpeningKind, Transaction, Wall};
    use glam::DVec2;

    #[test]
    fn each_stage_shows_its_elements() {
        let mut m = Model::new();
        let l = Element::new(
            "L",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let wall = |reno: Option<Reno>, y: f64| {
            let mut e = Element::new(
                "W",
                ElementKind::Wall(Wall::new(
                    l.id,
                    DVec2::new(0.0, y),
                    DVec2::new(5.0, y),
                    0.3,
                    2.7,
                )),
            );
            e.reno = reno;
            e
        };
        let (old, gone, new) = (
            wall(Some(Reno::Existing), 0.0),
            wall(Some(Reno::Demolished), 3.0),
            wall(None, 6.0),
        );
        let door = Element::new(
            "D",
            ElementKind::Opening(Opening {
                host: old.id,
                kind: OpeningKind::Door,
                offset: 1.0,
                sill: 0.0,
                width: 0.9,
                height: 2.1,
                material: None,
                frame: Default::default(),
                flip_hand: false,
                flip_facing: false,
                no_outer_sill: false,
                no_inner_sill: false,
                threshold: false,
                setback: None,
                reveal: None,
                division: None,
                door: None,
                texts: None,
            }),
        );
        let ids = (old.id, gone.id, new.id, door.id);
        Transaction::new("set up")
            .insert(l)
            .insert(old)
            .insert(gone)
            .insert(new)
            .insert(door)
            .apply(&mut m)
            .unwrap();
        let (o, g, n, d) = ids;
        let h = hidden(&m, RenoView::Existing);
        assert!(!h.contains(&o) && !h.contains(&g) && h.contains(&n) && h.contains(&d));
        // the new door's hole in the existing wall is demolition work
        let h = hidden(&m, RenoView::Demolition);
        assert!(!h.contains(&d) && h.contains(&n));
        let h = hidden(&m, RenoView::New);
        assert!(h.contains(&g) && !h.contains(&n) && !h.contains(&d));
        assert!(hidden(&m, RenoView::All).is_empty());
        assert_eq!(
            RenoView::Demolition.colour(Reno::Demolished),
            Some(DEMOLISHED)
        );
        assert_eq!(RenoView::New.colour(Reno::New), Some(NEW));
        assert_eq!(RenoView::Existing.colour(Reno::Demolished), None);
        assert_eq!(staged(&m, RenoView::Existing).iter().count(), 3);
    }
}
