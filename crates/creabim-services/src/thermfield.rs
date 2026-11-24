//! Two-dimensional steady-state temperature field of a construction detail
//! by STN EN ISO 10211 (chapter 9): the detail is cut into cells of one
//! material each, every node of the rectilinear mesh keeps its energy
//! balance (div q = 0 with Fourier's law q = −λ grad θ), the surfaces take
//! the Robin condition q = (θ − θs)/Rs (formula 6) and the cut planes are
//! adiabatic (9.2.2). The equations are solved by conjugate gradients
//! until the heat flows into the detail balance (C.2 e).
//!
//! The method is checked against the reference cases 1 and 2 of annex C
//! (tests read the user's private parameter file, the norm's values are
//! never in the code). From the field come the linear thermal
//! transmittance L2D (formula 15), Ψ = L2D − Σ U·l (18), the lowest
//! inner surface temperature θsi,min and the temperature factor fRsi
//! (formula 32), and the isotherms for the drawing.
//!
//! Lengths are in metres, temperatures in °C, λ in W/(m·K), Rs in
//! m²·K/W; the flows are per metre of the detail's length (W/m).

/// A material of the detail.
#[derive(Clone, Debug, PartialEq)]
pub struct Material {
    pub name: String,
    /// Design thermal conductivity λ, W/(m·K).
    pub lambda: f64,
}

/// A piece of one material: a closed polygon. A later region covers an
/// earlier one where they overlap.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub material: usize,
    pub poly: Vec<[f64; 2]>,
}

impl Region {
    /// The axis-aligned rectangle x0..x1 × y0..y1.
    pub fn rect(material: usize, x0: f64, y0: f64, x1: f64, y1: f64) -> Region {
        Region {
            material,
            poly: vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]],
        }
    }
}

/// A boundary environment: the inside air of a room, the outside air, an
/// unheated space.
#[derive(Clone, Debug, PartialEq)]
pub struct Env {
    pub name: String,
    /// Temperature, °C.
    pub theta: f64,
}

/// A stretch of the detail's surface touching an environment. Surfaces
/// that no edge covers are adiabatic (the cut planes).
#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub a: [f64; 2],
    pub b: [f64; 2],
    pub env: usize,
    /// Surface resistance Rs, m²·K/W; 0 holds the surface at the
    /// environment's temperature.
    pub rs: f64,
}

/// Where the air lies against a surface (the y axis points up): over it
/// (a floor, the heat flowing down into it from a warm room), beside it
/// (a wall) or under it (a ceiling).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Over,
    Beside,
    Under,
}

/// Air of one environment touching the detail: the surfaces it meets take
/// its surface resistance by where it lies against them.
#[derive(Clone, Debug, PartialEq)]
pub struct Air {
    pub env: usize,
    /// Rs where the air lies over, beside and under the surface, m²·K/W.
    pub rs_over: f64,
    pub rs_beside: f64,
    pub rs_under: f64,
}

impl Air {
    pub fn rs(&self, side: Side) -> f64 {
        match side {
            Side::Over => self.rs_over,
            Side::Beside => self.rs_beside,
            Side::Under => self.rs_under,
        }
    }
}

/// The detail to solve.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Detail {
    pub materials: Vec<Material>,
    pub regions: Vec<Region>,
    pub envs: Vec<Env>,
    /// Surfaces given by hand; they win over the air.
    pub edges: Vec<Edge>,
    /// Air spaces (a room's): polygons with their air.
    pub air: Vec<(Vec<[f64; 2]>, Air)>,
    /// The air everywhere else inside the mesh (outside); none leaves
    /// those surfaces adiabatic. Past the mesh's border all is adiabatic
    /// (the cut planes).
    pub outside: Option<Air>,
    /// The box the detail is cut to (its cut planes), lower and upper
    /// corner; none takes the regions' extent.
    pub bounds: Option<([f64; 2], [f64; 2])>,
}

/// How finely the detail is divided (C.2 b: the user decides).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mesh {
    /// Largest cell side, m.
    pub h_max: f64,
    /// Cell side at every material boundary, m.
    pub h_min: f64,
    /// Growth of the cells away from a boundary (1 = even).
    pub ratio: f64,
}

impl Default for Mesh {
    fn default() -> Mesh {
        Mesh {
            h_max: 0.02,
            h_min: 0.002,
            ratio: 1.25,
        }
    }
}

