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
    part_order(number).map(|p| p.1)
}

/// The same with the part's place in the documentation, from 0.
pub fn part_order(number: &str) -> Option<(usize, &'static str)> {
    const PARTS: [(&str, &str); 12] = [
        ("A00_", "A List of documentation"),
        ("B00_", "B Summary report"),
        ("C00_", "C Site drawings"),
        ("_ASR_", "D.1 Architecture (ASR)"),
        ("_STA_", "D.2 Structure (STA)"),
        ("_ZTI_", "D.3 Plumbing (ZTI)"),
        ("_VYK_", "D.4 Heating (VYK)"),
        ("_ELI_", "D.5 Electrical (ELI)"),
        ("_VZT_", "D.6 Ventilation (VZT)"),
        ("_PLY_", "D.7 Gas (PLY)"),
        ("_MPL_", "D.8 Medical gases (MPL)"),
        ("E00_", "E Annexes"),
    ];
    PARTS
        .iter()
        .position(|(k, _)| number.starts_with(k) || number.contains(k))
        .map(|i| (i, PARTS[i].1))
}

/// The file a sheet of the set numbered `number` is exported to, as annex
/// 24 names it: `prefix` (the construction's ID, the stage, the building
/// in its group and its code) before the sheet's number. None for a
/// sheet outside the set, or with no prefix given. The documentation's
/// `revision` ends the name.
pub fn file_name(prefix: &str, number: &str, revision: &str) -> Option<String> {
    let prefix = prefix.trim().trim_matches('_');
    (!prefix.is_empty() && part_of(number).is_some())
        .then(|| format!("{prefix}_{number}_{revision}"))
}

struct Builder {
    /// The model with what has been made so far.
    m: Model,
    tx: Transaction,
    made: Made,
    /// A view it found was put right.
    fixed: bool,
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

