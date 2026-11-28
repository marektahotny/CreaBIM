//! The fire-dangerous space (požiarne nebezpečný priestor, PNP) in front
//! of the fire-open areas of a fire zone and its separation distance d,
//! as STN 92 0201-4 sets them, drawn in a plan as a dashed red outline
//! with "PNP d = x,xx m".
//!
//! - The fire-open area is that of a fire zone in one plane of the
//!   building's face, over all the zone's storeys (5.2.1): its windows
//!   and the doors without a fire rating. Sp is the smallest rectangle
//!   holding them all, lu × hu, and po the share of it they make (5.3.1).
//! - d is read off the norm's table 3 by hu, lu, po and the zone's
//!   design fire load pv, or for a zone that is a dwelling off table 6
//!   (or 7 for lodging) by lu and po, times its factor for two or three
//!   storeys; between the table's values linearly (the tables' notes).
//! - The space reaches d straight out of the plane and to the sides is
//!   bounded by an arc of radius d about each edge of the area and the
//!   plane from the edge at 160° to the area (2.2.1, 2.2.2, figure 1).
//! - A zone without fire risk has none (2.6.4), nor a single fully
//!   open area under 0,5 m long and high (2.6.4 f), Z3).
//! - With po under 40 % each opening gets its own d off table 4; those
//!   nearer than 0,6 × (d1 + d2) are taken together by 5.3.1 (3.2.4 as
//!   Z3 has it, figure 7).
//! - Under the tables' lowest values d runs linearly to nought (the
//!   note to table 3 as Z3 has it).
//! - Planes of the face set back less than d behind the outermost are
//!   projected into it, those further back reckoned on their own
//!   (chapter 3, 3.2.2).
//! - Partly fire-open areas (fire doors of the ratings the designer
//!   lists) count × k10 and a fire-open roof × k11 (4.2.2, table 2, the
//!   roof seen square onto the face by 4.3); equation (3) where (2)
//!   exceeds Sp. A combustible or mixed structural system adds to pv
//!   (4.4.1 as Z3 has it).
//!
//! Every value of the norm is read from the designer's parameter tables
//! (`FIRE_DISTANCE` and the tables it names); with none, a side's
//! separation distance given in the project information is drawn.

use crate::annotation::{Annotation, Shape, TextLook};
use crate::plan::PlanItem;
use creabim_core::fireparams::{self, ParamTable};
use creabim_core::words::tr;
use creabim_core::{ElementId, ElementKind, Model, OpeningKind};
use glam::DVec2;

/// The single values of the separation distances by key (`i_crit`,
/// `side_angle`, `two_storey`, `three_storey`, `pv_default`, `table`
/// (3, 6 or 7: which table gives d), `d3` (1: load-bearing and fire
/// separating structures of type D3)).
pub const FIRE_DISTANCE: &str = "fire_distance";
/// Table 3: d by hu, lu, po and pv.
pub const FIRE_DISTANCE_D: &str = "fire_distance_d";
/// Table 6: d of a zone that is a dwelling, by po and lu, "a/b" with b
/// for D3 structures.
pub const FIRE_DISTANCE_DWELLING: &str = "fire_distance_dwelling";
/// Table 7: the same for lodging.
pub const FIRE_DISTANCE_LODGING: &str = "fire_distance_lodging";
/// Table 4 (as Z3 has it): d of a single opening by its length and
/// height, a column for each pv.
pub const FIRE_DISTANCE_SINGLE: &str = "fire_distance_single";
/// Table 2: k10 and k11 (and q) by the fire's equivalent time t.
pub const FIRE_DISTANCE_K: &str = "fire_distance_k";

/// cos 45°: a wall turned less than this from a face's main way is
/// projected into its reference plane (George's choice, 2026-11-19).
const MAIN_WAY_COS: f64 = std::f64::consts::FRAC_1_SQRT_2;

/// The colour the space is drawn in.
pub const PNP_RED: [f32; 3] = crate::firezones::DANGER;

/// A fire zone's fire-dangerous space in front of one plane of its face.
#[derive(Clone, Debug, PartialEq)]
pub struct Pnp {
    /// The first of its openings on the storey drawn (or the one asked
    /// for), which the drawing's items carry.
    pub opening: ElementId,
    /// The storey it is drawn on and the openings (and roofs) it is in
    /// front of.
    pub level: ElementId,
    pub members: Vec<ElementId>,
    /// Its outline in the plan, from one edge of the fire-open area (in
    /// the wall's outer face) round to the other.
    pub outline: Vec<DVec2>,
    /// The separation distance, m.
    pub d: f64,
    /// The middle of the fire-open area in the face and the way out.
    pub at: DVec2,
    pub outward: DVec2,
    /// The storeys whose openings are part of the area, by name, other
    /// than the one drawn.
    pub shared: Vec<String>,
    /// The fire-open area: lu × hu, m, and its share po, %.
    pub lu: f64,
    pub hu: f64,
    pub po: f64,
    /// The design fire load the distance is read for, kg/m²; none where
    /// a table without it or a given distance is used.
    pub pv: Option<f64>,
    /// The fire zone whose fire-open area it is ("" none).
    pub zone: String,
}

/// An opening fire can pass, as it sits in an outer wall's face.
struct Source {
    id: ElementId,
    level: ElementId,
    host: ElementId,
    /// The edges along the face at the wall's outer side, and the
    /// heights of the bottom and the top of the opening.
    a: DVec2,
    b: DVec2,
    z0: f64,
    z1: f64,
    /// The room behind it.
    inside: DVec2,
    /// Closed by a construction that makes it partly fire-open (4.1.3 c)).
    partial: bool,
}

/// The openings of the outer walls: windows and doors, a fire door
/// whose rating begins with one of `partial` as a partly fire-open
/// area, other fire doors not at all.
fn sources(model: &Model, partial: &[String]) -> Vec<Source> {
    let walls = crate::firezones::outer_walls(model);
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Opening(o) = &e.kind else {
            continue;
        };
        let rating = (o.kind == OpeningKind::Door)
            .then(|| o.door.as_ref().and_then(|d| d.fire.as_ref()))
            .flatten()
            .map(|f| f.rating.trim().to_uppercase());
        let is_partial = rating.as_ref().is_some_and(|r| {
            partial
                .iter()
                .any(|x| !x.is_empty() && r.starts_with(x.as_str()))
        });
        // a fire door holds the fire in
        if rating.is_some() && !is_partial {
            continue;
        }
        let Some(w) = walls.iter().find(|w| w.id == o.host) else {
            continue;
        };
        let wall = w.wall;
        let along = (wall.end - wall.start).normalize_or_zero();
        let face = w.outward * (wall.thickness / 2.0);
        let a = wall.start + along * o.offset + face;
        let b = a + along * o.width;
        let base = model.level(wall.level).map_or(0.0, |l| l.elevation);
        let mid = (a + b) / 2.0;
        out.push(Source {
            id: e.id,
            level: wall.level,
            host: o.host,
            a,
            b,
            z0: base + o.sill,
            z1: base + o.sill + o.height,
            inside: mid - w.outward * (wall.thickness + 0.15),
            partial: is_partial,
        });
    }
    out
}

/// A roof plane over a storey, as fire-open area of the face below its
/// eave (4.1.4, 4.3).
struct RoofPiece {
    id: ElementId,
    level: ElementId,
    pitch: f64,
    /// Its outline in plan and the height of each point.
    points: Vec<DVec2>,
    z: Vec<f64>,
    /// Which way it falls in plan (none for a flat one).
    down: DVec2,
}

fn roof_pieces(model: &Model) -> Vec<RoofPiece> {
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Roof(r) = &e.kind else {
            continue;
        };
        let base = r.base(model);
        for f in r.faces() {
            if f.outline.len() < 3 {
                continue;
            }
            out.push(RoofPiece {
                id: e.id,
                level: r.level,
                pitch: f.g.length().atan().to_degrees(),
                z: f.outline.iter().map(|p| base + f.z(*p)).collect(),
                points: f.outline.clone(),
                down: -f.g.normalize_or_zero(),
            });
        }
    }
    out
}

fn value(t: Option<&ParamTable>, key: &str) -> Option<f64> {
    fireparams::number(t?.cell(key, "value"))
}

/// The design fire load pv of the room behind each point, by its fire
/// zone; none where the zone has no figure or no fire risk.
struct Loads {
    zones: Vec<crate::firerisk::ZoneRisk>,
    default: Option<f64>,
}

impl Loads {
    fn new(model: &Model, table: Option<&ParamTable>) -> Self {
        Self {
            zones: crate::firerisk::zones(model),
            default: value(table, "pv_default").filter(|v| *v > 0.0),
        }
    }

    /// The load behind `s`; `Err` for a zone without fire risk, which
    /// has no fire-dangerous space.
    fn of(&self, model: &Model, s: &Source) -> Result<Option<f64>, ()> {
        let code = model
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::Room(r) if r.level == s.level => {
                    let sh = creabim_core::room::shape(model, e.id)?;
                    creabim_core::polygon::contains(&sh.outline, s.inside)
                        .then(|| r.fire_zone.trim().to_string())
                }
                _ => None,
            })
            .unwrap_or_default();
        match self
            .zones
            .iter()
            .find(|z| !code.is_empty() && z.code == code)
        {
            Some(z) if z.no_risk => Err(()),
            Some(z) if z.pv > 0.0 && z.missing.is_empty() => Ok(Some(z.pv)),
            _ => Ok(self.default),
        }
    }
}

/// A stretch of a building's face on one storey: outer walls of one fire
/// zone, facing one way, in one plane, end to end.
struct Panel {
    level: ElementId,
    walls: Vec<ElementId>,
    /// A point of the face's line and its way along; the stretch runs
    /// `lo`..`hi` along it.
    origin: DVec2,
    t: DVec2,
    lo: f64,
    hi: f64,
    outward: DVec2,
    facing: usize,
    zone: String,
}

