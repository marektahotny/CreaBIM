//! How a view's drawing looks on paper: its pens, and the object styles,
//! hatch patterns and graphic overrides of the project's style pack. They
//! are resolved per element once, when the view is placed, so drawing each
//! line is a lookup.

use crate::paper::{pen_stroke_with, Canvas, PaperMap, Stroke};
use crate::plan::{Hatch, Pen, PlanItem};
use creabim_core::styles::{self, HatchPattern, PatternUnits, Target};
use creabim_core::visibility::Category;
use creabim_core::{ElementId, ElementKind, LineWeights, Model, View, ViewKind};
use glam::DVec2;
use std::collections::{HashMap, HashSet};

/// The line a services drawing draws a pipe with, paper mm.
pub const PIPE_LINE: f64 = 0.35;
/// The line a services drawing draws a duct's outline with, paper mm.
pub const DUCT_LINE: f64 = 0.25;

/// The look of one element in one view, where it differs from the pens.
#[derive(Clone, Debug, Default, PartialEq)]
struct Look {
    projection: Option<f64>,
    cut: Option<f64>,
    colour: Option<[f32; 3]>,
    /// Its line pattern: none keeps the pen's, `Some(None)` is solid.
    dashes: Option<Option<Vec<f64>>>,
    /// How far toward white it is drawn, 0..1: halftone, transparency,
    /// distance.
    fade: f32,
    cut_fill: Option<[f32; 3]>,
    /// A pattern its category's style names for its cut faces.
    category_pattern: Option<String>,
    /// A pattern its material names, or its category's style for that
    /// material, and the hatch of that material.
    material_pattern: Option<(Hatch, String)>,
    hatch_off: bool,
    /// Every line of it in this one pen, mm, whatever its pen: the
    /// building and the other trades' objects under a trade's drawing.
    thin: Option<f64>,
    surface_fill: Option<[f32; 3]>,
    surface_pattern: Option<String>,
    order: i32,
}

fn mix(fade: &mut f32, by: f32) {
    *fade = 1.0 - (1.0 - *fade) * (1.0 - by.clamp(0.0, 1.0));
}

/// The pens and styles a drawing is drawn with.
#[derive(Clone, Debug)]
pub struct Looks {
    pub weights: LineWeights,
    by_id: HashMap<ElementId, Look>,
    patterns: HashMap<String, HatchPattern>,
    ordered: bool,
    /// Site areas whose surface a style fills or patterns: their boundary
    /// and holes, and the pattern drawn when the area has none of its own.
    areas: Vec<(ElementId, Vec<Vec<DVec2>>, Option<String>)>,
}

impl Looks {
    /// Only the pens: no styles, the built-in patterns.
    pub fn plain(weights: LineWeights) -> Self {
        Self {
            weights,
            by_id: HashMap::new(),
            patterns: HashMap::new(),
            ordered: false,
            areas: vec![],
        }
    }

