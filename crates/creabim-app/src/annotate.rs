//! Angle, radius, diameter and arc-length dimensions and text notes in the
//! 2D panes and on sheets: drawing them on screen, the tools that place
//! them, dragging them, and their properties and text styles.

use crate::planview::{Tool, Xf};
use crate::selection::Selection;
use crate::snaps::SnapKind;
use creabim_core::{
    Dimension, DimensionKind, Document, Element, ElementId, ElementKind, Font, HostPoint, Model,
    ModelError, Note, StyleUse, TagShape, Terminator, TextStyle, Transaction, View,
};
use creabim_geom::annotate::attach;
use creabim_geom::annotation::{
    angle_text, circle_through, line_intersection, measure, sweep, Annotation, Ends, Shape,
    TextLook,
};
use creabim_geom::paper::Anchor;
use creabim_geom::PlanItem;
use eframe::egui::{self, Color32, Pos2, Stroke, Vec2};
use glam::DVec2;

/// Annotations are drawn in this blue on screen unless they have a colour.
pub const INK: Color32 = Color32::from_rgb(30, 60, 120);
fn interaction_ink() -> Color32 {
    crate::theme::interaction()
}

pub fn color32([r, g, b]: [f32; 3]) -> Color32 {
    let c = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgb(c(r), c(g), c(b))
}

/// The screen colour of an annotation.
pub fn ink(look: &TextLook, selected: bool, highlight: Color32) -> Color32 {
    match (selected, look.rgb) {
        (true, _) => highlight,
        (false, Some(rgb)) => color32(rgb),
        (false, None) => INK,
    }
}

/// Text height on screen, in pixels, for a look in a pane at `mm` drawing
/// units per paper mm.
pub fn text_px(look: &TextLook, mm: f64, xf: &Xf) -> f32 {
    let px = (look.size * mm * xf.scale as f64) as f32;
    if (mm - 1.0).abs() < 1e-9 {
        // on a sheet's paper (mm = 1): its true size, shrinking with the
        // sheet and left out once too small, never spilling off (MU23)
        px.max(1.0)
    } else {
        // in a view while working: up to 9 px so a detail's tags read
        // (A5-7), but never more than twice their true size, so a view
        // zoomed far out does not fill with oversized words (MU28)
        px.max(9.0_f32.min(px * 2.0))
    }
}

/// Text on screen in a look's family, weight and slant, turned `angle`
/// radians counter-clockwise about its anchor.
#[allow(clippy::too_many_arguments)]
pub fn text(
    painter: &egui::Painter,
    at: Pos2,
    text: &str,
    px: f32,
    color: Color32,
    angle: f64,
    anchor: Anchor,
    look: &TextLook,
) {
    if px < 3.0 {
        return;
    }
    let family = match look.font {
        Font::Mono => egui::FontFamily::Monospace,
        Font::Sans => crate::drawfont::family(),
        Font::Serif => egui::FontFamily::Proportional,
    };
    // `px` is the capitals' height: the face is set at its em
    let em = creabim_geom::font::em(px as f64, look.font) as f32;
    let mut job = egui::text::LayoutJob::simple_singleline(
        text.to_string(),
        egui::FontId::new(em, family),
        color,
    );
    for s in &mut job.sections {
        s.format.italics = look.italic;
    }
    let galley = painter.layout_job(job);
    let stretch = creabim_geom::font::setup().stretch as f32;
    let size = galley.size();
    let w = size.x * stretch;
    let shift = match anchor {
        Anchor::Start => w / 2.0,
        Anchor::Middle => 0.0,
        Anchor::End => -w / 2.0,
    };
    // the screen's y runs down, so a counter-clockwise turn is a negative angle
    let rot = egui::emath::Rot2::from_angle(-angle as f32);
    let centre = at + rot * Vec2::new(shift, 0.0);
    // no bold face on screen: the text again, a hair to the right
    let nudge = rot * Vec2::new((px * 0.05).max(0.6), 0.0);
    if (stretch - 1.0).abs() > 1e-4 {
        // letters narrower or wider than the face's: its triangles, set
        // about the text's middle and squeezed along it
        let ctx = painter.ctx();
        let mut tess = egui::epaint::Tessellator::new(
            ctx.pixels_per_point(),
            egui::epaint::TessellationOptions {
                round_text_to_pixels: false,
                coarse_tessellation_culling: false,
                ..Default::default()
            },
            ctx.fonts(|f| f.font_image_size()),
            vec![],
        );
        let mut mesh = egui::Mesh::default();
        tess.tessellate_text(
            &egui::epaint::TextShape::new(Pos2::ZERO, galley, color),
            &mut mesh,
        );
        for v in &mut mesh.vertices {
            let p = Vec2::new((v.pos.x - size.x / 2.0) * stretch, v.pos.y - size.y / 2.0);
            v.pos = centre + rot * p;
        }
        if look.bold {
            let mut again = mesh.clone();
            again.translate(nudge);
            painter.add(egui::Shape::mesh(again));
        }
        painter.add(egui::Shape::mesh(mesh));
        return;
    }
    let pos = centre - rot * (size / 2.0);
    if look.bold {
        painter.add(
            egui::epaint::TextShape::new(pos + nudge, galley.clone(), color)
                .with_angle(-angle as f32),
        );
    }
    painter.add(egui::epaint::TextShape::new(pos, galley, color).with_angle(-angle as f32));
}

/// Draw an annotation in a pane.
pub fn paint(painter: &egui::Painter, xf: &Xf, item: &Annotation, color: Color32, mm: f64) {
    paint_weighted(painter, xf, item, color, mm, 1.0);
}

/// Draw an annotation with its strokes `w` times their weight: a hover or
/// selection traces the annotation itself, heavier, in its colour.
pub fn paint_weighted(
    painter: &egui::Painter,
    xf: &Xf,
    item: &Annotation,
    color: Color32,
    mm: f64,
    w: f32,
) {
    let m = item.marks(mm);
    for l in &m.lines {
        let pts: Vec<Pos2> = l.iter().map(|p| xf.to_screen(*p)).collect();
        painter.add(egui::Shape::line(pts, Stroke::new(w, color)));
    }
    for a in &m.arrows {
        let pts: Vec<Pos2> = a.iter().map(|p| xf.to_screen(*p)).collect();
        painter.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
    }
    paint_ends_weighted(painter, xf, &m.ends, color, w);
    for patch in item.back_patches(&m, mm) {
        let pts: Vec<Pos2> = patch.iter().map(|p| xf.to_screen(*p)).collect();
        painter.add(egui::Shape::convex_polygon(
            pts,
            Color32::WHITE,
            Stroke::NONE,
        ));
    }
    let px = text_px(&item.look, mm, xf);
    for t in &m.texts {
        text(
            painter,
            xf.to_screen(t.at),
            &t.text,
            px,
            color,
            t.angle,
            t.anchor,
            &item.look,
        );
    }
}

/// Draw dimension and leader ends in a pane: thin and heavy strokes and
/// filled shapes.
pub fn paint_ends(painter: &egui::Painter, xf: &Xf, ends: &Ends, color: Color32) {
    paint_ends_weighted(painter, xf, ends, color, 1.0);
}

/// Dimension and leader ends with their strokes `w` times their weight.
pub fn paint_ends_weighted(painter: &egui::Painter, xf: &Xf, ends: &Ends, color: Color32, w: f32) {
    let screen = |l: &Vec<DVec2>| -> Vec<Pos2> { l.iter().map(|p| xf.to_screen(*p)).collect() };
    for l in &ends.thin {
        painter.add(egui::Shape::line(screen(l), Stroke::new(w, color)));
    }
    for l in &ends.heavy {
        painter.add(egui::Shape::line(screen(l), Stroke::new(2.0 * w, color)));
    }
    for f in &ends.fills {
        painter.add(egui::Shape::convex_polygon(screen(f), color, Stroke::NONE));
    }
}

// ---- tools ------------------------------------------------------------------

/// The letter heights the Text tool offers, paper mm; 0 is the style's.
pub const TEXT_SIZES: [f64; 6] = [0.0, 2.5, 3.5, 5.0, 7.0, 10.0];

/// Is this one of the tools here?
pub fn is_tool(t: Tool) -> bool {
    matches!(
        t,
        Tool::Angle
            | Tool::Slope
            | Tool::Radius
            | Tool::Diameter
            | Tool::ArcLength
            | Tool::Note
            | Tool::Text
            | Tool::SpotLevel
    )
}

type Picked = (HostPoint, DVec2);

/// What a tool has picked, waiting for the click that places it.
#[derive(Clone, Copy, Debug)]
enum Ready {
    Angle {
        vertex: Picked,
        a: Picked,
        b: Picked,
    },
    Circle {
        center: Picked,
        r: f64,
        host: Option<ElementId>,
        diameter: bool,
    },
    Arc {
        center: Picked,
        a: Picked,
        b: Picked,
    },
    Leader {
        target: Picked,
    },
    /// The sloping line a slope dimension writes the slope of; the
    /// format (an index into `SLOPE_FORMATS`) and, in a plan, the slope
    /// typed.
    Slope {
        a: Picked,
        b: Picked,
        plan: bool,
        fmt: usize,
        value: f64,
    },
    /// The point a spot level measures.
    Spot {
        point: Picked,
        plan: bool,
        /// S-JTSK coordinates of the point, not its height
        coords: bool,
        /// its height above sea level (Bpv), not from ±0,000
        bpv: bool,
        /// the height it will read, for the preview: from ±0,000, or in
        /// Bpv
        z: Option<f64>,
    },
}

/// What a drag in the Select tool holds.
#[derive(Clone, Copy, Debug)]
pub enum Grab {
    /// A note's text: where the drag began and the note's position then.
    Note(ElementId, DVec2, DVec2),
    /// A note's leader arrow.
    Leader(ElementId),
    /// A dimension's line or arc.
    Shape(ElementId),
    /// A note's leader elbow.
    Elbow(ElementId),
    /// A tag's leader end, and its host's centre in the drawing.
    TagEnd(ElementId, DVec2),
    /// A tag's leader elbow, and its host's centre in the drawing.
    TagElbow(ElementId, DVec2),
    /// A level's head in a section: the level, and its datum line's end.
    LevelHead(ElementId, DVec2),
}

/// Where the elbow grip of a leader running `from` → `to` sits: its
/// elbow, or halfway along when it has none.
fn elbow_grip(elbow: Option<DVec2>, from: DVec2, to: DVec2) -> DVec2 {
    elbow.unwrap_or((from + to) / 2.0)
}