impl Panel {
    /// Whether the opening `o` is in this stretch: in one of its walls
    /// and between its ends (a wall behind rooms of two zones gives a
    /// stretch to each).
    fn holds(&self, o: &Source) -> bool {
        let m = ((o.a + o.b) / 2.0 - self.origin).dot(self.t);
        self.walls.contains(&o.host) && m >= self.lo - 0.05 && m <= self.hi + 0.05
    }
}

/// A wall's face split where the fire zone of the rooms behind it
/// changes: each piece's ends and zone.
fn zone_runs(
    rooms: &[(ElementId, Vec<DVec2>, String)],
    level: ElementId,
    a: DVec2,
    b: DVec2,
    inward: DVec2,
) -> Vec<(DVec2, DVec2, String)> {
    let zone = |p: DVec2| {
        rooms
            .iter()
            .find(|(l, outline, _)| *l == level && creabim_core::polygon::contains(outline, p))
            .map(|r| r.2.clone())
            .unwrap_or_default()
    };
    let n = ((a.distance(b) / 0.1).ceil() as usize).clamp(1, 2000);
    let mut out: Vec<(DVec2, DVec2, String)> = vec![];
    for k in 0..n {
        let (s0, s1) = (k as f64 / n as f64, (k + 1) as f64 / n as f64);
        let (p, q) = (a.lerp(b, s0), a.lerp(b, s1));
        let z = zone((p + q) / 2.0 + inward);
        match out.last_mut() {
            Some(last) if last.2 == z => last.1 = q,
            _ => out.push((p, q, z)),
        }
    }
    out
}

fn panels(model: &Model) -> Vec<Panel> {
    let mut out: Vec<Panel> = vec![];
    let rooms: Vec<(ElementId, Vec<DVec2>, String)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Room(r) => {
                let sh = creabim_core::room::shape(model, e.id)?;
                Some((r.level, sh.outline, r.fire_zone.trim().to_string()))
            }
            _ => None,
        })
        .collect();
    for w in crate::firezones::outer_walls(model) {
        let wall = w.wall;
        let face = w.outward * (wall.thickness / 2.0);
        let inward = -w.outward * (wall.thickness + 0.15);
        for (a, b, zone) in zone_runs(
            &rooms,
            wall.level,
            wall.start + face,
            wall.end + face,
            inward,
        ) {
            let found = out.iter_mut().find(|p| {
                p.level == wall.level
                    && p.zone == zone
                    && p.outward.dot(w.outward) > 0.99
                    && (a - p.origin).dot(p.outward).abs() < 0.05
                    && {
                        let (x, y) = ((a - p.origin).dot(p.t), (b - p.origin).dot(p.t));
                        x.min(y) <= p.hi + 0.05 && x.max(y) >= p.lo - 0.05
                    }
            });
            match found {
                Some(p) => {
                    let (x, y) = ((a - p.origin).dot(p.t), (b - p.origin).dot(p.t));
                    p.lo = p.lo.min(x.min(y));
                    p.hi = p.hi.max(x.max(y));
                    p.walls.push(w.id);
                }
                None => {
                    let t = w.outward.perp();
                    let (x, y) = (0.0f64, (b - a).dot(t));
                    out.push(Panel {
                        level: wall.level,
                        walls: vec![w.id],
                        origin: a,
                        t,
                        lo: x.min(y),
                        hi: x.max(y),
                        outward: w.outward,
                        facing: w.facing,
                        zone,
                    });
                }
            }
        }
    }
    // walls met out of order may join stretches found apart
    let mut i = 0;
    while i < out.len() {
        let j = (i + 1..out.len()).find(|&j| {
            let (p, q) = (&out[i], &out[j]);
            p.level == q.level
                && p.zone == q.zone
                && p.outward.dot(q.outward) > 0.99
                && (q.origin - p.origin).dot(p.outward).abs() < 0.05
                && {
                    let off = (q.origin - p.origin).dot(p.t);
                    q.lo + off <= p.hi + 0.05 && q.hi + off >= p.lo - 0.05
                }
        });
        match j {
            Some(j) => {
                let q = out.remove(j);
                let off = (q.origin - out[i].origin).dot(out[i].t);
                let p = &mut out[i];
                p.lo = p.lo.min(q.lo + off);
                p.hi = p.hi.max(q.hi + off);
                p.walls.extend(q.walls);
            }
            None => i += 1,
        }
    }
    out
}

/// The design load pvm of a concentrated fire load in the room at `p`,
/// where the designer gives one (4.4.5).
fn concentrated(model: &Model, level: ElementId, p: DVec2) -> Option<f64> {
    model.iter().find_map(|e| match &e.kind {
        ElementKind::Room(r) if r.level == level => {
            let sh = creabim_core::room::shape(model, e.id)?;
            creabim_core::polygon::contains(&sh.outline, p)
                .then_some(r.fire_pvm)
                .flatten()
        }
        _ => None,
    })
}

/// The fire-dangerous spaces of the fire zones' planes on `level` (every
/// level with none), or the one of the plane the opening `only` is in.
pub fn spaces(model: &Model, level: Option<ElementId>, only: Option<ElementId>) -> Vec<Pnp> {
    let table = crate::firerisk::table(model, FIRE_DISTANCE);
    let reader = Reader::new(model, table);
    let all = sources(model, &reader.partial);
    if all.is_empty() {
        return vec![];
    }
    let panels = panels(model);
    let roofs = roof_pieces(model);
    let loads = Loads::new(model, table);
    let ctx = Ctx {
        model,
        reader: &reader,
        loads: &loads,
        all: &all,
        roofs: &roofs,
        panels: &panels,
    };
    let mut out = vec![];
    // a fire zone's face in each direction, in its reference planes
    // (chapter 3): the outermost plane first, with the planes behind it
    // nearer than its distance d projected into it (3.1.1, 3.2.2); those
    // further back are reckoned on their own
    // the main way each stretch faces: of the ways within 45° of its own
    // (its own among them), the one the zone's face runs longest in; a
    // wall turned less than that is projected into the main way's
    // reference plane (the note to 3.1.2, the designer's choice)
    let len = |j: usize| panels[j].hi - panels[j].lo;
    let main: Vec<DVec2> = (0..panels.len())
        .map(|j| {
            let q = &panels[j];
            let mut best = (q.outward, -1.0);
            for k in 0..panels.len() {
                let r = &panels[k];
                if r.zone != q.zone || r.outward.dot(q.outward) <= MAIN_WAY_COS {
                    continue;
                }
                let run: f64 = (0..panels.len())
                    .filter(|&m| {
                        panels[m].zone == q.zone && panels[m].outward.dot(r.outward) > 0.99
                    })
                    .map(len)
                    .sum();
                if run > best.1 + 1e-9 {
                    best = (r.outward, run);
                }
            }
            best.0
        })
        .collect();
    let mut used = vec![false; panels.len()];
    for i in 0..panels.len() {
        if used[i] || main[i].dot(panels[i].outward) < 0.99 {
            continue;
        }
        let p0 = &panels[i];
        let mut family: Vec<usize> = (0..panels.len())
            .filter(|&j| !used[j] && panels[j].zone == p0.zone && main[j].dot(p0.outward) > 0.99)
            .collect();
        for &j in &family {
            used[j] = true;
        }
        // how far out a stretch stands: its outermost end
        let off = |j: usize| {
            let q = &panels[j];
            let (a, b) = (q.origin + q.t * q.lo, q.origin + q.t * q.hi);
            (a - p0.origin)
                .dot(p0.outward)
                .max((b - p0.origin).dot(p0.outward))
        };
        // fully open area of each stretch, and the zone's all told
        let open_of = |j: usize| -> f64 {
            all.iter()
                .filter(|o| !o.partial && panels[j].holds(o))
                .map(|o| o.a.distance(o.b) * (o.z1 - o.z0))
                .sum()
        };
        let zone_open: f64 = (0..panels.len())
            .filter(|&j| panels[j].zone == p0.zone)
            .map(open_of)
            .sum();
        // the stretches set back no more than d behind `top` (or in front
        // of it), and their spaces
        let solve = |family: &[usize], top: f64| {
            let mut members: Vec<usize> = family
                .iter()
                .copied()
                .filter(|&j| top - off(j) < 0.05)
                .collect();
            let mut found = ctx.plane(p0, top, &members);
            loop {
                let d = found.iter().map(|f| f.d).fold(0.0, f64::max);
                let more: Vec<usize> = family
                    .iter()
                    .copied()
                    .filter(|&j| !members.contains(&j) && top - off(j) <= d + 1e-9)
                    .collect();
                if more.is_empty() {
                    break;
                }
                members.extend(more);
                found = ctx.plane(p0, top, &members);
            }
            (members, found)
        };
        while !family.is_empty() {
            // 3.2.1: a projection with no fire-open area (a)) is no
            // reference plane; one with at most a share of the zone's
            // fire-open area standing out no more than d/10 (and a most)
            // in front of the next plane (b)) is projected into that one
            let mut planes: Vec<f64> = family
                .iter()
                .filter(|&&j| open_of(j) > 0.0)
                .map(|&j| off(j))
                .collect();
            planes.sort_by(|a, b| b.total_cmp(a));
            planes.dedup_by(|a, b| (*a - *b).abs() < 0.05);
            let Some(&first) = planes.first() else {
                break;
            };
            let mut top = first;
            let mut solved = solve(&family, top);
            if let (Some(share), Some(part), Some(most)) = (
                reader.protrusion_share,
                reader.protrusion_depth,
                reader.protrusion_max,
            ) {
                for &next in &planes[1..] {
                    let front: f64 = family
                        .iter()
                        .filter(|&&j| off(j) > next + 0.05)
                        .map(|&j| open_of(j))
                        .sum();
                    if zone_open <= 0.0 || front / zone_open * 100.0 > share + 1e-9 {
                        break;
                    }
                    let behind = solve(&family, next);
                    let d = behind.1.iter().map(|f| f.d).fold(0.0, f64::max);
                    if top - next > (d * part).min(most) + 1e-9 {
                        break;
                    }
                    top = next;
                    solved = behind;
                }
            }
            let (members, found) = solved;
            family.retain(|j| !members.contains(j));
            out.extend(found);
        }
    }
    out.retain(|p| level.is_none_or(|l| p.level == l));
    if let Some(id) = only {
        out.retain(|p| p.members.contains(&id));
        for p in &mut out {
            p.opening = id;
        }
    }
    out
}

