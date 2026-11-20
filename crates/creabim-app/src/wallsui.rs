//! How walls and their doors and windows behave, in the app: a wall's
//! location line, base and top constraints, end joins and attached top in
//! Properties, and the flip, join and attach edits picked from a button,
//! a key or a right-click menu. Every change is one transaction, so it
//! undoes like any edit.

use super::*;
use creabim_core::{walls, LocationLine, Opening, OpeningKind, Wall, WallTop};

/// A length field in a row, as the rest of Properties draws them.
fn num(ui: &mut egui::Ui, text: &str, v: &mut f64, lo: f64, hi: f64) -> bool {
    crate::props::row(ui, text, |ui| {
        crate::props::fill(ui, crate::units::drag(v).range(lo..=hi)).changed()
    })
}

/// The model's levels, lowest first.
fn levels(model: &Model) -> Vec<(ElementId, String, f64)> {
    let mut v: Vec<_> = model
        .levels()
        .map(|(e, l)| (e.id, e.name.clone(), l.elevation))
        .collect();
    v.sort_by(|a, b| a.2.total_cmp(&b.2));
    v
}

fn name_of(model: &Model, id: ElementId) -> String {
    model
        .get(id)
        .map_or("(missing)".to_string(), |e| e.name.clone())
}

/// Which side a wall's exterior is on, in words.
pub(super) fn exterior_side(w: &Wall) -> &'static str {
    if w.flip {
        "Exterior on the left (looking from start to end)"
    } else {
        "Exterior on the right (looking from start to end)"
    }
}

