//! The heated envelope of the building for an energy assessment, and its
//! export as a project file of ISOVER Fragment "Projektové hodnotenie"
//! (fragment.isover.sk), the web application Slovak designers make the
//! projektové energetické hodnotenie in. The file is the application's
//! own: a dump of its form, each field `{type, value}` under its section
//! (see `docs/fragment-export.md` for every field and where it comes
//! from). Fragment computes the assessment itself; CreaBIM gives it what
//! the model knows: the surfaces of the envelope with their build-ups'
//! U, the windows and doors in them with their Uw, the heated floor areas
//! and volumes, the building's size and use.
//!
//! What is heated: every room but a garage and the open-air ones
//! (loggias, terraces, balconies, porches). The envelope, by Fragment's kinds
//! (STN 73 0540-2's rows, see [`SurfaceKind`]):
//! - the outer walls along heated rooms, one row each wall and storey,
//!   on their outer face from corner to corner and storey high (floor to
//!   floor, the top storey its own height), facing their way; where the
//!   outside ground is above the storey's floor, the part under it is a
//!   wall against the ground by how deep the ground reaches;
//! - the walls between heated and unheated rooms;
//! - under heated rooms with nothing heated under them: the slab they
//!   stand on, by its outline where only heated rooms stand on it (else
//!   the rooms' areas): on the ground on the lowest storey, split into
//!   the strip within 2,0 m of the outer walls' inner face and the rest;
//!   over an unheated room; over the outside;
//! - over heated rooms with nothing heated over them: the slab of the
//!   storey above (under an unheated attic or room), else the roof.
//!
//! The outside ground is the lowest point of the terrain at the building
//! (else ±0,000). The reduction factors bx stay 1: Fragment's own table
//! of them is set there.
//!
//! The location is not set: Fragment keeps its municipalities under
//! codes of its own, and choosing the municipality there fills the
//! climate (temperature region, degree days, monthly temperatures).

use creabim_core::assembly::assembly_of;
use creabim_core::roomtype::RoomType;
use creabim_core::{levels, polygon, room, thermal, ElementId, ElementKind, Model, OpeningKind};
use creabim_services::heatloss;
use glam::DVec2;
use serde_json::{json, Value};

/// What kind of surface a row of Fragment's "Skladba plôch" is: its
/// "Druh" list, the rows of STN 73 0540-2's table of required U.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceKind {
    /// VSS 1, an outer wall (and a roof steeper than 45°).
    OuterWall,
    /// PSS 2, a flat or pitched roof up to 45°.
    Roof,
    /// SVP 3, a ceiling over the outside (a floor over a passage).
    OverOutside,
    /// SNP 4, a ceiling under an unheated space (an attic).
    UnderUnheated,
    /// VSH 5, an inner wall, heat flowing across (to a garage).
    InnerWall,
    /// VDH 6, an inner ceiling, heat flowing up (an unheated room over).
    InnerUp,
    /// VHD 7, an inner floor, heat flowing down (an unheated room under).
    InnerDown,
    /// SZ1 8, SZ2 9, SZ3 10: a wall against the ground no deeper than
    /// 0,5 m, 0,5 to 2,0 m, deeper.
    GroundWall1,
    GroundWall2,
    GroundWall3,
    /// PV1 11: a floor on the ground at most 0,5 m under the outside
    /// ground and within 2,0 m of the outer walls' inner face.
    FloorEdge,
    /// PVT 12: the rest of a floor on the ground.
    FloorOther,
}

impl SurfaceKind {
    /// Fragment's code, which is also the place in its list.
    pub fn code(self) -> i64 {
        use SurfaceKind::*;
        match self {
            OuterWall => 1,
            Roof => 2,
            OverOutside => 3,
            UnderUnheated => 4,
            InnerWall => 5,
            InnerUp => 6,
            InnerDown => 7,
            GroundWall1 => 8,
            GroundWall2 => 9,
            GroundWall3 => 10,
            FloorEdge => 11,
            FloorOther => 12,
        }
    }

    /// Its abbreviation in Fragment.
    pub fn abbr(self) -> &'static str {
        [
            "VSS", "PSS", "SVP", "SNP", "VSH", "VDH", "VHD", "SZ1", "SZ2", "SZ3", "PV1", "PVT",
        ][(self.code() - 1) as usize]
    }

    pub fn name(self) -> &'static str {
        use SurfaceKind::*;
        match self {
            OuterWall => "obvodová stena",
            Roof => "strecha",
            OverOutside => "strop nad exteriérom",
            UnderUnheated => "strop pod nevykurovaným priestorom",
            InnerWall => "stena k nevykurovanému priestoru",
            InnerUp => "strop k nevykurovanému priestoru",
            InnerDown => "podlaha nad nevykurovaným priestorom",
            GroundWall1 | GroundWall2 | GroundWall3 => "stena k zemine",
            FloorEdge => "podlaha na teréne do 2 m od steny",
            FloorOther => "podlaha na teréne",
        }
    }

    /// A wall to the ground by how deep the ground reaches, m.
    pub fn ground_wall(depth: f64) -> SurfaceKind {
        if depth <= 0.5 {
            SurfaceKind::GroundWall1
        } else if depth <= 2.0 {
            SurfaceKind::GroundWall2
        } else {
            SurfaceKind::GroundWall3
        }
    }

    pub fn horizontal(self) -> bool {
        use SurfaceKind::*;
        !matches!(
            self,
            OuterWall | InnerWall | GroundWall1 | GroundWall2 | GroundWall3
        )
    }

    /// Part of the heat-losing envelope (Fragment's "obal").
    pub fn envelope(self) -> bool {
        true
    }
}

/// What fills an opening: Fragment's "Druh otvoru".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ApertureKind {
    /// OK, a window.
    Window,
    /// PD, a glazed door.
    GlazedDoor,
    /// DV, a solid door.
    Door,
}

impl ApertureKind {
    /// Fragment's code: OK 1, SO 2, PD 3, DV 4, PR 5, PS 6, in the order
    /// of its list (the sample holds OK as 1 and PD as 3).
    pub fn code(self) -> i64 {
        match self {
            ApertureKind::Window => 1,
            ApertureKind::GlazedDoor => 3,
            ApertureKind::Door => 4,
        }
    }
}

/// A window or a door in a surface.
#[derive(Clone, Debug, PartialEq)]
pub struct Aperture {
    pub kind: ApertureKind,
    pub name: String,
    pub width: f64,
    pub height: f64,
    /// Where its lower left corner is on the surface seen from outside,
    /// m from the surface's left end and its bottom.
    pub x: f64,
    pub y: f64,
    /// Uw from its maker's data sheet; none where it has none.
    pub uw: Option<f64>,
}

