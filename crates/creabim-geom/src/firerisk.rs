//! The fire risk of the fire zones (požiarne riziko) and the greatest
//! area and number of storeys they may have, the way STN 92 0201-1
//! works them out for buildings other than production ones:
//!
//! - the mean fire load p of the zone, from each room's random load pn
//!   (by the item of the fire load table it is given) and its fixed
//!   load ps, weighed by the rooms' areas;
//! - the factor a of the speed of burning, weighed the same way by the
//!   loads;
//! - the factor b of the zone's openings: the windows and the doors
//!   with no fire rating in its outer walls, their area and height
//!   against the zone's area and clear height, through the factor k of
//!   the openings' geometry;
//! - the design fire load pv = p·a·b, whether the zone is without fire
//!   risk, its greatest area Smax and its greatest number of fire
//!   storeys.
//!
//! Every value of the norm is read from the designer's parameter file
//! (`creabim_core::fireparams`); the program has the procedure only.
//! Where a table or a value is missing the zone says so and gives no
//! figure. Concentrated loads, sprinklers and fire alarms (cn) are not
//! taken into account.

use crate::schedule::{Column, Row, Schedule};
use creabim_core::fireparams::{
    self, ParamTable, FIRE_FIXED_LOAD, FIRE_LOAD, FIRE_OPENING_K, FIRE_OPENING_K_LARGE, FIRE_RISK,
};
use creabim_core::room;
use creabim_core::{ElementId, ElementKind, Model, OpeningKind, ScheduleCategory};
use glam::DVec2;

/// A fire zone's risk and limits.
#[derive(Clone, Debug, PartialEq)]
pub struct ZoneRisk {
    pub code: String,
    pub first: ElementId,
    /// Its area S, m², and the largest room's Sm.
    pub area: f64,
    pub largest: f64,
    /// The mean loads, kg/m²: random pn, fixed ps and p = pn + ps.
    pub pn: f64,
    pub ps: f64,
    pub p: f64,
    pub a: f64,
    /// The openings: their area So, m², mean height ho and the zone's
    /// mean clear height hs, m.
    pub openings: f64,
    pub ho: f64,
    pub hs: f64,
    pub n: f64,
    pub k: f64,
    pub b: f64,
    pub pv: f64,
    /// Without fire risk (5.2.1, with a non-combustible structure).
    pub no_risk: bool,
    /// The greatest area of a fire storey, m², where it is worked out.
    pub smax: Option<f64>,
    /// The greatest number of fire storeys.
    pub storeys_max: Option<u32>,
    /// The fire storeys it has.
    pub storeys: usize,
    /// Rooms whose item gives no pn, counted as remaining corridor.
    pub unlisted: Vec<ElementId>,
    /// What could not be worked out: a missing table or value.
    pub missing: Vec<String>,
}

impl ZoneRisk {
    /// Is its area over the greatest a fire storey may have?
    pub fn too_large(&self) -> bool {
        let per_storey = self.area / self.storeys.max(1) as f64;
        self.smax.is_some_and(|s| per_storey > s)
    }

    /// Has it more fire storeys than it may?
    pub fn too_tall(&self) -> bool {
        self.storeys_max
            .is_some_and(|z| self.storeys > z as usize && !self.no_risk)
    }
}

/// One of the fire tables: the project's own, else the application's.
pub fn table<'a>(model: &'a Model, id: &str) -> Option<&'a ParamTable> {
    fireparams::find(&model.project_info()?.1.fire_params, id)
}

/// The fire risk's single value by its key.
fn value(t: Option<&ParamTable>, key: &str) -> Option<f64> {
    fireparams::number(t?.cell(key, "value"))
}

