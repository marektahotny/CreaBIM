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

/// Does the maker of `pipe_type` give the ζ of its fittings?
pub fn known(pipe_type: &str) -> bool {
    zeta(pipe_type, Part::Elbow, 20.0, false).is_some()
}

/// The fitting at the start of each stretch of a tree, where it joins
/// its parent: a tee (through or branch) where the parent has more than
/// one stretch hanging on it, an elbow where the way turns more than
/// 60°, a coupling where another pipe carries straight on; none at the
/// root and where a pipe goes on past a tap. A change of size is
/// decided with the sizes (see [`stretch_zeta`]).
pub fn joints(tree: &crate::tree::Tree) -> Vec<Option<Part>> {
    let kids = tree.children();
    let dir = |i: usize| {
        let st = &tree.stretches[i];
        (st.b - st.a).normalize_or_zero()
    };
    tree.stretches
        .iter()
        .enumerate()
        .map(|(i, st)| {
            let p = st.parent?;
            let straight = dir(i).dot(dir(p)) > 0.5f64.sqrt() * 1.2;
            if kids[p].len() > 1 {
                // the one going on straightest is the through way
                let through = kids[p]
                    .iter()
                    .copied()
                    .max_by(|&a, &b| dir(a).dot(dir(p)).total_cmp(&dir(b).dot(dir(p))));
                return Some(if through == Some(i) && straight {
                    Part::TeeThrough
                } else {
                    Part::TeeBranch
                });
            }
            if dir(i).dot(dir(p)) < 0.5 {
                Some(Part::Elbow)
            } else if tree.stretches[p].id != st.id {
                Some(Part::Coupling)
            } else {
                None
            }
        })
        .collect()
}

/// The maker's ζ at the start of stretch `i` of `pipe_type`, `nominal`
/// its size and `parent` its parent's: a coupling or a plain run between
/// two sizes is a reducer. None where the maker gives no ζ (the general
/// share of local losses is taken there); 0 where there is no fitting.
pub fn stretch_zeta(
    pipe_type: &str,
    joint: Option<Part>,
    nominal: f64,
    parent: Option<f64>,
) -> Option<f64> {
    if !known(pipe_type) {
        return None;
    }
    let reduced = parent.is_some_and(|p| (p - nominal).abs() > 0.5);
    let part = match joint {
        Some(Part::Coupling) | None if reduced => Part::Reducer,
        Some(p) => p,
        None => return Some(0.0),
    };
    // a size the maker lists no reducer for: as a coupling
    zeta(pipe_type, part, nominal, reduced)
        .or_else(|| zeta(pipe_type, Part::Coupling, nominal, false))
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
        assert!(known("HERZG") && !known("Cu"));
        assert_eq!(stretch_zeta("HERZ", None, 20.0, Some(20.0)), Some(0.0));
        assert_eq!(stretch_zeta("HERZ", None, 16.0, Some(20.0)), Some(4.0));
        assert_eq!(
            stretch_zeta("HERZG", Some(Part::Elbow), 20.0, Some(20.0)),
            Some(6.6)
        );
        assert_eq!(stretch_zeta("Cu", Some(Part::Elbow), 18.0, None), None);
    }
}
