//! The Print window (Ctrl+P): the sheets, the printer, its paper (A4 to
//! A0, a plotter's roll, the sheet's own size), 1:1 or shrunk to the
//! paper, colour or black and white, copies, and a preview of each page
//! as it lands on its paper. Printing runs beside the app, a page a
//! sheet, in one job.

use super::{CreaBimApp, ExportScope};
use crate::printing::{self, Job, Page, Settings};
use creabim_core::{ElementId, Ink};
use creabim_geom::printing::{paper_for, place, Fit, OnPaper, PaperChoice, PaperGot, Placement};
use creabim_geom::sheet::{self as sheets, PlacedView, SheetInfo};
use eframe::egui::{self, Color32};
use glam::DVec2;
use std::collections::{BTreeSet, HashMap};

/// The Print window's choices, kept for the session.
pub(crate) struct PrintState {
    pub open: bool,
    scope: ExportScope,
    chosen: BTreeSet<ElementId>,
    /// The printer's name, or [`printing::PDF`].
    printer: String,
    printers: Vec<String>,
    /// Each printer's own settings, once changed in its Properties.
    settings: HashMap<String, Settings>,
    paper: PaperChoice,
    fit: Fit,
    /// Over each drawing's own ink: colour keeps each as set on its sheet.
    ink: Ink,
    copies: u32,
    /// The page the preview shows.
    page: usize,
    /// What each printer gives for each paper asked, as asked once.
    got: HashMap<String, Option<PaperGot>>,
    job: Option<std::thread::JoinHandle<Result<usize, String>>>,
}

impl Default for PrintState {
    fn default() -> Self {
        Self {
            open: false,
            scope: ExportScope::This,
            chosen: BTreeSet::new(),
            printer: String::new(),
            printers: vec![],
            settings: HashMap::new(),
            paper: PaperChoice::Sheet,
            fit: Fit::Actual,
            ink: Ink::Colour,
            copies: 1,
            page: 0,
            got: HashMap::new(),
            job: None,
        }
    }
}

impl PrintState {
    fn settings(&self) -> Settings {
        self.settings.get(&self.printer).cloned().flatten()
    }

    /// The paper the printer gives a sheet of `sheet` mm, as the sheet
    /// sees it: asked once a printer, paper and settings.
    fn paper_got(&mut self, sheet: DVec2) -> (printing::Page, PaperGot) {
        let ask = paper_for(sheet, self.paper);
        let key = format!(
            "{}|{:?}|{:?}|{}",
            self.printer,
            ask,
            self.settings().map(|s| s.len()),
            self.settings.len()
        );
        let settings = self.settings();
        let printer = self.printer.clone();
        let got = *self
            .got
            .entry(key)
            .or_insert_with(|| printing::paper_got(&printer, &settings, &ask));
        let got = got.unwrap_or_else(|| PaperGot::whole(ask.size.unwrap_or(sheet)));
        (Page { sheet, ask }, got)
    }
}

/// Where a sheet goes on the paper it got.
fn placement(page: &Page, got: &PaperGot, fit: Fit) -> Placement {
    place(page.sheet, got, fit, page.ask.turned)
}

/// Draw sheet `id` onto its paper.
fn draw_on_paper(
    c: &mut dyn creabim_geom::paper::Canvas,
    model: &creabim_core::Model,
    id: ElementId,
    views: &[PlacedView],
    info: &SheetInfo,
    place: Placement,
    ink: Ink,
) {
    let mut on = OnPaper {
        inner: c,
        place,
        ink,
    };
    sheets::draw_sheet(&mut on, model, id, views, info);
}

/// "A1", or the size, of a paper.
fn paper_name(size: DVec2) -> String {
    let turn = if size.x > size.y + 0.5 {
        crate::i18n::tr("landscape")
    } else {
        crate::i18n::tr("portrait")
    };
    match creabim_geom::printing::iso_name(size) {
        Some(n) => format!("{n} {:.0} × {:.0} mm, {turn}", size.x, size.y),
        None => format!("{:.0} × {:.0} mm, {turn}", size.x, size.y),
    }
}

