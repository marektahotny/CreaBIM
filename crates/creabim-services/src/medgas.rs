//! Medical gas pipelines, as PilonDesigner's `MedGasAnalyzer`
//! (STN EN ISO 7396-1, diversity after HTM 02-01): each outlet counts in
//! its room's kind, a kind's n outlets take Q = Q1 + (n − 1)·Δ, the
//! pipes are copper (STN EN 13348) at most 10 m/s (vacuum 15 m/s), the
//! gas at its working pressure; the worst outlet may lose 20 kPa, no
//! more than the source keeps above the outlets' least, and its way is
//! enlarged until it does.

use crate::fluid::Fluid;
use crate::hydraulics;
use crate::tree::{self, Tree};
use creabim_core::fixture::ConnKind;
use creabim_core::{ElementId, ElementKind, Model};

/// The gases calculated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MedGas {
    O2,
    Air,
    Vac,
}

impl MedGas {
    pub const ALL: [MedGas; 3] = [MedGas::O2, MedGas::Air, MedGas::Vac];

    /// The system code its pipes are drawn in.
    pub fn code(self) -> &'static str {
        match self {
            MedGas::O2 => "O2",
            MedGas::Air => "AIR",
            MedGas::Vac => "VAC",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            MedGas::O2 => "Oxygen (O2)",
            MedGas::Air => "Medical air (AIR)",
            MedGas::Vac => "Vacuum (VAC)",
        }
    }
    fn conn(self) -> ConnKind {
        match self {
            MedGas::O2 => ConnKind::O2,
            MedGas::Air => ConnKind::AIR,
            MedGas::Vac => ConnKind::VAC,
        }
    }
    /// Molar mass, kg/mol, and dynamic viscosity, Pa·s.
    fn props(self) -> (f64, f64) {
        match self {
            MedGas::O2 => (0.031999, 2.04e-5),
            _ => (0.028966, 1.81e-5),
        }
    }
    /// The source's pressure (vacuum: below the atmosphere), the
    /// outlets' least, the loss allowed, kPa; the greatest velocity, m/s;
    /// the least inner diameter, mm.
    pub fn params(self) -> (f64, f64, f64, f64, f64) {
        match self {
            MedGas::Vac => (60.0, 40.0, 20.0, 15.0, 13.0),
            _ => (420.0, 370.0, 20.0, 10.0, 10.0),
        }
    }
}

/// A kind of room and its outlets' flows: Q1 and each further, l/min,
/// for O2, AIR and VAC (HTM 02-01).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoomKind {
    pub name: &'static str,
    pub words: &'static [&'static str],
    pub o2: (f64, f64),
    pub air: (f64, f64),
    pub vac: (f64, f64),
}

impl RoomKind {
    pub fn flows(&self, g: MedGas) -> (f64, f64) {
        match g {
            MedGas::O2 => self.o2,
            MedGas::Air => self.air,
            MedGas::Vac => self.vac,
        }
    }
}