/// The grips of a selected item: (where, what dragging it does).
fn grips(model: &Model, it: &PlanItem, mm: f64) -> Vec<(DVec2, Grab)> {
    match it {
        PlanItem::Annotation { id, item } => match &item.shape {
            Shape::Note {
                leader: Some(tip),
                elbow,
                ..
            } => {
                let m = item.marks(mm);
                let from = m
                    .lines
                    .last()
                    .and_then(|l| l.first().copied())
                    .unwrap_or(*tip);
                vec![
                    (*tip, Grab::Leader(*id)),
                    (elbow_grip(*elbow, from, *tip), Grab::Elbow(*id)),
                ]
            }
            // a level's head in a section or elevation, for this view only
            Shape::Level { at, line_end, .. } => {
                vec![(*at, Grab::LevelHead(*id, line_end.unwrap_or(*at)))]
            }
            // a spot level's mark: in a section it slides along its height,
            // still bound to what it stands on
            Shape::Spot {
                at, text_at, plan, ..
            } => {
                let g = if *plan {
                    *text_at
                } else {
                    DVec2::new(text_at.x, at.y)
                };
                vec![(g, Grab::Shape(*id))]
            }
            _ => vec![],
        },
        PlanItem::Tag {
            id,
            anchor,
            at,
            elbow,
            ..
        } => {
            let Some(ElementKind::Tag(t)) = model.get(*id).map(|e| &e.kind) else {
                return vec![];
            };
            let centre = *anchor - t.end.unwrap_or(DVec2::ZERO);
            vec![
                (*anchor, Grab::TagEnd(*id, centre)),
                (
                    elbow_grip(*elbow, *at, *anchor),
                    Grab::TagElbow(*id, centre),
                ),
            ]
        }
        _ => vec![],
    }
}

/// The tools' clicks so far, and their options.
#[derive(Debug)]
pub struct Draft {
    points: Vec<Picked>,
    /// The first line picked for an angle: its element, ends, and the click.
    line: Option<(ElementId, DVec2, DVec2, DVec2)>,
    ready: Option<Ready>,
    /// The Note tool draws a leader first.
    pub leader: bool,
    /// A note was just placed: Properties should put the cursor in its text.
    pub focus_text: bool,
    /// The letter height new free texts get, paper mm; 0 is their
    /// style's.
    pub text_size: f64,
    pub grab: Option<Grab>,
    /// The area whose drag took `grab`: another area, not dragged, must
    /// not let it go (MU31: a sheet note moved one frame, then stopped).
    pub grab_by: Option<egui::Id>,
    /// The styles new dimensions, notes and tags get (none: the default).
    pub dim_style: Option<ElementId>,
    pub note_style: Option<ElementId>,
    pub tag_style: Option<ElementId>,
    /// New tags read their host's material build-up (Annotate step 5).
    pub tag_material: bool,
    /// The spot level tool writes the point's S-JTSK coordinates instead,
    /// as PilonDesigner's cadastre mark.
    pub spot_coords: bool,
    /// The spot level tool writes heights above sea level (Bpv), from the
    /// project's ±0,000 in Bpv, instead of from ±0,000.
    pub spot_bpv: bool,
    /// How new slope dimensions read: an index into
    /// [`creabim_core::annotation::SLOPE_FORMATS`].
    pub slope_fmt: usize,
    /// The slope a new slope dimension in a plan writes, in that format.
    pub slope_value: f64,
}

impl Default for Draft {
    fn default() -> Self {
        Self {
            points: vec![],
            line: None,
            ready: None,
            leader: true,
            focus_text: false,
            text_size: 0.0,
            grab: None,
            grab_by: None,
            dim_style: None,
            note_style: None,
            tag_style: None,
            tag_material: false,
            spot_coords: false,
            spot_bpv: false,
            slope_fmt: 0,
            slope_value: 2.0,
        }
    }
}

impl Draft {
    pub fn clear(&mut self) {
        self.points.clear();
        self.line = None;
        self.ready = None;
    }

    /// The style a new element of this kind gets, if one is picked.
    pub fn style_for(&self, kind: &ElementKind) -> Option<ElementId> {
        match kind {
            ElementKind::Dimension(_) => self.dim_style,
            ElementKind::Note(_) => self.note_style,
            ElementKind::Tag(_) => self.tag_style,
            _ => None,
        }
    }

    /// Give a new dimension, note or tag the style picked for its kind.
    pub fn styled(&self, mut e: Element) -> Element {
        if let Some(id) = self.style_for(&e.kind) {
            set_style(&mut e.kind, id);
        }
        e
    }

    /// A free text at `at` in `view`: no leader, the size picked.
    pub fn free_text(&self, view: ElementId, at: DVec2) -> Note {
        let mut n = Note::new(view, at, creabim_core::words::tr("Text"));
        n.size = (self.text_size > 0.0).then_some(self.text_size);
        n
    }

    pub fn has_input(&self) -> bool {
        !self.points.is_empty() || self.line.is_some() || self.ready.is_some()
    }
}

/// The straight edge of the drawing nearest `p`, within `tol`.
pub fn segment_at(items: &[PlanItem], p: DVec2, tol: f64) -> Option<(ElementId, DVec2, DVec2)> {
    let mut best: Option<(f64, ElementId, DVec2, DVec2)> = None;
    let mut consider = |id: ElementId, a: DVec2, b: DVec2| {
        if a.distance(b) < 1e-9 {
            return;
        }
        let d = segment_distance(p, a, b);
        if d < tol && best.is_none_or(|(bd, ..)| d < bd) {
            best = Some((d, id, a, b));
        }
    };
    for it in items {
        match it {
            PlanItem::Line { id, a, b, .. } => consider(*id, *a, *b),
            PlanItem::Polygon { id, points, .. } => {
                for i in 0..points.len() {
                    consider(*id, points[i], points[(i + 1) % points.len()]);
                }
            }
            _ => {}
        }
    }
    best.map(|(_, id, a, b)| (id, a, b))
}

/// The arc or circle of the drawing nearest `p`, within `tol`: its element,
/// centre, radius and start and end angles.
fn arc_at(items: &[PlanItem], p: DVec2, tol: f64) -> Option<(ElementId, DVec2, f64, f64, f64)> {
    let mut best: Option<(f64, ElementId, DVec2, f64, f64, f64)> = None;
    for it in items {
        let PlanItem::Arc {
            id,
            center,
            radius,
            start,
            end,
            ..
        } = it
        else {
            continue;
        };
        let d = (p.distance(*center) - radius).abs();
        let span = (end - start).abs();
        let t = ((p - *center).to_angle() - start.min(*end)).rem_euclid(std::f64::consts::TAU);
        let on = span >= std::f64::consts::TAU - 1e-6 || t <= span + 1e-6;
        if on && d < tol && best.is_none_or(|b| d < b.0) {
            best = Some((d, *id, *center, *radius, *start, *end));
        }
    }
    best.map(|(_, id, c, r, s, e)| (id, c, r, s.min(e), s.max(e)))
}

fn segment_distance(p: DVec2, a: DVec2, b: DVec2) -> f64 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-18)).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// The point on the endless line through `a`, `b` nearest `p`.
fn project(p: DVec2, a: DVec2, b: DVec2) -> DVec2 {
    let d = (b - a).normalize_or_zero();
    a + d * (p - a).dot(d)
}

/// A point on the arm from `vertex` through the line `a`,`b` on the side of
/// `click`, at least a little way out.
fn arm(vertex: DVec2, a: DVec2, b: DVec2, click: DVec2) -> DVec2 {
    let q = project(click, a, b);
    if q.distance(vertex) > 1e-3 {
        return q;
    }
    // a click right on the corner: along the line to its further end
    if a.distance(vertex) > b.distance(vertex) {
        a
    } else {
        b
    }
}

