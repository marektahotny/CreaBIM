//! A drawing written as DXF: a view at its true size in millimetres, or
//! a sheet in paper millimetres, in the version asked for. Lines keep
//! their colours, widths (as lineweights) and dashes (as linetypes),
//! fills are solids, texts are texts, and what is drawn goes to a layer
//! by what it is (walls, doors, pipes...).
//!
//! R12 is the plainest and reads everywhere; the later ones add handles,
//! lineweights, light polylines and (from 2004) true colours, and write
//! texts in Unicode.

use crate::annotation::TextLook;
use crate::looks::Looks;
use crate::paper::{Anchor, Canvas, PaperMap, Stroke};
use crate::raster::Raster;
use crate::PlanView;
use creabim_core::Model;
use glam::DVec2;
use std::fmt::Write;
use std::sync::Arc;

/// The DXF versions a drawing can be written as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DxfVersion {
    R12,
    R2000,
    R2004,
    R2007,
    R2010,
    R2013,
    R2018,
}

impl DxfVersion {
    pub const ALL: [DxfVersion; 7] = [
        DxfVersion::R12,
        DxfVersion::R2000,
        DxfVersion::R2004,
        DxfVersion::R2007,
        DxfVersion::R2010,
        DxfVersion::R2013,
        DxfVersion::R2018,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DxfVersion::R12 => "AutoCAD R12",
            DxfVersion::R2000 => "AutoCAD 2000",
            DxfVersion::R2004 => "AutoCAD 2004",
            DxfVersion::R2007 => "AutoCAD 2007",
            DxfVersion::R2010 => "AutoCAD 2010",
            DxfVersion::R2013 => "AutoCAD 2013",
            DxfVersion::R2018 => "AutoCAD 2018",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            DxfVersion::R12 => "r12",
            DxfVersion::R2000 => "2000",
            DxfVersion::R2004 => "2004",
            DxfVersion::R2007 => "2007",
            DxfVersion::R2010 => "2010",
            DxfVersion::R2013 => "2013",
            DxfVersion::R2018 => "2018",
        }
    }

    pub fn from_id(s: &str) -> Option<DxfVersion> {
        DxfVersion::ALL.into_iter().find(|v| v.id() == s)
    }

    /// What `$ACADVER` says.
    fn code(self) -> &'static str {
        match self {
            DxfVersion::R12 => "AC1009",
            DxfVersion::R2000 => "AC1015",
            DxfVersion::R2004 => "AC1018",
            DxfVersion::R2007 => "AC1021",
            DxfVersion::R2010 => "AC1024",
            DxfVersion::R2013 => "AC1027",
            DxfVersion::R2018 => "AC1032",
        }
    }
}

/// The layer of what belongs to no element: frames, titles, notes.
pub const SHEET_LAYER: &str = "Harok";

/// The text style every text is written in: a narrow face, as the
/// drawings' own.
const STYLE: &str = "CREABIM";

/// A [`Canvas`] that writes DXF entities. `factor` takes the canvas's
/// paper millimetres to the file's units: a view's scale for a drawing at
/// true size, 1 for a sheet.
pub struct DxfCanvas {
    version: DxfVersion,
    factor: f64,
    layer: String,
    body: String,
    layers: Vec<String>,
    /// The dashes met: name, dash and gap in the file's units.
    ltypes: Vec<(String, [f64; 2])>,
    handle: u32,
    lo: DVec2,
    hi: DVec2,
}

/// The handles the fixed parts of a file take; entities count on from
/// [`FIRST`].
const MODEL_RECORD: u32 = 0x1F;
const PAPER_RECORD: u32 = 0x1B;
const FIRST: u32 = 0x100;

impl DxfCanvas {
    pub fn new(version: DxfVersion, factor: f64) -> Self {
        Self {
            version,
            factor,
            layer: SHEET_LAYER.into(),
            body: String::new(),
            layers: vec![],
            ltypes: vec![],
            handle: FIRST,
            lo: DVec2::splat(f64::MAX),
            hi: DVec2::splat(f64::MIN),
        }
    }

    /// What is drawn from now on goes to `layer`.
    pub fn set_layer(&mut self, layer: &str) {
        self.layer = layer_name(layer, self.version);
    }

    fn r12(&self) -> bool {
        self.version == DxfVersion::R12
    }

