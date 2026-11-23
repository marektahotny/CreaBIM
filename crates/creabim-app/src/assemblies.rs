//! The assembly editor: the project's layered wall, floor and roof
//! assemblies (add, duplicate, rename, delete, assign to the selection),
//! each a list of layers to add, remove, reorder and set: function,
//! material and thickness. The same layer table shows in Properties for an
//! assembly picked in the project browser. Every change is an ordinary
//! transaction (dragging a thickness is one undo step), and changing an
//! assembly changes the thickness of everything built of it.

use super::*;
use crate::ui_model::Item;
use creabim_core::assembly::{self, AssemblyUse, Layer, LayerFunction};
use creabim_core::Assembly;

/// The assembly editor's own state, for the session.
#[derive(Default)]
pub(super) struct AssembliesUi {
    pub open: bool,
    /// The assembly being edited.
    current: Option<ElementId>,
}

/// What a click in the editor asks for, done once it is drawn.
pub(super) enum AsmAsk {
    Pick(ElementId),
    New(AssemblyUse),
    Starters,
    Duplicate(ElementId),
    Delete(ElementId),
    Rename(ElementId, String),
    Edit(ElementId, &'static str, Assembly),
    SelectUsers(ElementId),
    Assign(Option<ElementId>),
    /// A library material a build-up is about to use.
    AddMaterial(Box<creabim_core::Element>),
}

/// The small drawn buttons of a layer row (the UI font has no arrows).
#[derive(Clone, Copy)]
enum Glyph {
    Up,
    Down,
    Remove,
}

struct GlyphButton(Glyph);

impl egui::Widget for GlyphButton {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
        let v = ui.style().interact(&resp);
        let p = ui.painter();
        p.rect(
            rect,
            3.0,
            v.weak_bg_fill,
            v.bg_stroke,
            egui::StrokeKind::Inside,
        );
        let c = rect.center();
        let color = if ui.is_enabled() {
            v.fg_stroke.color
        } else {
            ui.visuals().weak_text_color()
        };
        let stroke = egui::Stroke::new(1.5, color);
        let r = 4.0;
        match self.0 {
            Glyph::Up | Glyph::Down => {
                let s = if matches!(self.0, Glyph::Up) {
                    -1.0
                } else {
                    1.0
                };
                p.line_segment(
                    [
                        c + Vec2::new(-r, -s * r / 2.0),
                        c + Vec2::new(0.0, s * r / 2.0),
                    ],
                    stroke,
                );
                p.line_segment(
                    [
                        c + Vec2::new(0.0, s * r / 2.0),
                        c + Vec2::new(r, -s * r / 2.0),
                    ],
                    stroke,
                );
            }
            Glyph::Remove => {
                p.line_segment([c + Vec2::new(-r, -r), c + Vec2::new(r, r)], stroke);
                p.line_segment([c + Vec2::new(-r, r), c + Vec2::new(r, -r)], stroke);
            }
        }
        resp
    }
}

/// The colour a layer is drawn in: its material's, or one for its function.
fn layer_fill(model: &Model, l: &Layer) -> (Color32, creabim_core::Hatch) {
    let m = l.material.and_then(|m| match &model.get(m)?.kind {
        ElementKind::Material(m) => Some(m.clone()),
        _ => None,
    });
    match m {
        Some(m) => {
            let c = m.color.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8);
            (Color32::from_rgb(c[0], c[1], c[2]), m.hatch)
        }
        None => match l.function {
            LayerFunction::AirGap => (Color32::from_rgb(245, 247, 250), Hatch::Empty),
            LayerFunction::Insulation => (Color32::from_rgb(236, 220, 120), Hatch::Empty),
            _ => (Color32::from_gray(200), Hatch::Diagonal),
        },
    }
}

/// The assembly drawn as a cut strip: a wall's layers side by side
/// (exterior on the left), a floor's or roof's stacked (top up), each
/// in its material's colour with its cut hatch.
pub(super) fn strip(ui: &mut egui::Ui, model: &Model, a: &Assembly, width: f32, height: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter_at(rect);
    let total = a.total().max(1e-9);
    let vertical = a.usage == AssemblyUse::Wall;
    let ink = ui.visuals().text_color();
    let mut at = 0.0;
    for l in &a.layers {
        let k0 = (at / total) as f32;
        at += l.thickness;
        let k1 = (at / total) as f32;
        let r = if vertical {
            Rect::from_min_max(
                egui::pos2(rect.left() + rect.width() * k0, rect.top()),
                egui::pos2(rect.left() + rect.width() * k1, rect.bottom()),
            )
        } else {
            Rect::from_min_max(
                egui::pos2(rect.left(), rect.top() + rect.height() * k0),
                egui::pos2(rect.right(), rect.top() + rect.height() * k1),
            )
        };
        let (fill, hatch) = layer_fill(model, l);
        painter.rect_filled(r, 0.0, fill);
        let poly: Vec<DVec2> = [
            r.left_top(),
            r.right_top(),
            r.right_bottom(),
            r.left_bottom(),
        ]
        .iter()
        .map(|p| DVec2::new(p.x as f64, p.y as f64))
        .collect();
        let line = egui::Stroke::new(0.7, Color32::from_black_alpha(140));
        for (p, q) in creabim_geom::plan::hatch_lines(hatch, &poly, 6.0) {
            painter.line_segment(
                [
                    egui::pos2(p.x as f32, p.y as f32),
                    egui::pos2(q.x as f32, q.y as f32),
                ],
                line,
            );
        }
        painter.rect_stroke(
            r,
            0.0,
            egui::Stroke::new(0.8, ink),
            egui::StrokeKind::Inside,
        );
    }
    // the core, bracketed
    let (i, j) = a.core();
    let spans = a.spans(1.0);
    if let (Some(s0), Some(s1)) = (spans.get(i), spans.get(j)) {
        let (k0, k1) = (s0.0 as f32, s1.1 as f32);
        let accent = crate::theme::interaction();
        let stroke = egui::Stroke::new(1.5, accent);
        if vertical {
            let (x0, x1) = (
                rect.left() + rect.width() * k0,
                rect.left() + rect.width() * k1,
            );
            painter.line_segment(
                [
                    egui::pos2(x0, rect.bottom() - 2.0),
                    egui::pos2(x1, rect.bottom() - 2.0),
                ],
                stroke,
            );
        } else {
            let (y0, y1) = (
                rect.top() + rect.height() * k0,
                rect.top() + rect.height() * k1,
            );
            painter.line_segment(
                [
                    egui::pos2(rect.right() - 2.0, y0),
                    egui::pos2(rect.right() - 2.0, y1),
                ],
                stroke,
            );
        }
    }
}

