//! A terrain made from surveyed points (XYZ, TXT, PTS, CSV), as
//! PilonDesigner makes one from `teren.xyz`: the points thinned to at most
//! 1500, the terrain's outline their convex hull, each point a spot height
//! the surface passes through. Points in S-JTSK come to the project's
//! origin (whole metres) and heights above sea level to the project's
//! ±0,000 (its elevation, else the lowest point's whole metre).

use super::CreaBimApp;
use creabim_core::site::{Site, SiteForm};
use creabim_core::{Element, ElementId, ElementKind, Model, Transaction};
use creabim_exchange::xyz;
use glam::{DVec2, DVec3};

/// A terrain made from points, and what was done to place it.
#[derive(Debug)]
pub struct FromPoints {
    pub site: Site,
    /// Points read, and kept after thinning.
    pub read: usize,
    pub kept: usize,
    /// Taken off the points' coordinates (east, north, height).
    pub moved: DVec3,
}

/// The height of the project's ±0,000 in a survey whose lowest point is
/// `lo`: none (0) when its heights are already to ±0,000 (under 50 m);
/// above sea level, the project's elevation when it has one the survey is
/// near, else the lowest point's whole metre.
pub fn datum(model: &Model, lo: f64) -> f64 {
    let elevation = model.project_info().map_or(0.0, |(_, i)| i.elevation);
    if lo <= 50.0 {
        0.0
    } else if elevation != 0.0 && (lo - elevation).abs() < 500.0 {
        elevation
    } else {
        lo.floor()
    }
}

/// The terrain `points` (as read) make on `level`.
pub fn terrain_from_points(
    model: &Model,
    level: ElementId,
    points: &[DVec3],
) -> Option<FromPoints> {
    let pts = xyz::thin(points, xyz::MAX_POINTS);
    let hull = xyz::hull(&pts);
    if hull.len() < 3 {
        return None;
    }
    let lo = pts.iter().fold(DVec3::INFINITY, |a, p| a.min(*p));
    let hi = pts.iter().fold(DVec3::NEG_INFINITY, |a, p| a.max(*p));
    // far away (S-JTSK): its lower left corner, whole metres, to the origin
    let far = lo
        .truncate()
        .abs()
        .max_element()
        .max(hi.truncate().abs().max_element())
        > 2000.0;
    let xy = if far {
        lo.truncate().floor()
    } else {
        DVec2::ZERO
    };
    let z0 = datum(model, lo.z);
    let level_z = creabim_core::members::elevation(model, level);
    let moved = xy.extend(z0);
    let local: Vec<DVec3> = pts
        .iter()
        .map(|p| *p - moved - DVec3::Z * level_z)
        .collect();
    let boundary: Vec<DVec2> = hull.iter().map(|&i| local[i].truncate()).collect();
    let top = local.iter().map(|p| p.z).fold(f64::INFINITY, f64::min);
    let mut site = Site::terrain(level, boundary, top, 0.5);
    if let SiteForm::Terrain {
        points, contour, ..
    } = &mut site.form
    {
        *points = local;
        let span = hi.z - lo.z;
        *contour = if span > 20.0 {
            1.0
        } else if span > 4.0 {
            0.5
        } else {
            0.25
        };
    }
    site.validate().ok()?;
    Some(FromPoints {
        site,
        read: points.len(),
        kept: pts.len(),
        moved,
    })
}

