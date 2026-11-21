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

    /// The point `apply` takes to `q`.
    pub fn unapply(&self, q: DVec2) -> DVec2 {
        self.about + DVec2::from_angle(-self.degrees.to_radians()).rotate(q - self.by - self.about)
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
    let mut moving = members.clone();
    moving.extend(ground_sketches(model));
    for id in &moving {
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
    // the S-JTSK origin goes with the ground: read where the drawing it
    // is read from already lies, if `tx` moved that one
    let mut p = info_after(model, info, &r);
    let main = group(model).1;
    if main.is_some_and(touched) && p.sjtsk.is_some() {
        if let Some(q) = sjtsk_of(model, DVec2::ZERO) {
            p.sjtsk = Some([(q.x * 1e6).round() / 1e6, (q.y * 1e6).round() / 1e6]);
        }
    }
    if p != *info {
        if let Some(e) = model.get(pid) {
            let mut now = e.clone();
            now.kind = ElementKind::Project(p);
            out = out.replace(e.clone(), now);
        }
    }
    out
}

/// The project's information once the ground moved by `r` under the
/// building: true north turned with it, and the S-JTSK origin now the
/// ground point that came to ±0,000 (George: after placing the building
/// exactly, the origin still said where the download had put it). What
/// it was as the map came down is kept for Reset the coordinates.
fn info_after(model: &Model, info: &crate::ProjectInfo, r: &Rigid) -> crate::ProjectInfo {
    let mut p = info.clone();
    if p.map_origin.is_none() {
        if let Some(o) = p.sjtsk {
            p.map_origin = Some((o, p.true_north));
        }
    }
    if p.sjtsk.is_some() {
        if let Some(q) = sjtsk_of(model, r.unapply(DVec2::ZERO)) {
            p.sjtsk = Some([(q.x * 1e6).round() / 1e6, (q.y * 1e6).round() / 1e6]);
        }
    }
    let n = (p.true_north + r.degrees + 180.0).rem_euclid(360.0) - 180.0;
    p.true_north = (n * 1e9).round() / 1e9;
    p
}

/// The model point whose S-JTSK east and north are `s`: the other way
/// of [`sjtsk_of`].
fn model_point_of(model: &Model, s: DVec2) -> Option<DVec2> {
    let (_, info) = model.project_info()?;
    if let Some(ElementKind::Reference(r)) =
        group(model).1.and_then(|id| model.get(id)).map(|e| &e.kind)
    {
        return Some(r.place(s));
    }
    let [e0, n0] = info.sjtsk?;
    Some(DVec2::from_angle(info.true_north.to_radians()).rotate(s - DVec2::new(e0, n0)))
}

/// What was drawn on the ground in a site plan (utility networks, the
/// site's symbols, a sketch made there): it lies on the map, so it moves
/// with it.
pub fn ground_sketches(model: &Model) -> Vec<ElementId> {
    model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Sketch(s) if s.net.is_some() || s.mark.is_some() || s.view.is_some() => {
                Some(e.id)
            }
            _ => None,
        })
        .collect()
}

/// The ground (the map, the terrain, what was drawn on it in a site
/// plan) moved by `r`, the building staying where it is: true north turns
/// with the ground. Empty without a map.
pub fn move_ground(model: &Model, r: &Rigid) -> Transaction {
    let mut out = Transaction::new("Place the building on the site");
    let (mut members, _) = group(model);
    if members.is_empty() || r.is_nothing() {
        return out;
    }
    members.extend(ground_sketches(model));
    for id in &members {
        let Some(e) = model.get(*id) else {
            continue;
        };
        if let Some(n) = moved(model, e, r) {
            if n != *e {
                out = out.replace(e.clone(), n);
            }
        }
    }
    if let Some((pid, info)) = model.project_info() {
        let p = info_after(model, info, r);
        if p != *info {
            if let Some(e) = model.get(pid) {
                let mut now = e.clone();
                now.kind = ElementKind::Project(p);
                out = out.replace(e.clone(), now);
            }
        }
    }
    out
}

