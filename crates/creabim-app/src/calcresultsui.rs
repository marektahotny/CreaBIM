//! The calculations' results kept with the model
//! ([`creabim_core::results`]): each calculation writes what it found on
//! the rooms, pipes, valves and circuits when it runs and from its
//! window's *Write the results to the model*; the element's properties
//! show them and the IFC export takes them along.

use super::CreaBimApp;
use creabim_core::results::{self, ResultSet};
use eframe::egui;

/// Today's date, "YYYY-MM-DD" (UTC).
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs()) as i64;
    // days to a civil date (Howard Hinnant's algorithm)
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The button that writes a calculation's results to the model, off
/// while the project already holds these very results with these
/// settings (George 2026-11-27: reopened and unchanged, there is nothing
/// to write).
pub(super) fn write_button(ui: &mut egui::Ui, written: bool) -> bool {
    let t = crate::i18n::tr;
    let b = ui.add_enabled(!written, egui::Button::new(t("Write the results to the model")));
    if written {
        b.on_disabled_hover_text(t(
            "These results, with these settings, are already written in the project; \
             change a setting or the model to write them again.",
        ))
        .clicked()
    } else {
        b.on_hover_text(t(
            "Keeps what the calculation found on the rooms, pipes and devices: \
             it shows in their properties and goes to IFC.",
        ))
        .clicked()
    }
}

impl CreaBimApp {
    /// The settings calculation `calc` was last written to the project
    /// with, for its window opened again.
    pub(super) fn saved_settings<T: serde::de::DeserializeOwned>(&self, calc: &str) -> Option<T> {
        results::settings_of(self.doc.model(), calc)
    }

    /// The project already holds `set` (same results, same settings).
    pub(super) fn is_written(&self, set: &ResultSet) -> bool {
        results::is_written(self.doc.model(), set)
    }

    /// What the heat loss is calculated with: its open window's, else
    /// what it was last written with, else the locality's defaults.
    pub(super) fn heat_opts(&self) -> creabim_services::heatloss::Opts {
        match &self.heat_loss {
            Some(x) => x.0,
            None => self
                .saved_settings("heatloss")
                .unwrap_or_else(|| self.site_opts(Default::default())),
        }
    }

    /// Keep `set` in the model in place of what the same calculation
    /// wrote before; nothing when it found the same.
    pub(super) fn write_results(&mut self, set: ResultSet) {
        let tx = results::write(self.doc.model(), set, &today());
        if tx.ops.is_empty() {
            return;
        }
        if let Err(e) = self.doc.commit_merged(tx) {
            self.status = e.to_string();
        }
    }

    /// The same, saying so in the status bar.
    pub(super) fn write_results_said(&mut self, set: ResultSet) {
        let n = set.elements.len();
        self.write_results(set);
        self.status = format!("{} ({n})", crate::i18n::tr("Results written to the model"));
    }
}

/// The results the calculations wrote for `id`, for its properties:
/// each calculation's title and its values, as text.
pub fn of_element(
    model: &creabim_core::Model,
    id: creabim_core::ElementId,
) -> Vec<(String, Vec<(String, String)>)> {
    results::of(model, id)
        .into_iter()
        .map(|(set, values)| {
            let mut head = set.title.clone();
            if !set.at.is_empty() {
                head = format!("{head} ({})", set.at);
            }
            (
                head,
                values
                    .iter()
                    .map(|v| (v.label.clone(), v.shown()))
                    .collect(),
            )
        })
        .collect()
}

/// A calculation's values in the properties, over the panel's whole
/// width: the label on the left wrapping between words, the value with
/// its unit right-aligned (George 2026-11-24: a grid squeezed the labels
/// into a narrow column, breaking words, the right half empty).
pub(super) fn rows(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash + std::fmt::Debug,
    rows: &[(String, String)],
) {
    ui.push_id(salt, |ui| {
        let full = ui.available_width();
        let gap = ui.spacing().item_spacing.x;
        // the value's column: as wide as the widest value, at most 40 %
        let widest = rows
            .iter()
            .map(|(_, v)| {
                ui.painter()
                    .layout_no_wrap(
                        v.clone(),
                        egui::TextStyle::Body.resolve(ui.style()),
                        egui::Color32::WHITE,
                    )
                    .size()
                    .x
            })
            .fold(0.0_f32, f32::max);
        let vw = (widest + 6.0).min(full * 0.4);
        let lw = (full - vw - gap).max(40.0);
        let stripe = ui.visuals().faint_bg_color;
        for (n, (k, v)) in rows.iter().enumerate() {
            let bg = ui.painter().add(egui::Shape::Noop);
            let r = ui
                .horizontal_top(|ui| {
                    ui.set_width(full);
                    ui.allocate_ui_with_layout(
                        egui::vec2(lw, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(lw);
                            ui.add(egui::Label::new(k.as_str()).wrap());
                        },
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        ui.add(egui::Label::new(v.as_str()).truncate());
                    });
                })
                .response;
            if n % 2 == 1 {
                ui.painter().set(
                    bg,
                    egui::Shape::rect_filled(r.rect.expand2(egui::vec2(2.0, 1.0)), 2.0, stripe),
                );
            }
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn today_is_a_date() {
        let t = super::today();
        assert_eq!(t.len(), 10);
        assert!(t.starts_with("20"));
    }
}
