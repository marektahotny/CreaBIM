//! Columns and beams (lintels too) wrapped in thermal insulation with an
//! exterior render on it, side by side: each side its own thickness, none
//! on a side that stays bare.
//!
//! The boards are elements of their own (columns round a column, beams
//! along a beam), in the insulation's and the render's materials, so
//! plans, sections, 3D and the take-off show and count them as any other.
//! [`settle`] keeps them to their element after every edit; they are
//! never edited by hand.
//!
//! A column's sides, in its own axes (its width along x, its depth along
//! y): 0 front (−y), 1 right (+x), 2 back (+y), 3 left (−x). A column put
//! into a wall has its front on the wall's right side (the exterior,
//! unless the wall is flipped). A beam's: 0 left, 1 right (looking from
//! its start to its end), 2 under, 3 over. A lintel's right is its wall's
//! right.

use crate::members::{Beam, Column, Profile};
use crate::ops::Transaction;
use crate::{Element, ElementId, ElementKind, Model};
use glam::DVec2;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The insulation and render round a column or beam.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Wrap {
    /// The insulation's material.
    #[serde(default)]
    pub insulation: Option<ElementId>,
    /// Its thickness on each side, m (0: none there). See the module for
    /// which side is which.
    #[serde(default)]
    pub sides: [f64; 4],
    /// The render on the insulation (only where there is insulation).
    #[serde(default)]
    pub plaster: Option<ElementId>,
    /// The render's thickness, m (0: none).
    #[serde(default = "default_plaster")]
    pub plaster_thickness: f64,
}

/// A thin-layer render with its adhesive and mesh, m.
pub const PLASTER: f64 = 0.01;

fn default_plaster() -> f64 {
    PLASTER
}

impl Wrap {
    /// The insulation's thickness on side `i`, never below 0.
    pub fn side(&self, i: usize) -> f64 {
        self.sides.get(i).copied().unwrap_or(0.0).max(0.0)
    }

    /// The render's thickness on side `i`: only over insulation.
    pub fn plaster_on(&self, i: usize) -> f64 {
        if self.side(i) > 1e-6 && self.plaster.is_some() {
            self.plaster_thickness.max(0.0)
        } else {
            0.0
        }
    }

    pub fn is_empty(&self) -> bool {
        (0..4).all(|i| self.side(i) < 1e-6)
    }
}

/// A board of a wrap: which element, which side, insulation or render.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WrapPiece {
    pub host: ElementId,
    pub side: u8,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub plaster: bool,
}

/// The materials a wrap usually takes, with their λ (W/(m·K), STN 73
/// 0540-3 and the makers' sheets) and the library material they look
/// like: insulation first, then renders.
pub const PRESETS: [(&str, &str, f64, bool); 5] = [
    ("EPS 70 F fasádny", "Insulation, EPS", 0.039, false),
    ("XPS (styrodur)", "Insulation, XPS", 0.034, false),
    (
        "Minerálna vlna fasádna",
        "Insulation, mineral wool",
        0.040,
        false,
    ),
    ("Tenkovrstvová omietka", "Render, lime", 0.70, true),
    ("Vápennocementová omietka", "Render, cement", 0.88, true),
];

/// The preset material `name` in `model`, or a new one to insert (with
/// its λ). `None` for a name that is no preset and not in the model.
pub fn preset(model: &Model, name: &str) -> Option<(ElementId, Option<Element>)> {
    if let Some(e) = model
        .iter()
        .find(|e| e.name == name && matches!(e.kind, ElementKind::Material(_)))
    {
        return Some((e.id, None));
    }
    let (_, lib, lambda, _) = PRESETS.iter().find(|p| p.0 == name)?;
    let (_, Some(mut e)) = crate::library::find_or_new(&Model::default(), lib)? else {
        return None;
    };
    e.name = name.to_string();
    if let ElementKind::Material(m) = &mut e.kind {
        m.lambda = Some(*lambda);
    }
    Some((e.id, Some(e)))
}

