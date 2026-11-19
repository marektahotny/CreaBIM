//! Thermal expansion of pipe runs and the pipe supports they need, from
//! the makers' manuals (Wavin "Vnitřní instalace", 2026; HERZ
//! "Plasthliníková rúrka"):
//!
//! - a run is the straight line of pipes of one system and type between
//!   two turns, tees or ends; its change of length is Δl = α·L·Δt;
//! - the free arm a turn needs to take it up is Ls = k·√(D·Δl) (HERZ's
//!   bending arm BS with its c), where the run turns into a pipe at least
//!   that long the turn takes it; otherwise a U compensator of width
//!   Lk = 2·Δl + 150 mm, at least 10·D, or a fixed point halfway, which
//!   halves Δl;
//! - supports at the maker's greatest spacing for the size (and the
//!   water's temperature for PP-R), closer on a riser where the maker
//!   says so; counted for the bill of materials.
//!
//! Drainage of socketed pipe (AS+, SiTech+) takes its expansion in the
//! sockets and gets supports only.

use crate::pipe_types;
use creabim_core::mep::Pipe;
use creabim_core::{ElementId, ElementKind, Model};
use glam::DVec3;

/// A maker's data for a kind of pipe.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    /// What the manual calls it.
    pub source: &'static str,
    /// Coefficient of linear expansion, mm/(m·K); none where the joints
    /// take it.
    pub alpha: Option<f64>,
    /// The material constant of the free arm.
    pub k: f64,
    /// Greatest support spacing, horizontal and on a riser, m, by the
    /// outside diameter up to which it holds, mm.
    pub spacing: &'static [(f64, f64, f64)],
    /// PP-R's spacing falls with the water's temperature: its columns
    /// (20, 30, 40, 50, 60, 80 °C), cm, by diameter; empty otherwise.
    pub by_temp: &'static [(f64, [f64; 6])],
}

const PPR_S32: &[(f64, [f64; 6])] = &[
    (16.0, [80.0, 75.0, 75.0, 70.0, 70.0, 60.0]),
    (20.0, [90.0, 80.0, 80.0, 80.0, 70.0, 65.0]),
    (25.0, [95.0, 95.0, 95.0, 90.0, 80.0, 75.0]),
    (32.0, [110.0, 105.0, 105.0, 100.0, 95.0, 80.0]),
    (40.0, [120.0, 120.0, 115.0, 105.0, 100.0, 95.0]),
    (50.0, [135.0, 130.0, 125.0, 120.0, 115.0, 100.0]),
    (63.0, [155.0, 150.0, 145.0, 135.0, 130.0, 115.0]),
    (75.0, [170.0, 165.0, 160.0, 150.0, 145.0, 125.0]),
    (90.0, [180.0, 180.0, 170.0, 165.0, 160.0, 135.0]),
    (110.0, [200.0, 195.0, 190.0, 180.0, 175.0, 155.0]),
    (125.0, [220.0, 215.0, 200.0, 195.0, 190.0, 165.0]),
    (160.0, [220.0, 210.0, 200.0, 195.0, 185.0, 185.0]),
    (200.0, [245.0, 235.0, 225.0, 220.0, 210.0, 210.0]),
    (250.0, [275.0, 265.0, 250.0, 245.0, 235.0, 235.0]),
];

const PPRCT_EVO: &[(f64, [f64; 6])] = &[
    (16.0, [80.0, 75.0, 75.0, 70.0, 70.0, 60.0]),
    (20.0, [85.0, 80.0, 75.0, 75.0, 70.0, 65.0]),
    (25.0, [90.0, 90.0, 90.0, 85.0, 80.0, 75.0]),
    (32.0, [105.0, 100.0, 100.0, 95.0, 90.0, 80.0]),
    (40.0, [115.0, 115.0, 110.0, 105.0, 100.0, 90.0]),
    (50.0, [130.0, 125.0, 120.0, 115.0, 110.0, 95.0]),
    (63.0, [145.0, 140.0, 135.0, 130.0, 125.0, 110.0]),
    (75.0, [160.0, 155.0, 150.0, 140.0, 135.0, 120.0]),
    (90.0, [170.0, 170.0, 160.0, 155.0, 150.0, 130.0]),
    (110.0, [190.0, 185.0, 180.0, 170.0, 165.0, 145.0]),
    (125.0, [205.0, 200.0, 190.0, 185.0, 180.0, 160.0]),
];

