//! What a door or window is made of beyond its hole in the wall
//! (PilonDesigner's `windowframe.ts` and `doors.ts`): how it is divided
//! into fields by mullions and transoms, each field a leaf opening its
//! own way, and for a door its leaf's nominal size, its frame and its
//! fire rating (STN 92 0201, STN EN 13501-2).
//!
//! A division is seen from OUTSIDE: `x` from the left, `y` up from the
//! bottom, hinges on a leaf's left or right as seen from there.

use crate::{LeafOp, Opening, OpeningKind};
use serde::{Deserialize, Serialize};

/// One field of a division: how it opens and what fills it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sash {
    pub op: LeafOp,
    /// A solid panel instead of glass.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub panel: bool,
}

/// A division: a field, or fields side by side (split by mullions,
/// left to right) or one above the other (split by transoms, top to
/// bottom), each with its share of the room.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrameNode {
    Leaf(Sash),
    Split {
        /// Side by side; else one above the other.
        across: bool,
        sizes: Vec<f64>,
        parts: Vec<FrameNode>,
    },
}

impl Default for FrameNode {
    fn default() -> Self {
        FrameNode::Leaf(Sash::default())
    }
}

/// A field as laid out, m, in the opening seen from outside.
#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub path: Vec<usize>,
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
    pub sash: Sash,
}

/// A mullion or transom as laid out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bar {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

/// The smallest share a split leaves a field.
const MIN_SHARE: f64 = 0.1;

impl FrameNode {
    pub fn leaf(op: LeafOp) -> FrameNode {
        FrameNode::Leaf(Sash { op, panel: false })
    }

    /// The fields and bars of an opening `w` × `h` whose frame is `fw`
    /// wide and whose mullions and transoms are `mw` wide. A door's
    /// frame has no bottom member (`threshold` false).
    pub fn layout(
        &self,
        w: f64,
        h: f64,
        fw: f64,
        mw: f64,
        threshold: bool,
    ) -> (Vec<Field>, Vec<Bar>) {
        let (mut fields, mut bars) = (vec![], vec![]);
        fn walk(
            n: &FrameNode,
            path: Vec<usize>,
            r: [f64; 4],
            mw: f64,
            fields: &mut Vec<Field>,
            bars: &mut Vec<Bar>,
        ) {
            let [x0, y0, x1, y1] = r;
            match n {
                FrameNode::Leaf(s) => fields.push(Field {
                    path,
                    x0,
                    y0,
                    x1,
                    y1,
                    sash: *s,
                }),
                FrameNode::Split {
                    across,
                    sizes,
                    parts,
                } => {
                    let n = parts.len().max(1);
                    let along = if *across { x1 - x0 } else { y1 - y0 };
                    let free = (along - mw * (n - 1) as f64).max(0.0);
                    let size = |i: usize| sizes.get(i).copied().unwrap_or(1.0).max(0.0);
                    let sum: f64 = (0..n).map(size).sum();
                    let sum = if sum > 1e-9 { sum } else { 1.0 };
                    // mullions left to right, transoms top to bottom
                    let mut c = if *across { x0 } else { y1 };
                    for (i, p) in parts.iter().enumerate() {
                        let len = free * size(i) / sum;
                        let mut pp = path.clone();
                        pp.push(i);
                        if *across {
                            walk(p, pp, [c, y0, c + len, y1], mw, fields, bars);
                            c += len;
                            if i + 1 < n {
                                bars.push(Bar {
                                    x0: c,
                                    y0,
                                    x1: c + mw,
                                    y1,
                                });
                                c += mw;
                            }
                        } else {
                            walk(p, pp, [x0, c - len, x1, c], mw, fields, bars);
                            c -= len;
                            if i + 1 < n {
                                bars.push(Bar {
                                    x0,
                                    y0: c - mw,
                                    x1,
                                    y1: c,
                                });
                                c -= mw;
                            }
                        }
                    }
                }
            }
        }
        let y0 = if threshold { fw } else { 0.0 };
        walk(
            self,
            vec![],
            [fw, y0, w - fw, h - fw],
            mw,
            &mut fields,
            &mut bars,
        );
        (fields, bars)
    }

