//! The rooms' design heat loss by the base method of STN EN 12831-1
//! chapter 6 ([`crate::heatloss::Method::Detailed`]):
//!
//! - through the envelope to the outside, H = Σ A·(U + ΔU_TB)·f_U, the
//!   areas by the outside dimensions (7.3.2, table 6): a room's outline
//!   carried out to the outer face of its outside walls and to the axis
//!   of the walls inside, the walls a storey (floor to floor) high;
//! - to colder rooms beside, over and under, H = Σ A·U·f, f = (θi − θx) /
//!   (θi − θe) (formula 7, 6.3.2.5), thermal bridges left out;
//! - to the ground, H = f_θann·Σ A·U_equiv·f_ig·f_GW (formula 8), U_equiv
//!   by annex E from B′ (E.2) and the floor's depth;
//! - by the air (6.3.3, a zone of the whole building, no supply air):
//!   the envelope's leakage q_env,50 = n50·V/A_env and, with extract
//!   fans, their air q_v,exh,z (24), with f_e,z (29), shared out by the
//!   rooms' envelope areas (18, 19); a room with a fan gets its air as
//!   transfer air through its door ([`crate::heatloss::air_of`], 15).
//!
//! Every number of the norm (ΔU_TB of table B.1, f_θann, f_GW, the
//! constants of table E.1, f_qv,z of table B.8, f_dir, ρ·c_p) is read
//! from the norm's parameter file the designer keeps (tables
//! [`TB`], [`FACTORS`], [`GROUND`], [`F_QV`]); a part whose table is not
//! read is left out and the room says so.

use crate::heatloss::{
    azimuth, beyond, overlap_area, room_number, rooms_of, round3, u_of, wall_of, BuildingLoss,
    Item, Missing, Opts, RoomLoss, Shielding, R_CEILING_UP, R_FLOOR, R_FLOOR_DOWN, R_INNER, R_ROOF,
    R_WALL,
};
use crate::psi::{self, Frame, Ins, Joint, Picks};
use creabim_core::fireparams::{self, ParamTable};
use creabim_core::{polygon, room, ElementKind, Model, OpeningKind};
use glam::DVec2;

/// Table B.1: ΔU_TB by the kind of building. Columns `key`, `criteria`,
/// `du` (W/(m²K)).
pub const TB: &str = "heat_tb";
/// Single factors: rows `f_theta_ann` (B.2.3), `f_gw_far`, `f_gw_near`
/// (B.2.3), `rho_cp` (B.2.8, Wh/(m³K)), `f_dir` (B.2.14), `f_fac_1`,
/// `f_fac_more` (f_fac,z of table B.9, B.2.15). Columns `key`,
/// `value`, `text`.
pub const FACTORS: &str = "heat_factors";
/// Table E.1: rows `floor` and `basement_wall`, columns `key`, `a`, `b`,
/// `c1`, `c2`, `c3`, `n1`, `n2`, `n3`, `d`.
pub const GROUND: &str = "heat_ground";
/// Table B.8, f_qv,z: columns `key`, `facades` ("1" or "2+"), `h_from`,
/// `h_to` (m above the ground, empty for any), `shielding` (`heavy`,
/// `moderate`, `none` or empty for any), `dh5`, `dh10`, `dh20`, `dh_more`
/// (the zone's height Δh up to 5, 10, 20 m and more).
pub const F_QV: &str = "heat_f_qv";
/// The tables of this method.
pub const TABLES: [&str; 4] = [TB, FACTORS, GROUND, F_QV];

/// The norm's tables for model `model`: the project's, else the
/// application's.
pub fn tables_of(model: &Model) -> Vec<ParamTable> {
    let own = model
        .project_info()
        .map(|(_, p)| p.fire_params.clone())
        .unwrap_or_default();
    fireparams::effective(&own)
        .into_iter()
        .filter(|t| {
            TABLES.contains(&t.id.as_str()) || t.id == crate::psi::PSI || t.id == room::ROOF_SPACE
        })
        .collect()
}

/// The rows of table B.1: what each is for and its ΔU_TB.
pub fn tb_rows(tables: &[ParamTable]) -> Vec<(String, f64)> {
    let Some(t) = tables.iter().find(|t| t.id == TB) else {
        return vec![];
    };
    let (Some(c), Some(d)) = (t.column("criteria"), t.column("du")) else {
        return vec![];
    };
    t.rows
        .iter()
        .filter_map(|r| Some((r.get(c)?.clone(), fireparams::number(r.get(d)?)?)))
        .collect()
}

fn factor(tables: &[ParamTable], key: &str) -> Option<f64> {
    let t = tables.iter().find(|t| t.id == FACTORS)?;
    fireparams::number(t.cell(key, "value"))
}

/// The constants of table E.1 for `key` (`floor`, `basement_wall`).
pub fn ground_constants(tables: &[ParamTable], key: &str) -> Option<[f64; 9]> {
    let t = tables.iter().find(|t| t.id == GROUND)?;
    let mut out = [0.0; 9];
    for (i, c) in ["a", "b", "c1", "c2", "c3", "n1", "n2", "n3", "d"]
        .iter()
        .enumerate()
    {
        out[i] = fireparams::number(t.cell(key, c))?;
    }
    Some(out)
}

