//! The parameters of the fire calculations: the norms' tables kept with
//! the project (`creabim_core::fireparams`), in a window where a set is
//! read from the designer's file, every value can be changed after a
//! norm changes, a changed value shows what the file gave and goes back
//! to it, and the set is saved to a file for other projects.

use super::CreaBimApp;
use creabim_core::fireparams::{self, ParamTable, OCCUPANCY};
use eframe::egui::{self, Vec2};

/// The tables as they are edited, and the one shown.
#[derive(Clone, Debug, Default)]
pub(super) struct FireParamsDraft {
    pub tables: Vec<ParamTable>,
    pub shown: usize,
}

/// What a table is for, as the window names it.
fn title(t: &ParamTable) -> String {
    let what = match t.id.as_str() {
        OCCUPANCY => crate::i18n::t("Occupancy by persons").to_string(),
        other => other.to_string(),
    };
    format!("{what} ({})", t.norm)
}

impl CreaBimApp {
    /// `fire:params`: open the window on the project's tables.
    pub(super) fn fire_params_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "fire" || arg != "params" {
            return false;
        }
        self.open_fire_params();
        true
    }

    pub(super) fn open_fire_params(&mut self) {
        if self.fire_params.is_none() {
            let tables = self
                .doc
                .model()
                .project_info()
                .map(|(_, p)| p.fire_params.clone())
                .unwrap_or_default();
            self.fire_params = Some(FireParamsDraft { tables, shown: 0 });
        }
    }

    /// The window, while it is open.
    pub(super) fn fire_params_window(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.fire_params.take() else {
            return;
        };
        let t = crate::i18n::t;
        let mut open = true;
        let mut apply = false;
        let mut load = false;
        let mut save = false;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(crate::i18n::tr("Fire calculation parameters"))
            .id(egui::Id::new("fire.params"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(380.0, 280.0))
            .default_size(Vec2::new(760.0, 560.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(t(
                        "The program carries no norm values of its own. Read the parameter file written from your copy of the norms; change a value here when a norm changes. The tables are saved with the project.",
                    ))
                    .weak(),
                );
                ui.horizontal(|ui| {
                    load = ui.button(t("Read from a file…")).clicked();
                    save = ui
                        .add_enabled(!d.tables.is_empty(), egui::Button::new(t("Save to a file…")))
                        .clicked();
                });
                if d.tables.is_empty() {
                    ui.add_space(8.0);
                    ui.label(t("No tables yet."));
                    return;
                }
                d.shown = d.shown.min(d.tables.len() - 1);
                ui.horizontal_wrapped(|ui| {
                    for (i, table) in d.tables.iter().enumerate() {
                        ui.selectable_value(&mut d.shown, i, title(table));
                    }
                });
                let table = &mut d.tables[d.shown];
                ui.horizontal(|ui| {
                    ui.label(t("Norm"));
                    ui.add(egui::TextEdit::singleline(&mut table.norm).desired_width(160.0));
                    ui.label(t("Edition"));
                    ui.add(egui::TextEdit::singleline(&mut table.edition).desired_width(160.0));
                    let n = table.changed.len();
                    if n > 0 {
                        ui.colored_label(
                            ui.visuals().warn_fg_color,
                            crate::i18n::tr(&format!("{n} values changed")),
                        );
                        if ui.button(t("Reset all")).clicked() {
                            table.reset(None);
                        }
                    }
                });
                ui.separator();
                let columns = table.columns.len();
                let widths: Vec<f32> = table
                    .columns
                    .iter()
                    .map(|c| match c.as_str() {
                        "item" => 60.0,
                        "name" => 260.0,
                        "note" => 300.0,
                        _ => 150.0,
                    })
                    .collect();
                let warn = ui.visuals().warn_fg_color;
                let mut set: Option<(usize, usize, String)> = None;
                let mut reset: Option<(usize, usize)> = None;
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .max_height(ui.available_height() - 40.0)
                    .show(ui, |ui| {
                        egui::Grid::new("fire.params.grid")
                            .num_columns(columns)
                            .striped(true)
                            .show(ui, |ui| {
                                for c in &table.columns {
                                    ui.strong(crate::i18n::tr(c));
                                }
                                ui.end_row();
                                for (r, row) in table.rows.iter().enumerate() {
                                    for (c, cell) in row.iter().enumerate() {
                                        let was = table.was(r, c);
                                        let mut v = cell.clone();
                                        let mut edit = egui::TextEdit::singleline(&mut v)
                                            .desired_width(widths[c]);
                                        if was.is_some() {
                                            edit = edit.text_color(warn);
                                        }
                                        let mut resp = ui.add(edit);
                                        if let Some(w) = was {
                                            resp = resp.on_hover_text(format!(
                                                "{}: {w}. {}",
                                                t("In the file"),
                                                t("Right-click puts it back.")
                                            ));
                                            if resp.secondary_clicked() {
                                                reset = Some((r, c));
                                            }
                                        }
                                        if resp.changed() {
                                            set = Some((r, c, v));
                                        }
                                    }
                                    ui.end_row();
                                }
                            });
                    });
                if let Some((r, c, v)) = set {
                    table.set(r, c, &v);
                }
                if let Some(at) = reset {
                    table.reset(Some(at));
                }
                ui.separator();
                apply = ui.button(t("Apply")).clicked();
            });
        if load {
            if let Some(path) = rfd::FileDialog::new()
                .set_title(crate::i18n::tr("Fire calculation parameters"))
                .add_filter("TSV", &["tsv", "txt"])
                .pick_file()
            {
                match std::fs::read_to_string(&path) {
                    Ok(text) => {
                        let new = fireparams::parse(&text);
                        if new.is_empty() {
                            self.status = crate::i18n::tr(
                                "No table found in the file: a table starts with a line \"#table\" and a line \"#columns\".",
                            );
                        } else {
                            let n = new.len();
                            fireparams::merge(&mut d.tables, new);
                            self.status = crate::i18n::tr(&format!(
                                "{n} tables read. Apply keeps them with the project."
                            ));
                        }
                    }
                    Err(e) => {
                        self.status = crate::i18n::tr(&format!("The file could not be read: {e}"))
                    }
                }
            }
        }
        if save {
            if let Some(path) = rfd::FileDialog::new()
                .set_title(crate::i18n::tr("Fire calculation parameters"))
                .set_file_name("pbs-parametre.tsv")
                .add_filter("TSV", &["tsv"])
                .save_file()
            {
                self.status = match std::fs::write(&path, fireparams::write(&d.tables)) {
                    Ok(()) => crate::i18n::tr(&format!("Saved {}", path.display())),
                    Err(e) => crate::i18n::tr(&format!("The file could not be saved: {e}")),
                };
            }
        }
        if apply {
            let mut info = self
                .doc
                .model()
                .project_info()
                .map(|(_, p)| p.clone())
                .unwrap_or_default();
            info.fire_params = d.tables.clone();
            // an open Project information window keeps its own copy
            if let Some(p) = self.project_draft.as_mut() {
                p.fire_params = d.tables.clone();
            }
            let tx = creabim_core::set_project(self.doc.model(), info);
            let r = self.doc.commit(tx);
            self.report(
                r,
                "Fire calculation parameters applied: the schedules count with them now."
                    .to_string(),
            );
        }
        self.fire_params = open.then_some(d);
    }
}
