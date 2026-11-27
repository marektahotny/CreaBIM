//! The legend of a services drawing, as the office's ZTI and ÚK sheets
//! carry it beside the plan: the pipes (a sample of each system's line
//! with its name and what it is made of) and the sanitary fixtures (each
//! mark with the fixture, its trap and its valves). It is a schedule
//! view, so it goes on a sheet beside its plan.

use crate::joinerylist::wrap;
use crate::paper::Anchor;
use crate::plan::{Pen, PlanItem, PlanView};
use creabim_core::mep::{SystemDef, SYSTEMS};
use creabim_core::viewmodes::Trade;
use creabim_core::words::tr;
use creabim_core::{ElementId, ElementKind, Model};
use glam::DVec2;

const HEAD: f64 = 3.2;
const TEXT: f64 = 2.2;
const LINE: f64 = 3.6;
/// Where a row's text starts, after its line sample or its mark, mm.
const INDENT: f64 = 26.0;
const SAMPLE: f64 = 20.0;
const WIDTH: f64 = 150.0;
/// The sample's and the words' columns as drawn by default, mm.
pub const COLUMNS: (f64, f64) = (INDENT, WIDTH - INDENT);

/// The systems the model's pipes are in, in the systems' own order, each
/// with the names of the pipe types it is laid in.
pub fn systems(model: &Model) -> Vec<(&'static SystemDef, Vec<String>)> {
    let mut out: Vec<(&'static SystemDef, Vec<String>)> = vec![];
    for s in SYSTEMS {
        let mut types: Vec<String> = vec![];
        let mut any = false;
        for e in model.iter() {
            let ElementKind::Pipe(p) = &e.kind else {
                continue;
            };
            if p.system != s.code {
                continue;
            }
            any = true;
            let name = creabim_services::pipe_types::by_id(&p.pipe_type)
                .map_or(p.pipe_type.clone(), |t| t.name.clone());
            if !types.contains(&name) {
                types.push(name);
            }
        }
        if any {
            out.push((s, types));
        }
    }
    out
}

/// The equipment of the services: the placed objects of plumbing and of
/// heating and ventilation that are neither sanitary fixtures nor
/// heaters (heat pumps, tanks, boilers, manifolds), each type once with
/// its mark, or how many there are, and its name.
pub fn equipment(model: &Model, trade: Option<Trade>) -> Vec<(String, String)> {
    use creabim_core::objects::{self, ObjectCategory};
    let marks = creabim_core::marks::object_marks(model);
    // (type, how many)
    let mut found: Vec<(ElementId, usize)> = vec![];
    for e in model.iter() {
        if !matches!(e.kind, ElementKind::Object(_)) {
            continue;
        }
        let Some(r) = &e.of_type else { continue };
        let Some(t) = objects::type_of_id(model, r.id) else {
            continue;
        };
        let shown = matches!(
            (t.category, trade),
            (ObjectCategory::Plumbing | ObjectCategory::Hvac, None)
                | (ObjectCategory::Plumbing, Some(Trade::Plumbing))
                | (
                    ObjectCategory::Hvac,
                    Some(Trade::Heating | Trade::Ventilation)
                )
        );
        if !shown {
            continue;
        }
        let family = creabim_core::fixture::family_of(model, e);
        if family.is_some_and(|f| !creabim_core::fixture::is_manifold(f)) {
            continue;
        }
        match found.iter_mut().find(|f| f.0 == r.id) {
            Some(f) => f.1 += 1,
            None => found.push((r.id, 1)),
        }
    }
    found
        .into_iter()
        .map(|(id, n)| {
            let name = model.get(id).map(|t| t.name.clone()).unwrap_or_default();
            let lead = marks
                .get(&id)
                .filter(|m| !m.is_empty())
                .cloned()
                .unwrap_or_else(|| format!("{n}x"));
            (lead, name)
        })
        .collect()
}

/// The site plan's objects with a legend line of their own (see
/// [`creabim_core::catalogue::site::legend`]), each type once: its plan
/// symbol and the line.
pub fn site_objects(model: &Model) -> Vec<(Vec<creabim_core::objects::SymbolItem>, String)> {
    let mut seen = vec![];
    let mut out = vec![];
    for e in model.iter() {
        if !matches!(e.kind, ElementKind::Object(_)) {
            continue;
        }
        let Some(t) = creabim_core::objects::type_of(model, e) else {
            continue;
        };
        let Some(key) = t.catalogue.as_deref() else {
            continue;
        };
        let Some(line) = creabim_core::catalogue::site::legend(key) else {
            continue;
        };
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        out.push((t.plan_symbol(), tr(line)));
    }
    out
}

/// The trade the legend drawn as `view` is for; none is for all of them.
pub fn trade_of(model: &Model, view: ElementId) -> Option<Trade> {
    match model.get(view).map(|e| &e.kind) {
        Some(ElementKind::View(v)) => v.trade,
        _ => None,
    }
}

/// A row of the electrical legend: a type of device with its plan symbol.
pub struct DeviceRow {
    /// How many there are.
    pub count: usize,
    /// "Socket - Single socket".
    pub name: String,
    /// The type's plan symbol as strokes, metres, the wall along x at
    /// y = 0.
    pub symbol: Vec<(DVec2, DVec2)>,
}

/// The electrical devices, each type once, with its symbol as the plans
/// draw it.
pub fn devices(model: &Model) -> Vec<DeviceRow> {
    use creabim_core::electrical as el;
    let mut found: Vec<(el::DeviceKind, String, DeviceRow)> = vec![];
    for (e, _, kind) in el::devices(model) {
        let name = e
            .of_type
            .as_ref()
            .and_then(|r| model.get(r.id))
            .map_or(e.name.clone(), |t| t.name.clone());
        if let Some(d) = found.iter_mut().find(|d| d.0 == kind && d.1 == name) {
            d.2.count += 1;
            continue;
        }
        let mut symbol = vec![];
        for s in creabim_core::objects::type_of(model, e)
            .map(|t| t.plan_symbol())
            .unwrap_or_default()
        {
            symbol.extend(s.segments());
        }
        let row = DeviceRow {
            count: 1,
            name: format!("{} - {}", tr(kind.label()), tr(&name)),
            symbol,
        };
        found.push((kind, name, row));
    }
    found.sort_by_key(|d| d.0 as u8);
    found.into_iter().map(|d| d.2).collect()
}

/// The paper scale of a legend's symbols, mm to a metre: as a 1:50 plan
/// draws them.
const SYMBOL_SCALE: f64 = 20.0;
/// The room a symbol has in its row, mm.
const SYMBOL_BOX: DVec2 = DVec2::new(INDENT - 6.0, 9.0);

/// A device's symbol's bounds and its paper scale, mm to a metre: the
/// plans' scale unless it has to shrink into `SYMBOL_BOX`.
fn symbol_fit(row: &DeviceRow) -> Option<(DVec2, DVec2, f64)> {
    let first = row.symbol.first()?;
    let (lo, hi) = row
        .symbol
        .iter()
        .flat_map(|(a, b)| [*a, *b])
        .fold((first.0, first.0), |(lo, hi), p| (lo.min(p), hi.max(p)));
    let size = (hi - lo).max(DVec2::splat(1e-6));
    let k = SYMBOL_SCALE
        .min(SYMBOL_BOX.x / size.x)
        .min(SYMBOL_BOX.y / size.y);
    Some((lo, hi, k))
}

/// How tall a device's symbol is drawn, mm.
fn symbol_height(row: &DeviceRow) -> f64 {
    symbol_fit(row).map_or(0.0, |(lo, hi, k)| (hi.y - lo.y) * k)
}

/// A device's symbol about `centre`.
fn device_symbol(items: &mut Vec<PlanItem>, id: ElementId, row: &DeviceRow, centre: DVec2) {
    let Some((lo, hi, k)) = symbol_fit(row) else {
        return;
    };
    let mid = (lo + hi) / 2.0;
    for (a, b) in &row.symbol {
        items.push(PlanItem::Line {
            id,
            pen: crate::plan::Pen::Projection,
            a: centre + (*a - mid) * k,
            b: centre + (*b - mid) * k,
        });
    }
}

/// The notes of the legend drawn as `view`: its own, a line each, else
/// the usual ones of the trades the model has pipes of.
pub fn notes(model: &Model, view: ElementId) -> Vec<String> {
    // its own, less those switched off (all off: none, not the usual)
    if let Some(ElementKind::View(v)) = model.get(view).map(|e| &e.kind) {
        if !v.notes.trim().is_empty() {
            return v
                .notes
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !v.notes_off.iter().any(|o| o.trim() == *l))
                .map(str::to_string)
                .collect();
        }
    }
    let trade = trade_of(model, view);
    if trade.is_some_and(|t| t != Trade::Plumbing && t != Trade::Heating) {
        return vec![];
    }
    let has = |codes: &[&str]| {
        model
            .iter()
            .any(|e| matches!(&e.kind, ElementKind::Pipe(p) if codes.contains(&p.system.as_str())))
    };
    let mut out = vec![];
    if has(&["SV", "TV", "CTV"]) {
        out.push(tr(
            "Insulate the water pipes with foamed polyethylene insulation of the prescribed thickness.",
        ));
    }
    if model.iter().any(|e| matches!(e.kind, ElementKind::Pipe(_))) {
        out.push(tr(
            "Where pipes in the floor pass through masonry, lay them in a sleeve.",
        ));
        out.push(tr(
            "Before the works, check the position and depth of the existing public mains and adapt the laying of the services to them on site.",
        ));
    }
    out
}

