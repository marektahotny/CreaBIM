//! Trace and reference (MU18, v2 after MU19): something shown faintly
//! under (or over) a drawing to draw over or compare against: another
//! view, a picture (a scanned plan, a photo of a sketch) or a DXF drawing.
//!
//! The reference itself is a project element ([`creabim_core::Reference`]):
//! where it is put, turned and scaled, its colour, strength and the views
//! it shows in are saved with the project. What it shows (a picture's
//! pixels, a drawing's lines, a view's linework) is read here and kept in
//! a cache, as a link is: the file stays where it is.

use creabim_core::reference::{RefSource, Reference, REFERENCE_COLOURS};
use creabim_core::{ElementId, Model};
use creabim_geom::plan::PlanItem;
use creabim_geom::raster::Raster;
use eframe::egui::{self, Color32, Pos2, Stroke};
use glam::DVec2;
use std::collections::HashMap;
use std::sync::Arc;

/// Set each frame: true while a pointer button is down, so drags reuse
/// what the references showed instead of reading their views again.
static HOLD: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Did the last DXF read stop at the file's item budget, leaving part of
/// it out? (Told in the status when the reference is added.)
pub static DXF_CAPPED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Hold (or release) the references' read drawings; see [`RefCache::get`].
/// Is a pointer button down (a drag anywhere)?
pub fn held() -> bool {
    HOLD.load(std::sync::atomic::Ordering::Relaxed)
}

pub fn hold(on: bool) {
    HOLD.store(on, std::sync::atomic::Ordering::Relaxed);
}

/// A text of a drawing: at its insertion point, `h` metres tall, turned
/// `angle` degrees, `align` 1 to 9 as MTEXT's (7 bottom left).
#[derive(Clone, Debug, PartialEq)]
pub struct RefText {
    pub at: DVec2,
    pub text: String,
    pub h: f64,
    pub angle: f64,
    pub align: u8,
}

/// A drawing's linework, in its own metres: its lines (which it derefs
/// to), its texts and the points it gives to snap to.
#[derive(Clone, Debug, Default)]
pub struct Linework {
    pub lines: Vec<Vec<DVec2>>,
    pub texts: Vec<RefText>,
    pub points: Vec<DVec2>,
}

impl std::ops::Deref for Linework {
    type Target = Vec<Vec<DVec2>>;
    fn deref(&self) -> &Self::Target {
        &self.lines
    }
}

impl From<Vec<Vec<DVec2>>> for Linework {
    fn from(lines: Vec<Vec<DVec2>>) -> Self {
        Linework {
            lines,
            ..Default::default()
        }
    }
}

/// What a reference shows, read: a picture, or lines in its own metres.
#[derive(Clone, Debug)]
pub enum Shown {
    Picture(Arc<Raster>),
    Lines(Arc<Linework>),
    /// It couldn't be read; why.
    Missing(String),
}

/// The references' contents, read once per file (or per model revision
/// for a view's linework).
#[derive(Default)]
pub struct RefCache {
    read: HashMap<ElementId, (String, u64, Shown)>,
    /// DXF files read, by path: what a reference shows is cut from them
    /// by its layers and clip.
    dxf: HashMap<String, Arc<creabim_exchange::dxf::DxfImport>>,
    /// PDF pages found to be scans, by source key.
    scans: std::collections::HashSet<String>,
}

/// The cache key of a source: its file, or its view.
fn key(source: &RefSource) -> String {
    match source {
        RefSource::View(v) => format!("view:{v:?}"),
        RefSource::Image { path, .. } => format!("image:{path}"),
        RefSource::Dxf { path } => format!("dxf:{path}"),
        RefSource::Pdf { path, page, scale } => format!("pdf:{path}#{page}@{scale}"),
    }
}

/// Where a scanned PDF page's picture lies on its page, by source key.
static SCANS: std::sync::Mutex<Option<HashMap<String, [DVec2; 2]>>> = std::sync::Mutex::new(None);

