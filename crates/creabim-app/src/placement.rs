//! Where new constructions are drawn to, set in the tool bar before
//! drawing: the base and top of walls and columns, the top of slabs and
//! beams, the base of roofs, the height and build-up of ceilings (George
//! 2026-11-24: "už pri zadávaní konštrukcie chcem mať kontrolu do akej
//! výšky ju kreslím a aký materiál bude mať"). Heights are relative to the
//! floor of the level drawn on; the bar also shows them as elevations.

use crate::planview::{PlanState, Tool};
use creabim_core::members::{elevation, storey_height};
use creabim_core::{ElementId, ElementKind, Model};
use serde_json::{json, Value};

/// What new ceilings are built of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Build {
    /// As the last ceiling with a build-up (its build-up, thickness and
    /// roof space), so a storey's ceilings come out alike.
    #[default]
    LikeLast,
    /// One material, no build-up.
    Single,
    /// This build-up (a ceiling assembly).
    Assembly(ElementId),
}

/// Heights and build-ups for new constructions.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Placement {
    /// New walls' base above the level's floor.
    pub wall_base: f64,
    /// New walls' top above the level's floor; none, the type's height or
    /// the storey's.
    pub wall_top: Option<f64>,
    /// New slabs' top above the level's floor (negative: below).
    pub slab_top: f64,
    /// New roofs' base (wall plate) above the level's floor.
    pub roof_base: f64,
    /// New columns' base above the level's floor.
    pub column_base: f64,
    /// New columns' top above the level's floor; none, up to the next
    /// level.
    pub column_top: Option<f64>,
    /// New beams' top above the level's floor; none, under the slab above.
    pub beam_top: Option<f64>,
    /// New ceilings' underside above the floor; none, the usual height.
    pub ceiling_height: Option<f64>,
    pub ceiling_build: Build,
    /// New stairs' (and ramps') base above the level's floor.
    pub stair_base: f64,
    /// New stairs' top above the level's floor; none, up to the next
    /// level.
    pub stair_top: Option<f64>,
}

impl Placement {
    /// `w` (just made by the Wall tool) set on its base and top.
    pub fn wall(&self, w: &mut creabim_core::Wall) {
        let top = self.wall_top.unwrap_or(w.base_offset + w.height);
        w.base_offset = self.wall_base;
        w.height = (top - self.wall_base).max(0.05);
    }

    /// A new slab's or roof's height, by the tool drawing it.
    pub fn outline(&self, roof: bool, s: &mut creabim_core::Slab) {
        s.top_offset = if roof { self.roof_base } else { self.slab_top };
    }

    pub fn column(&self, c: &mut creabim_core::Column) {
        c.base_offset = self.column_base;
        if let Some(t) = self.column_top {
            c.height = Some((t - self.column_base).max(0.05));
        }
    }

    pub fn beam(&self, model: &Model, b: &mut creabim_core::Beam) {
        if let Some(t) = self.beam_top {
            b.top_offset = storey_height(model, b.level) - t;
        }
    }

    /// `s` (just made by the Stair tool) set on its base and top.
    pub fn stair(&self, s: &mut creabim_core::Stair) {
        s.base_offset = self.stair_base;
        if let Some(t) = self.stair_top {
            s.height = Some((t - self.stair_base).max(0.05));
        }
    }

    /// The height a new ceiling hangs at, given the usual one.
    pub fn ceiling_height(&self, usual: f64) -> f64 {
        self.ceiling_height.unwrap_or(usual)
    }

    /// A new ceiling's build-up; `like_last` copies the last one's.
    pub fn ceiling(
        &self,
        model: &Model,
        c: &mut creabim_core::Ceiling,
        like_last: impl Fn(&Model, &mut creabim_core::Ceiling),
    ) {
        match self.ceiling_build {
            Build::LikeLast => like_last(model, c),
            Build::Single => c.assembly = None,
            Build::Assembly(id) => {
                if let Some(ElementKind::Assembly(a)) = model.get(id).map(|e| &e.kind) {
                    c.assembly = Some(id);
                    c.thickness = a.total();
                }
            }
        }
    }
}

/// The ceiling build-ups of the project.
pub fn ceiling_assemblies(model: &Model) -> Vec<(ElementId, String)> {
    creabim_core::assembly::assemblies(model)
        .into_iter()
        .filter(|(_, _, a)| a.usage == creabim_core::AssemblyUse::Ceiling)
        .map(|(id, n, _)| (id, n.to_string()))
        .collect()
}

/// The name shown for the ceiling build-up picked.
pub fn build_label(model: &Model, b: Build) -> String {
    match b {
        Build::LikeLast => "As the last ceiling".into(),
        Build::Single => "None (single material)".into(),
        Build::Assembly(id) => model
            .get(id)
            .map_or_else(|| "As the last ceiling".into(), |e| e.name.clone()),
    }
}

fn num(key: &str, label: &str, value: f64, hint: &str) -> Value {
    json!({
        "kind": "number", "key": format!("place.{key}"), "label": label, "value": value,
        "unit": "m", "step": 0.01, "min": -50.0, "max": 200.0, "hint": hint,
    })
}