    pub fn at(&self, path: &[usize]) -> Option<&FrameNode> {
        match (path.split_first(), self) {
            (None, _) => Some(self),
            (Some((i, rest)), FrameNode::Split { parts, .. }) => parts.get(*i)?.at(rest),
            _ => None,
        }
    }

    pub fn at_mut(&mut self, path: &[usize]) -> Option<&mut FrameNode> {
        match (path.split_first(), self) {
            (None, n) => Some(n),
            (Some((i, rest)), FrameNode::Split { parts, .. }) => parts.get_mut(*i)?.at_mut(rest),
            _ => None,
        }
    }

    /// Split the field at `path` by a mullion (`across`) or a transom at
    /// `t` of the way from its left or top. Into a parent split the same
    /// way the new field goes beside it, with no nesting.
    pub fn split(&mut self, path: &[usize], across: bool, t: f64) {
        let t = t.clamp(MIN_SHARE, 1.0 - MIN_SHARE);
        let Some(FrameNode::Leaf(s)) = self.at(path).cloned() else {
            return;
        };
        if let Some((i, parent)) = path.split_last() {
            if let Some(FrameNode::Split {
                across: a,
                sizes,
                parts,
            }) = self.at_mut(parent)
            {
                if *a == across && *i < parts.len() && *i < sizes.len() {
                    let size = sizes[*i];
                    parts.insert(*i, FrameNode::Leaf(s));
                    sizes[*i] = size * t;
                    sizes.insert(*i + 1, size * (1.0 - t));
                    return;
                }
            }
        }
        if let Some(n) = self.at_mut(path) {
            *n = FrameNode::Split {
                across,
                sizes: vec![t, 1.0 - t],
                parts: vec![FrameNode::Leaf(s), FrameNode::Leaf(s)],
            };
        }
    }

    /// Take the field at `path` out: its room goes to its neighbour, and
    /// a split left with one part becomes that part.
    pub fn remove(&mut self, path: &[usize]) {
        let Some((i, parent)) = path.split_last() else {
            return;
        };
        let Some(FrameNode::Split { sizes, parts, .. }) = self.at_mut(parent) else {
            return;
        };
        if *i >= parts.len() || parts.len() < 2 {
            return;
        }
        sizes.resize(parts.len(), 1.0);
        let j = if *i > 0 { *i - 1 } else { 1 };
        sizes[j] += sizes[*i];
        parts.remove(*i);
        sizes.remove(*i);
        if parts.len() == 1 {
            let only = parts.remove(0);
            if let Some(n) = self.at_mut(parent) {
                *n = only;
            }
        }
    }

    /// Give the field at `path` the share `f` (0 to 1) of its parent's
    /// room; the others keep their proportions.
    pub fn set_share(&mut self, path: &[usize], f: f64) {
        let Some((i, parent)) = path.split_last() else {
            return;
        };
        let Some(FrameNode::Split { sizes, parts, .. }) = self.at_mut(parent) else {
            return;
        };
        let n = parts.len();
        if *i >= n || n < 2 {
            return;
        }
        sizes.resize(n, 1.0);
        let f = f.clamp(0.05, 1.0 - 0.05 * (n - 1) as f64);
        let sum: f64 = sizes.iter().sum();
        let rest = sum - sizes[*i];
        if sum <= 1e-9 || rest <= 1e-9 {
            return;
        }
        let k = (1.0 - f) / rest;
        for (j, s) in sizes.iter_mut().enumerate() {
            *s = if j == *i { f } else { *s * k };
        }
    }

