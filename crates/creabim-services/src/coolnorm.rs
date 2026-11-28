//! The parts of the cooling load that follow STN 73 0548 itself
//! ([`crate::coolingload::Method::Norm`]):
//!
//! - heat through outside walls and roofs by the sol-air temperature
//!   t_r = t_e + ε·I/α_e (formula 29), damped and delayed by the wall:
//!   Q = k·S·[(t_rm − t_i) + m·(t_rψ − t_rm)] (formula 30, which a
//!   program uses for every wall, art. 71), with the phase shift
//!   ψ = 32·δ − 0,5 h (31) and the damping m = (1 + 7,6·δ) / 2500^δ (32)
//!   by the wall's thickness δ;
//! - the sun through the windows taken off by what the room stores when
//!   it may get warmer by Δt: ΔQ = 0,05·M·Δt (27), at least the mean
//!   over the hours of use (28); M is half the thickness of the inner
//!   walls (at most 0,08 m of a wall thicker than 0,16 m), of the floor
//!   and of the ceiling (art. 69), a carpeted floor a quarter;
//! - the shading s of table 11.
//!
//! Every number of the norm's tables (ε of table 12, s of table 11, the
//! sun on the walls of table 5, α_e of art. 25, the 0,05 of formula 27 and
//! the usual Δt of art. 68) is read from the norm's parameter file the
//! designer keeps (tables [`EPS`], [`SHADE`], [`SUN_WALL`], [`FACTORS`]).

use crate::heatloss::{beyond, rooms_of, wall_of};
use creabim_core::fireparams::{self, ParamTable};
use creabim_core::{polygon, ElementId, ElementKind, Model};

/// Table 12, ε of a surface for the sun: columns `key`, `eps`, `text`.
pub const EPS: &str = "cool_eps";
/// Table 11, the shading coefficient s: columns `key`, `s`, `text`.
pub const SHADE: &str = "cool_shade";
/// Table 5, the sun falling on a wall of a face, W/m², on the 21st of a
/// month: rows `S`, `SV`, `V`, `JV`, `J`, `JZ`, `Z`, `SZ` (north,
/// north-east …) and `HOR` (a flat roof), columns `key`, `month` (3 to
/// 10; a table without it is July's), `h5` … `h19`.
pub const SUN_WALL: &str = "cool_sun_wall";
/// Table 10, the sun through a standard single-glazed window, W/m², laid
/// out as [`SUN_WALL`]; north's (`S`) is the diffuse one (art. 36).
pub const SUN_GLASS: &str = "cool_sun_glass";
/// Table 1, the highest outside temperature of a month: columns `key`
/// (the month, 3 to 10), `te_max` (°C), `text`.
pub const TE_MAX: &str = "cool_te_max";
/// Table 6, a man's sensible heat, W: columns `key`, `met`, `s21`,
/// `s24`, `s25`, `s26`, `s28` (at that room air temperature), `text`.
pub const PEOPLE: &str = "cool_people";
/// Single values: rows `alpha_e` (art. 25, W/(m²K)), `k_acc` (formula
/// 27, W/(kg·K)), `dt_allow` (art. 68, K), `amplitude` (A of formula 1,
/// K), `c_o_vidiek` and `c_o_mesto` (art. 64), `latitude` (degrees, of
/// formula 3). Columns `key`, `value`, `text`.
pub const FACTORS: &str = "cool_factors";
/// The tables of this method.
pub const TABLES: [&str; 7] = [EPS, SHADE, SUN_WALL, SUN_GLASS, FACTORS, TE_MAX, PEOPLE];

/// The faces of table 5 clockwise from north.
const FACES: [&str; 8] = ["S", "SV", "V", "JV", "J", "JZ", "Z", "SZ"];

/// The norm's tables for model `model`: the project's, else the
/// application's.
pub fn tables_of(model: &Model) -> Vec<ParamTable> {
    let own = model
        .project_info()
        .map(|(_, p)| p.fire_params.clone())
        .unwrap_or_default();
    fireparams::effective(&own)
        .into_iter()
        .filter(|t| TABLES.contains(&t.id.as_str()))
        .collect()
}

