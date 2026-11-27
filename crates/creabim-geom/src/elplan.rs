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
        let moved = match &e.kind {
            ElementKind::Object(o) => o.mark.unwrap_or(DVec2::ZERO),
            _ => DVec2::ZERO,
        };
        let at = DVec2::new(hi.x + (w / 2.0 + 1.0) * mm, (lo.y + hi.y) / 2.0) + moved;
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

/// Circuits' colours in the electrician's plan, by their order.
pub const CIRCUIT_COLOURS: [[f32; 3]; 12] = {
    const fn c(x: u32) -> [f32; 3] {
        [
            ((x >> 16) & 0xff) as f32 / 255.0,
            ((x >> 8) & 0xff) as f32 / 255.0,
            (x & 0xff) as f32 / 255.0,
        ]
    }
    [
        c(0xe11d48),
        c(0x2563eb),
        c(0x16a34a),
        c(0xd97706),
        c(0x9333ea),
        c(0x0891b2),
        c(0xdb2777),
        c(0x65a30d),
        c(0x7c3aed),
        c(0xea580c),
        c(0x0d9488),
        c(0x4f46e5),
    ]
};

/// A quadratic arc from `a` to `b` bowed `bend` of its length to the
/// left, as wiring is drawn by hand.
fn arc(a: DVec2, b: DVec2, bend: f64) -> Vec<DVec2> {
    let d = b - a;
    if d.length() < 1e-6 {
        return vec![];
    }
    let c = (a + b) / 2.0 + d.perp() * bend;
    (0..=12)
        .map(|i| {
            let t = i as f64 / 12.0;
            let u = 1.0 - t;
            a * u * u + c * 2.0 * u * t + b * t * t
        })
        .collect()
}

/// The wiring of each circuit on `level` as hand-drawn arcs, as
/// PilonDesigner's `wiringArcs`: the shortest tree joining its devices
/// and the feed from its board to the nearest of them, each circuit in
/// its colour.
pub fn wiring(model: &Model, level: ElementId, mm: f64) -> Vec<PlanItem> {
    let devs = el::devices(model);
    let mut out = vec![];
    for (k, (ce, c)) in el::circuits(model).into_iter().enumerate() {
        let rgb = CIRCUIT_COLOURS[k % CIRCUIT_COLOURS.len()];
        let at: Vec<DVec2> = devs
            .iter()
            .filter(|(e, o, kind)| {
                o.level == level
                    && *kind != DeviceKind::Board
                    && el::circuit_of(model, e.id) == Some(ce.id)
            })
            .map(|(_, o, _)| o.position)
            .collect();
        if at.is_empty() {
            continue;
        }
        let mut draw = |pts: Vec<DVec2>| {
            if !pts.is_empty() {
                crate::heatplan::strips(&mut out, ce.id, &pts, rgb, None, mm);
            }
        };
        // Prim: the shortest tree over the devices
        let mut inside = vec![0];
        let mut rest: Vec<usize> = (1..at.len()).collect();
        while !rest.is_empty() {
            let (mut best, mut bi, mut bj) = (f64::MAX, 0, 0);
            for &i in &inside {
                for (jj, &j) in rest.iter().enumerate() {
                    let d = at[i].distance(at[j]);
                    if d < best {
                        (best, bi, bj) = (d, i, jj);
                    }
                }
            }
            let j = rest.remove(bj);
            draw(arc(at[bi], at[j], 0.15));
            inside.push(j);
        }
        let board = c.board.and_then(|b| {
            devs.iter()
                .find(|(e, o, _)| e.id == b && o.level == level)
                .map(|(_, o, _)| o.position)
        });
        if let Some(b) = board {
            let near = at
                .iter()
                .copied()
                .min_by(|x, y| x.distance(b).total_cmp(&y.distance(b)))
                .unwrap_or(b);
            draw(arc(b, near, 0.1));
        }
    }
    out
}

/// Is `id` a device of the wiring?
pub fn is_device(model: &Model, id: ElementId) -> bool {
    model.get(id).is_some_and(|e| {
        matches!(e.kind, ElementKind::Object(_)) && el::device_kind(model, e).is_some()
    })
}

/// An electrical device that only the electrical drawings show: a
/// socket, switch, light or low-current device. Appliances (a fridge, a
/// cooker, a washing machine) are furniture to the architect, and the
/// distribution board is drawn in his plans too (George 2026-11-27).
pub fn is_wiring_device(model: &Model, id: ElementId) -> bool {
    model.get(id).is_some_and(|e| {
        matches!(e.kind, ElementKind::Object(_))
            && el::device_kind(model, e)
                .is_some_and(|k| !matches!(k, el::DeviceKind::Appliance | el::DeviceKind::Board))
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
    fn each_circuit_is_wired_as_a_tree_and_fed_from_its_board() {
        let m = starter_with_circuits();
        let level = el::devices(&m)
            .iter()
            .find(|d| d.2 == DeviceKind::Socket)
            .unwrap()
            .1
            .level;
        let items = wiring(&m, level, 0.05);
        assert!(!items.is_empty());
        for (ce, _) in el::circuits(&m) {
            let n = el::devices(&m)
                .iter()
                .filter(|(e, o, k)| {
                    o.level == level
                        && *k != DeviceKind::Board
                        && el::circuit_of(&m, e.id) == Some(ce.id)
                })
                .count();
            let drawn = items.iter().filter(|it| it.id() == ce.id).count();
            if n > 0 {
                // n − 1 arcs and the feed, 12 strips each
                assert!(drawn >= (n - 1) * 12, "{n} {drawn}");
            }
        }
        assert_eq!(arc(DVec2::ZERO, DVec2::X, 0.15).len(), 13);
        assert!((arc(DVec2::ZERO, DVec2::X, 0.15)[6].y - 0.075).abs() < 1e-9);
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

    /// The architect's plan draws the kitchen's appliances (a fridge, a
    /// cooker) but not the sockets, switches and lights.
    #[test]
    fn the_architects_plan_keeps_appliances_and_leaves_wiring_out() {
        let m = creabim_core::starter::starter_house();
        let named = |n: &str| {
            m.iter()
                .find(|e| {
                    matches!(&e.kind, ElementKind::Object(_))
                        && creabim_core::objects::type_of(&m, e)
                            .and_then(|t| t.catalogue.clone())
                            .is_some_and(|k| k.ends_with(n))
                })
                .unwrap()
                .id
        };
        let (fridge, cooker) = (named("fridge-freezer"), named("cooker-600"));
        let level = m.get(fridge).map(|e| match &e.kind {
            ElementKind::Object(o) => o.level,
            _ => unreachable!(),
        });
        let plan = m
            .iter()
            .find(|e| {
                matches!(&e.kind, ElementKind::View(v)
                    if matches!(v.kind, creabim_core::ViewKind::Plan { level: l, .. } if Some(l) == level)
                        && v.trade.is_none()
                        && creabim_core::viewmodes::DisplayMode::of(v)
                            != creabim_core::viewmodes::DisplayMode::Services)
            })
            .unwrap()
            .id;
        let d = crate::section::view_drawing(&m, plan).unwrap();
        assert!(d.items.iter().any(|it| it.id() == fridge));
        assert!(d.items.iter().any(|it| it.id() == cooker));
        assert!(!d.items.iter().any(|it| is_wiring_device(&m, it.id())));
    }
}
