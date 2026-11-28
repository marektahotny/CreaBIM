//! Underfloor heating ([`creabim_services::floorheat`], STN EN 1264): a
//! window with every room's design from its heat loss and the floor
//! build-up it stands on, a switch that gives a room its loops (drawn in
//! the plan), its spacing and how they are laid, its edge zones, places
//! of reduced output and holes, where its loops are split, and what to
//! set at the manifold.

use super::CreaBimApp;
use crate::planview::Tool;
use crate::roomtool::UfhZone;
use creabim_core::underfloor::{Layout, Split, Underfloor, ZoneKind};
use creabim_core::{ElementId, ElementKind, Transaction};
use creabim_services::floorheat;
use eframe::egui::{self, Vec2};

impl CreaBimApp {
    /// `calc:floorheat` from the Heating tab: open the window; `ufh:edge`,
    /// `ufh:reduced`, `ufh:hole`: draw such a zone in a room.
    pub(super) fn floor_heat_action(&mut self, head: &str, arg: &str) -> bool {
        if head == "heat" && arg == "bill" {
            self.open_heating_bill();
            return true;
        }
        if head == "ufh" {
            let kind = match arg {
                "edge" => UfhZone::Edge,
                "reduced" => UfhZone::Reduced,
                "hole" => UfhZone::Hole,
                _ => return false,
            };
            self.start_ufh_zone(kind);
            return true;
        }
        if head != "calc" || arg != "floorheat" {
            return false;
        }
        if self.floor_heat.is_none() {
            // reopened: as it was last written (George 2026-11-27)
            self.floor_heat = Some(self.saved_settings("floorheat").unwrap_or_default());
        }
        true
    }

    /// The heating's bill of materials: its schedule (the heating's
    /// pipe list), made when the project has none, opened.
    pub(super) fn open_heating_bill(&mut self) {
        use creabim_core::viewmodes::Trade;
        use creabim_core::{Element, ScheduleCategory, View, ViewKind};
        let found = self.doc.model().iter().find_map(|e| match &e.kind {
            ElementKind::View(v)
                if matches!(v.kind, ViewKind::Schedule { category, .. } if category == ScheduleCategory::PipeList)
                    && v.trade == Some(Trade::Heating) =>
            {
                Some(e.id)
            }
            _ => None,
        });
        let id = match found {
            Some(id) => id,
            None => {
                let mut v = View::schedule(ScheduleCategory::PipeList);
                v.trade = Some(Trade::Heating);
                let e = Element::new(
                    format!("{} VYK", crate::i18n::tr("Bill of materials")),
                    ElementKind::View(v),
                );
                let id = e.id;
                if let Err(e) = self
                    .doc
                    .commit(Transaction::new("Bill of materials").insert(e))
                {
                    self.status = format!("Couldn't do that: {e}");
                    return;
                }
                id
            }
        };
        self.active_view = Some(id);
    }

    /// The zone tool, drawing `kind` as a polygon or a rectangle.
    fn start_ufh_zone(&mut self, kind: UfhZone) {
        self.plan.tool = Tool::UfhZone;
        self.plan.cancel();
        self.plan.ufh_zone = kind;
        if !matches!(
            self.plan.slab_mode,
            crate::slabtool::SlabMode::Polygon | crate::slabtool::SlabMode::Rectangle
        ) {
            self.plan.slab_mode = crate::slabtool::SlabMode::Polygon;
        }
        self.status = format!(
            "{}: {}",
            crate::i18n::tr(kind.label()),
            crate::i18n::tr("click its corners (1 Polygon) or two opposite corners (2 Rectangle) inside a room with underfloor heating.")
        );
    }

