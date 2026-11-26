//! Timber roof framing (drevený krov) and steel members (George
//! 2026-11-26): rafters, purlins, wall plates, posts, collar ties, struts
//! and the rest of a roof's timber, and steel columns, beams (level or
//! sloped) and bracing rods with turnbuckles, drawn as the structural
//! engineer's drawing gives them. Not a structural check: each member
//! keeps what a later calculation needs (its axis, section and its
//! properties, material grade, how its ends are held).
//!
//! A member is a [`crate::Beam`] or a [`crate::Column`] carrying a
//! [`Framing`]; its geometry is the beam's or column's own.

use crate::{Element, ElementId, ElementKind, Model};
use glam::{DVec2, DVec3};
use serde::{Deserialize, Serialize};

/// What a member is in the roof or the frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    /// Krokva.
    #[default]
    Rafter,
    /// Väznica (a middle purlin).
    Purlin,
    /// Vrcholová (hrebeňová) väznica.
    RidgePurlin,
    /// Pomúrnica, on the wall's crown.
    WallPlate,
    /// Stĺpik under a purlin.
    Post,
    /// Kleštiny: a pair of boards clamping the rafters.
    Collar,
    /// Vzpera: a strut from a tie beam or post up to a rafter or purlin.
    Strut,
    /// Pásik: the short knee brace between a post and a purlin.
    KneeBrace,
    /// Väzný trám (a tie beam the posts stand on).
    TieBeam,
    /// Výmena: a trimmer round a chimney or a roof window.
    Trimmer,
    /// Úžlabná krokva: the valley rafter.
    ValleyRafter,
    /// Nárožná krokva: the hip rafter.
    HipRafter,
    /// Námetok: a short rafter extension at the eaves.
    Sprocket,
    /// A steel column.
    SteelColumn,
    /// A steel beam, level or sloped.
    SteelBeam,
    /// A bracing rod (zavetrovacia tyč), with a turnbuckle or not.
    BracingRod,
}