/// A value read off a grid by linear interpolation in both ways, held
/// at the edges: rows by their first cell, columns by their headers'
/// numbers (a header with no number is endless).
fn grid(t: &ParamTable, row: f64, col: f64, cols: &[usize]) -> Option<f64> {
    let rows: Vec<(f64, Vec<f64>)> = t
        .rows
        .iter()
        .filter_map(|r| {
            let key = fireparams::number(r.first()?)?;
            let vals: Option<Vec<f64>> = cols
                .iter()
                .map(|&c| r.get(c).and_then(|v| fireparams::number(v)))
                .collect();
            Some((key, vals?))
        })
        .collect();
    if rows.is_empty() || cols.is_empty() {
        return None;
    }
    let heads: Vec<f64> = cols
        .iter()
        .map(|&c| fireparams::number(&t.columns[c]).unwrap_or(f64::INFINITY))
        .collect();
    let along = |keys: &[f64], x: f64| -> (usize, usize, f64) {
        let last = keys.len() - 1;
        if x <= keys[0] {
            return (0, 0, 0.0);
        }
        for i in 0..last {
            if x <= keys[i + 1] {
                let span = keys[i + 1] - keys[i];
                let f = if span.is_finite() && span > 0.0 {
                    (x - keys[i]) / span
                } else {
                    0.0
                };
                return (i, i + 1, f);
            }
        }
        (last, last, 0.0)
    };
    let row_keys: Vec<f64> = rows.iter().map(|r| r.0).collect();
    let (r0, r1, fr) = along(&row_keys, row);
    let (c0, c1, fc) = along(&heads, col);
    let at = |r: usize| rows[r].1[c0] + (rows[r].1[c1] - rows[r].1[c0]) * fc;
    Some(at(r0) + (at(r1) - at(r0)) * fr)
}

