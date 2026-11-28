//! Underfloor heating, system A (the pipe in the screed) after STN EN
//! 1264-2, as PilonDesigner's `floorheat.ts`, taken from the floor the
//! room stands on (George 2026-11-27): the build-up under the room gives
//! the system board (its maker's pipes and raster), the screed over it
//! (its λE and the cover su over the pipe), RλB of the layers on the
//! screed and the insulation under the pipes for the loss downwards
//! (STN EN 1264-3: qu = (Ro·q + θi − θu) / Ru); the pipe is a UFH pipe of
//! the catalogue, by default the board maker's 16 × 2. The spacing is
//! chosen for the room's heat loss (STN EN 12831-1) from those the board
//! allows; the limit heat flux qG = 8.92 · (θF,max − θi)^1.1; loops of
//! 100 m at most; the flow, the loops' pressure losses and what to
//! throttle at the manifold.

use crate::heatloss::{self, RoomLoss};
use crate::pipe_types::{self, PipeType};
use creabim_core::assembly::{assembly_of, LayerFunction};
use creabim_core::catalogue::ufhmanifold::{Cabinet, ManifoldConfig, Range};
use creabim_core::fixture;
use creabim_core::underfloor::Underfloor;
use creabim_core::{polygon, ElementId, ElementKind, Model};
use std::sync::OnceLock;

/// What the system is designed with.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Supply temperature, °C, and the drop σ, K.
    pub supply_c: f64,
    pub sigma_k: f64,
    /// The pipe: a type of the catalogue for underfloor heating and its
    /// outside diameter, mm; none takes the system board maker's 16 × 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipe: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pipe_mm: Option<f64>,
    /// The manifolds' range, none for the board maker's
    /// ([`Range::for_pipe`]); its cabinet and whether a mixing set goes in
    /// it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Range>,
    pub cabinet: Cabinet,
    pub mixing: bool,
    /// The radiators fed from the manifold of their storey, through the
    /// floor's insulation layer (George 2026-11-27).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub radiators: Vec<ElementId>,
    /// The radiators' supply and return, °C.
    pub radiator_supply_c: f64,
    pub radiator_return_c: f64,
    /// The insulation the distribution pipes get.
    pub insulation: crate::pipeinsul::Product,
    /// With the maker's regulation: a thermal actuator on every loop, a
    /// thermostat in every room, the wiring centre.
    pub regulation: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            supply_c: 35.0,
            sigma_k: 5.0,
            pipe: None,
            pipe_mm: None,
            range: None,
            cabinet: Cabinet::Flush,
            mixing: false,
            radiators: vec![],
            radiator_supply_c: crate::heating::HeatSettings::default().supply_c,
            radiator_return_c: crate::heating::HeatSettings::default().return_c,
            insulation: crate::pipeinsul::Product::default(),
            regulation: false,
        }
    }
}

/// The spacings pipes are laid at with no board that says otherwise, m.
pub const SPACINGS: [f64; 5] = [0.1, 0.15, 0.2, 0.25, 0.3];
/// The pipe with no board and none picked: PE-Xa 16 × 2.
pub const PIPE_OUTER_MM: f64 = 16.0;
pub const MAX_LOOP_M: f64 = 100.0;
/// With no build-up under the room: the share lost downwards, 45 mm of
/// screed of λ 1.2 over the pipe (as before).
const DOWN_FACTOR: f64 = 1.1;
const SU_DEFAULT: f64 = 0.045;
const LAMBDA_E_DEFAULT: f64 = 1.2;
/// The finish's RλB with neither a build-up nor a finish named.
const FINISH_R_DEFAULT: f64 = 0.1;
/// A loop's local losses (bends, the manifold).
const LOCAL_FACTOR: f64 = 1.3;
/// The manifold's, valves' and source's loss on the pump, kPa.
pub const MANIFOLD_KPA: f64 = 5.0;
/// The floor laid with pipes (edge strips, furniture).
const AREA_FACTOR: f64 = 0.9;
/// The heat transfer coefficient at the floor's surface, W/(m²K)
/// (STN EN 1264-2: 10.8).
const ALPHA: f64 = 10.8;

/// What a system board's maker sheet allows (the `ufh` of
/// `sk-ufh-boards.json`).
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct BoardSpec {
    /// The pipes' outside diameters it holds, mm.
    pub pipe_mm: Option<[f64; 2]>,
    /// The raster pipes are laid at multiples of, mm.
    pub raster_mm: Option<f64>,
    /// Or the spacings it is made for, mm.
    pub spacings_mm: Vec<f64>,
    /// "A": the pipe in the screed; "B": a dry system under a dry screed.
    pub system: String,
}

#[derive(serde::Deserialize)]
struct BoardRow {
    name: String,
    #[serde(default)]
    maker: String,
    #[serde(default)]
    ufh: Option<BoardSpec>,
}

#[derive(serde::Deserialize)]
struct Boards {
    materials: Vec<BoardRow>,
}

fn boards() -> &'static [BoardRow] {
    static ALL: OnceLock<Vec<BoardRow>> = OnceLock::new();
    ALL.get_or_init(|| {
        serde_json::from_str::<Boards>(include_str!("../../creabim-core/data/sk-ufh-boards.json"))
            .map(|b| b.materials)
            .unwrap_or_default()
    })
}

/// The maker and the sheet's data of the system board named so.
pub fn board_spec(name: &str) -> Option<(&'static str, &'static BoardSpec)> {
    boards()
        .iter()
        .find(|b| b.name.trim() == name.trim())
        .and_then(|b| b.ufh.as_ref().map(|u| (b.maker.as_str(), u)))
}

impl BoardSpec {
    /// The spacings it can be laid at, m: those it is made for, or the
    /// raster's multiples from 100 to 300 mm (the range of [`SPACINGS`]).
    pub fn spacings(&self) -> Vec<f64> {
        if !self.spacings_mm.is_empty() {
            return self.spacings_mm.iter().map(|v| v / 1000.0).collect();
        }
        match self.raster_mm.filter(|r| *r > 0.0) {
            Some(r) => (1..=20)
                .map(|k| k as f64 * r)
                .filter(|v| (100.0 - 1e-6..=300.0 + 1e-6).contains(v))
                .map(|v| v / 1000.0)
                .collect(),
            None => SPACINGS.to_vec(),
        }
    }
}

/// The floor's build-up a room stands on, as the calculation reads it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FloorBuild {
    pub assembly: String,
    /// The system board's material, its maker and its sheet's data.
    pub board: Option<String>,
    pub maker: String,
    pub spec: BoardSpec,
    /// The screed the pipes lie in: its name, thickness, m, and λE.
    pub screed: String,
    pub screed_t: f64,
    pub lambda_e: f64,
    /// The layers on the screed (the finish) and their RλB, m²K/W.
    pub finish: Vec<String>,
    pub finish_r: f64,
    /// The layers under the pipes, the board's first, and their R, m²K/W.
    pub under: Vec<String>,
    pub r_under: f64,
}

/// The layer's R, m²K/W, and the material's name; none with no λ.
fn layer_r(model: &Model, l: &creabim_core::assembly::Layer) -> Option<(f64, String)> {
    let Some(m) = l.material else {
        return Some((0.18, String::new()));
    };
    let e = model.get(m)?;
    let ElementKind::Material(mat) = &e.kind else {
        return None;
    };
    let lam = mat.lambda.filter(|x| *x > 0.0)?;
    Some((l.thickness / lam, e.name.clone()))
}

fn material_of(
    model: &Model,
    l: &creabim_core::assembly::Layer,
) -> Option<(String, String, Option<f64>)> {
    let e = model.get(l.material?)?;
    let ElementKind::Material(m) = &e.kind else {
        return None;
    };
    Some((e.name.clone(), m.category.clone(), m.lambda))
}

