//! How a view shows the building to one trade: the architect's drawing
//! as it is, a "blind" drawing (outlines only: no dimensions, tags, notes,
//! section marks or wall fills) to hand to the other professions, and a services
//! drawing, where the building is that blind drawing in halftone and the
//! pipes, ducts and fixtures stand out over it.
//!
//! A mode is a set of a view's own settings (its drawing style, hidden
//! categories and category overrides), so everything it sets can still
//! be changed by hand afterwards.

use crate::graphics::DrawingStyle;
use crate::styles::Target;
use crate::visibility::Category;
use crate::View;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DisplayMode {
    /// The architect's drawing: hatches, dimensions, tags.
    Architecture,
    /// Outlines only, nothing written on it.
    Blind,
    /// The blind drawing in halftone under the services.
    Services,
}

/// What is written on a drawing rather than built.
pub const ANNOTATION: [Category; 4] = [
    Category::Dimensions,
    Category::Tags,
    Category::Notes,
    Category::SectionMarks,
];

/// The building itself: what a services drawing puts in halftone.
pub const BUILDING: [Category; 15] = [
    Category::Walls,
    Category::Slabs,
    Category::Doors,
    Category::Windows,
    Category::Columns,
    Category::Beams,
    Category::Roofs,
    Category::Stairs,
    Category::Railings,
    Category::Footings,
    Category::Ceilings,
    Category::Parts,
    Category::FreeMeshes,
    Category::Site,
    Category::Planting,
];

/// The line a blind drawing draws the building with, mm.
pub const THIN: f64 = 0.18;

impl DisplayMode {
    pub const ALL: [DisplayMode; 3] = [
        DisplayMode::Architecture,
        DisplayMode::Blind,
        DisplayMode::Services,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DisplayMode::Architecture => "Architecture",
            DisplayMode::Blind => "Blind drawing",
            DisplayMode::Services => "Services over a blind drawing",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            DisplayMode::Architecture => {
                "Hatches, dimensions, tags and notes, as the architect draws it"
            }
            DisplayMode::Blind => {
                "Outlines only: no dimensions, tags, notes, room names or wall fills"
            }
            DisplayMode::Services => {
                "The building in halftone outlines, pipes, ducts and fixtures in full over it"
            }
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            DisplayMode::Architecture => "architecture",
            DisplayMode::Blind => "blind",
            DisplayMode::Services => "services",
        }
    }

    pub fn from_id(s: &str) -> Option<DisplayMode> {
        DisplayMode::ALL.into_iter().find(|m| m.id() == s)
    }

    /// The mode `view` is in, by what it shows.
    pub fn of(view: &View) -> DisplayMode {
        let blind = view.graphics.drawing_style == DrawingStyle::Clean
            && ANNOTATION.iter().all(|c| view.hidden.hides_category(*c));
        if !blind {
            return DisplayMode::Architecture;
        }
        let halftone = view
            .look
            .overrides
            .iter()
            .any(|o| o.target == Target::Category(Category::Walls) && o.halftone);
        if halftone {
            DisplayMode::Services
        } else {
            DisplayMode::Blind
        }
    }

    /// Put `view` in this mode.
    pub fn apply(self, view: &mut View) {
        let blind = self != DisplayMode::Architecture;
        view.graphics.drawing_style = if blind {
            DrawingStyle::Clean
        } else {
            DrawingStyle::Technical
        };
        view.graphics.below_fill = !blind;
        for c in ANNOTATION {
            view.hidden.set_category(c, blind);
        }
        // rooms stay to be picked; their names go
        view.room_tags = blind.then_some(false);
        if blind {
            view.room_colours = false;
        }
        // the building in one thin line, cut or seen; under services lighter
        for c in BUILDING {
            let o = view.look.override_mut(Target::Category(c));
            o.cut_weight = blind.then_some(crate::styles::Weight::Mm(THIN));
            o.halftone = self == DisplayMode::Services;
        }
        view.look.tidy();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ElementKind, ViewKind};

    fn plan() -> View {
        let m = crate::sample::sample_house();
        let v = m
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::View(v) if matches!(v.kind, ViewKind::Plan { .. }) => Some(v.clone()),
                _ => None,
            })
            .unwrap();
        v
    }

    #[test]
    fn a_blind_drawing_is_outlines_with_nothing_written_on_it() {
        let mut v = plan();
        assert_eq!(DisplayMode::of(&v), DisplayMode::Architecture);
        DisplayMode::Blind.apply(&mut v);
        assert_eq!(DisplayMode::of(&v), DisplayMode::Blind);
        assert_eq!(v.graphics.drawing_style, DrawingStyle::Clean);
        assert!(v.hidden.hides_category(Category::Dimensions));
        assert!(!v.hidden.hides_category(Category::Pipes));
        // walls in one thin line, not halftone
        let walls = v
            .look
            .overrides
            .iter()
            .find(|o| o.target == Target::Category(Category::Walls))
            .unwrap();
        assert!(walls.cut_weight.is_some() && !walls.halftone);
        assert_eq!(v.room_tags, Some(false));
    }

    #[test]
    fn services_stand_out_over_the_building_in_halftone_and_it_comes_back() {
        let mut v = plan();
        let was = v.clone();
        DisplayMode::Services.apply(&mut v);
        assert_eq!(DisplayMode::of(&v), DisplayMode::Services);
        let half = |c: Category| {
            v.look
                .overrides
                .iter()
                .any(|o| o.target == Target::Category(c) && o.halftone)
        };
        assert!(half(Category::Walls) && half(Category::Stairs));
        assert!(!half(Category::Pipes) && !half(Category::Objects));
        DisplayMode::Architecture.apply(&mut v);
        assert_eq!(v, was);
    }
}

