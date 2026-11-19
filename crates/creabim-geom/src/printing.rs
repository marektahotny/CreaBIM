//! Printing sheets: the papers a printer takes, where a sheet lands on
//! one, and a [`Canvas`] that carries a sheet's drawing onto the paper.
//!
//! The printer itself (Windows GDI) is the app's; everything here is the
//! same on every system, so it is tested here and a PDF "printer" can use
//! it as well.

use crate::annotation::TextLook;
use crate::paper::{Anchor, Canvas, Stroke};
use crate::raster::Raster;
use glam::DVec2;
use std::sync::Arc;

/// A paper size, mm, upright (width ≤ height).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Paper {
    pub name: &'static str,
    pub width: f64,
    pub height: f64,
}

/// The ISO A papers, A4 to A0.
pub const ISO_A: [Paper; 5] = [
    Paper {
        name: "A4",
        width: 210.0,
        height: 297.0,
    },
    Paper {
        name: "A3",
        width: 297.0,
        height: 420.0,
    },
    Paper {
        name: "A2",
        width: 420.0,
        height: 594.0,
    },
    Paper {
        name: "A1",
        width: 594.0,
        height: 841.0,
    },
    Paper {
        name: "A0",
        width: 841.0,
        height: 1189.0,
    },
];

/// The usual roll widths of plotters, mm: A3, A2, A1 and A0 rolls and the
/// 24", 36" and 42" ones.
pub const ROLLS: [f64; 7] = [297.0, 420.0, 594.0, 610.0, 841.0, 914.0, 1067.0];

/// What a sheet is printed on.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum PaperChoice {
    /// Exactly the sheet's own size (A3, 3×A4, ...): the printer's form
    /// of that size, or a custom size where it has none.
    #[default]
    Sheet,
    /// Whatever paper the printer is set to (its Properties).
    Printer,
    /// One of [`ISO_A`].
    Iso(usize),
    /// A roll so wide, mm; the paper is as long as the sheet needs.
    Roll(f64),
}

impl PaperChoice {
    /// Every choice in the order a list shows them.
    pub fn all() -> Vec<PaperChoice> {
        let mut v = vec![PaperChoice::Sheet, PaperChoice::Printer];
        v.extend((0..ISO_A.len()).map(PaperChoice::Iso));
        v.extend(ROLLS.iter().map(|w| PaperChoice::Roll(*w)));
        v
    }

    /// Its name in a list (English; the app translates it).
    pub fn label(&self) -> String {
        match self {
            PaperChoice::Sheet => "As the sheet (its own size)".into(),
            PaperChoice::Printer => "As set in the printer".into(),
            PaperChoice::Iso(i) => {
                let p = ISO_A[(*i).min(ISO_A.len() - 1)];
                format!("{} ({:.0} × {:.0} mm)", p.name, p.width, p.height)
            }
            PaperChoice::Roll(w) => format!("Roll {w:.0} mm"),
        }
    }
}

/// How large the sheet prints.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Fit {
    /// 1:1, true to scale; what does not fit on the paper is cut off.
    #[default]
    Actual,
    /// Made smaller to fit the paper's printable area (never larger);
    /// the drawing's scale is then no longer true.
    Shrink,
}

/// The paper a sheet asks the printer for, as the sheet sees it: `size`
/// across and up the drawing. `landscape` when the printer is to turn it
/// (wider than high); `turned` when the drawing itself is turned a
/// quarter round on an upright paper instead (a roll fed by its length).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaperAsk {
    /// None: the printer's own setting.
    pub size: Option<DVec2>,
    pub landscape: bool,
    pub turned: bool,
    pub name: &'static str,
}

