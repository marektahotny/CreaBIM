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

/// "Add valve or accessory" under a pipe's fields: the kinds its medium
/// takes; the one picked.
pub fn valve_add(ui: &mut egui::Ui, p: &Pipe) -> Option<&'static str> {
    let kinds = creabim_core::mep::valves_for(p.kind());
    if kinds.is_empty() {
        return None;
    }
    let mut picked = None;
    let width = ui.available_width().min(220.0);
    crate::app::helpui::combo(ui, "Add valve")
        .selected_text(crate::i18n::tr("Valve or accessory…"))
        .width(width)
        .show_ui(ui, |ui| {
            for k in kinds {
                let r = ui.selectable_label(false, crate::i18n::tr(k.name));
                let r = if k.norm.is_empty() {
                    r
                } else {
                    r.on_hover_text(k.norm)
                };
                if r.clicked() {
                    picked = Some(k.code);
                }
            }
        });
    picked
}

/// Where a new valve goes on the pipe `id`: its middle, or the widest
/// free stretch between the valves already on it.
pub fn valve_place(model: &creabim_core::Model, id: creabim_core::ElementId) -> f64 {
    let mut ts: Vec<f64> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Valve(v) if v.pipe == id => Some(v.t),
            _ => None,
        })
        .collect();
    ts.extend([0.0, 1.0]);
    ts.sort_by(f64::total_cmp);
    ts.windows(2)
        .max_by(|a, b| (a[1] - a[0]).total_cmp(&(b[1] - b[0])))
        .map_or(0.5, |w| (w[0] + w[1]) / 2.0)
}

