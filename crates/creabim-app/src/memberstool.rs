//! The Column, Beam, Stair and Railing tools (the Roof tool draws its
//! outline with the Slab tool's modes), their settings for new elements,
//! and the Properties panel for every element they make.

use crate::planview::{PlanState, Tool};
use crate::selection::Selection;
use creabim_core::members::{self, Landing, Profile, RoofShape, RoofSlope};
use creabim_core::{
    Beam, Column, Document, Element, ElementId, ElementKind, Model, ModelError, Railing, Roof,
    Slab, Stair, Transaction,
};
use eframe::egui::{self, Pos2, Stroke};
use glam::{DVec2, DVec3};

/// What a tread or riser is finished with, or what its boards are.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StepFinishKind {
    #[default]
    None,
    /// Solid oak boards (DREVAX).
    Oak,
    /// Ceramic stair tiles (RAKO, 9 mm); on concrete only.
    Tile,
}

impl StepFinishKind {
    pub const ALL: [StepFinishKind; 3] = [
        StepFinishKind::None,
        StepFinishKind::Oak,
        StepFinishKind::Tile,
    ];

    pub fn label(self) -> &'static str {
        match self {
            StepFinishKind::None => "None",
            StepFinishKind::Oak => "Oak board",
            StepFinishKind::Tile => "Ceramic stair tiles",
        }
    }

    /// The library material it is of.
    pub fn material(self) -> Option<&'static str> {
        match self {
            StepFinishKind::None => None,
            StepFinishKind::Oak => Some("Timber, oak"),
            StepFinishKind::Tile => Some("Ceramic tile"),
        }
    }

    /// Its thickness from the makers' tables: on a tread or a riser, on
    /// concrete (cladding) or on stringers (the boards themselves).
    pub fn thickness(self, tread: bool, concrete: bool) -> f64 {
        match (self, tread, concrete) {
            (StepFinishKind::None, ..) => 0.0,
            (StepFinishKind::Tile, ..) => members::STAIR_TILE,
            (StepFinishKind::Oak, true, true) => members::OAK_TREAD_CLADDING,
            (StepFinishKind::Oak, true, false) => members::OAK_TREAD_BOARD,
            (StepFinishKind::Oak, false, true) => members::OAK_RISER_CLADDING,
            (StepFinishKind::Oak, false, false) => members::OAK_RISER_BOARD,
        }
    }
}

/// What new stairs are built of.
#[derive(Clone, Debug, PartialEq)]
pub struct StairBuild {
    pub carrier: members::StairCarrier,
    /// The stringer's catalogue name (see [`members::stringer_catalogue`]).
    pub stringer: String,
    /// Two stringers: their outer faces set in from the stair's edges.
    pub edge_offset: f64,
    pub tread: StepFinishKind,
    pub riser: StepFinishKind,
    pub risers_closed: bool,
    /// The plan form of new stairs (George 2026-11-24).
    pub shape: members::StairShape,
    /// An L or U turns on winders, not a landing.
    pub winders: bool,
    /// New stairs turn left (else right).
    pub left: bool,
    /// A U's well, a spiral's eye radius.
    pub well: f64,
}

impl Default for StairBuild {
    fn default() -> Self {
        Self {
            carrier: members::StairCarrier::Concrete,
            stringer: "KVH 60 × 120".into(),
            edge_offset: 0.0,
            tread: StepFinishKind::None,
            riser: StepFinishKind::None,
            risers_closed: true,
            shape: members::StairShape::Straight,
            winders: false,
            left: true,
            well: 0.2,
        }
    }
}

impl StairBuild {
    /// `s`'s plan form so, turning halfway up its `n` risers.
    pub fn shape(&self, s: &mut Stair, n: u32) {
        use members::StairShape as S;
        s.shape = self.shape;
        s.winders = self.winders && matches!(self.shape, S::L | S::U);
        s.well = self.well.max(0.0);
        s.landing = (!self.shape.is_straight()).then_some(Landing {
            after: (n / 2).max(1),
            left: self.left,
        });
    }

    /// `s` built so; the library materials the project lacks are returned,
    /// to be added with it.
    pub fn apply(&self, model: &Model, s: &mut Stair) -> Vec<Element> {
        let mut new = Vec::new();
        let concrete = self.carrier.is_concrete();
        // on stringers the treads are boards, never left out
        let tread = match self.tread {
            StepFinishKind::None if !concrete => StepFinishKind::Oak,
            k => k,
        };
        s.carrier = self.carrier;
        s.stringer = (!concrete)
            .then(|| {
                members::stringer_catalogue()
                    .into_iter()
                    .find(|x| x.name == self.stringer)
            })
            .flatten()
            .map(|mut x| {
                x.edge_offset = self.edge_offset;
                x
            });
        s.risers_closed = self.risers_closed;
        let mut finish = |k: StepFinishKind, tread: bool| -> Option<members::StepFinish> {
            let name = k.material()?;
            let material = library_material(model, name, &mut new);
            Some(members::StepFinish {
                material,
                thickness: k.thickness(tread, concrete),
            })
        };
        s.tread_finish = finish(tread, true);
        s.riser_finish = finish(self.riser, false);
        new
    }
}

/// The project's material of `name`, or a new one from the library pushed
/// onto `new` (once).
fn library_material(model: &Model, name: &str, new: &mut Vec<Element>) -> Option<ElementId> {
    if let Some((id, _, _)) = creabim_core::library::materials(model)
        .into_iter()
        .find(|(_, n, _)| *n == name)
    {
        return Some(id);
    }
    if let Some(e) = new.iter().find(|e| e.name == name) {
        return Some(e.id);
    }
    let entry = creabim_core::library::LIBRARY
        .iter()
        .find(|e| e.name == name)?;
    let e = entry.element();
    let id = e.id;
    new.push(e);
    Some(id)
}

/// Sizes and shapes for new elements, set in the tool bar.
#[derive(Clone, Debug)]
pub struct MemberSettings {
    pub column: Profile,
    pub beam_width: f64,
    pub beam_depth: f64,
    pub roof_shape: RoofShape,
    pub roof_pitch: f64,
    pub stair_width: f64,
    /// The Stair tool draws ramps (George 2026-11-24).
    pub stair_ramp: bool,
    /// What new stairs are built of: their carrier, stringer, finishes
    /// and risers (George 2026-11-24).
    pub stair_build: StairBuild,
    pub railing_height: f64,
    /// The railing path clicked so far.
    pub path: Vec<DVec2>,
    /// The kind of new footings (strip, pad or pile), unless the tool's
    /// type says.
    pub footing_kind: creabim_core::FootingKind,
    /// Width of new strips, the sides of new pads, a new pile's diameter.
    pub footing_width: f64,
    pub footing_thickness: f64,
    /// Where new strips under walls lie across them, and how far their
    /// edge stands proud of the core face they are set by.
    pub footing_align: creabim_core::FootingAlign,
    pub footing_offset: f64,
    /// How far above the floor of the level they are modelled on new
    /// footings' undersides sit (George 2026-11-20: as a wall's base).
    pub footing_base: f64,
    /// The material new footings are made of, by name: the project's, or
    /// the library's (added to the project with the first footing).
    pub footing_material: Option<String>,
    /// New doors', windows' and bare holes' width, height and sill, set in
    /// the bar before placing (MU18); a type, when picked, sets its own.
    pub door: [f64; 3],
    pub window: [f64; 3],
    pub hole: [f64; 3],
    /// New penetrations are round (their width the diameter).
    pub hole_round: bool,
    /// Where a click puts a new penetration: 0 the wall or else the slab
    /// under it (the floor, then the ceiling), 1 walls only, 2 the floor
    /// slab only, 3 the ceiling (the slab of the storey above) only.
    pub hole_host: u8,
    /// The library profile new columns and beams take; none, their own
    /// square, round or width × depth (MU40).
    pub column_section: Option<String>,
    pub beam_section: Option<String>,
    /// How a chain of beams meets at its corners (MU40).
    pub beam_corner: creabim_core::members::BeamCorner,
    /// Beams drawn in 3D lie on the face clicked (a roof slope, a wall),
    /// square to it, rather than level (MU40).
    pub beam_on_face: bool,
    /// The library profile new railings' top rail takes (MU40).
    pub railing_section: Option<String>,
    /// What new railings are: their type, metal, section and fixing
    /// (George 2026-11-24).
    pub railing_style: RailStyle,
    /// The beam this chain placed last: the next joins only it, and a new
    /// chain starts with none (MU40: a new Beam command joined the old one).
    pub last_beam: Option<ElementId>,
    /// The roof timber or steel member the Beam and Column tools draw
    /// next (Statika tab, George 2026-11-26); none, a plain beam or
    /// column.
    pub framing: Option<creabim_core::framing::Framing>,
    /// The slope a sloped member climbs at from its first point, degrees.
    pub framing_angle: f64,
    /// Collar ties (kleštiny) drawn as a pair, one each side of the
    /// rafters (George 2026-11-27).
    pub framing_pair: bool,
    /// What the Pipe tool draws next.
    pub pipe: crate::pipesui::PipeDraft,
    /// What the Gutter and Downpipe tools draw next.
    pub flashing: crate::flashingui::FlashingDraft,
    /// Heights and build-ups of new constructions (George 2026-11-24).
    pub place: crate::placement::Placement,
}

impl Default for MemberSettings {
    fn default() -> Self {
        Self {
            column: Profile::Rectangle {
                width: 0.3,
                depth: 0.3,
            },
            beam_width: 0.2,
            beam_depth: 0.4,
            roof_shape: RoofShape::Hip,
            roof_pitch: 30.0,
            stair_width: 1.0,
            stair_ramp: false,
            stair_build: StairBuild::default(),
            railing_height: 1.0,
            path: vec![],
            footing_kind: creabim_core::FootingKind::Strip,
            footing_width: creabim_core::footing::STRIP_WIDTH,
            footing_thickness: creabim_core::footing::STRIP_THICKNESS,
            footing_align: Default::default(),
            footing_offset: 0.05,
            footing_base: 0.0,
            footing_material: Some("Concrete, cast in place".into()),
            door: [0.8, 1.97, 0.0],
            window: [1.2, 1.3, 0.9],
            hole: [0.3, 0.3, 0.3],
            hole_round: false,
            hole_host: 0,
            column_section: None,
            beam_section: None,
            beam_corner: Default::default(),
            beam_on_face: false,
            railing_section: None,
            railing_style: RailStyle::default(),
            last_beam: None,
            framing: None,
            framing_angle: 0.0,
            framing_pair: true,
            pipe: Default::default(),
            flashing: Default::default(),
            place: Default::default(),
        }
    }
}

impl MemberSettings {
    /// Width (or diameter) of new columns.
    pub fn column_size(&self) -> f64 {
        match self.column {
            Profile::Rectangle { width, .. } => width,
            Profile::Round { diameter } => diameter,
        }
    }

    pub fn set_column_size(&mut self, v: f64) {
        self.column = match self.column {
            Profile::Rectangle { .. } => Profile::Rectangle { width: v, depth: v },
            Profile::Round { .. } => Profile::Round { diameter: v },
        };
    }

    pub fn round_columns(&self) -> bool {
        matches!(self.column, Profile::Round { .. })
    }

    pub fn set_round(&mut self, round: bool) {
        self.column_section = None;
        let v = self.column_size();
        self.column = if round {
            Profile::Round { diameter: v }
        } else {
            Profile::Rectangle { width: v, depth: v }
        };
    }
}

/// A roof from an outline drawn with the Slab tool's modes.
pub fn roof_from_slab(slab: &Slab, shape: RoofShape, pitch: f64) -> Element {
    let mut r = Roof::new(slab.level, slab.outline(), shape);
    r.pitch = pitch;
    r.base_offset = slab.top_offset;
    Element::new("Roof", ElementKind::Roof(r))
}

/// [`outline_element`] at the height the bar sets (George 2026-11-24):
/// for an outline drawn on the level, not on a face at its own height.
pub fn placed_outline_element(
    model: &creabim_core::Model,
    state: &PlanState,
    mut slab: Slab,
) -> (Element, &'static str) {
    if state.tool != Tool::Shaft {
        state
            .members
            .place
            .outline(state.tool == Tool::Roof, &mut slab);
    }
    outline_element(model, state, slab)
}

/// What the Slab tool's modes make: a slab, or with the Roof tool a roof.
pub fn outline_element(
    model: &creabim_core::Model,
    state: &PlanState,
    slab: Slab,
) -> (Element, &'static str) {
    if state.tool == Tool::Roof {
        let m = &state.members;
        (
            state.typed(model, roof_from_slab(&slab, m.roof_shape, m.roof_pitch)),
            "Draw roof",
        )
    } else if state.tool == Tool::Shaft {
        (
            crate::outline::shaft_from_outline(model, slab.level, slab.outline()),
            "Draw shaft",
        )
    } else {
        (
            state.typed(model, Element::new("Slab", ElementKind::Slab(slab))),
            "Draw slab",
        )
    }
}

/// A roof timber or steel member `e` added with `tx`, tagged at once in
/// the plans of its level: its mark and underside.
fn add_tagged(
    doc: &mut Document,
    selection: &mut Selection,
    e: Element,
    tx: Transaction,
) -> Result<(), ModelError> {
    selection.only(e.id);
    let id = e.id;
    let mut tx = tx.insert(e);
    let mut m = doc.model().clone();
    if tx.clone().apply(&mut m).is_ok() {
        for t in crate::planview::framing_tags(&m, &[id]) {
            tx = tx.insert(t);
        }
    }
    doc.commit(tx)
}