    fn at(&mut self, p: DVec2) -> DVec2 {
        let q = p * self.factor;
        self.lo = self.lo.min(q);
        self.hi = self.hi.max(q);
        q
    }

    /// An entity's opening groups: its kind, handle, layer, and for the
    /// later versions its subclass.
    fn open(&mut self, kind: &str, class: &str) {
        if !self.layers.contains(&self.layer) {
            self.layers.push(self.layer.clone());
        }
        let _ = write!(self.body, "0\n{kind}\n");
        if !self.r12() {
            let h = self.handle;
            self.handle += 1;
            let _ = write!(
                self.body,
                "5\n{h:X}\n330\n{MODEL_RECORD:X}\n100\nAcDbEntity\n"
            );
        }
        let _ = write!(self.body, "8\n{}\n", self.layer);
        let _ = class;
    }

    fn class(&mut self, class: &str) {
        if !self.r12() {
            let _ = write!(self.body, "100\n{class}\n");
        }
    }

    /// Colour, lineweight and linetype of a line.
    fn pen(&mut self, s: Option<Stroke>, rgb: [f32; 3]) {
        if let Some([a, b]) = s.and_then(|s| s.dash) {
            let (a, b) = (a * self.factor, b * self.factor);
            let name = format!("DASH_{}_{}", (a * 100.0).round(), (b * 100.0).round());
            if !self.ltypes.iter().any(|l| l.0 == name) {
                self.ltypes.push((name.clone(), [a, b]));
            }
            let _ = write!(self.body, "6\n{name}\n");
        }
        let _ = write!(self.body, "62\n{}\n", aci(rgb));
        if self.r12() {
            return;
        }
        if let Some(s) = s {
            let _ = write!(self.body, "370\n{}\n", lineweight(s.width));
        }
        let c = rgb.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u32);
        // black and white follow the screen's ground, as colour 7 does
        let plain = c.iter().all(|v| *v < 40) || c.iter().all(|v| *v > 235);
        if self.version >= DxfVersion::R2004 && !plain {
            let _ = write!(self.body, "420\n{}\n", c[0] << 16 | c[1] << 8 | c[2]);
        }
    }

    fn polyline(&mut self, pts: &[DVec2], s: Stroke, rgb: [f32; 3]) {
        let mut pts: Vec<DVec2> = pts.iter().map(|p| self.at(*p)).collect();
        pts.dedup_by(|a, b| a.distance(*b) < 1e-9);
        if pts.len() < 2 {
            return;
        }
        let closed = pts.len() > 3 && pts[0].distance(pts[pts.len() - 1]) < 1e-9;
        if closed {
            pts.pop();
        }
        if pts.len() == 2 {
            self.open("LINE", "AcDbLine");
            self.pen(Some(s), rgb);
            self.class("AcDbLine");
            let _ = write!(
                self.body,
                "10\n{}\n20\n{}\n30\n0.0\n11\n{}\n21\n{}\n31\n0.0\n",
                n(pts[0].x),
                n(pts[0].y),
                n(pts[1].x),
                n(pts[1].y)
            );
            return;
        }
        if self.r12() {
            self.open("POLYLINE", "");
            self.pen(Some(s), rgb);
            let _ = write!(
                self.body,
                "66\n1\n10\n0.0\n20\n0.0\n30\n0.0\n70\n{}\n",
                closed as u8
            );
            let layer = self.layer.clone();
            for p in &pts {
                let _ = write!(
                    self.body,
                    "0\nVERTEX\n8\n{layer}\n10\n{}\n20\n{}\n30\n0.0\n",
                    n(p.x),
                    n(p.y)
                );
            }
            let _ = write!(self.body, "0\nSEQEND\n8\n{layer}\n");
            return;
        }
        self.open("LWPOLYLINE", "AcDbPolyline");
        self.pen(Some(s), rgb);
        self.class("AcDbPolyline");
        let _ = write!(self.body, "90\n{}\n70\n{}\n", pts.len(), closed as u8);
        for p in &pts {
            let _ = write!(self.body, "10\n{}\n20\n{}\n", n(p.x), n(p.y));
        }
    }

    fn solid(&mut self, pts: &[DVec2], rgb: [f32; 3]) {
        for [a, b, c] in crate::triangulate::triangulate(pts) {
            let q = [pts[a], pts[b], pts[c], pts[c]].map(|p| self.at(p));
            self.open("SOLID", "AcDbTrace");
            self.pen(None, rgb);
            self.class("AcDbTrace");
            for (i, p) in q.iter().enumerate() {
                let _ = write!(
                    self.body,
                    "{}\n{}\n{}\n{}\n{}\n0.0\n",
                    10 + i,
                    n(p.x),
                    20 + i,
                    n(p.y),
                    30 + i
                );
            }
        }
    }

    fn write_text(
        &mut self,
        at: DVec2,
        size: f64,
        text: &str,
        anchor: Anchor,
        angle: f64,
        rgb: [f32; 3],
    ) {
        // a font's size to its capitals' height, which is a DXF text's
        let h = size * 0.7 * self.factor;
        let lines: Vec<&str> = text.split('\n').collect();
        let down = DVec2::from_angle(angle).perp() * -(size * 1.25);
        let first = at - down * (lines.len() as f64 - 1.0) / 2.0;
        for (i, line) in lines.iter().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            let p = self.at(first + down * i as f64);
            self.open("TEXT", "AcDbText");
            self.pen(None, rgb);
            self.class("AcDbText");
            let align = match anchor {
                Anchor::Start => 0,
                Anchor::Middle => 1,
                Anchor::End => 2,
            };
            let _ = write!(
                self.body,
                "10\n{x}\n20\n{y}\n30\n0.0\n40\n{}\n1\n{}\n50\n{}\n7\n{STYLE}\n72\n{align}\n11\n{x}\n21\n{y}\n31\n0.0\n",
                n(h),
                encode(line, self.version),
                n(angle.to_degrees()),
                x = n(p.x),
                y = n(p.y),
            );
            self.class("AcDbText");
            // middle of the capitals on the point
            let _ = write!(self.body, "73\n2\n");
        }
    }

    /// The whole file.
    pub fn finish(self) -> String {
        let r12 = self.r12();
        let (lo, hi) = if self.lo.x > self.hi.x {
            (DVec2::ZERO, DVec2::ONE)
        } else {
            (self.lo, self.hi)
        };
        let mut s = String::new();
        let mut w = |code: i32, value: &str| {
            let _ = write!(s, "{code}\n{value}\n");
        };
        w(0, "SECTION");
        w(2, "HEADER");
        w(9, "$ACADVER");
        w(1, self.version.code());
        if !r12 {
            w(9, "$DWGCODEPAGE");
            w(3, "ANSI_1250");
            w(9, "$HANDSEED");
            w(5, &format!("{:X}", self.handle + 0x100));
            w(9, "$INSUNITS");
            w(70, "4");
            w(9, "$MEASUREMENT");
            w(70, "1");
            w(9, "$LWDISPLAY");
            w(290, "1");
        }
        for (name, p) in [("$EXTMIN", lo), ("$EXTMAX", hi)] {
            w(9, name);
            w(10, &n(p.x));
            w(20, &n(p.y));
            w(30, "0.0");
        }
        w(9, "$LTSCALE");
        w(40, "1.0");
        w(0, "ENDSEC");
        if !r12 {
            w(0, "SECTION");
            w(2, "CLASSES");
            w(0, "ENDSEC");
        }
        w(0, "SECTION");
        w(2, "TABLES");
        // a table's head, and an entry's, as the version wants them
        let mut next = 0x30u32;
        let head = |s: &mut String, name: &str, handle: u32, count: usize| {
            let _ = write!(s, "0\nTABLE\n2\n{name}\n");
            if !r12 {
                let _ = write!(s, "5\n{handle:X}\n330\n0\n100\nAcDbSymbolTable\n");
            }
            let _ = write!(s, "70\n{count}\n");
        };
        let mut entry =
            |s: &mut String, kind: &str, owner: u32, class: &str, fixed: Option<u32>| {
                let _ = write!(s, "0\n{kind}\n");
                if !r12 {
                    let h = fixed.unwrap_or_else(|| {
                        next += 1;
                        next
                    });
                    let code = if kind == "DIMSTYLE" { 105 } else { 5 };
                    let _ = write!(
                        s,
                        "{code}\n{h:X}\n330\n{owner:X}\n100\nAcDbSymbolTableRecord\n100\n{class}\n"
                    );
                }
            };
        if !r12 {
            head(&mut s, "VPORT", 0x8, 1);
            entry(&mut s, "VPORT", 0x8, "AcDbViewportTableRecord", None);
            let c = (lo + hi) / 2.0;
            let size = (hi - lo).max(DVec2::ONE);
            let _ = write!(
                s,
                "2\n*Active\n70\n0\n10\n0.0\n20\n0.0\n11\n1.0\n21\n1.0\n12\n{}\n22\n{}\n13\n0.0\n23\n0.0\n14\n10.0\n24\n10.0\n15\n10.0\n25\n10.0\n16\n0.0\n26\n0.0\n36\n1.0\n17\n0.0\n27\n0.0\n37\n0.0\n40\n{}\n41\n{}\n42\n50.0\n43\n0.0\n44\n0.0\n50\n0.0\n51\n0.0\n71\n0\n72\n100\n73\n1\n74\n3\n75\n0\n76\n0\n77\n0\n78\n0\n",
                n(c.x),
                n(c.y),
                n(size.y * 1.1),
                n(size.x / size.y)
            );
            s += "0\nENDTAB\n";
        }
        head(&mut s, "LTYPE", 0x5, 3 + self.ltypes.len());
        if !r12 {
            for name in ["ByBlock", "ByLayer"] {
                entry(&mut s, "LTYPE", 0x5, "AcDbLinetypeTableRecord", None);
                let _ = write!(s, "2\n{name}\n70\n0\n3\n\n72\n65\n73\n0\n40\n0.0\n");
            }
        }
        entry(&mut s, "LTYPE", 0x5, "AcDbLinetypeTableRecord", None);
        s += "2\nCONTINUOUS\n70\n0\n3\nSolid line\n72\n65\n73\n0\n40\n0.0\n";
        for (name, [a, b]) in &self.ltypes {
            entry(&mut s, "LTYPE", 0x5, "AcDbLinetypeTableRecord", None);
            let _ = write!(
                s,
                "2\n{name}\n70\n0\n3\nDashed\n72\n65\n73\n2\n40\n{}\n49\n{}\n",
                n(a + b),
                n(*a)
            );
            if !r12 {
                s += "74\n0\n";
            }
            let _ = write!(s, "49\n{}\n", n(-*b));
            if !r12 {
                s += "74\n0\n";
            }
        }
        s += "0\nENDTAB\n";
        head(&mut s, "LAYER", 0x2, 1 + self.layers.len());
        for name in std::iter::once("0").chain(self.layers.iter().map(|l| l.as_str())) {
            entry(&mut s, "LAYER", 0x2, "AcDbLayerTableRecord", None);
            let _ = write!(s, "2\n{name}\n70\n0\n62\n7\n6\nCONTINUOUS\n");
        }
        s += "0\nENDTAB\n";
        head(&mut s, "STYLE", 0x3, 2);
        for (name, font) in [("STANDARD", "arial.ttf"), (STYLE, "arialn.ttf")] {
            entry(&mut s, "STYLE", 0x3, "AcDbTextStyleTableRecord", None);
            let font = if r12 { "txt" } else { font };
            let _ = write!(
                s,
                "2\n{name}\n70\n0\n40\n0.0\n41\n1.0\n50\n0.0\n71\n0\n42\n2.5\n3\n{font}\n4\n\n"
            );
        }
        s += "0\nENDTAB\n";
        if !r12 {
            head(&mut s, "VIEW", 0x6, 0);
            s += "0\nENDTAB\n";
            head(&mut s, "UCS", 0x7, 0);
            s += "0\nENDTAB\n";
            head(&mut s, "APPID", 0x9, 1);
            entry(&mut s, "APPID", 0x9, "AcDbRegAppTableRecord", None);
            s += "2\nACAD\n70\n0\n";
            s += "0\nENDTAB\n";
            let _ = write!(
                s,
                "0\nTABLE\n2\nDIMSTYLE\n5\nA\n330\n0\n100\nAcDbSymbolTable\n70\n1\n100\nAcDbDimStyleTable\n"
            );
            entry(&mut s, "DIMSTYLE", 0xA, "AcDbDimStyleTableRecord", None);
            s += "2\nSTANDARD\n70\n0\n";
            s += "0\nENDTAB\n";
            head(&mut s, "BLOCK_RECORD", 0x1, 2);
            for (name, h) in [
                ("*Model_Space", MODEL_RECORD),
                ("*Paper_Space", PAPER_RECORD),
            ] {
                entry(&mut s, "BLOCK_RECORD", 0x1, "AcDbBlockTableRecord", Some(h));
                let _ = write!(s, "2\n{name}\n");
            }
            s += "0\nENDTAB\n";
        }
        s += "0\nENDSEC\n0\nSECTION\n2\nBLOCKS\n";
        if !r12 {
            for (name, owner, a, b, paper) in [
                ("*Model_Space", MODEL_RECORD, 0x20, 0x21, 0),
                ("*Paper_Space", PAPER_RECORD, 0x1C, 0x1D, 1),
            ] {
                let space = if paper == 1 { "67\n1\n" } else { "" };
                let _ = write!(
                    s,
                    "0\nBLOCK\n5\n{a:X}\n330\n{owner:X}\n100\nAcDbEntity\n{space}8\n0\n100\nAcDbBlockBegin\n2\n{name}\n70\n0\n10\n0.0\n20\n0.0\n30\n0.0\n3\n{name}\n1\n\n0\nENDBLK\n5\n{b:X}\n330\n{owner:X}\n100\nAcDbEntity\n{space}8\n0\n100\nAcDbBlockEnd\n"
                );
            }
        }
        s += "0\nENDSEC\n0\nSECTION\n2\nENTITIES\n";
        s += &self.body;
        s += "0\nENDSEC\n";
        if !r12 {
            s += "0\nSECTION\n2\nOBJECTS\n0\nDICTIONARY\n5\nC\n330\n0\n100\nAcDbDictionary\n281\n1\n3\nACAD_GROUP\n350\nD\n0\nDICTIONARY\n5\nD\n330\nC\n100\nAcDbDictionary\n281\n1\n0\nENDSEC\n";
        }
        s += "0\nEOF\n";
        s
    }
}

