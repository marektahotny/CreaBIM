//! gbXML (Green Building XML, schema 7.03) export of the building's
//! energy model, for energy and daylight tools (OpenStudio/EnergyPlus,
//! IES VE, DesignBuilder, Sefaira, Trane TRACE, Carrier HAP…).
//!
//! The model is built from the rooms, as gbXML wants it: every room is a
//! space, its floor at its storey's floor and its ceiling at its own
//! height, drawn on the inner faces of its walls (gbXML's
//! "InteriorSurface" reference). Each room's sides are split by what is
//! beyond them, probed through the wall:
//! - another room of the storey: an interior wall between the two (once,
//!   with both spaces);
//! - nothing: an outer wall, the part under the outside ground an
//!   underground wall;
//!
//! under each room the room of the storey below (an interior floor
//! between the two), else on the lowest storey a slab on the ground (an
//! underground slab under the outside ground), else a floor over the
//! outside; over a room with no room above: its roof, or the slab of the
//! storey above it (a ceiling under an unheated attic, given as a roof:
//! the edge of the heated volume, with the slab's build-up). A pitched
//! roof is flat at the room's ceiling.
//!
//! Windows and doors sit in the surfaces of their walls, at their size.
//! The surfaces take the build-ups of their walls, slabs and roofs with
//! U and their layers (thickness, λ, R); windows their Uw and g, doors
//! their Ud, from the window's or door's data. Heated rooms are "Heated",
//! the others (a garage, a cellar) "Unconditioned"; loggias, terraces,
//! balconies and porches are the outside, not spaces.

use crate::fragment::heated;
use creabim_core::assembly::assembly_of;
use creabim_core::{levels, polygon, room, thermal, ElementId, ElementKind, Model, OpeningKind};
use creabim_services::heatloss;
use glam::{DVec2, DVec3};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// A part of a room's side: its surface type, bottom, top and spaces.
type WallPart = (&'static str, f64, f64, Vec<(String, &'static str)>);

/// A room as a space.
struct Space {
    id: String,
    name: String,
    occupancy: String,
    level: ElementId,
    elevation: f64,
    height: f64,
    outline: Vec<DVec2>,
    bounded_by: Vec<Option<ElementId>>,
    area: f64,
    heated: bool,
    inside: DVec2,
}

/// A window or door in a surface.
struct Hole {
    id: String,
    name: String,
    kind: &'static str,
    /// The window type or the door's construction.
    window_type: Option<String>,
    construction: Option<String>,
    poly: Vec<DVec3>,
    /// Left and bottom on its surface, width, height.
    rect: [f64; 4],
}

/// A surface of the energy model.
struct Surf {
    kind: &'static str,
    name: String,
    /// The spaces it bounds, the first the one it faces away from; the
    /// second with what it is for it.
    spaces: Vec<(String, &'static str)>,
    construction: String,
    poly: Vec<DVec3>,
    azimuth: f64,
    tilt: f64,
    holes: Vec<Hole>,
}

/// The constructions, layers, materials and window types written.
#[derive(Default)]
struct Library {
    out: String,
    by_element: HashMap<ElementId, String>,
    by_key: HashMap<String, String>,
    materials: HashMap<String, String>,
    layers: usize,
}

impl Library {
    fn unknown(&mut self) -> String {
        self.by_key
            .entry("unknown".into())
            .or_insert_with(|| {
                let _ = writeln!(
                    self.out,
                    "  <Construction id=\"con0\">\n    <Name>Neurčená skladba</Name>\n  </Construction>"
                );
                "con0".into()
            })
            .clone()
    }

