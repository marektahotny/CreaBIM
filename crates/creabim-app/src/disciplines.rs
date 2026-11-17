//! The building services' disciplines (the tabs beside Architecture): a
//! window per discipline with the standards its calculations follow under
//! the project's codes (STN or ČSN, in Project information) and the tools
//! it is getting, in their order.

use super::CreaBimApp;
use creabim_core::norms::{Discipline, NormSet};
use creabim_core::objects::ObjectCategory;
use eframe::egui::{self, Vec2};

impl CreaBimApp {
    /// The codes the project follows.
    pub(super) fn norm_set(&self) -> NormSet {
        self.doc
            .model()
            .project_info()
            .map(|(_, p)| p.norms)
            .unwrap_or_default()
    }

    /// Run a discipline action. False when the action isn't one.
    pub(super) fn discipline_action(&mut self, head: &str, arg: &str) -> bool {
        match head {
            "discipline" => {
                self.discipline = Discipline::from_id(arg);
            }
            // the catalogue opened at one category: a discipline's fixtures
            "catalogue.cat" => {
                let c = match arg {
                    "bathroom" => Some(ObjectCategory::Bathroom),
                    "kitchen" => Some(ObjectCategory::Kitchen),
                    "plumbing" => Some(ObjectCategory::Plumbing),
                    "hvac" => Some(ObjectCategory::Hvac),
                    "electrical" => Some(ObjectCategory::Electrical),
                    "lighting" => Some(ObjectCategory::Lighting),
                    _ => None,
                };
                self.open_catalogue_at(c);
            }
            _ => return false,
        }
        true
    }

    /// The discipline window, while one is open.
    pub(super) fn discipline_window(&mut self, ctx: &egui::Context) {
        let Some(d) = self.discipline else {
            return;
        };
        let set = self.norm_set();
        let mut open = true;
        let mut change_codes = false;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(crate::i18n::tr(d.label()))
            .id(egui::Id::new("discipline.window"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(210.0, 200.0))
            .constrain_to(area)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.set_width(420.0);
                crate::props::section(ui, "Standards");
                crate::props::row(ui, "Codes", |ui| {
                    ui.label(crate::i18n::t(set.label()));
                    if ui
                        .small_button(crate::i18n::t("Change…"))
                        .on_hover_text(crate::i18n::t(
                            "The project's codes are set in Project information",
                        ))
                        .clicked()
                    {
                        change_codes = true;
                    }
                });
                egui::Grid::new("discipline.norms")
                    .num_columns(2)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        for n in d.norms(set) {
                            ui.strong(crate::i18n::t(n.code));
                            ui.label(crate::i18n::t(n.title));
                            ui.end_row();
                        }
                    });
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(crate::i18n::t(
                        "Check each standard's current edition before relying on a result.",
                    ))
                    .weak(),
                );
                crate::props::section(ui, "Coming to this tab");
                for p in d.planned() {
                    ui.label(format!("•  {}", crate::i18n::t(p)));
                }
            });
        if change_codes {
            self.project_open = true;
        }
        if !open {
            self.discipline = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use creabim_core::norms::{Discipline, NormSet};

    #[test]
    fn every_discipline_text_is_translated() {
        // the window shows these through the tables; a missing one would
        // read in English in a Slovak window
        let sk: std::collections::HashMap<String, String> =
            serde_json::from_str(include_str!("../i18n/sk.json")).unwrap();
        let mut missing = vec![];
        for d in Discipline::ALL {
            let mut texts = vec![d.label()];
            texts.extend(d.planned());
            for set in NormSet::ALL {
                texts.push(set.label());
                texts.extend(d.norms(set).iter().map(|n| n.title));
            }
            missing.extend(texts.into_iter().filter(|t| !sk.contains_key(*t)));
        }
        missing.sort();
        missing.dedup();
        assert!(missing.is_empty(), "not in sk.json: {missing:?}");
    }
}