/// The layer table: a row per layer with its function, material and
/// thickness, buttons to move it up or down or remove it, and Add layer.
/// Returns an edit when something changed: (undo label, new assembly).
pub(super) fn layer_table(
    ui: &mut egui::Ui,
    model: &Model,
    id: ElementId,
    a: &Assembly,
    compact: bool,
) -> Option<(&'static str, Assembly)> {
    let mats = creabim_core::library::materials(model);
    let mut edited = a.clone();
    let mut label = "";
    let mut remove = None;
    let mut swap = None;
    let weak = ui.visuals().weak_text_color();
    let n = a.layers.len();
    let (core0, core1) = a.core();
    ui.label(
        egui::RichText::new(crate::i18n::tr(&format!("{} face", a.usage.first_face())))
            .size(10.5)
            .color(weak),
    );
    egui::Grid::new(("layers", id))
        .num_columns(5)
        .spacing([6.0, 4.0])
        .show(ui, |ui| {
            for (i, l) in edited.layers.iter_mut().enumerate() {
                let (fill, _) = layer_fill(model, l);
                let (r, _) = ui.allocate_exact_size(Vec2::new(10.0, 18.0), Sense::hover());
                ui.painter().rect_filled(r, 2.0, fill);
                if (core0..=core1).contains(&i) {
                    ui.painter().line_segment(
                        [r.left_top(), r.left_bottom()],
                        egui::Stroke::new(2.0, crate::theme::interaction()),
                    );
                }
                egui::ComboBox::from_id_salt(("fn", id, i))
                    .icon(crate::props::combo_icon)
                    .width(if compact { 78.0 } else { 96.0 })
                    .selected_text(crate::i18n::tw(l.function.label()))
                    .show_ui(ui, |ui| {
                        for f in LayerFunction::ALL {
                            if ui
                                .selectable_value(&mut l.function, f, crate::i18n::tw(f.label()))
                                .changed()
                            {
                                label = "Layer function";
                            }
                        }
                    });
                let shown = l
                    .material
                    .and_then(|m| mats.iter().find(|x| x.0 == m))
                    .map_or("None", |x| x.1)
                    .to_string();
                egui::ComboBox::from_id_salt(("mat", id, i))
                    .icon(crate::props::combo_icon)
                    .width(if compact { 110.0 } else { 190.0 })
                    .selected_text(crate::i18n::tw(shown))
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(
                                &mut l.material,
                                None,
                                crate::i18n::tw("None (air, or by type)"),
                            )
                            .changed()
                        {
                            label = "Layer material";
                        }
                        for (mid, name, _) in &mats {
                            if ui
                                .selectable_value(
                                    &mut l.material,
                                    Some(*mid),
                                    crate::i18n::tw(*name),
                                )
                                .changed()
                            {
                                label = "Layer material";
                            }
                        }
                    });
                let mut t = l.thickness;
                if ui
                    .add(
                        crate::units::drag(&mut t)
                            .speed(0.001)
                            .range(assembly::MIN_LAYER..=2.0),
                    )
                    .on_hover_text(crate::i18n::t("Thickness; type any unit"))
                    .changed()
                {
                    l.thickness = t.max(assembly::MIN_LAYER);
                    label = "Layer thickness";
                }
                crate::props::flow(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    if ui
                        .add_enabled(i > 0, GlyphButton(Glyph::Up))
                        .on_hover_text(crate::i18n::t("Move the layer out"))
                        .clicked()
                    {
                        swap = Some((i - 1, i));
                    }
                    if ui
                        .add_enabled(i + 1 < n, GlyphButton(Glyph::Down))
                        .on_hover_text(crate::i18n::t("Move the layer in"))
                        .clicked()
                    {
                        swap = Some((i, i + 1));
                    }
                    if ui
                        .add_enabled(n > 1, GlyphButton(Glyph::Remove))
                        .on_hover_text(crate::i18n::t("Remove the layer"))
                        .clicked()
                    {
                        remove = Some(i);
                    }
                });
                ui.end_row();
            }
        });
    ui.label(
        egui::RichText::new(crate::i18n::tr(&format!("{} face", a.usage.last_face())))
            .size(10.5)
            .color(weak),
    );
    if let Some((i, j)) = swap {
        edited.layers.swap(i, j);
        label = "Reorder layers";
    }
    if let Some(i) = remove {
        edited.layers.remove(i);
        label = "Remove layer";
    }
    crate::props::flow(ui, |ui| {
        if ui
            .button(crate::i18n::t("Add layer"))
            .on_hover_text(crate::i18n::t("A new layer on the inner (or lower) side"))
            .clicked()
        {
            edited
                .layers
                .push(Layer::new(LayerFunction::Finish, None, 0.013));
            label = "Add layer";
        }
        ui.label(crate::i18n::tr(&format!(
            "Total {}",
            crate::units::len(edited.layers.iter().map(|l| l.thickness).sum())
        )));
    });
    (!label.is_empty()).then_some((label, edited))
}

