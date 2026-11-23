//! Propose the valves a system still needs
//! ([`creabim_core::autovalves`]): the meter assembly, shut-offs, angle
//! valves and a water heater's valves on water; vent heads and cleanouts
//! on waste. What it proposes goes into the model in one step, to undo
//! as one.

use super::CreaBimApp;
use creabim_core::autovalves::{self, Fill, FillOpts};
use creabim_core::mep::valve_kind;
use creabim_core::{Element, ElementKind, Transaction};
use eframe::egui::{self, Vec2};

/// The systems it proposes for.
const SYSTEMS: [(&str, &str); 10] = [
    ("SV", "Cold water"),
    ("TV", "Hot water"),
    ("C", "Hot water circulation"),
    ("PV", "Fire water – hydrants"),
    ("K", "Waste"),
    ("UKP", "Heating – supply"),
    ("UKS", "Heating – return"),
    ("CHP", "Cooling – supply"),
    ("CHS", "Cooling – return"),
    ("ZP", "Gas – natural gas"),
];

/// What was added, in words.
fn report(f: &Fill) -> String {
    let t = crate::i18n::tr;
    let mut parts: Vec<String> = vec![];
    if !f.assembly.is_empty() {
        let names: Vec<String> = f
            .assembly
            .iter()
            .map(|k| t(valve_kind(k).map_or(k, |d| d.name)).to_lowercase())
            .collect();
        parts.push(format!("{}: {}", t("at the start"), names.join(", ")));
    }
    for (n, what) in [
        (
            f.heaters,
            "water heater inlets with a shut-off and a check valve",
        ),
        (f.shut_offs, "shut-offs"),
        (f.angle_valves, "valves at fixtures and radiators"),
        (f.vents, "vent heads"),
        (f.cleanouts, "cleanouts"),
        (f.air_vents, "air vents at high points"),
        (f.drains, "drain cocks at low points"),
        (f.sleeves, "sleeves where it passes walls and floors"),
        (f.thermal, "thermal shut-offs at walls between rooms"),
        (f.seals, "fire stopping into another fire zone"),
    ] {
        if n > 0 {
            parts.push(format!("{} {n}", t(what)));
        }
    }
    parts.join("; ")
}

impl CreaBimApp {
    /// `valves:auto` from the Plumbing tab: open the window.
    pub(super) fn auto_valves_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "valves" || arg != "auto" {
            return false;
        }
        if self.auto_valves.is_none() {
            self.auto_valves = Some(FillOpts::default());
        }
        true
    }

    /// The window, while it is open.
    pub(super) fn auto_valves_window(&mut self, ctx: &egui::Context) {
        let Some(mut o) = self.auto_valves else {
            return;
        };
        let t = crate::i18n::tr;
        let mut open = true;
        let mut run: Option<(&str, Fill)> = None;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Propose valves"))
            .id(egui::Id::new("valves.auto"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(200.0, 180.0))
            .default_size(crate::app::shell::at_least(Vec2::new(400.0, 340.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new(t("Water")).strong());
                ui.checkbox(
                    &mut o.assembly,
                    t("Meter assembly on the connection, a shut-off where hot water sets out, a water heater's inlet"),
                );
                ui.checkbox(&mut o.reducer, t("Pressure reducer in the assembly"));
                ui.checkbox(&mut o.branches, t("Shut-offs on risers and on branches to more fixtures"));
                ui.checkbox(&mut o.angle, t("Angle valves at fixtures"));
                ui.add_space(4.0);
                ui.label(egui::RichText::new(t("Waste")).strong());
                ui.checkbox(&mut o.vents, t("Vent heads on stacks"));
                ui.checkbox(
                    &mut o.cleanouts,
                    t("Cleanouts at the foot of stacks, at turns and every 20 m of a drain"),
                );
                ui.add_space(4.0);
                ui.label(egui::RichText::new(t("Water, heating, cooling, fire main")).strong());
                ui.checkbox(
                    &mut o.high_low,
                    t("Air vents at high points and drain cocks at low points of the pipes"),
                );
                ui.add_space(4.0);
                ui.label(egui::RichText::new(t("Gas")).strong());
                ui.checkbox(
                    &mut o.crossings,
                    t("Sleeves through walls and floors, thermal shut-offs at walls between rooms, fire stopping into another fire zone (TPH 03 2012)"),
                );
                ui.separator();
                for (code, name) in SYSTEMS {
                    let f = autovalves::fill(self.doc.model(), code, &o);
                    ui.horizontal(|ui| {
                        let n = f.valves.len();
                        let text = format!("{} ({code})", t(name));
                        if f.unsupported {
                            ui.add_enabled(false, egui::Button::new(text));
                            ui.weak(t("no pipes drawn"));
                        } else if n == 0 {
                            ui.add_enabled(false, egui::Button::new(text));
                            ui.weak(t("nothing missing"));
                        } else {
                            if ui.button(format!("{text}: {} {n}", t("add"))).clicked() {
                                run = Some((code, f.clone()));
                            }
                        }
                    });
                    if !f.valves.is_empty() {
                        ui.weak(report(&f));
                    }
                }
                ui.add_space(4.0);
                ui.weak(t("Heating: a thermostatic valve and a lockshield at every radiator, shut-offs at the manifold and the source, a strainer on the return. Gas: the main shut-off and a cock before every appliance."));
                ui.weak("STN EN 806-2 · STN 73 6660 · STN 06 0830 · STN EN 12056-2 · STN 75 6760 · STN EN 12828 · STN EN 1775 · TPH 03 2012");
            });
        if let Some((code, f)) = run {
            let mut tx = Transaction::new("Propose valves");
            let mut ids = vec![];
            for p in &f.valves {
                let name = t(valve_kind(&p.valve.kind).map_or(p.valve.kind.as_str(), |k| k.name));
                let name = match p.dn {
                    Some(dn) => format!("{name} DN {dn}"),
                    None => name,
                };
                let e = Element::new(name, ElementKind::Valve(p.valve.clone()));
                ids.push(e.id);
                tx = tx.insert(e);
            }
            match self.doc.commit(tx) {
                Ok(_) => {
                    self.selection.clear();
                    self.selection.extend(ids, true);
                    self.status = format!("{code}: {}.", report(&f));
                }
                Err(e) => self.status = format!("Couldn't do that: {e}"),
            }
        }
        self.auto_valves = if open { Some(o) } else { None };
    }
}
