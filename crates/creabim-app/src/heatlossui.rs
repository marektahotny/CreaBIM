//! The rooms' design heat loss ([`creabim_services::heatloss`],
//! simplified after STN EN 12831-1): a window with what the calculation
//! is given, each room's losses and what it lacks, the building's total,
//! and a step on to sizing the heat source by it.

use super::CreaBimApp;
use creabim_services::heatloss::{self, Opts, Shielding};
use eframe::egui::{self, Vec2};

impl CreaBimApp {
    /// `calc:heatloss` from the Heating tab: calculate and show the window.
    pub(super) fn heat_loss_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" || arg != "heatloss" {
            return false;
        }
        let o = self.heat_loss.as_ref().map_or_else(Opts::default, |x| x.0);
        let b = heatloss::building(self.doc.model(), &o);
        self.status = if b.rooms.is_empty() {
            crate::i18n::tr("No rooms to calculate: place rooms first.")
        } else {
            crate::i18n::tr("Heat loss calculated")
        };
        self.heat_loss = Some((o, b));
        true
    }

    /// The window, while it is open.
    pub(super) fn heat_loss_window(&mut self, ctx: &egui::Context) {
        let Some((mut o, b)) = self.heat_loss.clone() else {
            return;
        };
        let t = crate::i18n::tr;
        let num = crate::i18n::num;
        let mut open = true;
        let mut select = None;
        let mut to_source = false;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Heat loss of the rooms"))
            .id(egui::Id::new("heat.loss"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(330.0, 280.0))
            .default_size(Vec2::new(660.0, 560.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                egui::Grid::new("heat.loss.opts")
                    .num_columns(4)
                    .spacing([12.0, 5.0])
                    .show(ui, |ui| {
                        let val = |ui: &mut egui::Ui, text: &str, v: &mut f64, lo: f64, hi: f64, step: f64, unit: &str| {
                            ui.label(t(text));
                            ui.add(
                                egui::DragValue::new(v)
                                    .range(lo..=hi)
                                    .speed(step)
                                    .suffix(unit),
                            );
                        };
                        val(ui, "Design outside temperature", &mut o.te, -30.0, 5.0, 0.5, " °C");
                        val(ui, "Thermal bridges ΔU", &mut o.d_u, 0.0, 0.3, 0.01, " W/(m²K)");
                        ui.end_row();
                        let mut mean = o.te_mean.unwrap_or(0.0);
                        let mut has = o.te_mean.is_some();
                        ui.label(t("Mean annual temperature"));
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut has, "");
                            ui.add_enabled(
                                has,
                                egui::DragValue::new(&mut mean)
                                    .range(-5.0..=20.0)
                                    .speed(0.1)
                                    .suffix(" °C"),
                            );
                        });
                        o.te_mean = has.then_some(if o.te_mean.is_none() && has { 9.5 } else { mean });
                        val(ui, "Air changes n50", &mut o.n50, 0.3, 20.0, 0.1, " 1/h");
                        ui.end_row();
                        val(ui, "U of windows", &mut o.u_window, 0.3, 6.0, 0.05, " W/(m²K)");
                        val(ui, "U of outside doors", &mut o.u_door, 0.3, 6.0, 0.05, " W/(m²K)");
                        ui.end_row();
                    });
                egui::ComboBox::from_id_salt("heat.loss.shield")
                    .selected_text(t(o.shielding.name()))
                    .width(ui.available_width().min(420.0))
                    .show_ui(ui, |ui| {
                        for s in Shielding::ALL {
                            ui.selectable_value(&mut o.shielding, s, t(s.name()));
                        }
                    });
                ui.separator();
                if b.rooms.is_empty() {
                    ui.label(t("No rooms to calculate: place rooms first."));
                    return;
                }
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{}: {} kW · {} m² · {} W/m²",
                            t("The building"),
                            num(b.phi / 1000.0, 2),
                            num(b.area, 1),
                            num(if b.area > 0.0 { b.phi / b.area } else { 0.0 }, 0),
                        ))
                        .strong(),
                    );
                    if ui
                        .button(t("Size the heat source"))
                        .on_hover_text(t("Open the heat source window with this heat loss and outside temperature"))
                        .clicked()
                    {
                        to_source = true;
                    }
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("heat.loss.rooms")
                        .striped(true)
                        .num_columns(8)
                        .show(ui, |ui| {
                            for h in ["Room", "θi °C", "A m²", "V m³", "ΦT W", "ΦV W", "Φ W", "W/m²"] {
                                ui.strong(t(h));
                            }
                            ui.end_row();
                            for r in &b.rooms {
                                let name = format!("{} {}", r.number, r.name);
                                if ui.link(name.trim()).clicked() {
                                    select = Some(r.id);
                                }
                                ui.label(num(r.ti, 0));
                                ui.label(num(r.area, 1));
                                ui.label(num(r.volume, 1));
                                ui.label(num(r.phi_t, 0));
                                ui.label(num(r.phi_v, 0));
                                ui.label(egui::RichText::new(num(r.phi, 0)).strong());
                                ui.label(num(if r.area > 0.0 { r.phi / r.area } else { 0.0 }, 0));
                                ui.end_row();
                            }
                        });
                    ui.add_space(6.0);
                    for r in &b.rooms {
                        let title = format!("{} {} – {} W", r.number, r.name, num(r.phi, 0));
                        egui::CollapsingHeader::new(title.trim().to_string())
                            .id_salt(("heat.loss.room", r.id))
                            .show(ui, |ui| {
                                egui::Grid::new(("heat.loss.items", r.id))
                                    .striped(true)
                                    .num_columns(5)
                                    .show(ui, |ui| {
                                        for h in ["Through", "A m²", "U W/(m²K)", "f", "Φ W"] {
                                            ui.strong(t(h));
                                        }
                                        ui.end_row();
                                        for i in &r.items {
                                            let what = if i.detail.is_empty() {
                                                t(i.kind)
                                            } else {
                                                format!("{} – {}", t(i.kind), i.detail)
                                            };
                                            ui.label(what);
                                            ui.label(num(i.area, 2));
                                            ui.label(num(i.u, 3));
                                            ui.label(num(i.f, 3));
                                            ui.label(num(i.phi, 0));
                                            ui.end_row();
                                        }
                                    });
                                ui.label(format!(
                                    "{}: {} m³/h ({} {}, {} {}; {} {})",
                                    t("Ventilation"),
                                    num(r.v_min.max(r.v_inf), 1),
                                    t("least"),
                                    num(r.v_min, 1),
                                    t("infiltration"),
                                    num(r.v_inf, 1),
                                    t("façades with openings"),
                                    r.facades,
                                ));
                                for m in &r.missing {
                                    ui.colored_label(ui.visuals().warn_fg_color, t(m.text()));
                                }
                            });
                    }
                    ui.add_space(6.0);
                    ui.weak(t("Simplified after STN EN 12831-1: inside dimensions, outside walls where no room lies beyond, U from the build-ups' λ, floors and ceilings between storeys left out. A wall is counted for the room it bounds."));
                    ui.weak("STN EN 12831-1 · STN 73 0540-2 · STN EN ISO 6946");
                });
            });
        if let Some(id) = select {
            self.selection.only(id);
        }
        if to_source {
            let mut s = self.heat_source.clone().unwrap_or_default();
            s.input.heat_load = (b.phi / 1000.0).clamp(0.5, 200.0);
            s.input.te = o.te.clamp(-25.0, 0.0);
            self.heat_source = Some(s);
        }
        if !open {
            self.heat_loss = None;
        } else if self.heat_loss.as_ref().is_some_and(|x| x.0 != o) {
            let b = heatloss::building(self.doc.model(), &o);
            self.heat_loss = Some((o, b));
        }
    }
}
