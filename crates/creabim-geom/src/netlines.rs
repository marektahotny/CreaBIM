//! Utility networks in a plan (see [`creabim_core::utility`]): what is
//! drawn on a network's line besides the line itself, its code written
//! into it, arrows the way a sewer flows, crosses on one to go.

use crate::annotation::{Annotation, Shape, TextLook};
use crate::plan::{Pen, PlanItem};
use creabim_core::{ElementId, ElementKind, Model};
use glam::DVec2;

/// The marks of the networks drawn in `items` (a plan at `mm` drawing
/// units per paper mm): every 40 mm an arrow or a cross, the code on each
/// stretch longer than 30 mm, and again every 120 mm.
pub fn net_marks(model: &Model, items: &[PlanItem], mm: f64) -> Vec<PlanItem> {
    let mut out = vec![];
    for it in items {
        let PlanItem::Line { id, a, b, .. } = it else {
            continue;
        };
        let Some(net) = net_of(model, *id) else {
            continue;
        };
        let (a, b) = (*a, *b);
        let len = a.distance(b);
        if len < 1e-6 {
            continue;
        }
        let u = (b - a) / len;
        let n = u.perp();
        let step = 40.0 * mm;
        let s = 1.6 * mm;
        let mut t = step / 2.0;
        while t < len {
            let c = a + u * t;
            if net.crossed() {
                for d in [(u + n).normalize(), (u - n).normalize()] {
                    out.push(line(*id, c - d * s, c + d * s));
                }
            }
            if net.flows() {
                let back = c - u * s * 1.4;
                out.push(line(*id, back + n * s * 0.7, c));
                out.push(line(*id, back - n * s * 0.7, c));
            }
            t += step;
        }
        let count = ((len / (120.0 * mm)).floor() as usize).max(usize::from(len > 30.0 * mm));
        let look = TextLook {
            size: 2.5,
            rgb: Some(net.kind.rgb()),
            bold: true,
            ..TextLook::default()
        };
        let mut angle = u.to_angle();
        if angle > std::f64::consts::FRAC_PI_2 + 1e-9 {
            angle -= std::f64::consts::PI;
        } else if angle <= -std::f64::consts::FRAC_PI_2 + 1e-9 {
            angle += std::f64::consts::PI;
        }
        for j in 0..count {
            let at = a + (b - a) * ((j as f64 + 0.5) / count as f64);
            out.push(PlanItem::Annotation {
                id: *id,
                item: Box::new(Annotation {
                    shape: Shape::Along { at, angle },
                    text: net.text(),
                    look,
                    mm,
                }),
            });
        }
    }
    out
}

/// The network a sketch is drawn as.
pub fn net_of(model: &Model, id: ElementId) -> Option<&creabim_core::utility::UtilityLine> {
    match &model.get(id)?.kind {
        ElementKind::Sketch(sk) => sk.net.as_ref(),
        _ => None,
    }
}

fn line(id: ElementId, a: DVec2, b: DVec2) -> PlanItem {
    PlanItem::Line {
        id,
        pen: Pen::Symbol,
        a,
        b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::utility::{NetKind, NetState, UtilityLine};
    use creabim_core::{Element, Sketch, Transaction, Workplane};

    #[test]
    fn a_sewer_has_its_code_and_arrows() {
        let mut m = Model::new();
        let mut sk = Sketch::polyline(
            None,
            Workplane::ground(0.0),
            vec![DVec2::ZERO, DVec2::new(20.0, 0.0)],
            false,
        );
        sk.net = Some(UtilityLine {
            kind: NetKind::FoulSewer,
            state: NetState::Proposed,
            note: "DN 150".into(),
        });
        let e = Element::new("K", ElementKind::Sketch(sk));
        let id = e.id;
        Transaction::new("t").insert(e).apply(&mut m).unwrap();
        // 20 m at 1:200 is 100 mm on paper
        let mm = 0.2;
        let items = vec![line(id, DVec2::ZERO, DVec2::new(20.0, 0.0))];
        let marks = net_marks(&m, &items, mm);
        let texts: Vec<&str> = marks
            .iter()
            .filter_map(|i| match i {
                PlanItem::Annotation { item, .. } => Some(item.text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, ["K DN 150"]);
        // arrows at 20, 60 mm along: two strokes each
        let strokes = marks
            .iter()
            .filter(|i| matches!(i, PlanItem::Line { .. }))
            .count();
        assert_eq!(strokes, 4);
    }
}