/// U_equiv of annex E, formula E.1: `k` the constants of table E.1,
/// `bp` B′, `z` the depth of the floor under the ground, `u` U + ΔU_TB.
pub fn u_equiv(k: &[f64; 9], bp: f64, z: f64, u: f64) -> f64 {
    let [a, b, c1, c2, c3, n1, n2, n3, d] = *k;
    a / (b + (c1 + bp).powf(n1) + (c2 + z).powf(n2) + (c3 + u).powf(n3)) + d
}

/// f_qv,z of table B.8 for a zone with `many` exposed façades (more
/// than one), `h` m above the ground and `dh` m high.
pub fn f_qv(tables: &[ParamTable], many: bool, h: f64, dh: f64, s: Shielding) -> Option<f64> {
    let t = tables.iter().find(|t| t.id == F_QV)?;
    let col = |n: &str| t.column(n);
    let (cf, ch0, ch1, cs) = (
        col("facades")?,
        col("h_from")?,
        col("h_to")?,
        col("shielding")?,
    );
    let shield = match s {
        Shielding::None => "none",
        Shielding::Moderate => "moderate",
        Shielding::Heavy => "heavy",
    };
    let dcol = if dh <= 5.0 {
        "dh5"
    } else if dh <= 10.0 {
        "dh10"
    } else if dh <= 20.0 {
        "dh20"
    } else {
        "dh_more"
    };
    let dc = col(dcol)?;
    t.rows.iter().find_map(|r| {
        let f = r.get(cf)?.as_str();
        if (f == "1") == many {
            return None;
        }
        let lo = fireparams::number(r.get(ch0)?).unwrap_or(f64::MIN);
        let hi = fireparams::number(r.get(ch1)?).unwrap_or(f64::MAX);
        if h <= lo || h > hi {
            return None;
        }
        let sh = r.get(cs)?.as_str();
        if !sh.is_empty() && sh != shield {
            return None;
        }
        fireparams::number(r.get(dc)?)
    })
}

/// What one edge of a room's outline runs along.
#[derive(Clone, Copy, PartialEq)]
enum Edge {
    /// An outside wall of this thickness.
    Outside(f64),
    /// A wall to room `usize` of this thickness.
    Room(usize, f64),
    /// A wall to nothing found, or a separator.
    Other(f64),
}

impl Edge {
    /// How far the outside dimensions lie out of the room's face.
    fn out(self) -> f64 {
        match self {
            Edge::Outside(t) => t,
            Edge::Room(_, t) | Edge::Other(t) => t / 2.0,
        }
    }
}

/// The outline carried out by `out[i]` across each edge `i` (out is to
/// the right of an anticlockwise outline).
pub fn grown(outline: &[DVec2], out: &[f64]) -> Vec<DVec2> {
    let n = outline.len();
    let normal = |i: usize| {
        let d = (outline[(i + 1) % n] - outline[i]).normalize_or_zero();
        (d, DVec2::new(d.y, -d.x))
    };
    (0..n)
        .map(|i| {
            let p = (i + n - 1) % n;
            let ((dp, np), (di, ni)) = (normal(p), normal(i));
            let (a, b) = (outline[p] + np * out[p], outline[i] + ni * out[i]);
            let den = dp.perp_dot(di);
            if den.abs() < 1e-9 {
                return b;
            }
            let t = (b - a).perp_dot(di) / den;
            a + dp * t
        })
        .collect()
}

