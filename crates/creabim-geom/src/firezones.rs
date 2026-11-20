//! Fire zones (požiarne úseky), ported from PilonDesigner's
//! `firezones.ts`: rooms with the same zone code make a zone. The list of
//! zones with their areas and rooms, the doors on the zones' borders
//! (between rooms of different zones) and whether each of them is a fire
//! door. The fire risk and the degree of fire safety are not worked out
//! here: they need the fire load (STN 92 0201).

use crate::plan::PlanItem;
use crate::schedule::{Column, Row, Schedule};
use creabim_core::words::tr;
use creabim_core::{Element, ElementId, ElementKind, Model, OpeningKind, ScheduleCategory};
use glam::DVec2;

/// A fire zone: its code, its rooms, their area and their storeys.
#[derive(Clone, Debug, PartialEq)]
pub struct Zone {
    pub code: String,
    pub rooms: Vec<ElementId>,
    pub area: f64,
    pub levels: Vec<String>,
}

/// A door on the border of two zones.
#[derive(Clone, Debug, PartialEq)]
pub struct BorderDoor {
    pub door: ElementId,
    pub mark: String,
    /// The zones on its two sides; "-" where a side has none.
    pub zones: [String; 2],
    /// Its fire rating; none is a border left open.
    pub rating: Option<String>,
}

/// The zones of the model, by their codes.
pub fn zones(model: &Model) -> Vec<Zone> {
    let mut out: Vec<Zone> = vec![];
    for e in model.iter() {
        let ElementKind::Room(r) = &e.kind else {
            continue;
        };
        let code = r.fire_zone.trim();
        if code.is_empty() {
            continue;
        }
        let area = creabim_core::room::shape(model, e.id)
            .map_or(0.0, |s| creabim_core::room::net_area(model, r, &s));
        let level = model.get(r.level).map(|l| tr(&l.name)).unwrap_or_default();
        let z = match out.iter_mut().find(|z| z.code == code) {
            Some(z) => z,
            None => {
                out.push(Zone {
                    code: code.to_string(),
                    rooms: vec![],
                    area: 0.0,
                    levels: vec![],
                });
                out.last_mut().unwrap()
            }
        };
        z.rooms.push(e.id);
        z.area += area;
        if !z.levels.contains(&level) {
            z.levels.push(level);
        }
    }
    out.sort_by(|a, b| a.code.cmp(&b.code));
    out
}

/// The zone of the room at `p` on `level`; empty with no room there or
/// none given.
/// The fire zone of the room at `p` on `level` ("" none).
pub fn zone_at(model: &Model, level: ElementId, p: DVec2) -> String {
    model
        .iter()
        .find_map(|e| match &e.kind {
            ElementKind::Room(r) if r.level == level => {
                let s = creabim_core::room::shape(model, e.id)?;
                creabim_core::polygon::contains(&s.outline, p)
                    .then(|| r.fire_zone.trim().to_string())
            }
            _ => None,
        })
        .unwrap_or_default()
}

/// The doors between rooms of different zones, by their marks.
pub fn border_doors(model: &Model) -> Vec<BorderDoor> {
    let marks = creabim_core::marks(model);
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Opening(o) = &e.kind else {
            continue;
        };
        if o.kind != OpeningKind::Door {
            continue;
        }
        let Some(ElementKind::Wall(w)) = model.get(o.host).map(|h| &h.kind) else {
            continue;
        };
        let along = (w.end - w.start).normalize_or_zero();
        let mid = w.start + along * (o.offset + o.width / 2.0);
        let n = DVec2::new(-along.y, along.x) * (w.thickness / 2.0 + 0.15);
        let (a, b) = (
            zone_at(model, w.level, mid + n),
            zone_at(model, w.level, mid - n),
        );
        if a == b {
            continue;
        }
        let dash = |s: String| if s.is_empty() { "-".to_string() } else { s };
        out.push(BorderDoor {
            door: e.id,
            mark: marks.get(&e.id).cloned().unwrap_or_default(),
            zones: [dash(a), dash(b)],
            rating: o
                .door
                .as_ref()
                .and_then(|d| d.fire.as_ref())
                .map(|f| f.label()),
        });
    }
    out.sort_by(|a, b| a.mark.cmp(&b.mark));
    out
}

