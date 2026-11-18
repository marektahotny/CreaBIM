//! The fittings pipes need where they meet, found from how they are
//! drawn and never stored (PilonDesigner's `fittings.ts`,
//! `pipebends.ts`): two pipes make an elbow (a change of direction), a
//! reducer (of size) or a transition (of material); three a tee (a
//! branch, in drainage); four a cross. Where the pipe type comes with its
//! maker's fittings, the fitting is that maker's, by name and code. A
//! pipe that is bent rather than fitted takes no elbow.

use crate::pipe_types::{self, Fitting as CatalogueFitting, Medium, PipeType};
use creabim_core::mep::Pipe;
use creabim_core::{ElementId, ElementKind, Model};
use glam::DVec3;
use std::collections::BTreeMap;

/// What kind of fitting a node is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FittingKind {
    Elbow,
    Tee,
    /// A tee in drainage, at 45° or 87°.
    Branch,
    Cross,
    Reducer,
    /// From one material to another.
    Transition,
}

impl FittingKind {
    /// Its name in Slovak, as schedules name it.
    pub fn label(self) -> &'static str {
        match self {
            FittingKind::Elbow => "Koleno",
            FittingKind::Tee => "T-kus",
            FittingKind::Branch => "Odbočka",
            FittingKind::Cross => "Kríž",
            FittingKind::Reducer => "Redukcia",
            FittingKind::Transition => "Prechod",
        }
    }
}

/// A fitting where pipes meet.
#[derive(Clone, Debug, PartialEq)]
pub struct Fitting {
    pub kind: FittingKind,
    /// Where, in the model, m.
    pub pos: DVec3,
    pub level: ElementId,
    pub system: String,
    pub pipe_type: String,
    /// Outside diameters of the pipes it joins, mm, largest first.
    pub d: Vec<f64>,
    /// An elbow's or a branch's angle, degrees.
    pub angle: Option<f64>,
    pub pipes: Vec<ElementId>,
    /// What a schedule calls it.
    pub label: String,
    /// Its maker's code, if it is a maker's.
    pub code: String,
}

/// A node's key: its place to the millimetre.
pub fn key(p: DVec3) -> [i64; 3] {
    [p.x, p.y, p.z].map(|v| (v * 1000.0).round() as i64)
}

/// A pipe's ends in the model (its level's height added).
pub fn ends(model: &Model, p: &Pipe) -> (DVec3, DVec3) {
    let z = DVec3::Z * model.level(p.level).map_or(0.0, |l| l.elevation);
    (p.start + z, p.end + z)
}

/// The bending radius of a pipe that is bent rather than fitted, m: its
/// type allows it up to a size, at so many diameters with a bender.
pub fn bend_radius(p: &Pipe) -> Option<f64> {
    if p.height_mm.is_some() {
        return None;
    }
    let b = pipe_types::by_id(&p.pipe_type)?.bend?;
    (p.outer_mm <= b.max_outer_mm + 1e-6).then_some(p.outer_mm * b.r_tool / 1000.0)
}

/// The radius two pipes meeting at an angle are bent at instead of taking
/// an elbow: the same bendable pipe both sides, turning by 5° to 175°,
/// with arms `la` and `lb` long enough for the arc. `th` is the angle
/// between the arms, radians.
pub fn bend(p: &Pipe, q: &Pipe, la: f64, lb: f64, th: f64) -> Option<f64> {
    let least = 5.0_f64.to_radians();
    if th < least || std::f64::consts::PI - th < least {
        return None;
    }
    let same = q.pipe_type == p.pipe_type
        && q.system == p.system
        && (q.outer_mm - p.outer_mm).abs() < 1e-6;
    let r = bend_radius(p).filter(|_| same)?;
    // an arc too long for short arms is a fitting
    (r / (th / 2.0).tan() <= 0.45 * la.min(lb)).then_some(r)
}

