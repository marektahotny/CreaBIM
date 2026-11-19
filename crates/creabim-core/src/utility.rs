//! Utility networks in the site plan, as PilonDesigner draws them: each
//! kind of network has its line type, colour and code written into the
//! line (V, K, D, P, E...), as is the custom in Slovak site drawings (STN 01
//! 3462 to 01 3466, maps after ČSN 01 3411). A proposed network is drawn
//! heavy, an existing one thin, a cancelled one thin with crosses; gravity
//! sewers carry arrows the way they flow. Patterns are paper millimetres
//! (dash, gap, ...; a dash of 0 is a dot), the same length at every scale.

use serde::{Deserialize, Serialize};

/// A kind of utility network.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetKind {
    #[default]
    Water,
    FoulSewer,
    StormSewer,
    CombinedSewer,
    GasMedium,
    GasLow,
    PowerLow,
    PowerHigh,
    StreetLighting,
    Telecom,
    DistrictHeating,
    Drainage,
}

impl NetKind {
    pub const ALL: [NetKind; 12] = [
        NetKind::Water,
        NetKind::FoulSewer,
        NetKind::StormSewer,
        NetKind::CombinedSewer,
        NetKind::GasMedium,
        NetKind::GasLow,
        NetKind::PowerLow,
        NetKind::PowerHigh,
        NetKind::StreetLighting,
        NetKind::Telecom,
        NetKind::DistrictHeating,
        NetKind::Drainage,
    ];

    pub fn label(self) -> &'static str {
        match self {
            NetKind::Water => "Water main",
            NetKind::FoulSewer => "Foul sewer",
            NetKind::StormSewer => "Storm sewer",
            NetKind::CombinedSewer => "Combined sewer",
            NetKind::GasMedium => "Gas main, medium pressure",
            NetKind::GasLow => "Gas main, low pressure",
            NetKind::PowerLow => "Low voltage cable, buried",
            NetKind::PowerHigh => "High voltage cable, buried",
            NetKind::StreetLighting => "Street lighting",
            NetKind::Telecom => "Telecommunication line",
            NetKind::DistrictHeating => "District heating",
            NetKind::Drainage => "Drainage pipe",
        }
    }

    /// The code written into the line.
    pub fn code(self) -> &'static str {
        match self {
            NetKind::Water => "V",
            NetKind::FoulSewer => "K",
            NetKind::StormSewer => "D",
            NetKind::CombinedSewer => "J",
            NetKind::GasMedium => "P",
            NetKind::GasLow => "PN",
            NetKind::PowerLow => "E",
            NetKind::PowerHigh => "VN",
            NetKind::StreetLighting => "VO",
            NetKind::Telecom => "T",
            NetKind::DistrictHeating => "TP",
            NetKind::Drainage => "DR",
        }
    }

    /// Its colour, 0..1.
    pub fn rgb(self) -> [f32; 3] {
        let hex: u32 = match self {
            NetKind::Water => 0x1d6fd8,
            NetKind::FoulSewer => 0x7a3b12,
            NetKind::StormSewer => 0x2f7d6d,
            NetKind::CombinedSewer => 0x5b3a1a,
            NetKind::GasMedium => 0x16a34a,
            NetKind::GasLow => 0x65a30d,
            NetKind::PowerLow => 0xdc2626,
            NetKind::PowerHigh => 0xb91c1c,
            NetKind::StreetLighting => 0xea8a00,
            NetKind::Telecom => 0x7c3aed,
            NetKind::DistrictHeating => 0xc026d3,
            NetKind::Drainage => 0x0e9fb5,
        };
        [
            ((hex >> 16) & 255) as f32 / 255.0,
            ((hex >> 8) & 255) as f32 / 255.0,
            (hex & 255) as f32 / 255.0,
        ]
    }

    /// Its line pattern, paper mm.
    pub fn pattern(self) -> Vec<f64> {
        match self {
            NetKind::Water | NetKind::GasMedium | NetKind::GasLow | NetKind::DistrictHeating => {
                vec![8.0, 1.5, 0.0, 1.5]
            }
            NetKind::FoulSewer | NetKind::StormSewer => vec![6.0, 1.5, 0.0, 1.5, 0.0, 1.5],
            NetKind::CombinedSewer => vec![6.0, 1.5],
            NetKind::PowerLow => vec![10.0, 2.0],
            NetKind::PowerHigh => vec![10.0, 1.5, 2.0, 1.5],
            NetKind::StreetLighting => vec![6.0, 2.0],
            NetKind::Telecom => vec![4.0, 1.5],
            NetKind::Drainage => vec![2.0, 1.5],
        }
    }

    /// Gravity sewers: arrows the way they flow (as drawn).
    pub fn flows(self) -> bool {
        matches!(
            self,
            NetKind::FoulSewer | NetKind::StormSewer | NetKind::CombinedSewer
        )
    }
}