/// A row of table [`EPS`] or [`SHADE`].
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub key: String,
    pub text: String,
    pub value: f64,
}

/// The rows of table `id` (`EPS` or `SHADE`) with their value in column
/// `col` (`eps`, `s`).
pub fn rows(tables: &[ParamTable], id: &str, col: &str) -> Vec<Row> {
    let Some(t) = tables.iter().find(|t| t.id == id) else {
        return vec![];
    };
    let (Some(k), Some(c), Some(v)) = (t.column("key"), t.column("text"), t.column(col)) else {
        return vec![];
    };
    t.rows
        .iter()
        .filter_map(|r| {
            Some(Row {
                key: r.get(k)?.clone(),
                text: r.get(c)?.clone(),
                value: fireparams::number(r.get(v)?)?,
            })
        })
        .collect()
}

/// The row picked, `pick` an index of `rows`, else the row of key
/// `default`.
pub fn picked<'a>(rows: &'a [Row], pick: Option<usize>, default: &str) -> Option<&'a Row> {
    match pick {
        Some(i) => rows.get(i),
        None => rows.iter().find(|r| r.key == default),
    }
}

/// A single value of table [`FACTORS`].
pub fn factor(tables: &[ParamTable], key: &str) -> Option<f64> {
    let t = tables.iter().find(|t| t.id == FACTORS)?;
    fireparams::number(t.cell(key, "value"))
}

/// The sun of table 5 or 10 on a whole hour `h` (5 to 19, none else)
/// for face row `key` of month `month` (a table with no months is
/// July's).
fn sun_row(t: &ParamTable, key: &str, month: u8, h: i32) -> Option<f64> {
    if !(5..=19).contains(&h) {
        return Some(0.0);
    }
    let col = t.column(&format!("h{h}"))?;
    let row = match t.column("month") {
        Some(m) => t.rows.iter().find(|r| {
            r.first().is_some_and(|k| k == key)
                && r.get(m)
                    .and_then(|x| fireparams::number(x))
                    .is_some_and(|x| (x - month as f64).abs() < 0.5)
        })?,
        None if month == 7 => t.row(key)?,
        None => return None,
    };
    fireparams::number(row.get(col)?)
}

/// The sun of table `id` ([`SUN_WALL`], [`SUN_GLASS`]) on a face `az`
/// (degrees clockwise from north; `None` a flat roof) in month `month`
/// at the whole hour `h`, W/m², between two faces in proportion.
pub fn sun_of(tables: &[ParamTable], id: &str, month: u8, az: Option<f64>, h: i32) -> Option<f64> {
    let t = tables.iter().find(|t| t.id == id)?;
    let Some(az) = az else {
        return sun_row(t, "HOR", month, h);
    };
    let a = az.rem_euclid(360.0) / 45.0;
    let i = a.floor() as usize % 8;
    let f = a - a.floor();
    let x = sun_row(t, FACES[i], month, h)?;
    if f < 1e-9 {
        return Some(x);
    }
    let y = sun_row(t, FACES[(i + 1) % 8], month, h)?;
    Some(x + (y - x) * f)
}

/// The sun falling on a wall facing `az` (`None` a flat roof) in July
/// at the whole hour `h`, W/m² (table 5).
pub fn sun_on(tables: &[ParamTable], az: Option<f64>, h: i32) -> Option<f64> {
    sun_of(tables, SUN_WALL, 7, az, h)
}

/// The highest outside temperature of month `month` (table 1), °C.
pub fn te_max(tables: &[ParamTable], month: u8) -> Option<f64> {
    let t = tables.iter().find(|t| t.id == TE_MAX)?;
    fireparams::number(t.cell(&month.to_string(), "te_max"))
}

/// The outside air at hour `h` of the day whose highest is `te_max`
/// (formula 1): t_e = t_e,max − A·[1 − sin(15τ − 135)].
pub fn te_at(te_max: f64, amplitude: f64, h: f64) -> f64 {
    te_max - amplitude * (1.0 - (15.0 * h - 135.0).to_radians().sin())
}