fn toggle(id: &str, label: &str, on: bool, icon: &str, hint: &str) -> Value {
    json!({
        "kind": "toggle", "id": format!("reach:{id}"), "label": label, "on": on,
        "icon": icon, "hint": hint,
    })
}

/// The elevations from `lo` to `hi` above the floor of `level`, as the bar
/// shows them: the level's name, then the two elevations.
fn reach(model: &Model, level: ElementId, lo: f64, hi: Option<f64>) -> Value {
    let z = elevation(model, level);
    let name = model.get(level).map_or("", |e| e.name.as_str());
    let text = match hi {
        Some(hi) => format!(
            "{name}: {} … {}",
            crate::units::elevation(z + lo),
            crate::units::elevation(z + hi)
        ),
        None => format!("{name}: {}", crate::units::elevation(z + lo)),
    };
    json!({"kind": "meta", "label": text})
}

/// The tool bar's height and build-up settings for the tool in use, drawn
/// on `level`; `wall` is a wall as the Wall tool would draw it now.
pub fn tool_options(
    model: &Model,
    state: &PlanState,
    level: ElementId,
    wall: Option<&creabim_core::Wall>,
) -> Vec<Value> {
    let p = &state.members.place;
    let storey = storey_height(model, level);
    match state.tool {
        Tool::Wall => {
            let (base, top) = wall.map_or((p.wall_base, storey), |w| {
                (w.base_offset, w.base_offset + w.height)
            });
            vec![
                num("wall_base", "Base offset", base, "How far above the floor of the level new walls start (negative: below it)"),
                num("wall_top", "Top", top, "How high above the floor of the level new walls reach. Typing a value fixes it; Storey height takes the type's or the storey's again."),
                toggle("wall_top", "Storey height", p.wall_top.is_none(), "levels", "New walls reach the height of their type, else of the storey"),
                reach(model, level, base, Some(top)),
            ]
        }
        Tool::Slab => vec![
            num("slab_top", "Top offset", p.slab_top, "Where new slabs' top face lies above the floor of the level (negative: below it); their thickness and layers come from the type"),
            reach(model, level, p.slab_top, None),
        ],
        Tool::Roof => vec![
            num("roof_base", "Base offset", p.roof_base, "How high above the floor of the level new roofs start (the wall plate)"),
            reach(model, level, p.roof_base, None),
        ],
        Tool::Column => {
            let top = p.column_top.unwrap_or(storey);
            vec![
                num("column_base", "Base offset", p.column_base, "How far above the floor of the level new columns start"),
                num("column_top", "Top", top, "How high above the floor of the level new columns reach. Typing a value fixes it; To the level above lets them follow the next level again."),
                toggle("column_top", "To the level above", p.column_top.is_none(), "levels", "New columns run up to the next level"),
                reach(model, level, p.column_base, Some(top)),
            ]
        }
        Tool::Beam => {
            let mut v = vec![toggle(
                "beam_top",
                "Under the slab",
                p.beam_top.is_none(),
                "slab",
                "New beams hang with their top under the slab above",
            )];
            if let Some(t) = p.beam_top {
                v.push(num("beam_top", "Top", t, "How high above the floor of the level new beams' top lies"));
                v.push(reach(
                    model,
                    level,
                    t - state.members.beam_depth,
                    Some(t),
                ));
            } else {
                v.push(num("beam_top", "Top", storey, "How high above the floor of the level new beams' top lies; typing a value fixes it"));
            }
            v
        }
        Tool::Stair => {
            let top = p.stair_top.unwrap_or(storey);
            vec![
                num("stair_base", "Base offset", p.stair_base, "How far above the floor of the level new stairs start"),
                num("stair_top", "Top", top, "How high above the floor of the level new stairs climb. Typing a value fixes it; To the level above lets them climb to the next level again."),
                toggle("stair_top", "To the level above", p.stair_top.is_none(), "levels", "New stairs climb to the next level"),
                reach(model, level, p.stair_base, Some(top)),
            ]
        }
        Tool::Ceiling => {
            let h = p.ceiling_height(crate::roomtool::ceiling_height(model, level));
            vec![
                json!({
                    "kind": "menu", "key": "place.ceiling", "label": "Assembly",
                    "value": build_label(model, p.ceiling_build),
                    "hint": "What new ceilings are built of: a ceiling build-up (its layers and thickness), one material, or as the last ceiling",
                }),
                num("ceiling_height", "Underside above the floor", h, "How high above the floor of the level new ceilings' underside hangs"),
                reach(model, level, h, None),
            ]
        }
        _ => vec![],
    }
}

/// A number typed in the bar (`place.<what>`).
pub fn set_option(state: &mut PlanState, key: &str, v: f64) {
    let p = &mut state.members.place;
    let v = v.clamp(-50.0, 200.0);
    match key.trim_start_matches("place.") {
        "wall_base" => p.wall_base = v,
        "wall_top" => p.wall_top = Some(v),
        "slab_top" => p.slab_top = v,
        "roof_base" => p.roof_base = v,
        "column_base" => p.column_base = v,
        "column_top" => p.column_top = Some(v),
        "beam_top" => p.beam_top = Some(v),
        "stair_base" => p.stair_base = v,
        "stair_top" => p.stair_top = Some(v),
        "ceiling_height" => p.ceiling_height = Some(v.max(0.1)),
        _ => {}
    }
}

