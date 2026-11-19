//! What a sanitary fixture is on the plumbing drawings: its mark in plan
//! (U, WC, D...) and its lines in the legend: the fixture and its tap,
//! its size, its trap and how it is connected, with the sizes of its
//! pipes. Ported from PilonDesigner's `fixturespec.ts`; the texts are the
//! usual ones of Slovak plumbing drawings.

use creabim_core::fixture::{self, ConnKind};
use creabim_core::{ElementId, ElementKind, Model};

/// A fixture type's mark and description.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixtureSpec {
    /// Its mark on the drawing.
    pub mark: String,
    /// The fixture and its tap.
    pub description: String,
    pub size: String,
    /// Its trap.
    pub trap: String,
    /// Its valves and the sizes of its pipes.
    pub connection: String,
}

/// (mark, description, trap, connection) of the family `f` (lower case).
fn usual(f: &str) -> Option<(&'static str, &'static str, &'static str, &'static str)> {
    let has = |s: &str| f.contains(s);
    Some(if has("závesné wc") {
        (
            "WC",
            "Závesné WC, podomietkový inštalačný modul s nádržkou a ovládaním spredu, sedadlo s pomalým zatváraním",
            "odpad DN 100 (zápachová uzávierka v mise)",
            "rohový ventil G 1/2\" v module",
        )
    } else if has("wc") || has("záchod") {
        (
            "WC",
            "Stojace kombi WC s nádržkou, sedadlo s pomalým zatváraním",
            "odpad DN 100 (zápachová uzávierka v mise)",
            "rohový ventil G 1/2\"",
        )
    } else if has("umývadielko") || has("umývatko") {
        (
            "UM",
            "Keramické umývatko, stojanková jednopáková batéria",
            "zápachová uzávierka HL 134 + pripojovacia súprava 5/4\"",
            "rohové ventily 2× G 3/8\"",
        )
    } else if has("umývačka") {
        (
            "UR",
            "Umývačka riadu",
            "zápachová uzávierka HL 404 – DN 50",
            "1× rohový ventil G 1/2\"",
        )
    } else if has("umýv") {
        (
            "U",
            "Keramické umývadlo, stojanková jednopáková batéria",
            "zápachová uzávierka HL 134 + pripojovacia súprava 5/4\"",
            "rohové ventily 2× G 3/8\"",
        )
    } else if has("drez") {
        (
            "D",
            "Nerezový drez, stojanková jednopáková batéria",
            "zápachová uzávierka HL 100",
            "rohové ventily 2× G 3/8\"",
        )
    } else if has("práčka") {
        (
            "P",
            "Automatická práčka",
            "zápachová uzávierka HL 404 – DN 50",
            "1× rohový ventil G 1/2\"",
        )
    } else if has("sušičk") {
        (
            "SU",
            "Sušička prádla",
            "zápachová uzávierka HL 404 – DN 50",
            "",
        )
    } else if has("sprch") {
        (
            "S",
            "Sprchový kút, nástenná jednopáková sprchová batéria",
            "zápachová uzávierka HL 50 – DN 50",
            "",
        )
    } else if has("vaň") {
        (
            "V",
            "Vaňa, nástenná jednopáková vaňová batéria",
            "zápachová uzávierka HL 555 – DN 50",
            "",
        )
    } else if has("výtok") {
        ("VV", "Výtokový ventil s hadicovou prípojkou", "", "G 1/2\"")
    } else {
        return None;
    })
}