impl Role {
    /// In the order a structural drawing lists them (George's sample
    /// výkaz reziva 2026-11-26: pomúrnica, stredová väznica, vrcholová
    /// väznica, stĺpik, krokva, kliešťina, výmena, úžlabná krokva).
    pub const TIMBER: [Role; 13] = [
        Role::WallPlate,
        Role::Purlin,
        Role::RidgePurlin,
        Role::Post,
        Role::Rafter,
        Role::Collar,
        Role::Trimmer,
        Role::ValleyRafter,
        Role::HipRafter,
        Role::Strut,
        Role::KneeBrace,
        Role::TieBeam,
        Role::Sprocket,
    ];
    pub const STEEL: [Role; 3] = [Role::SteelColumn, Role::SteelBeam, Role::BracingRod];
    /// The timber, then the steel: the menus' order.
    pub const ALL: [Role; 16] = [
        Role::WallPlate,
        Role::Purlin,
        Role::RidgePurlin,
        Role::Post,
        Role::Rafter,
        Role::Collar,
        Role::Trimmer,
        Role::ValleyRafter,
        Role::HipRafter,
        Role::Strut,
        Role::KneeBrace,
        Role::TieBeam,
        Role::Sprocket,
        Role::SteelColumn,
        Role::SteelBeam,
        Role::BracingRod,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Role::Rafter => "Rafter",
            Role::Purlin => "Middle purlin",
            Role::RidgePurlin => "Ridge purlin",
            Role::WallPlate => "Wall plate",
            Role::Post => "Post",
            Role::Collar => "Collar tie",
            Role::ValleyRafter => "Valley rafter",
            Role::HipRafter => "Hip rafter",
            Role::Strut => "Strut",
            Role::KneeBrace => "Knee brace",
            Role::TieBeam => "Tie beam",
            Role::Trimmer => "Trimmer",
            Role::Sprocket => "Sprocket",
            Role::SteelColumn => "Steel column",
            Role::SteelBeam => "Steel beam",
            Role::BracingRod => "Bracing rod",
        }
    }

    /// The key of its icon.
    pub fn icon(self) -> &'static str {
        match self {
            Role::Post | Role::SteelColumn => "column",
            Role::BracingRod => "spline",
            _ => "beam",
        }
    }

    pub fn is_timber(self) -> bool {
        !Role::STEEL.contains(&self)
    }

    /// Is it upright, placed with a click as a column?
    pub fn is_column(self) -> bool {
        matches!(self, Role::Post | Role::SteelColumn)
    }

    /// Does it lie on a slope as it is drawn (its angle set in the bar)?
    pub fn slopes(self) -> bool {
        matches!(
            self,
            Role::Rafter
                | Role::ValleyRafter
                | Role::HipRafter
                | Role::Strut
                | Role::KneeBrace
                | Role::Sprocket
                | Role::SteelBeam
                | Role::BracingRod
        )
    }

    /// The slope a new one starts with, degrees.
    pub fn default_angle(self) -> f64 {
        match self {
            Role::Rafter | Role::Sprocket => 35.0,
            // in plan a valley or hip runs at 45° to the eaves: its slope
            // is set by the roof's (tan α / √2), typed in the bar
            Role::ValleyRafter | Role::HipRafter => 26.0,
            Role::Strut | Role::KneeBrace => 45.0,
            _ => 0.0,
        }
    }

    /// The material grade a new one starts with: timber strength class
    /// C24 (STN EN 338), steel S235 (STN EN 10025-2).
    pub fn default_grade(self) -> &'static str {
        if self.is_timber() {
            "C24"
        } else {
            "S235"
        }
    }

    /// The section a new one starts with.
    pub fn default_section(self) -> &'static str {
        match self {
            Role::Rafter | Role::Sprocket => "KVH 80 × 160",
            Role::Purlin | Role::RidgePurlin | Role::TieBeam => "KVH 140 × 200",
            Role::WallPlate => "KVH 140 × 140",
            Role::Post => "KVH 140 × 140",
            Role::Collar => "KVH 60 × 160",
            Role::Strut | Role::KneeBrace => "KVH 120 × 120",
            Role::Trimmer => "KVH 80 × 160",
            Role::ValleyRafter | Role::HipRafter => "KVH 100 × 240",
            Role::SteelColumn => "HEA 160",
            Role::SteelBeam => "IPE 200",
            Role::BracingRod => "Ø 16",
        }
    }

    /// The letter its position marks start with: D-01, D-02… for the
    /// roof's timber (drevo, as George's sample marks it), O-01… for the
    /// steel.
    pub fn prefix(self) -> &'static str {
        if self.is_timber() {
            "D"
        } else {
            "O"
        }
    }

    /// How the ends of a new one are held, for a later calculation: a
    /// rod and every timber joint a hinge.
    pub fn default_ends(self) -> [EndFix; 2] {
        [EndFix::Pinned, EndFix::Pinned]
    }
}

/// How an end of a member is held, for a later calculation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndFix {
    /// A hinge: held, free to turn.
    #[default]
    Pinned,
    /// Fixed against turning too.
    Fixed,
    /// Free (a cantilever's end).
    Free,
}

impl EndFix {
    pub const ALL: [EndFix; 3] = [EndFix::Pinned, EndFix::Fixed, EndFix::Free];
    pub fn label(self) -> &'static str {
        match self {
            EndFix::Pinned => "Pinned",
            EndFix::Fixed => "Fixed",
            EndFix::Free => "Free",
        }
    }
}

/// What makes a beam or column a member of the roof's timber or of a
/// steel frame.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Framing {
    pub role: Role,
    /// Its section by name: "KVH 100 × 160", "100/160" (timber, b/h in
    /// mm), "HEA 160", "Ø 16".
    pub section: String,
    /// Strength class or steel grade: "C24", "S235".
    pub grade: String,
    /// How its start and end (a column's foot and head) are held.
    #[serde(default)]
    pub ends: [EndFix; 2],
    /// A bracing rod's turnbuckle (napínák) at its middle.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub turnbuckle: bool,
    /// A mark typed over the numbered one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mark: Option<String>,
}

impl Framing {
    pub fn new(role: Role) -> Self {
        Self {
            role,
            section: role.default_section().into(),
            grade: role.default_grade().into(),
            ends: role.default_ends(),
            turnbuckle: role == Role::BracingRod,
            mark: None,
        }
    }
}

/// The framing of `e`, if it is a member.
pub fn of(e: &Element) -> Option<&Framing> {
    match &e.kind {
        ElementKind::Beam(b) => b.framing.as_ref(),
        ElementKind::Column(c) => c.framing.as_ref(),
        _ => None,
    }
}