    /// The construction of a wall, slab or roof: its build-up with U and
    /// layers.
    fn of_element(&mut self, model: &Model, id: Option<ElementId>) -> String {
        let Some(e) = id.and_then(|i| model.get(i)) else {
            return self.unknown();
        };
        if let Some(c) = self.by_element.get(&e.id) {
            return c.clone();
        }
        let name = assembly_of(model, e).map_or_else(|| e.name.clone(), |(a, _)| a.name.clone());
        let th = thermal::of_element(model, e);
        let key = format!(
            "{name}|{:?}|{}",
            th.as_ref().and_then(|t| t.u).map(|u| (u * 1e4).round()),
            th.as_ref().map_or(0, |t| t.layers.len())
        );
        if let Some(c) = self.by_key.get(&key) {
            let c = c.clone();
            self.by_element.insert(e.id, c.clone());
            return c;
        }
        let cid = format!("con{}", self.by_key.len() + 1);
        let mut layer_ids = vec![];
        let mut defs = String::new();
        if let Some(t) = &th {
            for l in &t.layers {
                let mkey = format!("{}|{:.4}|{:?}", l.name, l.thickness, l.lambda);
                let n = self.materials.len() + 1;
                let mid = self
                    .materials
                    .entry(mkey)
                    .or_insert_with(|| {
                        let mid = format!("mat{n}");
                        let _ = write!(
                            defs,
                            "  <Material id=\"{mid}\">\n    <Name>{}</Name>\n",
                            esc(&l.name)
                        );
                        if let Some(r) = l.r {
                            let _ = writeln!(
                                defs,
                                "    <R-value unit=\"SquareMeterKPerW\">{}</R-value>",
                                num(r)
                            );
                        }
                        let _ = writeln!(
                            defs,
                            "    <Thickness unit=\"Meters\">{}</Thickness>",
                            num(l.thickness)
                        );
                        if let Some(lambda) = l.lambda {
                            let _ = writeln!(
                                defs,
                                "    <Conductivity unit=\"WPerMeterK\">{}</Conductivity>",
                                num(lambda)
                            );
                        }
                        defs.push_str("  </Material>\n");
                        mid
                    })
                    .clone();
                self.layers += 1;
                let lid = format!("lay{}", self.layers);
                let _ = writeln!(
                    defs,
                    "  <Layer id=\"{lid}\">\n    <Name>{}</Name>\n    <MaterialId materialIdRef=\"{mid}\"/>\n  </Layer>",
                    esc(&l.name)
                );
                layer_ids.push(lid);
            }
        }
        let _ = write!(
            self.out,
            "  <Construction id=\"{cid}\">\n    <Name>{}</Name>\n",
            esc(&name)
        );
        if let Some(u) = th.as_ref().and_then(|t| t.u) {
            let _ = writeln!(
                self.out,
                "    <U-value unit=\"WPerSquareMeterK\">{}</U-value>",
                num(u)
            );
        }
        for l in &layer_ids {
            let _ = writeln!(self.out, "    <LayerId layerIdRef=\"{l}\"/>");
        }
        self.out.push_str("  </Construction>\n");
        self.out.push_str(&defs);
        self.by_key.insert(key, cid.clone());
        self.by_element.insert(e.id, cid.clone());
        cid
    }

    /// A door's construction: its Ud.
    fn door(&mut self, name: &str, u: Option<f64>) -> String {
        let key = format!("door|{name}|{:?}", u.map(|u| (u * 1e4).round()));
        if let Some(c) = self.by_key.get(&key) {
            return c.clone();
        }
        let cid = format!("con{}", self.by_key.len() + 1);
        let _ = write!(
            self.out,
            "  <Construction id=\"{cid}\">\n    <Name>Dvere – {}</Name>\n",
            esc(name)
        );
        if let Some(u) = u {
            let _ = writeln!(
                self.out,
                "    <U-value unit=\"WPerSquareMeterK\">{}</U-value>",
                num(u)
            );
        }
        self.out.push_str("  </Construction>\n");
        self.by_key.insert(key, cid.clone());
        cid
    }

    /// A window type: its Uw and g.
    fn window(&mut self, name: &str, uw: Option<f64>, g: Option<f64>) -> String {
        let key = format!("window|{name}|{uw:?}|{g:?}");
        if let Some(c) = self.by_key.get(&key) {
            return c.clone();
        }
        let n = self
            .by_key
            .keys()
            .filter(|k| k.starts_with("window|"))
            .count()
            + 1;
        let id = format!("wt{n}");
        let _ = write!(
            self.out,
            "  <WindowType id=\"{id}\">\n    <Name>{}</Name>\n",
            esc(name)
        );
        if let Some(u) = uw {
            let _ = writeln!(
                self.out,
                "    <U-value unit=\"WPerSquareMeterK\">{}</U-value>",
                num(u)
            );
        }
        if let Some(g) = g {
            let _ = writeln!(
                self.out,
                "    <SolarHeatGainCoeff unit=\"Fraction\">{}</SolarHeatGainCoeff>",
                num(g)
            );
        }
        self.out.push_str("  </WindowType>\n");
        self.by_key.insert(key, id.clone());
        id
    }
}

/// The building's use as gbXML's building type.
fn building_type(use_: &str) -> &'static str {
    let u = use_.to_lowercase();
    let has = |w: &[&str]| w.iter().any(|x| u.contains(x));
    if has(&["bytov", "apartment", "flats", "multi"]) {
        "MultiFamily"
    } else if has(&["admin", "office", "kancel"]) {
        "Office"
    } else if has(&["škol", "skol", "school"]) {
        "SchoolOrUniversity"
    } else if has(&["nemoc", "hospital", "zdrav"]) {
        "HospitalOrHealthcare"
    } else if has(&["hotel", "penzi"]) {
        "Hotel"
    } else if has(&["reštaur", "restaur"]) {
        "DiningFamily"
    } else if has(&["šport", "sport", "telocv"]) {
        "Gymnasium"
    } else if has(&["obchod", "shop", "retail", "predaj"]) {
        "Retail"
    } else if has(&["sklad", "warehouse"]) {
        "Warehouse"
    } else if has(&["výrob", "vyrob", "dielň", "workshop"]) {
        "Workshop"
    } else {
        // a family house: what CreaBIM is mostly drawn for
        "SingleFamily"
    }
}