/// A new wrap: EPS on `sides`, thin-layer render on it, the materials
/// found in `model` or to insert.
pub fn new_wrap(model: &Model, sides: [f64; 4]) -> (Wrap, Vec<Element>) {
    let mut new = vec![];
    let mut pick = |name: &str| {
        preset(model, name).map(|(id, e)| {
            new.extend(e);
            id
        })
    };
    let w = Wrap {
        insulation: pick(PRESETS[0].0),
        sides,
        plaster: pick(PRESETS[3].0),
        plaster_thickness: PLASTER,
    };
    (w, new)
}

/// A rectangle in a member's own axes: (lo, hi) along each.
type Rect = ((f64, f64), (f64, f64));

/// The boards round a core spanning `a` × `b` (a column's x × y, or a
/// beam's across × up): for each side and layer, the rectangle it takes.
/// Sides go `lo_b, hi_a, hi_b, lo_a` for a column (front, right, back,
/// left); a beam maps its own onto them.
fn boards(w: &Wrap, a: (f64, f64), b: (f64, f64), map: [usize; 4]) -> Vec<(u8, bool, Rect)> {
    // the insulation on lo_b, hi_a, hi_b, lo_a
    let t = |k: usize| w.side(map[k]);
    let q = |k: usize| w.plaster_on(map[k]);
    let (a0, a1) = (a.0 - t(3), a.1 + t(1));
    let (b0, b1) = (b.0 - t(0), b.1 + t(2));
    let mut out = vec![];
    let mut put = |k: usize, plaster: bool, r: Rect| {
        if (r.0 .1 - r.0 .0) > 1e-6 && (r.1 .1 - r.1 .0) > 1e-6 {
            out.push((map[k] as u8, plaster, r));
        }
    };
    // insulation: along b's faces the boards run over the corners
    put(0, false, ((a0, a1), (b0, b.0)));
    put(2, false, ((a0, a1), (b.1, b1)));
    put(1, false, ((a.1, a1), b));
    put(3, false, ((a0, a.0), b));
    // the render over it, the same way round
    put(0, true, ((a0 - q(3), a1 + q(1)), (b0 - q(0), b0)));
    put(2, true, ((a0 - q(3), a1 + q(1)), (b1, b1 + q(2))));
    put(1, true, ((a1, a1 + q(1)), (b0, b1)));
    put(3, true, ((a0 - q(3), a0), (b0, b1)));
    out
}

fn material_of(w: &Wrap, plaster: bool) -> Option<ElementId> {
    if plaster {
        w.plaster
    } else {
        w.insulation
    }
}

/// The boards round column `c` (`id`): rectangular columns only.
pub fn column_boards(id: ElementId, c: &Column) -> Vec<Column> {
    let Some(w) = &c.wrap else { return vec![] };
    let Profile::Rectangle { width, depth } = c.profile else {
        return vec![];
    };
    if c.section.is_some() {
        return vec![];
    }
    let turn = DVec2::from_angle(c.rotation.to_radians());
    let (x, y) = (width / 2.0, depth / 2.0);
    boards(w, (-x, x), (-y, y), [0, 1, 2, 3])
        .into_iter()
        .map(|(side, plaster, ((x0, x1), (y0, y1)))| {
            let mut b = c.clone();
            b.position = c.position + turn.rotate(DVec2::new((x0 + x1) / 2.0, (y0 + y1) / 2.0));
            b.profile = Profile::Rectangle {
                width: x1 - x0,
                depth: y1 - y0,
            };
            b.material = material_of(w, plaster);
            b.wrap = None;
            b.in_wall = None;
            b.piece = Some(WrapPiece {
                host: id,
                side,
                plaster,
            });
            b
        })
        .collect()
}

/// The boards along beam `b` (`id`): straight, level beams of their own
/// width and depth only.
pub fn beam_boards(id: ElementId, b: &Beam) -> Vec<Beam> {
    let Some(w) = &b.wrap else { return vec![] };
    if b.section.is_some() || b.up.is_some() || b.rise != [0.0, 0.0] || b.length() < 1e-6 {
        return vec![];
    }
    let n = (b.end - b.start).normalize_or_zero().perp();
    let (x, d) = (b.width / 2.0, b.depth);
    // across (+ to the left) × up from the top: right is lo across, left
    // hi; under is lo up, over hi
    boards(w, (-x, x), (-d, 0.0), [2, 0, 3, 1])
        .into_iter()
        .map(|(side, plaster, ((a0, a1), (z0, z1)))| {
            let mut p = b.clone();
            let off = n * ((a0 + a1) / 2.0);
            p.start = b.start + off;
            p.end = b.end + off;
            p.width = a1 - a0;
            p.depth = z1 - z0;
            p.top_offset = b.top_offset - z1;
            p.material = material_of(w, plaster);
            p.wrap = None;
            p.lintel = None;
            p.piece = Some(WrapPiece {
                host: id,
                side,
                plaster,
            });
            p
        })
        .collect()
}

