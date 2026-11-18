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
    use super::*;

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
}