/// U of the assembly (STN EN ISO 6946): the boundary it is counted
/// for, each layer's d, λ and R, the surface resistances and U. Returns
/// an edit of the boundary: (undo label, new assembly).
pub(super) fn thermal_panel(
    ui: &mut egui::Ui,
    model: &Model,
    id: ElementId,
    a: &Assembly,
) -> Option<(&'static str, Assembly)> {
    use creabim_core::thermal::{self, HeatFlow, Outside};
    let t = thermal::of_assembly(model, a, None);
    let mut b = t.boundary;
    let weak = ui.visuals().weak_text_color();
    ui.label(
        egui::RichText::new(crate::i18n::t("Thermal transmittance U"))
            .strong()
            .size(13.0),
    );
    crate::props::flow(ui, |ui| {
        egui::ComboBox::from_id_salt(("asm.flow", id))
            .selected_text(crate::i18n::t(b.flow.label()))
            .show_ui(ui, |ui| {
                for f in HeatFlow::ALL {
                    ui.selectable_value(&mut b.flow, f, crate::i18n::t(f.label()));
                }
            })
            .response
            .on_hover_text(crate::i18n::t(
                "Sets Rsi: 0.10 upwards, 0.13 horizontal, 0.17 downwards (STN EN ISO 6946)",
            ));
        ui.label(crate::i18n::t("On the outer side"));
        egui::ComboBox::from_id_salt(("asm.outside", id))
            .selected_text(crate::i18n::t(b.outside.label()))
            .show_ui(ui, |ui| {
                for o in Outside::ALL {
                    ui.selectable_value(&mut b.outside, o, crate::i18n::t(o.label()));
                }
            })
            .response
            .on_hover_text(crate::i18n::t(
                "Sets Rse: 0.04 outside air, Rse = Rsi towards another room, 0 against the ground",
            ));
    });
    let num = |v: f64| thermal::shown(v);
    egui::Grid::new(("asm.u", id))
        .num_columns(4)
        .spacing([12.0, 2.0])
        .striped(true)
        .show(ui, |ui| {
            for h in ["Layer", "d [m]", "λ [W/(m·K)]", "R [m²K/W]"] {
                ui.label(
                    egui::RichText::new(crate::i18n::t(h))
                        .size(11.0)
                        .color(weak),
                );
            }
            ui.end_row();
            let row = |ui: &mut egui::Ui, name: &str, d: &str, l: &str, r: &str, dim: bool| {
                let txt = |s: &str| {
                    let r = egui::RichText::new(s.to_string());
                    if dim {
                        r.color(weak)
                    } else {
                        r
                    }
                };
                ui.label(txt(name));
                ui.label(txt(d));
                ui.label(txt(l));
                ui.label(txt(r));
                ui.end_row();
            };
            row(ui, "Rsi", "", "", &num(t.rsi), false);
            for l in &t.layers {
                let name = crate::i18n::tr(&l.name);
                let lambda = l.lambda.map_or("—".to_string(), num);
                let r = if l.skipped {
                    crate::i18n::tr("not counted")
                } else {
                    l.r.map_or("?".to_string(), num)
                };
                row(ui, &name, &num(l.thickness), &lambda, &r, l.skipped);
            }
            row(ui, "Rse", "", "", &num(t.rse), false);
        });
    ui.label(format!(
        "R = {} m²K/W · RT = Rsi + R + Rse = {} m²K/W",
        num(t.r),
        num(t.r_total)
    ));
    match t.u {
        Some(u) => {
            ui.label(
                egui::RichText::new(format!("U = {} W/(m²·K)", num(u)))
                    .strong()
                    .size(15.0),
            );
        }
        None => {
            let names: Vec<String> = t
                .missing()
                .iter()
                .map(|l| format!("{}. {}", l.index + 1, crate::i18n::tr(&l.name)))
                .collect();
            ui.label(
                egui::RichText::new(format!(
                    "{} {}",
                    crate::i18n::t(
                        "U is not known yet. Give these layers a material with λ (Materials):"
                    ),
                    names.join(", ")
                ))
                .color(ui.visuals().warn_fg_color),
            );
        }
    }
    if t.layers.iter().any(|l| l.skipped && l.thickness > 0.0) {
        ui.label(
            egui::RichText::new(crate::i18n::t(
                "A ventilated air gap: it and the layers outside it are not counted, Rse = Rsi.",
            ))
            .size(11.0)
            .color(weak),
        );
    }
    if b == t.boundary {
        return None;
    }
    let mut new = a.clone();
    new.thermal = Some(b);
    Some(("Thermal boundary", new))
}

/// "U = 0,245 W/(m²·K)" of an assembly, for Properties.
fn u_line(ui: &mut egui::Ui, model: &Model, a: &Assembly) {
    let t = creabim_core::thermal::of_assembly(model, a, None);
    let text = match t.u {
        Some(u) => format!(
            "U = {} W/(m²·K) · R = {} m²K/W",
            creabim_core::thermal::shown(u),
            creabim_core::thermal::shown(t.r)
        ),
        None => crate::i18n::tr("U: a layer has no λ"),
    };
    ui.label(egui::RichText::new(text).weak())
        .on_hover_text(crate::i18n::t(
            "Thermal transmittance by STN EN ISO 6946, counted from the layers; the assembly editor shows how",
        ));
}

impl CreaBimApp {
    pub(super) fn open_assemblies(&mut self, current: Option<ElementId>) {
        self.asm.open = true;
        if current.is_some() {
            self.asm.current = current;
        }
    }

    /// The selected elements an assembly could go on.
    fn assembly_targets(&self) -> Vec<ElementId> {
        self.selection
            .all()
            .into_iter()
            .filter(|id| {
                self.doc
                    .model()
                    .get(*id)
                    .is_some_and(|e| assembly::assembly_slot(e).is_some())
            })
            .collect()
    }