/// One fire-open area of a reference plane: an opening or a roof's
/// share of it, as a rectangle in the plane (along, from..to,
/// bottom..top) and how open it is.
struct Part {
    id: ElementId,
    level: ElementId,
    rect: [f64; 4],
    kind: PartKind,
    /// Its fire-open area, m²: the rectangle's, or for a partly open wall
    /// its face less its openings.
    area: f64,
    /// The design load of a concentrated fire load behind it (4.4.5).
    pvm: Option<f64>,
}

#[derive(Clone, Copy, PartialEq)]
enum PartKind {
    /// Fully fire-open (4.1.2).
    Full,
    /// Partly fire-open (4.1.3), counted × k10 (4.2.1 b)).
    Partial,
    /// The roof (4.1.4), counted × k11.
    Roof,
}

struct Ctx<'a> {
    model: &'a Model,
    reader: &'a Reader<'a>,
    loads: &'a Loads,
    all: &'a [Source],
    roofs: &'a [RoofPiece],
    panels: &'a [Panel],
}

impl Ctx<'_> {
    /// The spaces of one reference plane, `top` out from `p0`'s line,
    /// holding the stretches `members` of the face on every storey.
    fn plane(&self, p0: &Panel, top: f64, members: &[usize]) -> Vec<Pnp> {
        let model = self.model;
        let reader = self.reader;
        let along = |v: DVec2| (v - p0.origin).dot(p0.t);
        let sources: Vec<&Source> = self
            .all
            .iter()
            .filter(|o| members.iter().any(|&j| self.panels[j].holds(o)))
            .collect();
        let mut parts: Vec<Part> = sources
            .iter()
            .map(|o| {
                let (x, y) = (along(o.a), along(o.b));
                Part {
                    id: o.id,
                    level: o.level,
                    rect: [x.min(y), x.max(y), o.z0, o.z1],
                    kind: if o.partial {
                        PartKind::Partial
                    } else {
                        PartKind::Full
                    },
                    area: (x - y).abs() * (o.z1 - o.z0),
                    pvm: (!o.partial)
                        .then(|| concentrated(model, o.level, o.inside))
                        .flatten(),
                }
            })
            .collect();
        // the zone's grade by STN 92 0201-2 (what its table 5 asks of
        // the outer walls and the roof)
        let zone_pv = self
            .loads
            .zones
            .iter()
            .find(|z| !p0.zone.is_empty() && z.code == p0.zone)
            .map(|z| z.pv)
            .or(self.loads.default);
        let grade = zone_pv.and_then(|pv| crate::firegrade::grade(model, pv));
        // 4.1.2 and 4.1.3 (Z3), STN 92 0201-2 5.4.4 and table 5 note 3):
        // an outer wall short of the fire resistance table 5 items 2/14
        // ask for the zone's grade is wholly open; one that meets it but
        // the designer marks as having a combustible face is partly
        // open; either its face less its openings (the wall's piece in
        // each stretch)
        let pieces: Vec<(ElementId, f64, f64)> = members
            .iter()
            .flat_map(|&j| {
                let q = &self.panels[j];
                let (a, b) = (along(q.origin + q.t * q.lo), along(q.origin + q.t * q.hi));
                q.walls.iter().map(move |&w| (w, a.min(b), a.max(b)))
            })
            .collect();
        for (w, lo, hi) in pieces {
            let Some(ElementKind::Wall(wall)) = model.get(w).map(|e| &e.kind) else {
                continue;
            };
            // 5.4.3/5.4.4: short of table 5 (minutes, REW or EW, D1)
            // a wholly fire-open area
            let short = grade
                .and_then(|g| crate::firegrade::wall_check(model, g, wall))
                .is_some_and(|(_, v)| match v {
                    crate::firegrade::WallVerdict::Meets => false,
                    crate::firegrade::WallVerdict::Unstated => reader.wall_unstated_open,
                    crate::firegrade::WallVerdict::KindUnstated => reader.kind_unstated_open,
                    crate::firegrade::WallVerdict::Short(_) => true,
                });
            if !short && !wall.fire_partial {
                continue;
            }
            let (x, y) = (along(wall.start), along(wall.end));
            let (x, y) = (x.min(y).max(lo), x.max(y).min(hi));
            if y - x < 1e-6 {
                continue;
            }
            let z0 = model.level(wall.level).map_or(0.0, |l| l.elevation) + wall.base_offset;
            let h = creabim_core::walls::constrained_height(model, wall).unwrap_or(wall.height);
            let holes: f64 = parts
                .iter()
                .filter(|o| {
                    let m = (o.rect[0] + o.rect[1]) / 2.0;
                    sources.iter().any(|s| s.id == o.id && s.host == w) && m >= x && m <= y
                })
                .map(|o| o.area)
                .sum();
            let area = ((x - y).abs() * h - holes).max(0.0);
            if area > 0.0 {
                parts.push(Part {
                    id: w,
                    level: wall.level,
                    rect: [x.min(y), x.max(y), z0, z0 + h],
                    kind: if short {
                        PartKind::Full
                    } else {
                        PartKind::Partial
                    },
                    area,
                    pvm: None,
                });
            }
        }
        // 4.3: a roof over the stretch, falling toward it or flat, seen
        // square onto the plane of the face: steeper than the limit as
        // high as it rises (at least hu_roof), flatter as long as it
        // runs along the face and hu_roof high
        let lines: Vec<(ElementId, f64, f64)> = members
            .iter()
            .map(|&j| {
                let q = &self.panels[j];
                let (a, b) = (
                    (q.origin + q.t * q.lo - p0.origin).dot(p0.t),
                    (q.origin + q.t * q.hi - p0.origin).dot(p0.t),
                );
                (q.level, a.min(b), a.max(b))
            })
            .collect();
        // 4.1.4: a roof is fire-open where the designer says so, or
        // where its covering falls short of the fire resistance table 5
        // of STN 92 0201-2 asks for the zone's grade (its note 4)); one
        // not stated as the designer says (`roof_unstated`)
        let required = grade.and_then(|g| crate::firegrade::roof_required(model, g));
        let open = |id: ElementId| {
            reader.roof_open
                || required.is_some_and(|need| {
                    crate::firegrade::roof_declared(model, id)
                        .map_or(reader.roof_unstated_open, |has| has < need)
                })
        };
        for r in self.roofs {
            if !open(r.id) {
                continue;
            }
            let Some(&(_, lo, hi)) = lines.iter().find(|l| l.0 == r.level) else {
                continue;
            };
            let flat = r.pitch <= reader.roof_steep;
            if !flat && r.down.dot(p0.outward) < 0.5 {
                continue;
            }
            // its eave runs along the face (out to the overhang)
            let near = r.points.iter().any(|p| {
                let out = (*p - p0.origin).dot(p0.outward) - top;
                out > -0.05 && out < reader.roof_reach
            });
            if !near {
                continue;
            }
            let xs: Vec<f64> = r.points.iter().map(|p| along(*p)).collect();
            let x0 = xs.iter().copied().fold(f64::INFINITY, f64::min).max(lo);
            let x1 = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max).min(hi);
            if x1 - x0 < 1e-3 {
                continue;
            }
            let z0 = r.z.iter().copied().fold(f64::INFINITY, f64::min);
            let z1 = if flat {
                z0 + reader.hu_roof
            } else {
                r.z.iter()
                    .copied()
                    .fold(f64::NEG_INFINITY, f64::max)
                    .max(z0 + reader.hu_roof)
            };
            parts.push(Part {
                id: r.id,
                level: r.level,
                rect: [x0, x1, z0, z1],
                kind: PartKind::Roof,
                area: (x1 - x0) * (z1 - z0),
                pvm: None,
            });
        }
        // 2.6.4 f) (Z3): a single fully open area under 0,5 m long and
        // high has no space; areas whose neighbouring edges are under
        // 1 m apart are not single
        let small = reader.small.unwrap_or(0.0);
        let near = reader.small_gap.unwrap_or(0.0);
        let keep: Vec<bool> = parts
            .iter()
            .map(|o| {
                let r = o.rect;
                let tiny = o.kind == PartKind::Full && r[1] - r[0] < small && r[3] - r[2] < small;
                !tiny
                    || parts
                        .iter()
                        .any(|q| q.id != o.id && gap(&r, &q.rect) < near)
            })
            .collect();
        let parts: Vec<Part> = parts
            .into_iter()
            .zip(keep)
            .filter_map(|(p, k)| k.then_some(p))
            .collect();
        if parts.is_empty() {
            return vec![];
        }
        let mut pv: Option<f64> = None;
        for o in &sources {
            match self.loads.of(model, o) {
                Ok(Some(v)) => pv = Some(pv.map_or(v, |x: f64| x.max(v))),
                Ok(None) => {}
                // 2.6.4 a): a zone without fire risk has no space
                Err(()) => return vec![],
            }
        }
        // 4.4.1: a combustible or mixed structural system adds to pv
        let pv = pv.map(|v| v + reader.pv_add(model));
        let storeys = storeys_of(model, &p0.zone);
        let table = reader.table_of(model, &p0.zone);
        let given = crate::firezones::distance_of(model, p0.facing);
        // 5.3.1: Sp the smallest rectangle holding the fire-open areas
        // of a group, Spo by 4.2.2, po the share, d off the table
        let combined = |group: &[usize]| -> Option<Group> {
            let mut r = [
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ];
            let mut spo = [0.0; 3];
            // 4.4.5: the fully open area in front of a concentrated
            // load, and its load
            let mut s2po = 0.0;
            let mut pvm: Option<f64> = None;
            for &i in group {
                let q = parts[i].rect;
                r = [
                    r[0].min(q[0]),
                    r[1].max(q[1]),
                    r[2].min(q[2]),
                    r[3].max(q[3]),
                ];
                match parts[i].pvm {
                    Some(m) if parts[i].kind == PartKind::Full => {
                        s2po += parts[i].area;
                        pvm = Some(pvm.map_or(m, |x: f64| x.max(m)));
                    }
                    _ => spo[parts[i].kind as usize] += parts[i].area,
                }
            }
            let (lu, hu) = (r[1] - r[0], r[3] - r[2]);
            if lu <= 1e-6 || hu <= 1e-6 {
                return None;
            }
            let sp = lu * hu;
            let mut load = pv;
            let open = if spo[1] + spo[2] > 0.0 {
                let (k10, k11) = reader.k(pv?)?;
                let s2 = spo[0] + k10 * spo[1] + k11 * spo[2];
                if s2 > sp {
                    // equation (3), d then read for pv_eq3
                    load = Some(reader.pv_eq3?);
                    spo[0] / k11 + k10 * spo[1] / k11 + spo[2]
                } else {
                    s2
                }
            } else {
                spo[0]
            };
            // 4.4.5: equation (6), or (7) with d read for pvm where (6)
            // exceeds Sp
            let open = match pvm {
                Some(m) if s2po > 0.0 => {
                    let (q1, q2) = (reader.q(pv?)?, reader.q(m)?);
                    let s6 = open + q2 / q1 * s2po;
                    if s6 > sp {
                        load = Some(m);
                        q1 / q2 * open + s2po
                    } else {
                        s6
                    }
                }
                _ => open,
            };
            let po = (open / sp * 100.0).min(100.0);
            let d = match given {
                Some(d) => Some(d),
                None => reader.distance(table, lu, hu, po, load, storeys),
            }?;
            Some(Group {
                members: group.to_vec(),
                rect: r,
                lu,
                hu,
                po,
                d,
                single: false,
                pv: load,
            })
        };
        let all_of: Vec<usize> = (0..parts.len()).collect();
        let Some(whole) = combined(&all_of) else {
            return vec![];
        };
        // 3.2.4 (Z3): with po under the limit, d for each fully open
        // area off table 4; those closer than 0,6 × (d1 + d2) are taken
        // together by 5.3.1, until no two groups are that close
        let mut groups = vec![whole];
        if let (None, Some(limit), Some(k), Some(t4)) =
            (given, reader.single_po, reader.single_gap, reader.single)
        {
            if groups[0].po < limit {
                let load = pv.unwrap_or(0.0);
                let singles: Option<Vec<Group>> = all_of
                    .iter()
                    .map(|&i| {
                        let r = parts[i].rect;
                        let (l, h) = (r[1] - r[0], r[3] - r[2]);
                        match (parts[i].kind, single(t4, l, h, load)) {
                            (PartKind::Full, Some(d)) => Some(Group {
                                members: vec![i],
                                rect: r,
                                lu: l,
                                hu: h,
                                po: 100.0,
                                d,
                                single: true,
                                pv,
                            }),
                            _ => combined(&[i]),
                        }
                    })
                    .collect();
                if let Some(mut gs) = singles {
                    loop {
                        let pair = (0..gs.len()).find_map(|i| {
                            (i + 1..gs.len())
                                .find(|&j| gap(&gs[i].rect, &gs[j].rect) < k * (gs[i].d + gs[j].d))
                                .map(|j| (i, j))
                        });
                        let Some((i, j)) = pair else {
                            break;
                        };
                        let b = gs.remove(j);
                        let mut m = gs[i].members.clone();
                        m.extend(b.members);
                        match combined(&m) {
                            Some(g) => gs[i] = g,
                            None => break,
                        }
                    }
                    groups = gs;
                }
            }
        }
        let base = p0.origin + p0.outward * top;
        let mut out = vec![];
        for g in groups {
            if g.d <= 1e-3 {
                continue;
            }
            let ms: Vec<&Part> = g.members.iter().map(|&i| &parts[i]).collect();
            let mut levels: Vec<ElementId> = ms.iter().map(|m| m.level).collect();
            levels.sort_by_key(|l| l.0);
            levels.dedup();
            let (e0, e1) = (base + p0.t * g.rect[0], base + p0.t * g.rect[1]);
            for &l in &levels {
                // a roof alone on a storey is drawn with its openings'
                if !ms.iter().any(|m| m.level == l && m.kind != PartKind::Roof) {
                    continue;
                }
                let first = ms
                    .iter()
                    .find(|m| m.level == l && m.kind != PartKind::Roof)
                    .map_or(ms[0].id, |m| m.id);
                let mut shared: Vec<String> = ms
                    .iter()
                    .filter(|m| m.level != l && m.kind != PartKind::Roof)
                    .filter_map(|m| model.get(m.level).map(|x| tr(&x.name)))
                    .collect();
                shared.sort();
                shared.dedup();
                out.push(Pnp {
                    opening: first,
                    level: l,
                    members: ms.iter().map(|m| m.id).collect(),
                    outline: norm_outline(e0, e1, p0.outward, g.d, reader.side_angle),
                    d: g.d,
                    at: (e0 + e1) / 2.0,
                    outward: p0.outward,
                    shared,
                    lu: g.lu,
                    hu: g.hu,
                    po: g.po,
                    pv: (table == 3 || g.single).then_some(g.pv).flatten(),
                    zone: p0.zone.clone(),
                });
            }
        }
        out
    }
}

