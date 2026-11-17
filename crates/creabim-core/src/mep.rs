//! Pipes and ducts of building services, as PilonDesigner draws them: a
//! straight segment between two points on a level, of a pipe type (a series
//! in the services catalogue: copper, HERZ PipeFix, PP-HT...) and a size in
//! it, in a system (cold water, drainage, gas...). Fittings are where
//! segments meet; the calculations read the segments as a network.

use crate::ElementId;
use glam::DVec3;
use serde::{Deserialize, Serialize};

/// What a system carries, as the calculations tell systems apart.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SystemKind {
    #[default]
    ColdWater,
    HotWater,
    Circulation,
    Sanitary,
    Storm,
    FireWater,
    Gas,
    HeatingSupply,
    HeatingReturn,
    CoolingSupply,
    CoolingReturn,
    MedGas,
    Flue,
    AirSupply,
    AirExhaust,
    Other,
}

impl SystemKind {
    /// IFC's predefined type of a distribution system.
    pub fn ifc_system(self) -> &'static str {
        match self {
            SystemKind::ColdWater => "DOMESTICCOLDWATER",
            SystemKind::HotWater | SystemKind::Circulation => "DOMESTICHOTWATER",
            SystemKind::Sanitary => "SEWAGE",
            SystemKind::Storm => "RAINWATER",
            SystemKind::FireWater => "FIREPROTECTION",
            SystemKind::Gas => "GAS",
            SystemKind::HeatingSupply | SystemKind::HeatingReturn => "HEATING",
            SystemKind::CoolingSupply | SystemKind::CoolingReturn => "CHILLEDWATER",
            SystemKind::MedGas => "OXYGEN",
            SystemKind::Flue => "EXHAUST",
            SystemKind::AirSupply => "SUPPLYAIR",
            SystemKind::AirExhaust => "EXHAUSTAIR",
            SystemKind::Other => "USERDEFINED",
        }
    }

    /// Ducts rather than pipes.
    pub fn is_air(self) -> bool {
        matches!(self, SystemKind::AirSupply | SystemKind::AirExhaust)
    }

    /// Drains by gravity: drawn on a fall.
    pub fn is_gravity(self) -> bool {
        matches!(self, SystemKind::Sanitary | SystemKind::Storm)
    }
}

/// A system pipes are drawn in, PilonDesigner's: its code is how drawings
/// label it ("SV", "K"), and it sets what a new pipe in it takes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SystemDef {
    pub code: &'static str,
    /// Its name, in English: the app translates it.
    pub name: &'static str,
    pub kind: SystemKind,
    /// Its colour in drawings, 0xRRGGBB.
    pub color: u32,
    /// The pipe type a new pipe takes (an id in the services catalogue).
    pub pipe_type: &'static str,
    pub nominal_mm: f64,
    /// Height of the axis above the floor, m.
    pub height: f64,
    /// Fall, % (drainage).
    pub slope_pct: f64,
    /// Drawn dashed (returns, circulation).
    pub dashed: bool,
}

const fn sys(
    code: &'static str,
    name: &'static str,
    kind: SystemKind,
    color: u32,
    pipe_type: &'static str,
    nominal_mm: f64,
    height: f64,
) -> SystemDef {
    SystemDef {
        code,
        name,
        kind,
        color,
        pipe_type,
        nominal_mm,
        height,
        slope_pct: 0.0,
        dashed: false,
    }
}

const fn dashed(s: SystemDef) -> SystemDef {
    SystemDef { dashed: true, ..s }
}

const fn fall(s: SystemDef, slope_pct: f64) -> SystemDef {
    SystemDef { slope_pct, ..s }
}