/// Whether a network is there already, proposed or to go.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetState {
    /// An existing public network.
    Existing,
    /// An existing connection to the building.
    ExistingConnection,
    #[default]
    Proposed,
    Cancelled,
}

impl NetState {
    pub const ALL: [NetState; 4] = [
        NetState::Existing,
        NetState::ExistingConnection,
        NetState::Proposed,
        NetState::Cancelled,
    ];

    pub fn label(self) -> &'static str {
        match self {
            NetState::Existing => "Existing public network",
            NetState::ExistingConnection => "Existing connection",
            NetState::Proposed => "Proposed network",
            NetState::Cancelled => "Cancelled",
        }
    }
}

/// A sketch drawn as a utility network.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UtilityLine {
    pub kind: NetKind,
    #[serde(default)]
    pub state: NetState,
    /// Written after the code: "DN 32", "PE 40".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

impl UtilityLine {
    /// Its pattern: a connection's a third shorter.
    pub fn pattern(&self) -> Vec<f64> {
        let p = self.kind.pattern();
        if self.state == NetState::ExistingConnection {
            p.into_iter().map(|x| x * 0.6).collect()
        } else {
            p
        }
    }

    /// Its line, paper mm: a proposed one heavy, the rest thin.
    pub fn weight(&self) -> f64 {
        if self.state == NetState::Proposed {
            0.5
        } else {
            0.25
        }
    }

    /// What is written into the line.
    pub fn text(&self) -> String {
        if self.note.trim().is_empty() {
            self.kind.code().to_string()
        } else {
            format!("{} {}", self.kind.code(), self.note.trim())
        }
    }

    pub fn flows(&self) -> bool {
        self.kind.flows() && self.state != NetState::Cancelled
    }

    pub fn crossed(&self) -> bool {
        self.state == NetState::Cancelled
    }

    /// Its layer in a drawing (DXF): one per kind, as PilonDesigner's.
    pub fn layer(&self) -> String {
        format!(
            "{} - {}",
            crate::words::tr("Utility networks"),
            crate::words::tr(self.kind.label())
        )
    }

    /// The legend's line: "Proposed - water main DN 32".
    pub fn legend(&self) -> String {
        let base = format!(
            "{} - {}",
            crate::words::tr(self.state.label()),
            crate::words::tr(self.kind.label()).to_lowercase()
        );
        if self.note.trim().is_empty() {
            base
        } else {
            format!("{base} {}", self.note.trim())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_its_own_code_and_a_pattern() {
        let mut codes: Vec<&str> = NetKind::ALL.iter().map(|k| k.code()).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), NetKind::ALL.len());
        assert!(NetKind::ALL.iter().all(|k| k.pattern().len() >= 2));
        let mut w = UtilityLine {
            kind: NetKind::FoulSewer,
            note: "DN 150".into(),
            ..Default::default()
        };
        assert_eq!(w.text(), "K DN 150");
        assert!(w.flows() && w.weight() > 0.4);
        w.state = NetState::Cancelled;
        assert!(!w.flows() && w.crossed());
        w.state = NetState::ExistingConnection;
        assert!((w.pattern()[0] - 3.6).abs() < 1e-9);
    }
}