    /// The looks of view `view` (`v`, as [`styles::resolved`] gives it)
    /// with the project's style pack.
    pub fn for_view(model: &Model, view: ElementId, v: &View) -> Self {
        let pack = styles::active_pack(model);
        let weights = styles::view_weights(model, v);
        let w = &weights;
        let cutters = creabim_core::visibility::cutters(model);
        let beyond = crate::ranges::beyond(model, v);
        let fades = distance_fades(model, v);
        let services = creabim_core::viewmodes::DisplayMode::of(v)
            == creabim_core::viewmodes::DisplayMode::Services;
        let mut by_id = HashMap::new();
        let mut areas = vec![];
        for e in model.iter() {
            if e.id == view {
                continue;
            }
            // a material's own pattern, for the patches of a legend of
            // materials, which draw in the material's id
            if let ElementKind::Material(m) = &e.kind {
                if let Some(p) = &m.pattern {
                    by_id.insert(
                        e.id,
                        Look {
                            material_pattern: Some((m.hatch, p.clone())),
                            ..Look::default()
                        },
                    );
                }
                continue;
            }
            let Some(cat) = Category::of(e, &cutters) else {
                continue;
            };
            let material = creabim_core::material_of(model, e);
            let mut l = Look::default();
            let apply = |l: &mut Look, s: &styles::ObjectStyle| {
                if let Some(x) = s.projection_weight {
                    l.projection = Some(x.mm(w));
                }
                if let Some(x) = s.cut_weight {
                    l.cut = Some(x.mm(w));
                }
                if let Some(c) = s.colour {
                    l.colour = Some(c.0);
                }
                if let Some(p) = &s.pattern {
                    l.dashes = Some(p.dashes());
                }
                if let Some(c) = s.cut_fill {
                    l.cut_fill = Some(c.0);
                }
                if let Some(c) = s.surface_fill {
                    l.surface_fill = Some(c.0);
                }
                if let Some(p) = &s.surface_pattern {
                    l.surface_pattern = Some(p.clone());
                }
                if let Some(o) = s.draw_order {
                    l.order = o;
                }
                if let Some(t) = s.transparency {
                    mix(&mut l.fade, t);
                }
            };
            if let Some(s) = pack.object_style(cat, None) {
                apply(&mut l, s);
                l.category_pattern = s.cut_pattern.clone();
            }
            if let Some((me, m)) = material {
                if let Some(s) = pack
                    .object_style(cat, Some(&me.name))
                    .filter(|s| s.material.is_some())
                {
                    apply(&mut l, s);
                    if let Some(p) = &s.cut_pattern {
                        l.material_pattern = Some((m.hatch, p.clone()));
                    }
                }
                if l.material_pattern.is_none() {
                    if let Some(p) = &m.pattern {
                        l.material_pattern = Some((m.hatch, p.clone()));
                    }
                }
            }
            // the view's overrides: its category's, then its own
            let ours = |o: &&styles::Override| match o.target {
                Target::Category(c) => c == cat,
                Target::Element(id) => id == e.id,
            };
            let mut overrides: Vec<&styles::Override> =
                v.look.overrides.iter().filter(ours).collect();
            overrides.sort_by_key(|o| matches!(o.target, Target::Element(_)));
            for o in overrides {
                if let Some(x) = o.weight {
                    l.projection = Some(x.mm(w));
                }
                if let Some(x) = o.cut_weight {
                    l.cut = Some(x.mm(w));
                }
                if let Some(c) = o.colour {
                    l.colour = Some(c.0);
                }
                if let Some(p) = &o.pattern {
                    l.dashes = Some(p.dashes());
                }
                if o.halftone {
                    mix(&mut l.fade, 0.5);
                }
                if let Some(t) = o.transparency {
                    mix(&mut l.fade, t);
                }
                if let Some(h) = o.hatch {
                    l.hatch_off = !h;
                }
            }
            // a coordination drawing tells the trades apart: each in one
            // colour, its pipes, what is on them and its devices
            if services && v.coordination && l.colour.is_none() {
                use creabim_core::viewmodes::Trade;
                let of = |p: &creabim_core::mep::Pipe| Trade::of_system(p.kind());
                let trade = match &e.kind {
                    ElementKind::Pipe(p) => Some(of(p)),
                    ElementKind::Valve(x) => x.host(model).map(of),
                    ElementKind::Object(_) if crate::elplan::is_device(model, e.id) => {
                        Some(Trade::Electrical)
                    }
                    _ => None,
                };
                l.colour = trade.map(Trade::colour);
            }
            // a trade's drawing: its own equipment in the medium pen, the
            // other objects (fixtures, furniture) very thin (George
            // 2026-11-22)
            if services && !v.coordination {
                if let ElementKind::Object(_) = &e.kind {
                    use creabim_core::viewmodes::{Trade, OWN_EQUIPMENT, THIN};
                    let own = match v.trade {
                        Some(t) => t.owns(model, e),
                        None => Trade::ALL.iter().any(|t| t.owns(model, e)),
                    };
                    if own {
                        l.projection = Some(OWN_EQUIPMENT);
                        l.fade = 0.0;
                    } else {
                        // in the building's very thin pen and its halftone,
                        // every line of it alike (George 2026-11-23)
                        l.projection = Some(THIN);
                        l.thin = Some(THIN);
                        if l.fade < 0.5 {
                            mix(&mut l.fade, 0.5);
                        }
                    }
                }
            }
            // the building under a trade's drawing: all its lines very
            // thin, door swings, lines over the cut and the drafted ones
            // too (George 2026-11-23)
            if services && !v.coordination && creabim_core::viewmodes::BUILDING.contains(&cat) {
                l.thin = Some(creabim_core::viewmodes::THIN);
            }
            // a services drawing's pipes: lines in their system's colour
            // and line type, unless the view says otherwise
            if let (true, ElementKind::Pipe(p)) = (services, &e.kind) {
                if l.colour.is_none() {
                    l.colour = Some(crate::mep::color(p));
                }
                // a duct is drawn in outline: told apart by its colour,
                // its lines solid
                if l.dashes.is_none() {
                    l.dashes = Some(
                        p.system_def()
                            .and_then(|s| s.dashes())
                            .filter(|_| !p.kind().is_air()),
                    );
                }
                if l.projection.is_none() {
                    l.projection = Some(if p.kind().is_air() {
                        DUCT_LINE
                    } else {
                        PIPE_LINE
                    });
                }
            }
            // a utility network: its kind's colour and line type, heavy
            // where proposed (PilonDesigner's site plans)
            // (its dashes are drawn as pieces, see crate::netlines)
            if let ElementKind::Sketch(creabim_core::Sketch { net: Some(n), .. }) = &e.kind {
                if l.colour.is_none() {
                    l.colour = Some(n.kind.rgb());
                }
                if l.dashes.is_none() {
                    l.dashes = Some(None);
                }
                if l.projection.is_none() {
                    l.projection = Some(n.weight());
                }
            }
            // a site plan's symbol: thin, in its network's colour
            if let ElementKind::Sketch(creabim_core::Sketch { mark: Some(m), .. }) = &e.kind {
                if l.colour.is_none() {
                    l.colour = Some(m.rgb());
                }
                if l.dashes.is_none() {
                    l.dashes = Some(None);
                }
                if l.projection.is_none() {
                    l.projection = Some(0.25);
                }
            }
            if beyond.contains(&e.id) {
                l.projection = Some(w.fine);
                mix(&mut l.fade, 0.45);
            }
            if let Some(f) = fades.get(&e.id) {
                mix(&mut l.fade, *f);
            }
            if let ElementKind::Site(creabim_core::site::Site {
                form:
                    creabim_core::site::SiteForm::Area {
                        boundary,
                        holes,
                        pattern,
                        ..
                    },
                ..
            }) = &e.kind
            {
                // an area's own pattern wins; else its style's or material's
                let named = (*pattern == creabim_core::site::Pattern::None)
                    .then(|| {
                        l.surface_pattern
                            .clone()
                            .or_else(|| l.material_pattern.as_ref().map(|(_, n)| n.clone()))
                    })
                    .flatten();
                if l.surface_fill.is_some() || named.is_some() {
                    let mut loops = vec![boundary.clone()];
                    loops.extend(holes.iter().cloned());
                    areas.push((e.id, loops, named));
                }
            }
            if l != Look::default() {
                by_id.insert(e.id, l);
            }
        }
        // what hangs on an element draws as it does, unless styled itself
        for e in model.iter() {
            let host = match &e.kind {
                ElementKind::Opening(o) if o.kind == creabim_core::OpeningKind::Hole => o.host,
                _ => continue,
            };
            if let (false, Some(l)) = (by_id.contains_key(&e.id), by_id.get(&host).cloned()) {
                by_id.insert(e.id, l);
            }
        }
        let patterns = pack
            .patterns
            .iter()
            .map(|p| (p.name.to_lowercase(), p.clone()))
            .collect();
        let ordered = by_id.values().any(|l| l.order != 0);
        Self {
            weights,
            by_id,
            patterns,
            ordered,
            areas,
        }
    }

    /// Whether any site area has a styled surface.
    pub fn has_surfaces(&self) -> bool {
        !self.areas.is_empty()
    }