    /// The window, while it is open.
    pub(super) fn floor_heat_window(&mut self, ctx: &egui::Context) {
        let Some(mut s) = self.floor_heat.clone() else {
            return;
        };
        let t = crate::i18n::tr;
        let num = crate::i18n::num;
        let o = self.heat_opts();
        let d = floorheat::design(self.doc.model(), &o, &s);
        let found = creabim_services::calcresults::floor_heat(&d).with_settings(&s);
        let written = self.is_written(&found);
        let mut write_now = false;
        let mut bill = false;
        let mut open = true;
        let mut select = None;
        let mut draw: Option<UfhZone> = None;
        // a manifold made to order, to place
        let mut place: Option<creabim_core::catalogue::ufhmanifold::ManifoldConfig> = None;
        // a room's underfloor heating to set (none takes it away)
        let mut set: Vec<(ElementId, Option<Underfloor>)> = vec![];
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(t("Underfloor heating"))
            .id(egui::Id::new("floor.heat"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(420.0, 300.0))
            .default_size(crate::app::shell::at_least(Vec2::new(900.0, 620.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if super::calcresultsui::write_button(ui, written) {
                        write_now = true;
                    }
                    if ui
                        .button(t("Bill of materials"))
                        .on_hover_text(t("The heating's bill of materials, as last written"))
                        .clicked()
                    {
                        bill = true;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(t("Supply temperature"));
                    ui.add(
                        egui::DragValue::new(&mut s.supply_c)
                            .range(25.0..=55.0)
                            .speed(0.5)
                            .suffix(" °C"),
                    );
                    ui.label(t("Drop"));
                    ui.add(
                        egui::DragValue::new(&mut s.sigma_k)
                            .range(3.0..=10.0)
                            .speed(0.5)
                            .suffix(" K"),
                    );
                    ui.weak(format!(
                        "θe {} °C ({})",
                        num(o.te, 0),
                        t("from the heat loss window")
                    ));
                });
                // the pipe: the board maker's by default, or one picked
                ui.horizontal(|ui| {
                    ui.label(t("Pipe"));
                    let now = d.pipe.clone();
                    let shown = now.as_ref().map_or(String::new(), |p| {
                        format!("{} {}{}", p.name, p.segment, code(&p.code))
                    });
                    let auto = s.pipe.is_none();
                    egui::ComboBox::from_id_salt("floor.heat.pipe")
                        .width(420.0)
                        .selected_text(if auto {
                            format!("{} ({})", shown, t("by the system board"))
                        } else {
                            shown
                        })
                        .show_ui(ui, |ui| {
                            if ui.selectable_label(auto, t("By the system board's maker")).clicked() {
                                s.pipe = None;
                                s.pipe_mm = None;
                            }
                            for ty in floorheat::ufh_pipes() {
                                for z in &ty.sizes {
                                    let on = !auto
                                        && s.pipe.as_deref() == Some(ty.id.as_str())
                                        && now.as_ref().is_some_and(|p| (p.outer_mm - z.outer_mm).abs() < 1e-6);
                                    let label = format!(
                                        "{} {}{}",
                                        ty.name,
                                        z.segment.clone().unwrap_or_else(|| format!("{}", z.outer_mm)),
                                        code(z.code.as_deref().unwrap_or(""))
                                    );
                                    if ui.selectable_label(on, label).clicked() {
                                        s.pipe = Some(ty.id.clone());
                                        s.pipe_mm = Some(z.outer_mm);
                                    }
                                }
                            }
                        });
                });
                if d.rooms.is_empty() {
                    ui.label(t("No rooms to calculate: place rooms first."));
                    return;
                }
                let on: Vec<&floorheat::RoomDesign> = d.rooms.iter().filter(|x| x.on).collect();
                let marks = creabim_core::marks(self.doc.model());
                ui.horizontal(|ui| {
                    if ui.button(t("All rooms")).clicked() {
                        for x in d.rooms.iter().filter(|x| !x.on) {
                            set.push((x.id, Some(x.underfloor(None))));
                        }
                    }
                    if ui
                        .add_enabled(!on.is_empty(), egui::Button::new(t("No room")))
                        .clicked()
                    {
                        for x in &on {
                            set.push((x.id, None));
                        }
                    }
                    if !on.is_empty() {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} kW · {} {} · {} m³/h · {} {} kPa",
                                num(d.output_kw, 2),
                                d.loops,
                                t("loops"),
                                num(d.flow_ls * 3.6, 2),
                                t("pump head"),
                                num(d.head_kpa, 1),
                            ))
                            .strong(),
                        );
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(t("Draw in a room:"));
                    for k in [UfhZone::Edge, UfhZone::Reduced, UfhZone::Hole] {
                        if ui.button(t(k.label())).clicked() {
                            draw = Some(k);
                        }
                    }
                });
                if !d.norm {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        t("The tables of STN EN 1264-2 are not loaded (Manage › Norm tables): the heat flux is approximated."),
                    );
                }
                if d.no_manifold {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        t("No manifold stands in the model: the loops' way to it is not counted. Place one from the catalogue (Underfloor heating manifold)."),
                    );
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    // step 1: the manifold and the pipes of each role
                    if !d.manifolds.is_empty() {
                        egui::CollapsingHeader::new(t("Manifold and pipes"))
                            .default_open(true)
                            .show(ui, |ui| {
                                manifold_ui(ui, self.doc.model(), &d, &mut s, &mut place);
                            });
                        ui.add_space(4.0);
                    }
                    use crate::sheet::{self, Col};
                    let cols = [
                        Col::text(""),
                        Col::text("Room"),
                        Col::num("Φ", "W"),
                        Col::num("q / needed", "W/m²"),
                        Col::text("Spacing"),
                        Col::text("Laying"),
                        Col::text("Connected by"),
                        Col::num("RλB", "m²K/W"),
                        Col::num("su", "mm"),
                        Col::num("qu", "W/m²"),
                        Col::num("Loops", "pcs"),
                        Col::num("Loop", "m"),
                        Col::num("Flow", "l/h"),
                        Col::num("Δp", "kPa"),
                    ];
                    let total = |row: &mut sheet::Row| {
                        row.empty();
                        row.text(t("Total"));
                        row.text(num(on.iter().map(|x| x.phi).sum::<f64>(), 0));
                        for _ in 0..7 {
                            row.empty();
                        }
                        row.text(d.loops.to_string());
                        row.empty();
                        row.text(num(d.flow_ls * 3600.0, 0));
                        row.text(num(d.critical_kpa, 1));
                    };
                    sheet::show(ui, "floor.heat.rooms", &cols, d.rooms.len(), 400.0, |row, i| {
                        let x = &d.rooms[i];
                        row.cell(|ui| {
                            let mut is_on = x.on;
                            if ui.checkbox(&mut is_on, "").changed() {
                                set.push((x.id, is_on.then(|| x.underfloor(None))));
                            }
                        });
                        let name = format!("{} {}", x.number, x.name);
                        row.cell(|ui| {
                            let r = ui.link(name.trim());
                            let r = if x.notes.is_empty() {
                                r
                            } else {
                                ui.colored_label(ui.visuals().warn_fg_color, "⚠").on_hover_text(
                                    x.notes.iter().map(|n| t(n.text())).collect::<Vec<_>>().join("\n"),
                                );
                                r
                            };
                            if r.clicked() {
                                select = Some(x.id);
                            }
                        });
                        row.text(num(x.phi, 0));
                        let q = format!("{} / {}", num(x.q, 0), num(x.q_req, 0));
                        if x.ok {
                            row.text(q);
                        } else {
                            row.cell(|ui| {
                                ui.colored_label(ui.visuals().warn_fg_color, q).on_hover_text(format!(
                                    "{} {} W {} {} W (qG {} W/m²)",
                                    t("The floor gives"),
                                    num(x.covered, 0),
                                    t("of"),
                                    num(x.phi, 0),
                                    num(x.q_limit, 0),
                                ));
                            });
                        }
                        row.cell(|ui| {
                            if x.on {
                                let mut sp = x.spacing;
                                egui::ComboBox::from_id_salt(("floor.heat.t", x.id))
                                    .width(70.0)
                                    .selected_text(format!("{} mm", num(sp * 1000.0, 0)))
                                    .show_ui(ui, |ui| {
                                        for v in &x.spacings {
                                            ui.selectable_value(
                                                &mut sp,
                                                *v,
                                                format!("{} mm", num(v * 1000.0, 0)),
                                            );
                                        }
                                    });
                                if (sp - x.spacing).abs() > 1e-9 {
                                    // loops for the new spacing
                                    let coil = x.area / sp;
                                    let loops = ((coil + 2.0 * x.feed) / floorheat::MAX_LOOP_M)
                                        .ceil()
                                        .max(1.0) as u32;
                                    let mut u = x.underfloor(None);
                                    u.spacing = sp;
                                    u.loops = loops;
                                    set.push((x.id, Some(u)));
                                }
                            } else {
                                ui.weak(format!("{} mm", num(x.spacing * 1000.0, 0)));
                            }
                        });
                        row.cell(|ui| {
                            if let (true, Some(u)) = (x.on, &x.ufh) {
                                let mut l = u.layout;
                                egui::ComboBox::from_id_salt(("floor.heat.l", x.id))
                                    .width(150.0)
                                    .selected_text(t(l.label()))
                                    .show_ui(ui, |ui| {
                                        for v in Layout::ALL {
                                            ui.selectable_value(&mut l, v, t(v.label()));
                                        }
                                    });
                                if l != u.layout {
                                    let mut u = u.clone();
                                    u.layout = l;
                                    set.push((x.id, Some(u)));
                                }
                            }
                        });
                        // the doorway the loops come in by from the manifold
                        row.cell(|ui| {
                            let (true, Some(u)) = (x.on, &x.ufh) else {
                                return;
                            };
                            if x.doors.is_empty() {
                                ui.weak("–");
                                return;
                            }
                            let name = |id: ElementId| {
                                marks.get(&id).cloned().unwrap_or_else(|| t("Door"))
                            };
                            let shown = match (u.door, x.door) {
                                (Some(d), _) => name(d),
                                (None, Some(d)) => format!("{} ({})", name(d), t("auto")),
                                _ => t("auto"),
                            };
                            let mut pick = u.door;
                            egui::ComboBox::from_id_salt(("floor.heat.door", x.id))
                                .width(90.0)
                                .selected_text(shown)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut pick, None, t("auto"));
                                    for w in &x.doors {
                                        ui.selectable_value(&mut pick, Some(w.id), name(w.id));
                                    }
                                });
                            if pick != u.door {
                                let mut u = u.clone();
                                u.door = pick;
                                set.push((x.id, Some(u)));
                            }
                        });
                        row.text(num(x.finish_r, 3));
                        row.text(num(x.su * 1000.0, 0));
                        row.text(num(x.q_down, 1));
                        row.text(x.loops.to_string());
                        row.text(num(x.loop_len, 0));
                        row.text(num(x.flow_ls * 3600.0, 0));
                        row.text(num(x.dp_kpa, 1));
                    }, Some(&total));
                    ui.add_space(6.0);
                    // the rooms' zones and how their loops are split
                    let zoned: Vec<&floorheat::RoomDesign> = d
                        .rooms
                        .iter()
                        .filter(|x| x.on && x.ufh.as_ref().is_some_and(|u| !u.zones.is_empty() || u.loops > 1))
                        .collect();
                    if !zoned.is_empty() {
                        egui::CollapsingHeader::new(t("Zones and loops split"))
                            .default_open(true)
                            .show(ui, |ui| {
                                for x in zoned {
                                    let Some(u) = &x.ufh else { continue };
                                    zones_ui(ui, x, u, &mut set);
                                }
                            });
                    }
                    // what the floors are built of
                    let floors = d.floors();
                    egui::CollapsingHeader::new(t("Floor build-ups"))
                        .default_open(true)
                        .show(ui, |ui| {
                            if floors.is_empty() {
                                ui.weak(t("No room with underfloor heating stands on a floor with a build-up."));
                            }
                            for (f, rooms) in floors {
                                floor_ui(ui, f, &rooms);
                            }
                        });
                    ui.add_space(6.0);
                    ui.weak(t("System A after STN EN 1264-2: λE and the cover su of the screed over the system board, RλB of the layers on the screed, the loss downwards qu from the insulation under the pipes (STN EN 1264-3, to the room under it or the ground at the year's mean outside temperature). Spacings as the board allows, loops of 100 m at most. Yellow: the floor alone does not cover the room's loss: add a radiator or raise the supply temperature."));
                    ui.weak("STN EN 1264-2 · STN EN 1264-3 · STN EN 1264-4 · STN EN 12831-1");
                });
            });
        if let Some(id) = select {
            self.selection.only(id);
        }
        if let Some(k) = draw {
            self.start_ufh_zone(k);
            self.park(t(k.label()));
        }
        if let Some(c) = place {
            let unit = creabim_core::catalogue::ufhmanifold::manifold(&c);
            self.status = format!(
                "{}: {}",
                unit.name,
                t("click in a plan or the 3D view to place it. Space turns it; Esc ends.")
            );
            let what = unit.name.clone();
            self.plan.objects.current = Some(unit);
            self.plan.objects.offset = None;
            self.plan.objects.project_type = None;
            self.plan.cancel();
            self.plan.tool = super::Tool::Object;
            self.park(what);
        }
        if !set.is_empty() {
            let mut tx = Transaction::new("Underfloor heating");
            for (id, u) in set {
                let Some(before) = self.doc.model().get(id).cloned() else {
                    continue;
                };
                let mut after = before.clone();
                if let ElementKind::Room(r) = &mut after.kind {
                    r.ufh = u;
                }
                tx = tx.replace(before, after);
            }
            match self.doc.commit(tx) {
                Ok(_) => self.status = t("Underfloor heating"),
                Err(e) => self.status = format!("Couldn't do that: {e}"),
            }
        }
        if write_now {
            self.write_results_said(found);
        }
        if bill {
            self.open_heating_bill();
        }
        self.floor_heat = if open { Some(s) } else { None };
    }
}

