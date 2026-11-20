//! Check the routes drawn ([`creabim_core::routecheck`]): pipes and ducts
//! through beams and lintels, gas too near other pipes, passages into
//! another fire zone. A level pipe through a beam can be led under it in
//! one step, to undo as one.

use super::CreaBimApp;
use creabim_core::routecheck::{self, Problem};
use creabim_core::ElementKind;
use eframe::egui::{self, Vec2};

/// The window's state: the air left under a beam a pipe is led under, m.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RouteCheck {
    pub gap: f64,
}

impl Default for RouteCheck {
    fn default() -> Self {
        // as TPH 03 2012 F.5.9 keeps gas off walls; editable
        RouteCheck { gap: 0.02 }
    }
}

impl CreaBimApp {
    /// `routes:check` from the services tabs: open the window.
    pub(super) fn route_check_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "routes" || arg != "check" {
            return false;
        }
        if self.route_check.is_none() {
            self.route_check = Some(RouteCheck::default());
        }
        true
    }

    /// The window, while it is open.
    pub(super) fn route_check_window(&mut self, ctx: &egui::Context) {
        let Some(mut st) = self.route_check else {
            return;
        };
        let t = crate::i18n::tr;
        let mut open = true;
        let mut show: Option<creabim_core::ElementId> = None;
        let mut lead: Option<(creabim_core::ElementId, creabim_core::ElementId)> = None;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        let found = routecheck::check(self.doc.model());
        let name = |id| {
            self.doc
                .model()
                .get(id)
                .map_or_else(String::new, |e| match &e.kind {
                    ElementKind::Pipe(p) => {
                        format!("{} {}", p.system, crate::pipesui::size_label(p))
                    }
                    _ => e.name.clone(),
                })
        };
        super::shell::window(t("Check routes"))
            .id(egui::Id::new("routes.check"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(240.0, 200.0))
            .default_size(Vec2::new(480.0, 380.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(t("Air under a beam a pipe is led under, mm"));
                    let mut mm = st.gap * 1000.0;
                    if ui
                        .add(egui::DragValue::new(&mut mm).range(0.0..=200.0).speed(1.0))
                        .changed()
                    {
                        st.gap = mm / 1000.0;
                    }
                });
                ui.separator();
                if found.is_empty() {
                    ui.weak(t("Nothing found on the routes drawn."));
                }
                egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                    for c in &found {
                        ui.horizontal_wrapped(|ui| {
                            let what = match &c.problem {
                                Problem::Beam { beam, lintel } => format!(
                                    "{}: {} {}",
                                    name(c.pipe),
                                    t(if *lintel {
                                        "runs through a lintel"
                                    } else {
                                        "runs through a beam"
                                    }),
                                    name(*beam)
                                ),
                                Problem::GasGap { other, gap } => format!(
                                    "{}: {} {} ({} mm, TPH 03 2012 F.5.7: {} mm)",
                                    name(c.pipe),
                                    t("too near"),
                                    name(*other),
                                    crate::i18n::num(gap * 1000.0, 0),
                                    crate::i18n::num(routecheck::GAS_GAP * 1000.0, 0)
                                ),
                                Problem::FireZone { from, to } => format!(
                                    "{}: {} {from} → {to}",
                                    name(c.pipe),
                                    t("passes into another fire zone: fire-stop it, a duct with a fire damper")
                                ),
                            };
                            ui.label(what);
                            if ui.small_button(t("Show")).clicked() {
                                show = Some(c.pipe);
                            }
                            if let Problem::Beam { beam, .. } = c.problem {
                                if ui
                                    .small_button(t("Lead under it"))
                                    .on_hover_text(t("Down in front of the beam, along under it and up behind it: the drop and the rise are no stack, the elbows and the length go into the calculation"))
                                    .clicked()
                                {
                                    lead = Some((c.pipe, beam));
                                }
                            }
                        });
                    }
                });
                ui.add_space(4.0);
                ui.weak(t("Air vents at high points, drain cocks at low points and gas sleeves, thermal shut-offs and fire stopping are proposed in Propose valves."));
            });
        if let Some(id) = show {
            self.selection.only(id);
            self.frame_selection();
        }
        if let Some((pipe, beam)) = lead {
            match routecheck::bypass(self.doc.model(), pipe, beam, st.gap) {
                Some(tx) => match self.doc.commit(tx) {
                    Ok(_) => {
                        self.selection.only(pipe);
                        self.status = t("Led the pipe under the beam.");
                    }
                    Err(e) => self.status = format!("Couldn't do that: {e}"),
                },
                None => {
                    self.status =
                        t("Only a level pipe above the beam's underside can be led under it.")
                }
            }
        }
        self.route_check = if open { Some(st) } else { None };
    }
}
