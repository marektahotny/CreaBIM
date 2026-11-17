//! Building services calculations: the water supply first, then drainage,
//! heating, gas, ventilation and medical gases, ported from the C# core
//! PilonHydraulika (PilonDesigner) and checked against it number for
//! number (`tests/parity.rs`, the expected values written by
//! `tools/parity`).
//!
//! A network is a tree from its source (the connection, a heater) out to
//! its fixtures: pipes, fittings, accessories and fixtures, each a
//! [`network::Node`]. The calculations size its pipes to the standards
//! (STN EN 806-3, STN 73 6655, DIN 1988-300) and find the pressure it
//! needs at its root.

pub mod fittings;
pub mod fixtures;
pub mod fluid;
pub mod hydraulics;
pub mod model;
pub mod network;
pub mod peak_flow;
pub mod pipe_types;
pub mod settings;
mod sizing;
pub mod water;

pub use fluid::Fluid;
pub use network::{Network, Node, NodeId, NodeKind, PipeSize};
pub use settings::Settings;
pub use water::{run as water, SystemKind, WaterResult};
