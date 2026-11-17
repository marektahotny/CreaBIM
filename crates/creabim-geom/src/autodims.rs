//! Dimensioning a floor plan's doors and windows the way PilonDesigner
//! does (after STN 01 3420): on each side of the building, outside it, a
//! chain through the jambs of the doors and windows in the walls seen from
//! that side, then (above and to the left) a chain through the faces of
//! the walls running across that side, then the overall size.
//!
//! The chains are made of ordinary dimensions whose ends are bound to the
//! walls and openings they measure, so they follow them when they move or
//! change size, and the padlock can hold any of them. A link across a
//! whole door or window writes its height (and a window's sill) under the
//! line (see [`crate::annotate::opening_below`]).
//!
//! Only straight walls running across or up the drawing take part.

use crate::annotate::{attach, resolve};
use creabim_core::{
    DimAxis, Dimension, Element, ElementId, ElementKind, Model, Transaction, View, ViewKind,
};
use glam::DVec2;

/// One end of a link in a chain: where it is along the side, the element
/// it is bound to, and the point on it.
#[derive(Clone, Copy, Debug)]
struct Stop {
    at: f64,
    host: ElementId,
    point: DVec2,
}

/// A straight wall running across (`horizontal`) or up the drawing.
struct Run {
    id: ElementId,
    horizontal: bool,
    /// Its centreline's place across the side, and its span along it.
    across: f64,
    lo: f64,
    hi: f64,
    half: f64,
}

fn runs(model: &Model, level: ElementId) -> Vec<Run> {
    model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Wall(w)
                if w.level == level
                    && w.bulge == 0.0
                    && w.curtain.is_none()
                    && w.length() > 1e-6 =>
            {
                let d = w.end - w.start;
                let horizontal = d.y.abs() < 1e-6;
                if !horizontal && d.x.abs() > 1e-6 {
                    return None;
                }
                let (across, a, b) = if horizontal {
                    (w.start.y, w.start.x, w.end.x)
                } else {
                    (w.start.x, w.start.y, w.end.y)
                };
                Some(Run {
                    id: e.id,
                    horizontal,
                    across,
                    lo: a.min(b),
                    hi: a.max(b),
                    half: w.thickness / 2.0,
                })
            }
            _ => None,
        })
        .collect()
}

