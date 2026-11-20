//! What a new project starts with in Slovak practice: the usual building
//! materials with their design values (λ by STN 73 0540-3 and common data
//! sheets, reaction to fire by STN EN 13501-1), and the usual build-ups of
//! walls, floors and roofs made of them, each with a type to draw with.
//! Ported from PilonDesigner (`materials.ts`, `types.ts`; the data in
//! `data/sk-base.json` is written by `tools/sk-base/export.sh`). The
//! values are for a first design and can be edited in the project.

use crate::assembly::{self, Assembly, AssemblyUse, Layer, LayerFunction, MIN_LAYER};
use crate::types::{self, ElementType, SlabType, TypeParams, WallType};
use crate::{Element, ElementId, ElementKind, Hatch, Material, Model, Transaction};
use serde::Deserialize;

#[derive(Deserialize)]
struct Base {
    #[serde(default)]
    materials: Vec<BaseMaterial>,
    #[serde(default)]
    walls: Vec<BuildUp>,
    #[serde(default)]
    floors: Vec<BuildUp>,
    #[serde(default)]
    roofs: Vec<BuildUp>,
}

#[derive(Deserialize)]
struct BaseMaterial {
    id: String,
    name: String,
    category: String,
    lambda: f64,
    density: f64,
    fire: String,
    color: String,
    #[serde(default)]
    maker: String,
    #[serde(default)]
    block: String,
    #[serde(default)]
    rw: Option<f64>,
    #[serde(default, rename = "fireRes")]
    fire_res: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    note: String,
}

#[derive(Deserialize)]
struct BuildUp {
    name: String,
    #[serde(default)]
    boundary: String,
    #[serde(default)]
    fire: String,
    #[serde(default)]
    rw: Option<f64>,
    /// A curtain wall: CreaBIM has its own, so it is left out.
    #[serde(default)]
    curtain: Option<serde_json::Value>,
    layers: Vec<BaseLayer>,
}

#[derive(Deserialize)]
struct BaseLayer {
    material: String,
    mm: f64,
    #[serde(default)]
    core: bool,
    /// Above a ventilated gap (roofing, battens).
    #[serde(default)]
    vent: bool,
}

/// PilonDesigner's base, with CreaBIM's own additions over it
/// (`data/sk-partitions.json`: partitions of the PORFIX and Porotherm
/// catalogues, 10 mm plaster both sides). An addition's material takes
/// the place of the base's of the same id, so a value checked against the
/// maker's sheet wins; its build-ups come after the base's.
fn base() -> Base {
    let mut b: Base =
        serde_json::from_str(include_str!("../data/sk-base.json")).expect("sk-base.json");
    // CreaBIM's own, past what PilonDesigner exports
    let extra: Base = serde_json::from_str(EXTRA).expect("skbase EXTRA");
    b.materials.extend(extra.materials);
    b.walls.extend(extra.walls);
    b.floors.extend(extra.floors);
    b.roofs.extend(extra.roofs);
    let more: Base = serde_json::from_str(include_str!("../data/sk-partitions.json"))
        .expect("sk-partitions.json");
    for m in more.materials {
        match b.materials.iter_mut().find(|x| x.id == m.id) {
            Some(x) => *x = m,
            None => b.materials.push(m),
        }
    }
    b.walls.extend(more.walls);
    b.floors.extend(more.floors);
    b.roofs.extend(more.roofs);
    b
}

/// What CreaBIM adds to the exported base (sk-base.json is rewritten by
/// `tools/sk-base/export.sh`, so these live here). The masonry on the
/// footings (George 2026-11-20): concrete formwork blocks DT 300 filled
/// with concrete as its load-bearing part, a 5 mm membrane and 140 mm of
/// XPS on its outer face; a foundation plan draws the blocks' outline
/// above as a thin dash and two dots. λ and ρ of the filled blocks are
/// plain concrete's (its fill) until the maker's sheet says otherwise.
const EXTRA: &str = r##"{
 "materials": [
  {"id": "dt-300", "category": "Betón", "name": "Debniaca tvárnica DT 300, vyplnená betónom",
   "lambda": 1.3, "density": 2300, "fire": "A1", "color": "#aeb3b8",
   "block": "DT 300", "note": "debniace tvárnice (DT) vyplnené betónom; λ a ρ ako prostý betón (výplň), upraviť podľa listu výrobcu"}
 ],
 "walls": [
  {"id": "NADZ-DT300-XPS140", "name": "Nadzákladová stena – DT 300 + hydroizolácia 5 + XPS 140",
   "boundary": "teren",
   "layers": [
    {"material": "xps", "mm": 140},
    {"material": "hydroizolacia", "mm": 5},
    {"material": "dt-300", "mm": 300, "core": true}
   ]}
 ],
 "floors": [],
 "roofs": []
}"##;

