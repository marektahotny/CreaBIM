//! Meshes and plan linework for columns, beams, roofs, stairs and railings
//! (see `creabim_core::members`).

use crate::plan::{Pen, PlanItem};
use crate::scene::slab_mesh;
use crate::triangulate::triangulate;
use crate::TriMesh;
use creabim_core::members;
use creabim_core::{Element, ElementId, ElementKind, Model, Roof, Stair};
use glam::{DVec2, DVec3};

/// A beam: its section (a library one, or width × depth) swept along its
/// top line, sloped by its rise, lying on the face it was drawn on, run
/// past or pulled back by its extensions and its ends cut where it meets
/// the next beam of its chain (MU40).
pub fn beam_mesh(model: &Model, b: &creabim_core::Beam) -> TriMesh {
    let prof: Vec<DVec2> = match &b.section {
        Some(s) if s.points.len() >= 3 => {
            let hi = s.points.iter().map(|p| p.y).fold(f64::MIN, f64::max);
            s.points.iter().map(|p| DVec2::new(p.x, p.y - hi)).collect()
        }
        _ => {
            let (w, d) = (b.width / 2.0, b.depth);
            vec![
                DVec2::new(-w, -d),
                DVec2::new(w, -d),
                DVec2::new(w, 0.0),
                DVec2::new(-w, 0.0),
            ]
        }
    };
    let (a, z) = b.axis(model);
    let mut m = match b.up {
        Some(n) => crate::parts::sweep_mesh_on(&prof, &[a, z], false, n),
        None => crate::parts::sweep_mesh(&prof, &[a, z], false),
    };
    // each end cut by its plane, through the corner it meets the next at:
    // the end's vertices slid along the beam onto the plane
    let dir = (z - a).normalize_or_zero();
    if dir == DVec3::ZERO {
        return m;
    }
    let top = b.top(model);
    let corners = [
        b.start.extend(top + b.rise[0]),
        b.end.extend(top + b.rise[1]),
    ];
    let len = a.distance(z);
    for (k, cut) in b.cuts.iter().enumerate() {
        let Some(n) = cut.map(|n| n.normalize_or_zero()) else {
            continue;
        };
        let along = n.dot(dir);
        if along.abs() < 0.05 {
            continue;
        }
        // a two-point sweep has only its two end rings: a vertex belongs
        // to the end it is nearer (MU40: on a sloped beam the section's
        // top and bottom sit off the end's plane square to the axis, and
        // were left uncut, ragged)
        let at_end = |p: DVec3| {
            let s = (p - a).dot(dir);
            if k == 0 {
                s < len / 2.0
            } else {
                s >= len / 2.0
            }
        };
        let out = if k == 0 { -dir } else { dir };
        for (p, nm) in m.positions.iter_mut().zip(m.normals.iter_mut()) {
            let q = DVec3::new(p[0] as f64, p[1] as f64, p[2] as f64);
            if !at_end(q) {
                continue;
            }
            let t = (corners[k] - q).dot(n) / along;
            *p = (q + dir * t).as_vec3().to_array();
            // the end face takes the plane's normal, facing out
            let nv = DVec3::new(nm[0] as f64, nm[1] as f64, nm[2] as f64);
            if nv.dot(out) > 0.9 {
                let face = if n.dot(out) >= 0.0 { n } else { -n };
                *nm = face.as_vec3().to_array();
            }
        }
    }
    m
}

/// A timber member from its side outline: the outline as a slab across
/// its width, turned into the upright plane of its axis.
pub fn side_profile_mesh(p: &creabim_core::framing::SideProfile) -> TriMesh {
    let mut m = slab_mesh(&p.points, p.width / 2.0, p.width);
    for t in &p.tenons {
        let w = p.width / 3.0;
        m.append(&slab_mesh(t, w / 2.0, w));
    }
    // a right-handed turn: x along the member, y up, z across
    let ex = p.dir.extend(0.0);
    let ey = DVec3::Z;
    let ez = ex.cross(ey);
    let turn = |v: [f32; 3]| {
        let v = DVec3::new(v[0] as f64, v[1] as f64, v[2] as f64);
        ex * v.x + ey * v.y + ez * v.z
    };
    for q in m.positions.iter_mut() {
        *q = (p.origin + turn(*q)).as_vec3().to_array();
    }
    for n in m.normals.iter_mut() {
        *n = turn(*n).as_vec3().to_array();
    }
    m
}

/// A bolt through collar ties and a rafter: the rod, and a washer and a
/// nut each side.
pub fn bolt_mesh(b: &creabim_core::framing::Bolt) -> TriMesh {
    let mut m = TriMesh::default();
    let Some(p) = creabim_core::framing::bolt_parts(b.thread) else {
        return m;
    };
    let ring = |r: f64, n: usize, turn: f64| -> Vec<DVec2> {
        (0..n)
            .map(|i| {
                let a = turn + i as f64 / n as f64 * std::f64::consts::TAU;
                DVec2::new(a.cos(), a.sin()) * r
            })
            .collect()
    };
    let rod = ring(b.thread as f64 / 2000.0, 16, 0.0);
    let washer = ring(p.d2 / 2000.0, 24, 0.0);
    // a hexagon across flats s: its corners at s / √3 out
    let nut = ring(p.s / 1000.0 / 3f64.sqrt(), 6, std::f64::consts::FRAC_PI_6);
    let (g, h, n) = (b.grip / 2.0, p.h / 1000.0, p.m / 1000.0);
    let len = b.length() / 2.0;
    let mut add = |pts: &[DVec2], z0: f64, z1: f64| {
        m.append(&slab_mesh(pts, z1, z1 - z0));
    };
    add(&rod, -len, len);
    for s in [-1.0, 1.0] {
        let (a, c) = (s * g, s * (g + h));
        add(&washer, a.min(c), a.max(c));
        let (a, c) = (s * (g + h), s * (g + h + n));
        add(&nut, a.min(c), a.max(c));
    }
    // x up, y across the axis, z along it
    let ez = b.axis;
    let ex = DVec3::Z;
    let ey = ez.cross(ex);
    let turn = |v: [f32; 3]| {
        let v = DVec3::new(v[0] as f64, v[1] as f64, v[2] as f64);
        ex * v.x + ey * v.y + ez * v.z
    };
    for q in m.positions.iter_mut() {
        *q = (b.at + turn(*q)).as_vec3().to_array();
    }
    for nn in m.normals.iter_mut() {
        *nn = turn(*nn).as_vec3().to_array();
    }
    m
}

/// The bolts a collar tie carries (those it is the first of), as one
/// mesh; none for anything else.
pub fn collar_bolts_mesh(model: &Model, e: &Element) -> Option<TriMesh> {
    let f = creabim_core::framing::of(e)?;
    f.bolt?;
    let mut m = TriMesh::default();
    for bolt in creabim_core::framing::bolts(model) {
        if bolt.collars.first() == Some(&e.id) {
            m.append(&bolt_mesh(&bolt));
        }
    }
    (!m.is_empty()).then_some(m)
}

/// The boxes round a timber member's tenons, in the model.
pub fn tenon_boxes(model: &Model, e: &Element) -> Vec<(DVec3, DVec3)> {
    let Some(p) = creabim_core::framing::side_profile(model, e) else {
        return vec![];
    };
    let ex = p.dir.extend(0.0);
    let ez = ex.cross(DVec3::Z);
    p.tenons
        .iter()
        .map(|t| {
            let mut lo = DVec3::splat(f64::INFINITY);
            let mut hi = DVec3::splat(f64::NEG_INFINITY);
            for q in t {
                for s in [-1.0, 1.0] {
                    let v = p.origin + ex * q.x + DVec3::Z * q.y + ez * (s * p.width / 6.0);
                    lo = lo.min(v);
                    hi = hi.max(v);
                }
            }
            (lo, hi)
        })
        .collect()
}

/// The mesh and default colour of a member; `None` for other elements.
pub fn member_mesh(model: &Model, e: &Element) -> Option<(TriMesh, [f32; 3])> {
    Some(match &e.kind {
        ElementKind::Column(c) => {
            let (z0, z1) = c.span(model);
            (slab_mesh(&c.outline(), z1, z1 - z0), [0.78, 0.76, 0.72])
        }
        // a library section swept along the beam, hung so its top is the
        // beam's top (MU37)
        // a timber member with its ends cut as its framing says (George
        // 2026-11-27): its side outline swept across its width
        ElementKind::Beam(b) => match creabim_core::framing::side_profile(model, e) {
            Some(p) => (side_profile_mesh(&p), [0.72, 0.64, 0.52]),
            None => (beam_mesh(model, b), [0.72, 0.64, 0.52]),
        },
        ElementKind::Roof(r) => {
            let holes = creabim_core::slab::holes_of(model, e);
            (
                cut_holes(roof_mesh(r, r.base(model)), &holes),
                [0.66, 0.34, 0.26],
            )
        }
        ElementKind::Stair(s) => (
            stair_mesh(s, s.base(model), s.rise(model)),
            [0.82, 0.80, 0.76],
        ),
        ElementKind::Railing(r) => {
            let (mut m, glass) = railing_meshes(model, r);
            m.append(&glass);
            (m, railing_color(r))
        }
        ElementKind::Footing(f) => {
            let top = f.top(model);
            let mut m = TriMesh::default();
            for piece in creabim_core::footing::outlines(model, e, None) {
                m.append(&slab_mesh(&piece, top, f.thickness));
            }
            (m, [0.66, 0.66, 0.64])
        }
        ElementKind::Part(_) | ElementKind::Site(_) => crate::parts::mesh_of(model, e)?,
        ElementKind::Pipe(p) => (crate::mep::pipe_mesh(model, p), crate::mep::color(p)),
        ElementKind::Flashing(f) => (
            crate::flashing::mesh(model, e.id, f),
            crate::flashing::COLOUR,
        ),
        _ => return None,
    })
}

