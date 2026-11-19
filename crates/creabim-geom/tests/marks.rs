//! Marks in tags and schedules, tag and note leaders, spot levels,
//! dimension breaks and the layout of level heads in sections.

use creabim_core::*;
use creabim_geom::annotate::{break_dimensions, tag_geometry, tag_text};
use creabim_geom::annotation::{Shape, TextLook};
use creabim_geom::section::{spread, view_drawing};
use creabim_geom::PlanItem;
use glam::DVec2;

fn named(m: &Model, name: &str) -> ElementId {
    m.iter().find(|e| e.name == name).unwrap().id
}

fn first(m: &Model, f: impl Fn(&ElementKind) -> bool) -> ElementId {
    m.iter().find(|e| f(&e.kind)).unwrap().id
}

fn is_door(k: &ElementKind) -> bool {
    matches!(k, ElementKind::Opening(o) if o.kind == OpeningKind::Door)
}

fn add(doc: &mut Document, e: Element) -> ElementId {
    let id = e.id;
    doc.commit(Transaction::new("Add").insert(e)).unwrap();
    id
}

#[test]
fn tags_and_schedules_read_typed_marks_and_the_rules() {
    let mut doc = Document::from_model(sample::sample_house());
    let door = first(doc.model(), is_door);
    // a door's tag: its hand (STN 01 3420) over its mark
    let (text, round) = tag_text(doc.model(), door);
    assert!(round && text.ends_with("\nD01"), "{text}");
    assert!(text.starts_with("L\n") || text.starts_with("P\n"), "{text}");
    doc.edit(door, "Mark", |e| e.mark = Some("FD-1".into()))
        .unwrap();
    assert!(tag_text(doc.model(), door).0.ends_with("\nFD-1"));
    // level-based rules renumber the rest
    let tx = marks::set_rules(doc.model(), MarkRules::level_based());
    doc.commit(tx).unwrap();
    let other = doc
        .model()
        .iter()
        .find(|e| is_door(&e.kind) && e.id != door)
        .unwrap()
        .id;
    assert!(tag_text(doc.model(), other)
        .0
        .lines()
        .last()
        .unwrap()
        .starts_with("D-"));
    // the door schedule shows the same marks
    let table = creabim_geom::schedule::schedule_by(doc.model(), ScheduleCategory::Doors, false);
    let marks: Vec<&str> = table.rows.iter().map(|r| r.cells[0].as_str()).collect();
    assert!(marks.contains(&"FD-1"), "{marks:?}");
    let other_tag = tag_text(doc.model(), other).0;
    assert!(marks.contains(&other_tag.lines().last().unwrap()));
    // a wall with a mark reads it, and the wall schedule grows a column
    let wall = first(doc.model(), |k| matches!(k, ElementKind::Wall(_)));
    let thickness = tag_text(doc.model(), wall).0;
    doc.edit(wall, "Mark", |e| e.mark = Some("WT1".into()))
        .unwrap();
    assert_eq!(tag_text(doc.model(), wall).0, "WT1");
    let walls = creabim_geom::schedule::schedule_by(doc.model(), ScheduleCategory::Walls, false);
    assert_eq!(walls.columns[1].title, "Mark");
    doc.undo();
    assert_eq!(tag_text(doc.model(), wall).0, thickness);
}

