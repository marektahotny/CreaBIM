//! Thermal transmittance U of a layered build-up (walls, floors, roofs),
//! by STN EN ISO 6946 / STN 73 0540-2: the layers' thermal resistances
//! R = d / λ added to the surface resistances Rsi and Rse,
//! U = 1 / (Rsi + Σ d/λ + Rse).
//!
//! The surface resistances are those the heat loss calculation already
//! counts with (STN EN ISO 6946, tab. 7): Rsi 0,10 for heat flowing up,
//! 0,13 across, 0,17 down; Rse 0,04 outside, Rse = Rsi towards another
//! room and 0 against the ground.
//!
//! A layer of function "Air gap" is a ventilated cavity (with battens
//! as its material or none): it and the layers outside it are left out
//! and the surface outside counts as an inner one (STN EN ISO 6946,
//! well-ventilated air layer), as PilonDesigner leaves out its `vent`
//! layers. A counted layer whose material has no λ, or that has no
//! material, leaves U unknown and is named so it can be filled in.

use crate::assembly::{Assembly, AssemblyUse, LayerFunction};
use crate::{Element, ElementKind, Model};
use serde::{Deserialize, Serialize};

/// Which way heat flows through the build-up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HeatFlow {
    /// Upwards: roofs, ceilings under a cold attic.
    Up,
    /// Across: walls.
    Horizontal,
    /// Downwards: floors over outside air or the ground.
    Down,
}

impl HeatFlow {
    pub const ALL: [HeatFlow; 3] = [HeatFlow::Up, HeatFlow::Horizontal, HeatFlow::Down];

    pub fn label(self) -> &'static str {
        match self {
            HeatFlow::Up => "Heat flow upwards",
            HeatFlow::Horizontal => "Heat flow horizontal",
            HeatFlow::Down => "Heat flow downwards",
        }
    }

    /// Rsi, m²K/W (STN EN ISO 6946, tab. 7).
    pub fn rsi(self) -> f64 {
        match self {
            HeatFlow::Up => 0.10,
            HeatFlow::Horizontal => 0.13,
            HeatFlow::Down => 0.17,
        }
    }
}

/// What lies on the outer side of the build-up.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Outside {
    /// Outside air: Rse 0,04.
    Exterior,
    /// Another room, heated or not: Rse = Rsi.
    Interior,
    /// The ground: Rse 0 (the ground's own resistance not counted).
    Ground,
}

impl Outside {
    pub const ALL: [Outside; 3] = [Outside::Exterior, Outside::Interior, Outside::Ground];

    pub fn label(self) -> &'static str {
        match self {
            Outside::Exterior => "Outside air",
            Outside::Interior => "Another room",
            Outside::Ground => "Ground contact",
        }
    }

    /// Rse, m²K/W, for heat flowing `flow`.
    pub fn rse(self, flow: HeatFlow) -> f64 {
        match self {
            Outside::Exterior => 0.04,
            Outside::Interior => flow.rsi(),
            Outside::Ground => 0.0,
        }
    }
}

/// The boundary conditions an assembly's U is counted for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Boundary {
    pub flow: HeatFlow,
    pub outside: Outside,
}

impl Boundary {
    /// What a build-up of `usage` usually is: an outside wall, a floor
    /// over outside air, a roof.
    pub fn default_for(usage: AssemblyUse) -> Boundary {
        let flow = match usage {
            AssemblyUse::Wall => HeatFlow::Horizontal,
            AssemblyUse::Floor => HeatFlow::Down,
            AssemblyUse::Roof => HeatFlow::Up,
        };
        Boundary {
            flow,
            outside: Outside::Exterior,
        }
    }

    /// What `a` is counted for: its own setting, else what the Slovak
    /// base wrote in its description (PilonDesigner's boundary: an inner
    /// or a ground-contact build-up), else its use's usual.
    pub fn of(a: &Assembly) -> Boundary {
        if let Some(b) = a.thermal {
            return b;
        }
        let mut b = Boundary::default_for(a.usage);
        if a.description.contains("vnútorná konštrukcia") {
            b.outside = Outside::Interior;
        } else if a.description.contains("konštrukcia pri teréne") {
            b.outside = Outside::Ground;
        }
        b
    }

    pub fn rsi(self) -> f64 {
        self.flow.rsi()
    }

    pub fn rse(self) -> f64 {
        self.outside.rse(self.flow)
    }

    /// Whether the outer side is the last layer: layers run from the
    /// exterior of a wall and from the top of a slab or roof, so heat
    /// flowing down leaves through the underside.
    fn outside_last(self) -> bool {
        self.flow == HeatFlow::Down
    }
}

/// One layer's share.
#[derive(Clone, Debug, PartialEq)]
pub struct LayerR {
    pub index: usize,
    pub name: String,
    /// m.
    pub thickness: f64,
    /// W/(m·K).
    pub lambda: Option<f64>,
    /// m²K/W; none when its λ is unknown or it doesn't count.
    pub r: Option<f64>,
    /// Outside a ventilated air gap, or the gap itself: not counted.
    pub skipped: bool,
}