/// The board `e` is, if any.
pub fn piece_of(e: &Element) -> Option<WrapPiece> {
    match &e.kind {
        ElementKind::Column(c) => c.piece,
        ElementKind::Beam(b) => b.piece,
        _ => None,
    }
}

/// The boards every wrapped column and beam should have, as the model
/// is now, made so: boards moved, added and taken away. Run by the
/// [`crate::Document`] after every edit.
pub fn settle(model: &Model) -> Transaction {
    let mut out = Transaction::new("Wrap");
    let mut have: HashMap<ElementId, Vec<&Element>> = HashMap::new();
    let mut hosts = vec![];
    for e in model.iter() {
        if let Some(p) = piece_of(e) {
            have.entry(p.host).or_default().push(e);
        }
        match &e.kind {
            ElementKind::Column(c) if c.wrap.is_some() => hosts.push(e),
            ElementKind::Beam(b) if b.wrap.is_some() => hosts.push(e),
            _ => {}
        }
    }
    if hosts.is_empty() && have.is_empty() {
        return out;
    }
    let mut kept = vec![];
    for h in hosts {
        let want: Vec<ElementKind> = match &h.kind {
            ElementKind::Column(c) => column_boards(h.id, c)
                .into_iter()
                .map(ElementKind::Column)
                .collect(),
            ElementKind::Beam(b) => beam_boards(h.id, b)
                .into_iter()
                .map(ElementKind::Beam)
                .collect(),
            _ => vec![],
        };
        let mut old = have.remove(&h.id).unwrap_or_default();
        for kind in want {
            let key = |k: &ElementKind| match k {
                ElementKind::Column(c) => c.piece.map(|p| (p.side, p.plaster)),
                ElementKind::Beam(b) => b.piece.map(|p| (p.side, p.plaster)),
                _ => None,
            };
            let k = key(&kind);
            let name = if k.is_some_and(|k| k.1) {
                "Omietka"
            } else {
                "Zateplenie"
            };
            match old.iter().position(|e| key(&e.kind) == k) {
                Some(i) => {
                    let e = old.remove(i);
                    if e.kind != kind {
                        let mut after = e.clone();
                        after.kind = kind;
                        out = out.replace(e.clone(), after);
                    }
                    kept.push(e.id);
                }
                None => out = out.insert(Element::new(name, kind)),
            }
        }
        for e in old {
            out = out.remove_cascade(model, e.id);
        }
    }
    // boards of what is gone or no longer wrapped
    for list in have.values() {
        for e in list {
            out = out.remove_cascade(model, e.id);
        }
    }
    out
}

/// Which of `e`'s sides faces the exterior, as heat flows across it: a
/// column's front or back, a beam's left or right. In a wall, the wall's
/// exterior; else the side with more insulation (front, right on a tie).
pub fn exterior_side(model: &Model, e: &Element) -> Option<usize> {
    let wall_ext_right = |wall: ElementId| match model.get(wall).map(|w| &w.kind) {
        Some(ElementKind::Wall(w)) => {
            let (ext, int) = crate::walls::core_span(model, w);
            Some(ext < int)
        }
        _ => None,
    };
    match &e.kind {
        ElementKind::Column(c) => {
            if let Some(right) = c.in_wall.as_ref().and_then(|iw| wall_ext_right(iw.wall)) {
                return Some(if right { 0 } else { 2 });
            }
            let w = c.wrap.as_ref();
            let t = |i| w.map_or(0.0, |w| w.side(i));
            Some(if t(2) > t(0) { 2 } else { 0 })
        }
        ElementKind::Beam(b) => {
            let wall = b.lintel.as_ref().and_then(|l| l.opening).and_then(|o| {
                match model.get(o).map(|e| &e.kind) {
                    Some(ElementKind::Opening(o)) => Some(o.host),
                    _ => None,
                }
            });
            if let Some(right) = wall.and_then(wall_ext_right) {
                return Some(if right { 1 } else { 0 });
            }
            let w = b.wrap.as_ref();
            let t = |i| w.map_or(0.0, |w| w.side(i));
            Some(if t(0) > t(1) { 0 } else { 1 })
        }
        _ => None,
    }
}

