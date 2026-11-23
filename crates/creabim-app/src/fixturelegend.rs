//! The legend of sanitary fixtures: each type placed with its mark, what
//! it is, its trap and its connection
//! ([`creabim_services::fixturespec`]), and how many there are.

use super::CreaBimApp;
use creabim_services::fixturespec;
use eframe::egui::{self, Vec2};

impl CreaBimApp {
    /// `calc:fixtures` from the Plumbing tab: show the window.
    pub(super) fn fixture_legend_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" || arg != "fixtures" {
            return false;
        }
        self.fixture_legend = true;
        true
    }

    /// The legend's window, while it is open.
    pub(super) fn fixture_legend_window(&mut self, ctx: &egui::Context) {
        if !self.fixture_legend {
            return;
        }
        let t = crate::i18n::tr;
        let rows = fixturespec::legend(self.doc.model(), None);
        let mut open = true;
        let mut select: Option<Vec<creabim_core::ElementId>> = None;
        let mut copy = false;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Legend of sanitary fixtures"))
            .id(egui::Id::new("fixtures.legend"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(320.0, 220.0))
            .default_size(crate::app::shell::at_least(Vec2::new(640.0, 440.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                if rows.is_empty() {
                    ui.label(t(
                        "No sanitary fixtures are placed yet: there is nothing to list.",
                    ));
                    return;
                }
                ui.horizontal(|ui| {
                    if ui.button(t("Copy as text")).clicked() {
                        copy = true;
                    }
                    ui.weak(t("The usual descriptions of plumbing drawings; a click on a mark picks its fixtures."));
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("fixtures.legend.grid")
                        .num_columns(3)
                        .striped(true)
                        .spacing([14.0, 6.0])
                        .show(ui, |ui| {
                            ui.strong(t("Mark"));
                            ui.strong(t("Fixture"));
                            ui.strong(t("Pieces"));
                            ui.end_row();
                            for r in &rows {
                                if ui.link(&r.spec.mark).clicked() {
                                    select = Some(r.fixtures.clone());
                                }
                                ui.vertical(|ui| {
                                    for l in fixturespec::legend_lines(&r.spec) {
                                        ui.label(l);
                                    }
                                });
                                ui.label(r.fixtures.len().to_string());
                                ui.end_row();
                            }
                        });
                });
            });
        if copy {
            let text: Vec<String> = rows
                .iter()
                .map(|r| {
                    format!(
                        "{}\t{}\t{} ks",
                        r.spec.mark,
                        fixturespec::legend_lines(&r.spec).join("\n\t"),
                        r.fixtures.len()
                    )
                })
                .collect();
            ctx.copy_text(text.join("\n"));
            self.status = t("Legend copied.");
        }
        if let Some(ids) = select {
            self.selection.clear();
            self.selection.extend(ids, true);
        }
        if !open {
            self.fixture_legend = false;
        }
    }
}
