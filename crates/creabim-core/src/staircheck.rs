//! Stairs against STN 73 4130 and STN 73 4301 (George 2026-11-24): the
//! step formula, the slope class and its riser heights, the least going
//! and tread, the risers in a flight, the winders' narrow ends, where the
//! walking line of a turning flight lies, and the least width and the
//! steepest slope by the kind of building.
//!
//! The norms' figures are the user's licensed copies, read from a
//! parameter file into the project ([`TABLES`]); without them nothing is
//! judged and nothing is set.

use crate::fireparams::{number, ParamTable};
use crate::members::{riser_count, Stair};

/// Čl. 12 c): the slope classes, by angle (`angle_over`, `angle_upto`,
/// degrees) with their riser heights (`h_min`, `h_max`, mm).
pub const CLASSES: &str = "stair_classes";
/// The step rules (`value`, mm or a count, by `key`): `sum`, `sum_min`,
/// `going_min`, `tread_min`, `narrow_min`, `risers_max`,
/// `risers_max_house`, `landing_extra`.
pub const STEPS: &str = "stair_steps";
/// Čl. 16: the walking line of a turning flight, from its outer edge, by
/// the flight's width up to `width_upto` (mm): `offset` mm, or a `share`
/// of the width.
pub const WALK: &str = "stair_walk";
/// Čl. 22, 23: headroom `a + b / cos α` and clear height `a + b · cos α`
/// with their least (`min`), mm.
pub const HEADROOM: &str = "stair_headroom";
/// The kind of building: least width (`width_min`, mm), steepest slope
/// (`angle_max`, `angle_max_lift`, degrees).
pub const BUILDING: &str = "stair_building";
/// The tables the check reads.
pub const TABLES: [&str; 5] = [CLASSES, STEPS, WALK, HEADROOM, BUILDING];

/// The kind of building the stair is in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Building {
    /// A family house, or inside a flat.
    #[default]
    House,
    /// A block of flats: its main stair.
    Flats,
}

impl Building {
    fn key(self) -> &'static str {
        match self {
            Building::House => "house",
            Building::Flats => "flats",
        }
    }
}

/// A slope class.
#[derive(Clone, Debug, PartialEq)]
pub struct Class {
    pub name: String,
    pub angle: (f64, f64),
    pub riser: (f64, f64),
}

/// The norms' figures as read from the tables, m and degrees.
#[derive(Clone, Debug, PartialEq)]
pub struct StairRules {
    pub classes: Vec<Class>,
    pub sum: f64,
    pub sum_min: f64,
    pub going_min: Option<f64>,
    pub tread_min: Option<f64>,
    pub narrow_min: Option<f64>,
    pub risers_max: Option<u32>,
    /// Čl. 38: a straight landing at least this plus a going long.
    pub landing_extra: Option<f64>,
    /// (width up to, offset, share of the width).
    pub walk: Vec<(Option<f64>, Option<f64>, Option<f64>)>,
    pub width_min: Option<f64>,
    pub angle_max: Option<f64>,
    /// Čl. 22: headroom `a + b / cos α`, and the least it may be brought
    /// down to (a house, inside a flat), m.
    pub headroom: Option<(f64, f64, f64)>,
    /// Čl. 23: clear height `a + b · cos α`, never under the least, m.
    pub clear: Option<(f64, f64, f64)>,
    pub building: Building,
}

fn mm(t: &ParamTable, key: &str, col: &str) -> Option<f64> {
    number(t.cell(key, col)).map(|v| v / 1000.0)
}

impl StairRules {
    /// The figures for `s`, by the kind of building it is in.
    pub fn for_stair(tables: &[ParamTable], s: &Stair) -> Option<StairRules> {
        let b = if s.flats {
            Building::Flats
        } else {
            Building::House
        };
        Self::read(tables, b, s.lift)
    }