/// The systems, in PilonDesigner's order.
pub const SYSTEMS: &[SystemDef] = &[
    sys(
        "SV",
        "Cold water",
        SystemKind::ColdWater,
        0x2f6fde,
        "HERZ",
        20.0,
        0.3,
    ),
    sys(
        "TV",
        "Hot water",
        SystemKind::HotWater,
        0xd9452b,
        "HERZ",
        20.0,
        0.3,
    ),
    dashed(sys(
        "C",
        "Hot water circulation",
        SystemKind::Circulation,
        0xf97316,
        "HERZ",
        16.0,
        0.3,
    )),
    fall(
        sys(
            "K",
            "Foul drainage",
            SystemKind::Sanitary,
            0x8b5a2b,
            "PPHT",
            100.0,
            -0.25,
        ),
        2.0,
    ),
    fall(
        sys(
            "D",
            "Rainwater drainage",
            SystemKind::Storm,
            0x0e7490,
            "PPHT",
            100.0,
            -0.25,
        ),
        1.0,
    ),
    sys(
        "PV",
        "Fire water – hydrants",
        SystemKind::FireWater,
        0xe11414,
        "NIROSAN",
        28.0,
        2.6,
    ),
    sys(
        "ZP",
        "Gas – natural gas",
        SystemKind::Gas,
        0x16a34a,
        "HERZG",
        20.0,
        2.4,
    ),
    sys(
        "UKP",
        "Heating – supply",
        SystemKind::HeatingSupply,
        0xc92a2a,
        "HERZ",
        16.0,
        0.15,
    ),
    dashed(sys(
        "UKS",
        "Heating – return",
        SystemKind::HeatingReturn,
        0x4c6ef5,
        "HERZ",
        16.0,
        0.1,
    )),
    sys(
        "CHP",
        "Cooling – supply",
        SystemKind::CoolingSupply,
        0x15aabf,
        "Cu",
        16.0,
        2.6,
    ),
    dashed(sys(
        "CHS",
        "Cooling – return",
        SystemKind::CoolingReturn,
        0x66a6b8,
        "Cu",
        16.0,
        2.55,
    )),
    sys(
        "O2",
        "Oxygen O2",
        SystemKind::MedGas,
        0x0096ff,
        "CUMED",
        15.0,
        2.6,
    ),
    sys(
        "AIR",
        "Medical air AIR",
        SystemKind::MedGas,
        0x00aa5a,
        "CUMED",
        15.0,
        2.6,
    ),
    sys(
        "VAC",
        "Vacuum VAC",
        SystemKind::MedGas,
        0xc89600,
        "CUMED",
        22.0,
        2.6,
    ),
    sys(
        "N2O",
        "Nitrous oxide N2O",
        SystemKind::MedGas,
        0xa046d2,
        "CUMED",
        15.0,
        2.6,
    ),
    sys(
        "CO2",
        "Carbon dioxide CO2",
        SystemKind::MedGas,
        0x8c8c8c,
        "CUMED",
        15.0,
        2.6,
    ),
    sys(
        "DYM",
        "Flue – boiler exhaust",
        SystemKind::Flue,
        0xd27832,
        "DYM",
        100.0,
        2.2,
    ),
    sys(
        "VZP",
        "Ventilation – supply air",
        SystemKind::AirSupply,
        0x2563eb,
        "VZK",
        160.0,
        2.7,
    ),
    dashed(sys(
        "VZO",
        "Ventilation – exhaust air",
        SystemKind::AirExhaust,
        0xca8a04,
        "VZK",
        160.0,
        2.7,
    )),
];

/// The system with this code.
pub fn system(code: &str) -> Option<&'static SystemDef> {
    SYSTEMS.iter().find(|s| s.code == code)
}

/// A straight pipe or duct segment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pipe {
    pub level: ElementId,
    /// The system's code ("SV", "K"...).
    pub system: String,
    /// Its ends: plan position, and height of the axis above the level, m.
    pub start: DVec3,
    pub end: DVec3,
    /// The pipe type: an id in the services catalogue ("HERZ", "Cu"...).
    pub pipe_type: String,
    /// Its size, as the type names it, mm.
    pub nominal_mm: f64,
    /// The size's diameters, mm, kept with it so drawings need no catalogue.
    pub outer_mm: f64,
    pub inner_mm: f64,
    /// A rectangular duct's height, mm (its width is `nominal_mm`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height_mm: Option<f64>,
    /// Insulation, mm; 0 for none.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub insulation_mm: f64,
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

impl Pipe {
    pub fn length(&self) -> f64 {
        self.start.distance(self.end)
    }

