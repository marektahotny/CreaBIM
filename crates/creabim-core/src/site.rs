//! The site around a building: the ground, hard landscape and planting.
//!
//! - **Terrain** is a ground surface over an outline, `thickness` deep:
//!   flat at its `top`, or following spot heights, between which it is
//!   interpolated (inverse distance weighting). Plans can show its
//!   contours. Hard landscape areas lying in a flat terrain are left out
//!   of it, so paving sits flush in the ground instead of on it.
//! - **Areas** are slab-like layers of hard landscape (paving, gravel, a
//!   driveway, decking), each with a material and a surface pattern that
//!   plans draw over it.
//! - **Plants** are trees and shrubs: a symbol the size of their canopy in
//!   plan, and a simple form (a trunk and a crown) in 3D and elevation.
//!
//! Cars, people and other entourage are catalogue objects (see
//! [`crate::catalogue`]). What each pattern or plant stands for is data:
//! the sizes and spacings are the element's own.

use crate::{transform::Xform, ElementId, Model};
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};

/// A site element on a level (usually the ground floor's or a site level).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Site {
    pub level: ElementId,
    pub form: SiteForm,
    /// What it is made of: the ground's, the area's surface, a plant's crown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<ElementId>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SiteForm {
    /// A ground surface over `boundary` (counter-clockwise), `top` above
    /// the level where it is flat, `thickness` deep below its lowest point.
    Terrain {
        boundary: Vec<DVec2>,
        #[serde(default)]
        top: f64,
        thickness: f64,
        /// Spot heights inside it (`z` above the level); none is flat.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        points: Vec<DVec3>,
        /// Contours every so much height in plan; 0 draws none.
        #[serde(default, skip_serializing_if = "is_zero")]
        contour: f64,
    },
    /// Hard landscape: a layer `thickness` deep with its top `top` above
    /// the level, less its holes, its surface patterned.
    Area {
        boundary: Vec<DVec2>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        holes: Vec<Vec<DVec2>>,
        #[serde(default)]
        top: f64,
        thickness: f64,
        #[serde(default)]
        pattern: Pattern,
    },
    /// Earthworks (PilonDesigner's terrain edits): a building pit dug to
    /// `bottom` (above the level) with sides sloping 1:`slope` up to the
    /// ground and back-filled after, so the ground stays; or (not `pit`) a
    /// platform levelled at `bottom` with slopes to the ground round it,
    /// which changes the ground for good. `topsoil` metres are stripped
    /// first. See [`crate::earthworks`].
    Earthwork {
        boundary: Vec<DVec2>,
        bottom: f64,
        slope: f64,
        pit: bool,
        #[serde(default, skip_serializing_if = "is_zero")]
        topsoil: f64,
    },
    /// A tree or shrub standing at `position`, its base `base` above the
    /// level: `canopy` across its crown, `height` to its top.
    Plant {
        position: DVec2,
        #[serde(default)]
        base: f64,
        form: PlantForm,
        canopy: f64,
        height: f64,
    },
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// The pattern drawn over an area's surface in plan.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum Pattern {
    /// Its outline only.
    #[default]
    None,
    /// Rectangular units `size` (along `angle`, across) with joints
    /// between: flags, blocks, setts. `stagger` offsets every other course
    /// by half a unit, as stretcher bond.
    Units {
        size: DVec2,
        #[serde(default)]
        stagger: bool,
        /// Degrees, counter-clockwise from the x axis.
        #[serde(default)]
        angle: f64,
    },
    /// Parallel lines `spacing` apart: boards, decking, ribbed concrete.
    Lines {
        spacing: f64,
        #[serde(default)]
        angle: f64,
    },
    /// Scattered dots about `spacing` apart: gravel, chippings, sand.
    Stipple { spacing: f64 },
}

impl Pattern {
    pub fn label(&self) -> &'static str {
        match self {
            Pattern::None => "None",
            Pattern::Units { .. } => "Units",
            Pattern::Lines { .. } => "Lines",
            Pattern::Stipple { .. } => "Stipple",
        }
    }

    fn validate(&self) -> Result<(), String> {
        let ok = match *self {
            Pattern::None => true,
            Pattern::Units { size, angle, .. } => {
                size.is_finite() && size.min_element() > 1e-3 && angle.is_finite()
            }
            Pattern::Lines { spacing, angle } => {
                spacing.is_finite() && spacing > 1e-3 && angle.is_finite()
            }
            Pattern::Stipple { spacing } => spacing.is_finite() && spacing > 1e-3,
        };
        if ok {
            Ok(())
        } else {
            Err("a surface pattern's sizes must be positive".into())
        }
    }
}

