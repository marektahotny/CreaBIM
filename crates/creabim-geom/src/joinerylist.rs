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

/// How many leaves, in a word: "single-leaf".
fn leaves_word(n: usize) -> String {
    match n {
        0 | 1 => tr("single-leaf"),
        2 => tr("two-leaf"),
        3 => tr("three-leaf"),
        n => tr("{}-leaf").replace("{}", &n.to_string()),
    }
}

/// What a window's profiles are made of, read from its material's name
/// as PilonDesigner sorts them; plastic where the name tells nothing.
fn profiles(model: &Model, id: ElementId) -> &'static str {
    let (name, category) = model
        .get(id)
        .and_then(|e| creabim_core::material_of(model, e))
        .map(|(e, m)| (e.name.to_lowercase(), m.category.to_lowercase()))
        .unwrap_or_default();
    // a plastic one by its category, though its foil looks like oak
    if category == "plastic" || name.contains("pvc") || name.contains("plast") {
        return "Plastic window profiles";
    }
    let has = |w: &[&str]| w.iter().any(|w| name.contains(w));
    let wood = has(&[
        "drev", "dřev", "wood", "timber", "smrek", "dub", "oak", "pine",
    ]);
    let alu = has(&["hliník", "hlinik", "alumin", "alu"]);
    match (wood, alu) {
        (true, true) => "Timber-aluminium window profiles",
        (true, false) => "Timber window profiles",
        (false, true) => "Aluminium window profiles",
        _ => "Plastic window profiles",
    }
}

/// Whether the door `id` in its wall leads outside: no room on one side
/// of its wall, a room on the other; none where the storey has no
/// rooms to tell by, or rooms on neither side.
pub fn leads_outside(model: &Model, id: ElementId) -> Option<bool> {
    let ElementKind::Opening(o) = &model.get(id)?.kind else {
        return None;
    };
    let ElementKind::Wall(w) = &model.get(o.host)?.kind else {
        return None;
    };
    let along = w.end - w.start;
    let len = along.length();
    if len < 1e-6 {
        return None;
    }
    let dir = along / len;
    let at = w.start + dir * (o.offset + o.width / 2.0);
    // past the wall's faces wherever its line runs in it
    let d = w.thickness + 0.25;
    let rooms: Vec<Vec<DVec2>> = model
        .iter()
        .filter(|e| matches!(&e.kind, ElementKind::Room(r) if r.level == w.level))
        .filter_map(|e| creabim_core::room::outline(model, e.id))
        .collect();
    if rooms.is_empty() {
        return None;
    }
    let roomed = |p: DVec2| rooms.iter().any(|r| creabim_core::polygon::contains(r, p));
    match (roomed(at + dir.perp() * d), roomed(at - dir.perp() * d)) {
        (true, true) => Some(false),
        (true, false) | (false, true) => Some(true),
        (false, false) => None,
    }
}

