//! How a plan view draws on paper: its pens' line weights, the tinted
//! footprints of what lies below the cut, and how far below its floor it
//! looks (its view depth).

use crate::visibility::Category;
use serde::{Deserialize, Serialize};

/// The thinnest pen of the standard pens, mm: hatching and fill
/// patterns are drawn with it (George 2026-11-22).
pub const THINNEST_PEN: f64 = 0.09;

/// Pen widths on paper, in millimetres, from heavy to light.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LineWeights {
    /// Material cut by the plan plane: walls, columns, footings.
    pub cut: f64,
    /// Seen below the cut, or beyond it: slab edges, the stair, low walls.
    pub projection: f64,
    /// Symbols: door leaves and swings, window frames, the walking line.
    pub symbol: f64,
    /// Small things: furniture, fixtures, sanitary ware.
    pub fine: f64,
    /// Hatching inside cut fills.
    pub hatch: f64,
    /// Dashed lines for what is above the cut or hidden.
    pub hidden: f64,
}

impl LineWeights {
    /// The pens of a drawing at 1:`scale`: the same at every scale, so
    /// a 1:50 section prints as fine as a 1:100 plan (George 2026-11-22:
    /// sections on sheets drew heavier than the view), and hatching in
    /// the thinnest of them.
    pub fn for_scale(_scale: f64) -> Self {
        Self {
            cut: 0.50,
            projection: 0.25,
            symbol: 0.18,
            fine: 0.13,
            hatch: THINNEST_PEN,
            hidden: 0.13,
        }
    }

    /// Every weight, with its name, for editing.
    pub fn fields_mut(&mut self) -> [(&'static str, &mut f64); 6] {
        [
            ("Cut", &mut self.cut),
            ("Projection", &mut self.projection),
            ("Symbols", &mut self.symbol),
            ("Furniture and fixtures", &mut self.fine),
            ("Hatching", &mut self.hatch),
            ("Hidden (dashed)", &mut self.hidden),
        ]
    }
}

/// The pens of [`LineWeights`], for naming one as data.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PenClass {
    Projection,
    Symbol,
    Fine,
}

impl PenClass {
    pub const ALL: [PenClass; 3] = [PenClass::Projection, PenClass::Symbol, PenClass::Fine];

    pub fn label(self) -> &'static str {
        match self {
            PenClass::Projection => "Projection",
            PenClass::Symbol => "Symbols",
            PenClass::Fine => "Fine",
        }
    }
}

/// Which pen a category's lines seen below the cut are drawn with (its
/// cut lines always take the cut pen, and what is overhead the hidden one).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CategoryPen {
    pub category: Category,
    pub pen: PenClass,
}

/// The categories drawn with a pen of their own unless a view says
/// otherwise: furniture and fixtures in the fine pen.
pub fn default_category_pens() -> Vec<CategoryPen> {
    vec![CategoryPen {
        category: Category::Objects,
        pen: PenClass::Fine,
    }]
}

fn is_default_category_pens(v: &[CategoryPen]) -> bool {
    v == default_category_pens().as_slice()
}

/// How far a plan looks, about its cut plane (which stays the plan's
/// `cut_height`): its view range. With none of `top`, `bottom` and
/// `beyond` set it draws its own level only, as plans always did; with
/// any of them it draws every level's elements within the range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PlanRange {
    /// How far below its level's floor the plan looks, in metres: what
    /// lies between the floor and the cut is *below* it, drawn in
    /// projection; what lies under the floor, down to this depth and seen
    /// through a hole in it (a stair from the level below, in its well), is
    /// *beyond*. None looks down to the level below.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view_depth: Option<f64>,
    /// The top of the range: levels starting above it are left out. None
    /// is the next level up.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<RangeLimit>,
    /// The bottom of the range: levels from here up to the cut are drawn,
    /// cut or in projection. None is the plan's own level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bottom: Option<RangeLimit>,
    /// The view depth as a plane: levels between it and the bottom are
    /// drawn in the lighter *beyond* style. None draws nothing beyond.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beyond: Option<RangeLimit>,
    /// Draw what is above the cut, up to the top, dashed: roof edges,
    /// beams, the floor over a void.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub overhead: bool,
    /// Look down from the top of the range with nothing cut, upper slabs
    /// and roofs hiding what is under them: a roof or site plan.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub from_above: bool,
    /// Seen from above, the building as a site plan shows it: only its
    /// outer walls and the columns outside, cut and filled dark grey,
    /// without plaster, openings, inner walls, pipes or view marks.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub footprint: bool,
}

/// A height given by a level and an offset from it, so it moves with the
/// level. A level that is gone counts as the plan's own.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RangeLimit {
    pub level: crate::ElementId,
    #[serde(default)]
    pub offset: f64,
}

