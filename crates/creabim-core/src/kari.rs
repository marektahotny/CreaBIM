//! Welded mesh (KARI) laid in a slab: a foundation base slab mostly, at
//! its bottom edge (George 2026-11-27: "KARI rohož KY50 8x150/8x150 mm
//! pri spodnom okraji podkladnej dosky"), any slab can have it.
//!
//! The meshes and their weights are WAGNER SERVICECENTRUM s.r.o.'s table
//! "Zvárané siete do betónu z rebrovanej ocele (BSt 500 M) – typy KARI
//! sietí" (wagnerxr.eu, the sheet George sent): wire, mesh, sheet size,
//! kg/m² and kg a sheet.
//!
//! Sheets are laid lapped: by default 300 mm both ways, two meshes of a
//! 150 mm sheet ("Štandardná prax odporúča, aby sa siete prekladali
//! minimálne cez tri oká (cca 30 cm)", atreon.sk, Pokládka KARI sietí;
//! eko-vystuze.sk, Výpočet KARI sietí: 1 oko 150 mm, 2 oká 300 mm, 3 oká
//! 450 mm, "Dĺžka styku je projektový údaj", its example lapped 0,30 m).
//! The count is eko-vystuze.sk's: along each side
//! n = ⌈(side − lap) ÷ (sheet − lap)⌉, a sheet for each place of that
//! grid the slab reaches into.

use crate::{ElementId, ElementKind, LayerFunction, Model, Slab};
use glam::DVec2;
use serde::{Deserialize, Serialize};

/// A mesh of the maker's table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeshType {
    /// The maker's type: "KY 50".
    pub code: &'static str,
    /// The wires' diameter, mm (both ways the same in the table).
    pub wire: f64,
    /// The mesh, mm (square).
    pub pitch: f64,
    /// The sheet's width and length, m.
    pub sheet: (f64, f64),
    /// Weight of a square metre, kg.
    pub kg_m2: f64,
    /// Weight of a sheet, kg.
    pub kg_sheet: f64,
}

const fn t(
    code: &'static str,
    wire: f64,
    pitch: f64,
    sheet: (f64, f64),
    kg_m2: f64,
    kg_sheet: f64,
) -> MeshType {
    MeshType {
        code,
        wire,
        pitch,
        sheet,
        kg_m2,
        kg_sheet,
    }
}

/// WAGNER SERVICECENTRUM s.r.o., typy KARI sietí (BSt 500 M), as listed.
pub const CATALOGUE: [MeshType; 22] = [
    t("KA 16", 4.0, 100.0, (2.0, 3.0), 1.98, 11.88),
    t("KA 17", 4.0, 150.0, (2.0, 3.0), 1.35, 8.12),
    t("KA 18", 4.0, 200.0, (2.0, 3.0), 0.99, 5.94),
    t("KD 35", 5.0, 100.0, (2.0, 3.0), 3.08, 18.50),
    t("KD 37", 5.0, 150.0, (2.0, 3.0), 2.11, 12.63),
    t("KD 36", 5.0, 200.0, (2.0, 3.0), 1.54, 9.24),
    t("KH 30", 6.0, 100.0, (2.0, 3.0), 4.44, 26.64),
    t("KH 20", 6.0, 150.0, (2.0, 3.0), 3.03, 18.20),
    t("KH 31", 6.0, 200.0, (2.0, 3.0), 2.22, 13.30),
    t("KY 49", 8.0, 100.0, (2.0, 3.0), 7.90, 47.40),
    t("KY 50", 8.0, 150.0, (2.0, 3.0), 5.40, 32.40),
    t("KY 51", 8.0, 200.0, (2.0, 3.0), 3.95, 23.70),
    t("KZ 60", 10.0, 100.0, (2.0, 3.0), 12.34, 74.04),
    t("KZ 70", 10.0, 150.0, (2.0, 3.0), 8.44, 50.60),
    t("KZ 71", 10.0, 200.0, (2.0, 3.0), 6.17, 37.02),
    t("Q 188", 6.0, 150.0, (2.3, 6.0), 3.02, 41.70),
    t("Q 335", 8.0, 150.0, (2.3, 6.0), 5.38, 74.30),
    t("AQ 50", 5.0, 100.0, (2.4, 6.0), 3.08, 44.40),
    t("AQ 60", 6.0, 100.0, (2.4, 6.0), 4.44, 63.94),
    t("KY 85", 8.0, 100.0, (2.4, 6.0), 7.90, 113.80),
    t("KY 14", 8.0, 150.0, (2.4, 6.0), 5.27, 75.84),
    t("KZ 100", 10.0, 100.0, (2.4, 6.0), 12.29, 177.70),
];

