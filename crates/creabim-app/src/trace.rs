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
    /// Each line's layer, by its index in `layers` (a drawing's; none for
    /// a view's linework).
    pub layer_of: Vec<u32>,
    pub layers: Vec<String>,
}

impl Linework {
    /// The layer line `i` is on, if known.
    pub fn layer(&self, i: usize) -> Option<&str> {
        self.layers
            .get(*self.layer_of.get(i)? as usize)
            .map(|s| s.as_str())
    }
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
    /// A point cloud; a plan shows a slice of it ([`RefCache::slice`]).
    Cloud(Arc<creabim_exchange::cloud::Cloud>),
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
    /// Point clouds read, by source key.
    clouds: HashMap<String, Arc<creabim_exchange::cloud::Cloud>>,
    /// What a plan shows of a cloud, by reference and view, with what it
    /// was cut by.
    slices: HashMap<(ElementId, ElementId), (String, Arc<creabim_exchange::cloud::Slice>)>,
    /// The points 3D shows of a cloud, by source key.
    samples: HashMap<String, Arc<Vec<u32>>>,
    /// A drawing's lines where the views show it, indexed to snap to and
    /// pick by, with the reference as it was when they were put there.
    indexes: HashMap<ElementId, (String, Arc<crate::snaps::LineIndex>)>,
}

/// Most points a plan shows of a cloud's slice, and 3D of a cloud.
pub const SLICE_POINTS: usize = 150_000;
pub const SAMPLE_POINTS: usize = 150_000;

/// The cache key of a source: its file, or its view.
fn key(source: &RefSource) -> String {
    match source {
        RefSource::View(v) => format!("view:{v:?}"),
        RefSource::Image { path, .. } => format!("image:{path}"),
        RefSource::Dxf { path, unit: None } => format!("dxf:{path}"),
        RefSource::Dxf {
            path,
            unit: Some(u),
        } => format!("dxf:{path}@{u}"),
        RefSource::Pdf { path, page, scale } => format!("pdf:{path}#{page}@{scale}"),
        RefSource::Points { path, .. } => format!("points:{path}"),
    }
}

/// What is added to a cloud's heights to stand it in the model: less the
/// survey's ±0,000 (as a terrain's), and its own lift.
pub fn cloud_lift(model: &Model, source: &RefSource, c: &creabim_exchange::cloud::Cloud) -> f64 {
    let lift = match source {
        RefSource::Points { lift, .. } => *lift,
        _ => 0.0,
    };
    lift - crate::app::terrainimport::datum(model, c.lo.z)
}

