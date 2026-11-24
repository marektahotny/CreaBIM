//! The checks of a detail's temperature field by STN 73 0540-2 (čl. 5.3:
//! the lowest inner surface temperature θsi ≥ θsi,80 + Δθsi, table 4, and
//! the temperature factor fRsi ≥ fRsi,N, table 5) with the surface
//! resistances of STN 73 0540-3 (table 15) or of STN EN ISO 13788, and
//! the saturation pressure of water vapour (STN 73 0540-3, formulas 8 to
//! 10). Every value comes from the norms' parameter files read into the
//! project; none is written in the code.

use creabim_core::fireparams::{self, number, ParamTable};

/// STN 73 0540-3 table 15: `key`, `Rsi`, `Rse`, `hi`, `he`, `text`.
pub const RS: &str = "prestup_tepla";
/// STN 73 0540-3 formulas: `key`, `vzorec`, `platnost`, `text`.
pub const FORMULAS: &str = "vzorce";
/// STN 73 0540-2 table 4: `key`, `heating`, `delta_plocha_K`,
/// `delta_kut_K`, `text`.
pub const DELTA: &str = "frsi_delta";
/// STN 73 0540-2 čl. 5.3 constants: `key`, `value`, `unit`, `text`.
pub const CONSTANTS: &str = "frsi_konstanty";
/// STN 73 0540-2 table 5: `theta_e`, `theta_si`, `frsi_n_20`, `frsi_n_22`.
pub const FRSI_N: &str = "frsi_n";
/// STN EN ISO 13788 boundary conditions: `key`, `value`, `unit`, `text`.
pub const ISO_13788: &str = "okrajove_podmienky";
/// STN EN ISO 10211 simplifications (mesh criteria): `key`, `value`, …
pub const ISO_10211: &str = "zjednodusenia";
/// STN EN ISO 10211 input data (soil λ): `key`, `value`, …
pub const ISO_10211_INPUT: &str = "vstupne_udaje";

/// STN 73 0540-3 table 16: `pol`, `skupina`, `material`, `rho`,
/// `lambda_ext`, `lambda_int`, `c`, `mu`, `delta`, `pozn`.
pub const MATERIALS: &str = "materialy";
/// STN 73 0540-3 table 17 (thin layers): `pol`, `skupina`, `material`,
/// `rho`, `d_mm`, `mu`, `delta_s`, `pozn`.
pub const THIN_LAYERS: &str = "tenke_vrstvy";

/// STN 73 0540-2 čl. 6.1 limits of condensation inside a build-up:
/// `key`, `mc_max_kg_m2a`, `vzorec`, `text`.
pub const CONDENSATION_LIMITS: &str = "kondenzacia_limity";

/// STN 73 0540-2 čl. 6.1.3 constants: `key`, `value` (`phi_i_zakladna`).
pub const CONDENSATION_SK: &str = "kondenzacia_konstanty_sk";

/// φi for the condensation inside a build-up (čl. 6.1.3), %.
pub fn phi_i_condensation(tables: &[ParamTable]) -> Option<f64> {
    value(tables, CONDENSATION_SK, "phi_i_zakladna", "value")
}

/// The kinds of build-up of čl. 6.1 with their limits of Mc: key, Mc
/// max (kg/(m²·a)), text.
pub fn condensation_kinds(tables: &[ParamTable]) -> Vec<(String, f64, String)> {
    let Some(t) = table(tables, CONDENSATION_LIMITS) else {
        return vec![];
    };
    t.rows
        .iter()
        .filter_map(|r| {
            Some((
                r[0].clone(),
                number(t.cell(&r[0], "mc_max_kg_m2a"))?,
                t.cell(&r[0], "text").to_string(),
            ))
        })
        .collect()
}

/// STN EN ISO 10211 the model of a junction with the ground (7.2.3,
/// table 5, 12.4.1): `key`, `value`, …
pub const GROUND: &str = "styk_s_terenom";

/// The extents of a junction with the ground for `soil` and B.
pub fn ground_params(
    tables: &[ParamTable],
    soil: usize,
    b: f64,
) -> Option<crate::thermfield::Ground> {
    let v = |k: &str| value(tables, GROUND, k, "value");
    Some(crate::thermfield::Ground {
        soil,
        b,
        half_max: v("podlaha_max")?,
        soil_max: v("zemina_max")?,
        soil_factor: v("zemina_nasobok_B")?,
        floor_factor: v("podlaha_nasobok_B")?,
        cut_min: v("rez_min")?,
        cut_factor: v("rez_nasobok_hrubky")?,
    })
}

