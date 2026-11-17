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
    use glam::DVec3;

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
