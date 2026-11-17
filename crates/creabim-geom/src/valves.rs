//! Valves and accessories on pipes: in plan a single-line symbol
//! (STN EN ISO 6412-2: a valve is two triangles point to point), in 3D a
//! body on the pipe. Ported from PilonDesigner's `valveSymbol`.

use crate::mesh::TriMesh;
use crate::plan::{Pen, PlanItem};
use creabim_core::mep::{Pipe, Valve};
use creabim_core::{Element, Model};
use glam::{DVec2, DVec3};

/// A symbol's strokes, solid parts, circles and letters.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Symbol {
    pub segs: Vec<[DVec2; 2]>,
    pub fills: Vec<Vec<DVec2>>,
    pub circles: Vec<(DVec2, f64)>,
    pub text: Option<&'static str>,
}

/// The symbol of the kind `kind` at `c` on a pipe running along `u` (a
/// unit vector), half as long as `h`, metres.
pub fn symbol(kind: &str, c: DVec2, u: DVec2, h: f64) -> Symbol {
    let n = u.perp();
    let w = h * 0.7;
    let p = |s: f64, t: f64| c + u * s + n * t;
    let mut out = Symbol::default();
    let bow = |o: &mut Symbol| {
        o.segs.extend([
            [p(-h, w), p(-h, -w)],
            [p(-h, -w), p(h, w)],
            [p(h, w), p(h, -w)],
            [p(h, -w), p(-h, w)],
        ])
    };
    let boxed = |o: &mut Symbol, k: f64| {
        let (a, b) = (h * k, w * k);
        o.segs.extend([
            [p(-a, b), p(a, b)],
            [p(a, b), p(a, -b)],
            [p(a, -b), p(-a, -b)],
            [p(-a, -b), p(-a, b)],
        ])
    };
    let cross =
        |o: &mut Symbol, k: f64| o.segs.extend([[p(-k, -k), p(k, k)], [p(-k, k), p(k, -k)]]);
    let diamond = |o: &mut Symbol| {
        o.segs.extend([
            [p(-h, 0.0), p(0.0, w)],
            [p(0.0, w), p(h, 0.0)],
            [p(h, 0.0), p(0.0, -w)],
            [p(0.0, -w), p(-h, 0.0)],
        ])
    };
    match kind {
        "uzaver" => bow(&mut out),
        // solid on the side the water comes from
        "spatna" => {
            bow(&mut out);
            out.fills.push(vec![p(-h, w), p(-h, -w), p(0.0, 0.0)]);
        }
        "filter" => {
            diamond(&mut out);
            out.segs.push([p(-h * 0.5, -w * 0.5), p(h * 0.5, w * 0.5)]);
        }
        "redukcny" => {
            bow(&mut out);
            out.segs.extend([
                [p(0.0, 0.0), p(0.0, w * 1.6)],
                [p(-h * 0.5, w * 1.6), p(h * 0.5, w * 1.6)],
                [p(-h * 0.5, w * 1.6), p(0.0, w * 2.3)],
                [p(h * 0.5, w * 1.6), p(0.0, w * 2.3)],
            ]);
        }
        "vyvazovaci" | "termostaticky" | "spiatocka" | "rtd" => {
            bow(&mut out);
            out.segs.extend([
                [p(-h * 1.1, -w * 1.3), p(h * 1.1, w * 1.3)],
                [p(h * 1.1, w * 1.3), p(h * 0.55, w * 1.15)],
                [p(h * 1.1, w * 1.3), p(h * 0.85, w * 0.75)],
            ]);
        }
        "vodomer" | "plynomer" => {
            out.circles.push((c, w * 1.2));
            out.segs
                .extend([[p(-h, 0.0), p(-w * 1.2, 0.0)], [p(w * 1.2, 0.0), p(h, 0.0)]]);
            out.text = Some(if kind == "vodomer" { "V" } else { "P" });
        }
        "rohovy" => {
            bow(&mut out);
            out.segs.extend([
                [p(0.0, 0.0), p(0.0, w * 1.8)],
                [p(0.0, w * 1.8), p(h * 0.6, w * 1.8)],
            ]);
        }
        "hlavica" => {
            out.circles.push((c, w * 1.3));
            cross(&mut out, w * 0.9);
        }
        "cistiaci" => {
            boxed(&mut out, 1.0);
            out.text = Some("Č");
        }
        "kohut" => {
            bow(&mut out);
            out.circles.push((c, w * 0.35));
        }
        "hup" => {
            bow(&mut out);
            boxed(&mut out, 1.35);
            out.text = Some("HUP");
        }
        "regulator" => {
            bow(&mut out);
            out.segs.push([p(0.0, 0.0), p(0.0, w * 1.5)]);
            out.circles.push((p(0.0, w * 2.0), w * 0.5));
        }
        "tap" => {
            bow(&mut out);
            out.text = Some("T");
        }
        "krabica" => {
            bow(&mut out);
            boxed(&mut out, 1.35);
        }
        "spotrebic" => {
            boxed(&mut out, 1.35);
            out.text = Some("G");
        }
        "vpust" => {
            out.circles.push((c, w * 1.1));
            cross(&mut out, w * 0.75);
        }
        "hydrant" => {
            boxed(&mut out, 1.35);
            out.fills.push(vec![
                p(-w * 0.6, -w * 0.6),
                p(w * 0.6, -w * 0.6),
                p(w * 0.6, w * 0.6),
            ]);
            out.text = Some("H");
        }
        "vyvod" => {
            out.circles.push((c, w * 1.1));
            out.fills.push(vec![
                p(-w * 0.5, -w * 0.5),
                p(w * 0.5, -w * 0.5),
                p(w * 0.5, w * 0.5),
                p(-w * 0.5, w * 0.5),
            ]);
        }
        // air: a circle with a solid arrow out (supply) or in (exhaust)
        "vyustP" => {
            out.circles.push((c, w * 1.3));
            out.fills.push(vec![
                p(-w * 0.2, -w * 0.6),
                p(-w * 0.2, w * 0.6),
                p(w * 0.9, 0.0),
            ]);
        }
        "vyustO" => {
            out.circles.push((c, w * 1.3));
            out.fills.push(vec![
                p(w * 0.2, -w * 0.6),
                p(w * 0.2, w * 0.6),
                p(-w * 0.9, 0.0),
            ]);
        }
        "klapkaReg" => {
            out.segs.extend([
                [p(0.0, -w * 1.3), p(0.0, w * 1.3)],
                [p(-h * 0.7, -w * 0.9), p(h * 0.7, w * 0.9)],
            ]);
            out.circles.push((c, w * 0.25));
        }
        "klapkaPoz" => {
            boxed(&mut out, 1.35);
            out.segs
                .push([p(-h * 1.35, -w * 1.35), p(h * 1.35, w * 1.35)]);
            out.text = Some("PK");
        }
        "tlmic" => {
            boxed(&mut out, 1.6);
            out.segs.extend([
                [p(-h * 0.8, -w * 1.6), p(-h * 0.8, w * 1.6)],
                [p(h * 0.8, -w * 1.6), p(h * 0.8, w * 1.6)],
            ]);
            out.text = Some("TH");
        }
        "filterVzt" => {
            diamond(&mut out);
            out.segs.push([p(0.0, -w), p(0.0, w)]);
        }
        // a kind not known here: a plain valve
        _ => bow(&mut out),
    }
    out
}

