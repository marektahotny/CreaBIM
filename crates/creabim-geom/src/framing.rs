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
    let rows: Vec<Row> = positions
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
        .collect();
    let volume: f64 = positions.iter().map(Position::volume).sum();
    Schedule {
        category: creabim_core::ScheduleCategory::TimberList,
        columns,
        listed: rows.len(),
        rows,
        totals: vec![
            tr("Timber in all"),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            comma(volume, 6),
        ],
        above_totals: vec![],
    }
}

/// The list of steel members (výpis oceľových prvkov): mark, member,
/// section, grade, pieces, length of one and of all, kg/m and mass, and
/// the total mass of the structure, as the structural engineer's "Výpis
/// materiálu"; a bracing rod's turnbuckles under it.
pub fn steel_schedule(model: &Model) -> Schedule {
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
    for p in &positions {
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
                rows.push(Row {
                    id: p.members[0],
                    cells: vec![
                        String::new(),
                        format!("{} M{}", tr("Turnbuckle"), t.thread),
                        format!("Rothoblaas {}", t.code),
                        "S355".into(),
                        p.turnbuckles.to_string(),
                        String::new(),
                        String::new(),
                        String::new(),
                        String::new(),
                    ],
                    group: false,
                });
            }
        }
    }
    Schedule {
        category: creabim_core::ScheduleCategory::SteelList,
        columns,
        listed: positions.len(),
        rows,
        totals: vec![
            tr("Total mass of the structure"),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            comma(total, 2),
        ],
        above_totals: vec![],
    }
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
}
