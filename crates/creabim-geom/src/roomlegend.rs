//! The legend of rooms an architectural plan carries beside it, as the
//! office's ASR sheets do: each room of the storey with its number, its
//! use, its area and the finishes of its floor, walls and ceiling. What
//! a room does not say of its finishes is read from the build-ups round
//! it: the top layer of the floor under it, the layers of its walls
//! that face it, the ceiling over it.

use crate::schedule::{Column, Row, Schedule};
use creabim_core::assembly::{Assembly, LayerFunction};
use creabim_core::words::tr;
use creabim_core::{Element, ElementId, ElementKind, Model, Room, ScheduleCategory};
use glam::DVec2;

/// The finishes of a room, each as the legend writes it; empty where
/// neither the room nor the build-ups say.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Finishes {
    pub floor: String,
    pub walls: String,
    pub ceiling: String,
}

fn material_name(model: &Model, id: Option<ElementId>) -> Option<String> {
    let e = model.get(id?)?;
    matches!(e.kind, ElementKind::Material(_)).then(|| e.name.clone())
}

/// A layer of a build-up as a finish: its material, when it is a finish
/// or the build-up has one layer of it only at that face.
fn face(model: &Model, a: &Assembly, last: bool) -> Option<String> {
    let l = if last {
        a.layers.last()?
    } else {
        a.layers.first()?
    };
    if a.layers.len() > 1 && !matches!(l.function, LayerFunction::Finish | LayerFunction::Substrate)
    {
        return None;
    }
    material_name(model, l.material)
}

/// The slab of `level` under or over the point `p`.
fn slab_at(model: &Model, level: ElementId, p: DVec2) -> Option<&Element> {
    model.iter().find(|e| {
        matches!(&e.kind, ElementKind::Slab(s)
            if s.level == level && creabim_core::polygon::contains(&s.boundary, p))
    })
}

/// Each floor build-up under a room its own mark, P1, P2… (George
/// 2026-11-22: tiles in one room and laminate in the next are two
/// floors, marked apart): build-ups in the order of the rooms over them
/// (by storey and number), a mark typed on a build-up kept and skipped.
pub fn floor_marks(model: &Model) -> std::collections::HashMap<ElementId, String> {
    let mut rooms: Vec<(f64, &str, ElementId, DVec2)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Room(r) => Some((
                model.level(r.level).map_or(0.0, |l| l.elevation),
                r.number.as_str(),
                r.level,
                r.at,
            )),
            _ => None,
        })
        .collect();
    rooms.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(b.1)));
    let mut order: Vec<ElementId> = vec![];
    let mut taken: Vec<String> = vec![];
    for (_, _, level, at) in rooms {
        let Some(slab) = slab_at(model, level, at) else {
            continue;
        };
        if creabim_core::marks::manual(slab).is_some() {
            continue;
        }
        let Some((a, _)) = creabim_core::assembly_of(model, slab) else {
            continue;
        };
        match creabim_core::marks::manual(a) {
            Some(m) => taken.push(m.to_string()),
            None if !order.contains(&a.id) => order.push(a.id),
            None => {}
        }
    }
    let mut out = std::collections::HashMap::new();
    let mut n = 1;
    for id in order {
        let mut m = format!("P{n}");
        while taken.contains(&m) {
            n += 1;
            m = format!("P{n}");
        }
        n += 1;
        out.insert(id, m);
    }
    out
}