#[test]
fn tag_leaders_bend_at_their_elbow_and_end_in_their_mark() {
    let look = TextLook {
        leader_end: Some(Terminator::Dot),
        ..TextLook::default()
    };
    let anchor = DVec2::new(0.0, -2.0);
    let elbow = DVec2::new(1.0, -1.0);
    let g = tag_geometry(anchor, DVec2::ZERO, Some(elbow), "D01", true, &look, 0.01);
    assert_eq!(g.leader.len(), 3);
    assert_eq!(g.leader[1], elbow);
    assert_eq!(*g.leader.last().unwrap(), anchor);
    assert_eq!(g.ends.fills.len(), 1, "a dot at the end");
    // no end mark: a plain line, as tags always had
    let plain = tag_geometry(
        anchor,
        DVec2::ZERO,
        None,
        "D01",
        true,
        &TextLook::default(),
        0.01,
    );
    assert_eq!(plain.leader.len(), 2);
    assert!(plain.ends.fills.is_empty() && plain.ends.thin.is_empty());

    // a tag's own leader end and elbow reach its drawing
    let mut doc = Document::from_model(sample::sample_house());
    let plan = named(doc.model(), "Ground Floor plan");
    // (a door's or window's mark has a straight leader, square to its wall)
    let wall = first(doc.model(), |k| matches!(k, ElementKind::Wall(_)));
    let mut tag = Tag::new(plan, wall, DVec2::new(2.0, 2.0));
    tag.elbow = Some(DVec2::new(2.0, 0.0));
    tag.end = Some(DVec2::new(0.1, 0.0));
    tag.leader_end = Some(Terminator::Arrow);
    let id = add(&mut doc, Element::new("Tag", ElementKind::Tag(tag)));
    let d = view_drawing(doc.model(), plan).unwrap();
    let (anchor, at, elbow, look) = d
        .items
        .iter()
        .find_map(|i| match i {
            PlanItem::Tag {
                id: t,
                anchor,
                at,
                elbow,
                look,
                ..
            } if *t == id => Some((*anchor, *at, *elbow, *look)),
            _ => None,
        })
        .unwrap();
    assert!((at - anchor - DVec2::new(1.9, 2.0)).length() < 1e-9);
    assert!((elbow.unwrap() - anchor - DVec2::new(1.9, 0.0)).length() < 1e-9);
    assert_eq!(look.leader_end, Some(Terminator::Arrow));
}

#[test]
fn notes_bend_their_leaders_at_an_elbow() {
    let mut doc = Document::from_model(sample::sample_house());
    let plan = named(doc.model(), "Ground Floor plan");
    let mut n = Note::new(plan, DVec2::new(5.0, 5.0), "Note");
    n.leader = Some(HostPoint::free(DVec2::new(0.0, 0.0)));
    n.elbow = Some(DVec2::new(2.0, 1.0));
    n.leader_end = Some(Terminator::DotOpen);
    let id = add(&mut doc, Element::new("Note", ElementKind::Note(n)));
    let d = view_drawing(doc.model(), plan).unwrap();
    let item = d
        .items
        .iter()
        .find_map(|i| match i {
            PlanItem::Annotation { id: n, item } if *n == id => Some(item.clone()),
            _ => None,
        })
        .unwrap();
    let m = item.marks(0.1);
    let leader = m.lines.last().unwrap();
    assert!(leader.contains(&DVec2::new(2.0, 1.0)), "{leader:?}");
    assert_eq!(*leader.last().unwrap(), DVec2::ZERO);
    assert!(
        m.arrows.is_empty() && !m.ends.thin.is_empty(),
        "an open dot"
    );
}

#[test]
fn spot_levels_read_heights_from_their_pattern() {
    let mut doc = Document::from_model(sample::sample_house());
    let model = doc.model().clone();
    let plan = named(&model, "Ground Floor plan");
    let (level, ground) = model
        .levels()
        .map(|(e, l)| (e.id, l.elevation))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap();
    // a level marker in the plan: the plan's level
    let free = HostPoint::free(DVec2::new(-3.0, -3.0));
    let marker = Dimension::spot(
        plan,
        free,
        HostPoint::free(DVec2::new(-1.0, -2.0)),
        Some("{m} FFL".into()),
    );
    let marker = add(
        &mut doc,
        Element::new("Spot", ElementKind::Dimension(marker)),
    );
    // a section: the point's own height, 0.45 up
    let section = View::section(DVec2::new(-5.0, 2.0), DVec2::new(20.0, 2.0), 20.0, true);
    let section = add(&mut doc, Element::new("S", ElementKind::View(section)));
    let p = HostPoint::free(DVec2::new(1.0, ground + 0.45));
    let spot = Dimension::spot(
        section,
        p,
        HostPoint::free(DVec2::new(3.0, 1.0)),
        Some("{m} FFL ({level})".into()),
    );
    let spot = add(&mut doc, Element::new("Spot", ElementKind::Dimension(spot)));
    let text = |view: ElementId, id: ElementId| {
        view_drawing(doc.model(), view)
            .unwrap()
            .items
            .into_iter()
            .find_map(|i| match i {
                PlanItem::Annotation { id: n, item } if n == id => {
                    assert!(matches!(item.shape, Shape::Spot { .. }));
                    Some(item.text)
                }
                _ => None,
            })
            .unwrap()
    };
    let level_name = doc.model().get(level).unwrap().name.clone();
    assert_eq!(
        text(plan, marker),
        format!("{} FFL", annotation::spot_text(doc.model(), "{m}", ground))
    );
    assert_eq!(
        text(section, spot),
        format!(
            "{} FFL ({level_name})",
            annotation::spot_text(doc.model(), "{m}", ground + 0.45)
        )
    );
    assert_eq!(annotation::spot_text(doc.model(), "{m}", 0.45), "+0.45");
    assert_eq!(annotation::spot_text(doc.model(), "{m}", 0.0), "±0.00");
    assert_eq!(annotation::spot_text(doc.model(), "{mm}", -0.45), "-450");
    // older dimensions read without a pattern
    let old = r#"{"view":"236746a3-3a36-452f-801d-b4b755309b62","a":{"host":null,"local":[0.0,0.0],"from_end":false},"b":{"host":null,"local":[3.0,0.0],"from_end":false},"offset":-1.5}"#;
    let d: Dimension = serde_json::from_str(old).unwrap();
    assert_eq!(d.pattern, None);
}

