//! CreaBIM model core: the element store, edit operations and undo history.
//!
//! Every view, drawing and export reads from a [`Model`]. Edits never mutate
//! the model directly; they are expressed as a [`Transaction`] of [`Op`]s and
//! committed through a [`Document`], which records them for undo and redo.

pub mod annotation;
pub mod assembly;
pub mod autovalves;
pub mod catalogue;
pub mod checks;
pub mod constraint;
pub mod cropbox;
pub mod csg;
pub mod curtain;
pub mod detail;
pub mod divide;
pub mod document;
pub mod dss;
pub mod earthworks;
pub mod editmesh;
pub mod electrical;
pub mod element;
pub mod fireparams;
pub mod fixture;
pub mod flashing;
pub mod floorfinish;
pub mod footing;
pub mod framing;
pub mod garden;
pub mod graphics;
pub mod grids;
pub mod holds;
pub mod id;
pub mod joinery;
pub mod joins;
pub mod levels;
pub mod library;
pub mod lock;
pub mod mapgroup;
pub mod marks;
pub mod masonry;
pub mod members;
pub mod mep;
pub mod model;
pub mod modgen;
pub mod modifiers;
pub mod norms;
pub mod objects;
pub mod offset;
pub mod ops;
pub mod papermark;
pub mod parts;
pub mod pickup;
pub mod plinth;
pub mod polygon;
pub mod profiles;
pub mod project;
pub mod pushpull;
pub mod railcheck;
pub mod reference;
pub mod refpath;
pub mod regions;
pub mod reno;
pub mod results;
pub mod roofmetal;
pub mod room;
pub mod roomtype;
pub mod routecheck;
pub mod samemat;
pub mod sample;
pub mod screen;
pub mod site;
pub mod skbase;
pub mod skeleton;
pub mod sketch;
pub mod slab;
pub mod split;
pub mod staircheck;
pub mod stairshape;
pub mod starter;
pub mod steeltables;
pub mod structural;
pub mod styles;
pub mod takeoff;
pub mod thermal;
pub mod titleblock;
pub mod transform;
pub mod types;
pub mod underfloor;
pub mod units;
pub mod utility;
pub mod viewmodes;
pub mod visibility;
pub mod walls;
pub mod words;
pub mod wrap;

pub use annotation::{
    text_style, DimAxis, DimensionKind, Font, Note, StyleUse, TagShape, Terminator, TextAlign,
    TextStyle,
};
pub use assembly::{assembly_of, Assembly, AssemblyUse, Layer, LayerFunction, RevealWrap};
pub use constraint::{Constraint, SolveError};
pub use cropbox::{CropBox, CropScope};
pub use csg::BoolOp;
pub use curtain::{CurtainGrid, GridRule, PanelKind};
pub use detail::{DetailItem, DetailKind, DrawingFrame, LineLook, LinePen, LineType, MembraneLook};
pub use document::{Document, Step, StepId};
pub use editmesh::EditMesh;
pub use element::{layer_edge_offsets, set_layer_edge, LayerEdge, LayerEnd, LocationLine, WallTop};
pub use element::{
    ColumnAlign, Cut, Dimension, Element, ElementKind, FreeMesh, Grid, GridGroup, Guide, Hatch,
    HostPoint, Ink, LabelMove, LeafOp, Level, LevelHead, Material, Opening, OpeningFrame,
    OpeningKind, Penetration, ScheduleCategory, SectionJog, SectionLine, Sheet, SheetTitle, Sketch,
    Slab, SlabKind, Source, Tag, Underlay, View, ViewKind, ViewStyle, Viewport, Wall, Workplane,
};
pub use footing::{Footing, FootingAlign, FootingKind};
pub use graphics::{LineWeights, PlanGraphics};
pub use id::ElementId;
pub use joins::{
    corner_partner, wall_joins, CornerJoin, CornerPreset, Gap, JoinRule, JoinWinner, WallEnds,
};
pub use marks::{marks, material_of, MarkClass, MarkOrder, MarkRule, MarkRules};
pub use members::{
    Beam, Column, Landing, Profile, Railing, Roof, RoofShape, RoofSlope, Stair, StairSide,
};
pub use model::Model;
pub use modifiers::{Axis, Modifier, ModifierKind};
pub use objects::{CatalogueObject, Object, ObjectCategory, ObjectType};
pub use ops::{Op, Transaction};
pub use papermark::{PaperMark, PaperMarkKind};
pub use parts::Part;
pub use project::{set_project, EticsBarriers, ProjectInfo};
pub use reference::{LayerStyle, RefSource, Reference};
pub use room::{Ceiling, Room, RoomField, Separator};
pub use screen::ScreenPattern;
pub use site::{PlantForm, Site, SiteForm};
pub use sketch::{bulge_through, Circle, Curve, SegmentKind};
pub use slab::{room_outline, room_outline_to_core, slab_from_sketch, Shaft};
pub use titleblock::SheetTemplate;
pub use transform::Xform;
pub use types::{ElementType, TypeCategory, TypeParams, TypeRef};
pub use units::{write_number, LengthUnit, NumberFormat, NumberFormats, NumberKind, Units};
pub use visibility::{Category, ViewHidden};

/// Errors raised when an edit would leave the model inconsistent.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ModelError {
    #[error("element {0} already exists")]
    Duplicate(ElementId),
    #[error("element {0} not found")]
    NotFound(ElementId),
    #[error("element {element} references missing element {missing}")]
    DanglingReference {
        element: ElementId,
        missing: ElementId,
    },
    #[error("element {element} is still referenced by {by}")]
    StillReferenced { element: ElementId, by: ElementId },
    #[error("invalid element {0}: {1}")]
    Invalid(ElementId, String),
    /// The edit would move an element locked against moving.
    #[error("It is locked against moving: unlock it first (the padlock).")]
    Locked(ElementId),
}
