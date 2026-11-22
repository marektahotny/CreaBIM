//! Doors and windows built inside their wall openings: a frame round the
//! edge, glass set back in it (split into panes by glazing bars), and for a
//! door a leaf with a handle. Every part is a box in the opening's own
//! frame, so the 3D view, drawings and IFC export share one description.

use crate::wall::Cut;
use crate::TriMesh;
use creabim_core::assembly::Assembly;
use creabim_core::joinery::{Field, FrameNode};
use creabim_core::{LeafOp, Opening, OpeningFrame, OpeningKind, Wall};
use glam::{DVec2, DVec3};

/// What a part of a door or window is, which decides how it is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    /// Frame members and glazing bars, in the opening's material.
    Frame,
    /// A door's leaf, in the opening's material.
    Leaf,
    Glass,
    Handle,
    /// A solid panel filling a field.
    Panel,
    /// The stiles and rails of a glazed door leaf round its pane, in
    /// the leaf's material.
    LeafRail,
    /// The pane in a door leaf: glass, but never drawn across the
    /// opening in a plan (the leaf there is its swing).
    LeafGlass,
    /// The whole of a glazed leaf, for its swing and reach: only from
    /// [`leaf_boxes`], never built.
    LeafOutline,
}

/// One part as a box in opening coordinates: `x` along the wall from the
/// opening's left edge, `y` across the wall from its centre line (towards
/// the wall's left, where a door swings), `z` up from the opening's bottom.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartBox {
    pub part: Part,
    pub lo: DVec3,
    pub hi: DVec3,
}

/// Glass thickness, kept thin but solid so drawings show it.
const GLASS: f64 = 0.02;

/// The opening as built: the rectangle it fills in the wall (clipped to
/// the wall) and its frame fitted to that size.
#[derive(Clone, Copy, Debug)]
pub struct Fitted {
    pub cut: Cut,
    pub frame: OpeningFrame,
}

impl Fitted {
    pub fn of(o: &Opening, wall: &Wall) -> Option<Fitted> {
        let cut = Cut::of(o, wall)?;
        let frame = o
            .frame
            .fitted(cut.u1 - cut.u0, cut.v1 - cut.v0, wall.thickness);
        Some(Fitted { cut, frame })
    }

    pub fn width(&self) -> f64 {
        self.cut.u1 - self.cut.u0
    }

    pub fn height(&self) -> f64 {
        self.cut.v1 - self.cut.v0
    }

    /// A door leaf's thickness.
    pub fn leaf(&self) -> f64 {
        (self.frame.depth * 0.6).min(0.045)
    }
}

/// Every part of a door or window, in opening coordinates, in a wall
/// built up as `asm`: its frame stands where [`Opening::frame_span`] says.
pub fn opening_boxes(o: &Opening, wall: &Wall, asm: Option<&Assembly>) -> Vec<PartBox> {
    let mut out = all_boxes(o, wall, asm);
    out.retain(|b| b.part != Part::LeafOutline);
    out
}

/// Each door leaf whole, solid or glazed: what swings in a plan.
pub fn leaf_boxes(o: &Opening, wall: &Wall, asm: Option<&Assembly>) -> Vec<PartBox> {
    let mut out = all_boxes(o, wall, asm);
    out.retain(|b| matches!(b.part, Part::Leaf | Part::LeafOutline));
    out
}