/// The factor k of the openings' geometry by n, the largest room's area
/// and the clear height.
fn opening_k(model: &Model, area: f64, n: f64, sm: f64, hs: f64, missing: &mut Vec<String>) -> f64 {
    let risk = table(model, FIRE_RISK);
    let limit = value(risk, "k_small_area").unwrap_or(f64::INFINITY);
    if area <= limit {
        let Some(t) = table(model, FIRE_OPENING_K) else {
            missing.push(FIRE_OPENING_K.into());
            return 0.0;
        };
        let cols: Vec<usize> = (1..t.columns.len()).collect();
        return grid(t, n, sm, &cols).unwrap_or(0.0);
    }
    let Some(t) = table(model, FIRE_OPENING_K_LARGE) else {
        missing.push(FIRE_OPENING_K_LARGE.into());
        return 0.0;
    };
    // the columns come in pairs of a room area and a low and high
    // clear height ("750/3,6", "750/10,8")
    let (low, high) = (
        value(risk, "hs_low").unwrap_or(0.0),
        value(risk, "hs_high").unwrap_or(0.0),
    );
    let mut lows = vec![];
    let mut highs = vec![];
    for (i, c) in t.columns.iter().enumerate().skip(1) {
        let h = c.split('/').nth(1).and_then(fireparams::number);
        match h {
            Some(h) if (h - low).abs() < 1e-6 => lows.push(i),
            Some(h) if (h - high).abs() < 1e-6 => highs.push(i),
            _ => {}
        }
    }
    let area_of =
        |c: &str| fireparams::number(c.split('/').next().unwrap_or("")).unwrap_or(f64::INFINITY);
    // the column of the room area: the first whose limit holds it
    let pick = |cols: &[usize]| -> Option<f64> {
        let c = cols
            .iter()
            .copied()
            .find(|&c| sm <= area_of(&t.columns[c]))
            .or(cols.last().copied())?;
        grid(t, n, 0.0, &[c])
    };
    let (Some(kl), Some(kh)) = (pick(&lows), pick(&highs)) else {
        missing.push(FIRE_OPENING_K_LARGE.into());
        return 0.0;
    };
    let f = if high > low {
        ((hs - low) / (high - low)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    kl + (kh - kl) * f
}

/// The fixed fire load of a room by its area: windows, doors and floor.
fn fixed_load(model: &Model, area: f64) -> Option<f64> {
    let t = table(model, FIRE_FIXED_LOAD)?;
    let row = t.rows.iter().find(|r| {
        fireparams::number(r.first().map_or("", |s| s.as_str())).is_none_or(|a| area <= a)
    })?;
    Some(
        row.iter()
            .skip(1)
            .filter_map(|c| fireparams::number(c))
            .sum(),
    )
}

/// Levels with rooms, above ground and below it (a level under 1 m
/// below the zero is underground).
fn storeys(model: &Model) -> (usize, usize) {
    let mut above = vec![];
    let mut below = vec![];
    for e in model.iter() {
        if let ElementKind::Room(r) = &e.kind {
            let Some(l) = model.level(r.level) else {
                continue;
            };
            let list = if l.elevation < -1.0 {
                &mut below
            } else {
                &mut above
            };
            if !list.contains(&r.level) {
                list.push(r.level);
            }
        }
    }
    (above.len(), below.len())
}

/// The fire risk of every zone of the model.
pub fn zones(model: &Model) -> Vec<ZoneRisk> {
    let risk = table(model, FIRE_RISK);
    let loads = table(model, FIRE_LOAD);
    let system = model
        .project_info()
        .map(|p| p.1.fire_system.trim().to_string())
        .unwrap_or_default();
    let system = if system.is_empty() {
        "N".into()
    } else {
        system
    };
    let (above, below) = storeys(model);
    let walls = crate::firezones::outer_walls(model);
    let mut out = vec![];
    for z in crate::firezones::zones(model) {
        let mut missing: Vec<String> = vec![];
        for (id, t) in [(FIRE_LOAD, loads), (FIRE_RISK, risk)] {
            if t.is_none() {
                missing.push(id.into());
            }
        }
        let comm = value(risk, "comm_pn").unwrap_or(0.0);
        let an_default = value(risk, "an_default").unwrap_or(1.0);
        let a_fixed = value(risk, "as").unwrap_or(1.0);
        let (mut s, mut sm, mut pn_s, mut ps_s, mut pa_s, mut hs_s) =
            (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let (mut so, mut so_h, mut so_root) = (0.0, 0.0, 0.0);
        let mut unlisted = vec![];
        let mut underground = true;
        for &id in &z.rooms {
            let Some(ElementKind::Room(r)) = model.get(id).map(|e| &e.kind) else {
                continue;
            };
            let Some(shape) = room::shape(model, id) else {
                continue;
            };
            let area = room::net_area(model, r, &shape);
            if model.level(r.level).is_none_or(|l| l.elevation >= -1.0) {
                underground = false;
            }
            let row = loads.and_then(|t| t.row(r.fire_load.trim()));
            let cell =
                |c: &str| row.and_then(|_| fireparams::number(loads?.cell(r.fire_load.trim(), c)));
            let (pn, an) = match cell("pn") {
                Some(pn) => (pn, cell("an").unwrap_or(an_default)),
                None => {
                    unlisted.push(id);
                    (comm, an_default)
                }
            };
            let ps = match r.fire_ps {
                Some(ps) => ps,
                None => fixed_load(model, area).unwrap_or_else(|| {
                    if !missing.iter().any(|m| m == FIRE_FIXED_LOAD) {
                        missing.push(FIRE_FIXED_LOAD.into());
                    }
                    0.0
                }),
            };
            s += area;
            sm = f64::max(sm, area);
            pn_s += pn * area;
            ps_s += ps * area;
            pa_s += (pn * an + ps * a_fixed) * area;
            hs_s += room::height(model, r) * area;
            // the room's openings in outer walls
            for w in walls.iter().filter(|w| w.wall.level == r.level) {
                let along = (w.wall.end - w.wall.start).normalize_or_zero();
                for e in model.iter() {
                    let ElementKind::Opening(o) = &e.kind else {
                        continue;
                    };
                    if o.host != w.id {
                        continue;
                    }
                    let counted = match o.kind {
                        OpeningKind::Window => true,
                        OpeningKind::Door => o.door.as_ref().is_none_or(|d| d.fire.is_none()),
                        OpeningKind::Hole => false,
                    };
                    if !counted {
                        continue;
                    }
                    let mid: DVec2 = w.wall.start + along * (o.offset + o.width / 2.0)
                        - w.outward * (w.wall.thickness / 2.0 + 0.15);
                    if !creabim_core::polygon::contains(&shape.outline, mid) {
                        continue;
                    }
                    let a = o.width * o.height;
                    so += a;
                    so_h += a * o.height;
                    so_root += a * o.height.sqrt();
                }
            }
        }
        let first = z.rooms.first().copied().unwrap_or_default();
        if s <= 0.0 {
            continue;
        }
        let p = (pn_s + ps_s) / s;
        let a = if pn_s + ps_s > 0.0 {
            pa_s / (pn_s + ps_s)
        } else {
            an_default
        };
        let hs = hs_s / s;
        let ho = if so > 0.0 { so_h / so } else { 0.0 };
        let n_min = value(risk, "n_min").unwrap_or(0.0);
        let n = if so > 0.0 && hs > 0.0 {
            (so / s * (ho / hs).sqrt()).max(n_min)
        } else {
            n_min
        };
        let k = opening_k(model, s, n, sm, hs, &mut missing);
        let b_raw = if so_root > 0.0 {
            s * k / so_root
        } else if n_min > 0.0 && hs > 0.0 {
            k / (n_min * hs.sqrt())
        } else {
            0.0
        };
        let b = b_raw.clamp(
            value(risk, "b_min").unwrap_or(0.0),
            value(risk, "b_max").unwrap_or(f64::INFINITY),
        );
        let pv = p * a * b;
        let no_risk = system == "N"
            && (value(risk, "none_pv").is_some_and(|v| pv <= v)
                || (value(risk, "none_pv_low_a").is_some_and(|v| pv <= v)
                    && value(risk, "none_a").is_some_and(|v| a <= v)));
        let storeys_here = z.levels.len();
        let smax = if no_risk || value(risk, "no_smax_area").is_none_or(|v| s <= v) {
            None
        } else {
            let a_c = a.clamp(
                value(risk, "a_min").unwrap_or(0.0),
                value(risk, "a_max").unwrap_or(f64::INFINITY),
            );
            let c = if underground {
                value(risk, "c_N")
                    .zip(value(risk, "c_under"))
                    .map(|(c, u)| c * (u + below as f64).sqrt())
            } else {
                value(risk, &format!("c_{system}"))
            };
            let npn = above.max(1) as f64;
            match (value(risk, "smax_1"), value(risk, "smax_2"), c) {
                (Some(s1), Some(s2), Some(c)) if c > 0.0 => {
                    Some((s1 - s2 * a_c.ln()) / (c * npn.sqrt()))
                }
                _ => None,
            }
        };
        let z_key = format!("z_{}", system.chars().next().unwrap_or('N'));
        let storeys_max = value(risk, &z_key)
            .filter(|_| pv > 0.0)
            .map(|v| (v / pv).floor().max(1.0) as u32);
        out.push(ZoneRisk {
            code: z.code,
            first,
            area: s,
            largest: sm,
            pn: pn_s / s,
            ps: ps_s / s,
            p,
            a,
            openings: so,
            ho,
            hs,
            n,
            k,
            b,
            pv,
            no_risk,
            smax,
            storeys_max,
            storeys: storeys_here,
            unlisted,
            missing,
        });
    }
    out
}

fn num(v: f64, places: usize) -> String {
    format!("{v:.places$}").replace('.', ",")
}

/// The fire risk of the zones as a table.
pub fn schedule(model: &Model) -> Schedule {
    let col = |title: &'static str, width: f64, numeric: bool| Column {
        title,
        field: None,
        width,
        numeric,
    };
    let columns = vec![
        col("Fire zone", 22.0, false),
        col("Area", 20.0, true),
        col("p [kg/m²]", 16.0, true),
        col("a", 12.0, true),
        col("b", 12.0, true),
        col("pv [kg/m²]", 18.0, true),
        col("Smax [m²]", 18.0, true),
        col("Max. fire storeys", 18.0, true),
        col("Note", 60.0, false),
    ];
    let units = model.units();
    let rows = zones(model)
        .into_iter()
        .map(|z| {
            let mut notes: Vec<String> = vec![];
            if z.no_risk {
                notes.push(creabim_core::words::tr("without fire risk"));
            }
            if z.too_large() {
                notes.push(creabim_core::words::tr("AREA OVER Smax"));
            }
            if z.too_tall() {
                notes.push(creabim_core::words::tr("MORE FIRE STOREYS THAN ALLOWED"));
            }
            if !z.unlisted.is_empty() {
                notes.push(format!(
                    "{}: {}",
                    creabim_core::words::tr("rooms without a fire load item, taken as corridor"),
                    z.unlisted.len()
                ));
            }
            if !z.missing.is_empty() {
                notes.push(format!(
                    "{}: {}",
                    creabim_core::words::tr("missing parameters"),
                    z.missing.join(", ")
                ));
            }
            Row {
                id: z.first,
                group: false,
                cells: vec![
                    z.code.clone(),
                    units.area(z.area),
                    num(z.p, 1),
                    num(z.a, 2),
                    num(z.b, 2),
                    num(z.pv, 1),
                    z.smax.map(|s| num(s, 0)).unwrap_or_else(|| "-".into()),
                    z.storeys_max
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "-".into()),
                    notes.join("; "),
                ],
            }
        })
        .collect::<Vec<_>>();
    let listed = rows.len();
    Schedule {
        category: ScheduleCategory::FireRisk,
        columns,
        rows,
        totals: vec![],
        listed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::{Element, Transaction};

    /// Made-up parameters in the file's shape: the norm's values are
    /// not in the repository.
    const PARAMS: &str = "#table fire_load\n#columns item\tname\tan\tpn\tnote\n\
        T1\tTest room\t1,0\t40\t\n\
        #table fire_fixed_load\n#columns area\twindows\tdoors\tfloors\n\
        1000\t2,0\t1,0\t4,0\n\t1,0\t0,5\t4,0\n\
        #table fire_opening_k\n#columns n\t10\t100\n\
        0,005\t0,01\t0,02\n0,1\t0,1\t0,2\n\
        #table fire_risk\n#columns key\tvalue\tnote\n\
        comm_pn\t5\t\nas\t0,9\t\nan_default\t1,2\t\nb_min\t0,5\t\nb_max\t2,0\t\n\
        n_min\t0,005\t\nk_small_area\t500\t\nnone_pv\t3\t\nsmax_1\t1000\t\n\
        smax_2\t2000\t\na_min\t0,3\t\na_max\t1,3\t\nc_N\t0,2\t\nz_N\t200\t\nno_smax_area\t10\t\n";

    #[test]
    fn a_zone_gets_its_design_fire_load_and_limits() {
        let mut m = creabim_core::starter::starter_house();
        let rooms: Vec<Element> = m
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Room(_)))
            .cloned()
            .collect();
        let mut tx = Transaction::new("zones");
        for (i, e) in rooms.iter().enumerate() {
            let mut n = e.clone();
            if let ElementKind::Room(r) = &mut n.kind {
                r.fire_zone = "N1.01".into();
                if i > 0 {
                    r.fire_load = "T1".into();
                }
            }
            tx = tx.replace(e.clone(), n);
        }
        tx.apply(&mut m).unwrap();
        // without the parameters it says what is missing
        let z = zones(&m);
        assert_eq!(z.len(), 1);
        assert!(z[0].missing.iter().any(|t| t == FIRE_LOAD));
        let mut info = m.project_info().map(|p| p.1.clone()).unwrap_or_default();
        info.fire_params = fireparams::parse(PARAMS);
        creabim_core::set_project(&m, info).apply(&mut m).unwrap();
        let z = &zones(&m)[0];
        assert!(z.missing.is_empty(), "{:?}", z.missing);
        assert_eq!(z.unlisted.len(), 1);
        assert!(z.p > 5.0 && z.p < 47.0, "{z:?}");
        assert!(z.a > 0.9 && z.a < 1.2);
        assert!(z.openings > 0.0 && z.b >= 0.5 && z.b <= 2.0);
        assert!((z.pv - z.p * z.a * z.b).abs() < 1e-9);
        let smax = z.smax.expect("Smax worked out");
        assert!(smax > 0.0);
        assert!(z.storeys_max.unwrap() >= 1);
        let s = schedule(&m);
        assert_eq!(s.rows.len(), 1);
        assert_eq!(s.rows[0].cells[0], "N1.01");
    }

    #[test]
    fn the_grid_reads_between_its_rows_and_columns() {
        let t = &fireparams::parse(PARAMS)
            .into_iter()
            .find(|t| t.id == FIRE_OPENING_K)
            .unwrap();
        let v = grid(t, 0.0525, 55.0, &[1, 2]).unwrap();
        assert!((v - 0.0825).abs() < 1e-9, "{v}");
        assert_eq!(grid(t, 0.0, 0.0, &[1, 2]), Some(0.01));
        assert_eq!(grid(t, 1.0, 1000.0, &[1, 2]), Some(0.2));
    }
}
