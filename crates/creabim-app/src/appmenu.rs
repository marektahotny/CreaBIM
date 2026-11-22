//! The CreaBIM application menu (About, Check for updates, Settings,
//! Exit), the Units switch in the status bar, the Settings window and the
//! New project window's choices.

use super::*;
use crate::ui_model::Item;
use creabim_core::{LengthUnit, Units};

/// The Save the changes window (REV02): where it is.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum CloseState {
    /// Save, Don't save or Cancel?
    #[default]
    Ask,
    /// Save was pressed: the button spins until the file is written.
    Saving {
        since: std::time::Instant,
        /// The file is written (or not), once tried.
        done: Option<Result<(), String>>,
        /// It failed before with this: Save as… goes elsewhere.
        retry: Option<String>,
    },
    /// Saved: the nest settles, then CreaBIM closes.
    Saved(std::time::Instant),
    /// The save failed, for this reason; nothing closes.
    Failed(String),
}

/// Save as picked a file that is there already: the app's own question,
/// Replace or not (Windows' own one could fall behind the app window).
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Overwrite {
    pub path: String,
    pub then: OverwriteThen,
}

/// What a Replace goes on with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum OverwriteThen {
    /// Save as… from the menu.
    SaveAs,
    /// Saving before closing (Save the changes).
    CloseSave,
}

/// Replace the file?: its heading, its line and the file's name.
fn overwrite_texts(path: &str) -> (String, String, String) {
    let file = std::path::Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(path)
        .to_string();
    (
        crate::i18n::tr("The file already exists. Replace it?"),
        crate::i18n::tr("What is in it now is lost; the project is written there instead."),
        file,
    )
}

/// How long Save spins at least, so it is seen, and how long "Saved."
/// holds before CreaBIM closes (the page fades out in its last 0.3 s).
const SAVING_AT_LEAST: std::time::Duration = std::time::Duration::from_millis(450);
const SAVED_HOLDS: std::time::Duration = std::time::Duration::from_millis(1200);

/// "just now", "4 minutes ago", "2 hours ago", "3 days ago".
pub(crate) fn ago(d: std::time::Duration) -> String {
    let s = d.as_secs();
    let (n, unit) = match s {
        0..60 => return "just now".into(),
        60..3600 => (s / 60, "minute"),
        3600..172_800 => (s / 3600, "hour"),
        _ => (s / 86_400, "day"),
    };
    format!("{n} {unit}{} ago", if n == 1 { "" } else { "s" })
}

/// The Save the changes window's line under its title.
pub(crate) fn changes_line(n: usize, since: Option<std::time::Duration>) -> String {
    let changes = format!("{n} change{}", if n == 1 { "" } else { "s" });
    match since {
        Some(d) => format!("{changes} since it was last saved, {}.", ago(d)),
        None => format!("{changes} since it was last saved."),
    }
}

/// Why a save failed, in plain words, from the system's message.
pub(crate) fn save_error(e: &str) -> String {
    let l = e.to_ascii_lowercase();
    let any = |words: &[&str]| words.iter().any(|w| l.contains(w));
    if any(&[
        "os error 5)",
        "os error 19)",
        "os error 30)",
        "os error 13)",
        "access is denied",
        "permission denied",
        "read-only",
        "write-protected",
    ]) {
        "The file or its folder is read-only, or CreaBIM isn't allowed to write there. Choose another folder with Save as…".into()
    } else if any(&[
        "os error 112)",
        "os error 28)",
        "no space",
        "disk is full",
        "not enough space",
    ]) {
        "The disk is full. Free some space, or choose another drive with Save as…".into()
    } else {
        format!("The file couldn't be written ({e}). Choose another folder with Save as…")
    }
}

impl CreaBimApp {
    /// The project has no file yet: Save asks where.
    fn never_saved(&self) -> bool {
        !std::path::Path::new(&self.path).exists()
    }

    /// The Save the changes window's state, for its page.
    pub(super) fn close_state_json(&self) -> serde_json::Value {
        let file = std::path::Path::new(&self.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project.creabim")
            .to_string();
        let fresh = self.never_saved()
            && !matches!(self.close_state, CloseState::Saved(_))
            && !matches!(
                &self.close_state,
                CloseState::Saving {
                    done: Some(Ok(())),
                    ..
                }
            );
        let since = std::fs::metadata(&self.path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| std::time::SystemTime::now().duration_since(t).ok());
        let (phase, error) = match &self.close_state {
            CloseState::Ask => ("ask", None),
            CloseState::Saving { retry, .. } => ("saving", retry.clone()),
            CloseState::Saved(_) => ("saved", None),
            CloseState::Failed(e) => ("failed", Some(e.clone())),
        };
        serde_json::json!({
            "phase": phase,
            "file": file,
            "fresh": fresh,
            "line": if fresh {
                "It hasn't been saved to a file yet.".to_string()
            } else {
                changes_line(self.changes_since_saved(), since)
            },
            "error": error,
        })
    }

    /// A button or key of the Save the changes window: save, discard or
    /// cancel.
    pub(super) fn close_do(&mut self, ctx: &egui::Context, what: &str) {
        crate::actlog::line(format!("close: {what}"));
        if !self.close_ask {
            return;
        }
        let busy = matches!(
            self.close_state,
            CloseState::Saving { .. } | CloseState::Saved(_)
        );
        match what {
            "save" if !busy => {
                let retry = match &self.close_state {
                    CloseState::Failed(e) => Some(e.clone()),
                    _ => None,
                };
                self.close_state = CloseState::Saving {
                    since: std::time::Instant::now(),
                    done: None,
                    retry,
                };
                ctx.request_repaint();
            }
            "discard" if !busy => self.close_now(ctx),
            "cancel" if !busy => {
                self.close_ask = false;
                self.close_state = CloseState::Ask;
                self.hide_close_dialog();
                self.status = "Close cancelled: the project is still open.".into();
            }
            _ => {}
        }
    }

    /// Let the close through: saved, or the changes let go.
    fn close_now(&mut self, ctx: &egui::Context) {
        // saved, or the changes let go on purpose: nothing to offer back
        self.autosave_clear();
        self.close_ask = false;
        self.close_state = CloseState::Ask;
        self.hide_close_dialog();
        self.close_ok = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// Saving, then saved, then closed: one step a frame.
    fn close_step(&mut self, ctx: &egui::Context) {
        let soon = std::time::Duration::from_millis(40);
        match self.close_state.clone() {
            CloseState::Saving {
                since,
                done: None,
                retry,
            } => {
                // a frame first, so the page shows Saving… before the
                // save holds the app (and a file picker opens over it)
                if since.elapsed() < soon {
                    ctx.request_repaint_after(soon);
                    return;
                }
                let r = if self.never_saved() || retry.is_some() {
                    let start = with_extension(&self.path, "creabim");
                    match crate::files::pick_save_path(&start, "Save the project as", "creabim") {
                        // a file there already: the app asks first, the
                        // question waits meanwhile
                        Some(path) if crate::files::ask_overwrite(&path) => {
                            self.close_state = retry.map_or(CloseState::Ask, CloseState::Failed);
                            self.ask_overwrite(path, OverwriteThen::CloseSave);
                            ctx.request_repaint();
                            return;
                        }
                        Some(path) => {
                            let was = std::mem::replace(&mut self.path, path);
                            let r = self.save_file();
                            if r.is_err() {
                                self.path = was;
                            }
                            Some(r)
                        }
                        // Save as cancelled: back to the question
                        None => None,
                    }
                } else {
                    Some(self.save_file())
                };
                crate::actlog::line(match &r {
                    None => "close: Save as cancelled".to_string(),
                    Some(Ok(())) => format!("close: saved {}", self.path),
                    Some(Err(e)) => format!("close: couldn't save {}: {e}", self.path),
                });
                self.close_state = match r {
                    None => retry.map_or(CloseState::Ask, CloseState::Failed),
                    Some(Err(e)) => {
                        self.status = format!("Couldn't save {}: {e}", self.path);
                        CloseState::Failed(save_error(&e))
                    }
                    Some(Ok(())) => {
                        self.status = format!("Saved {}", self.path);
                        CloseState::Saving {
                            since,
                            done: Some(Ok(())),
                            retry: None,
                        }
                    }
                };
                ctx.request_repaint();
            }
            CloseState::Saving { since, .. } => {
                if since.elapsed() >= SAVING_AT_LEAST {
                    self.close_state = CloseState::Saved(std::time::Instant::now());
                }
                ctx.request_repaint_after(soon);
            }
            CloseState::Saved(at) => {
                if at.elapsed() >= SAVED_HOLDS {
                    self.close_now(ctx);
                } else {
                    ctx.request_repaint_after(soon);
                }
            }
            _ => {}
        }
    }

    /// Save as picked `path`, a file there already: Replace or not is
    /// asked in the app's own window.
    pub(crate) fn ask_overwrite(&mut self, path: String, then: OverwriteThen) {
        crate::actlog::line(format!("save as: {path} is there, asking to replace it"));
        self.overwrite = Some(Overwrite { path, then });
    }

    /// The answer to Replace the file?: `yes` writes over it, no goes back.
    pub(crate) fn overwrite_answer(&mut self, ctx: &egui::Context, yes: bool) {
        let Some(o) = self.overwrite.take() else {
            return;
        };
        self.hide_overwrite_dialog();
        crate::actlog::line(format!(
            "save as: {} {}",
            o.path,
            if yes {
                "replaced"
            } else {
                "kept, nothing saved"
            }
        ));
        match (o.then, yes) {
            (OverwriteThen::SaveAs, true) => self.save_as_to(o.path),
            (OverwriteThen::SaveAs, false) => {
                self.status = crate::i18n::tr("Save as cancelled: the file there is kept.");
            }
            (OverwriteThen::CloseSave, true) => {
                // the file is there now: Save writes it, then CreaBIM closes
                self.path = o.path;
                self.close_state = CloseState::Saving {
                    since: std::time::Instant::now() - std::time::Duration::from_millis(50),
                    done: None,
                    retry: None,
                };
            }
            // back to Save the changes
            (OverwriteThen::CloseSave, false) => {}
        }
        ctx.request_repaint();
    }

    /// Replace the file?: the web window in the middle of the app (a page
    /// of the app window, so never behind it), or egui's own without pages.
    pub(super) fn overwrite_window(&mut self, ctx: &egui::Context) {
        let Some(o) = self.overwrite.clone() else {
            return;
        };
        if self.shell.web.is_some() {
            self.show_overwrite_dialog(ctx);
            // the project behind waits
            egui::Modal::new(egui::Id::new("overwrite-ask"))
                .frame(egui::Frame::NONE)
                .backdrop_color(egui::Color32::TRANSPARENT)
                .show(ctx, |_| {});
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.overwrite_answer(ctx, false);
            }
            return;
        }
        let (title, line, file) = overwrite_texts(&o.path);
        let (mut yes, mut no) = (false, false);
        egui::Modal::new(egui::Id::new("overwrite-ask")).show(ctx, |ui| {
            ui.set_width(420.0);
            ui.heading(&title);
            ui.add_space(6.0);
            ui.label(egui::RichText::new(&file).strong());
            ui.label(&line);
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                yes = ui.button(crate::i18n::tr("Replace")).clicked();
                no = ui.button(crate::i18n::tr("Cancel")).clicked()
                    || ui.input(|i| i.key_pressed(egui::Key::Escape));
            });
        });
        if yes || no {
            self.overwrite_answer(ctx, yes);
        }
    }

    /// The Replace the file? window's state, for its page.
    pub(super) fn overwrite_state_json(&self) -> serde_json::Value {
        let Some(o) = &self.overwrite else {
            return serde_json::Value::Null;
        };
        let (title, line, file) = overwrite_texts(&o.path);
        let dir = std::path::Path::new(&o.path)
            .parent()
            .map(|d| d.display().to_string())
            .unwrap_or_default();
        serde_json::json!({
            "title": title,
            "line": line,
            "file": file,
            "dir": dir,
            "yes": crate::i18n::tr("Replace"),
            "no": crate::i18n::tr("Cancel"),
            "hint": crate::i18n::tr("back without saving"),
        })
    }

    /// Closing with unsaved changes: Save, Don't save or Cancel. The web
    /// window of REV02 where the pages are; the plain one otherwise.
    pub(super) fn close_window(&mut self, ctx: &egui::Context) {
        if !self.close_ask {
            return;
        }
        // Replace the file? is asked over it
        if self.overwrite.is_some() {
            return;
        }
        if self.shell.web.is_some() {
            self.show_close_dialog(ctx);
            // the project behind waits: nothing in it can be clicked. Not
            // dimmed: the web panes can't be, and half a shade looks broken
            egui::Modal::new(egui::Id::new("close-unsaved"))
                .frame(egui::Frame::NONE)
                .backdrop_color(egui::Color32::TRANSPARENT)
                .show(ctx, |_| {});
            // the keys, while the views have the keyboard
            if matches!(self.close_state, CloseState::Ask | CloseState::Failed(_)) {
                let (enter, esc) = ctx.input(|i| {
                    (
                        i.key_pressed(egui::Key::Enter),
                        i.key_pressed(egui::Key::Escape),
                    )
                });
                if enter {
                    self.close_do(ctx, "save");
                } else if esc {
                    self.close_do(ctx, "cancel");
                }
            }
            self.close_step(ctx);
            return;
        }
        let name = std::path::Path::new(&self.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("this project")
            .to_string();
        let pal = crate::theme::Palette::of(ctx);
        let (mut save, mut discard, mut cancel) = (false, false, false);
        egui::Modal::new(egui::Id::new("close-unsaved")).show(ctx, |ui| {
            ui.set_width(400.0);
            ui.heading(crate::i18n::tr(&format!("Save the changes to {name}?")));
            ui.add_space(6.0);
            ui.label(crate::i18n::t("They are lost if you close without saving."));
            ui.label(
                egui::RichText::new(crate::i18n::t("Enter saves, Esc goes back to the project."))
                    .small()
                    .color(pal.muted),
            );
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                save = ui.button(crate::i18n::t("Save")).clicked()
                    || ui.input(|i| i.key_pressed(egui::Key::Enter));
                discard = ui.button(crate::i18n::t("Don't save")).clicked();
                cancel = ui.button(crate::i18n::t("Cancel")).clicked()
                    || ui.input(|i| i.key_pressed(egui::Key::Escape));
            });
        });
        if cancel {
            self.close_ask = false;
            self.status = "Close cancelled: the project is still open.".into();
            return;
        }
        if save {
            // where it has a file already, there; otherwise Save as asks
            if std::path::Path::new(&self.path).exists() {
                self.save();
            } else {
                self.save_as();
            }
            if self.unsaved() {
                // the save was cancelled or failed: the project stays open
                self.close_ask = false;
                return;
            }
        }
        if save || discard {
            self.close_ask = false;
            self.close_ok = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

/// Where a manual update check looks: the project's releases page. Nothing
/// is fetched in the background; the page opens in the browser when asked.
pub(crate) const RELEASES: &str = "https://github.com/marektahotny/CreaBIM/releases";

/// One way to start a project in the New project window.
pub(crate) struct Start {
    /// The action that opens it.
    pub id: &'static str,
    pub title: &'static str,
    /// One line on what it is, or what it shows.
    pub line: &'static str,
    pub icon: &'static str,
}

/// The large choice at the top of New project; the recent projects
/// follow it.
pub(crate) const STARTS: [Start; 1] = [Start {
    id: "new.empty",
    title: "Empty project",
    line: "Two levels and nothing else, in your units. The everyday start.",
    icon: "file-plus",
}];

/// The samples to learn from, each showing one thing.
pub(crate) const SAMPLES: [Start; 6] = [
    Start {
        id: "new.simple",
        title: "Simple house",
        line: "Walls, doors, windows, a roof",
        icon: "house",
    },
    Start {
        id: "new.layered",
        title: "Layered walls",
        line: "Assemblies and how layers join",
        icon: "layers",
    },
    Start {
        id: "new.buried",
        title: "Beam in a wall",
        line: "Structure cutting through walls",
        icon: "beam",
    },
    Start {
        id: "new.typed",
        title: "Wall types",
        line: "Types, and editing many at once",
        icon: "make-type",
    },
    Start {
        id: "new.furnished",
        title: "Furnished house",
        line: "Objects, fixtures and clearances",
        icon: "sofa",
    },
    Start {
        id: "new.garden",
        title: "House in its garden",
        line: "Terrain, paving and planting",
        icon: "planting",
    },
];

/// A short name for a unit, as the status bar shows it.
pub(crate) fn unit_short(u: LengthUnit) -> &'static str {
    match u {
        LengthUnit::Millimetres => "mm",
        LengthUnit::Centimetres => "cm",
        LengthUnit::Metres => "m",
        LengthUnit::FeetInches => "ft-in",
        LengthUnit::Inches => "in",
        LengthUnit::Feet => "ft",
    }
}