fn all_boxes(o: &Opening, wall: &Wall, asm: Option<&Assembly>) -> Vec<PartBox> {
    let Some(f) = Fitted::of(o, wall) else {
        return vec![];
    };
    // a bare hole has nothing in it
    if o.kind == OpeningKind::Hole {
        return vec![];
    }
    let (w, h) = (f.width(), f.height());
    let fw = f.frame.width;
    let hd = (if o.is_lining() {
        wall.thickness
    } else {
        f.frame.depth
    }) / 2.0;
    // built about the frame's middle, facing +y, and moved to its place
    // across the wall at the end
    let across = !o.faces_left(wall);
    let span = o.frame_span(wall, asm, f.frame.depth);
    let yc = (span.0 + span.1) / 2.0;
    // from the frame's middle to the wall's face it faces out of
    let face = wall.thickness / 2.0 - if across { -yc } else { yc };
    let mut out = Vec::new();
    let mut add = |part, lo: DVec3, hi: DVec3| {
        if (hi - lo).min_element() > 1e-6 {
            out.push(PartBox { part, lo, hi });
        }
    };
    let b = DVec3::new;
    // a window's sills: the outer one of sheet metal from the frame out
    // over the wall's face, the inner board from the frame in over the
    // inner face; a window down to the floor has no board
    let inner = wall.thickness - face;
    let sills = |add: &mut dyn FnMut(Part, DVec3, DVec3)| {
        if !o.no_outer_sill && face > hd + 0.005 {
            add(Part::Frame, b(0.0, hd, 0.0), b(w, face + SILL_NOSE, 0.03));
        }
        if !o.no_inner_sill && o.sill >= 0.1 && inner > hd + 0.005 {
            add(
                Part::Frame,
                b(0.0, -inner - INNER_SILL_NOSE, 0.0),
                b(w, -hd, 0.025),
            );
        }
    };
    // jambs and head, round every opening
    // a door as joinery: its frame is what its leaf leaves of the opening
    let (jw, hh) = match (&o.door, o.kind, &o.division) {
        (Some(d), OpeningKind::Door, None) => d.members(w, h),
        // a divided door in its own frame, as the editor and the list
        // draw it
        // one leaf (glazed, or hung the other way) is still the door
        // its spec makes
        (Some(d), OpeningKind::Door, Some(FrameNode::Leaf(_))) => d.members(w, h),
        (Some(d), OpeningKind::Door, Some(_)) => {
            let j = d.members(w, h).0;
            (j, j)
        }
        _ => (fw, fw),
    };
    let fw = jw;
    add(Part::Frame, b(0.0, -hd, 0.0), b(fw, hd, h));
    add(Part::Frame, b(w - fw, -hd, 0.0), b(w, hd, h));
    add(Part::Frame, b(fw, -hd, h - hh), b(w - fw, hd, h));
    let divided = o.division.as_ref().filter(|_| o.kind != OpeningKind::Hole);
    if let Some(node) = divided {
        let door = o.kind == OpeningKind::Door;
        if !door {
            add(Part::Frame, b(fw, -hd, 0.0), b(w - fw, hd, fw));
            sills(&mut add);
        }
        // a divided door's threshold between its jambs, its leaves and
        // fixed fields standing on it
        let foot = if door && o.threshold {
            add(Part::Frame, b(fw, -hd, 0.0), b(w - fw, hd, THRESHOLD));
            THRESHOLD
        } else {
            0.0
        };
        // the division is seen from outside (+y here, −y once turned
        // across): its left is at the far x until then
        let x_of = |x: f64| if across { x } else { w - x };
        let (mut fields, bars) = node.layout_of(door, w, h, fw);
        // a leaf under a door's head that is deeper than its jambs
        if door {
            for f in &mut fields {
                f.y1 = f.y1.min(h - hh);
            }
        }
        for bar in &bars {
            let (xa, xb) = (
                x_of(bar.x0).min(x_of(bar.x1)),
                x_of(bar.x0).max(x_of(bar.x1)),
            );
            add(Part::Frame, b(xa, -hd, bar.y0), b(xb, hd, bar.y1));
        }
        let lt = o
            .door
            .as_ref()
            .map_or(f.leaf(), |d| d.leaf_thickness().min(f.frame.depth));
        for fl in &fields {
            field_boxes(&mut add, fl, &x_of, across, hd, fw, lt, foot);
        }
    }
    match o.kind {
        _ if divided.is_some() => {}
        OpeningKind::Window => {
            // the sill member, then glazing bars splitting the glass
            add(Part::Frame, b(fw, -hd, 0.0), b(w - fw, hd, fw));
            // the outside sill, from the frame out past the wall's face,
            // on the side the window faces out of (built facing +y)
            sills(&mut add);
            let (x0, x1, z0, z1) = (fw, w - fw, fw, h - fw);
            let (na, nu) = (f.frame.panes_across, f.frame.panes_up);
            let bar = fw * 0.6;
            let bd = hd * 0.7;
            // pane edges along x and z: between bars, bars centred on the splits
            let edges = |a: f64, z: f64, n: u32| -> Vec<(f64, f64)> {
                let step = (z - a) / n as f64;
                (0..n)
                    .map(|i| {
                        let lo = a + step * i as f64 + if i > 0 { bar / 2.0 } else { 0.0 };
                        let hi =
                            a + step * (i + 1) as f64 - if i + 1 < n { bar / 2.0 } else { 0.0 };
                        (lo, hi)
                    })
                    .collect()
            };
            let xs = edges(x0, x1, na);
            let zs = edges(z0, z1, nu);
            for pair in xs.windows(2) {
                add(Part::Frame, b(pair[0].1, -bd, z0), b(pair[1].0, bd, z1));
            }
            for &(xa, xb) in &xs {
                for pair in zs.windows(2) {
                    add(Part::Frame, b(xa, -bd, pair[0].1), b(xb, bd, pair[1].0));
                }
                for &(za, zb) in &zs {
                    add(Part::Glass, b(xa, -GLASS / 2.0, za), b(xb, GLASS / 2.0, zb));
                }
            }
        }
        OpeningKind::Door => {
            // the leaf, flush with the frame's face on the side it opens to
            let gap = 0.004;
            let lt = o
                .door
                .as_ref()
                .map_or(f.leaf(), |d| d.leaf_thickness().min(f.frame.depth));
            let (l0, l1) = (jw + gap, w - jw - gap);
            // over its threshold where it has one
            let foot = if o.threshold { THRESHOLD + gap } else { 0.01 };
            add(Part::Leaf, b(l0, hd - lt, foot), b(l1, hd, h - hh - gap));
            if o.threshold {
                add(Part::Frame, b(jw, -hd, 0.0), b(w - jw, hd, THRESHOLD));
            }
            // a lining's architraves, on both faces of the wall round
            // the opening
            if o.is_lining() {
                let (aw, at) = (0.06, 0.012);
                for (y0, y1) in [(hd, hd + at), (-hd - at, -hd)] {
                    add(Part::Frame, b(-aw + jw, y0, 0.0), b(jw, y1, h - hh + aw));
                    add(
                        Part::Frame,
                        b(w - jw, y0, 0.0),
                        b(w - jw + aw, y1, h - hh + aw),
                    );
                    add(Part::Frame, b(jw, y0, h - hh), b(w - jw, y1, h - hh + aw));
                }
            }
            // a lever handle on each face, on the side away from the hinge
            if l1 - l0 > 0.3 && h > 1.2 {
                let x = l1 - 0.07;
                let z = 1.0f64.min(h * 0.45);
                let (f0, f1) = (hd - lt, hd);
                for (r0, r1, a0, a1) in [
                    (f1, f1 + 0.012, f1 + 0.045, f1 + 0.062),
                    (f0 - 0.012, f0, f0 - 0.062, f0 - 0.045),
                ] {
                    add(
                        Part::Handle,
                        b(x - 0.025, r0, z - 0.04),
                        b(x + 0.025, r1, z + 0.04),
                    );
                    add(
                        Part::Handle,
                        b(x - 0.008, r0.min(a0), z - 0.008),
                        b(x + 0.008, r1.max(a1), z + 0.008),
                    );
                    add(
                        Part::Handle,
                        b(x - 0.13, a0, z - 0.01),
                        b(x + 0.01, a1, z + 0.01),
                    );
                }
            }
        }
        OpeningKind::Hole => {}
    }
    // built opening to (facing out of) +y, a door hinged at x = 0: turn it
    // round as the opening is set
    let along = o.kind == OpeningKind::Door && o.flip_hand;
    if across || along {
        for pb in &mut out {
            let (lo, hi) = (pb.lo, pb.hi);
            if along {
                (pb.lo.x, pb.hi.x) = (w - hi.x, w - lo.x);
            }
            if across {
                (pb.lo.y, pb.hi.y) = (-hi.y, -lo.y);
            }
        }
    }
    for pb in &mut out {
        pb.lo.y += yc;
        pb.hi.y += yc;
    }
    out
}

/// A lift-and-slide leaf as a floor plan marks it: along the opening
/// from `x0` to `x1` (opening coordinates, as [`opening_boxes`]), which
/// way it slides (towards `x1` when `to_high`), and the frame's face
/// inside, `y_in`, with `inward` the sign of the way into the room.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiftSlide {
    pub x0: f64,
    pub x1: f64,
    pub to_high: bool,
    pub y_in: f64,
    pub inward: f64,
}