/// A valve's fields; the name of the change, or "" for none.
pub fn valve_fields(
    ui: &mut egui::Ui,
    model: &creabim_core::Model,
    v: &mut creabim_core::mep::Valve,
) -> &'static str {
    let mut label = "";
    let Some(pipe) = v.host(model) else {
        return label;
    };
    let width = ui.available_width().min(220.0);
    let this = v.kind_def();
    crate::app::helpui::combo(ui, "Kind")
        .selected_text(this.map_or(v.kind.clone(), |k| crate::i18n::tr(k.name)))
        .width(width)
        .show_ui(ui, |ui| {
            for k in creabim_core::mep::valves_for(pipe.kind()) {
                if ui
                    .selectable_label(v.kind == k.code, crate::i18n::tr(k.name))
                    .clicked()
                    && v.kind != k.code
                {
                    v.kind = k.code.into();
                    label = "Valve kind";
                }
            }
        });
    // along the pipe, from its start
    let len = pipe.start.distance(pipe.end);
    let mut at = v.t * len;
    if len > 1e-6 && num(ui, "From the pipe's start", &mut at, 0.0, len) {
        v.t = (at / len).clamp(0.0, 1.0);
        label = "Move valve";
    }
    if v.kind == "vpust" {
        // a roof drain's catchment: 0 takes the roof over it
        let mut a = v.area_m2.unwrap_or(0.0);
        let changed = crate::props::row(ui, "Catchment area", |ui| {
            crate::props::fill(
                ui,
                egui::DragValue::new(&mut a)
                    .range(0.0..=100_000.0)
                    .speed(1.0)
                    .suffix(" m²"),
            )
            .on_hover_text(crate::i18n::tr(
                "0 takes the roof over the drain, shared among the drains on it",
            ))
            .changed()
        });
        if changed {
            v.area_m2 = (a > 0.0).then_some(a);
            label = "Catchment area";
        }
        let mut c = v.coef.unwrap_or(0.0);
        let changed = crate::props::row(ui, "Runoff coefficient", |ui| {
            crate::props::fill(
                ui,
                egui::DragValue::new(&mut c).range(0.0..=1.0).speed(0.05),
            )
            .on_hover_text(crate::i18n::tr(
                "0 takes it from the roof's pitch: 1.0 over 5 %, else 0.8",
            ))
            .changed()
        });
        if changed {
            v.coef = (c > 0.0).then_some(c);
            label = "Runoff coefficient";
        }
    }
    if v.is_air_terminal() {
        let mut q = v.flow_m3h.unwrap_or(0.0);
        let changed = crate::props::row(ui, "Air flow", |ui| {
            crate::props::fill(
                ui,
                egui::DragValue::new(&mut q)
                    .range(0.0..=20_000.0)
                    .speed(5.0)
                    .suffix(" m³/h"),
            )
            .on_hover_text(crate::i18n::tr(
                "0 shares the room's air flow among the room's terminals",
            ))
            .changed()
        });
        if changed {
            v.flow_m3h = (q > 0.0).then_some(q);
            label = "Air flow";
        }
    }
    if v.kind == "hydrant" {
        let now = v.hydrant_def();
        crate::app::helpui::combo(ui, "Hose system")
            .selected_text(now.name)
            .width(width)
            .show_ui(ui, |ui| {
                for h in &creabim_core::mep::HYDRANTS {
                    let text = format!("{} ({} l/s)", h.name, crate::i18n::num(h.q_ls, 1));
                    if ui.selectable_label(now.code == h.code, text).clicked() && now.code != h.code
                    {
                        v.hydrant = Some(h.code.into());
                        label = "Hose system";
                    }
                }
            });
    }
    if let Some(k) = this {
        crate::props::row(ui, "System", |ui| {
            ui.label(format!("{} DN {:.0}", pipe.system, pipe.nominal_mm))
        });
        if let Some(z) = k.zeta {
            crate::props::row(ui, "Loss coefficient ζ", |ui| {
                ui.label(crate::i18n::num(z, 1))
            });
        }
        if !k.norm.is_empty() {
            crate::props::row(ui, "Standard", |ui| ui.label(k.norm));
        }
    }
    label
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
                let r = r.on_hover_text(crate::i18n::tr(&format!("{}\n{}", t.joint, t.standards)));
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
            egui::RichText::new(crate::i18n::tr(&format!(
                "{}: {} %",
                crate::i18n::tr("Fall"),
                crate::i18n::num(p.fall_pct(), 1)
            )))
            .weak(),
        );
    }
    ui.label(
        egui::RichText::new(crate::i18n::tr(&format!(
            "{}: {:.2} m",
            crate::i18n::tr("Length"),
            p.length()
        )))
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
    /// The height was changed while a run is being drawn: the next click
    /// first rises or drops to it where the run is.
    pub height_set: bool,
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
            height_set: false,
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

    /// The height the run is at now, m above the floor.
    pub fn z_now(&self) -> f64 {
        self.last_z.unwrap_or(self.height)
    }

    /// The riser the next click draws first: from where the run is to
    /// the height set since, as (from, to).
    pub fn rise(&self) -> Option<(f64, f64)> {
        let z = self.z_now();
        (self.height_set && (self.height - z).abs() > 1e-3).then_some((z, self.height))
    }

    /// The height the next pipe starts at: the height set for it, else
    /// where the run is.
    pub fn z_next(&self) -> f64 {
        self.rise().map_or(self.z_now(), |r| r.1)
    }

    /// A vertical pipe at `at` from `z0` to `z1`.
    pub fn riser(&self, level: ElementId, at: DVec2, z0: f64, z1: f64) -> Pipe {
        let mut p = self.pipe(level, at, at);
        p.start = at.extend(z0);
        p.end = at.extend(z1);
        p
    }

    /// A pipe from `a` to `b` (plan points) on `level`: level, or for a
    /// drain falling by its system's slope from where the run is.
    pub fn pipe(&self, level: ElementId, a: DVec2, b: DVec2) -> Pipe {
        let z0 = self.z_next();
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

/// How near a click must be to a fixture's connector to join it, m.
const CONNECTOR_REACH: f64 = 0.05;

/// The fixture connector on `level` nearest `p`, within reach.
pub fn connector_at(
    model: &creabim_core::Model,
    level: ElementId,
    p: DVec2,
) -> Option<creabim_core::fixture::Connector> {
    model
        .iter()
        .filter(|e| matches!(&e.kind, ElementKind::Object(o) if o.level == level))
        .flat_map(|e| creabim_core::fixture::connectors(model, e))
        .map(|c| (c.at.truncate().distance(p), c))
        .filter(|(d, _)| *d <= CONNECTOR_REACH)
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, c)| c)
}