/// The IFC category and the cut hatch of a material of PilonDesigner's
/// group `category`, the hatch as the office's ASR legend of materials
/// draws it (George's sample section ASR_006, 2026-11-20): outer
/// load-bearing blocks, inner load-bearing blocks and partitions each
/// their own, reinforced and plain concrete, undisturbed ground, fills of
/// earth and of gravel, and every thermal insulation in batt loops.
fn kind_of(m: &BaseMaterial) -> (&'static str, Hatch) {
    let (category, _) = kind_of_before(m);
    let c = m.category.as_str();
    let n = m.name.to_lowercase();
    // the thickness a block's name gives, mm
    let mm = n
        .rsplit(|c: char| !c.is_ascii_digit())
        .filter_map(|w| w.parse::<u32>().ok())
        .find(|v| (50..=500).contains(v));
    let masonry = |m: &BaseMaterial| {
        if n.contains("priečkov")
            || n.contains("nenosn")
            // Porotherm's partition bricks 8 to 14 cm
            || (m.id.starts_with("porotherm-") && !m.id.starts_with("porotherm-17"))
        {
            Hatch::Diagonal
        } else if n.contains("nosná vnútorná")
            || m.category.contains("AKU")
            || m.id == "vapennopiesk"
            || m.id.starts_with("porotherm-")
        {
            Hatch::MasonryInner
        } else if m.id.starts_with("porobeton-") {
            // the light ones build outer walls, the dense ones inner
            if m.id.ends_with("350") || m.id.ends_with("400") {
                Hatch::MasonryOuter
            } else {
                Hatch::MasonryInner
            }
        } else {
            match mm {
                Some(t) if t >= 300 => Hatch::MasonryOuter,
                _ => Hatch::MasonryInner,
            }
        }
    };
    let hatch = if m.id == "tehla-plna" {
        Hatch::Brick
    } else if c.starts_with("Tehly") || c.contains("AKU") || c.contains("órobetón") {
        masonry(m)
    } else if m.id == "zelezobeton" {
        Hatch::Reinforced
    } else if c == "Minerálna vlna" || c == "Polystyrén" {
        Hatch::Insulation
    } else if m.id == "zemina" {
        Hatch::Ground
    } else if m.id == "nasyp-zemina" {
        Hatch::EarthFill
    } else if m.id == "strk" {
        Hatch::Gravel
    } else {
        kind_of_before(m).1
    };
    (category, hatch)
}

/// The category and hatch the base gave its materials before the hatches
/// of the ASR legend: a project's material still in its old hatch takes
/// the new one ([`refresh_hatches`]).
fn kind_of_before(m: &BaseMaterial) -> (&'static str, Hatch) {
    let c = m.category.as_str();
    if m.id == "tehla-plna" {
        ("brick", Hatch::Brick)
    } else if c.starts_with("Tehly") || c.contains("AKU") {
        ("block", Hatch::Brick)
    } else if c.contains("órobetón") {
        ("block", Hatch::Diagonal)
    } else if c == "Betón" {
        ("concrete", Hatch::Concrete)
    } else if c == "Minerálna vlna" || c == "Polystyrén" {
        ("insulation", Hatch::Cross)
    } else if c.starts_with("Omietky") {
        ("plaster", Hatch::Solid)
    } else if c.starts_with("Sadrokartón") {
        if m.id == "sdk" {
            ("gypsum", Hatch::Solid)
        } else {
            ("wood", Hatch::Timber)
        }
    } else if c == "Podlahy" {
        if m.id == "laminat" {
            ("wood", Hatch::Timber)
        } else {
            ("finish", Hatch::Solid)
        }
    } else if c == "Obklady" {
        if m.id == "obklad-kamen" {
            ("stone", Hatch::Solid)
        } else {
            ("finish", Hatch::Solid)
        }
    } else if c.starts_with("Izolácie") {
        ("membrane", Hatch::Solid)
    } else if c.starts_with("Strešné") {
        ("roofing", Hatch::Solid)
    } else if c == "Kameň" {
        ("stone", Hatch::Brick)
    } else if c.starts_with("Zeminy") {
        ("earth", Hatch::Diagonal)
    } else if c == "Kovy" {
        if m.id == "hlinik" {
            ("aluminium", Hatch::Solid)
        } else {
            ("steel", Hatch::Solid)
        }
    } else if c.starts_with("Sklo") {
        ("glass", Hatch::Empty)
    } else {
        ("", Hatch::Solid)
    }
}

