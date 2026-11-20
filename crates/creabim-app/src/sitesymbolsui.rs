//! The cadastre's window of site symbols and networks: the networks of
//! the map key (6.152–6.732, 8.021) to draw, in the state they are in,
//! and the point symbols (6.120–6.580, the entrances) to place.

use super::egui;
use super::{CreaBimApp, Tool};
use creabim_core::utility::{NetKind, NetState, SiteMark, UtilityLine};
use creabim_geom::netlines::{self, Stroke};
use egui::Vec2;

/// A stroke list drawn into a small box, `mm` paper mm to a point.
fn preview(
    ui: &mut egui::Ui,
    strokes: &[Stroke],
    rgb: [f32; 3],
    size: Vec2,
    origin: egui::Pos2,
    weight: f32,
) {
    let c = egui::Color32::from_rgb(
        (rgb[0] * 255.0) as u8,
        (rgb[1] * 255.0) as u8,
        (rgb[2] * 255.0) as u8,
    );
    let p = ui.painter();
    let k = 2.4_f32;
    let to = |v: &glam::DVec2| origin + Vec2::new(v.x as f32 * k, -(v.y as f32) * k);
    for s in strokes {
        match s {
            Stroke::Line(pts) => {
                p.add(egui::Shape::line(
                    pts.iter().map(to).collect(),
                    egui::Stroke::new(weight, c),
                ));
            }
            Stroke::Fill(pts) => {
                p.add(egui::Shape::convex_polygon(
                    pts.iter().map(to).collect(),
                    c,
                    egui::Stroke::NONE,
                ));
            }
            Stroke::Text {
                at, text, size: h, ..
            } => {
                p.text(
                    to(at),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::proportional(*h as f32 * k * 1.1),
                    c,
                );
            }
        }
    }
    let _ = size;
}

impl CreaBimApp {
    /// The window, while it is open.
    pub(super) fn site_symbols_window(&mut self, ctx: &egui::Context) {
        if !self.site_symbols_open {
            return;
        }
        let t = crate::i18n::tr;
        let mut open = true;
        let mut pick_net = None;
        let mut pick_mark = None;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Site symbols and networks"))
            .id(egui::Id::new("site.symbols"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(240.0, 300.0))
            .default_size(Vec2::new(480.0, 620.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.strong(t("Networks"));
                    ui.horizontal_wrapped(|ui| {
                        let mut s = crate::sketch::net_state();
                        for st in NetState::ALL {
                            if ui.selectable_label(s == st, t(st.label())).clicked() {
                                s = st;
                            }
                        }
                        if s != crate::sketch::net_state() {
                            crate::sketch::set_net_state(s);
                        }
                    });
                    let state = crate::sketch::net_state();
                    let current = crate::sketch::network();
                    for k in NetKind::ALL {
                        let net = UtilityLine {
                            kind: k,
                            state,
                            note: String::new(),
                        };
                        ui.horizontal(|ui| {
                            let (r, _) = ui
                                .allocate_exact_size(Vec2::new(150.0, 22.0), egui::Sense::hover());
                            let strokes = netlines::run(
                                &net,
                                glam::DVec2::ZERO,
                                glam::DVec2::new(60.0, 0.0),
                                1.0,
                                true,
                            );
                            preview(
                                ui,
                                &strokes,
                                k.rgb(),
                                r.size(),
                                r.left_center() + Vec2::new(2.0, 0.0),
                                (net.weight() * 4.0) as f32,
                            );
                            let label = format!("{} {}", k.norm(), t(k.label()));
                            if ui.selectable_label(current == Some(k), label).clicked() {
                                pick_net = Some(k);
                            }
                        });
                    }
                    ui.separator();
                    ui.strong(t("Point symbols"));
                    let current = crate::sketch::mark();
                    for m in SiteMark::ALL {
                        ui.horizontal(|ui| {
                            let (r, _) =
                                ui.allocate_exact_size(Vec2::new(40.0, 30.0), egui::Sense::hover());
                            let strokes = netlines::mark(m, glam::DVec2::ZERO, glam::DVec2::Y, 1.0);
                            preview(ui, &strokes, m.rgb(), r.size(), r.center(), 1.2);
                            let label = match m.norm() {
                                Some(n) => format!("{n} {}", t(m.label())),
                                None => t(m.label()).to_string(),
                            };
                            if ui.selectable_label(current == Some(m), label).clicked() {
                                pick_mark = Some(m);
                            }
                        });
                    }
                });
            });
        if !open {
            self.site_symbols_open = false;
        }
        if pick_net.is_none() && pick_mark.is_none() {
            return;
        }
        let Some(i) = Tool::ALL.iter().position(|t| *t == Tool::Sketch) else {
            return;
        };
        self.run_action(ctx, &format!("tool:{i}"));
        self.shell.tab_tool = Some(Tool::Sketch);
        if let Some(k) = pick_net {
            crate::sketch::set_network(Some(k));
            self.status = t(
                "Utility network: click its bends, Enter or right-click ends it. Its kind (water, sewer, gas, power…), state and text are in Properties.",
            );
        } else if let Some(m) = pick_mark {
            crate::sketch::set_mark(Some(m));
            self.status = t("Site symbol: click its point, then the way it faces (an entrance: where it leads).");
        }
    }
}