/// The map put back where it came down (George: "a button to reset the
/// coordinates"): each drawing in S-JTSK straight again with the project's
/// S-JTSK origin at ±0,000; the rest of the map (the orthophoto, the
/// terrain, what was drawn on it in a site plan) moved back as the
/// drawing S-JTSK is read from; true north turned back with it. Empty
/// without a map or an S-JTSK origin, or when nothing moved.
pub fn reset(model: &Model) -> Transaction {
    let mut out = Transaction::new("Reset the map's position");
    let Some((pid, info)) = model.project_info() else {
        return out;
    };
    let Some(now_origin) = info.sjtsk else {
        return out;
    };
    // as it came down
    let (origin, north) = info.map_origin.unwrap_or((now_origin, info.true_north));
    let shift = DVec2::new(origin[0], origin[1]);
    let (mut members, _) = group(model);
    if members.is_empty() {
        return out;
    }
    // the ground point that came down at ±0,000 goes back there, turned
    // back as far as the ground has turned since
    let back = model_point_of(model, shift).map(|q| Rigid {
        about: q,
        degrees: north - info.true_north,
        by: -q,
    });
    members.extend(ground_sketches(model));
    for id in &members {
        let Some(e) = model.get(*id) else {
            continue;
        };
        let now = match &e.kind {
            // a drawing in S-JTSK: straight, its origin at ±0,000
            ElementKind::Reference(r) if is_jtsk_drawing(r) => {
                let mut n = e.clone();
                if let ElementKind::Reference(q) = &mut n.kind {
                    q.rotation = 0.0;
                    q.at = q.centre() - shift;
                }
                Some(n)
            }
            _ => back.and_then(|b| moved(model, e, &b)),
        };
        if let Some(n) = now {
            if n != *e {
                out = out.replace(e.clone(), n);
            }
        }
    }
    let mut p = info.clone();
    p.sjtsk = Some(origin);
    p.true_north = north;
    p.map_origin = None;
    if p != *info {
        if let Some(e) = model.get(pid) {
            let mut now = e.clone();
            now.kind = ElementKind::Project(p);
            out = out.replace(e.clone(), now);
        }
    }
    out
}

/// The rigid move that lays the ground's corner `ground.0`, its side
/// running to `ground.1`, on the building's corner `building.0`, its side
/// running to `building.1`.
pub fn fit(ground: (DVec2, DVec2), building: (DVec2, DVec2)) -> Rigid {
    let turn =
        ((building.1 - building.0).to_angle() - (ground.1 - ground.0).to_angle()).to_degrees();
    let degrees = (turn + 180.0).rem_euclid(360.0) - 180.0;
    Rigid {
        about: ground.0,
        degrees,
        by: building.0 - ground.0,
    }
}