impl CreaBimApp {
    /// Open the Print window: for the sheet in front, else for the sheets
    /// chosen in the browser, else for all.
    pub(super) fn open_print(&mut self) {
        let (names, default) = printing::printers();
        let p = &mut self.print;
        p.printers = names;
        p.printers.push(printing::PDF.into());
        if !p.printers.contains(&p.printer) {
            p.printer = default
                .filter(|d| p.printers.contains(d))
                .unwrap_or_else(|| p.printers[0].clone());
        }
        p.got.clear();
        let all: Vec<ElementId> = sheets::sheets(self.doc.model())
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        let picked: BTreeSet<ElementId> = self
            .selection
            .all()
            .iter()
            .copied()
            .filter(|id| all.contains(id))
            .collect();
        let p = &mut self.print;
        if self.active_view.is_some_and(|v| all.contains(&v)) {
            p.scope = ExportScope::This;
        } else if !picked.is_empty() {
            p.scope = ExportScope::Chosen;
        } else {
            p.scope = ExportScope::All;
        }
        if !picked.is_empty() {
            p.chosen = picked;
        }
        p.page = 0;
        p.open = true;
    }

    /// The sheets the Print window prints, in sheet order.
    fn sheets_to_print(&self) -> Vec<ElementId> {
        let all: Vec<ElementId> = sheets::sheets(self.doc.model())
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        match self.print.scope {
            ExportScope::All => all,
            ExportScope::Chosen => all
                .into_iter()
                .filter(|id| self.print.chosen.contains(id))
                .collect(),
            ExportScope::This => self
                .active_view
                .filter(|id| all.contains(id))
                .into_iter()
                .collect(),
        }
    }

    /// Print the chosen sheets on the chosen printer (the PDF printer
    /// asks where to write its file).
    pub(super) fn print_now(&mut self) {
        let ids = self.sheets_to_print();
        if ids.is_empty() {
            self.status = "No sheets to print: open one, pick some, or choose All.".into();
            return;
        }
        if self.print.job.as_ref().is_some_and(|j| !j.is_finished()) {
            self.status = "Still printing the last job: wait for it to go out.".into();
            return;
        }
        let model = self.doc.model().clone();
        let info = self.sheet_info();
        let mut pages = vec![];
        let mut views = vec![];
        for id in &ids {
            let Some(s) = sheets::sheet_of(&model, *id) else {
                continue;
            };
            let sheet = DVec2::new(s.width, s.height);
            pages.push(Page {
                sheet,
                ask: paper_for(sheet, self.print.paper),
            });
            views.push(self.placed_views(*id));
        }
        let title = {
            let name = model
                .get(ids[0])
                .map(|e| e.name.clone())
                .unwrap_or_default();
            let n = sheets::sheet_of(&model, ids[0])
                .map(|s| s.number.clone())
                .unwrap_or_default();
            if ids.len() == 1 {
                format!("{} {n} {name}", info.project)
            } else {
                format!("{} ({} sheets)", info.project, ids.len())
            }
        };
        let job = Job {
            printer: self.print.printer.clone(),
            settings: self.print.settings(),
            title,
            pages,
            copies: self.print.copies.max(1),
            mono: self.print.ink == Ink::Black,
        };
        let (fit, ink) = (self.print.fit, self.print.ink);
        if job.printer == printing::PDF {
            let default = with_pdf(&self.path, ids.len() == 1, &job.title);
            let Some(out) = self.export_to(default) else {
                return;
            };
            let bytes = printing::pdf(&job, &mut |i, got, c| {
                let p = placement(&job.pages[i], got, fit);
                draw_on_paper(c, &model, ids[i], &views[i], &info, p, ink);
            });
            let n = job.pages.len() * job.copies as usize;
            let r = std::fs::write(&out, bytes);
            self.report(r, format!("Printed {n} pages to {out}"));
            return;
        }
        let n = job.pages.len() * job.copies as usize;
        let printer = job.printer.clone();
        self.print.job = Some(std::thread::spawn(move || {
            printing::print(&job, &mut |i, got, c| {
                let p = placement(&job.pages[i], got, fit);
                draw_on_paper(c, &model, ids[i], &views[i], &info, p, ink);
            })
        }));
        self.status = format!("Printing {n} pages on {printer}…");
    }