/// A PDF page read: its drawing, or its scan with where it lies.
pub fn pdf_page(
    path: &str,
    page: u32,
    scale: f64,
) -> Result<creabim_exchange::pdf::PdfPage, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("couldn't read {path}: {e}"))?;
    let p = creabim_exchange::pdf::read(&bytes, page, scale).map_err(|e| format!("{path}: {e}"))?;
    if p.scan.is_none()
        && p.imp.polys.is_empty()
        && p.imp.fills.is_empty()
        && p.imp.texts.is_empty()
    {
        return Err(if p.scan_unread {
            format!("{path}: the page is a scan in a format not read here (JBIG2, CCITT or JPEG 2000); export the page as PNG or JPEG and add that.")
        } else {
            format!("{path}: nothing drawn on page {page}")
        });
    }
    Ok(p)
}

/// A PDF page as a reference shows it: a scan as its picture (its place
/// kept for [`bounds_of`]), else its drawing.
fn pdf_shown(
    source: &RefSource,
    p: creabim_exchange::pdf::PdfPage,
) -> (Shown, Option<Arc<creabim_exchange::dxf::DxfImport>>) {
    match p.scan {
        Some(s) => {
            if let Ok(mut m) = SCANS.lock() {
                m.get_or_insert_with(HashMap::new)
                    .insert(key(source), s.rect);
            }
            (Shown::Picture(Arc::new(s.raster)), None)
        }
        None => {
            let d = Arc::new(p.imp);
            (Shown::Lines(Arc::new(dxf_linework(&d, &[], None))), Some(d))
        }
    }
}

/// A reference's key: its source's, and the layers and clip it shows.
fn ref_key(r: &Reference) -> String {
    let mut k = key(&r.source);
    if !r.layers_off.is_empty() {
        k.push_str(&format!("|off:{}", r.layers_off.join("\u{1}")));
    }
    if let Some([a, b]) = r.clip {
        k.push_str(&format!("|clip:{a}:{b}"));
    }
    k
}

impl RefCache {
    /// What reference `id` shows, read if not yet (or stale).
    pub fn get(&mut self, model: &Model, id: ElementId, r: &Reference) -> Shown {
        let k = ref_key(r);
        let rev = match r.source {
            RefSource::View(_) => model.revision(),
            _ => 0,
        };
        if let Some((kk, rr, s)) = self.read.get(&id) {
            // while a button is held (a drag), what was read stands: a view's
            // drawing was read again on every frame of a drag, because each
            // frame's move is a change to the model (MU37: references dragged
            // slowly); it is read again once, when the drag ends
            if *kk == k && (*rr == rev || HOLD.load(std::sync::atomic::Ordering::Relaxed)) {
                return s.clone();
            }
        }
        let s = match &r.source {
            RefSource::Dxf { .. } | RefSource::Pdf { .. } => match self.drawing(&r.source) {
                Ok(Some(d)) => Shown::Lines(Arc::new(dxf_linework(&d, &r.layers_off, r.clip))),
                Ok(None) => read(model, &r.source),
                Err(e) => Shown::Missing(e),
            },
            src => clip_shown(read(model, src), r.clip),
        };
        self.read.insert(id, (k, rev, s.clone()));
        s
    }

    /// A DXF file, read once.
    pub fn dxf_file(
        &mut self,
        path: &str,
    ) -> Result<Arc<creabim_exchange::dxf::DxfImport>, String> {
        if let Some(d) = self.dxf.get(path) {
            return Ok(d.clone());
        }
        let d = Arc::new(dxf_import(path)?);
        self.dxf.insert(path.to_string(), d.clone());
        Ok(d)
    }

    /// A DXF drawing or a PDF page's drawing, read once; none for a
    /// scanned page (a picture).
    pub fn drawing(
        &mut self,
        source: &RefSource,
    ) -> Result<Option<Arc<creabim_exchange::dxf::DxfImport>>, String> {
        match source {
            RefSource::Dxf { path } => self.dxf_file(path).map(Some),
            RefSource::Pdf { path, page, scale } => {
                let k = key(source);
                if let Some(d) = self.dxf.get(&k) {
                    return Ok(Some(d.clone()));
                }
                if self.scans.contains(&k) {
                    return Ok(None);
                }
                let (_, d) = pdf_shown(source, pdf_page(path, *page, *scale)?);
                match &d {
                    Some(d) => {
                        self.dxf.insert(k, d.clone());
                    }
                    None => {
                        self.scans.insert(k);
                    }
                }
                Ok(d)
            }
            _ => Ok(None),
        }
    }