/// Every lift-and-slide leaf of a window or door, for its plan mark.
pub fn lift_slides(o: &Opening, wall: &Wall, asm: Option<&Assembly>) -> Vec<LiftSlide> {
    let Some(f) = Fitted::of(o, wall) else {
        return vec![];
    };
    let Some(node) = o.division.as_ref().filter(|_| o.kind != OpeningKind::Hole) else {
        return vec![];
    };
    let (w, h) = (f.width(), f.height());
    let door = o.kind == OpeningKind::Door;
    let fw = match (&o.door, door) {
        (Some(d), true) => d.members(w, h).0,
        _ => f.frame.width,
    };
    let hd = f.frame.depth / 2.0;
    let across = !o.faces_left(wall);
    let span = o.frame_span(wall, asm, f.frame.depth);
    let yc = (span.0 + span.1) / 2.0;
    // built facing +y, inside −y; turned across, inside is +y
    let inward = if across { 1.0 } else { -1.0 };
    let x_of = |x: f64| if across { x } else { w - x };
    let low_x = |left: bool| left == across;
    let (fields, _) = node.layout_of(door, w, h, fw);
    fields
        .iter()
        .filter(|fl| fl.sash.op.lifts())
        .filter_map(|fl| {
            let left = fl.sash.op.slides_left()?;
            let (xa, xb) = (x_of(fl.x0).min(x_of(fl.x1)), x_of(fl.x0).max(x_of(fl.x1)));
            Some(LiftSlide {
                x0: xa,
                x1: xb,
                // it closes against the stile its handle is on and
                // slides away from it
                to_high: !low_x(left),
                y_in: yc + inward * (hd + SASH_PROUD),
                inward,
            })
        })
        .collect()
}

/// How far an opening leaf's sash stands proud of the frame inside.
const SASH_PROUD: f64 = 0.012;

/// One field of a division, in the opening's boxes: fixed glazing or a
/// panel in the frame; an opening leaf as a sash proud of the frame
/// inside, its glass in the sash and a handle inside; a door's leaf with
/// a lever on both faces. Built facing +y: inside is −y.
#[allow(clippy::too_many_arguments)]
fn field_boxes(
    add: &mut impl FnMut(Part, DVec3, DVec3),
    fl: &Field,
    x_of: &dyn Fn(f64) -> f64,
    across: bool,
    hd: f64,
    fw: f64,
    leaf_t: f64,
    foot: f64,
) {
    let b = DVec3::new;
    let (xa, xb) = (x_of(fl.x0).min(x_of(fl.x1)), x_of(fl.x0).max(x_of(fl.x1)));
    // a field down at the floor stands on the door's threshold
    let on_sill = foot > 0.0 && fl.y0 < 1e-6;
    let (z0, z1) = (if on_sill { foot } else { fl.y0 }, fl.y1);
    let op = fl.sash.op;
    let fill = if fl.sash.panel {
        Part::Panel
    } else {
        Part::Glass
    };
    // seen from outside its left is at the far x, unless turned across
    let low_x = |left: bool| left == across;
    // a pane set in `lo`..`hi` (a leaf or a panel), from `rail` round
    // it: the pane where the field's glazing puts it, x as drawn
    let pane_in = |lo: DVec3, hi: DVec3| -> Option<(f64, f64, f64, f64)> {
        let (fa, fb) = (x_of(lo.x), x_of(hi.x));
        // the leaf's edges in the field's own (drawn) x, left to right
        let (d0, d1) = (fa.min(fb), fa.max(fb));
        let p = fl.sash.glazing?.pane(d0, d1, lo.z, hi.z)?;
        let (pa, pb) = (x_of(p[0]), x_of(p[1]));
        Some((pa.min(pb), pa.max(pb), p[2], p[3]))
    };
    let glazed = |add: &mut dyn FnMut(Part, DVec3, DVec3),
                  rail: Part,
                  glass: Part,
                  lo: DVec3,
                  hi: DVec3,
                  p: (f64, f64, f64, f64)| {
        let (px0, px1, pz0, pz1) = p;
        add(rail, lo, b(px0, hi.y, hi.z));
        add(rail, b(px1, lo.y, lo.z), hi);
        add(rail, b(px0, lo.y, lo.z), b(px1, hi.y, pz0));
        add(rail, b(px0, lo.y, pz1), b(px1, hi.y, hi.z));
        let ym = (lo.y + hi.y) / 2.0;
        add(
            glass,
            b(px0, ym - GLASS / 2.0, pz0),
            b(px1, ym + GLASS / 2.0, pz1),
        );
    };
    if op.is_door() {
        let gap = 0.004;
        let (lo, hi) = (
            b(xa + gap, hd - leaf_t, z0 + if on_sill { gap } else { 0.01 }),
            b(xb - gap, hd, z1 - gap),
        );
        match pane_in(lo, hi) {
            Some(p) => {
                add(Part::LeafOutline, lo, hi);
                glazed(add, Part::LeafRail, Part::LeafGlass, lo, hi, p);
            }
            None => add(Part::Leaf, lo, hi),
        }
        if xb - xa > 0.3 && z1 - z0 > 1.2 {
            let hinge_low = low_x(op.hinged_left().unwrap_or(true));
            let x = if hinge_low { xb - 0.07 } else { xa + 0.07 };
            let dir = if hinge_low { -1.0 } else { 1.0 };
            let z = z0 + 1.0f64.min((z1 - z0) * 0.45);
            let (f0, f1) = (hd - leaf_t, hd);
            for (r0, r1, a0, a1) in [
                (f1, f1 + 0.012, f1 + 0.045, f1 + 0.062),
                (f0 - 0.012, f0, f0 - 0.062, f0 - 0.045),
            ] {
                add(
                    Part::Handle,
                    b(x - 0.025, r0, z - 0.04),
                    b(x + 0.025, r1, z + 0.04),
                );
                add(
                    Part::Handle,
                    b(x - 0.008, r0.min(a0), z - 0.008),
                    b(x + 0.008, r1.max(a1), z + 0.008),
                );
                let (h0, h1) = (x - 0.01, x + dir * 0.13);
                add(
                    Part::Handle,
                    b(h0.min(h1), a0, z - 0.01),
                    b(h0.max(h1), a1, z + 0.01),
                );
            }
        }
        return;
    }
    if op == LeafOp::Fixed {
        let (lo, hi) = (b(xa, -GLASS / 2.0, z0), b(xb, GLASS / 2.0, z1));
        // a solid panel may hold a pane
        match pane_in(lo, hi).filter(|_| fill == Part::Panel) {
            Some(p) => glazed(add, Part::Panel, Part::Glass, lo, hi, p),
            None => add(fill, lo, hi),
        }
        return;
    }
    let sw = (fw * 0.75).min((xb - xa) / 4.0).min((z1 - z0) / 4.0);
    let slides = op.slides_left().is_some();
    // a sliding leaf runs behind the frame's middle
    let (s0, s1) = if slides {
        (-hd, -0.004)
    } else {
        (-hd - SASH_PROUD, hd * 0.6)
    };
    add(Part::Frame, b(xa, s0, z0), b(xa + sw, s1, z1));
    add(Part::Frame, b(xb - sw, s0, z0), b(xb, s1, z1));
    add(Part::Frame, b(xa + sw, s0, z0), b(xb - sw, s1, z0 + sw));
    add(Part::Frame, b(xa + sw, s0, z1 - sw), b(xb - sw, s1, z1));
    let gy = if slides { -hd / 2.0 } else { 0.0 };
    add(
        fill,
        b(xa + sw, gy - GLASS / 2.0, z0 + sw),
        b(xb - sw, gy + GLASS / 2.0, z1 - sw),
    );
    // the handle: a rose on the sash and a lever standing off it
    if xb - xa < 0.2 || z1 - z0 < 0.3 {
        return;
    }
    let y = s0;
    let (r, a0, a1) = (y - 0.01, y - 0.05, y - 0.032);
    let (xm, zm) = ((xa + xb) / 2.0, ((z0 + z1) / 2.0).min(z0 + 1.05));
    // on the rail away from the hinges, the lever along it
    let rail = |add: &mut dyn FnMut(Part, DVec3, DVec3), z: f64| {
        add(
            Part::Handle,
            b(xm - 0.035, r, z - 0.014),
            b(xm + 0.035, y, z + 0.014),
        );
        add(
            Part::Handle,
            b(xm - 0.008, a0, z - 0.008),
            b(xm + 0.008, y, z + 0.008),
        );
        add(
            Part::Handle,
            b(xm - 0.01, a0, z - 0.009),
            b(xm + 0.11, a1, z + 0.009),
        );
    };
    match op {
        LeafOp::Tilt => return rail(add, z1 - sw / 2.0),
        LeafOp::TopHung => return rail(add, z0 + sw / 2.0),
        _ => {}
    }
    // on the stile away from the hinges, or the one a sliding leaf
    // closes against (it slides away from it), the lever down
    let hinge_left = op.hinged_left().or(op.slides_left()).unwrap_or(true);
    let x = if low_x(hinge_left) {
        xb - sw / 2.0
    } else {
        xa + sw / 2.0
    };
    let long = if op.lifts() { 0.22 } else { 0.11 };
    add(
        Part::Handle,
        b(x - 0.014, r, zm - 0.035),
        b(x + 0.014, y, zm + 0.035),
    );
    add(
        Part::Handle,
        b(x - 0.008, a0, zm - 0.008),
        b(x + 0.008, y, zm + 0.008),
    );
    add(
        Part::Handle,
        b(x - 0.009, a0, zm - long),
        b(x + 0.009, a1, zm + 0.01),
    );
}