/// A sample of a system's line from `a`, `len` long: its dashes as thin
/// strips in its colour.
fn sample(items: &mut Vec<PlanItem>, id: ElementId, s: &SystemDef, a: DVec2, len: f64) {
    let rgb = [
        ((s.color >> 16) & 0xff) as f32 / 255.0,
        ((s.color >> 8) & 0xff) as f32 / 255.0,
        (s.color & 0xff) as f32 / 255.0,
    ];
    let half = crate::looks::PIPE_LINE / 2.0;
    let mut strip = |x0: f64, x1: f64| {
        items.push(PlanItem::Shade {
            id,
            points: vec![
                a + DVec2::new(x0, -half),
                a + DVec2::new(x1, -half),
                a + DVec2::new(x1, half),
                a + DVec2::new(x0, half),
            ],
            rgb,
        })
    };
    let Some(dashes) = s.dashes().filter(|d| d.iter().sum::<f64>() > 0.0) else {
        strip(0.0, len);
        return;
    };
    let (mut x, mut i) = (0.0, 0);
    while x < len {
        let d = dashes[i % dashes.len()];
        if i % 2 == 0 {
            // a dash of nothing is a dot
            strip(x, (x + d.max(2.0 * half)).min(len));
        }
        x += d;
        i += 1;
    }
}

