//! The legend of a services drawing, as the office's ZTI and ÚK sheets
//! carry it beside the plan: the pipes (a sample of each system's line
//! with its name and what it is made of) and the sanitary fixtures (each
//! mark with the fixture, its trap and its valves). It is a schedule
//! view, so it goes on a sheet beside its plan.

use crate::joinerylist::wrap;
use crate::paper::Anchor;
use crate::plan::{PlanItem, PlanView};
use creabim_core::mep::{SystemDef, SYSTEMS};
use creabim_core::words::tr;
use creabim_core::{ElementId, ElementKind, Model};
use glam::DVec2;

const HEAD: f64 = 3.2;
const TEXT: f64 = 2.2;
const LINE: f64 = 3.6;
/// Where a row's text starts, after its line sample or its mark, mm.
const INDENT: f64 = 26.0;
const SAMPLE: f64 = 20.0;
const WIDTH: f64 = 150.0;

/// The systems the model's pipes are in, in the systems' own order, each
/// with the names of the pipe types it is laid in.
pub fn systems(model: &Model) -> Vec<(&'static SystemDef, Vec<String>)> {
    let mut out: Vec<(&'static SystemDef, Vec<String>)> = vec![];
    for s in SYSTEMS {
        let mut types: Vec<String> = vec![];
        let mut any = false;
        for e in model.iter() {
            let ElementKind::Pipe(p) = &e.kind else {
                continue;
            };
            if p.system != s.code {
                continue;
            }
            any = true;
            let name = creabim_services::pipe_types::by_id(&p.pipe_type)
                .map_or(p.pipe_type.clone(), |t| t.name.clone());
            if !types.contains(&name) {
                types.push(name);
            }
        }
        if any {
            out.push((s, types));
        }
    }
    out
}

/// A sample of a system's line from `a`, `len` long: its dashes as thin
/// strips in its colour.
fn sample(items: &mut Vec<PlanItem>, id: ElementId, s: &SystemDef, a: DVec2, len: f64) {
    let rgb = [
        ((s.color >> 16) & 0xff) as f32 / 255.0,
        ((s.color >> 8) & 0xff) as f32 / 255.0,
        (s.color & 0xff) as f32 / 255.0,
    ];
    let half = crate::looks::PIPE_LINE / 2.0;
    let mut strip = |x0: f64, x1: f64| {
        items.push(PlanItem::Shade {
            id,
            points: vec![
                a + DVec2::new(x0, -half),
                a + DVec2::new(x1, -half),
                a + DVec2::new(x1, half),
                a + DVec2::new(x0, half),
            ],
            rgb,
        })
    };
    let Some(dashes) = s.dashes().filter(|d| d.iter().sum::<f64>() > 0.0) else {
        strip(0.0, len);
        return;
    };
    let (mut x, mut i) = (0.0, 0);
    while x < len {
        let d = dashes[i % dashes.len()];
        if i % 2 == 0 {
            // a dash of nothing is a dot
            strip(x, (x + d.max(2.0 * half)).min(len));
        }
        x += d;
        i += 1;
    }
}

/// The legend as a drawing in paper millimetres, its top left corner at
/// the origin.
pub fn drawing(view: ElementId, title: &str, model: &Model) -> PlanView {
    let mut items = vec![];
    let text = |items: &mut Vec<PlanItem>, at: DVec2, t: &str, size: f64, rgb| {
        items.push(PlanItem::Text {
            id: view,
            at,
            text: t.to_string(),
            size,
            align: Anchor::Start,
            mm: 1.0,
            rgb,
            bold: false,
        })
    };
    text(&mut items, DVec2::new(0.0, 5.0), title, 5.0, None);
    let mut y = -2.0;
    let systems = systems(model);
    if !systems.is_empty() {
        text(
            &mut items,
            DVec2::new(0.0, y - HEAD / 2.0),
            &tr("PIPES"),
            HEAD,
            None,
        );
        y -= HEAD + 3.0;
        for (s, types) in &systems {
            sample(&mut items, view, s, DVec2::new(0.0, y - LINE / 2.0), SAMPLE);
            let name = format!("{} - {}", tr(s.name), types.join(", "));
            for l in wrap(&name, WIDTH - INDENT, TEXT) {
                text(
                    &mut items,
                    DVec2::new(INDENT, y - LINE / 2.0),
                    &l,
                    TEXT,
                    None,
                );
                y -= LINE;
            }
            y -= 1.0;
        }
        y -= 3.0;
    }
    let rows = creabim_services::fixturespec::legend(model, None);
    if !rows.is_empty() {
        text(
            &mut items,
            DVec2::new(0.0, y - HEAD / 2.0),
            &tr("SANITARY FIXTURES"),
            HEAD,
            None,
        );
        y -= HEAD + 3.0;
        for r in &rows {
            text(
                &mut items,
                DVec2::new(2.0, y - LINE / 2.0),
                &r.spec.mark,
                TEXT + 0.6,
                Some(crate::mep::LABEL_GREEN),
            );
            for line in creabim_services::fixturespec::legend_lines(&r.spec) {
                for l in wrap(&line, WIDTH - INDENT, TEXT) {
                    text(
                        &mut items,
                        DVec2::new(INDENT, y - LINE / 2.0),
                        &l,
                        TEXT,
                        None,
                    );
                    y -= LINE;
                }
            }
            y -= 1.5;
        }
    }
    if systems.is_empty() && rows.is_empty() {
        text(
            &mut items,
            DVec2::new(0.0, y - LINE / 2.0),
            &tr("No pipes or sanitary fixtures in the model yet."),
            TEXT,
            None,
        );
    }
    PlanView { items }
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::mep::Pipe;
    use creabim_core::{Document, Element, Transaction};
    use glam::DVec3;

    #[test]
    fn the_legend_lists_the_systems_in_use_with_their_lines() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let view = ElementId::new();
        assert!(systems(doc.model()).is_empty());
        let mut tx = Transaction::new("pipes");
        for system in ["TV", "SV", "SV"] {
            tx = tx.insert(Element::new(
                "Pipe",
                ElementKind::Pipe(Pipe {
                    level,
                    system: system.into(),
                    start: DVec3::ZERO,
                    end: DVec3::X,
                    pipe_type: "HERZ".into(),
                    nominal_mm: 16.0,
                    outer_mm: 20.0,
                    inner_mm: 16.0,
                    height_mm: None,
                    insulation_mm: 0.0,
                }),
            ));
        }
        doc.commit(tx).unwrap();
        let ss = systems(doc.model());
        // in the systems' order, each once, with its pipe type's name
        assert_eq!(
            ss.iter().map(|(s, _)| s.code).collect::<Vec<_>>(),
            vec!["SV", "TV"]
        );
        assert_eq!(ss[0].1.len(), 1);
        let d = drawing(view, "Legend", doc.model());
        let texts: Vec<&str> = d
            .items
            .iter()
            .filter_map(|i| match i {
                PlanItem::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(texts[..2], ["Legend", "PIPES"]);
        assert!(texts[2].starts_with("Cold water - "));
        // cold water's sample: dashes and dots in its blue
        let strips = d
            .items
            .iter()
            .filter(|i| matches!(i, PlanItem::Shade { rgb, .. } if rgb[2] > 0.8))
            .count();
        assert!(strips >= 4, "{strips}");
    }
}
