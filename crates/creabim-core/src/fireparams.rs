//! The norms' tables behind the fire calculations, as parameters kept
//! with the project. The program carries no norm values of its own: a
//! set of tables is read from a file the designer has (written from
//! their own copy of the norms), every value can be changed when a norm
//! changes, a changed value keeps what it was, and the set can be saved
//! to a file and read into another project.
//!
//! The file is tab-separated text (`<TAB>` below) with one or more tables. A table
//! starts with its header lines:
//!
//! ```text
//! #table occupancy
//! #norm STN 92 0241
//! #edition 2012 + Z1
//! #columns item<TAB>name<TAB>area<TAB>factor<TAB>note
//! 1.1.1<TAB>Offices of one workplace<TAB>10,0<TAB><TAB>
//! ```
//!
//! Any other line starting with `#` is a remark.

use serde::{Deserialize, Serialize};

/// The persons a room is counted for (STN 92 0241, table 1).
pub const OCCUPANCY: &str = "occupancy";
/// The fire load of a kind of space, random pn and its factor an
/// (STN 92 0201-1, annex A).
pub const FIRE_LOAD: &str = "fire_load";
/// The fixed fire load ps of windows, doors and floors by a room's
/// area (STN 92 0201-1, table 1).
pub const FIRE_FIXED_LOAD: &str = "fire_fixed_load";
/// The factor k of the openings' geometry by n and the room area, for
/// zones up to the limit and over it (STN 92 0201-1, annex E).
pub const FIRE_OPENING_K: &str = "fire_opening_k";
pub const FIRE_OPENING_K_LARGE: &str = "fire_opening_k_large";
/// The fire risk's single values by key (STN 92 0201-1, 4 to 6).
pub const FIRE_RISK: &str = "fire_risk";
/// The localities the heat loss and gain calculations take their outside
/// temperatures from: name, region, te, te_year, te_heating, days,
/// altitude (the designer's table, STN 73 0540-3).
pub const CLIMATE_SITES: &str = "climate_sites";

/// The highest outside temperature of each summer month for the
/// cooling load: month, te_max (STN 73 0548).
pub const SUMMER_MAX: &str = "summer_max";

/// The air changes at 50 Pa a building's envelope lets through, by the
/// building and the tightness of its envelope: building, tightness,
/// from, to (an empty bound is open).
pub const N50_RANGES: &str = "n50_ranges";

/// The rain intensities of the rain gauge stations for soakaways:
/// station, n (the periodicity, 1/years), then l/(s·ha) for each duration
/// in minutes named by the column (the designer's table, SHMÚ series).
pub const RAIN_INTENSITY: &str = "rain_intensity";

/// What a room type is counted as where the designer has not said:
/// type (a room type's name, "Spálňa"), occupancy (an item of
/// [`OCCUPANCY`]), fire_load (an item of [`FIRE_LOAD`]), when ("byt":
/// only in a building with a room of a row without it).
pub const ROOM_TYPE_USES: &str = "room_type_uses";

/// The tables the climate file carries.
pub const CLIMATE_TABLES: [&str; 4] = [CLIMATE_SITES, SUMMER_MAX, N50_RANGES, RAIN_INTENSITY];

/// A rain gauge station's rain series.
#[derive(Clone, Debug, PartialEq)]
pub struct RainStation {
    pub name: String,
    /// Per periodicity n: (minutes, l/(s·ha)) for each duration.
    pub series: Vec<(f64, Vec<(f64, f64)>)>,
}

impl RainStation {
    /// The series of periodicity `n`.
    pub fn series_for(&self, n: f64) -> Option<&Vec<(f64, f64)>> {
        self.series
            .iter()
            .find(|(m, _)| (m - n).abs() < 1e-6)
            .map(|(_, s)| s)
    }
}

/// The rain gauge stations among `tables`, in the table's order.
pub fn rain_stations(tables: &[ParamTable]) -> Vec<RainStation> {
    let Some(t) = tables.iter().find(|t| t.id == RAIN_INTENSITY) else {
        return vec![];
    };
    let durations: Vec<(usize, f64)> = t
        .columns
        .iter()
        .enumerate()
        .skip(2)
        .filter_map(|(i, c)| Some((i, number(c)?)))
        .collect();
    let mut out: Vec<RainStation> = vec![];
    for r in &t.rows {
        let (Some(name), Some(n)) = (r.first(), r.get(1).and_then(|x| number(x))) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        let series: Vec<(f64, f64)> = durations
            .iter()
            .filter_map(|(i, m)| Some((*m, number(r.get(*i)?)?)))
            .collect();
        match out.iter_mut().find(|s| &s.name == name) {
            Some(s) => s.series.push((n, series)),
            None => out.push(RainStation {
                name: name.clone(),
                series: vec![(n, series)],
            }),
        }
    }
    out
}