/// The base's materials of a project still in the hatch the base gave
/// them before the ASR legend's hatches, in the new hatch; none when
/// there is nothing to change. A hatch picked by hand stays.
pub fn refresh_hatches(model: &Model) -> Option<Transaction> {
    let b = base();
    let mut tx = Transaction::new("Hatches of the legend of materials");
    let mut any = false;
    for e in model.iter() {
        let ElementKind::Material(mat) = &e.kind else {
            continue;
        };
        let Some(m) = b.materials.iter().find(|m| m.name == e.name) else {
            continue;
        };
        let (old, new) = (kind_of_before(m).1, kind_of(m).1);
        if mat.hatch == old && old != new && mat.pattern.is_none() {
            let mut after = e.clone();
            after.kind = ElementKind::Material(Material {
                hatch: new,
                ..mat.clone()
            });
            tx = tx.replace(e.clone(), after);
            any = true;
        }
    }
    any.then_some(tx)
}

fn colour(hex: &str) -> [f32; 3] {
    let v = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0xcccccc);
    [(v >> 16) & 255, (v >> 8) & 255, v & 255].map(|c| c as f32 / 255.0)
}

fn material(m: &BaseMaterial) -> Material {
    let (category, hatch) = kind_of(m);
    // a product's data, as its sheet gives them
    let mut d: Vec<String> = vec![];
    if !m.maker.is_empty() {
        d.push(m.maker.clone());
    }
    if !m.kind.is_empty() {
        d.push(m.kind.clone());
    }
    if !m.block.is_empty() {
        d.push(format!("tvárnica {}", m.block));
    }
    if let Some(rw) = m.rw {
        d.push(format!("Rw {rw} dB"));
    }
    if !m.fire_res.is_empty() {
        d.push(m.fire_res.clone());
    }
    if !m.note.is_empty() {
        d.push(m.note.clone());
    }
    Material {
        hatch,
        color: colour(&m.color),
        category: category.into(),
        description: d.join("; "),
        metallic: if category == "steel" || category == "aluminium" {
            1.0
        } else {
            0.0
        },
        roughness: if category == "glass" { 0.1 } else { 0.8 },
        opacity: if category == "glass" { 0.35 } else { 1.0 },
        lambda: Some(m.lambda),
        density: Some(m.density),
        fire: m.fire.clone(),
        ..Material::default()
    }
}

/// What a layer does in its build-up: the core carries; over a
/// ventilated gap the outermost is the finish and the rest (battens) the
/// gap; else by its material.
fn function(l: &BaseLayer, first: bool, category: &str, screed: bool) -> LayerFunction {
    if l.core {
        LayerFunction::Structure
    } else if l.vent {
        if first {
            LayerFunction::Finish
        } else {
            LayerFunction::AirGap
        }
    } else if screed {
        LayerFunction::Substrate
    } else {
        match category {
            "insulation" => LayerFunction::Insulation,
            "membrane" => LayerFunction::Membrane,
            _ => LayerFunction::Finish,
        }
    }
}

/// What the base holds: (materials, wall, floor, roof build-ups).
pub fn counts() -> (usize, usize, usize, usize) {
    let b = base();
    let n = |l: &[BuildUp]| l.iter().filter(|t| t.curtain.is_none()).count();
    (b.materials.len(), n(&b.walls), n(&b.floors), n(&b.roofs))
}