    /// Every field's leaf, left to right and top to bottom as written.
    pub fn leaves(&self) -> Vec<Sash> {
        match self {
            FrameNode::Leaf(s) => vec![*s],
            FrameNode::Split { parts, .. } => parts.iter().flat_map(|p| p.leaves()).collect(),
        }
    }

    /// As text: a field is its leaf's code (`OS-L`, with `*` after it
    /// for a solid panel), a split `v(1:OS-L,1:FIX)` side by side or
    /// `h(0.3:S,0.7:OS-L)` one above the other, each part after its
    /// share.
    pub fn code(&self) -> String {
        match self {
            FrameNode::Leaf(s) => format!("{}{}", s.op.code(), if s.panel { "*" } else { "" }),
            FrameNode::Split {
                across,
                sizes,
                parts,
            } => {
                let inner: Vec<String> = parts
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        let s = sizes.get(i).copied().unwrap_or(1.0);
                        format!("{}:{}", (s * 1000.0).round() / 1000.0, p.code())
                    })
                    .collect();
                format!("{}({})", if *across { "v" } else { "h" }, inner.join(","))
            }
        }
    }

    /// Read [`FrameNode::code`] back.
    pub fn parse(s: &str) -> Result<FrameNode, String> {
        let s = s.trim();
        let split = |across: bool| -> Result<FrameNode, String> {
            let inner = s[1..]
                .strip_prefix('(')
                .and_then(|x| x.strip_suffix(')'))
                .ok_or_else(|| format!("`{s}` needs its parts in brackets"))?;
            // commas at depth 0
            let (mut depth, mut from, mut items) = (0, 0, vec![]);
            for (i, c) in inner.char_indices() {
                match c {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    ',' if depth == 0 => {
                        items.push(&inner[from..i]);
                        from = i + 1;
                    }
                    _ => {}
                }
            }
            items.push(&inner[from..]);
            let (mut sizes, mut parts) = (vec![], vec![]);
            for it in items {
                let (size, node) = it
                    .split_once(':')
                    .ok_or_else(|| format!("`{it}` needs a share, as in `1:FIX`"))?;
                let size: f64 = size
                    .trim()
                    .parse()
                    .map_err(|_| format!("`{size}` is not a number"))?;
                if !(size.is_finite() && size > 0.0) {
                    return Err(format!("a share must be more than nothing, got {size}"));
                }
                sizes.push(size);
                parts.push(FrameNode::parse(node)?);
            }
            if parts.len() < 2 {
                return Err(format!("`{s}` needs at least two parts"));
            }
            Ok(FrameNode::Split {
                across,
                sizes,
                parts,
            })
        };
        if s.starts_with("v(") {
            return split(true);
        }
        if s.starts_with("h(") {
            return split(false);
        }
        let (code, panel) = match s.strip_suffix('*') {
            Some(c) => (c, true),
            None => (s, false),
        };
        let op = LeafOp::of_code(code).ok_or_else(|| {
            format!(
                "`{code}` is not a leaf: one of {}",
                LeafOp::ALL.map(|o| o.code()).join(", ")
            )
        })?;
        Ok(FrameNode::Leaf(Sash { op, panel }))
    }

    /// Sound: every split has two parts or more, as many shares as
    /// parts, each more than nothing.
    pub fn validate(&self) -> Result<(), String> {
        match self {
            FrameNode::Leaf(_) => Ok(()),
            FrameNode::Split { sizes, parts, .. } => {
                if parts.len() < 2 || sizes.len() != parts.len() {
                    return Err(
                        "a division's split needs two parts or more, each with its share".into(),
                    );
                }
                if sizes.iter().any(|s| !(s.is_finite() && *s > 0.0)) {
                    return Err("a division's shares must be more than nothing".into());
                }
                parts.iter().try_for_each(|p| p.validate())
            }
        }
    }
}

