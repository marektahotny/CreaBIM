//! What STN 92 0201-2 asks of the fire closures in a design:
//!
//! - a door between two fire zones: the least fire resistance of table 5
//!   item 4 (a: under ground, b: above ground, c: the last storey above
//!   ground) or, in a single-storey building, item 13, for the grade of
//!   the zone with the higher requirements (5.2.1); type EW, which EI may
//!   stand in for (5.7.2);
//! - an outer wall of one zone inside the fire-dangerous space (PNP) of
//!   another (5.4.9, 5.4.10, figure 10): its fire resistance from the
//!   outside Ro = ro(R) × d'/d, where R is the resistance the radiating
//!   zone's outer walls must have (table 5 item 2a, 14), d its distance
//!   and d' how far the wall stands in from the space's edge; its
//!   openings self-closing EI closures of 0,5 Ro, at least 30 minutes;
//!   the wall of kind D1, or D1 or D2 where d'/d < 0,7 (5.4.11).
//!
//! Every value comes from the designer's parameter tables (`fire_closure`,
//! `fire_exposed_wall` read off figure 10, `fire_exposed`); with none,
//! nothing is asked.

use crate::firedistance::Pnp;
use crate::firegrade::{self, declared};
use creabim_core::fireparams::{self, ParamTable};
use creabim_core::{ElementId, ElementKind, Model, OpeningKind};

/// Table 5 items 4 and 13: grade, then a closure's least fire resistance
/// ("30", "45/D1") `under`, `above`, `last` storey above ground, and
/// `single` storey buildings.
pub const FIRE_CLOSURE: &str = "fire_closure";
/// Figure 10: R, then Ro at d'/d = 1,0.
pub const FIRE_EXPOSED_WALL: &str = "fire_exposed_wall";
/// Single values: `opening_share`, `opening_min`, `d1_from`, `classes`.
pub const FIRE_EXPOSED: &str = "fire_exposed";

/// What a closure must be.
#[derive(Clone, Debug, PartialEq)]
pub struct ClosureNeed {
    pub minutes: f64,
    /// Of kind D1.
    pub d1: bool,
    /// EI (else EW, which EI may stand in for).
    pub ei: bool,
}

impl ClosureNeed {
    /// As a door's rating is written: "EW 30/D3", "EI 45/D1".
    pub fn rating(&self) -> String {
        format!(
            "{} {}/{}",
            if self.ei { "EI" } else { "EW" },
            self.minutes.round(),
            if self.d1 { "D1" } else { "D3" }
        )
    }

    /// Whether a door rated `rating` meets it.
    pub fn met_by(&self, rating: &str) -> bool {
        let r = rating.trim().to_uppercase();
        let ei = r.starts_with("EI");
        declared(&r).is_some_and(|m| m + 1e-6 >= self.minutes)
            && (!self.ei || ei)
            && (!self.d1 || r.contains("D1"))
    }
}

fn single(t: Option<&ParamTable>, key: &str) -> Option<f64> {
    fireparams::number(t?.cell(key, "value"))
}

/// The grade of the zone `code`; none without its figures.
pub fn zone_grade(model: &Model, zones: &[crate::firerisk::ZoneRisk], code: &str) -> Option<u32> {
    if code.is_empty() || code == "-" {
        return None;
    }
    let z = zones.iter().find(|z| z.code == code)?;
    // a grade given by rule or by hand needs no pv
    let pv = if z.no_risk {
        0.0
    } else if z.missing.is_empty() {
        z.pv
    } else {
        f64::NAN
    };
    firegrade::grade(model, pv)
}

/// Which storey row of table 5 a storey falls in.
fn storey_row(model: &Model, level: ElementId) -> Option<&'static str> {
    let fh = crate::firezones::fire_height(model)?;
    if fh.above <= 1 && fh.below == 0 {
        return Some("single");
    }
    let elev = |id: ElementId| model.level(id).map(|l| l.elevation);
    let (here, first) = (elev(level)?, elev(fh.first)?);
    Some(if here < first - 1e-6 {
        "under"
    } else if level == fh.last {
        "last"
    } else {
        "above"
    })
}

fn cell<'a>(t: &'a ParamTable, grade: u32, column: &str) -> Option<&'a str> {
    let i = t.columns.iter().position(|c| c == column)?;
    t.rows
        .iter()
        .find(|r| {
            r.first()
                .and_then(|s| fireparams::number(s))
                .is_some_and(|g| g.round() as u32 == grade)
        })
        .and_then(|r| r.get(i))
        .map(|s| s.as_str())
}