/// A click with the Column, Beam, Stair or Railing tool on `level`.
pub fn click(
    doc: &mut Document,
    level: ElementId,
    tool: Tool,
    p: DVec2,
    state: &mut PlanState,
    selection: &mut Selection,
) -> Result<(), ModelError> {
    let set = state.members.clone();
    let add = |doc: &mut Document, selection: &mut Selection, e: Element, label: &str| {
        selection.only(e.id);
        doc.commit(Transaction::new(label).insert(e))
    };
    match tool {
        Tool::Column => {
            let mut c = Column::new(level, p, set.column);
            // a profile from the library, if one is picked (MU40)
            let lib = set
                .column_section
                .as_deref()
                .and_then(|n| Some((n, creabim_core::profiles::find(doc.model(), n)?)));
            if let Some((n, pts)) = &lib {
                c.section = Some(creabim_core::members::Section {
                    name: n.to_string(),
                    points: pts.clone(),
                });
            }
            state.status = Some(format!(
                "Column {} placed. Its height runs to the next level; drag its top handle or set a height in Properties.",
                lib.map_or_else(|| set.column.label(), |(n, _)| n.to_string())
            ));
            set.place.column(&mut c);
            // a post of the roof's timber or a steel column (Statika tab)
            let name = match &set.framing {
                Some(f) => {
                    crate::app::framingui::make_column(&mut c, f);
                    crate::app::framingui::name_of(f)
                }
                None => "Column".to_string(),
            };
            let e = state.typed(doc.model(), Element::new(name, ElementKind::Column(c)));
            if set.framing.is_some() {
                return add_tagged(doc, selection, e, Transaction::new("Place member"));
            }
            add(doc, selection, e, "Place column")
        }
        Tool::Beam => {
            match state.pending {
                None => {
                    state.members.last_beam = None;
                    state.pending = Some(p);
                    state.status = Some("Click the beam's other end.".into());
                    Ok(())
                }
                Some(a) if a.distance(p) > 0.05 => {
                    let mut b = Beam::new(level, a, p);
                    b.width = set.beam_width;
                    b.depth = set.beam_depth;
                    // a profile from the library, if one is picked (MU40)
                    if let Some(n) = set.beam_section.as_deref() {
                        if let Some(pts) = creabim_core::profiles::find(doc.model(), n) {
                            b.section = Some(creabim_core::members::Section {
                                name: n.to_string(),
                                points: pts,
                            });
                        }
                    }
                    // under the slab above, not inside it
                    b.top_offset = creabim_core::members::beam_top_offset(doc.model(), level, a, p);
                    // or where the bar says (George 2026-11-24)
                    set.place.beam(doc.model(), &mut b);
                    // a roof's timber or a steel member (Statika tab): its
                    // section, and the slope it climbs at from `a`
                    let name = match &set.framing {
                        Some(f) => {
                            crate::app::framingui::make_beam(&mut b, f, set.framing_angle);
                            crate::app::framingui::name_of(f)
                        }
                        None => "Beam".to_string(),
                    };
                    state.pending = Some(p);
                    state.status = Some(
                        "Beam added. Click on to chain another; Esc or right-click ends.".into(),
                    );
                    let e = state.typed(doc.model(), Element::new(name, ElementKind::Beam(b)));
                    // the beam before it in the chain, ending here: the two
                    // meet as the tool's Corners says (MU40)
                    let prev = state
                    .members
                    .last_beam
                    .and_then(|id| doc.model().get(id).cloned())
                    .filter(|pe| matches!(&pe.kind, ElementKind::Beam(pb) if pb.end.distance(a) < 1e-6));
                    state.members.last_beam = Some(e.id);
                    match prev {
                        Some(pe) => {
                            let (mut pe2, mut ne) = (pe.clone(), e);
                            if let (ElementKind::Beam(pb), ElementKind::Beam(nb)) =
                                (&mut pe2.kind, &mut ne.kind)
                            {
                                creabim_core::members::join_beams(pb, nb, set.beam_corner);
                            }
                            if set.framing.is_some() {
                                let tx = Transaction::new("Draw member").replace(pe, pe2);
                                return add_tagged(doc, selection, ne, tx);
                            }
                            selection.only(ne.id);
                            doc.commit(Transaction::new("Draw beam").replace(pe, pe2).insert(ne))
                        }
                        None if set.framing.is_some() => {
                            // collar ties in a pair, either side of the rafter
                            let paired = set.framing_pair
                                && set
                                    .framing
                                    .as_ref()
                                    .is_some_and(|f| f.role == creabim_core::framing::Role::Collar);
                            if paired {
                                let (one, two) = crate::app::framingui::pair(doc.model(), &e);
                                return add_tagged(
                                    doc,
                                    selection,
                                    one,
                                    Transaction::new("Draw member").insert(two),
                                );
                            }
                            add_tagged(doc, selection, e, Transaction::new("Draw member"))
                        }
                        None => add(doc, selection, e, "Draw beam"),
                    }
                }
                Some(_) => Ok(()),
            }
        }
        Tool::Stair => match state.pending.take() {
            None => {
                state.pending = Some(p);
                state.status = Some(
                    if set.stair_ramp {
                        "Click the way the ramp climbs."
                    } else {
                        "Click the way the stair climbs (an L or a U: where it turns)."
                    }
                    .into(),
                );
                Ok(())
            }
            Some(a) if a.distance(p) > 0.05 => {
                let s = new_stair(doc.model(), level, a, p, &set);
                let lay = s.layout(s.rise(doc.model()));
                if s.ramp.is_some() {
                    state.status = Some(format!(
                        "Ramp added: {} long. Railing tool: click inside it to add its railings (near a side: only that one).",
                        crate::units::len(lay.flights[0].run(s.tread))
                    ));
                    return add(
                        doc,
                        selection,
                        Element::new("Ramp", ElementKind::Stair(s)),
                        "Draw ramp",
                    );
                }
                state.status = Some(format!(
                    "Stair added: {} risers of {}. Railing tool: click inside it to add its railings (near a side: only that one).",
                    lay.risers,
                    crate::units::len(lay.riser)
                ));
                // its carrier and finishes as the bar says, with the
                // library materials they need
                let mut s = s;
                let mats = set.stair_build.apply(doc.model(), &mut s);
                let e = Element::new("Stair", ElementKind::Stair(s));
                selection.only(e.id);
                let mut tx = Transaction::new("Draw stair");
                for m in mats {
                    tx = tx.insert(m);
                }
                doc.commit(tx.insert(e))
            }
            Some(_) => Ok(()),
        },
        Tool::Footing => place_footing(doc, level, p, state, selection),
        Tool::Gutter => crate::flashingui::gutter_click(doc, level, p, state, selection),
        Tool::Downpipe => crate::flashingui::downpipe_click(doc, p, state, selection),
        Tool::Capping | Tool::ChimneyFlashing | Tool::RoofOutlet => {
            crate::flashingui::roof_metal_click(doc, level, tool, p, state, selection)
        }
        Tool::Railing => {
            if state.members.path.is_empty() {
                if let Some(stair) = stair_at(doc.model(), level, p) {
                    // near a side: that side only; in the middle both
                    let side = stair_side(doc.model(), stair, p);
                    state.status = Some(
                        match side {
                            Some(true) => "Railing added along the stair's left side.",
                            Some(false) => "Railing added along the stair's right side.",
                            None => "Railings added along both sides of the stair.",
                        }
                        .into(),
                    );
                    let sides = match side {
                        Some(l) => vec![l],
                        None => vec![true, false],
                    };
                    return add_stair_railings(
                        doc,
                        stair,
                        set.railing_height,
                        set.railing_style,
                        &sides,
                        selection,
                    );
                }
            }
            let path = &mut state.members.path;
            if path.last().is_some_and(|q| q.distance(p) < 0.05) {
                // clicking the last point again finishes
                let pts = std::mem::take(path);
                state.pending = None;
                if pts.len() < 2 {
                    return Ok(());
                }
                let mut r = Railing::new(level, pts.iter().map(|q| q.extend(0.0)).collect());
                r.height = set.railing_height;
                set.railing_style.apply(&mut r);
                if set.railing_section.is_some() {
                    r.rail_profile = set.railing_section.clone();
                }
                state.status = Some("Railing added.".into());
                return add(
                    doc,
                    selection,
                    Element::new("Railing", ElementKind::Railing(r)),
                    "Draw railing",
                );
            }
            path.push(p);
            state.pending = Some(p);
            state.status = Some(
                "Click the next point; click the last one again to finish. Esc cancels.".into(),
            );
            Ok(())
        }
        _ => Ok(()),
    }
}

/// The kind of footing the Footing tool places: its type's, or the bar's.
pub fn footing_kind(model: &Model, state: &PlanState) -> creabim_core::FootingKind {
    use creabim_core::{types::TypeParams, TypeCategory};
    let typed = state
        .types
        .get(&TypeCategory::Footing)
        .and_then(|t| match &model.get(*t)?.kind {
            ElementKind::ElementType(t) => match &t.params {
                TypeParams::Footing(f) => Some(f.kind),
                _ => None,
            },
            _ => None,
        });
    typed.unwrap_or(state.members.footing_kind)
}

/// A new footing sized from the tool bar, of the tool's type if it fits.
fn sized_footing(model: &Model, state: &PlanState, mut f: creabim_core::Footing) -> Element {
    let m = &state.members;
    // as wide as the bar says, under a wall too (George 2026-11-20: the
    // width typed before placing is the one built, not widened to the
    // wall's thickness and 300 mm)
    // under a column `footing::under` makes a pad: a pile when piles
    // are being placed
    if f.kind == creabim_core::FootingKind::Pad
        && footing_kind(model, state) == creabim_core::FootingKind::Pile
    {
        f.kind = creabim_core::FootingKind::Pile;
    }
    f.width = m.footing_width;
    if f.kind.at_point() {
        f.length = m.footing_width;
    }
    f.thickness = m.footing_thickness;
    f.align = m.footing_align;
    f.align_offset = m.footing_offset;
    let name = f.kind.label();
    let mut e = state.typed(model, Element::new(name, ElementKind::Footing(f)));
    keep_bar_sizes(&mut e, state);
    e
}

/// The base offset a new footing is set with: the bar's, the underside
/// of a strip or pad above the level's floor; a pile's head is there
/// instead, its toe its length below.
pub fn base_of(f: &creabim_core::Footing, state: &PlanState) -> f64 {
    let b = state.members.footing_base;
    if f.kind == creabim_core::FootingKind::Pile {
        b - f.thickness
    } else {
        b
    }
}

/// Footing `f`, placed with the plan of `level` open, set on the level it
/// is modelled on (George 2026-11-20): its underside on that level's
/// floor, `offset` above it, and moving with the level. Under a wall or
/// column standing on that level or lower, it goes on the nearest level
/// under the host's base (a foundation level); with none there it stays
/// where it was put (under the floor slab), as an offset from its level.
pub fn set_on_level(model: &Model, f: &mut creabim_core::Footing, level: ElementId, offset: f64) {
    let el = |l: ElementId| model.level(l).map(|x| x.elevation);
    let host_base = if let Some(w) = f.host_wall(model) {
        Some(creabim_core::walls::base_z(model, w))
    } else {
        f.host_column(model).map(|c| c.span(model).0)
    };
    let chosen = match host_base {
        None => el(level).map(|_| level),
        Some(b) if el(level).is_some_and(|z| z < b - 1e-6) => Some(level),
        Some(b) => model
            .levels()
            .filter(|(_, l)| l.elevation < b - 1e-6)
            .max_by(|a, b| a.1.elevation.total_cmp(&b.1.elevation))
            .map(|(e, _)| e.id),
    };
    match chosen {
        Some(l) => {
            f.level = l;
            f.base_offset = Some(offset);
        }
        None => {
            let underside = f.top(model) - f.thickness;
            f.base_offset = Some(underside - creabim_core::members::elevation(model, f.level));
        }
    }
}

/// A new footing of a type keeps the width and height typed in the bar
/// before placing it, as the type's own overridden where they differ
/// (George 2026-11-20: the width typed is the width built).
pub fn keep_bar_sizes(e: &mut Element, state: &PlanState) {
    use creabim_core::types::Param;
    let m = &state.members;
    let ElementKind::Footing(f) = &mut e.kind else {
        return;
    };
    if m.footing_kind != f.kind {
        return;
    }
    let mut changed = vec![];
    if (f.width - m.footing_width).abs() > 1e-9 {
        f.width = m.footing_width;
        changed.push(Param::Width);
    }
    if f.kind.at_point() && (f.length - m.footing_width).abs() > 1e-9 {
        f.length = m.footing_width;
        changed.push(Param::Length);
    }
    if (f.thickness - m.footing_thickness).abs() > 1e-9 {
        f.thickness = m.footing_thickness;
        changed.push(Param::Thickness);
    }
    if let Some(r) = &mut e.of_type {
        for p in changed {
            if !r.overrides.contains(&p) {
                r.overrides.push(p);
            }
        }
    }
}

/// The material the bar picked for new footings: the project's of that
/// name, or a new one from the library (the element to add with them).
pub fn footing_material(model: &Model, state: &PlanState) -> (Option<ElementId>, Option<Element>) {
    let Some(name) = &state.members.footing_material else {
        return (None, None);
    };
    if let Some((id, _, _)) = creabim_core::library::materials(model)
        .into_iter()
        .find(|(_, n, _)| n == name)
    {
        return (Some(id), None);
    }
    match creabim_core::library::LIBRARY
        .iter()
        .find(|e| e.name == name)
    {
        Some(entry) => {
            let e = entry.element();
            (Some(e.id), Some(e))
        }
        None => (None, None),
    }
}

/// New footings' material from the bar, unless their type gives one, and
/// their top at the underside of the floor slab they are under; the
/// material added to the project if it is new.
pub fn finish_footings(model: &Model, state: &PlanState, tx: Transaction) -> Transaction {
    let (mat, new) = footing_material(model, state);
    let mut used = false;
    let mut tx = tx;
    for op in &mut tx.ops {
        let creabim_core::Op::Insert(e) = op else {
            continue;
        };
        if let ElementKind::Footing(f) = &mut e.kind {
            if f.material.is_none() && e.of_type.is_none() {
                f.material = mat;
                used |= mat.is_some();
            }
            if f.host.is_none() && f.base_offset.is_none() {
                // its top on the underside of the storey's floor
                let at = match f.kind {
                    creabim_core::FootingKind::Strip => (f.start + f.end) / 2.0,
                    creabim_core::FootingKind::Pad | creabim_core::FootingKind::Pile => f.start,
                };
                let base = creabim_core::members::elevation(model, f.level);
                if let Some(d) = creabim_core::members::slab_depth_at(model, at, base) {
                    f.below = d;
                }
            }
        }
    }
    match new {
        Some(m) if used => tx.insert(m),
        _ => tx,
    }
}

/// A click with the Footing tool: a strip under the wall clicked, or the
/// ends of one on its own; a pad under the column clicked, or one on its own.
fn place_footing(
    doc: &mut Document,
    level: ElementId,
    p: DVec2,
    state: &mut PlanState,
    selection: &mut Selection,
) -> Result<(), ModelError> {
    use creabim_core::{Footing, FootingKind};
    let model = doc.model();
    let kind = footing_kind(model, state);
    // the host the pointer showed (Tab may have moved past the first)
    let shown = state
        .host_pick
        .filter(|h| hosts_at(model, level, Tool::Footing, state, p).contains(h));
    let host = match kind {
        FootingKind::Strip if state.pending.is_none() => shown.or_else(|| wall_at(model, level, p)),
        FootingKind::Pad | FootingKind::Pile => shown.or_else(|| column_at(model, level, p)),
        FootingKind::Strip => None,
    };
    if let Some(h) = host {
        if !creabim_core::footing::footings_under(model, h).is_empty() {
            state.status = Some("That already has a footing under it.".into());
            return Ok(());
        }
        let (tx, ids) = creabim_core::footing::under(model, &[h], &|e| {
            if let ElementKind::Footing(f) = &e.kind {
                *e = sized_footing(model, state, f.clone());
                if let ElementKind::Footing(f) = &mut e.kind {
                    set_on_level(model, f, level, base_of(f, state));
                }
            }
        });
        let name = model.get(h).map_or(String::new(), |e| e.name.clone());
        state.status = Some(format!(
            "{} footing placed under {name}. Ctrl+Z to undo.",
            match kind {
                FootingKind::Strip => "Strip",
                FootingKind::Pad => "Pad",
                FootingKind::Pile => "Pile",
            }
        ));
        state.footing_placed = true;
        if let Some(id) = ids.first() {
            selection.only(*id);
        }
        let mut tx = finish_footings(model, state, tx);
        tx.label = format!("Place {}", kind.label().to_lowercase());
        doc.commit(tx)?;
        // the foundation plan cut through it
        if let Some(v) = foundation_plan(doc.model()) {
            if let Some(fit) = fit_foundation_cut(doc.model(), v) {
                doc.commit(fit)?;
            }
        }
        return Ok(());
    }
    let add = |doc: &mut Document, selection: &mut Selection, e: Element, state: &PlanState| {
        selection.only(e.id);
        let label = format!("Place {}", kind.label().to_lowercase());
        let tx = finish_footings(doc.model(), state, Transaction::new(label).insert(e));
        doc.commit(tx)
    };
    match kind {
        FootingKind::Pad => {
            let mut f = Footing::pad(level, p);
            f.base_offset = Some(state.members.footing_base);
            let e = sized_footing(model, state, f);
            state.status = Some("Pad footing placed. Set its size and depth in Properties.".into());
            add(doc, selection, e, state)
        }
        FootingKind::Pile => {
            let mut e = sized_footing(model, state, Footing::pile(level, p));
            if let ElementKind::Footing(f) = &mut e.kind {
                f.base_offset = Some(base_of(f, state));
            }
            state.status = Some("Pile placed. Set its diameter and length in Properties.".into());
            add(doc, selection, e, state)
        }
        FootingKind::Strip => match state.pending.take() {
            None => {
                state.pending = Some(p);
                state.status = Some("Click the strip's other end.".into());
                Ok(())
            }
            Some(a) if a.distance(p) > 0.05 => {
                let mut f = Footing::strip(level, a, p);
                f.base_offset = Some(state.members.footing_base);
                let e = sized_footing(model, state, f);
                // drawn on in a chain, as walls are: each strip from where
                // the last ended, until Esc (they join at the corners)
                state.pending = Some(p);
                state.status =
                    Some("Strip footing placed. Click the next point, or Esc to finish.".into());
                add(doc, selection, e, state)
            }
            Some(a) => {
                state.pending = Some(a);
                Ok(())
            }
        },
    }
}

