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

    /// Drawn in the krov plan as one thin dash-dot line along its axis,
    /// not its outline (George 2026-11-26, as his sample drawing: the
    /// rafters, hip and valley rafters over everything else).
    pub fn schematic(self) -> bool {
        matches!(
            self,
            Role::Rafter | Role::ValleyRafter | Role::HipRafter | Role::Sprocket
        )
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

/// How a timber member's end is cut, seen from its side (George
/// 2026-11-27, his sketches of a rafter at the eaves and at the ridge):
/// every cut starts at the end's point on the top edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum EndCut {
    /// Square to the member's axis.
    #[default]
    Square,
    /// Plumb (zvislý rez).
    Plumb,
    /// Plumb down to the height of a square cut's lower corner, then level
    /// in to it (zvislý a vodorovný rez, at the eaves).
    PlumbLevel,
    /// Level (vodorovný rez): a brace butting under a purlin.
    Level,
    /// At this angle from the horizontal, degrees, the lower edge running
    /// on outward (a collar tie cut to the rafter's pitch).
    Angle(f64),
    /// Against the square-cut end and the underside of the member meeting
    /// it at the same point (a rafter at the ridge: one runs through cut
    /// square, the other is fitted to it).
    Fitted,
    /// A lap joint (preplátovanie, spoj na plát) over this length, m: the
    /// lower half runs on (`upper` false) or the upper half does.
    Lap { length: f64, upper: bool },
}

impl EndCut {
    /// The kinds offered, an angle and a lap with their first values.
    pub const ALL: [EndCut; 7] = [
        EndCut::Square,
        EndCut::Plumb,
        EndCut::PlumbLevel,
        EndCut::Level,
        EndCut::Angle(90.0),
        EndCut::Fitted,
        EndCut::Lap {
            length: 0.0,
            upper: false,
        },
    ];

    pub fn label(self) -> &'static str {
        match self {
            EndCut::Square => "Square cut",
            EndCut::Plumb => "Plumb cut",
            EndCut::PlumbLevel => "Plumb and level cut",
            EndCut::Level => "Level cut",
            EndCut::Angle(_) => "Cut at an angle",
            EndCut::Fitted => "Fitted to the member it meets",
            EndCut::Lap { .. } => "Lap joint",
        }
    }

    /// The same kind (an angle or a lap whatever its values).
    pub fn same_kind(self, other: EndCut) -> bool {
        std::mem::discriminant(&self) == std::mem::discriminant(&other)
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
    /// How a timber beam's start and end are cut (see [`side_profile`]).
    #[serde(default, skip_serializing_if = "square_ends")]
    pub cuts: [EndCut; 2],
}