/// A unit's name in the Units list, with an example beside it.
fn unit_row(u: LengthUnit) -> (&'static str, &'static str) {
    match u {
        LengthUnit::Millimetres => ("Millimetres", "3200"),
        LengthUnit::Centimetres => ("Centimetres", "320.0"),
        LengthUnit::Metres => ("Metres", "3.200"),
        LengthUnit::FeetInches => ("Feet and inches", "10'-6\""),
        LengthUnit::Inches => ("Inches", "126\""),
        LengthUnit::Feet => ("Decimal feet", "10.50'"),
    }
}

/// The Units list's order: metric, then imperial.
const UNIT_ORDER: [LengthUnit; 6] = [
    LengthUnit::Millimetres,
    LengthUnit::Centimetres,
    LengthUnit::Metres,
    LengthUnit::FeetInches,
    LengthUnit::Inches,
    LengthUnit::Feet,
];

impl CreaBimApp {
    /// The CreaBIM menu, first in the menu bar.
    pub(super) fn app_menu(&self) -> Vec<Item> {
        vec![
            Item::new("app.about", "About CreaBIM")
                .icon("info")
                .hint("The version, the licence and the guide"),
            Item::Header(crate::help::version_line()),
            Item::Separator,
            Item::new("app.updates", "Check for updates…")
                .icon("refresh-cw")
                .hint("Opens the releases page in your browser. CreaBIM never checks by itself: it stays offline."),
            Item::new("app.settings", "Settings…")
                .key("Ctrl+,")
                .icon("settings")
                .hint("Your theme and the project's units"),
            Item::new("help:shortcuts", "Keyboard shortcuts")
                .key("Ctrl+/")
                .icon("keyboard"),
            Item::new("app.log", "Open the activity log")
                .icon("file")
                .hint("What CreaBIM did, step by step (drags frame by frame, edits, tools): attach it to a bug report. It stays on this computer."),
            Item::Separator,
            Item::new("app.exit", "Exit").key("Alt+F4").icon("log-out"),
        ]
    }

    /// The status bar's Units list: metric, then imperial, each with how a
    /// length looks in it.
    pub(super) fn unit_items(&self) -> Vec<Item> {
        let now = self.doc.model().units().length;
        let mut items = vec![Item::Header("Metric".into())];
        for (i, u) in UNIT_ORDER.into_iter().enumerate() {
            if u == LengthUnit::FeetInches {
                items.push(Item::Header("Imperial".into()));
            }
            let (name, example) = unit_row(u);
            items.push(
                Item::new(format!("units.set:{i}"), name)
                    .key(example)
                    .checked(u == now)
                    .hint("Applies at once to typing, readouts and dimensions, and is saved with the project"),
            );
        }
        // what each kind of annotation writes in (A5-2)
        items.push(Item::Header("Number formats".into()));
        let units = self.doc.model().units();
        let pack = creabim_core::styles::active_pack(self.doc.model());
        for (i, k) in creabim_core::NumberKind::ALL.into_iter().enumerate() {
            let f = pack.number_formats.get(k);
            let said = if f.is_default() {
                "project".to_string()
            } else {
                f.write(&units, k, if k.is_level() { 3.0 } else { 1.0668 })
            };
            items.push(
                Item::new(
                    format!("units.kind:{i}"),
                    format!("{}  ·  {said}", k.label()),
                )
                .hint("Opens its number format"),
            );
        }
        items.push(Item::Separator);
        items.push(
            Item::new("units", "Number formats…")
                .icon("units")
                .hint("Project units, and how dimensions, level heads, spot levels, tags and schedules write numbers. Saved with the style pack."),
        );
        items
    }

    /// Switch the project to a unit, keeping the rounding when it stays in
    /// the same kind (decimal or fractional).
    pub(super) fn set_unit(&mut self, u: LengthUnit) {
        let before = self.doc.model().units();
        if before.length == u {
            return;
        }
        let units = Units::of(u);
        let tx = creabim_core::units::set_units(self.doc.model(), units);
        let r = self.doc.commit(tx);
        crate::units::set(units);
        self.report(
            r,
            format!(
                "Units: {}. Typed values with a unit (3 m, 10'6\") still work.",
                unit_row(u).0.to_lowercase()
            ),
        );
    }

