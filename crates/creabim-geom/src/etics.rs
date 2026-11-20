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
        out.push(EticsCheck {
            assembly: aid,
            insulation: m.name.clone(),
            class: mat.fire.clone(),
            thickness: l.thickness,
            height: h,
            required,
            ok: meets(&mat.fire, required.split_whitespace().next().unwrap_or("")),
            barriers,
        });
    }
    out
}

/// The fire barriers made in the model's ETICS: the heights of the
/// bands, the material they are of, and for each wall that gets them
/// the index of its layer they are cut into.
#[derive(Clone, Debug, Default)]
pub struct Barriers {
    /// Bottom and top of each band, m, absolute.
    pub zs: Vec<(f64, f64)>,
    pub material: Option<ElementId>,
    pub walls: std::collections::HashMap<ElementId, usize>,
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

/// Where the barriers go, by the Weber table: none on one storey or
/// over 30 m (all of it must then be A2-s1,d0); the plinth one with its
/// top `plinth_top` over the ground; the first about the lowest floor
/// over the ground storey; above 12 m (or when asked) one about every
/// floor.
pub fn barrier_heights(model: &Model, set: &creabim_core::EticsBarriers) -> Vec<(f64, f64)> {
    let Some(fh) = crate::firezones::fire_height(model) else {
        return vec![];
    };
    if fh.above <= 1 || fh.height > 30.0 {
        return vec![];
    }
    let w = set.width().clamp(0.05, 1.0);
    let ground = fh.ground.unwrap_or(0.0);
    let first_z = model.level(fh.first).map_or(0.0, |l| l.elevation);
    let mut zs = vec![(ground + set.plinth_top() - w, ground + set.plinth_top())];
    // the floors over the first above-ground storey, as fire counts them
    let mut floors: Vec<f64> = creabim_core::levels::sorted(model)
        .into_iter()
        .filter(|(id, ..)| {
            model
                .iter()
                .any(|e| matches!(&e.kind, ElementKind::Wall(x) if x.level == *id))
        })
        .map(|(_, _, l)| l.elevation)
        .filter(|z| *z > first_z + 1e-6)
        .collect();
    floors.sort_by(f64::total_cmp);
    if !(set.every_floor || fh.height > 12.0) {
        floors.truncate(1);
    }
    zs.extend(floors.into_iter().map(|z| (z - w / 2.0, z + w / 2.0)));
    zs
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
        if b.material.is_some() {
            b.zs = barrier_heights(model, &set);
        }
        if !b.zs.is_empty() {
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
                if let Some(i) = combustible_layer(model, a) {
                    b.walls.insert(e.id, i);
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
