//! The Room, Room separator and Ceiling tools. A room is placed by
//! clicking inside walls (it then follows them as they move) or drawn as
//! an outline where nothing closes it off; each new room gets a tag in
//! the level's plans. A ceiling goes over the room clicked, following it,
//! or is drawn. A room separator is a line of two clicks.

use crate::planview::{PlanState, Tool};
use crate::selection::Selection;
use creabim_core::room::{self, CEILING_HEIGHT};
use creabim_core::{
    Ceiling, Document, Element, ElementId, ElementKind, Model, ModelError, Room, RoomField,
    Separator, Tag, Transaction, ViewKind,
};
use eframe::egui;
use glam::DVec2;

/// What an outline drawn with the slab modes becomes, instead of a slab.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Space {
    Room,
    Ceiling,
}

impl Space {
    pub fn of(tool: Tool) -> Option<Space> {
        match tool {
            Tool::Room => Some(Space::Room),
            Tool::Ceiling => Some(Space::Ceiling),
            _ => None,
        }
    }
}

/// A point well inside `poly`: its middle when that is inside, else the
/// middle of its first triangle.
fn inside_point(poly: &[DVec2]) -> DVec2 {
    let middle = poly.iter().copied().sum::<DVec2>() / poly.len().max(1) as f64;
    if creabim_core::polygon::contains(poly, middle) {
        return middle;
    }
    creabim_core::polygon::triangulate(poly)
        .first()
        .map_or(middle, |t| (poly[t[0]] + poly[t[1]] + poly[t[2]]) / 3.0)
}

/// The height a new ceiling hangs at on `level`: the usual one, or lower
/// in a storey too low for it.
pub fn ceiling_height(model: &Model, level: ElementId) -> f64 {
    let storey = model.level(level).map_or(3.0, |l| l.height);
    CEILING_HEIGHT.min(storey - 0.1).max(0.5)
}

/// A new ceiling built as the last ceiling with a build-up is (its
/// build-up, thickness and roof space), so a storey's ceilings are placed
/// alike.
fn like_last(model: &Model, c: &mut Ceiling) {
    let last = model.iter().filter_map(|e| match &e.kind {
        ElementKind::Ceiling(x) if x.assembly.is_some() => Some(x),
        _ => None,
    });
    if let Some(x) = last.last() {
        c.assembly = x.assembly;
        c.thickness = x.thickness;
        c.roof_space = x.roof_space.clone();
    }
}

/// A room on `level` whose outline holds `p`.
pub fn room_at(model: &Model, level: ElementId, p: DVec2) -> Option<ElementId> {
    model
        .iter()
        .filter(|e| matches!(&e.kind, ElementKind::Room(r) if r.level == level))
        .find(|e| {
            room::outline(model, e.id).is_some_and(|o| creabim_core::polygon::contains(&o, p))
        })
        .map(|e| e.id)
}

/// A new room with its next number, and a tag for it at its point in each
/// of its level's plans (not its ceiling plans).
fn room_with_tags(model: &Model, r: Room) -> (Element, Vec<Element>) {
    let level = r.level;
    let mut r = r;
    r.number = room::next_number(model, level);
    let room = Element::new("Room", ElementKind::Room(r));
    let style = model
        .iter()
        .find(|e| e.name == "Room name" && matches!(e.kind, ElementKind::TextStyle(_)))
        .map(|e| e.id);
    let tags = creabim_core::levels::plan_views(model, level)
        .into_iter()
        .filter(|v| {
            matches!(model.get(*v).map(|e| &e.kind),
                Some(ElementKind::View(v)) if matches!(v.kind, ViewKind::Plan { .. }) && !v.ceiling_plan)
        })
        .map(|v| {
            let mut t = Tag::new(v, room.id, DVec2::ZERO);
            t.style = style;
            Element::new("Room tag", ElementKind::Tag(t))
        })
        .collect();
    (room, tags)
}

fn commit_room(
    doc: &mut Document,
    r: Room,
    label: &str,
    selection: &mut Selection,
) -> Result<ElementId, ModelError> {
    let (room, tags) = room_with_tags(doc.model(), r);
    let id = room.id;
    let mut tx = Transaction::new(label).insert(room);
    for t in tags {
        tx = tx.insert(t);
    }
    doc.commit(tx)?;
    selection.only(id);
    Ok(id)
}

