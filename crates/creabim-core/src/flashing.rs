//! Sheet-metal work of the roof (klampiarske výrobky, George 2026-11-21):
//! eaves gutters, half-round or square, hung along the roof's eaves at
//! an exact offset in plan and in height, and downpipes, round or
//! square, each placed on a gutter and slid along it. A gutter falls to
//! its downpipes: each run between a downpipe and the next high point
//! (midway to the next downpipe, or the gutter's end) falls towards the
//! downpipe.
//!
//! What drains to each downpipe is counted from the roof's faces, and
//! the flow from it as Q = r · A · C (tzb-info, Návrh střešního žlabu,
//! help: r = 0,03 l/s·m² for roofs, C = 1,0 for roofs other than ones
//! with a permeable top layer over 100 mm). Whether the gutter and the
//! downpipe carry it is checked once the STN EN 12056-3 text is in.

use crate::{ElementId, ElementKind, Model};
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};

/// A piece of the roof's sheet-metal work.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Flashing {
    /// The level of the roof it belongs to.
    pub level: ElementId,
    pub item: FlashingItem,
    /// What it is made of, as the table of sheet-metal items says it:
    /// "z pozinkovaného plechu".
    pub material: String,
    /// The sheet's thickness, mm.
    pub thickness_mm: f64,
    /// Its finish: "opatrený ochranným náterom proti narušeniu zinku".
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub finish: String,
    /// Developed width (rozvinutá šírka, r. š.) typed by hand, mm; used
    /// only when [`Flashing::developed_set`], else it is counted from the
    /// profile ([`Flashing::developed`]).
    pub developed_mm: f64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub developed_set: bool,
    /// The sheet past the profile itself, mm: a gutter's front bead and
    /// back edge, a downpipe's seam. None takes the usual
    /// ([`Flashing::default_edges`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edges_mm: Option<f64>,
    /// Its mark in the table typed by hand ("K-3"); none numbers it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<String>,
    /// A note of its own, added to the table's note column.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub note: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FlashingItem {
    Gutter(Gutter),
    Downpipe(Downpipe),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GutterProfile {
    HalfRound,
    Square,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownpipeProfile {
    Round,
    Square,
}

/// An eaves gutter.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Gutter {
    /// The roof it is hung from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub roof: Option<ElementId>,
    /// The roof's eave line it follows, point by point, in plan.
    pub trace: Vec<DVec2>,
    pub profile: GutterProfile,
    /// Its width (a half-round's diameter), mm.
    pub size_mm: f64,
    /// From the eave line out to the gutter's back edge, in plan, m.
    #[serde(default)]
    pub plan_offset: f64,
    /// From the roof's top at the eave down to the gutter's top edge at
    /// its high point, m.
    #[serde(default)]
    pub height_offset: f64,
    /// The fall towards the downpipes, mm per m.
    pub fall: f64,
    /// Hooks one every so far, m.
    pub hook_spacing: f64,
    /// Rain intensity r, l/(s·m²).
    #[serde(default = "default_rain")]
    pub rain: f64,
    /// Runoff coefficient C.
    #[serde(default = "default_runoff")]
    pub runoff: f64,
    /// Its outlets have a sieve or a leaf trap (halves what it carries).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sieve: bool,
}

fn default_rain() -> f64 {
    0.03
}

fn default_runoff() -> f64 {
    1.0
}

/// A downpipe from a gutter.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Downpipe {
    pub gutter: ElementId,
    /// Where on the gutter, m along its trace from the first point.
    pub at: f64,
    pub profile: DownpipeProfile,
    /// Its diameter, or a square one's side, mm.
    pub size_mm: f64,
    /// The height its foot ends at, absolute (±0,000 = 0), m.
    #[serde(default)]
    pub bottom: f64,
    /// Elbows in it (under the outlet to the wall and more).
    pub elbows: u32,
    /// The elbows' angle, degrees.
    pub elbow_angle: f64,
    /// Clamps one every so far, m.
    pub clamp_spacing: f64,
    /// Turned under the outlet by its elbows back to the facade, to run
    /// down the outer face of the wall (its insulation and render).
    #[serde(default = "yes")]
    pub to_wall: bool,
    /// The gap between the pipe and the facade, m.
    #[serde(default = "default_facade_gap")]
    pub facade_gap: f64,
}

fn yes() -> bool {
    true
}

fn default_facade_gap() -> f64 {
    0.03
}

impl Flashing {
    /// A half-round gutter along `trace` (as George's table has it:
    /// galvanised 0,6 mm, painted, Ø 140, r. š. 333).
    pub fn gutter(level: ElementId, roof: Option<ElementId>, trace: Vec<DVec2>) -> Self {
        Self {
            level,
            item: FlashingItem::Gutter(Gutter {
                roof,
                trace,
                profile: GutterProfile::HalfRound,
                size_mm: 140.0,
                plan_offset: 0.0,
                height_offset: 0.0,
                fall: 3.0,
                hook_spacing: 0.6,
                rain: default_rain(),
                runoff: default_runoff(),
                sieve: false,
            }),
            material: GALVANISED.into(),
            thickness_mm: 0.6,
            finish: PAINTED.into(),
            developed_mm: 333.0,
            developed_set: false,
            edges_mm: None,
            mark: None,
            note: String::new(),
        }
    }

    /// A round downpipe Ø 100 on `gutter`, `at` along it.
    pub fn downpipe(level: ElementId, gutter: ElementId, at: f64) -> Self {
        Self {
            level,
            item: FlashingItem::Downpipe(Downpipe {
                gutter,
                at,
                profile: DownpipeProfile::Round,
                size_mm: 100.0,
                bottom: 0.0,
                elbows: 3,
                elbow_angle: 67.0,
                clamp_spacing: 2.0,
                to_wall: true,
                facade_gap: default_facade_gap(),
            }),
            material: GALVANISED.into(),
            thickness_mm: 0.6,
            finish: PAINTED.into(),
            developed_mm: 333.0,
            developed_set: false,
            edges_mm: None,
            mark: None,
            note: String::new(),
        }
    }