/// Where the sun stands in month `month` at solar hour `tau` at the
/// latitude `lat`: its height h and azimuth a (clockwise from north),
/// degrees (formulas 2, 3, 4; the hours of table 4).
pub fn sun_position(month: u8, tau: f64, lat: f64) -> (f64, f64) {
    let d = (-23.5 * (30.0 * month as f64).to_radians().cos()).to_radians();
    let (p, w) = (lat.to_radians(), (15.0 * tau).to_radians());
    let sin_h = p.sin() * d.sin() - p.cos() * d.cos() * w.cos();
    let h = sin_h.clamp(-1.0, 1.0).asin();
    // sin a of formula 4, its cosine to tell the quadrant
    let sin_a = w.sin() * d.cos() / h.cos().max(1e-9);
    let cos_a = (d.sin() - sin_h * p.sin()) / (h.cos() * p.cos()).max(1e-9);
    let a = sin_a.atan2(cos_a).to_degrees().rem_euclid(360.0);
    (h.to_degrees(), a)
}

/// The sunlit part of a window S_os, m² (formulas 24 to 26): glass
/// `la` × `lb` m set `d` m deep in the wall (its reveals at the side and
/// at the head the same), facing `gamma`, the sun at height `h` and
/// azimuth `a` (degrees). None in the sun when it shines along or behind
/// the face.
pub fn sunlit(la: f64, lb: f64, d: f64, gamma: f64, h: f64, a: f64) -> f64 {
    let x = (a - gamma).to_radians();
    if h <= 0.0 || x.cos() <= 1e-6 {
        return 0.0;
    }
    let e1 = d * x.tan().abs();
    let e2 = d * h.to_radians().tan() / x.cos();
    (la - e1).max(0.0) * (lb - e2).max(0.0)
}

/// A man's sensible heat by row `key` of table 6 at room air `ti`, W,
/// between the table's temperatures in a straight line.
pub fn person_w(tables: &[ParamTable], key: &str, ti: f64) -> Option<f64> {
    let t = tables.iter().find(|t| t.id == PEOPLE)?;
    let pts: Vec<(f64, f64)> = [21.0, 24.0, 25.0, 26.0, 28.0]
        .iter()
        .filter_map(|x| Some((*x, fireparams::number(t.cell(key, &format!("s{x}")))?)))
        .collect();
    let (first, last) = (pts.first()?, pts.last()?);
    if ti <= first.0 {
        return Some(first.1);
    }
    if ti >= last.0 {
        return Some(last.1);
    }
    pts.windows(2)
        .find(|w| ti >= w[0].0 && ti <= w[1].0)
        .map(|w| w[0].1 + (w[1].1 - w[0].1) * (ti - w[0].0) / (w[1].0 - w[0].0))
}

/// A window of a room for formula 23: where it faces, its glass, how
/// deep it sits from the wall's outside face.
#[derive(Clone, Debug, PartialEq)]
pub struct Win {
    pub az: f64,
    pub la: f64,
    pub lb: f64,
    pub depth: f64,
}

