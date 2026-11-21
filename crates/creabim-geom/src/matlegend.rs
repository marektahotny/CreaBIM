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

/// The airborne sound reduction a text gives: `Rw 56 dB`, dB.
fn rw_in(text: &str) -> Option<u32> {
    let t = text.to_lowercase();
    let at = t.find("rw")? + 2;
    let rest = t[at..].trim_start_matches([' ', '=', ':']);
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// A layer the legend lists: structure and insulation, and a membrane
/// drawn as waterproofing (not a vapour foil).
fn built_of(model: &Model, l: &creabim_core::Layer) -> bool {
    use creabim_core::LayerFunction as F;
    match l.function {
        F::Structure | F::Insulation => true,
        F::Membrane => l.material.and_then(|m| model.get(m)).is_some_and(
            |e| matches!(&e.kind, ElementKind::Material(m) if m.hatch == Hatch::Membrane),
        ),
        _ => false,
    }
}

/// The renovation states of what the plan of `level` holds (every
/// storey's when none); none in a project that is not a renovation.
pub fn reno_states(model: &Model, level: Option<ElementId>) -> Vec<creabim_core::reno::Reno> {
    use creabim_core::reno::{has_state, state, Reno};
    let renovation = model
        .iter()
        .any(|e| has_state(&e.kind) && matches!(e.reno, Some(Reno::Existing | Reno::Demolished)));
    if !renovation {
        return vec![];
    }
    let mut out = vec![];
    for e in model.iter().filter(|e| has_state(&e.kind)) {
        let on = match (&e.kind, level) {
            (_, None) => true,
            (ElementKind::Wall(w), Some(l)) => crate::plan::wall_rise(model, w, l, 1.2).is_some(),
            (k, Some(l)) => {
                creabim_core::levels::level_of(k).or_else(|| creabim_core::members::level_of(k))
                    == Some(l)
            }
        };
        let st = state(e);
        if on && !out.contains(&st) {
            out.push(st);
        }
    }
    // in the order the legend gives them
    out.sort_by_key(|s| Reno::ALL.iter().position(|x| x == s));
    out
}

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
    rows_of(model, None, &[])
}

/// The rows of the legend `view` draws: the materials its storey's plan
/// cuts (every storey's when it names none), less those it leaves out.
pub fn rows_of_view(model: &Model, view: ElementId) -> Vec<Row> {
    let Some(ElementKind::View(v)) = model.get(view).map(|e| &e.kind) else {
        return rows(model);
    };
    match storeys(model, view) {
        None => rows_of(model, None, &v.left_out),
        Some(levels) => {
            let mut out: Vec<Row> = vec![];
            for l in levels {
                for r in rows_of(model, Some(l), &v.left_out) {
                    if !out
                        .iter()
                        .any(|x| x.material == r.material && x.thickness_mm == r.thickness_mm)
                    {
                        out.push(r);
                    }
                }
            }
            out.sort_by_key(|r| (r.thickness_mm.is_none(), -r.thickness_mm.unwrap_or(0)));
            out
        }
    }
}

/// The plans a legend stands beside: those placed on the sheets it is
/// placed on.
pub fn beside(model: &Model, view: ElementId) -> Vec<&creabim_core::View> {
    let sheets: Vec<ElementId> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Viewport(vp) if vp.view == view => Some(vp.sheet),
            _ => None,
        })
        .collect();
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::Viewport(vp) = &e.kind else {
            continue;
        };
        if !sheets.contains(&vp.sheet) {
            continue;
        }
        if let Some(ElementKind::View(p)) = model.get(vp.view).map(|x| &x.kind) {
            if matches!(p.kind, creabim_core::ViewKind::Plan { .. })
                && !p.ceiling_plan
                && !p.graphics.range.from_above
            {
                out.push(p);
            }
        }
    }
    out
}

/// The storeys a legend lists: its own, else those of the plans it
/// stands beside; none for every storey (beside sections, or not placed).
pub fn storeys(model: &Model, view: ElementId) -> Option<Vec<ElementId>> {
    if let Some(ElementKind::View(v)) = model.get(view).map(|e| &e.kind) {
        if let Some(l) = v.of_level {
            return Some(vec![l]);
        }
    }
    let mut out = vec![];
    for p in beside(model, view) {
        if let creabim_core::ViewKind::Plan { level, .. } = p.kind {
            if !out.contains(&level) {
                out.push(level);
            }
        }
    }
    (!out.is_empty()).then_some(out)
}

