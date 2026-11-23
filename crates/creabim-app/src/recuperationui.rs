//! Ventilation with heat recovery designed in one step
//! ([`creabim_services::recuperation`]): the window shows what every
//! room gets, the unit, the ducts' pressure loss and the transfer under
//! the doors; a room is switched to its own wall units here; "Place in
//! the model" draws the unit, boxes, ducts and plate valves (replacing
//! what it drew before), and the bill with prices is read back from the
//! model, so hand edits are priced as drawn.

use super::CreaBimApp;
use creabim_core::{ElementKind, Transaction};
use creabim_services::heatloss;
use creabim_services::recuperation::{self as rec, Opts, Role, Route};
use eframe::egui::{self, Vec2};

impl CreaBimApp {
    /// `calc:recuperation` from the Ventilation tab: open the window.
    pub(super) fn recuperation_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" || arg != "recuperation" {
            return false;
        }
        if self.recuperation.is_none() {
            self.recuperation = Some(Opts::default());
        }
        true
    }

    /// The window, while it is open.
    pub(super) fn recuperation_window(&mut self, ctx: &egui::Context) {
        let Some(mut o) = self.recuperation.clone() else {
            return;
        };
        let t = crate::i18n::tr;
        let num = crate::i18n::num;
        let ho = self
            .heat_loss
            .as_ref()
            .map_or_else(heatloss::Opts::default, |x| x.0);
        let d = rec::design(self.doc.model(), &ho, &o);
        let bill = rec::bill(self.doc.model());
        let cat = rec::catalogue();
        let mut open = true;
        let mut select = None;
        let mut toggle: Option<creabim_core::ElementId> = None;
        let mut place = false;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Heat recovery ventilation"))
            .id(egui::Id::new("vzt.recuperation"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(380.0, 300.0))
            .default_size(crate::app::shell::at_least(Vec2::new(760.0, 680.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(t("Ducts laid"));
                    ui.selectable_value(&mut o.route, Route::Floor, t("in the floor"));
                    ui.selectable_value(&mut o.route, Route::Ceiling, t("above the ceiling"));
                    ui.separator();
                    ui.label(t("Unit"));
                    ui.selectable_value(&mut o.series, "Pro-V".to_string(), t("at a wall (Pro-V)"));
                    ui.selectable_value(&mut o.series, "Pro".to_string(), t("under the ceiling (Pro)"));
                    ui.selectable_value(&mut o.series, "Easy2".to_string(), t("under the ceiling, compact (Easy2)"));
                    ui.selectable_value(&mut o.series, "Slim".to_string(), t("at a wall, slim (Slim)"));
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(t("Most air in one duct Ø75"));
                    ui.add(
                        egui::DragValue::new(&mut o.per_duct)
                            .range(10.0..=50.0)
                            .speed(1.0)
                            .suffix(" m³/h"),
                    );
                    ui.label(t("unit run at most at"));
                    let mut pc = o.use_of_unit * 100.0;
                    if ui
                        .add(egui::DragValue::new(&mut pc).range(40.0..=100.0).suffix(" %"))
                        .changed()
                    {
                        o.use_of_unit = pc / 100.0;
                    }
                    ui.label(t("Room unit"));
                    egui::ComboBox::from_id_salt("vzt.rec.roomunit")
                        .selected_text(
                            cat.room_units
                                .get(o.room_unit)
                                .map_or(String::new(), |u| u.name.clone()),
                        )
                        .show_ui(ui, |ui| {
                            for (i, u) in cat.room_units.iter().enumerate() {
                                ui.selectable_value(&mut o.room_unit, i, &u.name)
                                    .on_hover_text(&u.note);
                            }
                        });
                });
                ui.separator();
                let unit = d.unit.map(|i| &cat.units[i]);
                ui.label(
                    egui::RichText::new(format!(
                        "{}: {} · {} {} m³/h ({} {} · {} {} m³/h)",
                        t("Unit"),
                        unit.map_or("–".to_string(), |u| u.name.clone()),
                        t("balanced flow"),
                        num(d.flow, 0),
                        t("supply needed"),
                        num(d.need_supply, 0),
                        t("extract needed"),
                        num(d.need_extract, 0),
                    ))
                    .strong(),
                );
                if !d.branches.is_empty() {
                    ui.label(format!(
                        "{}: {} {} Pa · {} {} Pa · {} {} · GP Ø75 {} m",
                        t("Pressure loss of the ducts"),
                        t("supply"),
                        num(d.dp_supply, 0),
                        t("extract"),
                        num(d.dp_extract, 0),
                        t("ducts"),
                        d.branches.len(),
                        num(d.branches.iter().map(|b| b.length).sum::<f64>(), 1),
                    ));
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    use crate::sheet::{self, Col};
                    let cols = [
                        Col::text("Room"),
                        Col::text("Air"),
                        Col::num("Needed", "m³/h"),
                        Col::num("Designed", "m³/h"),
                        Col::num("Ducts Ø75", "pcs"),
                        Col::num("Terminals", "pcs"),
                        Col::text("Decentral"),
                    ];
                    let total = |row: &mut sheet::Row| {
                        row.text(t("Total"));
                        row.empty();
                        row.text(num(d.rooms.iter().map(|r| r.need).sum::<f64>(), 0));
                        row.text(num(d.rooms.iter().map(|r| r.design).sum::<f64>(), 0));
                        row.text(d.rooms.iter().map(|r| r.ducts as usize).sum::<usize>().to_string());
                        row.text(d.rooms.iter().map(|r| r.terminals.len()).sum::<usize>().to_string());
                    };
                    sheet::show(ui, "vzt.rec.rooms", &cols, d.rooms.len(), 400.0, |row, i| {
                        let r = &d.rooms[i];
                        let name = format!("{} {}", r.number, r.name);
                        row.cell(|ui| {
                            if ui.link(name.trim()).clicked() {
                                select = Some(r.id);
                            }
                        });
                        row.text(t(match r.role {
                            Role::Supply => "supply",
                            Role::Extract => "extract",
                            Role::Transfer => "transfer",
                            Role::Decentral => "room units",
                            Role::Out => "left out",
                        }));
                        row.text(num(r.need, 0));
                        row.text(num(r.design, 0));
                        row.text(r.ducts.to_string());
                        row.text(r.terminals.len().to_string());
                        row.cell(|ui| {
                            if r.role != Role::Out {
                                let mut on = r.role == Role::Decentral;
                                if ui.checkbox(&mut on, "").changed() {
                                    toggle = Some(r.id);
                                }
                            }
                        });
                    }, Some(&total));
                    if !d.room_units.is_empty() {
                        ui.add_space(4.0);
                        if let Some(ru) = cat.room_units.get(o.room_unit) {
                            for r in &d.room_units {
                                let room = &d.rooms[r.room];
                                ui.label(format!(
                                    "{} {}: {} × {} ({} {} m³/h)",
                                    room.number,
                                    room.name,
                                    r.places.len(),
                                    ru.name,
                                    t("with heat recovery up to"),
                                    num(ru.q_hr * r.places.len() as f64, 0)
                                ));
                            }
                        }
                    }
                    ui.add_space(6.0);
                    egui::CollapsingHeader::new(t("Ducts: length, velocity, pressure loss, throttling"))
                        .id_salt("vzt.rec.branches")
                        .show(ui, |ui| {
                            let cols = [
                                Col::text("Room"),
                                Col::text("Air"),
                                Col::num("L", "m"),
                                Col::num("Flow", "m³/h"),
                                Col::num("v", "m/s"),
                                Col::num("Δp", "Pa"),
                                Col::num("Throttle", "Pa"),
                            ];
                            let total = |row: &mut sheet::Row| {
                                row.text(t("Total"));
                                row.empty();
                                row.text(num(d.branches.iter().map(|b| b.length).sum::<f64>(), 1));
                            };
                            sheet::show(ui, "vzt.rec.br", &cols, d.branches.len(), 400.0, |row, i| {
                                let b = &d.branches[i];
                                let r = &d.rooms[b.room];
                                row.text(format!("{} {}", r.number, r.name));
                                row.text(t(if b.supply { "supply" } else { "extract" }));
                                row.text(num(b.length, 1));
                                row.text(num(b.flow, 1));
                                row.text(num(b.velocity, 2));
                                row.text(num(b.dp, 1));
                                row.text(num(b.throttle, 1));
                            }, Some(&total));
                        });
                    if !d.transfers.is_empty() {
                        egui::CollapsingHeader::new(t("Transfer air under the doors"))
                            .id_salt("vzt.rec.transfer")
                            .show(ui, |ui| {
                                for tr in &d.transfers {
                                    let name = self
                                        .doc
                                        .model()
                                        .get(tr.door)
                                        .map(|e| e.mark.clone().unwrap_or_else(|| e.name.clone()))
                                        .unwrap_or_default();
                                    if ui
                                        .link(format!(
                                            "{name}: {} m³/h · {} {} mm",
                                            num(tr.flow, 0),
                                            t("undercut"),
                                            num(tr.gap_mm, 0)
                                        ))
                                        .clicked()
                                    {
                                        select = Some(tr.door);
                                    }
                                }
                            });
                    }
                    if !d.noise.is_empty() {
                        egui::CollapsingHeader::new(t("Noise in the rooms (night limit governs)"))
                            .id_salt("vzt.rec.noise")
                            .default_open(true)
                            .show(ui, |ui| {
                                let cols = [
                                    Col::text("Room"),
                                    Col::text("Category"),
                                    Col::num("LAmax,p day / evening / night", "dB"),
                                    Col::num("Level", "dB(A)"),
                                    Col::text(""),
                                ];
                                sheet::show(ui, "vzt.rec.noiseg", &cols, d.noise.len(), 400.0, |row, i| {
                                    let n = &d.noise[i];
                                    let r = &d.rooms[n.room];
                                    row.text(format!("{} {}", r.number, r.name));
                                    row.text(n.category.to_string());
                                    row.text(format!(
                                        "{} / {} / {}",
                                        num(n.limits[0], 0),
                                        num(n.limits[1], 0),
                                        num(n.limits[2], 0)
                                    ));
                                    row.text(n.level.map_or("–".into(), |l| num(l, 0)));
                                    row.cell(|ui| {
                                        match n.ok {
                                            Some(true) => ui.label(format!("✔ {}", n.note)),
                                            Some(false) => ui.colored_label(
                                                ui.visuals().error_fg_color,
                                                format!("✘ {}", n.note),
                                            ),
                                            None => ui.colored_label(ui.visuals().warn_fg_color, &n.note),
                                        };
                                    });
                                }, None);
                                ui.weak(rec::NOISE_SOURCE);
                            });
                    }
                    for w in &d.warnings {
                        ui.colored_label(ui.visuals().warn_fg_color, w);
                    }
                    ui.add_space(6.0);
                    if ui
                        .button(t("Place in the model"))
                        .on_hover_text(t("Draws the unit, the distribution boxes, the flexible ducts with their plate valves and the room units; what an earlier design drew is replaced, anything drawn by hand stays. Edit them afterwards as any duct or object."))
                        .clicked()
                    {
                        place = true;
                    }
                    ui.add_space(6.0);
                    egui::CollapsingHeader::new(t("Take-off VZT – recuperation (as drawn)"))
                        .id_salt("vzt.rec.bill")
                        .default_open(true)
                        .show(ui, |ui| {
                            if bill.lines.is_empty() {
                                ui.weak(t("Nothing placed yet."));
                                return;
                            }
                            egui::Grid::new("vzt.rec.billg").striped(true).show(ui, |ui| {
                                for h in ["Group", "Code", "Item", "Qty", "Unit of measure", "€/unit", "€"] {
                                    ui.strong(t(h));
                                }
                                ui.end_row();
                                for l in &bill.lines {
                                    ui.label(l.group);
                                    ui.label(&l.code);
                                    ui.label(&l.name).on_hover_text(&l.source);
                                    ui.label(num(l.qty, if l.qty.fract() == 0.0 { 0 } else { 1 }));
                                    ui.label(&l.unit);
                                    match (l.price, l.total()) {
                                        (Some(p), Some(s)) => {
                                            ui.label(num(p, 2));
                                            ui.label(num(s, 2));
                                        }
                                        _ => {
                                            ui.colored_label(ui.visuals().warn_fg_color, t("on request"));
                                            ui.label("");
                                        }
                                    }
                                    ui.end_row();
                                }
                            });
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} {} € · {} {} € · {} {} €",
                                    t("Without VAT"),
                                    num(bill.net, 2),
                                    t("VAT"),
                                    num(bill.vat, 2),
                                    t("with VAT"),
                                    num(bill.gross, 2)
                                ))
                                .strong(),
                            );
                            if bill.unpriced > 0 {
                                ui.colored_label(
                                    ui.visuals().warn_fg_color,
                                    format!("{}: {}", t("Items without a price (not in the total)"), bill.unpriced),
                                );
                            }
                            ui.weak(&cat.note);
                        });
                });
            });
        // for checking by eye: placed once the window is up
        if std::env::var("CREABIM_SHOT_RECUP_PLACE").is_ok() && ctx.cumulative_frame_nr() == 6 {
            place = true;
        }
        if let Some(id) = select {
            self.selection.only(id);
        }
        if let Some(id) = toggle {
            if let Some(before) = self.doc.model().get(id).cloned() {
                let mut after = before.clone();
                if let ElementKind::Room(r) = &mut after.kind {
                    r.decentral_hrv = !r.decentral_hrv;
                }
                if let Err(e) = self
                    .doc
                    .commit(Transaction::new("Decentral recuperation").replace(before, after))
                {
                    self.status = format!("Couldn't do that: {e}");
                }
            }
        }
        if place {
            let mut ok = true;
            for c in rec::objects(&d, &o) {
                let (tx, _) = creabim_core::objects::add_to_model(self.doc.model(), &c);
                if let Err(e) = self.doc.commit(tx) {
                    self.status = format!("Couldn't do that: {e}");
                    ok = false;
                }
            }
            if ok {
                let tx = rec::apply(self.doc.model(), &d, &o);
                match self.doc.commit(tx) {
                    Ok(_) => {
                        self.status = format!(
                            "{}: {} {} · {} m³/h",
                            t("Recuperation placed"),
                            d.branches.len(),
                            t("ducts"),
                            num(d.flow, 0)
                        )
                    }
                    Err(e) => self.status = format!("Couldn't do that: {e}"),
                }
            }
        }
        let open = open
            && !(place && std::env::var("CREABIM_SHOT_RECUP_PLACE").is_ok_and(|v| v == "close"));
        self.recuperation = if open { Some(o) } else { None };
    }
}