/// The heights (the file's) between which `view` shows a cloud: its
/// slice above the plan's level, or all of it; none in other views.
pub fn cloud_range(
    model: &Model,
    source: &RefSource,
    c: &creabim_exchange::cloud::Cloud,
    view: ElementId,
) -> Option<(f64, f64)> {
    let RefSource::Points {
        from, to, whole, ..
    } = source
    else {
        return None;
    };
    let creabim_core::ElementKind::View(v) = &model.get(view)?.kind else {
        return None;
    };
    let creabim_core::ViewKind::Plan { level, .. } = &v.kind else {
        return None;
    };
    if *whole {
        return Some((f64::NEG_INFINITY, f64::INFINITY));
    }
    let z = model.level(*level).map_or(0.0, |l| l.elevation) - cloud_lift(model, source, c);
    Some((z + from.min(*to), z + from.max(*to)))
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
            RefSource::Points { path, .. } => match self.cloud(path) {
                Ok(c) => Shown::Cloud(c),
                Err(e) => Shown::Missing(e),
            },
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

    /// The lines and points reference `id` shows (a DXF or PDF drawing,
    /// another view's linework) where the views show them, indexed; none
    /// for a picture or a cloud.
    pub fn index(
        &mut self,
        model: &Model,
        id: ElementId,
        r: &Reference,
    ) -> Option<Arc<crate::snaps::LineIndex>> {
        let Shown::Lines(lines) = self.get(model, id, r) else {
            self.indexes.remove(&id);
            return None;
        };
        let k = format!(
            "{}|{:?}|{}|{}|{:?}|{}",
            ref_key(r),
            r.at,
            r.rotation,
            r.scale,
            r.bounds,
            Arc::as_ptr(&lines) as usize
        );
        if let Some((kk, ix)) = self.indexes.get(&id) {
            if *kk == k {
                return Some(ix.clone());
            }
        }
        let placed: Vec<Vec<DVec2>> = lines
            .iter()
            .map(|l| l.iter().map(|p| r.place(*p)).collect())
            .collect();
        let points: Vec<DVec2> = lines.points.iter().map(|p| r.place(*p)).collect();
        let ix = Arc::new(crate::snaps::LineIndex::new(&placed, &points));
        self.indexes.insert(id, (k, ix.clone()));
        Some(ix)
    }

    /// A point cloud's file, read once.
    pub fn cloud(&mut self, path: &str) -> Result<Arc<creabim_exchange::cloud::Cloud>, String> {
        let k = format!("points:{path}");
        if let Some(c) = self.clouds.get(&k) {
            return Ok(c.clone());
        }
        let c = Arc::new(creabim_exchange::cloud::read_file(path)?);
        self.clouds.insert(k, c.clone());
        Ok(c)
    }

    /// A point cloud's file, if read already.
    pub fn cached_cloud(&self, path: &str) -> Option<Arc<creabim_exchange::cloud::Cloud>> {
        self.clouds.get(&format!("points:{path}")).cloned()
    }

    /// A point cloud read elsewhere (in the background), kept as read.
    pub fn put_cloud(&mut self, path: &str, c: Arc<creabim_exchange::cloud::Cloud>) {
        let k = format!("points:{path}");
        self.samples.remove(&k);
        self.clouds.insert(k, c);
    }

    /// What the plan `view` shows of the cloud reference `id`: its points
    /// between the slice's heights, within its clip.
    pub fn slice(
        &mut self,
        model: &Model,
        id: ElementId,
        r: &Reference,
        view: ElementId,
    ) -> Option<Arc<creabim_exchange::cloud::Slice>> {
        let RefSource::Points { path, .. } = &r.source else {
            return None;
        };
        let c = self.cloud(path).ok()?;
        let (from, to) = cloud_range(model, &r.source, &c, view)?;
        let k = format!("{}|{from}:{to}", ref_key(r));
        if let Some((kk, s)) = self.slices.get(&(id, view)) {
            if *kk == k {
                return Some(s.clone());
            }
        }
        let s = Arc::new(c.slice(from, to, r.clip, SLICE_POINTS));
        self.slices.insert((id, view), (k, s.clone()));
        Some(s)
    }

    /// What the section or elevation `view` shows of the cloud reference
    /// `id`: its points in front of the view's line, within its depth,
    /// in the view's own coordinates (along the line, height).
    pub fn section(
        &mut self,
        model: &Model,
        id: ElementId,
        r: &Reference,
        view: ElementId,
    ) -> Option<Arc<creabim_exchange::cloud::Slice>> {
        let RefSource::Points { path, .. } = &r.source else {
            return None;
        };
        let creabim_core::ElementKind::View(v) = &model.get(view)?.kind else {
            return None;
        };
        let creabim_core::ViewKind::Section {
            start, end, depth, ..
        } = v.kind
        else {
            return None;
        };
        let c = self.cloud(path).ok()?;
        let off = cloud_lift(model, &r.source, &c);
        let k = format!(
            "{}|{start}:{end}:{depth}|{}:{}:{}:{off}",
            ref_key(r),
            r.at,
            r.rotation,
            r.scale
        );
        if let Some((kk, s)) = self.slices.get(&(id, view)) {
            if *kk == k {
                return Some(s.clone());
            }
        }
        let along = (end - start).normalize_or_zero();
        let look = along.perp();
        let clip = r.clip;
        let s = Arc::new(c.section(
            &|q| {
                let own = q.truncate();
                if clip.is_some_and(|[a, b]| !(own.cmpge(a).all() && own.cmple(b).all())) {
                    return None;
                }
                let w = r.place(own) - start;
                let d = w.dot(look);
                (d >= 0.0 && d <= depth.max(0.0)).then(|| DVec2::new(w.dot(along), q.z + off))
            },
            SLICE_POINTS,
        ));
        self.slices.insert((id, view), (k, s.clone()));
        Some(s)
    }

    /// The points 3D shows of a cloud.
    pub fn sample(&mut self, path: &str) -> Option<Arc<Vec<u32>>> {
        let k = format!("points:{path}");
        if let Some(s) = self.samples.get(&k) {
            return Some(s.clone());
        }
        let s = Arc::new(self.cloud(path).ok()?.sample(SAMPLE_POINTS));
        self.samples.insert(k, s.clone());
        Some(s)
    }

    /// A DXF file, read once.
    pub fn dxf_file(
        &mut self,
        path: &str,
        unit: Option<f64>,
    ) -> Result<Arc<creabim_exchange::dxf::DxfImport>, String> {
        let k = key(&RefSource::Dxf {
            path: path.into(),
            unit,
        });
        if let Some(d) = self.dxf.get(&k) {
            return Ok(d.clone());
        }
        let d = Arc::new(dxf_import(path, unit)?);
        self.dxf.insert(k, d.clone());
        Ok(d)
    }

    /// A DXF drawing or a PDF page's drawing, read once; none for a
    /// scanned page (a picture).
    pub fn drawing(
        &mut self,
        source: &RefSource,
    ) -> Result<Option<Arc<creabim_exchange::dxf::DxfImport>>, String> {
        match source {
            RefSource::Dxf { path, unit } => self.dxf_file(path, *unit).map(Some),
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
            self.dxf.remove(&k);
            self.scans.remove(&k);
            self.clouds.remove(&k);
            self.samples.remove(&k);
        }
        self.slices.retain(|(r, _), _| *r != id);
        self.indexes.remove(&id);
    }
}

