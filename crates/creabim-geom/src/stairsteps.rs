//! Plan linework and meshes for stairs laid out tread by tread (winders,
//! spirals): see `creabim_core::stairshape`.

use crate::members::StairParts;
use crate::plan::{Pen, PlanItem};
use crate::scene::slab_mesh;
use crate::triangulate::triangulate;
use crate::TriMesh;
use creabim_core::members::StairCarrier;
use creabim_core::stairshape::StepPlan;
use creabim_core::{Element, Stair};
use glam::{DVec2, DVec3};

/// Is `p` one of the ends of `line`?
fn on_line(p: DVec2, line: [DVec2; 2]) -> bool {
    p.distance(line[0]) < 1e-7 || p.distance(line[1]) < 1e-7
}

/// The edges of tread `k` (1-based) that are the stair's sides, not its
/// nosings.
fn side_edges(sp: &StepPlan, k: usize) -> Vec<(DVec2, DVec2)> {
    let poly = &sp.treads[k - 1];
    let (front, back) = (sp.nosings[k - 1], sp.nosings[k]);
    let n = poly.len();
    (0..n)
        .map(|i| (poly[i], poly[(i + 1) % n]))
        .filter(|(a, b)| {
            !((on_line(*a, front) && on_line(*b, front))
                || (on_line(*a, back) && on_line(*b, back)))
        })
        .collect()
}

/// The walking line's arrowhead at its top end.
fn arrow(walk: &[DVec2]) -> Option<[(DVec2, DVec2); 2]> {
    let n = walk.len();
    if n < 2 {
        return None;
    }
    let end = walk[n - 1];
    let dir = (end - walk[n - 2]).normalize_or_zero();
    let (back, wing) = (-dir * 0.25, dir.perp() * 0.12);
    Some([(end, end + back + wing), (end, end + back - wing)])
}

/// A stair laid out by steps, in a floor plan cut at `cut` above its
/// base: the steps below the cut seen, those past it dashed, a break
/// across the cut tread, and the walking line up.
pub fn step_stair_plan(items: &mut Vec<PlanItem>, e: &Element, s: &Stair, rise: f64, cut: f64) {
    let sp = s.step_plan(rise);
    let r = sp.riser;
    let n = sp.risers as usize;
    let mut line = |pen, a, b| {
        items.push(PlanItem::Line {
            id: e.id,
            pen,
            a,
            b,
        })
    };
    // the first nosing the cut plane passes under
    let cut_at = (0..n).find(|i| (*i + 1) as f64 * r > cut + 1e-9);
    let pen_nosing = |i: usize| match cut_at {
        Some(c) if i > c => Pen::Overhead,
        _ => Pen::Projection,
    };
    for (i, [a, b]) in sp.nosings.iter().enumerate() {
        line(pen_nosing(i), *a, *b);
    }
    for k in 1..n {
        // a tread is seen if its front nosing is
        let pen = pen_nosing(k);
        for (a, b) in side_edges(&sp, k) {
            line(pen, a, b);
        }
    }
    if let Some(c) = cut_at.filter(|c| c + 1 < n) {
        // the break across the tread the cut passes through
        let [i0, o0] = sp.nosings[c];
        let [i1, o1] = sp.nosings[c + 1];
        let (a, b) = ((i0 + i1) / 2.0, (o0 + o1) / 2.0);
        let along = (b - a).normalize_or_zero();
        let side = along.perp() * 0.12;
        let m = (a + b) / 2.0;
        let pts = [
            a - along * 0.1,
            m - along * 0.1,
            m - along * 0.03 + side,
            m + along * 0.03 - side,
            m + along * 0.1,
            b + along * 0.1,
        ];
        for w in pts.windows(2) {
            line(Pen::Symbol, w[0], w[1]);
        }
    }
    for w in sp.walk.windows(2) {
        line(Pen::Symbol, w[0], w[1]);
    }
    if let Some(wings) = arrow(&sp.walk) {
        for (a, b) in wings {
            line(Pen::Symbol, a, b);
        }
    }
    if let (Some(&p0), Some(&p1)) = (sp.walk.first(), sp.walk.get(1)) {
        let d = (p1 - p0).normalize_or_zero();
        items.push(PlanItem::Label {
            id: e.id,
            at: p0 - d * 0.35,
            text: "UP".into(),
        });
    }
}