/// A wall's location line, flip, constraints, joins and attached top.
/// Returns the edit's name when something changed.
pub(super) fn wall_fields(
    ui: &mut egui::Ui,
    model: &Model,
    id: ElementId,
    w: &mut Wall,
) -> &'static str {
    let mut label = "";
    crate::props::flow(ui, |ui| {
        crate::app::helpui::combo(ui, "Location line")
            .selected_text(crate::i18n::tw(walls::location_label(model, w)))
            .width(ui.available_width().min(130.0))
            .show_ui(ui, |ui| {
                for l in LocationLine::ALL {
                    if ui.selectable_value(&mut w.location, l, crate::i18n::tw(l.label())).changed() {
                        label = "Wall location line"
                    }
                }
                let lines = walls::layer_boundaries(model, w);
                if !lines.is_empty() {
                    ui.separator();
                    ui.weak(crate::i18n::t("By a layer boundary"));
                    for (l, name) in lines {
                        if ui
                            .selectable_value(&mut w.location, l, crate::i18n::tw(name))
                            .on_hover_text(crate::i18n::t("Draw and dimension the wall on this line between layers. The wall stays where it is."))
                            .changed()
                        {
                            label = "Wall location line"
                        }
                    }
                }
            })
            .response
            .on_hover_text(
                crate::i18n::t("The line the wall is drawn and dimensioned on. It stays put when the wall gets thicker, changes its layers or is flipped."),
            );
    });
    // Blender's rows (MU32): the operator in the field column, what it
    // did said under it
    if crate::props::action(
        ui,
        "Faces",
        "Flip inside / outside",
        "Swap the wall's exterior and interior faces, about its location line (Space). Its doors and windows turn with it.",
        true,
    )
    .clicked()
    {
        walls::flip_wall(w);
        label = "Flip wall"
    }
    crate::props::note(ui, exterior_side(w));

    ui.add_space(4.0);
    let lv = levels(model);
    crate::props::flow(ui, |ui| {
        crate::app::helpui::combo(ui, "Base level")
            .selected_text(crate::i18n::tw(name_of(model, w.level)))
            .width(ui.available_width().min(130.0))
            .show_ui(ui, |ui| {
                for (l, n, _) in &lv {
                    if ui
                        .selectable_value(&mut w.level, *l, crate::i18n::tw(n))
                        .changed()
                    {
                        label = "Wall base level"
                    }
                }
            });
    });
    if num(ui, "Base offset", &mut w.base_offset, -10.0, 50.0) {
        label = "Wall base offset"
    }
    let shown = match w.top {
        WallTop::Unconnected => "Unconnected".to_string(),
        WallTop::UpToLevel { level, .. } => format!("Up to {}", name_of(model, level)),
    };
    crate::props::flow(ui, |ui| {
        crate::app::helpui::combo(ui, "Top")
            .selected_text(crate::i18n::tw(shown))
            .width(ui.available_width().min(130.0))
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(w.top.is_unconnected(), crate::i18n::tw("Unconnected"))
                    .clicked()
                    && !w.top.is_unconnected()
                {
                    w.top = WallTop::Unconnected;
                    label = "Wall top constraint"
                }
                for (l, n, _) in &lv {
                    let on = matches!(w.top, WallTop::UpToLevel { level, .. } if level == *l);
                    if ui.selectable_label(on, crate::i18n::tw(format!("Up to {n}"))).clicked() && !on {
                        w.top = WallTop::UpToLevel {
                            level: *l,
                            offset: 0.0,
                        };
                        label = "Wall top constraint"
                    }
                }
            })
            .response
            .on_hover_text(
                crate::i18n::t("Unconnected: the wall is as tall as its height. Up to a level: its top follows that level when it moves."),
            );
    });
    match &mut w.top {
        WallTop::UpToLevel { offset, .. } => {
            if num(ui, "Top offset", offset, -50.0, 50.0) {
                label = "Wall top offset"
            }
            let h = walls::constrained_height(model, w).unwrap_or(w.height);
            ui.label(
                egui::RichText::new(crate::i18n::tr(&format!(
                    "Height {}, from its top constraint",
                    crate::units::len(h.max(walls::MIN_HEIGHT))
                )))
                .weak(),
            );
        }
        WallTop::Unconnected => {
            if num(ui, "Height", &mut w.height, 0.1, 50.0) {
                label = "Wall height"
            }
        }
    }

    // load-bearing or not: what the masonry check measures it by
    let auto = creabim_core::masonry::is_bearing(
        model,
        &Wall {
            bearing: None,
            ..w.clone()
        },
        &creabim_core::masonry::rules(model),
    );
    let says = |b: Option<bool>| match b {
        None if auto => "Auto: load-bearing (by its core)",
        None => "Auto: partition (by its core)",
        Some(true) => "Load-bearing",
        Some(false) => "Partition",
    };
    crate::props::row(ui, "Masonry", |ui| {
        egui::ComboBox::from_id_salt(("wall.bearing", id))
            .icon(crate::props::combo_icon)
            .selected_text(crate::i18n::tr(says(w.bearing)))
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for b in [None, Some(true), Some(false)] {
                    if ui
                        .selectable_value(&mut w.bearing, b, crate::i18n::tr(says(b)))
                        .changed()
                    {
                        label = "Wall load-bearing";
                    }
                }
            });
    });

    // 4.1.3 of STN 92 0201-4 (Z3): an outer wall with a combustible face
    if crate::props::switch(ui, &mut w.fire_partial, "Combustible outer face")
        .on_hover_text(crate::i18n::t(
            "The wall meets its fire resistance, but its outer face can give off more than 100 MJ/m² in a fire: a partly fire-open area in the separation distance (STN 92 0201-4, 4.1.3)",
        ))
        .changed()
    {
        label = "Wall fire-open face";
    }

    // no ETICS fire barriers in this wall (a gable without openings)
    if crate::props::switch(ui, &mut w.no_fire_barriers, "No fire barriers")
        .on_hover_text(crate::i18n::t(
            "Leave the automatic ETICS fire barriers out of this wall, as on a gable wall without openings (Project information › ETICS fire barriers)",
        ))
        .changed()
    {
        label = "Wall fire barriers";
    }

    ui.add_space(4.0);
    crate::props::row(ui, "Joins at", |ui| {
        crate::props::flow(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 1.0;
            let h = ui.spacing().interact_size.y;
            let bw = ((ui.available_width() - 1.0) / 2.0).max(40.0);
            for (i, end) in ["Start", "End"].into_iter().enumerate() {
                let on = !w.disallow_join[i];
                if ui
                    .add_sized([bw, h], egui::Button::selectable(on, end))
                    .on_hover_text(crate::i18n::t("Off: this end stays square and free, and is not pulled along when the wall it touches moves. Right-click the end in plan to switch it too."))
                    .clicked()
                {
                    w.disallow_join[i] = on;
                    label = if on { "Disallow wall join" } else { "Allow wall join" }
                }
            }
        })
    });

    if w.attach_top.is_empty() {
        let found = walls::attach_candidates(model, id);
        let tip = if found.is_empty() {
            "There is no roof or slab above this wall."
        } else {
            "The top follows the underside of the roof or slab above, cut to its slope."
        };
        if crate::props::action(ui, "Top", "Attach to roof / slab", tip, !found.is_empty())
            .clicked()
        {
            w.attach_top = found;
            label = "Attach wall top"
        }
    } else {
        let names: Vec<String> = w.attach_top.iter().map(|a| name_of(model, *a)).collect();
        if crate::props::action(
            ui,
            "Top",
            "Detach",
            "The top goes back to the wall's own height.",
            true,
        )
        .clicked()
        {
            w.attach_top.clear();
            label = "Detach wall top"
        }
        crate::props::note(ui, format!("Attached to {}", names.join(", ")));
    }

    crate::props::row(ui, "Length", |ui| ui.label(crate::units::len(w.length())));
    if w.is_arc() {
        // a curved wall's radius: its ends stay, it bends more or less
        let chord = w.start.distance(w.end);
        let creabim_core::sketch::Curve::Arc { radius, sweep, .. } = w.curve() else {
            unreachable!()
        };
        let mut r = radius;
        if num(ui, "Radius", &mut r, chord / 2.0 + 1e-3, 1e4) {
            let minor = 2.0 * (chord / (2.0 * r)).clamp(-1.0, 1.0).asin();
            // the side it bulges to, and more or less than a half circle, stay
            let s = if sweep.abs() > std::f64::consts::PI {
                std::f64::consts::TAU - minor
            } else {
                minor
            };
            w.bulge = w.bulge.signum() * (s / 4.0).tan();
            label = "Change wall radius"
        }
    }
    let (mut a, mut b) = walls::location_line(model, w);
    let moved_a = num(ui, "Start X", &mut a.x, -1e4, 1e4) | num(ui, "Start Y", &mut a.y, -1e4, 1e4);
    let moved_b = num(ui, "End X", &mut b.x, -1e4, 1e4) | num(ui, "End Y", &mut b.y, -1e4, 1e4);
    if moved_a || moved_b {
        walls::set_location_line(model, w, a, b);
        label = if moved_a {
            "Move wall start"
        } else {
            "Move wall end"
        }
    }
    label
}

/// A door's size: its leaf's, the opening in the wall following from
/// it and the frame. Returns the edit's name.
pub(super) fn leaf_fields(ui: &mut egui::Ui, o: &mut Opening) -> &'static str {
    use creabim_core::joinery::{self, DoorFrame, DoorSpec};
    let t = crate::i18n::t;
    let mut d = o
        .door
        .clone()
        .unwrap_or_else(|| DoorSpec::fitting(DoorFrame::Lining, o.width, o.height));
    let mut changed = false;
    changed |= crate::props::row(ui, "Leaf width", |ui| {
        crate::props::fill(ui, crate::units::drag(&mut d.nom_width).range(0.3..=5.0))
            .on_hover_text(t("The leaf's nominal width, without the frame: 600, 700, 800, 900 mm. The opening in the wall follows from it and the frame."))
            .changed()
    });
    changed |= crate::props::row(ui, "Leaf height", |ui| {
        crate::props::fill(ui, crate::units::drag(&mut d.nom_height).range(1.0..=5.0)).changed()
    });
    let (ow, oh) = d.opening();
    crate::props::row(ui, "Structural opening", |ui| {
        ui.label(format!("{:.0} × {:.0} mm", ow * 1000.0, oh * 1000.0))
    });
    if changed {
        joinery::apply_door(o, d);
        return "Door leaf and frame";
    }
    ""
}

