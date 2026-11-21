//! Classification by the data standard for buildings (Datový standard
//! staveb, DSS, as the Czech Agency for Standardisation publishes it in
//! comBIMe): which data template (class) each element belongs to and
//! which properties that class asks for in each phase of the project.
//!
//! The classes and properties CreaBIM's elements fall into are kept in
//! `triedy.tsv` and `vlastnosti.tsv`, translated to Slovak. Each element
//! gets its class from what it is (a load-bearing wall, a WC pan, a cold
//! water pipe...); the designer can pick another one. Values the model
//! knows (lengths, areas, DN, the system, calculation results) are filled
//! in by themselves; the rest are typed in and kept in one element of
//! the model ([`DssData`]), like the calculation results.

use crate::{ElementId, ElementKind, Model, Transaction};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// What a property's value is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PropKind {
    Text,
    Number,
    Integer,
    Bool,
    Date,
}

impl PropKind {
    pub fn label(self) -> &'static str {
        match self {
            PropKind::Text => "text",
            PropKind::Number => "číslo",
            PropKind::Integer => "celé číslo",
            PropKind::Bool => "áno/nie",
            PropKind::Date => "dátum",
        }
    }
}

/// A property of the standard.
#[derive(Clone, Debug, PartialEq)]
pub struct Prop {
    pub index: usize,
    /// Its name, in Slovak ("menovitý priemer").
    pub name: &'static str,
    pub kind: PropKind,
    /// The units the standard allows, the first one CreaBIM writes.
    pub units: Vec<&'static str>,
}

impl Prop {
    /// The unit CreaBIM writes it in: metres, square and cubic metres
    /// where the standard allows them, else its first.
    pub fn unit(&self) -> &'static str {
        // diameters, bores and thicknesses in millimetres, as drawn
        let small = ["priemer", "svetlosť", "hrúbka"]
            .iter()
            .any(|w| self.name.contains(w));
        if small && self.units.contains(&"mm") {
            return "mm";
        }
        ["m", "m²", "m³"]
            .into_iter()
            .find(|u| self.units.contains(u))
            .or_else(|| self.units.first().copied())
            .unwrap_or("")
    }
}

/// Which part of the standard a class is from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    /// A construction entity: the building.
    Entity,
    /// A built space: a room.
    Space,
    /// A technical system: a pipe network, the wiring.
    System,
    /// A component: a wall, a door, a valve, a WC pan...
    Component,
}

impl Group {
    pub fn label(self) -> &'static str {
        match self {
            Group::Entity => "Stavebné entity",
            Group::Space => "Vybudované priestory",
            Group::System => "Technické systémy",
            Group::Component => "Komponenty",
        }
    }
}

/// A phase of the project the standard asks for properties in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// For the building permit.
    Permit,
    /// For building it.
    Build,
    /// For running it.
    Operate,
}

impl Phase {
    pub const ALL: [Phase; 3] = [Phase::Permit, Phase::Build, Phase::Operate];
    pub fn label(self) -> &'static str {
        match self {
            Phase::Permit => "povolenie",
            Phase::Build => "realizácia",
            Phase::Operate => "užívanie",
        }
    }
}

/// The phases a stage of documentation answers for (George, 2026-11-19):
/// the building intention and the notified project for the permit, the
/// project and the execution project for building, the intention with
/// the project both. Without a stage, the permit's.
pub fn phases_of_stage(code: Option<&str>) -> Vec<Phase> {
    match code {
        Some("SZP") | Some("PSO") => vec![Phase::Permit],
        Some("PSP") | Some("VPP") => vec![Phase::Build],
        Some("SZP/PSP") => vec![Phase::Permit, Phase::Build],
        _ => vec![Phase::Permit],
    }
}

/// The project's phases, from the stage its information names.
pub fn project_phases(model: &Model) -> Vec<Phase> {
    let code = model
        .project_info()
        .and_then(|(_, p)| crate::project::stage_code(&p.status));
    phases_of_stage(code)
}

/// A class (data template) of the standard.
#[derive(Clone, Debug, PartialEq)]
pub struct Class {
    /// Its code ("ULM001").
    pub code: &'static str,
    pub group: Group,
    /// The IFC entity and predefined type the standard pairs it with, as
    /// one word ("IfcWallPARTITIONING"); empty when it names none.
    pub ifc: &'static str,
    /// Its name, in Slovak.
    pub name: &'static str,
    /// The template's own properties.
    pub props: Vec<usize>,
    /// The properties asked for in each phase.
    pub permit: Vec<usize>,
    pub build: Vec<usize>,
    pub operate: Vec<usize>,
}

impl Class {
    /// The IFC entity and the predefined type, if one is named.
    pub fn ifc_entity(&self) -> (&'static str, Option<&'static str>) {
        let s = self.ifc;
        // the predefined type is the run of capitals at the end
        let cut = s
            .char_indices()
            .collect::<Vec<_>>()
            .windows(2)
            .find(|w| w[0].1.is_ascii_uppercase() && w[1].1.is_ascii_uppercase())
            .map(|w| w[0].0);
        match cut {
            Some(i) if i > 3 => (&s[..i], Some(&s[i..])),
            _ => (s, None),
        }
    }

    /// The properties asked for in `phases`, each once, in the
    /// standard's order.
    pub fn required(&self, phases: &[Phase]) -> Vec<&'static Prop> {
        let mut out: Vec<usize> = Vec::new();
        for ph in phases {
            let list = match ph {
                Phase::Permit => &self.permit,
                Phase::Build => &self.build,
                Phase::Operate => &self.operate,
            };
            for i in list {
                if !out.contains(i) {
                    out.push(*i);
                }
            }
        }
        out.into_iter().filter_map(prop).collect()
    }

    /// Every property it knows: its own and those of every phase.
    pub fn all_props(&self) -> Vec<&'static Prop> {
        let mut out: Vec<usize> = self.props.clone();
        for i in self.permit.iter().chain(&self.build).chain(&self.operate) {
            if !out.contains(i) {
                out.push(*i);
            }
        }
        out.into_iter().filter_map(prop).collect()
    }

    /// "ULM001 stena nosná".
    pub fn title(&self) -> String {
        format!("{} {}", self.code, self.name)
    }
}

struct Tables {
    props: Vec<Prop>,
    classes: Vec<Class>,
}

fn list(s: &str) -> Vec<usize> {
    s.split(',').filter_map(|t| t.trim().parse().ok()).collect()
}

fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(|| {
        let mut props = Vec::new();
        for line in include_str!("vlastnosti.tsv").lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let f: Vec<&'static str> = line.split('\t').collect();
            let Some(index) = f.first().and_then(|s| s.parse().ok()) else {
                continue;
            };
            let kind = match f.get(2).copied().unwrap_or("t") {
                "c" => PropKind::Number,
                "n" => PropKind::Integer,
                "b" => PropKind::Bool,
                "d" => PropKind::Date,
                _ => PropKind::Text,
            };
            let units = f
                .get(3)
                .map(|u| u.split(',').filter(|s| !s.is_empty()).collect())
                .unwrap_or_default();
            props.push(Prop {
                index,
                name: f.get(1).copied().unwrap_or(""),
                kind,
                units,
            });
        }
        let mut classes = Vec::new();
        for line in include_str!("triedy.tsv").lines() {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let f: Vec<&'static str> = line.split('\t').collect();
            let get = |i: usize| f.get(i).copied().unwrap_or("");
            let group = match get(1) {
                "E" => Group::Entity,
                "P" => Group::Space,
                "S" => Group::System,
                _ => Group::Component,
            };
            classes.push(Class {
                code: get(0),
                group,
                ifc: get(2),
                name: get(3),
                props: list(get(4)),
                permit: list(get(5)),
                build: list(get(6)),
                operate: list(get(7)),
            });
        }
        Tables { props, classes }
    })
}

/// Every class CreaBIM knows, in the standard's order.
pub fn classes() -> &'static [Class] {
    &tables().classes
}

