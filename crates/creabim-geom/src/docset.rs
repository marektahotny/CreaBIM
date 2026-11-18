//! The drawings of a building as the documentation of a construction is
//! set out by the decree 60/2025 Z. z. (to the Building Act 25/2025
//! Z. z.): C the site drawings, D the building `S01` by profession (ASR
//! the architecture, ZTI plumbing, VYK heating), each drawing numbered
//! and named as the decree's annex 24 names its files
//! (`S01_ASR_001_N01`: the building, the profession, the drawing's number
//! and its kind: N01 the plan of the first storey above ground, P01 of
//! the first below, VYZ foundations, VYS the roof, REZ sections, POH
//! elevations, VVZ lists). One call makes the views still missing (a
//! plan of every storey, two sections, four elevations, the site and
//! roof plans, the lists of windows and doors, the services plans and
//! their legend) and a sheet for every drawing, sized to it.

use crate::section::view_drawing;
use creabim_core::viewmodes::DisplayMode;
use creabim_core::words::tr;
use creabim_core::{
    fixture, levels, Element, ElementId, ElementKind, Model, OpeningKind, ScheduleCategory, Sheet,
    Transaction, View, ViewKind,
};
use glam::DVec2;

/// What [`drawing_set`] made.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Made {
    pub views: usize,
    pub sheets: usize,
}

/// The part of the documentation a sheet numbered `number` belongs to,
/// to list it under: `C00`, or the profession of `S01_ASR_...`.
pub fn part_of(number: &str) -> Option<&'static str> {
    const PARTS: [(&str, &str); 8] = [
        ("C00_", "C Site drawings"),
        ("_ASR_", "D.1 Architecture (ASR)"),
        ("_STA_", "D.2 Structure (STA)"),
        ("_ZTI_", "D.3 Plumbing (ZTI)"),
        ("_VYK_", "D.4 Heating (VYK)"),
        ("_ELI_", "D.5 Electrical (ELI)"),
        ("_VZT_", "D.6 Ventilation (VZT)"),
        ("_PLY_", "D.7 Gas (PLY)"),
    ];
    PARTS
        .iter()
        .find(|(k, _)| number.starts_with(k) || number.contains(k))
        .map(|p| p.1)
}

struct Builder {
    /// The model with what has been made so far.
    m: Model,
    tx: Transaction,
    made: Made,
}

impl Builder {
    fn add(&mut self, e: Element) -> ElementId {
        let id = e.id;
        let one = Transaction::new("one").insert(e.clone());
        if one.apply(&mut self.m).is_ok() {
            let tx = std::mem::replace(&mut self.tx, Transaction::new(""));
            self.tx = tx.insert(e);
        }
        id
    }

    fn view(&mut self, name: String, v: View) -> ElementId {
        self.made.views += 1;
        self.add(Element::new(name, ElementKind::View(v)))
    }

    fn find(&self, pick: impl Fn(&Element, &View) -> bool) -> Option<ElementId> {
        self.m.iter().find_map(|e| match &e.kind {
            ElementKind::View(v) if v.callout.is_none() && pick(e, v) => Some(e.id),
            _ => None,
        })
    }

    /// A view's size on paper, mm.
    fn size(&self, view: ElementId) -> DVec2 {
        let scale = match self.m.get(view).map(|e| &e.kind) {
            Some(ElementKind::View(v)) => v.paper_scale(),
            _ => 100.0,
        };
        view_drawing(&self.m, view)
            .and_then(|d| d.bounds())
            .map_or(DVec2::new(100.0, 100.0), |(lo, hi)| {
                (hi - lo) * 1000.0 / scale
            })
    }

    /// A sheet `number` called `name` with `views` on it, unless there
    /// is one with that number: the smallest A size they fit on beside
    /// one another (two to a row), over the title block.
    fn sheet(&mut self, number: String, name: String, views: &[ElementId]) {
        if views.is_empty()
            || self
                .m
                .iter()
                .any(|e| matches!(&e.kind, ElementKind::Sheet(s) if s.number == number))
        {
            return;
        }
        let sizes: Vec<DVec2> = views.iter().map(|v| self.size(*v)).collect();
        // two drawings to a row
        let (mut w, mut h) = (0.0, 0.0);
        for row in sizes.chunks(2) {
            w = row.iter().map(|s| s.x + 20.0).sum::<f64>().max(w);
            h += row.iter().map(|s| s.y).fold(0.0, f64::max) + 25.0;
        }
        let (need_w, need_h) = (w + 40.0, h + 75.0);
        let size = Sheet::SIZES
            .iter()
            .rev()
            .find(|(_, sw, sh)| *sw >= need_w && *sh >= need_h)
            .map_or("A0", |s| s.0);
        let sheet = self.add(Element::new(
            name,
            ElementKind::Sheet(Sheet::a(number, size)),
        ));
        self.made.sheets += 1;
        for v in views {
            let at = crate::sheet::free_spot(&self.m, sheet, *v);
            let vp = crate::sheet::new_viewport(&self.m, sheet, *v, at);
            self.add(Element::new("Viewport", ElementKind::Viewport(vp)));
        }
    }
}

