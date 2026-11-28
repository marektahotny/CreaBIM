//! Pipes and ducts in 3D and in plan: a pipe a cylinder (a rectangular
//! duct a box) along its axis, in its system's colour; in plan its
//! outline, washed in that colour, cut or seen below as other members.
//!
//! Where pipes meet end to end they are drawn joined: two rigid pipes at
//! a corner mitre into each other (an elbow), a pipe that is bent rather
//! than fitted (multilayer pipe from a coil, up to the size its maker
//! allows) turns in an arc of the maker's bending radius, and at a tee or
//! a cross no end is closed. The model stays straight pipes sharing a
//! point; the joints are how they are drawn (PilonDesigner's
//! `pipebends.ts`).

use crate::mesh::TriMesh;
use crate::plan::{Pen, PlanItem};
use creabim_core::mep::Pipe;
use creabim_core::{Element, Model};
use glam::{DVec2, DVec3};

/// Sides of a pipe's cylinder.
const SIDES: usize = 16;

/// A system's colour, sRGB 0..1; grey when the system is unknown.
pub fn color(p: &Pipe) -> [f32; 3] {
    let c = p.system_def().map_or(0x8c8c8c, |s| s.color);
    [
        ((c >> 16) & 0xff) as f32 / 255.0,
        ((c >> 8) & 0xff) as f32 / 255.0,
        (c & 0xff) as f32 / 255.0,
    ]
}

/// Its ends in the model, m.
pub fn ends(model: &Model, p: &Pipe) -> (DVec3, DVec3) {
    let z = DVec3::Z * model.level(p.level).map_or(0.0, |l| l.elevation);
    (p.start + z, p.end + z)
}

/// Two axes square to `d` (a unit vector): the first level, unless `d` is
/// upright.
fn across(d: DVec3) -> (DVec3, DVec3) {
    let side = if d.truncate().length() > 1e-9 {
        DVec3::Z.cross(d).normalize()
    } else {
        DVec3::X
    };
    (side, d.cross(side).normalize())
}

/// How one end of a pipe meets other pipes.
#[derive(Clone, Debug, PartialEq)]
pub enum Joint {
    /// Nothing there: the end is closed.
    Free,
    /// One other pipe, bent into: the pipe stops at `at`, where the arc
    /// `arc` (from this pipe's stop to the other's) begins; one of the
    /// two draws it.
    Bend {
        at: DVec3,
        arc: Vec<DVec3>,
        owner: bool,
    },
    /// One other rigid pipe at an angle: `to` is the way it leaves.
    Elbow { to: DVec3, owner: bool },
    /// Two or more others (a tee, a cross), or one straight on.
    Node { owner: bool },
}

use creabim_services::fittings::{bend, key};

/// How the pipe's start and end meet the model's other pipes.
pub fn joints(model: &Model, p: &Pipe) -> [Joint; 2] {
    let (a, b) = ends(model, p);
    let mut out = [Joint::Free, Joint::Free];
    if a.distance(b) < 1e-6 {
        return out;
    }
    // the others at each end: (their pipe, their far end)
    let mut at: [Vec<(&Pipe, DVec3)>; 2] = [vec![], vec![]];
    let keys = [key(a), key(b)];
    for e in model.iter() {
        let creabim_core::ElementKind::Pipe(q) = &e.kind else {
            continue;
        };
        let (qa, qb) = ends(model, q);
        if (key(qa) == keys[0] && key(qb) == keys[1]) || (key(qa) == keys[1] && key(qb) == keys[0])
        {
            // itself (or one laid over it)
            continue;
        }
        for (i, k) in keys.iter().enumerate() {
            if key(qa) == *k {
                at[i].push((q, qb));
            } else if key(qb) == *k {
                at[i].push((q, qa));
            }
        }
    }
    // of those at a node, the one whose far end comes first draws what is shared
    let first = |x: DVec3, y: DVec3| (x.x, x.y, x.z) < (y.x, y.y, y.z);
    for (i, others) in at.iter().enumerate() {
        let (node, far) = if i == 0 { (a, b) } else { (b, a) };
        let owner = others.iter().all(|(_, f)| first(far, *f));
        out[i] = match others.as_slice() {
            [] => Joint::Free,
            [(q, qfar)] => {
                let (ua, la) = ((far - node).normalize(), far.distance(node));
                let lb = qfar.distance(node);
                if lb < 1e-6 {
                    Joint::Node { owner }
                } else {
                    let ub = (*qfar - node) / lb;
                    // the angle between the two arms, and how far the run turns
                    let th = ua.dot(ub).clamp(-1.0, 1.0).acos();
                    let dev = std::f64::consts::PI - th;
                    let least = 5.0_f64.to_radians();
                    if dev < least || th < least {
                        Joint::Node { owner }
                    } else {
                        let bent = bend(p, q, la, lb, th).map(|r| {
                            let t = r / (th / 2.0).tan();
                            {
                                let (t1, t2) = (node + ua * t, node + ub * t);
                                let c = node + (ua + ub).normalize() * (r / (th / 2.0).sin());
                                let (r1, r2) = (t1 - c, t2 - c);
                                let n =
                                    ((dev / (std::f64::consts::PI / 16.0)).ceil() as usize).max(4);
                                let s = dev.sin();
                                let arc = (0..=n)
                                    .map(|k| {
                                        let f = k as f64 / n as f64;
                                        c + r1 * (((1.0 - f) * dev).sin() / s)
                                            + r2 * ((f * dev).sin() / s)
                                    })
                                    .collect();
                                (t1, arc)
                            }
                        });
                        match bent {
                            Some((at, arc)) => Joint::Bend { at, arc, owner },
                            None => Joint::Elbow { to: ub, owner },
                        }
                    }
                }
            }
            _ => Joint::Node { owner },
        };
    }
    out
}

/// Rings of a round pipe of radius `r` along `path`, each square to the
/// way the path runs there, as a tube without ends.
pub(crate) fn tube(m: &mut TriMesh, path: &[DVec3], r: f64) {
    let path: Vec<(DVec3, f64)> = path.iter().map(|p| (*p, r)).collect();
    taper(m, &path)
}

/// As [`tube`], each ring of its own radius: a reducer is a cone.
pub(crate) fn taper(m: &mut TriMesh, path: &[(DVec3, f64)]) {
    if path.len() < 2 {
        return;
    }
    let (path, radii): (Vec<DVec3>, Vec<f64>) = path.iter().copied().unzip();
    let path = path.as_slice();
    let dir = |i: usize| {
        let (lo, hi) = (i.saturating_sub(1), (i + 1).min(path.len() - 1));
        (path[hi] - path[lo]).normalize_or_zero()
    };
    let (mut u, _) = across(dir(0));
    let base = m.positions.len() as u32;
    for (i, c) in path.iter().enumerate() {
        let d = dir(i);
        let r = radii[i];
        // carried along the path, so the rings don't twist
        u = (u - d * u.dot(d)).normalize_or(across(d).0);
        let v = d.cross(u);
        for k in 0..SIDES {
            let t = k as f64 / SIDES as f64 * std::f64::consts::TAU;
            let n = u * t.cos() + v * t.sin();
            m.positions.push((*c + n * r).as_vec3().to_array());
            m.normals.push(n.as_vec3().to_array());
        }
    }
    let s = SIDES as u32;
    for i in 0..path.len() as u32 - 1 {
        for k in 0..s {
            let j = (k + 1) % s;
            let (a0, a1, b0, b1) = (
                base + i * s + k,
                base + i * s + j,
                base + (i + 1) * s + k,
                base + (i + 1) * s + j,
            );
            m.indices.extend([a0, a1, b1, a0, b1, b0]);
        }
    }
}

/// A ball of radius `r` at `c`: what an elbow or a tee is in 3D.
pub(crate) fn ball(m: &mut TriMesh, c: DVec3, r: f64) {
    const RINGS: usize = 8;
    let base = m.positions.len() as u32;
    for i in 0..=RINGS {
        let lat = i as f64 / RINGS as f64 * std::f64::consts::PI;
        for k in 0..SIDES {
            let lon = k as f64 / SIDES as f64 * std::f64::consts::TAU;
            let n = DVec3::new(lat.sin() * lon.cos(), lat.sin() * lon.sin(), lat.cos());
            m.positions.push((c + n * r).as_vec3().to_array());
            m.normals.push(n.as_vec3().to_array());
        }
    }
    let s = SIDES as u32;
    for i in 0..RINGS as u32 {
        for k in 0..s {
            let j = (k + 1) % s;
            let (a0, a1, b0, b1) = (
                base + i * s + k,
                base + i * s + j,
                base + (i + 1) * s + k,
                base + (i + 1) * s + j,
            );
            m.indices.extend([a0, b1, a1, a0, b0, b1]);
        }
    }
}

