//! A calculation window steps aside while what it adds is placed in the
//! drawing (George 2026-11-28): it shrinks to a bar at the bottom right
//! of the work area, the cursor says what is being placed, and once it
//! is placed (or the tool ends) the window comes back where it stood.

use super::{CreaBimApp, Tool};
use eframe::egui;

/// A window stepped aside.
pub(crate) struct Parked {
    /// The window's own key ("floor.heat").
    pub window: &'static str,
    pub title: String,
    /// What is being placed.
    pub what: String,
    /// The tool placing it, and the model as it was.
    pub tool: Tool,
    pub revision: u64,
}

impl CreaBimApp {
    /// Steps `window` aside while the tool just set places `what`.
    pub(super) fn park(&mut self, window: &'static str, title: String, what: String) {
        self.parked = Some(Parked {
            window,
            title,
            what,
            tool: self.plan.tool,
            revision: self.doc.model().revision(),
        });
    }

    /// Once a frame: brings the window back once something is placed or
    /// the tool ends; else labels the cursor with what is placed.
    pub(super) fn parked_tick(&mut self, ctx: &egui::Context) {
        let Some(p) = &self.parked else { return };
        let placed = self.doc.model().revision() != p.revision;
        let ended = self.plan.tool != p.tool;
        if placed || ended {
            if !ended {
                self.plan.cancel();
                self.plan.tool = Tool::Select;
            }
            self.parked = None;
            return;
        }
        let Some(pos) = ctx.pointer_hover_pos() else {
            return;
        };
        // over the views only, not over a window or a bar
        if ctx
            .layer_id_at(pos)
            .is_some_and(|l| l.order != egui::Order::Background)
        {
            return;
        }
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::new("parked.cursor"),
        ));
        let font = egui::FontId::proportional(13.0);
        let text = p.what.clone();
        let galley = painter.layout_no_wrap(text, font, ctx.global_style().visuals.text_color());
        let at = pos + egui::vec2(18.0, 18.0);
        let rect = egui::Rect::from_min_size(at, galley.size()).expand(4.0);
        let v = &ctx.global_style().visuals;
        painter.rect(
            rect,
            3.0,
            v.window_fill,
            egui::Stroke::new(1.0, v.window_stroke.color),
            egui::StrokeKind::Outside,
        );
        painter.galley(at, galley, v.text_color());
    }

    /// The bar of `window` while it stands aside; true when it does (the
    /// window itself is not drawn then).
    pub(super) fn parked_bar(&mut self, ctx: &egui::Context, window: &'static str) -> bool {
        let Some(p) = self.parked.as_ref().filter(|p| p.window == window) else {
            return false;
        };
        let t = crate::i18n::tr;
        let room = super::shell::window_room(ctx);
        let (title, what) = (p.title.clone(), p.what.clone());
        let mut back = false;
        egui::Area::new(egui::Id::new(("parked.bar", window)))
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::RIGHT_BOTTOM)
            .fixed_pos(room.right_bottom() - egui::vec2(8.0, 8.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.strong(title);
                        ui.label(format!("{}: {what}", t("Placing")));
                        if ui
                            .button(t("Back to the window"))
                            .on_hover_text(t("Ends placing (Esc)"))
                            .clicked()
                        {
                            back = true;
                        }
                    });
                });
            });
        if back {
            self.plan.cancel();
            self.plan.tool = Tool::Select;
            self.parked = None;
        }
        true
    }
}
