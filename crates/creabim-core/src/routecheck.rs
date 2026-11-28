//! Critical places on the pipe and duct routes drawn, beyond the valves
//! [`crate::autovalves`] proposes:
//!
//! - a pipe or duct running through a beam: a system lintel, a ring beam
//!   or any other beam. A level pipe can be led under it
//!   ([`bypass`]): down in front of it, along under it and up again
//!   behind it, as one draws a heating pipe under the ceiling past a
//!   door's lintel; the drop and the rise are no stack (see
//!   `geom::mep::stack_mark`), and the four elbows and the extra length go
//!   into the calculations as any others drawn;
//! - a gas pipe nearer another pipe than 20 mm, surface to surface, where
//!   they run side by side or cross (TPH 03 2012 F.5.7);
//! - a pipe or duct of another trade than gas passing into another fire
//!   zone: listed to be fire-stopped (or, on a duct, given a fire damper)
//!   as the designer decides; gas gets its fire stopping proposed with
//!   its valves.

use crate::members::{elevation, Beam};
use crate::mep::{Pipe, SystemKind};
use crate::{Element, ElementId, ElementKind, Model, Transaction};
use glam::{DVec2, DVec3};

/// The least gap between a gas pipe and another pipe, surface to
/// surface, m (TPH 03 2012 F.5.7).
pub const GAS_GAP: f64 = 0.02;

/// What is wrong where.
#[derive(Clone, Debug, PartialEq)]
pub enum Problem {
    /// The pipe runs through a beam; `lintel` when it is a system lintel.
    Beam { beam: ElementId, lintel: bool },
    /// A gas pipe `gap` metres from another pipe, surface to surface.
    GasGap { other: ElementId, gap: f64 },
    /// It passes from one fire zone into another.
    FireZone { from: String, to: String },
}

/// A critical place on a route.
#[derive(Clone, Debug, PartialEq)]
pub struct Clash {
    pub pipe: ElementId,
    pub problem: Problem,
    /// Where, m in the model.
    pub at: DVec3,
}

/// A pipe's ends in the model, m.
fn ends(model: &Model, p: &Pipe) -> (DVec3, DVec3) {
    let z = DVec3::Z * elevation(model, p.level);
    (p.start + z, p.end + z)
}

/// How far its outside reaches from its axis, m: half its diameter (or a
/// duct's height) and its insulation.
pub fn reach(p: &Pipe) -> f64 {
    let d = p.height_mm.map_or(p.outer_mm, |h| h.max(p.outer_mm));
    d / 2000.0 + p.insulation() / 1000.0
}

/// The share of the plan segment `a`–`b` inside the convex polygon
/// `poly` (any winding): (from, to), or none.
fn clip(a: DVec2, b: DVec2, poly: &[DVec2]) -> Option<(f64, f64)> {
    let d = b - a;
    let area: f64 = (0..poly.len())
        .map(|i| poly[i].perp_dot(poly[(i + 1) % poly.len()]))
        .sum();
    let sign = if area >= 0.0 { 1.0 } else { -1.0 };
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    for i in 0..poly.len() {
        let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
        let e = q - p;
        // inside is left of each edge (counter-clockwise)
        let num = sign * e.perp_dot(a - p);
        let den = sign * e.perp_dot(d);
        if den.abs() < 1e-12 {
            if num < 0.0 {
                return None;
            }
            continue;
        }
        let t = -num / den;
        if den > 0.0 {
            t0 = t0.max(t);
        } else {
            t1 = t1.min(t);
        }
        if t0 > t1 {
            return None;
        }
    }
    Some((t0, t1))
}

/// A beam that stands plumb and level: its plan outline and its
/// underside and top, m in the model. Sloped rafters are not checked.
fn beam_box(model: &Model, b: &Beam) -> Option<(Vec<DVec2>, f64, f64)> {
    if b.rise != [0.0, 0.0] || b.up.is_some() || b.length() < 1e-6 {
        return None;
    }
    let top = b.top(model);
    Some((b.outline(), top - b.depth, top))
}

/// Where pipe `p` runs through a beam's box, grown by the pipe's reach:
/// the share along the pipe (from, to).
pub fn through_beam(model: &Model, p: &Pipe, b: &Beam) -> Option<(f64, f64)> {
    let (lo, hi, poly) = {
        let (poly, lo, hi) = beam_box(model, b)?;
        (lo, hi, poly)
    };
    let (a, e) = ends(model, p);
    let r = reach(p);
    let (t0, t1) = if a.truncate().distance(e.truncate()) < 1e-6 {
        // upright: through when its axis stands in the outline
        if !crate::polygon::contains(&poly, a.truncate()) {
            return None;
        }
        (0.0, 1.0)
    } else {
        clip(a.truncate(), e.truncate(), &poly)?
    };
    let (za, zb) = (a.z + (e.z - a.z) * t0, a.z + (e.z - a.z) * t1);
    let (plo, phi) = (za.min(zb) - r, za.max(zb) + r);
    (plo < hi - 1e-6 && phi > lo + 1e-6).then_some((t0, t1))
}