    /// The sheet past the profile, mm, as set or by default: what makes
    /// George's sample come out (half-round Ø 140 r. š. 333, round
    /// downpipe Ø 100 r. š. 333).
    pub fn edges(&self) -> f64 {
        self.edges_mm.unwrap_or_else(|| self.default_edges())
    }

    pub fn default_edges(&self) -> f64 {
        match &self.item {
            FlashingItem::Gutter(_) => 333.0 - std::f64::consts::PI * 70.0,
            FlashingItem::Downpipe(_) => 333.0 - std::f64::consts::PI * 100.0,
        }
    }

    /// The developed width, mm: typed by hand, or the profile's length
    /// (a half-round's half circle, a square gutter's three sides, a
    /// downpipe's whole round or four sides) and its edges, to whole mm.
    pub fn developed(&self) -> f64 {
        if self.developed_set {
            return self.developed_mm;
        }
        let pi = std::f64::consts::PI;
        let profile = match &self.item {
            FlashingItem::Gutter(g) => match g.profile {
                GutterProfile::HalfRound => pi * g.size_mm / 2.0,
                GutterProfile::Square => 3.0 * g.size_mm,
            },
            FlashingItem::Downpipe(d) => match d.profile {
                DownpipeProfile::Round => pi * d.size_mm,
                DownpipeProfile::Square => 4.0 * d.size_mm,
            },
        };
        (profile + self.edges()).round()
    }

    pub fn validate(&self) -> Result<(), String> {
        let pos = |v: f64, what: &str| {
            if v.is_finite() && v > 0.0 {
                Ok(())
            } else {
                Err(format!("{what} must be positive"))
            }
        };
        pos(self.thickness_mm, "sheet thickness")?;
        match &self.item {
            FlashingItem::Gutter(g) => {
                pos(g.size_mm, "gutter size")?;
                pos(g.hook_spacing, "hook spacing")?;
                if g.trace.len() < 2 || g.trace.iter().any(|p| !p.is_finite()) {
                    return Err("a gutter needs two finite points".into());
                }
                if !(g.fall.is_finite() && g.fall >= 0.0) {
                    return Err("gutter fall must not be negative".into());
                }
                Ok(())
            }
            FlashingItem::Downpipe(d) => {
                pos(d.size_mm, "downpipe size")?;
                pos(d.clamp_spacing, "clamp spacing")?;
                if !d.at.is_finite() || !d.bottom.is_finite() {
                    return Err("downpipe place must be finite".into());
                }
                Ok(())
            }
        }
    }

    /// The words of the table's Popis column.
    pub fn description(&self) -> String {
        let t = crate::flashing::num(self.thickness_mm);
        let mut s = match &self.item {
            FlashingItem::Gutter(g) => format!(
                "Odkvapový žľab {} tvaru",
                match g.profile {
                    GutterProfile::HalfRound => "polkruhového",
                    GutterProfile::Square => "štvorcového",
                }
            ),
            FlashingItem::Downpipe(d) => format!(
                "Odpadová rúra {} prierezu",
                match d.profile {
                    DownpipeProfile::Round => "kruhového",
                    DownpipeProfile::Square => "štvorcového",
                }
            ),
        };
        if !self.material.is_empty() {
            s += &format!(" {}", self.material);
        }
        s += &format!(" hr. {t} mm");
        if !self.finish.is_empty() {
            s += &format!(" {}", self.finish);
        }
        s += &match &self.item {
            FlashingItem::Gutter(g) => match g.profile {
                GutterProfile::HalfRound => format!(" fi {} mm", num(g.size_mm)),
                GutterProfile::Square => format!(" {0} x {0} mm", num(g.size_mm)),
            },
            FlashingItem::Downpipe(d) => match d.profile {
                DownpipeProfile::Round => format!(" fi {} mm", num(d.size_mm)),
                DownpipeProfile::Square => format!(" {0} x {0} mm", num(d.size_mm)),
            },
        };
        s
    }
}

pub const GALVANISED: &str = "z pozinkovaného plechu";
pub const PAINTED: &str = "opatrený ochranným náterom proti narušeniu zinku";

/// What the sheet can be made of, as the table writes it.
pub const MATERIALS: [&str; 6] = [
    "z pozinkovaného plechu",
    "z titanzinkového plechu",
    "z hliníkového plechu",
    "z medeného plechu",
    "z poplastovaného plechu",
    "z nerezového plechu",
];

/// A number the Slovak way: 0,6 / 140.
pub fn num(v: f64) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.replace('.', ",")
}

impl Gutter {
    /// The trace's stations: how far along it each point is, m.
    pub fn stations(&self) -> Vec<f64> {
        let mut out = vec![0.0];
        for w in self.trace.windows(2) {
            out.push(out.last().unwrap() + w[0].distance(w[1]));
        }
        out
    }

    pub fn length(&self) -> f64 {
        *self.stations().last().unwrap_or(&0.0)
    }

    /// The width across, m.
    pub fn width(&self) -> f64 {
        self.size_mm / 1000.0
    }

    /// How deep it is, m: a half-round's radius, a square's side.
    pub fn depth(&self) -> f64 {
        match self.profile {
            GutterProfile::HalfRound => self.width() / 2.0,
            GutterProfile::Square => self.width(),
        }
    }

    /// The side of each trace segment the gutter hangs on, as a unit
    /// normal: away from the roof's outline (or to the right of the
    /// trace when there is no roof).
    pub fn outward(&self, model: &Model) -> Vec<DVec2> {
        let eaves = self.roof_of(model).map(|r| r.eave_outline());
        self.trace
            .windows(2)
            .map(|w| {
                let d = (w[1] - w[0]).normalize_or_zero();
                let right = -d.perp();
                let mid = (w[0] + w[1]) / 2.0;
                match &eaves {
                    Some(o) if crate::polygon::contains(o, mid + right * 0.01) => -right,
                    _ => right,
                }
            })
            .collect()
    }