/// A range of air changes at 50 Pa.
#[derive(Clone, Debug, PartialEq)]
pub struct N50Range {
    pub building: String,
    pub tightness: String,
    pub from: Option<f64>,
    pub to: Option<f64>,
}

impl N50Range {
    /// How the range reads: "4 – 10", "> 10", "< 4".
    pub fn text(&self) -> String {
        let f = |v: f64| format!("{v}").replace('.', ",");
        match (self.from, self.to) {
            (Some(a), Some(b)) => format!("{} – {}", f(a), f(b)),
            (Some(a), None) => format!("> {}", f(a)),
            (None, Some(b)) => format!("< {}", f(b)),
            (None, None) => String::new(),
        }
    }

    /// A value to start from: the middle, or the open end's bound.
    pub fn usual(&self) -> Option<f64> {
        match (self.from, self.to) {
            (Some(a), Some(b)) => Some((a + b) / 2.0),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        }
    }

    /// `v` brought into the range.
    pub fn clamp(&self, v: f64) -> f64 {
        let v = self.from.map_or(v, |a| v.max(a));
        self.to.map_or(v, |b| v.min(b))
    }
}

/// The rows of the n50 table among `tables`.
pub fn n50_ranges(tables: &[ParamTable]) -> Vec<N50Range> {
    let Some(t) = tables.iter().find(|t| t.id == N50_RANGES) else {
        return vec![];
    };
    let col = |r: &Vec<String>, name: &str| {
        t.column(name)
            .and_then(|c| r.get(c))
            .map_or(String::new(), |s| s.clone())
    };
    t.rows
        .iter()
        .map(|r| N50Range {
            building: col(r, "building"),
            tightness: col(r, "tightness"),
            from: number(&col(r, "from")),
            to: number(&col(r, "to")),
        })
        .filter(|r| !r.building.is_empty())
        .collect()
}

/// The summer months and their highest outside temperature, °C.
pub fn summer_max(tables: &[ParamTable]) -> Vec<(String, f64)> {
    let Some(t) = tables.iter().find(|t| t.id == SUMMER_MAX) else {
        return vec![];
    };
    let (Some(m), Some(v)) = (t.column("month"), t.column("te_max")) else {
        return vec![];
    };
    t.rows
        .iter()
        .filter_map(|r| Some((r.get(m)?.clone(), number(r.get(v)?)?)))
        .collect()
}

/// A locality's climate.
#[derive(Clone, Debug, PartialEq)]
pub struct ClimateSite {
    pub name: String,
    /// Its region ("Žilinský kraj").
    pub region: String,
    /// Design outside temperature θe, °C.
    pub te: f64,
    /// Mean annual outside temperature, °C.
    pub te_year: Option<f64>,
    /// Mean outside temperature of the heating season, °C.
    pub te_heating: Option<f64>,
    /// Days of the heating season.
    pub days: Option<f64>,
    /// Height above sea level, m.
    pub altitude: Option<f64>,
}

impl ClimateSite {
    /// Degree days of the heating season for an inside `ti`: d (ti − te,m).
    pub fn degree_days(&self, ti: f64) -> Option<f64> {
        Some(self.days? * (ti - self.te_heating?))
    }
}

/// The localities of the climate table among `tables`, in its order;
/// rows without a design temperature are left out.
pub fn climate_sites(tables: &[ParamTable]) -> Vec<ClimateSite> {
    let Some(t) = tables.iter().find(|t| t.id == CLIMATE_SITES) else {
        return vec![];
    };
    let col = |r: &Vec<String>, name: &str| {
        t.column(name)
            .and_then(|c| r.get(c))
            .map_or("", |s| s.as_str())
            .to_string()
    };
    t.rows
        .iter()
        .filter_map(|r| {
            Some(ClimateSite {
                name: col(r, "name"),
                region: col(r, "region"),
                te: number(&col(r, "te"))?,
                te_year: number(&col(r, "te_year")),
                te_heating: number(&col(r, "te_heating")),
                days: number(&col(r, "days")),
                altitude: number(&col(r, "altitude")),
            })
        })
        .filter(|s| !s.name.is_empty())
        .collect()
}