// ---------------------------------------------------------------- sections

/// A section's shape family.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// Sawn or glued timber, b × h.
    Timber,
    /// A rolled I or H (IPE, HEA, HEB).
    I,
    /// A rolled channel (UPN).
    U,
    /// An equal angle.
    L,
    /// A closed square or rectangular hollow section (jäkl), h × b × t.
    Hollow,
    /// A round bar.
    Bar,
}

/// A section with what a calculation needs, in SI units (m, m², m⁴…).
#[derive(Clone, Debug, PartialEq)]
pub struct SectionData {
    pub name: String,
    pub shape: Shape,
    /// Height h and width b.
    pub h: f64,
    pub b: f64,
    /// Web and flange thickness (steel), m.
    pub tw: f64,
    pub tf: f64,
    /// Root radius (rolled steel), m.
    pub r: f64,
    /// Area, m².
    pub area: f64,
    /// Second moments about the strong (y) and weak (z) axis, m⁴.
    pub iy: f64,
    pub iz: f64,
    /// Elastic section moduli, m³.
    pub wy: f64,
    pub wz: f64,
    /// Plastic section moduli (steel), m³.
    pub wply: Option<f64>,
    pub wplz: Option<f64>,
    /// Mass per metre (steel), kg/m; timber is listed by volume.
    pub mass: Option<f64>,
    /// Where the numbers come from.
    pub source: &'static str,
}

impl SectionData {
    /// The section's outline about its centre, x across, y up.
    pub fn outline(&self) -> Vec<DVec2> {
        let (h, b) = (self.h, self.b);
        let (x, y) = (b / 2.0, h / 2.0);
        let p = |v: &[(f64, f64)]| v.iter().map(|&(a, c)| DVec2::new(a, c)).collect::<Vec<_>>();
        match self.shape {
            Shape::Timber => p(&[(-x, -y), (x, -y), (x, y), (-x, y)]),
            Shape::I => {
                let (w, tf) = (self.tw / 2.0, self.tf);
                p(&[
                    (-x, -y),
                    (x, -y),
                    (x, -y + tf),
                    (w, -y + tf),
                    (w, y - tf),
                    (x, y - tf),
                    (x, y),
                    (-x, y),
                    (-x, y - tf),
                    (-w, y - tf),
                    (-w, -y + tf),
                    (-x, -y + tf),
                ])
            }
            Shape::U => {
                // the web on the left, the flanges reaching right
                let (tw, tf) = (self.tw, self.tf);
                p(&[
                    (-x, -y),
                    (x, -y),
                    (x, -y + tf),
                    (-x + tw, -y + tf),
                    (-x + tw, y - tf),
                    (x, y - tf),
                    (x, y),
                    (-x, y),
                ])
            }
            Shape::L => {
                let t = self.tw;
                p(&[
                    (-x, -y),
                    (x, -y),
                    (x, -y + t),
                    (-x + t, -y + t),
                    (-x + t, y),
                    (-x, y),
                ])
            }
            Shape::Hollow => {
                let t = self.tw;
                let outer = p(&[(-x, -y), (x, -y), (x, y), (-x, y)]);
                let inner = p(&[
                    (-x + t, -y + t),
                    (-x + t, y - t),
                    (x - t, y - t),
                    (x - t, -y + t),
                ]);
                crate::profiles::merge(outer, &[inner])
            }
            Shape::Bar => (0..16)
                .map(|i| DVec2::from_angle(std::f64::consts::TAU * i as f64 / 16.0) * (b / 2.0))
                .collect(),
        }
    }
}

/// The density of steel for masses worked out here: 7850 kg/m³ (STN EN
/// 1993-1-1, 3.2.6).
pub const STEEL_DENSITY: f64 = 7850.0;

/// A timber section b × h, its properties worked out.
pub fn timber(name: &str, b: f64, h: f64) -> SectionData {
    SectionData {
        name: name.into(),
        shape: Shape::Timber,
        h,
        b,
        tw: 0.0,
        tf: 0.0,
        r: 0.0,
        area: b * h,
        iy: b * h.powi(3) / 12.0,
        iz: h * b.powi(3) / 12.0,
        wy: b * h * h / 6.0,
        wz: h * b * b / 6.0,
        wply: None,
        wplz: None,
        mass: None,
        source: "b × h",
    }
}

