//! Outer thermal insulation (ETICS) against the building's fire height:
//! the reaction to fire its insulation must have at least and the fire
//! barriers it then needs. Source: Weber, Technologický predpis ETICS,
//! s. 16 (tables "Požiadavky na triedu reakcie na oheň" and by building
//! category) and s. 17 (where A2-s1,d0 is a must), which follow
//! STN 73 0802; George 2026-11-20. Only the dwelling and family-house
//! category is given there; other buildings go by STN 73 0802.

use creabim_core::{ElementId, ElementKind, LayerFunction, Model};

/// A wall build-up used on outer walls whose insulation lies outside its
/// load-bearing layer, checked against the building's fire height.
#[derive(Clone, Debug, PartialEq)]
pub struct EticsCheck {
    pub assembly: ElementId,
    /// The insulation's material name and its reaction to fire.
    pub insulation: String,
    pub class: String,
    pub thickness: f64,
    /// The fire height it is checked against, m (0 for one storey).
    pub height: f64,
    /// The least reaction to fire the insulation must have.
    pub required: &'static str,
    /// Whether its class meets that.
    pub ok: bool,
    /// The fire barriers it needs, empty when none.
    pub barriers: &'static str,
    /// What the automatic barriers did about it: the mineral wool over
    /// 22.5 m, the first band brought under 7 m.
    pub note: String,
}

/// Non-combustible by the Weber manual (s. 16): A1 or A2-s1,d0.
pub fn noncombustible(class: &str) -> bool {
    let c = class.replace(' ', "").to_uppercase();
    c.starts_with("A1") || c.starts_with("A2-S1,D0")
}

/// The insulation's least class and the barriers by fire height h, m
/// (Weber s. 16, dwellings and family houses).
pub fn requirement(h: f64, combustible: bool) -> (&'static str, &'static str) {
    let required = if h > 30.0 {
        "A2-s1,d0"
    } else if h > 22.5 {
        "E do 22,5 m, nad 22,5 m A2-s1,d0"
    } else {
        "E"
    };
    let barriers = if !combustible || h > 30.0 || h <= 0.0 {
        // over 30 m all of it is A2-s1,d0; one storey needs none
        ""
    } else if h <= 12.0 {
        "soklová zábrana max. 600 mm nad terénom, prvá max. 7 m nad terénom; MW A2-s1,d0 šírky 200 mm (150–400)"
    } else {
        "soklová zábrana max. 600 mm nad terénom, prvá max. 7 m nad terénom, ďalšie od 12 m podľa PD; MW A2-s1,d0 šírky 200 mm (150–400)"
    };
    (required, barriers)
}

/// Whether a class meets the least one (only E and A2-s1,d0 are asked).
fn meets(class: &str, required: &str) -> bool {
    if required.starts_with("A2") {
        noncombustible(class)
    } else {
        // E or better: anything classed but F
        let c = class.trim().to_uppercase();
        !c.is_empty() && !c.starts_with('F')
    }
}

/// Every wall build-up on the model's outer walls with insulation outside
/// its load-bearing layer, checked once.
pub fn checks(model: &Model) -> Vec<EticsCheck> {
    let Some(fh) = crate::firezones::fire_height(model) else {
        return vec![];
    };
    let h = if fh.above <= 1 { 0.0 } else { fh.height };
    let mut out: Vec<EticsCheck> = vec![];
    for w in crate::firezones::outer_walls(model) {
        let Some(aid) = w.wall.assembly else { continue };
        if out.iter().any(|c| c.assembly == aid) {
            continue;
        }
        let Some(ElementKind::Assembly(a)) = model.get(aid).map(|e| &e.kind) else {
            continue;
        };
        // the layers outside the first load-bearing one
        let core = a
            .layers
            .iter()
            .position(|l| l.function == LayerFunction::Structure)
            .unwrap_or(0);
        let Some(l) = a.layers[..core]
            .iter()
            .find(|l| l.function == LayerFunction::Insulation)
        else {
            continue;
        };
        let Some(m) = l.material.and_then(|id| model.get(id)) else {
            continue;
        };
        let ElementKind::Material(mat) = &m.kind else {
            continue;
        };
        let combustible = !noncombustible(&mat.fire);
        let (required, barriers) = requirement(h, combustible);
        let made = crate::etics::barriers(model);
        let wall_has = made.walls.contains_key(&w.id);
        let mut notes: Vec<String> = vec![];
        let mut ok = meets(&mat.fire, required.split_whitespace().next().unwrap_or(""));
        if combustible && h > A2_OVER && h <= 30.0 {
            match made.heights.as_ref().and_then(|x| x.a2_from) {
                Some(z) if wall_has => notes.push(format!(
                    "nad +{} izolácia nahradená minerálnou vlnou A2-s1,d0",
                    format!("{z:.3}").replace('.', ",")
                )),
                // asked of it, and no barriers made: it does not meet
                _ => ok = false,
            }
        }
        if wall_has && made.heights.as_ref().is_some_and(|x| x.first_moved) {
            notes.push(format!(
                "prvá zábrana posunutá na {} m nad terénom (strop je vyššie)",
                FIRST_MOST
            ));
        }
        out.push(EticsCheck {
            assembly: aid,
            insulation: m.name.clone(),
            class: mat.fire.clone(),
            thickness: l.thickness,
            height: h,
            required,
            ok,
            barriers,
            note: notes.join("; "),
        });
    }
    out
}

