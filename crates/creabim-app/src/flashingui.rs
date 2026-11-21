//! Gutters and downpipes (klampiarske výrobky): the Gutter tool, traced
//! along a roof's eaves; the Downpipe tool, clicked on a gutter; and
//! their properties. What drains to each downpipe and the flow from it
//! are shown on the downpipe.

use crate::planview::PlanState;
use creabim_core::flashing::{
    self, DownpipeProfile, Flashing, FlashingItem, GutterProfile, MATERIALS,
};
use creabim_core::{Document, Element, ElementId, ElementKind, Model, Transaction};
use eframe::egui;
use glam::DVec2;

/// What the Gutter and Downpipe tools draw next.
#[derive(Clone, Debug, PartialEq)]
pub struct FlashingDraft {
    pub gutter_profile: GutterProfile,
    pub gutter_mm: f64,
    pub plan_offset: f64,
    pub height_offset: f64,
    pub fall: f64,
    pub pipe_profile: DownpipeProfile,
    pub pipe_mm: f64,
    /// The trace clicked so far, and its roof.
    pub trace: Vec<DVec2>,
    pub roof: Option<ElementId>,
}

impl Default for FlashingDraft {
    fn default() -> Self {
        Self {
            gutter_profile: GutterProfile::HalfRound,
            gutter_mm: 140.0,
            plan_offset: 0.0,
            height_offset: 0.0,
            fall: 3.0,
            pipe_profile: DownpipeProfile::Round,
            pipe_mm: 100.0,
            trace: vec![],
            roof: None,
        }
    }
}

/// How far a click may be from an eave or a gutter to take it, m.
const REACH: f64 = 0.6;

/// Where a Gutter-tool click lands: on the nearest roof eave.
pub fn snap(model: &Model, p: DVec2) -> Option<(ElementId, DVec2)> {
    flashing::snap_eave(model, p, REACH)
}

/// A click of the Gutter tool: each click a point on the roof's eave;
/// clicking the last point again (or Enter) hangs the gutter.
pub fn gutter_click(
    doc: &mut Document,
    level: ElementId,
    p: DVec2,
    state: &mut PlanState,
    selection: &mut crate::selection::Selection,
) -> Result<(), creabim_core::ModelError> {
    let model = doc.model();
    let Some((roof, q)) = snap(model, p) else {
        state.status = Some("Click on the eave of a roof: the gutter follows it.".into());
        return Ok(());
    };
    let d = &mut state.members.flashing;
    if d.trace.last().is_some_and(|l| l.distance(q) < 0.02) {
        return finish_gutter(doc, level, state, selection);
    }
    if d.roof.is_some_and(|r| r != roof) && !d.trace.is_empty() {
        state.status = Some("A gutter follows one roof: click on the same roof's eave.".into());
        return Ok(());
    }
    d.roof = Some(roof);
    d.trace.push(q);
    state.pending = Some(q);
    state.status = Some(
        "Click the next corner of the eave; click the last point again (or Enter) to hang the gutter. Esc cancels."
            .into(),
    );
    Ok(())
}

/// Hang the gutter along the trace clicked so far.
pub fn finish_gutter(
    doc: &mut Document,
    level: ElementId,
    state: &mut PlanState,
    selection: &mut crate::selection::Selection,
) -> Result<(), creabim_core::ModelError> {
    let d = &mut state.members.flashing;
    let trace = std::mem::take(&mut d.trace);
    let roof = d.roof.take();
    state.pending = None;
    if trace.len() < 2 {
        return Ok(());
    }
    let level = roof
        .and_then(|r| creabim_core::members::level_of(&doc.model().get(r)?.kind))
        .unwrap_or(level);
    let mut f = Flashing::gutter(level, roof, trace);
    if let FlashingItem::Gutter(g) = &mut f.item {
        g.profile = d.gutter_profile;
        g.size_mm = d.gutter_mm;
        g.plan_offset = d.plan_offset;
        g.height_offset = d.height_offset;
        g.fall = d.fall;
    }
    let e = Element::new("Žľab", ElementKind::Flashing(f));
    selection.only(e.id);
    state.status = Some("Gutter hung. Downpipe tool: click on it to place its downpipes.".into());
    doc.commit(Transaction::new("Draw gutter").insert(e))
}