    /// The figures for a stair in `building`; none without the step
    /// table.
    pub fn read(tables: &[ParamTable], building: Building, lift: bool) -> Option<StairRules> {
        let find = |id: &str| tables.iter().find(|t| t.id == id);
        let st = find(STEPS)?;
        let sum = mm(st, "sum", "value")?;
        let classes = find(CLASSES)
            .map(|t| {
                t.rows
                    .iter()
                    .filter_map(|r| {
                        let key = r.first()?.clone();
                        let v = |c: &str| number(t.cell(&key, c));
                        Some(Class {
                            name: t.cell(&key, "text").to_string(),
                            angle: (v("angle_over")?, v("angle_upto")?),
                            riser: (v("h_min")? / 1000.0, v("h_max")? / 1000.0),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let walk = find(WALK)
            .map(|t| {
                t.rows
                    .iter()
                    .filter_map(|r| {
                        let key = r.first()?.clone();
                        Some((
                            mm(t, &key, "width_upto"),
                            mm(t, &key, "offset"),
                            number(t.cell(&key, "share")),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let b = find(BUILDING);
        let k = building.key();
        let house = building == Building::House;
        let risers = |key: &str| number(st.cell(key, "value")).map(|v| v.round() as u32);
        Some(StairRules {
            classes,
            sum,
            sum_min: mm(st, "sum_min", "value").unwrap_or(sum),
            going_min: mm(st, "going_min", "value"),
            tread_min: mm(st, "tread_min", "value"),
            narrow_min: mm(st, "narrow_min", "value"),
            risers_max: if house {
                risers("risers_max_house").or_else(|| risers("risers_max"))
            } else {
                risers("risers_max")
            },
            landing_extra: mm(st, "landing_extra", "value"),
            walk,
            width_min: b.and_then(|t| mm(t, k, "width_min")),
            headroom: find(HEADROOM)
                .and_then(|t| Some((mm(t, "h1", "a")?, mm(t, "h1", "b")?, mm(t, "h1", "min")?))),
            clear: find(HEADROOM)
                .and_then(|t| Some((mm(t, "h2", "a")?, mm(t, "h2", "b")?, mm(t, "h2", "min")?))),
            building,
            angle_max: b.and_then(|t| {
                let col = if lift { "angle_max_lift" } else { "angle_max" };
                number(t.cell(k, col))
            }),
        })
    }

    /// Čl. 16: where the walking line of a turning flight `width` wide
    /// lies, from its outer edge.
    pub fn walk_from_outer(&self, width: f64) -> Option<f64> {
        let row = self
            .walk
            .iter()
            .find(|(upto, _, _)| upto.is_none_or(|u| width <= u + 1e-6))?;
        match row {
            (_, Some(o), _) => Some(*o),
            (_, None, Some(share)) => Some(width * share),
            _ => None,
        }
    }

    /// The slope class of a flight `angle` degrees steep.
    pub fn class(&self, angle: f64) -> Option<&Class> {
        self.classes
            .iter()
            .find(|c| angle > c.angle.0 - 1e-9 && angle <= c.angle.1 + 1e-9)
    }

    /// The going the step formula gives for risers of `riser`.
    pub fn going_for(&self, riser: f64) -> f64 {
        let g = ((self.sum - 2.0 * riser) * 1000.0).round() / 1000.0;
        g.max(self.going_min.unwrap_or(0.0))
    }

    /// `s` as the rules want it: its going from the step formula, its
    /// walking line and its winders' narrow ends.
    pub fn apply(&self, s: &mut Stair, rise: f64) {
        if s.ramp.is_some() {
            return;
        }
        let n = riser_count(rise, s.max_riser);
        s.tread = self.going_for(rise / n as f64);
        if s.by_steps() || !s.form().is_straight() {
            s.walk = self.walk_from_outer(s.width);
        }
        s.narrow = self.narrow_min;
    }

    /// `s` brought to the norm after the user changed it (George
    /// 2026-11-24: the stair helps while it is drawn and resized): the
    /// risers, their height and the going that keep 2h + b in its range
    /// with what was changed kept as near as the norm allows; never
    /// narrower than the least width.
    pub fn fit(&self, s: &mut Stair, rise: f64, how: Fit) {
        if s.ramp.is_some() || rise <= 0.0 {
            return;
        }
        let set_count = |s: &mut Stair, n: u32| {
            s.max_riser = ((rise / n.max(1) as f64) * 1e4).ceil() / 1e4;
        };
        match how {
            Fit::Width => {}
            Fit::Rise => {
                let n = riser_count(rise, s.max_riser);
                s.tread = self.going_for(rise / n as f64);
            }
            Fit::Risers(n) => {
                let n = n.max(1);
                set_count(s, n);
                s.tread = self.going_for(rise / n as f64);
            }
            Fit::Going => {
                let least = self
                    .going_min
                    .unwrap_or(0.0)
                    .max(self.tread_min.unwrap_or(0.0));
                let b = s.tread.max(least);
                // the riser 2h + b = sum wants, then the count giving a
                // riser no higher, so 2h + b stays at or under it
                let h = ((self.sum - b) / 2.0).max(0.05);
                let n = riser_count(rise, h);
                set_count(s, n);
                let have = 2.0 * rise / n as f64 + b;
                s.tread = if have >= self.sum_min - 5e-4 {
                    b
                } else {
                    self.going_for(rise / n as f64)
                };
            }
            Fit::Run(length) => {
                // the count whose going, from the formula, makes the
                // treads nearest the length asked for
                let best = (2..=80u32)
                    .map(|n| {
                        let b = self.going_for(rise / n as f64);
                        (n, b, ((n - 1) as f64 * b - length).abs())
                    })
                    .min_by(|x, y| x.2.total_cmp(&y.2));
                if let Some((n, b, _)) = best {
                    set_count(s, n);
                    s.tread = b;
                }
            }
        }
        if let Some(w) = self.width_min {
            s.width = s.width.max(w);
        }
        if s.by_steps() || !s.form().is_straight() {
            s.walk = self.walk_from_outer(s.width);
        }
        if s.narrow.is_none() {
            s.narrow = self.narrow_min;
        }
    }
}

/// What the user changed on a stair, for [`StairRules::fit`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Fit {
    /// Its rise or its highest riser: the count follows, then the going.
    Rise,
    /// Its going (the tread width b).
    Going,
    /// Its count of risers.
    Risers(u32),
    /// The plan length of its treads on the walking line, m.
    Run(f64),
    /// Its width (or anything else): only the least width and the walking
    /// line.
    Width,
}

/// [`StairRules::fit`] with the project's tables, when the stair is kept
/// to the norm and the tables are read; true when it was.
pub fn keep_to_norm(model: &crate::Model, s: &mut Stair, how: Fit) -> bool {
    if !s.to_norm || s.ramp.is_some() {
        return false;
    }
    let tables = crate::room::project_tables(model);
    let Some(rules) = StairRules::for_stair(&tables, s) else {
        return false;
    };
    let rise = s.rise(model);
    rules.fit(s, rise, how);
    true
}

/// What does not meet the rules.
#[derive(Clone, Debug, PartialEq)]
pub enum StairFault {
    /// 2h + b out of its range.
    Sum {
        have: f64,
        min: f64,
        max: f64,
    },
    /// Steeper than any class, or the risers not of its class.
    Class {
        angle: f64,
        class: Option<String>,
        riser: f64,
        range: Option<(f64, f64)>,
    },
    Going {
        have: f64,
        min: f64,
    },
    Tread {
        have: f64,
        min: f64,
    },
    Narrow {
        have: f64,
        min: f64,
    },
    Risers {
        have: u32,
        max: u32,
    },
    /// The walking line not where čl. 16 puts it.
    Walk {
        have: f64,
        need: f64,
    },
    Width {
        have: f64,
        min: f64,
    },
    Steep {
        angle: f64,
        max: f64,
    },
    /// Čl. 22: too little headroom over a nosing.
    Headroom {
        have: f64,
        need: f64,
    },
    /// Čl. 23: too little clear height over the pitch line.
    Clear {
        have: f64,
        need: f64,
    },
}

impl StairRules {
    /// Čl. 22, 23: what headroom `h1` (over the nosings) and clear height
    /// over the pitch line (`v`, measured upright) a flight `angle`
    /// degrees steep lacks.
    pub fn check_height(&self, angle: f64, h1: Option<f64>, v: Option<f64>) -> Vec<StairFault> {
        let mut out = Vec::new();
        let cos = angle.to_radians().cos().max(1e-6);
        if let (Some((a, b, min)), Some(have)) = (self.headroom, h1) {
            let full = a + b / cos;
            // a house or a flat may bring it down as far as the least
            let need = if self.building == Building::House {
                full.min(min)
            } else {
                full
            };
            if have < need - 5e-4 {
                out.push(StairFault::Headroom { have, need });
            }
        }
        if let (Some((a, b, min)), Some(v)) = (self.clear, v) {
            let need = (a + b * cos).max(min);
            let have = v * cos;
            if have < need - 5e-4 {
                out.push(StairFault::Clear { have, need });
            }
        }
        out
    }
}

/// The faults of `s` climbing `rise`, by `rules`.
pub fn check(s: &Stair, rise: f64, rules: &StairRules) -> Vec<StairFault> {
    let mut out = Vec::new();
    if s.ramp.is_some() {
        return out;
    }
    let lay = s.layout(rise);
    let (r, b) = (lay.riser, s.tread);
    let sum = 2.0 * r + b;
    if sum < rules.sum_min - 5e-4 || sum > rules.sum + 5e-4 {
        out.push(StairFault::Sum {
            have: sum,
            min: rules.sum_min,
            max: rules.sum,
        });
    }
    let angle = (r / b.max(1e-6)).atan().to_degrees();
    if !rules.classes.is_empty() {
        match rules.class(angle) {
            Some(c) if r >= c.riser.0 - 5e-4 && r <= c.riser.1 + 5e-4 => {}
            c => out.push(StairFault::Class {
                angle,
                class: c.map(|c| c.name.clone()),
                riser: r,
                range: c.map(|c| c.riser),
            }),
        }
    }
    if let Some(min) = rules.going_min.filter(|m| b < m - 5e-4) {
        out.push(StairFault::Going { have: b, min });
    }
    if let Some(min) = rules.tread_min.filter(|m| b < m - 5e-4) {
        out.push(StairFault::Tread { have: b, min });
    }
    if let Some(max) = rules.risers_max {
        let most = if s.by_steps() {
            lay.risers
        } else {
            lay.flights.iter().map(|f| f.risers).max().unwrap_or(0)
        };
        if most > max {
            out.push(StairFault::Risers { have: most, max });
        }
    }
    if s.by_steps() {
        let sp = s.step_plan(rise);
        if let Some(min) = rules.narrow_min {
            let have = sp.narrow.iter().copied().fold(f64::INFINITY, f64::min);
            if have < min - 5e-4 {
                out.push(StairFault::Narrow { have, min });
            }
        }
        if let Some(need) = rules.walk_from_outer(s.width) {
            let have = s.width - s.walk_from_inner();
            if (have - need).abs() > 5e-4 {
                out.push(StairFault::Walk { have, need });
            }
        }
    }
    if let Some(min) = rules.width_min.filter(|m| s.width < m - 5e-4) {
        out.push(StairFault::Width { have: s.width, min });
    }
    if let Some(max) = rules.angle_max.filter(|m| angle > m + 1e-6) {
        out.push(StairFault::Steep { angle, max });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fireparams::parse;
    use crate::members::{Landing, StairShape};
    use crate::ElementId;
    use glam::DVec2;

    // made-up figures, not the norm's
    const FILE: &str = "#table stair_steps\n#norm X\n#edition 1\n#columns key\tvalue\ttext\n\
        sum\t640\t\nsum_min\t610\t\ngoing_min\t200\t\ntread_min\t240\t\nnarrow_min\t120\t\n\
        risers_max\t15\t\nrisers_max_house\t17\t\n\
        #table stair_classes\n#norm X\n#edition 1\n#columns key\tangle_over\tangle_upto\th_min\th_max\ttext\n\
        a\t20\t30\t120\t170\tlow\nb\t30\t40\t170\t200\thigh\n\
        #table stair_walk\n#norm X\n#edition 1\n#columns key\twidth_upto\toffset\tshare\ttext\n\
        1\t1000\t350\t\t\n2\t\t\t0.5\t\n\
        #table stair_building\n#norm X\n#edition 1\n#columns key\twidth_min\tangle_max\tangle_max_lift\ttext\n\
        house\t950\t\t\t\n";

    fn rules() -> StairRules {
        StairRules::read(&parse(FILE), Building::House, false).unwrap()
    }

    #[test]
    fn the_rules_set_the_going_and_the_walking_line_and_then_nothing_fails() {
        let r = rules();
        let mut s = Stair::new(ElementId::new(), DVec2::ZERO, 0.0);
        s.width = 1.0;
        s.max_riser = 0.18;
        s.shape = StairShape::L;
        s.winders = true;
        s.landing = Some(Landing {
            after: 6,
            left: true,
        });
        r.apply(&mut s, 2.8);
        // 16 risers of 175: 640 − 350
        assert!((s.tread - 0.29).abs() < 1e-9, "{}", s.tread);
        assert_eq!(s.walk, Some(0.35));
        assert_eq!(s.narrow, Some(0.12));
        let found = check(&s, 2.8, &r);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn headroom_may_come_down_to_its_least_in_a_house_only() {
        let file = format!(
            "{FILE}#table stair_headroom\n#norm X\n#edition 1\n#columns key\ta\tb\tmin\ttext\n\
             h1\t1400\t800\t2000\t\nh2\t700\t1400\t1800\t\n"
        );
        let t = parse(&file);
        let house = StairRules::read(&t, Building::House, false).unwrap();
        let flats = StairRules::read(&t, Building::Flats, false).unwrap();
        // 1400 + 800 / cos 30° = 2324 mm; a house may go down to 2000
        assert!(house.check_height(30.0, Some(2.1), None).is_empty());
        let f = flats.check_height(30.0, Some(2.1), None);
        assert!(
            matches!(f[0], StairFault::Headroom { need, .. } if (need - 2.32376).abs() < 1e-4),
            "{f:?}"
        );
        // clear height: 2,0 upright over the pitch line is 1,732 square to it,
        // under 700 + 1400 · cos 30° = 1912
        let f = house.check_height(30.0, None, Some(2.0));
        assert!(matches!(f[0], StairFault::Clear { .. }), "{f:?}");
    }

    #[test]
    fn a_narrow_steep_stair_fails_on_each_rule() {
        let r = rules();
        let mut s = Stair::new(ElementId::new(), DVec2::ZERO, 0.0);
        s.width = 0.8;
        s.max_riser = 0.21;
        s.tread = 0.19;
        let found = check(&s, 2.8, &r);
        let has = |f: fn(&StairFault) -> bool| found.iter().any(f);
        assert!(has(|f| matches!(f, StairFault::Sum { .. })), "{found:?}");
        assert!(has(|f| matches!(f, StairFault::Going { .. })));
        assert!(has(|f| matches!(f, StairFault::Tread { .. })));
        assert!(has(|f| matches!(f, StairFault::Width { .. })));
        assert!(has(|f| matches!(f, StairFault::Class { class: None, .. })));
    }

    #[test]
    fn a_stair_kept_to_the_norm_follows_what_was_changed() {
        let r = rules();
        let mut s = Stair::new(ElementId::new(), DVec2::ZERO, 0.0);
        s.width = 0.6;
        // a going typed: the risers follow so 2h + b fits
        s.tread = 0.29;
        r.fit(&mut s, 2.8, Fit::Going);
        let n = riser_count(2.8, s.max_riser);
        let sum = 2.0 * 2.8 / n as f64 + s.tread;
        assert!(sum <= r.sum + 1e-6 && sum >= r.sum_min - 1e-6, "{n} {sum}");
        assert!((s.tread - 0.29).abs() < 1e-9);
        assert!(s.width >= r.width_min.unwrap() - 1e-9);
        // a count typed: the riser and going follow
        r.fit(&mut s, 2.8, Fit::Risers(14));
        assert_eq!(riser_count(2.8, s.max_riser), 14);
        assert!((s.tread - r.going_for(0.2)).abs() < 1e-9);
        // a length dragged: the count whose treads come nearest
        r.fit(&mut s, 2.8, Fit::Run(4.0));
        let n = riser_count(2.8, s.max_riser);
        assert!(
            ((n - 1) as f64 * s.tread - 4.0).abs() < 0.3,
            "{n} {}",
            s.tread
        );
    }
}
