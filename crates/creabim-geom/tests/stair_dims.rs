//! Dimensions to a stair's steps follow it and drive it (George 2026-11-24).

use creabim_core::{
    Dimension, DimensionKind, Element, ElementKind, HostPoint, Level, Model, Stair, View,
};
use creabim_geom::annotate;
use glam::DVec2;

fn setup() -> (Model, Element, Element) {
    let l0 = Element::new(
        "L0",
        ElementKind::Level(Level {
            elevation: 0.0,
            height: 2.8,
        }),
    );
    let mut s = Stair::new(l0.id, DVec2::ZERO, 0.0);
    s.width = 1.0;
    s.tread = 0.27;
    s.max_riser = 0.175;
    s.height = Some(2.8);
    let st = Element::new("St", ElementKind::Stair(s));
    let v = Element::new("Plan", ElementKind::View(View::plan(l0.id)));
    let m = Model::from_elements([l0, st.clone(), v.clone()]).unwrap();
    (m, st, v)
}

fn dim(view: creabim_core::ElementId, a: HostPoint, b: HostPoint) -> Dimension {
    serde_json::from_value(serde_json::json!({"view": view, "a": a, "b": b, "offset": 0.5}))
        .unwrap()
}

fn view(v: &Element) -> &View {
    let ElementKind::View(v) = &v.kind else {
        unreachable!()
    };
    v
}

#[test]
fn a_dimension_between_nosings_follows_the_going_and_sets_it() {
    let (mut m, st, ve) = setup();
    let v = view(&ve);
    // the first nosing's side to the last's, on the same side
    let a = annotate::attach(&m, v, Some(st.id), DVec2::new(0.0, -0.5));
    let b = annotate::attach(&m, v, Some(st.id), DVec2::new(15.0 * 0.27, -0.5));
    assert!(
        a.step.is_some() && b.step.is_some() && b.from_end,
        "{a:?} {b:?}"
    );
    let d = dim(ve.id, a, b);
    let pa = annotate::resolve(&m, v, &d.a).unwrap();
    let pb = annotate::resolve(&m, v, &d.b).unwrap();
    assert!((pa.distance(pb) - 15.0 * 0.27).abs() < 1e-9);
    // the going made wider: the dimension follows its last nosing
    let mut now = st.clone();
    if let ElementKind::Stair(s) = &mut now.kind {
        s.tread = 0.28;
    }
    creabim_core::Transaction::new("t")
        .replace(st.clone(), now.clone())
        .apply(&mut m)
        .unwrap();
    let pb = annotate::resolve(&m, v, &d.b).unwrap();
    assert!((pb.x - 15.0 * 0.28).abs() < 1e-9, "{pb:?}");
    // typed on the dimension: the going follows
    let tx = annotate::driven(&m, v, &d, 0, 15.0 * 0.26, &[]).unwrap();
    tx.apply(&mut m).unwrap();
    let ElementKind::Stair(s) = &m.get(st.id).unwrap().kind else {
        unreachable!()
    };
    assert!((s.tread - 0.26).abs() < 1e-9, "{}", s.tread);
    assert_eq!(d.kind, DimensionKind::Linear);
}

#[test]
fn a_dimension_across_a_nosing_sets_the_width() {
    let (mut m, st, ve) = setup();
    let v = view(&ve);
    let a = annotate::attach(&m, v, Some(st.id), DVec2::new(0.27, -0.5));
    let b = annotate::attach(&m, v, Some(st.id), DVec2::new(0.27, 0.5));
    let d = dim(ve.id, a, b);
    let tx = annotate::driven(&m, v, &d, 0, 1.2, &[]).unwrap();
    tx.apply(&mut m).unwrap();
    let ElementKind::Stair(s) = &m.get(st.id).unwrap().kind else {
        unreachable!()
    };
    assert!((s.width - 1.2).abs() < 1e-9, "{}", s.width);
}
