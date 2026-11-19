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
    /// A size being typed on its dimension in the drawing, and the text
    /// typed so far; applied on Enter or the tick, not before.
    typing: Option<(Target, String)>,
}

/// What a dimension in the editor's drawing measures, to be typed on.
#[derive(Clone, Debug, PartialEq)]
enum Target {
    /// The field at the path: its width (`true`) or height.
    Field(Vec<usize>, bool),
    /// The whole opening's width (`true`) or height.
    Whole(bool),
}

/// The split above `path` that sets the field's width (`across`) or
/// height: the nearest one running that way, with the child of it the
/// field is in.
fn sizing_split(node: &FrameNode, path: &[usize], across: bool) -> Option<Vec<usize>> {
    for n in (1..=path.len()).rev() {
        if let Some(FrameNode::Split { across: a, .. }) = node.at(&path[..n - 1]) {
            if *a == across {
                return Some(path[..n].to_vec());
            }
        }
    }
    None
}

/// Make the field at `path`, now `len` long along `across`, `target`
/// long: its share of the split that sizes it, the fields beside it
/// taking the rest. False where it cannot be so long.
fn set_length(node: &mut FrameNode, path: &[usize], across: bool, len: f64, target: f64) -> bool {
    let Some(child) = sizing_split(node, path, across) else {
        return false;
    };
    let Some((i, parent)) = child.split_last() else {
        return false;
    };
    let Some(FrameNode::Split { sizes, parts, .. }) = node.at(parent) else {
        return false;
    };
    let sum: f64 = (0..parts.len())
        .map(|j| sizes.get(j).copied().unwrap_or(1.0))
        .sum();
    let share = sizes.get(*i).copied().unwrap_or(1.0) / sum.max(1e-9);
    // the room the split shares out, without its bars
    let free = len / share.max(1e-9);
    let others = parts.len().saturating_sub(1) as f64;
    if target < 0.1 || free - target < 0.1 * others {
        return false;
    }
    node.set_share(&child, target / free);
    true
}

/// A typed length in mm ("1 470", "1470,5") in metres.
fn typed_mm(text: &str) -> Option<f64> {
    let t: String = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| if c == ',' { '.' } else { c })
        .collect();
    let t = t.trim_end_matches("mm");
    t.parse::<f64>()
        .ok()
        .filter(|v| *v > 0.0)
        .map(|v| v / 1000.0)
}

/// What the opening being edited is, read once a frame.
struct Subject {
    name: String,
    door: bool,
    w: f64,
    h: f64,
    fw: f64,
    /// A door's frame allowance: what its opening is wider and higher
    /// than its leaf.
    allow: Option<(f64, f64)>,
}