/// A click with an annotation tool in a plan or section. `hit` is the
/// snapped point under the pointer and what it attaches to; `raw` the
/// pointer itself. Returns a hint for the status bar.
#[allow(clippy::too_many_arguments)]
pub fn click(
    doc: &mut Document,
    view_id: ElementId,
    view: &View,
    tool: Tool,
    hit: Option<(HostPoint, DVec2, Option<SnapKind>)>,
    raw: DVec2,
    items: &[PlanItem],
    tol: f64,
    d: &mut Draft,
    selection: &mut Selection,
) -> Result<String, ModelError> {
    let model = doc.model();
    let (hp, p, snapped) = match hit {
        Some((hp, p, k)) => (hp, p, k.is_some_and(|k| k.is_point())),
        None => (HostPoint::free(raw), raw, false),
    };
    if let Some(ready) = d.ready.take() {
        d.clear();
        // a slope's own pattern is set below, not the last spot level's
        let mut e = d.styled(place(model, view_id, view, ready, tool, raw));
        if let ElementKind::Dimension(dim) = &mut e.kind {
            if dim.kind == DimensionKind::Spot {
                // a new spot level reads like the last one of its sort
                // made: heights like heights, S-JTSK coordinates like
                // coordinates (a coordinate mark never loses its Y and X)
                let coords = |p: &Option<String>| match p.as_deref() {
                    Some(p) if creabim_core::annotation::writes_coordinates(p) => 2,
                    Some(p) if p.contains("{Bpv}") => 1,
                    _ => 0,
                };
                let own = coords(&dim.pattern);
                let last = model
                    .iter()
                    .filter_map(|e| match &e.kind {
                        ElementKind::Dimension(d)
                            if d.kind == DimensionKind::Spot && coords(&d.pattern) == own =>
                        {
                            Some(d.pattern.clone())
                        }
                        _ => None,
                    })
                    .last();
                if let Some(p) = last {
                    if p.is_some() || own == 0 {
                        dim.pattern = p;
                    }
                }
                // and its line long or short as the last spot level's
                // (George 2026-11-23)
                dim.short = model
                    .iter()
                    .filter_map(|e| match &e.kind {
                        ElementKind::Dimension(d) if d.kind == DimensionKind::Spot => Some(d.short),
                        _ => None,
                    })
                    .last()
                    .unwrap_or(false);
            }
        }
        if let (
            ElementKind::Dimension(dim),
            Ready::Slope {
                plan, fmt, value, ..
            },
        ) = (&mut e.kind, ready)
        {
            dim.pattern = slope_pattern(plan, fmt, value);
        }
        let id = e.id;
        let label = match &e.kind {
            ElementKind::Note(_) => "Add note",
            _ => "Add dimension",
        };
        doc.commit(Transaction::new(label).insert(e))?;
        selection.only(id);
        return Ok(added(tool, d));
    }
    match tool {
        Tool::Text => {
            let e = d.styled(Element::new(
                "Text",
                ElementKind::Note(d.free_text(view_id, raw)),
            ));
            selection.only(e.id);
            doc.commit(Transaction::new("Add text").insert(e))?;
            Ok(added(tool, d))
        }
        Tool::Note if !d.leader => {
            let e = d.styled(Element::new(
                "Note",
                ElementKind::Note(Note::new(view_id, raw, "Note")),
            ));
            selection.only(e.id);
            doc.commit(Transaction::new("Add note").insert(e))?;
            Ok(added(tool, d))
        }
        Tool::SpotLevel => {
            let plan = matches!(view.kind, creabim_core::ViewKind::Plan { .. });
            let coords = plan && d.spot_coords;
            let point = match view.kind {
                // coordinates: the point snapped to (a plot's corner, a
                // cadastre line's bend, the building's corner)
                creabim_core::ViewKind::Plan { .. } if coords => (hp, p),
                // in a plan, kept on what is under the point snapped to
                // (a roof's face or edge, a slab, the terrain, a stair),
                // whose height there it reads: the mark sits right where
                // the pointer stuck, not where the pointer itself is
                creabim_core::ViewKind::Plan { level, .. } => {
                    (spot_point(model, view, level, p), p)
                }
                _ => (hp, p),
            };
            d.ready = Some(Ready::Spot {
                point,
                plan,
                coords,
                bpv: !coords && d.spot_bpv,
                z: (!coords)
                    .then(|| {
                        let probe = Dimension::spot(view_id, point.0, point.0, None);
                        creabim_geom::annotate::spot_height(model, view, &probe, point.1)
                    })
                    .flatten()
                    .map(|z| {
                        if d.spot_bpv {
                            model.project_info().map_or(0.0, |(_, i)| i.elevation) + z
                        } else {
                            z
                        }
                    }),
            });
            Ok("Now click where its text goes.".into())
        }
        Tool::Slope => {
            let plan = matches!(view.kind, creabim_core::ViewKind::Plan { .. });
            let (fmt, value) = (d.slope_fmt, d.slope_value);
            let ready = |a: Picked, b: Picked| Ready::Slope {
                a,
                b,
                plan,
                fmt,
                value,
            };
            // in a section or elevation, a click on a sloping line takes it
            if d.points.is_empty() && !snapped && !plan {
                if let Some((id, a, b)) = segment_at(items, raw, tol) {
                    let (a, b) = if a.x <= b.x { (a, b) } else { (b, a) };
                    d.ready = Some(ready(
                        (attach(model, view, Some(id), a), a),
                        (attach(model, view, Some(id), b), b),
                    ));
                    return Ok("Now click the side the arrow goes.".into());
                }
            }
            d.points.push((hp, p));
            if let [a, b] = d.points[..] {
                d.points.clear();
                if a.1.distance(b.1) < 1e-6 {
                    return Ok("Those points are the same: click two apart.".into());
                }
                d.ready = Some(ready(a, b));
                return Ok("Now click the side the arrow goes.".into());
            }
            Ok(if plan {
                "Now click the second point: the slope falls from the first towards it.".into()
            } else {
                "Now click a second point on the sloping line.".into()
            })
        }
        Tool::Note => {
            // a click inside a cut (a ceiling's layers in a section) with
            // nothing snapped points at what is cut there, so the note
            // can read its mark (George 2026-11-23)
            let hp = if hp.host.is_none() {
                items
                    .iter()
                    .rev()
                    .find_map(|it| match it {
                        PlanItem::Fill { points, .. } | PlanItem::Polygon { points, .. }
                            if points.len() > 2
                                && model.get(it.id()).is_some()
                                && creabim_core::polygon::contains(points, raw) =>
                        {
                            Some(it.id())
                        }
                        _ => None,
                    })
                    .map_or(hp, |id| attach(model, view, Some(id), p))
            } else {
                hp
            };
            d.ready = Some(Ready::Leader { target: (hp, p) });
            Ok("Now click where the note's text goes.".into())
        }
        Tool::Angle => {
            if d.points.is_empty() && !snapped {
                if let Some((id, a, b)) = segment_at(items, raw, tol) {
                    match d.line.take() {
                        None => {
                            d.line = Some((id, a, b, raw));
                            return Ok("Now click the second line.".into());
                        }
                        Some((id0, a0, b0, c0)) => {
                            let Some(v) = line_intersection(a0, b0, a, b) else {
                                return Ok("Those lines are parallel: pick two that meet.".into());
                            };
                            let (pa, pb) = (arm(v, a0, b0, c0), arm(v, a, b, raw));
                            d.ready = Some(Ready::Angle {
                                vertex: (attach(model, view, Some(id0), v), v),
                                a: (attach(model, view, Some(id0), pa), pa),
                                b: (attach(model, view, Some(id), pb), pb),
                            });
                            return Ok("Now click where the arc goes.".into());
                        }
                    }
                }
            }
            if d.line.is_some() {
                return Ok("Click a second line, or Esc to start again.".into());
            }
            d.points.push((hp, p));
            match d.points.as_slice() {
                [vertex, a, b] => {
                    d.ready = Some(Ready::Angle {
                        vertex: *vertex,
                        a: *a,
                        b: *b,
                    });
                    d.points.clear();
                    Ok("Now click where the arc goes.".into())
                }
                [_] => Ok("Now click a point on the first arm.".into()),
                _ => Ok("Now click a point on the second arm.".into()),
            }
        }
        Tool::Radius | Tool::Diameter | Tool::ArcLength => {
            if d.points.is_empty() {
                if let Some((id, c, r, start, end)) = arc_at(items, raw, tol) {
                    let center = (attach(model, view, Some(id), c), c);
                    d.ready = Some(if tool == Tool::ArcLength {
                        let (a, b) = (
                            c + DVec2::from_angle(start) * r,
                            c + DVec2::from_angle(end) * r,
                        );
                        Ready::Arc {
                            center,
                            a: (attach(model, view, Some(id), a), a),
                            b: (attach(model, view, Some(id), b), b),
                        }
                    } else {
                        Ready::Circle {
                            center,
                            r,
                            host: Some(id),
                            diameter: tool == Tool::Diameter,
                        }
                    });
                    return Ok("Now click where the dimension line goes.".into());
                }
            }
            d.points.push((hp, p));
            if d.points.len() < 3 {
                return Ok(format!(
                    "Point {} of 3 on the {}. Or click an arc or circle in the drawing.",
                    d.points.len(),
                    if tool == Tool::ArcLength {
                        "arc: its start, a point on it, then its end"
                    } else {
                        "circle"
                    }
                ));
            }
            let pts = std::mem::take(&mut d.points);
            let Some((c, r)) = circle_through(pts[0].1, pts[1].1, pts[2].1) else {
                return Ok("Those points are in a line: pick three on a curve.".into());
            };
            // the centre is kept on what the first point is on, so it moves with it
            let host = pts[0].0.host;
            let center = (attach(model, view, host, c), c);
            d.ready = Some(if tool == Tool::ArcLength {
                let (mut a, mut b) = (pts[0], pts[2]);
                if sweep(c, a.1, pts[1].1) > sweep(c, a.1, b.1) {
                    std::mem::swap(&mut a, &mut b);
                }
                Ready::Arc { center, a, b }
            } else {
                Ready::Circle {
                    center,
                    r,
                    host,
                    diameter: tool == Tool::Diameter,
                }
            });
            Ok("Now click where the dimension line goes.".into())
        }
        _ => Ok(String::new()),
    }
}

/// A plan spot level's point at `p`, kept on the element whose surface is
/// there: the first one under the plan's cut (seen from above, under the
/// top of its range), else the topmost slab of `level` there. An element
/// without its own axes (a roof, the terrain, a stair) keeps the point
/// where it is, reading its height there as it changes.
fn spot_point(model: &Model, view: &View, level: ElementId, p: DVec2) -> HostPoint {
    let host = creabim_geom::annotate::spot_from(model, view)
        .and_then(|(below, above)| creabim_geom::annotate::surface_under(model, p, below, above))
        .map(|(id, _)| id)
        .or_else(|| slab_under(model, level, p));
    let Some(host) = host else {
        return HostPoint::free(p);
    };
    let hp = attach(model, view, Some(host), p);
    if hp.host.is_some() {
        hp
    } else {
        HostPoint {
            host: Some(host),
            local: p,
            from_end: false,
            body: None,
            step: None,
        }
    }
}

/// The topmost slab on `level` whose outline holds `p`.
fn slab_under(model: &Model, level: ElementId, p: DVec2) -> Option<ElementId> {
    model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Slab(s)
                if s.level == level && creabim_core::polygon::contains(&s.boundary, p) =>
            {
                Some((e.id, s.top_offset))
            }
            _ => None,
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|s| s.0)
}

fn added(tool: Tool, d: &mut Draft) -> String {
    if tool == Tool::Note {
        d.focus_text = true;
        "Note added. Type its text in Properties; drag it to move it.".into()
    } else if tool == Tool::Text {
        d.focus_text = true;
        "Text added. Type it in Properties, where its size and angle are too; drag it to move it, R turns it.".into()
    } else if tool == Tool::SpotLevel {
        "Spot level added. Its text pattern is in Properties; drag its text to move it.".into()
    } else {
        "Dimension added. Drag it to move its line; its style is in Properties.".into()
    }
}

/// The element a placing click at `at` makes.
fn place(
    model: &Model,
    view_id: ElementId,
    view: &View,
    ready: Ready,
    tool: Tool,
    at: DVec2,
) -> Element {
    match ready {
        Ready::Slope { a, b, .. } => Element::new(
            "Slope",
            ElementKind::Dimension(Dimension::slope(
                view_id,
                a.0,
                b.0,
                slope_offset(a.1, b.1, at),
                None,
            )),
        ),
        Ready::Angle { vertex, a, b } => {
            let (a, b) = if sweep(vertex.1, a.1, at) > sweep(vertex.1, a.1, b.1) {
                (b, a)
            } else {
                (a, b)
            };
            let radius = at.distance(vertex.1);
            Element::new(
                "Angle",
                ElementKind::Dimension(Dimension::about(
                    DimensionKind::Angular,
                    view_id,
                    vertex.0,
                    a.0,
                    b.0,
                    radius,
                )),
            )
        }
        Ready::Circle {
            center,
            r,
            host,
            diameter,
        } => {
            let dir = (at - center.1).normalize_or(DVec2::X);
            let on = attach(model, view, host, center.1 + dir * r);
            let extra = (at.distance(center.1) - r).max(0.0);
            let (name, kind) = if diameter {
                ("Diameter", DimensionKind::Diameter)
            } else {
                ("Radius", DimensionKind::Radius)
            };
            Element::new(
                name,
                ElementKind::Dimension(Dimension::about(kind, view_id, center.0, on, on, extra)),
            )
        }
        Ready::Spot {
            point, coords, bpv, ..
        } => Element::new(
            if coords {
                "S-JTSK coordinates"
            } else {
                "Spot level"
            },
            ElementKind::Dimension(Dimension::spot(
                view_id,
                point.0,
                attach(model, view, point.0.host, at),
                if coords {
                    Some(creabim_core::annotation::SJTSK_PATTERN.to_string())
                } else {
                    bpv.then(|| creabim_core::annotation::BPV_PATTERN.to_string())
                },
            )),
        ),
        Ready::Arc { center, a, b } => {
            let offset = at.distance(center.1) - center.1.distance(a.1);
            Element::new(
                "Arc length",
                ElementKind::Dimension(Dimension::about(
                    DimensionKind::ArcLength,
                    view_id,
                    center.0,
                    a.0,
                    b.0,
                    offset,
                )),
            )
        }
        Ready::Leader { target } => {
            let _ = tool;
            // a leader to what has a mark typed on it (a ceiling's PDH1)
            // reads its mark (George 2026-11-23)
            let text = target
                .0
                .host
                .and_then(|h| model.get(h))
                .and_then(creabim_core::marks::manual)
                .map_or("Note".to_string(), |m| m.to_string());
            let mut n = Note::new(view_id, at, text);
            n.leader = Some(target.0);
            Element::new("Note", ElementKind::Note(n))
        }
    }
}

