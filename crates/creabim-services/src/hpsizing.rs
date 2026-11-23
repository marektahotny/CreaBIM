//! Sizing an air-to-water heat pump, as PilonDesigner's `hpsizing.ts`
//! (bivalent or mono-energetic running, after STN EN 12831-1, STN EN
//! 15450 and VDI 4645): a heat pump's output falls with the outside
//! temperature, so its curve is set against the building's need as the
//! outside temperature goes. The need grows by hot water and by the
//! hours the supply is blocked. Where the two cross is the bivalence
//! point; below it the electric heater makes up the rest. A boiler is
//! sized straight on the design heat load: its output does not depend
//! on the weather.

use creabim_core::catalogue::heat::{backup_kw, has_curve, hp_curve, HeatKind};

/// Hot water's share, kW a person (VDI 4645).
pub const DHW_KW_PER_PERSON: f64 = 0.25;
/// Heating stops at this outside temperature, °C.
pub const T_HEAT_LIMIT: f64 = 15.0;
/// The heat pump runs down to this outside temperature, °C (Panasonic
/// Aquarea J: −20 to +35 °C).
pub const T_HP_MIN: f64 = -20.0;

/// What the heat pump is sized for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HpInput {
    /// Design heat load, kW (STN EN 12831-1).
    pub heat_load: f64,
    /// Design outside temperature θe, °C.
    pub te: f64,
    /// Flow temperature, °C: underfloor 35, low-temperature radiators
    /// 45, radiators 55.
    pub t_flow: f64,
    /// People, for hot water.
    pub persons: f64,
    /// Hours a day the supply is blocked.
    pub block_h: f64,
    /// The bivalence point aimed at, °C.
    pub t_biv: f64,
}

impl Default for HpInput {
    fn default() -> Self {
        Self {
            heat_load: 8.0,
            te: -13.0,
            t_flow: 35.0,
            persons: 4.0,
            block_h: 2.0,
            t_biv: -7.0,
        }
    }
}

/// The output needed at outside temperature `t`, kW: heating falling in
/// a line to the heating limit, plus hot water, all over the hours the
/// supply is on.
pub fn need(i: &HpInput, t: f64) -> f64 {
    let heat = if t >= T_HEAT_LIMIT {
        0.0
    } else {
        i.heat_load * (T_HEAT_LIMIT - t) / (T_HEAT_LIMIT - i.te)
    };
    (heat + i.persons * DHW_KW_PER_PERSON) * (24.0 / (24.0 - i.block_h.clamp(0.0, 8.0)))
}

/// The heat pump's output at outside temperature `t`, kW: between the
/// curve's points in a line, beyond them along the end ones, constant
/// over the last.
pub fn capacity(pts: &[(f64, f64)], t: f64) -> f64 {
    if pts.is_empty() || t < T_HP_MIN {
        return 0.0;
    }
    let last = pts[pts.len() - 1];
    if pts.len() == 1 || t >= last.0 {
        return last.1;
    }
    let mut i = 0;
    while i < pts.len() - 2 && t > pts[i + 1].0 {
        i += 1;
    }
    let (a, b) = (pts[i], pts[i + 1]);
    (a.1 + (b.1 - a.1) * (t - a.0) / (b.0 - a.0)).max(0.0)
}

/// How a heat pump fares against the need.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HpResult {
    /// Its output at θe, kW.
    pub q_te: f64,
    /// The need at θe with hot water and blocking, kW.
    pub need_te: f64,
    /// The bivalence point, °C; none when it covers the need down to
    /// θe on its own.
    pub t_biv: Option<f64>,
    /// The electric heating needed at θe, kW.
    pub backup: f64,
}

pub fn size(i: &HpInput, pts: &[(f64, f64)]) -> HpResult {
    let (q_te, need_te) = (capacity(pts, i.te), need(i, i.te));
    let mut t_biv = None;
    if q_te < need_te {
        let (mut lo, mut hi) = (i.te, T_HEAT_LIMIT);
        if capacity(pts, hi) < need(i, hi) {
            t_biv = Some(hi);
        } else {
            for _ in 0..40 {
                let m = (lo + hi) / 2.0;
                if capacity(pts, m) >= need(i, m) {
                    hi = m;
                } else {
                    lo = m;
                }
            }
            t_biv = Some((hi * 10.0).round() / 10.0);
        }
    }
    HpResult {
        q_te,
        need_te,
        t_biv,
        backup: (need_te - q_te).max(0.0),
    }
}

/// The size to take from the range: the smallest whose bivalence point
/// is at or under the one aimed at and whose electric heater covers the
/// rest at θe; the largest when none does. Only kits whose catalogue
/// curve down to A-7 is known are taken.
pub fn recommend(i: &HpInput) -> u32 {
    let sizes: Vec<u32> = HeatKind::HpOutdoor
        .sizes()
        .iter()
        .copied()
        .filter(|s| has_curve(*s))
        .collect();
    for s in &sizes {
        let r = size(i, &hp_curve(*s, i.t_flow));
        if r.t_biv.is_none_or(|t| t <= i.t_biv) && r.backup <= backup_kw(*s) + 1e-9 {
            return *s;
        }
    }
    sizes[sizes.len() - 1]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_need_takes_hot_water_and_blocking() {
        let i = HpInput::default();
        // 8 kW + 4 × 0.25 kW over 22 of 24 hours
        assert!((need(&i, -13.0) - 9.0 * 24.0 / 22.0).abs() < 1e-12);
        assert!((need(&i, 15.0) - 24.0 / 22.0).abs() < 1e-12);
        assert!((need(&i, 1.0) - (4.0 + 1.0) * 24.0 / 22.0).abs() < 1e-12);
    }

    #[test]
    fn the_curve_is_read_between_and_beyond_its_points() {
        // KIT-WC09J3E5 W35: A-7 6,12, A2 7,00, A7 9,00 kW
        let pts = hp_curve(9, 35.0);
        assert_eq!(capacity(&pts, 7.0), 9.0);
        assert_eq!(capacity(&pts, 20.0), 9.0);
        assert!((capacity(&pts, -2.5) - (6.12 + 0.88 * 0.5)).abs() < 1e-12);
        // under A-7 held, under −20 °C off
        assert!((capacity(&pts, -15.0) - 6.12).abs() < 1e-12);
        assert_eq!(capacity(&pts, -21.0), 0.0);
    }

    #[test]
    fn the_smallest_with_its_bivalence_point_low_enough_is_taken() {
        let i = HpInput {
            heat_load: 5.0,
            ..HpInput::default()
        };
        let s = recommend(&i);
        let r = size(&i, &hp_curve(s, i.t_flow));
        assert!(r.t_biv.is_none_or(|t| t <= -7.0), "{r:?}");
        assert!(r.backup <= backup_kw(s));
        // the one under it crosses the need above −7 °C
        let sizes = HeatKind::HpOutdoor.sizes();
        let k = sizes.iter().position(|x| *x == s).unwrap();
        assert!(k > 0);
        let under = size(&i, &hp_curve(sizes[k - 1], i.t_flow));
        assert!(under.t_biv.unwrap() > -7.0, "{under:?}");
        if let Some(t) = r.t_biv {
            let pts = hp_curve(s, i.t_flow);
            assert!((capacity(&pts, t) - need(&i, t)).abs() < 0.05);
        }
        assert_eq!(s, 7);
    }

    #[test]
    fn a_big_house_gets_the_largest_with_a_known_curve() {
        let i = HpInput {
            heat_load: 40.0,
            ..HpInput::default()
        };
        assert_eq!(recommend(&i), 9);
    }
}