    /// Forget what was read, so it is read again (Reload).
    pub fn forget(&mut self, id: ElementId) {
        if let Some((k, ..)) = self.read.remove(&id) {
            let k = k.split('|').next().unwrap_or(&k).to_string();
            if let Some(path) = k.strip_prefix("dxf:") {
                self.dxf.remove(path);
            }
            self.dxf.remove(&k);
            self.scans.remove(&k);
        }
    }
}

/// A DXF file read whole.
pub fn dxf_import(path: &str) -> Result<creabim_exchange::dxf::DxfImport, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("couldn't read {path}: {e}"))?;
    let d = creabim_exchange::dxf::read(&creabim_exchange::dxf::decode(&bytes), None);
    if d.polys.is_empty() && d.texts.is_empty() && d.points.is_empty() && d.faces.is_empty() {
        return Err(format!("{path}: no drawing in it"));
    }
    Ok(d)
}

/// What a drawing shows with `off` layers left out, within `clip`.
pub fn dxf_linework(
    d: &creabim_exchange::dxf::DxfImport,
    off: &[String],
    clip: Option<[DVec2; 2]>,
) -> Linework {
    let on = |l: &str| !off.iter().any(|x| x == l);
    let inside = |p: DVec2| clip.is_none_or(|[a, b]| p.cmpge(a).all() && p.cmple(b).all());
    let mut lines = vec![];
    for p in d.polys.iter().filter(|p| on(&p.layer)) {
        let mut pts = p.pts.clone();
        if p.closed {
            pts.push(pts[0]);
        }
        lines.extend(clip_line(&pts, clip));
    }
    for f in d.fills.iter().filter(|f| on(&f.layer)) {
        for r in &f.rings {
            let mut pts = r.clone();
            pts.push(r[0]);
            lines.extend(clip_line(&pts, clip));
        }
    }
    for f in d.faces.iter().filter(|f| on(&f.layer)) {
        let mut pts: Vec<DVec2> = f.pts.iter().map(|q| q.truncate()).collect();
        pts.push(pts[0]);
        lines.extend(clip_line(&pts, clip));
    }
    Linework {
        lines,
        texts: d
            .texts
            .iter()
            .filter(|t| on(&t.layer) && inside(t.p) && t.h > 0.0)
            .map(|t| RefText {
                at: t.p,
                text: t.text.clone(),
                h: t.h,
                angle: t.angle,
                align: t.align,
            })
            .collect(),
        points: d
            .points
            .iter()
            .filter(|m| on(&m.layer) && inside(m.p))
            .map(|m| m.p)
            .collect(),
    }
}

/// A picture or a view's lines within `clip` (a picture is clipped where
/// it is drawn).
fn clip_shown(s: Shown, clip: Option<[DVec2; 2]>) -> Shown {
    match (s, clip) {
        (Shown::Lines(l), Some(_)) => Shown::Lines(Arc::new(Linework {
            lines: l.lines.iter().flat_map(|p| clip_line(p, clip)).collect(),
            ..(*l).clone()
        })),
        (s, _) => s,
    }
}

/// A polyline's pieces within `clip` (Liang–Barsky on each segment).
pub fn clip_line(pts: &[DVec2], clip: Option<[DVec2; 2]>) -> Vec<Vec<DVec2>> {
    let Some([lo, hi]) = clip else {
        return if pts.len() >= 2 {
            vec![pts.to_vec()]
        } else {
            vec![]
        };
    };
    let mut out: Vec<Vec<DVec2>> = vec![];
    let mut cur: Vec<DVec2> = vec![];
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let d = b - a;
        let (mut t0, mut t1) = (0.0f64, 1.0f64);
        let mut keep = true;
        for (p, q) in [
            (-d.x, a.x - lo.x),
            (d.x, hi.x - a.x),
            (-d.y, a.y - lo.y),
            (d.y, hi.y - a.y),
        ] {
            if p.abs() < 1e-15 {
                if q < 0.0 {
                    keep = false;
                    break;
                }
                continue;
            }
            let t = q / p;
            if p < 0.0 {
                if t > t1 {
                    keep = false;
                    break;
                }
                t0 = t0.max(t);
            } else {
                if t < t0 {
                    keep = false;
                    break;
                }
                t1 = t1.min(t);
            }
        }
        if !keep {
            if cur.len() >= 2 {
                out.push(std::mem::take(&mut cur));
            }
            cur.clear();
            continue;
        }
        let (s, e) = (a + d * t0, a + d * t1);
        if cur.last().is_some_and(|l| l.distance(s) < 1e-12) {
            cur.push(e);
        } else {
            if cur.len() >= 2 {
                out.push(std::mem::take(&mut cur));
            }
            cur = vec![s, e];
        }
        if t1 < 1.0 {
            out.push(std::mem::take(&mut cur));
        }
    }
    if cur.len() >= 2 {
        out.push(cur);
    }
    out
}

