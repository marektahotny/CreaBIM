//! Storm water drainage inside a building, as PilonDesigner's `storm.ts`
//! (STN EN 12056-3, STN 75 6760): Q = i · A · C for each roof drain or
//! downpipe, summed downstream; a stack by Wyly-Eaton at a filling of
//! 0.33, a drain laid flat by Colebrook-White at h/d = 0.7 and its own
//! fall (0.5 % at least), with the pipe type's roughness. A size never
//! falls downstream.

use crate::tree::{self, Tree};
use creabim_core::{polygon, ElementId, ElementKind, Model};
use glam::DVec3;

/// Rain intensity, l/(s·m²): STN 75 6760, the national annex of STN EN
/// 12056-3.
pub const INTENSITY: f64 = 0.03;
/// A stack's filling (STN EN 12056-3, 6.3).
pub const STACK_FILL: f64 = 0.33;
/// A drain's filling inside the building, h/d.
pub const DRAIN_FILL: f64 = 0.7;
/// A drain's least fall.
pub const MIN_SLOPE: f64 = 0.005;
/// Water's kinematic viscosity at 10 °C, m²/s.
const NU: f64 = 1.31e-6;

/// The runoff coefficient by the roof's pitch (STN 75 6760): 1.0 over a
/// fall of 5 %, else 0.8.
pub fn runoff_coef(pitch_deg: f64) -> f64 {
    if pitch_deg.to_radians().tan() > 0.05 {
        1.0
    } else {
        0.8
    }
}

/// A stack's capacity, l/s: Wyly-Eaton (STN EN 12056-3, equation 9),
/// the inner diameter and the roughness in mm.
pub fn stack_capacity(di_mm: f64, kb_mm: f64, f: f64) -> f64 {
    2.5e-4 * kb_mm.max(0.01).powf(-1.0 / 6.0) * f.powf(5.0 / 3.0) * di_mm.powf(8.0 / 3.0)
}

/// The flow, l/s, and velocity, m/s, of a pipe filled to `hd`
/// (Colebrook-White on the hydraulic diameter).
pub fn partial_flow(di_mm: f64, hd: f64, slope: f64, kb_mm: f64) -> (f64, f64) {
    let (d, k, g) = (di_mm / 1000.0, kb_mm / 1000.0, 9.81);
    let th = 2.0 * (1.0 - 2.0 * hd.clamp(1e-4, 1.0)).acos();
    let a = d * d / 8.0 * (th - th.sin());
    let p = d * th / 2.0;
    let dh = 4.0 * a / p;
    let s = (2.0 * g * dh * slope.max(1e-6)).sqrt();
    let v = -2.0 * (2.51 * NU / (dh * s) + k / (3.71 * dh)).log10() * s;
    (v * a * 1000.0, v)
}

/// The filling h/d a flow runs at.
pub fn fill_for(q: f64, di_mm: f64, slope: f64, kb_mm: f64) -> f64 {
    if q <= 0.0 {
        return 0.0;
    }
    if partial_flow(di_mm, 0.94, slope, kb_mm).0 < q {
        return 1.0;
    }
    let (mut lo, mut hi) = (0.001, 1.0);
    for _ in 0..40 {
        let m = (lo + hi) / 2.0;
        if partial_flow(di_mm, m, slope, kb_mm).0 < q {
            lo = m;
        } else {
            hi = m;
        }
    }
    (lo + hi) / 2.0
}

/// A drain's catchment.
#[derive(Clone, Debug, PartialEq)]
pub struct DrainResult {
    pub id: ElementId,
    pub name: String,
    pub area_m2: f64,
    pub coef: f64,
    pub flow_ls: f64,
    /// Its area is the roof's over it, not its own.
    pub from_roof: bool,
}

/// A pipe's result (its worst stretch).
#[derive(Clone, Debug, PartialEq)]
pub struct PipeResult {
    pub id: ElementId,
    pub flow_ls: f64,
    pub vertical: bool,
    /// The fall designed with (0 for a stack).
    pub slope: f64,
    pub nominal_mm: f64,
    /// The size proposed, as the type names it.
    pub proposed_mm: f64,
    pub capacity_ls: f64,
    /// h/d at the proposed size (0 for a stack).
    pub fill: f64,
    pub velocity_ms: f64,
    /// Drawn with less than the least fall.
    pub too_flat: bool,
    /// Even the type's largest size does not carry the flow.
    pub too_small: bool,
}

#[derive(Clone, Debug, Default)]
pub struct StormResult {
    pub drains: Vec<DrainResult>,
    pub pipes: Vec<PipeResult>,
    pub area_m2: f64,
    /// The flow out of the building, l/s.
    pub flow_ls: f64,
    /// Drains with no area of their own and no roof over them.
    pub no_area: usize,
    pub unreached: usize,
}

/// The roofs of the model: outline in the model, area, pitch.
fn roofs(model: &Model) -> Vec<(Vec<glam::DVec2>, f64, f64, f64)> {
    model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Roof(r) if r.boundary.len() > 2 => Some((
                r.boundary.clone(),
                polygon::signed_area2(&r.boundary).abs() / 2.0,
                r.pitch,
                r.overhang.max(0.0) + 0.3,
            )),
            _ => None,
        })
        .collect()
}

