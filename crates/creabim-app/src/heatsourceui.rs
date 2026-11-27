//! Heat sources from PilonDesigner's library
//! ([`creabim_core::catalogue::heat`]): a hot water cylinder, a gas
//! condensing boiler or a heat pump's units, picked from their range by
//! the heat load (a heat pump by its bivalence point,
//! [`creabim_services::hpsizing`]), then placed as any catalogue object.

use super::CreaBimApp;
use creabim_core::catalogue::heat::{self, HeatKind};
use creabim_services::hpsizing::{self, HpInput};
use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};

/// What the window is set to.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct HeatSource {
    pub kind: HeatKind,
    pub size: u32,
    pub input: HpInput,
}

impl Default for HeatSource {
    fn default() -> Self {
        Self {
            kind: HeatKind::HpOutdoor,
            size: 9,
            input: HpInput::default(),
        }
    }
}

fn is_heat_pump(k: HeatKind) -> bool {
    k.is_heat_pump()
}

/// The outdoor unit whose curve sizes heat pump `k`: Midea's own, the
/// Panasonic kits for the rest (the indoor units belong to them).
pub(super) fn curve_kind(k: HeatKind) -> HeatKind {
    if k.is_midea() {
        HeatKind::HpOutdoorMidea
    } else {
        HeatKind::HpOutdoor
    }
}

/// The bivalence diagram: the building's need, the range's curves, the
/// one taken, θe and the bivalence point.
fn diagram(ui: &mut egui::Ui, i: &HpInput, kind: HeatKind, sel: u32) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 200.0), egui::Sense::hover());
    let p = ui.painter_at(rect);
    let ink = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let plot = egui::Rect::from_min_max(
        rect.min + Vec2::new(34.0, 8.0),
        rect.max - Vec2::new(10.0, 22.0),
    );
    let t0 = (-20.0_f64).min((i.te / 5.0).floor() * 5.0 - 5.0);
    let t1 = hpsizing::T_HEAT_LIMIT + 5.0;
    // the kits whose catalogue curve is known (A-7 to A7)
    let sizes: Vec<u32> = kind
        .sizes()
        .iter()
        .copied()
        .filter(|s| heat::has_curve_of(kind, *s))
        .collect();
    let top = sizes
        .iter()
        .map(|s| {
            let pts = heat::curve_of(kind, *s, i.t_flow);
            pts.iter().map(|x| x.1).fold(0.0, f64::max)
        })
        .fold(hpsizing::need(i, t0), f64::max);
    let qmax = (top / 2.0).ceil() * 2.0 + 2.0;
    let at = |t: f64, q: f64| {
        Pos2::new(
            plot.left() + ((t - t0) / (t1 - t0)) as f32 * plot.width(),
            plot.bottom() - (q / qmax) as f32 * plot.height(),
        )
    };
    let font = egui::FontId::proportional(10.0);
    let mut t = t0;
    while t <= t1 + 1e-9 {
        p.line_segment(
            [at(t, 0.0), at(t, qmax)],
            Stroke::new(0.5, weak.gamma_multiply(0.35)),
        );
        p.text(
            at(t, 0.0) + Vec2::new(0.0, 3.0),
            egui::Align2::CENTER_TOP,
            format!("{t:.0}"),
            font.clone(),
            weak,
        );
        t += 5.0;
    }
    let step = if qmax > 24.0 { 4.0 } else { 2.0 };
    let mut q = 0.0;
    while q <= qmax + 1e-9 {
        p.line_segment(
            [at(t0, q), at(t1, q)],
            Stroke::new(0.5, weak.gamma_multiply(0.35)),
        );
        p.text(
            at(t0, q) - Vec2::new(4.0, 0.0),
            egui::Align2::RIGHT_CENTER,
            format!("{q:.0}"),
            font.clone(),
            weak,
        );
        q += step;
    }
    let line = |f: &dyn Fn(f64) -> f64, stroke: Stroke| {
        let n = ((t1 - t0) * 2.0) as usize;
        let pts: Vec<Pos2> = (0..=n)
            .map(|k| {
                let t = t0 + k as f64 * 0.5;
                at(t, f(t))
            })
            .collect();
        p.add(egui::Shape::line(pts, stroke));
    };
    let blue = Color32::from_rgb(37, 99, 235);
    for s in &sizes {
        let pts = heat::curve_of(kind, *s, i.t_flow);
        if *s != sel {
            line(&|t| hpsizing::capacity(&pts, t), Stroke::new(1.0, weak));
        }
        p.text(
            at(t1, hpsizing::capacity(&pts, t1)) - Vec2::new(2.0, 1.0),
            egui::Align2::RIGHT_BOTTOM,
            s.to_string(),
            font.clone(),
            if *s == sel { blue } else { weak },
        );
    }
    line(
        &|t| hpsizing::need(i, t),
        Stroke::new(2.0, Color32::from_rgb(220, 38, 38)),
    );
    let pts = heat::curve_of(kind, sel, i.t_flow);
    line(&|t| hpsizing::capacity(&pts, t), Stroke::new(2.4, blue));
    p.line_segment([at(i.te, 0.0), at(i.te, qmax)], Stroke::new(1.0, blue));
    p.text(
        at(i.te, qmax) + Vec2::new(4.0, 0.0),
        egui::Align2::LEFT_TOP,
        format!("θe = {:.0} °C", i.te),
        font.clone(),
        blue,
    );
    if let Some(tb) = hpsizing::size(i, &pts).t_biv {
        p.circle(
            at(tb, hpsizing::need(i, tb)),
            4.5,
            Color32::from_rgb(245, 158, 11),
            Stroke::new(1.0, ink),
        );
    }
    p.text(
        Pos2::new(plot.center().x, rect.bottom()),
        egui::Align2::CENTER_BOTTOM,
        "θ [°C] · [kW]",
        font,
        weak,
    );
}

