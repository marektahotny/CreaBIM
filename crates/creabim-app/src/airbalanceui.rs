//! The air balance ([`creabim_services::airbalance`], STN EN 16798-1):
//! a window with the air every room needs, whether it is supplied or
//! exhausted, what its diffusers and grilles carry, and the flow the
//! air handling unit is sized for.

use super::CreaBimApp;
use creabim_services::{airbalance, heatloss};
use eframe::egui::{self, Vec2};

impl CreaBimApp {
    /// `calc:air` from the Ventilation tab: open the window.
    pub(super) fn air_balance_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" || arg != "air" {
            return false;
        }
        self.air_balance = true;
        self.results_due = Some("air");
        true
    }

    /// The window, while it is open.
    pub(super) fn air_balance_window(&mut self, ctx: &egui::Context) {
        if !self.air_balance {
            return;
        }
        let t = crate::i18n::tr;
        let num = crate::i18n::num;
        let o = self
            .heat_loss
            .as_ref()
            .map_or_else(heatloss::Opts::default, |x| x.0);
        let b = airbalance::balance(self.doc.model(), &o);
        let mut write_now = self.results_due.take_if(|c| *c == "air").is_some();
        let mut open = true;
        let mut select = None;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Air balance"))
            .id(egui::Id::new("air.balance"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(320.0, 240.0))
            .default_size(crate::app::shell::at_least(Vec2::new(640.0, 480.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                if super::calcresultsui::write_button(ui) {
                    write_now = true;
                }
                if b.rooms.is_empty() {
                    ui.label(t("No rooms to calculate: place rooms first."));
                    return;
                }
                ui.label(
                    egui::RichText::new(format!(
                        "{}: {} m³/h · {} {} m³/h · {} {} m³/h",
                        t("Air handling unit"),
                        num(b.unit_flow(), 0),
                        t("supply needed"),
                        num(b.need_supply, 0),
                        t("exhaust needed"),
                        num(b.need_exhaust, 0),
                    ))
                    .strong(),
                );
                ui.label(format!(
                    "{}: {} {} m³/h · {} {} m³/h",
                    t("Through the terminals"),
                    t("supply"),
                    num(b.supply, 0),
                    t("exhaust"),
                    num(b.exhaust, 0),
                ));
                if b.stray > 0 {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        format!("{}: {}", t("Air terminals standing in no room"), b.stray),
                    );
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("air.balance.rooms")
                        .striped(true)
                        .num_columns(7)
                        .show(ui, |ui| {
                            for h in [
                                "Room", "V m³", "Air", "Needed m³/h", "Supply m³/h",
                                "Exhaust m³/h", "Terminals",
                            ] {
                                ui.strong(t(h));
                            }
                            ui.end_row();
                            for r in &b.rooms {
                                let name = format!("{} {}", r.number, r.name);
                                if ui.link(name.trim()).clicked() {
                                    select = Some(r.id);
                                }
                                ui.label(num(r.volume, 1));
                                ui.label(if r.exhaust_room {
                                    t("exhaust")
                                } else {
                                    t("supply")
                                });
                                if r.ok {
                                    ui.label(num(r.need, 0));
                                } else {
                                    ui.colored_label(ui.visuals().warn_fg_color, num(r.need, 0))
                                        .on_hover_text(t(
                                            "The room's terminals do not carry what it needs: place a diffuser (supply) or a grille (exhaust) in it, or raise their flow.",
                                        ));
                                }
                                ui.label(num(r.supply, 0));
                                ui.label(num(r.exhaust, 0));
                                ui.label(r.terminals.to_string());
                                ui.end_row();
                            }
                        });
                    ui.add_space(6.0);
                    ui.weak(t("A room needs its air change × volume or the least flow of its kind. Living rooms and offices are supplied; bathrooms, WCs, kitchens and stores are exhausted. A terminal with no flow of its own shares the room's need with the others; the flow is set in a diffuser's or a grille's properties."));
                    ui.weak("STN EN 16798-1 · STN EN 16798-3 · STN EN 15665");
                });
            });
        if let Some(id) = select {
            self.selection.only(id);
        }
        if write_now {
            self.write_results_said(creabim_services::calcresults::air(&b));
        }
        self.air_balance = open;
    }
}
