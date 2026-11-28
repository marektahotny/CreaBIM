//! The way the loops' pipes go from the manifold to their room (George
//! 2026-11-27, steps 3 to 6 of the underfloor heating): from room to
//! room through the doorways, the shortest way, into the room by its
//! connection doorway (the room's own pick, else the last one of that
//! way). The rooms passed through are the transit rooms (their floor
//! carries the pipes, drawn as the area of transit pipes); every doorway
//! crossed is a joint the pipes cross in sleeves.

use creabim_core::{polygon, ElementId, ElementKind, Model};
use glam::DVec2;

/// A doorway between two rooms of a storey (or a room and none).
#[derive(Clone, Debug, PartialEq)]
pub struct Door {
    pub id: ElementId,
    /// Its middle in the wall, and a step either side into the rooms.
    pub at: DVec2,
    pub sides: [DVec2; 2],
    pub rooms: [Option<ElementId>; 2],
    pub width: f64,
}

/// The way from the manifold to a room.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Route {
    /// From the manifold to the room's connection doorway (inside it).
    pub points: Vec<DVec2>,
    /// The doorways crossed, the last the room's own.
    pub doors: Vec<ElementId>,
    /// The rooms passed through, the manifold's first.
    pub transit: Vec<ElementId>,
    /// Its length, m.
    pub length: f64,
}

/// The doorways of a storey with the rooms either side (`rooms` the
/// storey's rooms and their outlines).
pub fn doors(model: &Model, level: ElementId, rooms: &[(ElementId, Vec<DVec2>)]) -> Vec<Door> {
    let room_at = |p: DVec2| {
        rooms
            .iter()
            .find(|r| r.1.len() > 2 && polygon::contains(&r.1, p))
            .map(|r| r.0)
    };
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Opening(o) = &e.kind else {
            continue;
        };
        if o.kind != creabim_core::OpeningKind::Door {
            continue;
        }
        let Some(ElementKind::Wall(w)) = model.get(o.host).map(|h| &h.kind) else {
            continue;
        };
        if w.level != level {
            continue;
        }
        let along = (w.end - w.start).normalize_or_zero();
        let at = w.start + along * (o.offset + o.width / 2.0);
        let n = DVec2::new(-along.y, along.x) * (w.thickness / 2.0 + 0.2);
        let sides = [at + n, at - n];
        out.push(Door {
            id: e.id,
            at,
            sides,
            rooms: [room_at(sides[0]), room_at(sides[1])],
            width: o.width,
        });
    }
    out
}

/// The shortest way from `from` (in room `start`) to room `to`, through
/// the doorways; into `to` by `by` when it is one of its doorways.
pub fn route(
    doors: &[Door],
    start: ElementId,
    from: DVec2,
    to: ElementId,
    by: Option<ElementId>,
) -> Option<Route> {
    if start == to {
        return Some(Route {
            points: vec![from],
            ..Route::default()
        });
    }
    // Dijkstra over the doorways: a state is a doorway crossed into a
    // room, its cost the way walked
    #[derive(Clone, Copy)]
    struct Node {
        cost: f64,
        door: usize,
        side: usize,
        prev: Option<usize>,
    }
    let mut nodes: Vec<Node> = vec![];
    let mut done: Vec<(usize, usize)> = vec![];
    // the doorways of the start room
    for (i, d) in doors.iter().enumerate() {
        for k in 0..2 {
            if d.rooms[k] == Some(start) {
                nodes.push(Node {
                    cost: from.distance(d.sides[k]) + d.sides[k].distance(d.sides[1 - k]),
                    door: i,
                    side: 1 - k,
                    prev: None,
                });
            }
        }
    }
    let mut best: Option<usize> = None;
    // the cheapest not yet done, each in turn
    while let Some(n) = (0..nodes.len())
        .filter(|&i| !done.contains(&(nodes[i].door, nodes[i].side)))
        .min_by(|&a, &b| nodes[a].cost.total_cmp(&nodes[b].cost))
    {
        let cur = nodes[n];
        done.push((cur.door, cur.side));
        let here = doors[cur.door].rooms[cur.side];
        let at = doors[cur.door].sides[cur.side];
        if here == Some(to) && by.is_none_or(|b| doors[cur.door].id == b) {
            best = Some(n);
            break;
        }
        let Some(here) = here else { continue };
        for (i, d) in doors.iter().enumerate() {
            if i == cur.door {
                continue;
            }
            for k in 0..2 {
                if d.rooms[k] == Some(here) && !done.contains(&(i, 1 - k)) {
                    nodes.push(Node {
                        cost: cur.cost
                            + at.distance(d.sides[k])
                            + d.sides[k].distance(d.sides[1 - k]),
                        door: i,
                        side: 1 - k,
                        prev: Some(n),
                    });
                }
            }
        }
    }
    let mut n = best?;
    let mut chain = vec![];
    loop {
        chain.push(nodes[n]);
        match nodes[n].prev {
            Some(p) => n = p,
            None => break,
        }
    }
    chain.reverse();
    let mut points = vec![from];
    let mut out = Route::default();
    out.transit.push(start);
    for c in &chain {
        let d = &doors[c.door];
        points.push(d.sides[1 - c.side]);
        points.push(d.sides[c.side]);
        out.doors.push(d.id);
        if let Some(r) = d.rooms[c.side].filter(|r| *r != to) {
            out.transit.push(r);
        }
    }
    out.length = points.windows(2).map(|w| w[0].distance(w[1])).sum();
    out.points = points;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_way_goes_through_the_hall() {
        // A ─ H ─ B: A and B open only to the hall H
        let (a, h, b) = (ElementId::new(), ElementId::new(), ElementId::new());
        let (d10, d11) = (ElementId::new(), ElementId::new());
        let door = |n, x: f64, ra, rb| Door {
            id: n,
            at: DVec2::new(x, 0.0),
            sides: [DVec2::new(x - 0.2, 0.0), DVec2::new(x + 0.2, 0.0)],
            rooms: [Some(ra), Some(rb)],
            width: 0.8,
        };
        let doors = vec![door(d10, 0.0, a, h), door(d11, 4.0, h, b)];
        let r = route(&doors, a, DVec2::new(-2.0, 0.0), b, None).unwrap();
        assert_eq!(r.doors, vec![d10, d11]);
        assert_eq!(r.transit, vec![a, h]);
        assert!((r.length - 6.2).abs() < 1e-9, "{}", r.length);
        // the room itself: no way at all
        assert_eq!(route(&doors, a, DVec2::ZERO, a, None).unwrap().length, 0.0);
    }
}
