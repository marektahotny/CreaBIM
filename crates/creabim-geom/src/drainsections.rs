//! The developed sections of the foul drainage (rozvinuté rezy
//! kanalizácie), as the office's ZTI sheet "Rezy kanalizácie" (ZTI.005):
//! a section for every stack (K1, K2…), its pipes unrolled into the
//! plane of the drawing, so each connecting pipe is drawn at its true
//! length and its true height. The stack is a solid line, the pipes into
//! it dashed, each with its outside diameter and an arrow the way it
//! drains; the fittings where they meet are written as their maker's
//! (`110/50-87°`, `R 110/50`), the fixtures drawn in their elevation with
//! their marks, the storeys' levels across with their heights, and under
//! the stack its mark with the waste water flow Qww of the calculation
//! (STN EN 12056-2, system I) and the capacity of the stack Qmax by its
//! DN (STN EN 12056-2, table 11). Drawn on paper (1 unit = 1 mm) at
//! [`SCALE`].

use crate::paper::Anchor;
use crate::plan::{Pen, PlanItem, PlanView};
use creabim_core::mep::{Pipe, SystemKind, SYSTEMS};
use creabim_core::words::tr;
use creabim_core::{ElementId, ElementKind, Model};
use creabim_services::network::NodeKind;
use creabim_services::{fittings, model as netmodel, sanitary, Settings};
use glam::{DVec2, DVec3};

/// The scale the sections are drawn at, as the office's sheet: 1:50.
pub const SCALE: f64 = 50.0;
/// Paper millimetres a metre of the model takes.
const K: f64 = 1000.0 / SCALE;
const HEAD: f64 = 3.2;
const TEXT: f64 = 2.2;
const SMALL: f64 = 1.8;
/// The widest row of sections before the next row starts, mm.
const ROW_WIDTH: f64 = 400.0;
/// Room left of a section for its levels' heights, and right of it, mm.
const LEFT: f64 = 16.0;
const RIGHT: f64 = 8.0;
/// A pipe shorter than this gets no size and no arrow, m.
const SHORT: f64 = 0.25;
/// How near a point must be to a pipe's end to be at it, m.
const NEAR: f64 = 0.03;

/// A pipe of a section, unrolled: its ends in the section (developed
/// length along, height), downstream end first.
#[derive(Clone, Debug, PartialEq)]
pub struct LaidPipe {
    pub pipe: ElementId,
    pub a: DVec2,
    pub b: DVec2,
    /// Where its ends are in the model.
    pub a3: DVec3,
    pub b3: DVec3,
    /// Part of the stack itself.
    pub stack: bool,
    pub outer_mm: f64,
    pub rgb: [f32; 3],
}

/// One stack's developed section, in metres: along the drawing, and
/// height above ±0,000.
#[derive(Clone, Debug, PartialEq)]
pub struct Section {
    /// K1, K2… as the plans mark the stack.
    pub mark: String,
    pub pipes: Vec<LaidPipe>,
    /// The fixtures draining into it and where their connection is laid.
    pub fixtures: Vec<(ElementId, DVec2)>,
    /// The stack's DN, its waste water flow Qww and its capacity Qmax,
    /// l/s.
    pub dn: f64,
    pub qww: f64,
    pub qmax: f64,
}

impl Section {
    /// Where a point of the model on one of its pipes is laid.
    pub fn lay(&self, p: DVec3) -> Option<DVec2> {
        self.pipes.iter().find_map(|q| {
            let d = q.b3 - q.a3;
            let l2 = d.length_squared();
            if l2 < 1e-12 {
                return (q.a3.distance(p) <= NEAR).then_some(q.a);
            }
            let t = ((p - q.a3).dot(d) / l2).clamp(0.0, 1.0);
            (q.a3.lerp(q.b3, t).distance(p) <= NEAR).then(|| q.a.lerp(q.b, t))
        })
    }

