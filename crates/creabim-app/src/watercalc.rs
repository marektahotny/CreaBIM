//! The water supply calculation of what is drawn: the cold and hot water
//! pipes and the fixtures they reach, sized by PilonHydraulika's method
//! (STN EN 806-3, STN 73 6655), in a window with each pipe's flow and
//! proposed size and a button that takes the proposals into the model.

use super::CreaBimApp;
use creabim_core::{ElementId, ElementKind, Model, Transaction};
use creabim_services::model::{self as net, ModelNetwork, PipeResult};
use creabim_services::{Settings, SystemKind, WaterResult};
use eframe::egui::{self, Vec2};

/// One system's calculation.
pub struct SystemCalc {
    pub code: &'static str,
    pub result: WaterResult,
    pub net: ModelNetwork,
    pub pipes: Vec<(ElementId, PipeResult)>,
}

/// The cold and hot water systems drawn, calculated.
pub fn calculate(model: &Model) -> Vec<SystemCalc> {
    let s = Settings::default();
    [("SV", SystemKind::ColdWater), ("TV", SystemKind::HotWater)]
        .into_iter()
        .filter_map(|(code, kind)| {
            let mut n = net::build(model, code, &s)?;
            let result = creabim_services::water(&s, &mut n.net, kind, true, None);
            let pipes = net::pipe_results(&n);
            Some(SystemCalc {
                code,
                result,
                net: n,
                pipes,
            })
        })
        .collect()
}

