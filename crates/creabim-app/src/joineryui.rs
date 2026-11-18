//! The window and door editor (PilonDesigner's `frameeditor.ts`): the
//! opening drawn from outside or inside, a click picks a field to set
//! how it opens, what fills it and how large it is; the Mullion and
//! Transom tools split the field clicked where it is clicked. The
//! opening symbols are STN 01 3420's: a triangle's point is at the
//! hinges.

use super::CreaBimApp;
use creabim_core::joinery::{division_of, sash_symbol, Field, FrameNode, Sash};
use creabim_core::{ElementId, ElementKind, LeafOp, OpeningKind, Transaction};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Select,
    Mullion,
    Transom,
}

/// The editor's state while it is open.
pub(super) struct JoineryEdit {
    id: ElementId,
    node: FrameNode,
    sel: Option<Vec<usize>>,
    mode: Mode,
    inside: bool,
    /// The whole opening's width and height, as typed here, m.
    w: f64,
    h: f64,
}

/// What the opening being edited is, read once a frame.
struct Subject {
    name: String,
    door: bool,
    w: f64,
    h: f64,
    fw: f64,
}

impl CreaBimApp {
    /// Open the editor on the door or window `id`.
    pub(super) fn joinery_open(&mut self, id: ElementId) {
        let Some(ElementKind::Opening(o)) = self.doc.model().get(id).map(|e| &e.kind) else {
            return;
        };
        if o.kind == OpeningKind::Hole {
            return;
        }
        self.joinery = Some(JoineryEdit {
            id,
            node: division_of(o),
            sel: None,
            mode: Mode::Select,
            inside: false,
            w: o.width,
            h: o.height,
        });
    }

