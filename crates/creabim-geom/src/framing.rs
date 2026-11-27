//! The roof's timber and the steel members on drawings and in lists
//! (see `creabim_core::framing`): the underside written in a box by each
//! (`S.H.=+3,680`), the list of timber (výkaz reziva) as George's sample
//! krov has it, and the list of steel members with their mass as the
//! structural engineer's "Výpis materiálu".

use crate::schedule::{Column, Row, Schedule};
use creabim_core::framing::{self, Position, Shape};
use creabim_core::{ElementId, Model};

fn comma(v: f64, places: usize) -> String {
    format!("{v:.places$}").replace('.', ",")
}

/// A height above ±0,000 as a drawing writes it: `+3,680`.
fn level_text(z: f64) -> String {
    let s = comma(z.abs(), 3);
    if z < -5e-4 {
        format!("-{s}")
    } else {
        format!("+ {s}")
    }
}

/// The lowest point of member `id`'s underside above ±0,000, m.
pub fn underside(model: &Model, id: ElementId) -> Option<f64> {
    let e = model.get(id)?;
    match &e.kind {
        creabim_core::ElementKind::Beam(b) => {
            let (a, z) = b.axis(model);
            let up = b.up.unwrap_or(glam::DVec3::Z).normalize_or(glam::DVec3::Z);
            // the section's depth hangs down square to the axis
            Some(a.z.min(z.z) - b.depth * up.z)
        }
        creabim_core::ElementKind::Column(c) => Some(c.span(model).0),
        _ => None,
    }
}

/// Is member `id` a beam lying on a slope (a rafter), its underside no
/// one height?
pub fn sloped(model: &Model, id: ElementId) -> bool {
    matches!(
        model.get(id).map(|e| &e.kind),
        Some(creabim_core::ElementKind::Beam(b)) if (b.rise[1] - b.rise[0]).abs() > 1e-6
    )
}

/// What a member's underside tag reads: `S.H.=+ 3,680`.
pub fn underside_text(model: &Model, id: ElementId) -> String {
    underside(model, id).map_or_else(String::new, |z| format!("S.H.={}", level_text(z)))
}

fn col(title: &'static str, width: f64, numeric: bool) -> Column {
    Column {
        title,
        field: None,
        width,
        numeric,
        align: None,
    }
}

fn mm(v: f64) -> String {
    format!("{:.0}", v * 1000.0)
}

/// The list of timber (výkaz reziva), as George's sample: mark on the
/// drawing, name, pieces, width and height in mm, length of one and of
/// all in m, volume in m³, and the timber in all.
pub fn timber_schedule(model: &Model) -> Schedule {
    timber_schedule_by(model, false)
}

/// Positions alike in section and grade taken together, for the budget
/// (George 2026-11-26): what they are, how many, their total length,
/// volume or mass and turnbuckles; the first position's first member
/// standing for the row.
struct Lot<'a> {
    first: &'a Position,
    roles: Vec<String>,
    count: usize,
    length: f64,
    volume: f64,
    mass: Option<f64>,
    turnbuckles: usize,
}

fn lots(positions: &[Position]) -> Vec<Lot<'_>> {
    use creabim_core::words::tr;
    let mut out: Vec<Lot> = Vec::new();
    for p in positions {
        let key = (framing::section_label(&p.section), p.grade.clone());
        let found = out.iter_mut().find(|l| {
            (
                framing::section_label(&l.first.section),
                l.first.grade.clone(),
            ) == key
        });
        let lot = match found {
            Some(l) => l,
            None => {
                out.push(Lot {
                    first: p,
                    roles: vec![],
                    count: 0,
                    length: 0.0,
                    volume: 0.0,
                    mass: Some(0.0),
                    turnbuckles: 0,
                });
                out.last_mut().unwrap()
            }
        };
        let role = tr(p.role.label());
        if !lot.roles.contains(&role) {
            lot.roles.push(role);
        }
        lot.count += p.count();
        lot.length += p.length * p.count() as f64;
        lot.volume += p.volume();
        lot.mass = lot.mass.zip(p.mass()).map(|(a, b)| a + b);
        lot.turnbuckles += p.turnbuckles;
    }
    out
}

