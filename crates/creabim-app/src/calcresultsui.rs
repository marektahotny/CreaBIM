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

/// The button that writes a calculation's results to the model.
pub(super) fn write_button(ui: &mut egui::Ui) -> bool {
    ui.button(crate::i18n::tr("Write the results to the model"))
        .on_hover_text(crate::i18n::tr(
            "Keeps what the calculation found on the rooms, pipes and devices: \
             it shows in their properties and goes to IFC.",
        ))
        .clicked()
}

impl CreaBimApp {
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

#[cfg(test)]
mod tests {
    #[test]
    fn today_is_a_date() {
        let t = super::today();
        assert_eq!(t.len(), 10);
        assert!(t.starts_with("20"));
    }
}