/// A door as joinery (its leaf's nominal size, its frame, its fire
/// rating), where a door's or window's frame stands across its wall,
/// how far the reveal laps it, and its division with the button that
/// opens the editor (`edit` set). Returns the edit's name.
pub(super) fn seat_fields(
    ui: &mut egui::Ui,
    model: &Model,
    o: &mut Opening,
    edit: &mut bool,
) -> &'static str {
    use creabim_core::joinery::{self, DoorFrame, DoorMaterial, DoorSpec, FireDoor};
    let mut label = "";
    if o.kind == OpeningKind::Hole {
        return label;
    }
    let Some(he) = model.get(o.host) else {
        return label;
    };
    let ElementKind::Wall(host) = &he.kind else {
        return label;
    };
    let asm = creabim_core::assembly_of(model, he).map(|x| x.1);
    let t = crate::i18n::t;
    if o.kind == OpeningKind::Door {
        ui.separator();
        ui.label(egui::RichText::new(t("Door leaf and frame")).strong());
        // what it is now, or what its opening would hold
        let mut d = o
            .door
            .clone()
            .unwrap_or_else(|| DoorSpec::fitting(DoorFrame::Lining, o.width, o.height));
        let mut changed = false;
        changed |= crate::props::row(ui, "Door frame", |ui| {
            let mut c = false;
            egui::ComboBox::from_id_salt("door_frame")
                .icon(crate::props::combo_icon)
                .selected_text(crate::i18n::tw(d.frame.label()))
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for x in DoorFrame::ALL {
                        c |= ui
                            .selectable_value(&mut d.frame, x, crate::i18n::tw(x.label()))
                            .changed();
                    }
                });
            c
        });
        changed |= crate::props::row(ui, "Leaf material", |ui| {
            let mut c = false;
            egui::ComboBox::from_id_salt("door_leaf")
                .icon(crate::props::combo_icon)
                .selected_text(crate::i18n::tw(d.leaf.label()))
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for x in DoorMaterial::ALL {
                        c |= ui
                            .selectable_value(&mut d.leaf, x, crate::i18n::tw(x.label()))
                            .changed();
                    }
                });
            c
        });
        // a plastic frame or leaf in its RAL colour: the door's material,
        // from the library's plastics (George: as the lists and 3D show it)
        if d.frame == DoorFrame::Plastic || d.leaf == DoorMaterial::Plastic {
            let current = o
                .material
                .and_then(|m| model.get(m))
                .map(|e| e.name.clone());
            let plastic = o.material.and_then(|m| model.get(m)).is_some_and(
                |e| matches!(&e.kind, ElementKind::Material(m) if m.category == "plastic"),
            );
            let mut pick = None;
            crate::props::row(ui, "Colour (RAL)", |ui| {
                egui::ComboBox::from_id_salt("door_ral")
                    .icon(crate::props::combo_icon)
                    .selected_text(if plastic {
                        crate::i18n::tw(current.as_deref().unwrap_or(""))
                    } else {
                        crate::i18n::tw("Pick a colour")
                    })
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for e in creabim_core::library::LIBRARY
                            .iter()
                            .filter(|e| e.category == "plastic")
                        {
                            let on = current.as_deref() == Some(e.name);
                            if ui.selectable_label(on, crate::i18n::tw(e.name)).clicked() {
                                pick = Some(e.name);
                            }
                        }
                    });
                false
            });
            // a door just made plastic starts white
            let fresh = !plastic
                && o.door.as_ref().is_some_and(|x| {
                    x.frame != DoorFrame::Plastic && x.leaf != DoorMaterial::Plastic
                });
            if let Some(name) = pick.or(fresh.then_some(PVC_WHITE)) {
                ui.data_mut(|m| m.insert_temp(egui::Id::new(PVC_PICK), name));
            }
        }
        let mut fire = d.fire.is_some();
        if crate::props::row(ui, "Fire door", |ui| ui.checkbox(&mut fire, "").changed()) {
            d.fire = fire.then(FireDoor::default);
            changed = true;
        }
        if let Some(f) = &mut d.fire {
            changed |= crate::props::row(ui, "Fire rating", |ui| {
                let mut c = false;
                egui::ComboBox::from_id_salt("door_fire")
                    .icon(crate::props::combo_icon)
                    .selected_text(f.rating.clone())
                    .width(ui.available_width())
                    .show_ui(ui, |ui| {
                        for x in joinery::FIRE_RATINGS {
                            c |= ui
                                .selectable_value(&mut f.rating, x.to_string(), x)
                                .changed();
                        }
                    });
                c
            });
            changed |= crate::props::row(ui, "Closer (C)", |ui| {
                ui.checkbox(&mut f.self_closing, "").changed()
            });
            changed |= crate::props::row(ui, "Smoke tight (S)", |ui| {
                ui.checkbox(&mut f.smoke_tight, "").changed()
            });
        }
        if changed {
            joinery::apply_door(o, d);
            label = "Door leaf and frame";
        } else if o.door.is_some()
            && crate::props::row(ui, "", |ui| {
                ui.button(crate::i18n::t("Plain door"))
                    .on_hover_text(t("Take the leaf's nominal size, frame and fire rating off: the door is its opening again."))
                    .clicked()
            })
        {
            o.door = None;
            label = "Door leaf and frame";
        }
    }
    if performance_fields(
        ui,
        &mut o.performance,
        o.kind == OpeningKind::Door,
        "opening",
    ) {
        label = "Window or door values";
    }
    ui.separator();
    ui.label(egui::RichText::new(t("Seating in the wall")).strong());
    if o.is_lining() {
        ui.label(t("A lining runs through the whole wall."));
    } else {
        let mut sb = o.setback_in(host, asm);
        let changed = crate::props::row(ui, "Setback from the core", |ui| {
            crate::props::fill(ui, crate::units::drag(&mut sb).range(-1.0..=2.0))
                .on_hover_text(t("How far the frame's outer face stands in from the outer edge of the wall's load-bearing layers; less than nothing moves it out into the insulation. In a wall with no layers it is measured from the wall's outside face."))
                .changed()
        });
        if changed {
            o.setback = Some(sb);
            label = "Opening setback";
        }
    }
    if o.kind == OpeningKind::Window {
        let mut lap = o
            .reveal
            .unwrap_or(creabim_core::assembly::REVEAL_INSULATION);
        let changed = crate::props::row(ui, "Reveal overlap", |ui| {
            crate::props::fill(ui, crate::units::drag(&mut lap).range(0.0..=0.3))
                .on_hover_text(t("How far the reveal's insulation laps over the frame."))
                .changed()
        });
        if changed {
            o.reveal = Some(lap);
            label = "Reveal overlap";
        }
    }
    // what the window or door list says of it in words
    ui.separator();
    ui.label(
        egui::RichText::new(t(if o.kind == OpeningKind::Door {
            "In the door list"
        } else {
            "In the window list"
        }))
        .strong(),
    );
    let mut texts = o.texts.clone().unwrap_or_default();
    let mut changed = false;
    for (text, v, hint) in [
        ("Execution", &mut texts.execution, "How it is made: profiles, system, leaves. Left empty, the list writes what the model tells."),
        ("Finish", &mut texts.finish, "Its surface: colour, texture. Left empty, the list writes its material."),
        ("Note", &mut texts.note, "Glazing, fittings, anything else. Left empty, a door's frame is named."),
    ] {
        changed |= crate::props::row(ui, text, |ui| {
            ui.add(
                egui::TextEdit::multiline(v)
                    .desired_rows(2)
                    .desired_width(f32::INFINITY),
            )
            .on_hover_text(t(hint))
            .changed()
        });
    }
    if changed {
        o.texts = (!texts.is_empty()).then_some(texts);
        label = "Joinery list texts";
    }
    ui.separator();
    ui.label(egui::RichText::new(t("Division and opening")).strong());
    crate::props::row(ui, "Fields", |ui| ui.label(joinery::operation(o)));
    if crate::props::row(ui, "", |ui| {
        ui.button(t("Edit the division…"))
            .on_hover_text(t(
                "Open the editor: split it by mullions and transoms and set how each field opens.",
            ))
            .clicked()
    }) {
        *edit = true;
    }
    label
}