    /// The system it is in, if its code is known.
    pub fn system_def(&self) -> Option<&'static SystemDef> {
        system(&self.system)
    }

    pub fn kind(&self) -> SystemKind {
        self.system_def().map_or(SystemKind::Other, |s| s.kind)
    }

    /// Rises or falls more than it runs: a riser or a drop.
    pub fn is_vertical(&self) -> bool {
        let d = self.end - self.start;
        d.z.abs() > d.truncate().length() && d.z.abs() > 0.05
    }

    /// Its width and height across, m, insulation aside: a rectangular
    /// duct's sides, else the outside diameter twice.
    pub fn section_m(&self) -> (f64, f64) {
        match self.height_mm {
            Some(h) => (self.nominal_mm / 1000.0, h / 1000.0),
            None => (self.outer_mm / 1000.0, self.outer_mm / 1000.0),
        }
    }

    /// Fall along it, % (positive: the end is lower).
    pub fn fall_pct(&self) -> f64 {
        let run = (self.end - self.start).truncate().length();
        if run < 1e-9 {
            0.0
        } else {
            (self.start.z - self.end.z) / run * 100.0
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.start.is_finite() || !self.end.is_finite() {
            return Err("a pipe's ends must be finite".into());
        }
        if self.length() < 1e-4 {
            return Err("a pipe needs two different ends".into());
        }
        if !(self.outer_mm > 0.0 && self.inner_mm > 0.0 && self.inner_mm <= self.outer_mm) {
            return Err("a pipe needs an inner diameter within its outer".into());
        }
        if !(self.nominal_mm.is_finite() && self.nominal_mm > 0.0) {
            return Err("a pipe needs a size".into());
        }
        if self.height_mm.is_some_and(|h| !(h.is_finite() && h > 0.0)) {
            return Err("a duct's height must be positive".into());
        }
        if !(self.insulation_mm.is_finite() && self.insulation_mm >= 0.0) {
            return Err("insulation must not be negative".into());
        }
        Ok(())
    }
}

/// Which pipes a valve or accessory goes on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValveMedium {
    Water,
    Heating,
    Cooling,
    Fire,
    Gas,
    MedGas,
    Drain,
    Air,
}

impl ValveMedium {
    /// The medium of a system's pipes.
    pub fn of(kind: SystemKind) -> Option<ValveMedium> {
        Some(match kind {
            SystemKind::ColdWater | SystemKind::HotWater | SystemKind::Circulation => {
                ValveMedium::Water
            }
            SystemKind::Sanitary | SystemKind::Storm => ValveMedium::Drain,
            SystemKind::FireWater => ValveMedium::Fire,
            SystemKind::Gas => ValveMedium::Gas,
            SystemKind::HeatingSupply | SystemKind::HeatingReturn => ValveMedium::Heating,
            SystemKind::CoolingSupply | SystemKind::CoolingReturn => ValveMedium::Cooling,
            SystemKind::MedGas => ValveMedium::MedGas,
            SystemKind::AirSupply | SystemKind::AirExhaust => ValveMedium::Air,
            SystemKind::Flue | SystemKind::Other => return None,
        })
    }
}

/// A kind of valve or pipe accessory (PilonDesigner's `VALVE_KINDS`, the
/// accessory roles of the Pilon Hydraulika core).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValveKind {
    /// Its code, as files keep it ("uzaver", "cistiaci"...).
    pub code: &'static str,
    /// Its name, in Slovak.
    pub name: &'static str,
    /// Local loss coefficient ζ.
    pub zeta: Option<f64>,
    /// A fixed pressure loss, kPa (a water meter, a pressure reducer).
    pub dp_kpa: Option<f64>,
    pub media: &'static [ValveMedium],
    /// IFC 4 entity and predefined type.
    pub ifc: (&'static str, &'static str),
    /// The standards it is designed to.
    pub norm: &'static str,
    /// The end of a run (a gas appliance, a medical gas outlet).
    pub terminal: bool,
    /// Without effect on the hydraulic calculation (a vent cap, a
    /// cleanout).
    pub nocalc: bool,
}