/// A mesh with each hole (a polygon in plan) cut upright through it.
pub fn cut_holes(mesh: TriMesh, holes: &[Vec<DVec2>]) -> TriMesh {
    let Some((lo, hi)) = mesh.bounds() else {
        return mesh;
    };
    let (z0, z1) = (lo.z as f64 - 1.0, hi.z as f64 + 1.0);
    holes.iter().filter(|h| h.len() >= 3).fold(mesh, |m, h| {
        let tool = slab_mesh(h, z1, z1 - z0);
        crate::scene::boolean_mesh(&m, &tool, creabim_core::BoolOp::Difference)
    })
}

/// A roof as a closed solid: each flat piece of its underside, raised by the
/// roof's upright thickness for the top, with upright edges round the eaves.
pub fn roof_mesh(r: &Roof, base: f64) -> TriMesh {
    roof_layer_mesh(r, base, 0.0, r.vertical_thickness(), None)
}

/// One layer of a layered roof: the part of the roof from `from` to `to`
/// above its underside, measured upright.
pub fn roof_layer_mesh(r: &Roof, base: f64, from: f64, to: f64, layer: Option<usize>) -> TriMesh {
    let mut m = TriMesh::default();
    let tv = to - from;
    // a layer whose edges were moved by hand has eaves of its own
    let (eaves, faces) = match layer {
        Some(i) => (r.layer_eaves(i), r.layer_faces(i)),
        None => (r.eave_outline(), r.faces()),
    };
    let on_eaves = |a: DVec2, b: DVec2| {
        let mid = (a + b) / 2.0;
        let n = eaves.len();
        (0..n).any(|i| {
            let (p, q) = (eaves[i], eaves[(i + 1) % n]);
            let d = q - p;
            let t = ((mid - p).dot(d) / d.length_squared().max(1e-12)).clamp(0.0, 1.0);
            mid.distance(p + d * t) < 1e-6
                && (b - a)
                    .normalize_or_zero()
                    .perp_dot(d.normalize_or_zero())
                    .abs()
                    < 1e-6
        })
    };
    for f in faces {
        let pts = &f.outline;
        let tris = triangulate(pts);
        let lo: Vec<DVec3> = pts
            .iter()
            .map(|p| p.extend(base + from + f.z(*p)))
            .collect();
        let hi: Vec<DVec3> = lo.iter().map(|p| *p + DVec3::Z * tv).collect();
        let up = DVec3::new(-f.g.x, -f.g.y, 1.0);
        m.add_polygon(&hi, &tris, up);
        m.add_polygon(&lo, &tris, -up);
        let ccw = crate::triangulate::signed_area2(pts) > 0.0;
        let n = pts.len();
        for i in 0..n {
            let j = (i + 1) % n;
            if !on_eaves(pts[i], pts[j]) {
                continue;
            }
            let e = (pts[j] - pts[i]).normalize_or_zero();
            let out = if ccw { -e.perp() } else { e.perp() };
            m.add_quad([lo[i], lo[j], hi[j], hi[i]], out.extend(0.0));
        }
    }
    m
}

/// A side profile (along, up) extruded `width` across, centred on `origin`.
fn extrude_profile(m: &mut TriMesh, profile: &[DVec2], origin: DVec3, dir: DVec2, width: f64) {
    let side = dir.perp();
    let at = |p: DVec2, v: f64| origin + (dir * p.x + side * v).extend(p.y);
    let tris = triangulate(profile);
    let left: Vec<DVec3> = profile.iter().map(|p| at(*p, width / 2.0)).collect();
    let right: Vec<DVec3> = profile.iter().map(|p| at(*p, -width / 2.0)).collect();
    m.add_polygon(&left, &tris, side.extend(0.0));
    m.add_polygon(&right, &tris, -side.extend(0.0));
    let ccw = crate::triangulate::signed_area2(profile) > 0.0;
    let n = profile.len();
    for i in 0..n {
        let j = (i + 1) % n;
        let e = profile[j] - profile[i];
        if e.length() < 1e-9 {
            continue;
        }
        let out = if ccw {
            DVec2::new(e.y, -e.x)
        } else {
            DVec2::new(-e.y, e.x)
        };
        let normal = (dir * out.x).extend(out.y);
        m.add_quad([right[i], right[j], left[j], left[i]], normal);
    }
}

/// The side profile of one flight, counter-clockwise in (along, up): the
/// steps on top, a sloping waist underneath. Its top is the last tread; the
/// last riser climbs onto the floor or landing beyond.
pub fn flight_profile(risers: u32, riser: f64, tread: f64, waist: f64) -> Vec<DVec2> {
    let n = risers.max(1);
    if n == 1 {
        return vec![
            DVec2::ZERO,
            DVec2::new(tread, 0.0),
            DVec2::new(tread, riser),
            DVec2::new(0.0, riser),
        ];
    }
    let run = (n - 1) as f64 * tread;
    let slope = riser / tread;
    // the underside runs parallel to the line through the inner corners
    let wv = waist * (1.0 + slope * slope).sqrt();
    let u0 = wv / slope;
    let mut p = vec![DVec2::new(0.0, riser), DVec2::ZERO];
    if u0 < run - 1e-9 {
        p.push(DVec2::new(u0, 0.0));
        p.push(DVec2::new(run, (n - 1) as f64 * riser - wv));
    } else {
        p.push(DVec2::new(run, 0.0));
    }
    // back down the steps: the last riser is the edge of the floor (or
    // landing) it climbs to, so the flight stops at its top tread
    let mut steps = Vec::new();
    for i in 1..n {
        let u = i as f64 * tread;
        steps.push(DVec2::new(u, i as f64 * riser));
        if i + 1 < n {
            steps.push(DVec2::new(u, (i + 1) as f64 * riser));
        }
    }
    p.extend(steps.into_iter().rev());
    p.dedup_by(|a, b| a.distance(*b) < 1e-9);
    p
}

/// A ramp's side profile, counter-clockwise in (along, up): its sloping
/// top from its foot to `rise` over `len`, the underside `waist` under
/// it, cut flat at its base.
pub fn ramp_profile(len: f64, rise: f64, waist: f64) -> Vec<DVec2> {
    let slope = rise / len.max(1e-9);
    let wv = waist * (1.0 + slope * slope).sqrt();
    let u0 = wv / slope.max(1e-9);
    if u0 >= len - 1e-9 || rise <= wv {
        return vec![DVec2::ZERO, DVec2::new(len, 0.0), DVec2::new(len, rise)];
    }
    vec![
        DVec2::ZERO,
        DVec2::new(u0, 0.0),
        DVec2::new(len, rise - wv),
        DVec2::new(len, rise),
    ]
}

/// Solid steps for every flight, and the landing between them.
pub fn stair_mesh(s: &Stair, base: f64, rise: f64) -> TriMesh {
    let mut p = stair_parts(s, base, rise);
    p.carrier.append(&p.treads);
    p.carrier.append(&p.risers);
    p.carrier
}

/// A stair's pieces apart, each of its own material: what carries it (the
/// concrete flight and landing, or the stringers and landing), the treads'
/// finish or boards, the risers' finish or boards.
#[derive(Default)]
pub struct StairParts {
    pub carrier: TriMesh,
    pub treads: TriMesh,
    pub risers: TriMesh,
}

/// A box in a flight's frame: `u` along, `v` across (both from-to), `z`
/// up, from `origin` climbing along `dir`.
fn flight_box(m: &mut TriMesh, origin: DVec3, dir: DVec2, u: [f64; 2], v: [f64; 2], z: [f64; 2]) {
    if u[1] - u[0] < 1e-6 || v[1] - v[0] < 1e-6 || z[1] - z[0] < 1e-6 {
        return;
    }
    let prof = [
        DVec2::new(u[0], z[0]),
        DVec2::new(u[1], z[0]),
        DVec2::new(u[1], z[1]),
        DVec2::new(u[0], z[1]),
    ];
    let mid = (v[0] + v[1]) / 2.0;
    extrude_profile(
        m,
        &prof,
        origin + (dir.perp() * mid).extend(0.0),
        dir,
        v[1] - v[0],
    );
}

/// The band of a stringer between `d0` and `d1` below its sloping top
/// (`c + k·u` in the flight's frame), cut level at the flight's foot and
/// plumb at `end`, `v` across.
fn stringer_band(
    m: &mut TriMesh,
    origin: DVec3,
    dir: DVec2,
    (c, k, end): (f64, f64, f64),
    (d0, d1): (f64, f64),
    v: [f64; 2],
) {
    let cos = 1.0 / (1.0 + k * k).sqrt();
    let (z0, z1) = (c - d0 / cos, c - d1 / cos);
    // where each line meets the foot, and its height at the end
    let foot = |z: f64| -z / k;
    let mut prof = vec![
        DVec2::new(foot(z1), 0.0),
        DVec2::new(end, z1 + k * end),
        DVec2::new(end, z0 + k * end),
        DVec2::new(foot(z0), 0.0),
    ];
    prof.dedup_by(|a, b| a.distance(*b) < 1e-9);
    if prof.len() < 3 || crate::triangulate::signed_area2(&prof).abs() < 1e-9 {
        return;
    }
    if crate::triangulate::signed_area2(&prof) < 0.0 {
        prof.reverse();
    }
    let mid = (v[0] + v[1]) / 2.0;
    extrude_profile(
        m,
        &prof,
        origin + (dir.perp() * mid).extend(0.0),
        dir,
        v[1] - v[0],
    );
}