/// A door's hand and swing, or a window's facing. Returns the edit's name.
/// Where the door fields leave the plastic colour picked, for the
/// properties to take from the library and give the door.
pub(super) const PVC_PICK: &str = "door.pvc_pick";
/// The colour a door made plastic starts in.
const PVC_WHITE: &str = "PVC, RAL 9016 white";

pub(super) fn opening_fields(ui: &mut egui::Ui, model: &Model, o: &mut Opening) -> &'static str {
    let mut label = "";
    let host = match model.get(o.host).map(|e| &e.kind) {
        Some(ElementKind::Wall(w)) => w.clone(),
        _ => return label,
    };
    match o.kind {
        OpeningKind::Door => {
            // a label-column row, then two equal buttons, as Blender (MU29)
            crate::props::row(ui, "Hung", |ui| {
                ui.label(crate::i18n::t(creabim_geom::opening::door_hand(o, &host)))
                    .on_hover_text(crate::i18n::t("The hinge side seen from the side it opens to, and whether it opens into the building or out of it."))
            });
            // flipped by the arrows beside it in the plan (as Revit),
            // or Space and Shift+Space
            if crate::props::switch(ui, &mut o.threshold, "Threshold")
                .on_hover_text(crate::i18n::t(
                    "A threshold between the jambs: drawn in plans and 3D",
                ))
                .changed()
            {
                label = "Threshold"
            }
        }
        OpeningKind::Window => {
            // the exterior is on the wall's left when it is flipped
            let out = o.faces_left(&host) == host.flip;
            // the same rows as a door's (MU29)
            crate::props::row(ui, "Outside", |ui| {
                ui.label(crate::i18n::t(if out {
                    "faces the exterior"
                } else {
                    "turned to the interior"
                }))
            });
            // turned round by the arrow beside it in the plan, or Space
            // its sills: the outer one of sheet metal (with its mark and
            // its row in the list of sheet-metal work), the inner board
            let (mut outer, mut inner) = (!o.no_outer_sill, !o.no_inner_sill);
            if crate::props::switch(ui, &mut outer, "Outer sill")
                .on_hover_text(crate::i18n::t(
                    "The sheet-metal sill outside: drawn in plans and 3D, marked K/n beside the window's mark and counted in the list of sheet-metal work",
                ))
                .changed()
            {
                o.no_outer_sill = !outer;
                label = "Outer sill"
            }
            if crate::props::switch(ui, &mut inner, "Inner sill")
                .on_hover_text(crate::i18n::t(
                    "The sill board inside: drawn in plans and 3D",
                ))
                .changed()
            {
                o.no_inner_sill = !inner;
                label = "Inner sill"
            }
        }
        OpeningKind::Hole => {
            ui.label(crate::i18n::t(
                "A bare opening: a hole through the wall with no door or window in it",
            ));
        }
    }
    label
}

