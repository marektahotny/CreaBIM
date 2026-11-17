//! Sanitary fixtures by name: what each draws (loading units, qA), the
//! pressure it needs and what it discharges, as the PilonHydraulika
//! settings give them. A fixture is matched by keywords in its name,
//! accents and case aside.

use serde::{Deserialize, Serialize};

/// One kind of fixture and its values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FixtureRule {
    /// Its name, in Slovak as the settings write it.
    #[serde(rename = "Nazov")]
    pub name: String,
    /// Words in a family or type name that make a fixture this one.
    #[serde(rename = "KlucoveSlova")]
    pub keywords: Vec<String>,
    /// Loading units, STN EN 806-3 (qA = LU · 0.1 l/s).
    #[serde(rename = "LU")]
    pub lu: f64,
    /// qA by STN 73 6655, l/s.
    #[serde(rename = "QA_STN736655")]
    pub qa_stn: f64,
    /// Discharge units, STN EN 12056-2.
    #[serde(rename = "DU")]
    pub du: f64,
    /// The least flow pressure it needs, kPa.
    #[serde(rename = "MinTlak_kPa")]
    pub min_pressure_kpa: f64,
    /// Its waste pipe, DN.
    #[serde(rename = "OdpadDN")]
    pub drain_dn: f64,
    #[serde(rename = "JeWC")]
    pub is_wc: bool,
    /// Simultaneity φ, STN 73 6655 outside dwellings.
    #[serde(rename = "Fi")]
    pub phi: f64,
}

impl Default for FixtureRule {
    fn default() -> Self {
        FixtureRule {
            name: String::new(),
            keywords: vec![],
            lu: 0.0,
            qa_stn: 0.0,
            du: 0.0,
            min_pressure_kpa: 100.0,
            drain_dn: 50.0,
            is_wc: false,
            phi: 1.0,
        }
    }
}

/// A fixture as the calculation sees it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FixtureData {
    pub rule: String,
    /// Found by its name; false when the unknown fixture's values stand in.
    pub matched: bool,
    pub lu: f64,
    pub qa_stn: f64,
    pub du: f64,
    pub min_pressure_kpa: f64,
    pub drain_dn: f64,
    pub is_wc: bool,
    pub phi: f64,
    /// Its own connection, DN; no pipe to it is sized smaller.
    pub connector_nominal_mm: f64,
}

impl FixtureData {
    /// qA by STN EN 806-3, l/s.
    pub fn qa_ls(&self) -> f64 {
        self.lu * 0.1
    }

    pub fn from_rule(r: &FixtureRule, matched: bool) -> FixtureData {
        FixtureData {
            rule: r.name.clone(),
            matched,
            lu: r.lu,
            qa_stn: r.qa_stn,
            du: r.du,
            min_pressure_kpa: r.min_pressure_kpa,
            drain_dn: r.drain_dn,
            is_wc: r.is_wc,
            phi: r.phi,
            connector_nominal_mm: 0.0,
        }
    }
}

/// Lower case without accents, as names are compared.
pub fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'ä' | 'à' | 'â' | 'ã' | 'å' => 'a',
            'č' | 'ç' | 'ć' => 'c',
            'ď' => 'd',
            'é' | 'ě' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ĺ' | 'ľ' | 'ł' => 'l',
            'ň' | 'ñ' | 'ń' => 'n',
            'ó' | 'ô' | 'ö' | 'ò' | 'õ' | 'ő' => 'o',
            'ŕ' | 'ř' => 'r',
            'š' | 'ś' => 's',
            'ť' => 't',
            'ú' | 'ů' | 'ü' | 'ù' | 'û' | 'ű' => 'u',
            'ý' | 'ÿ' => 'y',
            'ž' | 'ź' | 'ż' => 'z',
            c => c,
        })
        .collect()
}

/// The first rule whose keyword is in `family` + `type_name`, else the
/// unknown fixture's.
pub fn match_fixture(
    rules: &[FixtureRule],
    unknown: &FixtureRule,
    family: &str,
    type_name: &str,
) -> FixtureData {
    let text = normalize(&format!("{family} {type_name}"));
    rules
        .iter()
        .find(|r| {
            r.keywords
                .iter()
                .any(|k| !k.trim().is_empty() && text.contains(&normalize(k)))
        })
        .map(|r| FixtureData::from_rule(r, true))
        .unwrap_or_else(|| FixtureData::from_rule(unknown, false))
}