impl CreaBimApp {
    /// `terrain:points` from the Cadastre tab: pick a survey file, make a
    /// terrain from it on the working level.
    pub(super) fn terrain_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "terrain" || arg != "points" {
            return false;
        }
        let Some(path) = rfd::FileDialog::new()
            .set_title(crate::i18n::tr("Terrain from surveyed points"))
            .add_filter(
                crate::i18n::tr("Points (XYZ, TXT, PTS, CSV, LAS)"),
                &["xyz", "txt", "pts", "csv", "asc", "las"],
            )
            .pick_file()
        else {
            return true;
        };
        self.terrain_from_file(&path.display().to_string());
        true
    }

    /// Make a terrain from the points in `path`.
    pub(super) fn terrain_from_file(&mut self, path: &str) {
        let t = crate::i18n::tr;
        let Ok(bytes) = std::fs::read(path) else {
            self.status = format!("{} {path}", t("Couldn't read"));
            return;
        };
        // a LAS cloud's points, a couple of hundred thousand of them
        let read = if bytes.starts_with(b"LASF") {
            match creabim_exchange::cloud::read_las(std::io::Cursor::new(&bytes), 200_000) {
                Ok(c) => xyz::Points {
                    points: (0..c.points.len()).map(|n| c.at(n)).collect(),
                    skipped: 0,
                    jtsk: c.jtsk,
                },
                Err(e) => {
                    self.status = format!("{path}: {e}");
                    return;
                }
            }
        } else {
            xyz::read(&creabim_exchange::dxf::decode(&bytes))
        };
        let name = std::path::Path::new(path)
            .file_stem()
            .map_or("Terrain".into(), |n| n.to_string_lossy().into_owned());
        self.terrain_from(
            &read.points,
            name,
            "No terrain made: the file needs at least three points, one per line as x y z.",
        );
    }

    /// Make a terrain from a 3D drawing reference's heights (PilonDesigner's
    /// Terén z DXF): the points, lines' vertices and faces' corners on its
    /// layers shown, within its clip, where the reference puts them.
    pub(super) fn terrain_from_reference(&mut self, id: ElementId) {
        let model = self.doc.model();
        let Some(e) = model.get(id) else {
            return;
        };
        let ElementKind::Reference(r) = &e.kind else {
            return;
        };
        let (r, name) = (r.clone(), e.name.clone());
        let Ok(Some(d)) = self.plan.ref_cache.drawing(&r.source) else {
            return;
        };
        let points: Vec<DVec3> = d
            .heights(&r.layers_off, r.clip)
            .into_iter()
            .map(|q| r.place(q.truncate()).extend(q.z))
            .collect();
        self.terrain_from(
            &points,
            name,
            "No terrain made: the layers shown hold fewer than three points with heights.",
        );
    }

    /// Make a terrain from a point cloud reference: its lowest point in
    /// each half metre (the ground under trees and cars), where the
    /// reference puts them.
    pub(super) fn terrain_from_cloud(&mut self, id: ElementId) {
        let model = self.doc.model();
        let Some(e) = model.get(id) else {
            return;
        };
        let ElementKind::Reference(r) = &e.kind else {
            return;
        };
        let (r, name) = (r.clone(), e.name.clone());
        let creabim_core::RefSource::Points { path, lift, .. } = &r.source else {
            return;
        };
        let Ok(c) = self.plan.ref_cache.cloud(path) else {
            return;
        };
        let points: Vec<DVec3> = c
            .ground(0.5, r.clip)
            .into_iter()
            .map(|q| r.place(q.truncate()).extend(q.z + lift))
            .collect();
        self.terrain_from(
            &points,
            name,
            "No terrain made: the cloud holds fewer than three points.",
        );
    }

    /// Make a terrain named `name` from `points` (as read) on the working
    /// level; `none` says why when they make none.
    fn terrain_from(&mut self, points: &[DVec3], name: String, none: &str) {
        let t = crate::i18n::tr;
        let Some((level, _)) = self.work_level() else {
            return;
        };
        let Some(made) = terrain_from_points(self.doc.model(), level, points) else {
            self.status = t(none);
            return;
        };
        let e = Element::new(name, ElementKind::Site(made.site));
        let id = e.id;
        if let Err(err) = self
            .doc
            .commit(Transaction::new("Terrain from points").insert(e))
        {
            self.status = err.to_string();
            return;
        }
        self.selection.only(id);
        let n = crate::i18n::num;
        let mut s = format!("{}: {}", t("Terrain made from points"), made.kept);
        if made.kept < made.read {
            s.push_str(&format!(" ({} {})", t("thinned from"), made.read));
        }
        s.push('.');
        if made.moved.truncate() != DVec2::ZERO {
            s.push_str(&format!(
                " {} E {} m, N {} m.",
                t("S-JTSK: its origin is at"),
                n(made.moved.x, 0),
                n(made.moved.y, 0)
            ));
        }
        if made.moved.z != 0.0 {
            s.push_str(&format!(
                " {} {} m {}.",
                t("±0,000 is at"),
                n(made.moved.z, 2),
                t("above sea level")
            ));
        }
        self.status = s;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_survey_in_sjtsk_makes_a_terrain_at_the_origin() {
        let m = creabim_core::sample::sample_house();
        let level = m
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Level(_)))
            .unwrap()
            .id;
        let read = xyz::read(
            "1 523456.40 1234567.70 302.15\n\
             2 523436.40 1234567.70 303.15\n\
             3 523436.40 1234547.70 304.15\n\
             4 523456.40 1234547.70 303.65\n\
             5 523446.40 1234557.70 305.00\n",
        );
        let made = terrain_from_points(&m, level, &read.points).unwrap();
        assert_eq!((made.read, made.kept), (5, 5));
        // east and north are −Y and −X; the lower left whole metre comes
        // to the origin, the lowest whole metre to ±0,000
        assert_eq!(made.moved, DVec3::new(-523457.0, -1234568.0, 302.0));
        let SiteForm::Terrain {
            boundary, points, ..
        } = &made.site.form
        else {
            panic!()
        };
        assert_eq!(boundary.len(), 4);
        assert!(boundary
            .iter()
            .all(|p| p.x >= 0.0 && p.y >= 0.0 && p.x < 22.0));
        let level_z = creabim_core::members::elevation(&m, level);
        assert!((points[4].z + level_z - 3.0).abs() < 1e-9);
        // the surface passes through each point
        assert!((made.site.ground_at(points[4].truncate()) - points[4].z).abs() < 1e-9);
    }
}