/// The finishes of room `e`: its own words first, else the build-ups'.
pub fn finishes(model: &Model, e: &Element, r: &Room) -> Finishes {
    let mut f = Finishes {
        floor: r.floor_finish.trim().to_string(),
        walls: r.wall_finish.trim().to_string(),
        ceiling: r.ceiling_finish.trim().to_string(),
    };
    let shape = creabim_core::room::shape(model, e.id);
    if f.floor.is_empty() {
        // floors list their layers from the top down
        if let Some(slab) = slab_at(model, r.level, r.at) {
            if let Some((a, asm)) = creabim_core::assembly_of(model, slab) {
                if let Some(top) = face(model, asm, false) {
                    // the finish and the floor's mark (P1), not the whole
                    // build-up's name
                    let mark = creabim_core::marks::manual(slab)
                        .or_else(|| creabim_core::marks::manual(a))
                        .map(str::to_string)
                        .or_else(|| creabim_core::marks::mark_of(model, slab.id))
                        .or_else(|| floor_marks(model).remove(&a.id));
                    f.floor = match mark {
                        Some(m) => format!("{top} {m}"),
                        None => top,
                    };
                }
            }
        }
    }
    if f.walls.is_empty() {
        if let Some(s) = &shape {
            let mut names: Vec<String> = vec![];
            let n = s.outline.len();
            for (i, by) in s.bounded_by.iter().enumerate() {
                let Some(wall) = by.and_then(|id| model.get(id)) else {
                    continue;
                };
                let ElementKind::Wall(w) = &wall.kind else {
                    continue;
                };
                let Some((_, asm)) = creabim_core::assembly_of(model, wall) else {
                    continue;
                };
                // the outline runs counter-clockwise, so the room is to
                // the left of its edge; a wall's exterior is to its
                // right, and its layers run from the exterior in
                let edge = s.outline[(i + 1) % n] - s.outline[i];
                let along = edge.dot(w.end - w.start) > 0.0;
                let inside = along != w.flip;
                if let Some(name) = face(model, asm, inside) {
                    if !names.contains(&name) {
                        names.push(name);
                    }
                }
            }
            f.walls = names.join(", ");
        }
    }
    // tiling goes with the walls' finish, with its height and area
    if let (Some(t), Some(s)) = (&r.tiling, &shape) {
        let name = if t.name.trim().is_empty() {
            tr("Tiling")
        } else {
            t.name.trim().to_string()
        };
        let area = creabim_core::room::tiling_area(model, r, s);
        let tiles = format!(
            "{name} {} {:.0} mm ({})",
            tr("up to"),
            t.height * 1000.0,
            model.units().area(area)
        );
        f.walls = if f.walls.is_empty() {
            tiles
        } else {
            format!("{}; {tiles}", f.walls)
        };
    }
    if f.ceiling.is_empty() {
        let top = creabim_core::room::height(model, r) + 0.5;
        if let Some((id, _)) = creabim_core::room::ceiling_over(model, r.level, r.at, top) {
            if let Some(c) = model.get(id) {
                f.ceiling = creabim_core::material_of(model, c)
                    .map(|(m, _)| m.name.clone())
                    .or_else(|| {
                        c.of_type
                            .as_ref()
                            .and_then(|t| model.get(t.id))
                            .map(|t| t.name.clone())
                    })
                    .unwrap_or_else(|| c.name.clone());
            }
        } else {
            // the underside of the floor of the storey above
            let levels = creabim_core::levels::sorted(model);
            let above = levels
                .iter()
                .position(|l| l.0 == r.level)
                .and_then(|i| levels.get(i + 1));
            if let Some(slab) = above.and_then(|l| slab_at(model, l.0, r.at)) {
                if let Some((_, asm)) = creabim_core::assembly_of(model, slab) {
                    f.ceiling = face(model, asm, true).unwrap_or_default();
                }
            }
        }
    }
    f
}

/// The undersides of the roofs, as triangles: what limits the clear
/// height of the rooms under them.
pub struct Undersides(pub(crate) Vec<[glam::DVec3; 3]>);

/// The clear height under which a room's floor is not counted, m (STN
/// 73 4301 7.2.4, 10.1).
pub const LEAST_COUNTED_HEIGHT: f64 = 1.3;

impl Undersides {
    pub fn of(model: &Model) -> Undersides {
        let roofs: Vec<ElementId> = model
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Roof(_)))
            .map(|e| e.id)
            .collect();
        let mut out = vec![];
        if roofs.is_empty() {
            return Undersides(out);
        }
        for it in crate::scene::build_scene(model) {
            if !roofs.contains(&it.id) || it.ghost {
                continue;
            }
            let m = &it.mesh;
            for t in m.indices.chunks_exact(3) {
                if m.normals[t[0] as usize][2] > -0.05 {
                    continue;
                }
                let p = |i: u32| {
                    let q = m.positions[i as usize];
                    glam::DVec3::new(q[0] as f64, q[1] as f64, q[2] as f64)
                };
                out.push([p(t[0]), p(t[1]), p(t[2])]);
            }
        }
        Undersides(out)
    }
}