impl CreaBimApp {
    /// `heat:source` from the Heating tab: open the window.
    pub(super) fn heat_source_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "heat" || arg != "source" {
            return false;
        }
        if self.heat_source.is_none() {
            // its power from the heat loss of the model, not a fixed one
            let mut c = HeatSource::default();
            if let Some(i) = self.hp_input_from_model() {
                c.input = i;
                c.size = hpsizing::recommend_of(curve_kind(c.kind), &i);
            }
            self.heat_source = Some(c);
        }
        true
    }

    /// The heat pump's input from the model's heat loss (STN EN 12831-1,
    /// with the project's locality): none without rooms.
    pub(super) fn hp_input_from_model(&self) -> Option<HpInput> {
        let o = self.heat_loss.as_ref().map_or_else(
            || self.site_opts(creabim_services::heatloss::Opts::default()),
            |x| x.0,
        );
        let b = creabim_services::heatloss::building(self.doc.model(), &o);
        if b.rooms.is_empty() || b.phi <= 0.0 {
            return None;
        }
        let mut i = self
            .heat_source
            .as_ref()
            .map_or_else(HpInput::default, |c| c.input);
        i.heat_load = (b.phi / 1000.0).clamp(0.5, 200.0);
        i.te = o.te.clamp(-25.0, 0.0);
        Some(i)
    }

    /// A heat pump unit from the catalogue, sized by the model's heat
    /// loss (its bivalence point) when it has rooms: the catalogue's own
    /// size only stands in until then.
    pub(super) fn sized_heat_unit(
        &self,
        c: creabim_core::objects::CatalogueObject,
    ) -> creabim_core::objects::CatalogueObject {
        let kind = [
            HeatKind::HpOutdoor,
            HeatKind::HpOutdoorMidea,
            HeatKind::HpIndoor,
            HeatKind::HpIndoorCylinder,
        ]
        .into_iter()
        .find(|k| {
            c.key
                .strip_prefix(&format!("heat-{}-", k.id().to_lowercase()))
                .is_some_and(|n| n.parse::<u32>().is_ok())
        });
        match (kind, self.hp_input_from_model()) {
            (Some(k), Some(i)) => heat::unit(k, hpsizing::recommend_of(curve_kind(k), &i)),
            _ => c,
        }
    }

    /// The Midea outdoor unit's size the catalogue's Midea block places,
    /// and whether it is the heat loss's own: the one picked in the block,
    /// else the heat source window's Midea size, else the size the model's
    /// heat loss recommends, else 8 kW.
    pub(super) fn midea_size(&self) -> (u32, bool) {
        if let Some(s) = self.midea_out {
            return (HeatKind::HpOutdoorMidea.size(s), false);
        }
        if let Some(c) = self.heat_source.as_ref().filter(|c| c.kind.is_midea()) {
            return (HeatKind::HpOutdoorMidea.size(c.size), true);
        }
        match self.hp_input_from_model() {
            Some(i) => (hpsizing::recommend_of(HeatKind::HpOutdoorMidea, &i), true),
            None => (8, false),
        }
    }

    /// The Midea indoor unit's kind the block places: the one picked in
    /// the block, else the heat source window's Midea indoor unit, else
    /// the hydronic box. The 190 l tank goes with 4–10 kW only: over that,
    /// the 240 l one.
    pub(super) fn midea_indoor(&self, size: u32) -> HeatKind {
        let k = self
            .midea_in
            .or_else(|| {
                self.heat_source
                    .as_ref()
                    .map(|c| c.kind)
                    .filter(|k| k.is_midea() && *k != HeatKind::HpOutdoorMidea)
            })
            .unwrap_or(HeatKind::HpIndoorMidea);
        if k == HeatKind::HpIndoorMideaTank190 && size > 10 {
            HeatKind::HpIndoorMideaTank240
        } else {
            k
        }
    }

    /// The window, while it is open.
    pub(super) fn heat_source_window(&mut self, ctx: &egui::Context) {
        let Some(mut c) = self.heat_source.clone() else {
            return;
        };
        let t = crate::i18n::tr;
        let num = crate::i18n::num;
        let mut open = true;
        let mut place = false;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Heat source"))
            .id(egui::Id::new("heat.source"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(240.0, 300.0))
            .default_size(crate::app::shell::at_least(Vec2::new(480.0, 600.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                egui::ComboBox::from_id_salt("heat.source.kind")
                    .width(ui.available_width() - 8.0)
                    .selected_text(c.kind.name())
                    .show_ui(ui, |ui| {
                        for k in HeatKind::ALL {
                            if ui.selectable_label(c.kind == k, k.name()).clicked() && c.kind != k {
                                let was_hp = is_heat_pump(c.kind);
                                c.kind = k;
                                c.size = match k {
                                    HeatKind::Cylinder => 200,
                                    HeatKind::Boiler => 20,
                                    HeatKind::BoilerCombi => 25,
                                    _ if was_hp => c.size,
                                    _ => 8,
                                };
                            }
                        }
                    });
                c.size = c.kind.size(c.size);
                let hp = is_heat_pump(c.kind);
                let boiler = matches!(c.kind, HeatKind::Boiler | HeatKind::BoilerCombi);
                let mut pick = None;
                egui::Grid::new("heat.source.grid")
                    .num_columns(2)
                    .spacing([14.0, 5.0])
                    .show(ui, |ui| {
                        let i = &mut c.input;
                        if hp || boiler {
                            ui.label(t("Design heat load"));
                            ui.add(
                                egui::DragValue::new(&mut i.heat_load)
                                    .range(0.5..=200.0)
                                    .speed(0.1)
                                    .suffix(" kW"),
                            );
                            ui.end_row();
                        }
                        if hp {
                            ui.label(t("Design outside temperature"));
                            ui.add(
                                egui::DragValue::new(&mut i.te)
                                    .range(-25.0..=0.0)
                                    .speed(0.5)
                                    .suffix(" °C"),
                            );
                            ui.end_row();
                            ui.label(t("Flow temperature"));
                            ui.add(
                                egui::DragValue::new(&mut i.t_flow)
                                    .range(25.0..=60.0)
                                    .speed(0.5)
                                    .suffix(" °C"),
                            );
                            ui.end_row();
                            ui.label(t("People (hot water)"));
                            ui.add(egui::DragValue::new(&mut i.persons).range(0.0..=200.0));
                            ui.end_row();
                            ui.label(t("Supply blocked"));
                            ui.add(
                                egui::DragValue::new(&mut i.block_h)
                                    .range(0.0..=8.0)
                                    .speed(0.1)
                                    .suffix(" h"),
                            );
                            ui.end_row();
                            ui.label(t("Bivalence point aimed at"));
                            ui.add(
                                egui::DragValue::new(&mut i.t_biv)
                                    .range(-20.0..=5.0)
                                    .speed(0.5)
                                    .suffix(" °C"),
                            );
                            ui.end_row();
                        }
                        ui.label(t("Size"));
                        ui.horizontal(|ui| {
                            egui::ComboBox::from_id_salt("heat.source.size")
                                .selected_text(format!("{} {}", c.size, c.kind.unit()))
                                .show_ui(ui, |ui| {
                                    for s in c.kind.sizes() {
                                        ui.selectable_value(
                                            &mut c.size,
                                            *s,
                                            format!("{s} {}", c.kind.unit()),
                                        );
                                    }
                                });
                            if hp {
                                let s = hpsizing::recommend_of(curve_kind(c.kind), &c.input);
                                if ui
                                    .button(format!("{}: {s} kW", t("By the bivalence point")))
                                    .clicked()
                                {
                                    pick = Some(s);
                                }
                            } else if boiler {
                                let s = c.kind.pick(c.input.heat_load);
                                if ui
                                    .button(format!("{}: {s} kW", t("By the heat load")))
                                    .clicked()
                                {
                                    pick = Some(s);
                                }
                            }
                        });
                        ui.end_row();
                    });
                if let Some(s) = pick {
                    c.size = s;
                }
                if hp {
                    let ck = curve_kind(c.kind);
                    let r = hpsizing::size(&c.input, &heat::curve_of(ck, c.size, c.input.t_flow));
                    let biv = match r.t_biv {
                        Some(tb) => format!("{} {} °C", t("bivalence point"), num(tb, 1)),
                        None => t("covers the need on its own (monovalent)"),
                    };
                    ui.label(format!(
                        "{} {} kW · {} {} kW · {} · {} {} kW",
                        t("Output at θe"),
                        num(r.q_te, 1),
                        t("need"),
                        num(r.need_te, 1),
                        biv,
                        t("electric heating"),
                        num(r.backup, 1),
                    ));
                    if let Some(bk) =
                        heat::backup_of(ck, c.size).filter(|bk| r.backup > bk + 1e-9)
                    {
                        ui.colored_label(
                            ui.visuals().warn_fg_color,
                            format!(
                                "{} ({} kW).",
                                t("The indoor unit's electric heater does not cover the rest"),
                                num(bk, 0)
                            ),
                        );
                    }
                    // the sizes with a catalogue curve down to A-7 that
                    // reach the bivalence point aimed at: none, say so
                    let best = hpsizing::recommend_of(ck, &c.input);
                    let rb = hpsizing::size(&c.input, &heat::curve_of(ck, best, c.input.t_flow));
                    if let Some(tb) = rb.t_biv.filter(|tb| *tb > c.input.t_biv + 1e-9) {
                        ui.colored_label(
                            ui.visuals().warn_fg_color,
                            format!(
                                "{} {} °C: {} {best} kW {} {} °C. {}",
                                t("No kit with the catalogue's output down to A-7 reaches the bivalence point aimed at"),
                                num(c.input.t_biv, 1),
                                t("the largest,"),
                                t("crosses the need at"),
                                num(tb, 1),
                                if ck == HeatKind::HpOutdoor {
                                    t("The 12 and 16 kW kits (H) have only their A7 output in the catalogue, so they cannot be sized by the bivalence point.")
                                } else {
                                    String::new()
                                },
                            ),
                        );
                    }
                    diagram(ui, &c.input, ck, c.size);
                    ui.weak("STN EN 12831-1 · STN EN 15450");
                }
                ui.separator();
                let u = heat::describe(c.kind, c.size);
                ui.label(egui::RichText::new(&u.name).strong());
                egui::ScrollArea::vertical()
                    .max_height(220.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        egui::Grid::new("heat.source.params")
                            .num_columns(2)
                            .striped(true)
                            .spacing([14.0, 3.0])
                            .show(ui, |ui| {
                                for (k, v) in &u.params {
                                    ui.label(k);
                                    ui.label(v);
                                    ui.end_row();
                                }
                                for port in &u.ports {
                                    ui.weak(port.name);
                                    ui.weak(if port.dn > 0 {
                                        format!("{} DN {}", port.code, port.dn)
                                    } else {
                                        port.code.to_string()
                                    });
                                    ui.end_row();
                                }
                            });
                    });
                ui.weak(t(if c.kind.is_midea() {
                    "Midea M thermal Arctic split: the maker's catalogue values (maximum heating capacity, EN 14511) down to -25 °C; between the table's points the output is interpolated in a line."
                } else if hp {
                    "Panasonic Aquarea bi-bloc J and H, all-in-one J: the maker's catalogue values. Under A-7 the catalogue prints no output: it is held at A-7."
                } else {
                    "Usual values of the market's ranges, to design with: replace them with the maker's once the product is chosen."
                }));
                ui.add_space(4.0);
                if ui.button(t("Place it")).clicked() {
                    place = true;
                }
            });
        if place {
            // the indoor unit first, then its outdoor one; the windows
            // out of the way meanwhile (crate::app::heatplace)
            let size = c.kind.size(c.size);
            let mi = self.midea_indoor(size);
            let units = super::heatplace::pair(c.kind, c.size, mi);
            self.heat_place_start(ctx, &units);
        }
        self.heat_source = if open { Some(c) } else { None };
    }
}