/// The U of a build-up and how it was reached.
#[derive(Clone, Debug, PartialEq)]
pub struct Thermal {
    pub boundary: Boundary,
    pub layers: Vec<LayerR>,
    /// Surface resistances used, m²K/W (Rse is Rsi past a ventilated
    /// gap).
    pub rsi: f64,
    pub rse: f64,
    /// Σ d/λ of the counted layers, m²K/W.
    pub r: f64,
    /// Rsi + R + Rse, m²K/W.
    pub r_total: f64,
    /// W/(m²K); none while a counted layer's λ is unknown.
    pub u: Option<f64>,
}

impl Thermal {
    /// The layers holding U back.
    pub fn missing(&self) -> Vec<&LayerR> {
        self.layers
            .iter()
            .filter(|l| !l.skipped && l.r.is_none())
            .collect()
    }
}

fn lambda_of(model: &Model, m: crate::ElementId) -> (String, Option<f64>) {
    match model.get(m) {
        Some(e) => match &e.kind {
            ElementKind::Material(mat) => (e.name.clone(), mat.lambda.filter(|x| *x > 0.0)),
            _ => (e.name.clone(), None),
        },
        None => (String::new(), None),
    }
}

/// The U of assembly `a` for `boundary` (its own when none).
pub fn of_assembly(model: &Model, a: &Assembly, boundary: Option<Boundary>) -> Thermal {
    let b = boundary.unwrap_or_else(|| Boundary::of(a));
    let n = a.layers.len();
    // the ventilated gap nearest the inside cuts off all beyond it
    let order: Vec<usize> = if b.outside_last() {
        (0..n).collect()
    } else {
        (0..n).rev().collect()
    };
    let mut cut = None;
    for &i in &order {
        let l = &a.layers[i];
        if l.function == LayerFunction::AirGap && l.thickness > 0.0 {
            cut = Some(i);
            break;
        }
    }
    let beyond = |i: usize| match cut {
        None => false,
        Some(c) if b.outside_last() => i >= c,
        Some(c) => i <= c,
    };
    let mut layers = vec![];
    let mut r = 0.0;
    let mut known = true;
    for (i, l) in a.layers.iter().enumerate() {
        let (name, lambda) = match l.material {
            Some(m) => lambda_of(model, m),
            None => (l.function.label().to_string(), None),
        };
        let skipped = beyond(i) || l.thickness <= 0.0;
        let lr = if skipped {
            None
        } else {
            lambda.map(|x| l.thickness / x)
        };
        if !skipped {
            match lr {
                Some(x) => r += x,
                None => known = false,
            }
        }
        layers.push(LayerR {
            index: i,
            name,
            thickness: l.thickness,
            lambda,
            r: lr,
            skipped,
        });
    }
    let rsi = b.rsi();
    // past a ventilated gap the outer surface is sheltered like an
    // inner one
    let rse = if cut.is_some() { b.flow.rsi() } else { b.rse() };
    let r_total = rsi + r + rse;
    Thermal {
        boundary: b,
        layers,
        rsi,
        rse,
        r,
        r_total,
        u: (known && r_total > 0.0).then(|| 1.0 / r_total),
    }
}

/// The U of a wall, slab or roof: of its assembly, or of its one
/// material through its thickness. None for other elements and for
/// those of no material.
pub fn of_element(model: &Model, e: &Element) -> Option<Thermal> {
    if let Some((_, a)) = crate::assembly::assembly_of(model, e) {
        return Some(of_assembly(model, a, None));
    }
    let usage = AssemblyUse::of(&e.kind)?;
    let (material, d) = match &e.kind {
        ElementKind::Wall(w) => (w.material, w.thickness),
        ElementKind::Slab(s) => (s.material, s.thickness),
        ElementKind::Roof(r) => (r.material, r.thickness),
        _ => return None,
    };
    let one = Assembly::new(
        usage,
        vec![crate::assembly::Layer::new(
            LayerFunction::Structure,
            Some(material?),
            d,
        )],
    );
    Some(of_assembly(model, &one, None))
}

