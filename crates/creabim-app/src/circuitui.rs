//! A final circuit's check ([`creabim_services::electrical`], STN 33
//! 2000-5-52, STN 33 2000-4-43, STN 33 2000-4-41): a window where a
//! circuit's power, length, cable and breaker are put in and its design
//! current, voltage drop, protection and fault loop come out, with a
//! button that proposes the smallest breaker and cable that pass.

use super::CreaBimApp;
use creabim_services::electrical::{self, Circuit, Curve, Method, Supply, BREAKER_IN, IZ_S};
use eframe::egui::{self, Vec2};

impl CreaBimApp {
    /// `calc:circuit` from the Electrical tab: open the window.
    pub(super) fn circuit_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" || arg != "circuit" {
            return false;
        }
        if self.circuit.is_none() {
            self.circuit = Some(Circuit::default());
        }
        true
    }

    /// The window, while it is open.
    pub(super) fn circuit_window(&mut self, ctx: &egui::Context) {
        let Some(mut c) = self.circuit else {
            return;
        };
        let t = crate::i18n::tr;
        let num = crate::i18n::num;
        let mut open = true;
        let mut failed = false;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Circuit check"))
            .id(egui::Id::new("circuit.check"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(230.0, 240.0))
            .default_size(crate::app::shell::at_least(Vec2::new(460.0, 480.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                egui::Grid::new("circuit.in").num_columns(2).show(ui, |ui| {
                    ui.label(t("Installed power"));
                    ui.add(
                        egui::DragValue::new(&mut c.power_w)
                            .range(0.0..=200_000.0)
                            .speed(50.0)
                            .suffix(" W"),
                    );
                    ui.end_row();
                    ui.label("cos φ");
                    ui.add(
                        egui::DragValue::new(&mut c.cos_phi)
                            .range(0.3..=1.0)
                            .speed(0.01),
                    );
                    ui.end_row();
                    ui.label(t("Phases"));
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut c.three_phase, false, "1 × 230 V");
                        ui.selectable_value(&mut c.three_phase, true, "3 × 400 V");
                    });
                    ui.end_row();
                    ui.label(t("Cable length"));
                    ui.add(
                        egui::DragValue::new(&mut c.length_m)
                            .range(0.0..=1000.0)
                            .speed(0.5)
                            .suffix(" m"),
                    );
                    ui.end_row();
                    ui.label(t("Cross-section"));
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("circuit.s")
                            .width(80.0)
                            .selected_text(format!("{} mm²", num(c.s_mm2, 1)))
                            .show_ui(ui, |ui| {
                                for s in IZ_S {
                                    ui.selectable_value(
                                        &mut c.s_mm2,
                                        s,
                                        format!("{} mm²", num(s, 1)),
                                    );
                                }
                            });
                        ui.selectable_value(&mut c.aluminium, false, "Cu");
                        ui.selectable_value(&mut c.aluminium, true, "Al");
                    });
                    ui.end_row();
                    ui.label(t("Breaker"));
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("circuit.curve")
                            .width(40.0)
                            .selected_text(c.curve.letter())
                            .show_ui(ui, |ui| {
                                for x in Curve::ALL {
                                    ui.selectable_value(&mut c.curve, x, x.letter());
                                }
                            });
                        egui::ComboBox::from_id_salt("circuit.in_a")
                            .width(70.0)
                            .selected_text(format!("{} A", num(c.in_a, 0)))
                            .show_ui(ui, |ui| {
                                for x in BREAKER_IN {
                                    ui.selectable_value(&mut c.in_a, x, format!("{} A", num(x, 0)));
                                }
                            });
                    });
                    ui.end_row();
                    ui.label(t("Laid"));
                    egui::ComboBox::from_id_salt("circuit.method")
                        .width(260.0)
                        .selected_text(t(c.method.name()))
                        .show_ui(ui, |ui| {
                            for m in Method::ALL {
                                ui.selectable_value(&mut c.method, m, t(m.name()));
                            }
                        });
                    ui.end_row();
                    ui.label(t("Circuit"));
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut c.lighting, false, t("sockets and others"));
                        ui.selectable_value(&mut c.lighting, true, t("lighting"));
                    });
                    ui.end_row();
                    ui.label(t("Fed from"));
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut c.supply, Supply::Public, t("the public network"));
                        ui.selectable_value(&mut c.supply, Supply::Own, t("its own source"));
                    });
                    ui.end_row();
                    ui.label(t("Loop before the board Ze"));
                    ui.horizontal(|ui| {
                        let mut on = c.ze.is_some();
                        ui.checkbox(&mut on, "");
                        let mut ze = c.ze.unwrap_or(0.5);
                        ui.add_enabled(
                            on,
                            egui::DragValue::new(&mut ze)
                                .range(0.0..=5.0)
                                .speed(0.01)
                                .suffix(" Ω"),
                        );
                        c.ze = on.then_some(ze);
                    });
                    ui.end_row();
                });
                ui.separator();
                let r = electrical::check(&c);
                let mark = |ui: &mut egui::Ui, ok: Option<bool>, text: String| match ok {
                    Some(true) => {
                        ui.label(format!("✔ {text}"));
                    }
                    Some(false) => {
                        ui.colored_label(ui.visuals().warn_fg_color, format!("✖ {text}"));
                    }
                    None => {
                        ui.weak(text);
                    }
                };
                ui.label(
                    egui::RichText::new(format!("IB = {} A", num(r.ib, 1))).strong(),
                );
                mark(
                    ui,
                    Some(r.ok_du),
                    format!(
                        "{}: {} V = {} % ≤ {} %",
                        t("Voltage drop"),
                        num(r.u, 2),
                        num(r.du, 2),
                        num(r.du_limit, 0)
                    ),
                );
                match r.iz {
                    Some(iz) => mark(
                        ui,
                        r.ok_prot,
                        format!(
                            "{}: IB {} A ≤ In {} A ≤ Iz {} A",
                            t("Protection"),
                            num(r.ib, 1),
                            num(c.in_a, 0),
                            num(iz, 1)
                        ),
                    ),
                    None => mark(
                        ui,
                        None,
                        t("Protection: Iz is not in the tables for aluminium; take it from the maker."),
                    ),
                }
                match r.zs {
                    Some(zs) => mark(
                        ui,
                        r.ok_zs,
                        format!(
                            "{}: Zs {} Ω ≤ {} Ω",
                            t("Automatic disconnection"),
                            num(zs, 2),
                            num(r.zs_max, 2)
                        ),
                    ),
                    None => mark(
                        ui,
                        None,
                        t("Automatic disconnection: tick Ze to check the fault loop."),
                    ),
                }
                ui.add_space(4.0);
                if ui
                    .button(t("Propose breaker and cable"))
                    .on_hover_text(t("The smallest preferred breaker (10 A for lighting, 16 A otherwise) and the smallest copper cable that pass"))
                    .clicked()
                {
                    let min = if c.lighting { 10.0 } else { 16.0 };
                    match electrical::propose(&c, min) {
                        Some(p) => c = p,
                        None => failed = true,
                    }
                }
                ui.add_space(6.0);
                ui.weak(t("Copper cables with PVC insulation at 30 °C, one circuit alone: grouping and other temperatures lower Iz."));
                ui.weak("STN 33 2000-5-52 · STN 33 2000-4-43 · STN 33 2000-4-41 · STN EN 60898-1");
            });
        if failed {
            self.status =
                t("No cable up to 25 mm² passes: split the circuit or feed it with three phases.");
        }
        self.circuit = if open { Some(c) } else { None };
    }
}