/// The wall on `level` under `p` (on it, or within a few centimetres).
pub fn wall_at(model: &Model, level: ElementId, p: DVec2) -> Option<ElementId> {
    walls_at(model, level, p).first().copied()
}

/// Every wall on `level` under `p`, nearest its centre first: across the
/// wall's whole thickness about its centreline (`start` and `end`), and a
/// few centimetres round it, whatever line it was drawn on.
pub fn walls_at(model: &Model, level: ElementId, p: DVec2) -> Vec<ElementId> {
    let mut hits: Vec<(f64, ElementId)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Wall(w) if w.level == level && w.curtain.is_none() => {
                // round the arc on a curved wall
                let (_, s) = w.along(p);
                (s <= w.thickness / 2.0 + 0.05).then_some((s, e.id))
            }
            _ => None,
        })
        .collect();
    hits.sort_by(|a, b| a.0.total_cmp(&b.0));
    hits.into_iter().map(|(_, id)| id).collect()
}

/// The project's foundation plan: a plan view whose name says Foundation.
pub fn foundation_plan(model: &Model) -> Option<ElementId> {
    model.iter().find_map(|e| match &e.kind {
        ElementKind::View(v)
            if matches!(v.kind, creabim_core::ViewKind::Plan { .. })
                && e.name.to_lowercase().contains("foundation") =>
        {
            Some(e.id)
        }
        _ => None,
    })
}

/// The foundation plan `view` cut through the footings, when its plane
/// passes through none of them: the plane moved to halfway down the
/// shallowest footing, its underlay of the floor above dropped.
pub fn fit_foundation_cut(model: &Model, view: ElementId) -> Option<Transaction> {
    let e = model.get(view)?;
    let ElementKind::View(v) = &e.kind else {
        return None;
    };
    let creabim_core::ViewKind::Plan { level, cut_height } = v.kind else {
        return None;
    };
    if creabim_geom::plan::is_foundation_plan(model, level, cut_height) {
        return None;
    }
    let z0 = model.level(level)?.elevation;
    // the footings under the plan's level's storey: the shallowest top
    let (top, bottom) = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Footing(f) => {
                let t = f.top(model);
                Some((t, t - f.thickness))
            }
            _ => None,
        })
        .filter(|(t, _)| *t > z0)
        .min_by(|a, b| a.0.total_cmp(&b.0))?;
    let mid = (top + bottom.max(z0)) / 2.0 - z0;
    let mut after = e.clone();
    if let ElementKind::View(v) = &mut after.kind {
        v.kind = creabim_core::ViewKind::Plan {
            level,
            cut_height: mid,
        };
        if v.underlay == creabim_core::Underlay::Above {
            v.underlay = creabim_core::Underlay::None;
        }
    }
    Some(Transaction::new("Foundation plan through the footings").replace(e.clone(), after))
}

/// Does `tool` go on a host it is clicked onto (a footing under a wall or
/// column, a door or window in a wall)?
pub fn hosted(tool: Tool) -> bool {
    matches!(
        tool,
        Tool::Footing | Tool::Door | Tool::Window | Tool::Opening
    )
}

/// The hosts `tool` could go on at `p`, best first, before its first
/// click: these win over snaps (A3-2). None once a free strip has its
/// first point.
pub fn hosts_at(
    model: &Model,
    level: ElementId,
    tool: Tool,
    state: &PlanState,
    p: DVec2,
) -> Vec<ElementId> {
    use creabim_core::FootingKind;
    if state.pending.is_some() {
        return vec![];
    }
    match tool {
        Tool::Footing => match footing_kind(model, state) {
            FootingKind::Strip => walls_at(model, level, p),
            FootingKind::Pad | FootingKind::Pile => {
                column_at(model, level, p).into_iter().collect()
            }
        },
        Tool::Door | Tool::Window | Tool::Opening => walls_at(model, level, p),
        _ => vec![],
    }
}

/// What a click would do on `host`, as the preview by the pointer says it:
/// "Strip 600 × 300 under W12", "Door in W12".
pub fn host_preview(model: &Model, state: &PlanState, tool: Tool, host: ElementId) -> String {
    let name = model.get(host).map_or(String::new(), |e| e.name.clone());
    match tool {
        Tool::Footing => {
            let kind = footing_kind(model, state);
            let mut f = match kind {
                creabim_core::FootingKind::Strip => {
                    creabim_core::Footing::strip(ElementId::NIL, DVec2::ZERO, DVec2::X)
                }
                creabim_core::FootingKind::Pad => {
                    creabim_core::Footing::pad(ElementId::NIL, DVec2::ZERO)
                }
                creabim_core::FootingKind::Pile => {
                    creabim_core::Footing::pile(ElementId::NIL, DVec2::ZERO)
                }
            };
            f.host = Some(host);
            if let Some(ElementKind::Wall(w)) = model.get(host).map(|e| &e.kind) {
                f.width = w.thickness;
            }
            let ElementKind::Footing(f) = sized_footing(model, state, f).kind else {
                return name;
            };
            let mm = |v: f64| crate::units::len(v).trim_end_matches(" mm").to_string();
            let taken = !creabim_core::footing::footings_under(model, host).is_empty();
            if taken {
                return format!("{name} already has a footing");
            }
            match kind {
                creabim_core::FootingKind::Strip => {
                    format!("Strip {} × {} under {name}", mm(f.width), mm(f.thickness))
                }
                creabim_core::FootingKind::Pad => format!(
                    "Pad {} × {} × {} under {name}",
                    mm(f.width),
                    mm(f.length),
                    mm(f.thickness)
                ),
                creabim_core::FootingKind::Pile => {
                    format!("Pile Ø {} × {} under {name}", mm(f.width), mm(f.thickness))
                }
            }
        }
        Tool::Door => format!("Door in {name}"),
        Tool::Window => format!("Window in {name}"),
        _ => format!("Opening in {name}"),
    }
}

/// The column standing on `level` under `p`.
pub fn column_at(model: &Model, level: ElementId, p: DVec2) -> Option<ElementId> {
    model.iter().find_map(|e| match &e.kind {
        ElementKind::Column(c) if c.level == level => {
            let o = c.outline();
            (inside(&o, p) || c.position.distance(p) < 0.1).then_some(e.id)
        }
        _ => None,
    })
}

/// A stair from `a` climbing towards `b`, sized from the tool's settings.
pub fn new_stair(
    model: &Model,
    level: ElementId,
    a: DVec2,
    b: DVec2,
    set: &MemberSettings,
) -> Stair {
    let dir = (b - a).to_angle().to_degrees();
    if set.stair_ramp {
        let mut s = Stair::new_ramp(level, a, dir);
        s.width = set.stair_width;
        set.place.stair(&mut s);
        return s;
    }
    let mut s = Stair::new(level, a, dir);
    s.width = set.stair_width;
    // its base and top as the bar says (George 2026-11-24)
    set.place.stair(&mut s);
    let n = members::riser_count(s.rise(model), s.max_riser);
    set.stair_build.shape(&mut s, n);
    // to the norm from the start, when its tables are loaded
    let tables = creabim_core::room::project_tables(model);
    if let Some(rules) = creabim_core::staircheck::StairRules::for_stair(&tables, &s) {
        let rise = s.rise(model);
        rules.apply(&mut s, rise);
        // a straight flight as long as it was drawn, in as many risers as
        // the norm's going makes nearest that (George 2026-11-24)
        let len = (b - a).length();
        let how = if s.form().is_straight() && !s.by_steps() && len >= 1.0 {
            creabim_core::staircheck::Fit::Run(len)
        } else {
            creabim_core::staircheck::Fit::Width
        };
        rules.fit(&mut s, rise, how);
    }
    // an L or a U turns where the second click is, if that is far enough
    // up to leave a flight on either side
    if let (Some(l), true) = (&mut s.landing, s.tread > 1e-3) {
        let k = ((b - a).length() / s.tread).round() as u32 + 1;
        if k >= 2 && k + 2 <= n {
            l.after = k;
        }
    }
    s
}

/// The stair on `level` whose footprint holds `p`.
fn stair_at(model: &Model, level: ElementId, p: DVec2) -> Option<ElementId> {
    model.iter().find_map(|e| match &e.kind {
        ElementKind::Stair(s) if s.level == level => {
            let rise = s.rise(model);
            let mut shapes = s.flight_outlines(rise);
            if let Some((l, _)) = s.layout(rise).landing {
                shapes.push(l);
            }
            shapes.iter().any(|o| inside(o, p)).then_some(e.id)
        }
        _ => None,
    })
}

/// Which side of `stair` a click at `p` is near: its outer quarter on
/// the left (looking up) or the right; none in the middle.
fn stair_side(model: &Model, stair: ElementId, p: DVec2) -> Option<bool> {
    let Some(ElementKind::Stair(s)) = model.get(stair).map(|e| &e.kind) else {
        return None;
    };
    let lay = s.layout(s.rise(model));
    let f = lay.flights.iter().find(|f| {
        let n = f.dir.perp() * s.width / 2.0;
        let e = f.end(s.tread);
        inside(&[f.start - n, e - n, e + n, f.start + n], p)
    })?;
    let off = (p - f.start).dot(f.dir.perp());
    let edge = s.width / 4.0;
    if off > s.width / 2.0 - edge {
        Some(true)
    } else if off < -(s.width / 2.0 - edge) {
        Some(false)
    } else {
        None
    }
}

fn inside(poly: &[DVec2], p: DVec2) -> bool {
    let n = poly.len();
    let mut c = false;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            c = !c;
        }
    }
    c
}

/// What new railings are made as (George 2026-11-24): a type (none:
/// rails only, as before), the metal, round or square parts, and how
/// the posts are fixed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RailStyle {
    pub kind: Option<creabim_core::members::RailingType>,
    pub metal: creabim_core::members::RailMetal,
    pub round: bool,
    pub fixing: creabim_core::members::Fixing,
}

impl Default for RailStyle {
    fn default() -> Self {
        RailStyle {
            kind: Some(creabim_core::members::RailingType::UprightBars),
            metal: creabim_core::members::RailMetal::Stainless,
            round: false,
            fixing: creabim_core::members::Fixing::Top,
        }
    }
}

impl RailStyle {
    /// Make `r` so; its height stays.
    pub fn apply(&self, r: &mut Railing) {
        if let Some(k) = self.kind {
            k.apply(r, self.metal, self.round);
        }
        r.fixing = self.fixing;
    }

    /// The style a railing is made in, to draw more like it.
    pub fn of(r: &Railing) -> Self {
        RailStyle {
            kind: creabim_core::members::RailingType::of(r),
            metal: r
                .metal
                .unwrap_or(creabim_core::members::RailMetal::Stainless),
            round: r.round,
            fixing: r.fixing,
        }
    }
}

/// Railings along `sides` of `stair` (true: its left), as one undo
/// step; a wall handrail takes the wall on the stair's outside.
pub fn add_stair_railings(
    doc: &mut Document,
    stair: ElementId,
    height: f64,
    style: RailStyle,
    sides: &[bool],
    selection: &mut Selection,
) -> Result<(), ModelError> {
    let mut tx = Transaction::new("Add stair railings");
    let mut first = None;
    for &left in sides {
        if let Some(mut r) = Railing::along_stair(doc.model(), stair, left) {
            r.height = height;
            style.apply(&mut r);
            if let Some(w) = &mut r.wall {
                w.left = left;
            }
            let e = Element::new("Railing", ElementKind::Railing(r));
            first.get_or_insert(e.id);
            tx = tx.insert(e);
        }
    }
    if let Some(id) = first {
        selection.only(id);
    }
    doc.commit(tx)
}

/// What the tool would make, drawn while its second point is chosen:
/// a beam's width, a stair's flights, a railing's path so far.
#[allow(clippy::too_many_arguments)]
pub fn preview(
    painter: &egui::Painter,
    project: &dyn Fn(DVec3) -> Option<Pos2>,
    model: &Model,
    level: Option<ElementId>,
    tool: Tool,
    state: &PlanState,
    a: DVec3,
    b: DVec3,
) {
    let stroke = Stroke::new(1.0, crate::theme::interaction());
    let z = a.z;
    let ring = |pts: &[DVec2]| {
        let scr: Option<Vec<Pos2>> = pts
            .iter()
            .chain(pts.first())
            .map(|p| project(p.extend(z)))
            .collect();
        if let Some(scr) = scr {
            painter.add(egui::Shape::line(scr, stroke));
        }
    };
    let set = &state.members;
    match (tool, level) {
        (Tool::Beam, Some(level)) if a.distance(b) > 1e-3 => {
            let mut bm = Beam::new(level, a.truncate(), b.truncate());
            bm.width = set.beam_width;
            ring(&bm.outline());
        }
        (Tool::Stair, Some(level)) if a.distance(b) > 1e-3 => {
            let s = new_stair(model, level, a.truncate(), b.truncate(), set);
            let rise = s.rise(model);
            for o in s.flight_outlines(rise) {
                ring(&o);
            }
            if let Some((l, _)) = s.layout(rise).landing {
                ring(&l);
            }
        }
        (Tool::Footing, Some(level)) if a.distance(b) > 1e-3 => {
            let mut f = creabim_core::Footing::strip(level, a.truncate(), b.truncate());
            f.width = set.footing_width;
            let n = (b - a).truncate().normalize_or_zero().perp() * f.width / 2.0;
            ring(&[
                a.truncate() - n,
                b.truncate() - n,
                b.truncate() + n,
                a.truncate() + n,
            ]);
        }
        // a partial tiling: the walls it will run along
        (Tool::Tiling, Some(level)) if a.distance(b) > 1e-3 => {
            let (a2, b2) = (a.truncate(), b.truncate());
            if let Some((_, s)) = crate::roomtool::tiling_room(model, level, a2, b2) {
                let (height, bottom) = state.details.tiling;
                let t = creabim_core::room::Tiling {
                    height,
                    bottom,
                    span: Some([a2, b2]),
                    ..Default::default()
                };
                for (p, q) in creabim_core::room::tiling_plan_runs_of(model, &s, &t) {
                    if let (Some(p), Some(q)) = (project(p.extend(z)), project(q.extend(z))) {
                        painter.line_segment([p, q], Stroke::new(3.0, stroke.color));
                    }
                }
            }
        }
        (Tool::Pipe, _) => crate::pipesui::preview(painter, project, &set.pipe, a, b),
        (Tool::Railing, _) => {
            let scr: Option<Vec<Pos2>> = set.path.iter().map(|p| project(p.extend(z))).collect();
            if let Some(scr) = scr {
                painter.add(egui::Shape::line(scr, Stroke::new(2.0, stroke.color)));
            }
        }
        _ => {}
    }
}