/// Fire-open areas taken together, as rectangles in the plane of the
/// face: along it from..to, bottom..top.
struct Group {
    members: Vec<usize>,
    rect: [f64; 4],
    lu: f64,
    hu: f64,
    po: f64,
    d: f64,
    /// d read off table 4 for a single opening.
    single: bool,
    /// The load d is read for.
    pv: Option<f64>,
}

/// The distance between the edges of two areas of the face.
fn gap(a: &[f64; 4], b: &[f64; 4]) -> f64 {
    let dx = (b[0] - a[1]).max(a[0] - b[1]).max(0.0);
    let dz = (b[2] - a[3]).max(a[2] - b[3]).max(0.0);
    dx.hypot(dz)
}

/// Table 4 (Z3): d of a single opening l × h; between the lengths and
/// heights linearly (its note 2), pv in the next higher column; none
/// beyond the table's openings.
fn single(t: &ParamTable, l: f64, h: f64, pv: f64) -> Option<f64> {
    let n = |s: &str| fireparams::number(s);
    let rows: Vec<(f64, f64, &Vec<String>)> = t
        .rows
        .iter()
        .filter_map(|r| Some((n(r.first()?)?, n(r.get(1)?)?, r)))
        .collect();
    let pvs: Vec<(usize, f64)> = (2..t.columns.len())
        .filter_map(|c| Some((c, n(&t.columns[c])?)))
        .collect();
    if rows.is_empty() || pvs.is_empty() {
        return None;
    }
    let ls = sorted(rows.iter().map(|r| r.0).collect());
    let hs = sorted(rows.iter().map(|r| r.1).collect());
    if l > ls[ls.len() - 1] + 1e-9 || h > hs[hs.len() - 1] + 1e-9 {
        return None;
    }
    let col = pvs
        .iter()
        .find(|c| c.1 >= pv - 1e-9)
        .unwrap_or(&pvs[pvs.len() - 1])
        .0;
    let cell = |a: f64, b: f64| {
        rows.iter()
            .find(|r| r.0 == a && r.1 == b)
            .and_then(|r| n(r.2.get(col)?))
    };
    let (i, j, fl) = bracket(&ls, l);
    let (u, v, fh) = bracket(&hs, h);
    let at = |x: f64| -> Option<f64> {
        let (a, b) = (cell(x, hs[u])?, cell(x, hs[v])?);
        Some(a + (b - a) * fh)
    };
    let (a, b) = (at(ls[i])?, at(ls[j])?);
    Some(a + (b - a) * fl)
}

/// The storeys a fire zone's rooms are on (the rooms with no zone for
/// an empty code).
fn storeys_of(model: &Model, zone: &str) -> usize {
    let mut levels: Vec<ElementId> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Room(r) if r.fire_zone.trim() == zone => Some(r.level),
            _ => None,
        })
        .collect();
    levels.sort_by_key(|l| l.0);
    levels.dedup();
    levels.len().max(1)
}

