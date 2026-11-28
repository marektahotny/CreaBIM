//! Calculation windows step aside while what they add is placed in the
//! drawing (George 2026-11-28): the open windows shrink to tabs at the
//! bottom right of the work area (as for the heat source, see
//! `heatplace`), the cursor says what is being placed, and once it is
//! placed (or the tool ends) the windows come back where they stood, in
//! their order.

use super::heatplace::{self, Windows};
use super::{CreaBimApp, Tool};
use eframe::egui;

/// The windows stepped aside.
pub(crate) struct Parked {
    /// What is being placed.
    pub what: String,
    /// The tool placing it, and the model as it was.
    pub tool: Tool,
    pub revision: u64,
    /// Read from the first frame on.
    windows: Option<Windows>,
}

impl CreaBimApp {
    /// Steps the windows aside while the tool just set places `what`.
    pub(super) fn park(&mut self, what: String) {
        self.parked = Some(Parked {
            what,
            tool: self.plan.tool,
            revision: self.doc.model().revision(),
            windows: None,
        });
    }

    /// Once a frame, after the windows: tucks them away, brings them back
    /// once something is placed or the tool ends; else labels the cursor
    /// with what is placed.
    pub(super) fn parked_tick(&mut self, ctx: &egui::Context) {
        let Some(mut p) = self.parked.take() else {
            return;
        };
        let placed = self.doc.model().revision() != p.revision;
        let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        let ended = self.plan.tool != p.tool;
        if placed || ended || esc {
            if !ended {
                self.plan.cancel();
                self.plan.tool = Tool::Select;
            }
            if let Some(w) = &p.windows {
                self.untuck(ctx, w);
            }
            return;
        }
        let w = p
            .windows
            .get_or_insert_with(|| heatplace::begin_windows(ctx));
        heatplace::read_windows(ctx, w);
        if let Windows::Tucked(t) = w {
            heatplace::tabs(
                ctx,
                t,
                "Shows the window for a while; it comes back where it was once placed.",
            );
        }
        heatplace::pointer_note(ctx, &format!("{}: {}", crate::i18n::t("Placing"), p.what));
        self.parked = Some(p);
    }
}