impl Canvas for DxfCanvas {
    fn line(&mut self, pts: &[DVec2], stroke: Stroke) {
        let g = stroke.gray as f32;
        self.polyline(pts, stroke, [g; 3]);
    }

    fn line_rgb(&mut self, pts: &[DVec2], stroke: Stroke, rgb: [f32; 3]) {
        self.polyline(pts, stroke, rgb);
    }

    fn fill(&mut self, pts: &[DVec2], gray: f64) {
        // white is the paper: nothing to fill
        if gray < 0.995 {
            self.solid(pts, [gray as f32; 3]);
        }
    }

    fn fill_rgb(&mut self, pts: &[DVec2], rgb: [f32; 3]) {
        self.solid(pts, rgb);
    }

    fn text_angle(&mut self, at: DVec2, size: f64, text: &str, anchor: Anchor, angle: f64) {
        self.write_text(at, size, text, anchor, angle, [0.0; 3]);
    }

    fn styled_text(&mut self, at: DVec2, text: &str, anchor: Anchor, angle: f64, look: &TextLook) {
        self.write_text(
            at,
            look.size,
            text,
            anchor,
            angle,
            look.rgb.unwrap_or([0.0; 3]),
        );
    }

    /// Pictures stay out: a DXF carries only a path to one.
    fn image(&mut self, _: DVec2, _: DVec2, _: Option<(DVec2, DVec2)>, _: &Arc<Raster>) {}
}