/// The tables the checks read.
pub const TABLES: [&str; 16] = [
    RS,
    FORMULAS,
    DELTA,
    CONSTANTS,
    FRSI_N,
    ISO_13788,
    ISO_10211,
    ISO_10211_INPUT,
    MATERIALS,
    THIN_LAYERS,
    crate::glaser::CONSTANTS,
    CONDENSATION_LIMITS,
    CONDENSATION_SK,
    crate::glaser::SK_CONSTANTS,
    crate::glaser::SK_ANNUAL,
    GROUND,
];

/// The norm tables of the project, else the application's.
pub fn tables_of(model: &creabim_core::Model) -> Vec<ParamTable> {
    let own = model
        .project_info()
        .map(|(_, p)| p.fire_params.clone())
        .unwrap_or_default();
    fireparams::effective(&own)
        .into_iter()
        .filter(|t| TABLES.contains(&t.id.as_str()))
        .collect()
}

fn table<'a>(tables: &'a [ParamTable], id: &str) -> Option<&'a ParamTable> {
    tables.iter().find(|t| t.id == id)
}

fn value(tables: &[ParamTable], id: &str, key: &str, col: &str) -> Option<f64> {
    number(table(tables, id)?.cell(key, col))
}

/// The surface resistances of a detail, m²·K/W.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceRs {
    /// Inside, the heat flowing up (a ceiling), level (a wall), down (a
    /// floor).
    pub up: f64,
    pub level: f64,
    pub down: f64,
    /// Outside.
    pub e: f64,
}

/// Rsi by STN 73 0540-3 table 15 (by the heat flow's direction), or with
/// `iso` the 0,25 of STN EN ISO 13788 (4.4.1) on every inner surface; Rse
/// of table 15 (winter). None when a table is missing.
pub fn surface_rs(tables: &[ParamTable], iso: bool) -> Option<SurfaceRs> {
    let e = value(tables, RS, "vonkajsi_zima", "Rse")?;
    if iso {
        let r = value(tables, ISO_13788, "Rsi_plesne_nepriesvitne", "value")?;
        return Some(SurfaceRs {
            up: r,
            level: r,
            down: r,
            e,
        });
    }
    Some(SurfaceRs {
        up: value(tables, RS, "vnutorny_nahor", "Rsi")?,
        level: value(tables, RS, "vnutorny_vodorovne", "Rsi")?,
        down: value(tables, RS, "vnutorny_nadol", "Rsi")?,
        e,
    })
}

/// The numbers of a formula `a·exp(b·θ/(c+θ))`, as written in the norm's
/// table ("610,5·exp(17,269·θ/(237,3+θ))").
fn magnus(text: &str) -> Option<(f64, f64, f64)> {
    let nums: Vec<f64> = text
        .split(|c: char| !(c.is_ascii_digit() || c == ',' || c == '.'))
        .filter(|s| s.chars().any(|c| c.is_ascii_digit()))
        .filter_map(number)
        .collect();
    (nums.len() >= 3).then(|| (nums[0], nums[1], nums[2]))
}

/// Saturation pressure of water vapour over water (θ ≥ 0) or ice, Pa
/// (STN 73 0540-3 formulas 8, 9).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Psat {
    plus: (f64, f64, f64),
    minus: (f64, f64, f64),
}

impl Psat {
    pub fn of(tables: &[ParamTable]) -> Option<Psat> {
        let t = table(tables, FORMULAS)?;
        Some(Psat {
            plus: magnus(t.cell("p_sat_plus", "vzorec"))?,
            minus: magnus(t.cell("p_sat_minus", "vzorec"))?,
        })
    }

    pub fn p(&self, theta: f64) -> f64 {
        let (a, b, c) = if theta >= 0.0 { self.plus } else { self.minus };
        a * (b * theta / (c + theta)).exp()
    }

