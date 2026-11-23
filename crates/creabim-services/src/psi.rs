//! Linear thermal bridges for the detailed heat loss by annex C of
//! STN EN 12831-1 (Σ l·Ψ, formula C.1) with the default values Ψ of STN EN
//! ISO 14683 annex C, table C.2 ([`crate::heatloss::Opts::psi`]).
//!
//! The junctions are found in the model: the corners of the outside walls
//! (out and in), an inner wall meeting an outside wall, the floors between
//! storeys, the roof and the floor on the ground along the outside walls,
//! and the edge of every window and door. Each takes the detail of table
//! C.2 that matches where the outside wall's main insulation lies (the
//! layer of the largest thermal resistance: outside, in the core, inside,
//! or a wall with none, as of light or aerated blocks), and for a window
//! where its frame stands; the designer may pick another detail for each
//! kind of junction. The values Ψ come from the norm's parameter file
//! (table [`PSI`]), never from the code; the detailed method measures from
//! the outside, so it takes Ψe.

use creabim_core::assembly::{self, LayerFunction};
use creabim_core::fireparams::{self, ParamTable};
use creabim_core::{Element, ElementKind, Model, Opening, Wall};

/// Table C.2: columns `key` (R1, C1, IF1 …), `group` (R, B, C, IF, IW,
/// GF, P, W), `psi_e`, `psi_oi`, `psi_i` (W/(m·K)), `text`.
pub const PSI: &str = "heat_psi";

/// Where a wall's main insulation lies (STN EN ISO 14683, annex C: a) to
/// d)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ins {
    Outside,
    Core,
    Inside,
    /// None apart: aerated or light blocks, a timber frame.
    Even,
}

impl Ins {
    fn n(self) -> usize {
        match self {
            Ins::Outside => 1,
            Ins::Core => 2,
            Ins::Inside => 3,
            Ins::Even => 4,
        }
    }
}

/// The kinds of junction the model gives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Joint {
    CornerOut,
    CornerIn,
    Floor,
    Roof,
    Ground,
    InnerWall,
    Window,
}

impl Joint {
    pub const ALL: [Joint; 7] = [
        Joint::CornerOut,
        Joint::CornerIn,
        Joint::InnerWall,
        Joint::Floor,
        Joint::Roof,
        Joint::Ground,
        Joint::Window,
    ];

    /// In English: the app translates it.
    pub fn name(self) -> &'static str {
        match self {
            Joint::CornerOut => "Outside corner",
            Joint::CornerIn => "Inside corner",
            Joint::Floor => "Floor between storeys",
            Joint::Roof => "Roof at the wall",
            Joint::Ground => "Floor on the ground at the wall",
            Joint::InnerWall => "Inner wall at the outside wall",
            Joint::Window => "Window and door edges",
        }
    }

    /// The group of table C.2 its details are in.
    pub fn group(self) -> &'static str {
        match self {
            Joint::CornerOut | Joint::CornerIn => "C",
            Joint::Floor => "IF",
            Joint::Roof => "R",
            Joint::Ground => "GF",
            Joint::InnerWall => "IW",
            Joint::Window => "W",
        }
    }

    fn index(self) -> usize {
        Joint::ALL.iter().position(|x| *x == self).unwrap_or(0)
    }
}

/// Where a window's frame stands across the wall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Frame {
    /// Out in the outside insulation.
    InInsulation,
    Outer,
    Middle,
    Inner,
}

/// A row of table C.2.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub key: String,
    pub group: String,
    pub psi_e: f64,
    pub text: String,
}

/// The rows of table C.2 read.
pub fn rows(tables: &[ParamTable]) -> Vec<Row> {
    let Some(t) = tables.iter().find(|t| t.id == PSI) else {
        return vec![];
    };
    let (Some(k), Some(g), Some(e), Some(x)) = (
        t.column("key"),
        t.column("group"),
        t.column("psi_e"),
        t.column("text"),
    ) else {
        return vec![];
    };
    t.rows
        .iter()
        .filter_map(|r| {
            Some(Row {
                key: r.get(k)?.clone(),
                group: r.get(g)?.clone(),
                psi_e: fireparams::number(r.get(e)?)?,
                text: r.get(x).cloned().unwrap_or_default(),
            })
        })
        .collect()
}

/// Where the main insulation of wall `e` lies: its layer of the largest
/// thermal resistance among those marked insulation, by the layers on
/// either side of it (only finishes and membranes outside: outside; only
/// those inside: inside; else in the core). A wall with no insulation
/// layer counts as even.
pub fn insulation(model: &Model, e: &Element) -> Ins {
    let Some((_, a)) = assembly::assembly_of(model, e) else {
        return Ins::Even;
    };
    let r = |l: &assembly::Layer| -> f64 {
        l.material
            .and_then(|m| model.get(m))
            .and_then(|m| match &m.kind {
                ElementKind::Material(x) => x.lambda,
                _ => None,
            })
            .filter(|x| *x > 0.0)
            .map_or(0.0, |lambda| l.thickness / lambda)
    };
    let Some(i) = a
        .layers
        .iter()
        .enumerate()
        .filter(|(_, l)| l.function == LayerFunction::Insulation && l.thickness > 0.0)
        .max_by(|x, y| r(x.1).total_cmp(&r(y.1)))
        .map(|x| x.0)
    else {
        return Ins::Even;
    };
    let skin = |l: &assembly::Layer| {
        matches!(
            l.function,
            LayerFunction::Finish | LayerFunction::Membrane | LayerFunction::AirGap
        ) || l.thickness <= 0.0
    };
    // the layers run from the exterior face inwards
    if a.layers[..i].iter().all(skin) {
        Ins::Outside
    } else if a.layers[i + 1..].iter().all(skin) {
        Ins::Inside
    } else {
        Ins::Core
    }
}