/// The model's energy model as a gbXML document, and what it could not
/// tell, in Slovak.
pub fn export(model: &Model) -> (String, Vec<String>) {
    let mut warnings: Vec<String> = vec![];
    let info = model
        .project_info()
        .map(|(_, p)| p.clone())
        .unwrap_or_default();
    let north = info.true_north;
    let ground = crate::summary::ground_at_building(model).map_or(0.0, |g| g.0);
    let storeys = levels::sorted(model);
    let mut spaces: Vec<Space> = vec![];
    for e in model.iter() {
        let ElementKind::Room(r) = &e.kind else {
            continue;
        };
        // a loggia, a terrace, a balcony, a porch is the outside
        let porch = ["porch", "zápraž", "zapraz"]
            .iter()
            .any(|w| e.name.to_lowercase().contains(w) || r.occupancy.to_lowercase().contains(w));
        if porch || room::is_open_air(&e.name, r) {
            continue;
        }
        let Some(s) = room::shape(model, e.id).filter(|s| s.outline.len() > 2) else {
            continue;
        };
        let inside = if polygon::contains(&s.outline, r.at) {
            r.at
        } else {
            s.outline.iter().copied().sum::<DVec2>() / s.outline.len() as f64
        };
        let name = match (r.number.trim(), e.name.trim()) {
            ("", n) => n.to_string(),
            (num, n) => format!("{num} {n}"),
        };
        spaces.push(Space {
            id: format!("sp{}", spaces.len() + 1),
            name,
            occupancy: r.occupancy.clone(),
            level: r.level,
            elevation: model.level(r.level).map_or(0.0, |l| l.elevation),
            height: room::height(model, r),
            area: room::net_area(model, r, &s),
            heated: heated(&e.name, r),
            inside,
            outline: s.outline,
            bounded_by: s.bounded_by,
        });
    }
    if spaces.is_empty() {
        warnings
            .push("V modeli nie sú miestnosti: gbXML potrebuje miestnosti ako priestory.".into());
    }
    let lowest = spaces.iter().map(|s| s.elevation).fold(f64::MAX, f64::min);
    let walls: Vec<(ElementId, &creabim_core::Wall)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Wall(w) => Some((e.id, w)),
            _ => None,
        })
        .collect();
    let room_at = |level: ElementId, p: DVec2, not: usize| {
        spaces
            .iter()
            .enumerate()
            .find(|(k, s)| *k != not && s.level == level && polygon::contains(&s.outline, p))
            .map(|(k, _)| k)
    };
    // the room of the storey nearest under or over a room
    let stacked = |i: usize, up: bool| {
        let s = &spaces[i];
        spaces
            .iter()
            .enumerate()
            .filter(|(_, x)| {
                (if up {
                    x.elevation > s.elevation + 1e-6
                } else {
                    x.elevation < s.elevation - 1e-6
                }) && polygon::contains(&x.outline, s.inside)
            })
            .min_by(|a, b| {
                let d = |x: &Space| (x.elevation - s.elevation).abs();
                d(a.1).total_cmp(&d(b.1))
            })
            .map(|(k, _)| k)
    };
    let slab_at = |level: ElementId, p: DVec2| {
        model
            .iter()
            .find(|e| {
                matches!(&e.kind, ElementKind::Slab(s)
                    if s.level == level && s.boundary.len() > 2 && polygon::contains(&s.boundary, p))
            })
            .map(|e| e.id)
    };
    let roof_at = |p: DVec2| {
        model
            .iter()
            .find(|e| {
                matches!(&e.kind, ElementKind::Roof(r) if r.boundary.len() > 2 && polygon::contains(&r.boundary, p))
            })
            .map(|e| e.id)
    };

    let mut lib = Library::default();
    let mut surfs: Vec<Surf> = vec![];
    let mut placed: HashSet<ElementId> = HashSet::new();
    let mut holes = 0usize;
    let mut no_uw = 0usize;
    for (i, s) in spaces.iter().enumerate() {
        let n = s.outline.len();
        let (z0, z1) = (s.elevation, s.elevation + s.height);
        for k in 0..n {
            let (a, b) = (s.outline[k], s.outline[(k + 1) % n]);
            let len = a.distance(b);
            if len < 0.01 {
                continue;
            }
            let dir = (b - a) / len;
            let out = DVec2::new(dir.y, -dir.x);
            // the wall along this side: the one it is bounded by, else the
            // nearest parallel one
            let wall = s.bounded_by[k]
                .filter(|id| walls.iter().any(|w| w.0 == *id))
                .or_else(|| {
                    let mid = (a + b) / 2.0;
                    walls
                        .iter()
                        .filter(|(_, w)| w.level == s.level && !w.is_arc())
                        .filter(|(_, w)| {
                            let d = (w.end - w.start).normalize_or_zero();
                            d.perp_dot(dir).abs() < 0.02
                                && crate::fragment::segment_distance(mid, w.start, w.end)
                                    <= w.thickness / 2.0 + 0.15
                        })
                        .min_by(|x, y| {
                            crate::fragment::segment_distance(mid, x.1.start, x.1.end).total_cmp(
                                &crate::fragment::segment_distance(mid, y.1.start, y.1.end),
                            )
                        })
                        .map(|w| w.0)
                });
            let reach = wall
                .and_then(|id| walls.iter().find(|w| w.0 == id))
                .map_or(0.3, |w| w.1.thickness + 0.15);
            // what is beyond, along the side
            let steps = ((len / 0.1).ceil() as usize).max(3);
            let beyond: Vec<Option<usize>> = (0..steps)
                .map(|j| {
                    let t = (j as f64 + 0.5) / steps as f64 * len;
                    let p = a + dir * t;
                    let mut d = 0.04;
                    while d <= reach {
                        if let Some(o) = room_at(s.level, p + out * d, i) {
                            return Some(o);
                        }
                        d += 0.04;
                    }
                    None
                })
                .collect();
            let mut runs: Vec<(f64, f64, Option<usize>)> = vec![];
            for (j, nb) in beyond.iter().enumerate() {
                let t1 = (j + 1) as f64 / steps as f64 * len;
                match runs.last_mut() {
                    Some(r) if r.2 == *nb => r.1 = t1,
                    _ => runs.push((runs.last().map_or(0.0, |r| r.1), t1, *nb)),
                }
            }
            let az = heatloss::azimuth(out, north);
            for (t0, t1, nb) in runs {
                if t1 - t0 < 0.02 {
                    continue;
                }
                // the parts of the side: (kind, bottom, top, the spaces)
                let parts: Vec<WallPart> = match nb {
                    // between two rooms, written by the first
                    Some(o) if o < i => continue,
                    Some(o) => vec![(
                        "InteriorWall",
                        z0,
                        z1,
                        vec![
                            (s.id.clone(), "InteriorWall"),
                            (spaces[o].id.clone(), "InteriorWall"),
                        ],
                    )],
                    None if ground > z0 + 0.05 => {
                        let g = ground.min(z1);
                        let mut v = vec![(
                            "UndergroundWall",
                            z0,
                            g,
                            vec![(s.id.clone(), "UndergroundWall")],
                        )];
                        if z1 - g > 0.05 {
                            v.push(("ExteriorWall", g, z1, vec![(s.id.clone(), "ExteriorWall")]));
                        }
                        v
                    }
                    None => vec![("ExteriorWall", z0, z1, vec![(s.id.clone(), "ExteriorWall")])],
                };
                let construction = lib.of_element(model, wall);
                let wname = wall
                    .and_then(|w| model.get(w))
                    .map_or(String::new(), |w| w.name.clone());
                for (kind, zb, zt, adj) in parts {
                    let pa = a + dir * t0;
                    let pb = a + dir * t1;
                    let poly = vec![pa.extend(zb), pb.extend(zb), pb.extend(zt), pa.extend(zt)];
                    // the windows and doors of the wall in this part
                    let mut hs = vec![];
                    if let Some((wid, w)) = wall.and_then(|id| walls.iter().find(|w| w.0 == id)) {
                        let wd = (w.end - w.start).normalize_or_zero();
                        let base =
                            model.level(w.level).map_or(0.0, |l| l.elevation) + w.base_offset;
                        for o in model.iter() {
                            let ElementKind::Opening(op) = &o.kind else {
                                continue;
                            };
                            if op.host != *wid
                                || op.kind == OpeningKind::Hole
                                || placed.contains(&o.id)
                            {
                                continue;
                            }
                            let p0 = w.start + wd * op.offset;
                            let p1 = p0 + wd * op.width;
                            let (u0, u1) = ((p0 - a).dot(dir), (p1 - a).dot(dir));
                            let (lo, hi) = (u0.min(u1), u0.max(u1));
                            let (bot, top) = (base + op.sill, base + op.sill + op.height);
                            let (c, cz) = ((lo + hi) / 2.0, (bot + top) / 2.0);
                            if c < t0 || c > t1 || cz < zb || cz > zt {
                                continue;
                            }
                            let (lo, hi) = (lo.max(t0 + 0.001), hi.min(t1 - 0.001));
                            let (bot, top) = (bot.max(zb + 0.001), top.min(zt - 0.001));
                            if hi - lo < 0.01 || top - bot < 0.01 {
                                continue;
                            }
                            placed.insert(o.id);
                            holes += 1;
                            let perf = op.performance.as_ref();
                            let uw = perf.and_then(|p| p.uw);
                            if uw.is_none() && kind != "InteriorWall" {
                                no_uw += 1;
                            }
                            let glazed = perf.is_some_and(|p| p.g.is_some());
                            let label = perf
                                .map(|p| p.product.trim().to_string())
                                .filter(|p| !p.is_empty())
                                .unwrap_or_else(|| o.name.clone());
                            let (okind, wt, con) = match op.kind {
                                OpeningKind::Window => (
                                    "OperableWindow",
                                    Some(lib.window(&label, uw, perf.and_then(|p| p.g))),
                                    None,
                                ),
                                _ if glazed => (
                                    "NonSlidingDoor",
                                    Some(lib.window(&label, uw, perf.and_then(|p| p.g))),
                                    None,
                                ),
                                _ => ("NonSlidingDoor", None, Some(lib.door(&label, uw))),
                            };
                            let (qa, qb) = (a + dir * lo, a + dir * hi);
                            hs.push(Hole {
                                id: format!("op{holes}"),
                                name: o.name.clone(),
                                kind: okind,
                                window_type: wt,
                                construction: con,
                                poly: vec![
                                    qa.extend(bot),
                                    qb.extend(bot),
                                    qb.extend(top),
                                    qa.extend(top),
                                ],
                                rect: [lo - t0, bot - zb, hi - lo, top - bot],
                            });
                        }
                    }
                    surfs.push(Surf {
                        kind,
                        name: if wname.is_empty() {
                            s.name.clone()
                        } else {
                            format!("{} – {}", s.name, wname)
                        },
                        spaces: adj,
                        construction: construction.clone(),
                        poly,
                        azimuth: az,
                        tilt: 90.0,
                        holes: hs,
                    });
                }
            }
        }
        // the floor
        let floor: Vec<DVec3> = s.outline.iter().rev().map(|p| p.extend(z0)).collect();
        let ceiling: Vec<DVec3> = s.outline.iter().map(|p| p.extend(z1)).collect();
        let slab = slab_at(s.level, s.inside);
        let (kind, adj) = match stacked(i, false) {
            Some(o) => (
                "InteriorFloor",
                vec![
                    (s.id.clone(), "InteriorFloor"),
                    (spaces[o].id.clone(), "Ceiling"),
                ],
            ),
            None if s.elevation <= lowest + 1e-6 && z0 < ground - 0.05 => {
                ("UndergroundSlab", vec![(s.id.clone(), "UndergroundSlab")])
            }
            None if s.elevation <= lowest + 1e-6 => {
                ("SlabOnGrade", vec![(s.id.clone(), "SlabOnGrade")])
            }
            None => ("ExposedFloor", vec![(s.id.clone(), "ExposedFloor")]),
        };
        if slab.is_none() {
            warnings.push(format!(
                "Pod miestnosťou „{}“ nie je doska: podlaha má neurčenú skladbu.",
                s.name
            ));
        }
        surfs.push(Surf {
            kind,
            name: format!("{} – podlaha", s.name),
            spaces: adj,
            construction: lib.of_element(model, slab),
            poly: floor,
            azimuth: 0.0,
            tilt: 180.0,
            holes: vec![],
        });
        // the ceiling or the roof, when no room is over it
        if stacked(i, true).is_none() {
            let next = storeys
                .iter()
                .filter(|l| l.2.elevation > s.elevation + 1e-6)
                .min_by(|a, b| a.2.elevation.total_cmp(&b.2.elevation))
                .and_then(|l| slab_at(l.0, s.inside));
            let (top, what) = match next {
                Some(id) => (Some(id), "strop"),
                None => (roof_at(s.inside), "strecha"),
            };
            if top.is_none() {
                warnings.push(format!(
                    "Nad miestnosťou „{}“ nie je strop ani strecha: neurčená skladba.",
                    s.name
                ));
            }
            surfs.push(Surf {
                kind: "Roof",
                name: format!("{} – {what}", s.name),
                spaces: vec![(s.id.clone(), "Roof")],
                construction: lib.of_element(model, top),
                poly: ceiling,
                azimuth: 0.0,
                tilt: 0.0,
                holes: vec![],
            });
        }
    }
    if no_uw > 0 {
        warnings.push(format!(
            "{no_uw} okien/dverí v obálke nemá Uw (vlastnosti okna/dverí)."
        ));
    }

    // the document
    let mut x = String::new();
    x.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    x.push_str("<gbXML xmlns=\"http://www.gbxml.org/schema\" temperatureUnit=\"C\" lengthUnit=\"Meters\" areaUnit=\"SquareMeters\" volumeUnit=\"CubicMeters\" useSIUnitsForResults=\"true\" version=\"7.03\" SurfaceReferenceLocation=\"InteriorSurface\">\n");
    x.push_str("<Campus id=\"campus\">\n");
    let title = [&info.building_name, &info.name]
        .into_iter()
        .find(|s| !s.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| "CreaBIM".into());
    let _ = writeln!(x, "  <Name>{}</Name>", esc(&title));
    x.push_str("  <Location>\n");
    let _ = writeln!(
        x,
        "    <ZipcodeOrPostalCode>{}</ZipcodeOrPostalCode>",
        esc(&postal_code(&info.address))
    );
    let place = [&info.site_name, &info.address]
        .into_iter()
        .find(|s| !s.trim().is_empty())
        .cloned()
        .unwrap_or_default();
    if !place.is_empty() {
        let _ = writeln!(x, "    <Name>{}</Name>", esc(&place));
    }
    if let (Some(lat), Some(lon)) = (info.latitude, info.longitude) {
        let _ = writeln!(
            x,
            "    <Latitude>{}</Latitude>\n    <Longitude>{}</Longitude>",
            num(lat),
            num(lon)
        );
    } else {
        warnings.push(
            "Projekt nemá zemepisnú šírku a dĺžku (Údaje o stavbe): poloha v gbXML chýba.".into(),
        );
    }
    let _ = writeln!(x, "    <Elevation>{}</Elevation>", num(info.elevation));
    // model north (drawing up) clockwise from true north
    let _ = writeln!(
        x,
        "    <CADModelAzimuth>{}</CADModelAzimuth>",
        num(north.rem_euclid(360.0))
    );
    x.push_str("  </Location>\n");
    let _ = writeln!(
        x,
        "  <Building id=\"bldg\" buildingType=\"{}\">",
        building_type(&info.building_use)
    );
    let _ = writeln!(x, "    <Name>{}</Name>", esc(&title));
    if !info.address.trim().is_empty() {
        let _ = writeln!(
            x,
            "    <StreetAddress>{}</StreetAddress>",
            esc(&info.address)
        );
    }
    let total: f64 = spaces.iter().map(|s| s.area).sum();
    let _ = writeln!(x, "    <Area unit=\"SquareMeters\">{}</Area>", num(total));
    let mut storey_ids: HashMap<ElementId, String> = HashMap::new();
    for (lid, lname, l) in &storeys {
        if !spaces.iter().any(|s| s.level == *lid) {
            continue;
        }
        let id = format!("bs{}", storey_ids.len() + 1);
        let _ = writeln!(
            x,
            "    <BuildingStorey id=\"{id}\">\n      <Name>{}</Name>\n      <Level unit=\"Meters\">{}</Level>\n    </BuildingStorey>",
            esc(lname),
            num(l.elevation)
        );
        storey_ids.insert(*lid, id);
    }
    for s in &spaces {
        let cond = if s.heated { "Heated" } else { "Unconditioned" };
        let _ = writeln!(
            x,
            "    <Space id=\"{}\" conditionType=\"{cond}\" buildingStoreyIdRef=\"{}\">",
            s.id,
            storey_ids.get(&s.level).map_or("", |v| v.as_str())
        );
        let _ = writeln!(x, "      <Name>{}</Name>", esc(&s.name));
        if !s.occupancy.trim().is_empty() {
            let _ = writeln!(x, "      <Description>{}</Description>", esc(&s.occupancy));
        }
        let _ = writeln!(
            x,
            "      <Area unit=\"SquareMeters\">{}</Area>",
            num(s.area)
        );
        let _ = writeln!(
            x,
            "      <Volume unit=\"CubicMeters\">{}</Volume>",
            num(s.area * s.height)
        );
        let z0 = s.elevation;
        let z1 = z0 + s.height;
        let _ = writeln!(
            x,
            "      <PlanarGeometry>\n{}      </PlanarGeometry>",
            loop_xml(
                &s.outline.iter().map(|p| p.extend(z0)).collect::<Vec<_>>(),
                8
            )
        );
        let _ = writeln!(
            x,
            "      <ShellGeometry id=\"{}sh\" unit=\"Meters\">\n        <ClosedShell>",
            s.id
        );
        x.push_str(&loop_xml(
            &s.outline
                .iter()
                .rev()
                .map(|p| p.extend(z0))
                .collect::<Vec<_>>(),
            10,
        ));
        x.push_str(&loop_xml(
            &s.outline.iter().map(|p| p.extend(z1)).collect::<Vec<_>>(),
            10,
        ));
        let n = s.outline.len();
        for k in 0..n {
            let (a, b) = (s.outline[k], s.outline[(k + 1) % n]);
            if a.distance(b) < 0.01 {
                continue;
            }
            x.push_str(&loop_xml(
                &[a.extend(z0), b.extend(z0), b.extend(z1), a.extend(z1)],
                10,
            ));
        }
        x.push_str("        </ClosedShell>\n      </ShellGeometry>\n    </Space>\n");
    }
    x.push_str("  </Building>\n");
    for (k, s) in surfs.iter().enumerate() {
        let _ = writeln!(
            x,
            "  <Surface id=\"su{}\" surfaceType=\"{}\" constructionIdRef=\"{}\">",
            k + 1,
            s.kind,
            s.construction
        );
        let _ = writeln!(x, "    <Name>{}</Name>", esc(&s.name));
        for (j, (sp, t)) in s.spaces.iter().enumerate() {
            if j == 0 {
                let _ = writeln!(x, "    <AdjacentSpaceId spaceIdRef=\"{sp}\"/>");
            } else {
                let _ = writeln!(
                    x,
                    "    <AdjacentSpaceId spaceIdRef=\"{sp}\" surfaceType=\"{t}\"/>"
                );
            }
        }
        let (w, h) = rect_size(&s.poly, s.tilt);
        let o = s.poly[0];
        let _ = writeln!(
            x,
            "    <RectangularGeometry>\n      <Azimuth>{}</Azimuth>\n{}      <Tilt>{}</Tilt>\n      <Width>{}</Width>\n      <Height>{}</Height>\n    </RectangularGeometry>",
            num(s.azimuth),
            point_xml(o, 6),
            num(s.tilt),
            num(w),
            num(h)
        );
        let _ = writeln!(
            x,
            "    <PlanarGeometry>\n{}    </PlanarGeometry>",
            loop_xml(&s.poly, 6)
        );
        for hl in &s.holes {
            let mut attrs = format!("id=\"{}\" openingType=\"{}\"", hl.id, hl.kind);
            if let Some(wt) = &hl.window_type {
                let _ = write!(attrs, " windowTypeIdRef=\"{wt}\"");
            }
            if let Some(c) = &hl.construction {
                let _ = write!(attrs, " constructionIdRef=\"{c}\"");
            }
            let _ = writeln!(
                x,
                "    <Opening {attrs}>\n      <Name>{}</Name>",
                esc(&hl.name)
            );
            let _ = writeln!(
                x,
                "      <RectangularGeometry>\n        <CartesianPoint>\n          <Coordinate>{}</Coordinate>\n          <Coordinate>{}</Coordinate>\n        </CartesianPoint>\n        <Width>{}</Width>\n        <Height>{}</Height>\n      </RectangularGeometry>",
                num(hl.rect[0]),
                num(hl.rect[1]),
                num(hl.rect[2]),
                num(hl.rect[3])
            );
            let _ = writeln!(
                x,
                "      <PlanarGeometry>\n{}      </PlanarGeometry>\n    </Opening>",
                loop_xml(&hl.poly, 8)
            );
        }
        x.push_str("  </Surface>\n");
    }
    x.push_str("</Campus>\n");
    x.push_str(&lib.out);
    x.push_str("</gbXML>\n");
    (x, warnings)
}

