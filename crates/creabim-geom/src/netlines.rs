//! Utility networks and the site plan's symbols in a plan (see
//! [`creabim_core::utility`]): a network's line drawn dashed with its
//! symbol set into it every 40 mm (6.152–6.732, 8.021), its code written
//! in, a cancelled one crossed between its symbols; a symbol of the site
//! plan (6.120–6.580, the entrances) drawn where its sketch is. One
//! colour for all of a network, its symbols too, given by its looks.

use crate::annotation::{Annotation, Shape, TextLook};
use crate::plan::{Pen, PlanItem};
use creabim_core::utility::{Prim, SiteMark, UtilityLine};
use creabim_core::{ElementId, ElementKind, Model};
use glam::DVec2;

/// What a network or a symbol draws: lines, filled shapes and letters.
#[derive(Clone, Debug, PartialEq)]
pub enum Stroke {
    Line(Vec<DVec2>),
    Fill(Vec<DVec2>),
    /// Letters `size` mm tall at `at`, reading along `angle`.
    Text {
        at: DVec2,
        angle: f64,
        text: String,
        size: f64,
    },
}

/// Symbols along a line this far apart, paper mm.
pub const STEP: f64 = 40.0;
/// A network's code written this tall, paper mm.
const CODE: f64 = 2.5;

/// `prims` (paper mm about their origin, +x along `u`) placed at `o`, at
/// `mm` drawing units per paper mm.
pub fn place(prims: &[Prim], o: DVec2, u: DVec2, mm: f64, out: &mut Vec<Stroke>) {
    let n = u.perp();
    let at = |p: [f64; 2]| o + (u * p[0] + n * p[1]) * mm;
    let arc = |c: [f64; 2], r: f64, a0: f64, a1: f64| -> Vec<DVec2> {
        let k = 16;
        (0..=k)
            .map(|i| {
                let t = (a0 + (a1 - a0) * i as f64 / k as f64).to_radians();
                at([c[0] + r * t.cos(), c[1] + r * t.sin()])
            })
            .collect()
    };
    let mut angle = u.to_angle();
    if angle > std::f64::consts::FRAC_PI_2 + 1e-9 {
        angle -= std::f64::consts::PI;
    } else if angle <= -std::f64::consts::FRAC_PI_2 + 1e-9 {
        angle += std::f64::consts::PI;
    }
    for p in prims {
        match p {
            Prim::Seg(a, b) => out.push(Stroke::Line(vec![at(*a), at(*b)])),
            Prim::Poly(pts) => out.push(Stroke::Line(pts.iter().map(|p| at(*p)).collect())),
            Prim::Arc { c, r, from, to } => out.push(Stroke::Line(arc(*c, *r, *from, *to))),
            Prim::Circle { c, r } => out.push(Stroke::Line(arc(*c, *r, 0.0, 360.0))),
            Prim::Disk { c, r } => {
                let mut ring = arc(*c, *r, 0.0, 360.0);
                ring.pop();
                out.push(Stroke::Fill(ring));
            }
            Prim::HalfDisk { c, r } => out.push(Stroke::Fill(arc(*c, *r, 0.0, 180.0))),
            Prim::Fill(pts) => out.push(Stroke::Fill(pts.iter().map(|p| at(*p)).collect())),
            Prim::Text { at: p, text } => out.push(Stroke::Text {
                at: at(*p),
                angle,
                text: text.clone(),
                size: 2.0,
            }),
        }
    }
}

