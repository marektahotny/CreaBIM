//! VELOX ring-beam formwork (debnenie vencov): wood-cement boards, with
//! or without EPS, left in place as the formwork, insulation and render
//! base of a ring beam (George 2026-11-27, also for monolithic RC
//! lintels).
//!
//! Every value is from the maker's brochure *Debnenie vencov* (VELOX-WERK
//! s.r.o., Bělotínská 288, Hranice, CZ; www.debnenie-vencov.sk), issue
//! SKBVA5/11.24/4.0, table *Technické vlastnosti vencových dosiek VELOX*:
//!
//! | Board | thickness | insulation | height | length | wall | R |
//! |---|---|---|---|---|---|---|
//! | WSC 35 | 35 | 0 | 250 | 2000 | 200–500 | 0,29 |
//! | WSC-EPS 85 | 85 | 50 | 250 | 2000 | 375–500 | 1,57 |
//! | WSC-EPS 135 | 135 | 100 | 250 | 2000 | 440–500 | 2,86 |
//!
//! and its text: one-sided formwork = 1 board and 1 set of HB clips
//! (5 pcs) for 2 m; two-sided = 2 boards and 1 set of AB + ES clips
//! (10 pcs) for 2 m; the clips as long as the masonry is wide. Its
//! pictures put the EPS against the concrete and the wood-cement board
//! outside (the render base).
//!
//! λ of the layers is worked back from the maker's R (λ = d / R): the
//! wood-cement board 0,035 / 0,29; the EPS of each board from what it adds,
//! (1,57 − 0,29) and (2,86 − 0,29), so each board's R is the maker's.

use crate::{Element, ElementId, ElementKind, Hatch, Material, Model};

/// One VELOX ring-beam board.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Board {
    pub id: &'static str,
    /// As the maker writes it.
    pub name: &'static str,
    /// The wood-cement board, m.
    pub board: f64,
    /// The EPS on it, m (0: none).
    pub eps: f64,
    pub height: f64,
    pub length: f64,
    /// The recommended width of the wall (its masonry), m.
    pub wall: (f64, f64),
    /// Its thermal resistance R, m²K/W.
    pub r: f64,
}

impl Board {
    /// The whole board, m.
    pub fn thickness(&self) -> f64 {
        self.board + self.eps
    }

    /// "WSC-EPS 85".
    pub fn short(&self) -> &'static str {
        self.name.trim_start_matches("Vencová doska so zárezmi ")
    }

    /// Is a wall whose masonry is `core` m wide in its recommended range?
    pub fn fits(&self, core: f64) -> bool {
        core + 1e-4 >= self.wall.0 && core - 1e-4 <= self.wall.1
    }

    /// λ of its EPS, W/(m·K), so that the board's R is the maker's.
    pub fn eps_lambda(&self) -> Option<f64> {
        (self.eps > 0.0).then(|| self.eps / (self.r - WSC.r))
    }
}

/// The wood-cement board alone.
pub const WSC: Board = Board {
    id: "velox-wsc-35",
    name: "Vencová doska so zárezmi WSC 35",
    board: 0.035,
    eps: 0.0,
    height: 0.25,
    length: 2.0,
    wall: (0.2, 0.5),
    r: 0.29,
};

pub const BOARDS: [Board; 3] = [
    WSC,
    Board {
        id: "velox-wsc-eps-85",
        name: "Vencová doska so zárezmi WSC-EPS 85",
        board: 0.035,
        eps: 0.05,
        height: 0.25,
        length: 2.0,
        wall: (0.375, 0.5),
        r: 1.57,
    },
    Board {
        id: "velox-wsc-eps-135",
        name: "Vencová doska so zárezmi WSC-EPS 135",
        board: 0.035,
        eps: 0.1,
        height: 0.25,
        length: 2.0,
        wall: (0.44, 0.5),
        r: 2.86,
    },
];

/// The maker, as lists name it.
pub const MAKER: &str = "VELOX";

/// The board `id`.
pub fn board(id: &str) -> Option<&'static Board> {
    BOARDS.iter().find(|b| b.id == id)
}

/// The board that suits a wall whose masonry is `core` m wide best: the
/// most insulation it is recommended for (WSC 35 when none is).
pub fn best_for(core: f64) -> &'static Board {
    BOARDS
        .iter()
        .rev()
        .find(|b| b.fits(core))
        .unwrap_or(&BOARDS[0])
}

/// Metres of formwork one set of clips does, for both kinds.
pub const CLIP_SET_LENGTH: f64 = 2.0;
/// Pieces in a set of HB clips (one-sided formwork).
pub const HB_SET: usize = 5;
/// Pieces in a set of AB + ES clips (two-sided formwork).
pub const AB_ES_SET: usize = 10;

