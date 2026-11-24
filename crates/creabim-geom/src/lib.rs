//! Geometry for CreaBIM: turns model elements into triangle meshes for the
//! viewport and into 2D linework for plan drawings.
//!
//! This crate is the seam where a full B-rep kernel (OpenCascade) plugs in
//! later. Architectural elements in the prototype are boxes, extrusions and
//! polygon meshes, which this crate builds exactly.

pub mod annotate;
pub mod annotation;
pub mod autodims;
pub mod coverpage;
pub mod crop;
pub mod netlines;
pub mod turn;

pub mod boarddiagram;
pub mod cropbox;
pub mod curtain;
pub mod declutter;
pub mod detail;
pub mod doclist;
pub mod docset;
pub mod dxfout;
pub mod elplan;
pub mod etics;
pub mod firedistance;
pub mod fireexposure;
pub mod firegrade;
pub mod firerisk;
pub mod firezones;
pub mod flashing;
pub mod flashingtable;
pub mod font;
pub mod fragment;
pub mod gbxml;
pub mod heatplan;
pub mod joinerylist;
pub mod looks;
pub mod markers;
pub mod masonry;
pub mod matlegend;
pub mod members;
pub mod mep;
pub mod mesh;
pub mod objects;
pub mod occupancy;
pub mod opening;
pub mod paper;
pub mod parts;
pub mod pick;
pub mod plan;
pub mod plinthtakeoff;
pub mod printing;
pub mod prio;
pub mod ranges;
pub mod raster;
pub mod refs;
pub mod roofplan;
pub mod roomlegend;
pub mod rooms;
pub mod scene;
pub mod schedule;
pub mod section;
pub mod serviceslegend;
pub mod sheet;
pub mod sitefoot;
pub mod sitetables;
pub mod slice;
pub mod stairmarks;
pub mod stairsteps;
pub mod structural;
pub mod summary;
pub mod takeoff;
pub mod titleblock;
pub mod tradelists;
pub mod triangulate;
pub mod valves;
pub mod wall;
pub mod xlsx;

pub use mesh::TriMesh;
pub use plan::{plan_view, PlanItem, PlanView};
pub use scene::{build_scene, shared_scene, shared_scene_ready, SceneItem};