/// The fire barriers made in the model's ETICS: the material they are
/// of and, for each wall that gets them, the layer they are cut into
/// and the bands.
#[derive(Clone, Debug, Default)]
pub struct Barriers {
    /// The heights of the bands running round the building (plinth,
    /// floors), m, absolute: bottom and top.
    pub zs: Vec<(f64, f64)>,
    pub material: Option<ElementId>,
    pub walls: std::collections::HashMap<ElementId, WallBarriers>,
    /// Where they were put, for the checks to tell.
    pub heights: Option<Heights>,
}

/// One wall's barriers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WallBarriers {
    /// The index of its burning insulation layer.
    pub layer: usize,
    /// Each band: from and to along the wall (from its start, m; a band
    /// round the building runs past both ends) and its bottom and top
    /// (absolute, m).
    pub boxes: Vec<[f64; 4]>,
}

impl WallBarriers {
    /// Whether a band runs the wall's whole length.
    pub fn continuous(b: &[f64; 4], length: f64) -> bool {
        b[0] < 0.0 && b[1] > length
    }
}

/// The ETICS layer of a wall's assembly: its first insulation outside
/// the first load-bearing layer, when it burns (not A1 or A2-s1,d0).
pub fn combustible_layer(model: &Model, a: &creabim_core::Assembly) -> Option<usize> {
    let core = a
        .layers
        .iter()
        .position(|l| l.function == LayerFunction::Structure)?;
    let (i, l) = a.layers[..core]
        .iter()
        .enumerate()
        .find(|(_, l)| l.function == LayerFunction::Insulation)?;
    let ElementKind::Material(m) = &model.get(l.material?)?.kind else {
        return None;
    };
    (!noncombustible(&m.fire)).then_some(i)
}

/// The mineral wool the barriers are of: the project's choice, else the
/// base's façade board, else any insulation of class A1.
fn barrier_material(model: &Model, set: &creabim_core::EticsBarriers) -> Option<ElementId> {
    if let Some(m) = set.material.filter(|m| model.get(*m).is_some()) {
        return Some(m);
    }
    let materials = || {
        model.iter().filter_map(|e| match &e.kind {
            ElementKind::Material(m) => Some((e, m)),
            _ => None,
        })
    };
    materials()
        .find(|(e, _)| e.name == "Minerálna vlna – fasádna doska")
        .or_else(|| {
            materials().find(|(e, m)| {
                noncombustible(&m.fire) && e.name.to_lowercase().contains("minerálna vlna")
            })
        })
        .map(|(e, _)| e.id)
}

/// A bottom and a top, m.
pub type Span = (f64, f64);

/// Where the barriers go.
#[derive(Clone, Debug, PartialEq)]
pub struct Heights {
    /// The plinth band.
    pub plinth: Span,
    /// The floors they go about, each with the floor under it; the first
    /// band's middle, which may sit under its floor (see `first_moved`).
    pub floors: Vec<(f64, f64)>,
    /// From where all of the insulation must be A2-s1,d0 (mineral wool):
    /// 22.5 m over the first above-ground floor, in a building up to 30 m.
    pub a2_from: Option<f64>,
    /// The first band brought down to 7 m over the ground: its floor is
    /// higher (Weber s. 16: the first barrier at most 7 m over the ground).
    pub first_moved: bool,
}

