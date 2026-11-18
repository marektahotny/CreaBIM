//! The legend of a services drawing, as the office's ZTI and ÚK sheets
//! carry it beside the plan: the pipes (a sample of each system's line
//! with its name and what it is made of) and the sanitary fixtures (each
//! mark with the fixture, its trap and its valves). It is a schedule
//! view, so it goes on a sheet beside its plan.

use crate::joinerylist::wrap;
use crate::paper::Anchor;
use crate::plan::{PlanItem, PlanView};
use creabim_core::mep::{SystemDef, SYSTEMS};
use creabim_core::viewmodes::Trade;
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

/// The equipment of the services: the placed objects of plumbing and of
/// heating and ventilation that are neither sanitary fixtures nor
/// heaters (heat pumps, tanks, boilers, manifolds), each type once with
/// its mark, or how many there are, and its name.
pub fn equipment(model: &Model, trade: Option<Trade>) -> Vec<(String, String)> {
    use creabim_core::objects::{self, ObjectCategory};
    let marks = creabim_core::marks::object_marks(model);
    // (type, how many)
    let mut found: Vec<(ElementId, usize)> = vec![];
    for e in model.iter() {
        if !matches!(e.kind, ElementKind::Object(_)) {
            continue;
        }
        let Some(r) = &e.of_type else { continue };
        let Some(t) = objects::type_of_id(model, r.id) else {
            continue;
        };
        let shown = matches!(
            (t.category, trade),
            (ObjectCategory::Plumbing | ObjectCategory::Hvac, None)
                | (ObjectCategory::Plumbing, Some(Trade::Plumbing))
                | (
                    ObjectCategory::Hvac,
                    Some(Trade::Heating | Trade::Ventilation)
                )
        );
        if !shown {
            continue;
        }
        let family = creabim_core::fixture::family_of(model, e);
        if family.is_some_and(|f| !creabim_core::fixture::is_manifold(f)) {
            continue;
        }
        match found.iter_mut().find(|f| f.0 == r.id) {
            Some(f) => f.1 += 1,
            None => found.push((r.id, 1)),
        }
    }
    found
        .into_iter()
        .map(|(id, n)| {
            let name = model.get(id).map(|t| t.name.clone()).unwrap_or_default();
            let lead = marks
                .get(&id)
                .filter(|m| !m.is_empty())
                .cloned()
                .unwrap_or_else(|| format!("{n}x"));
            (lead, name)
        })
        .collect()
}

/// The trade the legend drawn as `view` is for; none is for all of them.
pub fn trade_of(model: &Model, view: ElementId) -> Option<Trade> {
    match model.get(view).map(|e| &e.kind) {
        Some(ElementKind::View(v)) => v.trade,
        _ => None,
    }
}

/// The electrical devices, each type once: its kind and how many there
/// are, and the type's name.
pub fn devices(model: &Model) -> Vec<(String, String)> {
    use creabim_core::electrical as el;
    let mut found: Vec<(el::DeviceKind, String, usize)> = vec![];
    for (e, _, kind) in el::devices(model) {
        let name = e
            .of_type
            .as_ref()
            .and_then(|r| model.get(r.id))
            .map_or(e.name.clone(), |t| t.name.clone());
        match found.iter_mut().find(|d| d.0 == kind && d.1 == name) {
            Some(d) => d.2 += 1,
            None => found.push((kind, name, 1)),
        }
    }
    found.sort_by_key(|d| d.0 as u8);
    found
        .into_iter()
        .map(|(kind, name, n)| (format!("{n}x"), format!("{} - {name}", tr(kind.label()))))
        .collect()
}