/// The windows of every room in its outside walls.
pub fn windows(model: &Model) -> Vec<(ElementId, Win)> {
    let rooms = rooms_of(model);
    let north = model.project_info().map_or(0.0, |(_, p)| p.true_north);
    let mut out = vec![];
    let mut seen: Vec<ElementId> = vec![];
    for r in &rooms {
        let n = r.shape.outline.len();
        for i in 0..n {
            let (a, b) = (r.shape.outline[i], r.shape.outline[(i + 1) % n]);
            let Some((we, wall)) = wall_of(model, r.shape.bounded_by.get(i).copied().flatten())
            else {
                continue;
            };
            let p = beyond(a, b, wall.thickness);
            if rooms.iter().any(|x| {
                x.level == r.level && x.e.id != r.e.id && polygon::contains(&x.shape.outline, p)
            }) {
                continue;
            }
            let u = (wall.end - wall.start).normalize_or_zero();
            let (s0, s1) = {
                let (x, y) = ((a - wall.start).dot(u), (b - wall.start).dot(u));
                (x.min(y), x.max(y))
            };
            let az = crate::heatloss::azimuth(glam::DVec2::new((b - a).y, -(b - a).x), north);
            let asm = creabim_core::assembly::assembly_of(model, we).map(|x| x.1);
            let t = wall.thickness / 2.0;
            let core = asm
                .filter(|a| a.layers.len() >= 2)
                .map_or((-t, t), |a| creabim_core::assembly::wall_core(a, wall));
            // from the wall's outside face to the core's outer edge
            let to_core = if wall.flip { t - core.1 } else { core.0 + t };
            for e in model.iter() {
                let ElementKind::Opening(op) = &e.kind else {
                    continue;
                };
                let mid = op.offset + op.width / 2.0;
                if op.host != we.id
                    || op.kind != creabim_core::OpeningKind::Window
                    || seen.contains(&e.id)
                    || mid < s0 - 1e-3
                    || mid > s1 + 1e-3
                {
                    continue;
                }
                seen.push(e.id);
                let fw = op.frame.width.max(0.0);
                out.push((
                    r.e.id,
                    Win {
                        az,
                        la: (op.width - 2.0 * fw).max(0.0),
                        lb: (op.height - 2.0 * fw).max(0.0),
                        depth: (to_core + op.setback_in(wall, asm)).max(0.0),
                    },
                ));
            }
        }
    }
    out
}

/// The phase shift ψ of a wall `delta` m thick, h (formula 31).
pub fn psi(delta: f64) -> f64 {
    (32.0 * delta - 0.5).max(0.0)
}

/// The damping m of a wall `delta` m thick (formula 32).
pub fn damping(delta: f64) -> f64 {
    ((1.0 + 7.6 * delta) / 2500f64.powf(delta)).min(1.0)
}

/// The sol-air temperatures t_r of the 24 hours (index 0 = 24 h, 1 = 1 h
/// …) for outside air `te` of the hour and the sun `sun` on the face:
/// t_r = t_e + ε·I/α_e (formula 29).
pub fn sol_air(
    te: impl Fn(f64) -> f64,
    sun: impl Fn(i32) -> f64,
    eps: f64,
    alpha_e: f64,
) -> [f64; 24] {
    let mut out = [0.0; 24];
    for (i, x) in out.iter_mut().enumerate() {
        let h = if i == 0 { 24 } else { i as i32 };
        *x = te(h as f64) + eps * sun(h) / alpha_e;
    }
    out
}

/// Formula 30 for 1 m²K/W of k·S: (t_rm − t_i) + m·(t_rψ − t_rm) at
/// hour `h`, the sol-air temperature ψ hours before read between the
/// whole hours.
pub fn wall_dt(tr: &[f64; 24], ti: f64, delta: f64, h: f64) -> f64 {
    let mean = tr.iter().sum::<f64>() / 24.0;
    let at = (h - psi(delta)).rem_euclid(24.0);
    let i = at.floor() as usize % 24;
    let f = at - at.floor();
    let then = tr[i] + (tr[(i + 1) % 24] - tr[i]) * f;
    (mean - ti) + damping(delta) * (then - mean)
}

/// The layers of element `e` from one face: (thickness m, density
/// kg/m³ or none, material name); walls from the exterior, slabs from the
/// top.
fn layers(model: &Model, e: &creabim_core::Element) -> Vec<(f64, Option<f64>, String)> {
    let mat = |id: Option<ElementId>| {
        id.and_then(|m| model.get(m))
            .map_or((None, String::new()), |me| {
                let d = match &me.kind {
                    ElementKind::Material(x) => x.density,
                    _ => None,
                };
                (d, me.name.clone())
            })
    };
    if let Some((_, asm)) = creabim_core::assembly::assembly_of(model, e) {
        return asm
            .layers
            .iter()
            .filter(|l| l.thickness > 0.0)
            .map(|l| {
                // an air gap weighs nothing
                let (d, n) = if l.material.is_none() {
                    (Some(0.0), String::new())
                } else {
                    mat(l.material)
                };
                (l.thickness, d, n)
            })
            .collect();
    }
    let (m, d) = match &e.kind {
        ElementKind::Wall(w) => (w.material, w.thickness),
        ElementKind::Slab(s) => (s.material, s.thickness),
        _ => return vec![],
    };
    let (den, n) = mat(m);
    vec![(d, den, n)]
}