/// Where the barriers go, by the Weber table (s. 16): none on one storey
/// or over 30 m (all of it must then be A2-s1,d0). The plinth band with
/// its top `plinth_top` over the ground; the first about the lowest
/// floor over the ground storey, but at most 7 m over the ground; above
/// 12 m (or when asked) one about every floor; above 22.5 m mineral wool
/// all the way up.
pub fn barrier_heights(model: &Model, set: &creabim_core::EticsBarriers) -> Option<Heights> {
    let fh = crate::firezones::fire_height(model)?;
    if fh.above <= 1 || fh.height > 30.0 {
        return None;
    }
    let w = set.width().clamp(0.05, 1.0);
    let ground = fh.ground.unwrap_or(0.0);
    let first_z = model.level(fh.first).map_or(0.0, |l| l.elevation);
    let plinth = (ground + set.plinth_top() - w, ground + set.plinth_top());
    let mut all: Vec<f64> = creabim_core::levels::sorted(model)
        .into_iter()
        .filter(|(id, ..)| {
            model
                .iter()
                .any(|e| matches!(&e.kind, ElementKind::Wall(x) if x.level == *id))
        })
        .map(|(_, _, l)| l.elevation)
        .filter(|z| *z > first_z + 1e-6)
        .collect();
    all.sort_by(f64::total_cmp);
    let mut floors: Vec<(f64, f64)> = all
        .iter()
        .enumerate()
        .map(|(i, z)| (*z, if i == 0 { first_z } else { all[i - 1] }))
        .collect();
    if !(set.every_floor || fh.height > 12.0) {
        floors.truncate(1);
    }
    let mut first_moved = false;
    if let Some(f) = floors.first_mut() {
        let most = ground + FIRST_MOST - w / 2.0;
        if f.0 > most + 1e-6 {
            f.0 = most;
            first_moved = true;
        }
    }
    let a2_from = (fh.height > A2_OVER).then_some(first_z + A2_OVER);
    if let Some(a2) = a2_from {
        // the bands above it are in the mineral wool already
        floors.retain(|f| f.0 + w / 2.0 <= a2 + 1e-6);
    }
    Some(Heights {
        plinth,
        floors,
        a2_from,
        first_moved,
    })
}

/// The first barrier at most this far over the ground, m (Weber s. 16).
pub const FIRST_MOST: f64 = 7.0;
/// Over this fire height the insulation must be A2-s1,d0, m.
pub const A2_OVER: f64 = 22.5;

/// Whether some roof is carried by a structure that burns (a timber
/// truss): the walls under it then get a band at their top (atiková
/// zábrana, Weber s. 16).
fn roof_burns(model: &Model) -> bool {
    model.iter().any(|e| {
        matches!(e.kind, ElementKind::Roof(_))
            && creabim_core::assembly::assembly_of(model, e).is_some_and(|(_, a)| {
                a.layers.iter().any(|l| {
                    l.function == LayerFunction::Structure
                        && l.material
                            .and_then(|m| match &model.get(m)?.kind {
                                ElementKind::Material(m) => {
                                    Some(!m.fire.is_empty() && !noncombustible(&m.fire))
                                }
                                _ => None,
                            })
                            .unwrap_or(false)
                })
            })
    })
}