/// How near a click must be to a pipe's end or axis to join it, m.
const PIPE_REACH: f64 = 0.12;

/// Where a click joins a pipe already drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct PipeNode {
    /// On the pipe's axis, heights above its level.
    pub at: DVec3,
    pub system: String,
    pub pipe_type: String,
    pub nominal_mm: f64,
}

/// The place on a pipe of `level` a click at `p` joins: an end within
/// reach first, else the nearest point of an axis. Of ends one over the
/// other (a riser's) the one nearest the height `z`. Only pipes of
/// `system`, where one is given.
pub fn pipe_node_at(
    model: &creabim_core::Model,
    level: ElementId,
    p: DVec2,
    z: f64,
    system: Option<&str>,
) -> Option<PipeNode> {
    let mut best: Option<(bool, f64, PipeNode)> = None;
    for e in model.iter() {
        let ElementKind::Pipe(q) = &e.kind else {
            continue;
        };
        if q.level != level || system.is_some_and(|s| s != q.system) {
            continue;
        }
        let node = |at: DVec3| PipeNode {
            at,
            system: q.system.clone(),
            pipe_type: q.pipe_type.clone(),
            nominal_mm: q.nominal_mm,
        };
        let mut offer = |end: bool, d: f64, at: DVec3| {
            if d > PIPE_REACH {
                return;
            }
            // ends before axes; then the nearer in plan, then in height
            let key = (!end, d + (at.z - z).abs() * 1e-3);
            if best.as_ref().is_none_or(|b| (!b.0, b.1) > key) {
                best = Some((end, key.1, node(at)));
            }
        };
        for at in [q.start, q.end] {
            offer(true, at.truncate().distance(p), at);
        }
        let (a, b) = (q.start.truncate(), q.end.truncate());
        let run = b - a;
        let len2 = run.length_squared();
        if len2 < 1e-9 {
            // a riser: the height asked for, within it
            let (lo, hi) = (q.start.z.min(q.end.z), q.start.z.max(q.end.z));
            offer(false, a.distance(p), a.extend(z.clamp(lo, hi)));
            continue;
        }
        let t = ((p - a).dot(run) / len2).clamp(0.0, 1.0);
        let on = q.start + (q.end - q.start) * t;
        offer(false, on.truncate().distance(p), on);
    }
    best.map(|b| b.2)
}

fn insert(tx: Transaction, pipe: Pipe) -> (Transaction, ElementId) {
    let name = format!("{} {}", pipe.system, size_label(&pipe));
    let e = Element::new(name, ElementKind::Pipe(pipe));
    let id = e.id;
    (tx.insert(e), id)
}

