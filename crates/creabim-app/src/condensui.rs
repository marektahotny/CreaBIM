//! Water vapour condensation inside a build-up (Heating › Condensation
//! in a build-up): the Glaser profile of an assembly
//! ([`creabim_services::glaser`]) and its annual moisture balance by STN
//! 73 0540-2 annex B with the durations of STN 73 0540-3 table 4, checked
//! against čl. 6.1 (Mc ≤ its limit) and 6.2 (Mc < Mev).

use super::CreaBimApp;
use crate::sheet::{self, Col};
use creabim_core::fireparams::ParamTable;
use creabim_core::{ElementId, ElementKind, Model};
use creabim_services::glaser::{self, Annual, Conditions, Layer, Profile};
use creabim_services::thermcheck::{self, Psat};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

/// What the designer sets, and the last result.
#[derive(Clone, Debug)]
pub struct Condens {
    pub assembly: Option<ElementId>,
    pub ti: f64,
    /// %.
    pub phi_i: f64,
    /// The site's design θe (picks column A or B of table 4); NaN while
    /// there is none.
    pub te: f64,
    pub site: String,
    /// The kind of build-up of STN 73 0540-2 čl. 6.1.
    pub kind: String,
    /// The row of table 4 drawn.
    pub row: usize,
    pub result: Option<std::sync::Arc<Outcome>>,
    pub error: Option<String>,
}

impl Default for Condens {
    fn default() -> Self {
        Self {
            assembly: None,
            ti: 20.0,
            phi_i: f64::NAN,
            te: f64::NAN,
            site: String::new(),
            kind: String::new(),
            row: 0,
            result: None,
            error: None,
        }
    }
}

#[derive(Debug)]
pub struct Outcome {
    pub name: String,
    pub layers: Vec<Layer>,
    pub rsi: f64,
    pub rse: f64,
    pub annual: Annual,
    /// The profile of each row of table 4, for the drawing.
    pub profiles: Vec<Profile>,
    pub limit: Option<(f64, String)>,
}

pub fn solve(model: &Model, tables: &[ParamTable], s: &Condens) -> Result<Outcome, String> {
    let t = crate::i18n::tr;
    let id = s.assembly.ok_or_else(|| t("Choose a build-up."))?;
    let e = model.get(id).ok_or_else(|| t("Choose a build-up."))?;
    let ElementKind::Assembly(a) = &e.kind else {
        return Err(t("Choose a build-up."));
    };
    if s.te.is_nan() {
        return Err(t("θe of the locality is not set: enter it, or choose the project's locality (Project › Project information)."));
    }
    if s.phi_i.is_nan() {
        return Err(t("Enter φi."));
    }
    let psat = Psat::of(tables).ok_or("vzorce")?;
    let rs = thermcheck::surface_rs(tables, false).ok_or(thermcheck::RS)?;
    let (layers, rsi, rse) = glaser::assembly_layers(model, a, &rs, tables).map_err(|m| {
        if m == "not against outside air" {
            t("The build-up is not against outside air (set it in the build-up's U: On the outer side).")
        } else {
            format!("{}: {m}", t("Missing in a layer"))
        }
    })?;
    let annual = glaser::annual_sk(
        &layers,
        s.ti,
        s.phi_i / 100.0,
        (rsi, rse),
        s.te,
        tables,
        &psat,
    )?;
    let fine = glaser::slices(&layers, glaser::SLICES);
    let k = glaser::Constants {
        delta0: annual.delta_a,
        mu_metal: f64::INFINITY,
        r_max: f64::INFINITY,
    };
    let mut profiles = vec![];
    for r in &annual.rows {
        let c = Conditions {
            theta_i: s.ti,
            phi_i: s.phi_i / 100.0,
            theta_e: r.theta_e,
            phi_e: r.pe / psat.p(r.theta_e),
            rsi,
            rse,
        };
        profiles.push(glaser::profile(&fine, &c, &psat, &k)?);
    }
    let limit = thermcheck::condensation_kinds(tables)
        .into_iter()
        .find(|k| k.0 == s.kind)
        .map(|k| (k.1, k.2));
    Ok(Outcome {
        name: e.name.clone(),
        layers,
        rsi,
        rse,
        annual,
        profiles,
        limit,
    })
}

