//! Earthworks, ported from PilonDesigner's `earthworks.ts`:
//!
//! - a **building pit** is dug to its bottom with sides sloping 1:n up
//!   to the ground, and back-filled once the building stands: the ground
//!   stays as it was;
//! - a **platform** levels the ground at its height with slopes 1:n down
//!   or up to the ground round it: the ground changes for good, cut where
//!   it was higher and filled where it was lower.
//!
//! The schedule of earthworks takes off topsoil stripping, the pit and
//! the cut of platforms, trenches under strip footings and pits under pad
//! footings, moving the spoil, back-fill, fill and what is left to take
//! away, with the item codes of the KROS/Cenkros earthworks (1xx) as a
//! guide. Volumes are the difference of the ground before and after, in
//! cells of 0.5 m.

use crate::footing::FootingKind;
use crate::polygon::contains;
use crate::site::{Site, SiteForm};
use crate::{members, ElementId, ElementKind, Model};
use glam::{DVec2, DVec3};

/// The cells volumes are summed over, metres.
pub const CELL: f64 = 0.5;

/// A terrain's surface, heights absolute (the model's z): its
/// triangulation through the spot heights, as drawn, so a volume is
/// worked out on the surface that is shown.
pub struct Ground {
    verts: Vec<DVec3>,
    tris: Vec<[usize; 3]>,
    /// Triangles by grid cell, to find the one under a point quickly.
    cells: Vec<Vec<usize>>,
    lo: DVec2,
    cell: f64,
    nx: usize,
    ny: usize,
    pub(crate) z: f64,
}

impl Ground {
    /// The ground of terrain `s` (none for other site elements).
    pub fn of(model: &Model, s: &Site) -> Option<Ground> {
        let SiteForm::Terrain {
            boundary,
            top,
            points,
            ..
        } = &s.form
        else {
            return None;
        };
        let z = members::elevation(model, s.level);
        let inside: Vec<DVec2> = points
            .iter()
            .map(|p| p.truncate())
            .filter(|p| contains(boundary, *p))
            .collect();
        let (v2, tris) = crate::site::tin(boundary, &inside);
        let controls = crate::site::controls(boundary, *top, points);
        let verts: Vec<DVec3> = v2
            .iter()
            .map(|p| p.extend(z + crate::site::surface_height(*top, &controls, *p)))
            .collect();
        let (lo, hi) = bounds(&v2);
        let span = (hi - lo).max(DVec2::splat(1e-6));
        let cell = (span.x * span.y / tris.len().max(1) as f64)
            .sqrt()
            .max(0.05)
            * 2.0;
        let nx = ((span.x / cell).ceil() as usize).max(1);
        let ny = ((span.y / cell).ceil() as usize).max(1);
        let mut cells = vec![vec![]; nx * ny];
        let at = |p: DVec2| {
            let q = (p - lo) / cell;
            ((q.x as usize).min(nx - 1), (q.y as usize).min(ny - 1))
        };
        for (k, t) in tris.iter().enumerate() {
            let pts = t.map(|i| v2[i]);
            let (a, b) = bounds(&pts);
            let ((x0, y0), (x1, y1)) = (at(a), at(b));
            for y in y0..=y1 {
                for x in x0..=x1 {
                    cells[y * nx + x].push(k);
                }
            }
        }
        Some(Ground {
            verts,
            tris,
            cells,
            lo,
            cell,
            nx,
            ny,
            z,
        })
    }

    /// Its height at `p`, absolute; none outside it.
    pub fn at(&self, p: DVec2) -> Option<f64> {
        let q = (p - self.lo) / self.cell;
        if q.x < 0.0 || q.y < 0.0 {
            return None;
        }
        let (x, y) = (q.x as usize, q.y as usize);
        if x >= self.nx || y >= self.ny {
            return None;
        }
        for &k in &self.cells[y * self.nx + x] {
            let [a, b, c] = self.tris[k].map(|i| self.verts[i]);
            let (a2, b2, c2) = (a.truncate(), b.truncate(), c.truncate());
            let d = (b2 - a2).perp_dot(c2 - a2);
            if d.abs() < 1e-18 {
                continue;
            }
            let u = (p - a2).perp_dot(c2 - a2) / d;
            let v = (b2 - a2).perp_dot(p - a2) / d;
            if u >= -1e-9 && v >= -1e-9 && u + v <= 1.0 + 1e-9 {
                return Some(a.z + u * (b.z - a.z) + v * (c.z - a.z));
            }
        }
        None
    }
}

