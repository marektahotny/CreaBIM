//! The results of the calculations, kept with the model: each
//! calculation (heat loss, water supply, gas...) writes what it found out
//! about each room, pipe, valve or circuit, so the values show in that
//! element's properties and go with it to IFC, whether or not the
//! calculation's window is open. One element holds them all (at most one
//! per project, read through [`results`]); each calculation's set is
//! replaced as a whole when it runs again.

use crate::{ElementId, ElementKind, Model, Transaction};
use serde::{Deserialize, Serialize};

/// The unit a value is in, as the calculation gives it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Unit {
    /// A count or a plain number.
    None,
    /// Power, W.
    W,
    /// Power, kW.
    KW,
    /// Flow, l/s.
    Ls,
    /// Flow, l/min.
    Lmin,
    /// Flow, m³/h.
    M3h,
    /// Mass flow, kg/h.
    Kgh,
    /// Velocity, m/s.
    Ms,
    /// Pressure, Pa.
    Pa,
    /// Pressure, kPa.
    KPa,
    /// Pressure loss per length, Pa/m.
    PaM,
    /// Temperature, °C.
    C,
    /// Length, mm.
    Mm,
    /// Length, m.
    M,
    /// Area, m².
    M2,
    /// Volume, m³.
    M3,
    /// Air changes, 1/h.
    PerH,
    /// Electric current, A.
    A,
    /// Voltage, V.
    V,
    /// Per cent.
    Pct,
    /// Sound level, dB(A).
    DbA,
    /// Heat flux, W/m².
    Wm2,
}

impl Unit {
    /// How it is written after a number.
    pub fn text(self) -> &'static str {
        match self {
            Unit::None => "",
            Unit::W => "W",
            Unit::KW => "kW",
            Unit::Ls => "l/s",
            Unit::Lmin => "l/min",
            Unit::M3h => "m³/h",
            Unit::Kgh => "kg/h",
            Unit::Ms => "m/s",
            Unit::Pa => "Pa",
            Unit::KPa => "kPa",
            Unit::PaM => "Pa/m",
            Unit::C => "°C",
            Unit::Mm => "mm",
            Unit::M => "m",
            Unit::M2 => "m²",
            Unit::M3 => "m³",
            Unit::PerH => "1/h",
            Unit::A => "A",
            Unit::V => "V",
            Unit::Pct => "%",
            Unit::DbA => "dB(A)",
            Unit::Wm2 => "W/m²",
        }
    }

    /// Decimals worth showing.
    pub fn decimals(self) -> usize {
        match self {
            Unit::None | Unit::W | Unit::Pa | Unit::Mm | Unit::DbA | Unit::Wm2 => 0,
            Unit::Ls | Unit::Ms => 3,
            Unit::Kgh => 1,
            Unit::KW | Unit::Lmin | Unit::M3h | Unit::KPa | Unit::PerH | Unit::A => 2,
            _ => 1,
        }
    }

    /// The value in SI units (W, m³/s, kg/s, m/s, Pa, Pa/m, °C, m, m², m³, A,
    /// V, a ratio), as IFC's measures take it.
    pub fn si(self, v: f64) -> f64 {
        match self {
            Unit::KW => v * 1000.0,
            Unit::Ls => v / 1000.0,
            Unit::Lmin => v / 60_000.0,
            Unit::M3h => v / 3600.0,
            Unit::Kgh => v / 3600.0,
            Unit::KPa => v * 1000.0,
            Unit::Mm => v / 1000.0,
            Unit::Pct => v / 100.0,
            _ => v,
        }
    }
}

/// One value.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Val {
    Num(f64),
    Text(String),
    Flag(bool),
}

/// One result: what it is (`key`, for programs: "phi", "flow"...), its
/// name as people read it, its value and unit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CalcValue {
    pub key: String,
    pub label: String,
    pub value: Val,
    #[serde(default = "no_unit")]
    pub unit: Unit,
}

fn no_unit() -> Unit {
    Unit::None
}

