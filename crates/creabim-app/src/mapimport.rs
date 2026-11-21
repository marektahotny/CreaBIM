//! A package from Pilon Map Downloader (ZIP) put into the project, as
//! PilonDesigner's *ZIP z máp*: its files unpacked into a folder beside
//! it; the site plan's drawings (cadastre, networks, contours) and the
//! orthophoto as references in the plan, where the map puts them; a
//! terrain from the elevation model; and where the project lies in S-JTSK
//! and above sea level, from `bod_pociatku.txt`.

use super::CreaBimApp;
use creabim_core::{
    Element, ElementId, ElementKind, RefSource, Reference, Transaction, View, ViewKind,
};
use creabim_exchange::{mapdl, xyz};
use glam::{DVec2, DVec3};

impl CreaBimApp {
    /// `map:zip` from the Cadastre tab: pick a package and put it in.
    pub(super) fn map_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "map" || arg != "zip" {
            return false;
        }
        let Some(path) = crate::files::file_dialog()
            .set_title(crate::i18n::tr("Map package (Pilon Map Downloader)"))
            .add_filter("ZIP", &["zip"])
            .pick_file()
        else {
            return true;
        };
        self.map_from_zip(&path, None);
        true
    }

    /// The plan the package's references show in: `view` (it was dropped
    /// on), else the one open, else the working level's first.
    pub(super) fn map_plan(&self, view: Option<ElementId>) -> Option<(ElementId, ElementId)> {
        let model = self.doc.model();
        let plan_of = |v: ElementId| match model.get(v).map(|e| &e.kind) {
            Some(ElementKind::View(View {
                kind: ViewKind::Plan { level, .. },
                ..
            })) => Some((v, *level)),
            _ => None,
        };
        if let Some(p) = view.or(self.active_view).and_then(plan_of) {
            return Some(p);
        }
        let (level, _) = self.work_level()?;
        model
            .iter()
            .filter_map(|e| plan_of(e.id))
            .find(|(_, l)| *l == level)
    }

    /// The site's level, "3D terén", and its plan: made (and kept in the
    /// project's information) the first time a map is put in.
    pub(super) fn site_level_plan(&mut self) -> Result<(ElementId, ElementId), String> {
        let name = "3D terén";
        let model = self.doc.model();
        let level = match model.site_level() {
            Some(l) => l,
            None => {
                let e = Element::new(
                    name,
                    ElementKind::Level(creabim_core::Level {
                        elevation: 0.0,
                        height: 3.0,
                    }),
                );
                let id = e.id;
                self.doc
                    .commit(Transaction::new("Site level").insert(e))
                    .map_err(|e| e.to_string())?;
                let mut info = self
                    .doc
                    .model()
                    .project_info()
                    .map(|(_, p)| p.clone())
                    .unwrap_or_default();
                info.site_level = Some(id);
                let tx = creabim_core::set_project(self.doc.model(), info);
                self.doc.commit(tx).map_err(|e| e.to_string())?;
                id
            }
        };
        let model = self.doc.model();
        let plan = model.iter().find_map(|e| match &e.kind {
            ElementKind::View(v)
                if matches!(v.kind, ViewKind::Plan { level: l, .. } if l == level)
                    && !v.graphics.range.from_above =>
            {
                Some(e.id)
            }
            _ => None,
        });
        let plan = match plan {
            Some(p) => p,
            None => {
                let z = model.level(level).map_or(0.0, |l| l.elevation);
                let v = creabim_core::levels::plan_of(model, level, z);
                let e = Element::new(name, ElementKind::View(v));
                let id = e.id;
                self.doc
                    .commit(Transaction::new("Site level").insert(e))
                    .map_err(|e| e.to_string())?;
                id
            }
        };
        Ok((plan, level))
    }

    /// Put in the package at `path`, in the plan `view` if given.
    pub(super) fn map_from_zip(&mut self, path: &std::path::Path, view: Option<ElementId>) {
        let t = crate::i18n::tr;
        match std::fs::read(path)
            .map_err(|e| e.to_string())
            .and_then(|b| mapdl::unzip(&b))
        {
            // unpacked beside it, for the references to read
            Ok(files) => self.map_put_in(files, &path.with_extension(""), view),
            Err(e) => self.status = format!("{} {}: {e}", t("Couldn't read"), path.display()),
        }
    }

    /// Put in a package's `files`, written into `dir` for the references
    /// to read, in the plan `view` if given.
    pub(super) fn map_put_in(
        &mut self,
        files: Vec<(String, Vec<u8>)>,
        dir: &std::path::Path,
        view: Option<ElementId>,
    ) {
        let t = crate::i18n::tr;
        let pkg = mapdl::package(&files);
        if pkg.dxfs.is_empty() && pkg.ortho.is_none() && pkg.xyz.is_none() && pkg.origin.is_none() {
            self.status = t("The package holds no site plan, orthophoto, terrain or origin point.");
            return;
        }
        let _ = view;
        // the site gets a level of its own, "3D terén", drawn in its own
        // plan and the site plan only: no floor plan shows the terrain
        let (view, level) = match self.site_level_plan() {
            Ok(v) => v,
            Err(e) => {
                self.status = e;
                return;
            }
        };
        // the map's drawings show there and in the site plan
        let mut shown_in = vec![view];
        for e in self.doc.model().iter() {
            if let ElementKind::View(v) = &e.kind {
                if matches!(v.kind, ViewKind::Plan { .. }) && v.graphics.range.is_site() {
                    shown_in.push(e.id);
                }
            }
        }
        let file = |name: &str| dir.join(name).display().to_string();
        if let Err(e) = std::fs::create_dir_all(dir).and_then(|_| {
            files
                .iter()
                .try_for_each(|(n, b)| std::fs::write(dir.join(n), b))
        }) {
            self.status = format!("{} {}: {e}", t("Couldn't write"), dir.display());
            return;
        }
        let mut done: Vec<String> = vec![];
        // where the project is: its ±0,000 if it has none yet
        let origin = pkg.origin;
        if let Some(o) = origin {
            let mut info = self
                .doc
                .model()
                .project_info()
                .map(|(_, p)| p.clone())
                .unwrap_or_default();
            info.sjtsk = Some([o.e, o.n]);
            if info.elevation == 0.0 {
                info.elevation = o.h0;
            }
            let tx = creabim_core::set_project(self.doc.model(), info);
            if tx.ops.is_empty() {
            } else if let Err(e) = self.doc.commit(tx) {
                self.status = e.to_string();
                return;
            }
            done.push(format!(
                "S-JTSK E {}, N {}, ±0,000 = {} m Bpv",
                crate::i18n::num(o.e, 2),
                crate::i18n::num(o.n, 2),
                crate::i18n::num(o.h0, 3)
            ));
        }
        let shift = origin.map_or(DVec2::ZERO, |o| DVec2::new(o.e, o.n));
        let mut tx = Transaction::new("Map package");
        let mut first = None;
        // the orthophoto under the drawings
        if let Some((img, world)) = &pkg.ortho {
            let p = file(img);
            let px = crate::trace::picture_px(&p);
            let world = std::fs::read(file(world)).unwrap_or_default();
            let at = px.and_then(|px| {
                mapdl::ortho_placement(&creabim_exchange::dxf::decode(&world), px, origin.as_ref())
            });
            if let Some([min, size]) = at {
                let mut r = Reference::new(
                    RefSource::Image {
                        path: p,
                        width: size.x,
                        dpi: None,
                    },
                    shown_in.clone(),
                );
                r.bounds = Some([DVec2::ZERO, size]);
                r.at = min + size / 2.0;
                r.colour = None;
                r.opacity = 0.75;
                let e = Element::new(t("Orthophoto"), ElementKind::Reference(r));
                first.get_or_insert(e.id);
                tx = tx.insert(e);
                done.push(t("orthophoto"));
            }
        }
        // the site plan, in metres; in S-JTSK brought to the origin
        let mut drawings = 0;
        for (name, jtsk) in &pkg.dxfs {
            let source = RefSource::Dxf {
                path: file(name),
                unit: Some(1.0),
            };
            let shown = crate::trace::read(self.doc.model(), &source);
            let Some(b) = crate::trace::bounds_of(&source, &shown) else {
                continue;
            };
            let mut r = Reference::new(source, shown_in.clone());
            r.in_place(b);
            // over the model, so the terrain's fill doesn't cover its lines
            r.over = true;
            // its layers drawn as a site plan's: parcel boundaries thin
            // black, contours dashed light brown, the main ones darker,
            // buildings grey (changed under Layers)
            if let crate::trace::Shown::Lines(l) = &shown {
                r.layer_styles = l
                    .layers
                    .iter()
                    .filter_map(|n| creabim_core::LayerStyle::for_map_layer(n))
                    .collect();
            }
            // named so, or kilometres away (a package in S-JTSK only)
            if *jtsk || xyz::is_jtsk(b[0].x, b[0].y) {
                if origin.is_some() {
                    r.at -= shift;
                } else {
                    r.at_origin(b);
                }
            }
            let stem = name.trim_end_matches(".dxf").trim_end_matches(".DXF");
            let label = match stem.trim_end_matches("_lokal").trim_end_matches("_jtsk") {
                "situacia" => t("Site plan: cadastre, networks, contours"),
                _ => stem.to_string(),
            };
            let e = Element::new(label, ElementKind::Reference(r));
            first.get_or_insert(e.id);
            tx = tx.insert(e);
            drawings += 1;
        }
        if drawings > 0 {
            done.push(format!("{drawings} {}", t("drawings")));
        }
        // the terrain: heights above sea level, to the project's ±0,000
        if let Some(name) = &pkg.xyz {
            let text = std::fs::read(file(name))
                .map(|b| creabim_exchange::dxf::decode(&b))
                .unwrap_or_default();
            let read = xyz::read(&text);
            let h0 = origin.map_or(0.0, |o| o.h0);
            let points: Vec<DVec3> = read
                .points
                .iter()
                .map(|p| {
                    if read.jtsk {
                        (p.truncate() - shift).extend(p.z)
                    } else {
                        p.truncate().extend(p.z + h0)
                    }
                })
                .collect();
            if let Some(made) =
                super::terrainimport::terrain_from_points(self.doc.model(), level, &points)
            {
                let e = Element::new(t("Terrain from the map"), ElementKind::Site(made.site));
                first.get_or_insert(e.id);
                tx = tx.insert(e);
                done.push(format!("{} ({} {})", t("terrain"), made.kept, t("points")));
            }
        }
        // all of it moves and turns as one, S-JTSK read off its drawing
        let group: Vec<ElementId> = tx
            .ops
            .iter()
            .filter_map(|o| match o {
                creabim_core::Op::Insert(e) => Some(e.id),
                _ => None,
            })
            .collect();
        let jtsk_ref = tx.ops.iter().find_map(|o| match o {
            creabim_core::Op::Insert(e) => match &e.kind {
                ElementKind::Reference(r)
                    if matches!(r.source, RefSource::Dxf { .. })
                        && origin.is_some()
                        && r.bounds.is_some_and(|[a, _]| xyz::is_jtsk(a.x, a.y)) =>
                {
                    Some(e.id)
                }
                _ => None,
            },
            _ => None,
        });
        if !tx.ops.is_empty() {
            if let Some((pid, info)) = self.doc.model().project_info() {
                let mut info = info.clone();
                // a map put in again replaces the old one's part in it
                info.map_group
                    .retain(|id| self.doc.model().get(*id).is_some());
                info.map_group.extend(&group);
                if jtsk_ref.is_some() {
                    info.map_ref = jtsk_ref;
                }
                if let Some(e) = self.doc.model().get(pid) {
                    let mut now = e.clone();
                    now.kind = ElementKind::Project(info);
                    tx = tx.replace(e.clone(), now);
                }
            }
            if let Err(e) = self.doc.commit(tx) {
                self.status = e.to_string();
                return;
            }
        }
        if let Some(id) = first {
            self.selection.only(id);
        }
        self.status = format!("{}: {}.", t("Map package put in"), done.join(", "));
    }

    /// Break drawing reference `id` into lines of the model: each of its
    /// lines (on the layers shown, within its clip) a line on the level of
    /// the plan it shows in, named by its layer; the reference hidden.
    pub(super) fn explode_reference(&mut self, id: ElementId) {
        let t = crate::i18n::tr;
        let model = self.doc.model();
        let Some(ElementKind::Reference(r)) = model.get(id).map(|e| e.kind.clone()) else {
            return;
        };
        let level = r
            .shown_in
            .iter()
            .find_map(|v| match model.get(*v).map(|e| &e.kind) {
                Some(ElementKind::View(View {
                    kind: ViewKind::Plan { level, .. },
                    ..
                })) => Some(*level),
                _ => None,
            });
        let Some(level) = level.or_else(|| self.work_level().map(|(l, _)| l)) else {
            self.status = t("Open a plan first: the map goes under it.");
            return;
        };
        let crate::trace::Shown::Lines(lines) = self.plan.ref_cache.get(self.doc.model(), id, &r)
        else {
            self.status = t("Only a drawing (DXF, PDF) breaks into lines.");
            return;
        };
        const MOST: usize = 50_000;
        if lines.len() > MOST {
            self.status = format!(
                "{} {} {}",
                t("Too many lines to break up:"),
                lines.len(),
                t("Turn some layers off or clip it first.")
            );
            return;
        }
        let mut tx = Transaction::new("Explode reference");
        let mut n = 0;
        for (i, l) in lines.iter().enumerate() {
            let pts: Vec<DVec2> = l.iter().map(|p| r.place(*p)).collect();
            if pts.len() < 2 {
                continue;
            }
            let closed = pts.len() > 3 && pts[0].distance(pts[pts.len() - 1]) < 1e-6;
            let pts = if closed {
                pts[..pts.len() - 1].to_vec()
            } else {
                pts
            };
            let sk = creabim_core::Sketch::polyline(
                Some(level),
                creabim_core::Workplane::ground(0.0),
                pts,
                closed,
            );
            let name = lines.layer(i).unwrap_or("Line").to_string();
            tx = tx.insert(Element::new(name, ElementKind::Sketch(sk)));
            n += 1;
        }
        if n == 0 {
            self.status = t("It has no lines to break up.");
            return;
        }
        if let Some(before) = self.doc.model().get(id).cloned() {
            let mut after = before.clone();
            if let ElementKind::Reference(r) = &mut after.kind {
                r.hidden = true;
            }
            tx = tx.replace(before, after);
        }
        match self.doc.commit(tx) {
            Ok(_) => {
                self.status = format!(
                    "{} {n} {}",
                    t("Reference broken into"),
                    t("lines; the reference is hidden.")
                )
            }
            Err(e) => self.status = e.to_string(),
        }
    }
}

