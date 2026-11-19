//! HERZ valves (www.herz.sk, data sheets 2025) and their presettings, as
//! PilonDesigner's `herzvalves.ts`: the kv of each presetting follows
//! HERZ's own programs "Nastavenie STRÖMAX MS a TS-98-V" and
//! "Nastavovanie RTD". A valve on HERZ pipes with no product chosen is
//! taken as HERZ's default for its kind; its loss is (Q / kv)² · 100 kPa.

/// One size of a valve: kvs 0 where HERZ gives none (the kind's ζ then
/// stands); its own presettings where they differ by size.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HerzSize {
    pub dn: u32,
    pub code: &'static str,
    pub kvs: f64,
    pub presets: &'static [(&'static str, f64)],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HerzValve {
    pub id: &'static str,
    /// The valve kind it is (see `creabim_core::mep::VALVE_KINDS`).
    pub kind: &'static str,
    pub name: &'static str,
    /// HERZ catalogue number.
    pub cat: &'static str,
    /// voda, ukur, chlad.
    pub media: &'static [&'static str],
    /// Presettings and their kv, m³/h, for every size.
    pub presets: &'static [(&'static str, f64)],
    pub sizes: &'static [HerzSize],
}

pub const HERZ_VALVES: &[HerzValve] = &[
    HerzValve {
        id: "HZ-GK-PV",
        kind: "uzaver",
        name: "HERZ guľový kohút pre pitnú vodu, zelená páka",
        cat: "2100",
        media: &["voda"],
        presets: &[],
        sizes: &[
            HerzSize {
                dn: 15,
                code: "2 2100 01",
                kvs: 0.0,
                presets: &[],
            },
            HerzSize {
                dn: 20,
                code: "2 2100 02",
                kvs: 0.0,
                presets: &[],
            },
            HerzSize {
                dn: 25,
                code: "2 2100 03",
                kvs: 0.0,
                presets: &[],
            },
            HerzSize {
                dn: 32,
                code: "2 2100 04",
                kvs: 0.0,
                presets: &[],
            },
            HerzSize {
                dn: 40,
                code: "2 2100 05",
                kvs: 0.0,
                presets: &[],
            },
            HerzSize {
                dn: 50,
                code: "2 2100 06",
                kvs: 0.0,
                presets: &[],
            },
        ],
    },
    HerzValve {
        id: "HZ-SPV",
        kind: "spatna",
        name: "HERZ pružinový spätný ventil",
        cat: "2622",
        media: &["voda", "ukur", "chlad"],
        presets: &[],
        sizes: &[
            HerzSize {
                dn: 15,
                code: "1 2622 41",
                kvs: 0.0,
                presets: &[],
            },
            HerzSize {
                dn: 20,
                code: "1 2622 42",
                kvs: 0.0,
                presets: &[],
            },
            HerzSize {
                dn: 25,
                code: "1 2622 43",
                kvs: 0.0,
                presets: &[],
            },
            HerzSize {
                dn: 32,
                code: "1 2622 44",
                kvs: 0.0,
                presets: &[],
            },
            HerzSize {
                dn: 40,
                code: "1 2622 45",
                kvs: 0.0,
                presets: &[],
            },
            HerzSize {
                dn: 50,
                code: "1 2622 46",
                kvs: 0.0,
                presets: &[],
            },
        ],
    },
    HerzValve {
        id: "HZ-FILTER",
        kind: "filter",
        name: "HERZ filter závitový",
        cat: "2662",
        media: &["voda", "ukur", "chlad"],
        presets: &[],
        sizes: &[
            HerzSize {
                dn: 15,
                code: "1 2662 01",
                kvs: 3.1,
                presets: &[],
            },
            HerzSize {
                dn: 20,
                code: "1 2662 02",
                kvs: 6.3,
                presets: &[],
            },
            HerzSize {
                dn: 25,
                code: "1 2662 03",
                kvs: 10.4,
                presets: &[],
            },
            HerzSize {
                dn: 32,
                code: "1 2662 04",
                kvs: 16.5,
                presets: &[],
            },
            HerzSize {
                dn: 40,
                code: "1 2662 05",
                kvs: 27.4,
                presets: &[],
            },
            HerzSize {
                dn: 50,
                code: "1 2662 06",
                kvs: 36.7,
                presets: &[],
            },
        ],
    },
    HerzValve {
        id: "HZ-STROMAX-4217GM",
        kind: "vyvazovaci",
        name: "HERZ STRÖMAX 4217 GM vyvažovací ventil s meracími ventilčekmi",
        cat: "4217",
        media: &["ukur", "chlad"],
        presets: &[],
        sizes: &[
            HerzSize {
                dn: 15,
                code: "1 4217 01",
                kvs: 6.05,
                presets: &[
                    ("2", 1.14),
                    ("2,5", 1.8),
                    ("3", 2.45),
                    ("3,5", 3.63),
                    ("4", 4.81),
                    ("4,5", 5.22),
                    ("5", 5.62),
                    ("5,5", 5.84),
                ],
            },
            HerzSize {
                dn: 20,
                code: "1 4217 32",
                kvs: 6.11,
                presets: &[
                    ("2", 1.05),
                    ("2,5", 1.66),
                    ("3", 2.27),
                    ("3,5", 3.23),
                    ("4", 4.19),
                    ("4,5", 4.76),
                    ("5", 5.32),
                    ("5,5", 5.72),
                ],
            },
            HerzSize {
                dn: 25,
                code: "1 4217 33",
                kvs: 9.22,
                presets: &[
                    ("2", 1.7),
                    ("2,5", 2.34),
                    ("3", 2.99),
                    ("3,5", 3.72),
                    ("4", 4.45),
                    ("4,5", 5.27),
                    ("5", 6.1),
                    ("5,5", 6.75),
                    ("6", 7.41),
                    ("6,5", 7.93),
                    ("7", 8.45),
                    ("7,5", 8.84),
                ],
            },
            HerzSize {
                dn: 32,
                code: "1 4217 34",
                kvs: 18.83,
                presets: &[
                    ("2", 2.48),
                    ("2,5", 3.55),
                    ("3", 4.61),
                    ("3,5", 5.92),
                    ("4", 7.22),
                    ("4,5", 8.95),
                    ("5", 10.68),
                    ("5,5", 12.38),
                    ("6", 14.08),
                    ("6,5", 15.34),
                    ("7", 16.61),
                    ("7,5", 17.11),
                    ("8", 17.6),
                    ("8,5", 18.21),
                ],
            },
            HerzSize {
                dn: 40,
                code: "1 4217 35",
                kvs: 23.29,
                presets: &[
                    ("2", 3.11),
                    ("2,5", 4.28),
                    ("3", 5.45),
                    ("3,5", 7.03),
                    ("4", 8.62),
                    ("4,5", 10.55),
                    ("5", 12.49),
                    ("5,5", 14.34),
                    ("6", 16.2),
                    ("6,5", 17.49),
                    ("7", 18.79),
                    ("7,5", 20.13),
                    ("8", 21.47),
                    ("8,5", 22.38),
                ],
            },
            HerzSize {
                dn: 50,
                code: "1 4217 36",
                kvs: 35.26,
                presets: &[
                    ("2", 4.39),
                    ("2,5", 7.1),
                    ("3", 9.81),
                    ("3,5", 13.61),
                    ("4", 17.41),
                    ("4,5", 21.82),
                    ("5", 26.24),
                    ("5,5", 28.85),
                    ("6", 31.46),
                    ("6,5", 33.36),
                ],
            },
            HerzSize {
                dn: 65,
                code: "1 4217 07",
                kvs: 52.11,
                presets: &[
                    ("2", 15.94),
                    ("2,5", 18.64),
                    ("3", 21.35),
                    ("3,5", 25.29),
                    ("4", 29.23),
                    ("4,5", 34.11),
                    ("5", 38.98),
                    ("5,5", 42.95),
                    ("6", 46.93),
                    ("6,5", 49.51),
                    ("7", 52.1),
                    ("7,5", 52.1),
                ],
            },
            HerzSize {
                dn: 80,
                code: "1 4217 08",
                kvs: 76.1,
                presets: &[
                    ("2", 26.27),
                    ("2,5", 29.42),
                    ("3", 32.56),
                    ("3,5", 37.49),
                    ("4", 42.41),
                    ("4,5", 47.43),
                    ("5", 52.45),
                    ("5,5", 56.58),
                    ("6", 60.7),
                    ("6,5", 64.88),
                    ("7", 69.06),
                    ("7,5", 72.58),
                ],
            },
        ],
    },
    HerzValve {
        id: "HZ-STROMAX-4117MW",
        kind: "vyvazovaci",
        name: "HERZ STRÖMAX 4117 MW vyvažovací ventil pre pitnú vodu (cirkulácia)",
        cat: "4117",
        media: &["voda"],
        presets: &[],
        sizes: &[
            HerzSize {
                dn: 15,
                code: "2 4117 51",
                kvs: 3.96,
                presets: &[],
            },
            HerzSize {
                dn: 20,
                code: "2 4117 52",
                kvs: 5.59,
                presets: &[],
            },
            HerzSize {
                dn: 25,
                code: "2 4117 53",
                kvs: 8.68,
                presets: &[],
            },
            HerzSize {
                dn: 32,
                code: "2 4117 54",
                kvs: 15.97,
                presets: &[],
            },
            HerzSize {
                dn: 40,
                code: "2 4117 55",
                kvs: 23.5,
                presets: &[],
            },
            HerzSize {
                dn: 50,
                code: "2 4117 56",
                kvs: 47.89,
                presets: &[],
            },
        ],
    },
    HerzValve {
        id: "HZ-TS90V",
        kind: "termostaticky",
        name: "HERZ TS-90-V termostatický ventil priamy, skryté prednastavenie",
        cat: "7723",
        media: &["ukur"],
        presets: &[
            ("1", 0.03),
            ("2", 0.05),
            ("3", 0.09),
            ("4", 0.15),
            ("5", 0.2),
            ("6", 0.25),
        ],
        sizes: &[
            HerzSize {
                dn: 10,
                code: "1 7723 65",
                kvs: 1.0,
                presets: &[],
            },
            HerzSize {
                dn: 15,
                code: "1 7723 67",
                kvs: 1.1,
                presets: &[],
            },
            HerzSize {
                dn: 20,
                code: "1 7723 69",
                kvs: 1.1,
                presets: &[],
            },
        ],
    },
    HerzValve {
        id: "HZ-RL5",
        kind: "spiatocka",
        name: "HERZ RL-5 ventil do spiatočky priamy, s prednastavením",
        cat: "3923",
        media: &["ukur"],
        presets: &[],
        sizes: &[
            HerzSize {
                dn: 10,
                code: "1 3923 00",
                kvs: 1.4,
                presets: &[],
            },
            HerzSize {
                dn: 15,
                code: "1 3923 01",
                kvs: 1.5,
                presets: &[],
            },
            HerzSize {
                dn: 20,
                code: "1 3923 02",
                kvs: 1.65,
                presets: &[],
            },
        ],
    },
    HerzValve {
        id: "HZ-RTD-4002",
        kind: "rtd",
        name: "HERZ 4002 regulátor tlakovej diferencie 5–30 kPa",
        cat: "4002",
        media: &["ukur", "chlad"],
        presets: &[],
        sizes: &[
            HerzSize {
                dn: 15,
                code: "1 4002 41",
                kvs: 2.66,
                presets: &[],
            },
            HerzSize {
                dn: 20,
                code: "1 4002 42",
                kvs: 4.36,
                presets: &[],
            },
            HerzSize {
                dn: 25,
                code: "1 4002 43",
                kvs: 5.38,
                presets: &[],
            },
            HerzSize {
                dn: 32,
                code: "1 4002 44",
                kvs: 9.48,
                presets: &[],
            },
            HerzSize {
                dn: 40,
                code: "1 4002 45",
                kvs: 14.95,
                presets: &[],
            },
            HerzSize {
                dn: 50,
                code: "1 4002 46",
                kvs: 14.95,
                presets: &[],
            },
        ],
    },
    HerzValve {
        id: "HZ-STROMAX-MS",
        kind: "vyvazovaci",
        name: "HERZ STRÖMAX MS vyvažovací ventil",
        cat: "4216",
        media: &["ukur", "chlad"],
        presets: &[
            ("1", 0.4),
            ("1,5", 0.43),
            ("2", 0.48),
            ("2,5", 0.59),
            ("3", 0.74),
            ("3,5", 0.98),
            ("4", 1.28),
            ("4,5", 1.58),
            ("5", 1.88),
            ("5,5", 2.18),
            ("6", 2.48),
            ("6,5", 2.72),
            ("7", 2.92),
            ("7,5", 3.09),
            ("8", 3.24),
            ("8,5", 3.33),
            ("9", 3.38),
        ],
        sizes: &[
            HerzSize {
                dn: 15,
                code: "1 4216 31",
                kvs: 3.4,
                presets: &[],
            },
            HerzSize {
                dn: 20,
                code: "1 4216 32",
                kvs: 3.4,
                presets: &[],
            },
        ],
    },
    HerzValve {
        id: "HZ-TS98V",
        kind: "termostaticky",
        name: "HERZ TS-98-V termostatický ventil priamy, plynulé prednastavenie",
        cat: "7217",
        media: &["ukur"],
        presets: &[
            ("1", 0.21),
            ("1,5", 0.252),
            ("2", 0.322),
            ("2,5", 0.389),
            ("3", 0.454),
            ("3,5", 0.513),
            ("4", 0.568),
            ("4,5", 0.623),
            ("5", 0.678),
            ("5,5", 0.715),
            ("6", 0.74),
        ],
        sizes: &[HerzSize {
            dn: 15,
            code: "1 7217 67",
            kvs: 0.75,
            presets: &[],
        }],
    },
    HerzValve {
        id: "HZ-RTD-4007",
        kind: "rtd",
        name: "HERZ 4007 regulátor tlakovej diferencie 5–30 kPa",
        cat: "4007",
        media: &["ukur", "chlad"],
        presets: &[],
        sizes: &[
            HerzSize {
                dn: 15,
                code: "1 4007 01",
                kvs: 4.8,
                presets: &[],
            },
            HerzSize {
                dn: 20,
                code: "1 4007 02",
                kvs: 5.9,
                presets: &[],
            },
            HerzSize {
                dn: 25,
                code: "1 4007 03",
                kvs: 9.5,
                presets: &[],
            },
            HerzSize {
                dn: 32,
                code: "1 4007 04",
                kvs: 13.2,
                presets: &[],
            },
            HerzSize {
                dn: 40,
                code: "1 4007 05",
                kvs: 15.6,
                presets: &[],
            },
            HerzSize {
                dn: 50,
                code: "1 4007 06",
                kvs: 25.2,
                presets: &[],
            },
            HerzSize {
                dn: 65,
                code: "1 4007 07",
                kvs: 26.0,
                presets: &[],
            },
            HerzSize {
                dn: 80,
                code: "1 4007 08",
                kvs: 27.0,
                presets: &[],
            },
        ],
    },
];

