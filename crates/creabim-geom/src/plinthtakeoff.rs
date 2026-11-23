//! The take-off of the walls' plinths (sokel, [`creabim_core::plinth`]):
//! its XPS, membrane and render measured on the pieces the 3D view
//! builds, each on its own line under "Plinths" (they are priced apart
//! from the facade), and the work they come with, from George's details
//! of a recessed and a flush plinth (2026-11-22): the XPS glued, the
//! base coat with its glass-fibre mesh and the primer over the render's
//! face, the plinth (starter) profile along its top, and below the
//! finished ground the dimpled protective foil and the extra protection
//! of the waterproofing to 50 mm over the ground.

use crate::mesh::TriMesh;
use crate::scene::{closed_volume, SceneItem, PLINTH_MEMBRANE};
use creabim_core::assembly::{wall_bands, Band};
use creabim_core::takeoff::{Line, Quantity};
use creabim_core::{assembly_of, ElementId, ElementKind, LayerFunction as F, Model};
use glam::DVec2;
use std::collections::HashMap;

/// How far over the finished ground the waterproofing is protected (the
/// details' 50 mm).
pub const PROTECTION_OVER_GROUND: f64 = 0.05;

/// What a plinth wall is built of, to tell its pieces from the facade's.
struct Wall {
    level: ElementId,
    /// The plinth's top, highest of its steps.
    top: f64,
    /// The heights of its steps' tops.
    tops: Vec<f64>,
    lap: f64,
    /// The pieces' thickness: XPS, render.
    xps: f64,
    render: f64,
    insulation: usize,
    finish: Option<usize>,
    /// Along the wall: (share of its length, finished ground over the
    /// plinth's bottom), one for each step of its top.
    below: Vec<(f64, f64)>,
}

fn walls(model: &Model) -> HashMap<ElementId, Wall> {
    let mut out = HashMap::new();
    for e in model.iter() {
        let ElementKind::Wall(w) = &e.kind else {
            continue;
        };
        let Some(p) = creabim_core::plinth::of_wall(model, w) else {
            continue;
        };
        let Some((_, a)) = assembly_of(model, e) else {
            continue;
        };
        let bands: Vec<Band> = wall_bands(a, w);
        let (Some(first), Some(last)) = (bands.first(), bands.last()) else {
            continue;
        };
        let out_dir = if first.s0 + first.s1 < last.s0 + last.s1 {
            -1.0
        } else {
            1.0
        };
        let Some(ins) = bands.iter().find(|b| b.function == F::Insulation).copied() else {
            continue;
        };
        let inner = if out_dir < 0.0 { ins.s1 } else { ins.s0 };
        // the finish outside the insulation: the render
        let finish = bands
            .iter()
            .find(|b| {
                b.function == F::Finish
                    && if out_dir < 0.0 {
                        b.s1 <= inner + 1e-9
                    } else {
                        b.s0 >= inner - 1e-9
                    }
            })
            .copied();
        let tops = creabim_core::plinth::tops(model, w, p);
        let top = tops.iter().map(|t| t.2).fold(f64::MIN, f64::max);
        let bottom = creabim_core::plinth::base_z(model, w)
            - creabim_core::plinth::below_base(model, e, w, p);
        // the ground along the outer face, step by step
        let len = w.length().max(1e-9);
        let d = w.direction();
        let n = DVec2::new(d.y, -d.x) * if w.flip { -1.0 } else { 1.0 };
        let off = n * (w.thickness / 2.0);
        let below = tops
            .iter()
            .map(|&(u0, u1, z)| {
                let a = w.start + d * u0 + off;
                let b = w.start + d * u1 + off;
                let ground = creabim_core::plinth::ground_along(model, a, b).unwrap_or(z - p.above);
                ((u1 - u0) / len, ground - bottom)
            })
            .collect();
        out.insert(
            e.id,
            Wall {
                level: w.level,
                top,
                tops: tops.iter().map(|t| t.2).collect(),
                lap: p.lap.max(0.0),
                xps: p.thickness.clamp(0.005, ins.s1 - ins.s0 + 0.2),
                render: finish.map_or(0.0, |b| b.s1 - b.s0),
                insulation: ins.index,
                finish: finish.map(|b| b.index),
                below,
            },
        );
    }
    out
}