/// Wavin PP-R S 3,2: all plastic.
pub const WAVIN_PPR: Material = Material {
    source: "Wavin PP-R/PP-RCT",
    alpha: Some(0.12),
    k: 20.0,
    spacing: &[],
    by_temp: PPR_S32,
};

/// Wavin PP-RCT EVO: all plastic.
pub const WAVIN_EVO: Material = Material {
    source: "Wavin PP-RCT EVO",
    alpha: Some(0.12),
    k: 20.0,
    spacing: &[],
    by_temp: PPRCT_EVO,
};

/// Wavin PP-RCT Basalt: reinforced, whatever the water's temperature.
pub const WAVIN_BASALT: Material = Material {
    source: "Wavin PP-RCT Basalt",
    alpha: Some(0.05),
    k: 20.0,
    spacing: &[
        (20.0, 0.90, 1.17),
        (25.0, 1.10, 1.43),
        (32.0, 1.20, 1.56),
        (40.0, 1.30, 1.69),
        (50.0, 1.40, 1.82),
        (63.0, 1.60, 2.08),
        (75.0, 1.65, 2.15),
        (90.0, 1.80, 2.34),
        (110.0, 1.90, 2.47),
        (125.0, 2.00, 2.60),
    ],
    by_temp: &[],
};

/// Wavin Tigris: multilayer, press fittings.
pub const WAVIN_TIGRIS: Material = Material {
    source: "Wavin Tigris K5/M5/K1/M1",
    alpha: Some(0.025),
    k: 30.0,
    spacing: &[
        (16.0, 1.0, 1.0),
        (20.0, 1.2, 1.2),
        (32.0, 1.4, 1.4),
        (50.0, 1.8, 1.8),
        (63.0, 2.0, 2.0),
        (75.0, 2.2, 2.2),
    ],
    by_temp: &[],
};

/// HERZ PipeFix and GASPRESS: plastic-aluminium.
pub const HERZ_PIPE: Material = Material {
    source: "HERZ plasthliníková rúrka",
    alpha: Some(0.024),
    k: 33.0,
    spacing: &[
        (16.0, 0.8, 0.8),
        (20.0, 1.0, 1.0),
        (26.0, 1.2, 1.2),
        (32.0, 1.6, 1.6),
        (40.0, 1.7, 1.7),
        (50.0, 1.8, 1.8),
        (63.0, 2.0, 2.0),
    ],
    by_temp: &[],
};

/// Wavin HDPE drainage, welded: horizontal without a support trough.
pub const WAVIN_HDPE: Material = Material {
    source: "Wavin HDPE",
    alpha: Some(0.2),
    k: 10.0,
    spacing: &[
        (50.0, 0.6, 0.9),
        (63.0, 0.8, 0.9),
        (75.0, 0.8, 1.2),
        (90.0, 0.9, 1.4),
        (110.0, 1.1, 1.7),
        (125.0, 1.3, 1.9),
        (160.0, 1.6, 2.4),
        (315.0, 2.0, 3.0),
    ],
    by_temp: &[],
};

/// Wavin AS+: horizontal 15·D, on a riser at most 2 m.
pub const WAVIN_AS: Material = Material {
    source: "Wavin AS+",
    alpha: None,
    k: 0.0,
    spacing: &[
        (50.0, 0.75, 1.25),
        (75.0, 1.125, 1.875),
        (90.0, 1.35, 2.0),
        (110.0, 1.5, 2.0),
        (125.0, 1.625, 2.0),
        (160.0, 2.0, 2.0),
        (200.0, 2.15, 2.0),
    ],
    by_temp: &[],
};