/// The shape a plant is drawn as.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlantForm {
    /// A broadleaf tree: a clear trunk under a round crown.
    #[default]
    Tree,
    /// A conifer: a cone from near the ground.
    Conifer,
    /// A palm: a tall bare trunk and a head of fronds.
    Palm,
    /// A shrub or bush: a low mound from the ground.
    Shrub,
}

impl PlantForm {
    pub const ALL: [PlantForm; 4] = [
        PlantForm::Tree,
        PlantForm::Conifer,
        PlantForm::Palm,
        PlantForm::Shrub,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PlantForm::Tree => "Tree",
            PlantForm::Conifer => "Conifer",
            PlantForm::Palm => "Palm",
            PlantForm::Shrub => "Shrub",
        }
    }
}

impl Site {
    /// A flat terrain over `boundary`.
    pub fn terrain(level: ElementId, mut boundary: Vec<DVec2>, top: f64, thickness: f64) -> Self {
        ccw(&mut boundary);
        Self {
            level,
            form: SiteForm::Terrain {
                boundary,
                top,
                thickness,
                points: vec![],
                contour: 0.0,
            },
            material: None,
        }
    }

    /// A hard landscape area over `boundary`.
    pub fn area(
        level: ElementId,
        mut boundary: Vec<DVec2>,
        thickness: f64,
        pattern: Pattern,
    ) -> Self {
        ccw(&mut boundary);
        Self {
            level,
            form: SiteForm::Area {
                boundary,
                holes: vec![],
                top: 0.0,
                thickness,
                pattern,
            },
            material: None,
        }
    }

    /// A building pit (or a platform) over `boundary`, its bottom
    /// `bottom` above the level, its sides sloping 1:`slope`.
    pub fn earthwork(
        level: ElementId,
        mut boundary: Vec<DVec2>,
        bottom: f64,
        slope: f64,
        pit: bool,
    ) -> Self {
        ccw(&mut boundary);
        Self {
            level,
            form: SiteForm::Earthwork {
                boundary,
                bottom,
                slope,
                pit,
                topsoil: 0.0,
            },
            material: None,
        }
    }

    /// A plant at `position`.
    pub fn plant(
        level: ElementId,
        position: DVec2,
        form: PlantForm,
        canopy: f64,
        height: f64,
    ) -> Self {
        Self {
            level,
            form: SiteForm::Plant {
                position,
                base: 0.0,
                form,
                canopy,
                height,
            },
            material: None,
        }
    }