/// The area of a mesh's level faces looking up at one of the heights
/// `at` (a plinth's top: not the bottom of a door cut out of it), and of
/// those looking down at its lowest point (its bottom, which no opening
/// reaches).
fn level_faces(m: &TriMesh, at: &[f64]) -> (f64, f64) {
    let lowest = m
        .positions
        .iter()
        .map(|p| p[2] as f64)
        .fold(f64::MAX, f64::min);
    let (mut up, mut down) = (0.0, 0.0);
    for t in m.indices.chunks_exact(3) {
        let p = |i: u32| glam::Vec3::from(m.positions[i as usize]).as_dvec3();
        let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
        let n = (b - a).cross(c - a);
        let area = n.length() / 2.0;
        if area <= 1e-12 {
            continue;
        }
        let z = (a.z + b.z + c.z) / 3.0;
        let nz = n.z / (2.0 * area);
        if nz > 0.99 && at.iter().any(|h| (z - h).abs() < 0.003) {
            up += area;
        } else if nz < -0.99 && (z - lowest).abs() < 0.003 {
            down += area;
        }
    }
    (up, down)
}

/// A plinth piece's wall, layer, function and material.
type PieceKey = (ElementId, usize, u8, Option<ElementId>);

/// The pieces of the plinths in `scene`: by element, function and
/// material, their volume, the area of their top and of their bottom. The
/// rest of the scene is the facade's.
#[derive(Default)]
pub struct Pieces {
    walls: HashMap<ElementId, Wall>,
    pieces: HashMap<PieceKey, (f64, f64, f64)>,
}

fn code(f: F) -> u8 {
    match f {
        F::Insulation => 0,
        F::Membrane => 1,
        _ => 2,
    }
}

impl Pieces {
    pub fn new(model: &Model) -> Self {
        Self {
            walls: walls(model),
            pieces: HashMap::new(),
        }
    }

    /// Takes `item` when it is a piece of a plinth; true when it did.
    pub fn take(&mut self, item: &SceneItem) -> bool {
        let Some(w) = self.walls.get(&item.id) else {
            return false;
        };
        let Some(l) = item.layer else {
            return false;
        };
        let ours = match l.function {
            F::Insulation | F::Membrane => l.index == w.insulation,
            F::Finish => Some(l.index) == w.finish,
            _ => false,
        };
        if !ours {
            return false;
        }
        let Some((_, hi)) = item.mesh.bounds() else {
            return false;
        };
        let reach = w.top
            + if l.function == F::Membrane {
                w.lap
            } else {
                0.0
            };
        if hi.z as f64 > reach + 1e-3 {
            return false;
        }
        let e = self
            .pieces
            .entry((item.id, l.index, code(l.function), l.material))
            .or_default();
        let (up, down) = level_faces(&item.mesh, &w.tops);
        e.0 += closed_volume(&item.mesh);
        e.1 += up;
        e.2 += down;
        true
    }

