//! Masonry (Architecture tab): the storey's masonry plan, where system
//! lintels are put by hand over the doors and windows picked (nothing is
//! placed by itself), RC columns put into walls, and the check of how far
//! masonry walls run between the walls and columns that brace them (see
//! [`creabim_core::masonry`]).

use super::CreaBimApp;
use creabim_core::masonry::{self, Family, LintelSet};
use creabim_core::{Element, ElementId, ElementKind, Material, Model, Transaction, ViewKind, Wall};
use eframe::egui::{self, Vec2};
use glam::DVec2;

/// The Masonry window's state.
#[derive(Clone, Debug)]
pub struct MasonryDraft {
    pub family: Family,
    /// The pieces' width; none takes the usual one for each wall.
    pub width: Option<f64>,
    /// How many pieces lie side by side; none, the usual for each wall.
    pub count: Option<usize>,
}

impl Default for MasonryDraft {
    fn default() -> Self {
        Self {
            family: Family::PorfixNp,
            width: None,
            count: None,
        }
    }
}

/// What the next click in a plan is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MasonryPick {
    /// The door or window to lay lintels over.
    Lintel,
    /// The point of a wall to put an RC column in.
    Column,
}

fn tr(s: &str) -> String {
    crate::i18n::tr(s)
}

/// The door, window or hole of a wall on `level` nearest `p` in plan,
/// when `p` is on it.
fn opening_at(model: &Model, level: ElementId, p: DVec2) -> Option<ElementId> {
    model
        .iter()
        .filter_map(|e| {
            let ElementKind::Opening(o) = &e.kind else {
                return None;
            };
            let ElementKind::Wall(w) = &model.get(o.host)?.kind else {
                return None;
            };
            if w.level != level {
                return None;
            }
            let d = w.direction();
            let u = (p - w.start).dot(d);
            let off = (p - w.start).perp_dot(d).abs();
            let along = if u < o.offset {
                o.offset - u
            } else if u > o.offset + o.width {
                u - o.offset - o.width
            } else {
                0.0
            };
            (off <= w.thickness / 2.0 + 0.3 && along < 0.15).then_some((e.id, off + along))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

/// The wall on `level` that `p` is on (or within a hand's breadth of).
fn wall_at(model: &Model, level: ElementId, p: DVec2) -> Option<ElementId> {
    model
        .iter()
        .filter_map(|e| {
            let ElementKind::Wall(w) = &e.kind else {
                return None;
            };
            if w.level != level || w.curtain.is_some() || w.bulge.abs() > 1e-9 {
                return None;
            }
            let d = w.direction();
            let u = (p - w.start).dot(d);
            if u < -0.05 || u > w.length() + 0.05 {
                return None;
            }
            let off = (p - w.start).perp_dot(d).abs();
            (off <= w.thickness / 2.0 + 0.1).then_some((e.id, off))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

/// The material lintels of `family` are drawn in: the project's of that
/// name, else a new one to insert.
fn lintel_material(model: &Model, family: Family) -> (ElementId, Option<Element>) {
    let (name, colour, hatch) = family.material();
    if let Some(e) = model
        .iter()
        .find(|e| matches!(e.kind, ElementKind::Material(_)) && e.name == name)
    {
        return (e.id, None);
    }
    let m = Material {
        category: if family == Family::PorfixU {
            "concrete".into()
        } else {
            "masonry".into()
        },
        ..Material::new(hatch, colour)
    };
    let e = Element::new(name, ElementKind::Material(m));
    (e.id, Some(e))
}

impl CreaBimApp {
    /// `masonry:<what>` from the Architecture tab's Masonry menu.
    pub(super) fn masonry_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "masonry" {
            return false;
        }
        match arg {
            "plan" => self.open_masonry_plan(),
            "window" => {
                if self.masonry.is_none() {
                    self.masonry = Some(MasonryDraft::default());
                }
            }
            "lintel" => {
                if self.masonry.is_none() {
                    self.masonry = Some(MasonryDraft::default());
                }
                self.lintels_over_selection_or_pick();
            }
            "column" => self.start_masonry_pick(MasonryPick::Column),
            "check" => self.check_masonry(),
            "fill" => self.fill_wall_columns(),
            _ => return false,
        }
        true
    }

    /// The masonry plan of the open plan's storey: opened, made the first
    /// time as a copy of the open plan.
    fn open_masonry_plan(&mut self) {
        let model = self.doc.model();
        let Some((now, level)) =
            self.active_plan()
                .and_then(|(id, level)| match &model.get(id)?.kind {
                    ElementKind::View(v) => Some((v.clone(), level)),
                    _ => None,
                })
        else {
            self.status = tr("Open a floor plan first: the masonry plan is made from it.");
            return;
        };
        if now.masonry {
            self.status = tr("This is the storey's masonry plan.");
            return;
        }
        let found = model.iter().find_map(|e| match &e.kind {
            ElementKind::View(v)
                if v.masonry
                    && v.callout.is_none()
                    && matches!(v.kind, ViewKind::Plan { level: l, .. } if l == level) =>
            {
                Some((e.id, e.name.clone()))
            }
            _ => None,
        });
        let (id, name) = match found {
            Some(f) => f,
            None => {
                let storey = model.get(level).map_or(String::new(), |e| e.name.clone());
                let mut v = now;
                v.browser_order = None;
                creabim_core::viewmodes::Purpose::Architecture.apply(&mut v);
                v.masonry = true;
                let name = format!("{storey} {}", tr("masonry"));
                let e = Element::new(name.clone(), ElementKind::View(v));
                let id = e.id;
                if self
                    .doc
                    .commit(Transaction::new("Masonry plan").insert(e))
                    .is_err()
                {
                    return;
                }
                (id, name)
            }
        };
        self.active_view = Some(id);
        self.status = tr(
            "Opened the masonry plan {}: lintels are put and shown here, sections show them cut.",
        )
        .replace("{}", &name);
    }

    fn start_masonry_pick(&mut self, pick: MasonryPick) {
        if self.active_plan().is_none() {
            self.status = tr("Open a floor plan first.");
            return;
        }
        self.masonry_pick = Some(pick);
        self.plan.point_pick = true;
        self.status = match pick {
            MasonryPick::Lintel => {
                tr("Lintels: click a door or window in the plan. Esc or a right-click leaves.")
            }
            MasonryPick::Column => {
                tr("RC column: click the wall where it goes. Esc or a right-click leaves.")
            }
        };
    }

    /// Lintels over the doors and windows selected; with none selected,
    /// the next one clicked.
    fn lintels_over_selection_or_pick(&mut self) {
        let model = self.doc.model();
        let picked: Vec<ElementId> = self
            .selection
            .all()
            .into_iter()
            .filter(|id| {
                matches!(
                    model.get(*id).map(|e| &e.kind),
                    Some(ElementKind::Opening(_))
                )
            })
            .collect();
        if picked.is_empty() {
            self.start_masonry_pick(MasonryPick::Lintel);
        } else {
            self.put_lintels(&picked);
        }
    }

    /// The window's set for the wall of `opening`.
    fn lintel_set(&self, wall: &Wall) -> LintelSet {
        let d = self.masonry.clone().unwrap_or_default();
        let usual = masonry::usual_set(self.doc.model(), wall, d.family);
        LintelSet {
            family: d.family,
            width: d.width.unwrap_or(usual.width),
            count: d.count.unwrap_or(usual.count),
        }
    }

    /// Lay lintels over `openings`, replacing the ones they had.
    fn put_lintels(&mut self, openings: &[ElementId]) {
        let model = self.doc.model();
        let family = self.masonry.clone().unwrap_or_default().family;
        let (mat, new_mat) = lintel_material(model, family);
        let mut tx = Transaction::new("Put lintels");
        if let Some(m) = new_mat {
            tx = tx.insert(m);
        }
        let mut put = 0;
        let mut errors = vec![];
        let mut new_ids = vec![];
        for o in openings {
            let Some(ElementKind::Opening(op)) = model.get(*o).map(|e| &e.kind) else {
                continue;
            };
            let Some(ElementKind::Wall(w)) = model.get(op.host).map(|e| &e.kind) else {
                continue;
            };
            let set = self.lintel_set(w);
            match masonry::lintels_for(model, *o, set) {
                Ok(pieces) => {
                    for old in masonry::lintels_over(model, *o) {
                        if let Some(e) = model.get(old) {
                            tx.ops.push(creabim_core::Op::Remove(e.clone()));
                        }
                    }
                    for mut p in pieces {
                        if let ElementKind::Beam(b) = &mut p.kind {
                            b.material = Some(mat);
                        }
                        new_ids.push(p.id);
                        tx = tx.insert(p);
                    }
                    put += 1;
                }
                Err(e) => errors.push(tr(&e)),
            }
        }
        let r = self.doc.commit(tx);
        if put > 0 {
            self.selection.clear();
            self.selection.extend(new_ids, false);
        }
        let mut msg = tr("Lintels put over {} openings ({}). Ctrl+Z undoes it.")
            .replacen("{}", &put.to_string(), 1)
            .replacen("{}", &tr(family.label()), 1);
        if !errors.is_empty() {
            msg = format!("{msg} {}", errors.join(" "));
        }
        self.report(r, msg);
    }

    /// An RC column in the wall at `p`.
    fn put_wall_column(&mut self, level: ElementId, p: DVec2) -> bool {
        let model = self.doc.model();
        let Some(wall) = wall_at(model, level, p) else {
            self.status = tr("No wall there: click on a wall.");
            return false;
        };
        let Some(c) = masonry::wall_column(model, wall, p) else {
            return false;
        };
        let id = c.id;
        let r = self
            .doc
            .commit(Transaction::new("RC column in wall").insert(c));
        self.selection.only(id);
        self.report(
            r,
            tr("RC column put in the wall: as deep as its core, it runs to the next level. Click the next one, Esc leaves."),
        );
        true
    }

    /// The clicks a lintel or column pick waits for. Called every frame
    /// while one is on.
    pub(super) fn masonry_steps(&mut self, ctx: &egui::Context) {
        let Some(pick) = self.masonry_pick else {
            return;
        };
        let picked = self.plan.point_picked.take();
        let left = !self.plan.point_pick && picked.is_none();
        if left || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.plan.point_pick = false;
            self.masonry_pick = None;
            self.status = tr("Masonry pick left.");
            return;
        }
        self.plan.point_pick = true;
        let Some(p) = picked else { return };
        let Some((_, level)) = self.active_plan() else {
            return;
        };
        match pick {
            MasonryPick::Lintel => match opening_at(self.doc.model(), level, p) {
                Some(o) => self.put_lintels(&[o]),
                None => self.status = tr("No door or window there: click on one."),
            },
            MasonryPick::Column => {
                self.put_wall_column(level, p);
            }
        }
    }

    /// The masonry walls that break the rules, selected, and a message.
    fn check_masonry(&mut self) {
        let level = self.active_plan().map(|(_, l)| l);
        let issues = masonry::check_walls(self.doc.model(), level);
        if issues.is_empty() {
            self.status =
                tr("Every masonry wall is braced often enough and no partition stands too high.");
            return;
        }
        let mut ids: Vec<ElementId> = issues.iter().flat_map(|i| i.walls.clone()).collect();
        ids.sort();
        ids.dedup();
        self.selection.clear();
        self.selection.extend(ids, false);
        let lines: Vec<String> = issues.iter().map(issue_text).collect();
        self.status = format!(
            "{} {}",
            tr("Masonry walls to brace with a cross wall or an RC column (selected):"),
            lines.join("; ")
        );
    }

    /// RC columns into every load-bearing wall of the open plan's storey
    /// that runs too far, evenly, off its doors and windows.
    fn fill_wall_columns(&mut self) {
        let level = self.active_plan().map(|(_, l)| l);
        let model = self.doc.model();
        let mut tx = Transaction::new("RC columns in long walls");
        let mut ids = vec![];
        for issue in masonry::check_walls(model, level) {
            for c in masonry::columns_for(model, &issue) {
                ids.push(c.id);
                tx = tx.insert(c);
            }
        }
        if ids.is_empty() {
            self.status = tr("No wall runs too far between braces: no columns to add.");
            return;
        }
        let n = ids.len();
        let r = self.doc.commit(tx);
        self.selection.clear();
        self.selection.extend(ids, false);
        self.report(
            r,
            tr("{} RC columns put into the walls that ran too far (selected). Ctrl+Z undoes it.")
                .replace("{}", &n.to_string()),
        );
    }

    /// The Masonry window: the lintels to put and the rules walls are
    /// checked against.
    pub(super) fn masonry_window(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.masonry.clone() else {
            return;
        };
        let mut open = true;
        let mut act: Option<&'static str> = None;
        let mut rules = masonry::rules(self.doc.model());
        let was_rules = rules.clone();
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(tr("Masonry: lintels and wall columns"))
            .id(egui::Id::new("masonry.window"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(220.0, 260.0))
            .default_size(Vec2::new(440.0, 520.0))
            .constrain_to(area)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.strong(tr("System lintels"));
                    ui.label(
                        egui::RichText::new(tr("Put by hand over the doors and windows you pick; the masonry plan shows them, sections show them cut. They follow their opening."))
                            .weak(),
                    );
                    egui::Grid::new("masonry.lintels")
                        .num_columns(2)
                        .spacing([12.0, 4.0])
                        .show(ui, |ui| {
                            ui.label(tr("Lintel"));
                            egui::ComboBox::from_id_salt("masonry.family")
                                .selected_text(tr(d.family.label()))
                                .width(240.0)
                                .show_ui(ui, |ui| {
                                    for f in Family::ALL {
                                        if ui
                                            .selectable_value(&mut d.family, f, tr(f.label()))
                                            .changed()
                                        {
                                            d.width = None;
                                        }
                                    }
                                });
                            ui.end_row();
                            ui.label(tr("Width"));
                            let widths = masonry::widths(d.family);
                            let shown = d.width.map_or_else(
                                || tr("As the wall takes"),
                                |w| format!("{:.0} mm", w * 1000.0),
                            );
                            egui::ComboBox::from_id_salt("masonry.width")
                                .selected_text(shown)
                                .width(240.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut d.width, None, tr("As the wall takes"));
                                    for w in widths {
                                        ui.selectable_value(
                                            &mut d.width,
                                            Some(w),
                                            format!("{:.0} mm", w * 1000.0),
                                        );
                                    }
                                });
                            ui.end_row();
                            ui.label(tr("Pieces across the wall"));
                            ui.horizontal(|ui| {
                                let mut auto = d.count.is_none();
                                if ui.checkbox(&mut auto, tr("As the wall takes")).changed() {
                                    d.count = if auto { None } else { Some(2) };
                                }
                                if let Some(c) = &mut d.count {
                                    ui.add(egui::DragValue::new(c).range(1..=6));
                                }
                            });
                            ui.end_row();
                        });
                    ui.label(
                        egui::RichText::new(tr("PORFIX: at least two load-bearing lintels on the inner side; 250 wall 2× NP 125, 300 and 375 wall 3× NP 100, 500 wall 4× NP 100, insulation in the rest. Porotherm KP 7: as many as fit across. Have a lintel made up of several pieces checked by the structural engineer."))
                            .weak()
                            .small(),
                    );
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(tr("Over the selected openings")).clicked() {
                            act = Some("lintel");
                        }
                        if ui.button(tr("Click an opening…")).clicked() {
                            act = Some("pick-lintel");
                        }
                        if ui.button(tr("Masonry plan")).clicked() {
                            act = Some("plan");
                        }
                    });
                    ui.separator();
                    ui.strong(tr("Wall lengths between braces"));
                    egui::Grid::new("masonry.rules")
                        .num_columns(2)
                        .spacing([12.0, 4.0])
                        .show(ui, |ui| {
                            ui.label(tr("Load-bearing wall at most"));
                            ui.add(
                                egui::DragValue::new(&mut rules.bearing_max)
                                    .range(2.0..=20.0)
                                    .speed(0.1)
                                    .suffix(" m"),
                            );
                            ui.end_row();
                            ui.label(tr("Load-bearing from a core of"));
                            ui.add(
                                egui::DragValue::new(&mut rules.bearing_from)
                                    .range(0.05..=1.0)
                                    .speed(0.005)
                                    .suffix(" m"),
                            );
                            ui.end_row();
                            ui.label(tr("Seismic area"));
                            ui.checkbox(&mut rules.seismic, tr("RC columns at most 5 m apart (STN EN 1998-1)"));
                            ui.end_row();
                            ui.label(tr("RC column width"));
                            ui.add(
                                egui::DragValue::new(&mut rules.column_width)
                                    .range(0.15..=1.0)
                                    .speed(0.005)
                                    .suffix(" m"),
                            );
                            ui.end_row();
                        });
                    ui.label(
                        egui::RichText::new(tr("PORFIX: stiffening walls 4 to 6 m apart in smaller buildings, 6 to 8 m in larger ones; partitions by PORFIX's table of free-standing walls (thickness, top propped or free). A wall across braces when it is at least 0.3 times as thick (85 mm) and a fifth of the height long (STN EN 1996-1-1). Each wall can be set load-bearing or partition in Properties."))
                            .weak()
                            .small(),
                    );
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(tr("Check the walls")).clicked() {
                            act = Some("check");
                        }
                        if ui.button(tr("RC column in a wall…")).clicked() {
                            act = Some("column");
                        }
                        if ui.button(tr("RC columns into the long walls")).clicked() {
                            act = Some("fill");
                        }
                    });
                    // the open storey's faults, listed
                    let level = self.active_plan().map(|(_, l)| l);
                    let issues = masonry::check_walls(self.doc.model(), level);
                    if issues.is_empty() {
                        ui.label(tr("No wall of this storey breaks the rules."));
                    } else {
                        for i in issues.iter().take(30) {
                            ui.colored_label(egui::Color32::from_rgb(200, 40, 40), issue_text(i));
                        }
                    }
                });
            });
        if rules != was_rules {
            self.set_masonry_rules(rules);
        }
        if !open {
            self.masonry = None;
            return;
        }
        self.masonry = Some(d);
        match act {
            Some("lintel") => self.lintels_over_selection_or_pick(),
            Some("pick-lintel") => self.start_masonry_pick(MasonryPick::Lintel),
            Some("plan") => self.open_masonry_plan(),
            Some("check") => self.check_masonry(),
            Some("column") => self.start_masonry_pick(MasonryPick::Column),
            Some("fill") => self.fill_wall_columns(),
            _ => {}
        }
    }

    fn set_masonry_rules(&mut self, rules: masonry::MasonryRules) {
        let model = self.doc.model();
        let r = match model.project_info() {
            Some((id, _)) => self.doc.edit_merged(id, "Masonry rules", |e| {
                if let ElementKind::Project(p) = &mut e.kind {
                    p.masonry = rules;
                }
            }),
            None => self
                .doc
                .commit(Transaction::new("Masonry rules").insert(Element::new(
                    "Project",
                    ElementKind::Project(creabim_core::ProjectInfo {
                        masonry: rules,
                        ..Default::default()
                    }),
                ))),
        };
        if let Err(e) = r {
            self.status = format!("Couldn't do that: {e}");
        }
    }
}

/// A fault in words, translated, numbers kept.
fn issue_text(i: &masonry::WallIssue) -> String {
    let what = tr(if i.bearing {
        "Load-bearing wall"
    } else {
        "Partition"
    });
    let n = |v: f64| format!("{v:.2}").replace('.', ",");
    match i.fault {
        masonry::WallFault::TooLong { length, limit } => format!(
            "{what}: {} m {} {} m",
            n(length),
            tr("between braces, at most"),
            n(limit)
        ),
        masonry::WallFault::TooHigh { height, limit } => format!(
            "{what}: {} {} m, {} {} m",
            tr("height"),
            n(height),
            tr("at most"),
            n(limit)
        ),
    }
}
