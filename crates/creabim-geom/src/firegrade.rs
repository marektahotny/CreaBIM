//! The fire safety grade (stupeň protipožiarnej bezpečnosti, SPB) of a
//! fire zone by STN 92 0201-2, and what it asks of a roof: the least
//! fire resistance of the roof covering (table 5, item 3), short of
//! which the roof is a fire-open area (its note 4)).
//!
//! - Non-production buildings: table 2, by the structural system, the
//!   zone's design fire load pv and the building's fire height h (3.3);
//!   the values in brackets for a mixed system by 2.6.4 b).
//! - Dwellings and lodging of group B: table 3, by the system and the
//!   number of storeys above ground (3.4); group A is grade I.
//!
//! Every value is read from the designer's parameter tables; with none,
//! nothing is worked out.

use creabim_core::fireparams::{self, ParamTable};
use creabim_core::{ElementId, ElementKind, Model};

/// Single values: `rule` (2: table 2; 3: table 3, dwellings of group B;
/// 1: grade I, dwellings of group A), `grade` (a grade given by the
/// designer, 1 to 5, over the rule).
pub const FIRE_GRADE: &str = "fire_grade";
/// Table 2: system (N, Z, H), pv up to, then the greatest fire height
/// for grades I to V ("bez" no limit, "0" single storey, "0a" single
/// storey with a ≤ 1,1, "N1"/"N2" not allowed, "18(12)" the bracketed
/// value for a mixed system by 2.6.4 b)).
pub const FIRE_GRADE_NONPROD: &str = "fire_grade_nonprod";
/// Table 3: system, storeys above ground up to, grade.
pub const FIRE_GRADE_DWELLING: &str = "fire_grade_dwelling";
/// Table 5 item 3: grade, the roof covering's least fire resistance,
/// minutes.
pub const FIRE_ROOF: &str = "fire_roof";
/// Table 5 items 2 and 14: grade, then an outer wall's least fire
/// resistance, minutes: `above` load-bearing in a storey above ground
/// (2a2), `last` load-bearing in the last storey above ground (2a3),
/// `nonbearing` (2b), `single` in a single-storey building (14).
pub const FIRE_OUTER_WALL: &str = "fire_outer_wall";

fn value(t: Option<&ParamTable>, key: &str) -> Option<f64> {
    fireparams::number(t?.cell(key, "value"))
}

/// The building's structural system as the tables name it, and whether
/// it is the mixed one by 2.6.4 b).
fn system(model: &Model) -> (char, bool) {
    let s = model
        .project_info()
        .map(|p| p.1.fire_system.trim().to_lowercase())
        .unwrap_or_default();
    match s.as_str() {
        "za" => ('Z', false),
        "zb" => ('Z', true),
        "ha" | "hb" => ('H', false),
        _ => ('N', false),
    }
}

/// The grade, 1 to 5, of a zone with design fire load `pv`; none where
/// the tables or the figures it needs are missing, or no grade allows it.
pub fn grade(model: &Model, pv: f64) -> Option<u32> {
    let t = crate::firerisk::table(model, FIRE_GRADE)?;
    if let Some(g) = value(Some(t), "grade").filter(|g| (1.0..=5.0).contains(g)) {
        return Some(g.round() as u32);
    }
    let (sys, b) = system(model);
    let fh = crate::firezones::fire_height(model)?;
    match value(Some(t), "rule").map_or(2, |v| v.round() as u32) {
        1 => Some(1),
        3 => {
            let tab = crate::firerisk::table(model, FIRE_GRADE_DWELLING)?;
            let n = |s: &str| fireparams::number(s);
            tab.rows
                .iter()
                .filter(|r| r.first().is_some_and(|s| s.trim().starts_with(sys)))
                .filter_map(|r| Some((n(r.get(1)?)?, n(r.get(2)?)?)))
                .filter(|(up_to, _)| fh.above as f64 <= *up_to + 1e-9)
                .min_by(|a, b| a.0.total_cmp(&b.0))
                .map(|(_, g)| g.round() as u32)
        }
        _ => {
            let tab = crate::firerisk::table(model, FIRE_GRADE_NONPROD)?;
            let single = fh.above <= 1 && fh.below == 0;
            nonprod(tab, sys, b, pv, fh.height, single)
        }
    }
}

