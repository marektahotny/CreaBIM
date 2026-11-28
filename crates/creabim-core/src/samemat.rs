//! One material, one name: materials that are the same thing under
//! another name (an older wrap preset, an English library entry next to
//! the Slovak base) are merged into one, so take-off, legends and the
//! U-values count them together (George 2026-11-22: "nechcem duplicitné
//! materiály pod iným obdobným názvom").
//!
//! [`merge`] remaps everything made of a duplicate to the material that
//! stays and removes the duplicate; files are mended on open.

use crate::{Element, ElementId, ElementKind, Model, Transaction};

/// A duplicate name and the material it is merged into. The material
/// that stays is one of the Slovak base (sk-base.json).
pub const SAME: &[(&str, &str)] = &[
    // the wrap presets before they took the base's names
    ("EPS 70 F fasádny", "EPS 70F – fasádny"),
    ("XPS (styrodur)", "XPS – extrudovaný (sokel, pod základ)"),
    ("Minerálna vlna fasádna", "Minerálna vlna – fasádna doska"),
    (
        "Tenkovrstvová omietka",
        "Omietka silikónová (tenkovrstvová)",
    ),
    ("Vápennocementová omietka", "Omietka vápennocementová"),
    // the English library next to the Slovak base
    ("Plaster, gypsum", "Omietka sadrová"),
    ("Plasterboard", "Sadrokartón"),
    ("Screed, cement", "Cementový poter"),
    ("Membrane, bitumen", "Hydroizolácia – asfaltový pás"),
    ("Roof tile, clay", "Pálená / betónová škridla"),
    ("Timber, softwood", "Drevo mäkké (smrek)"),
    ("Brick, common red", "Plná pálená tehla"),
    ("Aluminium, anodised", "Hliník (zliatina)"),
    ("Concrete, cast in place", "Železobetón"),
    ("Ceramic tile", "Keramická dlažba"),
    ("Insulation, EPS", "EPS 100 – podlahový"),
    ("Insulation, mineral wool", "Minerálna vlna – fasádna doska"),
    ("Insulation, XPS", "XPS – extrudovaný (sokel, pod základ)"),
    ("OSB", "OSB doska"),
];

/// The name a material should have: the base's for a known duplicate,
/// else `name` itself.
pub fn canonical(name: &str) -> &str {
    SAME.iter()
        .find(|(dup, _)| *dup == name)
        .map(|(_, to)| *to)
        .unwrap_or(name)
}

/// A name reduced for comparing: lower case, no accents, letters and
/// digits only.
pub fn key(name: &str) -> String {
    name.chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| match c {
            'á' | 'ä' => 'a',
            'č' => 'c',
            'ď' => 'd',
            'é' | 'ě' => 'e',
            'í' => 'i',
            'ĺ' | 'ľ' => 'l',
            'ň' => 'n',
            'ó' | 'ô' => 'o',
            'ŕ' | 'ř' => 'r',
            'š' => 's',
            'ť' => 't',
            'ú' | 'ů' => 'u',
            'ý' => 'y',
            'ž' => 'z',
            c => c,
        })
        .filter(|c| c.is_alphanumeric())
        .collect()
}

/// The words of a name, reduced and sorted, so "Tenkovrstvová omietka"
/// and "Omietka tenkovrstvová" read the same.
fn words(name: &str) -> Vec<String> {
    let mut w: Vec<String> = name
        .split(|c: char| !c.is_alphanumeric())
        .map(key)
        .filter(|w| !w.is_empty())
        .collect();
    w.sort();
    w
}

/// Two names that read as the same material: equal once reduced, or the
/// same words in another order.
pub fn alike(a: &str, b: &str) -> bool {
    a != b && (key(a) == key(b) || words(a) == words(b))
}

/// One merge done: the duplicate's name, the material kept, and how many
/// elements were made of the duplicate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Merged {
    pub from: String,
    pub into: String,
    pub users: usize,
}

