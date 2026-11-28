//! The schedule of earthworks (PilonDesigner's výkaz zemných prác), in a
//! window: each item with its KROS/Cenkros code, quantity and how it was
//! worked out, each pit and platform, and a copy for a spreadsheet or an
//! estimate (semicolons, as Cenkros reads them).

use super::CreaBimApp;
use creabim_core::earthworks::{schedule, Row, Taken};
use eframe::egui::{self, Vec2};

/// The schedule as text for a spreadsheet: code; item; unit; quantity;
/// how.
pub fn csv(rows: &[Row]) -> String {
    let t = crate::i18n::tr;
    let mut s = format!(
        "{};{};{};{};{}\n",
        t("Code"),
        t("Item"),
        t("Unit"),
        t("Quantity"),
        t("Worked out as")
    );
    for r in rows {
        s.push_str(&format!(
            "{};{};{};{};{}\n",
            r.code,
            t(r.text),
            r.unit,
            crate::i18n::num(r.qty, 3),
            t(r.calc)
        ));
    }
    s
}

impl CreaBimApp {
    /// `earthworks:schedule` from the Cadastre tab.
    pub(super) fn earthworks_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "earthworks" || arg != "schedule" {
            return false;
        }
        self.earthworks = Some(schedule(self.doc.model()));
        true
    }

    /// The schedule's window, while it is open.
    pub(super) fn earthworks_window(&mut self, ctx: &egui::Context) {
        let Some((rows, taken)) = &self.earthworks else {
            return;
        };
        let t = crate::i18n::tr;
        let model = self.doc.model();
        let mut open = true;
        let mut again = false;
        let mut select = None;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Earthworks"))
            .id(egui::Id::new("earthworks.schedule"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(320.0, 240.0))
            .default_size(crate::app::shell::at_least(Vec2::new(760.0, 480.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button(t("Work out again")).clicked() {
                        again = true;
                    }
                    if ui
                        .add_enabled(!rows.is_empty(), egui::Button::new(t("Copy for a spreadsheet")))
                        .on_hover_text(t("Code; item; unit; quantity, separated by semicolons, for Excel or Cenkros"))
                        .clicked()
                    {
                        ui.ctx().copy_text(csv(rows));
                    }
                });
                if rows.is_empty() {
                    ui.label(t(
                        "Nothing to take off: draw a terrain, then a building pit or a platform with the Terrain tool, or footings.",
                    ));
                }
                egui::ScrollArea::both().show(ui, |ui| {
                    use crate::sheet::{self, Col};
                    let cols = [
                        Col::text("Code"),
                        Col::text("Item"),
                        Col::text("Unit"),
                        Col::num("Quantity", ""),
                    ];
                    sheet::show(ui, "earthworks.rows", &cols, rows.len(), 400.0, |row, i| {
                        let r = &rows[i];
                        row.text(r.code);
                        row.cell(|ui| {
                            ui.label(t(r.text)).on_hover_text(t(r.calc));
                        });
                        row.text(r.unit);
                        row.text(crate::i18n::num(r.qty, 3));
                    }, None);
                    if !taken.is_empty() {
                        crate::props::section(ui, &t("Pits and platforms"));
                        let cols = [
                            Col::text("Name"),
                            Col::num("Area", "m²"),
                            Col::num("Cut", "m³"),
                            Col::num("Fill", "m³"),
                            Col::num("Back-fill", "m³"),
                        ];
                        let total = |row: &mut sheet::Row| {
                            row.text(t("Total"));
                            row.text(crate::i18n::num(taken.iter().map(|w| w.area).sum::<f64>(), 1));
                            row.text(crate::i18n::num(taken.iter().map(|w| w.cut).sum::<f64>(), 1));
                            row.text(crate::i18n::num(taken.iter().map(|w| w.fill).sum::<f64>(), 1));
                            row.text(crate::i18n::num(taken.iter().map(|w| w.backfill).sum::<f64>(), 1));
                        };
                        sheet::show(ui, "earthworks.taken", &cols, taken.len(), 300.0, |row, i| {
                            let Taken { id, area, cut, fill, backfill, .. } = &taken[i];
                            let name = model.get(*id).map_or(String::new(), |e| e.name.clone());
                            row.cell(|ui| {
                                if ui.link(name).clicked() {
                                    select = Some(*id);
                                }
                            });
                            row.text(crate::i18n::num(*area, 1));
                            row.text(crate::i18n::num(*cut, 1));
                            row.text(crate::i18n::num(*fill, 1));
                            row.text(crate::i18n::num(*backfill, 1));
                        }, Some(&total));
                    }
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(t(
                            "The codes are a guide: pick the estimate's item by the soil class, the width dug and the haul. Volumes are the ground before less after, in cells of 0.5 m.",
                        ))
                        .weak(),
                    );
                });
            });
        if let Some(id) = select {
            self.selection.only(id);
        }
        if again {
            self.earthworks = Some(schedule(self.doc.model()));
        }
        if !open {
            self.earthworks = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_schedule_copies_with_semicolons() {
        let rows = vec![Row {
            key: "pit",
            code: "131201101",
            text: "Excavating an unsupported pit in soil class 3 (slopes as designed)",
            unit: "m3",
            qty: 12.5,
            calc: "x",
            ids: vec![],
        }];
        let s = csv(&rows);
        assert_eq!(s.lines().count(), 2);
        assert!(s.lines().nth(1).unwrap().starts_with("131201101;"));
    }
}
