//! Reading a PDF page for a reference (a plan sent as PDF, a scanned
//! drawing), ported from PilonDesigner's `readPdfPage`: the page's lines
//! (strokes), solid fills in their colour and texts, at the drawing's
//! scale 1:N (a PDF point is 1/72 inch of paper; the model is paper × N),
//! in the same shape as a DXF drawing so it is shown, clipped and its
//! layers turned off the same way. A page with no lines (a scan) comes
//! back as its picture instead.

use crate::dxf::{DxfImport, Fill, Poly, Text};
use creabim_geom::raster::Raster;
use glam::DVec2;
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::HashMap;

/// The layers a page's content goes on.
pub const LAYER_LINES: &str = "PDF";
pub const LAYER_FILLS: &str = "PDF výplne";
pub const LAYER_TEXTS: &str = "PDF texty";

/// Most items read from one page; a page with more is read in part.
const MAX_POLYS: usize = 300_000;
const MAX_FILLS: usize = 100_000;
/// A scan is shown at most this many pixels on its longer side.
const MAX_PX: usize = 4096;

/// A scanned page's picture and where it lies on the page, metres.
#[derive(Debug)]
pub struct Scan {
    pub raster: Raster,
    pub rect: [DVec2; 2],
}

/// One page read.
#[derive(Debug)]
pub struct PdfPage {
    /// Its lines, fills and texts, metres at the scale asked for.
    pub imp: DxfImport,
    /// How many pages the file has.
    pub pages: u32,
    /// The page's size, metres.
    pub size: DVec2,
    /// The picture, when the page is a scan.
    pub scan: Option<Scan>,
    /// A picture was there but in a format not read (JBIG2, CCITT, JPEG
    /// 2000).
    pub scan_unread: bool,
}

/// How many pages a PDF has.
pub fn page_count(bytes: &[u8]) -> Result<u32, String> {
    Ok(load(bytes)?.get_pages().len() as u32)
}

fn load(bytes: &[u8]) -> Result<Document, String> {
    let mut doc =
        Document::load_mem(bytes).map_err(|e| format!("not a PDF that can be read: {e}"))?;
    if doc.is_encrypted() {
        doc.decrypt("")
            .map_err(|_| "the PDF is protected by a password".to_string())?;
    }
    Ok(doc)
}

/// Page `page` (from 1) of a PDF, at scale 1:`scale`.
pub fn read(bytes: &[u8], page: u32, scale: f64) -> Result<PdfPage, String> {
    let doc = load(bytes)?;
    let pages = doc.get_pages();
    let n = pages.len() as u32;
    if n == 0 {
        return Err("the PDF has no pages".into());
    }
    let page_no = page.clamp(1, n);
    let pid = pages[&page_no];
    let unit = 0.0254 / 72.0 * scale.max(1e-9);
    let pdict = doc.get_dictionary(pid).map_err(|e| e.to_string())?;
    let mbox = inherited(&doc, pdict, b"CropBox")
        .or_else(|| inherited(&doc, pdict, b"MediaBox"))
        .and_then(|o| numbers(&doc, o))
        .filter(|v| v.len() == 4)
        .map(|v| {
            [
                DVec2::new(v[0].min(v[2]), v[1].min(v[3])),
                DVec2::new(v[0].max(v[2]), v[1].max(v[3])),
            ]
        })
        .unwrap_or([DVec2::ZERO, DVec2::new(595.0, 842.0)]);
    let rotate = inherited(&doc, pdict, b"Rotate")
        .and_then(|o| num(&doc, o))
        .map_or(0, |r| (r as i64).rem_euclid(360) / 90 * 90);
    // the page as it is seen: turned clockwise by /Rotate, its lower left
    // corner at the origin, in metres
    let view = view_matrix(mbox, rotate, unit);
    let content = doc.get_page_content(pid).unwrap_or_default();
    let (res, res_ids) = doc.get_page_resources(pid).map_err(|e| e.to_string())?;
    let mut resources = res.cloned().unwrap_or_default();
    for id in res_ids {
        if let Ok(d) = doc.get_dictionary(id) {
            for (k, v) in d.iter() {
                if !resources.has(k) {
                    resources.set(k.clone(), v.clone());
                }
            }
        }
    }
    let page_area = {
        let d = mbox[1] - mbox[0];
        d.x * d.y
    };
    let mut r = Interp {
        doc: &doc,
        out: DxfImport {
            unit,
            ..Default::default()
        },
        page_area,
        images: vec![],
        fonts: HashMap::new(),
        last_text: None,
    };
    let gs = GState {
        ctm: view,
        ..GState::default()
    };
    r.run(&content, &resources, gs, 0);
    let Interp {
        mut out, images, ..
    } = r;
    out.texts.retain(|t| !t.text.trim().is_empty());
    let vectors = out.polys.len() + out.fills.len();
    let mut scan = None;
    let mut scan_unread = false;
    if vectors + out.texts.len() == 0 || (!images.is_empty() && vectors < 5) {
        if let Some(im) = images.iter().max_by(|a, b| a.area.total_cmp(&b.area)) {
            match picture(&doc, im) {
                Some(s) => scan = Some(s),
                None => scan_unread = true,
            }
        }
    }
    let mut layers: Vec<String> = vec![];
    let mut lo = DVec2::splat(f64::INFINITY);
    let mut hi = DVec2::splat(f64::NEG_INFINITY);
    let mut see = |l: &str, ps: &mut dyn Iterator<Item = DVec2>| {
        if !layers.iter().any(|x| x == l) {
            layers.push(l.to_string());
        }
        for p in ps {
            lo = lo.min(p);
            hi = hi.max(p);
        }
    };
    for p in &out.polys {
        see(&p.layer, &mut p.pts.iter().copied());
    }
    for f in &out.fills {
        see(&f.layer, &mut f.rings.iter().flatten().copied());
    }
    for t in &out.texts {
        see(&t.layer, &mut std::iter::once(t.p));
    }
    out.layers = layers;
    out.bounds = lo.x.is_finite().then_some([lo, hi]);
    let d = (mbox[1] - mbox[0]) * unit;
    let size = if rotate % 180 == 0 {
        d
    } else {
        DVec2::new(d.y, d.x)
    };
    Ok(PdfPage {
        imp: out,
        pages: n,
        size,
        scan,
        scan_unread,
    })
}

