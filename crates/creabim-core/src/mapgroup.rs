//! What came down from the map site (Pilon Map Downloader: the cadastre
//! drawing, the orthophoto, the terrain) is one piece of ground. Moving or
//! turning any of it moves and turns the rest the same way, and with it
//! where the project lies in S-JTSK: the building stays where it is in
//! the model and the ground goes round it, so the coordinates written
//! stay those of the ground under each point (George, 2026-11-20).

use crate::transform::{transformed, Xform};
use crate::{Element, ElementId, ElementKind, Model, Op, Transaction};
use glam::{DVec2, DVec3};

/// A rigid move in plan: turned `degrees` about `about`, then moved `by`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rigid {
    pub about: DVec2,
    pub degrees: f64,
    pub by: DVec2,
}

impl Rigid {
    pub fn apply(&self, p: DVec2) -> DVec2 {
        self.about + DVec2::from_angle(self.degrees.to_radians()).rotate(p - self.about) + self.by
    }

    fn is_nothing(&self) -> bool {
        self.degrees.abs() < 1e-9 && self.by.length() < 1e-9
    }
}

/// How an element of the map moved from `before` to `after`, if it moved
/// as a whole (not scaled, not reshaped).
fn rigid_of(before: &Element, after: &Element) -> Option<Rigid> {
    match (&before.kind, &after.kind) {
        (ElementKind::Reference(a), ElementKind::Reference(b)) => {
            if (a.scale - b.scale).abs() > 1e-12 || a.bounds != b.bounds {
                return None;
            }
            let d = b.rotation - a.rotation;
            let degrees = (d + 180.0).rem_euclid(360.0) - 180.0;
            Some(Rigid {
                about: a.at,
                degrees,
                by: b.at - a.at,
            })
        }
        (ElementKind::Site(a), ElementKind::Site(b)) => {
            let (pa, pb) = (outline(&a.form)?, outline(&b.form)?);
            if pa.len() != pb.len() || pa.len() < 2 {
                return None;
            }
            // the first two corners give the turn; the rest must agree
            let (a0, a1, b0, b1) = (pa[0], pa[1], pb[0], pb[1]);
            if (a0.distance(a1) - b0.distance(b1)).abs() > 1e-6 {
                return None;
            }
            let degrees = ((b1 - b0).to_angle() - (a1 - a0).to_angle()).to_degrees();
            let degrees = (degrees + 180.0).rem_euclid(360.0) - 180.0;
            let r = Rigid {
                about: a0,
                degrees,
                by: b0 - a0,
            };
            pa.iter()
                .zip(&pb)
                .all(|(p, q)| r.apply(*p).distance(*q) < 1e-6)
                .then_some(r)
        }
        _ => None,
    }
}

fn outline(f: &crate::site::SiteForm) -> Option<Vec<DVec2>> {
    match f {
        crate::site::SiteForm::Terrain { boundary, .. } => Some(boundary.clone()),
        _ => None,
    }
}

/// `e` moved by `r`, as the Move and Rotate tools would.
fn moved(model: &Model, e: &Element, r: &Rigid) -> Option<Element> {
    let turned = if r.degrees.abs() > 1e-12 {
        transformed(
            model,
            e,
            &Xform::Rotate {
                center: r.about,
                degrees: r.degrees,
            },
        )?
    } else {
        e.clone()
    };
    if r.by.length() > 1e-12 {
        transformed(
            model,
            &turned,
            &Xform::Move(DVec3::new(r.by.x, r.by.y, 0.0)),
        )
    } else {
        Some(turned)
    }
}

fn is_jtsk_drawing(r: &crate::Reference) -> bool {
    matches!(r.source, crate::RefSource::Dxf { .. })
        && r.bounds
            .is_some_and(|[a, _]| a.x.abs() > 50_000.0 && a.y.abs() > 50_000.0)
}

