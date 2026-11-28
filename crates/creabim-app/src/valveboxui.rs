//! A medical gas valve box made to order: its gases, flush or on the
//! wall, through or not, the pipe size and its height
//! ([`creabim_core::catalogue::valvebox`]), then placed as any catalogue
//! object.

use super::CreaBimApp;
use super::Tool;
use creabim_core::catalogue::valvebox::{self, ValveBoxConfig, BOX_GASES, BOX_PIPES};
use eframe::egui::{self, Vec2};

impl CreaBimApp {
    /// `med:box` from the Medical gases tab: open the window.
    pub(super) fn valve_box_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "med" || arg != "box" {
            return false;
        }
        if self.valve_box.is_none() {
            self.valve_box = Some(ValveBoxConfig::default());
        }
        true
    }

    /// The window, while it is open.
    pub(super) fn valve_box_window(&mut self, ctx: &egui::Context) {
        let Some(mut c) = self.valve_box.clone() else {
            return;
        };
        let t = crate::i18n::tr;
        let mut open = true;
        let mut place = false;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Medical gas valve box"))
            .id(egui::Id::new("med.box"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(200.0, 200.0))
            .default_size(crate::app::shell::at_least(Vec2::new(400.0, 380.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                egui::Grid::new("med.box.grid")
                    .num_columns(2)
                    .spacing([14.0, 5.0])
                    .show(ui, |ui| {
                        ui.label(t("Gases"));
                        ui.horizontal(|ui| {
                            for (i, g) in BOX_GASES.iter().enumerate() {
                                ui.checkbox(&mut c.gases[i], *g);
                            }
                        });
                        ui.end_row();
                        // each gas's module: its fittings on or off, the
                        // ball valves always in
                        for (i, g) in BOX_GASES.iter().enumerate() {
                            if !c.gases[i] {
                                continue;
                            }
                            ui.label(format!("{} {g}", t("Module")));
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut c.nist[i], "NIST");
                                ui.checkbox(&mut c.gauge[i], t("Gauge"));
                                ui.checkbox(&mut c.sensor[i], t("Pressure switch (alarm)"));
                            });
                            ui.end_row();
                        }
                        ui.label(t("Mounting"));
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut c.wall, false, t("Flush in the wall"));
                            ui.selectable_value(&mut c.wall, true, t("On the wall"));
                        });
                        ui.end_row();
                        ui.label(t("Gas out at the bottom"));
                        ui.checkbox(&mut c.through, "");
                        ui.end_row();
                        ui.label(t("Copper pipe"));
                        egui::ComboBox::from_id_salt("med.box.pipe")
                            .selected_text(format!("Cu {:.0}", c.pipe()))
                            .show_ui(ui, |ui| {
                                for d in BOX_PIPES {
                                    ui.selectable_value(&mut c.pipe_mm, d, format!("Cu {d:.0}"));
                                }
                            });
                        ui.end_row();
                        ui.label(t("Inlet to outlet"));
                        ui.add(
                            egui::DragValue::new(&mut c.spacing_mm)
                                .range(40.0..=150.0)
                                .speed(1.0)
                                .suffix(" mm"),
                        );
                        ui.end_row();
                        ui.label(t("Underside above the floor"));
                        ui.add(
                            egui::DragValue::new(&mut c.height)
                                .range(0.3..=2.2)
                                .speed(0.01)
                                .suffix(" m"),
                        );
                        ui.end_row();
                    });
                ui.separator();
                let (w, h, d) = c.size();
                ui.label(egui::RichText::new(c.code()).strong());
                ui.weak(format!(
                    "{w:.0} × {h:.0} × {d:.0} mm · {} · STN EN ISO 7396-1, STN EN 13348",
                    t("in on the right, out on the left seen at the door")
                ));
                ui.weak(t(
                    "Each module has its ball valves (in and out); the NIST inlet, the control gauge and the pressure switch for the alarm go in as the requirements ask.",
                ));
                if !(1.0..=1.8).contains(&c.height) {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        t("Valve boxes are mounted 1.0 to 1.8 m above the floor (HTM 02-01)."),
                    );
                }
                ui.weak(t(
                    "Sizes after the Flídr medical UP range: check them with the maker. Its loss in the calculation comes from a valve box valve on the pipe (ζ = 3).",
                ));
                ui.add_space(4.0);
                if ui.button(t("Place it")).clicked() {
                    place = true;
                }
            });
        if place {
            let unit = valvebox::valve_box(&c);
            self.status = format!(
                "{}: {}",
                unit.name,
                t("click in a plan or the 3D view to place it. Space turns it; Esc ends.")
            );
            let what = unit.name.clone();
            self.plan.objects.current = Some(unit);
            self.plan.objects.offset = None;
            self.plan.objects.project_type = None;
            self.plan.cancel();
            self.plan.tool = Tool::Object;
            self.park(what);
        }
        self.valve_box = if open { Some(c) } else { None };
    }
}
