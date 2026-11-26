//! The labels of a drawn schedule that can be dragged (George
//! 2026-11-26): the axonometry of the water supply, the drain sections.
//! Each label is of an element (a pipe's size, a fixture's mark, a
//! stack's mark) and counted among that element's labels in the order
//! the drawing writes them; where it was dragged is kept in its view
//! ([`creabim_core::LabelMove`]). A label is a [`PlanItem::Text`] with its
//! element's id; the drawing's other texts carry the view's id, so the
//! drawing's labels can be found again from its items.

use crate::paper::Anchor;
use crate::plan::PlanItem;
use creabim_core::{ElementId, ElementKind, LabelMove, Model};
use glam::DVec2;
use std::collections::HashMap;

/// The labels written so far and where they were dragged.
pub struct Labels {
    view: ElementId,
    moves: Vec<LabelMove>,
    count: HashMap<ElementId, u32>,
}

impl Labels {
    /// The labels of the view `view` in `model`.
    pub fn of(model: &Model, view: ElementId) -> Self {
        let moves = match model.get(view).map(|e| &e.kind) {
            Some(ElementKind::View(v)) => v.label_moves.clone(),
            _ => vec![],
        };
        Labels {
            view,
            moves,
            count: HashMap::new(),
        }
    }

    /// The next label of `of`: how far it was dragged, paper mm.
    pub fn next(&mut self, of: ElementId) -> DVec2 {
        let n = self.count.entry(of).or_default();
        let i = *n;
        *n += 1;
        self.moves
            .iter()
            .find(|m| m.of == of && m.n == i)
            .map_or(DVec2::ZERO, |m| m.by)
    }

    /// Writes the next label of `of` at `at` (where it falls by itself),
    /// moved where it was dragged; returns how far it was moved, for
    /// what goes with it (the circle around a stack's mark).
    #[allow(clippy::too_many_arguments)]
    pub fn write(
        &mut self,
        items: &mut Vec<PlanItem>,
        of: ElementId,
        at: DVec2,
        text: &str,
        size: f64,
        align: Anchor,
        rgb: Option<[f32; 3]>,
    ) -> DVec2 {
        let by = if of == self.view {
            DVec2::ZERO
        } else {
            self.next(of)
        };
        items.push(PlanItem::Text {
            id: of,
            at: at + by,
            text: text.to_string(),
            size,
            align,
            mm: 1.0,
            rgb,
            bold: false,
        });
        by
    }
}

/// A label found in a drawn schedule's items: what it is of, its number
/// among that element's labels, and the corners of its text.
pub fn found(view: ElementId, items: &[PlanItem]) -> Vec<(ElementId, u32, DVec2, DVec2)> {
    let mut count: HashMap<ElementId, u32> = HashMap::new();
    let mut out = vec![];
    for it in items {
        let PlanItem::Text { id, .. } = it else {
            continue;
        };
        if *id == view {
            continue;
        }
        let n = count.entry(*id).or_default();
        let pts = it.extent();
        if let Some(first) = pts.first() {
            let (lo, hi) = pts
                .iter()
                .fold((*first, *first), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
            out.push((*id, *n, lo, hi));
        }
        *n += 1;
    }
    out
}

/// Is the view a drawn schedule whose labels can be dragged?
pub fn movable(model: &Model, view: ElementId) -> bool {
    use creabim_core::{ScheduleCategory as C, ViewKind};
    matches!(
        model.get(view).map(|e| &e.kind),
        Some(ElementKind::View(v)) if matches!(
            v.kind,
            ViewKind::Schedule { category: C::WaterAxonometry | C::DrainSections, .. }
        )
    )
}