/// The renovation states a legend explains: those the drawings beside it
/// show in colour, with black for what stays; the whole renovation's when
/// it stands beside no plan.
pub fn reno_of_view(model: &Model, view: ElementId) -> Vec<creabim_core::reno::Reno> {
    use creabim_core::reno::Reno;
    let plans = beside(model, view);
    if plans.is_empty() {
        let level = match model.get(view).map(|e| &e.kind) {
            Some(ElementKind::View(v)) => v.of_level,
            _ => None,
        };
        return reno_states(model, level);
    }
    let mut out = vec![];
    for p in plans {
        let (Some(stage), creabim_core::ViewKind::Plan { level, .. }) = (p.reno, &p.kind) else {
            continue;
        };
        for r in reno_states(model, Some(*level)) {
            let shown = stage.shows(r) && (r == Reno::Existing || stage.colour(r).is_some());
            if shown && !out.contains(&r) {
                out.push(r);
            }
        }
    }
    out.sort_by_key(|s| Reno::ALL.iter().position(|x| x == s));
    out
}

/// The materials the plan of `level` cuts (every plan's when none),
/// less those `left_out`.
pub fn rows_of(model: &Model, level: Option<ElementId>, left_out: &[ElementId]) -> Vec<Row> {
    let mut all = cut(model, level);
    all.retain(|r| !left_out.contains(&r.material));
    all
}