/// One wall's bands.
fn wall_boxes(
    model: &Model,
    id: ElementId,
    w: &creabim_core::Wall,
    set: &creabim_core::EticsBarriers,
    heights: &Heights,
    parapet: bool,
) -> Vec<[f64; 4]> {
    let (plinth, floors) = (heights.plinth, &heights.floors[..]);
    let wd = set.width().clamp(0.05, 1.0);
    let len = w.start.distance(w.end);
    let base = model.level(w.level).map_or(0.0, |l| l.elevation) + w.base_offset;
    let top = base + w.height;
    let stands = |z0: f64, z1: f64| z1 > base + 1e-6 && z0 < top - 1e-6;
    let whole = |z0: f64, z1: f64| [-1.0, len + 1.0, z0, z1];
    let mut out = vec![];
    if stands(plinth.0, plinth.1) {
        out.push(whole(plinth.0, plinth.1));
    }
    let openings = creabim_core::curtain::wall_openings(model, id);
    for &(z, under) in floors {
        if !set.over_openings {
            if stands(z - wd / 2.0, z + wd / 2.0) {
                out.push(whole(z - wd / 2.0, z + wd / 2.0));
            }
            continue;
        }
        // over each opening of the storey under the floor, 500 mm past
        // its sides; none over one of at most 500 x 500 mm (Weber s. 17)
        for o in &openings {
            let Some(c) = crate::wall::Cut::of(o, w) else {
                continue;
            };
            if c.u1 - c.u0 <= 0.5 + 1e-6 && c.v1 - c.v0 <= 0.5 + 1e-6 {
                continue;
            }
            let head = base + c.v1;
            if head <= under + 1e-6 || head > z + 1e-6 {
                continue;
            }
            let (z0, z1) = (head, (head + wd).min(top));
            if z1 > z0 + 1e-6 {
                out.push([c.u0 - 0.5, c.u1 + 0.5, z0, z1]);
            }
        }
    }
    // the parapet band under a roof that burns, at the top of the walls
    // of the last storey
    if parapet && floors.last().is_none_or(|f| base >= f.0 - 1e-6) && top - wd > base {
        out.push(whole(top - wd, top));
    }
    // over 22.5 m all of it mineral wool; in the storey where that
    // starts, at least 100 mm under the head of its openings (Weber s. 16)
    if let Some(a2) = heights.a2_from {
        if top > a2 + 1e-6 {
            let mut from = a2.max(base);
            if base <= a2 {
                for o in &openings {
                    if let Some(c) = crate::wall::Cut::of(o, w) {
                        from = from.min(base + c.v1 - 0.1);
                    }
                }
            }
            // a storey wholly over it: all of its insulation, also where it
            // laps past the wall's own base or top
            let from = if base > a2 { a2 } else { from.max(base - 0.01) };
            out.retain(|b| b[3] <= from + 1e-6);
            out.push([-1.0, len + 1.0, from, top + 10.0]);
        }
    }
    merge(out)
}