impl CalcValue {
    pub fn num(key: &str, label: &str, v: f64, unit: Unit) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            value: Val::Num(if v.is_finite() { v } else { 0.0 }),
            unit,
        }
    }
    pub fn text(key: &str, label: &str, v: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            value: Val::Text(v.into()),
            unit: Unit::None,
        }
    }
    pub fn flag(key: &str, label: &str, v: bool) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            value: Val::Flag(v),
            unit: Unit::None,
        }
    }

    /// The number, if it is one.
    pub fn number(&self) -> Option<f64> {
        match self.value {
            Val::Num(v) => Some(v),
            _ => None,
        }
    }

    /// The value as people read it: "1 250 W", "áno".
    pub fn shown(&self) -> String {
        match &self.value {
            Val::Num(v) => {
                let s = format!("{:.*}", self.unit.decimals(), v);
                match self.unit {
                    Unit::None => s,
                    u => format!("{s} {}", u.text()),
                }
            }
            Val::Text(t) => t.clone(),
            Val::Flag(true) => "áno".into(),
            Val::Flag(false) => "nie".into(),
        }
    }
}

/// What one calculation found, element by element.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ResultSet {
    /// Which calculation: "heatloss", "water", "gas"...
    pub calc: String,
    /// Its name, as people read it ("Tepelné straty").
    pub title: String,
    /// The standard it follows, if any ("STN EN 12831").
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub norm: String,
    /// When it ran, "YYYY-MM-DD HH:MM".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub at: String,
    /// Each element's values.
    pub elements: Vec<(ElementId, Vec<CalcValue>)>,
    /// The whole system's values, by the code its pipes are drawn in
    /// ("SV", "UK"...); "" for the whole building (its total heat loss).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub systems: Vec<(String, Vec<CalcValue>)>,
}

impl ResultSet {
    pub fn new(calc: &str, title: &str, norm: &str) -> Self {
        Self {
            calc: calc.into(),
            title: title.into(),
            norm: norm.into(),
            at: String::new(),
            elements: vec![],
            systems: vec![],
        }
    }

    /// Add `values` for the system drawn in `code` ("" the building).
    pub fn put_system(&mut self, code: &str, values: Vec<CalcValue>) {
        if values.is_empty() {
            return;
        }
        match self.systems.iter_mut().find(|(c, _)| c == code) {
            Some((_, v)) => v.extend(values),
            None => self.systems.push((code.to_string(), values)),
        }
    }

    /// Add `values` for `id`, joined to what it already has (a pipe in
    /// two networks of one calculation keeps both).
    pub fn put(&mut self, id: ElementId, values: Vec<CalcValue>) {
        if values.is_empty() {
            return;
        }
        match self.elements.iter_mut().find(|(e, _)| *e == id) {
            Some((_, v)) => v.extend(values),
            None => self.elements.push((id, values)),
        }
    }

    /// The same results, ignoring when they ran.
    fn same(&self, other: &ResultSet) -> bool {
        self.calc == other.calc
            && self.title == other.title
            && self.norm == other.norm
            && self.elements == other.elements
            && self.systems == other.systems
    }
}

/// All the calculations' results in the project.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CalcResults {
    pub sets: Vec<ResultSet>,
}

impl CalcResults {
    pub fn validate(&self) -> Result<(), String> {
        let ok = self.sets.iter().all(|s| {
            s.elements
                .iter()
                .flat_map(|(_, v)| v)
                .all(|v| !matches!(v.value, Val::Num(n) if !n.is_finite()))
        });
        if ok {
            Ok(())
        } else {
            Err("a calculation result is not a finite number".into())
        }
    }

    /// Every set's values for `id`: the set's title and the values.
    pub fn of(&self, id: ElementId) -> Vec<(&ResultSet, &[CalcValue])> {
        self.sets
            .iter()
            .filter_map(|s| {
                s.elements
                    .iter()
                    .find(|(e, _)| *e == id)
                    .map(|(_, v)| (s, v.as_slice()))
            })
            .collect()
    }
}