    fn extent(&self) -> (DVec2, DVec2) {
        let pts = self.pipes.iter().flat_map(|p| [p.a, p.b]);
        pts.fold(
            (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN)),
            |(lo, hi), p| (lo.min(p), hi.max(p)),
        )
    }
}

fn rgb_of(c: u32) -> [f32; 3] {
    [
        ((c >> 16) & 0xff) as f32 / 255.0,
        ((c >> 8) & 0xff) as f32 / 255.0,
        (c & 0xff) as f32 / 255.0,
    ]
}

/// The developed sections of every stack of the model's foul drainage,
/// in the order of their marks.
pub fn sections(model: &Model) -> Vec<Section> {
    let mut out = vec![];
    for sys in SYSTEMS.iter().filter(|s| s.kind == SystemKind::Sanitary) {
        let settings = Settings::default();
        let Some(m) = netmodel::build(model, sys.code, &settings) else {
            continue;
        };
        let calc = sanitary::run(
            model,
            sys.code,
            &settings,
            &sanitary::DrainSettings::default(),
        );
        let net = &m.net;
        let n = net.nodes.len();
        let pipe_of = |i: usize| -> Option<(ElementId, &Pipe)> {
            let id = m.pipes[i]?;
            match &model.get(id)?.kind {
                ElementKind::Pipe(p) => Some((id, p)),
                _ => None,
            }
        };
        // the stack each upright pipe is of
        let mark: Vec<Option<String>> = (0..n)
            .map(|i| {
                let node = &net.nodes[i];
                if !node.is_pipe() || !node.is_vertical() {
                    return None;
                }
                crate::mep::stack_mark(model, pipe_of(i)?.1)
            })
            .collect();
        let mut marks: Vec<String> = mark.iter().flatten().cloned().collect();
        marks.sort_by_key(|s| {
            let d: String = s.chars().filter(char::is_ascii_digit).collect();
            (d.parse::<u64>().unwrap_or(u64::MAX), s.clone())
        });
        marks.dedup();
        let v3 = |p: [f64; 3]| DVec3::new(p[0], p[1], p[2]);
        let order = net.pre_order();
        for k in marks {
            // each node's place along the drawing at its two ends, and
            // the side of the stack its branch is laid to (0: on it)
            let mut x: Vec<Option<(f64, f64)>> = vec![None; n];
            let mut side = vec![0.0f64; n];
            // how far each side's branches reach already, m: a new
            // branch goes to the side less taken, so they keep apart
            let mut reach = [0.0f64; 2];
            let mut sec = Section {
                mark: k.clone(),
                pipes: vec![],
                fixtures: vec![],
                dn: 0.0,
                qww: 0.0,
                qmax: 0.0,
            };
            let mut lowest: Option<(f64, usize)> = None;
            for &i in &order {
                let node = &net.nodes[i];
                let mine = mark[i].as_deref() == Some(k.as_str());
                if mine {
                    x[i] = Some((0.0, 0.0));
                    let z = node.p1[2].min(node.p2[2]);
                    if lowest.is_none_or(|l| z < l.0) {
                        lowest = Some((z, i));
                    }
                } else {
                    // another stack's pipes are its own section's
                    if mark[i].is_some() {
                        continue;
                    }
                    let Some(p) = node.parent else {
                        continue;
                    };
                    let Some((_, from)) = x[p] else {
                        continue;
                    };
                    let mut s = side[p];
                    if node.is_pipe() {
                        let d = DVec2::new(node.p2[0] - node.p1[0], node.p2[1] - node.p1[1]);
                        let run = d.length();
                        if s == 0.0 {
                            s = if reach[1] < reach[0] { -1.0 } else { 1.0 };
                        }
                        x[i] = Some((from, from + s * run));
                    } else {
                        x[i] = Some((from, from));
                    }
                    side[i] = s;
                    let far = x[i].map_or(0.0, |(a, b)| a.abs().max(b.abs()));
                    // a fixture takes its width beside where it drains
                    let far = if node.kind == NodeKind::Fixture {
                        far + 0.45
                    } else {
                        far
                    };
                    let k = usize::from(s < 0.0);
                    if s != 0.0 {
                        reach[k] = reach[k].max(far);
                    }
                }
                let (xa, xb) = x[i].expect("just laid");
                if node.is_pipe() {
                    if let Some((id, p)) = pipe_of(i) {
                        sec.pipes.push(LaidPipe {
                            pipe: id,
                            a: DVec2::new(xa, node.p1[2]),
                            b: DVec2::new(xb, node.p2[2]),
                            a3: v3(node.p1),
                            b3: v3(node.p2),
                            stack: mine,
                            outer_mm: p.outer_mm,
                            rgb: rgb_of(sys.color),
                        });
                    }
                } else if node.kind == NodeKind::Fixture {
                    if let Some(f) = m.fixtures[i] {
                        sec.fixtures.push((f, DVec2::new(xa, node.p1[2])));
                    }
                }
            }
            if let Some((_, i)) = lowest {
                if let Some((id, p)) = pipe_of(i) {
                    sec.dn = sanitary::dn_of_inner(p.inner_mm);
                    sec.qmax = sanitary::stack_capacity(sec.dn, false);
                    sec.qww = calc
                        .as_ref()
                        .and_then(|c| c.pipes.iter().find(|r| r.id == id))
                        .map_or(0.0, |r| r.flow_ls);
                }
            }
            out.push(sec);
        }
    }
    out
}