/// A network's line from `a` to `b` (drawing units, `mm` to the paper
/// mm): dashes, its symbol every [`STEP`] mm, a cross between them when
/// cancelled, its code written in every third gap. With `sample` the
/// symbol sits once in the middle and nothing is written (a legend's).
pub fn run(net: &UtilityLine, a: DVec2, b: DVec2, mm: f64, sample: bool) -> Vec<Stroke> {
    let mut out = vec![];
    let len = a.distance(b);
    if len < 1e-9 {
        return out;
    }
    let u = (b - a) / len;
    let n = u.perp();
    let sym = net.kind.symbol();
    let s = sym.len * mm;
    let step = STEP * mm;
    // where symbols sit: their middles
    let mut centres = vec![];
    if sample || len < step {
        if len > s + 2.0 * mm {
            centres.push(len / 2.0);
        }
    } else {
        let mut t = step / 2.0;
        while t + s / 2.0 < len - mm {
            centres.push(t);
            t += step;
        }
    }
    // stretches left out of the dashes: the symbols and the codes
    let mut gaps: Vec<(f64, f64)> = vec![];
    for c in &centres {
        let x0 = c - s / 2.0;
        gaps.push((x0, x0 + s));
        place(&sym.prims, a + u * x0, u, mm, &mut out);
    }
    let text = net.text();
    let between: Vec<f64> = if centres.len() > 1 {
        centres.windows(2).map(|w| (w[0] + w[1]) / 2.0).collect()
    } else if let Some(c) = centres.first() {
        // one symbol: its code past it, if there is room
        let t = (c + len) / 2.0;
        if len - c > s + 12.0 * mm {
            vec![t]
        } else {
            vec![]
        }
    } else {
        vec![]
    };
    if !sample && !text.is_empty() {
        let w = (1.0 + 1.7 * text.chars().count() as f64) * mm;
        for t in between.iter().step_by(3) {
            gaps.push((t - w / 2.0, t + w / 2.0));
            let mut angle = u.to_angle();
            if angle > std::f64::consts::FRAC_PI_2 + 1e-9 {
                angle -= std::f64::consts::PI;
            } else if angle <= -std::f64::consts::FRAC_PI_2 + 1e-9 {
                angle += std::f64::consts::PI;
            }
            out.push(Stroke::Text {
                at: a + u * *t,
                angle,
                text: text.clone(),
                size: CODE,
            });
        }
    }
    // a cancelled network: crossed between the symbols, by turns
    if net.crossed() {
        let k = 1.6 * mm;
        let mut at: Vec<f64> = vec![];
        for c in &centres {
            for d in [-step / 4.0, step / 4.0] {
                let t = c + d;
                if t > k && t < len - k {
                    at.push(t);
                }
            }
        }
        if at.is_empty() {
            at = vec![len * 0.25, len * 0.75];
        }
        for t in at {
            let c = a + u * t;
            for d in [(u + n).normalize(), (u - n).normalize()] {
                out.push(Stroke::Line(vec![c - d * k, c + d * k]));
            }
        }
    }
    // the dashes between
    gaps.sort_by(|x, y| x.0.total_cmp(&y.0));
    let pattern: Vec<f64> = net.pattern().iter().map(|d| d * mm).collect();
    let period: f64 = pattern.iter().sum();
    let mut from = 0.0;
    let mut runs = vec![];
    for (g0, g1) in gaps.iter().chain(std::iter::once(&(len, len))) {
        if *g0 > from {
            runs.push((from, g0.min(len)));
        }
        from = from.max(*g1);
    }
    for (x0, x1) in runs {
        if period <= 0.0 {
            out.push(Stroke::Line(vec![a + u * x0, a + u * x1]));
            continue;
        }
        let (mut x, mut i) = (x0, 0);
        while x < x1 - 1e-9 {
            let d = pattern[i % pattern.len()];
            if i % 2 == 0 {
                out.push(Stroke::Line(vec![a + u * x, a + u * (x + d).min(x1)]));
            }
            x += d;
            i += 1;
        }
    }
    out
}

/// A site plan symbol at `at` facing `dir`.
pub fn mark(m: SiteMark, at: DVec2, dir: DVec2, mm: f64) -> Vec<Stroke> {
    // +y of the symbol is the way it faces: its x runs to the right of it
    let u = (-dir.perp()).normalize_or(DVec2::X);
    let mut out = vec![];
    place(&m.symbol(), at, u, mm, &mut out);
    out
}

