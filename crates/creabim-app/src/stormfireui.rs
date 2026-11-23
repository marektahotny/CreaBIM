//! The storm water (STN EN 12056-3) and fire main (STN 92 0400)
//! calculations of what is drawn ([`creabim_services::storm`],
//! [`creabim_services::firewater`]): a window with each pipe's flow and
//! proposed size, and a button that takes the proposals into the model.

use super::CreaBimApp;
use creabim_core::{ElementId, ElementKind, Model, Transaction};
use creabim_services::{circulation, firewater, gas, heating, medgas, sanitary, storm};
use eframe::egui::{self, Vec2};

/// The calculation shown.
pub(super) enum NetCalc {
    Storm(Option<storm::StormResult>),
    Fire(Option<firewater::FireResult>),
    Gas(Option<gas::GasResult>, Box<gas::GasSettings>),
    Heat(Option<heating::HeatResult>, heating::HeatSettings),
    Drain(Option<sanitary::DrainResult>, sanitary::DrainSettings),
    Circ(Option<circulation::CircResult>),
    Med(Vec<medgas::MedResult>),
}

impl NetCalc {
    fn storm(model: &Model) -> Self {
        NetCalc::Storm(storm::run(model, "D"))
    }
    fn fire(model: &Model) -> Self {
        NetCalc::Fire(firewater::run(model, "PV"))
    }
    fn med(model: &Model) -> Self {
        NetCalc::Med(medgas::run_all(model))
    }
    fn circ(model: &Model) -> Self {
        NetCalc::Circ(circulation::run(
            model,
            &circulation::CircSettings::default(),
        ))
    }
    fn drain(model: &Model, s: sanitary::DrainSettings) -> Self {
        NetCalc::Drain(
            sanitary::run(model, "K", &creabim_services::Settings::default(), &s),
            s,
        )
    }
    fn heat(model: &Model, s: heating::HeatSettings) -> Self {
        NetCalc::Heat(heating::run(model, &s), s)
    }
    fn gas(model: &Model, s: gas::GasSettings) -> Self {
        NetCalc::Gas(gas::run(model, "ZP", &s), Box::new(s))
    }
    /// What it found, for the model.
    fn results(&self) -> Option<creabim_core::results::ResultSet> {
        use creabim_services::calcresults as cr;
        Some(match self {
            NetCalc::Storm(Some(r)) => cr::storm(r),
            NetCalc::Fire(Some(r)) => cr::fire(r),
            NetCalc::Gas(Some(r), _) => cr::gas(r),
            NetCalc::Heat(Some(r), _) => cr::heating(r),
            NetCalc::Drain(Some(r), _) => cr::drain(r),
            NetCalc::Circ(Some(r)) => cr::circulation(r),
            NetCalc::Med(rs) if !rs.is_empty() => cr::med(rs),
            _ => return None,
        })
    }
    /// Each pipe and the size proposed for it.
    fn proposals(&self) -> Vec<(ElementId, f64)> {
        match self {
            NetCalc::Storm(Some(r)) => r.pipes.iter().map(|p| (p.id, p.proposed_mm)).collect(),
            NetCalc::Fire(Some(r)) => r.pipes.iter().map(|p| (p.id, p.proposed_mm)).collect(),
            NetCalc::Gas(Some(r), _) => r.pipes.iter().map(|p| (p.id, p.proposed_mm)).collect(),
            NetCalc::Heat(Some(r), _) => r.pipes.iter().map(|p| (p.id, p.proposed_mm)).collect(),
            NetCalc::Drain(Some(r), _) => r.pipes.iter().map(|p| (p.id, p.proposed_mm)).collect(),
            NetCalc::Circ(Some(r)) => r.pipes.iter().map(|p| (p.id, p.proposed_mm)).collect(),
            NetCalc::Med(rs) => rs
                .iter()
                .flat_map(|r| r.pipes.iter().map(|p| (p.id, p.proposed_mm)))
                .collect(),
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
            "medgas" => NetCalc::med(self.doc.model()),
            "circulation" => NetCalc::circ(self.doc.model()),
            "drain" => NetCalc::drain(self.doc.model(), sanitary::DrainSettings::default()),
            "heating" => NetCalc::heat(self.doc.model(), heating::HeatSettings::default()),
            "gas" => NetCalc::gas(self.doc.model(), gas::GasSettings::default()),
            _ => return false,
        });
        if let Some(set) = self.net_calc.as_ref().and_then(NetCalc::results) {
            self.write_results(set);
        }
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
        let mut write_now = false;
        let mut select = None;
        let sizes = calc.proposals();
        let n_changes = changes(model, &sizes).len();
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        let title = match calc {
            NetCalc::Storm(_) => "Storm water drainage calculation",
            NetCalc::Fire(_) => "Fire main calculation",
            NetCalc::Gas(..) => "Gas installation calculation",
            NetCalc::Heat(..) => "Heating pipes calculation",
            NetCalc::Drain(..) => "Foul drainage calculation",
            NetCalc::Circ(_) => "Hot water circulation calculation",
            NetCalc::Med(_) => "Medical gases calculation",
        };
        let mut drain_settings = match calc {
            NetCalc::Drain(_, s) => Some(*s),
            _ => None,
        };
        let mut heat_settings = match calc {
            NetCalc::Heat(_, s) => Some(*s),
            _ => None,
        };
        let mut do_presets = false;
        let presets: Vec<(ElementId, &'static str)> = match calc {
            NetCalc::Heat(Some(r), _) => r.presets.iter().map(|p| (p.valve, p.preset)).collect(),
            _ => vec![],
        };
        let mut gas_settings = match calc {
            NetCalc::Gas(_, s) => Some((**s).clone()),
            _ => None,
        };
        super::shell::window(t(title))
            .id(egui::Id::new("net.calc"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(310.0, 250.0))
            .default_size(crate::app::shell::at_least(Vec2::new(620.0, 500.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                if super::calcresultsui::write_button(ui) {
                    write_now = true;
                }
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
                    NetCalc::Med(rs) => {
                        if rs.is_empty() {
                            ui.label(t("Draw the medical gas pipes (O2, AIR, VAC) from the source to the outlets, end each with an outlet or a supply unit, then calculate."));
                        }
                        for r in rs {
                            med_body(ui, model, r, &mut select);
                            ui.separator();
                        }
                    }
                    NetCalc::Circ(None) => {
                        ui.label(t("Draw the hot water (TV) from the heater and the circulation (C) from the heater's circulation inlet, ending each loop by a hot water pipe, then calculate."));
                    }
                    NetCalc::Circ(Some(r)) => circ_body(ui, model, r, &mut select),
                    NetCalc::Drain(r, _) => {
                        if let Some(ds) = drain_settings.as_mut() {
                            if drain_options(ui, ds) {
                                again = true;
                            }
                        }
                        match r {
                            None => {
                                ui.label(t("Draw the waste pipes (K) from each fixture's waste to where they leave the building, falling towards it, then calculate."));
                            }
                            Some(r) => drain_body(ui, model, r, &mut select),
                        }
                    }
                    NetCalc::Heat(r, _) => {
                        if let Some(hs) = heat_settings.as_mut() {
                            if heat_options(ui, hs) {
                                again = true;
                            }
                        }
                        match r {
                            None => {
                                ui.label(t("Draw the heating supply (UKP) and return (UKS) from the boiler to each radiator's connections, then calculate."));
                            }
                            Some(r) => {
                                if ui
                                    .add_enabled(!presets.is_empty(), egui::Button::new(t("Write the presettings to the valves")))
                                    .clicked()
                                {
                                    do_presets = true;
                                }
                                heat_body(ui, model, r, &mut select)
                            }
                        }
                    }
                    NetCalc::Gas(r, _) => {
                        if let Some(gs) = gas_settings.as_mut() {
                            if gas_options(ui, gs) {
                                again = true;
                            }
                        }
                        match r {
                            None => {
                                ui.label(t("Draw gas pipes (ZP) from the cabinet at the boundary to the appliances, put a gas appliance on each branch's end, then calculate."));
                            }
                            Some(r) => gas_body(ui, model, r, &mut select),
                        }
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
        let fire = matches!(self.net_calc, Some(NetCalc::Fire(_)));
        let circ = matches!(self.net_calc, Some(NetCalc::Circ(_)));
        let med = matches!(self.net_calc, Some(NetCalc::Med(_)));
        if do_presets {
            if let Some(tx) = write_presets(self.doc.model(), &presets) {
                if let Err(e) = self.doc.commit(tx) {
                    self.status = e.to_string();
                }
            }
            again = true;
        }
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
            } else if fire {
                NetCalc::fire(self.doc.model())
            } else if med {
                NetCalc::med(self.doc.model())
            } else if circ {
                NetCalc::circ(self.doc.model())
            } else if let Some(ds) = drain_settings {
                NetCalc::drain(self.doc.model(), ds)
            } else if let Some(hs) = heat_settings {
                NetCalc::heat(self.doc.model(), hs)
            } else {
                NetCalc::gas(self.doc.model(), gas_settings.unwrap_or_default())
            });
        }
        if write_now {
            if let Some(set) = self.net_calc.as_ref().and_then(NetCalc::results) {
                self.write_results_said(set);
            }
        }
        if !open {
            self.net_calc = None;
        }
    }
}

/// The gas calculation's settings: true when one changed.
fn gas_options(ui: &mut egui::Ui, s: &mut gas::GasSettings) -> bool {
    let t = crate::i18n::tr;
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        egui::ComboBox::from_id_salt("gas.kind")
            .selected_text(t(s.gas.props().name))
            .show_ui(ui, |ui| {
                for k in gas::GasKind::ALL {
                    changed |= ui.selectable_value(&mut s.gas, k, t(k.props().name)).changed();
                }
            });
        changed |= ui
            .checkbox(&mut s.medium_pressure, t("Medium pressure inside"))
            .on_hover_text(t("Low pressure up to 5 kPa (100 Pa loss, 10 m/s); medium 5–400 kPa (10 % loss, 20 m/s, a regulator before each appliance)"))
            .changed();
        changed |= ui
            .checkbox(&mut s.service_medium, t("Medium pressure service pipe"))
            .on_hover_text(t("The cabinet at the boundary then has a pressure regulator"))
            .changed();
        changed |= ui.checkbox(&mut s.metering, t("Gas meter")).changed();
    });
    changed
}