/// The norm's tables as the distances are read off them.
struct Reader<'a> {
    table: u32,
    d3: bool,
    side_angle: f64,
    factors: (f64, f64),
    d: Option<&'a ParamTable>,
    /// Tables 6 and 7; with `table` 0 a zone takes 6 when all its
    /// rooms' fire load items are among `dwelling_loads`, 7 when among
    /// `lodging_loads`, else table 3.
    dwelling_cells: Option<&'a ParamTable>,
    lodging_cells: Option<&'a ParamTable>,
    dwelling_loads: Vec<String>,
    lodging_loads: Vec<String>,
    /// 3.2.4: table 4, the po under which it is used and the share of
    /// the sum of two distances neighbours must be apart.
    single: Option<&'a ParamTable>,
    single_po: Option<f64>,
    single_gap: Option<f64>,
    /// 2.6.4 f): the length and height a single area is under to have
    /// no space, and how near a neighbour makes it not single.
    small: Option<f64>,
    small_gap: Option<f64>,
    /// 3.2.1 b): the share of the zone's fire-open area, %, a projection
    /// may hold, and how far it may stand out: a part of d and at most.
    protrusion_share: Option<f64>,
    protrusion_depth: Option<f64>,
    protrusion_max: Option<f64>,
    /// Table 2 (k10, k11 by t), 4.2.2's pv for equation (3), the fire
    /// door ratings that make a partly fire-open area (4.1.3 c)).
    k: Option<&'a ParamTable>,
    pv_eq3: Option<f64>,
    partial: Vec<String>,
    /// 4.1.4, 4.3: whether the roofs are fire-open (the designer's
    /// judgement), the pitch over which a roof is seen square onto the
    /// face, its least height there, how far out its eave may reach.
    roof_open: bool,
    /// A roof whose assembly states no fire resistance: fire-open (not
    /// shown) unless the designer takes it as meeting the requirement.
    roof_unstated_open: bool,
    /// An outer wall whose assembly states no fire resistance: wholly
    /// fire-open (not shown) unless the designer takes it as meeting the
    /// requirement (George 2026-11-20: open, and say so).
    wall_unstated_open: bool,
    /// D1 asked and the wall's rating names no kind: open (1) or taken
    /// as meeting it (0).
    kind_unstated_open: bool,
    roof_steep: f64,
    hu_roof: f64,
    roof_reach: f64,
    /// 4.4.1 (Z3): what each structural system adds to pv.
    pv_adds: [Option<f64>; 4],
}

impl<'a> Reader<'a> {
    fn new(model: &'a Model, t: Option<&'a ParamTable>) -> Self {
        let table = value(t, "table").map_or(3, |v| v.round() as u32);
        let items = |key: &str| -> Vec<String> {
            t.map(|t| t.cell(key, "value"))
                .unwrap_or_default()
                .split([';', ' '])
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect()
        };
        Self {
            table,
            dwelling_cells: t.and(crate::firerisk::table(model, FIRE_DISTANCE_DWELLING)),
            lodging_cells: t.and(crate::firerisk::table(model, FIRE_DISTANCE_LODGING)),
            dwelling_loads: items("dwelling_loads"),
            lodging_loads: items("lodging_loads"),
            d3: value(t, "d3").is_some_and(|v| v > 0.5),
            side_angle: value(t, "side_angle").unwrap_or(160.0),
            factors: (
                value(t, "two_storey").unwrap_or(1.0),
                value(t, "three_storey").unwrap_or(1.0),
            ),
            d: t.and(crate::firerisk::table(model, FIRE_DISTANCE_D)),
            single: t.and(crate::firerisk::table(model, FIRE_DISTANCE_SINGLE)),
            single_po: value(t, "single_po"),
            single_gap: value(t, "single_gap"),
            small: value(t, "small"),
            small_gap: value(t, "small_gap"),
            protrusion_share: value(t, "protrusion_share"),
            protrusion_depth: value(t, "protrusion_depth"),
            protrusion_max: value(t, "protrusion_max"),
            k: t.and(crate::firerisk::table(model, FIRE_DISTANCE_K)),
            pv_eq3: value(t, "pv_eq3"),
            partial: t
                .map(|t| t.cell("partial_ratings", "value"))
                .unwrap_or_default()
                .split([',', ';', ' '])
                .map(|x| x.trim().to_uppercase())
                .filter(|x| !x.is_empty())
                .collect(),
            roof_open: value(t, "roof_open").is_some_and(|v| v > 0.5),
            roof_unstated_open: value(t, "roof_unstated").is_none_or(|v| v > 0.5),
            wall_unstated_open: value(t, "wall_unstated").is_none_or(|v| v > 0.5),
            kind_unstated_open: value(t, "kind_unstated").is_none_or(|v| v > 0.5),
            roof_steep: value(t, "roof_steep").unwrap_or(15.0),
            hu_roof: value(t, "hu_roof").unwrap_or(2.0),
            roof_reach: value(t, "roof_reach").unwrap_or(1.5),
            pv_adds: [
                value(t, "pv_add_za"),
                value(t, "pv_add_zb"),
                value(t, "pv_add_ha"),
                value(t, "pv_add_hb"),
            ],
        }
    }

    /// 4.4.1: what the building's structural system adds to pv.
    fn pv_add(&self, model: &Model) -> f64 {
        let system = model
            .project_info()
            .map(|p| p.1.fire_system.trim().to_lowercase())
            .unwrap_or_default();
        let i = match system.as_str() {
            "za" => 0,
            "zb" => 1,
            "ha" => 2,
            "hb" => 3,
            _ => return 0.0,
        };
        self.pv_adds[i].unwrap_or(0.0)
    }

    /// Table 2: the heat flux q for t (pv), kW/m², between its rows
    /// linearly.
    fn q(&self, t: f64) -> Option<f64> {
        let tab = self.k?;
        let n = |s: &str| fireparams::number(s);
        let rows: Vec<(f64, f64)> = tab
            .rows
            .iter()
            .filter_map(|r| Some((n(r.first()?)?, n(r.get(3)?)?)))
            .collect();
        if rows.is_empty() {
            return None;
        }
        let keys = sorted(rows.iter().map(|r| r.0).collect());
        let (i, j, f) = bracket(&keys, t);
        let at = |k: f64| rows.iter().find(|r| r.0 == k).map(|r| r.1);
        let (a, b) = (at(keys[i])?, at(keys[j])?);
        Some(a + (b - a) * f)
    }

    /// Table 2: k10 and k11 for t (pv), between its rows linearly.
    fn k(&self, t: f64) -> Option<(f64, f64)> {
        let tab = self.k?;
        let n = |s: &str| fireparams::number(s);
        let rows: Vec<(f64, f64, f64)> = tab
            .rows
            .iter()
            .filter_map(|r| Some((n(r.first()?)?, n(r.get(1)?)?, n(r.get(2)?)?)))
            .collect();
        if rows.is_empty() {
            return None;
        }
        let keys = sorted(rows.iter().map(|r| r.0).collect());
        let (i, j, f) = bracket(&keys, t);
        let at = |k: f64| rows.iter().find(|r| r.0 == k).copied();
        let (a, b) = (at(keys[i])?, at(keys[j])?);
        Some((a.1 + (b.1 - a.1) * f, a.2 + (b.2 - a.2) * f))
    }

    /// The table a zone's d is read off: the one set, or with 0 by what
    /// its rooms are for (their fire load items, given or by type).
    fn table_of(&self, model: &Model, zone: &str) -> u32 {
        if self.table != 0 {
            return self.table;
        }
        let auto = crate::firerisk::AutoUses::new(model);
        let items: Vec<String> = model
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Room(r) if r.fire_zone.trim() == zone => {
                    Some(auto.fire_load(r, &e.name))
                }
                _ => None,
            })
            .collect();
        let all_in = |list: &[String]| {
            !items.is_empty() && !list.is_empty() && items.iter().all(|i| list.contains(i))
        };
        if all_in(&self.dwelling_loads) {
            6
        } else if all_in(&self.lodging_loads) {
            7
        } else {
            3
        }
    }

    /// d for an area lu × hu with po % open, of a zone with `pv` and
    /// `storeys` storeys; none without the table it needs.
    #[allow(clippy::too_many_arguments)]
    fn distance(
        &self,
        table: u32,
        lu: f64,
        hu: f64,
        po: f64,
        pv: Option<f64>,
        storeys: usize,
    ) -> Option<f64> {
        match table {
            6 | 7 => {
                let cells = if table == 6 {
                    self.dwelling_cells
                } else {
                    self.lodging_cells
                };
                let d = dwelling(cells?, lu, po, self.d3)?;
                let f = match storeys {
                    0 | 1 => 1.0,
                    2 => self.factors.0,
                    _ => self.factors.1,
                };
                Some(d * f)
            }
            _ => table3(self.d?, hu, lu, po, pv?),
        }
    }
}

/// Where `x` falls among the sorted `keys`: the two around it and the
/// share of the way from one to the other, held at the ends.
fn bracket(keys: &[f64], x: f64) -> (usize, usize, f64) {
    let last = keys.len() - 1;
    if x <= keys[0] {
        return (0, 0, 0.0);
    }
    for i in 0..last {
        if x <= keys[i + 1] {
            let span = keys[i + 1] - keys[i];
            return (
                i,
                i + 1,
                if span > 0.0 {
                    (x - keys[i]) / span
                } else {
                    0.0
                },
            );
        }
    }
    (last, last, 0.0)
}

fn sorted(mut v: Vec<f64>) -> Vec<f64> {
    v.sort_by(f64::total_cmp);
    v.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    v
}

