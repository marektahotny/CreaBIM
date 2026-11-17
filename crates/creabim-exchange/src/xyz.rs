//! Reading surveyed points (XYZ, TXT, PTS, CSV), ported from
//! PilonDesigner's `parseXyz`: a point per line, its first three numbers
//! (separated by spaces, tabs, semicolons or commas) its x, y and height;
//! a leading point number is skipped. Coordinates in S-JTSK (both over
//! 50 km) are turned to east and north (Y, X positive → −Y, −X), as
//! PilonDesigner does. Many points are thinned on a grid, the one nearest
//! each cell's middle kept, so a terrain made from them stays quick.

use glam::{DVec2, DVec3};

/// Most points a terrain is made from (PilonDesigner's).
pub const MAX_POINTS: usize = 1500;

/// Points read.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Points {
    pub points: Vec<DVec3>,
    /// Lines that held no point (headers, notes).
    pub skipped: usize,
    /// They were in S-JTSK.
    pub jtsk: bool,
}

/// Is (x, y) in S-JTSK, kilometres from any project's origin?
pub fn is_jtsk(x: f64, y: f64) -> bool {
    x.abs() > 50_000.0 && y.abs() > 50_000.0
}

/// The points of a survey file's text.
pub fn read(text: &str) -> Points {
    let mut out = Points::default();
    for line in text.lines() {
        let nums: Vec<f64> = line
            .split(|c: char| c.is_whitespace() || c == ';' || c == ',')
            .filter(|t| !t.is_empty())
            .map_while(|t| t.parse::<f64>().ok())
            .collect();
        if nums.len() < 3 {
            if !line.trim().is_empty() {
                out.skipped += 1;
            }
            continue;
        }
        // "number Y X Z …": a whole first number before two coordinates
        let k = usize::from(
            nums.len() >= 4
                && nums[0].fract() == 0.0
                && nums[0].abs() < 1e6
                && is_jtsk(nums[1], nums[2])
                && !is_jtsk(nums[0], nums[1]),
        );
        let (mut x, mut y, z) = (nums[k], nums[k + 1], nums[k + 2]);
        if !(x.is_finite() && y.is_finite() && z.is_finite()) {
            out.skipped += 1;
            continue;
        }
        if is_jtsk(x, y) {
            out.jtsk = true;
            if x > 0.0 {
                (x, y) = (-x, -y);
            }
        }
        out.points.push(DVec3::new(x, y, z));
    }
    out
}

/// At most about `max` of `points`, one per cell of a grid over them: the
/// one nearest its cell's middle.
pub fn thin(points: &[DVec3], max: usize) -> Vec<DVec3> {
    if points.len() <= max || max == 0 {
        return points.to_vec();
    }
    let (lo, hi) = points.iter().fold(
        (DVec2::splat(f64::INFINITY), DVec2::splat(f64::NEG_INFINITY)),
        |(lo, hi), p| (lo.min(p.truncate()), hi.max(p.truncate())),
    );
    let span = hi - lo;
    let cell = ((span.x * span.y) / max as f64).sqrt();
    let cell = if cell > 0.0 { cell } else { 1.0 };
    let nx = ((span.x / cell).ceil() as usize).max(1);
    let mut best: std::collections::HashMap<usize, (f64, usize)> = Default::default();
    for (n, q) in points.iter().enumerate() {
        let i = (((q.x - lo.x) / cell).floor() as usize).min(nx - 1);
        let j = ((q.y - lo.y) / cell).floor() as usize;
        let mid = lo + DVec2::new(i as f64 + 0.5, j as f64 + 0.5) * cell;
        let d = q.truncate().distance_squared(mid);
        let e = best.entry(j * nx + i).or_insert((f64::INFINITY, n));
        if d < e.0 {
            *e = (d, n);
        }
    }
    let mut keep: Vec<usize> = best.into_values().map(|(_, n)| n).collect();
    keep.sort_unstable();
    keep.into_iter().map(|n| points[n]).collect()
}

/// The convex hull of points in plan, counter-clockwise (Andrew's
/// monotone chain); the indices of `points` on it.
pub fn hull(points: &[DVec3]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..points.len()).collect();
    idx.sort_by(|&a, &b| {
        let (p, q) = (points[a], points[b]);
        p.x.total_cmp(&q.x).then(p.y.total_cmp(&q.y))
    });
    idx.dedup_by(|a, b| points[*a].truncate().distance(points[*b].truncate()) < 1e-9);
    if idx.len() < 3 {
        return idx;
    }
    let cross = |o: usize, a: usize, b: usize| {
        let (o, a, b) = (
            points[o].truncate(),
            points[a].truncate(),
            points[b].truncate(),
        );
        (a - o).perp_dot(b - o)
    };
    let mut h: Vec<usize> = Vec::with_capacity(idx.len() * 2);
    for pass in 0..2 {
        let start = h.len();
        let it: Box<dyn Iterator<Item = &usize>> = if pass == 0 {
            Box::new(idx.iter())
        } else {
            Box::new(idx.iter().rev())
        };
        for &i in it {
            while h.len() >= start + 2 && cross(h[h.len() - 2], h[h.len() - 1], i) <= 0.0 {
                h.pop();
            }
            h.push(i);
        }
        h.pop();
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn points_are_read_with_their_numbers_and_headers_left_out() {
        let p = read(
            "Y;X;Z\n\
             1 523456.12 1234567.30 302.15\n\
             2;523460.00;1234570.00;302.90\n\
             \n\
             note\n",
        );
        assert_eq!(p.points.len(), 2);
        assert_eq!(p.skipped, 2);
        assert!(p.jtsk);
        // S-JTSK turned to east and north
        assert_eq!(p.points[0], DVec3::new(-523456.12, -1234567.30, 302.15));
        let local = read("0 0 1.5\n10,0,2\n0\t10\t2.5 7");
        assert!(!local.jtsk);
        assert_eq!(local.points[2], DVec3::new(0.0, 10.0, 2.5));
    }

    #[test]
    fn many_points_are_thinned_on_a_grid() {
        let pts: Vec<DVec3> = (0..10_000)
            .map(|i| DVec3::new((i % 100) as f64 * 0.5, (i / 100) as f64 * 0.5, 1.0))
            .collect();
        let t = thin(&pts, 400);
        assert!(t.len() <= 450 && t.len() >= 350, "{}", t.len());
        assert_eq!(thin(&pts[..10], 400).len(), 10);
    }

    #[test]
    fn the_hull_goes_round_the_outermost_points() {
        let pts = [
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::new(4.0, 0.0, 0.0),
            DVec3::new(2.0, 1.0, 0.0),
            DVec3::new(4.0, 4.0, 0.0),
            DVec3::new(0.0, 4.0, 0.0),
        ];
        assert_eq!(hull(&pts), [0, 1, 3, 4]);
    }
}