fn gas_body(ui: &mut egui::Ui, model: &Model, r: &gas::GasResult, select: &mut Option<ElementId>) {
    let t = crate::i18n::tr;
    let num = crate::i18n::num;
    let warn = |ui: &mut egui::Ui, s: String| {
        ui.colored_label(ui.visuals().warn_fg_color, s);
    };
    let row = |ui: &mut egui::Ui, k: &str, v: String| {
        crate::props::row(ui, k, |ui| ui.label(v));
    };
    row(ui, "Appliances", r.appliances.len().to_string());
    row(
        ui,
        "Installed flow",
        format!(
            "{} m³/h ({} kW)",
            num(r.installed_m3h, 2),
            num(r.installed_kw, 1)
        ),
    );
    row(ui, "Design flow Vr", format!("{} m³/h", num(r.vr_m3h, 2)));
    row(
        ui,
        "Loss on the critical branch",
        format!(
            "{} Pa ({} {} Pa)",
            num(r.loss_pa, 1),
            t("limit"),
            num(r.allowed_pa, 0)
        ),
    );
    row(
        ui,
        "Friction and local losses",
        format!(
            "{} {} Pa, {} {} Pa",
            t("horizontal"),
            num(r.horizontal_pa, 1),
            t("risers"),
            num(r.vertical_pa, 1)
        ),
    );
    row(
        ui,
        "Lift of the gas",
        format!("{} Pa (Δh = {} m)", num(r.lift_pa, 1), num(r.rise_m, 2)),
    );
    row(
        ui,
        "Pressure before the critical appliance",
        format!(
            "{} kPa ({} {} kPa)",
            num(r.pressure_kpa, 2),
            t("start"),
            num(r.start_kpa, 1)
        ),
    );
    row(ui, "Gas density", format!("{} kg/m³", num(r.rho, 3)));
    row(
        ui,
        "Highest velocity",
        format!("{} m/s", num(r.vmax_found, 1)),
    );
    if let Some((g, q)) = r.meter {
        row(ui, "Gas meter", format!("{g} (Qmax {} m³/h)", num(q, 0)));
    }
    if r.loss_pa > r.allowed_pa + 1e-6 {
        warn(ui, t("The loss is over the allowed: the largest sizes of the critical branch are not enough, choose a larger series."));
    }
    if r.enlarged > 0 {
        ui.label(format!(
            "{}: {}",
            t("The critical branch was enlarged, steps"),
            r.enlarged
        ));
    }
    if r.over_house_set {
        warn(ui, t("The flow is over 25 m³/h: a house regulator set is not enough, design a regulating station."));
    }
    if r.appliances.is_empty() {
        warn(ui, t("No appliance stands on the pipes: add a gas appliance to each branch's end (the pipe's properties, Add valve)."));
    }
    if r.appliances.iter().any(|a| !a.matched) {
        warn(ui, t("Some appliances were not known by their name and were taken as a 25 kW boiler: give their heat input in their properties."));
    }
    if r.unreached > 0 {
        warn(
            ui,
            format!("{}: {}", t("Pipes not joined to the network"), r.unreached),
        );
    }
    let name_of = |id: ElementId| model.get(id).map_or(String::new(), |e| e.name.clone());
    egui::Grid::new("gas.apps")
        .striped(true)
        .num_columns(6)
        .show(ui, |ui| {
            for h in [
                "Appliance",
                "Kind",
                "Category",
                "kW",
                "m³/h",
                "Pressure kPa",
            ] {
                ui.strong(t(h));
            }
            ui.end_row();
            for a in &r.appliances {
                let name = if a.critical {
                    egui::RichText::new(&a.name).strong()
                } else {
                    egui::RichText::new(&a.name)
                };
                if ui.link(name).clicked() {
                    *select = Some(a.id);
                }
                if a.matched {
                    ui.label(t(a.kind));
                } else {
                    ui.colored_label(ui.visuals().warn_fg_color, t(a.kind));
                }
                ui.label(format!("K{}", a.cat));
                ui.label(num(a.kw, 1));
                ui.label(num(a.flow_m3h, 2));
                ui.label(num(a.pressure_kpa, 2));
                ui.end_row();
            }
        });
    ui.add_space(6.0);
    egui::Grid::new("gas.pipes")
        .striped(true)
        .num_columns(7)
        .show(ui, |ui| {
            for h in [
                "Pipe", "Size", "Proposed", "Vr m³/h", "v m/s", "R Pa/m", "Loss Pa",
            ] {
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
                    *select = Some(p.id);
                }
                ui.label(&p.size);
                ui.label(&p.proposed);
                ui.label(num(p.vr_m3h, 2));
                let v = num(p.velocity_ms, 2);
                if p.too_fast {
                    ui.colored_label(ui.visuals().error_fg_color, v);
                } else {
                    ui.label(v);
                }
                ui.label(num(p.r_pa_m, 1));
                ui.label(num(p.loss_pa, 1));
                ui.end_row();
            }
        });
    ui.add_space(6.0);
    ui.weak(t("Bold: the critical branch. Vr = Σ Ki·Vi (K1 = n^-0.5 cooking, K2 = n^-0.15 local and storage heaters, K3 = n^-0.1 boilers, K4 = 1 industrial); local losses +50 %; a lift of light gas in risers makes up for their losses."));
    ui.weak(t("Cabinet at the boundary: main shut-off (HUP), a pressure regulator with a filter for a medium pressure service pipe, the gas meter, an outlet valve; ventilated, 1 m protective zone."));
    ui.weak("STN EN 1775 · TPP 704 01 · TPP 704 03 · STN EN 1359");
}