/// Half the length of a valve's symbol on `pipe`, metres: 2.5 mm on a
/// 1:50 drawing, more on a thick pipe.
pub fn symbol_half(pipe: &Pipe) -> f64 {
    let (w, _) = pipe.section_m();
    (w * 1.2).max(0.125)
}

/// The way `pipe` runs in plan; east for a riser.
fn along(pipe: &Pipe) -> DVec2 {
    (pipe.end - pipe.start)
        .truncate()
        .try_normalize()
        .unwrap_or(DVec2::X)
}

/// How a valve is drawn in the plan of its pipe's level.
pub fn valve_plan(model: &Model, e: &Element, v: &Valve) -> Vec<PlanItem> {
    let Some(pipe) = v.host(model) else {
        return vec![];
    };
    let c = v.at(pipe).truncate();
    let h = symbol_half(pipe);
    let s = symbol(&v.kind, c, along(pipe), h);
    let mut items = vec![];
    // the pipe's own lines don't run through the symbol
    let (w, _) = pipe.section_m();
    if w < h * 1.4 {
        let r = h * 1.35;
        items.push(PlanItem::Shade {
            id: e.id,
            points: (0..12)
                .map(|i| c + DVec2::from_angle(i as f64 / 12.0 * std::f64::consts::TAU) * r)
                .collect(),
            rgb: [1.0, 1.0, 1.0],
        });
    }
    for [a, b] in s.segs {
        items.push(PlanItem::Line {
            id: e.id,
            pen: Pen::Symbol,
            a,
            b,
        });
    }
    for (center, radius) in s.circles {
        items.push(PlanItem::Arc {
            id: e.id,
            pen: Pen::Symbol,
            center,
            radius,
            start: 0.0,
            end: std::f64::consts::TAU,
        });
    }
    for points in s.fills {
        items.push(PlanItem::Shade {
            id: e.id,
            points,
            rgb: [0.0, 0.0, 0.0],
        });
    }
    if let Some(text) = s.text {
        items.push(PlanItem::Text {
            id: e.id,
            at: c,
            text: text.to_string(),
            size: if text.len() > 2 { 1.2 } else { 1.8 },
            align: crate::paper::Anchor::Middle,
            mm: 1.0,
            rgb: None,
            bold: true,
        });
    }
    items
}

