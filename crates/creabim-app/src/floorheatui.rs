//! Underfloor heating ([`creabim_services::floorheat`], STN EN 1264): a
//! window with every room's design from its heat loss, a switch that
//! gives a room its loops (drawn in the plan), its spacing, and what to
//! set at the manifold.

use super::CreaBimApp;
use creabim_core::underfloor::Underfloor;
use creabim_core::{ElementId, ElementKind, Transaction};
use creabim_services::floorheat::{self, Settings, SPACINGS};
use creabim_services::heatloss;
use eframe::egui::{self, Vec2};

impl CreaBimApp {
    /// `calc:floorheat` from the Heating tab: open the window.
    pub(super) fn floor_heat_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" || arg != "floorheat" {
            return false;
        }
        if self.floor_heat.is_none() {
            self.floor_heat = Some(Settings::default());
        }
        self.results_due = Some("floorheat");
        true
    }

    /// The window, while it is open.
    pub(super) fn floor_heat_window(&mut self, ctx: &egui::Context) {
        let Some(mut s) = self.floor_heat else {
            return;
        };
        let t = crate::i18n::tr;
        let num = crate::i18n::num;
        let o = self
            .heat_loss
            .as_ref()
            .map_or_else(heatloss::Opts::default, |x| x.0);
        let d = floorheat::design(self.doc.model(), &o, &s);
        let mut write_now = self.results_due.take_if(|c| *c == "floorheat").is_some();
        let mut open = true;
        let mut select = None;
        // a room's underfloor heating to set (none takes it away)
        let mut set: Vec<(ElementId, Option<Underfloor>)> = vec![];
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Underfloor heating"))
            .id(egui::Id::new("floor.heat"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(360.0, 260.0))
            .default_size(crate::app::shell::at_least(Vec2::new(720.0, 520.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                if super::calcresultsui::write_button(ui) {
                    write_now = true;
                }
                ui.horizontal(|ui| {
                    ui.label(t("Supply temperature"));
                    ui.add(
                        egui::DragValue::new(&mut s.supply_c)
                            .range(25.0..=55.0)
                            .speed(0.5)
                            .suffix(" °C"),
                    );
                    ui.label(t("Drop"));
                    ui.add(
                        egui::DragValue::new(&mut s.sigma_k)
                            .range(3.0..=10.0)
                            .speed(0.5)
                            .suffix(" K"),
                    );
                    ui.weak(format!(
                        "θe {} °C ({})",
                        num(o.te, 0),
                        t("from the heat loss window")
                    ));
                });
                if d.rooms.is_empty() {
                    ui.label(t("No rooms to calculate: place rooms first."));
                    return;
                }
                let on: Vec<&floorheat::RoomDesign> = d.rooms.iter().filter(|x| x.on).collect();
                ui.horizontal(|ui| {
                    if ui.button(t("All rooms")).clicked() {
                        for x in d.rooms.iter().filter(|x| !x.on) {
                            set.push((x.id, Some(x.underfloor(None))));
                        }
                    }
                    if ui
                        .add_enabled(!on.is_empty(), egui::Button::new(t("No room")))
                        .clicked()
                    {
                        for x in &on {
                            set.push((x.id, None));
                        }
                    }
                    if !on.is_empty() {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} kW · {} {} · {} m³/h · {} {} kPa",
                                num(d.output_kw, 2),
                                d.loops,
                                t("loops"),
                                num(d.flow_ls * 3.6, 2),
                                t("pump head"),
                                num(d.head_kpa, 1),
                            ))
                            .strong(),
                        );
                    }
                });
                if d.no_manifold {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        t("No manifold stands in the model: the loops' way to it is not counted. Place one from the catalogue (Underfloor heating manifold)."),
                    );
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    use crate::sheet::{self, Col};
                    let cols = [
                        Col::text(""),
                        Col::text("Room"),
                        Col::num("Φ", "W"),
                        Col::num("q / needed", "W/m²"),
                        Col::text("Spacing"),
                        Col::num("RλB", "m²K/W"),
                        Col::num("Loops", "pcs"),
                        Col::num("Loop", "m"),
                        Col::num("Flow", "l/h"),
                        Col::num("Δp", "kPa"),
                        Col::num("Throttle by", "kPa"),
                    ];
                    let on: Vec<_> = d.rooms.iter().filter(|x| x.on).collect();
                    let total = |row: &mut sheet::Row| {
                        row.empty();
                        row.text(t("Total"));
                        row.text(num(on.iter().map(|x| x.phi).sum::<f64>(), 0));
                        row.empty();
                        row.empty();
                        row.empty();
                        row.text(d.loops.to_string());
                        row.empty();
                        row.text(num(d.flow_ls * 3600.0, 0));
                        row.text(num(d.critical_kpa, 1));
                    };
                    sheet::show(ui, "floor.heat.rooms", &cols, d.rooms.len(), 400.0, |row, i| {
                        let x = &d.rooms[i];
                        row.cell(|ui| {
                            let mut is_on = x.on;
                            if ui.checkbox(&mut is_on, "").changed() {
                                set.push((x.id, is_on.then(|| x.underfloor(None))));
                            }
                        });
                        let name = format!("{} {}", x.number, x.name);
                        row.cell(|ui| {
                            if ui.link(name.trim()).clicked() {
                                select = Some(x.id);
                            }
                        });
                        row.text(num(x.phi, 0));
                        let q = format!("{} / {}", num(x.q, 0), num(x.q_req, 0));
                        if x.ok {
                            row.text(q);
                        } else {
                            row.cell(|ui| {
                                ui.colored_label(ui.visuals().warn_fg_color, q).on_hover_text(format!(
                                    "{} {} W {} {} W (qG {} W/m²)",
                                    t("The floor gives"),
                                    num(x.covered, 0),
                                    t("of"),
                                    num(x.phi, 0),
                                    num(x.q_limit, 0),
                                ));
                            });
                        }
                        row.cell(|ui| {
                                if x.on {
                                    let mut sp = x.spacing;
                                    egui::ComboBox::from_id_salt(("floor.heat.t", x.id))
                                        .width(70.0)
                                        .selected_text(format!("{} mm", num(sp * 1000.0, 0)))
                                        .show_ui(ui, |ui| {
                                            for v in SPACINGS {
                                                ui.selectable_value(
                                                    &mut sp,
                                                    v,
                                                    format!("{} mm", num(v * 1000.0, 0)),
                                                );
                                            }
                                        });
                                    if (sp - x.spacing).abs() > 1e-9 {
                                        // loops for the new spacing
                                        let coil = x.area / sp;
                                        let loops = ((coil + 2.0 * x.feed) / floorheat::MAX_LOOP_M)
                                            .ceil()
                                            .max(1.0) as u32;
                                        set.push((
                                            x.id,
                                            Some(Underfloor {
                                                spacing: sp,
                                                loops,
                                                finish_r: None,
                                            }),
                                        ));
                                    }
                                } else {
                                    ui.weak(format!("{} mm", num(x.spacing * 1000.0, 0)));
                                }
                        });
                        row.text(num(x.finish_r, 2));
                        row.text(x.loops.to_string());
                        row.text(num(x.loop_len, 0));
                        row.text(num(x.flow_ls * 3600.0, 0));
                        row.text(num(x.dp_kpa, 1));
                        if x.on {
                            row.text(num(d.critical_kpa - x.dp_kpa, 1));
                        }
                    }, Some(&total));
                    ui.add_space(6.0);
                    ui.weak(t("System A, PE-Xa / PE-RT 16 × 2 under 45 mm of screed, loops of 100 m at most. Yellow: the floor alone does not cover the room's loss (the limit flux or the spacing): add a radiator or raise the supply temperature. RλB comes from the room's floor finish."));
                    ui.weak("STN EN 1264-2 · STN EN 1264-4 · STN EN 12831-1");
                });
            });
        if let Some(id) = select {
            self.selection.only(id);
        }
        if !set.is_empty() {
            let mut tx = Transaction::new("Underfloor heating");
            for (id, u) in set {
                let Some(before) = self.doc.model().get(id).cloned() else {
                    continue;
                };
                let mut after = before.clone();
                if let ElementKind::Room(r) = &mut after.kind {
                    r.ufh = u;
                }
                tx = tx.replace(before, after);
            }
            match self.doc.commit(tx) {
                Ok(_) => self.status = t("Underfloor heating"),
                Err(e) => self.status = format!("Couldn't do that: {e}"),
            }
        }
        if write_now {
            self.write_results_said(creabim_services::calcresults::floor_heat(&d));
        }
        self.floor_heat = if open { Some(s) } else { None };
    }
}