/// Read a source.
pub fn read(model: &Model, source: &RefSource) -> Shown {
    match source {
        RefSource::View(v) => Shown::Lines(Arc::new(
            creabim_geom::section::view_drawing(model, *v)
                .map(|d| linework(&d.items))
                .unwrap_or_default()
                .into(),
        )),
        RefSource::Image { path, .. } => match picture(path) {
            Ok(r) => Shown::Picture(Arc::new(r)),
            Err(e) => Shown::Missing(e),
        },
        RefSource::Dxf { path } => match dxf_import(path) {
            Ok(d) => {
                DXF_CAPPED.store(
                    d.faces.len() >= 400_000,
                    std::sync::atomic::Ordering::Relaxed,
                );
                Shown::Lines(Arc::new(dxf_linework(&d, &[], None)))
            }
            Err(e) => Shown::Missing(e),
        },
        RefSource::Pdf { path, page, scale } => match pdf_page(path, *page, *scale) {
            Ok(p) => pdf_shown(source, p).0,
            Err(e) => Shown::Missing(e),
        },
    }
}

/// What a source spans in its own coordinates, once read.
pub fn bounds_of(source: &RefSource, shown: &Shown) -> Option<[DVec2; 2]> {
    match (source, shown) {
        (RefSource::Image { width, .. }, Shown::Picture(r)) => {
            let h = width * r.height as f64 / r.width.max(1) as f64;
            Some([DVec2::ZERO, DVec2::new(*width, h)])
        }
        (RefSource::Pdf { .. }, Shown::Picture(_)) => {
            SCANS.lock().ok()?.as_ref()?.get(&key(source)).copied()
        }
        (_, Shown::Lines(l)) => {
            let mut it = l.iter().flatten();
            let first = *it.next()?;
            let (lo, hi) = it.fold((first, first), |(lo, hi), p| (lo.min(*p), hi.max(*p)));
            Some([lo, hi])
        }
        _ => None,
    }
}

/// A reference's name: the view's, or the file's.
pub fn label(model: &Model, source: &RefSource) -> String {
    let file = |p: &str| {
        std::path::Path::new(p)
            .file_name()
            .map_or(p.to_string(), |n| n.to_string_lossy().into_owned())
    };
    match source {
        RefSource::View(v) => model.get(*v).map_or("a view".into(), |e| e.name.clone()),
        RefSource::Image { path, .. } | RefSource::Dxf { path } => file(path),
        RefSource::Pdf { path, page, .. } => format!("{} ({page})", file(path)),
    }
}

/// The ink a reference is drawn in; none keeps its own colours.
pub fn colour(r: &Reference) -> Option<Color32> {
    r.colour.map(|i| {
        let [cr, cg, cb] = REFERENCE_COLOURS[(i as usize).min(REFERENCE_COLOURS.len() - 1)].1;
        Color32::from_rgb(cr, cg, cb)
    })
}

/// Draw the references shown in `view` that go under the model (or, with
/// `over`, over it); `to_screen` places a model point.
pub fn draw_view(
    painter: &egui::Painter,
    model: &Model,
    cache: &mut RefCache,
    view: ElementId,
    over: bool,
    to_screen: &dyn Fn(DVec2) -> Pos2,
) {
    for (id, r) in creabim_core::reference::in_view(model, view) {
        if r.hidden || r.over != over || r.opacity <= 0.0 {
            continue;
        }
        let shown = cache.get(model, id, r);
        draw(painter, r, &shown, to_screen);
    }
}