impl CreaBimApp {
    /// Flip the selected walls, doors and windows: a wall's faces, a door's
    /// swing (or, with `hand`, its hinge side), a window's facing.
    pub(super) fn flip_selected(&mut self, hand: bool) {
        let model = self.doc.model();
        let mut tx = Transaction::new(if hand { "Flip door hand" } else { "Flip" });
        let mut n = 0;
        for id in self.selection.all() {
            let Some(before) = model.get(id) else {
                continue;
            };
            // a section or elevation looks the other way, its notes mirrored
            if let (false, Some(flip)) = (hand, creabim_core::transform::flip_view(model, id)) {
                tx.ops.extend(flip.ops);
                n += 1;
                continue;
            }
            let mut after = before.clone();
            let changed = match &mut after.kind {
                ElementKind::Wall(w) if !hand => {
                    walls::flip_wall(w);
                    true
                }
                ElementKind::Opening(o) if hand => {
                    let door = o.kind == OpeningKind::Door;
                    if door {
                        walls::flip_hand(o);
                    }
                    door
                }
                ElementKind::Opening(o) => {
                    walls::flip_facing(o);
                    true
                }
                // furniture turns round about its middle
                ElementKind::Object(_) if !hand => {
                    crate::handles::turn_object(model, &mut after, 180.0);
                    true
                }
                _ => false,
            };
            if changed {
                tx = tx.replace(before.clone(), after);
                n += 1;
            }
        }
        if n == 0 {
            self.status = if hand {
                "Select a door to flip its hand.".into()
            } else {
                "Select a wall, door, window, section or elevation to flip it.".into()
            };
            return;
        }
        let r = self.doc.commit(tx);
        self.report(
            r,
            format!("Flipped {n} element{}.", if n == 1 { "" } else { "s" }),
        );
    }

    /// Allow or disallow the join at a wall's end (0 start, 1 end).
    pub(super) fn toggle_join(&mut self, id: ElementId, end: usize) {
        let Some(ElementKind::Wall(w)) = self.doc.model().get(id).map(|e| &e.kind) else {
            return;
        };
        let allow = w.disallow_join[end];
        let label = if allow {
            "Allow wall join"
        } else {
            "Disallow wall join"
        };
        let r = self.doc.edit(id, label, |e| {
            if let ElementKind::Wall(w) = &mut e.kind {
                w.disallow_join[end] = !allow;
            }
        });
        let which = if end == 0 { "start" } else { "end" };
        self.report(
            r,
            if allow {
                format!("The wall's {which} joins again.")
            } else {
                format!("The wall's {which} is free: it stays square and is not pulled along.")
            },
        );
    }

    /// Attach the selected walls' tops to the roof or slab above them, or
    /// detach them.
    pub(super) fn attach_selected(&mut self, attach: bool) {
        let model = self.doc.model();
        let mut tx = Transaction::new(if attach {
            "Attach wall top"
        } else {
            "Detach wall top"
        });
        let mut n = 0;
        for id in self.selection.all() {
            let Some(before) = model.get(id) else {
                continue;
            };
            let ElementKind::Wall(w) = &before.kind else {
                continue;
            };
            let to = if attach {
                walls::attach_candidates(model, id)
            } else {
                vec![]
            };
            if to == w.attach_top || (attach && to.is_empty()) {
                continue;
            }
            let mut after = before.clone();
            if let ElementKind::Wall(w) = &mut after.kind {
                w.attach_top = to;
            }
            tx = tx.replace(before.clone(), after);
            n += 1;
        }
        if n == 0 {
            self.status = if attach {
                "Select walls with a roof or slab above them.".into()
            } else {
                "Select walls whose tops are attached.".into()
            };
            return;
        }
        let r = self.doc.commit(tx);
        let what = if attach {
            "attached to the roof or slab above"
        } else {
            "detached"
        };
        self.report(
            r,
            format!("{n} wall top{} {what}.", if n == 1 { "" } else { "s" }),
        );
    }
}

/// The wall end nearer `p`: 0 its start, 1 its end.
pub(super) fn nearest_end(w: &Wall, p: DVec2) -> usize {
    usize::from(p.distance(w.end) < p.distance(w.start))
}