/// What a closed outline drawn with the Room or Ceiling tool becomes.
pub fn drawn(
    doc: &mut Document,
    space: Space,
    level: ElementId,
    outline: Vec<DVec2>,
    place: &crate::placement::Placement,
    selection: &mut Selection,
) -> Result<String, ModelError> {
    match space {
        Space::Room => {
            let mut r = Room::new(level, inside_point(&outline));
            r.boundary = outline;
            if creabim_core::polygon::signed_area2(&r.boundary) < 0.0 {
                r.boundary.reverse();
            }
            let id = commit_room(doc, r, "Draw room", selection)?;
            let units = doc.model().units();
            Ok(format!(
                "Room {} added with the outline you drew ({}). Name it in Properties. Ctrl+Z to undo.",
                number_of(doc.model(), id),
                units.area(room::area(doc.model(), id))
            ))
        }
        Space::Ceiling => {
            let height = place.ceiling_height(ceiling_height(doc.model(), level));
            let mut c = Ceiling::drawn(level, outline, height);
            place.ceiling(doc.model(), &mut c, like_last);
            let e = Element::new("Ceiling", ElementKind::Ceiling(c));
            let id = e.id;
            doc.commit(Transaction::new("Draw ceiling").insert(e))?;
            selection.only(id);
            Ok(format!(
                "Ceiling added {} above the floor. Set its height, thickness and grid in Properties. Ctrl+Z to undo.",
                crate::units::len(height)
            ))
        }
    }
}

fn number_of(model: &Model, id: ElementId) -> String {
    match model.get(id).map(|e| &e.kind) {
        Some(ElementKind::Room(r)) => r.number.clone(),
        _ => String::new(),
    }
}

/// A click inside walls with the Room or Ceiling tool (Fill room).
pub fn fill(
    doc: &mut Document,
    space: Space,
    level: ElementId,
    p: DVec2,
    place: &crate::placement::Placement,
    selection: &mut Selection,
) -> Result<String, ModelError> {
    let model = doc.model();
    match space {
        Space::Room => {
            if let Some(there) = room_at(model, level, p) {
                selection.only(there);
                let name = model.get(there).map_or("", |e| e.name.as_str());
                return Ok(format!(
                    "There's a room here already ({name}). Add a room separator to split it."
                ));
            }
            let Some(shape) = room::enclosed(model, level, p) else {
                return Ok("Not closed: the walls and room separators here leave a gap (shown in red). Close it, or draw the room's outline (1 Polygon, 2 Rectangle, 4 From sketch).".into());
            };
            let id = commit_room(doc, Room::new(level, p), "Place room", selection)?;
            Ok(format!(
                "Room {} added: {}, to the walls' inner faces. It follows them when they move. Name it in Properties. Ctrl+Z to undo.",
                number_of(doc.model(), id),
                doc.model().units().area(shape.area())
            ))
        }
        Space::Ceiling => {
            let height = place.ceiling_height(ceiling_height(model, level));
            let c = match room_at(model, level, p) {
                Some(r) => {
                    let covered = model.iter().any(
                        |e| matches!(&e.kind, ElementKind::Ceiling(c) if c.room == Some(r)),
                    );
                    if covered {
                        return Ok("That room has a ceiling already; select it to change its height.".into());
                    }
                    Ceiling::in_room(level, r, height)
                }
                None => match room::enclosed(model, level, p) {
                    Some(shape) => Ceiling::drawn(level, shape.outline, height),
                    None => {
                        return Ok("Not closed: the walls here leave a gap (shown in red). Close it, or draw the ceiling's outline (1 Polygon, 2 Rectangle, 4 From sketch).".into())
                    }
                },
            };
            let follows = c.room.is_some();
            let mut c = c;
            place.ceiling(model, &mut c, like_last);
            let e = Element::new("Ceiling", ElementKind::Ceiling(c));
            let id = e.id;
            doc.commit(Transaction::new("Place ceiling").insert(e))?;
            selection.only(id);
            Ok(if follows {
                format!("Ceiling added over the room, {} above the floor; it follows the room's walls. Ctrl+Z to undo.", crate::units::len(height))
            } else {
                format!("Ceiling added inside the walls, {} above the floor. Place a room first for one that follows them. Ctrl+Z to undo.", crate::units::len(height))
            })
        }
    }
}

/// A click with the Room separator tool: its start, then its end.
pub fn separator_click(
    doc: &mut Document,
    level: ElementId,
    p: DVec2,
    state: &mut PlanState,
    selection: &mut Selection,
) -> Result<(), ModelError> {
    let Some(a) = state.pending.take() else {
        state.pending = Some(p);
        state.status = Some("Click where the separator ends.".into());
        return Ok(());
    };
    if a.distance(p) < 0.05 {
        state.pending = Some(a);
        return Ok(());
    }
    let e = Element::new(
        "Room separator",
        ElementKind::RoomSeparator(Separator {
            level,
            a,
            b: p,
            look: state.details.separator,
            bounds_slabs: false,
        }),
    );
    selection.only(e.id);
    state.status = Some("Room separator added: rooms on either side of it are now apart.".into());
    doc.commit(Transaction::new("Draw room separator").insert(e))
}