/// An earthwork's numbers: its outline, height (absolute), slope 1:n,
/// whether a pit, and topsoil depth.
#[derive(Clone, Debug, PartialEq)]
pub struct Work {
    pub outline: Vec<DVec2>,
    pub z: f64,
    pub slope: f64,
    pub pit: bool,
    pub topsoil: f64,
}

impl Work {
    pub fn of(model: &Model, s: &Site) -> Option<Work> {
        let SiteForm::Earthwork {
            boundary,
            bottom,
            slope,
            pit,
            topsoil,
        } = &s.form
        else {
            return None;
        };
        Some(Work {
            outline: boundary.clone(),
            z: members::elevation(model, s.level) + bottom,
            slope: *slope,
            pit: *pit,
            topsoil: *topsoil,
        })
    }
}

/// Points round an outline, at most `step` apart, each with its outward
/// direction (at a corner, between its two sides').
pub fn rim(pts: &[DVec2], step: f64) -> Vec<(DVec2, DVec2)> {
    let n = pts.len();
    let ccw = crate::polygon::signed_area2(pts) > 0.0;
    let out_of = |u: DVec2| {
        if ccw {
            DVec2::new(u.y, -u.x)
        } else {
            DVec2::new(-u.y, u.x)
        }
    };
    let mut out = vec![];
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let l = a.distance(b);
        if l < 1e-9 {
            continue;
        }
        let u = (b - a) / l;
        let nrm = out_of(u);
        let prev = pts[(i + n - 1) % n];
        let np = out_of((a - prev).normalize_or_zero());
        out.push((a, (nrm + np).normalize_or(nrm)));
        let k = (l / step).ceil().max(1.0) as usize;
        for j in 1..k {
            out.push((a + u * (l * j as f64 / k as f64), nrm));
        }
    }
    out
}

/// The top edge of an earthwork's slopes: from each rim point, outwards
/// to where the slope 1:n from its height meets the ground. (rim point,
/// top point, ground height there.)
pub fn slope_ring(w: &Work, g: &Ground) -> Vec<(DVec2, DVec2, f64)> {
    let n = w.slope.max(0.0);
    rim(&w.outline, 1.0)
        .into_iter()
        .map(|(q, dir)| {
            let mut s = 0.0;
            let mut zt = g.at(q).unwrap_or(w.z);
            if n > 0.0 {
                while s < 40.0 {
                    let t = q + dir * s;
                    let Some(zo) = g.at(t) else {
                        break;
                    };
                    zt = zo;
                    if (zo - w.z).abs() <= s / n + 1e-6 {
                        break;
                    }
                    s += 0.1;
                }
            }
            (q, q + dir * s, zt)
        })
        .collect()
}

/// A terrain's spot heights (its level's) after the platforms on it: the
/// points under a platform and its slopes give way to the platform's rim,
/// its slopes' top edge and a grid on its top.
pub fn edited_points(model: &Model, terrain: &Site) -> Option<Vec<DVec3>> {
    let SiteForm::Terrain {
        boundary, points, ..
    } = &terrain.form
    else {
        return None;
    };
    let g = Ground::of(model, terrain)?;
    let works: Vec<Work> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Site(s) => Work::of(model, s),
            _ => None,
        })
        .filter(|w| !w.pit && w.outline.iter().any(|p| contains(boundary, *p)))
        .collect();
    if works.is_empty() {
        return None;
    }
    let lz = g.z;
    let mut pts = points.clone();
    for w in &works {
        let ring = slope_ring(w, &g);
        let outer: Vec<DVec2> = ring.iter().map(|r| r.1).collect();
        pts.retain(|b| {
            let p = b.truncate();
            !contains(&w.outline, p) && !(outer.len() > 2 && contains(&outer, p))
        });
        for (q, top, zt) in &ring {
            pts.push(q.extend(w.z - lz));
            if top.distance(*q) > 0.05 {
                pts.push(top.extend(zt - lz));
            }
        }
        let (lo, hi) = bounds(&w.outline);
        let mut x = lo.x + 2.0;
        while x < hi.x - 1.0 {
            let mut y = lo.y + 2.0;
            while y < hi.y - 1.0 {
                let p = DVec2::new(x, y);
                if contains(&w.outline, p) {
                    pts.push(p.extend(w.z - lz));
                }
                y += 3.0;
            }
            x += 3.0;
        }
    }
    Some(pts)
}

fn bounds(pts: &[DVec2]) -> (DVec2, DVec2) {
    pts.iter().fold(
        (DVec2::splat(f64::INFINITY), DVec2::splat(f64::NEG_INFINITY)),
        |(lo, hi), p| (lo.min(*p), hi.max(*p)),
    )
}