    /// Say how the last print job went, once it has.
    fn print_job_done(&mut self) {
        if !self.print.job.as_ref().is_some_and(|j| j.is_finished()) {
            return;
        }
        let Some(job) = self.print.job.take() else {
            return;
        };
        self.status = match job.join() {
            Ok(Ok(n)) => format!(
                "Sent {n} page{} to {}.",
                if n == 1 { "" } else { "s" },
                self.print.printer
            ),
            Ok(Err(e)) => format!("Couldn't print: {e}"),
            Err(_) => "Couldn't print: the printer's driver failed.".into(),
        };
    }

    /// The Print window.
    pub(super) fn print_window(&mut self, ctx: &egui::Context) {
        self.print_job_done();
        if self.print.job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
        }
        if !self.print.open {
            return;
        }
        let model = self.doc.model();
        let all: Vec<(ElementId, String, DVec2)> = sheets::sheets(model)
            .into_iter()
            .map(|(id, s)| {
                let t = model.get(id).map(|e| e.name.clone()).unwrap_or_default();
                (
                    id,
                    format!("{} {t}", s.number),
                    DVec2::new(s.width, s.height),
                )
            })
            .collect();
        let this = self
            .active_view
            .filter(|id| all.iter().any(|(s, _, _)| s == id));
        let ids = self.sheets_to_print();
        self.print.page = self.print.page.min(ids.len().saturating_sub(1));
        // the page previewed, its drawings and its paper
        let shown = ids.get(self.print.page).copied();
        let preview = shown.and_then(|id| {
            let (_, name, size) = all.iter().find(|(s, _, _)| *s == id)?;
            let views = self.placed_views(id);
            let (page, got) = self.print.paper_got(*size);
            Some((id, name.clone(), page, got, views))
        });
        let info = self.sheet_info();
        let mut open = true;
        let (mut go, mut cancel, mut props) = (false, false, false);
        let printing = self.print.job.is_some();
        let model = self.doc.model();
        let x = &mut self.print;
        crate::app::shell::window(crate::i18n::tr("Print sheets"))
            .id(egui::Id::new("print-window"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(760.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(360.0);
                        settings_ui(ui, x, &all, this.is_some(), &mut props);
                    });
                    ui.add_space(14.0);
                    ui.vertical(|ui| {
                        ui.set_width(380.0);
                        preview_ui(ui, x, ids.len(), preview.as_ref(), model, &info);
                    });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let label = if x.printer == printing::PDF {
                            crate::i18n::t("Print to PDF…")
                        } else {
                            crate::i18n::t("Print")
                        };
                        go = ui
                            .add_enabled(!ids.is_empty() && !printing, egui::Button::new(label))
                            .on_disabled_hover_text(crate::i18n::t(
                                "No sheets to print, or the last job is still going out",
                            ))
                            .clicked();
                        cancel = ui.button(crate::i18n::t("Cancel")).clicked();
                    });
                });
            });
        if props {
            let printer = self.print.printer.clone();
            let mut s = self.print.settings();
            if printing::properties(&printer, &mut s) {
                self.print.settings.insert(printer, s);
                self.print.got.clear();
            }
        }
        if go {
            self.print.open = false;
            self.print_now();
        } else if cancel || !open {
            self.print.open = false;
        }
    }
}

/// The PDF the PDF printer writes, by the project's file.
fn with_pdf(path: &str, one: bool, title: &str) -> String {
    let p = std::path::Path::new(path);
    let dir = p.parent().unwrap_or(std::path::Path::new(""));
    let name = if one {
        title
            .trim()
            .chars()
            .map(|c| if "\\/:*?\"<>|".contains(c) { '-' } else { c })
            .collect::<String>()
    } else {
        format!(
            "{} - print",
            p.file_stem().and_then(|s| s.to_str()).unwrap_or("sheets")
        )
    };
    dir.join(format!("{name}.pdf"))
        .to_string_lossy()
        .into_owned()
}