    /// The Materials menu's assembly items.
    pub(super) fn assemblies_menu(&self) -> Vec<Item> {
        let targets = self.assembly_targets();
        let model = self.doc.model();
        let mut assign = vec![Item::new("asm.assign:none", "None (single material)")];
        for (id, name, a) in assembly::assemblies(model) {
            let fits = targets.iter().any(|t| {
                model
                    .get(*t)
                    .is_some_and(|e| AssemblyUse::of(&e.kind) == Some(a.usage))
            });
            assign.push(
                Item::new(format!("asm.assign:{}", id.0), name)
                    .hint(a.summary(model))
                    .enabled(fits),
            );
        }
        vec![
            Item::Separator,
            Item::new("asm.browser", "Assemblies…")
                .icon("layers-2")
                .hint("Layered wall, floor and roof types: add, edit their layers, assign"),
            Item::new("asm.starters", "Add starter assemblies")
                .icon("plus")
                .hint(
                "A cavity wall, a partition, a floor and a pitched roof, with the materials they use",
            ),
            Item::sub("Assign assembly", assign)
                .icon("layers-2")
                .enabled(!targets.is_empty()),
        ]
    }

    /// Run an assembly menu action. False when the action isn't one.
    pub(super) fn assemblies_action(&mut self, head: &str, arg: &str) -> bool {
        match head {
            "asm.browser" => self.open_assemblies(None),
            "asm.starters" => self.run_asm(AsmAsk::Starters),
            "asm.assign" => {
                let a = self
                    .doc
                    .model()
                    .iter()
                    .find(|e| e.id.0.to_string() == arg)
                    .map(|e| e.id);
                self.run_asm(AsmAsk::Assign(a));
            }
            _ => return self.types_action(head, arg),
        }
        true
    }

    /// The assembly picker in Properties for a wall, slab or roof: which
    /// assembly it is built of, and its layers. True when it changed.
    pub(super) fn assembly_combo(
        &mut self,
        ui: &mut egui::Ui,
        usage: AssemblyUse,
        current: &mut Option<ElementId>,
    ) -> bool {
        let model = self.doc.model();
        let choices: Vec<(ElementId, String)> = assembly::assemblies(model)
            .into_iter()
            .filter(|(_, _, a)| a.usage == usage)
            .map(|(id, n, _)| (id, n.to_string()))
            .collect();
        let shown = current
            .and_then(|c| choices.iter().find(|x| x.0 == c))
            .map_or("None (single material)".to_string(), |x| x.1.clone());
        let mut changed = false;
        let mut create = false;
        let mut edit = false;
        crate::props::flow(ui, |ui| {
            crate::app::helpui::combo(ui, "Assembly")
                .selected_text(crate::i18n::tw(shown))
                .width(ui.available_width().min(150.0))
                .show_ui(ui, |ui| {
                    changed |= ui
                        .selectable_value(current, None, crate::i18n::tw("None (single material)"))
                        .changed();
                    for (id, n) in &choices {
                        changed |= ui
                            .selectable_value(current, Some(*id), crate::i18n::tw(n))
                            .changed();
                    }
                    ui.separator();
                    create = ui.button(crate::i18n::t("New assembly…")).clicked();
                });
        });
        if let Some((_, a)) =
            current
                .and_then(|c| self.doc.model().get(c))
                .and_then(|e| match &e.kind {
                    ElementKind::Assembly(a) => Some((e, a.clone())),
                    _ => None,
                })
        {
            strip(
                ui,
                self.doc.model(),
                &a,
                ui.available_width().min(240.0),
                26.0,
            );
            ui.label(egui::RichText::new(a.summary(self.doc.model())).weak());
            u_line(ui, self.doc.model(), &a);
            edit = ui
                .button(crate::i18n::t("Edit layers…"))
                .on_hover_text(crate::i18n::t(
                    "Change this assembly: every element built of it follows",
                ))
                .clicked();
        }
        if create {
            let a = Assembly::new(usage, vec![Layer::new(LayerFunction::Structure, None, 0.2)]);
            let e = Element::new(
                assembly::unique_name(self.doc.model(), "New assembly"),
                ElementKind::Assembly(a),
            );
            *current = Some(e.id);
            let r = self.doc.commit(Transaction::new("New assembly").insert(e));
            self.open_assemblies(*current);
            self.report(r, "New assembly. Set its layers in the assembly editor.");
            changed = true;
        }
        if edit {
            self.open_assemblies(*current);
        }
        changed
    }

    /// Properties for an assembly picked in the project browser.
    pub(super) fn assembly_properties(&mut self, ui: &mut egui::Ui, id: ElementId) {
        let model = self.doc.model();
        let Some(ElementKind::Assembly(a)) = model.get(id).map(|e| &e.kind) else {
            return;
        };
        let a = a.clone();
        ui.label(
            egui::RichText::new(crate::i18n::tr(&format!(
                "For {}",
                a.usage.label().to_lowercase()
            )))
            .weak(),
        );
        strip(ui, model, &a, ui.available_width().min(240.0), 30.0);
        let edit = layer_table(ui, model, id, &a, true);
        u_line(ui, model, &a);
        let users = assembly::users(model, id).len();
        ui.label(
            egui::RichText::new(match users {
                0 => crate::i18n::t("Not used yet.").to_string(),
                n => crate::i18n::t("Used by {n} element{}.")
                    .replace("{n}", &n.to_string())
                    .replace("{}", if n == 1 { "" } else { "s" }),
            })
            .weak(),
        );
        let open = ui
            .button(crate::i18n::t("Open in the assembly editor"))
            .clicked();
        if let Some((label, new)) = edit {
            self.run_asm(AsmAsk::Edit(id, label, new));
        }
        if open {
            self.open_assemblies(Some(id));
        }
    }

