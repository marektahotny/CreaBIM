//! Utility networks in the site plan, after the map symbols George sent
//! (2026-11-20, tables 6.120 to 6.732 and 8.021 of the map symbol norm,
//! and a table of colours and codes): every network is a dashed line with
//! its symbol set into it again and again, one colour for the whole line
//! and its symbols, each network a colour of its own. A proposed network
//! or connection is drawn heavy, an existing one thin, a cancelled one
//! thin and crossed. Lengths are paper millimetres, the same at every
//! scale.

use serde::{Deserialize, Serialize};

/// A kind of utility network.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetKind {
    /// 6.162 drinking water.
    #[default]
    Water,
    /// 6.272 foul sewer.
    FoulSewer,
    /// 6.262 storm sewer.
    StormSewer,
    /// 6.242 combined sewer.
    CombinedSewer,
    /// 6.342 gas, medium pressure.
    GasMedium,
    /// 6.332 gas, low pressure.
    GasLow,
    /// 6.602 power, low voltage.
    PowerLow,
    /// 6.612 power, high voltage.
    PowerHigh,
    /// Street lighting: a low voltage line (6.602) of its own colour.
    StreetLighting,
    /// 6.702 telecommunication line.
    Telecom,
    /// 6.522 primary hot water heating.
    DistrictHeating,
    /// Drainage: a sewer without telling its kind (6.232).
    Drainage,
    /// 6.152 water, of no stated kind.
    WaterAny,
    /// 6.172 utility water.
    WaterUtility,
    /// 6.232 sewer, of no stated kind.
    SewerAny,
    /// 6.252 relief sewer of a combined system.
    SewerRelief,
    /// 6.282 industrial waste water.
    SewerIndustrial,
    /// 6.292 sludge pressure pipe.
    SludgePressure,
    /// 6.322 gas, pressure not stated.
    GasAny,
    /// 6.352 gas, high pressure.
    GasHigh,
    /// 6.362 technical gas (oxygen, acetylene…).
    TechnicalGas,
    /// 6.372 compressed air.
    CompressedAir,
    /// 6.512 heat pipe, of no stated kind.
    HeatAny,
    /// 6.532 primary high temperature hot water.
    HeatHotWater,
    /// 6.542 primary steam.
    HeatSteam,
    /// 6.552 secondary heat distribution.
    HeatSecondary,
    /// 6.592 power, kind not stated.
    PowerAny,
    /// 6.622 power, very high voltage.
    PowerVeryHigh,
    /// 6.712 fire alarm line.
    TelecomFire,
    /// 6.722 clock line.
    TelecomClock,
    /// 6.732 aerial distribution.
    TelecomAerial,
    /// 8.021 watercourse.
    Watercourse,
}

/// The families the colours go by (George's table): every network of a
/// family has its colour, no two families share one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetFamily {
    Water,
    Foul,
    Storm,
    OtherSewer,
    Drainage,
    Gas,
    OtherGas,
    Power,
    Lighting,
    Telecom,
    Heat,
    Watercourse,
}

impl NetFamily {
    /// sRGB 0..1. Where the table gives two, the one no other family has:
    /// water blue, foul brown, storm green, the other sewers violet,
    /// drainage grey, gas yellow (a deep one, to read on paper), power
    /// red, street lighting burgundy, telecom orange, heat pink.
    pub fn rgb(self) -> [f32; 3] {
        let hex: u32 = match self {
            NetFamily::Water => 0x1d4ed8,
            NetFamily::Foul => 0x8b4513,
            NetFamily::Storm => 0x15803d,
            NetFamily::OtherSewer => 0x7e22ce,
            NetFamily::Drainage => 0x6b7280,
            NetFamily::Gas => 0xd4a000,
            NetFamily::OtherGas => 0x334155,
            NetFamily::Power => 0xdc2626,
            NetFamily::Lighting => 0x7f1d1d,
            NetFamily::Telecom => 0xf97316,
            NetFamily::Heat => 0xdb2777,
            NetFamily::Watercourse => 0x0891b2,
        };
        [
            ((hex >> 16) & 255) as f32 / 255.0,
            ((hex >> 8) & 255) as f32 / 255.0,
            (hex & 255) as f32 / 255.0,
        ]
    }
}

