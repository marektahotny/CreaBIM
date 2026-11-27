//! The cooling load ([`creabim_services::coolingload`], after STN 73
//! 0548): a window with every room's load in summer, what it is made
//! of, and the fan-coil that covers it.

use super::CreaBimApp;
use creabim_services::coolingload::{self, Mass, Method, Shading};
use creabim_services::coolnorm;
use eframe::egui::{self, Vec2};

impl CreaBimApp {
    /// `calc:cooling` from the Ventilation tab: open the window.
    pub(super) fn cooling_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" || !matches!(arg, "cooling" | "cooling-norm") {
            return false;
        }
        if self.cooling.is_none() {
            // reopened: as it was last written (George 2026-11-27)
            self.cooling = Some(self.saved_settings("cooling").unwrap_or_default());
        }
        let Some(s) = self.cooling.as_mut() else {
            return false;
        };
        if arg == "cooling-norm" {
            s.method = Method::Norm;
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
        let o = self.heat_opts();
        let b = coolingload::building(self.doc.model(), &o, &s);
        let norm_tables = coolnorm::tables_of(self.doc.model());
        let months = creabim_core::fireparams::summer_max(&self.climate_tables());
        let mut load = false;
        let found = creabim_services::calcresults::cooling(&b).with_settings(&s);
        let written = self.is_written(&found);
        let mut write_now = false;
        let mut open = true;
        let mut select = None;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Cooling load"))
            .id(egui::Id::new("cooling.load"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(340.0, 260.0))
            .default_size(crate::app::shell::at_least(Vec2::new(680.0, 520.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                if super::calcresultsui::write_button(ui, written) {
                    write_now = true;
                }
                ui.horizontal(|ui| {
                    ui.label(t("Method"));
                    egui::ComboBox::from_id_salt("cooling.method")
                        .selected_text(t(s.method.name()))
                        .show_ui(ui, |ui| {
                            for x in Method::ALL {
                                ui.selectable_value(&mut s.method, x, t(x.name()));
                            }
                        });
                });
                if s.method == Method::Norm {
                    let eps = coolnorm::rows(&norm_tables, coolnorm::EPS, "eps");
                    let shades = coolnorm::rows(&norm_tables, coolnorm::SHADE, "s");
                    // an older copy of the file lacks the months, A and the latitude
                    let stale = coolnorm::factor(&norm_tables, "amplitude").is_none()
                        || coolnorm::factor(&norm_tables, "latitude").is_none()
                        || coolnorm::sun_of(&norm_tables, coolnorm::SUN_WALL, s.month, Some(180.0), 12).is_none()
                        || coolnorm::sun_of(&norm_tables, coolnorm::SUN_GLASS, s.month, Some(180.0), 12).is_none();
                    if norm_tables.len() < coolnorm::TABLES.len() || stale {
                        ui.horizontal(|ui| {
                            ui.colored_label(
                                ui.visuals().warn_fg_color,
                                t("The tables of STN 73 0548 are not read: read its parameter file"),
                            );
                            if ui.button(t("Read the table…")).clicked() {
                                load = true;
                            }
                        });
                    }
                    ui.horizontal_wrapped(|ui| {
                        let pick = |ui: &mut egui::Ui, id: &str, label: &str, v: &mut Option<usize>, default: &str| {
                            ui.label(t(label));
                            let now = coolnorm::picked(&eps, *v, default)
                                .map_or(String::from("–"), |r| format!("{} ({})", r.text, num(r.value, 2)));
                            egui::ComboBox::from_id_salt(id)
                                .selected_text(now)
                                .width(220.0)
                                .show_ui(ui, |ui| {
                                    for (i, r) in eps.iter().enumerate() {
                                        let text = format!("{} ({})", r.text, num(r.value, 2));
                                        let on = coolnorm::picked(&eps, *v, default).is_some_and(|x| x.key == r.key);
                                        if ui.selectable_label(on, text).clicked() {
                                            *v = Some(i);
                                        }
                                    }
                                })
                                .response
                                .on_hover_text(t("ε of the outer surface for the sun (STN 73 0548, table 12)"));
                        };
                        pick(ui, "cooling.eps_wall", "Walls ε", &mut s.eps_wall, coolingload::EPS_WALL_KEY);
                        pick(ui, "cooling.eps_roof", "Roof ε", &mut s.eps_roof, coolingload::EPS_ROOF_KEY);
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(t("Sun shading"));
                        let now = s
                            .shade
                            .and_then(|i| shades.get(i))
                            .map_or(t("No shading"), |r| format!("{} (s {})", r.text, num(r.value, 2)));
                        egui::ComboBox::from_id_salt("cooling.shade")
                            .selected_text(now)
                            .width(260.0)
                            .show_ui(ui, |ui| {
                                if ui.selectable_label(s.shade.is_none(), t("No shading")).clicked() {
                                    s.shade = None;
                                }
                                for (i, r) in shades.iter().enumerate() {
                                    let text = format!("{} (s {})", r.text, num(r.value, 2));
                                    if ui.selectable_label(s.shade == Some(i), text).clicked() {
                                        s.shade = Some(i);
                                    }
                                }
                            })
                            .response
                            .on_hover_text(t("The shading coefficient s (STN 73 0548, table 11)"));
                        ui.label(t("Allowed rise"))
                            .on_hover_text(t("How much warmer the room may get at the peak: the sun's load is cut by what the inner walls, floor and ceiling store (STN 73 0548, art. 68 and 69)"));
                        let mut dt = s
                            .dt_allow
                            .or_else(|| coolnorm::factor(&norm_tables, "dt_allow"))
                            .unwrap_or(0.0);
                        if ui
                            .add(egui::DragValue::new(&mut dt).range(0.0..=5.0).speed(0.1).suffix(" K"))
                            .changed()
                        {
                            s.dt_allow = Some(dt);
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        const MONTHS: [&str; 8] = [
                            "March", "April", "May", "June", "July", "August", "September", "October",
                        ];
                        let name = |m: u8| t(MONTHS[(m.clamp(3, 10) - 3) as usize]);
                        ui.label(t("Month"));
                        egui::ComboBox::from_id_salt("cooling.norm_month")
                            .selected_text(name(s.month))
                            .show_ui(ui, |ui| {
                                for m in 3..=10u8 {
                                    ui.selectable_value(&mut s.month, m, name(m));
                                }
                            })
                            .response
                            .on_hover_text(t("The sun of the 21st of the month (STN 73 0548, tables 5 and 10) and its outside air (table 1, formula 1)"));
                        match (
                            coolnorm::te_max(&norm_tables, s.month),
                            coolnorm::factor(&norm_tables, "amplitude"),
                        ) {
                            (Some(te), Some(a)) => {
                                ui.label(format!("θe,max {} °C · A {} K", num(te, 1), num(a, 1)))
                                    .on_hover_text(t("STN 73 0548, table 1 and formula 1: the method takes these, not the outside temperature below"));
                            }
                            _ => {
                                ui.label(t("θe,max as below (table 1 not read)"));
                            }
                        }
                        ui.label(t("Air"));
                        egui::ComboBox::from_id_salt("cooling.region")
                            .selected_text(t(s.region.name()))
                            .show_ui(ui, |ui| {
                                for x in coolingload::Region::ALL {
                                    ui.selectable_value(&mut s.region, x, t(x.name()));
                                }
                            })
                            .response
                            .on_hover_text(t("How clear the air is: the correction c_o of the sun through the windows (STN 73 0548, art. 64)"));
                        ui.checkbox(&mut s.reveal, t("Reveals shade the glass"))
                            .on_hover_text(t("The sunlit part of each window by the depth of its reveals and where the sun stands (STN 73 0548, formulas 24 to 26)"));
                    });
                }
                ui.horizontal_wrapped(|ui| {
                    ui.label(t("Outside in summer"));
                    ui.add(
                        egui::DragValue::new(&mut s.te)
                            .range(25.0..=45.0)
                            .speed(0.5)
                            .suffix(" °C"),
                    );
                    if !months.is_empty() {
                        egui::ComboBox::from_id_salt("cooling.month")
                            .selected_text(t("Temperatures"))
                            .show_ui(ui, |ui| {
                                for (m, v) in &months {
                                    let text = format!("{m}   {} °C", num(*v, 1));
                                    if ui.selectable_label((s.te - *v).abs() < 1e-9, text).clicked() {
                                        s.te = *v;
                                    }
                                }
                            })
                            .response
                            .on_hover_text(t("The highest outside temperature of a summer month (STN 73 0548)"));
                    } else if ui
                        .button(t("Temperatures"))
                        .on_hover_text(t("Read the table of localities and summer temperatures (TSV)"))
                        .clicked()
                    {
                        load = true;
                    }
                    ui.label(t("Daily range"))
                        .on_hover_text(t("The outside air's daily range in the summer months: the hours before and after 15 h are cooler"));
                    ui.add(
                        egui::DragValue::new(&mut s.swing)
                            .range(0.0..=25.0)
                            .speed(0.5)
                            .suffix(" °C"),
                    );
                    ui.label(t("Used from"));
                    ui.add(egui::DragValue::new(&mut s.from_h).range(0..=23).suffix(" h"));
                    ui.label(t("to"));
                    ui.add(egui::DragValue::new(&mut s.to_h).range(0..=23).suffix(" h"));
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
                    if s.method == Method::Norm {
                        return;
                    }
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
                ui.horizontal_wrapped(|ui| {
                    ui.label(t("Summer stability: at most"))
                        .on_hover_text(t("The highest room air temperature on a summer day without cooling (STN 73 0540-2: 27 °C)"));
                    ui.add(
                        egui::DragValue::new(&mut s.summer_tmax)
                            .range(24.0..=32.0)
                            .speed(0.5)
                            .suffix(" °C"),
                    );
                    ui.label(t("Airing by windows"));
                    ui.add(
                        egui::DragValue::new(&mut s.summer_ach)
                            .range(0.0..=6.0)
                            .speed(0.1)
                            .suffix(" 1/h"),
                    );
                    ui.label(t("Inner layers"));
                    egui::ComboBox::from_id_salt("cooling.mass")
                        .selected_text(t(s.mass.name()))
                        .show_ui(ui, |ui| {
                            for x in Mass::ALL {
                                ui.selectable_value(&mut s.mass, x, t(x.name()));
                            }
                        });
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
                    use crate::sheet::{self, Col};
                    let cols = [
                        Col::text("Room"),
                        Col::num("A", "m²"),
                        Col::num("Φ", "W"),
                        Col::num("Φ/A", "W/m²"),
                        Col::text("Worst hour"),
                        Col::num("Fan-coil", "kW"),
                        Col::num("Summer θmax", "°C"),
                    ];
                    let total = |row: &mut sheet::Row| {
                        row.text(t("Total of the cooled rooms"));
                        row.text(num(b.area, 1));
                        row.text(num(b.phi, 0));
                        row.text(num(if b.area > 0.0 { b.phi / b.area } else { 0.0 }, 0));
                        row.empty();
                        row.text(num(
                            b.rooms.iter().filter(|r| r.cooled).map(|r| r.fancoil_kw).sum::<f64>(),
                            1,
                        ));
                    };
                    sheet::show(ui, "cooling.rooms", &cols, b.rooms.len(), 360.0, |row, i| {
                        let r = &b.rooms[i];
                        let name = format!("{} {}", r.number, r.name);
                        row.cell(|ui| {
                            if ui.link(name.trim()).clicked() {
                                select = Some(r.id);
                            }
                        });
                        row.text(num(r.area, 1));
                        if r.cooled {
                            row.strong(num(r.phi, 0));
                        } else {
                            row.cell(|ui| {
                                ui.weak(num(r.phi, 0))
                                    .on_hover_text(t("Not cooled: a bathroom, corridor or store is left out of the total."));
                            });
                        }
                        row.text(num(if r.area > 0.0 { r.phi / r.area } else { 0.0 }, 0));
                        row.text(r.hour.map_or(String::new(), |h| format!("{h}:00")));
                        if r.cooled {
                            row.text(num(r.fancoil_kw, 1));
                        } else {
                            row.empty();
                        }
                        let text = num(r.summer.t_max, 1);
                        if r.summer.ok {
                            row.text(text);
                        } else {
                            row.cell(|ui| {
                                ui.colored_label(egui::Color32::from_rgb(200, 40, 40), text)
                                    .on_hover_text(t("Above the summer limit"));
                            });
                        }
                    }, Some(&total));
                    ui.add_space(6.0);
                    for r in &b.rooms {
                        let title = format!("{} {} – {} W", r.number, r.name, num(r.phi, 0));
                        egui::CollapsingHeader::new(title.trim().to_string())
                            .id_salt(("cooling.room", r.id))
                            .show(ui, |ui| {
                                let cols = [Col::text("Gain"), Col::num("Φ", "W"), Col::text("From")];
                                let total = |row: &mut sheet::Row| {
                                    row.text(t("Total"));
                                    row.text(num(r.phi, 0));
                                };
                                sheet::show(ui, ("cooling.items", r.id), &cols, r.items.len(), 400.0, |row, k| {
                                    let i = &r.items[k];
                                    row.text(t(i.kind));
                                    row.text(num(i.phi, 0));
                                    row.weak(i.note.clone());
                                }, Some(&total));
                                let su = &r.summer;
                                ui.label(format!(
                                    "{}: {} {} °C, {} {} °C – {}",
                                    t("Summer without cooling"),
                                    t("highest"),
                                    num(su.t_max, 1),
                                    t("mean"),
                                    num(su.t_mean, 1),
                                    if su.ok { t("meets the limit") } else { t("above the limit") },
                                ));
                                if !su.ok {
                                    let fix = coolingload::summer_remedy(self.doc.model(), &o, &s, r.id);
                                    ui.label(format!("{}: {}", t("What helps"), t(fix)));
                                }
                                ui.weak(&su.note);
                            });
                    }
                    ui.add_space(6.0);
                    if s.method == Method::Norm {
                        ui.weak(t("By STN 73 0548: the sun through the windows (table 10) with g and the shading s of table 11, cut by what the inner walls, floor and ceiling store (formula 27, art. 69); walls and roof by the sol-air temperature with ε of table 12 and the sun of table 5 (formula 29), damped and delayed by their thickness (formulas 30 to 32); people, lighting, appliances and ventilation. Each room at its own worst hour."));
                    } else {
                        ui.weak(t("A simplified load: the sun through the windows on 21 July at 50° north by where each faces, heat through walls, roof and windows with the U values of the heat loss, people, lighting, appliances and ventilation. Each room at its own worst hour. Fan-coils: sensible output at 7/12 °C, usual values to design with."));
                    }
                    ui.weak(t("Summer stability roughly by the admittance method: the day's mean from the mean gains and the loss, the swing from the peaks damped by what the inner surfaces store; airing by opening windows."));
                    ui.weak("STN 73 0548 · STN EN ISO 52016-1 · STN 73 0540-2 · STN EN ISO 13792");
                });
            });
        if let Some(id) = select {
            self.selection.only(id);
        }
        if write_now {
            self.write_results_said(found);
        }
        self.cooling = if open { Some(s) } else { None };
        if load {
            self.load_climate_sites();
        }
    }
}