/// The class with this code.
pub fn class(code: &str) -> Option<&'static Class> {
    classes().iter().find(|c| c.code == code)
}

/// The property with this index.
pub fn prop(index: usize) -> Option<&'static Prop> {
    tables().props.iter().find(|p| p.index == index)
}

/// The property with this name.
pub fn prop_named(name: &str) -> Option<&'static Prop> {
    tables().props.iter().find(|p| p.name == name)
}

// ---------------------------------------------------------------------
// what is typed in

/// One element's (or system's, or the building's) class picked by hand
/// and the values typed in.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// The class picked by hand: none takes the one it has by what it
    /// is; an empty code leaves it without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    /// The values typed in, by property name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<(String, String)>,
}

impl Entry {
    pub fn is_empty(&self) -> bool {
        self.class.is_none() && self.values.is_empty()
    }
    pub fn value(&self, name: &str) -> Option<&str> {
        self.values
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
    pub fn set_value(&mut self, name: &str, value: &str) {
        let value = value.trim();
        self.values.retain(|(n, _)| n != name);
        if !value.is_empty() {
            self.values.push((name.to_string(), value.to_string()));
        }
    }
}

/// Everything typed in for the standard (at most one per project).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DssData {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elements: Vec<(ElementId, Entry)>,
    /// By the code the system's pipes are drawn in ("SV").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub systems: Vec<(String, Entry)>,
    #[serde(default, skip_serializing_if = "Entry::is_empty")]
    pub building: Entry,
}

impl DssData {
    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }
    pub fn of(&self, id: ElementId) -> Option<&Entry> {
        self.elements.iter().find(|(e, _)| *e == id).map(|(_, v)| v)
    }
    pub fn of_mut(&mut self, id: ElementId) -> &mut Entry {
        if let Some(i) = self.elements.iter().position(|(e, _)| *e == id) {
            return &mut self.elements[i].1;
        }
        self.elements.push((id, Entry::default()));
        &mut self.elements.last_mut().unwrap().1
    }
    pub fn system(&self, code: &str) -> Option<&Entry> {
        self.systems.iter().find(|(c, _)| c == code).map(|(_, v)| v)
    }
    pub fn system_mut(&mut self, code: &str) -> &mut Entry {
        if let Some(i) = self.systems.iter().position(|(c, _)| c == code) {
            return &mut self.systems[i].1;
        }
        self.systems.push((code.to_string(), Entry::default()));
        &mut self.systems.last_mut().unwrap().1
    }
    fn tidy(&mut self, model: &Model) {
        self.elements
            .retain(|(id, e)| !e.is_empty() && model.get(*id).is_some());
        self.systems.retain(|(_, e)| !e.is_empty());
    }
}

/// The project's typed-in data (empty when there is none yet).
pub fn data(model: &Model) -> DssData {
    model
        .iter()
        .find_map(|e| match &e.kind {
            ElementKind::Dss(d) => Some(d.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// The edit that puts `new` in place of the project's typed-in data;
/// nothing when it is the same.
pub fn write(model: &Model, mut new: DssData, label: &str) -> Transaction {
    let tx = Transaction::new(label.to_string());
    new.tidy(model);
    let current = model.iter().find(|e| matches!(e.kind, ElementKind::Dss(_)));
    match current {
        Some(e) => {
            if let ElementKind::Dss(d) = &e.kind {
                if *d == new {
                    return tx;
                }
            }
            let mut after = e.clone();
            after.kind = ElementKind::Dss(new);
            tx.replace(e.clone(), after)
        }
        None if new == DssData::default() => tx,
        None => tx.insert(crate::Element::new(
            "Dátový štandard (DSS)",
            ElementKind::Dss(new),
        )),
    }
}

// ---------------------------------------------------------------------
// the class an element has by what it is

/// A pipe system's name in Slovak, by its code.
pub fn system_name(code: &str) -> String {
    match code {
        "SV" => "Studená voda",
        "TV" => "Teplá voda",
        "C" => "Cirkulácia teplej vody",
        "K" => "Splašková kanalizácia",
        "D" => "Dažďová kanalizácia",
        "PV" => "Požiarna voda – hydranty",
        "ZP" => "Plyn – zemný plyn",
        "UKP" => "Vykurovanie – prívod",
        "UKS" => "Vykurovanie – spiatočka",
        "CHP" => "Chladenie – prívod",
        "CHS" => "Chladenie – spiatočka",
        "O2" => "Kyslík O2",
        "AIR" => "Medicinálny vzduch AIR",
        "VAC" => "Vákuum VAC",
        "N2O" => "Oxid dusný N2O",
        "CO2" => "Oxid uhličitý CO2",
        "DYM" => "Dymovod – spaliny kotla",
        "VZP" => "VZT – prívod vzduchu",
        "VZO" => "VZT – odvod vzduchu",
        other => {
            return crate::mep::system(other).map_or(other.to_string(), |s| s.name.to_string())
        }
    }
    .to_string()
}

/// The class of a pipe system by its code.
pub fn system_class(code: &str) -> Option<&'static Class> {
    let kind = crate::mep::system(code).map(|s| s.kind)?;
    use crate::mep::SystemKind as K;
    let c = match kind {
        K::ColdWater => "JB01",
        K::HotWater => "JB02",
        K::Circulation => "JB03",
        K::Sanitary | K::Storm => "JD02",
        K::FireWater => "JB06",
        K::Gas => "JA01",
        K::HeatingSupply => "JG01",
        K::HeatingReturn => "JG02",
        K::CoolingSupply => "JF01",
        K::CoolingReturn => "JF02",
        K::Flue => "JE03",
        K::AirSupply => "JJ04",
        K::AirExhaust => "JJ05",
        K::MedGas | K::Other => return None,
    };
    class(c)
}

/// The class of a system, the hand-picked one first.
pub fn system_class_in(d: &DssData, code: &str) -> Option<&'static Class> {
    match d.system(code).and_then(|e| e.class.as_deref()) {
        Some(c) => class(c),
        None => system_class(code),
    }
}

fn by_ifc(entity: &str, predefined: &str) -> Option<&'static str> {
    let full = format!("{entity}{predefined}");
    classes()
        .iter()
        .filter(|c| c.group == Group::Component)
        .find(|c| c.ifc.eq_ignore_ascii_case(&full))
        .or_else(|| {
            classes()
                .iter()
                .filter(|c| c.group == Group::Component)
                .find(|c| c.ifc_entity().0.eq_ignore_ascii_case(entity))
        })
        .map(|c| c.code)
}

pub(crate) fn object_class(class: &str, predefined: &str, key: &str) -> Option<&'static str> {
    let key = key.to_ascii_lowercase();
    Some(match (class, predefined) {
        ("IfcSanitaryTerminal", "WASHHANDBASIN") => "XKA003",
        ("IfcSanitaryTerminal", "TOILETPAN") | ("IfcSanitaryTerminal", "WCSEAT") => "XKB001",
        ("IfcSanitaryTerminal", "URINAL") => "XKC001",
        ("IfcSanitaryTerminal", _) => "XKA001",
        ("IfcWasteTerminal", _) => "XKE001",
        ("IfcValve", "FAUCET") => "RNA003",
        // a medical gas zone valve box
        ("IfcValve", _) if key.contains("med-") => "RNB001",
        ("IfcValve", "GASCOCK") => "RNB001",
        ("IfcValve", "CHECK") => "RMA001",
        ("IfcValve", "REGULATING") | ("IfcValve", "PRESSUREREDUCING") => "RNA002",
        ("IfcValve", _) => "RNA001",
        ("IfcFurniture", "BED") => "UVB001",
        ("IfcFurniture", "CHAIR") => "UVA001",
        ("IfcFurniture", "TABLE") | ("IfcFurniture", "DESK") => "UBE001",
        ("IfcFurniture", "SOFA") => "CLC001",
        // cars and people give scale; they are not part of the building
        ("IfcFurniture", "NOTDEFINED")
            if key.contains("car") || key.contains("person") || key.contains("child") =>
        {
            return None
        }
        ("IfcFurniture", _) => "CMB001",
        ("IfcElectricDistributionBoard", _) => "XDC010",
        ("IfcOutlet", _) => "XDB001",
        ("IfcSensor", "SMOKESENSOR") => "BQF002",
        ("IfcLightFixture", "SECURITYLIGHTING") => "EAA002",
        ("IfcLightFixture", _) => "EAA001",
        // a gas boiler (the catalogue's are condensing gas boilers)
        ("IfcBoiler", _) if key.contains("elektr") => "EBA003",
        ("IfcBoiler", _) => "EMB002",
        ("IfcFan", _) => "GQB002",
        ("IfcSpaceHeater", "RADIATOR") => "EPC001",
        ("IfcTank", _) if key.contains("zasobnik") => "CPA001",
        ("IfcTank", _) if key.contains("retention") || key.contains("soakaway") => "CMA008",
        ("IfcTank", _) => "CMB003",
        // a pumping shaft: the standard has no class for it
        ("IfcDistributionChamberElement", "SUMP") if key.contains("pumping") => return None,
        ("IfcDistributionChamberElement", "SUMP") => "CMA008",
        ("IfcDistributionChamberElement", _) => "XMC001",
        ("IfcUnitaryEquipment", _) if key.contains("heat-tc") => "EGA002",
        ("IfcUnitaryEquipment", "SPLITSYSTEM") => "EQD001",
        ("IfcFlowMeter", "GASMETER") => "BFA006",
        ("IfcFlowMeter", "WATERMETER") => "BFA002",
        ("IfcMedicalDevice", _) => "XZA003",
        _ => return by_ifc(class, predefined),
    })
}