#[test]
fn dimension_lines_break_where_tags_sit_across_them() {
    let look = TextLook {
        breaks: true,
        ..TextLook::default()
    };
    let tag = PlanItem::Tag {
        id: ElementId::new(),
        anchor: DVec2::new(5.0, 1.0),
        at: DVec2::new(5.0, 1.0),
        elbow: None,
        text: "D01".into(),
        round: true,
        look: TextLook::default(),
    };
    let dim = |look: TextLook| PlanItem::Dimension {
        id: ElementId::new(),
        a: DVec2::ZERO,
        b: DVec2::new(10.0, 0.0),
        offset: 1.0,
        text: "10000".into(),
        look,
        witness: None,
        breaks: vec![],
        below: None,
        place: Default::default(),
    };
    let mut items = vec![tag, dim(look), dim(TextLook::default())];
    break_dimensions(&mut items, 0.1);
    let PlanItem::Dimension { breaks, .. } = &items[1] else {
        unreachable!()
    };
    assert_eq!(breaks.len(), 1);
    let [t0, t1] = breaks[0];
    assert!(t0 < 0.5 && t1 > 0.5 && t1 - t0 < 0.2, "{t0} {t1}");
    let PlanItem::Dimension { breaks, .. } = &items[2] else {
        unreachable!()
    };
    assert!(breaks.is_empty(), "a style without breaks runs through");
    // the line is drawn in two pieces
    let mut g = creabim_geom::annotate::dimension_geometry(
        DVec2::ZERO,
        DVec2::new(10.0, 0.0),
        1.0,
        None,
        &look,
        0.1,
    );
    g.break_line(&[[t0, t1]]);
    let along: Vec<_> = g
        .lines
        .iter()
        .filter(|(p, q)| p.y == 1.0 && q.y == 1.0)
        .collect();
    assert_eq!(along.len(), 2);
}