/// The gutter nearest `p` in plan, within reach, and the station there.
pub fn gutter_at(model: &Model, p: DVec2) -> Option<(ElementId, f64)> {
    model
        .iter()
        .filter_map(|e| {
            let ElementKind::Flashing(Flashing {
                item: FlashingItem::Gutter(g),
                ..
            }) = &e.kind
            else {
                return None;
            };
            let s = g.station_on_axis(model, p);
            let axis = g.axis_plan(model);
            let d = g.axis_at(&axis, s).distance(p);
            (d < REACH).then_some((e.id, s, d))
        })
        .min_by(|a, b| a.2.total_cmp(&b.2))
        .map(|(id, s, _)| (id, s))
}

/// A click of the Downpipe tool: a downpipe on the gutter clicked, where
/// it was clicked.
pub fn downpipe_click(
    doc: &mut Document,
    p: DVec2,
    state: &mut PlanState,
    selection: &mut crate::selection::Selection,
) -> Result<(), creabim_core::ModelError> {
    let model = doc.model();
    let Some((gutter, at)) = gutter_at(model, p) else {
        state.status = Some("Click on a gutter to put a downpipe on it.".into());
        return Ok(());
    };
    let Some(level) = model
        .get(gutter)
        .and_then(|e| creabim_core::members::level_of(&e.kind))
    else {
        return Ok(());
    };
    let d = &state.members.flashing;
    let mut f = Flashing::downpipe(level, gutter, at);
    if let FlashingItem::Downpipe(dp) = &mut f.item {
        dp.profile = d.pipe_profile;
        dp.size_mm = d.pipe_mm;
        // the foot at the lowest level's floor
        dp.bottom = lowest_level(model);
    }
    let e = Element::new("Zvod", ElementKind::Flashing(f));
    selection.only(e.id);
    state.status = Some(
        "Downpipe placed. Move it to slide it along its gutter; the gutter falls to its downpipes."
            .into(),
    );
    doc.commit(Transaction::new("Place downpipe").insert(e))
}

/// The height of the lowest level, where downpipes end by default.
fn lowest_level(model: &Model) -> f64 {
    let z = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Level(l) => Some(l.elevation),
            _ => None,
        })
        .fold(f64::INFINITY, f64::min);
    if z.is_finite() {
        z.min(0.0)
    } else {
        0.0
    }
}

fn mm(ui: &mut egui::Ui, label: &str, v: &mut f64, lo: f64, hi: f64) -> bool {
    crate::props::row(ui, label, |ui| {
        crate::props::fill(
            ui,
            egui::DragValue::new(v)
                .range(lo..=hi)
                .speed(1.0)
                .suffix(" mm"),
        )
        .changed()
    })
}

fn metres(ui: &mut egui::Ui, label: &str, v: &mut f64, lo: f64, hi: f64) -> bool {
    crate::props::row(ui, label, |ui| {
        crate::props::fill(ui, crate::units::drag(v).range(lo..=hi)).changed()
    })
}

