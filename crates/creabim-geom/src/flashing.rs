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
            let path = creabim_core::flashing::downpipe_path(model, d);
            if path.len() < 2 || path[0].z - path[path.len() - 1].z < 1e-3 {
                return TriMesh::default();
            }
            // swept on its side: every leg of it then runs across the
            // sweep's plan, the upright ones too
            let across = (path[path.len() - 1] - path[0])
                .truncate()
                .normalize_or(DVec2::X)
                .perp();
            let profile = downpipe_outline(d.profile, d.size_mm / 1000.0, DVec2::ZERO);
            crate::parts::sweep_mesh_on(&profile, &path, false, across.extend(0.0))
        }
        FlashingItem::Capping(c) => {
            let Some((path, sheet, _, _)) = capping_sections(model, c, t) else {
                return TriMesh::default();
            };
            crate::parts::sweep_mesh(&sheet, &path, false)
        }
        FlashingItem::Chimney(c) => chimney_mesh(model, c, t),
        FlashingItem::Outlet(o) => {
            let z = creabim_core::roofmetal::roof_top_at(model, o.at).unwrap_or(0.0);
            // its bowl's rim, standing a little proud of the roof
            let r = (o.dn / 1000.0 / 2.0 + 0.05).max(0.05);
            let ring: Vec<DVec2> = (0..24)
                .map(|i| DVec2::from_angle(2.0 * PI * i as f64 / 24.0) * r)
                .collect();
            let path = [o.at.extend(z + 0.03), o.at.extend(z - 0.02)];
            crate::parts::sweep_mesh_on(&ring, &path, false, DVec3::X)
        }
    }
}

/// A capping's sections, in the sweep's axes along its wall (x to the
/// left, y up from the wall's top): the path, the sheet, the XPS wedge
/// and the OSB board.
#[allow(clippy::type_complexity)]
pub fn capping_sections(
    model: &Model,
    c: &creabim_core::roofmetal::Capping,
    t: f64,
) -> Option<([DVec3; 2], Vec<DVec2>, Vec<DVec2>, Vec<DVec2>)> {
    let p = creabim_core::roofmetal::capping_place(model, c)?;
    if p.length() < 1e-6 {
        return None;
    }
    let t = t.max(MIN_SHEET);
    let left = (p.b - p.a).normalize().perp();
    // the inner edge's x: the roof's side of the wall
    let xi = p.width / 2.0 * p.inward.dot(left).signum();
    let xo = -xi;
    let (ti, to) = (p.xps_in + c.osb, p.xps_out + c.osb);
    let s = xi.signum();
    // the sheet: the inner drip, up the inner leg, over the top falling
    // inwards, down the outer leg to its drip; its thickness outside
    let line = [
        DVec2::new(xi + s * (t + c.drip), ti - c.leg - c.drip * 0.5),
        DVec2::new(xi + s * t, ti - c.leg),
        DVec2::new(xi + s * t, ti + t),
        DVec2::new(xo - s * t, to + t),
        DVec2::new(xo - s * t, to - c.leg),
        DVec2::new(xo - s * (t + c.drip), to - c.leg - c.drip * 0.5),
    ];
    let inner = [
        DVec2::new(xo - s * c.drip, to - c.leg - c.drip * 0.5 + t),
        DVec2::new(xo, to - c.leg + t),
        DVec2::new(xo, to),
        DVec2::new(xi, ti),
        DVec2::new(xi, ti - c.leg + t),
        DVec2::new(xi + s * c.drip, ti - c.leg - c.drip * 0.5 + t),
    ];
    let sheet: Vec<DVec2> = line.iter().chain(inner.iter()).copied().collect();
    let xps = vec![
        DVec2::new(xi, 0.0),
        DVec2::new(xo, 0.0),
        DVec2::new(xo, p.xps_out),
        DVec2::new(xi, p.xps_in),
    ];
    let osb = vec![
        DVec2::new(xi, p.xps_in),
        DVec2::new(xo, p.xps_out),
        DVec2::new(xo, to),
        DVec2::new(xi, ti),
    ];
    Some(([p.a.extend(p.z), p.b.extend(p.z)], sheet, xps, osb))
}

/// What a capping is laid on, as solids of their own: the XPS in its
/// fall and the OSB board, each with the model's material of that name
/// when there is one.
pub fn capping_substrate(
    model: &Model,
    c: &creabim_core::roofmetal::Capping,
) -> Vec<(TriMesh, [f32; 3], Option<ElementId>)> {
    let Some((path, _, xps, osb)) = capping_sections(model, c, 0.0) else {
        return vec![];
    };
    let find = |key: &str| {
        model.iter().find_map(|e| match &e.kind {
            creabim_core::ElementKind::Material(_) if e.name.to_uppercase().contains(key) => {
                Some(e.id)
            }
            _ => None,
        })
    };
    let mut out = vec![];
    if c.xps > 1e-6 || c.fall > 1e-6 {
        out.push((
            crate::parts::sweep_mesh(&xps, &path, false),
            [0.55, 0.78, 0.92],
            find("XPS"),
        ));
    }
    out.push((
        crate::parts::sweep_mesh(&osb, &path, false),
        [0.82, 0.68, 0.45],
        find("OSB"),
    ));
    out
}