impl Aperture {
    pub fn area(&self) -> f64 {
        self.width * self.height
    }
}

/// One surface of the envelope.
#[derive(Clone, Debug, PartialEq)]
pub struct Surface {
    pub kind: SurfaceKind,
    pub name: String,
    /// The storey it belongs to.
    pub storey: String,
    /// Length and height (a wall), or the two sides of the rectangle of
    /// the same area (a floor, a ceiling, a roof), m.
    pub length: f64,
    pub width: f64,
    /// Its area with the openings, m².
    pub gross: f64,
    /// Where it faces, degrees clockwise from north; none for the
    /// horizontal ones.
    pub azimuth: Option<f64>,
    /// U of its build-up, W/(m²K); none where a layer lacks λ.
    pub u: Option<f64>,
    /// R of its layers, m²K/W.
    pub r: Option<f64>,
    /// Its thickness, mm.
    pub thickness: f64,
    /// The build-up's name and its layers in words.
    pub assembly: String,
    pub layers: String,
    pub openings: Vec<Aperture>,
}

impl Surface {
    /// Its area less its openings', m².
    pub fn net(&self) -> f64 {
        (self.gross - self.openings.iter().map(Aperture::area).sum::<f64>()).max(0.0)
    }
}

/// A heated storey: its rooms' floor area and volume.
#[derive(Clone, Debug, PartialEq)]
pub struct HeatedStorey {
    pub name: String,
    pub area: f64,
    pub height: f64,
    pub volume: f64,
}

/// The envelope and what Fragment asks of the building.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Envelope {
    pub surfaces: Vec<Surface>,
    pub storeys: Vec<HeatedStorey>,
    /// The building's size west–east, south–north and its height, m.
    pub size: [f64; 3],
    /// What the export could not tell, in Slovak, for the designer.
    pub warnings: Vec<String>,
}

/// A room heated in winter: not a garage and not in the open air.
fn heated(name: &str, r: &creabim_core::Room) -> bool {
    let porch = ["porch", "zápraž", "zapraz"]
        .iter()
        .any(|w| name.to_lowercase().contains(w) || r.occupancy.to_lowercase().contains(w));
    RoomType::of(&r.occupancy, name) != RoomType::Garage && !room::is_open_air(name, r) && !porch
}

struct RoomAt {
    level: ElementId,
    elevation: f64,
    outline: Vec<DVec2>,
    at: DVec2,
    area: f64,
    height: f64,
    heated: bool,
}

