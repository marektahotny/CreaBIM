//! The drawings' own typeface on this computer: a font file the user
//! picks in Settings (or one found by its usual name, Avenir Next
//! Condensed, beside the program or among the system's fonts) and the
//! width of the letters. See [`creabim_geom::font`]: PDF and SVG take
//! it from there; here it is given to the screen.

use creabim_geom::font::DrawingFont;
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// The family the screen draws Sans text in, once [`apply`] has named it.
const FAMILY: &str = "drawing";
static NAMED: AtomicBool = AtomicBool::new(false);

/// File names the face goes by when none is picked.
const USUAL: [&str; 4] = [
    "AvenirNextCondensed-Regular.ttf",
    "AvenirNextCondensed-Regular.otf",
    "Avenir Next Condensed Regular.ttf",
    "Avenir Next Condensed.ttf",
];

/// The screen's family for a drawing's Sans text.
pub fn family() -> egui::FontFamily {
    if NAMED.load(Ordering::Relaxed) {
        egui::FontFamily::Name(FAMILY.into())
    } else {
        egui::FontFamily::Proportional
    }
}

fn read(path: &Path) -> Option<DrawingFont> {
    DrawingFont::parse(std::fs::read(path).ok()?)
}

/// Folders a face is looked for in: `fonts` beside the program, the
/// settings' own, and the system's and the user's fonts.
fn folders() -> Vec<PathBuf> {
    let mut v = vec![];
    if let Some(dir) = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
    {
        v.push(dir.join("fonts"));
        v.push(dir);
    }
    if let Some(d) = crate::files::settings_dir() {
        v.push(d.join("fonts"));
    }
    let env = |k: &str| std::env::var_os(k).map(PathBuf::from);
    if let Some(w) = env("WINDIR") {
        v.push(w.join("Fonts"));
    }
    if let Some(l) = env("LOCALAPPDATA") {
        v.push(l.join("Microsoft").join("Windows").join("Fonts"));
    }
    if let Some(h) = env("HOME") {
        v.push(h.join("Library").join("Fonts"));
        v.push(h.join(".fonts"));
        v.push(h.join(".local").join("share").join("fonts"));
    }
    v.push("/Library/Fonts".into());
    v.push("/usr/share/fonts/truetype".into());
    v.push("/usr/local/share/fonts".into());
    v
}

/// The face at `picked`, else the usual one where it is found.
pub fn find(picked: &str) -> Option<(PathBuf, DrawingFont)> {
    if !picked.is_empty() {
        let p = PathBuf::from(picked);
        return read(&p).map(|f| (p, f));
    }
    folders().into_iter().find_map(|d| {
        USUAL.iter().find_map(|n| {
            let p = d.join(n);
            read(&p).map(|f| (p, f))
        })
    })
}

/// Set the drawings' text as the settings say, on screen and on paper;
/// the face's name and file if one is in use.
pub fn apply(ctx: &egui::Context, settings: &crate::files::Settings) -> Option<(String, PathBuf)> {
    let found = find(&settings.drawing_font);
    let mut defs = egui::FontDefinitions::default();
    let mut list = defs
        .families
        .get(&egui::FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    if let Some((_, f)) = &found {
        defs.font_data.insert(
            FAMILY.into(),
            Arc::new(egui::FontData::from_owned(f.bytes.to_vec())),
        );
        list.insert(0, FAMILY.into());
    }
    defs.families
        .insert(egui::FontFamily::Name(FAMILY.into()), list);
    ctx.set_fonts(defs);
    NAMED.store(true, Ordering::Relaxed);
    let name = found
        .as_ref()
        .map(|(p, f)| (f.family.clone(), p.to_path_buf()));
    creabim_geom::font::set_setup(found.map(|(_, f)| Arc::new(f)), settings.text_width);
    name
}
