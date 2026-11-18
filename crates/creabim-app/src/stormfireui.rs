//! The storm water (STN EN 12056-3) and fire main (STN 92 0400)
//! calculations of what is drawn ([`creabim_services::storm`],
//! [`creabim_services::firewater`]): a window with each pipe's flow and
//! proposed size, and a button that takes the proposals into the model.

use super::CreaBimApp;
use creabim_core::{ElementId, ElementKind, Model, Transaction};
use creabim_services::{firewater, storm};
use eframe::egui::{self, Vec2};

/// The calculation shown.
pub(super) enum NetCalc {
    Storm(Option<storm::StormResult>),
    Fire(Option<firewater::FireResult>),
}

impl NetCalc {
    fn storm(model: &Model) -> Self {
        NetCalc::Storm(storm::run(model, "D"))
    }
    fn fire(model: &Model) -> Self {
        NetCalc::Fire(firewater::run(model, "PV"))
    }
    /// Each pipe and the size proposed for it.
    fn proposals(&self) -> Vec<(ElementId, f64)> {
        match self {
            NetCalc::Storm(Some(r)) => r.pipes.iter().map(|p| (p.id, p.proposed_mm)).collect(),
            NetCalc::Fire(Some(r)) => r.pipes.iter().map(|p| (p.id, p.proposed_mm)).collect(),
            _ => vec![],
        }
    }
}

/// The pipes whose proposed size is not the one they have.
fn changes(model: &Model, sizes: &[(ElementId, f64)]) -> Vec<(ElementId, f64)> {
    sizes
        .iter()
        .copied()
        .filter(|(id, n)| {
            matches!(model.get(*id).map(|e| &e.kind),
                Some(ElementKind::Pipe(p)) if (p.nominal_mm - n).abs() > 1e-6)
        })
        .collect()
}

/// The edits that give every pipe its proposed size.
fn apply(model: &Model, sizes: &[(ElementId, f64)]) -> Option<Transaction> {
    let mut tx = Transaction::new("Apply proposed pipe sizes");
    let mut any = false;
    for (id, n) in changes(model, sizes) {
        let Some(e) = model.get(id) else { continue };
        let ElementKind::Pipe(p) = &e.kind else {
            continue;
        };
        let Some(t) = creabim_services::pipe_types::by_id(&p.pipe_type) else {
            continue;
        };
        let mut after = e.clone();
        if let ElementKind::Pipe(q) = &mut after.kind {
            crate::pipesui::set_size(q, t, n);
            after.name = format!("{} {}", q.system, crate::pipesui::size_label(q));
        }
        tx = tx.replace(e.clone(), after);
        any = true;
    }
    any.then_some(tx)
}

fn size_text(model: &Model, id: ElementId, nominal: Option<f64>) -> String {
    let Some(ElementKind::Pipe(p)) = model.get(id).map(|e| &e.kind) else {
        return String::new();
    };
    let mut q = p.clone();
    if let (Some(n), Some(t)) = (nominal, creabim_services::pipe_types::by_id(&p.pipe_type)) {
        crate::pipesui::set_size(&mut q, t, n);
    }
    crate::pipesui::size_label(&q)
}

