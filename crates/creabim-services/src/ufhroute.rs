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
    /// Where it crosses each doorway's joint, a step either side; none for
    /// a way through the doorways' own points.
    pub sides: Vec<[DVec2; 2]>,
    /// Where it leaves the main route, m along it, and to which side (1
    /// left, −1 right, 0 on at its end): the pipes side by side along the
    /// route are ordered by it, so that each pair peels off in turn
    /// without crossing the others (George 2026-11-28).
    pub peel: (f64, f64),
    /// The point it leaves the main route at.
    pub peel_at: DVec2,
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
        out.sides.push([d.sides[1 - c.side], d.sides[c.side]]);
        if let Some(r) = d.rooms[c.side].filter(|r| *r != to) {
            out.transit.push(r);
        }
    }
    out.length = points.windows(2).map(|w| w[0].distance(w[1])).sum();
    out.points = points;
    Some(out)
}

/// The nearest point to `p` on the polyline `line`, and how far along
/// it that is, m.
fn on_line(line: &[DVec2], p: DVec2) -> (DVec2, f64, usize) {
    let mut best = (line[0], 0.0, 0usize, p.distance(line[0]));
    let mut run = 0.0;
    for (i, w) in line.windows(2).enumerate() {
        let (a, b) = (w[0], w[1]);
        let d = b - a;
        let l = d.length();
        let t = if l < 1e-12 {
            0.0
        } else {
            ((p - a).dot(d) / (l * l)).clamp(0.0, 1.0)
        };
        let q = a + d * t;
        if q.distance(p) < best.3 - 1e-9 {
            best = (q, run + l * t, i, q.distance(p));
        }
        run += l;
    }
    (best.0, best.1, best.2)
}

