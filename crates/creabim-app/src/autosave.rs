//! Work is never lost to a crash: while the project has changes it is
//! written every few minutes to an autosave file beside it (or in the
//! settings folder while it has no file of its own yet), on a worker so the
//! window never waits. Saving removes the autosave and keeps the file it
//! replaced as the project's one backup. Opening a project whose autosave
//! is newer than it offers to bring the autosaved work back.

use super::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Where autosaving is.
#[derive(Default)]
pub struct State {
    /// When the model was last autosaved or saved (egui time, seconds).
    pub since: Option<f64>,
    /// The revision last autosaved.
    pub revision: Option<u64>,
    /// An autosave being written on a worker.
    pub busy: Arc<AtomicBool>,
    /// A newer autosave found on opening: the project's path and it.
    pub offer: Option<(String, PathBuf)>,
    /// Since when workers have been building drawings (egui time).
    pub busy_since: Option<f64>,
}

/// The autosave of a project at `path`: beside it, or in the settings
/// folder for a project that has no file yet.
pub fn file_for(path: &str) -> Option<PathBuf> {
    let p = std::path::Path::new(path);
    if p.is_absolute() {
        return Some(creabim_store::autosave_path(p));
    }
    let dir = crate::files::settings_dir()?.join("autosave");
    std::fs::create_dir_all(&dir).ok()?;
    Some(creabim_store::autosave_path(
        dir.join(
            p.file_name()
                .unwrap_or(std::ffi::OsStr::new("project.creabim")),
        ),
    ))
}

impl CreaBimApp {
    /// Once a frame: autosave when the changes have waited long enough.
    pub(crate) fn autosave_tick(&mut self, ctx: &egui::Context) {
        let minutes = self.shell.settings.autosave_minutes;
        let now = ctx.input(|i| i.time);
        let since = *self.autosave.since.get_or_insert(now);
        let rev = self.doc.model().revision();
        if minutes == 0 || !self.unsaved() || self.autosave.revision == Some(rev) {
            return;
        }
        let wait = minutes as f64 * 60.0 - (now - since);
        if wait > 0.0 {
            // wake for it, the app sleeping otherwise
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(wait));
            return;
        }
        if self.autosave.busy.swap(true, Ordering::SeqCst) {
            return;
        }
        let Some(file) = file_for(&self.path) else {
            self.autosave.busy.store(false, Ordering::SeqCst);
            return;
        };
        self.autosave.since = Some(now);
        self.autosave.revision = Some(rev);
        let model = self.doc.model().clone();
        let busy = self.autosave.busy.clone();
        std::thread::spawn(move || {
            let doc = creabim_core::Document::from_model(model);
            match creabim_store::save(&file, &doc) {
                Ok(()) => crate::actlog::line(format!("autosaved: {}", file.display())),
                Err(e) => crate::actlog::line(format!("autosave failed: {e}")),
            }
            busy.store(false, Ordering::SeqCst);
        });
    }

    /// The project was saved (or let go): its autosave goes.
    pub(crate) fn autosave_clear(&mut self) {
        self.autosave.since = None;
        self.autosave.revision = None;
        if let Some(f) = file_for(&self.path) {
            let _ = std::fs::remove_file(f);
        }
    }

    /// A project just opened: offer its autosave if it is newer.
    pub(crate) fn autosave_check(&mut self, path: &str) {
        self.autosave.offer = file_for(path)
            .filter(|f| {
                let when =
                    |p: &std::path::Path| std::fs::metadata(p).and_then(|m| m.modified()).ok();
                match (when(f), when(std::path::Path::new(path))) {
                    (Some(a), Some(p)) => a > p,
                    (Some(_), None) => true,
                    _ => false,
                }
            })
            .map(|f| (path.to_string(), f));
    }

    /// While workers build drawings for longer than a moment, a small
    /// note over the views says the app is still working on them.
    pub(crate) fn busy_note(&mut self, ctx: &egui::Context) {
        let n = self.builds.busy();
        let now = ctx.input(|i| i.time);
        if n == 0 {
            self.autosave.busy_since = None;
            return;
        }
        let since = *self.autosave.busy_since.get_or_insert(now);
        if now - since < 0.4 {
            ctx.request_repaint_after(std::time::Duration::from_millis(400));
            return;
        }
        let text = match n {
            1 => crate::i18n::tr("Calculating the drawing…"),
            n => crate::i18n::tr(&format!("Calculating {n} drawings…")),
        };
        let screen = ctx.content_rect();
        egui::Area::new(egui::Id::new("busy_note"))
            .order(egui::Order::Foreground)
            .interactable(false)
            .fixed_pos(egui::pos2(
                screen.center().x - 110.0,
                screen.bottom() - shell::STATUS_H - 40.0,
            ))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(egui::Spinner::new());
                        ui.label(text);
                    });
                });
            });
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }

    /// The window offering autosaved work back.
    pub(crate) fn autosave_window(&mut self, ctx: &egui::Context) {
        let Some((path, file)) = self.autosave.offer.clone() else {
            return;
        };
        let mut take = None;
        egui::Window::new(crate::i18n::t("Autosaved work found"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label(crate::i18n::t(
                    "CreaBIM closed without saving this project. An autosave newer than the file was found.",
                ));
                ui.label(egui::RichText::new(file.display().to_string()).weak());
                ui.horizontal(|ui| {
                    if ui.button(crate::i18n::t("Bring it back")).clicked() {
                        take = Some(true);
                    }
                    if ui.button(crate::i18n::t("Keep the saved file")).clicked() {
                        take = Some(false);
                    }
                });
            });
        match take {
            Some(true) => {
                self.autosave.offer = None;
                match creabim_store::load(&file) {
                    Ok(doc) => {
                        self.load_model(doc.model().clone());
                        self.path = path;
                        // changed from the file on disk until saved
                        self.saved_revision = None;
                        self.status = "Autosaved work brought back. Save it to keep it.".into();
                    }
                    Err(e) => self.status = format!("Couldn't read the autosave: {e}"),
                }
            }
            Some(false) => {
                self.autosave.offer = None;
                let _ = std::fs::remove_file(&file);
                self.status = "The autosave was let go.".into();
            }
            None => {}
        }
    }
}

/// A crash on the window's thread unwinds through the app: the work goes
/// to the autosave on the way out, to be offered back on the next start.
impl Drop for CreaBimApp {
    fn drop(&mut self) {
        if !std::thread::panicking() || !self.unsaved() {
            return;
        }
        if let Some(file) = file_for(&self.path) {
            let doc = creabim_core::Document::from_model(self.doc.model().clone());
            let r = creabim_store::save(&file, &doc);
            crate::actlog::line(format!("crash: work saved to {}: {r:?}", file.display()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_project_without_a_file_autosaves_in_the_settings_folder() {
        let f = file_for("/work/House.creabim").unwrap();
        assert_eq!(f, PathBuf::from("/work/House.autosave.creabim"));
        if let Some(f) = file_for("project.creabim") {
            assert!(f.ends_with("autosave/project.autosave.creabim"), "{f:?}");
        }
    }
}
