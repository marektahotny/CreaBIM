//! The building codes a project is designed to: Slovak (STN) first, Czech
//! (ČSN) where the project is in the Czech Republic. Each discipline of the
//! building services lists the standards its calculations follow, so the
//! user can see what a result rests on, and the calculations can ask which
//! set is in force.
//!
//! The lists name the standards; their current editions are checked
//! against ÚNMS SR (STN) and ÚNMZ/AGOSZ (ČSN) before a calculation is
//! built on them.

use serde::{Deserialize, Serialize};

/// Which national codes the project follows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NormSet {
    /// Slovak technical standards.
    #[default]
    Stn,
    /// Czech technical standards.
    Csn,
}

impl NormSet {
    pub const ALL: [NormSet; 2] = [NormSet::Stn, NormSet::Csn];

    pub fn label(self) -> &'static str {
        match self {
            NormSet::Stn => "STN (Slovakia)",
            NormSet::Csn => "ČSN (Czech Republic)",
        }
    }

    pub fn is_default(&self) -> bool {
        *self == NormSet::default()
    }
}

/// A discipline of the project: the architecture and the building
/// services, each with its tab of tools.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Discipline {
    Cadastre,
    Water,
    Drainage,
    Gas,
    RadiatorHeating,
    FloorHeating,
    WallHeating,
    CeilingHeating,
    Ventilation,
    MedicalGases,
    Electrical,
}

impl Discipline {
    pub const ALL: [Discipline; 11] = [
        Discipline::Cadastre,
        Discipline::Water,
        Discipline::Drainage,
        Discipline::Gas,
        Discipline::RadiatorHeating,
        Discipline::FloorHeating,
        Discipline::WallHeating,
        Discipline::CeilingHeating,
        Discipline::Ventilation,
        Discipline::MedicalGases,
        Discipline::Electrical,
    ];

