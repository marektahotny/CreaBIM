//! Underfloor heating zones edited in the plan (George 2026-11-28): an
//! edge zone, a place of reduced output or a hole picked with a click
//! inside it, then dragged to move it, a corner dragged to reshape it,
//! an edge zone's dashed line dragged to change its width, Delete to
//! take it away.

use crate::planview::{PlanState, Xf};
use creabim_core::underfloor::Zone;
use creabim_core::{Document, ElementId, ElementKind, Transaction};
use eframe::egui::{self, Color32, Stroke};
use glam::DVec2;

/// What a drag of a picked zone holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Grab {
    /// The whole zone.
    Move,
    /// One of its corners.
    Corner(usize),
    /// An edge zone's dashed line: its width.
    Line,
}

/// A zone picked: the room, which of its zones, and a drag of it under
/// way (what it holds and where it set out).
#[derive(Clone, Debug, Default)]
pub struct Picked {
    pub room: Option<(ElementId, usize)>,
    pub drag: Option<(Grab, DVec2)>,
    /// A drag of a manifold's main route: the manifold, the point (or,
    /// when inserting, the segment) and where it set out.
    pub route: Option<(ElementId, usize, bool, DVec2)>,
}

fn zones_on(doc: &Document, level: ElementId) -> Vec<(ElementId, usize, Zone)> {
    doc.model()
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Room(r) if r.level == level => Some(
                r.ufh
                    .as_ref()?
                    .zones
                    .iter()
                    .enumerate()
                    .map(move |(k, z)| (e.id, k, z.clone())),
            ),
            _ => None,
        })
        .flatten()
        .filter(|(_, _, z)| z.on && z.outline.len() > 2)
        .collect()
}

fn zone_of(doc: &Document, room: ElementId, k: usize) -> Option<Zone> {
    match doc.model().get(room).map(|e| &e.kind) {
        Some(ElementKind::Room(r)) => r.ufh.as_ref()?.zones.get(k).cloned(),
        _ => None,
    }
}

fn seg_dist(p: DVec2, a: DVec2, b: DVec2) -> f64 {
    let d = b - a;
    let t = ((p - a).dot(d) / d.length_squared().max(1e-12)).clamp(0.0, 1.0);
    (a + d * t).distance(p)
}

/// The zone as the drag `g` by `d` leaves it.
pub fn moved(z: &Zone, g: Grab, d: DVec2) -> Zone {
    let mut z = z.clone();
    match g {
        Grab::Move => {
            for p in z.outline.iter_mut() {
                *p += d;
            }
            if let Some(m) = z.mark.as_mut() {
                for p in m.iter_mut() {
                    *p += d;
                }
            }
        }
        Grab::Corner(i) => {
            if let Some(p) = z.outline.get_mut(i) {
                *p += d;
            }
        }
        Grab::Line => {
            if let Some(m) = z.mark.as_mut() {
                let (a, b) = (m[0], m[1]);
                let n = (b - a).perp().normalize_or_zero();
                let s = n * d.dot(n);
                for p in z.outline.iter_mut() {
                    if seg_dist(*p, a, b) < 0.005 {
                        *p += s;
                    }
                }
                m[0] += s;
                m[1] += s;
            }
        }
    }
    z
}

fn commit(
    doc: &mut Document,
    room: ElementId,
    k: usize,
    z: Option<Zone>,
    what: &str,
) -> Option<String> {
    let before = doc.model().get(room)?.clone();
    let mut after = before.clone();
    if let ElementKind::Room(r) = &mut after.kind {
        let u = r.ufh.as_mut()?;
        match z {
            Some(z) => *u.zones.get_mut(k)? = z,
            None => {
                if k >= u.zones.len() {
                    return None;
                }
                u.zones.remove(k);
            }
        }
    }
    match doc.commit(Transaction::new(what).replace(before, after)) {
        Ok(()) => Some(format!("{}. Ctrl+Z to undo.", crate::i18n::tr(what))),
        Err(e) => Some(e.to_string()),
    }
}

/// Delete pressed with a zone picked: it is taken away.
pub fn delete(doc: &mut Document, state: &mut PlanState) -> Option<String> {
    let (room, k) = state.ufh_zone_pick.room.take()?;
    state.ufh_zone_pick.drag = None;
    commit(doc, room, k, None, "Underfloor heating zone removed")
}

