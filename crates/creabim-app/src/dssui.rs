//! The data standard for buildings (DSS) in the app: the class of the
//! selected element and its properties in Properties, where another class
//! can be picked and the values the model doesn't know typed in; and the
//! check of the whole model (Project › Dátový štandard), which lists what
//! has no class and what lacks a value the project's stage asks for.
//! What is typed in is project data ([`creabim_core::dss::DssData`]);
//! every change is one transaction.

use super::*;
use creabim_core::dss::{self, DssData, Group, Source};

/// The check window's state, for the session.
#[derive(Default)]
pub(super) struct DssUi {
    pub open: bool,
    /// Show every property of the class, not only the required ones and
    /// those with a value.
    pub all: bool,
    /// The last check, worked out when the window opens or on Check again.
    check: Option<dss::Check>,
    /// The measures and marks of the model, for the revision they were
    /// worked out for: Properties shows them every frame.
    cx: Option<(u64, std::rc::Rc<dss::Ctx>)>,
}

/// The classes that fit an element of this kind: rooms take spaces,
/// everything else components.
fn group_for(kind: &ElementKind) -> Group {
    match kind {
        ElementKind::Room(_) => Group::Space,
        ElementKind::Circuit(_) => Group::System,
        _ => Group::Component,
    }
}

impl CreaBimApp {
    fn dss_ctx(&mut self) -> std::rc::Rc<dss::Ctx> {
        let rev = self.doc.model().revision();
        match &self.dss.cx {
            Some((r, cx)) if *r == rev => cx.clone(),
            _ => {
                let cx = std::rc::Rc::new(dss::Ctx::new(self.doc.model()));
                self.dss.cx = Some((rev, cx.clone()));
                cx
            }
        }
    }

    fn dss_commit(&mut self, data: DssData, label: &str) {
        let tx = dss::write(self.doc.model(), data, label);
        if tx.ops.is_empty() {
            return;
        }
        if let Err(e) = self.doc.commit(tx) {
            self.status = e.to_string();
        }
    }