/// "0,245" — U or R as the editor and the lists show it.
pub fn shown(v: f64) -> String {
    format!("{v:.3}").replace('.', ",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembly::Layer;
    use crate::element::Material;

    struct M(crate::Document);
    impl M {
        fn model(&self) -> &Model {
            self.0.model()
        }
    }
    fn new_model() -> M {
        M(crate::Document::new())
    }
    fn mat(model: &mut M, name: &str, lambda: Option<f64>) -> crate::ElementId {
        let m = Material {
            lambda,
            ..Default::default()
        };
        let e = Element::new(name, ElementKind::Material(m));
        let id = e.id;
        model
            .0
            .commit(crate::Transaction::new("m").insert(e))
            .unwrap();
        id
    }

    #[test]
    fn a_wall_adds_up_its_layers() {
        let mut model = new_model();
        let eps = mat(&mut model, "EPS", Some(0.04));
        let brick = mat(&mut model, "Brick", Some(0.2));
        let a = Assembly::new(
            AssemblyUse::Wall,
            vec![
                Layer::new(LayerFunction::Insulation, Some(eps), 0.1),
                Layer::new(LayerFunction::Structure, Some(brick), 0.3),
            ],
        );
        let t = of_assembly(model.model(), &a, None);
        // 0,13 + 2,5 + 1,5 + 0,04
        assert!((t.r - 4.0).abs() < 1e-12);
        assert!((t.r_total - 4.17).abs() < 1e-12);
        assert!((t.u.unwrap() - 1.0 / 4.17).abs() < 1e-12);
    }

    #[test]
    fn an_unknown_lambda_leaves_u_open() {
        let mut model = new_model();
        let x = mat(&mut model, "X", None);
        let a = Assembly::new(
            AssemblyUse::Wall,
            vec![Layer::new(LayerFunction::Structure, Some(x), 0.3)],
        );
        let t = of_assembly(model.model(), &a, None);
        assert_eq!(t.u, None);
        assert_eq!(t.missing().len(), 1);
    }

    #[test]
    fn a_ventilated_gap_cuts_off_the_cladding() {
        let mut model = new_model();
        let board = mat(&mut model, "Board", Some(0.2));
        let wool = mat(&mut model, "Wool", Some(0.04));
        // a roof from the top: tiles (no λ), ventilated gap, wool, board
        let tiles = mat(&mut model, "Tiles", None);
        let a = Assembly::new(
            AssemblyUse::Roof,
            vec![
                Layer::new(LayerFunction::Finish, Some(tiles), 0.02),
                Layer::new(LayerFunction::AirGap, None, 0.04),
                Layer::new(LayerFunction::Insulation, Some(wool), 0.2),
                Layer::new(LayerFunction::Finish, Some(board), 0.02),
            ],
        );
        let t = of_assembly(model.model(), &a, None);
        assert!(t.layers[0].skipped && t.layers[1].skipped);
        // Rsi 0,10 up, R 5,0 + 0,1, Rse = Rsi
        assert!((t.r_total - 5.3).abs() < 1e-12);
        assert!(t.u.is_some());
    }

    #[test]
    fn a_floor_loses_heat_through_its_underside() {
        let mut model = new_model();
        let c = mat(&mut model, "Concrete", Some(1.0));
        let a = Assembly::new(
            AssemblyUse::Floor,
            vec![Layer::new(LayerFunction::Structure, Some(c), 0.2)],
        );
        let t = of_assembly(
            model.model(),
            &a,
            Some(Boundary {
                flow: HeatFlow::Down,
                outside: Outside::Ground,
            }),
        );
        assert!((t.r_total - (0.17 + 0.2)).abs() < 1e-12);
    }
}

#[cfg(test)]
mod sample_tests {
    use crate::{ElementKind, Model};

    #[test]
    fn the_sample_houses_walls_have_u_in_the_data_standard() {
        // every material with a λ, as the Slovak base library gives them
        let mut doc = crate::Document::from_model(crate::sample::sample_house());
        let mut tx = crate::Transaction::new("λ");
        let mats: Vec<crate::Element> = doc
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
        let model: Model = doc.model().clone();
        let cx = crate::dss::Ctx::new(&model);
        let mut seen = 0;
        for e in model.iter() {
            if !matches!(e.kind, ElementKind::Wall(_) | ElementKind::Slab(_)) {
                continue;
            }
            let Some(u) = super::of_element(&model, e).and_then(|t| t.u) else {
                continue;
            };
            assert!(u > 0.05 && u < 10.0, "{u}");
            if let Some((_, f)) = crate::dss::filled(&model, &cx, e) {
                if let Some(v) = f
                    .iter()
                    .find(|f| f.prop.name == "súčiniteľ prechodu tepla – navrhnutý")
                {
                    assert!(v.value.is_some());
                    seen += 1;
                }
            }
        }
        assert!(seen > 0, "no load-bearing wall or slab got its U");
    }
}

#[cfg(test)]
mod base_tests {
    use crate::ElementKind;

    /// Every build-up of the Slovak base gets its U, inner ones with
    /// Rse = Rsi and the ventilated roofs without their roofing.
    #[test]
    fn the_slovak_base_build_ups_all_have_u() {
        let mut doc = crate::Document::new();
        let tx = crate::skbase::add_base(doc.model());
        doc.commit(tx).unwrap();
        let mut n = 0;
        for e in doc.model().iter() {
            let ElementKind::Assembly(a) = &e.kind else {
                continue;
            };
            let t = super::of_assembly(doc.model(), a, None);
            let missing: Vec<_> = t.missing().iter().map(|l| l.name.clone()).collect();
            eprintln!(
                "{:<70} {:?} U={:?} skipped={}",
                e.name,
                t.boundary,
                t.u.map(super::shown),
                t.layers.iter().filter(|l| l.skipped).count()
            );
            assert!(t.u.is_some(), "{}: {missing:?}", e.name);
            if e.name.starts_with("Šikmá strecha") {
                assert!(t.layers[0].skipped && t.layers[1].skipped, "{}", e.name);
            }
            n += 1;
        }
        assert!(n > 20);
    }
}