    /// The assembly editor window, when open.
    pub(super) fn assemblies_window(&mut self, ctx: &egui::Context) {
        if !self.asm.open {
            return;
        }
        let mut open = true;
        let mut asks = vec![];
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        crate::app::shell::window(crate::i18n::tr("Assemblies"))
            .open(&mut open)
            .default_pos(area.min + Vec2::new(24.0, 24.0))
            .constrain_to(area)
            .default_size({
                // as tall as the screen gives, so a wall's settings show
                // whole on 1920×1080 (George 2026-11-23)
                let (w, h) = crate::app::shell::window_max(area);
                [w.min(860.0), h]
            })
            .min_width(620.0)
            .collapsible(false)
            .show(ctx, |ui| self.assemblies_body(ui, &mut asks));
        self.asm.open = open;
        for ask in asks {
            self.run_asm(ask);
        }
    }

    fn assemblies_body(&self, ui: &mut egui::Ui, asks: &mut Vec<AsmAsk>) {
        let model = self.doc.model();
        let all = assembly::assemblies(model);
        let current = self
            .asm
            .current
            .filter(|c| all.iter().any(|(id, _, _)| id == c))
            .or_else(|| all.first().map(|a| a.0));
        let height = ui.available_height().max(320.0);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(230.0);
                egui::ScrollArea::vertical()
                    .id_salt("asm_list")
                    .max_height(height - 70.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for usage in AssemblyUse::ALL {
                            let rows: Vec<_> = all.iter().filter(|a| a.2.usage == usage).collect();
                            // a bold heading that folds, so a long list stays
                            // easy to scan (MU18)
                            let head = egui::RichText::new(format!(
                                "{}  ({})",
                                crate::i18n::t(usage.label()),
                                rows.len()
                            ))
                            .size(13.0)
                            .strong();
                            let open = rows.iter().any(|(id, _, _)| current == Some(*id))
                                || rows.len() <= 6;
                            let fold = egui::CollapsingHeader::new(head)
                                .id_salt(("asm_group", usage.label()))
                                .default_open(open)
                                .show(ui, |ui| {
                            for (id, name, a) in rows {
                                let users = assembly::users(model, *id).len();
                                let text = format!(
                                    "{name}  ·  {}{}",
                                    crate::units::len(a.total()),
                                    if users > 0 {
                                        format!("  ·  {users}")
                                    } else {
                                        String::new()
                                    }
                                );
                                if ui
                                    .selectable_label(current == Some(*id), crate::i18n::tw(text))
                                    .on_hover_text(a.summary(model))
                                    .clicked()
                                {
                                    asks.push(AsmAsk::Pick(*id));
                                }
                            }
                            if ui.small_button(crate::i18n::t("+ New")).clicked() {
                                asks.push(AsmAsk::New(usage));
                            }
                                });
                            let _ = fold;
                            ui.separator();
                        }
                        if all.is_empty() {
                            ui.label(
                                egui::RichText::new(
                                    crate::i18n::t("No assemblies yet. Add the starters, or make a new one."),
                                )
                                .weak(),
                            );
                        }
                    });
                crate::props::flow(ui, |ui| {
                    if ui
                        .button(crate::i18n::t("Add starters"))
                        .on_hover_text(crate::i18n::t("A cavity wall, a partition, a floor and a pitched roof"))
                        .clicked()
                    {
                        asks.push(AsmAsk::Starters);
                    }
                    if let Some(c) = current {
                        if ui.button(crate::i18n::t("Duplicate")).clicked() {
                            asks.push(AsmAsk::Duplicate(c));
                        }
                        if ui
                            .button(crate::i18n::t("Delete"))
                            .on_hover_text(crate::i18n::t("What is built of it keeps its thickness, in one material. Ctrl+Z brings it back."))
                            .clicked()
                        {
                            asks.push(AsmAsk::Delete(c));
                        }
                    }
                });
            });
            ui.separator();
            // the assembly's settings scroll on their own, the list
            // stays: the window fits any screen (George 2026-11-23)
            egui::ScrollArea::both()
                .id_salt("asm_detail")
                .max_height(height)
                .auto_shrink([false, true])
                .show(ui, |ui| {
            ui.vertical(|ui| {
                let Some(id) = current else { return };
                let Some(e) = model.get(id) else { return };
                let ElementKind::Assembly(a) = &e.kind else {
                    return;
                };
                let mut name = e.name.clone();
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut name).vertical_align(egui::Align::Center)
                            .font(egui::TextStyle::Heading)
                            .desired_width(320.0),
                    )
                    .changed()
                {
                    asks.push(AsmAsk::Rename(id, name));
                }
                ui.label(
                    egui::RichText::new(crate::i18n::tr(&format!(
                        "IfcMaterialLayerSet · {} · {} total",
                        a.usage.label(),
                        crate::units::len(a.total())
                    )))
                    .weak(),
                );
                let mut description = a.description.clone();
                if ui
                    .add(
                        // wrapped over lines: a maker's source with its
                        // pages reads whole, not cut at the field's end
                        egui::TextEdit::multiline(&mut description)
                            .hint_text(crate::i18n::t("Description"))
                            .desired_rows(1)
                            .desired_width(420.0),
                    )
                    .changed()
                {
                    let mut new = a.clone();
                    new.description = description;
                    asks.push(AsmAsk::Edit(id, "Assembly description", new));
                }
                // the fire resistance the fire checks compare (walls, roofs)
                if matches!(a.usage, AssemblyUse::Wall | AssemblyUse::Roof) {
                    let mut fire = a.fire_resistance.clone();
                    ui.horizontal(|ui| {
                        ui.label(crate::i18n::tr("Fire resistance"));
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut fire)
                                    .hint_text("REW 90/D1")
                                    .desired_width(140.0),
                            )
                            .on_hover_text(crate::i18n::t(
                                "As the assembly's test or the manufacturer states it (REW 90/D1, REI 30): the fire checks compare it with STN 92 0201-2 table 5; left empty, an outer wall counts as a wholly fire-open area",
                            ))
                            .changed()
                        {
                            let mut new = a.clone();
                            new.fire_resistance = fire;
                            asks.push(AsmAsk::Edit(id, "Assembly fire resistance", new));
                        }
                    });
                }
                // which layers turn into the windows' reveals
                if a.usage == AssemblyUse::Wall {
                    let mut wrap = a.wrap;
                    ui.horizontal(|ui| {
                        ui.label(crate::i18n::tr("Window reveals"));
                        egui::ComboBox::from_id_salt(("asm.wrap", id))
                            .selected_text(crate::i18n::tr(wrap.label()))
                            .show_ui(ui, |ui| {
                                for w in creabim_core::RevealWrap::ALL {
                                    ui.selectable_value(&mut wrap, w, crate::i18n::tr(w.label()));
                                }
                            })
                            .response
                            .on_hover_text(crate::i18n::tr(
                                "The thin layers next to the core turn round its edge into a window's reveal and run to the frame; insulation thins to 30 mm there.",
                            ));
                    });
                    if wrap != a.wrap {
                        let mut new = a.clone();
                        new.wrap = wrap;
                        asks.push(AsmAsk::Edit(id, "Window reveals", new));
                    }
                }
                // the plinth: XPS on a membrane below a line over the ground
                if a.usage == AssemblyUse::Wall {
                    if let Some((new, add)) = plinth_panel(ui, model, id, a) {
                        if let Some(m) = add {
                            asks.push(AsmAsk::AddMaterial(Box::new(m)));
                        }
                        asks.push(AsmAsk::Edit(id, "Plinth", new));
                    }
                }
                ui.add_space(6.0);
                strip(ui, model, a, ui.available_width().min(460.0), 44.0);
                ui.add_space(6.0);
                if let Some((label, new)) = layer_table(ui, model, id, a, false) {
                    asks.push(AsmAsk::Edit(id, label, new));
                }
                ui.add_space(8.0);
                ui.separator();
                if let Some((label, new)) = thermal_panel(ui, model, id, a) {
                    asks.push(AsmAsk::Edit(id, label, new));
                }
                ui.add_space(6.0);
                let (c0, c1) = a.core();
                // each sentence translated on its own (George 2026-11-20:
                // the dialog showed them half in English)
                let core = if c0 == c1 {
                    format!("Core: layer {} (orange).", c0 + 1)
                } else {
                    format!("Core: layers {}–{} (orange).", c0 + 1, c1 + 1)
                };
                let run = format!(
                    "Layers run from the {} face to the {}.",
                    a.usage.first_face().to_lowercase(),
                    a.usage.last_face().to_lowercase()
                );
                let mut text = format!("{} {}", crate::i18n::tr(&core), crate::i18n::tr(&run));
                if a.usage == AssemblyUse::Wall {
                    text.push(' ');
                    text.push_str(crate::i18n::t("Where layered walls meet at a T, the core runs on to the other wall's core. A wall's exterior is on its right as drawn (Flip in Properties swaps it)."));
                }
                ui.label(
                    egui::RichText::new(text)
                    .weak(),
                );
                ui.add_space(6.0);
                let users = assembly::users(model, id);
                crate::props::flow(ui, |ui| {
                    ui.label(match users.len() {
                        0 => crate::i18n::t("Not used yet.").to_string(),
                        n => crate::i18n::t("Used by {n} element{}.")
                            .replace("{n}", &n.to_string())
                            .replace("{}", if n == 1 { "" } else { "s" }),
                    });
                    if !users.is_empty() && ui.small_button(crate::i18n::t("Select them")).clicked() {
                        asks.push(AsmAsk::SelectUsers(id));
                    }
                });
                let fits = self
                    .assembly_targets()
                    .iter()
                    .filter(|t| {
                        model
                            .get(**t)
                            .is_some_and(|e| AssemblyUse::of(&e.kind) == Some(a.usage))
                    })
                    .count();
                let r = ui.add_enabled(
                    fits > 0,
                    egui::Button::new(match fits {
                        0 => crate::i18n::t("Assign to the selection").to_string(),
                        1 => crate::i18n::t("Assign to the selected element").to_string(),
                        n => crate::i18n::t("Assign to the {n} selected elements")
                            .replace("{n}", &n.to_string()),
                    }),
                );
                if r.on_disabled_hover_text(crate::i18n::tr(&format!(
                    "Select {} first",
                    a.usage.label().to_lowercase()
                )))
                .clicked()
                {
                    asks.push(AsmAsk::Assign(Some(id)));
                }
            });
                });
        });
    }

    pub(super) fn run_asm(&mut self, ask: AsmAsk) {
        match ask {
            AsmAsk::Pick(id) => self.asm.current = Some(id),
            AsmAsk::New(usage) => {
                let a = Assembly::new(usage, vec![Layer::new(LayerFunction::Structure, None, 0.2)]);
                let e = Element::new(
                    assembly::unique_name(self.doc.model(), "New assembly"),
                    ElementKind::Assembly(a),
                );
                let id = e.id;
                let r = self.doc.commit(Transaction::new("New assembly").insert(e));
                self.asm.current = Some(id);
                self.report(r, "New assembly. Name it and add its layers.");
            }
            AsmAsk::Starters => {
                // the Slovak base first: its materials and build-ups
                let base = creabim_core::skbase::add_base(self.doc.model());
                let based = !base.ops.is_empty();
                if based {
                    let r = self.doc.commit(base);
                    self.report(r, "");
                }
                let (tx, ids) = assembly::add_starters(self.doc.model());
                if tx.ops.is_empty() && !based {
                    self.status = "The starter assemblies are in the project already.".into();
                } else {
                    let r = if tx.ops.is_empty() {
                        Ok(())
                    } else {
                        self.doc.commit(tx)
                    };
                    self.report(
                        r,
                        "Added the usual Slovak materials and build-ups of walls, floors and roofs, and the starter assemblies.",
                    );
                }
                self.open_assemblies(ids.first().copied());
            }
            AsmAsk::Duplicate(id) => {
                if let Some(e) = assembly::duplicate(self.doc.model(), id) {
                    let new = e.id;
                    let r = self
                        .doc
                        .commit(Transaction::new("Duplicate assembly").insert(e));
                    self.asm.current = Some(new);
                    self.report(r, "Duplicated the assembly.");
                }
            }
            AsmAsk::Delete(id) => {
                let name = self
                    .doc
                    .model()
                    .get(id)
                    .map(|e| e.name.clone())
                    .unwrap_or_default();
                let tx = assembly::delete(self.doc.model(), id);
                let r = self.doc.commit(tx);
                self.asm.current = None;
                self.selection.retain(|s| s != id);
                self.report(r, format!("Deleted {name}. Ctrl+Z brings it back."));
            }
            AsmAsk::Rename(id, name) => {
                let r = self
                    .doc
                    .edit_merged(id, "Rename assembly", |e| e.name = name.clone());
                self.report(r, "Renamed.");
            }
            AsmAsk::AddMaterial(m) => {
                let r = self
                    .doc
                    .commit(Transaction::new("Material from the library").insert(*m));
                self.report(r, "Material from the library");
            }
            AsmAsk::Edit(id, label, a) => {
                if let Err(e) = a.validate() {
                    self.status = format!("Can't: {e}.");
                    return;
                }
                if let Some(tx) = assembly::edit(self.doc.model(), id, a, label) {
                    let n = tx.ops.len().saturating_sub(1);
                    let r = self.doc.commit_merged(tx);
                    self.report(
                        r,
                        match n {
                            0 => format!("{label}."),
                            n => format!(
                                "{label}: {n} element{} built of it follow.",
                                if n == 1 { "" } else { "s" }
                            ),
                        },
                    );
                }
            }
            AsmAsk::SelectUsers(id) => {
                let users = assembly::users(self.doc.model(), id);
                self.selection.extend(users.iter().copied(), false);
                self.status = format!("Selected {} element(s).", users.len());
            }
            AsmAsk::Assign(a) => {
                let targets = self.assembly_targets();
                if targets.is_empty() {
                    self.status = "Select walls, slabs or roofs first.".into();
                    return;
                }
                let (tx, n) = assembly::assign(self.doc.model(), &targets, a);
                let name = a
                    .and_then(|a| self.doc.model().get(a))
                    .map_or("one material".to_string(), |e| e.name.clone());
                let r = self.doc.commit(tx);
                self.report(
                    r,
                    match n {
                        0 => format!("Nothing selected takes {name}."),
                        n => format!(
                            "{n} element{} now built of {name}.",
                            if n == 1 { "" } else { "s" }
                        ),
                    },
                );
            }
        }
    }
}