/// A node on a surface touching an environment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceNode {
    pub p: [f64; 2],
    pub env: usize,
    pub theta: f64,
}

/// The solved temperature field.
#[derive(Clone, Debug)]
pub struct Field {
    /// Mesh lines.
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
    /// Node temperatures, row by row (index i + j·xs.len()); NaN off the
    /// detail.
    pub theta: Vec<f64>,
    /// Material of every cell (index i + j·(xs.len() − 1)).
    pub cells: Vec<Option<usize>>,
    /// Heat flow from each environment into the detail, W/m.
    pub flows: Vec<f64>,
    /// Nodes on surfaces with an environment.
    pub surface: Vec<SurfaceNode>,
    /// C.2 e: Σ flows / (½ Σ |flows|).
    pub balance: f64,
    pub iterations: usize,
}

/// Mesh lines along one axis: every key coordinate, the intervals between
/// them divided finely at each key line and growing towards their middle.
fn lines(keys: &[f64], mesh: &Mesh) -> Vec<f64> {
    let mut k: Vec<f64> = keys.to_vec();
    k.sort_by(|a, b| a.total_cmp(b));
    let span = k.last().copied().unwrap_or(0.0) - k.first().copied().unwrap_or(0.0);
    let eps = span.abs().max(1.0) * 1e-9;
    k.dedup_by(|a, b| (*a - *b).abs() <= eps);
    let h_min = mesh.h_min.max(1e-6).min(mesh.h_max);
    let ratio = mesh.ratio.max(1.0);
    let mut out = vec![];
    for w in k.windows(2) {
        let (a, b) = (w[0], w[1]);
        out.push(a);
        let len = b - a;
        // Cells from both ends: h_min, h_min·r, … up to h_max.
        let mut steps = vec![];
        let mut h = h_min;
        let mut used = 0.0;
        while used + 2.0 * h < len && h < mesh.h_max {
            steps.push(h);
            used += 2.0 * h;
            h = (h * ratio).min(mesh.h_max);
        }
        let mid = len - used;
        let n_mid = (mid / mesh.h_max).ceil().max(1.0) as usize;
        let mut pos = a;
        for s in &steps {
            pos += s;
            out.push(pos);
        }
        for m in 1..n_mid {
            out.push(pos + mid * m as f64 / n_mid as f64);
        }
        pos += mid;
        for s in steps.iter().rev() {
            if pos < b - eps {
                out.push(pos);
            }
            pos += s;
        }
    }
    if let Some(&l) = k.last() {
        out.push(l);
    }
    out.dedup_by(|a, b| (*a - *b).abs() <= eps);
    out
}

/// Every mesh interval cut in half (C.2 d: n and 2n divisions).
fn halve(v: &[f64]) -> Vec<f64> {
    let mut out = Vec::with_capacity(v.len() * 2);
    for w in v.windows(2) {
        out.push(w[0]);
        out.push(0.5 * (w[0] + w[1]));
    }
    if let Some(&l) = v.last() {
        out.push(l);
    }
    out
}

fn inside(poly: &[[f64; 2]], p: [f64; 2]) -> bool {
    let mut c = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a[1] > p[1]) != (b[1] > p[1]) {
            let x = a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
            if p[0] < x {
                c = !c;
            }
        }
    }
    c
}

fn seg_dist(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (qx, qy) = (a[0] + t * dx - p[0], a[1] + t * dy - p[1]);
    (qx * qx + qy * qy).sqrt()
}

impl Detail {
    fn keys(&self) -> (Vec<f64>, Vec<f64>) {
        let mut kx = vec![];
        let mut ky = vec![];
        for r in &self.regions {
            for p in &r.poly {
                kx.push(p[0]);
                ky.push(p[1]);
            }
        }
        if let Some((lo, hi)) = self.bounds {
            kx.extend([lo[0], hi[0]]);
            ky.extend([lo[1], hi[1]]);
        }
        for e in &self.edges {
            for p in [e.a, e.b] {
                kx.push(p[0]);
                ky.push(p[1]);
            }
        }
        (kx, ky)
    }

    /// The mesh lines for `mesh`.
    pub fn mesh_lines(&self, mesh: &Mesh) -> (Vec<f64>, Vec<f64>) {
        let (kx, ky) = self.keys();
        (lines(&kx, mesh), lines(&ky, mesh))
    }

