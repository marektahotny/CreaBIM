//! The calculations' settings, with PilonHydraulika's defaults and its
//! field names (Slovak), so its settings files read as they are.

use crate::fixtures::{default_rules, unknown_rule, FixtureRule};
use crate::network::AccessoryRole;
use crate::peak_flow::{BuildingType, PeakFlowMethod};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    #[serde(rename = "Voda")]
    pub water: WaterSettings,
    #[serde(rename = "Armatury")]
    pub components: ComponentSettings,
    #[serde(rename = "Zeta")]
    pub zeta: ZetaSettings,
    #[serde(rename = "ZariadovaciePredmety")]
    pub fixtures: Vec<FixtureRule>,
    #[serde(rename = "NeznamyPredmet")]
    pub unknown_fixture: FixtureRule,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            water: WaterSettings::default(),
            components: ComponentSettings::default(),
            zeta: ZetaSettings::default(),
            fixtures: default_rules(),
            unknown_fixture: unknown_rule(),
        }
    }
}

/// Water supply.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WaterSettings {
    #[serde(rename = "MetodaSucasnosti")]
    pub method: PeakFlowMethod,
    #[serde(rename = "TypBudovy")]
    pub building: BuildingType,
    /// ΣLU → QD, for [`PeakFlowMethod::TabulkaLU`].
    #[serde(rename = "TabulkaLU")]
    pub lu_table: Vec<[f64; 2]>,
    #[serde(rename = "TeplotaSV_C")]
    pub cold_c: f64,
    #[serde(rename = "TeplotaTV_C")]
    pub hot_c: f64,
    /// Most velocity in distribution pipes, m/s.
    #[serde(rename = "MaxRychlostRozvod_ms")]
    pub v_max_distribution: f64,
    /// Most velocity in a pipe to one fixture, m/s.
    #[serde(rename = "MaxRychlostPripojka_ms")]
    pub v_max_connection: f64,
    /// Most friction loss, Pa/m; 0 for none.
    #[serde(rename = "MaxR_Pa_m")]
    pub r_max: f64,
    #[serde(rename = "MinVnutornyPriemer_mm")]
    pub min_inner_mm: f64,
    /// The least pressure at the connection, kPa (from the water company).
    #[serde(rename = "TlakVPripojkeMin_kPa")]
    pub mains_min_kpa: f64,
    #[serde(rename = "TlakVPripojkeMax_kPa")]
    pub mains_max_kpa: f64,
    /// Most static pressure at an outlet, kPa (STN EN 806-2).
    #[serde(rename = "MaxStatickyTlakPriVytoku_kPa")]
    pub max_static_kpa: f64,
    #[serde(rename = "RezervaTlaku_kPa")]
    pub reserve_kpa: f64,
    #[serde(rename = "StrataVodomer_kPa")]
    pub meter_loss_kpa: f64,
    /// The meter is outside the model: its loss is added at the root.
    #[serde(rename = "VodomerJeMimoModelu")]
    pub meter_outside_model: bool,
    #[serde(rename = "StrataOhrievac_kPa")]
    pub heater_loss_kpa: f64,
    /// The pressure a hot water system has on its own, kPa.
    #[serde(rename = "TlakTVSamostatne_kPa")]
    pub hot_alone_kpa: f64,
    /// Enlarge the critical branch until the pressure suffices.
    #[serde(rename = "AutomatickyZvacsitKritickuVetvu")]
    pub enlarge_critical: bool,
    /// Never propose a pipe smaller than it is.
    #[serde(rename = "LenZvacsovat")]
    pub only_upsize: bool,
    /// No pipe smaller than one after it.
    #[serde(rename = "DodrzatPostupnostDimenzii")]
    pub monotonic: bool,
    /// No pipe smaller than the fixture's own connection.
    #[serde(rename = "DimenziaPodlaPripojkyPredmetu")]
    pub by_connector: bool,
}

impl Default for WaterSettings {
    fn default() -> Self {
        WaterSettings {
            method: PeakFlowMethod::default(),
            building: BuildingType::default(),
            lu_table: vec![],
            cold_c: 10.0,
            hot_c: 55.0,
            v_max_distribution: 1.5,
            v_max_connection: 2.0,
            r_max: 0.0,
            min_inner_mm: 10.0,
            mains_min_kpa: 350.0,
            mains_max_kpa: 600.0,
            max_static_kpa: 500.0,
            reserve_kpa: 20.0,
            meter_loss_kpa: 20.0,
            meter_outside_model: true,
            heater_loss_kpa: 10.0,
            hot_alone_kpa: 250.0,
            enlarge_critical: true,
            only_upsize: true,
            monotonic: true,
            by_connector: true,
        }
    }
}