/// A zone's wash in a fire drawing: one colour a code, the same every
/// time.
pub fn colour_of(code: &str) -> [f32; 3] {
    const PALETTE: [[f32; 3]; 6] = [
        [0.98, 0.80, 0.78],
        [0.80, 0.88, 0.98],
        [0.84, 0.95, 0.80],
        [0.99, 0.93, 0.74],
        [0.90, 0.82, 0.96],
        [0.80, 0.95, 0.93],
    ];
    let h = code
        .bytes()
        .fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
    PALETTE[h as usize % PALETTE.len()]
}

/// The zones as a table, and under them the doors on their borders.
pub fn schedule(model: &Model) -> Schedule {
    let col = |title: &'static str, width: f64, numeric: bool| Column {
        title,
        field: None,
        width,
        numeric,
    };
    let columns = vec![
        col("Fire zone", 30.0, false),
        col("Level", 30.0, false),
        col("Rooms", 70.0, false),
        col("Area", 20.0, true),
        col("Fire closure", 40.0, false),
        col("Required (STN 92 0201-2)", 55.0, false),
    ];
    let units = model.units();
    let name = |id: &ElementId| -> String {
        match model.get(*id) {
            Some(Element {
                name,
                kind: ElementKind::Room(r),
                ..
            }) => format!("{} {name}", r.number).trim().to_string(),
            _ => String::new(),
        }
    };
    let mut rows = vec![];
    let mut total = 0.0;
    for z in zones(model) {
        total += z.area;
        rows.push(Row {
            id: z.rooms[0],
            group: false,
            cells: vec![
                z.code.clone(),
                z.levels.join(", "),
                z.rooms.iter().map(name).collect::<Vec<_>>().join(", "),
                units.area(z.area),
                String::new(),
                String::new(),
            ],
        });
    }
    let risks = crate::firerisk::zones(model);
    let level_name = |id: ElementId| model.get(id).map(|l| tr(&l.name)).unwrap_or_default();
    let verdict = |ok: bool| if ok { tr("meets") } else { tr("DOES NOT MEET") };
    for d in border_doors(model) {
        let level = match model.get(d.door).map(|e| &e.kind) {
            Some(ElementKind::Opening(o)) => match model.get(o.host).map(|e| &e.kind) {
                Some(ElementKind::Wall(w)) => Some(w.level),
                _ => None,
            },
            _ => None,
        };
        let need = level.and_then(|l| {
            use crate::fireexposure::{border_need, zone_grade};
            border_need(
                model,
                l,
                [
                    zone_grade(model, &risks, &d.zones[0]),
                    zone_grade(model, &risks, &d.zones[1]),
                ],
            )
        });
        let required = need.as_ref().map_or(String::new(), |n| {
            let ok = d.rating.as_deref().is_some_and(|r| n.met_by(r));
            format!("{} ({})", n.rating(), verdict(ok))
        });
        rows.push(Row {
            id: d.door,
            group: false,
            cells: vec![
                format!("{} / {}", d.zones[0], d.zones[1]),
                level.map(level_name).unwrap_or_default(),
                format!("{} {}", tr("Door"), d.mark),
                String::new(),
                d.rating.unwrap_or_else(|| tr("NO FIRE DOOR")),
                required,
            ],
        });
    }
    // outer walls with no fire resistance stated: counted as wholly
    // fire-open (unless the designer's `wall_unstated` says otherwise)
    for (id, w, _) in crate::fireexposure::unstated_walls(model) {
        rows.push(Row {
            id,
            group: false,
            cells: vec![
                tr("Outer wall"),
                level_name(w.level),
                format!("{} {:.2} m", tr("Outer wall"), w.start.distance(w.end)),
                String::new(),
                tr("NO FIRE RESISTANCE STATED"),
                tr("counted as a wholly fire-open area (STN 92 0201-2 tab. 5 note 3)"),
            ],
        });
    }
    // the outer insulation (ETICS) against the fire height (Weber s. 16)
    for c in crate::etics::checks(model) {
        let name = model
            .get(c.assembly)
            .map(|e| e.name.clone())
            .unwrap_or_default();
        let mut need = format!(
            "{} {} ({})",
            tr("insulation at least"),
            c.required,
            verdict(c.ok)
        );
        if !c.barriers.is_empty() {
            need.push_str(&format!("; {}: {}", tr("fire barriers"), c.barriers));
        }
        need.push_str(" (Weber ETICS s. 16, STN 73 0802)");
        rows.push(Row {
            id: c.assembly,
            group: false,
            cells: vec![
                "ETICS".into(),
                format!("h = {:.2} m", c.height),
                format!("{name}: {} {:.0} mm", c.insulation, c.thickness * 1000.0),
                String::new(),
                c.class.clone(),
                need,
            ],
        });
    }
    // 5.4.9 to 5.4.11: outer walls in another zone's fire-dangerous
    // space, and their openings
    let marks = creabim_core::marks(model);
    for x in crate::fireexposure::exposed(model) {
        let wall_rating = x
            .declared
            .map_or_else(|| tr("not stated"), |m| format!("{m:.0} min"));
        rows.push(Row {
            id: x.wall,
            group: false,
            cells: vec![
                format!("{} {} {}", x.zone, tr("in PNP of"), x.from),
                level_name(x.level),
                format!(
                    "{} (d = {:.2} m, d'/d = {:.2}, R = {:.0} min)",
                    tr("Outer wall"),
                    x.d,
                    x.ratio,
                    x.r
                ),
                String::new(),
                wall_rating,
                format!(
                    "Ro {:.0} min {}, {}{}",
                    x.ro.ceil(),
                    tr("from outside"),
                    if x.d1 { "D1" } else { "D1/D2" },
                    x.met()
                        .map_or(String::new(), |ok| format!(" ({})", verdict(ok)))
                ),
            ],
        });
        for o in &x.openings {
            let ok = o.rating.as_deref().is_some_and(|r| o.need.met_by(r));
            rows.push(Row {
                id: o.id,
                group: false,
                cells: vec![
                    format!("{} {} {}", x.zone, tr("in PNP of"), x.from),
                    level_name(x.level),
                    format!(
                        "{} {}",
                        if o.door { tr("Door") } else { tr("Window") },
                        marks.get(&o.id).cloned().unwrap_or_default()
                    ),
                    String::new(),
                    o.rating.clone().unwrap_or_else(|| tr("NO FIRE CLOSURE")),
                    format!("{}-C ({})", o.need.rating(), verdict(ok)),
                ],
            });
        }
    }
    let listed = rows.len();
    Schedule {
        category: ScheduleCategory::FireZones,
        columns,
        rows,
        totals: vec![
            tr("Total"),
            String::new(),
            String::new(),
            units.area(total),
            String::new(),
            String::new(),
        ],
        listed,
    }
}

