//! A HERZ differential pressure regulator's design
//! ([`creabim_services::herzdp`]): a window where the guarded circuit's
//! flow and loss, the ends, the regulator and the pump are put in, and
//! the setting, the partner valve and the check of the pump come out.

use super::CreaBimApp;
use creabim_services::herzdp::{self, DpInput};
use eframe::egui::{self, Vec2};

impl CreaBimApp {
    /// `calc:herzdp` from the Heating tab: open the window.
    pub(super) fn herzdp_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" || arg != "herzdp" {
            return false;
        }
        if self.herzdp.is_none() {
            self.herzdp = Some(DpInput::default());
        }
        true
    }

    /// The window, while it is open.
    pub(super) fn herzdp_window(&mut self, ctx: &egui::Context) {
        let Some(mut i) = self.herzdp.clone() else {
            return;
        };
        let t = crate::i18n::tr;
        let num = crate::i18n::num;
        let mut open = true;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("HERZ differential pressure regulator"))
            .id(egui::Id::new("herz.dp"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(240.0, 260.0))
            .default_size(Vec2::new(480.0, 520.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                let auto = t("propose");
                egui::Grid::new("herzdp.in").num_columns(2).show(ui, |ui| {
                    ui.label(t("Flow of the guarded circuit"));
                    ui.add(
                        egui::DragValue::new(&mut i.flow_lh)
                            .range(1.0..=500_000.0)
                            .speed(10.0)
                            .suffix(" l/h"),
                    );
                    ui.end_row();
                    ui.label(t("Pressure loss of the guarded circuit"));
                    ui.add(
                        egui::DragValue::new(&mut i.circuit_kpa)
                            .range(0.1..=200.0)
                            .speed(0.1)
                            .suffix(" kPa"),
                    );
                    ui.end_row();
                    ui.label(t("Connection"));
                    ui.horizontal(|ui| {
                        let before = i.flanged;
                        ui.selectable_value(&mut i.flanged, false, t("thread"));
                        ui.selectable_value(&mut i.flanged, true, t("flange"));
                        if before != i.flanged {
                            i.kind = None;
                            i.dn = None;
                            i.partner = None;
                            i.partner_dn = None;
                        }
                    });
                    ui.end_row();
                    ui.label(t("Regulator"));
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("herzdp.kind")
                            .width(150.0)
                            .selected_text(i.kind.clone().unwrap_or(auto.clone()))
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(i.kind.is_none(), &auto).clicked() {
                                    i.kind = None;
                                    i.dn = None;
                                }
                                for k in herzdp::kinds(i.flanged) {
                                    if ui
                                        .selectable_label(i.kind.as_deref() == Some(k), k)
                                        .clicked()
                                    {
                                        i.kind = Some(k.to_string());
                                        i.dn = None;
                                    }
                                }
                            });
                        let dns = i.kind.as_deref().map(herzdp::dns).unwrap_or_default();
                        egui::ComboBox::from_id_salt("herzdp.dn")
                            .width(80.0)
                            .selected_text(i.dn.clone().unwrap_or(auto.clone()))
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(i.dn.is_none(), &auto).clicked() {
                                    i.dn = None;
                                }
                                for d in dns {
                                    if ui.selectable_label(i.dn.as_deref() == Some(d), d).clicked() {
                                        i.dn = Some(d.to_string());
                                    }
                                }
                            });
                    });
                    ui.end_row();
                    ui.label(t("Partner valve"));
                    egui::ComboBox::from_id_salt("herzdp.partner")
                        .width(150.0)
                        .selected_text(i.partner.clone().unwrap_or(auto.clone()))
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(i.partner.is_none(), &auto).clicked() {
                                i.partner = None;
                            }
                            for p in herzdp::partners(i.flanged) {
                                if ui
                                    .selectable_label(i.partner.as_deref() == Some(p), p)
                                    .clicked()
                                {
                                    i.partner = Some(p.to_string());
                                }
                            }
                        });
                    ui.end_row();
                    ui.label(t("System loss up to the regulator"));
                    ui.add(
                        egui::DragValue::new(&mut i.system_kpa)
                            .range(0.0..=500.0)
                            .speed(0.1)
                            .suffix(" kPa"),
                    );
                    ui.end_row();
                    ui.label(t("Pump head"));
                    ui.add(
                        egui::DragValue::new(&mut i.pump_kpa)
                            .range(0.0..=1000.0)
                            .speed(0.5)
                            .suffix(" kPa"),
                    );
                    ui.end_row();
                });
                ui.separator();
                let r = herzdp::design(&i);
                egui::Grid::new("herzdp.out").num_columns(2).show(ui, |ui| {
                    let mut row = |a: String, b: String| {
                        ui.label(a);
                        ui.label(b);
                        ui.end_row();
                    };
                    if !r.kind.is_empty() {
                        row(t("Regulator"), format!("HERZ {} {}", r.kind, r.dn));
                        row(t("Order number"), r.order.clone());
                        row(
                            t("Setting"),
                            r.setting.map_or("-".into(), |s| num(s, 1)),
                        );
                        row(t("Loss of the regulator body"), format!("{} kPa", num(r.body_kpa, 1)));
                        if r.residual_kpa > 0.0 {
                            row(t("Residual pressure"), format!("{} kPa", num(r.residual_kpa, 1)));
                        }
                    }
                    if let Some(p) = &r.partner {
                        row(
                            t("Partner valve"),
                            format!("HERZ {} {}, {} {}", p.valve, p.dn, t("setting"), num(p.setting, 1)),
                        );
                        row(t("Order number"), p.order.clone());
                        row(t("Loss of the partner valve"), format!("{} kPa", num(p.kpa, 1)));
                    }
                    if !r.kind.is_empty() {
                        row(t("Membrane throttles"), format!("{} kPa", num(r.membrane_kpa, 1)));
                        row(t("Least pump head"), format!("{} kPa", num(r.pump_min_kpa, 1)));
                    }
                    if let Some(d) = &r.dn_estimate {
                        row(t("DN of the system for the flow"), d.clone());
                    }
                });
                ui.add_space(4.0);
                if r.ok() && r.notes.is_empty() {
                    ui.colored_label(egui::Color32::from_rgb(40, 140, 60), t("Sound design"));
                }
                for n in &r.notes {
                    ui.colored_label(egui::Color32::from_rgb(190, 90, 30), n);
                }
                if !r.suitable.is_empty() {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(t("Regulators that would do")).strong());
                    let list: Vec<String> = r
                        .suitable
                        .iter()
                        .map(|(k, d)| format!("{k} {d}"))
                        .collect();
                    ui.label(list.join(", "));
                }
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(t(
                        "After HERZ's design tool for differential pressure regulators (version 1.0, IX. 2025). Check the results before use.",
                    ))
                    .weak(),
                );
            });
        self.herzdp = open.then_some(i);
    }
}