/// The lines of a field's opening symbol, seen from outside: a
/// triangle's point is at the hinges (STN 01 3420), an arrow shows the
/// way a sliding leaf goes.
pub fn sash_symbol(f: &Field, inset: f64) -> Vec<[f64; 4]> {
    let (x0, x1, y0, y1) = (f.x0 + inset, f.x1 - inset, f.y0 + inset, f.y1 - inset);
    let (xm, ym) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
    let mut l = vec![];
    let op = f.sash.op;
    if let Some(left) = op.hinged_left() {
        let (hx, ox) = if left { (x0, x1) } else { (x1, x0) };
        l.push([ox, y0, hx, ym]);
        l.push([ox, y1, hx, ym]);
    }
    if op.tilts() {
        l.push([x0, y1, xm, y0]);
        l.push([x1, y1, xm, y0]);
    }
    if op == LeafOp::TopHung {
        l.push([x0, y0, xm, y1]);
        l.push([x1, y0, xm, y1]);
    }
    if let Some(left) = op.slides_left() {
        let d = if left { -1.0 } else { 1.0 };
        let (a, b) = (xm - d * (x1 - x0) * 0.25, xm + d * (x1 - x0) * 0.25);
        l.push([a, ym, b, ym]);
        l.push([b, ym, b - d * 0.06, ym + 0.04]);
        l.push([b, ym, b - d * 0.06, ym - 0.04]);
    }
    l
}

/// A window's or door's division: the one set, else its panes as fixed
/// fields (a door: one leaf hung as the door is).
pub fn division_of(o: &Opening) -> FrameNode {
    if let Some(d) = &o.division {
        return d.clone();
    }
    if o.kind == OpeningKind::Door {
        return FrameNode::leaf(LeafOp::DoorLeft);
    }
    let cols = |n: u32, of: &dyn Fn() -> FrameNode| {
        if n <= 1 {
            of()
        } else {
            FrameNode::Split {
                across: true,
                sizes: vec![1.0; n as usize],
                parts: (0..n).map(|_| of()).collect(),
            }
        }
    };
    let up = o.frame.panes_up.clamp(1, 12);
    let column = || {
        if up <= 1 {
            FrameNode::leaf(LeafOp::Fixed)
        } else {
            FrameNode::Split {
                across: false,
                sizes: vec![1.0; up as usize],
                parts: (0..up).map(|_| FrameNode::leaf(LeafOp::Fixed)).collect(),
            }
        }
    };
    cols(o.frame.panes_across.clamp(1, 12), &column)
}

/// How a window opens, field by field, as schedules write it:
/// "OS-L + FIX".
pub fn operation(o: &Opening) -> String {
    division_of(o)
        .leaves()
        .iter()
        .map(|s| s.op.code())
        .collect::<Vec<_>>()
        .join(" + ")
}

/// What makes a window or door the kind it is: all of it but where it
/// stands and which way it is turned. Those with the same key are alike:
/// one row of a window or door list, and one mark where the project's
/// rules say so.
pub fn kind_key(o: &Opening) -> String {
    let mut plain = o.clone();
    plain.host = crate::ElementId::NIL;
    plain.offset = 0.0;
    plain.flip_hand = false;
    plain.flip_facing = false;
    plain.setback = None;
    format!("{plain:?}")
}

/// What a window or door list says of a window or door in words.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JoineryTexts {
    /// How it is made: "Plastové okenné profily, šesťkomorový systém".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub execution: String,
    /// Its finish: "Drevené dvere, textúra dub".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub finish: String,
    /// A note: "Zasklenie izolačným trojsklom".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

impl JoineryTexts {
    pub fn is_empty(&self) -> bool {
        self.execution.is_empty() && self.finish.is_empty() && self.note.is_empty()
    }
}

/// A door's frame: it sets the structural opening its leaf needs, how
/// it is drawn and what the schedule says.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoorFrame {
    /// A pressed steel frame built into the masonry.
    #[default]
    Steel,
    /// An aluminium frame.
    Aluminium,
    /// A solid timber frame.
    Timber,
    /// A timber lining round the whole reveal, with architraves on both
    /// faces, fitted into the finished opening.
    Lining,
}