/// The map's parts and the drawing S-JTSK is read from. A project whose
/// map came down before it was kept as one: its drawings in S-JTSK, the
/// pictures shown with them (the orthophoto) and the terrain on the site
/// level.
pub fn group(model: &Model) -> (Vec<ElementId>, Option<ElementId>) {
    let Some((_, info)) = model.project_info() else {
        return (vec![], None);
    };
    if !info.map_group.is_empty() {
        return (info.map_group.clone(), info.map_ref);
    }
    let refs: Vec<(&Element, &crate::Reference)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Reference(r) => Some((e, r)),
            _ => None,
        })
        .collect();
    let jtsk: Vec<&(&Element, &crate::Reference)> =
        refs.iter().filter(|(_, r)| is_jtsk_drawing(r)).collect();
    let Some(first) = jtsk.first() else {
        return (vec![], None);
    };
    let views = &first.1.shown_in;
    let mut ids: Vec<ElementId> = jtsk.iter().map(|(e, _)| e.id).collect();
    ids.extend(refs.iter().filter_map(|(e, r)| {
        (matches!(r.source, crate::RefSource::Image { .. })
            && r.shown_in.iter().any(|v| views.contains(v)))
        .then_some(e.id)
    }));
    if let Some(site) = model.site_level() {
        ids.extend(model.iter().filter_map(|e| match &e.kind {
            ElementKind::Site(s)
                if s.level == site && matches!(s.form, crate::site::SiteForm::Terrain { .. }) =>
            {
                Some(e.id)
            }
            _ => None,
        }));
    }
    (ids, Some(first.0.id))
}

/// After `tx` (already applied to `model`): the rest of the map moved as
/// the part of it `tx` moved, and true north turned with the ground. Empty
/// when `tx` moved no part of the map, or the whole of it was moved
/// together with the project's information (Rotate project north).
pub fn settle(model: &Model, tx: &Transaction) -> Transaction {
    let mut out = Transaction::new("Map moves as one");
    let Some((pid, info)) = model.project_info() else {
        return out;
    };
    let (members, _) = group(model);
    if members.is_empty() {
        return out;
    }
    let touched = |id: ElementId| {
        tx.ops.iter().any(|o| match o {
            Op::Replace { after, .. } | Op::Insert(after) | Op::Remove(after) => after.id == id,
        })
    };
    // what said where north is was set with it: leave both alone
    if touched(pid) {
        return out;
    }
    let Some(r) = tx.ops.iter().find_map(|o| match o {
        Op::Replace { before, after } if members.contains(&after.id) => {
            rigid_of(before, after).filter(|r| !r.is_nothing())
        }
        _ => None,
    }) else {
        return out;
    };
    for id in &members {
        if touched(*id) {
            continue;
        }
        let Some(e) = model.get(*id) else {
            continue;
        };
        if let Some(n) = moved(model, e, &r) {
            if n != *e {
                out = out.replace(e.clone(), n);
            }
        }
    }
    if r.degrees.abs() > 1e-12 {
        let mut p = info.clone();
        let n = (p.true_north + r.degrees + 180.0).rem_euclid(360.0) - 180.0;
        p.true_north = (n * 1e9).round() / 1e9;
        if let Some(e) = model.get(pid) {
            let mut now = e.clone();
            now.kind = ElementKind::Project(p);
            out = out.replace(e.clone(), now);
        }
    }
    out
}