/// The list of timber, by position mark or (`by_section`) with the
/// members of one cross-section and grade together, their lengths summed.
pub fn timber_schedule_by(model: &Model, by_section: bool) -> Schedule {
    use creabim_core::words::tr;
    let columns = vec![
        col("Mark on drawing", 14.0, false),
        col("Member", 34.0, false),
        col("Pieces", 9.0, true),
        col("Width (mm)", 11.0, true),
        col("Height (mm)", 11.0, true),
        col("Length of one (m)", 15.0, true),
        col("Total length (m)", 15.0, true),
        col("Volume (m³)", 15.0, true),
    ];
    let positions: Vec<Position> = framing::positions(model, true);
    let rows: Vec<Row> = if by_section {
        lots(&positions)
            .iter()
            .map(|l| Row {
                id: l.first.members[0],
                cells: vec![
                    String::new(),
                    l.roles.join(", "),
                    l.count.to_string(),
                    mm(l.first.section.b),
                    mm(l.first.section.h),
                    String::new(),
                    comma(l.length, 3),
                    comma(l.volume, 6),
                ],
                group: false,
            })
            .collect()
    } else {
        positions
            .iter()
            .map(|p| Row {
                id: p.members[0],
                cells: vec![
                    p.mark.clone(),
                    tr(p.role.label()),
                    p.count().to_string(),
                    mm(p.section.b),
                    mm(p.section.h),
                    comma(p.length, 3),
                    comma(p.length * p.count() as f64, 3),
                    comma(p.volume(), 6),
                ],
                group: false,
            })
            .collect()
    };
    let mut rows = rows;
    rows.extend(bolt_rows(model, 8));
    let volume: f64 = positions.iter().map(Position::volume).sum();
    let paint: f64 = positions.iter().map(Position::paint_area).sum();
    Schedule {
        category: creabim_core::ScheduleCategory::TimberList,
        columns,
        listed: rows.len(),
        rows,
        // the paint for all of it under the timber in all (George
        // 2026-11-26: one figure, not by member)
        totals: vec![
            tr("Total paint area (m²)"),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            comma(paint, 2),
        ],
        above_totals: {
            let mut a = vec![sum_row(tr("Timber in all"), None, volume)];
            let laths = batten_rows(model);
            if !laths.is_empty() {
                let all = volume + laths.iter().map(|x| x.1).sum::<f64>();
                for (text, v, l) in laths {
                    a.push(sum_row(text, Some(l), v));
                }
                a.push(sum_row(tr("Timber altogether"), None, all));
            }
            a
        },
    }
}

/// A row over the totals: its text, the length (m) if any, the volume.
fn sum_row(text: String, length: Option<f64>, volume: f64) -> Vec<String> {
    let mut r = vec![String::new(); 8];
    r[0] = text;
    if let Some(l) = length {
        r[6] = comma(l, 2);
    }
    r[7] = comma(volume, 6);
    r
}