/// One pipe's end at a node.
struct End<'a> {
    id: ElementId,
    pipe: &'a Pipe,
    /// Unit vector from the node along the pipe, and the pipe's length.
    dir: DVec3,
    len: f64,
}

/// The model's pipes' ends by node, in a fixed order.
fn nodes(model: &Model) -> BTreeMap<[i64; 3], (DVec3, Vec<End<'_>>)> {
    let mut at: BTreeMap<[i64; 3], (DVec3, Vec<End>)> = BTreeMap::new();
    let mut pipes: Vec<(ElementId, &Pipe)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Pipe(p) => Some((e.id, p)),
            _ => None,
        })
        .collect();
    pipes.sort_by_key(|(id, _)| *id);
    for (id, pipe) in pipes {
        let (a, b) = ends(model, pipe);
        let len = a.distance(b);
        if len < 1e-6 {
            continue;
        }
        for (node, far) in [(a, b), (b, a)] {
            at.entry(key(node))
                .or_insert_with(|| (node, vec![]))
                .1
                .push(End {
                    id,
                    pipe,
                    dir: (far - node) / len,
                    len,
                });
        }
    }
    at
}

/// A number as Slovak schedules write it: a comma, no trailing zero.
fn sk(v: f64) -> String {
    let r = (v * 10.0).round() / 10.0;
    if (r - r.round()).abs() < 1e-9 {
        format!("{}", r.round() as i64)
    } else {
        format!("{r:.1}").replace('.', ",")
    }
}

/// The maker's fitting of `kind` for these sizes (and the nearest angle).
fn item<'a>(
    t: Option<&'a PipeType>,
    kind: &str,
    d1: f64,
    d2: Option<f64>,
    angle: Option<f64>,
) -> Option<&'a CatalogueFitting> {
    let mut items = t?.fittings.iter().filter(|f| {
        f.kind == kind
            && (f.d1 - d1).abs() < 0.6
            && d2.is_none_or(|d2| (f.d2.unwrap_or(f.d1) - d2).abs() < 0.6)
    });
    let first = items.next()?;
    let Some(angle) = angle else {
        return Some(first);
    };
    let off = |f: &CatalogueFitting| (f.angle.unwrap_or(90.0) - angle).abs();
    Some(items.fold(first, |b, f| if off(f) < off(b) { f } else { b }))
}

const STD_ANGLES: [f64; 5] = [90.0, 87.0, 45.0, 30.0, 15.0];