/// The Properties rows of a gutter or downpipe. Returns the undo label
/// when something changed.
pub fn fields(ui: &mut egui::Ui, model: &Model, id: ElementId, f: &mut Flashing) -> &'static str {
    let mut label = "";
    match &mut f.item {
        FlashingItem::Gutter(g) => {
            if crate::props::choice(
                ui,
                "Profile",
                &mut g.profile,
                &[
                    (
                        GutterProfile::HalfRound,
                        "Half-round",
                        "A half-round gutter",
                    ),
                    (GutterProfile::Square, "Square", "A square (box) gutter"),
                ],
            ) {
                label = "Gutter profile";
            }
            if mm(ui, "Gutter size", &mut g.size_mm, 50.0, 400.0) {
                label = "Gutter size";
            }
            if metres(
                ui,
                "Offset from the eave in plan",
                &mut g.plan_offset,
                -1.0,
                2.0,
            ) {
                label = "Gutter offset";
            }
            if metres(
                ui,
                "Below the roof's top at the eave",
                &mut g.height_offset,
                -1.0,
                2.0,
            ) {
                label = "Gutter height";
            }
            if crate::props::row(ui, "Fall", |ui| {
                crate::props::fill(
                    ui,
                    egui::DragValue::new(&mut g.fall)
                        .range(0.0..=30.0)
                        .speed(0.1)
                        .suffix(" mm/m"),
                )
                .changed()
            }) {
                label = "Gutter fall";
            }
            if metres(ui, "Hook spacing", &mut g.hook_spacing, 0.2, 2.0) {
                label = "Hook spacing";
            }
            let outs = flashing::outlets(model, id);
            let len = g.length();
            crate::props::note(
                ui,
                format!(
                    "{} m, {} {}",
                    flashing::num(len),
                    outs.len(),
                    crate::i18n::tr("downpipes")
                ),
            );
            if outs.is_empty() {
                crate::props::note(
                    ui,
                    "No downpipe yet: the gutter is level. Downpipe tool: click on it.",
                );
            } else {
                let fall = outs.iter().fold(0.0f64, |m, o| m.max(g.drop_at(&outs, *o)));
                crate::props::note(
                    ui,
                    format!(
                        "{} {} mm",
                        crate::i18n::tr("Greatest drop to a downpipe:"),
                        flashing::num(fall * 1000.0)
                    ),
                );
            }
            if crate::props::row(ui, "Rain intensity r", |ui| {
                crate::props::fill(
                    ui,
                    egui::DragValue::new(&mut g.rain)
                        .range(0.0..=0.2)
                        .speed(0.001)
                        .max_decimals(3)
                        .suffix(" l/(s·m²)"),
                )
                .changed()
            }) {
                label = "Rain intensity";
            }
            if crate::props::row(ui, "Runoff coefficient C", |ui| {
                crate::props::fill(
                    ui,
                    egui::DragValue::new(&mut g.runoff)
                        .range(0.0..=1.0)
                        .speed(0.01)
                        .max_decimals(2),
                )
                .changed()
            }) {
                label = "Runoff coefficient";
            }
            if crate::props::row(ui, "Sieve at the outlets", |ui| {
                ui.checkbox(&mut g.sieve, "").changed()
            }) {
                label = "Sieve at the outlets";
            }
            // what drains to each downpipe, and whether each run of
            // gutter to it carries it (STN EN 12056-3)
            for (pipe, area) in flashing::catchment(model, id) {
                let name = model.get(pipe).map_or(String::new(), |e| e.name.clone());
                crate::props::note(
                    ui,
                    format!(
                        "{name}: A = {} m², Q = {} l/s",
                        flashing::num(area),
                        format!("{:.2}", g.rain * area * g.runoff).replace('.', ",")
                    ),
                );
                for c in flashing::check_gutter(model, id)
                    .into_iter()
                    .filter(|c| c.pipe == pipe)
                {
                    run_check(ui, &c);
                }
            }
        }
        FlashingItem::Downpipe(d) => {
            if crate::props::choice(
                ui,
                "Profile",
                &mut d.profile,
                &[
                    (DownpipeProfile::Round, "Round", "A round downpipe"),
                    (DownpipeProfile::Square, "Square", "A square downpipe"),
                ],
            ) {
                label = "Downpipe profile";
            }
            if mm(ui, "Downpipe size", &mut d.size_mm, 40.0, 300.0) {
                label = "Downpipe size";
            }
            let len = flashing::gutter_of(model, d.gutter).map_or(0.0, |g| g.length());
            if metres(ui, "Along the gutter", &mut d.at, 0.0, len) {
                label = "Downpipe place";
            }
            if metres(ui, "Foot height", &mut d.bottom, -20.0, 200.0) {
                label = "Downpipe foot";
            }
            let mut n = d.elbows as f64;
            if crate::props::row(ui, "Elbows", |ui| {
                crate::props::fill(
                    ui,
                    egui::DragValue::new(&mut n)
                        .range(0.0..=10.0)
                        .speed(0.1)
                        .max_decimals(0),
                )
                .changed()
            }) {
                d.elbows = n.round() as u32;
                label = "Downpipe elbows";
            }
            if crate::props::row(ui, "Elbow angle", |ui| {
                crate::props::fill(
                    ui,
                    egui::DragValue::new(&mut d.elbow_angle)
                        .range(0.0..=90.0)
                        .speed(1.0)
                        .suffix("°"),
                )
                .changed()
            }) {
                label = "Elbow angle";
            }
            if metres(ui, "Clamp spacing", &mut d.clamp_spacing, 0.3, 5.0) {
                label = "Clamp spacing";
            }
            if crate::props::row(ui, "To the facade", |ui| {
                ui.checkbox(&mut d.to_wall, "").changed()
            }) {
                label = "Downpipe to the facade";
            }
            if d.to_wall {
                if metres(ui, "Gap to the facade", &mut d.facade_gap, 0.0, 0.5) {
                    label = "Gap to the facade";
                }
                let path = flashing::downpipe_path(model, d);
                let at = path.first().map(|p| p.truncate());
                let wall = path.get(2).map(|p| p.truncate());
                crate::props::note(
                    ui,
                    match (at, wall) {
                        (Some(a), Some(w)) => format!(
                            "{} {} mm",
                            crate::i18n::tr("Offset by its elbows to the facade:"),
                            flashing::num(a.distance(w) * 1000.0)
                        ),
                        _ => crate::i18n::tr(
                            "No wall found under the gutter: the downpipe runs straight down.",
                        ),
                    },
                );
            }
            crate::props::note(
                ui,
                format!(
                    "{} {} m",
                    crate::i18n::tr("Length:"),
                    flashing::num(flashing::downpipe_length(model, d))
                ),
            );
            if let Some(g) = flashing::gutter_of(model, d.gutter) {
                if let Some((_, area)) = flashing::catchment(model, d.gutter)
                    .into_iter()
                    .find(|c| c.0 == id)
                {
                    crate::props::note(
                        ui,
                        format!(
                            "{} A = {} m², Q = r·A·C = {} l/s",
                            crate::i18n::tr("Drains:"),
                            flashing::num(area),
                            format!("{:.2}", g.rain * area * g.runoff).replace('.', ",")
                        ),
                    );
                }
                for c in flashing::check_gutter(model, d.gutter)
                    .into_iter()
                    .filter(|c| c.pipe == id)
                {
                    run_check(ui, &c);
                }
            }
        }
    }
    // the sheet, as the table of sheet-metal items writes it
    let width = ui.available_width().min(220.0);
    crate::app::helpui::combo(ui, "Sheet")
        .selected_text(f.material.clone())
        .width(width)
        .show_ui(ui, |ui| {
            for m in MATERIALS {
                if ui.selectable_label(f.material == m, m).clicked() && f.material != m {
                    f.material = m.into();
                    label = "Sheet material";
                }
            }
        });
    if mm(ui, "Sheet thickness", &mut f.thickness_mm, 0.1, 5.0) {
        label = "Sheet thickness";
    }
    if crate::props::row(ui, "Finish", |ui| {
        crate::props::fill(ui, egui::TextEdit::singleline(&mut f.finish)).changed()
    }) {
        label = "Sheet finish";
    }
    // counted from the profile unless typed
    let mut dev = f.developed();
    if mm(ui, "Developed width", &mut dev, 10.0, 2000.0) {
        f.developed_mm = dev;
        f.developed_set = true;
        label = "Developed width";
    }
    if f.developed_set {
        if crate::props::action(
            ui,
            "",
            "Count from the profile",
            "Count the developed width from the profile again",
            true,
        )
        .clicked()
        {
            f.developed_set = false;
            label = "Developed width";
        }
    } else {
        let mut edges = f.edges();
        if mm(ui, "Bead, edge and seam", &mut edges, 0.0, 500.0) {
            f.edges_mm = Some(edges);
            label = "Developed width";
        }
        crate::props::note(ui, "The developed width is the profile's length and this.");
    }
    let mut mark = f.mark.clone().unwrap_or_default();
    let auto = flashing::mark_of(model, id).unwrap_or_default();
    if crate::props::row(ui, "Mark", |ui| {
        crate::props::fill(ui, egui::TextEdit::singleline(&mut mark).hint_text(auto)).changed()
    }) {
        f.mark = (!mark.trim().is_empty()).then(|| mark.trim().to_string());
        label = "Sheet-metal mark";
    }
    if crate::props::row(ui, "Note", |ui| {
        crate::props::fill(ui, egui::TextEdit::singleline(&mut f.note)).changed()
    }) {
        label = "Sheet-metal note";
    }
    crate::props::note(ui, f.description());
    label
}

