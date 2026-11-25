//! Properties of a column's or beam's wrap: thermal insulation with an
//! exterior render, each side its own thickness (see
//! [`creabim_core::wrap`]).

use super::CreaBimApp;
use crate::i18n::tr;
use creabim_core::wrap::{self, PRESETS};
use creabim_core::{Element, ElementId, ElementKind, Model, Transaction};
use eframe::egui;

/// The names of `e`'s four sides, the exterior one marked when it is in
/// a wall.
fn side_names(model: &Model, e: &Element) -> [String; 4] {
    let (names, in_wall) = match &e.kind {
        ElementKind::Column(c) => (
            ["Front (−Y)", "Right (+X)", "Back (+Y)", "Left (−X)"],
            c.in_wall.is_some(),
        ),
        _ => (
            ["Left", "Right", "Under", "Over"],
            matches!(&e.kind, ElementKind::Beam(b) if b.lintel.is_some())
                || e.name == creabim_core::masonry::RING_BEAM,
        ),
    };
    let ext = in_wall.then(|| wrap::exterior_side(model, e)).flatten();
    let mut out = names.map(tr);
    if let Some(i) = ext {
        out[i] = format!("{} ({})", out[i], tr("exterior"));
        let inner = match &e.kind {
            ElementKind::Column(_) => [2, 3, 0, 1][i],
            _ => [1, 0, 3, 2][i],
        };
        out[inner] = format!("{} ({})", out[inner], tr("interior"));
    }
    out
}

/// The project's materials that fit (`render`: renders and plasters,
/// else insulation), and the presets it has not got yet.
fn choices(model: &Model, render: bool) -> (Vec<(ElementId, String)>, Vec<&'static str>) {
    let fits = |name: &str, cat: &str| {
        let n = name.to_lowercase();
        if render {
            cat.contains("plaster")
                || n.contains("render")
                || n.contains("plaster")
                || n.contains("omietk")
                || n.contains("omítk")
        } else {
            cat == "insulation"
        }
    };
    let have: Vec<(ElementId, String)> = creabim_core::library::materials(model)
        .into_iter()
        .filter(|(_, n, m)| fits(n, &m.category))
        .map(|(id, n, _)| (id, n.to_string()))
        .collect();
    let presets = PRESETS
        .iter()
        .filter(|p| p.3 == render && !have.iter().any(|h| h.1 == p.0))
        .map(|p| p.0)
        .collect();
    (have, presets)
}

/// The XPS a ring beam gets on its exterior side when its insulation is
/// turned on, m: 50 mm, as with the Porotherm VT 8 ring-beam block
/// (8 cm block + 50 mm insulation); set per beam.
const RING_XPS: f64 = 0.05;