/// The fittings of the whole model.
pub fn derive(model: &Model) -> Vec<Fitting> {
    let mut out = vec![];
    for (pos, list) in nodes(model).into_values() {
        if list.len() < 2 {
            continue;
        }
        // the largest pipe names the fitting
        let main = list
            .iter()
            .fold(&list[0], |b, x| {
                if x.pipe.outer_mm > b.pipe.outer_mm {
                    x
                } else {
                    b
                }
            })
            .pipe;
        let mut d: Vec<f64> = list.iter().map(|x| x.pipe.outer_mm).collect();
        d.sort_by(|a, b| b.total_cmp(a));
        let t = pipe_types::by_id(&main.pipe_type);
        let material = t.map_or("", |t| t.material.as_str());
        let drain = t.is_some_and(|t| t.media.contains(&Medium::Kanal));
        let size = |p: &Pipe| {
            pipe_types::by_id(&p.pipe_type).map_or_else(
                || sk(p.nominal_mm),
                |t| t.size_label(p.nominal_mm, p.height_mm),
            )
        };
        let mut push = |kind, angle: Option<f64>, label: String, code: Option<&str>| {
            out.push(Fitting {
                kind,
                pos,
                level: main.level,
                system: main.system.clone(),
                pipe_type: main.pipe_type.clone(),
                d: d.clone(),
                angle,
                pipes: list.iter().map(|x| x.id).collect(),
                label: label.replace("  ", " ").trim().to_string(),
                code: code.unwrap_or_default().to_string(),
            })
        };
        match list.as_slice() {
            [x, y] => {
                let th = x.dir.dot(y.dir).clamp(-1.0, 1.0).acos();
                // how far the run turns from straight on
                let dev = 180.0 - th.to_degrees();
                if x.pipe.pipe_type != y.pipe.pipe_type {
                    let (tx, ty) = (
                        pipe_types::by_id(&x.pipe.pipe_type),
                        pipe_types::by_id(&y.pipe.pipe_type),
                    );
                    fn m(t: Option<&PipeType>) -> &str {
                        t.map_or("", |t| t.material.as_str())
                    }
                    let (o1, o2) = (x.pipe.outer_mm, y.pipe.outer_mm);
                    // a maker's adaptor: PE to copper by both sizes, PE to steel by the PE's
                    let it = (m(ty).contains("Meď"))
                        .then(|| item(tx, "PeCu", o1, Some(o2), None))
                        .flatten()
                        .or_else(|| {
                            m(tx)
                                .contains("Meď")
                                .then(|| item(ty, "PeCu", o2, Some(o1), None))
                                .flatten()
                        })
                        .or_else(|| {
                            m(ty)
                                .contains("Oceľ")
                                .then(|| item(tx, "PeSteelBend", o1, None, None))
                                .flatten()
                        })
                        .or_else(|| {
                            m(tx)
                                .contains("Oceľ")
                                .then(|| item(ty, "PeSteelBend", o2, None, None))
                                .flatten()
                        });
                    let label = match it {
                        Some(it) => format!(
                            "{} d{}{}",
                            it.name,
                            sk(it.d1),
                            match (&it.thread, it.d2) {
                                (Some(t), _) => format!("×{t}"),
                                (None, Some(d2)) => format!("×{}", sk(d2)),
                                _ => String::new(),
                            }
                        ),
                        None => format!(
                            "Prechod {} {} / {} {}",
                            m(tx),
                            size(x.pipe),
                            m(ty),
                            size(y.pipe)
                        ),
                    };
                    push(
                        FittingKind::Transition,
                        (dev > 5.0).then_some(dev.round()),
                        label,
                        it.map(|i| i.code.as_str()),
                    );
                } else if (x.pipe.outer_mm - y.pipe.outer_mm).abs() > 0.1 {
                    let it = item(t, "Reducer", d[0], Some(d[1]), None);
                    let label = match it {
                        Some(it) => format!("{} d{}/{}", it.name, sk(d[0]), sk(d[1])),
                        None => format!("Redukcia {material} {}/{}", sk(d[0]), sk(d[1])),
                    };
                    push(
                        FittingKind::Reducer,
                        None,
                        label,
                        it.map(|i| i.code.as_str()),
                    );
                } else if dev > 5.0 && bend(x.pipe, y.pipe, x.len, y.len, th).is_none() {
                    let angle = STD_ANGLES
                        .iter()
                        .copied()
                        .find(|a| (a - dev).abs() < 3.0)
                        .unwrap_or(dev.round());
                    // drainage has no square elbow
                    let angle = if drain && angle == 90.0 { 87.0 } else { angle };
                    let it = item(t, "Elbow", d[0], None, Some(angle));
                    let label = match it {
                        Some(it) => format!("{} d{}", it.name, sk(d[0])),
                        None => format!("Koleno {}° {material} {}", sk(angle), size(main)),
                    };
                    push(
                        FittingKind::Elbow,
                        Some(angle),
                        label,
                        it.map(|i| i.code.as_str()),
                    );
                }
                // straight on in the same pipe: none (couplings by bar
                // length are not counted)
            }
            [_, _, _] => {
                // the branch is the pipe off the straightest pair; its
                // angle is to the nearer way of the run
                let c = |i: usize, j: usize| list[i].dir.dot(list[j].dir);
                let pairs = [(0, 1, 2), (0, 2, 1), (1, 2, 0)];
                let (ri, rj, bi) =
                    pairs.into_iter().fold(
                        pairs[0],
                        |b, x| if c(x.0, x.1) < c(b.0, b.1) { x } else { b },
                    );
                let br = [ri, rj]
                    .into_iter()
                    .map(|r| c(bi, r).clamp(-1.0, 1.0).acos().to_degrees())
                    .fold(f64::MAX, f64::min);
                let od = if br < 60.0 { 45.0 } else { 87.0 };
                let it = item(t, "Tee", d[0], Some(d[2]), None)
                    .or_else(|| item(t, "Tee", d[0], Some(d[0]), None));
                let kind = if drain {
                    FittingKind::Branch
                } else {
                    FittingKind::Tee
                };
                let label = match it {
                    Some(it) => format!(
                        "{} d{}{}",
                        it.name,
                        sk(it.d1),
                        match it.d2 {
                            Some(d2) if (d2 - it.d1).abs() > 0.1 => format!("/{}", sk(d2)),
                            _ => String::new(),
                        }
                    ),
                    None => format!(
                        "{} {material} {}",
                        if drain {
                            format!("Odbočka {}°", sk(od))
                        } else {
                            "T-kus".to_string()
                        },
                        if d[2] < d[0] - 0.1 {
                            format!("{}/{}", sk(d[0]), sk(d[2]))
                        } else {
                            sk(d[0])
                        }
                    ),
                };
                push(
                    kind,
                    drain.then_some(od),
                    label,
                    it.map(|i| i.code.as_str()),
                );
            }
            _ => push(
                FittingKind::Cross,
                None,
                format!("Kríž {material} {}", sk(d[0])),
                None,
            ),
        }
    }
    out
}

