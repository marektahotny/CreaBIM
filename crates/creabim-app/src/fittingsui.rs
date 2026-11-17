//! The fittings schedule: the elbows, tees, branches, reducers and
//! transitions the pipes drawn need where they meet
//! ([`creabim_services::fittings`]), counted by system.

use super::CreaBimApp;
use creabim_services::fittings::{self, Fitting};
use eframe::egui::{self, Vec2};

impl CreaBimApp {
    /// `calc:fittings` from the Plumbing tab: find them and show the window.
    pub(super) fn fittings_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" || arg != "fittings" {
            return false;
        }
        let found = fittings::derive(self.doc.model());
        self.status = if found.is_empty() {
            crate::i18n::tr("No pipes meet yet: there are no fittings to list.")
        } else {
            format!("{}: {}", crate::i18n::tr("Fittings"), found.len())
        };
        self.fittings = Some(found);
        true
    }

    /// The schedule's window, while it is open.
    pub(super) fn fittings_window(&mut self, ctx: &egui::Context) {
        let Some(found) = &self.fittings else {
            return;
        };
        let t = crate::i18n::tr;
        let mut open = true;
        let mut again = false;
        let mut select: Option<Vec<creabim_core::ElementId>> = None;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Fittings schedule"))
            .id(egui::Id::new("fittings.schedule"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(280.0, 220.0))
            .default_size(Vec2::new(560.0, 440.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button(t("Find again")).clicked() {
                        again = true;
                    }
                    ui.weak(t(
                        "Found from how the pipes meet; pipe that is bent takes no elbow.",
                    ));
                });
                if found.is_empty() {
                    ui.label(t("No pipes meet yet: there are no fittings to list."));
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let lines = fittings::summary(found);
                    let mut system = String::new();
                    egui::Grid::new("fittings.grid")
                        .num_columns(3)
                        .striped(true)
                        .spacing([14.0, 4.0])
                        .show(ui, |ui| {
                            for l in &lines {
                                if l.system != system {
                                    system = l.system.clone();
                                    let name = creabim_core::mep::system(&system)
                                        .map_or(system.clone(), |s| {
                                            format!("{} – {}", s.code, t(s.name))
                                        });
                                    ui.strong(name);
                                    ui.strong(t("Code"));
                                    ui.strong(t("Pieces"));
                                    ui.end_row();
                                }
                                // a click picks the pipes its fittings join
                                if ui.link(&l.label).clicked() {
                                    select = Some(
                                        found
                                            .iter()
                                            .filter(|f: &&Fitting| {
                                                f.system == l.system && f.label == l.label
                                            })
                                            .flat_map(|f| f.pipes.iter().copied())
                                            .collect(),
                                    );
                                }
                                ui.label(&l.code);
                                ui.label(l.count.to_string());
                                ui.end_row();
                            }
                        });
                });
            });
        if let Some(ids) = select {
            self.selection.clear();
            self.selection.extend(ids, true);
        }
        if again {
            self.fittings = Some(fittings::derive(self.doc.model()));
        } else if !open {
            self.fittings = None;
        }
    }
}