impl CreaBimApp {
    /// `calc:storm` and `calc:fire` from the Plumbing tab.
    pub(super) fn net_calc_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" {
            return false;
        }
        self.net_calc = Some(match arg {
            "storm" => NetCalc::storm(self.doc.model()),
            "fire" => NetCalc::fire(self.doc.model()),
            _ => return false,
        });
        true
    }

    /// The calculation's window, while there is one.
    pub(super) fn net_calc_window(&mut self, ctx: &egui::Context) {
        let Some(calc) = &self.net_calc else {
            return;
        };
        let t = crate::i18n::tr;
        let num = crate::i18n::num;
        let model = self.doc.model();
        let mut open = true;
        let (mut do_apply, mut again) = (false, false);
        let mut select = None;
        let sizes = calc.proposals();
        let n_changes = changes(model, &sizes).len();
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        let title = match calc {
            NetCalc::Storm(_) => "Storm water drainage calculation",
            NetCalc::Fire(_) => "Fire main calculation",
        };
        super::shell::window(t(title))
            .id(egui::Id::new("net.calc"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(310.0, 250.0))
            .default_size(Vec2::new(620.0, 500.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button(t("Calculate again")).clicked() {
                        again = true;
                    }
                    if ui
                        .add_enabled(n_changes > 0, egui::Button::new(t("Apply proposed sizes")))
                        .on_hover_text(format!("{}: {n_changes}", t("Pipes to change")))
                        .clicked()
                    {
                        do_apply = true;
                    }
                });
                let warn = |ui: &mut egui::Ui, s: String| {
                    ui.colored_label(ui.visuals().warn_fg_color, s);
                };
                let name_of = |id: ElementId| model.get(id).map_or(String::new(), |e| e.name.clone());
                egui::ScrollArea::vertical().show(ui, |ui| match calc {
                    NetCalc::Storm(None) => {
                        ui.label(t("Draw rainwater pipes (D) from the roof drains to where they leave the building, put a roof drain on each stack's top, then calculate."));
                    }
                    NetCalc::Fire(None) => {
                        ui.label(t("Draw fire water pipes (PV) from the connection to the hose systems, put a hydrant on each branch's end, then calculate."));
                    }
                    NetCalc::Storm(Some(r)) => {
                        crate::props::row(ui, "Rain intensity", |ui| {
                            ui.label(format!("{} l/(s·m²)", num(storm::INTENSITY, 3)))
                        });
                        crate::props::row(ui, "Area drained", |ui| {
                            ui.label(format!("{} m²", num(r.area_m2, 1)))
                        });
                        crate::props::row(ui, "Roof drains and downpipes", |ui| {
                            ui.label(r.drains.len().to_string())
                        });
                        crate::props::row(ui, "Flow out of the building", |ui| {
                            ui.label(format!("{} l/s", num(r.flow_ls, 2)))
                        });
                        if r.drains.is_empty() {
                            warn(ui, t("No roof drain stands on the pipes: add one to each stack's top (the pipe's properties, Add valve)."));
                        }
                        if r.no_area > 0 {
                            warn(ui, format!("{}: {}", t("Drains with no area and no roof over them, counted with 0 m²"), r.no_area));
                        }
                        if r.unreached > 0 {
                            warn(ui, format!("{}: {}", t("Pipes not joined to the network"), r.unreached));
                        }
                        egui::Grid::new("storm.drains").striped(true).num_columns(4).show(ui, |ui| {
                            for h in ["Drain", "Area m²", "C", "Flow l/s"] {
                                ui.strong(t(h));
                            }
                            ui.end_row();
                            for d in &r.drains {
                                if ui.link(&d.name).clicked() {
                                    select = Some(d.id);
                                }
                                ui.label(format!(
                                    "{}{}",
                                    num(d.area_m2, 1),
                                    if d.from_roof { " *" } else { "" }
                                ));
                                ui.label(num(d.coef, 1));
                                ui.label(num(d.flow_ls, 2));
                                ui.end_row();
                            }
                        });
                        ui.add_space(6.0);
                        egui::Grid::new("storm.pipes").striped(true).num_columns(7).show(ui, |ui| {
                            for h in ["Pipe", "Size", "Proposed", "Flow l/s", "Capacity l/s", "Fall %", "h/d"] {
                                ui.strong(t(h));
                            }
                            ui.end_row();
                            for p in &r.pipes {
                                if ui.link(name_of(p.id)).clicked() {
                                    select = Some(p.id);
                                }
                                ui.label(size_text(model, p.id, None));
                                ui.label(size_text(model, p.id, Some(p.proposed_mm)));
                                ui.label(num(p.flow_ls, 2));
                                let cap = num(p.capacity_ls, 2);
                                if p.too_small {
                                    ui.colored_label(ui.visuals().error_fg_color, cap);
                                } else {
                                    ui.label(cap);
                                }
                                if p.vertical {
                                    ui.label(t("stack"));
                                    ui.label("f = 0,33");
                                } else {
                                    let s = num(p.slope * 100.0, 1);
                                    if p.too_flat {
                                        ui.colored_label(ui.visuals().warn_fg_color, s);
                                    } else {
                                        ui.label(s);
                                    }
                                    ui.label(num(p.fill, 2));
                                }
                                ui.end_row();
                            }
                        });
                        ui.add_space(6.0);
                        ui.weak(t("* the roof over the drain, shared among the drains on it. A stack by Wyly-Eaton at f = 0.33, a drain at h/d = 0.7 and its fall, 0.5 % at least (yellow: drawn flatter); red: even the largest size is too small."));
                        ui.weak("STN EN 12056-3 · STN 75 6760");
                    }
                    NetCalc::Fire(Some(r)) => {
                        crate::props::row(ui, "Hose systems", |ui| ui.label(r.hydrants.len().to_string()));
                        crate::props::row(ui, "Working at once", |ui| {
                            ui.label(r.hydrants.len().min(firewater::SIMULTANEOUS).to_string())
                        });
                        crate::props::row(ui, "Flow at the inlet", |ui| {
                            ui.label(format!("{} l/s", num(r.flow_ls, 2)))
                        });
                        crate::props::row(ui, "Pressure needed at the inlet", |ui| {
                            ui.label(format!("{} kPa", num(r.required_kpa, 0)))
                        });
                        if r.hydrants.is_empty() {
                            warn(ui, t("No hydrant stands on the pipes: add one to each branch's end (the pipe's properties, Add valve)."));
                        }
                        if r.unreached > 0 {
                            warn(ui, format!("{}: {}", t("Pipes not joined to the network"), r.unreached));
                        }
                        egui::Grid::new("fire.hydrants").striped(true).num_columns(4).show(ui, |ui| {
                            for h in ["Hose system", "Kind", "Flow l/s", "At the inlet kPa"] {
                                ui.strong(t(h));
                            }
                            ui.end_row();
                            for h in &r.hydrants {
                                let name = if h.critical {
                                    egui::RichText::new(&h.name).strong()
                                } else {
                                    egui::RichText::new(&h.name)
                                };
                                if ui.link(name).clicked() {
                                    select = Some(h.id);
                                }
                                ui.label(h.kind);
                                ui.label(num(h.flow_ls, 2));
                                ui.label(num(h.inlet_kpa, 0));
                                ui.end_row();
                            }
                        });
                        ui.add_space(6.0);
                        egui::Grid::new("fire.pipes").striped(true).num_columns(7).show(ui, |ui| {
                            for h in ["Pipe", "Size", "Proposed", "Flow l/s", "v m/s", "R Pa/m", "Loss kPa"] {
                                ui.strong(t(h));
                            }
                            ui.end_row();
                            for p in &r.pipes {
                                let name = name_of(p.id);
                                let name = if p.critical {
                                    egui::RichText::new(name).strong()
                                } else {
                                    egui::RichText::new(name)
                                };
                                if ui.link(name).clicked() {
                                    select = Some(p.id);
                                }
                                ui.label(size_text(model, p.id, None));
                                ui.label(format!(
                                    "{}{}",
                                    size_text(model, p.id, Some(p.proposed_mm)),
                                    if p.by_min_dn { " *" } else { "" }
                                ));
                                ui.label(num(p.flow_ls, 2));
                                let v = num(p.velocity_ms, 2);
                                if p.too_fast {
                                    ui.colored_label(ui.visuals().error_fg_color, v);
                                } else {
                                    ui.label(v);
                                }
                                ui.label(num(p.r_pa_m, 0));
                                ui.label(num(p.loss_pa / 1000.0, 1));
                                ui.end_row();
                            }
                        });
                        ui.add_space(6.0);
                        ui.weak(t("Bold: the critical path. * the least size its hose systems join. At most 2.5 m/s, local losses +30 %, 200 kPa at the nozzle."));
                        ui.weak("STN EN 671-1 · STN EN 671-2 · STN 92 0400");
                    }
                });
            });
        if let Some(id) = select {
            self.selection.only(id);
        }
        let storm = matches!(self.net_calc, Some(NetCalc::Storm(_)));
        if do_apply {
            if let Some(tx) = apply(self.doc.model(), &sizes) {
                if let Err(e) = self.doc.commit(tx) {
                    self.status = e.to_string();
                }
            }
            again = true;
        }
        if again {
            self.net_calc = Some(if storm {
                NetCalc::storm(self.doc.model())
            } else {
                NetCalc::fire(self.doc.model())
            });
        }
        if !open {
            self.net_calc = None;
        }
    }
}
