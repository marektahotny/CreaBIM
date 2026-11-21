//! Gutters and downpipes drawn: a gutter as its profile swept along its
//! falling axis, a downpipe as an upright tube from under the gutter to
//! its foot; in plan, a downpipe cut by the plan as a heavy ring or
//! square.

use crate::plan::{Pen, PlanItem};
use crate::TriMesh;
use creabim_core::flashing::{
    downpipe_place, outlets, DownpipeProfile, Flashing, FlashingItem, Gutter, GutterProfile,
};
use creabim_core::{Element, ElementId, Model};
use glam::{DVec2, DVec3};
use std::f64::consts::PI;

/// The colour of sheet metal: a light zinc grey.
pub const COLOUR: [f32; 3] = [0.62, 0.64, 0.66];

/// The thinnest a sheet is modelled, m (a 0,6 mm sheet would not show).
const MIN_SHEET: f64 = 0.002;

/// A gutter's profile in the sweep's axes (x to the left of the path,
/// y up), its insertion point the middle of its top: open at the top.
pub fn gutter_profile(g: &Gutter, thickness: f64) -> Vec<DVec2> {
    let t = thickness.max(MIN_SHEET);
    let w = g.width();
    match g.profile {
        GutterProfile::HalfRound => {
            let r = w / 2.0;
            let n = 16;
            let arc = |rad: f64| -> Vec<DVec2> {
                (0..=n)
                    .map(|i| DVec2::from_angle(PI + PI * i as f64 / n as f64) * rad)
                    .collect()
            };
            let mut v = arc(r);
            let mut inner = arc(r - t);
            inner.reverse();
            v.extend(inner);
            v
        }
        GutterProfile::Square => {
            let (x, d) = (w / 2.0, g.depth());
            vec![
                DVec2::new(-x, 0.0),
                DVec2::new(-x, -d),
                DVec2::new(x, -d),
                DVec2::new(x, 0.0),
                DVec2::new(x - t, 0.0),
                DVec2::new(x - t, -d + t),
                DVec2::new(-x + t, -d + t),
                DVec2::new(-x + t, 0.0),
            ]
        }
    }
}

/// The solid of a gutter or a downpipe.
pub fn mesh(model: &Model, id: ElementId, f: &Flashing) -> TriMesh {
    let t = f.thickness_mm / 1000.0;
    match &f.item {
        FlashingItem::Gutter(g) => {
            let path = g.axis_3d(model, &outlets(model, id));
            crate::parts::sweep_mesh(&gutter_profile(g, t), &path, false)
        }
        FlashingItem::Downpipe(d) => {
            let Some((at, top, foot)) = downpipe_place(model, d) else {
                return TriMesh::default();
            };
            if top - foot < 1e-3 {
                return TriMesh::default();
            }
            let outline = downpipe_outline(d.profile, d.size_mm / 1000.0, at);
            crate::scene::slab_mesh(&outline, top, top - foot)
        }
    }
}

/// A downpipe's outline in plan, round as a 24-gon.
pub fn downpipe_outline(profile: DownpipeProfile, size: f64, at: DVec2) -> Vec<DVec2> {
    let r = size / 2.0;
    match profile {
        DownpipeProfile::Round => (0..24)
            .map(|i| at + DVec2::from_angle(2.0 * PI * i as f64 / 24.0) * r)
            .collect(),
        DownpipeProfile::Square => vec![
            at + DVec2::new(-r, -r),
            at + DVec2::new(r, -r),
            at + DVec2::new(r, r),
            at + DVec2::new(-r, r),
        ],
    }
}

/// What a plan cut at absolute height `z` draws of it: a downpipe the
/// cut passes through, heavy; one wholly under the cut, seen from above.
/// A gutter is drawn where the plan looks down on it (a roof plan).
pub fn plan_items(model: &Model, e: &Element, f: &Flashing, z: f64) -> Vec<PlanItem> {
    let mut out = vec![];
    match &f.item {
        FlashingItem::Downpipe(d) => {
            let Some((at, top, foot)) = downpipe_place(model, d) else {
                return out;
            };
            let outline = downpipe_outline(d.profile, d.size_mm / 1000.0, at);
            let pen = if foot < z && z < top {
                Pen::Cut
            } else if top <= z {
                Pen::Projection
            } else {
                return out;
            };
            ring(&mut out, e.id, pen, &outline);
        }
        FlashingItem::Gutter(g) => {
            if g.top(model) > z {
                return out;
            }
            let (back, front) = g.edges_plan(model);
            for line in [&back, &front] {
                for w in line.windows(2) {
                    out.push(PlanItem::Line {
                        id: e.id,
                        pen: Pen::Projection,
                        a: w[0],
                        b: w[1],
                    });
                }
            }
            // the ends closed
            for (a, b) in [(back.first(), front.first()), (back.last(), front.last())] {
                if let (Some(a), Some(b)) = (a, b) {
                    out.push(PlanItem::Line {
                        id: e.id,
                        pen: Pen::Projection,
                        a: *a,
                        b: *b,
                    });
                }
            }
        }
    }
    out
}