/// A pit's surface at `q`: its bottom inside it, its slopes 1:n up to the
/// ground round it, never above the ground.
fn pit_surface(w: &Work, g: &Ground, q: DVec2) -> Option<f64> {
    let zo = g.at(q)?;
    if contains(&w.outline, q) {
        return Some(zo.min(w.z));
    }
    if w.slope <= 0.0 {
        return Some(zo);
    }
    let n = w.outline.len();
    let d = (0..n)
        .map(|i| crate::footing::segment_distance(q, w.outline[i], w.outline[(i + 1) % n]))
        .fold(f64::INFINITY, f64::min);
    Some(zo.min(w.z + d / w.slope))
}

/// Cut and fill between two surfaces over `region`, and the area summed.
fn diff_volume(
    region: &[DVec2],
    before: &dyn Fn(DVec2) -> Option<f64>,
    after: &dyn Fn(DVec2) -> Option<f64>,
) -> (f64, f64, f64) {
    let (lo, hi) = bounds(region);
    let (mut cut, mut fill, mut area) = (0.0, 0.0, 0.0);
    let a = CELL * CELL;
    let mut x = lo.x + CELL / 2.0;
    while x < hi.x {
        let mut y = lo.y + CELL / 2.0;
        while y < hi.y {
            let q = DVec2::new(x, y);
            if contains(region, q) {
                if let (Some(zo), Some(zn)) = (before(q), after(q)) {
                    area += a;
                    if zo > zn {
                        cut += (zo - zn) * a;
                    } else {
                        fill += (zn - zo) * a;
                    }
                }
            }
            y += CELL;
        }
        x += CELL;
    }
    (cut, fill, area)
}

/// One earthwork taken off.
#[derive(Clone, Debug, PartialEq)]
pub struct Taken {
    pub id: ElementId,
    pub pit: bool,
    /// Its outline's area, and the area within its slopes' top edge.
    pub area: f64,
    pub top_area: f64,
    pub cut: f64,
    pub fill: f64,
    pub backfill: f64,
    pub topsoil: f64,
}

/// A line of the schedule of earthworks.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub key: &'static str,
    /// The KROS/Cenkros item code, a guide: the estimate picks the item by
    /// the soil class, the width dug and the haul.
    pub code: &'static str,
    pub text: &'static str,
    pub unit: &'static str,
    pub qty: f64,
    /// How it was worked out.
    pub calc: &'static str,
    pub ids: Vec<ElementId>,
}

