//! Result tables laid out as a spreadsheet: every cell framed in a grid,
//! a header row with the column's name and its unit over it that stays in
//! place while the rows scroll, numbers aligned to the right, and a
//! totals row in bold at the bottom. Used by every balance (heat loss,
//! cooling load, air, heating, water …).

use eframe::egui;
use egui::{Align, Color32, Layout, Margin, RichText, Stroke, StrokeKind, Ui, WidgetText};
use egui_extras::{Column, TableBuilder, TableRow};

/// A column: its name, its unit (shown under the name), and whether it
/// holds numbers (right-aligned).
#[derive(Clone, Copy)]
pub struct Col<'a> {
    pub title: &'a str,
    pub unit: &'a str,
    pub num: bool,
}

impl<'a> Col<'a> {
    /// A column of words.
    pub const fn text(title: &'a str) -> Self {
        Col {
            title,
            unit: "",
            num: false,
        }
    }

    /// A column of numbers in `unit`.
    pub const fn num(title: &'a str, unit: &'a str) -> Self {
        Col {
            title,
            unit,
            num: true,
        }
    }
}

/// The lines of the grid.
fn grid_stroke(ui: &Ui) -> Stroke {
    let c = ui.visuals().widgets.noninteractive.bg_stroke.color;
    Stroke::new(1.0, c)
}

/// The header's and the totals' ground.
fn head_fill(ui: &Ui) -> Color32 {
    ui.visuals().widgets.inactive.weak_bg_fill
}

/// One row being filled, cell after cell from the left.
pub struct Row<'a, 'b> {
    row: TableRow<'a, 'b>,
    cols: &'b [Col<'b>],
    next: usize,
    total: bool,
}

impl Row<'_, '_> {
    /// The next cell, its contents by `add`.
    pub fn cell(&mut self, add: impl FnOnce(&mut Ui)) {
        let num = self.cols.get(self.next).is_some_and(|c| c.num);
        let total = self.total;
        self.next += 1;
        self.row.col(|ui| {
            let rect = ui.max_rect();
            if total {
                ui.painter().rect_filled(rect, 0.0, head_fill(ui));
            }
            ui.painter()
                .rect_stroke(rect, 0.0, grid_stroke(ui), StrokeKind::Middle);
            // measured first at its own width, then the numbers moved right
            let layout = if num && !ui.is_sizing_pass() {
                Layout::right_to_left(Align::Center)
            } else {
                Layout::left_to_right(Align::Center)
            };
            egui::Frame::new()
                .inner_margin(Margin::symmetric(5, 0))
                .show(ui, |ui| {
                    ui.with_layout(layout, |ui| {
                        if total {
                            ui.style_mut().override_text_style = None;
                            ui.visuals_mut().override_text_color =
                                Some(ui.visuals().strong_text_color());
                        }
                        add(ui)
                    });
                });
        });
    }

    /// The next cell with text.
    pub fn text(&mut self, text: impl Into<WidgetText>) {
        let text = text.into();
        self.cell(|ui| {
            ui.label(text);
        });
    }

    /// The next cell with bold text.
    pub fn strong(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.cell(|ui| {
            ui.label(RichText::new(text).strong());
        });
    }

    /// The next cell with faint text.
    pub fn weak(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.cell(|ui| {
            ui.weak(text);
        });
    }

    /// An empty cell.
    pub fn empty(&mut self) {
        self.cell(|_| {});
    }

    /// The cells left empty to the end of the row, so its grid is whole.
    fn finish(&mut self) {
        while self.next < self.cols.len() {
            self.empty();
        }
    }
}

/// Shows a table `id` of `cols` with `rows` rows (filled by `add_row`
/// for each index) and, when `total` is given, a totals row under them.
/// The rows scroll under the header when they are taller than
/// `max_height`.
pub fn show(
    ui: &mut Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    cols: &[Col],
    rows: usize,
    max_height: f32,
    mut add_row: impl FnMut(&mut Row, usize),
    total: Option<&dyn Fn(&mut Row)>,
) {
    let t = crate::i18n::tr;
    let row_h = ui.spacing().interact_size.y + 2.0;
    let head_h = 2.0 * ui.text_style_height(&egui::TextStyle::Body) + 4.0;
    ui.push_id(id, |ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        let in_sight = max_height
            .min(ui.available_height() - head_h - row_h)
            .max(120.0);
        let mut b = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .vscroll(true)
            // no taller than what is left in sight, so the totals are
            // reached by the table's own scrolling under its header
            .max_scroll_height(in_sight)
            .auto_shrink([true, true]);
        // a headless screenshot of the last rows (CREABIM_SHOT_SHEET_END)
        if std::env::var_os("CREABIM_SHOT_SHEET_END").is_some() {
            let all = (rows + 1) as f32 * row_h;
            b = b.vertical_scroll_offset((all - in_sight).max(0.0));
        }
        for (i, _) in cols.iter().enumerate() {
            let c = Column::auto().at_least(if i == 0 { 120.0 } else { 56.0 });
            b = b.column(c);
        }
        b.header(head_h, |mut h| {
            for c in cols {
                h.col(|ui| {
                    let rect = ui.max_rect();
                    ui.painter().rect_filled(rect, 0.0, head_fill(ui));
                    ui.painter()
                        .rect_stroke(rect, 0.0, grid_stroke(ui), StrokeKind::Middle);
                    egui::Frame::new()
                        .inner_margin(Margin::symmetric(5, 0))
                        .show(ui, |ui| {
                            let add = |ui: &mut Ui| {
                                ui.add(
                                    egui::Label::new(RichText::new(t(c.title)).strong()).extend(),
                                );
                                ui.add(egui::Label::new(RichText::new(t(c.unit)).weak()).extend());
                            };
                            if ui.is_sizing_pass() {
                                ui.vertical(add);
                            } else {
                                ui.vertical_centered(add);
                            }
                        });
                });
            }
        })
        .body(|body| {
            let n = rows + usize::from(total.is_some());
            body.rows(row_h, n, |row| {
                let i = row.index();
                let mut r = Row {
                    row,
                    cols,
                    next: 0,
                    total: i >= rows,
                };
                match total {
                    Some(f) if i >= rows => f(&mut r),
                    _ => add_row(&mut r, i),
                }
                r.finish();
            });
        });
    });
}
