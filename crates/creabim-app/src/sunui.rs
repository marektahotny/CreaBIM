//! Sun and shadows of the 3D view: the window that sets them, and the
//! light each frame is drawn with.

use super::CreaBimApp;
use crate::render3d::{Lighting, RenderMode, Shadows};
use crate::sun::{Sun, SunMode, DEFAULT_PLACE};
use eframe::egui::{self, Vec2};

/// What the window says about the graphics card.
#[derive(Clone, Debug, Default)]
pub struct Gpu {
    pub name: String,
    pub backend: String,
    /// A card of its own, not the processor's graphics or software.
    pub discrete: bool,
    /// Drawn by the processor, without a graphics card at all.
    pub software: bool,
}

impl Gpu {
    pub fn from_info(i: &eframe::egui_wgpu::wgpu::AdapterInfo) -> Self {
        use eframe::egui_wgpu::wgpu::DeviceType;
        Self {
            name: i.name.clone(),
            backend: format!("{:?}", i.backend),
            discrete: i.device_type == DeviceType::DiscreteGpu,
            software: i.device_type == DeviceType::Cpu,
        }
    }
}

impl CreaBimApp {
    /// The project's latitude and longitude, Bratislava without them, and
    /// whether they are the project's own.
    fn sun_place(&self) -> ([f64; 2], bool) {
        match self.doc.model().project_info().map(|(_, p)| p) {
            Some(p) => match (p.latitude, p.longitude) {
                (Some(la), Some(lo)) => ([la, lo], true),
                _ => (DEFAULT_PLACE, false),
            },
            None => (DEFAULT_PLACE, false),
        }
    }

    fn true_north(&self) -> f64 {
        self.doc
            .model()
            .project_info()
            .map_or(0.0, |(_, p)| p.true_north)
    }

    /// The light the 3D view draws this frame with.
    pub(super) fn lighting_3d(&self) -> Lighting {
        let s = &self.shell.settings.sun;
        let (place, _) = self.sun_place();
        let (_, altitude) = s.position(place);
        let mut sun = s.direction(place, self.true_north());
        // below the horizon: a sun grazing it, everything in shadow
        let night = altitude <= 0.0;
        if night {
            let h = (sun[0] * sun[0] + sun[1] * sun[1]).sqrt().max(1e-6);
            sun = [sun[0] / h * 0.999, sun[1] / h * 0.999, 0.045];
        }
        let surfaces = self.shell.settings.render_3d != RenderMode::Wireframe;
        Lighting {
            sun,
            shadows: (s.shadows && surfaces).then(|| Shadows {
                strength: if night { 1.0 } else { s.strength },
                ground: s.ground.then_some(0.0),
                resolution: s.resolution,
                focus: self.camera.target.to_array(),
                distance: self.camera.distance,
            }),
        }
    }

    /// `sun3d` and `shadows3d` from the 3D view's menu and header.
    pub(super) fn sun_action(&mut self, id: &str) -> bool {
        match id {
            "sun3d" => self.sun_open = true,
            "shadows3d" => {
                let s = &mut self.shell.settings;
                s.sun.shadows = !s.sun.shadows;
                s.store();
                self.status = crate::i18n::tr(if s.sun.shadows {
                    "Shadows shown in the 3D view. View › Sun and shadows sets the sun."
                } else {
                    "Shadows hidden in the 3D view."
                });
            }
            _ => return false,
        }
        true
    }