/// A room's floor area as STN 73 4301 counts it (10.1): its net area
/// less where the roof over it leaves less than 1.3 m of clear height.
pub fn counted_area(model: &Model, under: &Undersides, r: &Room) -> f64 {
    let Some(s) = creabim_core::room::shape_of(model, r) else {
        return 0.0;
    };
    // without plaster (George)
    let s = creabim_core::room::bare_shape(model, &s);
    let net = creabim_core::room::net_area(model, r, &s);
    if under.0.is_empty() {
        return net;
    }
    let floor = model.level(r.level).map_or(0.0, |l| l.elevation);
    let (mut lo, mut hi) = (s.outline[0], s.outline[0]);
    for p in &s.outline {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let cell = 0.05;
    let (nx, ny) = (
        ((hi.x - lo.x) / cell).ceil().max(1.0) as usize,
        ((hi.y - lo.y) / cell).ceil().max(1.0) as usize,
    );
    // the lowest underside over each cell, above the floor
    let mut low = vec![f64::INFINITY; nx * ny];
    for [a, b, c] in &under.0 {
        let d = (b - a).truncate().perp_dot((c - a).truncate());
        if d.abs() < 1e-12 || a.z.max(b.z).max(c.z) <= floor {
            continue;
        }
        let tlo = a.truncate().min(b.truncate()).min(c.truncate());
        let thi = a.truncate().max(b.truncate()).max(c.truncate());
        if thi.x < lo.x || thi.y < lo.y || tlo.x > hi.x || tlo.y > hi.y {
            continue;
        }
        let i0 = ((tlo.x - lo.x) / cell).floor().max(0.0) as usize;
        let j0 = ((tlo.y - lo.y) / cell).floor().max(0.0) as usize;
        let i1 = (((thi.x - lo.x) / cell).ceil().max(0.0) as usize).min(nx);
        let j1 = (((thi.y - lo.y) / cell).ceil().max(0.0) as usize).min(ny);
        for j in j0..j1 {
            for i in i0..i1 {
                let q = lo + DVec2::new(i as f64 + 0.5, j as f64 + 0.5) * cell;
                let l2 = (b - a).truncate().perp_dot(q - a.truncate()) / d;
                let l1 = (q - a.truncate()).perp_dot((c - a).truncate()) / d;
                let l0 = 1.0 - l1 - l2;
                if l0 < -1e-9 || l1 < -1e-9 || l2 < -1e-9 {
                    continue;
                }
                let z = a.z * l0 + b.z * l1 + c.z * l2;
                if z > floor {
                    let k = j * nx + i;
                    low[k] = low[k].min(z);
                }
            }
        }
    }
    let mut cut = 0.0;
    for j in 0..ny {
        for i in 0..nx {
            let z = low[j * nx + i];
            if z.is_finite() && z - floor < LEAST_COUNTED_HEIGHT {
                let q = lo + DVec2::new(i as f64 + 0.5, j as f64 + 0.5) * cell;
                if creabim_core::polygon::contains(&s.outline, q) {
                    cut += cell * cell;
                }
            }
        }
    }
    (net - cut).max(0.0)
}

/// The legend as a table: the rooms of `level` (of every storey with
/// none), by their numbers, with the area of them all under it.
pub fn schedule(model: &Model, level: Option<ElementId>) -> Schedule {
    let col = |title: &'static str, width: f64, numeric: bool| Column {
        title,
        field: None,
        width,
        numeric,
        align: None,
    };
    let columns = vec![
        col("Mark", 14.0, false),
        col("Room use", 34.0, false),
        col("Area", 18.0, true),
        col("Floor", 40.0, false),
        col("Walls - finish", 50.0, false),
        col("Ceiling - finish", 40.0, false),
    ];
    // the finishes sit in the middle of their cells, as the office's
    // legends set them; the sheet may set each column otherwise
    let mut columns = columns;
    for c in columns.iter_mut().skip(3) {
        c.align = Some(creabim_core::ColumnAlign::Center);
    }
    let units = model.units();
    let dash = |s: String| if s.is_empty() { "-".to_string() } else { s };
    let under = Undersides::of(model);
    let mut rows: Vec<(f64, String, Row)> = vec![];
    let mut total = 0.0;
    let mut living = 0.0;
    for e in model.iter() {
        let ElementKind::Room(r) = &e.kind else {
            continue;
        };
        if level.is_some_and(|l| l != r.level) {
            continue;
        }
        // without what is under 1.3 m of clear height in an attic
        let area = counted_area(model, &under, r);
        // the floor (usable) area leaves balconies, loggias and
        // terraces out (STN 73 4301 2.14)
        if !creabim_core::room::is_open_air(&e.name, r) {
            total += area;
        }
        if creabim_core::room::counts_as_living(&e.name, r, area) {
            living += area;
        }
        let f = finishes(model, e, r);
        rows.push((
            model.level(r.level).map_or(0.0, |l| l.elevation),
            r.number.clone(),
            Row {
                id: e.id,
                group: false,
                cells: vec![
                    r.number.clone(),
                    e.name.clone(),
                    units.area(area),
                    dash(f.floor),
                    dash(f.walls),
                    dash(f.ceiling),
                ],
            },
        ));
    }
    rows.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    let rows: Vec<Row> = rows.into_iter().map(|r| r.2).collect();
    let listed = rows.len();
    Schedule {
        category: ScheduleCategory::RoomLegend,
        columns,
        rows,
        totals: vec![
            tr("Usable area"),
            String::new(),
            units.area(total),
            String::new(),
            String::new(),
            String::new(),
        ],
        listed,
        // the living area over the usable area; a view may leave it out
        above_totals: vec![vec![
            tr("Living area"),
            String::new(),
            units.area(living),
            String::new(),
            String::new(),
            String::new(),
        ]],
    }
}

