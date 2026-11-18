//! The window list and the door list of Slovak practice (výpis okien,
//! výpis dverí): each kind of window or door once, with its mark, its
//! structural opening (and a door's leaf), a drawing of it at 1:100 with
//! its sizes and how it opens (STN 01 3420: a triangle's point is at the
//! hinges), how many there are on each level (doors left and right), how
//! it is made, its finish and a note.

use crate::annotation::TextLook;
use crate::paper::Anchor;
use crate::plan::{Pen, PlanItem, PlanView};
use creabim_core::joinery::{division_of, sash_symbol};
use creabim_core::words::tr;
use creabim_core::{ElementId, ElementKind, LeafOp, Model, Opening, OpeningKind};
use glam::DVec2;

/// One kind of window or door.
#[derive(Clone, Debug)]
pub struct Entry {
    /// The marks of those of the kind, as their tags read.
    pub mark: String,
    pub ids: Vec<ElementId>,
    /// One of them, standing for all.
    pub opening: Opening,
    /// On each level, lowest first: its name, and how many (doors:
    /// left, right; windows: all, none).
    pub counts: Vec<(String, u32, u32)>,
    pub execution: String,
    pub finish: String,
    pub note: String,
}

impl Entry {
    pub fn total(&self) -> (u32, u32) {
        self.counts
            .iter()
            .fold((0, 0), |a, c| (a.0 + c.1, a.1 + c.2))
    }
}

/// A leaf's way of opening without its side: "Tilt and turn".
fn op_word(op: LeafOp) -> &'static str {
    match op {
        LeafOp::Fixed => "Fixed glazing",
        LeafOp::TurnLeft | LeafOp::TurnRight => "Turning",
        LeafOp::Tilt => "Tilting",
        LeafOp::TiltTurnLeft | LeafOp::TiltTurnRight => "Tilt and turn",
        LeafOp::TopHung => "Top hung",
        LeafOp::SlideLeft | LeafOp::SlideRight => "Sliding",
        LeafOp::LiftSlideLeft | LeafOp::LiftSlideRight => "Lift and slide",
        LeafOp::DoorLeft | LeafOp::DoorRight => "Door leaf",
    }
}

/// What the list says of `o` where its own texts say nothing.
fn told(model: &Model, id: ElementId, o: &Opening) -> (String, String, String) {
    let own = o.texts.clone().unwrap_or_default();
    let leaves = division_of(o).leaves();
    let mut words: Vec<&str> = vec![];
    for l in &leaves {
        let w = op_word(l.op);
        if !words.contains(&w) {
            words.push(w);
        }
    }
    let ops = words
        .iter()
        .map(|w| tr(w).to_lowercase())
        .collect::<Vec<_>>()
        .join(", ");
    let moving = leaves.iter().filter(|l| l.op != LeafOp::Fixed).count();
    let execution = if !own.execution.is_empty() {
        own.execution
    } else if o.kind == OpeningKind::Door {
        let mut s = match &o.door {
            Some(d) => format!("{}: {}", tr("Door leaf"), tr(d.leaf.label()).to_lowercase()),
            None => tr("Door"),
        };
        if let Some(f) = o.door.as_ref().and_then(|d| d.fire.as_ref()) {
            s.push_str(&format!(", {} {}", tr("fire door"), f.label()));
        }
        if leaves.len() > 1 {
            s.push_str(&format!(", {ops}"));
        }
        s
    } else {
        format!(
            "{}, {}: {}, {ops}",
            tr("Window"),
            tr("opening leaves").to_lowercase(),
            moving
        )
    };
    let finish = if !own.finish.is_empty() {
        own.finish
    } else {
        model
            .get(id)
            .and_then(|e| creabim_core::material_of(model, e))
            .map(|(m, _)| m.name.clone())
            .unwrap_or_default()
    };
    let note = if !own.note.is_empty() {
        own.note
    } else {
        o.door
            .as_ref()
            .filter(|_| o.kind == OpeningKind::Door)
            .map(|d| tr(d.frame.label()))
            .unwrap_or_default()
    };
    (execution, finish, note)
}