/// The paper for a sheet of `sheet` mm.
pub fn paper_for(sheet: DVec2, choice: PaperChoice) -> PaperAsk {
    let wide = sheet.x > sheet.y + 0.01;
    match choice {
        PaperChoice::Sheet => PaperAsk {
            size: Some(sheet),
            landscape: wide,
            turned: false,
            name: iso_name(sheet).unwrap_or(""),
        },
        PaperChoice::Printer => PaperAsk {
            size: None,
            landscape: wide,
            turned: false,
            name: "",
        },
        PaperChoice::Iso(i) => {
            let p = ISO_A[i.min(ISO_A.len() - 1)];
            PaperAsk {
                size: Some(if wide {
                    DVec2::new(p.height, p.width)
                } else {
                    DVec2::new(p.width, p.height)
                }),
                landscape: wide,
                turned: false,
                name: p.name,
            }
        }
        PaperChoice::Roll(across) => {
            // the roll's width across the paper, upright: the sheet's
            // longer side across when it fits (the least paper), else its
            // shorter side across and the longer along the roll
            let (short, long) = (sheet.x.min(sheet.y), sheet.x.max(sheet.y));
            let long_across = long <= across + 0.01;
            let length = if long_across { short } else { long };
            // the drawing's x along the roll's width when it is the side
            // that goes across
            let x_across = if long_across { wide } else { !wide };
            // a roll paper as long as the drawing, plus nothing: the
            // plotter cuts it there
            PaperAsk {
                size: Some(if x_across {
                    DVec2::new(across, length)
                } else {
                    DVec2::new(length, across)
                }),
                landscape: !x_across,
                turned: false,
                name: "",
            }
        }
    }
}

/// The ISO name of a paper `size` (either way round), within 1 mm.
pub fn iso_name(size: DVec2) -> Option<&'static str> {
    let (a, b) = (size.x.min(size.y), size.x.max(size.y));
    ISO_A
        .iter()
        .find(|p| (p.width - a).abs() < 1.0 && (p.height - b).abs() < 1.0)
        .map(|p| p.name)
}

/// The paper a printer really gave, as the sheet sees it (mm, y up from
/// its lower left corner), and the part of it the printer can reach.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaperGot {
    pub size: DVec2,
    pub printable_lo: DVec2,
    pub printable_hi: DVec2,
}

impl PaperGot {
    /// A paper the printer reaches to the edge (a PDF).
    pub fn whole(size: DVec2) -> Self {
        Self {
            size,
            printable_lo: DVec2::ZERO,
            printable_hi: size,
        }
    }
}

/// Where a sheet lands on its paper: paper = offset + scale × (turned) sheet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub scale: f64,
    /// The sheet turned a quarter round, counter-clockwise.
    pub turned: bool,
    pub offset: DVec2,
    /// What of the sheet's drawn area (inside its frame's margin) the
    /// printer cannot reach, mm at most on any side; 0 when all prints.
    pub cut: f64,
}

impl Placement {
    pub fn map(&self, p: DVec2) -> DVec2 {
        let q = if self.turned {
            DVec2::new(-p.y, p.x)
        } else {
            p
        };
        self.offset + q * self.scale
    }

    /// The sheet's corners on the paper, lower left and upper right.
    pub fn sheet_box(&self, sheet: DVec2) -> (DVec2, DVec2) {
        let a = self.map(DVec2::ZERO);
        let b = self.map(sheet);
        (a.min(b), a.max(b))
    }
}

/// How far in from the sheet's edges its drawing starts: its frame sits
/// 10 mm in, a millimetre is spared for the frame's line and the cut.
pub const DRAWN_INSET: f64 = crate::sheet::MARGIN - 1.0;

/// Where a sheet of `sheet` mm goes on `paper`: at 1:1 in the paper's
/// middle (an exact paper then takes it edge to edge), shrunk to fit,
/// into the middle of the printable area.
pub fn place(sheet: DVec2, paper: &PaperGot, fit: Fit, turned: bool) -> Placement {
    let along = if turned {
        DVec2::new(sheet.y, sheet.x)
    } else {
        sheet
    };
    let room = (paper.printable_hi - paper.printable_lo).max(DVec2::splat(1.0));
    let scale = match fit {
        Fit::Actual => 1.0,
        Fit::Shrink => (room.x / along.x).min(room.y / along.y).min(1.0),
    };
    let centre = match fit {
        Fit::Actual => paper.size / 2.0,
        Fit::Shrink => (paper.printable_lo + paper.printable_hi) / 2.0,
    };
    // the turned sheet's box runs from (-h, 0) to (0, w) before scaling
    let lo = if turned {
        DVec2::new(-sheet.y, 0.0)
    } else {
        DVec2::ZERO
    };
    let offset = centre - (lo + along / 2.0) * scale;
    let mut p = Placement {
        scale,
        turned,
        offset,
        cut: 0.0,
    };
    let inset = DVec2::splat(DRAWN_INSET);
    let a = p.map(inset);
    let b = p.map(sheet - inset);
    let (lo, hi) = (a.min(b), a.max(b));
    let cut = (paper.printable_lo - lo)
        .max(hi - paper.printable_hi)
        .max_element()
        .max(0.0);
    p.cut = if cut > 0.2 { cut } else { 0.0 };
    p
}