impl DoorFrame {
    pub const ALL: [DoorFrame; 4] = [
        DoorFrame::Steel,
        DoorFrame::Aluminium,
        DoorFrame::Timber,
        DoorFrame::Lining,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DoorFrame::Steel => "Steel frame",
            DoorFrame::Aluminium => "Aluminium frame",
            DoorFrame::Timber => "Timber frame",
            DoorFrame::Lining => "Timber lining with architraves",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            DoorFrame::Steel => "steel",
            DoorFrame::Aluminium => "aluminium",
            DoorFrame::Timber => "timber",
            DoorFrame::Lining => "lining",
        }
    }

    pub fn of_code(s: &str) -> Option<DoorFrame> {
        DoorFrame::ALL.into_iter().find(|f| f.code() == s)
    }

    /// What the structural opening is wider (both jambs together) and
    /// higher than the leaf's nominal size, m: a steel frame 800 → 950 ×
    /// 2050, a lining in the finished opening 800 → 900 × 2020, solid
    /// timber and aluminium frames 80 to 100 mm a side.
    pub fn allowance(self) -> (f64, f64) {
        match self {
            DoorFrame::Steel => (0.15, 0.08),
            DoorFrame::Lining => (0.10, 0.05),
            DoorFrame::Timber => (0.20, 0.10),
            DoorFrame::Aluminium => (0.16, 0.08),
        }
    }

    /// The structural opening for a leaf `w` × `h`.
    pub fn opening_for(self, w: f64, h: f64) -> (f64, f64) {
        let (aw, ah) = self.allowance();
        (mm(w + aw), mm(h + ah))
    }

    /// The colour it is drawn in where the door has no material.
    pub fn color(self) -> [f32; 3] {
        match self {
            DoorFrame::Steel => [0.36, 0.39, 0.45],
            DoorFrame::Aluminium => [0.73, 0.76, 0.80],
            DoorFrame::Timber => [0.60, 0.44, 0.28],
            DoorFrame::Lining => [0.78, 0.65, 0.48],
        }
    }
}

fn mm(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// What a door's leaf is made of.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DoorMaterial {
    #[default]
    Timber,
    Steel,
    Aluminium,
}

impl DoorMaterial {
    pub const ALL: [DoorMaterial; 3] = [
        DoorMaterial::Timber,
        DoorMaterial::Steel,
        DoorMaterial::Aluminium,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DoorMaterial::Timber => "Timber",
            DoorMaterial::Steel => "Steel",
            DoorMaterial::Aluminium => "Aluminium",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            DoorMaterial::Timber => "timber",
            DoorMaterial::Steel => "steel",
            DoorMaterial::Aluminium => "aluminium",
        }
    }

    pub fn of_code(s: &str) -> Option<DoorMaterial> {
        DoorMaterial::ALL.into_iter().find(|f| f.code() == s)
    }

    pub fn color(self) -> [f32; 3] {
        match self {
            DoorMaterial::Timber => [0.62, 0.45, 0.30],
            DoorMaterial::Steel => [0.50, 0.53, 0.58],
            DoorMaterial::Aluminium => [0.70, 0.73, 0.77],
        }
    }
}

/// The usual fire resistance classes of doors (STN EN 13501-2, STN 92
/// 0201).
pub const FIRE_RATINGS: [&str; 8] = [
    "EW 15/D3", "EW 30/D3", "EI 30/D3", "EW 45/D1", "EI 45/D1", "EW 60/D1", "EI 60/D1", "EI 90/D1",
];

/// A fire door's rating.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FireDoor {
    /// One of [`FIRE_RATINGS`], or as written.
    pub rating: String,
    /// With a closer (C).
    #[serde(default = "yes")]
    pub self_closing: bool,
    /// Smoke tight (S200).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub smoke_tight: bool,
}