/// The trade a services drawing is for: it shows that trade's pipes and
/// equipment only (ZTI and VYK plans of one storey differ by it).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Trade {
    /// Water, drainage, fire water and gas (ZTI, PLY).
    Plumbing,
    /// Heating and cooling (VYK).
    Heating,
    /// Ventilation (VZT).
    Ventilation,
    /// Medical gases.
    MedGas,
    /// Wiring: sockets, lights, boards (ELI).
    Electrical,
}

impl Trade {
    pub const ALL: [Trade; 5] = [
        Trade::Plumbing,
        Trade::Heating,
        Trade::Ventilation,
        Trade::MedGas,
        Trade::Electrical,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Trade::Plumbing => "Plumbing and gas (ZTI)",
            Trade::Heating => "Heating and cooling (VYK)",
            Trade::Ventilation => "Ventilation (VZT)",
            Trade::MedGas => "Medical gases",
            Trade::Electrical => "Electrical (ELI)",
        }
    }

    /// Does its drawing show pipes of the system `kind`?
    pub fn shows(self, kind: crate::mep::SystemKind) -> bool {
        use crate::mep::SystemKind as K;
        match kind {
            K::ColdWater
            | K::HotWater
            | K::Circulation
            | K::Sanitary
            | K::Storm
            | K::FireWater
            | K::Gas => self == Trade::Plumbing,
            K::HeatingSupply | K::HeatingReturn | K::CoolingSupply | K::CoolingReturn | K::Flue => {
                self == Trade::Heating
            }
            K::AirSupply | K::AirExhaust => self == Trade::Ventilation,
            K::MedGas => self == Trade::MedGas,
            K::Other => true,
        }
    }

    /// Does its drawing show the element `id`: a pipe or what is on one by
    /// its system; anything else is the building all trades draw over.
    pub fn shows_element(self, model: &crate::Model, id: crate::ElementId) -> bool {
        use crate::ElementKind;
        let pipe = |id: crate::ElementId| match model.get(id).map(|e| &e.kind) {
            Some(ElementKind::Pipe(p)) => Some(self.shows(p.kind())),
            _ => None,
        };
        match model.get(id).map(|e| &e.kind) {
            Some(ElementKind::Pipe(p)) => self.shows(p.kind()),
            Some(ElementKind::Valve(v)) => pipe(v.pipe).unwrap_or(true),
            _ => true,
        }
    }
}