impl CreaBimApp {
    /// `calc:condensation` from the Heating tab.
    pub(super) fn condens_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" || arg != "condensation" {
            return false;
        }
        let mut s = self.condens.take().unwrap_or_default();
        let model = self.doc.model();
        let tables = thermcheck::tables_of(model);
        if s.phi_i.is_nan() {
            if let Some(p) = thermcheck::phi_i_condensation(&tables) {
                s.phi_i = p;
            }
        }
        if s.kind.is_empty() {
            s.kind = thermcheck::condensation_kinds(&tables)
                .into_iter()
                .map(|k| k.0)
                .find(|k| k == "ostatne")
                .unwrap_or_default();
        }
        if s.te.is_nan() {
            let name = model
                .project_info()
                .map(|(_, p)| p.climate_site.clone())
                .unwrap_or_default();
            if let Some(site) = self
                .climate_sites()
                .into_iter()
                .find(|c| c.name == name && !name.is_empty())
            {
                s.te = site.te;
                s.site = site.name;
            }
        }
        // a headless screenshot: the build-up by name
        if let Ok(n) = std::env::var("CREABIM_SHOT_CONDENS") {
            s.assembly = model.iter().find(|e| e.name == n).map(|e| e.id);
        }
        if s.assembly.is_some() {
            match solve(model, &tables, &s) {
                Ok(o) => {
                    s.row = first_row(&o);
                    s.result = Some(std::sync::Arc::new(o));
                    s.error = None;
                }
                Err(e) => s.error = Some(e),
            }
        }
        self.condens = Some(s);
        true
    }

    pub(super) fn condens_window(&mut self, ctx: &egui::Context) {
        let Some(mut s) = self.condens.clone() else {
            return;
        };
        let t = crate::i18n::tr;
        let model = self.doc.model();
        let tables = thermcheck::tables_of(model);
        let need = [
            thermcheck::RS,
            thermcheck::FORMULAS,
            thermcheck::CONDENSATION_LIMITS,
            thermcheck::CONDENSATION_SK,
            glaser::SK_CONSTANTS,
            glaser::SK_ANNUAL,
        ];
        let missing: Vec<&str> = need
            .iter()
            .copied()
            .filter(|id| !tables.iter().any(|t| t.id == *id))
            .collect();
        let mut asms: Vec<(ElementId, String)> = model
            .iter()
            .filter(|e| matches!(&e.kind, ElementKind::Assembly(_)))
            .map(|e| (e.id, e.name.clone()))
            .collect();
        asms.sort_by(|a, b| a.1.cmp(&b.1));
        let kinds = thermcheck::condensation_kinds(&tables);
        let mut open = true;
        let mut calc = false;
        let area = ctx.content_rect();
        super::shell::window(t("Condensation in a build-up"))
            .id(egui::Id::new("condens.window"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(590.0, 380.0))
            .default_size(crate::app::shell::at_least(Vec2::new(1180.0, 760.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("condens.scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if !missing.is_empty() {
                            ui.colored_label(
                                ui.visuals().warn_fg_color,
                                format!(
                                    "{}: {}",
                                    t("Norm tables not read (Manage › Norm tables)"),
                                    missing.join(", ")
                                ),
                            );
                        }
                        ui.horizontal_wrapped(|ui| {
                            ui.label(t("Build-up"));
                            let shown = s
                                .assembly
                                .and_then(|v| asms.iter().find(|d| d.0 == v))
                                .map_or(t("choose"), |d| d.1.clone());
                            egui::ComboBox::from_id_salt("condens.asm")
                                .selected_text(shown)
                                .width(280.0)
                                .show_ui(ui, |ui| {
                                    for (id, name) in &asms {
                                        if ui.selectable_value(&mut s.assembly, Some(*id), name).clicked() {
                                            calc = true;
                                        }
                                    }
                                });
                            ui.separator();
                            ui.label("θi");
                            ui.add(egui::DragValue::new(&mut s.ti).speed(0.1).suffix(" °C"));
                            ui.label("φi");
                            if s.phi_i.is_nan() {
                                if ui.button(t("enter")).clicked() {
                                    s.phi_i = 50.0;
                                }
                            } else {
                                ui.add(
                                    egui::DragValue::new(&mut s.phi_i)
                                        .speed(0.5)
                                        .range(0.0..=100.0)
                                        .suffix(" %"),
                                );
                            }
                            ui.label("θe");
                            if s.te.is_nan() {
                                if ui.button(t("enter")).clicked() {
                                    s.te = -11.0;
                                }
                            } else if ui
                                .add(egui::DragValue::new(&mut s.te).speed(0.1).suffix(" °C"))
                                .changed()
                            {
                                s.site.clear();
                            }
                            if !s.site.is_empty() {
                                ui.weak(format!("({}, STN 73 0540-3)", s.site));
                            }
                        });
                        ui.horizontal_wrapped(|ui| {
                            ui.label(t("Requirement (STN 73 0540-2, čl. 6.1)"));
                            let shown = kinds
                                .iter()
                                .find(|k| k.0 == s.kind)
                                .map_or(t("choose"), |k| k.2.clone());
                            egui::ComboBox::from_id_salt("condens.kind")
                                .selected_text(shown)
                                .width(520.0)
                                .show_ui(ui, |ui| {
                                    for (k, _, text) in &kinds {
                                        ui.selectable_value(&mut s.kind, k.clone(), text);
                                    }
                                });
                            if ui.button(t("Calculate")).clicked() {
                                calc = true;
                            }
                        });
                        if let Some(e) = &s.error {
                            ui.colored_label(ui.visuals().error_fg_color, e);
                        }
                        let Some(o) = s.result.clone() else {
                            return;
                        };
                        let w = ui.available_width();
                        if w >= 760.0 {
                            ui.horizontal_top(|ui| {
                                ui.vertical(|ui| {
                                    ui.set_width(w * 0.45);
                                    draw_profile(ui, &o, s.row, Vec2::new(w * 0.45, 360.0));
                                    layers_table(ui, &o);
                                });
                                ui.vertical(|ui| {
                                    summary_table(ui, &o);
                                    ui.add_space(6.0);
                                    rows_table(ui, &o, &mut s.row);
                                });
                            });
                        } else {
                            draw_profile(ui, &o, s.row, Vec2::new(w, 320.0));
                            summary_table(ui, &o);
                            rows_table(ui, &o, &mut s.row);
                            layers_table(ui, &o);
                        }
                        ui.add_space(4.0);
                        ui.weak(t("Glaser method: steady diffusion only; moisture's effect on λ, capillary transport, air flow and sorption are left out (STN EN ISO 13788, 6.3). The zone of each row of table 4 is where the line of p touches psat; rows without condensation keep the zone of the coldest row with one."));
                    });
            });
        if calc {
            let model = self.doc.model();
            match solve(model, &tables, &s) {
                Ok(o) => {
                    s.row = first_row(&o);
                    s.result = Some(std::sync::Arc::new(o));
                    s.error = None;
                }
                Err(e) => {
                    s.result = None;
                    s.error = Some(e);
                }
            }
        }
        self.condens = open.then_some(s);
    }
}