/// A fixture drawn in its elevation: its parts' boxes (lower and upper
/// corners, m) and the height of its top.
type Outline = (ElementId, Vec<(DVec2, DVec2)>, f64);

/// A number with a decimal comma and `places` places.
fn num(v: f64, places: usize) -> String {
    format!("{v:.places$}").replace('.', ",")
}

/// A height as levels write it: `±0,000`, `+3,000`, `-1,300`.
fn height(z: f64) -> String {
    if z.abs() < 5e-4 {
        "±0,000".into()
    } else if z > 0.0 {
        format!("+{}", num(z, 3))
    } else {
        num(z, 3)
    }
}

/// What a fitting is written as on the section: a branch its sizes and
/// angle (`110/50-87°`), a reducer `R 110/50`, an elbow `75-45°`.
pub fn fitting_text(f: &fittings::Fitting) -> Option<String> {
    use fittings::FittingKind as F;
    let sizes =
        f.d.iter()
            .map(|d| format!("{d:.0}"))
            .collect::<Vec<_>>()
            .join("/");
    let angle = f.angle.map_or(String::new(), |a| format!("-{a:.0}°"));
    match f.kind {
        F::Branch | F::Tee | F::Cross => Some(format!("{sizes}{angle}")),
        F::Reducer => Some(format!("R {sizes}")),
        F::Elbow => Some(format!(
            "{}{angle}",
            f.d.first().map_or(String::new(), |d| format!("{d:.0}"))
        )),
        F::Transition => None,
    }
}