/// The fire height of the building (the annex of decree 94/2004 Z. z.
/// on the first above-ground fire storey and the fire height): the
/// first above-ground fire storey is the lowest whose floor is no more
/// than 1.5 m under the highest ground beside the building; the height
/// runs from its floor to the floor of the last storey.
#[derive(Clone, Debug, PartialEq)]
pub struct FireHeight {
    pub first: ElementId,
    pub last: ElementId,
    /// From the first above-ground fire storey's floor to the last's, m.
    pub height: f64,
    /// Storeys above ground and under it, as fire counts them.
    pub above: usize,
    pub below: usize,
    /// The highest ground at the building against the project's zero;
    /// none when the model has no terrain there (the zero is taken).
    pub ground: Option<f64>,
}

/// How far under the highest ground beside it a storey's floor may be
/// and the storey still count as above ground, m.
pub const FIRST_STOREY_DROP: f64 = 1.5;

pub fn fire_height(model: &Model) -> Option<FireHeight> {
    let storeys: Vec<_> = creabim_core::levels::sorted(model)
        .into_iter()
        .filter(|(id, ..)| {
            model
                .iter()
                .any(|e| matches!(&e.kind, ElementKind::Wall(w) if w.level == *id))
        })
        .collect();
    let ground = crate::summary::ground_at_building(model).map(|g| g.1);
    let limit = ground.unwrap_or(0.0) - FIRST_STOREY_DROP;
    let first = storeys.iter().position(|s| s.2.elevation >= limit - 1e-9)?;
    let last = storeys.last()?;
    Some(FireHeight {
        first: storeys[first].0,
        last: last.0,
        height: last.2.elevation - storeys[first].2.elevation,
        above: storeys.len() - first,
        below: first,
        ground,
    })
}