/// A click, a drag and the picked zone's outline in a plan of `level`
/// with the Select tool. `Some` when it took the pointer.
#[allow(clippy::too_many_arguments)]
pub fn handle(
    doc: &mut Document,
    level: ElementId,
    xf: &Xf,
    response: &egui::Response,
    painter: &egui::Painter,
    state: &mut PlanState,
    selection: &mut crate::selection::Selection,
    shown: bool,
) -> Option<Option<String>> {
    if !shown {
        state.ufh_zone_pick = Picked::default();
        return None;
    }
    let px = 1.0 / xf.scale as f64;
    if let Some(r) = route(doc, level, xf, response, painter, state, selection, px) {
        return Some(r);
    }
    // the picked zone stays picked while its room is
    if let Some((room, _)) = state.ufh_zone_pick.room {
        if !selection.contains(room) {
            state.ufh_zone_pick = Picked::default();
        }
    }
    let picked = state
        .ufh_zone_pick
        .room
        .and_then(|(room, k)| Some((room, k, zone_of(doc, room, k)?)));
    // a drag of the picked zone
    if let Some((room, k, z)) = &picked {
        if response.drag_started_by(egui::PointerButton::Primary) {
            let p0 = response.interact_pointer_pos().map(|p| xf.to_world(p))?;
            let corner = z.outline.iter().position(|c| c.distance(p0) < 8.0 * px);
            let line = z.mark.filter(|m| seg_dist(p0, m[0], m[1]) < 6.0 * px);
            let g = match (corner, line) {
                (Some(i), _) => Some(Grab::Corner(i)),
                (None, Some(_)) => Some(Grab::Line),
                _ if creabim_core::polygon::contains(&z.outline, p0) => Some(Grab::Move),
                _ => None,
            };
            if let Some(g) = g {
                state.ufh_zone_pick.drag = Some((g, p0));
            }
        }
        if let Some((g, p0)) = state.ufh_zone_pick.drag {
            let now = response
                .interact_pointer_pos()
                .or(response.hover_pos())
                .map(|p| xf.to_world(p))
                .unwrap_or(p0);
            let shown = moved(z, g, now - p0);
            outline(painter, xf, &shown, true);
            if response.drag_stopped() || !response.dragged() {
                state.ufh_zone_pick.drag = None;
                if now.distance(p0) > 1e-4 {
                    let what = match g {
                        Grab::Move => "Underfloor heating zone moved",
                        Grab::Corner(_) => "Underfloor heating zone reshaped",
                        Grab::Line => "Edge zone width changed",
                    };
                    return Some(commit(doc, *room, *k, Some(shown), what));
                }
            }
            return Some(None);
        }
        outline(painter, xf, z, false);
    }
    // a click inside a zone picks it
    if response.clicked() {
        let p = response.interact_pointer_pos().map(|p| xf.to_world(p))?;
        let hit = zones_on(doc, level).into_iter().find(|(_, _, z)| {
            creabim_core::polygon::contains(&z.outline, p)
                || z.mark.is_some_and(|m| seg_dist(p, m[0], m[1]) < 6.0 * px)
        });
        match hit {
            Some((room, k, _)) => {
                state.ufh_zone_pick = Picked {
                    room: Some((room, k)),
                    ..Picked::default()
                };
                selection.only(room);
                return Some(Some(crate::i18n::tr("Zone picked: drag it to move it, a corner to reshape it, an edge zone's dashed line to change its width; Delete takes it away.").to_string()));
            }
            None => state.ufh_zone_pick = Picked::default(),
        }
    }
    None
}

