//! Renovation stages on a plan: what each stage draws and in what colour.
use creabim_core::reno::{self, Reno, RenoView};
use creabim_core::*;
use creabim_geom::plan::PlanItem;
use creabim_geom::section::view_drawing;

/// The sample house with one wall demolished, the rest (and every door
/// and window) existing but one door, which is new: its plan view shows
/// stage `stage`.
fn house(stage: Option<RenoView>) -> (Model, ElementId, ElementId, ElementId) {
    let mut m = sample::sample_house();
    let view = m
        .iter()
        .find(
            |e| matches!(&e.kind, ElementKind::View(v) if matches!(v.kind, ViewKind::Plan { .. })),
        )
        .unwrap()
        .id;
    let door = m
        .iter()
        .find(|e| matches!(&e.kind, ElementKind::Opening(o) if o.kind == OpeningKind::Door))
        .unwrap()
        .clone();
    let ElementKind::Opening(o) = &door.kind else {
        unreachable!()
    };
    let drawn = view_drawing(&m, view).unwrap();
    let gone = m
        .iter()
        .find(|e| {
            matches!(e.kind, ElementKind::Wall(_))
                && e.id != o.host
                && drawn.items.iter().any(|i| i.id() == e.id)
        })
        .map(|e| e.id)
        .unwrap();
    let mut tx = Transaction::new("Renovation");
    for e in m.iter() {
        if !reno::has_state(&e.kind) {
            continue;
        }
        let mut now = e.clone();
        now.reno = Some(if e.id == gone {
            Reno::Demolished
        } else if e.id == door.id {
            Reno::New
        } else {
            Reno::Existing
        });
        tx = tx.replace(e.clone(), now);
    }
    let v = m.get(view).unwrap().clone();
    let mut staged = v.clone();
    if let ElementKind::View(sv) = &mut staged.kind {
        sv.reno = stage;
    }
    tx = tx.replace(v, staged);
    tx.apply(&mut m).unwrap();
    (m, view, gone, door.id)
}

fn tints(m: &Model, view: ElementId, id: ElementId) -> Vec<[f32; 3]> {
    view_drawing(m, view)
        .unwrap()
        .items
        .iter()
        .filter_map(|i| match i {
            PlanItem::Tint { id: t, rgb, .. } if *t == id => Some(*rgb),
            _ => None,
        })
        .collect()
}

fn shown(m: &Model, view: ElementId, id: ElementId) -> bool {
    view_drawing(m, view)
        .unwrap()
        .items
        .iter()
        .any(|i| i.id() == id)
}

#[test]
fn each_stage_draws_its_part_of_the_work() {
    // no stage: everything as usual, no colours
    let (m, v, gone, door) = house(None);
    assert!(shown(&m, v, gone) && shown(&m, v, door));
    assert!(tints(&m, v, gone).is_empty());

    // existing: the demolished wall plain, the new door not there
    let (m, v, gone, door) = house(Some(RenoView::Existing));
    assert!(shown(&m, v, gone) && !shown(&m, v, door));
    assert!(tints(&m, v, gone).is_empty());

    // demolition: the wall yellow, the new door's hole yellow too
    let (m, v, gone, door) = house(Some(RenoView::Demolition));
    assert!(tints(&m, v, gone).contains(&reno::DEMOLISHED));
    assert!(tints(&m, v, door).contains(&reno::DEMOLISHED));

    // new: the wall gone, the door in red
    let (m, v, gone, door) = house(Some(RenoView::New));
    assert!(!shown(&m, v, gone) && shown(&m, v, door));
    assert_eq!(tints(&m, v, door), vec![reno::NEW]);
}

#[test]
fn a_window_keeps_its_mark_in_every_stage() {
    let (m, ..) = house(None);
    let marks = |stage: Option<RenoView>| {
        let staged = match stage {
            Some(s) => reno::staged(&m, s),
            None => m.clone(),
        };
        m.iter()
            .filter(|e| matches!(&e.kind, ElementKind::Opening(_)))
            .filter_map(|e| Some((e.id, marks::mark_of(&staged, e.id)?)))
            .collect::<std::collections::HashMap<_, _>>()
    };
    let all = marks(None);
    for s in RenoView::ALL {
        for (id, m) in marks(Some(s)) {
            assert_eq!(all.get(&id), Some(&m), "{s:?}");
        }
    }
}