    /// `joinery:edit` from a tab: the editor on the selected door or
    /// window.
    pub(super) fn joinery_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "joinery" || arg != "edit" {
            return false;
        }
        let picked = self.selection.all().into_iter().find(|id| {
            matches!(
                self.doc.model().get(*id).map(|e| &e.kind),
                Some(ElementKind::Opening(o)) if o.kind != OpeningKind::Hole
            )
        });
        match picked {
            Some(id) => self.joinery_open(id),
            None => {
                self.status =
                    crate::i18n::tr("Select a window or a door first, then open its editor.")
            }
        }
        true
    }

    /// The editor's window, while it is open.
    pub(super) fn joinery_window(&mut self, ctx: &egui::Context) {
        let Some(mut ed) = self.joinery.take() else {
            return;
        };
        let subject = match self.doc.model().get(ed.id) {
            Some(e) => match &e.kind {
                ElementKind::Opening(o) => {
                    let door = o.kind == OpeningKind::Door;
                    let fw = match (&o.door, door) {
                        (Some(d), true) => d.members(ed.w, ed.h).0,
                        _ => o.frame.fitted(ed.w, ed.h, 1.0).width,
                    };
                    Subject {
                        name: e.name.clone(),
                        door,
                        w: ed.w,
                        h: ed.h,
                        fw,
                    }
                }
                _ => return,
            },
            None => return,
        };
        // the others of its kind that have a division, to take one over
        let others: Vec<(String, FrameNode)> = self
            .doc
            .model()
            .iter()
            .filter(|e| e.id != ed.id)
            .filter_map(|e| match &e.kind {
                ElementKind::Opening(o)
                    if (o.kind == OpeningKind::Door) == subject.door && o.division.is_some() =>
                {
                    Some((e.name.clone(), o.division.clone()?))
                }
                _ => None,
            })
            .collect();
        let t = crate::i18n::tr;
        let mut open = true;
        let (mut done, mut cancel) = (false, false);
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        let title = format!(
            "{} {}",
            t(if subject.door {
                "Door editor"
            } else {
                "Window editor"
            }),
            subject.name
        );
        super::shell::window(title)
            .id(egui::Id::new("joinery.edit"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(430.0, 330.0))
            .default_size(Vec2::new(860.0, 660.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (m, text, hint) in [
                        (
                            Mode::Select,
                            "Select",
                            "Click a field to set how it opens, what fills it and how large it is.",
                        ),
                        (
                            Mode::Mullion,
                            "Mullion",
                            "Click in a field to split it by an upright where you click.",
                        ),
                        (
                            Mode::Transom,
                            "Transom",
                            "Click in a field to split it by a cross member where you click.",
                        ),
                    ] {
                        ui.selectable_value(&mut ed.mode, m, t(text))
                            .on_hover_text(t(hint));
                    }
                    ui.separator();
                    ui.selectable_value(&mut ed.inside, false, t("From outside"));
                    ui.selectable_value(&mut ed.inside, true, t("From inside"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(t("Done")).clicked() {
                            done = true;
                        }
                        if ui.button(t("Cancel")).clicked() {
                            cancel = true;
                        }
                    });
                });
                ui.separator();
                ui.horizontal_top(|ui| {
                    draw(ui, &mut ed, &subject);
                    ui.separator();
                    ui.vertical(|ui| {
                        ui.set_width(250.0);
                        side(ui, &mut ed, &subject, &others);
                    });
                });
            });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            cancel = true;
        }
        if done {
            self.joinery_commit(&ed, subject.door);
        } else if open && !cancel {
            self.joinery = Some(ed);
        }
    }

    fn joinery_commit(&mut self, ed: &JoineryEdit, door: bool) {
        let Some(before) = self.doc.model().get(ed.id).cloned() else {
            return;
        };
        let mut after = before.clone();
        if let ElementKind::Opening(o) = &mut after.kind {
            // a division that says nothing is left out
            let plain = match &ed.node {
                FrameNode::Leaf(s) => {
                    !s.panel
                        && if door {
                            s.op == LeafOp::DoorLeft
                        } else {
                            s.op == LeafOp::Fixed
                                && o.frame.panes_across <= 1
                                && o.frame.panes_up <= 1
                        }
                }
                _ => false,
            };
            o.division = if plain { None } else { Some(ed.node.clone()) };
            // the whole size typed here; a door's leaf keeps its frame's
            // allowance
            let (dw, dh) = (ed.w - o.width, ed.h - o.height);
            if dw.abs() > 1e-6 || dh.abs() > 1e-6 {
                o.width = ed.w;
                o.height = ed.h;
                if let Some(d) = &mut o.door {
                    d.nom_width = (d.nom_width + dw).max(0.3);
                    d.nom_height = (d.nom_height + dh).max(1.0);
                }
            }
        }
        if after == before {
            return;
        }
        let label = if door {
            "Door division"
        } else {
            "Window division"
        };
        match self
            .doc
            .commit(Transaction::new(label).replace(before, after))
        {
            Ok(_) => self.status = crate::i18n::tr(label),
            Err(e) => self.status = format!("Couldn't do that: {e}"),
        }
    }
}

/// The ops a field of a window or of a door can have.
fn ops(door: bool) -> Vec<LeafOp> {
    LeafOp::ALL
        .into_iter()
        .filter(|o| door || !o.is_door())
        .collect()
}

fn mm(v: f64) -> String {
    format!("{:.0}", v * 1000.0)
}