fn code(c: &str) -> String {
    if c.trim().is_empty() {
        String::new()
    } else {
        format!(" ({})", c.trim())
    }
}

/// A room's zones (on, kind, area, spacing or share, remove) and where
/// its loops are split.
fn zones_ui(
    ui: &mut egui::Ui,
    x: &floorheat::RoomDesign,
    u: &Underfloor,
    set: &mut Vec<(ElementId, Option<Underfloor>)>,
) {
    let t = crate::i18n::tr;
    let num = crate::i18n::num;
    ui.label(egui::RichText::new(format!("{} {}", x.number, x.name).trim().to_string()).strong());
    let mut v = u.clone();
    let mut drop = None;
    ui.indent(("ufh.zones", x.id), |ui| {
        for (k, z) in v.zones.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.checkbox(&mut z.on, "");
                let area = z.area();
                match &mut z.kind {
                    ZoneKind::Edge { spacing } => {
                        ui.label(format!("{} · {} m²", t("Edge zone"), num(area, 2)));
                        ui.label(t("Spacing"));
                        let mut mm = *spacing * 1000.0;
                        egui::ComboBox::from_id_salt(("ufh.edge.t", x.id, k))
                            .width(70.0)
                            .selected_text(format!("{} mm", num(mm, 0)))
                            .show_ui(ui, |ui| {
                                let mut all: Vec<f64> =
                                    x.spacings.iter().map(|v| v * 1000.0).collect();
                                // the board's raster halved, as a nub board lays it
                                if let Some(r) = x.floor.as_ref().and_then(|f| f.spec.raster_mm) {
                                    all.insert(0, r);
                                }
                                if !all.iter().any(|v| (v - 50.0).abs() < 1e-6)
                                    && x.floor.as_ref().is_none_or(|f| f.spec.raster_mm.is_none())
                                {
                                    all.insert(0, 50.0);
                                }
                                for a in all {
                                    ui.selectable_value(&mut mm, a, format!("{} mm", num(a, 0)));
                                }
                            });
                        *spacing = mm / 1000.0;
                        if x.edge_area > 0.0 {
                            ui.weak(format!("q {} W/m²", num(x.q_edge, 0)));
                        }
                    }
                    ZoneKind::Reduced { percent } => {
                        ui.label(format!(
                            "{} · {} m²",
                            t("Place of reduced output"),
                            num(area, 2)
                        ));
                        ui.label(t("gives less by"));
                        ui.add(
                            egui::DragValue::new(percent)
                                .range(0.0..=100.0)
                                .speed(1.0)
                                .suffix(" %"),
                        );
                    }
                    ZoneKind::Hole => {
                        ui.label(format!("{} · {} m²", t("Hole with no pipe"), num(area, 2)));
                    }
                }
                if ui.small_button("✕").on_hover_text(t("Remove")).clicked() {
                    drop = Some(k);
                }
            });
        }
        if x.reduced_w > 0.0 {
            ui.weak(format!(
                "{} {} W",
                t("The places of reduced output take off"),
                num(x.reduced_w, 0)
            ));
        }
        if u.loops > 1 {
            ui.horizontal(|ui| {
                ui.label(t("Loops split"));
                let mut way = match &v.split {
                    None => 0,
                    Some(s) if s.vertical => 1,
                    Some(_) => 2,
                };
                let before = way;
                for (i, l) in ["evenly", "vertically", "horizontally"].iter().enumerate() {
                    ui.radio_value(&mut way, i, t(l));
                }
                if way != before {
                    v.split = match way {
                        0 => None,
                        w => Some(Split {
                            vertical: w == 1,
                            at: vec![],
                        }),
                    };
                }
            });
            if let Some(sp) = &mut v.split {
                ui.horizontal(|ui| {
                    ui.label(if sp.vertical { t("at x") } else { t("at y") });
                    while sp.at.len() < u.loops as usize - 1 {
                        sp.at.push(f64::NAN);
                    }
                    sp.at.truncate(u.loops as usize - 1);
                    for a in sp.at.iter_mut() {
                        let mut val = if a.is_finite() { *a } else { 0.0 };
                        let r = ui.add(egui::DragValue::new(&mut val).speed(0.01).suffix(" m"));
                        if r.changed() {
                            *a = val;
                        }
                    }
                    ui.weak(t("(plan coordinates, m)"));
                });
                sp.at.retain(|a| a.is_finite());
            }
        }
    });
    if let Some(k) = drop {
        v.zones.remove(k);
    }
    if v != *u {
        set.push((x.id, Some(v)));
    }
}

