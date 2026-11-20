//! Alt+click picks up an element's parameters (George 2026-11-20, as
//! ArchiCAD's): the tool that made the element is taken up, set as the
//! element is (its type, sizes, pens, styles in the tool bar), and every
//! new element it draws takes all of the element's parameters, keeping
//! only where it is drawn (see [`creabim_core::pickup`]). Changing a
//! setting in the tool bar afterwards, or another tool, lets the template
//! go: the bar's settings rule again.

use crate::planview::{PlanState, Tool};
use creabim_core::detail::DetailKind;
use creabim_core::{Document, Element, ElementId, ElementKind, Model, TypeCategory};

/// The element picked up, the tool it set, and the tool's settings then.
#[derive(Clone, Debug)]
pub struct Picked {
    pub tool: Tool,
    pub template: Element,
    key: String,
}

/// The tool that makes elements like `e`.
pub fn tool_for(e: &Element) -> Option<Tool> {
    Some(match &e.kind {
        ElementKind::Wall(_) => Tool::Wall,
        ElementKind::Slab(_) => Tool::Slab,
        ElementKind::Opening(o) => match o.kind {
            creabim_core::OpeningKind::Door => Tool::Door,
            creabim_core::OpeningKind::Window => Tool::Window,
            creabim_core::OpeningKind::Hole => Tool::Opening,
        },
        ElementKind::Column(_) => Tool::Column,
        ElementKind::Beam(_) => Tool::Beam,
        ElementKind::Roof(_) => Tool::Roof,
        ElementKind::Stair(_) => Tool::Stair,
        ElementKind::Railing(_) => Tool::Railing,
        ElementKind::Footing(_) => Tool::Footing,
        ElementKind::Shaft(_) => Tool::Shaft,
        ElementKind::Object(_) => Tool::Object,
        ElementKind::FreeMesh(_) => Tool::Mass,
        ElementKind::Part(p) => match (&p.form, p.void) {
            (_, true) => Tool::Void,
            (creabim_core::parts::Form::Extrusion { .. }, _) => Tool::Extrusion,
            (creabim_core::parts::Form::Sweep { .. }, _) => Tool::Sweep,
        },
        ElementKind::Site(s) => match &s.form {
            creabim_core::site::SiteForm::Terrain { .. }
            | creabim_core::site::SiteForm::Earthwork { .. } => Tool::Terrain,
            creabim_core::site::SiteForm::Area { .. } => Tool::Paving,
            creabim_core::site::SiteForm::Plant { .. } => Tool::Planting,
        },
        ElementKind::Room(_) => Tool::Room,
        ElementKind::RoomSeparator(_) => Tool::RoomSeparator,
        ElementKind::Ceiling(_) => Tool::Ceiling,
        ElementKind::Detail(d) => match &d.kind {
            DetailKind::Line(_) if d.points.len() > 2 => Tool::Polyline,
            DetailKind::Line(_) => Tool::Line,
            DetailKind::Arc(_) => Tool::Arc,
            DetailKind::Curve(_) => Tool::Curve,
            _ => Tool::Detail,
        },
        ElementKind::Pipe(_) => Tool::Pipe,
        ElementKind::Dimension(d) => {
            use creabim_core::annotation::DimensionKind as K;
            match d.kind {
                K::Linear => Tool::Dimension,
                K::Angular => Tool::Angle,
                K::Radius => Tool::Radius,
                K::Diameter => Tool::Diameter,
                K::ArcLength => Tool::ArcLength,
                K::Spot => Tool::SpotLevel,
                K::Slope => Tool::Slope,
            }
        }
        ElementKind::Tag(_) => Tool::Tag,
        ElementKind::Note(_) => Tool::Note,
        ElementKind::Sketch(_) => Tool::Sketch,
        ElementKind::Grid(_) => Tool::Grid,
        ElementKind::Guide(_) => Tool::Guide,
        ElementKind::Level(_) => Tool::Level,
        ElementKind::View(v) => match v.kind {
            creabim_core::ViewKind::Section { cut: true, .. } => Tool::Section,
            creabim_core::ViewKind::Section { cut: false, .. } => Tool::Elevation,
            _ => return None,
        },
        _ => return None,
    })
}

