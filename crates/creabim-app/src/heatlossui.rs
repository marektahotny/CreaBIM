//! The rooms' design heat loss ([`creabim_services::heatloss`],
//! simplified after STN EN 12831-1): a window with what the calculation
//! is given, each room's losses and what it lacks, the building's total,
//! and a step on to sizing the heat source by it.

use super::CreaBimApp;
use creabim_core::catalogue::heat::HeatKind;
use creabim_core::fireparams::{self, ClimateSite};
use creabim_services::heatdetail;
use creabim_services::heatloss::{self, Method, Opts, Shielding};
use creabim_services::psi;

/// `o` with the temperatures of locality `s`.
fn with_site(o: Opts, s: &ClimateSite) -> Opts {
    Opts {
        te: s.te,
        te_mean: s.te_year.or(o.te_mean),
        te_heating: s.te_heating,
        heating_days: s.days,
        ..o
    }
}
use eframe::egui::{self, Vec2};

/// The climate table of the designer's PC, kept when a table is read.
fn saved_sites_path() -> Option<std::path::PathBuf> {
    crate::files::settings_dir().map(|d| d.join("lokality.tsv"))
}

impl CreaBimApp {
    /// The climate tables: the project's or the application's, else the
    /// ones read last.
    pub(super) fn climate_tables(&self) -> Vec<fireparams::ParamTable> {
        let own: Vec<_> = self
            .doc
            .model()
            .project_info()
            .map(|(_, p)| {
                fireparams::effective(&p.fire_params)
                    .into_iter()
                    .filter(|t| fireparams::CLIMATE_TABLES.contains(&t.id.as_str()))
                    .collect()
            })
            .unwrap_or_default();
        if !own.is_empty() {
            return own;
        }
        saved_sites_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|t| fireparams::parse(&t))
            .unwrap_or_default()
    }

    /// The localities: the project's table, else the one read last.
    fn climate_sites(&self) -> Vec<ClimateSite> {
        let own = self
            .doc
            .model()
            .project_info()
            .map(|(_, p)| fireparams::climate_sites(&fireparams::effective(&p.fire_params)))
            .unwrap_or_default();
        if !own.is_empty() {
            return own;
        }
        saved_sites_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .map(|t| fireparams::climate_sites(&fireparams::parse(&t)))
            .unwrap_or_default()
    }

    /// `o` with the project's locality's temperatures.
    pub(super) fn site_opts(&self, o: Opts) -> Opts {
        let name = self
            .doc
            .model()
            .project_info()
            .map(|(_, p)| p.climate_site.clone())
            .unwrap_or_default();
        match self.climate_sites().into_iter().find(|s| s.name == name) {
            Some(s) => with_site(o, &s),
            None => o,
        }
    }

    /// Keep the locality chosen, and its table, with the project.
    fn set_climate_site(&mut self, s: &ClimateSite) {
        let mut info = self
            .doc
            .model()
            .project_info()
            .map(|(_, p)| p.clone())
            .unwrap_or_default();
        info.climate_site = s.name.clone();
        if fireparams::climate_sites(&fireparams::effective(&info.fire_params)).is_empty() {
            if let Some(t) = saved_sites_path().and_then(|p| std::fs::read_to_string(p).ok()) {
                let tables = fireparams::parse(&t)
                    .into_iter()
                    .filter(|t| fireparams::CLIMATE_TABLES.contains(&t.id.as_str()))
                    .collect();
                fireparams::merge(&mut info.fire_params, tables);
            }
        }
        if let Some(p) = self.project_draft.as_mut() {
            p.climate_site = info.climate_site.clone();
            p.fire_params = info.fire_params.clone();
        }
        let tx = creabim_core::set_project(self.doc.model(), info);
        let r = self.doc.commit(tx);
        self.report(r, format!("{}: {}", crate::i18n::tr("Locality"), s.name));
    }

    /// Read a climate table from a file into the project and keep a copy
    /// for new projects.
    pub(super) fn load_climate_sites(&mut self) {
        let t = crate::i18n::tr;
        let Some(path) = crate::files::file_dialog()
            .set_title(t("Table of localities"))
            .add_filter("TSV", &["tsv", "txt"])
            .pick_file()
        else {
            return;
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(x) => x,
            Err(e) => {
                self.status = t(&format!("The file could not be read: {e}"));
                return;
            }
        };
        let all = fireparams::parse(&text);
        let heat: Vec<_> = all
            .iter()
            .filter(|t| {
                heatdetail::TABLES.contains(&t.id.as_str())
                    || creabim_services::coolnorm::TABLES.contains(&t.id.as_str())
                    || t.id == psi::PSI
            })
            .cloned()
            .collect();
        if !heat.is_empty() {
            self.load_heat_tables(heat);
            return;
        }
        let tables: Vec<_> = all
            .into_iter()
            .filter(|t| fireparams::CLIMATE_TABLES.contains(&t.id.as_str()))
            .collect();
        let n = fireparams::climate_sites(&tables).len();
        if n == 0 {
            self.status =
                t("No localities found in the file: it needs a table \"#table climate_sites\".");
            return;
        }
        if let Some(p) = saved_sites_path() {
            let _ = std::fs::create_dir_all(p.parent().unwrap_or(&p));
            let _ = std::fs::write(&p, fireparams::write(&tables));
        }
        let mut info = self
            .doc
            .model()
            .project_info()
            .map(|(_, p)| p.clone())
            .unwrap_or_default();
        fireparams::merge(&mut info.fire_params, tables);
        if let Some(p) = self.project_draft.as_mut() {
            p.fire_params = info.fire_params.clone();
        }
        let tx = creabim_core::set_project(self.doc.model(), info);
        let r = self.doc.commit(tx);
        self.report(r, format!("{n} {}", t("localities read")));
    }

    /// Takes the tables of STN EN 12831-1 into the project and keeps them
    /// with the application's norm tables for new projects.
    fn load_heat_tables(&mut self, tables: Vec<fireparams::ParamTable>) {
        let n = tables.len();
        let mut app = fireparams::app_tables().to_vec();
        fireparams::merge(&mut app, tables.clone());
        if let Some(p) = crate::files::app_tables_path() {
            let _ = std::fs::create_dir_all(p.parent().unwrap_or(&p));
            let _ = std::fs::write(&p, fireparams::write(&app));
        }
        fireparams::set_app_tables(app);
        let mut info = self
            .doc
            .model()
            .project_info()
            .map(|(_, p)| p.clone())
            .unwrap_or_default();
        fireparams::merge(&mut info.fire_params, tables);
        if let Some(p) = self.project_draft.as_mut() {
            p.fire_params = info.fire_params.clone();
        }
        let tx = creabim_core::set_project(self.doc.model(), info);
        let r = self.doc.commit(tx);
        self.report(
            r,
            format!("{n} {}", crate::i18n::t("tables of the norm read")),
        );
    }

    /// `calc:heatloss` from the Heating tab: calculate and show the window.
    pub(super) fn heat_loss_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" || !matches!(arg, "heatloss" | "heatloss-detailed" | "heatloss-psi") {
            return false;
        }
        let mut o = self
            .heat_loss
            .as_ref()
            .map_or_else(|| self.site_opts(Opts::default()), |x| x.0);
        if arg != "heatloss" {
            o.method = Method::Detailed;
            o.psi = arg == "heatloss-psi";
        }
        let b = heatloss::building(self.doc.model(), &o);
        self.status = if b.rooms.is_empty() {
            crate::i18n::tr("No rooms to calculate: place rooms first.")
        } else {
            crate::i18n::tr("Heat loss calculated")
        };
        self.write_results(heatloss_results(&b, &o));
        self.heat_loss = Some((o, b));
        true
    }

    /// The window, while it is open.
    pub(super) fn heat_loss_window(&mut self, ctx: &egui::Context) {
        let Some((mut o, b)) = self.heat_loss.clone() else {
            return;
        };
        let t = crate::i18n::tr;
        let num = crate::i18n::num;
        let mut open = true;
        let mut select = None;
        let mut to_source = false;
        let mut write_now = false;
        let sites = self.climate_sites();
        let site_name = self
            .doc
            .model()
            .project_info()
            .map(|(_, p)| p.climate_site.clone())
            .unwrap_or_default();
        let mut pick: Option<ClimateSite> = None;
        let n50 = fireparams::n50_ranges(&self.climate_tables());
        let mut load_sites = false;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Heat loss of the rooms"))
            .id(egui::Id::new("heat.loss"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(330.0, 280.0))
            .default_size(crate::app::shell::at_least(Vec2::new(660.0, 560.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                if super::calcresultsui::write_button(ui) {
                    write_now = true;
                }
                ui.horizontal(|ui| {
                    ui.label(t("Locality"));
                    let shown = if site_name.is_empty() {
                        t("choose")
                    } else {
                        site_name.clone()
                    };
                    egui::ComboBox::from_id_salt("heat.loss.site")
                        .selected_text(shown)
                        .width(220.0)
                        .height(420.0)
                        .show_ui(ui, |ui| {
                            let mut region = "";
                            for s in &sites {
                                if s.region != region {
                                    region = &s.region;
                                    ui.colored_label(egui::Color32::from_rgb(40, 130, 40), region);
                                }
                                let text = format!(
                                    "{}   {} °C · {} m",
                                    s.name,
                                    num(s.te, 1),
                                    s.altitude.map_or(String::from("–"), |a| num(a, 0))
                                );
                                if ui.selectable_label(s.name == site_name, text).clicked() {
                                    pick = Some(s.clone());
                                }
                            }
                        });
                    if ui
                        .button(t("Read the table…"))
                        .on_hover_text(t("Read the table of localities (TSV) into the project; it is kept for new projects too"))
                        .clicked()
                    {
                        load_sites = true;
                    }
                    if sites.is_empty() {
                        ui.weak(t("no table of localities read yet"));
                    }
                });
                if let Some(s) = sites.iter().find(|s| s.name == site_name) {
                    ui.weak(format!(
                        "θe {} °C · {} {} °C · {} {} °C · {} {} · {} {} m",
                        num(s.te, 1),
                        t("mean annual"),
                        s.te_year.map_or(String::from("–"), |v| num(v, 1)),
                        t("heating season"),
                        s.te_heating.map_or(String::from("–"), |v| num(v, 1)),
                        s.days.map_or(String::from("–"), |v| num(v, 0)),
                        t("days"),
                        t("altitude"),
                        s.altitude.map_or(String::from("–"), |v| num(v, 0)),
                    ));
                }
                let heat_tables = heatdetail::tables_of(self.doc.model());
                ui.horizontal(|ui| {
                    ui.label(t("Method"));
                    egui::ComboBox::from_id_salt("heat.loss.method")
                        .selected_text(t(o.method.name()))
                        .show_ui(ui, |ui| {
                            for m in Method::ALL {
                                ui.selectable_value(&mut o.method, m, t(m.name()));
                            }
                        });
                    if o.method == Method::Detailed {
                        ui.weak(t("STN EN 12831-1 chapter 6: outside dimensions, rooms over and under, ground by annex E"));
                    }
                });
                if o.method == Method::Detailed {
                    let rows = heatdetail::tb_rows(&heat_tables);
                    if heat_tables.len() < heatdetail::TABLES.len() {
                        ui.horizontal(|ui| {
                            ui.colored_label(
                                ui.visuals().warn_fg_color,
                                t("The tables of STN EN 12831-1 are not read: read its parameter file"),
                            );
                            if ui.button(t("Read the table…")).clicked() {
                                load_sites = true;
                            }
                        });
                    }
                    ui.horizontal(|ui| {
                        ui.label(t("Thermal bridges"));
                        egui::ComboBox::from_id_salt("heat.loss.tb.mode")
                            .selected_text(if o.psi {
                                t("Σ l·Ψ (annex C, STN EN ISO 14683)")
                            } else {
                                t("ΔU_TB (tab. B.1)")
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut o.psi, false, t("ΔU_TB (tab. B.1)"));
                                ui.selectable_value(&mut o.psi, true, t("Σ l·Ψ (annex C, STN EN ISO 14683)"));
                            });
                    });
                    if o.psi {
                        let rows = psi::rows(&heat_tables);
                        if rows.is_empty() {
                            ui.horizontal(|ui| {
                                ui.colored_label(
                                    ui.visuals().warn_fg_color,
                                    t("The table of STN EN ISO 14683 is not read: read its parameter file"),
                                );
                                if ui.button(t("Read the table…")).clicked() {
                                    load_sites = true;
                                }
                            });
                        }
                        egui::CollapsingHeader::new(t("Details of the junctions (STN EN ISO 14683, table C.2)"))
                            .id_salt("heat.loss.psi")
                            .show(ui, |ui| {
                                egui::Grid::new("heat.loss.psi.grid")
                                    .num_columns(2)
                                    .spacing([12.0, 4.0])
                                    .show(ui, |ui| {
                                        for j in psi::Joint::ALL {
                                            ui.label(t(j.name()));
                                            let slot = psi::slot(j);
                                            let show = |r: &psi::Row| {
                                                format!("{} – Ψe {} – {}", r.key, num(r.psi_e, 2), r.text)
                                            };
                                            let now = o.psi_pick[slot]
                                                .and_then(|i| rows.get(i as usize))
                                                .map_or(t("By the build-up"), show);
                                            egui::ComboBox::from_id_salt(("heat.loss.psi", slot))
                                                .selected_text(now)
                                                .width(ui.available_width().min(420.0))
                                                .show_ui(ui, |ui| {
                                                    ui.selectable_value(&mut o.psi_pick[slot], None, t("By the build-up"));
                                                    for (i, r) in rows.iter().enumerate() {
                                                        if r.group == j.group() {
                                                            ui.selectable_value(&mut o.psi_pick[slot], Some(i as u16), show(r));
                                                        }
                                                    }
                                                });
                                            ui.end_row();
                                        }
                                    });
                                ui.weak(t("By the build-up: the detail for where the outside wall's main insulation lies (outside, in the core, inside, none) and, for windows, where the frame stands and whether the insulation laps it."));
                            });
                    } else {
                        ui.horizontal(|ui| {
                            ui.label(t("Thermal bridges ΔU_TB (tab. B.1)"));
                            let shown = rows
                                .get(o.tb_row)
                                .map_or(t("choose"), |r| format!("{} W/(m²K) – {}", num(r.1, 2), r.0));
                            egui::ComboBox::from_id_salt("heat.loss.tb")
                                .selected_text(shown)
                                .width(ui.available_width().min(460.0))
                                .show_ui(ui, |ui| {
                                    for (i, r) in rows.iter().enumerate() {
                                        ui.selectable_value(
                                            &mut o.tb_row,
                                            i,
                                            format!("{} W/(m²K) – {}", num(r.1, 2), r.0),
                                        );
                                    }
                                });
                        });
                    }
                    ui.checkbox(&mut o.gw_near, t("Ground water within 1 m under the floor slab (f_GW)"));
                }
                egui::Grid::new("heat.loss.opts")
                    .num_columns(4)
                    .spacing([12.0, 5.0])
                    .show(ui, |ui| {
                        let val = |ui: &mut egui::Ui, text: &str, v: &mut f64, lo: f64, hi: f64, step: f64, unit: &str| {
                            ui.label(t(text));
                            ui.add(
                                egui::DragValue::new(v)
                                    .range(lo..=hi)
                                    .speed(step)
                                    .suffix(unit),
                            );
                        };
                        val(ui, "Design outside temperature", &mut o.te, -30.0, 5.0, 0.5, " °C");
                        if o.method == Method::Simplified {
                            val(ui, "Thermal bridges ΔU", &mut o.d_u, 0.0, 0.3, 0.01, " W/(m²K)");
                        }
                        ui.end_row();
                        let mut mean = o.te_mean.unwrap_or(0.0);
                        let mut has = o.te_mean.is_some();
                        ui.label(t("Mean annual temperature"));
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut has, "");
                            ui.add_enabled(
                                has,
                                egui::DragValue::new(&mut mean)
                                    .range(-5.0..=20.0)
                                    .speed(0.1)
                                    .suffix(" °C"),
                            );
                        });
                        o.te_mean = has.then_some(if o.te_mean.is_none() && has { 9.5 } else { mean });
                        ui.label(t("Air changes n50"));
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut o.n50)
                                    .range(0.3..=20.0)
                                    .speed(0.1)
                                    .suffix(" 1/h"),
                            );
                            if !n50.is_empty() {
                                egui::ComboBox::from_id_salt("heat.loss.n50")
                                    .selected_text(t("Choose"))
                                    .width(90.0)
                                    .show_ui(ui, |ui| {
                                        let mut building = "";
                                        for r in &n50 {
                                            if r.building != building {
                                                building = &r.building;
                                                ui.colored_label(egui::Color32::from_rgb(40, 130, 40), building);
                                            }
                                            let text = format!("{}   ({})", r.tightness, r.text());
                                            if ui.selectable_label(false, text).clicked() {
                                                if let Some(v) = r.usual() {
                                                    o.n50 = v.max(0.3);
                                                }
                                            }
                                        }
                                    })
                                    .response
                                    .on_hover_text(t("Air changes at 50 Pa for the whole building by the building and the tightness of its envelope; the value can be changed within the range"));
                            }
                        });
                        ui.end_row();
                        val(ui, "U of windows", &mut o.u_window, 0.3, 6.0, 0.05, " W/(m²K)");
                        val(ui, "U of outside doors", &mut o.u_door, 0.3, 6.0, 0.05, " W/(m²K)");
                        ui.end_row();
                    });
                egui::ComboBox::from_id_salt("heat.loss.shield")
                    .selected_text(t(o.shielding.name()))
                    .width(ui.available_width().min(420.0))
                    .show_ui(ui, |ui| {
                        for s in Shielding::ALL {
                            ui.selectable_value(&mut o.shielding, s, t(s.name()));
                        }
                    });
                ui.separator();
                if b.rooms.is_empty() {
                    ui.label(t("No rooms to calculate: place rooms first."));
                    return;
                }
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{}: {} kW · {} m² · {} W/m²",
                            t("The building"),
                            num(b.phi / 1000.0, 2),
                            num(b.area, 1),
                            num(if b.area > 0.0 { b.phi / b.area } else { 0.0 }, 0),
                        ))
                        .strong(),
                    );
                    if ui
                        .button(t("Size the heat source"))
                        .on_hover_text(t("Open the heat source window with this heat loss and outside temperature"))
                        .clicked()
                    {
                        to_source = true;
                    }
                });
                if let (Some(d), Some(e)) = (o.degree_days(), o.yearly_kwh(b.phi)) {
                    ui.weak(format!(
                        "{}: {} K·d · {}: {} kWh/{}",
                        t("Degree days"),
                        num(d, 0),
                        t("rough yearly heat need"),
                        num(e, 0),
                        t("year"),
                    ));
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    use crate::sheet::{self, Col};
                    let cols = [
                        Col::text("Room"),
                        Col::num("θi", "°C"),
                        Col::num("A", "m²"),
                        Col::num("V", "m³"),
                        Col::num("ΦT", "W"),
                        Col::num("ΦV", "W"),
                        Col::num("Φ", "W"),
                        Col::num("Φ/A", "W/m²"),
                    ];
                    let total = |row: &mut sheet::Row| {
                        row.text(t("Total"));
                        row.empty();
                        row.text(num(b.area, 1));
                        row.text(num(b.rooms.iter().map(|r| r.volume).sum::<f64>(), 1));
                        row.text(num(b.rooms.iter().map(|r| r.phi_t).sum::<f64>(), 0));
                        row.text(num(b.rooms.iter().map(|r| r.phi_v).sum::<f64>(), 0));
                        row.text(num(b.phi, 0));
                        row.text(num(if b.area > 0.0 { b.phi / b.area } else { 0.0 }, 0));
                    };
                    sheet::show(ui, "heat.loss.rooms", &cols, b.rooms.len(), 360.0, |row, i| {
                        let r = &b.rooms[i];
                        let name = format!("{} {}", r.number, r.name);
                        row.cell(|ui| {
                            if ui.link(name.trim()).clicked() {
                                select = Some(r.id);
                            }
                        });
                        row.text(num(r.ti, 0));
                        row.text(num(r.area, 1));
                        row.text(num(r.volume, 1));
                        row.text(num(r.phi_t, 0));
                        row.text(num(r.phi_v, 0));
                        row.strong(num(r.phi, 0));
                        row.text(num(if r.area > 0.0 { r.phi / r.area } else { 0.0 }, 0));
                    }, Some(&total));
                    ui.add_space(6.0);
                    for r in &b.rooms {
                        let title = format!("{} {} – {} W", r.number, r.name, num(r.phi, 0));
                        egui::CollapsingHeader::new(title.trim().to_string())
                            .id_salt(("heat.loss.room", r.id))
                            .show(ui, |ui| {
                                let cols = [
                                    Col::text("Through"),
                                    Col::num("A / l", "m² / m"),
                                    Col::num("U / Ψ", "W/(m²K) / W/(mK)"),
                                    Col::num("f", "–"),
                                    Col::num("Φ", "W"),
                                ];
                                let total = |row: &mut sheet::Row| {
                                    row.text(t("Total"));
                                    row.empty();
                                    row.empty();
                                    row.empty();
                                    row.text(num(r.phi_t, 0));
                                };
                                sheet::show(ui, ("heat.loss.items", r.id), &cols, r.items.len(), 400.0, |row, k| {
                                    let i = &r.items[k];
                                    let what = if i.detail.is_empty() {
                                        t(i.kind)
                                    } else {
                                        format!("{} – {}", t(i.kind), i.detail)
                                    };
                                    row.text(what);
                                    if i.kind == "Thermal bridge" {
                                        row.text(format!("l {}", num(i.area, 2)));
                                        row.text(format!("Ψ {}", num(i.u, 2)));
                                    } else {
                                        row.text(num(i.area, 2));
                                        row.text(num(i.u, 3));
                                    }
                                    row.text(num(i.f, 3));
                                    row.text(num(i.phi, 0));
                                }, Some(&total));
                                ui.label(format!(
                                    "{}: {} m³/h ({} {}, {} {}; {} {})",
                                    t("Ventilation"),
                                    num(r.v_min.max(r.v_inf), 1),
                                    t("least"),
                                    num(r.v_min, 1),
                                    t("infiltration"),
                                    num(r.v_inf, 1),
                                    t("façades with openings"),
                                    r.facades,
                                ));
                                for m in &r.missing {
                                    ui.colored_label(ui.visuals().warn_fg_color, t(m.text()));
                                }
                            });
                    }
                    ui.add_space(6.0);
                    if o.method == Method::Detailed {
                        ui.weak(t("After STN EN 12831-1 chapter 6: areas by the outside dimensions (walls a storey high, to the outer face of outside walls and the axis of inner ones), U from the build-ups' λ plus ΔU_TB of table B.1, heat to colder rooms beside, over and under (f = (θi − θx)/(θi − θe)), the ground by annex E with f_θann, f_GW and θe,m, the air by 6.3.3 for the building as one zone with no mechanical ventilation."));
                        ui.weak("STN EN 12831-1:2019 · STN EN ISO 6946");
                    } else {
                        ui.weak(t("Simplified after STN EN 12831-1: inside dimensions, outside walls where no room lies beyond, U from the build-ups' λ, floors and ceilings between storeys left out. A wall is counted for the room it bounds."));
                        ui.weak("STN EN 12831-1 · STN 73 0540-2 · STN EN ISO 6946");
                    }
                });
            });
        if let Some(id) = select {
            self.selection.only(id);
        }
        if let Some(s) = pick {
            o = with_site(o, &s);
            self.set_climate_site(&s);
        }
        if load_sites {
            self.load_climate_sites();
        }
        if to_source {
            let mut s = self.heat_source.clone().unwrap_or_default();
            s.input.heat_load = (b.phi / 1000.0).clamp(0.5, 200.0);
            s.input.te = o.te.clamp(-25.0, 0.0);
            if matches!(
                s.kind,
                HeatKind::HpOutdoor | HeatKind::HpIndoor | HeatKind::HpIndoorCylinder
            ) {
                s.size = creabim_services::hpsizing::recommend(&s.input);
            }
            self.heat_source = Some(s);
        }
        if write_now {
            self.write_results_said(heatloss_results(&b, &o));
        }
        if !open {
            self.heat_loss = None;
        } else if self.heat_loss.as_ref().is_some_and(|x| x.0 != o) {
            let b = heatloss::building(self.doc.model(), &o);
            self.heat_loss = Some((o, b));
        }
    }
}

fn heatloss_results(b: &heatloss::BuildingLoss, o: &Opts) -> creabim_core::results::ResultSet {
    creabim_services::calcresults::heat_loss(b, o)
}
