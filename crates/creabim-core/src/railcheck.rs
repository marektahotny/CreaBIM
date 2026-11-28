//! Railings and window sills against STN 74 3305 (George 2026-11-23):
//! where a railing is needed (tab. 1), how high (tab. 2), how wide the
//! gaps in its infill may be (4.4.3, 4.4.5, 4.4.6), and the windows whose
//! sill is lower than a railing would have to be over a fall deeper than
//! tab. 1 allows (3.1.1: the glazing then stands for the railing up to
//! that height).
//!
//! The norm's values are the user's licensed copy, read from a parameter
//! file into the project ([`DEPTH`], [`HEIGHT`], [`GAPS`]); without them
//! nothing is judged.

use crate::fireparams::{number, ParamTable};
use crate::members::{bars_in, Infill, InfillKind, Railing};
use crate::{ElementId, ElementKind, Model, OpeningKind};
use glam::{DVec2, DVec3};

/// Tab. 1: the free space's depth `d` and width `b` over which a railing
/// must be built, mm, by the kind of floor (`key`).
pub const DEPTH: &str = "railing_depth";
/// Tab. 2: the least height `h`, mm, over a depth over `d_over` and up to
/// `d_upto`, mm (either may be empty).
pub const HEIGHT: &str = "railing_height";
/// 4.4.3, 4.4.5, 4.4.6: the widest gaps, mm, by who comes (`key`):
/// between upright bars (`vertical`), between level ones (`horizontal`),
/// between the floor and the infill (`bottom`), and up to which height
/// only a solid or an upright infill may be (`low_zone`).
pub const GAPS: &str = "railing_gaps";
/// 4.5.4, 4.5.5: a handrail's least clear distance from the wall or the
/// railing beside it (`clear`), and the diameter it should have
/// (`grip_min`, `grip_max`), mm, by who comes (`key`: adults, children).
/// Optional: without it handrails are not judged.
pub const HANDRAIL: &str = "railing_handrail";
/// The tables the check reads.
pub const TABLES: [&str; 4] = [DEPTH, HEIGHT, GAPS, HANDRAIL];

/// Who comes onto the floor (3.2.1 b)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Access {
    /// Free access of adults, and single children under their eye: houses,
    /// flats, shops (3.2.1 b) 2)).
    #[default]
    Adults,
    /// Blocks of flats and lodgings (4.4.6): as adults, but low down only
    /// a solid or an upright infill.
    Housing,
    /// Children under 12 in numbers: schools, nurseries (3.2.1 b) 3)).
    Children,
}

impl Access {
    pub const ALL: [Access; 3] = [Access::Adults, Access::Housing, Access::Children];

    pub fn label(self) -> &'static str {
        match self {
            Access::Adults => "Free access of adults (family houses, shops)",
            Access::Housing => "Blocks of flats and lodgings (4.4.6)",
            Access::Children => "Places for children (4.4.5)",
        }
    }

    /// Its row of tab. 1 and of the gaps.
    fn depth_key(self) -> &'static str {
        match self {
            Access::Adults | Access::Housing => "3",
            Access::Children => "4",
        }
    }

    fn gaps_key(self) -> &'static str {
        match self {
            Access::Adults => "adults",
            Access::Housing => "housing",
            Access::Children => "children",
        }
    }
}

/// The norm's figures as read from the tables, m.
#[derive(Clone, Debug, PartialEq)]
pub struct Rules {
    /// Tab. 1: a railing over a fall deeper than this.
    pub depth: f64,
    /// Tab. 2: (least height, over depth, up to depth).
    pub heights: Vec<(f64, Option<f64>, Option<f64>)>,
    pub vertical: f64,
    pub horizontal: f64,
    pub bottom: f64,
    pub low_zone: f64,
    /// The widest plan gap between the floor's edge and a railing fixed
    /// in front of it; none when the table has no such column.
    pub offset: Option<f64>,
    /// The handrail's least clear distance and its diameter's range,
    /// when the handrail table is read.
    pub handrail: Option<(f64, f64, f64)>,
}

fn mm(t: &ParamTable, key: &str, col: &str) -> Option<f64> {
    number(t.cell(key, col)).map(|v| v / 1000.0)
}

