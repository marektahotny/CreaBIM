//! Forced extraction of single rooms ([`creabim_services::extractfans`]):
//! a window that designs a fan for the rooms picked (an extract valve in
//! the ceiling, Spiro Ø100 through the attic, the fan, a roof outlet)
//! and checks every fan drawn on the exhaust ducts: the air its room
//! needs against the point where the fan's curve meets the ducts', and
//! the undercut of the door the air comes in by; with the bill.

use super::CreaBimApp;
use creabim_core::{ElementId, Transaction};
use creabim_services::extractfans::{self as xf, Opts};
use eframe::egui::{self, Vec2};

/// What the window keeps while it is open.
#[derive(Clone, Debug, PartialEq)]
pub struct ExtractState {
    pub opts: Opts,
    /// The rooms picked for the design; none picks the bathrooms, WCs
    /// and plant rooms.
    pub rooms: Option<Vec<ElementId>>,
    pub fan: String,
    pub speed: String,
    /// One fan and one roof outlet for all the rooms picked, on one
    /// collecting duct (George 2026-11-25); else a fan each.
    pub central: bool,
}

impl Default for ExtractState {
    fn default() -> Self {
        Self {
            opts: Opts::default(),
            rooms: None,
            fan: xf::CENTRAL_FAN.into(),
            speed: String::new(),
            central: true,
        }
    }
}