/// The heat loss of every room by the detailed method.
pub fn building(model: &Model, o: &Opts) -> BuildingLoss {
    let rooms = rooms_of(model);
    let tables = tables_of(model);
    let elev = |id| model.level(id).map_or(0.0, |l| l.elevation);
    let storey = |id| model.level(id).map_or(3.0, |l| l.height);
    let north = model.project_info().map_or(0.0, |(_, p)| p.true_north);
    let d_tb = tb_rows(&tables).get(o.tb_row).map(|r| r.1);
    let f_ann = factor(&tables, "f_theta_ann");
    let f_gw = factor(&tables, if o.gw_near { "f_gw_near" } else { "f_gw_far" });
    let rho_cp = factor(&tables, "rho_cp");
    let f_dir = factor(&tables, "f_dir");
    let floor_k = ground_constants(&tables, "floor");
    let psi_rows = psi::rows(&tables);
    let room_at = |level, p: DVec2, not| {
        rooms.iter().position(|x| {
            x.level == level && x.e.id != not && polygon::contains(&x.shape.outline, p)
        })
    };
    // each room's edges and its outline by the outside dimensions
    let edges: Vec<Vec<Edge>> = rooms
        .iter()
        .map(|r| {
            let n = r.shape.outline.len();
            (0..n)
                .map(|i| {
                    let (a, b) = (r.shape.outline[i], r.shape.outline[(i + 1) % n]);
                    match wall_of(model, r.shape.bounded_by.get(i).copied().flatten()) {
                        None => Edge::Other(0.0),
                        Some((_, w)) => match room_at(r.level, beyond(a, b, w.thickness), r.e.id) {
                            None => Edge::Outside(w.thickness),
                            Some(j) => Edge::Room(j, w.thickness),
                        },
                    }
                })
                .collect()
        })
        .collect();
    let outer: Vec<Vec<DVec2>> = rooms
        .iter()
        .zip(&edges)
        .map(|(r, es)| {
            grown(
                &r.shape.outline,
                &es.iter().map(|e| e.out()).collect::<Vec<_>>(),
            )
        })
        .collect();
    let exposed = |k: usize| -> f64 {
        let o = &outer[k];
        let n = o.len();
        (0..n)
            .filter(|i| matches!(edges[k][*i], Edge::Outside(_)))
            .map(|i| o[i].distance(o[(i + 1) % n]))
            .sum()
    };
    // B′ of a level: the floor area over half the exposed perimeter (E.2)
    let bp_level = |level| {
        let (mut a, mut p) = (0.0, 0.0);
        for (k, r) in rooms.iter().enumerate() {
            if r.level == level {
                a += polygon::signed_area2(&outer[k]).abs() / 2.0;
                p += exposed(k);
            }
        }
        (p > 1e-6).then(|| a / (0.5 * p))
    };
    // the zone (the whole building) for the air: its height and the
    // façades with openings
    let (z_lo, z_hi) = rooms.iter().fold((f64::MAX, f64::MIN), |(lo, hi), r| {
        let ElementKind::Room(rm) = &r.e.kind else {
            return (lo, hi);
        };
        let z = elev(r.level);
        (lo.min(z), hi.max(z + room::height(model, rm)))
    });
    let mut out = BuildingLoss::default();
    // per room: transmission first, the envelope's area for the air
    struct Part {
        items: Vec<Item>,
        missing: Vec<Missing>,
        a_env: f64,
        facades: Vec<f64>,
        volume: f64,
    }
    let mut parts: Vec<Part> = vec![];
    for (k, r) in rooms.iter().enumerate() {
        let ElementKind::Room(rm) = &r.e.kind else {
            continue;
        };
        let ti = r.ti;
        let dt = ti - o.te;
        let h = room::height(model, rm);
        let hs = storey(r.level);
        let mut items: Vec<Item> = vec![];
        let mut missing: Vec<Missing> = vec![];
        let mut miss = |m: Missing| {
            if !missing.contains(&m) {
                missing.push(m);
            }
        };
        let mut add =
            |kind: &'static str, detail: &str, a: f64, u: f64, f: f64, az: Option<f64>| {
                if a <= 1e-4 || u <= 0.0 {
                    return;
                }
                let phi = a * u * f * dt;
                match items.iter_mut().find(|x| {
                    x.kind == kind
                        && x.detail == detail
                        && (x.u - u).abs() < 1e-9
                        && x.f == f
                        && x.azimuth == az
                }) {
                    Some(x) => {
                        x.area += a;
                        x.phi += phi;
                    }
                    None => items.push(Item {
                        kind,
                        detail: detail.into(),
                        area: a,
                        u,
                        f,
                        phi,
                        azimuth: az,
                    }),
                }
            };
        // with Σ l·Ψ (annex C) the areas take no ΔU_TB
        let tb = if o.psi {
            0.0
        } else {
            d_tb.unwrap_or_else(|| {
                miss(Missing::NormTable);
                0.0
            })
        };
        // the junctions for Σ l·Ψ: kind, the detail left to itself, length
        let mut joints: Vec<(Joint, String, f64)> = vec![];
        let mut ins_of: Vec<Option<Ins>> = vec![None; r.shape.outline.len()];
        let mut a_env = 0.0;
        let mut facades: Vec<f64> = vec![];
        let mut seen = vec![];
        let shape = &r.shape;
        let n = shape.outline.len();
        let ring = &outer[k];
        for i in 0..n {
            let (a, b) = (shape.outline[i], shape.outline[(i + 1) % n]);
            if a.distance(b) < 1e-3 {
                continue;
            }
            let Some((we, wall)) = wall_of(model, shape.bounded_by.get(i).copied().flatten())
            else {
                continue;
            };
            let len = ring[i].distance(ring[(i + 1) % n]);
            let gross = len * hs;
            match edges[k][i] {
                Edge::Outside(_) => {
                    a_env += gross;
                    let ins = psi::insulation(model, we);
                    ins_of[i] = Some(ins);
                    let u = (wall.end - wall.start).normalize_or_zero();
                    let (s0, s1) = {
                        let (x, y) = ((a - wall.start).dot(u), (b - wall.start).dot(u));
                        (x.min(y), x.max(y))
                    };
                    let az = azimuth(DVec2::new((b - a).y, -(b - a).x), north);
                    let mut net = gross;
                    for e in model.iter() {
                        let ElementKind::Opening(op) = &e.kind else {
                            continue;
                        };
                        let mid = op.offset + op.width / 2.0;
                        if op.host != we.id
                            || op.kind == OpeningKind::Hole
                            || seen.contains(&e.id)
                            || mid < s0 - 1e-3
                            || mid > s1 + 1e-3
                        {
                            continue;
                        }
                        seen.push(e.id);
                        let ang = u.y.atan2(u.x);
                        if !facades
                            .iter()
                            .any(|x: &f64| ((x - ang).sin().atan2((x - ang).cos())).abs() < 0.35)
                        {
                            facades.push(ang);
                        }
                        let oa = (op.width * op.height).min(net.max(0.0));
                        net -= oa;
                        let (kind, uo) = if op.kind == OpeningKind::Window {
                            ("Window", o.u_window)
                        } else {
                            ("Outside door", o.u_door)
                        };
                        add(kind, "", oa, uo + tb, 1.0, Some(az));
                        // the frame's edge: a door's threshold left out
                        let frame = psi::frame_of(model, we, wall, op);
                        let lapped = op
                            .reveal
                            .unwrap_or(creabim_core::assembly::REVEAL_INSULATION)
                            > 1e-3;
                        let edge = if op.kind == OpeningKind::Window {
                            2.0 * (op.width + op.height)
                        } else {
                            op.width + 2.0 * op.height
                        };
                        joints.push((
                            Joint::Window,
                            psi::auto_key(Joint::Window, ins, frame, lapped),
                            edge,
                        ));
                    }
                    match u_of(model, we, R_WALL) {
                        Some((uw, name)) => {
                            add("Outside wall", &name, net.max(0.0), uw + tb, 1.0, Some(az))
                        }
                        None => miss(Missing::WallBuildUp),
                    }
                }
                Edge::Room(j, _) if ti - rooms[j].ti > 0.05 => {
                    let x = &rooms[j];
                    let label = format!("{} {} ({} °C)", room_number(x.e), x.e.name, x.ti);
                    match u_of(model, we, R_INNER) {
                        Some((uw, _)) => add(
                            "To the room",
                            label.trim(),
                            gross,
                            uw,
                            round3((ti - x.ti) / dt),
                            None,
                        ),
                        None => miss(Missing::InnerWallBuildUp),
                    }
                }
                _ => {}
            }
        }
        // the corners of the outside walls and the inner walls meeting
        // them, a storey high (an inner wall's half to each room beside it)
        for i in 0..n {
            let p = (i + n - 1) % n;
            let (ea, eb) = (&edges[k][p], &edges[k][i]);
            let (a, b, c) = (ring[p], ring[i], ring[(i + 1) % n]);
            match (ea, eb) {
                (Edge::Outside(_), Edge::Outside(_)) => {
                    let cross = (b - a).perp_dot(c - b);
                    if cross.abs() < 1e-6 * (b - a).length() * (c - b).length() {
                        continue;
                    }
                    let ins = ins_of[i].or(ins_of[p]).unwrap_or(Ins::Even);
                    // the outline runs anticlockwise: a left turn is an outside corner
                    let j = if cross > 0.0 {
                        Joint::CornerOut
                    } else {
                        Joint::CornerIn
                    };
                    joints.push((j, psi::auto_key(j, ins, Frame::Outer, false), hs));
                }
                (Edge::Outside(_), Edge::Room(..)) | (Edge::Room(..), Edge::Outside(_)) => {
                    let ins = ins_of[i].or(ins_of[p]).unwrap_or(Ins::Even);
                    let j = Joint::InnerWall;
                    joints.push((j, psi::auto_key(j, ins, Frame::Outer, false), hs / 2.0));
                }
                _ => {}
            }
        }
        // the outside walls' outer lengths, for the floors, roof and ground
        // along them
        let along: Vec<(Ins, f64)> = (0..n)
            .filter_map(|i| Some((ins_of[i]?, ring[i].distance(ring[(i + 1) % n]))))
            .collect();
        let mut along_with = |j: Joint, share: f64| {
            for (ins, l) in &along {
                joints.push((j, psi::auto_key(j, *ins, Frame::Outer, false), l * share));
            }
        };
        let area_out = polygon::signed_area2(ring).abs() / 2.0;
        let c = rm.at;
        let me = elev(r.level);
        // the storeys right over and under that have rooms
        let next = |up: bool| {
            rooms
                .iter()
                .map(|x| elev(x.level))
                .filter(|z| if up { *z > me + 1e-6 } else { *z < me - 1e-6 })
                .fold(None, |m: Option<f64>, z| {
                    Some(m.map_or(z, |m: f64| if up { m.min(z) } else { m.max(z) }))
                })
        };
        let mut covered = [0.0f64; 2];
        for (side, up) in [(0usize, true), (1, false)] {
            let Some(z) = next(up) else { continue };
            for (j, x) in rooms.iter().enumerate() {
                if (elev(x.level) - z).abs() > 1e-6 {
                    continue;
                }
                let shared = overlap_area(ring, &outer[j]);
                if shared < 1e-3 {
                    continue;
                }
                covered[side] += shared;
                if ti - x.ti <= 0.05 {
                    continue;
                }
                // the slab between: the upper room's floor
                let (upper, probe) = if up {
                    (
                        x,
                        match &x.e.kind {
                            ElementKind::Room(q) => q.at,
                            _ => c,
                        },
                    )
                } else {
                    (r, c)
                };
                let slab = model.iter().find(|e| {
                    matches!(&e.kind, ElementKind::Slab(s)
                        if s.level == upper.level && s.boundary.len() > 2
                            && polygon::contains(&s.boundary, probe))
                });
                let label = format!("{} {} ({} °C)", room_number(x.e), x.e.name, x.ti);
                match slab
                    .and_then(|s| u_of(model, s, if up { R_CEILING_UP } else { R_FLOOR_DOWN }))
                {
                    Some((u, _)) => add(
                        if up {
                            "To the room above"
                        } else {
                            "To the room below"
                        },
                        label.trim(),
                        shared,
                        u,
                        round3((ti - x.ti) / dt),
                        None,
                    ),
                    None => miss(Missing::InnerSlabBuildUp),
                }
            }
        }
        // the floor on the ground, where no room is under the room's middle
        // a floor between storeys: half its edge to the room over it, half
        // to the room under it
        for side in [0usize, 1] {
            if covered[side] > 1e-3 {
                along_with(Joint::Floor, 0.5);
            }
        }
        let below = covered[1] > 1e-3
            || rooms
                .iter()
                .any(|x| elev(x.level) < me - 1e-6 && polygon::contains(&x.shape.outline, c));
        if !below {
            let slab = model.iter().find(|e| {
                matches!(&e.kind, ElementKind::Slab(s)
                    if s.level == r.level && s.boundary.len() > 2 && polygon::contains(&s.boundary, c))
            });
            match slab.and_then(|s| u_of(model, s, R_FLOOR)) {
                Some((u, name)) => {
                    a_env += area_out;
                    // B′ of the whole storey for a room with no outside wall
                    // or a well insulated floor, else the room's own (E.2)
                    let own = exposed(k);
                    let bp = if own < 1e-6 || u <= 0.5 {
                        bp_level(r.level)
                    } else {
                        Some(area_out / (0.5 * own))
                    };
                    let z = (-me).max(0.0);
                    match (floor_k, f_ann, f_gw, o.te_mean) {
                        (Some(kf), Some(fa), Some(fg), Some(tm)) => {
                            let ue =
                                u_equiv(&kf, bp.unwrap_or(f64::MAX).clamp(1e-3, 1e6), z, u + tb);
                            let f_ig = (ti - tm) / dt;
                            along_with(Joint::Ground, 1.0);
                            add(
                                "Floor on the ground",
                                &name,
                                area_out,
                                round3(ue),
                                round3(fa * f_ig * fg),
                                None,
                            );
                        }
                        (_, _, _, None) => miss(Missing::MeanTemperature),
                        _ => miss(Missing::NormTable),
                    }
                }
                None => miss(Missing::FloorBuildUp),
            }
        }
        // upwards: a ceiling with a build-up (podhľad) and the roof space
        // over it (STN EN ISO 6946 table 9), else a roof, where no room is
        // over it
        let open_up = covered[0] < 1e-3
            && !rooms
                .iter()
                .any(|x| elev(x.level) > me + 1e-6 && polygon::contains(&x.shape.outline, c));
        let ceiling = open_up
            .then(|| room::built_ceiling_over(model, r.e.id, r.level, c))
            .flatten();
        if let Some(ce) = ceiling {
            match crate::heatloss::ceiling_u(model, ce, &tables) {
                Some((u, name)) => {
                    a_env += area_out;
                    along_with(Joint::Roof, 1.0);
                    add("Ceiling", &name, area_out, u + tb, 1.0, None)
                }
                None => miss(Missing::RoofBuildUp),
            }
        } else if open_up {
            let roof = model.iter().find(|e| {
                matches!(&e.kind, ElementKind::Roof(rf)
                    if rf.boundary.len() > 2 && polygon::contains(&rf.boundary, c))
            });
            match roof {
                Some(e) => {
                    let ElementKind::Roof(rf) = &e.kind else {
                        unreachable!()
                    };
                    let slope = if matches!(rf.shape, creabim_core::RoofShape::Flat) {
                        1.0
                    } else {
                        1.0 / rf.pitch.clamp(0.0, 80.0).to_radians().cos()
                    };
                    match u_of(model, e, R_ROOF) {
                        Some((u, name)) => {
                            a_env += area_out * slope;
                            along_with(Joint::Roof, 1.0);
                            add("Roof", &name, area_out * slope, u + tb, 1.0, None)
                        }
                        None => miss(Missing::RoofBuildUp),
                    }
                }
                None => miss(Missing::NoRoofOrCeiling),
            }
        }
        if o.psi {
            let picks = Picks {
                rows: &psi_rows,
                pick: o.psi_pick,
            };
            if psi_rows.is_empty() {
                miss(Missing::NormTable);
            }
            for (j, auto, l) in joints {
                let Some(row) = picks.row(j, &auto) else {
                    continue;
                };
                if l <= 1e-4 || row.psi_e == 0.0 {
                    continue;
                }
                let detail = format!("{} {}", row.key, row.text);
                let phi = l * row.psi_e * dt;
                match items
                    .iter_mut()
                    .find(|x| x.kind == "Thermal bridge" && x.detail == detail)
                {
                    Some(x) => {
                        x.area += l;
                        x.phi += phi;
                    }
                    None => items.push(Item {
                        kind: "Thermal bridge",
                        detail: detail.trim().to_string(),
                        area: l,
                        u: row.psi_e,
                        f: 1.0,
                        phi,
                        azimuth: None,
                    }),
                }
            }
        }
        parts.push(Part {
            items,
            missing,
            a_env,
            facades,
            volume: shape.area() * h,
        });
    }
    // the air of the zone (6.3.3.3: no supply air, no air terminal
    // devices, no large openings)
    let a_env_z: f64 = parts.iter().map(|p| p.a_env).sum();
    let v_z: f64 = parts.iter().map(|p| p.volume).sum();
    let many = {
        let mut dirs: Vec<f64> = vec![];
        for p in &parts {
            for a in &p.facades {
                if !dirs
                    .iter()
                    .any(|x| ((x - a).sin().atan2((x - a).cos())).abs() < 0.35)
                {
                    dirs.push(*a);
                }
            }
        }
        dirs.len() > 1
    };
    // the zone's top over the ground (±0.000 taken as the ground)
    let h_z = z_hi.max(0.0);
    let fqv = f_qv(&tables, many, h_z, (z_hi - z_lo).max(0.0), o.shielding);
    // q_env,50 = n50·V/A_env (6.3.3.4)
    let q_env50 = if a_env_z > 1e-6 {
        o.n50 * v_z / a_env_z
    } else {
        0.0
    };
    // the fans' extract air, unbalanced (no supply): q_v,exh,z
    let q_exh_z: f64 = if o.exhaust_fans {
        rooms
            .iter()
            .map(|r| crate::heatloss::q_exh_of(r, true))
            .sum()
    } else {
        0.0
    };
    let leak50 = q_env50 * a_env_z;
    // f_e,z (29): 1 when nothing is taken out unbalanced; f_fac,z of
    // table B.9 by the exposed façades
    let f_e = if q_exh_z > 1e-9 && leak50 > 1e-9 {
        match (
            factor(&tables, if many { "f_fac_more" } else { "f_fac_1" }),
            fqv,
        ) {
            (Some(ff), Some(fq)) => Some(1.0 / (1.0 + ff / fq * (q_exh_z / leak50).powi(2))),
            _ => None,
        }
    } else {
        Some(1.0)
    };
    // q_v,inf-add,z (28) and q_v,env,z (24)
    let q_add_z = fqv.zip(f_e).map(|(f, fe)| leak50 * f * fe);
    let q_env_z = q_add_z.map(|a| q_exh_z + a);
    for (k, (r, mut p)) in rooms.iter().zip(parts).enumerate() {
        let ElementKind::Room(rm) = &r.e.kind else {
            continue;
        };
        let dt = r.ti - o.te;
        // q_v,env,i (18) with q_v,leak+ATD,i = q_v,env,z·A_env,i/A_env,z (19)
        let v_inf = match (q_env_z, q_add_z, f_dir) {
            (Some(q), Some(add), Some(fd)) if a_env_z > 1e-6 && q > 1e-9 => {
                let leak = q * p.a_env / a_env_z;
                add / q * q.min(leak * fd) + (q - add) / q * leak
            }
            (Some(_), Some(_), Some(_)) => 0.0,
            _ => {
                if !p.missing.contains(&Missing::NormTable) {
                    p.missing.push(Missing::NormTable);
                }
                0.0
            }
        };
        let (v_min, v_out, v_transfer, t_transfer) =
            crate::heatloss::air_of(model, &rooms, k, p.volume, v_inf, o.exhaust_fans);
        let phi_t: f64 = p.items.iter().map(|x| x.phi).sum();
        let phi_v = match rho_cp {
            Some(c) => c * (v_out * dt + v_transfer * (r.ti - t_transfer.unwrap_or(r.ti))),
            None => {
                if !p.missing.contains(&Missing::NormTable) {
                    p.missing.push(Missing::NormTable);
                }
                0.0
            }
        };
        let area = r.shape.area();
        out.phi += phi_t + phi_v;
        out.area += area;
        out.rooms.push(RoomLoss {
            id: r.e.id,
            kind: r.kind,
            number: rm.number.clone(),
            name: r.e.name.clone(),
            level: model.get(r.level).map_or(String::new(), |l| l.name.clone()),
            ti: r.ti,
            area,
            volume: p.volume,
            items: p.items,
            phi_t,
            phi_v,
            phi: phi_t + phi_v,
            v_min,
            v_inf,
            v_transfer,
            t_transfer,
            facades: p.facades.len(),
            missing: p.missing,
        });
    }
    out.rooms.sort_by(|a, b| a.number.cmp(&b.number));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heatloss;

    #[test]
    fn the_outline_grows_out_by_each_edge() {
        let sq = polygon::rectangle(DVec2::ZERO, DVec2::new(4.0, 3.0));
        // all round by 0.5: 5 × 4
        let g = grown(&sq, &[0.5; 4]);
        assert!((polygon::signed_area2(&g).abs() / 2.0 - 20.0).abs() < 1e-9);
        // one side only
        let g = grown(&sq, &[0.0, 0.5, 0.0, 0.0]);
        assert!((polygon::signed_area2(&g).abs() / 2.0 - 13.5).abs() < 1e-9);
    }

    /// Test tables in the file's form, with made-up numbers that only
    /// show which cell is read: not the norm's.
    fn test_tables() -> Vec<ParamTable> {
        fireparams::parse(
            "#table heat_tb\n#columns key\tcriteria\tdu\nA\tfirst\t0,01\nB\tsecond\t0,02\n\
             #table heat_factors\n#columns key\tvalue\ttext\nf_theta_ann\t1,1\t\nf_gw_far\t1\t\nf_gw_near\t1,2\t\nrho_cp\t0,3\t\nf_dir\t3\t\nf_fac_1\t10\t\nf_fac_more\t6\t\n\
             #table heat_ground\n#columns key\ta\tb\tc1\tc2\tc3\tn1\tn2\tn3\td\nfloor\t1\t0\t0\t0\t0\t1\t0\t-1\t0\n\
             #table heat_f_qv\n#columns key\tfacades\th_from\th_to\tshielding\tdh5\tdh10\tdh20\tdh_more\n\
             one\t1\t\t\t\t0,1\t0,2\t0,3\t0,4\nmore\t2+\t0\t50\tmoderate\t0,5\t0,6\t0,7\t0,8\nhigh\t2+\t50\t\t\t0,9\t0,9\t0,9\t0,9\n",
        )
    }

    #[test]
    fn the_tables_are_read_by_their_keys() {
        let t = test_tables();
        assert_eq!(tb_rows(&t).len(), 2);
        assert_eq!(tb_rows(&t)[1].1, 0.02);
        assert_eq!(factor(&t, "f_gw_near"), Some(1.2));
        assert_eq!(f_qv(&t, false, 3.0, 3.0, Shielding::None), Some(0.1));
        assert_eq!(f_qv(&t, true, 3.0, 7.0, Shielding::Moderate), Some(0.6));
        assert_eq!(f_qv(&t, true, 60.0, 7.0, Shielding::Heavy), Some(0.9));
        assert_eq!(f_qv(&t, true, 3.0, 7.0, Shielding::Heavy), None);
        // a / (b + (c1+B')^1 + (c2+z)^0 + (c3+u)^-1) + d = 1 / (B' + 1 + 1/u)
        let k = ground_constants(&t, "floor").unwrap();
        assert!((u_equiv(&k, 8.0, 0.0, 0.5) - 1.0 / 11.0).abs() < 1e-12);
    }

    fn detailed() -> Opts {
        Opts {
            method: heatloss::Method::Detailed,
            te_mean: Some(9.0),
            ..Opts::default()
        }
    }

    #[test]
    fn a_room_with_no_tables_says_what_it_lacks() {
        let model = heatloss::test_house();
        let b = building(&model, &detailed());
        assert_eq!(b.rooms.len(), 2);
        for r in &b.rooms {
            assert!(r.missing.contains(&Missing::NormTable), "{r:?}");
        }
    }

    #[test]
    fn the_detailed_method_takes_the_outside_dimensions() {
        let mut doc = creabim_core::Document::from_model(heatloss::test_house());
        let mut info = doc
            .model()
            .project_info()
            .map(|(_, p)| p.clone())
            .unwrap_or_default();
        info.fire_params = test_tables();
        let tx = creabim_core::set_project(doc.model(), info);
        doc.commit(tx).unwrap();
        let model = doc.model().clone();
        let o = detailed();
        let d = building(&model, &o);
        let s = heatloss::building(&model, &Opts::default());
        assert_eq!(d.rooms.len(), 2);
        for (rd, rs) in d.rooms.iter().zip(&s.rooms) {
            // the sample house has no roof
            assert!(
                rd.missing.iter().all(|m| *m == Missing::NoRoofOrCeiling),
                "{:?}",
                rd.missing
            );
            let wall = |r: &RoomLoss| -> f64 {
                r.items
                    .iter()
                    .filter(|i| matches!(i.kind, "Outside wall" | "Window" | "Outside door"))
                    .map(|i| i.area)
                    .sum()
            };
            // the outside faces are longer than the inside ones
            assert!(wall(rd) > wall(rs), "{} {}", wall(rd), wall(rs));
            let sum: f64 = rd.items.iter().map(|i| i.phi).sum();
            assert!((sum - rd.phi_t).abs() < 1e-6);
            assert!(rd.phi_v > 0.0);
        }
        // ΔU_TB of the row chosen
        let w = d.rooms[1]
            .items
            .iter()
            .find(|i| i.kind == "Window")
            .unwrap();
        assert!((w.u - (o.u_window + 0.02)).abs() < 1e-9);
        // the ground: f = f_θann·f_ig·f_GW
        let g = d.rooms[0]
            .items
            .iter()
            .find(|i| i.kind == "Floor on the ground")
            .unwrap();
        let ti = d.rooms[0].ti;
        assert!((g.f - round3(1.1 * (ti - 9.0) / (ti - o.te))).abs() < 1e-9);
        let near = building(&model, &Opts { gw_near: true, ..o });
        let gn = near.rooms[0]
            .items
            .iter()
            .find(|i| i.kind == "Floor on the ground")
            .unwrap();
        assert!(gn.f > g.f);
    }

    #[test]
    fn the_fans_air_by_formulas_24_28_29() {
        let mut doc = creabim_core::Document::from_model(heatloss::test_house());
        let mut info = doc
            .model()
            .project_info()
            .map(|(_, p)| p.clone())
            .unwrap_or_default();
        // f_dir = 1: the rooms' shares add up to the zone's air
        let mut t = test_tables();
        for x in t.iter_mut().filter(|x| x.id == FACTORS) {
            *x = fireparams::parse(
                "#table heat_factors\n#columns key\tvalue\ttext\nf_theta_ann\t1,1\t\nf_gw_far\t1\t\nrho_cp\t0,3\t\nf_dir\t1\t\nf_fac_1\t10\t\nf_fac_more\t6\t\n",
            )
            .remove(0);
        }
        info.fire_params = t;
        let tx = creabim_core::set_project(doc.model(), info);
        doc.commit(tx).unwrap();
        let model = doc.model().clone();
        let off = building(
            &model,
            &Opts {
                exhaust_fans: false,
                ..detailed()
            },
        );
        let on = building(
            &model,
            &Opts {
                exhaust_fans: true,
                ..detailed()
            },
        );
        let sum = |b: &BuildingLoss| b.rooms.iter().map(|r| r.v_inf).sum::<f64>();
        // q_v,inf-add,z with f_e = 1 is the air without fans
        let add0 = sum(&off);
        let leak50 = detailed().n50 * off.rooms.iter().map(|r| r.volume).sum::<f64>();
        let fqv = add0 / leak50;
        let fe = 1.0 / (1.0 + 6.0 / fqv * (90.0 / leak50).powi(2));
        assert!(
            (sum(&on) - (90.0 + add0 * fe)).abs() < 1e-6,
            "{} {}",
            sum(&on),
            add0
        );
        assert!(on
            .rooms
            .iter()
            .all(|r| !r.missing.contains(&Missing::NormTable)));
    }

    #[test]
    fn thermal_bridges_as_psi_times_length() {
        let mut doc = creabim_core::Document::from_model(heatloss::test_house());
        let mut info = doc
            .model()
            .project_info()
            .map(|(_, p)| p.clone())
            .unwrap_or_default();
        // made-up Ψ in the norm's table's shape, not the norm's
        let mut t = test_tables();
        let mut text =
            String::from("#table heat_psi\n#columns key\tgroup\tpsi_e\tpsi_oi\tpsi_i\ttext\n");
        for (g, n) in [
            ("C", 8),
            ("IF", 4),
            ("R", 4),
            ("GF", 4),
            ("IW", 4),
            ("W", 18),
        ] {
            for i in 1..=n {
                text += &format!("{g}{i}\t{g}\t0,1\t0\t0\t{g}\n");
            }
        }
        t.extend(fireparams::parse(&text));
        info.fire_params = t;
        let tx = creabim_core::set_project(doc.model(), info);
        doc.commit(tx).unwrap();
        let model = doc.model().clone();
        let o = Opts {
            psi: true,
            ..detailed()
        };
        let d = building(&model, &o);
        let b1 = building(&model, &detailed());
        for (r, rb) in d.rooms.iter().zip(&b1.rooms) {
            let bridges: Vec<&Item> = r
                .items
                .iter()
                .filter(|i| i.kind == "Thermal bridge")
                .collect();
            assert!(!bridges.is_empty(), "{:?}", r.items);
            // no ΔU_TB on the areas
            let w = r.items.iter().find(|i| i.kind == "Window").unwrap();
            assert!((w.u - o.u_window).abs() < 1e-9);
            // windows round their whole edge, corners a storey high
            let win = bridges.iter().find(|i| i.detail.starts_with('W')).unwrap();
            assert!(win.area > 0.0 && (win.u - 0.1).abs() < 1e-9);
            assert!(bridges
                .iter()
                .all(|b| (b.phi - b.area * b.u * (r.ti - o.te)).abs() < 1e-9));
            let sum: f64 = r.items.iter().map(|i| i.phi).sum();
            assert!((sum - r.phi_t).abs() < 1e-6);
            assert_ne!(r.phi_t, rb.phi_t);
        }
        // a detail picked for a kind of junction takes its row
        let mut pick = [None; 7];
        pick[crate::psi::slot(Joint::Window)] = Some(0);
        let other = building(
            &model,
            &Opts {
                psi_pick: pick,
                ..o
            },
        );
        // row 0 is C1, not a window detail: the windows are left out
        assert!(other.rooms[1]
            .items
            .iter()
            .all(|i| !(i.kind == "Thermal bridge" && i.detail.starts_with('W'))));
    }
}