/// kg/m² of the layers down to `depth` from the first face (or from the
/// last with `back`); none when a layer in reach has no density.
fn mass_to(ls: &[(f64, Option<f64>, String)], depth: f64, back: bool) -> Option<f64> {
    let mut left = depth;
    let mut kg = 0.0;
    let it: Box<dyn Iterator<Item = &(f64, Option<f64>, String)>> = if back {
        Box::new(ls.iter().rev())
    } else {
        Box::new(ls.iter())
    };
    for (d, den, _) in it {
        if left <= 1e-9 {
            break;
        }
        let take = d.min(left);
        kg += take * (*den)?;
        left -= take;
    }
    Some(kg)
}

/// What the room stores for formula 27 (art. 69): kg, and what of it is
/// unknown (an element whose material has no density).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Storing {
    pub kg: f64,
    pub inner_walls: f64,
    pub floor: f64,
    pub ceiling: f64,
    /// Elements left out: a material with no density.
    pub unknown: usize,
    /// The floor is carpeted: a quarter of its mass counts.
    pub carpet: bool,
}

/// The mass of room `id` that stores the sun's heat (art. 69): half the
/// thickness of its inner walls, at most 0,08 m of a wall thicker than
/// 0,16 m (an inner wall's mean of both faces), half of the floor slab
/// from the top and of the ceiling slab from the bottom; the outside
/// walls and the furniture are not counted.
pub fn storing(model: &Model, id: ElementId, height: f64) -> Storing {
    let mut out = Storing::default();
    let rooms = rooms_of(model);
    let Some(r) = rooms.iter().find(|x| x.e.id == id) else {
        return out;
    };
    let ElementKind::Room(rm) = &r.e.kind else {
        return out;
    };
    let ring = &r.shape.outline;
    let n = ring.len();
    let mut seen: Vec<ElementId> = vec![];
    for i in 0..n {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let len = a.distance(b);
        if len < 1e-3 {
            continue;
        }
        let Some((we, wall)) = wall_of(model, r.shape.bounded_by.get(i).copied().flatten()) else {
            continue;
        };
        let past = beyond(a, b, wall.thickness);
        let inner = rooms.iter().any(|x| {
            x.e.id != id && x.level == r.level && polygon::contains(&x.shape.outline, past)
        });
        if !inner {
            continue;
        }
        let depth = if wall.thickness > 0.16 {
            0.08
        } else {
            wall.thickness / 2.0
        };
        let ls = layers(model, we);
        match (mass_to(&ls, depth, false), mass_to(&ls, depth, true)) {
            (Some(x), Some(y)) => out.inner_walls += (x + y) / 2.0 * len * height,
            _ => {
                if !seen.contains(&we.id) {
                    seen.push(we.id);
                    out.unknown += 1;
                }
            }
        }
    }
    let area = r.shape.area();
    let c = rm.at;
    let elev = |l: ElementId| model.level(l).map_or(0.0, |x| x.elevation);
    let me = elev(r.level);
    let slab_at = |pick: &dyn Fn(ElementId) -> bool| {
        model.iter().find(|e| {
            matches!(&e.kind, ElementKind::Slab(s)
                if pick(s.level) && s.boundary.len() > 2 && polygon::contains(&s.boundary, c))
        })
    };
    if let Some(s) = slab_at(&|l| l == r.level) {
        let ls = layers(model, s);
        let t: f64 = ls.iter().map(|x| x.0).sum();
        out.carpet = ls
            .first()
            .is_some_and(|x| x.2.to_lowercase().contains("koberec"));
        match mass_to(&ls, t / 2.0, false) {
            Some(kg) => out.floor = kg * area * if out.carpet { 0.25 } else { 1.0 },
            None => out.unknown += 1,
        }
    }
    // the ceiling: the floor slab of the nearest storey above over the
    // room's middle
    let above = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Slab(s)
                if elev(s.level) > me + 1e-6
                    && s.boundary.len() > 2
                    && polygon::contains(&s.boundary, c) =>
            {
                Some((elev(s.level), e))
            }
            _ => None,
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|x| x.1);
    if let Some(s) = above {
        let ls = layers(model, s);
        let t: f64 = ls.iter().map(|x| x.0).sum();
        match mass_to(&ls, t / 2.0, true) {
            Some(kg) => out.ceiling = kg * area,
            None => out.unknown += 1,
        }
    }
    out.kg = out.inner_walls + out.floor + out.ceiling;
    out
}