/// The frame a door's leaf is reckoned with: its own, or a lining.
fn door_frame(o: &creabim_core::Opening) -> creabim_core::joinery::DoorFrame {
    o.door
        .as_ref()
        .map_or(creabim_core::joinery::DoorFrame::Lining, |d| d.frame)
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
            // a door's turning leaves are its door leaves
            node: if o.kind == OpeningKind::Door {
                division_of(o).as_door()
            } else {
                division_of(o)
            },
            sel: None,
            mode: Mode::Select,
            inside: false,
            w: o.width,
            h: o.height,
            typing: None,
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
                        // its mark as the plans show it, else its name in
                        // the interface's language
                        name: creabim_core::marks::mark_of(self.doc.model(), e.id)
                            .unwrap_or_else(|| crate::i18n::tr(&e.name)),
                        door,
                        w: ed.w,
                        h: ed.h,
                        fw,
                        allow: door.then(|| door_frame(o).allowance()),
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
        let typing = ed.typing.is_some();
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
        // Esc ends typing on a dimension first, the editor only after
        if !typing && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
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
                if door {
                    // a door is its leaf: the opening follows from it
                    let frame = door_frame(o);
                    let (aw, ah) = frame.allowance();
                    let mut d = o.door.clone().unwrap_or_else(|| {
                        creabim_core::joinery::DoorSpec::fitting(frame, o.width, o.height)
                    });
                    d.nom_width = (ed.w - aw).max(0.3);
                    d.nom_height = (ed.h - ah).max(1.0);
                    creabim_core::joinery::apply_door(o, d);
                } else {
                    o.width = ed.w;
                    o.height = ed.h;
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
/// The ways a field can open: a door's leaves are door leaves (no
/// turning or tilting sashes), fixed or sliding parts beside them.
fn ops(door: bool) -> Vec<LeafOp> {
    LeafOp::ALL
        .into_iter()
        .filter(|o| {
            if door {
                *o == o.in_door() && !o.tilts() && *o != LeafOp::TopHung
            } else {
                !o.is_door()
            }
        })
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
    let (fields, bars) = ed.node.layout_of(s.door, s.w, s.h, s.fw);
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
        // how it opens, dashed (George: as on paper)
        let sym = Stroke::new(1.2, Color32::from_gray(30));
        for l in sash_symbol(f, sw) {
            painter.extend(egui::Shape::dashed_line(
                &[Pos2::new(px(l[0]), py(l[1])), Pos2::new(px(l[2]), py(l[3]))],
                sym,
                7.0,
                4.0,
            ));
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
    // each number's place, to be clicked and typed on
    let mut spots: Vec<(Rect, Target, f64)> = vec![];
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
    let spot_h = |a: f64, b: f64, y: f32| {
        Rect::from_center_size(
            Pos2::new((px(a) + px(b)) / 2.0, y - 9.0),
            Vec2::new(60.0, 16.0),
        )
    };
    let spot_v = |a: f64, b: f64, x: f32| {
        Rect::from_center_size(
            Pos2::new(x - 24.0, (py(a) + py(b)) / 2.0),
            Vec2::new(48.0, 16.0),
        )
    };
    for f in &fields {
        if (f.y0 - bottom).abs() < 1e-6 && fields.len() > 1 {
            let y = py(0.0) + 16.0;
            if sizing_split(&ed.node, &f.path, true).is_some() {
                spots.push((
                    spot_h(f.x0, f.x1, y),
                    Target::Field(f.path.clone(), true),
                    f.x1 - f.x0,
                ));
            }
            dim_h(f.x0, f.x1, y, mm(f.x1 - f.x0));
        }
        if (f.x0 - left).abs() < 1e-6 && fields.len() > 1 {
            let x = x_left - 12.0;
            if sizing_split(&ed.node, &f.path, false).is_some() {
                spots.push((
                    spot_v(f.y0, f.y1, x),
                    Target::Field(f.path.clone(), false),
                    f.y1 - f.y0,
                ));
            }
            dim_v(f.y0, f.y1, x, mm(f.y1 - f.y0));
        }
    }
    dim_h(0.0, s.w, py(0.0) + 38.0, mm(s.w));
    dim_v(0.0, s.h, x_left - 50.0, mm(s.h));
    spots.push((spot_h(0.0, s.w, py(0.0) + 38.0), Target::Whole(true), s.w));
    spots.push((spot_v(0.0, s.h, x_left - 50.0), Target::Whole(false), s.h));
    typing_on(ui, ed, s, &spots);
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

/// The numbers of the drawing typed on: a click on one opens a box over
/// it with a tick; Enter or the tick sets the size and only then is it
/// redrawn, Esc leaves it as it was.
fn typing_on(ui: &mut egui::Ui, ed: &mut JoineryEdit, s: &Subject, spots: &[(Rect, Target, f64)]) {
    let t = crate::i18n::tr;
    for (i, (r, target, now)) in spots.iter().enumerate() {
        let id = egui::Id::new(("joinery.dim", i));
        let editing = ed.typing.as_ref().is_some_and(|(x, _)| x == target);
        if !editing {
            let hit = ui
                .interact(*r, id, egui::Sense::click())
                .on_hover_text(t("Click to type the size, then Enter or ✔"));
            if hit.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
            }
            if hit.clicked() {
                ed.typing = Some((target.clone(), format!("{:.0}", now * 1000.0)));
            }
            continue;
        }
        let Some((_, text)) = ed.typing.as_mut() else {
            continue;
        };
        let field = Rect::from_min_size(
            r.min - Vec2::new(4.0, 2.0),
            Vec2::new(r.width() + 8.0, 20.0),
        );
        let edit = ui.put(
            field,
            egui::TextEdit::singleline(text)
                .id(id.with("text"))
                .horizontal_align(egui::Align::Center),
        );
        if !edit.has_focus() && !edit.lost_focus() {
            edit.request_focus();
        }
        let tick = ui.put(
            Rect::from_min_size(
                field.right_top() + Vec2::new(3.0, 0.0),
                Vec2::new(22.0, 20.0),
            ),
            egui::Button::new("✔"),
        );
        let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            ed.typing = None;
            return;
        }
        if !(enter || tick.clicked()) {
            continue;
        }
        let Some(v) = typed_mm(text) else {
            ed.typing = None;
            return;
        };
        match target {
            Target::Whole(true) => ed.w = v.clamp(0.3, 12.0),
            Target::Whole(false) => ed.h = v.clamp(0.3, 6.0),
            Target::Field(path, across) => {
                let _ = set_length(&mut ed.node, path, *across, *now, v);
            }
        }
        let _ = s;
        ed.typing = None;
        return;
    }
}

/// The side panel: the field picked, and the whole division.
fn side(ui: &mut egui::Ui, ed: &mut JoineryEdit, s: &Subject, others: &[(String, FrameNode)]) {
    let t = crate::i18n::tr;
    let (fields, _) = ed.node.layout_of(s.door, s.w, s.h, s.fw);
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
    // a door is typed by its leaf, the opening following from the
    // frame; a window by its whole opening, in millimetres
    // a door divided into leaves and fixed parts: its whole opening
    let divided = matches!(ed.node, FrameNode::Split { .. });
    let (aw, ah) = if divided {
        (0.0, 0.0)
    } else {
        s.allow.unwrap_or((0.0, 0.0))
    };
    ui.label(
        egui::RichText::new(t(if s.door && !divided {
            "Door leaf"
        } else {
            "The whole opening"
        }))
        .strong(),
    );
    for (label, v, a, lo, hi) in [
        ("Width", &mut ed.w, aw, 0.3, 12.0),
        ("Height", &mut ed.h, ah, 0.3, 6.0),
    ] {
        ui.horizontal(|ui| {
            ui.label(t(label));
            let mut mm = ((*v - a) * 1000.0).round();
            if ui
                .add(
                    // typed numbers apply on Enter or on leaving the field
                    egui::DragValue::new(&mut mm)
                        .speed(10.0)
                        .range(lo * 1000.0..=hi * 1000.0)
                        .max_decimals(0)
                        .update_while_editing(false)
                        .suffix(" mm"),
                )
                .changed()
            {
                *v = mm / 1000.0 + a;
            }
        });
    }
    if s.door && !divided {
        ui.label(
            egui::RichText::new(format!(
                "{}: {} × {} mm",
                t("Structural opening"),
                mm(ed.w),
                mm(ed.h)
            ))
            .weak(),
        );
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
                    let mut v = (len * 1000.0).round();
                    ui.horizontal(|ui| {
                        ui.label(t(if across {
                            "Field width"
                        } else {
                            "Field height"
                        }));
                        if ui
                            .add(
                                // typed whole: applied on Enter or on leaving
                                // the field, the fields beside it take the rest
                                egui::DragValue::new(&mut v)
                                    .speed(5.0)
                                    .range(100.0..=(free * 1000.0 - 100.0).round())
                                    .max_decimals(0)
                                    .update_while_editing(false)
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

#[cfg(test)]
mod typing_tests {
    use super::*;

    #[test]
    fn a_field_typed_on_its_dimension_gets_that_width_exactly() {
        let mut node = FrameNode::parse("v(1:OS-L,1:FIX)").unwrap();
        let (w, h, fw) = (1.75, 1.5, 0.07);
        let (fields, _) = node.layout(w, h, fw, fw, true);
        let f = &fields[0];
        assert!(set_length(&mut node, &f.path, true, f.x1 - f.x0, 1.2));
        let (fields, _) = node.layout(w, h, fw, fw, true);
        assert!((fields[0].x1 - fields[0].x0 - 1.2).abs() < 1e-9);
        // a field can't leave the one beside it nothing
        let now = fields[0].x1 - fields[0].x0;
        assert!(!set_length(&mut node, &fields[0].path, true, now, 1.8));
        // no split runs up and down here: its height is the window's
        assert!(sizing_split(&node, &fields[0].path, false).is_none());
    }

    #[test]
    fn typed_sizes_read_with_spaces_and_commas() {
        assert_eq!(typed_mm("1 470"), Some(1.47));
        assert_eq!(typed_mm("1470,5 mm"), Some(1.4705));
        assert_eq!(typed_mm("14x"), None);
        assert_eq!(typed_mm(""), None);
    }
}