    /// Solves the field on the mesh `mesh`.
    pub fn solve(&self, mesh: &Mesh) -> Field {
        let (xs, ys) = self.mesh_lines(mesh);
        self.solve_on(xs, ys)
    }

    /// Solves on `mesh` and on the mesh with every interval halved;
    /// returns both fields (C.2 d, 7.2.7).
    pub fn solve_checked(&self, mesh: &Mesh) -> (Field, Field) {
        let (xs, ys) = self.mesh_lines(mesh);
        let fine = self.solve_on(halve(&xs), halve(&ys));
        (self.solve_on(xs, ys), fine)
    }

    /// The air at point `p` off the detail.
    fn air_at(&self, p: [f64; 2]) -> Option<&Air> {
        self.air
            .iter()
            .find(|(poly, _)| inside(poly, p))
            .map(|(_, a)| a)
            .or(self.outside.as_ref())
    }

    /// The edge a boundary face from `a` to `b` lies on: exactly, or for a
    /// sloping edge drawn as steps, with both ends near it.
    fn edge_at(&self, a: [f64; 2], b: [f64; 2]) -> Option<usize> {
        let h = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        let m = [0.5 * (a[0] + b[0]), 0.5 * (a[1] + b[1])];
        let mut best: Option<(usize, f64)> = None;
        for (k, e) in self.edges.iter().enumerate() {
            let d = seg_dist(m, e.a, e.b);
            let near = d <= 1e-9 * h.max(1.0)
                || (seg_dist(a, e.a, e.b) <= 0.75 * h
                    && seg_dist(b, e.a, e.b) <= 0.75 * h
                    && d <= 0.75 * h);
            if near && best.is_none_or(|(_, bd)| d < bd) {
                best = Some((k, d));
            }
        }
        best.map(|(k, _)| k)
    }