/// An affine map `[a b c d e f]`: (x, y) → (a x + c y + e, b x + d y + f).
type M6 = [f64; 6];

const IDENT: M6 = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `a` then `b`.
fn mul(a: &M6, b: &M6) -> M6 {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}

fn ap(m: &M6, x: f64, y: f64) -> DVec2 {
    DVec2::new(m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

/// User space to the seen page in metres.
fn view_matrix(mbox: [DVec2; 2], rotate: i64, unit: f64) -> M6 {
    let [lo, hi] = mbox;
    let (w, h) = (hi.x - lo.x, hi.y - lo.y);
    let to_origin = [1.0, 0.0, 0.0, 1.0, -lo.x, -lo.y];
    // turned clockwise, then back to positive coordinates
    let turn = match rotate {
        90 => [0.0, -1.0, 1.0, 0.0, 0.0, w],
        180 => [-1.0, 0.0, 0.0, -1.0, w, h],
        270 => [0.0, 1.0, -1.0, 0.0, h, 0.0],
        _ => IDENT,
    };
    let scale = [unit, 0.0, 0.0, unit, 0.0, 0.0];
    mul(&mul(&to_origin, &turn), &scale)
}

/// A page attribute, from the page or the pages above it.
fn inherited<'a>(doc: &'a Document, d: &'a Dictionary, key: &[u8]) -> Option<&'a Object> {
    let mut d = d;
    for _ in 0..32 {
        if let Ok(o) = d.get_deref(key, doc) {
            return Some(o);
        }
        d = d.get_deref(b"Parent", doc).ok()?.as_dict().ok()?;
    }
    None
}

fn num(doc: &Document, o: &Object) -> Option<f64> {
    match doc.dereference(o).ok()?.1 {
        Object::Integer(i) => Some(*i as f64),
        Object::Real(r) => Some(*r as f64),
        _ => None,
    }
}

fn numbers(doc: &Document, o: &Object) -> Option<Vec<f64>> {
    doc.dereference(o)
        .ok()?
        .1
        .as_array()
        .ok()?
        .iter()
        .map(|x| num(doc, x))
        .collect()
}

fn name(o: &Object) -> Option<&[u8]> {
    o.as_name().ok()
}

#[derive(Clone, Debug)]
struct GState {
    ctm: M6,
    fill: u32,
    font: Option<Vec<u8>>,
    size: f64,
    tc: f64,
    tw: f64,
    th: f64,
    tl: f64,
    rise: f64,
    mode: i64,
}

impl Default for GState {
    fn default() -> Self {
        GState {
            ctm: IDENT,
            fill: 0,
            font: None,
            size: 10.0,
            tc: 0.0,
            tw: 0.0,
            th: 1.0,
            tl: 0.0,
            rise: 0.0,
            mode: 0,
        }
    }
}

/// A font as text is measured and decoded with it.
struct Font {
    two_byte: bool,
    widths: HashMap<u32, f64>,
    default_w: f64,
    /// Its ToUnicode map: a code's text.
    unicode: HashMap<u32, String>,
    /// Characters its /Differences give codes.
    diffs: HashMap<u32, char>,
}

/// An image painted on the page.
struct Placed {
    id: Option<ObjectId>,
    ctm: M6,
    area: f64,
}

struct Interp<'a> {
    doc: &'a Document,
    out: DxfImport,
    page_area: f64,
    images: Vec<Placed>,
    fonts: HashMap<(usize, Vec<u8>), Font>,
    /// The text last written and where it ended, so pieces of one line
    /// written apart are joined.
    last_text: Option<(usize, DVec2)>,
}

/// A path being built: its subpaths.
#[derive(Default)]
struct Path {
    subs: Vec<(Vec<DVec2>, bool)>,
    open: bool,
    last: (f64, f64),
}

impl Path {
    fn point(&mut self, p: DVec2) {
        if !self.open {
            self.subs.push((vec![], false));
            self.open = true;
        }
        if let Some(s) = self.subs.last_mut() {
            s.0.push(p);
        }
    }
}

fn bez(p0: (f64, f64), p1: (f64, f64), p2: (f64, f64), p3: (f64, f64)) -> Vec<(f64, f64)> {
    (1..=8)
        .map(|k| {
            let t = k as f64 / 8.0;
            let s = 1.0 - t;
            let f = |a: f64, b: f64, c: f64, d: f64| {
                s * s * s * a + 3.0 * s * s * t * b + 3.0 * s * t * t * c + t * t * t * d
            };
            (f(p0.0, p1.0, p2.0, p3.0), f(p0.1, p1.1, p2.1, p3.1))
        })
        .collect()
}

fn ring_area(r: &[DVec2]) -> f64 {
    (0..r.len())
        .map(|i| r[i].perp_dot(r[(i + 1) % r.len()]))
        .sum::<f64>()
        .abs()
        / 2.0
}

fn rgb(c: &[f64]) -> u32 {
    let b = |v: f64| ((v.clamp(0.0, 1.0) * 255.0).round() as u32) & 255;
    match c.len() {
        1 => b(c[0]) * 0x010101,
        3 => b(c[0]) << 16 | b(c[1]) << 8 | b(c[2]),
        4 => {
            let k = 1.0 - c[3];
            b((1.0 - c[0]) * k) << 16 | b((1.0 - c[1]) * k) << 8 | b((1.0 - c[2]) * k)
        }
        _ => 0,
    }
}

impl<'a> Interp<'a> {
    fn run(&mut self, content: &[u8], res: &'a Dictionary, gs: GState, depth: usize) {
        let Ok(ops) = lopdf::content::Content::decode(content) else {
            return;
        };
        let res_key = res as *const Dictionary as usize;
        let mut gs = gs;
        let mut stack: Vec<GState> = vec![];
        let mut path = Path::default();
        let mut tm = IDENT;
        let mut tlm = IDENT;
        for op in &ops.operations {
            let a = &op.operands;
            let f = |i: usize| a.get(i).and_then(|o| num(self.doc, o)).unwrap_or(0.0);
            let fs = || -> Vec<f64> { a.iter().filter_map(|o| num(self.doc, o)).collect() };
            let user = |gs: &GState, x: f64, y: f64| ap(&gs.ctm, x, y);
            match op.operator.as_str() {
                "q" => stack.push(gs.clone()),
                "Q" => {
                    if let Some(g) = stack.pop() {
                        gs = g;
                    }
                }
                "cm" if a.len() == 6 => {
                    let m = [f(0), f(1), f(2), f(3), f(4), f(5)];
                    gs.ctm = mul(&m, &gs.ctm);
                }
                "m" => {
                    path.open = false;
                    path.last = (f(0), f(1));
                    path.point(user(&gs, f(0), f(1)));
                }
                "l" => {
                    path.last = (f(0), f(1));
                    path.point(user(&gs, f(0), f(1)));
                }
                "c" | "v" | "y" => {
                    let (p1, p2, p3) = match op.operator.as_str() {
                        "c" => ((f(0), f(1)), (f(2), f(3)), (f(4), f(5))),
                        "v" => (path.last, (f(0), f(1)), (f(2), f(3))),
                        _ => ((f(0), f(1)), (f(2), f(3)), (f(2), f(3))),
                    };
                    if !path.open {
                        path.point(user(&gs, path.last.0, path.last.1));
                    }
                    for q in bez(path.last, p1, p2, p3) {
                        path.point(user(&gs, q.0, q.1));
                    }
                    path.last = p3;
                }
                "h" => {
                    if let (true, Some(s)) = (path.open, path.subs.last_mut()) {
                        s.1 = true;
                    }
                    path.open = false;
                }
                "re" => {
                    let (x, y, w, h) = (f(0), f(1), f(2), f(3));
                    path.subs.push((
                        vec![
                            user(&gs, x, y),
                            user(&gs, x + w, y),
                            user(&gs, x + w, y + h),
                            user(&gs, x, y + h),
                        ],
                        true,
                    ));
                    path.open = false;
                    path.last = (x, y);
                }
                "S" | "s" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" | "n" => {
                    let o = op.operator.as_str();
                    if matches!(o, "s" | "b" | "b*") {
                        for s in &mut path.subs {
                            s.1 = true;
                        }
                    }
                    if matches!(o, "f" | "F" | "f*" | "B" | "B*" | "b" | "b*") {
                        self.fill(&path, gs.fill);
                    }
                    if matches!(o, "S" | "s" | "B" | "B*" | "b" | "b*") {
                        self.stroke(&path);
                    }
                    path = Path::default();
                }
                "g" | "rg" | "k" | "sc" | "scn" => {
                    let c = fs();
                    if !c.is_empty() {
                        gs.fill = rgb(&c);
                    }
                }
                "BT" => {
                    tm = IDENT;
                    tlm = IDENT;
                }
                "Tf" => {
                    gs.font = a.first().and_then(name).map(<[u8]>::to_vec);
                    gs.size = f(1);
                }
                "Tc" => gs.tc = f(0),
                "Tw" => gs.tw = f(0),
                "Tz" => gs.th = f(0) / 100.0,
                "TL" => gs.tl = f(0),
                "Ts" => gs.rise = f(0),
                "Tr" => gs.mode = f(0) as i64,
                "Td" | "TD" => {
                    if op.operator == "TD" {
                        gs.tl = -f(1);
                    }
                    tlm = mul(&[1.0, 0.0, 0.0, 1.0, f(0), f(1)], &tlm);
                    tm = tlm;
                }
                "Tm" if a.len() == 6 => {
                    tlm = [f(0), f(1), f(2), f(3), f(4), f(5)];
                    tm = tlm;
                }
                "T*" => {
                    tlm = mul(&[1.0, 0.0, 0.0, 1.0, 0.0, -gs.tl], &tlm);
                    tm = tlm;
                }
                "Tj" | "'" | "\"" | "TJ" => {
                    if op.operator != "Tj" && op.operator != "TJ" {
                        if op.operator == "\"" {
                            gs.tw = f(0);
                            gs.tc = f(1);
                        }
                        tlm = mul(&[1.0, 0.0, 0.0, 1.0, 0.0, -gs.tl], &tlm);
                        tm = tlm;
                    }
                    let items: Vec<&Object> = match a.last() {
                        Some(Object::Array(v)) if op.operator == "TJ" => v.iter().collect(),
                        Some(o) => vec![o],
                        None => vec![],
                    };
                    self.show(res, res_key, &gs, &mut tm, &items);
                }
                "Do" => {
                    let Some(n) = a.first().and_then(name) else {
                        continue;
                    };
                    self.paint(res, n, &gs, depth);
                }
                "BI" | "EI" | "ID" => self.images.push(Placed {
                    id: None,
                    ctm: gs.ctm,
                    area: (gs.ctm[0] * gs.ctm[3] - gs.ctm[1] * gs.ctm[2]).abs(),
                }),
                _ => {}
            }
        }
    }

    fn stroke(&mut self, path: &Path) {
        for (pts, closed) in &path.subs {
            if pts.len() >= 2 && self.out.polys.len() < MAX_POLYS {
                self.out.polys.push(Poly {
                    pts: pts.clone(),
                    closed: *closed,
                    layer: LAYER_LINES.into(),
                    dashed: false,
                    z: None,
                });
            }
        }
    }

    fn fill(&mut self, path: &Path, colour: u32) {
        let rings: Vec<Vec<DVec2>> = path
            .subs
            .iter()
            .filter(|s| s.0.len() >= 3)
            .map(|s| s.0.clone())
            .collect();
        // white areas and the page's background are left out: they would
        // hide what is drawn under them
        if rings.is_empty() || colour == 0xffffff || self.out.fills.len() >= MAX_FILLS {
            return;
        }
        let u2 = self.out.unit * self.out.unit;
        if rings.iter().map(|r| ring_area(r)).sum::<f64>() > self.page_area * u2 * 0.6 {
            return;
        }
        self.out.fills.push(Fill {
            rings,
            layer: LAYER_FILLS.into(),
            color: colour,
        });
    }

    /// An XObject painted: a form's content is read in its place, an
    /// image is counted (the largest is the page's scan).
    fn paint(&mut self, res: &'a Dictionary, n: &[u8], gs: &GState, depth: usize) {
        let doc = self.doc;
        let Some((id, stream)) = res
            .get_deref(b"XObject", doc)
            .ok()
            .and_then(|o| o.as_dict().ok())
            .and_then(|d| d.get(n).ok())
            .and_then(|o| doc.dereference(o).ok())
            .and_then(|(id, o)| Some((id, o.as_stream().ok()?)))
        else {
            return;
        };
        match stream.dict.get(b"Subtype").ok().and_then(name) {
            Some(b"Image") => {
                if stream
                    .dict
                    .get(b"ImageMask")
                    .ok()
                    .and_then(|o| o.as_bool().ok())
                    == Some(true)
                {
                    return;
                }
                let m = gs.ctm;
                self.images.push(Placed {
                    id,
                    ctm: m,
                    area: (m[0] * m[3] - m[1] * m[2]).abs(),
                });
            }
            Some(b"Form") if depth < 12 => {
                let mut g = gs.clone();
                if let Some(m) = stream
                    .dict
                    .get(b"Matrix")
                    .ok()
                    .and_then(|o| numbers(doc, o))
                    .filter(|v| v.len() == 6)
                {
                    g.ctm = mul(&[m[0], m[1], m[2], m[3], m[4], m[5]], &g.ctm);
                }
                let inner = stream
                    .dict
                    .get_deref(b"Resources", doc)
                    .ok()
                    .and_then(|o| o.as_dict().ok())
                    .unwrap_or(res);
                if let Ok(c) = stream.get_plain_content() {
                    self.run(&c, inner, g, depth + 1);
                }
            }
            _ => {}
        }
    }

    /// The font named `n` in `res`, read once.
    fn font(&mut self, res: &'a Dictionary, res_key: usize, n: &[u8]) -> Option<&Font> {
        let key = (res_key, n.to_vec());
        if !self.fonts.contains_key(&key) {
            let doc = self.doc;
            let d = res
                .get_deref(b"Font", doc)
                .ok()?
                .as_dict()
                .ok()?
                .get_deref(n, doc)
                .ok()?
                .as_dict()
                .ok()?;
            self.fonts.insert(key.clone(), read_font(doc, d));
        }
        self.fonts.get(&key)
    }

    fn show(
        &mut self,
        res: &'a Dictionary,
        res_key: usize,
        gs: &GState,
        tm: &mut M6,
        items: &[&Object],
    ) {
        let Some(fname) = gs.font.clone() else {
            return;
        };
        let size = gs.size;
        let doc = self.doc;
        let Some(font) = self.font(res, res_key, &fname) else {
            return;
        };
        let two = font.two_byte;
        let mut text = String::new();
        let start = *tm;
        let adv = |tm: &mut M6, tx: f64| *tm = mul(&[1.0, 0.0, 0.0, 1.0, tx, 0.0], tm);
        for it in items {
            match it {
                Object::String(bytes, _) => {
                    text.push_str(&decode(font, bytes));
                    let codes: Vec<u32> = if two {
                        bytes
                            .chunks(2)
                            .map(|c| (c[0] as u32) << 8 | *c.get(1).unwrap_or(&0) as u32)
                            .collect()
                    } else {
                        bytes.iter().map(|b| *b as u32).collect()
                    };
                    for c in codes {
                        let w = font.widths.get(&c).copied().unwrap_or(font.default_w);
                        let space = if !two && c == 32 { gs.tw } else { 0.0 };
                        adv(tm, (w / 1000.0 * size + gs.tc + space) * gs.th);
                    }
                }
                o => {
                    if let Some(k) = num(doc, o) {
                        adv(tm, -k / 1000.0 * size * gs.th);
                        // a wide kern is a space between words
                        if k < -250.0 && !text.ends_with(' ') {
                            text.push(' ');
                        }
                    }
                }
            }
        }
        if gs.mode == 3 || gs.mode == 7 || text.trim().is_empty() {
            return;
        }
        let m0 = mul(
            &mul(&[size * gs.th, 0.0, 0.0, size, 0.0, gs.rise], &start),
            &gs.ctm,
        );
        let m1 = mul(
            &mul(&[size * gs.th, 0.0, 0.0, size, 0.0, gs.rise], tm),
            &gs.ctm,
        );
        let p = ap(&m0, 0.0, 0.0);
        let end = ap(&m1, 0.0, 0.0);
        let h = DVec2::new(m0[2], m0[3]).length();
        let angle = m0[1].atan2(m0[0]).to_degrees();
        // a piece that goes on where the last one ended joins it
        if let Some((i, e)) = self.last_text {
            if let Some(t) = self.out.texts.get_mut(i) {
                let gap = e.distance(p);
                let same =
                    (t.h - h * 0.72).abs() < 0.1 * t.h.max(1e-12) && (t.angle - angle).abs() < 0.5;
                if same && gap < 0.3 * h {
                    if gap > 0.12 * h && !t.text.ends_with(' ') && !text.starts_with(' ') {
                        t.text.push(' ');
                    }
                    t.text.push_str(&text);
                    self.last_text = Some((i, end));
                    return;
                }
            }
        }
        self.out.texts.push(Text {
            p,
            text,
            h: h * 0.72,
            angle,
            layer: LAYER_TEXTS.into(),
            align: 7,
            wf: 1.0,
        });
        self.last_text = Some((self.out.texts.len() - 1, end));
    }
}