impl CreaBimApp {
    /// The right-click menu's wall, door and window items, for the
    /// selection and where the menu was opened.
    pub(super) fn wall_menu_items(&self) -> Vec<crate::ui_model::Item> {
        use crate::ui_model::Item;
        let model = self.doc.model();
        let mut items = vec![];
        let sel: Vec<&Element> = self
            .selection
            .all()
            .into_iter()
            .filter_map(|id| model.get(id))
            .collect();
        let walls: Vec<(ElementId, &Wall)> = sel
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Wall(w) => Some((e.id, w)),
                _ => None,
            })
            .collect();
        let doors = sel
            .iter()
            .any(|e| matches!(&e.kind, ElementKind::Opening(o) if o.kind == OpeningKind::Door));
        let openings = sel
            .iter()
            .any(|e| matches!(&e.kind, ElementKind::Opening(_)));
        if !walls.is_empty() || openings {
            let what = match (walls.is_empty(), doors) {
                (false, _) => "Flip wall",
                (true, true) => "Flip door swing",
                (true, false) => "Flip window inside/outside",
            };
            items.push(Item::new("wall.flip", what).key("Space").icon("brick-wall"));
        }
        let views = sel.iter().any(|e| {
            matches!(&e.kind, ElementKind::View(v) if matches!(v.kind, ViewKind::Section { .. }))
        });
        if views && walls.is_empty() && !openings {
            items.push(
                Item::new("wall.flip", "Flip view direction")
                    .key("Space")
                    .icon("flip-horizontal-2")
                    .hint("Look the other way along the same line"),
            );
        }
        if doors {
            items.push(
                Item::new("wall.hand", "Flip door hand")
                    .key("Shift+Space")
                    .icon("door-open"),
            );
        }
        // the join at the end nearest the click, for one wall
        let at = if self.shell.context_3d {
            self.menu_at_3d
        } else {
            self.plan.menu_at
        };
        if let ([(id, w)], Some(at)) = (walls.as_slice(), at) {
            let end = nearest_end(w, at.truncate());
            let which = if end == 0 { "start" } else { "end" };
            let text = if w.disallow_join[end] {
                format!("Allow join at its {which}")
            } else {
                format!("Disallow join at its {which}")
            };
            items.push(
                Item::new(format!("wall.join:{}:{end}", id.0), text)
                    .icon("link")
                    .hint("A disallowed end stays square and free: it doesn't join, and isn't pulled along"),
            );
            // the corner at that end, if it meets one wall there
            let model = self.doc.model();
            if let Some(other) = creabim_core::corner_partner(model, *id, end == 1) {
                let rule = creabim_core::joins::rule_for(model, *id, other).map(|(_, r)| r);
                let now = creabim_core::CornerPreset::of(rule.as_ref(), *id);
                let presets = creabim_core::CornerPreset::ALL
                    .iter()
                    .enumerate()
                    .map(|(k, p)| {
                        Item::new(format!("wall.corner:{}:{end}:{k}", id.0), p.label())
                            .checked(*p == now)
                    })
                    .collect();
                items.push(Item::sub("Corner join", presets).icon("link"));
            }
            items.push(
                Item::new(format!("wall.split:{}", id.0), "Split wall here")
                    .icon("scissors")
                    .hint("Two walls, meeting where you right-clicked; doors and windows stay where they are"),
            );
        }
        if !walls.is_empty() {
            // as in Properties: only when there is a roof or slab overhead
            let overhead = walls
                .iter()
                .any(|(id, _)| !walls::attach_candidates(model, *id).is_empty());
            items.push(
                Item::new("wall.attach", "Attach top to roof/slab")
                    .enabled(overhead)
                    .hint(if overhead {
                        "The top follows the underside of the roof or slab above"
                    } else {
                        "There is no roof or slab above this wall"
                    }),
            );
            if walls.iter().any(|(_, w)| !w.attach_top.is_empty()) {
                items.push(Item::new("wall.detach", "Detach top"));
            }
        }
        if !items.is_empty() {
            items.push(Item::Separator);
        }
        items
    }

    /// Run a wall item picked from a menu or the tool strip; false when
    /// `head` is not one.
    pub(super) fn wall_action(&mut self, head: &str, arg: &str) -> bool {
        match head {
            "wall.flip" => self.flip_selected(false),
            "wall.hand" => self.flip_selected(true),
            "wall.attach" => self.attach_selected(true),
            "wall.split" => {
                let at = if self.shell.context_3d {
                    self.menu_at_3d
                } else {
                    self.plan.menu_at
                };
                let id = self
                    .doc
                    .model()
                    .iter()
                    .find(|e| e.id.0.to_string() == arg)
                    .map(|e| e.id);
                let r = match (id, at) {
                    (Some(id), Some(at)) => {
                        creabim_core::walls::split_wall(self.doc.model(), id, at.truncate())
                            .and_then(|tx| self.doc.commit(tx).map_err(|e| e.to_string()))
                    }
                    _ => Err("right-click the wall where it should split".into()),
                };
                self.report(r, "Split the wall in two. Ctrl+Z to undo.");
            }
            "wall.detach" => self.attach_selected(false),
            "wall.corner" => {
                let mut parts = arg.split(':');
                let (id, end, k) = (parts.next(), parts.next(), parts.next());
                let id = self
                    .doc
                    .model()
                    .iter()
                    .find(|e| Some(e.id.0.to_string().as_str()) == id)
                    .map(|e| e.id);
                let preset = k
                    .and_then(|k| k.parse::<usize>().ok())
                    .and_then(|k| creabim_core::CornerPreset::ALL.get(k).copied());
                if let (Some(id), Some(p)) = (id, preset) {
                    let at_end = end == Some("1");
                    match creabim_core::corner_partner(self.doc.model(), id, at_end) {
                        Some(other) => {
                            let tx = creabim_core::joins::set_rule(
                                self.doc.model(),
                                id,
                                other,
                                "Corner join",
                                |r| p.apply(r, id),
                            );
                            let r = self.doc.commit(tx);
                            self.report(
                                r,
                                format!("Corner: {}. Ctrl+Z to undo.", p.label().to_lowercase()),
                            );
                        }
                        None => self.status = "That end meets no one wall.".into(),
                    }
                }
            }
            "wall.join" => {
                let (id, end) = arg.rsplit_once(':').unwrap_or((arg, "0"));
                let id = self
                    .doc
                    .model()
                    .iter()
                    .find(|e| e.id.0.to_string() == id)
                    .map(|e| e.id);
                if let Some(id) = id {
                    self.toggle_join(id, usize::from(end == "1"));
                }
            }
            "wall.loc" => {
                if let Some(l) = arg
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| LocationLine::ALL.get(i))
                {
                    // the type picked for new walls stays: the line they
                    // are drawn on is the tool's own choice
                    self.plan.wall_location = *l;
                    self.status =
                        format!("New walls are drawn on their {}.", l.label().to_lowercase());
                }
            }
            "wall.shape" => {
                self.plan.wall_shape = arg.parse().unwrap_or(0);
                self.plan.pending = None;
                self.plan.wall_arc_end = None;
                self.status = match self.plan.wall_shape {
                    1 => "Arc wall: click its start, its end, then a point it passes through.",
                    2 => "Circle wall: click its centre, then a point on it.",
                    3 => "Rectangle of walls: click a corner, then the opposite one.",
                    _ => "Straight walls: click to start, click for each corner.",
                }
                .into();
            }
            "tempdims.ref" => {
                if let Some(r) = arg
                    .parse::<usize>()
                    .ok()
                    .and_then(|i| creabim_core::walls::WallRef::ALL.get(i))
                {
                    self.plan.temp_ref = *r;
                    self.shell.settings.temp_ref = *r;
                    self.shell.settings.store();
                    use creabim_core::walls::WallRef as R;
                    self.status = match r {
                        R::Face => "Temporary dimensions measure walls from their faces.",
                        R::CoreFace => {
                            "Temporary dimensions measure walls from the faces of their load-bearing core."
                        }
                        R::CoreCentre => {
                            "Temporary dimensions measure walls from the middle of their core."
                        }
                        R::Centre => "Temporary dimensions measure walls from their centrelines.",
                    }
                    .into();
                }
            }
            "view.reno" => {
                use creabim_core::reno::RenoView;
                let Some(id) = self.active_view else {
                    return true;
                };
                let Some(mut v) = self.view(id).cloned() else {
                    return true;
                };
                let i = arg.parse::<usize>().unwrap_or(0);
                v.reno = i.checked_sub(1).and_then(|i| RenoView::ALL.get(i).copied());
                let r = self.doc.edit_merged(id, "Renovation stage", |e| {
                    e.kind = ElementKind::View(v.clone())
                });
                let msg = match v.reno {
                    None => "Renovation off: everything drawn as usual.",
                    Some(RenoView::Existing) => {
                        "Existing state: what stands now, the elements to keep and to take down."
                    }
                    Some(RenoView::Demolition) => {
                        "Demolition: what is taken down in yellow, with the holes for new doors and windows."
                    }
                    Some(RenoView::New) => "New state: what stays and what is built, the new in red.",
                    Some(RenoView::All) => "All stages at once: demolished in yellow, new in red.",
                };
                self.report(r, crate::i18n::tr(msg));
            }
            "wall.flipnew" => {
                self.plan.wall_flip = !self.plan.wall_flip;
                self.status = format!(
                    "New walls have their exterior on the {} (Space flips it).",
                    if self.plan.wall_flip { "left" } else { "right" }
                );
            }
            _ => return false,
        }
        true
    }

    /// What temporary dimensions measure walls from, for the view strip.
    pub(super) fn temp_ref_items(&self) -> Vec<crate::ui_model::Item> {
        creabim_core::walls::WallRef::ALL
            .iter()
            .enumerate()
            .map(|(i, r)| {
                crate::ui_model::Item::new(format!("tempdims.ref:{i}"), r.label())
                    .checked(self.plan.temp_ref == *r)
            })
            .collect()
    }

    /// Which renovation stage the view shows, for the view strip.
    pub(super) fn reno_items(&self) -> Vec<crate::ui_model::Item> {
        use creabim_core::reno::RenoView;
        let now = self
            .active_view
            .and_then(|id| self.view(id))
            .and_then(|v| v.reno);
        std::iter::once(crate::ui_model::Item::new("view.reno:0", "Off").checked(now.is_none()))
            .chain(RenoView::ALL.iter().enumerate().map(|(i, r)| {
                crate::ui_model::Item::new(format!("view.reno:{}", i + 1), r.label())
                    .checked(now == Some(*r))
            }))
            .collect()
    }

    /// The Wall tool's location line choices, for its option strip.
    pub(super) fn wall_location_items(&self) -> Vec<crate::ui_model::Item> {
        LocationLine::ALL
            .iter()
            .enumerate()
            .map(|(i, l)| {
                crate::ui_model::Item::new(format!("wall.loc:{i}"), l.label())
                    .checked(self.plan.wall_location == *l)
            })
            .collect()
    }
}

