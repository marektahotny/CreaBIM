//! The cooling load ([`creabim_services::coolingload`], after STN 73
//! 0548): a window with every room's load in summer, what it is made
//! of, and the fan-coil that covers it.

use super::CreaBimApp;
use creabim_services::coolingload::{self, Settings, Shading};
use creabim_services::heatloss;
use eframe::egui::{self, Vec2};

impl CreaBimApp {
    /// `calc:cooling` from the Ventilation tab: open the window.
    pub(super) fn cooling_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" || arg != "cooling" {
            return false;
        }
        if self.cooling.is_none() {
            self.cooling = Some(Settings::default());
        }
        true
    }

    /// The window, while it is open.
    pub(super) fn cooling_window(&mut self, ctx: &egui::Context) {
        let Some(mut s) = self.cooling else {
            return;
        };
        let t = crate::i18n::tr;
        let num = crate::i18n::num;
        let o = self
            .heat_loss
            .as_ref()
            .map_or_else(heatloss::Opts::default, |x| x.0);
        let b = coolingload::building(self.doc.model(), &o, &s);
        let mut open = true;
        let mut select = None;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Cooling load"))
            .id(egui::Id::new("cooling.load"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(340.0, 260.0))
            .default_size(Vec2::new(680.0, 520.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(t("Outside in summer"));
                    ui.add(
                        egui::DragValue::new(&mut s.te)
                            .range(25.0..=45.0)
                            .speed(0.5)
                            .suffix(" °C"),
                    );
                    ui.label(t("Inside"));
                    ui.add(
                        egui::DragValue::new(&mut s.ti)
                            .range(20.0..=28.0)
                            .speed(0.5)
                            .suffix(" °C"),
                    );
                    ui.label(t("Glazing g"));
                    ui.add(
                        egui::DragValue::new(&mut s.g)
                            .range(0.1..=0.87)
                            .speed(0.01),
                    );
                    ui.label(t("Sun shading"));
                    egui::ComboBox::from_id_salt("cooling.shading")
                        .selected_text(t(s.shading.name()))
                        .show_ui(ui, |ui| {
                            for x in Shading::ALL {
                                ui.selectable_value(&mut s.shading, x, t(x.name()));
                            }
                        });
                    ui.label(t("Heat storage"))
                        .on_hover_text(t("Storage factor of the sun's load: 1 takes nothing off; lower for a heavy building."));
                    ui.add(
                        egui::DragValue::new(&mut s.storage)
                            .range(0.5..=1.0)
                            .speed(0.01),
                    );
                });
                if b.rooms.is_empty() {
                    ui.label(t("No rooms to calculate: place rooms first."));
                    return;
                }
                ui.label(
                    egui::RichText::new(format!(
                        "{}: {} kW · {} m² · {} W/m² · {} {} m³/h ({}/{} °C)",
                        t("Cooled rooms"),
                        num(b.phi / 1000.0, 2),
                        num(b.area, 1),
                        num(if b.area > 0.0 { b.phi / b.area } else { 0.0 }, 0),
                        t("chilled water"),
                        num(b.flow_ls * 3.6, 2),
                        num(s.supply_c, 0),
                        num(s.return_c, 0),
                    ))
                    .strong(),
                );
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("cooling.rooms")
                        .striped(true)
                        .num_columns(6)
                        .show(ui, |ui| {
                            for h in ["Room", "A m²", "Φ W", "W/m²", "Worst hour", "Fan-coil kW"] {
                                ui.strong(t(h));
                            }
                            ui.end_row();
                            for r in &b.rooms {
                                let name = format!("{} {}", r.number, r.name);
                                if ui.link(name.trim()).clicked() {
                                    select = Some(r.id);
                                }
                                ui.label(num(r.area, 1));
                                if r.cooled {
                                    ui.label(egui::RichText::new(num(r.phi, 0)).strong());
                                } else {
                                    ui.weak(num(r.phi, 0))
                                        .on_hover_text(t("Not cooled: a bathroom, corridor or store is left out of the total."));
                                }
                                ui.label(num(if r.area > 0.0 { r.phi / r.area } else { 0.0 }, 0));
                                ui.label(r.hour.map_or(String::new(), |h| format!("{h}:00")));
                                if r.cooled {
                                    ui.label(num(r.fancoil_kw, 1));
                                } else {
                                    ui.label("");
                                }
                                ui.end_row();
                            }
                        });
                    ui.add_space(6.0);
                    for r in &b.rooms {
                        let title = format!("{} {} – {} W", r.number, r.name, num(r.phi, 0));
                        egui::CollapsingHeader::new(title.trim().to_string())
                            .id_salt(("cooling.room", r.id))
                            .show(ui, |ui| {
                                egui::Grid::new(("cooling.items", r.id))
                                    .striped(true)
                                    .num_columns(3)
                                    .show(ui, |ui| {
                                        for i in &r.items {
                                            ui.label(t(i.kind));
                                            ui.label(num(i.phi, 0));
                                            ui.weak(&i.note);
                                            ui.end_row();
                                        }
                                    });
                            });
                    }
                    ui.add_space(6.0);
                    ui.weak(t("A simplified load: the sun through the windows on 21 July at 50° north by where each faces, heat through walls, roof and windows with the U values of the heat loss, people, lighting, appliances and ventilation. Each room at its own worst hour. Fan-coils: sensible output at 7/12 °C, usual values to design with."));
                    ui.weak("STN 73 0548 · STN EN ISO 52016-1");
                });
            });
        if let Some(id) = select {
            self.selection.only(id);
        }
        self.cooling = if open { Some(s) } else { None };
    }
}