/// The material named first of `names` the model has.
fn material_named(model: &Model, names: &[&str]) -> Option<ElementId> {
    names.iter().find_map(|n| {
        creabim_core::library::materials(model)
            .into_iter()
            .find(|(_, m, _)| m == n)
            .map(|(id, _, _)| id)
    })
}

/// A material picker over the model's materials of `categories`, with
/// `none` for no material.
#[allow(clippy::too_many_arguments)]
fn plinth_material(
    ui: &mut egui::Ui,
    model: &Model,
    salt: (&str, ElementId),
    label: &str,
    none: &str,
    categories: &[&str],
    current: &mut Option<ElementId>,
    add: Option<&mut Option<creabim_core::Element>>,
) -> bool {
    let mats: Vec<(ElementId, String)> = creabim_core::library::materials(model)
        .into_iter()
        .filter(|(id, _, m)| categories.contains(&m.category.as_str()) || *current == Some(*id))
        .map(|(id, n, _)| (id, n.to_string()))
        .collect();
    // the Slovak base's materials of those kinds the project lacks yet
    // (the mosaic render for a plinth, George 2026-11-22)
    let base: Vec<String> = if add.is_some() {
        creabim_core::skbase::base_material_names()
            .into_iter()
            .filter(|n| !mats.iter().any(|m| &m.1 == n))
            .filter(|n| {
                creabim_core::skbase::base_material(n).is_some_and(|e| {
                    matches!(&e.kind, creabim_core::ElementKind::Material(m)
                        if categories.contains(&m.category.as_str()))
                })
            })
            .collect()
    } else {
        vec![]
    };
    let mut take: Option<String> = None;
    let shown = current
        .and_then(|c| mats.iter().find(|m| m.0 == c))
        .map_or_else(|| crate::i18n::tr(none), |m| m.1.clone());
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(crate::i18n::tr(label));
        egui::ComboBox::from_id_salt(salt)
            .selected_text(shown)
            .width(260.0)
            .truncate()
            .show_ui(ui, |ui| {
                changed |= ui
                    .selectable_value(current, None, crate::i18n::tr(none))
                    .changed();
                for (id, n) in &mats {
                    changed |= ui
                        .selectable_value(current, Some(*id), n.as_str())
                        .changed();
                }
                if !base.is_empty() {
                    ui.separator();
                    ui.label(egui::RichText::new(crate::i18n::tr("From the library")).weak());
                    for n in &base {
                        if ui.selectable_label(false, n.as_str()).clicked() {
                            take = Some(n.clone());
                        }
                    }
                }
            });
    });
    if let (Some(n), Some(add)) = (take, add) {
        if let Some(e) = creabim_core::skbase::base_material(&n) {
            *current = Some(e.id);
            *add = Some(e);
            changed = true;
        }
    }
    changed
}