    pub fn roof_of<'a>(&self, model: &'a Model) -> Option<&'a crate::Roof> {
        match &model.get(self.roof?)?.kind {
            ElementKind::Roof(r) => Some(r),
            _ => None,
        }
    }

    /// The line its axis runs along in plan: the trace moved out by the
    /// plan offset and half the width, mitred at the corners.
    pub fn axis_plan(&self, model: &Model) -> Vec<DVec2> {
        let by = self.plan_offset + self.width() / 2.0;
        offset_line(&self.trace, &self.outward(model), by)
    }

    /// The outline of the gutter in plan: its back and front edges.
    pub fn edges_plan(&self, model: &Model) -> (Vec<DVec2>, Vec<DVec2>) {
        let n = self.outward(model);
        (
            offset_line(&self.trace, &n, self.plan_offset),
            offset_line(&self.trace, &n, self.plan_offset + self.width()),
        )
    }

    /// Absolute height of its top edge at its high points: the roof's
    /// top at the lowest point of the eave it follows, less the height
    /// offset.
    pub fn top(&self, model: &Model) -> f64 {
        let eave = match self.roof_of(model) {
            Some(r) => {
                let base = r.base(model);
                self.trace
                    .iter()
                    .map(|p| base + r.underside_at(*p) + r.vertical_thickness())
                    .fold(f64::INFINITY, f64::min)
            }
            None => crate::members::elevation(model, ElementId::NIL),
        };
        let eave = if eave.is_finite() { eave } else { 0.0 };
        eave - self.height_offset
    }

    /// The high points of the runs between `outlets` (sorted stations):
    /// the ends, and midway between neighbours.
    pub fn high_points(&self, outlets: &[f64]) -> Vec<f64> {
        let len = self.length();
        if outlets.is_empty() {
            return vec![];
        }
        let mut out = vec![];
        if outlets[0] > 1e-6 {
            out.push(0.0);
        }
        for w in outlets.windows(2) {
            out.push((w[0] + w[1]) / 2.0);
        }
        if *outlets.last().unwrap() < len - 1e-6 {
            out.push(len);
        }
        out
    }

    /// How far below its top edge at the high points the gutter's top
    /// edge is at station `s`, m, as it falls to the nearest outlet.
    pub fn drop_at(&self, outlets: &[f64], s: f64) -> f64 {
        let len = self.length();
        let Some(&near) = outlets
            .iter()
            .min_by(|a, b| (*a - s).abs().total_cmp(&(*b - s).abs()))
        else {
            return 0.0;
        };
        // the high point of the run s is in
        let high = if s < near {
            outlets
                .iter()
                .rev()
                .find(|o| **o < near - 1e-9)
                .map_or(0.0, |o| (o + near) / 2.0)
        } else {
            outlets
                .iter()
                .find(|o| **o > near + 1e-9)
                .map_or(len, |o| (o + near) / 2.0)
        };
        let run = (high - near).abs();
        let to_high = (s - high).abs().min(run);
        self.fall / 1000.0 * to_high
    }

    /// The point of the axis in plan at station `s` of the trace.
    pub fn axis_at(&self, axis: &[DVec2], s: f64) -> DVec2 {
        let st = self.stations();
        for i in 0..self.trace.len().saturating_sub(1) {
            if s <= st[i + 1] + 1e-9 || i + 2 == self.trace.len() {
                let len = (st[i + 1] - st[i]).max(1e-12);
                let f = ((s - st[i]) / len).clamp(0.0, 1.0);
                return axis[i].lerp(axis[i + 1], f);
            }
        }
        axis.first().copied().unwrap_or(DVec2::ZERO)
    }

    /// The top-edge line of the axis in 3D (absolute heights), with a
    /// point at every corner, outlet and high point, falling to the
    /// outlets.
    pub fn axis_3d(&self, model: &Model, outlets: &[f64]) -> Vec<DVec3> {
        let axis = self.axis_plan(model);
        let top = self.top(model);
        let mut st = self.stations();
        st.extend(outlets.iter().copied());
        st.extend(self.high_points(outlets));
        st.retain(|s| *s >= -1e-9 && *s <= self.length() + 1e-9);
        st.sort_by(f64::total_cmp);
        st.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
        // a corner's station must land on the corner point
        st.iter()
            .map(|s| {
                self.axis_at(&axis, *s)
                    .extend(top - self.drop_at(outlets, *s))
            })
            .collect()
    }

    /// Corners along it (turns over 10°).
    pub fn corners(&self) -> usize {
        self.trace
            .windows(3)
            .filter(|w| {
                let (a, b) = (
                    (w[1] - w[0]).normalize_or_zero(),
                    (w[2] - w[1]).normalize_or_zero(),
                );
                a.dot(b) < 10f64.to_radians().cos()
            })
            .count()
    }

    /// Station of the trace whose axis point is nearest `p`.
    pub fn station_on_axis(&self, model: &Model, p: DVec2) -> f64 {
        let axis = self.axis_plan(model);
        let st = self.stations();
        let mut best = (0.0, f64::INFINITY);
        for i in 0..axis.len().saturating_sub(1) {
            let (a, b) = (axis[i], axis[i + 1]);
            let d = b - a;
            let t = ((p - a).dot(d) / d.length_squared().max(1e-12)).clamp(0.0, 1.0);
            let dist = (a + d * t).distance(p);
            if dist < best.1 {
                best = (st[i] + t * (st[i + 1] - st[i]), dist);
            }
        }
        best.0
    }

    /// Station of the point of the trace nearest `p`, and how far `p` is
    /// from the trace.
    pub fn station_of(&self, p: DVec2) -> (f64, f64) {
        let st = self.stations();
        let mut best = (0.0, f64::INFINITY);
        for (i, w) in self.trace.windows(2).enumerate() {
            let d = w[1] - w[0];
            let len2 = d.length_squared().max(1e-12);
            let t = ((p - w[0]).dot(d) / len2).clamp(0.0, 1.0);
            let q = w[0] + d * t;
            let dist = q.distance(p);
            if dist < best.1 {
                best = (st[i] + t * d.length(), dist);
            }
        }
        best
    }
}