/// The kinds of windows or doors of the model: those alike in size,
/// division, joinery, material and texts are one kind.
pub fn entries(model: &Model, kind: OpeningKind) -> Vec<Entry> {
    let marks = creabim_core::marks(model);
    let mut levels: Vec<(ElementId, f64, String)> = model
        .levels()
        .map(|(e, l)| (e.id, l.elevation, e.name.clone()))
        .collect();
    levels.sort_by(|a, b| a.1.total_cmp(&b.1));
    // the kind's key, its row, and where its pieces stand (level, left)
    type Kind = (String, Entry, Vec<(ElementId, bool)>);
    let mut out: Vec<Kind> = vec![];
    for e in model.iter() {
        let ElementKind::Opening(o) = &e.kind else {
            continue;
        };
        if o.kind != kind {
            continue;
        }
        let Some(ElementKind::Wall(w)) = model.get(o.host).map(|h| &h.kind) else {
            continue;
        };
        let key = creabim_core::joinery::kind_key(o);
        let mut plain = o.clone();
        plain.host = ElementId::NIL;
        plain.offset = 0.0;
        let left = crate::annotate::door_hand(model, e.id) != Some("P");
        let i = match out.iter().position(|x| x.0 == key) {
            Some(i) => i,
            None => {
                let (execution, finish, note) = told(model, e.id, o);
                out.push((
                    key,
                    Entry {
                        mark: String::new(),
                        ids: vec![],
                        opening: plain,
                        counts: vec![],
                        execution,
                        finish,
                        note,
                    },
                    vec![],
                ));
                out.len() - 1
            }
        };
        out[i].1.ids.push(e.id);
        out[i].2.push((w.level, left));
    }
    let mut entries: Vec<Entry> = out
        .into_iter()
        .map(|(_, mut en, at)| {
            let mut ms: Vec<String> = en
                .ids
                .iter()
                .filter_map(|id| marks.get(id).cloned())
                .collect();
            ms.sort();
            ms.dedup();
            en.mark = ms.join(", ");
            for (lid, _, name) in &levels {
                let l = at.iter().filter(|x| x.0 == *lid && x.1).count() as u32;
                let r = at.iter().filter(|x| x.0 == *lid && !x.1).count() as u32;
                if l + r > 0 {
                    en.counts.push(if kind == OpeningKind::Door {
                        (name.clone(), l, r)
                    } else {
                        (name.clone(), l + r, 0)
                    });
                }
            }
            en
        })
        .collect();
    entries.sort_by(|a, b| a.mark.cmp(&b.mark));
    entries
}

/// Column widths, paper mm: mark, sizes, drawing, level, left, right,
/// make, finish, note.
const COLS: [f64; 9] = [13.0, 25.0, 50.0, 17.0, 8.0, 8.0, 30.0, 26.0, 24.0];
const HEAD: f64 = 14.0;
const TITLE: f64 = 10.0;
const TEXT: f64 = 2.2;
const LINE: f64 = 3.3;
/// Paper mm to a metre of the drawings: 1:100.
const K: f64 = 10.0;
const MIN_ROW: f64 = 30.0;

fn num(model: &Model, v: f64) -> String {
    creabim_core::write_number(model, creabim_core::NumberKind::Dimensions, None, v)
}

/// `text` broken into lines no wider than `width`, at its spaces.
pub fn wrap(text: &str, width: f64, size: f64) -> Vec<String> {
    let mut lines = vec![];
    for para in text.split('\n') {
        let mut line = String::new();
        for word in para.split_whitespace() {
            let trial = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && crate::paper::text_width(&trial, size) > width {
                lines.push(std::mem::take(&mut line));
                line = word.to_string();
            } else {
                line = trial;
            }
        }
        if !line.is_empty() {
            lines.push(line);
        }
    }
    lines
}

struct Pencil<'a> {
    items: &'a mut Vec<PlanItem>,
    id: ElementId,
}

impl Pencil<'_> {
    fn line(&mut self, a: DVec2, b: DVec2, pen: Pen) {
        self.items.push(PlanItem::Line {
            id: self.id,
            pen,
            a,
            b,
        });
    }

    fn rect(&mut self, lo: DVec2, hi: DVec2, pen: Pen) {
        let c = [lo, DVec2::new(hi.x, lo.y), hi, DVec2::new(lo.x, hi.y)];
        for i in 0..4 {
            self.line(c[i], c[(i + 1) % 4], pen);
        }
    }

    fn text(&mut self, at: DVec2, text: &str, size: f64, align: Anchor, bold: bool) {
        self.items.push(PlanItem::Text {
            id: self.id,
            at,
            text: text.to_string(),
            size,
            align,
            mm: 1.0,
            rgb: None,
            bold,
        });
    }

    /// Lines of text centred in a cell from `x` `w` wide, from `top`.
    fn block(&mut self, x: f64, w: f64, top: f64, text: &str) {
        for (i, l) in wrap(text, w - 3.0, TEXT).iter().enumerate() {
            self.text(
                DVec2::new(x + w / 2.0, top - 3.0 - i as f64 * LINE),
                l,
                TEXT,
                Anchor::Middle,
                false,
            );
        }
    }

    fn dim(&mut self, a: DVec2, b: DVec2, offset: f64, text: String) {
        self.items.push(PlanItem::Dimension {
            id: self.id,
            a,
            b,
            offset,
            text,
            look: TextLook {
                size: 2.0,
                end_size: 1.6,
                ..TextLook::default()
            },
            witness: None,
            breaks: vec![],
            below: None,
        });
    }
}