/// The edits that write each valve's presetting.
fn write_presets(model: &Model, presets: &[(ElementId, &'static str)]) -> Option<Transaction> {
    let mut tx = Transaction::new("Preset valves");
    let mut any = false;
    for &(id, p) in presets {
        let Some(e) = model.get(id) else { continue };
        let ElementKind::Valve(v) = &e.kind else {
            continue;
        };
        if v.preset.as_deref() == Some(p) {
            continue;
        }
        let mut after = e.clone();
        if let ElementKind::Valve(w) = &mut after.kind {
            w.preset = Some(p.to_string());
        }
        tx = tx.replace(e.clone(), after);
        any = true;
    }
    any.then_some(tx)
}

/// The heating calculation's temperatures: true when one changed.
fn heat_options(ui: &mut egui::Ui, s: &mut heating::HeatSettings) -> bool {
    let t = crate::i18n::tr;
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.label(t("Supply"));
        changed |= ui
            .add(
                egui::DragValue::new(&mut s.supply_c)
                    .range(25.0..=95.0)
                    .suffix(" °C"),
            )
            .changed();
        ui.label(t("Return"));
        changed |= ui
            .add(
                egui::DragValue::new(&mut s.return_c)
                    .range(20.0..=90.0)
                    .suffix(" °C"),
            )
            .changed();
        ui.label(t("Radiator and valve"));
        changed |= ui
            .add(
                egui::DragValue::new(&mut s.terminal_kpa)
                    .range(0.0..=50.0)
                    .suffix(" kPa"),
            )
            .changed();
    });
    changed
}