/// Wavin SiTech+.
pub const WAVIN_SITECH: Material = Material {
    source: "Wavin SiTech+",
    alpha: None,
    k: 0.0,
    spacing: &[
        (32.0, 0.5, 1.5),
        (40.0, 0.6, 1.5),
        (50.0, 0.75, 1.5),
        (75.0, 1.1, 2.0),
        (90.0, 1.35, 2.0),
        (110.0, 1.65, 2.0),
        (125.0, 1.85, 2.0),
        (160.0, 2.4, 2.0),
    ],
    by_temp: &[],
};

/// The maker's data for a pipe type, where the manuals give it.
pub fn material(pipe_type: &str) -> Option<Material> {
    Some(match pipe_type {
        "WPPR" | "PPR" => WAVIN_PPR,
        "WPPRCT" => WAVIN_EVO,
        "WPPRB" => WAVIN_BASALT,
        "WTIGK" | "WTIGM" => WAVIN_TIGRIS,
        "HERZ" | "HERZT" | "HERZG" | "HERZGT" => HERZ_PIPE,
        "WHDPE" => WAVIN_HDPE,
        "WAS" => WAVIN_AS,
        "WSI" => WAVIN_SITECH,
        _ => return None,
    })
}

/// Temperatures the runs are worked out with, °C.
#[derive(Clone, Debug, PartialEq)]
pub struct Temps {
    /// When it is laid.
    pub install: f64,
    pub cold: f64,
    pub hot: f64,
    pub heating_supply: f64,
    pub heating_return: f64,
    pub cooling: f64,
    pub drainage: f64,
}

impl Default for Temps {
    fn default() -> Self {
        Temps {
            install: 20.0,
            cold: 10.0,
            hot: 60.0,
            heating_supply: 70.0,
            heating_return: 55.0,
            cooling: 6.0,
            drainage: 50.0,
        }
    }
}

impl Temps {
    /// The temperature a system runs at.
    pub fn of(&self, system: &str) -> f64 {
        match system {
            "SV" | "PV" | "H" => self.cold,
            "TV" | "C" | "CTV" => self.hot,
            "UKP" => self.heating_supply,
            "UKS" => self.heating_return,
            "CHP" | "CHS" | "CH" => self.cooling,
            s if s.starts_with('K') || s == "D" => self.drainage,
            _ => self.install,
        }
    }
}

/// The greatest support spacing for a pipe, m.
pub fn spacing(m: &Material, outer_mm: f64, temp: f64, riser: bool) -> Option<f64> {
    if !m.by_temp.is_empty() {
        let row = m
            .by_temp
            .iter()
            .find(|r| outer_mm <= r.0 + 0.5)
            .or(m.by_temp.last())?;
        let cols = [20.0, 30.0, 40.0, 50.0, 60.0, 80.0];
        let i = cols.iter().position(|c| temp <= *c + 0.5).unwrap_or(5);
        let h = row.1[i] / 100.0;
        // a riser's spacing 1,3 times the horizontal
        return Some(if riser { h * 1.3 } else { h });
    }
    let row = m
        .spacing
        .iter()
        .find(|r| outer_mm <= r.0 + 0.5)
        .or(m.spacing.last())?;
    Some(if riser { row.2 } else { row.1 })
}

/// A straight run of pipes.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    pub pipes: Vec<ElementId>,
    pub system: String,
    pub pipe_type: String,
    pub outer_mm: f64,
    pub length: f64,
    pub riser: bool,
    pub source: &'static str,
    /// The temperatures' difference, K, and the change of length, mm.
    pub dt: f64,
    pub dl: Option<f64>,
    /// The free arm needed, mm, and the shorter arm the run turns into
    /// at its ends, mm (none at a free end or a tee).
    pub arm_needed: Option<f64>,
    pub arm_there: Option<f64>,
    /// The U compensator's width, mm, where the arms are too short.
    pub compensator: Option<f64>,
    /// Support spacing, m, and supports along the run.
    pub spacing: Option<f64>,
    pub supports: usize,
}

impl Run {
    /// Do its turns take up its expansion?
    pub fn arms_ok(&self) -> bool {
        match (self.arm_needed, self.arm_there) {
            (Some(n), Some(t)) => t >= n,
            (Some(_), None) => false,
            _ => true,
        }
    }
}

struct Seg<'a> {
    id: ElementId,
    pipe: &'a Pipe,
    a: DVec3,
    b: DVec3,
}