pub const ROOMS: [RoomKind; 10] = [
    RoomKind {
        name: "Operating theatre",
        words: &["operacn", "op sala", "op. sala", "operating", "theatre"],
        o2: (100.0, 10.0),
        air: (40.0, 10.0),
        vac: (80.0, 40.0),
    },
    RoomKind {
        name: "Anaesthetic room",
        words: &["anestez", "uvod do anest", "indukc", "anaesthetic"],
        o2: (100.0, 0.0),
        air: (40.0, 0.0),
        vac: (40.0, 0.0),
    },
    RoomKind {
        name: "Recovery",
        words: &["dospav", "pooperac", "zotav", "recovery", "prebudzac"],
        o2: (10.0, 6.0),
        air: (40.0, 2.5),
        vac: (40.0, 10.0),
    },
    RoomKind {
        name: "Intensive care",
        words: &[
            "jis", "aro", "kaimst", "intenziv", "icu", "jip", "hdu", "resuscit",
        ],
        o2: (10.0, 4.5),
        air: (80.0, 40.0),
        vac: (40.0, 10.0),
    },
    RoomKind {
        name: "Emergency, resuscitation bed",
        words: &["urgent", "prijem", "emergency", "expektac"],
        o2: (100.0, 1.5),
        air: (40.0, 5.0),
        vac: (40.0, 10.0),
    },
    RoomKind {
        name: "Delivery room",
        words: &["porod", "delivery", "ldrp"],
        o2: (10.0, 1.5),
        air: (20.0, 2.5),
        vac: (40.0, 10.0),
    },
    RoomKind {
        name: "Neonatal",
        words: &["novoroden", "neonat", "inkubat", "nursery"],
        o2: (10.0, 6.0),
        air: (80.0, 40.0),
        vac: (40.0, 10.0),
    },
    RoomKind {
        name: "Dialysis",
        words: &["dialyz", "renal"],
        o2: (10.0, 1.5),
        air: (20.0, 2.5),
        vac: (40.0, 10.0),
    },
    RoomKind {
        name: "Examination / treatment room",
        words: &[
            "vysetr",
            "ambulanc",
            "zakrok",
            "osetrov",
            "ordinac",
            "treatment",
            "rtg",
            "endoskop",
        ],
        o2: (10.0, 0.6),
        air: (20.0, 2.5),
        vac: (40.0, 5.0),
    },
    RoomKind {
        name: "Ward",
        words: &["izba", "lozk", "pokoj", "ward", "bed"],
        o2: (10.0, 1.5),
        air: (20.0, 2.5),
        vac: (40.0, 10.0),
    },
];

/// A room of no known kind.
pub const OTHER: RoomKind = RoomKind {
    name: "Other room",
    words: &[],
    o2: (10.0, 1.5),
    air: (20.0, 2.5),
    vac: (40.0, 10.0),
};

/// The kind of a room named `name`.
pub fn room_kind(name: &str) -> usize {
    let n = crate::fixtures::normalize(name);
    ROOMS
        .iter()
        .position(|r| r.words.iter().any(|w| n.contains(w)))
        .unwrap_or(ROOMS.len())
}

fn kind_at(k: usize) -> &'static RoomKind {
    ROOMS.get(k).unwrap_or(&OTHER)
}