impl RangeLimit {
    /// The height it stands for, with `own` the plan's level elevation.
    pub fn z(&self, model: &crate::Model, own: f64) -> f64 {
        model.level(self.level).map_or(own, |l| l.elevation) + self.offset
    }
}

impl PlanRange {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// A site plan: seen from above with the building as its footprint.
    /// A roof plan is seen from above too, but shows no map, terrain,
    /// networks or site sketches (George).
    pub fn is_site(&self) -> bool {
        self.from_above && self.footprint
    }

    /// Does it reach past its own level (or look from above)?
    pub fn spans_levels(&self) -> bool {
        self.top.is_some()
            || self.bottom.is_some()
            || self.beyond.is_some()
            || self.overhead
            || self.from_above
    }
}

/// A plan's graphics. Every field has a default, so views saved before
/// it read as they drew then, with the weights tuned to their scale.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlanGraphics {
    /// Pen weights of the user's own; none takes them from the scale.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub weights: Option<LineWeights>,
    /// The pens categories draw with, by category: a table, so an office
    /// standard can set them.
    #[serde(
        default = "default_category_pens",
        skip_serializing_if = "is_default_category_pens"
    )]
    pub category_pens: Vec<CategoryPen>,
    /// Fill what lies wholly below the cut (furniture, fittings, imported
    /// meshes) with a light tint of its material's colour.
    #[serde(default = "yes")]
    pub below_fill: bool,
    /// How strong that tint is, 0 (none) to 1 (the material's colour).
    #[serde(default = "default_strength")]
    pub below_strength: f32,
    /// Fill it in light grey rather than in its material's colour.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub below_grey: bool,
    /// How far it looks above and below the cut.
    #[serde(default, skip_serializing_if = "PlanRange::is_default")]
    pub range: PlanRange,
    /// How what is cut is drawn (A4-11).
    #[serde(default, skip_serializing_if = "DrawingStyle::is_default")]
    pub drawing_style: DrawingStyle,
    /// What lies behind the hatch of what is cut, in a drawing that is not
    /// a renovation stage (those keep their own colours).
    #[serde(default, skip_serializing_if = "HatchGround::is_default")]
    pub hatch_ground: HatchGround,
}

/// The background under a cut hatch (walls, slabs): very light grey by
/// default, the paper's colour, or the darker grey drawings had before.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HatchGround {
    /// A very light grey, the hatch reading clearly over it.
    #[default]
    Light,
    /// The paper's (background's) colour: no fill under the hatch.
    Paper,
    /// The grey of the hatch pattern, or the graphic styles' own fill.
    Grey,
}

impl HatchGround {
    pub const ALL: [HatchGround; 3] = [HatchGround::Light, HatchGround::Paper, HatchGround::Grey];

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn label(self) -> &'static str {
        match self {
            HatchGround::Light => "Very light grey",
            HatchGround::Paper => "Paper colour",
            HatchGround::Grey => "Grey (pattern's own)",
        }
    }

    /// The fill under a hatch in this ground, none leaving the pattern's
    /// own (or the graphic styles').
    pub fn rgb(self) -> Option<[f32; 3]> {
        match self {
            HatchGround::Light => Some([0.965; 3]),
            HatchGround::Paper => Some([1.0; 3]),
            HatchGround::Grey => None,
        }
    }
}

/// How a view draws what it cuts (A4-11): a set of overrides over its
/// graphic styles, the same on screen, on sheets and in PDFs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DrawingStyle {
    /// Hatches by material, as the graphic styles set them.
    #[default]
    Technical,
    /// What is cut in solid ink, no hatch.
    Poche,
    /// What is cut in its material's colour.
    MaterialColour,
    /// Outlines only: no hatches, no fills.
    Clean,
    /// What is cut in a soft wash of its material's colour (MU21).
    ColourWash,
    /// What is cut in an even light grey, no hatch (MU21).
    GreyWash,
    /// Each cut material in its own colour with its hatch over it (MU32).
    ColourHatch,
}

impl DrawingStyle {
    pub const ALL: [DrawingStyle; 7] = [
        DrawingStyle::Technical,
        DrawingStyle::Poche,
        DrawingStyle::ColourHatch,
        DrawingStyle::MaterialColour,
        DrawingStyle::ColourWash,
        DrawingStyle::GreyWash,
        DrawingStyle::Clean,
    ];

    pub fn is_default(&self) -> bool {
        *self == Self::Technical
    }