/// The room on `level` whose walls the points `a` and `p` run along.
pub fn tiling_room(
    model: &Model,
    level: ElementId,
    a: DVec2,
    p: DVec2,
) -> Option<(ElementId, room::RoomShape)> {
    let near = |s: &room::RoomShape, q: DVec2| {
        let n = s.outline.len();
        (0..n)
            .map(|i| {
                let (x, y) = (s.outline[i], s.outline[(i + 1) % n]);
                let l2 = x.distance_squared(y).max(1e-12);
                let t = ((q - x).dot(y - x) / l2).clamp(0.0, 1.0);
                q.distance(x + (y - x) * t)
            })
            .fold(f64::INFINITY, f64::min)
    };
    model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Room(r) if r.level == level => {
                let s = room::shape_of(model, r)?;
                let inside = [(a + p) / 2.0, a, p]
                    .iter()
                    .any(|q| crate::planview::point_in_polygon(*q, &s.outline));
                let d = near(&s, a) + near(&s, p);
                (inside || d < 0.6).then_some((e.id, s, d))
            }
            _ => None,
        })
        .min_by(|x, y| x.2.total_cmp(&y.2))
        .map(|(id, s, _)| (id, s))
}

/// A click with the Tiling tool: where the tiling starts on a room's
/// walls, then where it ends; it runs along the walls between, the
/// shorter way round, `state.details.tiling` high from the floor up.
pub fn tiling_click(
    doc: &mut Document,
    level: ElementId,
    p: DVec2,
    state: &mut PlanState,
    selection: &mut Selection,
) -> Result<(), ModelError> {
    let Some(a) = state.pending.take() else {
        state.pending = Some(p);
        state.status = Some("Click where the tiling ends, along the room's walls.".into());
        return Ok(());
    };
    if a.distance(p) < 0.05 {
        state.pending = Some(a);
        return Ok(());
    }
    let Some((id, _)) = tiling_room(doc.model(), level, a, p) else {
        state.status = Some("No room here: place a room first, then click along its walls.".into());
        return Ok(());
    };
    let (height, bottom) = state.details.tiling;
    let t = room::Tiling {
        height,
        bottom,
        span: Some([a, p]),
        ..room::Tiling::default()
    };
    let mark = t.mark_text();
    let r = doc.edit(id, "Draw tiling", |e| {
        if let ElementKind::Room(r) = &mut e.kind {
            r.tiling_parts.push(t);
        }
    });
    selection.only(id);
    state.status = Some(format!(
        "{} {mark}",
        crate::i18n::t("Tiling added along the room's walls; its mark can be dragged along it.")
    ));
    r
}

/// Tag every room on plan `view`'s level that has no tag in it yet, at the
/// room's point. Returns how many were tagged.
pub fn tag_rooms(doc: &mut Document, view: ElementId) -> Result<usize, ModelError> {
    let model = doc.model();
    let Some(ElementKind::View(v)) = model.get(view).map(|e| &e.kind) else {
        return Ok(0);
    };
    let ViewKind::Plan { level, .. } = v.kind else {
        return Ok(0);
    };
    let style = model
        .iter()
        .find(|e| e.name == "Room name" && matches!(e.kind, ElementKind::TextStyle(_)))
        .map(|e| e.id);
    let tagged: std::collections::HashSet<ElementId> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Tag(t) if t.view == view => Some(t.host),
            _ => None,
        })
        .collect();
    let mut tx = Transaction::new("Tag rooms");
    let mut n = 0;
    for e in model.iter() {
        if matches!(&e.kind, ElementKind::Room(r) if r.level == level) && !tagged.contains(&e.id) {
            let mut t = Tag::new(view, e.id, DVec2::ZERO);
            t.style = style;
            tx = tx.insert(Element::new("Room tag", ElementKind::Tag(t)));
            n += 1;
        }
    }
    if n > 0 {
        doc.commit(tx)?;
    }
    Ok(n)
}

/// Turn room labels written as notes into rooms; the status message.
pub fn rooms_from_notes(doc: &mut Document) -> String {
    let (tx, skipped) = room::from_notes(doc.model());
    let made = tx
        .ops
        .iter()
        .filter(
            |o| matches!(o, creabim_core::Op::Insert(e) if matches!(e.kind, ElementKind::Room(_))),
        )
        .count();
    if made > 0 {
        if let Err(e) = doc.commit(tx) {
            return format!("Couldn't do that: {e}");
        }
    }
    let left = if skipped.is_empty() {
        String::new()
    } else {
        format!(
            " Left as notes, as no walls close them off or a room is there already: {}. Draw those rooms, or add room separators.",
            skipped.join(", ")
        )
    };
    match made {
        0 => format!("No room labels (a name over an area, such as LIVING / 37.2 m²) became rooms.{left}"),
        n => format!("{n} room labels are now rooms with tags; their areas follow the walls. Ctrl+Z to undo.{left}"),
    }
}

/// The objects standing in a doorway or a door's swing, and a message
/// listing them.
pub fn check_doorways(model: &Model) -> (Vec<ElementId>, String) {
    let clashes = creabim_core::checks::objects_in_openings(model);
    if clashes.is_empty() {
        return (
            vec![],
            "No objects stand in a doorway, a window or a door's swing.".into(),
        );
    }
    let mut ids: Vec<ElementId> = clashes.iter().map(|c| c.object).collect();
    ids.dedup();
    let lines = creabim_core::checks::report(model, &clashes);
    (
        ids,
        format!(
            "{} objects are in the way (selected): {}.",
            clashes.len(),
            lines.join("; ")
        ),
    )
}

