//! Pipes and ducts in Properties: the system, the pipe type (a series from
//! PilonDesigner's catalogue offered for the system's medium), its size,
//! insulation and the heights of its ends.

use creabim_core::mep::{Pipe, SystemKind, SYSTEMS};
use creabim_services::pipe_types::{self, Medium, PipeType};
use eframe::egui;

/// The medium a system's pipes carry: which types it offers.
pub fn medium(kind: SystemKind) -> Medium {
    match kind {
        SystemKind::ColdWater | SystemKind::HotWater | SystemKind::Circulation => Medium::Voda,
        SystemKind::Sanitary | SystemKind::Storm => Medium::Kanal,
        SystemKind::FireWater => Medium::Poziar,
        SystemKind::Gas => Medium::Plyn,
        SystemKind::HeatingSupply | SystemKind::HeatingReturn => Medium::Ukur,
        SystemKind::CoolingSupply | SystemKind::CoolingReturn => Medium::Chlad,
        SystemKind::MedGas => Medium::Medplyn,
        SystemKind::Flue => Medium::Dym,
        SystemKind::AirSupply | SystemKind::AirExhaust => Medium::Vzt,
        SystemKind::Other => Medium::Voda,
    }
}

/// Give `p` the size of `t` nearest `nominal_mm`, its diameters with it.
pub fn set_size(p: &mut Pipe, t: &PipeType, nominal_mm: f64) {
    if let Some(s) = t.size(nominal_mm) {
        p.pipe_type = t.id.clone();
        p.nominal_mm = s.nominal_mm;
        p.outer_mm = s.outer_mm;
        p.inner_mm = s.inner_mm;
        p.height_mm = match t.shape {
            Some(pipe_types::DuctShape::Rect) => Some(p.height_mm.unwrap_or(s.nominal_mm / 2.0)),
            _ => None,
        };
    }
}

/// Change `p`'s type to `t`, keeping its outside diameter as near as the
/// series allows, as a run that changes material.
pub fn set_type(p: &mut Pipe, t: &PipeType) {
    let outer = p.outer_mm;
    if let Some(s) = t.closest_outer(outer) {
        let nominal = s.nominal_mm;
        set_size(p, t, nominal);
    }
}

/// A size's label: "16x2", "DN 15 (21,3)".
pub fn size_label(p: &Pipe) -> String {
    match pipe_types::by_id(&p.pipe_type) {
        Some(t) => t.size_label(p.nominal_mm, p.height_mm),
        None => format!("{:.0}", p.nominal_mm),
    }
}

/// A length field in a row, as the rest of Properties draws them.
fn num(ui: &mut egui::Ui, text: &str, v: &mut f64, lo: f64, hi: f64) -> bool {
    crate::props::row(ui, text, |ui| {
        crate::props::fill(ui, crate::units::drag(v).range(lo..=hi)).changed()
    })
}