/// `p` is on the roof or within `m` of its edge.
fn on_roof(outline: &[glam::DVec2], m: f64, p: glam::DVec2) -> bool {
    if polygon::contains(outline, p) {
        return true;
    }
    (0..outline.len()).any(|i| {
        let (a, b) = (outline[i], outline[(i + 1) % outline.len()]);
        let d = b - a;
        let t = ((p - a).dot(d) / d.length_squared().max(1e-12)).clamp(0.0, 1.0);
        (a + d * t).distance(p) <= m
    })
}

/// The storm water system `code` of the model, calculated.
pub fn run(model: &Model, code: &str) -> Option<StormResult> {
    let t = tree::build(model, code)?;
    Some(calc(model, &t))
}

fn calc(model: &Model, t: &Tree) -> StormResult {
    let mut out = StormResult {
        unreached: t.unreached.len(),
        ..StormResult::default()
    };
    // catchments: a drain's own area, else the roof over it shared
    // among the drains on it
    let roofs = roofs(model);
    let roof_of = |p: DVec3| {
        roofs
            .iter()
            .position(|(o, _, _, m)| on_roof(o, *m, p.truncate()))
    };
    let mut per_roof = vec![0usize; roofs.len()];
    for d in &t.terminals {
        if d.valve.area_m2.is_none() {
            if let Some(r) = roof_of(d.at) {
                per_roof[r] += 1;
            }
        }
    }
    for d in &t.terminals {
        let (mut area, mut coef, mut from_roof) =
            (d.valve.area_m2.unwrap_or(0.0), d.valve.coef, false);
        if area <= 0.0 {
            match roof_of(d.at) {
                Some(r) => {
                    area = roofs[r].1 / per_roof[r].max(1) as f64;
                    coef = coef.or(Some(runoff_coef(roofs[r].2)));
                    from_roof = true;
                }
                None => out.no_area += 1,
            }
        }
        let coef = coef.unwrap_or(1.0);
        out.area_m2 += area;
        out.drains.push(DrainResult {
            id: d.id,
            name: d.name.clone(),
            area_m2: area,
            coef,
            flow_ls: INTENSITY * area * coef,
            from_roof,
        });
    }
    // flows downstream
    let below = t.terminals_below();
    let flow: Vec<f64> = below
        .iter()
        .map(|b| b.iter().map(|&k| out.drains[k].flow_ls).sum())
        .collect();
    // sizes from the ends to the outlet: none falls downstream
    let kids = t.children();
    let n = t.stretches.len();
    let mut design = vec![0.0; n];
    struct Info {
        cap: f64,
        fill: f64,
        v: f64,
        slope: f64,
        vertical: bool,
        too_flat: bool,
        too_small: bool,
    }
    let mut info: Vec<Option<Info>> = (0..n).map(|_| None).collect();
    for i in (0..n).rev() {
        let s = &t.stretches[i];
        let ty = crate::pipe_types::by_id(&s.pipe.pipe_type);
        let kb = ty.map_or(1.0, |t| t.roughness_mm);
        let q = flow[i];
        let up = kids[i].iter().map(|&k| design[k]).fold(0.0, f64::max);
        let (dz, dh) = s.rise_run();
        let vertical = dz > dh;
        let real = if dh > 1e-6 { dz / dh } else { 0.0 };
        let slope = if vertical { 1.0 } else { real.max(MIN_SLOPE) };
        let cap = |di: f64| {
            if vertical {
                stack_capacity(di, kb, STACK_FILL)
            } else {
                partial_flow(di, DRAIN_FILL, slope, kb).0
            }
        };
        let sizes: Vec<(f64, f64)> = ty
            .map(|t| t.sizes.iter().map(|z| (z.nominal_mm, z.inner_mm)).collect())
            .unwrap_or_default();
        let pick = sizes
            .iter()
            .copied()
            .find(|z| z.0 >= up && cap(z.1) >= q)
            .or(sizes.last().copied());
        let (nom, di) = pick.unwrap_or((s.pipe.nominal_mm, s.pipe.inner_mm));
        let c = cap(di);
        let fill = if vertical {
            0.0
        } else {
            fill_for(q, di, slope, kb)
        };
        let v = if vertical || q <= 0.0 {
            0.0
        } else {
            partial_flow(di, fill.max(0.01), slope, kb).1
        };
        design[i] = nom;
        info[i] = Some(Info {
            cap: c,
            fill,
            v,
            slope: if vertical { 0.0 } else { slope },
            vertical,
            too_flat: !vertical && dh > 1e-6 && real < MIN_SLOPE,
            too_small: c < q,
        });
    }
    // a pipe split at tees takes its largest stretch
    for (id, parts) in t.by_pipe() {
        let worst = parts
            .iter()
            .copied()
            .max_by(|&a, &b| {
                design[a]
                    .total_cmp(&design[b])
                    .then(flow[a].total_cmp(&flow[b]))
            })
            .unwrap_or(parts[0]);
        let x = info[worst].as_ref().expect("every stretch is sized");
        out.pipes.push(PipeResult {
            id,
            flow_ls: flow[worst],
            vertical: x.vertical,
            slope: x.slope,
            nominal_mm: t.stretches[worst].pipe.nominal_mm,
            proposed_mm: design[worst],
            capacity_ls: x.cap,
            fill: x.fill,
            velocity_ms: x.v,
            too_flat: parts
                .iter()
                .any(|&p| info[p].as_ref().is_some_and(|i| i.too_flat)),
            too_small: parts
                .iter()
                .any(|&p| info[p].as_ref().is_some_and(|i| i.too_small)),
        });
    }
    out.flow_ls = t
        .stretches
        .iter()
        .enumerate()
        .filter(|(_, s)| s.parent.is_none())
        .map(|(i, _)| flow[i])
        .sum();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::mep::{Pipe, Valve};
    use creabim_core::{Document, Element, Transaction};

    #[test]
    fn the_formulas_are_the_standards() {
        // STN EN 12056-3 table 8: DN 100 (di 100), f = 0.33, kb 0.25: 10.7 l/s
        assert!((stack_capacity(100.0, 0.25, 0.33) - 10.7).abs() < 0.15);
        let (q, v) = partial_flow(100.0, 0.7, 0.01, 1.0);
        assert!(q > 3.5 && q < 5.5 && v > 0.6 && v < 1.0, "{q} {v}");
        let h = fill_for(q, 100.0, 0.01, 1.0);
        assert!((h - 0.7).abs() < 1e-6);
        assert_eq!(fill_for(1e3, 100.0, 0.01, 1.0), 1.0);
        assert_eq!((runoff_coef(2.0), runoff_coef(30.0)), (0.8, 1.0));
    }

    fn pipe(
        level: ElementId,
        ty: &crate::pipe_types::PipeType,
        a: [f64; 3],
        b: [f64; 3],
    ) -> Element {
        let z = &ty.sizes[0];
        Element::new(
            "pipe",
            ElementKind::Pipe(Pipe {
                level,
                system: "D".into(),
                start: DVec3::from(a),
                end: DVec3::from(b),
                pipe_type: ty.id.clone(),
                nominal_mm: z.nominal_mm,
                outer_mm: z.outer_mm,
                inner_mm: z.inner_mm,
                height_mm: None,
                insulation_mm: 0.0,
            }),
        )
    }

    #[test]
    fn two_drains_with_their_areas_size_the_stacks_and_the_drain() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let ty = crate::pipe_types::for_medium(crate::pipe_types::Medium::Kanal)
            .next()
            .expect("a drainage pipe type");
        // two stacks from the roof, joined under the floor, out at 1 %
        let s1 = pipe(level, ty, [20.0, 20.0, 6.0], [20.0, 20.0, -1.0]);
        let s2 = pipe(level, ty, [26.0, 20.0, 6.0], [26.0, 20.0, -1.06]);
        let d1 = pipe(level, ty, [20.0, 20.0, -1.0], [26.0, 20.0, -1.06]);
        let d2 = pipe(level, ty, [26.0, 20.0, -1.06], [36.0, 20.0, -1.16]);
        let (i1, i2, o) = (s1.id, s2.id, d2.id);
        let mut v1 = Valve::new(i1, 0.0, "vpust");
        v1.area_m2 = Some(150.0);
        let mut v2 = Valve::new(i2, 0.0, "vpust");
        v2.area_m2 = Some(250.0);
        v2.coef = Some(0.8);
        doc.commit(
            Transaction::new("storm")
                .insert(s1)
                .insert(s2)
                .insert(d1)
                .insert(d2)
                .insert(Element::new("V1", ElementKind::Valve(v1)))
                .insert(Element::new("V2", ElementKind::Valve(v2))),
        )
        .unwrap();
        let r = run(doc.model(), "D").expect("storm pipes");
        assert_eq!((r.drains.len(), r.no_area, r.unreached), (2, 0, 0));
        // 0.03 × 150 × 1.0 + 0.03 × 250 × 0.8
        assert!((r.flow_ls - 10.5).abs() < 1e-9, "{}", r.flow_ls);
        assert_eq!(r.area_m2, 400.0);
        let of = |id| r.pipes.iter().find(|p| p.id == id).unwrap();
        assert!((of(i1).flow_ls - 4.5).abs() < 1e-9 && of(i1).vertical);
        assert!((of(i2).flow_ls - 6.0).abs() < 1e-9);
        let out = of(o);
        assert!((out.flow_ls - 10.5).abs() < 1e-9 && !out.vertical);
        assert!((out.slope - 0.01).abs() < 1e-9);
        assert!(out.capacity_ls >= out.flow_ls && out.fill > 0.0 && out.fill <= 0.7 + 1e-6);
        // sizes never fall downstream
        assert!(out.proposed_mm >= of(i1).proposed_mm.max(of(i2).proposed_mm));
        assert!(of(i1).capacity_ls >= 4.5);
        assert!(run(doc.model(), "XX").is_none());
    }
}