/// Table 5 items 4/13 for a door on `level` between zones of grades
/// `grades` (5.2.1: the higher one counts).
pub fn border_need(
    model: &Model,
    level: ElementId,
    grades: [Option<u32>; 2],
) -> Option<ClosureNeed> {
    let grade = grades.into_iter().flatten().max()?;
    let t = crate::firerisk::table(model, FIRE_CLOSURE)?;
    let c = cell(t, grade, storey_row(model, level)?)?;
    Some(ClosureNeed {
        minutes: fireparams::number(c.split('/').next()?)?,
        d1: c.to_uppercase().contains("D1"),
        ei: false,
    })
}

/// Figure 10: Ro for a radiating zone whose outer walls must resist `r`
/// minutes, at `ratio` = d'/d; linear between the table's lines, held at
/// its ends.
pub fn ro(t: &ParamTable, r: f64, ratio: f64) -> Option<f64> {
    let mut pts: Vec<(f64, f64)> = t
        .rows
        .iter()
        .filter_map(|row| {
            Some((
                fireparams::number(row.first()?)?,
                fireparams::number(row.get(1)?)?,
            ))
        })
        .collect();
    pts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (first, last) = (*pts.first()?, *pts.last()?);
    let at1 = if r <= first.0 {
        first.1 * r / first.0.max(1e-9)
    } else if r >= last.0 {
        last.1
    } else {
        let w = pts.windows(2).find(|w| r <= w[1].0)?;
        let k = (r - w[0].0) / (w[1].0 - w[0].0);
        w[0].1 + k * (w[1].1 - w[0].1)
    };
    Some(at1 * ratio.clamp(0.0, 1.0))
}

/// Rounded up to the next class of fire resistance.
fn class_up(classes: &[f64], minutes: f64) -> f64 {
    classes
        .iter()
        .copied()
        .find(|c| *c + 1e-6 >= minutes)
        .unwrap_or(minutes.ceil())
}

/// An opening in an exposed wall and what its closure must be.
#[derive(Clone, Debug, PartialEq)]
pub struct ExposedOpening {
    pub id: ElementId,
    pub door: bool,
    pub need: ClosureNeed,
    /// A door's fire rating as entered.
    pub rating: Option<String>,
}

/// An outer wall standing in another zone's fire-dangerous space.
#[derive(Clone, Debug, PartialEq)]
pub struct Exposed {
    pub wall: ElementId,
    pub level: ElementId,
    /// Its own zone and the radiating one.
    pub zone: String,
    pub from: String,
    pub d: f64,
    /// d'/d.
    pub ratio: f64,
    /// The radiating zone's outer wall resistance R and the Ro asked of
    /// this wall from the outside, minutes.
    pub r: f64,
    pub ro: f64,
    /// What its assembly states.
    pub declared: Option<f64>,
    /// D1 only (else D1 or D2).
    pub d1: bool,
    pub openings: Vec<ExposedOpening>,
}

impl Exposed {
    pub fn met(&self) -> Option<bool> {
        self.declared.map(|has| has + 1e-6 >= self.ro.ceil())
    }
}