/// The division a window or door has, for drawing it: see
/// [`creabim_core::joinery::division_of`].
pub fn division(o: &Opening) -> FrameNode {
    creabim_core::joinery::division_of(o)
}

/// How far a window's outside sill runs past the wall's face.
pub const SILL_NOSE: f64 = 0.04;
/// How far an inner sill board stands over the wall's inner face, m.
pub const INNER_SILL_NOSE: f64 = 0.03;
/// How high a door's threshold stands, m.
pub const THRESHOLD: f64 = 0.02;

/// Which way a door is hung, in words: its hinge side seen from the
/// side it opens to, and which side that is.
pub fn door_hand(o: &Opening, wall: &Wall) -> &'static str {
    // looking at the side it opens to, the wall's start is on the right
    // when that is the wall's left
    let hinge_at_start = !o.flip_hand;
    match (hinge_at_start == o.faces_left(wall), o.flip_facing) {
        (true, false) => "Right, in",
        (false, false) => "Left, in",
        (true, true) => "Right, out",
        (false, true) => "Left, out",
    }
}

/// Opening coordinates → model coordinates, for an opening in `wall`
/// standing on a level at `level_z`.
pub fn to_model(o: &Opening, wall: &Wall, level_z: f64) -> Option<impl Fn(DVec3) -> DVec3> {
    let c = Cut::of(o, wall)?;
    // square to the wall at the opening's middle: on a curved wall the
    // door or window stands on the arc, along its tangent there
    let um = (c.u0 + c.u1) / 2.0;
    let d = wall.tangent_at(um);
    let n = DVec2::new(-d.y, d.x);
    let start = wall.place(um, 0.0) - d * um;
    let z0 = level_z + wall.base_offset + c.v0;
    Some(move |p: DVec3| (start + d * (c.u0 + p.x) + n * p.y).extend(z0 + p.z))
}

/// The opening's parts as meshes in model coordinates, one per kind of part.
pub fn opening_meshes(
    o: &Opening,
    wall: &Wall,
    asm: Option<&Assembly>,
    level_z: f64,
) -> Vec<(Part, TriMesh)> {
    let Some(at) = to_model(o, wall, level_z) else {
        return vec![];
    };
    let mut out: Vec<(Part, TriMesh)> = Vec::new();
    for pb in opening_boxes(o, wall, asm) {
        let i = match out.iter().position(|(p, _)| *p == pb.part) {
            Some(i) => i,
            None => {
                out.push((pb.part, TriMesh::default()));
                out.len() - 1
            }
        };
        add_box(&mut out[i].1, pb.lo, pb.hi, &at);
    }
    out
}

/// A closed box from `lo` to `hi`, its corners mapped through `at` (a
/// rigid map, so the faces stay square).
pub(crate) fn add_box(m: &mut TriMesh, lo: DVec3, hi: DVec3, at: &dyn Fn(DVec3) -> DVec3) {
    let c = |x: f64, y: f64, z: f64| at(DVec3::new(x, y, z));
    let o = at(DVec3::ZERO);
    let axis = |v: DVec3| at(v) - o;
    let (ex, ey, ez) = (axis(DVec3::X), axis(DVec3::Y), axis(DVec3::Z));
    let (a, b) = (lo, hi);
    m.add_quad(
        [
            c(a.x, a.y, a.z),
            c(a.x, b.y, a.z),
            c(a.x, b.y, b.z),
            c(a.x, a.y, b.z),
        ],
        -ex,
    );
    m.add_quad(
        [
            c(b.x, a.y, a.z),
            c(b.x, b.y, a.z),
            c(b.x, b.y, b.z),
            c(b.x, a.y, b.z),
        ],
        ex,
    );
    m.add_quad(
        [
            c(a.x, a.y, a.z),
            c(b.x, a.y, a.z),
            c(b.x, a.y, b.z),
            c(a.x, a.y, b.z),
        ],
        -ey,
    );
    m.add_quad(
        [
            c(a.x, b.y, a.z),
            c(b.x, b.y, a.z),
            c(b.x, b.y, b.z),
            c(a.x, b.y, b.z),
        ],
        ey,
    );
    m.add_quad(
        [
            c(a.x, a.y, a.z),
            c(b.x, a.y, a.z),
            c(b.x, b.y, a.z),
            c(a.x, b.y, a.z),
        ],
        -ez,
    );
    m.add_quad(
        [
            c(a.x, a.y, b.z),
            c(b.x, a.y, b.z),
            c(b.x, b.y, b.z),
            c(a.x, b.y, b.z),
        ],
        ez,
    );
}