/// `strokes` as items of element `id` in a plan: lines in the element's
/// own pen (its looks colour and weigh them), fills and letters in `rgb`.
pub fn plan_items(id: ElementId, strokes: Vec<Stroke>, rgb: [f32; 3], mm: f64) -> Vec<PlanItem> {
    let mut out = vec![];
    for s in strokes {
        match s {
            Stroke::Line(pts) => {
                for w in pts.windows(2) {
                    out.push(PlanItem::Line {
                        id,
                        pen: Pen::Projection,
                        a: w[0],
                        b: w[1],
                    });
                }
            }
            Stroke::Fill(points) => out.push(PlanItem::Shade { id, points, rgb }),
            Stroke::Text {
                at,
                angle,
                text,
                size,
            } => out.push(PlanItem::Annotation {
                id,
                item: Box::new(Annotation {
                    shape: Shape::Along { at, angle },
                    text,
                    look: TextLook {
                        size,
                        rgb: Some(rgb),
                        bold: true,
                        ..TextLook::default()
                    },
                    mm,
                }),
            }),
        }
    }
    out
}

/// `strokes` drawn on paper exactly, `weight` mm thick in `rgb`, for a
/// legend (whose items carry no element's looks).
pub fn paper_items(
    id: ElementId,
    strokes: Vec<Stroke>,
    rgb: [f32; 3],
    weight: f64,
) -> Vec<PlanItem> {
    let half = weight / 2.0;
    let mut out = vec![];
    for s in strokes {
        match s {
            Stroke::Line(pts) => {
                for w in pts.windows(2) {
                    let (p, q) = (w[0], w[1]);
                    let d = (q - p).normalize_or(DVec2::X);
                    let (p, q) = (p - d * half * 0.5, q + d * half * 0.5);
                    let n = d.perp() * half;
                    out.push(PlanItem::Shade {
                        id,
                        points: vec![p - n, q - n, q + n, p + n],
                        rgb,
                    });
                }
            }
            Stroke::Fill(points) => out.push(PlanItem::Shade { id, points, rgb }),
            Stroke::Text { at, text, .. } => out.push(PlanItem::Text {
                id,
                at,
                text,
                size: 2.0,
                align: crate::paper::Anchor::Middle,
                mm: 1.0,
                rgb: Some(rgb),
                bold: true,
            }),
        }
    }
    out
}

/// The network a sketch is drawn as.
pub fn net_of(model: &Model, id: ElementId) -> Option<&UtilityLine> {
    match &model.get(id)?.kind {
        ElementKind::Sketch(sk) => sk.net.as_ref(),
        _ => None,
    }
}

/// The site plan symbol a sketch is.
pub fn mark_of(model: &Model, id: ElementId) -> Option<SiteMark> {
    match &model.get(id)?.kind {
        ElementKind::Sketch(sk) => sk.mark,
        _ => None,
    }
}

/// Is element `id` a network or a site plan symbol: drawn by the site
/// plan only?
pub fn is_site_only(model: &Model, id: ElementId) -> bool {
    net_of(model, id).is_some() || mark_of(model, id).is_some()
}