impl Rules {
    /// The figures for `access` from `tables`; none when a table is missing.
    pub fn read(tables: &[ParamTable], access: Access) -> Option<Rules> {
        let find = |id: &str| tables.iter().find(|t| t.id == id);
        let (d, h, g) = (find(DEPTH)?, find(HEIGHT)?, find(GAPS)?);
        let (hc, oc, uc) = (h.column("h")?, h.column("d_over")?, h.column("d_upto")?);
        let heights = h
            .rows
            .iter()
            .filter_map(|r| {
                let v = |c: usize| r.get(c).and_then(|x| number(x)).map(|v| v / 1000.0);
                Some((v(hc)?, v(oc), v(uc)))
            })
            .collect::<Vec<_>>();
        if heights.is_empty() {
            return None;
        }
        let k = access.gaps_key();
        Some(Rules {
            depth: mm(d, access.depth_key(), "d")?,
            heights,
            vertical: mm(g, k, "vertical")?,
            horizontal: mm(g, k, "horizontal")?,
            bottom: mm(g, k, "bottom")?,
            low_zone: mm(g, k, "low_zone").unwrap_or(0.0),
            offset: mm(g, k, "offset"),
            handrail: find(HANDRAIL).and_then(|t| {
                let k = match access {
                    Access::Children => "children",
                    _ => "adults",
                };
                Some((
                    mm(t, k, "clear")?,
                    mm(t, k, "grip_min")?,
                    mm(t, k, "grip_max")?,
                ))
            }),
        })
    }

    /// Tab. 2: the least height over a fall `d` deep: the rows bounded by
    /// depth that hold for it (the highest of them), else the unbounded.
    pub fn height_for(&self, d: f64) -> f64 {
        let holds = |&&(_, over, upto): &&(f64, Option<f64>, Option<f64>)| {
            over.is_none_or(|o| d > o + 1e-9) && upto.is_none_or(|u| d <= u + 1e-9)
        };
        let bounded = self
            .heights
            .iter()
            .filter(|r| r.1.is_some() || r.2.is_some())
            .filter(holds)
            .map(|r| r.0)
            .fold(None, |m: Option<f64>, h| Some(m.map_or(h, |m| m.max(h))));
        bounded.unwrap_or_else(|| {
            self.heights
                .iter()
                .filter(|r| r.1.is_none() && r.2.is_none())
                .map(|r| r.0)
                .fold(0.0, f64::max)
        })
    }
}

/// What is wrong, figures in m.
#[derive(Clone, Debug, PartialEq)]
pub enum Fault {
    /// Lower than tab. 2 asks.
    Low { have: f64, need: f64 },
    /// A window's sill lower than a railing would have to be.
    Sill { have: f64, need: f64 },
    /// Upright gaps wider than allowed.
    Upright { have: f64, max: f64 },
    /// Level gaps wider than allowed.
    Level { have: f64, max: f64 },
    /// The infill higher off the floor than allowed.
    Bottom { have: f64, max: f64 },
    /// Level bars where up to `zone` only solid or upright infill may be.
    LowZone { zone: f64 },
    /// Fixed from the side further out from the floor's edge than allowed.
    Offset { have: f64, max: f64 },
    /// A handrail closer to the wall than allowed.
    Clear { have: f64, min: f64 },
    /// A round handrail thinner or thicker than a hand grips.
    Grip { have: f64, min: f64, max: f64 },
}

/// One railing or window judged.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub id: ElementId,
    pub window: bool,
    /// A handrail on a wall: it guards no fall, only its handrail is
    /// judged.
    pub wall: bool,
    /// The fall in front of it, m; none when nothing was found under it.
    pub depth: f64,
    /// Whether tab. 1 asks for a railing there.
    pub needed: bool,
    /// Tab. 2's least height there, m.
    pub need: f64,
    /// Its height (a window's sill over its floor), m.
    pub have: f64,
    pub faults: Vec<Fault>,
}

/// The highest floor under plan point `p` lower than `z`: a slab's top,
/// the terrain, else the ground storey's floor.
pub fn floor_under(model: &Model, p: DVec2, z: f64) -> f64 {
    let mut best: Option<f64> = None;
    let mut take = |v: f64| {
        if v < z - 0.02 {
            best = Some(best.map_or(v, |b: f64| b.max(v)));
        }
    };
    for e in model.iter() {
        match &e.kind {
            ElementKind::Slab(s) => {
                if crate::polygon::contains(&s.outline(), p) {
                    take(s.top(crate::members::elevation(model, s.level)));
                }
            }
            ElementKind::Site(s) => {
                if let crate::site::SiteForm::Terrain { boundary, .. } = &s.form {
                    if boundary.len() < 3 || crate::polygon::contains(boundary, p) {
                        take(crate::members::elevation(model, s.level) + s.ground_at(p));
                    }
                }
            }
            _ => {}
        }
    }
    best.unwrap_or_else(|| ground_floor(model).min(z))
}