    /// The Sun and shadows window, while it is open.
    pub(super) fn sun_window(&mut self, ctx: &egui::Context) {
        if !self.sun_open {
            return;
        }
        let t = crate::i18n::tr;
        let (place, own_place) = self.sun_place();
        let true_north = self.true_north();
        let gpu = self.gpu.clone();
        let mut s = self.shell.settings.sun;
        // the place the sun stands over: the project's own, typed here too
        let mut at = place;
        let mut open = true;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Sun and shadows"))
            .id(egui::Id::new("sun.shadows"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(180.0, 220.0))
            .default_width(360.0)
            .resizable(false)
            .constrain_to(area)
            .show(ctx, |ui| {
                ui.checkbox(&mut s.shadows, t("Cast shadows in the 3D view"));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut s.mode, SunMode::Time, t("Date and time"));
                    ui.selectable_value(&mut s.mode, SunMode::Manual, t("Set by hand"));
                });
                egui::Grid::new("sun.grid")
                    .num_columns(2)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| match s.mode {
                        SunMode::Time => {
                            ui.label(t("Date"));
                            ui.horizontal(|ui| {
                                let dim = crate::sun::days_in_month(s.year, s.month);
                                ui.add(egui::DragValue::new(&mut s.day).range(1..=dim));
                                ui.label(".");
                                ui.add(egui::DragValue::new(&mut s.month).range(1..=12));
                                ui.label(".");
                                ui.add(egui::DragValue::new(&mut s.year).range(1900..=2200));
                            });
                            ui.end_row();
                            ui.label(t("Time"));
                            ui.horizontal(|ui| {
                                let mut minutes = (s.hour * 60.0).round() as i32;
                                ui.add(
                                    egui::Slider::new(&mut minutes, 0..=24 * 60 - 1)
                                        .step_by(5.0)
                                        .show_value(false),
                                );
                                ui.label(format!("{:02}:{:02}", minutes / 60, minutes % 60));
                                s.hour = minutes as f64 / 60.0;
                            });
                            ui.end_row();
                            ui.label("");
                            ui.label(if s.summer_time {
                                t("Central European summer time (UTC+2)")
                            } else {
                                t("Central European time (UTC+1)")
                            });
                            ui.end_row();
                            ui.label(t("Quick dates"));
                            ui.horizontal_wrapped(|ui| {
                                for (label, m, d) in [
                                    ("21 March", 3, 21),
                                    ("21 June", 6, 21),
                                    ("23 September", 9, 23),
                                    ("21 December", 12, 21),
                                ] {
                                    if ui.small_button(t(label)).clicked() {
                                        s.month = m;
                                        s.day = d;
                                    }
                                }
                            });
                            ui.end_row();
                        }
                        SunMode::Manual => {
                            ui.label(t("Azimuth"));
                            ui.add(
                                egui::Slider::new(&mut s.azimuth, 0.0..=360.0)
                                    .suffix("°")
                                    .fixed_decimals(0),
                            )
                            .on_hover_text(t("Degrees clockwise from north: 90° east, 180° south, 270° west"));
                            ui.end_row();
                            ui.label(t("Height above the horizon"));
                            ui.add(
                                egui::Slider::new(&mut s.altitude, 1.0..=90.0)
                                    .suffix("°")
                                    .fixed_decimals(0),
                            )
                            .on_hover_text(t("Degrees above the horizon"));
                            ui.end_row();
                        }
                    });
                s.day = s.day.min(crate::sun::days_in_month(s.year, s.month));
                s.summer_time = Sun::summer_on(s.year, s.month, s.day);
                let (az, alt) = s.position(place);
                ui.add_space(4.0);
                if alt > 0.0 {
                    ui.label(format!(
                        "{}: {}°, {}: {}°",
                        t("Sun azimuth"),
                        crate::i18n::num(az, 1),
                        t("height above the horizon"),
                        crate::i18n::num(alt, 1)
                    ));
                } else {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        t("The sun is below the horizon: everything is in shadow."),
                    );
                }
                if s.mode == SunMode::Time {
                    ui.horizontal(|ui| {
                        ui.label(t("Location"));
                        ui.add(
                            egui::DragValue::new(&mut at[0])
                                .range(-90.0..=90.0)
                                .speed(0.01)
                                .max_decimals(4)
                                .suffix("° N"),
                        )
                        .on_hover_text(t("Latitude of the project, degrees north"));
                        ui.add(
                            egui::DragValue::new(&mut at[1])
                                .range(-180.0..=180.0)
                                .speed(0.01)
                                .max_decimals(4)
                                .suffix("° E"),
                        )
                        .on_hover_text(t("Longitude of the project, degrees east"));
                    });
                    let where_ = if own_place {
                        t("At the project's place")
                    } else {
                        t("In Bratislava: the project has no latitude and longitude yet (Manage › Project information)")
                    };
                    ui.label(
                        egui::RichText::new(format!(
                            "{where_} ({}° N, {}° E){}",
                            crate::i18n::num(place[0], 3),
                            crate::i18n::num(place[1], 3),
                            if true_north.abs() > 1e-9 {
                                format!(
                                    ", {} {}°",
                                    t("true north turned"),
                                    crate::i18n::num(true_north, 1)
                                )
                            } else {
                                String::new()
                            }
                        ))
                        .small()
                        .weak(),
                    );
                }
                ui.separator();
                egui::Grid::new("shadow.grid")
                    .num_columns(2)
                    .spacing([10.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(t("Shadow darkness"));
                        ui.add(
                            egui::Slider::new(&mut s.strength, 0.1..=1.0)
                                .custom_formatter(|v, _| format!("{:.0} %", v * 100.0)),
                        );
                        ui.end_row();
                        ui.label(t("Sharpness"));
                        ui.horizontal(|ui| {
                            for (side, label) in
                                [(2048, "Fast"), (4096, "Sharp"), (8192, "Very sharp")]
                            {
                                ui.selectable_value(&mut s.resolution, side, t(label))
                                    .on_hover_text(format!(
                                        "{} {side} × {side} px",
                                        t("Shadow map")
                                    ));
                            }
                        });
                        ui.end_row();
                    });
                ui.checkbox(&mut s.ground, t("Shadows on the ground at ±0,000"))
                    .on_hover_text(t("Where no terrain or slab is drawn, shadows still fall on the ground around the building"));
                ui.separator();
                ui.label(egui::RichText::new(t("Graphics card")).strong());
                match &gpu {
                    Some(g) => {
                        ui.label(format!("{} ({})", g.name, g.backend));
                        if g.software {
                            ui.colored_label(
                                ui.visuals().warn_fg_color,
                                t("Drawn by the processor, without the graphics card: update the graphics driver"),
                            );
                        } else if g.discrete {
                            ui.label(
                                egui::RichText::new(t("The dedicated graphics card draws the whole window and the 3D view"))
                                    .small()
                                    .weak(),
                            );
                        } else {
                            ui.label(
                                egui::RichText::new(t("The processor's built-in graphics draws: on a laptop with two cards, set CreaBIM to High performance in Windows Settings › Display › Graphics"))
                                    .small()
                                    .weak(),
                            );
                        }
                    }
                    None => {
                        ui.label(t("3D needs the wgpu renderer"));
                    }
                }
            });
        if at != place {
            // written to the project information, as Manage › Project
            // information does
            if let Some((id, _)) = self.doc.model().project_info() {
                let r = self.doc.edit_merged(id, "Project place", |e| {
                    if let creabim_core::ElementKind::Project(p) = &mut e.kind {
                        p.latitude = Some(at[0]);
                        p.longitude = Some(at[1]);
                    }
                });
                if let Err(e) = r {
                    self.status = format!("Couldn't do that: {e}");
                }
            }
        }
        if s != self.shell.settings.sun {
            self.shell.settings.sun = s;
            self.shell.settings.store();
            ctx.request_repaint();
        }
        self.sun_open = open;
    }
}