/// Add the Slovak base the model doesn't have yet (by name): materials,
/// build-ups of walls, floors and roofs, and a type of each build-up.
pub fn add_base(model: &Model) -> Transaction {
    let b = base();
    let mut tx = Transaction::new("Slovak materials and build-ups");
    let have: Vec<(String, ElementId)> = crate::library::materials(model)
        .into_iter()
        .map(|(id, n, _)| (n.to_string(), id))
        .collect();
    // each base material's element, new or already there, with what its
    // layers are
    let mut by_id: std::collections::HashMap<&str, (ElementId, &'static str, bool)> =
        Default::default();
    for m in &b.materials {
        let (category, _) = kind_of(m);
        let screed = m.id.starts_with("poter");
        let id = match have.iter().find(|(n, _)| *n == m.name) {
            Some((_, id)) => *id,
            None => {
                let e = Element::new(m.name.as_str(), ElementKind::Material(material(m)));
                let id = e.id;
                tx = tx.insert(e);
                id
            }
        };
        by_id.insert(m.id.as_str(), (id, category, screed));
    }
    let assemblies = assembly::assemblies(model);
    let kinds = types::types(model);
    for (usage, list) in [
        (AssemblyUse::Wall, &b.walls),
        (AssemblyUse::Floor, &b.floors),
        (AssemblyUse::Roof, &b.roofs),
    ] {
        for t in list.iter().filter(|t| t.curtain.is_none()) {
            let found = assemblies.iter().find(|(_, n, _)| *n == t.name);
            let (id, total) = match found {
                Some((id, _, a)) => (*id, a.total()),
                None => {
                    let layers: Vec<Layer> = t
                        .layers
                        .iter()
                        .enumerate()
                        .map(|(i, l)| {
                            let (m, category, screed) = match by_id.get(l.material.as_str()) {
                                Some((id, c, s)) => (Some(*id), *c, *s),
                                None => (None, "", false),
                            };
                            // foils thinner than a layer can be are drawn at 1 mm
                            Layer::new(
                                function(l, i == 0, category, screed),
                                m,
                                (l.mm / 1000.0).max(MIN_LAYER),
                            )
                        })
                        .collect();
                    let mut a = Assembly::new(usage, layers);
                    let mut d: Vec<String> = vec![];
                    match t.boundary.as_str() {
                        "exterier" => d.push("vonkajšia konštrukcia".into()),
                        "interier" => d.push("vnútorná konštrukcia".into()),
                        "teren" => d.push("konštrukcia pri teréne".into()),
                        _ => {}
                    }
                    if !t.fire.is_empty() {
                        d.push(format!("požiarna odolnosť {}", t.fire));
                    }
                    if let Some(rw) = t.rw {
                        d.push(format!("Rw {rw} dB"));
                    }
                    a.description = d.join("; ");
                    let mut b = crate::thermal::Boundary::of(&a);
                    // a ceiling under a cold attic gives its heat upwards
                    if usage == AssemblyUse::Floor && t.name.contains("podkrov") {
                        b.flow = crate::thermal::HeatFlow::Up;
                    }
                    a.thermal = Some(b);
                    let total = a.total();
                    let e = Element::new(t.name.as_str(), ElementKind::Assembly(a));
                    let id = e.id;
                    tx = tx.insert(e);
                    (id, total)
                }
            };
            if kinds.iter().any(|(_, n, _)| *n == t.name) {
                continue;
            }
            let params = match usage {
                AssemblyUse::Wall => TypeParams::Wall(WallType {
                    thickness: total,
                    assembly: Some(id),
                    material: None,
                    location: Default::default(),
                    height: None,
                    curtain: None,
                    screen: None,
                }),
                AssemblyUse::Floor => TypeParams::Slab(SlabType {
                    thickness: total,
                    assembly: Some(id),
                    material: None,
                }),
                AssemblyUse::Roof => TypeParams::Roof(SlabType {
                    thickness: total,
                    assembly: Some(id),
                    material: None,
                }),
            };
            tx = tx.insert(Element::new(
                t.name.as_str(),
                ElementKind::ElementType(ElementType::new(params)),
            ));
        }
    }
    tx
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;

    #[test]
    fn a_new_project_takes_the_base_once() {
        let mut doc = Document::new();
        doc.commit(add_base(doc.model())).unwrap();
        let m = doc.model();
        let (materials, walls, floors, roofs) = counts();
        assert_eq!((materials, walls, floors, roofs), (79, 34, 4, 3));
        assert_eq!(crate::library::materials(m).len(), materials);
        assert_eq!(assembly::assemblies(m).len(), walls + floors + roofs);
        assert_eq!(types::types(m).len(), walls + floors + roofs);
        // again adds nothing
        assert!(add_base(m).ops.is_empty());
    }

    #[test]
    fn the_masonry_on_the_footings_is_dt_blocks_with_membrane_and_xps_outside() {
        let mut doc = Document::new();
        doc.commit(add_base(doc.model())).unwrap();
        let m = doc.model();
        let all = assembly::assemblies(m);
        let (_, _, a) = all
            .iter()
            .find(|(_, n, _)| n.starts_with("Nadzákladová stena – DT 300"))
            .unwrap();
        a.validate().unwrap();
        assert!((a.total() - 0.445).abs() < 1e-9);
        let f: Vec<LayerFunction> = a.layers.iter().map(|l| l.function).collect();
        assert_eq!(
            f,
            [
                LayerFunction::Insulation,
                LayerFunction::Membrane,
                LayerFunction::Structure
            ]
        );
        let core = m.get(a.layers[2].material.unwrap()).unwrap();
        assert!(core.name.starts_with("Debniaca tvárnica DT 300"));
        // and a wall type to draw it with
        assert!(types::types(m)
            .iter()
            .any(|(_, n, _)| n.starts_with("Nadzákladová stena – DT 300")));
    }

    #[test]
    fn a_build_up_is_made_of_its_layers() {
        let mut doc = Document::new();
        doc.commit(add_base(doc.model())).unwrap();
        let m = doc.model();
        let all = assembly::assemblies(m);
        let (_, _, a) = all
            .iter()
            .find(|(_, n, _)| *n == "Obvodová – tehla 300 + EPS 150")
            .unwrap();
        a.validate().unwrap();
        assert!((a.total() - 0.47).abs() < 1e-9);
        let f: Vec<LayerFunction> = a.layers.iter().map(|l| l.function).collect();
        assert_eq!(
            f,
            [
                LayerFunction::Finish,
                LayerFunction::Insulation,
                LayerFunction::Structure,
                LayerFunction::Finish
            ]
        );
        // the insulation is EPS with its λ and reaction to fire
        let eps = m.get(a.layers[1].material.unwrap()).unwrap();
        let ElementKind::Material(mat) = &eps.kind else {
            panic!()
        };
        assert_eq!(eps.name, "EPS 70F – fasádny");
        assert_eq!((mat.lambda, mat.fire.as_str()), (Some(0.039), "E"));
        // every build-up is sound, foils at the thinnest a layer can be
        for (_, n, a) in &all {
            a.validate().unwrap_or_else(|e| panic!("{n}: {e}"));
        }
        // and its type is as thick
        let kinds = types::types(m);
        let (_, _, t) = kinds
            .iter()
            .find(|(_, n, _)| *n == "Obvodová – tehla 300 + EPS 150")
            .unwrap();
        let TypeParams::Wall(w) = &t.params else {
            panic!()
        };
        assert!((w.thickness - 0.47).abs() < 1e-9 && w.assembly.is_some());
    }

    #[test]
    fn partitions_of_porfix_and_porotherm_are_plastered_10_mm() {
        let mut doc = Document::new();
        doc.commit(add_base(doc.model())).unwrap();
        let m = doc.model();
        let all = assembly::assemblies(m);
        let parts: Vec<_> = all
            .iter()
            .filter(|(_, n, _)| {
                n.starts_with("Priečka – PORFIX") || n.starts_with("Priečka – Porotherm")
            })
            .collect();
        assert!(parts.len() >= 15, "{}", parts.len());
        for (_, n, a) in &parts {
            let l = &a.layers;
            assert_eq!(l.len(), 3, "{n}");
            assert!((l[0].thickness - 0.01).abs() < 1e-9, "{n}");
            assert!((l[2].thickness - 0.01).abs() < 1e-9, "{n}");
            assert_eq!(l[1].function, LayerFunction::Structure, "{n}");
            // every block has its λ, so the U is counted
            let u = crate::thermal::of_assembly(m, a, None);
            assert!(u.u.is_some(), "{n}");
        }
        let (_, _, a) = all
            .iter()
            .find(|(_, n, _)| *n == "Priečka – Porotherm 11,5 AKU")
            .unwrap();
        assert!((a.total() - 0.135).abs() < 1e-9);
        let brick = m.get(a.layers[1].material.unwrap()).unwrap();
        let ElementKind::Material(mat) = &brick.kind else {
            panic!()
        };
        assert_eq!((mat.lambda, mat.hatch), (Some(0.34), Hatch::Diagonal));
        // the PORFIX partition block takes λ10,dry from porfix.sk
        let (_, _, a) = all
            .iter()
            .find(|(_, n, _)| *n == "Priečka – PORFIX priečkovka P2-500 125")
            .unwrap();
        let ElementKind::Material(mat) = &m.get(a.layers[1].material.unwrap()).unwrap().kind else {
            panic!()
        };
        assert_eq!((mat.lambda, mat.hatch), (Some(0.12), Hatch::Diagonal));
    }
}