/// What the tool bar is set to, as far as new elements go: a change to
/// it after picking up drops the template.
pub fn settings_key(s: &PlanState) -> String {
    let mut types: Vec<_> = s
        .types
        .iter()
        .map(|(c, id)| format!("{c:?}={id:?}"))
        .collect();
    types.sort();
    let m = &s.members;
    let p = &s.parts;
    let o = &s.objects;
    let d = &s.details;
    let a = &s.anno;
    let walls = (s.wall_thickness, s.wall_location, s.wall_flip);
    let members = (
        (
            m.column,
            m.beam_width,
            m.beam_depth,
            m.roof_shape,
            m.roof_pitch,
        ),
        (
            m.stair_width,
            m.stair_turn,
            m.railing_height,
            &m.railing_section,
        ),
        (
            m.footing_pad,
            m.footing_width,
            m.footing_thickness,
            m.footing_align,
        ),
        (m.footing_offset, m.footing_base, &m.footing_material),
        (m.door, m.window, m.hole, &m.column_section, &m.beam_section),
        (
            &m.pipe.system,
            &m.pipe.pipe_type,
            m.pipe.nominal_mm,
            m.pipe.slope_pct,
        ),
        (m.beam_corner, m.beam_on_face),
    );
    let parts = (
        (
            p.extrusion_base,
            p.extrusion_height,
            p.void_base,
            p.void_height,
        ),
        (
            p.terrain_thickness,
            p.terrain_kind,
            p.earth_bottom,
            p.earth_slope,
        ),
        (p.area, p.plant, p.canopy, p.plant_height, p.steps),
        (&p.library, &p.extrusion_library),
    );
    let objects = (
        o.current.as_ref().map(|c| &c.key),
        o.project_type,
        o.rotation,
        o.offset,
        o.free,
    );
    let details = (&d.kind, d.line, d.width, d.callout_scale);
    let anno = (
        s.dim_short,
        a.dim_style,
        a.note_style,
        a.tag_style,
        a.tag_material,
        a.slope_fmt,
        a.slope_value,
    );
    let other = (
        crate::sketch::network(),
        crate::sketch::mark(),
        s.grid_group,
    );
    format!("{types:?}{walls:?}{members:?}{parts:?}{objects:?}{details:?}{anno:?}{other:?}")
}

