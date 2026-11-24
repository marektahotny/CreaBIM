//! Check railings and window sills against STN 74 3305
//! ([`creabim_core::railcheck`]): the table of what was found, its rows
//! showing the element, and the norm's tables read from the user's file.

use super::CreaBimApp;
use crate::sheet::{self, Col};
use creabim_core::fireparams;
use creabim_core::railcheck::{self, Access, Fault, Rules};
use eframe::egui::{self, Vec2};

/// The window's state: who comes onto the floors.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RailCheck {
    pub access: Access,
}

impl CreaBimApp {
    /// `railing:check` from the Architecture tab: open the window.
    pub(super) fn rail_check_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "railing" || arg != "check" {
            return false;
        }
        if self.rail_check.is_none() {
            self.rail_check = Some(RailCheck::default());
        }
        true
    }

    /// Read the norm's tables from the user's file into the project.
    fn load_rail_tables(&mut self) {
        let t = crate::i18n::tr;
        let Some(path) = crate::files::file_dialog()
            .set_title(t("Tables of STN 74 3305"))
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
        let tables: Vec<_> = fireparams::parse(&text)
            .into_iter()
            .filter(|x| railcheck::TABLES.contains(&x.id.as_str()))
            .collect();
        if tables.is_empty() {
            self.status = t("No table of STN 74 3305 in the file.");
            return;
        }
        self.load_heat_tables(tables);
    }

    /// The window, while it is open.
    pub(super) fn rail_check_window(&mut self, ctx: &egui::Context) {
        let Some(mut st) = self.rail_check else {
            return;
        };
        let t = crate::i18n::tr;
        let mut open = true;
        let mut show: Option<creabim_core::ElementId> = None;
        let mut load = false;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        let model = self.doc.model();
        let tables = creabim_core::room::project_tables(model);
        let rules = Rules::read(&tables, st.access);
        let found = rules
            .as_ref()
            .map(|r| railcheck::check(model, r))
            .unwrap_or_default();
        let mm = |v: f64| crate::i18n::num(v * 1000.0, 0);
        let fault_text = |f: &Fault| match *f {
            Fault::Low { have, need } => format!(
                "{} {} < {} mm ({})",
                t("height"),
                mm(have),
                mm(need),
                t("tab. 2")
            ),
            Fault::Sill { have, need } => format!(
                "{} {} < {} mm: {}",
                t("sill"),
                mm(have),
                mm(need),
                t("a railing is needed (3.1.1)")
            ),
            Fault::Upright { have, max } => format!(
                "{} {} > {} mm",
                t("gaps between upright bars"),
                mm(have),
                mm(max)
            ),
            Fault::Level { have, max } => {
                format!("{} {} > {} mm", t("level gaps"), mm(have), mm(max))
            }
            Fault::Bottom { have, max } => {
                format!("{} {} > {} mm", t("gap over the floor"), mm(have), mm(max))
            }
            Fault::Offset { have, max } => format!(
                "{} {} > {} mm",
                t("gap to the floor's edge"),
                mm(have),
                mm(max)
            ),
            Fault::Clear { have, min } => format!(
                "{} {} < {} mm (4.5.4)",
                t("handrail clear of the wall"),
                mm(have),
                mm(min)
            ),
            Fault::Grip { have, min, max } => format!(
                "{} Ø{} {} {}–{} mm (4.5.5)",
                t("handrail"),
                mm(have),
                t("not"),
                mm(min),
                mm(max)
            ),
            Fault::LowZone { zone } => format!(
                "{} {} mm {}",
                t("up to"),
                mm(zone),
                t("only a solid or upright infill")
            ),
        };
        let name = |id| {
            model.get(id).map_or_else(String::new, |e| {
                let mark = creabim_core::marks::manual(e);
                match mark {
                    Some(m) if !m.is_empty() => format!("{m} {}", e.name),
                    _ => e.name.clone(),
                }
            })
        };
        let rows: Vec<_> = found.iter().collect();
        super::shell::window(t("Check railings (STN 74 3305)"))
            .id(egui::Id::new("railing.check"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(380.0, 220.0))
            .default_size(crate::app::shell::at_least(Vec2::new(760.0, 420.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(t("Who comes onto the floors"));
                    egui::ComboBox::from_id_salt("railing.access")
                        .selected_text(t(st.access.label()))
                        .show_ui(ui, |ui| {
                            for a in Access::ALL {
                                ui.selectable_value(&mut st.access, a, t(a.label()));
                            }
                        });
                    if ui
                        .button(t("Read tables…"))
                        .on_hover_text(t("The file of the norm's tables (railing_depth, railing_height, railing_gaps, railing_handrail): it is kept with the project"))
                        .clicked()
                    {
                        load = true;
                    }
                });
                ui.separator();
                if rules.is_none() {
                    ui.label(t("The tables of STN 74 3305 are not read yet: read them from your file (Read tables…)."));
                    return;
                }
                if rows.is_empty() {
                    ui.weak(t("No railing, and no window over a fall that needs one."));
                    return;
                }
                let cols = [
                    Col::text("Element"),
                    Col::num("Fall in front", "mm"),
                    Col::num("Least height", "mm"),
                    Col::num("Height / sill", "mm"),
                    Col::text("Result"),
                ];
                sheet::show(ui, "railing.check.rows", &cols, rows.len(), 420.0, |row, i| {
                    let f = rows[i];
                    row.cell(|ui| {
                        if ui.link(name(f.id)).clicked() {
                            show = Some(f.id);
                        }
                    });
                    if f.wall {
                        row.weak("–");
                    } else {
                        row.text(mm(f.depth));
                    }
                    if f.needed && !f.wall {
                        row.text(mm(f.need));
                    } else {
                        row.weak("–");
                    }
                    row.text(mm(f.have));
                    if !f.needed {
                        row.weak(t("not needed (tab. 1)"));
                    } else if f.faults.is_empty() {
                        row.text(t("complies"));
                    } else {
                        let s = f.faults.iter().map(&fault_text).collect::<Vec<_>>().join("; ");
                        row.cell(|ui| {
                            ui.colored_label(ui.visuals().error_fg_color, s);
                        });
                    }
                }, None);
                ui.add_space(4.0);
                ui.weak(t("The fall is measured to what lies under the railing's sides (a window's: 0,5 m in front of it): a slab, the terrain, else the ground storey's floor; a stair's railing over the stair's whole rise. The free space's width (tab. 1) and a floor sloping to the edge (tab. 2) are not judged."));
            });
        if let Some(id) = show {
            self.selection.only(id);
            self.frame_selection();
        }
        if load {
            self.load_rail_tables();
        }
        self.rail_check = if open { Some(st) } else { None };
    }
}