/// One table of a norm.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ParamTable {
    /// What the program uses it for ([`OCCUPANCY`]).
    pub id: String,
    /// The norm it is from ("STN 92 0241") and its edition.
    pub norm: String,
    pub edition: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// The cells changed by hand: row, column and the value read from
    /// the file.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub changed: Vec<Change>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Change {
    pub row: usize,
    pub column: usize,
    pub was: String,
}

impl ParamTable {
    pub fn column(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c == name)
    }

    /// The row whose first cell is `key`.
    pub fn row(&self, key: &str) -> Option<&Vec<String>> {
        self.rows
            .iter()
            .find(|r| r.first().is_some_and(|k| k == key))
    }

    /// A cell of the row with `key`, by its column's name; empty where
    /// there is none.
    pub fn cell(&self, key: &str, column: &str) -> &str {
        self.column(column)
            .and_then(|c| self.row(key)?.get(c))
            .map_or("", |s| s.as_str())
    }

    /// Sets a cell, remembering what the file gave; setting it back
    /// forgets the change.
    pub fn set(&mut self, row: usize, column: usize, value: &str) {
        let Some(cell) = self.rows.get_mut(row).and_then(|r| r.get_mut(column)) else {
            return;
        };
        if cell == value {
            return;
        }
        let was = std::mem::replace(cell, value.to_string());
        match self
            .changed
            .iter()
            .position(|c| c.row == row && c.column == column)
        {
            Some(i) if self.changed[i].was == value => {
                self.changed.remove(i);
            }
            Some(_) => {}
            None => self.changed.push(Change { row, column, was }),
        }
    }

    /// What a changed cell was in the file.
    pub fn was(&self, row: usize, column: usize) -> Option<&str> {
        self.changed
            .iter()
            .find(|c| c.row == row && c.column == column)
            .map(|c| c.was.as_str())
    }

    /// Puts a cell, or with `None` every cell, back to the file's value.
    pub fn reset(&mut self, cell: Option<(usize, usize)>) {
        let mut keep = vec![];
        for c in std::mem::take(&mut self.changed) {
            if cell.is_some_and(|at| at != (c.row, c.column)) {
                keep.push(c);
            } else if let Some(v) = self.rows.get_mut(c.row).and_then(|r| r.get_mut(c.column)) {
                *v = c.was;
            }
        }
        self.changed = keep;
    }
}

/// The tables of a parameter file. Lines before the first `#table` and
/// tables with no columns are left out.
pub fn parse(text: &str) -> Vec<ParamTable> {
    let mut out: Vec<ParamTable> = vec![];
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(head) = line.strip_prefix('#') {
            let (key, value) = head.split_once(' ').unwrap_or((head, ""));
            let value = value.trim();
            match (key, out.last_mut()) {
                ("table", _) => out.push(ParamTable {
                    id: value.to_string(),
                    ..Default::default()
                }),
                ("norm", Some(t)) => t.norm = value.to_string(),
                ("edition", Some(t)) => t.edition = value.to_string(),
                ("columns", Some(t)) => {
                    t.columns = value.split('\t').map(|c| c.trim().to_string()).collect()
                }
                _ => {}
            }
            continue;
        }
        let Some(t) = out.last_mut() else { continue };
        if line.trim().is_empty() || t.columns.is_empty() {
            continue;
        }
        let mut cells: Vec<String> = line.split('\t').map(|c| c.trim().to_string()).collect();
        cells.resize(t.columns.len(), String::new());
        t.rows.push(cells);
    }
    out.retain(|t| !t.id.is_empty() && !t.columns.is_empty());
    out
}

/// The tables as a parameter file, with the values as they stand.
pub fn write(tables: &[ParamTable]) -> String {
    let mut s = String::new();
    for t in tables {
        s += &format!(
            "#table {}\n#norm {}\n#edition {}\n#columns {}\n",
            t.id,
            t.norm,
            t.edition,
            t.columns.join("\t")
        );
        for r in &t.rows {
            s += &r.join("\t");
            s.push('\n');
        }
    }
    s
}

/// The tables kept in the application (the designer's copy, saved once
/// and used by every project): a project's own table with the same id
/// wins. Set at start and when the designer saves them; the slices are
/// kept for the program's life, a few kilobytes each time.
static APP_TABLES: std::sync::RwLock<&'static [ParamTable]> = std::sync::RwLock::new(&[]);

/// Puts `tables` in place of the application's tables.
pub fn set_app_tables(tables: Vec<ParamTable>) {
    let leaked: &'static [ParamTable] = Box::leak(tables.into_boxed_slice());
    if let Ok(mut t) = APP_TABLES.write() {
        *t = leaked;
    }
}