/// The heated envelope of the model.
pub fn envelope(model: &Model) -> Envelope {
    let mut out = Envelope::default();
    let storeys = levels::sorted(model);
    let all: Vec<RoomAt> = model
        .iter()
        .filter_map(|e| {
            let ElementKind::Room(r) = &e.kind else {
                return None;
            };
            let s = room::shape(model, e.id).filter(|s| s.outline.len() > 2)?;
            Some(RoomAt {
                level: r.level,
                elevation: model.level(r.level).map_or(0.0, |l| l.elevation),
                area: room::net_area(model, r, &s),
                height: room::height(model, r),
                at: r.at,
                outline: s.outline,
                heated: heated(&e.name, r),
            })
        })
        .collect();
    if !all.iter().any(|r| r.heated) {
        out.warnings
            .push("V modeli nie sú vykurované miestnosti: obálka je prázdna.".into());
        return out;
    }
    let north = model.project_info().map_or(0.0, |(_, p)| p.true_north);
    // the outside ground at the building: its lowest point, else ±0.000
    let ground = crate::summary::ground_at_building(model).map_or(0.0, |g| g.0);
    let room_at = |level: ElementId, p: DVec2| {
        all.iter()
            .find(|r| r.level == level && polygon::contains(&r.outline, p))
    };
    let mut corners: Vec<DVec2> = vec![];
    let mut top = f64::MIN;
    let mut bottom = f64::MAX;
    for (k, (level, lname, l)) in storeys.iter().enumerate() {
        let here: Vec<&RoomAt> = all
            .iter()
            .filter(|r| r.level == *level && r.heated)
            .collect();
        if here.is_empty() {
            continue;
        }
        let storey_h = storeys
            .get(k + 1)
            .map(|n| n.2.elevation - l.elevation)
            .filter(|h| *h > 0.1)
            .unwrap_or(l.height);
        bottom = bottom.min(l.elevation);
        top = top.max(l.elevation + storey_h);
        let area: f64 = here.iter().map(|r| r.area).sum();
        let volume: f64 = here.iter().map(|r| r.area * r.height).sum();
        out.storeys.push(HeatedStorey {
            name: lname.clone(),
            area,
            height: if area > 0.0 { volume / area } else { 0.0 },
            volume,
        });
        // the outer walls along heated rooms, on their outer face; the
        // ground against the part of them under it
        let faces = outer_faces(model, *level, &|p| room_at(*level, p).map(|r| r.heated));
        let soil = (ground - l.elevation).min(storey_h);
        for f in &faces {
            corners.extend([f.a, f.b]);
            let e = model.get(f.wall).expect("a wall of the model");
            let mut s = surface(model, e, SurfaceKind::OuterWall, lname);
            s.length = round3(f.a.distance(f.b));
            s.azimuth = Some(heatloss::azimuth(f.out, north));
            s.openings = apertures(model, f.wall, f.a, f.b);
            if soil > 0.05 {
                let mut g = s.clone();
                g.kind = SurfaceKind::ground_wall(soil);
                g.width = round3(soil);
                g.gross = g.length * soil;
                g.openings.clear();
                out.surfaces.push(g);
                s.width = round3(storey_h - soil);
                for o in &mut s.openings {
                    o.y = (o.y - soil).max(0.0);
                }
            } else {
                s.width = round3(storey_h);
            }
            s.gross = s.length * s.width;
            if s.width > 0.05 {
                out.surfaces.push(s);
            }
        }
        // the walls between heated and unheated rooms (a garage)
        let outer: Vec<ElementId> = faces.iter().map(|f| f.wall).collect();
        for e in model.iter() {
            let ElementKind::Wall(w) = &e.kind else {
                continue;
            };
            if w.level != *level || outer.contains(&e.id) || w.is_arc() {
                continue;
            }
            let d = (w.end - w.start).normalize_or_zero();
            let mid = (w.start + w.end) / 2.0;
            let side = |s: f64| room_at(*level, mid + d.perp() * s * (w.thickness / 2.0 + 0.1));
            let toward = match (side(1.0), side(-1.0)) {
                (Some(a), Some(b)) if a.heated && !b.heated => -d.perp(),
                (Some(a), Some(b)) if !a.heated && b.heated => d.perp(),
                _ => continue,
            };
            let mut s = surface(model, e, SurfaceKind::InnerWall, lname);
            s.length = round3(w.start.distance(w.end));
            s.width = round3(storey_h);
            s.gross = s.length * s.width;
            s.azimuth = Some(heatloss::azimuth(toward, north));
            s.openings = apertures(model, e.id, w.start, w.end);
            out.surfaces.push(s);
        }
    }
    // the floors under and the ceilings or roofs over heated rooms
    let elevation = |id: ElementId| model.level(id).map_or(0.0, |l| l.elevation);
    let lowest = storeys
        .iter()
        .filter(|s| all.iter().any(|r| r.level == s.0))
        .map(|s| s.2.elevation)
        .fold(f64::MAX, f64::min);
    let slab_at = |level: ElementId, p: DVec2| {
        model.iter().find(|e| {
            matches!(&e.kind, ElementKind::Slab(s)
                if s.level == level && s.boundary.len() > 2 && polygon::contains(&s.boundary, p))
        })
    };
    let mut groups: Vec<(ElementId, SurfaceKind, Vec<usize>)> = vec![];
    let mut push = |id: ElementId, kind: SurfaceKind, i: usize| match groups
        .iter_mut()
        .find(|g| g.0 == id && g.1 == kind)
    {
        Some(g) => g.2.push(i),
        None => groups.push((id, kind, vec![i])),
    };
    let mut nothing_over = 0;
    let mut nothing_under = 0;
    for (i, r) in all.iter().enumerate() {
        if !r.heated {
            continue;
        }
        // the nearest room under it and over it
        let under = all
            .iter()
            .filter(|x| x.elevation < r.elevation - 1e-6 && polygon::contains(&x.outline, r.at))
            .max_by(|a, b| a.elevation.total_cmp(&b.elevation));
        let over = all
            .iter()
            .filter(|x| x.elevation > r.elevation + 1e-6 && polygon::contains(&x.outline, r.at))
            .min_by(|a, b| a.elevation.total_cmp(&b.elevation));
        let below_kind = match under {
            Some(x) if x.heated => None,
            Some(_) => Some(SurfaceKind::InnerDown),
            None if r.elevation <= lowest + 1e-6 => Some(SurfaceKind::FloorOther),
            None => Some(SurfaceKind::OverOutside),
        };
        if let Some(kind) = below_kind {
            match slab_at(r.level, r.at) {
                Some(s) => push(s.id, kind, i),
                None => nothing_under += 1,
            }
        }
        if over.is_some_and(|x| x.heated) {
            continue;
        }
        // the slab of the next storey up, else a roof
        let next = storeys
            .iter()
            .map(|s| s.0)
            .filter(|id| elevation(*id) > r.elevation + 1e-6)
            .min_by(|a, b| elevation(*a).total_cmp(&elevation(*b)));
        let ceiling = next.and_then(|n| slab_at(n, r.at));
        let kind = if over.is_some() {
            SurfaceKind::InnerUp
        } else {
            SurfaceKind::UnderUnheated
        };
        match ceiling {
            Some(c) => push(c.id, kind, i),
            None => {
                let roof = model.iter().find(|e| {
                    matches!(&e.kind, ElementKind::Roof(rf)
                        if rf.boundary.len() > 2 && polygon::contains(&rf.boundary, r.at))
                });
                match roof {
                    Some(e) => push(e.id, SurfaceKind::Roof, i),
                    None => nothing_over += 1,
                }
            }
        }
    }
    if nothing_over > 0 {
        out.warnings.push(format!(
            "Nad {nothing_over} vykurovanými miestnosťami nie je strop ani strecha: chýbajú v obálke."
        ));
    }
    if nothing_under > 0 {
        out.warnings.push(format!(
            "Pod {nothing_under} vykurovanými miestnosťami nie je doska: chýbajú v obálke."
        ));
    }
    for (id, kind, of) in &groups {
        let e = model.get(*id).expect("an element of the model");
        let level = all[of[0]].level;
        let storey = model.get(level).map_or(String::new(), |l| l.name.clone());
        // the element's outline where only these rooms are on it, else
        // the rooms' own outlines
        let (outline, holes): (Vec<DVec2>, Vec<Vec<DVec2>>) = match &e.kind {
            ElementKind::Slab(s) => (s.outline(), s.holes.clone()),
            ElementKind::Roof(r) => (r.boundary.clone(), vec![]),
            _ => continue,
        };
        let own: Vec<Vec<DVec2>> = of.iter().map(|i| all[*i].outline.clone()).collect();
        let others = covered(model, level, &outline, &own) < 0.95;
        let regions: Vec<Vec<DVec2>> = if others || matches!(e.kind, ElementKind::Roof(_)) {
            of.iter().map(|i| all[*i].outline.clone()).collect()
        } else {
            vec![outline.clone()]
        };
        let plan: f64 = if others || matches!(e.kind, ElementKind::Roof(_)) {
            of.iter().map(|i| all[*i].area).sum()
        } else {
            let h: f64 = holes
                .iter()
                .map(|h| polygon::signed_area2(h).abs() / 2.0)
                .sum();
            (polygon::signed_area2(&outline).abs() / 2.0 - h).max(0.0)
        };
        let mut kind = *kind;
        let mut slope = 1.0;
        if let ElementKind::Roof(rf) = &e.kind {
            let pitch = if matches!(rf.shape, creabim_core::RoofShape::Flat) {
                0.0
            } else {
                rf.pitch.clamp(0.0, 80.0)
            };
            slope = 1.0 / pitch.to_radians().cos();
            if pitch > 45.0 {
                kind = SurfaceKind::OuterWall;
            }
        }
        let parts: Vec<(SurfaceKind, f64)> = if kind == SurfaceKind::FloorOther {
            // within 2 m of the outer walls' inner face, and no deeper
            // than 0,5 m under the outside ground: PV1; the rest PVT
            let depth = ground - elevation(level);
            let edge = if depth > 0.5 {
                0.0
            } else {
                near_walls(model, level, &regions, 2.0)
            };
            vec![
                (SurfaceKind::FloorEdge, plan * edge),
                (SurfaceKind::FloorOther, plan * (1.0 - edge)),
            ]
        } else {
            vec![(kind, plan * slope)]
        };
        for (kind, area) in parts {
            if area < 0.01 {
                continue;
            }
            let mut s = surface(model, e, kind, &storey);
            let (l, w) = rectangle(&outline, area);
            s.length = l;
            s.width = w;
            s.gross = area;
            out.surfaces.push(s);
        }
    }
    // how big the building is, along true north
    let t = north.to_radians();
    let (n, east) = (DVec2::new(-t.sin(), t.cos()), DVec2::new(t.cos(), t.sin()));
    let span = |axis: DVec2| {
        let (lo, hi) = corners.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
            (lo.min(p.dot(axis)), hi.max(p.dot(axis)))
        });
        (hi - lo).max(0.0)
    };
    let height = crate::summary::figures(model).height.max(top) - bottom.min(top);
    out.size = [round3(span(east)), round3(span(n)), round3(height.max(0.0))];
    for s in &out.surfaces {
        if s.u.is_none() {
            out.warnings.push(format!(
                "{} „{}“ ({}): skladbe chýba λ niektorej vrstvy, U nie je známe.",
                s.kind.abbr(),
                s.name,
                s.storey
            ));
        }
        for o in &s.openings {
            if o.uw.is_none() {
                out.warnings.push(format!(
                    "„{}“ v stene „{}“: chýba Uw (vlastnosti okna/dverí).",
                    o.name, s.name
                ));
            }
        }
    }
    if out.surfaces.iter().any(|s| {
        !matches!(
            s.kind,
            SurfaceKind::OuterWall | SurfaceKind::Roof | SurfaceKind::OverOutside
        )
    }) {
        out.warnings.push(
            "Plochy k zemine a k nevykurovaným priestorom majú bx = 1: nastav ich vo Fragmente (redukčné faktory bx).".into(),
        );
    }
    out
}

