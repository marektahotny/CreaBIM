//! Local loss coefficients ζ of the makers' own fittings, where their
//! manuals give them: Wavin PP-R/PP-RCT ("Vnitřní instalace", 2026,
//! table of ζ for fittings) and HERZ press fittings for PipeFix and
//! GASPRESS ("Plasthliníková rúrka", ζ for water and for gas, by size).
//! The calculations take these in place of the general ζ of their
//! settings for pipes of these makers.

/// What a fitting is to its ζ.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Coupling,
    Reducer,
    Elbow,
    TeeThrough,
    TeeBranch,
}

/// HERZ press fittings by outside diameter: coupling, elbow, tee
/// through, tee branch; water and gas.
const HERZ_WATER: &[(f64, [f64; 4])] = &[
    (16.0, [4.0, 13.5, 5.0, 15.1]),
    (20.0, [2.1, 8.5, 2.5, 7.5]),
    (26.0, [1.0, 6.8, 1.3, 6.2]),
    (32.0, [0.8, 4.2, 1.0, 4.7]),
];
const HERZ_GAS: &[(f64, [f64; 4])] = &[
    (16.0, [6.2, 15.2, 8.2, 18.7]),
    (20.0, [1.8, 6.6, 2.8, 8.3]),
    (26.0, [1.3, 6.1, 2.3, 7.6]),
    (32.0, [1.3, 5.1, 1.3, 5.8]),
];

/// The maker's ζ of a fitting on a pipe of `pipe_type` and size, where
/// it is known. A reduced tee's branch on Wavin PP-R takes 4,3.
pub fn zeta(pipe_type: &str, part: Part, outer_mm: f64, reduced: bool) -> Option<f64> {
    match pipe_type {
        "WPPR" | "WPPRCT" | "WPPRB" => Some(match part {
            Part::Coupling => 0.2,
            Part::Reducer => 0.55,
            Part::Elbow => 1.5,
            Part::TeeThrough => 1.1,
            Part::TeeBranch if reduced => 4.3,
            Part::TeeBranch => 1.5,
        }),
        "HERZ" | "HERZT" | "HERZG" | "HERZGT" => {
            let table = if pipe_type.starts_with("HERZG") {
                HERZ_GAS
            } else {
                HERZ_WATER
            };
            let i = match part {
                Part::Coupling => 0,
                Part::Elbow => 1,
                Part::TeeThrough => 2,
                Part::TeeBranch => 3,
                Part::Reducer => return None,
            };
            // the nearest size listed: HERZ lists 16 to 32
            let row = table
                .iter()
                .min_by(|a, b| (a.0 - outer_mm).abs().total_cmp(&(b.0 - outer_mm).abs()))?;
            Some(row.1[i])
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_makers_tables_answer() {
        assert_eq!(zeta("WPPR", Part::Elbow, 25.0, false), Some(1.5));
        assert_eq!(zeta("WPPR", Part::TeeBranch, 25.0, true), Some(4.3));
        assert_eq!(zeta("HERZ", Part::Elbow, 20.0, false), Some(8.5));
        assert_eq!(zeta("HERZG", Part::TeeBranch, 16.0, false), Some(18.7));
        assert_eq!(zeta("PPHT", Part::Elbow, 110.0, false), None);
    }
}