/// The application's tables.
pub fn app_tables() -> &'static [ParamTable] {
    APP_TABLES.read().map(|t| *t).unwrap_or(&[])
}

/// The table `id`: the project's own, else the application's.
pub fn find<'a>(own: &'a [ParamTable], id: &str) -> Option<&'a ParamTable> {
    own.iter()
        .find(|t| t.id == id)
        .or_else(|| app_tables().iter().find(|t| t.id == id))
}

/// The application's tables with the project's own put over them.
pub fn effective(own: &[ParamTable]) -> Vec<ParamTable> {
    let mut all = app_tables().to_vec();
    merge(&mut all, own.to_vec());
    all
}

/// Takes the tables of a file into the project's: one with an id the
/// project has replaces it.
pub fn merge(into: &mut Vec<ParamTable>, new: Vec<ParamTable>) {
    for t in new {
        match into.iter_mut().find(|o| o.id == t.id) {
            Some(o) => *o = t,
            None => into.push(t),
        }
    }
}

/// A number written the Slovak way ("1,5") or with a point.
pub fn number(text: &str) -> Option<f64> {
    let t: String = text
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit() || matches!(c, ',' | '.' | '-'))
        .collect();
    t.replace(',', ".").parse().ok()
}

/// A value that changes by steps of a quantity ("0-100 m2: 0,8; nad
/// 100 m2: 1,2", "prvých 10: 1,3; ďalšie: 0,5") or is one number: the
/// upper limit of each step, endless for the last, and its value.
pub fn steps(text: &str) -> Vec<(f64, f64)> {
    let text = text.trim();
    if !text.contains(':') {
        return number(text)
            .map(|v| vec![(f64::INFINITY, v)])
            .unwrap_or_default();
    }
    let mut out = vec![];
    for part in text.split(';') {
        let Some((range, value)) = part.split_once(':') else {
            continue;
        };
        let Some(value) = number(value) else { continue };
        // The step's upper limit is the last number of its range; a
        // range that opens upward ("nad 100", "ďalšie") has none.
        let range = range
            .trim()
            .to_lowercase()
            .replace("m2", "")
            .replace("m²", "");
        let open =
            range.starts_with("nad") || range.starts_with("ďal") || range.starts_with("over");
        let last = range
            .split(|c: char| !(c.is_ascii_digit() || c == ',' || c == '.'))
            .filter_map(number)
            .next_back();
        let limit = match (open, last) {
            (false, Some(l)) => l,
            _ => f64::INFINITY,
        };
        out.push((limit, value));
    }
    out
}

/// A quantity over a stepped divisor, part by part: 150 m² at "0-100:
/// 0,8; over 100: 1,2" is 100 / 0,8 + 50 / 1,2.
pub fn divided(quantity: f64, steps: &[(f64, f64)]) -> f64 {
    stepped(
        quantity,
        steps,
        |part, v| if v > 0.0 { part / v } else { 0.0 },
    )
}

/// A quantity times a stepped factor, part by part.
pub fn multiplied(quantity: f64, steps: &[(f64, f64)]) -> f64 {
    stepped(quantity, steps, |part, v| part * v)
}