/// Table 3: rows hu, lu, po, then a column for each pv.
fn table3(t: &ParamTable, hu: f64, lu: f64, po: f64, pv: f64) -> Option<f64> {
    let n = |s: &str| fireparams::number(s);
    let rows: Vec<(f64, f64, f64, &Vec<String>)> = t
        .rows
        .iter()
        .filter_map(|r| Some((n(r.first()?)?, n(r.get(1)?)?, n(r.get(2)?)?, r)))
        .collect();
    let pvs: Vec<(usize, f64)> = (3..t.columns.len())
        .filter_map(|c| Some((c, n(&t.columns[c])?)))
        .collect();
    if rows.is_empty() || pvs.is_empty() {
        return None;
    }
    let pv_keys: Vec<f64> = pvs.iter().map(|p| p.1).collect();
    let (c0, c1, fc) = bracket(&pv_keys, pv);
    // the note (as Z3 has it): linearly also between the lowest values
    // and nought
    let toward = |x: f64, low: f64| {
        if x < low && low > 0.0 {
            x.max(0.0) / low
        } else {
            1.0
        }
    };
    let cell = |r: &Vec<String>| -> Option<f64> {
        let a = n(r.get(pvs[c0].0)?)?;
        let b = n(r.get(pvs[c1].0)?)?;
        Some((a + (b - a) * fc) * toward(pv, pv_keys[0]))
    };
    let at_po = |h: f64, l: f64| -> Option<f64> {
        let these: Vec<_> = rows.iter().filter(|r| r.0 == h && r.1 == l).collect();
        let keys = sorted(these.iter().map(|r| r.2).collect());
        if keys.is_empty() {
            return None;
        }
        let (i, j, f) = bracket(&keys, po);
        let v = |k: f64| these.iter().find(|r| r.2 == k).and_then(|r| cell(r.3));
        let (a, b) = (v(keys[i])?, v(keys[j])?);
        Some((a + (b - a) * f) * toward(po, keys[0]))
    };
    let at_lu = |h: f64| -> Option<f64> {
        let keys = sorted(rows.iter().filter(|r| r.0 == h).map(|r| r.1).collect());
        let (i, j, f) = bracket(&keys, lu);
        let (a, b) = (at_po(h, keys[i])?, at_po(h, keys[j])?);
        Some((a + (b - a) * f) * toward(lu, keys[0]))
    };
    let keys = sorted(rows.iter().map(|r| r.0).collect());
    let (i, j, f) = bracket(&keys, hu);
    let (a, b) = (at_lu(keys[i])?, at_lu(keys[j])?);
    // the note (as Z3 has it): linearly also between the lowest values
    // and nought
    Some((a + (b - a) * f) * toward(hu, keys[0]))
}

/// Tables 6 and 7: rows po, a column for each lu, cells "a/b".
fn dwelling(t: &ParamTable, lu: f64, po: f64, d3: bool) -> Option<f64> {
    let n = |s: &str| fireparams::number(s);
    let lus: Vec<(usize, f64)> = (1..t.columns.len())
        .filter_map(|c| Some((c, n(&t.columns[c])?)))
        .collect();
    let rows: Vec<(f64, &Vec<String>)> = t
        .rows
        .iter()
        .filter_map(|r| Some((n(r.first()?)?, r)))
        .collect();
    if lus.is_empty() || rows.is_empty() {
        return None;
    }
    let cell = |r: &Vec<String>, c: usize| -> Option<f64> {
        let s = r.get(c)?;
        let (a, b) = s.split_once('/').unwrap_or((s, s));
        n(if d3 { b } else { a })
    };
    let lu_keys: Vec<f64> = lus.iter().map(|l| l.1).collect();
    let (c0, c1, fc) = bracket(&lu_keys, lu);
    let po_keys = sorted(rows.iter().map(|r| r.0).collect());
    let (r0, r1, fr) = bracket(&po_keys, po);
    let at = |k: f64| -> Option<f64> {
        let r = rows.iter().find(|r| r.0 == k)?.1;
        let (a, b) = (cell(r, lus[c0].0)?, cell(r, lus[c1].0)?);
        Some(a + (b - a) * fc)
    };
    let (a, b) = (at(po_keys[r0])?, at(po_keys[r1])?);
    Some(a + (b - a) * fr)
}

/// The space's outline as the norm draws it (2.2.2, figure 1): from each
/// edge of the fire-open area a straight line out at `angle` to the area
/// (160°: 20° off the wall) as far as d, an arc of radius d about the
/// edge round to straight out, and in front a straight line at d.
pub fn norm_outline(e0: DVec2, e1: DVec2, outward: DVec2, d: f64, angle: f64) -> Vec<DVec2> {
    let t = (e1 - e0).normalize_or_zero();
    let off = (180.0 - angle).clamp(0.0, 90.0).to_radians();
    let dir = |th: f64| t * th.cos() + outward * th.sin();
    let pi = std::f64::consts::PI;
    let half = pi / 2.0;
    let n = 16;
    let mut out = vec![e0];
    // about the first edge, from 20° off the wall on its side round to
    // straight out
    for i in 0..=n {
        let th = (pi - off) - (half - off) * i as f64 / n as f64;
        out.push(e0 + dir(th) * d);
    }
    for i in 0..=n {
        let th = half - (half - off) * i as f64 / n as f64;
        out.push(e1 + dir(th) * d);
    }
    out.push(e1);
    out
}

/// The label of a space: its distance, and the storeys it is shared
/// with on a second line.
pub fn label(p: &Pnp) -> String {
    let mut s = format!("PNP d = {:.2} m", p.d).replace('.', ",");
    if !p.shared.is_empty() {
        s += &format!(
            "\n{} {}",
            tr("shared with the openings of"),
            p.shared.join(", ")
        );
    }
    s
}

/// The spaces as a plan draws them: a dashed red outline, its label
/// beyond its farthest point along the wall. `mm` is metres to a paper
/// millimetre.
pub fn items(model: &Model, level: ElementId, only: Option<ElementId>, mm: f64) -> Vec<PlanItem> {
    let mut items = vec![];
    for p in spaces(model, Some(level), only) {
        items.extend(items_of(&p, mm));
    }
    // outer walls whose assembly states no fire resistance where one is
    // asked: a note on the wall's outer face
    if only.is_none() {
        for f in crate::fireexposure::failing_walls(model) {
            use crate::firegrade::WallVerdict;
            let (id, w, outward) = (f.id, f.wall, f.outward);
            if w.level != level {
                continue;
            }
            let text = match f.verdict {
                WallVerdict::Unstated => tr("NO FIRE RESISTANCE STATED: fire-open area"),
                WallVerdict::Short(_) => format!(
                    "{} {}: {}",
                    tr("FIRE RESISTANCE SHORT OF"),
                    f.need.text(),
                    tr("fire-open area")
                ),
                WallVerdict::KindUnstated => tr("KIND D1 NOT STATED"),
                WallVerdict::Meets => continue,
            };
            let mid = (w.start + w.end) / 2.0 + outward * (w.thickness / 2.0 + 2.5 * mm);
            let along = (w.end - w.start).normalize_or_zero();
            let angle = if along.x < -1e-9 || (along.x.abs() < 1e-9 && along.y < 0.0) {
                (-along).to_angle()
            } else {
                along.to_angle()
            };
            items.push(PlanItem::Annotation {
                id,
                item: Box::new(Annotation {
                    shape: Shape::Along { at: mid, angle },
                    text,
                    look: TextLook {
                        size: 2.0,
                        rgb: Some(PNP_RED),
                        ..TextLook::default()
                    },
                    mm,
                }),
            });
        }
    }
    items
}

