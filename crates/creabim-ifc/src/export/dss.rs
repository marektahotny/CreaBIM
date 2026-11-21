//! The data standard for buildings ([`creabim_core::dss`]) in the IFC
//! file: the standard as an `IfcClassification`, each class used as an
//! `IfcClassificationReference` (its code and its Slovak name) associated
//! with every element, pipe system and the building that belongs to it,
//! and each one's values in the property set `DSS`, by the standard's
//! property names in Slovak, each with its unit in the description.

use super::Ctx;
use crate::step::{list, string, Ref};
use creabim_core::dss::{self, Filled, Value};
use creabim_core::Model;
use std::collections::{BTreeMap, HashMap};

/// The property set the values go in.
pub const PSET: &str = "DSS";

fn real(v: f64) -> String {
    let s = format!("{v:.6}");
    let s = s.trim_end_matches('0');
    if s.ends_with('.') {
        format!("{s}0")
    } else {
        s.to_string()
    }
}

fn value_step(v: &Value) -> String {
    match v {
        Value::Num(n) => format!("IFCREAL({})", real(*n)),
        Value::Int(n) => format!("IFCINTEGER({n})"),
        Value::Bool(b) => format!("IFCBOOLEAN({})", if *b { ".T." } else { ".F." }),
        Value::Text(t) if t.chars().count() > 255 => format!("IFCTEXT({})", string(t)),
        Value::Text(t) => format!("IFCLABEL({})", string(t)),
    }
}

/// The set of the values known, on `object`; nothing when none is.
fn values_on(c: &mut Ctx, object: Ref, class: &dss::Class, props: &[Filled]) {
    let mut refs: Vec<Ref> = vec![];
    // the class itself, so it reads without a classification browser
    refs.push(c.w.add(
        "IFCPROPERTYSINGLEVALUE",
        format!(
            "{},$,IFCLABEL({}),$",
            string("trieda DSS"),
            string(&class.title())
        ),
    ));
    for f in props {
        let Some((v, source)) = &f.value else {
            continue;
        };
        if *source == dss::Source::NotApplicable {
            continue;
        }
        let unit = match v {
            Value::Num(_) => f.prop.unit(),
            _ => "",
        };
        let desc = if unit.is_empty() {
            "$".to_string()
        } else {
            string(unit)
        };
        refs.push(c.w.add(
            "IFCPROPERTYSINGLEVALUE",
            format!("{},{desc},{},$", string(f.prop.name), value_step(v)),
        ));
    }
    let g = c.guid();
    let set = c.w.add(
        "IFCPROPERTYSET",
        format!(
            "{g},$,{},{},{}",
            string(PSET),
            string("Dátový štandard stavieb (DSS)"),
            list(&refs)
        ),
    );
    let g = c.guid();
    c.w.add(
        "IFCRELDEFINESBYPROPERTIES",
        format!("{g},$,$,$,({object}),{set}"),
    );
}

/// The classes and values of every element written, of the pipe systems
/// (`systems`, by code) and of the building.
pub(super) fn write(c: &mut Ctx, model: &Model, building: Ref, systems: &HashMap<String, Ref>) {
    let cx = dss::Ctx::new(model);
    // by class code, the objects in it
    let mut members: BTreeMap<&'static str, Vec<Ref>> = BTreeMap::new();
    let mut ids: Vec<_> = c.products.keys().copied().collect();
    ids.sort();
    for id in ids {
        let Some(e) = model.get(id) else { continue };
        if !dss::is_classifiable(e) {
            continue;
        }
        let r = c.products[&id];
        if let Some((class, props)) = dss::filled(model, &cx, e) {
            members.entry(class.code).or_default().push(r);
            values_on(c, r, class, &props);
        }
    }
    let mut codes: Vec<&String> = systems.keys().collect();
    codes.sort();
    for code in codes {
        let r = systems[code];
        if let Some((class, props)) = dss::filled_system(model, &cx.data, code) {
            members.entry(class.code).or_default().push(r);
            values_on(c, r, class, &props);
        }
    }
    if let Some((class, props)) = dss::filled_building(model, &cx.data) {
        members.entry(class.code).or_default().push(building);
        values_on(c, building, class, &props);
    }
    if members.is_empty() {
        return;
    }
    let source = c.w.add(
        "IFCCLASSIFICATION",
        format!(
            "{},{},$,{},{},{},$",
            string("Česká agentúra pre štandardizáciu (comBIMe)"),
            string("2026-10-09"),
            string("DSS"),
            string("Dátový štandard stavieb, triedy preložené do slovenčiny"),
            string("https://dss.koncepcebim.gov.cz"),
        ),
    );
    for (code, objects) in members {
        let Some(class) = dss::class(code) else {
            continue;
        };
        let reference = c.w.add(
            "IFCCLASSIFICATIONREFERENCE",
            format!(
                "$,{},{},{source},{},$",
                string(class.code),
                string(class.name),
                string(class.group.label())
            ),
        );
        let g = c.guid();
        c.w.add(
            "IFCRELASSOCIATESCLASSIFICATION",
            format!("{g},$,$,$,{},{reference}", list(&objects)),
        );
    }
}