/// The ground storey's floor: the lowest storey with walls at or over
/// −0,800 (as the building's figures take it), else ±0,000.
fn ground_floor(model: &Model) -> f64 {
    crate::levels::sorted(model)
        .into_iter()
        .filter(|(id, ..)| {
            model
                .iter()
                .any(|e| matches!(&e.kind, ElementKind::Wall(w) if w.level == *id))
        })
        .map(|(.., l)| l.elevation)
        .find(|z| *z >= -0.8)
        .unwrap_or(0.0)
}

/// The fall beside a railing: from its floor to the lowest of what lies
/// under either side of it; a stair's railing over the stair's whole
/// rise (4.2.2 b)).
pub fn railing_depth(model: &Model, r: &Railing) -> f64 {
    if let Some(side) = r.stair {
        if let Some(ElementKind::Stair(s)) = model.get(side.stair).map(|e| &e.kind) {
            return s.rise(model);
        }
    }
    let path = r.path_in(model);
    let mut d: f64 = 0.0;
    for w in path.windows(2) {
        let (a, b) = (w[0].truncate(), w[1].truncate());
        let Some(dir) = (b - a).try_normalize() else {
            continue;
        };
        let mid = (w[0] + w[1]) / 2.0;
        for s in [-1.0, 1.0] {
            let p = mid.truncate() + dir.perp() * (0.3 * s);
            d = d.max(mid.z - floor_under(model, p, mid.z));
        }
    }
    d
}

/// A railing's gaps as built: (upright, level, from the floor), m.
pub fn railing_gaps(r: &Railing, model: &Model) -> (f64, f64, f64) {
    let post = r.post();
    let posts = r.posts(model);
    let widest_bay = posts
        .windows(2)
        .map(|w| w[0].truncate().distance(w[1].truncate()) - post)
        .fold(0.0, f64::max);
    let top_under = r.height - r.rail();
    match r.infill {
        None => {
            // the bottom rail 100 mm up, nothing between it and the top
            let low_top = 0.1 + post / 2.0;
            (widest_bay, top_under - low_top, 0.1 - post / 2.0)
        }
        Some(Infill {
            kind,
            gap,
            bar,
            bottom,
            ..
        }) => {
            let span = top_under - (bottom + post);
            match kind {
                InfillKind::Vertical => (bars_in(widest_bay, gap, bar).1, 0.0, bottom),
                // level bars run past the posts: their slits are level,
                // none between the posts upright (4.4.3 a), b))
                InfillKind::Horizontal => (0.0, bars_in(span, gap, bar).1, bottom),
                // panes clear of the posts, the rails over and under them
                InfillKind::Panel => (crate::members::GLASS_GAP, crate::members::GLASS_GAP, bottom),
            }
        }
    }
}

fn judge_railing(model: &Model, id: ElementId, r: &Railing, rules: &Rules) -> Finding {
    let depth = railing_depth(model, r);
    let need = rules.height_for(depth);
    let mut faults = vec![];
    if r.height + 1e-6 < need {
        faults.push(Fault::Low {
            have: r.height,
            need,
        });
    }
    let (up, level, bottom) = railing_gaps(r, model);
    if up > rules.vertical + 1e-6 {
        faults.push(Fault::Upright {
            have: up,
            max: rules.vertical,
        });
    }
    if level > rules.horizontal + 1e-6 {
        faults.push(Fault::Level {
            have: level,
            max: rules.horizontal,
        });
    }
    if bottom > rules.bottom + 1e-6 {
        faults.push(Fault::Bottom {
            have: bottom,
            max: rules.bottom,
        });
    }
    let level_bars = matches!(
        r.infill.map(|f| f.kind),
        None | Some(InfillKind::Horizontal)
    );
    if rules.low_zone > 0.0 && level_bars {
        faults.push(Fault::LowZone {
            zone: rules.low_zone,
        });
    }
    if let (crate::members::Fixing::Side, Some(max)) = (r.fixing, rules.offset) {
        let have = r.gap_to_edge();
        if have > max + 1e-6 {
            faults.push(Fault::Offset { have, max });
        }
    }
    faults.extend(handrail_faults(r, rules));
    let needed = depth > rules.depth + 1e-6;
    Finding {
        id,
        window: false,
        wall: false,
        depth,
        needed,
        need,
        have: r.height,
        faults: if needed { faults } else { vec![] },
    }
}

