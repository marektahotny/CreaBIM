//! Duct sizing ([`creabim_services::ductsizing`]): a window that lists
//! the ventilation ducts drawn with the air each carries, the velocity
//! in it as drawn and the diameter proposed for the velocity allowed,
//! and sets the proposed diameters on the round ducts in one step.

use super::CreaBimApp;
use creabim_core::{ElementId, ElementKind, Transaction};
use creabim_services::ductsizing::{self};
use eframe::egui::{self, Vec2};

/// The air systems it sizes.
const SYSTEMS: [(&str, &str); 2] = [("VZP", "Supply air"), ("VZO", "Exhaust air")];

impl CreaBimApp {
    /// `calc:ducts` from the Ventilation tab: open the window.
    pub(super) fn ducts_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" || arg != "ducts" {
            return false;
        }
        if self.ducts.is_none() {
            // reopened: as it was last written (George 2026-11-27)
            self.ducts = Some(self.saved_settings("ducts").unwrap_or_default());
        }
        true
    }

    /// The window, while it is open.
    pub(super) fn ducts_window(&mut self, ctx: &egui::Context) {
        let Some(mut limits) = self.ducts else {
            return;
        };
        let t = crate::i18n::tr;
        let num = crate::i18n::num;
        let o = self.heat_opts();
        // the results are gathered while the window is drawn: the button
        // goes by what the last frame found
        let written = self.ducts_written;
        let start = limits;
        let mut write_now = false;
        let mut found = creabim_core::results::ResultSet::new(
            "ducts",
            "Dimenzovanie VZT potrubia",
            "STN EN 1505",
        );
        let mut open = true;
        let mut apply: Option<Vec<(ElementId, f64)>> = None;
        let mut select = None;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Duct sizing"))
            .id(egui::Id::new("ducts.sizing"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(330.0, 250.0))
            .default_size(crate::app::shell::at_least(Vec2::new(660.0, 500.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                if super::calcresultsui::write_button(ui, written) {
                    write_now = true;
                }
                ui.horizontal(|ui| {
                    ui.label(t("Velocity allowed"));
                    ui.label(t("main run"));
                    ui.add(
                        egui::DragValue::new(&mut limits.main)
                            .range(1.0..=12.0)
                            .speed(0.1)
                            .suffix(" m/s"),
                    );
                    ui.label(t("branches to terminals"));
                    ui.add(
                        egui::DragValue::new(&mut limits.branch)
                            .range(0.5..=8.0)
                            .speed(0.1)
                            .suffix(" m/s"),
                    );
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (code, name) in SYSTEMS {
                        ui.label(egui::RichText::new(format!("{} ({code})", t(name))).strong());
                        let Some(rows) = ductsizing::size(self.doc.model(), code, &o, limits)
                        else {
                            ui.weak(t("no ducts drawn"));
                            ui.add_space(6.0);
                            continue;
                        };
                        creabim_services::calcresults::ducts(&mut found, code, &rows);
                        let change: Vec<(ElementId, f64)> = rows
                            .iter()
                            .filter_map(|r| {
                                let d = r.proposed_mm?;
                                ((d - r.drawn_mm).abs() > 0.5 && r.flow_m3h > 0.0)
                                    .then_some((r.id, d))
                            })
                            .collect();
                        use crate::sheet::{self, Col};
                        let cols = [
                            Col::text("Duct"),
                            Col::num("V", "m³/h"),
                            Col::num("L", "m"),
                            Col::text("Drawn"),
                            Col::num("v / limit", "m/s"),
                            Col::text("Proposed"),
                            Col::num("R", "Pa/m"),
                        ];
                        let total = |row: &mut sheet::Row| {
                            row.text(t("Total"));
                            row.empty();
                            row.text(num(rows.iter().map(|r| r.length).sum::<f64>(), 1));
                        };
                        sheet::show(ui, ("ducts.grid", code), &cols, rows.len(), 400.0, |row, i| {
                            let r = &rows[i];
                            row.cell(|ui| {
                                if ui
                                    .link(format!(
                                        "{} {}",
                                        i + 1,
                                        t(if r.branch { "branch" } else { "main run" })
                                    ))
                                    .clicked()
                                {
                                    select = Some(r.id);
                                }
                            });
                            row.text(num(r.flow_m3h, 0));
                            row.text(num(r.length, 1));
                            row.text(match r.rect {
                                Some((w, h)) => format!("{w:.0}×{h:.0}"),
                                None => format!("Ø{:.0}", r.drawn_mm),
                            });
                            let v = format!("{} / {}", num(r.v_drawn, 1), num(r.limit, 1));
                            if r.ok() {
                                row.text(v);
                            } else {
                                row.cell(|ui| {
                                    ui.colored_label(ui.visuals().warn_fg_color, v);
                                });
                            }
                            row.text(match r.proposed_mm {
                                Some(d) => format!("Ø{d:.0} · {} m/s", num(r.v, 1)),
                                None => "—".to_string(),
                            });
                            row.text(num(r.r_pa_m, 2));
                        }, Some(&total));
                        if rows.iter().all(|r| r.flow_m3h <= 0.0) {
                            ui.weak(t("No air on these ducts yet: put diffusers or grilles on their ends and place rooms."));
                        } else if change.is_empty() {
                            ui.weak(t("The round ducts already have the proposed diameters."));
                        } else if ui
                            .button(format!(
                                "{} ({})",
                                t("Set the proposed diameters"),
                                change.len()
                            ))
                            .clicked()
                        {
                            apply = Some(change);
                        }
                        ui.add_space(8.0);
                    }
                    ui.weak(t("Round ducts take the smallest diameter of the series (STN EN 1506) that keeps the air under the velocity allowed; rectangular ducts are checked as drawn. Friction for galvanised sheet, without fittings."));
                });
            });
        if let Some(id) = select {
            self.selection.only(id);
        }
        if let Some(change) = apply {
            let mut tx = Transaction::new("Size ducts");
            for (id, d) in &change {
                if let Some(before) = self.doc.model().get(*id).cloned() {
                    let mut after = before.clone();
                    if let ElementKind::Pipe(p) = &mut after.kind {
                        let wall = (p.outer_mm - p.inner_mm).max(0.0);
                        p.nominal_mm = *d;
                        p.outer_mm = *d + wall;
                        p.inner_mm = *d;
                    }
                    tx = tx.replace(before, after);
                }
            }
            match self.doc.commit(tx) {
                Ok(_) => self.status = format!("{}: {}", t("Ducts sized"), change.len()),
                Err(e) => self.status = format!("Couldn't do that: {e}"),
            }
        }
        let found = found.with_settings(&start);
        if write_now {
            self.write_results_said(found);
            self.ducts_written = true;
        } else {
            self.ducts_written = self.is_written(&found);
        }
        self.ducts = if open { Some(limits) } else { None };
    }
}