fn heat_body(
    ui: &mut egui::Ui,
    model: &Model,
    r: &heating::HeatResult,
    select: &mut Option<ElementId>,
) {
    let t = crate::i18n::tr;
    let num = crate::i18n::num;
    let warn = |ui: &mut egui::Ui, s: String| {
        ui.colored_label(ui.visuals().warn_fg_color, s);
    };
    let row = |ui: &mut egui::Ui, k: &str, v: String| {
        crate::props::row(ui, k, |ui| ui.label(v));
    };
    row(
        ui,
        "Radiators / output",
        format!("{} / {} kW", r.radiators.len(), num(r.power_w / 1000.0, 2)),
    );
    row(
        ui,
        "Total flow",
        format!(
            "{} l/s = {} m³/h",
            num(r.flow_ls, 3),
            num(r.flow_ls * 3.6, 2)
        ),
    );
    row(
        ui,
        "Critical circuit",
        format!("{} kPa", num(r.critical_pa / 1000.0, 1)),
    );
    row(
        ui,
        "Circulation pump",
        format!(
            "Q = {} m³/h, H = {} kPa ≈ {} m",
            num(r.flow_ls * 3.6, 2),
            num(r.pump_kpa, 1),
            num(r.pump_m, 2)
        ),
    );
    if r.no_power > 0 {
        warn(
            ui,
            format!(
                "{}: {}",
                t("Radiators of no known output, counted with no flow"),
                r.no_power
            ),
        );
    }
    if r.no_return {
        warn(
            ui,
            t("No return (UKS) is drawn: the circuits hold the supply only."),
        );
    }
    if r.radiators.iter().any(|x| !x.returned) && !r.no_return {
        warn(ui, t("Some radiators are not on the return."));
    }
    if r.unreached > 0 {
        warn(
            ui,
            format!("{}: {}", t("Pipes not joined to the network"), r.unreached),
        );
    }
    egui::Grid::new("heat.rads")
        .striped(true)
        .num_columns(6)
        .show(ui, |ui| {
            for h in [
                "Radiator",
                "W",
                "l/h",
                "Supply kPa",
                "Return kPa",
                "Throttle kPa",
            ] {
                ui.strong(t(h));
            }
            ui.end_row();
            for x in &r.radiators {
                let name = if x.critical {
                    egui::RichText::new(&x.name).strong()
                } else {
                    egui::RichText::new(&x.name)
                };
                if ui.link(name).clicked() {
                    *select = Some(x.id);
                }
                ui.label(num(x.power_w, 0));
                ui.label(num(x.flow_ls * 3600.0, 0));
                ui.label(num(x.supply_pa / 1000.0, 2));
                ui.label(num(x.return_pa / 1000.0, 2));
                ui.label(num(x.throttle_pa / 1000.0, 2));
                ui.end_row();
            }
        });
    if !r.presets.is_empty() {
        ui.add_space(6.0);
        ui.strong(t("Valve presettings (HERZ)"));
        egui::Grid::new("heat.presets").striped(true).num_columns(7).show(ui, |ui| {
            for h in ["Valve", "DN", "Q m³/h", "Δp target kPa", "kv needed", "Presetting", "Δp kPa"] {
                ui.strong(t(h));
            }
            ui.end_row();
            for p in &r.presets {
                if ui.link(p.product).on_hover_text(p.code).clicked() {
                    *select = Some(p.valve);
                }
                ui.label(p.dn.to_string());
                ui.label(num(p.flow_m3h, 3));
                ui.label(num(p.target_kpa, 1));
                ui.label(num(p.kv_req, 3));
                let pre = format!("{} (kv {})", p.preset, num(p.kv, 3));
                match p.note {
                    heating::PresetNote::Under => {
                        ui.colored_label(ui.visuals().warn_fg_color, pre)
                            .on_hover_text(t("Even the smallest presetting throttles too little: a smaller valve or a differential pressure regulator"));
                    }
                    heating::PresetNote::Open => {
                        ui.label(pre).on_hover_text(t("Wide open: its circuit is the critical one"));
                    }
                    heating::PresetNote::None => {
                        ui.label(pre);
                    }
                }
                ui.label(num(p.dp_kpa, 1));
                ui.end_row();
            }
        });
    }
    ui.add_space(6.0);
    let name_of = |id: ElementId| model.get(id).map_or(String::new(), |e| e.name.clone());
    egui::Grid::new("heat.pipes")
        .striped(true)
        .num_columns(7)
        .show(ui, |ui| {
            for h in [
                "Pipe", "Size", "Proposed", "Flow l/s", "v m/s", "R Pa/m", "Loss Pa",
            ] {
                ui.strong(t(h));
            }
            ui.end_row();
            for p in &r.pipes {
                let name = format!("{} {}", p.code, name_of(p.id));
                let name = if p.critical {
                    egui::RichText::new(name).strong()
                } else {
                    egui::RichText::new(name)
                };
                if ui.link(name).clicked() {
                    *select = Some(p.id);
                }
                ui.label(&p.size);
                ui.label(&p.proposed);
                ui.label(num(p.flow_ls, 3));
                let v = num(p.velocity_ms, 2);
                if p.too_fast {
                    ui.colored_label(ui.visuals().error_fg_color, v);
                } else {
                    ui.label(v);
                }
                ui.label(num(p.r_pa_m, 0));
                ui.label(num(p.loss_pa, 0));
                ui.end_row();
            }
        });
    ui.add_space(6.0);
    ui.weak(t("Bold: the critical circuit. At most 0.8 m/s on the mains, 0.5 m/s to one radiator, 150 Pa/m; fittings +30 %; a radiator with its valve 10 kPa, the source 15 kPa, the pump +10 %. Presetting as HERZ's programs: kv = Q / √(Δp / 100), the largest kv no more than it, upstream valves first."));
    ui.weak("STN EN 12828 · STN EN 442 · HERZ STRÖMAX MS, TS-98-V");
}