/// The walls with a layer moved by hand at a join, leaving a gap or
/// overlapping the wall it meets, and a message listing them.
pub fn check_layers(model: &Model) -> (Vec<ElementId>, String) {
    let issues = creabim_core::checks::layer_issues(model);
    if issues.is_empty() {
        return (
            vec![],
            "No layer leaves a gap or overlaps at a join.".into(),
        );
    }
    let mut ids: Vec<ElementId> = issues.iter().map(|i| i.wall).collect();
    ids.dedup();
    let lines = creabim_core::checks::layer_report(model, &issues);
    (
        ids,
        format!(
            "{} (walls selected): {}.",
            match issues.len() {
                1 => "1 layer end leaves a gap or overlap".to_string(),
                n => format!("{n} layer ends leave a gap or overlap"),
            },
            lines.join("; ")
        ),
    )
}

/// A text field with its label; true when it changed.
fn text(ui: &mut egui::Ui, label: &str, v: &mut String) -> bool {
    crate::props::flow(ui, |ui| {
        ui.label(crate::i18n::tr(label));
        ui.text_edit_singleline(v).changed()
    })
    .inner
}

/// A length in the project's units, with its label; true when it changed.
fn length(ui: &mut egui::Ui, label: &str, v: &mut f64, lo: f64, hi: f64) -> bool {
    crate::props::row(ui, label, |ui| {
        crate::props::fill(ui, crate::units::drag(v).range(lo..=hi)).changed()
    })
}