/// How far `at` is to the left of the line from `a` to `b`.
/// A spot level's preview text: its height as it will read (`+2,700`,
/// `±0,000`, `245,350 m n. m. Bpv`).
fn spot_preview(z: Option<f64>, bpv: bool) -> String {
    let Some(z) = z else {
        return "Spot level".into();
    };
    let t = format!("{:.3}", z.abs()).replace('.', ",");
    if bpv {
        format!("{}{t} m n. m. Bpv", if z < 0.0 { "-" } else { "" })
    } else if t == "0,000" {
        format!("±{t}")
    } else {
        format!("{}{t}", if z < 0.0 { "-" } else { "+" })
    }
}

fn slope_offset(a: DVec2, b: DVec2, at: DVec2) -> f64 {
    (at - a).dot((b - a).normalize_or(DVec2::X).perp())
}

/// What a new slope dimension's pattern is: its format in a section (none
/// for the default percentage), the slope typed in that format in a plan.
pub fn slope_pattern(plan: bool, fmt: usize, value: f64) -> Option<String> {
    let f = creabim_core::annotation::SLOPE_FORMATS
        .get(fmt)
        .copied()
        .unwrap_or("%");
    if !plan {
        return (f != "%").then(|| f.to_string());
    }
    let v = {
        let r = (value * 10.0).round() / 10.0;
        if (r - r.round()).abs() < 1e-9 {
            format!("{r:.0}")
        } else {
            format!("{r:.1}").replace('.', ",")
        }
    };
    Some(match f {
        "°" => format!("{v}°"),
        "1:n" => format!("1:{v}"),
        _ => format!("{v} %"),
    })
}

/// The annotation a placing click at `at` would make, for the preview.
fn shape_of(ready: Ready, at: DVec2) -> (Shape, String) {
    match ready {
        Ready::Slope {
            a,
            b,
            plan,
            fmt,
            value,
        } => (
            Shape::Slope {
                a: a.1,
                b: b.1,
                offset: slope_offset(a.1, b.1, at),
                plan,
            },
            creabim_geom::annotate::slope_reading(
                slope_pattern(plan, fmt, value).as_deref(),
                b.1 - a.1,
                plan,
            ),
        ),
        Ready::Spot {
            point,
            plan,
            coords,
            bpv,
            z,
        } => (
            Shape::Spot {
                at: point.1,
                text_at: at,
                plan,
                cross: coords,
                short: false,
            },
            if coords {
                "Y = …\nX = …".into()
            } else {
                spot_preview(z, bpv)
            },
        ),
        Ready::Angle { vertex, a, b } => {
            let (a, b) = if sweep(vertex.1, a.1, at) > sweep(vertex.1, a.1, b.1) {
                (b.1, a.1)
            } else {
                (a.1, b.1)
            };
            (
                Shape::Angle {
                    vertex: vertex.1,
                    a,
                    b,
                    radius: at.distance(vertex.1),
                },
                angle_text(sweep(vertex.1, a, b)),
            )
        }
        Ready::Circle {
            center,
            r,
            diameter,
            ..
        } => {
            let c = center.1;
            let on = c + (at - c).normalize_or(DVec2::X) * r;
            let kind = if diameter {
                DimensionKind::Diameter
            } else {
                DimensionKind::Radius
            };
            (
                Shape::Radius {
                    center: c,
                    at: on,
                    extra: (at.distance(c) - r).max(0.0),
                    diameter,
                },
                measure(&crate::units::get(), kind, c, on, on),
            )
        }
        Ready::Arc { center, a, b } => (
            Shape::ArcLength {
                center: center.1,
                a: a.1,
                b: b.1,
                offset: at.distance(center.1) - center.1.distance(a.1),
            },
            measure(
                &crate::units::get(),
                DimensionKind::ArcLength,
                center.1,
                a.1,
                b.1,
            ),
        ),
        Ready::Leader { target } => (
            Shape::Note {
                at,
                leader: Some(target.1),
                elbow: None,
                angle: 0.0,
            },
            "Note".into(),
        ),
    }
}

/// What a tool will make, drawn in the accent colour as the pointer moves.
#[allow(clippy::too_many_arguments)]
pub fn preview(
    painter: &egui::Painter,
    xf: &Xf,
    tool: Tool,
    d: &Draft,
    hover: Option<DVec2>,
    raw: Option<DVec2>,
    items: &[PlanItem],
    tol: f64,
    mm: f64,
) {
    let accent = Stroke::new(1.5, interaction_ink());
    if let (Some(ready), Some(at)) = (d.ready, raw) {
        let (shape, text) = shape_of(ready, at);
        let item = Annotation {
            shape,
            text,
            look: TextLook::default(),
            mm,
        };
        paint(painter, xf, &item, interaction_ink(), mm);
        return;
    }
    if let Some((_, a, b, _)) = d.line {
        painter.line_segment(
            [xf.to_screen(a), xf.to_screen(b)],
            Stroke::new(3.0, interaction_ink()),
        );
    }
    for (_, p) in &d.points {
        painter.circle_stroke(xf.to_screen(*p), 4.0, accent);
    }
    if let (Some(first), Some(h)) = (d.points.first(), hover) {
        let pts: Vec<Pos2> = d
            .points
            .iter()
            .map(|p| xf.to_screen(p.1))
            .chain([xf.to_screen(h)])
            .collect();
        if tool == Tool::Angle {
            // the arms from the vertex
            for p in &pts[1..] {
                painter.line_segment([xf.to_screen(first.1), *p], accent);
            }
        } else {
            painter.add(egui::Shape::dashed_line(&pts, accent, 6.0, 4.0));
        }
    }
    // what a click would pick
    if d.points.is_empty() {
        let Some(p) = raw else { return };
        match tool {
            Tool::Radius | Tool::Diameter | Tool::ArcLength => {
                if let Some((_, c, r, s, e)) = arc_at(items, p, tol) {
                    let n = 32;
                    let pts: Vec<Pos2> = (0..=n)
                        .map(|i| {
                            xf.to_screen(
                                c + DVec2::from_angle(s + (e - s) * i as f64 / n as f64) * r,
                            )
                        })
                        .collect();
                    painter.add(egui::Shape::line(pts, Stroke::new(3.0, interaction_ink())));
                }
            }
            Tool::Angle | Tool::Slope => {
                if let Some((_, a, b)) = segment_at(items, p, tol) {
                    painter.line_segment(
                        [xf.to_screen(a), xf.to_screen(b)],
                        Stroke::new(2.0, interaction_ink().gamma_multiply(0.6)),
                    );
                }
            }
            _ => {}
        }
    }
}

// ---- dragging ---------------------------------------------------------------

/// Drag a note (its text, or its leader's arrow when it is selected) or a
/// dimension's line in the Select tool. `snapped` is the point under the
/// pointer and what it attaches to, for a leader. Returns `Some` while a
/// drag is in progress.
#[allow(clippy::too_many_arguments)]
pub fn drag(
    doc: &mut Document,
    view_id: Option<ElementId>,
    view: Option<&View>,
    items: &[PlanItem],
    xf: &Xf,
    response: &egui::Response,
    d: &mut Draft,
    selection: &mut Selection,
    mm: f64,
    snapped: Option<(HostPoint, DVec2)>,
) -> Option<Option<String>> {
    let tol = 6.0 / xf.scale as f64;
    if response.drag_started_by(egui::PointerButton::Primary) {
        let press = response
            .ctx
            .input(|i| i.pointer.press_origin())
            .or(response.interact_pointer_pos())?;
        let w = xf.to_world(press);
        let model = doc.model();
        // the grips of what is selected come first
        let grip = items.iter().rev().find_map(|it| {
            if !selection.contains(it.id()) {
                return None;
            }
            grips(model, it, mm)
                .into_iter()
                .find(|(p, _)| xf.to_screen(*p).distance(press) < 9.0)
                .map(|g| g.1)
        });
        d.grab = grip.or_else(|| {
            items.iter().rev().find_map(|it| {
                let PlanItem::Annotation { id, item } = it else {
                    return None;
                };
                if !item.hit(w, tol, mm) {
                    return None;
                }
                Some(match item.shape {
                    Shape::Note { at, .. } => Grab::Note(*id, w, at),
                    _ => Grab::Shape(*id),
                })
            })
        });
        match d.grab {
            Some(
                Grab::Note(id, ..)
                | Grab::Leader(id)
                | Grab::Shape(id)
                | Grab::Elbow(id)
                | Grab::TagEnd(id, _)
                | Grab::TagElbow(id, _)
                | Grab::LevelHead(id, _),
            ) => selection.only(id),
            None => {}
        }
        d.grab_by = d.grab.map(|_| response.id);
    }
    if d.grab.is_some() && d.grab_by.is_some_and(|o| o != response.id) {
        return None;
    }
    let grab = d.grab?;
    if response.drag_stopped() || !response.dragged() {
        d.grab = None;
        d.grab_by = None;
        return Some(None);
    }
    let p = xf.to_world(response.interact_pointer_pos()?);
    let r = match grab {
        Grab::Note(id, from, start) => doc.edit_merged(id, "Move note", |e| {
            if let ElementKind::Note(n) = &mut e.kind {
                n.at = start + p - from;
            }
        }),
        Grab::Leader(id) => {
            let target = snapped.map_or(HostPoint::free(p), |s| s.0);
            doc.edit_merged(id, "Move leader", |e| {
                if let ElementKind::Note(n) = &mut e.kind {
                    n.leader = Some(target);
                }
            })
        }
        Grab::Shape(id) => {
            let shape = items.iter().find_map(|it| match it {
                PlanItem::Annotation { id: i, item } if *i == id => Some(item.shape.clone()),
                _ => None,
            });
            let (Some(shape), Some(view)) = (shape, view) else {
                return Some(None);
            };
            let model = doc.model().clone();
            doc.edit_merged(id, "Move dimension line", |e| {
                let ElementKind::Dimension(dim) = &mut e.kind else {
                    return;
                };
                match shape {
                    Shape::Angle { vertex, .. } => dim.offset = p.distance(vertex),
                    Shape::Radius { center, at, .. } => {
                        let r = at.distance(center);
                        let dir = (p - center).normalize_or(DVec2::X);
                        let on = attach(&model, view, dim.a.host, center + dir * r);
                        dim.a = on;
                        dim.b = on;
                        dim.offset = (p.distance(center) - r).max(0.0);
                    }
                    Shape::ArcLength { center, a, .. } => {
                        dim.offset = p.distance(center) - a.distance(center)
                    }
                    Shape::Slope { a, b, .. } => dim.offset = slope_offset(a, b, p),
                    // a spot level's text moves, staying with what it is on
                    Shape::Spot { .. } => dim.b = attach(&model, view, dim.a.host, p),
                    Shape::Note { .. } | Shape::Level { .. } | Shape::Along { .. } => {}
                }
            })
        }
        Grab::Elbow(id) => doc.edit_merged(id, "Move leader elbow", |e| {
            if let ElementKind::Note(n) = &mut e.kind {
                n.elbow = Some(p);
            }
        }),
        Grab::TagEnd(id, centre) => doc.edit_merged(id, "Move leader", |e| {
            if let ElementKind::Tag(t) = &mut e.kind {
                t.end = Some(p - centre);
            }
        }),
        Grab::TagElbow(id, centre) => doc.edit_merged(id, "Move leader elbow", |e| {
            if let ElementKind::Tag(t) = &mut e.kind {
                t.elbow = Some(p - centre);
            }
        }),
        Grab::LevelHead(level, end) => {
            let Some(view_id) = view_id else {
                return Some(None);
            };
            doc.edit_merged(view_id, "Move level head", |e| {
                if let ElementKind::View(v) = &mut e.kind {
                    v.level_heads.entry(level).or_default().offset = Some(p - end);
                }
            })
        }
    };
    Some(r.err().map(|e| e.to_string()))
}