/// A valve's body in 3D: a thicker piece of its pipe with a ball in the
/// middle, darker than the pipe.
pub fn valve_mesh(model: &Model, v: &Valve) -> Option<(TriMesh, [f32; 3])> {
    let pipe = v.host(model)?;
    let z = DVec3::Z * model.level(pipe.level).map_or(0.0, |l| l.elevation);
    let c = v.at(pipe) + z;
    let d = (pipe.end - pipe.start).try_normalize().unwrap_or(DVec3::Z);
    let (w, _) = pipe.section_m();
    let r = w / 2.0 + pipe.insulation_mm / 1000.0;
    let mut m = TriMesh::default();
    let terminal = v.kind_def().is_some_and(|k| k.terminal);
    if terminal {
        crate::mep::ball(&mut m, c, r * 1.8);
    } else {
        let l = (w * 1.6).max(0.05);
        crate::mep::taper(
            &mut m,
            &[
                (c - d * l, r),
                (c - d * l, r * 1.5),
                (c + d * l, r * 1.5),
                (c + d * l, r),
            ],
        );
        crate::mep::ball(&mut m, c, r * 1.9);
    }
    let col = crate::mep::color(pipe).map(|x| x * 0.6);
    Some((m, col))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn symbols_are_pilon_designers() {
        let u = symbol("uzaver", DVec2::ZERO, DVec2::X, 0.1);
        assert_eq!(u.segs.len(), 4);
        assert!(u.fills.is_empty() && u.circles.is_empty() && u.text.is_none());
        assert_eq!(symbol("spatna", DVec2::ZERO, DVec2::X, 0.1).fills.len(), 1);
        assert_eq!(
            symbol("vodomer", DVec2::ZERO, DVec2::Y, 0.1).circles.len(),
            1
        );
        assert_eq!(symbol("hup", DVec2::ZERO, DVec2::X, 0.1).text, Some("HUP"));
        // every kind has a symbol of its own size
        for k in creabim_core::mep::VALVE_KINDS {
            let s = symbol(k.code, DVec2::new(3.0, 2.0), DVec2::Y, 0.1);
            assert!(
                !s.segs.is_empty() || !s.circles.is_empty(),
                "{} has no symbol",
                k.code
            );
            for [a, b] in &s.segs {
                assert!(a.distance(DVec2::new(3.0, 2.0)) < 0.3 && b.distance(*a) > 1e-9);
            }
        }
    }
}