/// What the list says of `o` where its own texts say nothing, worded
/// as PilonDesigner's list and the lists of Slovak practice: how it is
/// made and opens, its finish (its material), and a note (a window's
/// glazing and outer sill, a door's frame).
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
    let execution = if !own.execution.is_empty() {
        own.execution
    } else if o.kind == OpeningKind::Door {
        // a door drawn by its division has glass where a field is no panel
        let glazed = o.division.is_some() && leaves.iter().any(|l| !l.panel);
        let plastic = model
            .get(id)
            .and_then(|e| creabim_core::material_of(model, e))
            .is_some_and(|(_, m)| m.category.eq_ignore_ascii_case("plastic"));
        let mut s = match &o.door {
            _ if plastic => tr("Plastic door"),
            Some(d) => tr(match d.leaf {
                creabim_core::joinery::DoorMaterial::Timber => "Timber door",
                creabim_core::joinery::DoorMaterial::Steel => "Steel door",
                creabim_core::joinery::DoorMaterial::Aluminium => "Aluminium door",
                creabim_core::joinery::DoorMaterial::Plastic => "Plastic door",
            }),
            None => tr("Door"),
        };
        s.push(' ');
        s.push_str(&tr(if glazed { "glazed" } else { "without glazing" }));
        if let Some(out) = leads_outside(model, id) {
            s.push_str(&format!(
                ", {}",
                tr(if out { "external" } else { "internal" })
            ));
        }
        let doors = leaves.iter().filter(|l| l.op.is_door()).count().max(1);
        s.push_str(&format!(", {}", leaves_word(doors)));
        if leaves.len() > doors {
            s.push_str(&format!(" {}", tr("with a side light")));
        }
        s.push(' ');
        s.push_str(&tr(if o.threshold {
            "with a threshold"
        } else {
            "without a threshold"
        }));
        if let Some(f) = o.door.as_ref().and_then(|d| d.fire.as_ref()) {
            s.push_str(&format!(", {} {}", tr("fire door"), f.label()));
        }
        s
    } else {
        let moving = leaves.iter().filter(|l| l.op != LeafOp::Fixed).count();
        let mut s = format!("{}, {}", tr(profiles(model, id)), leaves_word(moving));
        if !ops.is_empty() {
            s.push_str(&format!(", {ops}"));
        }
        s
    };
    let finish = if !own.finish.is_empty() {
        own.finish
    } else {
        model
            .get(id)
            .and_then(|e| creabim_core::material_of(model, e))
            .map(|(m, _)| tr(&m.name))
            .unwrap_or_default()
    };
    let note = if !own.note.is_empty() {
        own.note
    } else if o.kind == OpeningKind::Door {
        o.door
            .as_ref()
            .map(|d| tr(d.frame.label()))
            .unwrap_or_default()
    } else {
        // the glazing PilonDesigner's list writes, and the outer sill
        // by its mark in the list of sheet-metal work
        let mut s = tr("Insulating triple glazing");
        if let Some(k) = creabim_core::joinery::outer_sill_of(model, id) {
            s.push_str(&format!(
                ", {} {}/{}",
                tr("outer sill"),
                creabim_core::joinery::SILL_LETTER,
                k.number
            ));
        }
        s
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

/// The list's columns, each by its heading (the key its width set on the
/// sheet is kept under) and its own width, paper mm: mark, sizes,
/// drawing, level, the pieces (doors: left and right), make, finish,
/// note.
pub fn columns(kind: OpeningKind) -> Vec<(&'static str, f64)> {
    let mut c = vec![
        ("Mark", 13.0),
        ("Structural opening", 25.0),
        ("Schematic drawing", 50.0),
        ("Level", 9.0),
    ];
    if kind == OpeningKind::Door {
        c.push(("Left hand", 7.0));
        c.push(("Right hand", 7.0));
    } else {
        c.push(("Number of pieces", 12.0));
    }
    c.extend([("Execution", 32.0), ("Finish", 28.0), ("Note", 28.0)]);
    c
}

/// The widths of `view`'s columns: those dragged on the sheet over the
/// columns' own.
pub fn widths(model: &Model, view: ElementId, kind: OpeningKind) -> Vec<f64> {
    let set = match model.get(view).map(|e| &e.kind) {
        Some(ElementKind::View(v)) => Some(&v.column_widths),
        _ => None,
    };
    columns(kind)
        .iter()
        .map(|(t, w)| {
            set.and_then(|s| s.get(*t))
                .map_or(*w, |w| w.max(crate::schedule::MIN_COLUMN))
        })
        .collect()
}

/// The lines between `view`'s columns a drag on the sheet moves: each
/// column's heading, where it ends and its width; none for a view not
/// a window or door list.
pub fn column_edges(model: &Model, view: ElementId) -> Option<Vec<(String, f64, f64)>> {
    let Some(ElementKind::View(v)) = model.get(view).map(|e| &e.kind) else {
        return None;
    };
    let creabim_core::ViewKind::Schedule { category, .. } = v.kind else {
        return None;
    };
    let kind = category.joinery_list()?;
    let mut x = 0.0;
    Some(
        columns(kind)
            .iter()
            .zip(widths(model, view, kind))
            .map(|((t, _), w)| {
                x += w;
                (t.to_string(), x, w)
            })
            .collect(),
    )
}

const HEAD: f64 = 16.0;
const TITLE: f64 = 10.0;
const TEXT: f64 = 2.2;
const LINE: f64 = 3.3;
/// Paper mm to a metre of the drawings: 1:100, smaller where the
/// drawing's column is too narrow for the widest.
const K: f64 = 10.0;
const MIN_ROW: f64 = 30.0;
/// The height of the row of totals, paper mm.
const TOTAL_ROW: f64 = 9.0;

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

    /// Text reading upwards, its lines centred on `at`.
    fn vtext(&mut self, at: DVec2, text: &str, size: f64, bold: bool) {
        self.items.push(PlanItem::Annotation {
            id: self.id,
            item: Box::new(crate::annotation::Annotation {
                shape: crate::annotation::Shape::Along {
                    at,
                    angle: std::f64::consts::FRAC_PI_2,
                },
                text: text.to_string(),
                look: TextLook {
                    size,
                    bold,
                    ..TextLook::default()
                },
                mm: 1.0,
            }),
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
            place: Default::default(),
        });
    }
}

