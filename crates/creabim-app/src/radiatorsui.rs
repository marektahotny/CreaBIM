//! Radiators ([`creabim_services::radiators`], STN EN 442): a window
//! with every room's radiators sized from its heat loss, less what its
//! underfloor heating gives, and a button that takes a room's radiator
//! to place.

use super::{CreaBimApp, Tool};
use creabim_core::catalogue::radiators::{Panel, HEIGHTS};
use creabim_services::heatloss;
use creabim_services::radiators::{self, Settings};
use eframe::egui::{self, Vec2};

impl CreaBimApp {
    /// `calc:radiators` from the Heating tab: open the window.
    pub(super) fn radiators_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" || arg != "radiators" {
            return false;
        }
        if self.radiators.is_none() {
            self.radiators = Some(Settings::default());
        }
        self.results_due = Some("radiators");
        true
    }

    /// The window, while it is open.
    pub(super) fn radiators_window(&mut self, ctx: &egui::Context) {
        let Some(mut s) = self.radiators else {
            return;
        };
        let t = crate::i18n::tr;
        let num = crate::i18n::num;
        let o = self
            .heat_loss
            .as_ref()
            .map_or_else(heatloss::Opts::default, |x| x.0);
        let floor = self.floor_heat.unwrap_or_default();
        let d = radiators::design(self.doc.model(), &o, &s, &floor);
        let mut write_now = self.results_due.take_if(|c| *c == "radiators").is_some();
        let mut open = true;
        let mut select = None;
        let mut place: Option<Panel> = None;
        let mut resize = false;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Radiators"))
            .id(egui::Id::new("radiators"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(360.0, 240.0))
            .default_size(Vec2::new(720.0, 480.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if super::calcresultsui::write_button(ui) {
                        write_now = true;
                    }
                    if ui
                        .button(t("Size the standing radiators"))
                        .on_hover_text(t("Makes every panel radiator standing in a room the size the room needs: what the floor and the towel radiators leave, shared among them. A locked radiator keeps its size and its output is counted."))
                        .clicked()
                    {
                        resize = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(t("Supply temperature"));
                    ui.add(
                        egui::DragValue::new(&mut s.supply_c)
                            .range(35.0..=90.0)
                            .speed(0.5)
                            .suffix(" °C"),
                    );
                    ui.label(t("Return"));
                    ui.add(
                        egui::DragValue::new(&mut s.return_c)
                            .range(25.0..=80.0)
                            .speed(0.5)
                            .suffix(" °C"),
                    );
                    s.return_c = s.return_c.min(s.supply_c - 1.0);
                    ui.label(t("Height"));
                    egui::ComboBox::from_id_salt("radiators.height")
                        .width(70.0)
                        .selected_text(format!("{} mm", s.height))
                        .show_ui(ui, |ui| {
                            for h in HEIGHTS {
                                ui.selectable_value(&mut s.height, h, format!("{h} mm"));
                            }
                        });
                    ui.label(t("Longest"));
                    let mut len = s.max_len as f64;
                    ui.add(
                        egui::DragValue::new(&mut len)
                            .range(400.0..=3000.0)
                            .speed(100.0)
                            .suffix(" mm"),
                    );
                    s.max_len = (len / 100.0).round() as u32 * 100;
                });
                if d.rooms.is_empty() {
                    ui.label(t("No rooms to calculate: place rooms first."));
                    return;
                }
                ui.label(
                    egui::RichText::new(format!(
                        "{} {} · {} kW · {} kg/h · {} {} kW",
                        d.count,
                        t("radiators"),
                        num(d.output_kw, 2),
                        num(d.flow, 0),
                        t("standing in the model"),
                        num(d.installed_kw, 2),
                    ))
                    .strong(),
                );
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("radiators.rooms")
                        .striped(true)
                        .num_columns(10)
                        .show(ui, |ui| {
                            for h in [
                                "Room", "θi °C", "Φ W", "Floor W", "Towel radiators W", "Needed W", "Proposal",
                                "Output W", "kg/h", "Standing W", "",
                            ] {
                                ui.strong(t(h));
                            }
                            ui.end_row();
                            for x in &d.rooms {
                                let name = format!("{} {}", x.number, x.name);
                                if ui.link(name.trim()).clicked() {
                                    select = Some(x.id);
                                }
                                ui.label(num(x.ti, 0));
                                ui.label(num(x.phi, 0));
                                ui.label(if x.floor_w > 0.0 {
                                    num(x.floor_w, 0)
                                } else {
                                    String::new()
                                });
                                ui.label(if x.towels_w > 0.0 {
                                    num(x.towels_w, 0)
                                } else {
                                    String::new()
                                });
                                ui.label(num(x.need, 0));
                                match x.panel {
                                    Some(p) => {
                                        let text = format!("{} × {}", x.count, p.label());
                                        if x.ok {
                                            ui.label(text);
                                        } else {
                                            ui.colored_label(ui.visuals().warn_fg_color, text)
                                                .on_hover_text(t(
                                                    "The radiators allowed do not cover the room's loss: raise the supply temperature, the height or the longest length.",
                                                ));
                                        }
                                    }
                                    None => {
                                        ui.weak(t("covered by the floor"));
                                    }
                                }
                                ui.label(num(x.output, 0));
                                ui.label(num(x.flow, 0));
                                if x.installed.is_empty() {
                                    ui.label("");
                                } else {
                                    let text = num(x.installed_w, 0);
                                    let list = x
                                        .installed
                                        .iter()
                                        .map(creabim_core::catalogue::radiators::Heater::label)
                                        .collect::<Vec<_>>()
                                        .join(", ");
                                    if x.installed_w >= x.need * 0.98 {
                                        ui.label(text).on_hover_text(list);
                                    } else {
                                        ui.colored_label(ui.visuals().warn_fg_color, text)
                                            .on_hover_text(list);
                                    }
                                }
                                match x.panel {
                                    Some(p) => {
                                        if ui.small_button(t("Place it")).clicked() {
                                            place = Some(p);
                                        }
                                    }
                                    None => {
                                        ui.label("");
                                    }
                                }
                                ui.end_row();
                            }
                        });
                    ui.add_space(6.0);
                    ui.weak(t("KORADO RADIK VK panel radiators, one under each window of the room, the slimmest type that covers its share; outputs and exponents from KORADO's tables. Towel radiators KORADO KORALUX LINEAR CLASSIC-M on the same water are counted first."));
                    ui.weak("STN EN 442-1 · STN EN 442-2 · STN EN 12831-1 · STN EN 12828");
                });
            });
        if resize {
            let tx = radiators::resize(self.doc.model(), &d, &s);
            if tx.ops.is_empty() {
                self.status = t("The standing radiators already have the size the rooms need.");
            } else {
                let r = self.doc.commit(tx);
                self.report(
                    r,
                    t("Radiators sized to the rooms; locked ones kept their size."),
                );
            }
        }
        if let Some(id) = select {
            self.selection.only(id);
        }
        if let Some(p) = place {
            let unit = p.object();
            self.status = format!(
                "{}: {}",
                unit.name,
                t("click in a plan or the 3D view to place it. Space turns it; Esc ends.")
            );
            self.plan.objects.current = Some(unit);
            self.plan.objects.offset = None;
            self.plan.objects.project_type = None;
            self.plan.cancel();
            self.plan.tool = Tool::Object;
        }
        if write_now {
            self.write_results_said(creabim_services::calcresults::radiators(&d));
        }
        self.radiators = if open { Some(s) } else { None };
    }
}
