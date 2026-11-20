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