impl CreaBimApp {
    /// `calc:extract` from the Ventilation tab: open the window.
    pub(super) fn extract_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" || arg != "extract" {
            return false;
        }
        if self.extract.is_none() {
            self.extract = Some(ExtractState::default());
        }
        true
    }

    /// The window, while it is open.
    pub(super) fn extract_window(&mut self, ctx: &egui::Context) {
        let Some(mut st) = self.extract.clone() else {
            return;
        };
        let t = crate::i18n::tr;
        let num = crate::i18n::num;
        let ho = self.heat_opts();
        let model = self.doc.model();
        let cands = xf::candidates(model);
        let picked = st.rooms.clone().unwrap_or_else(|| {
            cands
                .iter()
                .filter(|(id, _, _)| {
                    let Some(e) = model.get(*id) else {
                        return false;
                    };
                    let creabim_core::ElementKind::Room(r) = &e.kind else {
                        return false;
                    };
                    use creabim_core::roomtype::RoomType as R;
                    matches!(
                        R::of(&r.occupancy, &e.name),
                        R::Bathroom | R::Toilet | R::Utility | R::Laundry
                    )
                })
                .map(|c| c.0)
                .collect()
        });
        let rows = xf::check(model, &ho, &st.opts);
        let bill = xf::bill(model);
        let mut open = true;
        let mut select = None;
        let mut design = false;
        let mut picked_now = picked.clone();
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Forced extraction of rooms"))
            .id(egui::Id::new("vzt.extract"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(380.0, 300.0))
            .default_size(crate::app::shell::at_least(Vec2::new(780.0, 640.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(egui::RichText::new(t("Design")).strong());
                    ui.horizontal_wrapped(|ui| {
                        ui.label(t("Rooms"));
                        for (id, number, name) in &cands {
                            let mut on = picked_now.contains(id);
                            if ui.checkbox(&mut on, format!("{number} {name}")).changed() {
                                if on {
                                    picked_now.push(*id);
                                } else {
                                    picked_now.retain(|x| x != id);
                                }
                            }
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        ui.label(t("Fan"));
                        let shown = xf::fan(&st.fan).map_or(String::new(), |f| {
                            format!("{} {}", f.maker, f.name)
                        });
                        egui::ComboBox::from_id_salt("vzt.extract.fan")
                            .selected_text(shown)
                            .show_ui(ui, |ui| {
                                for f in xf::FANS {
                                    if ui
                                        .selectable_label(
                                            st.fan == f.id,
                                            format!("{} {}", f.maker, f.name),
                                        )
                                        .on_hover_text(f.about)
                                        .clicked()
                                    {
                                        st.fan = f.id.into();
                                        st.speed.clear();
                                    }
                                }
                            });
                        if let Some(f) = xf::fan(&st.fan).filter(|f| f.speeds.len() > 1) {
                            ui.label(t("Speed"));
                            for s in f.speeds {
                                let text = format!("{} ({} m³/h)", s.name, num(s.q_free, 0));
                                let on = st.speed == s.name
                                    || (st.speed.is_empty() && s.name == f.speeds[0].name);
                                if ui.selectable_label(on, text).clicked() {
                                    st.speed = s.name.into();
                                }
                            }
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .checkbox(&mut st.central, t("One fan for all the rooms picked"))
                            .on_hover_text(t("The valves join one collecting duct in the attic with one fan and one roof outlet; the valves are set to share the air by each room's need"))
                            .changed()
                        {
                            st.fan = if st.central {
                                xf::CENTRAL_FAN.into()
                            } else {
                                xf::FANS[0].id.into()
                            };
                            st.speed.clear();
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .button(t("Place in the model"))
                            .on_hover_text(t("An extract valve in the ceiling, Spiro Ø100 up into the attic, the fan, the duct on through the roof and a roof outlet, for every room picked; replaces what it placed before for those rooms"))
                            .clicked()
                        {
                            design = true;
                        }
                        ui.weak(t("Valve in the middle of the room's ceiling, up into the attic, 1.5 m on a level run with the fan towards the ridge, then out through the roof; move the valve or the duct as needed."));
                    });
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(t("Bends"));
                        ui.add(
                            egui::DragValue::new(&mut st.opts.zeta_bend)
                                .range(0.0..=2.0)
                                .speed(0.05)
                                .prefix("ζ "),
                        );
                        ui.label(t("Extract valve"));
                        ui.add(
                            egui::DragValue::new(&mut st.opts.zeta_valve)
                                .range(0.0..=50.0)
                                .speed(0.1)
                                .prefix("ζ "),
                        );
                        ui.label(t("Outlet"));
                        ui.add(
                            egui::DragValue::new(&mut st.opts.zeta_outlet)
                                .range(0.0..=10.0)
                                .speed(0.1)
                                .prefix("ζ "),
                        );
                        ui.label(t("air under the door"));
                        ui.add(
                            egui::DragValue::new(&mut st.opts.transfer_v)
                                .range(0.3..=3.0)
                                .speed(0.1)
                                .suffix(" m/s"),
                        );
                    });
                    ui.separator();
                    ui.label(egui::RichText::new(t("Fans drawn")).strong());
                    if rows.is_empty() {
                        ui.weak(t("No fan on the exhaust ducts yet: place the design, or put a Fan valve on an exhaust duct (VZO)."));
                    }
                    use crate::sheet::{self, Col};
                    let cols = [
                        Col::text("Room"),
                        Col::text("Fan"),
                        Col::num("Need", "m³/h"),
                        Col::num("L / bends", "m / ks"),
                        Col::num("Ducts at need", "Pa"),
                        Col::num("Fan at need", "Pa"),
                        Col::num("Working point", "m³/h · Pa"),
                        Col::num("Door undercut", "mm"),
                        Col::num("Valve throttles", "Pa"),
                    ];
                    sheet::show(ui, "vzt.extract.grid", &cols, rows.len(), 300.0, |row, i| {
                        let r = &rows[i];
                        let rooms: Vec<String> = r
                            .inlets
                            .iter()
                            .map(|x| {
                                x.room
                                    .as_ref()
                                    .map_or("–".into(), |(_, n, m)| format!("{n} {m}"))
                            })
                            .collect();
                        row.cell(|ui| {
                            if ui.link(rooms.join(", ")).clicked() {
                                select = Some(r.id);
                            }
                        });
                        row.text(if r.speed.name.is_empty() {
                            r.fan.name.to_string()
                        } else {
                            format!("{} ({})", r.fan.name, r.speed.name)
                        });
                        row.text(num(r.need, 0));
                        let (l, b) = r
                            .inlets
                            .iter()
                            .map(|x| (x.length, x.bends))
                            .fold((0.0, 0), |m, x| (f64::max(m.0, x.0), m.1.max(x.1)));
                        row.text(format!("{} / {b}", num(l, 1)));
                        row.text(num(r.dp_need, 1));
                        row.text(num(r.p_need, 1));
                        let wp = format!("{} · {}", num(r.q, 0), num(r.p, 1));
                        if r.ok() {
                            row.text(wp);
                        } else {
                            row.cell(|ui| {
                                ui.colored_label(ui.visuals().warn_fg_color, wp);
                            });
                        }
                        row.text(
                            r.inlets
                                .iter()
                                .filter_map(|x| x.undercut.map(|(g, _)| num(g.ceil(), 0)))
                                .collect::<Vec<_>>()
                                .join(", "),
                        );
                        row.text(
                            r.inlets
                                .iter()
                                .map(|x| num(x.throttle.max(0.0), 0))
                                .collect::<Vec<_>>()
                                .join(", "),
                        );
                    }, None);
                    let mut said: Vec<&String> = vec![];
                    for n in rows.iter().flat_map(|r| &r.notes) {
                        if !said.contains(&n) {
                            said.push(n);
                            ui.colored_label(ui.visuals().warn_fg_color, n);
                        }
                    }
                    ui.weak(t("The working point is where the fan's curve (maker's chart) meets the ducts' loss: Spiro friction (k 0.15 mm), bends, the valve and the outlet as the makers give them. Need: the room type's extract flow, or the flow typed on the valve. STN 73 4301 čl. 4.2.2.8 recommends forced extraction for bathrooms and WCs."));
                    ui.separator();
                    ui.label(egui::RichText::new(t("Bill")).strong());
                    let bcols = [
                        Col::text("Item"),
                        Col::num("Qty", ""),
                        Col::num("Unit price", "excl. VAT"),
                        Col::num("Total", "excl. VAT"),
                    ];
                    sheet::show(ui, "vzt.extract.bill", &bcols, bill.len(), 240.0, |row, i| {
                        let b = &bill[i];
                        row.cell(|ui| {
                            ui.label(&b.name).on_hover_text(&b.source);
                        });
                        row.text(format!("{} {}", num(b.qty, if b.unit == "m" { 1 } else { 0 }), b.unit));
                        row.text(b.price.map_or(t("on request"), |(p, c)| format!("{} {c}", num(p, 2))));
                        row.text(
                            b.price
                                .map_or("–".into(), |(p, c)| format!("{} {c}", num(p * b.qty, 2))),
                        );
                    }, None);
                });
            });
        if picked_now != picked {
            st.rooms = Some(picked_now.clone());
        }
        if let Some(id) = select {
            self.selection.only(id);
        }
        if design {
            let speed = (!st.speed.is_empty()).then_some(st.speed.as_str());
            let (add, remove, warn) = if st.central {
                xf::propose_central(self.doc.model(), &ho, &picked_now, &st.fan, speed)
            } else {
                xf::propose(self.doc.model(), &picked_now, &st.fan, speed)
            };
            let mut tx = Transaction::new("Design forced extraction");
            tx = tx.remove_all_cascade(self.doc.model(), &remove);
            let n = add
                .iter()
                .filter(|e| e.name.contains(" – ventil\u{e1}tor"))
                .count();
            for e in add {
                tx = tx.insert(e);
            }
            match self.doc.commit(tx) {
                Ok(_) => {
                    // the new devices' position marks, as tags
                    self.tag_trade_rooms();
                    self.status = format!("{}: {n}", t("Extract fans placed"));
                    if !warn.is_empty() {
                        self.status = format!("{} · {}", self.status, warn.join(" "));
                    }
                }
                Err(e) => self.status = format!("Couldn't do that: {e}"),
            }
        }
        self.extract = if open { Some(st) } else { None };
    }
}