/// The drawing: the frame, its bars and fields with their symbols and
/// sizes; a click picks or splits.
fn draw(ui: &mut egui::Ui, ed: &mut JoineryEdit, s: &Subject) {
    let pad = 56.0_f32;
    // room on the left for both rows of heights with their numbers
    let pad_x = 88.0_f32;
    let room = 500.0_f32;
    let k = (room / s.w as f32).min(room / s.h as f32);
    let size = Vec2::new(s.w as f32 * k + pad_x + pad, s.h as f32 * k + 2.0 * pad);
    let (resp, painter) = ui.allocate_painter(size, egui::Sense::click());
    let o = resp.rect.min;
    let inside = ed.inside;
    let px = |x: f64| o.x + pad_x + (if inside { s.w - x } else { x }) as f32 * k;
    let py = |y: f64| o.y + pad + (s.h - y) as f32 * k;
    let rect = |x0: f64, y0: f64, x1: f64, y1: f64| {
        Rect::from_two_pos(Pos2::new(px(x0), py(y0)), Pos2::new(px(x1), py(y1)))
    };
    let vis = ui.visuals();
    let ink = vis.text_color();
    let dark = vis.dark_mode;
    let frame_fill = if dark {
        Color32::from_gray(205)
    } else {
        Color32::from_gray(235)
    };
    let line = Stroke::new(1.0, Color32::from_gray(if dark { 40 } else { 70 }));
    let glass = Color32::from_rgb(168, 205, 228);
    let panel = Color32::from_gray(150);
    let leaf = Color32::from_rgb(170, 128, 88);
    let accent = vis.selection.stroke.color;
    painter.rect_filled(rect(0.0, 0.0, s.w, s.h), 0.0, frame_fill);
    painter.rect_stroke(
        rect(0.0, 0.0, s.w, s.h),
        0.0,
        line,
        egui::StrokeKind::Inside,
    );
    let (fields, bars) = ed.node.layout(s.w, s.h, s.fw, s.fw, !s.door);
    for b in &bars {
        painter.rect_stroke(
            rect(b.x0, b.y0, b.x1, b.y1),
            0.0,
            line,
            egui::StrokeKind::Inside,
        );
    }
    let sash = |f: &Field| {
        if f.sash.op == LeafOp::Fixed {
            0.0
        } else if f.sash.op.is_door() {
            0.004
        } else {
            (s.fw * 0.75)
                .min((f.x1 - f.x0) / 4.0)
                .min((f.y1 - f.y0) / 4.0)
        }
    };
    for f in &fields {
        let sw = sash(f);
        let fill = if f.sash.op.is_door() {
            leaf
        } else if f.sash.panel {
            panel
        } else {
            glass
        };
        if sw > 0.0 && !f.sash.op.is_door() {
            painter.rect_stroke(
                rect(f.x0, f.y0, f.x1, f.y1),
                0.0,
                line,
                egui::StrokeKind::Inside,
            );
        }
        let inner = rect(f.x0 + sw, f.y0 + sw, f.x1 - sw, f.y1 - sw);
        painter.rect_filled(inner, 0.0, fill);
        painter.rect_stroke(inner, 0.0, line, egui::StrokeKind::Inside);
        let sym = Stroke::new(1.2, Color32::from_gray(30));
        for l in sash_symbol(f, sw) {
            painter.line_segment(
                [Pos2::new(px(l[0]), py(l[1])), Pos2::new(px(l[2]), py(l[3]))],
                sym,
            );
        }
        if ed.sel.as_deref() == Some(f.path.as_slice()) {
            painter.rect_stroke(
                rect(f.x0, f.y0, f.x1, f.y1),
                0.0,
                Stroke::new(2.5, accent),
                egui::StrokeKind::Inside,
            );
        }
    }
    // sizes: the fields along the bottom and the left, and the whole
    let font = egui::FontId::proportional(11.0);
    let dim = Stroke::new(1.0, ink.gamma_multiply(0.7));
    let dim_h = |a: f64, b: f64, y: f32, text: String| {
        let (xa, xb) = (px(a), px(b));
        painter.line_segment([Pos2::new(xa, y), Pos2::new(xb, y)], dim);
        for x in [xa, xb] {
            painter.line_segment([Pos2::new(x, y - 4.0), Pos2::new(x, y + 4.0)], dim);
        }
        painter.text(
            Pos2::new((xa + xb) / 2.0, y - 2.0),
            egui::Align2::CENTER_BOTTOM,
            text,
            font.clone(),
            ink,
        );
    };
    let dim_v = |a: f64, b: f64, x: f32, text: String| {
        let (ya, yb) = (py(a), py(b));
        painter.line_segment([Pos2::new(x, ya), Pos2::new(x, yb)], dim);
        for y in [ya, yb] {
            painter.line_segment([Pos2::new(x - 4.0, y), Pos2::new(x + 4.0, y)], dim);
        }
        painter.text(
            Pos2::new(x - 3.0, (ya + yb) / 2.0),
            egui::Align2::RIGHT_CENTER,
            text,
            font.clone(),
            ink,
        );
    };
    let bottom = fields.iter().map(|f| f.y0).fold(f64::MAX, f64::min);
    let left = fields.iter().map(|f| f.x0).fold(f64::MAX, f64::min);
    let x_left = px(if inside { s.w } else { 0.0 });
    for f in &fields {
        if (f.y0 - bottom).abs() < 1e-6 && fields.len() > 1 {
            dim_h(f.x0, f.x1, py(0.0) + 16.0, mm(f.x1 - f.x0));
        }
        if (f.x0 - left).abs() < 1e-6 && fields.len() > 1 {
            dim_v(f.y0, f.y1, x_left - 12.0, mm(f.y1 - f.y0));
        }
    }
    dim_h(0.0, s.w, py(0.0) + 38.0, mm(s.w));
    dim_v(0.0, s.h, x_left - 50.0, mm(s.h));
    painter.text(
        Pos2::new(o.x + pad_x, o.y + 8.0),
        egui::Align2::LEFT_TOP,
        crate::i18n::tr(if inside {
            "Seen from inside. A triangle's point is at the hinges."
        } else {
            "Seen from outside. A triangle's point is at the hinges."
        }),
        font.clone(),
        ink.gamma_multiply(0.7),
    );
    if !resp.clicked() {
        return;
    }
    let Some(p) = resp.interact_pointer_pos() else {
        return;
    };
    let x = {
        let v = ((p.x - o.x - pad_x) / k) as f64;
        if inside {
            s.w - v
        } else {
            v
        }
    };
    let y = s.h - ((p.y - o.y - pad) / k) as f64;
    let hit = fields
        .iter()
        .find(|f| x >= f.x0 && x <= f.x1 && y >= f.y0 && y <= f.y1);
    match (hit, ed.mode) {
        (None, _) => ed.sel = None,
        (Some(f), Mode::Select) => ed.sel = Some(f.path.clone()),
        (Some(f), Mode::Mullion) => {
            ed.node
                .split(&f.path, true, (x - f.x0) / (f.x1 - f.x0).max(1e-6));
            ed.sel = None;
        }
        (Some(f), Mode::Transom) => {
            ed.node
                .split(&f.path, false, (f.y1 - y) / (f.y1 - f.y0).max(1e-6));
            ed.sel = None;
        }
    }
}