    /// "Terrain", "Hard landscape", "Tree"...: what it is called by default.
    pub fn label(&self) -> &'static str {
        match &self.form {
            SiteForm::Terrain { .. } => "Terrain",
            SiteForm::Area { .. } => "Hard landscape",
            SiteForm::Earthwork { pit: true, .. } => "Building pit",
            SiteForm::Earthwork { .. } => "Platform",
            SiteForm::Plant { form, .. } => form.label(),
        }
    }

    pub fn is_plant(&self) -> bool {
        matches!(self.form, SiteForm::Plant { .. })
    }

    pub fn is_earthwork(&self) -> bool {
        matches!(self.form, SiteForm::Earthwork { .. })
    }

    /// Ground and areas can be cut by voids (a pool, a light well);
    /// plants and earthworks can't.
    pub fn cuttable(&self) -> bool {
        !self.is_plant() && !self.is_earthwork()
    }

    /// The outline of a terrain or an area.
    pub fn boundary(&self) -> Option<&[DVec2]> {
        match &self.form {
            SiteForm::Terrain { boundary, .. }
            | SiteForm::Area { boundary, .. }
            | SiteForm::Earthwork { boundary, .. } => Some(boundary),
            SiteForm::Plant { .. } => None,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let positive = |v: f64, what: &str| {
            if v.is_finite() && v > 0.0 {
                Ok(())
            } else {
                Err(format!("{what} must be positive, got {v}"))
            }
        };
        let outline = |b: &[DVec2]| {
            if b.len() >= 3 && b.iter().all(|p| p.is_finite()) {
                Ok(())
            } else {
                Err("a site outline needs three finite corners".to_string())
            }
        };
        match &self.form {
            SiteForm::Terrain {
                boundary,
                top,
                thickness,
                points,
                contour,
            } => {
                outline(boundary)?;
                positive(*thickness, "terrain thickness")?;
                if !top.is_finite() || points.iter().any(|p| !p.is_finite()) {
                    return Err("terrain heights must be finite".into());
                }
                if !(contour.is_finite() && *contour >= 0.0) {
                    return Err("the contour interval can't be negative".into());
                }
                Ok(())
            }
            SiteForm::Area {
                boundary,
                holes,
                top,
                thickness,
                pattern,
            } => {
                outline(boundary)?;
                holes.iter().try_for_each(|h| outline(h))?;
                positive(*thickness, "area thickness")?;
                if !top.is_finite() {
                    return Err("an area's top must be finite".into());
                }
                pattern.validate()
            }
            SiteForm::Earthwork {
                boundary,
                bottom,
                slope,
                topsoil,
                ..
            } => {
                outline(boundary)?;
                if !(bottom.is_finite() && slope.is_finite() && *slope >= 0.0) {
                    return Err("an earthwork's bottom and slope must be finite".into());
                }
                if !(topsoil.is_finite() && *topsoil >= 0.0) {
                    return Err("topsoil depth can't be negative".into());
                }
                Ok(())
            }
            SiteForm::Plant {
                position,
                base,
                canopy,
                height,
                ..
            } => {
                if !(position.is_finite() && base.is_finite()) {
                    return Err("a plant's position must be finite".into());
                }
                positive(*canopy, "canopy size")?;
                positive(*height, "plant height")
            }
        }
    }

    /// [`Site::ground_at`] for many points, its spot heights gathered
    /// once (a surveyed terrain has hundreds).
    pub fn ground_heights(&self, ps: &[DVec2]) -> Vec<f64> {
        let SiteForm::Terrain {
            boundary,
            top,
            points,
            ..
        } = &self.form
        else {
            return ps.iter().map(|p| self.ground_at(*p)).collect();
        };
        if points.is_empty() {
            return vec![*top; ps.len()];
        }
        let controls = controls(boundary, *top, points);
        ps.iter()
            .map(|p| surface_height(*top, &controls, *p))
            .collect()
    }

    /// The height of a terrain's surface above its level at `p`: its top
    /// when flat, else weighted towards the nearest spot heights.
    pub fn ground_at(&self, p: DVec2) -> f64 {
        match &self.form {
            SiteForm::Terrain {
                boundary,
                top,
                points,
                ..
            } => {
                if points.is_empty() {
                    return *top;
                }
                surface_height(*top, &controls(boundary, *top, points), p)
            }
            SiteForm::Area { top, .. } => *top,
            SiteForm::Earthwork { bottom, .. } => *bottom,
            SiteForm::Plant { base, .. } => *base,
        }
    }

    /// The corners of its bounding box, in model coordinates.
    pub fn bounds(&self, model: &Model) -> Option<(DVec3, DVec3)> {
        let z = crate::members::elevation(model, self.level);
        match &self.form {
            SiteForm::Terrain {
                boundary,
                top,
                thickness,
                points,
                ..
            } => {
                let (lo, hi) = flat_bounds(boundary)?;
                let zs = points.iter().map(|p| p.z).chain([*top]);
                let (zlo, zhi) = zs.fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| {
                    (a.min(v), b.max(v))
                });
                Some((lo.extend(z + zlo - thickness), hi.extend(z + zhi)))
            }
            SiteForm::Area {
                boundary,
                top,
                thickness,
                ..
            } => {
                let (lo, hi) = flat_bounds(boundary)?;
                Some((lo.extend(z + top - thickness), hi.extend(z + top)))
            }
            SiteForm::Earthwork {
                boundary, bottom, ..
            } => {
                let (lo, hi) = flat_bounds(boundary)?;
                Some((lo.extend(z + bottom), hi.extend(z + bottom.max(0.0))))
            }
            SiteForm::Plant {
                position,
                base,
                canopy,
                height,
                ..
            } => {
                let r = DVec2::splat(canopy / 2.0);
                Some((
                    (*position - r).extend(z + base),
                    (*position + r).extend(z + base + height),
                ))
            }
        }
    }

    /// Where it is, for turning and scaling it about itself.
    pub fn pivot(&self, model: &Model) -> Option<DVec3> {
        let (lo, hi) = self.bounds(model)?;
        Some(((lo + hi) / 2.0).truncate().extend(lo.z))
    }
}