/// A labelled row of the Print window: a narrow label, then its
/// controls, wrapping onto a second line rather than cut short.
fn row<R>(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.horizontal_top(|ui| {
        let h = ui.spacing().interact_size.y;
        let muted = ui.visuals().widgets.noninteractive.fg_stroke.color;
        ui.allocate_ui_with_layout(
            egui::vec2(78.0, h),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.set_min_width(78.0);
                ui.label(egui::RichText::new(crate::i18n::tr(label)).color(muted));
            },
        );
        ui.vertical(|ui| add(ui)).inner
    })
    .inner
}

fn settings_ui(
    ui: &mut egui::Ui,
    x: &mut PrintState,
    all: &[(ElementId, String, DVec2)],
    has_this: bool,
    props: &mut bool,
) {
    row(ui, "Printer", |ui| {
        let shown = if x.printer == printing::PDF {
            crate::i18n::tr("PDF file")
        } else {
            x.printer.clone()
        };
        egui::ComboBox::from_id_salt("print-printer")
            .selected_text(shown)
            .width(ui.available_width().max(60.0))
            .show_ui(ui, |ui| {
                for p in x.printers.clone() {
                    let label = if p == printing::PDF {
                        crate::i18n::tr("PDF file")
                    } else {
                        p.clone()
                    };
                    if ui.selectable_value(&mut x.printer, p, label).changed() {
                        x.got.clear();
                    }
                }
            });
    });
    if x.printer != printing::PDF {
        row(ui, "", |ui| {
            *props = ui
                .button(crate::i18n::t("Printer properties…"))
                .on_hover_text(crate::i18n::t(
                    "The printer's own settings: tray, quality, colour, roll and cutter. The paper below goes over its paper unless it is As set in the printer.",
                ))
                .clicked();
        });
    }
    row(ui, "Sheets", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(
                &mut x.scope,
                ExportScope::All,
                crate::i18n::tw(format!("All ({})", all.len())),
            );
            ui.selectable_value(
                &mut x.scope,
                ExportScope::Chosen,
                crate::i18n::tw(format!("Chosen ({})", x.chosen.len())),
            );
            ui.add_enabled_ui(has_this, |ui| {
                ui.selectable_value(&mut x.scope, ExportScope::This, crate::i18n::tw("This one"))
                    .on_disabled_hover_text(crate::i18n::t("Open a sheet first"));
            });
        })
    });
    if x.scope == ExportScope::Chosen {
        ui.horizontal(|ui| {
            if ui.small_button(crate::i18n::t("All")).clicked() {
                x.chosen = all.iter().map(|(id, _, _)| *id).collect();
            }
            if ui.small_button(crate::i18n::t("None")).clicked() {
                x.chosen.clear();
            }
        });
        egui::ScrollArea::vertical()
            .id_salt("print-sheets")
            .max_height(170.0)
            .show(ui, |ui| {
                for (id, name, size) in all {
                    let mut on = x.chosen.contains(id);
                    let label = format!("{name}  ({:.0} × {:.0})", size.x, size.y);
                    if crate::props::check(ui, "", &mut on, label.as_str()).changed() {
                        if on {
                            x.chosen.insert(*id);
                        } else {
                            x.chosen.remove(id);
                        }
                    }
                }
            });
    }
    row(ui, "Paper", |ui| {
        egui::ComboBox::from_id_salt("print-paper")
            .selected_text(crate::i18n::tr(&x.paper.label()))
            .width(ui.available_width().max(60.0))
            .show_ui(ui, |ui| {
                for c in PaperChoice::all() {
                    let label = crate::i18n::tr(&c.label());
                    ui.selectable_value(&mut x.paper, c, label);
                }
            })
            .response
            .on_hover_text(crate::i18n::t(
                "As the sheet: each sheet on paper of its own size (A4 to A0, 3×A4 and other long sheets as a custom size on a plotter). A4 to A0: every sheet on that paper, turned the sheet's way. Roll: a plotter's roll so wide, cut as long as the sheet.",
            ));
    });
    row(ui, "Scale", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut x.fit, Fit::Actual, crate::i18n::tw("1:1 (true size)"))
                .on_hover_text(crate::i18n::t(
                    "The drawings keep their scales: a 1:100 plan measures 1:100 on paper. What does not fit the paper is cut off.",
                ));
            ui.selectable_value(&mut x.fit, Fit::Shrink, crate::i18n::tw("Fit to paper"))
                .on_hover_text(crate::i18n::t(
                    "Smaller to fit the paper's printable area (never larger); the scales are then no longer true.",
                ));
        })
    });
    row(ui, "Colour", |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.selectable_value(&mut x.ink, Ink::Colour, crate::i18n::tw("As on the sheets"))
                .on_hover_text(crate::i18n::t(
                    "Each drawing as set on its sheet: in colour, in shades of grey or black and white (the drawing's Properties on the sheet)",
                ));
            ui.selectable_value(&mut x.ink, Ink::Grey, crate::i18n::tw("Shades of grey"))
                .on_hover_text(crate::i18n::t(
                    "Every drawing in shades of grey: each colour by its lightness",
                ));
            ui.selectable_value(&mut x.ink, Ink::Black, crate::i18n::tw("Black and white"))
                .on_hover_text(crate::i18n::t(
                    "Every line and text black, colour fills grey by their lightness",
                ));
        })
    });
    row(ui, "Copies", |ui| {
        ui.add(egui::DragValue::new(&mut x.copies).range(1..=99));
    });
}

