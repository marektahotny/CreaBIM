//! The legend of materials an architectural plan carries beside it, as
//! the office's ASR sheets do: a patch of each material's hatch with
//! what it is, for everything the plans cut (walls by their layers and
//! thicknesses, columns and beams). It is a schedule view, so it goes on
//! a sheet beside its plan.

use crate::joinerylist::wrap;
use crate::paper::Anchor;
use crate::plan::{Pen, PlanItem, PlanView};
use creabim_core::words::tr;
use creabim_core::{ElementId, ElementKind, Hatch, Model};
use glam::DVec2;

const HEAD: f64 = 3.2;
const TEXT: f64 = 2.2;
const LINE: f64 = 3.6;
/// A patch of hatch: its width and height, mm.
const PATCH: DVec2 = DVec2::new(16.0, 7.0);
const INDENT: f64 = 22.0;
const WIDTH: f64 = 150.0;
/// The sample's and the words' columns as drawn by default, mm.
pub const COLUMNS: (f64, f64) = (INDENT, WIDTH - INDENT);

/// One row of the legend.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub material: ElementId,
    pub hatch: Hatch,
    /// What it is: the material's description, else its name, with the
    /// thickness it is built in.
    pub text: String,
    /// Its thickness in walls, mm; none for columns and beams.
    pub thickness_mm: Option<i64>,
}

/// The materials the plans cut, each once for a thickness: walls first,
/// the thickest first, then what columns and beams are of.
pub fn rows(model: &Model) -> Vec<Row> {
    let mut out: Vec<Row> = vec![];
    let mut add = |id: ElementId, t: Option<f64>| {
        let Some(e) = model.get(id) else { return };
        let ElementKind::Material(m) = &e.kind else {
            return;
        };
        if m.hatch == Hatch::Empty && m.pattern.is_none() {
            return;
        }
        let mm = t.map(|t| (t * 1000.0).round() as i64);
        if out.iter().any(|r| r.material == id && r.thickness_mm == mm) {
            return;
        }
        let what = if m.description.trim().is_empty() {
            e.name.clone()
        } else {
            m.description.trim().to_string()
        };
        let text = match mm {
            Some(mm) => format!("{what}, {} {mm} mm", tr("thickness")),
            None => what,
        };
        out.push(Row {
            material: id,
            hatch: m.hatch,
            text,
            thickness_mm: mm,
        });
    };
    for e in model.iter() {
        let ElementKind::Wall(w) = &e.kind else {
            continue;
        };
        match creabim_core::assembly_of(model, e) {
            Some((_, a)) => {
                // what the wall is built of, not its plasters and foils
                let built = |l: &&creabim_core::Layer| {
                    a.layers.len() == 1
                        || matches!(
                            l.function,
                            creabim_core::LayerFunction::Structure
                                | creabim_core::LayerFunction::Insulation
                        )
                };
                for l in a.layers.iter().filter(built) {
                    if let Some(m) = l.material {
                        add(m, Some(l.thickness));
                    }
                }
            }
            None => {
                if let Some(m) = w.material {
                    add(m, Some(w.thickness));
                }
            }
        }
    }
    for e in model.iter() {
        if creabim_core::members::is_member(&e.kind) {
            if let Some(m) = creabim_core::members::material(&e.kind) {
                add(m, None);
            }
        }
    }
    out.sort_by_key(|r| (r.thickness_mm.is_none(), -r.thickness_mm.unwrap_or(0)));
    out
}