/// A side of the building on a storey: its outer walls facing one way,
/// with the openings in them that fire can pass (windows, and doors
/// with no fire rating).
#[derive(Clone, Debug, PartialEq)]
pub struct Facade {
    pub level: ElementId,
    /// Where it faces: 0 north, 1 north-east, on clockwise to 7.
    pub facing: usize,
    /// Its length, m, and its area, m².
    pub length: f64,
    pub area: f64,
    /// The area of its openings fire can pass, m².
    pub open: f64,
    pub first: ElementId,
}

impl Facade {
    /// The share of its area fire can pass, per cent.
    pub fn open_percent(&self) -> f64 {
        if self.area > 0.0 {
            self.open / self.area * 100.0
        } else {
            0.0
        }
    }
}

/// The sides' names, each in the compass's context for translation.
pub const FACING: [&str; 8] = [
    "compass\u{4}N",
    "compass\u{4}NE",
    "compass\u{4}E",
    "compass\u{4}SE",
    "compass\u{4}S",
    "compass\u{4}SW",
    "compass\u{4}W",
    "compass\u{4}NW",
];

/// Is there a room at `p` on `level`?
pub fn room_at(model: &Model, level: ElementId, p: DVec2) -> bool {
    model.iter().any(|e| match &e.kind {
        ElementKind::Room(r) if r.level == level => creabim_core::room::shape(model, e.id)
            .is_some_and(|s| creabim_core::polygon::contains(&s.outline, p)),
        _ => false,
    })
}

/// An outer wall: one with a room on one side only.
pub struct OuterWall<'a> {
    pub id: ElementId,
    pub wall: &'a creabim_core::Wall,
    /// The way out of the building, a unit vector.
    pub outward: DVec2,
    /// Where it faces: 0 north, on clockwise to 7.
    pub facing: usize,
}

/// The outer walls of the model, each with where it faces.
pub fn outer_walls(model: &Model) -> Vec<OuterWall<'_>> {
    let north = model
        .project_info()
        .map_or(0.0, |p| p.1.true_north)
        .to_radians();
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Wall(w) = &e.kind else {
            continue;
        };
        let along = (w.end - w.start).normalize_or_zero();
        if w.start.distance(w.end) < 1e-6 {
            continue;
        }
        let mid = (w.start + w.end) / 2.0;
        let n = DVec2::new(-along.y, along.x);
        let off = w.thickness / 2.0 + 0.15;
        let (left, right) = (
            room_at(model, w.level, mid + n * off),
            room_at(model, w.level, mid - n * off),
        );
        if left == right {
            continue;
        }
        let outward = if left { -n } else { n };
        // clockwise from true north, which is `true_north` anticlockwise
        // from the drawings' up
        let up = DVec2::from_angle(north).rotate(DVec2::Y);
        let angle = up.angle_to(outward);
        let facing = ((-angle.to_degrees()).rem_euclid(360.0) / 45.0).round() as usize % 8;
        out.push(OuterWall {
            id: e.id,
            wall: w,
            outward,
            facing,
        });
    }
    out
}

/// The separation distance given for the side facing `facing`, m.
pub fn distance_of(model: &Model, facing: usize) -> Option<f64> {
    model
        .project_info()
        .and_then(|p| p.1.fire_distances.get(facing).copied())
        .filter(|d| d.is_finite() && *d > 0.0)
}

/// The colour the fire-dangerous space is drawn in.
pub const DANGER: [f32; 3] = [0.86, 0.2, 0.16];