/// The drawing of one window or door, seen from outside, its bottom
/// left corner at `at`; with its sizes under it and on its right.
fn scheme(p: &mut Pencil, model: &Model, o: &Opening, at: DVec2) {
    let (w, h) = (o.width, o.height);
    let pt = |x: f64, y: f64| at + DVec2::new(x, y) * K;
    let door = o.kind == OpeningKind::Door;
    // a door as joinery: its leaf in its frame
    let spec = o.door.as_ref().filter(|_| door && o.division.is_none());
    let (jamb, head) = spec.map_or((0.0, 0.0), |d| d.members(w, h));
    let fw = match (&o.door, door) {
        (Some(d), true) => d.members(w, h).0,
        _ => o.frame.fitted(w, h, 1.0).width,
    };
    let mut xs = vec![0.0, w];
    if let Some(d) = spec {
        // the opening lightly, the leaf in it with its handle
        p.rect(pt(0.0, 0.0), pt(w, h), Pen::Fine);
        p.rect(pt(jamb, 0.0), pt(w - jamb, h - head), Pen::Cut);
        let hx = w - jamb - 0.07;
        p.line(pt(hx, 1.0), pt(hx - 0.12, 1.0), Pen::Cut);
        p.line(pt(hx, 0.95), pt(hx, 1.05), Pen::Cut);
        xs = vec![0.0, jamb, w - jamb, w];
        let _ = d;
    } else {
        p.rect(pt(0.0, 0.0), pt(w, h), Pen::Cut);
        let node = division_of(o);
        let (fields, bars) = node.layout(w, h, fw, fw, !door);
        for b in &bars {
            p.rect(pt(b.x0, b.y0), pt(b.x1, b.y1), Pen::Symbol);
        }
        let bottom = fields.iter().map(|f| f.y0).fold(f64::MAX, f64::min);
        for f in &fields {
            let op = f.sash.op;
            let sw = if op == LeafOp::Fixed || op.is_door() {
                0.0
            } else {
                (fw * 0.75)
                    .min((f.x1 - f.x0) / 4.0)
                    .min((f.y1 - f.y0) / 4.0)
            };
            p.rect(pt(f.x0, f.y0), pt(f.x1, f.y1), Pen::Symbol);
            if sw > 0.0 {
                p.rect(
                    pt(f.x0 + sw, f.y0 + sw),
                    pt(f.x1 - sw, f.y1 - sw),
                    Pen::Symbol,
                );
            }
            if f.sash.panel {
                p.line(
                    pt(f.x0 + sw, f.y0 + sw),
                    pt(f.x1 - sw, f.y1 - sw),
                    Pen::Fine,
                );
            }
            for l in sash_symbol(f, sw) {
                p.line(pt(l[0], l[1]), pt(l[2], l[3]), Pen::Fine);
            }
            // the handle, on the stile away from the hinges
            if let Some(left) = op.hinged_left().or(op.slides_left()) {
                let x = if left {
                    f.x1 - sw / 2.0
                } else {
                    f.x0 + sw / 2.0
                };
                let y = ((f.y0 + f.y1) / 2.0).min(f.y0 + 1.05);
                p.line(pt(x, y - 0.06), pt(x, y + 0.06), Pen::Cut);
            }
            // the fields along the bottom are measured, to the middle
            // of the bars between them
            if (f.y0 - bottom).abs() < 1e-6 && f.x1 < w - fw - 1e-6 {
                xs.push(f.x1 + fw / 2.0);
            }
        }
        xs.sort_by(|a, b| a.total_cmp(b));
    }
    // sizes: the parts under it, the whole under those; heights on its
    // right
    let n = |v: f64| num(model, v);
    let parts = xs.len() > 2;
    if parts {
        for pair in xs.windows(2) {
            p.dim(
                pt(pair[0], 0.0),
                pt(pair[1], 0.0),
                -3.5,
                n(pair[1] - pair[0]),
            );
        }
    }
    p.dim(
        pt(0.0, 0.0),
        pt(w, 0.0),
        if parts { -8.0 } else { -4.0 },
        n(w),
    );
    if spec.is_some() && head > 1e-6 {
        p.dim(pt(w, 0.0), pt(w, h - head), -4.0, n(h - head));
        p.dim(pt(w, h - head), pt(w, h), -4.0, n(head));
        p.dim(pt(w, 0.0), pt(w, h), -9.0, n(h));
    } else {
        p.dim(pt(w, 0.0), pt(w, h), -4.5, n(h));
    }
}