/// The roofs' laths and counter-laths, alike ones together, as George's
/// list gives them: "Latovanie pod strešnú krytinu 50 x 40 vo
/// vzdialenosti 33,53 cm" and "Latovanie pozdĺž krokiev 50 x 50
/// celkovej dĺžky l=204,55 m": text, volume m³, length m.
fn batten_rows(model: &Model) -> Vec<(String, f64, f64)> {
    use creabim_core::words::tr;
    let mut laths: Vec<([f64; 2], f64, f64)> = vec![];
    let mut counters: Vec<([f64; 2], f64)> = vec![];
    for r in framing::batten_runs(model) {
        let b = &r.battens;
        match laths
            .iter_mut()
            .find(|x| x.0 == b.lath && (x.1 - b.spacing).abs() < 1e-6)
        {
            Some(x) => x.2 += r.laths,
            None => laths.push((b.lath, b.spacing, r.laths)),
        }
        if let Some(c) = b.counter {
            match counters.iter_mut().find(|x| x.0 == c) {
                Some(x) => x.1 += r.counter,
                None => counters.push((c, r.counter)),
            }
        }
    }
    let mm0 = |v: f64| format!("{:.0}", v * 1000.0);
    let mut out = vec![];
    for (s, sp, l) in laths {
        let text = tr("Battens under the roofing {b} x {h} at {s} cm")
            .replace("{b}", &mm0(s[0]))
            .replace("{h}", &mm0(s[1]))
            .replace("{s}", &comma(sp * 100.0, 2));
        out.push((text, l * s[0] * s[1], l));
    }
    for (s, l) in counters {
        let text = tr("Battens along the rafters {b} x {h}, total length l={l} m")
            .replace("{b}", &mm0(s[0]))
            .replace("{h}", &mm0(s[1]))
            .replace("{l}", &comma(l, 2));
        out.push((text, l * s[0] * s[1], l));
    }
    out
}

/// The collar ties' bolts under the timber, alike ones together: the
/// threaded rod with its washers and nuts, how many, the rod's length
/// (George 2026-11-27). `n` cells a row.
fn bolt_rows(model: &Model, n: usize) -> Vec<Row> {
    use creabim_core::words::tr;
    let mut lots: Vec<(u32, f64, usize, ElementId)> = vec![];
    for b in framing::bolts(model) {
        let l = b.length();
        match lots
            .iter_mut()
            .find(|x| x.0 == b.thread && (x.1 - l).abs() < 1e-6)
        {
            Some(x) => x.2 += 1,
            None => lots.push((b.thread, l, 1, b.collars[0])),
        }
    }
    lots.sort_by(|a, b| (a.0, a.1).partial_cmp(&(b.0, b.1)).unwrap());
    lots.into_iter()
        .map(|(t, l, k, id)| {
            let mut cells = vec![String::new(); n];
            cells[1] = format!(
                "{} M{t} × {:.0}, {}",
                tr("Bolt"),
                l * 1000.0,
                tr("2 washers DIN 440 R, 2 nuts DIN 934")
            );
            cells[2] = k.to_string();
            cells[5] = comma(l, 3);
            cells[6] = comma(l * k as f64, 3);
            Row {
                id,
                cells,
                group: false,
            }
        })
        .collect()
}

/// The list of steel members (výpis oceľových prvkov): mark, member,
/// section, grade, pieces, length of one and of all, kg/m and mass, and
/// the total mass of the structure, as the structural engineer's "Výpis
/// materiálu"; a bracing rod's turnbuckles under it.
pub fn steel_schedule(model: &Model) -> Schedule {
    steel_schedule_by(model, false)
}