/// The pipe's fields; the name of the change, or "" for none.
pub fn pipe_fields(ui: &mut egui::Ui, p: &mut Pipe) -> &'static str {
    let mut label = "";
    let width = ui.available_width().min(220.0);
    let current = p.system_def();
    crate::app::helpui::combo(ui, "System")
        .selected_text(crate::i18n::tw(match current {
            Some(s) => format!("{} – {}", s.code, crate::i18n::tr(s.name)),
            None => p.system.clone(),
        }))
        .width(width)
        .show_ui(ui, |ui| {
            for s in SYSTEMS {
                let text = format!("{} – {}", s.code, crate::i18n::tr(s.name));
                if ui.selectable_label(p.system == s.code, text).clicked() && p.system != s.code {
                    let was = medium(p.kind());
                    p.system = s.code.into();
                    // another medium takes its own pipe
                    if medium(s.kind) != was {
                        if let Some(t) = pipe_types::by_id(s.pipe_type) {
                            set_size(p, t, s.nominal_mm);
                        }
                    }
                    label = "Pipe system";
                }
            }
        });
    let types: Vec<&PipeType> = pipe_types::for_medium(medium(p.kind())).collect();
    let this = pipe_types::by_id(&p.pipe_type);
    crate::app::helpui::combo(ui, "Pipe type")
        .selected_text(crate::i18n::tw(
            this.map_or(p.pipe_type.clone(), |t| t.name.clone()),
        ))
        .width(width)
        .show_ui(ui, |ui| {
            for t in &types {
                let r = ui.selectable_label(p.pipe_type == t.id, crate::i18n::tw(t.name.clone()));
                let r = r.on_hover_text(format!("{}\n{}", t.joint, t.standards));
                if r.clicked() && p.pipe_type != t.id {
                    set_type(p, t);
                    label = "Pipe type";
                }
            }
        });
    if let Some(t) = this {
        crate::app::helpui::combo(ui, "Size")
            .selected_text(size_label(p))
            .width(width)
            .show_ui(ui, |ui| {
                for s in &t.sizes {
                    let text = t.size_label(s.nominal_mm, p.height_mm);
                    if ui
                        .selectable_label((s.nominal_mm - p.nominal_mm).abs() < 1e-6, text)
                        .clicked()
                    {
                        set_size(p, t, s.nominal_mm);
                        label = "Pipe size";
                    }
                }
            });
        if let Some(h) = &mut p.height_mm {
            let mut m = *h / 1000.0;
            if num(ui, "Duct height", &mut m, 0.05, 3.0) {
                *h = (m * 1000.0).round();
                label = "Duct height";
            }
        }
    }
    let mut ins = p.insulation_mm / 1000.0;
    if num(ui, "Insulation", &mut ins, 0.0, 0.2) {
        p.insulation_mm = (ins * 1000.0).round();
        label = "Pipe insulation";
    }
    if num(ui, "Start height", &mut p.start.z, -20.0, 50.0)
        | num(ui, "End height", &mut p.end.z, -20.0, 50.0)
    {
        label = "Pipe height";
    }
    if p.kind().is_gravity() {
        ui.label(
            egui::RichText::new(format!(
                "{}: {:.1} %",
                crate::i18n::tr("Fall"),
                p.fall_pct()
            ))
            .weak(),
        );
    }
    ui.label(
        egui::RichText::new(format!(
            "{}: {:.2} m",
            crate::i18n::tr("Length"),
            p.length()
        ))
        .weak(),
    );
    label
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::ElementId;

    fn pipe() -> Pipe {
        Pipe {
            level: ElementId::new(),
            system: "SV".into(),
            start: DVec3::new(0.0, 0.0, 0.3),
            end: DVec3::new(3.0, 0.0, 0.3),
            pipe_type: "HERZ".into(),
            nominal_mm: 16.0,
            outer_mm: 20.0,
            inner_mm: 16.0,
            height_mm: None,
            insulation_mm: 0.0,
        }
    }

    #[test]
    fn every_system_offers_its_default_type() {
        for s in SYSTEMS {
            let t = pipe_types::by_id(s.pipe_type).expect(s.pipe_type);
            assert!(t.media.contains(&medium(s.kind)), "{}", s.code);
            assert!(t.size(s.nominal_mm).is_some(), "{}", s.code);
        }
    }

    #[test]
    fn a_new_type_keeps_the_outside_diameter() {
        let mut p = pipe();
        set_type(&mut p, pipe_types::by_id("PPR").unwrap());
        assert_eq!((p.pipe_type.as_str(), p.outer_mm), ("PPR", 20.0));
        assert!(p.inner_mm < 20.0);
        set_size(&mut p, pipe_types::by_id("VZH").unwrap(), 400.0);
        assert_eq!(p.height_mm, Some(200.0));
        assert_eq!(size_label(&p), "400×200");
    }
}

/// What the Pipe tool draws next, set in its bar.
#[derive(Clone, Debug)]
pub struct PipeDraft {
    /// The system's code.
    pub system: String,
    pub pipe_type: String,
    pub nominal_mm: f64,
    /// Height of the axis above the floor where a run starts, m.
    pub height: f64,
    /// Where the run's last pipe ended, so the next starts there (a drain
    /// keeps falling); none at a run's start.
    pub last_z: Option<f64>,
}

impl Default for PipeDraft {
    fn default() -> Self {
        Self::for_system(&SYSTEMS[0])
    }
}

impl PipeDraft {
    /// A system's defaults: its pipe type, size and height.
    pub fn for_system(s: &creabim_core::mep::SystemDef) -> Self {
        PipeDraft {
            system: s.code.into(),
            pipe_type: s.pipe_type.into(),
            nominal_mm: s.nominal_mm,
            height: s.height,
            last_z: None,
        }
    }