/// A legend of rooms for a trade's drawings: the room's name and its
/// area only (George 2026-11-23), without the mark, the floor build-up
/// and the finishes of walls and ceiling; the totals keep their titles.
pub fn without_finishes(s: &mut Schedule) {
    const USE: usize = 1;
    const AREA: usize = 2;
    fn pick<T>(v: &mut Vec<T>) {
        if v.len() > AREA {
            let area = v.swap_remove(AREA);
            let name = v.swap_remove(USE);
            *v = vec![name, area];
        }
    }
    pick(&mut s.columns);
    for r in &mut s.rows {
        pick(&mut r.cells);
    }
    // "Usable area" and "Living area" stand where the names do
    let title_and_area = |v: &mut Vec<String>| {
        if v.len() > AREA {
            *v = vec![std::mem::take(&mut v[0]), std::mem::take(&mut v[AREA])];
        }
    };
    title_and_area(&mut s.totals);
    for t in &mut s.above_totals {
        title_and_area(t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::{Document, Transaction};

    /// A room that says nothing of its floor reads it from the build-up
    /// of the floor under it; its own words win.
    #[test]
    fn a_room_reads_its_finishes_from_the_build_ups_unless_it_says() {
        let mut doc = Document::from_model(creabim_core::starter::starter_house());
        let m = doc.model();
        let all = schedule(m, None);
        assert!(!all.rows.is_empty());
        let one = m
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Room(_)))
            .unwrap()
            .clone();
        let ElementKind::Room(r) = &one.kind else {
            unreachable!()
        };
        // one storey's rooms only
        let level = schedule(m, Some(r.level));
        assert!(level.rows.len() <= all.rows.len() && !level.rows.is_empty());
        // its own word wins; without it the floor's top layer speaks
        let mut said = one.clone();
        let mut bare = one.clone();
        if let ElementKind::Room(r) = &mut said.kind {
            r.floor_finish = "Oak boards".into();
        }
        if let ElementKind::Room(r) = &mut bare.kind {
            r.floor_finish.clear();
        }
        doc.commit(Transaction::new("t").replace(one.clone(), said.clone()))
            .unwrap();
        let ElementKind::Room(r) = &said.kind else {
            unreachable!()
        };
        assert_eq!(finishes(doc.model(), &said, r).floor, "Oak boards");
        doc.commit(Transaction::new("t").replace(said, bare.clone()))
            .unwrap();
        let ElementKind::Room(r) = &bare.kind else {
            unreachable!()
        };
        let f = finishes(doc.model(), &bare, r);
        let slab = slab_at(doc.model(), r.level, r.at);
        let layered = slab.is_some_and(|s| creabim_core::assembly_of(doc.model(), s).is_some());
        assert!(layered || f.floor.is_empty(), "{f:?}");
    }

    /// Floors of two build-ups are marked apart, P1 and P2; rooms on
    /// the same build-up share its mark.
    #[test]
    fn floors_of_other_build_ups_get_other_marks() {
        let m = creabim_core::starter::starter_house();
        let marks = floor_marks(&m);
        let mut seen: Vec<(ElementId, String)> = vec![];
        for e in m.iter() {
            let ElementKind::Room(r) = &e.kind else {
                continue;
            };
            let Some(slab) = slab_at(&m, r.level, r.at) else {
                continue;
            };
            let Some((a, _)) = creabim_core::assembly_of(&m, slab) else {
                continue;
            };
            if let Some(mark) = marks.get(&a.id) {
                seen.push((a.id, mark.clone()));
            }
        }
        for (a, ma) in &seen {
            for (b, mb) in &seen {
                assert_eq!(a == b, ma == mb, "{seen:?}");
            }
        }
        assert!(marks.values().all(|v| v.starts_with('P')));
    }

    /// The living area sums the living rooms over 8 m² and kitchens
    /// over 12 m², each room's own word winning; the view may drop it.
    #[test]
    fn the_living_area_sits_over_the_usable_area() {
        let mut m = creabim_core::starter::starter_house();
        let s = schedule(&m, None);
        assert_eq!(s.above_totals.len(), 1);
        assert_eq!(s.above_totals[0][0], "Living area");
        let area = |m: &Model, e: &Element| {
            creabim_core::room::shape(m, e.id)
                .map_or(0.0, |sh| creabim_core::room::bare_area(m, room(e), &sh))
        };
        fn room(e: &Element) -> &Room {
            match &e.kind {
                ElementKind::Room(r) => r,
                _ => unreachable!(),
            }
        }
        let rooms: Vec<Element> = m
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Room(_)))
            .cloned()
            .collect();
        let sum: f64 = rooms
            .iter()
            .filter(|e| creabim_core::room::counts_as_living(&e.name, room(e), area(&m, e)))
            .map(|e| area(&m, e))
            .sum();
        assert!(sum > 0.0);
        assert_eq!(s.above_totals[0][2], m.units().area(sum));
        // a kitchen on its own is príslušenstvo (STN 73 4301/Z1 8.4.6),
        // however big; a living room with a kitchen counts; a WC does not
        let kitchen = rooms.iter().find(|e| e.name == "Kitchen").unwrap();
        assert!(area(&m, kitchen) > 12.0);
        assert!(!creabim_core::room::counts_as_living(
            "Kitchen",
            room(kitchen),
            area(&m, kitchen)
        ));
        let mut both = room(kitchen).clone();
        both.occupancy.clear();
        assert!(creabim_core::room::counts_as_living(
            "Obývacia izba s kuchyňou",
            &both,
            30.0
        ));
        let wc = rooms.iter().find(|e| e.name == "WC").unwrap();
        assert!(!creabim_core::room::counts_as_living("WC", room(wc), 20.0));
        // the designer's word wins
        let mut wc2 = wc.clone();
        if let ElementKind::Room(r) = &mut wc2.kind {
            r.living = Some(true);
        }
        assert!(creabim_core::room::counts_as_living("WC", room(&wc2), 3.0));
        // the usable area leaves the loggia, terrace and balcony out
        let usable: f64 = rooms
            .iter()
            .filter(|e| !creabim_core::room::is_open_air(&e.name, room(e)))
            .map(|e| area(&m, e))
            .sum();
        let all: f64 = rooms.iter().map(|e| area(&m, e)).sum();
        assert!(usable < all - 10.0);
        assert_eq!(s.totals[2], m.units().area(usable));
        // a view without it
        let mut v = creabim_core::View::schedule(ScheduleCategory::RoomLegend);
        v.no_living_area = true;
        let e = Element::new("L", ElementKind::View(v));
        let id = e.id;
        creabim_core::Transaction::new("t")
            .insert(e)
            .apply(&mut m)
            .unwrap();
        let t = crate::schedule::view_table(&m, id).unwrap();
        assert!(t.above_totals.is_empty());
    }

    /// Under a sloping roof the floor where the clear height is under
    /// 1.3 m is not counted (STN 73 4301 7.2.4, 10.1).
    #[test]
    fn an_attic_counts_only_what_is_over_1_3_m_high() {
        use glam::DVec3;
        let m = creabim_core::starter::starter_house();
        let (e, r) = m
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::Room(r) if e.name == "Study" => Some((e, r)),
                _ => None,
            })
            .unwrap();
        let s = creabim_core::room::shape_of(&m, r).unwrap();
        let s = creabim_core::room::bare_shape(&m, &s);
        let net = creabim_core::room::net_area(&m, r, &s);
        let floor = m.level(r.level).unwrap().elevation;
        let (mut lo, mut hi) = (s.outline[0], s.outline[0]);
        for p in &s.outline {
            lo = lo.min(*p);
            hi = hi.max(*p);
        }
        // a slope from 0.5 m over the floor at one side to 3.0 m at the other
        let z = |x: f64| floor + 0.5 + 2.5 * (x - lo.x) / (hi.x - lo.x);
        let (a, b, c, d) = (
            DVec3::new(lo.x, lo.y, z(lo.x)),
            DVec3::new(hi.x, lo.y, z(hi.x)),
            DVec3::new(hi.x, hi.y, z(hi.x)),
            DVec3::new(lo.x, hi.y, z(lo.x)),
        );
        let under = Undersides(vec![[a, b, c], [a, c, d]]);
        let counted = counted_area(&m, &under, r);
        let rect = (hi.x - lo.x) * (hi.y - lo.y);
        assert!(counted < net - 1.0, "{counted} {net}");
        if (rect - net).abs() < 0.05 {
            // a rectangle: (1.3 - 0.5) / 2.5 of it is too low
            assert!(
                (counted - net * (1.0 - 0.8 / 2.5)).abs() < 0.3,
                "{counted} {net}"
            );
        }
        // no roof over it: all of it
        assert!((counted_area(&m, &Undersides(vec![]), r) - net).abs() < 1e-9);
        let _ = e;
    }

    /// Tiling runs along the room's walls less its doors, and the legend
    /// writes it with the walls: its height and its area.
    #[test]
    fn tiling_leaves_the_doors_out_and_goes_to_the_legend() {
        let m = creabim_core::starter::starter_house();
        // a room with a door in one of its walls
        let (e, shape) = m
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Room(_)))
            .filter_map(|e| Some((e, creabim_core::room::shape(&m, e.id)?)))
            .find(|(_, s)| {
                let runs: f64 = creabim_core::room::tiling_runs(&m, s, &[])
                    .iter()
                    .map(|(a, b)| a.distance(*b))
                    .sum();
                let walls = s.bounded_by.iter().all(|b| {
                    b.and_then(|id| m.get(id))
                        .is_some_and(|w| matches!(w.kind, ElementKind::Wall(_)))
                });
                walls && runs < s.perimeter() - 0.5
            })
            .expect("a room with a door");
        let mut tiled = e.clone();
        let ElementKind::Room(r) = &mut tiled.kind else {
            unreachable!()
        };
        r.tiling = Some(creabim_core::room::Tiling {
            height: 2.0,
            name: "Ceramic tiles".into(),
            skip: vec![],
        });
        let runs: f64 = creabim_core::room::tiling_runs(&m, &shape, &[])
            .iter()
            .map(|(a, b)| a.distance(*b))
            .sum();
        let r = r.clone();
        // the runs by the height, less the windows below it
        let area = creabim_core::room::tiling_area(&m, &r, &shape);
        assert!(area > 0.0 && area <= runs * 2.0 + 1e-9);
        // a wall left out takes its run with it
        let wall = creabim_core::room::bounding_walls(&m, &shape)[0];
        let less: f64 = creabim_core::room::tiling_runs(&m, &shape, &[wall])
            .iter()
            .map(|(a, b)| a.distance(*b))
            .sum();
        assert!(less < runs - 0.1);
        let f = finishes(&m, &tiled, &r);
        assert!(f.walls.contains("Ceramic tiles up to 2000 mm ("), "{f:?}");
    }
}

