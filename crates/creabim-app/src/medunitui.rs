//! A medical supply unit made to order: the wall rail NR-H3 or the
//! ceiling bridge SM-P 5.4 with the outlets, sockets and the rest it is
//! to carry ([`creabim_core::catalogue::medical`]), then placed as any
//! catalogue object.

use super::CreaBimApp;
use super::Tool;
use creabim_core::catalogue::medical::{self, MedUnitConfig, MedUnitKind, MED_GASES};
use eframe::egui::{self, Vec2};

impl CreaBimApp {
    /// `med:unit` from the Medical gases tab: open the window.
    pub(super) fn med_unit_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "med" || arg != "unit" {
            return false;
        }
        if self.med_unit.is_none() {
            self.med_unit = Some(MedUnitConfig::usual(MedUnitKind::Rail));
        }
        true
    }

    /// The window, while it is open.
    pub(super) fn med_unit_window(&mut self, ctx: &egui::Context) {
        let Some(mut c) = self.med_unit.clone() else {
            return;
        };
        let t = crate::i18n::tr;
        let mut open = true;
        let mut place = false;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Medical supply unit"))
            .id(egui::Id::new("med.unit"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(210.0, 240.0))
            .default_size(Vec2::new(420.0, 480.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for k in [MedUnitKind::Rail, MedUnitKind::Bridge] {
                        if ui.selectable_label(c.kind == k, k.name()).clicked() && c.kind != k {
                            c = MedUnitConfig::usual(k);
                        }
                    }
                });
                let bridge = c.kind == MedUnitKind::Bridge;
                let count = |ui: &mut egui::Ui, text: &str, v: &mut u32, hi: u32| {
                    ui.label(text);
                    ui.add(egui::DragValue::new(v).range(0..=hi));
                    ui.end_row();
                };
                egui::Grid::new("med.unit.grid")
                    .num_columns(2)
                    .spacing([14.0, 5.0])
                    .show(ui, |ui| {
                        ui.label(t("Length"));
                        ui.add(
                            egui::DragValue::new(&mut c.length_mm)
                                .range(600.0..=6000.0)
                                .speed(50.0)
                                .suffix(" mm"),
                        );
                        ui.end_row();
                        for (i, gas) in MED_GASES.iter().enumerate() {
                            count(ui, &format!("{} {gas}", t("Outlets")), &mut c.gases[i], 12);
                        }
                        count(ui, &t("Sockets 230 V"), &mut c.sockets, 24);
                        count(ui, &t("Sockets 230 V, backed up"), &mut c.sockets_backed, 24);
                        count(ui, &t("Data sockets RJ45"), &mut c.data, 12);
                        count(ui, &t("USB charging"), &mut c.usb, 12);
                        ui.label(t("Nurse call"));
                        ui.checkbox(&mut c.nurse, "");
                        ui.end_row();
                        ui.label(t("Light switch"));
                        ui.checkbox(&mut c.light, "");
                        ui.end_row();
                        ui.label(t("Gases on the left"));
                        ui.checkbox(&mut c.gases_left, "");
                        ui.end_row();
                        if bridge {
                            ui.label(t("Outlets on both sides"));
                            ui.checkbox(&mut c.two_sided, "");
                            ui.end_row();
                        }
                        ui.label(if bridge {
                            t("Underside above the floor")
                        } else {
                            t("Axis above the floor")
                        });
                        ui.add(
                            egui::DragValue::new(&mut c.height)
                                .range(0.3..=3.0)
                                .speed(0.01)
                                .suffix(" m"),
                        );
                        ui.end_row();
                        if bridge {
                            ui.label(t("Ceiling above the floor"));
                            ui.add(
                                egui::DragValue::new(&mut c.ceiling)
                                    .range(c.height + 0.4..=6.0)
                                    .speed(0.01)
                                    .suffix(" m"),
                            );
                            ui.end_row();
                        }
                    });
                c.ceiling = c.ceiling.max(c.height + 0.4);
                ui.separator();
                let l = medical::layout(&c);
                ui.label(egui::RichText::new(c.code()).strong());
                if l.lengthened {
                    ui.weak(format!(
                        "{} {:.0} mm.",
                        t("The outlets don't fit the length asked: the unit is made"),
                        l.length
                    ));
                }
                let sockets = (c.sockets + c.sockets_backed) as f64;
                ui.weak(format!(
                    "{}: {} kVA · Cu 12 × 1 · STN EN ISO 11197, STN EN ISO 9170-1, STN EN ISO 7396-1",
                    t("Sockets' load (200 VA each)"),
                    crate::i18n::num(sockets * 0.2, 1)
                ));
                ui.weak(t(
                    "The profile's depth (rail) and height (bridge) are estimates: check them with the maker.",
                ));
                ui.add_space(4.0);
                if ui.button(t("Place it")).clicked() {
                    place = true;
                }
            });
        if place {
            let unit = medical::unit(&c);
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
        self.med_unit = if open { Some(c) } else { None };
    }
}