/// A window's or door's heat transfer, solar factor, light transmittance
/// and sound reduction (Uw, g, τv, Rw): picked as a maker's product from
/// the app's data sheets, each value then typed over if need be. `salt`
/// keeps two of these apart. Returns whether anything changed.
pub(super) fn performance_fields(
    ui: &mut egui::Ui,
    perf: &mut Option<creabim_core::joinery::Performance>,
    door: bool,
    salt: &str,
) -> bool {
    use creabim_core::joinery::{self, Performance};
    let t = crate::i18n::t;
    let mut changed = false;
    ui.separator();
    ui.label(egui::RichText::new(t("Heat, light and sound")).strong())
        .on_hover_text(t("Uw, g, τv and Rw as the maker's data sheet gives them. They go to the data standard (DSS) and the IFC file."));
    let shown = perf
        .as_ref()
        .map(|p| p.product.clone())
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| t("Not given").to_string());
    crate::props::row(ui, "Product", |ui| {
        egui::ComboBox::from_id_salt(("joinery_product", salt))
            .icon(crate::props::combo_icon)
            .selected_text(shown)
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                if ui
                    .selectable_label(perf.is_none(), t("Not given"))
                    .clicked()
                {
                    *perf = None;
                    changed = true;
                }
                // the makers' products for this kind first, then the others
                let mut list: Vec<&joinery::Preset> = joinery::presets().iter().collect();
                list.sort_by_key(|p| p.door != door);
                for p in list {
                    let on = perf.as_ref().is_some_and(|x| x.product == p.name);
                    if ui
                        .selectable_label(on, &p.name)
                        .on_hover_text(&p.source)
                        .clicked()
                    {
                        *perf = Some(p.performance());
                        changed = true;
                    }
                }
            });
    });
    let mut value = |ui: &mut egui::Ui,
                     label: &str,
                     get: fn(&mut Performance) -> &mut Option<f64>,
                     scale: f64,
                     range: std::ops::RangeInclusive<f64>,
                     unit: &str,
                     decimals: usize| {
        let mut p = perf.clone().unwrap_or_default();
        let had = get(&mut p).is_some();
        let mut v = get(&mut p).unwrap_or(0.0) * scale;
        let mut on = had;
        let c = crate::props::row(ui, label, |ui| {
            ui.horizontal(|ui| {
                let mut c = ui.checkbox(&mut on, "").changed();
                if on {
                    c |= crate::props::fill(
                        ui,
                        egui::DragValue::new(&mut v)
                            .range(range.clone())
                            .speed(0.01 * scale.max(1.0))
                            .max_decimals(decimals)
                            .suffix(unit),
                    )
                    .changed();
                }
                c
            })
            .inner
        });
        if c {
            if on && !had {
                // a value switched on starts from the middle of its range
                v = (range.start() + range.end()) / 2.0;
            }
            *get(&mut p) = on.then_some(v / scale);
            // typed over: no longer the maker's product as it was
            if let Some(pre) = joinery::presets().iter().find(|x| x.name == p.product) {
                if pre.performance() != p {
                    p.product = format!("{} ({})", pre.name, t("edited"));
                }
            }
            *perf = Some(p);
            changed = true;
        }
    };
    value(ui, "Uw", |p| &mut p.uw, 1.0, 0.3..=6.0, " W/(m²K)", 2);
    value(
        ui,
        "Solar factor g",
        |p| &mut p.g,
        100.0,
        0.0..=100.0,
        " %",
        0,
    );
    value(
        ui,
        "Light transmittance τv",
        |p| &mut p.tv,
        100.0,
        0.0..=100.0,
        " %",
        0,
    );
    value(
        ui,
        "Sound reduction Rw",
        |p| &mut p.rw,
        1.0,
        20.0..=60.0,
        " dB",
        0,
    );
    if let Some(limit) = perf
        .as_ref()
        .and_then(|p| joinery::over_requirement(p, door))
    {
        ui.colored_label(
            ui.visuals().warn_fg_color,
            format!(
                "⚠ {} {} W/(m²K)",
                t("Uw is over the most asked for:"),
                format!("{limit:.2}").replace('.', ",")
            ),
        )
        .on_hover_text(&joinery::requirement().source);
    }
    if let Some(src) = perf
        .as_ref()
        .and_then(|p| joinery::presets().iter().find(|x| x.name == p.product))
    {
        crate::props::note(ui, &src.source);
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether the right-click menu's attach item can be picked.
    fn attach_enabled(app: &CreaBimApp) -> bool {
        app.wall_menu_items().iter().any(|i| {
            matches!(i, crate::ui_model::Item::Action { id, enabled: true, .. } if id == "wall.attach")
        })
    }

    #[test]
    fn attach_top_is_offered_only_with_a_roof_or_slab_overhead() {
        let mut app = crate::app::test_app();
        let model = app.doc.model();
        let covered = model
            .iter()
            .find(|e| {
                matches!(e.kind, ElementKind::Wall(_))
                    && !walls::attach_candidates(model, e.id).is_empty()
            })
            .map(|e| e.id)
            .expect("the sample has a wall under its roof");
        app.selection.only(covered);
        assert!(attach_enabled(&app));
        // the same wall far from the house has nothing overhead
        app.doc
            .edit(covered, "Move wall", |e| {
                if let ElementKind::Wall(w) = &mut e.kind {
                    w.start += DVec2::splat(500.0);
                    w.end += DVec2::splat(500.0);
                }
            })
            .unwrap();
        assert!(walls::attach_candidates(app.doc.model(), covered).is_empty());
        assert!(!attach_enabled(&app));
    }

    #[test]
    fn a_location_line_picked_after_a_type_keeps_the_type() {
        use creabim_core::types::{ElementType, WallType};
        use creabim_core::types::{Param, TypeParams};
        use glam::DVec2;
        let mut doc = creabim_core::Document::from_model(creabim_core::sample::sample_house());
        let tloc = LocationLine::Centre;
        let ty = creabim_core::Element::new(
            "Wall 300",
            creabim_core::ElementKind::ElementType(ElementType::new(TypeParams::Wall(WallType {
                thickness: 0.3,
                assembly: None,
                material: None,
                location: tloc,
                height: None,
                curtain: None,
                screen: None,
            }))),
        );
        let tid = ty.id;
        doc.commit(creabim_core::Transaction::new("type").insert(ty))
            .unwrap();
        let model = doc.model().clone();
        let level = model.levels().next().unwrap().0.id;
        let other = LocationLine::ALL.into_iter().find(|l| *l != tloc).unwrap();
        let mut state = crate::planview::PlanState::default();
        state.types.insert(creabim_core::TypeCategory::Wall, tid);
        state.wall_location = other;
        let w = crate::planview::new_wall(&model, level, &state, DVec2::ZERO, DVec2::X * 4.0);
        assert_eq!(w.location, other);
        let e = state.typed(
            &model,
            creabim_core::Element::new("Wall", creabim_core::ElementKind::Wall(w)),
        );
        let r = e.of_type.expect("the wall keeps its type");
        assert_eq!(r.id, tid);
        assert_eq!(r.overrides, vec![Param::Location]);
        // on the type's own line nothing is overridden
        state.wall_location = tloc;
        let w = crate::planview::new_wall(&model, level, &state, DVec2::ZERO, DVec2::X * 4.0);
        let e = state.typed(
            &model,
            creabim_core::Element::new("Wall", creabim_core::ElementKind::Wall(w)),
        );
        assert!(e.of_type.unwrap().overrides.is_empty());
    }
}