/// The picked zone's outline and its corners' grips.
fn outline(painter: &egui::Painter, xf: &Xf, z: &Zone, dragging: bool) {
    let c = if dragging {
        Color32::from_rgb(230, 120, 0)
    } else {
        Color32::from_rgb(30, 120, 230)
    };
    let mut pts: Vec<egui::Pos2> = z.outline.iter().map(|p| xf.to_screen(*p)).collect();
    if let Some(&f) = pts.first() {
        pts.push(f);
    }
    painter.add(egui::Shape::line(pts.clone(), Stroke::new(2.0, c)));
    for p in &pts {
        painter.rect_filled(
            egui::Rect::from_center_size(*p, egui::vec2(7.0, 7.0)),
            1.0,
            c,
        );
    }
    if let Some(m) = z.mark {
        painter.line_segment(
            [xf.to_screen(m[0]), xf.to_screen(m[1])],
            Stroke::new(3.0, c),
        );
    }
}

/// The selected manifold's main route: not printed, shown only while the
/// manifold is selected, its points dragged and a midpoint dragged to put
/// in a new one (George 2026-11-28: "bude sa dat editovat ako ciara").
#[allow(clippy::too_many_arguments)]
fn route(
    doc: &mut Document,
    level: ElementId,
    xf: &Xf,
    response: &egui::Response,
    painter: &egui::Painter,
    state: &mut PlanState,
    selection: &crate::selection::Selection,
    px: f64,
) -> Option<Option<String>> {
    let (id, pts) = doc.model().iter().find_map(|e| match &e.kind {
        ElementKind::Object(ob)
            if ob.level == level && ob.route.len() >= 2 && selection.contains(e.id) =>
        {
            Some((e.id, ob.route.clone()))
        }
        _ => None,
    })?;
    let mids: Vec<DVec2> = pts.windows(2).map(|w| (w[0] + w[1]) / 2.0).collect();
    if response.drag_started_by(egui::PointerButton::Primary) {
        if let Some(p0) = response.interact_pointer_pos().map(|p| xf.to_world(p)) {
            if let Some(i) = pts.iter().position(|c| c.distance(p0) < 8.0 * px) {
                state.ufh_zone_pick.route = Some((id, i, false, p0));
            } else if let Some(i) = mids.iter().position(|c| c.distance(p0) < 8.0 * px) {
                state.ufh_zone_pick.route = Some((id, i, true, p0));
            }
        }
    }
    let mut shown = pts.clone();
    if let Some((rid, i, insert, p0)) = state.ufh_zone_pick.route.filter(|r| r.0 == id) {
        let now = response
            .interact_pointer_pos()
            .or(response.hover_pos())
            .map(|p| xf.to_world(p))
            .unwrap_or(p0);
        if insert {
            shown.insert(i + 1, mids[i] + (now - p0));
        } else if let Some(p) = shown.get_mut(i) {
            *p += now - p0;
        }
        draw_route(painter, xf, &shown, true);
        if response.drag_stopped() || !response.dragged() {
            state.ufh_zone_pick.route = None;
            if now.distance(p0) > 1e-4 {
                let before = doc.model().get(rid)?.clone();
                let mut after = before.clone();
                if let ElementKind::Object(ob) = &mut after.kind {
                    ob.route = shown;
                }
                let what = "Main route changed";
                return Some(Some(
                    match doc.commit(Transaction::new(what).replace(before, after)) {
                        Ok(()) => format!("{}. Ctrl+Z to undo.", crate::i18n::tr(what)),
                        Err(e) => e.to_string(),
                    },
                ));
            }
        }
        return Some(None);
    }
    draw_route(painter, xf, &shown, false);
    None
}

/// A main route drawn on screen only: a thin line, its points' grips and
/// small grips at its segments' midpoints.
fn draw_route(painter: &egui::Painter, xf: &Xf, pts: &[DVec2], dragging: bool) {
    let c = if dragging {
        Color32::from_rgb(230, 120, 0)
    } else {
        Color32::from_rgb(200, 40, 160)
    };
    let s: Vec<egui::Pos2> = pts.iter().map(|p| xf.to_screen(*p)).collect();
    painter.add(egui::Shape::dashed_line(&s, Stroke::new(2.0, c), 10.0, 5.0));
    for p in &s {
        painter.rect_filled(
            egui::Rect::from_center_size(*p, egui::vec2(7.0, 7.0)),
            1.0,
            c,
        );
    }
    for w in s.windows(2) {
        let m = w[0] + (w[1] - w[0]) / 2.0;
        painter.circle_stroke(m, 4.0, Stroke::new(1.5, c));
    }
}