impl NetKind {
    pub const ALL: [NetKind; 32] = [
        NetKind::WaterAny,
        NetKind::Water,
        NetKind::WaterUtility,
        NetKind::SewerAny,
        NetKind::CombinedSewer,
        NetKind::SewerRelief,
        NetKind::StormSewer,
        NetKind::FoulSewer,
        NetKind::SewerIndustrial,
        NetKind::SludgePressure,
        NetKind::Drainage,
        NetKind::GasAny,
        NetKind::GasLow,
        NetKind::GasMedium,
        NetKind::GasHigh,
        NetKind::TechnicalGas,
        NetKind::CompressedAir,
        NetKind::HeatAny,
        NetKind::DistrictHeating,
        NetKind::HeatHotWater,
        NetKind::HeatSteam,
        NetKind::HeatSecondary,
        NetKind::PowerAny,
        NetKind::PowerLow,
        NetKind::PowerHigh,
        NetKind::PowerVeryHigh,
        NetKind::StreetLighting,
        NetKind::Telecom,
        NetKind::TelecomFire,
        NetKind::TelecomClock,
        NetKind::TelecomAerial,
        NetKind::Watercourse,
    ];

    pub fn label(self) -> &'static str {
        match self {
            NetKind::Water => "Drinking water main",
            NetKind::FoulSewer => "Foul sewer",
            NetKind::StormSewer => "Storm sewer",
            NetKind::CombinedSewer => "Combined sewer",
            NetKind::GasMedium => "Gas main, medium pressure",
            NetKind::GasLow => "Gas main, low pressure",
            NetKind::PowerLow => "Low voltage power line",
            NetKind::PowerHigh => "High voltage power line",
            NetKind::StreetLighting => "Street lighting",
            NetKind::Telecom => "Telecommunication line",
            NetKind::DistrictHeating => "Primary hot water heating",
            NetKind::Drainage => "Drainage pipe",
            NetKind::WaterAny => "Water main",
            NetKind::WaterUtility => "Utility water main",
            NetKind::SewerAny => "Sewer",
            NetKind::SewerRelief => "Relief sewer of a combined system",
            NetKind::SewerIndustrial => "Industrial waste water sewer",
            NetKind::SludgePressure => "Sludge pressure pipe",
            NetKind::GasAny => "Gas main",
            NetKind::GasHigh => "Gas main, high pressure",
            NetKind::TechnicalGas => "Technical gas pipe",
            NetKind::CompressedAir => "Compressed air pipe",
            NetKind::HeatAny => "Heat pipe",
            NetKind::HeatHotWater => "Primary high temperature hot water",
            NetKind::HeatSteam => "Primary steam pipe",
            NetKind::HeatSecondary => "Secondary heat distribution",
            NetKind::PowerAny => "Power line",
            NetKind::PowerVeryHigh => "Very high voltage power line",
            NetKind::TelecomFire => "Fire alarm line",
            NetKind::TelecomClock => "Clock line",
            NetKind::TelecomAerial => "Aerial distribution line",
            NetKind::Watercourse => "Watercourse",
        }
    }

    /// Its number in the map symbol tables George sent.
    pub fn norm(self) -> &'static str {
        match self {
            NetKind::WaterAny => "6.152",
            NetKind::Water => "6.162",
            NetKind::WaterUtility => "6.172",
            NetKind::SewerAny | NetKind::Drainage => "6.232",
            NetKind::CombinedSewer => "6.242",
            NetKind::SewerRelief => "6.252",
            NetKind::StormSewer => "6.262",
            NetKind::FoulSewer => "6.272",
            NetKind::SewerIndustrial => "6.282",
            NetKind::SludgePressure => "6.292",
            NetKind::GasAny => "6.322",
            NetKind::GasLow => "6.332",
            NetKind::GasMedium => "6.342",
            NetKind::GasHigh => "6.352",
            NetKind::TechnicalGas => "6.362",
            NetKind::CompressedAir => "6.372",
            NetKind::HeatAny => "6.512",
            NetKind::DistrictHeating => "6.522",
            NetKind::HeatHotWater => "6.532",
            NetKind::HeatSteam => "6.542",
            NetKind::HeatSecondary => "6.552",
            NetKind::PowerAny => "6.592",
            NetKind::PowerLow | NetKind::StreetLighting => "6.602",
            NetKind::PowerHigh => "6.612",
            NetKind::PowerVeryHigh => "6.622",
            NetKind::Telecom => "6.702",
            NetKind::TelecomFire => "6.712",
            NetKind::TelecomClock => "6.722",
            NetKind::TelecomAerial => "6.732",
            NetKind::Watercourse => "8.021",
        }
    }

    pub fn family(self) -> NetFamily {
        use NetKind::*;
        match self {
            Water | WaterAny | WaterUtility => NetFamily::Water,
            FoulSewer => NetFamily::Foul,
            StormSewer => NetFamily::Storm,
            CombinedSewer | SewerAny | SewerRelief | SewerIndustrial | SludgePressure => {
                NetFamily::OtherSewer
            }
            Drainage => NetFamily::Drainage,
            GasAny | GasLow | GasMedium | GasHigh => NetFamily::Gas,
            TechnicalGas | CompressedAir => NetFamily::OtherGas,
            PowerAny | PowerLow | PowerHigh | PowerVeryHigh => NetFamily::Power,
            StreetLighting => NetFamily::Lighting,
            Telecom | TelecomFire | TelecomClock | TelecomAerial => NetFamily::Telecom,
            HeatAny | DistrictHeating | HeatHotWater | HeatSteam | HeatSecondary => NetFamily::Heat,
            Watercourse => NetFamily::Watercourse,
        }
    }

    /// The code written into the line (George's table: V, Ks, Kd, P, E,
    /// VO, T, TV).
    pub fn code(self) -> &'static str {
        use NetKind::*;
        match self {
            Water | WaterAny => "V",
            WaterUtility => "VÚ",
            FoulSewer => "Ks",
            StormSewer => "Kd",
            CombinedSewer => "Kj",
            SewerAny => "K",
            SewerRelief => "Ko",
            SewerIndustrial => "Kp",
            SludgePressure => "Kv",
            Drainage => "DR",
            GasAny => "P",
            GasLow => "P NTL",
            GasMedium => "P STL",
            GasHigh => "P VTL",
            TechnicalGas => "TP",
            CompressedAir => "SV",
            PowerAny => "E",
            PowerLow => "E NN",
            PowerHigh => "E VN",
            PowerVeryHigh => "E VVN",
            StreetLighting => "VO",
            Telecom => "T",
            TelecomFire => "T PO",
            TelecomClock => "T H",
            TelecomAerial => "T A",
            HeatAny | DistrictHeating | HeatHotWater | HeatSteam | HeatSecondary => "TV",
            Watercourse => "",
        }
    }

    /// The kind an object's plan symbol names by its code (the codes of
    /// older files too: "K" a foul sewer, "D" a storm one).
    pub fn of_code(code: &str) -> Option<NetKind> {
        match code {
            "V" => Some(NetKind::Water),
            "K" => Some(NetKind::FoulSewer),
            "D" => Some(NetKind::StormSewer),
            "P" => Some(NetKind::GasLow),
            "E" => Some(NetKind::PowerLow),
            "T" => Some(NetKind::Telecom),
            c => NetKind::ALL.into_iter().find(|k| k.code() == c),
        }
    }

    /// Its colour, 0..1: its family's.
    pub fn rgb(self) -> [f32; 3] {
        self.family().rgb()
    }

    /// The dashes of its line between the symbols, paper mm.
    pub fn pattern(self) -> Vec<f64> {
        vec![5.0, 1.5]
    }

    /// Gravity sewers flow the way they are drawn; their symbol shows it.
    pub fn flows(self) -> bool {
        matches!(
            self.family(),
            NetFamily::Foul | NetFamily::Storm | NetFamily::OtherSewer
        )
    }

    /// The symbol set into its line: its strokes along +x from 0 to its
    /// length, +y to the left, paper mm, the line itself drawn by them
    /// where they cover it (6.120–6.732, 8.021 as George sent them).
    pub fn symbol(self) -> LineSymbol {
        use Prim::*;
        let seg = |a: (f64, f64), b: (f64, f64)| Seg([a.0, a.1], [b.0, b.1]);
        // an arrowhead with its tip at x, `back` long, `half` wide
        let head = |x: f64, back: f64, half: f64| {
            vec![
                seg((x - back, half), (x, 0.0)),
                seg((x - back, -half), (x, 0.0)),
            ]
        };
        // the sewers' bracket, its hollow facing back
        let bracket = |x: f64| Arc {
            c: [x, 0.0],
            r: 1.2,
            from: -90.0,
            to: 90.0,
        };
        let tick = |x: f64, h: f64| seg((x, -h), (x, h));
        let mut p: Vec<Prim> = vec![];
        let len;
        match self {
            NetKind::WaterAny | NetKind::Watercourse => {
                len = 4.0;
                p.push(seg((0.0, 0.0), (len, 0.0)));
                p.extend(head(len, 1.2, 0.6));
            }
            NetKind::Water | NetKind::WaterUtility => {
                len = 5.0;
                p.push(seg((0.0, 0.0), (len, 0.0)));
                if self == NetKind::Water {
                    p.push(Arc {
                        c: [0.8, 0.0],
                        r: 0.8,
                        from: 0.0,
                        to: 180.0,
                    });
                } else {
                    p.push(HalfDisk {
                        c: [0.8, 0.0],
                        r: 0.8,
                    });
                }
                p.extend(head(len, 1.2, 0.6));
            }
            NetKind::SewerAny
            | NetKind::Drainage
            | NetKind::CombinedSewer
            | NetKind::SewerRelief
            | NetKind::StormSewer
            | NetKind::FoulSewer
            | NetKind::SewerIndustrial
            | NetKind::SludgePressure => {
                len = 7.5;
                p.push(bracket(0.0));
                p.push(seg((1.2, 0.0), (len, 0.0)));
                match self {
                    NetKind::CombinedSewer => {
                        p.extend(head(3.3, 1.2, 1.1));
                        p.push(bracket(4.1));
                    }
                    NetKind::SewerRelief => {
                        p.extend(head(3.2, 0.6, 0.55));
                        p.extend(head(5.3, 0.6, 0.55));
                    }
                    NetKind::StormSewer => p.extend(head(3.3, 1.2, 1.1)),
                    NetKind::FoulSewer => p.push(bracket(4.1)),
                    NetKind::SewerIndustrial => {
                        p.push(tick(4.3, 1.1));
                        p.push(seg((3.1, 1.1), (4.3, 1.1)));
                        p.push(seg((3.1, -1.1), (4.3, -1.1)));
                    }
                    NetKind::SludgePressure => {
                        p.extend(head(3.3, 1.2, 1.1));
                        p.extend(head(5.4, 1.2, 1.1));
                    }
                    _ => {}
                }
            }
            NetKind::GasAny | NetKind::GasLow | NetKind::GasMedium | NetKind::GasHigh => {
                len = 5.3;
                let (x0, x1, d) = (1.5, 3.8, -0.74);
                p.push(seg((0.0, 0.0), (x0, 0.0)));
                p.push(seg((x0, 0.0), (x0, d)));
                p.push(seg((x0, d), (x1, d)));
                p.push(seg((x1, d), (x1, 0.0)));
                p.push(seg((x1, 0.0), (len, 0.0)));
                let n = match self {
                    NetKind::GasLow => 1,
                    NetKind::GasMedium => 2,
                    NetKind::GasHigh => 3,
                    _ => 0,
                };
                for i in 1..=n {
                    let x = x0 + (x1 - x0) * i as f64 / (n + 1) as f64;
                    p.push(seg((x, d), (x, 0.0)));
                }
            }
            NetKind::TechnicalGas => {
                len = 5.3;
                p.push(seg((0.0, 0.0), (len, 0.0)));
                p.push(Arc {
                    c: [2.65, 0.0],
                    r: 1.16,
                    from: 0.0,
                    to: 180.0,
                });
            }
            NetKind::CompressedAir => {
                len = 2.3;
                p.push(Arc {
                    c: [1.15, 0.0],
                    r: 1.15,
                    from: 0.0,
                    to: 180.0,
                });
            }
            NetKind::HeatAny
            | NetKind::DistrictHeating
            | NetKind::HeatHotWater
            | NetKind::HeatSteam
            | NetKind::HeatSecondary => {
                len = 5.94;
                let mid = len / 2.0;
                match self {
                    NetKind::HeatSteam => {
                        p.push(Circle {
                            c: [0.63, 0.0],
                            r: 0.63,
                        });
                        p.push(Circle {
                            c: [len - 0.63, 0.0],
                            r: 0.63,
                        });
                        p.push(seg((1.26, 0.0), (len - 1.26, 0.0)));
                    }
                    _ => p.push(seg((0.0, 0.0), (len, 0.0))),
                }
                match self {
                    NetKind::DistrictHeating | NetKind::HeatHotWater => {
                        p.push(seg((1.85, 0.6), (0.0, 0.0)));
                        p.push(seg((1.85, -0.6), (0.0, 0.0)));
                        p.extend(head(len, 1.75, 0.6));
                    }
                    _ => {}
                }
                let ticks: &[f64] = match self {
                    NetKind::HeatHotWater => &[-0.4, 0.4],
                    NetKind::HeatSecondary => &[-0.4, 0.0, 0.4],
                    _ => &[0.0],
                };
                for t in ticks {
                    p.push(tick(mid + t, 0.68));
                }
            }
            NetKind::PowerAny
            | NetKind::PowerLow
            | NetKind::PowerHigh
            | NetKind::PowerVeryHigh
            | NetKind::StreetLighting => {
                len = 5.13;
                p.push(seg((0.0, 0.0), (0.86, 0.0)));
                p.push(seg((0.86, 0.0), (0.86, 0.83)));
                p.push(seg((0.86, 0.83), (2.54, 0.0)));
                p.push(seg((2.54, 0.0), (len, 0.0)));
                let heads = match self {
                    NetKind::PowerLow | NetKind::StreetLighting => 1,
                    NetKind::PowerHigh => 2,
                    NetKind::PowerVeryHigh => 3,
                    _ => 0,
                };
                for i in 0..heads {
                    p.extend(head(len - i as f64 * 0.55, 1.0, 0.6));
                }
            }
            NetKind::Telecom => {
                len = 4.0;
                let pts: Vec<[f64; 2]> = (0..=24)
                    .map(|i| {
                        let t = i as f64 / 24.0;
                        [t * len, 0.4 * (t * std::f64::consts::TAU).sin()]
                    })
                    .collect();
                p.push(Poly(pts));
            }
            NetKind::TelecomFire | NetKind::TelecomClock | NetKind::TelecomAerial => {
                let t = match self {
                    NetKind::TelecomFire => "PO",
                    NetKind::TelecomClock => "H",
                    _ => "A",
                };
                len = 1.0 + 1.6 * t.chars().count() as f64;
                p.push(Text {
                    at: [len / 2.0, 0.0],
                    text: t.to_string(),
                });
            }
        }
        LineSymbol { len, prims: p }
    }
}