#[derive(Clone, Debug, PartialEq)]
pub struct OutletResult {
    pub id: ElementId,
    pub name: String,
    pub room: String,
    pub kind: &'static str,
    pub loss_pa: f64,
    pub critical: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PipeResult {
    pub id: ElementId,
    /// Design flow, l/min of free gas.
    pub flow_lmin: f64,
    pub outlets: usize,
    pub size: String,
    pub proposed: String,
    pub proposed_mm: f64,
    pub velocity_ms: f64,
    pub r_pa_m: f64,
    pub critical: bool,
    pub too_fast: bool,
}

/// The outlets of a kind of room on the system.
#[derive(Clone, Debug, PartialEq)]
pub struct KindRow {
    pub kind: &'static str,
    pub n: usize,
    pub q1: f64,
    pub add: f64,
    pub q: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MedResult {
    pub gas: MedGas,
    pub outlets: Vec<OutletResult>,
    pub pipes: Vec<PipeResult>,
    pub kinds: Vec<KindRow>,
    pub flow_lmin: f64,
    pub loss_pa: f64,
    pub limit_pa: f64,
    /// The source's pressure, and at the worst outlet, kPa.
    pub source_kpa: f64,
    pub outlet_kpa: f64,
    pub min_kpa: f64,
    pub rho: f64,
    pub enlarged: usize,
    pub unreached: usize,
}

/// Each medical gas drawn in the model, calculated.
pub fn run_all(model: &Model) -> Vec<MedResult> {
    MedGas::ALL
        .into_iter()
        .filter_map(|g| run(model, g))
        .collect()
}

/// Medical gas `g` of the model.
pub fn run(model: &Model, g: MedGas) -> Option<MedResult> {
    let t = tree::build_objects(model, g.code(), g.conn(), false)?;
    let rooms: Vec<(usize, String)> = t
        .terminals
        .iter()
        .map(|term| room_of(model, term.valve.pipe, term.at))
        .collect();
    Some(calc(&t, g, &rooms))
}

/// The room around `at`, on the level of pipe `pipe`: its kind and name.
fn room_of(model: &Model, pipe: ElementId, at: glam::DVec3) -> (usize, String) {
    let level = match model.get(pipe).map(|e| &e.kind) {
        Some(ElementKind::Pipe(p)) => p.level,
        _ => return (ROOMS.len(), String::new()),
    };
    for e in model.iter() {
        let ElementKind::Room(r) = &e.kind else {
            continue;
        };
        if r.level != level {
            continue;
        }
        let Some(shape) = creabim_core::room::shape_of(model, r) else {
            continue;
        };
        if creabim_core::polygon::contains(&shape.outline, at.truncate()) {
            let text = format!("{} {}", e.name, r.occupancy);
            return (room_kind(&text), e.name.clone());
        }
    }
    (ROOMS.len(), String::new())
}

fn calc(t: &Tree, g: MedGas, rooms: &[(usize, String)]) -> MedResult {
    let (source, min, max_loss, vmax, min_inner) = g.params();
    let atm = 101.325;
    let vac = g == MedGas::Vac;
    let mid = (source + min) / 2.0;
    let p_abs = if vac { atm - mid } else { atm + mid };
    let (mm, mu) = g.props();
    let rho = p_abs * 1000.0 * mm / (8.314462 * (20.0 + 273.15));
    let fluid = Fluid {
        temp_c: 20.0,
        rho,
        nu: mu / rho,
        cp: 1005.0,
    };
    let below = t.terminals_below();
    let n = t.stretches.len();
    let kinds_n = ROOMS.len() + 1;
    let design = |b: &[usize]| -> f64 {
        let mut cnt = vec![0usize; kinds_n];
        for &h in b {
            cnt[rooms[h].0] += 1;
        }
        cnt.iter()
            .enumerate()
            .filter(|(_, &c)| c > 0)
            .map(|(k, &c)| {
                let (q1, add) = kind_at(k).flows(g);
                q1 + (c - 1) as f64 * add
            })
            .sum()
    };
    let qn: Vec<f64> = below.iter().map(|b| design(b)).collect();
    let q: Vec<f64> = qn.iter().map(|&x| x / 60.0 * (atm / p_abs)).collect();
    let types: Vec<_> = t
        .stretches
        .iter()
        .map(|st| crate::pipe_types::by_id(&st.pipe.pipe_type))
        .collect();
    let sizes: Vec<Vec<(f64, f64, String)>> = types
        .iter()
        .map(|ty| {
            ty.map_or_else(Vec::new, |ty| {
                ty.sizes
                    .iter()
                    .filter(|z| z.inner_mm >= min_inner)
                    .map(|z| (z.nominal_mm, z.inner_mm, ty.size_label(z.nominal_mm, None)))
                    .collect()
            })
        })
        .collect();
    let kb: Vec<f64> = types
        .iter()
        .map(|t| t.map_or(0.0015, |t| t.roughness_mm) / 1000.0)
        .collect();
    let mut pick: Vec<Option<usize>> = (0..n)
        .map(|i| {
            sizes[i]
                .iter()
                .position(|z| hydraulics::velocity(q[i], z.1 / 1000.0) <= vmax)
                .or(sizes[i].len().checked_sub(1))
        })
        .collect();
    let inner = |i: usize, pick: &[Option<usize>]| {
        pick[i].map_or(t.stretches[i].pipe.inner_mm, |k| sizes[i][k].1)
    };
    let even = |pick: &mut Vec<Option<usize>>| {
        for _ in 0..4 {
            for i in (0..n).rev() {
                let Some(p) = t.stretches[i].parent else {
                    continue;
                };
                let need = inner(i, pick);
                if inner(p, pick) + 1e-6 < need {
                    if let Some(k) = sizes[p].iter().position(|z| z.1 + 1e-6 >= need) {
                        pick[p] = Some(k);
                    }
                }
            }
            for (_, parts) in t.by_pipe() {
                if let Some(big) = parts
                    .iter()
                    .copied()
                    .max_by(|&a, &b| inner(a, pick).total_cmp(&inner(b, pick)))
                {
                    for &p in &parts {
                        pick[p] = pick[big];
                    }
                }
            }
        }
    };
    even(&mut pick);
    let losses = |pick: &[Option<usize>]| -> Vec<(f64, f64, f64)> {
        (0..n)
            .map(|i| {
                let d = inner(i, pick) / 1000.0;
                let r = hydraulics::r(q[i], d, kb[i], &fluid);
                (
                    r,
                    hydraulics::velocity(q[i], d),
                    r * t.stretches[i].length() * 1.3,
                )
            })
            .collect()
    };
    let way = |h: usize| {
        let mut v = vec![];
        let mut at = t.stretches.iter().position(|st| st.terminals.contains(&h));
        while let Some(i) = at {
            v.push(i);
            at = t.stretches[i].parent;
        }
        v
    };
    let worst = |ls: &[(f64, f64, f64)]| {
        (0..t.terminals.len())
            .map(|h| (h, way(h).iter().map(|&i| ls[i].2).sum::<f64>()))
            .max_by(|a, b| a.1.total_cmp(&b.1))
    };
    let margin = (source - min).abs() * 1000.0;
    let limit = (max_loss * 1000.0).min(if margin > 0.0 { margin } else { f64::INFINITY });
    let mut ls = losses(&pick);
    let mut w = worst(&ls);
    let mut enlarged = 0;
    while let Some((h, req)) = w {
        if req <= limit || enlarged >= 300 {
            break;
        }
        let path: Vec<usize> = way(h)
            .into_iter()
            .filter(|&i| pick[i].is_some_and(|k| k + 1 < sizes[i].len()))
            .collect();
        if path.is_empty() {
            break;
        }
        let top = path.iter().map(|&i| ls[i].0).fold(0.0, f64::max);
        for &i in &path {
            if ls[i].0 >= 0.8 * top {
                pick[i] = pick[i].map(|k| k + 1);
            }
        }
        even(&mut pick);
        ls = losses(&pick);
        w = worst(&ls);
        enlarged += 1;
    }
    let crit_way = w.map_or_else(Vec::new, |(h, _)| way(h));
    let loss = w.map_or(0.0, |x| x.1);
    let mut out = MedResult {
        gas: g,
        outlets: vec![],
        pipes: vec![],
        kinds: vec![],
        flow_lmin: (0..n)
            .filter(|&i| t.stretches[i].parent.is_none())
            .map(|i| qn[i])
            .sum(),
        loss_pa: loss,
        limit_pa: limit,
        source_kpa: source,
        outlet_kpa: source - loss / 1000.0,
        min_kpa: min,
        rho,
        enlarged,
        unreached: t.unreached.len(),
    };
    for (h, term) in t.terminals.iter().enumerate() {
        out.outlets.push(OutletResult {
            id: term.id,
            name: term.name.clone(),
            room: rooms[h].1.clone(),
            kind: kind_at(rooms[h].0).name,
            loss_pa: way(h).iter().map(|&i| ls[i].2).sum(),
            critical: w.is_some_and(|x| x.0 == h),
        });
    }
    for k in 0..kinds_n {
        let c = rooms.iter().filter(|r| r.0 == k).count();
        if c == 0 {
            continue;
        }
        let (q1, add) = kind_at(k).flows(g);
        out.kinds.push(KindRow {
            kind: kind_at(k).name,
            n: c,
            q1,
            add,
            q: q1 + (c - 1) as f64 * add,
        });
    }
    for (id, parts) in t.by_pipe() {
        let big = parts
            .iter()
            .copied()
            .max_by(|&a, &b| q[a].total_cmp(&q[b]))
            .unwrap_or(parts[0]);
        let st = &t.stretches[big];
        let size = types[big].map_or_else(
            || format!("{}", st.pipe.nominal_mm),
            |t| t.size_label(st.pipe.nominal_mm, None),
        );
        let (proposed, proposed_mm) = pick[big].map_or((size.clone(), st.pipe.nominal_mm), |k| {
            (sizes[big][k].2.clone(), sizes[big][k].0)
        });
        out.pipes.push(PipeResult {
            id,
            flow_lmin: qn[big],
            outlets: below[big].len(),
            size,
            proposed,
            proposed_mm,
            velocity_ms: ls[big].1,
            r_pa_m: ls[big].0,
            critical: parts.iter().any(|p| crit_way.contains(p)),
            too_fast: ls[big].1 > vmax + 1e-6,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::mep::{Pipe, Valve};
    use creabim_core::{Document, Element, Transaction};
    use glam::DVec3;

    #[test]
    fn rooms_are_known_by_their_names() {
        assert_eq!(
            kind_at(room_kind("Operačná sála 2")).name,
            "Operating theatre"
        );
        assert_eq!(kind_at(room_kind("JIS – lôžko")).name, "Intensive care");
        assert_eq!(kind_at(room_kind("Sklad")).name, "Other room");
    }

    #[test]
    fn four_ward_outlets_take_q1_and_three_increments() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let ty = crate::pipe_types::by_id("CUMED").unwrap();
        let z = ty.sizes[ty.sizes.len() / 2].clone();
        let pipe = |a: [f64; 3], b: [f64; 3]| {
            Element::new(
                "pipe",
                ElementKind::Pipe(Pipe {
                    level,
                    system: "O2".into(),
                    start: DVec3::from(a),
                    end: DVec3::from(b),
                    pipe_type: ty.id.clone(),
                    nominal_mm: z.nominal_mm,
                    outer_mm: z.outer_mm,
                    inner_mm: z.inner_mm,
                    height_mm: None,
                    insulation_mm: 0.0,
                    insulation_set: false,
                    mark: None,
                }),
            )
        };
        let mut tx = Transaction::new("o2").insert(pipe([0.0, 0.0, 0.0], [0.0, 0.0, 2.6]));
        let mut at = [0.0, 0.0, 2.6];
        for k in 0..4 {
            let x = 3.0 * (k + 1) as f64;
            let main = pipe(at, [x, 0.0, 2.6]);
            let drop = pipe([x, 0.0, 2.6], [x, 1.0, 1.4]);
            let id = drop.id;
            tx = tx.insert(main).insert(drop).insert(Element::new(
                format!("V{k}"),
                ElementKind::Valve(Valve::new(id, 1.0, "vyvod")),
            ));
            at = [x, 0.0, 2.6];
        }
        doc.commit(tx).unwrap();
        // no rooms drawn: other rooms, 10 + 3·1,5 l/min of O2
        let r = run(doc.model(), MedGas::O2).expect("oxygen");
        assert_eq!(r.outlets.len(), 4, "{r:?}");
        assert!((r.flow_lmin - 14.5).abs() < 1e-9, "{}", r.flow_lmin);
        assert_eq!(r.kinds.len(), 1);
        assert!(r.loss_pa <= r.limit_pa && r.outlet_kpa >= r.min_kpa);
        assert!(r.pipes.iter().all(|p| !p.too_fast));
        assert!(r.outlets.iter().filter(|o| o.critical).count() == 1);
        assert!(run(doc.model(), MedGas::Vac).is_none());
    }
}
