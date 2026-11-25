//! The penetrations the trades' pipes and ducts make through the building,
//! taken over into the architect's and the structural engineer's drawings
//! (George 2026-11-25: "niektoré veci sa budú musieť preberať z profesií do
//! ASR, možno aj STA (prierazy cez nosné steny, cez nosné stropy…), lebo
//! z VZT zostala v architektúre diera v podhľade; a v ASR má byť aj
//! vyvedenie nad strešnú rovinu zobrazené, označené").
//!
//! They are worked out from the pipes as they run, every time a drawing
//! is made, so they follow the trade's design without being kept twice: a
//! wall a pipe runs through, a floor or ceiling slab, a suspended ceiling
//! (podhľad) and the roof. The hole is the pipe's outer size with its
//! insulation, rounded up to whole centimetres; no clearance is added.

use creabim_core::mep::SystemKind;
use creabim_core::viewmodes::Trade;
use creabim_core::{ElementId, ElementKind, Model, Penetration};
use glam::{DVec2, DVec3};

use crate::plan::{Pen, PlanItem};

/// What a pipe passes through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Through {
    Wall,
    Slab,
    /// A suspended ceiling (podhľad).
    Ceiling,
    Roof,
}

/// Where a trade's pipe passes through the building.
#[derive(Clone, Debug)]
pub struct Crossing {
    pub pipe: ElementId,
    /// The wall, slab, ceiling or roof it passes through.
    pub host: ElementId,
    pub through: Through,
    /// The level the host stands on.
    pub host_level: ElementId,
    /// Where its axis passes the host's middle, absolute.
    pub at: DVec3,
    /// The hole: round with its diameter, or a rectangle's width and
    /// height, m.
    pub round: bool,
    pub size: [f64; 2],
    /// The pipe's code in the label: "VZT", "ZTI"…
    pub code: &'static str,
    /// The pipe's own size: "Ø160", "400×200".
    pub pipe_size: String,
    /// Through a wall: how far along it, and the bottom of the hole
    /// above the wall's level.
    pub along: f64,
    pub sill: f64,
    /// The wall's direction there (through a wall).
    pub dir: DVec2,
    /// A load-bearing wall, a slab or a roof: drawn in the structural
    /// drawings too.
    pub bearing: bool,
}

/// The trade code a pipe of `kind` is marked with.
pub fn code(kind: SystemKind) -> &'static str {
    match kind {
        SystemKind::Gas => "PLY",
        k => match Trade::of_system(k) {
            Trade::Plumbing => "ZTI",
            Trade::Heating => "VYK",
            Trade::Ventilation => "VZT",
            Trade::MedGas => "MP",
            Trade::Electrical => "ELI",
        },
    }
}

/// Up to whole centimetres, m.
fn whole_cm(v: f64) -> f64 {
    (v * 100.0 - 1e-6).ceil() / 100.0
}

fn mm(v: f64) -> String {
    format!("{}", (v * 1000.0).round() as i64)
}

/// The hole a pipe needs: its outer size with its insulation.
fn hole_of(p: &creabim_core::mep::Pipe) -> (bool, [f64; 2], String) {
    let ins = p.insulation() / 1000.0;
    match p.height_mm {
        Some(h) => {
            let w = p.nominal_mm / 1000.0;
            let h = h / 1000.0;
            (
                false,
                [whole_cm(w + 2.0 * ins), whole_cm(h + 2.0 * ins)],
                format!("{}×{}", mm(w), mm(h)),
            )
        }
        None => {
            // a duct is named by its outside (Spiro Ø100 is 100 mm
            // across), a pipe by its own size: its outer diameter
            let d = if p.kind().is_air() {
                p.nominal_mm
            } else {
                p.outer_mm.max(p.nominal_mm)
            } / 1000.0;
            (
                true,
                [whole_cm(d + 2.0 * ins); 2],
                format!("Ø{}", p.nominal_mm.round() as i64),
            )
        }
    }
}