/// The dimensions that dimension the doors and windows of the plan `view`,
/// leaving out any the view has already between the same two points.
pub fn opening_chains(model: &Model, view_id: ElementId) -> Vec<Dimension> {
    let Some(ElementKind::View(view)) = model.get(view_id).map(|e| &e.kind) else {
        return vec![];
    };
    let ViewKind::Plan { level, .. } = view.kind else {
        return vec![];
    };
    let runs = runs(model, level);
    if runs.is_empty() {
        return vec![];
    }
    // the building's box, faces and all
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for r in &runs {
        let (lo, hi, a0, a1) = (r.lo, r.hi, r.across - r.half, r.across + r.half);
        if r.horizontal {
            (x0, x1, y0, y1) = (x0.min(lo), x1.max(hi), y0.min(a0), y1.max(a1));
        } else {
            (y0, y1, x0, x1) = (y0.min(lo), y1.max(hi), x0.min(a0), x1.max(a1));
        }
    }
    // a corner sticks out by the half thickness of the wall that ends there
    for r in &runs {
        if r.horizontal {
            let ends = runs.iter().filter(|o| !o.horizontal);
            for o in ends {
                if (o.across - r.lo).abs() < 1e-6 {
                    x0 = x0.min(o.across - o.half);
                }
                if (o.across - r.hi).abs() < 1e-6 {
                    x1 = x1.max(o.across + o.half);
                }
            }
        }
    }
    let k = view.scale / 100.0;
    let (first, step) = (0.35 * k, 0.8 * k);
    let mut out = vec![];
    // top, left, bottom, right: (along x?, its edge, outward)
    let sides = [
        (true, y1, 1.0),
        (false, x0, -1.0),
        (true, y0, -1.0),
        (false, x1, 1.0),
    ];
    for (i, &(horizontal, edge, side)) in sides.iter().enumerate() {
        let (lo, hi) = if horizontal { (x0, x1) } else { (y0, y1) };
        let pt = |along: f64, across: f64| {
            if horizontal {
                DVec2::new(along, across)
            } else {
                DVec2::new(across, along)
            }
        };
        // the walls seen from this side: their outer face is outermost
        // wherever they run
        let face = |r: &Run| r.across + side * r.half;
        let facade: Vec<&Run> = runs
            .iter()
            .filter(|r| r.horizontal == horizontal)
            .filter(|r| {
                !runs.iter().any(|o| {
                    o.horizontal == horizontal
                        && o.id != r.id
                        && o.lo < r.hi - 1e-3
                        && o.hi > r.lo + 1e-3
                        && (face(o) - face(r)) * side > 1e-3
                })
            })
            .collect();
        let Some(outer) = facade
            .iter()
            .max_by(|a, b| (face(a) * side).total_cmp(&(face(b) * side)))
        else {
            continue;
        };
        let ends = |stops: &mut Vec<Stop>| {
            // the box's corners, on the outermost wall of this side
            let lo_run = facade
                .iter()
                .min_by(|a, b| a.lo.total_cmp(&b.lo))
                .unwrap_or(outer);
            let hi_run = facade
                .iter()
                .max_by(|a, b| a.hi.total_cmp(&b.hi))
                .unwrap_or(outer);
            stops.push(Stop {
                at: lo,
                host: lo_run.id,
                point: pt(lo, face(lo_run)),
            });
            stops.push(Stop {
                at: hi,
                host: hi_run.id,
                point: pt(hi, face(hi_run)),
            });
        };
        let mut row = 0.0;
        let mut chain = |stops: Vec<Stop>, out: &mut Vec<Dimension>| {
            let at = edge + side * (first + step * (row + 1.0));
            row += 1.0;
            out.extend(links(model, view, view_id, horizontal, at, stops));
        };
        // the doors and windows in the walls seen from here
        let mut stops = vec![];
        for r in &facade {
            for e in model.iter() {
                let ElementKind::Opening(o) = &e.kind else {
                    continue;
                };
                if o.host != r.id {
                    continue;
                }
                let Some(ElementKind::Wall(w)) = model.get(r.id).map(|h| &h.kind) else {
                    continue;
                };
                let d = w.direction();
                for u in [o.offset, o.offset + o.width] {
                    let p = w.start + d * u;
                    let along = if horizontal { p.x } else { p.y };
                    stops.push(Stop {
                        at: along,
                        host: e.id,
                        point: pt(along, face(r)),
                    });
                }
            }
        }
        if !stops.is_empty() {
            ends(&mut stops);
            chain(stops, &mut out);
        }
        if i < 2 {
            // the faces of the walls running across this side
            let mut stops = vec![];
            for r in runs.iter().filter(|r| r.horizontal != horizontal) {
                // its end nearer this side
                let near = if side > 0.0 { r.hi } else { r.lo };
                for f in [r.across - r.half, r.across + r.half] {
                    stops.push(Stop {
                        at: f,
                        host: r.id,
                        point: pt(f, near),
                    });
                }
            }
            ends(&mut stops);
            let mut s = stops.clone();
            s.sort_by(|a, b| a.at.total_cmp(&b.at));
            s.dedup_by(|a, b| (a.at - b.at).abs() < 1e-4);
            if s.len() > 2 {
                chain(stops, &mut out);
            }
            let mut total = vec![];
            ends(&mut total);
            chain(total, &mut out);
        }
    }
    // leave out what the view already has
    let have: Vec<(DVec2, DVec2)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Dimension(d) if d.view == view_id && d.kind.is_linear() => {
                Some((resolve(model, view, &d.a)?, resolve(model, view, &d.b)?))
            }
            _ => None,
        })
        .collect();
    out.retain(|d| {
        let (Some(a), Some(b)) = (resolve(model, view, &d.a), resolve(model, view, &d.b)) else {
            return true;
        };
        !have.iter().any(|(p, q)| {
            (p.distance(a) < 1e-3 && q.distance(b) < 1e-3)
                || (p.distance(b) < 1e-3 && q.distance(a) < 1e-3)
        })
    });
    out
}