/// A line of the fittings schedule: so many of one fitting in a system.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub system: String,
    pub kind: FittingKind,
    pub label: String,
    pub code: String,
    pub count: usize,
}

/// The fittings counted for a schedule: by system, then by name.
pub fn summary(fittings: &[Fitting]) -> Vec<Line> {
    let mut m: BTreeMap<(String, String), Line> = BTreeMap::new();
    for f in fittings {
        m.entry((f.system.clone(), f.label.clone()))
            .or_insert_with(|| Line {
                system: f.system.clone(),
                kind: f.kind,
                label: f.label.clone(),
                code: f.code.clone(),
                count: 0,
            })
            .count += 1;
    }
    m.into_values().collect()
}

/// A line of the insulation schedule: one thickness on one size of pipe
/// in one system (PilonDesigner's `pipeSchedule` insul rows).
#[derive(Clone, Debug, PartialEq)]
pub struct InsulationLine {
    pub system: String,
    /// The pipe's outside diameter (the insulation's inside), mm.
    pub outer_mm: f64,
    pub mm: f64,
    /// Heat insulation, else against condensation.
    pub heat: bool,
    /// Metres of pipe it goes on.
    pub length: f64,
    /// With `waste_pct` more for offcuts.
    pub gross: f64,
    pub pipes: Vec<ElementId>,
}

/// The insulation the pipes drawn need, by system, size and thickness:
/// what was typed on each, else what its system wants
/// ([`Pipe::insulation`]); `waste_pct` is added for offcuts.
pub fn insulation(model: &Model, waste_pct: f64) -> Vec<InsulationLine> {
    let mut m: BTreeMap<(String, i64, i64), InsulationLine> = BTreeMap::new();
    for e in model.iter() {
        let ElementKind::Pipe(p) = &e.kind else {
            continue;
        };
        let mm = p.insulation();
        if mm <= 0.0 || p.height_mm.is_some() {
            continue;
        }
        let heat = creabim_core::mep::insulation_purpose(p.kind()) == "heat";
        let l = m
            .entry((
                p.system.clone(),
                (p.outer_mm * 10.0).round() as i64,
                (mm * 10.0).round() as i64,
            ))
            .or_insert_with(|| InsulationLine {
                system: p.system.clone(),
                outer_mm: p.outer_mm,
                mm,
                heat,
                length: 0.0,
                gross: 0.0,
                pipes: vec![],
            });
        l.length += p.length();
        l.pipes.push(e.id);
    }
    let mut out: Vec<InsulationLine> = m.into_values().collect();
    for l in &mut out {
        l.gross = l.length * (1.0 + waste_pct / 100.0);
    }
    out
}