/// The views and sheets still missing from the drawing set of `model`;
/// none when it has no walls yet or nothing is missing.
pub fn drawing_set(model: &Model) -> Option<(Transaction, Made)> {
    let walls: Vec<&creabim_core::Wall> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Wall(w) => Some(w),
            _ => None,
        })
        .collect();
    let first = walls.first()?.start;
    let (lo, hi) = walls
        .iter()
        .flat_map(|w| [w.start, w.end])
        .fold((first, first), |(lo, hi), p| (lo.min(p), hi.max(p)));
    let mid = (lo + hi) / 2.0;
    let mut b = Builder {
        m: model.clone(),
        tx: Transaction::new("Drawing set"),
        made: Made::default(),
    };
    // the storeys: those with walls of their own, from the lowest up
    let storeys: Vec<(ElementId, String, f64)> = levels::sorted(model)
        .into_iter()
        .filter(|(id, ..)| walls.iter().any(|w| w.level == *id))
        .map(|(id, name, l)| (id, name, l.elevation))
        .collect();
    let (bottom, top) = (storeys.first()?.clone(), storeys.last()?.clone());
    let plain = |v: &View| {
        !v.ceiling_plan
            && !v.graphics.range.from_above
            && DisplayMode::of(v) == DisplayMode::Architecture
    };
    let plan_of = |b: &mut Builder, level: ElementId, name: &str, z: f64| {
        b.find(|_, v| matches!(v.kind, ViewKind::Plan { level: l, .. } if l == level) && plain(v))
            .unwrap_or_else(|| {
                let v = levels::plan_of(&b.m, level, z);
                b.view(format!("{name} {}", tr("plan")), v)
            })
    };
    let schedule = |b: &mut Builder, c: ScheduleCategory, name: &str| {
        b.find(|_, v| matches!(v.kind, ViewKind::Schedule { category, .. } if category == c))
            .unwrap_or_else(|| b.view(tr(name), View::schedule(c)))
    };
    let mut n = 0;
    let number = |part: &str, kind: &str, n: &mut usize| {
        *n += 1;
        format!("{part}_{:03}_{kind}", *n)
    };

    // C: the site, seen from above
    let site = b
        .find(|_, v| {
            matches!(v.kind, ViewKind::Plan { level, .. } if level == bottom.0)
                && v.graphics.range.from_above
                && v.scale > 150.0
        })
        .unwrap_or_else(|| {
            let mut v = View::plan(bottom.0);
            v.graphics.range.from_above = true;
            v.scale = 200.0;
            b.view(tr("Site plan"), v)
        });
    b.sheet("C00_SIT_001".into(), tr("Site plan"), &[site]);

    // D S01 ASR: foundations, a plan a storey, the roof, sections,
    // elevations, the lists of windows and doors
    let asr = "S01_ASR";
    let footing_level = model.iter().find_map(|e| match &e.kind {
        ElementKind::Footing(f) => Some(f.plan_level(model)),
        _ => None,
    });
    if let Some((id, name, l)) = footing_level.and_then(|id| {
        levels::sorted(model)
            .into_iter()
            .find(|l| l.0 == id && l.0 != bottom.0)
    }) {
        let v = plan_of(&mut b, id, &name, l.elevation);
        b.sheet(number(asr, "VYZ", &mut n), tr("Foundations"), &[v]);
    }
    let (mut above, mut below) = (0, storeys.iter().filter(|s| s.2 < -1.0).count() + 1);
    // each storey's code and its plan
    let mut plans = vec![];
    for (id, name, z) in &storeys {
        let code = if *z < -1.0 {
            below -= 1;
            format!("P{below:02}")
        } else {
            above += 1;
            format!("N{above:02}")
        };
        let v = plan_of(&mut b, *id, name, *z);
        b.sheet(
            number(asr, &code, &mut n),
            format!("{} {name}", tr("Floor plan")),
            &[v],
        );
        plans.push((*id, name.clone(), code, v));
    }
    if model.iter().any(|e| matches!(e.kind, ElementKind::Roof(_))) {
        let roof = b
            .find(|_, v| v.graphics.range.from_above && v.scale <= 150.0)
            .unwrap_or_else(|| {
                let mut v = View::plan(top.0);
                v.graphics.range.from_above = true;
                b.view(tr("Roof plan"), v)
            });
        b.sheet(number(asr, "VYS", &mut n), tr("Roof plan"), &[roof]);
    }
    let pad = 2.0;
    let mut sections: Vec<ElementId> = b
        .m
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::View(v)
                if v.callout.is_none() && matches!(v.kind, ViewKind::Section { cut: true, .. }) =>
            {
                Some(e.id)
            }
            _ => None,
        })
        .collect();
    if sections.is_empty() {
        for (name, a, c) in [
            (
                "Section A-A",
                DVec2::new(mid.x, lo.y - pad),
                DVec2::new(mid.x, hi.y + pad),
            ),
            (
                "Section B-B",
                DVec2::new(lo.x - pad, mid.y),
                DVec2::new(hi.x + pad, mid.y),
            ),
        ] {
            sections.push(b.view(tr(name), View::section(a, c, 30.0, true)));
        }
    }
    b.sheet(number(asr, "REZ", &mut n), tr("Sections"), &sections);
    let mut elevations: Vec<ElementId> =
        b.m.iter()
            .filter_map(|e| match &e.kind {
                ElementKind::View(v)
                    if v.callout.is_none()
                        && matches!(v.kind, ViewKind::Section { cut: false, .. }) =>
                {
                    Some(e.id)
                }
                _ => None,
            })
            .collect();
    if elevations.is_empty() {
        let (l, h) = (lo - DVec2::splat(pad), hi + DVec2::splat(pad));
        // each looks to the left of its line: at the house
        for (name, a, c) in [
            (
                "South elevation",
                DVec2::new(l.x, l.y),
                DVec2::new(h.x, l.y),
            ),
            ("East elevation", DVec2::new(h.x, l.y), DVec2::new(h.x, h.y)),
            (
                "North elevation",
                DVec2::new(h.x, h.y),
                DVec2::new(l.x, h.y),
            ),
            ("West elevation", DVec2::new(l.x, h.y), DVec2::new(l.x, l.y)),
        ] {
            elevations.push(b.view(tr(name), View::section(a, c, 30.0, false)));
        }
    }
    b.sheet(number(asr, "POH", &mut n), tr("Elevations"), &elevations);
    for (kind, category, name) in [
        (
            OpeningKind::Window,
            ScheduleCategory::WindowList,
            "Window list",
        ),
        (OpeningKind::Door, ScheduleCategory::DoorList, "Door list"),
    ] {
        if model
            .iter()
            .any(|e| matches!(&e.kind, ElementKind::Opening(o) if o.kind == kind))
        {
            let v = schedule(&mut b, category, name);
            b.sheet(number(asr, "VVZ", &mut n), tr(name), &[v]);
        }
    }

    // D S01 ZTI and VYK: a services plan a storey with the legend
    let object = |pick: &dyn Fn(&str) -> bool| {
        model.iter().any(|e| {
            matches!(e.kind, ElementKind::Object(_))
                && fixture::family_of(model, e).is_some_and(pick)
        })
    };
    let pipes = model.iter().any(|e| matches!(e.kind, ElementKind::Pipe(_)));
    let plumbing = pipes || !creabim_services::fixturespec::legend(model, None).is_empty();
    let heating = object(&|f| fixture::is_heating(f) || fixture::is_manifold(f))
        || model
            .iter()
            .any(|e| matches!(&e.kind, ElementKind::Room(r) if r.ufh.is_some()));
    for (on, code) in [(plumbing, "ZTI"), (heating, "VYK")] {
        if !on {
            continue;
        }
        let legend = schedule(&mut b, ScheduleCategory::ServicesLegend, "Services legend");
        let part = format!("S01_{code}");
        let mut n = 0;
        for (level, name, storey, arch) in &plans {
            let title = format!("{name} {code}");
            let v = b
                .find(|e, v| {
                    matches!(v.kind, ViewKind::Plan { level: l, .. } if l == *level)
                        && DisplayMode::of(v) == DisplayMode::Services
                        && e.name == title
                })
                .unwrap_or_else(|| {
                    let mut v = match b.m.get(*arch).map(|e| &e.kind) {
                        Some(ElementKind::View(v)) => v.clone(),
                        _ => View::plan(*level),
                    };
                    v.browser_order = None;
                    v.scale = 50.0;
                    DisplayMode::Services.apply(&mut v);
                    b.view(title.clone(), v)
                });
            b.sheet(
                number(&part, storey, &mut n),
                format!("{} {name}", tr("Floor plan")),
                &[v, legend],
            );
        }
    }
    (b.made != Made::default()).then_some((b.tx, b.made))
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::Document;

    fn numbers(m: &Model) -> Vec<String> {
        let mut v: Vec<String> = m
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Sheet(s) => Some(s.number.clone()),
                _ => None,
            })
            .collect();
        v.sort();
        v
    }

    #[test]
    fn the_set_has_a_sheet_a_drawing_numbered_as_the_decree_names_files() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let before = numbers(doc.model()).len();
        let (tx, made) = drawing_set(doc.model()).unwrap();
        doc.commit(tx).unwrap();
        let m = doc.model();
        let all = numbers(m);
        assert_eq!(all.len(), before + made.sheets);
        for want in ["C00_SIT_001", "S01_ASR_001_N01", "S01_ASR_002_N02"] {
            assert!(all.iter().any(|x| x == want), "{want} in {all:?}");
        }
        assert!(all.iter().any(|x| x.ends_with("_REZ")));
        assert!(all.iter().any(|x| x.ends_with("_POH")));
        // the sample's own sections and elevations are used, not doubled
        let sections = |m: &Model| {
            m.iter()
                .filter(|e| {
                    matches!(&e.kind, ElementKind::View(v)
                        if matches!(v.kind, ViewKind::Section { .. }))
                })
                .count()
        };
        assert_eq!(sections(m), sections(&creabim_core::sample::sample_house()));
        // every new sheet has its drawing on it, inside the paper
        for e in m.iter() {
            let ElementKind::Viewport(vp) = &e.kind else {
                continue;
            };
            let Some(ElementKind::Sheet(s)) = m.get(vp.sheet).map(|e| &e.kind) else {
                panic!("a viewport without its sheet");
            };
            assert!(vp.center.x > 0.0 && vp.center.x < s.width);
            assert!(vp.center.y > 0.0 && vp.center.y < s.height);
        }
        // made once: nothing is missing the second time
        assert!(drawing_set(m).is_none());
        assert_eq!(part_of("S01_ASR_001_N01"), Some("D.1 Architecture (ASR)"));
        assert_eq!(part_of("C00_SIT_001"), Some("C Site drawings"));
        assert_eq!(part_of("A101"), None);
    }

    #[test]
    fn a_bare_model_gets_its_sections_and_elevations() {
        use creabim_core::{Level, Wall};
        let mut m = Model::new();
        let level = Element::new(
            "1.NP",
            ElementKind::Level(Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let l = level.id;
        let mut tx = Transaction::new("t").insert(level);
        for (a, c) in [
            (DVec2::ZERO, DVec2::new(8.0, 0.0)),
            (DVec2::new(8.0, 0.0), DVec2::new(8.0, 6.0)),
            (DVec2::new(8.0, 6.0), DVec2::new(0.0, 6.0)),
            (DVec2::new(0.0, 6.0), DVec2::ZERO),
        ] {
            tx = tx.insert(Element::new(
                "Wall",
                ElementKind::Wall(Wall::new(l, a, c, 0.3, 3.0)),
            ));
        }
        tx.apply(&mut m).unwrap();
        let (tx, made) = drawing_set(&m).unwrap();
        tx.apply(&mut m).unwrap();
        // the site, the plan, two sections and four elevations
        assert_eq!(made.views, 8);
        assert_eq!(
            numbers(&m),
            [
                "C00_SIT_001",
                "S01_ASR_001_N01",
                "S01_ASR_002_REZ",
                "S01_ASR_003_POH"
            ]
        );
        // both sections on the one sheet
        let rez = m
            .iter()
            .find(|e| matches!(&e.kind, ElementKind::Sheet(s) if s.number.ends_with("REZ")))
            .unwrap()
            .id;
        let on = m
            .iter()
            .filter(|e| matches!(&e.kind, ElementKind::Viewport(v) if v.sheet == rez))
            .count();
        assert_eq!(on, 2);
    }
}