/// The networks and symbols among `items` (a plan at `mm` drawing units
/// per paper mm) drawn as they are on a site plan: each network's line
/// dashed with its symbols, each symbol in place of its sketch.
pub fn net_lines(model: &Model, items: &mut Vec<PlanItem>, mm: f64) {
    let mut seen: Vec<ElementId> = vec![];
    let mut out = vec![];
    items.retain(|it| {
        let id = it.id();
        if let Some(net) = net_of(model, id) {
            if let PlanItem::Line { a, b, .. } = it {
                out.extend(plan_items(
                    id,
                    run(net, *a, *b, mm, false),
                    net.kind.rgb(),
                    mm,
                ));
                return false;
            }
            return !matches!(it, PlanItem::Tint { .. });
        }
        if let Some(m) = mark_of(model, id) {
            if !seen.contains(&id) {
                seen.push(id);
                if let Some(ElementKind::Sketch(sk)) = model.get(id).map(|e| &e.kind) {
                    let p = |q: DVec2| sk.plane.to_model(q).truncate();
                    if let [a, b, ..] = sk.points[..] {
                        let (at, to) = (p(a), p(b));
                        out.extend(plan_items(id, mark(m, at, to - at, mm), m.rgb(), mm));
                    } else if let [a] = sk.points[..] {
                        out.extend(plan_items(id, mark(m, p(a), DVec2::Y, mm), m.rgb(), mm));
                    }
                }
            }
            return false;
        }
        true
    });
    items.extend(out);
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::utility::{NetKind, NetState};
    use creabim_core::{Element, Sketch, Transaction, Workplane};

    fn net(m: &mut Model, kind: NetKind, state: NetState) -> ElementId {
        let mut sk = Sketch::polyline(
            None,
            Workplane::ground(0.0),
            vec![DVec2::ZERO, DVec2::new(20.0, 0.0)],
            false,
        );
        sk.net = Some(UtilityLine {
            kind,
            state,
            note: "DN 150".into(),
        });
        let e = Element::new("K", ElementKind::Sketch(sk));
        let id = e.id;
        Transaction::new("t").insert(e).apply(m).unwrap();
        id
    }

    fn line(id: ElementId, a: DVec2, b: DVec2) -> PlanItem {
        PlanItem::Line {
            id,
            pen: Pen::Projection,
            a,
            b,
        }
    }

    #[test]
    fn a_sewer_is_dashed_with_its_symbols_and_its_code() {
        let mut m = Model::new();
        let id = net(&mut m, NetKind::FoulSewer, NetState::Proposed);
        // 20 m at 1:200 is 100 mm on paper: symbols at 20, 60 mm along
        let mm = 0.2;
        let mut items = vec![line(id, DVec2::ZERO, DVec2::new(20.0, 0.0))];
        net_lines(&m, &mut items, mm);
        let texts: Vec<&str> = items
            .iter()
            .filter_map(|i| match i {
                PlanItem::Annotation { item, .. } => Some(item.text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(texts, ["Ks DN 150"]);
        // the symbol's bracket reaches 1.2 mm off the line
        let off = items
            .iter()
            .filter_map(|i| match i {
                PlanItem::Line { a, b, .. } => Some(a.y.abs().max(b.y.abs())),
                _ => None,
            })
            .fold(0.0, f64::max);
        assert!((off - 1.2 * mm).abs() < 1e-6, "{off}");
        // no stroke runs across where the code is written
        let at = items
            .iter()
            .find_map(|i| match i {
                PlanItem::Annotation { item, .. } => match item.shape {
                    Shape::Along { at, .. } => Some(at.x),
                    _ => None,
                },
                _ => None,
            })
            .unwrap();
        assert!(!items.iter().any(|i| matches!(i,
            PlanItem::Line { a, b, .. } if a.y == 0.0 && b.y == 0.0 && a.x.min(b.x) < at && a.x.max(b.x) > at)));
    }

    #[test]
    fn a_cancelled_network_is_crossed_and_a_mark_drawn_in_place() {
        let mut m = Model::new();
        let id = net(&mut m, NetKind::GasLow, NetState::Cancelled);
        let mut items = vec![line(id, DVec2::ZERO, DVec2::new(20.0, 0.0))];
        net_lines(&m, &mut items, 0.2);
        let crosses = items
            .iter()
            .filter(|i| matches!(i, PlanItem::Line { a, b, .. } if (a.x - b.x).abs() > 1e-6 && (a.y - b.y).abs() > 1e-6))
            .count();
        assert!(crosses >= 4, "{crosses}");
        // a manhole: its circle where its sketch is
        let mut sk = Sketch::polyline(
            None,
            Workplane::ground(0.0),
            vec![DVec2::new(5.0, 5.0), DVec2::new(5.0, 6.0)],
            false,
        );
        sk.mark = Some(SiteMark::SewerManhole);
        let e = Element::new("M", ElementKind::Sketch(sk));
        let mid = e.id;
        Transaction::new("m").insert(e).apply(&mut m).unwrap();
        let mut items = vec![line(mid, DVec2::new(5.0, 5.0), DVec2::new(5.0, 6.0))];
        net_lines(&m, &mut items, 0.2);
        let far = items
            .iter()
            .filter_map(|i| match i {
                PlanItem::Line { a, .. } => Some(a.distance(DVec2::new(5.0, 5.0))),
                _ => None,
            })
            .fold(0.0, f64::max);
        assert!((far - 0.2).abs() < 1e-6, "radius 1 mm at 1:200: {far}");
    }
}