/// A room's properties: what it measures, then its number, finishes,
/// occupancy, department and height. Returns the edit's label, or "".
pub fn room_fields(ui: &mut egui::Ui, model: &Model, r: &mut Room, name: &str) -> &'static str {
    let mut label = "";
    match room::shape_of(model, r) {
        Some(s) => {
            ui.label(crate::i18n::tr(&format!(
                "Area {}   Perimeter {}",
                crate::units::area(room::net_area(model, r, &s)),
                crate::units::len(s.perimeter())
            )));
            ui.label(
                egui::RichText::new(if r.is_drawn() {
                    "Its outline is drawn; it doesn't follow the walls."
                } else {
                    "Measured to the walls' inner faces; it follows them as they move."
                })
                .weak(),
            );
        }
        None => {
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "Not enclosed: the walls and separators round its point don't close it off.",
            );
        }
    }
    if text(ui, "Number", &mut r.number) {
        label = "Room number";
    }
    for (name, v) in [
        ("Floor finish", &mut r.floor_finish),
        ("Wall finish", &mut r.wall_finish),
        ("Ceiling finish", &mut r.ceiling_finish),
        ("Occupancy", &mut r.occupancy),
        ("Department", &mut r.department),
        ("Fire zone", &mut r.fire_zone),
    ] {
        if text(ui, name, v) {
            label = "Room data";
        }
    }
    // its type for heat loss, cooling, air and floor heating: picking one
    // writes the occupancy
    {
        use creabim_core::roomtype::{Gains, RoomType};
        let tr = crate::i18n::tr;
        let found = RoomType::find(&r.occupancy);
        crate::props::row(ui, "Room type", |ui| {
            ui.spacing_mut().combo_width = ui.available_width();
            egui::ComboBox::from_id_salt("room_type")
                .icon(crate::props::combo_icon)
                .selected_text(found.map_or_else(|| tr("by its name"), |t| tr(t.label())))
                .show_ui(ui, |ui| {
                    for t in RoomType::ALL {
                        if ui
                            .selectable_label(found == Some(t), tr(t.label()))
                            .clicked()
                        {
                            r.occupancy = tr(t.label());
                            label = "Room data";
                        }
                    }
                });
        });
        let mut n = found.unwrap_or(RoomType::Other).needs();
        // the temperature it is heated to: by its type, or set here
        crate::props::row(ui, "Heated to", |ui| {
            let mut own = r.heated_to.is_some();
            if ui
                .checkbox(&mut own, tr("own"))
                .on_hover_text(tr("Off: the temperature of its room type. On: the temperature set here (a corridor inside the house heated as the rooms)."))
                .changed()
            {
                r.heated_to = own.then_some(n.temp);
                label = "Room data";
            }
            let mut t = r.heated_to.unwrap_or(n.temp);
            if ui
                .add_enabled(
                    own,
                    egui::DragValue::new(&mut t)
                        .range(0.0..=30.0)
                        .speed(0.5)
                        .suffix(" °C"),
                )
                .changed()
            {
                r.heated_to = Some(t);
                label = "Room data";
            }
        });
        if let Some(t) = r.heated_to {
            n.temp = t;
        }
        let mut air = vec![];
        if let Some(a) = n.ach {
            air.push(format!("{} {a} 1/h", tr("air change")));
        }
        if let Some(f) = n.flow {
            air.push(format!("{} {f:.0} m³/h", tr("extract")));
        }
        ui.label(
            egui::RichText::new(format!(
                "θint {:.0} °C · {} · {} · {}",
                n.temp,
                if air.is_empty() {
                    tr("no least air flow")
                } else {
                    air.join(", ")
                },
                if n.exhaust {
                    tr("air taken out")
                } else {
                    tr("air supplied")
                },
                match n.gains {
                    Gains::Service => tr("not cooled"),
                    Gains::Kitchen => tr("cooled, kitchen gains"),
                    Gains::Office => tr("cooled, office gains"),
                    Gains::Home => tr("cooled, dwelling gains"),
                }
            ))
            .weak(),
        );
    }
    // in the living area or not: by its type and area, or by the
    // designer's word
    {
        let tr = crate::i18n::tr;
        let area = room::shape_of(model, r).map_or(0.0, |s| room::net_area(model, r, &s));
        let by_type = room::counts_as_living(
            name,
            &Room {
                living: None,
                ..r.clone()
            },
            area,
        );
        let shown = |v: Option<bool>| match v {
            None => format!(
                "{} ({})",
                tr("by its type"),
                if by_type { tr("yes") } else { tr("no") }
            ),
            Some(true) => tr("Counted in"),
            Some(false) => tr("Left out"),
        };
        crate::props::row(ui, "Living area", |ui| {
            ui.spacing_mut().combo_width = ui.available_width();
            egui::ComboBox::from_id_salt("room_living")
                .icon(crate::props::combo_icon)
                .selected_text(shown(r.living))
                .show_ui(ui, |ui| {
                    for v in [None, Some(true), Some(false)] {
                        if ui.selectable_label(r.living == v, shown(v)).clicked() {
                            r.living = v;
                            label = "Room data";
                        }
                    }
                })
        })
        .response
        .on_hover_text(tr("Living rooms (also with a kitchen in them), bedrooms, children's rooms, dining rooms and studies of at least 8 m² count in the living area; a kitchen on its own does not (STN 73 4301/Z1). Set otherwise for this room here."));
    }
    // what it counts as for its persons, and the designed count
    let table = creabim_geom::occupancy::table(model);
    // with none given, the item its type gives (Fire calculation
    // parameters, table of room types)
    let auto = creabim_geom::firerisk::AutoUses::new(model);
    let by_type = |own: &str, item: String, t: Option<&creabim_core::fireparams::ParamTable>| {
        let text = |item: &str| match t.and_then(|t| t.row(item)) {
            Some(row) => format!("{} {}", row[0], row.get(1).map_or("", |s| s.as_str())),
            None => item.to_string(),
        };
        if own.trim().is_empty() && !item.is_empty() {
            format!("{} ({})", text(&item), crate::i18n::t("by the room type"))
        } else {
            text(own.trim())
        }
    };
    let shown = by_type(&r.fire_use, auto.occupancy(r, name), table);
    let mut picked: Option<String> = None;
    crate::props::row(ui, "Use for the persons", |ui| {
        egui::ComboBox::from_id_salt("room.fire_use")
            .selected_text(shown)
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                if ui.selectable_label(r.fire_use.is_empty(), "-").clicked() {
                    picked = Some(String::new());
                }
                match table {
                    Some(t) => {
                        for row in &t.rows {
                            let name = row.get(1).map_or("", |s| s.as_str());
                            let text = format!("{} {name}", row[0]);
                            if ui.selectable_label(r.fire_use == row[0], text).clicked() {
                                picked = Some(row[0].clone());
                            }
                        }
                    }
                    None => {
                        ui.label(crate::i18n::t(
                            "No occupancy table in the project: read it in Project information, Fire calculation parameters.",
                        ));
                    }
                }
            });
    });
    if let Some(p) = picked {
        r.fire_use = p;
        label = "Room data";
    }
    let mut counted = r.design_count.is_some();
    if crate::props::switch(ui, &mut counted, "Designed count")
        .on_hover_text(crate::i18n::t(
            "The persons, beds, seats or parking places the norm's factor multiplies",
        ))
        .changed()
    {
        r.design_count = counted.then_some(1.0);
        label = "Room data";
    }
    if let Some(n) = r.design_count.as_mut() {
        let changed = crate::props::row(ui, "Count", |ui| {
            ui.add(egui::DragValue::new(n).range(0.0..=100_000.0).speed(0.1))
                .changed()
        });
        if changed {
            label = "Room data";
        }
    }
    // what it is for its fire load, and its fixed load where given
    let loads = creabim_geom::firerisk::table(model, creabim_core::fireparams::FIRE_LOAD);
    let shown = by_type(&r.fire_load, auto.fire_load(r, name), loads);
    let mut picked: Option<String> = None;
    crate::props::row(ui, "Use for the fire load", |ui| {
        egui::ComboBox::from_id_salt("room.fire_load")
            .selected_text(shown)
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                if ui.selectable_label(r.fire_load.is_empty(), "-").clicked() {
                    picked = Some(String::new());
                }
                match loads {
                    Some(t) => {
                        for row in &t.rows {
                            let name = row.get(1).map_or("", |s| s.as_str());
                            let text = format!("{} {name}", row[0]);
                            if ui.selectable_label(r.fire_load == row[0], text).clicked() {
                                picked = Some(row[0].clone());
                            }
                        }
                    }
                    None => {
                        ui.label(crate::i18n::t(
                            "No fire load table in the project: read it in Project information, Fire calculation parameters.",
                        ));
                    }
                }
            });
    });
    if let Some(p) = picked {
        r.fire_load = p;
        label = "Room data";
    }
    let mut own_ps = r.fire_ps.is_some();
    if crate::props::switch(ui, &mut own_ps, "Own fixed fire load")
        .on_hover_text(crate::i18n::t(
            "Without it the fixed load of windows, doors and floors is taken from the table by the room's area",
        ))
        .changed()
    {
        r.fire_ps = own_ps.then_some(0.0);
        label = "Room data";
    }
    if let Some(ps) = r.fire_ps.as_mut() {
        let changed = crate::props::row(ui, "ps", |ui| {
            ui.add(
                egui::DragValue::new(ps)
                    .range(0.0..=10_000.0)
                    .speed(0.1)
                    .suffix(" kg/m²"),
            )
            .changed()
        });
        if changed {
            label = "Room data";
        }
    }
    // 4.4.5 of STN 92 0201-4: a concentrated fire load, its openings
    // with their own heat flux in the separation distance
    let mut has_pvm = r.fire_pvm.is_some();
    if crate::props::switch(ui, &mut has_pvm, "Concentrated fire load")
        .on_hover_text(crate::i18n::t(
            "A concentrated fire load in the room: its openings count with their own heat flux in the separation distance (STN 92 0201-4, 4.4.5)",
        ))
        .changed()
    {
        r.fire_pvm = has_pvm.then_some(0.0);
        label = "Room data";
    }
    if let Some(pvm) = r.fire_pvm.as_mut() {
        let changed = crate::props::row(ui, "pvm", |ui| {
            ui.add(
                egui::DragValue::new(pvm)
                    .range(0.0..=10_000.0)
                    .speed(0.1)
                    .suffix(" kg/m²"),
            )
            .changed()
        });
        if changed {
            label = "Room data";
        }
    }
    let use_item = auto.occupancy(r, name);
    if !use_item.is_empty() || r.design_count.is_some() {
        let area = room::shape_of(model, r).map_or(0.0, |s| room::net_area(model, r, &s));
        let cell = |c: &str| table.map_or("", |t| t.cell(&use_item, c));
        let p = creabim_geom::occupancy::persons(
            cell(creabim_geom::occupancy::AREA),
            cell(creabim_geom::occupancy::FACTOR),
            area,
            r.design_count,
        );
        ui.label(match p {
            Some(p) => format!("{}: {p}", crate::i18n::t("Persons")),
            None => crate::i18n::t("Persons: the item gives no value to count by here").to_string(),
        });
    }
    // tiling of the walls: what it is and how high it reaches
    let mut tiled = r.tiling.is_some();
    if crate::props::switch(ui, &mut tiled, "Wall tiling")
        .on_hover_text(crate::i18n::t(
            "A thin line along the room's walls in the plan, the doors left out; its area goes to the legend of rooms",
        ))
        .changed()
    {
        r.tiling = tiled.then(room::Tiling::default);
        label = "Wall tiling";
    }
    let tiled_shape = r.tiling.as_ref().and_then(|_| room::shape_of(model, r));
    if let Some(t) = r.tiling.as_mut() {
        if text(ui, "Tiling", &mut t.name) {
            label = "Wall tiling";
        }
        if length(ui, "Tiling height", &mut t.height, 0.1, 10.0) {
            label = "Wall tiling";
        }
        if length(ui, "Above the floor", &mut t.bottom, 0.0, 10.0) {
            label = "Wall tiling";
        }
        let mut shown = !t.no_mark;
        if ui
            .checkbox(&mut shown, crate::i18n::t("Mark (height)(above the floor)"))
            .changed()
        {
            t.no_mark = !shown;
            label = "Wall tiling";
        }
        if let Some(s) = &tiled_shape {
            // which of its walls carry tiles
            for (i, wall) in room::bounding_walls(model, s).into_iter().enumerate() {
                let len: f64 = room::tiling_runs(
                    model,
                    s,
                    &room::bounding_walls(model, s)
                        .into_iter()
                        .filter(|w| *w != wall)
                        .collect::<Vec<_>>(),
                )
                .iter()
                .map(|(a, b)| a.distance(*b))
                .sum();
                let mut on = !t.skip.contains(&wall);
                let name = format!(
                    "{} {} ({})",
                    crate::i18n::t("Wall"),
                    i + 1,
                    crate::units::len(len)
                );
                if ui.checkbox(&mut on, name).changed() {
                    if on {
                        t.skip.retain(|w| *w != wall);
                    } else {
                        t.skip.push(wall);
                    }
                    label = "Wall tiling";
                }
            }
        }
    }
    // tiling on part of the walls, drawn with the Tiling tool: a
    // kitchen's splashback (George 2026-11-24)
    let mut gone = None;
    for (i, t) in r.tiling_parts.iter_mut().enumerate() {
        ui.push_id(("tiling part", i), |ui| {
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} {} {}",
                        crate::i18n::t("Partial tiling"),
                        i + 1,
                        t.mark_text()
                    ))
                    .strong(),
                );
                if ui
                    .small_button("🗑")
                    .on_hover_text(crate::i18n::t("Remove this tiling"))
                    .clicked()
                {
                    gone = Some(i);
                }
            });
            if text(ui, "Tiling", &mut t.name) {
                label = "Wall tiling";
            }
            if length(ui, "Tiling height", &mut t.height, 0.05, 10.0) {
                label = "Wall tiling";
            }
            if length(ui, "Above the floor", &mut t.bottom, 0.0, 10.0) {
                label = "Wall tiling";
            }
            let mut shown = !t.no_mark;
            if ui
                .checkbox(&mut shown, crate::i18n::t("Mark (height)(above the floor)"))
                .changed()
            {
                t.no_mark = !shown;
                label = "Wall tiling";
            }
        });
    }
    if let Some(i) = gone {
        r.tiling_parts.remove(i);
        label = "Wall tiling";
    }
    ui.label(
        egui::RichText::new(crate::i18n::t(
            "Tiling on part of the walls (over the kitchen's base cabinets): the Tiling tool, two clicks along the walls.",
        ))
        .weak(),
    );
    let tiled_shape = tiled_shape.or_else(|| {
        (!r.tiling_parts.is_empty())
            .then(|| room::shape_of(model, r))
            .flatten()
    });
    if let Some(s) = &tiled_shape {
        ui.label(format!(
            "{} {}",
            crate::i18n::t("Tiled area"),
            model.units().area(room::tiling_area(model, r, s))
        ));
    }
    let mut own = r.height.is_some();
    let found = room::height(
        model,
        &Room {
            height: None,
            ..r.clone()
        },
    );
    if crate::props::switch(ui, &mut own, "Own height")
        .on_hover_text(crate::i18n::t(
            "Otherwise it is as high as its ceiling, or its storey",
        ))
        .changed()
    {
        r.height = own.then_some(found);
        label = "Room height";
    }
    match r.height.as_mut() {
        Some(h) => {
            if length(ui, "Height", h, 0.1, 100.0) {
                label = "Room height";
            }
        }
        None => {
            ui.label(crate::i18n::tr(&format!(
                "Height {} (to its ceiling or the storey above)",
                crate::units::len(found)
            )));
        }
    }
    if r.is_drawn()
        && ui
            .button(crate::i18n::t("Follow the walls instead"))
            .on_hover_text(crate::i18n::t(
                "Forget the drawn outline: measure it to the walls round its point",
            ))
            .clicked()
    {
        r.boundary.clear();
        label = "Room follows walls";
    }
    label
}