/// The build-up of the floor slab under the room's point `at` on its
/// level (as the heat loss finds it), read for underfloor heating.
pub fn floor_build(model: &Model, level: ElementId, at: glam::DVec2) -> Option<FloorBuild> {
    let slab = model.iter().find(|e| {
        matches!(&e.kind, ElementKind::Slab(s)
            if s.level == level && s.boundary.len() > 2 && polygon::contains(&s.boundary, at))
    })?;
    let (ae, asm) = assembly_of(model, slab)?;
    let ls = &asm.layers;
    let mut out = FloorBuild {
        assembly: ae.name.clone(),
        lambda_e: LAMBDA_E_DEFAULT,
        ..FloorBuild::default()
    };
    // the system board, from the top
    let b = ls
        .iter()
        .position(|l| material_of(model, l).is_some_and(|(_, c, _)| c == "system-board"));
    // the screed: the substrate over the board, or the top one
    let s = match b {
        Some(b) => (0..b)
            .rev()
            .find(|i| ls[*i].function == LayerFunction::Substrate && ls[*i].thickness >= 0.02),
        None => ls
            .iter()
            .position(|l| l.function == LayerFunction::Substrate && l.thickness >= 0.02),
    };
    if let Some(b) = b {
        let (name, _, _) = material_of(model, &ls[b]).unwrap_or_default();
        if let Some((maker, spec)) = board_spec(&name) {
            out.maker = maker.to_string();
            out.spec = spec.clone();
        }
        out.board = Some(name);
    }
    if let Some(s) = s {
        let (name, _, lam) = material_of(model, &ls[s]).unwrap_or_default();
        out.screed = name;
        out.screed_t = ls[s].thickness;
        if let Some(l) = lam.filter(|x| *x > 0.0) {
            out.lambda_e = l;
        }
        for l in &ls[..s] {
            if l.thickness <= 0.0 {
                continue;
            }
            if let Some((r, n)) = layer_r(model, l) {
                out.finish_r += r;
                out.finish.push(n);
            }
        }
    }
    // under the pipes: from the board down, else from under the screed
    let from = b.or(s.map(|s| s + 1));
    if let Some(from) = from {
        for l in &ls[from..] {
            if l.thickness <= 0.0 {
                continue;
            }
            if let Some((r, n)) = layer_r(model, l) {
                out.r_under += r;
                out.under.push(n);
            }
        }
    }
    Some(out)
}

/// RλB of a floor finish, m²K/W, from its description (STN EN 1264-2:
/// usually 0.0 to 0.15): for a room with no build-up under it.
pub fn finish_r(finish: &str) -> f64 {
    let f = finish.to_lowercase();
    let has = |keys: &[&str]| keys.iter().any(|k| f.contains(k));
    if has(&[
        "dlažb", "dlazb", "kame", "keram", "epox", "liaty", "tile", "stone", "ceramic",
    ]) {
        0.02
    } else if has(&["vinyl", "pvc"]) {
        0.05
    } else if has(&["lamin"]) {
        0.08
    } else if has(&["drev", "parket", "wood", "parquet", "timber"]) {
        0.1
    } else if has(&["kober", "carpet"]) {
        0.15
    } else {
        FINISH_R_DEFAULT
    }
}

/// The equivalent coefficient KH, W/(m²K): q = KH · ΔθH (STN EN 1264-2,
/// system A: aB with the screed's λE, aT by RλB, aD^mD by the pipe's
/// outside diameter d, m; the cover's au^mu is left at 1 until its table
/// is in the norm's parameter file).
pub fn k_h(t: f64, r: f64, lambda_e: f64, d: f64) -> f64 {
    let a_b = (1.0 / ALPHA + 0.045 / 1.0) / (1.0 / ALPHA + 0.045 / lambda_e + r);
    let a_t = if r <= 0.0 {
        1.23
    } else if r <= 0.05 {
        1.23 - 0.042 * r / 0.05
    } else if r <= 0.1 {
        1.188 - 0.032 * (r - 0.05) / 0.05
    } else {
        1.156 - 0.022 * (r - 0.1).min(0.05) / 0.05
    };
    let m_t = 1.0 - t / 0.075;
    let (a_d, m_d): (f64, f64) = (1.03, 250.0 * (d - 0.02));
    6.7 * a_b * a_t.powf(m_t) * a_d.powf(m_d)
}

/// The logarithmic temperature difference ΔθH.
pub fn d_theta_h(supply: f64, ret: f64, ti: f64) -> f64 {
    (supply - ret) / ((supply - ti) / (ret - ti)).ln()
}

/// The edge zone's limit with no norm tables loaded (θF,max 35 °C).
fn q_limit_edge(ti: f64) -> f64 {
    8.92 * (35.0 - ti).max(0.0).powf(1.1)
}

/// The limit heat flux qG, W/m² (θF,max 29 °C where people stay, 33 °C
/// in a bathroom).
pub fn q_limit(ti: f64, bath: bool) -> f64 {
    8.92 * (if bath { 33.0 } else { 29.0 } - ti).max(0.0).powf(1.1)
}

/// What is not as it should be in a room's floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Note {
    /// No slab with a build-up under the room: the defaults are used.
    NoBuildUp,
    /// The build-up has no system board.
    NoBoard,
    /// The pipe is not one the board holds.
    PipeNotForBoard,
    /// The room's own spacing is not one the board allows.
    SpacingNotForBoard,
    /// A dry system: system A's characteristic does not hold for it.
    DrySystem,
    /// The cover over the pipe is less than the pipe's diameter.
    ThinCover,
    /// The layers under the pipes resist less than STN EN 1264-4 asks.
    InsulationThin,
}

impl Note {
    pub fn text(self) -> &'static str {
        match self {
            Note::NoBuildUp => "No floor build-up under the room: 45 mm of screed, RλB of the floor finish and 10 % downwards are assumed",
            Note::NoBoard => "The floor build-up has no underfloor heating system board",
            Note::PipeNotForBoard => "The pipe is not one the system board holds",
            Note::SpacingNotForBoard => "The spacing is not one the system board allows",
            Note::DrySystem => "A dry system: the characteristic of system A does not hold for it",
            Note::ThinCover => "The screed over the pipe is thinner than the pipe",
            Note::InsulationThin => "The layers under the pipes resist less than STN EN 1264-4, table 1 asks (Rλ,ins)",
        }
    }
}

/// The pipe the system is laid of.
#[derive(Clone, Debug, PartialEq)]
pub struct Pipe {
    pub id: String,
    pub name: String,
    pub segment: String,
    pub code: String,
    pub outer_mm: f64,
    pub inner_mm: f64,
    pub roughness_mm: f64,
}

/// The catalogue's pipes for underfloor heating (STN EN 1264).
pub fn ufh_pipes() -> impl Iterator<Item = &'static PipeType> {
    pipe_types::all()
        .iter()
        .filter(|t| t.standards.contains("1264") && !t.sizes.is_empty())
}

fn pipe_of(t: &PipeType, outer: f64) -> Option<Pipe> {
    let z = t.closest_outer(outer)?;
    Some(Pipe {
        id: t.id.clone(),
        name: t.name.clone(),
        segment: z
            .segment
            .clone()
            .unwrap_or_else(|| format!("{} x {}", z.outer_mm, (z.outer_mm - z.inner_mm) / 2.0)),
        code: z.code.clone().unwrap_or_default(),
        outer_mm: z.outer_mm,
        inner_mm: z.inner_mm,
        roughness_mm: t.roughness_mm,
    })
}

/// The pipe picked, else the board maker's UFH pipe 16 × 2 (HERZ: HERZ
/// plasthliník for UFH, Wavin: PE-RT/EVOH), else PE-Xa 16 × 2.
pub fn pipe(s: &Settings, maker: &str) -> Pipe {
    if let Some(p) = s
        .pipe
        .as_deref()
        .and_then(pipe_types::by_id)
        .and_then(|t| pipe_of(t, s.pipe_mm.unwrap_or(PIPE_OUTER_MM)))
    {
        return p;
    }
    let id = match maker {
        "HERZ" => "HERZFH",
        "Wavin" => "WFHE",
        _ => "PEX",
    };
    pipe_types::by_id(id)
        .and_then(|t| pipe_of(t, PIPE_OUTER_MM))
        .unwrap_or(Pipe {
            id: "PEX".into(),
            name: "PE-Xa".into(),
            segment: "16 x 2".into(),
            code: String::new(),
            outer_mm: 16.0,
            inner_mm: 12.0,
            roughness_mm: 0.007,
        })
}