/// Every place a trade's pipe passes through a wall, a slab, a ceiling or
/// a roof (where no penetration was drawn there by hand).
pub fn crossings(model: &Model) -> Vec<Crossing> {
    let rules = creabim_core::masonry::rules(model);
    let elev = |l: ElementId| model.level(l).map_or(0.0, |l| l.elevation);
    let mut out: Vec<Crossing> = vec![];
    // holes drawn by hand: a penetration already there
    let mut manual: Vec<Vec<DVec2>> = vec![];
    let mut manual_wall: Vec<(ElementId, f64, f64)> = vec![];
    for e in model.iter() {
        match &e.kind {
            ElementKind::Shaft(sh) if sh.penetration.is_some() => manual.push(sh.boundary.clone()),
            ElementKind::Opening(o) if o.kind == creabim_core::OpeningKind::Hole => {
                manual_wall.push((o.host, o.offset, o.offset + o.width));
            }
            _ => {}
        }
    }
    for e in model.iter() {
        let ElementKind::Pipe(p) = &e.kind else {
            continue;
        };
        let z0 = elev(p.level);
        let a = p.start + DVec3::Z * z0;
        let b = p.end + DVec3::Z * z0;
        let (round, size, pipe_size) = hole_of(p);
        let code = code(p.kind());
        let base = |host: ElementId, through, host_level, at: DVec3, bearing| Crossing {
            pipe: e.id,
            host,
            through,
            host_level,
            at,
            round,
            size,
            code,
            pipe_size: pipe_size.clone(),
            along: 0.0,
            sill: 0.0,
            dir: DVec2::X,
            bearing,
        };
        // through something lying flat: where the axis passes height z
        let at_height = |z: f64| -> Option<DVec3> {
            let (lo, hi) = (a.z.min(b.z), a.z.max(b.z));
            if (b.z - a.z).abs() < 1e-6 || z < lo + 1e-6 || z > hi - 1e-6 {
                return None;
            }
            Some(a.lerp(b, (z - a.z) / (b.z - a.z)))
        };
        for h in model.iter() {
            match &h.kind {
                ElementKind::Wall(w) if w.curtain.is_none() && !w.is_arc() => {
                    let (s, t) = (w.start, w.end);
                    let (pa, pb) = (a.truncate(), b.truncate());
                    let d = pb - pa;
                    let wd = t - s;
                    let den = d.perp_dot(wd);
                    if den.abs() < 1e-9 || d.length() < 1e-6 {
                        continue;
                    }
                    let k = (s - pa).perp_dot(wd) / den;
                    let u = (s - pa).perp_dot(d) / den;
                    if !(0.0..=1.0).contains(&k) || !(0.0..=1.0).contains(&u) {
                        continue;
                    }
                    let at = a.lerp(b, k);
                    let wb = creabim_core::walls::base_z(model, w);
                    let top = wb + crate::plan::wall_top_height(model, w);
                    if at.z < wb || at.z > top {
                        continue;
                    }
                    let along = u * wd.length();
                    if manual_wall
                        .iter()
                        .any(|(id, u0, u1)| *id == h.id && along > u0 - 0.05 && along < u1 + 0.05)
                    {
                        continue;
                    }
                    let mut c = base(
                        h.id,
                        Through::Wall,
                        w.level,
                        at,
                        creabim_core::masonry::is_bearing(model, w, &rules),
                    );
                    c.along = along;
                    c.dir = wd.normalize();
                    c.sill = at.z - size[1] / 2.0 - elev(w.level);
                    out.push(c);
                }
                ElementKind::Slab(s) => {
                    let top = s.top(elev(s.level));
                    let Some(at) = at_height(top - s.thickness / 2.0) else {
                        continue;
                    };
                    let q = at.truncate();
                    if !creabim_core::polygon::contains(&s.outline(), q)
                        || s.holes
                            .iter()
                            .any(|o| creabim_core::polygon::contains(o, q))
                        || manual.iter().any(|o| creabim_core::polygon::contains(o, q))
                    {
                        continue;
                    }
                    out.push(base(h.id, Through::Slab, s.level, at, true));
                }
                ElementKind::Ceiling(c) => {
                    let (lo, hi) = c.span(model);
                    let Some(at) = at_height((lo + hi) / 2.0) else {
                        continue;
                    };
                    let q = at.truncate();
                    if !c
                        .outline(model)
                        .is_some_and(|o| creabim_core::polygon::contains(&o, q))
                        || manual.iter().any(|o| creabim_core::polygon::contains(o, q))
                    {
                        continue;
                    }
                    out.push(base(h.id, Through::Ceiling, c.level, at, false));
                }
                ElementKind::Roof(r) => {
                    // a pipe standing up through it
                    if a.truncate().distance(b.truncate()) > 0.01 {
                        continue;
                    }
                    let q = a.truncate();
                    let rb = r.base(model);
                    let Some(under) = r
                        .faces()
                        .into_iter()
                        .filter(|f| creabim_core::polygon::contains(&f.outline, q))
                        .map(|f| rb + f.z(q))
                        .reduce(f64::max)
                    else {
                        continue;
                    };
                    let Some(at) = at_height(under + r.vertical_thickness() / 2.0)
                        // a pipe ending on the roof's top: its outlet sits on it
                        .or_else(|| {
                            let hi = if a.z > b.z { a } else { b };
                            let lo = a.z.min(b.z);
                            (lo < under && hi.z >= under).then_some(hi)
                        })
                    else {
                        continue;
                    };
                    if manual.iter().any(|o| creabim_core::polygon::contains(o, q)) {
                        continue;
                    }
                    out.push(base(h.id, Through::Roof, r.level, at, true));
                }
                _ => {}
            }
        }
    }
    // a pipe's two pieces meeting in a wall or a slab: once
    let mut kept: Vec<Crossing> = vec![];
    for c in out {
        if !kept
            .iter()
            .any(|k| k.host == c.host && k.at.distance(c.at) < 0.02)
        {
            kept.push(c);
        }
    }
    kept
}