/// Polyline `line` moved by `by` along each segment's normal `n`, mitred.
pub fn offset_line(line: &[DVec2], n: &[DVec2], by: f64) -> Vec<DVec2> {
    let k = line.len();
    if k < 2 || n.len() + 1 != k {
        return line.to_vec();
    }
    (0..k)
        .map(|i| {
            let m = if i == 0 {
                n[0]
            } else if i == k - 1 {
                n[k - 2]
            } else {
                let s = (n[i - 1] + n[i]).normalize_or(n[i]);
                s / s.dot(n[i]).max(0.3)
            };
            line[i] + m * by
        })
        .collect()
}

/// The downpipes on gutter `gutter`, by their station, sorted.
pub fn downpipes_of(model: &Model, gutter: ElementId) -> Vec<(ElementId, f64)> {
    let mut out: Vec<(ElementId, f64)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Flashing(Flashing {
                item: FlashingItem::Downpipe(d),
                ..
            }) if d.gutter == gutter => Some((e.id, d.at)),
            _ => None,
        })
        .collect();
    out.sort_by(|a, b| a.1.total_cmp(&b.1));
    out
}

/// The gutter `id` is, if it is one.
pub fn gutter_of(model: &Model, id: ElementId) -> Option<&Gutter> {
    match &model.get(id)?.kind {
        ElementKind::Flashing(Flashing {
            item: FlashingItem::Gutter(g),
            ..
        }) => Some(g),
        _ => None,
    }
}

/// The outlets' stations of gutter `id`.
pub fn outlets(model: &Model, id: ElementId) -> Vec<f64> {
    let Some(g) = gutter_of(model, id) else {
        return vec![];
    };
    let len = g.length();
    downpipes_of(model, id)
        .into_iter()
        .map(|(_, s)| s.clamp(0.0, len))
        .collect()
}

/// Where a downpipe stands and the heights it runs between: its plan
/// centre, the gutter's bottom over it (its top), its foot.
pub fn downpipe_place(model: &Model, d: &Downpipe) -> Option<(DVec2, f64, f64)> {
    let g = gutter_of(model, d.gutter)?;
    let outs = outlets(model, d.gutter);
    let s = d.at.clamp(0.0, g.length());
    let axis = g.axis_plan(model);
    let at = g.axis_at(&axis, s);
    let top = g.top(model) - g.drop_at(&outs, s) - g.depth();
    Some((at, top, d.bottom.min(top)))
}