#[cfg(test)]
mod shot_project {
    use creabim_core::{Element, ElementKind, RefSource, Reference, Transaction};
    use glam::DVec2;

    /// Writes a project for checking the turned site plan by eye:
    /// `CREABIM_SHOT_PROJECT=<dir> cargo test -p creabim-app shot_project -- --ignored`.
    #[test]
    #[ignore]
    fn write_a_turned_site_plan_project() {
        let Ok(dir) = std::env::var("CREABIM_SHOT_PROJECT") else {
            return;
        };
        let dir = std::path::PathBuf::from(dir);
        let mut app = crate::app::test_app();
        for e in app.doc.model().iter() {
            if let ElementKind::View(v) = &e.kind {
                eprintln!(
                    "view {:?} from_above={} footprint={}",
                    e.name, v.graphics.range.from_above, v.graphics.range.footprint
                );
            }
        }
        // a cadastre in S-JTSK: parcels at 25° to the building
        let shift = DVec2::new(-547382.28, -1253318.85);
        let r = DVec2::from_angle(25f64.to_radians());
        let mut dxf = String::from("0\nSECTION\n2\nENTITIES\n");
        let mut line = |a: DVec2, b: DVec2, layer: &str| {
            let (a, b) = (shift + r.rotate(a), shift + r.rotate(b));
            dxf += &format!(
                "0\nLINE\n8\n{layer}\n10\n{}\n20\n{}\n11\n{}\n21\n{}\n",
                a.x, a.y, b.x, b.y
            );
        };
        for k in -2..=2 {
            let x = k as f64 * 20.0 - 4.0;
            line(DVec2::new(x, -40.0), DVec2::new(x, 40.0), "hranice");
            let y = k as f64 * 20.0 - 5.0;
            line(DVec2::new(-44.0, y), DVec2::new(36.0, y), "hranice");
        }
        for k in 0..12 {
            let y = -30.0 + k as f64 * 5.0;
            line(
                DVec2::new(-44.0, y),
                DVec2::new(36.0, y + 6.0),
                "vrstevnice",
            );
        }
        dxf += "0\nENDSEC\n0\nEOF\n";
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("situacia_jtsk.dxf");
        std::fs::write(&path, dxf).unwrap();
        let ground = app.doc.model().levels().next().unwrap().0.id;
        let mut sv = creabim_core::View::plan(ground);
        sv.graphics.range.from_above = true;
        sv.graphics.range.footprint = true;
        sv.scale = 200.0;
        app.doc
            .commit(
                Transaction::new("site").insert(Element::new("Situácia", ElementKind::View(sv))),
            )
            .unwrap();
        // a roof plan, which the map was put in before too
        let top = app.doc.model().levels().last().unwrap().0.id;
        let mut roof = creabim_core::View::plan(top);
        roof.graphics.range.from_above = true;
        app.doc
            .commit(
                Transaction::new("roof").insert(Element::new("Strecha", ElementKind::View(roof))),
            )
            .unwrap();
        let views: Vec<_> = app
            .doc
            .model()
            .iter()
            .filter(|e| matches!(&e.kind, ElementKind::View(v) if v.graphics.range.from_above))
            .map(|e| e.id)
            .collect();
        let source = RefSource::Dxf {
            path: path.to_string_lossy().into(),
            unit: Some(1.0),
        };
        let shown = crate::trace::read(app.doc.model(), &source);
        let b = crate::trace::bounds_of(&source, &shown).unwrap();
        let mut rf = Reference::new(source, views.clone());
        rf.in_place(b);
        rf.at -= shift;
        rf.over = true;
        rf.in_3d = true;
        rf.layer_styles = ["hranice", "vrstevnice"]
            .into_iter()
            .filter_map(creabim_core::LayerStyle::for_map_layer)
            .collect();
        let e = Element::new("Situácia z mapy", ElementKind::Reference(rf));
        let id = e.id;
        let info = creabim_core::ProjectInfo {
            sjtsk: Some([shift.x, shift.y]),
            map_group: vec![id],
            map_ref: Some(id),
            ..app
                .doc
                .model()
                .project_info()
                .map(|p| p.1.clone())
                .unwrap_or_default()
        };
        app.doc.commit(Transaction::new("map").insert(e)).unwrap();
        // an orthophoto under the house, shown in 3D too
        let png = dir.join("ortofoto.png");
        let mut img = image::RgbaImage::new(64, 64);
        for (x, y, p) in img.enumerate_pixels_mut() {
            *p = if (x / 8 + y / 8) % 2 == 0 {
                image::Rgba([90, 140, 70, 255])
            } else {
                image::Rgba([200, 190, 150, 255])
            };
        }
        img.save(&png).unwrap();
        let src = RefSource::Image {
            path: png.to_string_lossy().into(),
            width: 40.0,
            dpi: None,
        };
        let shown = crate::trace::read(app.doc.model(), &src);
        let b = crate::trace::bounds_of(&src, &shown).unwrap();
        let mut photo = Reference::new(src, views.clone());
        photo.in_place(b);
        photo.at = DVec2::new(-15.0, -17.0);
        photo.in_3d = true;
        let photo = Element::new("Ortofoto", ElementKind::Reference(photo));
        let photo_id = photo.id;
        app.doc
            .commit(Transaction::new("photo").insert(photo))
            .unwrap();
        let tx = creabim_core::set_project(app.doc.model(), info);
        app.doc.commit(tx).unwrap();
        let mut info2 = app.doc.model().project_info().unwrap().1.clone();
        info2.map_group.push(photo_id);
        let tx = creabim_core::set_project(app.doc.model(), info2);
        app.doc.commit(tx).unwrap();
        // the site plan turned square to the parcels
        for v in &views {
            let t = crate::planview::turn_to_line(0.0, shift * 0.0, r.rotate(DVec2::Y));
            crate::planview::set_view_turn(&mut app.doc, *v, t).unwrap();
        }
        // networks in each state and the point symbols, for the shots
        use creabim_core::utility::{NetKind, NetState, SiteMark, UtilityLine};
        use creabim_core::{Sketch, Workplane};
        let mut tx = Transaction::new("nets");
        let nets = [
            (NetKind::Water, NetState::Proposed, -4.0),
            (NetKind::FoulSewer, NetState::Proposed, -7.0),
            (NetKind::GasLow, NetState::Existing, -10.0),
            (NetKind::PowerLow, NetState::ExistingConnection, -13.0),
            (NetKind::Telecom, NetState::Cancelled, -16.0),
            (NetKind::DistrictHeating, NetState::Existing, -19.0),
        ];
        for (kind, state, y) in nets {
            let mut sk = Sketch::polyline(
                Some(ground),
                Workplane::ground(0.0),
                vec![DVec2::new(-10.0, y), DVec2::new(22.0, y)],
                false,
            );
            sk.net = Some(UtilityLine {
                kind,
                state,
                note: String::new(),
            });
            tx = tx.insert(Element::new("net", ElementKind::Sketch(sk)));
        }
        for (i, m) in [
            SiteMark::HydrantAbove,
            SiteMark::Valve,
            SiteMark::SewerManhole,
            SiteMark::Gully,
            SiteMark::GasSniffer,
            SiteMark::LampOnMast,
            SiteMark::EntryBuilding,
            SiteMark::EntryPlot,
        ]
        .into_iter()
        .enumerate()
        {
            let at = DVec2::new(-10.0 + i as f64 * 4.0, -22.5);
            let mut sk = Sketch::polyline(
                Some(ground),
                Workplane::ground(0.0),
                vec![at, at + DVec2::Y],
                false,
            );
            sk.mark = Some(m);
            tx = tx.insert(Element::new("mark", ElementKind::Sketch(sk)));
        }
        // an S-JTSK coordinate mark, to be seen selected
        let dim = creabim_core::Dimension::spot(
            views[0],
            creabim_core::HostPoint::free(DVec2::new(10.3, 0.0)),
            creabim_core::HostPoint::free(DVec2::new(13.0, 2.0)),
            Some(creabim_core::annotation::SJTSK_PATTERN.to_string()),
        );
        tx = tx.insert(Element::new("spot", ElementKind::Dimension(dim)));
        app.doc.commit(tx).unwrap();
        for v in &views {
            let d = creabim_geom::section::view_drawing(app.doc.model(), *v).unwrap();
            let n = d
                .items
                .iter()
                .filter(|it| creabim_geom::netlines::is_site_only(app.doc.model(), it.id()))
                .count();
            eprintln!("site view: {n} network/symbol items of {}", d.items.len());
        }
        // a place for the building marked on the map, off and askew, drawn
        // in the site plan; then the site laid under the building
        let o = DVec2::new(-25.0, 12.0);
        let (u, w) = (DVec2::from_angle(0.35), DVec2::from_angle(0.35).perp());
        let mut rect = Sketch::polyline(
            Some(ground),
            Workplane::ground(0.0),
            vec![o, o + u * 10.3, o + u * 10.3 + w * 8.0, o + w * 8.0],
            true,
        );
        rect.view = Some(views[0]);
        let rect_e = Element::new("place", ElementKind::Sketch(rect));
        app.doc
            .commit(Transaction::new("place").insert(rect_e))
            .unwrap();
        creabim_store::save(dir.join("before.creabim"), &app.doc).unwrap();
        let mut lo = DVec2::splat(1e9);
        let mut hi = DVec2::splat(-1e9);
        for e in app.doc.model().iter() {
            if let ElementKind::Wall(w) = &e.kind {
                for p in [w.start, w.end] {
                    lo = lo.min(p - DVec2::splat(w.thickness / 2.0));
                    hi = hi.max(p + DVec2::splat(w.thickness / 2.0));
                }
            }
        }
        let r = creabim_core::mapgroup::fit((o, o + u), (lo, DVec2::new(hi.x, lo.y)));
        let tx = creabim_core::mapgroup::move_ground(app.doc.model(), &r);
        eprintln!("placed: {} ops, building {lo} {hi}", tx.ops.len());
        app.doc.commit(tx).unwrap();
        creabim_store::save(dir.join("placed.creabim"), &app.doc).unwrap();
        // a plot with a slanted rear boundary, drawn on the map; the house
        // set exactly 2 m off its side boundary and 5 m off the rear one
        let q = |x: f64, y: f64| {
            DVec2::new(-60.0, 30.0) + DVec2::from_angle(-0.5).rotate(DVec2::new(x, y))
        };
        let plot_pts = vec![q(0.0, 0.0), q(30.0, 0.0), q(26.0, 22.0), q(0.0, 22.0)];
        let mut plot =
            Sketch::polyline(Some(ground), Workplane::ground(0.0), plot_pts.clone(), true);
        plot.view = Some(views[0]);
        app.doc
            .commit(
                Transaction::new("plot").insert(Element::new("plot", ElementKind::Sketch(plot))),
            )
            .unwrap();
        let corners = [lo, DVec2::new(hi.x, lo.y), hi, DVec2::new(lo.x, hi.y)];
        let fit = creabim_core::mapgroup::fit_exact(
            (plot_pts[0], plot_pts[1]),
            (corners[0], corners[1]),
            (plot_pts[1], plot_pts[2]),
            &corners,
            2.0,
            5.0,
        )
        .unwrap();
        let tx = creabim_core::mapgroup::move_ground(app.doc.model(), &fit);
        app.doc.commit(tx).unwrap();
        let (a, b, c) = (
            fit.apply(plot_pts[0]),
            fit.apply(plot_pts[1]),
            fit.apply(plot_pts[2]),
        );
        let off =
            |p: DVec2, l0: DVec2, l1: DVec2| (p - l0).perp_dot(l1 - l0).abs() / (l1 - l0).length();
        eprintln!(
            "exact: side {:.3} rear {:.3}",
            corners
                .iter()
                .map(|p| off(*p, a, b))
                .fold(f64::INFINITY, f64::min),
            corners
                .iter()
                .map(|p| off(*p, b, c))
                .fold(f64::INFINITY, f64::min)
        );
        let dim0 = creabim_core::Dimension::spot(
            views[0],
            creabim_core::HostPoint::free(DVec2::ZERO),
            creabim_core::HostPoint::free(DVec2::new(-4.0, -3.0)),
            Some(creabim_core::annotation::SJTSK_PATTERN.to_string()),
        );
        app.doc
            .commit(
                Transaction::new("spot0")
                    .insert(Element::new("spot0", ElementKind::Dimension(dim0))),
            )
            .unwrap();
        let info = app.doc.model().project_info().unwrap().1.clone();
        eprintln!(
            "exact origin {:?} reads {:?}",
            info.sjtsk,
            creabim_core::mapgroup::sjtsk_of(app.doc.model(), DVec2::ZERO)
        );
        creabim_store::save(dir.join("exact.creabim"), &app.doc).unwrap();
        let tx = creabim_core::mapgroup::reset(app.doc.model());
        app.doc.commit(tx).unwrap();
        eprintln!(
            "reset: origin reads {:?}",
            creabim_core::mapgroup::sjtsk_of(app.doc.model(), DVec2::ZERO)
        );
        creabim_store::save(dir.join("reset.creabim"), &app.doc).unwrap();
        let legend =
            creabim_geom::serviceslegend::drawing(views[0], "Legenda situácie", app.doc.model());
        std::fs::write(dir.join("legend.svg"), legend.to_svg(1000.0)).unwrap();
        creabim_store::save(dir.join("turned.creabim"), &app.doc).unwrap();
        // the site plan on a sheet, its map under it
        let site = app
            .doc
            .model()
            .iter()
            .find(|e| e.name == "Situácia")
            .unwrap()
            .id;
        let sheet = Element::new(
            "Výkres situácie",
            ElementKind::Sheet(creabim_core::Sheet::a("C1", "A3")),
        );
        let sid = sheet.id;
        app.doc
            .commit(Transaction::new("sheet").insert(sheet))
            .unwrap();
        let mut vp =
            creabim_geom::sheet::new_viewport(app.doc.model(), sid, site, DVec2::new(190.0, 160.0));
        vp.target = DVec2::new(4.0, 3.0);
        app.doc
            .commit(Transaction::new("vp").insert(Element::new("vp", ElementKind::Viewport(vp))))
            .unwrap();
        creabim_store::save(dir.join("sheet.creabim"), &app.doc).unwrap();
        creabim_geom::refs::set_provider(crate::trace::sheet_items);
        let svg = creabim_geom::sheet::sheet_svg(app.doc.model(), sid, &Default::default());
        std::fs::write(dir.join("sheet.svg"), svg.unwrap()).unwrap();
    }
}
