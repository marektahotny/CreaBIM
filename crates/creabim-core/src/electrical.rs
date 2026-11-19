//! The electrical installation's circuits (after PilonDesigner's
//! `elektro.ts`): a circuit is an element of its own that names its
//! distribution board, its breaker and cable, and the devices it feeds
//! with their installed power. The devices themselves are objects of the
//! catalogue (sockets, switches, lights, boards, detectors); what each is
//! follows from its type's IFC class.

use crate::{Element, ElementId, ElementKind, Model, Object};
use serde::{Deserialize, Serialize};

/// A device a circuit feeds, and its installed power, W (the designer's
/// figure for the appliance; nothing until it is given).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Load {
    pub device: ElementId,
    #[serde(default)]
    pub power_w: f64,
}

/// A final circuit of a distribution board. Its name is the element's.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Circuit {
    /// The board it leaves; soft: a circuit whose board is gone says so.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub board: Option<ElementId>,
    pub three_phase: bool,
    /// A lighting circuit: the tighter voltage drop.
    pub lighting: bool,
    /// The breaker's rated current In, A.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub breaker_a: Option<f64>,
    /// The cable's cross-section, mm² of copper.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cable_mm2: Option<f64>,
    /// The cable's length, m; left out, measured square from the board
    /// to the farthest device.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<f64>,
    /// The phase of a single-phase circuit, 1 to 3.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase: Option<u8>,
    /// Its residual current device's rated residual current, mA.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rcd_ma: Option<f64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub loads: Vec<Load>,
}

impl Circuit {
    pub fn validate(&self) -> Result<(), String> {
        let bad = |v: Option<f64>| v.is_some_and(|x| !(x.is_finite() && x > 0.0));
        if bad(self.breaker_a) || bad(self.cable_mm2) || bad(self.length) || bad(self.rcd_ma) {
            return Err(
                "a circuit's breaker, cable, length and RCD must be more than nothing".into(),
            );
        }
        if self.phase.is_some_and(|p| !(1..=3).contains(&p)) {
            return Err("a circuit's phase is 1, 2 or 3".into());
        }
        if self
            .loads
            .iter()
            .any(|l| !(l.power_w.is_finite() && l.power_w >= 0.0))
        {
            return Err("a device's power can't be less than nothing".into());
        }
        Ok(())
    }

    /// The cable as it is written: `CYKY-J 3×2,5`.
    pub fn cable(&self) -> Option<String> {
        let s = self.cable_mm2?;
        let cores = if self.three_phase { 5 } else { 3 };
        let s = if s.fract() == 0.0 {
            format!("{s:.0}")
        } else {
            format!("{s}").replace('.', ",")
        };
        Some(format!("CYKY-J {cores}×{s}"))
    }
}

/// What an electrical object is to a circuit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DeviceKind {
    Socket,
    Switch,
    Light,
    /// A distribution board.
    Board,
    /// Detectors, alarms, data: not a load of the power circuits.
    Weak,
    /// Anything else electrical: a fixed appliance.
    Appliance,
}

impl DeviceKind {
    pub fn label(self) -> &'static str {
        match self {
            DeviceKind::Socket => "Socket",
            DeviceKind::Switch => "Switch",
            DeviceKind::Light => "Light fitting",
            DeviceKind::Board => "Distribution board",
            DeviceKind::Weak => "Low-current device",
            DeviceKind::Appliance => "Appliance",
        }
    }

    /// Does it draw power from its circuit?
    pub fn is_load(self) -> bool {
        matches!(
            self,
            DeviceKind::Socket | DeviceKind::Light | DeviceKind::Appliance
        )
    }
}

/// What the object `e` is to a circuit; none for anything not electrical.
pub fn device_kind(model: &Model, e: &Element) -> Option<DeviceKind> {
    use crate::objects::ObjectCategory;
    let ElementKind::Object(_) = &e.kind else {
        return None;
    };
    let t = crate::objects::type_of(model, e)?;
    let class = t.ifc.class.to_ascii_lowercase();
    let kind = match class.as_str() {
        "ifcoutlet" => DeviceKind::Socket,
        "ifcswitchingdevice" => DeviceKind::Switch,
        "ifclightfixture" | "ifclamp" => DeviceKind::Light,
        "ifcelectricdistributionboard" => DeviceKind::Board,
        "ifcsensor" | "ifcalarm" | "ifccommunicationsappliance" | "ifcaudiovisualappliance" => {
            DeviceKind::Weak
        }
        _ => match t.category {
            ObjectCategory::Lighting => DeviceKind::Light,
            ObjectCategory::Electrical => DeviceKind::Appliance,
            _ if class == "ifcelectricappliance" => DeviceKind::Appliance,
            _ => return None,
        },
    };
    Some(kind)
}

/// The electrical devices of the model, in its order.
pub fn devices(model: &Model) -> Vec<(&Element, &Object, DeviceKind)> {
    model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Object(o) => Some((e, o, device_kind(model, e)?)),
            _ => None,
        })
        .collect()
}

/// The circuits of the model, in its order.
pub fn circuits(model: &Model) -> Vec<(&Element, &Circuit)> {
    model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Circuit(c) => Some((e, c)),
            _ => None,
        })
        .collect()
}

/// The circuit that feeds device `id`.
pub fn circuit_of(model: &Model, id: ElementId) -> Option<ElementId> {
    circuits(model)
        .into_iter()
        .find(|(_, c)| c.loads.iter().any(|l| l.device == id))
        .map(|(e, _)| e.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cable_is_written_with_its_cores() {
        let mut c = Circuit {
            cable_mm2: Some(2.5),
            ..Default::default()
        };
        assert_eq!(c.cable().as_deref(), Some("CYKY-J 3×2,5"));
        c.three_phase = true;
        c.cable_mm2 = Some(6.0);
        assert_eq!(c.cable().as_deref(), Some("CYKY-J 5×6"));
        c.phase = Some(4);
        assert!(c.validate().is_err());
    }
}