/// Its length, m: along its axis, the offset to the wall included.
pub fn downpipe_length(model: &Model, d: &Downpipe) -> f64 {
    let path = downpipe_path(model, d);
    path.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// The facade a downpipe under `at` runs down, looking from `at` along
/// `dir` (in plan): how far its outer face is, if a wall stands there
/// below `z`, within 3 m.
pub fn facade_distance(model: &Model, at: DVec2, dir: DVec2, z: f64) -> Option<f64> {
    let mut best: Option<f64> = None;
    for e in model.iter() {
        let ElementKind::Wall(w) = &e.kind else {
            continue;
        };
        if w.is_arc() || w.curtain.is_some() {
            continue;
        }
        let base = crate::members::elevation(model, w.level) + w.base_offset;
        if base >= z {
            continue;
        }
        let d = w.direction();
        let n = d.perp();
        let t = w.thickness / 2.0;
        for side in [t, -t] {
            // the face line, run on by the wall's thickness past its ends
            let a = w.start - d * w.thickness + n * side;
            let b = w.end + d * w.thickness + n * side;
            let e2 = b - a;
            let den = dir.perp_dot(e2);
            if den.abs() < 1e-9 {
                continue;
            }
            let s = (a - at).perp_dot(e2) / den;
            let u = (a - at).perp_dot(dir) / den;
            if s > 1e-6 && s < 3.0 && (0.0..=1.0).contains(&u) && best.is_none_or(|x| s < x) {
                best = Some(s);
            }
        }
    }
    best
}

/// The downpipe's axis, top down: from the gutter's outlet a short neck
/// down, then (turned to the wall) the offset by its elbows to the
/// facade, and down the facade to its foot.
pub fn downpipe_path(model: &Model, d: &Downpipe) -> Vec<DVec3> {
    let Some((at, top, foot)) = downpipe_place(model, d) else {
        return vec![];
    };
    let r = d.size_mm / 2000.0;
    let neck = (top - (d.size_mm / 1000.0)).max(foot);
    let mut path = vec![at.extend(top), at.extend(neck)];
    let inward = gutter_of(model, d.gutter).and_then(|g| {
        let s = d.at.clamp(0.0, g.length());
        let st = g.stations();
        let k = (0..st.len().saturating_sub(1))
            .find(|i| s <= st[i + 1] + 1e-9)
            .unwrap_or(0);
        g.outward(model).get(k).map(|n| -*n)
    });
    if let (true, Some(dir)) = (d.to_wall, inward) {
        if let Some(face) = facade_distance(model, at, dir, neck) {
            let run = face - d.facade_gap - r;
            let slope = d.elbow_angle.clamp(5.0, 85.0).to_radians();
            let drop = run / slope.tan();
            if run > 1e-3 && neck - drop > foot {
                let wall_top = (at + dir * run).extend(neck - drop);
                path.push(wall_top);
                path.push(wall_top.truncate().extend(foot));
                return path;
            }
        }
    }
    path.push(at.extend(foot));
    path
}

/// The plan area of roof that drains to each downpipe of gutter `id`, m²:
/// each roof face sloping down to a stretch of the gutter's trace sends
/// its water straight down its slope, so a face's part over the stretch
/// nearest a downpipe goes to it.
pub fn catchment(model: &Model, id: ElementId) -> Vec<(ElementId, f64)> {
    let Some(g) = gutter_of(model, id) else {
        return vec![];
    };
    let pipes = downpipes_of(model, id);
    if pipes.is_empty() {
        return vec![];
    }
    let Some(roof) = g.roof_of(model) else {
        return pipes.into_iter().map(|(p, _)| (p, 0.0)).collect();
    };
    let faces = roof.faces();
    let outward = g.outward(model);
    let st = g.stations();
    let len = g.length();
    // the stretch of trace each downpipe takes: to midway between it and
    // its neighbours
    let bounds: Vec<(f64, f64)> = (0..pipes.len())
        .map(|i| {
            let lo = if i == 0 {
                0.0
            } else {
                (pipes[i - 1].1 + pipes[i].1) / 2.0
            };
            let hi = if i + 1 == pipes.len() {
                len
            } else {
                (pipes[i].1 + pipes[i + 1].1) / 2.0
            };
            (lo, hi)
        })
        .collect();
    let mut area = vec![0.0; pipes.len()];
    for (k, w) in g.trace.windows(2).enumerate() {
        let d = (w[1] - w[0]).normalize_or_zero();
        let n = outward[k];
        for f in &faces {
            // a face sloping down towards this stretch, away from the roof
            let down = -f.g.normalize_or_zero();
            if f.g.length() < 1e-9 || down.dot(n) < 0.99 {
                continue;
            }
            // and ending on its line
            let on_line = f.outline.iter().any(|p| (*p - w[0]).dot(n).abs() < 0.05);
            if !on_line {
                continue;
            }
            for (i, (lo, hi)) in bounds.iter().enumerate() {
                let a = lo.max(st[k]);
                let b = hi.min(st[k + 1]);
                if b <= a {
                    continue;
                }
                // the part of the face whose foot is within a..b
                let o = d.dot(w[0]) - st[k];
                let mut part = crate::members::clip_half_plane(&f.outline, -d, -(a + o));
                part = crate::members::clip_half_plane(&part, d, b + o);
                area[i] += crate::polygon::signed_area2(&part).abs() / 2.0;
            }
        }
    }
    pipes.into_iter().map(|(p, _)| p).zip(area).collect()
}

/// The flow factor F_L for a run `l_w` times as long as the gutter's
/// design depth, by its fall: linear between the points of the tzb-info
/// gutter calculator (STN EN 12056-3), read in George's browser
/// 2026-11-21. None from 500 on, beyond the table.
pub fn length_factor(l_w: f64, fall_mm_m: f64) -> Option<f64> {
    if l_w >= 500.0 {
        return None;
    }
    const FLAT: [f64; 19] = [
        1.00, 0.97, 0.93, 0.90, 0.86, 0.83, 0.80, 0.78, 0.77, 0.75, 0.73, 0.72, 0.70, 0.68, 0.67,
        0.65, 0.63, 0.62, 0.60,
    ];
    // from 50 to 200 by 25, the same past it
    const FALLS: [(f64, [f64; 7]); 4] = [
        (4.0, [1.00, 1.02, 1.03, 1.05, 1.07, 1.08, 1.10]),
        (6.0, [1.00, 1.04, 1.08, 1.12, 1.17, 1.21, 1.25]),
        (8.0, [1.00, 1.07, 1.13, 1.20, 1.27, 1.33, 1.40]),
        (10.0, [1.00, 1.09, 1.18, 1.27, 1.37, 1.46, 1.55]),
    ];
    let row: &[f64] = match FALLS.iter().rev().find(|(f, _)| fall_mm_m >= f - 1e-9) {
        Some((_, r)) => r,
        None => &FLAT,
    };
    let x = ((l_w - 50.0) / 25.0).max(0.0);
    let i = (x.floor() as usize).min(row.len() - 1);
    let j = (i + 1).min(row.len() - 1);
    Some(row[i] + (row[j] - row[i]) * (x - i as f64).min(1.0))
}

/// F_d by W/T of a square or trapezoid gutter, from 0,05 by 0,05 (the
/// same calculator; F_s is 1 for its bottom as wide as its top).
const FD: [f64; 40] = [
    0.9075, 0.9175, 0.9267, 0.9356, 0.9438, 0.9516, 0.9588, 0.9656, 0.9717, 0.9774, 0.9826, 0.9872,
    0.9913, 0.9949, 0.9980, 1.0005, 1.0025, 1.0041, 1.0050, 1.0055, 1.0054, 1.0049, 1.0038, 1.0021,
    1.0000, 0.9973, 0.9941, 0.9904, 0.9862, 0.9814, 0.9762, 0.9704, 0.9641, 0.9572, 0.9499, 0.9420,
    0.9336, 0.9247, 0.9152, 0.9053,
];

fn by_twentieths(t: &[f64], x: f64) -> f64 {
    let x = (x / 0.05 - 1.0).clamp(0.0, (t.len() - 1) as f64);
    let i = x.floor() as usize;
    let j = (i + 1).min(t.len() - 1);
    t[i] + (t[j] - t[i]) * (x - i as f64)
}

impl Gutter {
    /// Its design depth W, mm: a half-round's radius, a square's side.
    pub fn design_depth_mm(&self) -> f64 {
        self.depth() * 1000.0
    }

    /// Its cross-section A_E, mm².
    pub fn section_mm2(&self) -> f64 {
        let w = self.design_depth_mm();
        match self.profile {
            GutterProfile::HalfRound => std::f64::consts::PI * w * w / 2.0,
            // as wide at the bottom as at the top
            GutterProfile::Square => self.size_mm * w,
        }
    }

    /// The flow a run `run` m long from its high end to an outlet can
    /// carry, l/s (Q_dov = 0,9·Q_N·F_L, less for a corner in the run and
    /// for a sieve); None when the run is too long for the table.
    pub fn capacity(&self, run: f64, corner: bool) -> Option<f64> {
        let w = self.design_depth_mm();
        if w <= 0.0 {
            return None;
        }
        let fl = length_factor(run * 1000.0 / w, self.fall)?;
        let ae = self.section_mm2().powf(1.25);
        let qn = match self.profile {
            GutterProfile::HalfRound => 2.78e-5 * ae,
            GutterProfile::Square => {
                let t = self.size_mm;
                3.48e-5 * ae * by_twentieths(&FD, w / t)
            }
        };
        let mut q = 0.9 * qn * fl;
        if corner {
            q *= 0.85;
        }
        if self.sieve {
            q *= 0.5;
        }
        Some(q)
    }

    /// The stations of its corners (turns over 10°).
    pub fn corner_stations(&self) -> Vec<f64> {
        let st = self.stations();
        self.trace
            .windows(3)
            .enumerate()
            .filter(|(_, w)| {
                let (a, b) = (
                    (w[1] - w[0]).normalize_or_zero(),
                    (w[2] - w[1]).normalize_or_zero(),
                );
                a.dot(b) < 10f64.to_radians().cos()
            })
            .map(|(i, _)| st[i + 1])
            .collect()
    }
}

/// One run of a gutter checked: from its high end to the outlet of
/// `pipe`, `length` m, the flow `q` l/s that comes down it and what it
/// can carry, `allowed` (None: too long for the table).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RunCheck {
    pub pipe: ElementId,
    pub length: f64,
    pub q: f64,
    pub allowed: Option<f64>,
}