/// The tool bar of the Gutter and Downpipe tools.
pub fn tool_options(d: &FlashingDraft, tool: crate::planview::Tool) -> Vec<serde_json::Value> {
    use serde_json::json;
    let seg = |items: Vec<serde_json::Value>| json!({"kind": "seg", "items": items});
    let opt = |id: &str, label: &str, on: bool, hint: &str| json!({"id": format!("member:{id}"), "label": label, "on": on, "hint": hint});
    let num = |key: &str,
               label: &str,
               v: f64,
               unit: &str,
               step: f64,
               lo: f64,
               hi: f64,
               hint: &str| {
        json!({
            "kind": "number", "key": format!("member.flashing.{key}"), "label": label, "value": v,
            "unit": unit, "step": step, "min": lo, "max": hi, "hint": hint,
        })
    };
    if tool == crate::planview::Tool::Gutter {
        vec![
            seg(vec![
                opt(
                    "flashing.halfround",
                    "Half-round",
                    d.gutter_profile == GutterProfile::HalfRound,
                    "A half-round gutter",
                ),
                opt(
                    "flashing.squaregutter",
                    "Square",
                    d.gutter_profile == GutterProfile::Square,
                    "A square (box) gutter",
                ),
            ]),
            num(
                "gutter",
                "Size",
                d.gutter_mm,
                "mm",
                5.0,
                50.0,
                400.0,
                "Width of new gutters (a half-round's diameter)",
            ),
            num(
                "offset",
                "Offset",
                d.plan_offset,
                "m",
                0.005,
                -1.0,
                2.0,
                "From the roof's eave line out to the gutter's back edge, in plan",
            ),
            num(
                "below",
                "Below eave",
                d.height_offset,
                "m",
                0.005,
                -1.0,
                2.0,
                "From the roof's top at the eave down to the gutter's top edge at its high points",
            ),
            num(
                "fall",
                "Fall",
                d.fall,
                "mm/m",
                0.5,
                0.0,
                30.0,
                "How much new gutters fall towards their downpipes",
            ),
            json!({"kind": "sep"}),
        ]
    } else {
        vec![
            seg(vec![
                opt(
                    "flashing.roundpipe",
                    "Round",
                    d.pipe_profile == DownpipeProfile::Round,
                    "A round downpipe",
                ),
                opt(
                    "flashing.squarepipe",
                    "Square",
                    d.pipe_profile == DownpipeProfile::Square,
                    "A square downpipe",
                ),
            ]),
            num(
                "pipe",
                "Size",
                d.pipe_mm,
                "mm",
                5.0,
                40.0,
                300.0,
                "Diameter, or side, of new downpipes",
            ),
            json!({"kind": "sep"}),
        ]
    }
}

