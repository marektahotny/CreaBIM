//! The interface's language: English as written in the code, or a
//! translation from `i18n/<lang>.json` beside this crate's sources.
//!
//! The English text is the key, as in gettext: `t("Wall")` gives "Stena" in
//! Slovak and "Wall" when there is no translation for it, so untranslated
//! text still reads. The language is process-wide, picked once from the
//! settings and changed from *View › Language*. Tests run in English.
//!
//! Most text reaches the screen through the web panes as JSON, so
//! [`translate_json`] translates a pane's whole state on its way out
//! ([`crate::webui::WebUi::send`]); menus translate their items as they are
//! built ([`crate::ui_model::Item`]).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

/// A language the interface can speak.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Lang {
    #[default]
    En,
    Sk,
    Cs,
}

impl Lang {
    pub const ALL: [Lang; 3] = [Lang::Sk, Lang::Cs, Lang::En];

    pub fn id(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Sk => "sk",
            Lang::Cs => "cs",
        }
    }

    /// The language's own name, so it can be found whatever is showing.
    pub fn label(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Sk => "Slovenčina",
            Lang::Cs => "Čeština",
        }
    }

    pub fn from_id(id: &str) -> Option<Lang> {
        Lang::ALL.into_iter().find(|l| l.id() == id)
    }

    fn source(self) -> &'static str {
        match self {
            Lang::En => "{}",
            Lang::Sk => include_str!("../i18n/sk.json"),
            Lang::Cs => include_str!("../i18n/cs.json"),
        }
    }
}

static LANG: AtomicU8 = AtomicU8::new(0);

/// The language now in use.
pub fn lang() -> Lang {
    match LANG.load(Ordering::Relaxed) {
        1 => Lang::Sk,
        2 => Lang::Cs,
        _ => Lang::En,
    }
}

/// Speak `l` from now on. Text already on screen follows on the next frame.
pub fn set_lang(l: Lang) {
    let n = match l {
        Lang::En => 0,
        Lang::Sk => 1,
        Lang::Cs => 2,
    };
    LANG.store(n, Ordering::Relaxed);
}

type Table = HashMap<&'static str, &'static str>;

/// A language's table, read once. A broken file gives an empty table (and
/// English text), never a failure: a test checks that every file parses.
fn table(l: Lang) -> &'static Table {
    static TABLES: [OnceLock<Table>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let i = l as usize;
    TABLES[i].get_or_init(|| parse(l.source()).unwrap_or_default())
}

fn parse(src: &'static str) -> Result<Table, serde_json::Error> {
    let map: HashMap<String, String> = serde_json::from_str(src)?;
    // read once per process and kept to the end: leaking is the point
    Ok(map
        .into_iter()
        .filter(|(k, v)| !k.starts_with("//") && !v.is_empty())
        .map(|(k, v)| {
            (
                &*Box::leak(k.into_boxed_str()),
                &*Box::leak(v.into_boxed_str()),
            )
        })
        .collect())
}

/// `english` in the interface's language.
pub fn t(english: &str) -> &str {
    lookup(lang(), english).unwrap_or(english)
}

/// `english` in the interface's language, as an owned string. Joined
/// paths such as `Create › Wall` are translated part by part.
pub fn tr(english: &str) -> String {
    let l = lang();
    if l == Lang::En {
        return english.to_string();
    }
    if let Some(s) = lookup(l, english) {
        return s.to_string();
    }
    if english.contains(" › ") {
        return english
            .split(" › ")
            .map(|p| lookup(l, p).unwrap_or(p))
            .collect::<Vec<_>>()
            .join(" › ");
    }
    english.to_string()
}

fn lookup(l: Lang, english: &str) -> Option<&'static str> {
    if l == Lang::En || english.is_empty() {
        return None;
    }
    table(l).get(english).copied()
}

/// The keys of a pane's state whose text is read by people.
const TEXT_KEYS: [&str; 11] = [
    "label",
    "message",
    "samples_title",
    "hint",
    "name",
    "title",
    "text",
    "placeholder",
    "header",
    "note",
    "section",
];

/// Translate every piece of text in a pane's state, in place.
pub fn translate_json(v: &mut serde_json::Value) {
    if lang() == Lang::En {
        return;
    }
    walk(v);
}

fn walk(v: &mut serde_json::Value) {
    use serde_json::Value;
    match v {
        Value::Object(map) => {
            // a menu button shows its value as its caption; elsewhere a value
            // is what was typed, and stays as it was
            let menu = map.get("kind").and_then(Value::as_str) == Some("menu");
            for (k, val) in map.iter_mut() {
                match val {
                    Value::String(s)
                        if TEXT_KEYS.contains(&k.as_str()) || (menu && k == "value") =>
                    {
                        let out = tr(s);
                        if out != *s {
                            *s = out;
                        }
                    }
                    _ => walk(val),
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(walk),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_file_parses() {
        for l in Lang::ALL {
            parse(l.source()).unwrap_or_else(|e| panic!("i18n/{}.json: {e}", l.id()));
        }
    }

    #[test]
    fn english_is_the_key_and_the_fallback() {
        assert_eq!(lookup(Lang::En, "Wall"), None);
        assert_eq!(lookup(Lang::Sk, "Wall"), Some("Stena"));
        assert_eq!(lookup(Lang::Cs, "Wall"), Some("Stěna"));
        assert_eq!(lookup(Lang::Sk, "No such text, surely"), None);
    }

    #[test]
    fn languages_round_trip_by_id() {
        for l in Lang::ALL {
            assert_eq!(Lang::from_id(l.id()), Some(l));
        }
    }

    #[test]
    fn slovak_and_czech_cover_the_same_text() {
        // a text translated in one and not the other is easy to miss
        let sk = parse(Lang::Sk.source()).unwrap();
        let cs = parse(Lang::Cs.source()).unwrap();
        let mut missing: Vec<_> = sk.keys().filter(|k| !cs.contains_key(*k)).collect();
        missing.sort();
        assert!(
            missing.is_empty(),
            "in sk.json but not cs.json: {missing:?}"
        );
    }
}