/// A stringer's section as rectangles across (from-to) and down from its
/// top (from-to): a timber beam is one, an I three, a channel or an angle
/// two or three. `out` is the side the stair's edge is on.
pub(crate) fn stringer_pieces(st: &members::Stringer, out: f64) -> Vec<([f64; 2], [f64; 2])> {
    use members::StringerKind as K;
    let (h, b) = (st.depth, st.width);
    let (tw, tf) = (st.web.max(0.002), st.flange.max(0.002));
    let x = b / 2.0;
    let across = |a: f64, c: f64| {
        let (p, q) = (a * out, c * out);
        [p.min(q), p.max(q)]
    };
    match st.kind {
        K::Timber => vec![([-x, x], [0.0, h])],
        K::I => vec![
            ([-x, x], [0.0, tf]),
            ([-tw / 2.0, tw / 2.0], [tf, h - tf]),
            ([-x, x], [h - tf, h]),
        ],
        K::U => vec![
            (across(x - tw, x), [0.0, h]),
            (across(-x, x - tw), [0.0, tf]),
            (across(-x, x - tw), [h - tf, h]),
        ],
        K::L => vec![
            (across(x - tw, x), [0.0, h]),
            (across(-x, x - tw), [h - tw, h]),
        ],
    }
}

pub fn stair_parts(s: &Stair, base: f64, rise: f64) -> StairParts {
    if s.by_steps() {
        return crate::stairsteps::step_stair_parts(s, base, rise);
    }
    let lay = s.layout(rise);
    let mut p = StairParts::default();
    let (r, t) = (lay.riser, s.tread);
    let (tt, tr) = (s.tread_t(), s.riser_t());
    let concrete = s.carrier.is_concrete() || s.ramp.is_some() || s.stringer.is_none();
    let w = s.width / 2.0;
    for f in &lay.flights {
        let origin = f.start.extend(base + f.z0);
        if let Some(len) = f.length {
            let prof = ramp_profile(len, r, s.waist);
            extrude_profile(&mut p.carrier, &prof, origin, f.dir, s.width);
            continue;
        }
        let n = f.risers.max(1);
        let run = (n - 1) as f64 * t;
        if concrete {
            // the cast flight under its finishes: set back by the riser
            // finish, down by the tread finish, so the finished steps are
            // the steps as laid out
            let prof = flight_profile(n, r, t, s.waist);
            let at = origin + (f.dir * tr).extend(-tt);
            extrude_profile(&mut p.carrier, &prof, at, f.dir, s.width);
        } else if let Some(st) = &s.stringer {
            // stringers under the boards: their top through each tread's
            // front underside
            let line = (r - tt, r / t, run);
            for o in s.stringer_offsets() {
                let out = if o >= 0.0 { 1.0 } else { -1.0 };
                for (v, d) in stringer_pieces(st, out) {
                    stringer_band(
                        &mut p.carrier,
                        origin,
                        f.dir,
                        line,
                        (d[0], d[1]),
                        [o + v[0], o + v[1]],
                    );
                }
            }
        }
        // the treads: finish or boards, each tread k (1..n-1) at k·r
        if tt > 0.0 {
            for k in 1..n {
                let u0 = (k - 1) as f64 * t;
                let z = k as f64 * r;
                flight_box(
                    &mut p.treads,
                    origin,
                    f.dir,
                    [u0, u0 + t],
                    [-w, w],
                    [z - tt, z],
                );
            }
        }
        // the risers: closed, under each tread's front, the last under
        // the floor or landing it climbs to
        if s.risers_closed && tr > 0.0 {
            for k in 1..=n {
                let u0 = (k - 1) as f64 * t;
                let z0 = (k - 1) as f64 * r;
                flight_box(
                    &mut p.risers,
                    origin,
                    f.dir,
                    [u0, u0 + tr],
                    [-w, w],
                    [z0, k as f64 * r - tt],
                );
            }
        }
    }
    if let Some((outline, z)) = &lay.landing {
        let th = s.waist.max(r);
        let top = base + z - tt;
        p.carrier
            .append(&slab_mesh(outline, top, th.min(z - tt).max(0.02)));
        if tt > 0.0 {
            p.treads.append(&slab_mesh(outline, base + z, tt));
        }
    }
    p
}

/// A square bar of side `size` from `a` to `b`.
fn bar(m: &mut TriMesh, a: DVec3, b: DVec3, size: f64) {
    let along = (b - a).normalize_or_zero();
    if along == DVec3::ZERO {
        return;
    }
    let s = DVec3::Z.cross(along).try_normalize().unwrap_or(DVec3::X);
    let u = along.cross(s).normalize();
    let h = size / 2.0;
    let c = |p: DVec3, i: f64, j: f64| p + s * (i * h) + u * (j * h);
    let corners = [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)];
    for k in 0..4 {
        let (i0, j0) = corners[k];
        let (i1, j1) = corners[(k + 1) % 4];
        let normal = s * (i0 + i1) + u * (j0 + j1);
        m.add_quad(
            [c(a, i0, j0), c(a, i1, j1), c(b, i1, j1), c(b, i0, j0)],
            normal,
        );
    }
    let ring = |p: DVec3| corners.map(|(i, j)| c(p, i, j));
    m.add_quad(ring(a), -along);
    m.add_quad(ring(b), along);
}

/// A rod from `a` to `b`: a round tube `d` across with its ends closed,
/// or a square bar of side `d`.
fn rod(m: &mut TriMesh, a: DVec3, b: DVec3, d: f64, round: bool) {
    if round {
        rail_along(m, &[a, b], d, true);
    } else {
        bar(m, a, b, d);
    }
}

/// A rail `d` across along `path`: one round tube bending at the corners
/// with its ends closed, or square bars a segment each.
fn rail_along(m: &mut TriMesh, path: &[DVec3], d: f64, round: bool) {
    if path.len() < 2 {
        return;
    }
    if !round {
        for w in path.windows(2) {
            bar(m, w[0], w[1], d);
        }
        return;
    }
    crate::mep::tube(m, path, d / 2.0);
    let n = path.len();
    disc(m, path[0], path[0] - path[1], d / 2.0, 0.0);
    disc(m, path[n - 1], path[n - 1] - path[n - 2], d / 2.0, 0.0);
}

/// A round disc of radius `r` at `c` facing `out`, `t` thick behind it
/// (0: a flat cap).
fn disc(m: &mut TriMesh, c: DVec3, out: DVec3, r: f64, t: f64) {
    let Some(n) = out.try_normalize() else {
        return;
    };
    if t > 0.0 {
        crate::mep::tube(m, &[c - n * t, c], r);
        disc(m, c - n * t, -n, r, 0.0);
    }
    let u = if n.truncate().length() > 1e-9 {
        DVec3::Z.cross(n).normalize()
    } else {
        DVec3::X
    };
    let v = n.cross(u);
    let k = 16;
    let ring: Vec<DVec3> = (0..k)
        .map(|i| {
            let a = i as f64 / k as f64 * std::f64::consts::TAU;
            c + (u * a.cos() + v * a.sin()) * r
        })
        .collect();
    let fan: Vec<[usize; 3]> = (1..k - 1).map(|i| [0, i, i + 1]).collect();
    m.add_polygon(&ring, &fan, n);
}

/// A box `w` × `w` × `t` lying flat, its underside's centre at `c`.
fn pad(m: &mut TriMesh, c: DVec3, w: f64, t: f64) {
    let h = w / 2.0;
    let pts = [
        DVec2::new(c.x - h, c.y - h),
        DVec2::new(c.x + h, c.y - h),
        DVec2::new(c.x + h, c.y + h),
        DVec2::new(c.x - h, c.y + h),
    ];
    m.append(&slab_mesh(&pts, c.z + t, t));
}