/// The drainage calculation's frequency factor: true when it changed.
fn drain_options(ui: &mut egui::Ui, s: &mut sanitary::DrainSettings) -> bool {
    let t = crate::i18n::tr;
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        const K: [(f64, &str); 4] = [
            (0.5, "K = 0.5 homes, offices"),
            (0.7, "K = 0.7 schools, hospitals, restaurants"),
            (1.0, "K = 1.0 public toilets, showers"),
            (1.2, "K = 1.2 laboratories"),
        ];
        let now = K
            .iter()
            .find(|k| (k.0 - s.k).abs() < 1e-9)
            .map_or(format!("K = {}", s.k), |k| t(k.1));
        egui::ComboBox::from_id_salt("drain.k")
            .selected_text(now)
            .show_ui(ui, |ui| {
                for (k, name) in K {
                    changed |= ui.selectable_value(&mut s.k, k, t(name)).changed();
                }
            });
        changed |= ui
            .checkbox(&mut s.swept, t("Swept branches into the stacks"))
            .changed();
    });
    changed
}

fn drain_body(
    ui: &mut egui::Ui,
    model: &Model,
    r: &sanitary::DrainResult,
    select: &mut Option<ElementId>,
) {
    let t = crate::i18n::tr;
    let num = crate::i18n::num;
    let warn = |ui: &mut egui::Ui, s: String| {
        ui.colored_label(ui.visuals().warn_fg_color, s);
    };
    let row = |ui: &mut egui::Ui, k: &str, v: String| {
        crate::props::row(ui, k, |ui| ui.label(v));
    };
    row(
        ui,
        "ΣDU / fixtures",
        format!("{} l/s / {}", num(r.sum_du, 1), r.fixtures),
    );
    row(
        ui,
        "Waste flow at the outlet Qww",
        format!("{} l/s", num(r.flow_ls, 2)),
    );
    if r.outlet_dn > 0.0 {
        row(
            ui,
            "Outlet pipe",
            format!(
                "DN {}, {} {} %, v = {} m/s",
                num(r.outlet_dn, 0),
                t("filled"),
                num(r.outlet_fill * 100.0, 0),
                num(r.outlet_velocity, 2)
            ),
        );
    }
    row(ui, "Stacks", r.stacks.to_string());
    if r.fixtures == 0 {
        warn(
            ui,
            t("No fixture drains into the pipes: end a pipe at each fixture's waste connection."),
        );
    }
    if !r.unmatched.is_empty() {
        warn(
            ui,
            format!(
                "{}: {}",
                t("Fixtures not known by their name"),
                r.unmatched.join(", ")
            ),
        );
    }
    let n = r.pipes.iter().filter(|p| p.no_slope).count();
    if n > 0 {
        warn(
            ui,
            format!("{}: {n}", t("Horizontal pipes with no fall, taken at 2 %")),
        );
    }
    let n = r.pipes.iter().filter(|p| p.too_flat).count();
    if n > 0 {
        warn(
            ui,
            format!(
                "{}: {n}",
                t("Horizontal pipes flatter than the least fall (1 %, connecting 2 %)")
            ),
        );
    }
    if r.pipes.iter().any(|p| p.over) {
        warn(ui, t("Some pipes are too small even at the largest size of their type: a larger series, a steeper fall or another stack."));
    }
    if r.unreached > 0 {
        warn(
            ui,
            format!("{}: {}", t("Pipes not joined to the network"), r.unreached),
        );
    }
    let name_of = |id: ElementId| model.get(id).map_or(String::new(), |e| e.name.clone());
    egui::Grid::new("drain.pipes")
        .striped(true)
        .num_columns(8)
        .show(ui, |ui| {
            for h in [
                "Pipe", "Size", "Proposed", "ΣDU", "Q l/s", "Fall %", "h/d", "v m/s",
            ] {
                ui.strong(t(h));
            }
            ui.end_row();
            for p in &r.pipes {
                if ui.link(name_of(p.id)).clicked() {
                    *select = Some(p.id);
                }
                ui.label(&p.size);
                if p.over {
                    ui.colored_label(ui.visuals().error_fg_color, &p.proposed);
                } else {
                    ui.label(&p.proposed);
                }
                ui.label(num(p.sum_du, 1));
                ui.label(num(p.flow_ls, 2));
                if p.stack {
                    ui.label(t("stack"));
                    ui.label(format!("≤ {}", num(p.capacity_ls, 1)));
                } else {
                    let s = num(p.slope * 100.0, 1);
                    if p.too_flat || p.no_slope {
                        ui.colored_label(ui.visuals().warn_fg_color, s);
                    } else {
                        ui.label(s);
                    }
                    ui.label(num(p.fill, 2));
                }
                let v = num(p.velocity_ms, 2);
                if p.slow {
                    ui.colored_label(ui.visuals().warn_fg_color, v);
                } else {
                    ui.label(v);
                }
                ui.end_row();
            }
        });
    // the Wavin stacks' installation noise
    let loud: Vec<&sanitary::PipeResult> =
        r.pipes.iter().filter(|p| p.noise_db.is_some()).collect();
    if !loud.is_empty() {
        ui.add_space(6.0);
        ui.strong(t("Installation noise of the Wavin stacks"));
        egui::Grid::new("drain.noise")
            .striped(true)
            .num_columns(4)
            .show(ui, |ui| {
                for h in ["Stack", "Q l/s", "Clip dB(A)", "Double clip dB(A)"] {
                    ui.strong(t(h));
                }
                ui.end_row();
                for p in loud {
                    let (a, b) = p.noise_db.unwrap_or_default();
                    if ui.link(name_of(p.id)).clicked() {
                        *select = Some(p.id);
                    }
                    ui.label(num(p.flow_ls, 2));
                    for v in [a, b] {
                        let text = num(v, 0);
                        if v > sanitary::NOISE_LIMIT_DB {
                            ui.colored_label(ui.visuals().warn_fg_color, text);
                        } else {
                            ui.label(text);
                        }
                    }
                    ui.end_row();
                }
            });
        ui.weak(t(
            "Wavin's measurements at 2 and 4 l/s, against 30 dB(A) (DIN 4109).",
        ));
    }
    ui.add_space(6.0);
    ui.weak(t("Qww = K·√ΣDU, no less than the largest fixture's DU. Horizontal pipes half full (h/d 0.5, kb 1 mm), connecting pipes the fixture's DN, stacks by their capacity, DN 100 downstream of a WC. Yellow: under the least fall, or slower than 0.7 m/s."));
    ui.weak("STN EN 12056-2 · STN 75 6760");
}