    /// The temperature at which the saturation pressure is `p`, °C.
    pub fn theta(&self, p: f64) -> f64 {
        let at = |(a, b, c): (f64, f64, f64)| {
            let l = (p / a).ln();
            c * l / (b - l)
        };
        if p >= self.plus.0 {
            at(self.plus)
        } else {
            at(self.minus)
        }
    }

    /// The critical surface temperature for mould θsi,80 at θai, φi (%)
    /// (formula 10).
    pub fn theta_si80(&self, theta_i: f64, phi_i: f64) -> f64 {
        self.theta(phi_i * self.p(theta_i) / 80.0)
    }

    /// The dew point at θa, φa (%).
    pub fn dew_point(&self, theta: f64, phi: f64) -> f64 {
        self.theta(phi / 100.0 * self.p(theta))
    }
}

/// The ways of heating of table 4: key and text.
pub fn heating_modes(tables: &[ParamTable]) -> Vec<(String, String)> {
    let Some(t) = table(tables, DELTA) else {
        return vec![];
    };
    let text = t.column("text");
    t.rows
        .iter()
        .map(|r| {
            (
                r[0].clone(),
                text.and_then(|c| r.get(c)).cloned().unwrap_or_default(),
            )
        })
        .collect()
}

/// Δθsi of table 4 for heating `key`, on a plain surface or in a corner.
pub fn delta_theta_si(tables: &[ParamTable], key: &str, corner: bool) -> Option<f64> {
    value(
        tables,
        DELTA,
        key,
        if corner {
            "delta_kut_K"
        } else {
            "delta_plocha_K"
        },
    )
}

/// φi above which the surface temperature is not proven by calculation
/// (čl. 5.3.1), %.
pub fn phi_i_max(tables: &[ParamTable]) -> Option<f64> {
    value(tables, CONSTANTS, "phi_i_max", "value")
}

/// fRsi,N of table 5 for θe and θi, interpolated linearly; an error
/// when they lie outside the table (no extrapolation).
pub fn frsi_n(tables: &[ParamTable], theta_e: f64, theta_i: f64) -> Result<f64, String> {
    let t = table(tables, FRSI_N).ok_or_else(|| FRSI_N.to_string())?;
    let mut rows: Vec<(f64, f64, f64)> = t
        .rows
        .iter()
        .filter_map(|r| {
            Some((
                number(&r[0])?,
                number(t.cell(&r[0], "frsi_n_20"))?,
                number(t.cell(&r[0], "frsi_n_22"))?,
            ))
        })
        .collect();
    rows.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (lo, hi) = (rows.first().ok_or("")?.0, rows.last().ok_or("")?.0);
    if !(lo - 1e-9..=hi + 1e-9).contains(&theta_e) {
        return Err(format!("θe {theta_e} °C ∉ ⟨{lo}; {hi}⟩"));
    }
    if !(20.0 - 1e-9..=22.0 + 1e-9).contains(&theta_i) {
        return Err(format!("θi {theta_i} °C ∉ ⟨20; 22⟩"));
    }
    let k = rows
        .iter()
        .position(|r| r.0 >= theta_e - 1e-9)
        .unwrap_or(rows.len() - 1);
    let (a, b) = if k == 0 {
        (rows[0], rows[0])
    } else {
        (rows[k - 1], rows[k])
    };
    let s = if (b.0 - a.0).abs() > 1e-12 {
        (theta_e - a.0) / (b.0 - a.0)
    } else {
        0.0
    };
    let f20 = a.1 + s * (b.1 - a.1);
    let f22 = a.2 + s * (b.2 - a.2);
    Ok(f20 + (theta_i - 20.0) / 2.0 * (f22 - f20))
}

/// The mesh criteria of STN EN ISO 10211 čl. 7.2.7: the heat flow's
/// change (fraction) and fRsi's on halving the cells.
pub fn mesh_limits(tables: &[ParamTable]) -> Option<(f64, f64)> {
    Some((
        value(tables, ISO_10211, "konvergencia_tok", "value")? / 100.0,
        value(tables, ISO_10211, "konvergencia_frsi", "value")?,
    ))
}

/// λ of the soil where nothing else is known (STN EN ISO 10211, 8.2).
pub fn soil_lambda(tables: &[ParamTable]) -> Option<f64> {
    value(tables, ISO_10211_INPUT, "lambda_zemina_default", "value")
}