/// The socket of a fitting on a round pipe's end, as it is drawn: its
/// radius and its length along the pipe, m. A drain's socket (hrdlo) is
/// a little wider than the pipe and about half its diameter deep; a
/// press or welded water fitting's sleeve wider and shorter. None on an
/// insulated pipe, whose insulation hides it.
pub fn socket(p: &Pipe) -> Option<(f64, f64)> {
    if p.height_mm.is_some() || p.insulation_mm > 0.0 {
        return None;
    }
    // a flexible hose (ATREA GP) is laid in one piece: no fitting
    if creabim_services::pipe_types::by_id(&p.pipe_type).is_some_and(|t| t.flex) {
        return None;
    }
    let d = p.outer_mm;
    let drain = creabim_services::pipe_types::by_id(&p.pipe_type).map_or_else(
        || p.system == "K" || p.system.starts_with('K'),
        |t| {
            t.media
                .contains(&creabim_services::pipe_types::Medium::Kanal)
        },
    );
    let (r, l) = if drain {
        (d * 0.56 + 3.0, d * 0.5 + 15.0)
    } else {
        (d * 0.68 + 2.0, d * 0.8 + 10.0)
    };
    Some((r / 1000.0, l / 1000.0))
}

/// Does this end of the pipe sit in a fitting (an elbow, a tee, a
/// branch, a cross)?
fn fitted(j: &Joint) -> bool {
    matches!(j, Joint::Elbow { .. } | Joint::Node { .. })
}

/// How long a reducer from `big` to `small` (outside diameters, mm) is
/// drawn, m.
fn taper_len(big: f64, small: f64) -> f64 {
    ((big - small) * 1.5).max(30.0) / 1000.0
}

/// The pipe's solid, insulation included, with what joins it to the
/// pipes at its ends: the arc it is bent in, or a ball where it is
/// fitted to them.
pub fn pipe_mesh(model: &Model, p: &Pipe) -> TriMesh {
    let (mut a, mut b) = ends(model, p);
    let mut m = TriMesh::default();
    if a.distance(b) < 1e-9 {
        return m;
    }
    let ins = p.insulation_mm / 1000.0;
    let (w, h) = p.section_m();
    let round = p.height_mm.is_none();
    let js = joints(model, p);
    if round {
        let r = w / 2.0 + ins;
        // narrower than the largest pipe at an end: a cone from its size
        let red = creabim_services::fittings::reductions_of(model, p);
        let along = (b - a).normalize_or_zero();
        for (i, big) in red.iter().enumerate() {
            let (Some(big), false) = (big, matches!(js[i], Joint::Bend { .. })) else {
                continue;
            };
            let (node, out) = if i == 0 { (a, along) } else { (b, -along) };
            let rb = big / 2000.0 + ins;
            let l = taper_len(*big, p.outer_mm).min(a.distance(b) / 2.0);
            taper(&mut m, &[(node, rb), (node + out * l, r)]);
        }
        for (i, j) in js.iter().enumerate() {
            let node = if i == 0 { a } else { b };
            match j {
                Joint::Bend { at, arc, owner } => {
                    if *owner {
                        tube(&mut m, arc, r);
                    }
                    if i == 0 {
                        a = *at;
                    } else {
                        b = *at;
                    }
                }
                Joint::Elbow { owner: true, .. } | Joint::Node { owner: true } => {
                    ball(&mut m, node, socket(p).map_or(r, |s| s.0))
                }
                _ => {}
            }
        }
        // the fitting's socket on each fitted end, stepping down to the
        // pipe where it ends
        if let Some((rs, ls)) = socket(p) {
            let along = (b - a).normalize_or_zero();
            let ls = ls.min(a.distance(b) / 2.0);
            for (i, j) in js.iter().enumerate() {
                if !fitted(j) || red[i].is_some() {
                    continue;
                }
                let (node, out) = if i == 0 { (a, along) } else { (b, -along) };
                let tip = node + out * ls;
                taper(&mut m, &[(node, rs), (tip, rs), (tip + out * 1e-4, r)]);
            }
        }
    }
    let d = (b - a).normalize_or_zero();
    if d == DVec3::ZERO {
        return m;
    }
    let (u, v) = across(d);
    match p.height_mm {
        Some(_) => {
            let (hw, hh) = (w / 2.0 + ins, h / 2.0 + ins);
            let corner = |s: f64, t: f64| u * s * hw + v * t * hh;
            let ring = [
                corner(-1.0, -1.0),
                corner(1.0, -1.0),
                corner(1.0, 1.0),
                corner(-1.0, 1.0),
            ];
            for i in 0..4 {
                let (c0, c1) = (ring[i], ring[(i + 1) % 4]);
                let n = (c0 + c1).normalize();
                m.add_quad([a + c0, a + c1, b + c1, b + c0], n);
            }
            m.add_polygon(&ring.map(|c| a + c), &[[0, 1, 2], [0, 2, 3]], -d);
            m.add_polygon(&ring.map(|c| b + c), &[[0, 1, 2], [0, 2, 3]], d);
        }
        None => {
            let r = w / 2.0 + ins;
            let ring: Vec<DVec3> = (0..SIDES)
                .map(|i| {
                    let t = i as f64 / SIDES as f64 * std::f64::consts::TAU;
                    (u * t.cos() + v * t.sin()) * r
                })
                .collect();
            // smooth sides: a normal at each vertex
            let base = m.positions.len() as u32;
            for c in &ring {
                let n = c.normalize().as_vec3().to_array();
                for end in [a, b] {
                    m.positions.push((end + *c).as_vec3().to_array());
                    m.normals.push(n);
                }
            }
            for i in 0..SIDES as u32 {
                let j = (i + 1) % SIDES as u32;
                let (a0, b0, a1, b1) = (
                    base + 2 * i,
                    base + 2 * i + 1,
                    base + 2 * j,
                    base + 2 * j + 1,
                );
                m.indices.extend([a0, a1, b1, a0, b1, b0]);
            }
            let fan: Vec<[usize; 3]> = (1..SIDES - 1).map(|i| [0, i, i + 1]).collect();
            let cap = |end: DVec3| ring.iter().map(|c| end + *c).collect::<Vec<_>>();
            m.add_polygon(&cap(a), &fan, -d);
            m.add_polygon(&cap(b), &fan, d);
        }
    }
    m
}

/// The least radius a riser's circle is drawn with in plan, m.
pub const RISER_MARK: f64 = 0.035;

/// The pipe as one line along its axis, as a services drawing shows it
/// (the office's ZTI and ÚK plans, PilonDesigner's "výkres TZB"): the
/// view gives the line its system's colour and line type. A riser is a
/// small circle; a bent pipe turns in its arc.
pub fn pipe_line(model: &Model, e: &Element, p: &Pipe) -> Vec<PlanItem> {
    let pen = Pen::Projection;
    let line = |a: DVec2, b: DVec2| PlanItem::Line {
        id: e.id,
        pen,
        a,
        b,
    };
    let (mut a, mut b) = (p.start.truncate(), p.end.truncate());
    if a.distance(b) < 1e-6 {
        return vec![PlanItem::Polygon {
            id: e.id,
            pen,
            points: (0..SIDES)
                .map(|i| {
                    let t = i as f64 / SIDES as f64 * std::f64::consts::TAU;
                    a + DVec2::new(t.cos(), t.sin()) * RISER_MARK
                })
                .collect(),
        }];
    }
    let z = model.level(p.level).map_or(0.0, |l| l.elevation);
    let flat = |q: DVec3| (q - DVec3::Z * z).truncate();
    let mut items = vec![];
    for (i, j) in joints(model, p).iter().enumerate() {
        if let Joint::Bend { at, arc, owner } = j {
            if i == 0 {
                a = flat(*at);
            } else {
                b = flat(*at);
            }
            if *owner {
                let pts: Vec<DVec2> = arc.iter().map(|q| flat(*q)).collect();
                items.extend(pts.windows(2).map(|w| line(w[0], w[1])));
            }
        }
    }
    items.push(line(a, b));
    items
}

/// What a services drawing writes at a pipe, as the office's: a water
/// pipe its system's code and outside diameter by wall (`SV 25x3.5`), a drain its
/// `DN`, the rest their system's code and size.
pub fn pipe_text(p: &Pipe) -> String {
    use creabim_core::mep::SystemKind as K;
    let n = |v: f64| {
        if (v - v.round()).abs() < 0.05 {
            format!("{v:.0}")
        } else {
            format!("{v:.1}")
        }
    };
    // an insulated pipe with an `i` after its size (George 2026-11-26)
    let i = if p.insulation_mm > 0.0 { "i" } else { "" };
    let size = format!(
        "{}x{}{i}",
        n(p.outer_mm),
        n((p.outer_mm - p.inner_mm).max(0.0) / 2.0)
    );
    match p.kind() {
        // SV, TV, C before the size (George 2026-11-26)
        K::ColdWater => format!("SV {size}"),
        K::HotWater => format!("TV {size}"),
        K::Circulation => format!("C {size}"),
        // a duct: its diameter, and its insulation under it, as VZT
        // drawings write them (George 2026-11-25)
        // and its height above the floor, as the samples write it: a
        // round duct's axis "OS 2800 NP", a rectangular one's bottom
        // edge "SH 2400 NP", its size "400/200" (George 2026-11-25)
        K::AirSupply | K::AirExhaust => {
            let mut lines = vec![match p.height_mm {
                Some(h) => format!("{}/{}", n(p.nominal_mm), n(h)),
                None => format!("Ø{}", n(p.nominal_mm)),
            }];
            if (p.start.z - p.end.z).abs() < 1e-3 && !p.is_vertical() {
                let z = p.start.z * 1000.0;
                lines.push(match p.height_mm {
                    Some(h) => format!("SH {:.0} NP", z - h / 2.0),
                    None => format!("OS {z:.0} NP"),
                });
            }
            if p.insulation_mm > 0.0 {
                lines.push(format!(
                    "{} {} mm",
                    creabim_core::words::tr("insulation"),
                    n(p.insulation_mm)
                ));
            }
            lines.join("\n")
        }
        K::Sanitary | K::Storm | K::Flue => {
            format!("DN{}{i}", n(p.nominal_mm))
        }
        _ => format!("{} {size}", p.system),
    }
}