/// A number as DXF writes it.
fn n(v: f64) -> String {
    let v = if v.is_finite() { v } else { 0.0 };
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0');
    if s.ends_with('.') {
        format!("{s}0")
    } else {
        s.to_string()
    }
}

/// The lineweight (hundredths of a millimetre) DXF knows nearest `mm`.
fn lineweight(mm: f64) -> i32 {
    const KNOWN: [i32; 24] = [
        0, 5, 9, 13, 15, 18, 20, 25, 30, 35, 40, 50, 53, 60, 70, 80, 90, 100, 106, 120, 140, 158,
        200, 211,
    ];
    let want = (mm * 100.0).round() as i32;
    *KNOWN
        .iter()
        .min_by_key(|k| (*k - want).abs())
        .unwrap_or(&25)
}

/// The colour index nearest `rgb`: 7 for black and white (it turns with
/// the screen's ground), the greys, else the nearest of the basic hues.
fn aci(rgb: [f32; 3]) -> i32 {
    let [r, g, b] = rgb.map(|v| v.clamp(0.0, 1.0));
    let (lo, hi) = (r.min(g).min(b), r.max(g).max(b));
    if hi - lo < 0.12 {
        return match hi {
            v if v < 0.3 => 7,
            v if v < 0.6 => 8,
            v if v < 0.92 => 9,
            _ => 7,
        };
    }
    const HUES: [(i32, [f32; 3]); 8] = [
        (1, [1.0, 0.0, 0.0]),
        (2, [1.0, 1.0, 0.0]),
        (3, [0.0, 1.0, 0.0]),
        (4, [0.0, 1.0, 1.0]),
        (5, [0.0, 0.0, 1.0]),
        (6, [1.0, 0.0, 1.0]),
        (30, [1.0, 0.5, 0.0]),
        (94, [0.0, 0.5, 0.0]),
    ];
    HUES.iter()
        .min_by(|x, y| {
            let d = |c: &[f32; 3]| (c[0] - r).powi(2) + (c[1] - g).powi(2) + (c[2] - b).powi(2);
            d(&x.1).total_cmp(&d(&y.1))
        })
        .map_or(7, |h| h.0)
}

