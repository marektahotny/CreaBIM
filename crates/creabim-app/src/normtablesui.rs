//! The norms' tables in one place (Manage › Norm tables): every table
//! file of a folder read at once into the project and the application,
//! which tables each check needs and which are there, and the folder
//! read again when the tables are updated (George 2026-11-24: they were
//! read in the fire parameters, the heat loss, the railings check, each
//! on its own).

use super::CreaBimApp;
use creabim_core::fireparams::{self, ParamTable};
use eframe::egui::{self, Vec2};

/// The window's state: the files the tables were read from last.
#[derive(Clone, Debug, Default)]
pub struct NormTables {
    /// Each table read, by its id: the file it came from.
    pub from: Vec<(String, String)>,
}

/// What each check needs: its norm and the tables' ids.
fn groups() -> Vec<(&'static str, Vec<&'static str>)> {
    use creabim_core::{railcheck, staircheck};
    use creabim_services::{coolnorm, heatdetail, psi};
    let mut heat: Vec<&str> = heatdetail::TABLES.to_vec();
    heat.push(psi::PSI);
    heat.push(creabim_core::room::ROOF_SPACE);
    vec![
        (
            "Fire safety (STN 92 0201, STN 92 0241)",
            vec![
                fireparams::OCCUPANCY,
                fireparams::FIRE_LOAD,
                fireparams::FIRE_FIXED_LOAD,
                fireparams::FIRE_OPENING_K,
                fireparams::FIRE_OPENING_K_LARGE,
                fireparams::FIRE_RISK,
                "fire_closure",
                "fire_distance",
                "fire_distance_d",
                "fire_distance_dwelling",
                "fire_distance_k",
                "fire_distance_lodging",
                "fire_distance_single",
                "fire_exposed",
                "fire_exposed_wall",
                "fire_grade",
                "fire_grade_dwelling",
                "fire_grade_nonprod",
                "fire_outer_wall",
                "fire_pipe_d1",
                "fire_roof",
                fireparams::ROOM_TYPE_USES,
            ],
        ),
        (
            "Heat loss (STN EN 12831-1, STN EN ISO 14683, STN EN ISO 6946)",
            heat,
        ),
        ("Cooling load (STN 73 0548)", coolnorm::TABLES.to_vec()),
        (
            "Underfloor heating (STN EN 1264-2, STN EN 1264-4)",
            creabim_services::ufhnorm::TABLES.to_vec(),
        ),
        (
            "Climate and localities",
            fireparams::CLIMATE_TABLES.to_vec(),
        ),
        (
            "Thermal field of details (STN EN ISO 10211, STN 73 0540-2, -3, STN EN ISO 13788)",
            creabim_services::thermcheck::TABLES.to_vec(),
        ),
        ("Railings (STN 74 3305)", railcheck::TABLES.to_vec()),
        (
            "Stairs (STN 73 4130, STN 73 4301)",
            staircheck::TABLES.to_vec(),
        ),
    ]
}

/// The file keeping the folder the tables were read from.
fn folder_file() -> Option<std::path::PathBuf> {
    crate::files::settings_dir().map(|d| d.join("tabulky-noriem-priecinok.txt"))
}

/// The folder the tables were read from last, if any.
pub fn saved_folder() -> Option<std::path::PathBuf> {
    let text = std::fs::read_to_string(folder_file()?).ok()?;
    let p = std::path::PathBuf::from(text.trim());
    (!text.trim().is_empty()).then_some(p)
}

/// Every table in the `.tsv` and `.txt` files of `dir`, with the file's
/// name; a file with no table (notes) adds none.
pub fn read_folder(dir: &std::path::Path) -> std::io::Result<Vec<(ParamTable, String)>> {
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|x| x.to_str())
                    .is_some_and(|x| x.eq_ignore_ascii_case("tsv") || x.eq_ignore_ascii_case("txt"))
        })
        .collect();
    // the newest file last, so its tables win: a copy saved later under a
    // new name (with a time stamp) holds the current values
    files.sort_by_key(|f| {
        (
            std::fs::metadata(f).and_then(|m| m.modified()).ok(),
            f.clone(),
        )
    });
    let mut out = vec![];
    for f in files {
        let Ok(text) = std::fs::read_to_string(&f) else {
            continue;
        };
        let name = f
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        for t in fireparams::parse(&text) {
            out.push((t, name.clone()));
        }
    }
    Ok(out)
}