/// The outer walls inside other zones' fire-dangerous spaces, worked out
/// from the spaces `pnps` (of every storey).
pub fn exposed_in(model: &Model, pnps: &[Pnp]) -> Vec<Exposed> {
    let (Some(fig), Some(t)) = (
        crate::firerisk::table(model, FIRE_EXPOSED_WALL),
        crate::firerisk::table(model, FIRE_EXPOSED),
    ) else {
        return vec![];
    };
    let share = single(Some(t), "opening_share").unwrap_or(0.5);
    let least = single(Some(t), "opening_min").unwrap_or(30.0);
    let d1_from = single(Some(t), "d1_from").unwrap_or(0.7);
    let classes: Vec<f64> = t
        .cell("classes", "value")
        .split([';', ' '])
        .filter_map(fireparams::number)
        .collect();
    let zones = crate::firerisk::zones(model);
    let marks_zone = |level: ElementId, p: glam::DVec2| crate::firezones::zone_at(model, level, p);
    let mut out: Vec<Exposed> = vec![];
    for w in crate::firezones::outer_walls(model) {
        let wall = w.wall;
        let along = wall.end - wall.start;
        let len = along.length();
        if len < 1e-6 {
            continue;
        }
        let t_dir = along / len;
        let face = |x: f64| wall.start + t_dir * x + w.outward * (wall.thickness / 2.0);
        let zone = marks_zone(
            wall.level,
            wall.start + along / 2.0 - w.outward * (wall.thickness / 2.0 + 0.15),
        );
        for p in pnps.iter().filter(|p| p.level == wall.level) {
            if p.zone.is_empty() || p.zone == zone || p.d <= 1e-3 {
                continue;
            }
            // the point of the wall's face deepest in the space
            let n = (len / 0.05).ceil().max(1.0) as usize;
            let inside = |x: f64| {
                let q = face(x);
                let s = (q - p.at).dot(p.outward);
                (s > 0.05 && creabim_core::polygon::contains(&p.outline, q)).then_some(s)
            };
            let Some(s) = (0..=n)
                .filter_map(|i| inside(len * i as f64 / n as f64))
                .reduce(f64::min)
            else {
                continue;
            };
            let ratio = ((p.d - s) / p.d).clamp(0.0, 1.0);
            let Some(grade) = zone_grade(model, &zones, &p.zone) else {
                continue;
            };
            let Some(r) = firegrade::outer_wall_minutes(model, grade) else {
                continue;
            };
            let Some(ro_v) = ro(fig, r, ratio) else {
                continue;
            };
            let need = ClosureNeed {
                minutes: class_up(&classes, (share * ro_v).max(least)),
                d1: ratio >= d1_from,
                ei: true,
            };
            let openings: Vec<ExposedOpening> = model
                .iter()
                .filter_map(|e| match &e.kind {
                    ElementKind::Opening(o) if o.host == w.id => {
                        let mid = o.offset + o.width / 2.0;
                        inside(mid).map(|_| ExposedOpening {
                            id: e.id,
                            door: o.kind == OpeningKind::Door,
                            need: need.clone(),
                            rating: o
                                .door
                                .as_ref()
                                .and_then(|d| d.fire.as_ref())
                                .map(|f| f.rating.clone()),
                        })
                    }
                    _ => None,
                })
                .collect();
            let item = Exposed {
                wall: w.id,
                level: wall.level,
                zone: zone.clone(),
                from: p.zone.clone(),
                d: p.d,
                ratio,
                r,
                ro: ro_v,
                declared: firegrade::wall_declared(model, wall),
                d1: ratio >= d1_from,
                openings,
            };
            // the most demanding space a wall stands in
            match out.iter_mut().find(|x| x.wall == w.id) {
                Some(x) if x.ro >= item.ro => {}
                Some(x) => *x = item,
                None => out.push(item),
            }
        }
    }
    out
}

/// The outer walls a fire zone asks a fire resistance of (table 5 items
/// 2/14) whose assembly states none: each with its way out.
pub fn unstated_walls(model: &Model) -> Vec<(ElementId, &creabim_core::Wall, glam::DVec2)> {
    let zones = crate::firerisk::zones(model);
    crate::firezones::outer_walls(model)
        .into_iter()
        .filter(|w| {
            let wall = w.wall;
            let mid = (wall.start + wall.end) / 2.0 - w.outward * (wall.thickness / 2.0 + 0.15);
            let zone = crate::firezones::zone_at(model, wall.level, mid);
            zone_grade(model, &zones, &zone)
                .and_then(|g| firegrade::wall_required(model, g, wall))
                .is_some()
                && firegrade::wall_declared(model, wall).is_none()
        })
        .map(|w| (w.id, w.wall, w.outward))
        .collect()
}

/// The same, over the spaces of the whole model.
pub fn exposed(model: &Model) -> Vec<Exposed> {
    exposed_in(model, &crate::firedistance::spaces(model, None, None))
}