/// The mark of the stack a riser is of, as the office's drawings number
/// them: `K` for foul drains, `D` for rainwater, `V` for water, with its
/// number among the model's stacks of that letter, counted from the left
/// and then from the bottom of the plan, so a stack has one mark on every
/// floor. None for the other systems' risers.
pub fn stack_mark(model: &Model, p: &Pipe) -> Option<String> {
    use creabim_core::mep::SystemKind as K;
    use creabim_core::ElementKind;
    let letter = |k: K| match k {
        K::Sanitary => Some("K"),
        K::Storm => Some("D"),
        K::ColdWater | K::HotWater | K::Circulation => Some("V"),
        _ => None,
    };
    let l = letter(p.kind())?;
    // a stack's place, to the centimetre; water risers standing together
    // (cold, hot, circulation within 30 cm) are one stack
    let cell = |q: DVec2| ((q.x * 100.0).round() as i64, (q.y * 100.0).round() as i64);
    // the vertical pipes of the letter: their places and heights
    let upright: Vec<((i64, i64), f64, f64)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Pipe(o)
                if letter(o.kind()) == Some(l)
                    && o.start.truncate().distance(o.end.truncate()) < 1e-6 =>
            {
                let z = model.level(o.level).map_or(0.0, |l| l.elevation);
                Some((
                    cell(o.start.truncate()),
                    z + o.start.z.min(o.end.z),
                    z + o.start.z.max(o.end.z),
                ))
            }
            _ => None,
        })
        .collect();
    // a stack goes through a ceiling into another storey; a rise or a
    // drop within its storey, into the floor too, is none (the office's
    // rule of drawing, for every trade)
    let floors = floors_between(model);
    let through = |c: (i64, i64)| {
        let here = upright.iter().filter(|u| u.0 == c);
        let lo = here.clone().map(|u| u.1).fold(f64::MAX, f64::min);
        let hi = here.map(|u| u.2).fold(f64::MIN, f64::max);
        floors.iter().any(|f| lo < f - CEILING && hi > f - 1e-6)
    };
    let mut places: Vec<(i64, i64)> = upright.iter().map(|u| u.0).collect();
    places.sort();
    places.dedup();
    places.retain(|c| through(*c));
    let near = if l == "V" { 30 } else { 0 };
    let mut stacks: Vec<(i64, i64)> = vec![];
    for q in places {
        if !stacks
            .iter()
            .any(|s| (s.0 - q.0).abs() <= near && (s.1 - q.1).abs() <= near)
        {
            stacks.push(q);
        }
    }
    let me = cell(p.start.truncate());
    let i = stacks
        .iter()
        .position(|s| (s.0 - me.0).abs() <= near && (s.1 - me.1).abs() <= near)?;
    Some(format!("{l}{}", i + 1))
}

/// As far under a storey's floor as its ceiling below may reach, m: a
/// pipe that only dips into the floor stays above it.
const CEILING: f64 = 0.5;

/// The ceilings a stack may go through: the floors that have a storey
/// under them and the top storey's ceiling.
fn floors_between(model: &Model) -> Vec<f64> {
    let mut z: Vec<f64> = model.levels().map(|(_, l)| l.elevation).collect();
    // and the top storey's ceiling, which a vent goes through to the roof
    let top = model
        .levels()
        .map(|(_, l)| l.elevation + l.height)
        .fold(f64::MIN, f64::max);
    z.push(top);
    z.sort_by(f64::total_cmp);
    z.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
    z.into_iter().skip(1).collect()
}

/// Does a rainwater pipe lead on from the stack standing at `at`: one
/// that is no riser, with an end at the stack?
fn led_away(model: &Model, at: DVec2) -> bool {
    use creabim_core::mep::SystemKind as K;
    model.iter().any(|e| match &e.kind {
        creabim_core::ElementKind::Pipe(o) if o.kind() == K::Storm => {
            let (a, b) = (o.start.truncate(), o.end.truncate());
            a.distance(b) > 1e-6 && (a.distance(at) < 0.05 || b.distance(at) < 0.05)
        }
        _ => false,
    })
}

/// The green the office's services drawings write their pipes' sizes
/// and fixtures' marks in.
pub const LABEL_GREEN: [f32; 3] = [0.0, 0.6, 0.32];

/// The labels of a services plan's pipes: pipes running side by side
/// share one, a line each in their order across, turned along them and
/// set beside the bundle, with where they run (in the floor, in a wall)
/// under them; a riser has its size beside its circle. The same label is
/// not written again within four metres. `mm` is drawing units per paper
/// mm.
pub fn pipe_labels(model: &Model, pipes: &[&Element], mm: f64) -> Vec<PlanItem> {
    use crate::annotation::{Annotation, Shape, TextLook};
    use creabim_core::ElementKind;
    let look = TextLook {
        size: 2.0,
        rgb: Some(LABEL_GREEN),
        ..TextLook::default()
    };
    let label = |id, at: DVec2, angle: f64, text: String| PlanItem::Annotation {
        id,
        item: Box::new(Annotation {
            shape: Shape::Along { at, angle },
            text,
            look,
            mm,
        }),
    };
    struct Run {
        id: creabim_core::ElementId,
        a: DVec2,
        u: DVec2,
        len: f64,
        z: f64,
        level: creabim_core::ElementId,
        text: String,
        /// where its label was dragged to, from where it falls by itself
        moved: Option<DVec2>,
        /// half its width with the insulation, m: a duct's label stands
        /// clear of its outline
        half: f64,
    }
    let mut out = vec![];
    let mut runs: Vec<Run> = vec![];
    // the stacks met: mark, the first riser and its place, their sizes
    type Stack = (
        String,
        creabim_core::ElementId,
        DVec2,
        Vec<String>,
        Option<DVec2>,
    );
    let mut stacks: Vec<Stack> = vec![];
    for e in pipes {
        let ElementKind::Pipe(p) = &e.kind else {
            continue;
        };
        let (a, b) = (p.start.truncate(), p.end.truncate());
        let len = a.distance(b);
        if len < 1e-6 {
            // a riser: its stack's mark in a circle up and to the right
            // (K1, D2, V1), its size beside that
            let text = pipe_text(p);
            let Some(mark) = stack_mark(model, p) else {
                let home = a + DVec2::X * (RISER_MARK + (look.width(&text) / 2.0 + 1.0) * mm);
                // dragged away: the label alone, no leader (George
                // 2026-11-25)
                let at = home + p.mark.unwrap_or(DVec2::ZERO);
                out.push(label(e.id, at, 0.0, text));
                continue;
            };
            // risers of one stack share its mark, a line of sizes each
            match stacks.iter_mut().find(|s| s.0 == mark) {
                Some(s) => {
                    if !s.3.contains(&text) {
                        s.3.push(text);
                    }
                }
                None => stacks.push((mark, e.id, a, vec![text], p.mark)),
            }
            continue;
        }
        if len < (0.8f64).max(14.0 * mm) {
            continue;
        }
        // reading left to right, or upwards
        let (a, b) =
            if (b.x - a.x).abs() > 1e-9 && b.x < a.x || (b.x - a.x).abs() <= 1e-9 && b.y < a.y {
                (b, a)
            } else {
                (a, b)
            };
        runs.push(Run {
            id: e.id,
            a,
            u: (b - a) / len,
            len,
            z: (p.start.z + p.end.z) / 2.0,
            level: p.level,
            text: pipe_text(p),
            moved: p.mark,
            half: if p.kind().is_air() {
                p.section_m().0 / 2.0 + p.insulation_mm / 1000.0
            } else {
                0.0
            },
        });
    }
    for (mark, id, a, sizes, moved) in stacks {
        // the line starts at the riser; the circle goes where it was put
        let at = a + moved.unwrap_or(DVec2::new(5.0, 5.0) * mm);
        out.push(PlanItem::Tag {
            id,
            anchor: a,
            at,
            elbow: None,
            text: mark.clone(),
            round: true,
            look,
        });
        let mut sizes = sizes;
        // a rainwater pipe nothing leads away from under the ground
        // empties onto it
        if mark.starts_with('D') && !led_away(model, a) {
            sizes.push(creabim_core::words::tr("Discharge above the ground"));
        }
        let w = sizes.iter().map(|t| look.width(t)).fold(0.0, f64::max);
        let beside = at + DVec2::X * (3.5 + w / 2.0) * mm;
        out.push(label(id, beside, 0.0, sizes.join("\n")));
    }
    runs.sort_by(|x, y| y.len.total_cmp(&x.len));
    let in_wall = |level, q: DVec2| {
        model.iter().any(|e| match &e.kind {
            ElementKind::Wall(w) if w.level == level && !w.is_arc() => {
                let d = w.end - w.start;
                let t = ((q - w.start).dot(d) / d.length_squared().max(1e-12)).clamp(0.0, 1.0);
                (w.start + d * t).distance(q) < w.thickness / 2.0
            }
            _ => false,
        })
    };
    let mut used = vec![false; runs.len()];
    let mut placed: Vec<(String, DVec2)> = vec![];
    for i in 0..runs.len() {
        if used[i] {
            continue;
        }
        used[i] = true;
        let seed = &runs[i];
        let (u, n) = (seed.u, seed.u.perp());
        // along the seed: where the bundle's pipes all run, and each
        // one's place across
        let (mut lo, mut hi) = (0.0, seed.len);
        let mut members = vec![(0.0, i)];
        for j in i + 1..runs.len() {
            let r = &runs[j];
            if used[j] || r.u.perp_dot(u).abs() > 0.02 {
                continue;
            }
            let across = (r.a - seed.a).dot(n);
            let (s0, s1) = (
                (r.a - seed.a).dot(u),
                (r.a - seed.a).dot(u) + r.len * r.u.dot(u),
            );
            let (s0, s1) = (s0.min(s1), s0.max(s1));
            let both = s1.min(hi) - s0.max(lo);
            if across.abs() < 0.4 && both >= 0.5 * r.len.min(hi - lo) {
                used[j] = true;
                members.push((across, j));
                (lo, hi) = (lo.max(s0), hi.min(s1));
            }
        }
        // top line first, as they lie across
        members.sort_by(|x, y| y.0.total_cmp(&x.0));
        let mut lines: Vec<String> = vec![];
        for (_, j) in &members {
            if !lines.contains(&runs[*j].text) {
                lines.push(runs[*j].text.clone());
            }
        }
        let mid = seed.a + u * (lo + hi) / 2.0;
        let floor = members.iter().all(|(_, j)| runs[*j].z < 0.1);
        let wall = members
            .iter()
            .all(|(c, _)| in_wall(seed.level, mid + n * *c));
        if wall {
            lines.push(creabim_core::words::tr("in the wall"));
        } else if floor {
            lines.push(creabim_core::words::tr("in the floor"));
        }
        let text = lines.join("\n");
        if placed
            .iter()
            .any(|(t, at)| *t == text && at.distance(mid) < 4.0)
        {
            continue;
        }
        placed.push((text.clone(), mid));
        // beside the bundle, clear of its outermost pipe
        let top = members.first().map_or(0.0, |m| m.0);
        let half = lines.len() as f64 * look.size * 1.25 / 2.0;
        let wide = members
            .iter()
            .map(|(_, j)| runs[*j].half)
            .fold(0.0, f64::max);
        let home = mid + n * (top + wide + (half + 1.0) * mm);
        let moved = seed.moved.unwrap_or(DVec2::ZERO);
        // dragged away: the label alone, no leader (George 2026-11-25)
        let at = home + moved;
        out.push(label(seed.id, at, u.to_angle(), text));
    }
    out
}