/// The grips drawn on selected notes' and tags' leaders (their ends and
/// elbows) and on selected levels' heads.
pub fn draw_grips(
    painter: &egui::Painter,
    xf: &Xf,
    model: &Model,
    items: &[PlanItem],
    selection: &Selection,
    mm: f64,
) {
    // a selected room tag: traced in the accent (planview::trace) and one
    // square to move it by, no box round it
    let mut boxes: Vec<(ElementId, egui::Rect)> = vec![];
    for it in items {
        let PlanItem::Annotation { id, item } = it else {
            continue;
        };
        if !selection.contains(*id) || !matches!(item.shape, Shape::Note { .. }) {
            continue;
        }
        if !matches!(model.get(*id).map(|e| &e.kind), Some(ElementKind::Tag(_))) {
            continue;
        }
        for b in item.text_boxes() {
            let r = egui::Rect::from_points(&b.map(|p| xf.to_screen(p)));
            match boxes.iter_mut().find(|(i, _)| i == id) {
                Some((_, all)) => *all = all.union(r),
                None => boxes.push((*id, r)),
            }
        }
    }
    for (_, r) in boxes {
        let r = r.expand(5.0);
        painter.rect(
            egui::Rect::from_center_size(r.center_top(), Vec2::splat(8.0)),
            1.0,
            Color32::WHITE,
            Stroke::new(1.5, interaction_ink()),
            egui::StrokeKind::Middle,
        );
    }
    for it in items {
        if !selection.contains(it.id()) {
            continue;
        }
        for (p, g) in grips(model, it, mm) {
            let elbow = matches!(g, Grab::Elbow(_) | Grab::TagElbow(..));
            let r = egui::Rect::from_center_size(xf.to_screen(p), Vec2::splat(7.0));
            if elbow {
                painter.circle(
                    r.center(),
                    3.5,
                    Color32::WHITE,
                    Stroke::new(1.5, interaction_ink()),
                );
            } else {
                painter.rect(
                    r,
                    0.0,
                    Color32::WHITE,
                    Stroke::new(1.5, interaction_ink()),
                    egui::StrokeKind::Middle,
                );
            }
        }
    }
}

// ---- sheets -----------------------------------------------------------------

/// Notes on a sheet itself: placing them with the Note tool, picking and
/// dragging them. `xf` maps sheet millimetres to the screen. Returns true
/// when it took the pointer.
#[allow(clippy::too_many_arguments)]
pub fn sheet_input(
    doc: &mut Document,
    sheet: ElementId,
    painter: &egui::Painter,
    response: &egui::Response,
    tool: Tool,
    d: &mut Draft,
    selection: &mut Selection,
    xf: &Xf,
    status: &mut Option<String>,
    error: &mut Option<String>,
) -> bool {
    let items = creabim_geom::annotate::sheet_notes(doc.model(), sheet);
    let to_paper = |p: Pos2| xf.to_world(p);
    let zoom = xf.scale;
    for it in &items {
        if let PlanItem::Annotation { id, item } = it {
            // the sheet draws the note itself; a selected one is traced
            if selection.contains(*id) {
                paint_weighted(
                    painter,
                    xf,
                    item,
                    interaction_ink(),
                    1.0,
                    crate::planview::TRACE,
                );
            }
        }
    }
    draw_grips(painter, xf, doc.model(), &items, selection, 1.0);
    let pointer = response.hover_pos().or(response.interact_pointer_pos());
    if tool == Tool::Note || tool == Tool::Text {
        let raw = pointer.map(to_paper);
        preview(painter, xf, tool, d, raw, raw, &[], 0.0, 1.0);
        if response.clicked() {
            if let Some(p) = raw {
                let mut ghost = Selection::default();
                let r = if tool == Tool::Text {
                    let e = d.styled(Element::new(
                        "Text",
                        ElementKind::Note(d.free_text(sheet, p)),
                    ));
                    ghost.only(e.id);
                    doc.commit(Transaction::new("Add text").insert(e))
                        .map(|_| added(Tool::Text, d))
                } else {
                    sheet_note_click(doc, sheet, p, d, &mut ghost)
                };
                match r {
                    Ok(msg) => *status = Some(msg),
                    Err(e) => *error = Some(e.to_string()),
                }
                if let Some(id) = ghost.active {
                    selection.only(id);
                }
            }
        }
        return true;
    }
    if tool != Tool::Select {
        return false;
    }
    if let Some(r) = drag(
        doc, None, None, &items, xf, response, d, selection, 1.0, None,
    ) {
        if let Some(e) = r {
            *error = Some(e);
        }
        return true;
    }
    if response.clicked() {
        if let Some(p) = pointer.map(to_paper) {
            let hit = items.iter().rev().find_map(|it| match it {
                PlanItem::Annotation { id, item } if item.hit(p, 6.0 / zoom as f64, 1.0) => {
                    Some(*id)
                }
                _ => None,
            });
            if let Some(id) = hit {
                selection.only(id);
                return true;
            }
        }
    }
    false
}

/// A click with the Note tool on a sheet: the leader's point, then the text.
fn sheet_note_click(
    doc: &mut Document,
    sheet: ElementId,
    p: DVec2,
    d: &mut Draft,
    selection: &mut Selection,
) -> Result<String, ModelError> {
    if d.leader && d.ready.is_none() {
        d.ready = Some(Ready::Leader {
            target: (HostPoint::free(p), p),
        });
        return Ok("Now click where the note's text goes.".into());
    }
    let mut n = Note::new(sheet, p, "Note");
    if let Some(Ready::Leader { target }) = d.ready.take() {
        n.leader = Some(target.0);
    }
    d.clear();
    let e = d.styled(Element::new("Note", ElementKind::Note(n)));
    selection.only(e.id);
    doc.commit(Transaction::new("Add note").insert(e))?;
    Ok(added(Tool::Note, d))
}

// ---- properties -------------------------------------------------------------

/// What a style picker did.
pub enum LookEdit {
    None,
    Changed(&'static str),
    /// "New text style…" was picked.
    NewStyle,
}

/// The styles meant for `purpose` (and those for any annotation), by name.
pub fn styles_for(model: &Model, purpose: StyleUse) -> Vec<(ElementId, String)> {
    model
        .iter()
        .filter(|e| match &e.kind {
            ElementKind::TextStyle(s) => s.purpose == purpose || s.purpose == StyleUse::Any,
            _ => false,
        })
        .map(|e| (e.id, e.name.clone()))
        .collect()
}

/// The dimensions, tags and notes that use a style.
pub fn style_user_ids(model: &Model, id: ElementId) -> Vec<ElementId> {
    model
        .iter()
        .filter(|e| style_of(&e.kind) == Some(id))
        .map(|e| e.id)
        .collect()
}

/// The text styles in the model, by name.
pub fn styles(model: &Model) -> Vec<(ElementId, String)> {
    model
        .iter()
        .filter(|e| matches!(e.kind, ElementKind::TextStyle(_)))
        .map(|e| (e.id, e.name.clone()))
        .collect()
}

/// A text style picker and a colour of its own, for a dimension, tag or note.
pub fn look_ui(
    ui: &mut egui::Ui,
    model: &Model,
    style: &mut Option<ElementId>,
    color: &mut Option<[f32; 3]>,
) -> LookEdit {
    let all = styles(model);
    let name = style
        .and_then(|id| all.iter().find(|s| s.0 == id))
        .map_or("Default", |s| s.1.as_str())
        .to_string();
    let mut out = LookEdit::None;
    crate::app::helpui::combo(ui, "Text style")
        .selected_text(crate::i18n::tw(name))
        .show_ui(ui, |ui| {
            if ui
                .selectable_value(style, None, crate::i18n::tw("Default"))
                .changed()
            {
                out = LookEdit::Changed("Text style");
            }
            for (id, n) in &all {
                if ui
                    .selectable_value(style, Some(*id), crate::i18n::tw(n))
                    .changed()
                {
                    out = LookEdit::Changed("Text style");
                }
            }
            ui.separator();
            if ui.button(crate::i18n::t("New text style…")).clicked() {
                out = LookEdit::NewStyle;
            }
        });
    // the switch, then the swatch on a row of its own (MU34: the swatch
    // sat past the panel's edge)
    let mut own = color.is_some();
    if crate::props::switch(ui, &mut own, "Own colour")
        .on_hover_text(crate::i18n::t("Override the style's colour for this one"))
        .changed()
    {
        *color = own.then_some([0.8, 0.1, 0.1]);
        out = LookEdit::Changed("Annotation colour");
    }
    if let Some(c) = color {
        crate::props::row(ui, "Colour", |ui| {
            let h = ui.spacing().interact_size.y;
            ui.spacing_mut().interact_size.x = ui.available_width().max(40.0);
            ui.spacing_mut().interact_size.y = h;
            if ui.color_edit_button_rgb(c).changed() {
                out = LookEdit::Changed("Annotation colour");
            }
        });
    }
    out
}

/// Add a text style, named after how many there are. Returns its id.
pub fn new_text_style(doc: &mut Document) -> Result<ElementId, ModelError> {
    let n = styles(doc.model()).len() + 1;
    let e = Element::new(
        format!("Text style {n}"),
        ElementKind::TextStyle(TextStyle::default()),
    );
    let id = e.id;
    doc.commit(Transaction::new("New text style").insert(e))?;
    Ok(id)
}

/// Point an annotation at a text style.
pub fn set_style(kind: &mut ElementKind, id: ElementId) {
    match kind {
        ElementKind::Dimension(d) => d.style = Some(id),
        ElementKind::Tag(t) => t.style = Some(id),
        ElementKind::Note(n) => n.style = Some(id),
        _ => {}
    }
}

/// A text style's font, size, colour, weight and slant.
pub fn style_ui(ui: &mut egui::Ui, s: &mut TextStyle) -> Option<&'static str> {
    let mut label = None;
    crate::app::helpui::combo(ui, "Font")
        .selected_text(crate::i18n::tw(s.font.label()))
        .show_ui(ui, |ui| {
            for f in Font::ALL {
                if ui
                    .selectable_value(&mut s.font, f, crate::i18n::tw(f.label()))
                    .changed()
                {
                    label = Some("Text font");
                }
            }
        });
    // full-width fields in the label column, as every other row (MU29)
    crate::props::row(ui, "Size on paper", |ui| {
        if crate::props::fill(
            ui,
            egui::DragValue::new(&mut s.size)
                .speed(0.05)
                .range(0.5..=50.0)
                .suffix(crate::i18n::t(" mm")),
        )
        .changed()
        {
            label = Some("Text size");
        }
    });
    crate::props::flow(ui, |ui| {
        let mut own = s.color.is_some();
        if crate::props::switch(ui, &mut own, "Colour")
            .on_hover_text(crate::i18n::t("Off: black on paper, blue on screen"))
            .changed()
        {
            s.color = own.then_some([0.0, 0.0, 0.0]);
            label = Some("Text colour");
        }
        if let Some(c) = &mut s.color {
            if ui.color_edit_button_rgb(c).changed() {
                label = Some("Text colour");
            }
        }
    });
    // Blender's rows (MU31): toggles in the field column, then how the
    // lines line up
    crate::props::row(ui, "Emphasis", |ui| {
        crate::props::flow(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 1.0;
            let h = ui.spacing().interact_size.y;
            let w = ((ui.available_width() - 1.0) / 2.0).max(30.0);
            if ui
                .add_sized([w, h], egui::Button::selectable(s.bold, "Bold"))
                .clicked()
            {
                s.bold = !s.bold;
                label = Some("Bold text");
            }
            if ui
                .add_sized([w, h], egui::Button::selectable(s.italic, "Italic"))
                .clicked()
            {
                s.italic = !s.italic;
                label = Some("Italic text");
            }
        })
    });
    use creabim_core::TextAlign;
    if crate::props::choice(
        ui,
        "Align",
        &mut s.align,
        &[
            (
                TextAlign::Usual,
                "Usual",
                "Each kind's own: notes from the left, room tags centred",
            ),
            (
                TextAlign::Left,
                "Left",
                "Every line from the block's left edge",
            ),
            (
                TextAlign::Centre,
                "Centre",
                "Every line on the block's middle",
            ),
            (
                TextAlign::Right,
                "Right",
                "Every line against the block's right edge",
            ),
        ],
    ) {
        label = Some("Text alignment");
    }
    if let Some(l) = ends_ui(ui, s) {
        label = Some(l);
    }
    label
}