/// A handrail against 4.5.4 and 4.5.5: clear of its wall, and a round
/// one of a hand's grip (a square one is not judged).
fn handrail_faults(r: &Railing, rules: &Rules) -> Vec<Fault> {
    let Some((clear, gmin, gmax)) = rules.handrail else {
        return vec![];
    };
    let mut out = vec![];
    if let Some(w) = r.wall {
        if w.clear + 1e-6 < clear {
            out.push(Fault::Clear {
                have: w.clear,
                min: clear,
            });
        }
    }
    let d = r.rail();
    if r.round && r.rail_profile.is_none() && (d + 1e-6 < gmin || d > gmax + 1e-6) {
        out.push(Fault::Grip {
            have: d,
            min: gmin,
            max: gmax,
        });
    }
    out
}

/// A window's (a door's) fall: from its storey's floor to what lies
/// 0,5 m out in front of its outer face.
fn window_depth(model: &Model, o: &crate::Opening) -> Option<(f64, f64)> {
    let Some(ElementKind::Wall(w)) = model.get(o.host).map(|e| &e.kind) else {
        return None;
    };
    let len = w.length();
    let dir = (w.end - w.start).try_normalize()?;
    let mid = w.start + dir * (o.offset + o.width / 2.0).min(len);
    let out = if o.faces_left(w) {
        dir.perp()
    } else {
        -dir.perp()
    };
    let p = mid + out * (w.thickness / 2.0 + 0.5);
    let floor = crate::members::elevation(model, w.level);
    Some((
        floor - floor_under(model, p, floor),
        o.sill_from_floor(model),
    ))
}

/// Every railing and every window (and glazed door) of `model` judged by
/// `rules`: railings always, windows where tab. 1 asks for a railing.
pub fn check(model: &Model, rules: &Rules) -> Vec<Finding> {
    let mut out = vec![];
    for e in model.iter() {
        match &e.kind {
            // a handrail on a wall guards no fall: only its handrail
            ElementKind::Railing(r) if r.wall.is_some() => out.push(Finding {
                id: e.id,
                window: false,
                wall: true,
                depth: 0.0,
                needed: true,
                need: 0.0,
                have: r.height,
                faults: handrail_faults(r, rules),
            }),
            ElementKind::Railing(r) => out.push(judge_railing(model, e.id, r, rules)),
            ElementKind::Opening(o)
                if matches!(o.kind, OpeningKind::Window | OpeningKind::Door) =>
            {
                let Some((depth, sill)) = window_depth(model, o) else {
                    continue;
                };
                if depth <= rules.depth + 1e-6 {
                    continue;
                }
                let need = rules.height_for(depth);
                let faults = if sill + 1e-6 < need {
                    vec![Fault::Sill { have: sill, need }]
                } else {
                    vec![]
                };
                out.push(Finding {
                    id: e.id,
                    window: true,
                    wall: false,
                    depth,
                    needed: true,
                    need,
                    have: sill,
                    faults,
                });
            }
            _ => {}
        }
    }
    out
}