    /// Draw the styled surfaces of the site areas in `drawn` (the elements
    /// a drawing shows), under everything else: the style's fill, and its
    /// pattern where the area has none of its own.
    pub fn surfaces(&self, canvas: &mut dyn Canvas, drawn: &HashSet<ElementId>, map: &PaperMap) {
        for (id, loops, named) in &self.areas {
            if !drawn.contains(id) || loops.is_empty() {
                continue;
            }
            let l = self.look(*id);
            let fade = l.map_or(0.0, |l| l.fade);
            let paper = |pts: &[DVec2]| pts.iter().map(|p| map.to_paper(*p)).collect::<Vec<_>>();
            if let Some(c) = l.and_then(|l| l.surface_fill) {
                canvas.fill_rgb(&paper(&loops[0]), c.map(|v| 1.0 - (1.0 - v) * (1.0 - fade)));
                for h in &loops[1..] {
                    canvas.fill_rgb(&paper(h), [1.0; 3]);
                }
            }
            let Some(p) = named.as_deref().and_then(|n| self.pattern(n)) else {
                continue;
            };
            let (s, c, _) = self.stroke(*id, Pen::Fine);
            let s = Stroke {
                width: self.weights.hatch,
                ..s
            };
            for (a, b) in pattern_lines(&p, &loops[0], map.k()) {
                for (a, b) in crate::parts::clip_line(loops, a, b) {
                    let seg = [map.to_paper(a), map.to_paper(b)];
                    match c {
                        Some(c) => canvas.line_rgb(&seg, s, c),
                        None => canvas.line(&seg, s),
                    }
                }
            }
        }
    }

    fn look(&self, id: ElementId) -> Option<&Look> {
        self.by_id.get(&id)
    }

    /// A pattern by name: the pack's, else a built-in one.
    pub fn pattern(&self, name: &str) -> Option<HatchPattern> {
        if let Some(p) = self.patterns.get(&name.to_lowercase()) {
            return Some(p.clone());
        }
        Hatch::ALL
            .iter()
            .find(|h| h.label().eq_ignore_ascii_case(name))
            .map(|h| styles::builtin_pattern(*h))
    }

    /// The items of a drawing in the order they draw: by their styles'
    /// draw order, lowest first, else as they come.
    pub fn order<'a>(&self, items: &'a [PlanItem]) -> Vec<&'a PlanItem> {
        let mut v: Vec<&PlanItem> = items.iter().collect();
        if self.ordered {
            v.sort_by_key(|it| self.look(it.id()).map_or(0, |l| l.order));
        }
        v
    }

    /// The stroke of a pen for element `id`, with its colour if it has one.
    pub fn stroke(&self, id: ElementId, pen: Pen) -> (Stroke, Option<[f32; 3]>, Option<Vec<f64>>) {
        let mut s = pen_stroke_with(pen, &self.weights);
        let overhead = match pen {
            Pen::Overhead => Some(crate::paper::OVERHEAD_DASHES.to_vec()),
            Pen::StairAbove { hidden } => Some(crate::paper::stair_above_dashes(hidden).to_vec()),
            // a dash-dot drafted line: longer than a dash and a gap
            Pen::Drafted(l) => l.kind.dashes().filter(|d| d.len() > 2).map(<[f64]>::to_vec),
            _ => None,
        };
        let Some(l) = self.look(id) else {
            return (s, None, overhead);
        };
        if let Some(x) = l.thin {
            s.width = x;
        }
        match pen {
            _ if l.thin.is_some() => {}
            Pen::Cut => {
                if let Some(x) = l.cut {
                    s.width = x;
                }
            }
            Pen::Projection | Pen::Symbol | Pen::Fine => {
                if let Some(x) = l.projection {
                    s.width = x;
                }
            }
            _ => {}
        }
        let mut long = overhead;
        if !matches!(pen, Pen::Hidden | Pen::Overhead | Pen::Drafted(_)) {
            match &l.dashes {
                Some(None) => s.dash = None,
                Some(Some(d)) if d.len() == 2 && d[0] > 0.0 => s.dash = Some([d[0], d[1]]),
                Some(Some(d)) => {
                    s.dash = None;
                    long = Some(d.clone());
                }
                None => {}
            }
        }
        let rgb = l
            .colour
            .map(|c| c.map(|v| 1.0 - (1.0 - v) * (1.0 - l.fade)));
        if rgb.is_none() {
            s.gray += (1.0 - s.gray) * l.fade as f64;
        }
        (s, rgb, long)
    }

    /// Draw a polyline (paper mm) of element `id` in `pen`.
    pub fn line(&self, canvas: &mut dyn Canvas, id: ElementId, pen: Pen, pts: &[DVec2]) {
        let (s, rgb, long) = self.stroke(id, pen);
        let mut draw = |pts: &[DVec2]| match rgb {
            Some(c) => canvas.line_rgb(pts, s, c),
            None => canvas.line(pts, s),
        };
        match long {
            Some(d) => {
                for piece in dash_polyline(pts, &d, s.width) {
                    draw(&piece);
                }
            }
            None => draw(pts),
        }
    }

    /// The pattern and background a cut fill of element `id` takes, whose
    /// drawing says `hatch`.
    pub fn cut_pattern(
        &self,
        id: ElementId,
        hatch: Hatch,
    ) -> (Option<[f32; 3]>, Option<HatchPattern>) {
        let builtin = || self.pattern(hatch.label());
        let Some(l) = self.look(id) else {
            let p = builtin();
            return (p.as_ref().and_then(|p| p.fill.map(|c| c.0)), p);
        };
        let named = |n: &str| self.pattern(n);
        let pattern = match &l.material_pattern {
            // a layer of another material keeps its own hatch
            Some((h, name)) if *h == hatch => named(name),
            _ => None,
        }
        .or_else(|| match &l.category_pattern {
            Some(name) if l.material_pattern.as_ref().is_none_or(|(h, _)| *h == hatch) => {
                named(name)
            }
            _ => None,
        })
        .or_else(builtin);
        let fill = l
            .cut_fill
            .or_else(|| pattern.as_ref().and_then(|p| p.fill.map(|c| c.0)));
        let fill = fill.map(|c| c.map(|v| 1.0 - (1.0 - v) * (1.0 - l.fade)));
        let pattern = if l.hatch_off { None } else { pattern };
        (fill, pattern)
    }

    /// Draw a cut fill: its background, then its pattern's lines.
    pub fn fill(
        &self,
        canvas: &mut dyn Canvas,
        id: ElementId,
        points: &[DVec2],
        hatch: Hatch,
        rgb: Option<[f32; 3]>,
        map: &PaperMap,
    ) {
        let (fill, pattern) = self.cut_pattern(id, hatch);
        let pts: Vec<DVec2> = points.iter().map(|p| map.to_paper(*p)).collect();
        // Colour and hatch: the material's colour under its hatch (MU32)
        canvas.fill_rgb(&pts, rgb.or(fill).unwrap_or([1.0; 3]));
        if let Some(p) = pattern {
            let fade = self.look(id).map_or(0.0, |l| l.fade);
            let s = Stroke {
                gray: 0.2 + 0.8 * fade as f64,
                ..Stroke::solid(self.weights.hatch, 0.2)
            };
            for (a, b) in pattern_lines(&p, points, map.k()) {
                canvas.line(&[map.to_paper(a), map.to_paper(b)], s);
            }
        }
    }

    /// Draw a surface seen in projection (a [`PlanItem::Shade`]): its
    /// style's fill and pattern, else `rgb`.
    pub fn shade(
        &self,
        canvas: &mut dyn Canvas,
        id: ElementId,
        points: &[DVec2],
        rgb: [f32; 3],
        map: &PaperMap,
    ) {
        let pts: Vec<DVec2> = points.iter().map(|p| map.to_paper(*p)).collect();
        let l = self.look(id);
        let fill = l.and_then(|l| l.surface_fill).unwrap_or(rgb);
        canvas.fill_rgb(&pts, fill);
        if let Some(p) = l
            .and_then(|l| l.surface_pattern.as_deref())
            .and_then(|n| self.pattern(n))
        {
            let (s, c, _) = self.stroke(id, Pen::Fine);
            let s = Stroke {
                width: self.weights.hatch,
                ..s
            };
            for (a, b) in pattern_lines(&p, points, map.k()) {
                let seg = [map.to_paper(a), map.to_paper(b)];
                match c {
                    Some(c) => canvas.line_rgb(&seg, s, c),
                    None => canvas.line(&seg, s),
                }
            }
        }
    }
}