/// A terminator picker, with "Usual" for none.
fn terminator_combo(
    ui: &mut egui::Ui,
    salt: &str,
    text: &str,
    usual: &str,
    t: &mut Option<Terminator>,
) -> bool {
    let mut changed = false;
    crate::props::row(ui, text, |ui| {
        egui::ComboBox::from_id_salt(salt)
            .icon(crate::props::combo_icon)
            .selected_text(crate::i18n::tw(
                t.map_or(usual.to_string(), |t| t.label().to_string()),
            ))
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                changed |= ui
                    .selectable_value(t, None, crate::i18n::tw(usual))
                    .changed();
                for k in Terminator::ALL {
                    changed |= ui
                        .selectable_value(t, Some(k), crate::i18n::tw(k.label()))
                        .changed();
                }
            });
    });
    changed
}

/// The end of one tag's or note's leader, and its elbow: picked here or
/// left to its style. Returns the undo label of a change.
pub fn leader_ui(
    ui: &mut egui::Ui,
    end: &mut Option<Terminator>,
    usual: &str,
    elbow: &mut Option<DVec2>,
) -> Option<&'static str> {
    let mut label = None;
    crate::props::flow(ui, |ui| {
        if terminator_combo(ui, "leader_end_own", "Leader end", usual, end) {
            label = Some("Leader end");
        }
    });
    if elbow.is_some()
        && ui
            .button(crate::i18n::t("Straighten the leader"))
            .on_hover_text(crate::i18n::t(
                "Take its elbow out; drag the round grip on the leader to bend it again",
            ))
            .clicked()
    {
        *elbow = None;
        label = Some("Straighten leader");
    }
    label
}

/// A style's line ends, tag shape and what it is for.
pub fn ends_ui(ui: &mut egui::Ui, s: &mut TextStyle) -> Option<&'static str> {
    let mut label = None;
    crate::props::flow(ui, |ui| {
        if terminator_combo(
            ui,
            "style_end",
            "Dimension ends",
            "Usual (tick, arrow)",
            &mut s.terminator,
        ) {
            label = Some("Dimension ends");
        }
    });
    crate::props::flow(ui, |ui| {
        if terminator_combo(
            ui,
            "style_leader",
            "Leader end",
            "Usual (arrow)",
            &mut s.leader_end,
        ) {
            label = Some("Leader end");
        }
    });
    crate::props::row(ui, "End size", |ui| {
        let mut size = s.end_size.unwrap_or(creabim_geom::annotation::END_SIZE);
        if crate::props::fill(
            ui,
            egui::DragValue::new(&mut size)
                .speed(0.05)
                .range(0.5..=10.0)
                .suffix(crate::i18n::t(" mm")),
        )
        .changed()
        {
            s.end_size = Some(size);
            label = Some("Terminator size");
        }
    });
    crate::app::helpui::combo(ui, "Tag shape")
        .selected_text(crate::i18n::tw(s.tag_shape.label()))
        .show_ui(ui, |ui| {
            for k in TagShape::ALL {
                if ui
                    .selectable_value(&mut s.tag_shape, k, crate::i18n::tw(k.label()))
                    .changed()
                {
                    label = Some("Tag shape");
                }
            }
        });
    if crate::props::switch(ui, &mut s.break_at_tags, "Break dimension lines at tags")
        .on_hover_text(crate::i18n::t(
            "Dimension lines in this style leave a gap where a tag or a note sits across them",
        ))
        .changed()
    {
        label = Some("Dimension breaks");
    }
    if crate::props::switch(ui, &mut s.back, "Text background")
        .on_hover_text(crate::i18n::t(
            "Texts in this style cover what is drawn under them: dimension numbers, spot levels, notes",
        ))
        .changed()
    {
        label = Some("Text background");
    }
    crate::app::helpui::combo(ui, "Meant for")
        .selected_text(crate::i18n::tw(s.purpose.label()))
        .show_ui(ui, |ui| {
            for k in StyleUse::ALL {
                if ui
                    .selectable_value(&mut s.purpose, k, crate::i18n::tw(k.label()))
                    .changed()
                {
                    label = Some("Style use");
                }
            }
        });
    label
}

/// The text style a dimension, tag or note refers to.
pub fn style_of(kind: &ElementKind) -> Option<ElementId> {
    match kind {
        ElementKind::Dimension(d) => d.style,
        ElementKind::Tag(t) => t.style,
        ElementKind::Note(n) => n.style,
        _ => None,
    }
}

/// The style an annotation uses, editable in place under its picker, so a
/// change shows at once on everything using it. `open` unfolds it (for a
/// style just made). Returns a status message after an edit.
pub fn style_inline(
    ui: &mut egui::Ui,
    doc: &mut Document,
    kind: &ElementKind,
    open: bool,
) -> Option<String> {
    let id = style_of(kind)?;
    let e = doc.model().get(id)?;
    let ElementKind::TextStyle(mut s) = e.kind else {
        return None;
    };
    let users = style_users(doc.model(), id);
    let mut label = None;
    // a section of its own, with the panel's chevron (MU26)
    let key = egui::Id::new(("style.open", id));
    if open {
        ui.ctx().data_mut(|d| d.insert_temp(key, true));
    }
    let shown = ui.ctx().data(|d| d.get_temp::<bool>(key)).unwrap_or(false);
    let title = format!("Text style \u{201c}{}\u{201d}", e.name);
    let (resp, _) = crate::props::header(ui, &title, shown, false);
    if resp.clicked() {
        ui.ctx().data_mut(|d| d.insert_temp(key, !shown));
    }
    if shown {
        label = style_ui(ui, &mut s);
        ui.label(
            egui::RichText::new(crate::i18n::tr(&format!(
                "Changes every one of its {users} users."
            )))
            .weak(),
        );
    }
    let label = label?;
    let r = doc.edit_merged(id, label, |e| e.kind = ElementKind::TextStyle(s));
    Some(match r {
        Ok(()) => format!("{label}."),
        Err(e) => format!("Couldn't do that: {e}"),
    })
}

/// How many dimensions, tags and notes use a style.
pub fn style_users(model: &Model, id: ElementId) -> usize {
    model
        .iter()
        .filter(|e| match &e.kind {
            ElementKind::Dimension(d) => d.style == Some(id),
            ElementKind::Tag(t) => t.style == Some(id),
            ElementKind::Note(n) => n.style == Some(id),
            _ => false,
        })
        .count()
}

/// What a dimension's offset means, for its kind.
pub fn offset_label(kind: DimensionKind) -> &'static str {
    match kind {
        DimensionKind::Linear => "Line offset",
        DimensionKind::Angular => "Arc radius",
        DimensionKind::Radius | DimensionKind::Diameter => "Line past circle",
        DimensionKind::ArcLength => "Line offset",
        DimensionKind::Spot => "Offset",
        DimensionKind::Slope => "Arrow offset",
    }
}

