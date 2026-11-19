//! The building services' systems and circuits, and what the
//! calculations found ([`creabim_core::results`]), where IFC viewers look
//! for them:
//!
//! - each pipe system as an `IfcDistributionSystem` grouping its pipes,
//!   ducts and valves, each circuit as an `IfcDistributionCircuit`
//!   grouping its devices;
//! - standard property sets where IFC has one: a room's heat loss, heat
//!   gain, temperature and air in `Pset_SpaceThermalDesign`, a segment's
//!   diameters in `Pset_PipeSegmentTypeCommon` / `Pset_DuctSegmentTypeCommon`
//!   and its length in `Qto_…BaseQuantities`, a radiator's output in
//!   `Pset_SpaceHeaterTypeCommon`, a valve's kv in `Pset_ValveTypeCommon`;
//! - everything each calculation found in its own set, `CreaBIM_<calc>`
//!   (`CreaBIM_TepelneStraty`, `CreaBIM_Vodovod`...), in the units the
//!   calculation gives, written in each property's name ("Prietok [l/s]"),
//!   on the element, its system or the building.

use super::Ctx;
use crate::params::Prop;
use crate::step::{list, string, Ref};
use creabim_core::results::{CalcValue, ResultSet, Unit, Val};
use creabim_core::{ElementId, ElementKind, Model};
use std::collections::HashMap;

/// The units of the measures the results are written in, for the
/// project's unit assignment.
pub(super) fn units(c: &mut Ctx) -> Vec<Ref> {
    let si = |c: &mut Ctx, kind: &str, prefix: &str, name: &str| {
        c.w.add("IFCSIUNIT", format!("*,.{kind}.,{prefix},.{name}."))
    };
    let watt = si(c, "POWERUNIT", "$", "WATT");
    let pascal = si(c, "PRESSUREUNIT", "$", "PASCAL");
    let celsius = si(c, "THERMODYNAMICTEMPERATUREUNIT", "$", "DEGREE_CELSIUS");
    let ampere = si(c, "ELECTRICCURRENTUNIT", "$", "AMPERE");
    let volt = si(c, "ELECTRICVOLTAGEUNIT", "$", "VOLT");
    let second = si(c, "TIMEUNIT", "$", "SECOND");
    let kilogram = si(c, "MASSUNIT", ".KILO.", "GRAM");
    // the base units the derived ones are made of
    let metre = si(c, "LENGTHUNIT", "$", "METRE");
    let m2 = si(c, "AREAUNIT", "$", "SQUARE_METRE");
    let m3 = si(c, "VOLUMEUNIT", "$", "CUBIC_METRE");
    let derived = |c: &mut Ctx, kind: &str, parts: &[(Ref, i32)]| {
        let elements: Vec<Ref> = parts
            .iter()
            .map(|(u, e)| c.w.add("IFCDERIVEDUNITELEMENT", format!("{u},{e}")))
            .collect();
        c.w.add("IFCDERIVEDUNIT", format!("{},.{kind}.,$", list(&elements)))
    };
    let flow = derived(c, "VOLUMETRICFLOWRATEUNIT", &[(m3, 1), (second, -1)]);
    let mass_flow = derived(c, "MASSFLOWRATEUNIT", &[(kilogram, 1), (second, -1)]);
    let velocity = derived(c, "LINEARVELOCITYUNIT", &[(metre, 1), (second, -1)]);
    let flux = derived(c, "HEATFLUXDENSITYUNIT", &[(watt, 1), (m2, -1)]);
    vec![
        watt, pascal, celsius, ampere, volt, second, kilogram, flow, mass_flow, velocity, flux,
    ]
}

/// A value as a typed IFC measure in SI units, for the standard sets.
fn measure(v: &CalcValue) -> Option<Prop> {
    let n = v.number()?;
    let t = match v.unit {
        Unit::W | Unit::KW => "IFCPOWERMEASURE",
        Unit::Ls | Unit::Lmin | Unit::M3h => "IFCVOLUMETRICFLOWRATEMEASURE",
        Unit::Kgh => "IFCMASSFLOWRATEMEASURE",
        Unit::Ms => "IFCLINEARVELOCITYMEASURE",
        Unit::Pa | Unit::KPa => "IFCPRESSUREMEASURE",
        Unit::C => "IFCTHERMODYNAMICTEMPERATUREMEASURE",
        Unit::Mm | Unit::M => "IFCLENGTHMEASURE",
        Unit::M2 => "IFCAREAMEASURE",
        Unit::M3 => "IFCVOLUMEMEASURE",
        Unit::A => "IFCELECTRICCURRENTMEASURE",
        Unit::V => "IFCELECTRICVOLTAGEMEASURE",
        Unit::Pct => "IFCRATIOMEASURE",
        Unit::Wm2 => "IFCHEATFLUXDENSITYMEASURE",
        Unit::DbA => "IFCSOUNDPRESSURELEVELMEASURE",
        Unit::None | Unit::PaM | Unit::PerH => "IFCREAL",
    };
    Some(Prop::Measure(t, v.unit.si(n)))
}