    /// The application menu's actions. Returns false for any other.
    pub(super) fn app_action(&mut self, ctx: &egui::Context, head: &str, arg: &str) -> bool {
        match head {
            "app.about" => self.open_help("about"),
            "app.updates" => {
                ctx.open_url(egui::OpenUrl::new_tab(RELEASES));
                self.status = format!(
                    "This is CreaBIM {}. The releases page is open in your browser; nothing was downloaded.",
                    crate::help::version_line().trim_start_matches("Version ")
                );
            }
            "app.log" => {
                self.status = match crate::actlog::path() {
                    Some(p) => match crate::files::open_folder(&p) {
                        Ok(()) => format!("The activity log is {}.", p.display()),
                        Err(e) => e,
                    },
                    None => "There is no activity log on this computer.".into(),
                };
            }
            "app.report" => self.report_problem(),
            "app.settings" => self.settings_open = true,
            "app.exit" => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            "units.kind" => {
                if let Some(k) = arg
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| creabim_core::NumberKind::ALL.get(i))
                {
                    self.levels.units_kind = Some(*k);
                    self.levels.units_open = true;
                }
            }
            "units.set" => {
                if let Some(u) = arg.parse::<usize>().ok().and_then(|i| UNIT_ORDER.get(i)) {
                    self.set_unit(*u);
                }
            }
            _ => return false,
        }
        true
    }

    /// Settings: what each person sets for themselves (the theme), and the
    /// project's units.
    pub(super) fn settings_window(&mut self, ctx: &egui::Context) {
        if !self.settings_open {
            return;
        }
        let mut open = true;
        let mut theme = self.shell.theme;
        let mut unit = self.doc.model().units().length;
        let mut keys = false;
        let mut grips = self.shell.settings.grips;
        let mut spacing = self.plan.snap.step_um;
        let mut marks = self.plan.snap.marks;
        let mut autosave = self.shell.settings.autosave_minutes;
        let mut paper = self.shell.settings.drawing_paper.clone();
        let mut navigation = self.shell.settings.navigation.clone();
        let mut draw_font = self.shell.settings.drawing_font.clone();
        let mut text_width = self.shell.settings.text_width;
        let mut grip_hover = None;
        let mut edit_bar = self.shell.settings.edit_bar.clone();
        let mut axis_lines = self.shell.settings.axis_lines;
        let mut interaction = self.shell.settings.interaction.clone();
        let mut edge_w = self.shell.settings.edit_edge_width;
        let mut vert_s = self.shell.settings.edit_vertex_size;
        let theme_accent = self.shell.theme.selection();
        let theme_logo = self.shell.theme.logo();
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        // as Blender's Preferences (MU39): its categories down the left, the
        // one picked on the right, scrolling, so it never grows past the
        // window as settings are added
        let tab_id = egui::Id::new("settings.tab");
        let mut tab: usize = ctx.data(|d| d.get_temp(tab_id)).unwrap_or(0);
        const TABS: [(&str, &str); 6] = [
            ("Interface", "layout-panel-left"),
            ("Viewport", "box"),
            ("Navigation", "hand"),
            ("Snapping", "magnet"),
            ("Project", "folder"),
            ("Keymap", "keyboard"),
        ];
        let height = (area.height() - 80.0).clamp(300.0, 560.0);
        crate::app::shell::window(crate::i18n::tr("Settings"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(310.0, height / 2.0))
            .constrain_to(area)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.set_width(620.0);
                ui.set_height(height);
                let p = crate::theme::Palette::of(ui.ctx());
                ui.horizontal_top(|ui| {
                    // the categories
                    ui.vertical(|ui| {
                        ui.set_width(140.0);
                        ui.spacing_mut().item_spacing.y = 2.0;
                        // the logo, rising in once as Settings opens (MU39)
                        let t = {
                            let now = ui.input(|i| i.time);
                            let key = egui::Id::new("settings.logo.t0");
                            let t0 = ui.ctx().data_mut(|d| *d.get_temp_mut_or_insert_with(key, || now));
                            let t = (((now - t0) / 0.6) as f32).clamp(0.0, 1.0);
                            if t < 1.0 {
                                ui.ctx().request_repaint();
                            }
                            1.0 - (1.0 - t).powi(3)
                        };
                        let (r, _) = ui.allocate_exact_size(Vec2::new(140.0, 46.0), egui::Sense::hover());
                        let tex = logo_texture(ui.ctx());
                        let s = 36.0;
                        let at = egui::Rect::from_min_size(
                            egui::pos2(r.left() + 8.0, r.top() + 4.0 + (1.0 - t) * 10.0),
                            Vec2::splat(s),
                        );
                        egui::Image::new((tex.id(), Vec2::splat(s)))
                            .tint(theme_logo.gamma_multiply(t))
                            .paint_at(ui, at);
                        ui.painter().text(
                            egui::pos2(at.right() + 8.0, r.center().y),
                            egui::Align2::LEFT_CENTER,
                            crate::i18n::tr("Settings"),
                            egui::FontId::proportional(14.0),
                            p.text.gamma_multiply(t),
                        );
                        for (i, (name, icon)) in TABS.iter().enumerate() {
                            let (r, resp) = ui.allocate_exact_size(
                                Vec2::new(140.0, 28.0),
                                egui::Sense::click(),
                            );
                            let on = i == tab;
                            if on {
                                ui.painter().rect_filled(r, 6.0, p.accent.gamma_multiply(0.22));
                            } else if resp.hovered() {
                                ui.painter().rect_filled(r, 6.0, p.raise);
                            }
                            let ink = if on { p.text } else { p.muted };
                            crate::svgicon::paint(
                                ui.painter(),
                                egui::Rect::from_center_size(
                                    egui::pos2(r.left() + 16.0, r.center().y),
                                    Vec2::splat(15.0),
                                ),
                                icon,
                                ink,
                            );
                            ui.painter().text(
                                egui::pos2(r.left() + 32.0, r.center().y),
                                egui::Align2::LEFT_CENTER,
                                *name,
                                egui::FontId::proportional(13.0),
                                ink,
                            );
                            if resp.clicked() {
                                tab = i;
                            }
                        }
                    });
                    let (sep, _) = ui.allocate_exact_size(Vec2::new(9.0, height), egui::Sense::hover());
                    ui.painter().line_segment(
                        [sep.center_top(), sep.center_bottom()],
                        egui::Stroke::new(1.0, p.border),
                    );
                    // the category picked
                    ui.vertical(|ui| {
                        ui.set_width(460.0);
                        egui::ScrollArea::vertical()
                            .id_salt(("settings.page", tab))
                            .max_height(height)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.set_width(448.0);
                                let yours = |ui: &mut egui::Ui| {
                                    ui.add_space(6.0);
                                    ui.label(
                                        egui::RichText::new(crate::i18n::t("Yours alone: it never changes anyone's drawings, and goes with you, not with the project."))
                                            .weak()
                                            .small(),
                                    );
                                };
                                match tab {
                                    0 => {
                                        crate::props::section(ui, "Look");
                                        crate::props::row(ui, "Theme", |ui| {
                                            egui::ComboBox::from_id_salt("settings.theme").icon(crate::props::combo_icon)
                                                .selected_text(crate::i18n::tw(theme.label()))
                                                .width(ui.available_width())
                                                .show_ui(ui, |ui| {
                                                    for t in crate::theme::Theme::ALL {
                                                        ui.selectable_value(&mut theme, t, crate::i18n::tw(t.label()));
                                                    }
                                                });
                                        });
                                        crate::props::row(ui, "Drawing paper", |ui| {
                                            let label = |k: &str| match k {
                                                "light" => "Always light",
                                                "dark" => "Always dark",
                                                _ => "Follow the theme",
                                            };
                                            egui::ComboBox::from_id_salt("settings.paper").icon(crate::props::combo_icon)
                                                .selected_text(crate::i18n::tw(label(&paper)))
                                                .width(ui.available_width())
                                                .show_ui(ui, |ui| {
                                                    for k in ["theme", "light", "dark"] {
                                                        ui.selectable_value(&mut paper, k.to_string(), crate::i18n::tw(label(k)));
                                                    }
                                                })
                                                .response
                                                .on_hover_text(crate::i18n::t("Plans, sections and the 3D view on light or dark paper. Prints and PDFs are always on white."));
                                        });
                                        crate::props::section(ui, "Drawing text");
                                        crate::props::row(ui, "Font", |ui| {
                                            let name = creabim_geom::font::setup()
                                                .font
                                                .map(|f| f.family.clone())
                                                .unwrap_or_else(|| crate::i18n::tr("Built-in sans"));
                                            ui.horizontal(|ui| {
                                                if !draw_font.is_empty()
                                                    && ui.small_button("×").on_hover_text(crate::i18n::tr("Back to the usual font")).clicked()
                                                {
                                                    draw_font.clear();
                                                }
                                                if ui.button(name)
                                                    .on_hover_text(crate::i18n::tr("Pick a font file (TTF or OTF) for the text of drawings: dimensions, tags, notes, title blocks. It is used on screen and carried in PDF and SVG. With none picked, Avenir Next Condensed is used where it is found: in a fonts folder beside the program or among the system's fonts."))
                                                    .clicked()
                                                {
                                                    if let Some(p) = crate::files::file_dialog()
                                                        .set_title(crate::i18n::tr("Font of the drawings"))
                                                        .add_filter("TrueType, OpenType", &["ttf", "otf"])
                                                        .pick_file()
                                                    {
                                                        draw_font = p.display().to_string();
                                                    }
                                                }
                                            });
                                        });
                                        crate::props::row(ui, "Letter width", |ui| {
                                            ui.add(egui::DragValue::new(&mut text_width).range(0.3..=2.0).speed(0.01).fixed_decimals(2))
                                                .on_hover_text(crate::i18n::tr("1 is the font's own width; 0.7 draws every letter of the drawings at 70 % of it."))
                                        });
                                        crate::props::section(ui, "Move, Rotate and Scale bar");
                                        for (k, name, note) in [
                                            ("top", "Over the view", "Default. A slim bar at the top of the view"),
                                            ("pointer", "By the pointer", "A small palette where you started"),
                                        ] {
                                            let r = crate::choices::choice_row(ui, edit_bar == k, name, note, |p, r| {
                                                crate::choices::paint_edit_bar(p, r, k == "pointer")
                                            });
                                            if r.clicked() {
                                                edit_bar = k.to_string();
                                            }
                                        }
                                        yours(ui);
                                    }
                                    1 => {
                                        crate::props::section(ui, "Selection and grips colour");
                                        for (k, name, note, ink) in [
                                            ("orange", "Orange", "Default. The selection, grips and ghosts in orange", crate::theme::ORANGE),
                                            ("theme", "Follow the theme", "In the theme's own accent colour", theme_accent),
                                        ] {
                                            let r = crate::choices::choice_row(ui, interaction == k, name, note, |p, r| {
                                                crate::choices::paint_interaction(p, r, ink)
                                            });
                                            if r.clicked() {
                                                interaction = k.to_string();
                                            }
                                        }
                                        crate::props::section(ui, "Grips");
                                        for g in crate::handles::GripStyle::ALL {
                                            let note = match g {
                                                crate::handles::GripStyle::Quiet => "Default. Squares at corners, bars on edges",
                                                crate::handles::GripStyle::Edges => "Almost nothing at rest; edges drag",
                                                crate::handles::GripStyle::OneShape => "The same small circle everywhere",
                                            };
                                            let r = crate::choices::choice_row(ui, grips == g, g.label(), note, |p, r| {
                                                crate::choices::paint_grips(p, r, g)
                                            });
                                            if r.clicked() {
                                                grips = g;
                                            } else if r.hovered() {
                                                grip_hover = Some(g);
                                            }
                                        }
                                        crate::props::section(ui, "Edit mode");
                                        crate::props::row(ui, "Edge width", |ui| {
                                            ui.add(egui::Slider::new(&mut edge_w, 0.5..=3.0).step_by(0.25).suffix(crate::i18n::t(" px")))
                                        });
                                        crate::props::row(ui, "Vertex size", |ui| {
                                            ui.add(egui::Slider::new(&mut vert_s, 2.0..=10.0).step_by(1.0).suffix(crate::i18n::t(" px")))
                                        });
                                        crate::props::section(ui, "3D view");
                                        ui.checkbox(&mut axis_lines, crate::i18n::t("X and Y axes on the grid"))
                                            .on_hover_text(crate::i18n::t("The model's X axis in red and Y in green through its origin. Z shows, in blue, only while a move keeps to it."));
                                        yours(ui);
                                    }
                                    2 => {
                                        crate::props::section(ui, "Mouse");
                                        crate::props::row(ui, "Mouse", |ui| {
                                            let label = |k: &str| match k {
                                                "classic" => "Classic: left drag orbits",
                                                _ => "CreaBIM: middle pans, Shift + middle orbits, left boxes",
                                            };
                                            egui::ComboBox::from_id_salt("settings.navigation").icon(crate::props::combo_icon)
                                                .selected_text(crate::i18n::tw(label(&navigation)))
                                                .width(ui.available_width())
                                                .show_ui(ui, |ui| {
                                                    for k in ["creabim", "classic"] {
                                                        ui.selectable_value(&mut navigation, k.to_string(), crate::i18n::tw(label(k)));
                                                    }
                                                });
                                        });
                                        ui.label(
                                            egui::RichText::new(crate::i18n::t("CreaBIM (recommended): left click selects, a left drag on empty space draws a box, a middle drag pans, Shift + middle orbits (Alt + left drag without a middle button), a right drag pans too, a right click opens the menu, the wheel zooms. Classic: a left drag orbits in 3D."))
                                                .weak()
                                                .small(),
                                        );
                                        yours(ui);
                                    }
                                    3 => {
                                        crate::props::section(ui, "Grid snap");
                                        crate::props::row(ui, "Spacing", |ui| {
                                            let steps: &[u32] = if crate::units::get().is_imperial() {
                                                &crate::snaps::STEPS_IMPERIAL
                                            } else {
                                                &crate::snaps::STEPS_METRIC
                                            };
                                            egui::ComboBox::from_id_salt("settings.spacing").icon(crate::props::combo_icon)
                                                .selected_text(crate::i18n::tw(crate::units::len(spacing as f64 / 1e6)))
                                                .width(ui.available_width())
                                                .show_ui(ui, |ui| {
                                                    for um in steps {
                                                        ui.selectable_value(&mut spacing, *um, crate::i18n::tw(crate::units::len(*um as f64 / 1e6)));
                                                    }
                                                });
                                        });
                                        ui.checkbox(&mut marks, crate::i18n::t("Distance marks while dragging"))
                                            .on_hover_text(crate::i18n::t("Faint marks every spacing from where you grabbed; a mark holds the drag only when the pointer is near it."));
                                        ui.label(
                                            egui::RichText::new(crate::i18n::t("Drags and points are free. Grid snap (Alt+S, or the status bar) lands them on a grid of this spacing; it starts off each time."))
                                                .weak()
                                                .small(),
                                        );
                                        yours(ui);
                                    }
                                    4 => {
                                        crate::props::section(ui, "This project");
                                        crate::props::row(ui, "Units", |ui| {
                                            egui::ComboBox::from_id_salt("settings.units").icon(crate::props::combo_icon)
                                                .selected_text(crate::i18n::tw(unit_row(unit).0))
                                                .width(ui.available_width())
                                                .show_ui(ui, |ui| {
                                                    for u in UNIT_ORDER {
                                                        ui.selectable_value(&mut unit, u, crate::i18n::tw(unit_row(u).0));
                                                    }
                                                });
                                        });
                                        ui.label(
                                            egui::RichText::new(crate::i18n::t("Saved with the project: everyone who opens it works in these units."))
                                                .weak()
                                                .small(),
                                        );
                                    }
                                    _ => {
                                        crate::props::section(ui, "Autosave");
                                        crate::props::row(ui, "Every", |ui| {
                                            egui::ComboBox::from_id_salt("settings.autosave").icon(crate::props::combo_icon)
                                                .selected_text(match autosave {
                                                    0 => crate::i18n::tr("Off"),
                                                    m => crate::i18n::tr(&format!("{m} min")),
                                                })
                                                .width(ui.available_width())
                                                .show_ui(ui, |ui| {
                                                    for m in [0u32, 2, 5, 10, 15, 30] {
                                                        let label = match m {
                                                            0 => crate::i18n::tr("Off"),
                                                            m => crate::i18n::tr(&format!("{m} min")),
                                                        };
                                                        ui.selectable_value(&mut autosave, m, label);
                                                    }
                                                });
                                        });
                                        ui.label(
                                            egui::RichText::new(crate::i18n::t("While there are changes the project is written to Name.autosave.creabim beside it, and offered back if CreaBIM closes without saving. Saving keeps the file it replaces as Name.bak.creabim."))
                                                .weak()
                                                .small(),
                                        );
                                        ui.add_space(8.0);
                                        crate::props::section(ui, "Shortcuts");
                                        ui.label(
                                            egui::RichText::new(crate::i18n::t("Every key, by what it does. Ctrl+/ opens the list any time."))
                                                .weak()
                                                .small(),
                                        );
                                        ui.add_space(4.0);
                                        keys = ui.button(crate::i18n::t("Keyboard shortcuts…")).clicked();
                                    }
                                }
                            });
                    });
                });
            });
        ctx.data_mut(|d| d.insert_temp(tab_id, tab));
        self.settings_open = open;
        if !open {
            ctx.data_mut(|d| d.remove::<f64>(egui::Id::new("settings.logo.t0")));
        }
        // resting on a grip look shows it at once; leaving puts yours back
        crate::handles::set_grip_style(grip_hover.filter(|_| open).unwrap_or(grips));
        if edge_w != self.shell.settings.edit_edge_width
            || vert_s != self.shell.settings.edit_vertex_size
        {
            crate::handles::set_edit_display(edge_w, vert_s);
            self.shell.settings.edit_edge_width = edge_w;
            self.shell.settings.edit_vertex_size = vert_s;
            self.shell.settings.store();
        }
        if autosave != self.shell.settings.autosave_minutes {
            self.shell.settings.autosave_minutes = autosave;
            self.shell.settings.store();
        }
        if interaction != self.shell.settings.interaction {
            self.shell.settings.interaction = interaction;
            self.shell.settings.store();
        }
        if axis_lines != self.shell.settings.axis_lines {
            self.shell.settings.axis_lines = axis_lines;
            self.shell.settings.store();
            self.geometry_key = None;
        }
        if edit_bar != self.shell.settings.edit_bar {
            crate::modal::set_bar_by_pointer(edit_bar == "pointer");
            self.shell.settings.edit_bar = edit_bar;
            self.shell.settings.store();
        }
        if draw_font != self.shell.settings.drawing_font
            || (text_width - self.shell.settings.text_width).abs() > 1e-9
        {
            let picked = draw_font != self.shell.settings.drawing_font;
            self.shell.settings.drawing_font = draw_font;
            self.shell.settings.text_width = text_width;
            let found = crate::drawfont::apply(ctx, &self.shell.settings);
            if picked && found.is_none() && !self.shell.settings.drawing_font.is_empty() {
                // what was picked is no font: back to the usual
                self.status = crate::i18n::tr("That file is not a font CreaBIM can read.");
                self.shell.settings.drawing_font.clear();
                crate::drawfont::apply(ctx, &self.shell.settings);
            }
            self.shell.settings.store();
            self.geometry_key = None;
        }
        if navigation != self.shell.settings.navigation {
            self.shell.settings.navigation = navigation;
            self.shell.settings.store();
        }
        if paper != self.shell.settings.drawing_paper {
            self.shell.settings.drawing_paper = paper;
            self.shell.settings.store();
        }
        if grips != self.shell.settings.grips {
            self.shell.settings.grips = grips;
            self.shell.settings.store();
            self.status = format!("Grips: {}.", grips.label());
        }
        if spacing != self.plan.snap.step_um || marks != self.plan.snap.marks {
            self.plan.snap.step_um = spacing;
            self.plan.snap.marks = marks;
            self.shell.settings.snap_spacing_um = spacing;
            self.shell.settings.distance_marks = marks;
            self.shell.settings.store();
        }
        if theme != self.shell.theme {
            self.set_theme(ctx, theme);
        }
        if unit != self.doc.model().units().length {
            self.set_unit(unit);
        }
        if keys {
            self.open_help("shortcuts");
        }
    }
}