/// A note's text and leader. `unit` is a drawing unit's worth of a
/// leader's default reach (larger on sheets, which are in mm).
pub fn note_ui(
    ui: &mut egui::Ui,
    n: &mut Note,
    focus: &mut bool,
    unit: f64,
    pop_out: &mut bool,
) -> Option<&'static str> {
    let mut label = None;
    // the field keeps its size and scrolls inside; the small button at its
    // corner opens the large editor (MU27)
    crate::props::flow(ui, |ui| {
        ui.label(crate::i18n::t("Text"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if crate::props::icon_button(
                ui,
                "maximise",
                "Edit in a larger window (or double-click the note)",
            )
            .clicked()
            {
                *pop_out = true;
            }
        });
    });
    let r = egui::ScrollArea::vertical()
        .id_salt("note.text")
        .max_height(110.0)
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut n.text)
                    .desired_rows(4)
                    .desired_width(f32::INFINITY),
            )
        })
        .inner;
    if std::mem::take(focus) {
        r.request_focus();
        // select it all, so typing replaces the placeholder
        if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), r.id) {
            let all = egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(n.text.chars().count()),
            );
            state.cursor.set_char_range(Some(all));
            state.store(ui.ctx(), r.id);
        }
    }
    if r.changed() {
        label = Some("Note text");
    }
    // its letters' height on paper (none: its style's) and its turn
    crate::props::row(ui, "Text size", |ui| {
        crate::props::flow(ui, |ui| {
            let mut own = n.size.is_some();
            if ui
                .checkbox(&mut own, crate::i18n::t("Own"))
                .on_hover_text(crate::i18n::t(
                    "Its own letter height on paper; off, its style's",
                ))
                .changed()
            {
                n.size = own.then_some(3.5);
                label = Some("Text size");
            }
            if let Some(h) = &mut n.size {
                if ui
                    .add(
                        egui::DragValue::new(h)
                            .speed(0.1)
                            .range(0.5..=100.0)
                            .max_decimals(1)
                            .suffix(crate::i18n::t(" mm")),
                    )
                    .changed()
                {
                    label = Some("Text size");
                }
            }
        });
    });
    crate::props::row(ui, "Angle", |ui| {
        if ui
            .add(
                egui::DragValue::new(&mut n.angle)
                    .speed(1.0)
                    .range(-360.0..=360.0)
                    .max_decimals(2)
                    .suffix("°"),
            )
            .on_hover_text(crate::i18n::t(
                "How far its lines turn on paper, anticlockwise (R turns it with the mouse)",
            ))
            .changed()
        {
            label = Some("Text angle");
        }
    });
    // a text box (MU26): wraps to a width, reads markdown
    let mut boxed = n.width.is_some();
    crate::props::row(ui, "Box", |ui| {
        crate::props::flow(ui, |ui| {
            if ui
                .checkbox(&mut boxed, crate::i18n::t("Wrap at"))
                .on_hover_text(crate::i18n::t(
                    "Lines wrap to this width on paper; off, each line runs as long as it is",
                ))
                .changed()
            {
                n.width = boxed.then_some(60.0);
                label = Some("Note box");
            }
            if let Some(w) = &mut n.width {
                if ui
                    .add(
                        egui::DragValue::new(w)
                            .speed(0.5)
                            .range(5.0..=800.0)
                            .suffix(crate::i18n::t(" mm")),
                    )
                    .changed()
                {
                    label = Some("Note box");
                }
            }
        });
    });
    if crate::props::switch(ui, &mut n.markdown, "Markdown")
        .on_hover_text(crate::i18n::t("# heading (capitals, underlined), - bullet, 1. numbered, - [ ] and - [x] checkboxes; **bold** and *italic* marks are dropped on paper"))
        .changed()
    {
        label = Some("Note markdown");
    }
    if crate::props::switch(ui, &mut n.back, "Text background")
        .on_hover_text(crate::i18n::t("The text covers what is drawn under it"))
        .changed()
    {
        label = Some("Text background");
    }
    let mut leader = n.leader.is_some();
    if crate::props::switch(ui, &mut leader, "Leader")
        .on_hover_text(crate::i18n::t(
            "An arrow from the note to what it is about. Drag its square to move it.",
        ))
        .changed()
    {
        n.leader = leader.then(|| HostPoint::free(n.at + DVec2::new(-1.5, -1.0) * unit));
        label = Some("Note leader");
    }
    if n.leader.is_some_and(|l| l.host.is_some()) {
        ui.label(
            egui::RichText::new(crate::i18n::t(
                "Its arrow is attached and follows what it points at.",
            ))
            .weak(),
        );
    }
    label
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::ViewKind;
    use creabim_geom::plan::Pen;

    fn plan_of(doc: &Document) -> (ElementId, View) {
        doc.model()
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::View(v) if matches!(v.kind, ViewKind::Plan { .. }) => {
                    Some((e.id, v.clone()))
                }
                _ => None,
            })
            .unwrap()
    }

    /// The one element added since the model had `before`.
    fn added_since(doc: &Document, before: &[ElementId]) -> Element {
        let new: Vec<&Element> = doc
            .model()
            .iter()
            .filter(|e| !before.contains(&e.id))
            .collect();
        assert_eq!(new.len(), 1);
        new[0].clone()
    }

    fn ids(doc: &Document) -> Vec<ElementId> {
        doc.model().iter().map(|e| e.id).collect()
    }

    #[test]
    fn two_lines_make_an_angle_on_the_clicked_side() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let (vid, view) = plan_of(&doc);
        let line = |a: DVec2, b: DVec2| PlanItem::Line {
            id: ElementId::NIL,
            pen: Pen::Projection,
            a,
            b,
        };
        let items = [
            line(DVec2::new(0.0, 0.0), DVec2::new(4.0, 0.0)),
            line(DVec2::new(1.0, 1.0), DVec2::new(1.0, 5.0)),
        ];
        let (mut d, mut sel) = (Draft::default(), Selection::default());
        let before = ids(&doc);
        for at in [
            DVec2::new(3.0, 0.02),
            DVec2::new(1.03, 3.0),
            DVec2::new(2.0, 1.5),
        ] {
            click(
                &mut doc,
                vid,
                &view,
                Tool::Angle,
                None,
                at,
                &items,
                0.1,
                &mut d,
                &mut sel,
            )
            .unwrap();
        }
        let e = added_since(&doc, &before);
        let ElementKind::Dimension(dim) = &e.kind else {
            panic!("not a dimension")
        };
        assert_eq!(dim.kind, DimensionKind::Angular);
        assert!((dim.offset - DVec2::new(1.0, 1.5).length()).abs() < 1e-9);
        let drawing = creabim_geom::section::view_drawing(doc.model(), vid).unwrap();
        let text = drawing.items.iter().find_map(|i| match i {
            PlanItem::Annotation { id, item } if *id == e.id => Some(item.text.clone()),
            _ => None,
        });
        assert_eq!(text.as_deref(), Some("90°"));
        // undo takes it away
        doc.undo();
        assert_eq!(ids(&doc), before);
    }

    #[test]
    fn a_coordinate_mark_writes_y_and_x_after_a_spot_level() {
        // a spot level placed before used to hand its "±0,000" pattern
        // to the coordinate mark made after it
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let (vid, view) = plan_of(&doc);
        let mut info = doc
            .model()
            .project_info()
            .map(|(_, p)| p.clone())
            .unwrap_or_default();
        info.sjtsk = Some([-547382.28, -1253318.85]);
        let tx = creabim_core::project::set_project(doc.model(), info);
        doc.commit(tx).unwrap();
        let (mut sel, mut placed) = (Selection::default(), vec![]);
        for coords in [false, true, true] {
            let mut d = Draft {
                spot_coords: coords,
                ..Draft::default()
            };
            let before = ids(&doc);
            for at in [DVec2::new(1.0, 1.0), DVec2::new(3.0, 2.0)] {
                click(
                    &mut doc,
                    vid,
                    &view,
                    Tool::SpotLevel,
                    None,
                    at,
                    &[],
                    0.1,
                    &mut d,
                    &mut sel,
                )
                .unwrap();
            }
            placed.push(added_since(&doc, &before).id);
        }
        let drawing = creabim_geom::section::view_drawing(doc.model(), vid).unwrap();
        let text = |id: ElementId| {
            drawing
                .items
                .iter()
                .find_map(|i| match i {
                    PlanItem::Annotation { id: j, item } if *j == id => Some(item.text.clone()),
                    _ => None,
                })
                .unwrap()
        };
        assert!(!text(placed[0]).contains("Y ="), "{}", text(placed[0]));
        for id in &placed[1..] {
            assert!(text(*id).contains("547"), "{}", text(*id));
        }
    }

    #[test]
    fn an_arc_gives_its_radius_and_a_note_its_leader() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let (vid, view) = plan_of(&doc);
        let items = [PlanItem::Arc {
            id: ElementId::NIL,
            pen: Pen::Symbol,
            center: DVec2::new(5.0, 5.0),
            radius: 2.0,
            start: 0.0,
            end: std::f64::consts::PI,
        }];
        let (mut d, mut sel) = (Draft::default(), Selection::default());
        let before = ids(&doc);
        let mut go = |doc: &mut Document, tool: Tool, at: DVec2, items: &[PlanItem]| {
            click(
                doc, vid, &view, tool, None, at, items, 0.1, &mut d, &mut sel,
            )
            .unwrap();
        };
        go(&mut doc, Tool::Radius, DVec2::new(5.0, 7.03), &items);
        go(&mut doc, Tool::Radius, DVec2::new(8.0, 5.0), &items);
        let e = added_since(&doc, &before);
        let ElementKind::Dimension(dim) = &e.kind else {
            panic!("not a dimension")
        };
        assert_eq!(dim.kind, DimensionKind::Radius);
        assert!(dim.a.local.distance(DVec2::new(7.0, 5.0)) < 1e-9);
        assert!((dim.offset - 1.0).abs() < 1e-9);
        doc.undo();

        // a note: its arrow's point, then its text
        go(&mut doc, Tool::Note, DVec2::new(1.0, 1.0), &[]);
        go(&mut doc, Tool::Note, DVec2::new(3.0, 2.0), &[]);
        let e = added_since(&doc, &before);
        let ElementKind::Note(note) = &e.kind else {
            panic!("not a note")
        };
        assert_eq!(note.at, DVec2::new(3.0, 2.0));
        assert_eq!(note.leader.unwrap().local, DVec2::new(1.0, 1.0));
        assert!(d.focus_text);
        assert_eq!(sel.active, Some(e.id));
    }

    #[test]
    fn a_spot_level_reads_the_roof_it_is_clicked_on_in_bpv_too() {
        // George: on a roof it wrote the floor's level, not the roof's
        // height where it was clicked
        let mut doc = Document::from_model(creabim_core::sample::layered_house());
        let ff = doc
            .model()
            .iter()
            .find(|e| e.name == "First Floor")
            .unwrap()
            .id;
        let roof = doc
            .model()
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Roof(_)))
            .unwrap()
            .id;
        let mut v = View::plan(ff);
        v.graphics.range.from_above = true;
        let e = Element::new("Roof plan", ElementKind::View(v.clone()));
        let vid = e.id;
        doc.commit(Transaction::new("plan").insert(e)).unwrap();
        let mut info = doc
            .model()
            .project_info()
            .map(|(_, i)| i.clone())
            .unwrap_or_default();
        info.elevation = 245.35;
        doc.commit(creabim_core::set_project(doc.model(), info))
            .unwrap();
        let (mut d, mut sel) = (Draft::default(), Selection::default());
        let at = DVec2::new(5.0, 0.8);
        let mut place = |doc: &mut Document, d: &mut Draft| {
            let before = ids(doc);
            for p in [at, at + DVec2::new(1.0, -1.5)] {
                click(
                    doc,
                    vid,
                    &v,
                    Tool::SpotLevel,
                    None,
                    p,
                    &[],
                    0.1,
                    d,
                    &mut sel,
                )
                .unwrap();
            }
            let e = added_since(doc, &before);
            let drawing = creabim_geom::section::view_drawing(doc.model(), vid).unwrap();
            let text = drawing.items.iter().find_map(|i| match i {
                PlanItem::Annotation { id, item } if *id == e.id => Some(item.text.clone()),
                _ => None,
            });
            (e, text.unwrap())
        };
        let (e, text) = place(&mut doc, &mut d);
        let ElementKind::Dimension(dim) = &e.kind else {
            panic!("not a dimension")
        };
        assert_eq!(dim.a.host, Some(roof));
        let z = creabim_geom::annotate::surface_height(doc.model(), roof, at).unwrap();
        let level = doc.model().level(ff).unwrap().elevation;
        assert!(z > level + 3.0, "{z}");
        let want = creabim_core::annotation::spot_text(
            doc.model(),
            creabim_core::annotation::SPOT_PATTERN,
            z,
        );
        assert_eq!(text, want);
        // read in Bpv: the project's ±0,000 in Bpv plus the height
        d.spot_bpv = true;
        let (_, text) = place(&mut doc, &mut d);
        assert_eq!(
            text,
            format!(
                "{} m n. m. Bpv",
                format!("{:.3}", 245.35 + z).replace('.', ",")
            )
        );
    }

    /// George: with the pointer stuck to a roof's edge, the spot level
    /// sat off the edge, where the pointer was, and read the height there.
    /// It sits on the point snapped to and reads the ridge's height.
    #[test]
    fn a_spot_level_snapped_to_a_ridge_sits_on_it_and_reads_it() {
        let mut doc = Document::from_model(creabim_core::sample::layered_house());
        let ff = doc
            .model()
            .iter()
            .find(|e| e.name == "First Floor")
            .unwrap()
            .id;
        let roof = doc
            .model()
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Roof(_)))
            .unwrap()
            .id;
        let top = creabim_geom::scene::shared_scene(doc.model())
            .iter()
            .filter(|i| i.id == roof)
            .flat_map(|i| i.mesh.positions.clone())
            .max_by(|a, b| a[2].total_cmp(&b[2]))
            .unwrap();
        let ridge = DVec2::new(top[0] as f64, top[1] as f64);
        let mut v = View::plan(ff);
        v.graphics.range.from_above = true;
        let e = Element::new("Roof plan", ElementKind::View(v.clone()));
        let vid = e.id;
        doc.commit(Transaction::new("plan").insert(e)).unwrap();
        let (mut d, mut sel) = (Draft::default(), Selection::default());
        let before = ids(&doc);
        // the pointer a little off the ridge, stuck to it
        let raw = ridge + DVec2::new(0.2, 0.3);
        let hit = Some((HostPoint::free(ridge), ridge, Some(SnapKind::Endpoint)));
        click(
            &mut doc,
            vid,
            &v,
            Tool::SpotLevel,
            hit,
            raw,
            &[],
            0.1,
            &mut d,
            &mut sel,
        )
        .unwrap();
        let text_at = ridge + DVec2::new(1.0, 1.0);
        click(
            &mut doc,
            vid,
            &v,
            Tool::SpotLevel,
            None,
            text_at,
            &[],
            0.1,
            &mut d,
            &mut sel,
        )
        .unwrap();
        let e = added_since(&doc, &before);
        let ElementKind::Dimension(dim) = &e.kind else {
            panic!("not a dimension")
        };
        assert_eq!(dim.a.host, Some(roof));
        assert!(
            dim.a.local.distance(ridge) < 1e-6,
            "{:?} {ridge:?}",
            dim.a.local
        );
        let drawing = creabim_geom::section::view_drawing(doc.model(), vid).unwrap();
        let text = drawing
            .items
            .iter()
            .find_map(|i| match i {
                PlanItem::Annotation { id, item } if *id == e.id => Some(item.text.clone()),
                _ => None,
            })
            .unwrap();
        let want = creabim_core::annotation::spot_text(
            doc.model(),
            creabim_core::annotation::SPOT_PATTERN,
            top[2] as f64,
        );
        assert_eq!(text, want);
    }

    /// George: every annotation must work in plans, the site plan,
    /// sections and elevations; each tool here placed in each and drawn
    /// there, a turned free text too.
    #[test]
    fn every_annotation_tool_works_in_every_kind_of_drawing() {
        let mut m = creabim_core::sample::sample_house();
        let (plan, _) = {
            let doc = Document::from_model(m.clone());
            plan_of(&doc)
        };
        let ElementKind::View(pv) = m.get(plan).unwrap().kind.clone() else {
            unreachable!()
        };
        let mut site = pv.clone();
        site.graphics.range.from_above = true;
        site.scale = 200.0;
        let section = View::section(DVec2::new(-1.0, 3.0), DVec2::new(12.0, 3.0), 8.0, true);
        let elevation = View::section(DVec2::new(-1.0, -4.0), DVec2::new(12.0, -4.0), 30.0, false);
        let views: Vec<Element> = [
            ("Site", site),
            ("Section", section),
            ("Elevation", elevation),
        ]
        .into_iter()
        .map(|(n, v)| Element::new(n, ElementKind::View(v)))
        .collect();
        Transaction::new("views")
            .insert(views[0].clone())
            .insert(views[1].clone())
            .insert(views[2].clone())
            .apply(&mut m)
            .unwrap();
        let mut doc = Document::from_model(m);
        let p = |x: f64, y: f64| DVec2::new(x, y);
        let tools: [(Tool, Vec<DVec2>); 8] = [
            (Tool::Text, vec![p(2.0, 1.0)]),
            (Tool::Note, vec![p(1.0, 1.0), p(3.0, 2.0)]),
            (
                Tool::Angle,
                vec![p(1.0, 1.0), p(3.0, 1.0), p(1.0, 3.0), p(2.0, 2.0)],
            ),
            (Tool::Slope, vec![p(1.0, 2.0), p(4.0, 1.0), p(2.0, 2.5)]),
            (
                Tool::Radius,
                vec![p(3.0, 1.0), p(1.0, 3.0), p(-1.0, 1.0), p(4.0, 2.0)],
            ),
            (
                Tool::Diameter,
                vec![p(3.0, 1.0), p(1.0, 3.0), p(-1.0, 1.0), p(4.0, 2.0)],
            ),
            (
                Tool::ArcLength,
                vec![p(3.0, 1.0), p(1.0, 3.0), p(-1.0, 1.0), p(4.0, 2.0)],
            ),
            (Tool::SpotLevel, vec![p(2.0, 0.0), p(3.0, 1.0)]),
        ];
        let ids_of_views: Vec<ElementId> = std::iter::once(plan)
            .chain(views.iter().map(|v| v.id))
            .collect();
        for vid in ids_of_views {
            let Some(ElementKind::View(view)) = doc.model().get(vid).map(|e| e.kind.clone()) else {
                unreachable!()
            };
            for (tool, clicks) in &tools {
                let (mut d, mut sel) = (Draft::default(), Selection::default());
                d.text_size = 5.0;
                let before = ids(&doc);
                for at in clicks {
                    click(
                        &mut doc,
                        vid,
                        &view,
                        *tool,
                        None,
                        *at,
                        &[],
                        0.1,
                        &mut d,
                        &mut sel,
                    )
                    .unwrap();
                }
                let e = added_since(&doc, &before);
                if *tool == Tool::Text {
                    // turned by 30°, as R would
                    let mut t = e.clone();
                    let ElementKind::Note(n) = &mut t.kind else {
                        panic!("a text is a note")
                    };
                    assert_eq!(n.size, Some(5.0));
                    assert!(n.leader.is_none());
                    n.angle = 30.0;
                    doc.commit(Transaction::new("turn").replace(e.clone(), t))
                        .unwrap();
                }
                let drawing = creabim_geom::section::view_drawing(doc.model(), vid).unwrap();
                let item = drawing.items.iter().find_map(|i| match i {
                    PlanItem::Annotation { id, item } if *id == e.id => Some(item.clone()),
                    _ => None,
                });
                let item = item.unwrap_or_else(|| {
                    panic!(
                        "{tool:?} not drawn in {}",
                        doc.model().get(vid).unwrap().name
                    )
                });
                if *tool == Tool::Text {
                    let t = &item.marks(item.mm).texts[0];
                    assert!((t.angle - 30f64.to_radians()).abs() < 1e-9);
                    assert_eq!(item.look.size, 5.0);
                }
            }
        }
    }

    #[test]
    fn a_slope_in_a_section_reads_its_line_and_points_down_it() {
        let mut doc = Document::new();
        let v = Element::new(
            "Section A",
            ElementKind::View(View::section(DVec2::ZERO, DVec2::new(8.0, 0.0), 3.0, true)),
        );
        let (vid, view) = match &v.kind {
            ElementKind::View(view) => (v.id, view.clone()),
            _ => unreachable!(),
        };
        doc.commit(Transaction::new("setup").insert(v)).unwrap();
        let (mut d, mut sel) = (Draft::default(), Selection::default());
        let before = ids(&doc);
        // a roof falling 1 m over 4 m, from left to right; arrow above it
        for at in [
            DVec2::new(0.0, 3.0),
            DVec2::new(4.0, 2.0),
            DVec2::new(2.0, 3.0),
        ] {
            click(
                &mut doc,
                vid,
                &view,
                Tool::Slope,
                None,
                at,
                &[],
                0.1,
                &mut d,
                &mut sel,
            )
            .unwrap();
        }
        let e = added_since(&doc, &before);
        let ElementKind::Dimension(dim) = &e.kind else {
            panic!("not a dimension")
        };
        assert_eq!(dim.kind, DimensionKind::Slope);
        assert!(dim.offset > 0.0, "above the roof: {}", dim.offset);
        let drawing = creabim_geom::section::view_drawing(doc.model(), vid).unwrap();
        let item = drawing
            .items
            .iter()
            .find_map(|i| match i {
                PlanItem::Annotation { id, item } if *id == e.id => Some(item.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(item.text, "25 %");
        // the arrowhead is at the lower, right end
        let m = item.marks(0.05);
        let tip = m.arrows[0][0];
        assert!(tip.x > 2.0 && tip.y < 2.5 + dim.offset, "{tip}");
        // in degrees, from Properties
        doc.edit(e.id, "fmt", |el| {
            if let ElementKind::Dimension(d) = &mut el.kind {
                d.pattern = Some("°".into());
            }
        })
        .unwrap();
        let drawing = creabim_geom::section::view_drawing(doc.model(), vid).unwrap();
        assert!(drawing.items.iter().any(
            |i| matches!(i, PlanItem::Annotation { id, item } if *id == e.id && item.text == "14°")
        ));
    }

    #[test]
    fn a_slope_in_a_plan_writes_the_slope_typed() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let (vid, view) = plan_of(&doc);
        let (mut d, mut sel) = (Draft::default(), Selection::default());
        d.slope_value = 2.5;
        let before = ids(&doc);
        for at in [
            DVec2::new(1.0, 1.0),
            DVec2::new(3.0, 1.0),
            DVec2::new(2.0, 0.5),
        ] {
            click(
                &mut doc,
                vid,
                &view,
                Tool::Slope,
                None,
                at,
                &[],
                0.1,
                &mut d,
                &mut sel,
            )
            .unwrap();
        }
        let e = added_since(&doc, &before);
        let ElementKind::Dimension(dim) = &e.kind else {
            panic!("not a dimension")
        };
        assert_eq!(dim.pattern.as_deref(), Some("2,5 %"));
        assert!(dim.offset < 0.0);
        assert_eq!(slope_pattern(true, 2, 50.0).as_deref(), Some("1:50"));
        assert_eq!(slope_pattern(false, 0, 2.0), None);
    }
}