/// The hole an opening cuts, through the whole wall, as a box: what a
/// bare hole shows (see-through) and is picked by, and what the placement
/// ghost shows around a door or window (A7-4, MU18).
pub fn cut_box_mesh(o: &Opening, wall: &Wall, level_z: f64) -> Option<crate::TriMesh> {
    let c = crate::wall::Cut::of(o, wall)?;
    let t = wall.thickness / 2.0;
    // round the arc on a curved wall
    let n = if wall.is_arc() { 8 } else { 1 };
    let us: Vec<f64> = (0..=n)
        .map(|i| c.u0 + (c.u1 - c.u0) * i as f64 / n as f64)
        .collect();
    let mut outline: Vec<glam::DVec2> = us.iter().map(|u| wall.place(*u, -t)).collect();
    outline.extend(us.iter().rev().map(|u| wall.place(*u, t)));
    let z0 = level_z + wall.base_offset;
    Some(crate::scene::slab_mesh(&outline, z0 + c.v1, c.v1 - c.v0))
}

/// Does a round penetration cut its wall as a cylinder (a straight wall);
/// in a curved one it stays a rectangular cut.
pub fn cuts_round(o: &Opening, wall: &Wall) -> bool {
    o.is_round() && !wall.is_arc()
}

/// How much wider than its circle a round penetration's square is cut
/// (its corners filled back), so the fill is never of no thickness.
pub const ROUND_CLEAR: f64 = 0.005;

/// A shape drawn in a wall's (along, up) plane pushed across it from `s0`
/// to `s1`, in model coordinates.
fn across_wall(
    wall: &Wall,
    level_z: f64,
    shape: &[glam::DVec2],
    s0: f64,
    s1: f64,
) -> crate::TriMesh {
    let mut m = crate::scene::slab_mesh(shape, s1, s1 - s0);
    let d = wall.direction();
    let n = glam::DVec2::new(-d.y, d.x);
    let z0 = level_z + wall.base_offset;
    let to = |p: [f32; 3], origin: bool| -> [f32; 3] {
        let (u, v, s) = (p[0] as f64, p[1] as f64, p[2] as f64);
        let base = if origin {
            wall.start
        } else {
            glam::DVec2::ZERO
        };
        let q = base + d * u + n * s;
        let z = if origin { z0 + v } else { v };
        [q.x as f32, q.y as f32, z as f32]
    };
    for p in m.positions.iter_mut() {
        *p = to(*p, true);
    }
    for p in m.normals.iter_mut() {
        *p = to(*p, false);
    }
    // (along, up, across) is left-handed in the wall: turn the triangles
    for t in m.indices.chunks_mut(3) {
        t.swap(1, 2);
    }
    m
}

/// A round penetration is cut as its square, and the four corners
/// between the square and the circle are filled back: solids across the
/// wall from `s0` to `s1` (one layer, or the whole wall), so the wall and
/// its sections show the hole round (PilonDesigner cuts a round prestup
/// round). Empty for any other opening.
pub fn round_fillers(o: &Opening, wall: &Wall, level_z: f64, s0: f64, s1: f64) -> crate::TriMesh {
    let mut out = crate::TriMesh::default();
    if !cuts_round(o, wall) {
        return out;
    }
    let Some(c) = crate::wall::Cut::of(o, wall) else {
        return out;
    };
    let r = (c.u1 - c.u0) / 2.0;
    let (uc, vc) = ((c.u0 + c.u1) / 2.0, c.v0 + r);
    let centre = glam::DVec2::new(uc, vc);
    const STEPS: usize = 8;
    for q in 0..4 {
        // each quarter about the square's sides, split on its diagonals
        // (a section through the middle never meets a seam): two of the
        // square's corners and the arc between them
        let a0 = std::f64::consts::FRAC_PI_4 + q as f64 * std::f64::consts::FRAC_PI_2;
        let a1 = a0 + std::f64::consts::FRAC_PI_2;
        let h = r + ROUND_CLEAR;
        let corner = |a: f64| centre + glam::DVec2::new(a.cos().signum(), a.sin().signum()) * h;
        let mut shape = vec![corner(a0), corner(a1)];
        for i in 0..=STEPS {
            let a = a1 - std::f64::consts::FRAC_PI_2 * i as f64 / STEPS as f64;
            shape.push(centre + glam::DVec2::new(a.cos(), a.sin()) * r);
        }
        // counter-clockwise
        if creabim_core::polygon::signed_area2(&shape) < 0.0 {
            shape.reverse();
        }
        out.append(&across_wall(wall, level_z, &shape, s0, s1));
    }
    out
}