/// A stroke of a symbol, paper mm.
#[derive(Clone, Debug, PartialEq)]
pub enum Prim {
    Seg([f64; 2], [f64; 2]),
    Poly(Vec<[f64; 2]>),
    /// An arc from `from` to `to` degrees anticlockwise.
    Arc {
        c: [f64; 2],
        r: f64,
        from: f64,
        to: f64,
    },
    Circle {
        c: [f64; 2],
        r: f64,
    },
    Disk {
        c: [f64; 2],
        r: f64,
    },
    /// The upper half of a disk, filled.
    HalfDisk {
        c: [f64; 2],
        r: f64,
    },
    /// A filled outline.
    Fill(Vec<[f64; 2]>),
    /// Letters centred on `at`, upright along the line.
    Text {
        at: [f64; 2],
        text: String,
    },
}

/// A network's symbol: what it draws over `len` mm of its line.
#[derive(Clone, Debug, PartialEq)]
pub struct LineSymbol {
    pub len: f64,
    pub prims: Vec<Prim>,
}

/// A symbol placed on its own in the site plan (6.120–6.580, and the way
/// into the building and onto the plot): the sketch it is set on gives
/// where it is and which way it faces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SiteMark {
    HydrantAbove,
    HydrantBelow,
    Valve,
    Washout,
    WaterBend,
    SewerManhole,
    SewerVent,
    Gully,
    GullyAlongRoad,
    Overflow,
    GasSniffer,
    LampOnMast,
    LampOnBuilding,
    FestiveOnMast,
    FestiveOnBuilding,
    FestiveOnPlinth,
    /// The way into the building: a filled triangle.
    EntryBuilding,
    /// The way onto the plot: an empty triangle.
    EntryPlot,
}