/// A railing's metal parts and its glass, apart so the glass can be seen
/// through: posts on base plates or side brackets, the handrail on top,
/// a bottom rail and the infill between; a wall handrail on its
/// brackets alone.
pub fn railing_meshes(model: &Model, r: &members::Railing) -> (TriMesh, TriMesh) {
    let mut m = TriMesh::default();
    let mut glass = TriMesh::default();
    let line = r.axis(model);
    if line.len() < 2 {
        return (m, glass);
    }
    let (post, rail, round) = (r.post(), r.rail(), r.round);
    let out = r.outward(model, &line);
    // the segment a point along the line lies on, for its outward normal
    let out_at = |p: DVec3| -> DVec2 {
        let mut best = (f64::INFINITY, out[0]);
        for (w, o) in line.windows(2).zip(&out) {
            let (a, b) = (w[0].truncate(), w[1].truncate());
            let ab = b - a;
            let t = ((p.truncate() - a).dot(ab) / ab.length_squared().max(1e-12)).clamp(0.0, 1.0);
            let d = (a + ab * t).distance(p.truncate());
            if d < best.0 {
                best = (d, *o);
            }
        }
        best.1
    };
    // the handrail: a library profile with its insertion point on top of
    // the posts, else a tube or bar its size, its top at the height
    let profile = r
        .rail_profile
        .as_deref()
        .and_then(|n| creabim_core::profiles::find(model, n));
    let under = r.height - rail;
    let at = |z: f64| -> Vec<DVec3> { line.iter().map(|q| *q + DVec3::Z * z).collect() };
    match &profile {
        Some(p) => m.append(&crate::parts::sweep_mesh(p, &at(under), false)),
        None => rail_along(&mut m, &at(under + rail / 2.0), rail, round),
    }
    if let Some(w) = r.wall {
        // brackets: from a rosette on the wall to under the handrail
        let zc = under + rail / 2.0;
        for p in members::posts_along(&line, w.spacing) {
            let o = out_at(p);
            let face = p + (o * (w.clear + rail / 2.0)).extend(0.0);
            let low = DVec3::Z * (zc - rail / 2.0 - members::WALL_BRACKET_DROP);
            let rod_d = members::WALL_BRACKET_ROD;
            bar_or_tube(&mut m, face + low, p + low, rod_d);
            bar_or_tube(&mut m, p + low, p + DVec3::Z * (zc - rail / 2.0), rod_d);
            disc(
                &mut m,
                face + low,
                -o.extend(0.0),
                members::WALL_ROSETTE / 2.0,
                members::ROSETTE_T,
            );
        }
        return (m, glass);
    }
    let posts = members::posts_along(&line, r.post_spacing);
    let drop = r.drop();
    for p in &posts {
        rod(
            &mut m,
            *p - DVec3::Z * drop,
            *p + DVec3::Z * under,
            post,
            round,
        );
        match r.fixing {
            members::Fixing::Top => {
                // a base plate, round under a round post
                if round {
                    disc(
                        &mut m,
                        *p + DVec3::Z * members::BASE_T,
                        DVec3::Z,
                        members::BASE_PLATE / 2.0,
                        members::BASE_T,
                    );
                } else {
                    pad(&mut m, *p, members::BASE_PLATE, members::BASE_T);
                }
            }
            members::Fixing::Side => {
                // two brackets into the edge's face: under the floor and
                // at the post's foot
                let o = out_at(*p).extend(0.0);
                let face = *p - o * (r.gap_to_edge() + post / 2.0);
                for z in [-members::SIDE_UPPER, -drop + members::SIDE_LOWER] {
                    let a = face + DVec3::Z * z;
                    bar_or_tube(&mut m, a, *p + DVec3::Z * z, members::SIDE_ROD);
                    disc(
                        &mut m,
                        a + o * members::ROSETTE_T,
                        o,
                        members::SIDE_ROSETTE / 2.0,
                        members::ROSETTE_T,
                    );
                }
            }
        }
    }
    // the bottom rail: 100 mm up, or on the infill's clear height
    let low = r.infill.map_or(0.1, |f| f.bottom + post / 2.0);
    rail_along(&mut m, &at(low), post, round);
    let Some(f) = r.infill else {
        return (m, glass);
    };
    // between the bottom rail's top and the handrail's underside
    let (z0, z1) = (low + post / 2.0, under);
    match f.kind {
        members::InfillKind::Vertical => {
            for w in posts.windows(2) {
                let len = w[0].truncate().distance(w[1].truncate());
                if len < 1e-6 {
                    continue;
                }
                let (n, g) = members::bars_in(len - post, f.gap, f.bar);
                let dir = (w[1] - w[0]) / len;
                for i in 0..n {
                    let s = post / 2.0 + g * (i + 1) as f64 + f.bar * (i as f64 + 0.5);
                    let p = w[0] + dir * s;
                    rod(&mut m, p + DVec3::Z * z0, p + DVec3::Z * z1, f.bar, f.round);
                }
            }
        }
        members::InfillKind::Horizontal => {
            let (n, g) = members::bars_in(z1 - z0, f.gap, f.bar);
            for i in 0..n {
                let z = z0 + g * (i + 1) as f64 + f.bar * (i as f64 + 0.5);
                rail_along(&mut m, &at(z), f.bar, f.round);
            }
        }
        members::InfillKind::Panel => {
            // a pane between each two posts, clear of them
            for w in posts.windows(2) {
                let len = w[0].truncate().distance(w[1].truncate());
                let edge = post / 2.0 + members::GLASS_GAP;
                if len <= 2.0 * edge {
                    continue;
                }
                let dir = (w[1] - w[0]) / len;
                plate(
                    &mut glass,
                    w[0] + dir * edge,
                    w[1] - dir * edge,
                    z0 + members::GLASS_GAP,
                    z1 - members::GLASS_GAP,
                    members::GLASS_442,
                );
            }
        }
    }
    (m, glass)
}

/// A railing's metal colour: brushed stainless, anodised aluminium, or
/// the painted steel older railings had.
pub fn railing_color(r: &members::Railing) -> [f32; 3] {
    match r.metal {
        Some(members::RailMetal::Stainless) => [0.70, 0.71, 0.72],
        Some(members::RailMetal::Aluminium) => [0.78, 0.79, 0.80],
        None => [0.32, 0.34, 0.38],
    }
}

/// A bracket's rod: a round tube with closed ends.
fn bar_or_tube(m: &mut TriMesh, a: DVec3, b: DVec3, d: f64) {
    rod(m, a, b, d, true);
}

/// A plate `t` thick standing from `a` to `b` between heights `z0` and
/// `z1` over them.
fn plate(m: &mut TriMesh, a: DVec3, b: DVec3, z0: f64, z1: f64, t: f64) {
    let along = (b - a).truncate();
    let Some(d) = along.try_normalize() else {
        return;
    };
    let n = DVec3::new(-d.y, d.x, 0.0) * (t / 2.0);
    let c = |p: DVec3, s: f64, z: f64| p + n * s + DVec3::Z * z;
    let (u, nz) = (DVec3::new(d.x, d.y, 0.0), DVec3::Z);
    m.add_quad(
        [c(a, 1.0, z0), c(b, 1.0, z0), c(b, 1.0, z1), c(a, 1.0, z1)],
        n,
    );
    m.add_quad(
        [
            c(b, -1.0, z0),
            c(a, -1.0, z0),
            c(a, -1.0, z1),
            c(b, -1.0, z1),
        ],
        -n,
    );
    m.add_quad(
        [c(a, -1.0, z1), c(a, 1.0, z1), c(b, 1.0, z1), c(b, -1.0, z1)],
        nz,
    );
    m.add_quad(
        [c(a, 1.0, z0), c(a, -1.0, z0), c(b, -1.0, z0), c(b, 1.0, z0)],
        -nz,
    );
    m.add_quad(
        [c(a, -1.0, z0), c(a, 1.0, z0), c(a, 1.0, z1), c(a, -1.0, z1)],
        -u,
    );
    m.add_quad(
        [c(b, 1.0, z0), c(b, -1.0, z0), c(b, -1.0, z1), c(b, 1.0, z1)],
        u,
    );
}

/// Closed outline lines.
fn ring(items: &mut Vec<PlanItem>, e: &Element, pen: Pen, pts: &[DVec2]) {
    for i in 0..pts.len() {
        items.push(PlanItem::Line {
            id: e.id,
            pen,
            a: pts[i],
            b: pts[(i + 1) % pts.len()],
        });
    }
}

/// How a member on `level` is drawn in its plan, cut `cut` metres above
/// the level.
/// The lowest point of a roof's top faces: its eaves.
pub fn roof_low(model: &Model, r: &Roof) -> f64 {
    let base = r.base(model);
    r.faces()
        .iter()
        .flat_map(|f| f.outline.iter().map(|p| f.z(*p)).collect::<Vec<_>>())
        .fold(f64::INFINITY, f64::min)
        .min(f64::MAX)
        + base
}