/// Local loss coefficients ζ of fittings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ZetaSettings {
    #[serde(rename = "Koleno90")]
    pub elbow90: f64,
    #[serde(rename = "Koleno45")]
    pub elbow45: f64,
    /// A tee, straight through.
    #[serde(rename = "TPriechod")]
    pub tee_through: f64,
    /// A tee, into the branch.
    #[serde(rename = "TOdbocka")]
    pub tee_branch: f64,
    #[serde(rename = "Kriz")]
    pub cross: f64,
    #[serde(rename = "Redukcia")]
    pub reducer: f64,
    #[serde(rename = "Spojka")]
    pub coupling: f64,
    #[serde(rename = "Ina")]
    pub other: f64,
    #[serde(rename = "InaArmatura")]
    pub other_accessory: f64,
}

impl Default for ZetaSettings {
    fn default() -> Self {
        ZetaSettings {
            elbow90: 1.0,
            elbow45: 0.5,
            tee_through: 0.5,
            tee_branch: 1.5,
            cross: 2.0,
            reducer: 0.5,
            coupling: 0.3,
            other: 0.5,
            other_accessory: 1.0,
        }
    }
}

/// An accessory's losses: a ζ, or a fixed loss (a meter).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ComponentFamily {
    #[serde(rename = "Zeta")]
    pub zeta: f64,
    #[serde(rename = "PevnaStrata_kPa")]
    pub fixed_loss_kpa: f64,
}

/// Valves and the like on a water network.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ComponentSettings {
    /// Propose a pressure reducer where the static pressure is too high.
    #[serde(rename = "NavrhnutRedukcnyVentil")]
    pub propose_reducer: bool,
    #[serde(rename = "Uzaver")]
    pub shut_off: ComponentFamily,
    #[serde(rename = "SpatnaKlapka")]
    pub check_valve: ComponentFamily,
    #[serde(rename = "Filter")]
    pub strainer: ComponentFamily,
    #[serde(rename = "RedukcnyVentil")]
    pub pressure_reducer: ComponentFamily,
    #[serde(rename = "VyvazovaciVentil")]
    pub balancing: ComponentFamily,
    #[serde(rename = "VentilovaKrabica")]
    pub valve_box: ComponentFamily,
    #[serde(rename = "Vodomer")]
    pub meter: ComponentFamily,
}

impl Default for ComponentSettings {
    fn default() -> Self {
        let z = |zeta| ComponentFamily {
            zeta,
            fixed_loss_kpa: 0.0,
        };
        ComponentSettings {
            propose_reducer: true,
            shut_off: z(0.3),
            check_valve: z(3.0),
            strainer: z(3.0),
            pressure_reducer: z(0.0),
            balancing: z(2.0),
            valve_box: z(3.0),
            meter: ComponentFamily {
                zeta: 0.0,
                fixed_loss_kpa: 20.0,
            },
        }
    }
}

impl Settings {
    /// An accessory's losses by its role. The valve box takes the medical
    /// gases' ζ in PilonHydraulika; 3.0 by default.
    pub fn family_for(&self, role: AccessoryRole) -> ComponentFamily {
        let c = &self.components;
        match role {
            AccessoryRole::ShutOff => c.shut_off.clone(),
            AccessoryRole::CheckValve => c.check_valve.clone(),
            AccessoryRole::Strainer => c.strainer.clone(),
            AccessoryRole::PressureReducer => c.pressure_reducer.clone(),
            AccessoryRole::Balancing => c.balancing.clone(),
            AccessoryRole::WaterMeter => c.meter.clone(),
            AccessoryRole::TeeTap => ComponentFamily {
                zeta: self.zeta.tee_through,
                fixed_loss_kpa: 0.0,
            },
            AccessoryRole::ValveBox => ComponentFamily {
                zeta: c.valve_box.zeta,
                fixed_loss_kpa: 0.0,
            },
            _ => ComponentFamily {
                zeta: self.zeta.other_accessory,
                fixed_loss_kpa: 0.0,
            },
        }
    }
}