/// The usual description of a fixture of `family`: its size from its
/// shape, its pipes' sizes from its connectors.
pub fn default_spec(family: &str) -> FixtureSpec {
    let f = family.to_lowercase();
    let sh = fixture::shape(family, None);
    let conns = fixture::local_connectors(family, None);
    let dn = |k: ConnKind| conns.iter().find(|c| c.kind == k).map(|c| c.dn);
    let dims: Vec<String> = [
        (ConnKind::SV, "SV DN"),
        (ConnKind::TV, "TV DN"),
        (ConnKind::K, "odpad DN"),
    ]
    .into_iter()
    .filter_map(|(k, text)| dn(k).map(|d| format!("{text} {d:.0}")))
    .collect();
    let dims = dims.join(", ");
    let (mark, description, trap, connection) = match usual(&f) {
        Some((m, d, t, c)) => (m.to_string(), d.to_string(), t, c),
        None => (
            family.chars().take(2).collect::<String>().to_uppercase(),
            family.to_string(),
            "",
            "",
        ),
    };
    FixtureSpec {
        mark,
        description,
        size: format!("{:.0}×{:.0} mm", sh.width * 1000.0, sh.depth * 1000.0),
        trap: trap.to_string(),
        connection: [connection, dims.as_str()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · "),
    }
}

/// The description of a maker's product from the catalogue (its type's
/// catalogue key), by its technical sheet; none for a generic fixture.
pub fn product_spec(key: Option<&str>, family: &str) -> Option<FixtureSpec> {
    let key = key?.rsplit('/').next()?;
    let (description, size, trap) = match key {
        "jika-basin-lyra-plus-400" => (
            "Umývadielko JIKA Lyra Plus H8153800001041, závesné, s jednopákovou stojankovou batériou",
            "400×340 mm",
            "sifón fľašový G 5/4\" × DN 32",
        ),
        "jika-basin-cubito-450" => (
            "Umývadielko asymetrické JIKA Cubito H8114240001061, batéria vpravo, s jednopákovou stojankovou batériou",
            "450×250 mm",
            "sifón umývadlový G 5/4\" × DN 32 (JIKA H3742F07160001)",
        ),
        "jika-wc-deep-modul" => (
            "Závesný klozet JIKA Deep Rimless H8206160000001 na podomietkovom module JIKA WC system H8956520000001 v predstene, ovládacie tlačidlo JIKA PL3",
            "360×510 mm",
            "",
        ),
        _ => return None,
    };
    let base = default_spec(family);
    Some(FixtureSpec {
        description: description.into(),
        size: size.into(),
        trap: if trap.is_empty() {
            base.trap
        } else {
            trap.into()
        },
        ..base
    })
}

/// A row of the legend: a type, how many there are and which.
#[derive(Clone, Debug, PartialEq)]
pub struct LegendRow {
    pub family: &'static str,
    pub spec: FixtureSpec,
    pub fixtures: Vec<ElementId>,
}

/// The legend of the sanitary fixtures placed (on `level` only, if
/// given), by mark; heaters, manifolds and fan-coils are not in it.
pub fn legend(model: &Model, level: Option<ElementId>) -> Vec<LegendRow> {
    let mut rows: Vec<LegendRow> = vec![];
    for e in model.iter() {
        let ElementKind::Object(o) = &e.kind else {
            continue;
        };
        if level.is_some_and(|l| l != o.level) {
            continue;
        }
        let Some(family) = fixture::family_of(model, e) else {
            continue;
        };
        if !fixture::is_sanitary(family) {
            continue;
        }
        let key = creabim_core::objects::type_of(model, e).and_then(|t| t.catalogue.as_deref());
        let spec = product_spec(key, family).unwrap_or_else(|| default_spec(family));
        // the same description is one row
        match rows.iter_mut().find(|r| r.spec == spec) {
            Some(r) => r.fixtures.push(e.id),
            None => rows.push(LegendRow {
                family,
                spec,
                fixtures: vec![e.id],
            }),
        }
    }
    rows.sort_by(|a, b| {
        a.spec
            .mark
            .cmp(&b.spec.mark)
            .then(a.spec.description.cmp(&b.spec.description))
    });
    rows
}

/// A type's lines in the legend: the fixture and its size, then its trap
/// and its connection, each after a dash.
pub fn legend_lines(s: &FixtureSpec) -> Vec<String> {
    let mut lines = vec![if s.size.is_empty() {
        s.description.clone()
    } else {
        format!("{} {}", s.description, s.size)
    }];
    lines.extend(
        [&s.trap, &s.connection]
            .into_iter()
            .filter(|l| !l.is_empty())
            .map(|l| format!("- {l}")),
    );
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec2;

    #[test]
    fn the_usual_descriptions_are_pilon_designers() {
        let u = default_spec("Umývadlo");
        assert_eq!(u.mark, "U");
        assert_eq!(u.size, "600×460 mm");
        assert_eq!(
            u.connection,
            "rohové ventily 2× G 3/8\" · SV DN 15, TV DN 15, odpad DN 40"
        );
        assert_eq!(default_spec("Umývačka riadu").mark, "UR");
        assert_eq!(default_spec("Kuchynský drez").mark, "D");
        let wc = default_spec("Závesné WC s podomietkovou nádržkou");
        assert_eq!(wc.mark, "WC");
        assert!(wc.description.starts_with("Závesné WC"));
        assert!(default_spec("WC s nádržkou")
            .description
            .starts_with("Stojace"));
        // what has no usual text is named by its family
        let b = default_spec("Bidet");
        assert_eq!((b.mark.as_str(), b.description.as_str()), ("BI", "Bidet"));
        assert_eq!(legend_lines(&u).len(), 3);
        let jika = product_spec(Some("starter/jika-basin-cubito-450"), "Umývadlo").unwrap();
        assert_eq!(
            (jika.mark.as_str(), jika.size.as_str()),
            ("U", "450×250 mm")
        );
        assert!(jika.description.contains("H8114240001061"));
        assert!(product_spec(Some("starter/washbasin"), "Umývadlo").is_none());
    }

    #[test]
    fn the_legend_counts_the_fixtures_placed() {
        let mut doc = creabim_core::Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let was: usize = legend(doc.model(), None)
            .iter()
            .map(|r| r.fixtures.len())
            .sum();
        for (key, x) in [
            ("starter/washbasin", 2.0),
            ("starter/washbasin", 3.0),
            ("starter/pd-zavesne-wc-s-podomietkovou-nadrzkou", 4.0),
        ] {
            let c = creabim_core::catalogue::starter(key).expect(key);
            let (tx, _) =
                creabim_core::objects::place(doc.model(), &c, level, DVec2::new(x, 3.0), 0.0);
            doc.commit(tx).unwrap();
        }
        let rows = legend(doc.model(), None);
        assert_eq!(
            rows.iter().map(|r| r.fixtures.len()).sum::<usize>(),
            was + 3
        );
        let wc = rows
            .iter()
            .find(|r| r.family == "Závesné WC s podomietkovou nádržkou")
            .unwrap();
        assert_eq!(wc.fixtures.len(), 1);
        assert!(rows
            .iter()
            .any(|r| r.spec.mark == "U" && r.fixtures.len() >= 2));
        // heaters are not in it
        assert!(rows.iter().all(|r| fixture::is_sanitary(r.family)));
    }
}
