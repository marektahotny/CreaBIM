//! The parameters of the fire calculations: the norms' tables kept with
//! the project (`creabim_core::fireparams`), in a window where a set is
//! read from the designer's file, every value can be changed after a
//! norm changes, a changed value shows what the file gave and goes back
//! to it, and the set is saved to a file for other projects or kept in
//! the application, where every project finds it (a project keeps only
//! the tables it changes).

use super::CreaBimApp;
use creabim_core::fireparams::{
    self, ParamTable, FIRE_FIXED_LOAD, FIRE_LOAD, FIRE_OPENING_K, FIRE_OPENING_K_LARGE, FIRE_RISK,
    OCCUPANCY,
};
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
        FIRE_LOAD => crate::i18n::t("Fire load of the spaces").to_string(),
        FIRE_FIXED_LOAD => crate::i18n::t("Fixed fire load").to_string(),
        FIRE_OPENING_K => crate::i18n::t("Factor k of the openings").to_string(),
        FIRE_OPENING_K_LARGE => crate::i18n::t("Factor k of the openings, large zones").to_string(),
        FIRE_RISK => crate::i18n::t("Fire risk factors").to_string(),
        "fire_distance" => crate::i18n::t("Separation distances, settings").to_string(),
        "fire_distance_k" => crate::i18n::t("Factors k10 and k11 of the opening").to_string(),
        "fire_pipe_d1" => crate::i18n::t("Distance d1 from the pipe outlet").to_string(),
        "fire_distance_d" => crate::i18n::t("Separation distance d (table 3)").to_string(),
        "fire_distance_dwelling" => {
            crate::i18n::t("Separation distance of dwellings (table 6)").to_string()
        }
        "fire_distance_lodging" => {
            crate::i18n::t("Separation distance of lodging (table 7)").to_string()
        }
        "fire_distance_single" => {
            crate::i18n::t("Separation distance of a single opening (table 4)").to_string()
        }
        "fire_grade" => crate::i18n::t("Fire safety grade, settings").to_string(),
        "fire_grade_nonprod" => {
            crate::i18n::t("Fire safety grade, non-production buildings").to_string()
        }
        "fire_grade_dwelling" => crate::i18n::t("Fire safety grade, dwellings").to_string(),
        "fire_roof" => crate::i18n::t("Fire resistance of the roof").to_string(),
        fireparams::CLIMATE_SITES => crate::i18n::t("Localities").to_string(),
        fireparams::SUMMER_MAX => crate::i18n::t("Summer outdoor temperatures").to_string(),
        fireparams::RAIN_INTENSITY => crate::i18n::t("Rain intensity").to_string(),
        "n50_ranges" => crate::i18n::t("Air tightness n50").to_string(),
        fireparams::ROOM_TYPE_USES => {
            crate::i18n::t("Room types for the fire calculations").to_string()
        }
        other => other.to_string(),
    };
    if own(t) && !fireparams::app_tables().is_empty() {
        format!("{what} ({}) · {}", t.norm, crate::i18n::t("project's own"))
    } else {
        format!("{what} ({})", t.norm)
    }
}

/// Whether the project keeps `t` itself: the application has no table
/// with its id, or one with other values.
fn own(t: &ParamTable) -> bool {
    !fireparams::app_tables().iter().any(|a| a == t)
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
                .map(|(_, p)| fireparams::effective(&p.fire_params))
                .unwrap_or_else(|| fireparams::app_tables().to_vec());
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
        let mut keep = false;
        let mut app_only = false;
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
                ui.label(
                    egui::RichText::new(t(
                        "Save in the application keeps the tables on this computer: every project, new or old, uses them, and a project keeps only the tables changed in it.",
                    ))
                    .weak(),
                );
                ui.horizontal(|ui| {
                    load = ui.button(t("Read from a file…")).clicked();
                    save = ui
                        .add_enabled(!d.tables.is_empty(), egui::Button::new(t("Save to a file…")))
                        .clicked();
                    keep = ui
                        .add_enabled(
                            !d.tables.is_empty(),
                            egui::Button::new(t("Save in the application")),
                        )
                        .on_hover_text(t(
                            "Every project uses these tables; a project keeps only the tables changed in it.",
                        ))
                        .clicked();
                    let app = fireparams::app_tables();
                    let owns = d.tables.iter().filter(|x| own(x)).count();
                    if !app.is_empty() && owns > 0 {
                        app_only = ui
                            .button(t("Use the application's tables"))
                            .on_hover_text(t(
                                "Drops the project's own tables: the project uses the ones saved in the application.",
                            ))
                            .clicked();
                    }
                    if !app.is_empty() {
                        ui.label(
                            egui::RichText::new(crate::i18n::tr(&format!(
                                "In the application: {} tables",
                                app.len()
                            )))
                            .weak(),
                        );
                    }
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
                    // a fixed height: the window does not grow with each frame,
                    // and Apply stays in sight
                    .max_height((area.height() - 360.0).clamp(140.0, 520.0))
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
            if let Some(path) = crate::files::file_dialog()
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
            if let Some(path) = crate::files::file_dialog()
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
        if keep {
            match crate::files::app_tables_path() {
                Some(path) => {
                    let _ = std::fs::create_dir_all(path.parent().unwrap_or(&path));
                    match std::fs::write(&path, fireparams::write(&d.tables)) {
                        Ok(()) => {
                            fireparams::set_app_tables(d.tables.clone());
                            apply = true;
                        }
                        Err(e) => {
                            self.status =
                                crate::i18n::tr(&format!("The file could not be saved: {e}"))
                        }
                    }
                }
                None => {
                    self.status =
                        crate::i18n::tr("The application's settings folder could not be found.")
                }
            }
        }
        if app_only {
            d.tables = fireparams::app_tables().to_vec();
            d.shown = 0;
            apply = true;
        }
        if apply {
            // the project keeps only what differs from the application's
            let tables: Vec<ParamTable> = d.tables.iter().filter(|x| own(x)).cloned().collect();
            let mut info = self
                .doc
                .model()
                .project_info()
                .map(|(_, p)| p.clone())
                .unwrap_or_default();
            info.fire_params = tables.clone();
            // an open Project information window keeps its own copy
            if let Some(p) = self.project_draft.as_mut() {
                p.fire_params = tables;
            }
            let same = self
                .doc
                .model()
                .project_info()
                .is_some_and(|(_, p)| p.fire_params == info.fire_params);
            let r = if same {
                Ok(())
            } else {
                let tx = creabim_core::set_project(self.doc.model(), info);
                self.doc.commit(tx).map_err(|e| e.to_string())
            };
            let said = if keep {
                "Tables saved in the application: every project uses them now."
            } else {
                "Fire calculation parameters applied: the schedules count with them now."
            };
            self.report(r, said.to_string());
        }
        self.fire_params = open.then_some(d);
    }
}