/// A letter without its accent, for what must stay ASCII.
fn plain(c: char) -> char {
    const FROM: &str = "áäčďéěíĺľňóôöŕřšťúůüýžÁÄČĎÉĚÍĹĽŇÓÔÖŔŘŠŤÚŮÜÝŽ";
    const TO: &str = "aacdeeillnooorrstuuuyzAACDEEILLNOOORRSTUUUYZ";
    match FROM.chars().position(|f| f == c) {
        Some(i) => TO.chars().nth(i).unwrap_or('_'),
        None if c.is_ascii() => c,
        None => '_',
    }
}

/// A layer's name as the version takes it: no spaces or signs, and for
/// R12 capitals without accents, 31 letters at most.
pub fn layer_name(name: &str, version: DxfVersion) -> String {
    let s: String = name
        .chars()
        .map(|c| match c {
            ' ' | '/' | '\\' | ':' | ';' | '<' | '>' | '?' | '"' | '*' | '|' | ',' | '=' | '`'
            | '.' => '_',
            c => plain(c),
        })
        .collect();
    let s = if s.is_empty() { "0".to_string() } else { s };
    if version == DxfVersion::R12 {
        s.to_uppercase().chars().take(31).collect()
    } else {
        s
    }
}

/// A text as the version reads it: Unicode from 2007 on, `\U+XXXX` for
/// what is not ASCII before, and without accents in R12, which knows
/// neither.
fn encode(text: &str, version: DxfVersion) -> String {
    let text = text.replace(['\r', '\n'], " ");
    if version >= DxfVersion::R2007 {
        return text;
    }
    let mut out = String::new();
    for c in text.chars() {
        if c.is_ascii() {
            out.push(c);
        } else if version == DxfVersion::R12 {
            out.push(match c {
                '²' => '2',
                '³' => '3',
                '±' => '+',
                c => plain(c),
            });
        } else {
            let _ = write!(out, "\\U+{:04X}", c as u32);
        }
    }
    out
}