/// The layers heat crosses through a wrapped column (front to back) or
/// beam (side to side), from the exterior in: render, insulation, the
/// column's own XPS in a wall, the core, and the same on the inner side.
pub fn layers_across(model: &Model, e: &Element) -> Option<Vec<(Option<ElementId>, f64)>> {
    let ext = exterior_side(model, e)?;
    let (wrap, core_mat, core, opposite, xps) = match &e.kind {
        ElementKind::Column(c) => {
            let Profile::Rectangle { depth, .. } = c.profile else {
                return None;
            };
            let xps = c
                .in_wall
                .as_ref()
                .filter(|iw| iw.xps && iw.of.is_none())
                .and_then(|iw| {
                    let ElementKind::Wall(w) = &model.get(iw.wall)?.kind else {
                        return None;
                    };
                    let (a, b) = crate::walls::core_span(model, w);
                    let t = iw.insulation((b - a).abs());
                    let m = model.iter().find_map(|b| match &b.kind {
                        ElementKind::Column(x)
                            if x.in_wall.as_ref().and_then(|i| i.of) == Some(e.id) =>
                        {
                            x.material
                        }
                        _ => None,
                    });
                    (t > 1e-6).then_some((m, t))
                });
            (c.wrap.as_ref(), c.material, depth, [2, 3, 0, 1], xps)
        }
        ElementKind::Beam(b) => (b.wrap.as_ref(), b.material, b.width, [1, 0, 3, 2], None),
        _ => return None,
    };
    if wrap.is_none() && xps.is_none() {
        return None;
    }
    let int = opposite[ext];
    let mut out = vec![];
    let side = |out: &mut Vec<_>, i: usize, outer_first: bool| {
        if let Some(w) = wrap {
            let mut l = vec![];
            if w.plaster_on(i) > 0.0 {
                l.push((w.plaster, w.plaster_on(i)));
            }
            if w.side(i) > 0.0 {
                l.push((w.insulation, w.side(i)));
            }
            if !outer_first {
                l.reverse();
            }
            out.extend(l);
        }
    };
    side(&mut out, ext, true);
    out.extend(xps);
    out.push((core_mat, core));
    side(&mut out, int, false);
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Document, Level};

    fn level(doc: &mut Document) -> ElementId {
        let l = Element::new(
            "1NP",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let id = l.id;
        doc.commit(Transaction::new("l").insert(l)).unwrap();
        id
    }

    fn boards_of(doc: &Document, host: ElementId) -> Vec<Element> {
        doc.model()
            .iter()
            .filter(|e| piece_of(e).is_some_and(|p| p.host == host))
            .cloned()
            .collect()
    }

    fn outline_box(pts: &[DVec2]) -> (DVec2, DVec2) {
        let lo = pts.iter().fold(DVec2::splat(f64::MAX), |a, p| a.min(*p));
        let hi = pts.iter().fold(DVec2::splat(f64::MIN), |a, p| a.max(*p));
        (lo, hi)
    }

    #[test]
    fn a_column_is_wrapped_side_by_side_and_the_boards_follow_it() {
        let mut doc = Document::new();
        let l = level(&mut doc);
        let mut c = Column::new(
            l,
            DVec2::new(1.0, 1.0),
            Profile::Rectangle {
                width: 0.3,
                depth: 0.3,
            },
        );
        // 100 mm in front, 50 on the right, none behind or on the left
        let (w, mats) = new_wrap(doc.model(), [0.1, 0.05, 0.0, 0.0]);
        c.wrap = Some(w);
        let ce = Element::new("Stĺp", ElementKind::Column(c));
        let id = ce.id;
        let mut tx = Transaction::new("c").insert(ce);
        for m in mats {
            tx = tx.insert(m);
        }
        doc.commit(tx).unwrap();
        let b = boards_of(&doc, id);
        // two insulation boards, two render boards
        assert_eq!(b.len(), 4, "{b:#?}");
        let all: Vec<DVec2> = b
            .iter()
            .flat_map(|e| match &e.kind {
                ElementKind::Column(c) => c.outline(),
                _ => vec![],
            })
            .collect();
        let (lo, hi) = outline_box(&all);
        // the core 0,85…1,15; front (−y) 100 + 10, right (+x) 50 + 10
        assert!((lo.y - (0.85 - 0.11)).abs() < 1e-9, "{lo:?}");
        assert!((hi.x - (1.15 + 0.06)).abs() < 1e-9, "{hi:?}");
        assert!((lo.x - 0.85).abs() < 1e-9 && (hi.y - 1.15).abs() < 1e-9);
        // moved and turned: the boards go with it
        doc.edit(id, "m", |e| {
            if let ElementKind::Column(c) = &mut e.kind {
                c.position = DVec2::new(5.0, 1.0);
                c.rotation = 90.0;
            }
        })
        .unwrap();
        let b = boards_of(&doc, id);
        assert_eq!(b.len(), 4);
        for e in &b {
            let ElementKind::Column(p) = &e.kind else {
                panic!()
            };
            assert!((p.rotation - 90.0).abs() < 1e-9);
            assert!((p.position - DVec2::new(5.0, 1.0)).length() < 0.3);
        }
        // the right side off: its boards go
        doc.edit(id, "w", |e| {
            if let ElementKind::Column(c) = &mut e.kind {
                c.wrap.as_mut().unwrap().sides[1] = 0.0;
            }
        })
        .unwrap();
        assert_eq!(boards_of(&doc, id).len(), 2);
        // U across it, front to back: render, EPS, the core
        let e = doc.model().get(id).unwrap().clone();
        let layers = layers_across(doc.model(), &e).unwrap();
        assert_eq!(layers.len(), 3);
        assert!((layers[1].1 - 0.1).abs() < 1e-9);
        // the column deleted: its boards with it
        let tx = Transaction::new("d").remove_cascade(doc.model(), id);
        doc.commit(tx).unwrap();
        assert!(boards_of(&doc, id).is_empty());
    }

    #[test]
    fn a_beam_is_wrapped_under_and_on_one_side() {
        let mut doc = Document::new();
        let l = level(&mut doc);
        let mut b = Beam::new(l, DVec2::ZERO, DVec2::new(2.0, 0.0));
        b.width = 0.25;
        b.depth = 0.25;
        b.top_offset = 0.5;
        // right 80, under 50
        let (w, mats) = new_wrap(doc.model(), [0.0, 0.08, 0.05, 0.0]);
        b.wrap = Some(w);
        let be = Element::new("Preklad", ElementKind::Beam(b));
        let id = be.id;
        let mut tx = Transaction::new("b").insert(be);
        for m in mats {
            tx = tx.insert(m);
        }
        doc.commit(tx).unwrap();
        let b = boards_of(&doc, id);
        assert_eq!(b.len(), 4, "{b:#?}");
        let beams: Vec<Beam> = b
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Beam(b) => Some(b.clone()),
                _ => None,
            })
            .collect();
        // the right board: on the −y side, as deep as the core (the under
        // board runs across under it)
        let right = beams
            .iter()
            .find(|b| b.piece.is_some_and(|p| p.side == 1 && !p.plaster))
            .unwrap();
        assert!(
            (right.start.y + 0.125 + 0.04).abs() < 1e-9,
            "{:?}",
            right.start
        );
        assert!((right.depth - 0.25).abs() < 1e-9);
        assert!((right.top_offset - 0.5).abs() < 1e-9);
        let under = beams
            .iter()
            .find(|b| b.piece.is_some_and(|p| p.side == 2 && !p.plaster))
            .unwrap();
        assert!((under.top_offset - 0.75).abs() < 1e-9);
        assert!((under.width - 0.33).abs() < 1e-9);
        let render = beams
            .iter()
            .find(|b| b.piece.is_some_and(|p| p.side == 2 && p.plaster))
            .unwrap();
        assert!((render.top_offset - 0.80).abs() < 1e-9);
        assert!((render.width - 0.34).abs() < 1e-9);
    }
}