impl CreaBimApp {
    /// New project with the classic panels: the same choices as the web
    /// window, as a list.
    pub(super) fn new_project_window(&mut self, ctx: &egui::Context) {
        if !self.new_project_classic {
            return;
        }
        let mut open = true;
        let mut pick = None;
        crate::app::shell::window(crate::i18n::tr("New project"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                for s in STARTS.iter() {
                    if ui
                        .button(egui::RichText::new(s.title).strong())
                        .on_hover_text(crate::i18n::tr(s.line))
                        .clicked()
                    {
                        pick = Some(s.id.to_string());
                    }
                }
                for (id, title, line) in &self.recent_cards {
                    if ui
                        .button(egui::RichText::new(title).strong())
                        .on_hover_text(line)
                        .clicked()
                    {
                        pick = Some(id.clone());
                    }
                }
                crate::props::section(ui, "Samples to learn from");
                for s in SAMPLES.iter() {
                    if ui
                        .button(crate::i18n::tr(s.title))
                        .on_hover_text(crate::i18n::tr(s.line))
                        .clicked()
                    {
                        pick = Some(s.id.to_string());
                    }
                }
            });
        self.new_project_classic = open && pick.is_none();
        if let Some(id) = pick {
            self.run_action(ctx, &id);
        }
    }
}