/// Draw one reference.
pub fn draw(
    painter: &egui::Painter,
    r: &Reference,
    shown: &Shown,
    to_screen: &dyn Fn(DVec2) -> Pos2,
) {
    let a = r.opacity.clamp(0.0, 1.0);
    match shown {
        Shown::Picture(raster) => {
            let Some([lo, hi]) = r.bounds.or_else(|| bounds_of(&r.source, shown)) else {
                return;
            };
            let corners = [lo, DVec2::new(hi.x, lo.y), hi, DVec2::new(lo.x, hi.y)]
                .map(|c| to_screen(r.place(c)));
            let id = crate::textures::texture(painter.ctx(), raster);
            // its own colours, or tinted with the reference colour
            let tint = colour(r).unwrap_or(Color32::WHITE).gamma_multiply(a);
            let mut mesh = egui::Mesh::with_texture(id);
            let uv = [
                egui::pos2(0.0, 1.0),
                egui::pos2(1.0, 1.0),
                egui::pos2(1.0, 0.0),
                egui::pos2(0.0, 0.0),
            ];
            for (c, t) in corners.iter().zip(uv) {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: *c,
                    uv: t,
                    color: tint,
                });
            }
            mesh.indices.extend([0, 1, 2, 0, 2, 3]);
            painter.add(egui::Shape::mesh(mesh));
        }
        Shown::Lines(lines) => {
            let ink = colour(r)
                .unwrap_or(Color32::from_gray(70))
                .gamma_multiply(a);
            let stroke = Stroke::new(1.0, ink);
            for line in lines.iter() {
                let pts: Vec<Pos2> = line.iter().map(|p| to_screen(r.place(*p))).collect();
                if pts.len() >= 2 {
                    painter.add(egui::Shape::line(pts, stroke));
                }
            }
            draw_texts(painter, r, &lines.texts, ink, to_screen);
        }
        Shown::Missing(_) => {}
    }
}

/// A drawing's texts, where they are big enough to read.
fn draw_texts(
    painter: &egui::Painter,
    r: &Reference,
    texts: &[RefText],
    ink: Color32,
    to_screen: &dyn Fn(DVec2) -> Pos2,
) {
    if texts.is_empty() {
        return;
    }
    // screen pixels a metre of the reference spans, and its turn on screen
    let o = to_screen(r.place(DVec2::ZERO));
    let x = to_screen(r.place(DVec2::X));
    let px = (x - o).length() as f64;
    let screen_turn = -((x - o).y as f64).atan2((x - o).x as f64);
    let clip = painter.clip_rect().expand(200.0);
    for t in texts {
        let size = (t.h * px) as f32;
        if !(4.0..=400.0).contains(&size) {
            continue;
        }
        let at = to_screen(r.place(t.at));
        if !clip.contains(at) {
            continue;
        }
        let galley = painter.layout_no_wrap(t.text.clone(), egui::FontId::proportional(size), ink);
        let (w, h) = (galley.size().x, galley.size().y);
        // 1 2 3 top, 4 5 6 middle, 7 8 9 bottom; left, centre, right
        let col = ((t.align.max(1) - 1) % 3) as f32;
        let row = ((t.align.max(1) - 1) / 3) as f32;
        let (dx, dy) = (-w * col / 2.0, -h * row / 2.0);
        let angle = -(t.angle.to_radians() + screen_turn) as f32;
        let rot = egui::emath::Rot2::from_angle(angle);
        let pos = at + rot * egui::vec2(dx, dy);
        painter.add(egui::epaint::TextShape::new(pos, galley, ink).with_angle(angle));
    }
}

/// A drawing's linework as polylines: lines, outlines and arcs, no text
/// or fills.
pub fn linework(items: &[PlanItem]) -> Vec<Vec<DVec2>> {
    let mut out = Vec::new();
    for it in items {
        match it {
            PlanItem::Line { a, b, .. } => out.push(vec![*a, *b]),
            PlanItem::Polygon { points, .. } | PlanItem::Fill { points, .. } => {
                if let Some(first) = points.first() {
                    let mut p = points.clone();
                    p.push(*first);
                    out.push(p);
                }
            }
            PlanItem::Arc {
                center,
                radius,
                start,
                end,
                ..
            } => {
                let sweep = end - start;
                let n = ((sweep.abs() / 0.15).ceil() as usize).clamp(4, 64);
                out.push(
                    (0..=n)
                        .map(|i| {
                            let a = start + sweep * i as f64 / n as f64;
                            *center + DVec2::from_angle(a) * *radius
                        })
                        .collect(),
                );
            }
            _ => {}
        }
    }
    out
}

