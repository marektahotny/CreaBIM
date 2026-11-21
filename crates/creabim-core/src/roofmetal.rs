//! The rest of the roof's sheet-metal work (George 2026-11-21, phase 2):
//! the capping of a flat roof's parapet (oplechovanie atiky) on an OSB
//! board laid on XPS in a fall towards the roof, with the corner strip
//! at the parapet's foot (kútová lišta) and the parapet's insulation
//! counted; the flashing round a chimney (lemovanie komína); and the
//! roof outlets (strešné vpusty), counted in pieces.

use crate::{ElementId, ElementKind, Model};
use glam::DVec2;
use serde::{Deserialize, Serialize};

/// A parapet's capping, along the whole top of wall `wall`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Capping {
    pub wall: ElementId,
    /// The roof is on the wall's right (looking from its start to its
    /// end) unless this is set; the capping falls towards it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub inside_left: bool,
    /// The sheet's leg down each face of the parapet, m.
    pub leg: f64,
    /// The drip turned out at the foot of each leg, m.
    pub drip: f64,
    /// The OSB board the sheet is fixed to, m thick.
    pub osb: f64,
    /// The XPS under the board at its thin (inner) edge, m.
    pub xps: f64,
    /// The fall towards the roof, %.
    pub fall: f64,
    /// With a corner strip (kútová lišta) at the parapet's inner foot.
    #[serde(default = "yes")]
    pub corner_strip: bool,
    /// The corner strip's developed width, mm.
    #[serde(default = "default_strip")]
    pub strip_mm: f64,
}

fn yes() -> bool {
    true
}

fn default_strip() -> f64 {
    100.0
}

impl Capping {
    /// George's sample (K-4): legs 50 mm with 20 mm drips, on OSB 22 mm
    /// and XPS laid in a 3 % fall.
    pub fn new(wall: ElementId) -> Self {
        Self {
            wall,
            inside_left: false,
            leg: 0.05,
            drip: 0.02,
            osb: 0.022,
            xps: 0.02,
            fall: 3.0,
            corner_strip: true,
            strip_mm: default_strip(),
        }
    }
}

/// Where a capping lies: along the wall's centreline from `a` to `b`,
/// `width` across (the wall with its insulation), the roof towards
/// `inward`, on the wall's top at `z`, the XPS `xps_in` thick at the
/// inner edge and `xps_out` at the outer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CappingPlace {
    pub a: DVec2,
    pub b: DVec2,
    pub width: f64,
    pub inward: DVec2,
    pub z: f64,
    pub xps_in: f64,
    pub xps_out: f64,
}

impl CappingPlace {
    pub fn length(&self) -> f64 {
        self.a.distance(self.b)
    }

    /// The sheet's top at the inner and outer edge.
    pub fn top(&self, c: &Capping) -> (f64, f64) {
        (self.z + self.xps_in + c.osb, self.z + self.xps_out + c.osb)
    }
}

pub fn capping_place(model: &Model, c: &Capping) -> Option<CappingPlace> {
    let ElementKind::Wall(w) = &model.get(c.wall)?.kind else {
        return None;
    };
    if w.is_arc() {
        return None;
    }
    let d = w.direction();
    let right = -d.perp();
    let inward = if c.inside_left { -right } else { right };
    let z = crate::walls::base_z(model, w) + w.height;
    let xps_out = c.xps + w.thickness * c.fall / 100.0;
    Some(CappingPlace {
        a: w.start,
        b: w.end,
        width: w.thickness,
        inward,
        z,
        xps_in: c.xps,
        xps_out,
    })
}

/// Is the roof to the wall's left (looking from its start to its end)?
/// The side just past the wall that a roof or a slab covers, below the
/// wall's top; the right when neither or both are.
pub fn roof_on_left(model: &Model, wall: ElementId) -> bool {
    let Some(ElementKind::Wall(w)) = model.get(wall).map(|e| &e.kind) else {
        return false;
    };
    let top = crate::walls::base_z(model, w) + w.height;
    let mid = (w.start + w.end) / 2.0;
    let left = w.direction().perp();
    let covered = |p: DVec2| {
        model.iter().any(|e| match &e.kind {
            ElementKind::Roof(r) => {
                crate::polygon::contains(&r.eave_outline(), p) && r.base(model) < top
            }
            ElementKind::Slab(s) => {
                crate::polygon::contains(&s.boundary, p)
                    && crate::members::elevation(model, s.level) + s.top_offset < top
            }
            _ => false,
        })
    };
    let off = w.thickness / 2.0 + 0.3;
    covered(mid + left * off) && !covered(mid - left * off)
}

/// The wall under `p` whose top is highest (a parapet over its roof).
pub fn parapet_at(model: &Model, p: DVec2) -> Option<ElementId> {
    model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Wall(w) if !w.is_arc() && w.curtain.is_none() => {
                let (_, s) = w.along(p);
                (s <= w.thickness / 2.0 + 0.05)
                    .then_some((crate::walls::base_z(model, w) + w.height, e.id))
            }
            _ => None,
        })
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .map(|x| x.1)
}

/// The part under `p` pulled up from a plan outline (a chimney).
pub fn chimney_at(model: &Model, p: DVec2) -> Option<ElementId> {
    model.iter().find_map(|e| {
        let o = chimney_outline(model, e.id)?;
        crate::polygon::contains(&o, p).then_some(e.id)
    })
}