fn valve_class(code: &str) -> Option<&'static str> {
    Some(match code {
        "uzaver" | "rohovy" | "spiatocka" | "krabica" => "RNA001",
        "spatna" => "RMA001",
        "filter" | "filterVzt" => "HQB003",
        "redukcny" | "vyvazovaci" | "termostaticky" => "RNA002",
        "rtd" => "KHA001",
        "vodomer" => "BFA002",
        "kohut" | "hup" => "RNB001",
        "plynomer" => "BFA006",
        "regulator" => "RNB010",
        "tap" => "RNB011",
        "spotrebic" => "EMA004",
        "cistiaci" => "XMA012",
        "vpust" => "XKE001",
        "hydrant" => "RNA006",
        "vyustP" | "vyustO" => "RNB008",
        "klapkaReg" => "RNB006",
        "klapkaPoz" => "FMA001",
        "tlmic" => "RQC002",
        "vyvod" => "XZA003",
        _ => return None,
    })
}

fn room_class(rt: crate::roomtype::RoomType) -> Option<&'static str> {
    use crate::roomtype::RoomType as R;
    Some(match rt {
        R::Living => "AAB01",
        R::Bedroom | R::Children => "AAA01",
        R::Dining => "BEA01",
        R::Kitchen => "BBA02",
        R::Bathroom => "ABA01",
        R::Toilet => "ABB01",
        R::Hall => "EAC01",
        R::Staircase => "EAD01",
        R::Wardrobe => "ABD01",
        R::Pantry => "CAG01",
        R::Laundry => "BAG01",
        R::Utility => "DAC01",
        R::Garage => "CBD01",
        R::Office => "BAA01",
        R::Ward => "BAH01",
        _ => return None,
    })
}

/// The class an element has by what it is, if the standard has one for
/// it.
pub fn auto_class(model: &Model, e: &crate::Element) -> Option<&'static Class> {
    let code = match &e.kind {
        ElementKind::Wall(w) if w.curtain.is_some() => None,
        ElementKind::Wall(w) => {
            let rules = crate::masonry::rules(model);
            Some(if crate::masonry::is_bearing(model, w, &rules) {
                "ULM001"
            } else {
                "NAB003"
            })
        }
        ElementKind::Slab(_) => Some("ULK001"),
        ElementKind::Opening(o) => match o.kind {
            crate::OpeningKind::Door => Some("QQC001"),
            crate::OpeningKind::Window => {
                let in_roof = model
                    .get(o.host)
                    .is_some_and(|h| matches!(h.kind, ElementKind::Roof(_)));
                Some(if in_roof {
                    "QQA003"
                } else if o.sill < 0.05 && o.height >= 1.9 {
                    "QQB002"
                } else {
                    "QQA002"
                })
            }
            crate::OpeningKind::Hole => None,
        },
        ElementKind::Shaft(_) => Some("XTB001"),
        ElementKind::Column(_) => Some("ULD001"),
        ElementKind::Beam(_) => Some("ULE001"),
        ElementKind::Roof(r) => Some(match r.effective_shape() {
            crate::RoofShape::Flat => "NCE003",
            _ => "NCE004",
        }),
        ElementKind::Stair(_) => Some("XSB001"),
        ElementKind::Railing(_) => Some("FQD002"),
        ElementKind::Footing(f) => Some(match f.kind {
            crate::FootingKind::Strip => "ULC011",
            crate::FootingKind::Pad => "ULG004",
        }),
        ElementKind::Ceiling(_) => Some("NCD001"),
        ElementKind::Room(r) => room_class(crate::roomtype::RoomType::of(&r.occupancy, &e.name)),
        ElementKind::Site(s) => match &s.form {
            crate::SiteForm::Plant { form, .. } => Some(match form {
                crate::site::PlantForm::Shrub => "TRA001",
                _ => "TRA002",
            }),
            _ => None,
        },
        ElementKind::Pipe(p) => Some(if p.kind().is_air() && p.height_mm.is_some() {
            "WPB001"
        } else {
            "WPA001"
        }),
        ElementKind::Valve(v) => valve_class(&v.kind),
        ElementKind::Circuit(_) => Some("JK01"),
        ElementKind::Object(_) => crate::objects::type_of(model, e).and_then(|t| {
            object_class(
                &t.ifc.class,
                &t.ifc.predefined,
                t.catalogue.as_deref().unwrap_or(""),
            )
        }),
        ElementKind::Part(p) if !p.void => by_ifc(&p.ifc_class, ""),
        ElementKind::FreeMesh(m) => by_ifc(&m.ifc_class, ""),
        _ => None,
    };
    code.and_then(class)
}

/// An element's class: the one picked by hand, else its own by what it
/// is.
pub fn class_in(model: &Model, d: &DssData, e: &crate::Element) -> Option<&'static Class> {
    match d.of(e.id).and_then(|x| x.class.as_deref()) {
        Some(code) => class(code),
        None => auto_class(model, e),
    }
}

/// An element's class (looks the typed-in data up; for many elements
/// read [`data`] once and use [`class_in`]).
pub fn class_of(model: &Model, e: &crate::Element) -> Option<&'static Class> {
    class_in(model, &data(model), e)
}

/// The building's class: picked by hand, else a one-dwelling house.
pub fn building_class(d: &DssData) -> Option<&'static Class> {
    match d.building.class.as_deref() {
        Some(c) => class(c),
        None => class("AA01"),
    }
}

// ---------------------------------------------------------------------
// values

/// A value of a property.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// A number in the property's first unit.
    Num(f64),
    Int(i64),
    Text(String),
    Bool(bool),
}

impl Value {
    pub fn shown(&self, prop: &Prop) -> String {
        match self {
            Value::Num(v) => {
                let s = if (v.fract()).abs() < 1e-9 {
                    format!("{v:.0}")
                } else if v.abs() >= 100.0 {
                    format!("{v:.1}")
                } else {
                    format!("{v:.3}")
                        .trim_end_matches('0')
                        .trim_end_matches('.')
                        .to_string()
                };
                match prop.unit() {
                    "" => s,
                    u => format!("{s} {u}"),
                }
            }
            Value::Int(v) => v.to_string(),
            Value::Text(t) => t.clone(),
            Value::Bool(true) => "áno".into(),
            Value::Bool(false) => "nie".into(),
        }
    }