    pub fn system_def(&self) -> Option<&'static creabim_core::mep::SystemDef> {
        creabim_core::mep::system(&self.system)
    }

    pub fn kind(&self) -> SystemKind {
        self.system_def().map_or(SystemKind::Other, |s| s.kind)
    }

    pub fn pipe_type(&self) -> Option<&'static PipeType> {
        pipe_types::by_id(&self.pipe_type)
    }

    /// The size label shown in the bar: "20x2".
    pub fn size_label(&self) -> String {
        self.pipe_type().map_or_else(
            || format!("{:.0}", self.nominal_mm),
            |t| t.size_label(self.nominal_mm, None),
        )
    }

    /// A pipe from `a` to `b` (plan points) on `level`: level, or for a
    /// drain falling by its system's slope from where the run is.
    pub fn pipe(&self, level: ElementId, a: DVec2, b: DVec2) -> Pipe {
        let z0 = self.last_z.unwrap_or(self.height);
        let fall = self.system_def().map_or(0.0, |s| s.slope_pct) / 100.0;
        let z1 = z0 - fall * a.distance(b);
        let mut p = Pipe {
            level,
            system: self.system.clone(),
            start: a.extend(z0),
            end: b.extend(z1),
            pipe_type: self.pipe_type.clone(),
            nominal_mm: self.nominal_mm,
            outer_mm: self.nominal_mm,
            inner_mm: self.nominal_mm,
            height_mm: None,
            insulation_mm: 0.0,
        };
        if let Some(t) = self.pipe_type() {
            set_size(&mut p, t, self.nominal_mm);
        }
        p
    }
}

use creabim_core::{Document, Element, ElementId, ElementKind, ModelError, Transaction};
use glam::{DVec2, DVec3};

/// A click with the Pipe tool on `level`: the first starts a run, each
/// next one draws a pipe to it and goes on from there.
pub fn click(
    doc: &mut Document,
    level: ElementId,
    p: DVec2,
    state: &mut crate::planview::PlanState,
    selection: &mut crate::selection::Selection,
) -> Result<(), ModelError> {
    let draft = &mut state.members.pipe;
    match state.pending {
        None => {
            draft.last_z = None;
            state.pending = Some(p);
            state.status = Some(crate::i18n::tr(
                "Click where the pipe goes; keep clicking to go on. Esc or right-click ends.",
            ));
            Ok(())
        }
        Some(a) if a.distance(p) > 0.01 => {
            let pipe = draft.pipe(level, a, p);
            draft.last_z = Some(pipe.end.z);
            let name = format!("{} {}", pipe.system, size_label(&pipe));
            let e = Element::new(name, ElementKind::Pipe(pipe));
            selection.only(e.id);
            state.pending = Some(p);
            doc.commit(Transaction::new("Draw pipe").insert(e))
        }
        Some(_) => Ok(()),
    }
}

/// The pipe the tool would draw from `a` to `b`, outlined while the next
/// point is chosen.
pub fn preview(
    painter: &egui::Painter,
    project: &dyn Fn(DVec3) -> Option<egui::Pos2>,
    draft: &PipeDraft,
    a: DVec3,
    b: DVec3,
) {
    if a.distance(b) < 1e-3 {
        return;
    }
    let p = draft.pipe(ElementId::new(), a.truncate(), b.truncate());
    let half = p.section_m().0.max(0.01) / 2.0;
    let n = (b - a).truncate().normalize_or_zero().perp() * half;
    let pts = [
        a.truncate() - n,
        b.truncate() - n,
        b.truncate() + n,
        a.truncate() + n,
        a.truncate() - n,
    ];
    let c = p.system_def().map_or(0x8c8c8c, |s| s.color);
    let color = egui::Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8);
    let scr: Option<Vec<egui::Pos2>> = pts.iter().map(|q| project(q.extend(a.z))).collect();
    if let Some(scr) = scr {
        painter.add(egui::Shape::line(scr, egui::Stroke::new(1.5, color)));
    }
}

/// The Pipe tool's bar: its system, type, size and height.
pub fn tool_options(draft: &PipeDraft) -> Vec<serde_json::Value> {
    use serde_json::json;
    let sys = draft.system_def().map_or(draft.system.clone(), |s| {
        format!("{} – {}", s.code, crate::i18n::tr(s.name))
    });
    vec![
        json!({
            "kind": "menu", "key": "pipe.system", "label": "System", "value": sys, "icon": "pipe",
            "hint": "The system new pipes are in: cold and hot water, drainage, gas, heating, medical gases, ventilation",
        }),
        json!({
            "kind": "menu", "key": "pipe.type", "label": "Pipe type",
            "value": draft.pipe_type().map_or(draft.pipe_type.clone(), |t| t.name.clone()),
            "hint": "The pipe series, from PilonDesigner's catalogue: those for the system's medium",
        }),
        json!({
            "kind": "menu", "key": "pipe.size", "label": "Size", "value": draft.size_label(),
            "hint": "Size of new pipes",
        }),
        json!({
            "kind": "number", "key": "member.pipe_height", "label": "Height", "value": draft.height,
            "unit": "m", "step": 0.05, "min": -5.0, "max": 20.0,
            "hint": "Height of the pipe's axis above the floor where a run starts",
        }),
    ]
}