/// One position of a VZT drawing: a kind of device on the ducts (an
/// extract valve, a fan, a roof outlet, a silencer), each product and
/// size once, numbered as the legend lists them (George 2026-11-25).
#[derive(Clone, Debug, PartialEq)]
pub struct AirPosition {
    /// Its device (zariadenie) and its number in it, as George's sample
    /// VZT drawings number positions: "1.12".
    pub device: usize,
    pub number: usize,
    /// What the device is: "Rekuperácia", "Odvetranie 1.07+1.08", or empty
    /// for the devices drawn by hand.
    pub device_name: String,
    pub name: String,
    pub valves: Vec<creabim_core::ElementId>,
}

impl AirPosition {
    /// "1.12"
    pub fn label(&self) -> String {
        format!("{}.{}", self.device, self.number)
    }
}

/// The devices on the model's ducts, as positions: in the order of the
/// valve kinds, then by name.
pub fn air_positions(model: &Model) -> Vec<AirPosition> {
    use creabim_core::ElementKind;
    use creabim_services::extractfans as xf;
    let mut out: Vec<(usize, AirPosition)> = vec![];
    for e in model.iter() {
        let ElementKind::Valve(v) = &e.kind else {
            continue;
        };
        let Some(p) = v.host(model) else { continue };
        // its device: what its duct's name starts with ("Rekuperácia –
        // …", "Odvetranie 1.07 – …"), as the designs name them
        let device_name = model
            .get(v.pipe)
            .and_then(|d| d.name.split_once(" – ").map(|x| x.0.to_string()))
            .unwrap_or_default();
        if !p.kind().is_air() {
            continue;
        }
        let product = v.product.as_deref().unwrap_or("");
        let name = if let Some(f) = xf::fan(product) {
            let speed = v.preset.as_deref().filter(|_| f.speeds.len() > 1);
            match speed {
                Some(s) => format!("{} {} ({s})", f.maker, f.name),
                None => format!("{} {}", f.maker, f.name),
            }
        } else if let Some(a) = xf::valve(product) {
            format!("{} {}", a.maker, a.name)
        } else {
            let kind = v.kind_def().map_or(v.kind.clone(), |k| k.name.to_string());
            let dn = p.nominal_mm;
            format!("{kind} Ø{dn:.0}")
        };
        let order = creabim_core::mep::VALVE_KINDS
            .iter()
            .position(|k| k.code == v.kind)
            .unwrap_or(usize::MAX);
        match out
            .iter_mut()
            .find(|x| x.1.name == name && x.1.device_name == device_name)
        {
            Some(x) => x.1.valves.push(e.id),
            None => out.push((
                order,
                AirPosition {
                    device: 0,
                    number: 0,
                    device_name,
                    name,
                    valves: vec![e.id],
                },
            )),
        }
    }
    // the devices: heat recovery first, then by name, the hand-drawn last
    let rank = |d: &str| (d.is_empty(), !d.starts_with("Rekuper"), d.to_string());
    out.sort_by(|a, b| {
        rank(&a.1.device_name)
            .cmp(&rank(&b.1.device_name))
            .then(a.0.cmp(&b.0))
            .then_with(|| a.1.name.cmp(&b.1.name))
    });
    let mut devices: Vec<String> = vec![];
    let mut counts: Vec<usize> = vec![];
    out.into_iter()
        .map(|(_, mut x)| {
            let d = match devices.iter().position(|d| *d == x.device_name) {
                Some(d) => d,
                None => {
                    devices.push(x.device_name.clone());
                    counts.push(0);
                    devices.len() - 1
                }
            };
            counts[d] += 1;
            x.device = d + 1;
            x.number = counts[d];
            x
        })
        .collect()
}

/// The position marks the VZT plans still lack, as tags (the Tag tool's
/// own, George 2026-11-25: dragged with the mouse like any tag): each
/// device on the ducts in each ventilation plan of its storey, its number
/// in a circle up and to the right of it, in the label green.
pub fn air_position_tags(model: &Model) -> Vec<Element> {
    use creabim_core::viewmodes::{DisplayMode, Trade};
    use creabim_core::{ElementKind, ViewKind};
    let positions = air_positions(model);
    if positions.is_empty() {
        return vec![];
    }
    let tagged: std::collections::HashSet<(creabim_core::ElementId, creabim_core::ElementId)> =
        model
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Tag(t) if !t.ufh => Some((t.view, t.host)),
                _ => None,
            })
            .collect();
    let mut out = vec![];
    for ve in model.iter() {
        let ElementKind::View(v) = &ve.kind else {
            continue;
        };
        let ViewKind::Plan { level, .. } = v.kind else {
            continue;
        };
        if v.callout.is_some()
            || v.graphics.range.from_above
            || DisplayMode::of(v) != DisplayMode::Services
            || v.trade.is_some_and(|t| t != Trade::Ventilation)
        {
            continue;
        }
        let mm = v.scale / 1000.0;
        for pos in &positions {
            for &id in &pos.valves {
                let Some(ElementKind::Valve(x)) = model.get(id).map(|e| &e.kind) else {
                    continue;
                };
                if x.host(model).is_none_or(|p| p.level != level) || tagged.contains(&(ve.id, id)) {
                    continue;
                }
                let mut t =
                    creabim_core::Tag::new(ve.id, id, x.mark.unwrap_or(DVec2::new(6.0, 6.0) * mm));
                t.color = Some(LABEL_GREEN);
                out.push(Element::new("Position tag", ElementKind::Tag(t)));
            }
        }
    }
    out
}

/// A device's position mark in a VZT drawing: its number in a circle,
/// the leader from the device, the circle where it was dragged to or up
/// and to the right. `mm` is drawing units per paper mm.
pub fn air_mark(
    model: &Model,
    e: &Element,
    v: &creabim_core::mep::Valve,
    number: &str,
    mm: f64,
) -> Option<PlanItem> {
    let pipe = v.host(model)?;
    let anchor = v.at(pipe).truncate();
    let at = anchor + v.mark.unwrap_or(DVec2::new(6.0, 6.0) * mm);
    Some(PlanItem::Tag {
        id: e.id,
        anchor,
        at,
        elbow: None,
        text: number.to_string(),
        round: true,
        look: crate::annotation::TextLook {
            size: 2.5,
            rgb: Some(LABEL_GREEN),
            ..Default::default()
        },
    })
}