/// A smaller pipe joined to a larger one at a node: it narrows there (a
/// reducer, a reducing tee or cross).
#[derive(Clone, Debug, PartialEq)]
pub struct Reduction {
    pub pos: DVec3,
    /// Unit vector from the node into the smaller pipe.
    pub dir: DVec3,
    pub pipe: ElementId,
    /// Outside diameters of the larger and of the smaller pipe, mm.
    pub d_big: f64,
    pub d_small: f64,
    /// The smaller pipe's length, m.
    pub len: f64,
}

/// The reductions at a pipe's ends (start, end): the largest pipe of its
/// system there, mm, where this one is smaller. A drawing shows each as
/// a solid triangle pointing into the smaller pipe (STN EN ISO 6412).
pub fn reductions_of(model: &Model, p: &Pipe) -> [Option<f64>; 2] {
    let (a, b) = ends(model, p);
    let keys = [key(a), key(b)];
    let mut big = [p.outer_mm; 2];
    let mut other_type = [false; 2];
    let mut count = [0usize; 2];
    for e in model.iter() {
        let ElementKind::Pipe(q) = &e.kind else {
            continue;
        };
        let (qa, qb) = ends(model, q);
        let (ka, kb) = (key(qa), key(qb));
        if (ka == keys[0] && kb == keys[1]) || (ka == keys[1] && kb == keys[0]) {
            continue;
        }
        for i in 0..2 {
            if ka == keys[i] || kb == keys[i] {
                count[i] += 1;
                other_type[i] |= q.pipe_type != p.pipe_type;
                if q.system == p.system {
                    big[i] = big[i].max(q.outer_mm);
                }
            }
        }
    }
    [0, 1].map(|i| {
        // two pipes of different makes: a transition, with its own mark
        let transition = count[i] == 1 && other_type[i];
        (big[i] > p.outer_mm + 0.1 && !transition).then_some(big[i])
    })
}

