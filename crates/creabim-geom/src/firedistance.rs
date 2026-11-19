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

/// The colour the space is drawn in.
pub const PNP_RED: [f32; 3] = crate::firezones::DANGER;

/// A fire zone's fire-dangerous space in front of one plane of its face.
#[derive(Clone, Debug, PartialEq)]
pub struct Pnp {
    /// The first of its openings on the storey drawn (or the one asked
    /// for), which the drawing's items carry.
    pub opening: ElementId,
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
}

/// An opening fire can pass, as it sits in an outer wall's face.
struct Source {
    id: ElementId,
    level: ElementId,
    /// The edges along the face at the wall's outer side, and the
    /// heights of the bottom and the top of the opening.
    a: DVec2,
    b: DVec2,
    z0: f64,
    z1: f64,
    /// The room behind it.
    inside: DVec2,
}

fn sources(model: &Model) -> Vec<Source> {
    let walls = crate::firezones::outer_walls(model);
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Opening(o) = &e.kind else {
            continue;
        };
        // a fire door holds the fire in
        if o.kind == OpeningKind::Door && o.door.as_ref().is_some_and(|d| d.fire.is_some()) {
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
            a,
            b,
            z0: base + o.sill,
            z1: base + o.sill + o.height,
            inside: mid - w.outward * (wall.thickness + 0.15),
        });
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