/// A picture file's pixels.
pub fn picture(path: &str) -> Result<Raster, String> {
    let img = image::open(path).map_err(|e| format!("couldn't read {path}: {e}"))?;
    let rgba = img.to_rgba8();
    Ok(Raster {
        width: rgba.width() as usize,
        height: rgba.height() as usize,
        rgba: rgba.into_raw(),
        key: path_key(path),
    })
}

fn path_key(path: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut h);
    h.finish() | 1 << 63
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_view_reference_reads_its_linework_and_spans_it() {
        let m = creabim_core::sample::sample_house();
        let v = m
            .iter()
            .find(|e| matches!(&e.kind, creabim_core::ElementKind::View(v) if matches!(v.kind, creabim_core::ViewKind::Plan { .. })))
            .unwrap()
            .id;
        let src = RefSource::View(v);
        let shown = read(&m, &src);
        let Shown::Lines(l) = &shown else { panic!() };
        assert!(!l.is_empty(), "a plan has linework");
        let [lo, hi] = bounds_of(&src, &shown).unwrap();
        assert!(hi.x > lo.x && hi.y > lo.y);
    }

    #[test]
    fn a_dxf_reference_reads_its_lines_in_metres() {
        let dir = std::env::temp_dir().join("creabim-trace-test.dxf");
        let text = "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nLINE\n8\n0\n10\n0\n20\n0\n11\n5000\n21\n0\n0\nENDSEC\n0\nEOF\n";
        std::fs::write(&dir, text).unwrap();
        let d = dxf_import(&dir.to_string_lossy()).unwrap();
        let lines = dxf_linework(&d, &[], None).lines;
        assert_eq!(lines.len(), 1);
        assert!((lines[0][1].x - 5.0).abs() < 1e-9, "{:?}", lines[0]);
    }

    #[test]
    fn a_drawing_shows_its_layers_on_and_what_lies_in_its_clip() {
        let t = "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n6\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n\
                 0\nLINE\n8\nA\n10\n0\n20\n0\n11\n10\n21\n0\n\
                 0\nLINE\n8\nB\n10\n0\n20\n5\n11\n10\n21\n5\n\
                 0\nTEXT\n8\nA\n10\n8\n20\n1\n40\n0.5\n1\nIzba\n0\nENDSEC\n0\nEOF\n";
        let d = creabim_exchange::dxf::read(t, None);
        let all = dxf_linework(&d, &[], None);
        assert_eq!((all.lines.len(), all.texts.len()), (2, 1));
        let a = dxf_linework(&d, &["B".to_string()], None);
        assert_eq!(a.lines.len(), 1);
        let clip = Some([DVec2::new(2.0, -1.0), DVec2::new(6.0, 6.0)]);
        let c = dxf_linework(&d, &[], clip);
        assert_eq!(c.lines.len(), 2);
        assert_eq!(c.lines[0], vec![DVec2::new(2.0, 0.0), DVec2::new(6.0, 0.0)]);
        assert!(c.texts.is_empty(), "the text lies outside the clip");
    }

    #[test]
    fn a_line_leaving_and_coming_back_into_a_clip_is_cut_in_two() {
        let clip = Some([DVec2::ZERO, DVec2::splat(10.0)]);
        let pts = [
            DVec2::new(1.0, 1.0),
            DVec2::new(1.0, 20.0),
            DVec2::new(5.0, 20.0),
            DVec2::new(5.0, 1.0),
        ];
        let out = clip_line(&pts, clip);
        assert_eq!(out.len(), 2, "{out:?}");
        assert_eq!(out[0], vec![DVec2::new(1.0, 1.0), DVec2::new(1.0, 10.0)]);
        assert_eq!(out[1], vec![DVec2::new(5.0, 10.0), DVec2::new(5.0, 1.0)]);
    }
}
