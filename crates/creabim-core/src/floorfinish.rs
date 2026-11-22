//! A floor's finish with what it lies on (George 2026-11-22): tiles
//! 8 mm on 2 mm of tile adhesive, laminate flooring 8 mm on a 2 mm
//! underlay. Build-ups made before have the bare finish; [`mend`] takes
//! the 2 mm off it and lays them under it, so the floor stays as thick.

use crate::{
    Assembly, AssemblyUse, ElementId, ElementKind, Layer, LayerFunction, Model, Transaction,
};

/// (finish, what lies under it, its thickness in m)
pub const UNDER: &[(&str, &str, f64)] = &[
    ("Keramická dlažba", "Lepidlo na dlažbu", 0.002),
    (
        "Laminátová / drevená podlaha",
        "Podložka pod parkety",
        0.002,
    ),
];

fn name_of(model: &Model, id: Option<ElementId>) -> Option<&str> {
    id.and_then(|i| model.get(i)).map(|e| e.name.as_str())
}

/// Lay the adhesive under tiles and the underlay under laminate in every
/// floor build-up that lacks them. Returns the change and the names of
/// the build-ups it mends; `None` when all have them.
pub fn mend(model: &Model) -> Option<(Transaction, Vec<String>)> {
    let mut tx = Transaction::new("Tile adhesive and flooring underlay");
    let mut made: Vec<(String, ElementId)> = vec![];
    let mut mended = vec![];
    for e in model.iter() {
        let ElementKind::Assembly(a) = &e.kind else {
            continue;
        };
        if a.usage != AssemblyUse::Floor {
            continue;
        }
        let mut layers: Vec<Layer> = vec![];
        let mut changed = false;
        for (i, l) in a.layers.iter().enumerate() {
            let under =
                name_of(model, l.material).and_then(|n| UNDER.iter().find(|(f, ..)| *f == n));
            let Some((_, u, t)) = under else {
                layers.push(l.clone());
                continue;
            };
            let next = a.layers.get(i + 1).and_then(|n| name_of(model, n.material));
            if next == Some(*u) {
                layers.push(l.clone());
                continue;
            }
            // the material under it: the project's, else the base's
            let id = match crate::library::materials(model)
                .into_iter()
                .find(|(_, n, _)| n == u)
            {
                Some((id, ..)) => id,
                None => match made.iter().find(|(n, _)| n == u) {
                    Some((_, id)) => *id,
                    None => {
                        let Some(m) = crate::skbase::base_material(u) else {
                            layers.push(l.clone());
                            continue;
                        };
                        let id = m.id;
                        made.push((u.to_string(), id));
                        tx = tx.insert(m);
                        id
                    }
                },
            };
            let mut top = l.clone();
            if top.thickness >= 3.0 * t {
                top.thickness -= t;
            }
            layers.push(top);
            layers.push(Layer::new(LayerFunction::Substrate, Some(id), *t));
            changed = true;
        }
        if changed {
            let mut new = e.clone();
            new.kind = ElementKind::Assembly(Assembly {
                layers,
                ..a.clone()
            });
            mended.push(e.name.clone());
            tx = tx.replace(e.clone(), new);
        }
    }
    (!mended.is_empty()).then_some((tx, mended))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Document, Element};

    #[test]
    fn tiles_and_laminate_get_what_lies_under_them_and_keep_the_floor_as_thick() {
        let mut doc = Document::from_model(crate::sample::sample_house());
        doc.commit(crate::skbase::add_base(doc.model())).unwrap();
        // the base's own floors have them already
        assert!(mend(doc.model()).is_none(), "the base lacks them");
        // a floor made before: bare tiles 10 mm on screed
        let tile = crate::library::materials(doc.model())
            .into_iter()
            .find(|(_, n, _)| *n == "Keramická dlažba")
            .unwrap()
            .0;
        let screed = crate::library::materials(doc.model())
            .into_iter()
            .find(|(_, n, _)| *n == "Cementový poter")
            .unwrap()
            .0;
        let a = Assembly::new(
            AssemblyUse::Floor,
            vec![
                Layer::new(LayerFunction::Finish, Some(tile), 0.01),
                Layer::new(LayerFunction::Substrate, Some(screed), 0.06),
            ],
        );
        let e = Element::new("Old floor", ElementKind::Assembly(a));
        let id = e.id;
        doc.commit(Transaction::new("t").insert(e)).unwrap();
        let (tx, names) = mend(doc.model()).unwrap();
        assert_eq!(names, vec!["Old floor".to_string()]);
        doc.commit(tx).unwrap();
        let ElementKind::Assembly(a) = &doc.model().get(id).unwrap().kind else {
            panic!()
        };
        let t: Vec<f64> = a.layers.iter().map(|l| l.thickness).collect();
        assert!(
            (t[0] - 0.008).abs() < 1e-9 && (t[1] - 0.002).abs() < 1e-9,
            "{t:?}"
        );
        assert!((t.iter().sum::<f64>() - 0.07).abs() < 1e-9);
        assert_eq!(
            name_of(doc.model(), a.layers[1].material),
            Some("Lepidlo na dlažbu")
        );
        assert!(mend(doc.model()).is_none());
    }
}