/// A click with the Pipe tool on `level`: the first starts a run, each
/// next one draws a pipe to it and goes on from there. A run started at
/// a fixture's connector or on a pipe takes its system and height; one
/// ended at a connector or on a pipe of its system drops or rises to it
/// there, and ends. A height set in the bar while a run is drawn makes
/// the next click rise or drop to it first; a click on the run's own
/// last point draws that riser alone.
pub fn click(
    doc: &mut Document,
    level: ElementId,
    p: DVec2,
    state: &mut crate::planview::PlanState,
    selection: &mut crate::selection::Selection,
) -> Result<(), ModelError> {
    let conn = connector_at(doc.model(), level, p);
    let draft = &mut state.members.pipe;
    match state.pending {
        None => {
            draft.last_z = None;
            draft.height_set = false;
            let mut p = p;
            if let Some(c) = conn {
                if let Some(def) = creabim_core::mep::system(c.kind.code()) {
                    if draft.system != def.code {
                        *draft = PipeDraft::for_system(def);
                    }
                }
                draft.last_z = Some(c.at.z);
                p = c.at.truncate();
            } else if let Some(n) = pipe_node_at(doc.model(), level, p, draft.height, None) {
                // going on from a pipe: as it is, at its height
                if draft.system != n.system {
                    if let Some(def) = creabim_core::mep::system(&n.system) {
                        *draft = PipeDraft::for_system(def);
                    }
                }
                draft.pipe_type = n.pipe_type.clone();
                draft.nominal_mm = n.nominal_mm;
                draft.last_z = Some(n.at.z);
                p = n.at.truncate();
            }
            state.pending = Some(p);
            state.status = Some(format!(
                "Z {} m. {}",
                crate::i18n::num(draft.z_now(), 3),
                crate::i18n::tr(
                    "Click where the pipe goes; change Height in the bar to rise or drop here. Esc or right-click ends."
                )
            ));
            Ok(())
        }
        Some(a) => {
            let mut tx = Transaction::new("Draw pipe");
            let mut last = None;
            // the height was changed: rise or drop to it where the run is
            if let Some((z0, z1)) = draft.rise() {
                let (t, id) = insert(tx, draft.riser(level, a, z0, z1));
                tx = t;
                last = Some(id);
                draft.last_z = Some(z1);
            }
            draft.height_set = false;
            if a.distance(p) <= 0.01 {
                // on the run's own point: the riser alone
                let Some(id) = last else {
                    return Ok(());
                };
                selection.only(id);
                state.status = Some(format!("Z {} m", crate::i18n::num(draft.z_now(), 3)));
                return doc.commit(tx);
            }
            let conn = conn.filter(|c| c.kind.code() == draft.system);
            // what the run ends on: a connector, else a pipe of its system
            // (not the pipe it has just come from)
            let join: Option<DVec3> = conn.map(|c| c.at).or_else(|| {
                pipe_node_at(doc.model(), level, p, draft.z_now(), Some(&draft.system))
                    .map(|n| n.at)
                    .filter(|at| at.truncate().distance(a) > 0.01)
            });
            let b = join.map_or(p, |at| at.truncate());
            let pipe = draft.pipe(level, a, b);
            let end = pipe.end;
            let (t, id) = insert(tx, pipe);
            tx = t;
            last = Some(id);
            draft.last_z = Some(end.z);
            if let Some(at) = join {
                if (at.z - end.z).abs() > 0.01 {
                    let (t, id) = insert(tx, draft.riser(level, b, end.z, at.z));
                    tx = t;
                    last = Some(id);
                }
                // joined: the run ends there
                draft.last_z = None;
                state.pending = None;
            } else {
                state.pending = Some(b);
                state.status = Some(format!("Z {} m", crate::i18n::num(end.z, 3)));
            }
            if let Some(id) = last {
                selection.only(id);
            }
            doc.commit(tx)
        }
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
    // the heights it runs at, beside the cursor: a riser first, then
    // the pipe's own (two where it falls)
    let num = |v: f64| crate::i18n::num(v, 3);
    let mut text = String::new();
    if let Some((z0, z1)) = draft.rise() {
        text = format!("↕ {} → {}   ", num(z0), num(z1));
    }
    if (p.start.z - p.end.z).abs() > 5e-4 {
        text += &format!("Z {} → {} m", num(p.start.z), num(p.end.z));
    } else {
        text += &format!("Z {} m", num(p.start.z));
    }
    if let Some(at) = project(b.truncate().extend(a.z)) {
        let font = egui::FontId::proportional(12.0);
        let galley = painter.layout_no_wrap(text, font, egui::Color32::BLACK);
        let pos = at + egui::vec2(14.0, 10.0);
        let back = egui::Rect::from_min_size(pos, galley.size()).expand(3.0);
        painter.rect_filled(back, 3.0, egui::Color32::from_white_alpha(230));
        painter.galley(pos, galley, egui::Color32::BLACK);
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
            "hint": "Height of the pipe's axis above the floor (Z). Changed while a run is drawn, the next click rises or drops to it where the run is; a click on the same point draws the riser alone",
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
    draft.height_set = false;
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

    #[test]
    fn a_run_from_a_basin_takes_its_system_and_ends_dropping_to_another() {
        use creabim_core::Document;
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let basin = creabim_core::catalogue::starter("starter/washbasin").unwrap();
        let mut place = |x: f64| {
            let (tx, _) =
                creabim_core::objects::place(doc.model(), &basin, level, DVec2::new(x, 3.0), 0.0);
            doc.commit(tx).unwrap();
        };
        place(1.0);
        place(4.0);
        let model = doc.model();
        let hot = |x: f64| {
            let c = connector_at(model, level, DVec2::new(x - 0.08, 2.98)).unwrap();
            assert_eq!(c.kind, creabim_core::fixture::ConnKind::TV);
            c
        };
        let (a, b) = (hot(1.0), hot(4.0));
        let mut state = crate::planview::PlanState::default();
        let mut sel = crate::selection::Selection::default();
        click(
            &mut doc,
            level,
            a.at.truncate() + DVec2::new(0.01, 0.0),
            &mut state,
            &mut sel,
        )
        .unwrap();
        assert_eq!(state.members.pipe.system, "TV");
        assert_eq!(state.members.pipe.last_z, Some(a.at.z));
        let n = doc.model().len();
        click(&mut doc, level, DVec2::new(2.5, 2.0), &mut state, &mut sel).unwrap();
        // the last leg runs lower than the next tap, so it rises to it
        state.members.pipe.last_z = Some(0.3);
        click(&mut doc, level, b.at.truncate(), &mut state, &mut sel).unwrap();
        assert_eq!(doc.model().len(), n + 3, "two runs and a rise to the tap");
        assert!(state.pending.is_none(), "joining a fixture ends the run");
        let rise = doc
            .model()
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Pipe(p) if p.is_vertical() => Some(p.clone()),
                _ => None,
            })
            .next()
            .unwrap();
        assert!((rise.end - b.at).length() < 1e-9);
    }

    #[test]
    fn a_height_set_in_a_run_rises_there_and_the_run_goes_on_at_it() {
        use creabim_core::Document;
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let mut state = crate::planview::PlanState::default();
        let mut sel = crate::selection::Selection::default();
        assert!(action(&mut state.members.pipe, "pipe", "SV"));
        state.members.pipe.height = 0.3;
        let pipes = |doc: &Document| -> Vec<Pipe> {
            doc.model()
                .iter()
                .filter_map(|e| match &e.kind {
                    ElementKind::Pipe(p) => Some(p.clone()),
                    _ => None,
                })
                .collect()
        };
        let n = pipes(&doc).len();
        click(&mut doc, level, DVec2::new(1.0, 1.0), &mut state, &mut sel).unwrap();
        click(&mut doc, level, DVec2::new(3.0, 1.0), &mut state, &mut sel).unwrap();
        assert_eq!(pipes(&doc).len(), n + 1);
        // the height changed in the bar while the run is drawn
        crate::memberstool::set_option(&mut state, "member.pipe_height", 2.7);
        assert_eq!(state.members.pipe.rise(), Some((0.3, 2.7)));
        // a click on the run's own point: the riser alone
        click(&mut doc, level, DVec2::new(3.0, 1.0), &mut state, &mut sel).unwrap();
        let all = pipes(&doc);
        assert_eq!(all.len(), n + 2);
        let riser = all.iter().find(|p| p.is_vertical()).unwrap();
        assert_eq!(riser.start, DVec3::new(3.0, 1.0, 0.3));
        assert_eq!(riser.end, DVec3::new(3.0, 1.0, 2.7));
        assert_eq!(state.members.pipe.rise(), None);
        // and the run goes on up there
        click(&mut doc, level, DVec2::new(3.0, 4.0), &mut state, &mut sel).unwrap();
        let top = pipes(&doc).into_iter().find(|p| p.end.y == 4.0).unwrap();
        assert_eq!((top.start.z, top.end.z), (2.7, 2.7));
        // a drop and the next pipe in one click
        crate::memberstool::set_option(&mut state, "member.pipe_height", 1.0);
        click(&mut doc, level, DVec2::new(5.0, 4.0), &mut state, &mut sel).unwrap();
        let all = pipes(&doc);
        assert_eq!(all.len(), n + 5);
        assert!(all
            .iter()
            .any(|p| p.is_vertical() && p.start.z == 2.7 && p.end.z == 1.0));
        assert!(all.iter().any(|p| p.end == DVec3::new(5.0, 4.0, 1.0)));
    }

    #[test]
    fn a_run_joins_the_pipes_already_drawn() {
        use creabim_core::Document;
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let mut state = crate::planview::PlanState::default();
        let mut sel = crate::selection::Selection::default();
        assert!(action(&mut state.members.pipe, "pipe", "SV"));
        state.members.pipe.height = 2.5;
        action(&mut state.members.pipe, "pipe.size", "32");
        click(&mut doc, level, DVec2::new(1.0, 1.0), &mut state, &mut sel).unwrap();
        click(&mut doc, level, DVec2::new(5.0, 1.0), &mut state, &mut sel).unwrap();
        state.cancel();
        // near its end: the end itself, at its height
        let n = pipe_node_at(doc.model(), level, DVec2::new(5.05, 1.04), 0.0, None).unwrap();
        assert_eq!(n.at, DVec3::new(5.0, 1.0, 2.5));
        assert_eq!((n.system.as_str(), n.nominal_mm), ("SV", 32.0));
        // along it: the point of its axis
        let m = pipe_node_at(doc.model(), level, DVec2::new(3.0, 1.08), 0.0, None).unwrap();
        assert!((m.at - DVec3::new(3.0, 1.0, 2.5)).length() < 1e-9);
        assert!(pipe_node_at(doc.model(), level, DVec2::new(3.0, 1.5), 0.0, None).is_none());
        assert!(pipe_node_at(doc.model(), level, DVec2::new(3.0, 1.0), 0.0, Some("TV")).is_none());

        // another system and height in the bar; started on the pipe, the
        // run takes the pipe's system, size and height
        assert!(action(&mut state.members.pipe, "pipe", "TV"));
        click(
            &mut doc,
            level,
            DVec2::new(5.03, 1.02),
            &mut state,
            &mut sel,
        )
        .unwrap();
        let d = &state.members.pipe;
        assert_eq!(
            (d.system.as_str(), d.nominal_mm, d.z_now()),
            ("SV", 32.0, 2.5)
        );
        assert_eq!(state.pending, Some(DVec2::new(5.0, 1.0)));
        click(&mut doc, level, DVec2::new(5.0, 3.0), &mut state, &mut sel).unwrap();
        state.cancel();

        // a low run ended on the high pipe rises to it there, and ends
        assert!(action(&mut state.members.pipe, "pipe", "SV"));
        state.members.pipe.height = 0.3;
        click(&mut doc, level, DVec2::new(2.0, 4.0), &mut state, &mut sel).unwrap();
        let before = doc.model().len();
        click(
            &mut doc,
            level,
            DVec2::new(2.03, 1.05),
            &mut state,
            &mut sel,
        )
        .unwrap();
        assert_eq!(doc.model().len(), before + 2, "the pipe and its rise");
        assert!(state.pending.is_none());
        let rise = doc
            .model()
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Pipe(p) if p.is_vertical() => Some(p.clone()),
                _ => None,
            })
            .next()
            .unwrap();
        assert!(
            (rise.start - DVec3::new(2.03, 1.0, 0.3)).length() < 1e-9,
            "{rise:?}"
        );
        assert!((rise.end - DVec3::new(2.03, 1.0, 2.5)).length() < 1e-9);
    }
}