/// How much lighter each element draws for its distance from the cut or
/// the viewer, in a view with distance fading.
fn distance_fades(model: &Model, v: &View) -> HashMap<ElementId, f32> {
    let mut out = HashMap::new();
    let fade = v.look.cues.fade;
    if fade <= 0.0 {
        return out;
    }
    // each element's nearest point, as a share of the view's depth
    let near: Box<dyn Fn(glam::DVec3, glam::DVec3) -> Option<f64>> = match &v.kind {
        ViewKind::Section {
            start, end, depth, ..
        } => {
            let frame = crate::section::SectionFrame::new(*start, *end, *depth);
            let depth = *depth;
            Box::new(move |lo, hi| {
                let d = corners(lo, hi)
                    .map(|c| frame.project(c).z)
                    .fold(f64::MAX, f64::min);
                Some((d.max(0.0) / depth.max(1e-6)).min(1.0))
            })
        }
        ViewKind::Plan { level, cut_height } if v.graphics.range.from_above => {
            let (top, bottom) = crate::ranges::from_above_span(model, v, *level, *cut_height);
            Box::new(move |_, hi| Some(((top - hi.z) / (top - bottom).max(1e-6)).clamp(0.0, 1.0)))
        }
        _ => return out,
    };
    for s in crate::scene::drawn_scene(model) {
        let Some((lo, hi)) = s.mesh.bounds() else {
            continue;
        };
        if let Some(t) = near(lo.as_dvec3(), hi.as_dvec3()) {
            let f = fade * t as f32;
            let e = out.entry(s.id).or_insert(f);
            *e = e.min(f);
        }
    }
    out
}

fn corners(lo: glam::DVec3, hi: glam::DVec3) -> impl Iterator<Item = glam::DVec3> {
    (0..8).map(move |i| {
        glam::DVec3::new(
            if i & 1 == 0 { lo.x } else { hi.x },
            if i & 2 == 0 { lo.y } else { hi.y },
            if i & 4 == 0 { lo.z } else { hi.z },
        )
    })
}

/// The most lines one pattern draws in one fill; finer than that it
/// would only be grey, so it is left as its background.
const MAX_PATTERN_LINES: usize = 20_000;

/// The lines of a pattern clipped to `polygon` (drawing units), with
/// `k` paper millimetres per drawing unit. Dots come back as very short
/// segments.
pub fn pattern_lines(p: &HatchPattern, polygon: &[DVec2], k: f64) -> Vec<(DVec2, DVec2)> {
    let mut out = Vec::new();
    if polygon.len() < 3 || k <= 0.0 {
        return out;
    }
    let unit = match p.units {
        PatternUnits::Paper => 1.0 / k,
        PatternUnits::Model => 1.0,
    };
    let dot = 0.15 / k;
    // a layer bent along its length (a roof's insulation cut across a
    // ridge or a hip, a wall's round a corner): a pattern that runs
    // along the layer runs along each straight piece of it
    let strips = strip_pieces(polygon);
    let pieces: Vec<(Vec<DVec2>, Option<DVec2>)> = match &strips {
        Some(s) => s.iter().map(|(q, d)| (q.clone(), Some(*d))).collect(),
        None => vec![(polygon.to_vec(), None)],
    };
    for l in &p.lines {
        let spacing = l.spacing * unit;
        let dashes: Vec<f64> = l.dashes.iter().map(|d| d * unit).collect();
        let whole = [(polygon.to_vec(), None)];
        let over: &[(Vec<DVec2>, Option<DVec2>)] = if l.along_longest { &pieces } else { &whole };
        for (piece, dir) in over {
            let mut angle = l.angle.to_radians();
            if l.along_longest {
                angle += dir
                    .or_else(|| longest_dir(piece))
                    .map_or(0.0, |d| d.to_angle());
            }
            if !family(
                piece,
                angle,
                spacing,
                l.offset * unit,
                l.shift * unit,
                &dashes,
                dot,
                &mut out,
            ) {
                return vec![];
            }
        }
    }
    for (piece, dir) in &pieces {
        if p.batt {
            out.extend(batt_lines(piece, *dir, unit));
        }
        if p.membrane {
            out.extend(membrane_lines(piece, *dir, unit));
        }
        if p.zigzag {
            out.extend(zigzag_lines(piece, *dir, unit));
        }
    }
    if let Some(t) = &p.tile {
        let (w, h) = (t.size[0] * unit, t.size[1] * unit);
        let (lo, hi) = bounds(polygon);
        let cells = ((hi.x - lo.x) / w).ceil() * ((hi.y - lo.y) / h).ceil();
        if cells.is_nan() || cells >= MAX_PATTERN_LINES as f64 {
            return out;
        }
        // one cell more below and to the left: a stroke may stand out of
        // its cell (a pebble over the tile's edge)
        let (i0, j0) = ((lo.x / w).floor() as i64 - 1, (lo.y / h).floor() as i64 - 1);
        let (i1, j1) = ((hi.x / w).ceil() as i64, (hi.y / h).ceil() as i64);
        for i in i0..i1 {
            for j in j0..j1 {
                let o = DVec2::new(i as f64 * w, j as f64 * h);
                for s in &t.strokes {
                    for pair in s.windows(2) {
                        let a = o + DVec2::new(pair[0][0], pair[0][1]) * unit;
                        let b = o + DVec2::new(pair[1][0], pair[1][1]) * unit;
                        for (u0, u1) in inside(a, b, polygon) {
                            out.push((a.lerp(b, u0), a.lerp(b, u1)));
                        }
                    }
                }
            }
        }
    }
    out
}