/// A product by its id.
pub fn by_id(id: &str) -> Option<&'static HerzValve> {
    HERZ_VALVES.iter().find(|h| h.id == id)
}

/// HERZ's default for a valve kind on a medium: the thermostatic valve
/// with stepless presetting TS-98-V, the STRÖMAX MS balancing valve up
/// to DN 20 (STRÖMAX GM above), else the first of the kind.
pub fn default_for(kind: &str, medium: &str, dn: u32) -> Option<&'static HerzValve> {
    let pick = match (kind, medium) {
        ("termostaticky", "ukur") => Some("HZ-TS98V"),
        ("vyvazovaci", "ukur" | "chlad") if dn <= 20 => Some("HZ-STROMAX-MS"),
        ("vyvazovaci", "ukur" | "chlad") => Some("HZ-STROMAX-4217GM"),
        _ => None,
    };
    pick.and_then(by_id).or_else(|| {
        HERZ_VALVES
            .iter()
            .find(|h| h.kind == kind && h.media.contains(&medium))
    })
}

/// The DN a valve joins a HERZ multilayer pipe of outside diameter
/// `outer` with (HERZ adaptors: 16, 20 → ½", 26 → ¾", 32 → 1"...).
pub fn dn_for(outer_mm: f64, inner_mm: f64) -> u32 {
    const T: [(f64, u32); 10] = [
        (14.0, 10),
        (16.0, 15),
        (18.0, 15),
        (20.0, 15),
        (26.0, 20),
        (32.0, 25),
        (40.0, 32),
        (50.0, 40),
        (63.0, 50),
        (75.0, 65),
    ];
    if let Some(&(_, dn)) = T.iter().find(|(d, _)| (d - outer_mm).abs() < 0.6) {
        return dn;
    }
    [10, 15, 20, 25, 32, 40, 50, 65, 80, 100]
        .into_iter()
        .find(|&d| d as f64 >= inner_mm - 1.0)
        .unwrap_or(100)
}