/// The schedule of earthworks of a model: its rows, and each earthwork.
pub fn schedule(model: &Model) -> (Vec<Row>, Vec<Taken>) {
    let terrain = model.iter().find_map(|e| match &e.kind {
        ElementKind::Site(s) if matches!(s.form, SiteForm::Terrain { .. }) => Some(s),
        _ => None,
    });
    let works: Vec<(ElementId, Work)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Site(s) => Work::of(model, s).map(|w| (e.id, w)),
            _ => None,
        })
        .collect();
    let orig = terrain.and_then(|t| Ground::of(model, t));
    let fin = terrain.and_then(|t| {
        let mut t = t.clone();
        if let (Some(p), SiteForm::Terrain { points, .. }) = (edited_points(model, &t), &mut t.form)
        {
            *points = p;
        }
        Ground::of(model, &t)
    });
    let surf = |q: DVec2| fin.as_ref().and_then(|g| g.at(q));
    // the building's underground: slabs on levels at ±0,000 + 0.5 or below
    let under: Vec<(Vec<DVec2>, f64)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Slab(s) => {
                let z = members::elevation(model, s.level);
                (z <= 0.5 && s.boundary.len() > 2)
                    .then(|| (s.boundary.clone(), z + s.top_offset - s.thickness))
            }
            _ => None,
        })
        .collect();
    let mut taken = vec![];
    let (mut topsoil, mut pit_cut, mut backfill, mut grade_cut, mut grade_fill) =
        (0.0, 0.0, 0.0, 0.0, 0.0);
    let (mut pit_ids, mut grade_ids) = (vec![], vec![]);
    if let Some(g) = &orig {
        for (id, w) in &works {
            let ring = slope_ring(w, g);
            let top: Vec<DVec2> = ring.iter().map(|r| r.1).collect();
            let region = if top.len() > 2 {
                top
            } else {
                w.outline.clone()
            };
            let top_area = crate::polygon::signed_area2(&region).abs() / 2.0;
            let ts = top_area * w.topsoil;
            let area = crate::polygon::signed_area2(&w.outline).abs() / 2.0;
            if w.pit {
                let ps = |q: DVec2| pit_surface(w, g, q);
                let (cut, _, _) = diff_volume(&region, &|q| g.at(q), &ps);
                // what the building takes of the pit is not back-filled
                let mut occupied = 0.0;
                let (lo, hi) = bounds(&region);
                let mut x = lo.x + CELL / 2.0;
                while x < hi.x {
                    let mut y = lo.y + CELL / 2.0;
                    while y < hi.y {
                        let q = DVec2::new(x, y);
                        if contains(&region, q) {
                            if let (Some(zo), Some(zn)) = (g.at(q), ps(q)) {
                                let b = under
                                    .iter()
                                    .filter(|u| contains(&u.0, q))
                                    .map(|u| u.1)
                                    .fold(f64::INFINITY, f64::min);
                                if zo > zn && b.is_finite() {
                                    occupied += (zo - zn.max(b)).max(0.0) * CELL * CELL;
                                }
                            }
                        }
                        y += CELL;
                    }
                    x += CELL;
                }
                let c = (cut - ts).max(0.0);
                let bf = (cut - occupied - ts).max(0.0);
                taken.push(Taken {
                    id: *id,
                    pit: true,
                    area,
                    top_area,
                    cut: c,
                    fill: 0.0,
                    backfill: bf,
                    topsoil: ts,
                });
                topsoil += ts;
                pit_cut += c;
                backfill += bf;
                pit_ids.push(*id);
            } else {
                let (cut, fill, _) = diff_volume(&region, &|q| g.at(q), &surf);
                let c = (cut - ts).max(0.0);
                taken.push(Taken {
                    id: *id,
                    pit: false,
                    area,
                    top_area,
                    cut: c,
                    fill,
                    backfill: 0.0,
                    topsoil: ts,
                });
                topsoil += ts;
                grade_cut += c;
                grade_fill += fill;
                grade_ids.push(*id);
            }
        }
    }
    // trenches and pits under footings: from the ground (the pit's bottom
    // or the levelled ground; else 150 below ±0,000) to their underside
    let strips = crate::footing::strip_outlines(model);
    let (mut strip_v, mut pad_v) = (0.0, 0.0);
    let (mut strip_ids, mut pad_ids) = (vec![], vec![]);
    for e in model.iter() {
        let ElementKind::Footing(f) = &e.kind else {
            continue;
        };
        let under_side = f.top(model) - f.thickness;
        for o in crate::footing::outlines(model, e, Some(&strips)) {
            if o.len() < 3 {
                continue;
            }
            let c = o.iter().copied().sum::<DVec2>() / o.len() as f64;
            let mut z = surf(c)
                .or_else(|| orig.as_ref().and_then(|g| g.at(c)))
                .unwrap_or(-0.15);
            if let Some(g) = &orig {
                for (_, w) in works.iter().filter(|(_, w)| w.pit) {
                    if let Some(zz) = pit_surface(w, g, c) {
                        z = z.min(zz);
                    }
                }
            }
            let v = crate::polygon::signed_area2(&o).abs() / 2.0 * (z - under_side).max(0.0);
            match f.kind {
                FootingKind::Strip => {
                    strip_v += v;
                    strip_ids.push(e.id);
                }
                FootingKind::Pad => {
                    pad_v += v;
                    pad_ids.push(e.id);
                }
                // a pile is bored, not dug out
                FootingKind::Pile => {}
            }
        }
    }
    strip_ids.dedup();
    let mut rows = vec![];
    let all: Vec<ElementId> = pit_ids.iter().chain(&grade_ids).copied().collect();
    let mut row = |key, code, text, unit, qty: f64, calc, ids: Vec<ElementId>| {
        if qty > 1e-6 {
            rows.push(Row {
                key,
                code,
                text,
                unit,
                qty: (qty * 1000.0).round() / 1000.0,
                calc,
                ids,
            });
        }
    };
    row(
        "topsoil",
        "121101101",
        "Removing topsoil, moved to heaps within 100 m",
        "m3",
        topsoil,
        "area within the top edge × topsoil depth",
        all.clone(),
    );
    row(
        "cut",
        "122201101",
        "Cut, unsupported, in soil class 3",
        "m3",
        grade_cut,
        "ground before − levelled ground (cells 0.5 m)",
        grade_ids.clone(),
    );
    row(
        "pit",
        "131201101",
        "Excavating an unsupported pit in soil class 3 (slopes as designed)",
        "m3",
        pit_cut,
        "ground − the pit's bottom and slopes (cells 0.5 m)",
        pit_ids.clone(),
    );
    row(
        "trenches",
        "132201101",
        "Excavating trenches up to 600 mm wide in soil class 3: strip footings",
        "m3",
        strip_v,
        "footings' plan × depth from the ground to their underside",
        strip_ids,
    );
    row(
        "shafts",
        "133201101",
        "Excavating unsupported shafts in soil class 3: pad footings",
        "m3",
        pad_v,
        "footings' plan × depth from the ground to their underside",
        pad_ids,
    );
    let dug = topsoil + pit_cut + grade_cut + strip_v + pad_v;
    row(
        "move",
        "162201102",
        "Moving spoil of soil classes 1 to 4 (to a stockpile)",
        "m3",
        dug,
        "all that is dug",
        vec![],
    );
    row(
        "backfill",
        "174101001",
        "Back-filling pits, shafts, trenches and round buildings, compacted",
        "m3",
        backfill,
        "the pit − the building's underground",
        pit_ids,
    );
    row(
        "fill",
        "171101101",
        "Placing fill in embankments, spread and compacted",
        "m3",
        grade_fill,
        "levelled ground − ground before (cells 0.5 m)",
        grade_ids,
    );
    let away = dug - backfill - grade_fill - topsoil;
    row(
        "haul",
        "162701105",
        "Moving surplus spoil to a tip, distance as designed",
        "m3",
        away,
        "dug − back-fill − fill − topsoil (kept to spread again)",
        vec![],
    );
    row(
        "tip",
        "171201201",
        "Placing spoil on a tip",
        "m3",
        away,
        "as hauled",
        vec![],
    );
    (rows, taken)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Element, Model};

    fn level(m: &Model) -> ElementId {
        m.iter()
            .find(|e| matches!(&e.kind, ElementKind::Level(l) if l.elevation == 0.0))
            .unwrap()
            .id
    }

    fn square(c: f64, r: f64) -> Vec<DVec2> {
        crate::polygon::rectangle(DVec2::splat(c - r), DVec2::splat(c + r))
    }

    fn with(m: &Model, sites: Vec<Site>) -> Model {
        Model::from_elements(
            m.iter()
                .filter(|e| !matches!(e.kind, ElementKind::Site(_)))
                .cloned()
                .chain(
                    sites
                        .into_iter()
                        .map(|s| Element::new("x", ElementKind::Site(s))),
                ),
        )
        .unwrap()
    }

    #[test]
    fn a_pit_in_flat_ground_is_its_prism_and_slopes() {
        let m0 = crate::sample::sample_house();
        let l = level(&m0);
        let ground = Site::terrain(l, square(0.0, 30.0), 0.0, 0.5);
        // 10 × 10 m, 2 m deep, sides 1:1
        let pit = Site::earthwork(l, square(0.0, 5.0), -2.0, 1.0, true);
        let m = with(&m0, vec![ground, pit]);
        let (rows, taken) = schedule(&m);
        let t = &taken[0];
        // a frustum: 2 m deep, 10 m square at the bottom, 14 m at the top
        let exact = 2.0 / 3.0 * (100.0 + 196.0 + (100.0f64 * 196.0).sqrt());
        assert!((t.cut - exact).abs() / exact < 0.05, "{} vs {exact}", t.cut);
        assert!((t.top_area - 196.0).abs() < 10.0, "{}", t.top_area);
        assert!(rows.iter().any(|r| r.key == "pit"));
        assert!(rows.iter().any(|r| r.key == "haul"));
    }

    #[test]
    fn a_platform_cuts_where_the_ground_was_higher_and_changes_it() {
        let m0 = crate::sample::sample_house();
        let l = level(&m0);
        let mut ground = Site::terrain(l, square(0.0, 30.0), 1.0, 0.5);
        if let SiteForm::Terrain { points, .. } = &mut ground.form {
            // ground 1 m up everywhere
            for p in square(0.0, 30.0) {
                points.push(p.extend(1.0));
            }
        }
        let plat = Site::earthwork(l, square(0.0, 5.0), 0.0, 2.0, false);
        let m = with(&m0, vec![ground.clone(), plat]);
        let (rows, taken) = schedule(&m);
        assert!(taken[0].cut > 100.0 && taken[0].fill < 1.0, "{taken:?}");
        assert!(rows.iter().any(|r| r.key == "cut"));
        let pts = edited_points(&m, &ground).unwrap();
        assert!(pts
            .iter()
            .any(|p| p.truncate().length() < 3.0 && p.z.abs() < 1e-9));
    }
}