    /// Solves the field on the given mesh lines.
    pub fn solve_on(&self, xs: Vec<f64>, ys: Vec<f64>) -> Field {
        let (nx, ny) = (xs.len(), ys.len());
        let (cx, cy) = (nx.saturating_sub(1), ny.saturating_sub(1));
        let mut cells = vec![None; cx * cy];
        for j in 0..cy {
            let yc = 0.5 * (ys[j] + ys[j + 1]);
            for i in 0..cx {
                let p = [0.5 * (xs[i] + xs[i + 1]), yc];
                for r in self.regions.iter().rev() {
                    if inside(&r.poly, p) {
                        cells[i + j * cx] = Some(r.material);
                        break;
                    }
                }
            }
        }
        let lam = |i: isize, j: isize| -> f64 {
            if i < 0 || j < 0 || i as usize >= cx || j as usize >= cy {
                return 0.0;
            }
            cells[i as usize + j as usize * cx]
                .map(|m| self.materials[m].lambda)
                .unwrap_or(0.0)
        };
        let solid = |i: isize, j: isize| lam(i, j) > 0.0;
        let n = nx * ny;
        let id = |i: usize, j: usize| i + j * nx;
        // Conductances to the east and north neighbours.
        let mut ge = vec![0.0; n];
        let mut gn = vec![0.0; n];
        for j in 0..ny {
            for i in 0..nx {
                if i + 1 < nx {
                    let dx = xs[i + 1] - xs[i];
                    let below = if j > 0 {
                        lam(i as isize, j as isize - 1) * 0.5 * (ys[j] - ys[j - 1])
                    } else {
                        0.0
                    };
                    let above = if j + 1 < ny {
                        lam(i as isize, j as isize) * 0.5 * (ys[j + 1] - ys[j])
                    } else {
                        0.0
                    };
                    ge[id(i, j)] = (below + above) / dx;
                }
                if j + 1 < ny {
                    let dy = ys[j + 1] - ys[j];
                    let left = if i > 0 {
                        lam(i as isize - 1, j as isize) * 0.5 * (xs[i] - xs[i - 1])
                    } else {
                        0.0
                    };
                    let right = if i + 1 < nx {
                        lam(i as isize, j as isize) * 0.5 * (xs[i + 1] - xs[i])
                    } else {
                        0.0
                    };
                    gn[id(i, j)] = (left + right) / dy;
                }
            }
        }
        // Surface conductances to environments: (node, env, G); fixed nodes.
        let mut film: Vec<(usize, usize, f64)> = vec![];
        let mut fixed: Vec<Option<(f64, usize)>> = vec![None; n];
        let mut fix_sum: Vec<(f64, f64)> = vec![(0.0, 0.0); n];
        // The air cell beside a face: its centre, and whether the air lies
        // over the surface, beside it or under it.
        let air_cell = |i: isize, j: isize, side: Side| -> Option<([f64; 2], Side)> {
            if i < 0 || j < 0 || i as usize >= cx || j as usize >= cy {
                return None;
            }
            let (i, j) = (i as usize, j as usize);
            Some(([0.5 * (xs[i] + xs[i + 1]), 0.5 * (ys[j] + ys[j + 1])], side))
        };
        let mut face = |a: usize,
                        b: usize,
                        pa: [f64; 2],
                        pb: [f64; 2],
                        air: Option<([f64; 2], Side)>,
                        film: &mut Vec<(usize, usize, f64)>| {
            let len = ((pb[0] - pa[0]).powi(2) + (pb[1] - pa[1]).powi(2)).sqrt();
            let found = match self.edge_at(pa, pb) {
                Some(k) => Some((self.edges[k].env, self.edges[k].rs)),
                None => air.and_then(|(c, side)| self.air_at(c).map(|a| (a.env, a.rs(side)))),
            };
            if let Some((env, rs)) = found {
                if rs <= 0.0 {
                    let th = self.envs[env].theta;
                    for node in [a, b] {
                        fix_sum[node].0 += th;
                        fix_sum[node].1 += 1.0;
                        fixed[node] = Some((0.0, env));
                    }
                } else {
                    film.push((a, env, 0.5 * len / rs));
                    film.push((b, env, 0.5 * len / rs));
                }
            }
        };
        for j in 0..ny {
            for i in 0..nx {
                // Horizontal face y_j, x_i..x_i+1 between cells (i,j-1) and (i,j).
                let (ii, jj) = (i as isize, j as isize);
                if i + 1 < nx && solid(ii, jj - 1) != solid(ii, jj) {
                    let air = if solid(ii, jj - 1) {
                        air_cell(ii, jj, Side::Over)
                    } else {
                        air_cell(ii, jj - 1, Side::Under)
                    };
                    face(
                        id(i, j),
                        id(i + 1, j),
                        [xs[i], ys[j]],
                        [xs[i + 1], ys[j]],
                        air,
                        &mut film,
                    );
                }
                if j + 1 < ny && solid(ii - 1, jj) != solid(ii, jj) {
                    let air = if solid(ii - 1, jj) {
                        air_cell(ii, jj, Side::Beside)
                    } else {
                        air_cell(ii - 1, jj, Side::Beside)
                    };
                    face(
                        id(i, j),
                        id(i, j + 1),
                        [xs[i], ys[j]],
                        [xs[i], ys[j + 1]],
                        air,
                        &mut film,
                    );
                }
            }
        }
        for k in 0..n {
            if let Some((_, env)) = fixed[k] {
                fixed[k] = Some((fix_sum[k].0 / fix_sum[k].1, env));
            }
        }
        let mut diag = vec![0.0; n];
        let mut rhs = vec![0.0; n];
        for k in 0..n {
            let (i, j) = (k % nx, k / nx);
            diag[k] += ge[k] + gn[k];
            if i > 0 {
                diag[k] += ge[k - 1];
            }
            if j > 0 {
                diag[k] += gn[k - nx];
            }
        }
        for &(k, env, g) in &film {
            diag[k] += g;
            rhs[k] += g * self.envs[env].theta;
        }
        let active: Vec<bool> = (0..n).map(|k| diag[k] > 0.0).collect();
        // Unknowns: active, not fixed. Fixed values move to the rhs.
        let mut x = vec![0.0; n];
        let mean = if self.envs.is_empty() {
            0.0
        } else {
            self.envs.iter().map(|e| e.theta).sum::<f64>() / self.envs.len() as f64
        };
        for k in 0..n {
            x[k] = match fixed[k] {
                Some((t, _)) => t,
                None => mean,
            };
        }
        let free: Vec<bool> = (0..n).map(|k| active[k] && fixed[k].is_none()).collect();
        let apply = |v: &[f64], out: &mut [f64], only_free: bool| {
            for k in 0..n {
                if !free[k] {
                    out[k] = 0.0;
                    continue;
                }
                let (i, j) = (k % nx, k / nx);
                let val = |m: usize| if only_free && !free[m] { 0.0 } else { v[m] };
                let mut s = diag[k] * v[k];
                if i + 1 < nx {
                    s -= ge[k] * val(k + 1);
                }
                if i > 0 {
                    s -= ge[k - 1] * val(k - 1);
                }
                if j + 1 < ny {
                    s -= gn[k] * val(k + nx);
                }
                if j > 0 {
                    s -= gn[k - nx] * val(k - nx);
                }
                out[k] = s;
            }
        };
        // b = rhs − A_fixed·x_fixed for the free nodes.
        let mut b = vec![0.0; n];
        {
            let mut ax = vec![0.0; n];
            let mut xf = x.clone();
            for k in 0..n {
                if free[k] {
                    xf[k] = 0.0;
                }
            }
            apply(&xf, &mut ax, false);
            for k in 0..n {
                if free[k] {
                    b[k] = rhs[k] - ax[k];
                }
            }
        }
        // Conjugate gradients preconditioned by the incomplete Cholesky
        // factor IC(0) of the five-point stencil.
        let mut ic = vec![0.0; n];
        for k in 0..n {
            if !free[k] {
                continue;
            }
            let (i, j) = (k % nx, k / nx);
            let mut d = diag[k];
            if i > 0 && free[k - 1] {
                d -= ge[k - 1] * ge[k - 1] / ic[k - 1];
            }
            if j > 0 && free[k - nx] {
                d -= gn[k - nx] * gn[k - nx] / ic[k - nx];
            }
            ic[k] = if d > 1e-12 * diag[k] { d } else { diag[k] };
        }
        let precond = |r: &[f64], z: &mut [f64]| {
            // (D + L) u = r
            for k in 0..n {
                if !free[k] {
                    z[k] = 0.0;
                    continue;
                }
                let (i, j) = (k % nx, k / nx);
                let mut v = r[k];
                if i > 0 && free[k - 1] {
                    v += ge[k - 1] * z[k - 1];
                }
                if j > 0 && free[k - nx] {
                    v += gn[k - nx] * z[k - nx];
                }
                z[k] = v / ic[k];
            }
            // (D + Lᵀ) z = D u
            for k in (0..n).rev() {
                if !free[k] {
                    continue;
                }
                let (i, j) = (k % nx, k / nx);
                let mut v = ic[k] * z[k];
                if i + 1 < nx && free[k + 1] {
                    v += ge[k] * z[k + 1];
                }
                if j + 1 < ny && free[k + nx] {
                    v += gn[k] * z[k + nx];
                }
                z[k] = v / ic[k];
            }
        };
        let mut iterations = 0;
        {
            let mut xv: Vec<f64> = (0..n).map(|k| if free[k] { x[k] } else { 0.0 }).collect();
            let mut ax = vec![0.0; n];
            apply(&xv, &mut ax, true);
            let mut r: Vec<f64> = (0..n)
                .map(|k| if free[k] { b[k] - ax[k] } else { 0.0 })
                .collect();
            let mut z = vec![0.0; n];
            precond(&r, &mut z);
            let mut p = z.clone();
            let mut rz: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
            let bnorm = b.iter().map(|v| v * v).sum::<f64>().sqrt().max(1e-300);
            let mut ap = vec![0.0; n];
            let max_it = 20 * n + 100;
            while iterations < max_it {
                let rn = r.iter().map(|v| v * v).sum::<f64>().sqrt();
                if rn <= 1e-12 * bnorm {
                    break;
                }
                apply(&p, &mut ap, true);
                let pap: f64 = p.iter().zip(&ap).map(|(a, b)| a * b).sum();
                if pap <= 0.0 {
                    break;
                }
                let alpha = rz / pap;
                for k in 0..n {
                    xv[k] += alpha * p[k];
                    r[k] -= alpha * ap[k];
                }
                precond(&r, &mut z);
                let rz2: f64 = r.iter().zip(&z).map(|(a, b)| a * b).sum();
                let beta = rz2 / rz;
                rz = rz2;
                for k in 0..n {
                    p[k] = z[k] + beta * p[k];
                }
                iterations += 1;
            }
            for k in 0..n {
                if free[k] {
                    x[k] = xv[k];
                }
            }
        }
        let theta: Vec<f64> = (0..n)
            .map(|k| {
                if active[k] || fixed[k].is_some() {
                    x[k]
                } else {
                    f64::NAN
                }
            })
            .collect();
        // Heat flows from each environment.
        let mut flows = vec![0.0; self.envs.len()];
        for &(k, env, g) in &film {
            flows[env] += g * (self.envs[env].theta - theta[k]);
        }
        for k in 0..n {
            if let Some((t, env)) = fixed[k] {
                // Reaction: the heat the fixed node gives its neighbours.
                let (i, j) = (k % nx, k / nx);
                let mut q = 0.0;
                let mut take = |m: usize, g: f64| {
                    if g > 0.0 && fixed[m].is_none() {
                        q += g * (t - theta[m]);
                    }
                };
                if i + 1 < nx {
                    take(k + 1, ge[k]);
                }
                if i > 0 {
                    take(k - 1, ge[k - 1]);
                }
                if j + 1 < ny {
                    take(k + nx, gn[k]);
                }
                if j > 0 {
                    take(k - nx, gn[k - nx]);
                }
                flows[env] += q;
            }
        }
        let abs: f64 = flows.iter().map(|f| f.abs()).sum();
        let balance = if abs > 0.0 {
            flows.iter().sum::<f64>() / (0.5 * abs)
        } else {
            0.0
        };
        let mut surface = vec![];
        let mut seen = vec![usize::MAX; n];
        for &(k, env, _) in &film {
            if seen[k] != env {
                seen[k] = env;
                surface.push(SurfaceNode {
                    p: [xs[k % nx], ys[k / nx]],
                    env,
                    theta: theta[k],
                });
            }
        }
        for k in 0..n {
            if let Some((t, env)) = fixed[k] {
                surface.push(SurfaceNode {
                    p: [xs[k % nx], ys[k / nx]],
                    env,
                    theta: t,
                });
            }
        }
        Field {
            xs,
            ys,
            theta,
            cells,
            flows,
            surface,
            balance,
            iterations,
        }
    }
}