/// The layer of an element in a drawing: its category, in the
/// application's language.
pub fn layer_of(
    model: &Model,
    id: creabim_core::ElementId,
    cutters: &std::collections::HashSet<creabim_core::ElementId>,
) -> String {
    // a utility network on a layer of its kind
    if let Some(n) = crate::netlines::net_of(model, id) {
        return n.layer();
    }
    model
        .get(id)
        .and_then(|e| creabim_core::visibility::Category::of(e, cutters))
        .map_or(SHEET_LAYER.to_string(), |c| {
            creabim_core::words::tr(c.label())
        })
}

/// Draws `drawing` on the canvas, each element's lines on the layer of
/// what it is.
pub fn draw_layered(
    c: &mut DxfCanvas,
    model: &Model,
    drawing: &PlanView,
    map: &PaperMap,
    looks: &Looks,
) {
    let cutters = creabim_core::visibility::cutters(model);
    let mut groups: Vec<(String, PlanView)> = vec![];
    for it in &drawing.items {
        let layer = layer_of(model, it.id(), &cutters);
        match groups.iter_mut().find(|g| g.0 == layer) {
            Some(g) => g.1.items.push(it.clone()),
            None => groups.push((
                layer,
                PlanView {
                    items: vec![it.clone()],
                },
            )),
        }
    }
    for (layer, part) in &groups {
        c.set_layer(layer);
        crate::paper::draw_looks(c, part, map, looks);
    }
    c.set_layer(SHEET_LAYER);
}