    /// A typed-in text as a value of `prop`; none when it doesn't read
    /// as one.
    pub fn parse(prop: &Prop, text: &str) -> Option<Value> {
        let t = text.trim();
        if t.is_empty() {
            return None;
        }
        match prop.kind {
            PropKind::Number => {
                let num: String = t
                    .replace(',', ".")
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                    .collect();
                num.parse().ok().map(Value::Num)
            }
            PropKind::Integer => t.parse().ok().map(Value::Int),
            PropKind::Bool => match t.to_lowercase().as_str() {
                "áno" | "ano" | "a" | "1" | "true" | "yes" => Some(Value::Bool(true)),
                "nie" | "ne" | "n" | "0" | "false" | "no" => Some(Value::Bool(false)),
                _ => None,
            },
            PropKind::Text | PropKind::Date => Some(Value::Text(t.to_string())),
        }
    }
}

/// Where a value came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Measured on the model or taken from it.
    Model,
    /// A calculation's result.
    Calc,
    /// Typed in.
    Typed,
    /// Not asked of this element (a fixed pane has no active leaf, a
    /// window or door that is no fire closure no fire rating): the value
    /// is the reason, and the IFC file leaves it out.
    NotApplicable,
}

/// A length in metres in a property's first unit.
fn length_in(prop: &Prop, metres: f64) -> f64 {
    match prop.unit() {
        "mm" => metres * 1000.0,
        "cm" => metres * 100.0,
        "km" => metres / 1000.0,
        _ => metres,
    }
}

fn area_in(prop: &Prop, m2: f64) -> f64 {
    match prop.unit() {
        "cm²" => m2 * 1e4,
        "mm²" => m2 * 1e6,
        "km²" => m2 / 1e6,
        _ => m2,
    }
}

fn volume_in(prop: &Prop, m3: f64) -> f64 {
    match prop.unit() {
        "cm³" => m3 * 1e6,
        "l" => m3 * 1000.0,
        _ => m3,
    }
}