impl Crossing {
    fn pen(&self) -> Penetration {
        Penetration {
            round: self.round,
            size: Some(self.size),
            tag: Some("P".into()),
            ..Default::default()
        }
    }

    /// Its label: "P Ø230 VZT", through a wall with its bottom
    /// "P Ø230 VZT sp. 2 450".
    pub fn label(&self) -> String {
        let p = self.pen();
        let size = p.slab_label();
        match self.through {
            Through::Wall => {
                let n = (self.sill * 1000.0).round() as i64;
                let s = n.abs().to_string();
                let mut sp = String::new();
                for (i, ch) in s.chars().enumerate() {
                    if i > 0 && (s.len() - i) % 3 == 0 {
                        sp.push(' ');
                    }
                    sp.push(ch);
                }
                let sign = if n < 0 { "-" } else { "" };
                format!("{size} {} sp. {sign}{sp}", self.code)
            }
            _ => format!("{size} {}", self.code),
        }
    }

    /// Its outline in plan: round or the rectangle, square to a wall.
    fn outline(&self) -> Vec<DVec2> {
        let c = self.at.truncate();
        match self.through {
            Through::Wall => vec![],
            _ => self.pen().slab_outline(c),
        }
    }
}

/// The level right above `level`, if any.
fn level_above(model: &Model, level: ElementId) -> Option<ElementId> {
    let here = model.level(level)?.elevation;
    model
        .levels()
        .map(|(e, l)| (e.id, l.elevation))
        .filter(|(_, z)| *z > here + 1e-6)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

const SIZE: f64 = 2.0;

fn text(id: ElementId, at: DVec2, text: String, mm: f64) -> PlanItem {
    PlanItem::Text {
        id,
        at,
        text,
        size: SIZE,
        align: crate::paper::Anchor::Middle,
        mm,
        rgb: None,
        bold: false,
    }
}

/// A label up and right of a hole's outline `ring`, clear of the room's
/// name a valve in the middle of the room would sit on.
fn beside(id: ElementId, ring: &[DVec2], label: String, mm: f64) -> PlanItem {
    // a plain label: the plan's declutter moves it off the lines and the
    // other text it would sit on
    let hi = ring.iter().fold(DVec2::splat(f64::MIN), |m, q| m.max(*q));
    let w = crate::paper::text_width(&label, 2.5) * mm;
    PlanItem::Label {
        id,
        at: hi + DVec2::new(w / 2.0 + 1.0 * mm, 2.0 * mm),
        text: label,
    }
}

/// The trades' penetrations in a floor plan of `level` cut at
/// `cut_height`, drawn as the Prestup tool draws its own, at the paper size
/// of a drawing of `mm` metres per paper millimetre: through the walls the
/// plan cuts (crossed where the cut passes it, dashed and dotted above,
/// dashed below), through the floor (crossed) and through the slab or
/// suspended ceiling over the storey (dash and two dots, "(strop)" or
/// "(podhľad)"). A structural plan shows only what passes load-bearing
/// walls and slabs.
pub fn plan_items(
    model: &Model,
    level: ElementId,
    cut_height: f64,
    mm: f64,
    structural: bool,
) -> Vec<PlanItem> {
    let Some(here) = model.level(level).map(|l| l.elevation) else {
        return vec![];
    };
    let above = level_above(model, level);
    let top = above
        .and_then(|l| model.level(l))
        .map_or(f64::MAX, |l| l.elevation);
    let mut out = vec![];
    for c in crossings(model) {
        if structural && !c.bearing {
            continue;
        }
        match c.through {
            Through::Wall => {
                let Some(ElementKind::Wall(w)) = model.get(c.host).map(|h| &h.kind) else {
                    continue;
                };
                // the walls of this storey
                if c.at.z < here - 1e-6 || c.at.z > top + 1e-6 {
                    continue;
                }
                if crate::plan::wall_rise(model, w, level, cut_height).is_none() {
                    continue;
                }
                let cut = here + cut_height;
                let (lo, hi) = (c.at.z - c.size[1] / 2.0, c.at.z + c.size[1] / 2.0);
                let pen = if lo > cut {
                    Pen::Overhead
                } else if hi < cut {
                    Pen::Hidden
                } else {
                    Pen::Symbol
                };
                let n = c.dir.perp();
                let half = w.thickness / 2.0;
                let mid = c.at.truncate();
                let (mid, n) = {
                    // onto the wall's centreline
                    let (u, _) = w.along(mid);
                    (w.place(u, 0.0), n)
                };
                let hw = c.size[0] / 2.0;
                let ring = vec![
                    mid - c.dir * hw - n * half,
                    mid + c.dir * hw - n * half,
                    mid + c.dir * hw + n * half,
                    mid - c.dir * hw + n * half,
                ];
                if pen == Pen::Symbol {
                    out.extend(crate::plan::hole_marks(c.host, &ring, pen));
                } else {
                    out.push(PlanItem::Polygon {
                        id: c.host,
                        pen,
                        points: ring,
                    });
                }
                let gap = 1.5 * mm;
                let at = if n.x.abs() > 0.7 {
                    mid + n * (half + gap + SIZE * 2.0 * mm)
                } else {
                    mid + n * (half + gap + SIZE * 0.6 * mm)
                };
                let mut t = text(c.host, at, c.label(), mm);
                if n.x.abs() > 0.7 {
                    if let PlanItem::Text { align, at, .. } = &mut t {
                        *at = mid + n * (half + gap);
                        *align = if n.x > 0.0 {
                            crate::paper::Anchor::Start
                        } else {
                            crate::paper::Anchor::End
                        };
                    }
                }
                out.push(t);
            }
            Through::Slab | Through::Ceiling | Through::Roof => {
                // in its own floor: cut, crossed; in what is over the
                // storey: above the cut
                let own_floor = c.through == Through::Slab && c.host_level == level;
                let overhead = match c.through {
                    Through::Slab => Some(c.host_level) == above,
                    Through::Ceiling => c.host_level == level,
                    // the roof plan shows where it comes out
                    Through::Roof => false,
                    Through::Wall => false,
                };
                if !own_floor && !overhead {
                    continue;
                }
                let pen = if own_floor {
                    Pen::Symbol
                } else {
                    Pen::Overhead
                };
                let ring = c.outline();
                let p = c.pen();
                out.push(PlanItem::Polygon {
                    id: c.host,
                    pen,
                    points: ring.clone(),
                });
                for w in p.slab_mark(c.at.truncate()).windows(2) {
                    out.push(PlanItem::Line {
                        id: c.host,
                        pen,
                        a: w[0],
                        b: w[1],
                    });
                }
                let mut label = c.label();
                match (overhead, c.through) {
                    (true, Through::Ceiling) => label.push_str(" (podhľad)"),
                    (true, _) => label.push_str(" (strop)"),
                    _ => {}
                }
                out.push(beside(c.host, &ring, label, mm));
            }
        }
    }
    out
}

/// The holes the trades' pipes make in the suspended ceilings of `level`,
/// in its ceiling plan (looking up at them: cut, crossed) with their
/// labels.
pub fn ceiling_plan_items(model: &Model, level: ElementId, mm: f64) -> Vec<PlanItem> {
    let mut out = vec![];
    for c in crossings(model) {
        if c.host_level != level || !matches!(c.through, Through::Ceiling | Through::Slab) {
            continue;
        }
        if c.through == Through::Slab && level_above(model, level) != Some(c.host_level) {
            // a slab of this level is its floor, not over it
            continue;
        }
        let ring = c.outline();
        out.extend(crate::plan::hole_marks(c.host, &ring, Pen::Symbol));
        out.push(beside(c.host, &ring, c.label(), mm));
    }
    out
}

/// The trades' pipes coming out through the roofs, in a plan from above
/// (the roof plan): the pipe's outline where it stands out of the roof and
/// "VZT Ø160 – vyvedenie nad strechu".
pub fn roof_items(model: &Model, mm: f64) -> Vec<PlanItem> {
    let mut out = vec![];
    for c in crossings(model) {
        if c.through != Through::Roof {
            continue;
        }
        let Some(ElementKind::Pipe(p)) = model.get(c.pipe).map(|e| &e.kind) else {
            continue;
        };
        let centre = c.at.truncate();
        let (round, size, _) = hole_of(p);
        let shown = Penetration {
            round,
            size: Some(size),
            ..Default::default()
        };
        let ring = shown.slab_outline(centre);
        out.push(PlanItem::Polygon {
            id: c.host,
            pen: Pen::Projection,
            points: ring.clone(),
        });
        out.push(beside(
            c.host,
            &ring,
            format!("{} {} – vyvedenie nad strechu", c.code, c.pipe_size),
            mm,
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::mep::Pipe;
    use creabim_core::{Ceiling, Element, Level, Wall};

    fn duct(level: ElementId, a: DVec3, b: DVec3) -> Element {
        Element::new(
            "duct",
            ElementKind::Pipe(Pipe {
                level,
                system: "VZO".into(),
                start: a,
                end: b,
                pipe_type: "VZK".into(),
                nominal_mm: 160.0,
                outer_mm: 160.0,
                inner_mm: 160.0,
                height_mm: None,
                insulation_mm: 30.0,
                insulation_set: true,
                mark: None,
            }),
        )
    }

    #[test]
    fn a_duct_through_a_wall_and_a_suspended_ceiling_is_marked_in_the_plan() {
        let mut m = Model::new();
        let lv = Element::new(
            "1. NP",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let l = lv.id;
        let mut w = Wall::new(l, DVec2::new(2.0, -2.0), DVec2::new(2.0, 2.0), 0.3, 3.0);
        w.bearing = Some(true);
        let wall = Element::new("w", ElementKind::Wall(w));
        let ceiling = Element::new(
            "c",
            ElementKind::Ceiling(Ceiling {
                level: l,
                boundary: vec![
                    DVec2::new(-5.0, -5.0),
                    DVec2::new(5.0, -5.0),
                    DVec2::new(5.0, 5.0),
                    DVec2::new(-5.0, 5.0),
                ],
                room: None,
                height: 2.6,
                thickness: 0.1,
                material: None,
                grid: 0.0,
                assembly: None,
                roof_space: None,
            }),
        );
        // up through the ceiling, across through the wall
        creabim_core::Transaction::new("t")
            .insert(lv)
            .insert(wall)
            .insert(ceiling)
            .insert(duct(
                l,
                DVec3::new(0.0, 0.0, 2.4),
                DVec3::new(0.0, 0.0, 2.8),
            ))
            .insert(duct(
                l,
                DVec3::new(0.0, 0.0, 2.4),
                DVec3::new(4.0, 0.0, 2.4),
            ))
            .apply(&mut m)
            .unwrap();
        let c = crossings(&m);
        assert_eq!(c.len(), 2, "{c:?}");
        let wall = c.iter().find(|c| c.through == Through::Wall).unwrap();
        // Ø160 with 30 mm insulation: a Ø220 hole, its bottom at 2 290
        assert_eq!(wall.size, [0.22, 0.22]);
        assert_eq!(wall.label(), "P Ø220 VZT sp. 2 290");
        let items = plan_items(&m, l, 1.2, 0.05, false);
        let texts: Vec<_> = items
            .iter()
            .filter_map(|i| match i {
                PlanItem::Text { text, .. } | PlanItem::Label { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            texts.contains(&"P Ø220 VZT (podhľad)".to_string()),
            "{texts:?}"
        );
        assert!(
            texts.contains(&"P Ø220 VZT sp. 2 290".to_string()),
            "{texts:?}"
        );
        // a structural plan: the load-bearing wall's hole, never the
        // suspended ceiling's
        let st: Vec<_> = plan_items(&m, l, 1.2, 0.05, true)
            .into_iter()
            .filter_map(|i| match i {
                PlanItem::Text { text, .. } | PlanItem::Label { text, .. } => Some(text),
                _ => None,
            })
            .collect();
        assert_eq!(st, vec!["P Ø220 VZT sp. 2 290".to_string()]);
    }
}