/// A triangulation of `boundary` with `inner` points added as corners,
/// its edges flipped towards a Delaunay triangulation.
pub fn tin(boundary: &[DVec2], inner: &[DVec2]) -> (Vec<DVec2>, Vec<[usize; 3]>) {
    let mut verts = boundary.to_vec();
    let mut tris = crate::polygon::triangulate(boundary);
    let n0 = verts.len();
    let ccw = |t: &[usize; 3], v: &[DVec2]| (v[t[1]] - v[t[0]]).perp_dot(v[t[2]] - v[t[0]]) > 0.0;
    for t in tris.iter_mut() {
        if !ccw(t, &verts) {
            t.swap(1, 2);
        }
    }
    for p in inner {
        let Some(i) = tris.iter().position(|t| {
            let [a, b, c] = t.map(|i| verts[i]);
            (b - a).perp_dot(*p - a) >= 0.0
                && (c - b).perp_dot(*p - b) >= 0.0
                && (a - c).perp_dot(*p - c) >= 0.0
        }) else {
            continue;
        };
        if verts.iter().any(|v| v.distance(*p) < 1e-3) {
            continue;
        }
        let k = verts.len();
        verts.push(*p);
        let [a, b, c] = tris[i];
        // on (or a hair from) an edge, as a survey's grid points are on
        // the outline: the edge is split, in its other triangle too, so no
        // flat sliver is left along it
        let on_edge = [(a, b, c), (b, c, a), (c, a, b)]
            .into_iter()
            .find(|&(u, v, _)| {
                let e = verts[v] - verts[u];
                e.perp_dot(*p - verts[u]).abs() < 1e-6 * e.length().max(1e-9)
            });
        let Some((u, v, w)) = on_edge else {
            tris[i] = [a, b, k];
            tris.push([b, c, k]);
            tris.push([c, a, k]);
            continue;
        };
        tris[i] = [u, k, w];
        tris.push([k, v, w]);
        let twin = tris
            .iter()
            .position(|t| (0..3).any(|e| t[e] == v && t[(e + 1) % 3] == u));
        if let Some(j) = twin {
            let x = tris[j].into_iter().find(|&q| q != u && q != v);
            if let Some(x) = x {
                tris[j] = [v, k, x];
                tris.push([k, u, x]);
            }
        }
    }
    // Lawson flips: an interior edge whose opposite corner falls inside
    // the other triangle's circumcircle is swapped
    let in_circle = |a: DVec2, b: DVec2, c: DVec2, d: DVec2| {
        let (ax, ay) = (a.x - d.x, a.y - d.y);
        let (bx, by) = (b.x - d.x, b.y - d.y);
        let (cx, cy) = (c.x - d.x, c.y - d.y);
        (ax * ax + ay * ay) * (bx * cy - cx * by) - (bx * bx + by * by) * (ax * cy - cx * ay)
            + (cx * cx + cy * cy) * (ax * by - bx * ay)
            > 1e-12
    };
    let boundary_edge =
        |a: usize, b: usize| a < n0 && b < n0 && ((a + 1) % n0 == b || (b + 1) % n0 == a);
    // each directed edge to the triangle it runs round; edges to check
    // are kept on a stack, so a flip looks again only at its neighbours
    let mut edges: std::collections::HashMap<(usize, usize), usize> = Default::default();
    let mut stack: Vec<(usize, usize)> = vec![];
    for (i, t) in tris.iter().enumerate() {
        for e in 0..3 {
            edges.insert((t[e], t[(e + 1) % 3]), i);
            stack.push((t[e], t[(e + 1) % 3]));
        }
    }
    let third = |t: &[usize; 3], a: usize, b: usize| t.iter().copied().find(|&v| v != a && v != b);
    let mut budget = 64 * tris.len().max(1);
    while let Some((a, b)) = stack.pop() {
        if budget == 0 {
            break;
        }
        budget -= 1;
        if boundary_edge(a, b) {
            continue;
        }
        let (Some(&i), Some(&j)) = (edges.get(&(a, b)), edges.get(&(b, a))) else {
            continue;
        };
        let (Some(c), Some(d)) = (third(&tris[i], a, b), third(&tris[j], a, b)) else {
            continue;
        };
        let (pa, pb, pc, pd) = (verts[a], verts[b], verts[c], verts[d]);
        // only a convex quad can flip
        let convex = (pd - pc).perp_dot(pa - pc) * (pd - pc).perp_dot(pb - pc) < 0.0;
        if convex && in_circle(pa, pb, pc, pd) {
            edges.remove(&(a, b));
            edges.remove(&(b, a));
            tris[i] = [c, a, d];
            tris[j] = [d, b, c];
            for (k, t) in [(i, tris[i]), (j, tris[j])] {
                for e in 0..3 {
                    edges.insert((t[e], t[(e + 1) % 3]), k);
                }
            }
            stack.extend([(a, d), (d, b), (b, c), (c, a)]);
        }
    }
    (verts, tris)
}