impl Field {
    fn node(&self, i: usize, j: usize) -> f64 {
        self.theta[i + j * self.xs.len()]
    }

    /// Temperature at a point, interpolated linearly between the nodes
    /// (9.2.4); None off the detail.
    pub fn at(&self, p: [f64; 2]) -> Option<f64> {
        let fi = |v: &[f64], c: f64| -> Option<(usize, f64)> {
            if v.len() < 2 || c < v[0] - 1e-12 || c > v[v.len() - 1] + 1e-12 {
                return None;
            }
            let i = v.partition_point(|&x| x <= c).clamp(1, v.len() - 1) - 1;
            Some((i, ((c - v[i]) / (v[i + 1] - v[i])).clamp(0.0, 1.0)))
        };
        let (i, tx) = fi(&self.xs, p[0])?;
        let (j, ty) = fi(&self.ys, p[1])?;
        let c = [
            self.node(i, j),
            self.node(i + 1, j),
            self.node(i, j + 1),
            self.node(i + 1, j + 1),
        ];
        let w = [
            (1.0 - tx) * (1.0 - ty),
            tx * (1.0 - ty),
            (1.0 - tx) * ty,
            tx * ty,
        ];
        let (mut s, mut ws) = (0.0, 0.0);
        for k in 0..4 {
            if c[k].is_finite() && w[k] > 0.0 {
                s += c[k] * w[k];
                ws += w[k];
            }
        }
        if ws > 0.0 {
            Some(s / ws)
        } else {
            [c[0], c[1], c[2], c[3]].into_iter().find(|v| v.is_finite())
        }
    }