fn read_font<'a>(doc: &'a Document, d: &'a Dictionary) -> Font {
    let subtype = d.get(b"Subtype").ok().and_then(name).unwrap_or(b"");
    let two_byte = subtype == b"Type0";
    let mut widths = HashMap::new();
    let mut default_w = 500.0;
    if two_byte {
        let cid = d
            .get_deref(b"DescendantFonts", doc)
            .ok()
            .and_then(|o| o.as_array().ok())
            .and_then(|v| v.first())
            .and_then(|o| doc.dereference(o).ok())
            .and_then(|(_, o)| o.as_dict().ok());
        if let Some(cid) = cid {
            default_w = cid
                .get(b"DW")
                .ok()
                .and_then(|o| num(doc, o))
                .unwrap_or(1000.0);
            if let Some(w) = cid
                .get_deref(b"W", doc)
                .ok()
                .and_then(|o| o.as_array().ok())
            {
                let mut i = 0;
                while i + 1 < w.len() {
                    let first = num(doc, &w[i]).unwrap_or(0.0) as u32;
                    if let Ok(list) = doc
                        .dereference(&w[i + 1])
                        .map(|x| x.1)
                        .and_then(Object::as_array)
                    {
                        for (k, x) in list.iter().enumerate() {
                            widths.insert(first + k as u32, num(doc, x).unwrap_or(default_w));
                        }
                        i += 2;
                    } else if i + 2 < w.len() {
                        let last = num(doc, &w[i + 1]).unwrap_or(0.0) as u32;
                        let v = num(doc, &w[i + 2]).unwrap_or(default_w);
                        for c in first..=last.min(first + 65535) {
                            widths.insert(c, v);
                        }
                        i += 3;
                    } else {
                        break;
                    }
                }
            }
        }
    } else {
        let first = d
            .get(b"FirstChar")
            .ok()
            .and_then(|o| num(doc, o))
            .unwrap_or(0.0) as u32;
        if let Some(v) = d.get(b"Widths").ok().and_then(|o| numbers(doc, o)) {
            for (k, w) in v.into_iter().enumerate() {
                widths.insert(first + k as u32, w);
            }
        }
        if let Some(mw) = d
            .get_deref(b"FontDescriptor", doc)
            .ok()
            .and_then(|o| o.as_dict().ok())
            .and_then(|fd| fd.get(b"MissingWidth").ok())
            .and_then(|o| num(doc, o))
        {
            default_w = mw;
        }
    }
    let unicode = d
        .get_deref(b"ToUnicode", doc)
        .ok()
        .and_then(|o| o.as_stream().ok())
        .and_then(|s| s.get_plain_content().ok())
        .map(|c| to_unicode(&c))
        .unwrap_or_default();
    let mut diffs = HashMap::new();
    if let Some(v) = d
        .get_deref(b"Encoding", doc)
        .ok()
        .and_then(|o| o.as_dict().ok())
        .and_then(|e| e.get_deref(b"Differences", doc).ok())
        .and_then(|o| o.as_array().ok())
    {
        let mut code = 0u32;
        for o in v {
            match o {
                Object::Integer(i) => code = *i as u32,
                Object::Name(n) => {
                    if let Some(c) = glyph(&String::from_utf8_lossy(n)) {
                        diffs.insert(code, c);
                    }
                    code += 1;
                }
                _ => {}
            }
        }
    }
    Font {
        two_byte,
        widths,
        default_w,
        unicode,
        diffs,
    }
}