fn yes() -> bool {
    true
}

impl Default for FireDoor {
    fn default() -> Self {
        FireDoor {
            rating: "EW 30/D3".into(),
            self_closing: true,
            smoke_tight: false,
        }
    }
}

impl FireDoor {
    /// As drawings write it: "EW 30/D3-C S200".
    pub fn label(&self) -> String {
        format!(
            "{}{}{}",
            self.rating,
            if self.self_closing { "-C" } else { "" },
            if self.smoke_tight { " S200" } else { "" }
        )
    }
}

/// The usual nominal widths of door leaves, m, and their height.
pub const NOMINAL_WIDTHS: [f64; 6] = [0.6, 0.7, 0.8, 0.9, 1.0, 1.1];
pub const NOMINAL_HEIGHT: f64 = 1.97;

/// A door as joinery: the leaf's nominal size (what its mark and the
/// schedule give: 800 × 1970), its frame and what the leaf is. The
/// opening in the wall is larger by the frame's allowance (900 × 2020
/// for a lining).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DoorSpec {
    pub frame: DoorFrame,
    pub leaf: DoorMaterial,
    /// The leaf's nominal width and height, without the frame.
    pub nom_width: f64,
    pub nom_height: f64,
    /// A fire door's rating.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fire: Option<FireDoor>,
}

impl DoorSpec {
    /// The door an opening `w` × `h` holds with a frame `frame`: its
    /// leaf is what the frame leaves.
    pub fn fitting(frame: DoorFrame, w: f64, h: f64) -> DoorSpec {
        let (aw, ah) = frame.allowance();
        DoorSpec {
            frame,
            leaf: DoorMaterial::Timber,
            nom_width: mm((w - aw).max(0.3)),
            nom_height: mm((h - ah).max(1.0)),
            fire: None,
        }
    }

    /// The structural opening it needs.
    pub fn opening(&self) -> (f64, f64) {
        self.frame.opening_for(self.nom_width, self.nom_height)
    }

    /// The frame's width at each jamb and its head's height in an
    /// opening `w` × `h`.
    pub fn members(&self, w: f64, h: f64) -> (f64, f64) {
        (
            ((w - self.nom_width) / 2.0).clamp(0.01, (w / 3.0).max(0.01)),
            (h - self.nom_height).clamp(0.01, (h / 3.0).max(0.01)),
        )
    }

    /// The leaf's thickness: a fire door's is thicker.
    pub fn leaf_thickness(&self) -> f64 {
        if self.fire.is_some() {
            0.055
        } else {
            0.04
        }
    }

    /// The leaf as plans write it at the door: "800/1970", a fire
    /// door's rating after it.
    pub fn mark(&self) -> String {
        let n = |v: f64| format!("{:.0}", v * 1000.0);
        let mut s = format!("{}/{}", n(self.nom_width), n(self.nom_height));
        if let Some(f) = &self.fire {
            s.push(' ');
            s.push_str(&f.label());
        }
        s
    }

    pub fn validate(&self) -> Result<(), String> {
        if !(self.nom_width.is_finite() && self.nom_width > 0.0)
            || !(self.nom_height.is_finite() && self.nom_height > 0.0)
        {
            return Err("a door leaf's nominal size must be more than nothing".into());
        }
        Ok(())
    }
}