/// A model point's S-JTSK east and north (both negative in Slovakia):
/// read off the map drawing where it lies under the point, else from the
/// project's S-JTSK origin turned by true north. None without either.
pub fn sjtsk_of(model: &Model, p: DVec2) -> Option<DVec2> {
    let (_, info) = model.project_info()?;
    if let Some(ElementKind::Reference(r)) =
        group(model).1.and_then(|id| model.get(id)).map(|e| &e.kind)
    {
        return Some(r.unplace(p));
    }
    let [e0, n0] = info.sjtsk?;
    let q = DVec2::from_angle(-info.true_north.to_radians()).rotate(p);
    Some(DVec2::new(e0, n0) + q)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Document, ProjectInfo, RefSource, Reference};

    /// A map drawing in S-JTSK and the project's information naming it.
    fn mapped() -> (Document, ElementId, ElementId) {
        let mut doc = Document::new();
        let mut r = Reference::new(
            RefSource::Dxf {
                path: "situacia_jtsk.dxf".into(),
                unit: Some(1.0),
            },
            vec![],
        );
        let shift = DVec2::new(-547382.28, -1253318.85);
        r.in_place([shift - DVec2::splat(100.0), shift + DVec2::splat(100.0)]);
        r.at -= shift;
        let mut o = r.clone();
        o.source = RefSource::Dxf {
            path: "ortofoto.dxf".into(),
            unit: Some(1.0),
        };
        o.at += DVec2::new(5.0, 0.0);
        let map = Element::new("Map", ElementKind::Reference(r));
        let ortho = Element::new("Ortho", ElementKind::Reference(o));
        let (m, oid) = (map.id, ortho.id);
        let info = ProjectInfo {
            sjtsk: Some([shift.x, shift.y]),
            map_group: vec![m, oid],
            map_ref: Some(m),
            ..Default::default()
        };
        doc.commit(
            Transaction::new("put in")
                .insert(map)
                .insert(ortho)
                .insert(Element::new("Project", ElementKind::Project(info))),
        )
        .unwrap();
        (doc, m, oid)
    }

    #[test]
    fn the_map_moves_as_one_and_the_coordinates_go_with_the_ground() {
        let (mut doc, map, ortho) = mapped();
        let p = DVec2::new(3.0, 4.0);
        let before = sjtsk_of(doc.model(), p).unwrap();
        // the old reading, from the origin, agrees while nothing moved
        assert!((before - DVec2::new(-547382.28 + 3.0, -1253318.85 + 4.0)).length() < 1e-6);
        let ground = |doc: &Document, id| match &doc.model().get(id).unwrap().kind {
            ElementKind::Reference(r) => (r.at, r.rotation),
            _ => unreachable!(),
        };
        let o0 = ground(&doc, ortho);
        // turn the cadastre 30° about a point and move it, as the tools do
        let turn = Rigid {
            about: DVec2::new(10.0, -2.0),
            degrees: 30.0,
            by: DVec2::new(7.0, 1.0),
        };
        let e = doc.model().get(map).unwrap().clone();
        let n = moved(doc.model(), &e, &turn).unwrap();
        doc.commit(Transaction::new("Rotate").replace(e, n))
            .unwrap();
        // the orthophoto came along
        let o1 = ground(&doc, ortho);
        assert!((o1.0 - turn.apply(o0.0)).length() < 1e-9);
        assert!((o1.1 - (o0.1 + 30.0)).abs() < 1e-9);
        // true north turned with the ground
        let north = doc.model().project_info().unwrap().1.true_north;
        assert!((north - 30.0).abs() < 1e-9, "{north}");
        // the ground point that lay under p now lies under p moved
        let after = sjtsk_of(doc.model(), turn.apply(p)).unwrap();
        assert!((after - before).length() < 1e-6, "{after} {before}");
        // and one undo puts it all back
        doc.undo();
        assert_eq!(ground(&doc, ortho), o0);
        assert_eq!(doc.model().project_info().unwrap().1.true_north, 0.0);
    }

    #[test]
    fn a_map_put_in_before_is_found_by_its_drawing_in_sjtsk() {
        let (mut doc, map, ortho) = mapped();
        let (pid, info) = doc.model().project_info().unwrap();
        let mut old = doc.model().get(pid).unwrap().clone();
        let mut i = info.clone();
        i.map_group.clear();
        i.map_ref = None;
        old.kind = ElementKind::Project(i);
        let e = doc.model().get(pid).unwrap().clone();
        doc.commit(Transaction::new("old").replace(e, old)).unwrap();
        let (ids, at) = group(doc.model());
        // both are DXF drawings in S-JTSK here
        assert!(ids.contains(&map) && ids.contains(&ortho), "{ids:?}");
        assert_eq!(at, Some(map));
    }

    #[test]
    fn rotating_project_north_turns_the_map_once() {
        let (mut doc, map, _) = mapped();
        let p = DVec2::new(3.0, 4.0);
        let before = sjtsk_of(doc.model(), p).unwrap();
        doc.commit(crate::project::rotate_north(doc.model(), 20.0))
            .unwrap();
        let r = match &doc.model().get(map).unwrap().kind {
            ElementKind::Reference(r) => r.rotation,
            _ => unreachable!(),
        };
        assert!((r - 20.0).abs() < 1e-9, "{r}");
        let q = DVec2::from_angle(20f64.to_radians()).rotate(p);
        assert!((sjtsk_of(doc.model(), q).unwrap() - before).length() < 1e-6);
    }
}
