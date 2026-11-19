//! The allowed time of evacuation on escape routes, minutes: the table
//! of the annex to decree 94/2004 Z. z. (as amended to 1. 1. 2019),
//! "Dovolený čas evakuácie osôb na únikových cestách". Values between
//! the rows of an unprotected route are interpolated, as its note allows.

/// The escape route the time is asked for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Route {
    /// Unprotected, in a fire zone with the coefficient of combustibles `a`.
    UnprotectedA(f64),
    /// Unprotected, in a fire zone with the probability of a fire's
    /// start and spread `p1`.
    UnprotectedP1(f64),
    /// Partly protected, by § 51 par. 4 letter a), b) or c).
    PartlyA,
    PartlyB,
    PartlyC,
    /// Protected, of type A, B or C.
    ProtectedA,
    ProtectedB,
    ProtectedC,
}

/// (the value, the time with one route, the time with more)
const BY_A: [(f64, f64, f64); 4] = [
    (0.7, 3.0, 5.0),
    (0.9, 2.3, 4.0),
    (1.1, 1.7, 3.5),
    (1.3, 1.0, 2.5),
];
const OVER_A: (f64, f64) = (0.75, 1.5);
const BY_P1: [(f64, f64, f64); 3] = [(0.55, 3.0, 5.0), (1.2, 2.5, 4.0), (2.7, 1.5, 2.5)];
const OVER_P1: (f64, f64) = (0.75, 1.5);

fn between(rows: &[(f64, f64, f64)], over: (f64, f64), v: f64, single: bool) -> f64 {
    let pick = |r: &(f64, f64, f64)| if single { r.1 } else { r.2 };
    let (first, last) = (rows[0], rows[rows.len() - 1]);
    if v <= first.0 {
        return pick(&first);
    }
    if v > last.0 {
        return if single { over.0 } else { over.1 };
    }
    for w in rows.windows(2) {
        if v <= w[1].0 {
            let t = (v - w[0].0) / (w[1].0 - w[0].0);
            return pick(&w[0]) + (pick(&w[1]) - pick(&w[0])) * t;
        }
    }
    pick(&last)
}

/// The allowed time of evacuation on `route`, minutes; `single` when it
/// is the only escape route. None where the table has no value (a partly
/// protected route by letter c) as the only one).
pub fn allowed_time(route: Route, single: bool) -> Option<f64> {
    let pair = |one: f64, more: f64| Some(if single { one } else { more });
    match route {
        Route::UnprotectedA(a) => Some(between(&BY_A, OVER_A, a, single)),
        Route::UnprotectedP1(p) => Some(between(&BY_P1, OVER_P1, p, single)),
        Route::PartlyA => pair(4.0, 6.0),
        Route::PartlyB => pair(3.0, 4.0),
        Route::PartlyC => (!single).then_some(6.0),
        Route::ProtectedA => pair(6.0, 10.0),
        Route::ProtectedB => pair(15.0, 20.0),
        Route::ProtectedC => pair(30.0, 30.0),
    }
}

/// The time less what an escape ladder takes: half a minute for every
/// started 8 m of height it overcomes.
pub fn with_ladder(time: f64, height: f64) -> f64 {
    (time - 0.5 * (height / 8.0).ceil().max(0.0)).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_table_s_rows_come_back_and_values_between_are_interpolated() {
        assert_eq!(allowed_time(Route::UnprotectedA(0.5), true), Some(3.0));
        assert_eq!(allowed_time(Route::UnprotectedA(0.9), false), Some(4.0));
        assert_eq!(allowed_time(Route::UnprotectedA(1.5), true), Some(0.75));
        let t = allowed_time(Route::UnprotectedA(1.0), true).unwrap();
        assert!((t - 2.0).abs() < 1e-9);
        assert_eq!(allowed_time(Route::UnprotectedP1(1.2), true), Some(2.5));
        assert_eq!(allowed_time(Route::UnprotectedP1(3.0), false), Some(1.5));
        assert_eq!(allowed_time(Route::PartlyC, true), None);
        assert_eq!(allowed_time(Route::ProtectedB, false), Some(20.0));
        assert_eq!(with_ladder(3.0, 9.0), 2.0);
    }
}