/// A size row: the catalogue's parts of the shape asked as a list, and
/// "Own size" with a row to type it. True when it changed.
fn size_row(
    ui: &mut egui::Ui,
    text: &str,
    v: &mut f64,
    parts: &[creabim_core::members::RailPart],
    round: bool,
) -> bool {
    let mut changed = false;
    let fits: Vec<_> = parts.iter().filter(|p| p.round == round).collect();
    let listed = fits.iter().find(|p| (p.size - *v).abs() < 1e-6);
    crate::props::row(ui, text, |ui| {
        let w = ui.available_width();
        egui::ComboBox::from_id_salt(("rail.size", text))
            .icon(crate::props::combo_icon)
            .width(w)
            .truncate()
            .selected_text(crate::i18n::tw(listed.map_or("Own size", |p| p.name)))
            .show_ui(ui, |ui| {
                for p in &fits {
                    if ui
                        .selectable_label((p.size - *v).abs() < 1e-6, p.name)
                        .clicked()
                    {
                        *v = p.size;
                        changed = true;
                    }
                }
                // a size of one's own: a millimetre off the catalogue's,
                // then typed in the row under it
                if ui
                    .selectable_label(listed.is_none(), crate::i18n::tw("Own size"))
                    .clicked()
                    && listed.is_some()
                {
                    *v += 0.001;
                    changed = true;
                }
            });
    });
    if listed.is_none() || fits.is_empty() {
        changed |= crate::props::row(ui, "Own size", |ui| {
            crate::props::fill(ui, crate::units::drag(v).range(0.005..=0.2)).changed()
        });
    }
    changed
}

/// A railing's type, metal, section, sizes and fixing (George
/// 2026-11-24). True when any changed.
fn railing_kind_rows(ui: &mut egui::Ui, r: &mut creabim_core::Railing) -> bool {
    use creabim_core::members::{self as mb, Fixing, RailMetal, RailingType};
    let mut changed = false;
    let metal = r.metal.unwrap_or(RailMetal::Stainless);
    // the type: made over from the catalogue, its metal and section kept
    let mut kind = RailingType::of(r);
    crate::props::row(ui, "Type", |ui| {
        let w = ui.available_width();
        egui::ComboBox::from_id_salt("rail.type")
            .icon(crate::props::combo_icon)
            .width(w)
            .truncate()
            .selected_text(crate::i18n::tw(kind.map_or("Rails only", |k| k.label())))
            .show_ui(ui, |ui| {
                for k in RailingType::ALL {
                    if ui
                        .selectable_label(kind == Some(k), crate::i18n::tw(k.label()))
                        .clicked()
                        && kind != Some(k)
                    {
                        kind = Some(k);
                        k.apply(r, metal, r.round);
                        changed = true;
                    }
                }
            });
    });
    let mut m = metal;
    if crate::props::choice(
        ui,
        "Metal",
        &mut m,
        &[
            (RailMetal::Stainless, "Stainless steel", "UMAKOV, AISI 304"),
            (RailMetal::Aluminium, "Aluminium", "ALZABRADLI, EN AW 6060"),
        ],
    ) || r.metal.is_none() && m != metal
    {
        match kind {
            Some(k) => k.apply(r, m, r.round),
            None => r.metal = Some(m),
        }
        changed = true;
    }
    let mut round = r.round;
    if crate::props::choice(
        ui,
        "Cross-section",
        &mut round,
        &[
            (false, "Square profile", "JAKL"),
            (true, "Round profile", "Tubes"),
        ],
    ) {
        match kind {
            Some(k) => k.apply(r, m, round),
            None => r.round = round,
        }
        changed = true;
    }
    if r.wall.is_none() {
        let mut post = r.post();
        if size_row(ui, "Post", &mut post, mb::rail_posts(m), r.round) {
            r.post_size = Some(post);
            changed = true;
        }
    }
    let mut rail = r.rail();
    if size_row(ui, "Handrail", &mut rail, mb::rail_handrails(m), r.round) {
        r.rail_size = Some(rail);
        changed = true;
    }
    if let Some(f) = &mut r.infill {
        if f.kind != mb::InfillKind::Panel {
            let mut bar = f.bar;
            if size_row(ui, "Bars", &mut bar, mb::rail_bars(m), f.round) {
                f.bar = bar;
                changed = true;
            }
            if crate::props::choice(
                ui,
                "Bars' section",
                &mut f.round,
                &[(false, "Square profile", ""), (true, "Round profile", "")],
            ) {
                changed = true;
            }
        } else {
            crate::props::note(ui, crate::i18n::tr("Glass Connex 4.4.2, 8,76 mm"));
        }
    }
    match &mut r.wall {
        Some(w) => {
            changed |= crate::props::choice(
                ui,
                "Wall",
                &mut w.left,
                &[(true, "On the left", ""), (false, "On the right", "")],
            );
            changed |= crate::props::row(ui, "Clear of the wall", |ui| {
                crate::props::fill(ui, crate::units::drag(&mut w.clear).range(0.0..=0.3)).changed()
            });
            changed |= crate::props::row(ui, "Brackets at most", |ui| {
                crate::props::fill(ui, crate::units::drag(&mut w.spacing).range(0.2..=3.0))
                    .changed()
            });
        }
        None => {
            changed |= crate::props::choice(
                ui,
                "Fixing",
                &mut r.fixing,
                &[
                    (Fixing::Top, "From the top", "On a flange on the floor"),
                    (
                        Fixing::Side,
                        "From the side",
                        "Into the edge of the slab, stair or ramp",
                    ),
                ],
            );
            if r.fixing == Fixing::Side {
                let mut gap = r.gap_to_edge();
                if crate::props::row(ui, "Gap to the edge", |ui| {
                    crate::props::fill(ui, crate::units::drag(&mut gap).range(0.0..=0.3)).changed()
                }) {
                    r.side_gap = Some(gap);
                    changed = true;
                }
                let mut drop = r.drop();
                if crate::props::row(ui, "Below the floor", |ui| {
                    crate::props::fill(ui, crate::units::drag(&mut drop).range(0.05..=1.0))
                        .changed()
                }) {
                    r.side_drop = Some(drop);
                    changed = true;
                }
            }
        }
    }
    changed
}