fn key(p: DVec3) -> [i64; 3] {
    crate::fittings::key(p)
}

/// The runs of the model with their expansion and supports.
pub fn runs(model: &Model, temps: &Temps) -> Vec<Run> {
    let segs: Vec<Seg> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Pipe(p) => {
                let (a, b) = crate::fittings::ends(model, p);
                (a.distance(b) > 1e-6).then_some(Seg {
                    id: e.id,
                    pipe: p,
                    a,
                    b,
                })
            }
            _ => None,
        })
        .collect();
    let mut at: std::collections::HashMap<[i64; 3], Vec<usize>> = Default::default();
    for (i, s) in segs.iter().enumerate() {
        at.entry(key(s.a)).or_default().push(i);
        at.entry(key(s.b)).or_default().push(i);
    }
    let dir = |s: &Seg| (s.b - s.a).normalize();
    // the one pipe going straight on from `i` at `node`, of its system
    // and type, where only the two meet
    let next = |i: usize, node: DVec3| -> Option<usize> {
        let list = at.get(&key(node))?;
        if list.len() != 2 {
            return None;
        }
        let j = *list.iter().find(|&&j| j != i)?;
        let (s, t) = (&segs[i], &segs[j]);
        (s.pipe.system == t.pipe.system
            && s.pipe.pipe_type == t.pipe.pipe_type
            && dir(s).cross(dir(t)).length() < 0.02)
            .then_some(j)
    };
    let mut used = vec![false; segs.len()];
    let mut out = vec![];
    for start in 0..segs.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let mut chain = vec![start];
        // walk both ways
        let mut ends = [segs[start].a, segs[start].b];
        for (side, end) in ends.iter_mut().enumerate() {
            let mut cur = start;
            while let Some(j) = next(cur, *end) {
                if used[j] {
                    break;
                }
                used[j] = true;
                if side == 0 {
                    chain.insert(0, j);
                } else {
                    chain.push(j);
                }
                let s = &segs[j];
                *end = if key(s.a) == key(*end) { s.b } else { s.a };
                cur = j;
            }
        }
        let first = &segs[start];
        let p = first.pipe;
        let Some(m) = material(&p.pipe_type) else {
            continue;
        };
        let length = ends[0].distance(ends[1]);
        let riser = (ends[1] - ends[0]).normalize().z.abs() > 0.9;
        let temp = temps.of(&p.system);
        let dt = (temp - temps.install).abs();
        let dl = m.alpha.map(|a| a * length * dt);
        let arm_needed = dl.map(|dl| m.k * (p.outer_mm * dl).sqrt());
        // the arms the run turns into: the pipe at each end that leaves
        // at an angle (an elbow), the shorter of the two
        let run_dir = (ends[1] - ends[0]).normalize();
        let arm_at = |node: DVec3| -> Option<f64> {
            let list = at.get(&key(node))?;
            if list.len() != 2 {
                return None;
            }
            let j = *list.iter().find(|&&j| !chain.contains(&j))?;
            let t = &segs[j];
            (dir(t).cross(run_dir).length() > 0.5).then(|| t.a.distance(t.b) * 1000.0)
        };
        let arms: Vec<Option<f64>> = ends.iter().map(|e| arm_at(*e)).collect();
        let arm_there = match (arms[0], arms[1]) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        let compensator = match (dl, arm_needed, arm_there) {
            (Some(dl), Some(n), t) if t.is_none_or(|t| t < n) => {
                Some((2.0 * dl + 150.0).max(10.0 * p.outer_mm))
            }
            _ => None,
        };
        let spacing = spacing(&m, p.outer_mm, temp, riser);
        let supports = spacing.map_or(0, |s| (length / s).ceil() as usize + 1);
        out.push(Run {
            pipes: chain.iter().map(|&i| segs[i].id).collect(),
            system: p.system.clone(),
            pipe_type: p.pipe_type.clone(),
            outer_mm: p.outer_mm,
            length,
            riser,
            source: m.source,
            dt,
            dl,
            arm_needed,
            arm_there,
            compensator,
            spacing,
            supports,
        });
    }
    out
}