/// A stair laid out by steps seen whole from above: `line(pen, a, b, z)`
/// as `members::stair_lines`.
pub fn step_stair_lines(s: &Stair, rise: f64, line: &mut dyn FnMut(Pen, DVec2, DVec2, f64)) {
    let sp = s.step_plan(rise);
    let r = sp.riser;
    for (i, [a, b]) in sp.nosings.iter().enumerate() {
        line(Pen::Projection, *a, *b, (i + 1) as f64 * r);
    }
    for k in 1..sp.risers as usize {
        for (a, b) in side_edges(&sp, k) {
            line(Pen::Projection, a, b, k as f64 * r);
        }
    }
    let top = sp.risers as f64 * r;
    for w in sp.walk.windows(2) {
        line(Pen::Symbol, w[0], w[1], top);
    }
    if let Some(wings) = arrow(&sp.walk) {
        for (a, b) in wings {
            line(Pen::Symbol, a, b, top);
        }
    }
}

/// A solid whose top is `poly` at `top` and whose underside runs through
/// `bottom[i]` under each corner (a sloping, maybe twisted soffit).
fn soffit_solid(m: &mut TriMesh, poly: &[DVec2], top: f64, bottom: &[f64]) {
    let n = poly.len();
    if n < 3 {
        return;
    }
    let tris = triangulate(poly);
    let up: Vec<DVec3> = poly.iter().map(|p| p.extend(top)).collect();
    let down: Vec<DVec3> = poly
        .iter()
        .zip(bottom)
        .map(|(p, z)| p.extend(z.min(top - 1e-4)))
        .collect();
    m.add_polygon(&up, &tris, DVec3::Z);
    for t in &tris {
        let [a, b, c] = t.map(|i| down[i]);
        let mut nrm = (b - a).cross(c - a);
        if nrm.z > 0.0 {
            nrm = -nrm;
        }
        m.add_polygon(&[a, b, c], &[[0, 1, 2]], nrm);
    }
    let ccw = crate::triangulate::signed_area2(poly) > 0.0;
    for i in 0..n {
        let j = (i + 1) % n;
        let e = (poly[j] - poly[i]).normalize_or_zero();
        let out = if ccw {
            DVec2::new(e.y, -e.x)
        } else {
            DVec2::new(-e.y, e.x)
        };
        m.add_quad([down[i], down[j], up[j], up[i]], out.extend(0.0));
    }
}

/// How far `p` lies from the line through `l`.
fn off_line(p: DVec2, l: [DVec2; 2]) -> f64 {
    let d = (l[1] - l[0]).normalize_or_zero();
    (p - l[0]).perp_dot(d).abs()
}

/// A plate standing on the nosing `l`, `t` thick toward `into`, from
/// `z0` to `z1`.
fn riser_plate(m: &mut TriMesh, l: [DVec2; 2], into: DVec2, t: f64, z0: f64, z1: f64) {
    if z1 - z0 < 1e-6 || t < 1e-6 {
        return;
    }
    let d = (l[1] - l[0]).normalize_or_zero();
    let mut nrm = d.perp();
    if nrm.dot(into) < 0.0 {
        nrm = -nrm;
    }
    let poly = [l[0], l[1], l[1] + nrm * t, l[0] + nrm * t];
    let poly = if crate::triangulate::signed_area2(&poly) < 0.0 {
        poly.iter().rev().copied().collect::<Vec<_>>()
    } else {
        poly.to_vec()
    };
    m.append(&slab_mesh(&poly, z1, z1 - z0));
}

/// A stringer piece swept along `path` (its top line over the nosings),
/// `v` across (left of the way up positive) and `d` down from the top,
/// square to the slope; cut level at the floor.
fn sweep_piece(m: &mut TriMesh, path: &[DVec3], v: [f64; 2], d: [f64; 2]) {
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let run = (b - a).truncate();
        let len = run.length();
        if len < 1e-6 {
            continue;
        }
        let k = (b.z - a.z) / len;
        let cos = 1.0 / (1.0 + k * k).sqrt();
        let n = (run / len).perp();
        let corner = |p: DVec3, vv: f64, dd: f64| {
            let q = p.truncate() + n * vv;
            q.extend((p.z - dd / cos).max(0.0))
        };
        let c = |p: DVec3| {
            [
                corner(p, v[0], d[0]),
                corner(p, v[1], d[0]),
                corner(p, v[1], d[1]),
                corner(p, v[0], d[1]),
            ]
        };
        let (p, q) = (c(a), c(b));
        if p.iter().chain(&q).all(|x| x.z <= 1e-9) {
            continue;
        }
        let dir = (b - a).normalize_or_zero();
        let up = n.extend(0.0).cross(dir);
        let side = n.extend(0.0);
        m.add_quad([p[0], p[1], q[1], q[0]], up);
        m.add_quad([p[3], p[2], q[2], q[3]], -up);
        m.add_quad([p[1], p[2], q[2], q[1]], side);
        m.add_quad([p[0], p[3], q[3], q[0]], -side);
        m.add_quad([p[0], p[1], p[2], p[3]], -dir);
        m.add_quad([q[0], q[1], q[2], q[3]], dir);
    }
}