/// The grey a colour prints as in black and white.
pub fn luma([r, g, b]: [f32; 3]) -> f64 {
    (0.299 * r + 0.587 * g + 0.114 * b) as f64
}

/// A [`Canvas`] that puts what is drawn in sheet millimetres onto paper
/// millimetres by a [`Placement`], in colour or in black and white.
pub struct OnPaper<'a> {
    pub inner: &'a mut dyn Canvas,
    pub place: Placement,
    /// Black and white: lines and text black, fills grey by their
    /// lightness.
    pub mono: bool,
}

impl OnPaper<'_> {
    fn pts(&self, pts: &[DVec2]) -> Vec<DVec2> {
        pts.iter().map(|p| self.place.map(*p)).collect()
    }

    fn stroke(&self, s: Stroke) -> Stroke {
        Stroke {
            width: s.width * self.place.scale,
            gray: if self.mono { 0.0 } else { s.gray },
            dash: s.dash.map(|d| d.map(|v| v * self.place.scale)),
        }
    }

    fn angle(&self, a: f64) -> f64 {
        if self.place.turned {
            a + std::f64::consts::FRAC_PI_2
        } else {
            a
        }
    }
}

impl Canvas for OnPaper<'_> {
    fn line(&mut self, pts: &[DVec2], s: Stroke) {
        let (p, s) = (self.pts(pts), self.stroke(s));
        self.inner.line(&p, s);
    }

    fn line_rgb(&mut self, pts: &[DVec2], s: Stroke, rgb: [f32; 3]) {
        let (p, s) = (self.pts(pts), self.stroke(s));
        if self.mono {
            self.inner.line(&p, s);
        } else {
            self.inner.line_rgb(&p, s, rgb);
        }
    }

    fn fill(&mut self, pts: &[DVec2], gray: f64) {
        let p = self.pts(pts);
        self.inner.fill(&p, gray);
    }

    fn fill_rgb(&mut self, pts: &[DVec2], rgb: [f32; 3]) {
        let p = self.pts(pts);
        if self.mono {
            self.inner.fill(&p, luma(rgb));
        } else {
            self.inner.fill_rgb(&p, rgb);
        }
    }

    fn tint(&mut self, pts: &[DVec2], rgb: [f32; 3]) {
        let p = self.pts(pts);
        let rgb = if self.mono {
            [luma(rgb) as f32; 3]
        } else {
            rgb
        };
        self.inner.tint(&p, rgb);
    }

    fn text_angle(&mut self, at: DVec2, size: f64, text: &str, anchor: Anchor, angle: f64) {
        let (at, a) = (self.place.map(at), self.angle(angle));
        self.inner
            .text_angle(at, size * self.place.scale, text, anchor, a);
    }

    fn styled_text(&mut self, at: DVec2, text: &str, anchor: Anchor, angle: f64, look: &TextLook) {
        let mut look = *look;
        look.size *= self.place.scale;
        look.end_size *= self.place.scale;
        if self.mono {
            look.rgb = None;
        }
        let (at, a) = (self.place.map(at), self.angle(angle));
        self.inner.styled_text(at, text, anchor, a, &look);
    }

    fn image(&mut self, lo: DVec2, hi: DVec2, clip: Option<(DVec2, DVec2)>, raster: &Arc<Raster>) {
        let boxed = |a: DVec2, b: DVec2| {
            let (a, b) = (self.place.map(a), self.place.map(b));
            (a.min(b), a.max(b))
        };
        let (l, h) = boxed(lo, hi);
        let clip = clip.map(|(a, b)| boxed(a, b));
        if self.place.turned {
            let r = Arc::new(turned(raster));
            self.inner.image(l, h, clip, &r);
        } else {
            self.inner.image(l, h, clip, raster);
        }
    }
}