/// A view's drawing as DXF, at its true size in millimetres.
pub fn view_dxf(
    model: &Model,
    drawing: &PlanView,
    looks: &Looks,
    scale: f64,
    version: DxfVersion,
) -> String {
    let map = PaperMap {
        model: DVec2::ZERO,
        paper: DVec2::ZERO,
        scale,
    };
    let mut c = DxfCanvas::new(version, scale);
    draw_layered(&mut c, model, drawing, &map, looks);
    c.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canvas(v: DxfVersion) -> String {
        let mut c = DxfCanvas::new(v, 50.0);
        c.set_layer("Steny a priečky");
        c.line(
            &[DVec2::ZERO, DVec2::new(10.0, 0.0)],
            Stroke::solid(0.5, 0.0),
        );
        c.line_rgb(
            &[DVec2::ZERO, DVec2::new(10.0, 0.0), DVec2::new(10.0, 5.0)],
            Stroke {
                width: 0.25,
                gray: 0.0,
                dash: Some([2.0, 1.0]),
            },
            [0.18, 0.44, 0.87],
        );
        c.fill_rgb(
            &[DVec2::ZERO, DVec2::new(4.0, 0.0), DVec2::new(4.0, 4.0)],
            [1.0, 0.0, 0.0],
        );
        c.text(
            DVec2::new(1.0, 1.0),
            2.5,
            "Obývačka 20,5 m²",
            Anchor::Middle,
        );
        c.finish()
    }

    /// The groups of a DXF text: (code, value).
    fn groups(s: &str) -> Vec<(i32, String)> {
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(lines.len() % 2, 0, "codes and values in pairs");
        lines
            .chunks(2)
            .map(|c| (c[0].trim().parse().expect("a group code"), c[1].to_string()))
            .collect()
    }

    #[test]
    fn every_version_is_whole_and_at_true_size() {
        for v in DxfVersion::ALL {
            let s = canvas(v);
            let g = groups(&s);
            assert_eq!(g.last(), Some(&(0, "EOF".to_string())));
            assert!(g.contains(&(1, v.code().to_string())));
            // sections open and close in pairs
            let open = g.iter().filter(|x| **x == (0, "SECTION".into())).count();
            let close = g.iter().filter(|x| **x == (0, "ENDSEC".into())).count();
            assert_eq!(open, close);
            assert_eq!(open, if v == DxfVersion::R12 { 4 } else { 6 });
            // 10 mm on paper at 1:50 is 500 mm
            assert!(g.contains(&(11, "500.0".into())));
            // the dashes are a linetype of their own, in true lengths
            assert!(g.contains(&(2, "DASH_10000_5000".into())));
            assert!(g.contains(&(6, "DASH_10000_5000".into())));
        }
    }

    #[test]
    fn handles_are_unique_and_owned() {
        let s = canvas(DxfVersion::R2010);
        let g = groups(&s);
        let mut handles: Vec<&String> = g
            .iter()
            .filter(|(c, _)| *c == 5 || *c == 105)
            .map(|(_, v)| v)
            .collect();
        // $HANDSEED is written under 5 too: it is above them all
        let seed = u32::from_str_radix(handles.remove(0), 16).unwrap();
        let n = handles.len();
        assert!(handles
            .iter()
            .all(|h| u32::from_str_radix(h, 16).unwrap() < seed));
        handles.sort();
        handles.dedup();
        assert_eq!(handles.len(), n);
        assert!(!canvas(DxfVersion::R12).contains("AcDbEntity"));
    }

    #[test]
    fn texts_and_layers_read_in_every_version() {
        let r12 = canvas(DxfVersion::R12);
        assert!(r12.contains("STENY_A_PRIECKY"));
        assert!(r12.contains("Obyvacka 20,5 m2"));
        let r2000 = canvas(DxfVersion::R2000);
        assert!(r2000.contains("Steny_a_priecky"));
        assert!(r2000.contains("Ob\\U+00FDva\\U+010Dka 20,5 m\\U+00B2"));
        assert!(r2000.is_ascii());
        // true colour comes with 2004
        assert!(!r2000.contains("\n420\n"));
        let r2018 = canvas(DxfVersion::R2018);
        assert!(r2018.contains("Obývačka 20,5 m²"));
        assert!(r2018.contains("\n420\n"));
        assert!(r2018.contains("\n370\n50\n"));
    }
}