fn square_ends(c: &[EndCut; 2]) -> bool {
    c[0] == EndCut::Square && c[1] == EndCut::Square
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
            cuts: [EndCut::Square; 2],
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
    /// The perimeter painted, m (painting surface per metre, m²/m),
    /// worked out from the section's dimensions: an I or U section's
    /// faces with the root radii (the toe radii and a UPN's flange slope
    /// left out), an angle's, a hollow section's with its outer corners
    /// rounded to 2 t (EN 10219-2 for t ≤ 6 mm; 2,5 t to 10 mm), a round
    /// bar's circumference, timber's four sides.
    pub fn paint_perimeter(&self) -> f64 {
        use std::f64::consts::PI;
        let (h, b, tw, r) = (self.h, self.b, self.tw, self.r);
        match self.shape {
            Shape::Timber => 2.0 * (h + b),
            Shape::I => 2.0 * h + 4.0 * b - 2.0 * tw - (8.0 - 2.0 * PI) * r,
            Shape::U => 2.0 * h + 4.0 * b - 2.0 * tw - (4.0 - PI) * r,
            Shape::L => 2.0 * (h + b) - (2.0 - PI / 2.0) * r,
            Shape::Hollow => {
                let ro = if tw <= 0.006 + 1e-9 {
                    2.0 * tw
                } else {
                    2.5 * tw
                };
                2.0 * (h + b) - (8.0 - 2.0 * PI) * ro
            }
            Shape::Bar => PI * b,
        }
    }

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

/// A member's length along its axis, m; a timber beam's cut ends taken
/// in, the longest it runs (the piece to order).
pub fn length(model: &Model, e: &Element) -> f64 {
    if let Some(p) = side_profile(model, e) {
        let d = glam::DVec2::new(1.0, p.slope).normalize();
        let s: Vec<f64> = p.points.iter().map(|q| q.dot(d)).collect();
        let lo = s.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = s.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        return hi - lo;
    }
    axis(model, e).map_or(0.0, |(a, b)| a.distance(b))
}

/// A timber beam seen from its side: the outline of its cut ends, in the
/// upright plane through its axis.
#[derive(Clone, Debug, PartialEq)]
pub struct SideProfile {
    /// The start of its top line in the model, m.
    pub origin: DVec3,
    /// Its direction in plan, unit.
    pub dir: glam::DVec2,
    /// How much its top rises a metre in plan.
    pub slope: f64,
    /// Its outline: x along `dir` in plan, z up from `origin`, m: the end's
    /// cut from its top point down, then the start's from its bottom up.
    pub points: Vec<glam::DVec2>,
    /// Its width across, m.
    pub width: f64,
}

/// A timber beam's side outline with its ends cut as its framing says;
/// none for steel, a column, a beam lying on a slope sideways (its
/// section not upright) or one without framing.
pub fn side_profile(model: &Model, e: &Element) -> Option<SideProfile> {
    use glam::DVec2;
    let ElementKind::Beam(b) = &e.kind else {
        return None;
    };
    let f = b.framing.as_ref()?;
    if !f.role.is_timber() {
        return None;
    }
    let (a, z) = b.axis(model);
    let run = (z - a).truncate();
    let len = run.length();
    if len < 1e-6 {
        return None;
    }
    let dir = run / len;
    let across = dir.perp().extend(0.0);
    // the section upright in the plane of the axis
    if b.up
        .is_some_and(|u| u.normalize_or(DVec3::Z).dot(across).abs() > 1e-3)
    {
        return None;
    }
    let slope = (z.z - a.z) / len;
    let h = b.depth;
    let hv = h * (1.0 + slope * slope).sqrt();
    let top = |x: f64| slope * x;
    let bottom = |x: f64| slope * x - hv;
    // square to the axis, down
    let down = DVec2::new(slope, -1.0).normalize() * h;
    // where the line from `p` along `d` meets the bottom line
    let to_bottom = |p: DVec2, d: DVec2| -> Option<DVec2> {
        let den = d.y - slope * d.x;
        if den.abs() < 1e-9 {
            return None;
        }
        let t = (bottom(p.x) - p.y) / den;
        (t > 0.0).then(|| p + d * t)
    };
    let cut = |k: usize| -> Vec<DVec2> {
        let tx = if k == 0 { 0.0 } else { len };
        let t = DVec2::new(tx, top(tx));
        let out = if k == 0 { -1.0 } else { 1.0 };
        let plumb = vec![t, DVec2::new(tx, bottom(tx))];
        match f.cuts[k] {
            EndCut::Square => vec![t, t + down],
            EndCut::Plumb => plumb,
            EndCut::PlumbLevel => {
                let sq = t + down;
                // the square cut's lower corner inside the end: plumb to
                // its height, level in to it
                if (sq.x - tx) * out < -1e-6 {
                    vec![t, DVec2::new(tx, sq.y), sq]
                } else {
                    plumb
                }
            }
            EndCut::Level => to_bottom(t, DVec2::new(out, 0.0))
                .or_else(|| to_bottom(t, DVec2::new(-out, 0.0)))
                .map_or(plumb.clone(), |p| vec![t, p]),
            EndCut::Angle(deg) => {
                let r = deg.clamp(5.0, 175.0).to_radians();
                to_bottom(t, DVec2::new(out * r.cos(), -r.sin())).map_or(plumb, |p| vec![t, p])
            }
            EndCut::Lap { length, upper } => {
                let l = length.max(0.0).min(len / 2.0);
                if l < 1e-6 {
                    return plumb;
                }
                let x1 = tx - out * l;
                let mid = |x: f64| top(x) - hv / 2.0;
                if upper {
                    vec![
                        t,
                        DVec2::new(tx, mid(tx)),
                        DVec2::new(x1, mid(x1)),
                        DVec2::new(x1, bottom(x1)),
                    ]
                } else {
                    vec![
                        DVec2::new(x1, top(x1)),
                        DVec2::new(x1, mid(x1)),
                        DVec2::new(tx, mid(tx)),
                        DVec2::new(tx, bottom(tx)),
                    ]
                }
            }
            EndCut::Fitted => fitted(model, e.id, b, k, a, dir, slope, hv, t).unwrap_or(plumb),
        }
    };
    let end = cut(1);
    let start = cut(0);
    let mut points = end;
    points.extend(start.into_iter().rev());
    Some(SideProfile {
        origin: a,
        dir,
        slope,
        points,
        width: b.width,
    })
}

/// A rafter's end at the ridge fitted to the rafter meeting it there
/// (square cut, running through): down its cut face, then up under its
/// underside to this one's own. In `this` beam's side frame.
#[allow(clippy::too_many_arguments)]
fn fitted(
    model: &Model,
    id: ElementId,
    this: &crate::Beam,
    k: usize,
    origin: DVec3,
    dir: glam::DVec2,
    slope: f64,
    hv: f64,
    t: glam::DVec2,
) -> Option<Vec<glam::DVec2>> {
    use glam::DVec2;
    let (a, z) = this.axis(model);
    let at = if k == 0 { a } else { z };
    let frame = |p: DVec3| DVec2::new((p.truncate() - origin.truncate()).dot(dir), p.z - origin.z);
    for o in model.iter() {
        let ElementKind::Beam(ob) = &o.kind else {
            continue;
        };
        if o.id == id || ob.framing.is_none() {
            continue;
        }
        let (oa, oz) = ob.axis(model);
        // the other's end at this one's end, in the same upright plane
        let near = if oa.distance(at) < 0.02 {
            oa
        } else if oz.distance(at) < 0.02 {
            oz
        } else {
            continue;
        };
        let far = if near == oa { oz } else { oa };
        let off = (far.truncate() - origin.truncate()).perp_dot(dir).abs();
        if off > 0.02 {
            continue;
        }
        let (p, q) = (frame(near), frame(far));
        let os = (q.y - p.y) / (q.x - p.x);
        if !os.is_finite() || (os - slope).abs() < 1e-6 {
            continue;
        }
        let ohv = ob.depth * (1.0 + os * os).sqrt();
        // the other's square face, from the shared top point down to its
        // lower corner, which lies past this end
        let along = (q - p).normalize();
        let sq_down = DVec2::new(-along.y, along.x);
        let sq_down = if sq_down.y > 0.0 { -sq_down } else { sq_down };
        let corner = p + sq_down * ob.depth;
        // where the other's underside meets this one's
        let ob_bottom = |x: f64| p.y + os * (x - p.x) - ohv;
        let x = (p.y - os * p.x - ohv + hv) / (slope - os);
        let meet = DVec2::new(x, slope * x - hv);
        debug_assert!((ob_bottom(x) - meet.y).abs() < 1e-6);
        return Some(vec![t, corner, meet]);
    }
    None
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
    /// The surface of all of them painted, m²: their sides along the
    /// length (the ends left out).
    pub fn paint_area(&self) -> f64 {
        self.section.paint_perimeter() * self.length * self.count() as f64
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

    fn rafter(cuts: [EndCut; 2]) -> (Model, Element) {
        use crate::{Beam, Level, Transaction};
        let lv = Element::new(
            "L",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let l = lv.id;
        let mut m = Model::from_elements([lv]).unwrap();
        // 4 m in plan rising 2 m, 80/200
        let mut b = Beam::new(l, DVec2::ZERO, DVec2::new(4.0, 0.0));
        b.rise = [0.0, 2.0];
        let mut f = Framing::new(Role::Rafter);
        f.section = "80/200".into();
        f.cuts = cuts;
        let mut e = Element::new("r", ElementKind::Beam(b));
        apply(&mut e, f);
        Transaction::new("t")
            .insert(e.clone())
            .apply(&mut m)
            .unwrap();
        let e = m.get(e.id).unwrap().clone();
        (m, e)
    }

    #[test]
    fn the_eaves_cuts_as_george_sketched_them() {
        let s = 0.5f64;
        let hv = 0.2 * (1.0 + s * s).sqrt();
        // plumb: the start's cut straight down from the top
        let (m, e) = rafter([EndCut::Plumb, EndCut::Square]);
        let p = side_profile(&m, &e).unwrap();
        let start: Vec<_> = p.points.iter().rev().take(2).collect();
        assert!(start.iter().all(|q| q.x.abs() < 1e-9));
        assert!(p
            .points
            .iter()
            .any(|q| (q.y + hv).abs() < 1e-9 && q.x.abs() < 1e-9));
        // square: its lower corner square to the axis, inside the end
        let (m, e) = rafter([EndCut::Square, EndCut::Square]);
        let p = side_profile(&m, &e).unwrap();
        let corner = p.points[p.points.len() - 2];
        let d = DVec2::new(1.0, s).normalize();
        assert!(corner.dot(d).abs() < 1e-9 && corner.x > 0.0);
        // plumb and level: down to that corner's height, then in to it
        let (m, e) = rafter([EndCut::PlumbLevel, EndCut::Square]);
        let p = side_profile(&m, &e).unwrap();
        let n = p.points.len();
        let (c, l, t) = (p.points[n - 3], p.points[n - 2], p.points[n - 1]);
        assert!(t.length() < 1e-9 && l.x.abs() < 1e-9 && (l.y - c.y).abs() < 1e-9);
        assert!((c - corner).length() < 1e-9);
    }

    #[test]
    fn a_rafter_fitted_at_the_ridge_takes_in_the_one_running_through() {
        use crate::Transaction;
        let (mut m, e) = rafter([EndCut::Square, EndCut::Square]);
        let ElementKind::Beam(b) = &e.kind else {
            panic!()
        };
        // its pair from the other side, meeting at the ridge
        let mut o = b.clone();
        o.start = DVec2::new(8.0, 0.0);
        o.end = DVec2::new(4.0, 0.0);
        if let Some(f) = &mut o.framing {
            f.cuts = [EndCut::Square, EndCut::Fitted];
        }
        let oe = Element::new("r2", ElementKind::Beam(o));
        Transaction::new("t")
            .insert(oe.clone())
            .apply(&mut m)
            .unwrap();
        let p = side_profile(&m, m.get(oe.id).unwrap()).unwrap();
        // its end: the ridge, the other's square cut's lower corner past
        // the ridge on this side, then under the other's underside back
        // to the ridge's plumb line
        let (t, c, q) = (p.points[0], p.points[1], p.points[2]);
        assert!((t.x - 4.0).abs() < 1e-9);
        assert!(c.x < 4.0 && c.y < t.y);
        assert!((q.x - 4.0).abs() < 1e-6, "{q:?}");
        // the piece to order runs to the ridge on its top edge
        assert!((length(&m, m.get(oe.id).unwrap()) - (4.0f64.powi(2) + 4.0).sqrt()).abs() < 1e-6);
    }
}