/// A round steel bar Ø `d`, its properties worked out.
pub fn bar(d: f64) -> SectionData {
    let r = d / 2.0;
    let i = std::f64::consts::PI * r.powi(4) / 4.0;
    let area = std::f64::consts::PI * r * r;
    SectionData {
        name: format!("Ø {:.0}", d * 1000.0),
        shape: Shape::Bar,
        h: d,
        b: d,
        tw: 0.0,
        tf: 0.0,
        r: 0.0,
        area,
        iy: i,
        iz: i,
        wy: i / r,
        wz: i / r,
        // a solid round: d³ / 6
        wply: Some(d.powi(3) / 6.0),
        wplz: Some(d.powi(3) / 6.0),
        mass: Some(area * STEEL_DENSITY),
        source: "Ø, 7850 kg/m³",
    }
}

/// KVH sections sold by DEKWOOD (dekwood.cz/produkty/kvh), b × h in mm,
/// as the stair stringers already take them.
pub const KVH: [(u32, u32); 10] = [
    (60, 120),
    (60, 140),
    (60, 160),
    (60, 200),
    (60, 240),
    (80, 160),
    (80, 200),
    (80, 240),
    (100, 200),
    (100, 240),
];

/// The timber sections offered: the sellers' sizes.
pub fn timber_sections() -> Vec<SectionData> {
    KVH.iter()
        .map(|&(b, h)| {
            let mut s = timber(
                &format!("KVH {b} × {h}"),
                b as f64 / 1000.0,
                h as f64 / 1000.0,
            );
            s.source = "DEKWOOD KVH";
            s
        })
        .collect()
}

/// Round bars offered for bracing rods, Ø in mm: the threads the
/// turnbuckles take (see [`TURNBUCKLES`]).
pub const BARS: [u32; 6] = [12, 16, 20, 24, 27, 30];

/// A turnbuckle for a bracing rod.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Turnbuckle {
    pub code: &'static str,
    /// The rod's thread, mm.
    pub thread: u32,
    /// The tube's length (A) and outer diameter (C), m.
    pub length: f64,
    pub tube: f64,
    /// How deep a rod goes in at each end (K), m.
    pub insert: f64,
    /// Characteristic tensile resistance Nax,k, kN (for a later check).
    pub nax_k: f64,
}

/// Rothoblaas TENDITORE turnbuckles (DIN 1478, S355 galvanized;
/// rothoblaas.com TENDITORE-sk.pdf): code, thread, A, C, K mm, Nax,k kN.
/// M27 the maker marks as not to DIN 1478.
pub const TURNBUCKLES: [Turnbuckle; 6] = [
    tb("SSS12125", 12, 125.0, 25.0, 35.0, 66.20),
    tb("SSS16170", 16, 170.0, 30.0, 45.0, 97.38),
    tb("SSS20200", 20, 200.0, 33.7, 55.0, 119.09),
    tb("SSS24255", 24, 255.0, 42.4, 70.0, 184.69),
    tb("SSS27255", 27, 255.0, 42.4, 85.0, 184.69),
    tb("SSS30255", 30, 255.0, 51.0, 85.0, 245.92),
];

const fn tb(code: &'static str, thread: u32, a: f64, c: f64, k: f64, n: f64) -> Turnbuckle {
    Turnbuckle {
        code,
        thread,
        length: a / 1000.0,
        tube: c / 1000.0,
        insert: k / 1000.0,
        nax_k: n,
    }
}

/// The turnbuckle a rod Ø `d` (m) takes.
pub fn turnbuckle_for(d: f64) -> Option<Turnbuckle> {
    let mm = (d * 1000.0).round() as u32;
    TURNBUCKLES.iter().copied().find(|t| t.thread == mm)
}

/// The steel sections offered.
pub fn steel_sections() -> Vec<SectionData> {
    let mut v = crate::steeltables::rolled();
    v.extend(BARS.iter().map(|d| bar(*d as f64 / 1000.0)));
    v
}