/// Merge duplicate materials: those [`SAME`] names and those of the very
/// same name (the first kept). Everything made of a duplicate is made of
/// the kept one. `None` when there is nothing to merge.
pub fn merge(model: &Model) -> Option<(Transaction, Vec<Merged>)> {
    let materials: Vec<(ElementId, &str)> = model
        .iter()
        .filter(|e| matches!(e.kind, ElementKind::Material(_)))
        .map(|e| (e.id, e.name.as_str()))
        .collect();
    // (duplicate, kept)
    let mut pairs: Vec<(ElementId, ElementId)> = vec![];
    for (i, (id, name)) in materials.iter().enumerate() {
        let want = canonical(name);
        let kept = materials
            .iter()
            .enumerate()
            .find(|(j, (_, n))| *n == want && (want != *name || *j < i))
            .map(|(_, (k, _))| *k);
        if let Some(k) = kept {
            pairs.push((*id, k));
        }
    }
    if pairs.is_empty() {
        return None;
    }
    let ids: Vec<(String, String)> = pairs
        .iter()
        .map(|(d, k)| (d.0.hyphenated().to_string(), k.0.hyphenated().to_string()))
        .collect();
    let gone: Vec<ElementId> = pairs.iter().map(|(d, _)| *d).collect();
    let mut users = vec![0usize; pairs.len()];
    let mut tx = Transaction::new("Merge duplicate materials");
    for e in model.iter().filter(|e| !gone.contains(&e.id)) {
        let Ok(text) = serde_json::to_string(e) else {
            continue;
        };
        let mut after = text.clone();
        for (n, (d, k)) in ids.iter().enumerate() {
            if after.contains(d.as_str()) {
                after = after.replace(d.as_str(), k);
                users[n] += 1;
            }
        }
        if after == text {
            continue;
        }
        if let Ok(new) = serde_json::from_str::<Element>(&after) {
            tx = tx.replace(e.clone(), new);
        }
    }
    let mut report = vec![];
    for (n, (d, k)) in pairs.iter().enumerate() {
        let (Some(dup), Some(kept)) = (model.get(*d), model.get(*k)) else {
            continue;
        };
        report.push(Merged {
            from: dup.name.clone(),
            into: kept.name.clone(),
            users: users[n],
        });
        tx.ops.push(crate::Op::Remove(dup.clone()));
    }
    Some((tx, report))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Document, Layer, LayerFunction, Material};

    #[test]
    fn names_that_read_alike_are_caught() {
        assert!(alike("EPS 70 F fasádny", "EPS 70F – fasádny"));
        assert!(alike("Omietka tenkovrstvová", "Tenkovrstvová omietka"));
        assert!(!alike("EPS 100 – podlahový", "EPS 150 – podlahový"));
        assert_eq!(canonical("Plasterboard"), "Sadrokartón");
        assert_eq!(canonical("Sadrokartón"), "Sadrokartón");
    }

    /// Every merge target is a material of the base, and no two materials
    /// a new project gets (base, starters, wrap presets) read alike.
    #[test]
    fn a_new_project_has_no_duplicate_materials() {
        let base: Vec<String> = crate::skbase::base_material_names();
        for (dup, to) in SAME {
            assert!(
                base.iter().any(|b| b == to),
                "{dup} → {to} is not in the base"
            );
        }
        let mut doc = Document::from_model(crate::sample::sample_house());
        doc.commit(crate::skbase::add_base(doc.model())).unwrap();
        let (tx, _) = crate::assembly::add_starters(doc.model());
        doc.commit(tx).unwrap();
        for (name, ..) in crate::wrap::PRESETS {
            if let Some((_, Some(e))) = crate::wrap::preset(doc.model(), name) {
                doc.commit(Transaction::new("wrap").insert(e)).unwrap();
            }
        }
        let names: Vec<String> = crate::library::materials(doc.model())
            .into_iter()
            .map(|(_, n, _)| n.to_string())
            .collect();
        for (i, a) in names.iter().enumerate() {
            assert!(!names[..i].contains(a), "material {a} is there twice");
            assert!(canonical(a) == a, "{a} should have been {}", canonical(a));
            for b in &names[..i] {
                assert!(!alike(a, b), "{a} and {b} read as one material");
            }
        }
        assert!(merge(doc.model()).is_none());
    }

    #[test]
    fn a_duplicate_hands_its_users_over_and_goes() {
        let mut doc = Document::from_model(crate::sample::sample_house());
        doc.commit(crate::skbase::add_base(doc.model())).unwrap();
        let kept = crate::library::materials(doc.model())
            .into_iter()
            .find(|(_, n, _)| *n == "Omietka silikónová (tenkovrstvová)")
            .unwrap()
            .0;
        let dup = Element::new(
            "Tenkovrstvová omietka",
            ElementKind::Material(Material::default()),
        );
        let twin = Element::new(
            "Omietka silikónová (tenkovrstvová)",
            ElementKind::Material(Material::default()),
        );
        let a = crate::Assembly::new(
            crate::AssemblyUse::Wall,
            vec![
                Layer::new(LayerFunction::Finish, Some(dup.id), 0.005),
                Layer::new(LayerFunction::Finish, Some(twin.id), 0.005),
            ],
        );
        let asm = Element::new("Test", ElementKind::Assembly(a));
        let asm_id = asm.id;
        let (dup_id, twin_id) = (dup.id, twin.id);
        doc.commit(Transaction::new("t").insert(dup).insert(twin).insert(asm))
            .unwrap();
        let (tx, report) = merge(doc.model()).unwrap();
        doc.commit(tx).unwrap();
        assert_eq!(report.len(), 2);
        assert!(report.iter().all(|m| m.users == 1));
        assert!(doc.model().get(dup_id).is_none());
        assert!(doc.model().get(twin_id).is_none());
        let ElementKind::Assembly(a) = &doc.model().get(asm_id).unwrap().kind else {
            panic!()
        };
        assert!(a.layers.iter().all(|l| l.material == Some(kept)));
    }
}