/// The drawing of the view `view`, named `title`.
pub fn drawing(view: ElementId, title: &str, model: &Model) -> PlanView {
    let mut items: Vec<PlanItem> = vec![];
    let green = Some(crate::mep::LABEL_GREEN);
    let text = |items: &mut Vec<PlanItem>,
                at: DVec2,
                t: &str,
                size: f64,
                align: Anchor,
                rgb: Option<[f32; 3]>| {
        items.push(PlanItem::Text {
            id: view,
            at,
            text: t.to_string(),
            size,
            align,
            mm: 1.0,
            rgb,
            bold: false,
        })
    };
    let line = |items: &mut Vec<PlanItem>, a: DVec2, b: DVec2, pen: Pen| {
        items.push(PlanItem::Line {
            id: view,
            pen,
            a,
            b,
        })
    };
    let p = DVec2::new;
    items.push(PlanItem::Text {
        id: view,
        at: p(0.0, 6.0),
        text: title.to_uppercase(),
        size: HEAD + 0.8,
        align: Anchor::Start,
        mm: 1.0,
        rgb: None,
        bold: true,
    });
    let secs = sections(model);
    if secs.is_empty() {
        text(
            &mut items,
            p(0.0, -4.0),
            &tr("No foul drainage stack in the model yet."),
            TEXT,
            Anchor::Start,
            None,
        );
        return PlanView { items };
    }
    let levels: Vec<f64> = model.levels().map(|(_, l)| l.elevation).collect();
    // the heights every section spans: its levels and its pipes
    let (mut zlo, mut zhi) = (f64::MAX, f64::MIN);
    for s in &secs {
        let (lo, hi) = s.extent();
        zlo = zlo.min(lo.y);
        zhi = zhi.max(hi.y);
    }
    let levels: Vec<f64> = levels
        .into_iter()
        .filter(|z| *z >= zlo - 0.5 && *z <= zhi + 0.5)
        .collect();
    for z in &levels {
        zlo = zlo.min(*z);
        zhi = zhi.max(*z);
    }
    let fits = fittings::derive(model);
    let marks: Vec<(ElementId, String)> = creabim_services::fixturespec::legend(model, None)
        .into_iter()
        .flat_map(|r| {
            let m = r.spec.mark.clone();
            r.fixtures.into_iter().map(move |f| (f, m.clone()))
        })
        .collect();
    // under the title, a row of sections after another
    let row_height = (zhi - zlo) * K + 40.0;
    let mut ox = 0.0;
    let mut top = -12.0;
    for s in &secs {
        let (lo, hi) = s.extent();
        // the fixtures widen it
        let (mut xlo, mut xhi) = (lo.x, hi.x);
        let outlines: Vec<Outline> = s
            .fixtures
            .iter()
            .filter_map(|(id, at)| {
                let e = model.get(*id)?;
                let ElementKind::Object(o) = &e.kind else {
                    return None;
                };
                let t = creabim_core::objects::type_of(model, e)?;
                let (blo, bhi) = t.bounds();
                let mid = (blo.x + bhi.x) / 2.0;
                let floor = model.level(o.level).map_or(0.0, |l| l.elevation) + o.offset;
                let boxes: Vec<(DVec2, DVec2)> = t
                    .parts
                    .iter()
                    .filter_map(|part| {
                        let mut v = part.mesh.vertices.iter();
                        let f = *v.next()?;
                        let (l, h) = v.fold((f, f), |(l, h), q| (l.min(*q), h.max(*q)));
                        Some((
                            p(at.x + l.x - mid, floor + l.z),
                            p(at.x + h.x - mid, floor + h.z),
                        ))
                    })
                    .collect();
                Some((*id, boxes, floor + bhi.z))
            })
            .collect();
        for (_, boxes, _) in &outlines {
            for (l, h) in boxes {
                xlo = xlo.min(l.x);
                xhi = xhi.max(h.x);
            }
        }
        let width = ((xhi - xlo) * K + LEFT + RIGHT).max(70.0);
        if ox > 0.0 && ox + width > ROW_WIDTH {
            ox = 0.0;
            top -= row_height;
        }
        // paper from the section's metres
        let base = top - (zhi - zlo) * K - 10.0;
        let at = |q: DVec2| p(ox + LEFT + (q.x - xlo) * K, base + (q.y - zlo) * K);
        let (x0, x1) = (ox + 2.0, ox + width - 2.0);
        // the levels across, below ±0 dashed, each with its height
        for z in &levels {
            let y = at(p(xlo, *z)).y;
            let pen = if *z < -1e-6 {
                Pen::Hidden
            } else {
                Pen::Projection
            };
            line(&mut items, p(x0, y), p(x1, y), pen);
            line(
                &mut items,
                p(x0 + 2.0, y),
                p(x0 + 0.9, y + 1.6),
                Pen::Symbol,
            );
            line(
                &mut items,
                p(x0 + 0.9, y + 1.6),
                p(x0 + 3.1, y + 1.6),
                Pen::Symbol,
            );
            line(
                &mut items,
                p(x0 + 3.1, y + 1.6),
                p(x0 + 2.0, y),
                Pen::Symbol,
            );
            text(
                &mut items,
                p(x0 + 4.0, y + 0.6),
                &height(*z),
                TEXT,
                Anchor::Start,
                green,
            );
        }
        // the fixtures in their elevation, their marks over them
        for (id, boxes, top_z) in &outlines {
            for (l, h) in boxes {
                let (l, h) = (at(*l), at(*h));
                items.push(PlanItem::Polygon {
                    id: *id,
                    pen: Pen::Fine,
                    points: vec![l, p(h.x, l.y), h, p(l.x, h.y)],
                });
            }
            let fx = s
                .fixtures
                .iter()
                .find(|f| f.0 == *id)
                .map_or(0.0, |f| f.1.x);
            if let Some((_, m)) = marks.iter().find(|m| m.0 == *id) {
                text(
                    &mut items,
                    at(p(fx, *top_z)) + p(0.0, 2.5),
                    m,
                    HEAD,
                    Anchor::Middle,
                    green,
                );
            }
        }
        // the pipes: the stack solid, those into it dashed, each with its
        // size and the way it drains
        let mut sized: Vec<(ElementId, bool)> = vec![];
        for q in &s.pipes {
            let (a, b) = (at(q.a), at(q.b));
            items.push(PlanItem::Curve {
                id: q.pipe,
                points: vec![a, b],
                rgb: q.rgb,
                dashed: !q.stack,
            });
            if q.a3.distance(q.b3) < SHORT || (a - b).length() < 1.0 {
                continue;
            }
            let mid = (a + b) / 2.0;
            let u = (a - b).normalize();
            let n = u.perp();
            let tip = mid + u * 0.8;
            line(&mut items, tip, tip - u * 1.4 + n * 1.0, Pen::Projection);
            line(&mut items, tip, tip - u * 1.4 - n * 1.0, Pen::Projection);
            let vertical = (a.x - b.x).abs() < 1e-6;
            let key = (q.pipe, vertical);
            if sized.contains(&key) {
                continue;
            }
            sized.push(key);
            let size = format!("{:.0}", q.outer_mm);
            if vertical {
                text(
                    &mut items,
                    mid + p(-1.5, 3.0),
                    &size,
                    SMALL,
                    Anchor::End,
                    green,
                );
            } else {
                text(
                    &mut items,
                    mid + p(0.0, 1.2),
                    &size,
                    SMALL,
                    Anchor::Middle,
                    green,
                );
            }
        }
        // the fittings where its pipes meet
        let mut written: Vec<DVec2> = vec![];
        for f in &fits {
            let Some(t) = fitting_text(f) else {
                continue;
            };
            if !f
                .pipes
                .iter()
                .all(|id| s.pipes.iter().any(|q| q.pipe == *id))
            {
                continue;
            }
            let Some(q) = s.lay(f.pos) else {
                continue;
            };
            let here = at(q);
            if written.iter().any(|w| w.distance(here) < 0.5) {
                continue;
            }
            written.push(here);
            text(
                &mut items,
                here + p(1.2, -2.6),
                &t,
                SMALL,
                Anchor::Start,
                green,
            );
        }
        // its accessories: the vent cap, the cleaning piece
        for e in model.iter() {
            let ElementKind::Valve(v) = &e.kind else {
                continue;
            };
            if !s.pipes.iter().any(|q| q.pipe == v.pipe) {
                continue;
            }
            let Some(ElementKind::Pipe(pp)) = model.get(v.pipe).map(|e| &e.kind) else {
                continue;
            };
            let z = model.level(pp.level).map_or(0.0, |l| l.elevation);
            let w = pp.start.lerp(pp.end, v.t) + DVec3::Z * z;
            let Some(q) = s.lay(w) else {
                continue;
            };
            let c = at(q);
            let name = if e.name.is_empty() {
                v.kind_def().map_or(v.kind.clone(), |k| k.name.to_string())
            } else {
                e.name.clone()
            };
            let r = 1.2;
            items.push(PlanItem::Polygon {
                id: e.id,
                pen: Pen::Projection,
                points: vec![c + p(0.0, r), c + p(r, 0.0), c + p(0.0, -r), c + p(-r, 0.0)],
            });
            text(
                &mut items,
                c + p(r + 1.0, -0.8),
                &name,
                SMALL,
                Anchor::Start,
                green,
            );
        }
        // its mark in a circle under it, with its flows
        let lowest = at(p(0.0, zlo)).y;
        let c = p(at(p(0.0, 0.0)).x, lowest - 9.0);
        items.push(PlanItem::Arc {
            id: view,
            pen: Pen::Projection,
            center: c,
            radius: 3.5,
            start: 0.0,
            end: std::f64::consts::TAU,
        });
        text(
            &mut items,
            c + p(0.0, -HEAD * 0.36),
            &s.mark,
            HEAD,
            Anchor::Middle,
            Some([0.55, 0.1, 0.1]),
        );
        let lines = [
            format!("Qww = {} l/s", num(s.qww, 3)),
            format!("Qmax = {} l/s (DN {:.0})", num(s.qmax, 1), s.dn),
        ];
        for (j, l) in lines.iter().enumerate() {
            text(
                &mut items,
                c + p(5.5, 1.5 - j as f64 * 3.2),
                l,
                SMALL,
                Anchor::Start,
                green,
            );
        }
        ox += width + 6.0;
    }
    PlanView { items }
}