/// A section by name: the catalogues', else a timber "b/h" or "b × h"
/// in mm, else a bar "Ø d".
pub fn section(name: &str) -> Option<SectionData> {
    let name = name.trim();
    if let Some(s) = timber_sections()
        .into_iter()
        .chain(steel_sections())
        .find(|s| s.name == name)
    {
        return Some(s);
    }
    if let Some(d) = name.strip_prefix("Ø") {
        let d: f64 = d.trim().replace(',', ".").parse().ok()?;
        return (d > 0.0).then(|| bar(d / 1000.0));
    }
    let (b, h) = name
        .split_once('/')
        .or_else(|| name.split_once('×'))
        .or_else(|| name.split_once('x'))?;
    let num = |s: &str| {
        s.trim()
            .replace(',', ".")
            .parse::<f64>()
            .ok()
            .filter(|v| *v > 0.0)
    };
    let (b, h) = (num(b)?, num(h)?);
    Some(timber(&format!("{b:.0}/{h:.0}"), b / 1000.0, h / 1000.0))
}

/// The label a member's section is written with on a drawing: timber
/// "100/160" (b/h), steel by its name.
pub fn section_label(s: &SectionData) -> String {
    match s.shape {
        Shape::Timber => format!("{:.0}/{:.0}", s.b * 1000.0, s.h * 1000.0),
        _ => s.name.clone(),
    }
}

/// Set `e`'s beam or column to `f`'s section: its width and depth, its
/// outline for a steel one. A section not known is left as it is.
pub fn apply(e: &mut Element, f: Framing) {
    match &mut e.kind {
        ElementKind::Beam(b) => {
            b.framing = Some(f);
            fit_beam(b);
        }
        ElementKind::Column(c) => {
            c.framing = Some(f);
            fit_column(c);
        }
        _ => {}
    }
}

/// A member beam's width, depth and outline from its framing's section.
pub fn fit_beam(b: &mut crate::Beam) {
    let Some(s) = b.framing.as_ref().and_then(|f| section(&f.section)) else {
        return;
    };
    b.width = s.b;
    b.depth = s.h;
    b.section = (s.shape != Shape::Timber).then(|| crate::members::Section {
        name: s.name.clone(),
        points: s.outline(),
    });
}

/// A member column's profile and outline from its framing's section.
pub fn fit_column(c: &mut crate::Column) {
    let Some(s) = c.framing.as_ref().and_then(|f| section(&f.section)) else {
        return;
    };
    c.profile = match s.shape {
        Shape::Bar => crate::members::Profile::Round { diameter: s.b },
        _ => crate::members::Profile::Rectangle {
            width: s.b,
            depth: s.h,
        },
    };
    c.section = (!matches!(s.shape, Shape::Timber | Shape::Bar)).then(|| crate::members::Section {
        name: s.name.clone(),
        points: s.outline(),
    });
}

/// The names of the sections a member of `role` can take.
pub fn section_names(role: Role) -> Vec<String> {
    if role.is_timber() {
        timber_sections().into_iter().map(|s| s.name).collect()
    } else if role == Role::BracingRod {
        BARS.iter().map(|d| format!("Ø {d}")).collect()
    } else {
        steel_sections().into_iter().map(|s| s.name).collect()
    }
}

/// A member's axis in 3D (the analytical line a calculation takes): a
/// beam's from its top line down to the section's middle, a column's
/// from foot to head through its position.
pub fn axis(model: &Model, e: &Element) -> Option<(DVec3, DVec3)> {
    match &e.kind {
        ElementKind::Beam(b) => {
            let (a, z) = b.axis(model);
            let up = b.up.unwrap_or(DVec3::Z).normalize_or(DVec3::Z);
            let down = up * (b.depth / 2.0);
            Some((a - down, z - down))
        }
        ElementKind::Column(c) => {
            let (z0, z1) = c.span(model);
            Some((c.position.extend(z0), c.position.extend(z1)))
        }
        _ => None,
    }
}

/// A member's length along its axis, m.
pub fn length(model: &Model, e: &Element) -> f64 {
    axis(model, e).map_or(0.0, |(a, b)| a.distance(b))
}

/// A sloped member from `a` to `b` in plan rising `angle` degrees from
/// `a`: how far its end rises above its start.
pub fn rise_for(a: DVec2, b: DVec2, angle: f64) -> f64 {
    a.distance(b) * angle.to_radians().tan()
}

// ---------------------------------------------------------------- positions

