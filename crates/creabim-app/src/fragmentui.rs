//! Table 30 of ISOVER Fragment's Projektové hodnotenie ("Potreba
//! primárnej energie budovy"): Fragment works out the energy need of the
//! heating and the hot water itself (from the municipality, by its
//! monthly method), so this window takes those two numbers as Fragment
//! shows them and says what goes into which column for the heat source
//! of the model, by Fragment's own factors
//! ([`creabim_geom::fragment::Generator`]).

use super::CreaBimApp;
use creabim_geom::fragment::{generators, table30_row, Generator};
use eframe::egui::{self, Vec2};

/// What the window is given.
#[derive(Clone, Debug, PartialEq)]
pub struct Table30 {
    /// The needs Fragment shows in its column "Potreba energie",
    /// kWh/(m²·a).
    pub heating: f64,
    pub water: f64,
    pub heat_by: Generator,
    pub heat_factor: f64,
    pub water_by: Generator,
    pub water_factor: f64,
}

impl Table30 {
    /// With the heat source the model holds.
    pub fn of(model: &creabim_core::Model) -> Self {
        let (h, w) = generators(model);
        let heat_by = h.unwrap_or(Generator::HpAirWaterLowTemp);
        let water_by = w.unwrap_or(Generator::Electric);
        Self {
            heating: 0.0,
            water: 0.0,
            heat_by,
            heat_factor: heat_by.factor(),
            water_by,
            water_factor: water_by.factor(),
        }
    }
}

fn generator_combo(ui: &mut egui::Ui, id: &str, g: &mut Generator, factor: &mut f64) {
    egui::ComboBox::from_id_salt(id)
        .width(360.0)
        .selected_text(g.label())
        .show_ui(ui, |ui| {
            for k in Generator::ALL {
                if ui.selectable_label(*g == k, k.label()).clicked() && *g != k {
                    *g = k;
                    *factor = k.factor();
                }
            }
        });
}

impl CreaBimApp {
    pub(super) fn open_table30(&mut self) {
        let mut c = Table30::of(self.doc.model());
        // dev shots: the needs typed in ("108.92,15.27")
        if let Ok(v) = std::env::var("CREABIM_SHOT_TABLE30") {
            let n: Vec<f64> = v.split(',').filter_map(|x| x.trim().parse().ok()).collect();
            (c.heating, c.water) = (
                n.first().copied().unwrap_or(0.0),
                n.get(1).copied().unwrap_or(0.0),
            );
        }
        self.table30 = Some(c);
    }

    /// The window, while it is open.
    pub(super) fn table30_window(&mut self, ctx: &egui::Context) {
        let Some(mut c) = self.table30.clone() else {
            return;
        };
        let t = crate::i18n::tr;
        let num = crate::i18n::num;
        let mut open = true;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Table 30 for ISOVER Fragment"))
            .id(egui::Id::new("fragment.table30"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(300.0, 220.0))
            .default_size(crate::app::shell::at_least(Vec2::new(600.0, 420.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                ui.label(t(
                    "Type the energy need Fragment shows in table 30 (column Potreba energie), after CALC.",
                ));
                ui.add_space(4.0);
                egui::Grid::new("fragment.table30.in")
                    .num_columns(2)
                    .spacing([14.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(t("Heating: energy need"));
                        ui.add(
                            egui::DragValue::new(&mut c.heating)
                                .range(0.0..=2000.0)
                                .speed(0.1)
                                .suffix(" kWh/(m²a)"),
                        );
                        ui.end_row();
                        ui.label(t("Heating: made by"));
                        generator_combo(ui, "fragment.heat.by", &mut c.heat_by, &mut c.heat_factor);
                        ui.end_row();
                        ui.label(t("Heating: factor"));
                        ui.add(
                            egui::DragValue::new(&mut c.heat_factor)
                                .range(0.5..=8.0)
                                .speed(0.01),
                        );
                        ui.end_row();
                        ui.label(t("Hot water: energy need"));
                        ui.add(
                            egui::DragValue::new(&mut c.water)
                                .range(0.0..=500.0)
                                .speed(0.1)
                                .suffix(" kWh/(m²a)"),
                        );
                        ui.end_row();
                        ui.label(t("Hot water: made by"));
                        generator_combo(ui, "fragment.water.by", &mut c.water_by, &mut c.water_factor);
                        ui.end_row();
                        ui.label(t("Hot water: factor"));
                        ui.add(
                            egui::DragValue::new(&mut c.water_factor)
                                .range(0.5..=8.0)
                                .speed(0.01),
                        );
                        ui.end_row();
                    });
                ui.weak(t(
                    "Factors from Fragment's table Transformačné koeficienty; a heat pump maker's SCOP may replace them.",
                ));
                ui.separator();
                let (he, ha) = table30_row(c.heating, c.heat_by, c.heat_factor);
                let (we, wa) = table30_row(c.water, c.water_by, c.water_factor);
                egui::Grid::new("fragment.table30.out")
                    .num_columns(3)
                    .striped(true)
                    .spacing([18.0, 6.0])
                    .show(ui, |ui| {
                        ui.strong(t("Row"));
                        ui.strong(t("Elektrická energia"));
                        ui.strong(t("Energetický nosič n (energia prostredia – TČ)"));
                        ui.end_row();
                        for (row, e, a) in [("Vykurovanie", he, ha), ("Príprava teplej vody", we, wa)] {
                            ui.label(row);
                            ui.label(egui::RichText::new(num(e, 2)).strong());
                            ui.label(egui::RichText::new(num(a, 2)).strong());
                            ui.end_row();
                        }
                    });
                ui.weak(t("Every other column of these rows: 0."));
            });
        self.table30 = if open { Some(c) } else { None };
    }
}