/// The mesh George lays by default: KY 50, 8/150 × 8/150.
pub const DEFAULT_MESH: &str = "KY 50";
/// The lap by default, m (see the module's sources).
pub const DEFAULT_LAP: f64 = 0.30;

fn default_lap() -> f64 {
    DEFAULT_LAP
}

/// The mesh a slab is reinforced with.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SlabMesh {
    /// The maker's type, as [`MeshType::code`].
    pub mesh: String,
    /// How far sheets lap one another, both ways, m.
    #[serde(default = "default_lap")]
    pub lap: f64,
    /// A second mesh, the same, at the slab's top edge too (George
    /// 2026-11-27: a slab against water under pressure).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub top: bool,
}

impl Default for SlabMesh {
    fn default() -> Self {
        Self {
            mesh: DEFAULT_MESH.into(),
            lap: DEFAULT_LAP,
            top: false,
        }
    }
}

/// The mesh of the table called `code`.
pub fn mesh(code: &str) -> Option<&'static MeshType> {
    let key = |s: &str| s.replace(' ', "").to_uppercase();
    CATALOGUE.iter().find(|m| key(m.code) == key(code))
}

fn num(v: f64) -> String {
    if (v - v.round()).abs() < 1e-9 {
        format!("{v:.0}")
    } else {
        format!("{v}").replace('.', ",")
    }
}

impl MeshType {
    /// "KY50 8x150/8x150 mm", as George's legend writes it.
    pub fn designation(&self) -> String {
        let (d, p) = (num(self.wire), num(self.pitch));
        format!("{} {d}x{p}/{d}x{p} mm", self.code.replace(' ', ""))
    }
}

/// The outline the mesh lies in: the slab's structural layer (the lowest
/// one, the base slab of a floor on the ground) or the slab's own, with
/// the holes through it.
pub fn outline(slab: &Slab, model: &Model) -> (Vec<DVec2>, Vec<Vec<DVec2>>) {
    let layer = slab
        .assembly
        .and_then(|a| model.get(a))
        .and_then(|e| match &e.kind {
            ElementKind::Assembly(a) => a
                .layers
                .iter()
                .rposition(|l| l.function == LayerFunction::Structure),
            _ => None,
        });
    let out = match layer {
        Some(i) => slab.layer_outline(i),
        None => slab.outline(),
    };
    (out, slab.holes.clone())
}

fn area(p: &[DVec2]) -> f64 {
    (crate::polygon::signed_area2(p) / 2.0).abs()
}

/// The part of `poly` inside the box `lo`–`hi` (Sutherland–Hodgman; a
/// concave polygon may come out with seams of no width, which add no
/// area).
fn clip(poly: &[DVec2], lo: DVec2, hi: DVec2) -> Vec<DVec2> {
    let mut out = poly.to_vec();
    for (axis, bound, keep_above) in [
        (0, lo.x, true),
        (0, hi.x, false),
        (1, lo.y, true),
        (1, hi.y, false),
    ] {
        if out.is_empty() {
            break;
        }
        let inside = |p: DVec2| {
            let v = if axis == 0 { p.x } else { p.y };
            if keep_above {
                v >= bound
            } else {
                v <= bound
            }
        };
        let cross = |a: DVec2, b: DVec2| {
            let (va, vb) = if axis == 0 { (a.x, b.x) } else { (a.y, b.y) };
            let t = (bound - va) / (vb - va);
            a + (b - a) * t
        };
        let input = std::mem::take(&mut out);
        for i in 0..input.len() {
            let (a, b) = (input[i], input[(i + 1) % input.len()]);
            match (inside(a), inside(b)) {
                (true, true) => out.push(b),
                (true, false) => out.push(cross(a, b)),
                (false, true) => {
                    out.push(cross(a, b));
                    out.push(b);
                }
                (false, false) => {}
            }
        }
    }
    out
}