/// A ceiling's properties: its height, thickness and grid. Returns the
/// edit's label, or "".
pub fn ceiling_fields(
    ui: &mut egui::Ui,
    model: &Model,
    c: &mut Ceiling,
    layered: bool,
) -> &'static str {
    let mut label = "";
    match c.room.and_then(|r| model.get(r)) {
        Some(r) if c.boundary.is_empty() => {
            ui.label(crate::i18n::tr(&format!(
                "Over {} {}: it follows the room's walls.",
                number_of(model, r.id),
                r.name
            )));
        }
        _ => {
            ui.label(egui::RichText::new(crate::i18n::t("Its outline is drawn.")).weak());
        }
    }
    if length(ui, "Height", &mut c.height, 0.1, 50.0) {
        label = "Ceiling height";
    }
    if layered {
        ui.label(crate::i18n::tr(&format!(
            "Thickness {}, from its layers",
            crate::units::len(c.thickness)
        )));
        // the roof space over it: STN EN ISO 6946 table 9 (George
        // 2026-11-23)
        let tables = creabim_core::room::project_tables(model);
        let rows = creabim_core::room::roof_spaces(&tables);
        if rows.is_empty() {
            ui.label(
                egui::RichText::new(crate::i18n::t(
                    "Roof space over it: read table 9 of STN EN ISO 6946 (Heat loss, Read table…) to pick one.",
                ))
                .weak(),
            );
        } else {
            let shown = c
                .roof_space
                .as_ref()
                .and_then(|k| rows.iter().find(|r| &r.0 == k))
                .map_or(crate::i18n::t("None").to_string(), |r| {
                    format!("{} · Ru {}", r.0, format!("{}", r.1).replace('.', ","))
                });
            crate::props::row(ui, "Roof space", |ui| {
                egui::ComboBox::from_id_salt("ceiling-roof-space")
                    .selected_text(shown)
                    .width(ui.available_width().min(160.0))
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(c.roof_space.is_none(), crate::i18n::t("None"))
                            .clicked()
                        {
                            c.roof_space = None;
                            label = "Roof space";
                        }
                        for (k, ru, text) in &rows {
                            let on = c.roof_space.as_deref() == Some(k.as_str());
                            let t = format!("{k} · Ru {} · {text}", format!("{ru}").replace('.', ","));
                            if ui.selectable_label(on, t).clicked() {
                                c.roof_space = Some(k.clone());
                                label = "Roof space";
                            }
                        }
                    })
                    .response
                    .on_hover_text(crate::i18n::t(
                        "STN EN ISO 6946, table 9: the ventilated roof space over the ceiling counts as a layer of resistance Ru in the heat loss.",
                    ))
            });
        }
    } else if length(ui, "Thickness", &mut c.thickness, 0.001, 1.0) {
        label = "Ceiling thickness";
    }
    let mut grid = c.grid > 0.0;
    if crate::props::switch(ui, &mut grid, "Suspended grid")
        .on_hover_text(crate::i18n::t("Tiles drawn in ceiling plans"))
        .changed()
    {
        c.grid = if grid { 0.6 } else { 0.0 };
        label = "Ceiling grid";
    }
    if c.grid > 0.0 && length(ui, "Tile", &mut c.grid, 0.1, 5.0) {
        label = "Ceiling grid";
    }
    label
}

