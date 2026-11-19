//! Printing sheets on the system's printers (Windows GDI), vector, at 1:1
//! or shrunk to the paper, a page a sheet in one print job. Where a sheet
//! lands on the paper is `creabim_geom::printing`'s; this draws it there.
//!
//! Besides the system's printers there is always [`PDF`]: the same pages,
//! papers and placement written as a PDF file (and the only printer on
//! systems other than Windows).

use creabim_geom::paper::Canvas;
use creabim_geom::printing::{PaperAsk, PaperGot};
use glam::DVec2;

/// The printer that writes a PDF file.
pub const PDF: &str = "PDF";

/// The system's printers by name, and the default one.
pub fn printers() -> (Vec<String>, Option<String>) {
    #[cfg(windows)]
    {
        win::printers()
    }
    #[cfg(not(windows))]
    {
        (vec![], None)
    }
}

/// A printer's own settings (paper tray, quality, colour...), as its
/// driver keeps them: none until the user changes them in Properties.
pub type Settings = Option<Vec<u8>>;

/// The printer's Properties dialog; true when the user pressed OK, and
/// `settings` are then theirs.
pub fn properties(printer: &str, settings: &mut Settings) -> bool {
    #[cfg(windows)]
    {
        win::properties(printer, settings)
    }
    #[cfg(not(windows))]
    {
        let _ = (printer, settings);
        false
    }
}

/// The paper `printer` gives for `ask` and where it can print, as the
/// sheet sees it; none when it cannot tell (the PDF: the paper whole).
pub fn paper_got(printer: &str, settings: &Settings, ask: &PaperAsk) -> Option<PaperGot> {
    #[cfg(windows)]
    {
        if printer != PDF {
            return win::paper_got(printer, settings, ask);
        }
    }
    let _ = (printer, settings);
    ask.size.map(PaperGot::whole)
}

/// One page to print: the sheet's size and the paper it asks for.
#[derive(Clone, Copy, Debug)]
pub struct Page {
    pub sheet: DVec2,
    pub ask: PaperAsk,
}

/// A print job: its pages, `copies` of each set, on `printer`.
#[cfg_attr(not(windows), allow(dead_code))]
pub struct Job {
    pub printer: String,
    pub settings: Settings,
    pub title: String,
    pub pages: Vec<Page>,
    pub copies: u32,
    /// Ask the printer for black and white too.
    pub mono: bool,
}

/// What draws page `i` on its paper: given the paper the printer gave,
/// it draws in that paper's millimetres, y up.
pub type Draw<'a> = dyn FnMut(usize, &PaperGot, &mut dyn Canvas) + 'a;

/// Print `job` on its printer: the number of pages sent.
pub fn print(job: &Job, draw: &mut Draw) -> Result<usize, String> {
    if job.pages.is_empty() {
        return Ok(0);
    }
    #[cfg(windows)]
    {
        if job.printer != PDF {
            return win::print(job, draw);
        }
    }
    let _ = draw;
    Err("no printer".into())
}

/// `job` as a PDF: a page a sheet, each on the paper it asked for.
pub fn pdf(job: &Job, draw: &mut Draw) -> Vec<u8> {
    let mut w = creabim_geom::paper::PdfWriter::new();
    for _ in 0..job.copies.max(1) {
        for (i, p) in job.pages.iter().enumerate() {
            let got = PaperGot::whole(p.ask.size.unwrap_or(p.sheet));
            let mut page = w.page(got.size);
            draw(i, &got, &mut page);
        }
    }
    w.finish()
}

#[cfg(windows)]
mod win {
    //! The Windows printer: GDI, in device pixels.