    /// The plinths' lines: their materials, then the work by area and by
    /// length.
    pub fn lines(&self) -> Vec<Line> {
        let mut keys: Vec<_> = self.pieces.keys().copied().collect();
        keys.sort_by_key(|k| (k.0, k.1, k.2));
        let mut out = vec![];
        // by wall: XPS area, render area, its top's length (no doors), its
        // face's length (at the bottom)
        let mut work: Vec<(ElementId, f64, f64, f64, f64)> = vec![];
        for k @ (id, index, f, material) in keys {
            let (v, up, down) = self.pieces[&k];
            let w = &self.walls[&id];
            let (function, t) = match f {
                0 => (F::Insulation, w.xps),
                1 => (F::Membrane, PLINTH_MEMBRANE),
                _ => (F::Finish, w.render),
            };
            if v <= 1e-9 || t <= 1e-6 {
                continue;
            }
            let area = v / t;
            let i = match work.iter().position(|x| x.0 == id) {
                Some(i) => i,
                None => {
                    work.push((id, 0.0, 0.0, 0.0, 0.0));
                    work.len() - 1
                }
            };
            match function {
                F::Insulation => {
                    work[i].1 += area;
                    // the top's length, where there is no render to tell it
                    if w.finish.is_none() {
                        work[i].3 += up / t;
                        work[i].4 += down / t;
                    }
                }
                F::Finish => {
                    work[i].2 += area;
                    work[i].3 += up / t;
                    work[i].4 += down / t;
                }
                _ => {}
            }
            out.push(Line {
                element: id,
                category: "Plinths",
                level: Some(w.level),
                material,
                layer: Some((index, function)),
                thickness: t,
                quantity: Quantity {
                    volume: v,
                    area,
                    ..Default::default()
                },
                perimeter: 0.0,
                label: None,
            });
        }
        let tr = creabim_core::words::tr;
        for (id, xps, render, top, face) in work {
            let w = &self.walls[&id];
            let line = |label: &str, area: f64, length: f64| Line {
                element: id,
                category: "Plinths",
                level: Some(w.level),
                material: None,
                layer: None,
                thickness: 0.0,
                quantity: Quantity {
                    area,
                    length,
                    ..Default::default()
                },
                perimeter: 0.0,
                label: Some(tr(label)),
            };
            if xps > 1e-6 {
                out.push(line("Plinth: XPS boards glued", xps, 0.0));
            }
            if render > 1e-6 {
                out.push(line("Plinth: base coat with glass-fibre mesh", render, 0.0));
                out.push(line("Plinth: primer", render, 0.0));
            }
            if top > 1e-6 {
                out.push(line("Plinth: starter profile", 0.0, top));
            }
            if face > 1e-6 {
                // under the ground: the face's length shared out by step
                let under = |over: f64| -> f64 {
                    w.below
                        .iter()
                        .map(|(share, h)| share * face * (h + over).max(0.0))
                        .sum()
                };
                let foil = under(0.0);
                if foil > 1e-6 {
                    out.push(line(
                        "Plinth: dimpled protective foil below ground",
                        foil,
                        0.0,
                    ));
                }
                let guard = under(PROTECTION_OVER_GROUND);
                if guard > 1e-6 {
                    out.push(line(
                        "Plinth: waterproofing protection to 50 mm over the ground",
                        guard,
                        0.0,
                    ));
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use creabim_core::plinth::Plinth;
    use creabim_core::{
        AssemblyUse, Document, Element, ElementId, ElementKind, Layer, LayerFunction as F,
        Material, Transaction,
    };

    /// George 2026-11-22: the plinth's XPS, membrane and render on lines of
    /// their own, the XPS 300 mm high along the faces its starter profile
    /// runs on, nothing under the ground where there is none.
    #[test]
    fn a_plinth_is_counted_on_its_own_lines() {
        let mut doc = Document::from_model(creabim_core::sample::layered_house());
        let mat = |n: &str| Element::new(n, ElementKind::Material(Material::default()));
        let (xps, foil, sokel, eps) =
            (mat("XPS"), mat("Fólia"), mat("Soklová omietka"), mat("EPS"));
        let (xid, fid, sid, eid) = (xps.id, foil.id, sokel.id, eps.id);
        doc.commit(
            Transaction::new("m")
                .insert(xps)
                .insert(foil)
                .insert(sokel)
                .insert(eps),
        )
        .unwrap();
        let ids: Vec<ElementId> = doc.model().iter().map(|e| e.id).collect();
        for id in ids {
            if let ElementKind::Assembly(a) = doc.model().get(id).unwrap().kind.clone() {
                if a.usage != AssemblyUse::Wall {
                    continue;
                }
                doc.edit(id, "plinth", |e| {
                    if let ElementKind::Assembly(a) = &mut e.kind {
                        a.layers = vec![
                            Layer::new(F::Finish, None, 0.005),
                            Layer::new(F::Insulation, Some(eid), 0.15),
                            Layer::new(F::Structure, None, 0.25),
                            Layer::new(F::Finish, None, 0.015),
                        ];
                        let mut p = Plinth::new(0.12);
                        p.insulation = Some(xid);
                        p.membrane = Some(fid);
                        p.render = Some(sid);
                        a.plinth = Some(p);
                    }
                })
                .unwrap();
            }
        }
        let m = doc.model();
        let lines = crate::takeoff::lines(m);
        let sum = |f: &dyn Fn(&creabim_core::takeoff::Line) -> bool,
                   q: fn(&creabim_core::takeoff::Quantity) -> f64|
         -> f64 { lines.iter().filter(|l| f(l)).map(|l| q(&l.quantity)).sum() };
        let plinth = |mat: ElementId| {
            move |l: &creabim_core::takeoff::Line| {
                l.category == "Plinths" && l.material == Some(mat)
            }
        };
        let xps = sum(&plinth(xid), |q| q.area);
        let render = sum(&plinth(sid), |q| q.area);
        let membrane = sum(&plinth(fid), |q| q.area);
        let label =
            |t: &'static str| move |l: &creabim_core::takeoff::Line| l.label.as_deref() == Some(t);
        let profile = sum(&label("Plinth: starter profile"), |q| q.length);
        assert!(profile > 10.0, "profile {profile}");
        // 300 mm high along the faces
        for (what, a) in [("xps", xps), ("render", render), ("membrane", membrane)] {
            let h = a / profile;
            assert!((h - 0.3).abs() < 0.03, "{what} {a} over {profile}: {h}");
        }
        assert!((sum(&label("Plinth: XPS boards glued"), |q| q.area) - xps).abs() < 1e-9);
        assert!((sum(&label("Plinth: primer"), |q| q.area) - render).abs() < 1e-9);
        // no ground: nothing below it
        assert_eq!(
            sum(
                &label("Plinth: dimpled protective foil below ground"),
                |q| q.area
            ),
            0.0
        );
        // the facade's lines hold none of it
        assert_eq!(
            sum(&|l| l.category == "Walls" && l.material == Some(xid), |q| q
                .volume),
            0.0
        );
    }
}