    /// Change the element `id` as `f` does, in the model and the
    /// transaction.
    fn edit(&mut self, id: ElementId, f: impl Fn(&mut Element)) {
        let Some(old) = self.m.get(id).cloned() else {
            return;
        };
        let mut now = old.clone();
        f(&mut now);
        if now == old {
            return;
        }
        let one = Transaction::new("one").replace(old.clone(), now.clone());
        if one.apply(&mut self.m).is_ok() {
            let tx = std::mem::replace(&mut self.tx, Transaction::new(""));
            self.tx = tx.replace(old, now);
            self.fixed = true;
        }
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
        // a drawing with its legends: the legends in a column beside it
        let legend = |v: &ElementId| {
            matches!(self.m.get(*v).map(|e| &e.kind),
                Some(ElementKind::View(x)) if matches!(x.kind, ViewKind::Schedule { .. }))
        };
        if views.len() > 2 && !legend(&views[0]) && views[1..].iter().all(legend) {
            let side = &sizes[1..];
            w = sizes[0].x + 20.0 + side.iter().map(|s| s.x + 20.0).fold(0.0, f64::max);
            h = (sizes[0].y + 25.0).max(side.iter().map(|s| s.y + 25.0).sum());
        }
        let (need_w, need_h) = (w + 40.0, h + 75.0);
        let size = Sheet::SIZES
            .iter()
            .filter(|(_, sw, sh)| *sw >= need_w && *sh >= need_h)
            .min_by(|a, b| (a.1 * a.2).total_cmp(&(b.1 * b.2)))
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
    set(model, false)
}

/// The frame of the drawing set a new project starts with, before
/// anything is drawn: the site, a plan a storey with its legends, the
/// sections and elevations, the lists of windows and doors and the list
/// of documentation.
pub fn skeleton(model: &Model) -> Option<(Transaction, Made)> {
    set(model, true)
}

/// The house a new project's sections and elevations are set round, m.
const FRAME: DVec2 = DVec2::new(10.0, 8.0);

fn set(model: &Model, frame: bool) -> Option<(Transaction, Made)> {
    let walls: Vec<&creabim_core::Wall> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Wall(w) => Some(w),
            _ => None,
        })
        .collect();
    // a frame is for a project with nothing built yet
    let frame = frame && walls.is_empty();
    let (lo, hi) = if frame {
        (DVec2::ZERO, FRAME)
    } else {
        let first = walls.first()?.start;
        walls
            .iter()
            .flat_map(|w| [w.start, w.end])
            .fold((first, first), |(lo, hi), p| (lo.min(p), hi.max(p)))
    };
    let mid = (lo + hi) / 2.0;
    let mut b = Builder {
        m: model.clone(),
        tx: Transaction::new("Drawing set"),
        made: Made::default(),
        fixed: false,
    };
    // the storeys: those with walls of their own, from the lowest up
    let storeys: Vec<(ElementId, String, f64)> = levels::sorted(model)
        .into_iter()
        .filter(|(id, ..)| frame || walls.iter().any(|w| w.level == *id))
        .map(|(id, name, l)| (id, name, l.elevation))
        .collect();
    let (bottom, top) = (storeys.first()?.clone(), storeys.last()?.clone());
    let plain = |v: &View| {
        !v.ceiling_plan
            && !v.graphics.range.from_above
            && DisplayMode::of(v) == DisplayMode::Architecture
    };
    // the architect's plan of a storey, without the trades' pipes
    let plan_of = |b: &mut Builder, level: ElementId, name: &str, z: f64| {
        let pipes = creabim_core::visibility::Category::Pipes;
        let found = b.find(|_, v| {
            matches!(v.kind, ViewKind::Plan { level: l, .. } if l == level) && plain(v)
        });
        let Some(id) = found else {
            let mut v = levels::plan_of(&b.m, level, z);
            v.hidden.set_category(pipes, true);
            return b.view(levels::plan_name(name), v);
        };
        if let Some(old) = b.m.get(id).cloned() {
            let mut now = old.clone();
            if let ElementKind::View(x) = &mut now.kind {
                x.hidden.set_category(pipes, true);
            }
            if now != old {
                let one = Transaction::new("one").replace(old.clone(), now.clone());
                if one.apply(&mut b.m).is_ok() {
                    let tx = std::mem::replace(&mut b.tx, Transaction::new(""));
                    b.tx = tx.replace(old, now);
                    b.fixed = true;
                }
            }
        }
        id
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
            v.graphics.range.footprint = true;
            v.scale = 200.0;
            b.view(tr("Site plan"), v)
        });
    // its legend once networks are drawn
    let legend = (!crate::serviceslegend::networks(model).is_empty()).then(|| {
        b.find(|_, v| v.site_legend).unwrap_or_else(|| {
            let mut v = View::schedule(ScheduleCategory::ServicesLegend);
            v.site_legend = true;
            b.view(tr("Site plan legend"), v)
        })
    });
    b.sheet(
        "C00_SIT_001".into(),
        tr("Site plan"),
        &std::iter::once(site).chain(legend).collect::<Vec<_>>(),
    );

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
    // the legend of materials beside the plans, once a wall has one
    let materials = (frame || !crate::matlegend::rows(model).is_empty()).then(|| {
        schedule(
            &mut b,
            ScheduleCategory::MaterialLegend,
            "Legend of materials",
        )
    });
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
        // the storey's rooms with their finishes, once it has rooms
        let rooms = (frame
            || model
                .iter()
                .any(|e| matches!(&e.kind, ElementKind::Room(r) if r.level == *id)))
        .then(|| {
            b.find(|_, v| {
                matches!(v.kind, ViewKind::Schedule { category, .. }
                        if category == ScheduleCategory::RoomLegend)
                    && v.of_level == Some(*id)
            })
            .unwrap_or_else(|| {
                let mut v = View::schedule(ScheduleCategory::RoomLegend);
                v.of_level = Some(*id);
                b.view(format!("{} {name}", tr("Legend of rooms")), v)
            })
        });
        b.sheet(
            number(asr, &code, &mut n),
            format!("{} {name}", tr("Floor plan")),
            &std::iter::once(v)
                .chain(rooms)
                .chain(materials)
                .collect::<Vec<_>>(),
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
        if frame
            || model
                .iter()
                .any(|e| matches!(&e.kind, ElementKind::Opening(o) if o.kind == kind))
        {
            // a long list page by page, each on its own sheet (Okná 1,
            // Okná 2); a list already set whole stays whole
            let whole = b.find(|_, v| {
                matches!(v.kind, ViewKind::Schedule { category: c, .. } if c == category)
                    && v.page.is_none()
            });
            let split = crate::joinerylist::pages(model, kind);
            if split.len() < 2 {
                let v = schedule(&mut b, category, name);
                b.sheet(number(asr, "VVZ", &mut n), tr(name), &[v]);
            } else {
                // the whole list grown too long: its sheet takes the first
                // page
                if let Some(id) = whole {
                    let title = format!("{} 1", tr(name));
                    b.edit(id, |e| {
                        e.name = title.clone();
                        if let ElementKind::View(v) = &mut e.kind {
                            v.page = Some(1);
                        }
                    });
                    let sheets: Vec<ElementId> =
                        b.m.iter()
                            .filter_map(|e| match &e.kind {
                                ElementKind::Viewport(vp) if vp.view == id => Some(vp.sheet),
                                _ => None,
                            })
                            .collect();
                    for sh in sheets {
                        b.edit(sh, |e| e.name = title.clone());
                    }
                }
                for page in 1..=split.len() as u32 {
                    let title = format!("{} {page}", tr(name));
                    let v = b
                        .find(|_, v| {
                            matches!(v.kind, ViewKind::Schedule { category: c, .. } if c == category)
                                && v.page == Some(page)
                        })
                        .unwrap_or_else(|| {
                            let mut v = View::schedule(category);
                            v.page = Some(page);
                            b.view(title.clone(), v)
                        });
                    // a number taken by another sheet: this page's after it
                    let mut no = number(asr, "VVZ", &mut n);
                    let shown =
                        b.m.iter()
                            .any(|e| matches!(&e.kind, ElementKind::Viewport(vp) if vp.view == v));
                    let taken = |b: &Builder, no: &str| {
                        b.m.iter()
                            .any(|e| matches!(&e.kind, ElementKind::Sheet(s) if s.number == no))
                    };
                    if !shown && taken(&b, &no) {
                        no = format!("{no}_{page}");
                    }
                    if !shown {
                        b.sheet(no, title, &[v]);
                    }
                }
            }
        }
    }
    // the system lintels placed by hand, listed
    if model.iter().any(creabim_core::masonry::is_lintel) {
        let v = schedule(&mut b, ScheduleCategory::LintelList, "Lintel list");
        b.sheet(number(asr, "VVZ", &mut n), tr("Lintel list"), &[v]);
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
    use creabim_core::viewmodes::Trade;
    let ducts = model.iter().any(|e| {
        matches!(&e.kind, ElementKind::Pipe(p) if Trade::Ventilation.shows(p.kind())
            && !Trade::Plumbing.shows(p.kind()))
    });
    let wiring = !creabim_core::electrical::devices(model).is_empty();
    for (on, code, trade) in [
        (plumbing, "ZTI", Trade::Plumbing),
        (heating, "VYK", Trade::Heating),
        (ducts, "VZT", Trade::Ventilation),
        (wiring, "ELI", Trade::Electrical),
    ] {
        if !on {
            continue;
        }
        // the trade's own legend; one made before legends were told
        // apart serves water and heating as it did
        let legend = b
            .find(|_, v| {
                matches!(v.kind, ViewKind::Schedule { category, .. }
                    if category == ScheduleCategory::ServicesLegend)
                    && !v.site_legend
                    && (v.trade == Some(trade)
                        || (v.trade.is_none() && matches!(trade, Trade::Plumbing | Trade::Heating)))
            })
            .unwrap_or_else(|| {
                let mut v = View::schedule(ScheduleCategory::ServicesLegend);
                v.trade = Some(trade);
                b.view(format!("{} {code}", tr("Legend")), v)
            });
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
                    v.trade = Some(trade);
                    b.view(title.clone(), v)
                });
            // one made before trades were told apart takes its trade now
            if let Some(old) = b.m.get(v).cloned() {
                if matches!(&old.kind, ElementKind::View(x) if x.trade.is_none()) {
                    let mut now = old.clone();
                    if let ElementKind::View(x) = &mut now.kind {
                        x.trade = Some(trade);
                    }
                    let one = Transaction::new("one").replace(old.clone(), now.clone());
                    if one.apply(&mut b.m).is_ok() {
                        let tx = std::mem::replace(&mut b.tx, Transaction::new(""));
                        b.tx = tx.replace(old, now);
                    }
                }
            }
            // the trade's take-off on its first sheet, beside the legend
            let list = (n == 0)
                .then(|| {
                    if trade == Trade::Electrical {
                        return Some(schedule(
                            &mut b,
                            ScheduleCategory::ElectricalList,
                            "Electrical list",
                        ));
                    }
                    if crate::tradelists::pipes(&b.m, Some(trade)).rows.is_empty() {
                        return None;
                    }
                    Some(
                        b.find(|_, v| {
                            matches!(v.kind, ViewKind::Schedule { category, .. }
                                if category == ScheduleCategory::PipeList)
                                && v.trade == Some(trade)
                        })
                        .unwrap_or_else(|| {
                            let mut v = View::schedule(ScheduleCategory::PipeList);
                            v.trade = Some(trade);
                            b.view(format!("{} {code}", tr("Pipe list")), v)
                        }),
                    )
                })
                .flatten();
            b.sheet(
                number(&part, storey, &mut n),
                format!("{} {name}", tr("Floor plan")),
                &[v, legend].into_iter().chain(list).collect::<Vec<_>>(),
            );
        }
        // the electrician's board diagram on a sheet of its own
        if trade == Trade::Electrical && !creabim_core::electrical::circuits(&b.m).is_empty() {
            let d = schedule(&mut b, ScheduleCategory::BoardDiagram, "Board diagram");
            b.sheet(number(&part, "SCH", &mut n), tr("Board diagram"), &[d]);
        }
    }
    // the title pages: the project's own and one before each part
    covers(&mut b, frame);
    // A: the list of all of it, made last so it lists every sheet
    // the material take-off, every material of every construction
    schedule(&mut b, ScheduleCategory::Materials, "Material take-off");
    let list = schedule(&mut b, ScheduleCategory::Documents, "List of documentation");
    b.sheet("A00_TXT_001".into(), tr("List of documentation"), &[list]);
    (b.made != Made::default() || b.fixed).then_some((b.tx, b.made))
}