/// Give `o` the door `spec` and the structural opening it needs, its
/// middle staying where it is along the wall.
pub fn apply_door(o: &mut Opening, spec: DoorSpec) {
    let (w, h) = spec.opening();
    o.offset = (o.offset + (o.width - w) / 2.0).max(0.0);
    o.width = w;
    o.height = h;
    o.door = Some(spec);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_leaf_of_800_needs_900_by_2020_with_a_lining() {
        assert_eq!(DoorFrame::Lining.opening_for(0.8, 1.97), (0.9, 2.02));
        assert_eq!(DoorFrame::Steel.opening_for(0.8, 1.97), (0.95, 2.05));
        let d = DoorSpec::fitting(DoorFrame::Lining, 0.9, 2.02);
        assert_eq!((d.nom_width, d.nom_height), (0.8, 1.97));
        assert_eq!(d.mark(), "800/1970");
        let (jamb, head) = d.members(0.9, 2.02);
        assert!((jamb - 0.05).abs() < 1e-9 && (head - 0.05).abs() < 1e-9);
        let fire = DoorSpec {
            fire: Some(FireDoor {
                smoke_tight: true,
                ..FireDoor::default()
            }),
            ..d
        };
        assert_eq!(fire.mark(), "800/1970 EW 30/D3-C S200");
        assert!(fire.leaf_thickness() > 0.05);
    }

    #[test]
    fn a_division_lays_out_splits_and_reads_back() {
        let mut n = FrameNode::leaf(LeafOp::TiltTurnLeft);
        n.split(&[], true, 0.5);
        *n.at_mut(&[1]).unwrap() = FrameNode::leaf(LeafOp::Fixed);
        // a fanlight over the first field
        n.split(&[0], false, 0.3);
        assert_eq!(n.code(), "v(0.5:h(0.3:OS-L,0.7:OS-L),0.5:FIX)");
        assert_eq!(FrameNode::parse(&n.code()).unwrap(), n);
        let (fields, bars) = n.layout(1.5, 1.5, 0.06, 0.06, true);
        assert_eq!((fields.len(), bars.len()), (3, 2));
        // the fanlight is the top one, 30 % of the height inside
        let top = &fields[0];
        assert_eq!(top.path, vec![0, 0]);
        assert!((top.y1 - 1.44).abs() < 1e-9);
        assert!((top.y1 - top.y0 - (1.38 - 0.06) * 0.3).abs() < 1e-9);
        // the fixed field on the right
        assert!((fields[2].x1 - 1.44).abs() < 1e-9);
        // splitting beside a split the same way doesn't nest
        n.split(&[1], true, 0.5);
        assert_eq!(n.code(), "v(0.5:h(0.3:OS-L,0.7:OS-L),0.25:FIX,0.25:FIX)");
        n.set_share(&[0], 0.4);
        let FrameNode::Split { sizes, .. } = &n else {
            panic!()
        };
        assert!((sizes[0] - 0.4).abs() < 1e-9 && (sizes[1] - 0.3).abs() < 1e-9);
        n.remove(&[2]);
        n.remove(&[1]);
        assert_eq!(n.code(), "h(0.3:OS-L,0.7:OS-L)");
        assert!(FrameNode::parse("v(1:XX,1:FIX)").is_err());
        assert!(FrameNode::parse("v(1:FIX)").is_err());
        assert!(n.validate().is_ok());
        // a door's frame has no bottom member
        let (f, _) = FrameNode::leaf(LeafOp::DoorLeft).layout(0.9, 2.02, 0.05, 0.05, false);
        assert_eq!(f[0].y0, 0.0);
    }

    #[test]
    fn the_symbols_point_is_at_the_hinges() {
        let (f, _) = FrameNode::leaf(LeafOp::TiltTurnLeft).layout(1.0, 1.0, 0.0, 0.0, true);
        let s = sash_symbol(&f[0], 0.0);
        assert_eq!(s.len(), 4);
        // hinged on the left: both lines of the side triangle end there
        assert!(s[..2].iter().all(|l| l[2] == 0.0 && l[3] == 0.5));
        // tilting: the point is at the bottom's middle
        assert!(s[2..].iter().all(|l| l[2] == 0.5 && l[3] == 0.0));
        let (f, _) = FrameNode::leaf(LeafOp::Fixed).layout(1.0, 1.0, 0.0, 0.0, true);
        assert!(sash_symbol(&f[0], 0.0).is_empty());
    }
}
