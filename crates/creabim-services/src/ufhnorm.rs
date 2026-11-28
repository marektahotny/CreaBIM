//! STN EN 1264-2 for system A (the pipe in the screed): the heat flux
//! from the characteristic (6.2) and the limit curves (6.5), read from
//! the norm's tables the user loads (Manage › Norm tables; the tables
//! are the user's licensed copy, never part of the program). Without
//! them the underfloor heating falls back to its old approximation.
//!
//! q = B · aB · aT^mT · aU^mU · aD^mD · ΔθH, with
//! aB = (1/α + su,0/λu,0) / (1/α + su/λE + RλB), mT = 1 − T/0,075,
//! mU = 100 · (0,045 − su), mD = 250 · (D − 0,020); for T > 0,375 the
//! flux at 0,375 times 0,375/T. The limit curve qG = φ · BG · (ΔθH/φ)^nG,
//! φ = (θF,max − θi)/Δθ0, crosses the characteristic at ΔθH,G: the
//! floor gives at most KH · ΔθH,G. Tables are read between their rows
//! and columns linearly.

use creabim_core::fireparams::{self, ParamTable};
use creabim_core::Model;

pub const AT: &str = "ufh_at";
pub const AU: &str = "ufh_au";
pub const AD: &str = "ufh_ad";
pub const BG: &str = "ufh_bg";
pub const BG_SUT: &str = "ufh_bg_sut";
pub const NG: &str = "ufh_ng";
pub const NG_SUT: &str = "ufh_ng_sut";
pub const QG_MAX: &str = "ufh_qgmax";
pub const FACTORS: &str = "ufh_factors";
pub const R_INS: &str = "ufh_rins";
/// The tables system A needs.
pub const TABLES: [&str; 10] = [AT, AU, AD, BG, BG_SUT, NG, NG_SUT, QG_MAX, FACTORS, R_INS];

/// A table of numbers: its rows' keys, its columns' keys, the values.
#[derive(Clone, Debug, PartialEq)]
struct Grid {
    rows: Vec<f64>,
    cols: Vec<f64>,
    v: Vec<Vec<f64>>,
}

impl Grid {
    /// The table with numbers for keys (its first column) and headers.
    fn of(t: &ParamTable) -> Option<Grid> {
        let cols: Vec<f64> = t
            .columns
            .iter()
            .skip(1)
            .filter_map(|c| fireparams::number(c))
            .collect();
        if cols.len() + 1 != t.columns.len() {
            return None;
        }
        let mut rows = vec![];
        let mut v = vec![];
        for r in &t.rows {
            let k = fireparams::number(r.first()?)?;
            let vals: Vec<f64> = r
                .iter()
                .skip(1)
                .map(|c| fireparams::number(c))
                .collect::<Option<_>>()?;
            if vals.len() != cols.len() {
                return None;
            }
            rows.push(k);
            v.push(vals);
        }
        (!rows.is_empty()).then_some(Grid { rows, cols, v })
    }

    /// Its value at row key `r`, column key `c`, between them linearly;
    /// held at its edges.
    fn at(&self, r: f64, c: f64) -> f64 {
        let (i0, i1, fi) = span(&self.rows, r);
        let (j0, j1, fj) = span(&self.cols, c);
        let a = self.v[i0][j0] + (self.v[i0][j1] - self.v[i0][j0]) * fj;
        let b = self.v[i1][j0] + (self.v[i1][j1] - self.v[i1][j0]) * fj;
        a + (b - a) * fi
    }
}

/// Where `x` falls among sorted keys: the two either side and how far.
fn span(keys: &[f64], x: f64) -> (usize, usize, f64) {
    let n = keys.len();
    if n == 1 || x <= keys[0] {
        return (0, 0, 0.0);
    }
    if x >= keys[n - 1] {
        return (n - 1, n - 1, 0.0);
    }
    let i = keys.iter().position(|k| *k > x).unwrap_or(n - 1);
    let (a, b) = (keys[i - 1], keys[i]);
    (i - 1, i, if b > a { (x - a) / (b - a) } else { 0.0 })
}

/// A one-row table (aT by RλB, read at row 0) or a two-column one (BG
/// by su/T, read at column 0) as a grid.
fn curve(t: &ParamTable) -> Option<Grid> {
    let cols: Vec<f64> = t
        .columns
        .iter()
        .skip(1)
        .filter_map(|c| fireparams::number(c))
        .collect();
    if cols.len() + 1 == t.columns.len() && t.rows.len() == 1 {
        let v: Vec<f64> = t.rows[0]
            .iter()
            .skip(1)
            .map(|c| fireparams::number(c))
            .collect::<Option<_>>()?;
        return (v.len() == cols.len()).then(|| Grid {
            rows: vec![0.0],
            cols,
            v: vec![v],
        });
    }
    let mut rows = vec![];
    let mut v = vec![];
    for r in &t.rows {
        rows.push(fireparams::number(r.first()?)?);
        v.push(vec![fireparams::number(r.get(1)?)?]);
    }
    (!rows.is_empty()).then_some(Grid {
        rows,
        cols: vec![0.0],
        v,
    })
}