const fn vk(
    code: &'static str,
    name: &'static str,
    zeta: Option<f64>,
    dp_kpa: Option<f64>,
    media: &'static [ValveMedium],
    ifc: (&'static str, &'static str),
    norm: &'static str,
) -> ValveKind {
    ValveKind {
        code,
        name,
        zeta,
        dp_kpa,
        media,
        ifc,
        norm,
        terminal: false,
        nocalc: false,
    }
}

const fn terminal(k: ValveKind) -> ValveKind {
    ValveKind {
        terminal: true,
        ..k
    }
}

const fn nocalc(k: ValveKind) -> ValveKind {
    ValveKind { nocalc: true, ..k }
}

use ValveMedium as M;

/// The valves and accessories, in PilonDesigner's order.
pub const VALVE_KINDS: &[ValveKind] = &[
    vk(
        "uzaver",
        "Guľový uzáver",
        Some(0.3),
        None,
        &[M::Water, M::Heating, M::Cooling, M::Fire, M::MedGas],
        ("IfcValve", "ISOLATING"),
        "STN EN 13828",
    ),
    vk(
        "rohovy",
        "Rohový ventil (pripojenie predmetu)",
        Some(3.0),
        None,
        &[M::Water],
        ("IfcValve", "ISOLATING"),
        "STN EN 806-2, STN EN 200",
    ),
    vk(
        "spatna",
        "Spätná klapka",
        Some(3.0),
        None,
        &[M::Water, M::Heating, M::Cooling, M::Fire],
        ("IfcValve", "CHECK"),
        "STN EN 13959",
    ),
    vk(
        "filter",
        "Filter (lapač nečistôt)",
        Some(3.0),
        None,
        &[M::Water, M::Heating, M::Cooling, M::Gas],
        ("IfcFilter", "STRAINER"),
        "STN EN 13443-1",
    ),
    vk(
        "redukcny",
        "Redukčný ventil tlaku",
        None,
        Some(0.0),
        &[M::Water, M::Fire],
        ("IfcValve", "PRESSUREREDUCING"),
        "STN EN 1567",
    ),
    vk(
        "vyvazovaci",
        "Vyvažovací / cirkulačný ventil",
        Some(2.0),
        None,
        &[M::Water, M::Heating, M::Cooling],
        ("IfcValve", "REGULATING"),
        "",
    ),
    vk(
        "termostaticky",
        "Termostatický ventil (vykurovacie teleso)",
        Some(8.0),
        None,
        &[M::Heating],
        ("IfcValve", "REGULATING"),
        "STN EN 215",
    ),
    vk(
        "spiatocka",
        "Ventil do spiatočky (regulačné šróbenie)",
        Some(2.0),
        None,
        &[M::Heating],
        ("IfcValve", "ISOLATING"),
        "STN EN 215",
    ),
    vk(
        "rtd",
        "Regulátor tlakovej diferencie",
        Some(0.0),
        None,
        &[M::Heating, M::Cooling],
        ("IfcValve", "REGULATING"),
        "",
    ),
    vk(
        "vodomer",
        "Vodomer",
        None,
        Some(20.0),
        &[M::Water],
        ("IfcFlowMeter", "WATERMETER"),
        "STN EN ISO 4064",
    ),
    vk(
        "kohut",
        "Plynový guľový kohút",
        Some(0.3),
        None,
        &[M::Gas],
        ("IfcValve", "GASCOCK"),
        "STN EN 331",
    ),
    vk(
        "hup",
        "Skrinka HUP (hlavný uzáver plynu)",
        Some(0.0),
        None,
        &[M::Gas],
        ("IfcValve", "ISOLATING"),
        "TPP 609 01, STN EN 1775",
    ),
    vk(
        "plynomer",
        "Plynomer",
        Some(0.0),
        None,
        &[M::Gas],
        ("IfcFlowMeter", "GASMETER"),
        "STN EN 1359",
    ),
    vk(
        "regulator",
        "Regulátor tlaku plynu",
        Some(0.0),
        None,
        &[M::Gas],
        ("IfcValve", "PRESSUREREDUCING"),
        "STN EN 88-1",
    ),
    vk(
        "tap",
        "Termický uzáver (TAP)",
        Some(1.0),
        None,
        &[M::Gas],
        ("IfcValve", "SAFETYCUTOFF"),
        "TPP 704 01",
    ),
    vk(
        "krabica",
        "Uzáverová (ventilová) skrinka",
        Some(3.0),
        None,
        &[M::MedGas],
        ("IfcValve", "ISOLATING"),
        "STN EN ISO 7396-1",
    ),
    terminal(vk(
        "spotrebic",
        "Plynový spotrebič",
        None,
        None,
        &[M::Gas],
        ("IfcBurner", "NOTDEFINED"),
        "TPP 704 01",
    )),
    nocalc(vk(
        "hlavica",
        "Odvetrávacia hlavica",
        None,
        None,
        &[M::Drain],
        ("IfcPipeFitting", "USERDEFINED"),
        "STN EN 12056-2, STN 75 6760",
    )),
    nocalc(vk(
        "cistiaci",
        "Čistiaci kus",
        None,
        None,
        &[M::Drain],
        ("IfcPipeFitting", "USERDEFINED"),
        "STN EN 12056-2, STN 75 6760",
    )),
    terminal(vk(
        "vpust",
        "Strešná vpusť / zvod zo žľabu",
        None,
        None,
        &[M::Drain],
        ("IfcWasteTerminal", "ROOFDRAIN"),
        "STN EN 1253, STN EN 12056-3",
    )),
    terminal(vk(
        "hydrant",
        "Hydrant (hadicové zariadenie)",
        None,
        None,
        &[M::Fire],
        ("IfcFireSuppressionTerminal", "FIREHYDRANT"),
        "STN EN 671-1, STN EN 671-2, STN 92 0400",
    )),
    terminal(vk(
        "vyustP",
        "Prívodná vyústka / tanierový ventil",
        None,
        None,
        &[M::Air],
        ("IfcAirTerminal", "DIFFUSER"),
        "STN EN 12238, STN EN 16798-3",
    )),
    terminal(vk(
        "vyustO",
        "Odvodný tanierový ventil / mriežka",
        None,
        None,
        &[M::Air],
        ("IfcAirTerminal", "GRILLE"),
        "STN EN 12238, STN EN 16798-3",
    )),
    vk(
        "klapkaReg",
        "Regulačná klapka",
        None,
        None,
        &[M::Air],
        ("IfcDamper", "CONTROLDAMPER"),
        "STN EN 1751",
    ),
    vk(
        "klapkaPoz",
        "Protipožiarna klapka",
        None,
        None,
        &[M::Air],
        ("IfcDamper", "FIREDAMPER"),
        "STN EN 15650, STN 73 0872",
    ),
    vk(
        "tlmic",
        "Tlmič hluku",
        None,
        None,
        &[M::Air],
        ("IfcDuctSilencer", "NOTDEFINED"),
        "STN EN ISO 7235",
    ),
    vk(
        "filterVzt",
        "Filter vzduchu",
        None,
        None,
        &[M::Air],
        ("IfcFilter", "AIRPARTICLEFILTER"),
        "STN EN ISO 16890",
    ),
    terminal(vk(
        "vyvod",
        "Vývod med. plynu (koncová jednotka)",
        None,
        None,
        &[M::MedGas],
        ("IfcMedicalDevice", "NOTDEFINED"),
        "STN EN ISO 9170-1",
    )),
];