fn preview_ui(
    ui: &mut egui::Ui,
    x: &mut PrintState,
    pages: usize,
    preview: Option<&(ElementId, String, Page, PaperGot, Vec<PlacedView>)>,
    model: &creabim_core::Model,
    info: &SheetInfo,
) {
    ui.horizontal(|ui| {
        ui.strong(crate::i18n::t("Preview"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add_enabled(x.page + 1 < pages, egui::Button::new("▶").small())
                .clicked()
            {
                x.page += 1;
            }
            ui.label(format!("{} / {pages}", (x.page + 1).min(pages)));
            if ui
                .add_enabled(x.page > 0, egui::Button::new("◀").small())
                .clicked()
            {
                x.page -= 1;
            }
        });
    });
    let (rect, _) = ui.allocate_exact_size(egui::vec2(380.0, 300.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, ui.visuals().extreme_bg_color);
    let Some((id, name, page, got, views)) = preview else {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            crate::i18n::t("No sheets to print"),
            egui::FontId::proportional(13.0),
            ui.visuals().weak_text_color(),
        );
        return;
    };
    let place = placement(page, got, x.fit);
    // the paper as large as fits, with room round it
    let zoom = ((rect.width() - 24.0) / got.size.x as f32)
        .min((rect.height() - 24.0) / got.size.y as f32)
        .max(0.01);
    let half = egui::vec2(got.size.x as f32, got.size.y as f32) * zoom / 2.0;
    let paper = egui::Rect::from_center_size(rect.center(), half * 2.0);
    painter.rect_filled(
        paper.translate(egui::vec2(2.0, 3.0)),
        0.0,
        Color32::from_black_alpha(60),
    );
    painter.rect_filled(paper, 0.0, Color32::WHITE);
    let clip = painter.with_clip_rect(paper);
    crate::sheetview::paint_paper(&clip, paper, got.size / 2.0, zoom, |c| {
        draw_on_paper(c, model, *id, views, info, place, x.ink);
    });
    // where the printer cannot reach
    let at = |p: DVec2| {
        egui::pos2(
            paper.left() + p.x as f32 * zoom,
            paper.bottom() - p.y as f32 * zoom,
        )
    };
    let reach = egui::Rect::from_two_pos(at(got.printable_lo), at(got.printable_hi));
    if reach.width() < paper.width() - 0.5 || reach.height() < paper.height() - 0.5 {
        let c = egui::Stroke::new(1.0, Color32::from_rgb(90, 140, 220));
        let corners = [
            reach.left_top(),
            reach.right_top(),
            reach.right_bottom(),
            reach.left_bottom(),
            reach.left_top(),
        ];
        clip.extend(egui::Shape::dashed_line(&corners, c, 4.0, 3.0));
    }
    painter.rect_stroke(
        paper,
        0.0,
        egui::Stroke::new(1.0, Color32::from_gray(120)),
        egui::StrokeKind::Outside,
    );
    ui.label(egui::RichText::new(name).strong());
    let scale = if (place.scale - 1.0).abs() < 1e-6 {
        crate::i18n::tr("1:1")
    } else {
        crate::i18n::tr(&format!("shrunk to {:.0} %", place.scale * 100.0))
    };
    let size = format!("{:.0} × {:.0}", page.sheet.x, page.sheet.y);
    let on = paper_name(got.size);
    ui.label(crate::i18n::tr(&format!(
        "Sheet {size} mm on paper {on}, {scale}"
    )));
    if place.cut > 0.0 {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            crate::i18n::tr(&format!(
                "It does not fit: up to {:.0} mm of the drawing is cut off. Choose a larger paper or Fit to paper.",
                place.cut.ceil()
            )),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_opens_for_the_sheet_in_front_and_prints_it_to_pdf() {
        let mut app = crate::app::test_app();
        let all: Vec<ElementId> = sheets::sheets(app.doc.model())
            .iter()
            .map(|(id, _)| *id)
            .collect();
        let sheet = all[0];
        app.active_view = Some(sheet);
        let ctx = egui::Context::default();
        app.run_action(&ctx, "file.print");
        assert!(app.print.open, "File › Print sheets opens the window");
        assert_eq!(app.print.scope, ExportScope::This);
        assert_eq!(app.sheets_to_print(), vec![sheet]);
        assert!(app.print.printers.iter().any(|p| p == printing::PDF));
        // the window and its preview draw
        app.print.printer = printing::PDF.into();
        app.print.paper = PaperChoice::Iso(1);
        app.print.fit = Fit::Shrink;
        let raw = egui::RawInput {
            max_texture_side: Some(16384),
            ..Default::default()
        };
        let mut out = ctx.run_ui(raw, |ui| app.print_window(ui.ctx()));
        out.textures_delta.clear();
        assert!(app.print.open);
        // printed to a PDF file on A3
        let dir = std::env::temp_dir().join(format!("creabim-print-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        app.path = dir.join("house.creabim").to_string_lossy().into_owned();
        app.print_now();
        assert!(app.status.starts_with("Printed 1 pages"), "{}", app.status);
        let pdf = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .find(|e| e.path().extension().is_some_and(|x| x == "pdf"))
            .expect("a PDF written");
        let bytes = std::fs::read(pdf.path()).unwrap();
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            text.contains("/MediaBox [0 0 1190.55 841.89]"),
            "A3 landscape"
        );
        let _ = std::fs::remove_dir_all(&dir);
        // all sheets, two copies
        app.print.scope = ExportScope::All;
        assert_eq!(app.sheets_to_print().len(), all.len());
    }

    #[test]
    fn a_cover_sheet_prints_on_a4_whole() {
        // title pages are A4 sheets with four labels: as the sheet, on
        // A4, 1:1, nothing cut
        let a4 = DVec2::new(210.0, 297.0);
        let ask = paper_for(a4, PaperChoice::Sheet);
        assert_eq!(ask.name, "A4");
        let got = PaperGot {
            size: a4,
            printable_lo: DVec2::splat(4.2),
            printable_hi: a4 - DVec2::splat(4.2),
        };
        let p = place(a4, &got, Fit::Actual, false);
        assert_eq!((p.scale, p.cut), (1.0, 0.0));
    }
}
