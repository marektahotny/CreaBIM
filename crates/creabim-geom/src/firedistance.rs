//! The fire-dangerous space (požiarne nebezpečný priestor, PNP) in front
//! of an opening in an outer wall and its separation distance d, drawn
//! in a plan as a dashed red outline with "PNP d = x,xx m".
//!
//! The space is where the heat radiated by the burning opening is over
//! the critical heat flux. The opening radiates as a flat surface at the
//! temperature of the standard fire curve (ISO 834) at the time the
//! design fire load pv of its fire zone gives; what reaches a point is
//! that times the view factor of the opening seen from it, turned
//! towards it (the irradiance vector of the radiating surfaces, by
//! Lambert's contour integral). Openings of other storeys right over or
//! under it, in the same face, radiate with it: its space is shared
//! with them.
//!
//! The values that belong to the norm (the critical heat flux, the
//! emissivity, the time a kilogram of design load stands for and the
//! design load of an opening whose room has no fire zone) are read from
//! the designer's parameter table [`FIRE_DISTANCE`]; with none, an
//! opening still draws its space when its side of the building has a
//! separation distance given in the project information, its outline
//! then set to reach that distance.

use crate::annotation::{Annotation, Shape, TextLook};
use crate::plan::PlanItem;
use creabim_core::fireparams::{self, ParamTable};
use creabim_core::words::tr;
use creabim_core::{ElementId, ElementKind, Model, OpeningKind};
use glam::{DVec2, DVec3};

/// The parameter table of the separation distances, single values by
/// key: `i_crit` (kW/m²), `emissivity`, `minutes_per_pv` (min per
/// kg/m²) and `pv_default` (kg/m²).
pub const FIRE_DISTANCE: &str = "fire_distance";

/// The colour the space is drawn in.
pub const PNP_RED: [f32; 3] = crate::firezones::DANGER;

/// Stefan-Boltzmann constant, kW/(m²·K⁴).
const SIGMA: f64 = 5.670_374e-11;

/// An opening's fire-dangerous space.
#[derive(Clone, Debug, PartialEq)]
pub struct Pnp {
    pub opening: ElementId,
    /// Its outline in the plan, from one edge of the opening (in the
    /// wall's outer face) round to the other.
    pub outline: Vec<DVec2>,
    /// The separation distance: how far it reaches out of the face, m.
    pub d: f64,
    /// The middle of the opening in the wall's face and the way out.
    pub at: DVec2,
    pub outward: DVec2,
    /// The storeys whose openings radiate with it, by name.
    pub shared: Vec<String>,
    /// The design fire load it is worked out from, kg/m²; none where the
    /// distance given for the side of the building is drawn.
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
    outward: DVec2,
    facing: usize,
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
            outward: w.outward,
            facing: w.facing,
            inside: mid - w.outward * (wall.thickness + 0.15),
        });
    }
    out
}

/// The irradiance vector at `p` of a uniform surface bounded by `poly`
/// radiating one unit: its length is the view factor of the surface
/// from a small area at `p` turned straight at it.
fn irradiance(p: DVec3, poly: &[DVec3; 4]) -> DVec3 {
    let mut e = DVec3::ZERO;
    for i in 0..4 {
        let (a, b) = (poly[i] - p, poly[(i + 1) % 4] - p);
        let c = a.cross(b);
        let len = c.length();
        if len < 1e-12 {
            continue;
        }
        e += c / len * a.angle_between(b);
    }
    e / (2.0 * std::f64::consts::PI)
}

/// What of a group of openings a point in the plan sees: the greatest
/// view factor at any of the openings' heights.
struct Group {
    polys: Vec<[DVec3; 4]>,
    heights: Vec<f64>,
}

impl Group {
    fn new(src: &[&Source]) -> Self {
        let mut heights = vec![];
        let polys = src
            .iter()
            .map(|s| {
                heights.push((s.z0 + s.z1) / 2.0);
                let (a, b) = (s.a, s.b);
                [
                    a.extend(s.z0),
                    b.extend(s.z0),
                    b.extend(s.z1),
                    a.extend(s.z1),
                ]
            })
            .collect();
        Self { polys, heights }
    }

