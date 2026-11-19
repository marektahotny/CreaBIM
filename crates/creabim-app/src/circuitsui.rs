//! The circuits of the electrical installation
//! ([`creabim_services::circuits`], after PilonDesigner): a window that
//! lists the model's circuits with their power, design current, breaker,
//! cable, phase and check, lets each device's installed power be typed,
//! proposes circuits for devices that have none, spreads the circuits
//! over the phases and sums the building's power.

use super::CreaBimApp;
use creabim_core::electrical as core_el;
use creabim_core::{Element, ElementId, ElementKind, Transaction};
use creabim_services::circuits;
use creabim_services::electrical::{Method, BREAKER_IN, IZ_S};
use eframe::egui::{self, Vec2};

/// The window's state while it is open.
pub(super) struct CircuitsWindow {
    /// The simultaneity β the designer takes, and the most sockets a
    /// proposed circuit gets.
    beta: f64,
    max_sockets: usize,
    /// The circuit whose devices are shown.
    open: Option<ElementId>,
}

impl Default for CircuitsWindow {
    fn default() -> Self {
        Self {
            beta: 1.0,
            max_sockets: 10,
            open: None,
        }
    }
}

/// An issue of a circuit in the application's language: its words
/// before the figures.
fn issue(text: &str) -> String {
    let t = crate::i18n::tr;
    for head in [
        "without power",
        "voltage drop",
        "without a board",
        "give the breaker In",
        "give the cable's cross-section",
        "a 30 mA RCD is missing",
    ] {
        if let Some(rest) = text.strip_prefix(head) {
            return format!("{}{rest}", t(head));
        }
    }
    text.to_string()
}

