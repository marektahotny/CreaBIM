//! The fittings schedule: the elbows, tees, branches, reducers and
//! transitions the pipes drawn need where they meet
//! ([`creabim_services::fittings`]), counted by system.

use super::CreaBimApp;
use creabim_services::fittings::{self, Fitting};
use eframe::egui::{self, Vec2};

impl CreaBimApp {
    /// `calc:fittings` from the Plumbing tab: find them and show the window.
    pub(super) fn fittings_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" || arg != "fittings" {
            return false;
        }
        let found = fittings::derive(self.doc.model());
        self.status = if found.is_empty() {
            crate::i18n::tr("No pipes meet yet: there are no fittings to list.")
        } else {
            format!("{}: {}", crate::i18n::tr("Fittings"), found.len())
        };
        self.fittings = Some(found);
        true
    }

    /// The schedule's window, while it is open.
    pub(super) fn fittings_window(&mut self, ctx: &egui::Context) {
        let Some(found) = &self.fittings else {
            return;
        };
        let t = crate::i18n::tr;
        // by kind, system and size: the valves themselves
        let mut valves: std::collections::BTreeMap<
            (&'static str, String, u32),
            Vec<creabim_core::ElementId>,
        > = Default::default();
        for e in self.doc.model().iter() {
            let creabim_core::ElementKind::Valve(v) = &e.kind else {
                continue;
            };
            let (Some(k), Some(p)) = (v.kind_def(), v.host(self.doc.model())) else {
                continue;
            };
            valves
                .entry((k.name, p.system.clone(), p.nominal_mm.round() as u32))
                .or_default()
                .push(e.id);
        }
        let mut open = true;
        let mut again = false;
        let mut select: Option<Vec<creabim_core::ElementId>> = None;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Fittings schedule"))
            .id(egui::Id::new("fittings.schedule"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(280.0, 220.0))
            .default_size(Vec2::new(560.0, 440.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button(t("Find again")).clicked() {
                        again = true;
                    }
                    ui.weak(t(
                        "Found from how the pipes meet; pipe that is bent takes no elbow.",
                    ));
                });
                if found.is_empty() && valves.is_empty() {
                    ui.label(t("No pipes meet yet: there are no fittings to list."));
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let lines = fittings::summary(found);
                    let mut system = String::new();
                    egui::Grid::new("fittings.grid")
                        .num_columns(3)
                        .striped(true)
                        .spacing([14.0, 4.0])
                        .show(ui, |ui| {
                            for l in &lines {
                                if l.system != system {
                                    system = l.system.clone();
                                    let name = creabim_core::mep::system(&system)
                                        .map_or(system.clone(), |s| {
                                            format!("{} – {}", s.code, t(s.name))
                                        });
                                    ui.strong(name);
                                    ui.strong(t("Code"));
                                    ui.strong(t("Pieces"));
                                    ui.end_row();
                                }
                                // a click picks the pipes its fittings join
                                if ui.link(&l.label).clicked() {
                                    select = Some(
                                        found
                                            .iter()
                                            .filter(|f: &&Fitting| {
                                                f.system == l.system && f.label == l.label
                                            })
                                            .flat_map(|f| f.pipes.iter().copied())
                                            .collect(),
                                    );
                                }
                                ui.label(&l.code);
                                ui.label(l.count.to_string());
                                ui.end_row();
                            }
                        });
                    // the insulation the pipes need (PilonDesigner's)
                    let ins = fittings::insulation(self.doc.model(), 5.0);
                    if !ins.is_empty() {
                        ui.add_space(8.0);
                        egui::Grid::new("fittings.insulation")
                            .num_columns(4)
                            .striped(true)
                            .spacing([14.0, 4.0])
                            .show(ui, |ui| {
                                ui.strong(t("Pipe insulation"));
                                ui.strong(t("System"));
                                ui.strong(t("Length, m"));
                                ui.strong(t("With 5 % for offcuts, m"));
                                ui.end_row();
                                for l in &ins {
                                    let what = if l.heat {
                                        t("heat (λ ≤ 0,035 W/m·K)")
                                    } else {
                                        t("against condensation (rubber / PE)")
                                    };
                                    let text = format!(
                                        "{} {what}, Ø {} mm, {} {} mm",
                                        t("Insulation"),
                                        crate::i18n::num(l.outer_mm, 0),
                                        t("thickness"),
                                        crate::i18n::num(l.mm, 0)
                                    );
                                    if ui.link(text).clicked() {
                                        select = Some(l.pipes.clone());
                                    }
                                    ui.label(&l.system);
                                    ui.label(crate::i18n::num(l.length, 2));
                                    ui.label(crate::i18n::num(l.gross, 2));
                                    ui.end_row();
                                }
                            });
                    }
                    // the valves and accessories put on the pipes
                    if !valves.is_empty() {
                        ui.add_space(8.0);
                        egui::Grid::new("fittings.valves")
                            .num_columns(3)
                            .striped(true)
                            .spacing([14.0, 4.0])
                            .show(ui, |ui| {
                                ui.strong(t("Valves and accessories"));
                                ui.strong(t("System"));
                                ui.strong(t("Pieces"));
                                ui.end_row();
                                for ((name, system, dn), ids) in &valves {
                                    if ui.link(format!("{} DN {dn}", t(name))).clicked() {
                                        select = Some(ids.clone());
                                    }
                                    ui.label(system);
                                    ui.label(ids.len().to_string());
                                    ui.end_row();
                                }
                            });
                    }
                });
            });
        if let Some(ids) = select {
            self.selection.clear();
            self.selection.extend(ids, true);
        }
        if again {
            self.fittings = Some(fittings::derive(self.doc.model()));
        } else if !open {
            self.fittings = None;
        }
    }
}