pub fn member_plan(model: &Model, e: &Element, cut: f64) -> Vec<PlanItem> {
    let mut items = Vec::new();
    let Some(level) = members::level_of(&e.kind) else {
        return items;
    };
    let z = members::elevation(model, level) + cut;
    match &e.kind {
        ElementKind::Part(_) | ElementKind::Site(_) => {
            return crate::parts::plan_items(model, e, cut);
        }
        ElementKind::Pipe(p) => return crate::mep::pipe_plan(model, e, p, cut),
        ElementKind::Column(c) => {
            let (z0, z1) = c.span(model);
            let pts = c.outline();
            if z0 < z && z < z1 {
                items.push(PlanItem::Polygon {
                    id: e.id,
                    pen: Pen::Cut,
                    points: pts,
                });
            } else if z1 <= z {
                ring(&mut items, e, Pen::Projection, &pts);
            } else {
                // above the cut: a thin line of a dash and two dots
                ring(&mut items, e, Pen::Overhead, &pts);
            }
        }
        ElementKind::Beam(b) => {
            // a sloped beam spans from its lowest to its highest point
            let base = b.top(model);
            let top = base + b.rise[0].max(b.rise[1]);
            let bottom = base + b.rise[0].min(b.rise[1]) - b.depth;
            // a timber member as far as its cut ends reach
            let pts = match creabim_core::framing::side_profile(model, e) {
                Some(p) => {
                    let lo = p.points.iter().map(|q| q.x).fold(f64::INFINITY, f64::min);
                    let hi = p
                        .points
                        .iter()
                        .map(|q| q.x)
                        .fold(f64::NEG_INFINITY, f64::max);
                    let o = p.origin.truncate();
                    let c = p.dir.perp() * (p.width / 2.0);
                    vec![
                        o + p.dir * lo - c,
                        o + p.dir * hi - c,
                        o + p.dir * hi + c,
                        o + p.dir * lo + c,
                    ]
                }
                None => b.outline(),
            };
            if bottom < z && z < top {
                items.push(PlanItem::Polygon {
                    id: e.id,
                    pen: Pen::Cut,
                    points: pts,
                });
            } else {
                // above the cut: a thin line of a dash and two dots
                let pen = if top <= z {
                    Pen::Projection
                } else {
                    Pen::Overhead
                };
                ring(&mut items, e, pen, &pts);
            }
        }
        ElementKind::Roof(r) => {
            // seen from above: eaves, and the ridges and hips between faces;
            // wholly above the cut (a floor plan whose range reaches up to
            // it) a thin line of a dash and two dots (George 2026-11-21)
            let edge = if roof_low(model, r) > z + 1e-6 {
                Pen::Overhead
            } else {
                Pen::Projection
            };
            // every face's edges, each once, with how many faces share it
            let mut seen: Vec<(DVec2, DVec2, usize)> = Vec::new();
            for f in r.faces() {
                let n = f.outline.len();
                for i in 0..n {
                    let (a, b) = (f.outline[i], f.outline[(i + 1) % n]);
                    if a.distance(b) <= 1e-6 {
                        continue;
                    }
                    match seen.iter_mut().find(|(p, q, _)| {
                        (p.distance(a) < 1e-6 && q.distance(b) < 1e-6)
                            || (p.distance(b) < 1e-6 && q.distance(a) < 1e-6)
                    }) {
                        Some(s) => s.2 += 1,
                        None => seen.push((a, b, 1)),
                    }
                }
            }
            // above the cut only its outline, the edges of one face: no
            // ridges, hips or valleys (George 2026-11-21)
            for (a, b, faces) in seen {
                if edge == Pen::Overhead && faces > 1 {
                    continue;
                }
                items.push(PlanItem::Line {
                    id: e.id,
                    pen: edge,
                    a,
                    b,
                });
            }
            // the wall plate, where the roof meets the walls, dashed (not
            // over a floor plan's walls, under it)
            if r.overhang > 1e-6 && edge == Pen::Projection {
                ring(&mut items, e, Pen::Hidden, &r.boundary);
            }
            // holes through it, crossed
            for h in creabim_core::slab::holes_of(model, e) {
                items.extend(crate::plan::hole_marks(e.id, &h, edge));
            }
            // seen from above the roof plan shows which way each face falls
            let top = r
                .faces()
                .iter()
                .flat_map(|f| f.outline.iter().map(|p| f.z(*p)).collect::<Vec<_>>())
                .fold(0.0, f64::max);
            if z > r.base(model) + top + r.vertical_thickness() {
                slope_arrows(&mut items, e, r, 0.1);
            }
        }
        ElementKind::Stair(s) => {
            let first = crate::stairmarks::first_number(model, e.id, s);
            stair_plan(&mut items, e, s, s.rise(model), cut - s.base_offset, first)
        }
        ElementKind::Footing(f) => {
            // below the floor, as usual: dashed; cut when the plan plane
            // passes through it
            let top = f.top(model);
            let z = members::elevation(model, f.plan_level(model)) + cut;
            for piece in creabim_core::footing::outlines(model, e, None) {
                if top - f.thickness < z && z < top {
                    items.push(PlanItem::Polygon {
                        id: e.id,
                        pen: Pen::Cut,
                        points: piece,
                    });
                } else {
                    let pen = if top <= z {
                        Pen::Hidden
                    } else {
                        Pen::Projection
                    };
                    ring(&mut items, e, pen, &piece);
                }
            }
        }
        ElementKind::Railing(r) => {
            // the handrail's line, and each post square or round
            let path = r.axis(model);
            for w in path.windows(2) {
                items.push(PlanItem::Line {
                    id: e.id,
                    pen: Pen::Projection,
                    a: w[0].truncate(),
                    b: w[1].truncate(),
                });
            }
            let h = r.post() / 2.0;
            for p in r.posts(model) {
                let c = p.truncate();
                let pts: Vec<DVec2> = if r.round {
                    (0..16)
                        .map(|i| {
                            let a = i as f64 / 16.0 * std::f64::consts::TAU;
                            c + DVec2::new(a.cos(), a.sin()) * h
                        })
                        .collect()
                } else {
                    vec![
                        c + DVec2::new(-h, -h),
                        c + DVec2::new(h, -h),
                        c + DVec2::new(h, h),
                        c + DVec2::new(-h, h),
                    ]
                };
                ring(&mut items, e, Pen::Projection, &pts);
            }
        }
        _ => {}
    }
    items
}

/// The pitch of a slope as roof plans write it: degrees, or for a slope
/// too shallow to read in degrees a fall of one in so many.
pub fn pitch_text(degrees: f64) -> String {
    if degrees >= 5.0 {
        format!("{}°", (degrees * 10.0).round() / 10.0)
    } else {
        format!("1:{}", (1.0 / degrees.to_radians().tan()).round())
    }
}

/// An arrow down each sloping face of a roof, from near its top toward
/// its eave, with the pitch written beside it 2.5 mm tall, in a drawing of
/// `mm` metres per paper millimetre. The arrow is at most 8 mm long on
/// paper (George 2026-11-22: shorter, the text always 2.5 mm).
fn slope_arrows(items: &mut Vec<PlanItem>, e: &Element, r: &Roof, mm: f64) {
    for f in r.faces() {
        let n = f.outline.len();
        if n < 3 || f.g.length() < 1e-6 {
            continue;
        }
        // the face's centre, by area
        let (mut c, mut area) = (DVec2::ZERO, 0.0);
        for i in 0..n {
            let (a, b) = (f.outline[i], f.outline[(i + 1) % n]);
            let k = a.perp_dot(b);
            c += (a + b) * k;
            area += k;
        }
        if area.abs() < 1e-9 {
            continue;
        }
        let c = c / (3.0 * area);
        let down = -f.g.normalize();
        // as long as the face is deep down its slope, up to 8 mm on paper
        let depth = {
            let ts = f.outline.iter().map(|p| (*p - c).dot(down));
            let (lo, hi) = ts.fold((f64::MAX, f64::MIN), |(lo, hi), t| (lo.min(t), hi.max(t)));
            hi - lo
        };
        let len = (depth * 0.4).min(8.0 * mm);
        if len < 3.0 * mm {
            continue;
        }
        let (tail, head) = (c - down * len / 2.0, c + down * len / 2.0);
        let line = |items: &mut Vec<PlanItem>, a, b| {
            items.push(PlanItem::Line {
                id: e.id,
                pen: Pen::Symbol,
                a,
                b,
            })
        };
        line(items, tail, head);
        let (back, wing) = (-down * 1.8 * mm, down.perp() * 0.6 * mm);
        line(items, head, head + back + wing);
        line(items, head, head + back - wing);
        // the pitch beside the shaft, on its right going down the slope
        let slope = f.g.length().atan().to_degrees();
        items.push(PlanItem::Text {
            id: e.id,
            at: c - down.perp() * 3.5 * mm,
            text: pitch_text(slope),
            size: SLOPE_TEXT,
            align: crate::paper::Anchor::Middle,
            mm,
            rgb: None,
            bold: false,
        });
    }
}

/// The paper height of a roof's pitch, in mm.
const SLOPE_TEXT: f64 = 2.5;

/// Draws the roofs' slope arrows of a drawing again at its own scale, `mm`
/// metres per paper millimetre (the plans draw them at 1:100).
pub fn size_slope_arrows(model: &Model, items: &mut Vec<PlanItem>, mm: f64) {
    let roofs: Vec<ElementId> = items
        .iter()
        .filter_map(|it| match it {
            PlanItem::Text { id, size, .. } if *size == SLOPE_TEXT => Some(*id),
            _ => None,
        })
        .filter(|id| matches!(model.get(*id).map(|e| &e.kind), Some(ElementKind::Roof(_))))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    if roofs.is_empty() {
        return;
    }
    items.retain(|it| match it {
        PlanItem::Text { id, .. }
        | PlanItem::Line {
            id,
            pen: Pen::Symbol,
            ..
        } => !roofs.contains(id),
        _ => true,
    });
    for id in roofs {
        if let Some(e) = model.get(id) {
            if let ElementKind::Roof(r) = &e.kind {
                slope_arrows(items, e, r, mm);
            }
        }
    }
}