    /// The selected element's class and properties, in Properties.
    pub(super) fn dss_section(&mut self, ui: &mut egui::Ui, id: ElementId) {
        let classifiable = self.doc.model().get(id).is_some_and(dss::is_classifiable);
        if !classifiable {
            return;
        }
        if !self.prop_header(
            ui,
            "dss",
            "Dátový štandard (DSS)",
            true,
            Some("Trieda prvku podľa dátového štandardu stavieb a vlastnosti, ktoré pre ňu štandard predpisuje. Ide s prvkom do IFC (klasifikácia DSS a sada vlastností DSS)."),
        ) {
            return;
        }
        let cx = self.dss_ctx();
        let model = self.doc.model();
        let Some(e) = model.get(id) else { return };
        let auto = dss::auto_class(model, e);
        let picked = cx.data.of(id).and_then(|x| x.class.clone());
        let current = dss::class_in(model, &cx.data, e);
        let group = current.map_or(group_for(&e.kind), |c| c.group);
        let kind = e.kind.clone();
        // the class
        let mut choice: Option<Option<String>> = None;
        let shown = match (&picked, current) {
            (None, Some(c)) => format!("{} (automaticky)", c.title()),
            (None, None) => "bez triedy".to_string(),
            (Some(code), _) if code.is_empty() => "bez triedy".to_string(),
            (Some(_), Some(c)) => c.title(),
            (Some(code), None) => code.clone(),
        };
        ui.horizontal(|ui| {
            ui.label("Trieda");
            egui::ComboBox::from_id_salt(("dss.class", id))
                .selected_text(shown)
                .width(ui.available_width().max(120.0))
                .show_ui(ui, |ui| {
                    let auto_text = auto.map_or("bez triedy".to_string(), |c| c.title());
                    if ui
                        .selectable_label(picked.is_none(), format!("Automaticky: {auto_text}"))
                        .clicked()
                    {
                        choice = Some(None);
                    }
                    if ui
                        .selectable_label(picked.as_deref() == Some(""), "Bez triedy")
                        .clicked()
                    {
                        choice = Some(Some(String::new()));
                    }
                    ui.separator();
                    let fit = group_for(&kind);
                    for c in dss::classes()
                        .iter()
                        .filter(|c| c.group == fit || c.group == group)
                    {
                        let on = picked.as_deref() == Some(c.code);
                        if ui.selectable_label(on, c.title()).clicked() {
                            choice = Some(Some(c.code.to_string()));
                        }
                    }
                });
        });
        if let Some(choice) = choice {
            let mut data = cx.data.clone();
            data.of_mut(id).class = choice;
            self.dss_commit(data, "Trieda DSS");
            return;
        }
        let Some((class, props)) = dss::filled(self.doc.model(), &cx, e) else {
            return;
        };
        let (entity, predefined) = class.ifc_entity();
        if !entity.is_empty() {
            ui.label(
                egui::RichText::new(format!(
                    "{} · IFC: {}{}",
                    class.group.label(),
                    entity,
                    predefined.map_or(String::new(), |p| format!(" / {p}"))
                ))
                .weak(),
            );
        }
        let phases: Vec<&str> = cx.phases.iter().map(|p| p.label()).collect();
        let missing = props
            .iter()
            .filter(|f| f.required && f.value.is_none())
            .count();
        ui.label(
            egui::RichText::new(format!(
                "Fáza podľa stupňa projektu: {}{}",
                phases.join(", "),
                if missing > 0 {
                    format!(" · chýba {missing} povinných hodnôt")
                } else {
                    String::new()
                }
            ))
            .weak(),
        );
        ui.checkbox(&mut self.dss.all, "Ukázať všetky vlastnosti triedy");
        let all = self.dss.all;
        let mut edit: Option<(String, String)> = None;
        egui::Grid::new(("dss.props", id))
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                for f in &props {
                    if !all && !f.required && f.value.is_none() {
                        continue;
                    }
                    let mut name = egui::RichText::new(f.prop.name);
                    if f.required {
                        name = name.strong();
                    }
                    let unit = f.prop.unit();
                    ui.label(name).on_hover_text(format!(
                        "{}{}{}",
                        f.prop.kind.label(),
                        if unit.is_empty() { "" } else { ", " },
                        unit
                    ));
                    let typed = matches!(f.value, Some((_, Source::Typed)));
                    let hint = match &f.value {
                        Some((v, Source::Model)) => format!("{} (z modelu)", v.shown(f.prop)),
                        Some((v, Source::Calc)) => format!("{} (z výpočtu)", v.shown(f.prop)),
                        _ if f.required => "chýba".to_string(),
                        _ => String::new(),
                    };
                    let key = egui::Id::new(("dss.value", id, f.prop.index));
                    let start = if typed {
                        cx.data
                            .of(id)
                            .and_then(|x| x.value(f.prop.name))
                            .unwrap_or("")
                            .to_string()
                    } else {
                        String::new()
                    };
                    let mut text = ui
                        .data_mut(|d| d.get_temp::<String>(key))
                        .unwrap_or_else(|| start.clone());
                    let r = ui.add(
                        egui::TextEdit::singleline(&mut text)
                            .hint_text(hint)
                            .desired_width(150.0),
                    );
                    if r.changed() {
                        ui.data_mut(|d| d.insert_temp(key, text.clone()));
                    }
                    if r.lost_focus() {
                        ui.data_mut(|d| d.remove::<String>(key));
                        if text.trim() != start.trim() {
                            edit = Some((f.prop.name.to_string(), text.clone()));
                        }
                    }
                    ui.end_row();
                }
            });
        if let Some((name, value)) = edit {
            let mut data = cx.data.clone();
            data.of_mut(id).set_value(&name, &value);
            self.dss_commit(data, "Hodnota DSS");
        }
    }

    /// Project › Dátový štandard: what has no class, what lacks a
    /// required value; the building's and the systems' classes and values.
    pub(super) fn dss_window(&mut self, ctx: &egui::Context) {
        if !self.dss.open {
            self.dss.check = None;
            return;
        }
        if self.dss.check.is_none() {
            self.dss.check = Some(dss::check(self.doc.model()));
        }
        let mut open = true;
        let mut select: Option<ElementId> = None;
        let mut recheck = false;
        let mut new_data: Option<(DssData, &'static str)> = None;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        let model = self.doc.model();
        let data = dss::data(model);
        let phases: Vec<&str> = dss::project_phases(model)
            .iter()
            .map(|p| p.label())
            .collect();
        let check = self.dss.check.clone().unwrap_or_default();
        crate::app::shell::window("Dátový štandard stavieb (DSS)".to_string())
            .open(&mut open)
            .default_pos(area.min + Vec2::new(60.0, 80.0))
            .default_width(460.0)
            .constrain_to(area)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "Prvky sú zaradené do tried dátového štandardu stavieb (comBIMe, preložené do slovenčiny). \
                         Povinné vlastnosti sa kontrolujú podľa stupňa projektu (Projekt › Údaje o projekte): {}.",
                        phases.join(", ")
                    ))
                    .weak(),
                );
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(format!("Zaradených prvkov: {}", check.classified));
                    ui.label(format!("bez triedy: {}", check.unclassified.len()));
                    ui.label(format!("s chýbajúcimi hodnotami: {}", check.missing.len()));
                    if ui.button("Skontrolovať znova").clicked() {
                        recheck = true;
                    }
                });
                egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                    // the building
                    egui::CollapsingHeader::new("Budova").default_open(false).show(ui, |ui| {
                        let mut d = data.clone();
                        if let Some((class, props)) = dss::filled_building(model, &d) {
                            let mut code = d.building.class.clone().unwrap_or_default();
                            egui::ComboBox::from_id_salt("dss.building")
                                .selected_text(class.title())
                                .show_ui(ui, |ui| {
                                    for c in dss::classes().iter().filter(|c| c.group == Group::Entity) {
                                        ui.selectable_value(&mut code, c.code.to_string(), c.title());
                                    }
                                });
                            if !code.is_empty() && Some(code.as_str()) != d.building.class.as_deref() {
                                d.building.class = Some(code);
                                new_data = Some((d.clone(), "Trieda DSS budovy"));
                            }
                            if let Some(v) = values_grid(ui, "dss.building.values", &props, &d.building) {
                                d.building.set_value(&v.0, &v.1);
                                new_data = Some((d, "Hodnota DSS budovy"));
                            }
                        }
                    });
                    // the pipe systems in the model
                    let mut codes: Vec<String> = model
                        .iter()
                        .filter_map(|e| match &e.kind {
                            ElementKind::Pipe(p) => Some(p.system.clone()),
                            _ => None,
                        })
                        .collect();
                    codes.sort();
                    codes.dedup();
                    if !codes.is_empty() {
                        egui::CollapsingHeader::new("Systémy rozvodov").default_open(false).show(ui, |ui| {
                            for code in &codes {
                                let mut d = data.clone();
                                let title = dss::system_class_in(&d, code)
                                    .map_or("bez triedy".to_string(), |c| c.title());
                                egui::CollapsingHeader::new(format!("{code} – {}: {title}", dss::system_name(code)))
                                    .id_salt(("dss.sys", code.as_str()))
                                    .show(ui, |ui| {
                                        let mut pick = d.system(code).and_then(|x| x.class.clone()).unwrap_or_default();
                                        egui::ComboBox::from_id_salt(("dss.sys.class", code.as_str()))
                                            .selected_text(title.clone())
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(&mut pick, String::new(), "Automaticky");
                                                for c in dss::classes().iter().filter(|c| c.group == Group::System) {
                                                    ui.selectable_value(&mut pick, c.code.to_string(), c.title());
                                                }
                                            });
                                        let now = d.system(code).and_then(|x| x.class.clone()).unwrap_or_default();
                                        if pick != now {
                                            d.system_mut(code).class = (!pick.is_empty()).then_some(pick);
                                            new_data = Some((d.clone(), "Trieda DSS systému"));
                                        }
                                        if let Some((_, props)) = dss::filled_system(model, &d, code) {
                                            let entry = d.system(code).cloned().unwrap_or_default();
                                            if let Some(v) = values_grid(ui, ("dss.sys.values", code.as_str()), &props, &entry) {
                                                d.system_mut(code).set_value(&v.0, &v.1);
                                                new_data = Some((d, "Hodnota DSS systému"));
                                            }
                                        }
                                    });
                            }
                        });
                    }
                    if !check.unclassified.is_empty() {
                        egui::CollapsingHeader::new(format!("Bez triedy ({})", check.unclassified.len()))
                            .default_open(true)
                            .show(ui, |ui| {
                                for id in check.unclassified.iter().take(300) {
                                    let Some(e) = model.get(*id) else { continue };
                                    if ui.link(format!("{} ({})", e.name, kind_label(&e.kind))).clicked() {
                                        select = Some(*id);
                                    }
                                }
                            });
                    }
                    if !check.missing.is_empty() {
                        egui::CollapsingHeader::new(format!("Chýbajúce povinné hodnoty ({})", check.missing.len()))
                            .default_open(true)
                            .show(ui, |ui| {
                                for (id, miss) in check.missing.iter().take(300) {
                                    let Some(e) = model.get(*id) else { continue };
                                    let class = dss::class_in(model, &data, e).map_or(String::new(), |c| c.title());
                                    ui.horizontal_wrapped(|ui| {
                                        if ui.link(format!("{} · {class}", e.name)).clicked() {
                                            select = Some(*id);
                                        }
                                        ui.label(egui::RichText::new(miss.join(", ")).weak());
                                    });
                                }
                            });
                    }
                });
            });
        if let Some((d, label)) = new_data {
            self.dss_commit(d, label);
        }
        if let Some(id) = select {
            self.selection.set(Some(id));
        }
        if recheck {
            self.dss.check = None;
        }
        if !open {
            self.dss.open = false;
            self.dss.check = None;
        }
    }
}

