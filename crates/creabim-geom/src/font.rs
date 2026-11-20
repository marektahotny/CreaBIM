//! The drawings' own typeface and the width of their letters: a TrueType
//! or OpenType file the user picks (a licensed face such as Avenir Next
//! Condensed is theirs to supply, never the program's to ship), measured
//! here so that the screen, PDF and SVG lay text out alike, and a width
//! factor (0.7 draws every letter at 70 % of its width, as a CAD text
//! style's).
//!
//! The face stands in for [`creabim_core::Font::Sans`], the family every
//! text style starts with; Serif and Mono stay as they are. The width
//! factor is for all three.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// A typeface read from a file.
#[derive(Debug)]
pub struct DrawingFont {
    /// Its family, as the file names it ("Avenir Next Condensed").
    pub family: String,
    /// Its PostScript name, no spaces ("AvenirNextCondensed-Regular").
    pub ps_name: String,
    /// The file.
    pub bytes: Arc<Vec<u8>>,
    /// TrueType outlines (`glyf`); else CFF, as most `.otf`.
    pub truetype: bool,
    /// Font units to the em.
    pub units: f64,
    pub ascent: f64,
    pub descent: f64,
    pub cap_height: f64,
    /// The box around all its glyphs, font units.
    pub bbox: [f64; 4],
    /// Each character's glyph and how far it advances, font units.
    glyphs: HashMap<char, (u16, u16)>,
}

impl DrawingFont {
    /// The face in `bytes` (the first of a collection); none if it is no
    /// font or has no letters.
    pub fn parse(bytes: Vec<u8>) -> Option<Self> {
        let face = ttf_parser::Face::parse(&bytes, 0).ok()?;
        let name = |id: u16| {
            face.names()
                .into_iter()
                .filter(|n| n.name_id == id)
                .find_map(|n| n.to_string())
        };
        let family = name(ttf_parser::name_id::TYPOGRAPHIC_FAMILY)
            .or_else(|| name(ttf_parser::name_id::FAMILY))?;
        let ps_name: String = name(ttf_parser::name_id::POST_SCRIPT_NAME)
            .unwrap_or_else(|| family.clone())
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        let mut glyphs = HashMap::new();
        for sub in face.tables().cmap?.subtables {
            if !sub.is_unicode() {
                continue;
            }
            sub.codepoints(|cp| {
                let Some(c) = char::from_u32(cp) else { return };
                if let Some(g) = sub.glyph_index(cp) {
                    let adv = face.glyph_hor_advance(g).unwrap_or(0);
                    glyphs.entry(c).or_insert((g.0, adv));
                }
            });
        }
        if !glyphs.contains_key(&'a') && !glyphs.contains_key(&'0') {
            return None;
        }
        let units = f64::from(face.units_per_em());
        let b = face.global_bounding_box();
        let font = Self {
            family,
            ps_name: if ps_name.is_empty() {
                "DrawingFont".into()
            } else {
                ps_name
            },
            truetype: face.tables().glyf.is_some(),
            units,
            ascent: f64::from(face.ascender()),
            descent: f64::from(face.descender()),
            cap_height: face
                .capital_height()
                .map_or(units * 0.7, f64::from)
                .max(units * 0.3),
            bbox: [b.x_min, b.y_min, b.x_max, b.y_max].map(f64::from),
            glyphs,
            bytes: Arc::new(vec![]),
        };
        Some(Self {
            bytes: Arc::new(bytes),
            ..font
        })
    }

    /// The glyph of `c` and its advance in font units; the face's
    /// "missing" glyph for a character it has not.
    pub fn glyph(&self, c: char) -> (u16, u16) {
        self.glyphs.get(&c).copied().unwrap_or_else(|| {
            // a narrow or non-breaking space is a space
            let space = matches!(c, '\u{a0}' | '\u{202f}' | '\u{2009}');
            match self.glyphs.get(&' ') {
                Some(s) if space => *s,
                s => (0, s.map_or(500, |s| s.1 * 2)),
            }
        })
    }

    /// Width of `text` set `size` high (its em), unstretched.
    pub fn width(&self, text: &str, size: f64) -> f64 {
        text.chars()
            .map(|c| f64::from(self.glyph(c).1))
            .sum::<f64>()
            / self.units
            * size
    }
}

/// How the drawings' text is set: the face in place of Sans, and the
/// width of all letters.
#[derive(Clone, Debug)]
pub struct TextSetup {
    pub font: Option<Arc<DrawingFont>>,
    /// 1 is each face's own width.
    pub stretch: f64,
}

impl Default for TextSetup {
    fn default() -> Self {
        Self {
            font: None,
            stretch: 1.0,
        }
    }
}

impl TextSetup {
    /// The face a look's family is drawn in, if it is the user's.
    pub fn face(&self, font: creabim_core::Font) -> Option<&Arc<DrawingFont>> {
        match font {
            creabim_core::Font::Sans => self.font.as_ref(),
            _ => None,
        }
    }
}

/// How high the capitals of the standard faces stand, a share of their
/// em: Helvetica (and Arial), Times, Courier.
pub const SANS_CAP: f64 = 0.718;
pub const SERIF_CAP: f64 = 0.662;
pub const MONO_CAP: f64 = 0.571;

impl TextSetup {
    /// How high capitals stand in `font`, a share of its em: the drawings'
    /// own face by its own measure, else the standard face's.
    pub fn cap_ratio(&self, font: creabim_core::Font) -> f64 {
        match self.face(font) {
            Some(f) if f.units > 0.0 => f.cap_height / f.units,
            _ => match font {
                creabim_core::Font::Sans => SANS_CAP,
                creabim_core::Font::Serif => SERIF_CAP,
                creabim_core::Font::Mono => MONO_CAP,
            },
        }
    }

    /// The em (the size a face is set at) of letters whose capitals stand
    /// `cap` high. A drawing's text size is the height of its capitals, as
    /// ISO 3098 measures it: 2.5 mm text has 2.5 mm high capitals on paper.
    pub fn em(&self, cap: f64, font: creabim_core::Font) -> f64 {
        cap / self.cap_ratio(font)
    }
}

/// [`TextSetup::em`] in the setup in use.
pub fn em(cap: f64, font: creabim_core::Font) -> f64 {
    setup().em(cap, font)
}

static SETUP: RwLock<Option<TextSetup>> = RwLock::new(None);

/// The setup in use.
pub fn setup() -> TextSetup {
    SETUP
        .read()
        .ok()
        .and_then(|s| s.clone())
        .unwrap_or_default()
}

/// Use `font` (none: the built-in families) and letters `stretch` wide,
/// from now on, everywhere.
pub fn set_setup(font: Option<Arc<DrawingFont>>, stretch: f64) {
    let stretch = if stretch.is_finite() {
        stretch.clamp(0.3, 2.0)
    } else {
        1.0
    };
    if let Ok(mut s) = SETUP.write() {
        *s = Some(TextSetup { font, stretch });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_no_font_is_not_read() {
        assert!(DrawingFont::parse(b"not a font".to_vec()).is_none());
        assert!(DrawingFont::parse(vec![]).is_none());
    }

    #[test]
    fn the_setup_starts_plain() {
        let s = TextSetup::default();
        assert!(s.font.is_none() && s.stretch == 1.0);
        assert!(s.face(creabim_core::Font::Sans).is_none());
    }
}