/// The parts of a stair laid out by steps, at `base`.
pub fn step_stair_parts(s: &Stair, base: f64, rise: f64) -> StairParts {
    let sp = s.step_plan(rise);
    let mut p = StairParts::default();
    let r = sp.riser;
    let n = sp.risers as usize;
    let (tt, tr) = (s.tread_t(), s.riser_t());
    let concrete = s.carrier.is_concrete() || s.stringer.is_none();
    let cos = 1.0 / (1.0 + (r / sp.going.max(1e-6)).powi(2)).sqrt();
    let wc = s.waist / cos;
    let centroid = |poly: &[DVec2]| poly.iter().copied().sum::<DVec2>() / poly.len().max(1) as f64;
    let lift = |m: TriMesh| {
        let mut m = m;
        for q in &mut m.positions {
            q[2] += base as f32;
        }
        m
    };
    let (mut carrier, mut treads, mut risers) =
        (TriMesh::default(), TriMesh::default(), TriMesh::default());
    for k in 1..n {
        let poly = &sp.treads[k - 1];
        let (front, back) = (sp.nosings[k - 1], sp.nosings[k]);
        let into = centroid(poly) - (front[0] + front[1]) / 2.0;
        let z = k as f64 * r;
        if concrete {
            // the cast step under its finishes: its front set back by the
            // riser finish, its top down by the tread finish; its soffit
            // parallel to the pitch, the waist thick square to it
            let mut cast = poly.clone();
            if tr > 0.0 {
                let d = (front[1] - front[0]).normalize_or_zero();
                let mut nrm = d.perp();
                if nrm.dot(into) < 0.0 {
                    nrm = -nrm;
                }
                for q in cast.iter_mut().filter(|q| on_line(**q, front)) {
                    *q += nrm * tr;
                }
            }
            let bottom: Vec<f64> = cast
                .iter()
                .map(|q| {
                    let (a, b) = (off_line(*q, front), off_line(*q, back));
                    let t = a / (a + b).max(1e-9);
                    ((k - 1) as f64 * r - wc + t * r).max(0.0)
                })
                .collect();
            soffit_solid(&mut carrier, &cast, z - tt, &bottom);
        }
        if tt > 0.0 {
            treads.append(&slab_mesh(poly, z, tt));
        }
        if s.risers_closed && tr > 0.0 {
            riser_plate(&mut risers, front, into, tr, (k - 1) as f64 * r, z - tt);
        }
    }
    if s.risers_closed && tr > 0.0 && n >= 2 {
        // the last riser, up to the floor it climbs to
        let last = sp.nosings[n - 1];
        let before = &sp.treads[n - 2];
        let away = (last[0] + last[1]) / 2.0 - centroid(before);
        riser_plate(
            &mut risers,
            last,
            away,
            tr,
            (n - 1) as f64 * r,
            n as f64 * r - tt,
        );
    }
    if let (false, Some(st)) = (concrete, &s.stringer) {
        // stringers under the boards: their top through each tread's
        // front underside, along the stair's sides or its middle
        let w = s.width;
        let lines: Vec<(bool, f64)> = match s.carrier {
            StairCarrier::CentreStringer => vec![(true, w / 2.0)],
            StairCarrier::TwoStringers => {
                let inset = (st.edge_offset + st.width / 2.0).min(w / 2.0);
                vec![(true, inset), (false, inset)]
            }
            StairCarrier::Concrete => vec![],
        };
        for (left, inset) in lines {
            let path: Vec<DVec3> = s
                .side_path(rise, left, inset)
                .into_iter()
                .map(|q| q - DVec3::Z * tt)
                .collect();
            let out = if left { 1.0 } else { -1.0 };
            for (v, d) in crate::members::stringer_pieces(st, out) {
                sweep_piece(&mut carrier, &path, v, d);
            }
        }
    }
    p.carrier = lift(carrier);
    p.treads = lift(treads);
    p.risers = lift(risers);
    p
}