/// Extruded polystyrene's triangles along a fill's longest side: a
/// zigzag from one face of the layer to the other, as wide as the fill
/// is thick there; a thick fill takes several rows (George 2026-11-23).
/// `unit` is drawing units per paper mm.
fn zigzag_lines(polygon: &[DVec2], dir: Option<DVec2>, unit: f64) -> Vec<(DVec2, DVec2)> {
    let n = polygon.len();
    let Some(dir) = dir.or_else(|| longest_dir(polygon)) else {
        return vec![];
    };
    if dir == DVec2::ZERO {
        return vec![];
    }
    let nrm = dir.perp();
    let local: Vec<DVec2> = polygon
        .iter()
        .map(|p| DVec2::new(p.dot(dir), p.dot(nrm)))
        .collect();
    let (lo, hi) = bounds(&local);
    let longest = (0..n)
        .map(|i| (polygon[(i + 1) % n] - polygon[i]).length())
        .fold(0.0, f64::max)
        .max(hi.x - lo.x);
    let area = (0..n)
        .map(|i| local[i].perp_dot(local[(i + 1) % n]))
        .sum::<f64>()
        .abs()
        / 2.0;
    let band = (hi.y - lo.y).min(area / longest.max(1e-12));
    let t = band / (band / (25.0 * unit)).ceil().max(1.0);
    if t < 0.3 * unit || (hi.x - lo.x) / t > 4000.0 {
        return vec![];
    }
    // a triangle's base about as wide as the batt loops' pitch
    let p = (t * 0.6).clamp(0.6 * unit, 3.0 * unit);
    let at = |x: f64, y: f64| dir * x + nrm * y;
    let mut out = vec![];
    let mut push = |a: DVec2, b: DVec2| {
        for (u0, u1) in inside(a, b, polygon) {
            out.push((a.lerp(b, u0), a.lerp(b, u1)));
        }
    };
    let mut y0 = lo.y;
    while y0 < hi.y - t * 0.5 {
        let top = y0 + t;
        let mut k = ((lo.x - p) / p).floor() as i64;
        while (k as f64) * p < hi.x + p {
            let x = k as f64 * p;
            push(at(x, y0), at(x + p / 2.0, top));
            push(at(x + p / 2.0, top), at(x + p, y0));
            k += 1;
        }
        y0 = top;
    }
    out
}

/// Batt insulation loops along a fill's longest side, as wide as the fill
/// is thick there; a thick fill takes several rows. `unit` is drawing
/// units per paper mm.
fn batt_lines(polygon: &[DVec2], dir: Option<DVec2>, unit: f64) -> Vec<(DVec2, DVec2)> {
    let n = polygon.len();
    let Some(dir) = dir.or_else(|| longest_dir(polygon)) else {
        return vec![];
    };
    if dir == DVec2::ZERO {
        return vec![];
    }
    let nrm = dir.perp();
    let local: Vec<DVec2> = polygon
        .iter()
        .map(|p| DVec2::new(p.dot(dir), p.dot(nrm)))
        .collect();
    let (lo, hi) = bounds(&local);
    // along the run: its longest side, or the piece's length when the
    // run is given (a bent layer's straight piece, its sides cut up)
    let longest = (0..n)
        .map(|i| (polygon[(i + 1) % n] - polygon[i]).length())
        .fold(0.0, f64::max)
        .max(hi.x - lo.x);
    let area = (0..n)
        .map(|i| local[i].perp_dot(local[(i + 1) % n]))
        .sum::<f64>()
        .abs()
        / 2.0;
    // a band's thickness: across the fill, or its mean width for an L
    let band = (hi.y - lo.y).min(area / longest.max(1e-12));
    // a very thick fill (a whole roof's insulation in a patch) in rows of
    // at most 25 mm on paper
    let t = band / (band / (25.0 * unit)).ceil().max(1.0);
    if t < 0.3 * unit || (hi.x - lo.x) / t > 4000.0 {
        return vec![];
    }
    // loops about as far apart as the sample's (EPS 140, MW 160, 220 mm)
    let p = (t * 0.6).clamp(0.6 * unit, 3.0 * unit);
    let r = p / 2.0;
    let at = |x: f64, y: f64| dir * x + nrm * y;
    let mut out = vec![];
    let push = |a: DVec2, b: DVec2, out: &mut Vec<(DVec2, DVec2)>| {
        for (u0, u1) in inside(a, b, polygon) {
            out.push((a.lerp(b, u0), a.lerp(b, u1)));
        }
    };
    let mut y0 = lo.y;
    while y0 < hi.y - t * 0.5 {
        let top = y0 + t;
        let r = r.min(t / 2.0);
        let mut k = ((lo.x - p) / p).floor() as i64;
        while (k as f64) * p < hi.x + p {
            let x = k as f64 * p;
            // a half circle at the top round x, one at the bottom half a
            // pitch on, and the two straights between them, crossing
            let arc = |c: DVec2, from: f64| -> Vec<DVec2> {
                (0..=8)
                    .map(|i| {
                        let a = from + std::f64::consts::PI * i as f64 / 8.0;
                        c + DVec2::new(a.cos(), a.sin()) * r
                    })
                    .collect()
            };
            let ct = DVec2::new(x, top - r);
            let cb = DVec2::new(x + p / 2.0, y0 + r);
            for arcs in [arc(ct, 0.0), arc(cb, std::f64::consts::PI)] {
                for w in arcs.windows(2) {
                    push(at(w[0].x, w[0].y), at(w[1].x, w[1].y), &mut out);
                }
            }
            push(at(x + r, top - r), at(x + p / 2.0 - r, y0 + r), &mut out);
            push(
                at(x + p / 2.0 + r, y0 + r),
                at(x + p - r, top - r),
                &mut out,
            );
            k += 1;
        }
        y0 = top;
    }
    out
}