    /// Its id in actions and files.
    pub fn id(self) -> &'static str {
        match self {
            Discipline::Cadastre => "cadastre",
            Discipline::Water => "water",
            Discipline::Drainage => "drainage",
            Discipline::Gas => "gas",
            Discipline::RadiatorHeating => "heating.radiators",
            Discipline::FloorHeating => "heating.floor",
            Discipline::WallHeating => "heating.wall",
            Discipline::CeilingHeating => "heating.ceiling",
            Discipline::Ventilation => "ventilation",
            Discipline::MedicalGases => "medgas",
            Discipline::Electrical => "electrical",
        }
    }

    pub fn from_id(id: &str) -> Option<Discipline> {
        Discipline::ALL.into_iter().find(|d| d.id() == id)
    }

    pub fn label(self) -> &'static str {
        match self {
            Discipline::Cadastre => "Cadastre and site",
            Discipline::Water => "Water supply",
            Discipline::Drainage => "Drainage",
            Discipline::Gas => "Gas installation",
            Discipline::RadiatorHeating => "Radiator heating",
            Discipline::FloorHeating => "Underfloor heating",
            Discipline::WallHeating => "Wall heating",
            Discipline::CeilingHeating => "Ceiling heating",
            Discipline::Ventilation => "Ventilation",
            Discipline::MedicalGases => "Medical gases",
            Discipline::Electrical => "Electrical installation",
        }
    }

    /// What the discipline's tools will do, in the order they come.
    pub fn planned(self) -> &'static [&'static str] {
        match self {
            Discipline::Cadastre => &[
                "Cadastral map, orthophoto and terrain (DMR) under the site plan, in S-JTSK",
                "Parcels and their owners' sheet numbers on the site plan",
            ],
            Discipline::Water => &[
                "Cold and hot water pipes drawn in plan and 3D",
                "Pipe sizing (DN) from design flows, and the pressure balance",
                "Hot water circulation: balancing valves and the pump",
            ],
            Discipline::Drainage => &[
                "Waste and soil pipes, stacks and ventilation drawn in plan and 3D",
                "Pipe sizing from discharge units, and slopes",
            ],
            Discipline::Gas => &[
                "Low and medium pressure gas pipes, the main shut-off and the meter",
                "Pipe sizing and the pressure drop to each appliance",
            ],
            Discipline::RadiatorHeating => &[
                "Room heat loss",
                "Radiators sized to each room, pipes, flows and DN",
                "Balancing and the circulation pump",
            ],
            Discipline::FloorHeating => &[
                "Loops, pipe spacing and output per room",
                "Manifolds and loop balancing",
            ],
            Discipline::WallHeating => &["Wall heating panels: layout and output per room"],
            Discipline::CeilingHeating => {
                &["Ceiling heating and cooling panels: layout and output"]
            }
            Discipline::Ventilation => &[
                "Ducts, terminals and units drawn in plan and 3D",
                "Air volumes per room, pressure losses and balancing",
            ],
            Discipline::MedicalGases => &[
                "O2, medical air, vacuum, N2O and CO2 pipes and terminal units",
                "Flows by diversity, pipe sizing and pressure drop",
            ],
            Discipline::Electrical => &[
                "Sockets, switches, lights and boards on their circuits",
                "Circuit loads, cable sizes and protective devices",
                "Lightning protection",
            ],
        }
    }

    /// The standards the discipline's calculations follow in `set`.
    pub fn norms(self, set: NormSet) -> &'static [Norm] {
        use NormSet::{Csn, Stn};
        match (self, set) {
            (Discipline::Cadastre, Stn) => &[
                Norm {
                    code: "S-JTSK (EPSG:5514)",
                    title: "Coordinate system of the cadastral maps",
                },
                Norm {
                    code: "Act 162/1995",
                    title: "Cadastre of real estate",
                },
                Norm {
                    code: "Act 25/2025",
                    title: "Building act",
                },
            ],
            (Discipline::Cadastre, Csn) => &[
                Norm {
                    code: "S-JTSK (EPSG:5514)",
                    title: "Coordinate system of the cadastral maps",
                },
                Norm {
                    code: "Act 256/2013",
                    title: "Cadastre of real estate",
                },
                Norm {
                    code: "Act 283/2021",
                    title: "Building act",
                },
            ],
            (Discipline::Water, Stn) => &[
                Norm {
                    code: "STN EN 806-3",
                    title: "Drinking water installations: pipe sizing",
                },
                Norm {
                    code: "STN 73 6655",
                    title: "Calculation of water supply inside buildings",
                },
                Norm {
                    code: "DIN 1988-300",
                    title: "Pipe sizing (simplified method)",
                },
            ],
            (Discipline::Water, Csn) => &[
                Norm {
                    code: "ČSN EN 806-3",
                    title: "Drinking water installations: pipe sizing",
                },
                Norm {
                    code: "ČSN 75 5455",
                    title: "Calculation of water supply inside buildings",
                },
                Norm {
                    code: "ČSN 73 6660",
                    title: "Water supply inside buildings",
                },
            ],
            (Discipline::Drainage, Stn) => &[
                Norm {
                    code: "STN EN 12056-2",
                    title: "Gravity drainage inside buildings: sanitary pipework",
                },
                Norm {
                    code: "STN 73 6760",
                    title: "Drainage inside buildings",
                },
            ],
            (Discipline::Drainage, Csn) => &[
                Norm {
                    code: "ČSN EN 12056-2",
                    title: "Gravity drainage inside buildings: sanitary pipework",
                },
                Norm {
                    code: "ČSN 75 6760",
                    title: "Drainage inside buildings",
                },
            ],
            (Discipline::Gas, Stn) => &[
                Norm {
                    code: "TPP 704 01",
                    title: "Gas pipes inside buildings",
                },
                Norm {
                    code: "TPP 702 01",
                    title: "Gas service pipes and connections",
                },
                Norm {
                    code: "STN EN 1775",
                    title: "Gas pipework for buildings",
                },
                Norm {
                    code: "STN EN 12007-2",
                    title: "Gas networks up to 10 bar: polyethylene",
                },
            ],
            (Discipline::Gas, Csn) => &[
                Norm {
                    code: "TPG 704 01",
                    title: "Gas pipes inside buildings",
                },
                Norm {
                    code: "TPG 702 01",
                    title: "Gas service pipes and connections",
                },
                Norm {
                    code: "ČSN EN 1775",
                    title: "Gas pipework for buildings",
                },
            ],
            (Discipline::RadiatorHeating, Stn) => &[
                Norm {
                    code: "STN EN 12831-1",
                    title: "Design heat load",
                },
                Norm {
                    code: "STN EN 12828",
                    title: "Design of water-based heating systems",
                },
                Norm {
                    code: "STN 73 0540",
                    title: "Thermal protection of buildings",
                },
            ],
            (Discipline::RadiatorHeating, Csn) => &[
                Norm {
                    code: "ČSN EN 12831-1",
                    title: "Design heat load",
                },
                Norm {
                    code: "ČSN EN 12828",
                    title: "Design of water-based heating systems",
                },
                Norm {
                    code: "ČSN 06 0310",
                    title: "Heating systems in buildings: design and installation",
                },
                Norm {
                    code: "ČSN 73 0540",
                    title: "Thermal protection of buildings",
                },
            ],
            (
                Discipline::FloorHeating | Discipline::WallHeating | Discipline::CeilingHeating,
                Stn,
            ) => &[
                Norm {
                    code: "STN EN 1264",
                    title: "Embedded water-based surface heating and cooling",
                },
                Norm {
                    code: "STN EN 12831-1",
                    title: "Design heat load",
                },
            ],
            (
                Discipline::FloorHeating | Discipline::WallHeating | Discipline::CeilingHeating,
                Csn,
            ) => &[
                Norm {
                    code: "ČSN EN 1264",
                    title: "Embedded water-based surface heating and cooling",
                },
                Norm {
                    code: "ČSN EN 12831-1",
                    title: "Design heat load",
                },
            ],
            (Discipline::Ventilation, Stn) => &[
                Norm {
                    code: "STN EN 16798-3",
                    title: "Ventilation of non-residential buildings",
                },
                Norm {
                    code: "STN EN 1505 / 1506",
                    title: "Rectangular and circular ducts",
                },
                Norm {
                    code: "Decree 259/2008",
                    title: "Indoor environment of buildings",
                },
            ],
            (Discipline::Ventilation, Csn) => &[
                Norm {
                    code: "ČSN EN 16798-3",
                    title: "Ventilation of non-residential buildings",
                },
                Norm {
                    code: "ČSN EN 1505 / 1506",
                    title: "Rectangular and circular ducts",
                },
                Norm {
                    code: "Decree 20/2012",
                    title: "Technical requirements for buildings",
                },
            ],
            (Discipline::MedicalGases, Stn) => &[
                Norm {
                    code: "STN EN ISO 7396-1",
                    title: "Medical gas pipeline systems",
                },
                Norm {
                    code: "HTM 02-01",
                    title: "Medical gas pipeline systems (design guidance)",
                },
            ],
            (Discipline::MedicalGases, Csn) => &[
                Norm {
                    code: "ČSN EN ISO 7396-1",
                    title: "Medical gas pipeline systems",
                },
                Norm {
                    code: "HTM 02-01",
                    title: "Medical gas pipeline systems (design guidance)",
                },
            ],
            (Discipline::Electrical, Stn) => &[
                Norm {
                    code: "STN 33 2000",
                    title: "Low-voltage electrical installations (HD 60364)",
                },
                Norm {
                    code: "STN 33 2130",
                    title: "Internal electrical distribution",
                },
                Norm {
                    code: "STN EN 62305",
                    title: "Protection against lightning",
                },
                Norm {
                    code: "STN EN 12464-1",
                    title: "Lighting of indoor work places",
                },
            ],
            (Discipline::Electrical, Csn) => &[
                Norm {
                    code: "ČSN 33 2000",
                    title: "Low-voltage electrical installations (HD 60364)",
                },
                Norm {
                    code: "ČSN 33 2130",
                    title: "Internal electrical distribution",
                },
                Norm {
                    code: "ČSN EN 62305",
                    title: "Protection against lightning",
                },
                Norm {
                    code: "ČSN EN 12464-1",
                    title: "Lighting of indoor work places",
                },
            ],
        }
    }
}

/// A standard or regulation: its number and what it covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Norm {
    pub code: &'static str,
    pub title: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_discipline_has_norms_and_plans_in_both_sets() {
        for d in Discipline::ALL {
            assert!(!d.planned().is_empty(), "{d:?}");
            for set in NormSet::ALL {
                assert!(!d.norms(set).is_empty(), "{d:?} {set:?}");
            }
            assert_eq!(Discipline::from_id(d.id()), Some(d));
        }
    }

    #[test]
    fn czech_lists_name_czech_standards() {
        for d in Discipline::ALL {
            assert!(
                d.norms(NormSet::Csn)
                    .iter()
                    .all(|n| !n.code.starts_with("STN")),
                "{d:?}"
            );
            assert!(
                d.norms(NormSet::Stn)
                    .iter()
                    .all(|n| !n.code.starts_with("ČSN")),
                "{d:?}"
            );
        }
    }

    #[test]
    fn the_set_is_stored_by_name_and_slovak_by_default() {
        assert_eq!(NormSet::default(), NormSet::Stn);
        assert_eq!(serde_json::to_string(&NormSet::Csn).unwrap(), "\"csn\"");
    }
}