impl CreaBimApp {
    /// `norms:tables` from the Manage tab: open the window.
    pub(super) fn norm_tables_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "norms" || arg != "tables" {
            return false;
        }
        if self.norm_tables.is_none() {
            self.norm_tables = Some(NormTables::default());
        }
        true
    }

    /// Read every table of `dir` into the project and the application's
    /// tables (for new projects), the localities also where the heat loss
    /// keeps them.
    fn read_norm_folder(&mut self, dir: &std::path::Path) {
        let t = crate::i18n::tr;
        let found = match read_folder(dir) {
            Ok(f) => f,
            Err(e) => {
                self.status = t(&format!("The folder could not be read: {e}"));
                return;
            }
        };
        if found.is_empty() {
            self.status =
                t("No norm table found in the folder: a table starts with a line \"#table\".");
            return;
        }
        if let Some(f) = folder_file() {
            let _ = std::fs::create_dir_all(f.parent().unwrap_or(&f));
            let _ = std::fs::write(&f, dir.to_string_lossy().as_bytes());
        }
        let climate: Vec<ParamTable> = found
            .iter()
            .filter(|(x, _)| fireparams::CLIMATE_TABLES.contains(&x.id.as_str()))
            .map(|(x, _)| x.clone())
            .collect();
        if !climate.is_empty() {
            if let Some(p) = crate::files::settings_dir().map(|d| d.join("lokality.tsv")) {
                let _ = std::fs::create_dir_all(p.parent().unwrap_or(&p));
                let _ = std::fs::write(&p, fireparams::write(&climate));
            }
        }
        if let Some(st) = self.norm_tables.as_mut() {
            st.from = found
                .iter()
                .map(|(x, f)| (x.id.clone(), f.clone()))
                .collect();
        }
        let n = found.len();
        self.load_heat_tables(found.into_iter().map(|(x, _)| x).collect());
        self.status = t(&format!(
            "{n} tables read from {}: kept with the project and for new projects.",
            dir.display()
        ));
    }

    /// The window, while it is open.
    pub(super) fn norm_tables_window(&mut self, ctx: &egui::Context) {
        let Some(st) = self.norm_tables.clone() else {
            return;
        };
        let t = crate::i18n::tr;
        let mut open = true;
        let mut pick = false;
        let mut again = false;
        let mut params = false;
        let folder = saved_folder();
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        let own: Vec<ParamTable> = self
            .doc
            .model()
            .project_info()
            .map(|(_, p)| p.fire_params.clone())
            .unwrap_or_default();
        let app = fireparams::app_tables();
        super::shell::window(t("Norm tables"))
            .id(egui::Id::new("norm.tables"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(360.0, 300.0))
            .default_size(crate::app::shell::at_least(Vec2::new(720.0, 600.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                ui.set_max_width(820.0);
                ui.label(
                    egui::RichText::new(t(
                        "The program carries no norm values of its own. Keep the table files written from your copies of the norms in one folder and read it here: every table is read at once, kept with the project and for new projects. When a norm or its table changes, change the file and read the folder again.",
                    ))
                    .weak(),
                );
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(t("Folder:"));
                    ui.label(
                        egui::RichText::new(
                            folder
                                .as_ref()
                                .map(|p| p.display().to_string())
                                .unwrap_or_else(|| t("none read yet")),
                        )
                        .strong(),
                    );
                });
                ui.horizontal(|ui| {
                    pick = ui.button(t("Read a folder of tables…")).clicked();
                    if ui
                        .add_enabled(folder.is_some(), egui::Button::new(t("Read the folder again")))
                        .on_hover_text(t("After the table files were changed: reads them all again"))
                        .clicked()
                    {
                        again = true;
                    }
                    params = ui
                        .button(t("Edit values…"))
                        .on_hover_text(t("The tables' values, to change one by hand"))
                        .clicked();
                });
                ui.separator();
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    for (title, ids) in groups() {
                        let have = ids
                            .iter()
                            .filter(|id| {
                                fireparams::find(&own, id).is_some()
                                    || app.iter().any(|x| x.id == **id)
                            })
                            .count();
                        let all = have == ids.len();
                        let head = format!("{}  ({have}/{})", t(title), ids.len());
                        egui::CollapsingHeader::new(if all {
                            egui::RichText::new(head)
                        } else {
                            egui::RichText::new(head).color(ui.visuals().warn_fg_color)
                        })
                        .id_salt(title)
                        .default_open(!all)
                        .show(ui, |ui| {
                            egui::Grid::new(("norm.tables.grid", title))
                                .num_columns(4)
                                .striped(true)
                                .spacing([16.0, 4.0])
                                .show(ui, |ui| {
                                    ui.label(egui::RichText::new(t("Table")).strong());
                                    ui.label(egui::RichText::new(t("Norm and edition")).strong());
                                    ui.label(egui::RichText::new(t("State")).strong());
                                    ui.label(egui::RichText::new(t("File")).strong());
                                    ui.end_row();
                                    for id in ids {
                                        let in_project = fireparams::find(&own, id);
                                        let in_app = app.iter().find(|x| x.id == id);
                                        let table = in_project.or(in_app);
                                        ui.label(id);
                                        ui.label(table.map_or(String::new(), |x| {
                                            format!("{} {}", x.norm, x.edition).trim().to_string()
                                        }));
                                        match (in_project, in_app) {
                                            (Some(_), _) => ui.label(t("in the project")),
                                            (None, Some(_)) => ui.label(t("in the application")),
                                            (None, None) => ui.colored_label(
                                                ui.visuals().error_fg_color,
                                                t("missing"),
                                            ),
                                        };
                                        ui.label(
                                            st.from
                                                .iter()
                                                .find(|(i, _)| i == id)
                                                .map_or("", |(_, f)| f.as_str()),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });
                    }
                });
            });
        if pick {
            let mut dialog = crate::files::file_dialog().set_title(t("Folder of norm tables"));
            if let Some(f) = &folder {
                dialog = dialog.set_directory(f);
            }
            if let Some(dir) = dialog.pick_folder() {
                self.read_norm_folder(&dir);
            }
        }
        if again {
            if let Some(dir) = folder {
                self.read_norm_folder(&dir);
            }
        }
        if params {
            self.open_fire_params();
        }
        if !open {
            self.norm_tables = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_is_read_whole_its_notes_skipped() {
        let dir = std::env::temp_dir().join(format!("creabim-norms-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(
            dir.join("b-stairs.tsv"),
            "#table stair_classes\n#columns class\tmax\nA\t1\n#table stair_walk\n#columns k\tv\nx\t2\n",
        )
        .unwrap();
        std::fs::write(dir.join("a-notes.tsv"), "# only notes\n1\tADMINISTRATÍVA\n").unwrap();
        std::fs::write(dir.join("c.md"), "#table not_read\n").unwrap();
        let found = read_folder(&dir).unwrap();
        let ids: Vec<(&str, &str)> = found
            .iter()
            .map(|(t, f)| (t.id.as_str(), f.as_str()))
            .collect();
        assert_eq!(
            ids,
            [
                ("stair_classes", "b-stairs.tsv"),
                ("stair_walk", "b-stairs.tsv")
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
        // every check's tables are listed once
        let all: Vec<&str> = groups().into_iter().flat_map(|(_, ids)| ids).collect();
        let mut dedup = all.clone();
        dedup.sort();
        dedup.dedup();
        assert_eq!(all.len(), dedup.len());
    }
}