/// A valve kind by its code.
pub fn valve_kind(code: &str) -> Option<&'static ValveKind> {
    VALVE_KINDS.iter().find(|k| k.code == code)
}

/// The valves that go on a system's pipes; the first is the usual one.
pub fn valves_for(kind: SystemKind) -> Vec<&'static ValveKind> {
    let Some(m) = ValveMedium::of(kind) else {
        return vec![];
    };
    VALVE_KINDS
        .iter()
        .filter(|k| k.media.contains(&m))
        .collect()
}

/// A valve or accessory on a pipe: it sits at `t` along it (0 its start,
/// 1 its end) and moves with it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Valve {
    pub pipe: ElementId,
    pub t: f64,
    /// Its kind's code (see [`VALVE_KINDS`]).
    pub kind: String,
}

impl Valve {
    pub fn kind_def(&self) -> Option<&'static ValveKind> {
        valve_kind(&self.kind)
    }

    /// Its host pipe.
    pub fn host<'a>(&self, model: &'a crate::Model) -> Option<&'a Pipe> {
        match model.get(self.pipe).map(|e| &e.kind) {
            Some(crate::ElementKind::Pipe(p)) => Some(p),
            _ => None,
        }
    }

    /// Where it is on its pipe, level-relative as the pipe's ends.
    pub fn at(&self, pipe: &Pipe) -> DVec3 {
        pipe.start.lerp(pipe.end, self.t)
    }

    pub fn validate(&self) -> Result<(), String> {
        if !(self.t.is_finite() && (0.0..=1.0).contains(&self.t)) {
            return Err("a valve sits on its pipe, between its ends".into());
        }
        if self.kind.is_empty() {
            return Err("a valve needs a kind".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pipe(a: DVec3, b: DVec3) -> Pipe {
        Pipe {
            level: ElementId::new(),
            system: "K".into(),
            start: a,
            end: b,
            pipe_type: "PPHT".into(),
            nominal_mm: 100.0,
            outer_mm: 110.0,
            inner_mm: 104.6,
            height_mm: None,
            insulation_mm: 0.0,
        }
    }

    #[test]
    fn systems_are_pilon_designers() {
        assert_eq!(SYSTEMS.len(), 19);
        let k = system("K").unwrap();
        assert!(k.kind.is_gravity() && k.slope_pct == 2.0 && k.pipe_type == "PPHT");
        assert!(system("UKS").unwrap().dashed);
        assert!(system("VZP").unwrap().kind.is_air());
        let mut codes: Vec<_> = SYSTEMS.iter().map(|s| s.code).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), SYSTEMS.len());
    }

    #[test]
    fn a_drain_falls_along_its_run() {
        let p = pipe(DVec3::new(0.0, 0.0, -0.2), DVec3::new(5.0, 0.0, -0.3));
        assert!((p.fall_pct() - 2.0).abs() < 1e-9);
        assert!(!p.is_vertical());
        assert_eq!(p.kind(), SystemKind::Sanitary);
        assert!(p.validate().is_ok());
        let riser = pipe(DVec3::ZERO, DVec3::new(0.0, 0.0, 3.0));
        assert!(riser.is_vertical());
        assert!(pipe(DVec3::ZERO, DVec3::ZERO).validate().is_err());
    }

    #[test]
    fn a_pipe_round_trips_through_json() {
        let p = pipe(DVec3::ZERO, DVec3::new(1.0, 2.0, 0.3));
        let j = serde_json::to_string(&p).unwrap();
        assert!(!j.contains("insulation_mm") && !j.contains("height_mm"));
        assert_eq!(serde_json::from_str::<Pipe>(&j).unwrap(), p);
    }

    #[test]
    fn valves_are_pilon_designers_and_sit_on_their_pipe() {
        assert_eq!(VALVE_KINDS.len(), 28);
        let mut codes: Vec<_> = VALVE_KINDS.iter().map(|k| k.code).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), VALVE_KINDS.len());
        // each medium is offered its own
        let water: Vec<_> = valves_for(SystemKind::ColdWater)
            .iter()
            .map(|k| k.code)
            .collect();
        assert!(water.contains(&"uzaver") && water.contains(&"vodomer"));
        assert!(!water.contains(&"cistiaci"));
        let drain: Vec<_> = valves_for(SystemKind::Sanitary)
            .iter()
            .map(|k| k.code)
            .collect();
        assert!(drain.contains(&"cistiaci") && drain.contains(&"hlavica"));
        let p = pipe(DVec3::new(0.0, 0.0, 0.3), DVec3::new(4.0, 0.0, 0.3));
        let mut v = Valve {
            pipe: ElementId::new(),
            t: 0.25,
            kind: "uzaver".into(),
        };
        assert_eq!(v.at(&p), DVec3::new(1.0, 0.0, 0.3));
        assert!(v.validate().is_ok());
        v.t = 1.2;
        assert!(v.validate().is_err());
    }
}