/// One slab's (or several touching slabs') mesh: what the take-off and
/// the legend list.
#[derive(Clone, Debug, PartialEq)]
pub struct Laid {
    pub mesh: MeshType,
    pub lap: f64,
    pub level: ElementId,
    /// At the slab's top edge (else at its bottom edge).
    pub top: bool,
    pub slabs: Vec<ElementId>,
    /// The area reinforced, m² (the slabs' outlines less their holes).
    pub area: f64,
    /// Sheets laid, lapped.
    pub pieces: usize,
}

impl Laid {
    /// Weight of the sheets laid, kg.
    pub fn kg(&self) -> f64 {
        self.pieces as f64 * self.mesh.kg_sheet
    }
}

/// Sheets to cover `parts` (outlines, each with its holes) lapped `lap`:
/// a grid from the parts' lower left corner, the sheets' length along
/// the longer side of their box; each place counts when the slab reaches
/// into what that sheet adds beyond its neighbours' laps.
pub fn sheets(parts: &[(Vec<DVec2>, Vec<Vec<DVec2>>)], m: &MeshType, lap: f64) -> usize {
    let pts = parts.iter().flat_map(|p| p.0.iter());
    let (lo, hi) = pts.fold(
        (DVec2::splat(f64::INFINITY), DVec2::splat(f64::NEG_INFINITY)),
        |(lo, hi), p| (lo.min(*p), hi.max(*p)),
    );
    if !(hi.x > lo.x && hi.y > lo.y) {
        return 0;
    }
    let size = hi - lo;
    let (w, l) = (m.sheet.0.min(m.sheet.1), m.sheet.0.max(m.sheet.1));
    let sheet = if size.x >= size.y {
        DVec2::new(l, w)
    } else {
        DVec2::new(w, l)
    };
    let lap = lap.clamp(0.0, w * 0.5);
    let step = sheet - DVec2::splat(lap);
    let n = |s: f64, st: f64| (((s - lap) / st).ceil().max(1.0)) as usize;
    let (nx, ny) = (n(size.x, step.x), n(size.y, step.y));
    let mut count = 0;
    for i in 0..nx {
        for j in 0..ny {
            // what this sheet adds past the one before it each way
            let a = lo
                + DVec2::new(
                    i as f64 * step.x + if i > 0 { lap } else { 0.0 },
                    j as f64 * step.y + if j > 0 { lap } else { 0.0 },
                );
            let b = lo + DVec2::new(i as f64 * step.x, j as f64 * step.y) + sheet;
            let covered: f64 = parts
                .iter()
                .map(|(o, holes)| {
                    area(&clip(o, a, b)) - holes.iter().map(|h| area(&clip(h, a, b))).sum::<f64>()
                })
                .sum();
            if covered > 1e-6 {
                count += 1;
            }
        }
    }
    count
}