/// The reductions of the whole model.
pub fn reductions(model: &Model) -> Vec<Reduction> {
    let mut out = vec![];
    for (pos, list) in nodes(model).into_values() {
        if list.len() < 2 {
            continue;
        }
        if list.len() == 2 && list[0].pipe.pipe_type != list[1].pipe.pipe_type {
            continue;
        }
        let big = list.iter().fold(&list[0], |b, x| {
            if x.pipe.outer_mm > b.pipe.outer_mm {
                x
            } else {
                b
            }
        });
        for x in &list {
            if x.pipe.outer_mm > big.pipe.outer_mm - 0.1 || x.pipe.system != big.pipe.system {
                continue;
            }
            out.push(Reduction {
                pos,
                dir: x.dir,
                pipe: x.id,
                d_big: big.pipe.outer_mm,
                d_small: x.pipe.outer_mm,
                len: x.len,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::{Element, Level, Transaction};

    /// A pipe: its system, type, size and ends.
    type Run<'a> = (&'a str, &'a str, f64, [f64; 3], [f64; 3]);

    fn model(pipes: &[Run]) -> Model {
        let mut m = Model::new();
        let l = Element::new(
            "Ground",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let level = l.id;
        let mut tx = Transaction::new("pipes").insert(l);
        for (system, kind, nominal, a, b) in pipes {
            let t = pipe_types::by_id(kind).unwrap();
            let s = t.size(*nominal).unwrap();
            tx = tx.insert(Element::new(
                "Pipe",
                ElementKind::Pipe(Pipe {
                    level,
                    system: system.to_string(),
                    start: DVec3::from_array(*a),
                    end: DVec3::from_array(*b),
                    pipe_type: kind.to_string(),
                    nominal_mm: *nominal,
                    outer_mm: s.outer_mm,
                    inner_mm: s.inner_mm,
                    height_mm: None,
                    insulation_mm: 0.0,
                    insulation_set: false,
                    mark: None,
                }),
            ));
        }
        tx.apply(&mut m).unwrap();
        m
    }

    #[test]
    fn a_corner_is_an_elbow_unless_the_pipe_is_bent() {
        // drainage at a right angle: an 87° elbow
        let m = model(&[
            ("K", "PPHT", 100.0, [0.0, 0.0, 0.0], [2.0, 0.0, 0.0]),
            ("K", "PPHT", 100.0, [2.0, 0.0, 0.0], [2.0, 2.0, 0.0]),
        ]);
        let f = derive(&m);
        assert_eq!(f.len(), 1);
        assert_eq!((f[0].kind, f[0].angle), (FittingKind::Elbow, Some(87.0)));
        assert!(f[0].label.starts_with("Koleno 87°"), "{}", f[0].label);
        // multilayer pipe 16: bent, no fitting
        let m = model(&[
            ("SV", "HERZ", 16.0, [0.0, 0.0, 0.3], [2.0, 0.0, 0.3]),
            ("SV", "HERZ", 16.0, [2.0, 0.0, 0.3], [2.0, 2.0, 0.3]),
        ]);
        assert!(derive(&m).is_empty());
        // 32: too large to bend, the maker's elbow with its code
        let m = model(&[
            ("SV", "HERZ", 32.0, [0.0, 0.0, 0.3], [2.0, 0.0, 0.3]),
            ("SV", "HERZ", 32.0, [2.0, 0.0, 0.3], [2.0, 2.0, 0.3]),
        ]);
        let f = derive(&m);
        assert_eq!(f[0].kind, FittingKind::Elbow);
        assert!(
            f[0].label.contains("HERZ") && !f[0].code.is_empty(),
            "{:?}",
            f[0]
        );
    }

    #[test]
    fn three_pipes_are_a_tee_or_a_branch_and_sizes_reduce() {
        let m = model(&[
            ("K", "PPHT", 100.0, [0.0, 0.0, 0.0], [2.0, 0.0, 0.0]),
            ("K", "PPHT", 100.0, [2.0, 0.0, 0.0], [4.0, 0.0, 0.0]),
            ("K", "PPHT", 50.0, [2.0, 0.0, 0.0], [3.0, 1.0, 0.0]),
        ]);
        let f = derive(&m);
        assert_eq!(f.len(), 1);
        assert_eq!((f[0].kind, f[0].angle), (FittingKind::Branch, Some(45.0)));
        assert_eq!(f[0].label, "Odbočka 45° PP-HT 110/50", "{}", f[0].label);
        // the branch narrows at the node
        // insulation: heat insulation on hot water, against condensation
        // on cold, none on drainage; typed 0 keeps a pipe bare
        let ins = insulation(&m, 5.0);
        assert!(ins.iter().all(|l| l.system != "K" && l.mm > 0.0));
        let r = reductions(&m);
        assert_eq!(r.len(), 1);
        assert_eq!((r[0].d_big, r[0].d_small), (110.0, 50.0));
        let ElementKind::Pipe(small) = &m.get(r[0].pipe).unwrap().kind else {
            panic!()
        };
        assert_eq!(reductions_of(&m, small), [Some(110.0), None]);
        // counted for the schedule
        let s = summary(&f);
        assert_eq!((s.len(), s[0].count), (1, 1));
    }

    #[test]
    fn two_sizes_in_line_are_a_reducer() {
        let m = model(&[
            ("SV", "PPR", 25.0, [0.0, 0.0, 0.3], [2.0, 0.0, 0.3]),
            ("SV", "PPR", 20.0, [2.0, 0.0, 0.3], [4.0, 0.0, 0.3]),
        ]);
        let f = derive(&m);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, FittingKind::Reducer);
    }
}