/// One space's outline and label.
pub fn items_of(p: &Pnp, mm: f64) -> Vec<PlanItem> {
    let look = TextLook {
        size: 2.5,
        rgb: Some(PNP_RED),
        ..TextLook::default()
    };
    let lines = label(p).lines().count() as f64;
    let along = p.outward.perp();
    vec![
        PlanItem::Curve {
            id: p.opening,
            points: p.outline.clone(),
            rgb: PNP_RED,
            dashed: true,
        },
        PlanItem::Annotation {
            id: p.opening,
            item: Box::new(Annotation {
                shape: Shape::Along {
                    at: p.at + p.outward * (p.d + (1.5 + lines * 1.6) * mm),
                    angle: along.to_angle(),
                },
                text: label(p),
                look,
                mm,
            }),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_tables(m: &mut Model, text: &str) {
        let mut info = m.project_info().map(|p| p.1.clone()).unwrap_or_default();
        fireparams::merge(&mut info.fire_params, fireparams::parse(text));
        let tx = creabim_core::set_project(m, info);
        tx.apply(m).unwrap();
    }

    /// A made-up table in the norm's layout (not its values).
    const TABLES: &str = "#table fire_distance\n#columns key\tvalue\tnote\n\
        table\t3\t\nside_angle\t160\t\ntwo_storey\t1,5\t\nthree_storey\t2\t\npv_default\t50\t\n\
        #table fire_distance_d\n#columns hu\tlu\tpo\t10\t100\n\
        3\t4,5\t100\t2\t6\n3\t4,5\t20\t0\t2\n3\t36\t100\t4\t12\n3\t36\t20\t0\t3\n\
        18\t9\t100\t8\t20\n18\t9\t20\t0\t5\n18\t45\t100\t18\t44\n18\t45\t20\t0\t12\n\
        #table fire_distance_dwelling\n#columns po\t4,5\t24\n100\t4,4/5,4\t7,6/10,1\n20\t0,7/1,5\t0,8/1,8\n";

    #[test]
    fn table_3_is_read_between_its_values_and_held_at_its_ends() {
        let t = &fireparams::parse(TABLES)[1];
        assert_eq!(table3(t, 3.0, 4.5, 100.0, 10.0), Some(2.0));
        assert_eq!(table3(t, 3.0, 4.5, 100.0, 55.0), Some(4.0));
        assert_eq!(table3(t, 3.0, 4.5, 60.0, 100.0), Some(4.0));
        // under the lowest values toward nought (the note as Z3 has it),
        // over the highest held
        let low = table3(t, 1.5, 4.5, 100.0, 10.0).unwrap();
        assert!((low - 1.0).abs() < 1e-9);
        assert!((table3(t, 3.0, 4.5, 100.0, 5.0).unwrap() - 1.0).abs() < 1e-9);
        assert_eq!(table3(t, 30.0, 60.0, 100.0, 300.0), Some(44.0));
        // half way between two heights
        let mid = table3(t, 10.5, 9.0, 100.0, 10.0).unwrap();
        let (a, b) = (table3(t, 3.0, 9.0, 100.0, 10.0).unwrap(), 8.0);
        assert!((mid - (a + b) / 2.0).abs() < 1e-9);
    }

    #[test]
    fn table_6_takes_the_value_for_d3_after_the_slash() {
        let t = &fireparams::parse(TABLES)[2];
        assert_eq!(dwelling(t, 4.5, 100.0, false), Some(4.4));
        assert_eq!(dwelling(t, 4.5, 100.0, true), Some(5.4));
        assert!((dwelling(t, 4.5, 60.0, false).unwrap() - 2.55).abs() < 1e-9);
    }

    /// Figure 1: out of each edge at 20° off the wall as far as d, round
    /// about the edge, straight across at d.
    #[test]
    fn the_outline_is_the_norms() {
        let o = norm_outline(DVec2::ZERO, DVec2::new(3.0, 0.0), DVec2::Y, 2.0, 160.0);
        assert_eq!(
            (o[0], *o.last().unwrap()),
            (DVec2::ZERO, DVec2::new(3.0, 0.0))
        );
        let a = 20f64.to_radians();
        assert!(o[1].distance(DVec2::new(-2.0 * a.cos(), 2.0 * a.sin())) < 1e-9);
        assert!(o.iter().all(|p| p.y <= 2.0 + 1e-9 && p.y >= 0.0));
        let front: Vec<_> = o.iter().filter(|p| (p.y - 2.0).abs() < 1e-9).collect();
        assert!(
            front.iter().any(|p| p.x.abs() < 1e-9)
                && front.iter().any(|p| (p.x - 3.0).abs() < 1e-9)
        );
    }

    #[test]
    fn nothing_is_drawn_without_the_tables_or_a_given_distance() {
        let m = creabim_core::starter::starter_house();
        assert!(spaces(&m, None, None).is_empty());
    }

    /// One space for each plane of the face, running on over all its
    /// openings, those of the storey over it shared.
    #[test]
    fn a_plane_of_the_face_gets_one_space_over_all_its_openings() {
        let mut m = creabim_core::starter::starter_house();
        with_tables(&mut m, TABLES);
        let all = spaces(&m, None, None);
        assert!(!all.is_empty());
        for p in &all {
            assert!(p.d > 0.0 && p.po > 0.0 && p.po <= 100.0, "{p:?}");
            assert_eq!(p.pv, Some(50.0));
        }
        assert!(all.iter().any(|p| !p.shared.is_empty()));
        // a selected window shows its plane's space
        let p = &all[0];
        let one = spaces(&m, None, Some(p.opening));
        assert!(one.iter().any(|q| q.outline == p.outline));
        // table 6 for a dwelling, the storeys' factor on it
        with_tables(&mut m, &TABLES.replace("table\t3", "table\t6"));
        let dw = spaces(&m, None, None);
        assert!(dw.iter().all(|p| p.pv.is_none() && p.d > 0.0));
    }

    /// 3.2.4: openings far apart for their own distances get a space
    /// each; near ones are taken together.
    #[test]
    fn single_openings_far_apart_get_their_own_spaces() {
        let mut m = creabim_core::starter::starter_house();
        with_tables(&mut m, TABLES);
        let whole = spaces(&m, None, None);
        let single = "#table fire_distance_single\n#columns l\th\t10\t100\n\
            0,5\t0,5\t0,1\t0,2\n0,5\t3\t0,2\t0,3\n5\t0,5\t0,2\t0,3\n5\t3\t0,3\t0,4\n";
        let extra = "#table fire_distance\n#columns key\tvalue\tnote\n\
            table\t3\t\nside_angle\t160\t\ntwo_storey\t1,5\t\nthree_storey\t2\t\npv_default\t50\t\n\
            single_po\t100\t\nsingle_gap\t0,6\t\n";
        with_tables(&mut m, &format!("{extra}{single}"));
        let apart = spaces(&m, None, None);
        assert!(apart.len() > whole.len(), "{} {}", apart.len(), whole.len());
        assert!(apart.iter().any(|p| p.d <= 0.4 && p.po == 100.0));
        // so near that every pair is taken together: as before
        with_tables(&mut m, &extra.replace("0,6", "100"));
        let near = spaces(&m, None, None);
        assert_eq!(near.len(), whole.len());
    }

    fn given(m: &mut Model, d: f64) {
        let mut info = m.project_info().map(|p| p.1.clone()).unwrap_or_default();
        info.fire_distances = vec![d; 8];
        let tx = creabim_core::set_project(m, info);
        tx.apply(m).unwrap();
    }

    /// 3.2.2: a stretch set back behind the reference plane further than
    /// d is reckoned on its own; nearer, it is projected into the plane.
    #[test]
    fn a_stretch_set_back_further_than_d_is_on_its_own() {
        let south = |m: &Model| {
            spaces(m, None, None)
                .into_iter()
                .filter(|p| p.outward.y < -0.9)
                .map(|p| p.members.len())
                .collect::<Vec<_>>()
        };
        // the starter house's kitchen wall is 4,6 m back
        let mut m = creabim_core::starter::starter_house();
        given(&mut m, 3.5);
        let apart = south(&m);
        given(&mut m, 5.0);
        let together = south(&m);
        assert!(apart.len() > together.len(), "{apart:?} {together:?}");
        assert_eq!(apart.iter().max(), Some(&8));
        assert!(together.iter().all(|&n| n == 9), "{together:?}");
    }

    /// 4.4.1 (Z3): the structural system adds to pv; 4.1.4: a fire-open
    /// roof adds its share to the face's area.
    #[test]
    fn the_structural_system_and_an_open_roof_count() {
        let extra = "#table fire_distance\n#columns key\tvalue\tnote\n\
            table\t3\t\nside_angle\t160\t\npv_default\t50\t\npv_add_hb\t25\t\n\
            pv_eq3\t30\t\nroof_steep\t15\t\nhu_roof\t2\t\n\
            #table fire_distance_k\n#columns t\tk10\tk11\tq\n15\t1\t1,5\t59\n100\t0,5\t0,7\t160\n";
        let mut m = creabim_core::starter::starter_house();
        with_tables(&mut m, &format!("{TABLES}{extra}"));
        let plain = spaces(&m, None, None);
        assert!(plain.iter().all(|p| p.pv == Some(50.0)));
        let mut info = m.project_info().map(|p| p.1.clone()).unwrap_or_default();
        info.fire_system = "Hb".into();
        let tx = creabim_core::set_project(&m, info);
        tx.apply(&mut m).unwrap();
        assert!(spaces(&m, None, None).iter().all(|p| p.pv == Some(75.0)));
        let hu = |v: &[Pnp]| v.iter().map(|p| p.hu).fold(0.0, f64::max);
        let before = hu(&spaces(&m, None, None));
        with_tables(
            &mut m,
            &format!(
                "{TABLES}{}",
                extra.replace("pv_eq3", "roof_open\t1\t\npv_eq3")
            ),
        );
        let roofed = spaces(&m, None, None);
        assert!(hu(&roofed) > before + 1.0, "{} {before}", hu(&roofed));
    }

    /// 4.1.3: a wall marked with a combustible face adds its face less
    /// its openings × k10; 4.4.5: openings in front of a concentrated
    /// load count × q''/q'.
    #[test]
    fn a_combustible_face_and_a_concentrated_load_count() {
        let extra = "#table fire_distance\n#columns key\tvalue\tnote\n\
            table\t3\t\nside_angle\t160\t\npv_default\t50\t\npv_eq3\t30\t\n\
            #table fire_distance_k\n#columns t\tk10\tk11\tq\n15\t1\t1,5\t59\n100\t0,5\t0,7\t160\n500\t0,2\t0,3\t300\n";
        let mut m = creabim_core::starter::starter_house();
        with_tables(&mut m, &format!("{TABLES}{extra}"));
        let south = |m: &Model| {
            spaces(m, None, None)
                .into_iter()
                .filter(|p| p.outward.y < -0.9)
                .map(|p| (p.po, p.d))
                .fold((0.0, 0.0), |a, b| (f64::max(a.0, b.0), f64::max(a.1, b.1)))
        };
        let before = south(&m);
        let wall = spaces(&m, None, None)
            .into_iter()
            .find(|p| p.outward.y < -0.9 && p.members.len() > 2)
            .and_then(|p| {
                m.get(p.members[0]).and_then(|e| match &e.kind {
                    ElementKind::Opening(o) => Some(o.host),
                    _ => None,
                })
            })
            .unwrap();
        let before_w = m.get(wall).unwrap().clone();
        let mut w = before_w.clone();
        if let ElementKind::Wall(x) = &mut w.kind {
            x.fire_partial = true;
        }
        creabim_core::Transaction::new("t")
            .replace(before_w, w)
            .apply(&mut m)
            .unwrap();
        let partial = south(&m);
        assert!(partial.1 > before.1 + 0.1, "{partial:?} {before:?}");
        // a concentrated load in every room
        let rooms: Vec<_> = m
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Room(_)))
            .cloned()
            .collect();
        let mut tx = creabim_core::Transaction::new("t");
        for e in rooms {
            let mut after = e.clone();
            if let ElementKind::Room(r) = &mut after.kind {
                r.fire_pvm = Some(500.0);
            }
            tx = tx.replace(e, after);
        }
        tx.apply(&mut m).unwrap();
        let loaded = south(&m);
        assert!(loaded.1 > partial.1, "{loaded:?} {partial:?}");
    }

    /// STN 92 0201-2 table 5 note 3): an outer wall short of the fire
    /// resistance its zone's grade asks is wholly open, more so than one
    /// with only a combustible face.
    #[test]
    fn a_wall_short_of_its_fire_resistance_is_wholly_open() {
        let extra = |unstated: u8| {
            format!(
                "#table fire_distance\n#columns key\tvalue\tnote\n\
                table\t3\t\nside_angle\t160\t\npv_default\t50\t\npv_eq3\t30\t\n\
                wall_unstated\t{unstated}\t\n\
                #table fire_distance_k\n#columns t\tk10\tk11\tq\n15\t1\t1,5\t59\n100\t0,5\t0,7\t160\n\
                #table fire_grade\n#columns key\tvalue\tnote\nrule\t1\t\n\
                #table fire_outer_wall\n#columns grade\tabove\tlast\tnonbearing\tsingle\n\
                1\t30\t15\t15\t15\n"
            )
        };
        let south = |m: &Model| {
            spaces(m, None, None)
                .into_iter()
                .filter(|p| p.outward.y < -0.9)
                .map(|p| p.d)
                .fold(0.0, f64::max)
        };
        let mut m = creabim_core::starter::starter_house();
        with_tables(&mut m, &format!("{TABLES}{}", extra(0)));
        let meets = south(&m);
        with_tables(&mut m, &format!("{TABLES}{}", extra(1)));
        let short = south(&m);
        assert!(short > meets + 0.1, "{short} {meets}");
    }

    /// The makers' fire data of the Slovak base (sk-fire.json): outer
    /// walls of PORFIX P2-440 (REI-M 240 by the PORFIX handbook) meet the
    /// grade and stay partly open; PORFIX Premium P2-400, for which the
    /// handbook states none, counts as wholly open.
    #[test]
    fn a_porfix_wall_with_its_handbook_rating_is_not_wholly_open() {
        let tables = format!(
            "{TABLES}#table fire_distance\n#columns key\tvalue\tnote\n\
             table\t3\t\nside_angle\t160\t\npv_default\t50\t\npv_eq3\t30\t\n\
             wall_unstated\t1\t\n\
             #table fire_distance_k\n#columns t\tk10\tk11\tq\n15\t1\t1,5\t59\n100\t0,5\t0,7\t160\n\
             #table fire_grade\n#columns key\tvalue\tnote\nrule\t1\t\n\
             #table fire_outer_wall\n#columns grade\tabove\tlast\tnonbearing\tsingle\n\
             1\t30\t15\t15\t15\n"
        );
        let south = |m: &Model| {
            spaces(m, None, None)
                .into_iter()
                .filter(|p| p.outward.y < -0.9)
                .map(|p| p.d)
                .fold(0.0, f64::max)
        };
        let with = |name: &str| {
            let mut m = creabim_core::starter::starter_house();
            creabim_core::skbase::add_base(&m).apply(&mut m).unwrap();
            let asm = creabim_core::assembly::assemblies(&m)
                .into_iter()
                .find(|(_, n, _)| *n == name)
                .map(|(id, _, _)| id)
                .unwrap();
            let walls: Vec<_> = m
                .iter()
                .filter(|e| matches!(e.kind, ElementKind::Wall(_)))
                .cloned()
                .collect();
            let mut tx = creabim_core::Transaction::new("t");
            for e in walls {
                let mut after = e.clone();
                if let ElementKind::Wall(w) = &mut after.kind {
                    w.assembly = Some(asm);
                }
                tx = tx.replace(e, after);
            }
            tx.apply(&mut m).unwrap();
            with_tables(&mut m, &tables);
            south(&m)
        };
        let rated = with("Obvodová – PORFIX P2-440 300 + EPS šedý 150");
        let unrated = with("Obvodová – PORFIX Premium P2-400 500 bez zateplenia");
        assert!(unrated > rated + 0.1, "{unrated} {rated}");
    }

    /// 5.4.3: a bearing outer wall asks REW (I stands for W), and a cell
    /// with "/D1" the kind D1; short of either the wall is wholly open.
    #[test]
    fn the_class_and_the_kind_count_not_only_the_minutes() {
        let tables = |kind_unstated: u8| {
            format!(
                "{TABLES}#table fire_distance\n#columns key\tvalue\tnote\n\
                 table\t3\t\nside_angle\t160\t\npv_default\t50\t\npv_eq3\t30\t\n\
                 wall_unstated\t1\t\nkind_unstated\t{kind_unstated}\t\n\
                 #table fire_distance_k\n#columns t\tk10\tk11\tq\n15\t1\t1,5\t59\n100\t0,5\t0,7\t160\n\
                 #table fire_grade\n#columns key\tvalue\tnote\nrule\t1\t\n\
                 #table fire_outer_wall\n#columns grade\tabove\tlast\tnonbearing\tsingle\n\
                 1\t30/D1\t15/D1\t15/D1\t15/D1\n"
            )
        };
        let south = |rating: &str, kind_unstated: u8| {
            let mut m = creabim_core::starter::starter_house();
            let all: Vec<_> = m
                .iter()
                .filter(|e| matches!(e.kind, ElementKind::Assembly(_) | ElementKind::Wall(_)))
                .cloned()
                .collect();
            let mut tx = creabim_core::Transaction::new("t");
            for e in all {
                let mut after = e.clone();
                match &mut after.kind {
                    ElementKind::Assembly(a) => {
                        a.fire_resistance = rating.into();
                        a.description.clear();
                    }
                    // every wall bearing: REW asked
                    ElementKind::Wall(w) => w.bearing = Some(true),
                    _ => {}
                }
                tx = tx.replace(e, after);
            }
            tx.apply(&mut m).unwrap();
            with_tables(&mut m, &tables(kind_unstated));
            spaces(&m, None, None)
                .into_iter()
                .filter(|p| p.outward.y < -0.9)
                .map(|p| p.d)
                .fold(0.0, f64::max)
        };
        let meets = south("REI 240/D1", 1);
        assert!(
            south("EI 240/D1", 1) > meets + 0.1,
            "no R on a bearing wall"
        );
        assert!(south("REI 240/D2", 1) > meets + 0.1, "D2 where D1 is asked");
        assert!(south("REI 240", 1) > meets + 0.1, "no kind, counted open");
        assert!(
            (south("REI 240", 0) - meets).abs() < 1e-9,
            "no kind, taken as met"
        );
        assert!((south("REW 240 D1", 1) - meets).abs() < 1e-9);
    }

    #[test]
    fn a_given_distance_is_drawn_without_tables() {
        let mut m = creabim_core::starter::starter_house();
        let mut info = m.project_info().map(|p| p.1.clone()).unwrap_or_default();
        info.fire_distances = vec![3.5; 8];
        let tx = creabim_core::set_project(&m, info);
        tx.apply(&mut m).unwrap();
        let all = spaces(&m, None, None);
        assert!(!all.is_empty() && all.iter().all(|p| p.d == 3.5));
    }

    /// A wall behind rooms of two zones gives each its own stretch: a
    /// window of one room keeps its space though the room at the wall's
    /// middle has none or is missing.
    #[test]
    fn a_wall_behind_two_zones_gives_each_its_stretch() {
        let base = creabim_core::starter::starter_house();
        let rooms: Vec<ElementId> = base
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Room(_)))
            .map(|e| e.id)
            .collect();
        assert!(rooms.len() > 1);
        for gone in &rooms {
            let mut m = base.clone();
            with_tables(&mut m, TABLES);
            // a zone of each room
            let mut tx = creabim_core::Transaction::new("zones");
            for (i, id) in rooms.iter().enumerate() {
                let before = m.get(*id).unwrap().clone();
                let mut after = before.clone();
                if let ElementKind::Room(r) = &mut after.kind {
                    r.fire_zone = format!("Z{i}");
                }
                tx = tx.replace(before, after);
            }
            tx.apply(&mut m).unwrap();
            let tx = creabim_core::Transaction::new("gone").remove_cascade(&m, *gone);
            tx.apply(&mut m).unwrap();
            let all = spaces(&m, None, None);
            let reader = Reader::new(&m, crate::firerisk::table(&m, FIRE_DISTANCE));
            for o in sources(&m, &reader.partial) {
                let behind = m.iter().any(|e| match &e.kind {
                    ElementKind::Room(r) if r.level == o.level => {
                        creabim_core::room::shape(&m, e.id).is_some_and(|sh| {
                            creabim_core::polygon::contains(&sh.outline, o.inside)
                        })
                    }
                    _ => false,
                });
                if behind {
                    assert!(
                        all.iter().any(|p| p.members.contains(&o.id)),
                        "opening {:?} lost its space with room {gone:?} gone",
                        o.id
                    );
                }
            }
        }
    }

    /// With table 0 a zone whose rooms are all dwellings by their fire
    /// load items takes table 6, others table 3.
    #[test]
    fn a_dwelling_zone_takes_table_6_by_itself() {
        let mut m = creabim_core::starter::starter_house();
        let mut tx = creabim_core::Transaction::new("items");
        let rooms: Vec<creabim_core::Element> = m
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Room(_)))
            .cloned()
            .collect();
        for before in rooms {
            let mut after = before.clone();
            if let ElementKind::Room(r) = &mut after.kind {
                r.fire_load = "B1".into();
            }
            tx = tx.replace(before, after);
        }
        tx.apply(&mut m).unwrap();
        with_tables(&mut m, &TABLES.replace("table\t3", "table\t6"));
        let six: Vec<f64> = spaces(&m, None, None).iter().map(|p| p.d).collect();
        with_tables(&mut m, TABLES);
        let three: Vec<f64> = spaces(&m, None, None).iter().map(|p| p.d).collect();
        assert_ne!(six, three);
        let auto = TABLES.replace("table\t3\t", "table\t0\t\ndwelling_loads\tB1\t");
        with_tables(&mut m, &auto);
        let found: Vec<f64> = spaces(&m, None, None).iter().map(|p| p.d).collect();
        assert_eq!(found, six);
        with_tables(
            &mut m,
            &auto.replace("dwelling_loads\tB1", "dwelling_loads\tB2"),
        );
        let found: Vec<f64> = spaces(&m, None, None).iter().map(|p| p.d).collect();
        assert_eq!(found, three);
    }
}
