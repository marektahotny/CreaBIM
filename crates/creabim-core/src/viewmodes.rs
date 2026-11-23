//! How a view shows the building to one trade: the architect's drawing
//! as it is, a "blind" drawing (outlines only: no dimensions, tags, notes,
//! section marks or wall fills) to hand to the other professions, and a services
//! drawing, where the building is that blind drawing in halftone and the
//! pipes, ducts and fixtures stand out over it. Pipes, ducts and wiring
//! show in the services drawing only.
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

/// The line a blind drawing draws the building with, mm: the very thin
/// pen (George 2026-11-22: in a trade's drawing walls, windows and
/// fixtures very thin, in their architecture line types).
pub const THIN: f64 = 0.10;

/// The line a trade's drawing draws its own equipment with, mm: the
/// medium pen (LinePen::Thick).
pub const OWN_EQUIPMENT: f64 = 0.20;

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
        if view.graphics.drawing_style != DrawingStyle::Clean {
            return DisplayMode::Architecture;
        }
        let halftone = view
            .look
            .overrides
            .iter()
            .any(|o| o.target == Target::Category(Category::Walls) && o.halftone);
        if halftone {
            DisplayMode::Services
        } else if ANNOTATION.iter().all(|c| view.hidden.hides_category(*c)) {
            DisplayMode::Blind
        } else {
            DisplayMode::Architecture
        }
    }

    /// A services drawing kept its dimensions, tags and notes hidden
    /// before the trades dimensioned their own drawings: shows them
    /// again. True when `view` changed.
    pub fn show_own_annotation(view: &mut View) -> bool {
        let own = [Category::Dimensions, Category::Tags, Category::Notes];
        if DisplayMode::of(view) != DisplayMode::Services
            || !own.iter().all(|c| view.hidden.hides_category(*c))
        {
            return false;
        }
        for c in own {
            view.hidden.set_category(c, false);
        }
        true
    }

    /// A services drawing named for a trade ("Pôdorys 1. NP ZTI") from
    /// before its trade was set: that trade's. True when `view` changed.
    pub fn trade_by_name(view: &mut View, name: &str) -> bool {
        if view.trade.is_some()
            || view.coordination
            || DisplayMode::of(view) != DisplayMode::Services
        {
            return false;
        }
        let t = match name.rsplit(' ').next() {
            Some("ZTI") => Trade::Plumbing,
            Some("VYK") => Trade::Heating,
            Some("VZT") => Trade::Ventilation,
            Some("ELI") => Trade::Electrical,
            Some("MPL") => Trade::MedGas,
            _ => return false,
        };
        view.trade = Some(t);
        true
    }

    /// A blind or services drawing from before its building was drawn in
    /// the very thin pen (cut and seen, George 2026-11-22): brought to it.
    /// True when `view` changed.
    pub fn thin_building(view: &mut View) -> bool {
        if DisplayMode::of(view) == DisplayMode::Architecture {
            return false;
        }
        let thin = Some(crate::styles::Weight::Mm(THIN));
        let mut changed = false;
        for c in BUILDING {
            let o = view.look.override_mut(Target::Category(c));
            if o.cut_weight != thin || o.weight != thin {
                o.cut_weight = thin;
                o.weight = thin;
                changed = true;
            }
        }
        view.look.tidy();
        // a trade's drawing numbers its rooms
        if DisplayMode::of(view) == DisplayMode::Services && view.room_tags == Some(false) {
            view.room_tags = None;
            changed = true;
        }
        changed
    }

    /// Put `view` in this mode.
    pub fn apply(self, view: &mut View) {
        let blind = self != DisplayMode::Architecture;
        view.coordination = false;
        view.graphics.drawing_style = if blind {
            DrawingStyle::Clean
        } else {
            DrawingStyle::Technical
        };
        view.graphics.below_fill = !blind;
        // what is written on a drawing belongs to its view: the trade's
        // drawing shows its own dimensions, tags and notes, never the
        // architect's
        for c in ANNOTATION {
            let hide = match self {
                DisplayMode::Architecture => false,
                DisplayMode::Blind => true,
                DisplayMode::Services => c == Category::SectionMarks,
            };
            view.hidden.set_category(c, hide);
        }
        // pipes, ducts and wiring are the trades' to draw
        view.hidden
            .set_category(Category::Pipes, self != DisplayMode::Services);
        // rooms stay to be picked; their names go (a trade's drawing
        // keeps their numbers, George 2026-11-22)
        view.room_tags = (self == DisplayMode::Blind).then_some(false);
        if blind {
            view.room_colours = false;
        }
        // the building in one very thin line, cut or seen; under services
        // lighter
        for c in BUILDING {
            let o = view.look.override_mut(Target::Category(c));
            o.cut_weight = blind.then_some(crate::styles::Weight::Mm(THIN));
            o.weight = blind.then_some(crate::styles::Weight::Mm(THIN));
            o.halftone = self == DisplayMode::Services;
        }
        view.look.tidy();
    }
}