impl CreaBimApp {
    /// Export > IFC (MU20): how walls, slabs and roofs go out, with what
    /// the other side will see under each; the last choice is ticked.
    pub(super) fn ifc_export_window(&mut self, ctx: &egui::Context) {
        use creabim_ifc::IfcBody;
        if !self.ifc_dialog {
            return;
        }
        let key = |b: IfcBody| match b {
            IfcBody::Modelled => "modelled",
            IfcBody::Joined => "joined",
            IfcBody::Layers => "layers",
        };
        let mut body = match self.shell.settings.ifc_body.as_str() {
            "joined" => IfcBody::Joined,
            "layers" => IfcBody::Layers,
            _ => IfcBody::Modelled,
        };
        let mut open = true;
        let mut go = false;
        let mut cancel = false;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        crate::app::shell::window(crate::i18n::tr("Export IFC"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(230.0, 160.0))
            .constrain_to(area)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.set_width(460.0);
                ui.label(egui::RichText::new(crate::i18n::t("How walls, slabs and roofs go out")).strong());
                let choices = [
                    (
                        IfcBody::Modelled,
                        "As modelled elements (recommended)",
                        "Each wall and slab is one IfcWall or IfcSlab, its layers carried as its layer set, its body the plain extrusion. Every BIM program reads it, and it comes back into CreaBIM as it went out. The other program settles the joins.",
                    ),
                    (
                        IfcBody::Joined,
                        "Joined shapes, as drawn",
                        "Bodies cut where walls run through slabs, as CreaBIM shows them: for coordination and clash checks. Some programs then read a cut slab as a generic shape, not a slab, and it won't come back into CreaBIM as a slab.",
                    ),
                    (
                        IfcBody::Layers,
                        "Layers as separate elements",
                        "The structure stays the IfcWall or IfcSlab; finishes become IfcCovering (cladding, flooring, roofing), insulation and membranes too, each covering its element. For trades and quantities by layer.",
                    ),
                ];
                for (b, title, what) in choices {
                    ui.add_space(4.0);
                    ui.radio_value(&mut body, b, egui::RichText::new(title).strong());
                    ui.indent(("ifc", key(b)), |ui| {
                        ui.label(egui::RichText::new(what).weak());
                    });
                }
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(crate::i18n::t("Doors, windows, openings and everything else go out the same way in all three."))
                        .weak(),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    go = ui.button(crate::i18n::t("Export…")).clicked();
                    cancel = ui.button(crate::i18n::t("Cancel")).clicked();
                });
            });
        if key(body) != self.shell.settings.ifc_body {
            self.shell.settings.ifc_body = key(body).into();
            self.shell.settings.store();
        }
        self.ifc_dialog = open && !cancel && !go;
        if go {
            let Some(out) = self.export_to(with_extension(&self.path, "ifc")) else {
                return;
            };
            let mut opts =
                creabim_ifc::ExportOptions::for_model(self.doc.model(), &stem(&self.path));
            opts.body = body;
            let r = std::fs::write(&out, creabim_ifc::export_ifc4(self.doc.model(), &opts));
            self.report(r, format!("Exported {out}"));
        }
    }

    /// Project information (Manage › Project information, A7-5): a copy is
    /// edited, and Apply makes it one undo step.
    pub(super) fn project_window(&mut self, ctx: &egui::Context) {
        if !self.project_open {
            self.project_draft = None;
            return;
        }
        let current = self
            .doc
            .model()
            .project_info()
            .map(|(_, p)| p.clone())
            .unwrap_or_default();
        let mut draft = self.project_draft.take().unwrap_or_else(|| current.clone());
        let mut open = true;
        let mut apply = false;
        let mut close = false;
        // the project's north turned: by an angle, or by a line picked in a plan
        let mut north_turn: Option<f64> = None;
        let mut model_turn: Option<f64> = None;
        let mut north_line = false;
        let mut fire_params = false;
        let mut fire_doors = false;
        // the plot's data asked of the cadastre
        let mut cadastre = false;
        let screen = crate::app::shell::window_room(ctx);
        // side by side in columns as the room right of the project
        // browser allows: project and sheets, site and place, fire safety
        // (440 a column: the fire column's long labels and the S-JTSK fields
        // fit without cutting or running into the next column)
        let n = (((screen.width() - 30.0) / 456.0).floor() as usize).clamp(1, 3);
        crate::app::shell::window(crate::i18n::tr("Project information"))
            // an id by the columns: a size kept from another layout (the
            // one-column window) is not reused
            .id(egui::Id::new(("project.info.columns", n)))
            .open(&mut open)
            .pivot(egui::Align2::CENTER_CENTER)
            .default_pos(screen.center())
            .constrain_to(screen)
            .resizable(false)
            .collapsible(false)
            .show(ctx, |ui| {
                // the fields scroll and the buttons stay in sight
                ui.set_width(n as f32 * 440.0 + (n - 1) as f32 * 16.0);
                egui::ScrollArea::vertical()
                    .id_salt("project.info.scroll")
                    .max_height((screen.height() - 140.0).max(200.0))
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                let text = |ui: &mut egui::Ui, label: &str, v: &mut String, hint: &str| {
                    crate::props::row_wrapped(ui, label, |ui| {
                        ui.add(
                            egui::TextEdit::singleline(v).vertical_align(egui::Align::Center)
                                .hint_text(crate::i18n::tr(hint))
                                .desired_width(ui.available_width()),
                        );
                    });
                };
                let degrees = |ui: &mut egui::Ui, label: &str, v: &mut Option<f64>, range: f64| {
                    crate::props::row(ui, label, |ui| {
                        let mut on = v.is_some();
                        if ui.checkbox(&mut on, "").changed() {
                            *v = on.then_some(0.0);
                        }
                        if let Some(d) = v {
                            ui.add(
                                egui::DragValue::new(d)
                                    .range(-range..=range)
                                    .speed(0.0001)
                                    .max_decimals(6)
                                    .suffix("°"),
                            );
                        }
                    });
                };
                // the parts: 0 project and sheets, 1 site and place, 2 fire
                let parts: &[&[u8]] = match n {
                    3 => &[&[0], &[1], &[2]],
                    2 => &[&[0, 2], &[1]],
                    _ => &[&[0, 1, 2]],
                };
                ui.columns(n, |cols| {
                for (c, ui) in cols.iter_mut().enumerate() {
                // nothing in a column reaches into the next one
                let col = ui.max_rect();
                ui.set_clip_rect(col.intersect(ui.clip_rect()));
                for &part in parts[c] {
                match part {
                0 => {
                crate::props::section(ui, "Project");
                text(ui, "Name", &mut draft.name, "Maple Lane House");
                text(ui, "Number", &mut draft.number, "Job number");
                text(ui, "Client", &mut draft.client, "");
                crate::props::row(ui, "Stage", |ui| {
                    let shown = match creabim_core::project::stage_code(&draft.status) {
                        Some(code) => format!("{} ({code})", draft.status.trim()),
                        None => draft.status.clone(),
                    };
                    egui::ComboBox::from_id_salt("project.stage")
                        .selected_text(shown)
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut draft.status, String::new(), "-");
                            for (name, code) in creabim_core::project::STAGES {
                                ui.selectable_value(
                                    &mut draft.status,
                                    name.to_string(),
                                    format!("{name} ({code})"),
                                );
                            }
                        });
                });
                text(ui, "Issue date", &mut draft.issue_date, "");
                text(ui, "Revision", &mut draft.revision, "00");
                crate::props::section(ui, "Title block");
                text(ui, "Chief architect", &mut draft.architect, "");
                text(ui, "Drawn by", &mut draft.drawn_by, "");
                text(ui, "Responsible designer", &mut draft.designer, "");
                crate::props::row(ui, "Logo", |ui| {
                    crate::props::flow(ui, |ui| {
                        if ui.button(crate::i18n::t("Choose…")).clicked() {
                            if let Some(img) = crate::files::file_dialog()
                                .add_filter("Picture", &["png", "jpg", "jpeg"])
                                .pick_file()
                                .and_then(|p| image::open(p).ok())
                            {
                                // a logo needs no more; the project stays light
                                let img = if img.width().max(img.height()) > 1024 {
                                    img.resize(1024, 1024, image::imageops::FilterType::Triangle)
                                } else {
                                    img
                                };
                                let rgba = img.to_rgba8();
                                draft.logo = Some(creabim_core::project::Logo {
                                    width: rgba.width(),
                                    height: rgba.height(),
                                    rgba: creabim_geom::raster::store_image(rgba.as_raw()),
                                });
                            }
                        }
                        if draft.logo.is_some() {
                            if ui.button(crate::i18n::t("Remove")).clicked() {
                                draft.logo = None;
                            }
                            ui.label(egui::RichText::new(crate::i18n::t("In the title block")).weak());
                        }
                    })
                });
                crate::props::section(ui, "Title pages");
                text(ui, "Design office", &mut draft.office, "");
                text(ui, "Office address and contact", &mut draft.office_contact, "");
                let stage = creabim_geom::coverpage::heading(&creabim_core::project::ProjectInfo {
                    cover_heading: String::new(),
                    status: draft.status.clone(),
                    ..Default::default()
                });
                crate::props::row(ui, "Heading", |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut draft.cover_heading)
                            .vertical_align(egui::Align::Center)
                            .hint_text(stage)
                            .desired_width(ui.available_width()),
                    );
                });
                }
                1 => {
                crate::props::section(ui, "Site");
                text(ui, "Site name", &mut draft.site_name, "");
                text(ui, "Address", &mut draft.address, "");
                text(ui, "Plots", &mut draft.parcels, "1838/116");
                text(ui, "Cadastral district", &mut draft.cadastre, "");
                crate::props::row(ui, "Plot area", |ui| {
                    let mut on = draft.plot_area.is_some();
                    if ui.checkbox(&mut on, "").changed() {
                        draft.plot_area = on.then_some(0.0);
                    }
                    if let Some(a) = &mut draft.plot_area {
                        ui.add(
                            egui::DragValue::new(a)
                                .range(0.0..=1e7)
                                .speed(1.0)
                                .max_decimals(2)
                                .suffix(" m²"),
                        )
                        .on_hover_text(crate::i18n::t(
                            "The summary report works out the built-up index and the green coefficient from it",
                        ));
                    }
                });
                crate::props::row(ui, "Max. built-up index", |ui| {
                    let mut on = draft.max_coverage.is_some();
                    if ui.checkbox(&mut on, "").changed() {
                        draft.max_coverage = on.then_some(0.4);
                    }
                    if let Some(a) = &mut draft.max_coverage {
                        ui.add(
                            egui::DragValue::new(a)
                                .range(0.0..=1.0)
                                .speed(0.01)
                                .max_decimals(2),
                        )
                        .on_hover_text(crate::i18n::t(
                            "The highest built-up index the zoning plan allows: written in the area data of the site plan",
                        ));
                    }
                });
                text(ui, "Town", &mut draft.town, "");
                text(ui, "District", &mut draft.district, "");
                text(ui, "Region", &mut draft.region, "");
                text(ui, "Client's seat", &mut draft.client_address, "");
                text(ui, "Owner of the plot", &mut draft.land_owner, "");
                text(ui, "Sheet of ownership (LV)", &mut draft.ownership_sheet, "");
                if ui
                    .add_enabled(
                        draft.sjtsk.is_some(),
                        egui::Button::new(crate::i18n::t("Plot from the cadastre")),
                    )
                    .on_hover_text(crate::i18n::t(
                        "Asks the cadastre's public service for the parcel under the building: its number, district and area. Needs the S-JTSK origin and the internet.",
                    ))
                    .clicked()
                {
                    cadastre = true;
                }
                degrees(ui, "Latitude", &mut draft.latitude, 90.0);
                degrees(ui, "Longitude", &mut draft.longitude, 180.0);
                crate::props::row(ui, "site\u{4}Elevation", |ui| {
                    ui.add(
                        egui::DragValue::new(&mut draft.elevation)
                            .speed(0.01)
                            .suffix(" m"),
                    )
                    .on_hover_text(crate::i18n::t("The project's zero above sea level"));
                });
                crate::props::row(ui, "S-JTSK origin", |ui| {
                    let mut on = draft.sjtsk.is_some();
                    if ui
                        .checkbox(&mut on, "")
                        .on_hover_text(crate::i18n::t("Where the project's origin (0, 0) lies in S-JTSK (EPSG:5514), east and north"))
                        .changed()
                    {
                        draft.sjtsk = on.then_some([0.0, 0.0]);
                    }
                    if let Some([e, _]) = &mut draft.sjtsk {
                        ui.add(
                            egui::DragValue::new(e)
                                .speed(0.01)
                                .max_decimals(3)
                                .prefix("E ")
                                .suffix(" m"),
                        );
                    }
                });
                // north under east: both on one line ran into the next column
                if let Some([_, n]) = &mut draft.sjtsk {
                    crate::props::row(ui, "", |ui| {
                        ui.add_space(22.0);
                        ui.add(
                            egui::DragValue::new(n)
                                .speed(0.01)
                                .max_decimals(3)
                                .prefix("N ")
                                .suffix(" m"),
                        );
                    });
                }
                crate::props::row(ui, "True north", |ui| {
                    ui.add(
                        egui::DragValue::new(&mut draft.true_north)
                            .range(-180.0..=180.0)
                            .speed(0.5)
                            .suffix("°"),
                    )
                    .on_hover_text(crate::i18n::t("How far true north turns from the drawings' up, anticlockwise"));
                });
                crate::props::row_wrapped(ui, "Rotate project north", |ui| {
                    let key = egui::Id::new("project.north.turn");
                    let mut by: f64 = ui.data(|d| d.get_temp(key)).unwrap_or(0.0);
                    ui.add(
                        egui::DragValue::new(&mut by)
                            .range(-180.0..=180.0)
                            .speed(0.5)
                            .max_decimals(4)
                            .suffix("°"),
                    )
                    .on_hover_text(crate::i18n::t("The site plans turn by this angle, anticlockwise, on screen and paper. The model stays as it was drawn, so the floor plans keep their way up; the S-JTSK coordinates and heights stay true"));
                    ui.data_mut(|d| d.insert_temp(key, by));
                    if ui
                        .add_enabled(by != 0.0, egui::Button::new(crate::i18n::t("Rotate")))
                        .clicked()
                    {
                        north_turn = Some(by);
                        ui.data_mut(|d| d.insert_temp(key, 0.0f64));
                    }
                });
                crate::props::row_wrapped(ui, "Rotate the whole model", |ui| {
                    let key = egui::Id::new("project.model.turn");
                    let mut by: f64 = ui.data(|d| d.get_temp(key)).unwrap_or(0.0);
                    ui.add(
                        egui::DragValue::new(&mut by)
                            .range(-180.0..=180.0)
                            .speed(0.5)
                            .max_decimals(4)
                            .suffix("°"),
                    )
                    .on_hover_text(crate::i18n::t("The whole project (building, terrain, parcels, maps) turns about its origin by this angle, anticlockwise: the floor plans draw turned too. True north stays where it is on the ground, and the S-JTSK coordinates and heights stay true. A turn made with it is undone by the same angle the other way"));
                    ui.data_mut(|d| d.insert_temp(key, by));
                    if ui
                        .add_enabled(by != 0.0, egui::Button::new(crate::i18n::t("Rotate")))
                        .clicked()
                    {
                        model_turn = Some(by);
                        ui.data_mut(|d| d.insert_temp(key, 0.0f64));
                    }
                });
                if ui
                    .button(crate::i18n::t("Align project north by a line in the plan…"))
                    .on_hover_text(crate::i18n::t("Then click a line in a plan (a parcel boundary, a map line, a wall): the site plans turn so that it lies exactly level or upright, whichever is nearer; the floor plans stay as drawn"))
                    .clicked()
                {
                    north_line = true;
                }
                crate::props::section(ui, "Building");
                text(ui, "Name", &mut draft.building_name, "");
                text(ui, "Use", &mut draft.building_use, "Dwelling");
                crate::props::section(ui, "Standards");
                crate::props::row(ui, "Codes", |ui| {
                    egui::ComboBox::from_id_salt("project.norms")
                        .icon(crate::props::combo_icon)
                        .selected_text(crate::i18n::tw(draft.norms.label()))
                        .width(ui.available_width())
                        .show_ui(ui, |ui| {
                            for set in creabim_core::norms::NormSet::ALL {
                                ui.selectable_value(&mut draft.norms, set, crate::i18n::tw(set.label()));
                            }
                        })
                        .response
                        .on_hover_text(crate::i18n::t(
                            "The standards the building services' calculations follow: Slovak first, Czech where the building is in the Czech Republic",
                        ));
                });
                }
                _ => {
                crate::props::section(ui, "Fire safety");
                if ui
                    .button(crate::i18n::t("Fire calculation parameters…"))
                    .on_hover_text(crate::i18n::t(
                        "The norms' tables the fire calculations count with: read from your file, changed when a norm changes, saved with the project",
                    ))
                    .clicked()
                {
                    fire_params = true;
                }
                if ui
                    .button(crate::i18n::t("Propose fire closures of doors"))
                    .on_hover_text(crate::i18n::t(
                        "Sets the fire rating STN 92 0201-2 asks of each door between fire zones (table 5) and in an outer wall inside another zone's fire-dangerous space (5.4.10), where the door has none or a weaker one",
                    ))
                    .clicked()
                {
                    fire_doors = true;
                }
                crate::props::row_wrapped(ui, "Structural system for fire", |ui| {
                    const SYSTEMS: [(&str, &str); 5] = [
                        ("N", "Non-combustible"),
                        ("Za", "Mixed, 2.6.4 a)"),
                        ("Zb", "Mixed, 2.6.4 b)"),
                        ("Ha", "Combustible, 2.6.5 a)"),
                        ("Hb", "Combustible, 2.6.5 b)"),
                    ];
                    let now = if draft.fire_system.is_empty() {
                        "N"
                    } else {
                        draft.fire_system.as_str()
                    };
                    let shown = SYSTEMS
                        .iter()
                        .find(|s| s.0 == now)
                        .map_or(now.to_string(), |s| crate::i18n::t(s.1).to_string());
                    let mut picked = None;
                    egui::ComboBox::from_id_salt("project.fire_system")
                        .selected_text(shown)
                        .show_ui(ui, |ui| {
                            for (code, name) in SYSTEMS {
                                if ui.selectable_label(now == code, crate::i18n::t(name)).clicked() {
                                    picked = Some(code);
                                }
                            }
                        });
                    if let Some(c) = picked {
                        draft.fire_system = if c == "N" { String::new() } else { c.into() };
                    }
                });
                // the ETICS fire barriers (Weber ETICS manual s. 16-17)
                crate::props::section(ui, "ETICS fire barriers");
                {
                    let eb = &mut draft.etics_barriers;
                    let mut on = !eb.off;
                    if crate::props::switch(ui, &mut on, "Made automatically")
                        .on_hover_text(crate::i18n::t(
                            "Bands of mineral wool in the burning insulation (EPS) of outer walls: the plinth one and the first about the lowest upper floor, over 12 m one about every floor (Weber ETICS manual p. 16, after STN 73 0802). The walls stay whole; the bands show in elevations, sections and 3D and the material take-off counts them.",
                        ))
                        .changed()
                    {
                        eb.off = !on;
                    }
                    if on {
                        let mm = |ui: &mut egui::Ui, label: &str, v: f64, lo: f64, hi: f64| -> f64 {
                            let mut x = (v * 1000.0).round();
                            crate::props::row_wrapped(ui, label, |ui| {
                                ui.add(
                                    egui::DragValue::new(&mut x)
                                        .range(lo..=hi)
                                        .speed(10.0)
                                        .suffix(" mm"),
                                );
                            });
                            x / 1000.0
                        };
                        let top = mm(ui, "Plinth barrier top over ground", eb.plinth_top(), 100.0, 600.0);
                        if (top - eb.plinth_top()).abs() > 1e-9 {
                            eb.plinth_top = Some(top);
                        }
                        let w = mm(ui, "Barrier height", eb.width(), 150.0, 400.0);
                        if (w - eb.width()).abs() > 1e-9 {
                            eb.width = Some(w);
                        }
                        crate::props::switch(ui, &mut eb.every_floor, "At every floor");
                        crate::props::switch(ui, &mut eb.over_openings, "Over the openings")
                            .on_hover_text(crate::i18n::t(
                                "Bands over each opening, 500 mm past its sides, instead of round the building about the floors; none over openings of at most 500 × 500 mm (Weber p. 17)",
                            ));
                        let mut parapet = !eb.no_parapet;
                        if crate::props::switch(ui, &mut parapet, "Parapet band under a roof that burns")
                            .changed()
                        {
                            eb.no_parapet = !parapet;
                        }
                        let model = self.doc.model();
                        let wools: Vec<(creabim_core::ElementId, String)> =
                            creabim_core::library::materials(model)
                                .into_iter()
                                .filter(|(_, _, m)| m.category == "insulation")
                                .map(|(id, n, _)| (id, n.to_string()))
                                .collect();
                        crate::props::row_wrapped(ui, "Barrier material", |ui| {
                            let shown = eb
                                .material
                                .and_then(|id| wools.iter().find(|w| w.0 == id))
                                .map_or_else(|| crate::i18n::t("Façade mineral wool").to_string(), |w| w.1.clone());
                            egui::ComboBox::from_id_salt("project.etics_wool")
                                .selected_text(shown)
                                .show_ui(ui, |ui| {
                                    if ui
                                        .selectable_label(eb.material.is_none(), crate::i18n::t("Façade mineral wool"))
                                        .clicked()
                                    {
                                        eb.material = None;
                                    }
                                    for (id, n) in &wools {
                                        if ui.selectable_label(eb.material == Some(*id), n).clicked() {
                                            eb.material = Some(*id);
                                        }
                                    }
                                });
                        });
                    }
                }
                crate::props::section(ui, "Fire separation distances");
                ui.label(
                    egui::RichText::new(crate::i18n::t(
                        "The distance of each side of the building, as the norm's tables give it. Plans with the fire zones switched on draw the fire-dangerous space from it.",
                    ))
                    .weak(),
                );
                draft.fire_distances.resize(8, 0.0);
                egui::Grid::new("project.fire_distances")
                    .num_columns(4)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        for (i, d) in draft.fire_distances.iter_mut().enumerate() {
                            let side = creabim_core::words::tr(creabim_geom::firezones::FACING[i]);
                            ui.label(side);
                            ui.add(
                                egui::DragValue::new(d)
                                    .range(0.0..=200.0)
                                    .speed(0.05)
                                    .max_decimals(2)
                                    .suffix(" m"),
                            );
                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });
                if draft.fire_distances.iter().all(|d| *d <= 0.0) {
                    draft.fire_distances.clear();
                }
                }
                }
                }
                }
                });
                ui.label(
                    egui::RichText::new(
                        crate::i18n::t("These fill the title blocks of every sheet (a value a sheet sets itself still wins) and name the project, site and building in IFC exports."),
                    )
                    .weak(),
                );
                    });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    apply = ui
                        .add_enabled(draft != current, egui::Button::new(crate::i18n::t("Apply")))
                        .clicked();
                    close = ui.button(crate::i18n::t("Close")).clicked();
                });
            });
        if apply {
            let tx = creabim_core::set_project(self.doc.model(), draft.clone());
            let r = self.doc.commit(tx);
            self.report(
                r,
                "Project information applied: the title blocks show it now.".to_string(),
            );
        }
        if let Some(by) = north_turn {
            let tx = creabim_core::project::turn_site_plans(self.doc.model(), by);
            let r = self.doc.commit(tx);
            self.report(
                r,
                crate::i18n::tr(&format!(
                    "Site plans turned by {by}°; the floor plans stay as drawn. Ctrl+Z to undo."
                )),
            );
        }
        if let Some(by) = model_turn {
            let tx = creabim_core::project::rotate_north(self.doc.model(), by);
            let r = self.doc.commit(tx);
            let now = self
                .doc
                .model()
                .project_info()
                .map_or(0.0, |(_, p)| p.true_north);
            draft.true_north = now;
            self.report(
                r,
                crate::i18n::tr(&format!(
                    "Project north turned by {by}°: true north is now at {now}°. Ctrl+Z to undo."
                )),
            );
        }
        if cadastre {
            self.status = match plot_from_cadastre(self.doc.model(), &mut draft) {
                Ok(s) => s,
                Err(e) => crate::i18n::tr(&format!(
                    "The cadastre did not answer ({e}): check the connection, or type the plot's data in."
                )),
            };
        }
        if fire_params {
            self.open_fire_params();
        }
        if fire_doors {
            self.propose_fire_doors();
        }
        if north_line {
            self.plan.north_pick = true;
            close = true;
            self.status = "Click a line in a plan: the site plans turn so that it lies level or upright. Esc leaves it.".into();
        }
        self.project_open = open && !close;
        self.project_draft = self.project_open.then_some(draft);
    }
}