/// The flashing round a chimney: up its faces and out over the roof,
/// following the roof's slope round it.
fn chimney_mesh(model: &Model, c: &creabim_core::roofmetal::ChimneyFlashing, t: f64) -> TriMesh {
    let Some(mut o) = creabim_core::roofmetal::chimney_outline(model, c.part) else {
        return TriMesh::default();
    };
    // counter-clockwise: the chimney to the left of the path
    if crate::parts::signed_area(&o) < 0.0 {
        o.reverse();
    }
    let t = t.max(MIN_SHEET);
    let path: Vec<DVec3> = o
        .iter()
        .map(|p| p.extend(creabim_core::roofmetal::roof_top_at(model, *p).unwrap_or(0.0)))
        .collect();
    // x = 0 the chimney's face, out to the right (negative x)
    let profile = [
        DVec2::new(0.0, 0.0),
        DVec2::new(0.0, c.upstand),
        DVec2::new(-t, c.upstand),
        DVec2::new(-t, t),
        DVec2::new(-c.apron, t),
        DVec2::new(-c.apron, 0.0),
    ];
    crate::parts::sweep_mesh(&profile, &path, true)
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
            let path = creabim_core::flashing::downpipe_path(model, d);
            let (Some(top), Some(foot)) = (path.first(), path.last()) else {
                return out;
            };
            let size = d.size_mm / 1000.0;
            if top.z <= z {
                // seen from above: the outlet, and where it runs down the
                // facade under the eaves
                ring(
                    &mut out,
                    e.id,
                    Pen::Projection,
                    &downpipe_outline(d.profile, size, top.truncate()),
                );
                // the offset to the facade and the pipe down it are under
                // the roof: dashed, its two sides from ring to ring
                let (a, b) = (top.truncate(), foot.truncate());
                if b.distance(a) > 1e-3 {
                    // one outline, so its dashes run on round it
                    out.push(PlanItem::Polygon {
                        id: e.id,
                        pen: Pen::Hidden,
                        points: downpipe_outline(d.profile, size, b),
                    });
                    let n = (b - a).normalize().perp() * size / 2.0;
                    for side in [n, -n] {
                        out.push(PlanItem::Line {
                            id: e.id,
                            pen: Pen::Hidden,
                            a: a + side,
                            b: b + side,
                        });
                    }
                }
            } else if foot.z < z {
                // cut where its axis passes the plan's height
                let at = path
                    .windows(2)
                    .find(|w| w[1].z <= z && z <= w[0].z)
                    .map(|w| {
                        let f = (w[0].z - z) / (w[0].z - w[1].z).max(1e-9);
                        w[0].lerp(w[1], f).truncate()
                    })
                    .unwrap_or(foot.truncate());
                ring(
                    &mut out,
                    e.id,
                    Pen::Cut,
                    &downpipe_outline(d.profile, size, at),
                );
            }
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
        FlashingItem::Capping(c) => {
            let Some(p) = creabim_core::roofmetal::capping_place(model, c) else {
                return out;
            };
            if p.z > z {
                return out;
            }
            // the sheet's outer edges, past the faces by its drips, and
            // the faces under it
            let half = p.width / 2.0;
            for (k, pen) in [
                (half + c.drip, Pen::Projection),
                (half, Pen::Projection),
                (-half, Pen::Projection),
                (-half - c.drip, Pen::Projection),
            ] {
                let n = p.inward * k;
                out.push(PlanItem::Line {
                    id: e.id,
                    pen,
                    a: p.a + n,
                    b: p.b + n,
                });
            }
            for end in [p.a, p.b] {
                out.push(PlanItem::Line {
                    id: e.id,
                    pen: Pen::Projection,
                    a: end + p.inward * (half + c.drip),
                    b: end - p.inward * (half + c.drip),
                });
            }
        }
        FlashingItem::Chimney(c) => {
            let Some(o) = creabim_core::roofmetal::chimney_outline(model, c.part) else {
                return out;
            };
            let m = o.iter().copied().sum::<DVec2>() / o.len() as f64;
            if creabim_core::roofmetal::roof_top_at(model, m).is_none_or(|t| t > z) {
                return out;
            }
            // the apron round the chimney, out over the roof
            let mut ccw = o.clone();
            if crate::parts::signed_area(&ccw) < 0.0 {
                ccw.reverse();
            }
            let n: Vec<DVec2> = (0..ccw.len())
                .map(|i| {
                    let a = ccw[i];
                    let b = ccw[(i + 1) % ccw.len()];
                    -(b - a).normalize_or_zero().perp()
                })
                .collect();
            let mut closed = ccw.clone();
            closed.push(ccw[0]);
            let mut normals = n.clone();
            normals.push(n[0]);
            let apron = creabim_core::flashing::offset_line(&closed, &normals, c.apron);
            for w in apron.windows(2) {
                out.push(PlanItem::Line {
                    id: e.id,
                    pen: Pen::Projection,
                    a: w[0],
                    b: w[1],
                });
            }
        }
        FlashingItem::Outlet(o) => {
            let top = creabim_core::roofmetal::roof_top_at(model, o.at).unwrap_or(0.0);
            if top > z {
                return out;
            }
            let r = (o.dn / 1000.0 / 2.0 + 0.05).max(0.05);
            for k in [r, o.dn / 2000.0] {
                ring(
                    &mut out,
                    e.id,
                    Pen::Projection,
                    &downpipe_outline(DownpipeProfile::Round, 2.0 * k, o.at),
                );
            }
            let g = o.dn / 2000.0;
            for (a, b) in [(DVec2::X, -DVec2::X), (DVec2::Y, -DVec2::Y)] {
                out.push(PlanItem::Line {
                    id: e.id,
                    pen: Pen::Projection,
                    a: o.at + a * g,
                    b: o.at + b * g,
                });
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
            FlashingItem::Capping(c) => {
                let Some(p) = creabim_core::roofmetal::capping_place(model, c) else {
                    continue;
                };
                let mid = (p.a + p.b) / 2.0;
                // the fall towards the roof, an arrow across the top
                arrow(
                    &mut out,
                    e.id,
                    mid - p.inward * (p.width * 0.35),
                    mid + p.inward * (p.width * 0.35),
                    k,
                );
                let at = (p.a * 0.75 + p.b * 0.25) - p.inward * (p.width / 2.0);
                (at, -p.inward)
            }
            FlashingItem::Chimney(c) => {
                let Some(o) = creabim_core::roofmetal::chimney_outline(model, c.part) else {
                    continue;
                };
                let m = o.iter().copied().sum::<DVec2>() / o.len() as f64;
                let far = o
                    .iter()
                    .copied()
                    .max_by(|a, b| (a.x + a.y).total_cmp(&(b.x + b.y)))
                    .unwrap_or(m);
                (far, (far - m).normalize_or(DVec2::Y))
            }
            FlashingItem::Outlet(o) => (o.at, DVec2::new(1.0, 1.0).normalize()),
        };
        out.push(PlanItem::Tag {
            id: e.id,
            anchor,
            // where it was dragged, its leader still starting on it
            at: anchor + f.mark_at.unwrap_or(out_dir * (9.0 * k)),
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

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::flashing::Flashing;
    use creabim_core::{Document, Element, ElementKind, Level, Transaction};

    #[test]
    fn a_dragged_mark_keeps_its_leader_on_the_item() {
        let mut doc = Document::new();
        let level = Element::new(
            "1. NP",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let lid = level.id;
        let gutter = Element::new(
            "Žľab",
            ElementKind::Flashing(Flashing::gutter(
                lid,
                None,
                vec![DVec2::ZERO, DVec2::X * 10.0],
            )),
        );
        let gid = gutter.id;
        doc.commit(Transaction::new("t").insert(level).insert(gutter))
            .unwrap();
        let tag = |doc: &Document| {
            roof_plan_marks(doc.model(), 100.0)
                .into_iter()
                .find_map(|it| match it {
                    PlanItem::Tag {
                        id,
                        anchor,
                        at,
                        text,
                        ..
                    } if id == gid => Some((anchor, at, text)),
                    _ => None,
                })
                .expect("the gutter's mark")
        };
        let (anchor, at, text) = tag(&doc);
        assert!(text.starts_with("K-"), "{text}");
        assert!((at - anchor).length() > 0.5, "beside it by itself");
        let to = DVec2::new(1.5, -2.0);
        doc.edit(gid, "move", |e| {
            if let ElementKind::Flashing(f) = &mut e.kind {
                f.mark_at = Some(to);
            }
        })
        .unwrap();
        let (a2, at2, _) = tag(&doc);
        // the leader still starts on the gutter, the bubble where it went
        assert!(a2.distance(anchor) < 1e-9);
        assert!(at2.distance(anchor + to) < 1e-9);
    }
}
