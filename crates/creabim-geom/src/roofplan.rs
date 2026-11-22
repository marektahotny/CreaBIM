//! The roof plan as STN drawings have it: the roof and every fold of it
//! in a thin solid line, as all else seen from above outside it or
//! standing up through it; under the roof, the load-bearing layer of the
//! outer walls and the outer columns in a thin dashed line.

use crate::plan::{Pen, PlanItem};
use creabim_core::{ElementId, ElementKind, Model};
use glam::DVec2;

/// The plan outlines of every roof face.
fn roof_faces(model: &Model) -> Vec<Vec<DVec2>> {
    model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Roof(r) => Some(r.faces().into_iter().map(|f| f.outline)),
            _ => None,
        })
        .flatten()
        .filter(|o| o.len() >= 3)
        .collect()
}

/// The parts of a–b under the roof.
fn under(faces: &[Vec<DVec2>], a: DVec2, b: DVec2) -> Vec<(DVec2, DVec2)> {
    let d = b - a;
    let mut ts = vec![0.0, 1.0];
    for f in faces {
        for i in 0..f.len() {
            let (p, q) = (f[i], f[(i + 1) % f.len()]);
            let e = q - p;
            let den = d.perp_dot(e);
            if den.abs() < 1e-12 {
                continue;
            }
            let t = (p - a).perp_dot(e) / den;
            let u = (p - a).perp_dot(d) / den;
            if (0.0..=1.0).contains(&t) && (-1e-9..=1.0 + 1e-9).contains(&u) {
                ts.push(t);
            }
        }
    }
    ts.sort_by(f64::total_cmp);
    let inside = |p: DVec2| faces.iter().any(|f| creabim_core::polygon::contains(f, p));
    let mut out: Vec<(DVec2, DVec2)> = vec![];
    for w in ts.windows(2) {
        if w[1] - w[0] < 1e-9 || !inside(a + d * ((w[0] + w[1]) / 2.0)) {
            continue;
        }
        let (p, q) = (a + d * w[0], a + d * w[1]);
        match out.last_mut() {
            Some(last) if last.1.distance(p) < 1e-9 => last.1 = q,
            _ => out.push((p, q)),
        }
    }
    out
}

/// Turn a drawing seen from above into a roof plan.
pub fn apply(model: &Model, items: &mut Vec<PlanItem>) {
    let faces = roof_faces(model);
    if faces.is_empty() {
        return;
    }
    // what is seen, thin and solid
    for it in items.iter_mut() {
        let (PlanItem::Line { id, pen, .. }
        | PlanItem::Polygon { id, pen, .. }
        | PlanItem::Arc { id, pen, .. }) = it
        else {
            continue;
        };
        let building = !matches!(
            model.get(*id).map(|e| &e.kind),
            Some(ElementKind::View(_) | ElementKind::Grid(_) | ElementKind::Level(_)) | None
        );
        if building
            && matches!(
                pen,
                Pen::Cut | Pen::Projection | Pen::Hidden | Pen::Overhead
            )
        {
            *pen = Pen::Symbol;
        }
    }
    // columns and beams under it only as their load-bearing core, dashed
    // (George 2026-11-22: no insulation or render round them)
    items.retain(|it| {
        !matches!(
            model.get(it.id()).map(|e| &e.kind),
            Some(ElementKind::Column(_) | ElementKind::Beam(_))
        )
    });
    // the cores of the outer walls and the outer columns under it
    let joins = creabim_core::wall_joins(model);
    let add = |id: ElementId, a: DVec2, b: DVec2, items: &mut Vec<PlanItem>| {
        for (a, b) in under(&faces, a, b) {
            items.push(PlanItem::Line {
                id,
                pen: Pen::Hidden,
                a,
                b,
            });
        }
    };
    let levels: Vec<ElementId> = model.levels().map(|(e, _)| e.id).collect();
    for level in levels {
        for wid in crate::sitefoot::outer_walls(model, level) {
            let Some(ElementKind::Wall(w)) = model.get(wid).map(|e| &e.kind) else {
                continue;
            };
            let len = w.length();
            if w.is_arc() || len < 1e-6 {
                continue;
            }
            let d = (w.end - w.start) / len;
            let n = d.perp();
            let ends = joins.get(&wid).cloned().unwrap_or_default();
            let (s0, s1) = creabim_core::walls::core_span(model, w);
            for s in [s0, s1] {
                let at = |u: f64| w.start + d * ends.place(w, u, s, true) + n * s;
                add(wid, at(0.0), at(len), items);
            }
        }
        for e in model.iter() {
            let ElementKind::Column(c) = &e.kind else {
                continue;
            };
            // a board of a wrap or the XPS of a wall column is no core
            let board = c.piece.is_some() || c.in_wall.as_ref().is_some_and(|w| w.of.is_some());
            if board || c.level != level || crate::firezones::room_at(model, level, c.position) {
                continue;
            }
            let o = c.outline();
            for i in 0..o.len() {
                add(e.id, o[i], o[(i + 1) % o.len()], items);
            }
        }
    }
    // the lintels and beams of the top storey under it, their core
    let top = model
        .levels()
        .filter(|(e, _)| {
            model
                .iter()
                .any(|x| matches!(&x.kind, ElementKind::Wall(w) if w.level == e.id))
        })
        .max_by(|a, b| a.1.elevation.total_cmp(&b.1.elevation))
        .map(|(e, _)| e.id);
    for e in model.iter() {
        let ElementKind::Beam(b) = &e.kind else {
            continue;
        };
        if b.piece.is_some() || Some(b.level) != top {
            continue;
        }
        let o = b.outline();
        for i in 0..o.len() {
            add(e.id, o[i], o[(i + 1) % o.len()], items);
        }
    }
}

#[cfg(test)]
mod tests {
    use creabim_core::{ElementKind, Transaction, View};

    #[test]
    fn a_roof_plan_draws() {
        let mut m = creabim_core::starter::starter_house();
        let top = m
            .levels()
            .max_by(|a, b| a.1.elevation.total_cmp(&b.1.elevation))
            .map(|(e, _)| e.id)
            .unwrap();
        let mut v = View::plan(top);
        v.graphics.range.from_above = true;
        let e = creabim_core::Element::new("Roof plan", ElementKind::View(v));
        let id = e.id;
        Transaction::new("roof").insert(e).apply(&mut m).unwrap();
        let d = crate::section::view_drawing(&m, id).unwrap();
        let dashed = d
            .items
            .iter()
            .filter(|it| {
                matches!(
                    it,
                    crate::plan::PlanItem::Line {
                        pen: crate::plan::Pen::Hidden,
                        ..
                    }
                )
            })
            .count();
        assert!(dashed >= 4, "{dashed}");
        // nothing heavy: the roof and all seen in a thin line
        assert!(!d.items.iter().any(|it| matches!(
            it,
            crate::plan::PlanItem::Line { pen: crate::plan::Pen::Cut | crate::plan::Pen::Projection, id, .. }
                | crate::plan::PlanItem::Polygon { pen: crate::plan::Pen::Cut | crate::plan::Pen::Projection, id, .. }
                if matches!(m.get(*id).map(|e| &e.kind), Some(ElementKind::Roof(_) | ElementKind::Wall(_)))
        )));
        if let Ok(dir) = std::env::var("CREABIM_ROOF_SVG") {
            std::fs::write(dir, d.to_svg(40.0)).unwrap();
        }
    }
}