/// A sample of a duct `len` long from `a`, as the VZT drawing shows it:
/// its outline in its system's colour washed lightly, or with `insulated`
/// the insulation's dashed outline round it.
fn duct_sample(
    items: &mut Vec<PlanItem>,
    id: ElementId,
    s: &SystemDef,
    a: DVec2,
    len: f64,
    insulated: bool,
) {
    let rgb = [
        ((s.color >> 16) & 0xff) as f32 / 255.0,
        ((s.color >> 8) & 0xff) as f32 / 255.0,
        (s.color & 0xff) as f32 / 255.0,
    ];
    let half = if insulated { 0.9 } else { 1.4 };
    let box_ = |h: f64| {
        vec![
            a + DVec2::new(0.0, -h),
            a + DVec2::new(len, -h),
            a + DVec2::new(len, h),
            a + DVec2::new(0.0, h),
            a + DVec2::new(0.0, -h),
        ]
    };
    items.push(PlanItem::Tint {
        id,
        points: box_(half)[..4].to_vec(),
        rgb,
    });
    items.push(PlanItem::Curve {
        id,
        points: box_(half),
        rgb,
        dashed: false,
    });
    if insulated {
        items.push(PlanItem::Curve {
            id,
            points: box_(1.6),
            rgb,
            dashed: true,
        });
    }
}

/// The utility networks drawn in the model, each kind, state and note
/// once, in the order of their kinds and states.
pub fn networks(model: &Model) -> Vec<creabim_core::utility::UtilityLine> {
    use creabim_core::utility::{NetKind, NetState};
    let mut out: Vec<creabim_core::utility::UtilityLine> = vec![];
    for e in model.iter() {
        if let ElementKind::Sketch(sk) = &e.kind {
            if let Some(n) = &sk.net {
                if !out.contains(n) {
                    out.push(n.clone());
                }
            }
        }
    }
    let rank = |k: NetKind| NetKind::ALL.iter().position(|x| *x == k).unwrap_or(99);
    let st = |k: NetState| NetState::ALL.iter().position(|x| *x == k).unwrap_or(99);
    out.sort_by(|a, b| {
        (rank(a.kind), st(a.state), &a.note).cmp(&(rank(b.kind), st(b.state), &b.note))
    });
    out
}

/// A sample of a network's line from `a`, `len` long: its pattern (dash,
/// gap, …; a dash of nothing a dot) in its colour and weight, crossed
/// when cancelled.
fn net_sample(
    items: &mut Vec<PlanItem>,
    id: ElementId,
    n: &creabim_core::utility::UtilityLine,
    a: DVec2,
    len: f64,
) {
    // as on the plan, at paper size: dashes, its symbol, crossed if to go
    let strokes = crate::netlines::run(n, a, a + DVec2::X * len, 1.0, true);
    items.extend(crate::netlines::paper_items(
        id,
        strokes,
        n.kind.rgb(),
        n.weight(),
    ));
}

/// The site plan's own symbols placed in the model (hydrants, manholes,
/// the entrances…), each once, in the order of the tables.
pub fn site_marks(model: &Model) -> Vec<creabim_core::utility::SiteMark> {
    use creabim_core::utility::SiteMark;
    let used: Vec<SiteMark> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Sketch(sk) => sk.mark,
            _ => None,
        })
        .collect();
    SiteMark::ALL
        .into_iter()
        .filter(|m| used.contains(m))
        .collect()
}