/// The notes of the legend drawn as `view`: its own, a line each, else
/// the usual ones of the trades the model has pipes of.
pub fn notes(model: &Model, view: ElementId) -> Vec<String> {
    let own: Vec<String> = match model.get(view).map(|e| &e.kind) {
        Some(ElementKind::View(v)) => v
            .notes
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(str::to_string)
            .collect(),
        _ => vec![],
    };
    if !own.is_empty() {
        return own;
    }
    let trade = trade_of(model, view);
    if trade.is_some_and(|t| t != Trade::Plumbing && t != Trade::Heating) {
        return vec![];
    }
    let has = |codes: &[&str]| {
        model
            .iter()
            .any(|e| matches!(&e.kind, ElementKind::Pipe(p) if codes.contains(&p.system.as_str())))
    };
    let mut out = vec![];
    if has(&["SV", "TV", "CTV"]) {
        out.push(tr(
            "Insulate the water pipes with foamed polyethylene insulation of the prescribed thickness.",
        ));
    }
    if model.iter().any(|e| matches!(e.kind, ElementKind::Pipe(_))) {
        out.push(tr(
            "Where pipes in the floor pass through masonry, lay them in a sleeve.",
        ));
        out.push(tr(
            "Before the works, check the position and depth of the existing public mains and adapt the laying of the services to them on site.",
        ));
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
    let trade = trade_of(model, view);
    let of = |t: Trade| trade.is_none_or(|x| x == t);
    let mut systems = systems(model);
    systems.retain(|(s, _)| trade.is_none_or(|t| t.shows(s.kind)));
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
    let heated = model
        .iter()
        .any(|e| matches!(&e.kind, ElementKind::Room(r) if r.ufh.is_some()))
        && of(Trade::Heating);
    if heated {
        text(
            &mut items,
            DVec2::new(0.0, y - HEAD / 2.0),
            &tr("UNDERFLOOR HEATING"),
            HEAD,
            None,
        );
        y -= HEAD + 3.0;
        let pipe = format!(
            "{}x{}",
            creabim_services::floorheat::PIPE_OUTER_MM,
            (creabim_services::floorheat::PIPE_OUTER_MM
                - creabim_services::floorheat::PIPE_INNER_MM)
                / 2.0
        );
        for (name, rgb, dashes) in [
            ("underfloor heating - supply", crate::heatplan::SUPPLY, None),
            (
                "underfloor heating - return",
                crate::heatplan::RETURN,
                Some(crate::heatplan::DASH),
            ),
        ] {
            let a = DVec2::new(0.0, y - LINE / 2.0);
            crate::heatplan::strips(
                &mut items,
                view,
                &[a, a + DVec2::X * SAMPLE],
                rgb,
                dashes,
                1.0,
            );
            text(
                &mut items,
                DVec2::new(INDENT, y - LINE / 2.0),
                &format!("{} {pipe}", tr(name)),
                TEXT,
                None,
            );
            y -= LINE + 1.0;
        }
        // the edge of a heated field
        let a = DVec2::new(0.0, y - LINE / 2.0);
        crate::heatplan::strips(
            &mut items,
            view,
            &[a, a + DVec2::X * SAMPLE],
            crate::heatplan::FIELD,
            Some((1.5, 1.0)),
            1.0,
        );
        text(
            &mut items,
            DVec2::new(INDENT, y - LINE / 2.0),
            &tr("edge of a heated field"),
            TEXT,
            None,
        );
        y -= LINE + 4.0;
    }
    let rows = if of(Trade::Plumbing) {
        creabim_services::fixturespec::legend(model, None)
    } else {
        vec![]
    };
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
    let equipment = equipment(model, trade);
    if !equipment.is_empty() {
        y -= 1.5;
        text(
            &mut items,
            DVec2::new(0.0, y - HEAD / 2.0),
            &tr("EQUIPMENT"),
            HEAD,
            None,
        );
        y -= HEAD + 3.0;
        for (lead, name) in &equipment {
            text(
                &mut items,
                DVec2::new(2.0, y - LINE / 2.0),
                lead,
                TEXT + 0.6,
                Some(crate::mep::LABEL_GREEN),
            );
            for l in wrap(name, WIDTH - INDENT, TEXT) {
                text(
                    &mut items,
                    DVec2::new(INDENT, y - LINE / 2.0),
                    &l,
                    TEXT,
                    None,
                );
                y -= LINE;
            }
            y -= 1.5;
        }
    }
    let devices = if of(Trade::Electrical) {
        devices(model)
    } else {
        vec![]
    };
    if !devices.is_empty() {
        y -= 1.5;
        text(
            &mut items,
            DVec2::new(0.0, y - HEAD / 2.0),
            &tr("ELECTRICAL DEVICES"),
            HEAD,
            None,
        );
        y -= HEAD + 3.0;
        for (lead, name) in &devices {
            text(
                &mut items,
                DVec2::new(2.0, y - LINE / 2.0),
                lead,
                TEXT + 0.6,
                Some(crate::mep::LABEL_GREEN),
            );
            for l in wrap(name, WIDTH - INDENT, TEXT) {
                text(
                    &mut items,
                    DVec2::new(INDENT, y - LINE / 2.0),
                    &l,
                    TEXT,
                    None,
                );
                y -= LINE;
            }
            y -= 1.5;
        }
    }
    let notes = notes(model, view);
    if !notes.is_empty() {
        y -= 1.5;
        text(
            &mut items,
            DVec2::new(0.0, y - HEAD / 2.0),
            &format!("{}:", tr("NOTE")),
            HEAD,
            None,
        );
        y -= HEAD + 3.0;
        for n in &notes {
            let n = format!("- {}", n.trim_start_matches("- "));
            for l in wrap(&n.to_uppercase(), WIDTH, TEXT) {
                text(&mut items, DVec2::new(0.0, y - LINE / 2.0), &l, TEXT, None);
                y -= LINE;
            }
            y -= 1.0;
        }
        // the project's zero above the sea, as the office's legends close
        let zero = model.project_info().map_or(0.0, |p| p.1.elevation);
        if zero.abs() > 1e-9 {
            y -= 1.5;
            text(
                &mut items,
                DVec2::new(0.0, y - LINE / 2.0),
                &format!("±0,000 = {}", format!("{zero:.2}").replace('.', ",")),
                TEXT + 0.6,
                None,
            );
            y -= LINE + 0.6;
        }
        // a heating drawing's design outside temperature
        if heated {
            y -= 1.5;
            let te = creabim_services::heatloss::Opts::default().te;
            text(
                &mut items,
                DVec2::new(0.0, y - LINE / 2.0),
                &format!("te = {te:.0} °C"),
                TEXT + 0.6,
                None,
            );
        }
    }
    if systems.is_empty()
        && rows.is_empty()
        && !heated
        && equipment.is_empty()
        && devices.is_empty()
    {
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

    /// Under the fixtures the legend writes its notes: the usual ones
    /// of the pipes drawn, or the view's own, a line each.
    #[test]
    fn the_legend_closes_with_notes_its_own_or_the_usual() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let legend = Element::new(
            "Legend",
            ElementKind::View(creabim_core::View::schedule(
                creabim_core::ScheduleCategory::ServicesLegend,
            )),
        );
        let id = legend.id;
        let pipe = Element::new(
            "Pipe",
            ElementKind::Pipe(Pipe {
                level,
                system: "SV".into(),
                start: DVec3::ZERO,
                end: DVec3::X,
                pipe_type: "HERZ".into(),
                nominal_mm: 16.0,
                outer_mm: 20.0,
                inner_mm: 16.0,
                height_mm: None,
                insulation_mm: 0.0,
            }),
        );
        assert!(notes(doc.model(), id).is_empty());
        doc.commit(Transaction::new("t").insert(legend.clone()).insert(pipe))
            .unwrap();
        // water pipes: insulation, sleeves, the public mains
        assert_eq!(notes(doc.model(), id).len(), 3);
        let texts = |m: &Model| {
            drawing(id, "Legend", m)
                .items
                .iter()
                .filter_map(|i| match i {
                    PlanItem::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert!(texts(doc.model()).iter().any(|t| t == "NOTE:"));
        let mut own = legend.clone();
        if let ElementKind::View(v) = &mut own.kind {
            v.notes = "First\n\n- Second".into();
        }
        doc.commit(Transaction::new("t").replace(legend, own))
            .unwrap();
        assert_eq!(notes(doc.model(), id), ["First", "- Second"]);
        let t = texts(doc.model());
        assert!(t.iter().any(|t| t == "- FIRST") && t.iter().any(|t| t == "- SECOND"));
    }

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