/// Bands at one height that touch or overlap as one: the cutter is a set
/// of separate boxes.
fn merge(mut boxes: Vec<[f64; 4]>) -> Vec<[f64; 4]> {
    boxes.sort_by(|a, b| {
        (a[2], a[3], a[0])
            .partial_cmp(&(b[2], b[3], b[0]))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut out: Vec<[f64; 4]> = vec![];
    for b in boxes {
        match out.last_mut() {
            Some(l)
                if (l[2] - b[2]).abs() < 1e-6
                    && (l[3] - b[3]).abs() < 1e-6
                    && b[0] <= l[1] + 1e-6 =>
            {
                l[1] = l[1].max(b[1]);
            }
            _ => out.push(b),
        }
    }
    // bands of different heights that overlap: the lower one stops under
    // the higher one where they meet is rare; keep the first of any two
    // that cross
    let mut kept: Vec<[f64; 4]> = vec![];
    for b in out {
        let crosses = kept.iter().any(|k| {
            b[0] < k[1] - 1e-6 && b[1] > k[0] + 1e-6 && b[2] < k[3] - 1e-6 && b[3] > k[2] + 1e-6
        });
        if !crosses {
            kept.push(b);
        }
    }
    kept
}

/// The model's ETICS fire barriers, worked out once per revision.
pub fn barriers(model: &Model) -> std::sync::Arc<Barriers> {
    use std::sync::{Arc, Mutex};
    static CACHE: Mutex<Option<(u64, u64, Arc<Barriers>)>> = Mutex::new(None);
    let key = (model as *const Model as u64, model.revision());
    if key.1 != 0 {
        if let Some((p, r, b)) = CACHE.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            if (*p, *r) == key {
                return b.clone();
            }
        }
    }
    let set = model
        .project_info()
        .map(|p| p.1.etics_barriers.clone())
        .unwrap_or_default();
    let mut b = Barriers::default();
    if !set.off {
        b.material = barrier_material(model, &set);
        let heights = b.material.and_then(|_| barrier_heights(model, &set));
        if let Some(heights) = heights {
            let wd = set.width().clamp(0.05, 1.0);
            b.zs.push(heights.plinth);
            if !set.over_openings {
                b.zs.extend(
                    heights
                        .floors
                        .iter()
                        .map(|f| (f.0 - wd / 2.0, f.0 + wd / 2.0)),
                );
            }
            b.heights = Some(heights.clone());
            let parapet = !set.no_parapet && roof_burns(model);
            for e in model.iter() {
                let ElementKind::Wall(w) = &e.kind else {
                    continue;
                };
                if w.no_fire_barriers {
                    continue;
                }
                let Some((_, a)) = creabim_core::assembly::assembly_of(model, e) else {
                    continue;
                };
                if let Some(layer) = combustible_layer(model, a) {
                    let boxes = wall_boxes(model, e.id, w, &set, &heights, parapet);
                    if !boxes.is_empty() {
                        b.walls.insert(e.id, WallBarriers { layer, boxes });
                    }
                }
            }
        }
    }
    let b = Arc::new(b);
    if key.1 != 0 {
        *CACHE.lock().unwrap_or_else(|e| e.into_inner()) = Some((key.0, key.1, b.clone()));
    }
    b
}

/// The edges of the fire barriers as an elevation from `start` to `end`
/// sees them: on the outer face of every wall turned to it, each band's
/// bottom and top a thin dashed line (they lie under the render).
pub fn elevation_lines(
    model: &Model,
    start: glam::DVec2,
    end: glam::DVec2,
    depth: f64,
) -> Vec<crate::plan::PlanItem> {
    let b = barriers(model);
    let frame = crate::section::SectionFrame::new(start, end, depth);
    let mut out = vec![];
    for (&id, wb) in b.walls.iter() {
        let Some(e) = model.get(id) else { continue };
        let ElementKind::Wall(w) = &e.kind else {
            continue;
        };
        let Some((_, a)) = creabim_core::assembly::assembly_of(model, e) else {
            continue;
        };
        let bands = creabim_core::assembly::wall_bands(a, w);
        let Some(first) = bands.first() else { continue };
        let d = w.direction();
        let n = glam::DVec2::new(-d.y, d.x);
        // the outer face, and which way it looks
        let s = if first.s0.abs() > first.s1.abs() {
            first.s0
        } else {
            first.s1
        };
        let outward = n * s.signum();
        if outward.dot(frame.look) > -0.1 {
            continue;
        }
        let len = w.start.distance(w.end);
        let at = |u: f64, z: f64| {
            let p = frame.project((w.start + d * u.clamp(0.0, len) + n * s).extend(0.0));
            (glam::DVec2::new(p.x, z), p.z)
        };
        let (_, near) = at(0.0, 0.0);
        let (_, far) = at(len, 0.0);
        if near.max(far) < 0.0 || near.min(far) > depth {
            continue;
        }
        let mut line = |p: glam::DVec2, q: glam::DVec2| {
            out.push(crate::plan::PlanItem::Line {
                id,
                pen: crate::plan::Pen::Hidden,
                a: p,
                b: q,
            })
        };
        // the openings, absolute: a band's edge stops at a door or window
        let base = model.level(w.level).map_or(0.0, |l| l.elevation) + w.base_offset;
        let holes: Vec<[f64; 4]> = creabim_core::curtain::wall_openings(model, id)
            .iter()
            .filter_map(|o| crate::wall::Cut::of(o, w))
            .map(|c| [c.u0, c.u1, base + c.v0, base + c.v1])
            .collect();
        let pieces = |u0: f64, u1: f64, z: f64| -> Vec<(f64, f64)> {
            let mut runs = vec![(u0.max(0.0), u1.min(len))];
            for h in holes.iter().filter(|h| z > h[2] + 1e-6 && z < h[3] - 1e-6) {
                runs = runs
                    .into_iter()
                    .flat_map(|(a, b)| {
                        let mut v = vec![];
                        if h[0] > a {
                            v.push((a, b.min(h[0])));
                        }
                        if h[1] < b {
                            v.push((a.max(h[1]), b));
                        }
                        v
                    })
                    .filter(|(a, b)| b > a)
                    .collect();
            }
            runs
        };
        for bx in &wb.boxes {
            // an edge past the wall (the mineral wool of a storey over
            // 22.5 m reaches beyond it) is not drawn
            for z in [bx[2], bx[3]]
                .into_iter()
                .filter(|z| *z >= base - 1e-6 && *z <= base + w.height + 1e-6)
            {
                for (a, b) in pieces(bx[0], bx[1], z) {
                    line(at(a, z).0, at(b, z).0);
                }
            }
            let (p0, _) = at(bx[0], bx[2]);
            let (p1, _) = at(bx[1], bx[2]);
            let (q0, _) = at(bx[0], bx[3]);
            let (q1, _) = at(bx[1], bx[3]);
            // a band over an opening shows its ends too
            if !WallBarriers::continuous(bx, len) {
                if bx[0] >= 0.0 {
                    line(p0, q0);
                }
                if bx[1] <= len {
                    line(p1, q1);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_weber_table_by_fire_height() {
        assert_eq!(requirement(0.0, true), ("E", ""));
        assert!(requirement(6.0, true).1.starts_with("soklová"));
        assert!(!requirement(6.0, true).1.contains("12 m"));
        assert!(requirement(18.0, true).1.contains("od 12 m"));
        assert_eq!(requirement(35.0, true).0, "A2-s1,d0");
        assert_eq!(requirement(6.0, false).1, "");
        assert!(noncombustible("A1") && noncombustible("A2-s1, d0"));
        assert!(!noncombustible("E") && !noncombustible("A2-s2,d0"));
        assert!(meets("E", "E") && !meets("E", "A2-s1,d0") && meets("A1", "A2-s1,d0"));
    }

    fn eps_house() -> Model {
        let mut m = creabim_core::starter::starter_house();
        creabim_core::skbase::add_base(&m).apply(&mut m).unwrap();
        let asm = creabim_core::assembly::assemblies(&m)
            .into_iter()
            .find(|(_, n, _)| *n == "Obvodová – PORFIX P2-440 300 + EPS šedý 150")
            .map(|(id, _, _)| id)
            .unwrap();
        let walls: Vec<_> = m
            .iter()
            .filter(|e| matches!(&e.kind, ElementKind::Wall(w) if w.assembly.is_some()))
            .cloned()
            .collect();
        let mut tx = creabim_core::Transaction::new("t");
        for e in walls {
            let mut after = e.clone();
            if let ElementKind::Wall(w) = &mut after.kind {
                // the outer ones: those whose assembly has insulation
                let outer = combustible_layer(
                    &m,
                    match &m.get(w.assembly.unwrap()).unwrap().kind {
                        ElementKind::Assembly(a) => a,
                        _ => unreachable!(),
                    },
                )
                .is_some()
                    || m.get(w.assembly.unwrap())
                        .is_some_and(|a| a.name.to_lowercase().contains("ext"));
                if outer {
                    w.assembly = Some(asm);
                }
            }
            tx = tx.replace(e, after);
        }
        tx.apply(&mut m).unwrap();
        m
    }

    /// Bands of mineral wool are cut out of the EPS of the outer walls:
    /// the plinth one and the first about the upper floor; the take-off
    /// counts less EPS and the wool on its own line.
    #[test]
    fn barriers_are_wool_cut_out_of_the_eps() {
        let m = eps_house();
        let b = barriers(&m);
        assert_eq!(b.zs.len(), 2, "{:?}", b.zs);
        assert!(!b.walls.is_empty());
        let wool = b.material.unwrap();
        assert_eq!(m.get(wool).unwrap().name, "Minerálna vlna – fasádna doska");
        let lines = crate::takeoff::lines(&m);
        let v = |id: ElementId| -> f64 {
            lines
                .iter()
                .filter(|l| l.material == Some(id))
                .map(|l| l.quantity.volume)
                .sum()
        };
        let w = v(wool);
        assert!(w > 0.1, "wool {w}");
        // two bands 0.2 m high and 0.15 m thick round about the house
        let length: f64 = b
            .walls
            .keys()
            .filter_map(|id| match &m.get(*id)?.kind {
                ElementKind::Wall(w) => Some(w.start.distance(w.end)),
                _ => None,
            })
            .sum();
        assert!(
            w < length * 2.0 * 0.2 * 0.15 * 1.3,
            "wool {w} length {length}"
        );
        // turned off: no wool
        let mut off = m.clone();
        let mut info = off.project_info().map(|p| p.1.clone()).unwrap_or_default();
        info.etics_barriers.off = true;
        creabim_core::set_project(&off, info)
            .apply(&mut off)
            .unwrap();
        assert!(barriers(&off).walls.is_empty());
        // an elevation draws their edges dashed on the faces it sees
        let elevation = m
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::View(v) => match &v.kind {
                    creabim_core::ViewKind::Section {
                        start,
                        end,
                        depth,
                        cut: false,
                    } => Some((*start, *end, *depth)),
                    _ => None,
                },
                _ => None,
            })
            .unwrap();
        let lines = elevation_lines(&m, elevation.0, elevation.1, elevation.2);
        assert!(!lines.is_empty());
        assert!(lines.iter().all(|l| matches!(
            l,
            crate::plan::PlanItem::Line {
                pen: crate::plan::Pen::Hidden,
                ..
            }
        )));
    }

    /// The house with its upper storey (and the walls on it) lifted to
    /// `z`, as if there were storeys between.
    fn lifted(z: f64) -> Model {
        let mut m = eps_house();
        let lv = creabim_core::levels::sorted(&m);
        let (id, ..) = lv
            .iter()
            .rfind(|(id, ..)| {
                m.iter()
                    .any(|e| matches!(&e.kind, ElementKind::Wall(w) if w.level == *id))
            })
            .unwrap()
            .clone();
        let e = m.get(id).unwrap().clone();
        let mut after = e.clone();
        if let ElementKind::Level(l) = &mut after.kind {
            l.elevation = z;
        }
        creabim_core::Transaction::new("t")
            .replace(e, after)
            .apply(&mut m)
            .unwrap();
        m
    }

    #[test]
    fn a_first_barrier_higher_than_7_m_moves_down() {
        let m = lifted(8.0);
        let h = barrier_heights(&m, &Default::default()).unwrap();
        assert!(h.first_moved);
        let f = h.floors[0].0;
        assert!((f - (FIRST_MOST - 0.1)).abs() < 0.05, "{f}");
        let c = checks(&m);
        assert!(c[0].note.contains("posunutá na 7 m"), "{c:?}");
        // a usual house keeps it on the ceiling
        let h = barrier_heights(&eps_house(), &Default::default()).unwrap();
        assert!(!h.first_moved);
    }

    #[test]
    fn eps_above_22_5_m_turns_to_wool() {
        let m = lifted(25.0);
        let h = barrier_heights(&m, &Default::default()).unwrap();
        let a2 = h.a2_from.unwrap();
        assert!((a2 - A2_OVER).abs() < 0.5, "{a2}");
        let b = barriers(&m);
        let wool = b.material.unwrap();
        // the walls of the top storey are all wool
        let top: Vec<_> = b
            .walls
            .iter()
            .filter(|(_, wb)| wb.boxes.iter().any(|x| x[3] > 25.5))
            .collect();
        assert!(!top.is_empty());
        let lines = crate::takeoff::lines(&m);
        let eps_up: f64 = lines
            .iter()
            .filter(|l| top.iter().any(|(id, _)| l.element == **id))
            .filter(|l| l.material.is_some() && l.material != Some(wool))
            .filter(|l| {
                m.get(l.material.unwrap())
                    .is_some_and(|e| e.name.to_lowercase().contains("eps"))
            })
            .map(|l| l.quantity.volume)
            .sum();
        assert!(eps_up < 1e-3, "eps left up there: {eps_up}");
        let c = checks(&m);
        assert!(c[0].note.contains("minerálnou vlnou"), "{c:?}");
        assert!(c[0].ok, "{c:?}");
    }

    #[test]
    fn an_eps_facade_of_a_two_storey_house_needs_barriers() {
        let mut m = creabim_core::starter::starter_house();
        creabim_core::skbase::add_base(&m).apply(&mut m).unwrap();
        let asm = creabim_core::assembly::assemblies(&m)
            .into_iter()
            .find(|(_, n, _)| *n == "Obvodová – PORFIX P2-440 300 + EPS šedý 150")
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
        let c = checks(&m);
        assert_eq!(c.len(), 1, "{c:?}");
        assert_eq!(c[0].class, "E");
        assert!(c[0].ok);
        assert!(c[0].height > 0.0);
        assert!(c[0].barriers.starts_with("soklová"));
    }
}