/// The legend as a drawing in paper millimetres, its top left corner at
/// the origin.
pub fn drawing(view: ElementId, title: &str, model: &Model) -> PlanView {
    // the columns' widths as set on the sheet
    let (ind, tw) = crate::schedule::legend_columns(model, view, INDENT, WIDTH - INDENT);
    let mut items = vec![];
    let text = |items: &mut Vec<PlanItem>, at: DVec2, t: &str, size: f64, rgb| {
        items.push(PlanItem::Text {
            id: view,
            at,
            text: t.to_string(),
            size,
            align: Anchor::Start,
            mm: 1.0,
            rgb,
            bold: false,
        })
    };
    text(&mut items, DVec2::new(0.0, 5.0), title, 5.0, None);
    let mut y = -2.0;
    let trade = trade_of(model, view);
    // the site plan's legend: its networks alone
    let site = matches!(model.get(view).map(|e| &e.kind),
        Some(ElementKind::View(v)) if v.site_legend);
    let of = |t: Trade| !site && trade.is_none_or(|x| x == t);
    let mut systems = systems(model);
    systems.retain(|(s, _)| trade.is_none_or(|t| t.shows(s.kind)));
    if !systems.is_empty() {
        text(
            &mut items,
            DVec2::new(0.0, y - HEAD / 2.0),
            &tr("PIPES"),
            HEAD,
            None,
        );
        y -= HEAD + 3.0;
        for (s, types) in &systems {
            if s.kind.is_air() {
                duct_sample(
                    &mut items,
                    view,
                    s,
                    DVec2::new(0.0, y - LINE / 2.0),
                    SAMPLE,
                    false,
                );
            } else {
                sample(&mut items, view, s, DVec2::new(0.0, y - LINE / 2.0), SAMPLE);
            }
            let name = format!("{} - {}", tr(s.name), types.join(", "));
            for l in wrap(&name, tw, TEXT) {
                text(&mut items, DVec2::new(ind, y - LINE / 2.0), &l, TEXT, None);
                y -= LINE;
            }
            y -= 1.0;
        }
        // a duct's insulation, as the drawing marks it
        let insulated = systems.iter().find(|(s, _)| {
            s.kind.is_air()
                && model.iter().any(|e| {
                    matches!(&e.kind, ElementKind::Pipe(p)
                        if p.system == s.code && p.insulation_mm > 0.0)
                })
        });
        if let Some((s, _)) = insulated {
            duct_sample(
                &mut items,
                view,
                s,
                DVec2::new(0.0, y - LINE / 2.0),
                SAMPLE,
                true,
            );
            for l in wrap(
                &tr("duct insulation (thickness at the duct's size)"),
                tw,
                TEXT,
            ) {
                text(&mut items, DVec2::new(ind, y - LINE / 2.0), &l, TEXT, None);
                y -= LINE;
            }
            y -= 1.0;
        }
        y -= 3.0;
    }
    // the utility networks of the site, with the water and drains
    let nets = if site || of(Trade::Plumbing) {
        networks(model)
    } else {
        vec![]
    };
    if !nets.is_empty() {
        text(
            &mut items,
            DVec2::new(0.0, y - HEAD / 2.0),
            &tr("UTILITY NETWORKS"),
            HEAD,
            None,
        );
        y -= HEAD + 3.0;
        for n in &nets {
            net_sample(&mut items, view, n, DVec2::new(0.0, y - LINE / 2.0), SAMPLE);
            for l in wrap(&n.legend(), tw, TEXT) {
                text(&mut items, DVec2::new(ind, y - LINE / 2.0), &l, TEXT, None);
                y -= LINE;
            }
            y -= 1.0;
        }
        y -= 3.0;
    }
    // the site plan's own symbols placed: each drawn and named
    let marks = if site || of(Trade::Plumbing) {
        site_marks(model)
    } else {
        vec![]
    };
    if !marks.is_empty() {
        text(
            &mut items,
            DVec2::new(0.0, y - HEAD / 2.0),
            &tr("SITE SYMBOLS"),
            HEAD,
            None,
        );
        y -= HEAD + 3.0;
        for m in &marks {
            let lines = wrap(&m.legend(), tw, TEXT);
            let tall = (lines.len() as f64 * LINE).max(4.0);
            let at = DVec2::new(SYMBOL_BOX.x / 2.0, y - tall / 2.0);
            let strokes = crate::netlines::mark(*m, at, DVec2::Y, 1.0);
            items.extend(crate::netlines::paper_items(view, strokes, m.rgb(), 0.25));
            let mut ty = y - (tall - lines.len() as f64 * LINE) / 2.0;
            for l in lines {
                text(&mut items, DVec2::new(ind, ty - LINE / 2.0), &l, TEXT, None);
                ty -= LINE;
            }
            y -= tall + 1.5;
        }
        y -= 3.0;
    }
    // the site plan's objects that have their own mark: a water meter
    // shaft, a pumping shaft
    let marked = if site { site_objects(model) } else { vec![] };
    for (symbol, line) in &marked {
        let lines = wrap(line, tw, TEXT);
        let words = lines.len() as f64 * LINE;
        let row = DeviceRow {
            count: 1,
            name: String::new(),
            symbol: symbol.iter().flat_map(|s| s.segments()).collect(),
        };
        let tall = words.max(symbol_height(&row));
        if let Some((lo, hi, k)) = symbol_fit(&row) {
            let mid = (lo + hi) / 2.0;
            let centre = DVec2::new(SYMBOL_BOX.x / 2.0, y - tall / 2.0);
            let at = |p: DVec2| centre + (p - mid) * k;
            for s in symbol {
                match s {
                    creabim_core::objects::SymbolItem::Net {
                        points,
                        code,
                        dashed,
                    } => {
                        let rgb = creabim_core::utility::NetKind::of_code(code)
                            .map_or([0.3, 0.3, 0.3], |n| n.rgb());
                        let mut points: Vec<_> = points.iter().map(|p| at(*p)).collect();
                        points.extend(points.first().copied());
                        items.push(PlanItem::Curve {
                            id: view,
                            points,
                            rgb,
                            dashed: *dashed,
                        });
                    }
                    creabim_core::objects::SymbolItem::Text { at: p, text: t } => {
                        let rgb = symbol.iter().find_map(|s| match s {
                            creabim_core::objects::SymbolItem::Net { code, .. } => {
                                creabim_core::utility::NetKind::of_code(code).map(|n| n.rgb())
                            }
                            _ => None,
                        });
                        text(&mut items, at(*p), t, TEXT, rgb);
                    }
                    other => {
                        for (a, b) in other.segments() {
                            items.push(PlanItem::Line {
                                id: view,
                                pen: Pen::Projection,
                                a: at(a),
                                b: at(b),
                            });
                        }
                    }
                }
            }
        }
        let mut ty = y - (tall - words) / 2.0;
        for l in lines {
            text(&mut items, DVec2::new(ind, ty - LINE / 2.0), &l, TEXT, None);
            ty -= LINE;
        }
        y -= tall + 1.5;
    }
    if !marked.is_empty() {
        y -= 3.0;
    }
    let heated = model
        .iter()
        .any(|e| matches!(&e.kind, ElementKind::Room(r) if r.ufh.is_some()))
        && of(Trade::Heating);
    if heated {
        text(
            &mut items,
            DVec2::new(0.0, y - HEAD / 2.0),
            &tr("UNDERFLOOR HEATING"),
            HEAD,
            None,
        );
        y -= HEAD + 3.0;
        let pipe = {
            use creabim_services::floorheat;
            let s: floorheat::Settings =
                creabim_core::results::settings_of(model, "floorheat").unwrap_or_default();
            floorheat::model_pipe(model, &s).segment.replace(' ', "")
        };
        for (name, rgb, dashes) in [
            ("underfloor heating - supply", crate::heatplan::SUPPLY, None),
            (
                "underfloor heating - return",
                crate::heatplan::RETURN,
                Some(crate::heatplan::DASH),
            ),
        ] {
            let a = DVec2::new(0.0, y - LINE / 2.0);
            crate::heatplan::strips(
                &mut items,
                view,
                &[a, a + DVec2::X * SAMPLE],
                rgb,
                dashes,
                1.0,
            );
            text(
                &mut items,
                DVec2::new(ind, y - LINE / 2.0),
                &format!("{} {pipe}", tr(name)),
                TEXT,
                None,
            );
            y -= LINE + 1.0;
        }
        // the edge of a heated field
        let a = DVec2::new(0.0, y - LINE / 2.0);
        crate::heatplan::strips(
            &mut items,
            view,
            &[a, a + DVec2::X * SAMPLE],
            crate::heatplan::FIELD,
            Some((1.5, 1.0)),
            1.0,
        );
        text(
            &mut items,
            DVec2::new(ind, y - LINE / 2.0),
            &tr("edge of a heated field"),
            TEXT,
            None,
        );
        y -= LINE + 4.0;
    }
    let rows = if of(Trade::Plumbing) {
        creabim_services::fixturespec::legend(model, None)
    } else {
        vec![]
    };
    if !rows.is_empty() {
        text(
            &mut items,
            DVec2::new(0.0, y - HEAD / 2.0),
            &tr("SANITARY FIXTURES"),
            HEAD,
            None,
        );
        y -= HEAD + 3.0;
        for r in &rows {
            text(
                &mut items,
                DVec2::new(2.0, y - LINE / 2.0),
                &r.spec.mark,
                TEXT + 0.6,
                Some(crate::mep::LABEL_GREEN),
            );
            for line in creabim_services::fixturespec::legend_lines(&r.spec) {
                for l in wrap(&line, tw, TEXT) {
                    text(&mut items, DVec2::new(ind, y - LINE / 2.0), &l, TEXT, None);
                    y -= LINE;
                }
            }
            y -= 1.5;
        }
    }
    let equipment = equipment(model, trade);
    if !equipment.is_empty() {
        y -= 1.5;
        text(
            &mut items,
            DVec2::new(0.0, y - HEAD / 2.0),
            &tr("EQUIPMENT"),
            HEAD,
            None,
        );
        y -= HEAD + 3.0;
        for (lead, name) in &equipment {
            text(
                &mut items,
                DVec2::new(2.0, y - LINE / 2.0),
                lead,
                TEXT + 0.6,
                Some(crate::mep::LABEL_GREEN),
            );
            for l in wrap(name, tw, TEXT) {
                text(&mut items, DVec2::new(ind, y - LINE / 2.0), &l, TEXT, None);
                y -= LINE;
            }
            y -= 1.5;
        }
    }
    // the devices on the ducts, by their position numbers in the plan
    let air = if of(Trade::Ventilation) {
        crate::mep::air_positions(model)
    } else {
        vec![]
    };
    if !air.is_empty() {
        y -= 1.5;
        text(
            &mut items,
            DVec2::new(0.0, y - HEAD / 2.0),
            &tr("VENTILATION DEVICES"),
            HEAD,
            None,
        );
        y -= HEAD + 3.0;
        for a in &air {
            text(
                &mut items,
                DVec2::new(2.0, y - LINE / 2.0),
                &a.number.to_string(),
                TEXT + 0.6,
                Some(crate::mep::LABEL_GREEN),
            );
            let name = format!("{}, {} {}", a.name, a.valves.len(), tr("pcs"));
            for l in wrap(&name, tw, TEXT) {
                text(&mut items, DVec2::new(ind, y - LINE / 2.0), &l, TEXT, None);
                y -= LINE;
            }
            y -= 1.5;
        }
    }
    let devices = if of(Trade::Electrical) {
        devices(model)
    } else {
        vec![]
    };
    if !devices.is_empty() {
        y -= 1.5;
        text(
            &mut items,
            DVec2::new(0.0, y - HEAD / 2.0),
            &tr("ELECTRICAL DEVICES"),
            HEAD,
            None,
        );
        y -= HEAD + 3.0;
        for row in &devices {
            let name = format!("{}, {} {}", row.name, row.count, tr("pcs"));
            let lines = wrap(&name, tw, TEXT);
            let words = lines.len() as f64 * LINE;
            let tall = words.max(symbol_height(row));
            device_symbol(
                &mut items,
                view,
                row,
                DVec2::new(SYMBOL_BOX.x / 2.0, y - tall / 2.0),
            );
            let mut ty = y - (tall - words) / 2.0;
            for l in lines {
                text(&mut items, DVec2::new(ind, ty - LINE / 2.0), &l, TEXT, None);
                ty -= LINE;
            }
            y -= tall + 1.5;
        }
    }
    let notes = notes(model, view);
    if !notes.is_empty() {
        y -= 1.5;
        text(
            &mut items,
            DVec2::new(0.0, y - HEAD / 2.0),
            &format!("{}:", tr("NOTE")),
            HEAD,
            None,
        );
        y -= HEAD + 3.0;
        for n in &notes {
            let n = format!("- {}", n.trim_start_matches("- "));
            for l in wrap(&n.to_uppercase(), ind + tw, TEXT) {
                text(&mut items, DVec2::new(0.0, y - LINE / 2.0), &l, TEXT, None);
                y -= LINE;
            }
            y -= 1.0;
        }
        // the project's zero above the sea, as the office's legends close
        let zero = model.project_info().map_or(0.0, |p| p.1.elevation);
        if zero.abs() > 1e-9 {
            y -= 1.5;
            text(
                &mut items,
                DVec2::new(0.0, y - LINE / 2.0),
                &format!("±0,000 = {}", format!("{zero:.2}").replace('.', ",")),
                TEXT + 0.6,
                None,
            );
            y -= LINE + 0.6;
        }
        // a heating drawing's design outside temperature
        if heated {
            y -= 1.5;
            let te = creabim_services::heatloss::Opts::default().te;
            text(
                &mut items,
                DVec2::new(0.0, y - LINE / 2.0),
                &format!("te = {te:.0} °C"),
                TEXT + 0.6,
                None,
            );
        }
    }
    if systems.is_empty()
        && rows.is_empty()
        && !heated
        && equipment.is_empty()
        && devices.is_empty()
        && air.is_empty()
        && marked.is_empty()
    {
        text(
            &mut items,
            DVec2::new(0.0, y - LINE / 2.0),
            &tr("No pipes or sanitary fixtures in the model yet."),
            TEXT,
            None,
        );
    }
    PlanView { items }
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::mep::Pipe;
    use creabim_core::{Document, Element, Transaction};
    use glam::DVec3;

    /// Under the fixtures the legend writes its notes: the usual ones
    /// of the pipes drawn, or the view's own, a line each.
    #[test]
    fn the_legend_closes_with_notes_its_own_or_the_usual() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let legend = Element::new(
            "Legend",
            ElementKind::View(creabim_core::View::schedule(
                creabim_core::ScheduleCategory::ServicesLegend,
            )),
        );
        let id = legend.id;
        let pipe = Element::new(
            "Pipe",
            ElementKind::Pipe(Pipe {
                level,
                system: "SV".into(),
                start: DVec3::ZERO,
                end: DVec3::X,
                pipe_type: "HERZ".into(),
                nominal_mm: 16.0,
                outer_mm: 20.0,
                inner_mm: 16.0,
                height_mm: None,
                insulation_mm: 0.0,
                insulation_set: false,
                mark: None,
            }),
        );
        assert!(notes(doc.model(), id).is_empty());
        doc.commit(Transaction::new("t").insert(legend.clone()).insert(pipe))
            .unwrap();
        // water pipes: insulation, sleeves, the public mains
        assert_eq!(notes(doc.model(), id).len(), 3);
        let texts = |m: &Model| {
            drawing(id, "Legend", m)
                .items
                .iter()
                .filter_map(|i| match i {
                    PlanItem::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert!(texts(doc.model()).iter().any(|t| t == "NOTE:"));
        let mut own = legend.clone();
        if let ElementKind::View(v) = &mut own.kind {
            v.notes = "First\n\n- Second".into();
        }
        doc.commit(Transaction::new("t").replace(legend, own))
            .unwrap();
        assert_eq!(notes(doc.model(), id), ["First", "- Second"]);
        let t = texts(doc.model());
        assert!(t.iter().any(|t| t == "- FIRST") && t.iter().any(|t| t == "- SECOND"));
    }

    #[test]
    fn the_legend_lists_the_systems_in_use_with_their_lines() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let view = ElementId::new();
        assert!(systems(doc.model()).is_empty());
        let mut tx = Transaction::new("pipes");
        for system in ["TV", "SV", "SV"] {
            tx = tx.insert(Element::new(
                "Pipe",
                ElementKind::Pipe(Pipe {
                    level,
                    system: system.into(),
                    start: DVec3::ZERO,
                    end: DVec3::X,
                    pipe_type: "HERZ".into(),
                    nominal_mm: 16.0,
                    outer_mm: 20.0,
                    inner_mm: 16.0,
                    height_mm: None,
                    insulation_mm: 0.0,
                    insulation_set: false,
                    mark: None,
                }),
            ));
        }
        doc.commit(tx).unwrap();
        let ss = systems(doc.model());
        // in the systems' order, each once, with its pipe type's name
        assert_eq!(
            ss.iter().map(|(s, _)| s.code).collect::<Vec<_>>(),
            vec!["SV", "TV"]
        );
        assert_eq!(ss[0].1.len(), 1);
        let d = drawing(view, "Legend", doc.model());
        let texts: Vec<&str> = d
            .items
            .iter()
            .filter_map(|i| match i {
                PlanItem::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(texts[..2], ["Legend", "PIPES"]);
        assert!(texts[2].starts_with("Cold water - "));
        // cold water's sample: dashes and dots in its blue
        let strips = d
            .items
            .iter()
            .filter(|i| matches!(i, PlanItem::Shade { rgb, .. } if rgb[2] > 0.8))
            .count();
        assert!(strips >= 4, "{strips}");
    }

    #[test]
    fn the_electrical_legend_draws_each_device_with_its_symbol() {
        let m = creabim_core::starter::starter_house();
        let rows = devices(&m);
        assert!(rows.len() > 3);
        // each type once, counted, with the strokes the plans draw
        assert!(rows.iter().all(|r| r.count > 0 && !r.symbol.is_empty()));
        let socket = rows.iter().find(|r| r.name.contains("Double socket"));
        assert!(socket.is_some_and(|r| r.count > 1));
        // a small symbol keeps the plans' scale, a large one shrinks to
        // its row
        for r in &rows {
            let (lo, hi, k) = symbol_fit(r).unwrap();
            assert!(k <= SYMBOL_SCALE + 1e-9);
            assert!((hi.x - lo.x) * k <= SYMBOL_BOX.x + 1e-9);
            assert!(symbol_height(r) <= SYMBOL_BOX.y + 1e-9);
        }
    }

    #[test]
    fn the_site_networks_have_their_lines_in_the_legend() {
        use creabim_core::utility::{NetKind, NetState, UtilityLine};
        use creabim_core::{Element, Sketch, Transaction, Workplane};
        let mut m = Model::new();
        let mut tx = Transaction::new("nets");
        for (kind, state) in [
            (NetKind::FoulSewer, NetState::Proposed),
            (NetKind::Water, NetState::Existing),
            (NetKind::FoulSewer, NetState::Proposed),
        ] {
            let mut sk = Sketch::polyline(
                None,
                Workplane::ground(0.0),
                vec![DVec2::ZERO, DVec2::new(5.0, 0.0)],
                false,
            );
            sk.net = Some(UtilityLine {
                kind,
                state,
                note: String::new(),
            });
            tx = tx.insert(Element::new("n", ElementKind::Sketch(sk)));
        }
        tx.apply(&mut m).unwrap();
        let nets = networks(&m);
        assert_eq!(nets.len(), 2);
        assert_eq!(nets[0].kind, NetKind::Water);
        let d = drawing(ElementId::default(), "Legend", &m);
        let texts: Vec<String> = d
            .items
            .iter()
            .filter_map(|i| match i {
                PlanItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|t| t == "UTILITY NETWORKS"), "{texts:?}");
        // the site plan's legend: the networks alone
        let mut v = creabim_core::View::schedule(creabim_core::ScheduleCategory::ServicesLegend);
        v.site_legend = true;
        let ve = Element::new("Site legend", ElementKind::View(v));
        let vid = ve.id;
        Transaction::new("v").insert(ve).apply(&mut m).unwrap();
        let d = drawing(vid, "Legend", &m);
        let heads: Vec<String> = d
            .items
            .iter()
            .filter_map(|i| match i {
                PlanItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(heads.iter().any(|t| t == "UTILITY NETWORKS"));
        assert!(!heads.iter().any(|t| t == "PIPES"));
        // a pumping shaft on the site: its circle in the sewer's colour,
        // its mark and its line
        let le = Element::new(
            "L",
            ElementKind::Level(creabim_core::Level {
                elevation: 0.0,
                height: 3.0,
            }),
        );
        let level = le.id;
        Transaction::new("l").insert(le).apply(&mut m).unwrap();
        let ps = creabim_core::catalogue::starter("starter/site-pumping-shaft-laura-l").unwrap();
        for at in [DVec2::new(10.0, 0.0), DVec2::new(14.0, 0.0)] {
            let (tx, _) = creabim_core::objects::place(&m, &ps, level, at, 0.0);
            tx.apply(&mut m).unwrap();
        }
        assert_eq!(site_objects(&m).len(), 1, "each type once");
        let d = drawing(vid, "Legend", &m);
        let texts: Vec<String> = d
            .items
            .iter()
            .filter_map(|i| match i {
                PlanItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|t| t == "PS PRŠ"), "{texts:?}");
        assert!(texts.iter().any(|t| t.contains("NORIA LUCA")), "{texts:?}");
        assert!(d.items.iter().any(|i| matches!(i, PlanItem::Curve { .. })));
    }

    #[test]
    fn the_vzt_legend_numbers_the_devices_and_marks_the_insulation() {
        let mut doc = Document::from_model(creabim_core::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let duct = Element::new(
            "Duct",
            ElementKind::Pipe(Pipe {
                level,
                system: "VZO".into(),
                start: DVec3::new(0.0, 0.0, 2.5),
                end: DVec3::new(2.0, 0.0, 2.5),
                pipe_type: "VZK".into(),
                nominal_mm: 100.0,
                outer_mm: 100.0,
                inner_mm: 100.0,
                height_mm: None,
                insulation_mm: 30.0,
                insulation_set: true,
                mark: None,
            }),
        );
        let mut fan = creabim_core::mep::Valve::new(duct.id, 0.5, "ventilator");
        fan.product = Some("SP-TD160-100-SILENT".into());
        let fan = Element::new("Fan", ElementKind::Valve(fan));
        let mut legend = Element::new(
            "Legenda VZT",
            ElementKind::View(creabim_core::View::schedule(
                creabim_core::ScheduleCategory::ServicesLegend,
            )),
        );
        if let ElementKind::View(v) = &mut legend.kind {
            v.trade = Some(Trade::Ventilation);
        }
        let id = legend.id;
        doc.commit(
            Transaction::new("t")
                .insert(duct)
                .insert(fan)
                .insert(legend),
        )
        .unwrap();
        let pos = crate::mep::air_positions(doc.model());
        assert_eq!(pos.len(), 1);
        assert_eq!(pos[0].number, 1);
        assert!(pos[0].name.contains("TD-160/100"));
        let texts: Vec<String> = drawing(id, "Legend", doc.model())
            .items
            .iter()
            .filter_map(|i| match i {
                PlanItem::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(texts.iter().any(|t| t == "VENTILATION DEVICES"));
        assert!(texts.iter().any(|t| t.contains("insulation")));
    }
}