/// The coldest row with condensation, else the first.
fn first_row(o: &Outcome) -> usize {
    o.annual.rows.iter().position(|r| r.condenses).unwrap_or(0)
}

fn summary_table(ui: &mut egui::Ui, o: &Outcome) {
    let t = crate::i18n::tr;
    let num = crate::i18n::num;
    let a = &o.annual;
    let yes_no = |ok: bool| if ok { t("meets") } else { t("does not meet") };
    let mut rows: Vec<(String, String, String)> = vec![
        (t("Build-up"), o.name.clone(), String::new()),
        (
            "Rsi / Rse".into(),
            format!("{} / {} m²·K/W", num(o.rsi, 2), num(o.rse, 2)),
            "STN 73 0540-3, tab. 15".into(),
        ),
        (
            "pdi".into(),
            format!("{} Pa", num(a.pi, 1)),
            "STN 73 0540-3, (8), (9)".into(),
        ),
        (
            "δa".into(),
            format!("{}·10^-10 s", num(a.delta_a * 1e10, 4)),
            "STN 73 0540-3, (14)".into(),
        ),
        (
            t("Column of table 4"),
            format!(
                "{} ({})",
                a.column,
                if a.column == 'A' {
                    "θe ≤ −15 °C"
                } else {
                    "θe > −15 °C"
                }
            ),
            "STN 73 0540-3, tab. 4, tz".into(),
        ),
        (
            "Mc".into(),
            format!("{} kg/(m²·a)", num(a.mc, 4)),
            "STN 73 0540-2, (B.2)".into(),
        ),
        (
            "Mev".into(),
            format!("{} kg/(m²·a)", num(a.mev, 4)),
            "STN 73 0540-2, (B.3)".into(),
        ),
    ];
    if let Some((lim, _)) = &o.limit {
        rows.push((
            format!("Mc ≤ {}", num(*lim, 1)),
            yes_no(a.mc <= *lim + 1e-12),
            "STN 73 0540-2, čl. 6.1".into(),
        ));
    }
    if a.mc > 0.0 {
        rows.push((
            "Mc < Mev".into(),
            yes_no(a.mc < a.mev),
            "STN 73 0540-2, čl. 6.2, (13)".into(),
        ));
    }
    let cols = [
        Col::text("Physical quantity"),
        Col::num("Value", ""),
        Col::text("Source"),
    ];
    sheet::show(
        ui,
        "condens.summary",
        &cols,
        rows.len(),
        320.0,
        |row, i| {
            let r = &rows[i];
            row.text(r.0.clone());
            row.strong(r.1.clone());
            row.weak(r.2.clone());
        },
        None,
    );
}