/// The project's calculation results (none yet: empty).
pub fn results(model: &Model) -> CalcResults {
    model
        .iter()
        .find_map(|e| match &e.kind {
            ElementKind::Results(r) => Some(r.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// The values the calculations wrote for `id`, by calculation.
pub fn of(model: &Model, id: ElementId) -> Vec<(ResultSet, Vec<CalcValue>)> {
    model
        .iter()
        .find_map(|e| match &e.kind {
            ElementKind::Results(r) => Some(
                r.of(id)
                    .into_iter()
                    .map(|(s, v)| {
                        let head = ResultSet {
                            calc: s.calc.clone(),
                            title: s.title.clone(),
                            norm: s.norm.clone(),
                            at: s.at.clone(),
                            elements: vec![],
                            systems: vec![],
                        };
                        (head, v.to_vec())
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

/// The edit that puts `set` in place of what the same calculation wrote
/// before; nothing when the values are the same. Elements no longer in
/// the model are left out.
pub fn write(model: &Model, mut set: ResultSet, at: &str) -> Transaction {
    let tx = Transaction::new(format!("Výsledky výpočtu: {}", set.title));
    set.elements.retain(|(id, _)| model.get(*id).is_some());
    set.at = at.to_string();
    let current = model
        .iter()
        .find(|e| matches!(e.kind, ElementKind::Results(_)));
    let mut all = match current.map(|e| &e.kind) {
        Some(ElementKind::Results(r)) => r.clone(),
        _ => CalcResults::default(),
    };
    match all.sets.iter_mut().find(|s| s.calc == set.calc) {
        Some(s) if s.same(&set) => return tx,
        Some(s) => *s = set,
        None if set.elements.is_empty() && set.systems.is_empty() => return tx,
        None => all.sets.push(set),
    }
    match current {
        Some(e) => {
            let mut after = e.clone();
            after.kind = ElementKind::Results(all);
            tx.replace(e.clone(), after)
        }
        None => tx.insert(crate::Element::new(
            "Výsledky výpočtov",
            ElementKind::Results(all),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;

    #[test]
    fn a_calculation_replaces_its_own_results_only() {
        let mut doc = Document::new();
        let level = crate::Element::new(
            "L",
            ElementKind::Level(crate::Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let id = level.id;
        doc.commit(Transaction::new("l").insert(level)).unwrap();
        let mut a = ResultSet::new("heatloss", "Tepelné straty", "STN EN 12831");
        a.put(
            id,
            vec![CalcValue::num("phi", "Tepelná strata", 1250.0, Unit::W)],
        );
        doc.commit(write(doc.model(), a.clone(), "2026-11-19 10:00"))
            .unwrap();
        let mut b = ResultSet::new("water", "Vodovod", "");
        b.put(id, vec![CalcValue::num("flow", "Prietok", 0.25, Unit::Ls)]);
        doc.commit(write(doc.model(), b, "")).unwrap();
        assert_eq!(of(doc.model(), id).len(), 2);
        // the same again changes nothing
        assert!(write(doc.model(), a.clone(), "later").ops.is_empty());
        a.elements[0].1[0] = CalcValue::num("phi", "Tepelná strata", 900.0, Unit::W);
        doc.commit(write(doc.model(), a, "")).unwrap();
        let got = of(doc.model(), id);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].1[0].shown(), "900 W");
        // elements that are gone are not kept
        let mut c = ResultSet::new("gas", "Plyn", "");
        c.put(ElementId::new(), vec![CalcValue::flag("x", "x", true)]);
        assert!(write(doc.model(), c, "").ops.is_empty());
    }

    #[test]
    fn units_convert_to_si() {
        assert_eq!(Unit::Ls.si(1.0), 0.001);
        assert_eq!(Unit::KPa.si(2.0), 2000.0);
        assert!((Unit::M3h.si(3.6) - 0.001).abs() < 1e-12);
    }
}