/// The doors whose fire rating falls short of what the norm asks (or is
/// missing), each with the rating proposed: on the zones' borders and in
/// exposed walls.
pub fn door_proposals(model: &Model) -> Vec<(ElementId, String)> {
    let mut out: Vec<(ElementId, ClosureNeed, Option<String>)> = vec![];
    let zones = crate::firerisk::zones(model);
    for d in crate::firezones::border_doors(model) {
        let Some(ElementKind::Opening(o)) = model.get(d.door).map(|e| &e.kind) else {
            continue;
        };
        let Some(ElementKind::Wall(w)) = model.get(o.host).map(|e| &e.kind) else {
            continue;
        };
        let g = [
            zone_grade(model, &zones, &d.zones[0]),
            zone_grade(model, &zones, &d.zones[1]),
        ];
        if let Some(need) = border_need(model, w.level, g) {
            let rating = o
                .door
                .as_ref()
                .and_then(|x| x.fire.as_ref())
                .map(|f| f.rating.clone());
            out.push((d.door, need, rating));
        }
    }
    for x in exposed(model) {
        for o in x.openings.into_iter().filter(|o| o.door) {
            match out.iter_mut().find(|e| e.0 == o.id) {
                // both apply: the stricter of each
                Some(e) => {
                    e.1 = ClosureNeed {
                        minutes: e.1.minutes.max(o.need.minutes),
                        d1: e.1.d1 || o.need.d1,
                        ei: e.1.ei || o.need.ei,
                    }
                }
                None => out.push((o.id, o.need, o.rating)),
            }
        }
    }
    out.into_iter()
        .filter(|(_, need, rating)| !rating.as_deref().is_some_and(|r| need.met_by(r)))
        .map(|(id, need, _)| (id, need.rating()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up figure in the norm's layout (not its values).
    const FIG: &str = "#table f\n#columns R\tro\n30\t40\n60\t100\n";

    #[test]
    fn figure_10_is_read_along_its_lines() {
        let t = &fireparams::parse(FIG)[0];
        assert_eq!(ro(t, 60.0, 0.5), Some(50.0));
        assert_eq!(ro(t, 45.0, 1.0), Some(70.0));
        assert_eq!(ro(t, 15.0, 1.0), Some(20.0));
        assert_eq!(ro(t, 90.0, 2.0), Some(100.0));
    }

    /// Doors between two zones get table 5 item 4/13's rating proposed;
    /// one already rated well enough is left.
    #[test]
    fn doors_between_zones_get_a_rating_proposed() {
        use creabim_core::{Element, Transaction};
        let mut m = creabim_core::starter::starter_house();
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
        assert!(door_proposals(&m).is_empty());
        let tables = "#table fire_grade\n#columns key\tvalue\tnote\nrule\t1\t\n\
            #table fire_closure\n#columns grade\tunder\tabove\tlast\tsingle\n\
            1\t30/D1\t30\t20\t25/D1\n";
        let mut info = m.project_info().map(|p| p.1.clone()).unwrap_or_default();
        fireparams::merge(&mut info.fire_params, fireparams::parse(tables));
        creabim_core::set_project(&m, info).apply(&mut m).unwrap();
        let p = door_proposals(&m);
        assert_eq!(p.len(), crate::firezones::border_doors(&m).len());
        assert!(
            p.iter()
                .all(|(_, r)| ["EW 30/D3", "EW 20/D3", "EW 25/D1"].contains(&r.as_str())),
            "{p:?}"
        );
    }

    /// Outer walls asked for a fire resistance their assembly does not
    /// state are listed (for the warning); none without the tables.
    #[test]
    fn outer_walls_stating_no_fire_resistance_are_listed() {
        use creabim_core::{Element, Transaction};
        let mut m = creabim_core::starter::starter_house();
        let rooms: Vec<Element> = m
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Room(_)))
            .cloned()
            .collect();
        let mut tx = Transaction::new("zones");
        for e in &rooms {
            let mut n = e.clone();
            if let ElementKind::Room(r) = &mut n.kind {
                r.fire_zone = "N1.01".into();
            }
            tx = tx.replace(e.clone(), n);
        }
        tx.apply(&mut m).unwrap();
        assert!(unstated_walls(&m).is_empty());
        let tables = "#table fire_grade\n#columns key\tvalue\tnote\nrule\t1\t\n\
            #table fire_outer_wall\n#columns grade\tabove\tlast\tnonbearing\tsingle\n\
            1\t30\t15\t15\t15\n";
        let mut info = m.project_info().map(|p| p.1.clone()).unwrap_or_default();
        fireparams::merge(&mut info.fire_params, fireparams::parse(tables));
        creabim_core::set_project(&m, info).apply(&mut m).unwrap();
        assert!(!unstated_walls(&m).is_empty());
    }

    #[test]
    fn a_rating_meets_a_need() {
        let n = ClosureNeed {
            minutes: 30.0,
            d1: false,
            ei: false,
        };
        assert!(n.met_by("EW 30/D3"));
        assert!(n.met_by("EI 45/D1"));
        assert!(!n.met_by("EW 15/D3"));
        let n = ClosureNeed {
            minutes: 30.0,
            d1: true,
            ei: true,
        };
        assert!(!n.met_by("EW 30/D1"));
        assert!(!n.met_by("EI 30/D3"));
        assert!(n.met_by("EI 30/D1"));
        assert_eq!(n.rating(), "EI 30/D1");
    }
}