impl HerzValve {
    /// The size for `dn`: the next larger, else the largest.
    pub fn size_for(&self, dn: u32) -> &'static HerzSize {
        self.sizes
            .iter()
            .find(|s| s.dn >= dn)
            .unwrap_or(&self.sizes[self.sizes.len() - 1])
    }
    /// The presettings of a size.
    pub fn presets_of(&self, s: &HerzSize) -> &'static [(&'static str, f64)] {
        if s.presets.is_empty() {
            self.presets
        } else {
            s.presets
        }
    }
}

/// The loss, kPa, of `q` m³/h through `kv`.
pub fn dp_kpa(q_m3h: f64, kv: f64) -> f64 {
    if kv <= 0.0 {
        0.0
    } else {
        (q_m3h / kv).powi(2) * 100.0
    }
}

/// A presetting chosen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pick {
    pub preset: &'static str,
    pub kv: f64,
    /// The kv the throttling asks for.
    pub kv_req: f64,
    /// The loss at the presetting chosen, kPa.
    pub dp_kpa: f64,
    /// Even the smallest presetting throttles too little.
    pub under: bool,
    /// Wide open: the circuit is the critical one.
    pub open: bool,
}

/// The presetting for `q` m³/h to lose `dp` kPa, as HERZ's programs:
/// kv = Q / √(Δp / 100), the largest kv no more than it (the valve
/// throttles at least what is asked); under the smallest, the smallest.
pub fn pick(presets: &'static [(&'static str, f64)], q_m3h: f64, dp: f64) -> Option<Pick> {
    let kv_req = if dp > 0.0 {
        q_m3h / (dp / 100.0).sqrt()
    } else {
        f64::INFINITY
    };
    let mut sorted: Vec<&(&str, f64)> = presets.iter().collect();
    sorted.sort_by(|a, b| a.1.total_cmp(&b.1));
    let first = **sorted.first()?;
    let last = **sorted.last()?;
    // HERZ's program rounds the kv asked for to hundredths before it
    // looks the presetting up
    let kv_round = (kv_req * 100.0).round() / 100.0;
    let (p, under) = sorted
        .iter()
        .rev()
        .find(|x| x.1 <= kv_round + 1e-9)
        .map_or((first, true), |x| (**x, false));
    Some(Pick {
        preset: p.0,
        kv: p.1,
        kv_req,
        dp_kpa: dp_kpa(q_m3h, p.1),
        under,
        open: kv_req > last.1 * 1.0001,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// HERZ's program "Nastavenie STRÖMAX MS a TS-98-V": 80 kPa and
    /// 660 l/h ask kv 0,74, presetting 3 on the MS and 6 on the TS-98-V.
    #[test]
    fn presettings_come_out_as_herzs_program_gives_them() {
        let ms = by_id("HZ-STROMAX-MS").unwrap();
        let ts = by_id("HZ-TS98V").unwrap();
        assert_eq!(pick(ms.presets, 0.66, 80.0).unwrap().preset, "3");
        assert_eq!(pick(ts.presets, 0.66, 80.0).unwrap().preset, "6");
    }

    #[test]
    fn the_presetting_throttles_at_least_what_is_asked() {
        let ts = by_id("HZ-TS98V").unwrap();
        let s = ts.size_for(15);
        assert_eq!(s.code, "1 7217 67");
        let p = ts.presets_of(s);
        // 100 l/h to lose 10 kPa: kv 0,316, so presetting 1,5 (0,252)
        let k = pick(p, 0.1, 10.0).unwrap();
        assert!((k.kv_req - 0.316).abs() < 0.001);
        assert_eq!(
            (k.preset, k.kv, k.under, k.open),
            ("1,5", 0.252, false, false)
        );
        assert!(k.dp_kpa >= 10.0);
        // nothing to throttle: wide open
        let k = pick(p, 0.1, 0.0).unwrap();
        assert_eq!((k.preset, k.open), ("6", true));
        // too much: the smallest, flagged
        assert!(pick(p, 0.01, 500.0).unwrap().under);
        // a STRÖMAX GM keeps presettings by size
        let gm = by_id("HZ-STROMAX-4217GM").unwrap();
        assert_eq!(gm.presets_of(gm.size_for(25))[0], ("2", 1.7));
        assert_eq!(
            default_for("vyvazovaci", "ukur", 15).unwrap().id,
            "HZ-STROMAX-MS"
        );
        assert_eq!(
            default_for("vyvazovaci", "ukur", 32).unwrap().id,
            "HZ-STROMAX-4217GM"
        );
        assert_eq!(dn_for(16.0, 12.0), 15);
        assert_eq!(dn_for(26.0, 20.0), 20);
    }
}