/// The title pages still missing: the project's own, with the picture
/// of the building once there is one to show, and one for each part
/// that has drawings (see [`crate::coverpage`]).
fn covers(b: &mut Builder, frame: bool) {
    use crate::coverpage;
    let has = |m: &Model, part: &str| {
        m.iter().find_map(|e| match &e.kind {
            ElementKind::Sheet(s) if coverpage::cover_of(s) == Some(part) => Some(e.id),
            _ => None,
        })
    };
    let mut parts = vec![coverpage::PROJECT];
    for (key, ..) in coverpage::PARTS {
        let drawn = b.m.iter().any(|e| {
            matches!(&e.kind, ElementKind::Sheet(s)
                if coverpage::cover_of(s).is_none()
                    && (s.number.starts_with(key) || s.number.contains(key)))
        });
        if drawn {
            parts.push(key);
        }
    }
    for part in parts {
        if has(&b.m, part).is_none() {
            let name = if part == coverpage::PROJECT {
                tr("Title page")
            } else {
                format!(
                    "{} {}",
                    tr("Title page"),
                    part.trim_matches('_').trim_end_matches("00")
                )
            };
            b.add(Element::new(
                name,
                ElementKind::Sheet(coverpage::sheet(part)),
            ));
            b.made.sheets += 1;
        }
    }
    // the picture: a view of the building from above at an angle, sized
    // to the frame, once there is a building
    let Some(main) = has(&b.m, coverpage::PROJECT) else {
        return;
    };
    let placed =
        b.m.iter()
            .any(|e| matches!(&e.kind, ElementKind::Viewport(v) if v.sheet == main));
    if placed || frame {
        return;
    }
    let Some(ElementKind::Sheet(sheet)) = b.m.get(main).map(|e| e.kind.clone()) else {
        return;
    };
    let mut v = View::axonometric(35.0, 30.0);
    v.sheet_title.hidden = true;
    v.browser_order = None;
    let view = b.view(tr("Title page picture"), v);
    let Some((lo, hi)) = view_drawing(&b.m, view).and_then(|d| d.bounds()) else {
        return;
    };
    let (blo, bhi) = coverpage::picture_box(&sheet);
    let (room, size) = (bhi - blo - DVec2::splat(4.0), hi - lo);
    let need = (size.x * 1000.0 / room.x).max(size.y * 1000.0 / room.y);
    // the next round scale it fits at
    let scale = [
        20.0, 25.0, 50.0, 75.0, 100.0, 150.0, 200.0, 250.0, 300.0, 400.0, 500.0, 750.0, 1000.0,
        1500.0, 2000.0,
    ]
    .into_iter()
    .find(|s| *s >= need)
    .unwrap_or(need.ceil());
    if let Some(old) = b.m.get(view).cloned() {
        let mut now = old.clone();
        if let ElementKind::View(x) = &mut now.kind {
            x.scale = scale;
        }
        let one = Transaction::new("one").replace(old.clone(), now.clone());
        if one.apply(&mut b.m).is_ok() {
            let tx = std::mem::replace(&mut b.tx, Transaction::new(""));
            b.tx = tx.replace(old, now);
        }
    }
    let vp = crate::sheet::new_viewport(&b.m, main, view, (blo + bhi) / 2.0);
    b.add(Element::new("Viewport", ElementKind::Viewport(vp)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::Document;

    fn numbers(m: &Model) -> Vec<String> {
        let mut v: Vec<String> = m
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Sheet(s) if crate::coverpage::cover_of(s).is_none() => {
                    Some(s.number.clone())
                }
                _ => None,
            })
            .collect();
        v.sort();
        v
    }

    /// The parts with a title page, the project's own first.
    fn covers(m: &Model) -> Vec<String> {
        let mut v: Vec<String> = m
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Sheet(s) => crate::coverpage::cover_of(s).map(str::to_string),
                _ => None,
            })
            .collect();
        v.sort();
        v
    }

    /// Part A lists every sheet under its part, the text of part B and
    /// the annexes the decree always asks for; its drawing carries the
    /// construction's data over the table.
    #[test]
    fn the_list_of_documentation_lists_every_sheet_by_part() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let (tx, _) = drawing_set(doc.model()).unwrap();
        doc.commit(tx).unwrap();
        let m = doc.model();
        let all = numbers(m);
        assert!(all.iter().any(|x| x == "A00_TXT_001"), "{all:?}");
        let s = crate::schedule::schedule_by(m, ScheduleCategory::Documents, false);
        assert_eq!(s.listed, all.len());
        let first: Vec<&str> = s.rows.iter().map(|r| r.cells[0].as_str()).collect();
        for n in &all {
            assert!(first.contains(&n.as_str()), "{n} listed");
        }
        assert!(first.contains(&crate::doclist::SUMMARY_NUMBER));
        // parts in the documentation's order: A before C before D before E
        let heads: Vec<&str> = s
            .rows
            .iter()
            .filter(|r| r.group)
            .map(|r| r.cells[0].as_str())
            .collect();
        assert!(heads.len() >= 5, "{heads:?}");
        let mut sorted = heads.clone();
        sorted.sort();
        assert_eq!(heads, sorted);
        // a plan's sheet carries its scale
        let plan = s.rows.iter().find(|r| r.cells[0] == "S01_ASR_001_N01");
        assert!(plan.is_some_and(|r| r.cells[2].starts_with("1:")));
        let list = m
            .iter()
            .find(|e| {
                matches!(&e.kind, ElementKind::View(v)
                    if matches!(v.kind, ViewKind::Schedule { category: ScheduleCategory::Documents, .. }))
            })
            .unwrap();
        let d = view_drawing(m, list.id).unwrap();
        let texts = d
            .items
            .iter()
            .filter(|i| matches!(i, crate::plan::PlanItem::Text { .. }))
            .count();
        assert!(texts > s.rows.len() * 2);
    }

    #[test]
    fn the_set_has_a_sheet_a_drawing_numbered_as_the_decree_names_files() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let before = numbers(doc.model()).len();
        let (tx, made) = drawing_set(doc.model()).unwrap();
        doc.commit(tx).unwrap();
        let m = doc.model();
        let all = numbers(m);
        assert_eq!(all.len() + covers(m).len(), before + made.sheets);
        // a title page for the project and each part drawn
        assert_eq!(covers(m), ["C00_", "_ASR_", "project"]);
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
        assert_eq!(
            file_name("24XY456_PSP_01_1111_", "S01_ASR_001_N01", "00").as_deref(),
            Some("24XY456_PSP_01_1111_S01_ASR_001_N01_00")
        );
        assert_eq!(file_name("", "S01_ASR_001_N01", "00"), None);
        assert_eq!(file_name("24XY456_PSP_01_1111", "A101", "00"), None);
    }

    #[test]
    fn a_new_project_starts_with_the_frame_of_its_drawing_set() {
        use creabim_core::Level;
        let mut m = Model::new();
        let mut tx = Transaction::new("t");
        for (name, z) in [("1. NP", 0.0), ("2. NP", 3.0)] {
            tx = tx.insert(Element::new(
                name,
                ElementKind::Level(Level {
                    elevation: z,
                    height: 3.0,
                }),
            ));
        }
        tx.apply(&mut m).unwrap();
        // nothing is drawn: the set by itself waits for walls
        assert!(drawing_set(&m).is_none());
        let (tx, _) = skeleton(&m).unwrap();
        tx.apply(&mut m).unwrap();
        assert_eq!(
            numbers(&m),
            [
                "A00_TXT_001",
                "C00_SIT_001",
                "S01_ASR_001_N01",
                "S01_ASR_002_N02",
                "S01_ASR_003_REZ",
                "S01_ASR_004_POH",
                "S01_ASR_005_VVZ",
                "S01_ASR_006_VVZ"
            ]
        );
        // title pages, the project's without a picture of nothing yet
        assert_eq!(covers(&m), ["C00_", "_ASR_", "project"]);
        // the architect's plans leave the trades' pipes out
        let plans = m
            .iter()
            .filter(|e| {
                matches!(&e.kind, ElementKind::View(v)
                if matches!(v.kind, ViewKind::Plan { .. })
                    && !v.graphics.range.from_above
                    && v.hidden.hides_category(creabim_core::visibility::Category::Pipes))
            })
            .count();
        assert_eq!(plans, 2);
        // and it is made once
        assert!(skeleton(&m).is_none());
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
        // the site, the plan, two sections, four elevations, the material
        // take-off, the list and the picture on the title page
        assert_eq!(made.views, 11);
        let cover = m
            .iter()
            .find(|e| matches!(&e.kind, ElementKind::Sheet(s) if crate::coverpage::cover_of(s) == Some("project")))
            .unwrap();
        let ElementKind::Sheet(sheet) = &cover.kind else {
            unreachable!()
        };
        let (lo, hi) = crate::coverpage::picture_box(sheet);
        let vp = m
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::Viewport(v) if v.sheet == cover.id => Some(v.clone()),
                _ => None,
            })
            .expect("the picture on the title page");
        assert!(
            vp.center.x > lo.x && vp.center.x < hi.x && vp.center.y > lo.y && vp.center.y < hi.y
        );
        assert_eq!(
            numbers(&m),
            [
                "A00_TXT_001",
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