/// The name of the wood-cement layer's material.
pub const WOOD_CEMENT: &str = "VELOX štiepkocementová doska 35 mm";

/// The name of the EPS layer's material of board `b`.
pub fn eps_name(b: &Board) -> String {
    format!("EPS vo vencovej doske VELOX {}", b.short())
}

fn material(name: &str, category: &str, hatch: Hatch, color: [f32; 3], lambda: f64) -> Element {
    let m = Material {
        hatch,
        color,
        category: category.into(),
        description: format!(
            "{MAKER} – Debnenie vencov (VELOX-WERK s.r.o., SKBVA5/11.24/4.0); λ = d / R z tabuľky výrobcu"
        ),
        roughness: 0.9,
        metallic: 0.0,
        opacity: 1.0,
        pattern: None,
        rate: None,
        lambda: Some(lambda),
        density: None,
        fire: String::new(),
        mu: None,
        sd: None,
    };
    Element::new(name, ElementKind::Material(m))
}

/// The materials of board `b`'s layers, wood-cement board and EPS (none
/// for WSC 35): found in `model` by name, else new ones to insert.
pub fn materials(model: &Model, b: &Board) -> (Option<ElementId>, Option<ElementId>, Vec<Element>) {
    let mut new = vec![];
    let mut find = |name: &str, make: &dyn Fn() -> Element| {
        if let Some(e) = model
            .iter()
            .find(|e| e.name == name && matches!(e.kind, ElementKind::Material(_)))
        {
            return e.id;
        }
        let e = make();
        let id = e.id;
        new.push(e);
        id
    };
    let wood = find(WOOD_CEMENT, &|| {
        material(
            WOOD_CEMENT,
            "wood",
            Hatch::Timber,
            [170.0 / 255.0, 128.0 / 255.0, 88.0 / 255.0],
            WSC.board / WSC.r,
        )
    });
    let eps = b.eps_lambda().map(|l| {
        let name = eps_name(b);
        find(&name, &|| {
            material(
                &name,
                "insulation",
                Hatch::Insulation,
                [240.0 / 255.0, 240.0 / 255.0, 236.0 / 255.0],
                l,
            )
        })
    });
    (Some(wood), eps, new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_layers_give_the_makers_r() {
        for b in BOARDS {
            let r = b.board / (WSC.board / WSC.r) + b.eps_lambda().map_or(0.0, |l| b.eps / l);
            assert!((r - b.r).abs() < 1e-12, "{}", b.name);
            assert!(
                (b.thickness()
                    - [0.035, 0.085, 0.135][BOARDS.iter().position(|x| x == &b).unwrap()])
                .abs()
                    < 1e-12
            );
        }
        assert_eq!(BOARDS[1].short(), "WSC-EPS 85");
    }

    #[test]
    fn the_wall_picks_the_board() {
        assert_eq!(best_for(0.3).id, "velox-wsc-35");
        assert_eq!(best_for(0.375).id, "velox-wsc-eps-85");
        assert_eq!(best_for(0.44).id, "velox-wsc-eps-135");
        assert_eq!(best_for(0.5).id, "velox-wsc-eps-135");
        assert!(!BOARDS[2].fits(0.43) && BOARDS[0].fits(0.2) && !BOARDS[0].fits(0.19));
    }

    /// A ring beam on a 440 mm wall with VELOX WSC-EPS 135: EPS against
    /// the concrete, the wood-cement board outside, the three filling
    /// the core; listed as boards and clips; heat loss counts the wall's
    /// face over it with the boards' R.
    #[test]
    fn a_ring_beam_in_velox_boards() {
        use crate::members::Beam;
        use crate::ops::Transaction;
        use crate::{Document, Level, Wall};
        use glam::DVec2;
        let mut doc = Document::new();
        let lv = Element::new(
            "1NP",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let l = lv.id;
        let mut w = Wall::new(l, DVec2::ZERO, DVec2::new(9.0, 0.0), 0.44, 3.0);
        w.base_offset = 0.0;
        w.bearing = Some(true);
        let we = Element::new("Wall", ElementKind::Wall(w));
        let wall = we.id;
        doc.commit(Transaction::new("t").insert(lv).insert(we))
            .unwrap();
        let p = board("velox-wsc-eps-135").unwrap();
        let (wood, eps, mats) = materials(doc.model(), p);
        let mut tx = Transaction::new("m");
        for m in mats {
            tx = tx.insert(m);
        }
        doc.commit(tx).unwrap();
        let mut ring = crate::masonry::ring_beam(doc.model(), wall).unwrap();
        if let ElementKind::Beam(b) = &mut ring.kind {
            b.wrap = Some(crate::wrap::Wrap {
                insulation: None,
                sides: [0.0; 4],
                plaster: None,
                plaster_thickness: crate::wrap::PLASTER,
                formwork: Some(crate::wrap::Formwork {
                    board: p.id.into(),
                    two_sided: false,
                    ext: 0,
                    wood,
                    eps,
                }),
            });
            *b = crate::masonry::fitted_ring(doc.model(), b).unwrap();
        }
        let rid = ring.id;
        doc.commit(Transaction::new("r").insert(ring)).unwrap();
        let beam = |doc: &Document| match &doc.model().get(rid).unwrap().kind {
            ElementKind::Beam(b) => b.clone(),
            _ => unreachable!(),
        };
        let b: Beam = beam(&doc);
        assert!((b.width - (0.44 - 0.135)).abs() < 1e-9, "{}", b.width);
        // flush with the core's inner face (+y): 0,22 − 0,305 / 2
        let y = (b.start.y + b.end.y) / 2.0;
        assert!((y - (0.22 - 0.305 / 2.0)).abs() < 1e-9, "{y}");
        // two boards on the exterior side (−y): EPS, then the wood-cement
        let pieces: Vec<Beam> = doc
            .model()
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Beam(x) if x.piece.is_some_and(|q| q.host == rid) => Some(x.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(pieces.len(), 2);
        let at = |m: Option<ElementId>| {
            let x = pieces.iter().find(|x| x.material == m).unwrap();
            ((x.start.y + x.end.y) / 2.0, x.width)
        };
        let (ye, te) = at(eps);
        let (yw, tw) = at(wood);
        assert!((te - 0.1).abs() < 1e-9 && (tw - 0.035).abs() < 1e-9);
        assert!((ye - (-0.22 + 0.035 + 0.05)).abs() < 1e-9, "{ye}");
        assert!((yw - (-0.22 + 0.0175)).abs() < 1e-9, "{yw}");
        // across it, from the exterior in: board, EPS, concrete
        let e = doc.model().get(rid).unwrap().clone();
        let layers = crate::wrap::layers_across(doc.model(), &e).unwrap();
        assert_eq!(layers[0], (wood, 0.035));
        assert_eq!(layers[1], (eps, 0.1));
        // the maker's R from the two board layers
        let r: f64 = layers[..2]
            .iter()
            .map(|(m, t)| match &doc.model().get(m.unwrap()).unwrap().kind {
                ElementKind::Material(x) => t / x.lambda.unwrap(),
                _ => unreachable!(),
            })
            .sum();
        assert!((r - 2.86).abs() < 1e-9, "{r}");
        // the take-off: 9,44 m → 5 boards, 5 sets of HB clips for 440 mm
        let lines = crate::takeoff::velox_lines(doc.model());
        assert_eq!(lines.len(), 2, "{lines:#?}");
        assert_eq!(lines[0].quantity.count, 5);
        assert!(lines[0].label.as_ref().unwrap().contains("WSC-EPS 135"));
        assert!(lines[1].label.as_ref().unwrap().contains("HB"));
        assert!(lines[1].label.as_ref().unwrap().contains("440"));
        // no EPS or wood-cement lines of its pieces besides
        let all = crate::takeoff::lines(doc.model());
        assert!(!all
            .iter()
            .any(|x| x.material.is_some() && (x.material == eps || x.material == wood)));
        // heat loss: the wall's face over it, its build-up through it
        let ins = crate::thermal::inserts(doc.model(), wall);
        assert_eq!(ins.len(), 1);
        assert!((ins[0].z.1 - 3.0).abs() < 1e-9 && (ins[0].z.0 - 2.75).abs() < 1e-9);
        let t: f64 = ins[0].assembly.layers.iter().map(|l| l.thickness).sum();
        assert!((t - 0.44).abs() < 1e-9, "{t}");
        // two-sided: a WSC 35 inside too, the concrete narrower
        doc.edit(rid, "two", |e| {
            if let ElementKind::Beam(b) = &mut e.kind {
                b.wrap
                    .as_mut()
                    .unwrap()
                    .formwork
                    .as_mut()
                    .unwrap()
                    .two_sided = true;
            }
        })
        .unwrap();
        let b = beam(&doc);
        assert!(
            (b.width - (0.44 - 0.135 - 0.035)).abs() < 1e-9,
            "{}",
            b.width
        );
        let lines = crate::takeoff::velox_lines(doc.model());
        assert_eq!(lines.len(), 3, "{lines:#?}");
        assert!(lines[2].label.as_ref().unwrap().contains("AB + ES"));
    }
}