/// A surface of element `e` with its build-up's U and layers; its size
/// and openings are the caller's.
fn surface(model: &Model, e: &creabim_core::Element, kind: SurfaceKind, storey: &str) -> Surface {
    let th = thermal::of_element(model, e);
    let (assembly, layers, thickness) = build_up(model, e, th.as_ref());
    Surface {
        kind,
        name: e.name.clone(),
        storey: storey.to_string(),
        length: 0.0,
        width: 0.0,
        gross: 0.0,
        azimuth: None,
        u: th.as_ref().and_then(|t| t.u),
        r: th.as_ref().map(|t| t.r),
        thickness,
        assembly,
        layers,
        openings: vec![],
    }
}

/// The windows and doors in wall `wall`, placed along its face `a` → `b`
/// (left to right seen from outside).
fn apertures(model: &Model, wall: ElementId, a: DVec2, b: DVec2) -> Vec<Aperture> {
    let Some(ElementKind::Wall(w)) = model.get(wall).map(|e| &e.kind) else {
        return vec![];
    };
    let length = a.distance(b);
    let d = (w.end - w.start).normalize_or_zero();
    let along = |p: DVec2| (p - a).dot((b - a).normalize_or_zero());
    let mut out = vec![];
    for o in model.iter() {
        let ElementKind::Opening(op) = &o.kind else {
            continue;
        };
        if op.host != wall || op.kind == OpeningKind::Hole {
            continue;
        }
        let p0 = w.start + d * op.offset;
        let p1 = p0 + d * op.width;
        // only those in this face's stretch of the wall
        let mid = along((p0 + p1) / 2.0);
        if mid < 0.0 || mid > length {
            continue;
        }
        let perf = op.performance.as_ref();
        let kind = match op.kind {
            OpeningKind::Window => ApertureKind::Window,
            // a door that lets the sun in is glazed
            _ if perf.is_some_and(|p| p.g.is_some()) => ApertureKind::GlazedDoor,
            _ => ApertureKind::Door,
        };
        out.push(Aperture {
            kind,
            name: o.name.clone(),
            width: op.width,
            height: op.height,
            x: along(p0).min(along(p1)).clamp(0.0, length),
            y: op.sill.max(0.0),
            uw: perf.and_then(|p| p.uw),
        });
    }
    out
}

/// What share of `outline`, outside the walls of `level`, `rooms` cover.
fn covered(model: &Model, level: ElementId, outline: &[DVec2], rooms: &[Vec<DVec2>]) -> f64 {
    let walls: Vec<creabim_core::Wall> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Wall(w) if w.level == level => Some(w.clone()),
            _ => None,
        })
        .collect();
    let (mut all, mut hit) = (0usize, 0usize);
    grid(&[outline.to_vec()], |p| {
        if walls
            .iter()
            .any(|w| segment_distance(p, w.start, w.end) <= w.thickness / 2.0 + 0.02)
        {
            return;
        }
        all += 1;
        if rooms.iter().any(|r| polygon::contains(r, p)) {
            hit += 1;
        }
    });
    if all == 0 {
        0.0
    } else {
        hit as f64 / all as f64
    }
}

/// Each point of a grid over `regions` that lies in one of them.
fn grid(regions: &[Vec<DVec2>], mut f: impl FnMut(DVec2)) {
    let (lo, hi) = regions.iter().flatten().fold(
        (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN)),
        |(lo, hi), p| (lo.min(*p), hi.max(*p)),
    );
    if lo.x > hi.x {
        return;
    }
    let size = hi - lo;
    let step = ((size.x * size.y) / 40_000.0).sqrt().max(0.05);
    let mut y = lo.y + step / 2.0;
    while y < hi.y {
        let mut x = lo.x + step / 2.0;
        while x < hi.x {
            let p = DVec2::new(x, y);
            if regions.iter().any(|r| polygon::contains(r, p)) {
                f(p);
            }
            x += step;
        }
        y += step;
    }
}

/// What share of `regions` lies within `reach` m of the inner face of
/// the outer walls of `level` (under the walls counts too), on a grid.
fn near_walls(model: &Model, level: ElementId, regions: &[Vec<DVec2>], reach: f64) -> f64 {
    let walls: Vec<creabim_core::Wall> = crate::sitefoot::outer_walls(model, level)
        .into_iter()
        .filter_map(|id| match model.get(id).map(|e| &e.kind) {
            Some(ElementKind::Wall(w)) => Some(w.clone()),
            _ => None,
        })
        .collect();
    if walls.is_empty() {
        return 1.0;
    }
    let (mut all, mut near) = (0usize, 0usize);
    grid(regions, |p| {
        all += 1;
        let d = walls
            .iter()
            .map(|w| segment_distance(p, w.start, w.end) - w.thickness / 2.0)
            .fold(f64::MAX, f64::min);
        if d <= reach {
            near += 1;
        }
    });
    if all == 0 {
        1.0
    } else {
        near as f64 / all as f64
    }
}

