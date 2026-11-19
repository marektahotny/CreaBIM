//! Underfloor heating, system A (the pipe in the screed) after STN EN
//! 1264-2, as PilonDesigner's `floorheat.ts`: the simplified
//! characteristic with the spacing T, the floor finish's resistance RλB
//! and 45 mm of screed over the pipe; the spacing chosen for the room's
//! heat loss (STN EN 12831-1); the limit heat flux
//! qG = 8.92 · (θF,max − θi)^1.1; loops of 100 m at most; the flow, the
//! loops' pressure losses and what to throttle at the manifold.

use crate::heatloss::{self, RoomLoss};
use creabim_core::fixture;
use creabim_core::underfloor::Underfloor;
use creabim_core::{ElementId, ElementKind, Model};

/// What the system is designed with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// Supply temperature, °C, and the drop σ, K.
    pub supply_c: f64,
    pub sigma_k: f64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            supply_c: 35.0,
            sigma_k: 5.0,
        }
    }
}

/// The spacings pipes are laid at, m.
pub const SPACINGS: [f64; 5] = [0.1, 0.15, 0.2, 0.25, 0.3];
/// PE-Xa / PE-RT 16 × 2.
pub const PIPE_OUTER_MM: f64 = 16.0;
pub const PIPE_INNER_MM: f64 = 12.0;
const ROUGHNESS_MM: f64 = 0.007;
pub const MAX_LOOP_M: f64 = 100.0;
/// The share lost downwards (insulation to STN EN 1264-4).
const DOWN_FACTOR: f64 = 1.1;
/// A loop's local losses (bends, the manifold).
const LOCAL_FACTOR: f64 = 1.3;
/// The manifold's, valves' and source's loss on the pump, kPa.
pub const MANIFOLD_KPA: f64 = 5.0;
/// The floor laid with pipes (edge strips, furniture).
const AREA_FACTOR: f64 = 0.9;

/// RλB of a floor finish, m²K/W, from its description (STN EN 1264-2:
/// usually 0.0 to 0.15).
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
        0.1
    }
}

/// The equivalent coefficient KH, W/(m²K): q = KH · ΔθH (STN EN 1264-2,
/// system A, su = 45 mm, λE = 1.2 W/(mK), d = 16 mm).
pub fn k_h(t: f64, r: f64) -> f64 {
    let a_b = (1.0 / 10.8 + 0.045 / 1.0) / (1.0 / 10.8 + 0.045 / 1.2 + r);
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
    let (a_d, m_d): (f64, f64) = (1.03, 250.0 * (PIPE_OUTER_MM / 1000.0 - 0.02));
    6.7 * a_b * a_t.powf(m_t) * a_d.powf(m_d)
}

/// The logarithmic temperature difference ΔθH.
pub fn d_theta_h(supply: f64, ret: f64, ti: f64) -> f64 {
    (supply - ret) / ((supply - ti) / (ret - ti)).ln()
}

/// The limit heat flux qG, W/m² (θF,max 29 °C where people stay, 33 °C
/// in a bathroom).
pub fn q_limit(ti: f64, bath: bool) -> f64 {
    8.92 * (if bath { 33.0 } else { 29.0 } - ti).max(0.0).powf(1.1)
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
    /// The spacing, m; `manual` when the room's own was kept.
    pub spacing: f64,
    pub manual: bool,
    pub finish_r: f64,
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
    /// The room has it in the model.
    pub on: bool,
}