impl RunCheck {
    pub fn ok(&self) -> bool {
        self.allowed.is_some_and(|a| a + 1e-9 >= self.q)
    }
}

/// Every run of gutter `id` checked: each downpipe takes a run from each
/// side, its drained area shared between them by their lengths.
pub fn check_gutter(model: &Model, id: ElementId) -> Vec<RunCheck> {
    let Some(g) = gutter_of(model, id) else {
        return vec![];
    };
    let pipes = downpipes_of(model, id);
    let areas = catchment(model, id);
    let len = g.length();
    let corners = g.corner_stations();
    let mut out = vec![];
    for (i, (pipe, at)) in pipes.iter().enumerate() {
        let at = at.clamp(0.0, len);
        let lo = if i == 0 {
            0.0
        } else {
            (pipes[i - 1].1.clamp(0.0, len) + at) / 2.0
        };
        let hi = if i + 1 == pipes.len() {
            len
        } else {
            (pipes[i + 1].1.clamp(0.0, len) + at) / 2.0
        };
        let area = areas.iter().find(|a| a.0 == *pipe).map_or(0.0, |a| a.1);
        let total = (hi - lo).max(1e-9);
        for (a, b) in [(lo, at), (at, hi)] {
            let run = b - a;
            if run < 1e-3 {
                continue;
            }
            let corner = corners.iter().any(|c| *c > a + 1e-6 && *c < b - 1e-6);
            out.push(RunCheck {
                pipe: *pipe,
                length: run,
                q: g.rain * g.runoff * area * run / total,
                allowed: g.capacity(run, corner),
            });
        }
    }
    out
}

/// The roof's true areas, ridges and hips, for the table's lines.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoofFigures {
    /// The roof planes' true area, m².
    pub area: f64,
    pub ridge: f64,
    pub hip: f64,
    pub valley: f64,
}

/// Every roof's figures added up.
pub fn roof_figures(model: &Model) -> RoofFigures {
    let mut out = RoofFigures::default();
    for e in model.iter() {
        let ElementKind::Roof(r) = &e.kind else {
            continue;
        };
        let faces = r.faces();
        for f in &faces {
            let plan = crate::polygon::signed_area2(&f.outline).abs() / 2.0;
            // the plane's slope: area grows by 1 / cos
            out.area += plan * (1.0 + f.g.length_squared()).sqrt();
        }
        // folds: an edge two faces share
        for (i, f) in faces.iter().enumerate() {
            let n = f.outline.len();
            for k in 0..n {
                let (a, b) = (f.outline[k], f.outline[(k + 1) % n]);
                if a.distance(b) < 1e-6 {
                    continue;
                }
                let shared = faces.iter().enumerate().skip(i + 1).any(|(_, h)| {
                    let m = h.outline.len();
                    (0..m).any(|j| {
                        let (c, d) = (h.outline[j], h.outline[(j + 1) % m]);
                        (c.distance(b) < 1e-6 && d.distance(a) < 1e-6)
                            || (c.distance(a) < 1e-6 && d.distance(b) < 1e-6)
                    })
                });
                if !shared {
                    continue;
                }
                let (za, zb) = (f.z(a), f.z(b));
                if (za - zb).abs() < 1e-6 {
                    out.ridge += a.distance(b);
                } else {
                    out.hip += a.extend(za).distance(b.extend(zb));
                }
            }
        }
    }
    out
}

/// One row of the table of sheet-metal items: the same item, its
/// lengths and accessories added up.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub mark: String,
    pub kind: RowKind,
    pub description: String,
    pub developed_mm: f64,
    /// Total length, m.
    pub length: f64,
    /// Accessories: what and how many pieces.
    pub extras: Vec<(String, usize)>,
    pub note: String,
    pub members: Vec<ElementId>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RowKind {
    DownpipeRound,
    DownpipeSquare,
    GutterHalfRound,
    GutterSquare,
}