/// The parcel under the building asked of the cadastre's public service
/// and written to `draft`: its number, district and area. The answer for
/// the status line.
fn plot_from_cadastre(
    model: &creabim_core::Model,
    draft: &mut creabim_core::ProjectInfo,
) -> Result<String, String> {
    use creabim_exchange::kataster::{eskn_url, parse_eskn, Register};
    let [e0, n0] = draft.sjtsk.ok_or("no S-JTSK origin")?;
    // the middle of the building, else the project's origin
    let ends: Vec<glam::DVec2> = model
        .iter()
        .filter_map(|e| match &e.kind {
            creabim_core::ElementKind::Wall(w) => Some([w.start, w.end]),
            _ => None,
        })
        .flatten()
        .collect();
    let mid = if ends.is_empty() {
        glam::DVec2::ZERO
    } else {
        let lo = ends.iter().fold(ends[0], |a, p| a.min(*p));
        let hi = ends.iter().fold(ends[0], |a, p| a.max(*p));
        (lo + hi) / 2.0
    };
    // read off the map under it (moved with the ground), else the origin
    // turned by true north (anticlockwise from the drawings' up)
    let p = match creabim_core::mapgroup::sjtsk_of(model, mid) {
        Some(q)
            if model
                .project_info()
                .is_some_and(|(_, i)| i.map_ref.is_some()) =>
        {
            q - glam::DVec2::new(e0, n0)
        }
        _ => glam::DVec2::from_angle(-draft.true_north.to_radians()).rotate(mid),
    };
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(10))
        .build();
    let mut last = String::from("no parcel there");
    for reg in [Register::C, Register::E] {
        let url = eskn_url(e0 + p.x, n0 + p.y, reg);
        let text = match agent.get(&url).call() {
            Ok(r) => r.into_string().map_err(|e| e.to_string())?,
            Err(e) => {
                last = e.to_string();
                continue;
            }
        };
        let Some(k) = parse_eskn(&text, reg) else {
            continue;
        };
        if let Some(n) = &k.number {
            draft.parcels = n.clone();
        }
        if let Some(d) = &k.district {
            draft.cadastre = d.clone();
        }
        if let Some(a) = k.area {
            draft.plot_area = Some(a);
        }
        if let Some(lv) = &k.sheet {
            draft.ownership_sheet = lv.clone();
        }
        return Ok(format!(
            "{}: {} {}, {} {}, {} m², {} {}. {}",
            crate::i18n::t("From the cadastre"),
            crate::i18n::t("plot no."),
            k.number.as_deref().unwrap_or("?"),
            crate::i18n::t("district"),
            k.district.as_deref().unwrap_or("?"),
            k.area.map_or("?".to_string(), |a| format!("{a:.0}")),
            crate::i18n::t("sheet of ownership"),
            k.sheet.as_deref().unwrap_or("?"),
            crate::i18n::t("Apply writes it to the project. A plot of more parcels: add their numbers and areas by hand."),
        ));
    }
    Err(last)
}