/// What an element is, in a few words.
fn kind_label(k: &ElementKind) -> &'static str {
    match k {
        ElementKind::Wall(w) if w.curtain.is_some() => "závesná stena",
        ElementKind::Wall(_) => "stena",
        ElementKind::Opening(_) => "otvor",
        ElementKind::Room(_) => "miestnosť",
        ElementKind::Object(_) => "objekt",
        ElementKind::Pipe(_) => "potrubie",
        ElementKind::Valve(_) => "armatúra",
        ElementKind::Site(_) => "terén / pozemok",
        ElementKind::Part(_) | ElementKind::FreeMesh(_) => "teleso",
        _ => "prvok",
    }
}

/// A grid of properties with their values: typed in, or shown as a hint
/// when the model knows them. Returns the value typed when a field is
/// left with a change.
fn values_grid(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash + Copy + std::fmt::Debug,
    props: &[dss::Filled],
    entry: &dss::Entry,
) -> Option<(String, String)> {
    let mut out = None;
    egui::Grid::new(salt)
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            for f in props {
                let mut name = egui::RichText::new(f.prop.name);
                if f.required {
                    name = name.strong();
                }
                ui.label(name);
                let start = entry.value(f.prop.name).unwrap_or("").to_string();
                let hint = match &f.value {
                    Some((v, Source::Model | Source::Calc)) => {
                        format!("{} (z modelu)", v.shown(f.prop))
                    }
                    _ if f.required => "chýba".into(),
                    _ => String::new(),
                };
                let key = egui::Id::new((salt, f.prop.index));
                let mut text = ui
                    .data_mut(|d| d.get_temp::<String>(key))
                    .unwrap_or_else(|| start.clone());
                let r = ui.add(
                    egui::TextEdit::singleline(&mut text)
                        .hint_text(hint)
                        .desired_width(150.0),
                );
                if r.changed() {
                    ui.data_mut(|d| d.insert_temp(key, text.clone()));
                }
                if r.lost_focus() {
                    ui.data_mut(|d| d.remove::<String>(key));
                    if text.trim() != start.trim() {
                        out = Some((f.prop.name.to_string(), text.clone()));
                    }
                }
                ui.end_row();
            }
        });
    out
}