fn rows_table(ui: &mut egui::Ui, o: &Outcome, sel: &mut usize) {
    let num = crate::i18n::num;
    let rows = &o.annual.rows;
    let cols = [
        Col::num("θe", "°C"),
        Col::num("φe", "%"),
        Col::num("pde", "Pa"),
        Col::num("tz", "10³ s"),
        Col::text("Zone A–B"),
        Col::num("ΔMd·t", "g/m²"),
    ];
    let mut clicked = None;
    sheet::show(
        ui,
        "condens.rows",
        &cols,
        rows.len(),
        320.0,
        |row, i| {
            let r = &rows[i];
            row.cell(|ui| {
                if ui.selectable_label(i == *sel, num(r.theta_e, 0)).clicked() {
                    clicked = Some(i);
                }
            });
            row.text(num(r.phi_e, 0));
            row.text(num(r.pe, 0));
            row.text(num(r.t / 1000.0, 1));
            match r.zone {
                Some((a, b)) => row.text(format!(
                    "{} – {} mm{}",
                    num(b * 1000.0, 1),
                    num(a * 1000.0, 1),
                    if r.condenses { "" } else { " *" }
                )),
                None => row.weak("–"),
            }
            row.strong(num(r.m() * 1000.0, 2));
        },
        Some(&|row: &mut sheet::Row| {
            row.text(crate::i18n::tr("Total"));
            row.empty();
            row.empty();
            row.text(num(rows.iter().map(|r| r.t).sum::<f64>() / 1000.0, 1));
            row.empty();
            row.text(num(rows.iter().map(|r| r.m()).sum::<f64>() * 1000.0, 2));
        }),
    );
    if let Some(i) = clicked {
        *sel = i;
    }
}

fn layers_table(ui: &mut egui::Ui, o: &Outcome) {
    let num = crate::i18n::num;
    let cols = [
        Col::text("Layer (from the outside)"),
        Col::num("d", "mm"),
        Col::num("R", "m²·K/W"),
        Col::num("sd", "m"),
    ];
    sheet::show(
        ui,
        "condens.layers",
        &cols,
        o.layers.len(),
        260.0,
        |row, i| {
            let l = &o.layers[i];
            row.text(l.name.clone());
            row.text(num(l.d * 1000.0, 1));
            row.text(num(l.r, 3));
            row.text(num(l.sd, 3));
        },
        None,
    );
}