/// A chain's links, one dimension between each two stops next to each
/// other along the side, their line `at` across it.
fn links(
    model: &Model,
    view: &View,
    view_id: ElementId,
    horizontal: bool,
    at: f64,
    mut stops: Vec<Stop>,
) -> Vec<Dimension> {
    stops.sort_by(|a, b| a.at.total_cmp(&b.at));
    stops.dedup_by(|a, b| (a.at - b.at).abs() < 1e-4);
    stops
        .windows(2)
        .map(|w| {
            let (s, t) = (w[0], w[1]);
            // the link's line runs `at` across: left of a → b is +y for a
            // chain along x, −x for one up y
            let offset = if horizontal {
                at - s.point.y
            } else {
                s.point.x - at
            };
            let mut d = Dimension::linear(
                view_id,
                attach(model, view, Some(s.host), s.point),
                attach(model, view, Some(t.host), t.point),
                offset,
            );
            d.axis = if horizontal {
                DimAxis::Horizontal
            } else {
                DimAxis::Vertical
            };
            d
        })
        .collect()
}

/// The transaction that adds [`opening_chains`] to the plan `view`, with
/// how many it adds.
pub fn dimension_openings(model: &Model, view: ElementId) -> (Transaction, usize) {
    let dims = opening_chains(model, view);
    let n = dims.len();
    let mut tx = Transaction::new("Dimension doors and windows");
    for d in dims {
        tx = tx.insert(Element::new("Dimension", ElementKind::Dimension(d)));
    }
    (tx, n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::{Level, Opening, OpeningKind, Wall};

    #[test]
    fn a_box_with_a_window_gets_its_chains() {
        let mut m = Model::new();
        let lvl = Element::new(
            "Ground",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let lid = lvl.id;
        let plan = Element::new("Plan", ElementKind::View(View::plan(lid)));
        let view = plan.id;
        let mut tx = Transaction::new("set up").insert(lvl).insert(plan);
        let wall = |a: (f64, f64), b: (f64, f64)| {
            Element::new(
                "W",
                ElementKind::Wall(Wall::new(
                    lid,
                    DVec2::new(a.0, a.1),
                    DVec2::new(b.0, b.1),
                    0.4,
                    2.7,
                )),
            )
        };
        // a 10 × 8 box of 400 walls on their centrelines
        let top = wall((0.0, 8.0), (10.0, 8.0));
        let top_id = top.id;
        for w in [
            wall((0.0, 0.0), (10.0, 0.0)),
            top,
            wall((0.0, 0.0), (0.0, 8.0)),
            wall((10.0, 0.0), (10.0, 8.0)),
        ] {
            tx = tx.insert(w);
        }
        let win = Element::new(
            "O01",
            ElementKind::Opening(Opening {
                host: top_id,
                kind: OpeningKind::Window,
                offset: 2.0,
                sill: 0.9,
                width: 1.5,
                height: 1.2,
                material: None,
                frame: Default::default(),
                flip_hand: false,
                flip_facing: false,
            }),
        );
        tx = tx.insert(win);
        tx.apply(&mut m).unwrap();
        let (tx, n) = dimension_openings(&m, view);
        // top: window chain (3 links), walls (3 links: −0.2..0.2..9.8..10.2),
        // total; left: walls (3) and total; bottom, right: none
        assert_eq!(n, 3 + 3 + 1 + 3 + 1, "{n}");
        tx.apply(&mut m).unwrap();
        let ElementKind::View(v) = &m.get(view).unwrap().kind else {
            unreachable!()
        };
        let items = crate::annotate::annotations(&m, view, v);
        let across: Vec<_> = items
            .iter()
            .filter_map(|it| match it {
                crate::plan::PlanItem::Dimension { text, below, .. } => {
                    below.as_ref().map(|b| (text.clone(), b.clone()))
                }
                _ => None,
            })
            .collect();
        assert_eq!(across.len(), 1, "{across:?}");
        assert!(across[0].1.contains("("), "{across:?}");
        // running it again adds nothing
        assert_eq!(opening_chains(&m, view).len(), 0);
    }
}