/// One position of the timber list (výkaz reziva) or the steel list:
/// members alike in what they are, their section, grade and length (George's
/// sample: D-01 and D-02 are both wall plates 150/150, 6,420 and 2,900 m).
#[derive(Clone, Debug, PartialEq)]
pub struct Position {
    /// `D-01`, `O-03`, or a typed mark.
    pub mark: String,
    pub role: Role,
    pub section: SectionData,
    pub grade: String,
    /// Each one's length, m.
    pub length: f64,
    pub members: Vec<ElementId>,
    /// Bracing rods with a turnbuckle.
    pub turnbuckles: usize,
}

impl Position {
    pub fn count(&self) -> usize {
        self.members.len()
    }
    /// Timber volume of all of them, m³.
    pub fn volume(&self) -> f64 {
        self.section.area * self.length * self.count() as f64
    }
    /// Steel mass of all of them, kg.
    pub fn mass(&self) -> Option<f64> {
        self.section
            .mass
            .map(|m| m * self.length * self.count() as f64)
    }
}

fn mm(v: f64) -> i64 {
    (v * 1000.0).round() as i64
}

/// The positions of the timber (`timber`) or the steel: by what they are
/// (in [`Role`]'s order), then section, then the longest first; numbered
/// D-01… or O-01… unless a mark is typed.
pub fn positions(model: &Model, timber: bool) -> Vec<Position> {
    let mut out: Vec<Position> = vec![];
    for e in model.iter() {
        let Some(f) = of(e) else { continue };
        if f.role.is_timber() != timber {
            continue;
        }
        let Some(s) = section(&f.section) else {
            continue;
        };
        let len = length(model, e);
        let same = |p: &Position| {
            p.role == f.role
                && p.section.name == s.name
                && p.grade == f.grade
                && mm(p.length) == mm(len)
                && p.mark == f.mark.clone().unwrap_or_default()
        };
        let tb = (f.role == Role::BracingRod && f.turnbuckle) as usize;
        match out.iter_mut().find(|p| same(p)) {
            Some(p) => {
                p.members.push(e.id);
                p.turnbuckles += tb;
            }
            None => out.push(Position {
                mark: f.mark.clone().unwrap_or_default(),
                role: f.role,
                section: s,
                grade: f.grade.clone(),
                length: len,
                members: vec![e.id],
                turnbuckles: tb,
            }),
        }
    }
    let rank = |r: Role| {
        Role::TIMBER
            .iter()
            .chain(Role::STEEL.iter())
            .position(|x| *x == r)
            .unwrap_or(99)
    };
    out.sort_by(|a, b| {
        rank(a.role)
            .cmp(&rank(b.role))
            .then_with(|| a.section.name.cmp(&b.section.name))
            .then_with(|| mm(b.length).cmp(&mm(a.length)))
    });
    let mut n = 0;
    for p in out.iter_mut() {
        if p.mark.is_empty() {
            n += 1;
            p.mark = format!("{}-{n:02}", p.role.prefix());
        }
    }
    out
}

/// The position mark of member `id` and its label on a drawing:
/// ("K1", "K1 krokva 80/160" is the caller's to translate).
pub fn mark_of(model: &Model, id: ElementId) -> Option<(String, String)> {
    let e = model.get(id)?;
    let f = of(e)?;
    let p = positions(model, f.role.is_timber())
        .into_iter()
        .find(|p| p.members.contains(&id))?;
    Some((p.mark, section_label(&p.section)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timber_properties_are_worked_out_from_b_and_h() {
        let s = section("100/160").unwrap();
        assert_eq!(s.name, "100/160");
        assert!((s.area - 0.016).abs() < 1e-12);
        // b h³ / 12 = 0.1 · 0.16³ / 12 = 3.4133e-5 m⁴
        assert!((s.iy - 3.413_333e-5).abs() < 1e-9);
        assert!((s.wy - 0.1 * 0.16 * 0.16 / 6.0).abs() < 1e-12);
        assert_eq!(section("KVH 80 × 160").unwrap().source, "DEKWOOD KVH");
    }

    #[test]
    fn a_bar_weighs_its_area_times_steel_density() {
        let s = section("Ø 16").unwrap();
        // π · 0.008² · 7850 = 1.578 kg/m
        assert!((s.mass.unwrap() - 1.5783).abs() < 1e-3);
    }

    #[test]
    fn rise_follows_the_angle() {
        let r = rise_for(DVec2::ZERO, DVec2::new(4.0, 0.0), 45.0);
        assert!((r - 4.0).abs() < 1e-9);
    }
}
