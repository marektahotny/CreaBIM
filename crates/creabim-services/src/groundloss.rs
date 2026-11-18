//! Heat loss to the ground by the simplified method of STN EN 12831
//! annex D.4.3, as PilonDesigner's `groundloss.ts`:
//! H = f_g1 · f_g2 · G_w · Σ(A · U_equiv), f_g1 = 1.45,
//! f_g2 = (θint − θm,e) / (θint − θe), G_w = 1.15 where ground water is
//! within 1 m of the floor. U_equiv of a floor by B′ = A / (0.5 · P)
//! and the floor's U at a depth of 0, 1.5 and 3.0 m; of a basement wall
//! by its U and depth. Outside the tables the end value is taken.

pub const FG1: f64 = 1.45;

const B: [f64; 10] = [2.0, 4.0, 6.0, 8.0, 10.0, 12.0, 14.0, 16.0, 18.0, 20.0];
/// Columns: uninsulated, U 2.0, 1.0, 0.5, 0.25.
const FLOOR_0: [[f64; 5]; 10] = [
    [1.3, 0.77, 0.55, 0.33, 0.17],
    [0.88, 0.59, 0.45, 0.3, 0.17],
    [0.68, 0.48, 0.38, 0.27, 0.17],
    [0.55, 0.41, 0.33, 0.25, 0.16],
    [0.47, 0.36, 0.3, 0.23, 0.15],
    [0.41, 0.32, 0.27, 0.21, 0.14],
    [0.37, 0.29, 0.24, 0.19, 0.14],
    [0.33, 0.26, 0.22, 0.18, 0.13],
    [0.31, 0.24, 0.21, 0.17, 0.12],
    [0.28, 0.22, 0.19, 0.16, 0.12],
];
const FLOOR_15: [[f64; 5]; 10] = [
    [0.86, 0.58, 0.44, 0.28, 0.16],
    [0.64, 0.48, 0.38, 0.26, 0.16],
    [0.52, 0.4, 0.33, 0.25, 0.15],
    [0.44, 0.35, 0.29, 0.23, 0.15],
    [0.38, 0.31, 0.26, 0.21, 0.14],
    [0.34, 0.28, 0.24, 0.19, 0.14],
    [0.3, 0.25, 0.22, 0.18, 0.13],
    [0.28, 0.23, 0.2, 0.17, 0.12],
    [0.25, 0.22, 0.19, 0.16, 0.12],
    [0.24, 0.2, 0.18, 0.15, 0.11],
];
const FLOOR_3: [[f64; 5]; 10] = [
    [0.63, 0.46, 0.35, 0.24, 0.14],
    [0.51, 0.4, 0.33, 0.24, 0.14],
    [0.43, 0.35, 0.29, 0.22, 0.14],
    [0.37, 0.31, 0.26, 0.21, 0.14],
    [0.32, 0.27, 0.24, 0.19, 0.13],
    [0.29, 0.25, 0.22, 0.18, 0.13],
    [0.26, 0.23, 0.2, 0.17, 0.12],
    [0.24, 0.21, 0.19, 0.16, 0.12],
    [0.22, 0.2, 0.18, 0.15, 0.11],
    [0.21, 0.18, 0.16, 0.14, 0.11],
];

/// Between the points of `xs` (rising) in a line, the end values beyond.
fn lerp(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    let v = x.clamp(xs[0], xs[xs.len() - 1]);
    let mut i = 0;
    while i < xs.len() - 2 && v > xs[i + 1] {
        i += 1;
    }
    let t = if xs[i + 1] == xs[i] {
        0.0
    } else {
        (v - xs[i]) / (xs[i + 1] - xs[i])
    };
    ys[i] + (ys[i + 1] - ys[i]) * t
}

/// A row of the floor table at the floor's U: uninsulated over 2.0,
/// in a line to 0 under 0.25.
fn floor_row(row: &[f64; 5], u: f64) -> f64 {
    if u > 2.0 {
        return row[0];
    }
    if u < 0.25 {
        return row[4] * u / 0.25;
    }
    lerp(&[0.25, 0.5, 1.0, 2.0], &[row[4], row[3], row[2], row[1]], u)
}

/// U_equiv of a floor on or in the ground, W/(m²K): by B′ (m), the
/// floor's U and how deep it is under the ground `z` (m).
pub fn u_equiv_floor(bp: f64, u: f64, z: f64) -> f64 {
    let at = |t: &[[f64; 5]; 10]| {
        let ys: Vec<f64> = t.iter().map(|r| floor_row(r, u)).collect();
        lerp(&B, &ys, bp)
    };
    let z = z.clamp(0.0, 3.0);
    if z <= 1.5 {
        lerp(&[0.0, 1.5], &[at(&FLOOR_0), at(&FLOOR_15)], z)
    } else {
        lerp(&[1.5, 3.0], &[at(&FLOOR_15), at(&FLOOR_3)], z)
    }
}

/// f_g2; 1 (the upper bound) with no mean annual temperature given.
pub fn fg2(ti: f64, te: f64, te_mean: Option<f64>) -> f64 {
    match te_mean {
        Some(m) if ti - te > 0.0 => ((ti - m) / (ti - te)).clamp(0.0, 1.0),
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tables_are_read_between_their_rows() {
        assert_eq!(u_equiv_floor(2.0, 5.0, 0.0), 1.3);
        assert_eq!(u_equiv_floor(10.0, 0.5, 0.0), 0.23);
        assert!((u_equiv_floor(9.0, 0.5, 0.0) - 0.24).abs() < 1e-12);
        assert!((u_equiv_floor(10.0, 0.75, 0.0) - 0.265).abs() < 1e-12);
        assert!((u_equiv_floor(10.0, 0.5, 0.75) - 0.22).abs() < 1e-12);
        assert_eq!(u_equiv_floor(50.0, 0.25, 3.0), 0.11);
        assert!((u_equiv_floor(10.0, 0.125, 0.0) - 0.075).abs() < 1e-12);
        assert!((fg2(20.0, -11.0, Some(9.5)) - 10.5 / 31.0).abs() < 1e-12);
        assert_eq!(fg2(20.0, -11.0, None), 1.0);
    }
}