/// The rows of the table, numbered K-1, K-2… in their order (downpipes,
/// then gutters, as George's table has them) unless marked by hand.
pub fn rows(model: &Model) -> Vec<Row> {
    let mut rows: Vec<Row> = vec![];
    for e in model.iter() {
        let ElementKind::Flashing(f) = &e.kind else {
            continue;
        };
        let (kind, length, extras): (RowKind, f64, Vec<(String, usize)>) = match &f.item {
            FlashingItem::Gutter(g) => {
                let len = g.length();
                let hooks = (len / g.hook_spacing).ceil() as usize + 1;
                let mut x = vec![("Hák".to_string(), hooks)];
                x.push(("Čelo žľabu".into(), 2));
                let c = g.corners();
                if c > 0 {
                    x.push(("Roh žľabu".into(), c));
                }
                let kind = match g.profile {
                    GutterProfile::HalfRound => RowKind::GutterHalfRound,
                    GutterProfile::Square => RowKind::GutterSquare,
                };
                (kind, len, x)
            }
            FlashingItem::Downpipe(d) => {
                let len = downpipe_length(model, d);
                let gutter = gutter_of(model, d.gutter).map_or(0.0, |g| g.size_mm);
                let mut x = vec![];
                if d.elbows > 0 {
                    x.push((
                        format!("Koleno zvodu {}°", num(d.elbow_angle)),
                        d.elbows as usize,
                    ));
                }
                x.push((format!("Kotlík {}/{}", num(gutter), num(d.size_mm)), 1));
                x.push((
                    "Objímka".into(),
                    (len / d.clamp_spacing).ceil().max(1.0) as usize,
                ));
                let kind = match d.profile {
                    DownpipeProfile::Round => RowKind::DownpipeRound,
                    DownpipeProfile::Square => RowKind::DownpipeSquare,
                };
                (kind, len, x)
            }
        };
        let description = f.description();
        let mark = f.mark.clone().unwrap_or_default();
        match rows.iter_mut().find(|r| {
            r.kind == kind
                && r.description == description
                && (r.developed_mm - f.developed()).abs() < 1e-6
                && r.mark == mark
        }) {
            Some(r) => {
                r.length += length;
                for (what, n) in extras {
                    match r.extras.iter_mut().find(|x| x.0 == what) {
                        Some(x) => x.1 += n,
                        None => r.extras.push((what, n)),
                    }
                }
                if !f.note.is_empty() && !r.note.contains(&f.note) {
                    if !r.note.is_empty() {
                        r.note += "; ";
                    }
                    r.note += &f.note;
                }
                r.members.push(e.id);
            }
            None => rows.push(Row {
                mark,
                kind,
                description,
                developed_mm: f.developed(),
                length,
                extras,
                note: f.note.clone(),
                members: vec![e.id],
            }),
        }
    }
    rows.sort_by_key(|r| r.kind);
    let mut k = 0;
    for r in rows.iter_mut() {
        if r.mark.is_empty() {
            k += 1;
            while rows_mark_taken(&format!("K-{k}"), model) {
                k += 1;
            }
            r.mark = format!("K-{k}");
        }
    }
    rows
}

fn rows_mark_taken(mark: &str, model: &Model) -> bool {
    model
        .iter()
        .any(|e| matches!(&e.kind, ElementKind::Flashing(f) if f.mark.as_deref() == Some(mark)))
}

/// The mark of the row element `id` is in.
pub fn mark_of(model: &Model, id: ElementId) -> Option<String> {
    rows(model)
        .into_iter()
        .find(|r| r.members.contains(&id))
        .map(|r| r.mark)
}

/// The nearest point on any roof's eave outline on `level` to `p`, within
/// `reach`: the roof and the point.
pub fn snap_eave(model: &Model, p: DVec2, reach: f64) -> Option<(ElementId, DVec2)> {
    let mut best: Option<(ElementId, DVec2, f64)> = None;
    for e in model.iter() {
        let ElementKind::Roof(r) = &e.kind else {
            continue;
        };
        let o = r.eave_outline();
        let n = o.len();
        for i in 0..n {
            let (a, b) = (o[i], o[(i + 1) % n]);
            // corners first
            for c in [a, b] {
                let d = c.distance(p);
                if d < reach * 0.6 && best.is_none_or(|x| d < x.2) {
                    best = Some((e.id, c, d * 0.5));
                }
            }
            let ab = b - a;
            let t = ((p - a).dot(ab) / ab.length_squared().max(1e-12)).clamp(0.0, 1.0);
            let q = a + ab * t;
            let d = q.distance(p);
            if d < reach && best.is_none_or(|x| d < x.2) {
                best = Some((e.id, q, d));
            }
        }
    }
    best.map(|(id, q, _)| (id, q))
}

#[cfg(test)]
mod capacity_tests {
    use super::*;

    fn half_round(size: f64, fall: f64) -> Gutter {
        let FlashingItem::Gutter(mut g) =
            Flashing::gutter(ElementId::new(), None, vec![DVec2::ZERO, DVec2::X * 10.0]).item
        else {
            unreachable!()
        };
        g.size_mm = size;
        g.fall = fall;
        g
    }

