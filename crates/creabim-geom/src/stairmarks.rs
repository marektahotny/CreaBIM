//! A stair's marks in a floor plan as STN 01 3420 (2008), 5.9 draws them
//! (George 2026-11-24): the walking line from a circle on the first
//! nosing to an arrow on the last (5.9.4), the risers' height and the
//! going written over it as `h × b` (5.9.5 c), the number of the first
//! and the last step of each flight by their nosings (5.9.9, 5.9.10) and
//! the line where the plan's cut crosses a straight flight, thin and at
//! 30° (5.9.3 a).

use crate::plan::{Pen, PlanItem};
use creabim_core::{Element, ElementKind, Model, Stair};
use glam::DVec2;

/// The circle on the walking line's first nosing, m.
const CIRCLE: f64 = 0.075;
/// How far beyond the flight's sides the cut line runs, m.
const CUT_PAST: f64 = 0.1;
/// The paper height of the stair's numbers and `h × b`, mm.
const TEXT: f64 = 2.0;

/// A text of the stair's at 1:100 ([`size_texts`] sets the view's).
fn text(e: &Element, at: DVec2, text: String) -> PlanItem {
    PlanItem::Text {
        id: e.id,
        at,
        text,
        size: TEXT,
        align: crate::paper::Anchor::Middle,
        mm: 0.1,
        rgb: None,
        bold: false,
    }
}

/// The stairs' texts of a drawing at its own scale, `mm` metres per
/// paper millimetre.
pub fn size_texts(model: &Model, items: &mut [PlanItem], mm: f64) {
    for it in items {
        if let PlanItem::Text { id, mm: m, .. } = it {
            if matches!(model.get(*id).map(|e| &e.kind), Some(ElementKind::Stair(_))) {
                *m = mm;
            }
        }
    }
}

/// The circle at the start of the walking line, as segments.
pub fn start_circle(at: DVec2) -> Vec<(DVec2, DVec2)> {
    let n = 20;
    let p = |i: usize| at + DVec2::from_angle(i as f64 / n as f64 * std::f64::consts::TAU) * CIRCLE;
    (0..n).map(|i| (p(i), p(i + 1))).collect()
}

/// The thin straight line where the plan cuts a straight flight, across
/// it at 30° to its nosings (STN 01 3420, 5.9.3 a): through the flight's
/// axis at `mid`, along `dir` up the flight, `width` wide.
pub fn cut_line(mid: DVec2, dir: DVec2, width: f64) -> (DVec2, DVec2) {
    let across = dir.perp();
    // 30° off the nosings: along the flight tan 30° for each across
    let slant = (30f64).to_radians().tan();
    let half = width / 2.0 + CUT_PAST;
    let a = mid - across * half - dir * half * slant;
    let b = mid + across * half + dir * half * slant;
    (a, b)
}

/// The first step's number of `stair`: the steps of the stairs under it
/// in the same stair space (lower storeys, footprints overlapping) come
/// first (5.9.9).
pub fn first_number(model: &Model, id: creabim_core::ElementId, s: &Stair) -> u32 {
    let level_z = |l| model.level(l).map_or(0.0, |l| l.elevation);
    let bounds = |st: &Stair| {
        let rise = st.rise(model);
        let pts: Vec<DVec2> = st.flight_outlines(rise).into_iter().flatten().collect();
        pts.iter().fold(
            (DVec2::splat(f64::MAX), DVec2::splat(f64::MIN)),
            |(lo, hi), p| (lo.min(*p), hi.max(*p)),
        )
    };
    let (lo, hi) = bounds(s);
    let z = level_z(s.level) + s.base_offset;
    let mut under = 0;
    for e in model.iter() {
        let ElementKind::Stair(o) = &e.kind else {
            continue;
        };
        if e.id == id || o.ramp.is_some() || level_z(o.level) + o.base_offset >= z - 1e-6 {
            continue;
        }
        let (olo, ohi) = bounds(o);
        let overlap = olo.x < hi.x && lo.x < ohi.x && olo.y < hi.y && lo.y < ohi.y;
        if overlap {
            under += o.layout(o.rise(model)).risers;
        }
    }
    under + 1
}

/// The marks of the norm on a stair's plan: the circle, the arrow is
/// drawn with the walking line; here the circle, `h × b` over the walking
/// line and the numbers at each flight's first and last nosing. `walk` is
/// the walking line; `flights` each flight's nosings, first to last, with
/// the direction up it; `first` the number of the stair's first step.
pub fn marks(
    items: &mut Vec<PlanItem>,
    e: &Element,
    walk: &[DVec2],
    flights: &[(Vec<[DVec2; 2]>, DVec2)],
    riser: f64,
    going: f64,
    first: u32,
) {
    if let Some(&p0) = walk.first() {
        for (a, b) in start_circle(p0) {
            items.push(PlanItem::Line {
                id: e.id,
                pen: Pen::Symbol,
                a,
                b,
            });
        }
    }
    // h × b over the walking line, low on its first flight (else its
    // longest piece), clear of the cut line
    let first_piece = walk
        .windows(2)
        .next()
        .filter(|w| w[0].distance(w[1]) >= 1.0);
    let longest = || {
        walk.windows(2)
            .max_by(|x, y| x[0].distance(x[1]).total_cmp(&y[0].distance(y[1])))
    };
    if let Some(w) = first_piece.or_else(longest) {
        let d = (w[1] - w[0]).normalize_or_zero();
        // over the line as it reads: on its upper side on the sheet
        let mut n = d.perp();
        if n.y < 0.0 || (n.y.abs() < 1e-9 && n.x > 0.0) {
            n = -n;
        }
        items.push(text(
            e,
            w[0].lerp(w[1], 0.3) + n * 0.15,
            format!(
                "{:.0} × {:.0}",
                (riser * 1000.0).round(),
                (going * 1000.0).round()
            ),
        ));
    }
    // the numbers by each flight's first and last nosing
    let mut k = first;
    for (nosings, dir) in flights {
        let count = nosings.len() as u32;
        if count == 0 {
            continue;
        }
        let ends = if count == 1 {
            vec![(0usize, k)]
        } else {
            vec![(0usize, k), (nosings.len() - 1, k + count - 1)]
        };
        for (i, num) in ends {
            let [a, b] = nosings[i];
            // by the nosing's left end, on the step below it
            let at = a + (b - a) * 0.12 - *dir * 0.1;
            items.push(text(e, at, num.to_string()));
        }
        k += count;
    }
}