/// A ToUnicode CMap's codes and their text (bfchar and bfrange).
fn to_unicode(cmap: &[u8]) -> HashMap<u32, String> {
    let text = String::from_utf8_lossy(cmap);
    let hex = |h: &str| u32::from_str_radix(h, 16).ok();
    let utf16 = |h: &str| -> String {
        let units: Vec<u16> = (0..h.len() / 4)
            .filter_map(|i| u16::from_str_radix(&h[i * 4..i * 4 + 4], 16).ok())
            .collect();
        String::from_utf16_lossy(&units)
    };
    // the tokens: <hex>, [ and ], words
    let mut toks = vec![];
    let mut it = text.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        match c {
            '<' => {
                let end = text[i + 1..].find('>').map_or(text.len(), |e| i + 1 + e);
                toks.push(text[i..=end.min(text.len() - 1)].to_string());
                while it.peek().is_some_and(|(j, _)| *j <= end) {
                    it.next();
                }
            }
            '[' | ']' => toks.push(c.to_string()),
            c if c.is_whitespace() => {}
            _ => {
                let mut w = c.to_string();
                while let Some((_, d)) = it.peek() {
                    if d.is_whitespace() || matches!(d, '<' | '[' | ']') {
                        break;
                    }
                    w.push(*d);
                    it.next();
                }
                toks.push(w);
            }
        }
    }
    let inner = |t: &str| {
        t.trim_start_matches('<')
            .trim_end_matches('>')
            .replace(char::is_whitespace, "")
    };
    let mut map = HashMap::new();
    let mut i = 0;
    let mut mode = "";
    while i < toks.len() {
        let t = toks[i].as_str();
        match t {
            "beginbfchar" | "beginbfrange" => {
                mode = if t == "beginbfchar" { "char" } else { "range" }
            }
            "endbfchar" | "endbfrange" => mode = "",
            _ if mode == "char" && i + 1 < toks.len() => {
                if let Some(c) = hex(&inner(t)) {
                    map.insert(c, utf16(&inner(&toks[i + 1])));
                }
                i += 1;
            }
            _ if mode == "range" && i + 2 < toks.len() => {
                let (lo, hi) = (hex(&inner(t)), hex(&inner(&toks[i + 1])));
                if let (Some(lo), Some(hi)) = (lo, hi) {
                    if toks[i + 2] == "[" {
                        let mut k = i + 3;
                        let mut c = lo;
                        while k < toks.len() && toks[k] != "]" {
                            map.insert(c, utf16(&inner(&toks[k])));
                            c += 1;
                            k += 1;
                        }
                        i = k;
                    } else {
                        let dst = inner(&toks[i + 2]);
                        let units: Vec<u16> = (0..dst.len() / 4)
                            .filter_map(|j| u16::from_str_radix(&dst[j * 4..j * 4 + 4], 16).ok())
                            .collect();
                        for c in lo..=hi.min(lo + 65535) {
                            let mut u = units.clone();
                            if let Some(last) = u.last_mut() {
                                *last = last.wrapping_add((c - lo) as u16);
                            }
                            map.insert(c, String::from_utf16_lossy(&u));
                        }
                        i += 2;
                    }
                }
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }
    map
}

/// A glyph name's character: Slovak and Czech letters, `uniXXXX`, and
/// one-letter names.
fn glyph(n: &str) -> Option<char> {
    if let Some(h) = n.strip_prefix("uni").filter(|h| h.len() == 4) {
        return u32::from_str_radix(h, 16).ok().and_then(char::from_u32);
    }
    let mut cs = n.chars();
    if let (Some(c), None) = (cs.next(), cs.next()) {
        return Some(c);
    }
    const ACCENTS: [(&str, char, char); 23] = [
        ("aacute", 'á', 'Á'),
        ("adieresis", 'ä', 'Ä'),
        ("ccaron", 'č', 'Č'),
        ("dcaron", 'ď', 'Ď'),
        ("eacute", 'é', 'É'),
        ("ecaron", 'ě', 'Ě'),
        ("iacute", 'í', 'Í'),
        ("lacute", 'ĺ', 'Ĺ'),
        ("lcaron", 'ľ', 'Ľ'),
        ("ncaron", 'ň', 'Ň'),
        ("oacute", 'ó', 'Ó'),
        ("ocircumflex", 'ô', 'Ô'),
        ("odieresis", 'ö', 'Ö'),
        ("racute", 'ŕ', 'Ŕ'),
        ("rcaron", 'ř', 'Ř'),
        ("scaron", 'š', 'Š'),
        ("tcaron", 'ť', 'Ť'),
        ("uacute", 'ú', 'Ú'),
        ("uring", 'ů', 'Ů'),
        ("udieresis", 'ü', 'Ü'),
        ("yacute", 'ý', 'Ý'),
        ("zcaron", 'ž', 'Ž'),
        ("germandbls", 'ß', 'ß'),
    ];
    let lower = n.to_lowercase();
    if let Some((_, l, u)) = ACCENTS.iter().find(|(g, ..)| *g == lower) {
        return Some(if n.starts_with(|c: char| c.is_uppercase()) {
            *u
        } else {
            *l
        });
    }
    const WORDS: [(&str, char); 22] = [
        ("space", ' '),
        ("zero", '0'),
        ("one", '1'),
        ("two", '2'),
        ("three", '3'),
        ("four", '4'),
        ("five", '5'),
        ("six", '6'),
        ("seven", '7'),
        ("eight", '8'),
        ("nine", '9'),
        ("period", '.'),
        ("comma", ','),
        ("hyphen", '-'),
        ("colon", ':'),
        ("parenleft", '('),
        ("parenright", ')'),
        ("slash", '/'),
        ("percent", '%'),
        ("plus", '+'),
        ("equal", '='),
        ("degree", '°'),
    ];
    WORDS.iter().find(|(w, _)| *w == n).map(|(_, c)| *c)
}

/// A string's text in a font: through its ToUnicode map where it has
/// one, else its one-byte encoding, else WinAnsi (Windows-1252).
fn decode(font: &Font, bytes: &[u8]) -> String {
    if font.two_byte {
        return bytes
            .chunks(2)
            .map(|c| (c[0] as u32) << 8 | *c.get(1).unwrap_or(&0) as u32)
            .filter_map(|c| font.unicode.get(&c).map(String::as_str))
            .collect();
    }
    bytes
        .iter()
        .map(|b| {
            let c = *b as u32;
            match (font.unicode.get(&c), font.diffs.get(&c)) {
                (Some(u), _) if !u.is_empty() && u != "\0" => u.clone(),
                (_, Some(d)) => d.to_string(),
                _ => encoding_rs::WINDOWS_1252.decode(&[*b]).0.into_owned(),
            }
        })
        .collect()
}

/// A scan's picture, turned and flipped as it is painted on the page.
fn picture(doc: &Document, im: &Placed) -> Option<Scan> {
    let stream = doc.get_object(im.id?).ok()?.as_stream().ok()?;
    let d = &stream.dict;
    let w = d.get(b"Width").ok().and_then(|o| num(doc, o))? as usize;
    let h = d.get(b"Height").ok().and_then(|o| num(doc, o))? as usize;
    if w == 0 || h == 0 {
        return None;
    }
    let filters = stream.filters().unwrap_or_default();
    let src: image::RgbaImage = if filters.iter().any(|f| f == "DCTDecode") {
        image::load_from_memory_with_format(&stream.content, image::ImageFormat::Jpeg)
            .ok()?
            .to_rgba8()
    } else if filters.iter().all(|f| f == "FlateDecode") {
        let raw = if filters.is_empty() {
            stream.content.clone()
        } else {
            use std::io::Read;
            let mut v = vec![];
            flate2::read::ZlibDecoder::new(&stream.content[..])
                .read_to_end(&mut v)
                .ok()?;
            v
        };
        let bpc = d
            .get(b"BitsPerComponent")
            .ok()
            .and_then(|o| num(doc, o))
            .unwrap_or(8.0) as usize;
        let (ncomp, palette) = colour_space(doc, d.get(b"ColorSpace").ok());
        let predictor = d
            .get_deref(b"DecodeParms", doc)
            .ok()
            .and_then(|o| o.as_dict().ok())
            .and_then(|p| p.get(b"Predictor").ok())
            .and_then(|o| num(doc, o))
            .unwrap_or(1.0);
        let row = (w * ncomp * bpc).div_ceil(8);
        let raw = if predictor >= 10.0 {
            unpredict(&raw, row, (ncomp * bpc).div_ceil(8))?
        } else {
            raw
        };
        if raw.len() < row * h || !(bpc == 8 || bpc == 1 && ncomp == 1) {
            return None;
        }
        let mut px = image::RgbaImage::new(w as u32, h as u32);
        for y in 0..h {
            for x in 0..w {
                let r = &raw[y * row..];
                let c = if bpc == 1 {
                    let bit = (r[x / 8] >> (7 - x % 8)) & 1;
                    let v = bit * 255;
                    [v, v, v]
                } else if let Some(p) = &palette {
                    let i = r[x] as usize * 3;
                    [*p.get(i)?, *p.get(i + 1)?, *p.get(i + 2)?]
                } else {
                    match ncomp {
                        1 => [r[x]; 3],
                        3 => [r[x * 3], r[x * 3 + 1], r[x * 3 + 2]],
                        4 => {
                            let k = 255 - r[x * 4 + 3] as u32;
                            let f = |c: u8| ((255 - c as u32) * k / 255) as u8;
                            [f(r[x * 4]), f(r[x * 4 + 1]), f(r[x * 4 + 2])]
                        }
                        _ => return None,
                    }
                };
                px.put_pixel(x as u32, y as u32, image::Rgba([c[0], c[1], c[2], 255]));
            }
        }
        px
    } else {
        return None;
    };
    // where the image's unit square lands on the seen page
    let m = im.ctm;
    let o = ap(&m, 0.0, 0.0);
    let u = ap(&m, 1.0, 0.0) - o;
    let v = ap(&m, 0.0, 1.0) - o;
    let corners = [o, o + u, o + v, o + u + v];
    let lo = corners.iter().fold(DVec2::INFINITY, |a, c| a.min(*c));
    let hi = corners.iter().fold(DVec2::NEG_INFINITY, |a, c| a.max(*c));
    let det = u.perp_dot(v);
    if det.abs() < 1e-18 {
        return None;
    }
    let (sw, sh) = (src.width() as usize, src.height() as usize);
    let turned = u.x.abs() < u.y.abs();
    let (mut ow, mut oh) = if turned { (sh, sw) } else { (sw, sh) };
    let k = (MAX_PX as f64 / ow.max(oh) as f64).min(1.0);
    ow = ((ow as f64 * k).round() as usize).max(1);
    oh = ((oh as f64 * k).round() as usize).max(1);
    let mut rgba = vec![255u8; ow * oh * 4];
    for oy in 0..oh {
        for ox in 0..ow {
            // the output's row 0 is its top
            let p = DVec2::new(
                lo.x + (ox as f64 + 0.5) / ow as f64 * (hi.x - lo.x),
                hi.y - (oy as f64 + 0.5) / oh as f64 * (hi.y - lo.y),
            ) - o;
            let s = p.perp_dot(v) / det;
            let t = u.perp_dot(p) / det;
            let sx = ((s * sw as f64) as usize).min(sw - 1);
            let sy = (((1.0 - t) * sh as f64) as usize).min(sh - 1);
            let q = src.get_pixel(sx as u32, sy as u32).0;
            rgba[(oy * ow + ox) * 4..][..4].copy_from_slice(&q);
        }
    }
    let key = {
        use std::hash::{Hash, Hasher};
        let mut hs = std::collections::hash_map::DefaultHasher::new();
        stream.content.len().hash(&mut hs);
        stream.content[..stream.content.len().min(4096)].hash(&mut hs);
        hs.finish() | 1 << 63
    };
    Some(Scan {
        raster: Raster {
            width: ow,
            height: oh,
            rgba,
            key,
        },
        rect: [lo, hi],
    })
}

/// An image's colour components, and its palette when indexed.
fn colour_space(doc: &Document, cs: Option<&Object>) -> (usize, Option<Vec<u8>>) {
    let Some((_, cs)) = cs.and_then(|o| doc.dereference(o).ok()) else {
        return (1, None);
    };
    match cs {
        Object::Name(n) => match n.as_slice() {
            b"DeviceRGB" | b"CalRGB" => (3, None),
            b"DeviceCMYK" => (4, None),
            _ => (1, None),
        },
        Object::Array(v) => match v.first().and_then(name) {
            Some(b"ICCBased") => {
                let n = v
                    .get(1)
                    .and_then(|o| doc.dereference(o).ok())
                    .and_then(|(_, o)| o.as_stream().ok())
                    .and_then(|s| s.dict.get(b"N").ok())
                    .and_then(|o| num(doc, o))
                    .unwrap_or(3.0);
                (n as usize, None)
            }
            Some(b"Indexed") => {
                let (base, _) = colour_space(doc, v.get(1));
                let table =
                    v.get(3)
                        .and_then(|o| doc.dereference(o).ok())
                        .and_then(|(_, o)| match o {
                            Object::String(b, _) => Some(b.clone()),
                            Object::Stream(s) => s.get_plain_content().ok(),
                            _ => None,
                        });
                let rgb = table.map(|t| {
                    if base == 3 {
                        t
                    } else {
                        t.chunks(base.max(1))
                            .flat_map(|c| {
                                let f: Vec<f64> = c.iter().map(|b| *b as f64 / 255.0).collect();
                                let x = rgb(&f);
                                [(x >> 16) as u8, (x >> 8) as u8, x as u8]
                            })
                            .collect()
                    }
                });
                (1, rgb)
            }
            Some(b"CalRGB") => (3, None),
            _ => (1, None),
        },
        _ => (1, None),
    }
}

/// PNG predictors undone (Flate with /Predictor 10–15).
fn unpredict(data: &[u8], row: usize, bpp: usize) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(data.len());
    let mut prev = vec![0u8; row];
    for line in data.chunks(row + 1) {
        if line.len() < row + 1 {
            break;
        }
        let (kind, src) = (line[0], &line[1..]);
        let mut cur = vec![0u8; row];
        for i in 0..row {
            let a = if i >= bpp { cur[i - bpp] as i16 } else { 0 };
            let b = prev[i] as i16;
            let c = if i >= bpp { prev[i - bpp] as i16 } else { 0 };
            let x = src[i] as i16;
            cur[i] = match kind {
                0 => x,
                1 => x + a,
                2 => x + b,
                3 => x + (a + b) / 2,
                4 => {
                    let p = a + b - c;
                    let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
                    x + if pa <= pb && pa <= pc {
                        a
                    } else if pb <= pc {
                        b
                    } else {
                        c
                    }
                }
                _ => return None,
            } as u8;
        }
        out.extend_from_slice(&cur);
        prev = cur;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::{dictionary, Stream};

    /// A one-page PDF with `content`, a Helvetica font F1 and image Im1
    /// when given.
    fn pdf(content: &str, rotate: i64, image: Option<Stream>) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.new_object_id();
        let font = doc.add_object(dictionary! {
            "Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica",
            "Encoding" => "WinAnsiEncoding",
        });
        let mut xobj = Dictionary::new();
        if let Some(s) = image {
            xobj.set("Im1", doc.add_object(s));
        }
        let res = doc.add_object(dictionary! {
            "Font" => dictionary! { "F1" => font },
            "XObject" => xobj,
        });
        let content = doc.add_object(Stream::new(dictionary! {}, content.as_bytes().to_vec()));
        let page = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages_id, "Contents" => content,
            "Resources" => res, "Rotate" => rotate,
        });
        doc.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1,
                "MediaBox" => vec![0.into(), 0.into(), 842.into(), 595.into()],
            }),
        );
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
        doc.trailer.set("Root", catalog);
        let mut out = vec![];
        doc.save_to(&mut out).unwrap();
        out
    }

    #[test]
    fn lines_fills_and_texts_come_at_the_drawings_scale() {
        let b = pdf(
            "0 0 1 rg 72 72 72 36 re f 1 1 1 rg 0 0 842 595 re f \
             10 10 m 82 10 l S 2 w 100 100 m 100 172 l 172 172 l h S \
             BT /F1 10 Tf 1 0 0 1 200 300 Tm (Dve) Tj ( izby) Tj ET",
            0,
            None,
        );
        let p = read(&b, 1, 100.0).unwrap();
        assert_eq!(p.pages, 1);
        // 72 pt is an inch of paper: 2.54 m at 1:100
        let l = &p.imp.polys[0];
        assert!((l.pts[1].x - l.pts[0].x - 2.54).abs() < 1e-9);
        assert!(p.imp.polys[1].closed);
        // the white page background is left out, the blue box kept
        assert_eq!(p.imp.fills.len(), 1);
        assert_eq!(p.imp.fills[0].color, 0x0000ff);
        assert_eq!(p.imp.texts.len(), 1, "{:?}", p.imp.texts);
        assert_eq!(p.imp.texts[0].text, "Dve izby");
        assert!((p.imp.texts[0].p.x - 200.0 * 0.0254 / 72.0 * 100.0).abs() < 1e-9);
        assert_eq!(p.imp.layers, [LAYER_LINES, LAYER_FILLS, LAYER_TEXTS]);
        assert!((p.size.x - 842.0 * 0.0254 / 72.0 * 100.0).abs() < 1e-9);
        assert!(p.scan.is_none());
    }

    #[test]
    fn a_turned_page_is_read_as_it_is_seen() {
        let b = pdf("0 0 m 72 0 l S", 90, None);
        let p = read(&b, 1, 72.0 / 0.0254).unwrap();
        // turned clockwise: the page is 595 wide, the line runs down
        assert!((p.size.x - 595.0).abs() < 1e-6);
        let l = &p.imp.polys[0].pts;
        assert!((l[0] - DVec2::new(0.0, 842.0)).length() < 1e-6, "{l:?}");
        assert!((l[1] - DVec2::new(0.0, 770.0)).length() < 1e-6, "{l:?}");
    }

    #[test]
    fn a_scanned_page_comes_as_its_picture() {
        // a 2×1 grey image: dark left, light right, over the whole page
        let im = Stream::new(
            dictionary! {
                "Type" => "XObject", "Subtype" => "Image", "Width" => 2, "Height" => 1,
                "ColorSpace" => "DeviceGray", "BitsPerComponent" => 8,
            },
            vec![10, 240],
        );
        let b = pdf("q 842 0 0 595 0 0 cm /Im1 Do Q", 0, Some(im));
        let p = read(&b, 1, 1.0).unwrap();
        let s = p.scan.expect("a scan");
        assert_eq!((s.raster.width, s.raster.height), (2, 1));
        assert_eq!(s.raster.rgba[0], 10);
        assert_eq!(s.raster.rgba[4], 240);
        assert!((s.rect[1].x - 842.0 * 0.0254 / 72.0).abs() < 1e-9);
    }

    #[test]
    fn texts_are_read_through_tounicode_and_differences() {
        let m = to_unicode(
            b"1 begincodespacerange <00> <FF> endcodespacerange\n2 beginbfchar\n<01> <00FD>\n<02> <010D>\nendbfchar\n\
              1 beginbfrange\n<10> <12> <0041>\nendbfrange\n1 beginbfrange <20> <21> [<0161> <017E>] endbfrange",
        );
        assert_eq!(m[&1], "ý");
        assert_eq!(m[&2], "č");
        assert_eq!(m[&0x12], "C");
        assert_eq!(m[&0x21], "ž");
        assert_eq!(glyph("Scaron"), Some('Š'));
        assert_eq!(glyph("uni013E"), Some('ľ'));
        assert_eq!(glyph("a"), Some('a'));
        let f = Font {
            two_byte: false,
            widths: HashMap::new(),
            default_w: 500.0,
            unicode: m,
            diffs: HashMap::from([(0x9a, 'š')]),
        };
        assert_eq!(decode(&f, &[0x01, b'x', 0x9a]), "ýxš");
    }

    #[test]
    fn png_predictors_are_undone() {
        // two rows of 3 bytes: Sub then Up
        let data = [1, 5, 1, 1, 2, 1, 1, 1];
        assert_eq!(unpredict(&data, 3, 1).unwrap(), [5, 6, 7, 6, 7, 8]);
    }

    #[test]
    fn not_a_pdf_says_so() {
        assert!(read(b"hello", 1, 100.0).is_err());
    }
}