/// The rigid move of the ground that sets the building exactly on the
/// plot: the side boundary `side` turned parallel to the building's side
/// `edge`, `to_side` metres out from it, and the building slid along it
/// until its nearest corner lies `to_rear` metres square off the rear
/// boundary `rear`, which may run askew (George: "2 m from the side
/// boundary, 5 m from the rear one"). `corners` are the building's
/// outline. The plot's inside is taken as the side of each boundary the
/// other one's far end lies on.
pub fn fit_exact(
    side: (DVec2, DVec2),
    edge: (DVec2, DVec2),
    rear: (DVec2, DVec2),
    corners: &[DVec2],
    to_side: f64,
    to_rear: f64,
) -> Result<Rigid, String> {
    let unit = |d: DVec2| d.try_normalize().ok_or("A line has no length.".to_string());
    if corners.is_empty() {
        return Err("The building has no outline in this plan.".into());
    }
    let centre = corners.iter().copied().sum::<DVec2>() / corners.len() as f64;
    let e = unit(edge.1 - edge.0)?;
    // out of the building, square off its side
    let mut n = e.perp();
    if (centre - edge.0).dot(n) > 0.0 {
        n = -n;
    }
    // the plot's inside off each boundary: toward the other's far end
    let far = |l: (DVec2, DVec2), from: (DVec2, DVec2)| -> Result<DVec2, String> {
        let d = unit(from.1 - from.0)?;
        let off = |p: DVec2| (p - from.0).perp_dot(d).abs();
        Ok(if off(l.0) > off(l.1) { l.0 } else { l.1 })
    };
    let inward = |l: (DVec2, DVec2), towards: DVec2| -> Result<DVec2, String> {
        let mut m = unit(l.1 - l.0)?.perp();
        if (towards - l.0).dot(m) < 0.0 {
            m = -m;
        }
        Ok(m)
    };
    let side_in = inward(side, far(rear, side)?)?;
    // turned so the side boundary's inside faces the building
    let turn = (-n).to_angle() - side_in.to_angle();
    let r = DVec2::from_angle(turn);
    let about = side.0;
    let at = |p: DVec2| about + r.rotate(p - about);
    // out from the building's side by to_side
    let s = (at(side.0) - edge.0).dot(n);
    let mut by = n * (to_side - s);
    // along it until the nearest corner is to_rear off the rear boundary
    let (r0, r1) = (at(rear.0) + by, at(rear.1) + by);
    let m = inward((r0, r1), at(far(side, rear)?) + by)?;
    let k = e.dot(m);
    if k.abs() < 1e-6 {
        return Err(
            "The two boundaries run side by side: pick one the building backs onto.".into(),
        );
    }
    let nearest = corners
        .iter()
        .map(|c| (*c - r0).dot(m))
        .fold(f64::INFINITY, f64::min);
    // moving the ground by u·e moves the corners by −u·e against it
    let u = (nearest - to_rear) / k;
    by += e * u;
    let degrees = (turn.to_degrees() + 180.0).rem_euclid(360.0) - 180.0;
    Ok(Rigid { about, degrees, by })
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

    #[test]
    fn fit_exact_sets_the_building_off_a_side_and_a_slanted_rear_boundary() {
        // a 10 × 6 house at the origin
        let corners = [
            DVec2::ZERO,
            DVec2::new(10.0, 0.0),
            DVec2::new(10.0, 6.0),
            DVec2::new(0.0, 6.0),
        ];
        // a plot far off and askew: its side boundary, its rear one slanted
        let r = DVec2::from_angle(0.6);
        let o = DVec2::new(40.0, -30.0);
        let g = |x: f64, y: f64| o + r.rotate(DVec2::new(x, y));
        let side = (g(0.0, 0.0), g(30.0, 0.0));
        let rear = (g(30.0, -2.0), g(26.0, 25.0));
        // the house's south side along the side boundary, 2 m off it
        let fit = fit_exact(side, (corners[0], corners[1]), rear, &corners, 2.0, 5.0).unwrap();
        let (a, b) = (fit.apply(side.0), fit.apply(side.1));
        let d = (b - a).normalize();
        assert!(d.perp_dot(DVec2::X).abs() < 1e-9, "parallel {d}");
        assert!((a.y - -2.0).abs() < 1e-9, "2 m off the side: {a}");
        let (p, q) = (fit.apply(rear.0), fit.apply(rear.1));
        let m = (q - p).normalize().perp();
        let dist = corners
            .iter()
            .map(|c| (*c - p).perp_dot(q - p).abs() / (q - p).length())
            .fold(f64::INFINITY, f64::min);
        assert!((dist - 5.0).abs() < 1e-9, "5 m off the rear: {dist}");
        // and on the plot's inside of the rear boundary, as of the side one
        let sides: Vec<f64> = corners.iter().map(|c| (*c - p).dot(m).signum()).collect();
        assert!(sides.iter().all(|x| *x == sides[0]));
        let far = fit.apply(side.0);
        assert_eq!((far - p).dot(m).signum(), sides[0]);
    }

    #[test]
    fn the_sjtsk_origin_follows_the_ground_under_the_building() {
        for with_drawing in [true, false] {
            let (mut doc, _, _) = mapped();
            if !with_drawing {
                // a map whose drawing is not the one S-JTSK is read from
                let (pid, info) = doc.model().project_info().unwrap();
                let mut i = info.clone();
                i.map_ref = None;
                let e = doc.model().get(pid).unwrap().clone();
                let mut n = e.clone();
                n.kind = ElementKind::Project(i);
                doc.commit(Transaction::new("no ref").replace(e, n))
                    .unwrap();
            }
            let o0 = doc.model().project_info().unwrap().1.sjtsk.unwrap();
            let r = Rigid {
                about: DVec2::new(-20.0, 7.0),
                degrees: -35.0,
                by: DVec2::new(12.0, -4.0),
            };
            // the ground point that comes to ±0,000
            let p = r.unapply(DVec2::ZERO);
            let s = sjtsk_of(doc.model(), p).unwrap();
            doc.commit(move_ground(doc.model(), &r)).unwrap();
            let info = doc.model().project_info().unwrap().1.clone();
            let o = info.sjtsk.unwrap();
            assert!(
                (DVec2::from(o) - s).length() < 1e-5,
                "{with_drawing}: {o:?} {s}"
            );
            // and every point reads its ground's coordinates
            let at = sjtsk_of(doc.model(), r.apply(DVec2::new(3.0, 4.0))).unwrap();
            let was = DVec2::new(o0[0] + 3.0, o0[1] + 4.0);
            assert!((at - was).length() < 1e-5, "{with_drawing}: {at} {was}");
            assert_eq!(info.map_origin, Some((o0, 0.0)));
            doc.commit(reset(doc.model())).unwrap();
            let back = doc.model().project_info().unwrap().1.clone();
            assert_eq!(back.sjtsk, Some(o0));
            assert!(back.true_north.abs() < 1e-9);
            let at = sjtsk_of(doc.model(), DVec2::new(3.0, 4.0)).unwrap();
            assert!(
                (at - was).length() < 1e-5,
                "{with_drawing} reset: {at} {was}"
            );
        }
    }

    #[test]
    fn reset_puts_the_map_back_where_it_came_down() {
        let (mut doc, map, ortho) = mapped();
        let p = DVec2::new(3.0, 4.0);
        let before = sjtsk_of(doc.model(), p).unwrap();
        let place = |doc: &Document, id| match &doc.model().get(id).unwrap().kind {
            ElementKind::Reference(r) => (r.at, r.rotation),
            _ => unreachable!(),
        };
        let o0 = place(&doc, ortho);
        let r = Rigid {
            about: DVec2::new(10.0, -2.0),
            degrees: 30.0,
            by: DVec2::new(7.0, 1.0),
        };
        doc.commit(move_ground(doc.model(), &r)).unwrap();
        assert!((sjtsk_of(doc.model(), p).unwrap() - before).length() > 1.0);
        doc.commit(reset(doc.model())).unwrap();
        assert!((sjtsk_of(doc.model(), p).unwrap() - before).length() < 1e-6);
        // a drawing in S-JTSK that had slipped 5 m off the other comes
        // back to its own coordinates too
        let a = place(&doc, ortho);
        assert!(a.0.length() < 1e-6 && a.1.abs() < 1e-9, "{a:?} {o0:?}");
        assert!(doc.model().project_info().unwrap().1.true_north.abs() < 1e-9);
        let _ = map;
        // nothing moved: nothing to do
        assert!(reset(doc.model()).ops.is_empty());
    }

    #[test]
    fn fit_lays_the_ground_corner_and_side_on_the_building() {
        let g = (
            DVec2::new(-25.0, 12.0),
            DVec2::new(-25.0, 12.0) + DVec2::from_angle(0.35),
        );
        let b = (DVec2::new(-0.15, -0.15), DVec2::new(10.15, -0.15));
        let r = fit(g, b);
        assert!(r.apply(g.0).distance(b.0) < 1e-9);
        let dir = (r.apply(g.1) - r.apply(g.0)).normalize();
        assert!(dir.distance(DVec2::X) < 1e-9, "{dir}");
    }

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