    #[test]
    fn a_gutter_carries_what_the_tzb_info_calculator_gives() {
        // W 60, A_E 5655 mm²: 1,23 l/s short, 1,03 at 10 m, 0,81 at 25 m
        let g = half_round(120.0, 0.0);
        let r = |q: Option<f64>| (q.unwrap() * 100.0).round() / 100.0;
        assert_eq!(r(g.capacity(2.0, false)), 1.23);
        assert_eq!(r(g.capacity(10.0, false)), 1.03);
        assert_eq!(r(g.capacity(25.0, false)), 0.81);
        assert_eq!(g.capacity(30.0, false), None);
        // a fall of 4 mm/m carries 1,10 times as much at L/W 200
        let f = half_round(120.0, 4.0).capacity(12.0, false).unwrap();
        assert!((f / g.capacity(1.0, false).unwrap() - 1.10).abs() < 1e-9);
        // a corner 0,85 times, a sieve half
        let c = g.capacity(2.0, true).unwrap() / g.capacity(2.0, false).unwrap();
        assert!((c - 0.85).abs() < 1e-9);
        let mut s = g.clone();
        s.sieve = true;
        assert!(
            (s.capacity(2.0, false).unwrap() * 2.0 - g.capacity(2.0, false).unwrap()).abs() < 1e-9
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Element, Transaction};

    fn house() -> (Model, ElementId, ElementId) {
        let mut m = Model::new();
        let level = Element::new(
            "L0",
            ElementKind::Level(crate::Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let lid = level.id;
        let mut r = crate::Roof::new(
            lid,
            vec![
                DVec2::new(0.0, 0.0),
                DVec2::new(10.0, 0.0),
                DVec2::new(10.0, 8.0),
                DVec2::new(0.0, 8.0),
            ],
            crate::members::RoofShape::Gable,
        );
        r.overhang = 0.5;
        let roof = Element::new("Roof", ElementKind::Roof(r));
        let rid = roof.id;
        Transaction::new("roof")
            .insert(level)
            .insert(roof)
            .apply(&mut m)
            .unwrap();
        (m, lid, rid)
    }

    #[test]
    fn a_gutter_falls_to_its_downpipes_from_high_points_between_them() {
        let (mut m, level, roof) = house();
        let g = Flashing::gutter(
            level,
            Some(roof),
            vec![DVec2::new(-0.5, -0.5), DVec2::new(10.5, -0.5)],
        );
        let ge = Element::new("Gutter", ElementKind::Flashing(g));
        let gid = ge.id;
        let a = Element::new(
            "Downpipe",
            ElementKind::Flashing(Flashing::downpipe(level, gid, 0.5)),
        );
        let b = Element::new(
            "Downpipe",
            ElementKind::Flashing(Flashing::downpipe(level, gid, 10.5)),
        );
        Transaction::new("g")
            .insert(ge)
            .insert(a)
            .insert(b)
            .apply(&mut m)
            .unwrap();
        let m = &m;
        let g = gutter_of(m, gid).unwrap();
        let outs = outlets(m, gid);
        assert_eq!(g.high_points(&outs), vec![0.0, 5.5, 11.0]);
        // 3 mm/m over the 5 m to the outlet from midway
        assert!((g.drop_at(&outs, 0.5) - 0.015).abs() < 1e-9);
        assert!(g.drop_at(&outs, 5.5).abs() < 1e-9);
        // the eave's half of the roof drains to the two downpipes
        let c = catchment(m, gid);
        let total: f64 = c.iter().map(|x| x.1).sum();
        assert!((total - 11.0 * 4.5).abs() < 1e-6, "{total}");
        assert!((c[0].1 - c[1].1).abs() < 1e-6);
        // the gutter hangs outside the roof
        let axis = g.axis_plan(m);
        assert!(axis[0].y < -0.5);
    }

    #[test]
    fn the_table_adds_up_the_same_items() {
        let (mut m, level, roof) = house();
        let g1 = Flashing::gutter(
            level,
            Some(roof),
            vec![DVec2::new(-0.5, -0.5), DVec2::new(10.5, -0.5)],
        );
        let g2 = Flashing::gutter(
            level,
            Some(roof),
            vec![DVec2::new(10.5, 8.5), DVec2::new(-0.5, 8.5)],
        );
        Transaction::new("g")
            .insert(Element::new("Gutter", ElementKind::Flashing(g1)))
            .insert(Element::new("Gutter", ElementKind::Flashing(g2)))
            .apply(&mut m)
            .unwrap();
        let rows = rows(&m);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].mark, "K-1");
        assert!((rows[0].length - 22.0).abs() < 1e-9);
        assert!(rows[0].description.contains("fi 140 mm"));
        // the developed width counted from the profile, as George's
        // sample has it
        assert_eq!(rows[0].developed_mm, 333.0);
        let fig = roof_figures(&m);
        assert!((fig.ridge - 11.0).abs() < 1e-6, "{fig:?}");
    }

    #[test]
    fn a_downpipe_turns_to_the_facade_under_any_overhang() {
        for overhang in [0.3, 0.8] {
            let (mut m, level, roof) = house();
            let mut tx = Transaction::new("w");
            if let Some(ElementKind::Roof(r)) = m.get(roof).map(|e| e.kind.clone()) {
                let mut r = r;
                r.overhang = overhang;
                r.base_offset = 3.0;
                let mut e = m.get(roof).unwrap().clone();
                e.kind = ElementKind::Roof(r);
                tx = tx.replace(m.get(roof).unwrap().clone(), e);
            }
            // a wall 500 thick (with its insulation) under the eave
            let wall =
                crate::Wall::new(level, DVec2::new(0.0, 0.0), DVec2::new(10.0, 0.0), 0.5, 3.0);
            tx = tx.insert(Element::new("W", ElementKind::Wall(wall)));
            tx.apply(&mut m).unwrap();
            let y = -overhang;
            let g = Flashing::gutter(
                level,
                Some(roof),
                vec![DVec2::new(-overhang, y), DVec2::new(10.0 + overhang, y)],
            );
            let ge = Element::new("Gutter", ElementKind::Flashing(g));
            let gid = ge.id;
            let d = Flashing::downpipe(level, gid, 1.0);
            let de = Element::new("Downpipe", ElementKind::Flashing(d.clone()));
            Transaction::new("g")
                .insert(ge)
                .insert(de)
                .apply(&mut m)
                .unwrap();
            let FlashingItem::Downpipe(d) = d.item else {
                unreachable!()
            };
            let path = downpipe_path(&m, &d);
            assert_eq!(path.len(), 4, "{path:?}");
            // down the facade: its face at -0.25, a 30 mm gap, half of 100
            assert!((path[3].y + 0.25 + 0.03 + 0.05).abs() < 1e-9, "{path:?}");
            assert!((path[2].y - path[3].y).abs() < 1e-9);
        }
    }
}