/// The rooms with underfloor heating open to each other (a room
/// separator on their common edge, no wall) taken as one space: each
/// knows the others, and the space's loss and what its floors give.
fn open_spaces(model: &Model, rooms: &[heatloss::RoomAt<'_>], designs: &mut [RoomDesign]) {
    use creabim_core::ElementKind;
    let seps: Vec<(ElementId, glam::DVec2, glam::DVec2)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::RoomSeparator(s) => Some((s.level, s.a, s.b)),
            _ => None,
        })
        .collect();
    if seps.is_empty() {
        return;
    }
    let near = |o: &[glam::DVec2], p: glam::DVec2| {
        (0..o.len()).any(|i| {
            let (a, b) = (o[i], o[(i + 1) % o.len()]);
            let d = b - a;
            let t = ((p - a).dot(d) / d.length_squared().max(1e-12)).clamp(0.0, 1.0);
            (a + d * t).distance(p) < 0.05
        })
    };
    let on: Vec<(usize, ElementId, ElementId, Vec<glam::DVec2>)> = designs
        .iter()
        .enumerate()
        .filter(|(_, x)| x.on)
        .filter_map(|(i, x)| {
            let r = rooms.iter().find(|r| r.e.id == x.id)?;
            Some((i, x.id, r.level, r.shape.outline.clone()))
        })
        .collect();
    // which open to which, joined into spaces
    let mut group: Vec<usize> = (0..designs.len()).collect();
    fn root(g: &mut [usize], i: usize) -> usize {
        let mut i = i;
        while g[i] != i {
            g[i] = g[g[i]];
            i = g[i];
        }
        i
    }
    for (k, a) in on.iter().enumerate() {
        for b in &on[k + 1..] {
            if a.2 != b.2 {
                continue;
            }
            let open = seps.iter().any(|(lv, p, q)| {
                *lv == a.2
                    && p.distance(*q) > 0.3
                    && [*p, (*p + *q) / 2.0, *q]
                        .iter()
                        .all(|m| near(&a.3, *m) && near(&b.3, *m))
            });
            if open {
                designs[a.0].open_to.push(b.1);
                designs[b.0].open_to.push(a.1);
                let (ra, rb) = (root(&mut group, a.0), root(&mut group, b.0));
                group[ra] = rb;
            }
        }
    }
    let mut totals: std::collections::HashMap<usize, (f64, f64)> = Default::default();
    for (i, x) in designs.iter().enumerate().filter(|(_, x)| x.on) {
        let r = root(&mut group, i);
        let t = totals.entry(r).or_default();
        t.0 += x.phi;
        t.1 += x.covered;
    }
    for (i, x) in designs.iter_mut().enumerate() {
        if !x.on || x.open_to.is_empty() {
            continue;
        }
        let r = root(&mut group, i);
        if let Some((phi, covered)) = totals.get(&r) {
            x.space_phi = *phi;
            x.space_covered = *covered;
        }
    }
}

/// A room's underfloor heating designed.
#[derive(Clone, Debug, PartialEq)]
pub struct RoomDesign {
    pub id: ElementId,
    pub number: String,
    pub name: String,
    /// The room's design temperature, °C.
    pub ti: f64,
    /// The room's heat loss, W.
    pub phi: f64,
    /// The floor laid with pipes, m².
    pub area: f64,
    /// The heat flux needed and given, W/m², and the limit.
    pub q_req: f64,
    pub q: f64,
    pub q_limit: f64,
    /// The spacing, m; `manual` when the room's own was kept; those the
    /// board allows.
    pub spacing: f64,
    pub manual: bool,
    pub spacings: Vec<f64>,
    pub finish_r: f64,
    /// The build-up under the room, none when there is none.
    pub floor: Option<FloorBuild>,
    /// The cover over the pipe su, m, and the screed's λE.
    pub su: f64,
    pub lambda_e: f64,
    /// Upwards Ro and downwards Ru, m²K/W, the temperature under the
    /// floor θu, °C, and the flux lost downwards qu, W/m² (STN EN 1264-3).
    pub r_up: f64,
    pub r_down: Option<f64>,
    pub theta_u: f64,
    pub q_down: f64,
    pub notes: Vec<Note>,
    /// The room's underfloor heating in the model.
    pub ufh: Option<Underfloor>,
    /// Its edge zones' area, m², spacing, m, and flux, W/m²; the places of
    /// reduced output's area, m², and the output they take off, W.
    pub edge_area: f64,
    pub edge_spacing: Option<f64>,
    pub q_edge: f64,
    pub reduced_area: f64,
    pub reduced_w: f64,
    /// The holes' area, m²: the board with no pipe in it.
    pub hole_area: f64,
    /// What the floor gives, W.
    pub covered: f64,
    pub loops: u32,
    /// A loop's length with its way from the manifold and back, m.
    pub loop_len: f64,
    /// The way from the manifold, m (0 with none on the storey).
    pub feed: f64,
    /// A loop's flow, l/s, and loss, kPa.
    pub flow_ls: f64,
    pub dp_kpa: f64,
    /// The floor covers the loss.
    pub ok: bool,
    /// The rooms with underfloor heating it is open to (a room separator
    /// between them, no wall): one space, its loss and what its floors
    /// give together, W (George 2026-11-28: the kitchen open to the
    /// living room). Alone, its own.
    pub open_to: Vec<ElementId>,
    pub space_phi: f64,
    pub space_covered: f64,
    /// The room has it in the model.
    pub on: bool,
    /// Along its walls, m (the edge strip).
    pub perimeter: f64,
    /// Its doorways: the joints in the screed (Wavin handbook p. 42).
    pub doors: Vec<Doorway>,
    /// The doorway its loops leave by to the manifold.
    pub door: Option<ElementId>,
    /// The way from the manifold: the doorways crossed and the rooms
    /// passed through.
    pub route: Option<crate::ufhroute::Route>,
}

/// A doorway of a room.
#[derive(Clone, Debug, PartialEq)]
pub struct Doorway {
    pub id: ElementId,
    pub at: glam::DVec2,
    pub width: f64,
}

/// A circuit of a manifold balanced: a loop or a radiator's outlet.
#[derive(Clone, Debug, PartialEq)]
pub struct Circuit {
    /// The room's number (and the loop's ordinal of several).
    pub label: String,
    /// The room or the radiator.
    pub id: ElementId,
    pub radiator: bool,
    pub flow_ls: f64,
    /// The pipe's loss there and back, kPa.
    pub dp_pipe_kpa: f64,
    /// The valve's presetting (turns) and its kv, by the maker's table.
    pub preset: Option<&'static str>,
    pub kv: Option<f64>,
    /// What the valve takes at its presetting, kPa.
    pub dp_valve_kpa: f64,
    /// Above what the flowmeter shows.
    pub over_flowmeter: bool,
    /// The index circuit: wide open.
    pub index: bool,
    /// Even the smallest presetting throttles too little.
    pub under: bool,
}

impl Circuit {
    /// The flow to set on its flowmeter, l/min.
    pub fn flow_lmin(&self) -> f64 {
        self.flow_ls * 60.0
    }
}