/// What a floor's zone is: where people stay, a bathroom, the edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    Occupied,
    Bathroom,
    Peripheral,
}

impl Zone {
    fn key(self) -> &'static str {
        match self {
            Zone::Occupied => "occupied",
            Zone::Bathroom => "bathroom",
            Zone::Peripheral => "peripheral",
        }
    }
}

/// The norm's tables read.
#[derive(Clone, Debug, PartialEq)]
pub struct Norm {
    at: Grid,
    au: Grid,
    ad: Grid,
    bg: Grid,
    bg_sut: Grid,
    ng: Grid,
    ng_sut: Grid,
    /// θF,max by zone.
    theta_f: [(Zone, f64); 3],
    pub b0: f64,
    pub alpha: f64,
    pub lambda_u0: f64,
    pub s_u0: f64,
    pub dtheta0: f64,
    /// EN 1264-4 table 1: Rλ,ins at least, by what is under the floor.
    r_ins: Vec<(String, f64)>,
}

/// One row of a key/value table.
fn value(t: &ParamTable, key: &str) -> Option<f64> {
    fireparams::number(t.cell(key, "value"))
}

impl Norm {
    /// From tables (the project's and the application's).
    pub fn of(tables: &[ParamTable]) -> Option<Norm> {
        let t = |id: &str| tables.iter().find(|t| t.id == id);
        let q = t(QG_MAX)?;
        let theta = |z: Zone| fireparams::number(q.cell(z.key(), "theta_f_max"));
        let f = t(FACTORS)?;
        let r = t(R_INS)?;
        let rc = r.column("r_min")?;
        Some(Norm {
            at: curve(t(AT)?)?,
            au: Grid::of(t(AU)?)?,
            ad: Grid::of(t(AD)?)?,
            bg: Grid::of(t(BG)?)?,
            bg_sut: curve(t(BG_SUT)?)?,
            ng: Grid::of(t(NG)?)?,
            ng_sut: curve(t(NG_SUT)?)?,
            theta_f: [
                (Zone::Occupied, theta(Zone::Occupied)?),
                (Zone::Bathroom, theta(Zone::Bathroom)?),
                (Zone::Peripheral, theta(Zone::Peripheral)?),
            ],
            b0: value(f, "b0")?,
            alpha: value(f, "alpha")?,
            lambda_u0: value(f, "lambda_u0")?,
            s_u0: value(f, "s_u0")?,
            dtheta0: value(f, "dtheta0")?,
            r_ins: r
                .rows
                .iter()
                .filter_map(|x| Some((x.first()?.clone(), fireparams::number(x.get(rc)?)?)))
                .collect(),
        })
    }

    /// The model's: the project's tables, else the application's.
    pub fn of_model(model: &Model) -> Option<Norm> {
        let own = model
            .project_info()
            .map(|(_, p)| p.fire_params.clone())
            .unwrap_or_default();
        let all: Vec<ParamTable> = fireparams::effective(&own)
            .into_iter()
            .filter(|t| TABLES.contains(&t.id.as_str()))
            .collect();
        Norm::of(&all)
    }

    /// θF,max of a zone, °C.
    pub fn theta_f_max(&self, z: Zone) -> f64 {
        self.theta_f.iter().find(|x| x.0 == z).map_or(29.0, |x| x.1)
    }

    /// The equivalent heat transmission coefficient KH, W/(m²K), at
    /// spacing `t`, the cover `su` and λE of the screed, RλB, the pipe's
    /// outside diameter `d` (m); T over 0,375 by 0,375/T.
    pub fn k_h(&self, t: f64, su: f64, lambda_e: f64, r: f64, d: f64) -> f64 {
        let tt = t.min(0.375);
        let a_b = (1.0 / self.alpha + self.s_u0 / self.lambda_u0)
            / (1.0 / self.alpha + su / lambda_e + r);
        let a_t = self.at.at(0.0, r).max(1e-9);
        let a_u = self.au.at(tt, r).max(1e-9);
        let a_d = self.ad.at(tt, r).max(1e-9);
        let m_t = 1.0 - tt / 0.075;
        let m_u = 100.0 * (0.045 - su);
        let m_d = 250.0 * (d - 0.020);
        let k = self.b0 * a_b * a_t.powf(m_t) * a_u.powf(m_u) * a_d.powf(m_d);
        if t > 0.375 {
            k * 0.375 / t
        } else {
            k
        }
    }

    /// BG and nG of the limit curve at spacing `t`, cover `su` and λE.
    fn bg_ng(&self, t: f64, su: f64, lambda_e: f64) -> (f64, f64) {
        let tt = t.min(0.375);
        let x = su / lambda_e;
        if x <= 0.0792 {
            (self.bg.at(tt, x), self.ng.at(tt, x))
        } else if su / tt > 0.75 {
            (100.0, 0.0)
        } else {
            (self.bg_sut.at(su / tt, 0.0), self.ng_sut.at(su / tt, 0.0))
        }
    }