/// The fire-dangerous space about the building as a plan draws it: from
/// each outer wall whose side has a separation distance, a strip that
/// deep, washed and outlined, the distance written at the longest wall
/// of the side. `mm` is metres to a paper millimetre.
pub fn distance_items(model: &Model, mm: f64) -> Vec<PlanItem> {
    use crate::plan::Pen;
    let walls = outer_walls(model);
    let mut items = vec![];
    let mut seen: Vec<[DVec2; 2]> = vec![];
    let mut longest: [Option<(f64, ElementId, DVec2, DVec2)>; 8] = [None; 8];
    for o in &walls {
        let Some(d) = distance_of(model, o.facing) else {
            continue;
        };
        let w = o.wall;
        let face = o.outward * (w.thickness / 2.0);
        let (a, b) = (w.start + face, w.end + face);
        // walls of storeys one over another make one strip
        if seen
            .iter()
            .any(|s| s[0].distance(a) < 1e-3 && s[1].distance(b) < 1e-3)
        {
            continue;
        }
        seen.push([a, b]);
        let (c, e) = (b + o.outward * d, a + o.outward * d);
        items.push(PlanItem::Tint {
            id: o.id,
            points: vec![a, b, c, e],
            rgb: DANGER,
        });
        for (p, q) in [(a, e), (e, c), (c, b)] {
            items.push(PlanItem::Line {
                id: o.id,
                pen: Pen::Overhead,
                a: p,
                b: q,
            });
        }
        let len = a.distance(b);
        if longest[o.facing].is_none_or(|l| len > l.0) {
            longest[o.facing] = Some((len, o.id, (c + e) / 2.0, o.outward));
        }
    }
    for (facing, l) in longest.iter().enumerate() {
        let (Some((_, id, at, outward)), Some(d)) = (l, distance_of(model, facing)) else {
            continue;
        };
        items.push(PlanItem::Text {
            id: *id,
            at: *at + *outward * 2.5 * mm,
            text: format!("d = {d:.2} m").replace('.', ","),
            size: 2.5,
            align: crate::paper::Anchor::Middle,
            mm,
            rgb: Some(DANGER),
            bold: false,
        });
    }
    items
}

/// The building's sides, storey by storey: walls with a room on one
/// side only, gathered by where they face.
pub fn facades(model: &Model) -> Vec<Facade> {
    let mut out: Vec<Facade> = vec![];
    for o in outer_walls(model) {
        let (w, facing) = (o.wall, o.facing);
        let len = w.start.distance(w.end);
        let open: f64 = model
            .iter()
            .filter_map(|x| match &x.kind {
                ElementKind::Opening(x) if x.host == o.id => {
                    let fire = x.door.as_ref().is_some_and(|d| d.fire.is_some());
                    (!fire).then_some(x.width * x.height)
                }
                _ => None,
            })
            .sum();
        let f = match out
            .iter_mut()
            .find(|f| f.level == w.level && f.facing == facing)
        {
            Some(f) => f,
            None => {
                out.push(Facade {
                    level: w.level,
                    facing,
                    length: 0.0,
                    area: 0.0,
                    open: 0.0,
                    first: o.id,
                });
                out.last_mut().unwrap()
            }
        };
        f.length += len;
        f.area += len * w.height;
        f.open += open;
    }
    let z = |f: &Facade| model.level(f.level).map_or(0.0, |l| l.elevation);
    out.sort_by(|a, b| z(a).total_cmp(&z(b)).then(a.facing.cmp(&b.facing)));
    out
}