fn circ_body(
    ui: &mut egui::Ui,
    model: &Model,
    r: &circulation::CircResult,
    select: &mut Option<ElementId>,
) {
    let t = crate::i18n::tr;
    let num = crate::i18n::num;
    let warn = |ui: &mut egui::Ui, s: String| {
        ui.colored_label(ui.visuals().warn_fg_color, s);
    };
    let row = |ui: &mut egui::Ui, k: &str, v: String| {
        crate::props::row(ui, k, |ui| ui.label(v));
    };
    row(
        ui,
        "Heat lost by the circulated pipes",
        format!("{} W ({} m)", num(r.loss_w, 0), num(r.circulated_m, 1)),
    );
    row(
        ui,
        "Circulation flow Qc",
        format!(
            "{} l/s = {} l/h",
            num(r.flow_ls, 3),
            num(r.flow_ls * 3600.0, 0)
        ),
    );
    row(
        ui,
        "Circulation pump",
        format!(
            "Q = {} l/h, H = {} kPa ≈ {} m",
            num(r.flow_ls * 3600.0, 0),
            num(r.pump_kpa, 1),
            num(r.pump_m, 2)
        ),
    );
    if r.loops.is_empty() {
        warn(ui, t("No circulation pipe ends by a hot water pipe: end each loop within 1 m of the hot water it keeps hot."));
    }
    if r.loose_ends > 0 {
        warn(
            ui,
            format!(
                "{}: {}",
                t("Circulation ends by no hot water pipe"),
                r.loose_ends
            ),
        );
    }
    if r.unreached > 0 {
        warn(
            ui,
            format!("{}: {}", t("Pipes not joined to the network"), r.unreached),
        );
    }
    let name_of = |id: ElementId| model.get(id).map_or(String::new(), |e| e.name.clone());
    egui::Grid::new("circ.loops")
        .striped(true)
        .num_columns(6)
        .show(ui, |ui| {
            for h in [
                "Loop",
                "l/h",
                "Hot water kPa",
                "Circulation kPa",
                "Valve Δp kPa",
                "kv",
            ] {
                ui.strong(t(h));
            }
            ui.end_row();
            for l in &r.loops {
                let name = name_of(l.circ_pipe);
                let name = if l.critical {
                    egui::RichText::new(name).strong()
                } else {
                    egui::RichText::new(name)
                };
                if ui.link(name).clicked() {
                    *select = Some(l.circ_pipe);
                }
                ui.label(num(l.flow_ls * 3600.0, 0));
                ui.label(num(l.hot_pa / 1000.0, 2));
                ui.label(num(l.circ_pa / 1000.0, 2));
                ui.label(num(l.valve_kpa, 1));
                ui.label(num(l.kv, 3));
                ui.end_row();
            }
        });
    ui.add_space(6.0);
    egui::Grid::new("circ.pipes")
        .striped(true)
        .num_columns(6)
        .show(ui, |ui| {
            for h in ["Pipe", "Size", "Proposed", "Flow l/s", "v m/s", "R Pa/m"] {
                ui.strong(t(h));
            }
            ui.end_row();
            for p in &r.pipes {
                if ui.link(name_of(p.id)).clicked() {
                    *select = Some(p.id);
                }
                ui.label(&p.size);
                ui.label(&p.proposed);
                ui.label(num(p.flow_ls, 3));
                let v = num(p.velocity_ms, 2);
                if p.too_fast {
                    ui.colored_label(ui.visuals().error_fg_color, v);
                } else {
                    ui.label(v);
                }
                ui.label(num(p.r_pa_m, 0));
                ui.end_row();
            }
        });
    ui.add_space(6.0);
    ui.weak(t("Bold: the critical loop. Hot water at 55 °C cooling 5 K; insulated pipes lose 7 W/m in risers and 10 W/m along; the circulation at most 0.5 m/s and 100 Pa/m; each loop's balancing valve (HERZ STRÖMAX 4117 MW) takes at least 5 kPa; the heater and check valve 3 kPa, the pump +10 %."));
    ui.weak("STN EN 806-2 · STN 06 0320");
}