/// A stair in plan as STN 01 3420, 5.9 draws it: its sides and risers up
/// to the cut, a thin line at 30° where the plan cuts it, the rest
/// dash-dot, the walking line from a circle to an arrow with `h × b` over
/// it, and its steps numbered from `first` (see [`crate::stairmarks`]).
fn stair_plan(items: &mut Vec<PlanItem>, e: &Element, s: &Stair, rise: f64, cut: f64, first: u32) {
    if s.by_steps() {
        return crate::stairsteps::step_stair_plan(items, e, s, rise, cut, first);
    }
    let above = Pen::StairAbove { hidden: false };
    let lay = s.layout(rise);
    let r = lay.riser;
    let line = |items: &mut Vec<PlanItem>, pen, a, b| {
        items.push(PlanItem::Line {
            id: e.id,
            pen,
            a,
            b,
        });
    };
    if let (Some(slope), Some(f)) = (s.ramp, lay.flights.first()) {
        // a ramp: its ends and sides, broken where the cut plane meets
        // its top, the walking line up and its slope written on it
        let side = f.dir.perp() * s.width / 2.0;
        let at = |u: f64| f.start + f.dir * u;
        let run = f.run(s.tread);
        let split = if slope > 1e-9 {
            (cut / slope).clamp(0.0, run)
        } else {
            run
        };
        let past = if split < run - 1e-9 {
            above
        } else {
            Pen::Projection
        };
        line(items, Pen::Projection, at(0.0) - side, at(0.0) + side);
        line(items, past, at(run) - side, at(run) + side);
        for sd in [-side, side] {
            if split > 1e-9 {
                line(items, Pen::Projection, at(0.0) + sd, at(split) + sd);
            }
            if run - split > 1e-9 {
                line(items, above, at(split) + sd, at(run) + sd);
            }
        }
        if split < run - 1e-9 {
            let (a, b) = crate::stairmarks::cut_line(at(split), f.dir, s.width);
            line(items, Pen::Symbol, a, b);
        }
        line(items, Pen::Symbol, at(0.0), at(run));
        let (end, back, wing) = (at(run), -f.dir * 0.25, f.dir.perp() * 0.12);
        line(items, Pen::Symbol, end, end + back + wing);
        line(items, Pen::Symbol, end, end + back - wing);
        items.push(PlanItem::Label {
            id: e.id,
            at: at(run / 2.0) + f.dir.perp() * 0.15,
            text: format!("{} %", (slope * 1000.0).round() / 10.0).replace('.', ","),
        });
        return;
    }
    let mut broken = false;
    for f in &lay.flights {
        let side = f.dir.perp() * s.width / 2.0;
        let at = |u: f64| f.start + f.dir * u;
        // the first riser the cut plane meets
        let cut_at: Option<u32> = (0..f.risers).find(|i| f.z0 + (*i + 1) as f64 * r > cut + 1e-9);
        let pen_for = |i: u32| match cut_at {
            Some(k) if i > k || broken => above,
            _ => Pen::Projection,
        };
        for i in 0..f.risers {
            let u = i as f64 * s.tread;
            line(items, pen_for(i), at(u) - side, at(u) + side);
        }
        // the sides, split at the break
        let run = f.run(s.tread);
        let split = match (cut_at, broken) {
            (_, true) => 0.0,
            (Some(k), false) => (k as f64 + 0.5) * s.tread,
            (None, false) => run,
        };
        let split = split.min(run);
        for sd in [-side, side] {
            if split > 1e-9 {
                line(items, Pen::Projection, at(0.0) + sd, at(split) + sd);
            }
            if run - split > 1e-9 {
                line(items, above, at(split) + sd, at(run) + sd);
            }
        }
        if let (Some(_), false) = (cut_at, broken) {
            let (a, b) = crate::stairmarks::cut_line(at(split), f.dir, s.width);
            line(items, Pen::Symbol, a, b);
            broken = true;
        }
    }
    if let Some((outline, z)) = &lay.landing {
        let pen = if broken || *z > cut {
            above
        } else {
            Pen::Projection
        };
        ring(items, e, pen, outline);
    }
    // walking line with an arrow at its top end
    let mut walk: Vec<DVec2> = Vec::new();
    for (k, f) in lay.flights.iter().enumerate() {
        if k == 1 {
            let f1 = lay.flights[0];
            walk.push(f1.end(s.tread) + f1.dir * s.width / 2.0);
            if f.dir.dot(f1.dir) < -0.5 {
                // a U: across the landing to the second flight's axis
                walk.push(f.start + f1.dir * s.width / 2.0);
            }
        }
        walk.push(f.start);
        walk.push(f.end(s.tread));
    }
    walk.dedup_by(|a, b| a.distance(*b) < 1e-9);
    for w in walk.windows(2) {
        line(items, Pen::Symbol, w[0], w[1]);
    }
    if let (Some(&end), Some(f)) = (walk.last(), lay.flights.last()) {
        let back = -f.dir * 0.25;
        let wing = f.dir.perp() * 0.12;
        line(items, Pen::Symbol, end, end + back + wing);
        line(items, Pen::Symbol, end, end + back - wing);
    }
    let flights: Vec<(Vec<[DVec2; 2]>, DVec2)> = lay
        .flights
        .iter()
        .map(|f| {
            let side = f.dir.perp() * s.width / 2.0;
            let nosings = (0..f.risers)
                .map(|k| {
                    let c = f.start + f.dir * (k as f64 * s.tread);
                    [c - side, c + side]
                })
                .collect();
            (nosings, f.dir)
        })
        .collect();
    crate::stairmarks::marks(items, e, &walk, &flights, r, s.tread, first);
}