/// The Properties panel rows for a column, beam, roof, stair or railing.
/// Returns the undo label when something changed.
pub fn properties(ui: &mut egui::Ui, model: &Model, kind: &mut ElementKind) -> &'static str {
    if matches!(kind, ElementKind::Part(_) | ElementKind::Site(_)) {
        return crate::partstool::properties(ui, model, kind);
    }
    let mut label = "";
    let num = |ui: &mut egui::Ui, text: &str, v: &mut f64, lo: f64, hi: f64| -> bool {
        crate::props::row(ui, text, |ui| {
            crate::props::fill(ui, crate::units::drag(v).range(lo..=hi)).changed()
        })
    };
    let deg = |ui: &mut egui::Ui, text: &str, v: &mut f64, lo: f64, hi: f64| -> bool {
        crate::props::row(ui, text, |ui| {
            crate::props::fill(
                ui,
                egui::DragValue::new(v)
                    .speed(0.5)
                    .range(lo..=hi)
                    .suffix("°"),
            )
            .changed()
        })
    };
    // under the fields, in their column, as Blender's notes (MU31)
    let weak = |ui: &mut egui::Ui, t: String| crate::props::note(ui, t);
    match kind {
        ElementKind::Column(c) => {
            if let Some(mut f) = c.framing.clone() {
                let l = crate::app::framingui::properties(ui, &mut f, None);
                if !l.is_empty() {
                    c.framing = Some(f);
                    creabim_core::framing::fit_column(c);
                    return l;
                }
            }
            let mut round = matches!(c.profile, Profile::Round { .. });
            if crate::props::choice(
                ui,
                "Profile",
                &mut round,
                &[(false, "Rectangular", ""), (true, "Round", "")],
            ) {
                label = "Column profile";
            }
            if label == "Column profile" {
                c.profile = match (round, c.profile) {
                    (true, Profile::Rectangle { width, .. }) => Profile::Round { diameter: width },
                    (false, Profile::Round { diameter }) => Profile::Rectangle {
                        width: diameter,
                        depth: diameter,
                    },
                    (_, p) => p,
                };
            }
            match &mut c.profile {
                Profile::Rectangle { width, depth } => {
                    if num(ui, "Width", width, 0.05, 5.0) | num(ui, "Depth", depth, 0.05, 5.0) {
                        label = "Column size"
                    }
                    if deg(ui, "Rotation", &mut c.rotation, -360.0, 360.0) {
                        label = "Turn column"
                    }
                }
                Profile::Round { diameter } => {
                    if num(ui, "Diameter", diameter, 0.05, 5.0) {
                        label = "Column size"
                    }
                }
            }
            if c.top.is_unconnected() {
                let mut fixed = c.height.is_some();
                if crate::props::switch(ui, &mut fixed, "Fixed height (else up to the next level)")
                    .changed()
                {
                    c.height = fixed.then(|| c.height_in(model));
                    label = "Column height";
                }
                if let Some(h) = &mut c.height {
                    if num(ui, "Height", h, 0.1, 100.0) {
                        label = "Column height"
                    }
                }
            }
            let l = column_levels(ui, model, c);
            if !l.is_empty() {
                label = l;
            }
            if let Some(iw) = &mut c.in_wall {
                if iw.of.is_some() {
                    weak(
                        ui,
                        crate::i18n::tr("XPS board of an RC column in a wall: it follows the column; switch it on the column."),
                    );
                } else {
                    weak(
                        ui,
                        crate::i18n::tr("RC column in a wall: it sits in the wall's load-bearing core, as deep as the core, moves only along the wall and takes its base, top and height from the wall."),
                    );
                    if crate::props::check(
                        ui,
                        "Insulation",
                        &mut iw.xps,
                        "XPS (Styrodur) on the exterior side",
                    )
                    .changed()
                    {
                        label = "Column XPS";
                    }
                    if iw.xps && num(ui, "XPS thickness", &mut iw.xps_thickness, 0.01, 0.2) {
                        label = "Column XPS";
                    }
                }
            }
            if num(ui, "X", &mut c.position.x, -1e4, 1e4)
                | num(ui, "Y", &mut c.position.y, -1e4, 1e4)
            {
                label = "Move column"
            }
            let (z0, z1) = c.span(model);
            weak(
                ui,
                format!(
                    "Base at {}, top at {}.",
                    crate::units::elevation(z0),
                    crate::units::elevation(z1)
                ),
            );
        }
        ElementKind::Footing(f) => {
            let l = footing_fields(ui, model, f);
            if !l.is_empty() {
                label = l;
            }
        }
        ElementKind::Beam(b) => {
            // a roof's timber or a steel member: what it is, its section
            if let Some(mut f) = b.framing.clone() {
                let run = b.start.distance(b.end);
                let mut angle = if run > 1e-6 {
                    ((b.rise[1] - b.rise[0]) / run).atan().to_degrees()
                } else {
                    0.0
                };
                let was = angle;
                let l = crate::app::framingui::properties(ui, &mut f, Some(&mut angle));
                if !l.is_empty() {
                    if (angle - was).abs() > 1e-9 {
                        b.rise[1] = b.rise[0] + run * angle.to_radians().tan();
                        b.up = crate::app::framingui::up_of(b);
                    }
                    b.framing = Some(f);
                    creabim_core::framing::fit_beam(b);
                    return l;
                }
                // its seats: how deep, against a third of its depth
                // (Tesařské spoje II: the notch a third of the member's
                // height)
                if f.seat {
                    let probe = creabim_core::Element::new("", ElementKind::Beam(b.clone()));
                    let h = b.depth;
                    for s in creabim_core::framing::side_profile(model, &probe)
                        .map(|p| p.seats)
                        .unwrap_or_default()
                    {
                        let name = model
                            .get(s.support)
                            .map(|o| {
                                creabim_core::framing::of(o).map_or(o.name.clone(), |f| {
                                    crate::i18n::tr(f.role.label()).to_string()
                                })
                            })
                            .unwrap_or_default();
                        let text = format!(
                            "{}: {name}, {} {} mm (h/3 = {} mm)",
                            crate::i18n::tr("Rafter seat"),
                            crate::i18n::tr("notch"),
                            (s.depth * 1000.0).round(),
                            (h / 3.0 * 1000.0).round()
                        );
                        if s.depth > h / 3.0 + 5e-4 {
                            ui.colored_label(
                                ui.visuals().warn_fg_color,
                                format!(
                                    "{text} — {}",
                                    crate::i18n::tr("deeper than a third of the rafter")
                                ),
                            );
                        } else {
                            weak(ui, text);
                        }
                    }
                }
                weak(
                    ui,
                    "Section from the member's cross-section; width and depth follow it.".into(),
                );
            }
            if num(ui, "Width", &mut b.width, 0.02, 3.0) {
                label = "Beam width"
            }
            if num(ui, "Depth", &mut b.depth, 0.02, 5.0) {
                label = "Beam depth"
            }
            if num(ui, "Below storey top", &mut b.top_offset, -10.0, 50.0) {
                label = "Beam top offset"
            }
            if num(ui, "Start X", &mut b.start.x, -1e4, 1e4)
                | num(ui, "Start Y", &mut b.start.y, -1e4, 1e4)
                | num(ui, "End X", &mut b.end.x, -1e4, 1e4)
                | num(ui, "End Y", &mut b.end.y, -1e4, 1e4)
            {
                label = "Move beam"
            }
            // a sloped beam: each end's top above the beam's top (MU40)
            if num(ui, "Start rise", &mut b.rise[0], -50.0, 50.0)
                | num(ui, "End rise", &mut b.rise[1], -50.0, 50.0)
            {
                label = "Beam slope"
            }
            // each end run past its point, or pulled back where a column
            // or wall runs through (MU40)
            if num(ui, "Start extension", &mut b.extend[0], -20.0, 20.0)
                | num(ui, "End extension", &mut b.extend[1], -20.0, 20.0)
            {
                label = "Beam extension"
            }
            let cuts = b.cuts.iter().filter(|c| c.is_some()).count();
            if cuts > 0 || b.up.is_some() {
                crate::props::row(ui, "Ends", |ui| {
                    ui.label(match cuts {
                        0 => "Square".to_string(),
                        1 => "One mitred".to_string(),
                        _ => "Both mitred".to_string(),
                    })
                });
                let (square, plumb) = crate::props::actions2(
                    ui,
                    "",
                    ("Square ends", "Take the mitre cuts off its ends", cuts > 0),
                    (
                        "Stand plumb",
                        "Its section upright again, not square to the face it was drawn on",
                        b.up.is_some(),
                    ),
                );
                if square {
                    b.cuts = [None, None];
                    label = "Square ends";
                }
                if plumb {
                    b.up = None;
                    label = "Beam plumb";
                }
            }
            weak(
                ui,
                format!(
                    "Length {}, top at {}. Plans show it dashed above the cut.",
                    crate::units::len(b.length()),
                    crate::units::elevation(b.top(model))
                ),
            );
        }
        ElementKind::Roof(r) => {
            // each shape with its own picture
            crate::props::Combo::new("Shape")
                .selected_text(crate::i18n::tw(r.shape.label()))
                .show_ui(ui, |ui| {
                    for s in RoofShape::ALL {
                        let pick = crate::props::picture_option(
                            ui,
                            r.shape == s,
                            s.label(),
                            |p, rect, st| roof_glyph(p, rect, st, s),
                        );
                        if pick.clicked() && r.shape != s {
                            r.shape = s;
                            label = "Roof shape"
                        }
                    }
                });
            if r.shape != RoofShape::Flat && deg(ui, "Pitch", &mut r.pitch, 1.0, 75.0) {
                label = "Roof pitch"
            }
            if num(ui, "Overhang", &mut r.overhang, 0.0, 5.0) {
                label = "Roof overhang"
            }
            if num(ui, "Thickness", &mut r.thickness, 0.02, 2.0) {
                label = "Roof thickness"
            }
            if num(ui, "Base offset", &mut r.base_offset, -10.0, 100.0) {
                label = "Roof base offset"
            }
            // where the eaves end: a steep pitch with a long overhang hangs
            // them down past the walls, which then look gone (seen
            // replaying a 55° hip with 2 m eaves over 3 m walls)
            if r.shape != RoofShape::Flat && r.overhang > 0.0 {
                let drop = r.overhang * r.pitch.to_radians().tan();
                let floor = creabim_core::members::elevation(model, r.level);
                let eave = r.base(model) - drop - floor;
                let text = format!(
                    "Eaves {} above the floor: the overhang drops them {} below the base.",
                    crate::units::len(eave),
                    crate::units::len(drop)
                );
                if eave < 2.1 {
                    crate::props::note(
                        ui,
                        format!(
                            "{text} They hang below head height and hide the walls under them: shorten the overhang, lower the pitch or raise the base offset."
                        ),
                    );
                } else {
                    weak(ui, text);
                }
            }
            let n = r.boundary.len();
            if matches!(r.shape, RoofShape::Shed | RoofShape::Gable) && n > 0 {
                let what = if r.shape == RoofShape::Shed {
                    "Low edge"
                } else {
                    "Eave edge"
                };
                crate::props::row(ui, what, |ui| {
                    crate::props::flow(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = 1.0;
                        let h = ui.spacing().interact_size.y;
                        let w = ((ui.available_width() - 2.0) / 3.0).max(24.0);
                        if ui
                            .add_sized([w, h], egui::Button::new("‹"))
                            .on_hover_text(crate::i18n::t("The edge before"))
                            .clicked()
                        {
                            r.edge = (r.edge + n - 1) % n;
                            label = "Roof edge";
                        }
                        ui.add_sized(
                            [w, h],
                            egui::Label::new(crate::i18n::tr(&format!(
                                "{} of {n}",
                                r.edge % n + 1
                            ))),
                        );
                        if ui
                            .add_sized([w, h], egui::Button::new("›"))
                            .on_hover_text(crate::i18n::t("The next edge"))
                            .clicked()
                        {
                            r.edge = (r.edge + 1) % n;
                            label = "Roof edge";
                        }
                    })
                });
            }
            // each edge's own slope: a gable end (štít) or its own pitch,
            // on any outline (the slopes meet as its straight skeleton)
            if matches!(r.shape, RoofShape::Hip | RoofShape::Gable)
                && n >= 3
                && crate::props::subsection(
                    ui,
                    "roof.edges",
                    "Slopes by edge",
                    !r.slopes.is_empty(),
                )
            {
                for i in 0..n {
                    let now = r.slope(i);
                    let rises = r.edge_pitch(i);
                    let text = format!("Edge {}", i + 1);
                    crate::props::row(ui, &text, |ui| {
                        ui.horizontal(|ui| {
                            let w = (ui.available_width() * 0.62).max(60.0);
                            let shown = match now {
                                RoofSlope::Auto => format!(
                                    "{} · {}",
                                    crate::i18n::t("As the shape"),
                                    match rises {
                                        Some(p) => format!("{p:.0}°"),
                                        None => crate::i18n::t("Gable end").to_string(),
                                    }
                                ),
                                _ => crate::i18n::t(RoofSlope::KINDS[now.kind()]).to_string(),
                            };
                            egui::ComboBox::from_id_salt(("roof.slope", i))
                                .icon(crate::props::combo_icon)
                                .width(w)
                                .selected_text(shown)
                                .show_ui(ui, |ui| {
                                    for (k, name) in RoofSlope::KINDS.iter().enumerate() {
                                        if ui
                                            .selectable_label(now.kind() == k, crate::i18n::t(name))
                                            .clicked()
                                            && now.kind() != k
                                        {
                                            let s = match k {
                                                0 => RoofSlope::Auto,
                                                1 => RoofSlope::Pitch(rises.unwrap_or(r.pitch)),
                                                _ => RoofSlope::Gable,
                                            };
                                            r.set_slope(i, s);
                                            label = "Roof edge slope";
                                        }
                                    }
                                });
                            if let RoofSlope::Pitch(mut p) = now {
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut p)
                                            .speed(0.5)
                                            .range(1.0..=85.0)
                                            .suffix("°"),
                                    )
                                    .changed()
                                {
                                    r.set_slope(i, RoofSlope::Pitch(p));
                                    label = "Roof edge slope";
                                }
                            }
                        });
                    });
                }
            }
            weak(
                ui,
                format!(
                    "{} corners, {} with the overhang, {} high.",
                    n,
                    crate::units::area(r.plan_area()),
                    crate::units::len(r.peak())
                ),
            );
        }
        ElementKind::Stair(s) => {
            let mut ramp = s.ramp.is_some();
            if crate::props::choice(
                ui,
                "Kind",
                &mut ramp,
                &[(false, "Stair", ""), (true, "Ramp", "")],
            ) {
                s.ramp = ramp.then_some(members::RAMP_SLOPE);
                s.landing = None;
                label = if ramp { "Make a ramp" } else { "Make a stair" };
            }
            // what was changed, for keeping the stair to the norm
            let mut fit: Option<creabim_core::staircheck::Fit> = None;
            if num(ui, "Width", &mut s.width, 0.5, 5.0) {
                label = "Stair width";
                fit = Some(creabim_core::staircheck::Fit::Width);
            }
            if let Some(slope) = &mut s.ramp {
                // the slope in per cent
                let mut pct = *slope * 100.0;
                if crate::props::row(ui, "Slope", |ui| {
                    crate::props::fill(
                        ui,
                        egui::DragValue::new(&mut pct)
                            .range(0.5..=50.0)
                            .speed(0.1)
                            .max_decimals(2)
                            .suffix(" %"),
                    )
                    .changed()
                }) {
                    *slope = pct / 100.0;
                    label = "Ramp slope";
                }
                if num(ui, "Thickness", &mut s.waist, 0.05, 0.6) {
                    label = "Ramp thickness"
                }
                if deg(ui, "Direction", &mut s.direction, -360.0, 360.0) {
                    label = "Turn ramp"
                }
                let mut fixed = s.height.is_some();
                if crate::props::switch(ui, &mut fixed, "Fixed rise (else up to the next level)")
                    .changed()
                {
                    s.height = fixed.then(|| s.rise(model));
                    label = "Ramp rise";
                }
                if let Some(h) = &mut s.height {
                    if num(ui, "Rise", h, 0.01, 20.0) {
                        label = "Ramp rise"
                    }
                }
                let rise = s.rise(model);
                let len = s.layout(rise).flights[0].run(s.tread);
                weak(
                    ui,
                    format!(
                        "{} {}, {} {}.",
                        crate::i18n::tr("Length"),
                        crate::units::len(len),
                        crate::i18n::tr("rise"),
                        crate::units::len(rise)
                    ),
                );
                return label;
            }
            let l = stair_build_fields(ui, s);
            if !l.is_empty() {
                label = l;
            }
            if num(ui, "Max riser", &mut s.max_riser, 0.1, 0.3) {
                label = "Stair riser";
                fit = Some(creabim_core::staircheck::Fit::Rise);
            }
            {
                // the count of risers, typed (George 2026-11-24)
                let rise = s.rise(model);
                let mut n = members::riser_count(rise, s.max_riser);
                if crate::props::row(ui, "Number of risers", |ui| {
                    crate::props::fill(ui, egui::DragValue::new(&mut n).range(1..=200)).changed()
                }) {
                    s.max_riser = ((rise / n.max(1) as f64) * 1e4).ceil() / 1e4;
                    label = "Stair risers";
                    fit = Some(creabim_core::staircheck::Fit::Risers(n));
                }
            }
            if num(ui, "Tread", &mut s.tread, 0.15, 0.6) {
                label = "Stair tread";
                fit = Some(creabim_core::staircheck::Fit::Going);
            }
            if num(ui, "Waist", &mut s.waist, 0.05, 0.6) {
                label = "Stair waist"
            }
            if deg(ui, "Direction", &mut s.direction, -360.0, 360.0) {
                label = "Turn stair"
            }
            let mut fixed = s.height.is_some();
            if crate::props::switch(ui, &mut fixed, "Fixed rise (else up to the next level)")
                .changed()
            {
                s.height = fixed.then(|| s.rise(model));
                label = "Stair rise";
                fit = Some(creabim_core::staircheck::Fit::Rise);
            }
            if let Some(h) = &mut s.height {
                if num(ui, "Stair rise", h, 0.1, 20.0) {
                    label = "Stair rise";
                    fit = Some(creabim_core::staircheck::Fit::Rise);
                }
            }
            let rise = s.rise(model);
            let n = members::riser_count(rise, s.max_riser);
            let mut shape = s.form();
            let mut picked = false;
            if s.ramp.is_none() {
                crate::props::Combo::new("Shape")
                    .selected_text(shape.label())
                    .show_ui(ui, |ui| {
                        for c in members::StairShape::ALL {
                            if ui
                                .selectable_label(c == shape, crate::i18n::t(c.label()))
                                .clicked()
                                && c != shape
                            {
                                shape = c;
                                picked = true;
                            }
                        }
                    });
            }
            if picked {
                s.shape = shape;
                s.landing = (!shape.is_straight()).then_some(Landing {
                    after: s.landing.map_or(n / 2, |l| l.after),
                    left: s.landing.is_none_or(|l| l.left),
                });
                if !matches!(shape, members::StairShape::L | members::StairShape::U) {
                    s.winders = false;
                }
                label = "Stair shape";
            }
            if matches!(s.form(), members::StairShape::L | members::StairShape::U)
                && crate::props::switch(ui, &mut s.winders, "On winders (no landing)").changed()
            {
                label = "Stair winders";
            }
            if matches!(
                s.form(),
                members::StairShape::U | members::StairShape::Spiral
            ) {
                let name = if s.form() == members::StairShape::U {
                    "Well"
                } else {
                    "Eye radius"
                };
                if num(ui, name, &mut s.well, 0.0, 3.0) {
                    label = "Stair well"
                }
            }
            if let Some(l) = &mut s.landing {
                // full rows in the label and field columns (MU31)
                crate::props::row(ui, "Turn after", |ui| {
                    let d = egui::DragValue::new(&mut l.after)
                        .range(1..=n.saturating_sub(1).max(1))
                        .suffix(crate::i18n::t(" risers"));
                    if crate::props::fill(ui, d).changed() {
                        label = "Stair landing";
                    }
                });
                if crate::props::choice(
                    ui,
                    "Turns to",
                    &mut l.left,
                    &[(true, "Left", ""), (false, "Right", "")],
                ) {
                    label = "Stair turn";
                }
            }
            let lay = s.layout(rise);
            let going: f64 = if s.by_steps() {
                lay.risers.saturating_sub(1) as f64 * s.tread
            } else {
                lay.flights.iter().map(|f| f.run(s.tread)).sum()
            };
            let r2g = 2.0 * lay.riser + s.tread;
            weak(
                ui,
                format!(
                    "{} {} {}, {} {}, {} {}. 2h + b = {}.",
                    lay.risers,
                    crate::i18n::t("risers of"),
                    crate::units::len(lay.riser),
                    crate::i18n::t("rise"),
                    crate::units::len(rise),
                    crate::i18n::t("going"),
                    crate::units::len(going),
                    crate::units::len(r2g),
                ),
            );
            if let Some(how) = fit {
                creabim_core::staircheck::keep_to_norm(model, s, how);
            }
            if s.ramp.is_none() && stair_norm_rows(ui, model, s, rise) {
                label = "Stair to the norm";
            }
        }
        ElementKind::Railing(r) => {
            if num(ui, "Height", &mut r.height, 0.2, 3.0) {
                label = "Railing height"
            }
            if r.wall.is_none() && num(ui, "Post spacing", &mut r.post_spacing, 0.1, 5.0) {
                label = "Railing posts"
            }
            if railing_kind_rows(ui, r) {
                label = "Railing type";
            }
            // its top rail from the profile library (MU36)
            crate::props::row(ui, "Handrail profile", |ui| {
                let w = ui.available_width();
                let mut names: Vec<String> = creabim_core::profiles::project_profiles(model)
                    .into_iter()
                    .map(|p| p.name)
                    .collect();
                names.extend(["Handrail", "Tube", "Bar"].iter().map(|s| s.to_string()));
                egui::ComboBox::from_id_salt("rail.profile")
                    .icon(crate::props::combo_icon)
                    .width(w)
                    .truncate()
                    .selected_text(crate::i18n::tw(
                        r.rail_profile
                            .clone()
                            .unwrap_or_else(|| "As the handrail above".into()),
                    ))
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(
                                r.rail_profile.is_none(),
                                crate::i18n::tw("As the handrail above"),
                            )
                            .clicked()
                        {
                            r.rail_profile = None;
                            label = "Railing top rail";
                        }
                        for n in &names {
                            if ui
                                .selectable_label(
                                    r.rail_profile.as_deref() == Some(n.as_str()),
                                    crate::i18n::tw(n),
                                )
                                .clicked()
                            {
                                r.rail_profile = Some(n.clone());
                                label = "Railing top rail";
                            }
                        }
                    });
            });
            // its infill (STN 74 3305 3.2.2), checked in Check railings
            crate::props::row(ui, "Infill", |ui| {
                let w = ui.available_width();
                let now = r.infill.map_or("Rails only", |f| f.kind.label());
                egui::ComboBox::from_id_salt("rail.infill")
                    .icon(crate::props::combo_icon)
                    .width(w)
                    .truncate()
                    .selected_text(crate::i18n::tw(now))
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(r.infill.is_none(), crate::i18n::tw("Rails only"))
                            .clicked()
                        {
                            r.infill = None;
                            label = "Railing infill";
                        }
                        for k in creabim_core::members::InfillKind::ALL {
                            if ui
                                .selectable_label(
                                    r.infill.is_some_and(|f| f.kind == k),
                                    crate::i18n::tw(k.label()),
                                )
                                .clicked()
                            {
                                let mut f = r.infill.unwrap_or_default();
                                f.kind = k;
                                r.infill = Some(f);
                                label = "Railing infill";
                            }
                        }
                    });
            });
            if let Some(f) = &mut r.infill {
                if f.kind != creabim_core::members::InfillKind::Panel
                    && num(ui, "Widest gap", &mut f.gap, 0.02, 1.0)
                {
                    label = "Railing infill";
                }
                if num(ui, "Clear height under it", &mut f.bottom, 0.0, 0.5) {
                    label = "Railing infill";
                }
                let (up, level, _) = creabim_core::railcheck::railing_gaps(r, model);
                let gap = up.max(level);
                if gap > 0.0 {
                    weak(
                        ui,
                        format!(
                            "{} {}",
                            crate::i18n::tr("Gaps as built:"),
                            crate::units::len(gap)
                        ),
                    );
                }
            }
            if let Some(side) = &mut r.stair {
                if crate::props::choice(
                    ui,
                    "Side",
                    &mut side.left,
                    &[(true, "Left", ""), (false, "Right", "")],
                ) {
                    label = "Railing side";
                }
                weak(ui, "Follows its stair.".into());
            }
            weak(
                ui,
                format!(
                    "Length {}, {} posts.",
                    crate::units::len(r.length(model)),
                    r.posts(model).len()
                ),
            );
        }
        _ => {}
    }
    label
}