/// A plan point of a railing to show it by.
pub fn railing_point(model: &Model, r: &Railing) -> Option<DVec3> {
    r.path_in(model).first().copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fireparams::parse;
    use crate::{Element, Level, Transaction, Wall};
    use glam::DVec2;

    /// Tables shaped as the user's parameter file (the norm's figures are
    /// licensed and stay out of the repository: these are made up).
    fn tables() -> Vec<ParamTable> {
        parse(
            "#table railing_depth\n#columns key\td\tb\n3\t400\t100\n4\t250\t50\n\
             #table railing_height\n#columns key\th\td_over\td_upto\n1\t850\t\t3500\n2\t950\t\t\n3\t1050\t10000\t\n4\t1150\t25000\t\n\
             #table railing_gaps\n#columns key\tvertical\thorizontal\tbottom\toffset\tlow_zone\n\
             adults\t115\t170\t110\t40\t0\nhousing\t115\t170\t110\t40\t550\nchildren\t75\t110\t70\t20\t550\n\
             #table railing_handrail\n#columns key\tclear\tgrip_min\tgrip_max\nadults\t55\t38\t48\nchildren\t55\t28\t38\n",
        )
    }

    #[test]
    fn the_least_height_follows_the_falls_depth() {
        let r = Rules::read(&tables(), Access::Adults).unwrap();
        assert_eq!(r.height_for(2.9), 0.85);
        assert_eq!(r.height_for(3.5), 0.85);
        assert_eq!(r.height_for(5.0), 0.95);
        assert_eq!(r.height_for(15.0), 1.05);
        assert_eq!(r.height_for(40.0), 1.15);
        assert!(Rules::read(&[], Access::Adults).is_none());
    }

    #[test]
    fn a_gallery_railing_of_rails_only_fails_and_balusters_pass() {
        let mut m = Model::new();
        let up = Element::new(
            "2. NP",
            ElementKind::Level(Level {
                elevation: 3.0,
                height: 3.0,
            }),
        );
        let ground = Element::new(
            "1. NP",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let gl = ground.id;
        let wall = Element::new(
            "W",
            ElementKind::Wall(Wall::new(gl, DVec2::ZERO, DVec2::new(1.0, 0.0), 0.3, 3.0)),
        );
        let path = vec![DVec3::ZERO, DVec3::new(3.0, 0.0, 0.0)];
        let mut r = Railing::new(up.id, path);
        r.height = 0.95;
        let re = Element::new("R", ElementKind::Railing(r.clone()));
        let rid = re.id;
        Transaction::new("t")
            .insert(up)
            .insert(ground)
            .insert(wall)
            .insert(re)
            .apply(&mut m)
            .unwrap();
        let rules = Rules::read(&tables(), Access::Adults).unwrap();
        let f = &check(&m, &rules)[0];
        assert!(f.needed && (f.depth - 3.0).abs() < 1e-9);
        assert_eq!(f.need, 0.85);
        assert!(f.faults.iter().any(|x| matches!(x, Fault::Upright { .. })));
        assert!(f.faults.iter().any(|x| matches!(x, Fault::Level { .. })));
        assert!(!f.faults.iter().any(|x| matches!(x, Fault::Low { .. })));
        // balusters 100 apart, 80 off the floor
        r.infill = Some(Infill {
            gap: 0.1,
            ..Infill::default()
        });
        let before = m.get(rid).unwrap().clone();
        let mut after = before.clone();
        after.kind = ElementKind::Railing(r);
        Transaction::new("i")
            .replace(before, after)
            .apply(&mut m)
            .unwrap();
        let f = &check(&m, &rules)[0];
        assert!(f.faults.is_empty(), "{:?}", f.faults);
        // in a block of flats level bars may not be low
        let flats = Rules::read(&tables(), Access::Housing).unwrap();
        assert!(check(&m, &flats)[0].faults.is_empty());
    }

    #[test]
    fn an_upstairs_window_with_a_low_sill_needs_a_railing() {
        let mut m = Model::new();
        let ground = Element::new(
            "1. NP",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let up = Element::new(
            "2. NP",
            ElementKind::Level(Level {
                elevation: 3.0,
                height: 3.0,
            }),
        );
        let w0 = Element::new(
            "W0",
            ElementKind::Wall(Wall::new(
                ground.id,
                DVec2::ZERO,
                DVec2::new(6.0, 0.0),
                0.3,
                3.0,
            )),
        );
        let w1 = Element::new(
            "W1",
            ElementKind::Wall(Wall::new(
                up.id,
                DVec2::ZERO,
                DVec2::new(6.0, 0.0),
                0.3,
                3.0,
            )),
        );
        let low = crate::Opening {
            host: w1.id,
            kind: OpeningKind::Window,
            offset: 1.0,
            sill: 0.5,
            width: 1.2,
            height: 1.5,
            material: None,
            frame: Default::default(),
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
        };
        let mut high = low.clone();
        high.offset = 4.0;
        high.sill = 0.85;
        let mut down = low.clone();
        down.host = w0.id;
        let lo = Element::new("low", ElementKind::Opening(low));
        let lid = lo.id;
        Transaction::new("t")
            .insert(ground)
            .insert(up)
            .insert(w0)
            .insert(w1)
            .insert(lo)
            .insert(Element::new("high", ElementKind::Opening(high)))
            .insert(Element::new("down", ElementKind::Opening(down)))
            .apply(&mut m)
            .unwrap();
        let rules = Rules::read(&tables(), Access::Adults).unwrap();
        let found = check(&m, &rules);
        // the ground floor's window has no fall in front of it
        assert_eq!(found.len(), 2);
        let bad: Vec<_> = found.iter().filter(|f| !f.faults.is_empty()).collect();
        assert_eq!(bad.len(), 1);
        assert_eq!(bad[0].id, lid);
        assert!(matches!(bad[0].faults[0], Fault::Sill { need, .. } if need == 0.85));
    }

    #[test]
    fn a_wall_handrail_is_judged_by_its_clearance_and_grip() {
        use crate::members::{RailMetal, RailingType};
        let mut m = Model::new();
        let l = Element::new(
            "1. NP",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let mut r = Railing::new(l.id, vec![DVec3::ZERO, DVec3::new(3.0, 0.0, 0.0)]);
        RailingType::WallHandrail.apply(&mut r, RailMetal::Aluminium, true);
        // the aluminium Ø50 handrail is thicker than 48, too close at 40
        r.wall.as_mut().unwrap().clear = 0.04;
        assert!(r.posts(&m).is_empty());
        let re = Element::new("R", ElementKind::Railing(r));
        Transaction::new("t")
            .insert(l)
            .insert(re)
            .apply(&mut m)
            .unwrap();
        let rules = Rules::read(&tables(), Access::Adults).unwrap();
        let f = &check(&m, &rules)[0];
        assert!(f.wall);
        assert!(f.faults.iter().any(|x| matches!(x, Fault::Clear { .. })));
        assert!(f.faults.iter().any(|x| matches!(x, Fault::Grip { .. })));
    }

    #[test]
    fn fixed_from_the_side_it_stands_out_past_the_edge() {
        use crate::members::{Fixing, RailMetal, RailingType};
        let mut m = Model::new();
        let up = Element::new(
            "2. NP",
            ElementKind::Level(Level {
                elevation: 3.0,
                height: 3.0,
            }),
        );
        let slab = crate::Slab::new(
            up.id,
            vec![
                DVec2::new(0.0, 0.0),
                DVec2::new(4.0, 0.0),
                DVec2::new(4.0, 2.0),
                DVec2::new(0.0, 2.0),
            ],
            0.2,
        );
        let ground = Element::new(
            "1. NP",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let mut r = Railing::new(up.id, vec![DVec3::ZERO, DVec3::new(4.0, 0.0, 0.0)]);
        RailingType::UprightBars.apply(&mut r, RailMetal::Stainless, true);
        r.fixing = Fixing::Side;
        r.side_gap = Some(0.07);
        let rid = Element::new("R", ElementKind::Railing(r.clone()));
        let id = rid.id;
        Transaction::new("t")
            .insert(up)
            .insert(ground)
            .insert(Element::new("S", ElementKind::Slab(slab)))
            .insert(rid)
            .apply(&mut m)
            .unwrap();
        // out is -y, the side with no slab: the posts' axis past the gap
        let axis = r.axis(&m);
        assert!((axis[0].y + 0.07 + 0.0424 / 2.0).abs() < 1e-9);
        assert!((r.drop() - crate::members::SIDE_DROP).abs() < 1e-12);
        let rules = Rules::read(&tables(), Access::Adults).unwrap();
        let f = check(&m, &rules).into_iter().find(|f| f.id == id).unwrap();
        assert!(f
            .faults
            .iter()
            .any(|x| matches!(x, Fault::Offset { max, .. } if (*max - 0.04).abs() < 1e-9)));
    }

    #[test]
    fn older_railings_load_as_they_were() {
        let r: Railing = serde_json::from_str(
            r#"{"level":"00000000-0000-0000-0000-000000000000","path":[[0,0,0],[1,0,0]],"height":1.0,"post_spacing":1.2,"material":null}"#,
        )
        .unwrap();
        assert!(r.metal.is_none() && !r.round && r.wall.is_none());
        assert_eq!(r.fixing, crate::members::Fixing::Top);
        assert_eq!(r.post(), crate::members::POST_SIZE);
    }
}
