//! A point cloud read in the background: a big scan (hundreds of
//! megabytes, or a file a cloud drive first fetches) is read while the
//! window keeps drawing, with how far it has got and a Cancel.

use super::CreaBimApp;
use creabim_core::ElementId;
use creabim_exchange::cloud::Cloud;
use eframe::egui;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc};

/// A cloud being read for a view.
pub struct CloudLoad {
    pub view: ElementId,
    pub path: String,
    /// The file's size, when known.
    size: u64,
    read: Arc<AtomicU64>,
    cancel: Arc<AtomicBool>,
    done: mpsc::Receiver<Result<Cloud, String>>,
    started: std::time::Instant,
}

impl CloudLoad {
    pub fn start(view: ElementId, path: String) -> Self {
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let read = Arc::new(AtomicU64::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, done) = mpsc::channel();
        {
            let (path, read, cancel) = (path.clone(), read.clone(), cancel.clone());
            std::thread::spawn(move || {
                let r = creabim_exchange::cloud::read_file_with(
                    &path,
                    creabim_exchange::cloud::MOST,
                    &|n| {
                        read.store(n, Ordering::Relaxed);
                        !cancel.load(Ordering::Relaxed)
                    },
                );
                tx.send(r).ok();
            });
        }
        Self {
            view,
            path,
            size,
            read,
            cancel,
            done,
            started: std::time::Instant::now(),
        }
    }

    /// How far it has got, 0 to 1.
    pub fn fraction(&self) -> f32 {
        if self.size == 0 {
            0.0
        } else {
            (self.read.load(Ordering::Relaxed) as f64 / self.size as f64).clamp(0.0, 1.0) as f32
        }
    }
}

impl CreaBimApp {
    /// Read a cloud for `view` in the background; it is added when read.
    pub(super) fn load_cloud(&mut self, view: ElementId, path: String) {
        if let Some(l) = &self.cloud_load {
            l.cancel.store(true, Ordering::Relaxed);
        }
        self.status = format!("{} {path}", crate::i18n::t("Reading the point cloud"));
        self.cloud_load = Some(CloudLoad::start(view, path));
    }

    /// The reading's window, while it reads; the cloud added once read.
    pub(super) fn cloud_load_window(&mut self, ctx: &egui::Context) {
        let Some(load) = &self.cloud_load else {
            return;
        };
        match load.done.try_recv() {
            Ok(Ok(c)) => {
                let (view, path) = (load.view, load.path.clone());
                self.cloud_load = None;
                self.plan.ref_cache.put_cloud(&path, Arc::new(c));
                self.add_reference(view, creabim_core::RefSource::points(path));
                ctx.request_repaint();
                return;
            }
            Ok(Err(e)) => {
                self.status = if e == creabim_exchange::cloud::CANCELLED {
                    crate::i18n::tr("Reading the point cloud cancelled.")
                } else {
                    e
                };
                self.cloud_load = None;
                return;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.cloud_load = None;
                return;
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
        let name = std::path::Path::new(&load.path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| load.path.clone());
        let f = load.fraction();
        let mb = |b: u64| b as f64 / 1_048_576.0;
        let read = load.read.load(Ordering::Relaxed);
        let secs = load.started.elapsed().as_secs();
        let mut cancel = false;
        crate::app::shell::small_window(crate::i18n::tr("Point cloud"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_width(360.0);
                ui.label(egui::RichText::new(&name).strong());
                ui.add(egui::ProgressBar::new(f).show_percentage().animate(true));
                ui.label(
                    egui::RichText::new(if read == 0 {
                        // a cloud drive's file is fetched before a byte comes
                        format!(
                            "{} ({secs} s)",
                            crate::i18n::t("Waiting for the file (a file kept only in the cloud, such as OneDrive's, is downloaded first)")
                        )
                    } else {
                        format!("{:.1} / {:.1} MB ({secs} s)", mb(read), mb(load.size))
                    })
                    .small()
                    .weak(),
                );
                if load.size > 100 << 20 {
                    ui.label(
                        egui::RichText::new(crate::i18n::t(
                            "A big scan keeps at most 6 million of its points, evenly thinned.",
                        ))
                        .small()
                        .weak(),
                    );
                }
                ui.add_space(4.0);
                if ui.button(crate::i18n::t("Cancel")).clicked() {
                    cancel = true;
                }
            });
        if cancel {
            load.cancel.store(true, Ordering::Relaxed);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}