    /// The coldest surface node towards environment `env`.
    pub fn min_surface(&self, env: usize) -> Option<SurfaceNode> {
        self.surface
            .iter()
            .filter(|s| s.env == env)
            .copied()
            .min_by(|a, b| a.theta.total_cmp(&b.theta))
    }

    /// Thermal coupling coefficient L2D between two environments when the
    /// detail has only these two (formula 15), W/(m·K).
    pub fn l2d(&self, warm: usize, cold: usize, envs: &[Env]) -> f64 {
        let dt = envs[warm].theta - envs[cold].theta;
        if dt.abs() < 1e-12 {
            return 0.0;
        }
        self.flows[warm] / dt
    }

    /// Number of nodes in the detail.
    pub fn nodes(&self) -> usize {
        self.theta.iter().filter(|t| t.is_finite()).count()
    }
}

/// Temperature factor fRsi = (θsi − θe)/(θi − θe) (formula 32).
pub fn frsi(theta_si: f64, theta_i: f64, theta_e: f64) -> f64 {
    (theta_si - theta_e) / (theta_i - theta_e)
}

/// Linear thermal transmittance Ψ = L2D − Σ Uj·lj (formula 18), W/(m·K).
pub fn psi(l2d: f64, parts: &[(f64, f64)]) -> f64 {
    l2d - parts.iter().map(|(u, l)| u * l).sum::<f64>()
}