/// The capping's developed width, mm: the top across the wall (on its
/// fall), a leg and a drip each side; George's K-4: 490 + 2 × 50 + 2 ×
/// 20 = 630.
pub fn capping_developed(model: &Model, c: &Capping) -> f64 {
    let w = capping_place(model, c).map_or(0.0, |p| p.width);
    let top = w * (1.0 + (c.fall / 100.0).powi(2)).sqrt();
    ((top + 2.0 * (c.leg + c.drip)) * 1000.0).round()
}

/// The height of the parapet's inner face over the roof inside it, m:
/// from its top down to the highest roof or slab top just inside it.
pub fn parapet_inner_height(model: &Model, c: &Capping) -> f64 {
    let Some(p) = capping_place(model, c) else {
        return 0.0;
    };
    let probe = (p.a + p.b) / 2.0 + p.inward * (p.width / 2.0 + 0.3);
    let mut best: Option<f64> = None;
    for e in model.iter() {
        let top = match &e.kind {
            ElementKind::Roof(r) => {
                if !crate::polygon::contains(&r.eave_outline(), probe) {
                    continue;
                }
                r.base(model) + r.underside_at(probe) + r.vertical_thickness()
            }
            ElementKind::Slab(s) => {
                if !crate::polygon::contains(&s.boundary, probe) {
                    continue;
                }
                crate::members::elevation(model, s.level) + s.top_offset
            }
            _ => continue,
        };
        if top < p.z + 1e-6 && best.is_none_or(|b| top > b) {
            best = Some(top);
        }
    }
    best.map_or(0.0, |b| p.z - b)
}

/// The insulation of a parapet: its inner face up from the roof, and its
/// top under the capping (George's sample under the table), m².
pub fn parapet_insulation(model: &Model, c: &Capping) -> (f64, f64) {
    let Some(p) = capping_place(model, c) else {
        return (0.0, 0.0);
    };
    let len = p.length();
    (parapet_inner_height(model, c) * len, p.width * len)
}

/// The flashing round a chimney (part `part`) where it comes through
/// the roof.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChimneyFlashing {
    pub part: ElementId,
    /// Up the chimney's face, m.
    pub upstand: f64,
    /// Out over the roof, m.
    pub apron: f64,
}

impl ChimneyFlashing {
    pub fn new(part: ElementId) -> Self {
        Self {
            part,
            upstand: 0.15,
            apron: 0.15,
        }
    }
}

/// The chimney's outline in plan, if `part` is one pulled up from a plan
/// outline.
pub fn chimney_outline(model: &Model, part: ElementId) -> Option<Vec<DVec2>> {
    let ElementKind::Part(p) = &model.get(part)?.kind else {
        return None;
    };
    match &p.form {
        crate::parts::Form::Extrusion {
            outline,
            plane: None,
            ..
        } if outline.len() >= 3 => Some(outline.clone()),
        _ => None,
    }
}

/// The roof's top at `p`, of the roof that covers it.
pub fn roof_top_at(model: &Model, p: DVec2) -> Option<f64> {
    model.iter().find_map(|e| match &e.kind {
        ElementKind::Roof(r) if crate::polygon::contains(&r.eave_outline(), p) => {
            Some(r.base(model) + r.underside_at(p) + r.vertical_thickness())
        }
        _ => None,
    })
}

/// Round the chimney, m: its outline's perimeter.
pub fn chimney_length(model: &Model, c: &ChimneyFlashing) -> f64 {
    chimney_outline(model, c.part).map_or(0.0, |o| {
        (0..o.len())
            .map(|i| o[i].distance(o[(i + 1) % o.len()]))
            .sum()
    })
}

/// A roof outlet (strešný vpust), counted in pieces.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoofOutlet {
    /// Where, in plan.
    pub at: DVec2,
    /// Its nominal size, DN.
    pub dn: f64,
    /// What it is ("PA6"), for the table.
    pub kind: String,
}

impl RoofOutlet {
    /// George's K-3: PA6 DN 110.
    pub fn new(at: DVec2) -> Self {
        Self {
            at,
            dn: 110.0,
            kind: "PA6".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_parapet_capping_comes_out_as_georges() {
        // a 490 mm parapet: 630 mm developed (his K-4), its XPS thicker
        // to the outside
        let mut m = Model::new();
        let lv = crate::Element::new(
            "L",
            ElementKind::Level(crate::Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let lid = lv.id;
        let w = crate::Wall::new(lid, DVec2::ZERO, DVec2::X * 10.0, 0.49, 3.5);
        let we = crate::Element::new("W", ElementKind::Wall(w));
        let wid = we.id;
        crate::Transaction::new("t")
            .insert(lv)
            .insert(we)
            .apply(&mut m)
            .unwrap();
        let mut c = Capping::new(wid);
        c.fall = 0.0;
        assert_eq!(capping_developed(&m, &c), 630.0);
        c.fall = 3.0;
        let p = capping_place(&m, &c).unwrap();
        assert!((p.z - 3.5).abs() < 1e-9);
        assert!(p.xps_out > p.xps_in);
        assert!((p.length() - 10.0).abs() < 1e-9);
        // the roof to the wall's right
        assert!((p.inward - DVec2::new(0.0, -1.0)).length() < 1e-9);
    }
}