/// A whole stair seen from above, as the floor over it sees it through
/// its well: `line(pen, a, b, z)` takes each tread edge, side and landing
/// edge in projection and the walking line with its arrow, `z` being how
/// high above the stair's base the line is.
pub fn stair_lines(s: &Stair, rise: f64, line: &mut dyn FnMut(Pen, DVec2, DVec2, f64)) {
    if s.by_steps() {
        return crate::stairsteps::step_stair_lines(s, rise, line);
    }
    let lay = s.layout(rise);
    let r = lay.riser;
    if let (Some(_), Some(f)) = (s.ramp, lay.flights.first()) {
        let side = f.dir.perp() * s.width / 2.0;
        let (a, b) = (f.start, f.end(s.tread));
        line(Pen::Projection, a - side, a + side, 0.0);
        line(Pen::Projection, b - side, b + side, rise);
        for sd in [-side, side] {
            line(Pen::Projection, a + sd, b + sd, rise);
        }
        line(Pen::Symbol, a, b, rise);
        return;
    }
    for f in &lay.flights {
        let side = f.dir.perp() * s.width / 2.0;
        let at = |u: f64| f.start + f.dir * u;
        for i in 0..f.risers {
            let u = i as f64 * s.tread;
            line(
                Pen::Projection,
                at(u) - side,
                at(u) + side,
                f.z0 + (i + 1) as f64 * r,
            );
        }
        let run = f.run(s.tread);
        let top = f.z0 + f.risers as f64 * r;
        for sd in [-side, side] {
            line(Pen::Projection, at(0.0) + sd, at(run) + sd, top);
        }
    }
    if let Some((outline, z)) = &lay.landing {
        let n = outline.len();
        for i in 0..n {
            line(Pen::Projection, outline[i], outline[(i + 1) % n], *z);
        }
    }
    let mut walk: Vec<(DVec2, f64)> = Vec::new();
    for (k, f) in lay.flights.iter().enumerate() {
        if k == 1 {
            let f1 = lay.flights[0];
            walk.push((f1.end(s.tread) + f1.dir * s.width / 2.0, f.z0));
            if f.dir.dot(f1.dir) < -0.5 {
                walk.push((f.start + f1.dir * s.width / 2.0, f.z0));
            }
        }
        walk.push((f.start, f.z0));
        walk.push((f.end(s.tread), f.z0 + f.risers as f64 * r));
    }
    walk.dedup_by(|a, b| a.0.distance(b.0) < 1e-9);
    for w in walk.windows(2) {
        line(Pen::Symbol, w[0].0, w[1].0, w[1].1);
    }
    // the circle on its first nosing (STN 01 3420, 5.9.4)
    if let Some(&(p0, z)) = walk.first() {
        for (a, b) in crate::stairmarks::start_circle(p0) {
            line(Pen::Symbol, a, b, z);
        }
    }
    if let (Some(&(end, z)), Some(f)) = (walk.last(), lay.flights.last()) {
        let back = -f.dir * 0.25;
        let wing = f.dir.perp() * 0.12;
        line(Pen::Symbol, end, end + back + wing, z);
        line(Pen::Symbol, end, end + back - wing, z);
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn a_stair_on_stringers_has_boards_and_its_stringers_and_concrete_sits_under_its_finish() {
        use creabim_core::members::{StairCarrier, StepFinish, Stringer, StringerKind};
        use creabim_core::{ElementId, Stair};
        let mut s = Stair::new(ElementId::new(), DVec2::ZERO, 0.0);
        s.height = Some(1.8);
        let bounds = |m: &TriMesh| {
            m.positions
                .iter()
                .fold((f64::MAX, f64::MIN), |(lo, hi), p| {
                    (lo.min(p[2] as f64), hi.max(p[2] as f64))
                })
        };
        // bare concrete: no finish pieces, the top tread at its height
        let bare = stair_parts(&s, 0.0, 1.8);
        assert!(bare.treads.indices.is_empty() && bare.risers.indices.is_empty());
        // finished: the concrete drops by the finish, the finish tops out
        // where the bare concrete did
        s.tread_finish = Some(StepFinish {
            material: None,
            thickness: 0.027,
        });
        s.riser_finish = Some(StepFinish {
            material: None,
            thickness: 0.018,
        });
        let fin = stair_parts(&s, 0.0, 1.8);
        assert!((bounds(&fin.treads).1 - bounds(&bare.carrier).1).abs() < 1e-4);
        assert!((bounds(&fin.carrier).1 - (bounds(&bare.carrier).1 - 0.027)).abs() < 1e-4);
        assert!(!fin.risers.indices.is_empty());
        // two stringers, open risers: stringers carry, no riser boards
        s.carrier = StairCarrier::TwoStringers;
        s.stringer = Some(Stringer {
            name: "KVH 60 × 240".into(),
            kind: StringerKind::Timber,
            depth: 0.24,
            width: 0.06,
            web: 0.0,
            flange: 0.0,
            edge_offset: 0.02,
        });
        s.risers_closed = false;
        assert_eq!(
            s.stringer_offsets(),
            vec![0.5 - 0.02 - 0.03, -(0.5 - 0.02 - 0.03)]
        );
        let st = stair_parts(&s, 0.0, 1.8);
        assert!(!st.carrier.indices.is_empty() && !st.treads.indices.is_empty());
        assert!(st.risers.indices.is_empty());
        // the stringers stand on the floor, under the treads
        assert!(bounds(&st.carrier).0.abs() < 1e-4);
    }

    #[test]
    fn mitred_beams_meet_on_the_bisector_and_butt_ones_overlap_by_half() {
        use creabim_core::members::{join_beams, BeamCorner};
        use creabim_core::{Beam, Element, Level};
        let lv = Element::new(
            "L0",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let m = Model::from_elements([lv.clone()]).unwrap();
        let mut a = Beam::new(lv.id, DVec2::ZERO, DVec2::new(4.0, 0.0));
        let mut b = Beam::new(lv.id, DVec2::new(4.0, 0.0), DVec2::new(4.0, 3.0));
        join_beams(&mut a, &mut b, BeamCorner::Mitred);
        // every vertex of a's far end lies on the 45 degree plane through
        // the corner, so the two close up with no gap and no overlap
        let n = DVec3::new(1.0, 1.0, 0.0).normalize();
        let corner = DVec3::new(4.0, 0.0, 0.0);
        let mesh = beam_mesh(&m, &a);
        let far: Vec<DVec3> = mesh
            .positions
            .iter()
            .map(|p| DVec3::new(p[0] as f64, p[1] as f64, p[2] as f64))
            .filter(|p| p.x > 3.5)
            .collect();
        assert!(!far.is_empty());
        for p in far {
            assert!(
                (p - corner).truncate().dot(n.truncate()).abs() < 1e-6,
                "{p}"
            );
        }
        // far from the origin, where f32 rounds the mesh, every end vertex
        // is still cut (MU40: ragged mitres at about 17 m out)
        let off = DVec2::new(16.919, -6.502);
        let mut fa = Beam::new(lv.id, off, off + DVec2::new(4.0, 0.0));
        let mut fb = Beam::new(
            lv.id,
            off + DVec2::new(4.0, 0.0),
            off + DVec2::new(1.0, 3.0),
        );
        join_beams(&mut fa, &mut fb, BeamCorner::Mitred);
        let corner = (off + DVec2::new(4.0, 0.0)).extend(0.0);
        let da = DVec2::X;
        let db = (DVec2::new(-3.0, 3.0)).normalize();
        let nn = (da + db).normalize();
        for p in beam_mesh(&m, &fa).positions {
            let q = DVec3::new(p[0] as f64, p[1] as f64, p[2] as f64);
            if q.x - off.x > 3.0 {
                assert!((q - corner).truncate().dot(nn).abs() < 1e-3, "{q}");
            }
        }
        // and the second's start, cut on the same plane
        for p in beam_mesh(&m, &fb).positions {
            let q = DVec3::new(p[0] as f64, p[1] as f64, p[2] as f64);
            if (q - corner).truncate().length() < 0.5 {
                assert!((q - corner).truncate().dot(nn).abs() < 1e-3, "start {q}");
            }
        }
        // a library section hung off its insertion point (a channel)
        let chan = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(0.1, 0.0),
            DVec2::new(0.1, 0.2),
            DVec2::new(0.09, 0.2),
            DVec2::new(0.09, 0.01),
            DVec2::new(0.0, 0.01),
        ];
        let sec = creabim_core::members::Section {
            name: "C".into(),
            points: chan,
        };
        let (mut ca, mut cb) = (fa.clone(), fb.clone());
        ca.section = Some(sec.clone());
        cb.section = Some(sec);
        join_beams(&mut ca, &mut cb, BeamCorner::Mitred);
        for (k, bm) in [(1, &ca), (0, &cb)] {
            for p in beam_mesh(&m, bm).positions {
                let q = DVec3::new(p[0] as f64, p[1] as f64, p[2] as f64);
                if (q - corner).truncate().length() < 0.5 {
                    assert!(
                        (q - corner).truncate().dot(nn).abs() < 1e-3,
                        "channel end {k} {q}"
                    );
                }
            }
        }
        // every preset, mitred both ways round a sharp and a gentle turn
        for name in creabim_core::profiles::PRESETS {
            let pts = creabim_core::profiles::preset(name).unwrap();
            for to in [
                DVec2::new(1.0, 3.0),
                DVec2::new(7.0, 1.0),
                DVec2::new(6.0, -2.5),
            ] {
                let sec = creabim_core::members::Section {
                    name: name.to_string(),
                    points: pts.clone(),
                };
                let mut pa = Beam::new(lv.id, off, off + DVec2::new(4.0, 0.0));
                let mut pb = Beam::new(lv.id, off + DVec2::new(4.0, 0.0), off + to);
                pa.section = Some(sec.clone());
                pb.section = Some(sec);
                join_beams(&mut pa, &mut pb, BeamCorner::Mitred);
                let db = (to - DVec2::new(4.0, 0.0)).normalize();
                let nn = (DVec2::X + db).normalize();
                for (k, bm) in [(1, &pa), (0, &pb)] {
                    let mesh = beam_mesh(&m, bm);
                    let (ax, bx) = bm.axis(&m);
                    let dir = (bx - ax).normalize();
                    let len = ax.distance(bx);
                    for p in &mesh.positions {
                        let q = DVec3::new(p[0] as f64, p[1] as f64, p[2] as f64);
                        let s = (q - ax).dot(dir);
                        // nothing of either beam crosses the mitre plane
                        let side = (q - corner).truncate().dot(nn);
                        if k == 1 {
                            assert!(side < 1e-3, "{name} a past the cut {q} side {side}");
                        } else {
                            assert!(side > -1e-3, "{name} b before the cut {q} side {side}");
                        }
                        let _ = (s, len);
                    }
                }
            }
        }
        // sloped beams (a roof's rafters), an I-section: every vertex
        // of each end lies on the shared cut
        let ipts = creabim_core::profiles::preset("I-section").unwrap();
        let sec = creabim_core::members::Section {
            name: "I".into(),
            points: ipts,
        };
        let mut sa = Beam::new(lv.id, DVec2::ZERO, DVec2::new(4.0, 0.0));
        let mut sb = Beam::new(lv.id, DVec2::new(4.0, 0.0), DVec2::new(4.0, 3.0));
        sa.rise = [0.0, 1.5];
        sb.rise = [1.5, 1.5];
        sa.section = Some(sec.clone());
        sb.section = Some(sec);
        join_beams(&mut sa, &mut sb, BeamCorner::Mitred);
        let n = sa.cuts[1].unwrap().normalize();
        let c = sa.axis(&m).1;
        for bm in [&sa, &sb] {
            let (ax, bx) = bm.axis(&m);
            for p in beam_mesh(&m, bm).positions {
                let q = DVec3::new(p[0] as f64, p[1] as f64, p[2] as f64);
                let near = q.distance(c) < ax.distance(bx) / 2.0;
                if near {
                    assert!((q - c).dot(n).abs() < 1e-3, "sloped end {q}");
                }
            }
        }
        // butt: the first runs through to the second's far face
        let (mut a, mut b) = (a.clone(), b.clone());
        join_beams(&mut a, &mut b, BeamCorner::Butt);
        assert!((a.axis(&m).1.x - 4.1).abs() < 1e-9);
        assert!((b.axis(&m).0.y - 0.1).abs() < 1e-9);
        // a sloped beam: its top rises from start to end
        let mut r = Beam::new(lv.id, DVec2::ZERO, DVec2::new(4.0, 0.0));
        r.rise = [0.0, 1.0];
        let (s, e) = r.axis(&m);
        assert!((e.z - s.z - 1.0).abs() < 1e-9);
    }

    use super::*;
    use creabim_core::{
        Beam, Column, ElementId, Landing, Level, Profile, Railing, RoofShape, Transaction,
    };

    fn model() -> (Model, ElementId) {
        let mut m = Model::new();
        let l = Element::new(
            "Ground",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let id = l.id;
        Transaction::new("l").insert(l).apply(&mut m).unwrap();
        (m, id)
    }

    fn rect(w: f64, d: f64) -> Vec<DVec2> {
        vec![
            DVec2::ZERO,
            DVec2::new(w, 0.0),
            DVec2::new(w, d),
            DVec2::new(0.0, d),
        ]
    }

    /// George 2026-11-21: a storey's plan with its top raised over the
    /// eaves shows the roof above, only its outline, a dash and two dots.
    #[test]
    fn a_plan_reaching_up_to_the_roof_shows_its_outline_only() {
        let (mut m, l) = model();
        let up = Element::new(
            "Roof level",
            ElementKind::Level(Level {
                elevation: 3.0,
                height: 3.0,
            }),
        );
        let ul = up.id;
        // no overhang: its eaves on the level, not under it
        let mut r = Roof::new(ul, rect(10.0, 6.0), RoofShape::Hip);
        r.overhang = 0.0;
        let roof = Element::new("R", ElementKind::Roof(r));
        let rid = roof.id;
        let mut v = creabim_core::View::plan(l);
        v.kind = creabim_core::ViewKind::Plan {
            level: l,
            cut_height: 1.2,
        };
        let ve = Element::new("Plan", ElementKind::View(v));
        let vid = ve.id;
        Transaction::new("r")
            .insert(up)
            .insert(roof)
            .insert(ve)
            .apply(&mut m)
            .unwrap();
        let roof_lines = |m: &Model| -> Vec<Pen> {
            crate::section::view_drawing(m, vid)
                .unwrap()
                .items
                .iter()
                .filter_map(|it| match it {
                    PlanItem::Line { id, pen, .. } if *id == rid => Some(*pen),
                    _ => None,
                })
                .collect()
        };
        // up to the next level only: the roof on it is left out
        assert!(roof_lines(&m).is_empty());
        let before = m.get(vid).unwrap().clone();
        let mut after = before.clone();
        if let ElementKind::View(v) = &mut after.kind {
            v.graphics.range.top = Some(creabim_core::graphics::RangeLimit {
                level: l,
                offset: 6.0,
            });
        }
        Transaction::new("top")
            .replace(before, after)
            .apply(&mut m)
            .unwrap();
        // the four eaves, no ridge or hips
        let pens = roof_lines(&m);
        assert_eq!(pens.len(), 4, "{pens:?}");
        assert!(pens.iter().all(|p| *p == Pen::Overhead));
        // and so with the range drawing the storeys above, dashed
        let before = m.get(vid).unwrap().clone();
        let mut after = before.clone();
        if let ElementKind::View(v) = &mut after.kind {
            v.graphics.range.top = None;
            v.graphics.range.overhead = true;
            v.graphics.range.top = Some(creabim_core::graphics::RangeLimit {
                level: ul,
                offset: 3.0,
            });
        }
        Transaction::new("over")
            .replace(before, after)
            .apply(&mut m)
            .unwrap();
        let pens = roof_lines(&m);
        assert_eq!(pens.len(), 4, "{pens:?}");
        assert!(pens.iter().all(|p| *p == Pen::Overhead));
    }

    #[test]
    fn column_and_beam_meshes_have_the_right_volume() {
        let (m, l) = model();
        let c = Element::new(
            "C",
            ElementKind::Column(Column::new(
                l,
                DVec2::new(2.0, 2.0),
                Profile::Rectangle {
                    width: 0.3,
                    depth: 0.4,
                },
            )),
        );
        let (mesh, _) = member_mesh(&m, &c).unwrap();
        assert!((mesh.volume() - 0.3 * 0.4 * 3.0).abs() < 1e-5);
        let round = Element::new(
            "C",
            ElementKind::Column(Column::new(
                l,
                DVec2::ZERO,
                Profile::Round { diameter: 0.4 },
            )),
        );
        let (mesh, _) = member_mesh(&m, &round).unwrap();
        let exact = std::f64::consts::PI * 0.04 * 3.0;
        assert!((mesh.volume() - exact).abs() / exact < 0.02);
        let b = Element::new(
            "B",
            ElementKind::Beam(Beam::new(l, DVec2::ZERO, DVec2::new(5.0, 0.0))),
        );
        let (mesh, _) = member_mesh(&m, &b).unwrap();
        assert!((mesh.volume() - 5.0 * 0.2 * 0.4).abs() < 1e-5);
        let (lo, hi) = mesh.bounds().unwrap();
        assert!((hi.z - 3.0).abs() < 1e-5 && (lo.z - 2.6).abs() < 1e-5);
    }

    #[test]
    fn roof_volume_is_plan_area_times_upright_thickness() {
        let (_, l) = model();
        for shape in RoofShape::ALL {
            let mut r = Roof::new(l, rect(10.0, 6.0), shape);
            r.pitch = 30.0;
            let m = roof_mesh(&r, 3.0);
            let want = r.plan_area() * r.vertical_thickness();
            assert!(
                (m.volume() - want).abs() / want < 1e-4,
                "{shape:?}: {} vs {want}",
                m.volume()
            );
        }
    }

    #[test]
    fn a_straight_stair_climbs_its_full_rise() {
        let (m, l) = model();
        let s = Stair::new(l, DVec2::ZERO, 0.0);
        let mesh = stair_mesh(&s, 0.0, s.rise(&m));
        let (lo, hi) = mesh.bounds().unwrap();
        assert!(lo.z.abs() < 1e-5);
        // up to the last tread, one riser below the floor above
        assert!((hi.z as f64 - 16.0 * 3.0 / 17.0).abs() < 1e-5);
        // 17 risers of 176 mm, 16 treads
        assert!((hi.x - 16.0 * 0.28).abs() < 1e-4);
        assert!(mesh.volume() > 0.0);
        let prof = flight_profile(17, 3.0 / 17.0, 0.28, 0.15);
        assert!(crate::triangulate::signed_area2(&prof) > 0.0);
    }

    #[test]
    fn an_l_stair_and_its_railings_mesh() {
        let (mut m, l) = model();
        let mut s = Stair::new(l, DVec2::ZERO, 0.0);
        s.landing = Some(Landing {
            after: 8,
            left: true,
        });
        let e = Element::new("S", ElementKind::Stair(s.clone()));
        let sid = e.id;
        Transaction::new("s").insert(e).apply(&mut m).unwrap();
        let mesh = stair_mesh(&s, 0.0, 3.0);
        assert!(mesh.volume() > 0.0);
        let r = Railing::along_stair(&m, sid, false).unwrap();
        let re = Element::new("R", ElementKind::Railing(r));
        let (rm, _) = member_mesh(&m, &re).unwrap();
        let (_, hi) = rm.bounds().unwrap();
        assert!(
            (hi.z - 4.0).abs() < 0.01,
            "a metre over the top nosing: {hi:?}"
        );
    }

    #[test]
    fn columns_and_beams_have_a_schedule() {
        let (mut m, l) = model();
        Transaction::new("add")
            .insert(Element::new(
                "C1",
                ElementKind::Column(Column::new(
                    l,
                    DVec2::ZERO,
                    Profile::Round { diameter: 0.3 },
                )),
            ))
            .insert(Element::new(
                "B1",
                ElementKind::Beam(Beam::new(l, DVec2::ZERO, DVec2::new(4.0, 0.0))),
            ))
            .apply(&mut m)
            .unwrap();
        let s = crate::schedule::schedule(&m, creabim_core::ScheduleCategory::Structure);
        assert_eq!(s.rows.len(), 2);
        assert_eq!(s.rows[0].cells[1], "Beam");
        assert_eq!(s.rows[1].cells[4], "Ø 300");
        assert_eq!(s.totals[5], "7000");
    }

    #[test]
    fn plans_show_beams_dashed_and_stairs_broken() {
        let (m, l) = model();
        let b = Element::new(
            "B",
            ElementKind::Beam(Beam::new(l, DVec2::ZERO, DVec2::new(5.0, 0.0))),
        );
        let items = member_plan(&m, &b, 1.2);
        assert!(items.iter().all(|i| matches!(
            i,
            PlanItem::Line {
                pen: Pen::Overhead,
                ..
            }
        )));
        let s = Element::new("S", ElementKind::Stair(Stair::new(l, DVec2::ZERO, 0.0)));
        let items = member_plan(&m, &s, 1.2);
        let count = |pen: Pen| {
            items
                .iter()
                .filter(|i| matches!(i, PlanItem::Line { pen: p, .. } if *p == pen))
                .count()
        };
        // past the cut dash-dot, with h × b and the first step's number
        // (STN 01 3420, 5.9)
        assert!(count(Pen::StairAbove { hidden: false }) > 0 && count(Pen::Projection) > 0);
        let texts: Vec<&str> = items
            .iter()
            .filter_map(|i| match i {
                PlanItem::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|t| t.contains(" × ")), "{texts:?}");
        assert!(texts.contains(&"1"), "{texts:?}");
        let c = Element::new(
            "C",
            ElementKind::Column(Column::new(
                l,
                DVec2::ZERO,
                Profile::Round { diameter: 0.3 },
            )),
        );
        assert!(matches!(
            member_plan(&m, &c, 1.2)[0],
            PlanItem::Polygon { pen: Pen::Cut, .. }
        ));
    }

    #[test]
    fn roof_plans_show_slope_arrows_with_their_pitch() {
        let m = creabim_core::starter::starter_house();
        let view = |name: &str| {
            let id = m.iter().find(|e| e.name == name).unwrap().id;
            crate::section::view_drawing(&m, id).unwrap().items
        };
        let pitches = |items: &[PlanItem]| -> Vec<String> {
            items
                .iter()
                .filter_map(|it| match it {
                    PlanItem::Text { text, .. } if text.ends_with('°') => Some(text.clone()),
                    _ => None,
                })
                .collect()
        };
        // the gable's two faces, each falling to its eave; the flat roofs none
        assert_eq!(pitches(&view("Roof plan")), vec!["35°", "35°"]);
        // the first floor is under the roof: no arrows there
        assert!(pitches(&view("First Floor plan")).is_empty());
        assert_eq!(pitch_text(35.0), "35°");
        assert_eq!(pitch_text((1.0f64 / 60.0).atan().to_degrees()), "1:60");
    }
}