/// Outcome of the mesh check (7.2.7, C.2 d).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshCheck {
    /// Relative change of the heat flow on halving the cells.
    pub flow_change: f64,
    /// Change of fRsi on halving the cells.
    pub frsi_change: f64,
    /// Either criterion met (flow ≤ 1 % or fRsi ≤ 0,005).
    pub ok: bool,
}

/// Compares the coarse and the halved mesh for the warm and cold
/// environments; `flow_limit` (fraction) and `frsi_limit` from the norm's
/// parameter file.
pub fn mesh_check(
    coarse: &Field,
    fine: &Field,
    warm: usize,
    cold: usize,
    envs: &[Env],
    flow_limit: f64,
    frsi_limit: f64,
) -> MeshCheck {
    let f0 = coarse.flows[warm];
    let f1 = fine.flows[warm];
    let flow_change = if f1.abs() > 0.0 {
        ((f1 - f0) / f1).abs()
    } else {
        0.0
    };
    let fr = |f: &Field| {
        f.min_surface(warm)
            .map(|s| frsi(s.theta, envs[warm].theta, envs[cold].theta))
            .unwrap_or(0.0)
    };
    let frsi_change = (fr(fine) - fr(coarse)).abs();
    MeshCheck {
        flow_change,
        frsi_change,
        ok: flow_change <= flow_limit || frsi_change <= frsi_limit,
    }
}