/// Table 2: the lowest grade whose greatest fire height takes `h`.
fn nonprod(t: &ParamTable, sys: char, b: bool, pv: f64, h: f64, single: bool) -> Option<u32> {
    let n = |s: &str| fireparams::number(s);
    let row = t
        .rows
        .iter()
        .filter(|r| r.first().is_some_and(|s| s.trim().starts_with(sys)))
        .filter_map(|r| Some((n(r.get(1)?)?, r)))
        .filter(|(up_to, _)| pv <= *up_to + 1e-9)
        .min_by(|a, b| a.0.total_cmp(&b.0))?
        .1;
    (0..5).find_map(|g| {
        let cell = row.get(2 + g)?.trim().to_lowercase();
        // "18(12)": the bracketed value for a mixed system by 2.6.4 b)
        let cell = match (cell.split_once('('), b) {
            (Some((_, inner)), true) => inner.trim_end_matches(')').to_string(),
            (Some((outer, _)), false) => outer.to_string(),
            (None, _) => cell,
        };
        let ok = if cell.starts_with("bez") {
            true
        } else if cell.starts_with('n') {
            false
        } else if cell == "0" || cell == "0a" {
            single
        } else {
            n(&cell).is_some_and(|max| h <= max + 1e-9)
        };
        ok.then_some(g as u32 + 1)
    })
}

/// Table 5 item 3: the roof covering's least fire resistance for
/// `grade`, minutes; none where none is asked (a single-storey building:
/// items 12 to 14 ask nothing of its roof).
pub fn roof_required(model: &Model, grade: u32) -> Option<f64> {
    let fh = crate::firezones::fire_height(model)?;
    if fh.above <= 1 && fh.below == 0 {
        return None;
    }
    let t = crate::firerisk::table(model, FIRE_ROOF)?;
    let n = |s: &str| fireparams::number(s);
    t.rows
        .iter()
        .find(|r| {
            r.first()
                .and_then(|s| n(s))
                .is_some_and(|g| g.round() as u32 == grade)
        })
        .and_then(|r| n(r.get(1)?))
}

/// Table 5 items 2 and 14: an outer wall's least fire resistance for
/// `grade`, minutes, short of which it is a wholly fire-open area (its
/// note 3), 5.4.4); none where the table or the figures are missing, or
/// for a wall under ground.
pub fn wall_required(model: &Model, grade: u32, wall: &creabim_core::Wall) -> Option<f64> {
    let fh = crate::firezones::fire_height(model)?;
    let t = crate::firerisk::table(model, FIRE_OUTER_WALL)?;
    let column = if fh.above <= 1 && fh.below == 0 {
        "single"
    } else {
        let elev = |id: ElementId| model.level(id).map(|l| l.elevation);
        let (here, first) = (elev(wall.level)?, elev(fh.first)?);
        if here < first - 1e-6 {
            return None;
        }
        let bearing =
            creabim_core::masonry::is_bearing(model, wall, &creabim_core::masonry::rules(model));
        if !bearing {
            "nonbearing"
        } else if wall.level == fh.last {
            "last"
        } else {
            "above"
        }
    };
    let i = t.columns.iter().position(|c| c == column)?;
    let n = |s: &str| fireparams::number(s);
    t.rows
        .iter()
        .find(|r| {
            r.first()
                .and_then(|s| n(s))
                .is_some_and(|g| g.round() as u32 == grade)
        })
        .and_then(|r| n(r.get(i)?))
}

/// Table 5 item 2a (a storey above ground) or 14 (a single-storey
/// building): what a zone of `grade` asks of its load-bearing outer
/// walls, minutes; the R of figure 10 (5.4.10).
pub fn outer_wall_minutes(model: &Model, grade: u32) -> Option<f64> {
    let fh = crate::firezones::fire_height(model)?;
    let t = crate::firerisk::table(model, FIRE_OUTER_WALL)?;
    let column = if fh.above <= 1 && fh.below == 0 {
        "single"
    } else {
        "above"
    };
    let i = t.columns.iter().position(|c| c == column)?;
    let n = |s: &str| fireparams::number(s);
    t.rows
        .iter()
        .find(|r| {
            r.first()
                .and_then(|s| n(s))
                .is_some_and(|g| g.round() as u32 == grade)
        })
        .and_then(|r| n(r.get(i)?))
}