#[allow(clippy::too_many_arguments)]
fn rule(
    name: &str,
    phi: f64,
    keywords: &[&str],
    lu: f64,
    qa_stn: f64,
    du: f64,
    min_kpa: f64,
    drain: f64,
    wc: bool,
) -> FixtureRule {
    FixtureRule {
        name: name.into(),
        keywords: keywords.iter().map(|k| k.to_string()).collect(),
        lu,
        qa_stn,
        du,
        min_pressure_kpa: min_kpa,
        drain_dn: drain,
        is_wc: wc,
        phi,
    }
}

/// The fixtures PilonHydraulika knows, in the order they are matched.
pub fn default_rules() -> Vec<FixtureRule> {
    vec![
        rule(
            "Umývačka riadu",
            0.2,
            &["umyvack", "dishwash", "mycka"],
            2.0,
            0.15,
            0.8,
            100.0,
            50.0,
            false,
        ),
        rule(
            "Práčka",
            0.2,
            &["pracka", "prack", "washing machine", "washer", "pracky"],
            2.0,
            0.2,
            0.8,
            100.0,
            50.0,
            false,
        ),
        rule(
            "Výlevka",
            0.2,
            &["vylevk", "slop"],
            2.0,
            0.3,
            2.0,
            100.0,
            100.0,
            true,
        ),
        rule(
            "Pisoár",
            0.3,
            &["pisoar", "urinal"],
            3.0,
            0.1,
            0.5,
            100.0,
            50.0,
            false,
        ),
        rule(
            "WC s nádržkou",
            0.3,
            &["wc", "zachod", "toilet", "closet", "klozet"],
            1.0,
            0.1,
            2.0,
            50.0,
            100.0,
            true,
        ),
        rule("Bidet", 0.3, &["bidet"], 1.0, 0.1, 0.5, 100.0, 40.0, false),
        rule(
            "Sprcha",
            1.0,
            &["sprch", "shower"],
            2.0,
            0.2,
            0.6,
            100.0,
            50.0,
            false,
        ),
        rule(
            "Vaňa",
            0.5,
            &["vana", "bath", "tub"],
            4.0,
            0.3,
            0.8,
            100.0,
            50.0,
            false,
        ),
        rule(
            "Kuchynský drez",
            0.3,
            &["drez", "kuchyn", "kitchen", "sink"],
            2.0,
            0.2,
            0.8,
            100.0,
            50.0,
            false,
        ),
        rule(
            "Umývadlo",
            0.8,
            &["umyvad", "umyvatk", "basin", "lavator", "washbasin"],
            1.0,
            0.2,
            0.5,
            100.0,
            40.0,
            false,
        ),
        rule(
            "Výtokový ventil (záhrada, garáž)",
            0.2,
            &["vytok", "zahrad", "garden", "hose", "kohut"],
            5.0,
            0.4,
            0.0,
            100.0,
            0.0,
            false,
        ),
        rule(
            "Podlahová vpusť",
            0.0,
            &["vpust", "floor drain", "gully", "podlahov"],
            0.0,
            0.0,
            0.8,
            0.0,
            50.0,
            false,
        ),
    ]
}

/// The unknown fixture's values.
pub fn unknown_rule() -> FixtureRule {
    rule(
        "Neznámy predmet (predvolené)",
        0.5,
        &[],
        1.0,
        0.2,
        0.5,
        100.0,
        50.0,
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixtures_are_found_by_their_names_accents_aside() {
        let rules = default_rules();
        let u = unknown_rule();
        assert_eq!(
            match_fixture(&rules, &u, "Umývadlo 600", "").rule,
            "Umývadlo"
        );
        assert_eq!(
            match_fixture(&rules, &u, "Washbasin 600", "").rule,
            "Umývadlo"
        );
        assert_eq!(match_fixture(&rules, &u, "Vaňa 1700", "").lu, 4.0);
        let x = match_fixture(&rules, &u, "Something", "");
        assert!(!x.matched);
        assert_eq!(x.lu, 1.0);
    }
}
