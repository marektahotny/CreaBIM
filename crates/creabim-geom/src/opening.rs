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
    // jambs and head, round every opening
    // a door as joinery: its frame is what its leaf leaves of the opening
    let (jw, hh) = match (&o.door, o.kind, &o.division) {
        (Some(d), OpeningKind::Door, None) => d.members(w, h),
        _ => (fw, fw),
    };
    let fw = if o.division.is_some() { fw } else { jw };
    add(Part::Frame, b(0.0, -hd, 0.0), b(fw, hd, h));
    add(Part::Frame, b(w - fw, -hd, 0.0), b(w, hd, h));
    add(Part::Frame, b(fw, -hd, h - hh), b(w - fw, hd, h));
    let divided = o.division.as_ref().filter(|_| o.kind != OpeningKind::Hole);
    if let Some(node) = divided {
        let door = o.kind == OpeningKind::Door;
        if !door {
            add(Part::Frame, b(fw, -hd, 0.0), b(w - fw, hd, fw));
            if face > hd + 0.005 {
                add(Part::Frame, b(0.0, hd, 0.0), b(w, face + SILL_NOSE, 0.03));
            }
        }
        // the division is seen from outside (+y here, −y once turned
        // across): its left is at the far x until then
        let x_of = |x: f64| if across { x } else { w - x };
        let (fields, bars) = node.layout(w, h, fw, fw, !door);
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
            field_boxes(&mut add, fl, &x_of, across, hd, fw, lt);
        }
    }
    match o.kind {
        _ if divided.is_some() => {}
        OpeningKind::Window => {
            // the sill member, then glazing bars splitting the glass
            add(Part::Frame, b(fw, -hd, 0.0), b(w - fw, hd, fw));
            // the outside sill, from the frame out past the wall's face,
            // on the side the window faces out of (built facing +y)
            if face > hd + 0.005 {
                add(Part::Frame, b(0.0, hd, 0.0), b(w, face + SILL_NOSE, 0.03));
            }
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
            add(Part::Leaf, b(l0, hd - lt, 0.01), b(l1, hd, h - hh - gap));
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

/// How far an opening leaf's sash stands proud of the frame inside.
const SASH_PROUD: f64 = 0.012;

/// One field of a division, in the opening's boxes: fixed glazing or a
/// panel in the frame; an opening leaf as a sash proud of the frame
/// inside, its glass in the sash and a handle inside; a door's leaf with
/// a lever on both faces. Built facing +y: inside is −y.
fn field_boxes(
    add: &mut impl FnMut(Part, DVec3, DVec3),
    fl: &Field,
    x_of: &dyn Fn(f64) -> f64,
    across: bool,
    hd: f64,
    fw: f64,
    leaf_t: f64,
) {
    let b = DVec3::new;
    let (xa, xb) = (x_of(fl.x0).min(x_of(fl.x1)), x_of(fl.x0).max(x_of(fl.x1)));
    let (z0, z1) = (fl.y0, fl.y1);
    let op = fl.sash.op;
    let fill = if fl.sash.panel {
        Part::Panel
    } else {
        Part::Glass
    };
    // seen from outside its left is at the far x, unless turned across
    let low_x = |left: bool| left == across;
    if op.is_door() {
        let gap = 0.004;
        add(
            Part::Leaf,
            b(xa + gap, hd - leaf_t, z0 + 0.01),
            b(xb - gap, hd, z1 - gap),
        );
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
        add(fill, b(xa, -GLASS / 2.0, z0), b(xb, GLASS / 2.0, z1));
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
            setback: None,
            reveal: None,
            division: None,
            door: None,
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
        // four frame members and the outside sill
        assert_eq!(count(&boxes, Part::Frame), 5);
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
        // the frame and sill, two upright bars, a cross bar in each column
        assert_eq!(count(&boxes, Part::Frame), 5 + 2 + 3);
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
        // frame of four, the outside sill, a mullion and a sash of four
        assert_eq!(count(&boxes, Part::Frame), 5 + 1 + 4);
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