/// A value for the calculation's own set: its name with the unit, the
/// number as the calculation gives it.
fn own(v: &CalcValue) -> (String, Prop) {
    match &v.value {
        Val::Num(n) => {
            let name = match v.unit {
                Unit::None => v.label.clone(),
                u => format!("{} [{}]", v.label, u.text()),
            };
            let digits = v.unit.decimals() as i32 + 1;
            let f = 10f64.powi(digits);
            (name, Prop::Real((n * f).round() / f))
        }
        Val::Text(t) => (v.label.clone(), Prop::Label(t.clone())),
        Val::Flag(b) => (v.label.clone(), Prop::Bool(*b)),
    }
}

/// The name of a calculation's own set: "CreaBIM_TepelneStraty".
pub(crate) fn pset_name(title: &str) -> String {
    let mut out = String::from("CreaBIM_");
    let mut up = true;
    for ch in title.chars() {
        let plain = match ch {
            'á' | 'ä' => 'a',
            'č' => 'c',
            'ď' => 'd',
            'é' | 'ě' => 'e',
            'í' => 'i',
            'ĺ' | 'ľ' => 'l',
            'ň' => 'n',
            'ó' | 'ô' => 'o',
            'ŕ' | 'ř' => 'r',
            'š' => 's',
            'ť' => 't',
            'ú' | 'ů' => 'u',
            'ý' => 'y',
            'ž' => 'z',
            'Á' | 'Ä' => 'A',
            'Č' => 'C',
            'Ď' => 'D',
            'É' => 'E',
            'Í' => 'I',
            'Ľ' | 'Ĺ' => 'L',
            'Ň' => 'N',
            'Ó' | 'Ô' => 'O',
            'Š' => 'S',
            'Ť' => 'T',
            'Ú' => 'U',
            'Ý' => 'Y',
            'Ž' => 'Z',
            c => c,
        };
        if plain.is_ascii_alphanumeric() {
            out.push(if up {
                plain.to_ascii_uppercase()
            } else {
                plain
            });
            up = false;
        } else {
            up = true;
        }
    }
    out
}

/// A calculation's own set of `values`, with its standard and when it ran.
fn own_set(c: &mut Ctx, on: &[Ref], set: &ResultSet, values: &[CalcValue]) {
    let mut props: Vec<(String, Prop)> = values.iter().map(own).collect();
    if !set.norm.is_empty() {
        props.push(("Norma".into(), Prop::Label(set.norm.clone())));
    }
    if !set.at.is_empty() {
        props.push(("Vypočítané".into(), Prop::Label(set.at.clone())));
    }
    // a name twice (two networks of one pipe) keeps the first
    let mut seen: Vec<String> = vec![];
    props.retain(|(k, _)| {
        let fresh = !seen.contains(k);
        seen.push(k.clone());
        fresh
    });
    c.pset_on(on, &pset_name(&set.title), &props);
}

fn find<'a>(
    sets: &'a [(&ResultSet, &[CalcValue])],
    calc: &str,
    key: &str,
) -> Option<&'a CalcValue> {
    sets.iter()
        .filter(|(s, _)| s.calc == calc)
        .flat_map(|(_, v)| v.iter())
        .find(|v| v.key == key)
}

/// IFC's predefined type of a distribution system, and the user-defined
/// type when IFC has none.
fn system_type(kind: creabim_core::mep::SystemKind) -> (&'static str, Option<&'static str>) {
    use creabim_core::mep::SystemKind as K;
    match kind {
        K::ColdWater => ("DOMESTICCOLDWATER", None),
        K::HotWater | K::Circulation => ("DOMESTICHOTWATER", None),
        K::Sanitary => ("SEWAGE", None),
        K::Storm => ("RAINWATER", None),
        K::FireWater => ("FIREPROTECTION", None),
        K::Gas => ("GAS", None),
        K::HeatingSupply | K::HeatingReturn => ("HEATING", None),
        K::CoolingSupply | K::CoolingReturn => ("CHILLEDWATER", None),
        K::MedGas => ("USERDEFINED", Some("MEDICALGAS")),
        K::Flue => ("EXHAUST", None),
        K::AirSupply | K::AirExhaust => ("VENTILATION", None),
        K::Other => ("NOTDEFINED", None),
    }
}