/// A length in millimetres; true when changed.
fn mm_field(ui: &mut egui::Ui, label: &str, hint: &str, value: &mut f64, max: f64) -> bool {
    let mut mm = (*value * 1000.0).round();
    let r = ui
        .horizontal(|ui| {
            ui.label(crate::i18n::tr(label));
            ui.add(
                egui::DragValue::new(&mut mm)
                    .range(0.0..=max)
                    .speed(5.0)
                    .suffix(" mm"),
            )
        })
        .inner
        .on_hover_text(crate::i18n::tr(hint));
    if r.changed() {
        *value = mm / 1000.0;
        true
    } else {
        false
    }
}

/// The build-up's plinth (sokel): on or off, its XPS, membrane, render,
/// height over the ground and bottom. The changed build-up, if any.
fn plinth_panel(
    ui: &mut egui::Ui,
    model: &Model,
    id: ElementId,
    a: &Assembly,
) -> Option<(Assembly, Option<creabim_core::Element>)> {
    use creabim_core::plinth::{Plinth, PlinthBottom};
    let mut new = a.clone();
    let mut add: Option<creabim_core::Element> = None;
    let mut changed = false;
    let mut on = a.plinth.is_some();
    ui.add_space(4.0);
    if ui
        .checkbox(&mut on, crate::i18n::tr("Plinth (XPS and waterproofing)"))
        .on_hover_text(crate::i18n::tr(
            "Below its top the facade insulation gives way to XPS on a waterproofing membrane turned up the masonry; its top lies so far over the highest finished ground along each wall (a wall may set its own heights in steps in Properties).",
        ))
        .changed()
    {
        changed = true;
        new.plinth = on.then(|| {
            // as thick as the facade insulation (flush), XPS on PVC foil
            let outer = a
                .layers
                .iter()
                .find(|l| l.function == LayerFunction::Insulation)
                .map_or(0.1, |l| l.thickness);
            let mut p = Plinth::new(outer);
            p.insulation = material_named(model, &["XPS – extrudovaný (sokel, pod základ)"]);
            p.membrane = material_named(
                model,
                &["Hydroizolácia – fólia PVC", "Hydroizolácia – asfaltový pás"],
            );
            p.lap = 0.15;
            p
        });
    }
    if let Some(p) = new.plinth.as_mut() {
        ui.indent(("asm.plinth", id), |ui| {
            changed |= plinth_material(
                ui,
                model,
                ("asm.plinth.xps", id),
                "Insulation",
                "As the facade",
                &["insulation"],
                &mut p.insulation,
                None,
            );
            let facade = a
                .layers
                .iter()
                .find(|l| l.function == LayerFunction::Insulation)
                .map_or(0.0, |l| l.thickness);
            changed |= mm_field(
                ui,
                "Thickness",
                "As thick as the facade insulation: flush with it; thinner: set back (recessed plinth)",
                &mut p.thickness,
                400.0,
            );
            if facade > 0.0 {
                let back = facade - p.thickness;
                ui.label(
                    egui::RichText::new(if back > 1e-4 {
                        crate::i18n::tr("Recessed plinth: {n} behind the facade")
                            .replace("{n}", &crate::units::len(back))
                    } else {
                        crate::i18n::tr("Flush with the facade")
                    })
                    .weak(),
                );
            }
            changed |= plinth_material(
                ui,
                model,
                ("asm.plinth.membrane", id),
                "Waterproofing",
                "None",
                &["membrane"],
                &mut p.membrane,
                None,
            );
            if p.membrane.is_some() {
                changed |= mm_field(
                    ui,
                    "Membrane over the plinth's top",
                    "How far the waterproofing runs up past the plinth's top, behind the facade insulation",
                    &mut p.lap,
                    1000.0,
                );
            }
            changed |= plinth_material(
                ui,
                model,
                ("asm.plinth.render", id),
                "Plinth render",
                "As the facade",
                &["plaster", "finish", "stone"],
                &mut p.render,
                Some(&mut add),
            );
            changed |= mm_field(
                ui,
                "Top over the finished ground",
                "Over the highest finished terrain or paving along the wall; 300 mm as usual",
                &mut p.above,
                2000.0,
            );
            let mut kind = p.bottom.kind();
            ui.horizontal(|ui| {
                ui.label(crate::i18n::tr("Bottom"));
                egui::ComboBox::from_id_salt(("asm.plinth.bottom", id))
                    .selected_text(crate::i18n::tr(PlinthBottom::KINDS[kind]))
                    .show_ui(ui, |ui| {
                        for (i, k) in PlinthBottom::KINDS.iter().enumerate() {
                            ui.selectable_value(&mut kind, i, crate::i18n::tr(k));
                        }
                    })
                    .response
                    .on_hover_text(crate::i18n::tr(
                        "At the wall's base: the masonry under it (DT blocks) carries its own XPS and membrane down to the footing. Over the slab's side: down the foundation slab's edge.",
                    ));
            });
            if kind != p.bottom.kind() {
                p.bottom = match kind {
                    0 => PlinthBottom::WallBase,
                    _ => PlinthBottom::SlabSide(0.0),
                };
                changed = true;
            }
            if let PlinthBottom::SlabSide(mut d) = p.bottom {
                if mm_field(
                    ui,
                    "Below the slab's underside",
                    "How far it runs on down past the slab's underside",
                    &mut d,
                    1500.0,
                ) {
                    p.bottom = PlinthBottom::SlabSide(d);
                    changed = true;
                }
            }
        });
    }
    changed.then_some((new, add))
}