/// A column's base and top constraints: the level it stands on and how
/// far above it, and the level its top is tied to (or none) and how far
/// above that. Standing it on another level keeps its top where it was.
fn column_levels(ui: &mut egui::Ui, model: &Model, c: &mut Column) -> &'static str {
    let mut label = "";
    let levels = creabim_core::levels::sorted(model);
    let name = |id: Option<ElementId>| {
        id.and_then(|i| levels.iter().find(|l| l.0 == i))
            .map_or("Unconnected".to_string(), |l| l.1.clone())
    };
    let mut base = c.level;
    crate::app::helpui::combo(ui, "Base level")
        .selected_text(crate::i18n::tw(name(Some(c.level))))
        .show_ui(ui, |ui| {
            for (lid, n, _) in levels.iter().rev() {
                ui.selectable_value(&mut base, *lid, crate::i18n::tw(n));
            }
        });
    if base != c.level {
        c.set_base_level(model, base);
        label = "Column base level";
    }
    let num = |ui: &mut egui::Ui, text: &str, v: &mut f64| -> bool {
        crate::props::row(ui, text, |ui| {
            crate::props::fill(ui, crate::units::drag(v).range(-50.0..=50.0)).changed()
        })
    };
    if num(ui, "Base offset", &mut c.base_offset) {
        label = "Column base offset";
    }
    let mut top = c.top_level();
    crate::app::helpui::combo(ui, "Top level")
        .selected_text(crate::i18n::tw(name(top)))
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut top, None, crate::i18n::tw("Unconnected"))
                .on_hover_text(crate::i18n::t("A fixed height, or up to the next level"));
            for (lid, n, _) in levels.iter().rev() {
                ui.selectable_value(&mut top, Some(*lid), crate::i18n::tw(n));
            }
        });
    if top != c.top_level() {
        // tied to a level: the top lands on it, wherever it was
        c.set_top_level(model, top, 0.0);
        label = "Column top level";
    }
    if let creabim_core::WallTop::UpToLevel { offset, .. } = &mut c.top {
        if num(ui, "Top offset", offset) {
            label = "Column top offset";
        }
    }
    label
}

/// A footing's Properties rows: its size, how far below its level (or its
/// host's base) it sits, where it is when it stands free, and its host.
fn footing_fields(ui: &mut egui::Ui, model: &Model, f: &mut creabim_core::Footing) -> &'static str {
    use creabim_core::FootingKind;
    let mut label = "";
    let num = |ui: &mut egui::Ui, text: &str, v: &mut f64, lo: f64, hi: f64| -> bool {
        crate::props::row(ui, text, |ui| {
            crate::props::fill(ui, crate::units::drag(v).range(lo..=hi)).changed()
        })
    };
    // under the fields, in their column, as Blender's notes (MU31)
    let weak = |ui: &mut egui::Ui, t: String| crate::props::note(ui, t);
    let pile = f.kind == FootingKind::Pile;
    if num(
        ui,
        if pile { "Diameter" } else { "Width" },
        &mut f.width,
        0.1,
        10.0,
    ) {
        if pile {
            f.length = f.width;
        }
        label = "Footing width";
    }
    if f.kind == FootingKind::Pad && num(ui, "Length", &mut f.length, 0.1, 10.0) {
        label = "Footing length";
    }
    if num(
        ui,
        if pile { "Length" } else { "Height" },
        &mut f.thickness,
        0.05,
        if pile { 60.0 } else { 5.0 },
    ) {
        label = "Footing thickness";
    }
    let host = f.host.and_then(|h| model.get(h));
    if let Some(mut o) = f.base_offset {
        // set on its level (George 2026-11-20): its underside the offset
        // above the level's floor, moving with it, as a wall's base
        let levels: Vec<(ElementId, String)> = model
            .levels()
            .map(|(e, _)| (e.id, e.name.clone()))
            .collect();
        let current = levels
            .iter()
            .find(|l| l.0 == f.level)
            .map_or(String::new(), |l| l.1.clone());
        crate::app::helpui::combo(ui, "Base level")
            .selected_text(crate::i18n::tw(current))
            .show_ui(ui, |ui| {
                for (lid, n) in &levels {
                    if ui
                        .selectable_value(&mut f.level, *lid, crate::i18n::tw(n))
                        .changed()
                    {
                        label = "Footing level";
                    }
                }
            });
        if num(ui, "Base offset", &mut o, -50.0, 50.0) {
            f.base_offset = Some(o);
            label = "Footing depth";
        }
    } else {
        let below = if host.is_some() {
            match f.kind {
                FootingKind::Strip => "Top below wall base",
                FootingKind::Pad | FootingKind::Pile => "Top below column base",
            }
        } else {
            "Top below level"
        };
        if num(ui, below, &mut f.below, -50.0, 50.0) {
            label = "Footing depth";
        }
        // an older footing: set it on its level where it is
        if ui
            .button(crate::i18n::t("Set on the level"))
            .on_hover_text(crate::i18n::t("Its underside set from the floor of its level by an offset, as a wall's base, moving with the level"))
            .clicked()
        {
            let underside = f.top(model) - f.thickness;
            let level = f.plan_level(model);
            f.level = level;
            f.base_offset = Some(underside - creabim_core::members::elevation(model, level));
            label = "Footing level";
        }
    }
    if f.kind == FootingKind::Strip && host.is_some() {
        crate::app::helpui::combo(ui, "Under the wall")
            .selected_text(crate::i18n::t(f.align.label()))
            .show_ui(ui, |ui| {
                for a in creabim_core::FootingAlign::ALL {
                    if ui
                        .selectable_value(&mut f.align, a, crate::i18n::t(a.label()))
                        .changed()
                    {
                        label = "Footing position";
                    }
                }
            });
        if f.align != creabim_core::FootingAlign::Line
            && num(ui, "Offset", &mut f.align_offset, -1.0, 2.0)
        {
            label = "Footing position";
        }
    }
    match host {
        Some(h) => {
            weak(
                ui,
                format!(
                    "Under {}: it follows it when it moves, and goes when it is deleted.",
                    h.name
                ),
            );
            if ui
                .button(crate::i18n::t("Let go of it"))
                .on_hover_text(crate::i18n::t("Keep the footing where it is, on its own"))
                .clicked()
            {
                *f = creabim_core::footing::detached(model, f);
                label = "Let go of host";
            }
        }
        None => {
            let levels = creabim_core::levels::sorted(model);
            let current = levels
                .iter()
                .find(|l| l.0 == f.level)
                .map_or(String::new(), |l| l.1.clone());
            crate::app::helpui::combo(ui, "Level")
                .selected_text(crate::i18n::tw(current))
                .show_ui(ui, |ui| {
                    for (lid, n, _) in levels.iter().rev() {
                        if ui
                            .selectable_value(&mut f.level, *lid, crate::i18n::tw(n))
                            .changed()
                        {
                            label = "Footing level";
                        }
                    }
                });
            match f.kind {
                FootingKind::Strip => {
                    if num(ui, "Start X", &mut f.start.x, -1e4, 1e4)
                        | num(ui, "Start Y", &mut f.start.y, -1e4, 1e4)
                        | num(ui, "End X", &mut f.end.x, -1e4, 1e4)
                        | num(ui, "End Y", &mut f.end.y, -1e4, 1e4)
                    {
                        label = "Move footing";
                    }
                }
                FootingKind::Pad | FootingKind::Pile => {
                    if num(ui, "X", &mut f.start.x, -1e4, 1e4)
                        | num(ui, "Y", &mut f.start.y, -1e4, 1e4)
                    {
                        f.end = f.start;
                        label = "Move footing";
                    }
                    let r = crate::props::flow(ui, |ui| {
                        ui.label(crate::i18n::t("Rotation"));
                        ui.add(
                            egui::DragValue::new(&mut f.rotation)
                                .speed(0.5)
                                .range(-360.0..=360.0)
                                .suffix("°"),
                        )
                        .changed()
                    });
                    if r.inner {
                        label = "Turn footing";
                    }
                }
            }
        }
    }
    let top = f.top(model);
    weak(
        ui,
        format!(
            "{}: top at {}, underside at {}.",
            match f.kind {
                FootingKind::Strip =>
                    format!("Strip {} long", crate::units::len(f.strip_length(model))),
                FootingKind::Pad => "Pad".to_string(),
                FootingKind::Pile => "Pile".to_string(),
            },
            crate::units::elevation(top),
            crate::units::elevation(top - f.thickness)
        ),
    );
    label
}

/// A stair's carrier, stringer and risers in Properties (its finishes'
/// materials are picked under them, by the app).
fn stair_build_fields(ui: &mut egui::Ui, s: &mut Stair) -> &'static str {
    let mut label = "";
    let now = s.carrier;
    crate::props::row(ui, "Construction", |ui| {
        egui::ComboBox::from_id_salt("stair-carrier")
            .icon(crate::props::combo_icon)
            .width(ui.available_width())
            .selected_text(crate::i18n::tr(now.label()))
            .show_ui(ui, |ui| {
                for c in members::StairCarrier::ALL {
                    if ui
                        .selectable_label(c == now, crate::i18n::tr(c.label()))
                        .clicked()
                        && c != now
                    {
                        s.carrier = c;
                        label = "Stair construction";
                    }
                }
            });
    });
    if label == "Stair construction" && !s.carrier.is_concrete() {
        // on stringers: a stringer, and treads that are boards
        if s.stringer.is_none() {
            s.stringer = members::stringer_catalogue().into_iter().next();
        }
        let t = s.tread_finish.get_or_insert(members::StepFinish {
            material: None,
            thickness: members::OAK_TREAD_BOARD,
        });
        if t.thickness < members::OAK_TREAD_CLADDING {
            t.thickness = members::OAK_TREAD_BOARD;
        }
    }
    if !s.carrier.is_concrete() {
        let name = s
            .stringer
            .as_ref()
            .map_or(String::new(), |x| x.name.clone());
        crate::props::row(ui, "Stringer", |ui| {
            egui::ComboBox::from_id_salt("stair-stringer")
                .icon(crate::props::combo_icon)
                .width(ui.available_width())
                .selected_text(name.clone())
                .show_ui(ui, |ui| {
                    for x in members::stringer_catalogue() {
                        if ui.selectable_label(x.name == name, &x.name).clicked() {
                            let off = s.stringer.as_ref().map_or(0.0, |o| o.edge_offset);
                            s.stringer = Some(members::Stringer {
                                edge_offset: off,
                                ..x
                            });
                            label = "Stair stringer";
                        }
                    }
                });
        });
        if s.carrier == members::StairCarrier::TwoStringers {
            if let Some(st) = &mut s.stringer {
                if crate::props::row(ui, "From the edge", |ui| {
                    crate::props::fill(ui, crate::units::drag(&mut st.edge_offset).range(0.0..=0.5))
                        .changed()
                }) {
                    label = "Stringer offset";
                }
            }
        }
    }
    if crate::props::switch(ui, &mut s.risers_closed, "Risers")
        .on_hover_text(crate::i18n::t(
            "Closed risers (faced on concrete, boarded on stringers); off, an open stair or bare concrete risers",
        ))
        .changed()
    {
        label = "Stair risers";
    }
    label
}

/// The Stair tool's build-up settings: carrier, stringer, finishes,
/// risers (George 2026-11-24); none for a ramp.
/// The stair against STN 73 4130 and STN 73 4301, from the norms' tables
/// in the project: what fails, and a button setting the going, the
/// walking line and the winders as the norm wants. Without the tables, a
/// line saying where to load them. True when the stair was changed.
fn stair_norm_rows(ui: &mut egui::Ui, model: &Model, s: &mut Stair, rise: f64) -> bool {
    use creabim_core::staircheck::{self, StairFault as F, StairRules};
    let t = crate::i18n::t;
    let mut changed = false;
    if crate::props::choice(
        ui,
        "Building",
        &mut s.flats,
        &[
            (
                false,
                "Family house",
                "A family house, or a stair inside a flat (STN 73 4301, 6.2.3)",
            ),
            (
                true,
                "Block of flats",
                "The main stair of a block of flats (STN 73 4301, 5.2.5, 5.2.6)",
            ),
        ],
    ) {
        changed = true;
    }
    if s.flats && crate::props::switch(ui, &mut s.lift, "The building has a lift").changed() {
        changed = true;
    }
    if crate::props::switch(ui, &mut s.to_norm, "Keep to the norm")
        .on_hover_text(crate::i18n::t(
            "Changing the rise, risers, going, width or length, the stair takes the riser height, going and count of risers the norm wants",
        ))
        .changed()
    {
        if s.to_norm {
            creabim_core::staircheck::keep_to_norm(model, s, creabim_core::staircheck::Fit::Rise);
        }
        changed = true;
    }
    let tables = creabim_core::room::project_tables(model);
    let Some(rules) = StairRules::for_stair(&tables, s) else {
        ui.label(
            egui::RichText::new(t(
                "To check the stair against STN 73 4130, read its tables in Manage › Norm tables.",
            ))
            .weak(),
        );
        return changed;
    };
    let mm = |v: f64| crate::i18n::num(v * 1000.0, 0);
    let mut found = staircheck::check(s, rise, &rules);
    // headroom and clear height over it (čl. 22, 23): measured in the
    // model, again only when the model or the stair changed
    let (h1, v) = stair_clearance_cached(model, s);
    let angle = (s.layout(rise).riser / s.tread.max(1e-6))
        .atan()
        .to_degrees();
    if s.ramp.is_none() {
        found.extend(rules.check_height(angle, h1, v));
    }
    if found.is_empty() {
        let text = if s.flats {
            "Meets STN 73 4130 and STN 73 4301 (block of flats)."
        } else {
            "Meets STN 73 4130 and STN 73 4301 (family house)."
        };
        ui.label(egui::RichText::new(t(text)).color(egui::Color32::from_rgb(30, 130, 60)));
    }
    for f in &found {
        let text = match f {
            F::Sum { have, min, max } => {
                format!(
                    "2h + b = {} mm, {} {}–{} mm (čl. 30)",
                    mm(*have),
                    t("should be"),
                    mm(*min),
                    mm(*max)
                )
            }
            F::Class {
                angle,
                class: Some(c),
                riser,
                range: Some((a, b)),
            } => format!(
                "{} {:.1}° ({c}): h = {} mm, {} {}–{} mm (čl. 12)",
                t("slope"),
                angle,
                mm(*riser),
                t("should be"),
                mm(*a),
                mm(*b)
            ),
            F::Class { angle, .. } => format!(
                "{} {:.1}°: {} (čl. 12)",
                t("slope"),
                angle,
                t("steeper than any class")
            ),
            F::Going { have, min } => format!("b = {} < {} mm (čl. 29)", mm(*have), mm(*min)),
            F::Tread { have, min } => {
                format!("{} {} < {} mm (čl. 29)", t("tread"), mm(*have), mm(*min))
            }
            F::Narrow { have, min } => format!(
                "{} {} < {} mm (čl. 28)",
                t("winders' narrow end"),
                mm(*have),
                mm(*min)
            ),
            F::Risers { have, max } => {
                format!("{} {} > {} (čl. 31)", t("risers in a flight"), have, max)
            }
            F::Walk { have, need } => format!(
                "{} {} mm, {} {} mm (čl. 16)",
                t("walking line from the outer edge"),
                mm(*have),
                t("should be"),
                mm(*need)
            ),
            F::Width { have, min } => format!(
                "{} {} < {} mm (STN 73 4301)",
                t("width"),
                mm(*have),
                mm(*min)
            ),
            F::Steep { angle, max } => {
                format!("{} {:.1}° > {:.0}° (STN 73 4301)", t("slope"), angle, max)
            }
            F::Headroom { have, need } => format!(
                "{} {} < {} mm (čl. 22)",
                t("headroom"),
                mm(*have),
                mm(*need)
            ),
            F::Clear { have, need } => format!(
                "{} {} < {} mm (čl. 23)",
                t("clear height"),
                mm(*have),
                mm(*need)
            ),
        };
        ui.label(egui::RichText::new(text).color(egui::Color32::from_rgb(190, 40, 30)));
    }
    let fixable = found.iter().any(|f| {
        matches!(
            f,
            F::Sum { .. }
                | F::Walk { .. }
                | F::Narrow { .. }
                | F::Class { .. }
                | F::Going { .. }
                | F::Tread { .. }
        )
    });
    if fixable && ui.button(t("Set to the norm")).on_hover_text(t("The going from 2h + b, the walking line and the winders' narrow ends as the norm wants")).clicked() {
        rules.apply(s, rise);
        return true;
    }
    changed
}

