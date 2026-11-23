//! The building services' disciplines (the tabs beside Architecture): a
//! window per discipline with the standards its calculations follow under
//! the project's codes (STN or ČSN, in Project information) and the tools
//! it is getting, in their order.

use super::CreaBimApp;
use creabim_core::norms::{Discipline, NormSet};
use creabim_core::objects::ObjectCategory;
use eframe::egui::{self, Vec2};

impl CreaBimApp {
    /// The codes the project follows.
    pub(super) fn norm_set(&self) -> NormSet {
        self.doc
            .model()
            .project_info()
            .map(|(_, p)| p.norms)
            .unwrap_or_default()
    }

    /// What the open view's drawing is for, in its settings: the
    /// architect's, a trade's or the coordination of all.
    pub(super) fn view_purpose_row(&mut self, ui: &mut egui::Ui, view: creabim_core::ElementId) {
        use creabim_core::viewmodes::Purpose;
        let Some(creabim_core::ElementKind::View(v)) = self.doc.model().get(view).map(|e| &e.kind)
        else {
            return;
        };
        // a legend of rooms is the architect's (with the finishes) or a
        // trade's (without them, George 2026-11-22)
        if matches!(
            v.kind,
            creabim_core::ViewKind::Schedule {
                category: creabim_core::ScheduleCategory::RoomLegend,
                ..
            }
        ) {
            use creabim_core::viewmodes::Trade;
            let was = v.trade;
            let mut now = was;
            let label = |t: Option<Trade>| {
                crate::i18n::tr(t.map_or(Purpose::Architecture.label(), |t| t.label()))
            };
            crate::props::row(ui, "Drawing for", |ui| {
                egui::ComboBox::from_id_salt("view.purpose")
                    .icon(crate::props::combo_icon)
                    .selected_text(label(now))
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut now, None, label(None));
                        for t in Trade::ALL {
                            ui.selectable_value(&mut now, Some(t), label(Some(t)));
                        }
                    });
            });
            if now != was {
                let r = self.doc.edit(view, "Drawing for", |e| {
                    if let creabim_core::ElementKind::View(x) = &mut e.kind {
                        x.trade = now;
                    }
                });
                self.report(
                    r,
                    format!("{}: {}.", crate::i18n::tr("Drawing for"), label(now)),
                );
            }
            return;
        }
        if matches!(v.kind, creabim_core::ViewKind::Schedule { .. }) {
            return;
        }
        let was = Purpose::of(v);
        let mut now = was;
        crate::props::row(ui, "Drawing for", |ui| {
            egui::ComboBox::from_id_salt("view.purpose")
                .icon(crate::props::combo_icon)
                .selected_text(crate::i18n::tr(now.label()))
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for p in Purpose::all() {
                        ui.selectable_value(&mut now, p, crate::i18n::tr(p.label()));
                    }
                });
        });
        // a plan can be the storey's masonry plan: its lintels and the
        // walls that run too far
        if let creabim_core::ViewKind::Plan { .. } = v.kind {
            let mut on = v.masonry;
            let changed = crate::props::row(ui, "Masonry plan", |ui| {
                ui.checkbox(&mut on, "")
                    .on_hover_text(crate::i18n::tr(
                        "Shows the system lintels and marks the masonry walls that run too far between braces; other plans leave the lintels out",
                    ))
                    .changed()
            });
            if changed {
                let r = self.doc.edit(view, "Masonry plan", |e| {
                    if let creabim_core::ElementKind::View(x) = &mut e.kind {
                        x.masonry = on;
                    }
                });
                self.report(r, crate::i18n::tr("Masonry plan"));
                return;
            }
        }
        if now != was {
            let r = self.doc.edit(view, "Drawing for", |e| {
                if let creabim_core::ElementKind::View(x) = &mut e.kind {
                    now.apply(x);
                }
            });
            self.report(
                r,
                format!(
                    "{}: {}.",
                    crate::i18n::tr("Drawing for"),
                    crate::i18n::tr(now.label())
                ),
            );
        }
    }

    /// Pipes are not drawn in the architect's plan, which leaves them
    /// out: with the open plan one of those, opens the storey's plan of
    /// the trade the Pipe tool draws for, made the first time. True when
    /// the view changed.
    pub(super) fn open_trade_plan(&mut self) -> bool {
        use creabim_core::viewmodes::{DisplayMode, Trade};
        use creabim_core::visibility::Category;
        use creabim_core::{ElementKind, ViewKind};
        let model = self.doc.model();
        let Some((now, level)) = self.active_view.and_then(|id| match &model.get(id)?.kind {
            ElementKind::View(v)
                if DisplayMode::of(v) != DisplayMode::Services && !v.graphics.range.from_above =>
            {
                match v.kind {
                    ViewKind::Plan { level, .. } => Some((v.clone(), level)),
                    _ => None,
                }
            }
            _ => None,
        }) else {
            return false;
        };
        let kind = creabim_core::mep::system(&self.plan.members.pipe.system)
            .map_or(creabim_core::mep::SystemKind::Other, |s| s.kind);
        let Some(trade) = Trade::ALL.into_iter().find(|t| t.shows(kind)) else {
            return false;
        };
        let code = match trade {
            Trade::Plumbing => "ZTI",
            Trade::Heating => "VYK",
            Trade::Ventilation => "VZT",
            Trade::MedGas => "MED",
            Trade::Electrical => "ELI",
        };
        let found = model.iter().find_map(|e| match &e.kind {
            ElementKind::View(v)
                if matches!(v.kind, ViewKind::Plan { level: l, .. } if l == level)
                    && DisplayMode::of(v) == DisplayMode::Services
                    && !v.hidden.hides_category(Category::Pipes)
                    && v.callout.is_none()
                    && v.trade.is_none_or(|t| t == trade) =>
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
                v.scale = 50.0;
                DisplayMode::Services.apply(&mut v);
                v.trade = Some(trade);
                let name = format!("{storey} {code}");
                let e = creabim_core::Element::new(name.clone(), ElementKind::View(v));
                let id = e.id;
                if self
                    .doc
                    .commit(creabim_core::Transaction::new("Trade plan").insert(e))
                    .is_err()
                {
                    return false;
                }
                (id, name)
            }
        };
        self.active_view = Some(id);
        self.status = crate::i18n::tr(
            "Opened the plan {}: the architecture plan leaves pipes out, they are drawn in the trade's plan.",
        )
        .replace("{}", &name);
        true
    }

    /// A pick from the Pipe tool's bar, or `pipe:<system>` from a tab,
    /// which takes up the Pipe tool in that system. False for another.
    pub(super) fn pipe_action(&mut self, head: &str, arg: &str) -> bool {
        if !crate::pipesui::action(&mut self.plan.members.pipe, head, arg) {
            return false;
        }
        if head == "pipe" {
            self.plan.tool = crate::planview::Tool::Pipe;
            self.plan.cancel();
            self.open_trade_plan();
        } else {
            // a run goes on in what was picked from its next point
            self.plan.pending = None;
        }
        let d = &self.plan.members.pipe;
        let name = d
            .system_def()
            .map_or(d.system.clone(), |s| crate::i18n::tr(s.name));
        self.status = format!(
            "{}: {}, {} {}",
            crate::i18n::tr("Pipe"),
            name,
            d.pipe_type()
                .map_or(d.pipe_type.clone(), |t| t.name.clone()),
            d.size_label()
        );
        true
    }

    /// Run a discipline action. False when the action isn't one.
    pub(super) fn discipline_action(&mut self, head: &str, arg: &str) -> bool {
        match head {
            "discipline" => {
                self.discipline = Discipline::from_id(arg);
            }
            // the catalogue opened at one category: a discipline's fixtures
            "catalogue.cat" => {
                let c = match arg {
                    "bathroom" => Some(ObjectCategory::Bathroom),
                    "wc" => Some(ObjectCategory::Toilet),
                    "kitchen" => Some(ObjectCategory::Kitchen),
                    "plumbing" => Some(ObjectCategory::Plumbing),
                    "hvac" => Some(ObjectCategory::Hvac),
                    "medical" => Some(ObjectCategory::Medical),
                    "electrical" => Some(ObjectCategory::Electrical),
                    "lighting" => Some(ObjectCategory::Lighting),
                    "site" => Some(ObjectCategory::Site),
                    _ => None,
                };
                self.open_catalogue_at(c);
            }
            // the open view as one trade draws it
            "viewmode" => {
                let Some(mode) = creabim_core::viewmodes::DisplayMode::from_id(arg) else {
                    return false;
                };
                let view = self.active_view.filter(|v| {
                    matches!(
                        self.doc.model().get(*v).map(|e| &e.kind),
                        Some(creabim_core::ElementKind::View(_))
                    )
                });
                let Some(v) = view else {
                    self.status = crate::i18n::tr(
                        "Open a plan, section or elevation first: the mode is the view's.",
                    );
                    return true;
                };
                let r = self.doc.edit(v, "Display mode", |e| {
                    if let creabim_core::ElementKind::View(x) = &mut e.kind {
                        mode.apply(x);
                    }
                });
                self.report(
                    r,
                    format!(
                        "{}: {}.",
                        crate::i18n::tr("Display mode"),
                        crate::i18n::tr(mode.label())
                    ),
                );
            }
            _ => return false,
        }
        true
    }

    /// The discipline window, while one is open.
    pub(super) fn discipline_window(&mut self, ctx: &egui::Context) {
        let Some(d) = self.discipline else {
            return;
        };
        let set = self.norm_set();
        let mut open = true;
        let mut change_codes = false;
        let area = self.mat.area.unwrap_or_else(|| ctx.content_rect());
        super::shell::window(crate::i18n::tr(d.label()))
            .id(egui::Id::new("discipline.window"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(210.0, 200.0))
            .constrain_to(area)
            .collapsible(false)
            .show(ctx, |ui| {
                ui.set_width(420.0);
                crate::props::section(ui, "Standards");
                crate::props::row(ui, "Codes", |ui| {
                    ui.label(crate::i18n::t(set.label()));
                    if ui
                        .small_button(crate::i18n::t("Change…"))
                        .on_hover_text(crate::i18n::t(
                            "The project's codes are set in Project information",
                        ))
                        .clicked()
                    {
                        change_codes = true;
                    }
                });
                egui::Grid::new("discipline.norms")
                    .num_columns(2)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        for n in d.norms(set) {
                            ui.strong(crate::i18n::t(n.code));
                            ui.label(crate::i18n::t(n.title));
                            ui.end_row();
                        }
                    });
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(crate::i18n::t(
                        "Check each standard's current edition before relying on a result.",
                    ))
                    .weak(),
                );
                crate::props::section(ui, "Coming to this tab");
                for p in d.planned() {
                    ui.label(crate::i18n::tr(&format!("•  {}", crate::i18n::t(p))));
                }
            });
        if change_codes {
            self.project_open = true;
        }
        if !open {
            self.discipline = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use creabim_core::norms::{Discipline, NormSet};

    #[test]
    fn every_discipline_text_is_translated() {
        // the window shows these through the tables; a missing one would
        // read in English in a Slovak window
        let sk: std::collections::HashMap<String, String> =
            serde_json::from_str(include_str!("../i18n/sk.json")).unwrap();
        let mut missing = vec![];
        for d in Discipline::ALL {
            let mut texts = vec![d.label()];
            texts.extend(d.planned());
            for set in NormSet::ALL {
                texts.push(set.label());
                texts.extend(d.norms(set).iter().map(|n| n.title));
            }
            missing.extend(texts.into_iter().filter(|t| !sk.contains_key(*t)));
        }
        missing.sort();
        missing.dedup();
        assert!(missing.is_empty(), "not in sk.json: {missing:?}");
    }
}