/// The edits that give every calculated pipe its proposed size.
pub fn apply_sizes(model: &Model, calcs: &[SystemCalc]) -> Option<Transaction> {
    let mut tx = Transaction::new("Apply proposed pipe sizes");
    let mut any = false;
    for (id, r) in calcs.iter().flat_map(|c| &c.pipes) {
        let (Some(size), Some(e)) = (r.proposed, model.get(*id)) else {
            continue;
        };
        let ElementKind::Pipe(p) = &e.kind else {
            continue;
        };
        if (p.nominal_mm - size.nominal_mm).abs() < 1e-6 {
            continue;
        }
        let Some(t) = creabim_services::pipe_types::by_id(&p.pipe_type) else {
            continue;
        };
        let mut after = e.clone();
        if let ElementKind::Pipe(q) = &mut after.kind {
            crate::pipesui::set_size(q, t, size.nominal_mm);
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

fn kpa(pa: f64) -> String {
    crate::i18n::num(pa / 1000.0, 1) + " kPa"
}

impl CreaBimApp {
    /// `calc:water` from the Plumbing tab: calculate and show the window.
    pub(super) fn calc_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" || arg != "water" {
            return false;
        }
        let calcs = calculate(self.doc.model());
        self.status = if calcs.is_empty() {
            crate::i18n::tr("No cold or hot water pipes are drawn to calculate.")
        } else {
            crate::i18n::tr("Water supply calculated")
        };
        self.water_calc = Some(calcs);
        true
    }

    /// The calculation's window, while there is one.
    pub(super) fn water_calc_window(&mut self, ctx: &egui::Context) {
        let Some(calcs) = &self.water_calc else {
            return;
        };
        let t = crate::i18n::tr;
        let model = self.doc.model();
        let mut open = true;
        let mut apply = false;
        let mut again = false;
        let mut select = None;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Water supply calculation"))
            .id(egui::Id::new("water.calc"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(300.0, 250.0))
            .default_size(Vec2::new(600.0, 500.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                if calcs.is_empty() {
                    ui.label(t("Draw cold (SV) or hot (TV) water pipes to the fixtures' connectors, then calculate."));
                }
                ui.horizontal(|ui| {
                    if ui.button(t("Calculate again")).clicked() {
                        again = true;
                    }
                    let changes = calcs
                        .iter()
                        .flat_map(|c| &c.pipes)
                        .filter(|(id, r)| {
                            matches!(
                                (r.proposed, model.get(*id).map(|e| &e.kind)),
                                (Some(s), Some(ElementKind::Pipe(p))) if (p.nominal_mm - s.nominal_mm).abs() > 1e-6
                            )
                        })
                        .count();
                    if ui
                        .add_enabled(changes > 0, egui::Button::new(t("Apply proposed sizes")))
                        .on_hover_text(format!("{}: {changes}", t("Pipes to change")))
                        .clicked()
                    {
                        apply = true;
                    }
                });
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for c in calcs {
                        let name = creabim_core::mep::system(c.code)
                            .map_or(c.code.to_string(), |s| format!("{} – {}", s.code, t(s.name)));
                        crate::props::section(ui, &name);
                        let r = &c.result;
                        let fixtures = c
                            .net
                            .net
                            .nodes
                            .iter()
                            .filter(|n| n.fixture.is_some())
                            .count();
                        crate::props::row(ui, "Fixtures", |ui| ui.label(fixtures.to_string()));
                        crate::props::row(ui, "Design flow", |ui| {
                            ui.label(crate::i18n::num(r.root_flow_ls, 2) + " l/s")
                        });
                        crate::props::row(ui, "Pressure needed", |ui| ui.label(kpa(r.required_pa)));
                        crate::props::row(ui, "Pressure available", |ui| ui.label(kpa(r.available_pa)));
                        let short = r.shortfall_pa();
                        crate::props::row(ui, "Result", |ui| {
                            if short > 0.0 {
                                ui.colored_label(
                                    ui.visuals().error_fg_color,
                                    format!("{} {}", t("Short by"), kpa(short)),
                                )
                            } else {
                                ui.label(t("Enough pressure"))
                            }
                        });
                        if let Some(k) = r.reducer_setting_kpa {
                            crate::props::row(ui, "Pressure reducer", |ui| {
                                ui.label(format!("{} {}", t("set to"), kpa(k * 1000.0)))
                            });
                        }
                        let warn = |ui: &mut egui::Ui, s: String| {
                            ui.colored_label(ui.visuals().warn_fg_color, s);
                        };
                        if !c.net.open_ends.is_empty() {
                            warn(ui, format!("{}: {}", t("Pipe ends joined to nothing"), c.net.open_ends.len()));
                        }
                        if !c.net.unreached.is_empty() {
                            warn(ui, format!("{}: {}", t("Pipes not joined to the network"), c.net.unreached.len()));
                        }
                        if c.net.loops > 0 {
                            warn(ui, format!("{}: {}", t("Loops left out"), c.net.loops));
                        }
                        for n in &r.notes {
                            if let creabim_services::water::Note::UnmatchedFixtures(v) = n {
                                warn(ui, format!("{}: {}", t("Fixtures counted as unknown"), v.join(", ")));
                            }
                        }
                        egui::Grid::new(("water.calc.pipes", c.code))
                            .striped(true)
                            .num_columns(6)
                            .show(ui, |ui| {
                                for h in ["Pipe", "Size", "Proposed", "Flow l/s", "v m/s", "R Pa/m"] {
                                    ui.strong(t(h));
                                }
                                ui.end_row();
                                for (id, p) in &c.pipes {
                                    let name = model.get(*id).map_or(String::new(), |e| e.name.clone());
                                    let label = if p.on_critical_path {
                                        egui::RichText::new(name).strong()
                                    } else {
                                        egui::RichText::new(name)
                                    };
                                    if ui.link(label).clicked() {
                                        select = Some(*id);
                                    }
                                    ui.label(size_text(model, *id, None));
                                    ui.label(size_text(model, *id, p.proposed.map(|s| s.nominal_mm)));
                                    ui.label(crate::i18n::num(p.flow_ls, 3));
                                    ui.label(crate::i18n::num(p.velocity_ms, 2));
                                    ui.label(crate::i18n::num(p.r_pa_per_m, 0));
                                    ui.end_row();
                                }
                            });
                    }
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(t(
                            "Bold: the critical path. The root is the lowest free pipe end, where the supply comes in.",
                        ))
                        .weak(),
                    );
                });
            });
        if let Some(id) = select {
            self.selection.only(id);
        }
        if apply {
            if let Some(tx) =
                apply_sizes(self.doc.model(), self.water_calc.as_deref().unwrap_or(&[]))
            {
                if let Err(e) = self.doc.commit(tx) {
                    self.status = e.to_string();
                }
            }
            again = true;
        }
        if again {
            self.water_calc = Some(calculate(self.doc.model()));
        }
        if !open {
            self.water_calc = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_model_with_no_water_pipes_has_nothing_to_calculate() {
        let m = creabim_core::sample::sample_house();
        assert!(calculate(&m).is_empty());
        assert!(apply_sizes(&m, &[]).is_none());
    }
}