/// The meshes laid in the model's slabs: a line for each mesh on each
/// storey, slabs touching one another laid as one.
pub fn laid(model: &Model) -> Vec<Laid> {
    struct One {
        id: ElementId,
        level: ElementId,
        mesh: MeshType,
        lap: f64,
        top: bool,
        part: (Vec<DVec2>, Vec<Vec<DVec2>>),
        lo: DVec2,
        hi: DVec2,
    }
    let mut ones: Vec<One> = vec![];
    for e in model.iter() {
        let ElementKind::Slab(s) = &e.kind else {
            continue;
        };
        let Some(r) = &s.reinforcement else { continue };
        // a base slab's only: a floor slab or a foundation slab without
        // footings gets tied bars (George 2026-11-27)
        if crate::slab::kind_of(model, s) != crate::SlabKind::Base {
            continue;
        }
        let Some(m) = mesh(&r.mesh) else { continue };
        let part = outline(s, model);
        let (lo, hi) = part.0.iter().fold(
            (DVec2::splat(f64::INFINITY), DVec2::splat(f64::NEG_INFINITY)),
            |(lo, hi), p| (lo.min(*p), hi.max(*p)),
        );
        ones.push(One {
            id: e.id,
            level: s.level,
            mesh: *m,
            lap: r.lap,
            top: r.top,
            part,
            lo,
            hi,
        });
    }
    // slabs of one mesh on a storey whose boxes touch: one surface
    let mut group: Vec<usize> = (0..ones.len()).collect();
    fn root(g: &mut [usize], i: usize) -> usize {
        let mut r = i;
        while g[r] != r {
            r = g[r];
        }
        g[i] = r;
        r
    }
    for i in 0..ones.len() {
        for j in i + 1..ones.len() {
            let (a, b) = (&ones[i], &ones[j]);
            let same =
                a.level == b.level && a.mesh.code == b.mesh.code && (a.lap - b.lap).abs() < 1e-9;
            let touch = a.lo.x <= b.hi.x + 1e-3
                && b.lo.x <= a.hi.x + 1e-3
                && a.lo.y <= b.hi.y + 1e-3
                && b.lo.y <= a.hi.y + 1e-3;
            if same && touch {
                let (ri, rj) = (root(&mut group, i), root(&mut group, j));
                group[ri] = rj;
            }
        }
    }
    let mut out: Vec<Laid> = vec![];
    let mut done: Vec<usize> = vec![];
    for i in 0..ones.len() {
        let r = root(&mut group, i);
        if done.contains(&r) {
            continue;
        }
        done.push(r);
        let members: Vec<&One> = (0..ones.len())
            .filter(|&k| root(&mut group, k) == r)
            .map(|k| &ones[k])
            .collect();
        // the bottom mesh in all of them, the top one where asked
        for top in [false, true] {
            let these: Vec<&&One> = members.iter().filter(|o| !top || o.top).collect();
            if these.is_empty() {
                continue;
            }
            let parts: Vec<_> = these.iter().map(|o| o.part.clone()).collect();
            let area: f64 = parts
                .iter()
                .map(|(o, h)| area(o) - h.iter().map(|x| area(x)).sum::<f64>())
                .sum();
            let first = these[0];
            let pieces = sheets(&parts, &first.mesh, first.lap);
            // the same mesh elsewhere on the storey: summed into its line
            if let Some(l) = out.iter_mut().find(|l| {
                l.level == first.level
                    && l.top == top
                    && l.mesh.code == first.mesh.code
                    && (l.lap - first.lap).abs() < 1e-9
            }) {
                l.area += area;
                l.pieces += pieces;
                l.slabs.extend(these.iter().map(|o| o.id));
                continue;
            }
            out.push(Laid {
                mesh: first.mesh,
                lap: first.lap,
                level: first.level,
                top,
                slabs: these.iter().map(|o| o.id).collect(),
                area,
                pieces,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(w: f64, h: f64) -> Vec<DVec2> {
        vec![
            DVec2::ZERO,
            DVec2::new(w, 0.0),
            DVec2::new(w, h),
            DVec2::new(0.0, h),
        ]
    }

    /// eko-vystuze.sk's example: 10 × 10 m, sheets 3 × 2 m lapped 0,30 m:
    /// 4 × 6 = 24 sheets.
    #[test]
    fn a_ten_metre_square_takes_24_sheets_lapped_300() {
        let m = mesh("KY50").unwrap();
        assert_eq!(m.kg_sheet, 32.40);
        assert_eq!(m.designation(), "KY50 8x150/8x150 mm");
        assert_eq!(sheets(&[(rect(10.0, 10.0), vec![])], m, 0.30), 24);
        // one sheet exactly
        assert_eq!(sheets(&[(rect(3.0, 2.0), vec![])], m, 0.30), 1);
        // a hair over: a second row
        assert_eq!(sheets(&[(rect(3.0, 2.01), vec![])], m, 0.30), 2);
    }

    /// An L leaves out the grid's places it does not reach.
    #[test]
    fn an_l_shaped_slab_skips_the_empty_corner() {
        let m = mesh("KY 50").unwrap();
        let l = vec![
            DVec2::ZERO,
            DVec2::new(10.0, 0.0),
            DVec2::new(10.0, 3.0),
            DVec2::new(3.0, 3.0),
            DVec2::new(3.0, 10.0),
            DVec2::new(0.0, 10.0),
        ];
        let n = sheets(&[(l, vec![])], m, 0.30);
        assert!(n < 24 && n > 8, "{n}");
    }
}