/// A material's description as a legend writes it: without where its
/// data come from (a maker's data sheets, a handbook, a declaration of
/// performance) and without its own λ, which the legend adds itself for
/// an outer wall.
pub fn plain_description(description: &str) -> String {
    const SOURCES: [&str; 8] = [
        "technické listy",
        "technický list",
        "technical data",
        "podľa:",
        "príručk",
        "vyhlásenie o parametroch",
        "v liste neuveden",
        "orientačne",
    ];
    description
        .split(';')
        .map(str::trim)
        .filter(|part| {
            let low = part.to_lowercase();
            !part.is_empty() && !part.starts_with('λ') && !SOURCES.iter().any(|s| low.contains(s))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn cut(model: &Model, level: Option<ElementId>) -> Vec<Row> {
    // what a plan of the storey cuts: its walls and those rising through
    // it from below, its columns
    let on = |e: &creabim_core::Element| match (&e.kind, level) {
        (_, None) => true,
        (ElementKind::Wall(w), Some(l)) => crate::plan::wall_rise(model, w, l, 1.2).is_some(),
        // of the members only columns are cut; beams, footings and the
        // slabs are seen, not cut, so their materials show no hatch
        (k @ ElementKind::Column(_), Some(l)) => creabim_core::members::level_of(k) == Some(l),
        (_, Some(_)) => false,
    };
    let mut out: Vec<Row> = vec![];
    // what each row adds after its thickness: λ for an outer wall's
    // materials, Rw for a wall between dwellings
    let mut lambda: Vec<(ElementId, Option<i64>)> = vec![];
    let mut rw: Vec<((ElementId, Option<i64>), u32)> = vec![];
    let mut add = |id: ElementId, t: Option<f64>| -> Option<(ElementId, Option<i64>)> {
        let e = model.get(id)?;
        let ElementKind::Material(m) = &e.kind else {
            return None;
        };
        if m.hatch == Hatch::Empty && m.pattern.is_none() {
            return None;
        }
        let mm = t.map(|t| (t * 1000.0).round() as i64);
        if out.iter().any(|r| r.material == id && r.thickness_mm == mm) {
            return Some((id, mm));
        }
        // the built-in library's English words in the drawing's language
        let what = if m.description.trim().is_empty() {
            tr(e.name.trim())
        } else {
            tr(&plain_description(&m.description))
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
        Some((id, mm))
    };
    // the outer walls of each storey
    let mut outer: Vec<ElementId> = vec![];
    for (level, ..) in creabim_core::levels::sorted(model) {
        outer.extend(crate::sitefoot::outer_walls(model, level));
    }
    for e in model.iter().filter(|e| on(e)) {
        let ElementKind::Wall(w) = &e.kind else {
            continue;
        };
        let is_outer = outer.contains(&e.id);
        let mut made = vec![];
        let mut sound = None;
        match creabim_core::assembly_of(model, e) {
            Some((ae, a)) => {
                // what the wall is built of, not its plasters and foils;
                // a waterproofing strip as the ASR legend lists it
                let built = |l: &&creabim_core::Layer| a.layers.len() == 1 || built_of(model, l);
                for l in a.layers.iter().filter(built) {
                    if let Some(m) = l.material {
                        made.extend(add(m, Some(l.thickness)));
                    }
                }
                // a wall between dwellings, as its build-up is named
                let words = format!("{} {}", ae.name, a.description).to_lowercase();
                if words.contains("medzibyt") || words.contains("between dwellings") {
                    sound = rw_in(&words).or_else(|| {
                        made.iter().find_map(|(m, _)| {
                            model.get(*m).and_then(|x| match &x.kind {
                                ElementKind::Material(mat) => rw_in(&mat.description),
                                _ => None,
                            })
                        })
                    });
                }
            }
            None => {
                if let Some(m) = w.material {
                    made.extend(add(m, Some(w.thickness)));
                }
            }
        }
        for k in made {
            if is_outer && !lambda.contains(&k) {
                lambda.push(k);
            }
            if let Some(r) = sound {
                if !rw.iter().any(|x| x.0 == k) {
                    rw.push((k, r));
                }
            }
        }
    }
    // what floors and roofs are built of: their structure, insulation and
    // waterproofing, as the legend of the ASR sections lists them; a
    // storey's plan cuts no slab (the floor lies under its cut, the
    // ceiling over it), so its legend leaves them out
    for e in model.iter().filter(|_| level.is_none()) {
        if !matches!(e.kind, ElementKind::Slab(_) | ElementKind::Roof(_)) {
            continue;
        }
        if let Some((_, a)) = creabim_core::assembly_of(model, e) {
            for l in a.layers.iter().filter(|l| built_of(model, l)) {
                if let Some(m) = l.material {
                    add(m, Some(l.thickness));
                }
            }
        }
    }
    for e in model.iter().filter(|e| on(e)) {
        if creabim_core::members::is_member(&e.kind) {
            if let Some(m) = creabim_core::members::material(&e.kind) {
                add(m, None);
            }
        }
    }
    for r in &mut out {
        let k = (r.material, r.thickness_mm);
        if lambda.contains(&k) {
            if let Some(l) = model.get(r.material).and_then(|e| match &e.kind {
                ElementKind::Material(m) => m.lambda.filter(|l| *l > 0.0),
                _ => None,
            }) {
                r.text += &format!(", λ = {} W/(m·K)", format!("{l:.3}").replace('.', ","));
            }
        }
        if let Some((_, v)) = rw.iter().find(|x| x.0 == k) {
            if rw_in(&r.text).is_none() {
                r.text += &format!(", Rw = {v} dB");
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
    let rows = rows_of_view(model, view);
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
    // the ground under the hatches and the drawing style as in the plans
    // it stands beside: its storey's plan, else the legend's own
    let (level, own) = match model.get(view).map(|e| &e.kind) {
        Some(ElementKind::View(v)) => (v.of_level, Some(v.graphics.clone())),
        _ => (None, None),
    };
    let placed = beside(model, view).first().map(|p| p.graphics.clone());
    let plan = placed.or_else(|| {
        level.and_then(|l| {
            model.iter().find_map(|x| match &x.kind {
                ElementKind::View(p)
                    if matches!(p.kind, creabim_core::ViewKind::Plan { level, .. } if level == l)
                        && !p.ceiling_plan
                        && !p.graphics.range.from_above
                        && p.callout.is_none() =>
                {
                    Some(p.graphics.clone())
                }
                _ => None,
            })
        })
    });
    if let Some(g) = plan.or(own) {
        crate::section::apply_drawing_style(model, &mut items, g.drawing_style);
        crate::section::apply_hatch_ground(&mut items, g.drawing_style, g.hatch_ground);
    }
    // a renovation: what stays black, what is taken down yellow, what is
    // built red, for the states the storey has
    let states = reno_of_view(model, view);
    if !states.is_empty() {
        y -= 2.0;
        for (state, rgb, words) in [
            (
                creabim_core::reno::Reno::Existing,
                [0.0, 0.0, 0.0],
                "Existing constructions, staying unchanged",
            ),
            (
                creabim_core::reno::Reno::Demolished,
                creabim_core::reno::DEMOLISHED,
                "Demolished constructions and elements",
            ),
            (
                creabim_core::reno::Reno::New,
                creabim_core::reno::NEW,
                "Proposed constructions and elements",
            ),
        ] {
            if !states.contains(&state) {
                continue;
            }
            let patch = vec![
                DVec2::new(0.0, y - PATCH.y),
                DVec2::new(PATCH.x, y - PATCH.y),
                DVec2::new(PATCH.x, y),
                DVec2::new(0.0, y),
            ];
            items.push(PlanItem::Fill {
                id: view,
                points: patch.clone(),
                hatch: Hatch::Solid,
                rgb: Some(rgb),
            });
            for i in 0..4 {
                items.push(PlanItem::Line {
                    id: view,
                    pen: Pen::Symbol,
                    a: patch[i],
                    b: patch[(i + 1) % 4],
                });
            }
            text(
                &mut items,
                DVec2::new(ind, y - PATCH.y / 2.0),
                &tr(words),
                TEXT,
            );
            y -= PATCH.y + 3.0;
        }
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
    // (George 2026-11-20: the project's zero and the ground at the
    // building are not the legend's; the title block carries the zero)
    let _ = y;
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

#[cfg(test)]
mod thermal_sound {
    use super::*;
    use creabim_core::Transaction;

    /// An outer wall's materials say their λ; a wall between dwellings
    /// its Rw, as its build-up's name gives it.
    #[test]
    fn outer_walls_give_lambda_and_walls_between_dwellings_rw() {
        let mut m = creabim_core::starter::starter_house();
        // the starter's materials know no λ: give them one
        let mut tx = Transaction::new("λ");
        for e in m.iter() {
            if let ElementKind::Material(x) = &e.kind {
                let mut now = e.clone();
                let mut x = x.clone();
                x.lambda = Some(0.45);
                now.kind = ElementKind::Material(x);
                tx = tx.replace(e.clone(), now);
            }
        }
        tx.apply(&mut m).unwrap();
        let before = rows(&m);
        assert!(
            before.iter().any(|r| r.text.contains("λ = 0,450 W/(m·K)")),
            "{:?}",
            before.iter().map(|r| &r.text).collect::<Vec<_>>()
        );
        // an inner wall's build-up named as a wall between flats
        let outer: Vec<ElementId> = creabim_core::levels::sorted(&m)
            .into_iter()
            .flat_map(|(l, ..)| crate::sitefoot::outer_walls(&m, l))
            .collect();
        let inner = m
            .iter()
            .find(|e| {
                matches!(e.kind, ElementKind::Wall(_))
                    && !outer.contains(&e.id)
                    && creabim_core::assembly_of(&m, e).is_some()
            })
            .expect("an inner wall with a build-up");
        let (a, _) = creabim_core::assembly_of(&m, inner).unwrap();
        let mut now = a.clone();
        now.name = "Medzibytová – PORFIX AKU 248 (Rw 56 dB)".into();
        Transaction::new("t")
            .replace(a.clone(), now)
            .apply(&mut m)
            .unwrap();
        let after = rows(&m);
        assert!(
            after.iter().any(|r| r.text.contains("Rw = 56 dB")),
            "{:?}",
            after.iter().map(|r| &r.text).collect::<Vec<_>>()
        );
        assert_eq!(rw_in("Medzibytová (Rw 56 dB)"), Some(56));
        assert_eq!(rw_in("Rw = 52 dB"), Some(52));
    }
}

#[cfg(test)]
mod plain {
    use super::*;

    #[test]
    fn a_legend_leaves_out_where_the_data_come_from() {
        assert_eq!(
            plain_description(
                "PORFIX; obvodové / nosné murivo; tvárnica 500 × 250 × 300 mm; Rw 47 dB; REI-M 240; \
                 λ10,dry 0.095 / λU 0.103 W/(m·K); PORFIX Technické listy 2025"
            ),
            "PORFIX; obvodové / nosné murivo; tvárnica 500 × 250 × 300 mm; Rw 47 dB; REI-M 240"
        );
        assert_eq!(
            plain_description(
                "PORFIX; nenosné murivo (priečky); Rw 39 dB; EI 180; λ 0.13 W/(m·K); \
                 PORFIX Technické listy 2025; λ orientačne (v liste neuvedená); \
                 požiarna odolnosť podľa: Odborná príručka PORFIX, s. 7; tab. 8 Odborná príručka PORFIX"
            ),
            "PORFIX; nenosné murivo (priečky); Rw 39 dB; EI 180"
        );
        assert_eq!(
            plain_description("Minerálna vlna - fasádna doska"),
            "Minerálna vlna - fasádna doska"
        );
    }
}

#[cfg(test)]
mod by_storey {
    use super::*;

    /// A storey's legend lists only what its plan cuts: not the
    /// foundations' materials below it, nor what a slab or beam is of.
    #[test]
    fn a_storeys_legend_lists_only_what_its_plan_cuts() {
        let m = creabim_core::starter::starter_house();
        let all = rows(&m);
        let levels = creabim_core::levels::sorted(&m);
        let mut union: Vec<(ElementId, Option<i64>)> = vec![];
        for (l, name, _) in &levels {
            let own = rows_of(&m, Some(*l), &[]);
            for r in &own {
                // every row is one of the whole legend's and has a thickness
                // (walls) or comes from a column
                assert!(all.iter().any(|a| a.material == r.material), "{name}");
                union.push((r.material, r.thickness_mm));
            }
        }
        // what only slabs are built of (the floor's insulation, its
        // waterproofing) is in no storey's legend, only the sections'
        let walls: Vec<(ElementId, Option<i64>)> = m
            .iter()
            .flat_map(|e| match (&e.kind, creabim_core::assembly_of(&m, e)) {
                (ElementKind::Wall(_), Some((_, a))) => a
                    .layers
                    .iter()
                    .map(|l| (l.material, l.thickness))
                    .collect::<Vec<_>>(),
                (ElementKind::Wall(w), None) => vec![(w.material, w.thickness)],
                _ => vec![],
            })
            .filter_map(|(mat, t)| Some((mat?, Some((t * 1000.0).round() as i64))))
            .collect();
        let slab_only: Vec<&Row> = all
            .iter()
            .filter(|r| r.thickness_mm.is_some() && !walls.contains(&(r.material, r.thickness_mm)))
            .collect();
        assert!(
            !slab_only.is_empty(),
            "the starter's floors add rows to the sections' legend"
        );
        for r in slab_only {
            assert!(!union.contains(&(r.material, r.thickness_mm)), "{}", r.text);
        }
        // some storey lists less than the whole
        assert!(levels
            .iter()
            .any(|(l, ..)| rows_of(&m, Some(*l), &[]).len() < all.len()));
        // and a material left out goes
        let first = all[0].material;
        assert!(rows_of(&m, None, &[first])
            .iter()
            .all(|r| r.material != first));
    }
}

#[cfg(test)]
mod renovation {
    use super::*;
    use creabim_core::reno::Reno;
    use creabim_core::Transaction;

    /// A renovation's legend says what black, yellow and red mean, for
    /// the states the storey has; an ordinary project's says nothing.
    #[test]
    fn a_renovation_legend_explains_its_colours() {
        let mut m = creabim_core::starter::starter_house();
        assert!(reno_states(&m, None).is_empty());
        let walls: Vec<_> = m
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Wall(_)))
            .cloned()
            .collect();
        let mut tx = Transaction::new("reno");
        for (i, w) in walls.iter().enumerate() {
            let mut now = w.clone();
            now.reno = Some(match i % 3 {
                0 => Reno::Existing,
                1 => Reno::Demolished,
                _ => Reno::New,
            });
            tx = tx.replace(w.clone(), now);
        }
        tx.apply(&mut m).unwrap();
        assert_eq!(
            reno_states(&m, None),
            [Reno::Existing, Reno::Demolished, Reno::New]
        );
        let v = creabim_core::Element::new(
            "Legend",
            ElementKind::View(creabim_core::View::schedule(
                creabim_core::ScheduleCategory::MaterialLegend,
            )),
        );
        let id = v.id;
        Transaction::new("t").insert(v).apply(&mut m).unwrap();
        let d = crate::section::view_drawing(&m, id).unwrap();
        let has = |t: &str| {
            d.items
                .iter()
                .any(|i| matches!(i, PlanItem::Text { text, .. } if text == t))
        };
        assert!(has("Demolished constructions and elements"));
        assert!(has("Proposed constructions and elements"));
        assert!(d.items.iter().any(|i| matches!(i,
            PlanItem::Fill { rgb: Some(c), .. } if *c == creabim_core::reno::DEMOLISHED)));
        // placed beside a plan of the demolition: black and yellow only
        let level = creabim_core::levels::sorted(&m)
            .into_iter()
            .find(|(l, ..)| reno_states(&m, Some(*l)).len() == 3)
            .unwrap()
            .0;
        let mut plan = creabim_core::View::plan(level);
        plan.reno = Some(creabim_core::reno::RenoView::Demolition);
        let plan = creabim_core::Element::new("Búracie práce", ElementKind::View(plan));
        let sheet = creabim_core::Element::new(
            "Sheet",
            ElementKind::Sheet(creabim_core::Sheet::a("S01_ASR_009_N01", "A3")),
        );
        let vp = |view: ElementId| {
            creabim_core::Element::new(
                "Viewport",
                ElementKind::Viewport(crate::sheet::new_viewport(&m, sheet.id, view, DVec2::ZERO)),
            )
        };
        let tx = Transaction::new("t").insert(vp(plan.id)).insert(vp(id));
        Transaction::new("t")
            .insert(sheet.clone())
            .insert(plan.clone())
            .apply(&mut m)
            .unwrap();
        tx.apply(&mut m).unwrap();
        assert_eq!(reno_of_view(&m, id), [Reno::Existing, Reno::Demolished]);
        assert_eq!(storeys(&m, id), Some(vec![level]));
    }
}