impl CreaBimApp {
    /// The wrap rows of column or beam `e` (being edited as `k`): on or
    /// off, each side's insulation, its material, the render on it, and
    /// U across. A board of a wrap only says whose it is.
    pub(super) fn wrap_fields(
        &mut self,
        ui: &mut egui::Ui,
        e: &Element,
        k: &mut ElementKind,
    ) -> &'static str {
        let mut label = "";
        let piece = match &*k {
            ElementKind::Column(c) => c.piece,
            ElementKind::Beam(b) => b.piece,
            _ => return label,
        };
        if let Some(p) = piece {
            let host = self
                .doc
                .model()
                .get(p.host)
                .map_or(String::new(), |h| h.name.clone());
            crate::props::note(
                ui,
                tr("Part of the insulation and render round {}: set them there.")
                    .replace("{}", &host),
            );
            return label;
        }
        let names = side_names(self.doc.model(), e);
        let ext = wrap::exterior_side(self.doc.model(), e).unwrap_or(0);
        // a ring beam: XPS on its exterior side only, inside the wall's
        // core (George 2026-11-25)
        let ring = e.name == creabim_core::masonry::RING_BEAM
            && matches!(&*k, ElementKind::Beam(b) if b.lintel.is_none());
        // a monolithic RC lintel: XPS outside and under it, each its own
        // thickness, laid into the formwork (George 2026-11-25)
        let cast = matches!(&*k, ElementKind::Beam(b) if b.lintel.as_ref().is_some_and(|l| l.cast.is_some()));
        if cast {
            if let ElementKind::Beam(b) = &mut *k {
                if let Some(c) = b.lintel.as_mut().and_then(|l| l.cast.as_mut()) {
                    let num = |ui: &mut egui::Ui,
                               text: &str,
                               hint: &str,
                               v: &mut f64,
                               lo: f64,
                               hi: f64|
                     -> bool {
                        crate::props::row(ui, text, |ui| {
                            crate::props::fill(ui, crate::units::drag(v).range(lo..=hi))
                                .on_hover_text(tr(hint))
                                .changed()
                        })
                    };
                    if num(ui, "Bearing", "How far it rests on the wall at each end: at least 250 mm (PORFIX, eStav), 400 to 500 mm over 4 to 6 m spans (PORFIX)", &mut c.bearing, 0.1, 1.0) {
                        label = "Lintel bearing";
                    }
                    if num(ui, "Concrete depth", "Without the insulation under it. Usual: a twelfth of the span, at least 250 mm (VUT Brno: průvlak h = L/15 to L/12)", &mut c.depth, 0.1, 1.5) {
                        label = "Lintel depth";
                    }
                }
            }
        }
        let only: Option<Vec<usize>> = if ring {
            Some(vec![ext])
        } else if cast {
            Some(vec![ext, 2])
        } else {
            None
        };
        let plain = match &*k {
            ElementKind::Beam(b) if ring => {
                creabim_core::masonry::ring_plain_width(self.doc.model(), b)
            }
            _ => None,
        };
        let w = match k {
            ElementKind::Column(c) => &mut c.wrap,
            ElementKind::Beam(b) => &mut b.wrap,
            _ => return label,
        };
        let mut on = w.is_some();
        if crate::props::check(
            ui,
            "Wrap",
            &mut on,
            "Thermal insulation and exterior render",
        )
        .changed()
        {
            label = "Insulation round it";
            if on {
                let mut sides = [0.0; 4];
                sides[ext] = if ring || cast { RING_XPS } else { 0.1 };
                let (mut nw, mut mats) = wrap::new_wrap(self.doc.model(), sides);
                if ring || cast {
                    // XPS, no render: the wall's insulation runs on over it
                    nw.plaster = None;
                    mats.clear();
                    if let Some((id, m)) = wrap::preset(self.doc.model(), PRESETS[1].0) {
                        nw.insulation = Some(id);
                        mats.extend(m);
                    }
                }
                if !mats.is_empty() {
                    let mut tx = Transaction::new("Insulation materials");
                    for m in mats {
                        tx = tx.insert(m);
                    }
                    let _ = self.doc.commit(tx);
                }
                *w = Some(nw);
            } else {
                *w = None;
            }
        }
        let Some(wr) = w.as_mut() else {
            // a ring beam back to the core's width less its shuttering
            if ring && !label.is_empty() {
                if let (ElementKind::Beam(b), Some(width)) = (&mut *k, plain) {
                    b.width = width;
                }
            }
            return label;
        };
        let mut take: Option<(&'static str, bool)> = None;
        let pick = |ui: &mut egui::Ui,
                    model: &Model,
                    text: &str,
                    current: &mut Option<ElementId>,
                    render: bool,
                    take: &mut Option<(&'static str, bool)>|
         -> bool {
            let (have, presets) = choices(model, render);
            let name = current
                .and_then(|id| model.get(id))
                .map_or(tr("None"), |e| e.name.clone());
            let mut changed = false;
            crate::app::helpui::combo(ui, text)
                .selected_text(name)
                .show_ui(ui, |ui| {
                    for (id, n) in &have {
                        changed |= ui.selectable_value(current, Some(*id), n).changed();
                    }
                    if !presets.is_empty() {
                        ui.separator();
                        for p in presets {
                            if ui.selectable_label(false, p).clicked() {
                                *take = Some((p, render));
                            }
                        }
                    }
                });
            changed
        };
        if pick(
            ui,
            self.doc.model(),
            "Insulation",
            &mut wr.insulation,
            false,
            &mut take,
        ) {
            label = "Insulation material";
        }
        let num = |ui: &mut egui::Ui, text: &str, v: &mut f64, hi: f64| -> bool {
            crate::props::row(ui, text, |ui| {
                crate::props::fill(ui, crate::units::drag(v).range(0.0..=hi)).changed()
            })
        };
        for (i, n) in names.iter().enumerate() {
            if only.as_ref().is_some_and(|o| !o.contains(&i)) {
                continue;
            }
            if num(ui, n, &mut wr.sides[i], 0.5) {
                label = "Insulation thickness";
            }
        }
        if only.is_some() {
            crate::props::note(
                ui,
                tr("Laid into the formwork as it is cast; the wall's insulation and render run on over it."),
            );
            return label;
        }
        if pick(
            ui,
            self.doc.model(),
            "Exterior render",
            &mut wr.plaster,
            true,
            &mut take,
        ) {
            label = "Render material";
        }
        if num(ui, "Render thickness", &mut wr.plaster_thickness, 0.05) {
            label = "Render thickness";
        }
        if let Some((name, render)) = take {
            if let Some((id, new)) = wrap::preset(self.doc.model(), name) {
                if let Some(m) = new {
                    let _ = self
                        .doc
                        .commit(Transaction::new("Material from the library").insert(m));
                }
                if render {
                    wr.plaster = Some(id);
                } else {
                    wr.insulation = Some(id);
                }
                label = "Insulation material";
            }
        }
        // U across it, from the exterior side in
        let mut now = e.clone();
        now.kind = k.clone();
        if let Some(t) = creabim_core::thermal::of_element(self.doc.model(), &now) {
            let text = match t.u {
                Some(u) => tr("U across it, from the exterior in: {} W/(m²·K)")
                    .replace("{}", &creabim_core::thermal::shown(u)),
                None => tr("U unknown: no λ for {}.").replace(
                    "{}",
                    &t.missing()
                        .iter()
                        .map(|l| l.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
            };
            crate::props::note(ui, text);
        }
        let rect = match &*k {
            ElementKind::Column(c) => {
                matches!(c.profile, creabim_core::Profile::Rectangle { .. }) && c.section.is_none()
            }
            ElementKind::Beam(b) => b.section.is_none() && b.up.is_none() && b.rise == [0.0, 0.0],
            _ => true,
        };
        if !rect {
            crate::props::note(
                ui,
                tr("Only a rectangular column, or a straight level beam, can be wrapped."),
            );
        }
        label
    }
}