/// A stair's clearances keyed by model revision and the stair itself.
type ClearanceCache = Option<(u64, String, (Option<f64>, Option<f64>))>;

thread_local! {
    static CLEARANCE: std::cell::RefCell<ClearanceCache> =
        const { std::cell::RefCell::new(None) };
}

/// [`creabim_geom::stairsteps::stair_clearance`], kept while neither the
/// model nor the stair changes.
fn stair_clearance_cached(model: &Model, s: &Stair) -> (Option<f64>, Option<f64>) {
    let key = format!("{s:?}");
    let rev = model.revision();
    CLEARANCE.with(|c| {
        if let Some((r, k, v)) = &*c.borrow() {
            if *r == rev && *k == key {
                return *v;
            }
        }
        let v = creabim_geom::stairsteps::stair_clearance(model, s);
        *c.borrow_mut() = Some((rev, key, v));
        v
    })
}

fn stair_build_options(m: &MemberSettings) -> Vec<serde_json::Value> {
    use serde_json::json;
    if m.stair_ramp {
        return vec![];
    }
    let b = &m.stair_build;
    use members::StairShape as S;
    let mut v = vec![json!({
        "kind": "menu", "key": "stair.shape", "label": "Shape",
        "value": b.shape.label(),
        "hint": "The stair's plan: straight, L or U turning halfway up (on a landing or on winders), or spiral",
    })];
    if !b.shape.is_straight() {
        v.push(json!({
            "kind": "toggle", "id": "member:stair.left", "label": "Turns left", "on": b.left,
            "icon": "corner-up-left",
            "hint": "The stair turns to the left going up; off, to the right",
        }));
    }
    if matches!(b.shape, S::L | S::U) {
        v.push(json!({
            "kind": "toggle", "id": "member:stair.winders", "label": "Winders", "on": b.winders,
            "icon": "stair",
            "hint": "Turn on winders (kosé stupne), no landing; their inner ends spread evenly along the inner edge",
        }));
    }
    if matches!(b.shape, S::U | S::Spiral) {
        let (label, hint) = if b.shape == S::U {
            (
                "Well",
                "The gap (zrkadlo) between the two flights of a U stair",
            )
        } else {
            (
                "Eye radius",
                "The radius of the spiral's eye, inside its inner edge",
            )
        };
        v.push(json!({
            "kind": "number", "key": "member.stair_well", "label": label,
            "value": b.well, "unit": "m", "step": 0.01, "min": 0.0, "max": 3.0,
            "hint": hint,
        }));
    }
    v.push(json!({
        "kind": "menu", "key": "stair.carrier", "label": "Construction",
        "value": b.carrier.label(),
        "hint": "What carries the steps: a reinforced concrete flight, one stringer under the middle, or two stringers set in from the edges",
    }));
    if !b.carrier.is_concrete() {
        v.push(json!({
            "kind": "menu", "key": "stair.stringer", "label": "Stringer",
            "value": b.stringer,
            "hint": "The stringers' section: KVH timber (DEKWOOD) or rolled steel IPE, UPN, UPE, L (ArcelorMittal)",
        }));
        if b.carrier == members::StairCarrier::TwoStringers {
            v.push(json!({
                "kind": "number", "key": "member.stair_edge", "label": "From the edge",
                "value": b.edge_offset, "unit": "m", "step": 0.005, "min": 0.0, "max": 0.5,
                "hint": "How far the stringers' outer faces are set in from the stair's edges",
            }));
        }
    }
    v.push(json!({
        "kind": "menu", "key": "stair.tread", "label": "Tread",
        "value": b.tread.label(),
        "hint": "The treads: oak boards (DREVAX: 40 mm on stringers, 27 mm on concrete) or ceramic stair tiles (RAKO, 9 mm, on concrete)",
    }));
    v.push(json!({
        "kind": "toggle", "id": "member:stair.risers", "label": "Risers", "on": b.risers_closed,
        "icon": "stair",
        "hint": "Closed risers (faced on concrete, boarded on stringers); off, an open stair or bare concrete risers",
    }));
    if b.risers_closed {
        v.push(json!({
            "kind": "menu", "key": "stair.riser", "label": "Riser board",
            "value": b.riser.label(),
            "hint": "The risers: oak (DREVAX: 20 mm boards on stringers, 18 mm on concrete) or ceramic tiles (RAKO, 9 mm, on concrete)",
        }));
    }
    v
}

/// The tool bar's settings for the active tool: sizes as numbers, shapes
/// as toggles (`member:<what>` actions).
pub fn tool_options(state: &PlanState) -> Vec<serde_json::Value> {
    use serde_json::json;
    let m = &state.members;
    let num = |key: &str,
               label: &str,
               value: f64,
               unit: &str,
               step: f64,
               min: f64,
               max: f64,
               hint: &str| {
        json!({
            "kind": "number", "key": format!("member.{key}"), "label": label, "value": value,
            "unit": unit, "step": step, "min": min, "max": max, "hint": hint,
        })
    };
    let toggle = |id: &str, label: &str, on: bool, icon: &str, hint: &str| {
        json!({
            "kind": "toggle", "id": format!("member:{id}"), "label": label, "on": on,
            "icon": icon, "hint": hint,
        })
    };
    let opening = |key: &str, s: [f64; 3], what: &str| {
        // a door is typed by its leaf
        let (wl, hl, wh) = if key == "door" {
            (
                "Leaf width",
                "Leaf height",
                "Width of the leaf of new doors, without the frame: the opening in the wall follows from it. Typing a size here places it without a type.".to_string(),
            )
        } else {
            (
                "Width",
                "Height",
                format!("Width of new {what}s. Typing a size here places it without a type."),
            )
        };
        vec![
            num(&format!("{key}_width"), wl, s[0], "m", 0.01, 0.1, 20.0, &wh),
            num(
                &format!("{key}_height"),
                hl,
                s[1],
                "m",
                0.01,
                0.1,
                20.0,
                &format!("Height of new {what}s"),
            ),
            num(
                &format!("{key}_sill"),
                "Sill",
                s[2],
                "m",
                0.01,
                0.0,
                20.0,
                &format!("How high new {what}s start above the wall's base"),
            ),
        ]
    };
    let framed = m.framing.is_some() && matches!(state.tool, Tool::Beam | Tool::Column);
    let mut out = match state.tool {
        // a roof's timber or a steel member (Statika tab)
        Tool::Beam | Tool::Column if framed => crate::app::framingui::tool_options(m),
        Tool::Door | Tool::Window => {
            let mut v = if state.tool == Tool::Door {
                opening("door", m.door, "door")
            } else {
                opening("window", m.window, "window")
            };
            v.push(toggle(
                "tagnew",
                "Tag",
                state.tag_new,
                "tag",
                "Tag each new door and window in the plans of its level as it is placed",
            ));
            v
        }
        Tool::Opening => {
            let mut v = vec![
                toggle("hole.auto", "Auto", m.hole_host == 0, "wall-opening", "Into the wall clicked, else the floor slab, else the ceiling slab"),
                toggle("hole.wall", "Wall", m.hole_host == 1, "wall", "Into walls only"),
                toggle("hole.floor", "Floor (slab)", m.hole_host == 2, "slab", "Into this storey's floor slab only"),
                toggle("hole.ceiling", "Ceiling (slab)", m.hole_host == 3, "ceiling", "Into the slab over this storey only: drawn with a dash and two dots, labelled (strop)"),
                toggle("hole.rect", "Rectangle", !m.hole_round, "square-dashed", "Rectangular penetrations"),
                toggle("hole.round", "Circle", m.hole_round, "circle-dot", "Round penetrations: the width is the diameter"),
            ];
            v.extend(opening("hole", m.hole, "penetration"));
            v
        }
        Tool::Column => vec![
            toggle(
                "square",
                "Square",
                m.column_section.is_none() && !m.round_columns(),
                "square-dashed",
                "Rectangular columns",
            ),
            toggle(
                "round",
                "Round",
                m.column_section.is_none() && m.round_columns(),
                "circle-dot",
                "Round columns",
            ),
            // or any profile of the library: an I, a T, an L, your own (MU40)
            json!({
                "kind": "menu", "key": "column.library", "label": "Profile",
                "value": m.column_section.clone().unwrap_or_else(|| "Library".into()), "icon": "profile",
                "on": m.column_section.is_some(),
                "hint": "Columns of a profile from the library: presets and the project's own",
            }),
            num(
                "column",
                if m.round_columns() {
                    "Diameter"
                } else {
                    "Size"
                },
                m.column_size(),
                "m",
                0.01,
                0.05,
                3.0,
                "Size of new columns",
            ),
        ],
        Tool::Pipe => crate::pipesui::tool_options(&m.pipe),
        Tool::Gutter
        | Tool::Downpipe
        | Tool::Capping
        | Tool::ChimneyFlashing
        | Tool::RoofOutlet => crate::flashingui::tool_options(&m.flashing, state.tool),
        Tool::Beam => vec![
            json!({
                "kind": "menu", "key": "beam.corner", "label": "Corners",
                "value": m.beam_corner.label(), "icon": m.beam_corner.icon(),
                "hint": "How the beams of a chain meet: Separate (concrete pieces), Mitred (welded steel) or Butt (one runs through, the next stops against it)",
            }),
            toggle(
                "beam.onface",
                "On face",
                m.beam_on_face,
                "scan",
                "In 3D, beams lie on the face clicked (a roof slope, a wall), square to it: purlins, rafters, braces",
            ),
            json!({
                "kind": "menu", "key": "beam.library", "label": "Profile",
                "value": m.beam_section.clone().unwrap_or_else(|| "Its own".into()), "icon": "profile",
                "on": m.beam_section.is_some(),
                "hint": "Beams of a profile from the library (an I, a T, an L, your own), or their own width × depth",
            }),
            num(
                "beam_width",
                "Width",
                m.beam_width,
                "m",
                0.01,
                0.05,
                2.0,
                "Width of new beams",
            ),
            num(
                "beam_depth",
                "Depth",
                m.beam_depth,
                "m",
                0.01,
                0.05,
                3.0,
                "Depth of new beams",
            ),
        ],
        Tool::Roof => {
            let mut v: Vec<serde_json::Value> = RoofShape::ALL
                .iter()
                .map(|s| {
                    toggle(
                        &format!("roof.{}", s.label()),
                        s.label(),
                        m.roof_shape == *s,
                        roof_icon(*s),
                        "Shape of new roofs",
                    )
                })
                .collect();
            if m.roof_shape != RoofShape::Flat {
                v.push(num(
                    "pitch",
                    "Pitch",
                    m.roof_pitch,
                    "°",
                    1.0,
                    1.0,
                    75.0,
                    "Slope of new roofs",
                ));
            }
            v
        }
        Tool::Stair => vec![
            toggle(
                "stair.steps",
                "Stair",
                !m.stair_ramp,
                "stair",
                "Steps: risers and treads climbing to the next level",
            ),
            toggle(
                "stair.ramp",
                "Ramp",
                m.stair_ramp,
                "slope-dim",
                "A ramp: one sloping run, 1 : 12 (8,33 %) unless set otherwise; railings fix to it as to a stair",
            ),
            num(
                "stair_width",
                "Width",
                m.stair_width,
                "m",
                0.05,
                0.5,
                5.0,
                "Width of new stairs",
            ),
        ]
        .into_iter()
        .chain(stair_build_options(m))
        .collect(),
        Tool::Footing => vec![
            toggle(
                "footing.strip",
                "Strip",
                m.footing_kind == creabim_core::FootingKind::Strip,
                "footing",
                "Strip footings: click a wall, or draw them point by point as walls (Esc ends the chain)",
            ),
            toggle(
                "footing.pad",
                "Pad",
                m.footing_kind == creabim_core::FootingKind::Pad,
                "square-dashed",
                "Pad footings: click a column (centred on its axis), or anywhere",
            ),
            toggle(
                "footing.pile",
                "Pile",
                m.footing_kind == creabim_core::FootingKind::Pile,
                "cylinder",
                "Bored piles: click a column (on its axis), or anywhere; the head on the level's floor plus the base offset, the toe its length below",
            ),
            num(
                "footing_width",
                match m.footing_kind {
                    creabim_core::FootingKind::Strip => "Width",
                    creabim_core::FootingKind::Pad => "Size",
                    creabim_core::FootingKind::Pile => "Diameter",
                },
                m.footing_width,
                "m",
                0.05,
                0.1,
                10.0,
                "Width of new strips, the sides of new pads, the diameter of new piles",
            ),
            num(
                "footing_thickness",
                if m.footing_kind == creabim_core::FootingKind::Pile {
                    "Length"
                } else {
                    "Height"
                },
                m.footing_thickness,
                "m",
                0.05,
                0.05,
                30.0,
                "Thickness of new footings",
            ),
            // a pile is set by its head, its toe its length below
            if m.footing_kind == creabim_core::FootingKind::Pile {
                num(
                    "footing_base",
                    "Head offset",
                    m.footing_base,
                    "m",
                    0.05,
                    -20.0,
                    20.0,
                    "How far above the floor of the level they are modelled on new piles' heads sit",
                )
            } else {
                num(
                    "footing_base",
                    "Base offset",
                    m.footing_base,
                    "m",
                    0.05,
                    -20.0,
                    20.0,
                    "How far above the floor of the level they are modelled on new footings' undersides sit",
                )
            },
        ]
        .into_iter()
        .chain((m.footing_kind == creabim_core::FootingKind::Strip).then(|| json!({
            "kind": "menu", "key": "footing.align", "label": "Under the wall",
            "value": m.footing_align.label(), "icon": "footing",
            "hint": "Where new strips lie across their walls: centred, or with an edge set by the face of the wall's load-bearing layer",
        })))
        .chain((m.footing_kind == creabim_core::FootingKind::Strip && m.footing_align != creabim_core::FootingAlign::Line).then(|| {
            num(
                "footing_offset",
                "Offset",
                m.footing_offset,
                "m",
                0.005,
                -1.0,
                2.0,
                "How far the strip's edge stands proud of the core face (negative: set back under the wall)",
            )
        }))
        .chain(std::iter::once(json!({
            "kind": "menu", "key": "footing.material", "label": "Material",
            "value": m.footing_material.clone().unwrap_or_else(|| "By type".into()),
            "hint": "What new footings are made of",
        })))
        .collect(),
        Tool::Railing => vec![
            json!({
                "kind": "menu", "key": "railing.type", "label": "Type",
                "value": m.railing_style.kind.map_or("Rails only", |k| k.label()),
                "hint": "Upright bars, horizontal bars, safety glass, or a handrail alone on a wall",
            }),
            json!({
                "kind": "menu", "key": "railing.metal", "label": "Metal",
                "value": m.railing_style.metal.label(),
                "hint": "Stainless steel (UMAKOV) or aluminium (ALZABRADLI): the sizes of the posts, handrail and bars come from its catalogue",
            }),
            json!({
                "kind": "menu", "key": "railing.round", "label": "Cross-section",
                "value": if m.railing_style.round { "Round profile" } else { "Square profile" },
                "hint": "Round tubes or square (JAKL) profiles",
            }),
            json!({
                "kind": "menu", "key": "railing.fixing", "label": "Fixing",
                "value": m.railing_style.fixing.label(),
                "hint": "From the top on a flange, or from the side into the edge of the slab, stair or wall",
            }),
            json!({
                "kind": "menu", "key": "railing.library", "label": "Rail",
                "value": m.railing_section.clone().unwrap_or_else(|| "Its own".into()), "icon": "profile",
                "on": m.railing_section.is_some(),
                "hint": "The top rail's profile from the library (a handrail, a tube, your own), or its own square bar",
            }),
            num(
                "railing_height",
                "Height",
                m.railing_height,
                "m",
                0.05,
                0.3,
                3.0,
                "Height of new railings",
            ),
        ],
        _ => vec![],
    };
    if !out.is_empty() {
        out.push(json!({"kind": "sep"}));
    }
    out
}