fn segment_distance(p: DVec2, a: DVec2, b: DVec2) -> f64 {
    let ab = b - a;
    let t = if ab.length_squared() < 1e-12 {
        0.0
    } else {
        ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0)
    };
    p.distance(a + ab * t)
}

/// A rectangle of area `area` with the proportions of `outline`: its
/// longer side first (Fragment multiplies them back).
fn rectangle(outline: &[DVec2], area: f64) -> (f64, f64) {
    let (lo, hi) = outline.iter().fold(
        (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN)),
        |(lo, hi), p| (lo.min(*p), hi.max(*p)),
    );
    let long = (hi - lo).max_element();
    if outline.len() < 3 || long <= 1e-6 || area <= 0.0 {
        return (round3(area), 1.0);
    }
    let long = long.max(area.sqrt());
    (round3(long), round3(area / long))
}

/// (thickness)", the thickness in millimetres), and its thickness, mm.
fn build_up(
    model: &Model,
    e: &creabim_core::Element,
    th: Option<&thermal::Thermal>,
) -> (String, String, f64) {
    let name = assembly_of(model, e)
        .map(|(a, _)| a.name.clone())
        .unwrap_or_default();
    let Some(th) = th else {
        return (name, String::new(), 0.0);
    };
    let layers: Vec<String> = th
        .layers
        .iter()
        .filter(|l| l.thickness > 0.0)
        .map(|l| format!("{} ({}[cm])", l.name, fmt(l.thickness * 1000.0)))
        .collect();
    let d: f64 = th.layers.iter().map(|l| l.thickness).sum::<f64>() * 1000.0;
    (name, layers.join(", "), round3(d))
}

/// An outer wall's outer face, from its left end to its right seen from
/// outside, corners met with the next outer walls'.
struct Face {
    wall: ElementId,
    a: DVec2,
    b: DVec2,
    /// Outwards.
    out: DVec2,
}

/// `room` tells whether a point is in a room on the level, and whether
/// that one is heated.
fn outer_faces(
    model: &Model,
    level: ElementId,
    inside: &dyn Fn(DVec2) -> Option<bool>,
) -> Vec<Face> {
    let outer = crate::sitefoot::outer_walls(model, level);
    let mut faces: Vec<(Face, creabim_core::Wall)> = vec![];
    // where along each wall from its start its heated part begins and
    // ends, where not at its ends
    let mut spans: Vec<(Option<f64>, Option<f64>)> = vec![];
    for id in outer {
        let Some(e) = model.get(id) else { continue };
        let ElementKind::Wall(w) = &e.kind else {
            continue;
        };
        if w.is_arc() {
            continue;
        }
        let d = (w.end - w.start).normalize_or_zero();
        let left = d.perp();
        // along it, which side the rooms are on and whether one of them
        // is heated
        let (mut plus, mut minus, mut warm) = (0, 0, false);
        for k in [0.1, 0.3, 0.5, 0.7, 0.9] {
            let at = w.start.lerp(w.end, k);
            for (s, n) in [(1.0, &mut plus), (-1.0, &mut minus)] {
                if let Some(h) = inside(at + left * s * (w.thickness / 2.0 + 0.1)) {
                    *n += 1;
                    warm |= h;
                }
            }
        }
        if !warm || plus == minus {
            continue;
        }
        let out = if plus > minus { -left } else { left };
        let off = out * w.thickness / 2.0;
        // seen from outside, left to right: out = left of the run means
        // the run goes right to left
        let (a, b) = if out == left {
            (w.end + off, w.start + off)
        } else {
            (w.start + off, w.end + off)
        };
        // the part of it along heated rooms: a wall shared with a garage
        // counts only where the house is warm
        let len = w.start.distance(w.end);
        let steps = (len / 0.1).ceil().max(1.0) as usize;
        let warm_at: Vec<f64> = (0..=steps)
            .map(|k| len * k as f64 / steps as f64)
            .filter(|u| inside(w.start + d * *u - out * (w.thickness / 2.0 + 0.1)) == Some(true))
            .collect();
        let (u0, u1) = match (warm_at.first(), warm_at.last()) {
            (Some(u0), Some(u1)) => (*u0, *u1),
            _ => continue,
        };
        // within a wall's thickness of an end is the whole way there
        let reach = w.thickness + 0.3;
        let span = ((u0 > reach).then_some(u0), (u1 < len - reach).then_some(u1));
        faces.push((
            Face {
                wall: id,
                a,
                b,
                out,
            },
            w.clone(),
        ));
        spans.push(span);
    }
    // meet each end with the face of the wall joined there
    let n = faces.len();
    let mut ends: Vec<(DVec2, DVec2)> = faces.iter().map(|f| (f.0.a, f.0.b)).collect();
    for i in 0..n {
        let wi = &faces[i].1;
        for end in [0, 1] {
            let p = if end == 0 { faces[i].0.a } else { faces[i].0.b };
            let c = p - faces[i].0.out * wi.thickness / 2.0;
            let near = (0..n).filter(|j| *j != i).find(|j| {
                let wj = &faces[*j].1;
                let reach = (wi.thickness + wj.thickness) / 2.0 + 0.05;
                (wj.start.distance(c) < reach || wj.end.distance(c) < reach)
                    && faces[*j].0.out.dot(faces[i].0.out).abs() < 0.99
            });
            if let Some(j) = near {
                let (fi, fj) = (&faces[i].0, &faces[j].0);
                if let Some(x) = cross(fi.a, fi.b - fi.a, fj.a, fj.b - fj.a) {
                    if end == 0 {
                        ends[i].0 = x;
                    } else {
                        ends[i].1 = x;
                    }
                }
            }
        }
    }
    for (i, (f, w)) in faces.iter().enumerate() {
        let d = (w.end - w.start).normalize_or_zero();
        let at = |u: f64| w.start + d * u + f.out * w.thickness / 2.0;
        // a runs from the start unless the face runs end to start
        let reversed = f.out == d.perp();
        let (s0, s1) = spans[i];
        let from_start = if reversed {
            &mut ends[i].1
        } else {
            &mut ends[i].0
        };
        if let Some(u) = s0 {
            *from_start = at(u);
        }
        let to_end = if reversed {
            &mut ends[i].0
        } else {
            &mut ends[i].1
        };
        if let Some(u) = s1 {
            *to_end = at(u);
        }
    }
    faces
        .into_iter()
        .zip(ends)
        .map(|((f, _), (a, b))| Face { a, b, ..f })
        .collect()
}