/// The shortest distance between the segments `p1`–`q1` and `p2`–`q2`.
fn seg_seg(p1: DVec3, q1: DVec3, p2: DVec3, q2: DVec3) -> f64 {
    let (d1, d2, r) = (q1 - p1, q2 - p2, p1 - p2);
    let (a, e, f) = (d1.length_squared(), d2.length_squared(), d2.dot(r));
    let (s, t);
    if a <= 1e-12 && e <= 1e-12 {
        return r.length();
    }
    if a <= 1e-12 {
        s = 0.0;
        t = (f / e).clamp(0.0, 1.0);
    } else {
        let c = d1.dot(r);
        if e <= 1e-12 {
            t = 0.0;
            s = (-c / a).clamp(0.0, 1.0);
        } else {
            let b = d1.dot(d2);
            let den = a * e - b * b;
            let mut s0 = if den > 1e-12 {
                ((b * f - c * e) / den).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut t0 = (b * s0 + f) / e;
            if t0 < 0.0 {
                t0 = 0.0;
                s0 = (-c / a).clamp(0.0, 1.0);
            } else if t0 > 1.0 {
                t0 = 1.0;
                s0 = ((b - c) / a).clamp(0.0, 1.0);
            }
            s = s0;
            t = t0;
        }
    }
    ((p1 + d1 * s) - (p2 + d2 * t)).length()
}

/// The rooms of the model: id, level, outline, fire zone.
fn rooms(model: &Model) -> Vec<(ElementId, ElementId, Vec<DVec2>, String)> {
    model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Room(r) => Some((
                e.id,
                r.level,
                crate::room::outline(model, e.id)?,
                r.fire_zone.clone(),
            )),
            _ => None,
        })
        .collect()
}

/// Every critical place on the routes drawn.
pub fn check(model: &Model) -> Vec<Clash> {
    let pipes: Vec<(ElementId, &Pipe)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Pipe(p) if p.length() > 0.005 => Some((e.id, p)),
            _ => None,
        })
        .collect();
    let beams: Vec<(ElementId, &Beam)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Beam(b) => Some((e.id, b)),
            _ => None,
        })
        .collect();
    let mut out = vec![];
    // 1) through beams and lintels
    for (id, p) in &pipes {
        let (a, b) = ends(model, p);
        for (bid, beam) in &beams {
            if let Some((t0, t1)) = through_beam(model, p, beam) {
                out.push(Clash {
                    pipe: *id,
                    problem: Problem::Beam {
                        beam: *bid,
                        lintel: beam.lintel.is_some(),
                    },
                    at: a.lerp(b, (t0 + t1) / 2.0),
                });
            }
        }
    }
    // 2) gas nearer other pipes than 20 mm
    for (id, p) in pipes.iter().filter(|(_, p)| p.kind() == SystemKind::Gas) {
        let (a, b) = ends(model, p);
        for (oid, o) in pipes.iter().filter(|(_, o)| o.kind() != SystemKind::Gas) {
            let (c, d) = ends(model, o);
            let gap = seg_seg(a, b, c, d) - reach(p) - reach(o);
            if gap < GAS_GAP - 1e-6 {
                out.push(Clash {
                    pipe: *id,
                    problem: Problem::GasGap { other: *oid, gap },
                    at: a.lerp(b, 0.5),
                });
            }
        }
    }
    // 3) into another fire zone (gas takes its fire stopping with its
    // valves)
    let rooms = rooms(model);
    let zone_at = |level: ElementId, p: DVec2| {
        rooms
            .iter()
            .find(|r| r.1 == level && crate::polygon::contains(&r.2, p))
            .map(|r| r.3.clone())
            .filter(|z| !z.is_empty())
    };
    let walls: Vec<&crate::Wall> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Wall(w) => Some(w),
            _ => None,
        })
        .collect();
    let levels: Vec<(ElementId, f64)> = {
        let mut v: Vec<(ElementId, f64)> =
            model.levels().map(|(e, l)| (e.id, l.elevation)).collect();
        v.sort_by(|a, b| a.1.total_cmp(&b.1));
        v
    };
    for (id, p) in pipes.iter().filter(|(_, p)| p.kind() != SystemKind::Gas) {
        let (a, b) = ends(model, p);
        for w in &walls {
            let Some((f, n)) = crate::autovalves::wall_crossing(model, w, a, b) else {
                continue;
            };
            let at = a.truncate().lerp(b.truncate(), f);
            let off = n * (w.thickness / 2.0 + 0.05);
            if let (Some(x), Some(y)) = (zone_at(w.level, at + off), zone_at(w.level, at - off)) {
                if x != y {
                    out.push(Clash {
                        pipe: *id,
                        problem: Problem::FireZone { from: x, to: y },
                        at: a.lerp(b, f),
                    });
                }
            }
        }
        for k in 1..levels.len() {
            let z = levels[k].1;
            if a.z.min(b.z) < z - 0.01 && a.z.max(b.z) > z + 0.01 {
                let f = (z - a.z) / (b.z - a.z);
                let at = a.truncate().lerp(b.truncate(), f);
                if let (Some(x), Some(y)) = (zone_at(levels[k - 1].0, at), zone_at(levels[k].0, at))
                {
                    if x != y {
                        out.push(Clash {
                            pipe: *id,
                            problem: Problem::FireZone { from: x, to: y },
                            at: a.lerp(b, f),
                        });
                    }
                }
            }
        }
    }
    out
}