fn ring(out: &mut Vec<PlanItem>, id: ElementId, pen: Pen, pts: &[DVec2]) {
    for i in 0..pts.len() {
        out.push(PlanItem::Line {
            id,
            pen,
            a: pts[i],
            b: pts[(i + 1) % pts.len()],
        });
    }
}

/// Every gutter and downpipe as a roof plan draws them, seen from above,
/// with their marks.
pub fn roof_plan(model: &Model, scale: f64) -> Vec<PlanItem> {
    let mut out = vec![];
    for e in model.iter() {
        if let creabim_core::ElementKind::Flashing(f) = &e.kind {
            out.extend(plan_items(model, e, f, f64::INFINITY));
        }
    }
    out.extend(roof_plan_marks(model, scale));
    out
}

/// The marks of the sheet-metal items in a plan looking down on them
/// (the roof plan): each gutter's mark by its middle, each downpipe's by
/// it, and the way each gutter run falls as an arrow along it.
pub fn roof_plan_marks(model: &Model, scale: f64) -> Vec<PlanItem> {
    let mut out = vec![];
    let k = scale / 1000.0;
    for e in model.iter() {
        let creabim_core::ElementKind::Flashing(f) = &e.kind else {
            continue;
        };
        let Some(mark) = creabim_core::flashing::mark_of(model, e.id) else {
            continue;
        };
        let (anchor, out_dir) = match &f.item {
            FlashingItem::Gutter(g) => {
                let axis = g.axis_plan(model);
                let s = g.length() / 2.0;
                let p = g.axis_at(&axis, s);
                let n = g
                    .outward(model)
                    .get(seg_of(g, s))
                    .copied()
                    .unwrap_or(DVec2::Y);
                // fall arrows: from each high point a short arrow towards
                // the outlet
                let outs = outlets(model, e.id);
                for h in g.high_points(&outs) {
                    // towards the outlet on each side of it
                    let left = outs.iter().rev().find(|o| **o < h - 1e-6).copied();
                    let right = outs.iter().find(|o| **o > h + 1e-6).copied();
                    for to in [left, right].into_iter().flatten() {
                        let dir = (to - h).signum();
                        let len = (12.0 * k).min((to - h).abs() * 0.8);
                        if len < 1.0 * k {
                            continue;
                        }
                        let (a, b) = (g.axis_at(&axis, h), g.axis_at(&axis, h + dir * len));
                        arrow(&mut out, e.id, a, b, k);
                    }
                }
                (p, n)
            }
            FlashingItem::Downpipe(d) => {
                let Some((at, _, _)) = downpipe_place(model, d) else {
                    continue;
                };
                let n = creabim_core::flashing::gutter_of(model, d.gutter)
                    .map(|g| {
                        let s = d.at.clamp(0.0, g.length());
                        g.outward(model)
                            .get(seg_of(g, s))
                            .copied()
                            .unwrap_or(DVec2::Y)
                    })
                    .unwrap_or(DVec2::Y);
                (at, n)
            }
        };
        out.push(PlanItem::Tag {
            id: e.id,
            anchor,
            at: anchor + out_dir * (9.0 * k),
            elbow: None,
            text: mark,
            round: true,
            look: crate::annotation::TextLook {
                size: 2.5,
                ..Default::default()
            },
        });
    }
    out
}

fn seg_of(g: &Gutter, s: f64) -> usize {
    let st = g.stations();
    (0..st.len().saturating_sub(1))
        .find(|i| s <= st[i + 1] + 1e-9)
        .unwrap_or(0)
}

/// A thin arrow from `a` to `b`, its head 2 mm on paper (`k` m a mm).
fn arrow(out: &mut Vec<PlanItem>, id: ElementId, a: DVec2, b: DVec2, k: f64) {
    let d = (b - a).normalize_or_zero();
    if d == DVec2::ZERO {
        return;
    }
    out.push(PlanItem::Line {
        id,
        pen: Pen::Symbol,
        a,
        b,
    });
    let h = 2.0 * k;
    for side in [1.0, -1.0] {
        out.push(PlanItem::Line {
            id,
            pen: Pen::Symbol,
            a: b,
            b: b - d * h + d.perp() * side * h * 0.35,
        });
    }
}

/// The 3D top line of a gutter, for tests and snapping.
pub fn gutter_line(model: &Model, id: ElementId) -> Vec<DVec3> {
    creabim_core::flashing::gutter_of(model, id)
        .map(|g| g.axis_3d(model, &outlets(model, id)))
        .unwrap_or_default()
}