/// Where the lines `p + t·d` and `q + s·e` cross.
fn cross(p: DVec2, d: DVec2, q: DVec2, e: DVec2) -> Option<DVec2> {
    let den = d.perp_dot(e);
    if den.abs() < 1e-9 {
        return None;
    }
    Some(p + d * ((q - p).perp_dot(e) / den))
}

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// A number as Fragment's text fields hold it: "292.5", "117".
fn fmt(v: f64) -> String {
    let s = format!("{:.3}", round3(v));
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// A JSON object that keeps its keys in the order they are put in, as
/// Fragment's form lists its fields (its own files keep that order).
#[derive(Clone, Debug, Default)]
pub struct Ordered(Vec<(String, Node)>);

#[derive(Clone, Debug)]
pub enum Node {
    Leaf(Value),
    Object(Ordered),
}

impl From<Value> for Node {
    fn from(v: Value) -> Self {
        Node::Leaf(v)
    }
}

impl From<Ordered> for Node {
    fn from(o: Ordered) -> Self {
        Node::Object(o)
    }
}

impl Ordered {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, key: impl Into<String>, v: impl Into<Node>) {
        self.0.push((key.into(), v.into()));
    }

    /// `self` with `key` put in.
    pub fn with(mut self, key: impl Into<String>, v: impl Into<Node>) -> Self {
        self.insert(key, v);
        self
    }

    /// The object as JSON text.
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        out.push('{');
        for (i, (k, v)) in self.0.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&Value::from(k.as_str()).to_string());
            out.push(':');
            match v {
                Node::Leaf(v) => out.push_str(&v.to_string()),
                Node::Object(o) => o.write(out),
            }
        }
        out.push('}');
    }
}

fn field(kind: &str) -> Ordered {
    Ordered::new().with("type", Value::from(kind))
}
fn text(v: &str) -> Ordered {
    field("text").with("value", Value::from(v))
}
fn number(v: f64) -> Ordered {
    field("number").with("value", Value::from(fmt(v)))
}
fn tel(v: f64) -> Ordered {
    field("tel").with("value", json!(round3(v)))
}
fn select(index: i64, value: &str) -> Ordered {
    field("select-one")
        .with("text", json!({}))
        .with("index", Value::from(index))
        .with("value", Value::from(value))
}
fn checkbox(on: bool) -> Ordered {
    field("checkbox")
        .with("checked", Value::from(on))
        .with("value", Value::from("on"))
}
fn textarea(v: &str) -> Ordered {
    field("textarea").with("value", Value::from(v))
}

/// Fragment's orientation: H (horizontal) first, then N, NE … NW by
/// 45°, as its value (degrees, 360 for horizontal) and its index.
fn orientation(az: Option<f64>) -> Ordered {
    match az {
        None => select(0, "360"),
        Some(a) => {
            let k = ((a.rem_euclid(360.0) / 45.0).round() as i64) % 8;
            select(k + 1, &(k * 45).to_string())
        }
    }
}

/// Fragment's "Druh budovy" by the project's building use: rodinné domy,
/// bytové domy, administratívne budovy, školy, nemocnice, hotely a
/// reštaurácie, športové haly, obchody, in the order of its list.
fn building_kind(use_: &str) -> i64 {
    let u = use_.to_lowercase();
    let has = |w: &[&str]| w.iter().any(|x| u.contains(x));
    if has(&["bytov", "apartment", "flats"]) {
        1
    } else if has(&["admin", "office", "kancel"]) {
        2
    } else if has(&["škol", "skol", "school"]) {
        3
    } else if has(&["nemoc", "hospital"]) {
        4
    } else if has(&["hotel", "reštaur", "restaur"]) {
        5
    } else if has(&["šport", "sport"]) {
        6
    } else if has(&["obchod", "shop", "retail"]) {
        7
    } else {
        0
    }
}