    fn view(&self, p: DVec2) -> f64 {
        self.heights
            .iter()
            .map(|z| {
                let q = p.extend(*z);
                self.polys
                    .iter()
                    .fold(DVec3::ZERO, |e, poly| e + irradiance(q, poly))
                    .length()
            })
            .fold(0.0, f64::max)
    }
}

/// The temperature of the standard fire curve (ISO 834) after `t`
/// minutes, °C.
pub fn fire_curve(t: f64) -> f64 {
    345.0 * (8.0 * t.max(0.0) + 1.0).log10() + 20.0
}

/// The heat flux a surface at `celsius` radiates, kW/m².
pub fn radiated(celsius: f64, emissivity: f64) -> f64 {
    emissivity * SIGMA * (celsius + 273.15).powi(4)
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

/// The fire-dangerous spaces of the openings on `level` (every level
/// with none), or of the one opening `only`.
pub fn spaces(model: &Model, level: Option<ElementId>, only: Option<ElementId>) -> Vec<Pnp> {
    let all = sources(model);
    if all.is_empty() {
        return vec![];
    }
    let table = crate::firerisk::table(model, FIRE_DISTANCE);
    let crit = value(table, "i_crit").filter(|v| *v > 0.0);
    let emissivity = value(table, "emissivity").unwrap_or(1.0);
    let per_pv = value(table, "minutes_per_pv").unwrap_or(1.0);
    let loads = table.map(|t| Loads::new(model, Some(t)));
    let mut out = vec![];
    for s in &all {
        if level.is_some_and(|l| l != s.level) || only.is_some_and(|o| o != s.id) {
            continue;
        }
        // the openings of other storeys over or under it in its face
        let t = (s.b - s.a).normalize_or_zero();
        let (s0, s1) = (0.0, s.a.distance(s.b));
        let mut group = vec![s];
        for o in &all {
            if o.level == s.level
                || o.outward.dot(s.outward) < 0.99
                || (o.a - s.a).dot(s.outward).abs() > 0.05
            {
                continue;
            }
            let (u0, u1) = {
                let (x, y) = ((o.a - s.a).dot(t), (o.b - s.a).dot(t));
                (x.min(y), x.max(y))
            };
            if u1 > s0 + 1e-3 && u0 < s1 - 1e-3 {
                group.push(o);
            }
        }
        let g = Group::new(&group);
        let at = (s.a + s.b) / 2.0;
        let given = crate::firezones::distance_of(model, s.facing);
        // the share of the view factor where the flux falls to the
        // critical one
        let (limit, pv) = match (&loads, crit) {
            (Some(loads), Some(crit)) => {
                let mut pv: Option<f64> = None;
                let mut none = false;
                for o in &group {
                    match loads.of(model, o) {
                        Ok(Some(v)) => pv = Some(pv.map_or(v, |p| p.max(v))),
                        Ok(None) => {}
                        Err(()) => none |= o.id == s.id,
                    }
                }
                if none {
                    continue;
                }
                match pv {
                    Some(pv) => (
                        Some(crit / radiated(fire_curve(pv * per_pv), emissivity)),
                        Some(pv),
                    ),
                    None => (given.map(|d| g.view(at + s.outward * d)), None),
                }
            }
            _ => (given.map(|d| g.view(at + s.outward * d)), None),
        };
        let Some(limit) = limit.filter(|l| *l > 1e-6) else {
            continue;
        };
        // a given distance is taken as it is; otherwise the farthest the
        // critical flux reaches
        let d = match (pv, given) {
            (None, Some(d)) => d,
            _ => contour(&g, s, at, limit).1,
        };
        // the radiating face: the opening and those of other storeys
        // over or under it, end to end along the wall
        let (lo, hi) = group.iter().fold((s0, s1), |(lo, hi), o| {
            let (x, y) = ((o.a - s.a).dot(t), (o.b - s.a).dot(t));
            (lo.min(x.min(y)), hi.max(x.max(y)))
        });
        let at = s.a + t * ((lo + hi) / 2.0);
        let outline = norm_outline(s.a + t * lo, s.a + t * hi, s.outward, d);
        let mut shared: Vec<String> = group[1..]
            .iter()
            .filter_map(|o| model.get(o.level).map(|l| tr(&l.name)))
            .collect();
        shared.sort();
        shared.dedup();
        out.push(Pnp {
            opening: s.id,
            outline,
            d,
            at,
            outward: s.outward,
            shared,
            pv,
        });
    }
    out
}

/// The space's outline as the norm draws it: from each edge of the
/// radiating face a straight line out to a third of the distance, then
/// round in front of it an arc of the ellipse about the face's middle
/// reaching `d` straight out and half the face plus `d` along the wall.
pub fn norm_outline(e0: DVec2, e1: DVec2, outward: DVec2, d: f64) -> Vec<DVec2> {
    let t = (e1 - e0).normalize_or_zero();
    let c = (e0 + e1) / 2.0;
    let (a, b) = (e0.distance(e1) / 2.0 + d, d);
    let start = (1.0f64 / 3.0).asin();
    let n = 48;
    let mut out = vec![e0];
    for i in 0..=n {
        let th = std::f64::consts::PI
            - start
            - (std::f64::consts::PI - 2.0 * start) * i as f64 / n as f64;
        out.push(c + t * (a * th.cos()) + outward * (b * th.sin()));
    }
    out.push(e1);
    out
}

/// The outline where the view of the group falls to `limit`, along rays
/// out of the middle of the opening, and how far it reaches.
fn contour(g: &Group, s: &Source, at: DVec2, limit: f64) -> (Vec<DVec2>, f64) {
    let t = (s.b - s.a).normalize_or_zero();
    let reach = |dir: DVec2| -> f64 {
        let mut lo = 1e-3;
        if g.view(at + dir * lo) < limit {
            return lo;
        }
        let mut hi = 0.5;
        while g.view(at + dir * hi) >= limit {
            lo = hi;
            hi *= 2.0;
            if hi > 500.0 {
                return hi;
            }
        }
        for _ in 0..30 {
            let m = (lo + hi) / 2.0;
            if g.view(at + dir * m) >= limit {
                lo = m;
            } else {
                hi = m;
            }
        }
        (lo + hi) / 2.0
    };
    let n = 72;
    let mut outline = vec![s.a];
    for i in 1..n {
        // from the side of a round to the side of b
        let ang = std::f64::consts::PI * (1.0 - i as f64 / n as f64);
        let dir = t * ang.cos() + s.outward * ang.sin();
        outline.push(at + dir * reach(dir));
    }
    outline.push(s.b);
    let d = outline
        .iter()
        .map(|p| (*p - at).dot(s.outward))
        .fold(0.0, f64::max);
    (outline, d)
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

    fn with_table(m: &mut Model, rows: &str) {
        let mut info = m.project_info().map(|p| p.1.clone()).unwrap_or_default();
        let text = format!("#table fire_distance\n#columns key\tvalue\tnote\n{rows}");
        fireparams::merge(&mut info.fire_params, fireparams::parse(&text));
        let tx = creabim_core::set_project(m, info);
        tx.apply(m).unwrap();
    }

    /// Straight in front of a square the view factor is the textbook
    /// one: four corner rectangles of a/x each side.
    #[test]
    fn the_view_factor_in_front_of_a_square_is_the_textbook_one() {
        let poly = [
            DVec3::new(-1.0, 0.0, -1.0),
            DVec3::new(1.0, 0.0, -1.0),
            DVec3::new(1.0, 0.0, 1.0),
            DVec3::new(-1.0, 0.0, 1.0),
        ];
        let x: f64 = 2.0;
        let (a, b) = (1.0 / x, 1.0 / x);
        let corner = 1.0 / (2.0 * std::f64::consts::PI)
            * (a / (1.0 + a * a).sqrt() * (b / (1.0 + a * a).sqrt()).atan()
                + b / (1.0 + b * b).sqrt() * (a / (1.0 + b * b).sqrt()).atan());
        let f = irradiance(DVec3::new(0.0, -x, 0.0), &poly).length();
        assert!((f - 4.0 * corner).abs() < 1e-9, "{f} {}", 4.0 * corner);
    }

    #[test]
    fn nothing_is_drawn_without_a_table_or_a_given_distance() {
        let m = creabim_core::starter::starter_house();
        assert!(spaces(&m, None, None).is_empty());
    }

    /// With the table every opening of the outer walls gets a space that
    /// starts at its edges and bulges out of the face; one over another
    /// shares it.
    #[test]
    fn every_opening_gets_a_space_out_of_its_face() {
        let mut m = creabim_core::starter::starter_house();
        with_table(
            &mut m,
            "i_crit\t18,5\t\nemissivity\t1,0\t\nminutes_per_pv\t1\t\npv_default\t50\t\n",
        );
        let all = spaces(&m, None, None);
        assert!(!all.is_empty());
        for p in &all {
            assert!(p.d > 0.3 && p.d < 20.0, "{}", p.d);
            assert_eq!(p.pv, Some(50.0));
            // nothing of it lies behind the face
            assert!(p.outline.iter().all(|q| (*q - p.at).dot(p.outward) > -1e-9));
            let far = p
                .outline
                .iter()
                .map(|q| (*q - p.at).dot(p.outward))
                .fold(0.0, f64::max);
            assert!((far - p.d).abs() < 1e-9);
        }
        // a bigger load reaches farther
        with_table(&mut m, "i_crit\t18,5\t\npv_default\t90\t\n");
        let far = spaces(&m, None, None);
        assert!(far.iter().zip(&all).all(|(a, b)| a.d > b.d));
        let items = items(&m, m_level(&m, &all[0]), Some(all[0].opening), 0.1);
        assert_eq!(items.len(), 2);
        assert!(
            matches!(&items[1], PlanItem::Annotation { item, .. } if item.text.starts_with("PNP d = "))
        );
    }

    fn m_level(m: &Model, p: &Pnp) -> ElementId {
        let Some(ElementKind::Opening(o)) = m.get(p.opening).map(|e| &e.kind) else {
            panic!()
        };
        match m.get(o.host).map(|e| &e.kind) {
            Some(ElementKind::Wall(w)) => w.level,
            _ => panic!(),
        }
    }

    /// A distance given for the side of the building sets the space's
    /// reach when there is no table.
    #[test]
    fn a_given_distance_sets_the_reach() {
        let mut m = creabim_core::starter::starter_house();
        let mut info = m.project_info().map(|p| p.1.clone()).unwrap_or_default();
        info.fire_distances = vec![3.5; 8];
        let tx = creabim_core::set_project(&m, info);
        tx.apply(&mut m).unwrap();
        let all = spaces(&m, None, None);
        assert!(!all.is_empty());
        for p in &all {
            assert!((p.d - 3.5).abs() < 0.02, "{}", p.d);
            assert_eq!(p.pv, None);
        }
    }

    /// The norm's outline of a 1,2 m window with d = 1,83 m: straight
    /// out of its edges to a third of d some 1,7 m aside, round to d in
    /// front of it.
    #[test]
    fn the_outline_runs_straight_out_of_the_edges_then_round() {
        let o = norm_outline(DVec2::ZERO, DVec2::new(1.2, 0.0), DVec2::Y, 1.83);
        assert_eq!(
            (o[0], *o.last().unwrap()),
            (DVec2::ZERO, DVec2::new(1.2, 0.0))
        );
        assert!((o[1].y - 1.83 / 3.0).abs() < 1e-9);
        assert!((-o[1].x - 1.69).abs() < 0.01, "{}", o[1].x);
        let far = o.iter().map(|p| p.y).fold(0.0, f64::max);
        assert!((far - 1.83).abs() < 1e-9);
    }

    #[test]
    fn the_fire_curve_and_the_flux() {
        assert!((fire_curve(0.0) - 20.0).abs() < 1e-9);
        assert!((fire_curve(60.0) - 945.3).abs() < 0.5);
        // a black body at 1000 K
        assert!((radiated(1000.0 - 273.15, 1.0) - 56.70374).abs() < 1e-6);
    }
}
