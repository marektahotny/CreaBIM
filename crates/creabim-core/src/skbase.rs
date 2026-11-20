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
    /// PilonDesigner's fire resistance: not used, the makers' documents
    /// in `data/sk-fire.json` stand in for it.
    #[serde(default)]
    #[allow(dead_code)]
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

/// Fire data checked against the makers' documents (`data/sk-fire.json`,
/// George 2026-11-20): the fire resistance of a build-up and of a block,
/// the reaction to fire of an insulation, each with where it is printed.
/// A build-up not listed there gets no fire resistance, so a fire check
/// counts it as stated by nobody (fire-open) until the designer fills it
/// in; the values PilonDesigner's export carries are not used.
#[derive(Deserialize)]
struct FireData {
    assemblies: std::collections::HashMap<String, FireAssembly>,
    materials: std::collections::HashMap<String, FireMaterial>,
}

#[derive(Deserialize)]
struct FireAssembly {
    fire: String,
    source: String,
    #[serde(default, rename = "maxHeight")]
    max_height: Option<u32>,
}

#[derive(Deserialize)]
struct FireMaterial {
    /// The block's fire resistance; absent keeps what the base gives
    /// (a sheet already quoted there).
    #[serde(default, rename = "fireRes")]
    fire_res: Option<String>,
    /// Reaction to fire (STN EN 13501-1).
    #[serde(default)]
    reaction: Option<String>,
    source: String,
}

fn fire_data() -> FireData {
    serde_json::from_str(include_str!("../data/sk-fire.json")).expect("sk-fire.json")
}

/// A build-up's fire resistance by the makers' documents, with its
/// source for the description; none when no document gives it.
fn fire_of(fire: &FireData, name: &str) -> Option<(String, String)> {
    let f = fire.assemblies.get(name)?;
    let mut d = format!("požiarna odolnosť {} ({}", f.fire, f.source);
    if let Some(h) = f.max_height {
        d.push_str(&format!("; najväčšia výška steny {} mm", group(h)));
    }
    d.push(')');
    Some((f.fire.clone(), d))
}

/// 3000 as "3 000".
fn group(n: u32) -> String {
    if n >= 1000 {
        format!("{} {:03}", n / 1000, n % 1000)
    } else {
        n.to_string()
    }
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
    // a block's fire resistance and an insulation's reaction to fire only
    // as a maker's document gives them
    let fire = fire_data();
    for m in &mut b.materials {
        match fire.materials.get(&m.id) {
            Some(f) => {
                if let Some(r) = &f.fire_res {
                    m.fire_res = r.clone();
                }
                if let Some(r) = &f.reaction {
                    m.fire = r.clone();
                }
                if !m.note.contains(&f.source) {
                    let what = if f.reaction.is_some() {
                        "reakcia na oheň"
                    } else {
                        "požiarna odolnosť"
                    };
                    m.note = [m.note.as_str(), &format!("{what} podľa: {}", f.source)]
                        .iter()
                        .filter(|x| !x.is_empty())
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("; ");
                }
            }
            None => m.fire_res.clear(),
        }
    }
    b
}