/// What stands over a stair: the least vertical clearance over its
/// nosings on the walking line, and over its pitch line (the line through
/// the nosings) anywhere along the walking line, m; `None` where nothing
/// is over it. The stair's own body, its railings and rooms are left out;
/// every other construction and piece of equipment counts.
pub fn stair_clearance(model: &creabim_core::Model, s: &Stair) -> (Option<f64>, Option<f64>) {
    use creabim_core::ElementKind;
    let rise = s.rise(model);
    let base = s.base(model);
    // the walking line in plan with the pitch line's height over it, and
    // which of its points are nosings
    let mut path: Vec<(DVec2, f64, bool)> = Vec::new();
    if s.by_steps() {
        let sp = s.step_plan(rise);
        let walk: Vec<DVec2> = sp.walk.clone();
        let mut acc = 0.0;
        let b = sp.going.max(1e-6);
        let r = sp.riser;
        let n = sp.risers as usize;
        let mut next = 0usize;
        for w in walk.windows(2) {
            let len = w[0].distance(w[1]);
            let steps = (len / 0.1).ceil().max(1.0) as usize;
            for i in 0..steps {
                let t = acc + len * i as f64 / steps as f64;
                let p = w[0].lerp(w[1], i as f64 / steps as f64);
                path.push((p, r + t / b * r, false));
            }
            // the nosings on this piece, exactly
            while next < n && (next as f64 * b) <= acc + len + 1e-9 {
                let t = next as f64 * b;
                let p = w[0].lerp(w[1], ((t - acc) / len.max(1e-9)).clamp(0.0, 1.0));
                path.push((p, (next + 1) as f64 * r, true));
                next += 1;
            }
            acc += len;
        }
    } else {
        let lay = s.layout(rise);
        let r = lay.riser;
        for f in &lay.flights {
            let run = f.run(s.tread);
            let steps = (run / 0.1).ceil().max(1.0) as usize;
            for i in 0..=steps {
                let u = run * i as f64 / steps as f64;
                path.push((
                    f.start + f.dir * u,
                    f.z0 + r + u / s.tread.max(1e-6) * r,
                    false,
                ));
            }
            for k in 0..f.risers {
                let u = k as f64 * s.tread;
                path.push((f.start + f.dir * u, f.z0 + (k + 1) as f64 * r, true));
            }
        }
    }
    // the scene without this stair, its railings and the rooms
    let own = |e: &creabim_core::Element| match &e.kind {
        ElementKind::Stair(o) => {
            o.level == s.level
                && o.start.distance(s.start) < 1e-6
                && (o.direction - s.direction).abs() < 1e-6
        }
        ElementKind::Room(_) => true,
        _ => false,
    };
    let skip: std::collections::HashSet<_> =
        model.iter().filter(|e| own(e)).map(|e| e.id).collect();
    let rails: std::collections::HashSet<_> = model
        .iter()
        .filter(|e| match &e.kind {
            ElementKind::Railing(r) => r.stair.is_some_and(|st| skip.contains(&st.stair)),
            _ => false,
        })
        .map(|e| e.id)
        .collect();
    let keep: std::collections::HashSet<_> = model
        .iter()
        .filter(|e| !skip.contains(&e.id) && !rails.contains(&e.id))
        .map(|e| e.id)
        .collect();
    let scene = crate::scene::build_scene_of(model, Some(&keep));
    let up = glam::Vec3::Z;
    let gap = |p: DVec2, z: f64| {
        let o = p.extend(base + z + 0.005).as_vec3();
        crate::pick::pick_items(&scene, o, up, &|i| !i.ghost).map(|h| h.t as f64 + 0.005)
    };
    let (mut h1, mut v): (Option<f64>, Option<f64>) = (None, None);
    for (p, z, nosing) in path {
        if let Some(g) = gap(p, z) {
            v = Some(v.map_or(g, |x: f64| x.min(g)));
            if nosing {
                h1 = Some(h1.map_or(g, |x: f64| x.min(g)));
            }
        }
    }
    (h1, v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::members::{Landing, StairShape};
    use creabim_core::ElementId;

    fn winder_l() -> Stair {
        let mut s = Stair::new(ElementId::new(), DVec2::ZERO, 0.0);
        s.width = 1.0;
        s.tread = 0.25;
        s.max_riser = 0.2;
        s.height = Some(2.8);
        s.shape = StairShape::L;
        s.winders = true;
        s.landing = Some(Landing {
            after: 5,
            left: true,
        });
        s
    }

    #[test]
    fn a_winder_stair_has_a_step_of_concrete_under_every_tread() {
        let s = winder_l();
        let p = step_stair_parts(&s, 0.0, 2.8);
        assert!(!p.carrier.is_empty());
        let (lo, hi) = p.carrier.bounds().unwrap();
        assert!(lo.z >= -1e-4 && (hi.z - 2.6).abs() < 1e-3, "{lo:?} {hi:?}");
    }

    #[test]
    fn a_winder_stair_draws_every_nosing_and_its_walking_line() {
        let s = winder_l();
        let mut lines = 0;
        step_stair_lines(&s, 2.8, &mut |pen, _, _, _| {
            if pen == Pen::Projection {
                lines += 1;
            }
        });
        assert!(lines >= 14 + 13 * 2, "{lines}");
    }
}