/// What a click in the window asks for.
enum Ask {
    Propose,
    Phases,
    New,
    Delete(ElementId),
    Edit(ElementId, &'static str, Box<core_el::Circuit>),
    Rename(ElementId, String),
    AddSelected(ElementId),
}

impl CreaBimApp {
    /// `calc:circuits` from the Electrical tab: open the window.
    pub(super) fn circuits_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" || arg != "circuits" {
            return false;
        }
        if self.circuits.is_none() {
            self.circuits = Some(CircuitsWindow::default());
        }
        self.results_due = Some("circuits");
        true
    }

    /// The window, while it is open.
    pub(super) fn circuits_window(&mut self, ctx: &egui::Context) {
        let Some(mut w) = self.circuits.take() else {
            return;
        };
        let t = crate::i18n::tr;
        let num = crate::i18n::num;
        let method = Method::A2;
        let model = self.doc.model();
        let results = circuits::results(model, method);
        let balance = circuits::balance(&results, w.beta);
        let list: Vec<(ElementId, String, core_el::Circuit)> = core_el::circuits(model)
            .into_iter()
            .map(|(e, c)| (e.id, e.name.clone(), c.clone()))
            .collect();
        let boards: Vec<(ElementId, String)> = core_el::devices(model)
            .into_iter()
            .filter(|d| d.2 == core_el::DeviceKind::Board)
            .map(|d| (d.0.id, d.0.name.clone()))
            .collect();
        let device_name = |id: ElementId| {
            model.get(id).map_or_else(
                || t("(deleted)"),
                |e| match core_el::device_kind(model, e) {
                    Some(k) => format!("{} · {}", e.name, t(k.label())),
                    None => e.name.clone(),
                },
            )
        };
        let unassigned = core_el::devices(model)
            .iter()
            .filter(|d| d.2.is_load() && core_el::circuit_of(model, d.0.id).is_none())
            .count();
        let selected_devices: Vec<ElementId> = self
            .selection
            .all()
            .into_iter()
            .filter(|id| {
                model
                    .get(*id)
                    .is_some_and(|e| core_el::device_kind(model, e).is_some())
            })
            .collect();
        let mut ask: Option<Ask> = None;
        let mut write_now = self.results_due.take_if(|c| *c == "circuits").is_some();
        let mut open = true;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Circuits"))
            .id(egui::Id::new("circuits.list"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(430.0, 280.0))
            .default_size(Vec2::new(860.0, 560.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                if super::calcresultsui::write_button(ui) {
                    write_now = true;
                }
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(t("Propose circuits"))
                        .on_hover_text(t("Circuits for the devices that have none: a level's lights together, sockets by rooms, each fixed appliance its own; the smallest breaker and cable, a 30 mA RCD where the standard asks for one"))
                        .clicked()
                    {
                        ask = Some(Ask::Propose);
                    }
                    ui.label(t("Sockets to a circuit"));
                    ui.add(egui::DragValue::new(&mut w.max_sockets).range(1..=20));
                    ui.separator();
                    if ui
                        .button(t("Spread over the phases"))
                        .on_hover_text(t("Give single-phase circuits without a phase one of L1 to L3, the largest first onto the phase loaded least"))
                        .clicked()
                    {
                        ask = Some(Ask::Phases);
                    }
                    if ui.button(t("New circuit")).clicked() {
                        ask = Some(Ask::New);
                    }
                });
                ui.weak(format!(
                    "{}: {} · {}: {}",
                    t("Distribution boards"),
                    boards.len(),
                    t("Devices without a circuit"),
                    unassigned
                ));
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height(ui.available_height() - 110.0)
                    .show(ui, |ui| {
                        egui::Grid::new("circuits.grid")
                            .num_columns(11)
                            .striped(true)
                            .spacing([10.0, 6.0])
                            .show(ui, |ui| {
                                for h in [
                                    "Circuit", "Devices", "P [W]", "IB [A]", "Breaker",
                                    "Cable", "Phase", "L [m]", "Δu [%]", "RCD", "",
                                ] {
                                    ui.label(egui::RichText::new(t(h)).strong());
                                }
                                ui.end_row();
                                for ((id, name, c), r) in list.iter().zip(&results) {
                                    let mut c2 = c.clone();
                                    let mut label = "";
                                    let mut n = name.clone();
                                    if ui
                                        .add(egui::TextEdit::singleline(&mut n).desired_width(60.0))
                                        .changed()
                                    {
                                        ask = Some(Ask::Rename(*id, n));
                                    }
                                    let shown = w.open == Some(*id);
                                    if ui
                                        .selectable_label(shown, format!("{} ▾", r.devices))
                                        .on_hover_text(t("Show its devices and their power"))
                                        .clicked()
                                    {
                                        w.open = (!shown).then_some(*id);
                                    }
                                    ui.label(num(r.power_w, 0));
                                    ui.label(num(r.ib, 1));
                                    egui::ComboBox::from_id_salt(("circuits.in", *id))
                                        .width(64.0)
                                        .selected_text(
                                            c.breaker_a
                                                .map_or("—".to_string(), |a| format!("B{a:.0}")),
                                        )
                                        .show_ui(ui, |ui| {
                                            for a in BREAKER_IN {
                                                if ui
                                                    .selectable_value(
                                                        &mut c2.breaker_a,
                                                        Some(a),
                                                        format!("B{a:.0}"),
                                                    )
                                                    .changed()
                                                {
                                                    label = "Circuit breaker";
                                                }
                                            }
                                        });
                                    egui::ComboBox::from_id_salt(("circuits.s", *id))
                                        .width(120.0)
                                        .selected_text(c.cable().unwrap_or_else(|| "—".into()))
                                        .show_ui(ui, |ui| {
                                            for s in IZ_S {
                                                let mut k = c2.clone();
                                                k.cable_mm2 = Some(s);
                                                if ui
                                                    .selectable_label(
                                                        c.cable_mm2 == Some(s),
                                                        k.cable().unwrap_or_default(),
                                                    )
                                                    .clicked()
                                                {
                                                    c2.cable_mm2 = Some(s);
                                                    label = "Circuit cable";
                                                }
                                            }
                                        });
                                    ui.horizontal(|ui| {
                                        if ui
                                            .selectable_label(c.three_phase, "3f")
                                            .on_hover_text("3 × 400 V")
                                            .clicked()
                                        {
                                            c2.three_phase = !c.three_phase;
                                            label = "Circuit phases";
                                        }
                                        if !c.three_phase {
                                            for p in 1..=3u8 {
                                                if ui
                                                    .selectable_label(
                                                        c.phase == Some(p),
                                                        format!("L{p}"),
                                                    )
                                                    .clicked()
                                                {
                                                    c2.phase =
                                                        (c.phase != Some(p)).then_some(p);
                                                    label = "Circuit phase";
                                                }
                                            }
                                        }
                                    });
                                    let mut len = r.length;
                                    if ui
                                        .add(
                                            egui::DragValue::new(&mut len)
                                                .range(0.0..=1000.0)
                                                .speed(0.5)
                                                .max_decimals(1),
                                        )
                                        .on_hover_text(t(if r.measured {
                                            "Measured square from the board to the farthest device; type over it to give your own"
                                        } else {
                                            "Given by hand; set 0 to measure it again"
                                        }))
                                        .changed()
                                    {
                                        c2.length = (len > 0.0).then_some(len);
                                        label = "Circuit length";
                                    }
                                    match &r.check {
                                        Some(k) if k.ok_du => ui.label(num(k.du, 2)),
                                        Some(k) => ui.colored_label(
                                            ui.visuals().warn_fg_color,
                                            num(k.du, 2),
                                        ),
                                        None => ui.weak("—"),
                                    };
                                    let mut rcd = c.rcd_ma.is_some();
                                    if ui
                                        .checkbox(&mut rcd, "30 mA")
                                        .on_hover_text(match r.rcd_required {
                                            Some(why) => format!("{}: {}", t("Asked for"), t(why)),
                                            None => t("Not asked for by the standard"),
                                        })
                                        .changed()
                                    {
                                        c2.rcd_ma = rcd.then_some(30.0);
                                        label = "Circuit RCD";
                                    }
                                    ui.horizontal(|ui| {
                                        if r.ok() {
                                            ui.label("✔");
                                        } else {
                                            ui.colored_label(
                                                ui.visuals().warn_fg_color,
                                                format!(
                                                    "✖ {}",
                                                    r.issues
                                                        .iter()
                                                        .map(|i| issue(i))
                                                        .collect::<Vec<_>>()
                                                        .join("; ")
                                                ),
                                            );
                                        }
                                        if ui
                                            .small_button("🗑")
                                            .on_hover_text(t("Delete the circuit"))
                                            .clicked()
                                        {
                                            ask = Some(Ask::Delete(*id));
                                        }
                                    });
                                    ui.end_row();
                                    if !label.is_empty() {
                                        ask = Some(Ask::Edit(*id, label, Box::new(c2)));
                                    }
                                    if shown {
                                        ui.label("");
                                        ui.vertical(|ui| {
                                            // its board
                                            ui.horizontal(|ui| {
                                                ui.label(t("Distribution board"));
                                                let cur = c
                                                    .board
                                                    .and_then(|b| {
                                                        boards.iter().find(|x| x.0 == b)
                                                    })
                                                    .map_or("—".to_string(), |b| b.1.clone());
                                                egui::ComboBox::from_id_salt(("circuits.b", *id))
                                                    .selected_text(cur)
                                                    .show_ui(ui, |ui| {
                                                        for (b, n) in &boards {
                                                            if ui
                                                                .selectable_label(
                                                                    c.board == Some(*b),
                                                                    n,
                                                                )
                                                                .clicked()
                                                            {
                                                                let mut k = c.clone();
                                                                k.board = Some(*b);
                                                                ask = Some(Ask::Edit(
                                                                    *id,
                                                                    "Circuit board",
                                                                    Box::new(k),
                                                                ));
                                                            }
                                                        }
                                                    });
                                                let mut light = c.lighting;
                                                if ui.checkbox(&mut light, t("Lighting")).changed()
                                                {
                                                    let mut k = c.clone();
                                                    k.lighting = light;
                                                    ask = Some(Ask::Edit(
                                                        *id,
                                                        "Circuit kind",
                                                        Box::new(k),
                                                    ));
                                                }
                                            });
                                            for (i, l) in c.loads.iter().enumerate() {
                                                ui.horizontal(|ui| {
                                                    let mut p = l.power_w;
                                                    if ui
                                                        .add(
                                                            egui::DragValue::new(&mut p)
                                                                .range(0.0..=100_000.0)
                                                                .speed(10.0)
                                                                .suffix(" W"),
                                                        )
                                                        .changed()
                                                    {
                                                        let mut k = c.clone();
                                                        k.loads[i].power_w = p;
                                                        ask = Some(Ask::Edit(
                                                            *id,
                                                            "Device power",
                                                            Box::new(k),
                                                        ));
                                                    }
                                                    ui.label(device_name(l.device));
                                                    if ui
                                                        .small_button("✖")
                                                        .on_hover_text(t(
                                                            "Take the device off the circuit",
                                                        ))
                                                        .clicked()
                                                    {
                                                        let mut k = c.clone();
                                                        k.loads.remove(i);
                                                        ask = Some(Ask::Edit(
                                                            *id,
                                                            "Circuit devices",
                                                            Box::new(k),
                                                        ));
                                                    }
                                                });
                                            }
                                            if !selected_devices.is_empty()
                                                && ui
                                                    .button(format!(
                                                        "{} ({})",
                                                        t("Add the selected devices"),
                                                        selected_devices.len()
                                                    ))
                                                    .clicked()
                                            {
                                                ask = Some(Ask::AddSelected(*id));
                                            }
                                        });
                                        ui.end_row();
                                    }
                                }
                            });
                        if list.is_empty() {
                            ui.weak(t("No circuits yet. Place sockets, lights and a distribution board from the catalogue, then propose the circuits."));
                        }
                    });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(t("Power balance")).strong());
                    ui.label("β");
                    ui.add(
                        egui::DragValue::new(&mut w.beta)
                            .range(0.05..=1.0)
                            .speed(0.01)
                            .max_decimals(2),
                    )
                    .on_hover_text(t("The simultaneity the designer takes: Pp = β · Pi"));
                });
                ui.label(format!(
                    "Pi = {} kW · Pp = {} kW · Ip = {} A{}",
                    num(balance.pi / 1000.0, 2),
                    num(balance.pp / 1000.0, 2),
                    num(balance.ip, 1),
                    balance.main_breaker.map_or(String::new(), |a| format!(
                        " · {}: {} A",
                        t("main breaker at least"),
                        num(a, 0)
                    )),
                ));
                ui.label(format!(
                    "L1 {} W / {} A · L2 {} W / {} A · L3 {} W / {} A{}{}",
                    num(balance.phase_p[0], 0),
                    num(balance.phase_i[0], 1),
                    num(balance.phase_p[1], 0),
                    num(balance.phase_i[1], 1),
                    num(balance.phase_p[2], 0),
                    num(balance.phase_i[2], 1),
                    balance.unbalance.map_or(String::new(), |u| format!(
                        " · {} {} %",
                        t("unbalance"),
                        num(u, 0)
                    )),
                    if balance.unphased > 0 {
                        format!(" · {}: {}", t("circuits without a phase"), balance.unphased)
                    } else {
                        String::new()
                    },
                ));
            });
        if write_now {
            self.write_results_said(creabim_services::calcresults::circuits(&results));
        }
        if let Some(a) = ask {
            self.circuits_apply(a, &w, method);
        }
        if open {
            self.circuits = Some(w);
        }
    }

    fn circuits_apply(&mut self, ask: Ask, w: &CircuitsWindow, method: Method) {
        let t = crate::i18n::tr;
        let set = |e: &mut Element, c: core_el::Circuit| {
            if let ElementKind::Circuit(x) = &mut e.kind {
                *x = c;
            }
        };
        let done = match ask {
            Ask::Propose => match circuits::propose(self.doc.model(), w.max_sockets, method) {
                Ok(new) => {
                    let n = new.len();
                    let mut tx = Transaction::new("Propose circuits");
                    for e in new {
                        tx = tx.insert(e);
                    }
                    let r = self.doc.commit(tx);
                    if r.is_ok() {
                        self.status = format!("{}: {n}", t("Circuits proposed"));
                    }
                    r
                }
                Err(why) => {
                    self.status = t(&why);
                    Ok(())
                }
            },
            Ask::Phases => {
                let results = circuits::results(self.doc.model(), method);
                let mut tx = Transaction::new("Spread over the phases");
                for (id, p) in circuits::balance_phases(&results) {
                    if let Some(before) = self.doc.model().get(id).cloned() {
                        let mut after = before.clone();
                        if let ElementKind::Circuit(c) = &mut after.kind {
                            c.phase = Some(p);
                        }
                        tx = tx.replace(before, after);
                    }
                }
                self.doc.commit(tx)
            }
            Ask::New => {
                let n = core_el::circuits(self.doc.model()).len() + 1;
                let board = core_el::devices(self.doc.model())
                    .into_iter()
                    .find(|d| d.2 == core_el::DeviceKind::Board)
                    .map(|d| d.0.id);
                let e = Element::new(
                    format!("{n}"),
                    ElementKind::Circuit(core_el::Circuit {
                        board,
                        breaker_a: Some(16.0),
                        cable_mm2: Some(2.5),
                        ..Default::default()
                    }),
                );
                self.doc.commit(Transaction::new("New circuit").insert(e))
            }
            Ask::Delete(id) => {
                let tx = Transaction::new("Delete circuit").remove_cascade(self.doc.model(), id);
                self.doc.commit(tx)
            }
            Ask::Edit(id, label, c) => self.doc.edit_merged(id, label, |e| set(e, *c)),
            Ask::Rename(id, name) => self
                .doc
                .edit_merged(id, "Rename circuit", |e| e.name = name),
            Ask::AddSelected(id) => {
                let model = self.doc.model();
                let add: Vec<ElementId> = self
                    .selection
                    .all()
                    .into_iter()
                    .filter(|d| {
                        model
                            .get(*d)
                            .is_some_and(|e| core_el::device_kind(model, e).is_some())
                            && core_el::circuit_of(model, *d).is_none()
                    })
                    .collect();
                self.doc.edit(id, "Circuit devices", |e| {
                    if let ElementKind::Circuit(c) = &mut e.kind {
                        c.loads.extend(add.into_iter().map(|device| core_el::Load {
                            device,
                            power_w: 0.0,
                        }));
                    }
                })
            }
        };
        if let Err(e) = done {
            self.status = format!("{e}");
        }
    }
}