/// θ, psat and p across the build-up for row `sel`, drawn on the scale of
/// sd (STN EN ISO 13788, 6.4.5), with the condensation zone shaded.
fn draw_profile(ui: &mut egui::Ui, o: &Outcome, sel: usize, size: Vec2) {
    let num = crate::i18n::num;
    let Some(p) = o.profiles.get(sel) else { return };
    let r = &o.annual.rows[sel];
    let (resp, painter) = ui.allocate_painter(size, egui::Sense::hover());
    let rect = resp.rect.shrink2(Vec2::new(46.0, 26.0));
    let vis = ui.visuals();
    painter.rect_filled(resp.rect, 4.0, vis.extreme_bg_color);
    let ymax = p
        .interfaces
        .iter()
        .map(|f| f.psat.max(f.p))
        .fold(p.pi.max(p.pe), f64::max)
        * 1.08;
    let x = |sd: f64| rect.left() + (sd / p.sd_total) as f32 * rect.width();
    let y = |v: f64| rect.bottom() - (v / ymax) as f32 * rect.height();
    // the layers' borders, on the sd scale
    let mut sd = 0.0;
    let text = vis.text_color();
    let faint = vis.weak_text_color();
    painter.line_segment(
        [
            Pos2::new(x(0.0), rect.top()),
            Pos2::new(x(0.0), rect.bottom()),
        ],
        Stroke::new(1.0, faint),
    );
    for l in &o.layers {
        sd += l.sd;
        painter.line_segment(
            [
                Pos2::new(x(sd), rect.top()),
                Pos2::new(x(sd), rect.bottom()),
            ],
            Stroke::new(1.0, faint),
        );
    }
    if let Some((a, b)) = r.zone {
        let at = |depth: f64| {
            p.interfaces
                .iter()
                .min_by(|f, g| (f.x - depth).abs().total_cmp(&(g.x - depth).abs()))
                .map_or(0.0, |f| f.sd)
        };
        let (xa, xb) = (x(at(a)), x(at(b)));
        let fill = if r.condenses {
            Color32::from_rgba_unmultiplied(60, 140, 255, 60)
        } else {
            Color32::from_rgba_unmultiplied(160, 160, 160, 40)
        };
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(xb.min(xa) - 1.0, rect.top()),
                Pos2::new(xa.max(xb) + 1.0, rect.bottom()),
            ),
            0.0,
            fill,
        );
    }
    let line = |v: &dyn Fn(&glaser::Interface) -> f64| -> Vec<Pos2> {
        p.interfaces
            .iter()
            .map(|f| Pos2::new(x(f.sd), y(v(f))))
            .collect()
    };
    painter.add(egui::Shape::line(
        line(&|f| f.psat),
        Stroke::new(2.0, Color32::from_rgb(220, 70, 50)),
    ));
    painter.add(egui::Shape::line(
        line(&|f| f.p),
        Stroke::new(2.0, Color32::from_rgb(40, 120, 230)),
    ));
    let font = egui::FontId::proportional(11.0);
    painter.text(
        Pos2::new(rect.left() + 4.0, resp.rect.top() + 4.0),
        egui::Align2::LEFT_TOP,
        format!("θe {} °C, pde {} Pa", num(r.theta_e, 0), num(r.pe, 0)),
        font.clone(),
        text,
    );
    painter.text(
        Pos2::new(rect.right(), resp.rect.top() + 4.0),
        egui::Align2::RIGHT_TOP,
        format!("pdi {} Pa", num(p.pi, 0)),
        font.clone(),
        text,
    );
    painter.text(
        Pos2::new(rect.left(), rect.bottom() + 4.0),
        egui::Align2::LEFT_TOP,
        crate::i18n::tr("outside"),
        font.clone(),
        faint,
    );
    painter.text(
        Pos2::new(rect.right(), rect.bottom() + 4.0),
        egui::Align2::RIGHT_TOP,
        format!(
            "{}  ·  s'd,T {} m",
            crate::i18n::tr("inside"),
            num(p.sd_total, 2)
        ),
        font.clone(),
        faint,
    );
    painter.text(
        Pos2::new(rect.left() - 4.0, rect.bottom()),
        egui::Align2::RIGHT_BOTTOM,
        "0",
        font.clone(),
        faint,
    );
    painter.text(
        Pos2::new(rect.left() - 4.0, y(ymax / 1.08)),
        egui::Align2::RIGHT_CENTER,
        num(ymax / 1.08, 0),
        font.clone(),
        faint,
    );
    let mid = rect.center().x;
    painter.text(
        Pos2::new(mid - 4.0, rect.bottom() + 4.0),
        egui::Align2::RIGHT_TOP,
        "psat",
        font.clone(),
        Color32::from_rgb(220, 70, 50),
    );
    painter.text(
        Pos2::new(mid + 4.0, rect.bottom() + 4.0),
        egui::Align2::LEFT_TOP,
        "p",
        font,
        Color32::from_rgb(40, 120, 230),
    );
}