/// What a drawing is for, as its view's settings pick it: the
/// architect's, a blind one, one trade's, all the trades', or the
/// coordination of them all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    Architecture,
    Blind,
    Trade(Trade),
    /// Every trade's pipes, each system in its own colour.
    Services,
    /// Everything, each trade in one colour of its own.
    Coordination,
}

impl Purpose {
    pub fn all() -> Vec<Purpose> {
        let mut v = vec![Purpose::Architecture, Purpose::Blind];
        v.extend(Trade::ALL.map(Purpose::Trade));
        v.extend([Purpose::Services, Purpose::Coordination]);
        v
    }

    pub fn label(self) -> &'static str {
        match self {
            Purpose::Architecture => "Architecture",
            Purpose::Blind => "Blind drawing",
            Purpose::Trade(t) => t.label(),
            Purpose::Services => "All trades",
            Purpose::Coordination => "Coordination",
        }
    }

    pub fn of(view: &View) -> Purpose {
        match (DisplayMode::of(view), view.trade) {
            (DisplayMode::Architecture, _) => Purpose::Architecture,
            (DisplayMode::Blind, _) => Purpose::Blind,
            (DisplayMode::Services, _) if view.coordination => Purpose::Coordination,
            (DisplayMode::Services, Some(t)) => Purpose::Trade(t),
            (DisplayMode::Services, None) => Purpose::Services,
        }
    }

    pub fn apply(self, view: &mut View) {
        match self {
            Purpose::Architecture => DisplayMode::Architecture.apply(view),
            Purpose::Blind => DisplayMode::Blind.apply(view),
            _ => DisplayMode::Services.apply(view),
        }
        view.trade = match self {
            Purpose::Trade(t) => Some(t),
            _ => None,
        };
        view.coordination = self == Purpose::Coordination;
    }
}