/// Which way the air runs in each duct: `true` from its start to its end.
/// The ducts of one system joined end to end are walked from their root:
/// the end at a ventilation unit (an object within 1 m in plan of a loose
/// end), else every loose end without an air terminal on it (a fan's
/// roof outlet). Supply and exhaust to outdoors run away from it, extract
/// and outdoor air towards it. A duct the walk doesn't reach is left out.
pub fn air_flows(model: &Model) -> std::collections::HashMap<creabim_core::ElementId, bool> {
    use creabim_core::mep::SystemKind;
    use creabim_core::ElementKind;
    use std::collections::{HashMap, VecDeque};
    let key = |p: DVec3| {
        (
            (p.x * 500.0).round() as i64,
            (p.y * 500.0).round() as i64,
            (p.z * 500.0).round() as i64,
        )
    };
    let world = |p: &Pipe| {
        let z = DVec3::Z * model.level(p.level).map_or(0.0, |l| l.elevation);
        [p.start + z, p.end + z]
    };
    // ducts per system, and the ends they share
    let ducts: Vec<(&Element, &Pipe)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Pipe(p) if p.kind().is_air() => Some((e, p)),
            _ => None,
        })
        .collect();
    // (system, end) to the ducts' ends there
    type Ends = HashMap<(String, (i64, i64, i64)), Vec<(usize, usize)>>;
    let mut at: Ends = HashMap::new();
    for (i, (_, p)) in ducts.iter().enumerate() {
        for (k, q) in world(p).into_iter().enumerate() {
            at.entry((p.system.clone(), key(q)))
                .or_default()
                .push((i, k));
        }
    }
    // the air terminals' ends, and the units
    let mut terminal = vec![[false; 2]; ducts.len()];
    for e in model.iter() {
        if let ElementKind::Valve(v) = &e.kind {
            if v.is_air_terminal() {
                if let Some(i) = ducts.iter().position(|(d, _)| d.id == v.pipe) {
                    terminal[i][usize::from(v.t > 0.5)] = true;
                }
            }
        }
    }
    let units: Vec<DVec2> = model
        .iter()
        .filter_map(|e| match &e.kind {
            // an air handling or heat recovery unit (the catalogue's
            // vzt/duplex/…, a room unit vzt/dec/…)
            ElementKind::Object(o)
                if creabim_core::objects::type_of(model, e)
                    .and_then(|t| t.catalogue.as_deref())
                    .is_some_and(|c| c.starts_with("vzt/duplex/") || c.starts_with("vzt/dec/")) =>
            {
                Some(o.position)
            }
            _ => None,
        })
        .collect();
    let loose = |i: usize, k: usize| {
        let p = ducts[i].1;
        at.get(&(p.system.clone(), key(world(p)[k])))
            .is_none_or(|v| v.len() == 1)
    };
    // the roots of each system's ducts
    let mut seen = vec![[None::<usize>; 2]; ducts.len()];
    let mut queue = VecDeque::new();
    let mut systems: Vec<&str> = ducts.iter().map(|(_, p)| p.system.as_str()).collect();
    systems.sort_unstable();
    systems.dedup();
    for sys in systems {
        let ends: Vec<(usize, usize)> = (0..ducts.len())
            .filter(|&i| ducts[i].1.system == sys)
            .flat_map(|i| [(i, 0), (i, 1)])
            .filter(|&(i, k)| loose(i, k))
            .collect();
        let at_unit: Vec<(usize, usize)> = ends
            .iter()
            .copied()
            .filter(|&(i, k)| {
                let q = world(ducts[i].1)[k].truncate();
                units.iter().any(|u| u.distance(q) < 1.0)
            })
            .collect();
        let roots = if at_unit.is_empty() {
            ends.into_iter().filter(|&(i, k)| !terminal[i][k]).collect()
        } else {
            at_unit
        };
        for (i, k) in roots {
            seen[i][k] = Some(0);
            queue.push_back((i, k));
        }
    }
    // walk out: through a duct to its other end, then to the ducts there
    while let Some((i, k)) = queue.pop_front() {
        let d = seen[i][k].unwrap_or(0);
        let o = 1 - k;
        if seen[i][o].is_none() {
            seen[i][o] = Some(d + 1);
            let p = ducts[i].1;
            for &(j, m) in at
                .get(&(p.system.clone(), key(world(p)[o])))
                .into_iter()
                .flatten()
            {
                if seen[j][m].is_none() {
                    seen[j][m] = Some(d + 1);
                    queue.push_back((j, m));
                }
            }
        }
    }
    let mut out = HashMap::new();
    for (i, (e, p)) in ducts.iter().enumerate() {
        let (Some(a), Some(b)) = (seen[i][0], seen[i][1]) else {
            continue;
        };
        if a == b {
            continue;
        }
        // away from the root: from the start when the start is nearer
        let away = a < b;
        let outward = matches!(p.system.as_str(), "VZP" | "VZE")
            || (p.system_def().map(|s| s.kind) == Some(SystemKind::AirSupply) && p.system != "VZC");
        out.insert(e.id, away == outward);
    }
    out
}

/// A duct as a VZT drawing shows it (George 2026-11-25): its outline in
/// true size in its system's colour (the view colours the lines), washed
/// lightly in that colour, and where it is insulated the insulation's
/// outline round it, thin and dashed in the same colour.
pub fn duct_plan(
    model: &Model,
    e: &Element,
    p: &Pipe,
    flow: Option<bool>,
    mm: f64,
) -> Vec<PlanItem> {
    let rgb = color(p);
    let mut bare = p.clone();
    bare.insulation_mm = 0.0;
    let mut items: Vec<PlanItem> = pipe_plan(model, e, &bare, f64::INFINITY)
        .into_iter()
        .map(|it| match it {
            // a light wash: the outline tells the duct
            PlanItem::Shade { id, points, .. } => PlanItem::Tint {
                id,
                points,
                rgb: rgb.map(|c| c + (1.0 - c) * 0.6),
            },
            other => other,
        })
        .collect();
    if p.insulation_mm > 0.0 {
        for it in pipe_plan(model, e, p, f64::INFINITY) {
            if let PlanItem::Line { id, a, b, .. } = it {
                items.push(PlanItem::Curve {
                    id,
                    points: vec![a, b],
                    rgb,
                    dashed: true,
                });
            }
        }
    }
    // its axis, thin dash-dot, with an arrow the way the air runs
    // (George's sample VZT drawings)
    let (a, b) = (p.start.truncate(), p.end.truncate());
    if a.distance(b) > 1e-6 {
        let look = creabim_core::LineLook {
            pen: creabim_core::LinePen::Thin,
            kind: creabim_core::LineType::DashDot,
        };
        items.push(PlanItem::Line {
            id: e.id,
            pen: Pen::Drafted(look),
            a,
            b,
        });
        if let Some(fwd) = flow {
            let d = if fwd { b - a } else { a - b }.normalize();
            let len = 2.0 * mm;
            // on longer runs only: a short piece between fittings would
            // be all arrow
            if a.distance(b) > 4.0 * len {
                let tip = (a + b) / 2.0 + d * len / 2.0;
                let solid = creabim_core::LineLook {
                    pen: creabim_core::LinePen::Thin,
                    kind: creabim_core::LineType::Solid,
                };
                for side in [1.0, -1.0] {
                    items.push(PlanItem::Line {
                        id: e.id,
                        pen: Pen::Drafted(solid),
                        a: tip,
                        b: tip - d * len + d.perp() * side * len * 0.4,
                    });
                }
            }
        }
    }
    items
}