/// A waterproofing strip along a fill's middle, as the ASR legend draws an
/// asphalt strip: a thin line, and on it dashes 4 mm long and 0.45 mm (as thick as the sample's)
/// thick on paper (close lines, so screen and paper show them alike).
/// `unit` is drawing units per paper mm.
fn membrane_lines(polygon: &[DVec2], dir: Option<DVec2>, unit: f64) -> Vec<(DVec2, DVec2)> {
    let Some(dir) = dir.or_else(|| longest_dir(polygon)) else {
        return vec![];
    };
    let nrm = dir.perp();
    let local: Vec<DVec2> = polygon
        .iter()
        .map(|p| DVec2::new(p.dot(dir), p.dot(nrm)))
        .collect();
    let (lo, hi) = bounds(&local);
    let (dash, half) = (4.0 * unit, 0.225 * unit);
    if (hi.x - lo.x) / dash > 4000.0 {
        return vec![];
    }
    let mid = (lo.y + hi.y) / 2.0;
    let at = |x: f64, y: f64| dir * x + nrm * y;
    let mut out = vec![];
    // the strip's line along its middle, cut to the fill; the dashes
    // stand over it wider than a thin layer is on paper
    let (a, b) = (at(lo.x, mid), at(hi.x, mid));
    let runs: Vec<(f64, f64)> = inside(a, b, polygon)
        .into_iter()
        .map(|(u0, u1)| (lo.x + (hi.x - lo.x) * u0, lo.x + (hi.x - lo.x) * u1))
        .collect();
    for (x0, x1) in &runs {
        out.push((at(*x0, mid), at(*x1, mid)));
    }
    let mut x = (lo.x / (2.0 * dash)).floor() * 2.0 * dash;
    while x < hi.x {
        for (x0, x1) in &runs {
            let (d0, d1) = (x.max(*x0), (x + dash).min(*x1));
            if d1 <= d0 {
                continue;
            }
            for k in 0..10 {
                let y = mid - half + 2.0 * half * k as f64 / 9.0;
                out.push((at(d0, y), at(d1, y)));
            }
        }
        x += 2.0 * dash;
    }
    out
}

/// The direction of a polygon's longest side, None for a degenerate one.
fn longest_dir(polygon: &[DVec2]) -> Option<DVec2> {
    let n = polygon.len();
    (0..n)
        .map(|i| polygon[(i + 1) % n] - polygon[i])
        .max_by(|a, b| a.length().total_cmp(&b.length()))
        .map(|d| d.normalize_or_zero())
        .filter(|d| *d != DVec2::ZERO)
}

/// `poly` cut to the side of the line through `at` along `along` where
/// `keep` lies.
fn clip_half(poly: &[DVec2], at: DVec2, along: DVec2, keep: DVec2) -> Vec<DVec2> {
    let side = |p: DVec2| (p - at).perp_dot(along);
    let s = side(keep).signum();
    let n = poly.len();
    let mut out = vec![];
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let (sp, sq) = (side(p) * s, side(q) * s);
        if sp >= 0.0 {
            out.push(p);
        }
        if (sp >= 0.0) != (sq >= 0.0) {
            out.push(p.lerp(q, sp / (sp - sq)));
        }
    }
    out
}

