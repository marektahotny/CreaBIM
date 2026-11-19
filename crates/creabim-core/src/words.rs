//! Words that drawings write themselves (the headings of a window list,
//! "TOTAL"), in the language the application runs in: the application
//! registers its translator once, and everything else stays English.

use std::sync::OnceLock;

static HOOK: OnceLock<fn(&str) -> String> = OnceLock::new();

/// Register the translator; the first call wins.
pub fn set_translator(f: fn(&str) -> String) {
    let _ = HOOK.set(f);
}

/// `english` in the application's language.
pub fn tr(english: &str) -> String {
    match HOOK.get() {
        Some(f) => f(english),
        None => english.to_string(),
    }
}