/// The thickness of the outside walls and roofs by the name of their
/// build-up (as the heat loss names them), m.
pub fn thickness_by_name(model: &Model) -> Vec<(String, f64)> {
    let mut out: Vec<(String, f64)> = vec![];
    for e in model.iter() {
        let t = match &e.kind {
            ElementKind::Wall(w) => w.thickness,
            ElementKind::Roof(r) => r.thickness,
            ElementKind::Ceiling(c) => c.thickness,
            _ => continue,
        };
        let name = match creabim_core::assembly::assembly_of(model, e) {
            Some((ae, _)) => ae.name.clone(),
            None => {
                let m = match &e.kind {
                    ElementKind::Wall(w) => w.material,
                    ElementKind::Roof(r) => r.material,
                    _ => None,
                };
                match m.and_then(|m| model.get(m)) {
                    Some(me) => me.name.clone(),
                    None => continue,
                }
            }
        };
        if !out.iter().any(|x| x.0 == name) {
            out.push((name, t));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phase_shift_and_damping_by_thickness() {
        // formula 31 and 32 as the norm prints them
        assert!((psi(0.3) - 9.1).abs() < 1e-9);
        assert_eq!(psi(0.01), 0.0);
        let m = damping(0.3);
        assert!((m - (1.0 + 7.6 * 0.3) / 2500f64.powf(0.3)).abs() < 1e-12);
        assert!(m > 0.3 && m < 0.32);
        assert_eq!(damping(0.0), 1.0);
    }

    #[test]
    fn a_light_wall_follows_the_sol_air_temperature() {
        // made-up numbers, not the norm's
        let tr = sol_air(
            |h| 20.0 + h,
            |h| if h == 12 { 150.0 } else { 0.0 },
            0.5,
            15.0,
        );
        assert!((tr[12] - (32.0 + 5.0)).abs() < 1e-9);
        assert!((tr[0] - 44.0).abs() < 1e-9);
        // δ 0: no shift, no damping: t_r − t_i of the hour
        assert!((wall_dt(&tr, 26.0, 0.0, 12.0) - (37.0 - 26.0)).abs() < 1e-9);
        // very thick: the day's mean only
        let mean = tr.iter().sum::<f64>() / 24.0;
        assert!((wall_dt(&tr, 26.0, 2.0, 12.0) - (mean - 26.0)).abs() < 1e-3);
    }

    #[test]
    fn the_sun_on_a_face_between_two_rows() {
        let text = "#table cool_sun_wall\n#columns key\th5\th6\th7\th8\th9\th10\th11\th12\th13\th14\th15\th16\th17\th18\th19\n\
S\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\t1\n\
SV\t3\t3\t3\t3\t3\t3\t3\t3\t3\t3\t3\t3\t3\t3\t3\n\
HOR\t9\t9\t9\t9\t9\t9\t9\t9\t9\t9\t9\t9\t9\t9\t9\n";
        let t = fireparams::parse(text);
        assert_eq!(sun_on(&t, Some(0.0), 12), Some(1.0));
        assert_eq!(sun_on(&t, Some(22.5), 12), Some(2.0));
        assert_eq!(sun_on(&t, None, 7), Some(9.0));
        // the night has no sun
        assert_eq!(sun_on(&t, None, 22), Some(0.0));
        // a face the table lacks
        assert_eq!(sun_on(&t, Some(90.0), 12), None);
    }

    #[test]
    fn mass_down_to_a_depth() {
        let ls = vec![
            (0.01, Some(1000.0), "omietka".to_string()),
            (0.2, Some(2000.0), "tehla".to_string()),
            (0.1, Some(30.0), "izolácia".to_string()),
        ];
        assert!((mass_to(&ls, 0.08, false).unwrap() - (10.0 + 0.07 * 2000.0)).abs() < 1e-9);
        assert!((mass_to(&ls, 0.12, true).unwrap() - (3.0 + 0.02 * 2000.0)).abs() < 1e-9);
        let gap = vec![(0.05, None, String::new())];
        assert_eq!(mass_to(&gap, 0.02, false), None);
    }

    #[test]
    fn the_sun_stands_as_table_4_says() {
        // table 4 (to the whole degree): July 10 h, June 12 h, March 7 h
        let (h, a) = sun_position(7, 10.0, 50.0);
        assert!((h - 52.0).abs() < 1.0 && (a - 131.0).abs() < 1.0, "{h} {a}");
        let (h, a) = sun_position(6, 12.0, 50.0);
        assert!((h - 63.0).abs() < 1.0 && (a - 180.0).abs() < 1.0, "{h} {a}");
        let (h, a) = sun_position(3, 7.0, 50.0);
        assert!((h - 10.0).abs() < 1.0 && (a - 101.0).abs() < 1.5, "{h} {a}");
        let (_, a) = sun_position(7, 16.0, 50.0);
        assert!((a - 260.0).abs() < 1.0, "{a}");
    }

    #[test]
    fn the_reveals_shade_the_glass_as_in_the_example() {
        // annex 1, P6: 2 × 1,3 m glass 0,2 m deep facing 135°, the sun at
        // a 131°, h 52° → S_os 2,07 m²
        let s = sunlit(2.0, 1.3, 0.2, 135.0, 52.0, 131.0);
        assert!((s - 2.07).abs() < 0.01, "{s}");
        assert_eq!(sunlit(2.0, 1.3, 0.2, 135.0, 52.0, 300.0), 0.0);
        assert_eq!(sunlit(2.0, 1.3, 0.0, 180.0, 40.0, 180.0), 2.6);
    }

    #[test]
    fn months_people_and_the_day_by_the_tables() {
        // made-up numbers, not the norm's
        let t = fireparams::parse(
            "#table cool_sun_wall\n#columns key\tmonth\th12\nJ\t7\t500\nJ\t3\t300\nS\t3\t100\n\
#table cool_te_max\n#columns key\tte_max\ttext\n3\t19\tx\n\
#table cool_people\n#columns key\tmet\ts21\ts24\ts25\ts26\ts28\ttext\na\t1\t90\t70\t66\t60\t50\tx\n",
        );
        assert_eq!(sun_of(&t, SUN_WALL, 7, Some(180.0), 12), Some(500.0));
        assert_eq!(sun_of(&t, SUN_WALL, 3, Some(180.0), 12), Some(300.0));
        assert_eq!(sun_of(&t, SUN_WALL, 4, Some(180.0), 12), None);
        assert_eq!(te_max(&t, 3), Some(19.0));
        assert_eq!(person_w(&t, "a", 26.0), Some(60.0));
        assert_eq!(person_w(&t, "a", 27.0), Some(55.0));
        assert_eq!(person_w(&t, "a", 30.0), Some(50.0));
        // formula 1: the highest at 15 h, 2A lower at 3 h
        assert!((te_at(30.0, 7.0, 15.0) - 30.0).abs() < 1e-9);
        assert!((te_at(30.0, 7.0, 3.0) - 16.0).abs() < 1e-9);
    }
}