    /// ΔθH,G: where the limit curve crosses the characteristic, K.
    #[allow(clippy::too_many_arguments)]
    pub fn dtheta_hg(
        &self,
        zone: Zone,
        ti: f64,
        t: f64,
        su: f64,
        lambda_e: f64,
        r: f64,
        d: f64,
    ) -> f64 {
        let phi = ((self.theta_f_max(zone) - ti) / self.dtheta0).max(1e-6);
        let (bg, ng) = self.bg_ng(t, su, lambda_e);
        let kh = self.k_h(t.min(0.375), su, lambda_e, r, d);
        phi * (bg / kh).powf(1.0 / (1.0 - ng))
    }

    /// The most the floor gives, W/m²: KH · ΔθH,G (for T over 0,375 by
    /// 0,375/T).
    #[allow(clippy::too_many_arguments)]
    pub fn q_g(&self, zone: Zone, ti: f64, t: f64, su: f64, lambda_e: f64, r: f64, d: f64) -> f64 {
        let kh = self.k_h(t, su, lambda_e, r, d);
        kh * self.dtheta_hg(zone, ti, t, su, lambda_e, r, d)
    }

    /// Rλ,ins the insulation under the pipes needs at least (STN EN
    /// 1264-4, table 1): a heated room under it, else unheated or the
    /// ground; outside air by the design outside temperature θd.
    pub fn r_ins_min(&self, heated_below: bool, outside_air: Option<f64>) -> Option<f64> {
        let key = match (heated_below, outside_air) {
            (_, Some(td)) if td >= 0.0 => "air_0",
            (_, Some(td)) if td >= -5.0 => "air_m5",
            (_, Some(_)) => "air_m15",
            (true, None) => "heated_below",
            (false, None) => "unheated_or_ground",
        };
        self.r_ins.iter().find(|x| x.0 == key).map(|x| x.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up table (no norm values in the program's sources).
    fn table(id: &str, columns: &[&str], rows: &[&[&str]]) -> ParamTable {
        ParamTable {
            id: id.into(),
            columns: columns.iter().map(|c| c.to_string()).collect(),
            rows: rows
                .iter()
                .map(|r| r.iter().map(|c| c.to_string()).collect())
                .collect(),
            ..ParamTable::default()
        }
    }

    #[test]
    fn grids_read_between_rows_and_columns() {
        let g = Grid::of(&table(
            "x",
            &["T", "0", "1"],
            &[&["0", "0", "10"], &["2", "20", "30"]],
        ))
        .unwrap();
        assert!((g.at(1.0, 0.5) - 15.0).abs() < 1e-12);
        assert!((g.at(5.0, 5.0) - 30.0).abs() < 1e-12);
        assert!((g.at(-1.0, 0.0) - 0.0).abs() < 1e-12);
    }

    #[test]
    fn the_limit_crosses_the_characteristic() {
        let one = |id: &str| table(id, &["T", "0", "1"], &[&["0", "1", "1"], &["1", "1", "1"]]);
        let tables = vec![
            table(AT, &["key", "0", "1"], &[&["aT", "1", "1"]]),
            one(AU),
            one(AD),
            table(
                BG,
                &["T", "0", "1"],
                &[&["0", "50", "50"], &["1", "50", "50"]],
            ),
            table(BG_SUT, &["su_T", "BG"], &[&["0", "50"], &["1", "50"]]),
            table(NG, &["T", "0", "1"], &[&["0", "0", "0"], &["1", "0", "0"]]),
            table(NG_SUT, &["su_T", "nG"], &[&["0", "0"], &["1", "0"]]),
            table(
                QG_MAX,
                &["key", "theta_f_max", "theta_i", "q_g_max"],
                &[
                    &["occupied", "30", "20", "1"],
                    &["bathroom", "30", "20", "1"],
                    &["peripheral", "40", "20", "1"],
                ],
            ),
            table(
                FACTORS,
                &["key", "value", "text"],
                &[
                    &["b0", "5", ""],
                    &["alpha", "10", ""],
                    &["lambda_u0", "1", ""],
                    &["s_u0", "0,05", ""],
                    &["dtheta0", "10", ""],
                ],
            ),
            table(
                R_INS,
                &["key", "r_min", "text"],
                &[&["heated_below", "0,5", ""]],
            ),
        ];
        let n = Norm::of(&tables).unwrap();
        // aB = 1 with su/λE = su,0/λu,0 and RλB 0; every a = 1: KH = B0
        let kh = n.k_h(0.1, 0.05, 1.0, 0.0, 0.02);
        assert!((kh - 5.0).abs() < 1e-9);
        // nG 0: qG = φ · BG = (30 − 20)/10 · 50
        let q = n.q_g(Zone::Occupied, 20.0, 0.1, 0.05, 1.0, 0.0, 0.02);
        assert!((q - 50.0).abs() < 1e-9);
        assert_eq!(n.r_ins_min(true, None), Some(0.5));
        // past 0,375 by 0,375/T
        assert!((n.k_h(0.75, 0.05, 1.0, 0.0, 0.02) - 2.5).abs() < 1e-9);
    }
}