/// A DXF file read whole.
pub fn dxf_import(
    path: &str,
    unit: Option<f64>,
) -> Result<creabim_exchange::dxf::DxfImport, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("couldn't read {path}: {e}"))?;
    let d = creabim_exchange::dxf::read(&creabim_exchange::dxf::decode(&bytes), unit);
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
    let mut layer_of: Vec<u32> = vec![];
    let mut layers: Vec<String> = vec![];
    let mut put = |layer: &str, pts: &[DVec2]| {
        let k = match layers.iter().position(|l| l == layer) {
            Some(k) => k,
            None => {
                layers.push(layer.to_string());
                layers.len() - 1
            }
        } as u32;
        for l in clip_line(pts, clip) {
            lines.push(l);
            layer_of.push(k);
        }
    };
    for p in d.polys.iter().filter(|p| on(&p.layer)) {
        let mut pts = p.pts.clone();
        if p.closed {
            pts.push(pts[0]);
        }
        put(&p.layer, &pts);
    }
    for f in d.fills.iter().filter(|f| on(&f.layer)) {
        for r in &f.rings {
            let mut pts = r.clone();
            pts.push(r[0]);
            put(&f.layer, &pts);
        }
    }
    for f in d.faces.iter().filter(|f| on(&f.layer)) {
        let mut pts: Vec<DVec2> = f.pts.iter().map(|q| q.truncate()).collect();
        pts.push(pts[0]);
        put(&f.layer, &pts);
    }
    Linework {
        lines,
        layer_of,
        layers,
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
            layer_of: vec![],
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
        RefSource::Dxf { path, unit } => match dxf_import(path, *unit) {
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
        RefSource::Points { path, .. } => match creabim_exchange::cloud::read_file(path) {
            Ok(c) => Shown::Cloud(Arc::new(c)),
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
        (_, Shown::Cloud(c)) => Some(c.bounds()),
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
        RefSource::Image { path, .. }
        | RefSource::Dxf { path, .. }
        | RefSource::Points { path, .. } => file(path),
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
    // a cloud is in the model: a section or an elevation shows what of it
    // stands in front of its line, within its depth
    for (id, r) in clouds_in_section(model, view) {
        if r.over != over || r.opacity <= 0.0 {
            continue;
        }
        if let Some(s) = cache.section(model, id, r, view) {
            let ink = colour(r).map(|c| c.gamma_multiply(r.opacity.clamp(0.0, 1.0)));
            let mut dots = Dots::new(painter.clip_rect());
            for (n, p) in s.points.iter().enumerate() {
                dots.add(to_screen(*p), || {
                    ink.unwrap_or_else(|| own_colour(s.rgb.get(n), r.opacity))
                });
            }
            dots.paint(painter);
        }
    }
    for (id, r) in creabim_core::reference::in_view(model, view) {
        if r.hidden || r.over != over || r.opacity <= 0.0 {
            continue;
        }
        let shown = cache.get(model, id, r);
        if let Shown::Cloud(_) = shown {
            if let Some(s) = cache.slice(model, id, r, view) {
                let ink = colour(r).map(|c| c.gamma_multiply(r.opacity.clamp(0.0, 1.0)));
                let mut dots = Dots::new(painter.clip_rect());
                for (n, p) in s.points.iter().enumerate() {
                    dots.add(to_screen(r.place(*p)), || {
                        ink.unwrap_or_else(|| own_colour(s.rgb.get(n), r.opacity))
                    });
                }
                dots.paint(painter);
            }
            continue;
        }
        draw(painter, r, &shown, to_screen);
    }
}

/// The clouds a section or an elevation `view` shows: every cloud not
/// hidden (a cloud is placed in plans; a section cuts it where it stands).
pub fn clouds_in_section(model: &Model, view: ElementId) -> Vec<(ElementId, &Reference)> {
    let is_section = matches!(
        model.get(view).map(|e| &e.kind),
        Some(creabim_core::ElementKind::View(v)) if matches!(v.kind, creabim_core::ViewKind::Section { .. })
    );
    if !is_section {
        return vec![];
    }
    model
        .iter()
        .filter_map(|e| match &e.kind {
            creabim_core::ElementKind::Reference(r)
                if !r.hidden && matches!(r.source, RefSource::Points { .. }) =>
            {
                Some((e.id, r))
            }
            _ => None,
        })
        .collect()
}

/// A cloud point's own colour, or grey where its file has none.
pub fn own_colour(rgb: Option<&[u8; 3]>, opacity: f32) -> Color32 {
    rgb.map_or(Color32::from_gray(70), |c| {
        Color32::from_rgb(c[0], c[1], c[2])
    })
    .gamma_multiply(opacity.clamp(0.0, 1.0))
}

/// Points drawn as dots of two pixels, one a place on the screen however
/// many fall there, so a cloud costs what the pane's pixels do.
pub struct Dots {
    rect: egui::Rect,
    w: usize,
    seen: Vec<u64>,
    mesh: egui::Mesh,
}

impl Dots {
    const SIZE: f32 = 2.0;

    pub fn new(rect: egui::Rect) -> Self {
        let w = (rect.width() / Self::SIZE).ceil().max(1.0) as usize;
        let h = (rect.height() / Self::SIZE).ceil().max(1.0) as usize;
        Self {
            rect,
            w,
            seen: vec![0; (w * h).div_ceil(64)],
            mesh: egui::Mesh::default(),
        }
    }

    /// A dot at `p`, of the colour `ink` gives, unless one is there.
    pub fn add(&mut self, p: Pos2, ink: impl FnOnce() -> Color32) {
        if !self.rect.contains(p) {
            return;
        }
        let i = ((p.x - self.rect.left()) / Self::SIZE) as usize;
        let j = ((p.y - self.rect.top()) / Self::SIZE) as usize;
        let n = j * self.w + i.min(self.w - 1);
        let Some(word) = self.seen.get_mut(n / 64) else {
            return;
        };
        if *word & (1 << (n % 64)) != 0 {
            return;
        }
        *word |= 1 << (n % 64);
        let at = self.rect.left_top() + egui::vec2(i as f32 * Self::SIZE, j as f32 * Self::SIZE);
        self.mesh.add_colored_rect(
            egui::Rect::from_min_size(at, egui::Vec2::splat(Self::SIZE)),
            ink(),
        );
    }

    pub fn paint(self, painter: &egui::Painter) {
        if !self.mesh.is_empty() {
            painter.add(egui::Shape::mesh(self.mesh));
        }
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
            // each layer as its style has it (a map's boundaries black, its
            // contours dashed brown), else in the drawing's ink
            let styles: Vec<Option<&creabim_core::LayerStyle>> = lines
                .layers
                .iter()
                .map(|l| r.layer_styles.iter().find(|s| s.layer == *l))
                .collect();
            for (i, line) in lines.iter().enumerate() {
                let pts: Vec<Pos2> = line.iter().map(|p| to_screen(r.place(*p))).collect();
                if pts.len() < 2 {
                    continue;
                }
                let style = lines
                    .layer_of
                    .get(i)
                    .and_then(|k| styles.get(*k as usize).copied().flatten());
                match style {
                    Some(st) => {
                        let [cr, cg, cb] = st.rgb;
                        let s =
                            Stroke::new(st.width, Color32::from_rgb(cr, cg, cb).gamma_multiply(a));
                        if st.dashed {
                            painter.extend(egui::Shape::dashed_line(&pts, s, 6.0, 4.0));
                        } else {
                            painter.add(egui::Shape::line(pts, s));
                        }
                    }
                    None => {
                        painter.add(egui::Shape::line(pts, stroke));
                    }
                }
            }
            draw_texts(painter, r, &lines.texts, ink, to_screen);
        }
        Shown::Missing(_) | Shown::Cloud(_) => {}
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
/// The picture files a reference can be.
pub const PICTURES: &[&str] = &["png", "jpg", "jpeg", "tif", "tiff", "bmp"];

/// A scan's resolution when its file doesn't say (PilonDesigner's).
pub const DEFAULT_DPI: f64 = 150.0;

/// A picture's size in pixels.
pub fn picture_px(path: &str) -> Option<(u32, u32)> {
    image::image_dimensions(path).ok()
}

/// A picture as a reference: as wide as its pixels at its resolution
/// (the file's, else 150 DPI) drawn at 1:100.
pub fn picture_source(path: &str) -> RefSource {
    let dpi = std::fs::read(path).ok().and_then(|b| file_dpi(&b));
    let width = picture_px(path).map_or(10.0, |(w, _)| {
        w as f64 / dpi.unwrap_or(DEFAULT_DPI) * 0.0254 * 100.0
    });
    RefSource::Image {
        path: path.to_string(),
        width,
        dpi,
    }
}

/// The resolution a picture file gives (PNG pHYs, JPEG JFIF, TIFF
/// XResolution), in dots per inch; none when it gives none, or less than
/// 50 (a screen's 72 or 96 says nothing about a scan, so 72 and 96 are
/// taken only from TIFF, which scanners write).
pub fn file_dpi(b: &[u8]) -> Option<f64> {
    let be32 = |i: usize| {
        b.get(i..i + 4)
            .map(|x| u32::from_be_bytes([x[0], x[1], x[2], x[3]]))
    };
    let dpi = if b.starts_with(b"\x89PNG") {
        let mut i = 8;
        let mut found = None;
        while let Some(len) = be32(i) {
            let kind = b.get(i + 4..i + 8)?;
            if kind == b"pHYs" {
                let (x, unit) = (be32(i + 8)?, *b.get(i + 16)?);
                found = (unit == 1).then_some(x as f64 * 0.0254);
                break;
            }
            if kind == b"IDAT" {
                break;
            }
            i += 12 + len as usize;
        }
        found
    } else if b.starts_with(&[0xff, 0xd8]) {
        let mut i = 2;
        let mut found = None;
        while i + 4 <= b.len() && b[i] == 0xff {
            let len = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
            if b[i + 1] == 0xe0 && b.get(i + 4..i + 9) == Some(b"JFIF\0") {
                let unit = *b.get(i + 11)?;
                let x = u16::from_be_bytes([*b.get(i + 12)?, *b.get(i + 13)?]) as f64;
                found = match unit {
                    1 => Some(x),
                    2 => Some(x * 2.54),
                    _ => None,
                };
                break;
            }
            if b[i + 1] == 0xda {
                break;
            }
            i += 2 + len;
        }
        found.filter(|d| *d > 96.0)
    } else if b.starts_with(b"II*\0") || b.starts_with(b"MM\0*") {
        let le = b[0] == b'I';
        let u16_at = |i: usize| {
            b.get(i..i + 2).map(|x| {
                if le {
                    u16::from_le_bytes([x[0], x[1]])
                } else {
                    u16::from_be_bytes([x[0], x[1]])
                }
            })
        };
        let u32_at = |i: usize| {
            b.get(i..i + 4).map(|x| {
                let a = [x[0], x[1], x[2], x[3]];
                if le {
                    u32::from_le_bytes(a)
                } else {
                    u32::from_be_bytes(a)
                }
            })
        };
        let ifd = u32_at(4)? as usize;
        let n = u16_at(ifd)? as usize;
        let (mut x, mut unit) = (None, 2);
        for k in 0..n {
            let e = ifd + 2 + k * 12;
            match u16_at(e)? {
                282 => {
                    let at = u32_at(e + 8)? as usize;
                    let (num, den) = (u32_at(at)?, u32_at(at + 4)?);
                    x = (den > 0).then(|| num as f64 / den as f64);
                }
                296 => unit = u16_at(e + 8)?,
                _ => {}
            }
        }
        match unit {
            2 => x,
            3 => x.map(|v| v * 2.54),
            _ => None,
        }
    } else {
        None
    };
    dpi.filter(|d| (50.0..=10000.0).contains(d))
}

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

/// What sheets show of a view's references, read once per model change:
/// the references themselves, and the items each view made of them.
type Made = HashMap<(ElementId, bool), (u64, Arc<Vec<PlanItem>>)>;

#[derive(Default)]
struct SheetRefs {
    cache: RefCache,
    made: Made,
}

static SHEET_REFS: std::sync::Mutex<Option<SheetRefs>> = std::sync::Mutex::new(None);

/// The references shown in `view` (under the model, or `over` it) as plan
/// items in model metres, for sheets, their PDF and print: a map's or a
/// plot plan's lines in their layers' styles and its texts upright, a
/// picture where it lies square (registered with
/// [`creabim_geom::refs::set_provider`]).
pub fn sheet_items(model: &Model, view: ElementId, over: bool) -> Vec<PlanItem> {
    let refs = creabim_core::reference::in_view(model, view);
    if refs.iter().all(|(_, r)| r.hidden || r.over != over) {
        return vec![];
    }
    let mut guard = SHEET_REFS.lock().unwrap_or_else(|e| e.into_inner());
    let s = guard.get_or_insert_with(SheetRefs::default);
    let rev = model.revision();
    if let Some((r, items)) = s.made.get(&(view, over)) {
        if *r == rev {
            return items.as_ref().clone();
        }
    }
    let (scale, turn) = match model.get(view).map(|e| &e.kind) {
        Some(creabim_core::ElementKind::View(v)) => (
            creabim_core::styles::resolved(model, v).paper_scale(),
            v.turn,
        ),
        _ => (100.0, 0.0),
    };
    // drawing metres to a paper millimetre
    let mm = scale / 1000.0;
    let mut out = Vec::new();
    for (id, r) in refs {
        if r.hidden || r.over != over || r.opacity <= 0.0 {
            continue;
        }
        let a = r.opacity.clamp(0.0, 1.0);
        // faded towards the paper as the screen fades it
        let fade = |c: [u8; 3]| c.map(|v| 1.0 - (1.0 - v as f32 / 255.0) * a);
        let own = r
            .colour
            .map(|i| REFERENCE_COLOURS[(i as usize).min(REFERENCE_COLOURS.len() - 1)].1);
        match s.cache.get(model, id, r) {
            Shown::Lines(lines) => {
                let ink = fade(own.unwrap_or([70, 70, 70]));
                let styles: Vec<Option<&creabim_core::LayerStyle>> = lines
                    .layers
                    .iter()
                    .map(|l| r.layer_styles.iter().find(|s| s.layer == *l))
                    .collect();
                for (i, line) in lines.iter().enumerate() {
                    if line.len() < 2 {
                        continue;
                    }
                    let style = lines
                        .layer_of
                        .get(i)
                        .and_then(|k| styles.get(*k as usize).copied().flatten());
                    out.push(PlanItem::Curve {
                        id,
                        points: line.iter().map(|p| r.place(*p)).collect(),
                        rgb: style.map_or(ink, |st| fade(st.rgb)),
                        dashed: style.is_some_and(|st| st.dashed),
                    });
                }
                let x = r.place(DVec2::X) - r.place(DVec2::ZERO);
                let (metre, turned) = (x.length(), x.y.atan2(x.x));
                for t in &lines.texts {
                    let size = t.h * metre / mm;
                    if !(0.8..=40.0).contains(&size) || t.text.trim().is_empty() {
                        continue;
                    }
                    // 1 2 3 top, 4 5 6 middle, 7 8 9 bottom; left, centre, right
                    let (col, row) = ((t.align.max(1) - 1) % 3, (t.align.max(1) - 1) / 3);
                    let up = DVec2::from_angle(t.angle.to_radians() + turned).perp();
                    out.push(PlanItem::Text {
                        id,
                        at: r.place(t.at) + up * (row as f64 - 1.0) * t.h * metre / 2.0,
                        text: t.text.clone(),
                        size,
                        align: match col {
                            0 => creabim_geom::paper::Anchor::Start,
                            1 => creabim_geom::paper::Anchor::Middle,
                            _ => creabim_geom::paper::Anchor::End,
                        },
                        mm,
                        rgb: Some(ink),
                        bold: false,
                    });
                }
            }
            // a picture lies square on paper: one turned with its view or
            // by itself is left to the screen
            Shown::Picture(raster)
                if r.rotation.rem_euclid(360.0) < 1e-6 && turn.rem_euclid(360.0) < 1e-6 =>
            {
                let shown = Shown::Picture(raster.clone());
                if let Some([lo, hi]) = r.bounds.or_else(|| bounds_of(&r.source, &shown)) {
                    let (p, q) = (r.place(lo), r.place(hi));
                    out.push(PlanItem::Image {
                        id,
                        raster,
                        lo: p.min(q),
                        hi: p.max(q),
                        clip: None,
                    });
                }
            }
            _ => {}
        }
    }
    s.made.insert((view, over), (rev, Arc::new(out.clone())));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scans_resolution_is_read_from_its_file() {
        // PNG with pHYs 11811 px/m (300 dpi)
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend([0, 0, 0, 9]);
        png.extend(b"pHYs");
        png.extend(11811u32.to_be_bytes());
        png.extend(11811u32.to_be_bytes());
        png.extend([1, 0, 0, 0, 0]);
        assert!((file_dpi(&png).unwrap() - 300.0).abs() < 0.1);
        // JPEG JFIF at 200 dpi; at 72 it says nothing
        let jfif = |d: u16| {
            let mut j = vec![0xff, 0xd8, 0xff, 0xe0, 0, 16];
            j.extend(b"JFIF\0");
            j.extend([1, 1, 1]);
            j.extend(d.to_be_bytes());
            j.extend(d.to_be_bytes());
            j.extend([0, 0]);
            j
        };
        assert_eq!(file_dpi(&jfif(200)), Some(200.0));
        assert_eq!(file_dpi(&jfif(72)), None);
        // little-endian TIFF, XResolution 400/1 inch
        let mut t = b"II*\0".to_vec();
        t.extend(8u32.to_le_bytes());
        t.extend(1u16.to_le_bytes());
        t.extend(282u16.to_le_bytes());
        t.extend(5u16.to_le_bytes());
        t.extend(1u32.to_le_bytes());
        t.extend(26u32.to_le_bytes());
        t.extend(0u32.to_le_bytes());
        t.extend(400u32.to_le_bytes());
        t.extend(1u32.to_le_bytes());
        assert_eq!(file_dpi(&t), Some(400.0));
        assert_eq!(file_dpi(b"nothing"), None);
    }

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
    fn a_cloud_reference_shows_a_plan_its_slice_and_snaps_to_it() {
        let m = creabim_core::sample::sample_house();
        let view = m
            .iter()
            .find(|e| matches!(&e.kind, creabim_core::ElementKind::View(v) if matches!(v.kind, creabim_core::ViewKind::Plan { .. })))
            .unwrap()
            .id;
        let creabim_core::ElementKind::View(v) = &m.get(view).unwrap().kind else {
            panic!()
        };
        let creabim_core::ViewKind::Plan { level, .. } = &v.kind else {
            panic!()
        };
        let z = m.level(*level).unwrap().elevation;
        // a wall's face scanned at two heights above the plan's level
        let path = std::env::temp_dir().join("creabim-trace-cloud.xyz");
        let mut t = String::new();
        for i in 0..100 {
            t += &format!("{} 3.0 {}\n", i as f64 * 0.05, z + 1.2);
            t += &format!("{} 4.0 {}\n", i as f64 * 0.05, z + 2.6);
        }
        std::fs::write(&path, t).unwrap();
        let src = RefSource::points(path.to_string_lossy().into_owned());
        let shown = read(&m, &src);
        let Shown::Cloud(c) = &shown else { panic!() };
        assert_eq!(c.points.len(), 200);
        let mut r = Reference::new(src, vec![view]);
        r.in_place(bounds_of(&r.source, &shown).unwrap());
        let id = ElementId::new();
        let mut cache = RefCache::default();
        let s = cache.slice(&m, id, &r, view).unwrap();
        assert_eq!(s.points.len(), 100, "only the points in the slice");
        assert!(s.points.iter().all(|p| (p.y - 3.0).abs() < 1e-6));
        // lifted 1,4 m, the other row is in the slice
        if let RefSource::Points { lift, .. } = &mut r.source {
            *lift = -1.4;
        }
        let s = cache.slice(&m, id, &r, view).unwrap();
        assert!(s.points.iter().all(|p| (p.y - 4.0).abs() < 1e-6));
        // the pointer near a point of it snaps there
        let mut snapper = crate::snaps::Snapper::new();
        snapper.add_cloud(s, &r, id);
        let project = |p: glam::DVec3| Some(egui::pos2(p.x as f32 * 100.0, p.y as f32 * 100.0));
        let free = glam::DVec3::new(1.012, 4.03, 0.0);
        let snap = snapper
            .snap(
                &crate::snaps::SnapSettings::default(),
                project(free).unwrap(),
                &project,
                None,
                Some(free),
            )
            .unwrap();
        assert!(
            snap.p.distance(glam::DVec3::new(1.0, 4.0, 0.0)) < 1e-6,
            "{:?}",
            snap.p
        );
        assert_eq!(snap.host, Some(id));
    }

    #[test]
    fn a_dxf_reference_reads_its_lines_in_metres() {
        let dir = std::env::temp_dir().join("creabim-trace-test.dxf");
        let text = "0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n0\nLINE\n8\n0\n10\n0\n20\n0\n11\n5000\n21\n0\n0\nENDSEC\n0\nEOF\n";
        std::fs::write(&dir, text).unwrap();
        let d = dxf_import(&dir.to_string_lossy(), None).unwrap();
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