/// The drawing of one window or door, seen from outside, its bottom
/// left corner at `at`; with its sizes under it and on its right.
fn scheme(p: &mut Pencil, model: &Model, o: &Opening, at: DVec2, k: f64) {
    let (w, h) = (o.width, o.height);
    let pt = |x: f64, y: f64| at + DVec2::new(x, y) * k;
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
        let (fields, bars) = node.layout_of(door, w, h, fw);
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
            // how it opens, dashed
            for l in sash_symbol(f, sw) {
                p.line(pt(l[0], l[1]), pt(l[2], l[3]), Pen::Hidden);
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

/// The scale of a list's drawings: 1:100, smaller where the drawing's
/// column is too narrow for the widest.
fn scale_of(es: &[Entry], cols: &[f64]) -> f64 {
    let widest = es.iter().map(|e| e.opening.width).fold(0.0, f64::max);
    if widest > 0.0 {
        K.min((cols[2] - 20.0).max(5.0) / widest)
    } else {
        K
    }
}

/// How tall the row of `en` is, paper mm: room for its drawing, its
/// words and its levels' names on their side.
fn row_height(en: &Entry, cols: &[f64], exe: usize, k: f64) -> f64 {
    let text_lines = |t: &str, w: f64| wrap(t, w - 3.0, TEXT).len() as f64;
    let need = text_lines(&en.execution, cols[exe])
        .max(text_lines(&en.finish, cols[exe + 1]))
        .max(text_lines(&en.note, cols[exe + 2]))
        * LINE
        + 5.0;
    let names = en
        .counts
        .iter()
        .map(|c| crate::paper::text_width(&tr(&c.0), TEXT) + 4.0)
        .sum::<f64>();
    MIN_ROW
        .max(en.opening.height * k + 17.0)
        .max(need)
        .max(names + TOTAL_ROW)
}

/// The rows a page of a list holds at most, paper mm: as many as an A4
/// sheet takes under its heading, above the title block (ASR_008).
pub const PAGE: f64 = 190.0;

/// The kinds of a long list page by page (Okná 1, Okná 2), each page's
/// rows no taller than [`PAGE`] in the columns' own widths; one page
/// for a short list.
pub fn pages(model: &Model, kind: OpeningKind) -> Vec<std::ops::Range<usize>> {
    let es = entries(model, kind);
    let cols: Vec<f64> = columns(kind).iter().map(|c| c.1).collect();
    let exe = if kind == OpeningKind::Door { 6 } else { 5 };
    let k = scale_of(&es, &cols);
    let mut out = vec![];
    let (mut from, mut h) = (0, 0.0);
    for (i, en) in es.iter().enumerate() {
        let r = row_height(en, &cols, exe, k);
        if i > from && h + r > PAGE {
            out.push(from..i);
            from = i;
            h = 0.0;
        }
        h += r;
    }
    out.push(from..es.len());
    out
}

/// The list as a drawing in paper millimetres, its top left corner at
/// the origin; laid out as the lists of PilonDesigner and of Slovak
/// practice: the level's name and the totals turned on their side, the
/// totals boxed, a note on measuring the openings under it all.
pub fn drawing(view: ElementId, title: &str, model: &Model, kind: OpeningKind) -> PlanView {
    let all = entries(model, kind);
    let door = kind == OpeningKind::Door;
    let cols = widths(model, view, kind);
    // one page of a long list; the last ends with the tables and note
    let page = match model.get(view).map(|e| &e.kind) {
        Some(ElementKind::View(v)) => v.page,
        _ => None,
    };
    let split = pages(model, kind);
    let (es, last) = match page {
        Some(n) => {
            let i = (n.max(1) as usize - 1).min(split.len() - 1);
            (&all[split[i].clone()], i + 1 == split.len())
        }
        None => (&all[..], true),
    };
    // the pieces' columns: doors left and right, windows one
    let np = if door { 2 } else { 1 };
    let (lev, pcs, exe) = (3, 4, 4 + np);
    let mut items = vec![];
    let width: f64 = cols.iter().sum();
    let x_of = |i: usize| cols[..i].iter().sum::<f64>();
    let mid_of = |i: usize| x_of(i) + cols[i] / 2.0;
    // one scale for every drawing of the list, small enough for the
    // widest in its column
    let k = scale_of(&all, &cols);
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
    let size_head = if door {
        format!(
            "{}\n{}\n{}",
            tr("Leaf"),
            tr("Structural opening"),
            tr("w × h (mm)")
        )
    } else {
        format!("{}\n{}", tr("Structural opening"), tr("w × h (mm)"))
    };
    let heads: [(usize, String); 6] = [
        (0, tr("Mark")),
        (1, size_head),
        (2, tr("Schematic drawing")),
        (exe, tr("Execution")),
        (exe + 1, tr("Finish")),
        (exe + 2, tr("Note")),
    ];
    for (i, text) in &heads {
        let lines = wrap(text, cols[*i] - 2.0, TEXT);
        let top = -HEAD / 2.0 + (lines.len() as f64 - 1.0) * LINE / 2.0;
        for (n, l) in lines.iter().enumerate() {
            p.text(
                DVec2::new(mid_of(*i), top - n as f64 * LINE),
                l,
                TEXT,
                Anchor::Middle,
                false,
            );
        }
    }
    // the level's name on its side, the pieces' heading small over them
    let small = 1.6;
    p.vtext(
        DVec2::new(mid_of(lev), -HEAD / 2.0),
        &wrap(&tr("Level"), HEAD - 2.0, small).join("\n"),
        small,
        false,
    );
    let pieces: f64 = cols[pcs..exe].iter().sum();
    if door {
        p.text(
            DVec2::new(x_of(pcs) + pieces / 2.0, -2.0),
            &tr("Number of pieces"),
            1.4,
            Anchor::Middle,
            false,
        );
        p.line(
            DVec2::new(x_of(pcs), -4.0),
            DVec2::new(x_of(exe), -4.0),
            Pen::Symbol,
        );
        for i in [pcs, pcs + 1] {
            p.vtext(
                DVec2::new(mid_of(i), -4.0 - (HEAD - 4.0) / 2.0),
                &tr(columns(kind)[i].0),
                small,
                false,
            );
        }
        p.line(
            DVec2::new(x_of(pcs + 1), -4.0),
            DVec2::new(x_of(pcs + 1), -HEAD),
            Pen::Symbol,
        );
    } else {
        let lines = wrap(&tr("Number of pieces"), pieces - 1.0, small);
        let top = -HEAD / 2.0 + (lines.len() as f64 - 1.0) * LINE * 0.4;
        for (n, l) in lines.iter().enumerate() {
            p.text(
                DVec2::new(x_of(pcs) + pieces / 2.0, top - n as f64 * LINE * 0.8),
                l,
                small,
                Anchor::Middle,
                false,
            );
        }
    }
    // the rows
    let mut y = -HEAD;
    let mut ys = vec![0.0, y];
    let shown = |v: u32| {
        if v == 0 {
            "-".to_string()
        } else {
            v.to_string()
        }
    };
    for en in es {
        let o = &en.opening;
        let row = row_height(en, &cols, exe, k);
        p.id = en.ids.first().copied().unwrap_or(view);
        let mid = y - row / 2.0;
        let marks = wrap(&en.mark, cols[0] - 2.0, TEXT);
        for (i, l) in marks.iter().enumerate() {
            p.text(
                DVec2::new(
                    mid_of(0),
                    mid + ((marks.len() as f64 - 1.0) / 2.0 - i as f64) * LINE,
                ),
                l,
                TEXT,
                Anchor::Middle,
                true,
            );
        }
        let size = |w: f64, h: f64| format!("{} × {}", num(model, w), num(model, h));
        match &o.door {
            Some(d) if door => {
                p.text(
                    DVec2::new(mid_of(1), mid + LINE),
                    &size(d.nom_width, d.nom_height),
                    TEXT,
                    Anchor::Middle,
                    false,
                );
                p.text(
                    DVec2::new(mid_of(1), mid - LINE),
                    &size(o.width, o.height),
                    TEXT,
                    Anchor::Middle,
                    false,
                );
            }
            _ => p.text(
                DVec2::new(mid_of(1), mid),
                &size(o.width, o.height),
                TEXT,
                Anchor::Middle,
                false,
            ),
        }
        // the drawing, in the middle of its cell, room for its sizes
        // under it and on its right
        let at = DVec2::new(
            x_of(2) + (cols[2] - o.width * k) / 2.0 - 4.0,
            y - row + (row - o.height * k) / 2.0 + 4.0,
        );
        scheme(&mut p, model, o, at, k);
        // how many on each level, the level's name on its side; the
        // totals boxed under them
        let n = en.counts.len().max(1) as f64;
        let each = (row - TOTAL_ROW) / n;
        for (i, (name, l, r)) in en.counts.iter().enumerate() {
            let cy = y - (i as f64 + 0.5) * each;
            p.vtext(DVec2::new(mid_of(lev), cy), &tr(name), TEXT, false);
            if door {
                p.text(
                    DVec2::new(mid_of(pcs), cy),
                    &shown(*l),
                    TEXT,
                    Anchor::Middle,
                    false,
                );
                p.text(
                    DVec2::new(mid_of(pcs + 1), cy),
                    &shown(*r),
                    TEXT,
                    Anchor::Middle,
                    false,
                );
            } else {
                p.text(
                    DVec2::new(x_of(pcs) + pieces / 2.0, cy),
                    &shown(*l),
                    TEXT,
                    Anchor::Middle,
                    false,
                );
            }
            if i > 0 {
                let ly = y - i as f64 * each;
                p.line(
                    DVec2::new(x_of(lev), ly),
                    DVec2::new(x_of(exe), ly),
                    Pen::Symbol,
                );
            }
        }
        let bottom = y - row;
        p.rect(
            DVec2::new(x_of(lev), bottom),
            DVec2::new(x_of(exe), bottom + TOTAL_ROW),
            Pen::Cut,
        );
        let (tl, trr) = en.total();
        let ty = bottom + TOTAL_ROW / 2.0;
        p.vtext(DVec2::new(mid_of(lev), ty), &tr("TOTAL"), 1.8, true);
        if door {
            p.text(
                DVec2::new(mid_of(pcs), ty),
                &shown(tl),
                TEXT,
                Anchor::Middle,
                true,
            );
            p.text(
                DVec2::new(mid_of(pcs + 1), ty),
                &shown(trr),
                TEXT,
                Anchor::Middle,
                true,
            );
            p.line(
                DVec2::new(x_of(pcs + 1), y),
                DVec2::new(x_of(pcs + 1), bottom),
                Pen::Symbol,
            );
        } else {
            p.text(
                DVec2::new(x_of(pcs) + pieces / 2.0, ty),
                &tl.to_string(),
                TEXT,
                Anchor::Middle,
                true,
            );
        }
        p.block(x_of(exe), cols[exe], y, &en.execution);
        p.block(x_of(exe + 1), cols[exe + 1], y, &en.finish);
        p.block(x_of(exe + 2), cols[exe + 2], y, &en.note);
        y = bottom;
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
    for i in 0..=cols.len() {
        // the line between left and right is drawn with the rows
        if door && i == pcs + 1 {
            continue;
        }
        let x = if i == cols.len() { width } else { x_of(i) };
        p.line(
            DVec2::new(x, 0.0),
            DVec2::new(x, y),
            if i == 0 || i == cols.len() {
                Pen::Cut
            } else {
                Pen::Symbol
            },
        );
    }
    crate::annotate::place_dimension_texts(&mut items, 1.0);
    if !last {
        return PlanView { items };
    }
    // under the windows, their outer sills for the list of sheet-metal work
    if !door {
        let sills = creabim_core::joinery::outer_sills(model);
        if !sills.is_empty() {
            let bottom = PlanView {
                items: items.clone(),
            }
            .bounds()
            .map_or(0.0, |b| b.0.y);
            let mut p = Pencil {
                items: &mut items,
                id: view,
            };
            let mut y = bottom - 12.0;
            p.text(
                DVec2::new(0.0, y),
                &tr("Outer window sills (sheet metal)"),
                3.5,
                Anchor::Start,
                false,
            );
            y -= 7.0;
            let cols = [0.0, 20.0, 50.0, 80.0, 110.0];
            let heads = [
                tr("Mark"),
                tr("Length (mm)"),
                tr("Depth (mm)"),
                tr("Girth (mm)"),
                tr("Number of pieces"),
            ];
            for (x, h) in cols.iter().zip(&heads) {
                p.text(DVec2::new(*x, y), h, TEXT, Anchor::Start, false);
            }
            for s in &sills {
                y -= LINE + 1.0;
                // the sheet unfolded: the depth with its upstand and drip
                let girth = s.depth + 0.07;
                let row = [
                    format!("{}/{}", creabim_core::joinery::SILL_LETTER, s.number),
                    num(model, s.length),
                    num(model, s.depth),
                    num(model, girth),
                    s.windows.len().to_string(),
                ];
                for (x, t) in cols.iter().zip(&row) {
                    p.text(DVec2::new(*x, y), t, TEXT, Anchor::Start, false);
                }
            }
        }
    }
    // the insulation turned into the reveals, billed apart from the
    // walls': thin strips as thick as the reveal is set back
    let strips = creabim_core::joinery::reveal_strips(model, door);
    if !strips.is_empty() {
        let bottom = PlanView {
            items: items.clone(),
        }
        .bounds()
        .map_or(0.0, |b| b.0.y);
        let mut p = Pencil {
            items: &mut items,
            id: view,
        };
        let mut y = bottom - 12.0;
        p.text(
            DVec2::new(0.0, y),
            &tr("Reveal insulation (strips)"),
            3.5,
            Anchor::Start,
            false,
        );
        y -= 7.0;
        let cols = [0.0, 60.0, 90.0, 120.0, 150.0, 180.0];
        let heads = [
            tr("Material"),
            tr("Thickness (mm)"),
            tr("Width (mm)"),
            tr("Length (m)"),
            tr("Area (m²)"),
            tr("Openings"),
        ];
        for (x, h) in cols.iter().zip(&heads) {
            p.text(DVec2::new(*x, y), h, TEXT, Anchor::Start, false);
        }
        let mut total = 0.0;
        for s in &strips {
            y -= LINE + 1.0;
            total += s.area();
            let material = s
                .material
                .and_then(|m| model.get(m))
                .map_or_else(|| tr("Insulation"), |m| tr(&m.name));
            let row = [
                material,
                num(model, s.thickness),
                num(model, s.width),
                format!("{:.2}", s.length).replace('.', ","),
                format!("{:.2}", s.area()).replace('.', ","),
                s.openings.to_string(),
            ];
            for (x, t) in cols.iter().zip(&row) {
                p.text(DVec2::new(*x, y), t, TEXT, Anchor::Start, false);
            }
        }
        y -= LINE + 1.0;
        p.text(
            DVec2::new(cols[3], y),
            &tr("Total"),
            TEXT,
            Anchor::Start,
            true,
        );
        p.text(
            DVec2::new(cols[4], y),
            &format!("{total:.2}").replace('.', ","),
            TEXT,
            Anchor::Start,
            true,
        );
    }
    // the note every list of Slovak practice ends with
    let bottom = PlanView {
        items: items.clone(),
    }
    .bounds()
    .map_or(0.0, |b| b.0.y);
    let mut p = Pencil {
        items: &mut items,
        id: view,
    };
    let mut y = bottom - 8.0;
    p.text(DVec2::new(0.0, y), &tr("Note:"), 2.5, Anchor::Start, false);
    for line in [
        "Before the openings are made, the structural openings left in the walls must be measured exactly.",
        "The maker and the supplier are chosen by the client.",
    ] {
        p.text(DVec2::new(16.0, y), &tr(line), 2.5, Anchor::Start, false);
        y -= 4.0;
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
        // the totals and the levels' names stand on their side
        let turned: Vec<&str> = d
            .items
            .iter()
            .filter_map(|i| match i {
                PlanItem::Annotation { item, .. } => Some(item.text.as_str()),
                _ => None,
            })
            .collect();
        assert!(turned.contains(&"TOTAL"), "{turned:?}");
        let (lo, hi) = d.bounds().unwrap();
        assert!(hi.x - lo.x >= columns(OpeningKind::Door).iter().map(|c| c.1).sum::<f64>() - 1e-6);
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
    fn a_long_list_goes_page_by_page_and_doors_say_where_they_lead() {
        // windows each a size of its own: too many rows for one page
        let h = house();
        let ws: Vec<_> = h
            .iter()
            .filter(|e| matches!(&e.kind, ElementKind::Opening(o) if o.kind == OpeningKind::Window))
            .cloned()
            .collect();
        let mut tx = Transaction::new("sizes");
        for (i, before) in ws.into_iter().enumerate() {
            let mut after = before.clone();
            if let ElementKind::Opening(o) = &mut after.kind {
                o.width = 0.6 + 0.1 * i as f64;
            }
            tx = tx.replace(before, after);
        }
        let mut d = creabim_core::Document::from_model(h);
        d.commit(tx).unwrap();
        let m = d.model().clone();
        let split = pages(&m, OpeningKind::Window);
        assert!(split.len() >= 2, "{split:?}");
        assert_eq!(
            split.last().unwrap().end,
            entries(&m, OpeningKind::Window).len()
        );
        // the sheets: one a page
        let (tx, _) = crate::docset::drawing_set(&m).unwrap();
        let mut d = creabim_core::Document::from_model(m.clone());
        d.commit(tx).unwrap();
        let paged: Vec<u32> = d
            .model()
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::View(v)
                    if matches!(
                        v.kind,
                        creabim_core::ViewKind::Schedule {
                            category: creabim_core::ScheduleCategory::WindowList,
                            ..
                        }
                    ) =>
                {
                    v.page
                }
                _ => None,
            })
            .collect();
        assert_eq!(paged.len(), split.len(), "{paged:?}");
        // a room inside the house: a door in its outer wall leads out
        let level = m.levels().next().unwrap().0.id;
        let walls: Vec<DVec2> = m
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Wall(w) if w.level == level => Some(w.start),
                _ => None,
            })
            .collect();
        let mid = walls.iter().fold(DVec2::ZERO, |a, p| a + *p) / walls.len() as f64;
        let mut tx = Transaction::new("room");
        tx = tx.insert(creabim_core::Element::new(
            "Room",
            ElementKind::Room(creabim_core::Room::new(level, mid)),
        ));
        let mut d = creabim_core::Document::from_model(m);
        d.commit(tx).unwrap();
        let m = d.model();
        let said: Vec<Option<bool>> = m
            .iter()
            .filter(|e| matches!(&e.kind, ElementKind::Opening(o) if o.kind == OpeningKind::Door))
            .map(|e| leads_outside(m, e.id))
            .collect();
        assert!(said.iter().any(|x| x.is_some()), "{said:?}");
    }

    #[test]
    fn plastic_windows_in_ral_colours_say_so() {
        let h = house();
        let entry = creabim_core::library::LIBRARY
            .iter()
            .find(|e| e.name == "PVC, RAL 7016 anthracite")
            .unwrap();
        let mat = creabim_core::Element::new(entry.name, ElementKind::Material(entry.material()));
        let mut tx = Transaction::new("pvc").insert(mat.clone());
        for e in h
            .iter()
            .filter(|e| matches!(&e.kind, ElementKind::Opening(_)))
        {
            let mut after = e.clone();
            if let ElementKind::Opening(o) = &mut after.kind {
                o.material = Some(mat.id);
            }
            tx = tx.replace(e.clone(), after);
        }
        let mut d = creabim_core::Document::from_model(h);
        d.commit(tx).unwrap();
        let w = entries(d.model(), OpeningKind::Window);
        assert!(
            w[0].execution.starts_with("Plastic window profiles"),
            "{}",
            w[0].execution
        );
        assert_eq!(w[0].finish, "PVC, RAL 7016 anthracite");
        let ds = entries(d.model(), OpeningKind::Door);
        assert!(
            ds[0].execution.starts_with("Plastic door"),
            "{}",
            ds[0].execution
        );
    }

    #[test]
    fn a_plastic_door_by_its_frame_and_leaf() {
        let h = house();
        let mut tx = Transaction::new("pvc");
        for e in h
            .iter()
            .filter(|e| matches!(&e.kind, ElementKind::Opening(o) if o.kind == OpeningKind::Door))
        {
            let mut after = e.clone();
            if let ElementKind::Opening(o) = &mut after.kind {
                let mut d = DoorSpec::fitting(DoorFrame::Plastic, 1.06, 2.05);
                d.leaf = creabim_core::joinery::DoorMaterial::Plastic;
                apply_door(o, d);
            }
            tx = tx.replace(e.clone(), after);
        }
        let mut d = creabim_core::Document::from_model(h);
        d.commit(tx).unwrap();
        let ds = entries(d.model(), OpeningKind::Door);
        assert!(
            ds[0].execution.starts_with("Plastic door"),
            "{}",
            ds[0].execution
        );
        assert_eq!(ds[0].note, "Plastic frame");
        // 900 leaf in a plastic frame: an opening 1060 × 2050
        assert_eq!((ds[0].opening.width, ds[0].opening.height), (1.06, 2.05));
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
