//! A plan turned on paper: a view's [`creabim_core::View::turn`] (a site
//! plan lying square to its parcel while the floor plans stay square to
//! the building). The model is not touched; only the drawing turns about
//! the model's origin, its text staying upright.

use crate::annotation::Shape;
use crate::plan::PlanItem;
use crate::section::Drawing;
use creabim_core::{ElementId, ElementKind, Model};
use glam::DVec2;

/// How far `view` is turned, degrees anticlockwise (0 for a view that is
/// not, or no view).
pub fn view_turn(model: &Model, view: ElementId) -> f64 {
    match model.get(view).map(|e| &e.kind) {
        Some(ElementKind::View(v)) => v.turn,
        _ => 0.0,
    }
}

/// `view`'s drawing as it lies on paper: turned by its turn.
pub fn paper_drawing(model: &Model, view: ElementId) -> Option<Drawing> {
    let d = crate::section::view_drawing(model, view)?;
    Some(turned(d, view_turn(model, view)))
}

/// `d` turned `degrees` anticlockwise about the origin.
pub fn turned(mut d: Drawing, degrees: f64) -> Drawing {
    if degrees.rem_euclid(360.0).abs() < 1e-12 {
        return d;
    }
    let a = degrees.to_radians();
    let r = DVec2::from_angle(a);
    for item in &mut d.items {
        turn_item(item, r, a);
    }
    d
}

/// One item turned `radians` anticlockwise about the origin.
pub fn turn_item_by(item: &mut PlanItem, radians: f64) {
    turn_item(item, DVec2::from_angle(radians), radians);
}

fn turn_item(item: &mut PlanItem, r: DVec2, a: f64) {
    let t = |p: &mut DVec2| *p = r.rotate(*p);
    match item {
        PlanItem::Polygon { points, .. }
        | PlanItem::Fill { points, .. }
        | PlanItem::Tint { points, .. }
        | PlanItem::Room { points, .. }
        | PlanItem::Shade { points, .. }
        | PlanItem::Curve { points, .. } => points.iter_mut().for_each(t),
        PlanItem::Line { a: p, b: q, .. } => {
            t(p);
            t(q);
        }
        PlanItem::Arc {
            center, start, end, ..
        } => {
            t(center);
            *start += a;
            *end += a;
        }
        PlanItem::Label { at, .. } | PlanItem::Text { at, .. } => t(at),
        PlanItem::Dimension {
            a: p,
            b: q,
            witness,
            place,
            ..
        } => {
            t(p);
            t(q);
            if let Some(w) = witness {
                w.iter_mut().for_each(t);
            }
            t(&mut place.shift);
        }
        PlanItem::Tag {
            anchor, at, elbow, ..
        } => {
            t(anchor);
            t(at);
            if let Some(e) = elbow {
                t(e);
            }
        }
        PlanItem::Annotation { item, .. } => match &mut item.shape {
            Shape::Angle {
                vertex, a: p, b: q, ..
            } => {
                t(vertex);
                t(p);
                t(q);
            }
            Shape::Radius { center, at, .. } => {
                t(center);
                t(at);
            }
            Shape::ArcLength {
                center, a: p, b: q, ..
            } => {
                t(center);
                t(p);
                t(q);
            }
            Shape::Note {
                at, leader, elbow, ..
            } => {
                t(at);
                if let Some(l) = leader {
                    t(l);
                }
                if let Some(e) = elbow {
                    t(e);
                }
            }
            Shape::Spot { at, text_at, .. } => {
                t(at);
                t(text_at);
            }
            Shape::Slope { a: p, b: q, .. } => {
                t(p);
                t(q);
            }
            Shape::Along { at, angle } => {
                t(at);
                *angle += a;
            }
            Shape::Level { at, line_end, .. } => {
                t(at);
                if let Some(l) = line_end {
                    t(l);
                }
            }
        },
        // shaded surfaces lie square to the paper: only where they sit turns
        PlanItem::Image { lo, hi, clip, .. } => {
            let mid = (*lo + *hi) * 0.5;
            let d = r.rotate(mid) - mid;
            *lo += d;
            *hi += d;
            if let Some((p, q)) = clip {
                *p += d;
                *q += d;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::Pen;

    #[test]
    fn a_quarter_turn_lays_a_line_across_and_leaves_text_upright() {
        let d = Drawing {
            items: vec![
                PlanItem::Line {
                    id: ElementId::NIL,
                    pen: Pen::Cut,
                    a: DVec2::ZERO,
                    b: DVec2::new(0.0, 10.0),
                },
                PlanItem::Label {
                    id: ElementId::NIL,
                    at: DVec2::new(1.0, 0.0),
                    text: "A".into(),
                },
            ],
        };
        // a quarter turn clockwise: up the paper becomes across it
        let d = turned(d, -90.0);
        let PlanItem::Line { b, .. } = &d.items[0] else {
            panic!()
        };
        assert!((*b - DVec2::new(10.0, 0.0)).length() < 1e-9, "{b}");
        let PlanItem::Label { at, .. } = &d.items[1] else {
            panic!()
        };
        assert!((*at - DVec2::new(0.0, -1.0)).length() < 1e-9, "{at}");
    }
}