/// A number typed in the tool bar ("flashing.gutter"...).
pub fn set_option(d: &mut FlashingDraft, key: &str, v: f64) {
    match key.trim_start_matches("flashing.") {
        "gutter" => d.gutter_mm = v.clamp(50.0, 400.0),
        "offset" => d.plan_offset = v.clamp(-1.0, 2.0),
        "below" => d.height_offset = v.clamp(-1.0, 2.0),
        "fall" => d.fall = v.clamp(0.0, 30.0),
        "pipe" => d.pipe_mm = v.clamp(40.0, 300.0),
        _ => {}
    }
}

/// A toggle clicked in the tool bar.
pub fn toggle_option(d: &mut FlashingDraft, what: &str) {
    match what {
        "flashing.halfround" => d.gutter_profile = GutterProfile::HalfRound,
        "flashing.squaregutter" => d.gutter_profile = GutterProfile::Square,
        "flashing.roundpipe" => d.pipe_profile = DownpipeProfile::Round,
        "flashing.squarepipe" => d.pipe_profile = DownpipeProfile::Square,
        _ => {}
    }
}

/// One run of gutter checked, in green when it carries what comes down
/// it, in red when not.
fn run_check(ui: &mut egui::Ui, c: &flashing::RunCheck) {
    let n = |v: f64| format!("{v:.2}").replace('.', ",");
    let (text, ok) = match c.allowed {
        Some(a) => (
            format!(
                "  {} {} m: Q = {} l/s {} Qdov = {} l/s",
                crate::i18n::tr("run"),
                flashing::num(c.length),
                n(c.q),
                if c.ok() { "≤" } else { ">" },
                n(a)
            ),
            c.ok(),
        ),
        None => (
            format!(
                "  {} {} m: {}",
                crate::i18n::tr("run"),
                flashing::num(c.length),
                crate::i18n::tr("too long for the gutter (L/W ≥ 500)")
            ),
            false,
        ),
    };
    let colour = if ok {
        egui::Color32::from_rgb(40, 140, 60)
    } else {
        egui::Color32::from_rgb(200, 50, 40)
    };
    ui.label(egui::RichText::new(text).small().color(colour));
}