/// A fill that is a layer bent along its length (a roof's insulation
/// cut across a ridge or hips, a wall's round a corner), in its straight
/// pieces, each with its run: the fill cut at each bend along the
/// bisector, so a pattern that follows the layer follows every piece.
/// None for a fill with no bend, which keeps its one run.
pub fn strip_pieces(polygon: &[DVec2]) -> Option<Vec<(Vec<DVec2>, DVec2)>> {
    let mut ring: Vec<DVec2> = vec![];
    for p in polygon {
        if ring.last().is_none_or(|q: &DVec2| q.distance(*p) > 1e-9) {
            ring.push(*p);
        }
    }
    if ring.len() > 1 && ring[0].distance(ring[ring.len() - 1]) <= 1e-9 {
        ring.pop();
    }
    let n = ring.len();
    if n < 5 {
        return None;
    }
    let area2: f64 = (0..n).map(|i| ring[i].perp_dot(ring[(i + 1) % n])).sum();
    let perimeter: f64 = (0..n).map(|i| ring[i].distance(ring[(i + 1) % n])).sum();
    if area2.abs() < 1e-12 || perimeter <= 0.0 {
        return None;
    }
    // the layer's thickness, were it a long strip
    let t = area2.abs() / perimeter;
    // straighten: corners that hardly stand out of their neighbours' line go
    let tol = 0.15 * t;
    loop {
        let m = ring.len();
        if m <= 4 {
            break;
        }
        let worst = (0..m)
            .map(|i| {
                let (a, p, b) = (ring[(i + m - 1) % m], ring[i], ring[(i + 1) % m]);
                let ab = b - a;
                let d = if ab.length() < 1e-12 {
                    p.distance(a)
                } else {
                    (p - a).perp_dot(ab).abs() / ab.length()
                };
                (i, d)
            })
            .min_by(|a, b| a.1.total_cmp(&b.1))?;
        if worst.1 >= tol {
            break;
        }
        ring.remove(worst.0);
    }
    let m = ring.len();
    if m <= 4 {
        return None;
    }
    // inward of a side: to its left round a counter-clockwise outline
    let ccw = area2 > 0.0;
    let edge = |i: usize| (ring[i % m], ring[(i + 1) % m]);
    let dir = |i: usize| {
        let (a, b) = edge(i);
        (b - a).normalize_or_zero()
    };
    let inward = |i: usize| {
        let d = dir(i);
        if ccw {
            d.perp()
        } else {
            -d.perp()
        }
    };
    let long: Vec<bool> = (0..m)
        .map(|i| edge(i).0.distance(edge(i).1) > 2.0 * t)
        .collect();
    let bent =
        (0..m).any(|i| long[i] && (0..m).any(|j| long[j] && dir(i).perp_dot(dir(j)).abs() > 0.09));
    if !bent {
        return None;
    }
    // a side whose opposite side (the layer's other face) comes first
    // is drawn with that one
    let partner = |i: usize, j: usize| {
        let (a, b) = edge(i);
        let (c, d) = edge(j);
        if dir(i).dot(dir(j)) > -0.98 {
            return false;
        }
        let gap = (c - a).dot(inward(i));
        let (u0, u1) = (0.0, (b - a).dot(dir(i)));
        let (v0, v1) = ((c - a).dot(dir(i)), (d - a).dot(dir(i)));
        gap > 0.3 * t && gap < 3.0 * t && v0.max(v1) > u0 + 1e-6 && v0.min(v1) < u1 - 1e-6
    };
    // the next long side round from `i`, `step` ±1, over short ones
    let next_long = |i: usize, back: bool| -> Option<usize> {
        let mut j = i;
        let mut run = 0.0;
        for _ in 0..4 {
            j = if back { (j + m - 1) % m } else { (j + 1) % m };
            if long[j] {
                return Some(j);
            }
            run += edge(j).0.distance(edge(j).1);
            if run > 2.0 * t {
                return None;
            }
        }
        None
    };
    let meet = |i: usize, j: usize| -> Option<DVec2> {
        let (a, _) = edge(i);
        let (c, _) = edge(j);
        let (d, e) = (dir(i), dir(j));
        let den = d.perp_dot(e);
        (den.abs() > 1e-9).then(|| a + d * ((c - a).perp_dot(e) / den))
    };
    let mut out = vec![];
    for i in 0..m {
        if !long[i] || (0..i).any(|j| long[j] && partner(i, j)) {
            continue;
        }
        let (a, b) = edge(i);
        let mid = (a + b) / 2.0 + inward(i) * (0.5 * t);
        let mut piece = polygon.to_vec();
        for back in [true, false] {
            let Some(j) = next_long(i, back) else {
                continue;
            };
            if dir(i).perp_dot(dir(j)).abs() <= 0.09 {
                continue;
            }
            let Some(x) = meet(i, j) else {
                continue;
            };
            let bis = (inward(i) + inward(j)).normalize_or_zero();
            if bis == DVec2::ZERO {
                continue;
            }
            piece = clip_half(&piece, x, bis, mid);
        }
        if piece.len() >= 3 {
            out.push((piece, dir(i)));
        }
    }
    (!out.is_empty()).then_some(out)
}

fn bounds(poly: &[DVec2]) -> (DVec2, DVec2) {
    poly.iter()
        .fold((DVec2::MAX, DVec2::MIN), |(a, b), p| (a.min(*p), b.max(*p)))
}

/// One family of parallel lines, `spacing` apart across them, into `out`.
/// False when it would be too many lines to draw.
#[allow(clippy::too_many_arguments)]
fn family(
    polygon: &[DVec2],
    angle: f64,
    spacing: f64,
    offset: f64,
    shift: f64,
    dashes: &[f64],
    dot: f64,
    out: &mut Vec<(DVec2, DVec2)>,
) -> bool {
    if spacing <= 0.0 {
        return true;
    }
    let (dir, nrm) = (DVec2::from_angle(angle), DVec2::from_angle(angle).perp());
    let local: Vec<DVec2> = polygon
        .iter()
        .map(|p| DVec2::new(p.dot(dir), p.dot(nrm) - offset))
        .collect();
    let (lo, hi) = bounds(&local);
    if (hi.y - lo.y) / spacing > MAX_PATTERN_LINES as f64 {
        return false;
    }
    let period: f64 = dashes.iter().sum();
    let mut k = (lo.y / spacing).ceil() as i64;
    while (k as f64) * spacing < hi.y {
        let y = k as f64 * spacing;
        let mut xs: Vec<f64> = Vec::new();
        for i in 0..local.len() {
            let (a, b) = (local[i], local[(i + 1) % local.len()]);
            if (a.y > y) != (b.y > y) {
                xs.push(a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x));
            }
        }
        xs.sort_by(|a, b| a.total_cmp(b));
        let at = |x: f64| dir * x + nrm * (y + offset);
        for pair in xs.chunks(2) {
            let [x0, x1] = pair else { continue };
            if dashes.is_empty() || period <= 0.0 {
                out.push((at(*x0), at(*x1)));
                continue;
            }
            if (x1 - x0) / period > MAX_PATTERN_LINES as f64 {
                return false;
            }
            // dashes start at x = shift * k, so rows stagger
            let phase = (k as f64 * shift).rem_euclid(period);
            let mut x = ((x0 - phase) / period).floor() * period + phase;
            while x < *x1 {
                let mut u = x;
                for (i, d) in dashes.iter().enumerate() {
                    if i % 2 == 0 {
                        let (a, b) = if *d <= 0.0 { (u, u + dot) } else { (u, u + d) };
                        let (a, b) = (a.max(*x0), b.min(*x1));
                        if b > a {
                            out.push((at(a), at(b)));
                        }
                    }
                    u += d;
                }
                x += period;
            }
        }
        k += 1;
        if out.len() > MAX_PATTERN_LINES * 4 {
            return false;
        }
    }
    true
}

/// The stretches of `a`→`b` inside a polygon (even-odd), as fractions.
fn inside(a: DVec2, b: DVec2, poly: &[DVec2]) -> Vec<(f64, f64)> {
    let d = b - a;
    let mut ts = vec![0.0, 1.0];
    let n = poly.len();
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        let e = q - p;
        let den = d.perp_dot(e);
        if den.abs() < 1e-12 {
            continue;
        }
        let t = (p - a).perp_dot(e) / den;
        let u = (p - a).perp_dot(d) / den;
        if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
            ts.push(t);
        }
    }
    ts.sort_by(|x, y| x.total_cmp(y));
    ts.windows(2)
        .filter(|w| w[1] - w[0] > 1e-9 && point_in(a.lerp(b, (w[0] + w[1]) / 2.0), poly))
        .map(|w| (w[0], w[1]))
        .collect()
}

fn point_in(p: DVec2, poly: &[DVec2]) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    inside
}