/// A number written as the whole cell ("17", "0,16"); none for an empty
/// cell, a range or a note ("min. 3,5").
fn exact(text: &str) -> Option<f64> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    t.replace(',', ".")
        .parse()
        .ok()
        .filter(|v: &f64| v.is_finite())
}

/// The values of a cell: one number, or both ends of a range "a–b" (the
/// norm's "from a to b"); none for anything else.
fn values(text: &str) -> Vec<f64> {
    if let Some(v) = exact(text) {
        return vec![v];
    }
    let parts: Vec<&str> = text.split('–').collect();
    if parts.len() == 2 {
        if let (Some(a), Some(b)) = (exact(parts[0]), exact(parts[1])) {
            return vec![a, b];
        }
    }
    vec![]
}

/// A material's diffusion data as the norm's table gives it.
#[derive(Clone, Debug, PartialEq)]
pub struct VapourChoice {
    /// The table and item ("tab. 16, pol. 1.2.1").
    pub source: String,
    /// The group and material with its density or thickness.
    pub name: String,
    /// The diffusion resistance factor μ.
    pub mu: f64,
    /// For a thin layer of table 17 with one thickness d: sd = μ·d, m.
    pub sd: Option<f64>,
    /// The lower or upper end of a range the norm gives.
    pub end: Option<bool>,
}

/// μ (and sd of the thin layers) from STN 73 0540-3 tables 16 and 17, a
/// choice per row, two for a range (its ends: which one holds depends on
/// where the layer lies, so the user picks).
pub fn vapour_choices(tables: &[ParamTable]) -> Vec<VapourChoice> {
    let mut out = vec![];
    for (id, tab, dcol) in [(MATERIALS, "16", None), (THIN_LAYERS, "17", Some("d_mm"))] {
        let Some(t) = table(tables, id) else { continue };
        let col = |name: &str| t.column(name);
        let (Some(cg), Some(cm), Some(cmu)) = (col("skupina"), col("material"), col("mu")) else {
            continue;
        };
        let crho = col("rho");
        let cd = dcol.and_then(col);
        for r in &t.rows {
            let get = |c: Option<usize>| c.and_then(|c| r.get(c)).map_or("", |s| s.trim());
            let mus = values(get(Some(cmu)));
            if mus.is_empty() {
                continue;
            }
            let d = cd.map(|c| get(Some(c)));
            let mut name = format!("{} – {}", get(Some(cg)), get(Some(cm)));
            let rho = get(crho);
            if !rho.is_empty() {
                name += &format!(", ρ {rho} kg/m³");
            }
            if let Some(d) = d.filter(|d| !d.is_empty()) {
                name += &format!(", d {d} mm");
            }
            let d_m = d.and_then(exact).map(|d| d / 1000.0);
            let range = mus.len() == 2;
            for (k, mu) in mus.iter().enumerate() {
                out.push(VapourChoice {
                    source: format!("STN 73 0540-3, tab. {tab}, pol. {}", r[0]),
                    name: name.clone(),
                    mu: *mu,
                    sd: d_m.map(|d| mu * d),
                    end: range.then_some(k == 1),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_formula_numbers() {
        assert_eq!(
            magnus("610,5·exp(17,269·θ/(237,3+θ))"),
            Some((610.5, 17.269, 237.3))
        );
    }

    #[test]
    fn reads_ranges_and_thin_layers() {
        assert_eq!(values("17"), vec![17.0]);
        assert_eq!(values("2,5–6,3"), vec![2.5, 6.3]);
        assert!(values("min. 3,5").is_empty() && values("").is_empty());
        let t = fireparams::parse(
            "#table tenke_vrstvy\n#columns pol\tskupina\tmaterial\trho\td_mm\tmu\tdelta_s\tpozn\n\
             1.35\tFólie\tPE fólia\t\t0,1\t124000–164000\t\t\n\
             1.11\tPásy\tBitagit\t1345\tmin. 3,5\t13500\t\t\n",
        );
        let c = vapour_choices(&t);
        assert_eq!(c.len(), 3);
        assert!((c[0].sd.unwrap() - 12.4).abs() < 1e-9 && c[0].end == Some(false));
        assert!((c[1].sd.unwrap() - 16.4).abs() < 1e-9 && c[1].end == Some(true));
        assert_eq!(c[2].sd, None);
    }
}