impl RoomDesign {
    pub fn underfloor(&self, own_r: Option<f64>) -> Underfloor {
        Underfloor {
            spacing: self.spacing,
            loops: self.loops,
            finish_r: own_r,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Design {
    /// Every room, those with underfloor heating marked `on`.
    pub rooms: Vec<RoomDesign>,
    /// Of the rooms that have it: the output, kW; the manifold's flow,
    /// l/s; the worst loop's loss and the pump's head, kPa.
    pub output_kw: f64,
    pub flow_ls: f64,
    pub critical_kpa: f64,
    pub head_kpa: f64,
    pub loops: u32,
    /// No manifold stands in the model.
    pub no_manifold: bool,
}

/// A loop's pressure loss, kPa, at `flow_ls` over `len` metres.
fn loop_loss(flow_ls: f64, len: f64) -> f64 {
    let d = PIPE_INNER_MM / 1000.0;
    let v = flow_ls / 1000.0 / (std::f64::consts::PI * d * d / 4.0);
    if v <= 0.0 {
        return 0.0;
    }
    let re = v * d / 0.75e-6;
    let mut lam: f64 = 0.03;
    for _ in 0..30 {
        lam = (-2.0 * (2.51 / (re * lam.sqrt()) + ROUGHNESS_MM / 1000.0 / (3.71 * d)).log10())
            .powi(-2);
    }
    if re < 2300.0 {
        lam = 64.0 / re.max(1.0);
    }
    lam / d * 994.0 * v * v / 2.0 * len * LOCAL_FACTOR / 1000.0
}

fn one(
    model: &Model,
    s: &Settings,
    hl: &RoomLoss,
    manifolds: &[(ElementId, glam::DVec2)],
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
    let rf = own
        .and_then(|u| u.finish_r)
        .unwrap_or_else(|| finish_r(&r.floor_finish));
    let dt = d_theta_h(s.supply_c, ret, ti);
    let qg = q_limit(ti, bath);
    let q_req = if area > 0.0 { hl.phi / area } else { 0.0 };
    // the widest spacing that covers the loss; the room's own is kept
    let spacing = own.map(|u| u.spacing).unwrap_or_else(|| {
        SPACINGS
            .iter()
            .rev()
            .copied()
            .find(|t| k_h(*t, rf) * dt >= q_req)
            .unwrap_or(SPACINGS[0])
    });
    let q = (k_h(spacing, rf) * dt).min(qg);
    // from the storey's manifold to the room, as the crow flies × 1.2
    let feed = manifolds
        .iter()
        .find(|m| m.0 == r.level)
        .map_or(0.0, |m| m.1.distance(r.at) * 1.2);
    let coil = area / spacing;
    let loops = ((coil + 2.0 * feed) / MAX_LOOP_M).ceil().max(1.0) as u32;
    let loop_len = coil / loops as f64 + 2.0 * feed;
    let flow_ls = area * q * DOWN_FACTOR / (4180.0 * s.sigma_k) / loops as f64;
    let covered = area * q;
    Some(RoomDesign {
        id: hl.id,
        number: hl.number.clone(),
        name: hl.name.clone(),
        ti,
        phi: hl.phi,
        area,
        q_req,
        q,
        q_limit: qg,
        spacing,
        manual: own.is_some(),
        finish_r: rf,
        covered,
        loops,
        loop_len,
        feed,
        flow_ls,
        dp_kpa: loop_loss(flow_ls, loop_len),
        ok: covered >= hl.phi * 0.98,
        on: own.is_some(),
    })
}

/// Underfloor heating for every room of the model, from its heat loss.
pub fn design(model: &Model, o: &heatloss::Opts, s: &Settings) -> Design {
    let manifolds: Vec<(ElementId, glam::DVec2)> = model
        .iter()
        .filter_map(|e| {
            let ElementKind::Object(ob) = &e.kind else {
                return None;
            };
            fixture::family_of(model, e)
                .filter(|f| fixture::is_manifold(f))
                .map(|_| (ob.level, ob.to_plan(glam::DVec2::ZERO)))
        })
        .collect();
    let losses = heatloss::building(model, o);
    let mut out = Design {
        no_manifold: manifolds.is_empty(),
        ..Design::default()
    };
    out.rooms = losses
        .rooms
        .iter()
        .filter_map(|hl| one(model, s, hl, &manifolds))
        .collect();
    for x in out.rooms.iter().filter(|x| x.on) {
        out.output_kw += x.covered / 1000.0;
        out.flow_ls += x.flow_ls * x.loops as f64;
        out.critical_kpa = out.critical_kpa.max(x.dp_kpa);
        out.loops += x.loops;
    }
    out.head_kpa = (out.critical_kpa + MANIFOLD_KPA) * 1.1;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_characteristic_and_the_limit_flux() {
        let k = k_h(0.15, 0.1);
        assert!(k > 3.0 && k < 4.0, "{k}");
        assert!(k_h(0.1, 0.02) > k_h(0.3, 0.02));
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
                    finish_r: None,
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
        assert!((d.head_kpa - (bath.dp_kpa + MANIFOLD_KPA) * 1.1).abs() < 1e-9);
    }
}
