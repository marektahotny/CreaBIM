//! The design (peak) flow of a pipe from the fixtures it feeds: not all of
//! them run at once.

use serde::{Deserialize, Serialize};

/// How the design flow is found.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeakFlowMethod {
    /// DIN 1988-300 from ΣqA, qA = LU · 0.1 l/s (STN EN 806-3).
    #[default]
    #[serde(rename = "DIN1988_300")]
    Din1988_300,
    /// STN 73 6655: √Σ(qA²·n) in dwellings, Σ(qA·φ·n) elsewhere.
    #[serde(rename = "STN736655")]
    Stn736655,
    /// PN-92/B-01706.
    #[serde(rename = "PN_B_01706")]
    PnB01706,
    /// A table ΣLU → QD given in the settings (STN EN 806-3).
    TabulkaLU,
}

/// The building's use, which sets how many fixtures run at once.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BuildingType {
    /// Dwellings.
    #[default]
    Bytovy,
    /// Offices.
    Administrativny,
    /// Schools.
    Skola,
    Hotel,
    /// Hospitals.
    Nemocnica,
    /// Care homes.
    DomovDochodcov,
}

/// DIN 1988-300's coefficients a, b, c for QD = a·(ΣqA)^b − c.
pub fn din_coefficients(t: BuildingType) -> (f64, f64, f64) {
    match t {
        BuildingType::Administrativny | BuildingType::Skola => (0.91, 0.31, 0.38),
        BuildingType::Hotel => (0.7, 0.48, 0.13),
        BuildingType::Nemocnica => (0.75, 0.44, 0.18),
        BuildingType::DomovDochodcov => (1.4, 0.14, 0.92),
        BuildingType::Bytovy => (1.48, 0.19, 0.94),
    }
}

/// What a node's fixtures add up to.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sums {
    pub sum_qa: f64,
    pub max_qa: f64,
    pub sum_qa_stn_sq: f64,
    pub sum_lu: f64,
    /// Σ(qA·φ), STN 73 6655 outside dwellings.
    pub sum_qa_phi: f64,
    pub max_qa_stn: f64,
}

/// The design flow, l/s.
pub fn compute(m: PeakFlowMethod, bt: BuildingType, s: Sums, table: &[[f64; 2]]) -> f64 {
    if s.sum_qa <= 0.0 && s.sum_qa_stn_sq <= 0.0 {
        return 0.0;
    }
    let din = || {
        let (a, b, c) = din_coefficients(bt);
        if s.sum_qa <= 0.2 {
            s.sum_qa
        } else {
            a * s.sum_qa.powf(b) - c
        }
    };
    let q = match m {
        PeakFlowMethod::Stn736655 => {
            return if bt != BuildingType::Bytovy && s.sum_qa_phi >= 0.0 {
                s.sum_qa_phi.max(s.max_qa_stn)
            } else {
                s.sum_qa_stn_sq.max(0.0).sqrt()
            };
        }
        PeakFlowMethod::PnB01706 => {
            if s.sum_qa <= 20.0 {
                0.682 * s.sum_qa.powf(0.45) - 0.14
            } else {
                1.7 * s.sum_qa.powf(0.45) - 0.7
            }
        }
        PeakFlowMethod::TabulkaLU if table.len() >= 2 => interpolate(table, s.sum_lu),
        _ => din(),
    };
    q.max(s.max_qa).min(s.sum_qa)
}

/// Log–log interpolation in a ΣLU → QD table, extrapolated at its ends.
fn interpolate(t: &[[f64; 2]], x: f64) -> f64 {
    let mut pts: Vec<[f64; 2]> = t
        .iter()
        .copied()
        .filter(|r| r[0] > 0.0 && r[1] > 0.0)
        .collect();
    pts.sort_by(|a, b| a[0].total_cmp(&b[0]));
    if pts.is_empty() {
        return 0.0;
    }
    if x <= pts[0][0] {
        return pts[0][1] * x / pts[0][0];
    }
    for w in pts.windows(2) {
        if x <= w[1][0] {
            let (lx0, lx1) = (w[0][0].ln(), w[1][0].ln());
            let (ly0, ly1) = (w[0][1].ln(), w[1][1].ln());
            let f = (x.ln() - lx0) / (lx1 - lx0);
            return (ly0 + f * (ly1 - ly0)).exp();
        }
    }
    if pts.len() < 2 {
        return pts[0][1];
    }
    let (a, b) = (pts[pts.len() - 2], pts[pts.len() - 1]);
    let slope = (b[1].ln() - a[1].ln()) / (b[0].ln() - a[0].ln());
    (b[1].ln() + slope * (x.ln() - b[0].ln())).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sums(sum_qa: f64, max_qa: f64) -> Sums {
        Sums {
            sum_qa,
            max_qa,
            sum_qa_stn_sq: 0.0,
            sum_lu: sum_qa * 10.0,
            sum_qa_phi: -1.0,
            max_qa_stn: 0.0,
        }
    }

    #[test]
    fn din_in_a_dwelling() {
        // 10 LU = 1 l/s: 1.48 · 1^0.19 − 0.94 = 0.54 l/s
        let q = compute(
            PeakFlowMethod::Din1988_300,
            BuildingType::Bytovy,
            sums(1.0, 0.2),
            &[],
        );
        assert!((q - 0.54).abs() < 1e-12);
        // never more than all of them, never less than the largest
        let q = compute(
            PeakFlowMethod::Din1988_300,
            BuildingType::Bytovy,
            sums(0.3, 0.3),
            &[],
        );
        assert_eq!(q, 0.3);
    }

    #[test]
    fn stn_736655_squares_in_dwellings_and_sums_by_phi_elsewhere() {
        let s = Sums {
            sum_qa: 0.4,
            max_qa: 0.2,
            sum_qa_stn_sq: 0.2 * 0.2 * 2.0,
            sum_lu: 4.0,
            sum_qa_phi: 0.2 * 0.8 + 0.2 * 0.3,
            max_qa_stn: 0.2,
        };
        let q = compute(PeakFlowMethod::Stn736655, BuildingType::Bytovy, s, &[]);
        assert!((q - (0.08f64).sqrt()).abs() < 1e-12);
        let q = compute(PeakFlowMethod::Stn736655, BuildingType::Skola, s, &[]);
        assert!((q - 0.22).abs() < 1e-12);
    }

    #[test]
    fn a_table_interpolates_on_logs() {
        let t = [[10.0, 0.5], [100.0, 1.5]];
        let q = interpolate(&t, 10f64.powf(1.5));
        assert!((q - (0.5f64 * 1.5).sqrt()).abs() < 1e-12);
    }
}