fn stepped(quantity: f64, steps: &[(f64, f64)], f: impl Fn(f64, f64) -> f64) -> f64 {
    let mut from = 0.0;
    let mut sum = 0.0;
    for (limit, value) in steps {
        let to = quantity.min(*limit);
        if to > from {
            sum += f(to - from, *value);
            from = to;
        }
    }
    sum
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_climate_table_gives_its_localities() {
        let text = "#table climate_sites\n#norm X\n#columns name\tregion\tte\tte_year\tte_heating\tdays\taltitude\nMesto A\tKraj\t-14,0\t8,0\t3,5\t240\t300\nBez teploty\tKraj\t\t\t\t\t\n";
        let s = climate_sites(&parse(text));
        assert_eq!(s.len(), 1);
        assert_eq!(
            (s[0].name.as_str(), s[0].te, s[0].te_year),
            ("Mesto A", -14.0, Some(8.0))
        );
        assert_eq!(s[0].degree_days(20.0), Some(240.0 * 16.5));
        let n = n50_ranges(&parse("#table n50_ranges\n#columns building\ttightness\tfrom\tto\nDom\tstredný\t4\t10\nDom\tvysoký\t\t4\n"));
        assert_eq!((n[0].text().as_str(), n[0].usual()), ("4 – 10", Some(7.0)));
        assert_eq!((n[1].text().as_str(), n[1].clamp(6.0)), ("< 4", 4.0));
    }

    use super::*;

    #[test]
    fn rain_stations_group_their_series() {
        let text = "#table rain_intensity\n#columns station\tn\t5\t180\nPiešťany\t1\t253\t20\nPiešťany\t0.2\t359\t31\nŽilina\t0.2\t300\t30\n";
        let st = rain_stations(&parse(text));
        assert_eq!(st.len(), 2);
        assert_eq!(
            st[0].series_for(0.2).unwrap(),
            &vec![(5.0, 359.0), (180.0, 31.0)]
        );
        assert!(st[1].series_for(1.0).is_none());
    }

    const FILE: &str = "remark before any table\n#table occupancy\n#norm N 1\n#edition 2012\n# a remark\n#columns item\tname\tarea\tfactor\n1\tOffices\t10,0\t\n2\tHall\t0-100 m2: 0,8; nad 100 m2: 1,2\n";

    #[test]
    fn a_file_reads_into_tables_and_writes_back() {
        let t = parse(FILE);
        assert_eq!(t.len(), 1);
        assert_eq!(
            (t[0].id.as_str(), t[0].norm.as_str(), t[0].edition.as_str()),
            ("occupancy", "N 1", "2012")
        );
        assert_eq!(t[0].rows.len(), 2);
        assert_eq!(t[0].cell("1", "area"), "10,0");
        // A short row is filled out to the columns.
        assert_eq!(t[0].rows[1].len(), 4);
        assert_eq!(parse(&write(&t)), t);
    }

    #[test]
    fn a_changed_cell_keeps_the_files_value_and_resets() {
        let mut t = parse(FILE).remove(0);
        t.set(0, 2, "12,0");
        t.set(0, 2, "14,0");
        assert_eq!(t.was(0, 2), Some("10,0"));
        assert_eq!(t.cell("1", "area"), "14,0");
        // Setting the file's value back forgets the change.
        t.set(0, 2, "10,0");
        assert!(t.changed.is_empty());
        t.set(0, 2, "9");
        t.set(1, 1, "Halls");
        t.reset(Some((0, 2)));
        assert_eq!((t.cell("1", "area"), t.changed.len()), ("10,0", 1));
        t.reset(None);
        assert_eq!(t, parse(FILE).remove(0));
    }

    #[test]
    fn a_file_replaces_the_table_with_its_id() {
        let mut have = parse(FILE);
        have[0].set(0, 2, "1");
        let mut other = parse(FILE);
        other[0].id = "other".into();
        merge(&mut have, parse(FILE));
        merge(&mut have, other);
        assert_eq!(have.len(), 2);
        assert!(have[0].changed.is_empty());
    }

    #[test]
    fn stepped_values_count_part_by_part() {
        assert_eq!(steps("1,5"), vec![(f64::INFINITY, 1.5)]);
        let s = steps("0-100 m2: 0,8; nad 100 m2: 1,2");
        assert_eq!(s, vec![(100.0, 0.8), (f64::INFINITY, 1.2)]);
        assert!((divided(150.0, &s) - (125.0 + 50.0 / 1.2)).abs() < 1e-9);
        assert!((divided(80.0, &s) - 100.0).abs() < 1e-9);
        let s = steps("do 150 m2: 10,0; 150-1000 m2: 50,0; nad 1000 m2: 100,0");
        assert_eq!(
            s,
            vec![(150.0, 10.0), (1000.0, 50.0), (f64::INFINITY, 100.0)]
        );
        let s = steps("prvých 10: 1,3; ďalšie: 0,5");
        assert!((multiplied(14.0, &s) - 15.0).abs() < 1e-9);
        assert!(steps("podľa 1.2.2").is_empty());
    }

    #[test]
    fn a_project_table_wins_over_the_application_s() {
        let app =
            parse("#table a\n#columns key\tvalue\nx\t1\n#table b\n#columns key\tvalue\nx\t2\n");
        set_app_tables(app);
        let own = parse("#table b\n#columns key\tvalue\nx\t3\n");
        assert_eq!(find(&own, "a").unwrap().cell("x", "value"), "1");
        assert_eq!(find(&own, "b").unwrap().cell("x", "value"), "3");
        assert_eq!(find(&[], "b").unwrap().cell("x", "value"), "2");
        let all = effective(&own);
        assert_eq!(all.len(), 2);
        assert_eq!(all[1].cell("x", "value"), "3");
        set_app_tables(Vec::new());
    }
}
