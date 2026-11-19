//! A rainwater soakaway of DRENBLOK blocks made to order and sized after
//! DWA-A 138 ([`creabim_core::catalogue::soakaway`]): the connected area,
//! the ground's permeability and the local rain series give the critical
//! rain and the trench length; the blocks follow, then it is placed as
//! any catalogue object.

use super::CreaBimApp;
use super::Tool;
use creabim_core::catalogue::soakaway::{
    self, critical, emptying_hours, Block, Filter, Rain, SoakawayConfig, BLOCK,
};
use eframe::egui::{self, Vec2};

/// The window's state: the soakaway and what sizes it.
#[derive(Clone, Debug)]
pub struct SoakawayDraft {
    pub c: SoakawayConfig,
    /// Roof and paved areas, m², and their runoff coefficients.
    pub roof: f64,
    pub roof_psi: f64,
    pub paved: f64,
    pub paved_psi: f64,
    /// The ground's permeability, m/s.
    pub kf: f64,
    /// DWA-A 138's surcharge factor.
    pub fz: f64,
    /// The rain series for the chosen return period: duration, min, and
    /// intensity, l/(s·ha).
    pub rains: Vec<Rain>,
}

impl Default for SoakawayDraft {
    fn default() -> Self {
        Self {
            c: SoakawayConfig::default(),
            roof: 150.0,
            roof_psi: 1.0,
            paved: 0.0,
            paved_psi: 0.6,
            kf: 1e-5,
            fz: 1.2,
            rains: [
                5.0, 10.0, 15.0, 20.0, 30.0, 45.0, 60.0, 90.0, 120.0, 180.0, 240.0, 360.0, 720.0,
                1440.0,
            ]
            .iter()
            .map(|m| Rain {
                minutes: *m,
                intensity: 0.0,
            })
            .collect(),
        }
    }
}

impl SoakawayDraft {
    /// The connected area A_u, m².
    pub fn area_u(&self) -> f64 {
        self.roof * self.roof_psi + self.paved * self.paved_psi
    }
}