/// A pipe's line to a new piece: from `a` to `b` (m above its level).
fn piece(p: &Pipe, a: DVec3, b: DVec3) -> Pipe {
    let mut q = p.clone();
    q.start = a;
    q.end = b;
    q.mark = None;
    q
}

/// Lead a level pipe under the beam it runs through: down in front of
/// it, along under it with `gap` metres of air under the beam, and up
/// behind it, so the drop and the rise stand `gap` off the beam's faces.
/// The pipe keeps its ends (and so its joints); its valves stay where
/// they are along the route. None when the pipe is not level or does not
/// run through the beam.
pub fn bypass(model: &Model, pipe: ElementId, beam: ElementId, gap: f64) -> Option<Transaction> {
    let e = model.get(pipe)?;
    let ElementKind::Pipe(p) = &e.kind else {
        return None;
    };
    let ElementKind::Beam(b) = &model.get(beam)?.kind else {
        return None;
    };
    if (p.end.z - p.start.z).abs() > 0.02 {
        return None;
    }
    let (t0, t1) = through_beam(model, p, b)?;
    let len = p.start.truncate().distance(p.end.truncate());
    if len < 1e-6 {
        return None;
    }
    let (_, under, _) = beam_box(model, b)?;
    let r = reach(p);
    let back = (r + gap) / len;
    let (ta, tb) = ((t0 - back).max(0.0), (t1 + back).min(1.0));
    let z = under - elevation(model, p.level) - gap - r;
    if z >= p.start.z.min(p.end.z) {
        return None;
    }
    let at = |t: f64| p.start.lerp(p.end, t);
    let (a, b2) = (at(ta), at(tb));
    let (a_low, b_low) = (DVec3::new(a.x, a.y, z), DVec3::new(b2.x, b2.y, z));
    let mut tx = Transaction::new("Lead pipe under beam");
    // the first piece stays the pipe itself
    let mut first = e.clone();
    if let ElementKind::Pipe(q) = &mut first.kind {
        q.end = a;
    }
    tx = tx.replace(e.clone(), first);
    let new = |name: &str, q: Pipe| Element::new(name, ElementKind::Pipe(q));
    let down = new(&e.name, piece(p, a, a_low));
    let along = new(&e.name, piece(p, a_low, b_low));
    let up = new(&e.name, piece(p, b_low, b2));
    let last = (tb < 1.0 - 1e-9).then(|| new(&e.name, piece(p, b2, p.end)));
    // the valves stay where they were along the route
    let mut moves = vec![];
    for v in model.iter() {
        let ElementKind::Valve(val) = &v.kind else {
            continue;
        };
        if val.pipe != pipe {
            continue;
        }
        let mut moved = v.clone();
        let ElementKind::Valve(m) = &mut moved.kind else {
            continue;
        };
        if val.t <= ta {
            m.t = if ta > 0.0 { val.t / ta } else { 0.0 };
        } else if val.t >= tb {
            match &last {
                Some(l) => {
                    m.pipe = l.id;
                    m.t = (val.t - tb) / (1.0 - tb);
                }
                None => {
                    m.pipe = up.id;
                    m.t = 1.0;
                }
            }
        } else {
            m.pipe = along.id;
            m.t = (val.t - ta) / (tb - ta);
        }
        moves.push((v.clone(), moved));
    }
    tx = tx.insert(down).insert(along).insert(up);
    if let Some(l) = last {
        tx = tx.insert(l);
    }
    for (before, after) in moves {
        tx = tx.replace(before, after);
    }
    Some(tx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;

    fn heating(level: ElementId, a: DVec3, b: DVec3) -> Element {
        Element::new(
            "UKP",
            ElementKind::Pipe(Pipe {
                level,
                system: "UKP".into(),
                start: a,
                end: b,
                pipe_type: "HERZ".into(),
                nominal_mm: 16.0,
                outer_mm: 16.0,
                inner_mm: 12.0,
                height_mm: None,
                insulation_mm: 0.0,
                insulation_set: true,
                mark: None,
            }),
        )
    }

    #[test]
    fn a_pipe_under_the_ceiling_through_a_lintel_is_found_and_led_under_it() {
        let mut doc = Document::from_model(crate::sample::sample_house());
        let level = doc
            .model()
            .levels()
            .min_by(|a, b| a.1.elevation.total_cmp(&b.1.elevation))
            .unwrap()
            .0
            .id;
        let top = elevation(doc.model(), level) + crate::members::storey_height(doc.model(), level);
        // a beam 0.25 deep at the storey's top across y = 5
        let mut beam = Beam::new(level, DVec2::new(2.0, 5.0), DVec2::new(4.0, 5.0));
        beam.width = 0.3;
        beam.depth = 0.25;
        let bid = Element::new("beam", ElementKind::Beam(beam.clone()));
        let beam_id = bid.id;
        let z = top - elevation(doc.model(), level) - 0.1;
        let pipe = heating(level, DVec3::new(3.0, 3.0, z), DVec3::new(3.0, 7.0, z));
        let pid = pipe.id;
        let v = Element::new(
            "v",
            ElementKind::Valve(crate::mep::Valve::new(pid, 0.9, "uzaver")),
        );
        let vid = v.id;
        doc.commit(Transaction::new("x").insert(bid).insert(pipe).insert(v))
            .unwrap();
        let found = check(doc.model());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            matches!(found[0].problem, Problem::Beam { beam, lintel: false } if beam == beam_id)
        );
        let tx = bypass(doc.model(), pid, beam_id, 0.02).unwrap();
        doc.commit(tx).unwrap();
        // no longer through it, five pieces end to end, the valve on the
        // last at the same place
        assert!(check(doc.model()).is_empty(), "{:?}", check(doc.model()));
        let pieces: Vec<&Pipe> = doc
            .model()
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Pipe(p) => Some(p),
                _ => None,
            })
            .collect();
        assert_eq!(pieces.len(), 5);
        let low = pieces
            .iter()
            .map(|p| p.start.z.min(p.end.z))
            .fold(f64::MAX, f64::min);
        let under = top - elevation(doc.model(), level) - 0.25;
        assert!((low - (under - 0.02 - 0.008)).abs() < 1e-9, "{low}");
        let length: f64 = pieces.iter().map(|p| p.length()).sum();
        assert!((length - (4.0 + 2.0 * (z - low))).abs() < 1e-9);
        let ElementKind::Valve(v) = &doc.model().get(vid).unwrap().kind else {
            unreachable!()
        };
        let ElementKind::Pipe(on) = &doc.model().get(v.pipe).unwrap().kind else {
            unreachable!()
        };
        let at = on.start.lerp(on.end, v.t);
        assert!(at.distance(DVec3::new(3.0, 6.6, z)) < 1e-9, "{at}");
    }

    #[test]
    fn gas_beside_a_water_pipe_closer_than_20_mm_is_found() {
        let mut doc = Document::from_model(crate::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let mut gas = heating(level, DVec3::new(1.0, 1.0, 1.0), DVec3::new(3.0, 1.0, 1.0));
        let mut water = heating(
            level,
            DVec3::new(1.0, 1.03, 1.0),
            DVec3::new(3.0, 1.03, 1.0),
        );
        if let ElementKind::Pipe(p) = &mut gas.kind {
            p.system = "ZP".into();
            p.pipe_type = "HERZG".into();
        }
        if let ElementKind::Pipe(p) = &mut water.kind {
            p.system = "SV".into();
            p.insulation_mm = 0.0;
        }
        doc.commit(Transaction::new("x").insert(gas).insert(water))
            .unwrap();
        let found = check(doc.model());
        // 30 mm apart axis to axis, 8 mm radii each: 14 mm of air
        assert!(found.iter().any(
            |c| matches!(c.problem, Problem::GasGap { gap, .. } if (gap - 0.014).abs() < 1e-9)
        ));
    }
}