/// What a floor is built of, as the calculation reads it.
fn floor_ui(ui: &mut egui::Ui, f: &floorheat::FloorBuild, rooms: &[&str]) {
    let t = crate::i18n::tr;
    let num = crate::i18n::num;
    ui.label(egui::RichText::new(&f.assembly).strong());
    ui.indent(("ufh.floor", &f.assembly), |ui| {
        ui.weak(format!("{}: {}", t("Rooms"), rooms.join(", ")));
        ui.label(format!(
            "{}: {} — RλB {} m²K/W",
            t("On the screed"),
            if f.finish.is_empty() {
                "–".to_string()
            } else {
                f.finish.join(" + ")
            },
            num(f.finish_r, 3)
        ));
        ui.label(format!(
            "{}: {} {} mm, λE {} W/(m·K)",
            t("Heating layer (screed with the pipes)"),
            f.screed,
            num(f.screed_t * 1000.0, 0),
            num(f.lambda_e, 2)
        ));
        match &f.board {
            Some(b) => {
                let mut x = vec![];
                if let Some([lo, hi]) = f.spec.pipe_mm {
                    x.push(format!("{} {}–{} mm", t("pipes"), num(lo, 0), num(hi, 0)));
                }
                if let Some(r) = f.spec.raster_mm {
                    x.push(format!("{} {} mm", t("raster"), num(r, 0)));
                }
                if !f.spec.spacings_mm.is_empty() {
                    x.push(format!(
                        "{} {} mm",
                        t("spacings"),
                        f.spec
                            .spacings_mm
                            .iter()
                            .map(|v| num(*v, 0))
                            .collect::<Vec<_>>()
                            .join(" / ")
                    ));
                }
                ui.label(format!(
                    "{}: {}{}",
                    t("System board"),
                    b,
                    if x.is_empty() {
                        String::new()
                    } else {
                        format!(" ({})", x.join(", "))
                    }
                ));
            }
            None => {
                ui.colored_label(
                    ui.visuals().warn_fg_color,
                    t("The floor build-up has no underfloor heating system board"),
                );
            }
        }
        ui.label(format!(
            "{}: {} — R {} m²K/W",
            t("Under the pipes"),
            f.under.join(" + "),
            num(f.r_under, 2)
        ));
    });
}