/// The list as a drawing in paper millimetres, its top left corner at
/// the origin.
pub fn drawing(view: ElementId, title: &str, model: &Model, kind: OpeningKind) -> PlanView {
    let es = entries(model, kind);
    let door = kind == OpeningKind::Door;
    let mut items = vec![];
    let width: f64 = COLS.iter().sum();
    let x_of = |i: usize| COLS[..i].iter().sum::<f64>();
    let mut p = Pencil {
        items: &mut items,
        id: view,
    };
    p.text(
        DVec2::new(0.0, TITLE / 2.0),
        title,
        5.0,
        Anchor::Start,
        false,
    );
    // the heading
    let heads: [(usize, f64, String); 6] = [
        (0, COLS[0], tr("Mark")),
        (
            1,
            COLS[1],
            if door {
                format!(
                    "{}\n{}\n{}",
                    tr("Leaf"),
                    tr("Structural opening"),
                    tr("w × h (mm)")
                )
            } else {
                format!("{}\n{}", tr("Structural opening"), tr("w × h (mm)"))
            },
        ),
        (2, COLS[2], tr("Schematic drawing")),
        (6, COLS[6], tr("Execution")),
        (7, COLS[7], tr("Finish")),
        (8, COLS[8], tr("Note")),
    ];
    for (i, w, text) in &heads {
        let lines = wrap(text, w - 2.0, TEXT);
        let top = -HEAD / 2.0 + (lines.len() as f64 - 1.0) * LINE / 2.0;
        for (k, l) in lines.iter().enumerate() {
            p.text(
                DVec2::new(x_of(*i) + w / 2.0, top - k as f64 * LINE),
                l,
                TEXT,
                Anchor::Middle,
                false,
            );
        }
    }
    // level, and the pieces left and right
    p.text(
        DVec2::new(x_of(3) + COLS[3] / 2.0, -HEAD / 2.0),
        &tr("Level"),
        TEXT,
        Anchor::Middle,
        false,
    );
    let pieces = COLS[4] + COLS[5];
    p.text(
        DVec2::new(x_of(4) + pieces / 2.0, -HEAD / 4.0),
        &tr("Number of pieces"),
        TEXT,
        Anchor::Middle,
        false,
    );
    if door {
        for (i, t) in [(4, tr("Left hand")), (5, tr("Right hand"))] {
            p.text(
                DVec2::new(x_of(i) + COLS[i] / 2.0, -HEAD * 0.75),
                &t,
                1.8,
                Anchor::Middle,
                false,
            );
        }
        p.line(
            DVec2::new(x_of(5), -HEAD / 2.0),
            DVec2::new(x_of(5), -HEAD),
            Pen::Symbol,
        );
    }
    p.line(
        DVec2::new(x_of(4), -HEAD / 2.0),
        DVec2::new(x_of(6), -HEAD / 2.0),
        Pen::Symbol,
    );
    // the rows
    let mut y = -HEAD;
    let mut ys = vec![0.0, y];
    for en in &es {
        let o = &en.opening;
        let text_lines = |t: &str, w: f64| wrap(t, w - 3.0, TEXT).len() as f64;
        let need = text_lines(&en.execution, COLS[6])
            .max(text_lines(&en.finish, COLS[7]))
            .max(text_lines(&en.note, COLS[8]))
            * LINE
            + 5.0;
        let counts = (en.counts.len() as f64 + 1.0) * 6.0;
        let row = MIN_ROW.max(o.height * K + 17.0).max(need).max(counts);
        p.id = en.ids.first().copied().unwrap_or(view);
        let mid = y - row / 2.0;
        let marks = wrap(&en.mark, COLS[0] - 2.0, TEXT);
        for (i, l) in marks.iter().enumerate() {
            p.text(
                DVec2::new(
                    x_of(0) + COLS[0] / 2.0,
                    mid + ((marks.len() as f64 - 1.0) / 2.0 - i as f64) * LINE,
                ),
                l,
                TEXT,
                Anchor::Middle,
                true,
            );
        }
        let size = |w: f64, h: f64| format!("{} × {}", num(model, w), num(model, h));
        let cx = x_of(1) + COLS[1] / 2.0;
        match &o.door {
            Some(d) if door => {
                p.text(
                    DVec2::new(cx, mid + LINE),
                    &size(d.nom_width, d.nom_height),
                    TEXT,
                    Anchor::Middle,
                    false,
                );
                p.text(
                    DVec2::new(cx, mid - LINE),
                    &size(o.width, o.height),
                    TEXT,
                    Anchor::Middle,
                    false,
                );
            }
            _ => p.text(
                DVec2::new(cx, mid),
                &size(o.width, o.height),
                TEXT,
                Anchor::Middle,
                false,
            ),
        }
        // the drawing, in the middle of its cell, room for its sizes
        // under it and on its right
        let at = DVec2::new(
            x_of(2) + (COLS[2] - o.width * K) / 2.0 - 4.0,
            y - row + (row - o.height * K) / 2.0 + 4.0,
        );
        scheme(&mut p, model, o, at);
        // how many on each level, and in all
        let sub = (row / (en.counts.len() as f64 + 1.0)).min(10.0);
        let total_y = y - row + sub;
        for (i, (name, l, r)) in en.counts.iter().enumerate() {
            let cy = y - (i as f64 + 0.5) * (row - sub) / en.counts.len() as f64;
            p.text(
                DVec2::new(x_of(3) + COLS[3] / 2.0, cy),
                name,
                TEXT,
                Anchor::Middle,
                false,
            );
            let shown = |v: u32| {
                if v == 0 {
                    "-".to_string()
                } else {
                    v.to_string()
                }
            };
            if door {
                p.text(
                    DVec2::new(x_of(4) + COLS[4] / 2.0, cy),
                    &shown(*l),
                    TEXT,
                    Anchor::Middle,
                    false,
                );
                p.text(
                    DVec2::new(x_of(5) + COLS[5] / 2.0, cy),
                    &shown(*r),
                    TEXT,
                    Anchor::Middle,
                    false,
                );
            } else {
                p.text(
                    DVec2::new(x_of(4) + pieces / 2.0, cy),
                    &shown(*l),
                    TEXT,
                    Anchor::Middle,
                    false,
                );
            }
            if i > 0 {
                let ly = y - i as f64 * (row - sub) / en.counts.len() as f64;
                p.line(
                    DVec2::new(x_of(3), ly),
                    DVec2::new(x_of(6), ly),
                    Pen::Symbol,
                );
            }
        }
        p.line(
            DVec2::new(x_of(3), total_y),
            DVec2::new(x_of(6), total_y),
            Pen::Cut,
        );
        let (tl, trr) = en.total();
        let ty = y - row + sub / 2.0;
        p.text(
            DVec2::new(x_of(3) + COLS[3] / 2.0, ty),
            &tr("TOTAL"),
            TEXT,
            Anchor::Middle,
            true,
        );
        if door {
            for (i, v) in [(4, tl), (5, trr)] {
                p.text(
                    DVec2::new(x_of(i) + COLS[i] / 2.0, ty),
                    &if v == 0 { "-".into() } else { v.to_string() },
                    TEXT,
                    Anchor::Middle,
                    true,
                );
            }
            p.line(
                DVec2::new(x_of(5), y),
                DVec2::new(x_of(5), y - row),
                Pen::Symbol,
            );
        } else {
            p.text(
                DVec2::new(x_of(4) + pieces / 2.0, ty),
                &tl.to_string(),
                TEXT,
                Anchor::Middle,
                true,
            );
        }
        p.block(x_of(6), COLS[6], y, &en.execution);
        p.block(x_of(7), COLS[7], y, &en.finish);
        p.block(x_of(8), COLS[8], y, &en.note);
        y -= row;
        ys.push(y);
    }
    p.id = view;
    if es.is_empty() {
        y -= MIN_ROW / 2.0;
        p.text(
            DVec2::new(width / 2.0, y + MIN_ROW / 4.0),
            &tr(if door {
                "No doors in the model"
            } else {
                "No windows in the model"
            }),
            TEXT,
            Anchor::Middle,
            false,
        );
        ys.push(y);
    }
    for (i, yy) in ys.iter().enumerate() {
        p.line(
            DVec2::new(0.0, *yy),
            DVec2::new(width, *yy),
            if i < 2 || i + 1 == ys.len() {
                Pen::Cut
            } else {
                Pen::Symbol
            },
        );
    }
    for i in 0..=COLS.len() {
        // the line between left and right is drawn with the rows
        if i == 5 {
            continue;
        }
        let x = if i == COLS.len() { width } else { x_of(i) };
        p.line(
            DVec2::new(x, 0.0),
            DVec2::new(x, y),
            if i == 0 || i == COLS.len() {
                Pen::Cut
            } else {
                Pen::Symbol
            },
        );
    }
    PlanView { items }
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::joinery::{apply_door, DoorFrame, DoorSpec, FrameNode};
    use creabim_core::{Document, Transaction};

    fn house() -> Model {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let all: Vec<_> = doc
            .model()
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Opening(_)))
            .cloned()
            .collect();
        let mut tx = Transaction::new("joinery");
        for before in all {
            let mut after = before.clone();
            if let ElementKind::Opening(o) = &mut after.kind {
                match o.kind {
                    OpeningKind::Door => {
                        apply_door(o, DoorSpec::fitting(DoorFrame::Lining, 0.9, 2.02))
                    }
                    OpeningKind::Window => {
                        o.width = 1.2;
                        o.height = 1.5;
                        o.division = Some(FrameNode::parse("v(1:OS-L,1:FIX)").unwrap());
                    }
                    OpeningKind::Hole => {}
                }
            }
            tx = tx.replace(before, after);
        }
        doc.commit(tx).unwrap();
        doc.model().clone()
    }

    #[test]
    fn those_alike_are_one_kind_counted_by_level() {
        let m = house();
        let windows = m
            .iter()
            .filter(|e| matches!(&e.kind, ElementKind::Opening(o) if o.kind == OpeningKind::Window))
            .count() as u32;
        let es = entries(&m, OpeningKind::Window);
        assert_eq!(es.len(), 1, "all the windows were made alike");
        assert_eq!(es[0].total(), (windows, 0));
        assert_eq!(es[0].ids.len() as u32, windows);
        assert!(!es[0].mark.is_empty());
        let ds = entries(&m, OpeningKind::Door);
        assert!(!ds.is_empty());
        let (l, r) = ds.iter().fold((0, 0), |a, e| {
            let t = e.total();
            (a.0 + t.0, a.1 + t.1)
        });
        let doors = m
            .iter()
            .filter(|e| matches!(&e.kind, ElementKind::Opening(o) if o.kind == OpeningKind::Door))
            .count() as u32;
        assert_eq!(l + r, doors);
        // a lining says so in the note
        assert_eq!(ds[0].note, "Timber lining with architraves");
    }

    #[test]
    fn the_list_draws_each_kind_with_its_sizes() {
        let m = house();
        let d = drawing(ElementId::NIL, "Door list", &m, OpeningKind::Door);
        let dims: Vec<&str> = d
            .items
            .iter()
            .filter_map(|i| match i {
                PlanItem::Dimension { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        // 50 | 800 | 50 under 900, and 1970 | 50 beside 2020
        for want in ["800", "900", "1970", "2020", "50"] {
            assert!(dims.contains(&want), "{want} in {dims:?}");
        }
        let texts: Vec<&str> = d
            .items
            .iter()
            .filter_map(|i| match i {
                PlanItem::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(texts.contains(&"800 × 1970") && texts.contains(&"900 × 2020"));
        assert!(texts.contains(&"TOTAL"));
        let (lo, hi) = d.bounds().unwrap();
        assert!(hi.x - lo.x >= COLS.iter().sum::<f64>() - 1e-6);
        let w = drawing(ElementId::NIL, "Window list", &m, OpeningKind::Window);
        // two fields measured under the window, and the whole
        let n = w
            .items
            .iter()
            .filter(|i| matches!(i, PlanItem::Dimension { .. }))
            .count();
        assert_eq!(n, 4);
    }

    #[test]
    fn long_text_wraps_at_its_spaces() {
        let lines = wrap(
            "Timber door without glazing, internal, single leaf",
            24.0,
            TEXT,
        );
        assert!(lines.len() >= 2);
        assert!(lines.iter().all(|l| !l.starts_with(' ') && !l.is_empty()));
    }
}