/// The side panel: the field picked, and the whole division.
fn side(ui: &mut egui::Ui, ed: &mut JoineryEdit, s: &Subject, others: &[(String, FrameNode)]) {
    let t = crate::i18n::tr;
    let (fields, _) = ed.node.layout(s.w, s.h, s.fw, s.fw, !s.door);
    ui.label(
        egui::RichText::new(format!(
            "{} × {} mm · {} {}",
            mm(s.w),
            mm(s.h),
            fields.len(),
            t(if fields.len() == 1 { "field" } else { "fields" })
        ))
        .weak(),
    );
    ui.add_space(6.0);
    // the whole opening's size, typed in millimetres
    ui.label(egui::RichText::new(t("The whole opening")).strong());
    for (label, v, lo, hi) in [
        ("Width", &mut ed.w, 0.3, 12.0),
        ("Height", &mut ed.h, 0.3, 6.0),
    ] {
        ui.horizontal(|ui| {
            ui.label(t(label));
            let mut mm = (*v * 1000.0).round();
            if ui
                .add(
                    egui::DragValue::new(&mut mm)
                        .speed(10.0)
                        .range(lo * 1000.0..=hi * 1000.0)
                        .suffix(" mm"),
                )
                .changed()
            {
                *v = mm / 1000.0;
            }
        });
    }
    ui.add_space(8.0);
    let sel = ed.sel.clone();
    let picked = sel
        .as_ref()
        .and_then(|p| fields.iter().find(|f| &f.path == p).cloned());
    match (sel, picked) {
        (Some(path), Some(f)) => {
            ui.label(egui::RichText::new(t("The field picked")).strong());
            let Sash { mut op, mut panel } = f.sash;
            let mut changed = false;
            egui::ComboBox::from_id_salt("joinery.op")
                .selected_text(t(op.label()))
                .width(230.0)
                .show_ui(ui, |ui| {
                    for x in ops(s.door) {
                        changed |= ui.selectable_value(&mut op, x, t(x.label())).changed();
                    }
                });
            if !op.is_door() {
                changed |= ui
                    .checkbox(&mut panel, t("A solid panel instead of glass"))
                    .changed();
            }
            if changed {
                if let Some(n) = ed.node.at_mut(&path) {
                    *n = FrameNode::Leaf(Sash { op, panel });
                }
            }
            ui.label(format!(
                "{} {} × {} mm",
                t("Clear size"),
                mm(f.x1 - f.x0),
                mm(f.y1 - f.y0)
            ));
            // its size along its parent's split
            if let Some((i, parent)) = path.split_last() {
                if let Some(FrameNode::Split { across, sizes, .. }) = ed.node.at(parent) {
                    let across = *across;
                    let sum: f64 = sizes.iter().sum();
                    let share = sizes.get(*i).copied().unwrap_or(1.0) / sum.max(1e-9);
                    let len = if across { f.x1 - f.x0 } else { f.y1 - f.y0 };
                    // only a leaf's own length is its share of the room
                    let free = len / share.max(1e-9);
                    let mut v = len * 1000.0;
                    ui.horizontal(|ui| {
                        ui.label(t(if across {
                            "Field width"
                        } else {
                            "Field height"
                        }));
                        if ui
                            .add(
                                egui::DragValue::new(&mut v)
                                    .speed(5.0)
                                    .range(100.0..=free * 1000.0 - 100.0)
                                    .suffix(" mm"),
                            )
                            .changed()
                        {
                            ed.node.set_share(&path, v / 1000.0 / free);
                        }
                    });
                }
            }
            ui.horizontal(|ui| {
                if ui.button(t("Split by a mullion")).clicked() {
                    ed.node.split(&path, true, 0.5);
                    ed.sel = None;
                }
                if ui.button(t("Split by a transom")).clicked() {
                    ed.node.split(&path, false, 0.5);
                    ed.sel = None;
                }
            });
            if !path.is_empty()
                && ui
                    .button(t("Remove the field"))
                    .on_hover_text(t("Its room goes to the field beside it."))
                    .clicked()
            {
                ed.node.remove(&path);
                ed.sel = None;
            }
        }
        _ => {
            ui.label(
                egui::RichText::new(t("Click a field to set how it opens, what fills it and how large it is. With Mullion or Transom, click in a field where it is to be split."))
                    .weak(),
            );
        }
    }
    ui.add_space(10.0);
    ui.separator();
    if !others.is_empty() {
        ui.label(egui::RichText::new(t("Take the division over")).strong());
        egui::ComboBox::from_id_salt("joinery.copy")
            .selected_text(t("From…"))
            .width(230.0)
            .show_ui(ui, |ui| {
                for (name, node) in others {
                    if ui.selectable_label(false, name).clicked() {
                        ed.node = node.clone();
                        ed.sel = None;
                    }
                }
            });
    }
    if ui.button(t("Clear the division")).clicked() {
        ed.node = FrameNode::leaf(if s.door {
            LeafOp::DoorLeft
        } else {
            LeafOp::TiltTurnLeft
        });
        ed.sel = None;
    }
    ui.add_space(6.0);
    ui.label(
        egui::RichText::new(format!(
            "{}: {}",
            t("In schedules"),
            ed.node
                .leaves()
                .iter()
                .map(|x| x.op.code())
                .collect::<Vec<_>>()
                .join(" + ")
        ))
        .weak(),
    );
}