/// The services' systems and circuits, and every calculation's results.
pub(super) fn write(c: &mut Ctx, model: &Model, building: Ref) {
    let systems = systems(c, model, building);
    circuits(c, model, building);
    let all = creabim_core::results::results(model);
    // by element, every calculation's values
    let mut by_element: HashMap<ElementId, Vec<(&ResultSet, &[CalcValue])>> = HashMap::new();
    for set in &all.sets {
        for (id, values) in &set.elements {
            by_element.entry(*id).or_default().push((set, values));
        }
        for (code, values) in &set.systems {
            let target = if code.is_empty() {
                Some(building)
            } else {
                systems.get(code.as_str()).copied()
            };
            if let Some(r) = target {
                own_set(c, &[r], set, values);
            }
        }
    }
    let mut ids: Vec<ElementId> = by_element.keys().copied().collect();
    ids.sort();
    for id in ids {
        let (Some(&r), Some(e)) = (c.products.get(&id), model.get(id)) else {
            continue;
        };
        let sets = &by_element[&id];
        for (set, values) in sets {
            own_set(c, &[r], set, values);
        }
        match &e.kind {
            ElementKind::Room(_) => space_thermal(c, r, sets),
            ElementKind::Valve(_) => {
                if let Some(kv) = sets
                    .iter()
                    .flat_map(|(_, v)| v.iter())
                    .find(|v| v.key == "kv")
                    .and_then(CalcValue::number)
                {
                    c.pset_on(
                        &[r],
                        "Pset_ValveTypeCommon",
                        &[("FlowCoefficient", Prop::Real(kv))],
                    );
                }
            }
            ElementKind::Object(_) => {
                // a radiator's output, as the heating calculation has it
                if let Some(p) = find(sets, "heating", "power").and_then(measure) {
                    c.pset_on(&[r], "Pset_SpaceHeaterTypeCommon", &[("OutputCapacity", p)]);
                }
            }
            _ => {}
        }
    }
}

/// A room's heat loss, heat gain, temperature and air, where IFC keeps
/// them.
fn space_thermal(c: &mut Ctx, r: Ref, sets: &[(&ResultSet, &[CalcValue])]) {
    let mut props: Vec<(&'static str, Prop)> = vec![];
    let mut put = |name: &'static str, v: Option<&CalcValue>| {
        if let Some(p) = v.and_then(measure) {
            props.push((name, p));
        }
    };
    put("TotalHeatLoss", find(sets, "heatloss", "phi_hl"));
    put("HeatingDryBulb", find(sets, "heatloss", "ti"));
    put("TotalHeatGain", find(sets, "cooling", "phi_c"));
    put(
        "VentilationAirFlowrate",
        find(sets, "air", "air_supply").or(find(sets, "heatloss", "v_min")),
    );
    put("ExhaustAirFlowrate", find(sets, "air", "air_exhaust"));
    if !props.is_empty() {
        c.pset_on(&[r], "Pset_SpaceThermalDesign", &props);
    }
}

/// Each system pipes are drawn in, grouping its pipes, ducts and valves,
/// with each segment's diameters and length; by code.
fn systems<'a>(c: &mut Ctx, model: &'a Model, building: Ref) -> HashMap<&'a str, Ref> {
    let mut members: Vec<(&str, Vec<Ref>)> = vec![];
    for e in model.iter() {
        let (pipe, own) = match &e.kind {
            ElementKind::Pipe(p) => (p, true),
            ElementKind::Valve(v) => match v.host(model) {
                Some(p) => (p, false),
                None => continue,
            },
            _ => continue,
        };
        let Some(&r) = c.products.get(&e.id) else {
            continue;
        };
        if own {
            segment(c, r, pipe);
        }
        match members.iter_mut().find(|(code, _)| *code == pipe.system) {
            Some((_, v)) => v.push(r),
            None => members.push((pipe.system.as_str(), vec![r])),
        }
    }
    let mut out = HashMap::new();
    for (code, refs) in members {
        let def = creabim_core::mep::system(code);
        let (kind, user) =
            system_type(def.map_or(creabim_core::mep::SystemKind::Other, |d| d.kind));
        let long = def.map_or_else(|| "$".to_string(), |d| string(d.name));
        let object_type = user.map_or_else(|| "$".to_string(), string);
        let s = c.w.add(
            "IFCDISTRIBUTIONSYSTEM",
            format!(
                "{},$,{},$,{object_type},{long},.{kind}.",
                c.guid(),
                string(code)
            ),
        );
        let g = c.guid();
        c.w.add(
            "IFCRELASSIGNSTOGROUP",
            format!("{g},$,$,$,{},$,{s}", list(&refs)),
        );
        let g = c.guid();
        c.w.add(
            "IFCRELSERVICESBUILDINGS",
            format!("{g},$,$,$,{s},({building})"),
        );
        out.insert(code, s);
    }
    out
}

