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
        let Some(path) = rfd::FileDialog::new()
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
        let Some((view, level)) = self.map_plan(view) else {
            self.status = t("Open a plan first: the map goes under it.");
            return;
        };
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
                    vec![view],
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
            let mut r = Reference::new(source, vec![view]);
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
        if !tx.ops.is_empty() {
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