/// A storey's manifold (the product its circuits pick, its range, cabinet
/// and mixing set, placing it) and the pipes of each role: the loops,
/// from the source to the manifold, from the manifold to the radiators
/// picked on it, with their sizes, flows and insulation.
fn manifold_ui(
    ui: &mut egui::Ui,
    model: &creabim_core::Model,
    d: &floorheat::Design,
    s: &mut floorheat::Settings,
    place: &mut Option<creabim_core::catalogue::ufhmanifold::ManifoldConfig>,
) {
    use creabim_core::catalogue::ufhmanifold::{Cabinet, Range};
    let t = crate::i18n::tr;
    let num = crate::i18n::num;
    let maker = floorheat::model_maker(model);
    ui.horizontal(|ui| {
        ui.label(t("Range"));
        let auto = Range::for_pipe(&maker);
        egui::ComboBox::from_id_salt("floor.heat.range")
            .width(300.0)
            .selected_text(match s.range {
                Some(r) => r.label().to_string(),
                None => format!("{} ({})", auto.label(), t("by the system board")),
            })
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(s.range.is_none(), t("By the system board's maker"))
                    .clicked()
                {
                    s.range = None;
                }
                for r in Range::ALL {
                    if ui.selectable_label(s.range == Some(r), r.label()).clicked() {
                        s.range = Some(r);
                    }
                }
            });
        ui.label(t("Cabinet"));
        egui::ComboBox::from_id_salt("floor.heat.cabinet")
            .selected_text(t(cabinet_label(s.cabinet)))
            .show_ui(ui, |ui| {
                for c in [Cabinet::Flush, Cabinet::Wall, Cabinet::None] {
                    ui.selectable_value(&mut s.cabinet, c, t(cabinet_label(c)));
                }
            });
        ui.checkbox(&mut s.mixing, t("Mixing set"));
    });
    ui.checkbox(&mut s.regulation, t("Regulation (actuators, thermostats)"));
    let level_name = |id| model.get(id).map_or(String::new(), |l| l.name.clone());
    for m in &d.manifolds {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.strong(level_name(m.level));
            match m.config.circuits() {
                Some(n) => {
                    ui.label(format!(
                        "{} · {} {} ({} + {})",
                        m.config.name(),
                        n,
                        t("circuits"),
                        m.config.loops,
                        m.config.extra
                    ));
                    if let Some(c) = m.config.cabinet_spec() {
                        ui.weak(format!(
                            "{} {} · {} × {} × {} mm",
                            t("cabinet"),
                            c.code,
                            num(c.width, 0),
                            num(c.height, 0),
                            num(c.depth, 0)
                        ));
                    } else if m.config.cabinet != Cabinet::None {
                        ui.colored_label(
                            ui.visuals().warn_fg_color,
                            t("The maker has no such cabinet for it"),
                        );
                    }
                    let label = if m.element.is_some() {
                        t("Place another")
                    } else {
                        t("Place it")
                    };
                    if ui.button(label).clicked() {
                        *place = Some(m.config.clone());
                    }
                }
                None => {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        format!(
                            "{} {} {}: {}",
                            m.config.needed(),
                            t("circuits"),
                            t("are more than the range makes"),
                            t("split the storey into two manifolds or pick another range")
                        ),
                    );
                }
            }
        });
        use crate::sheet::{self, Col};
        let cols = [
            Col::text("Pipe for"),
            Col::text("Pipe"),
            Col::text("Code"),
            Col::num("Flow", "l/h"),
            Col::num("v", "m/s"),
            Col::num("R", "Pa/m"),
            Col {
                title: "Insulation",
                unit: "mm",
                num: false,
            },
            Col::num("Length", "m"),
        ];
        // (role, pipe, code, flow l/h, v, R, insulation, length)
        let mut rows: Vec<[String; 8]> = vec![];
        if let Some(p) = &d.pipe {
            let (n, len) = d
                .rooms
                .iter()
                .filter(|x| x.on)
                .filter(|x| {
                    matches!(model.get(x.id).map(|e| &e.kind),
                    Some(creabim_core::ElementKind::Room(r)) if r.level == m.level)
                })
                .fold((0, 0.0), |(n, l), x| {
                    (n + x.loops, l + x.loop_len * x.loops as f64)
                });
            rows.push([
                format!("{} ({n})", t("Loops")),
                format!("{} {}", p.name, p.segment),
                p.code.clone(),
                num(
                    (m.flow_ls
                        - m.radiators
                            .iter()
                            .filter_map(|r| r.pipe.as_ref())
                            .map(|p| p.flow_ls)
                            .sum::<f64>())
                        * 3600.0,
                    0,
                ),
                String::new(),
                String::new(),
                "0".into(),
                num(len, 0),
            ]);
        }
        let sized = |role: String, p: &floorheat::SizedPipe, len: String| -> [String; 8] {
            [
                role,
                format!("{} {}", p.pipe.name, p.pipe.segment),
                p.pipe.code.clone(),
                num(p.flow_ls * 3600.0, 0),
                num(p.velocity, 2),
                num(p.r_pa_m, 0),
                // the least by the vyhláška at λ 0,035, the product's least
                // and its tube
                match &p.tube {
                    Some(tb) => format!(
                        "{} · {}: {} × {}{}",
                        num(p.insulation_mm, 0),
                        num(tb.least_mm, 0),
                        tb.inner_mm,
                        tb.wall_mm,
                        if tb.enough { "" } else { " !" }
                    ),
                    None => num(p.insulation_mm, 0),
                },
                len,
            ]
        };
        if let Some(p) = &m.feed {
            rows.push(sized(
                t("From the source to the manifold"),
                p,
                t("as drawn"),
            ));
        }
        for r in &m.radiators {
            let name = model.get(r.id).map_or(String::new(), |e| t(&e.name));
            if let Some(p) = &r.pipe {
                rows.push(sized(
                    format!(
                        "{} {} {} ({} W)",
                        t("Radiator"),
                        r.room,
                        name,
                        num(r.output_w, 0)
                    ),
                    p,
                    num(r.length, 1),
                ));
            }
        }
        sheet::show(
            ui,
            ("floor.heat.pipes", m.level),
            &cols,
            rows.len(),
            200.0,
            |row, i| {
                for c in &rows[i] {
                    row.text(c.clone());
                }
            },
            None,
        );
        ui.label(
            egui::RichText::new(t(
                "Insulation: the least thickness at λ 0.035 W/(m·K) by the pipe's inner diameter (vyhláška MH SR č. 14/2016 Z. z., príloha č. 1) · the product's least at its λ at 0 °C (príloha č. 2, α2 = 10 W/(m²·K)): its tube, inner diameter × wall, mm; ! not thick enough even at its thickest.",
            ))
            .small()
            .weak(),
        );
        // the circuits balanced: presetting, flow on the flowmeter
        if !m.circuits.is_empty() {
            let cols = [
                Col::text("Circuit"),
                Col::num("Flow", "l/h"),
                Col::num("Flowmeter", "l/min"),
                Col::num("Δp pipe", "kPa"),
                Col::text("Presetting"),
                Col::num("kv", "m³/h"),
                Col::num("Δp valve", "kPa"),
                Col::num("Δp total", "kPa"),
            ];
            sheet::show(
                ui,
                ("floor.heat.circuits", m.level),
                &cols,
                m.circuits.len(),
                220.0,
                |row, i| {
                    let c = &m.circuits[i];
                    let label = if c.radiator {
                        format!("{} {}", t("Radiator"), c.label)
                    } else {
                        c.label.clone()
                    };
                    if c.index {
                        row.cell(|ui| {
                            ui.strong(format!("{label} ({})", t("index")));
                        });
                    } else {
                        row.text(label);
                    }
                    row.text(num(c.flow_ls * 3600.0, 0));
                    let f = num(c.flow_lmin(), 2);
                    if c.over_flowmeter {
                        row.cell(|ui| {
                            ui.colored_label(ui.visuals().warn_fg_color, f)
                                .on_hover_text(t("More than the flowmeter shows: split the loop"));
                        });
                    } else {
                        row.text(f);
                    }
                    row.text(num(c.dp_pipe_kpa, 1));
                    match (c.preset, c.kv) {
                        (Some(p), Some(kv)) => {
                            if c.under {
                                row.cell(|ui| {
                                    ui.colored_label(ui.visuals().warn_fg_color, p)
                                        .on_hover_text(t(
                                            "Even the smallest presetting throttles too little",
                                        ));
                                });
                            } else {
                                row.text(p.to_string());
                            }
                            row.text(num(kv, 3));
                        }
                        _ => {
                            row.text(t("by the flowmeter"));
                            row.empty();
                        }
                    }
                    row.text(num(c.dp_valve_kpa, 1));
                    row.text(num(c.dp_pipe_kpa + c.dp_valve_kpa, 1));
                },
                None,
            );
            ui.weak(if m.config.range.presets().is_empty() {
                t("The maker gives no kv of its valves: every circuit is balanced by setting its flow on the flowmeter (l/min); the manifold's own loss is not counted.")
            } else {
                t("The index circuit (the worst) has its valve wide open; every other valve is preset to take what its pipe does not (kv = Q / √(Δp / 100), the maker's table). Set the flow of every circuit on its flowmeter as well.")
            });
        }
        // the radiators standing on the storey, to feed from the manifold
        let radiators: Vec<&creabim_core::Element> = model
            .iter()
            .filter(
                |e| matches!(&e.kind, creabim_core::ElementKind::Object(o) if o.level == m.level),
            )
            .filter(|e| {
                creabim_core::fixture::family_of(model, e)
                    .is_some_and(creabim_core::fixture::is_heating)
            })
            .collect();
        if !radiators.is_empty() {
            let numbers = floorheat::room_numbers(model, &radiators);
            ui.horizontal_wrapped(|ui| {
                ui.label(t("Radiators fed from the manifold:"));
                for e in radiators {
                    let room = numbers
                        .iter()
                        .find(|x| x.0 == e.id)
                        .map_or(String::new(), |x| x.1.clone());
                    let name = format!("{room} {}", t(&e.name)).trim().to_string();
                    let mut on = s.radiators.contains(&e.id);
                    if ui.checkbox(&mut on, name).changed() {
                        if on {
                            s.radiators.push(e.id);
                        } else {
                            s.radiators.retain(|x| *x != e.id);
                        }
                    }
                }
            });
        }
    }
    ui.horizontal(|ui| {
        ui.label(t("Radiators' supply / return"));
        ui.add(
            egui::DragValue::new(&mut s.radiator_supply_c)
                .range(35.0..=90.0)
                .speed(0.5)
                .suffix(" °C"),
        );
        ui.add(
            egui::DragValue::new(&mut s.radiator_return_c)
                .range(25.0..=80.0)
                .speed(0.5)
                .suffix(" °C"),
        );
    });
    ui.weak(t("Each pipe is its maker's: the loops the system board's (HERZ: plasthliník for underfloor heating, Wavin: PE-RT/EVOH), from the source to the manifold and to the radiators the maker's distribution pipe (HERZ PipeFix, Wavin Tigris K5), sized as the radiators' heating: 0.8 m/s on the main, 0.5 m/s to a radiator, 150 Pa/m. Radiators on the manifold are its extra outlets; their pipes run in the floor's insulation layer, insulated as heating pipes. The way to a radiator: as the crow flies × 1.2 until routes are drawn."));
}

fn cabinet_label(c: creabim_core::catalogue::ufhmanifold::Cabinet) -> &'static str {
    use creabim_core::catalogue::ufhmanifold::Cabinet;
    match c {
        Cabinet::Flush => "Under the plaster",
        Cabinet::Wall => "On the wall",
        Cabinet::None => "No cabinet",
    }
}