fn panels(model: &Model) -> Vec<Panel> {
    let mut out: Vec<Panel> = vec![];
    for w in crate::firezones::outer_walls(model) {
        let wall = w.wall;
        let face = w.outward * (wall.thickness / 2.0);
        let (a, b) = (wall.start + face, wall.end + face);
        let zone = zone_at(
            model,
            wall.level,
            (a + b) / 2.0 - w.outward * (wall.thickness + 0.15),
        );
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

/// The fire zone of the room at `p`; empty with none.
fn zone_at(model: &Model, level: ElementId, p: DVec2) -> String {
    model
        .iter()
        .find_map(|e| match &e.kind {
            ElementKind::Room(r) if r.level == level => {
                let sh = creabim_core::room::shape(model, e.id)?;
                creabim_core::polygon::contains(&sh.outline, p)
                    .then(|| r.fire_zone.trim().to_string())
            }
            _ => None,
        })
        .unwrap_or_default()
}

/// The fire-dangerous spaces of the fire zones' planes on `level` (every
/// level with none), or the one of the plane the opening `only` is in.
pub fn spaces(model: &Model, level: Option<ElementId>, only: Option<ElementId>) -> Vec<Pnp> {
    let all = sources(model);
    if all.is_empty() {
        return vec![];
    }
    let panels = panels(model);
    let host_of = |o: &Source| {
        model.get(o.id).and_then(|e| match &e.kind {
            ElementKind::Opening(x) => Some(x.host),
            _ => None,
        })
    };
    let wanted_host = only
        .and_then(|id| all.iter().find(|s| s.id == id))
        .and_then(host_of);
    if only.is_some() && wanted_host.is_none() {
        return vec![];
    }
    let table = crate::firerisk::table(model, FIRE_DISTANCE);
    let reader = Reader::new(model, table);
    let loads = Loads::new(model, table);
    let mut out = vec![];
    for p in &panels {
        if level.is_some_and(|l| l != p.level) || wanted_host.is_some_and(|h| !p.walls.contains(&h))
        {
            continue;
        }
        // the same zone's stretches of the other storeys in the plane
        let mut stack = vec![p];
        for q in &panels {
            if q.level == p.level
                || q.zone != p.zone
                || q.outward.dot(p.outward) < 0.99
                || (q.origin - p.origin).dot(p.outward).abs() > 0.05
            {
                continue;
            }
            let off = (q.origin - p.origin).dot(p.t);
            if q.hi + off > p.lo + 1e-3 && q.lo + off < p.hi - 1e-3 {
                stack.push(q);
            }
        }
        let openings: Vec<&Source> = all
            .iter()
            .filter(|o| host_of(o).is_some_and(|h| stack.iter().any(|q| q.walls.contains(&h))))
            .collect();
        // 2.6.4 f) (Z3): a single fully open area under 0,5 m long and
        // high has no space; areas whose neighbouring edges are under
        // 1 m apart are not single
        let along = |v: DVec2| (v - p.origin).dot(p.t);
        let rect = |o: &Source| {
            let (x, y) = (along(o.a), along(o.b));
            [x.min(y), x.max(y), o.z0, o.z1]
        };
        let small = reader.small.unwrap_or(0.0);
        let near = reader.small_gap.unwrap_or(0.0);
        let openings: Vec<&Source> = openings
            .iter()
            .copied()
            .filter(|o| {
                let r = rect(o);
                let tiny = r[1] - r[0] < small && r[3] - r[2] < small;
                !tiny
                    || openings
                        .iter()
                        .any(|q| q.id != o.id && gap(&r, &rect(q)) < near)
            })
            .collect();
        if !openings.iter().any(|o| o.level == p.level) {
            continue;
        }
        let mut pv: Option<f64> = None;
        let mut no_risk = false;
        for o in &openings {
            match loads.of(model, o) {
                Ok(Some(v)) => pv = Some(pv.map_or(v, |x: f64| x.max(v))),
                Ok(None) => {}
                Err(()) => no_risk = true,
            }
        }
        // 2.6.4 a): a zone without fire risk has no space
        if no_risk {
            continue;
        }
        let storeys = storeys_of(model, &p.zone);
        let given = crate::firezones::distance_of(model, p.facing);
        // 5.3.1: Sp the smallest rectangle holding the fire-open areas
        // of a group, po the share of it they make, d off the table
        let combined = |group: &[usize]| -> Option<Group> {
            let mut r = [
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
            ];
            let mut open = 0.0;
            for &i in group {
                let q = rect(openings[i]);
                r = [
                    r[0].min(q[0]),
                    r[1].max(q[1]),
                    r[2].min(q[2]),
                    r[3].max(q[3]),
                ];
                open += (q[1] - q[0]) * (q[3] - q[2]);
            }
            let (lu, hu) = (r[1] - r[0], r[3] - r[2]);
            if lu <= 1e-6 || hu <= 1e-6 {
                return None;
            }
            let po = (open / (lu * hu) * 100.0).min(100.0);
            let d = match given {
                Some(d) => Some(d),
                None => reader.distance(lu, hu, po, pv, storeys),
            }?;
            Some(Group {
                members: group.to_vec(),
                rect: r,
                lu,
                hu,
                po,
                d,
                single: false,
            })
        };
        let all_of: Vec<usize> = (0..openings.len()).collect();
        let Some(whole) = combined(&all_of) else {
            continue;
        };
        // 3.2.4 (Z3): with po under the limit, d for each opening off
        // table 4; those closer than 0,6 × (d1 + d2) are taken together
        // by 5.3.1, until no two groups are that close
        let mut groups = vec![whole];
        if let (None, Some(limit), Some(k), Some(t4)) =
            (given, reader.single_po, reader.single_gap, reader.single)
        {
            if groups[0].po < limit {
                let load = pv.unwrap_or(0.0);
                let singles: Option<Vec<Group>> = all_of
                    .iter()
                    .map(|&i| {
                        let r = rect(openings[i]);
                        let (l, h) = (r[1] - r[0], r[3] - r[2]);
                        match single(t4, l, h, load) {
                            Some(d) => Some(Group {
                                members: vec![i],
                                rect: r,
                                lu: l,
                                hu: h,
                                po: 100.0,
                                d,
                                single: true,
                            }),
                            None => combined(&[i]),
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
        for g in groups {
            if g.d <= 1e-3 {
                continue;
            }
            let members: Vec<&Source> = g.members.iter().map(|&i| openings[i]).collect();
            let Some(first) = members.iter().find(|o| o.level == p.level) else {
                continue;
            };
            if only.is_some_and(|id| !members.iter().any(|o| o.id == id)) {
                continue;
            }
            let (e0, e1) = (p.origin + p.t * g.rect[0], p.origin + p.t * g.rect[1]);
            let mut shared: Vec<String> = members
                .iter()
                .filter(|o| o.level != p.level)
                .filter_map(|o| model.get(o.level).map(|l| tr(&l.name)))
                .collect();
            shared.sort();
            shared.dedup();
            out.push(Pnp {
                opening: only.unwrap_or(first.id),
                outline: norm_outline(e0, e1, p.outward, g.d, reader.side_angle),
                d: g.d,
                at: (e0 + e1) / 2.0,
                outward: p.outward,
                shared,
                lu: g.lu,
                hu: g.hu,
                po: g.po,
                pv: (reader.table == 3 || g.single).then_some(pv).flatten(),
            });
        }
    }
    out
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
    cells: Option<&'a ParamTable>,
    /// 3.2.4: table 4, the po under which it is used and the share of
    /// the sum of two distances neighbours must be apart.
    single: Option<&'a ParamTable>,
    single_po: Option<f64>,
    single_gap: Option<f64>,
    /// 2.6.4 f): the length and height a single area is under to have
    /// no space, and how near a neighbour makes it not single.
    small: Option<f64>,
    small_gap: Option<f64>,
}

impl<'a> Reader<'a> {
    fn new(model: &'a Model, t: Option<&'a ParamTable>) -> Self {
        let table = value(t, "table").map_or(3, |v| v.round() as u32);
        let cells = match table {
            6 => crate::firerisk::table(model, FIRE_DISTANCE_DWELLING),
            7 => crate::firerisk::table(model, FIRE_DISTANCE_LODGING),
            _ => None,
        };
        Self {
            table,
            d3: value(t, "d3").is_some_and(|v| v > 0.5),
            side_angle: value(t, "side_angle").unwrap_or(160.0),
            factors: (
                value(t, "two_storey").unwrap_or(1.0),
                value(t, "three_storey").unwrap_or(1.0),
            ),
            d: t.and(crate::firerisk::table(model, FIRE_DISTANCE_D)),
            cells: t.and(cells),
            single: t.and(crate::firerisk::table(model, FIRE_DISTANCE_SINGLE)),
            single_po: value(t, "single_po"),
            single_gap: value(t, "single_gap"),
            small: value(t, "small"),
            small_gap: value(t, "small_gap"),
        }
    }

    /// d for an area lu × hu with po % open, of a zone with `pv` and
    /// `storeys` storeys; none without the table it needs.
    fn distance(&self, lu: f64, hu: f64, po: f64, pv: Option<f64>, storeys: usize) -> Option<f64> {
        match self.table {
            6 | 7 => {
                let d = dwelling(self.cells?, lu, po, self.d3)?;
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
}