/// A polyline broken into the dashes of `dashes` (paper mm; a dash of 0
/// is a dot as long as the line is wide), the pattern running on round
/// corners.
pub fn dash_polyline(pts: &[DVec2], dashes: &[f64], width: f64) -> Vec<Vec<DVec2>> {
    let period: f64 = dashes.iter().sum();
    if pts.len() < 2 || period <= 0.0 {
        return vec![pts.to_vec()];
    }
    let mut out: Vec<Vec<DVec2>> = Vec::new();
    let (mut i, mut left) = (0usize, dashes[0].max(width));
    let mut on = true;
    let mut cur: Vec<DVec2> = vec![pts[0]];
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let len = a.distance(b);
        let mut t = 0.0;
        while len - t > left {
            t += left;
            let p = a.lerp(b, t / len);
            if on {
                cur.push(p);
                out.push(std::mem::take(&mut cur));
            } else {
                cur = vec![p];
            }
            on = !on;
            i = (i + 1) % dashes.len();
            left = if on { dashes[i].max(width) } else { dashes[i] };
            if left <= 0.0 {
                left = 1e-6;
            }
        }
        left -= len - t;
        if on {
            cur.push(b);
        }
    }
    if on && cur.len() > 1 {
        out.push(cur);
    }
    out
}

/// The ids of the elements a view draws with a style of their own.
pub fn styled_ids(looks: &Looks) -> HashSet<ElementId> {
    looks.by_id.keys().copied().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::styles::HatchLine;

    fn square(s: f64) -> Vec<DVec2> {
        vec![
            DVec2::ZERO,
            DVec2::new(s, 0.0),
            DVec2::new(s, s),
            DVec2::new(0.0, s),
        ]
    }

    #[test]
    fn model_patterns_keep_their_true_size_and_paper_ones_do_not() {
        let mut p = styles::builtin_pattern(Hatch::Diagonal);
        p.lines = vec![HatchLine {
            angle: 0.0,
            spacing: 0.1,
            offset: 0.0,
            shift: 0.0,
            dashes: vec![],
            along_longest: false,
        }];
        p.units = PatternUnits::Model;
        // ten courses a metre whatever the scale
        assert_eq!(pattern_lines(&p, &square(1.0), 10.0).len(), 10);
        assert_eq!(pattern_lines(&p, &square(1.0), 20.0).len(), 10);
        // on paper: 0.1 mm apart, so 100 a metre at 1:1000 (k = 1)
        p.units = PatternUnits::Paper;
        assert_eq!(pattern_lines(&p, &square(1.0), 1.0).len(), 10);
        assert_eq!(pattern_lines(&p, &square(1.0), 2.0).len(), 20);
        // far too fine to draw: nothing, the fill stays
        assert!(pattern_lines(&p, &square(100.0), 1000.0).is_empty());
    }

    #[test]
    fn dashes_and_dots_follow_the_pattern() {
        let mut p = styles::builtin_pattern(Hatch::Diagonal);
        p.units = PatternUnits::Model;
        p.lines = vec![HatchLine {
            angle: 0.0,
            spacing: 0.5,
            offset: 0.25,
            shift: 0.0,
            dashes: vec![0.1, 0.1],
            along_longest: false,
        }];
        let lines = pattern_lines(&p, &square(1.0), 10.0);
        // two rows, five dashes each
        assert_eq!(lines.len(), 10);
        assert!(lines
            .iter()
            .all(|(a, b)| (a.distance(*b) - 0.1).abs() < 1e-9));
        let pieces = dash_polyline(
            &[DVec2::ZERO, DVec2::new(10.0, 0.0)],
            &[6.0, 1.0, 1.0, 1.0],
            0.1,
        );
        assert_eq!(pieces.len(), 3);
        assert!((pieces[0][1].x - 6.0).abs() < 1e-9);
        assert!((pieces[1][0].x - 7.0).abs() < 1e-9);
    }

    #[test]
    fn tiles_repeat_and_clip() {
        let p = HatchPattern {
            name: "t".into(),
            units: PatternUnits::Model,
            fill: None,
            lines: vec![],
            tile: Some(styles::Tile {
                size: [0.5, 0.5],
                strokes: vec![vec![[0.0, 0.25], [0.5, 0.25]]],
            }),
            batt: false,
            zigzag: false,
            membrane: false,
        };
        let lines = pattern_lines(&p, &square(1.0), 10.0);
        assert_eq!(lines.len(), 4);
    }

    /// George: a roof's mineral wool cut across its hips ran one way over
    /// the whole bent layer, rows crossing; each straight piece takes its
    /// own run.
    #[test]
    fn a_bent_layer_is_hatched_piece_by_piece_along_each_run() {
        let s = 0.5_f64; // slope rise per run
        let t = 0.2;
        // up a slope, along a flat, down a slope: the outer face, then
        // the inner one back
        let poly = vec![
            DVec2::new(0.0, 0.0),
            DVec2::new(4.0, 4.0 * s),
            DVec2::new(8.0, 4.0 * s),
            DVec2::new(12.0, 0.0),
            DVec2::new(12.0, -t),
            DVec2::new(8.0, 4.0 * s - t),
            DVec2::new(4.0, 4.0 * s - t),
            DVec2::new(0.0, -t),
        ];
        let pieces = strip_pieces(&poly).unwrap();
        assert_eq!(pieces.len(), 3, "{pieces:?}");
        let flat = pieces.iter().filter(|(_, d)| d.y.abs() < 1e-9).count();
        assert_eq!(flat, 1);
        // the pieces fill the layer, once
        let area = |q: &[DVec2]| {
            (0..q.len())
                .map(|i| q[i].perp_dot(q[(i + 1) % q.len()]))
                .sum::<f64>()
                .abs()
                / 2.0
        };
        let sum: f64 = pieces.iter().map(|(q, _)| area(q)).sum();
        assert!((sum - area(&poly)).abs() < 1e-6, "{sum} {}", area(&poly));
        // a plain strip keeps its one run
        assert!(strip_pieces(&square(1.0)).is_none());
        let p = HatchPattern {
            name: "wool".into(),
            units: PatternUnits::Paper,
            fill: None,
            lines: vec![],
            tile: None,
            batt: true,
            zigzag: false,
            membrane: false,
        };
        assert!(!pattern_lines(&p, &poly, 20.0).is_empty());
    }
}