/// The project file Fragment's "Projektové hodnotenie" opens.
pub fn project_json(model: &Model, env: &Envelope) -> Ordered {
    let info = model
        .project_info()
        .map(|(_, p)| p.clone())
        .unwrap_or_default();
    let name = [&info.building_name, &info.name]
        .into_iter()
        .find(|s| !s.trim().is_empty())
        .cloned()
        .unwrap_or_default();
    let designer = [&info.designer, &info.architect]
        .into_iter()
        .find(|s| !s.trim().is_empty())
        .cloned()
        .unwrap_or_default();
    let popis = Ordered::new().with(
        "",
        Ordered::new()
            .with("nazov_stavby", text(&name))
            .with("adresa", text(&info.address))
            .with("investor", text(&info.client))
            .with("meno_projektanta", text(&designer))
            .with("firma", text(&info.office)),
    );
    let vnut = Ordered::new().with(
        "",
        Ordered::new().with(
            "standart_podmienky",
            field("radio").with("value", Value::from("on")),
        ),
    );
    // heated storeys
    let n_storeys = env.storeys.len();
    let area: f64 = env.storeys.iter().map(|s| s.area).sum();
    let volume: f64 = env.storeys.iter().map(|s| s.volume).sum();
    let avg_h = if area > 0.0 { volume / area } else { 0.0 };
    let kind = building_kind(&info.building_use);
    let mut plochy = Ordered::new();
    let mut objemy = Ordered::new();
    for (i, s) in env.storeys.iter().enumerate() {
        let k = i + 1;
        let (l, w) = (s.area, 1.0);
        plochy.insert(format!("meno_{k}"), text(&s.name));
        plochy.insert(format!("pocet_{k}"), number(1.0));
        plochy.insert(format!("dlzka_{k}"), number(l));
        plochy.insert(format!("sirka_{k}"), number(w));
        plochy.insert(format!("plocha_{k}"), number(s.area));
        plochy.insert(format!("inTemp_{k}"), number(20.0));
        objemy.insert(format!("omeno_{k}"), text(&s.name));
        objemy.insert(format!("opocet_{k}"), number(1.0));
        objemy.insert(format!("oplocha_{k}"), number(s.area));
        objemy.insert(format!("vyska_{k}"), number(s.height));
        objemy.insert(format!("objem_{k}"), number(s.volume));
        objemy.insert(format!("oinTemp_{k}"), number(20.0));
    }
    plochy.insert(
        "pocet_teplovymenna_plocha".to_string(),
        number(n_storeys as f64),
    );
    plochy.insert("celkova_teplovymenna_plocha".to_string(), number(area));
    objemy.insert(
        "pocet_vykurovany_objem".to_string(),
        number(n_storeys as f64),
    );
    objemy.insert("celkovy_vykurovany_objem".to_string(), number(volume));
    let vyhrievane = Ordered::new()
        .with(
            "podlazia",
            Ordered::new()
                .with("druh_budovy", select(kind, &kind.to_string()))
                .with("pocet_podlazi", number(n_storeys as f64))
                .with("priemerna_vyska_podlazi", number(avg_h))
                .with("bud_orient", number(0.0))
                .with("ucel_budovy", select(0, "0"))
                .with("VZrozmer", number(env.size[0]))
                .with("SJrozmer", number(env.size[1]))
                .with("Z_rozmer", number(env.size[2])),
        )
        .with("teple_plochy", plochy)
        .with("teple_objemy", objemy);
    // the surfaces and their openings
    let mut st = Ordered::new();
    let (mut sum_net, mut sum_bua, mut n_open, mut sum_open, mut sum_open_bua) =
        (0.0, 0.0, 0usize, 0.0, 0.0);
    for (i, s) in env.surfaces.iter().enumerate() {
        let k = i + 1;
        let code = s.kind.code();
        let u = s.u.unwrap_or(0.0);
        let net = s.net();
        let ua = u * net;
        st.insert(
            format!("druh_plochy_{k}"),
            select(code - 1, &code.to_string()),
        );
        st.insert(format!("meno_{k}"), text(&s.name));
        st.insert(format!("pocet_{k}"), tel(1.0));
        st.insert(format!("dlzka_{k}"), tel(s.length));
        st.insert(format!("fix_dlzka_{k}"), tel(1.0));
        st.insert(format!("sirka_{k}"), tel(s.width));
        st.insert(format!("fix_sirka_{k}"), tel(0.0));
        st.insert(format!("orient_{k}"), orientation(s.azimuth));
        st.insert(format!("plocha_{k}"), tel(net));
        st.insert(format!("fix_plocha_{k}"), tel(0.0));
        st.insert(format!("z3D_{k}"), checkbox(false));
        let shape = json!({
            "tvar2D": {
                "vyber_tvar_plocha": "obdlznik",
                "obdlznik_plocha_usek_AyBy_By_Cy": round3(s.width),
                "obdlznik_plocha_usek_AxDx_Dx_Cx": round3(s.length),
                "areas_perimeter": {"obsah": round3(s.gross), "obvod": round3(2.0 * (s.length + s.width))},
            },
            "tvar3D": {"pole": [{
                "color": if s.kind.horizontal() { "#808000" } else { "#00ff00" },
                "position": {"x": 0, "y": 0, "z": 0},
                "rotation": {"_x": if s.kind.horizontal() { std::f64::consts::FRAC_PI_2 } else { 0.0 }, "_y": 0, "_z": 0, "order": "XYZ"},
            }]},
        });
        st.insert(format!("tvar2D3D_{k}"), textarea(&shape.to_string()));
        st.insert(format!("u_{k}"), tel(u));
        st.insert(format!("fix_u_{k}"), tel(0.0));
        st.insert(format!("ua_{k}"), tel(ua));
        st.insert(format!("bx_{k}"), tel(1.0));
        st.insert(format!("bxua_{k}"), tel(ua));
        st.insert(format!("obal_{k}"), checkbox(s.kind.envelope()));
        let konst = json!({
            "Uval": s.u.map(round3),
            "Rval": s.r,
            "hrubka": s.thickness,
            "file": "",
            "konstrukcia": s.assembly,
            "sumarizacia": s.layers,
        });
        st.insert(format!("konstrukcia_{k}"), textarea(&konst.to_string()));
        let (mut a_open, mut bua_open) = (0.0, 0.0);
        for (j, o) in s.openings.iter().enumerate() {
            let m = j + 1;
            let uw = o.uw.unwrap_or(0.0);
            let a = o.area();
            let code = o.kind.code();
            st.insert(
                format!("druh_otvoru_{k}_{m}"),
                select(code - 1, &code.to_string()),
            );
            st.insert(format!("omeno_{k}_{m}"), text(&o.name));
            st.insert(format!("opocet_{k}_{m}"), tel(1.0));
            st.insert(format!("osirka_{k}_{m}"), tel(o.width));
            st.insert(format!("fix_osirka_{k}_{m}"), tel(0.0));
            st.insert(format!("ovyska_{k}_{m}"), tel(o.height));
            st.insert(format!("fix_ovyska_{k}_{m}"), tel(1.0));
            st.insert(format!("oplocha_{k}_{m}"), tel(a));
            st.insert(format!("fix_oplocha_{k}_{m}"), tel(0.0));
            st.insert(format!("oz3D_{k}_{m}"), checkbox(true));
            let shape = json!({
                "vyber_tvar_otvory": "obdlznik",
                "rot_otvory": "0",
                "pole": [{"x": round3(o.x), "y": round3(o.y)}],
                "obdlznik_otvory_bod_Ax_Bx_Cx_Dx": 0,
                "obdlznik_otvory_bod_Ay_By_Cy_Dy": 0,
                "obdlznik_otvory_usek_AyBy_By_Cy": round3(o.height),
                "obdlznik_otvory_usek_AxDx_Dx_Cx": round3(o.width),
                "area_perimeter": {"obsah": round3(a), "obvod": round3(2.0 * (o.width + o.height))},
            });
            st.insert(format!("tvar2D_{k}_{m}"), textarea(&shape.to_string()));
            st.insert(format!("ou_{k}_{m}"), tel(uw));
            st.insert(format!("oua_{k}_{m}"), tel(uw * a));
            st.insert(format!("obx_{k}_{m}"), tel(1.0));
            st.insert(format!("obxua_{k}_{m}"), tel(uw * a));
            st.insert(
                format!("osolar_{k}_{m}"),
                checkbox(o.kind != ApertureKind::Door),
            );
            st.insert(format!("okonstrukcia_{k}_{m}"), textarea(""));
            a_open += a;
            bua_open += uw * a;
        }
        st.insert(format!("pocet_otvory_{k}"), tel(s.openings.len() as f64));
        st.insert(format!("celkova_plocha_otvory_{k}"), tel(a_open));
        st.insert(format!("celkovo_bxua_otvory_{k}"), tel(bua_open));
        sum_net += net;
        sum_bua += ua;
        n_open += s.openings.len();
        sum_open += a_open;
        sum_open_bua += bua_open;
    }
    st.insert("pocet_steny".to_string(), tel(env.surfaces.len() as f64));
    st.insert("celkova_plocha_panely".to_string(), tel(sum_net));
    st.insert("celkovo_bxua".to_string(), tel(sum_bua));
    st.insert("sumar_pocet_otvory".to_string(), tel(n_open as f64));
    st.insert("celkova_plocha_vsetky_otvory".to_string(), tel(sum_open));
    st.insert("celkovo_bxua_vsetky_otvory".to_string(), tel(sum_open_bua));
    st.insert("celkovo_plocha_obalky".to_string(), tel(sum_net + sum_open));
    st.insert(
        "celkovo_bxua_vsetko".to_string(),
        tel(sum_bua + sum_open_bua),
    );
    Ordered::new()
        .with("popis_zakazky", popis)
        .with("vnut_okraj_podmienky", vnut)
        .with("vyhrievane_plochy", vyhrievane)
        .with("Skladba", Ordered::new().with("stavby", st))
        .with("__setup", json!({"absoluteKompasAngle": 0}))
}