/// The room tags a trade's floor plans lack: each room tagged in the
/// architecture plan of its storey is tagged in every trade's plan of
/// that storey as there, at the same place and with the same fields
/// (George 2026-11-23: a trade's drawing keeps the rooms and their
/// tags as the architecture shows them). A room the architecture plan
/// leaves untagged stays so.
pub fn trade_room_tags(model: &crate::Model) -> Vec<crate::Element> {
    use crate::{ElementKind, ViewKind};
    let plan_of = |v: &View| match v.kind {
        ViewKind::Plan { level, .. } if v.callout.is_none() && !v.graphics.range.from_above => {
            Some(level)
        }
        _ => None,
    };
    let room =
        |id: crate::ElementId| matches!(model.get(id).map(|e| &e.kind), Some(ElementKind::Room(_)));
    // the tags of rooms by view
    let mut tags: std::collections::HashMap<crate::ElementId, Vec<&crate::Tag>> =
        std::collections::HashMap::new();
    for e in model.iter() {
        if let ElementKind::Tag(t) = &e.kind {
            if room(t.host) {
                tags.entry(t.view).or_default().push(t);
            }
        }
    }
    let mut out = vec![];
    for e in model.iter() {
        let ElementKind::View(v) = &e.kind else {
            continue;
        };
        let Some(level) = plan_of(v).filter(|_| DisplayMode::of(v) == DisplayMode::Services) else {
            continue;
        };
        // the storey's architecture plan with the most rooms tagged
        let source = model
            .iter()
            .filter(|a| match &a.kind {
                ElementKind::View(av) => {
                    plan_of(av) == Some(level) && DisplayMode::of(av) == DisplayMode::Architecture
                }
                _ => false,
            })
            .max_by_key(|a| tags.get(&a.id).map_or(0, Vec::len));
        let Some(source) = source.and_then(|a| tags.get(&a.id)) else {
            continue;
        };
        let have: std::collections::HashSet<crate::ElementId> = tags
            .get(&e.id)
            .map(|ts| ts.iter().map(|t| t.host).collect())
            .unwrap_or_default();
        let mut done = std::collections::HashSet::new();
        for t in source {
            if have.contains(&t.host) || !done.insert(t.host) {
                continue;
            }
            let mut copy = (*t).clone();
            copy.view = e.id;
            out.push(crate::Element::new("Room tag", ElementKind::Tag(copy)));
        }
    }
    out
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
        assert!(v.hidden.hides_category(Category::Pipes));
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
        // the architect's drawing leaves the trades' pipes out
        DisplayMode::Architecture.apply(&mut v);
        assert!(v.hidden.hides_category(Category::Pipes));
        let was = v.clone();
        DisplayMode::Services.apply(&mut v);
        assert_eq!(DisplayMode::of(&v), DisplayMode::Services);
        assert!(!v.hidden.hides_category(Category::Pipes));
        // the trade dimensions its own drawing
        assert!(!v.hidden.hides_category(Category::Dimensions));
        assert!(v.hidden.hides_category(Category::SectionMarks));
        // one from before, with all of it hidden, shows it again once
        let mut old = v.clone();
        for c in ANNOTATION {
            old.hidden.set_category(c, true);
        }
        assert!(DisplayMode::show_own_annotation(&mut old));
        assert!(!DisplayMode::show_own_annotation(&mut old));
        assert_eq!(old, v);
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

    #[test]
    fn a_drawing_is_for_the_architect_a_trade_or_the_coordination() {
        let mut v = plan();
        for p in Purpose::all() {
            p.apply(&mut v);
            assert_eq!(Purpose::of(&v), p);
        }
        // coordination: every trade, none left out
        Purpose::Coordination.apply(&mut v);
        assert!(v.coordination && v.trade.is_none());
        assert_eq!(DisplayMode::of(&v), DisplayMode::Services);
        Purpose::Architecture.apply(&mut v);
        assert!(!v.coordination);
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

    /// The trade's colour in a coordination drawing.
    pub fn colour(self) -> [f32; 3] {
        match self {
            Trade::Plumbing => [0.10, 0.40, 0.90],
            Trade::Heating => [0.85, 0.15, 0.10],
            Trade::Ventilation => [0.10, 0.60, 0.25],
            Trade::MedGas => [0.60, 0.20, 0.75],
            Trade::Electrical => [0.95, 0.55, 0.0],
        }
    }

    /// The trade that draws pipes of the system `kind`.
    pub fn of_system(kind: crate::mep::SystemKind) -> Trade {
        Trade::ALL
            .into_iter()
            .find(|t| t.shows(kind))
            .unwrap_or(Trade::Plumbing)
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

    /// Is the object `e` this trade's equipment, drawn in its drawing in
    /// the medium pen: sanitary ware and drains for plumbing, heaters and
    /// heat sources for heating, air terminals and units for ventilation,
    /// devices and lights for electrical, supply units for medical gases.
    pub fn owns(self, model: &crate::Model, e: &crate::Element) -> bool {
        use crate::objects::ObjectCategory as C;
        let Some(t) = crate::objects::type_of(model, e) else {
            return false;
        };
        let class = t.ifc.class.to_ascii_lowercase();
        let family = crate::fixture::family_of_type(t);
        let heat = family.is_some_and(|f| {
            crate::fixture::is_heating(f)
                || crate::fixture::is_cooling(f)
                || crate::fixture::is_manifold(f)
        }) || matches!(
            class.as_str(),
            "ifcspaceheater" | "ifcboiler" | "ifcunitaryequipment" | "ifccoil" | "ifcpump"
        );
        let air = matches!(
            class.as_str(),
            "ifcairterminal" | "ifcfan" | "ifcairtoairheatrecovery" | "ifcductsilencer"
        );
        match self {
            Trade::Plumbing => {
                (family.is_some() && !heat)
                    || matches!(t.category, C::Plumbing | C::Toilet)
                    || matches!(class.as_str(), "ifcsanitaryterminal" | "ifcwasteterminal")
            }
            Trade::Heating => heat,
            Trade::Ventilation => air,
            Trade::Electrical => matches!(t.category, C::Electrical | C::Lighting),
            Trade::MedGas => t.category == C::Medical,
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