/// What a terrain's surface passes through: its spot heights, and the
/// outline's corners at its top unless a spot height is on them.
pub(crate) fn controls(boundary: &[DVec2], top: f64, points: &[DVec3]) -> Vec<DVec3> {
    let corners = boundary
        .iter()
        .filter(|c| !points.iter().any(|q| q.truncate().distance(**c) < 1e-3))
        .map(|c| c.extend(top));
    points.iter().copied().chain(corners).collect()
}

fn ccw(pts: &mut [DVec2]) {
    if crate::polygon::signed_area2(pts) < 0.0 {
        pts.reverse();
    }
}

fn flat_bounds(pts: &[DVec2]) -> Option<(DVec2, DVec2)> {
    let first = *pts.first()?;
    Some(
        pts.iter()
            .fold((first, first), |(lo, hi), p| (lo.min(*p), hi.max(*p))),
    )
}

/// A surface through spot heights, inverse distance weighted (power 2);
/// `flat` where there are none.
pub fn surface_height(flat: f64, points: &[DVec3], p: DVec2) -> f64 {
    if points.is_empty() {
        return flat;
    }
    let (mut sum, mut weight) = (0.0, 0.0);
    for q in points {
        let d2 = q.truncate().distance_squared(p);
        if d2 < 1e-12 {
            return q.z;
        }
        let w = 1.0 / d2;
        sum += w * q.z;
        weight += w;
    }
    sum / weight
}

/// Is `inner` inside `outer` and clear of its edges (a hole touching
/// the outline would split the ground)?
fn clear_inside(inner: &[DVec2], outer: &[DVec2]) -> bool {
    let near = |p: DVec2, poly: &[DVec2]| {
        (0..poly.len()).any(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            let d = b - a;
            let t = ((p - a).dot(d) / d.length_squared().max(1e-12)).clamp(0.0, 1.0);
            p.distance(a + d * t) < 1e-3
        })
    };
    inner
        .iter()
        .all(|p| crate::polygon::contains(outer, *p) && !near(*p, outer))
        && outer.iter().all(|p| !near(*p, inner))
}

/// The hard landscape areas a flat terrain leaves room for: those on its
/// level lying inside it, whose top is in its depth. Their outlines, as
/// holes in the ground.
pub fn ground_holes(model: &Model, terrain: &Site) -> Vec<Vec<DVec2>> {
    let SiteForm::Terrain {
        boundary,
        top,
        thickness,
        points,
        ..
    } = &terrain.form
    else {
        return vec![];
    };
    if !points.is_empty() {
        return vec![];
    }
    model
        .iter()
        .filter_map(|e| match &e.kind {
            crate::ElementKind::Site(Site {
                level,
                form:
                    SiteForm::Area {
                        boundary: b,
                        top: t,
                        ..
                    },
                ..
            }) if *level == terrain.level
                && *t <= top + 0.1
                && *t > top - thickness
                && clear_inside(b, boundary) =>
            {
                Some(b.clone())
            }
            _ => None,
        })
        .collect()
}