/// A number typed in the tool bar.
pub fn set_option(state: &mut PlanState, key: &str, v: f64) {
    let m = &mut state.members;
    if crate::app::framingui::set_option(m, key.trim_start_matches("member."), v) {
        return;
    }
    match key.trim_start_matches("member.") {
        "column" => m.set_column_size(v.clamp(0.05, 3.0)),
        "beam_width" => m.beam_width = v.clamp(0.05, 2.0),
        "pipe_height" => {
            m.pipe.height = v.clamp(-5.0, 20.0);
            if state.pending.is_some() {
                // a run is being drawn: it rises or drops there
                m.pipe.height_set = true;
            } else {
                m.pipe.last_z = None;
                m.pipe.height_set = false;
            }
        }
        "pipe_slope" => m.pipe.slope_pct = v.clamp(0.0, 100.0),
        k if k.starts_with("flashing.") => crate::flashingui::set_option(&mut m.flashing, k, v),
        "beam_depth" => m.beam_depth = v.clamp(0.05, 3.0),
        "pitch" => m.roof_pitch = v.clamp(1.0, 75.0),
        "stair_width" => m.stair_width = v.clamp(0.5, 5.0),
        "stair_edge" => m.stair_build.edge_offset = v.clamp(0.0, 0.5),
        "stair_well" => m.stair_build.well = v.clamp(0.0, 3.0),
        "railing_height" => m.railing_height = v.clamp(0.3, 3.0),
        "footing_width" => m.footing_width = v.clamp(0.1, 10.0),
        "footing_thickness" => m.footing_thickness = v.clamp(0.05, 5.0),
        "footing_offset" => m.footing_offset = v.clamp(-1.0, 2.0),
        "footing_base" => m.footing_base = v.clamp(-20.0, 20.0),
        k if k.starts_with("door_") || k.starts_with("window_") || k.starts_with("hole_") => {
            let (kind, what) = k.split_once('_').unwrap_or((k, ""));
            let s = match kind {
                "door" => &mut m.door,
                "window" => &mut m.window,
                _ => &mut m.hole,
            };
            match what {
                "width" => s[0] = v.clamp(0.1, 20.0),
                "height" => s[1] = v.clamp(0.1, 20.0),
                "sill" => s[2] = v.clamp(0.0, 20.0),
                _ => {}
            }
        }
        _ => {}
    }
}

/// A toggle clicked in the tool bar.
pub fn toggle_option(state: &mut PlanState, what: &str) {
    if what == "tagnew" {
        state.tag_new = !state.tag_new;
        return;
    }
    let m = &mut state.members;
    if crate::app::framingui::toggle_option(m, what) {
        return;
    }
    match what {
        "beam.onface" => m.beam_on_face = !m.beam_on_face,
        w if w.starts_with("flashing.") => crate::flashingui::toggle_option(&mut m.flashing, w),
        "hole.auto" => m.hole_host = 0,
        "hole.wall" => m.hole_host = 1,
        "hole.floor" => m.hole_host = 2,
        "hole.ceiling" => m.hole_host = 3,
        "hole.rect" => m.hole_round = false,
        "hole.round" => m.hole_round = true,
        "square" => m.set_round(false),
        "round" => m.set_round(true),
        "stair.winders" => m.stair_build.winders = !m.stair_build.winders,
        "stair.left" => m.stair_build.left = !m.stair_build.left,
        "stair.steps" => m.stair_ramp = false,
        "stair.ramp" => m.stair_ramp = true,
        "stair.risers" => m.stair_build.risers_closed = !m.stair_build.risers_closed,
        "footing.strip" | "footing.pad" | "footing.pile" => {
            use creabim_core::{footing as fo, FootingKind};
            let kind = match what {
                "footing.pad" => FootingKind::Pad,
                "footing.pile" => FootingKind::Pile,
                _ => FootingKind::Strip,
            };
            if kind != m.footing_kind {
                m.footing_kind = kind;
                // each kind starts at its usual size
                (m.footing_width, m.footing_thickness) = match kind {
                    FootingKind::Pad => (fo::PAD_SIZE, fo::PAD_THICKNESS),
                    FootingKind::Pile => (fo::PILE_DIAMETER, fo::PILE_LENGTH),
                    FootingKind::Strip => (fo::STRIP_WIDTH, fo::STRIP_THICKNESS),
                };
            }
        }
        w => {
            if let Some(s) = RoofShape::ALL
                .into_iter()
                .find(|s| w == format!("roof.{}", s.label()))
            {
                m.roof_shape = s;
            }
        }
    }
}

/// The interface icon of a roof shape: each its own.
pub(crate) fn roof_icon(s: RoofShape) -> &'static str {
    match s {
        RoofShape::Flat => "roof-flat",
        RoofShape::Shed => "roof-mono",
        RoofShape::Gable => "roof-gable",
        RoofShape::Hip => "roof-hip",
    }
}

/// A roof shape's outline, drawn small in `r` for its drop-down: the
/// same pictures as the icons.
fn roof_glyph(p: &egui::Painter, r: egui::Rect, st: egui::Stroke, s: RoofShape) {
    let at = |x: f32, y: f32| egui::pos2(r.left() + x * r.width(), r.top() + y * r.height());
    let line = |pts: &[(f32, f32)]| {
        let pts: Vec<egui::Pos2> = pts.iter().map(|&(x, y)| at(x, y)).collect();
        p.add(egui::Shape::line(pts, st));
    };
    match s {
        RoofShape::Flat => {
            line(&[(0.1, 0.4), (0.9, 0.4), (0.9, 0.55), (0.1, 0.55), (0.1, 0.4)]);
            line(&[(0.2, 0.55), (0.2, 0.85)]);
            line(&[(0.8, 0.55), (0.8, 0.85)]);
        }
        RoofShape::Shed => {
            line(&[(0.1, 0.85), (0.1, 0.6), (0.9, 0.3), (0.9, 0.85)]);
        }
        RoofShape::Gable => {
            line(&[(0.1, 0.55), (0.5, 0.2), (0.9, 0.55)]);
            line(&[(0.2, 0.47), (0.2, 0.85), (0.8, 0.85), (0.8, 0.47)]);
        }
        RoofShape::Hip => {
            line(&[
                (0.1, 0.65),
                (0.3, 0.3),
                (0.7, 0.3),
                (0.9, 0.65),
                (0.1, 0.65),
            ]);
            line(&[(0.2, 0.65), (0.2, 0.85), (0.8, 0.85), (0.8, 0.65)]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::sample::sample_house;

    #[test]
    fn a_wall_is_hit_across_its_whole_thickness_whatever_its_location_line() {
        let (doc, level, _, _) = setup();
        let m = doc.model();
        // the starter house's walls are drawn on several location lines
        let walls: Vec<_> = m
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Wall(w) if w.level == level && w.curtain.is_none() => {
                    Some((e.id, w.clone()))
                }
                _ => None,
            })
            .collect();
        assert!(!walls.is_empty());
        for (id, w) in walls {
            let n = w.direction().perp();
            let mid = (w.start + w.end) / 2.0;
            for s in [-0.45, 0.0, 0.45] {
                let p = mid + n * s * w.thickness;
                assert!(
                    walls_at(m, level, p).contains(&id),
                    "{s} of its thickness off the centre"
                );
            }
            assert!(!walls_at(m, level, mid + n * (w.thickness / 2.0 + 0.2)).contains(&id));
        }
    }

    #[test]
    fn a_host_tool_finds_its_host_before_the_first_click_only() {
        let (doc, level, mut state, _) = setup();
        let m = doc.model();
        let (id, w) = m
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::Wall(w) if w.level == level && w.curtain.is_none() => {
                    Some((e.id, w.clone()))
                }
                _ => None,
            })
            .unwrap();
        let p = (w.start + w.end) / 2.0;
        for tool in [Tool::Footing, Tool::Door, Tool::Window] {
            assert_eq!(
                hosts_at(m, level, tool, &state, p).first(),
                Some(&id),
                "{tool:?}"
            );
        }
        assert!(host_preview(m, &state, Tool::Footing, id).contains("under"));
        // a free strip with its first point snaps as usual
        state.pending = Some(DVec2::ZERO);
        assert!(hosts_at(m, level, Tool::Footing, &state, p).is_empty());
        assert!(!hosted(Tool::Wall));
    }

    fn setup() -> (Document, ElementId, PlanState, Selection) {
        let doc = Document::from_model(sample_house());
        let level = doc
            .model()
            .levels()
            .min_by(|a, b| a.1.elevation.total_cmp(&b.1.elevation))
            .unwrap()
            .0
            .id;
        (doc, level, PlanState::default(), Selection::default())
    }

    fn count(doc: &Document, f: fn(&ElementKind) -> bool) -> usize {
        doc.model().iter().filter(|e| f(&e.kind)).count()
    }

    fn active<'a>(doc: &'a Document, sel: &Selection) -> Option<&'a ElementKind> {
        sel.active
            .and_then(|id| doc.model().get(id))
            .map(|e| &e.kind)
    }

    #[test]
    fn tools_place_each_member_and_undo() {
        let (mut doc, level, mut st, mut sel) = setup();
        let mut go = |doc: &mut Document, st: &mut PlanState, tool, x, y| {
            click(doc, level, tool, DVec2::new(x, y), st, &mut sel).unwrap();
        };
        go(&mut doc, &mut st, Tool::Column, 1.0, 1.0);
        assert_eq!(count(&doc, |k| matches!(k, ElementKind::Column(_))), 1);
        for (x, y) in [(0.0, 0.0), (4.0, 0.0), (4.0, 3.0)] {
            go(&mut doc, &mut st, Tool::Beam, x, y);
        }
        assert_eq!(count(&doc, |k| matches!(k, ElementKind::Beam(_))), 2);
        st.cancel();
        st.members.stair_build.shape = members::StairShape::L;
        go(&mut doc, &mut st, Tool::Stair, 20.0, 0.0);
        go(&mut doc, &mut st, Tool::Stair, 21.0, 0.0);
        assert_eq!(
            count(
                &doc,
                |k| matches!(k, ElementKind::Stair(s) if s.landing.is_some())
            ),
            1
        );
        // a click on the stair rails both its sides
        go(&mut doc, &mut st, Tool::Railing, 20.5, 0.0);
        assert_eq!(count(&doc, |k| matches!(k, ElementKind::Railing(_))), 2);
        // a free path: click the last point again to finish
        for (x, y) in [(30.0, 0.0), (33.0, 0.0), (33.0, 0.0)] {
            go(&mut doc, &mut st, Tool::Railing, x, y);
        }
        assert_eq!(count(&doc, |k| matches!(k, ElementKind::Railing(_))), 3);
        assert!(st.members.path.is_empty());
        doc.undo();
        assert_eq!(count(&doc, |k| matches!(k, ElementKind::Railing(_))), 2);
    }

    #[test]
    fn the_footing_tool_puts_strips_under_walls_and_pads_under_columns() {
        use creabim_core::{footing, FootingKind};
        let (mut doc, level, mut st, mut sel) = setup();
        let mut go = |doc: &mut Document, st: &mut PlanState, x: f64, y: f64| {
            click(doc, level, Tool::Footing, DVec2::new(x, y), st, &mut sel).unwrap();
        };
        let (wall, mid) = doc
            .model()
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::Wall(w) if w.level == level && w.curtain.is_none() => {
                    Some((e.id, (w.start + w.end) / 2.0))
                }
                _ => None,
            })
            .unwrap();
        // a click on a wall: a strip under it, hosted
        go(&mut doc, &mut st, mid.x, mid.y);
        assert_eq!(footing::footings_under(doc.model(), wall).len(), 1);
        // a second click there adds none
        go(&mut doc, &mut st, mid.x, mid.y);
        assert_eq!(footing::footings_under(doc.model(), wall).len(), 1);
        assert!(st.status.as_deref().unwrap_or("").contains("already"));
        // two clicks away from walls: a strip on its own
        go(&mut doc, &mut st, 50.0, 50.0);
        go(&mut doc, &mut st, 54.0, 50.0);
        let free = |doc: &Document, kind: FootingKind| {
            doc.model()
                .iter()
                .filter(|e| matches!(&e.kind, ElementKind::Footing(f) if f.kind == kind && f.host.is_none()))
                .count()
        };
        assert_eq!(free(&doc, FootingKind::Strip), 1);
        // drawn on in a chain as walls: the next click adds the next strip
        go(&mut doc, &mut st, 54.0, 54.0);
        assert_eq!(free(&doc, FootingKind::Strip), 2);
        assert_eq!(st.pending, Some(DVec2::new(54.0, 54.0)));
        // Esc ends the chain
        st.pending = None;
        // pads: under a column, and on their own, at the pad size
        click(
            &mut doc,
            level,
            Tool::Column,
            DVec2::new(70.0, 70.0),
            &mut st,
            &mut sel,
        )
        .unwrap();
        let column = sel.active.unwrap();
        toggle_option(&mut st, "footing.pad");
        assert_eq!(st.members.footing_kind, FootingKind::Pad);
        let mut go = |doc: &mut Document, st: &mut PlanState, x: f64, y: f64| {
            click(doc, level, Tool::Footing, DVec2::new(x, y), st, &mut sel).unwrap();
        };
        go(&mut doc, &mut st, 70.0, 70.0);
        let under = footing::footings_under(doc.model(), column);
        assert_eq!(under.len(), 1);
        assert!(matches!(
            doc.model().get(under[0]).map(|e| &e.kind),
            Some(ElementKind::Footing(f)) if (f.width - footing::PAD_SIZE).abs() < 1e-9
        ));
        go(&mut doc, &mut st, 80.0, 80.0);
        assert_eq!(free(&doc, FootingKind::Pad), 1);
        doc.undo();
        assert_eq!(free(&doc, FootingKind::Pad), 0);
    }

    #[test]
    fn the_roof_tool_draws_rectangles_and_outlines() {
        let (mut doc, level, mut st, mut sel) = setup();
        st.tool = Tool::Roof;
        st.members.roof_shape = RoofShape::Gable;
        st.slab_mode = crate::slabtool::SlabMode::Rectangle;
        crate::slabtool::click(&mut doc, level, DVec2::ZERO, &mut st, &mut sel).unwrap();
        crate::slabtool::click(&mut doc, level, DVec2::new(8.0, 5.0), &mut st, &mut sel).unwrap();
        assert!(matches!(
            active(&doc, &sel),
            Some(ElementKind::Roof(r)) if r.shape == RoofShape::Gable
        ));
        st.slab_mode = crate::slabtool::SlabMode::Polygon;
        crate::slabtool::click(&mut doc, level, DVec2::new(20.0, 0.0), &mut st, &mut sel).unwrap();
        let mut o = st.outline.take().unwrap();
        assert!(o.roof.is_some());
        for (x, y) in [(24.0, 0.0), (24.0, 4.0), (20.0, 4.0)] {
            o.click(DVec2::new(x, y), false, false);
        }
        o.click(DVec2::new(20.0, 0.0), true, false);
        let (_, id) = o.finish(&mut doc, true).unwrap();
        assert!(matches!(
            doc.model().get(id.unwrap()).map(|e| &e.kind),
            Some(ElementKind::Roof(_))
        ));
    }
}