/// The way from `from` along the main route `main` drawn by hand (George
/// 2026-11-28: the connecting pipes along the corridor), off it at its
/// point nearest the room's connection doorway and in through it; into a
/// room the route itself runs into where it first enters it. The rooms
/// the way passes through are its transit rooms, the doorways it crosses
/// its joints.
#[allow(clippy::too_many_arguments)]
pub fn along(
    main: &[DVec2],
    at: DVec2,
    from: DVec2,
    doors: &[Door],
    rooms: &[(ElementId, Vec<DVec2>)],
    start: ElementId,
    to: ElementId,
    by: Option<ElementId>,
) -> Option<Route> {
    if main.len() < 2 {
        return None;
    }
    if start == to {
        return Some(Route {
            points: vec![at, from],
            ..Route::default()
        });
    }
    let outline = &rooms.iter().find(|r| r.0 == to)?.1;
    let inside = |p: DVec2| polygon::contains(outline, p);
    // the route runs into the room itself: up to where it enters it
    // out of the manifold square to it, then on
    let mut points = vec![at, from];
    let mut entered = false;
    for (i, w) in main.windows(2).enumerate() {
        if i == 0 {
            points.push(w[0]);
        }
        let (a, b) = (w[0], w[1]);
        if inside(a) {
            entered = true;
            break;
        }
        // its first crossing of the room's outline on this stretch
        let mut first: Option<f64> = None;
        let n = outline.len();
        for k in 0..n {
            let (p, q) = (outline[k], outline[(k + 1) % n]);
            let (r, e) = (b - a, q - p);
            let den = r.perp_dot(e);
            if den.abs() < 1e-12 {
                continue;
            }
            let t = (p - a).perp_dot(e) / den;
            let v = (p - a).perp_dot(r) / den;
            if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&v) {
                first = Some(first.map_or(t, |f: f64| f.min(t)));
            }
        }
        if let Some(t) = first {
            // a step into it
            let r = b - a;
            let step = (0.3 / r.length().max(1e-9)).min(1.0 - t);
            points.push(a + r * (t + step));
            entered = true;
            break;
        }
        points.push(b);
    }
    let mut out = Route::default();
    if entered {
        let last = points[points.len() - 1];
        out.peel = (on_line(main, last).1, 0.0);
        out.peel_at = last;
    }
    if !entered {
        // off the route at its point nearest the room's doorway, in by it
        points = vec![at, from];
        let mut best: Option<(f64, DVec2, usize, &Door, usize)> = None;
        for d in doors {
            for k in 0..2 {
                if d.rooms[k] != Some(to) || by.is_some_and(|b| b != d.id) {
                    continue;
                }
                let o = d.sides[1 - k];
                let (p, run, i) = on_line(main, o);
                let cost = run + p.distance(o);
                if best.is_none_or(|b| cost < b.0) {
                    best = Some((cost, p, i, d, k));
                }
            }
        }
        match best {
            Some((_, p, i, d, k)) => {
                let (_, run, _) = on_line(main, p);
                let dir = main[(i + 1).min(main.len() - 1)] - main[i.min(main.len() - 2)];
                let s = dir.perp_dot(d.sides[1 - k] - p);
                out.peel = (run, if s.abs() < 1e-6 { 0.0 } else { s.signum() });
                out.peel_at = p;
                points.extend_from_slice(&main[..=i]);
                points.push(p);
                points.push(d.sides[1 - k]);
                points.push(d.sides[k]);
            }
            None => {
                // no doorway: straight across to its nearest edge
                let (q, _, _) = on_line(outline, main[main.len() - 1]);
                let (p, run, i) = on_line(main, q);
                let dir = main[(i + 1).min(main.len() - 1)] - main[i.min(main.len() - 2)];
                let s = dir.perp_dot(q - p);
                out.peel = (run, if s.abs() < 1e-6 { 0.0 } else { s.signum() });
                out.peel_at = p;
                points.extend_from_slice(&main[..=i]);
                points.push(p);
                points.push(q);
            }
        }
    }
    points.dedup_by(|a, b| a.distance(*b) < 1e-6);
    // the doorways it crosses and the rooms it passes through
    let room_at = |p: DVec2| {
        rooms
            .iter()
            .find(|r| r.1.len() > 2 && polygon::contains(&r.1, p))
            .map(|r| r.0)
    };
    out.transit.push(start);
    for w in points.windows(2) {
        let n = (w[1] - w[0]).length().max(1e-9);
        for k in 1..=((n / 0.25).ceil() as usize) {
            let m = w[0].lerp(w[1], (k as f64 - 0.5) / (n / 0.25).ceil());
            if let Some(r) = room_at(m) {
                if r != to && !out.transit.contains(&r) {
                    out.transit.push(r);
                }
            }
        }
        for d in doors {
            // the joint across the doorway, along the wall
            let across = (d.sides[1] - d.sides[0]).normalize_or_zero();
            let e = DVec2::new(-across.y, across.x) * d.width;
            let a = d.at - e / 2.0;
            let r = w[1] - w[0];
            let den = r.perp_dot(e);
            if den.abs() < 1e-12 || out.doors.contains(&d.id) {
                continue;
            }
            let t = (a - w[0]).perp_dot(e) / den;
            let v = (a - w[0]).perp_dot(r) / den;
            if (-1e-6..=1.0 + 1e-6).contains(&t) && (-1e-6..=1.0 + 1e-6).contains(&v) {
                out.doors.push(d.id);
                let x = w[0] + r * t;
                let u = r.normalize_or_zero();
                out.sides.push([x - u * 0.2, x + u * 0.2]);
            }
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

    #[test]
    fn the_pipes_peel_off_the_main_route_to_either_side() {
        // a corridor H along y from (1, 10) down to (1, 0), room L west of
        // it and R east, each with a doorway into H
        let sq = |x0: f64, y0: f64, x1: f64, y1: f64| {
            vec![
                DVec2::new(x0, y0),
                DVec2::new(x1, y0),
                DVec2::new(x1, y1),
                DVec2::new(x0, y1),
            ]
        };
        let (h, l, r) = (ElementId::new(), ElementId::new(), ElementId::new());
        let rooms = vec![
            (h, sq(0.0, 0.0, 2.0, 10.0)),
            (l, sq(-4.0, 2.0, 0.0, 5.0)),
            (r, sq(2.0, 6.0, 6.0, 9.0)),
        ];
        let door = |x: f64, y: f64, out: f64, a, b| Door {
            id: ElementId::new(),
            at: DVec2::new(x, y),
            sides: [DVec2::new(x + out, y), DVec2::new(x - out, y)],
            rooms: [Some(a), Some(b)],
            width: 0.8,
        };
        let doors = vec![door(0.0, 3.5, 0.2, h, l), door(2.0, 7.5, -0.2, h, r)];
        let main = vec![DVec2::new(1.0, 9.8), DVec2::new(1.0, 0.5)];
        let at = DVec2::new(1.0, 10.0);
        let wl = along(&main, at, main[0], &doors, &rooms, h, l, None).unwrap();
        let wr = along(&main, at, main[0], &doors, &rooms, h, r, None).unwrap();
        // going south, east is the left
        assert_eq!(wr.peel.1, 1.0);
        assert_eq!(wl.peel.1, -1.0);
        assert!((wr.peel.0 - 2.3).abs() < 1e-9, "{:?}", wr.peel);
        assert!((wl.peel.0 - 6.3).abs() < 1e-9, "{:?}", wl.peel);
        assert_eq!(wl.points[0], at);
        assert_eq!(wl.transit, vec![h]);
    }
}