#[test]
fn close_level_heads_spread_apart_and_follow_the_view() {
    // two levels 0.2 m apart, one well clear
    let ys = spread(&[0.0, 0.2, 3.0], 0.7);
    assert!((ys[1] - ys[0] - 0.7).abs() < 1e-9);
    assert!(
        ((ys[0] + ys[1]) / 2.0 - 0.1).abs() < 1e-9,
        "centred on the pair"
    );
    assert_eq!(ys[2], 3.0);

    let mut doc = Document::from_model(sample::sample_house());
    let ground = doc
        .model()
        .levels()
        .min_by(|a, b| a.1.elevation.total_cmp(&b.1.elevation))
        .unwrap()
        .0
        .id;
    let near = add(
        &mut doc,
        Element::new(
            "Mezzanine",
            ElementKind::Level(Level {
                elevation: 0.2,
                height: 3.0,
            }),
        ),
    );
    let section = View::section(DVec2::new(-5.0, 2.0), DVec2::new(20.0, 2.0), 20.0, true);
    let view = add(&mut doc, Element::new("S", ElementKind::View(section)));
    let heads = |m: &Model| -> Vec<(ElementId, DVec2, Option<DVec2>)> {
        view_drawing(m, view)
            .unwrap()
            .items
            .into_iter()
            .filter_map(|i| match i {
                PlanItem::Annotation { id, item } => match item.shape {
                    Shape::Level { at, line_end, .. } => Some((id, at, line_end)),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    };
    let h = heads(doc.model());
    let of = |h: &[(ElementId, DVec2, Option<DVec2>)], id| h.iter().find(|x| x.0 == id).copied();
    let (_, a, ea) = of(&h, ground).unwrap();
    let (_, b, eb) = of(&h, near).unwrap();
    assert!(ea.is_some() && eb.is_some(), "both get a dog-leg");
    assert!(b.y - a.y > 0.5, "spread apart: {a} {b}");
    // drag one, hide the other
    doc.edit(view, "Heads", |e| {
        if let ElementKind::View(v) = &mut e.kind {
            v.level_heads.insert(
                near,
                LevelHead {
                    hidden: false,
                    offset: Some(DVec2::new(1.0, 2.0)),
                },
            );
            v.level_heads.insert(
                ground,
                LevelHead {
                    hidden: true,
                    offset: None,
                },
            );
        }
    })
    .unwrap();
    let h = heads(doc.model());
    assert!(of(&h, ground).is_none(), "hidden in this view");
    let (_, b, eb) = of(&h, near).unwrap();
    assert!((b - eb.unwrap() - DVec2::new(1.0, 2.0)).length() < 1e-9);
    // the view's settings survive saving, and older views read without them
    let e = doc.model().get(view).unwrap();
    let back: Element = serde_json::from_str(&serde_json::to_string(e).unwrap()).unwrap();
    assert_eq!(&back, e);
    let old = r#"{"kind":{"Plan":{"level":"9801fb10-8165-4b45-b92c-b50f14e9679d","cut_height":1.2}},"scale":100.0,"style":"HiddenLine"}"#;
    let v: View = serde_json::from_str(old).unwrap();
    assert!(v.level_heads.is_empty());
    assert!(!serde_json::to_string(&v).unwrap().contains("level_heads"));
}

#[test]
fn room_labels_room_tags_and_bent_tags_clear_each_other_in_one_pass() {
    use creabim_geom::declutter::{overlapping_labels, place_labels};
    let mm = 0.05;
    let look = TextLook::default();
    let mut items = vec![
        PlanItem::Label {
            id: ElementId::new(),
            at: DVec2::ZERO,
            text: "LIVING".into(),
        },
        creabim_geom::annotate::room_tag_item(
            ElementId::new(),
            "DINING\n2  25.6 m²".into(),
            DVec2::new(0.05, 0.0),
            look,
            mm,
        ),
        PlanItem::Tag {
            id: ElementId::new(),
            anchor: DVec2::new(0.0, -1.0),
            at: DVec2::new(0.1, 0.05),
            elbow: Some(DVec2::new(0.0, -0.5)),
            text: "D-101".into(),
            round: true,
            look,
        },
    ];
    assert!(!overlapping_labels(&items, mm).is_empty());
    place_labels(&mut items, mm);
    let clash = overlapping_labels(&items, mm);
    assert!(clash.is_empty(), "{clash:?}");
}

#[test]
fn the_starter_house_plans_with_level_marks_have_no_overlapping_labels() {
    let mut doc = Document::from_model(starter::starter_house());
    let tx = marks::set_rules(doc.model(), MarkRules::level_based());
    doc.commit(tx).unwrap();
    let m = doc.model();
    for e in m.iter() {
        let ElementKind::View(v) = &e.kind else {
            continue;
        };
        if !matches!(v.kind, ViewKind::Plan { .. }) {
            continue;
        }
        let mm = v.scale / 1000.0;
        let items = view_drawing(m, e.id).unwrap().items;
        // door and window marks stand where they were put
        let stays = |k: usize| {
            matches!(m.get(items[k].id()).map(|e| &e.kind),
                Some(ElementKind::Tag(t))
                    if creabim_geom::annotate::tag_normal(m, v, t).is_some())
        };
        let clash: Vec<_> = creabim_geom::declutter::overlapping_labels(&items, mm)
            .into_iter()
            .filter(|(a, b)| !stays(*a) && !stays(*b))
            .collect();
        assert!(clash.is_empty(), "{}: {clash:?}", e.name);
    }
}

#[test]
fn a_door_mark_is_a_split_circle_with_its_hand_over_its_number() {
    use creabim_geom::annotate::tag_geometry;
    let look = creabim_geom::annotation::TextLook::default();
    let g = tag_geometry(
        DVec2::new(5.0, 0.0),
        DVec2::ZERO,
        None,
        "L\nD01",
        true,
        &look,
        0.01,
    );
    assert_eq!(g.texts.len(), 2);
    assert!(
        g.texts[0].0.y > 0.0 && g.texts[1].0.y < 0.0,
        "hand above, number below"
    );
    assert!(g.divider.is_some());
    let one = tag_geometry(
        DVec2::new(5.0, 0.0),
        DVec2::ZERO,
        None,
        "W01",
        true,
        &look,
        0.01,
    );
    assert_eq!(one.texts.len(), 1);
    assert!(one.divider.is_none());
}

#[test]
fn door_and_window_marks_stand_square_to_their_wall() {
    let mut doc = Document::from_model(sample::sample_house());
    let plan = named(doc.model(), "Ground Floor plan");
    let ElementKind::View(v) = doc.model().get(plan).unwrap().kind.clone() else {
        unreachable!()
    };
    // a door's tag pushed askew comes back square, on a straight leader
    let (id, n) = doc
        .model()
        .iter()
        .find_map(|e| match &e.kind {
            ElementKind::Tag(t) if t.view == plan && is_door(&doc.model().get(t.host)?.kind) => {
                Some((
                    e.id,
                    creabim_geom::annotate::tag_normal(doc.model(), &v, t)?,
                ))
            }
            _ => None,
        })
        .unwrap();
    doc.edit_merged(id, "Move tag", |e| {
        if let ElementKind::Tag(t) = &mut e.kind {
            t.offset = n * 3.0 + n.perp() * 2.0;
        }
    })
    .unwrap();
    let d = view_drawing(doc.model(), plan).unwrap();
    let (anchor, at) = d
        .items
        .iter()
        .find_map(|i| match i {
            PlanItem::Tag {
                id: t, anchor, at, ..
            } if *t == id => Some((*anchor, *at)),
            _ => None,
        })
        .unwrap();
    assert!((at - anchor - n * 3.0).length() < 1e-9);
    // its clear passage is written along the leader: width and height
    let along: Vec<String> = d
        .items
        .iter()
        .filter_map(|i| match i {
            PlanItem::Annotation { id: t, item } if *t == id => Some(item.text.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(along.len(), 2, "{along:?}");
    // a window's mark has its outer sill's beside it, K over its number
    assert!(d
        .items
        .iter()
        .any(|i| matches!(i, PlanItem::Tag { text, round: true, .. } if text.starts_with("K\n"))));
}

#[test]
fn a_typed_dimension_moves_what_its_end_is_on() {
    use creabim_core::{Dimension, HostPoint};
    use creabim_geom::annotate::{attach, driven, resolve, stretches};
    let mut doc = Document::from_model(sample::sample_house());
    let plan = named(doc.model(), "Ground Floor plan");
    let ElementKind::View(v) = doc.model().get(plan).unwrap().kind.clone() else {
        unreachable!()
    };
    // from a free point to a window's jamb, along its wall
    let (win, at) = doc
        .model()
        .iter()
        .find_map(|e| match &e.kind {
            ElementKind::Opening(o) if o.kind == OpeningKind::Window => {
                let p = resolve(
                    doc.model(),
                    &v,
                    &HostPoint {
                        host: Some(e.id),
                        local: DVec2::ZERO,
                        from_end: false,
                        body: None,
                    },
                )?;
                Some((e.id, p))
            }
            _ => None,
        })
        .unwrap();
    let ElementKind::Opening(o) = doc.model().get(win).unwrap().kind.clone() else {
        unreachable!()
    };
    let ElementKind::Wall(w) = doc.model().get(o.host).unwrap().kind.clone() else {
        unreachable!()
    };
    let from = at - w.direction() * 1.0;
    let d = Dimension::linear(
        plan,
        HostPoint::free(from),
        attach(doc.model(), &v, Some(win), at),
        0.5,
    );
    assert_eq!(stretches(doc.model(), &v, &d).len(), 1);
    let tx = driven(doc.model(), &v, &d, 0, 1.25, &[]).unwrap();
    doc.commit(tx).unwrap();
    let ElementKind::Opening(now) = &doc.model().get(win).unwrap().kind else {
        unreachable!()
    };
    assert!((now.offset - o.offset - 0.25).abs() < 1e-9);
    // nothing to move between two free points
    let free = Dimension::linear(plan, HostPoint::free(from), HostPoint::free(at), 0.5);
    assert!(driven(doc.model(), &v, &free, 0, 2.0, &[]).is_err());
}

/// An EQ chain: free point, a window's jamb, free point; the window
/// slides to the middle and each stretch reads EQ.
#[test]
fn an_eq_chain_moves_the_point_between_to_the_middle() {
    use creabim_core::{Dimension, HostPoint};
    use creabim_geom::annotate::{attach, equalised, resolve, stretches};
    let mut doc = Document::from_model(sample::sample_house());
    let plan = named(doc.model(), "Ground Floor plan");
    let ElementKind::View(v) = doc.model().get(plan).unwrap().kind.clone() else {
        unreachable!()
    };
    let (win, at) = doc
        .model()
        .iter()
        .find_map(|e| match &e.kind {
            ElementKind::Opening(o) if o.kind == OpeningKind::Window => {
                let p = resolve(
                    doc.model(),
                    &v,
                    &HostPoint {
                        host: Some(e.id),
                        local: DVec2::ZERO,
                        from_end: false,
                        body: None,
                    },
                )?;
                Some((e.id, p))
            }
            _ => None,
        })
        .unwrap();
    let ElementKind::Opening(o) = doc.model().get(win).unwrap().kind.clone() else {
        unreachable!()
    };
    let ElementKind::Wall(w) = doc.model().get(o.host).unwrap().kind.clone() else {
        unreachable!()
    };
    let u = w.direction();
    let mut d = Dimension::linear(
        plan,
        HostPoint::free(at - u * 1.0),
        HostPoint::free(at + u * 0.5),
        0.5,
    );
    d.mids.push(attach(doc.model(), &v, Some(win), at));
    d.eq = true;
    assert_eq!(stretches(doc.model(), &v, &d).len(), 2);
    doc.commit(equalised(doc.model(), &v, &d).unwrap()).unwrap();
    let s = stretches(doc.model(), &v, &d);
    assert!((s[0].2.distance(s[0].3) - 0.75).abs() < 1e-6);
    assert!((s[1].2.distance(s[1].3) - 0.75).abs() < 1e-6);
    let items = creabim_geom::annotate::linear_items(doc.model(), &v, plan, &d);
    assert!(items
        .iter()
        .all(|i| matches!(i, PlanItem::Dimension { text, .. } if text == "EQ")));
}

#[test]
fn a_jogged_section_cuts_each_stretch_on_its_own_plane() {
    use creabim_core::{SectionJog, ViewKind};
    let mut doc = Document::from_model(sample::sample_house());
    let (sec, len) = doc
        .model()
        .iter()
        .find_map(|e| match &e.kind {
            ElementKind::View(v) => match v.kind {
                ViewKind::Section {
                    start,
                    end,
                    cut: true,
                    ..
                } => Some((e.id, start.distance(end))),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    let plan = named(doc.model(), "Ground Floor plan");
    let heavy = |doc: &Document| {
        view_drawing(doc.model(), plan)
            .unwrap()
            .items
            .iter()
            .filter(|i| matches!(i, PlanItem::Line { id, pen: creabim_geom::plan::Pen::Cut, .. } if *id == sec))
            .count()
    };
    let before = view_drawing(doc.model(), sec).unwrap().items;
    let ends = heavy(&doc);
    doc.edit_merged(sec, "Jog", |e| {
        if let ElementKind::View(v) = &mut e.kind {
            v.line.gap = true;
            v.line.jogs = vec![SectionJog {
                at: len / 2.0,
                offset: -3.0,
            }];
        }
    })
    .unwrap();
    // three heavy strokes more at the corner, and no thin line between
    assert_eq!(heavy(&doc), ends + 3);
    let after = view_drawing(doc.model(), sec).unwrap().items;
    assert_ne!(before, after, "the second stretch cuts elsewhere");
    let ElementKind::View(v) = &doc.model().get(sec).unwrap().kind else {
        unreachable!()
    };
    assert_eq!(v.line.stretches(len).len(), 2);
}

#[test]
fn a_dimension_to_a_walls_face_follows_the_wall() {
    use creabim_core::{Dimension, HostPoint, Wall};
    use creabim_geom::annotate::{rehosted, resolve};
    let mut doc = Document::from_model(sample::sample_house());
    let plan = named(doc.model(), "Ground Floor plan");
    let ElementKind::View(v) = doc.model().get(plan).unwrap().kind.clone() else {
        unreachable!()
    };
    let ViewKind::Plan { level, .. } = v.kind else {
        unreachable!()
    };
    // a wall of the plan's level, and a free point on its face
    let (id, w): (ElementId, Wall) = doc
        .model()
        .iter()
        .find_map(|e| match &e.kind {
            ElementKind::Wall(w) if w.level == level && !w.is_arc() => Some((e.id, w.clone())),
            _ => None,
        })
        .unwrap();
    let n = w.direction().perp();
    let face = w.start + w.direction() * (w.length() / 2.0) + n * (w.thickness / 2.0);
    let d = Dimension::linear(
        plan,
        HostPoint::free(face + n * 2.0),
        HostPoint::free(face),
        0.5,
    );
    let d = rehosted(doc.model(), &v, &d);
    assert_eq!(d.b.host, Some(id), "the point on the face is the wall's");
    assert_eq!(d.a.host, None, "the one in the open stays free");
    // the wall moves a metre: the point goes with it
    doc.edit_merged(id, "Move", |e| {
        if let ElementKind::Wall(w) = &mut e.kind {
            w.start -= n;
            w.end -= n;
        }
    })
    .unwrap();
    let q = resolve(doc.model(), &v, &d.b).unwrap();
    assert!((q - (face - n)).length() < 1e-9);
}

#[test]
fn reveal_insulation_is_billed_for_windows_and_framed_doors() {
    use creabim_core::joinery::{apply_door, reveal_strips, DoorFrame, DoorSpec};
    let mut doc = Document::from_model(creabim_core::starter::starter_house());
    let windows = reveal_strips(doc.model(), false);
    assert!(
        !windows.is_empty(),
        "the starter house's walls are insulated"
    );
    assert!(windows.iter().all(|s| s.area() > 0.0 && s.thickness > 0.0));
    // a door with a lining covers its reveal; in a frame the layers turn in
    let doors: Vec<Element> = doc
        .model()
        .iter()
        .filter(|e| is_door(&e.kind))
        .cloned()
        .collect();
    let mut tx = Transaction::new("Frames");
    for e in doors {
        let mut now = e.clone();
        if let ElementKind::Opening(o) = &mut now.kind {
            let spec = DoorSpec::fitting(DoorFrame::Steel, o.width, o.height);
            apply_door(o, spec);
        }
        tx = tx.replace(e, now);
    }
    doc.commit(tx).unwrap();
    let framed = reveal_strips(doc.model(), true);
    assert!(!framed.is_empty());
}

#[test]
fn a_level_inside_a_storey_shows_the_storeys_floor_and_what_stands_on_it() {
    let mut doc = Document::from_model(sample::sample_house());
    let ground = doc
        .model()
        .levels()
        .min_by(|a, b| a.1.elevation.total_cmp(&b.1.elevation))
        .map(|(e, l)| (e.id, l.elevation))
        .unwrap();
    let (tx, id, plan) = creabim_core::levels::add(doc.model(), "Cut", ground.1 + 1.5, 1.5, true);
    doc.commit(tx).unwrap();
    let plan = plan.unwrap();
    assert_eq!(
        creabim_geom::plan::storey_cut(doc.model(), id, 0.2),
        (ground.0, 1.7)
    );
    // the ground floor and its walls are in the new level's plan
    let items = view_drawing(doc.model(), plan).unwrap().items;
    let shows = |pick: &dyn Fn(&ElementKind) -> bool| {
        items
            .iter()
            .any(|i| doc.model().get(i.id()).is_some_and(|e| pick(&e.kind)))
    };
    assert!(shows(&|k| matches!(k, ElementKind::Wall(_))));
    assert!(shows(&|k| matches!(k, ElementKind::Slab(_))));
}

/// A pipe dimensioned from a wall: typing a new number moves the pipe
/// (with the pipes in line with it), never the wall, and a branch joined
/// to it stretches to stay joined.
#[test]
fn a_typed_dimension_to_a_pipe_moves_the_pipe() {
    use creabim_core::mep::Pipe;
    use creabim_core::{Dimension, Transaction};
    use creabim_geom::annotate::{attach, driven, stretches};
    let mut doc = Document::from_model(sample::sample_house());
    let plan = named(doc.model(), "Ground Floor plan");
    let ElementKind::View(v) = doc.model().get(plan).unwrap().kind.clone() else {
        unreachable!()
    };
    let ViewKind::Plan { level, .. } = v.kind else {
        unreachable!()
    };
    let (wall, w) = doc
        .model()
        .iter()
        .find_map(|e| match &e.kind {
            ElementKind::Wall(w) if w.level == level && w.length() > 3.0 => Some((e.id, w.clone())),
            _ => None,
        })
        .unwrap();
    let d = w.direction();
    let n = d.perp();
    let face = w.start + d * 1.0 + n * (w.thickness / 2.0);
    let pipe = |a: DVec2, b: DVec2| {
        Element::new(
            "Pipe",
            ElementKind::Pipe(Pipe {
                level,
                system: "SV".into(),
                start: a.extend(0.5),
                end: b.extend(0.5),
                pipe_type: "HERZ".into(),
                nominal_mm: 16.0,
                outer_mm: 16.0,
                inner_mm: 12.0,
                height_mm: None,
                insulation_mm: 0.0,
                insulation_set: false,
                mark: None,
            }),
        )
    };
    // two pipes in line along the wall, 0.3 m off it, and a branch
    // leaving square from where they meet
    let at = face + n * 0.3;
    let p1 = pipe(at, at + d * 1.0);
    let p2 = pipe(at + d * 1.0, at + d * 2.0);
    let br = pipe(at + d * 1.0, at + d * 1.0 + n * 1.0);
    let (i1, i2, ib) = (p1.id, p2.id, br.id);
    doc.commit(Transaction::new("pipes").insert(p1).insert(p2).insert(br))
        .unwrap();
    let dim = Dimension::linear(
        plan,
        attach(doc.model(), &v, Some(wall), face),
        attach(doc.model(), &v, Some(i1), at),
        0.5,
    );
    assert_eq!(stretches(doc.model(), &v, &dim).len(), 1);
    // a free point drawn on the pipe is bound to it
    let free = Dimension::linear(plan, dim.a, creabim_core::HostPoint::free(at), 0.5);
    let bound = creabim_geom::annotate::rehosted(doc.model(), &v, &free);
    assert_eq!(bound.b.host, Some(i1));
    let tx = driven(doc.model(), &v, &dim, 0, 0.5, &[]).unwrap();
    doc.commit(tx).unwrap();
    let ElementKind::Wall(after) = &doc.model().get(wall).unwrap().kind else {
        unreachable!()
    };
    assert!(after.start.distance(w.start) < 1e-9 && after.end.distance(w.end) < 1e-9);
    let ends = |id| match &doc.model().get(id).unwrap().kind {
        ElementKind::Pipe(p) => (p.start.truncate(), p.end.truncate()),
        _ => unreachable!(),
    };
    let to = at + n * 0.2;
    assert!(ends(i1).0.distance(to) < 1e-6);
    assert!(ends(i2).1.distance(to + d * 2.0) < 1e-6);
    // the branch keeps its far end and stays joined
    let (bs, be) = ends(ib);
    assert!(bs.distance(to + d * 1.0) < 1e-6, "{bs:?}");
    assert!(be.distance(at + d * 1.0 + n * 1.0) < 1e-6);
}

/// A dimension in a section from a level to a footing's top stays on the
/// footing when it is set deeper, and typing it sets the footing's depth.
#[test]
fn a_section_dimension_holds_and_drives_a_footing() {
    use creabim_core::footing::Footing;
    use creabim_core::{Dimension, Transaction};
    use creabim_geom::annotate::{attach, driven, resolve};
    let mut doc = Document::from_model(sample::sample_house());
    let level = doc.model().levels().next().map(|(e, _)| e.id).unwrap();
    let z0 = doc.model().level(level).unwrap().elevation;
    let mut f = Footing::strip(level, DVec2::new(0.0, 5.0), DVec2::new(4.0, 5.0));
    f.below = 0.5;
    let fe = Element::new("Footing", ElementKind::Footing(f));
    let fid = fe.id;
    let v = View::section(DVec2::new(2.0, 0.0), DVec2::new(2.0, 10.0), 20.0, true);
    let ve = Element::new("Section", ElementKind::View(v.clone()));
    let vid = ve.id;
    doc.commit(Transaction::new("f").insert(fe).insert(ve)).unwrap();
    // the footing's top, where the section crosses it, and the level above
    let top = DVec2::new(5.0, z0 - 0.5);
    let a = attach(doc.model(), &v, Some(level), DVec2::new(5.0, z0));
    let b = attach(doc.model(), &v, Some(fid), top);
    assert_eq!(b.host, Some(fid));
    assert!(resolve(doc.model(), &v, &b).unwrap().distance(top) < 1e-9);
    let dim = Dimension::linear(vid, a, b, 0.5);
    // typed: 800 from the level, so the footing goes 300 deeper
    let tx = driven(doc.model(), &v, &dim, 0, 0.8, &[]).unwrap();
    doc.commit(tx).unwrap();
    let ElementKind::Footing(after) = &doc.model().get(fid).unwrap().kind else {
        unreachable!()
    };
    assert!((after.below - 0.8).abs() < 1e-9, "{}", after.below);
    // and the dimension's end went down with it
    let q = resolve(doc.model(), &v, &dim.b).unwrap();
    assert!((q.y - (z0 - 0.8)).abs() < 1e-9, "{q:?}");
}