/// The cylinder of a round penetration, a little past both faces of its
/// straight wall: how it shows (see-through) and is picked in 3D.
pub fn round_cutter(o: &Opening, wall: &Wall, level_z: f64) -> Option<crate::TriMesh> {
    if !cuts_round(o, wall) {
        return None;
    }
    let c = crate::wall::Cut::of(o, wall)?;
    let r = (c.u1 - c.u0) / 2.0;
    let (uc, vc) = ((c.u0 + c.u1) / 2.0, c.v0 + r);
    let circle: Vec<glam::DVec2> = (0..32)
        .map(|i| {
            let a = i as f64 / 32.0 * std::f64::consts::TAU;
            glam::DVec2::new(uc + r * a.cos(), vc + r * a.sin())
        })
        .collect();
    let t = wall.thickness / 2.0;
    Some(across_wall(wall, level_z, &circle, -t, t))
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::ElementId;

    fn wall() -> Wall {
        Wall {
            level: ElementId::new(),
            start: DVec2::new(0.0, 0.0),
            end: DVec2::new(5.0, 0.0),
            thickness: 0.3,
            height: 3.0,
            base_offset: 0.0,
            material: None,
            assembly: None,
            flip: false,
            location: Default::default(),
            top: Default::default(),
            disallow_join: [false; 2],
            attach_top: vec![],
            curtain: None,
            layer_ends: vec![],
            bulge: 0.0,
            bearing: None,
            fire_partial: false,
            no_fire_barriers: false,
        }
    }

    fn opening(kind: OpeningKind, frame: OpeningFrame) -> Opening {
        Opening {
            host: ElementId::new(),
            kind,
            offset: 1.0,
            sill: if kind == OpeningKind::Door { 0.0 } else { 0.9 },
            width: 1.2,
            height: if kind == OpeningKind::Door { 2.1 } else { 1.3 },
            material: None,
            frame,
            flip_hand: false,
            flip_facing: false,
            no_outer_sill: false,
            no_inner_sill: false,
            threshold: false,
            threshold_material: None,
            penetration: None,
            setback: None,
            reveal: None,
            division: None,
            door: None,
            texts: None,
            performance: None,
        }
    }

    fn count(boxes: &[PartBox], part: Part) -> usize {
        boxes.iter().filter(|b| b.part == part).count()
    }

    #[test]
    fn a_window_in_a_curved_wall_stands_on_its_arc() {
        // a half circle of radius 2.5 from (0,0) to (5,0)
        let w = Wall {
            bulge: 1.0,
            ..wall()
        };
        let o = opening(OpeningKind::Window, OpeningFrame::default());
        let at = to_model(&o, &w, 0.0).unwrap();
        // the window's middle, on the wall's centreline
        let mid = at(DVec3::new(o.width / 2.0, 0.0, 0.0)).truncate();
        let c = DVec2::new(2.5, 0.0);
        assert!((mid.distance(c) - 2.5).abs() < 0.01, "{mid}");
        assert!(w.along(mid).1 < 0.01);
        // square to the wall there: across it points at the centre
        let across = at(DVec3::new(o.width / 2.0, 1.0, 0.0)).truncate() - mid;
        assert!(across.normalize().perp_dot((c - mid).normalize()).abs() < 1e-6);
    }

    #[test]
    fn a_window_has_a_frame_round_set_back_glass() {
        let o = opening(OpeningKind::Window, OpeningFrame::default());
        let boxes = opening_boxes(&o, &wall(), None);
        // four frame members, the outside sill and the inner board
        assert_eq!(count(&boxes, Part::Frame), 6);
        // without its sills: the frame alone
        let mut bare = o.clone();
        (bare.no_outer_sill, bare.no_inner_sill) = (true, true);
        assert_eq!(count(&opening_boxes(&bare, &wall(), None), Part::Frame), 4);
        assert_eq!(count(&boxes, Part::Glass), 1);
        let glass = boxes.iter().find(|b| b.part == Part::Glass).unwrap();
        // the glass fills the frame's inside, behind its faces
        assert!((glass.lo.x - 0.06).abs() < 1e-9 && (glass.hi.x - 1.14).abs() < 1e-9);
        assert!(glass.hi.y < 0.05 && glass.lo.y > -0.05);
        // everything stays inside the opening
        for b in &boxes {
            assert!(b.lo.x >= -1e-9 && b.hi.x <= 1.2 + 1e-9);
            assert!(b.lo.z >= -1e-9 && b.hi.z <= 1.3 + 1e-9);
        }
    }

    #[test]
    fn panes_split_the_glass_with_bars() {
        let frame = OpeningFrame {
            panes_across: 3,
            panes_up: 2,
            ..OpeningFrame::default()
        };
        let boxes = opening_boxes(&opening(OpeningKind::Window, frame), &wall(), None);
        assert_eq!(count(&boxes, Part::Glass), 6);
        // the frame and sills, two upright bars, a cross bar in each column
        assert_eq!(count(&boxes, Part::Frame), 6 + 2 + 3);
        let glass: f64 = boxes
            .iter()
            .filter(|b| b.part == Part::Glass)
            .map(|b| (b.hi.x - b.lo.x) * (b.hi.z - b.lo.z))
            .sum();
        let bar = 0.06 * 0.6;
        let inside = (1.2 - 0.12 - 2.0 * bar) * (1.3 - 0.12 - bar);
        assert!((glass - inside).abs() < 1e-9);
    }

    #[test]
    fn a_door_has_a_leaf_and_handles_but_no_sill() {
        let o = opening(OpeningKind::Door, OpeningFrame::default());
        let boxes = opening_boxes(&o, &wall(), None);
        assert_eq!(count(&boxes, Part::Frame), 3);
        assert_eq!(count(&boxes, Part::Leaf), 1);
        assert_eq!(count(&boxes, Part::Glass), 0);
        assert!(count(&boxes, Part::Handle) >= 2);
        let meshes = opening_meshes(&o, &wall(), None, 0.0);
        // every part is a closed box
        for (_, m) in &meshes {
            assert!(m.volume() > 0.0);
        }
    }

    #[test]
    fn every_door_with_a_threshold_has_it_between_its_jambs() {
        use creabim_core::joinery::{DoorFrame, DoorMaterial, DoorSpec};
        let divisions = [
            None,
            Some("v(1:D-L,1:D-P)"),
            Some("v(0.6:D-L,0.4:FIX)"),
            Some("v(1:P-L,1:FIX)"),
            Some("v(1:ZP-P,1:FIX*)"),
            Some("h(0.8:v(1:D-L,1:D-P),0.2:FIX)"),
        ];
        for frame in DoorFrame::ALL {
            for leaf in DoorMaterial::ALL {
                for div in divisions {
                    for spec in [false, true] {
                        let mut o = opening(OpeningKind::Door, OpeningFrame::default());
                        o.threshold = true;
                        o.division = div.map(|d| FrameNode::parse(d).unwrap());
                        if spec {
                            let mut d = DoorSpec::fitting(frame, o.width, o.height);
                            d.leaf = leaf;
                            o.door = Some(d);
                        }
                        let boxes = opening_boxes(&o, &wall(), None);
                        let sill = boxes.iter().find(|b| {
                            b.part == Part::Frame
                                && b.lo.z <= 1e-9
                                && (b.hi.z - THRESHOLD).abs() < 1e-9
                                && b.hi.x - b.lo.x > 0.8
                        });
                        let what = format!("{frame:?} {leaf:?} {div:?} spec {spec}");
                        assert!(sill.is_some(), "no threshold: {what}");
                        // nothing else of the door reaches down into it
                        for b in &boxes {
                            if matches!(b.part, Part::Leaf | Part::Glass | Part::Panel) {
                                assert!(b.lo.z >= THRESHOLD - 1e-9, "{what}: {:?} sinks", b.part);
                            }
                        }
                        // and none without one
                        o.threshold = false;
                        let boxes = opening_boxes(&o, &wall(), None);
                        assert!(
                            !boxes.iter().any(|b| b.part == Part::Frame
                                && b.lo.z <= 1e-9
                                && (b.hi.z - THRESHOLD).abs() < 1e-9
                                && b.hi.x - b.lo.x > 0.8),
                            "{what}: a threshold not asked for"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_glazed_leaf_or_panel_holds_its_pane() {
        use creabim_core::joinery::{DoorFrame, DoorSpec};
        let mut o = opening(OpeningKind::Door, OpeningFrame::default());
        o.width = 1.6;
        o.door = Some(DoorSpec::fitting(DoorFrame::Aluminium, 1.6, 2.1));
        // a wide leaf with a pane 150 in, 400 up from its foot; the
        // narrow side leaf glazed whole; a panel beside with a pane
        o.division = Some(
            FrameNode::parse("v(0.5:D-L~150.150.150.400,0.3:D-P~0.0.0.0,0.2:FIX*~50.50.50.50)")
                .unwrap(),
        );
        let boxes = opening_boxes(&o, &wall(), None);
        assert_eq!(count(&boxes, Part::Leaf), 0);
        assert_eq!(count(&boxes, Part::LeafOutline), 0);
        assert_eq!(count(&boxes, Part::LeafGlass), 2);
        // rails round the first leaf's pane, none round a pane glazed whole
        assert_eq!(count(&boxes, Part::LeafRail), 4);
        assert_eq!(count(&boxes, Part::Glass), 1);
        assert_eq!(count(&boxes, Part::Panel), 4);
        // the swing still sees both leaves whole
        let leaves = leaf_boxes(&o, &wall(), None);
        assert_eq!(leaves.len(), 2);
        let wide = leaves
            .iter()
            .max_by(|a, b| (a.hi.x - a.lo.x).total_cmp(&(b.hi.x - b.lo.x)))
            .unwrap();
        let pane = boxes
            .iter()
            .filter(|b| b.part == Part::LeafGlass)
            .find(|g| g.lo.x > wide.lo.x - 1e-9 && g.hi.x < wide.hi.x + 1e-9)
            .unwrap();
        assert!(
            (pane.lo.x - wide.lo.x - 0.15).abs() < 1e-6,
            "{pane:?} {wide:?}"
        );
        assert!((wide.hi.x - pane.hi.x - 0.15).abs() < 1e-6);
        assert!((pane.lo.z - wide.lo.z - 0.4).abs() < 1e-6);
        assert!((wide.hi.z - pane.hi.z - 0.15).abs() < 1e-6);
        // a single leaf glazed keeps its door's head
        o.division = Some(FrameNode::parse("D-L~0.0.0.0").unwrap());
        o.width = 0.9;
        o.door = Some(DoorSpec::fitting(DoorFrame::Steel, 0.9, 2.1));
        let plain = {
            let mut p = o.clone();
            p.division = None;
            opening_boxes(&p, &wall(), None)
        };
        let solid = plain.iter().find(|b| b.part == Part::Leaf).unwrap();
        let glazed = leaf_boxes(&o, &wall(), None)[0];
        assert!((solid.hi.z - glazed.hi.z).abs() < 1e-9);
    }

    #[test]
    fn the_frame_fits_a_thin_wall() {
        let mut w = wall();
        w.thickness = 0.05;
        let o = opening(OpeningKind::Window, OpeningFrame::default());
        for b in opening_boxes(&o, &w, None) {
            assert!(b.lo.y >= -0.025 - 1e-9 && b.hi.y <= 0.025 + 1e-9);
        }
    }

    fn leaf(o: &Opening) -> PartBox {
        *opening_boxes(o, &wall(), None)
            .iter()
            .find(|b| b.part == Part::Leaf)
            .unwrap()
    }

    #[test]
    fn doors_flip_hand_and_facing() {
        let mut o = opening(OpeningKind::Door, OpeningFrame::default());
        // into the interior (the left of an unflipped wall), hinged at its start
        let l = leaf(&o);
        assert!(l.hi.y > 0.0 && l.lo.y > -0.01);
        assert_eq!(door_hand(&o, &wall()), "Right, in");
        let handle = |o: &Opening| {
            opening_boxes(o, &wall(), None)
                .iter()
                .find(|b| b.part == Part::Handle)
                .unwrap()
                .lo
                .x
        };
        assert!(handle(&o) > 0.6, "the handle is away from the hinge");
        o.flip_hand = true;
        assert!(handle(&o) < 0.6);
        assert!((leaf(&o).lo.x - (1.2 - l.hi.x)).abs() < 1e-9);
        assert_eq!(door_hand(&o, &wall()), "Left, in");
        o.flip_facing = true;
        let l = leaf(&o);
        assert!(l.lo.y < 0.0 && l.hi.y < 0.01, "now to the wall's right");
        assert_eq!(door_hand(&o, &wall()), "Right, out");
    }

    fn layered() -> Assembly {
        use creabim_core::assembly::{AssemblyUse, Layer, LayerFunction};
        let l = |function, thickness| Layer::new(function, None, thickness);
        // from outside: render, insulation, masonry, plaster = 0.3
        Assembly::new(
            AssemblyUse::Wall,
            vec![
                l(LayerFunction::Finish, 0.01),
                l(LayerFunction::Insulation, 0.09),
                l(LayerFunction::Structure, 0.19),
                l(LayerFunction::Finish, 0.01),
            ],
        )
    }

    fn frame_y(o: &Opening, w: &Wall, a: Option<&Assembly>) -> (f64, f64) {
        // a jamb: the frame's whole depth
        let b = opening_boxes(o, w, a)[0];
        (b.lo.y, b.hi.y)
    }

    #[test]
    fn the_frame_stands_on_the_cores_outer_edge() {
        let a = layered();
        let mut o = opening(OpeningKind::Window, OpeningFrame::default());
        let w = wall();
        // plain wall: in its middle
        let (lo, hi) = frame_y(&o, &w, None);
        assert!((lo + 0.05).abs() < 1e-9 && (hi - 0.05).abs() < 1e-9);
        // layered: the exterior is the wall's right (−0.15), the core
        // starts 0.1 in
        let (lo, hi) = frame_y(&o, &w, Some(&a));
        assert!(
            (lo + 0.05).abs() < 1e-9 && (hi - 0.05).abs() < 1e-9,
            "{lo} {hi}"
        );
        // out into the insulation by 40 mm
        o.setback = Some(-0.04);
        let (lo, hi) = frame_y(&o, &w, Some(&a));
        assert!(
            (lo + 0.09).abs() < 1e-9 && (hi - 0.01).abs() < 1e-9,
            "{lo} {hi}"
        );
        // a flipped wall has its exterior on the left
        let fw = Wall {
            flip: true,
            ..wall()
        };
        let (lo, hi) = frame_y(&o, &fw, Some(&a));
        assert!(
            (hi - 0.09).abs() < 1e-9 && (lo + 0.01).abs() < 1e-9,
            "{lo} {hi}"
        );
        // never out of the wall
        o.setback = Some(-1.0);
        let (lo, hi) = frame_y(&o, &w, Some(&a));
        assert!((lo + 0.15).abs() < 1e-9 && (hi + 0.05).abs() < 1e-9);
        // the outside sill still ends past the wall's outside face
        o.setback = None;
        let sill = opening_boxes(&o, &w, Some(&a))
            .into_iter()
            .map(|b| b.lo.y)
            .fold(f64::MAX, f64::min);
        assert!((sill + 0.15 + SILL_NOSE).abs() < 1e-9, "{sill}");
        // set in a plain wall, from its outside face
        o.setback = Some(0.02);
        let (lo, _) = frame_y(&o, &w, None);
        assert!((lo + 0.13).abs() < 1e-9);
    }

    #[test]
    fn each_field_opens_its_own_way_with_a_handle_inside() {
        let mut o = opening(OpeningKind::Window, OpeningFrame::default());
        // seen from outside: a tilt and turn leaf hinged left, then fixed
        o.division = Some(FrameNode::parse("v(1:OS-L,1:FIX)").unwrap());
        let boxes = opening_boxes(&o, &wall(), None);
        // frame of four, the two sills, a mullion and a sash of four
        assert_eq!(count(&boxes, Part::Frame), 6 + 1 + 4);
        assert_eq!(count(&boxes, Part::Glass), 2);
        assert_eq!(count(&boxes, Part::Handle), 3);
        // the window faces the wall's right (−y): inside is +y; seen from
        // outside (looking along +y) the left is at low x
        let h = boxes.iter().find(|b| b.part == Part::Handle).unwrap();
        assert!(h.lo.y > 0.05, "inside the frame's inner face: {}", h.lo.y);
        // hinged on the left: the handle by the mullion in the middle
        assert!(h.lo.x > 0.4 && h.hi.x < 0.6, "{} {}", h.lo.x, h.hi.x);
        // a tilting fanlight over a lift and slide leaf, a panel beside
        o.division = Some(FrameNode::parse("v(1:h(0.3:S,0.7:ZP-P),1:FIX*)").unwrap());
        let boxes = opening_boxes(&o, &wall(), None);
        assert_eq!(count(&boxes, Part::Handle), 6);
        assert_eq!(count(&boxes, Part::Panel), 1);
        assert!(boxes
            .iter()
            .any(|b| b.part == Part::Handle && b.lo.z > 1.3 - 0.12));
        for b in &boxes {
            assert!(b.lo.x >= -1e-9 && b.hi.x <= 1.2 + 1e-9);
        }
    }

    #[test]
    fn a_door_as_joinery_has_its_leafs_nominal_size() {
        use creabim_core::joinery::{apply_door, DoorFrame, DoorSpec};
        let mut o = opening(OpeningKind::Door, OpeningFrame::default());
        apply_door(&mut o, DoorSpec::fitting(DoorFrame::Lining, 0.9, 2.02));
        assert_eq!((o.width, o.height), (0.9, 2.02));
        let boxes = opening_boxes(&o, &wall(), None);
        let l = boxes.iter().find(|b| b.part == Part::Leaf).unwrap();
        // 800 less the gaps, under a 50 mm head
        assert!((l.hi.x - l.lo.x - 0.792).abs() < 1e-9);
        assert!((l.hi.z - (1.97 - 0.004)).abs() < 1e-9);
        // the lining runs through the whole wall, architraves on both faces
        let jamb = boxes[0];
        assert!((jamb.lo.y + 0.15).abs() < 1e-9 && (jamb.hi.y - 0.15).abs() < 1e-9);
        assert_eq!(count(&boxes, Part::Frame), 3 + 6);
        assert!(boxes.iter().any(|b| b.hi.y > 0.16) && boxes.iter().any(|b| b.lo.y < -0.16));
        // a steel frame stays its own depth
        apply_door(&mut o, DoorSpec::fitting(DoorFrame::Steel, 0.95, 2.05));
        let jamb = opening_boxes(&o, &wall(), None)[0];
        assert!((jamb.hi.y - jamb.lo.y - 0.1).abs() < 1e-9);
        assert!((jamb.hi.x - 0.075).abs() < 1e-9);
        // a door with a side light: the leaf in its field
        o.division = Some(FrameNode::parse("v(0.7:D-L,0.3:FIX)").unwrap());
        let boxes = opening_boxes(&o, &wall(), None);
        assert_eq!(count(&boxes, Part::Leaf), 1);
        assert_eq!(count(&boxes, Part::Glass), 1);
        // a turning leaf in a door is a door leaf (it swings in plan),
        // and the side light is no passage: the mark says the leaf only
        o.division = Some(FrameNode::parse("v(0.7:O-P,0.3:FIX)").unwrap());
        let boxes = opening_boxes(&o, &wall(), None);
        assert_eq!(count(&boxes, Part::Leaf), 1);
        let (pw, _) = creabim_core::joinery::clear_passage(&o).unwrap();
        let fw = creabim_core::joinery::frame_width(&o);
        assert!(
            (pw - ((o.width - 2.0 * fw) * 0.7 * 1000.0).round() / 1000.0).abs() < 1e-9,
            "{pw}"
        );
        // both opening: two leaves meeting with no mullion between, the
        // passage the whole width between the jambs
        o.division = Some(FrameNode::parse("v(0.7:D-L,0.3:O-P)").unwrap());
        let boxes = opening_boxes(&o, &wall(), None);
        assert_eq!(count(&boxes, Part::Leaf), 2);
        let (pw, _) = creabim_core::joinery::clear_passage(&o).unwrap();
        assert!(
            (pw - ((o.width - 2.0 * fw) * 1000.0).round() / 1000.0).abs() < 1e-9,
            "{pw}"
        );
        let (_, bars) = o
            .division
            .as_ref()
            .unwrap()
            .layout_of(true, o.width, o.height, fw);
        assert!(bars.is_empty());
    }

    #[test]
    fn a_window_faces_out_of_the_exterior_unless_flipped() {
        let mut o = opening(OpeningKind::Window, OpeningFrame::default());
        let sill = |o: &Opening| {
            opening_boxes(o, &wall(), None)
                .into_iter()
                .filter(|b| b.part == Part::Frame)
                .min_by(|a, b| a.lo.y.min(-a.hi.y).total_cmp(&b.lo.y.min(-b.hi.y)))
                .unwrap()
        };
        // the exterior is the wall's right: the sill runs out past it
        assert!((sill(&o).lo.y + 0.15 + SILL_NOSE).abs() < 1e-9);
        o.flip_facing = true;
        assert!((sill(&o).hi.y - 0.15 - SILL_NOSE).abs() < 1e-9);
    }
}
