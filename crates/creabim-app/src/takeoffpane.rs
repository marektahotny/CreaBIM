//! The material take-off as a spreadsheet in the app: every material of
//! every element on its own row (a layered wall's plaster, masonry,
//! insulation and render each), grouped by element type, level,
//! construction or material as picked, sorted by a click on a column's
//! heading, groups folded and unfolded, rows picked and shown where they
//! are in 3D or in the plan. Volumes come from the solids as trimmed by
//! joins and cuts ([`creabim_geom::takeoff`]).

use crate::selection::Selection;
use creabim_core::takeoff::{GroupBy, Layout, Rate, RateUnit};
use creabim_core::{Document, ElementId, ElementKind};
use creabim_geom::schedule::Field;
use creabim_geom::takeoff::Table;
use eframe::egui::{self, Color32, Rect};
use std::collections::HashSet;

const PAPER: Color32 = Color32::from_rgb(252, 252, 250);
const INK: Color32 = Color32::from_rgb(31, 37, 43);
const MUTED: Color32 = Color32::from_rgb(110, 116, 122);
const HEADER: Color32 = Color32::from_rgb(232, 235, 231);
const GROUP: [Color32; 3] = [
    Color32::from_rgb(222, 229, 236),
    Color32::from_rgb(234, 238, 242),
    Color32::from_rgb(242, 244, 246),
];
const STRIPE: Color32 = Color32::from_rgb(247, 248, 246);
const SELECTED: Color32 = Color32::from_rgb(206, 222, 244);
const GRID: Color32 = Color32::from_rgb(214, 217, 212);
const ACCENT: Color32 = Color32::from_rgb(200, 100, 20);
const ROW_H: f32 = 24.0;
const PAD: f32 = 8.0;
const INDENT: f32 = 16.0;

/// Where the rows picked in the take-off are to be shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShowIn {
    Model,
    Plan,
}

/// What the take-off asks of the app after it is drawn: to show these
/// elements in 3D or in the plan.
#[derive(Clone, Debug)]
pub struct ShowRequest {
    pub target: ShowIn,
    pub elements: Vec<ElementId>,
}

/// The take-off's own state between frames (not saved with the project).
#[derive(Clone, Debug, Default)]
struct PaneState {
    /// The picked rows, by key.
    picked: Vec<String>,
    /// Folded groups, by path.
    folded: HashSet<String>,
    filter: String,
}

fn request_id() -> egui::Id {
    egui::Id::new("takeoff_show_request")
}

fn stale_id() -> egui::Id {
    egui::Id::new("takeoff_stale")
}

/// The request the take-off made this frame, if any (taken).
pub fn take_request(ctx: &egui::Context) -> Option<ShowRequest> {
    ctx.data_mut(|d| d.remove_temp::<Option<ShowRequest>>(request_id()))
        .flatten()
}

/// Did the take-off show quantities measured on an older 3D scene, so the
/// scene of the model as it is should be built?
pub fn wants_scene(ctx: &egui::Context) -> bool {
    ctx.data_mut(|d| d.remove_temp::<bool>(stale_id()).unwrap_or(false))
}

fn paper_visuals() -> egui::Visuals {
    let mut v = egui::Visuals::light();
    v.override_text_color = Some(INK);
    v.extreme_bg_color = Color32::WHITE;
    v.widgets.inactive.weak_bg_fill = Color32::from_rgb(240, 241, 238);
    v.widgets.inactive.bg_fill = Color32::from_rgb(240, 241, 238);
    v
}

/// A row's key, steady while the model changes around it.
fn row_key(t: &Table, i: usize) -> String {
    let r = &t.schedule.rows[i];
    if r.group {
        return format!("G{}", t.path[i]);
    }
    format!(
        "{}#{}#{}#{:?}",
        t.path[i],
        r.cells[0].trim(),
        r.cells.get(1).map_or("", |s| s.as_str()),
        // an element's own row
        (t.elements[i].len() == 1).then(|| t.elements[i][0])
    )
}

/// Is row `i` inside a folded group?
fn hidden(t: &Table, i: usize, folded: &HashSet<String>) -> bool {
    if folded.is_empty() {
        return false;
    }
    let p = &t.path[i];
    let group = t.schedule.rows[i].group;
    folded.iter().any(|f| {
        let under = p.starts_with(&format!("{f}/"));
        if group {
            under
        } else {
            under || p == f
        }
    })
}