/// The legend as a drawing in paper millimetres, its top left corner at
/// the origin.
pub fn drawing(view: ElementId, title: &str, model: &Model) -> PlanView {
    // the columns' widths as set on the sheet
    let (ind, tw) = crate::schedule::legend_columns(model, view, INDENT, WIDTH - INDENT);
    let mut items = vec![];
    let text = |items: &mut Vec<PlanItem>, at: DVec2, t: &str, size: f64| {
        items.push(PlanItem::Text {
            id: view,
            at,
            text: t.to_string(),
            size,
            align: Anchor::Start,
            mm: 1.0,
            rgb: None,
            bold: false,
        })
    };
    text(
        &mut items,
        DVec2::new(0.0, 5.0 - HEAD / 2.0),
        &title.to_uppercase(),
        HEAD + 0.8,
    );
    let mut y = 0.0;
    let rows = rows(model);
    for r in &rows {
        let lines = wrap(&r.text, tw, TEXT);
        let h = PATCH.y.max(lines.len() as f64 * LINE);
        let top = y - (h - PATCH.y) / 2.0;
        let patch = vec![
            DVec2::new(0.0, top - PATCH.y),
            DVec2::new(PATCH.x, top - PATCH.y),
            DVec2::new(PATCH.x, top),
            DVec2::new(0.0, top),
        ];
        items.push(PlanItem::Fill {
            id: r.material,
            points: patch.clone(),
            hatch: r.hatch,
            rgb: None,
        });
        // its edge as lines: a cut polygon would fill the patch solid
        for i in 0..4 {
            items.push(PlanItem::Line {
                id: view,
                pen: Pen::Symbol,
                a: patch[i],
                b: patch[(i + 1) % 4],
            });
        }
        let mut ty = y - h / 2.0 + (lines.len() as f64 - 1.0) * LINE / 2.0;
        for l in &lines {
            text(&mut items, DVec2::new(ind, ty), l, TEXT);
            ty -= LINE;
        }
        y -= h + 3.0;
    }
    if rows.is_empty() {
        text(
            &mut items,
            DVec2::new(0.0, y - LINE / 2.0),
            &tr("No walls of a material with a hatch in the model yet."),
            TEXT,
        );
    }
    // the notes of the drawing, the legend view's own, a line each
    let notes: Vec<String> = match model.get(view).map(|e| &e.kind) {
        Some(ElementKind::View(v)) => v
            .notes
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|l| format!("- {}", l.trim_start_matches("- ")))
            .collect(),
        _ => vec![],
    };
    if !notes.is_empty() {
        y -= 3.0;
        text(
            &mut items,
            DVec2::new(0.0, y - HEAD / 2.0),
            &tr("NOTES"),
            HEAD + 0.8,
        );
        y -= HEAD + 3.0;
        for n in &notes {
            for l in wrap(&n.to_uppercase(), ind + tw, TEXT) {
                text(&mut items, DVec2::new(0.0, y - LINE / 2.0), &l, TEXT);
                y -= LINE;
            }
            y -= 1.0;
        }
    }
    // the project's zero above the sea, as the office's plans close
    let zero = model.project_info().map_or(0.0, |p| p.1.elevation);
    if zero.abs() > 1e-9 {
        y -= 3.0;
        text(
            &mut items,
            DVec2::new(0.0, y - LINE / 2.0),
            &format!("±0,000 = {}", format!("{zero:.2}").replace('.', ",")),
            TEXT + 0.6,
        );
        y -= LINE + 0.6;
    }
    // where the ground is against it at the building, from the site
    if let Some((lo, hi)) = crate::summary::ground_at_building(model) {
        let n = |v: f64| format!("{v:+.3}").replace('.', ",").replace('-', "−");
        let at = if (hi - lo).abs() < 0.0005 {
            n(lo)
        } else {
            format!("{} {} {}", n(lo), tr("to"), n(hi))
        };
        text(
            &mut items,
            DVec2::new(0.0, y - LINE / 2.0),
            &format!("{}: {at}", tr("Ground at the building")),
            TEXT + 0.6,
        );
    }
    PlanView { items }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sample house's walls give the legend its rows: a patch and a
    /// text each, no material twice in one thickness.
    #[test]
    fn the_legend_lists_each_wall_material_once_with_its_patch() {
        let m = creabim_core::sample::sample_house();
        let rows = rows(&m);
        assert!(!rows.is_empty());
        for (i, r) in rows.iter().enumerate() {
            assert!(rows[..i]
                .iter()
                .all(|o| (o.material, o.thickness_mm) != (r.material, r.thickness_mm)));
        }
        let d = drawing(ElementId::new(), "Legend of materials", &m);
        let patches = d
            .items
            .iter()
            .filter(|i| matches!(i, PlanItem::Fill { .. }))
            .count();
        assert_eq!(patches, rows.len());
        assert!(d
            .items
            .iter()
            .any(|i| matches!(i, PlanItem::Text { text, .. } if text == "LEGEND OF MATERIALS")));
    }
}