/// What a room's tag shows, as tick boxes in order. Returns the edit's
/// label, or "".
pub fn tag_fields(ui: &mut egui::Ui, fields: &mut Vec<RoomField>) -> &'static str {
    let mut label = "";
    let mut shown: Vec<RoomField> = if fields.is_empty() {
        RoomField::DEFAULT.to_vec()
    } else {
        fields.clone()
    };
    ui.label(crate::i18n::t("Shows"));
    for f in RoomField::ALL {
        let mut on = shown.contains(&f);
        if ui.checkbox(&mut on, crate::i18n::tr(f.label())).changed() {
            if on {
                // keep the order fields are listed in
                shown.push(f);
                shown.sort_by_key(|x| RoomField::ALL.iter().position(|y| y == x));
            } else {
                shown.retain(|x| *x != f);
            }
            label = "Room tag fields";
        }
    }
    if !label.is_empty() {
        *fields = if shown == RoomField::DEFAULT {
            vec![]
        } else {
            shown
        };
    }
    label
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::starter::starter_house;

    fn named(m: &Model, name: &str) -> ElementId {
        m.iter().find(|e| e.name == name).unwrap().id
    }

    #[test]
    fn a_room_placed_inside_walls_gets_a_number_and_a_tag() {
        let m = starter_house();
        let study = named(&m, "Study");
        let level = match &m.get(study).unwrap().kind {
            ElementKind::Room(r) => r.level,
            _ => unreachable!(),
        };
        // take the study away, then place it again
        let tx = Transaction::new("No study").remove_cascade(&m, study);
        let mut doc = Document::from_model(m.clone());
        doc.commit(tx).unwrap();
        let mut sel = Selection::default();
        let msg = fill(
            &mut doc,
            Space::Room,
            level,
            DVec2::new(2.0, 3.5),
            &Default::default(),
            &mut sel,
        )
        .unwrap();
        assert!(msg.starts_with("Room 1.10 added"), "{msg}");
        let id = sel.active.unwrap();
        assert!(room::area(doc.model(), id) > 15.0);
        let tags = doc
            .model()
            .iter()
            .filter(|e| matches!(&e.kind, ElementKind::Tag(t) if t.host == id))
            .count();
        assert!(tags >= 1);
        // clicking there again finds it
        let again = fill(
            &mut doc,
            Space::Room,
            level,
            DVec2::new(2.0, 3.5),
            &Default::default(),
            &mut sel,
        )
        .unwrap();
        assert!(again.contains("already"), "{again}");
        // and a ceiling over it follows it
        fill(
            &mut doc,
            Space::Ceiling,
            level,
            DVec2::new(2.0, 3.5),
            &Default::default(),
            &mut sel,
        )
        .unwrap();
        let c = sel.active.unwrap();
        assert!(
            matches!(&doc.model().get(c).unwrap().kind, ElementKind::Ceiling(c) if c.room == Some(id))
        );
    }

    #[test]
    fn a_separator_takes_two_clicks() {
        let m = starter_house();
        let level = named(&m, "Ground Floor");
        let mut doc = Document::from_model(m);
        let mut state = PlanState::default();
        let mut sel = Selection::default();
        separator_click(&mut doc, level, DVec2::new(8.0, 5.6), &mut state, &mut sel).unwrap();
        assert!(state.pending.is_some());
        separator_click(&mut doc, level, DVec2::new(8.0, 9.6), &mut state, &mut sel).unwrap();
        assert!(matches!(
            doc.model().get(sel.active.unwrap()).unwrap().kind,
            ElementKind::RoomSeparator(_)
        ));
    }
}