/// Set a material's rate.
fn set_rate(e: &mut creabim_core::Element, rate: Option<Rate>) {
    if let ElementKind::Material(m) = &mut e.kind {
        m.rate = rate;
    }
}

/// Show the take-off `view` in `rect`. Returns a status message when an
/// edit fails.
pub fn show(
    ui: &mut egui::Ui,
    rect: Rect,
    doc: &mut Document,
    view: ElementId,
    by_type: bool,
    selection: &mut Selection,
) -> Option<String> {
    let t = crate::i18n::tr;
    let state_id = egui::Id::new(("takeoff_pane", view));
    let mut state: PaneState = ui
        .ctx()
        .data_mut(|d| d.get_temp::<PaneState>(state_id))
        .unwrap_or_default();
    let layout: Layout = match doc.model().get(view).map(|e| &e.kind) {
        Some(ElementKind::View(v)) => v.takeoff.clone(),
        _ => Layout::default(),
    };
    let groups = layout.groups(by_type);
    let table = creabim_geom::takeoff::table(doc.model(), &layout, by_type, &state.filter);
    if !table.measured {
        ui.ctx().data_mut(|d| d.insert_temp(stale_id(), true));
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(200));
    }
    let mut message = None;
    let mut new_layout = layout.clone();
    let mut rate_edit: Option<(ElementId, Option<Rate>)> = None;
    let mut request: Option<ShowIn> = None;

    ui.painter().rect_filled(rect, 0.0, PAPER);
    let mut child =
        ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink2(egui::vec2(16.0, 10.0))));
    let ui = &mut child;
    ui.style_mut().visuals = paper_visuals();
    ui.spacing_mut().scroll.fade.strength = 0.0;
    ui.add_space(8.0);

    // the title and what the numbers are
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(t("Material take-off"))
                .size(17.0)
                .strong()
                .color(INK),
        );
        ui.add_space(12.0);
        let note = if table.measured {
            t("volumes measured on the trimmed 3D solids, after joins and cuts")
        } else {
            t("measuring the trimmed 3D solids…")
        };
        ui.label(egui::RichText::new(note).color(MUTED));
    });
    ui.add_space(6.0);

    // the bar: groups, detail, search, show in 3D / plan
    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new(t("Group by")).color(MUTED));
        let mut picked = groups.clone();
        let mut changed = false;
        for slot in 0..3 {
            if slot > picked.len() {
                break;
            }
            let now = picked.get(slot).copied();
            let mut chosen = now;
            egui::ComboBox::from_id_salt(("takeoff_group", view, slot))
                .icon(crate::props::combo_icon)
                .selected_text(match now {
                    Some(g) => t(g.label()),
                    None => t("—"),
                })
                .width(118.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut chosen, None, t("—"));
                    for g in GroupBy::ALL {
                        // each grouping once
                        if picked.iter().enumerate().any(|(i, p)| *p == g && i != slot) {
                            continue;
                        }
                        ui.selectable_value(&mut chosen, Some(g), t(g.label()));
                    }
                });
            if chosen != now {
                match chosen {
                    Some(g) if slot < picked.len() => picked[slot] = g,
                    Some(g) => picked.push(g),
                    None => picked.truncate(slot),
                }
                changed = true;
            }
            if slot < picked.len() && slot < 2 {
                ui.label(egui::RichText::new("›").color(MUTED));
            }
        }
        if changed {
            new_layout.group = Some(picked);
        }
        ui.separator();
        let mut detail = layout.detail;
        if ui
            .checkbox(&mut detail, t("Each element on its own row"))
            .on_hover_text(t(
                "List every layer of every element on a row of its own, instead of each material added up",
            ))
            .changed()
        {
            new_layout.detail = detail;
        }
        ui.separator();
        ui.label(egui::RichText::new(t("Find")).color(MUTED));
        ui.add(
            egui::TextEdit::singleline(&mut state.filter)
                .desired_width(130.0)
                .hint_text(t("material, element…")),
        );
        ui.separator();
        if ui.button(t("Fold all")).clicked() {
            state.folded = (0..table.schedule.rows.len())
                .filter(|i| table.schedule.rows[*i].group)
                .map(|i| table.path[i].clone())
                .collect();
        }
        if ui.button(t("Unfold all")).clicked() {
            state.folded.clear();
        }
        ui.separator();
        let any = !state.picked.is_empty();
        if ui
            .add_enabled(any, egui::Button::new(t("Show in 3D")))
            .on_hover_text(t("Select what the picked rows are made of and frame it in the 3D view"))
            .on_disabled_hover_text(t("Pick a row of the table first"))
            .clicked()
        {
            request = Some(ShowIn::Model);
        }
        if ui
            .add_enabled(any, egui::Button::new(t("Show in plan")))
            .on_hover_text(t("Select what the picked rows are made of and frame it in its floor plan"))
            .on_disabled_hover_text(t("Pick a row of the table first"))
            .clicked()
        {
            request = Some(ShowIn::Plan);
        }
    });
    ui.add_space(8.0);

    // column widths: each as wide as its widest cell
    let font = egui::TextStyle::Body.resolve(ui.style());
    let measure = |ui: &egui::Ui, s: &str| {
        ui.fonts_mut(|f| f.layout_no_wrap(s.to_string(), font.clone(), INK).size().x)
    };
    let cols = &table.schedule.columns;
    let rows = &table.schedule.rows;
    let widths: Vec<f32> = cols
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let head = measure(ui, &t(c.title).to_uppercase()) + 16.0;
            let cells = rows
                .iter()
                .enumerate()
                .map(|(n, r)| {
                    let extra = if i == 0 {
                        table.depth[n] as f32 * INDENT + if r.group { 16.0 } else { 0.0 }
                    } else {
                        0.0
                    };
                    measure(ui, &t(r.cells[i].trim_start())) + extra
                })
                .chain(table.schedule.totals.get(i).map(|s| measure(ui, &t(s))))
                .fold(head, f32::max);
            let editor = match c.field {
                Some(Field::Rate) => 64.0,
                Some(Field::RatePer) => 86.0,
                _ => 0.0,
            };
            cells.max(editor).min(if i == 0 { 420.0 } else { 260.0 }) + 2.0 * PAD
        })
        .collect();
    let total: f32 = widths.iter().sum();
    let cells_of = |r: Rect| {
        let mut x = r.left();
        widths
            .iter()
            .map(|w| {
                let c = Rect::from_min_size(egui::pos2(x, r.top()), egui::vec2(*w, r.height()));
                x += w;
                c
            })
            .collect::<Vec<_>>()
    };
    let text_at = |ui: &egui::Ui, c: Rect, s: &str, numeric: bool, color: Color32, indent: f32| {
        let s = t(s.trim_start());
        let (anchor, at) = if numeric {
            (
                egui::Align2::RIGHT_CENTER,
                c.right_center() - egui::vec2(PAD, 0.0),
            )
        } else {
            (
                egui::Align2::LEFT_CENTER,
                c.left_center() + egui::vec2(PAD + indent, 0.0),
            )
        };
        ui.painter()
            .with_clip_rect(c.shrink2(egui::vec2(2.0, 0.0)))
            .text(at, anchor, s, font.clone(), color);
    };
    let grid = |ui: &egui::Ui, r: Rect| {
        let p = ui.painter();
        let stroke = egui::Stroke::new(1.0, GRID);
        p.hline(r.x_range(), r.bottom(), stroke);
        let mut x = r.left();
        for w in &widths {
            x += w;
            p.vline(x, r.y_range(), stroke);
        }
        p.vline(r.left(), r.y_range(), stroke);
    };
    let keys: Vec<String> = (0..rows.len()).map(|i| row_key(&table, i)).collect();
    let modifiers = ui.input(|i| i.modifiers);
    let mut clicked: Option<(usize, bool)> = None;
    let mut fold: Option<String> = None;
    let mut sort_click: Option<&'static str> = None;

    egui::ScrollArea::horizontal()
        .id_salt(("takeoff_h", view))
        .auto_shrink(false)
        .show(ui, |ui| {
            ui.set_min_width(total + 2.0);
            // the heading stays while the rows scroll under it
            let (h, _) = ui.allocate_exact_size(egui::vec2(total, ROW_H + 4.0), egui::Sense::hover());
            ui.painter().rect_filled(h, 0.0, HEADER);
            for (i, (c, col)) in cells_of(h).into_iter().zip(cols).enumerate() {
                let resp = ui.interact(c, egui::Id::new(("takeoff_head", view, i)), egui::Sense::click());
                let arrow = match (&layout.sort, layout.descending) {
                    (Some(s), false) if s == col.title => " ▲",
                    (Some(s), true) if s == col.title => " ▼",
                    _ => "",
                };
                let color = if resp.hovered() { ACCENT } else { INK };
                text_at(
                    ui,
                    c,
                    &format!("{}{arrow}", t(col.title).to_uppercase()),
                    col.numeric,
                    color,
                    0.0,
                );
                if resp
                    .on_hover_text(t("Click to sort by this column; again to turn the order round, a third time for none"))
                    .clicked()
                {
                    sort_click = Some(col.title);
                }
            }
            grid(ui, h);
            egui::ScrollArea::vertical()
                .id_salt(("takeoff_v", view))
                .auto_shrink(false)
                .show(ui, |ui| {
                    let mut stripe = false;
                    for (n, r) in rows.iter().enumerate() {
                        if hidden(&table, n, &state.folded) {
                            continue;
                        }
                        let picked = state.picked.contains(&keys[n]);
                        let depth = table.depth[n];
                        let fill = if picked {
                            SELECTED
                        } else if r.group {
                            GROUP[depth.min(2)]
                        } else if stripe {
                            STRIPE
                        } else {
                            PAPER
                        };
                        if !r.group {
                            stripe = !stripe;
                        }
                        let (rr, resp) = ui.allocate_exact_size(
                            egui::vec2(total, ROW_H),
                            egui::Sense::click(),
                        );
                        if !ui.is_rect_visible(rr) {
                            continue;
                        }
                        ui.painter().rect_filled(rr, 0.0, fill);
                        if picked {
                            ui.painter().rect_filled(
                                Rect::from_min_size(rr.min, egui::vec2(3.0, rr.height())),
                                0.0,
                                crate::theme::interaction(),
                            );
                        }
                        let cs = cells_of(rr);
                        // a group's triangle folds it
                        let indent = depth as f32 * INDENT;
                        let mut first_indent = indent;
                        if r.group {
                            let tri = Rect::from_center_size(
                                egui::pos2(cs[0].left() + PAD + indent + 6.0, rr.center().y),
                                egui::vec2(14.0, 14.0),
                            );
                            let folded = state.folded.contains(&table.path[n]);
                            let tr_resp = ui.interact(
                                tri.expand(3.0),
                                egui::Id::new(("takeoff_fold", view, n)),
                                egui::Sense::click(),
                            );
                            {
                                // drawn, not a glyph: the fonts lack the triangles
                                let c = tri.center();
                                let k = 4.5;
                                let pts = if folded {
                                    vec![
                                        c + egui::vec2(-k * 0.6, -k),
                                        c + egui::vec2(k * 0.9, 0.0),
                                        c + egui::vec2(-k * 0.6, k),
                                    ]
                                } else {
                                    vec![
                                        c + egui::vec2(-k, -k * 0.6),
                                        c + egui::vec2(k, -k * 0.6),
                                        c + egui::vec2(0.0, k * 0.9),
                                    ]
                                };
                                let ink = if tr_resp.hovered() { ACCENT } else { INK };
                                ui.painter().add(egui::Shape::convex_polygon(
                                    pts,
                                    ink,
                                    egui::Stroke::NONE,
                                ));
                            }
                            if tr_resp.clicked() {
                                fold = Some(table.path[n].clone());
                            }
                            first_indent += 16.0;
                        }
                        let strong = r.group;
                        for (i, (c, col)) in cs.iter().zip(cols).enumerate() {
                            let mat = table.material[n].filter(|_| !r.group);
                            match (col.field, mat) {
                                (Some(Field::Rate), Some(m)) => {
                                    let rate = creabim_core::takeoff::rate_of(doc.model(), m);
                                    let per = rate.map_or(RateUnit::Area, |r| r.per);
                                    let mut amount = rate.map_or(0.0, |r| r.amount);
                                    let inner = c.shrink2(egui::vec2(4.0, 2.0));
                                    let resp = ui.put(
                                        inner,
                                        egui::DragValue::new(&mut amount)
                                            .range(0.0..=1.0e9)
                                            .speed(0.5)
                                            .max_decimals(2),
                                    );
                                    if resp
                                        .on_hover_text(t("Its price, in the currency you price the project in"))
                                        .changed()
                                    {
                                        rate_edit = Some((m, Some(Rate { amount, per })));
                                    }
                                }
                                (Some(Field::RatePer), Some(m)) => {
                                    let rate = creabim_core::takeoff::rate_of(doc.model(), m);
                                    let per = rate.map_or(RateUnit::Area, |r| r.per);
                                    let mut chosen = per;
                                    let inner = c.shrink2(egui::vec2(4.0, 2.0));
                                    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
                                        egui::ComboBox::from_id_salt(("takeoff_per", view, n))
                                            .icon(crate::props::combo_icon)
                                            .selected_text(t(per.label()))
                                            .width(inner.width())
                                            .show_ui(ui, |ui| {
                                                for u in RateUnit::ALL {
                                                    ui.selectable_value(&mut chosen, u, t(u.label()));
                                                }
                                            });
                                    });
                                    if chosen != per {
                                        let amount = rate.map_or(0.0, |r| r.amount);
                                        rate_edit = Some((m, Some(Rate { amount, per: chosen })));
                                    }
                                }
                                _ => {
                                    let color = if strong || !r.cells[i].trim().is_empty() {
                                        INK
                                    } else {
                                        MUTED
                                    };
                                    let ind = if i == 0 { first_indent } else { 0.0 };
                                    text_at(ui, *c, &r.cells[i], col.numeric, color, ind);
                                    if strong {
                                        // bold by a second pass a hair to the right
                                        let c2 = c.translate(egui::vec2(0.6, 0.0));
                                        text_at(ui, c2, &r.cells[i], col.numeric, color, ind);
                                    }
                                }
                            }
                        }
                        grid(ui, rr);
                        if resp.double_clicked() {
                            clicked = Some((n, false));
                            request = Some(ShowIn::Model);
                        } else if resp.clicked() {
                            clicked = Some((n, modifiers.command || modifiers.shift));
                        }
                        resp.on_hover_text(t(
                            "Click to pick the row (Ctrl to add more), double-click to show it in 3D",
                        ));
                    }
                    // totals
                    let (tr, _) = ui.allocate_exact_size(egui::vec2(total, ROW_H + 2.0), egui::Sense::hover());
                    ui.painter().rect_filled(tr, 0.0, HEADER);
                    for ((c, col), s) in cells_of(tr).into_iter().zip(cols).zip(&table.schedule.totals) {
                        text_at(ui, c, s, col.numeric, INK, 0.0);
                    }
                    grid(ui, tr);
                    ui.add_space(12.0);
                });
        });

    // what was clicked
    if let Some(f) = fold {
        if !state.folded.remove(&f) {
            state.folded.insert(f);
        }
    } else if let Some((n, add)) = clicked {
        let k = keys[n].clone();
        if add {
            if let Some(i) = state.picked.iter().position(|p| *p == k) {
                state.picked.remove(i);
            } else {
                state.picked.push(k);
            }
        } else {
            state.picked = vec![k];
        }
        // the model's selection follows the picked rows
        let elements = picked_elements(&table, &keys, &state.picked);
        selection.clear();
        selection.extend(elements, false);
    }
    if let Some(title) = sort_click {
        let (sort, desc) = match (&layout.sort, layout.descending) {
            (Some(s), false) if s == title => (Some(title.to_string()), true),
            (Some(s), true) if s == title => (None, false),
            _ => (Some(title.to_string()), false),
        };
        new_layout.sort = sort;
        new_layout.descending = desc;
    }
    if let Some(target) = request {
        let elements = picked_elements(&table, &keys, &state.picked);
        if !elements.is_empty() {
            selection.clear();
            selection.extend(elements.iter().copied(), false);
            ui.ctx()
                .data_mut(|d| d.insert_temp(request_id(), Some(ShowRequest { target, elements })));
        }
    }
    // forget picks of rows no longer there
    state.picked.retain(|p| keys.contains(p));
    ui.ctx().data_mut(|d| d.insert_temp(state_id, state));

    if let Some((m, rate)) = rate_edit {
        if let Err(e) = doc.edit_merged(m, "Edit rate in take-off", move |e| set_rate(e, rate)) {
            message = Some(format!("Couldn't do that: {e}"));
        }
    } else if new_layout != layout {
        let r = doc.edit_merged(view, "Take-off layout", move |e| {
            if let ElementKind::View(v) = &mut e.kind {
                v.takeoff = new_layout;
            }
        });
        if let Err(e) = r {
            message = Some(format!("Couldn't do that: {e}"));
        }
    }
    message
}

/// The elements the picked rows stand for, each once.
fn picked_elements(table: &Table, keys: &[String], picked: &[String]) -> Vec<ElementId> {
    let mut out: Vec<ElementId> = vec![];
    for (i, k) in keys.iter().enumerate() {
        if picked.contains(k) {
            for e in &table.elements[i] {
                if !out.contains(e) {
                    out.push(*e);
                }
            }
        }
    }
    out
}
