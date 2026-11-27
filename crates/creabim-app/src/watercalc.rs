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

/// What the calculation found, for the model.
fn water_results(calcs: &[SystemCalc]) -> creabim_core::results::ResultSet {
    let mut set =
        creabim_core::results::ResultSet::new("water", "Vodovod", "STN EN 806-3, STN 73 6655");
    for c in calcs {
        creabim_services::calcresults::water(&mut set, c.code, &c.net, &c.result);
    }
    set
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
        let mut write_now = false;
        let found = water_results(calcs);
        let written = self.is_written(&found);
        let mut select = None;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Water supply calculation"))
            .id(egui::Id::new("water.calc"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(380.0, 300.0))
            .default_size(crate::app::shell::at_least(Vec2::new(760.0, 600.0)))
            .min_width(640.0)
            .constrain_to(area)
            .show(ctx, |ui| {
                if super::calcresultsui::write_button(ui, written) {
                    write_now = true;
                }
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
                        .on_hover_text(crate::i18n::tr(&format!("{}: {changes}", t("Pipes to change"))))
                        .clicked()
                    {
                        apply = true;
                    }
                });
                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                    for c in calcs {
                        let name = creabim_core::mep::system(c.code)
                            .map_or(c.code.to_string(), |s| format!("{} – {}", s.code, t(s.name)));
                        ui.add_space(6.0);
                        ui.label(egui::RichText::new(name).size(15.0).strong());
                        ui.add_space(4.0);
                        let r = &c.result;
                        let fixtures = c
                            .net
                            .net
                            .nodes
                            .iter()
                            .filter(|n| n.fixture.is_some())
                            .count();
                        // the figures side by side, each in its own card
                        let short = r.shortfall_pa();
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                            let mut cards = vec![
                                ("Fixtures", fixtures.to_string()),
                                ("Design flow", crate::i18n::num(r.root_flow_ls, 2) + " l/s"),
                                ("Pressure needed", kpa(r.required_pa)),
                                ("Pressure available", kpa(r.available_pa)),
                            ];
                            if let Some(k) = r.reducer_setting_kpa {
                                cards.push(("Pressure reducer", kpa(k * 1000.0)));
                            }
                            for (label, value) in cards {
                                egui::Frame::group(ui.style())
                                    .inner_margin(egui::Margin::symmetric(12, 8))
                                    .corner_radius(6.0)
                                    .show(ui, |ui| {
                                        ui.vertical(|ui| {
                                            ui.set_min_width(104.0);
                                            ui.label(egui::RichText::new(t(label)).weak().size(11.0));
                                            ui.label(egui::RichText::new(value).size(16.0).strong());
                                        });
                                    });
                            }
                        });
                        ui.add_space(4.0);
                        // the verdict on a line of its own, in colour
                        let (color, verdict) = if short > 0.0 {
                            (
                                ui.visuals().error_fg_color,
                                format!("✖  {} {}", t("Short by"), kpa(short)),
                            )
                        } else {
                            (
                                egui::Color32::from_rgb(70, 170, 100),
                                format!("✔  {}", t("Enough pressure")),
                            )
                        };
                        ui.label(egui::RichText::new(verdict).color(color).strong());
                        let warn = |ui: &mut egui::Ui, s: String| {
                            ui.colored_label(ui.visuals().warn_fg_color, format!("⚠  {s}"));
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
                        ui.add_space(8.0);
                        // the stretches: names left, numbers right, a
                        // size to change in the accent
                        let accent = ui.visuals().selection.stroke.color;
                        use crate::sheet::{self, Col};
                        let cols = [
                            Col::num("#", ""),
                            Col::text("Pipe"),
                            Col::text("Size"),
                            Col::text("Proposed"),
                            Col::num("Flow", "l/s"),
                            Col::num("v", "m/s"),
                            Col::num("R", "Pa/m"),
                        ];
                        sheet::show(ui, ("water.calc.pipes", c.code), &cols, c.pipes.len(), 420.0, |row, n| {
                            let (id, p) = &c.pipes[n];
                            row.weak((n + 1).to_string());
                            let name = model.get(*id).map_or(String::new(), |e| e.name.clone());
                            let label = if p.on_critical_path {
                                egui::RichText::new(name).strong()
                            } else {
                                egui::RichText::new(name)
                            };
                            row.cell(|ui| {
                                if ui
                                    .link(label)
                                    .on_hover_text(t("Select the pipe in the drawing"))
                                    .clicked()
                                {
                                    select = Some(*id);
                                }
                            });
                            let now = size_text(model, *id, None);
                            let proposed = size_text(model, *id, p.proposed.map(|s| s.nominal_mm));
                            row.text(now.clone());
                            if proposed != now {
                                row.text(egui::RichText::new(proposed).color(accent).strong());
                            } else {
                                row.weak(proposed);
                            }
                            row.text(crate::i18n::num(p.flow_ls, 3));
                            row.text(crate::i18n::num(p.velocity_ms, 2));
                            row.text(crate::i18n::num(p.r_pa_per_m, 0));
                        }, None);
                        ui.add_space(10.0);
                        ui.separator();
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
        if write_now {
            let set = water_results(self.water_calc.as_deref().unwrap_or(&[]));
            self.write_results_said(set);
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
