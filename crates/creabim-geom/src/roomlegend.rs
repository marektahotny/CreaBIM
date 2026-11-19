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
                        .or_else(|| creabim_core::marks::mark_of(model, slab.id));
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

/// The legend as a table: the rooms of `level` (of every storey with
/// none), by their numbers, with the area of them all under it.
pub fn schedule(model: &Model, level: Option<ElementId>) -> Schedule {
    let col = |title: &'static str, width: f64, numeric: bool| Column {
        title,
        field: None,
        width,
        numeric,
    };
    let columns = vec![
        col("Mark", 14.0, false),
        col("Room use", 34.0, false),
        col("Area", 18.0, true),
        col("Floor", 40.0, false),
        col("Walls - finish", 50.0, false),
        col("Ceiling - finish", 40.0, false),
    ];
    let units = model.units();
    let dash = |s: String| if s.is_empty() { "-".to_string() } else { s };
    let mut rows: Vec<(f64, String, Row)> = vec![];
    let mut total = 0.0;
    for e in model.iter() {
        let ElementKind::Room(r) = &e.kind else {
            continue;
        };
        if level.is_some_and(|l| l != r.level) {
            continue;
        }
        let area = creabim_core::room::shape(model, e.id)
            .map_or(0.0, |s| creabim_core::room::net_area(model, r, &s));
        total += area;
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