/// Where the frame of window `op` in wall `we` stands: its setback from
/// the core's outer edge against the room the core leaves it.
pub fn frame_of(model: &Model, we: &Element, wall: &Wall, op: &Opening) -> Frame {
    let a = assembly::assembly_of(model, we).map(|x| x.1);
    let sb = op.setback_in(wall, a);
    if sb < -0.005 {
        return Frame::InInsulation;
    }
    let t = wall.thickness / 2.0;
    let core = a
        .filter(|a| a.layers.len() >= 2)
        .map_or((-t, t), |a| assembly::wall_core(a, wall));
    let room = (core.1 - core.0 - op.frame.depth).max(1e-3);
    let p = sb / room;
    if p < 1.0 / 3.0 {
        Frame::Outer
    } else if p < 2.0 / 3.0 {
        Frame::Middle
    } else {
        Frame::Inner
    }
}

/// The detail of table C.2 a junction takes left to itself.
pub fn auto_key(j: Joint, ins: Ins, frame: Frame, lapped: bool) -> String {
    let n = ins.n();
    match j {
        Joint::CornerOut => format!("C{n}"),
        Joint::CornerIn => format!("C{}", n + 4),
        Joint::Floor => format!("IF{n}"),
        Joint::Roof => format!("R{n}"),
        Joint::Ground => format!("GF{n}"),
        Joint::InnerWall => format!("IW{n}"),
        Joint::Window => {
            // W1–W6 the frame at the outer face, W7–W12 in the middle,
            // W13–W18 at the inner face; in each: 1 insulated outside, 3
            // inside, 4 light, 5 insulated in the core, 6 the outside
            // insulation lapping the frame
            let base = match frame {
                Frame::InInsulation | Frame::Outer => 0,
                Frame::Middle => 6,
                Frame::Inner => 12,
            };
            let row = match ins {
                Ins::Outside if frame == Frame::InInsulation => 1,
                Ins::Outside if lapped => 6,
                Ins::Outside => 1,
                Ins::Core => 5,
                Ins::Inside => 3,
                Ins::Even => 4,
            };
            format!("W{}", base + row)
        }
    }
}

/// What the junctions take: the table's rows and the detail picked for
/// each kind (an index into the rows; none: by the build-up).
pub struct Picks<'a> {
    pub rows: &'a [Row],
    pub pick: [Option<u16>; 7],
}

impl Picks<'_> {
    /// The row for junction `j` whose own detail is `auto`.
    pub fn row(&self, j: Joint, auto: &str) -> Option<&Row> {
        match self.pick[j.index()] {
            Some(i) => self.rows.get(i as usize).filter(|r| r.group == j.group()),
            None => self.rows.iter().find(|r| r.key == auto),
        }
    }
}

/// The slot of junction `j` in [`crate::heatloss::Opts::psi_pick`].
pub fn slot(j: Joint) -> usize {
    j.index()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn details_by_where_the_insulation_lies() {
        let f = Frame::Outer;
        assert_eq!(auto_key(Joint::CornerOut, Ins::Outside, f, false), "C1");
        assert_eq!(auto_key(Joint::CornerIn, Ins::Inside, f, false), "C7");
        assert_eq!(auto_key(Joint::Floor, Ins::Core, f, false), "IF2");
        assert_eq!(auto_key(Joint::Ground, Ins::Even, f, false), "GF4");
        assert_eq!(
            auto_key(Joint::Window, Ins::Outside, Frame::InInsulation, true),
            "W1"
        );
        assert_eq!(
            auto_key(Joint::Window, Ins::Outside, Frame::Outer, true),
            "W6"
        );
        assert_eq!(
            auto_key(Joint::Window, Ins::Outside, Frame::Middle, false),
            "W7"
        );
        assert_eq!(
            auto_key(Joint::Window, Ins::Core, Frame::Inner, false),
            "W17"
        );
        assert_eq!(
            auto_key(Joint::Window, Ins::Even, Frame::Middle, false),
            "W10"
        );
    }

    #[test]
    fn rows_by_key_and_picks_within_the_group() {
        // made-up numbers, not the norm's
        let t = fireparams::parse(
            "#table heat_psi\n#columns key\tgroup\tpsi_e\tpsi_oi\tpsi_i\ttext\n\
C1\tC\t-0,5\t0\t0\troh\nC5\tC\t0,5\t0\t0\tvnútorný\nW1\tW\t0,25\t0\t0\tokno\n",
        );
        let rows = rows(&t);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].psi_e, -0.5);
        let mut p = Picks {
            rows: &rows,
            pick: [None; 7],
        };
        assert_eq!(p.row(Joint::CornerOut, "C1").unwrap().key, "C1");
        p.pick[slot(Joint::CornerOut)] = Some(1);
        assert_eq!(p.row(Joint::CornerOut, "C1").unwrap().key, "C5");
        // a pick from another group is not taken
        p.pick[slot(Joint::CornerOut)] = Some(2);
        assert!(p.row(Joint::CornerOut, "C1").is_none());
        assert!(p.row(Joint::Roof, "R1").is_none());
    }
}