impl RoomDesign {
    /// The room's underfloor heating as designed, keeping how it is
    /// laid and its zones.
    pub fn underfloor(&self, own_r: Option<f64>) -> Underfloor {
        Underfloor {
            spacing: self.spacing,
            loops: self.loops,
            finish_r: own_r,
            ..self.ufh.clone().unwrap_or_default()
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Design {
    /// Every room, those with underfloor heating marked `on`.
    pub rooms: Vec<RoomDesign>,
    /// The pipe laid.
    pub pipe: Option<Pipe>,
    /// Of the rooms that have it: the output, kW; the manifold's flow,
    /// l/s; the worst loop's loss and the pump's head, kPa.
    pub output_kw: f64,
    pub flow_ls: f64,
    pub critical_kpa: f64,
    pub head_kpa: f64,
    pub loops: u32,
    /// No manifold stands in the model.
    pub no_manifold: bool,
    /// A manifold for each storey with underfloor heating.
    pub manifolds: Vec<ManifoldDesign>,
    /// With the maker's regulation (actuators, thermostats).
    pub regulation: bool,
    /// Calculated from the tables of STN EN 1264-2 (else approximated).
    pub norm: bool,
}

/// The maker's pipe for distribution (from the source to the manifold,
/// from the manifold to the radiators): HERZ PipeFix, Wavin Tigris K5;
/// PEX-AL-PEX for another maker.
pub fn distribution_type(maker: &str) -> &'static str {
    let m = maker.to_lowercase();
    if m.contains("wavin") {
        "WTIGK"
    } else if m.contains("herz") {
        "HERZ"
    } else {
        "PEXAL"
    }
}

/// A distribution pipe sized for its flow.
#[derive(Clone, Debug, PartialEq)]
pub struct SizedPipe {
    pub pipe: Pipe,
    /// Its flow, l/s, velocity, m/s, and loss by friction, Pa/m.
    pub flow_ls: f64,
    pub velocity: f64,
    pub r_pa_m: f64,
    /// Its heat insulation, mm at λ 0,035 (as a heating pipe of the
    /// services), and the maker's tube that does as well.
    pub insulation_mm: f64,
    pub tube: Option<crate::pipeinsul::Tube>,
}

/// The smallest size of the type `id` that carries `flow_ls` at
/// `vmax` m/s and `rmax` Pa/m at most (the rules of the radiators'
/// heating, [`crate::heating::HeatSettings`]); the biggest when none does.
pub fn size_pipe(id: &str, flow_ls: f64, vmax: f64, rmax: f64, temp_c: f64) -> Option<SizedPipe> {
    let t = pipe_types::by_id(id)?;
    let hs = crate::heating::HeatSettings::default();
    let fluid = crate::fluid::Fluid::water(temp_c);
    let k = t.roughness_mm / 1000.0;
    let fits = |inner: f64| {
        let d = inner / 1000.0;
        crate::hydraulics::velocity(flow_ls, d) <= vmax
            && crate::hydraulics::r(flow_ls, d, k, &fluid) <= rmax
    };
    let z = t
        .sizes
        .iter()
        .filter(|z| z.inner_mm >= hs.min_inner_mm)
        .find(|z| fits(z.inner_mm))
        .or_else(|| t.sizes.last())?;
    let pipe = pipe_of(t, z.outer_mm)?;
    let d = pipe.inner_mm / 1000.0;
    Some(SizedPipe {
        flow_ls,
        velocity: crate::hydraulics::velocity(flow_ls, d),
        r_pa_m: crate::hydraulics::r(flow_ls, d, k, &fluid),
        insulation_mm: creabim_core::mep::heat_insulation_mm(pipe.inner_mm),
        tube: None,
        pipe,
    })
}

/// Its tube of `product` for water at `medium_c`.
fn insulated(
    p: Option<SizedPipe>,
    product: crate::pipeinsul::Product,
    medium_c: f64,
) -> Option<SizedPipe> {
    p.map(|mut p| {
        p.tube = product.tube(p.pipe.outer_mm, p.insulation_mm, medium_c);
        p
    })
}

/// A radiator fed from a manifold.
#[derive(Clone, Debug, PartialEq)]
pub struct RadiatorFeed {
    pub id: ElementId,
    /// The number of the room it stands in.
    pub room: String,
    /// Its output at the radiators' temperatures, W.
    pub output_w: f64,
    /// The way to it and back, m (as the crow flies × 1.2 each way).
    pub length: f64,
    pub pipe: Option<SizedPipe>,
}

/// The numbers of the rooms objects stand in.
pub fn room_numbers(model: &Model, objects: &[&creabim_core::Element]) -> Vec<(ElementId, String)> {
    let rooms = heatloss::rooms_of(model);
    objects
        .iter()
        .filter_map(|e| {
            let ElementKind::Object(o) = &e.kind else {
                return None;
            };
            // a little into the room from the wall it hangs on (−y)
            let p = o.to_plan(glam::DVec2::new(0.0, -0.2));
            rooms
                .iter()
                .find(|x| x.level == o.level && polygon::contains(&x.shape.outline, p))
                .and_then(|x| match &x.e.kind {
                    ElementKind::Room(r) => Some((e.id, r.number.clone())),
                    _ => None,
                })
        })
        .collect()
}

/// A storey's manifold: the product its circuits pick, the pipe that
/// feeds it from the source and the radiators on it.
#[derive(Clone, Debug, PartialEq)]
pub struct ManifoldDesign {
    pub level: ElementId,
    /// The manifold standing on the storey, none when none does yet.
    pub element: Option<ElementId>,
    pub config: ManifoldConfig,
    /// The underfloor heating's flow and the radiators', l/s.
    pub flow_ls: f64,
    pub radiators: Vec<RadiatorFeed>,
    pub feed: Option<SizedPipe>,
    /// Its circuits balanced, and the index circuit's loss with its
    /// valve wide open, kPa.
    pub circuits: Vec<Circuit>,
    pub dp_kpa: f64,
}

impl Design {
    /// The build-ups the rooms with underfloor heating stand on, each
    /// once, with their rooms' numbers.
    pub fn floors(&self) -> Vec<(&FloorBuild, Vec<&str>)> {
        let mut out: Vec<(&FloorBuild, Vec<&str>)> = vec![];
        for x in self.rooms.iter().filter(|x| x.on) {
            let Some(f) = &x.floor else { continue };
            match out.iter_mut().find(|(g, _)| g.assembly == f.assembly) {
                Some((_, n)) => n.push(&x.number),
                None => out.push((f, vec![&x.number])),
            }
        }
        out
    }
}

/// A loop's pressure loss, kPa, at `flow_ls` over `len` metres.
fn loop_loss(p: &Pipe, flow_ls: f64, len: f64) -> f64 {
    let d = p.inner_mm / 1000.0;
    let v = flow_ls / 1000.0 / (std::f64::consts::PI * d * d / 4.0);
    if v <= 0.0 {
        return 0.0;
    }
    let re = v * d / 0.75e-6;
    let mut lam: f64 = 0.03;
    for _ in 0..30 {
        lam = (-2.0 * (2.51 / (re * lam.sqrt()) + p.roughness_mm / 1000.0 / (3.71 * d)).log10())
            .powi(-2);
    }
    if re < 2300.0 {
        lam = 64.0 / re.max(1.0);
    }
    lam / d * 994.0 * v * v / 2.0 * len * LOCAL_FACTOR / 1000.0
}

/// What is under a room: the room's temperature over it, or none on the
/// ground.
fn below_ti(
    rooms: &[heatloss::RoomAt],
    model: &Model,
    level: ElementId,
    at: glam::DVec2,
) -> Option<f64> {
    let elev = |id: ElementId| model.level(id).map_or(0.0, |l| l.elevation);
    rooms
        .iter()
        .filter(|x| elev(x.level) < elev(level) - 1e-6 && polygon::contains(&x.shape.outline, at))
        .max_by(|a, b| elev(a.level).total_cmp(&elev(b.level)))
        .map(|x| x.ti)
}

/// A storey's manifold for the ways to the rooms: its storey, the room
/// it stands in, the point in front of it and the storey's doorways.
pub struct Feeds {
    pub level: ElementId,
    pub start: ElementId,
    pub from: glam::DVec2,
    pub doors: Vec<crate::ufhroute::Door>,
}

/// The ways from each storey's manifold.
fn feeds(model: &Model, rooms: &[heatloss::RoomAt]) -> Vec<Feeds> {
    let mut out: Vec<Feeds> = vec![];
    for e in model.iter() {
        let ElementKind::Object(ob) = &e.kind else {
            continue;
        };
        if !fixture::family_of(model, e).is_some_and(fixture::is_manifold) {
            continue;
        }
        if out.iter().any(|f| f.level == ob.level) {
            continue;
        }
        let on: Vec<(ElementId, Vec<glam::DVec2>)> = rooms
            .iter()
            .filter(|r| r.level == ob.level)
            .map(|r| (r.e.id, r.shape.outline.clone()))
            .collect();
        // in front of it (−y is the room)
        let from = ob.to_plan(glam::DVec2::new(0.0, -0.3));
        let Some(start) = on
            .iter()
            .find(|r| r.1.len() > 2 && polygon::contains(&r.1, from))
            .map(|r| r.0)
        else {
            continue;
        };
        out.push(Feeds {
            level: ob.level,
            start,
            from,
            doors: crate::ufhroute::doors(model, ob.level, &on),
        });
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn one(
    model: &Model,
    s: &Settings,
    o: &heatloss::Opts,
    hl: &RoomLoss,
    manifolds: &[(ElementId, glam::DVec2)],
    rooms: &[heatloss::RoomAt],
    pipe: &Pipe,
    max_flow_ls: Option<f64>,
    norm: Option<&crate::ufhnorm::Norm>,
    ways: &[Feeds],
) -> Option<RoomDesign> {
    let e = model.get(hl.id)?;
    let ElementKind::Room(r) = &e.kind else {
        return None;
    };
    let ti = hl.ti;
    let area = hl.area * AREA_FACTOR;
    let bath = hl.kind.needs().wet;
    let ret = s.supply_c - s.sigma_k;
    let own = r.ufh.as_ref();
    let floor = floor_build(model, r.level, r.at);
    let d = pipe.outer_mm / 1000.0;
    let mut notes = vec![];
    let (su, lambda_e, build_r) = match &floor {
        Some(f) if f.screed_t > 0.0 => ((f.screed_t - d).max(0.0), f.lambda_e, Some(f.finish_r)),
        _ => (SU_DEFAULT, LAMBDA_E_DEFAULT, None),
    };
    match &floor {
        None => notes.push(Note::NoBuildUp),
        Some(f) if f.board.is_none() => notes.push(Note::NoBoard),
        Some(f) => {
            if let Some([lo, hi]) = f.spec.pipe_mm {
                if pipe.outer_mm < lo - 1e-6 || pipe.outer_mm > hi + 1e-6 {
                    notes.push(Note::PipeNotForBoard);
                }
            }
            if f.spec.system == "B" {
                notes.push(Note::DrySystem);
            }
        }
    }
    if floor.as_ref().is_some_and(|f| f.screed_t > 0.0) && su < d {
        notes.push(Note::ThinCover);
    }
    let rf = own
        .and_then(|u| u.finish_r)
        .or(build_r)
        .unwrap_or_else(|| finish_r(&r.floor_finish));
    let spacings = floor
        .as_ref()
        .map(|f| f.spec.spacings())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| SPACINGS.to_vec());
    let dt = d_theta_h(s.supply_c, ret, ti);
    let zone = if bath {
        crate::ufhnorm::Zone::Bathroom
    } else {
        crate::ufhnorm::Zone::Occupied
    };
    let q_req = if area > 0.0 { hl.phi / area } else { 0.0 };
    // STN EN 1264-2 from the norm's tables: KH · min(ΔθH, ΔθH,G) of the
    // zone; else the old approximation
    let flux_in = |t: f64, z: crate::ufhnorm::Zone| match norm {
        Some(n) => {
            n.k_h(t, su, lambda_e, rf, d) * dt.min(n.dtheta_hg(z, ti, t, su, lambda_e, rf, d))
        }
        None => (k_h(t, rf, lambda_e, d) * dt).min(match z {
            crate::ufhnorm::Zone::Peripheral => q_limit_edge(ti),
            _ => q_limit(ti, bath),
        }),
    };
    let limit = |t: f64| match norm {
        Some(n) => n.q_g(zone, ti, t, su, lambda_e, rf, d),
        None => q_limit(ti, bath),
    };
    let flux = |t: f64| flux_in(t, zone);
    // the edge zones at their spacing; the places of reduced output
    // give less by their share (George 2026-11-27)
    let edges: Vec<(f64, f64)> = own
        .map(|u| u.edges().map(|(z, t)| (z.area(), t)).collect())
        .unwrap_or_default();
    let edge_area = edges.iter().map(|e| e.0).sum::<f64>().min(area);
    let edge_w: f64 = edges
        .iter()
        .map(|(a, t)| a * flux_in(*t, crate::ufhnorm::Zone::Peripheral))
        .sum::<f64>()
        * if edges.iter().map(|e| e.0).sum::<f64>() > area {
            area / edges.iter().map(|e| e.0).sum::<f64>()
        } else {
            1.0
        };
    let reduced: Vec<(f64, f64)> = own
        .map(|u| u.reduced().map(|(z, p)| (z.area(), p / 100.0)).collect())
        .unwrap_or_default();
    // a hole has the board but no pipe: it gives nothing
    let hole_area: f64 = own
        .map(|u| u.holes().map(|z| z.area()).sum::<f64>())
        .unwrap_or(0.0)
        .min((area - edge_area).max(0.0));
    let reduced_area = reduced.iter().map(|r| r.0).sum::<f64>();
    let gives = |t: f64| {
        let q = flux(t);
        let off: f64 = reduced.iter().map(|(a, p)| a * q * p).sum();
        ((area - edge_area - hole_area) * q - off + edge_w, off)
    };
    // the widest spacing that covers the loss; the room's own is kept
    let spacing = own.map(|u| u.spacing).unwrap_or_else(|| {
        spacings
            .iter()
            .rev()
            .copied()
            .find(|t| gives(*t).0 >= hl.phi)
            .unwrap_or(spacings[0])
    });
    if own.is_some() && !spacings.iter().any(|v| (v - spacing).abs() < 1e-6) {
        notes.push(Note::SpacingNotForBoard);
    }
    let q_main = flux(spacing);
    let (covered, reduced_w) = gives(spacing);
    // the flux over the whole floor, as the loss and the flow take it
    let q = if area > 0.0 { covered / area } else { q_main };
    let q_edge = if edge_area > 0.0 {
        edge_w / edge_area
    } else {
        0.0
    };
    let _ = q_req;
    // downwards (STN EN 1264-3): to the room under it, or the ground at
    // the year's mean outside temperature (as STN EN 12831-1 takes it)
    let below = below_ti(rooms, model, r.level, r.at);
    let theta_u = below.unwrap_or(o.te_mean.unwrap_or(o.te));
    // STN EN 1264-4, table 1: the layers under the pipes resist enough
    if let (Some(n), Some(f)) = (norm, floor.as_ref()) {
        let need = n.r_ins_min(below.is_some_and(|t| t >= 15.0), None);
        if need.is_some_and(|m| f.r_under + 1e-9 < m) {
            notes.push(Note::InsulationThin);
        }
    }
    let r_up = 1.0 / ALPHA + rf + su / lambda_e;
    let r_down = floor.as_ref().filter(|f| f.r_under > 0.0).map(|f| {
        f.r_under
            + if below.is_some() {
                heatloss::R_FLOOR
            } else {
                0.0
            }
    });
    let q_down = match r_down {
        Some(ru) => ((r_up * q + ti - theta_u) / ru).max(0.0),
        None => q * (DOWN_FACTOR - 1.0),
    };
    // from the storey's manifold to the room, as the crow flies × 1.2
    // the way from the storey's manifold through the doorways, else as
    // the crow flies × 1.2
    let route = ways.iter().find(|f| f.level == r.level).and_then(|f| {
        crate::ufhroute::route(&f.doors, f.start, f.from, hl.id, own.and_then(|u| u.door))
    });
    let feed = match &route {
        Some(w) => w.length,
        None => manifolds
            .iter()
            .find(|m| m.0 == r.level)
            .map_or(0.0, |m| m.1.distance(r.at) * 1.2),
    };
    let coil =
        (area - edge_area - hole_area) / spacing + edges.iter().map(|(a, t)| a / t).sum::<f64>();
    // loops of 100 m at most, each within what its flowmeter shows
    let flow_all = area * (q + q_down) / (4180.0 * s.sigma_k);
    let by_flow = max_flow_ls.map_or(1.0, |m| (flow_all / m - 1e-9).ceil());
    let loops = ((coil + 2.0 * feed) / MAX_LOOP_M)
        .ceil()
        .max(by_flow)
        .max(1.0) as u32;
    let loop_len = coil / loops as f64 + 2.0 * feed;
    let flow_ls = flow_all / loops as f64;
    let outline = rooms
        .iter()
        .find(|x| x.e.id == hl.id)
        .map(|x| x.shape.outline.clone())
        .unwrap_or_default();
    let perimeter = perimeter_of(&outline);
    let doors = doorways(model, r.level, &outline);
    let door = match &route {
        Some(w) => w.doors.last().copied(),
        None => manifolds
            .iter()
            .find(|m| m.0 == r.level)
            .and_then(|m| {
                doors
                    .iter()
                    .min_by(|a, b| a.at.distance(m.1).total_cmp(&b.at.distance(m.1)))
            })
            .map(|d| d.id),
    };
    Some(RoomDesign {
        id: hl.id,
        number: hl.number.clone(),
        name: hl.name.clone(),
        ti,
        phi: hl.phi,
        area,
        q_req: if area > 0.0 { hl.phi / area } else { 0.0 },
        q,
        q_limit: limit(spacing),
        spacing,
        manual: own.is_some(),
        spacings,
        finish_r: rf,
        floor,
        su,
        lambda_e,
        r_up,
        r_down,
        theta_u,
        q_down,
        notes,
        ufh: own.cloned(),
        edge_area,
        edge_spacing: edges.first().map(|e| e.1),
        q_edge,
        reduced_area,
        reduced_w,
        hole_area,
        covered,
        loops,
        loop_len,
        feed,
        flow_ls,
        dp_kpa: loop_loss(pipe, flow_ls, loop_len),
        ok: covered >= hl.phi * 0.98,
        open_to: vec![],
        space_phi: hl.phi,
        space_covered: covered,
        on: own.is_some(),
        perimeter,
        doors,
        door,
        route,
    })
}

/// An outline's length round, m.
fn perimeter_of(outline: &[glam::DVec2]) -> f64 {
    let n = outline.len();
    if n < 2 {
        return 0.0;
    }
    let mut l: f64 = outline.windows(2).map(|w| w[0].distance(w[1])).sum();
    if outline[0].distance(outline[n - 1]) > 1e-9 {
        l += outline[0].distance(outline[n - 1]);
    }
    l
}

/// The doors in the walls round an outline on a storey.
fn doorways(model: &Model, level: ElementId, outline: &[glam::DVec2]) -> Vec<Doorway> {
    let mut out = vec![];
    if outline.len() < 3 {
        return out;
    }
    for e in model.iter() {
        let ElementKind::Opening(o) = &e.kind else {
            continue;
        };
        if o.kind != creabim_core::OpeningKind::Door {
            continue;
        }
        let Some(ElementKind::Wall(w)) = model.get(o.host).map(|h| &h.kind) else {
            continue;
        };
        if w.level != level {
            continue;
        }
        let along = (w.end - w.start).normalize_or_zero();
        let mid = w.start + along * (o.offset + o.width / 2.0);
        let n = glam::DVec2::new(-along.y, along.x) * (w.thickness / 2.0 + 0.15);
        if polygon::contains(outline, mid + n) || polygon::contains(outline, mid - n) {
            out.push(Doorway {
                id: e.id,
                at: mid,
                width: o.width,
            });
        }
    }
    out
}

/// The pipe of the model's underfloor heating: the one picked, else of
/// the maker of the boards most rooms with underfloor heating stand on.
pub fn model_pipe(model: &Model, s: &Settings) -> Pipe {
    pipe(s, &model_maker(model))
}

/// The maker of the boards most rooms with underfloor heating stand on.
pub fn model_maker(model: &Model) -> String {
    let mut makers: Vec<(String, usize)> = vec![];
    for e in model.iter() {
        let ElementKind::Room(r) = &e.kind else {
            continue;
        };
        if r.ufh.is_none() {
            continue;
        }
        if let Some(f) = floor_build(model, r.level, r.at) {
            match makers.iter_mut().find(|m| m.0 == f.maker) {
                Some(m) => m.1 += 1,
                None => makers.push((f.maker, 1)),
            }
        }
    }
    makers
        .iter()
        .max_by_key(|m| m.1)
        .map_or(String::new(), |m| m.0.clone())
}

/// Underfloor heating for every room of the model, from its heat loss.
pub fn design(model: &Model, o: &heatloss::Opts, s: &Settings) -> Design {
    let placed: Vec<(ElementId, ElementId, glam::DVec2)> = model
        .iter()
        .filter_map(|e| {
            let ElementKind::Object(ob) = &e.kind else {
                return None;
            };
            fixture::family_of(model, e)
                .filter(|f| fixture::is_manifold(f))
                .map(|_| (e.id, ob.level, ob.to_plan(glam::DVec2::ZERO)))
        })
        .collect();
    let manifolds: Vec<(ElementId, glam::DVec2)> = placed.iter().map(|m| (m.1, m.2)).collect();
    let losses = heatloss::building(model, o);
    let rooms = heatloss::rooms_of(model);
    let pipe = model_pipe(model, s);
    let max_flow_ls = s
        .range
        .unwrap_or_else(|| Range::for_pipe(&model_maker(model)))
        .flowmeter_lmin()
        .map(|m| m / 60.0);
    let norm = crate::ufhnorm::Norm::of_model(model);
    let ways = feeds(model, &rooms);
    let mut out = Design {
        norm: norm.is_some(),
        no_manifold: manifolds.is_empty(),
        regulation: s.regulation,
        ..Design::default()
    };
    out.rooms = losses
        .rooms
        .iter()
        .filter_map(|hl| {
            one(
                model,
                s,
                o,
                hl,
                &manifolds,
                &rooms,
                &pipe,
                max_flow_ls,
                norm.as_ref(),
                &ways,
            )
        })
        .collect();
    open_spaces(model, &rooms, &mut out.rooms);
    for x in out.rooms.iter().filter(|x| x.on) {
        out.output_kw += x.covered / 1000.0;
        out.flow_ls += x.flow_ls * x.loops as f64;
        out.critical_kpa = out.critical_kpa.max(x.dp_kpa);
        out.loops += x.loops;
    }
    out.manifolds = manifold_designs(model, s, &out, &placed);
    // the worst manifold's index circuit, its valves wide open, and the
    // rest of the way to the source
    let worst = out
        .manifolds
        .iter()
        .map(|m| m.dp_kpa)
        .fold(out.critical_kpa, f64::max);
    out.head_kpa = (worst + MANIFOLD_KPA) * 1.1;
    out.pipe = Some(pipe);
    out
}

/// A manifold for each storey with underfloor heating: its loops and the
/// radiators picked on it make its circuits, the range is the board
/// maker's; the pipe from the source carries both flows.
fn manifold_designs(
    model: &Model,
    s: &Settings,
    d: &Design,
    placed: &[(ElementId, ElementId, glam::DVec2)],
) -> Vec<ManifoldDesign> {
    let maker = model_maker(model);
    let dist = distribution_type(&maker);
    let hs = crate::heating::HeatSettings {
        supply_c: s.radiator_supply_c,
        return_c: s.radiator_return_c,
        ..Default::default()
    };
    let rooms = heatloss::rooms_of(model);
    let mut levels: Vec<ElementId> = vec![];
    for x in d.rooms.iter().filter(|x| x.on) {
        let Some(ElementKind::Room(r)) = model.get(x.id).map(|e| &e.kind) else {
            continue;
        };
        if !levels.contains(&r.level) {
            levels.push(r.level);
        }
    }
    let mut out = vec![];
    for level in levels {
        let here = placed.iter().find(|m| m.1 == level);
        let at = here.map(|m| m.2);
        let mut loops = 0;
        let mut flow = 0.0;
        for x in d.rooms.iter().filter(|x| x.on) {
            if matches!(model.get(x.id).map(|e| &e.kind), Some(ElementKind::Room(r)) if r.level == level)
            {
                loops += x.loops;
                flow += x.flow_ls * x.loops as f64;
            }
        }
        let mut radiators = vec![];
        let dt = (s.radiator_supply_c - s.radiator_return_c).max(1.0);
        let fluid = crate::fluid::Fluid::water((s.radiator_supply_c + s.radiator_return_c) / 2.0);
        for &id in &s.radiators {
            let Some(e) = model.get(id) else { continue };
            let ElementKind::Object(ob) = &e.kind else {
                continue;
            };
            if ob.level != level {
                continue;
            }
            let w = crate::heating::radiator_w(model, id, &hs).unwrap_or(0.0);
            let q = w / (fluid.rho * 4180.0 * dt) * 1000.0;
            let p = ob.to_plan(glam::DVec2::ZERO);
            let length = at.map_or(0.0, |m| 2.0 * 1.2 * m.distance(p));
            let room = rooms
                .iter()
                .find(|x| {
                    x.level == level
                        && polygon::contains(
                            &x.shape.outline,
                            ob.to_plan(glam::DVec2::new(0.0, -0.2)),
                        )
                })
                .and_then(|x| match &x.e.kind {
                    ElementKind::Room(r) => Some(r.number.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            radiators.push(RadiatorFeed {
                id,
                room,
                output_w: w,
                length,
                pipe: insulated(
                    size_pipe(dist, q, hs.vmax_branch, hs.rmax, s.radiator_supply_c),
                    s.insulation,
                    s.radiator_supply_c,
                ),
            });
        }
        let rad_flow: f64 = radiators
            .iter()
            .filter_map(|r| r.pipe.as_ref())
            .map(|p| p.flow_ls)
            .sum();
        let config = ManifoldConfig {
            range: s.range.unwrap_or_else(|| Range::for_pipe(&maker)),
            loops,
            extra: radiators.len() as u32,
            cabinet: s.cabinet,
            mixing: s.mixing,
        };
        let total = flow + rad_flow;
        let (circuits, dp_kpa) = balance(model, d, level, &radiators, config.range);
        let temp = if rad_flow > 0.0 {
            s.radiator_supply_c
        } else {
            s.supply_c
        };
        out.push(ManifoldDesign {
            level,
            element: here.map(|m| m.0),
            config,
            flow_ls: total,
            radiators,
            feed: insulated(
                (total > 0.0)
                    .then(|| size_pipe(dist, total, hs.vmax_main, hs.rmax, temp))
                    .flatten(),
                s.insulation,
                temp,
            ),
            circuits,
            dp_kpa,
        });
    }
    out
}

/// A manifold's circuits balanced (STN EN 1264-4, the maker's sheet):
/// the index circuit, the worst with its valve wide open, sets the
/// manifold's loss; every other circuit's valve is preset to take what
/// its pipe does not (kv = Q / √(Δp / 100), HERZ's way), else, with no
/// kv from the maker, its flowmeter is set to its flow.
fn balance(
    model: &Model,
    d: &Design,
    level: ElementId,
    radiators: &[RadiatorFeed],
    range: Range,
) -> (Vec<Circuit>, f64) {
    let mut c: Vec<Circuit> = vec![];
    for x in d.rooms.iter().filter(|x| x.on) {
        if !matches!(model.get(x.id).map(|e| &e.kind), Some(ElementKind::Room(r)) if r.level == level)
        {
            continue;
        }
        for i in 0..x.loops {
            c.push(Circuit {
                label: if x.loops > 1 {
                    format!("{}/{}", x.number, i + 1)
                } else {
                    x.number.clone()
                },
                id: x.id,
                radiator: false,
                flow_ls: x.flow_ls,
                dp_pipe_kpa: x.dp_kpa,
                preset: None,
                kv: None,
                dp_valve_kpa: 0.0,
                over_flowmeter: false,
                index: false,
                under: false,
            });
        }
    }
    for r in radiators {
        let Some(p) = &r.pipe else { continue };
        c.push(Circuit {
            label: r.room.clone(),
            id: r.id,
            radiator: true,
            flow_ls: p.flow_ls,
            dp_pipe_kpa: p.r_pa_m * r.length * LOCAL_FACTOR / 1000.0,
            preset: None,
            kv: None,
            dp_valve_kpa: 0.0,
            over_flowmeter: false,
            index: false,
            under: false,
        });
    }
    let presets = range.presets();
    let open = presets.iter().map(|p| p.1).fold(0.0, f64::max);
    let wide = |x: &Circuit| {
        x.dp_pipe_kpa
            + if open > 0.0 {
                crate::herzvalves::dp_kpa(x.flow_ls * 3.6, open)
            } else {
                0.0
            }
    };
    let Some((ix, dp)) = c
        .iter()
        .enumerate()
        .map(|(i, x)| (i, wide(x)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
    else {
        return (c, 0.0);
    };
    let fm = range.flowmeter_lmin();
    for (i, x) in c.iter_mut().enumerate() {
        x.index = i == ix;
        x.over_flowmeter = fm.is_some_and(|m| x.flow_lmin() > m + 1e-9);
        if presets.is_empty() {
            continue;
        }
        // the presetting nearest the kv asked for (the flowmeter trims
        // what is left); wide open for the index circuit
        let q = x.flow_ls * 3.6;
        let need = (dp - x.dp_pipe_kpa).max(0.0);
        let kv_req = if need > 0.0 {
            q / (need / 100.0).sqrt()
        } else {
            f64::INFINITY
        };
        let min = presets.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
        let pick = if x.index || kv_req >= open {
            presets.iter().max_by(|a, b| a.1.total_cmp(&b.1))
        } else {
            presets.iter().min_by(|a, b| {
                (a.1 / kv_req)
                    .ln()
                    .abs()
                    .total_cmp(&(b.1 / kv_req).ln().abs())
            })
        };
        if let Some(&(preset, kv)) = pick {
            x.preset = Some(preset);
            x.kv = Some(kv);
            x.dp_valve_kpa = crate::herzvalves::dp_kpa(q, kv);
            x.under = kv_req < min * 0.95;
        }
    }
    (c, dp)
}

/// A line of the heating's bill of materials.
#[derive(Clone, Debug, PartialEq)]
pub struct BillLine {
    /// The element it is counted on (for picking it in the list).
    pub id: ElementId,
    pub item: String,
    pub code: String,
    pub size: String,
    pub quantity: f64,
    /// "m", "m²", "pcs".
    pub unit: &'static str,
}

/// How many coils of `coil` metres the loops take, none cut across two
/// coils (first fit, the longest loops first).
pub fn coils(loops: &[f64], coil: f64) -> usize {
    let mut v: Vec<f64> = loops.iter().copied().filter(|l| *l > 0.0).collect();
    v.sort_by(|a, b| b.total_cmp(a));
    let mut left: Vec<f64> = vec![];
    for l in v {
        match left.iter_mut().find(|r| **r + 1e-9 >= l) {
            Some(r) => *r -= l,
            None => left.push((coil - l).max(0.0)),
        }
    }
    left.len()
}

/// The underfloor heating's materials as designed: the system boards
/// under the rooms with it, the loops' pipe (and the coils it comes in),
/// the manifolds with their cabinets, the pipes to the radiators on them
/// and their insulation.
pub fn bill(d: &Design) -> Vec<BillLine> {
    let mut out: Vec<BillLine> = vec![];
    let mut add = |l: BillLine| match out
        .iter_mut()
        .find(|x| x.item == l.item && x.code == l.code && x.size == l.size && x.unit == l.unit)
    {
        Some(x) => x.quantity += l.quantity,
        None => out.push(l),
    };
    let on: Vec<&RoomDesign> = d.rooms.iter().filter(|x| x.on).collect();
    for x in &on {
        if let Some(b) = x.floor.as_ref().and_then(|f| f.board.clone()) {
            add(BillLine {
                id: x.id,
                item: b,
                code: String::new(),
                size: String::new(),
                quantity: x.area,
                unit: "m²",
            });
        }
    }
    if let Some(p) = &d.pipe {
        let loops: Vec<f64> = on
            .iter()
            .flat_map(|x| std::iter::repeat_n(x.loop_len, x.loops as usize))
            .collect();
        let total: f64 = loops.iter().sum();
        if let Some(x) = on.first() {
            if total > 0.0 {
                add(BillLine {
                    id: x.id,
                    item: p.name.clone(),
                    code: p.code.clone(),
                    size: p.segment.clone(),
                    quantity: total,
                    unit: "m",
                });
                let coil = pipe_types::by_id(&p.id)
                    .and_then(|t| t.closest_outer(p.outer_mm))
                    .and_then(|z| z.coil_m);
                if let Some(c) = coil {
                    add(BillLine {
                        id: x.id,
                        item: format!("{} – kotúč {c} m", p.name),
                        code: p.code.clone(),
                        size: p.segment.clone(),
                        quantity: coils(&loops, c) as f64,
                        unit: "pcs",
                    });
                }
            }
        }
    }
    // every loop's two ends and every radiator outlet's pipe screwed to
    // the manifold's G 3/4 Eurocone outlets by the maker's compression
    // fitting; no joint in the screed (STN EN 1264-4)
    for m in &d.manifolds {
        let id = m.element.unwrap_or(m.level);
        let range = m.config.range;
        let mut ends = |segment: &str, n: f64| {
            if n > 0.0 {
                let (item, code) = range.outlet_fitting(segment);
                add(BillLine {
                    id,
                    item,
                    code: code.into(),
                    size: "G 3/4".into(),
                    quantity: n,
                    unit: "pcs",
                });
            }
        };
        if let Some(p) = &d.pipe {
            ends(&p.segment, 2.0 * m.config.loops as f64);
        }
        for r in &m.radiators {
            if let Some(p) = &r.pipe {
                ends(&p.pipe.segment, 2.0);
            }
        }
    }
    for m in &d.manifolds {
        let id = m.element.unwrap_or(m.level);
        if let (Some(_), Some(code)) = (m.config.circuits(), m.config.code()) {
            add(BillLine {
                id,
                item: m.config.name(),
                code,
                size: String::new(),
                quantity: 1.0,
                unit: "pcs",
            });
        }
        if let Some(c) = m.config.cabinet_spec() {
            add(BillLine {
                id,
                item: format!(
                    "Skriňa rozdeľovača {} × {} × {} mm",
                    c.width, c.height, c.depth
                ),
                code: c.code,
                size: String::new(),
                quantity: 1.0,
                unit: "pcs",
            });
        }
        for r in &m.radiators {
            let Some(p) = &r.pipe else { continue };
            add(BillLine {
                id: r.id,
                item: p.pipe.name.clone(),
                code: p.pipe.code.clone(),
                size: p.pipe.segment.clone(),
                quantity: r.length,
                unit: "m",
            });
            if let Some(t) = &p.tube {
                add(BillLine {
                    id: r.id,
                    item: t.name.clone(),
                    code: t.code.clone(),
                    size: String::new(),
                    quantity: r.length,
                    unit: "m",
                });
            }
        }
    }
    accessories(d, &mut add);
    out
}

/// What the loops are laid and joined with, by the manifolds' maker:
/// the edge strip round the rooms, the joints in their doorways and the
/// sleeves over the pipes crossing them ([`Range::sleeve_m`]), the bend guides where the pipes rise to the manifold, its
/// ball valves and mixing set, and the regulation picked.
fn accessories(d: &Design, add: &mut impl FnMut(BillLine)) {
    let Some(m0) = d.manifolds.first() else {
        return;
    };
    let on: Vec<&RoomDesign> = d.rooms.iter().filter(|x| x.on).collect();
    let Some(first) = on.first() else { return };
    let range = m0.config.range;
    let outer = d.pipe.as_ref().map_or(PIPE_OUTER_MM, |p| p.outer_mm);
    let line = |id, item: &str, code: &str, quantity: f64, unit: &'static str| BillLine {
        id,
        item: item.into(),
        code: code.into(),
        size: String::new(),
        quantity,
        unit,
    };
    // the edge strip, m and its rolls
    let edge: f64 = on.iter().map(|x| x.perimeter).sum();
    if edge > 0.0 {
        let (name, code, roll) = range.edge_strip();
        add(line(first.id, name, code, edge, "m"));
        if let Some(r) = roll {
            add(line(
                first.id,
                &format!("{name} – kotúč {} m", r),
                code,
                (edge / r).ceil(),
                "pcs",
            ));
        }
    }
    // the doorways: each once, a profile piece per 2 m of its width
    let mut seen: Vec<ElementId> = vec![];
    let (pname, pcode, piece) = range.joint_profile();
    for x in &on {
        for w in &x.doors {
            if seen.contains(&w.id) {
                continue;
            }
            seen.push(w.id);
            add(line(w.id, pname, pcode, (w.width / piece).ceil(), "pcs"));
        }
    }
    // the sleeves: both pipes of every loop through its doorway
    let sleeve: f64 = on
        .iter()
        .map(|x| {
            let crossed = x
                .route
                .as_ref()
                .map_or(x.door.is_some() as usize, |w| w.doors.len());
            2.0 * x.loops as f64 * range.sleeve_m() * crossed as f64
        })
        .sum();
    if sleeve > 0.0 {
        let (name, code, roll) = range.sleeve(outer);
        add(line(first.id, name, code, sleeve, "m"));
        if let Some(r) = roll {
            add(line(
                first.id,
                &format!("{name} – kotúč {} m", r),
                code,
                (sleeve / r).ceil(),
                "pcs",
            ));
        }
    }
    for m in &d.manifolds {
        let id = m.element.unwrap_or(m.level);
        let r = m.config.range;
        let loops = m.config.loops as f64;
        if loops > 0.0 {
            let (name, code) = r.bend_guide(outer);
            add(line(id, name, code, 2.0 * loops, "pcs"));
        }
        for (name, code, n) in r.ball_valves() {
            add(line(id, name, code, n, "pcs"));
        }
        if m.config.mixing {
            match r {
                Range::HerzBrass => add(line(
                    id,
                    "HERZ Unimix V20 zmiešavacia zostava s čerpadlom G 1\"",
                    "1732002",
                    1.0,
                    "pcs",
                )),
                Range::HerzPush => {
                    add(line(
                        id,
                        "HERZ Unimix V20 zmiešavacia zostava s čerpadlom G 1\"",
                        "1732002",
                        1.0,
                        "pcs",
                    ));
                    add(line(
                        id,
                        "HERZ distančný kus Unimix – PUSH",
                        "2140812R",
                        1.0,
                        "pcs",
                    ));
                }
                _ => add(line(
                    id,
                    "Wavin zmiešavacia sada s čerpadlom Wilo Yonos RD25/6",
                    "XF159988W",
                    1.0,
                    "pcs",
                )),
            }
        }
        if d.regulation && loops > 0.0 {
            let (name, code) = r.actuator();
            add(line(id, name, code, loops, "pcs"));
            let rooms = on
                .iter()
                .filter(|x| m.circuits.iter().any(|c| !c.radiator && c.id == x.id))
                .count() as f64;
            let (tname, tcode) = r.thermostat();
            add(line(id, tname, tcode, rooms, "pcs"));
            let ((wname, wcode, zones), ext) = r.wiring();
            match ext {
                // one unit for the manifold, extensions for the loops past it
                Some((ename, ecode, more)) => {
                    add(line(id, wname, wcode, 1.0, "pcs"));
                    let past = (loops - zones as f64).max(0.0);
                    if past > 0.0 {
                        add(line(id, ename, ecode, (past / more as f64).ceil(), "pcs"));
                    }
                }
                None => add(line(
                    id,
                    wname,
                    wcode,
                    (rooms / zones as f64).ceil().max(1.0),
                    "pcs",
                )),
            }
        }
    }
}

/// The bill of the model's underfloor heating with the settings and
/// the heat loss options written with its results (defaults otherwise).
pub fn model_bill(model: &Model) -> Vec<BillLine> {
    // once per revision of the model: the list redraws every frame
    static LAST: std::sync::Mutex<Option<(u64, Vec<BillLine>)>> = std::sync::Mutex::new(None);
    let rev = model.revision();
    if rev != 0 {
        if let Some((r, b)) = LAST.lock().unwrap_or_else(|e| e.into_inner()).as_ref() {
            if *r == rev {
                return b.clone();
            }
        }
    }
    let b = model_bill_now(model);
    if rev != 0 {
        *LAST.lock().unwrap_or_else(|e| e.into_inner()) = Some((rev, b.clone()));
    }
    b
}

fn model_bill_now(model: &Model) -> Vec<BillLine> {
    let s: Settings = creabim_core::results::settings_of(model, "floorheat").unwrap_or_default();
    let o: heatloss::Opts =
        creabim_core::results::settings_of(model, "heatloss").unwrap_or_default();
    if !model
        .iter()
        .any(|e| matches!(&e.kind, ElementKind::Room(r) if r.ufh.is_some()))
    {
        return vec![];
    }
    let d = design(model, &o, &s);
    bill(&d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loops_are_cut_from_whole_coils() {
        assert_eq!(coils(&[90.0, 80.0, 60.0, 40.0, 20.0], 200.0), 2);
        assert_eq!(coils(&[110.0, 100.0, 95.0], 200.0), 2);
        assert_eq!(coils(&[], 200.0), 0);
    }

    #[test]
    fn the_characteristic_and_the_limit_flux() {
        let k = k_h(0.15, 0.1, 1.2, 0.016);
        assert!(k > 3.0 && k < 4.0, "{k}");
        assert!(k_h(0.1, 0.02, 1.2, 0.016) > k_h(0.3, 0.02, 1.2, 0.016));
        // a screed that conducts better gives more
        assert!(k_h(0.15, 0.02, 1.4, 0.016) > k_h(0.15, 0.02, 1.2, 0.016));
        assert!((d_theta_h(35.0, 30.0, 20.0) - 12.33).abs() < 0.05);
        assert!((q_limit(20.0, false) - 99.7).abs() < 0.5);
        assert!(q_limit(24.0, true) > 90.0 && q_limit(30.0, false) == 0.0);
        assert_eq!(finish_r("Keramická dlažba"), 0.02);
        assert_eq!(finish_r("Oak parquet"), 0.1);
        assert_eq!(finish_r("Koberec"), 0.15);
    }

    #[test]
    fn rooms_get_a_spacing_loops_and_a_flow() {
        use creabim_core::{Document, Element, Room, Transaction};
        use glam::DVec2;
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let mut tx = Transaction::new("rooms");
        for (name, number, at, on) in [
            ("Kúpeľňa", "1.01", DVec2::new(2.5, 3.0), true),
            ("Obývacia izba", "1.02", DVec2::new(8.0, 3.0), false),
        ] {
            let mut r = Room::new(level, at);
            r.number = number.into();
            r.floor_finish = "dlažba".into();
            if on {
                r.ufh = Some(Underfloor {
                    spacing: 0.15,
                    loops: 1,
                    ..Underfloor::default()
                });
            }
            tx = tx.insert(Element::new(name, ElementKind::Room(r)));
        }
        doc.commit(tx).unwrap();
        let d = design(
            doc.model(),
            &heatloss::Opts::default(),
            &Settings::default(),
        );
        assert_eq!(d.rooms.len(), 2);
        assert!(d.no_manifold);
        let (bath, living) = (&d.rooms[0], &d.rooms[1]);
        // the bathroom keeps its own spacing, the living room gets one
        assert!(bath.on && bath.manual && bath.spacing == 0.15);
        assert!(!living.on && SPACINGS.contains(&living.spacing));
        for x in &d.rooms {
            assert!(x.loop_len <= MAX_LOOP_M + 2.0 * x.feed + 1e-9, "{x:?}");
            assert!(x.loops >= 1 && x.flow_ls > 0.0 && x.dp_kpa > 0.0);
            assert!(x.q <= x.q_limit + 1e-9 && x.finish_r == 0.02);
            assert!((x.covered - x.area * x.q).abs() < 1e-9);
        }
        // 33 m² of bathroom at 150 mm: more than one loop of 100 m
        assert!(bath.loops >= 2);
        // only the rooms that have it count for the manifold
        assert_eq!(d.loops, bath.loops);
        assert!((d.flow_ls - bath.flow_ls * bath.loops as f64).abs() < 1e-12);
        // the manifold's circuits: the bathroom's loops, the index one
        // wide open (HERZ 8531 kv 1.5), the others preset to match
        let m = &d.manifolds[0];
        assert_eq!(m.circuits.len(), bath.loops as usize);
        assert_eq!(m.circuits.iter().filter(|c| c.index).count(), 1);
        let open = crate::herzvalves::dp_kpa(bath.flow_ls * 3.6, 1.5);
        assert!((m.dp_kpa - (bath.dp_kpa + open)).abs() < 1e-9);
        assert!(m.circuits.iter().all(|c| c.preset.is_some()));
        assert!((d.head_kpa - (m.dp_kpa + MANIFOLD_KPA) * 1.1).abs() < 1e-9);
        // the bill: the edge strip round the bathroom, ball valves
        let b = bill(&d);
        assert!(b
            .iter()
            .any(|l| l.code == "3F08002A" && (l.quantity - bath.perimeter).abs() < 1e-9));
        assert!(b.iter().any(|l| l.code == "1220513"));
        assert!(b
            .iter()
            .any(|l| l.code == "1609803" && l.quantity == 2.0 * bath.loops as f64));
    }
}