/// Supports by system, type and size, for the bill of materials.
pub fn supports(runs: &[Run]) -> Vec<(String, String, f64, usize)> {
    let mut map: std::collections::BTreeMap<(String, String, i64), usize> = Default::default();
    for r in runs {
        *map.entry((
            r.system.clone(),
            r.pipe_type.clone(),
            (r.outer_mm * 10.0) as i64,
        ))
        .or_default() += r.supports;
    }
    map.into_iter()
        .map(|((s, t, d), n)| {
            let name = pipe_types::by_id(&t).map_or(t.clone(), |x| x.name.clone());
            (s, name, d as f64 / 10.0, n)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::{Element, Level, Transaction};

    /// A pipe: its system, type, outside diameter and ends.
    type Run<'a> = (&'a str, &'a str, f64, [f64; 3], [f64; 3]);

    fn model(pipes: &[Run]) -> Model {
        let mut m = Model::new();
        let l = Element::new(
            "Ground",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let level = l.id;
        let mut tx = Transaction::new("pipes").insert(l);
        for (system, kind, outer, a, b) in pipes {
            let p = Pipe {
                level,
                system: system.to_string(),
                start: DVec3::from_array(*a),
                end: DVec3::from_array(*b),
                pipe_type: kind.to_string(),
                nominal_mm: *outer,
                outer_mm: *outer,
                inner_mm: *outer * 0.8,
                height_mm: None,
                insulation_mm: 0.0,
                insulation_set: false,
                mark: None,
            };
            tx = tx.insert(Element::new("Pipe", ElementKind::Pipe(p)));
        }
        tx.apply(&mut m).unwrap();
        m
    }

    /// Wavin's example: PP-R, 10 m, 20 to 60 °C, D 40: Δl 48 mm, free
    /// arm 876 mm, U compensator 400 mm.
    #[test]
    fn wavins_example_comes_out() {
        let m = model(&[
            ("TV", "WPPR", 40.0, [0.0, 0.0, 0.0], [4.0, 0.0, 0.0]),
            ("TV", "WPPR", 40.0, [4.0, 0.0, 0.0], [10.0, 0.0, 0.0]),
            ("TV", "WPPR", 40.0, [10.0, 0.0, 0.0], [10.0, 0.5, 0.0]),
        ]);
        let r = runs(&m, &Temps::default());
        let main = r.iter().find(|r| r.length > 9.0).unwrap();
        assert_eq!(main.pipes.len(), 2);
        assert!((main.dl.unwrap() - 48.0).abs() < 1e-9);
        assert!((main.arm_needed.unwrap() - 876.4).abs() < 0.1);
        assert_eq!(main.arm_there, Some(500.0));
        assert!(!main.arms_ok());
        assert_eq!(main.compensator, Some(400.0));
        // 60 °C, D 40: 1,00 m spacing, 11 supports
        assert_eq!(main.spacing, Some(1.0));
        assert_eq!(main.supports, 11);
    }

    /// HDPE: 7 m, D 110, Δt 50 K: Δl 70 mm, free arm about 878 mm.
    #[test]
    fn hdpe_drainage_expands_by_wavins_example() {
        let m = model(&[("K", "WHDPE", 110.0, [0.0, 0.0, 0.0], [7.0, 0.0, 0.0])]);
        let t = Temps {
            install: 0.0,
            drainage: 50.0,
            ..Default::default()
        };
        let r = &runs(&m, &t)[0];
        assert!((r.dl.unwrap() - 70.0).abs() < 1e-9);
        assert!((r.arm_needed.unwrap() - 877.5).abs() < 1.0);
        assert_eq!(r.spacing, Some(1.1));
    }

    #[test]
    fn socketed_drainage_gets_supports_only() {
        let m = model(&[("K", "WAS", 110.0, [0.0, 0.0, 0.0], [0.0, 0.0, 6.0])]);
        let r = &runs(&m, &Temps::default())[0];
        assert!(r.riser && r.dl.is_none() && r.arms_ok());
        assert_eq!((r.spacing, r.supports), (Some(2.0), 4));
        let s = supports(&runs(&m, &Temps::default()));
        assert_eq!(s[0].3, 4);
    }
}