/// A toggle clicked in the bar (`reach:<what>`): back to the automatic top.
pub fn toggle_option(state: &mut PlanState, model: &Model, level: Option<ElementId>, what: &str) {
    let storey = level.map_or(3.0, |l| storey_height(model, l));
    let p = &mut state.members.place;
    match what {
        "wall_top" => {
            p.wall_top = if p.wall_top.is_some() {
                None
            } else {
                Some(storey)
            }
        }
        "column_top" => {
            p.column_top = if p.column_top.is_some() {
                None
            } else {
                Some(storey)
            }
        }
        "beam_top" => {
            p.beam_top = if p.beam_top.is_some() {
                None
            } else {
                Some(storey)
            }
        }
        "stair_top" => {
            p.stair_top = if p.stair_top.is_some() {
                None
            } else {
                Some(storey)
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::{Document, Element, Transaction};
    use glam::DVec2;

    fn house() -> (Document, ElementId) {
        let m = creabim_core::starter::starter_house();
        let l = m
            .levels()
            .min_by(|a, b| a.1.elevation.total_cmp(&b.1.elevation))
            .unwrap()
            .0
            .id;
        (Document::from_model(m), l)
    }

    #[test]
    fn a_wall_takes_its_base_and_top_from_the_bar() {
        let (doc, l) = house();
        let mut state = PlanState::default();
        let w = crate::planview::new_wall(doc.model(), l, &state, DVec2::ZERO, DVec2::X);
        let storey_top = w.base_offset + w.height;
        state.members.place.wall_base = -0.3;
        let w = crate::planview::new_wall(doc.model(), l, &state, DVec2::ZERO, DVec2::X);
        // the top stays where it was, the base drops
        assert_eq!(w.base_offset, -0.3);
        assert!((w.base_offset + w.height - storey_top).abs() < 1e-9);
        state.members.place.wall_top = Some(1.2);
        let w = crate::planview::new_wall(doc.model(), l, &state, DVec2::ZERO, DVec2::X);
        assert!((w.base_offset + w.height - 1.2).abs() < 1e-9);
    }

    #[test]
    fn a_beam_and_a_column_reach_the_tops_set() {
        let (doc, l) = house();
        let model = doc.model();
        let p = Placement {
            column_base: 0.1,
            column_top: Some(2.5),
            beam_top: Some(2.5),
            ..Default::default()
        };
        let mut c = creabim_core::Column::new(
            l,
            DVec2::ZERO,
            creabim_core::members::Profile::Round { diameter: 0.3 },
        );
        p.column(&mut c);
        assert!((c.base_offset + c.height_in(model) - 2.5).abs() < 1e-9);
        let mut b = creabim_core::Beam::new(l, DVec2::ZERO, DVec2::X);
        p.beam(model, &mut b);
        let top = b.top_offset;
        assert!((storey_height(model, l) - top - 2.5).abs() < 1e-9);
    }

    #[test]
    fn a_slab_and_a_roof_sit_where_the_bar_says() {
        let (doc, l) = house();
        let mut state = PlanState::default();
        state.members.place.slab_top = -0.12;
        state.members.place.roof_base = 2.9;
        let pts = vec![DVec2::ZERO, DVec2::X, DVec2::ONE];
        state.tool = Tool::Slab;
        let slab = creabim_core::Slab::new(l, pts.clone(), 0.25);
        let (e, _) = crate::memberstool::placed_outline_element(doc.model(), &state, slab.clone());
        assert!(matches!(e.kind, ElementKind::Slab(s) if s.top_offset == -0.12));
        state.tool = Tool::Roof;
        let (e, _) = crate::memberstool::placed_outline_element(doc.model(), &state, slab);
        assert!(matches!(e.kind, ElementKind::Roof(r) if r.base_offset == 2.9));
    }

    #[test]
    fn a_ceiling_takes_the_build_up_picked() {
        let (mut doc, l) = house();
        let a = Element::new(
            "SDK",
            ElementKind::Assembly(creabim_core::Assembly {
                usage: creabim_core::AssemblyUse::Ceiling,
                ..serde_json::from_str(r#"{"layers":[{"material":null,"thickness":0.0125}]}"#)
                    .unwrap()
            }),
        );
        let id = a.id;
        doc.commit(Transaction::new("a").insert(a)).unwrap();
        let p = Placement {
            ceiling_build: Build::Assembly(id),
            ceiling_height: Some(2.4),
            ..Default::default()
        };
        let mut c = creabim_core::Ceiling::drawn(
            l,
            vec![DVec2::ZERO, DVec2::X, DVec2::ONE],
            p.ceiling_height(2.6),
        );
        p.ceiling(doc.model(), &mut c, |_, _| {});
        assert_eq!(c.height, 2.4);
        assert_eq!(c.assembly, Some(id));
        assert!((c.thickness - 0.0125).abs() < 1e-9);
    }
}