impl SiteMark {
    pub const ALL: [SiteMark; 18] = [
        SiteMark::HydrantAbove,
        SiteMark::HydrantBelow,
        SiteMark::Valve,
        SiteMark::Washout,
        SiteMark::WaterBend,
        SiteMark::SewerManhole,
        SiteMark::SewerVent,
        SiteMark::Gully,
        SiteMark::GullyAlongRoad,
        SiteMark::Overflow,
        SiteMark::GasSniffer,
        SiteMark::LampOnMast,
        SiteMark::LampOnBuilding,
        SiteMark::FestiveOnMast,
        SiteMark::FestiveOnBuilding,
        SiteMark::FestiveOnPlinth,
        SiteMark::EntryBuilding,
        SiteMark::EntryPlot,
    ];

    pub fn label(self) -> &'static str {
        match self {
            SiteMark::HydrantAbove => "Hydrant above ground",
            SiteMark::HydrantBelow => "Hydrant below ground",
            SiteMark::Valve => "Valve (water, gas)",
            SiteMark::Washout => "Water washout",
            SiteMark::WaterBend => "Main bend point on a water main",
            SiteMark::SewerManhole => "Sewer manhole",
            SiteMark::SewerVent => "Vent shaft on a sewer",
            SiteMark::Gully => "Gully",
            SiteMark::GullyAlongRoad => "Gully, along the road's axis",
            SiteMark::Overflow => "Overflow",
            SiteMark::GasSniffer => "Gas sniffer",
            SiteMark::LampOnMast => "Outdoor lamp on a mast",
            SiteMark::LampOnBuilding => "Outdoor lamp on a building",
            SiteMark::FestiveOnMast => "Festive lighting lamp on a mast",
            SiteMark::FestiveOnBuilding => "Festive lighting lamp on a building",
            SiteMark::FestiveOnPlinth => "Festive lighting lamp on a plinth",
            SiteMark::EntryBuilding => "Entrance to the building",
            SiteMark::EntryPlot => "Entrance to the plot",
        }
    }

    /// Its number in the map symbol tables; the entrances have none.
    pub fn norm(self) -> Option<&'static str> {
        Some(match self {
            SiteMark::HydrantAbove => "6.120",
            SiteMark::HydrantBelow => "6.130",
            SiteMark::Valve => "6.140",
            SiteMark::Washout => "6.181",
            SiteMark::WaterBend => "6.190",
            SiteMark::SewerManhole => "6.200",
            SiteMark::SewerVent => "6.210",
            SiteMark::Gully => "6.300",
            SiteMark::GullyAlongRoad => "6.301",
            SiteMark::Overflow => "6.310",
            SiteMark::GasSniffer => "6.380",
            SiteMark::LampOnMast => "6.560",
            SiteMark::LampOnBuilding => "6.561",
            SiteMark::FestiveOnMast => "6.570",
            SiteMark::FestiveOnBuilding => "6.571",
            SiteMark::FestiveOnPlinth => "6.580",
            SiteMark::EntryBuilding | SiteMark::EntryPlot => return None,
        })
    }

    /// Its colour: the network it belongs to; the entrances black.
    pub fn rgb(self) -> [f32; 3] {
        use SiteMark::*;
        match self {
            HydrantAbove | HydrantBelow | Valve | Washout | WaterBend => NetFamily::Water.rgb(),
            SewerManhole | SewerVent => NetFamily::Foul.rgb(),
            Gully | GullyAlongRoad | Overflow => NetFamily::Storm.rgb(),
            GasSniffer => NetFamily::Gas.rgb(),
            LampOnMast | LampOnBuilding | FestiveOnMast | FestiveOnBuilding | FestiveOnPlinth => {
                NetFamily::Lighting.rgb()
            }
            EntryBuilding | EntryPlot => [0.0, 0.0, 0.0],
        }
    }

    /// Its strokes about its point, paper mm, +y the way it faces.
    pub fn symbol(self) -> Vec<Prim> {
        use Prim::*;
        let seg = |a: (f64, f64), b: (f64, f64)| Seg([a.0, a.1], [b.0, b.1]);
        // the lamps' crossed circle on its stem
        let lamp = |p: &mut Vec<Prim>| {
            p.push(Circle {
                c: [0.0, 0.9],
                r: 0.55,
            });
            let k = 0.55 * std::f64::consts::FRAC_1_SQRT_2;
            p.push(seg((-k, 0.9 - k), (k, 0.9 + k)));
            p.push(seg((-k, 0.9 + k), (k, 0.9 - k)));
            p.push(seg((0.0, 0.35), (0.0, -0.6)));
        };
        let mut p = vec![];
        match self {
            SiteMark::HydrantAbove | SiteMark::HydrantBelow => {
                p.push(Circle {
                    c: [0.0, 0.0],
                    r: 0.5,
                });
                p.push(seg((0.0, 0.5), (0.0, 1.2)));
                p.push(seg((-0.55, 1.2), (0.55, 1.2)));
                if self == SiteMark::HydrantBelow {
                    p.push(seg((-0.35, -0.35), (0.35, 0.35)));
                }
            }
            SiteMark::Valve => p.push(Circle {
                c: [0.0, 0.0],
                r: 0.4,
            }),
            SiteMark::Washout => {
                p.push(seg((0.0, 1.2), (0.0, 0.0)));
                p.push(seg((0.0, 0.0), (1.2, 0.0)));
                p.push(seg((0.8, 0.3), (1.2, 0.0)));
                p.push(seg((0.8, -0.3), (1.2, 0.0)));
            }
            SiteMark::WaterBend => {
                p.push(Fill(vec![[0.0, 0.5], [0.5, 0.0], [0.0, -0.5], [-0.5, 0.0]]))
            }
            SiteMark::SewerManhole => p.push(Circle {
                c: [0.0, 0.0],
                r: 1.0,
            }),
            SiteMark::SewerVent => {
                p.push(Circle {
                    c: [0.0, 0.0],
                    r: 0.8,
                });
                p.push(seg((0.0, -1.2), (0.0, 1.2)));
            }
            SiteMark::Gully | SiteMark::GullyAlongRoad => {
                let (w, h) = (0.42, 0.37);
                p.push(Fill(vec![[-w, -h], [w, -h], [w, h * 0.2], [-w, h * 0.2]]));
                p.push(Poly(vec![[-w, h * 0.2], [-w, h], [w, h], [w, h * 0.2]]));
            }
            SiteMark::Overflow => {
                p.push(Circle {
                    c: [0.0, 0.0],
                    r: 0.8,
                });
                p.push(Poly(vec![[-0.45, 0.5], [0.0, -0.6], [0.45, 0.5]]));
            }
            SiteMark::GasSniffer => {
                p.push(Disk {
                    c: [0.0, 0.25],
                    r: 0.35,
                });
                p.push(seg((-0.8, -0.5), (0.0, 0.25)));
                p.push(seg((0.8, -0.5), (0.0, 0.25)));
            }
            SiteMark::LampOnMast | SiteMark::FestiveOnMast => {
                lamp(&mut p);
                p.push(Disk {
                    c: [0.0, -0.6],
                    r: 0.25,
                });
                if self == SiteMark::FestiveOnMast {
                    p.push(seg((-0.9, 0.9), (0.9, 0.9)));
                }
            }
            SiteMark::LampOnBuilding | SiteMark::FestiveOnBuilding => {
                lamp(&mut p);
                if self == SiteMark::FestiveOnBuilding {
                    p.push(seg((-0.9, 0.9), (0.9, 0.9)));
                }
            }
            SiteMark::FestiveOnPlinth => {
                p.push(Circle {
                    c: [0.0, 0.0],
                    r: 0.55,
                });
                let k = 0.55 * std::f64::consts::FRAC_1_SQRT_2;
                p.push(seg((-k, -k), (k, k)));
                p.push(seg((-k, k), (k, -k)));
                p.push(seg((-0.9, -0.55), (0.9, -0.55)));
            }
            SiteMark::EntryBuilding => p.push(Fill(vec![[-1.2, -1.0], [1.2, -1.0], [0.0, 1.4]])),
            SiteMark::EntryPlot => p.push(Poly(vec![
                [-1.2, -1.0],
                [1.2, -1.0],
                [0.0, 1.4],
                [-1.2, -1.0],
            ])),
        }
        p
    }

    /// The legend's line: "6.200 Sewer manhole".
    pub fn legend(self) -> String {
        match self.norm() {
            Some(n) => format!("{n} {}", crate::words::tr(self.label())),
            None => crate::words::tr(self.label()).to_string(),
        }
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
    /// The dashes between its symbols, paper mm.
    pub fn pattern(&self) -> Vec<f64> {
        self.kind.pattern()
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
        if self.kind.code().is_empty() {
            self.note.trim().to_string()
        } else if self.note.trim().is_empty() {
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
            "{} {} - {}",
            self.kind.norm(),
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
    fn every_kind_has_its_symbol_and_the_families_their_own_colours() {
        for k in NetKind::ALL {
            let s = k.symbol();
            assert!(s.len > 0.5 && !s.prims.is_empty(), "{k:?}");
            assert!(k.norm().starts_with('6') || k.norm().starts_with('8'));
        }
        // no two families share a colour
        let mut fams: Vec<NetFamily> = NetKind::ALL.iter().map(|k| k.family()).collect();
        fams.dedup();
        for (i, a) in fams.iter().enumerate() {
            for b in &fams[i + 1..] {
                if a != b {
                    assert_ne!(a.rgb(), b.rgb(), "{a:?} {b:?}");
                }
            }
        }
        // older files' codes still find their kind
        assert_eq!(NetKind::of_code("K"), Some(NetKind::FoulSewer));
        assert_eq!(NetKind::of_code("V"), Some(NetKind::Water));
        let mut w = UtilityLine {
            kind: NetKind::FoulSewer,
            note: "DN 150".into(),
            ..Default::default()
        };
        assert_eq!(w.text(), "Ks DN 150");
        assert!(w.flows() && w.weight() > 0.4);
        w.state = NetState::Cancelled;
        assert!(!w.flows() && w.crossed());
        assert!(SiteMark::ALL.iter().all(|m| !m.symbol().is_empty()));
    }
}
