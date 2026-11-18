//! The electrician's plan: a services drawing of the trade Electrical
//! draws the small devices (sockets, switches, detectors) no smaller
//! than a symbol reads on paper, and writes at each device the circuit
//! that feeds it, at a board its name.

use crate::plan::PlanItem;
use creabim_core::electrical::{self as el, DeviceKind};
use creabim_core::{ElementId, ElementKind, Model};
use glam::DVec2;

/// The least a small device's symbol measures on paper, mm.
pub const SYMBOL_MIN: f64 = 3.5;

fn bounds(items: &[PlanItem], id: ElementId) -> Option<(DVec2, DVec2)> {
    let mut pts = items
        .iter()
        .filter(|it| it.id() == id)
        .flat_map(|it| it.extent());
    let first = pts.next()?;
    Some(pts.fold((first, first), |(lo, hi), p| (lo.min(p), hi.max(p))))
}

/// Grow the symbols of the small devices among `items` about the point
/// they sit at, so that none is smaller than `SYMBOL_MIN` on a drawing
/// with `mm` metres to a paper millimetre.
pub fn enlarge(model: &Model, items: &mut [PlanItem], mm: f64) {
    for (e, o, kind) in el::devices(model) {
        if !matches!(
            kind,
            DeviceKind::Socket | DeviceKind::Switch | DeviceKind::Weak
        ) {
            continue;
        }
        let Some((lo, hi)) = bounds(items, e.id) else {
            continue;
        };
        let size = (hi - lo).max_element();
        if size <= 0.0 {
            continue;
        }
        let k = SYMBOL_MIN * mm / size;
        if k <= 1.0 {
            continue;
        }
        let c = o.position;
        let grow = |p: &mut DVec2| *p = c + (*p - c) * k;
        for it in items.iter_mut().filter(|it| it.id() == e.id) {
            match it {
                PlanItem::Polygon { points, .. }
                | PlanItem::Fill { points, .. }
                | PlanItem::Tint { points, .. } => points.iter_mut().for_each(grow),
                PlanItem::Line { a, b, .. } => {
                    grow(a);
                    grow(b);
                }
                PlanItem::Arc { center, radius, .. } => {
                    grow(center);
                    *radius *= k;
                }
                _ => {}
            }
        }
    }
}

/// What is written at a device: its circuit's name, a board's own.
pub fn mark(model: &Model, id: ElementId) -> Option<String> {
    let e = model.get(id)?;
    if el::device_kind(model, e)? == DeviceKind::Board {
        return Some(e.name.clone()).filter(|n| !n.is_empty());
    }
    let c = el::circuit_of(model, id)?;
    model
        .get(c)
        .map(|c| c.name.clone())
        .filter(|n| !n.is_empty())
}

/// The marks of the devices drawn among `items`, each beside its symbol.
pub fn marks(model: &Model, items: &[PlanItem], mm: f64) -> Vec<PlanItem> {
    let look = crate::annotation::TextLook {
        size: 2.2,
        rgb: Some(crate::mep::LABEL_GREEN),
        ..Default::default()
    };
    let mut out = vec![];
    for (e, _, _) in el::devices(model) {
        let (Some(text), Some((lo, hi))) = (mark(model, e.id), bounds(items, e.id)) else {
            continue;
        };
        let w = look.width(&text);
        let at = DVec2::new(hi.x + (w / 2.0 + 1.0) * mm, (lo.y + hi.y) / 2.0);
        out.push(PlanItem::Annotation {
            id: e.id,
            item: Box::new(crate::annotation::Annotation {
                shape: crate::annotation::Shape::Along { at, angle: 0.0 },
                text,
                look,
                mm,
            }),
        });
    }
    out
}

/// Is `id` a device of the wiring?
pub fn is_device(model: &Model, id: ElementId) -> bool {
    model.get(id).is_some_and(|e| {
        matches!(e.kind, ElementKind::Object(_)) && el::device_kind(model, e).is_some()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::Transaction;

    fn starter_with_circuits() -> Model {
        let mut m = creabim_core::starter::starter_house();
        let made = creabim_services::circuits::propose(
            &m,
            8,
            creabim_services::electrical::Method::default(),
        )
        .unwrap();
        let mut tx = Transaction::new("circuits");
        for e in made {
            tx = tx.insert(e);
        }
        tx.apply(&mut m).unwrap();
        m
    }

    #[test]
    fn a_device_is_marked_with_its_circuit_and_a_board_with_its_name() {
        let m = starter_with_circuits();
        let devs = el::devices(&m);
        let socket = devs.iter().find(|d| d.2 == DeviceKind::Socket).unwrap();
        let circuit = el::circuit_of(&m, socket.0.id).unwrap();
        assert_eq!(
            mark(&m, socket.0.id),
            Some(m.get(circuit).unwrap().name.clone())
        );
        let board = devs.iter().find(|d| d.2 == DeviceKind::Board).unwrap();
        assert_eq!(mark(&m, board.0.id), Some(board.0.name.clone()));
    }

    #[test]
    fn small_symbols_grow_on_a_small_scale_and_keep_their_place() {
        let m = creabim_core::starter::starter_house();
        let devs = el::devices(&m);
        let (e, o, _) = devs.iter().find(|d| d.2 == DeviceKind::Switch).unwrap();
        let mut items = crate::objects::object_plan(&m, e, 1.2);
        let (lo, hi) = bounds(&items, e.id).unwrap();
        let was = (hi - lo).max_element();
        // 1:50: as modelled
        enlarge(&m, &mut items, 0.05);
        let (lo, hi) = bounds(&items, e.id).unwrap();
        assert!(((hi - lo).max_element() - was).abs() < 1e-9 || was < SYMBOL_MIN * 0.05);
        // 1:200: 3.5 mm on paper
        enlarge(&m, &mut items, 0.2);
        let (lo, hi) = bounds(&items, e.id).unwrap();
        assert!(((hi - lo).max_element() - SYMBOL_MIN * 0.2).abs() < 1e-6);
        // still touching the point it sits at
        let near = items
            .iter()
            .flat_map(|it| it.extent())
            .map(|p| p.distance(o.position))
            .fold(f64::MAX, f64::min);
        assert!(near < 1e-6, "{near}");
        assert!(is_device(&m, e.id));
    }
}