/// What CreaBIM adds to the exported base (sk-base.json is rewritten by
/// `tools/sk-base/export.sh`, so these live here). The masonry on the
/// footings (George 2026-11-20): concrete formwork blocks DT 300 filled
/// with concrete as its load-bearing part, a 5 mm membrane and 140 mm of
/// XPS on its outer face; its layers as George set them by hand (XPS,
/// PVC membrane, the blocks), so its U is worked out; a foundation plan
/// draws the blocks' outline above as a thin dash and two dots. The
/// blocks are a material of their own, "DT tvarovky", with every property
/// and the hatch of reinforced concrete, so the take-off lists them apart
/// (George 2026-11-20).
const EXTRA: &str = r##"{
 "materials": [
  {"id": "dt-300", "category": "Betón", "name": "DT tvarovky",
   "lambda": 1.58, "density": 2500, "fire": "A1", "color": "#9aa3ad",
   "block": "DT 300", "note": "debniace tvarovky (DT) vyplnené betónom s výstužou; vlastnosti ako železobetón"}
 ],
 "walls": [
  {"id": "NADZ-DT300-XPS140", "name": "Nadzákladová stena – DT 300 + hydroizolácia 5 + XPS 140",
   "boundary": "teren",
   "layers": [
    {"material": "xps", "mm": 140},
    {"material": "hydroizolacia-pvc", "mm": 5},
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
    } else if m.id == "zelezobeton" || m.id == "dt-300" {
        Hatch::Reinforced
    } else if c == "Minerálna vlna" || c == "Polystyrén" {
        Hatch::Insulation
    } else if m.id == "zemina" {
        Hatch::Ground
    } else if m.id == "nasyp-zemina" {
        Hatch::EarthFill
    } else if m.id == "strk" {
        Hatch::Gravel
    } else if m.id.starts_with("hydroizolacia") {
        Hatch::Membrane
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

/// The masonry on the footings of a project made before the material
/// "DT tvarovky" (George 2026-11-20): its load-bearing layer of
/// reinforced concrete, or of the old "Debniaca tvárnica DT 300", made of
/// DT blocks, the material added when the project has none. Only the
/// build-ups of that masonry: the base's "Nadzákladová stena…" and any
/// whose name has DT in it. None when there is nothing to change.
pub fn dt_blocks(model: &Model) -> Option<Transaction> {
    let b = base();
    let m = b.materials.iter().find(|m| m.id == "dt-300")?;
    let mut swaps: Vec<(Element, Element)> = vec![];
    let dt = model
        .iter()
        .find(|e| e.name == m.name && matches!(e.kind, ElementKind::Material(_)))
        .map(|e| e.id);
    let (dt, new) = match dt {
        Some(id) => (id, None),
        None => {
            let e = Element::new(m.name.as_str(), ElementKind::Material(material(m)));
            (e.id, Some(e))
        }
    };
    let named = |id: Option<ElementId>, n: &str| {
        id.and_then(|i| model.get(i))
            .is_some_and(|e| e.name == n || e.name.starts_with("Debniaca tvárnica DT 300"))
    };
    for e in model.iter() {
        let ElementKind::Assembly(a) = &e.kind else {
            continue;
        };
        let words: Vec<&str> = e.name.split(|c: char| !c.is_alphanumeric()).collect();
        let dt_wall = e.name.starts_with("Nadzákladová stena")
            || words
                .iter()
                .any(|w| w.eq_ignore_ascii_case("dt") || w.starts_with("DT"));
        if !dt_wall || a.usage != AssemblyUse::Wall {
            continue;
        }
        let mut after = a.clone();
        let mut changed = false;
        for (i, l) in after.layers.iter_mut().enumerate() {
            if a.is_core(i) && l.material != Some(dt) && named(l.material, "Železobetón") {
                l.material = Some(dt);
                changed = true;
            }
        }
        if changed {
            let mut ae = e.clone();
            ae.kind = ElementKind::Assembly(after);
            swaps.push((e.clone(), ae));
        }
    }
    if swaps.is_empty() {
        return None;
    }
    let mut tx = Transaction::new("DT blocks in the masonry on the footings");
    if let Some(n) = new {
        tx = tx.insert(n);
    }
    for (before, after) in swaps {
        tx = tx.replace(before, after);
    }
    Some(tx)
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
    let fire = fire_data();
    let kinds = types::types(model);
    for (usage, list) in [
        (AssemblyUse::Wall, &b.walls),
        (AssemblyUse::Floor, &b.floors),
        (AssemblyUse::Roof, &b.roofs),
    ] {
        for t in list.iter().filter(|t| t.curtain.is_none()) {
            let found = assemblies.iter().find(|(_, n, _)| *n == t.name);
            let (id, total) = match found {
                Some((id, _, a)) => {
                    // one made before the makers' fire data takes it in,
                    // unless the designer has stated their own
                    if a.fire_resistance.is_empty() {
                        if let (Some((f, _)), Some(before)) =
                            (fire_of(&fire, &t.name), model.get(*id))
                        {
                            let mut after = before.clone();
                            if let ElementKind::Assembly(x) = &mut after.kind {
                                x.fire_resistance = f;
                            }
                            tx = tx.replace(before.clone(), after);
                        }
                    }
                    (*id, a.total())
                }
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
                    if let Some((f, text)) = fire_of(&fire, &t.name) {
                        a.fire_resistance = f;
                        d.push(text);
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
        assert_eq!(core.name, "DT tvarovky");
        // and a wall type to draw it with
        assert!(types::types(m)
            .iter()
            .any(|(_, n, _)| n.starts_with("Nadzákladová stena – DT 300")));
    }

    #[test]
    fn an_older_masonry_on_the_footings_takes_the_dt_blocks() {
        let mut doc = Document::new();
        doc.commit(add_base(doc.model())).unwrap();
        // as George had it: his own build-up, its core reinforced concrete
        let m = doc.model();
        let rc = m.iter().find(|e| e.name == "Železobetón").unwrap().id;
        let (id, _, a) = assembly::assemblies(m)
            .into_iter()
            .find(|(_, n, _)| n.starts_with("Nadzákladová stena"))
            .unwrap();
        let mut a = a.clone();
        a.layers[2].material = Some(rc);
        let before = m.get(id).unwrap().clone();
        let mut mine = before.clone();
        mine.name = "DT 30, Hydroizolacia, XPS".into();
        mine.kind = ElementKind::Assembly(a);
        doc.commit(Transaction::new("t").replace(before, mine))
            .unwrap();
        let tx = dt_blocks(doc.model()).unwrap();
        doc.commit(tx).unwrap();
        let m = doc.model();
        let ElementKind::Assembly(a) = &m.get(id).unwrap().kind else {
            unreachable!()
        };
        let core = m.get(a.layers[2].material.unwrap()).unwrap();
        assert_eq!(core.name, "DT tvarovky");
        let ElementKind::Material(dt) = &core.kind else {
            unreachable!()
        };
        let ElementKind::Material(r) = &m.get(rc).unwrap().kind else {
            unreachable!()
        };
        assert_eq!(
            (dt.lambda, dt.density, dt.hatch),
            (r.lambda, r.density, r.hatch)
        );
        // once is enough
        assert!(dt_blocks(m).is_none());
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

#[cfg(test)]
mod fire_tests {
    use super::*;
    use crate::Document;

    fn fire_of_named(m: &Model, name: &str) -> String {
        let all = assembly::assemblies(m);
        let (_, _, a) = all.iter().find(|(_, n, _)| *n == name).unwrap();
        a.fire_resistance.clone()
    }

    #[test]
    fn build_ups_take_fire_resistance_only_from_makers_documents() {
        let mut doc = Document::new();
        doc.commit(add_base(doc.model())).unwrap();
        let m = doc.model();
        // PORFIX handbook, PDF p. 15 and 16
        assert_eq!(
            fire_of_named(m, "Obvodová – PORFIX P2-440 300 + EPS šedý 150"),
            "REI-M 240"
        );
        assert_eq!(
            fire_of_named(m, "Priečka – PORFIX priečkovka P2-500 125"),
            "EI 180"
        );
        // the handbook gives "-" for P2-400 and P6-650: left empty
        assert_eq!(
            fire_of_named(m, "Obvodová – PORFIX Premium P2-400 500 bez zateplenia"),
            ""
        );
        assert_eq!(fire_of_named(m, "Priečka – PORFIX P6-650 200"), "");
        // a generic brick has no maker's document
        assert_eq!(fire_of_named(m, "Obvodová – tehla 300 + EPS 150"), "");
        let all = assembly::assemblies(m);
        let (_, _, a) = all
            .iter()
            .find(|(_, n, _)| *n == "Obvodová – tehla 300 + EPS 150")
            .unwrap();
        assert!(!a.description.contains("požiarna"));
    }

    #[test]
    fn every_build_up_is_either_sourced_or_listed_as_missing() {
        let fire: serde_json::Value =
            serde_json::from_str(include_str!("../data/sk-fire.json")).unwrap();
        let b = base();
        for t in b.walls.iter().chain(&b.floors).chain(&b.roofs) {
            if t.curtain.is_some() {
                continue;
            }
            assert!(
                fire["assemblies"].get(&t.name).is_some() || fire["missing"].get(&t.name).is_some(),
                "{} neither sourced nor listed",
                t.name
            );
        }
    }

    #[test]
    fn a_project_made_before_gets_the_fire_resistance_once() {
        let mut doc = Document::new();
        doc.commit(add_base(doc.model())).unwrap();
        // as it was before: no fire resistance in the field
        let name = "Nosná vnútorná – PORFIX P4-600 250";
        let all = assembly::assemblies(doc.model());
        let (id, _, _) = all.iter().find(|(_, n, _)| *n == name).unwrap();
        let before = doc.model().get(*id).unwrap().clone();
        let mut after = before.clone();
        if let ElementKind::Assembly(x) = &mut after.kind {
            x.fire_resistance.clear();
        }
        doc.commit(Transaction::new("old").replace(before, after))
            .unwrap();
        assert_eq!(fire_of_named(doc.model(), name), "");
        doc.commit(add_base(doc.model())).unwrap();
        assert_eq!(fire_of_named(doc.model(), name), "REI-M 240");
        assert!(add_base(doc.model()).ops.is_empty());
    }

    #[test]
    fn insulation_reaction_to_fire_comes_with_its_sheet() {
        let b = base();
        let eps = b.materials.iter().find(|m| m.id == "eps-70f").unwrap();
        assert_eq!(eps.fire, "E");
        assert!(eps.note.contains("Isover EPS 70F"));
        let mw = b.materials.iter().find(|m| m.id == "mw-fasada").unwrap();
        assert_eq!(mw.fire, "A1");
        // PORFIX AKU is not in the handbook: no fire resistance
        let aku = b
            .materials
            .iter()
            .find(|m| m.id == "porfix-aku-248")
            .unwrap();
        assert!(aku.fire_res.is_empty());
    }
}