#[cfg(test)]
mod widths {
    use creabim_core::{Element, ElementKind, ScheduleCategory, Transaction, View};

    /// A column dragged wider on the sheet keeps its width by its
    /// heading; the table draws wider by as much and the columns after it
    /// move along.
    #[test]
    fn a_column_widened_on_the_sheet_widens_the_legend() {
        let mut m = creabim_core::starter::starter_house();
        let v = Element::new(
            "Legend",
            ElementKind::View(View::schedule(ScheduleCategory::RoomLegend)),
        );
        let id = v.id;
        Transaction::new("t").insert(v).apply(&mut m).unwrap();
        let before = crate::schedule::column_edges(&m, id).unwrap();
        let width = |m: &creabim_core::Model| {
            let (lo, hi) = crate::section::view_drawing(m, id)
                .unwrap()
                .bounds()
                .unwrap();
            hi.x - lo.x
        };
        let w0 = width(&m);
        let name = before[1].0.clone();
        let mut e = m.get(id).unwrap().clone();
        if let ElementKind::View(x) = &mut e.kind {
            x.column_widths.insert(name.clone(), before[1].2 + 20.0);
        }
        Transaction::new("t")
            .replace(m.get(id).unwrap().clone(), e)
            .apply(&mut m)
            .unwrap();
        let after = crate::schedule::column_edges(&m, id).unwrap();
        assert_eq!(after[0], before[0]);
        assert!((after[1].2 - before[1].2 - 20.0).abs() < 1e-9);
        assert!((after.last().unwrap().1 - before.last().unwrap().1 - 20.0).abs() < 1e-9);
        assert!((width(&m) - w0 - 20.0).abs() < 0.5, "{} {}", width(&m), w0);
    }
}