    use super::{Draw, Job, Settings};
    use creabim_geom::annotation::TextLook;
    use creabim_geom::paper::{Anchor, Canvas, Stroke};
    use creabim_geom::printing::{PaperAsk, PaperGot};
    use creabim_geom::raster::Raster;
    use glam::DVec2;
    use std::collections::HashMap;
    use std::ptr::{null, null_mut};
    use std::sync::Arc;
    use windows_sys::Win32::Foundation::{HANDLE, POINT};
    use windows_sys::Win32::Graphics::Gdi::*;
    use windows_sys::Win32::Graphics::Printing::*;
    use windows_sys::Win32::Storage::Xps::*;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    /// A NUL-ended wide string.
    unsafe fn from_wide(p: *const u16) -> String {
        if p.is_null() {
            return String::new();
        }
        let mut n = 0;
        while unsafe { *p.add(n) } != 0 {
            n += 1;
        }
        String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(p, n) })
    }

    pub fn printers() -> (Vec<String>, Option<String>) {
        let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
        let mut names = vec![];
        unsafe {
            let (mut needed, mut count) = (0u32, 0u32);
            EnumPrintersW(flags, null(), 4, null_mut(), 0, &mut needed, &mut count);
            if needed > 0 {
                // u64s: the records are pointers, aligned
                let mut buf = vec![0u64; (needed as usize).div_ceil(8)];
                if EnumPrintersW(
                    flags,
                    null(),
                    4,
                    buf.as_mut_ptr().cast(),
                    needed,
                    &mut needed,
                    &mut count,
                ) != 0
                {
                    let infos = std::slice::from_raw_parts(
                        buf.as_ptr().cast::<PRINTER_INFO_4W>(),
                        count as usize,
                    );
                    names = infos.iter().map(|i| from_wide(i.pPrinterName)).collect();
                }
            }
        }
        let mut len = 0u32;
        let default = unsafe {
            GetDefaultPrinterW(null_mut(), &mut len);
            let mut b = vec![0u16; len as usize + 1];
            if len > 0 && GetDefaultPrinterW(b.as_mut_ptr(), &mut len) != 0 {
                Some(from_wide(b.as_ptr()))
            } else {
                None
            }
        };
        (names, default)
    }

    /// The printer's settings as a DEVMODE buffer: `settings`, or its
    /// driver's defaults.
    fn devmode(name: &[u16], settings: &Settings) -> Option<Vec<u64>> {
        unsafe {
            let mut h = PRINTER_HANDLE::default();
            if OpenPrinterW(name.as_ptr(), &mut h, null()) == 0 {
                return None;
            }
            let size = DocumentPropertiesW(null_mut(), h, name.as_ptr(), null_mut(), null(), 0);
            let out = if size > 0 {
                let mut buf = vec![0u64; (size as usize).div_ceil(8) + 1];
                let input = settings
                    .as_ref()
                    .filter(|s| s.len() >= std::mem::size_of::<DEVMODEW>())
                    .map(|s| {
                        let mut v = vec![0u64; s.len().div_ceil(8)];
                        std::ptr::copy_nonoverlapping(s.as_ptr(), v.as_mut_ptr().cast(), s.len());
                        v
                    });
                let mode = if input.is_some() {
                    DM_OUT_BUFFER | DM_IN_BUFFER
                } else {
                    DM_OUT_BUFFER
                };
                let r = DocumentPropertiesW(
                    null_mut(),
                    h,
                    name.as_ptr(),
                    buf.as_mut_ptr().cast(),
                    input
                        .as_ref()
                        .map_or(null(), |v| v.as_ptr().cast::<DEVMODEW>()),
                    mode,
                );
                (r >= 0).then_some(buf)
            } else {
                None
            };
            ClosePrinter(h);
            out
        }
    }

    fn bytes_of(dm: &[u64]) -> Vec<u8> {
        let d = unsafe { &*dm.as_ptr().cast::<DEVMODEW>() };
        let n = d.dmSize as usize + d.dmDriverExtra as usize;
        let all = unsafe { std::slice::from_raw_parts(dm.as_ptr().cast::<u8>(), dm.len() * 8) };
        all[..n.min(all.len())].to_vec()
    }

    pub fn properties(printer: &str, settings: &mut Settings) -> bool {
        let name = wide(printer);
        let Some(mut dm) = devmode(&name, settings) else {
            return false;
        };
        unsafe {
            let mut h = PRINTER_HANDLE::default();
            if OpenPrinterW(name.as_ptr(), &mut h, null()) == 0 {
                return false;
            }
            let input = dm.clone();
            let hwnd = windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow();
            let r = DocumentPropertiesW(
                hwnd,
                h,
                name.as_ptr(),
                dm.as_mut_ptr().cast(),
                input.as_ptr().cast(),
                DM_IN_PROMPT | DM_IN_BUFFER | DM_OUT_BUFFER,
            );
            ClosePrinter(h);
            // IDOK
            if r == 1 {
                *settings = Some(bytes_of(&dm));
                return true;
            }
        }
        false
    }

    /// The driver's forms: (number, upright size in mm).
    fn forms(name: &[u16]) -> Vec<(u16, DVec2)> {
        unsafe {
            let n = DeviceCapabilitiesW(name.as_ptr(), null(), DC_PAPERS, null_mut(), null());
            let m = DeviceCapabilitiesW(name.as_ptr(), null(), DC_PAPERSIZE, null_mut(), null());
            if n <= 0 || m != n {
                return vec![];
            }
            let mut ids = vec![0u16; n as usize];
            let mut sizes = vec![POINT { x: 0, y: 0 }; n as usize];
            DeviceCapabilitiesW(name.as_ptr(), null(), DC_PAPERS, ids.as_mut_ptr(), null());
            DeviceCapabilitiesW(
                name.as_ptr(),
                null(),
                DC_PAPERSIZE,
                sizes.as_mut_ptr().cast(),
                null(),
            );
            ids.into_iter()
                .zip(sizes)
                .map(|(i, s)| (i, DVec2::new(s.x as f64, s.y as f64) / 10.0))
                .collect()
        }
    }

    /// The DEVMODE for one page: the printer's settings with the paper
    /// `ask` wants, as a form of the driver's when it has one that size,
    /// else as a custom size; then as the driver takes it.
    fn page_mode(
        name: &[u16],
        base: &[u64],
        forms: &[(u16, DVec2)],
        ask: &PaperAsk,
        mono: bool,
    ) -> Vec<u64> {
        let mut dm = base.to_vec();
        {
            let d = unsafe { &mut *dm.as_mut_ptr().cast::<DEVMODEW>() };
            let f = unsafe { &mut d.Anonymous1.Anonymous1 };
            f.dmOrientation = if ask.landscape {
                DMORIENT_LANDSCAPE
            } else {
                DMORIENT_PORTRAIT
            } as i16;
            d.dmFields |= DM_ORIENTATION;
            if let Some(size) = ask.size {
                // the paper as fed: across and along, before turning
                let fed = if ask.landscape {
                    DVec2::new(size.y, size.x)
                } else {
                    size
                };
                let form = forms
                    .iter()
                    .find(|(_, s)| (s.x - fed.x).abs() < 2.0 && (s.y - fed.y).abs() < 2.0);
                d.dmFields &= !(DM_PAPERSIZE | DM_PAPERLENGTH | DM_PAPERWIDTH);
                match form {
                    Some((id, _)) => {
                        f.dmPaperSize = *id as i16;
                        d.dmFields |= DM_PAPERSIZE;
                    }
                    None => {
                        f.dmPaperSize = DMPAPER_USER as i16;
                        f.dmPaperWidth = (fed.x * 10.0).round().min(32767.0) as i16;
                        f.dmPaperLength = (fed.y * 10.0).round().min(32767.0) as i16;
                        d.dmFields |= DM_PAPERSIZE | DM_PAPERWIDTH | DM_PAPERLENGTH;
                    }
                }
            }
            f.dmCopies = 1;
            d.dmFields |= DM_COPIES;
            if mono {
                d.dmColor = DMCOLOR_MONOCHROME;
                d.dmFields |= DM_COLOR;
            }
        }
        // the driver merges and checks it
        unsafe {
            let mut h = PRINTER_HANDLE::default();
            if OpenPrinterW(name.as_ptr(), &mut h, null()) != 0 {
                let mut out = vec![0u64; dm.len()];
                let r = DocumentPropertiesW(
                    null_mut(),
                    h,
                    name.as_ptr(),
                    out.as_mut_ptr().cast(),
                    dm.as_ptr().cast(),
                    DM_IN_BUFFER | DM_OUT_BUFFER,
                );
                ClosePrinter(h);
                if r >= 0 {
                    return out;
                }
            }
        }
        dm
    }

    /// The device's paper and reach.
    struct Device {
        dpi: DVec2,
        /// The paper, pixels, and where the printable area starts on it.
        phys: DVec2,
        off: DVec2,
        printable: DVec2,
    }

    impl Device {
        fn of(dc: HDC) -> Self {
            let cap = |i: u32| unsafe { GetDeviceCaps(dc, i as _) } as f64;
            Self {
                dpi: DVec2::new(cap(LOGPIXELSX), cap(LOGPIXELSY)).max(DVec2::splat(72.0)),
                phys: DVec2::new(cap(PHYSICALWIDTH), cap(PHYSICALHEIGHT)),
                off: DVec2::new(cap(PHYSICALOFFSETX), cap(PHYSICALOFFSETY)),
                printable: DVec2::new(cap(HORZRES), cap(VERTRES)),
            }
        }

        fn got(&self) -> PaperGot {
            let mm = 25.4 / self.dpi;
            let size = self.phys * mm;
            // y up from the paper's lower edge
            let lo = DVec2::new(
                self.off.x * mm.x,
                (self.phys.y - self.off.y - self.printable.y) * mm.y,
            );
            let hi = DVec2::new(
                (self.off.x + self.printable.x) * mm.x,
                (self.phys.y - self.off.y) * mm.y,
            );
            PaperGot {
                size,
                printable_lo: lo,
                printable_hi: hi,
            }
        }
    }

    pub fn paper_got(printer: &str, settings: &Settings, ask: &PaperAsk) -> Option<PaperGot> {
        let name = wide(printer);
        let base = devmode(&name, settings)?;
        let dm = page_mode(&name, &base, &forms(&name), ask, false);
        unsafe {
            let ic = CreateICW(null(), name.as_ptr(), null(), dm.as_ptr().cast());
            if ic.is_null() {
                return None;
            }
            let got = Device::of(ic).got();
            DeleteDC(ic);
            Some(got)
        }
    }

    pub fn print(job: &Job, draw: &mut Draw) -> Result<usize, String> {
        let name = wide(&job.printer);
        let base = devmode(&name, &job.settings)
            .ok_or_else(|| format!("The printer {} does not answer.", job.printer))?;
        let forms = forms(&name);
        let modes: Vec<Vec<u64>> = job
            .pages
            .iter()
            .map(|p| page_mode(&name, &base, &forms, &p.ask, job.mono))
            .collect();
        unsafe {
            let dc = CreateDCW(null(), name.as_ptr(), null(), modes[0].as_ptr().cast());
            if dc.is_null() {
                return Err(format!("The printer {} could not be opened.", job.printer));
            }
            // the drawings' own face, for this job
            let face = creabim_geom::font::setup().font.clone();
            let mut added: HANDLE = null_mut();
            if let Some(f) = &face {
                let mut n = 0u32;
                added = AddFontMemResourceEx(
                    f.bytes.as_ptr().cast(),
                    f.bytes.len() as u32,
                    null(),
                    (&raw mut n).cast_const(),
                );
            }
            let title = wide(&job.title);
            let info = DOCINFOW {
                cbSize: std::mem::size_of::<DOCINFOW>() as i32,
                lpszDocName: title.as_ptr(),
                lpszOutput: null(),
                lpszDatatype: null(),
                fwType: 0,
            };
            let mut sent = 0;
            let mut result = Ok(());
            if StartDocW(dc, &info) <= 0 {
                result = Err("The printer refused the job.".to_string());
            } else {
                'copies: for _ in 0..job.copies.max(1) {
                    for (i, mode) in modes.iter().enumerate() {
                        if i > 0 || sent > 0 {
                            ResetDCW(dc, mode.as_ptr().cast());
                        }
                        if StartPage(dc) <= 0 {
                            result = Err("The printer stopped taking pages.".to_string());
                            break 'copies;
                        }
                        let dev = Device::of(dc);
                        let got = dev.got();
                        {
                            let mut c = Gdi::new(dc, &dev, face.as_ref().map(|f| f.family.clone()));
                            draw(i, &got, &mut c);
                        }
                        if EndPage(dc) <= 0 {
                            result = Err("The printer stopped taking pages.".to_string());
                            break 'copies;
                        }
                        sent += 1;
                    }
                }
                if result.is_ok() {
                    EndDoc(dc);
                } else {
                    AbortDoc(dc);
                }
            }
            if !added.is_null() {
                RemoveFontMemResourceEx(added);
            }
            DeleteDC(dc);
            result.map(|_| sent)
        }
    }

    fn rgb(r: f64, g: f64, b: f64) -> u32 {
        let c = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u32;
        c(r) | (c(g) << 8) | (c(b) << 16)
    }

    /// A canvas on a printer's page: paper mm, y up, to device pixels.
    struct Gdi {
        dc: HDC,
        /// pixels a millimetre
        k: DVec2,
        /// paper height, mm, and the printable area's corner, pixels
        height: f64,
        off: DVec2,
        face: Option<String>,
        fonts: HashMap<(String, bool, bool, i32), HFONT>,
    }

    impl Gdi {
        fn new(dc: HDC, dev: &Device, face: Option<String>) -> Self {
            unsafe {
                SetGraphicsMode(dc, GM_ADVANCED);
                SetBkMode(dc, TRANSPARENT as i32);
                SetPolyFillMode(dc, WINDING);
                SetStretchBltMode(dc, HALFTONE);
            }
            let k = dev.dpi / 25.4;
            Self {
                dc,
                k,
                height: dev.phys.y / k.y,
                off: dev.off,
                face,
                fonts: HashMap::new(),
            }
        }

        fn px(&self, p: DVec2) -> DVec2 {
            DVec2::new(p.x * self.k.x, (self.height - p.y) * self.k.y) - self.off
        }

        fn pts(&self, pts: &[DVec2]) -> Vec<POINT> {
            pts.iter()
                .map(|p| {
                    let q = self.px(*p);
                    POINT {
                        x: q.x.round() as i32,
                        y: q.y.round() as i32,
                    }
                })
                .collect()
        }

        fn stroke(&mut self, pts: &[DVec2], s: Stroke, color: u32) {
            if pts.len() < 2 {
                return;
            }
            let k = self.k.x;
            let width = (s.width * k).round().max(1.0) as u32;
            let brush = LOGBRUSH {
                lbStyle: BS_SOLID,
                lbColor: color,
                lbHatch: 0,
            };
            let dashes: Vec<u32> = s
                .dash
                .map(|[a, b]| {
                    vec![
                        (a * k).round().max(1.0) as u32,
                        (b * k).round().max(1.0) as u32,
                    ]
                })
                .unwrap_or_default();
            let style = PS_GEOMETRIC as u32
                | PS_ENDCAP_ROUND as u32
                | PS_JOIN_ROUND as u32
                | if dashes.is_empty() {
                    PS_SOLID as u32
                } else {
                    PS_USERSTYLE as u32
                };
            let p = self.pts(pts);
            unsafe {
                let pen = ExtCreatePen(
                    style,
                    width,
                    &brush,
                    dashes.len() as u32,
                    if dashes.is_empty() {
                        null()
                    } else {
                        dashes.as_ptr()
                    },
                );
                let old = SelectObject(self.dc, pen);
                Polyline(self.dc, p.as_ptr(), p.len() as i32);
                SelectObject(self.dc, old);
                DeleteObject(pen);
            }
        }

        fn filled(&mut self, pts: &[DVec2], color: u32) {
            if pts.len() < 3 {
                return;
            }
            let p = self.pts(pts);
            unsafe {
                let brush = CreateSolidBrush(color);
                let old_b = SelectObject(self.dc, brush);
                let old_p = SelectObject(self.dc, GetStockObject(NULL_PEN));
                Polygon(self.dc, p.as_ptr(), p.len() as i32);
                SelectObject(self.dc, old_p);
                SelectObject(self.dc, old_b);
                DeleteObject(brush);
            }
        }

        fn font(&mut self, face: &str, bold: bool, italic: bool, px: i32) -> HFONT {
            let key = (face.to_string(), bold, italic, px);
            if let Some(f) = self.fonts.get(&key) {
                return *f;
            }
            let name = wide(face);
            let f = unsafe {
                CreateFontW(
                    -px,
                    0,
                    0,
                    0,
                    if bold { FW_BOLD } else { FW_NORMAL } as i32,
                    italic as u32,
                    0,
                    0,
                    DEFAULT_CHARSET as u32,
                    OUT_TT_PRECIS as u32,
                    CLIP_DEFAULT_PRECIS as u32,
                    PROOF_QUALITY as u32,
                    0,
                    name.as_ptr(),
                )
            };
            self.fonts.insert(key, f);
            f
        }

        fn text(&mut self, at: DVec2, text: &str, anchor: Anchor, angle: f64, look: &TextLook) {
            let size = look.size;
            if text.is_empty() || size <= 0.0 {
                return;
            }
            let setup = creabim_geom::font::setup();
            let own = setup.face(look.font).cloned();
            let w = look.width(text);
            let along = match anchor {
                Anchor::Start => 0.0,
                Anchor::Middle => -w / 2.0,
                Anchor::End => -w,
            };
            let (dir, up) = (DVec2::from_angle(angle), DVec2::from_angle(angle).perp());
            // the baseline a little under the capitals' middle, as in a PDF
            let drop = match &own {
                Some(f) => size * f.cap_height / f.units / 2.0,
                None => size * 0.35,
            };
            let o = self.px(at + dir * along - up * drop);
            let face = match (&own, &self.face, look.font) {
                (Some(_), Some(name), _) => name.clone(),
                (_, _, creabim_core::Font::Serif) => "Times New Roman".into(),
                (_, _, creabim_core::Font::Mono) => "Courier New".into(),
                _ => "Arial".into(),
            };
            let px = (size * self.k.y).round().max(1.0) as i32;
            let font = self.font(&face, look.bold, look.italic, px);
            let color = look
                .rgb
                .map_or(0, |[r, g, b]| rgb(r as f64, g as f64, b as f64));
            let s = setup.stretch as f32;
            let (sin, cos) = (angle.sin() as f32, angle.cos() as f32);
            // text's own x (letters `stretch` wide) along the angle, its y
            // down the page turned with it; y down on the device
            let xf = XFORM {
                eM11: s * cos,
                eM12: -s * sin,
                eM21: sin,
                eM22: cos,
                eDx: o.x as f32,
                eDy: o.y as f32,
            };
            let t: Vec<u16> = text.encode_utf16().collect();
            unsafe {
                let old = SelectObject(self.dc, font);
                SetTextColor(self.dc, color);
                SetTextAlign(self.dc, TA_BASELINE | TA_LEFT);
                SetWorldTransform(self.dc, &xf);
                TextOutW(self.dc, 0, 0, t.as_ptr(), t.len() as i32);
                ModifyWorldTransform(self.dc, null(), MWT_IDENTITY);
                SelectObject(self.dc, old);
            }
        }
    }

    impl Drop for Gdi {
        fn drop(&mut self) {
            for f in self.fonts.values() {
                unsafe {
                    DeleteObject(*f);
                }
            }
        }
    }

    impl Canvas for Gdi {
        fn line(&mut self, pts: &[DVec2], s: Stroke) {
            self.stroke(pts, s, rgb(s.gray, s.gray, s.gray));
        }

        fn line_rgb(&mut self, pts: &[DVec2], s: Stroke, [r, g, b]: [f32; 3]) {
            self.stroke(pts, s, rgb(r as f64, g as f64, b as f64));
        }

        fn fill(&mut self, pts: &[DVec2], gray: f64) {
            self.filled(pts, rgb(gray, gray, gray));
        }

        fn fill_rgb(&mut self, pts: &[DVec2], [r, g, b]: [f32; 3]) {
            self.filled(pts, rgb(r as f64, g as f64, b as f64));
        }

        fn text_angle(&mut self, at: DVec2, size: f64, text: &str, anchor: Anchor, angle: f64) {
            let look = TextLook {
                size,
                ..TextLook::default()
            };
            self.text(at, text, anchor, angle, &look);
        }

        fn styled_text(
            &mut self,
            at: DVec2,
            text: &str,
            anchor: Anchor,
            angle: f64,
            look: &TextLook,
        ) {
            self.text(at, text, anchor, angle, look);
        }

        fn image(&mut self, lo: DVec2, hi: DVec2, clip: Option<(DVec2, DVec2)>, r: &Arc<Raster>) {
            let (w, h) = (r.width, r.height);
            if w == 0 || h == 0 {
                return;
            }
            // 24-bit rows, bottom-up order not needed: a negative height
            // reads them from the top; see-through parts on white
            let row = (w * 3).div_ceil(4) * 4;
            let mut bits = vec![255u8; row * h];
            for y in 0..h {
                for x in 0..w {
                    let s = &r.rgba[(y * w + x) * 4..(y * w + x) * 4 + 4];
                    let a = s[3] as u32;
                    let on_white = |c: u8| ((c as u32 * a + 255 * (255 - a)) / 255) as u8;
                    let d = y * row + x * 3;
                    bits[d] = on_white(s[2]);
                    bits[d + 1] = on_white(s[1]);
                    bits[d + 2] = on_white(s[0]);
                }
            }
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: w as i32,
                    biHeight: -(h as i32),
                    biPlanes: 1,
                    biBitCount: 24,
                    biCompression: BI_RGB,
                    ..Default::default()
                },
                ..Default::default()
            };
            let (a, b) = (
                self.px(DVec2::new(lo.x, hi.y)),
                self.px(DVec2::new(hi.x, lo.y)),
            );
            unsafe {
                let saved = SaveDC(self.dc);
                if let Some((c0, c1)) = clip {
                    let (p, q) = (
                        self.px(DVec2::new(c0.x, c1.y)),
                        self.px(DVec2::new(c1.x, c0.y)),
                    );
                    IntersectClipRect(
                        self.dc,
                        p.x.round() as i32,
                        p.y.round() as i32,
                        q.x.round() as i32,
                        q.y.round() as i32,
                    );
                }
                StretchDIBits(
                    self.dc,
                    a.x.round() as i32,
                    a.y.round() as i32,
                    (b.x - a.x).round() as i32,
                    (b.y - a.y).round() as i32,
                    0,
                    0,
                    w as i32,
                    h as i32,
                    bits.as_ptr().cast(),
                    &info,
                    DIB_RGB_COLORS,
                    SRCCOPY,
                );
                RestoreDC(self.dc, saved);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_geom::printing::{paper_for, PaperChoice};

    #[test]
    fn the_pdf_printer_gives_each_sheet_the_paper_it_asked_for() {
        let a1 = DVec2::new(841.0, 594.0);
        let job = Job {
            printer: PDF.into(),
            settings: None,
            title: "t".into(),
            pages: vec![
                Page {
                    sheet: a1,
                    ask: paper_for(a1, PaperChoice::Iso(1)),
                },
                Page {
                    sheet: a1,
                    ask: paper_for(a1, PaperChoice::Printer),
                },
            ],
            copies: 2,
            mono: false,
        };
        let mut seen = vec![];
        let pdf = pdf(&job, &mut |i, got, _c| seen.push((i, got.size)));
        assert_eq!(
            seen,
            vec![
                (0, DVec2::new(420.0, 297.0)),
                (1, a1),
                (0, DVec2::new(420.0, 297.0)),
                (1, a1)
            ]
        );
        assert!(String::from_utf8_lossy(&pdf).contains("/Count 4"));
    }
}
