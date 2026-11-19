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

/// The fire resistance a roof's assembly states (its description, as
/// "REI 30" or "EI 15"), minutes; none where it states none.
pub fn roof_declared(model: &Model, roof: ElementId) -> Option<f64> {
    let ElementKind::Roof(r) = &model.get(roof)?.kind else {
        return None;
    };
    let ElementKind::Assembly(a) = &model.get(r.assembly?)?.kind else {
        return None;
    };
    declared(&a.description)
}

/// The minutes after a fire resistance class (R, RE, REI, EI, E, EW…).
pub fn declared(text: &str) -> Option<f64> {
    let words: Vec<&str> = text
        .split(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '/')
        .filter(|w| !w.is_empty())
        .collect();
    words.windows(2).find_map(|w| {
        let class = w[0].trim_matches(|c: char| !c.is_alphanumeric());
        let is_class = !class.is_empty()
            && class.chars().all(|c| "REIWM".contains(c))
            && class.contains(['R', 'E', 'I']);
        let digits: String = w[1].chars().take_while(|c| c.is_ascii_digit()).collect();
        is_class.then(|| digits.parse::<f64>().ok()).flatten()
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