#[cfg(test)]
mod legend_widths {
    use creabim_core::{Element, ElementKind, ScheduleCategory, Transaction, View};

    /// The legends of services and materials take the width of their
    /// words' column from the sheet: the words wrap to it.
    #[test]
    fn a_legends_words_column_is_set_on_the_sheet() {
        for category in [
            ScheduleCategory::MaterialLegend,
            ScheduleCategory::ServicesLegend,
        ] {
            let mut m = creabim_core::sample::sample_house();
            let v = Element::new("Legend", ElementKind::View(View::schedule(category)));
            let id = v.id;
            Transaction::new("t").insert(v).apply(&mut m).unwrap();
            let edges = crate::schedule::column_edges(&m, id).unwrap();
            assert_eq!(edges.len(), 2);
            let mut e = m.get(id).unwrap().clone();
            if let ElementKind::View(x) = &mut e.kind {
                x.column_widths.insert("Symbol".into(), edges[0].2 + 10.0);
                x.column_widths.insert("Description".into(), 40.0);
            }
            Transaction::new("t")
                .replace(m.get(id).unwrap().clone(), e)
                .apply(&mut m)
                .unwrap();
            let after = crate::schedule::column_edges(&m, id).unwrap();
            assert!((after[0].1 - edges[0].1 - 10.0).abs() < 1e-9);
            assert!((after[1].2 - 40.0).abs() < 1e-9);
            // the words start at the new edge
            let d = crate::section::view_drawing(&m, id).unwrap();
            let starts: Vec<f64> = d
                .items
                .iter()
                .filter_map(|i| match i {
                    crate::plan::PlanItem::Text { at, .. } if at.x > 1.0 => Some(at.x),
                    _ => None,
                })
                .collect();
            assert!(
                starts
                    .iter()
                    .all(|x| (x - after[0].1).abs() < 1e-9 || *x > after[0].1),
                "{category:?} {starts:?}"
            );
        }
    }
}

#[cfg(test)]
mod totals_row {
    /// The usable area's label runs on to its value: no column line
    /// crosses it in the totals row.
    #[test]
    fn the_usable_area_label_is_not_crossed_by_a_column_line() {
        let m = creabim_core::starter::starter_house();
        let s = super::schedule(&m, None);
        let d = crate::schedule::schedule_drawing(creabim_core::ElementId::NIL, "x", &s);
        let x1 = s.columns[0].width;
        // the line between the first two columns stops above the totals
        let low = d
            .items
            .iter()
            .filter_map(|i| match i {
                crate::plan::PlanItem::Line { a, b, .. }
                    if (a.x - x1).abs() < 1e-9 && (b.x - x1).abs() < 1e-9 =>
                {
                    Some(a.y.min(b.y))
                }
                _ => None,
            })
            .fold(f64::INFINITY, f64::min);
        let bottom = d
            .items
            .iter()
            .filter_map(|i| match i {
                crate::plan::PlanItem::Line { a, b, .. } => Some(a.y.min(b.y)),
                _ => None,
            })
            .fold(f64::INFINITY, f64::min);
        assert!(low > bottom + 1.0, "{low} {bottom}");
    }
}