/// Move, turn or scale a site element; it stays upright.
pub fn transform(site: &mut Site, t: &Xform) -> Option<()> {
    let dz = match t {
        Xform::Move(d) => d.z,
        _ => 0.0,
    };
    let factor = match t {
        Xform::Scale { factor, .. } => Some(*factor),
        Xform::Stretch { .. } if site.is_plant() => return None,
        _ => None,
    };
    match &mut site.form {
        SiteForm::Terrain {
            boundary,
            top,
            points,
            ..
        } => {
            for p in boundary.iter_mut() {
                *p = t.point2(*p);
            }
            ccw(boundary);
            for p in points.iter_mut() {
                *p = t.point2(p.truncate()).extend(p.z + dz);
            }
            *top += dz;
        }
        SiteForm::Area {
            boundary,
            holes,
            top,
            ..
        } => {
            for p in boundary.iter_mut().chain(holes.iter_mut().flatten()) {
                *p = t.point2(*p);
            }
            ccw(boundary);
            *top += dz;
        }
        SiteForm::Earthwork {
            boundary, bottom, ..
        } => {
            for p in boundary.iter_mut() {
                *p = t.point2(*p);
            }
            ccw(boundary);
            *bottom += dz;
        }
        SiteForm::Plant {
            position,
            base,
            canopy,
            height,
            ..
        } => {
            *position = t.point2(*position);
            *base += dz;
            if let Some(f) = factor {
                *canopy *= f;
                *height *= f;
            }
        }
    }
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_grid_of_points_on_and_inside_the_outline_leaves_no_slivers() {
        // a survey's grid: its outer points lie on the outline's edges
        let n = 12;
        let boundary = [
            DVec2::ZERO,
            DVec2::new(11.0, 0.0),
            DVec2::new(11.0, 11.0),
            DVec2::new(0.0, 11.0),
        ];
        let inner: Vec<DVec2> = (0..n * n)
            .map(|i| DVec2::new((i % n) as f64, (i / n) as f64))
            .collect();
        let (v, tris) = tin(&boundary, &inner);
        assert_eq!(v.len(), n * n);
        let areas: Vec<f64> = tris
            .iter()
            .map(|t| (v[t[1]] - v[t[0]]).perp_dot(v[t[2]] - v[t[0]]) / 2.0)
            .collect();
        assert!((areas.iter().sum::<f64>() - 121.0).abs() < 1e-9);
        // every triangle is half a cell: none flat, none long and thin
        assert_eq!(tris.len(), 2 * 11 * 11);
        assert!(areas.iter().all(|a| (a - 0.5).abs() < 1e-9), "{areas:?}");
    }
    use crate::{Element, ElementKind, Level};

    #[test]
    fn spot_heights_shape_the_ground() {
        let pts = [DVec3::new(0.0, 0.0, 0.0), DVec3::new(10.0, 0.0, 2.0)];
        assert_eq!(surface_height(0.5, &[], DVec2::ZERO), 0.5);
        assert_eq!(surface_height(0.5, &pts, DVec2::new(10.0, 0.0)), 2.0);
        let mid = surface_height(0.5, &pts, DVec2::new(5.0, 0.0));
        assert!((mid - 1.0).abs() < 1e-9);
    }

    #[test]
    fn paving_sits_in_the_ground() {
        let lv = Element::new(
            "Site",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let l = lv.id;
        let ground = Site::terrain(
            l,
            crate::polygon::rectangle(DVec2::splat(-20.0), DVec2::splat(20.0)),
            0.0,
            0.5,
        );
        let drive = Site::area(
            l,
            crate::polygon::rectangle(DVec2::ZERO, DVec2::new(3.0, 8.0)),
            0.15,
            Pattern::Units {
                size: DVec2::new(0.2, 0.1),
                stagger: true,
                angle: 0.0,
            },
        );
        let outside = Site::area(
            l,
            crate::polygon::rectangle(DVec2::splat(30.0), DVec2::splat(32.0)),
            0.15,
            Pattern::None,
        );
        let m = Model::from_elements([
            lv,
            Element::new("Ground", ElementKind::Site(ground.clone())),
            Element::new("Drive", ElementKind::Site(drive)),
            Element::new("Far", ElementKind::Site(outside)),
        ])
        .unwrap();
        assert_eq!(ground_holes(&m, &ground).len(), 1);
    }

    #[test]
    fn plants_scale_but_terrain_stretches() {
        let mut p = Site::plant(ElementId::new(), DVec2::ZERO, PlantForm::Tree, 4.0, 6.0);
        let stretch = Xform::Stretch {
            center: DVec3::ZERO,
            factor: DVec3::new(2.0, 1.0, 1.0),
        };
        assert!(transform(&mut p, &stretch).is_none());
        transform(
            &mut p,
            &Xform::Scale {
                center: DVec3::ZERO,
                factor: 0.5,
            },
        )
        .unwrap();
        assert!(matches!(p.form, SiteForm::Plant { canopy, .. } if canopy == 2.0));
        let mut t = Site::terrain(
            ElementId::new(),
            crate::polygon::rectangle(DVec2::ZERO, DVec2::ONE),
            0.0,
            0.3,
        );
        assert!(transform(&mut t, &stretch).is_some());
        assert!(t.validate().is_ok());
    }
}