/// The project file, as text, and what it could not tell.
pub fn export(model: &Model) -> (String, Vec<String>) {
    let env = envelope(model);
    (project_json(model, &env).to_json(), env.warnings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::{Document, Element, Room, Transaction};

    /// The layered sample house with a living room on each storey and a
    /// garage-less ground floor: rooms filling each storey.
    fn house() -> Model {
        let mut doc = Document::from_model(creabim_core::sample::layered_house());
        let mut tx = Transaction::new("rooms");
        let ground = doc
            .model()
            .levels()
            .min_by(|a, b| a.1.elevation.total_cmp(&b.1.elevation))
            .unwrap()
            .0
            .id;
        for (e, _) in doc.model().levels() {
            let mut r = Room::new(e.id, DVec2::new(7.5, 3.0));
            r.number = format!("{}.01", e.name.len());
            tx = tx.insert(Element::new("Obývacia izba", ElementKind::Room(r)));
        }
        // a garage beside the living room on the ground floor
        tx = tx.insert(Element::new(
            "Garáž",
            ElementKind::Room(Room::new(ground, DVec2::new(3.0, 3.0))),
        ));
        // every material a λ: insulation 0,04, the rest 0,8 W/(m·K)
        for before in doc.model().iter() {
            if let ElementKind::Material(m) = &before.kind {
                if m.lambda.is_none() {
                    let mut after = before.clone();
                    if let ElementKind::Material(m) = &mut after.kind {
                        m.lambda = Some(if before.name.contains("Insulation") {
                            0.04
                        } else {
                            0.8
                        });
                    }
                    tx = tx.replace(before.clone(), after);
                }
            }
        }
        doc.commit(tx).unwrap();
        doc.model().clone()
    }

    #[test]
    fn the_envelope_has_walls_on_every_side_a_floor_and_a_top() {
        let m = house();
        let env = envelope(&m);
        assert!(!env.storeys.is_empty(), "{env:#?}");
        let walls: Vec<&Surface> = env
            .surfaces
            .iter()
            .filter(|s| s.kind == SurfaceKind::OuterWall)
            .collect();
        assert!(walls.len() >= 4, "{:#?}", env.surfaces);
        // the four directions of a box
        let mut dirs: Vec<i64> = walls
            .iter()
            .map(|s| (s.azimuth.unwrap() / 90.0).round() as i64 % 4)
            .collect();
        dirs.sort();
        dirs.dedup();
        assert_eq!(dirs, vec![0, 1, 2, 3]);
        // outer faces meet at the corners: the outer perimeter of one
        // storey is more than the centrelines'
        assert!(walls.iter().all(|s| s.length > 0.0 && s.width > 2.0));
        assert!(walls.iter().any(|s| !s.openings.is_empty()));
        // a floor on the ground, split by the 2 m strip, and something on top
        let floor: f64 = env
            .surfaces
            .iter()
            .filter(|s| matches!(s.kind, SurfaceKind::FloorEdge | SurfaceKind::FloorOther))
            .map(|s| s.gross)
            .sum();
        assert!(floor > 10.0, "{:#?}", env.surfaces);
        assert!(env
            .surfaces
            .iter()
            .any(|s| s.kind == SurfaceKind::FloorEdge));
        assert!(env.surfaces.iter().any(|s| matches!(
            s.kind,
            SurfaceKind::Roof | SurfaceKind::UnderUnheated | SurfaceKind::OuterWall
        ) && s.kind.horizontal()
            || s.kind == SurfaceKind::Roof));
    }

    #[test]
    fn the_file_is_fragments_form() {
        let m = house();
        let (text, _) = export(&m);
        let v: Value = serde_json::from_str(&text).unwrap();
        for k in [
            "popis_zakazky",
            "vnut_okraj_podmienky",
            "vyhrievane_plochy",
            "Skladba",
            "__setup",
        ] {
            assert!(v.get(k).is_some(), "{k}");
        }
        let st = &v["Skladba"]["stavby"];
        let n = st["pocet_steny"]["value"].as_f64().unwrap() as usize;
        assert!(n >= 6);
        for k in 1..=n {
            let d = &st[format!("druh_plochy_{k}")];
            assert_eq!(d["type"], "select-one");
            let code: i64 = d["value"].as_str().unwrap().parse().unwrap();
            assert_eq!(d["index"].as_i64().unwrap(), code - 1);
            assert!((1..=12).contains(&code));
            let konst: Value =
                serde_json::from_str(st[format!("konstrukcia_{k}")]["value"].as_str().unwrap())
                    .unwrap();
            assert!(konst.get("Uval").is_some());
            let orient = &st[format!("orient_{k}")];
            let o: i64 = orient["value"].as_str().unwrap().parse().unwrap();
            assert!(o == 360 || o % 45 == 0);
        }
        // in the form's order, as Fragment's own files
        assert!(text.starts_with(r#"{"popis_zakazky":{"":{"nazov_stavby":{"type":"text","value":"#));
        let skladba = &text[text.find("\"Skladba\"").unwrap()..];
        assert!(skladba.find("druh_plochy_1").unwrap() < skladba.find("meno_1").unwrap());
        // numbers of the form are text, of the table numbers
        assert!(v["vyhrievane_plochy"]["podlazia"]["pocet_podlazi"]["value"].is_string());
        assert!(st["dlzka_1"]["value"].is_number());
    }

    #[test]
    fn orientation_and_kinds_follow_fragments_lists() {
        let o = |a| serde_json::from_str::<Value>(&orientation(a).to_json()).unwrap();
        assert_eq!(o(None)["value"], "360");
        assert_eq!(o(Some(270.0))["index"], 7);
        assert_eq!(o(Some(0.0))["index"], 1);
        assert_eq!(o(Some(359.0))["value"], "0");
        assert_eq!(SurfaceKind::FloorEdge.abbr(), "PV1");
        assert_eq!(SurfaceKind::ground_wall(1.0), SurfaceKind::GroundWall2);
        assert_eq!(ApertureKind::GlazedDoor.code(), 3);
    }
}