/// A Slovak postal code in an address: "040 01".
fn postal_code(address: &str) -> String {
    let digits: Vec<(usize, char)> = address.char_indices().collect();
    for i in 0..digits.len() {
        let rest: String = digits[i..].iter().map(|c| c.1).take(6).collect();
        let d: Vec<char> = rest
            .chars()
            .filter(|c| !c.is_whitespace())
            .take(5)
            .collect();
        let before_ok = i == 0 || !digits[i - 1].1.is_ascii_digit();
        if before_ok
            && d.len() == 5
            && d.iter().all(|c| c.is_ascii_digit())
            && rest.chars().take(3).all(|c| c.is_ascii_digit())
        {
            return format!(
                "{} {}",
                d[..3].iter().collect::<String>(),
                d[3..].iter().collect::<String>()
            );
        }
    }
    String::new()
}

/// Width and height of a surface: a wall's length and height, else the
/// sides of its bounding box.
fn rect_size(poly: &[DVec3], tilt: f64) -> (f64, f64) {
    if (tilt - 90.0).abs() < 1e-6 && poly.len() == 4 {
        return (
            poly[0].truncate().distance(poly[1].truncate()),
            (poly[2].z - poly[1].z).abs(),
        );
    }
    let (lo, hi) = poly.iter().fold(
        (DVec3::splat(f64::MAX), DVec3::splat(f64::MIN)),
        |(lo, hi), p| (lo.min(*p), hi.max(*p)),
    );
    (hi.x - lo.x, hi.y - lo.y)
}