/// The building's sides as a table: what of each is open to fire, the
/// figure the separation distances start from.
pub fn facade_schedule(model: &Model) -> Schedule {
    let col = |title: &'static str, width: f64, numeric: bool| Column {
        title,
        field: None,
        width,
        numeric,
    };
    let columns = vec![
        col("Level", 34.0, false),
        col("Facing", 16.0, false),
        col("Length", 20.0, true),
        col("Facade area", 24.0, true),
        col("Fire-open area", 28.0, true),
        col("Fire-open share", 26.0, true),
        col("Separation distance", 30.0, true),
    ];
    let units = model.units();
    let rows: Vec<Row> = facades(model)
        .into_iter()
        .map(|f| Row {
            id: f.first,
            group: false,
            cells: vec![
                model.get(f.level).map(|l| tr(&l.name)).unwrap_or_default(),
                tr(FACING[f.facing]),
                format!("{:.2} m", f.length).replace('.', ","),
                units.area(f.area),
                units.area(f.open),
                format!("{:.0} %", f.open_percent()),
                distance_of(model, f.facing)
                    .map(|d| format!("{d:.2} m").replace('.', ","))
                    .unwrap_or_default(),
            ],
        })
        .collect();
    let listed = rows.len();
    Schedule {
        category: ScheduleCategory::FireFacades,
        columns,
        rows,
        totals: vec![],
        listed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::Transaction;

    /// Rooms with one code make a zone; a door between two zones is on
    /// their border, and without a fire rating it is flagged.
    #[test]
    fn rooms_make_zones_and_a_plain_door_between_two_is_flagged() {
        let mut m = creabim_core::starter::starter_house();
        assert!(zones(&m).is_empty() && border_doors(&m).is_empty());
        let rooms: Vec<Element> = m
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Room(_)))
            .cloned()
            .collect();
        let mut tx = Transaction::new("zones");
        for (i, e) in rooms.iter().enumerate() {
            let mut n = e.clone();
            if let ElementKind::Room(r) = &mut n.kind {
                r.fire_zone = if i == 0 { "N1.02" } else { "N1.01" }.into();
            }
            tx = tx.replace(e.clone(), n);
        }
        tx.apply(&mut m).unwrap();
        let z = zones(&m);
        assert_eq!(
            z.iter().map(|z| z.code.as_str()).collect::<Vec<_>>(),
            ["N1.01", "N1.02"]
        );
        assert_eq!(z[1].rooms.len(), 1);
        assert_eq!(z[0].rooms.len(), rooms.len() - 1);
        // every door of the lone room's zone is on a border
        let doors = border_doors(&m);
        assert!(!doors.is_empty());
        assert!(doors.iter().all(|d| d.zones[0] != d.zones[1]));
        let s = schedule(&m);
        // the starter's insulated outer walls add their ETICS lines
        let etics = s.rows.iter().filter(|r| r.cells[0] == "ETICS").count();
        assert_eq!(etics, crate::etics::checks(&m).len());
        assert_eq!(s.rows.len() - etics, 2 + doors.len());
        assert!(s.rows.iter().any(|r| r.cells[4] == "NO FIRE DOOR"));
    }

    /// The fire height runs from the floor of the first storey above
    /// ground to the floor of the last.
    #[test]
    fn the_fire_height_is_between_the_first_and_the_last_floor() {
        let m = creabim_core::starter::starter_house();
        let h = fire_height(&m).unwrap();
        assert_eq!((h.above, h.below), (2, 0));
        let z = |id| m.level(id).unwrap().elevation;
        assert!((h.height - (z(h.last) - z(h.first))).abs() < 1e-9 && h.height > 2.0);
    }

    /// The outer walls of a storey gather by where they face; a side's
    /// openings are what of it fire can pass.
    #[test]
    fn the_sides_of_the_house_know_what_of_them_is_open() {
        let m = creabim_core::starter::starter_house();
        let f = facades(&m);
        assert!(f.len() >= 4, "{f:?}");
        assert!(f.iter().all(|f| f.area > 0.0 && f.open <= f.area));
        assert!(f.iter().any(|f| f.open > 0.0));
        let s = facade_schedule(&m);
        assert_eq!(s.rows.len(), f.len());
        assert!(s.rows[0].cells[5].ends_with('%'));
    }

    #[test]
    fn a_side_with_a_distance_draws_its_fire_dangerous_space() {
        let mut m = creabim_core::starter::starter_house();
        assert!(distance_items(&m, 0.05).is_empty());
        let mut info = m.project_info().map(|p| p.1.clone()).unwrap_or_default();
        // south only
        info.fire_distances = vec![0.0, 0.0, 0.0, 0.0, 4.5];
        let tx = creabim_core::set_project(&m, info);
        tx.apply(&mut m).unwrap();
        assert_eq!(distance_of(&m, 4), Some(4.5));
        assert_eq!(distance_of(&m, 0), None);
        let items = distance_items(&m, 0.05);
        let south: Vec<_> = outer_walls(&m)
            .into_iter()
            .filter(|o| o.facing == 4)
            .collect();
        assert!(!south.is_empty());
        // each strip reaches 4.5 m out of its wall's face
        let strips: Vec<_> = items
            .iter()
            .filter_map(|it| match it {
                PlanItem::Tint { points, .. } => Some(points),
                _ => None,
            })
            .collect();
        assert!(!strips.is_empty() && strips.len() <= south.len());
        for p in &strips {
            assert!((p[1].distance(p[2]) - 4.5).abs() < 1e-9);
        }
        // and the distance is written once for the side
        let texts: Vec<_> = items
            .iter()
            .filter_map(|it| match it {
                PlanItem::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, ["d = 4,50 m"]);
        // the table of sides carries it
        let s = facade_schedule(&m);
        assert!(s.rows.iter().any(|r| r.cells.last().unwrap() == "4,50 m"));
    }
}