/// A bracing rod's turnbuckles, a row under it.
fn turnbuckle_row(id: ElementId, t: &framing::Turnbuckle, n: usize) -> Row {
    use creabim_core::words::tr;
    Row {
        id,
        cells: vec![
            String::new(),
            format!("{} M{}", tr("Turnbuckle"), t.thread),
            format!("Rothoblaas {}", t.code),
            "S355".into(),
            n.to_string(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
        ],
        group: false,
    }
}

/// The list of KARI mesh in the base slabs (George 2026-11-27): the
/// number of its row in the legend of the foundations (a mesh type
/// and edge each), the mesh, the edge, the storey, the area reinforced, the lap,
/// the sheets laid with their laps, kg a sheet (WAGNER table) and the
/// weight in tonnes, with the total.
pub fn kari_schedule(model: &Model) -> Schedule {
    use creabim_core::words::tr;
    let columns = vec![
        col("Mark", 20.0, false),
        col("Mesh", 46.0, false),
        col("Edge of the slab", 14.0, false),
        col("Storey", 20.0, false),
        col("Area (m²)", 18.0, true),
        col("Lap (mm)", 20.0, true),
        col("Pieces", 12.0, true),
        col("kg/piece", 14.0, true),
        col("Weight (t)", 22.0, true),
    ];
    let laid = creabim_core::kari::laid(model);
    let mut marks: Vec<(&str, bool)> = vec![];
    let (mut rows, mut pieces, mut kg) = (vec![], 0, 0.0);
    for l in &laid {
        let key = (l.mesh.code, l.top);
        let i = marks.iter().position(|m| *m == key).unwrap_or_else(|| {
            marks.push(key);
            marks.len() - 1
        });
        pieces += l.pieces;
        kg += l.kg();
        rows.push(Row {
            id: l.slabs[0],
            cells: vec![
                (i + 1).to_string(),
                l.mesh.designation(),
                tr(if l.top { "top edge" } else { "bottom edge" }),
                model
                    .get(l.level)
                    .map_or_else(String::new, |e| e.name.clone()),
                comma(l.area, 2),
                format!("{:.0}", l.lap * 1000.0),
                l.pieces.to_string(),
                comma(l.mesh.kg_sheet, 2),
                comma(l.kg() / 1000.0, 3),
            ],
            group: false,
        });
    }
    Schedule {
        category: creabim_core::ScheduleCategory::KariList,
        columns,
        listed: laid.len(),
        rows,
        totals: vec![
            tr("Total"),
            String::new(),
            String::new(),
            String::new(),
            comma(laid.iter().map(|l| l.area).sum(), 2),
            String::new(),
            pieces.to_string(),
            String::new(),
            comma(kg / 1000.0, 3),
        ],
        above_totals: vec![],
    }
}

/// The list of steel members, by position mark or (`by_section`) with the
/// members of one section and grade together, lengths and masses summed.
pub fn steel_schedule_by(model: &Model, by_section: bool) -> Schedule {
    use creabim_core::words::tr;
    let columns = vec![
        col("Mark", 10.0, false),
        col("Member", 26.0, false),
        col("Cross-section", 20.0, false),
        col("Grade", 10.0, false),
        col("Pieces", 8.0, true),
        col("Length of one (m)", 14.0, true),
        col("Total length (m)", 14.0, true),
        col("kg/m", 10.0, true),
        col("Mass (kg)", 13.0, true),
    ];
    let positions: Vec<Position> = framing::positions(model, false);
    let mut rows = vec![];
    let mut total = 0.0;
    let paint: f64 = positions.iter().map(Position::paint_area).sum();
    if by_section {
        for l in lots(&positions) {
            let kg = l.mass.unwrap_or(0.0);
            total += kg;
            rows.push(Row {
                id: l.first.members[0],
                cells: vec![
                    String::new(),
                    l.roles.join(", "),
                    framing::section_label(&l.first.section),
                    l.first.grade.clone(),
                    l.count.to_string(),
                    String::new(),
                    comma(l.length, 3),
                    l.first
                        .section
                        .mass
                        .map_or_else(String::new, |m| comma(m, 2)),
                    comma(kg, 2),
                ],
                group: false,
            });
            if l.turnbuckles > 0 && l.first.section.shape == Shape::Bar {
                if let Some(t) = framing::turnbuckle_for(l.first.section.b) {
                    rows.push(turnbuckle_row(l.first.members[0], &t, l.turnbuckles));
                }
            }
        }
    }
    for p in positions.iter().filter(|_| !by_section) {
        let kg = p.mass().unwrap_or(0.0);
        total += kg;
        rows.push(Row {
            id: p.members[0],
            cells: vec![
                p.mark.clone(),
                tr(p.role.label()),
                framing::section_label(&p.section),
                p.grade.clone(),
                p.count().to_string(),
                comma(p.length, 3),
                comma(p.length * p.count() as f64, 3),
                p.section.mass.map_or_else(String::new, |m| comma(m, 2)),
                comma(kg, 2),
            ],
            group: false,
        });
        if p.turnbuckles > 0 && p.section.shape == Shape::Bar {
            if let Some(t) = framing::turnbuckle_for(p.section.b) {
                rows.push(turnbuckle_row(p.members[0], &t, p.turnbuckles));
            }
        }
    }
    Schedule {
        category: creabim_core::ScheduleCategory::SteelList,
        columns,
        listed: positions.len(),
        rows,
        totals: vec![
            tr("Total paint area (m²)"),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            comma(paint, 2),
        ],
        above_totals: vec![vec![
            tr("Total mass of the structure"),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            comma(total, 2),
        ]],
    }
}

// ---------------------------------------------------------------- the plan

/// The heights of member `e`'s underside and top over plan point `p`, m
/// (a sloped member at that point along it).
fn heights_at(model: &Model, e: &creabim_core::Element, p: glam::DVec2) -> Option<(f64, f64)> {
    match &e.kind {
        creabim_core::ElementKind::Beam(b) => {
            let (a, z) = b.axis(model);
            let d = (z - a).truncate();
            let t = if d.length_squared() < 1e-12 {
                0.0
            } else {
                ((p - a.truncate()).dot(d) / d.length_squared()).clamp(0.0, 1.0)
            };
            let top = a.z + (z.z - a.z) * t;
            let up = b.up.unwrap_or(glam::DVec3::Z).normalize_or(glam::DVec3::Z);
            // the depth square to a sloped axis is deeper plumb
            Some((top - b.depth / up.z.max(0.1), top))
        }
        creabim_core::ElementKind::Column(c) => Some(c.span(model)),
        _ => None,
    }
}

/// Is `p` well inside polygon `poly` (not on its edge)?
fn inside(poly: &[glam::DVec2], p: glam::DVec2) -> bool {
    let n = poly.len();
    let mut odd = false;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        // on an edge: not inside
        let ab = b - a;
        let l = ab.length();
        if l > 1e-12 {
            let t = (p - a).dot(ab) / (l * l);
            if (-1e-9..=1.0 + 1e-9).contains(&t) && (p - (a + ab * t)).length() < 1e-4 {
                return false;
            }
        }
        if (a.y > p.y) != (b.y > p.y) {
            let x = a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x);
            if p.x < x {
                odd = !odd;
            }
        }
    }
    odd
}