fn point_xml(p: DVec3, indent: usize) -> String {
    let pad = " ".repeat(indent);
    format!(
        "{pad}<CartesianPoint>\n{pad}  <Coordinate>{}</Coordinate>\n{pad}  <Coordinate>{}</Coordinate>\n{pad}  <Coordinate>{}</Coordinate>\n{pad}</CartesianPoint>\n",
        num(p.x),
        num(p.y),
        num(p.z)
    )
}

fn loop_xml(poly: &[DVec3], indent: usize) -> String {
    let pad = " ".repeat(indent);
    let mut s = format!("{pad}<PolyLoop>\n");
    for p in poly {
        s.push_str(&point_xml(*p, indent + 2));
    }
    let _ = writeln!(s, "{pad}</PolyLoop>");
    s
}

fn num(v: f64) -> String {
    let r = (v * 1e4).round() / 1e4;
    let r = if r == 0.0 { 0.0 } else { r };
    format!("{r}")
}

fn esc(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || *c == '\t')
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            c => c.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(x: &str, s: &str) -> usize {
        x.matches(s).count()
    }

    #[test]
    fn the_starter_house_has_spaces_walls_floors_and_roofs() {
        let m = creabim_core::starter::starter_house();
        let (x, _) = export(&m);
        assert!(x.starts_with("<?xml"));
        assert!(count(&x, "<Space ") > 0, "spaces");
        assert!(count(&x, "surfaceType=\"ExteriorWall\"") > 3);
        assert!(count(&x, "<Opening ") > 0);
        assert!(x.contains("<U-value unit=\"WPerSquareMeterK\">"));
        // every reference has its target
        for (attr, tag) in [
            ("constructionIdRef", "Construction"),
            ("windowTypeIdRef", "WindowType"),
            ("spaceIdRef", "Space"),
        ] {
            for part in x.split(&format!("{attr}=\"")).skip(1) {
                let id = &part[..part.find('"').unwrap()];
                assert!(x.contains(&format!("<{tag} id=\"{id}\"")), "{attr} {id}");
            }
        }
    }

    #[test]
    fn build_ups_with_lambda_carry_u_and_layers_and_loggias_are_outside() {
        let mut doc = creabim_core::Document::from_model(creabim_core::starter::starter_house());
        let mut tx = creabim_core::Transaction::new("λ");
        let mats: Vec<creabim_core::Element> = doc
            .model()
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Material(_)))
            .cloned()
            .collect();
        for before in mats {
            let mut after = before.clone();
            if let ElementKind::Material(m) = &mut after.kind {
                m.lambda = Some(0.5);
            }
            tx = tx.replace(before, after);
        }
        doc.commit(tx).unwrap();
        let (x, _) = export(doc.model());
        assert!(
            count(&x, "<Conductivity unit=\"WPerMeterK\">0.5</Conductivity>") > 3,
            "{}",
            &x[x.find("<Material ").unwrap()..][..600]
        );
        let walls = x
            .split("<Construction ")
            .filter(|c| c.contains("Exterior wall"))
            .count();
        assert!(walls > 0);
        for c in x
            .split("<Construction ")
            .skip(1)
            .filter(|c| c.contains("Exterior wall"))
        {
            assert!(c.contains("<U-value"), "{c}");
            assert!(c.contains("<LayerId"), "{c}");
        }
        assert!(!x.contains("Loggia</Name>"));
        assert!(!x.contains("Terrace</Name>"));
    }

    #[test]
    fn postal_codes_are_found_in_addresses() {
        assert_eq!(postal_code("Hlavná 12, 040 01 Košice"), "040 01");
        assert_eq!(postal_code("Hlavná 12, 04001 Košice"), "040 01");
        assert_eq!(postal_code("Bez PSČ 12"), "");
    }
}