/// The pipe in a plan cut `cut` above its level: its outline in plan,
/// washed in its system's colour, in the lightest line where it is cut
/// or seen below, dashed above. Its ends are closed only where it meets no
/// other pipe: at a corner its sides mitre into the other's, a bent pipe
/// turns in its arc.
pub fn pipe_plan(model: &Model, e: &Element, p: &Pipe, cut: f64) -> Vec<PlanItem> {
    let mut items = Vec::new();
    let (a, b) = (p.start, p.end);
    let (w, _) = p.section_m();
    let half = w / 2.0 + p.insulation_mm / 1000.0;
    let (a2, b2) = (a.truncate(), b.truncate());
    let run = b2 - a2;
    let lo = a.z.min(b.z) - half;
    let hi = a.z.max(b.z) + half;
    // the lightest line, cut or seen below: a heavier one would blot a
    // pipe a few millimetres wide at a drawing's scale
    let pen = if hi <= cut || lo < cut {
        Pen::Fine
    } else {
        Pen::Hidden
    };
    let line = |items: &mut Vec<PlanItem>, a: DVec2, b: DVec2| {
        items.push(PlanItem::Line {
            id: e.id,
            pen,
            a,
            b,
        })
    };
    // a riser shows as its section: a square or a circle round its axis
    if run.length() < 1e-6 {
        // no smaller than a drawing still shows: a thin pipe's own
        // section is a dot at 1:50
        let r = half.max(RISER_MARK);
        let pts: Vec<DVec2> = (0..SIDES)
            .map(|i| {
                let t = i as f64 / SIDES as f64 * std::f64::consts::TAU;
                a2 + DVec2::new(t.cos(), t.sin()) * r
            })
            .collect();
        // the riser's stroke: out of the circle, up and to the right
        let d = DVec2::new(1.0, 1.0).normalize();
        line(&mut items, a2 + d * r, a2 + d * (r + 2.0 * RISER_MARK));
        items.push(PlanItem::Shade {
            id: e.id,
            points: pts.clone(),
            rgb: color(p),
        });
        for i in 0..pts.len() {
            line(&mut items, pts[i], pts[(i + 1) % pts.len()]);
        }
        return items;
    }
    // joints are found in the model's heights; the plan is level
    let z = model.level(p.level).map_or(0.0, |l| l.elevation);
    let flat = |q: DVec3| (q - DVec3::Z * z).truncate();
    let js = joints(model, p);
    let d = run.normalize();
    let n = d.perp() * half;
    // each end's two corners (right of the run, left of it), whether it
    // is closed, and the arc drawn from it
    let mut corners = [[a2 - n, a2 + n], [b2 - n, b2 + n]];
    let mut closed = [true, true];
    let mut arcs: Vec<Vec<DVec2>> = vec![];
    for (i, j) in js.iter().enumerate() {
        // out of the pipe, at this end
        let out = if i == 0 { -d } else { d };
        match j {
            Joint::Free => {}
            Joint::Node { .. } => closed[i] = false,
            Joint::Bend { at, arc, owner } => {
                let c = flat(*at);
                corners[i] = [c - n, c + n];
                closed[i] = false;
                if *owner {
                    arcs.push(arc.iter().map(|q| flat(*q)).collect());
                }
            }
            Joint::Elbow { to, .. } => {
                let to = to.truncate();
                // a riser there, or nearly back on itself: closed as it is
                let Some(to) = to.try_normalize() else {
                    continue;
                };
                let turn = out.perp_dot(to);
                let cos = out.dot(to);
                if cos < -0.94 {
                    continue;
                }
                // the sides meet the other pipe's on the line that halves
                // the corner: the inner side shorter, the outer longer
                let k = half * turn / (1.0 + cos) * if i == 0 { -1.0 } else { 1.0 };
                let node = if i == 0 { a2 } else { b2 };
                corners[i] = [node - n + out * k, node + n - out * k];
                closed[i] = false;
            }
        }
    }
    // narrower than the largest pipe at an end: a solid triangle there,
    // pointing into this pipe (STN EN ISO 6412)
    let red = creabim_services::fittings::reductions_of(model, p);
    for (i, big) in red.iter().enumerate() {
        let (Some(big), false) = (big, matches!(js[i], Joint::Bend { .. })) else {
            continue;
        };
        let (node, into) = if i == 0 { (a2, d) } else { (b2, -d) };
        let nb = d.perp() * (big / 2000.0 + p.insulation_mm / 1000.0);
        let tip = node + into * taper_len(*big, p.outer_mm).min(run.length() / 2.0);
        // the pipe's own corners are square again: the triangle covers them
        corners[i] = [node - n, node + n];
        closed[i] = false;
        items.push(PlanItem::Shade {
            id: e.id,
            points: vec![node - nb, tip - n, tip + n, node + nb],
            rgb: color(p),
        });
        line(&mut items, node - nb, tip - n);
        line(&mut items, node + nb, tip + n);
        line(&mut items, node - nb, node + nb);
    }
    // the fitting's socket on each fitted end: a band a little wider
    // than the pipe, in a darker wash
    if let Some((rs, ls)) = socket(p) {
        let ls = ls.min(run.length() / 2.0);
        let ns = d.perp() * rs;
        let c = color(p);
        let dark = [c[0] * 0.7, c[1] * 0.7, c[2] * 0.7];
        for (i, j) in js.iter().enumerate() {
            if !fitted(j) || red[i].is_some() {
                continue;
            }
            let (node, into) = if i == 0 { (a2, d) } else { (b2, -d) };
            let tip = node + into * ls;
            let pts = vec![node - ns, tip - ns, tip + ns, node + ns];
            items.push(PlanItem::Shade {
                id: e.id,
                points: pts.clone(),
                rgb: dark,
            });
            line(&mut items, pts[0], pts[1]);
            line(&mut items, pts[1], pts[2]);
            line(&mut items, pts[2], pts[3]);
        }
    }
    let [[ar, al], [br, bl]] = corners;
    items.push(PlanItem::Shade {
        id: e.id,
        points: vec![ar, br, bl, al],
        rgb: color(p),
    });
    line(&mut items, ar, br);
    line(&mut items, al, bl);
    if closed[0] {
        line(&mut items, ar, al);
    }
    if closed[1] {
        line(&mut items, br, bl);
    }
    for arc in arcs {
        // the arc's two sides, each point set off square to the way it runs
        let side = |s: f64| -> Vec<DVec2> {
            (0..arc.len())
                .map(|i| {
                    let (lo, hi) = (i.saturating_sub(1), (i + 1).min(arc.len() - 1));
                    let t = (arc[hi] - arc[lo]).normalize_or_zero();
                    arc[i] + t.perp() * half * s
                })
                .collect()
        };
        let (r, l) = (side(-1.0), side(1.0));
        let mut points = r.clone();
        points.extend(l.iter().rev());
        items.push(PlanItem::Shade {
            id: e.id,
            points,
            rgb: color(p),
        });
        for s in [&r, &l] {
            for q in s.windows(2) {
                line(&mut items, q[0], q[1]);
            }
        }
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::{ElementKind, Level, Transaction};

    fn model_with(p: impl FnOnce(creabim_core::ElementId) -> Pipe) -> (Model, Element) {
        let mut m = Model::new();
        let l = Element::new(
            "Ground",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let e = Element::new("Pipe", ElementKind::Pipe(p(l.id)));
        Transaction::new("pipe")
            .insert(l)
            .insert(e.clone())
            .apply(&mut m)
            .unwrap();
        (m, e)
    }

    fn pipe(level: creabim_core::ElementId, height_mm: Option<f64>) -> Pipe {
        Pipe {
            level,
            system: "SV".into(),
            start: DVec3::new(0.0, 0.0, 0.3),
            end: DVec3::new(2.0, 0.0, 0.3),
            pipe_type: "HERZ".into(),
            nominal_mm: 16.0,
            outer_mm: 20.0,
            inner_mm: 16.0,
            height_mm,
            insulation_mm: 0.0,
            insulation_set: false,
            mark: None,
        }
    }

    #[test]
    fn a_pipe_is_a_closed_cylinder_in_its_systems_colour() {
        let (m, e) = model_with(|l| pipe(l, None));
        let ElementKind::Pipe(p) = &e.kind else {
            unreachable!()
        };
        let mesh = pipe_mesh(&m, p);
        assert!(mesh.is_closed());
        let (lo, hi) = mesh.bounds().unwrap();
        assert!((hi.z - lo.z - 0.02).abs() < 1e-3 && (hi.x - 2.0).abs() < 1e-6);
        assert!((mesh.volume() - std::f64::consts::PI * 0.01 * 0.01 * 2.0).abs() < 2e-5);
        assert_eq!(
            color(p),
            [
                0x2f as f32 / 255.0,
                0x6f as f32 / 255.0,
                0xde as f32 / 255.0
            ]
        );
    }

    #[test]
    fn a_duct_is_a_box() {
        let (m, e) = model_with(|l| {
            let mut p = pipe(l, Some(200.0));
            p.nominal_mm = 400.0;
            p
        });
        let ElementKind::Pipe(p) = &e.kind else {
            unreachable!()
        };
        let mesh = pipe_mesh(&m, p);
        assert!(mesh.is_closed());
        assert!((mesh.volume() - 0.4 * 0.2 * 2.0).abs() < 1e-6);
    }

    /// Two pipes of `kind` meeting at (2, 0) at a right angle, and the
    /// first of them.
    fn corner(kind: &str) -> (Model, Element) {
        let (mut m, e) = model_with(|l| {
            let mut p = pipe(l, None);
            p.pipe_type = kind.into();
            p
        });
        let ElementKind::Pipe(p) = &e.kind else {
            unreachable!()
        };
        let mut q = p.clone();
        q.start = p.end;
        q.end = DVec3::new(2.0, 3.0, 0.3);
        Transaction::new("second")
            .insert(Element::new("Pipe", ElementKind::Pipe(q)))
            .apply(&mut m)
            .unwrap();
        (m, e)
    }

    #[test]
    fn a_services_drawing_marks_its_sanitary_fixtures() {
        use creabim_core::viewmodes::DisplayMode;
        use creabim_core::{Document, ViewKind};
        let mut doc = Document::from_model(creabim_core::sample::furnished_house());
        let rows = creabim_services::fixturespec::legend(doc.model(), None);
        assert!(!rows.is_empty());
        let marks = |m: &Model, view| -> Vec<String> {
            crate::section::view_drawing(m, view)
                .unwrap()
                .items
                .into_iter()
                .filter_map(|i| match i {
                    PlanItem::Annotation { id, item }
                        if matches!(m.get(id).map(|e| &e.kind), Some(ElementKind::Object(_)))
                            && item.look.rgb == Some(LABEL_GREEN) =>
                    {
                        Some(item.text.clone())
                    }
                    _ => None,
                })
                .collect()
        };
        let views: Vec<_> = doc
            .model()
            .iter()
            .filter(|e| {
                matches!(&e.kind, ElementKind::View(v) if matches!(v.kind, ViewKind::Plan { .. }))
            })
            .map(|e| e.id)
            .collect();
        assert!(views.iter().all(|v| marks(doc.model(), *v).is_empty()));
        for v in &views {
            doc.edit(*v, "mode", |e| {
                if let ElementKind::View(v) = &mut e.kind {
                    DisplayMode::Services.apply(v);
                }
            })
            .unwrap();
        }
        let all: Vec<String> = views.iter().flat_map(|v| marks(doc.model(), *v)).collect();
        // every mark written is one of the legend's
        assert!(!all.is_empty());
        assert!(all.iter().all(|m| rows.iter().any(|r| r.spec.mark == *m)));
    }

    #[test]
    fn pipes_side_by_side_share_a_label() {
        use crate::annotation::Shape;
        let m = creabim_core::sample::sample_house();
        let level = m.levels().next().unwrap().0.id;
        let run = |system: &str, y: f64, outer: f64, inner: f64| {
            let mut p = pipe(level, None);
            p.system = system.into();
            p.start = DVec3::new(1.0, y, 0.0);
            p.end = DVec3::new(4.0, y, 0.0);
            (p.outer_mm, p.inner_mm, p.nominal_mm) = (outer, inner, outer);
            Element::new("Pipe", ElementKind::Pipe(p))
        };
        let mut riser = pipe(level, None);
        riser.system = "K".into();
        riser.nominal_mm = 110.0;
        riser.start = DVec3::new(6.0, 3.0, 0.0);
        riser.end = DVec3::new(6.0, 3.0, 2.5);
        // a rise within its storey is no stack: it has no mark
        {
            let mut m = m.clone();
            let e = Element::new("Rise", ElementKind::Pipe(riser.clone()));
            creabim_core::Transaction::new("t")
                .insert(e.clone())
                .apply(&mut m)
                .unwrap();
            let labels = pipe_labels(&m, &[&e], 0.05);
            assert!(!labels.iter().any(|l| matches!(l, PlanItem::Tag { .. })));
        }
        // through the ceiling into the storey above it is one
        riser.end = DVec3::new(6.0, 3.0, 4.0);
        let es = [
            run("SV", 2.0, 25.0, 18.0),
            run("TV", 2.1, 25.0, 18.0),
            run("C", 2.2, 20.0, 14.4),
            Element::new("Riser", ElementKind::Pipe(riser)),
        ];
        let ElementKind::Pipe(p) = &es[2].kind else {
            unreachable!()
        };
        assert_eq!(pipe_text(p), "C 20x2.8");
        let mut doc = creabim_core::Document::from_model(m);
        let mut tx = creabim_core::Transaction::new("pipes");
        for e in &es {
            tx = tx.insert(e.clone());
        }
        doc.commit(tx).unwrap();
        let m = doc.model().clone();
        let refs: Vec<&Element> = es.iter().collect();
        let labels = pipe_labels(&m, &refs, 0.05);
        let read = |i: &PlanItem| match i {
            PlanItem::Annotation { item, .. } => (item.text.clone(), item.shape.clone()),
            _ => unreachable!(),
        };
        assert_eq!(labels.len(), 3);
        // the riser: its stack's mark in a circle, and its size
        assert!(matches!(&labels[0], PlanItem::Tag { text, round: true, .. } if text == "K1"));
        let (text, _) = read(&labels[1]);
        assert_eq!(text, "DN110");
        // one label for the three, the one farthest up first, and where
        // they run
        let (text, shape) = read(&labels[2]);
        assert_eq!(text, "C 20x2.8\nTV 25x3.5\nSV 25x3.5\nin the floor");
        let Shape::Along { at, angle } = shape else {
            unreachable!()
        };
        assert_eq!(angle, 0.0);
        assert!((at.x - 2.5).abs() < 1e-9 && at.y > 2.2);
        // dragged away: the label goes with the drag, no leader
        let mut moved = es[0].clone();
        if let ElementKind::Pipe(p) = &mut moved.kind {
            p.mark = Some(DVec2::new(0.5, 1.0));
        }
        let lone = pipe_labels(&m, &[&moved], 0.05);
        let (_, shape) = read(lone.last().unwrap());
        let Shape::Along { at: now, .. } = shape else {
            unreachable!()
        };
        let mut home = es[0].clone();
        if let ElementKind::Pipe(p) = &mut home.kind {
            p.mark = None;
        }
        let first = pipe_labels(&m, &[&home], 0.05);
        let (_, shape) = read(first.last().unwrap());
        let Shape::Along { at: was, .. } = shape else {
            unreachable!()
        };
        assert!((now - was - DVec2::new(0.5, 1.0)).length() < 1e-9);
        assert!(lone.len() == 1 && first.len() == 1);
    }

    #[test]
    fn a_trades_drawing_leaves_the_other_trades_pipes_out() {
        use creabim_core::viewmodes::{DisplayMode, Trade};
        use creabim_core::{Document, Transaction, ViewKind};
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let (view, level) = doc
            .model()
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::View(v) => match v.kind {
                    ViewKind::Plan { level, .. } => Some((e.id, level)),
                    _ => None,
                },
                _ => None,
            })
            .unwrap();
        let mut water = pipe(level, None);
        water.start = DVec3::new(1.0, 1.0, 0.3);
        water.end = DVec3::new(3.0, 1.0, 0.3);
        let mut heat = water.clone();
        heat.start.y = 2.0;
        heat.end.y = 2.0;
        heat.system = creabim_core::mep::SYSTEMS
            .iter()
            .find(|s| s.kind == creabim_core::mep::SystemKind::HeatingSupply)
            .unwrap()
            .code
            .to_string();
        assert!(Trade::Plumbing.shows(water.kind()) && Trade::Heating.shows(heat.kind()));
        let (water, heat) = (
            Element::new("Water", ElementKind::Pipe(water)),
            Element::new("Heating", ElementKind::Pipe(heat)),
        );
        let (w, h) = (water.id, heat.id);
        doc.commit(Transaction::new("pipes").insert(water).insert(heat))
            .unwrap();
        let shown = |doc: &Document, id| {
            crate::section::view_drawing(doc.model(), view)
                .unwrap()
                .items
                .iter()
                .any(|i| i.id() == id)
        };
        for (trade, water_on, heat_on) in [
            (None, true, true),
            (Some(Trade::Plumbing), true, false),
            (Some(Trade::Heating), false, true),
        ] {
            doc.edit(view, "mode", |e| {
                if let ElementKind::View(v) = &mut e.kind {
                    DisplayMode::Services.apply(v);
                    v.trade = trade;
                }
            })
            .unwrap();
            assert_eq!(shown(&doc, w), water_on, "{trade:?}");
            assert_eq!(shown(&doc, h), heat_on, "{trade:?}");
        }
    }

    /// A services drawing is a scheme (a line a pipe, a valve's symbol
    /// the same size on paper at every scale) or shows the pipes and
    /// valves as they lie in space.
    #[test]
    fn a_services_drawing_is_a_scheme_or_in_space() {
        use creabim_core::viewmodes::DisplayMode;
        use creabim_core::{Document, Transaction, ViewKind};
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let (view, level) = doc
            .model()
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::View(v) => match v.kind {
                    ViewKind::Plan { level, .. } => Some((e.id, level)),
                    _ => None,
                },
                _ => None,
            })
            .unwrap();
        let mut run = pipe(level, None);
        run.start = DVec3::new(1.0, 1.0, 0.3);
        run.end = DVec3::new(3.0, 1.0, 0.3);
        let run = Element::new("Pipe", ElementKind::Pipe(run));
        let valve = Element::new(
            "Valve",
            ElementKind::Valve(creabim_core::mep::Valve {
                pipe: run.id,
                t: 0.5,
                kind: "uzaver".into(),
                area_m2: None,
                coef: None,
                hydrant: None,
                flow_lmin: None,
                flow_m3h: None,
                power_kw: None,
                gas_cat: None,
                product: None,
                preset: None,
                mark: None,
                under_ceiling: false,
            }),
        );
        let (run_id, valve_id) = (run.id, valve.id);
        doc.commit(Transaction::new("pipes").insert(run).insert(valve))
            .unwrap();
        let set = |doc: &mut Document, spatial: bool, scale: f64| {
            doc.edit(view, "mode", |e| {
                if let ElementKind::View(v) = &mut e.kind {
                    DisplayMode::Services.apply(v);
                    v.spatial = spatial;
                    v.scale = scale;
                }
            })
            .unwrap();
        };
        let of = |m: &Model, id| -> Vec<PlanItem> {
            crate::section::view_drawing(m, view)
                .unwrap()
                .items
                .into_iter()
                .filter(|i| i.id() == id)
                .collect()
        };
        let width = |items: &[PlanItem]| {
            let xs: Vec<f64> = items.iter().flat_map(|i| i.extent()).map(|p| p.x).collect();
            xs.iter().cloned().fold(f64::MIN, f64::max)
                - xs.iter().cloned().fold(f64::MAX, f64::min)
        };
        // a scheme: the pipe one line; the valve's symbol twice as large
        // in the model at 1:100 as at 1:50
        set(&mut doc, false, 50.0);
        assert_eq!(lines(&of(doc.model(), run_id)).len(), 1);
        let at_50 = width(&of(doc.model(), valve_id));
        set(&mut doc, false, 100.0);
        let at_100 = width(&of(doc.model(), valve_id));
        assert!((at_100 / at_50 - 2.0).abs() < 0.05, "{at_50} {at_100}");
        // in space: the pipe's two sides, the valve's body on it
        set(&mut doc, true, 50.0);
        assert!(of(doc.model(), run_id).len() > 2);
        let body = of(doc.model(), valve_id);
        assert!(body.iter().any(|i| matches!(i, PlanItem::Polygon { .. })));
        assert!(width(&body) < 0.2);
    }

    #[test]
    fn a_services_drawing_shows_a_pipe_as_a_line_of_its_system() {
        use creabim_core::viewmodes::DisplayMode;
        use creabim_core::{Document, Transaction, ViewKind};
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let (view, level) = doc
            .model()
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::View(v) => match v.kind {
                    ViewKind::Plan { level, .. } => Some((e.id, level)),
                    _ => None,
                },
                _ => None,
            })
            .unwrap();
        let mut run = pipe(level, None);
        run.start = DVec3::new(1.0, 1.0, 0.3);
        run.end = DVec3::new(3.0, 1.0, 0.3);
        let mut riser = pipe(level, None);
        riser.start = DVec3::new(4.0, 2.0, 0.0);
        riser.end = DVec3::new(4.0, 2.0, 4.0);
        let (run, riser) = (
            Element::new("Pipe", ElementKind::Pipe(run)),
            Element::new("Riser", ElementKind::Pipe(riser)),
        );
        let (run_id, riser_id) = (run.id, riser.id);
        doc.commit(Transaction::new("pipes").insert(run).insert(riser))
            .unwrap();
        let of = |m: &Model, id| -> Vec<PlanItem> {
            crate::section::view_drawing(m, view)
                .unwrap()
                .items
                .into_iter()
                .filter(|i| i.id() == id)
                .collect()
        };
        // the architect's drawing leaves the trades' pipes out
        assert!(of(doc.model(), run_id).is_empty());
        doc.edit(view, "mode", |e| {
            if let ElementKind::View(v) = &mut e.kind {
                DisplayMode::Services.apply(v);
            }
        })
        .unwrap();
        let m = doc.model();
        let items = of(m, run_id);
        assert_eq!(
            lines(&items),
            vec![(DVec2::new(1.0, 1.0), DVec2::new(3.0, 1.0))]
        );
        // the line and its label
        assert_eq!(items.len(), 2);
        assert!(matches!(items[1], PlanItem::Annotation { .. }));
        // a riser is its small circle, with its stack's mark and its
        // size beside it
        assert!(matches!(
            of(m, riser_id)[..],
            [
                PlanItem::Polygon { .. },
                PlanItem::Tag { .. },
                PlanItem::Annotation { .. }
            ]
        ));
        // drawn in cold water's blue, dash and dot
        let ElementKind::View(v) = &m.get(view).unwrap().kind else {
            unreachable!()
        };
        let looks = crate::looks::Looks::for_view(m, view, v);
        let (stroke, rgb, dashes) = looks.stroke(run_id, Pen::Projection);
        assert_eq!(stroke.width, crate::looks::PIPE_LINE);
        let ElementKind::Pipe(p) = &m.get(run_id).unwrap().kind else {
            unreachable!()
        };
        let (rgb, want) = (rgb.unwrap(), color(p));
        assert!((0..3).all(|i| (rgb[i] - want[i]).abs() < 1e-5), "{rgb:?}");
        assert_eq!(dashes, Some(vec![5.0, 1.2, 0.0, 1.2]));
    }

    fn lines(items: &[PlanItem]) -> Vec<(DVec2, DVec2)> {
        items
            .iter()
            .filter_map(|i| match i {
                PlanItem::Line { a, b, .. } => Some((*a, *b)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn rigid_pipes_mitre_at_a_corner() {
        let (m, e) = corner("PPR");
        let ElementKind::Pipe(p) = &e.kind else {
            unreachable!()
        };
        let js = joints(&m, p);
        assert_eq!(js[0], Joint::Free);
        assert!(matches!(js[1], Joint::Elbow { .. }));
        // closed at its free start only; at the corner the outer side
        // runs on by half the width and the inner stops as short
        let l = lines(&pipe_plan(&m, &e, p, 1.2));
        // the elbow's socket on the fitted end, three lines, first
        assert_eq!(l.len(), 6);
        let ends: Vec<DVec2> = l.iter().skip(3).take(2).map(|(_, b)| *b).collect();
        assert!(
            (ends[0] - DVec2::new(2.01, -0.01)).length() < 1e-9,
            "{ends:?}"
        );
        assert!(
            (ends[1] - DVec2::new(1.99, 0.01)).length() < 1e-9,
            "{ends:?}"
        );
        // in 3D one of the two has the elbow's ball
        let other = m
            .iter()
            .find_map(|x| match &x.kind {
                ElementKind::Pipe(q) if x.id != e.id => Some(q.clone()),
                _ => None,
            })
            .unwrap();
        let balls = [p, &other]
            .iter()
            .filter(|q| pipe_mesh(&m, q).positions.len() > 200)
            .count();
        assert_eq!(balls, 1);
    }

    #[test]
    fn a_pipe_from_a_coil_turns_in_an_arc() {
        // multilayer pipe, 20 mm outside: bent at 5 d = 100 mm
        let (m, e) = corner("HERZ");
        let ElementKind::Pipe(p) = &e.kind else {
            unreachable!()
        };
        let js = joints(&m, p);
        let Joint::Bend { at, arc, .. } = &js[1] else {
            panic!("{js:?}")
        };
        assert!((*at - DVec3::new(1.9, 0.0, 0.3)).length() < 1e-9);
        assert!((*arc.last().unwrap() - DVec3::new(2.0, 0.1, 0.3)).length() < 1e-9);
        // every point of the arc is 100 mm from its centre
        let c = DVec3::new(1.9, 0.1, 0.3);
        assert!(arc.iter().all(|q| (q.distance(c) - 0.1).abs() < 1e-9));
        // the straight part stops where the arc begins, and is not closed there
        let l = lines(&pipe_plan(&m, &e, p, 1.2));
        assert!((l[0].1.x - 1.9).abs() < 1e-9 && (l[1].1.x - 1.9).abs() < 1e-9);
        // too short an arm for the arc: a fitting, as rigid pipe
        let (mut m, e) = corner("HERZ");
        let other = m
            .iter()
            .find(|x| x.id != e.id && matches!(x.kind, ElementKind::Pipe(_)))
            .unwrap()
            .clone();
        let mut short = other.clone();
        if let ElementKind::Pipe(q) = &mut short.kind {
            q.end = DVec3::new(2.0, 0.15, 0.3);
        }
        Transaction::new("short")
            .replace(other, short)
            .apply(&mut m)
            .unwrap();
        assert!(matches!(joints(&m, p)[1], Joint::Elbow { .. }));
    }

    #[test]
    fn a_tee_leaves_the_ends_open() {
        let (mut m, e) = corner("PPR");
        let ElementKind::Pipe(p) = &e.kind else {
            unreachable!()
        };
        let mut on = p.clone();
        on.start = p.end;
        on.end = DVec3::new(4.0, 0.0, 0.3);
        Transaction::new("on")
            .insert(Element::new("Pipe", ElementKind::Pipe(on)))
            .apply(&mut m)
            .unwrap();
        assert!(matches!(joints(&m, p)[1], Joint::Node { .. }));
        // the tee's socket, two sides and the cap of the free start
        assert_eq!(lines(&pipe_plan(&m, &e, p, 1.2)).len(), 6);
    }

    #[test]
    fn a_pipe_is_cut_or_seen_in_plan() {
        let (m, e) = model_with(|l| pipe(l, None));
        let ElementKind::Pipe(p) = &e.kind else {
            unreachable!()
        };
        let pen = |cut| match pipe_plan(&m, &e, p, cut)[1] {
            PlanItem::Line { pen, .. } => pen,
            _ => unreachable!(),
        };
        assert_eq!(pen(0.3), Pen::Fine);
        assert_eq!(pen(1.2), Pen::Fine);
        assert_eq!(pen(0.1), Pen::Hidden);
        assert!(matches!(
            pipe_plan(&m, &e, p, 1.2)[0],
            PlanItem::Shade { .. }
        ));
    }
}