/// What the model knows of an element, by property name: (name, value in
/// SI or as text, what it is).
enum Known {
    Len(f64),
    Area(f64),
    Vol(f64),
    /// Already in the property's unit.
    Raw(f64),
    Text(String),
    Bool(bool),
    /// Not asked of it, for this reason.
    NotApplicable(&'static str),
}

fn put(out: &mut Vec<(&'static str, Known, Source)>, name: &'static str, k: Known) {
    if !out.iter().any(|(n, _, _)| *n == name) {
        out.push((name, k, Source::Model));
    }
}

fn level_name(model: &Model, id: Option<ElementId>) -> Option<String> {
    id.and_then(|l| model.get(l)).map(|l| l.name.clone())
}

fn level_of_element(e: &crate::Element) -> Option<ElementId> {
    match &e.kind {
        ElementKind::Wall(w) => Some(w.level),
        ElementKind::Slab(s) => Some(s.level),
        ElementKind::Room(r) => Some(r.level),
        ElementKind::Ceiling(c) => Some(c.level),
        ElementKind::Object(o) => Some(o.level),
        k => crate::members::level_of(k),
    }
}

/// What is worked out once for the whole model: the take-off's
/// measures, the marks, the typed-in data and the project's phases.
pub struct Ctx {
    lines: std::collections::HashMap<ElementId, Vec<crate::takeoff::Line>>,
    marks: std::collections::HashMap<ElementId, String>,
    pub data: DssData,
    pub phases: Vec<Phase>,
}

impl Ctx {
    pub fn new(model: &Model) -> Self {
        let mut lines: std::collections::HashMap<ElementId, Vec<crate::takeoff::Line>> =
            Default::default();
        for l in crate::takeoff::lines(model) {
            lines.entry(l.element).or_default().push(l);
        }
        Self {
            lines,
            marks: crate::marks(model),
            data: data(model),
            phases: project_phases(model),
        }
    }
}

/// The values the model and the calculations give an element, before
/// anything typed in.
fn known(model: &Model, cx: &Ctx, e: &crate::Element) -> Vec<(&'static str, Known, Source)> {
    let mut out = Vec::new();
    // measured as the take-off measures it
    let lines: &[crate::takeoff::Line] = cx.lines.get(&e.id).map_or(&[], |v| v.as_slice());
    if !lines.is_empty() {
        let vol: f64 = lines.iter().map(|l| l.quantity.volume).sum();
        let area = lines.iter().map(|l| l.quantity.area).fold(0.0, f64::max);
        let len = lines.iter().map(|l| l.quantity.length).fold(0.0, f64::max);
        let thick: f64 = lines.iter().map(|l| l.thickness).sum();
        if vol > 0.0 {
            put(&mut out, "objem", Known::Vol(vol));
        }
        if area > 0.0 {
            put(&mut out, "plocha", Known::Area(area));
        }
        if len > 0.0 {
            put(&mut out, "dĺžka", Known::Len(len));
        }
        if thick > 0.0 && !matches!(e.kind, ElementKind::Opening(_) | ElementKind::Object(_)) {
            put(&mut out, "hrúbka", Known::Len(thick));
        }
    }
    if let Some((m, _)) = crate::marks::material_of(model, e) {
        put(&mut out, "materiál", Known::Text(m.name.clone()));
    }
    if let Some(l) = level_name(model, level_of_element(e)) {
        put(&mut out, "podlažie", Known::Text(l));
    }
    match &e.kind {
        ElementKind::Wall(w) => {
            put(&mut out, "výška", Known::Len(w.height));
            put(&mut out, "hrúbka", Known::Len(w.thickness));
            put(&mut out, "dĺžka", Known::Len(w.length()));
        }
        ElementKind::Slab(s) => {
            put(&mut out, "hrúbka", Known::Len(s.thickness));
            let mut per = 0.0;
            for i in 0..s.boundary.len() {
                per += (s.boundary[(i + 1) % s.boundary.len()] - s.boundary[i]).length();
            }
            if per > 0.0 {
                put(&mut out, "obvod", Known::Len(per));
            }
        }
        ElementKind::Opening(o) => {
            put(&mut out, "šírka", Known::Len(o.width));
            put(&mut out, "výška", Known::Len(o.height));
            if o.kind == crate::OpeningKind::Window {
                put(
                    &mut out,
                    "výška parapetu",
                    Known::Len(o.sill_from_floor(model)),
                );
            }
            if o.kind != crate::OpeningKind::Hole {
                joinery_values(&mut out, o);
            }
        }
        ElementKind::Room(r) => {
            put(&mut out, "názov miestnosti", Known::Text(e.name.clone()));
            let area = crate::room::area(model, e.id);
            if area > 0.0 {
                put(&mut out, "plocha", Known::Area(area));
            }
            let h = crate::room::height(model, r);
            if h > 0.0 {
                put(&mut out, "svetlá výška", Known::Len(h));
                put(&mut out, "výška", Known::Len(h));
                if area > 0.0 {
                    put(&mut out, "objem", Known::Vol(area * h));
                }
            }
            if !r.fire_zone.is_empty() {
                put(&mut out, "požiarny úsek", Known::Text(r.fire_zone.clone()));
            }
            if !r.number.is_empty() {
                put(&mut out, "označenie", Known::Text(r.number.clone()));
            }
        }
        ElementKind::Pipe(p) => {
            put(&mut out, "dĺžka", Known::Len(p.length()));
            put(&mut out, "označenie systému", Known::Text(p.system.clone()));
            if let Some(s) = p.system_def() {
                put(&mut out, "typ média", Known::Text(system_name(s.code)));
                put(&mut out, "typ systému", Known::Text(system_name(s.code)));
                put(&mut out, "druh systému", Known::Text(system_name(s.code)));
            }
            if !p.pipe_type.is_empty() {
                put(&mut out, "druh rúry", Known::Text(p.pipe_type.clone()));
                put(&mut out, "materiál", Known::Text(p.pipe_type.clone()));
            }
            match p.height_mm {
                Some(h) => {
                    put(
                        &mut out,
                        "rozmery prierezu",
                        Known::Text(format!("{:.0} × {:.0}", p.outer_mm, h)),
                    );
                    put(&mut out, "šírka", Known::Len(p.outer_mm / 1000.0));
                    put(&mut out, "výška", Known::Len(h / 1000.0));
                    put(
                        &mut out,
                        "plocha prierezu",
                        Known::Area(p.outer_mm * h / 1e6),
                    );
                }
                None => {
                    if p.nominal_mm > 0.0 {
                        put(&mut out, "menovitý priemer", Known::Raw(p.nominal_mm));
                        put(&mut out, "menovitá svetlosť", Known::Raw(p.nominal_mm));
                    }
                    if p.outer_mm > 0.0 {
                        put(&mut out, "vonkajší priemer", Known::Raw(p.outer_mm));
                        put(&mut out, "priemer", Known::Len(p.outer_mm / 1000.0));
                    }
                    if p.outer_mm > p.inner_mm && p.inner_mm > 0.0 {
                        put(
                            &mut out,
                            "hrúbka steny",
                            Known::Raw((p.outer_mm - p.inner_mm) / 2.0),
                        );
                    }
                }
            }
            let ins = p.insulation();
            put(&mut out, "tepelná izolácia", Known::Bool(ins > 0.0));
            if ins > 0.0 {
                put(&mut out, "hrúbka izolácie", Known::Len(ins / 1000.0));
            }
        }
        ElementKind::Valve(v) => {
            if let Some(pipe) = v.host(model) {
                put(
                    &mut out,
                    "označenie systému",
                    Known::Text(pipe.system.clone()),
                );
                if pipe.nominal_mm > 0.0 {
                    put(&mut out, "menovitý priemer", Known::Raw(pipe.nominal_mm));
                    put(&mut out, "menovitá svetlosť", Known::Raw(pipe.nominal_mm));
                }
                if let Some(s) = pipe.system_def() {
                    put(&mut out, "typ systému", Known::Text(system_name(s.code)));
                    put(&mut out, "typ média", Known::Text(system_name(s.code)));
                }
                if let Some(l) = level_name(model, Some(pipe.level)) {
                    put(&mut out, "podlažie", Known::Text(l));
                }
            }
            if let Some(k) = v.kind_def() {
                put(
                    &mut out,
                    "typové označenie",
                    Known::Text(k.name.to_string()),
                );
                put(&mut out, "druh armatúry", Known::Text(k.name.to_string()));
            }
        }
        ElementKind::Object(_) => {
            if let Some(t) = crate::objects::type_of(model, e) {
                let s = t.size();
                put(&mut out, "šírka", Known::Len(s.x));
                put(&mut out, "dĺžka", Known::Len(s.y));
                put(&mut out, "výška", Known::Len(s.z));
                put(
                    &mut out,
                    "rozmery",
                    Known::Text(format!(
                        "{:.0} × {:.0} × {:.0} mm",
                        s.x * 1000.0,
                        s.y * 1000.0,
                        s.z * 1000.0
                    )),
                );
            }
            if let Some(t) = e.of_type.as_ref().and_then(|t| model.get(t.id)) {
                put(&mut out, "typové označenie", Known::Text(t.name.clone()));
            }
        }
        ElementKind::Footing(f) => match f.kind {
            crate::FootingKind::Strip => {
                put(&mut out, "šírka", Known::Len(f.width));
                put(&mut out, "výška", Known::Len(f.thickness));
                put(
                    &mut out,
                    "plocha prierezu",
                    Known::Area(f.width * f.thickness),
                );
                put(&mut out, "dĺžka", Known::Len(f.start.distance(f.end)));
            }
            crate::FootingKind::Pad => {
                put(&mut out, "šírka", Known::Len(f.width));
                put(&mut out, "dĺžka", Known::Len(f.length));
                put(&mut out, "hrúbka", Known::Len(f.thickness));
                put(&mut out, "výška", Known::Len(f.thickness));
                put(&mut out, "obvod", Known::Len(2.0 * (f.width + f.length)));
                put(&mut out, "plocha", Known::Area(f.width * f.length));
                put(
                    &mut out,
                    "objem",
                    Known::Vol(f.width * f.length * f.thickness),
                );
            }
        },
        ElementKind::Column(c) => {
            put(&mut out, "výška", Known::Len(c.height_in(model)));
            match &c.section {
                Some(s) => section_values(&mut out, s, "šírka", "dĺžka"),
                None => {
                    put(&mut out, "plocha prierezu", Known::Area(c.profile.area()));
                    match c.profile {
                        crate::Profile::Rectangle { width, depth } => {
                            put(&mut out, "typ prierezu", Known::Text("obdĺžnikový".into()));
                            put(&mut out, "šírka", Known::Len(width));
                            put(&mut out, "obvod", Known::Len(2.0 * (width + depth)));
                        }
                        crate::Profile::Round { diameter } => {
                            put(&mut out, "typ prierezu", Known::Text("kruhový".into()));
                            put(&mut out, "šírka", Known::Len(diameter));
                            put(&mut out, "priemer", Known::Len(diameter));
                            put(
                                &mut out,
                                "obvod",
                                Known::Len(std::f64::consts::PI * diameter),
                            );
                        }
                    }
                    put(&mut out, "rozmery prierezu", Known::Text(c.profile.label()));
                }
            }
        }
        ElementKind::Beam(b) => {
            match &b.section {
                Some(s) => section_values(&mut out, s, "šírka", "výška prierezu"),
                None => {
                    put(&mut out, "typ prierezu", Known::Text("obdĺžnikový".into()));
                    put(&mut out, "šírka", Known::Len(b.width));
                    put(&mut out, "výška prierezu", Known::Len(b.depth));
                    put(&mut out, "plocha prierezu", Known::Area(b.width * b.depth));
                }
            }
            put(&mut out, "dĺžka", Known::Len(b.length()));
        }
        ElementKind::Stair(s) => {
            put(&mut out, "šírka", Known::Len(s.width));
            put(&mut out, "hrúbka", Known::Len(s.waist));
        }
        ElementKind::Railing(r) => {
            let len = r.length(model);
            put(&mut out, "výška", Known::Len(r.height));
            if len > 0.0 {
                put(&mut out, "dĺžka", Known::Len(len));
                put(&mut out, "plocha", Known::Area(len * r.height));
            }
            put(&mut out, "šírka", Known::Len(crate::members::RAIL_SIZE));
        }
        ElementKind::Site(s) => {
            if let crate::SiteForm::Plant { height, canopy, .. } = &s.form {
                put(&mut out, "výška", Known::Len(*height));
                put(&mut out, "šírka", Known::Len(*canopy));
            }
        }
        _ => {}
    }
    if let Some(m) = cx.marks.get(&e.id) {
        put(&mut out, "označenie", Known::Text(m.clone()));
    }
    // U of what a wall, slab or roof is built of (STN EN ISO 6946)
    if let Some(u) = crate::thermal::of_element(model, e).and_then(|t| t.u) {
        out.push((
            "súčiniteľ prechodu tepla – navrhnutý",
            Known::Raw(u),
            Source::Calc,
        ));
    }
    // the calculations' results
    for (_, values) in crate::results::of(model, e.id) {
        for v in values {
            let Some(n) = v.number() else { continue };
            let is_air = matches!(&e.kind, ElementKind::Pipe(p) if p.kind().is_air());
            let (name, val) = match v.key.as_str() {
                "phi_hl" => ("tepelná strata", to_kw(n, v.unit)),
                "phi_c" => ("tepelná záťaž", to_kw(n, v.unit)),
                "ti" => ("teplota priestorová", n),
                "air_supply" => ("prietok vzduchu – prívod", to_m3h(n, v.unit)),
                "air_exhaust" => ("prietok vzduchu – odvod", to_m3h(n, v.unit)),
                "flow" if is_air => ("prietok vzduchu", to_m3h(n, v.unit)),
                "flow" => ("prietok", to_m3h(n, v.unit)),
                "loss" => ("tlaková strata", to_kpa(n, v.unit)),
                "kv" => ("prietokový súčiniteľ kv", n),
                "power" => ("menovitý výkon", to_w(n, v.unit)),
                _ => continue,
            };
            if !out.iter().any(|(x, _, _)| *x == name) {
                out.push((name, Known::Raw(val), Source::Calc));
            }
            if v.key == "power" && !out.iter().any(|(x, _, _)| *x == "vykurovací výkon") {
                out.push((
                    "vykurovací výkon",
                    Known::Raw(to_w(n, v.unit)),
                    Source::Calc,
                ));
            }
        }
    }
    out
}

/// A window's or door's leaves (the fields that open), how they open,
/// the active leaf's width, a door's clear passage and its fire closure.
fn joinery_values(out: &mut Vec<(&'static str, Known, Source)>, o: &crate::Opening) {
    use crate::joinery;
    use crate::LeafOp;
    let door = o.kind == crate::OpeningKind::Door;
    let (fields, _) =
        joinery::division_of(o).layout_of(door, o.width, o.height, joinery::frame_width(o));
    let opens: Vec<_> = fields
        .iter()
        .filter(|f| f.sash.op != LeafOp::Fixed)
        .collect();
    put(out, "počet krídel", Known::Raw(opens.len() as f64));
    let words: Vec<String> = fields.iter().map(|f| op_words(f.sash.op)).collect();
    let mut kinds: Vec<String> = vec![];
    for w in words {
        if !kinds.contains(&w) {
            kinds.push(w);
        }
    }
    put(
        out,
        "typ otvárania",
        Known::Text(format!("{} ({})", kinds.join(" + "), joinery::operation(o))),
    );
    // the active leaf: the widest that opens; an undivided door's is its
    // leaf's nominal width
    let active = match (&o.division, &o.door, door) {
        (None, Some(d), true) => Some(d.nom_width),
        _ => opens.iter().map(|f| f.x1 - f.x0).reduce(f64::max),
    };
    match active.filter(|w| *w > 0.0) {
        Some(w) => put(out, "šírka aktívneho krídla", Known::Len(w)),
        None if opens.is_empty() => put(
            out,
            "šírka aktívneho krídla",
            Known::NotApplicable("nemá, pevné zasklenie"),
        ),
        None => {}
    }
    if let Some((w, h)) = joinery::clear_passage(o) {
        put(out, "priechodná šírka", Known::Len(w));
        put(out, "priechodná výška", Known::Len(h));
    }
    if let Some(p) = &o.performance {
        if let Some(u) = p.uw {
            put(out, "súčiniteľ prechodu tepla – navrhnutý", Known::Raw(u));
        }
        if let Some(g) = p.g {
            put(out, "solárny faktor", Known::Raw(g));
        }
        if let Some(t) = p.tv {
            put(out, "činiteľ prestupu svetla", Known::Raw(t));
        }
        if let Some(r) = p.rw {
            put(out, "vážená nepriezvučnosť", Known::Raw(r));
        }
        if !p.product.trim().is_empty() {
            put(out, "typové označenie", Known::Text(p.product.clone()));
        }
    }
    match o.door.as_ref().and_then(|d| d.fire.as_ref()) {
        Some(f) => {
            if !f.rating.trim().is_empty() {
                put(out, "požiarna odolnosť", Known::Text(f.rating.clone()));
            }
            put(out, "samozatvárač", Known::Bool(f.self_closing));
            put(out, "dymotesnosť", Known::Bool(f.smoke_tight));
        }
        // no fire closure: none of it is asked
        None => {
            let why = if door {
                "nie je požiarny uzáver"
            } else {
                "okno nie je požiarny uzáver"
            };
            for name in ["požiarna odolnosť", "samozatvárač", "dymotesnosť"] {
                put(out, name, Known::NotApplicable(why));
            }
        }
    }
}

/// How a field opens, in words (left and right seen from outside).
fn op_words(op: crate::LeafOp) -> String {
    use crate::LeafOp::*;
    match op {
        Fixed => "pevné zasklenie",
        TurnLeft => "otváravé ľavé",
        TurnRight => "otváravé pravé",
        Tilt => "sklopné",
        TiltTurnLeft => "otváravo-sklopné ľavé",
        TiltTurnRight => "otváravo-sklopné pravé",
        TopHung => "výklopné",
        SlideLeft => "posuvné ľavé",
        SlideRight => "posuvné pravé",
        LiftSlideLeft => "zdvižno-posuvné ľavé",
        LiftSlideRight => "zdvižno-posuvné pravé",
        DoorLeft => "otočné ľavé",
        DoorRight => "otočné pravé",
    }
    .to_string()
}

/// A library section's type, area, perimeter and extents (`across` along
/// its x, `up` along its y).
fn section_values(
    out: &mut Vec<(&'static str, Known, Source)>,
    s: &crate::members::Section,
    across: &'static str,
    up: &'static str,
) {
    if !s.name.trim().is_empty() {
        put(out, "typ prierezu", Known::Text(s.name.clone()));
    }
    let p = &s.points;
    if p.len() < 3 {
        return;
    }
    let (mut area, mut per) = (0.0_f64, 0.0_f64);
    let (mut lo, mut hi) = (p[0], p[0]);
    for i in 0..p.len() {
        let (a, b) = (p[i], p[(i + 1) % p.len()]);
        area += a.perp_dot(b) / 2.0;
        per += a.distance(b);
        lo = lo.min(a);
        hi = hi.max(a);
    }
    put(out, "plocha prierezu", Known::Area(area.abs()));
    put(out, "obvod", Known::Len(per));
    put(out, across, Known::Len(hi.x - lo.x));
    put(out, up, Known::Len(hi.y - lo.y));
}

fn to_kw(v: f64, u: crate::results::Unit) -> f64 {
    u.si(v) / 1000.0
}
fn to_w(v: f64, u: crate::results::Unit) -> f64 {
    u.si(v)
}
fn to_m3h(v: f64, u: crate::results::Unit) -> f64 {
    u.si(v) * 3600.0
}
fn to_kpa(v: f64, u: crate::results::Unit) -> f64 {
    u.si(v) / 1000.0
}

fn to_value(prop: &Prop, k: &Known) -> Option<Value> {
    Some(match (prop.kind, k) {
        (PropKind::Number, Known::Len(m)) => Value::Num(length_in(prop, *m)),
        (PropKind::Number, Known::Area(a)) => Value::Num(area_in(prop, *a)),
        (PropKind::Number, Known::Vol(v)) => Value::Num(volume_in(prop, *v)),
        (PropKind::Number, Known::Raw(v)) => Value::Num(*v),
        (PropKind::Integer, Known::Raw(v)) => Value::Int(v.round() as i64),
        (PropKind::Bool, Known::Bool(b)) => Value::Bool(*b),
        (PropKind::Text | PropKind::Date, Known::Text(t)) if !t.trim().is_empty() => {
            Value::Text(t.clone())
        }
        _ => return None,
    })
}

/// One property of an element's class with its value, if it has one.
#[derive(Clone, Debug, PartialEq)]
pub struct Filled {
    pub prop: &'static Prop,
    pub value: Option<(Value, Source)>,
    /// Asked for in the project's phases.
    pub required: bool,
}

/// Every property of an element's class, the required ones first, with
/// what is known of each.
pub fn filled(
    model: &Model,
    cx: &Ctx,
    e: &crate::Element,
) -> Option<(&'static Class, Vec<Filled>)> {
    let d = &cx.data;
    let c = class_in(model, d, e)?;
    let req: Vec<usize> = c.required(&cx.phases).iter().map(|p| p.index).collect();
    let known = known(model, cx, e);
    let typed = d.of(e.id);
    let mut out: Vec<Filled> = c
        .all_props()
        .into_iter()
        .map(|p| {
            let value = typed
                .and_then(|t| t.value(p.name))
                .and_then(|t| Value::parse(p, t))
                .map(|v| (v, Source::Typed))
                .or_else(|| {
                    known
                        .iter()
                        .find(|(n, _, _)| *n == p.name)
                        .and_then(|(_, k, s)| match k {
                            Known::NotApplicable(why) => {
                                Some((Value::Text(why.to_string()), Source::NotApplicable))
                            }
                            _ => to_value(p, k).map(|v| (v, *s)),
                        })
                });
            Filled {
                prop: p,
                value,
                required: req.contains(&p.index),
            }
        })
        .collect();
    out.sort_by_key(|f| !f.required);
    Some((c, out))
}

/// The properties of a system's class with what is typed in.
pub fn filled_system(
    model: &Model,
    d: &DssData,
    code: &str,
) -> Option<(&'static Class, Vec<Filled>)> {
    let c = system_class_in(d, code)?;
    let phases = project_phases(model);
    let req: Vec<usize> = c.required(&phases).iter().map(|p| p.index).collect();
    let typed = d.system(code);
    let mut out: Vec<Filled> = c
        .all_props()
        .into_iter()
        .map(|p| {
            let value = typed
                .and_then(|t| t.value(p.name))
                .and_then(|t| Value::parse(p, t))
                .map(|v| (v, Source::Typed))
                .or_else(|| {
                    (p.name == "označenie systému")
                        .then(|| (Value::Text(code.to_string()), Source::Model))
                });
            Filled {
                prop: p,
                value,
                required: req.contains(&p.index),
            }
        })
        .collect();
    out.sort_by_key(|f| !f.required);
    Some((c, out))
}

/// The building's properties with what is typed in or known.
pub fn filled_building(model: &Model, d: &DssData) -> Option<(&'static Class, Vec<Filled>)> {
    let c = building_class(d)?;
    let phases = project_phases(model);
    let req: Vec<usize> = c.required(&phases).iter().map(|p| p.index).collect();
    let info = model.project_info().map(|(_, p)| p.clone());
    let above = model
        .iter()
        .filter(|e| matches!(&e.kind, ElementKind::Level(l) if l.elevation > -0.5))
        .count();
    let below = model
        .iter()
        .filter(|e| matches!(&e.kind, ElementKind::Level(l) if l.elevation <= -0.5))
        .count();
    let mut out: Vec<Filled> = c
        .all_props()
        .into_iter()
        .map(|p| {
            let typed = d
                .building
                .value(p.name)
                .and_then(|t| Value::parse(p, t))
                .map(|v| (v, Source::Typed));
            let known = match p.name {
                "názov stavby" => info
                    .as_ref()
                    .filter(|i| !i.name.is_empty())
                    .map(|i| Value::Text(i.name.clone())),
                "miesto stavby" => info
                    .as_ref()
                    .filter(|i| !i.cadastre.is_empty() || !i.parcels.is_empty())
                    .map(|i| {
                        Value::Text(format!("{} {}", i.cadastre, i.parcels).trim().to_string())
                    }),
                "počet nadzemných podlaží" | "počet podlaží – nadzemných" if above > 0 => {
                    Some(Value::Int(above as i64))
                }
                "počet podzemných podlaží" | "počet podlaží – podzemných" => {
                    Some(Value::Int(below as i64))
                }
                _ => None,
            };
            Filled {
                prop: p,
                value: typed.or(known.map(|v| (v, Source::Model))),
                required: req.contains(&p.index),
            }
        })
        .collect();
    out.sort_by_key(|f| !f.required);
    Some((c, out))
}

/// How the model stands against the standard: elements without a class,
/// and required properties without a value, element by element.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Check {
    pub classified: usize,
    pub unclassified: Vec<ElementId>,
    pub missing: Vec<(ElementId, Vec<&'static str>)>,
}

/// Is it a building element the standard is about (not a view, a
/// dimension, a material...)?
pub fn is_classifiable(e: &crate::Element) -> bool {
    matches!(
        e.kind,
        ElementKind::Wall(_)
            | ElementKind::Slab(_)
            | ElementKind::Opening(_)
            | ElementKind::Shaft(_)
            | ElementKind::Column(_)
            | ElementKind::Beam(_)
            | ElementKind::Roof(_)
            | ElementKind::Stair(_)
            | ElementKind::Railing(_)
            | ElementKind::Footing(_)
            | ElementKind::Ceiling(_)
            | ElementKind::Room(_)
            | ElementKind::Site(_)
            | ElementKind::Pipe(_)
            | ElementKind::Valve(_)
            | ElementKind::Circuit(_)
            | ElementKind::Object(_)
            | ElementKind::Part(_)
            | ElementKind::FreeMesh(_)
    )
}

pub fn check(model: &Model) -> Check {
    let cx = Ctx::new(model);
    let mut out = Check::default();
    for e in model.iter().filter(|e| is_classifiable(e)) {
        if let ElementKind::Opening(o) = &e.kind {
            if o.kind == crate::OpeningKind::Hole {
                continue;
            }
        }
        if let ElementKind::Part(p) = &e.kind {
            if p.void {
                continue;
            }
        }
        match filled(model, &cx, e) {
            Some((_, props)) => {
                out.classified += 1;
                let miss: Vec<&'static str> = props
                    .iter()
                    .filter(|f| f.required && f.value.is_none())
                    .map(|f| f.prop.name)
                    .collect();
                if !miss.is_empty() {
                    out.missing.push((e.id, miss));
                }
            }
            None => out.unclassified.push(e.id),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_read_and_refer_to_known_properties() {
        assert!(classes().len() > 90);
        assert!(prop(451).is_some());
        for c in classes() {
            for i in c
                .props
                .iter()
                .chain(&c.permit)
                .chain(&c.build)
                .chain(&c.operate)
            {
                assert!(prop(*i).is_some(), "{} refers to property {i}", c.code);
            }
        }
        let w = class("NAB003").unwrap();
        assert_eq!(w.ifc_entity(), ("IfcWall", Some("PARTITIONING")));
        assert_eq!(class("ULM001").unwrap().ifc_entity(), ("IfcWall", None));
        assert_eq!(
            class("BFA002").unwrap().ifc_entity(),
            ("IfcFlowMeter", Some("WATERMETER"))
        );
    }

    #[test]
    fn stages_pick_phases() {
        assert_eq!(phases_of_stage(Some("SZP")), vec![Phase::Permit]);
        assert_eq!(phases_of_stage(Some("VPP")), vec![Phase::Build]);
        assert_eq!(
            phases_of_stage(Some("SZP/PSP")),
            vec![Phase::Permit, Phase::Build]
        );
    }

    #[test]
    fn the_sample_house_is_classified() {
        let m = crate::sample::sample_house();
        let c = check(&m);
        assert!(c.classified > 10, "{c:?}");
        let wall = m
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Wall(_)))
            .unwrap();
        let (class, props) = filled(&m, &Ctx::new(&m), wall).unwrap();
        assert!(class.code == "ULM001" || class.code == "NAB003");
        if class.code == "ULM001" {
            assert!(props
                .iter()
                .any(|f| f.prop.name == "hrúbka" && f.value.is_some()));
        }
    }

    #[test]
    fn typed_values_and_classes_are_kept() {
        let mut d = DssData::default();
        let id = ElementId::new();
        d.of_mut(id).set_value("výrobca", "Wavin");
        d.of_mut(id).class = Some("XMC001".into());
        assert_eq!(d.of(id).unwrap().value("výrobca"), Some("Wavin"));
        d.of_mut(id).set_value("výrobca", "");
        assert_eq!(d.of(id).unwrap().value("výrobca"), None);
    }

    #[test]
    fn footings_columns_and_beams_fill_their_sizes() {
        use glam::DVec2;
        let mut m = crate::sample::sample_house();
        let level = m.levels().next().map(|(e, _)| e.id).unwrap();
        let strip = crate::Element::new(
            "Pás",
            ElementKind::Footing(crate::footing::Footing::strip(
                level,
                DVec2::new(0.0, -5.0),
                DVec2::new(4.0, -5.0),
            )),
        );
        let pad = crate::Element::new(
            "Pätka",
            ElementKind::Footing(crate::footing::Footing::pad(level, DVec2::new(10.0, -5.0))),
        );
        let col = crate::Element::new(
            "Stĺp",
            ElementKind::Column(crate::Column::new(
                level,
                DVec2::new(10.0, -5.0),
                crate::Profile::Rectangle {
                    width: 0.3,
                    depth: 0.4,
                },
            )),
        );
        let ids = [strip.id, pad.id, col.id];
        crate::Transaction::new("t")
            .insert(strip)
            .insert(pad)
            .insert(col)
            .apply(&mut m)
            .unwrap();
        let cx = Ctx::new(&m);
        let num = |id, name: &str| -> f64 {
            let (_, props) = filled(&m, &cx, m.get(id).unwrap()).unwrap();
            match props
                .iter()
                .find(|f| f.prop.name == name)
                .and_then(|f| f.value.clone())
            {
                Some((Value::Num(n), _)) => n,
                v => panic!("{name}: {v:?}"),
            }
        };
        let near = |a: f64, b: f64| assert!((a - b).abs() < 1e-6, "{a} != {b}");
        near(num(ids[0], "šírka"), 0.6);
        near(num(ids[0], "výška"), 0.3);
        near(num(ids[0], "plocha prierezu"), 0.18);
        near(num(ids[1], "šírka"), 1.2);
        near(num(ids[1], "dĺžka"), 1.2);
        near(num(ids[1], "hrúbka"), 400.0); // in mm, as hrúbka is
        near(num(ids[1], "obvod"), 4800.0);
        near(num(ids[2], "plocha prierezu"), 0.12);
        near(num(ids[2], "šírka"), 0.3);
        near(num(ids[2], "obvod"), 1400.0);
    }

    #[test]
    fn windows_and_doors_fill_leaves_passage_and_fire() {
        let mut m = crate::sample::sample_house();
        let pick = |m: &Model, k: crate::OpeningKind| {
            m.iter()
                .find(|e| matches!(&e.kind, ElementKind::Opening(o) if o.kind == k))
                .map(|e| e.id)
                .unwrap()
        };
        let win = pick(&m, crate::OpeningKind::Window);
        let door = pick(&m, crate::OpeningKind::Door);
        let before = m.get(door).unwrap().clone();
        let mut d = before.clone();
        if let ElementKind::Opening(o) = &mut d.kind {
            let mut spec = crate::joinery::DoorSpec::fitting(
                crate::joinery::DoorFrame::Steel,
                o.width,
                o.height,
            );
            spec.fire = Some(crate::joinery::FireDoor::default());
            crate::joinery::apply_door(o, spec);
        }
        crate::Transaction::new("t")
            .replace(before, d)
            .apply(&mut m)
            .unwrap();
        let cx = Ctx::new(&m);
        let val = |id, name: &str| {
            let (_, props) = filled(&m, &cx, m.get(id).unwrap()).unwrap();
            props
                .iter()
                .find(|f| f.prop.name == name)
                .and_then(|f| f.value.clone())
                .map(|(v, _)| v)
        };
        assert!(matches!(val(win, "počet krídel"), Some(Value::Int(_))));
        assert!(matches!(val(win, "typ otvárania"), Some(Value::Text(_))));
        assert!(matches!(val(door, "priechodná šírka"), Some(Value::Num(w)) if w > 0.0));
        assert!(matches!(val(door, "priechodná výška"), Some(Value::Num(h)) if h > 1000.0));
        assert!(matches!(
            val(door, "šírka aktívneho krídla"),
            Some(Value::Num(_))
        ));
        assert_eq!(val(door, "počet krídel"), Some(Value::Int(1)));
        assert_eq!(
            val(door, "požiarna odolnosť"),
            Some(Value::Text("EW 30/D3".into()))
        );
        assert_eq!(val(door, "samozatvárač"), Some(Value::Bool(true)));
        assert_eq!(val(door, "dymotesnosť"), Some(Value::Bool(false)));
        // a window as drawn has its maker's values
        let mut w = m.get(win).unwrap().clone();
        if let ElementKind::Opening(o) = &mut w.kind {
            o.performance = crate::joinery::default_window_performance();
        }
        let before = m.get(win).unwrap().clone();
        crate::Transaction::new("w")
            .replace(before, w)
            .apply(&mut m)
            .unwrap();
        let cx = Ctx::new(&m);
        let val = |id, name: &str| {
            let (_, props) = filled(&m, &cx, m.get(id).unwrap()).unwrap();
            props
                .iter()
                .find(|f| f.prop.name == name)
                .and_then(|f| f.value.clone())
                .map(|(v, _)| v)
        };
        assert_eq!(
            val(win, "súčiniteľ prechodu tepla – navrhnutý"),
            Some(Value::Num(0.6))
        );
        // what the maker does not give stays missing
        assert_eq!(val(win, "solárny faktor"), None);
        // the default window meets the 0,60 asked for; REHAU's best does not
        let p = crate::joinery::default_window_performance().unwrap();
        assert_eq!(crate::joinery::over_requirement(&p, false), None);
        let rehau = crate::joinery::preset("REHAU ARTEVO max, trojsklo").unwrap();
        assert_eq!(rehau.g, Some(0.53));
        assert_eq!(crate::joinery::over_requirement(&rehau, false), Some(0.6));
    }

    #[test]
    fn an_older_projects_windows_get_their_makers_values_and_fixed_panes_ask_no_leaf() {
        let mut m = crate::sample::sample_house();
        // as an older project: no values on any window or door
        let mut tx = crate::Transaction::new("old");
        for e in m.iter() {
            if let ElementKind::Opening(o) = &e.kind {
                if o.performance.is_some() {
                    let mut a = e.clone();
                    if let ElementKind::Opening(x) = &mut a.kind {
                        x.performance = None;
                    }
                    tx = tx.replace(e.clone(), a);
                }
            }
        }
        tx.apply(&mut m).unwrap();
        let door = m
            .iter()
            .find(|e| matches!(&e.kind, ElementKind::Opening(o) if o.kind == crate::OpeningKind::Door))
            .unwrap()
            .clone();
        let ElementKind::Opening(d) = &door.kind else {
            unreachable!()
        };
        let outer = std::collections::HashSet::from([d.host]);
        let pairs = crate::joinery::fill_missing_performance(&m, &outer);
        assert!(!pairs.is_empty());
        let mut tx = crate::Transaction::new("fill");
        for (b, a) in pairs {
            tx = tx.replace(b, a);
        }
        tx.apply(&mut m).unwrap();
        // done once: nothing more to fill
        assert!(crate::joinery::fill_missing_performance(&m, &outer).is_empty());
        let cx = Ctx::new(&m);
        let val = |id, name: &str| {
            let (_, props) = filled(&m, &cx, m.get(id).unwrap()).unwrap();
            props
                .iter()
                .find(|f| f.prop.name == name)
                .and_then(|f| f.value.clone())
        };
        let win = m
            .iter()
            .find(|e| matches!(&e.kind, ElementKind::Opening(o) if o.kind == crate::OpeningKind::Window && o.division.is_none() && o.frame.panes_across * o.frame.panes_up >= 1))
            .unwrap()
            .id;
        assert_eq!(
            val(win, "súčiniteľ prechodu tepla – navrhnutý"),
            Some((Value::Num(0.6), Source::Model))
        );
        assert!(matches!(
            val(door.id, "súčiniteľ prechodu tepla – navrhnutý"),
            Some((Value::Num(_), Source::Model))
        ));
        // a fixed window: no active leaf, no fire closure, nothing missing
        assert!(matches!(
            val(win, "šírka aktívneho krídla"),
            Some((_, Source::NotApplicable))
        ));
        for name in ["požiarna odolnosť", "samozatvárač", "dymotesnosť"] {
            assert!(
                matches!(val(win, name), Some((_, Source::NotApplicable))),
                "{name}"
            );
        }
    }
}