/// The fire resistance a wall's assembly states (its description, as
/// "REW 60" or "EW 30"), minutes; none where it states none.
pub fn wall_declared(model: &Model, wall: &creabim_core::Wall) -> Option<f64> {
    let ElementKind::Assembly(a) = &model.get(wall.assembly?)?.kind else {
        return None;
    };
    stated(a)
}

/// The fire resistance a roof's assembly states (its description, as
/// "REI 30" or "EI 15"), minutes; none where it states none.
pub fn roof_declared(model: &Model, roof: ElementId) -> Option<f64> {
    let ElementKind::Roof(r) = &model.get(roof)?.kind else {
        return None;
    };
    let ElementKind::Assembly(a) = &model.get(r.assembly?)?.kind else {
        return None;
    };
    stated(a)
}

/// An assembly's stated fire resistance, minutes: its own field first,
/// else its description.
pub fn stated(a: &creabim_core::Assembly) -> Option<f64> {
    declared(&a.fire_resistance).or_else(|| declared(&a.description))
}

/// The minutes after a fire resistance class (R, RE, REI, EI, E, EW…),
/// with or without a space or an "-M" ("REI-M 240", "REI 240", "REI240",
/// "REI-M240", "REW 90/D1").
pub fn declared(text: &str) -> Option<f64> {
    let is_class = |class: &str| {
        !class.is_empty()
            && class.chars().all(|c| "REIWM-".contains(c))
            && class.contains(['R', 'E', 'I'])
    };
    let minutes = |w: &str| -> Option<f64> {
        let digits: String = w.chars().take_while(|c| c.is_ascii_digit()).collect();
        digits.parse::<f64>().ok()
    };
    let words: Vec<&str> = text
        .split(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '/')
        .filter(|w| !w.is_empty())
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
        .filter(|w| !w.is_empty())
        .collect();
    words.iter().enumerate().find_map(|(i, w)| {
        // the class and its minutes in one word
        let at = w.find(|c: char| c.is_ascii_digit());
        if let Some(at) = at.filter(|&a| a > 0) {
            if is_class(w[..at].trim_end_matches('-')) {
                return minutes(&w[at..]);
            }
        }
        // or the minutes the next word
        (is_class(w) && at.is_none())
            .then(|| words.get(i + 1).and_then(|n| minutes(n)))
            .flatten()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A made-up table in the norm's layout (not its values).
    const TABLE: &str = "#table t\n#columns system\tpv\tI\tII\tIII\tIV\tV\n\
        N\t15\t20\tbez\tbez\tbez\tbez\nN\t999\tN1\t0a\t0\t10\t30\n\
        Z\t10\t10\t10\t15(10)\t20(10)\tN2\n";

    #[test]
    fn a_rating_with_m_is_read() {
        assert_eq!(declared("REI-M 240"), Some(240.0));
        assert_eq!(declared("požiarna odolnosť EI 180 (príručka)"), Some(180.0));
        assert_eq!(declared("REI-M240"), Some(240.0));
        assert_eq!(declared("REI240"), Some(240.0));
        assert_eq!(declared("REW 90/D1"), Some(90.0));
        assert_eq!(declared("EW 30 DP1"), Some(30.0));
        assert_eq!(declared("trieda reakcie A1"), None);
        assert_eq!(declared("PORFIX P2-440 300"), None);
    }

    #[test]
    fn table_2_gives_the_lowest_grade_that_takes_the_height() {
        let t = &fireparams::parse(TABLE)[0];
        assert_eq!(nonprod(t, 'N', false, 10.0, 15.0, false), Some(1));
        assert_eq!(nonprod(t, 'N', false, 10.0, 25.0, false), Some(2));
        assert_eq!(nonprod(t, 'N', false, 200.0, 0.0, true), Some(2));
        assert_eq!(nonprod(t, 'N', false, 200.0, 5.0, false), Some(4));
        assert_eq!(nonprod(t, 'Z', false, 5.0, 12.0, false), Some(3));
        assert_eq!(nonprod(t, 'Z', true, 5.0, 12.0, false), None);
    }

    #[test]
    fn a_stated_fire_resistance_is_read() {
        assert_eq!(
            declared("vonkajšia konštrukcia; požiarna odolnosť REI 30"),
            Some(30.0)
        );
        assert_eq!(declared("EI 15/D1"), Some(15.0));
        assert_eq!(declared("Rw 52 dB"), None);
    }
}