    pub fn label(self) -> &'static str {
        match self {
            DrawingStyle::Technical => "Technical",
            DrawingStyle::Poche => "Poché",
            DrawingStyle::MaterialColour => "Material colour",
            DrawingStyle::Clean => "Clean",
            DrawingStyle::ColourWash => "Colour wash",
            DrawingStyle::GreyWash => "Grey wash",
            DrawingStyle::ColourHatch => "Colour and hatch",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            DrawingStyle::Technical => "Cut materials hatched, as the graphic styles set them",
            DrawingStyle::Poche => "Cut walls and slabs solid ink, no hatch",
            DrawingStyle::MaterialColour => "Cut materials in their own colours, layer by layer, no hatch",
            DrawingStyle::Clean => "Outlines only: no hatches or fills",
            DrawingStyle::ColourWash => {
                "Cut materials in a soft wash of their colours, for presentation"
            }
            DrawingStyle::GreyWash => "Cut materials in an even light grey, no hatch",
            DrawingStyle::ColourHatch => {
                "Each cut material in its own colour, layer by layer, with its hatch over it: brick red, insulation yellow"
            }
        }
    }
}

fn yes() -> bool {
    true
}

fn default_strength() -> f32 {
    0.25
}

impl Default for PlanGraphics {
    fn default() -> Self {
        Self {
            weights: None,
            category_pens: default_category_pens(),
            below_fill: true,
            below_strength: default_strength(),
            below_grey: false,
            range: PlanRange::default(),
            drawing_style: DrawingStyle::default(),
            hatch_ground: HatchGround::default(),
        }
    }
}

impl PlanGraphics {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// The pen a category's lines seen below the cut take, if it has one
    /// of its own.
    pub fn pen_of(&self, category: Category) -> Option<PenClass> {
        self.category_pens
            .iter()
            .find(|c| c.category == category)
            .map(|c| c.pen)
    }

    /// The pens a view at 1:`scale` draws with.
    pub fn weights_at(&self, scale: f64) -> LineWeights {
        self.weights
            .unwrap_or_else(|| LineWeights::for_scale(scale))
    }

    /// The tint of a material colour (sRGB, 0..1) for what is below the
    /// cut, mixed toward white; none when the fill is off.
    pub fn below_tint(&self, rgb: [f32; 3]) -> Option<[f32; 3]> {
        if !self.below_fill || self.below_strength <= 0.0 {
            return None;
        }
        let rgb = if self.below_grey { [0.55; 3] } else { rgb };
        let k = self.below_strength.clamp(0.0, 1.0);
        Some(rgb.map(|c| 1.0 - (1.0 - c) * k))
    }

    /// How far below the floor of a level at `elevation` the plan looks,
    /// with `below` the elevation of the next level down, if any.
    pub fn depth(&self, elevation: f64, below: Option<f64>) -> f64 {
        self.range
            .view_depth
            .unwrap_or_else(|| below.map_or(3.0, |b| elevation - b))
            .max(0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weights_step_down_from_cut_to_hatch() {
        for s in [50.0, 100.0] {
            let w = LineWeights::for_scale(s);
            assert!(w.cut > w.projection && w.projection > w.symbol);
            assert!(w.symbol > w.fine && w.fine > w.hatch);
            assert!(w.hidden <= w.symbol);
        }
        // a section at 1:50 prints with the pens of a plan at 1:100
        assert_eq!(LineWeights::for_scale(50.0), LineWeights::for_scale(100.0));
        assert_eq!(LineWeights::for_scale(50.0).hatch, THINNEST_PEN);
    }

    #[test]
    fn an_old_view_reads_with_the_default_graphics() {
        let g: PlanGraphics = serde_json::from_str("{}").unwrap();
        assert_eq!(g, PlanGraphics::default());
        assert!(g.below_fill);
        let tint = g.below_tint([0.0, 0.5, 1.0]).unwrap();
        assert!(tint.iter().all(|c| *c >= 0.74));
        // a plan saved before views had graphics
        let old = r#"{"kind":{"Plan":{"level":"00000000-0000-0000-0000-000000000001","cut_height":1.2}},"scale":50.0}"#;
        let v: crate::View = serde_json::from_str(old).unwrap();
        assert!(v.graphics.is_default());
        assert_eq!(v.graphics.weights_at(v.scale), LineWeights::for_scale(50.0));
        // the defaults write nothing; a change is kept
        assert!(!serde_json::to_string(&v).unwrap().contains("graphics"));
        let mut changed = v.clone();
        changed.graphics.range.view_depth = Some(1.0);
        changed.graphics.below_fill = false;
        let back: crate::View =
            serde_json::from_str(&serde_json::to_string(&changed).unwrap()).unwrap();
        assert_eq!(back.graphics, changed.graphics);
    }
}