/// The logo's mark as a white texture, to tint with a theme's colour.
pub(crate) fn logo_texture(ctx: &egui::Context) -> egui::TextureHandle {
    static TEX: std::sync::OnceLock<egui::TextureHandle> = std::sync::OnceLock::new();
    TEX.get_or_init(|| {
        let img =
            image::load_from_memory(include_bytes!("../assets/logo/png/creabim-mark-128.png"))
                .map(|i| i.to_rgba8())
                .unwrap_or_default();
        let (w, h) = img.dimensions();
        let px: Vec<egui::Color32> = img
            .pixels()
            .map(|p| egui::Color32::from_rgba_unmultiplied(255, 255, 255, p[3]))
            .collect();
        ctx.load_texture(
            "creabim-logo",
            egui::ColorImage::new([w as usize, h as usize], px),
            egui::TextureOptions::LINEAR,
        )
    })
    .clone()
}

#[cfg(test)]
mod close_tests {
    use super::*;
    use std::time::Duration;

    /// One edit, as a user's: a copy of a material, one undo step.
    fn edit(app: &mut CreaBimApp, name: &str) {
        let kind = app
            .doc
            .model()
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Material(_)))
            .unwrap()
            .kind
            .clone();
        let e = creabim_core::Element::new(name, kind);
        app.doc
            .commit(creabim_core::Transaction::new(format!("Add {name}")).insert(e))
            .unwrap();
    }

    fn saved_app(tag: &str) -> (CreaBimApp, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("creabim-close-{tag}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut app = test_app();
        app.path = dir.join("Terrace house.creabim").display().to_string();
        app.save_file().unwrap();
        (app, dir)
    }

    #[test]
    fn the_line_says_how_many_changes_and_how_long_ago() {
        assert_eq!(ago(Duration::from_secs(20)), "just now");
        assert_eq!(ago(Duration::from_secs(60)), "1 minute ago");
        assert_eq!(ago(Duration::from_secs(4 * 60 + 30)), "4 minutes ago");
        assert_eq!(ago(Duration::from_secs(2 * 3600)), "2 hours ago");
        assert_eq!(ago(Duration::from_secs(3 * 86_400)), "3 days ago");
        assert_eq!(
            changes_line(23, Some(Duration::from_secs(18 * 60))),
            "23 changes since it was last saved, 18 minutes ago."
        );
        assert_eq!(
            changes_line(1, Some(Duration::from_secs(5))),
            "1 change since it was last saved, just now."
        );
    }

    #[test]
    fn a_failed_save_says_why_in_plain_words() {
        assert!(save_error("Access is denied. (os error 5)").contains("read-only"));
        assert!(save_error("Read-only file system (os error 30)").contains("read-only"));
        assert!(
            save_error("There is not enough space on the disk. (os error 112)")
                .contains("disk is full")
        );
        assert!(save_error("something odd").contains("something odd"));
    }

    #[test]
    fn the_changes_are_counted_in_undo_steps_since_the_save() {
        let (mut app, dir) = saved_app("count");
        assert_eq!(app.changes_since_saved(), 0);
        edit(&mut app, "A");
        edit(&mut app, "B");
        edit(&mut app, "C");
        assert_eq!(app.changes_since_saved(), 3);
        app.doc.undo();
        assert_eq!(app.changes_since_saved(), 2);
        let s = app.close_state_json();
        assert_eq!(s["fresh"], false);
        assert_eq!(s["file"], "Terrace house.creabim");
        assert!(s["line"]
            .as_str()
            .unwrap()
            .starts_with("2 changes since it was last saved, just now"));
        app.save_file().unwrap();
        assert_eq!(app.changes_since_saved(), 0);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn save_spins_then_settles_then_closes() {
        let ctx = egui::Context::default();
        let (mut app, dir) = saved_app("save");
        edit(&mut app, "A");
        app.close_ask = true;
        app.close_do(&ctx, "save");
        assert_eq!(app.close_state_json()["phase"], "saving");
        // the page gets a frame to show Saving… first
        app.close_step(&ctx);
        assert!(app.unsaved());
        std::thread::sleep(Duration::from_millis(60));
        app.close_step(&ctx);
        assert!(!app.unsaved(), "written");
        assert_eq!(
            app.close_state_json()["phase"],
            "saving",
            "it spins a moment"
        );
        std::thread::sleep(SAVING_AT_LEAST);
        app.close_step(&ctx);
        assert_eq!(app.close_state_json()["phase"], "saved");
        assert!(!app.close_ok);
        std::thread::sleep(SAVED_HOLDS);
        app.close_step(&ctx);
        assert!(app.close_ok && !app.close_ask, "then CreaBIM closes");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn replace_the_file_is_asked_in_the_app_and_answered_there() {
        let ctx = egui::Context::default();
        let (mut app, dir) = saved_app("overwrite");
        edit(&mut app, "A");
        let there = dir.join("there.creabim");
        std::fs::write(&there, "old").unwrap();
        let there = there.display().to_string();
        // Cancel: nothing is written, the project keeps its file
        let was = app.path.clone();
        app.ask_overwrite(there.clone(), OverwriteThen::SaveAs);
        assert_eq!(app.overwrite_state_json()["file"], "there.creabim");
        app.overwrite_answer(&ctx, false);
        assert!(app.overwrite.is_none() && app.unsaved());
        assert_eq!(app.path, was);
        assert_eq!(std::fs::read_to_string(&there).unwrap(), "old");
        // Replace: the project is written over it
        app.ask_overwrite(there.clone(), OverwriteThen::SaveAs);
        app.overwrite_answer(&ctx, true);
        assert!(app.overwrite.is_none() && !app.unsaved());
        assert_eq!(app.path, there);
        assert_ne!(std::fs::read_to_string(&there).unwrap_or_default(), "old");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_failed_save_keeps_the_project_open_and_offers_save_as() {
        let ctx = egui::Context::default();
        let (mut app, dir) = saved_app("fail");
        edit(&mut app, "A");
        // a folder where the file should be: nothing can be written there
        let blocked = dir.join("blocked.creabim");
        std::fs::create_dir_all(&blocked).unwrap();
        app.path = blocked.display().to_string();
        app.close_ask = true;
        app.close_do(&ctx, "save");
        std::thread::sleep(Duration::from_millis(60));
        app.close_step(&ctx);
        let s = app.close_state_json();
        assert_eq!(s["phase"], "failed");
        assert!(!s["error"].as_str().unwrap().is_empty());
        assert!(app.close_ask && !app.close_ok && app.unsaved());
        // Don't save still lets go; Cancel keeps it open
        app.close_do(&ctx, "cancel");
        assert!(!app.close_ask && !app.close_ok);
        assert_eq!(app.status, "Close cancelled: the project is still open.");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_project_with_no_file_asks_for_a_new_one() {
        let mut app = test_app();
        app.path = std::env::temp_dir()
            .join("creabim-never-saved-here/project.creabim")
            .display()
            .to_string();
        edit(&mut app, "A");
        let s = app.close_state_json();
        assert_eq!(s["fresh"], true);
        assert_eq!(s["line"], "It hasn't been saved to a file yet.");
    }

    #[test]
    fn dont_save_closes_at_once() {
        let ctx = egui::Context::default();
        let (mut app, dir) = saved_app("discard");
        edit(&mut app, "A");
        app.close_ask = true;
        app.close_do(&ctx, "discard");
        assert!(app.close_ok && !app.close_ask && app.unsaved());
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_energy_assessment_goes_out_for_isover_fragment() {
        let mut app = test_app();
        let dir = std::env::temp_dir().join(format!("creabim-fragment-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        app.path = dir.join("house.creabim").display().to_string();
        app.export("export.fragment");
        assert!(app.status.starts_with("Exported "), "{}", app.status);
        let text = std::fs::read_to_string(dir.join("house-fragment.json")).unwrap();
        let v: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(v["Skladba"]["stavby"].is_object() && v["popis_zakazky"].is_object());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn after_an_export_the_status_bar_offers_its_folder() {
        let mut app = test_app();
        let dir = std::env::temp_dir().join(format!("creabim-export-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        app.path = dir.join("house.creabim").display().to_string();
        app.export("export.ifcx");
        assert!(app.status.starts_with("Exported "), "{}", app.status);
        assert!(dir.join("house.ifcx").exists());
        let has = |app: &CreaBimApp| {
            app.status_state()["controls"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["id"] == "export.folder")
        };
        assert!(has(&app));
        assert_eq!(
            app.last_export_dir
                .as_deref()
                .map(|d| d.canonicalize().unwrap()),
            Some(dir.canonicalize().unwrap())
        );
        // once the bar says something else, the button goes
        app.status = "Moved.".into();
        assert!(!has(&app));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_placed_footing_offers_the_foundation_plan_and_makes_one_if_need_be() {
        let mut app = test_app();
        assert!(crate::memberstool::foundation_plan(app.doc.model()).is_none());
        let placed = |app: &mut CreaBimApp| {
            app.plan.footing_placed = true;
            app.status = "Strip footing placed under GF East. Ctrl+Z to undo.".into();
        };
        let offered = |app: &CreaBimApp| {
            app.status_state()["controls"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["id"] == "footing.foundation")
                .map(|c| c["label"].as_str().unwrap().to_string())
        };
        // without one, it is made through the footings, with nothing of
        // the floor above (George 2026-11-20), and opened
        placed(&mut app);
        assert_eq!(offered(&app).as_deref(), Some("Make a Foundation plan"));
        app.show_foundation_plan();
        let made = crate::memberstool::foundation_plan(app.doc.model()).unwrap();
        assert_eq!(app.active_view, Some(made));
        let ElementKind::View(v) = &app.doc.model().get(made).unwrap().kind else {
            panic!()
        };
        assert_eq!(v.underlay, creabim_core::Underlay::None);
        // once there is one, it is shown
        app.active_view = None;
        placed(&mut app);
        assert_eq!(offered(&app).as_deref(), Some("Show in Foundation plan"));
        app.show_foundation_plan();
        assert_eq!(app.active_view, Some(made));
        // and the button goes once the bar says something else
        placed(&mut app);
        app.status = "Moved.".into();
        assert!(offered(&app).is_none());
    }

    #[test]
    fn frame_all_reframes_only_the_pane_it_is_asked_in() {
        let mut app = test_app();
        let (a, b) = (ElementId::new(), ElementId::new());
        app.active_view = Some(a);
        app.rect_2d = Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(400.0, 300.0),
        ));
        app.plan.cameras.insert(a, (glam::DVec2::ZERO, 3.0));
        app.plan.cameras.insert(b, (glam::DVec2::ZERO, 3.0));
        app.camera.distance = 1234.0;
        app.frame_pane(true);
        assert!(!app.plan.cameras.contains_key(&a));
        assert!(app.plan.cameras.contains_key(&b));
        assert_eq!(app.camera.distance, 1234.0);
        app.plan.cameras.insert(a, (glam::DVec2::ZERO, 3.0));
        app.frame_pane(false);
        assert_ne!(app.camera.distance, 1234.0);
        assert!(app.plan.cameras.contains_key(&a));
    }

    #[test]
    fn the_units_switch_changes_the_project_units_in_one_undo() {
        let mut app = test_app();
        assert_eq!(app.doc.model().units().length, LengthUnit::Millimetres);
        let items = app.unit_items();
        let ids: Vec<String> = items
            .iter()
            .filter_map(|i| match i {
                Item::Action { id, .. } => Some(id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            ids.len(),
            12,
            "six units, five number formats and the window: {ids:?}"
        );
        let ctx = egui::Context::default();
        app.run_action(&ctx, "units.set:3");
        assert_eq!(app.doc.model().units().length, LengthUnit::FeetInches);
        assert_eq!(crate::units::get().length, LengthUnit::FeetInches);
        app.run_command(&ctx, crate::commands::Command::Undo, None);
        assert_eq!(app.doc.model().units().length, LengthUnit::Millimetres);
    }

    #[test]
    fn the_application_menu_has_about_updates_settings_and_exit() {
        let app = test_app();
        let ids: Vec<String> = app
            .app_menu()
            .into_iter()
            .filter_map(|i| match i {
                Item::Action { id, .. } => Some(id),
                _ => None,
            })
            .collect();
        for want in ["app.about", "app.updates", "app.settings", "app.exit"] {
            assert!(ids.iter().any(|i| i == want), "{want} in {ids:?}");
        }
    }

    #[test]
    fn samples_open_as_new_unsaved_projects() {
        let mut app = test_app();
        app.path = "/work/my house.creabim".into();
        let ctx = egui::Context::default();
        app.run_action(&ctx, "new.garden");
        assert_eq!(app.path, "House in its garden.creabim");
        app.run_action(&ctx, "new.empty");
        assert_eq!(app.path, "Untitled.creabim");
        assert_eq!(app.doc.model().levels().count(), 2);
        // one New project… in the File menu, the samples under Help
        let file: Vec<String> = app
            .menu_items("menu:File")
            .into_iter()
            .filter_map(|i| match i {
                Item::Action { id, .. } if id.starts_with("new.") => Some(id),
                _ => None,
            })
            .collect();
        assert_eq!(file, ["new.project", "new.sheet"]);
        let help = app.help_menu();
        assert!(help.iter().any(|i| matches!(i, Item::Sub { label, items, .. } if label == "Samples to learn from" && items.len() == 6)));
    }

    #[test]
    fn a_file_named_on_the_command_line_is_opened() {
        let args = ["--verbose", "house.creabim"].map(String::from);
        assert_eq!(
            launch_file(args.into_iter()).as_deref(),
            Some("house.creabim")
        );
        assert_eq!(launch_file(std::iter::empty()), None);
        let dir = std::env::temp_dir().join("creabim-launch-test");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("small.creabim.json");
        let model = sample_house();
        std::fs::write(&path, model.to_json()).unwrap();
        let mut app = CreaBimApp::blank(None);
        app.open_path(path.to_str().unwrap());
        // every element, and the material take-off a project always lists
        let takeoff = |m: &creabim_core::Model| {
            m.iter().any(|e| {
                matches!(&e.kind, ElementKind::View(v)
                    if matches!(v.kind, ViewKind::Schedule { category: ScheduleCategory::Materials, .. }))
            })
        };
        assert!(takeoff(app.doc.model()));
        let added = usize::from(!takeoff(&model));
        assert_eq!(app.doc.model().len(), model.len() + added);
    }
}