fn med_body(
    ui: &mut egui::Ui,
    model: &Model,
    r: &medgas::MedResult,
    select: &mut Option<ElementId>,
) {
    let t = crate::i18n::tr;
    let num = crate::i18n::num;
    let warn = |ui: &mut egui::Ui, s: String| {
        ui.colored_label(ui.visuals().warn_fg_color, s);
    };
    let row = |ui: &mut egui::Ui, k: &str, v: String| {
        crate::props::row(ui, k, |ui| ui.label(v));
    };
    ui.strong(t(r.gas.name()));
    row(ui, "Outlets", r.outlets.len().to_string());
    row(ui, "Design flow", format!("{} l/min", num(r.flow_lmin, 1)));
    row(
        ui,
        "Loss on the critical branch",
        format!(
            "{} kPa ({} {} kPa)",
            num(r.loss_pa / 1000.0, 2),
            t("limit"),
            num(r.limit_pa / 1000.0, 0)
        ),
    );
    row(
        ui,
        "At the critical outlet",
        format!(
            "{} kPa ({} {} kPa, {} {} kPa)",
            num(r.outlet_kpa, 1),
            t("source"),
            num(r.source_kpa, 0),
            t("least"),
            num(r.min_kpa, 0)
        ),
    );
    row(ui, "Gas density", format!("{} kg/m³", num(r.rho, 3)));
    if r.loss_pa > r.limit_pa + 1e-6 {
        warn(ui, t("The loss is over the allowed: the largest sizes of the critical branch are not enough, choose a larger series."));
    }
    if r.enlarged > 0 {
        ui.label(format!(
            "{}: {}",
            t("The critical branch was enlarged, steps"),
            r.enlarged
        ));
    }
    if r.unreached > 0 {
        warn(
            ui,
            format!("{}: {}", t("Pipes not joined to the network"), r.unreached),
        );
    }
    egui::Grid::new(("med.kinds", r.gas.code()))
        .striped(true)
        .num_columns(5)
        .show(ui, |ui| {
            for h in [
                "Kind of room",
                "Outlets",
                "Q1 l/min",
                "Each further l/min",
                "Q l/min",
            ] {
                ui.strong(t(h));
            }
            ui.end_row();
            for k in &r.kinds {
                ui.label(t(k.kind));
                ui.label(k.n.to_string());
                ui.label(num(k.q1, 1));
                ui.label(num(k.add, 2));
                ui.label(num(k.q, 1));
                ui.end_row();
            }
        });
    ui.add_space(6.0);
    let name_of = |id: ElementId| model.get(id).map_or(String::new(), |e| e.name.clone());
    egui::Grid::new(("med.outlets", r.gas.code()))
        .striped(true)
        .num_columns(4)
        .show(ui, |ui| {
            for h in ["Outlet", "Room", "Kind of room", "Loss kPa"] {
                ui.strong(t(h));
            }
            ui.end_row();
            for o in &r.outlets {
                let name = if o.critical {
                    egui::RichText::new(&o.name).strong()
                } else {
                    egui::RichText::new(&o.name)
                };
                if ui.link(name).clicked() {
                    *select = Some(o.id);
                }
                ui.label(&o.room);
                ui.label(t(o.kind));
                ui.label(num(o.loss_pa / 1000.0, 2));
                ui.end_row();
            }
        });
    ui.add_space(6.0);
    egui::Grid::new(("med.pipes", r.gas.code()))
        .striped(true)
        .num_columns(7)
        .show(ui, |ui| {
            for h in [
                "Pipe", "Size", "Proposed", "l/min", "v m/s", "R Pa/m", "Outlets",
            ] {
                ui.strong(t(h));
            }
            ui.end_row();
            for p in &r.pipes {
                let name = if p.critical {
                    egui::RichText::new(name_of(p.id)).strong()
                } else {
                    egui::RichText::new(name_of(p.id))
                };
                if ui.link(name).clicked() {
                    *select = Some(p.id);
                }
                ui.label(&p.size);
                ui.label(&p.proposed);
                ui.label(num(p.flow_lmin, 1));
                let v = num(p.velocity_ms, 2);
                if p.too_fast {
                    ui.colored_label(ui.visuals().error_fg_color, v);
                } else {
                    ui.label(v);
                }
                ui.label(num(p.r_pa_m, 0));
                ui.label(p.outlets.to_string());
                ui.end_row();
            }
        });
    ui.add_space(6.0);
    ui.weak(t("Bold: the critical branch. Each kind of room: Q = Q1 + (n − 1)·Δ (HTM 02-01); copper at most 10 m/s, vacuum 15 m/s; fittings +30 %; O2 and AIR from 420 kPa, at least 370 kPa at the outlets; VAC 60 kPa below the atmosphere, at least 40 kPa."));
    ui.weak("STN EN ISO 7396-1 · STN EN 13348 · STN EN ISO 9170-1");
}