/// Isotherm segments of temperature `t` (marching squares over the cells
/// of the detail).
pub fn isotherm(f: &Field, t: f64) -> Vec<[[f64; 2]; 2]> {
    let (nx, ny) = (f.xs.len(), f.ys.len());
    let mut out = vec![];
    for j in 0..ny.saturating_sub(1) {
        for i in 0..nx.saturating_sub(1) {
            if f.cells[i + j * (nx - 1)].is_none() {
                continue;
            }
            let p = [
                [f.xs[i], f.ys[j]],
                [f.xs[i + 1], f.ys[j]],
                [f.xs[i + 1], f.ys[j + 1]],
                [f.xs[i], f.ys[j + 1]],
            ];
            let v = [
                f.node(i, j),
                f.node(i + 1, j),
                f.node(i + 1, j + 1),
                f.node(i, j + 1),
            ];
            if v.iter().any(|x| !x.is_finite()) {
                continue;
            }
            let mut pts = vec![];
            for k in 0..4 {
                let (a, b) = (v[k], v[(k + 1) % 4]);
                if (a < t) != (b < t) {
                    let s = (t - a) / (b - a);
                    let (pa, pb) = (p[k], p[(k + 1) % 4]);
                    pts.push([pa[0] + s * (pb[0] - pa[0]), pa[1] + s * (pb[1] - pa[1])]);
                }
            }
            if pts.len() == 2 {
                out.push([pts[0], pts[1]]);
            } else if pts.len() == 4 {
                out.push([pts[0], pts[1]]);
                out.push([pts[2], pts[3]]);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Case 1 geometry without any norm value: the half square column with
    /// its surfaces held at 20 and 0 °C, against the Fourier series.
    fn column(h: f64) -> (Detail, Field) {
        let a = 1.0;
        let d = Detail {
            materials: vec![Material {
                name: "m".into(),
                lambda: 1.0,
            }],
            regions: vec![Region::rect(0, 0.0, 0.0, a / 2.0, a)],
            envs: vec![
                Env {
                    name: "hot".into(),
                    theta: 20.0,
                },
                Env {
                    name: "cold".into(),
                    theta: 0.0,
                },
            ],
            edges: vec![
                Edge {
                    a: [0.0, 0.0],
                    b: [a / 2.0, 0.0],
                    env: 0,
                    rs: 0.0,
                },
                Edge {
                    a: [0.0, 0.0],
                    b: [0.0, a],
                    env: 1,
                    rs: 0.0,
                },
                Edge {
                    a: [0.0, a],
                    b: [a / 2.0, a],
                    env: 1,
                    rs: 0.0,
                },
            ],
            ..Default::default()
        };
        let f = d.solve(&Mesh {
            h_max: h,
            h_min: h,
            ratio: 1.0,
        });
        (d, f)
    }

    /// Square a×a, top at 20 °C, the other sides 0 °C: Fourier series.
    fn exact(x: f64, y: f64) -> f64 {
        let mut s = 0.0;
        for k in 0..200 {
            let n = (2 * k + 1) as f64;
            let np = n * std::f64::consts::PI;
            // y from the hot edge: sinh(nπ(1−y))/sinh(nπ) without overflow.
            let r = ((-np * y).exp() - (-np * (2.0 - y)).exp()) / (1.0 - (-2.0 * np).exp());
            s += 4.0 / np * (np * x).sin() * r;
        }
        20.0 * s
    }

    #[test]
    fn column_matches_series() {
        let (_, f) = column(1.0 / 64.0);
        for row in 1..8 {
            for col in 1..5 {
                let (x, y) = (col as f64 / 8.0, row as f64 / 8.0);
                let t = f.at([x, y]).unwrap();
                assert!(
                    (t - exact(x, y)).abs() < 0.1,
                    "({col},{row}) {t} vs {}",
                    exact(x, y)
                );
            }
        }
        assert!(f.balance.abs() < 1e-4, "balance {}", f.balance);
    }

    #[test]
    fn layered_wall_gives_u() {
        // 1D: 0,3 m of λ 0,5 between Rsi 0,13 and Rse 0,04.
        let d = Detail {
            materials: vec![Material {
                name: "m".into(),
                lambda: 0.5,
            }],
            regions: vec![Region::rect(0, 0.0, 0.0, 0.3, 1.0)],
            envs: vec![
                Env {
                    name: "i".into(),
                    theta: 20.0,
                },
                Env {
                    name: "e".into(),
                    theta: -10.0,
                },
            ],
            edges: vec![
                Edge {
                    a: [0.0, 0.0],
                    b: [0.0, 1.0],
                    env: 0,
                    rs: 0.13,
                },
                Edge {
                    a: [0.3, 0.0],
                    b: [0.3, 1.0],
                    env: 1,
                    rs: 0.04,
                },
            ],
            ..Default::default()
        };
        let f = d.solve(&Mesh::default());
        let u = 1.0 / (0.13 + 0.3 / 0.5 + 0.04);
        assert!((f.l2d(0, 1, &d.envs) - u).abs() < 1e-6);
        let tsi = 20.0 - u * 30.0 * 0.13;
        assert!((f.min_surface(0).unwrap().theta - tsi).abs() < 1e-6);
        assert!(psi(f.l2d(0, 1, &d.envs), &[(u, 1.0)]).abs() < 1e-6);
        assert!(!isotherm(&f, 0.0).is_empty());
    }

    #[test]
    fn air_gives_surfaces() {
        // A 0,3 m wall in a 1,3 m wide box: a room on the left, outside on
        // the right, the box's top and bottom adiabatic.
        let d = Detail {
            materials: vec![Material {
                name: "m".into(),
                lambda: 0.5,
            }],
            regions: vec![Region::rect(0, 0.5, 0.0, 0.8, 1.0)],
            envs: vec![
                Env {
                    name: "i".into(),
                    theta: 20.0,
                },
                Env {
                    name: "e".into(),
                    theta: -10.0,
                },
            ],
            air: vec![(
                vec![[-1.0, -1.0], [0.5, -1.0], [0.5, 2.0], [-1.0, 2.0]],
                Air {
                    env: 0,
                    rs_over: 0.17,
                    rs_beside: 0.13,
                    rs_under: 0.10,
                },
            )],
            outside: Some(Air {
                env: 1,
                rs_over: 0.04,
                rs_beside: 0.04,
                rs_under: 0.04,
            }),
            bounds: Some(([0.0, 0.0], [1.3, 1.0])),
            ..Default::default()
        };
        let mut f = d.solve(&Mesh::default());
        // the box: air cells on both sides only where the mesh reaches
        assert!(f.surface.iter().all(|s| s.p[0] < 0.51 || s.p[0] > 0.79));
        let u = 1.0 / (0.13 + 0.3 / 0.5 + 0.04);
        assert!(
            (f.l2d(0, 1, &d.envs) - u).abs() < 1e-6,
            "{}",
            f.l2d(0, 1, &d.envs)
        );
        assert_eq!(f.flows.len(), 2);
        f.flows.clear();
    }
}