/// A pipe's or duct's diameters and length, in the standard sets.
fn segment(c: &mut Ctx, r: Ref, p: &creabim_core::mep::Pipe) {
    let mm = |v: f64| Prop::Measure("IFCLENGTHMEASURE", v / 1000.0);
    if p.kind().is_air() {
        let mut props = vec![("NominalDiameterOrWidth", mm(p.nominal_mm))];
        if let Some(h) = p.height_mm {
            props.push(("NominalHeight", mm(h)));
        }
        c.pset_on(&[r], "Pset_DuctSegmentTypeCommon", &props);
    } else {
        let mut props = vec![("NominalDiameter", mm(p.nominal_mm))];
        if p.inner_mm > 0.0 {
            props.push(("InnerDiameter", mm(p.inner_mm)));
        }
        if p.outer_mm > 0.0 {
            props.push(("OuterDiameter", mm(p.outer_mm)));
        }
        c.pset_on(&[r], "Pset_PipeSegmentTypeCommon", &props);
    }
    let q = c.w.add(
        "IFCQUANTITYLENGTH",
        format!("'Length',$,$,{},$", crate::step::real(p.length())),
    );
    let name = if p.kind().is_air() {
        "Qto_DuctSegmentBaseQuantities"
    } else {
        "Qto_PipeSegmentBaseQuantities"
    };
    let g = c.guid();
    let set = c.w.add(
        "IFCELEMENTQUANTITY",
        format!("{g},$,{},$,$,({q})", string(name)),
    );
    let g = c.guid();
    c.w.add(
        "IFCRELDEFINESBYPROPERTIES",
        format!("{g},$,$,$,({r}),{set}"),
    );
}

/// Each circuit of the electrical installation, grouping its devices,
/// with its breaker and cable.
fn circuits(c: &mut Ctx, model: &Model, building: Ref) {
    for (e, k) in creabim_core::electrical::circuits(model) {
        let r = c.w.add(
            "IFCDISTRIBUTIONCIRCUIT",
            format!(
                "{},$,{},$,$,$,.ELECTRICAL.",
                string(&e.id.ifc_guid()),
                string(&e.name)
            ),
        );
        c.products.insert(e.id, r);
        let devices: Vec<Ref> = k
            .loads
            .iter()
            .filter_map(|l| c.products.get(&l.device).copied())
            .collect();
        if !devices.is_empty() {
            let g = c.guid();
            c.w.add(
                "IFCRELASSIGNSTOGROUP",
                format!("{g},$,$,$,{},$,{r}", list(&devices)),
            );
        }
        let g = c.guid();
        c.w.add(
            "IFCRELSERVICESBUILDINGS",
            format!("{g},$,$,$,{r},({building})"),
        );
        let mut props: Vec<(&'static str, Prop)> = vec![(
            "NumberOfPhases",
            Prop::Count(if k.three_phase { 3 } else { 1 }),
        )];
        if let Some(b) = k.board.and_then(|b| model.get(b)) {
            props.push(("Rozvádzač", Prop::Label(b.name.clone())));
        }
        if let Some(a) = k.breaker_a {
            props.push(("Istič In [A]", Prop::Real(a)));
        }
        if let Some(cable) = k.cable() {
            props.push(("Kábel", Prop::Label(cable)));
        }
        if let Some(p) = k.phase {
            props.push(("Fáza", Prop::Label(format!("L{p}"))));
        }
        if let Some(ma) = k.rcd_ma {
            props.push(("Prúdový chránič IΔn [mA]", Prop::Real(ma)));
        }
        c.pset_on(&[r], "CreaBIM_Obvod", &props);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_names_are_plain() {
        assert_eq!(pset_name("Tepelné straty"), "CreaBIM_TepelneStraty");
        assert_eq!(
            pset_name("Ústredné vykurovanie"),
            "CreaBIM_UstredneVykurovanie"
        );
        assert_eq!(pset_name("Cirkulácia TV"), "CreaBIM_CirkulaciaTV");
    }
}