/// An image turned a quarter round counter-clockwise.
pub fn turned(r: &Raster) -> Raster {
    let (w, h) = (r.width, r.height);
    let mut rgba = vec![0u8; w * h * 4];
    // the new image is h wide and w high; its row y, column x shows the
    // old pixel at row x, column w-1-y
    for y in 0..w {
        for x in 0..h {
            let from = (x * w + (w - 1 - y)) * 4;
            let to = (y * h + x) * 4;
            rgba[to..to + 4].copy_from_slice(&r.rgba[from..from + 4]);
        }
    }
    Raster {
        width: h,
        height: w,
        rgba,
        key: r.key ^ 0x7e57_ed00,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A1: DVec2 = DVec2::new(841.0, 594.0);

    #[test]
    fn a_sheet_asks_for_its_own_size_turned_as_it_is() {
        let a = paper_for(A1, PaperChoice::Sheet);
        assert_eq!(a.size, Some(A1));
        assert!(a.landscape && !a.turned);
        assert_eq!(a.name, "A1");
        let a4 = paper_for(DVec2::new(210.0, 297.0), PaperChoice::Sheet);
        assert!(!a4.landscape && a4.name == "A4");
        // 3×A4 is no ISO paper: a custom size
        assert_eq!(
            paper_for(DVec2::new(630.0, 297.0), PaperChoice::Sheet).name,
            ""
        );
    }

    #[test]
    fn an_iso_paper_is_turned_the_sheets_way() {
        let a = paper_for(A1, PaperChoice::Iso(1));
        assert_eq!(a.size, Some(DVec2::new(420.0, 297.0)));
        assert!(a.landscape && a.name == "A3");
    }

    #[test]
    fn a_roll_takes_the_long_side_across_when_it_fits() {
        // A1 on a 914 roll: 841 across, 594 of roll
        let a = paper_for(A1, PaperChoice::Roll(914.0));
        assert_eq!(a.size, Some(DVec2::new(914.0, 594.0)));
        assert!(!a.landscape);
        // A1 on a 610 roll: 594 across, 841 of roll, the sheet turned
        let b = paper_for(A1, PaperChoice::Roll(610.0));
        assert_eq!(b.size, Some(DVec2::new(841.0, 610.0)));
        assert!(b.landscape);
    }

    #[test]
    fn at_1_1_an_exact_paper_takes_the_sheet_edge_to_edge() {
        let got = PaperGot {
            size: A1,
            printable_lo: DVec2::splat(4.0),
            printable_hi: A1 - DVec2::splat(4.0),
        };
        let p = place(A1, &got, Fit::Actual, false);
        assert_eq!(p.scale, 1.0);
        assert!((p.map(DVec2::ZERO) - DVec2::ZERO).length() < 1e-9);
        assert_eq!(p.cut, 0.0, "the frame 10 mm in prints");
        // on A3 at 1:1 most of it is cut away
        let a3 = PaperGot::whole(DVec2::new(420.0, 297.0));
        assert!(place(A1, &a3, Fit::Actual, false).cut > 200.0);
        // shrunk, it fits A3 whole: half size, centred
        let s = place(A1, &a3, Fit::Shrink, false);
        assert!((s.scale - 0.5).abs() < 0.01 && s.cut == 0.0);
        let (lo, hi) = s.sheet_box(A1);
        assert!(lo.x >= -1e-9 && hi.x <= 420.0 + 1e-9);
        assert!(((lo + hi) / 2.0 - DVec2::new(210.0, 148.5)).length() < 1e-9);
    }

    #[test]
    fn a_turned_sheet_fills_its_upright_paper() {
        let paper = PaperGot::whole(DVec2::new(594.0, 841.0));
        let p = place(A1, &paper, Fit::Actual, true);
        let (lo, hi) = p.sheet_box(A1);
        assert!(lo.length() < 1e-9 && (hi - DVec2::new(594.0, 841.0)).length() < 1e-9);
        assert_eq!(p.cut, 0.0);
    }

    #[test]
    fn a_printable_area_too_small_cuts_the_frame() {
        // a printer that reaches only 12 mm from the edges cuts 2 mm
        // of the frame (10 mm in, 1 mm spared)
        let got = PaperGot {
            size: DVec2::new(297.0, 210.0),
            printable_lo: DVec2::splat(12.0),
            printable_hi: DVec2::new(285.0, 198.0),
        };
        let p = place(DVec2::new(297.0, 210.0), &got, Fit::Actual, false);
        assert!((p.cut - 3.0).abs() < 1e-9, "{}", p.cut);
    }

    #[derive(Default)]
    struct Log(Vec<String>);
    impl Canvas for Log {
        fn line(&mut self, pts: &[DVec2], s: Stroke) {
            self.0
                .push(format!("line {:?} w{} g{}", pts, s.width, s.gray));
        }
        fn fill(&mut self, pts: &[DVec2], gray: f64) {
            self.0.push(format!("fill {} {gray:.3}", pts.len()));
        }
        fn text_angle(&mut self, at: DVec2, size: f64, text: &str, _: Anchor, angle: f64) {
            self.0.push(format!("text {text} {at:?} {size} {angle:.3}"));
        }
        fn image(&mut self, lo: DVec2, hi: DVec2, _: Option<(DVec2, DVec2)>, r: &Arc<Raster>) {
            self.0
                .push(format!("image {lo:?} {hi:?} {}x{}", r.width, r.height));
        }
    }

    #[test]
    fn on_paper_moves_scales_and_greys() {
        let mut log = Log::default();
        let place = Placement {
            scale: 0.5,
            turned: false,
            offset: DVec2::new(10.0, 20.0),
            cut: 0.0,
        };
        let mut c = OnPaper {
            inner: &mut log,
            place,
            mono: true,
        };
        c.line_rgb(
            &[DVec2::ZERO, DVec2::new(100.0, 0.0)],
            Stroke::solid(0.5, 0.3),
            [1.0, 0.0, 0.0],
        );
        c.fill_rgb(&[DVec2::ZERO, DVec2::X, DVec2::Y], [1.0, 1.0, 1.0]);
        c.text(DVec2::new(20.0, 0.0), 3.0, "1.NP", Anchor::Start);
        assert_eq!(
            log.0[0],
            "line [DVec2(10.0, 20.0), DVec2(60.0, 20.0)] w0.25 g0"
        );
        assert_eq!(log.0[1], "fill 3 1.000");
        assert_eq!(log.0[2], "text 1.NP DVec2(20.0, 20.0) 1.5 0.000");
    }

    #[test]
    fn a_turned_image_is_turned_with_the_sheet() {
        // 2 wide, 1 high: red then blue
        let r = Raster {
            width: 2,
            height: 1,
            rgba: vec![255, 0, 0, 255, 0, 0, 255, 255],
            key: 1,
        };
        let t = turned(&r);
        assert_eq!((t.width, t.height), (1, 2));
        // turned counter-clockwise the right (blue) end goes up, on top
        assert_eq!(&t.rgba[..4], &[0, 0, 255, 255]);
        let mut log = Log::default();
        let mut c = OnPaper {
            inner: &mut log,
            place: Placement {
                scale: 1.0,
                turned: true,
                offset: DVec2::new(100.0, 0.0),
                cut: 0.0,
            },
            mono: false,
        };
        c.image(DVec2::ZERO, DVec2::new(20.0, 10.0), None, &Arc::new(r));
        c.text_angle(DVec2::ZERO, 2.0, "x", Anchor::Start, 0.0);
        assert_eq!(log.0[0], "image DVec2(90.0, 0.0) DVec2(100.0, 20.0) 1x2");
        assert!(log.0[1].ends_with("1.571"));
    }
}