impl CreaBimApp {
    /// `site:soakaway`: open the window.
    pub(super) fn soakaway_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "site" || arg != "soakaway" {
            return false;
        }
        if self.soakaway.is_none() {
            self.soakaway = Some(SoakawayDraft::default());
        }
        true
    }

    /// The window, while it is open.
    pub(super) fn soakaway_window(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.soakaway.clone() else {
            return;
        };
        let t = crate::i18n::tr;
        let mut open = true;
        let mut place = false;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Soakaway (DRENBLOK)"))
            .id(egui::Id::new("site.soakaway"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(260.0, 300.0))
            .default_size(Vec2::new(520.0, 640.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.strong(t("What drains into it"));
                    egui::Grid::new("site.soak.area").num_columns(3).spacing([12.0, 4.0]).show(ui, |ui| {
                        ui.label(t("Roofs"));
                        ui.add(egui::DragValue::new(&mut d.roof).range(0.0..=1e5).speed(1.0).suffix(" m²"));
                        ui.add(egui::DragValue::new(&mut d.roof_psi).range(0.0..=1.0).speed(0.01).prefix("Ψ "));
                        ui.end_row();
                        ui.label(t("Paved areas"));
                        ui.add(egui::DragValue::new(&mut d.paved).range(0.0..=1e5).speed(1.0).suffix(" m²"));
                        ui.add(egui::DragValue::new(&mut d.paved_psi).range(0.0..=1.0).speed(0.01).prefix("Ψ "));
                        ui.end_row();
                        ui.label(t("Connected area A_u"));
                        ui.label(format!("{:.1} m²", d.area_u()));
                        ui.end_row();
                        ui.label(t("Permeability k_f"));
                        egui::ComboBox::from_id_salt("site.soak.kf")
                            .selected_text(format!("{:.0e} m/s", d.kf))
                            .show_ui(ui, |ui| {
                                for (k, what) in [
                                    (1e-3, "gravel"),
                                    (5e-4, "sandy gravel"),
                                    (1e-4, "coarse sand"),
                                    (5e-5, "medium sand"),
                                    (1e-5, "fine sand"),
                                    (5e-6, "silty sand"),
                                    (1e-6, "sandy silt (the least for a soakaway)"),
                                ] {
                                    ui.selectable_value(&mut d.kf, k, format!("{k:.0e} m/s · {}", t(what)));
                                }
                            });
                        ui.label(t("from the soil survey"));
                        ui.end_row();
                        ui.label(t("Surcharge factor f_z"));
                        ui.add(egui::DragValue::new(&mut d.fz).range(1.0..=1.5).speed(0.01));
                        ui.end_row();
                    });
                    ui.add_space(6.0);
                    ui.strong(t("Rain series for the return period (5 years usually)"));
                    ui.weak(t("Intensities in l/(s·ha) for the place, from the SHMÚ rain series or the maker's program: the critical rain is the one that needs the longest trench."));
                    egui::Grid::new("site.soak.rain").num_columns(4).spacing([12.0, 2.0]).show(ui, |ui| {
                        for (i, r) in d.rains.iter_mut().enumerate() {
                            ui.label(format!("{:.0} min", r.minutes));
                            ui.add(egui::DragValue::new(&mut r.intensity).range(0.0..=1000.0).speed(1.0));
                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });
                    ui.add_space(6.0);
                    ui.strong(t("Blocks"));
                    egui::Grid::new("site.soak.blocks").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                        ui.label(t("Block"));
                        ui.horizontal(|ui| {
                            for b in Block::ALL {
                                ui.selectable_value(&mut d.c.block, b, b.name());
                            }
                        });
                        ui.end_row();
                        ui.label(t("Layers"));
                        ui.add(egui::DragValue::new(&mut d.c.layers).range(1..=5));
                        ui.end_row();
                        ui.label(t("Blocks across"));
                        ui.add(egui::DragValue::new(&mut d.c.across).range(1..=20));
                        ui.end_row();
                        ui.label(t("Blocks along"));
                        ui.add(egui::DragValue::new(&mut d.c.along).range(1..=200));
                        ui.end_row();
                        ui.label(t("Earth over the blocks"));
                        ui.add(egui::DragValue::new(&mut d.c.cover).range(0.3..=4.6).speed(0.05).suffix(" m"));
                        ui.end_row();
                        ui.label(t("Filter shaft"));
                        egui::ComboBox::from_id_salt("site.soak.filter")
                            .selected_text(t(d.c.filter.name()))
                            .show_ui(ui, |ui| {
                                for f in Filter::ALL {
                                    ui.selectable_value(&mut d.c.filter, f, t(f.name()));
                                }
                            });
                        ui.end_row();
                        ui.label(t("Inlet"));
                        egui::ComboBox::from_id_salt("site.soak.dn")
                            .selected_text(format!("DN {}", d.c.inlet_dn))
                            .show_ui(ui, |ui| {
                                for dn in [110, 160, 200, 250, 315] {
                                    ui.selectable_value(&mut d.c.inlet_dn, dn, format!("DN {dn}"));
                                }
                            });
                        ui.end_row();
                    });
                    ui.separator();
                    let (_, w, h) = d.c.size();
                    let au = d.area_u();
                    match critical(au, &d.rains, w, h, d.kf, d.fz) {
                        Some((r, len)) => {
                            let need = (len / BLOCK).ceil().max(1.0) as u32;
                            ui.label(egui::RichText::new(format!(
                                "{}: {:.0} min, {:.0} l/(s·ha) · {} {:.2} m · {} {} × {} × {} = {} {}",
                                t("Critical rain"),
                                r.minutes,
                                r.intensity,
                                t("trench length"),
                                len,
                                t("blocks"),
                                need,
                                d.c.across,
                                d.c.layers,
                                need * d.c.across * d.c.layers,
                                t("pcs")
                            )).strong());
                            let hours = emptying_hours(w, h, d.kf);
                            let ok = hours <= 24.0;
                            let text = format!("{} {:.1} h", t("Emptying time"), hours);
                            if ok {
                                ui.label(text);
                            } else {
                                ui.colored_label(ui.visuals().warn_fg_color, format!("{text}: {}", t("over a day: fewer layers, wider, or another way to drain it")));
                            }
                            if d.c.along != need && ui.button(format!("{} {need}", t("Blocks along:"))).clicked() {
                                d.c.along = need;
                            }
                        }
                        None => {
                            ui.weak(t("Type the rain series to work out the length."));
                        }
                    }
                    if d.c.cover + h > d.c.max_cover() + 0.6 {
                        ui.colored_label(ui.visuals().warn_fg_color, t("Deeper than the maker allows for this many layers."));
                    }
                    if let Some((_, _, a)) = d.c.filter.size() {
                        if d.roof + d.paved > a {
                            ui.colored_label(ui.visuals().warn_fg_color, format!("{} {}", t("The filter shaft serves up to"), format_args!("{a:.0} m²")));
                        }
                    }
                    ui.weak(format!(
                        "{} · {:.2} m³ · {:.1} × {:.1} × {:.1} m",
                        d.c.code(),
                        d.c.storage(),
                        d.c.size().0,
                        w,
                        h
                    ));
                    ui.add_space(4.0);
                    if ui.button(t("Place it")).clicked() {
                        place = true;
                    }
                });
            });
        if place {
            let unit = soakaway::soakaway(&d.c);
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
        self.soakaway = if open { Some(d) } else { None };
    }
}