/// Take up the tool that makes `id`, set as it is. The status line to
/// show, or why nothing was picked up.
pub fn pick_up(state: &mut PlanState, model: &Model, id: ElementId) -> Result<String, String> {
    let e = model.get(id).ok_or("Nothing to pick up there.")?;
    let tool = tool_for(e).ok_or_else(|| {
        format!(
            "{}: no tool draws it, so there is nothing to pick up.",
            crate::hoverpick::describe(model, id)
        )
    })?;
    state.cancel();
    state.tool = tool;
    // the type it is of, or none
    if let Some(cat) = TypeCategory::of(&e.kind) {
        match e.of_type.as_ref().map(|t| t.id) {
            Some(t) if model.get(t).is_some() => {
                state.types.insert(cat, t);
            }
            _ => {
                state.types.remove(&cat);
            }
        }
    }
    let name = |m: Option<ElementId>| m.and_then(|m| model.get(m)).map(|e| e.name.clone());
    let s = state;
    match &e.kind {
        ElementKind::Wall(w) => {
            s.wall_thickness = w.thickness;
            s.wall_location = w.location;
            s.wall_flip = w.flip;
            s.wall_shape = 0;
        }
        ElementKind::Opening(o) => {
            let size = [o.width, o.height, o.sill];
            match o.kind {
                creabim_core::OpeningKind::Door => s.members.door = size,
                creabim_core::OpeningKind::Window => s.members.window = size,
                creabim_core::OpeningKind::Hole => s.members.hole = size,
            }
        }
        ElementKind::Column(c) => {
            s.members.column = c.profile;
            s.members.column_section = c.section.as_ref().map(|x| x.name.clone());
        }
        ElementKind::Beam(b) => {
            s.members.beam_width = b.width;
            s.members.beam_depth = b.depth;
            s.members.beam_section = b.section.as_ref().map(|x| x.name.clone());
            s.members.last_beam = None;
        }
        ElementKind::Roof(r) => {
            s.members.roof_shape = r.shape;
            s.members.roof_pitch = r.pitch;
        }
        ElementKind::Stair(st) => {
            s.members.stair_width = st.width;
            s.members.stair_turn = st.landing.is_some();
        }
        ElementKind::Railing(r) => {
            s.members.railing_height = r.height;
            s.members.railing_section = r.rail_profile.clone();
        }
        ElementKind::Footing(f) => {
            let m = &mut s.members;
            m.footing_pad = f.kind == creabim_core::FootingKind::Pad;
            m.footing_width = f.width;
            m.footing_thickness = f.thickness;
            m.footing_align = f.align;
            m.footing_offset = f.align_offset;
            m.footing_base = f.base_offset.unwrap_or(0.0);
            if let Some(n) = name(f.material) {
                m.footing_material = Some(n);
            }
        }
        ElementKind::Object(o) => {
            let own = e.of_type.as_ref().map(|t| t.id);
            if let Some((t, ty)) =
                own.and_then(|t| Some((t, creabim_core::objects::type_of_id(model, t)?)))
            {
                let tname = model.get(t).map_or(e.name.clone(), |x| x.name.clone());
                s.objects.current = Some(creabim_core::objects::CatalogueObject {
                    key: ty
                        .catalogue
                        .clone()
                        .unwrap_or_else(|| format!("project/{t:?}")),
                    name: tname,
                    description: String::new(),
                    object: ty.clone(),
                    materials: vec![],
                });
                s.objects.project_type = Some(t);
            }
            s.objects.offset = Some(o.offset);
            s.objects.rotation = o.rotation.to_degrees().rem_euclid(360.0);
        }
        ElementKind::Part(p) => {
            use creabim_core::parts::Form;
            match &p.form {
                Form::Extrusion { base, height, .. } if p.void => {
                    s.parts.void_base = *base;
                    s.parts.void_height = *height;
                }
                Form::Extrusion { base, height, .. } => {
                    s.parts.extrusion_base = *base;
                    s.parts.extrusion_height = *height;
                    s.parts.extrusion_library = p.profile.clone();
                }
                Form::Sweep { .. } => s.parts.library = p.profile.clone(),
            }
        }
        ElementKind::Site(site) => {
            use creabim_core::site::SiteForm;
            match &site.form {
                SiteForm::Terrain { thickness, .. } => {
                    s.parts.terrain_kind = 0;
                    s.parts.terrain_thickness = *thickness;
                }
                SiteForm::Earthwork {
                    bottom, slope, pit, ..
                } => {
                    s.parts.terrain_kind = if *pit { 1 } else { 2 };
                    s.parts.earth_bottom = *bottom;
                    s.parts.earth_slope = *slope;
                }
                SiteForm::Area { .. } => {}
                SiteForm::Plant {
                    form,
                    canopy,
                    height,
                    ..
                } => {
                    s.parts.plant = *form;
                    s.parts.canopy = *canopy;
                    s.parts.plant_height = *height;
                }
            }
        }
        ElementKind::Detail(d) => match &d.kind {
            DetailKind::Line(l) | DetailKind::Arc(l) | DetailKind::Curve(l) => s.details.line = *l,
            k => {
                s.details.kind = k.clone();
                s.details.width = d.width;
            }
        },
        ElementKind::Pipe(p) => {
            let draft = &mut s.members.pipe;
            draft.system = p.system.clone();
            draft.pipe_type = p.pipe_type.clone();
            draft.nominal_mm = p.nominal_mm;
            draft.height = p.start.z;
            draft.last_z = None;
            draft.height_set = false;
        }
        ElementKind::Dimension(d) => {
            s.dim_short = d.short;
            s.anno.dim_style = d.style;
        }
        ElementKind::Tag(t) => {
            s.anno.tag_style = t.style;
            s.anno.tag_material = t.material;
        }
        ElementKind::Note(n) => s.anno.note_style = n.style,
        ElementKind::Sketch(k) => match (&k.net, k.mark) {
            (Some(n), _) => crate::sketch::set_network(Some(n.kind)),
            (None, Some(m)) => crate::sketch::set_mark(Some(m)),
            (None, None) => {
                crate::sketch::set_network(None);
                crate::sketch::set_mark(None);
            }
        },
        ElementKind::Grid(g) => s.grid_group = g.group,
        _ => {}
    }
    let what = crate::hoverpick::describe(model, id);
    s.picked = creabim_core::pickup::can_template(&e.kind).then(|| Picked {
        tool,
        template: e.clone(),
        key: settings_key(s),
    });
    s.picked_now = true;
    Ok(format!(
        "Picked up from {what}: the {} tool draws the next one the same.",
        tool.label()
    ))
}

/// Keep the document's template in step with the tool: it goes when
/// another tool is taken or the bar is set otherwise.
pub fn sync(doc: &mut Document, state: &mut PlanState) {
    let keep = state
        .picked
        .as_ref()
        .is_some_and(|p| p.tool == state.tool && p.key == settings_key(state));
    if !keep {
        state.picked = None;
    }
    let want = state.picked.as_ref().map(|p| &p.template);
    if doc.pickup().map(|e| e.id) != want.map(|e| e.id) {
        doc.set_pickup(want.cloned());
    }
}

/// Of what is under the pointer, the one Alt+click picks up: the
/// selected one if it is there, else the first.
pub fn choose(under: &[ElementId], selected: &[ElementId]) -> Option<ElementId> {
    under
        .iter()
        .find(|id| selected.contains(id))
        .or(under.first())
        .copied()
}