/// The sections as a table, for the export to a spreadsheet: a row for
/// each stack.
pub fn schedule(model: &Model) -> crate::schedule::Schedule {
    use crate::schedule::{Column, Row, Schedule};
    let col = |title: &'static str, width: f64, numeric: bool| Column {
        title,
        field: None,
        width,
        numeric,
        align: None,
    };
    let columns = vec![
        col("Stack", 16.0, false),
        col("DN", 14.0, true),
        col("Qww l/s", 18.0, true),
        col("Qmax l/s", 18.0, true),
        col("Fixtures", 60.0, false),
    ];
    let marks: Vec<(ElementId, String)> = creabim_services::fixturespec::legend(model, None)
        .into_iter()
        .flat_map(|r| {
            let m = r.spec.mark.clone();
            r.fixtures.into_iter().map(move |f| (f, m.clone()))
        })
        .collect();
    let rows: Vec<Row> = sections(model)
        .into_iter()
        .map(|s| {
            let fx = s
                .fixtures
                .iter()
                .filter_map(|(id, _)| marks.iter().find(|m| m.0 == *id).map(|m| m.1.clone()))
                .collect::<Vec<_>>()
                .join(", ");
            Row {
                id: ElementId::default(),
                cells: vec![
                    s.mark,
                    format!("{:.0}", s.dn),
                    num(s.qww, 3),
                    num(s.qmax, 1),
                    fx,
                ],
                group: false,
            }
        })
        .collect();
    let listed = rows.len();
    Schedule {
        category: creabim_core::ScheduleCategory::DrainSections,
        columns,
        rows,
        totals: vec![],
        listed,
        above_totals: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::{Document, Element, Transaction};
    use glam::DVec2;

    /// A sample house with a WC and a basin draining into a stack that
    /// runs from under the floor up through the roof.
    fn house() -> Document {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let mut ids = vec![];
        for (key, x) in [("starter/washbasin", 1.0), ("starter/wc", 3.0)] {
            let o = creabim_core::catalogue::starter(key).unwrap();
            let (tx, id) =
                creabim_core::objects::place(doc.model(), &o, level, DVec2::new(x, 3.0), 0.0);
            doc.commit(tx).unwrap();
            ids.push(id);
        }
        let conn = |id| {
            let e = doc.model().get(id).unwrap();
            creabim_core::fixture::connectors(doc.model(), e)
                .into_iter()
                .find(|c| c.kind == creabim_core::fixture::ConnKind::K)
                .unwrap()
                .at
        };
        let (b, w) = (conn(ids[0]), conn(ids[1]));
        let pipe = |a: DVec3, b: DVec3, o: f64, i: f64, n: f64| {
            Element::new(
                "",
                ElementKind::Pipe(Pipe {
                    level,
                    system: "K".into(),
                    start: a,
                    end: b,
                    pipe_type: "PPHT".into(),
                    nominal_mm: n,
                    outer_mm: o,
                    inner_mm: i,
                    height_mm: None,
                    insulation_mm: 0.0,
                    insulation_set: false,
                    mark: None,
                }),
            )
        };
        // the stack behind the WC, the basin's pipe along the wall into it
        let s = DVec3::new(w.x, w.y + 0.15, 0.0);
        let at = |z: f64| DVec3::new(s.x, s.y, z);
        let bend = DVec3::new(b.x, s.y, b.z - 0.1);
        let tx = Transaction::new("drains")
            .insert(pipe(at(-1.0), at(0.02), 110.0, 104.6, 100.0))
            .insert(pipe(at(0.02), at(0.3), 110.0, 104.6, 100.0))
            .insert(pipe(at(0.3), at(8.0), 110.0, 104.6, 100.0))
            .insert(pipe(w, at(0.02), 110.0, 104.6, 100.0))
            .insert(pipe(b, DVec3::new(b.x, b.y, bend.z), 50.0, 46.4, 50.0))
            .insert(pipe(DVec3::new(b.x, b.y, bend.z), bend, 50.0, 46.4, 50.0))
            .insert(pipe(bend, at(0.3), 50.0, 46.4, 50.0))
            .insert(pipe(
                at(-1.0),
                DVec3::new(s.x, s.y + 4.0, -1.08),
                110.0,
                104.6,
                100.0,
            ));
        doc.commit(tx).unwrap();
        doc
    }

    #[test]
    fn a_stack_gets_its_section_with_its_fixtures_and_its_flow() {
        let doc = house();
        let secs = sections(doc.model());
        assert_eq!(secs.len(), 1, "{secs:#?}");
        let s = &secs[0];
        assert_eq!(s.mark, "K1");
        assert_eq!(s.fixtures.len(), 2);
        assert_eq!(s.dn, 100.0);
        // STN EN 12056-2: Qww = 0,5·√2,5 = 0,79, no less than the WC's 2,0
        assert!((s.qww - 2.0).abs() < 1e-9, "{s:?}");
        assert_eq!(s.qmax, 4.0);
        // the stack upright on 0, the basin's pipe laid out to its side
        assert!(s
            .pipes
            .iter()
            .filter(|p| p.stack)
            .all(|p| p.a.x == 0.0 && p.b.x == 0.0));
        let far = s.pipes.iter().map(|p| p.b.x.abs()).fold(0.0, f64::max);
        assert!(far > 1.5, "the basin's branch is laid at its length: {far}");
        // drawn with its mark, its flow and the fittings by size
        let d = drawing(ElementId::default(), "Drain sections", doc.model());
        let texts: Vec<&str> = d
            .items
            .iter()
            .filter_map(|it| match it {
                PlanItem::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"K1"), "{texts:?}");
        assert!(texts.contains(&"Qww = 2,000 l/s"), "{texts:?}");
        assert!(
            texts.iter().any(|t| t.starts_with("110/110/50")),
            "{texts:?}"
        );
        assert!(texts.contains(&"±0,000"), "{texts:?}");
    }

    #[test]
    fn no_drains_no_sections() {
        let m = creabim_core::sample::sample_house();
        assert!(sections(&m).is_empty());
        let d = drawing(ElementId::default(), "Drain sections", &m);
        assert!(d.items.len() == 2);
    }
}