/// Where segment `a`–`b` crosses the edges of `poly`, as parameters 0..1.
fn crossings(a: glam::DVec2, b: glam::DVec2, poly: &[glam::DVec2], out: &mut Vec<f64>) {
    let d = b - a;
    let n = poly.len();
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let e = q - p;
        let den = d.perp_dot(e);
        if den.abs() < 1e-12 {
            continue;
        }
        let t = (p - a).perp_dot(e) / den;
        let u = (p - a).perp_dot(d) / den;
        if (0.0..=1.0).contains(&t) && (-1e-9..=1.0 + 1e-9).contains(&u) {
            out.push(t);
        }
    }
}

/// A framing member's plan as the krov drawing has it (George
/// 2026-11-26): a rafter, hip or valley rafter one thin dash-dot line
/// along its axis; the others' seen edges broken off where a member
/// lying over them hides them, so the lines don't run across each other.
/// `items` is the member's plan as other members are drawn.
pub fn plan_items(
    model: &Model,
    e: &creabim_core::Element,
    items: Vec<crate::plan::PlanItem>,
) -> Vec<crate::plan::PlanItem> {
    use crate::plan::{Pen, PlanItem};
    let Some(f) = framing::of(e) else {
        return items;
    };
    if f.role.schematic() {
        if let creabim_core::ElementKind::Beam(b) = &e.kind {
            let look = creabim_core::LineLook {
                pen: creabim_core::LinePen::Thin,
                kind: creabim_core::LineType::DashDot,
            };
            return vec![PlanItem::Line {
                id: e.id,
                pen: Pen::Drafted(look),
                a: b.start,
                b: b.end,
            }];
        }
    }
    // the members that can lie over it: not the schematic ones
    let over: Vec<(&creabim_core::Element, Vec<glam::DVec2>)> = model
        .iter()
        .filter(|o| o.id != e.id)
        .filter_map(|o| {
            let fo = framing::of(o)?;
            if fo.role.schematic() {
                return None;
            }
            let outline = match &o.kind {
                creabim_core::ElementKind::Beam(b) => b.outline(),
                creabim_core::ElementKind::Column(c) => c.outline(),
                _ => return None,
            };
            Some((o, outline))
        })
        .collect();
    let mut out = Vec::new();
    for it in items {
        let PlanItem::Line {
            id,
            pen: Pen::Projection,
            a,
            b,
        } = it
        else {
            out.push(it);
            continue;
        };
        let mut ts = vec![0.0, 1.0];
        for (_, poly) in &over {
            crossings(a, b, poly, &mut ts);
        }
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|x, y| (*x - *y).abs() < 1e-9);
        let mut run: Option<(f64, f64)> = None;
        let flush = |run: &mut Option<(f64, f64)>, out: &mut Vec<PlanItem>| {
            if let Some((t0, t1)) = run.take() {
                out.push(PlanItem::Line {
                    id,
                    pen: Pen::Projection,
                    a: a + (b - a) * t0,
                    b: a + (b - a) * t1,
                });
            }
        };
        for w in ts.windows(2) {
            let m = a + (b - a) * ((w[0] + w[1]) / 2.0);
            let Some((_, mine)) = heights_at(model, e, m) else {
                continue;
            };
            // hidden under a member whose underside is at or over this
            // one's top there
            let hidden = over.iter().any(|(o, poly)| {
                inside(poly, m) && heights_at(model, o, m).is_some_and(|(lo, _)| lo >= mine - 0.02)
            });
            if hidden {
                flush(&mut run, &mut out);
            } else {
                match &mut run {
                    Some((_, end)) if (*end - w[0]).abs() < 1e-9 => *end = w[1],
                    _ => {
                        flush(&mut run, &mut out);
                        run = Some((w[0], w[1]));
                    }
                }
            }
        }
        flush(&mut run, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::framing::{Framing, Role};
    use creabim_core::{Beam, Element, ElementKind, Level, Transaction};
    use glam::DVec2;

    fn level(m: &mut Model) -> ElementId {
        let lv = Element::new(
            "Krov",
            ElementKind::Level(Level {
                elevation: 3.0,
                height: 3.0,
            }),
        );
        let id = lv.id;
        Transaction::new("t").insert(lv).apply(m).unwrap();
        id
    }

    fn member(m: &mut Model, l: ElementId, role: Role, section: &str, len: f64) -> ElementId {
        let mut f = Framing::new(role);
        f.section = section.into();
        let mut e = Element::new(
            role.label(),
            ElementKind::Beam(Beam::new(l, DVec2::ZERO, DVec2::new(len, 0.0))),
        );
        framing::apply(&mut e, f);
        let id = e.id;
        Transaction::new("t").insert(e).apply(m).unwrap();
        id
    }

    #[test]
    fn timber_list_counts_pieces_and_volume_like_the_sample() {
        // George's sample: D-01 pomúrnica 2 ks 150/150, 6,420 m: 0,288900 m³
        let mut m = Model::new();
        let l = level(&mut m);
        member(&mut m, l, Role::WallPlate, "150/150", 6.42);
        member(&mut m, l, Role::WallPlate, "150/150", 6.42);
        member(&mut m, l, Role::WallPlate, "150/150", 2.9);
        member(&mut m, l, Role::Rafter, "80/200", 6.14);
        let s = timber_schedule(&m);
        assert_eq!(s.rows[0].cells[0], "D-01");
        assert_eq!(s.rows[0].cells[2], "2");
        assert_eq!(s.rows[0].cells[7], "0,288900");
        assert_eq!(s.rows[1].cells[0], "D-02");
        assert_eq!(s.rows[2].cells[0], "D-03");
    }

    #[test]
    fn steel_list_weighs_each_position() {
        let mut m = Model::new();
        let l = level(&mut m);
        member(&mut m, l, Role::SteelBeam, "IPE 200", 5.0);
        let rod = member(&mut m, l, Role::BracingRod, "Ø 12", 4.0);
        assert!(framing::of(m.get(rod).unwrap()).unwrap().turnbuckle);
        let s = steel_schedule(&m);
        assert_eq!(s.rows[0].cells[0], "O-01");
        // 22,4 kg/m × 5 m
        assert_eq!(s.rows[0].cells[8], "112,00");
        assert!(s.rows[2].cells[2].contains("SSS12125"));
    }

    #[test]
    fn the_lists_end_with_the_paint_area_of_all() {
        let mut m = Model::new();
        let l = level(&mut m);
        // 2 × (0,15 + 0,15) × 6,42 m
        member(&mut m, l, Role::WallPlate, "150/150", 6.42);
        let s = timber_schedule(&m);
        assert_eq!(s.totals[7], "3,85");
        assert_eq!(s.above_totals[0][7], "0,144450");
        // IPE 200: 0,768 m²/m (ArcelorMittal's AL) × 5 m
        let mut m = Model::new();
        let l = level(&mut m);
        member(&mut m, l, Role::SteelBeam, "IPE 200", 5.0);
        let s = steel_schedule(&m);
        assert_eq!(s.totals[8], "3,84");
    }

    #[test]
    fn a_member_over_another_hides_its_lines_and_a_rafter_is_its_axis() {
        use crate::plan::{Pen, PlanItem};
        let mut m = Model::new();
        let l = level(&mut m);
        // a purlin along x and a collar tie across it, on top of it
        let purlin = member(&mut m, l, Role::Purlin, "160/240", 4.0);
        let mut tie = Beam::new(l, DVec2::new(2.0, -1.0), DVec2::new(2.0, 1.0));
        tie.top_offset = -0.2;
        let mut e = Element::new("Collar tie", ElementKind::Beam(tie));
        let mut f = Framing::new(Role::Collar);
        f.section = "50/200".into();
        framing::apply(&mut e, f);
        Transaction::new("t").insert(e).apply(&mut m).unwrap();
        let rafter = member(&mut m, l, Role::Rafter, "80/200", 3.0);
        let lines = |id: ElementId| -> Vec<PlanItem> {
            let e = m.get(id).unwrap();
            plan_items(&m, e, crate::members::member_plan(&m, e, 5.0))
        };
        // the purlin's two long edges each broken where the tie lies over
        let long = lines(purlin)
            .into_iter()
            .filter(|it| matches!(it, PlanItem::Line { a, b, .. } if (a.y - b.y).abs() < 1e-9 && a.y.abs() > 0.01))
            .count();
        assert_eq!(long, 4, "{:?}", lines(purlin));
        let r = lines(rafter);
        assert_eq!(r.len(), 1);
        assert!(
            matches!(&r[0], PlanItem::Line { pen: Pen::Drafted(look), .. } if look.kind == creabim_core::LineType::DashDot)
        );
    }

    #[test]
    fn grouped_by_section_the_lengths_are_summed() {
        let mut m = Model::new();
        let l = level(&mut m);
        member(&mut m, l, Role::WallPlate, "150/150", 6.42);
        member(&mut m, l, Role::WallPlate, "150/150", 2.9);
        member(&mut m, l, Role::Post, "150/150", 2.0);
        member(&mut m, l, Role::Rafter, "80/200", 6.14);
        let s = timber_schedule_by(&m, true);
        assert_eq!(s.rows.len(), 2);
        assert_eq!(s.rows[0].cells[2], "3");
        assert_eq!(s.rows[0].cells[6], "11,320");
        assert!(s.rows[0].cells[1].contains(", "));
        // the same totals as by mark
        assert_eq!(s.totals, timber_schedule(&m).totals);
    }
}