/// The items of the Pipe tool's menus, as (action, text, checked).
pub fn menu_items(draft: &PipeDraft, key: &str) -> Option<Vec<(String, String, bool)>> {
    Some(match key {
        "pipe.system" => SYSTEMS
            .iter()
            .map(|s| {
                (
                    format!("pipe.system:{}", s.code),
                    format!("{} – {}", s.code, crate::i18n::tr(s.name)),
                    s.code == draft.system,
                )
            })
            .collect(),
        "pipe.type" => pipe_types::for_medium(medium(draft.kind()))
            .map(|t| {
                (
                    format!("pipe.type:{}", t.id),
                    t.name.clone(),
                    t.id == draft.pipe_type,
                )
            })
            .collect(),
        "pipe.size" => draft
            .pipe_type()
            .map(|t| {
                t.sizes
                    .iter()
                    .map(|s| {
                        (
                            format!("pipe.size:{}", s.nominal_mm),
                            t.size_label(s.nominal_mm, None),
                            (s.nominal_mm - draft.nominal_mm).abs() < 1e-6,
                        )
                    })
                    .collect()
            })
            .unwrap_or_default(),
        _ => return None,
    })
}

/// A pick from the Pipe tool's menus, or `pipe:<system>` from a tab: the
/// Pipe tool in that system. False for another action.
pub fn action(draft: &mut PipeDraft, head: &str, arg: &str) -> bool {
    match head {
        "pipe" | "pipe.system" => {
            if let Some(s) = creabim_core::mep::system(arg) {
                let height = draft.height;
                let keep_height = head == "pipe.system" && s.kind == draft.kind();
                *draft = PipeDraft::for_system(s);
                if keep_height {
                    draft.height = height;
                }
            }
        }
        "pipe.type" => {
            if let Some(t) = pipe_types::by_id(arg) {
                let outer = draft
                    .pipe_type()
                    .and_then(|o| o.size(draft.nominal_mm))
                    .map_or(draft.nominal_mm, |s| s.outer_mm);
                draft.pipe_type = t.id.clone();
                if let Some(s) = t.closest_outer(outer) {
                    draft.nominal_mm = s.nominal_mm;
                }
            }
        }
        "pipe.size" => {
            if let Ok(v) = arg.parse::<f64>() {
                draft.nominal_mm = v;
            }
        }
        _ => return false,
    }
    draft.last_z = None;
    true
}

#[cfg(test)]
mod tool_tests {
    use super::*;

    #[test]
    fn a_drain_falls_along_its_run_and_goes_on_from_its_end() {
        let mut d = PipeDraft::default();
        assert!(action(&mut d, "pipe", "K"));
        assert_eq!((d.pipe_type.as_str(), d.nominal_mm), ("PPHT", 100.0));
        let l = ElementId::new();
        let p = d.pipe(l, DVec2::ZERO, DVec2::new(5.0, 0.0));
        assert!((p.start.z + 0.25).abs() < 1e-9 && (p.end.z + 0.35).abs() < 1e-9);
        assert_eq!(p.outer_mm, 110.0);
        d.last_z = Some(p.end.z);
        let q = d.pipe(l, DVec2::new(5.0, 0.0), DVec2::new(5.0, 2.0));
        assert!((q.start.z - p.end.z).abs() < 1e-12);
    }

    #[test]
    fn the_bar_offers_the_systems_types_and_sizes() {
        let mut d = PipeDraft::default();
        let types = menu_items(&d, "pipe.type").unwrap();
        assert!(types.iter().any(|(a, _, on)| a == "pipe.type:HERZ" && *on));
        assert!(!types.iter().any(|(a, ..)| a == "pipe.type:PPHT"));
        action(&mut d, "pipe.type", "PPR");
        // the outside diameter kept: 20 for HERZ 20x2 → PPR 20
        assert_eq!(d.size_label(), "20x3,4");
        assert!(menu_items(&d, "pipe.size")
            .unwrap()
            .iter()
            .any(|(_, t, on)| t == "20x3,4" && *on));
        assert_eq!(menu_items(&d, "pipe.system").unwrap().len(), SYSTEMS.len());
    }
}
