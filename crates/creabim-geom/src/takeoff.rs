//! The material take-off as a table: every material of every element on
//! its own, a layered wall, slab or roof giving one line for each of its
//! layers, grouped by element type, level, construction or material, as
//! the view's [`Layout`] asks.
//!
//! Volumes are measured on the solids as the 3D view and the drawings
//! show them, after joins and cuts (a slab running through a wall, a
//! column standing in it, a wall cut by a roof): the "cleaned up"
//! quantities a builder orders by. Where a solid can't be measured (it is
//! not built, or comes out empty) the line keeps its quantity worked out
//! from its size.

use crate::schedule::{Column, Field, Row, Schedule};
use creabim_core::takeoff::{self, GroupBy, Layout, Line, Quantity, CATEGORY_ORDER};
use creabim_core::words::tr;
use creabim_core::{ElementId, Model, ScheduleCategory};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Categories whose volume is measured on their trimmed solids.
const TRIMMED: [&str; 10] = [
    "Walls",
    "Slabs",
    "Foundation slabs",
    "Roofs",
    "Ceilings",
    "Columns",
    "Beams",
    "Lintels",
    "Footings",
    "Stairs",
];

use crate::scene::closed_volume as mesh_volume;

/// The take-off lines with their volumes measured on `scene`.
pub fn measure(model: &Model, scene: &[crate::scene::SceneItem]) -> Vec<Line> {
    let mut lines = takeoff::lines(model);
    // each solid's volume and the volume of its box: no closed solid holds
    // more than its box, a boolean that came out broken may
    // keyed by material too: a layer may hold parts of another material
    // (the mineral wool fire barriers in an ETICS of EPS)
    type Key = (ElementId, Option<usize>, Option<ElementId>);
    let mut solid: HashMap<Key, (f64, f64)> = HashMap::new();
    // the plinths' pieces counted on their own lines
    let mut plinths = crate::plinthtakeoff::Pieces::new(model);
    for item in scene.iter().filter(|i| !i.ghost && !i.is_lining()) {
        if plinths.take(item) {
            continue;
        }
        let room = item.mesh.bounds().map_or(0.0, |(lo, hi)| {
            let d = (hi - lo).as_dvec3();
            d.x * d.y * d.z
        });
        let e = solid
            .entry((
                item.id,
                item.layer.map(|l| l.index),
                item.layer.and_then(|l| l.material),
            ))
            .or_default();
        e.0 += mesh_volume(&item.mesh);
        e.1 += room;
    }
    // a part of a layer in another material than the layer's: a line of
    // its own, like the layer's
    let mut more: Vec<Line> = vec![];
    for &(id, layer, material) in solid.keys() {
        let Some(layer) = layer else { continue };
        let known = |l: &&Line| l.element == id && l.layer.map(|x| x.0) == Some(layer);
        if lines.iter().filter(known).any(|l| l.material == material) {
            continue;
        }
        if let Some(l) = lines.iter().find(known) {
            let mut extra = l.clone();
            extra.material = material;
            more.push(extra);
        }
    }
    more.sort_by_key(|l| (l.element, l.layer.map(|x| x.0)));
    lines.extend(more);
    // a layer wholly in another material (an ETICS over 22.5 m all of
    // mineral wool): its own material's line is gone
    lines.retain(|l| {
        let Some((layer, _)) = l.layer else {
            return true;
        };
        let key = (l.element, Some(layer), l.material);
        solid.contains_key(&key) || !solid.keys().any(|k| k.0 == l.element && k.1 == Some(layer))
    });
    for l in lines.iter_mut().filter(|l| TRIMMED.contains(&l.category)) {
        // an element of one material (not layered) is one solid, keyed
        // without it
        let Some((v, room)) = solid
            .get(&(l.element, l.layer.map(|x| x.0), l.material))
            .or_else(|| {
                l.layer
                    .is_none()
                    .then(|| solid.get(&(l.element, None, None)))
                    .flatten()
            })
            .copied()
        else {
            continue;
        };
        if v <= 1e-9 || v > room * 1.01 + 1e-6 {
            continue;
        }
        l.quantity.volume = v;
        // a wall's, slab's or roof's face as trimmed: its gable under the
        // roof in, what a slab or column took out of it out
        if PLANAR.contains(&l.category) && l.thickness > 1e-4 {
            l.quantity.area = v / l.thickness;
        }
    }
    lines.extend(plinths.lines());
    split_waterproofing(model, &mut lines);
    label_footings(model, &mut lines);
    lines
}

/// Footings told apart in the take-off (George 2026-11-25): a strip by
/// its width (a line for each width), a pad by its size, a pile by its
/// diameter and length.
pub fn label_footings(model: &Model, lines: &mut [Line]) {
    let mm = |v: f64| format!("{:.0}", v * 1000.0);
    for l in lines.iter_mut() {
        if l.category != "Footings" || l.label.is_some() {
            continue;
        }
        let Some(creabim_core::ElementKind::Footing(f)) = model.get(l.element).map(|e| &e.kind)
        else {
            continue;
        };
        let name = name_of(model, l.material).unwrap_or_else(|| tr("No material"));
        let what = match f.kind {
            creabim_core::FootingKind::Strip => {
                tr("{m} – strip footing w. {w} mm").replace("{w}", &mm(f.width))
            }
            creabim_core::FootingKind::Pad => tr("{m} – pad footing {a} × {b} mm")
                .replace("{a}", &mm(f.width))
                .replace("{b}", &mm(f.length)),
            creabim_core::FootingKind::Pile => tr("{m} – pile Ø {d} mm, length {h} mm")
                .replace("{d}", &mm(f.width))
                .replace("{h}", &mm(f.thickness)),
        };
        l.label = Some(what.replace("{m}", &name));
    }
}

/// Waterproofing listed apart laid flat and turned up a wall: Cenkros
/// prices horizontal and vertical waterproofing as items of their own
/// (George 2026-11-25). A line of a waterproofing material (its hatch is
/// the membrane strip) under a wall, a plinth or a column is vertical,
/// under a slab or a footing horizontal; its rate stays its material's.
pub fn split_waterproofing(model: &Model, lines: &mut [Line]) {
    for l in lines.iter_mut() {
        if l.label.is_some() {
            continue;
        }
        let Some(m) = l.material else { continue };
        let Some(creabim_core::ElementKind::Material(mat)) = model.get(m).map(|e| &e.kind) else {
            continue;
        };
        if mat.hatch != creabim_core::Hatch::Membrane {
            continue;
        }
        let way = match l.category {
            "Walls" | "Plinths" | "Columns" => "{m} – vertical",
            "Slabs" | "Foundation slabs" | "Footings" => "{m} – horizontal",
            _ => continue,
        };
        let name = name_of(model, Some(m)).unwrap_or_default();
        l.label = Some(tr(way).replace("{m}", &name));
    }
}

/// Categories whose area is their trimmed volume over their thickness.
const PLANAR: [&str; 6] = [
    "Walls",
    "Slabs",
    "Foundation slabs",
    "Roofs",
    "Ceilings",
    "Footings",
];

type Cache = Vec<(u64, Arc<Vec<Line>>)>;
static MEASURED: Mutex<Cache> = Mutex::new(Vec::new());

fn remember(rev: u64, lines: Arc<Vec<Line>>) {
    if rev == 0 {
        return;
    }
    let mut c = MEASURED.lock().unwrap_or_else(|e| e.into_inner());
    c.retain(|(r, _)| *r != rev);
    if c.len() >= 2 {
        c.remove(0);
    }
    c.push((rev, lines));
}

fn cached(rev: u64) -> Option<Arc<Vec<Line>>> {
    let c = MEASURED.lock().unwrap_or_else(|e| e.into_inner());
    c.iter()
        .find(|(r, _)| *r == rev && rev != 0)
        .map(|x| x.1.clone())
}

/// The measured take-off lines of `model`, once per revision.
pub fn lines(model: &Model) -> Arc<Vec<Line>> {
    let rev = model.revision();
    if let Some(l) = cached(rev) {
        return l;
    }
    let scene = crate::scene::shared_scene(model);
    let l = Arc::new(measure(model, &scene));
    remember(rev, l.clone());
    l
}

/// The measured lines if they can be had without building the 3D scene
/// here, and true; else the lines measured last (or, before any, from the
/// elements' sizes) and false: a table shown live asks for the scene to be
/// built and shows the fresh ones once it is.
pub fn lines_now(model: &Model) -> (Arc<Vec<Line>>, bool) {
    let rev = model.revision();
    if let Some(l) = cached(rev) {
        return (l, true);
    }
    if let Some(scene) = crate::scene::shared_scene_ready(model) {
        let l = Arc::new(measure(model, &scene));
        remember(rev, l.clone());
        return (l, true);
    }
    // the last measured, while it is of this model
    let last = {
        let c = MEASURED.lock().unwrap_or_else(|e| e.into_inner());
        c.last().map(|x| x.1.clone())
    };
    match last {
        Some(l) if l.iter().all(|x| model.get(x.element).is_some()) => (l, false),
        _ => (Arc::new(takeoff::lines(model)), false),
    }
}

/// A take-off table and, for each of its rows, what the app needs to work
/// with it.
#[derive(Clone, Debug)]
pub struct Table {
    pub schedule: Schedule,
    /// The elements each row stands for (a group's: all in it).
    pub elements: Vec<Vec<ElementId>>,
    /// The material of each row; `None` for a group or a line without one.
    pub material: Vec<Option<ElementId>>,
    /// How deep each row's group is (0 outermost); a row of a group heading
    /// is its group's depth, a material's row is one deeper than its group.
    pub depth: Vec<usize>,
    /// Each row's group path: a heading's own, a material row's group's.
    pub path: Vec<String>,
    /// The volumes were measured on the trimmed solids.
    pub measured: bool,
}

/// A value to sort a row by.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
enum Key {
    Num(f64),
    Text(String),
}

/// The name a line's element is listed under as a construction: its
/// type's, else its assembly's, else its own.
fn construction(model: &Model, id: ElementId) -> String {
    let Some(e) = model.get(id) else {
        return String::new();
    };
    if let Some((t, _)) = creabim_core::types::type_of(model, e) {
        return t.name.clone();
    }
    if let Some((a, _)) = creabim_core::assembly_of(model, e) {
        return a.name.clone();
    }
    e.name.clone()
}

fn name_of(model: &Model, id: Option<ElementId>) -> Option<String> {
    id.and_then(|i| model.get(i)).map(|e| tr(&e.name))
}

/// The group a line falls in by `g`: a key to order groups by and its
/// label.
fn group_of(model: &Model, l: &Line, g: GroupBy) -> (Key, String) {
    match g {
        GroupBy::Category => {
            let i = CATEGORY_ORDER
                .iter()
                .position(|c| *c == l.category)
                .unwrap_or(CATEGORY_ORDER.len());
            (Key::Num(i as f64), tr(l.category))
        }
        GroupBy::Level => {
            let z = l
                .level
                .and_then(|id| model.level(id))
                .map_or(f64::MAX, |lv| lv.elevation);
            (
                Key::Num(z),
                name_of(model, l.level).unwrap_or_else(|| tr("No level")),
            )
        }
        GroupBy::Construction => {
            let n = construction(model, l.element);
            (Key::Text(n.to_lowercase()), n)
        }
        GroupBy::Material => {
            let n = material_label(model, l);
            (Key::Text(n.to_lowercase()), n)
        }
    }
}

/// A line's material by name; a door, window or object without one by
/// its construction.
fn material_label(model: &Model, l: &Line) -> String {
    if let Some(label) = &l.label {
        return label.clone();
    }
    name_of(model, l.material).unwrap_or_else(|| {
        if matches!(l.category, "Doors" | "Windows" | "Objects" | "Railings") {
            construction(model, l.element)
        } else {
            tr("No material")
        }
    })
}

fn mm(t: f64) -> String {
    if t > 1e-9 {
        format!("{:.0}", t * 1000.0)
    } else {
        "-".into()
    }
}

fn line_cost(model: &Model, material: Option<ElementId>, q: &Quantity) -> Option<f64> {
    let r = takeoff::rate_of(model, material?)?;
    Some(r.amount * r.per.of(q))
}

/// One column of the table: its heading, editor and paper width.
fn col(title: &'static str, field: Option<Field>, width: f64, numeric: bool) -> Column {
    Column {
        title,
        field,
        width,
        numeric,
        align: None,
    }
}

/// The columns of a table laid out by `layout` grouped by `groups`.
fn columns(layout: &Layout, groups: &[GroupBy], perimeter: bool) -> Vec<Column> {
    let mut c = vec![
        col("Material", None, 44.0, false),
        col("Thickness", None, 16.0, true),
    ];
    if layout.detail {
        c.push(col("Element", None, 30.0, false));
        if !groups.contains(&GroupBy::Construction) {
            c.push(col("Construction", None, 40.0, false));
        }
        if !groups.contains(&GroupBy::Level) {
            c.push(col("Level", None, 22.0, false));
        }
        if !groups.contains(&GroupBy::Category) {
            c.push(col("Element type", None, 22.0, false));
        }
        c.push(col("Layer", None, 26.0, false));
    } else {
        c.push(col("Elements", None, 14.0, true));
    }
    c.push(col("Length", None, 18.0, true));
    // doors' and windows' perimeters, when there are any
    if perimeter {
        c.push(col("Perimeter", None, 18.0, true));
    }
    c.extend([
        col("Area", None, 20.0, true),
        col("Volume", None, 20.0, true),
        col("Rate", Some(Field::Rate), 18.0, true),
        col("Per", Some(Field::RatePer), 14.0, false),
        col("Cost", None, 20.0, true),
    ]);
    c
}

/// A row of materials before it is placed under its group.
struct Leaf {
    material: Option<ElementId>,
    elements: Vec<ElementId>,
    cells: HashMap<&'static str, (String, Key)>,
}

/// A perimeter's cell: metres, or a dash for none.
fn perimeter_cell(p: f64) -> (String, Key) {
    if p > 1e-9 {
        (format!("{p:.2} m"), Key::Num(p))
    } else {
        ("-".into(), Key::Num(0.0))
    }
}

fn leaf_cells(
    model: &Model,
    label: String,
    thickness: f64,
    material: Option<ElementId>,
    q: &Quantity,
    elements: usize,
) -> HashMap<&'static str, (String, Key)> {
    let rate = material.and_then(|m| takeoff::rate_of(model, m));
    let cost = line_cost(model, material, q);
    let mut c = HashMap::new();
    let num = |v: f64, unit: &str| (format!("{v:.2} {unit}"), Key::Num(v));
    c.insert("Material", (label.clone(), Key::Text(label.to_lowercase())));
    c.insert("Thickness", (mm(thickness), Key::Num(thickness)));
    c.insert(
        "Elements",
        (elements.to_string(), Key::Num(elements as f64)),
    );
    c.insert(
        "Length",
        if q.length > 1e-9 {
            num(q.length, "m")
        } else {
            ("-".into(), Key::Num(0.0))
        },
    );
    c.insert(
        "Area",
        if q.area > 1e-9 {
            num(q.area, "m²")
        } else {
            ("-".into(), Key::Num(0.0))
        },
    );
    c.insert(
        "Volume",
        if q.volume > 1e-9 {
            num(q.volume, "m³")
        } else {
            ("-".into(), Key::Num(0.0))
        },
    );
    c.insert(
        "Rate",
        rate.map_or(("-".into(), Key::Num(0.0)), |r| {
            (format!("{:.2}", r.amount), Key::Num(r.amount))
        }),
    );
    c.insert(
        "Per",
        rate.map_or(("-".into(), Key::Text(String::new())), |r| {
            (r.per.label().to_string(), Key::Text(r.per.label().into()))
        }),
    );
    c.insert(
        "Cost",
        cost.map_or(("-".into(), Key::Num(0.0)), |v| {
            (format!("{v:.2}"), Key::Num(v))
        }),
    );
    c
}

/// The rows of `lines`, one group's: each material (and thickness) added
/// up, or each line on its own with `detail`.
fn leaves(model: &Model, lines: &[&Line], detail: bool) -> Vec<Leaf> {
    if detail {
        return lines
            .iter()
            .map(|l| {
                let mut c = leaf_cells(
                    model,
                    material_label(model, l),
                    l.thickness,
                    l.material,
                    &l.quantity,
                    1,
                );
                c.insert("Perimeter", perimeter_cell(l.perimeter));
                let e = model.get(l.element);
                let element = e.map(|e| e.name.clone()).unwrap_or_default();
                c.insert(
                    "Element",
                    (element.clone(), Key::Text(element.to_lowercase())),
                );
                let k = construction(model, l.element);
                c.insert("Construction", (k.clone(), Key::Text(k.to_lowercase())));
                let (lk, ln) = group_of(model, l, GroupBy::Level);
                c.insert("Level", (ln, lk));
                let (ck, cn) = group_of(model, l, GroupBy::Category);
                c.insert("Element type", (cn, ck));
                let layer = l.layer.map_or(("-".to_string(), Key::Num(-1.0)), |(i, f)| {
                    (format!("{} · {}", i + 1, tr(f.label())), Key::Num(i as f64))
                });
                c.insert("Layer", layer);
                Leaf {
                    material: l.material,
                    elements: vec![l.element],
                    cells: c,
                }
            })
            .collect();
    }
    // (material or label, thickness in tenths of a millimetre)
    type LeafKey = (Option<ElementId>, String, i64);
    let mut order: Vec<LeafKey> = vec![];
    let mut sums: HashMap<LeafKey, (Quantity, Vec<ElementId>, f64, f64)> = HashMap::new();
    for l in lines {
        let label = material_label(model, l);
        let key = (
            l.material,
            // pieces (sills, thresholds) by their size too
            if l.material.is_some() && l.label.is_none() {
                String::new()
            } else {
                label
            },
            (l.thickness * 10_000.0).round() as i64,
        );
        let e = sums.entry(key.clone()).or_insert_with(|| {
            order.push(key);
            (Quantity::default(), vec![], l.thickness, 0.0)
        });
        e.3 += l.perimeter;
        e.0.area += l.quantity.area;
        e.0.volume += l.quantity.volume;
        e.0.length += l.quantity.length;
        if !e.1.contains(&l.element) {
            e.1.push(l.element);
        }
    }
    order
        .into_iter()
        .filter_map(|k| {
            let (mut q, elements, t, perimeter) = sums.remove(&k)?;
            q.count = elements.len();
            let label = if k.1.is_empty() {
                name_of(model, k.0).unwrap_or_default()
            } else {
                k.1
            };
            let mut cells = leaf_cells(model, label, t, k.0, &q, elements.len());
            cells.insert("Perimeter", perimeter_cell(perimeter));
            Some(Leaf {
                material: k.0,
                cells,
                elements,
            })
        })
        .collect()
}

/// The take-off of `model` as a table laid out by `layout`; `by_level` is
/// the older switch (see [`Layout::groups`]). Only lines whose material,
/// element or construction holds `filter` (any case) are listed.
pub fn table(model: &Model, layout: &Layout, by_level: bool, filter: &str) -> Table {
    let (all, measured) = lines_now(model);
    table_of(model, &all, measured, layout, by_level, filter)
}

/// [`table`] with the volumes measured on the 3D scene, waiting for it to
/// be built: for sheets and exports.
pub fn table_measured(model: &Model, layout: &Layout, by_level: bool) -> Table {
    let all = lines(model);
    table_of(model, &all, true, layout, by_level, "")
}

fn table_of(
    model: &Model,
    all: &[Line],
    measured: bool,
    layout: &Layout,
    by_level: bool,
    filter: &str,
) -> Table {
    let groups = layout.groups(by_level);
    let cols = columns(layout, &groups, all.iter().any(|l| l.perimeter > 1e-9));
    let needle = filter.trim().to_lowercase();
    let picked: Vec<&Line> = all
        .iter()
        .filter(|l| {
            needle.is_empty()
                || [
                    material_label(model, l),
                    construction(model, l.element),
                    model
                        .get(l.element)
                        .map(|e| e.name.clone())
                        .unwrap_or_default(),
                    tr(l.category),
                ]
                .iter()
                .any(|s| s.to_lowercase().contains(&needle))
        })
        .collect();
    let mut t = Table {
        schedule: Schedule {
            category: ScheduleCategory::Materials,
            columns: cols,
            rows: vec![],
            totals: vec![],
            listed: 0,
            above_totals: vec![],
        },
        elements: vec![],
        material: vec![],
        depth: vec![],
        path: vec![],
        measured,
    };
    let sort = layout.sort.as_deref().and_then(|s| {
        t.schedule
            .columns
            .iter()
            .any(|c| c.title == s)
            .then_some(s.to_string())
    });
    build(
        model,
        &picked,
        &groups,
        0,
        String::new(),
        layout,
        sort.as_deref(),
        &mut t,
    );
    // totals: what everything listed comes to
    let materials: std::collections::HashSet<Option<ElementId>> =
        picked.iter().map(|l| l.material).collect();
    let volume: f64 = picked.iter().map(|l| l.quantity.volume).sum();
    let costs: Vec<f64> = picked
        .iter()
        .filter_map(|l| line_cost(model, l.material, &l.quantity))
        .collect();
    let mut totals = vec![String::new(); t.schedule.columns.len()];
    totals[0] = format!("{} {}", materials.len(), tr("materials"));
    for (i, c) in t.schedule.columns.iter().enumerate() {
        match c.title {
            "Volume" => totals[i] = format!("{volume:.2} m³"),
            "Cost" => {
                totals[i] = if costs.is_empty() {
                    "-".into()
                } else {
                    format!("{:.2}", costs.iter().sum::<f64>())
                }
            }
            "Elements" => {
                let n: std::collections::HashSet<ElementId> =
                    picked.iter().map(|l| l.element).collect();
                totals[i] = n.len().to_string();
            }
            _ => {}
        }
    }
    t.schedule.totals = totals;
    t.schedule.listed = t.schedule.rows.iter().filter(|r| !r.group).count();
    t
}

/// Indentation of a group `depth` deep, as a sheet writes it.
fn indent(depth: usize) -> String {
    "    ".repeat(depth)
}

#[allow(clippy::too_many_arguments)]
fn build(
    model: &Model,
    lines: &[&Line],
    groups: &[GroupBy],
    depth: usize,
    path: String,
    layout: &Layout,
    sort: Option<&str>,
    t: &mut Table,
) {
    let titles: Vec<&'static str> = t.schedule.columns.iter().map(|c| c.title).collect();
    let Some((g, rest)) = groups.split_first() else {
        let mut ls = leaves(model, lines, layout.detail);
        if let Some(s) = sort {
            ls.sort_by(|a, b| {
                let (ka, kb) = (&a.cells[s].1, &b.cells[s].1);
                let o = ka.partial_cmp(kb).unwrap_or(std::cmp::Ordering::Equal);
                if layout.descending {
                    o.reverse()
                } else {
                    o
                }
            });
        }
        for l in ls {
            let mut cells: Vec<String> = titles
                .iter()
                .map(|c| l.cells.get(c).map(|x| x.0.clone()).unwrap_or_default())
                .collect();
            if depth > 0 {
                cells[0] = format!("{}{}", indent(depth), cells[0]);
            }
            t.schedule.rows.push(Row {
                id: l.material.unwrap_or(ElementId::NIL),
                cells,
                group: false,
            });
            t.elements.push(l.elements);
            t.material.push(l.material);
            t.depth.push(depth);
            t.path.push(path.clone());
        }
        return;
    };
    // the groups in their order, each with its lines in theirs
    let mut keyed: Vec<(Key, String, Vec<&Line>)> = vec![];
    for l in lines {
        let (k, label) = group_of(model, l, *g);
        match keyed.iter_mut().find(|x| x.1 == label) {
            Some(x) => x.2.push(l),
            None => keyed.push((k, label, vec![l])),
        }
    }
    keyed.sort_by(|a, b| {
        a.0.partial_cmp(&b.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.cmp(&b.1))
    });
    // a material's own group adds its areas and lengths up; other groups
    // mix layers, whose areas would count a wall's face once a layer
    let one_material = *g == GroupBy::Material;

    for (_, label, mine) in keyed {
        let here = format!("{path}/{}:{label}", g.label());
        let mut elements: Vec<ElementId> = vec![];
        for l in &mine {
            if !elements.contains(&l.element) {
                elements.push(l.element);
            }
        }
        let volume: f64 = mine.iter().map(|l| l.quantity.volume).sum();
        let area: f64 = mine.iter().map(|l| l.quantity.area).sum();
        let length: f64 = mine.iter().map(|l| l.quantity.length).sum();
        let perimeter: f64 = mine.iter().map(|l| l.perimeter).sum();
        // sills are pieces of one kind: their lengths add up too
        let pieces = mine
            .iter()
            .all(|l| matches!(l.category, "Outer sills" | "Inner sills" | "Thresholds"));
        let costs: Vec<f64> = mine
            .iter()
            .filter_map(|l| line_cost(model, l.material, &l.quantity))
            .collect();
        let mut cells = vec![String::new(); titles.len()];
        cells[0] = if titles.contains(&"Elements") {
            format!("{}{label}", indent(depth))
        } else {
            format!("{}{label} ({})", indent(depth), elements.len())
        };
        for (i, c) in titles.iter().enumerate() {
            cells[i] = match *c {
                "Elements" => elements.len().to_string(),
                "Volume" if volume > 1e-9 => format!("{volume:.2} m³"),
                "Area" if one_material && area > 1e-9 => format!("{area:.2} m²"),
                "Length" if (one_material || pieces) && length > 1e-9 => {
                    format!("{length:.2} m")
                }
                "Perimeter" if perimeter > 1e-9 => format!("{perimeter:.2} m"),
                "Cost" if !costs.is_empty() => format!("{:.2}", costs.iter().sum::<f64>()),
                _ => std::mem::take(&mut cells[i]),
            };
        }
        let material = if one_material {
            mine.first().and_then(|l| l.material)
        } else {
            None
        };
        t.schedule.rows.push(Row {
            id: material.unwrap_or(ElementId::NIL),
            cells,
            group: true,
        });
        t.elements.push(elements);
        t.material.push(material);
        t.depth.push(depth);
        t.path.push(here.clone());
        build(model, &mine, rest, depth + 1, here, layout, sort, t);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use creabim_core::{ElementKind, LayerFunction};

    fn starter() -> Model {
        creabim_core::starter::starter_house()
    }

    #[test]
    fn every_layer_of_a_layered_wall_is_its_own_material_row() {
        let model = starter();
        let all = lines(&model);
        // a wall with more than one layer
        let (wall, layers) = model
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::Wall(_) => creabim_core::assembly_of(&model, e)
                    .filter(|(_, a)| a.layers.len() > 1)
                    .map(|(_, a)| (e.id, a.layers.clone())),
                _ => None,
            })
            .expect("the starter house has a layered wall");
        let solid: Vec<_> = layers
            .iter()
            .filter(|l| !(l.function == LayerFunction::AirGap && l.material.is_none()))
            .collect();
        let mine = all.iter().filter(|l| l.element == wall).count();
        assert_eq!(mine, solid.len());
        // grouped by element type, every material of the walls has a row
        let t = table_measured(&model, &Layout::default(), false);
        let walls = t
            .schedule
            .rows
            .iter()
            .position(|r| r.group && r.cells[0].starts_with(&tr("Walls")))
            .expect("a heading for the walls");
        for l in &solid {
            let Some(m) = l.material else { continue };
            let name = model.get(m).unwrap().name.clone();
            assert!(
                t.schedule.rows[walls + 1..]
                    .iter()
                    .take_while(|r| !r.group)
                    .any(|r| r.cells[0].trim() == name),
                "{name} is listed under the walls"
            );
        }
    }

    #[test]
    fn volumes_are_measured_on_the_trimmed_solids() {
        let model = starter();
        let rough = takeoff::lines(&model);
        let measured = lines(&model);
        // every line stays; a layer part of another material (an ETICS
        // fire barrier of mineral wool) adds one of its own
        assert!(measured.len() >= rough.len());
        for r in &rough {
            assert!(measured
                .iter()
                .any(|l| l.element == r.element && l.layer == r.layer && l.material == r.material));
        }
        let sum = |ls: &[Line], cat: &str| {
            ls.iter()
                .filter(|l| l.category == cat)
                .map(|l| l.quantity.volume)
                .sum::<f64>()
        };
        // a slab gives up what the walls through it take
        let slabs = |ls: &[Line]| sum(ls, "Slabs") + sum(ls, "Foundation slabs");
        let (r, m) = (slabs(&rough), slabs(&measured));
        assert!(
            m > 0.0 && m <= r * 1.001,
            "slabs: measured {m:.3}, rough {r:.3}"
        );
        // the walls: gables under the roof in, what slabs and columns take out
        let (r, m) = (sum(&rough, "Walls"), sum(&measured, "Walls"));
        assert!(
            m > 0.5 * r && (m - r).abs() > 0.01,
            "walls: measured {m:.3}, rough {r:.3}"
        );
        // a wall layer's face is its trimmed volume over its thickness
        for l in measured
            .iter()
            .filter(|l| l.category == "Walls" && l.thickness > 1e-3)
        {
            assert!((l.quantity.area * l.thickness - l.quantity.volume).abs() < 1e-6);
        }
    }

    #[test]
    fn a_plain_box_measures_its_volume() {
        let (x, y) = (1000.0, 2000.0);
        let square = [
            glam::DVec2::new(x, y),
            glam::DVec2::new(x + 2.0, y),
            glam::DVec2::new(x + 2.0, y + 3.0),
            glam::DVec2::new(x, y + 3.0),
        ];
        let m = crate::scene::slab_mesh(&square, 7.0, 4.0);
        assert!((mesh_volume(&m) - 24.0).abs() < 1e-3, "{}", mesh_volume(&m));
    }

    #[test]
    fn groups_nest_and_their_volumes_add_up() {
        let model = starter();
        let layout = Layout {
            group: Some(vec![GroupBy::Level, GroupBy::Category]),
            ..Default::default()
        };
        let t = table_measured(&model, &layout, false);
        let total: f64 = lines(&model).iter().map(|l| l.quantity.volume).sum();
        let heads: f64 = t
            .schedule
            .rows
            .iter()
            .zip(&t.depth)
            .filter(|(r, d)| r.group && **d == 0)
            .filter_map(|(r, _)| {
                let i = t
                    .schedule
                    .columns
                    .iter()
                    .position(|c| c.title == "Volume")?;
                r.cells[i].trim_end_matches(" m³").parse::<f64>().ok()
            })
            .sum();
        assert!((heads - total).abs() < 0.05 * t.depth.len() as f64 / 100.0 + 0.05);
        // a level heading, then element types one deeper
        assert!(t.schedule.rows[0].group && t.depth[0] == 0);
        assert!(t.schedule.rows[1].group && t.depth[1] == 1);
        assert_eq!(t.schedule.rows.len(), t.elements.len());
    }

    #[test]
    fn detail_rows_sort_by_a_column() {
        let model = starter();
        let layout = Layout {
            group: Some(vec![]),
            detail: true,
            sort: Some("Volume".into()),
            descending: true,
        };
        let t = table_measured(&model, &layout, false);
        let i = t
            .schedule
            .columns
            .iter()
            .position(|c| c.title == "Volume")
            .unwrap();
        let v: Vec<f64> = t
            .schedule
            .rows
            .iter()
            .map(|r| r.cells[i].trim_end_matches(" m³").parse().unwrap_or(0.0))
            .collect();
        assert!(v.windows(2).all(|w| w[0] >= w[1] - 1e-9));
        assert!(t.schedule.rows.iter().all(|r| !r.group));
        // every element row points at its element
        assert!(t.elements.iter().all(|e| e.len() == 1));
    }

    #[test]
    fn a_filter_keeps_only_matching_lines() {
        let model = starter();
        let all = table_measured(&model, &Layout::default(), false);
        let some = table(&model, &Layout::default(), false, &tr("Walls"));
        assert!(some.schedule.rows.len() < all.schedule.rows.len());
        assert!(some.schedule.rows.iter().any(|r| r.group));
    }

    #[test]
    fn openings_have_their_perimeter_and_sills_count_by_size() {
        let model = starter();
        let all = lines(&model);
        // a window's or door's perimeter is round its size
        for l in all
            .iter()
            .filter(|l| matches!(l.category, "Doors" | "Windows"))
        {
            let Some(ElementKind::Opening(o)) = model.get(l.element).map(|e| &e.kind) else {
                panic!("an opening");
            };
            assert!((l.perimeter - 2.0 * (o.width + o.height)).abs() < 1e-9);
        }
        let t = table_measured(&model, &Layout::default(), false);
        let at = |title: &str| {
            t.schedule
                .columns
                .iter()
                .position(|c| c.title == title)
                .unwrap()
        };
        let (p, n) = (at("Perimeter"), at("Elements"));
        let windows = t
            .schedule
            .rows
            .iter()
            .find(|r| r.group && r.cells[0].trim() == tr("Windows"))
            .expect("the windows' heading");
        assert!(windows.cells[p].ends_with(" m"), "{:?}", windows.cells);
        let length = at("Length");
        // sills: outer and inner, a row for each size, pieces adding up to
        // the windows that have them
        for (cat, sills) in [
            ("Outer sills", creabim_core::joinery::outer_sills(&model)),
            ("Inner sills", creabim_core::joinery::inner_sills(&model)),
        ] {
            assert!(!sills.is_empty(), "{cat}");
            let head = t
                .schedule
                .rows
                .iter()
                .position(|r| r.group && r.cells[0].trim() == tr(cat))
                .expect("a heading for the sills");
            let rows: Vec<_> = t.schedule.rows[head + 1..]
                .iter()
                .take_while(|r| !r.group)
                .collect();
            assert_eq!(rows.len(), sills.len(), "{cat}: one row a size");
            assert!(t.schedule.rows[head].cells[length].ends_with(" m"));
            let pieces: usize = rows
                .iter()
                .map(|r| r.cells[n].parse::<usize>().unwrap())
                .sum();
            let windows: usize = sills.iter().map(|s| s.windows.len()).sum();
            assert_eq!(pieces, windows);
            assert!(rows.iter().all(|r| r.cells[0].contains(" × ")));
        }
        // an inner sill is longer than its window by its ears
        let inner = &creabim_core::joinery::inner_sills(&model)[0];
        let Some(ElementKind::Opening(o)) = model.get(inner.windows[0]).map(|e| &e.kind) else {
            panic!("a window");
        };
        assert!((inner.length - (o.width + 0.04)).abs() < 1e-3);
    }

    #[test]
    fn thresholds_count_by_material_and_size() {
        let model = starter();
        let wood = model
            .iter()
            .find(|e| matches!(e.kind, ElementKind::Material(_)))
            .unwrap()
            .clone();
        let mut els: Vec<_> = model.iter().cloned().collect();
        let mut doors = 0;
        for e in els.iter_mut() {
            if let ElementKind::Opening(o) = &mut e.kind {
                if o.kind == creabim_core::OpeningKind::Door {
                    o.threshold = true;
                    o.threshold_material = Some(wood.id);
                    doors += 1;
                }
            }
        }
        assert!(doors > 1);
        let model = Model::from_elements(els).unwrap();
        let all = takeoff::lines(&model);
        let sills: Vec<_> = all.iter().filter(|l| l.category == "Thresholds").collect();
        assert_eq!(sills.len(), doors);
        for l in &sills {
            assert_eq!(l.material, Some(wood.id));
            let label = l.label.as_deref().unwrap();
            assert!(
                label.contains(&wood.name) && label.contains(" × "),
                "{label}"
            );
            assert!(l.quantity.length > 0.3 && l.quantity.area > 0.0);
        }
        let t = table_measured(&model, &Layout::default(), false);
        let n = t
            .schedule
            .columns
            .iter()
            .position(|c| c.title == "Elements")
            .unwrap();
        let head = t
            .schedule
            .rows
            .iter()
            .position(|r| r.group && r.cells[0].trim() == tr("Thresholds"))
            .expect("a heading for the thresholds");
        let rows: Vec<_> = t.schedule.rows[head + 1..]
            .iter()
            .take_while(|r| !r.group)
            .collect();
        // a row for each size, named by it
        assert!(rows.iter().all(|r| r.cells[0].contains(" × ")));
        let pieces: usize = rows
            .iter()
            .map(|r| r.cells[n].parse::<usize>().unwrap())
            .sum();
        assert_eq!(pieces, doors);
    }

    #[test]
    fn foundation_slabs_are_listed_apart() {
        let model = starter();
        let all = takeoff::lines(&model);
        let named = |cat: &str| -> Vec<String> {
            let mut v: Vec<_> = all
                .iter()
                .filter(|l| l.category == cat)
                .filter_map(|l| model.get(l.element).map(|e| e.name.clone()))
                .collect();
            v.dedup();
            v
        };
        assert!(named("Foundation slabs").contains(&"GF Slab".to_string()));
        assert!(named("Slabs").contains(&"FF Slab".to_string()));
        // set by hand, it wins
        let mut els: Vec<_> = model.iter().cloned().collect();
        for e in els.iter_mut() {
            if let ElementKind::Slab(s) = &mut e.kind {
                if e.name == "GF Slab" {
                    s.kind = Some(creabim_core::SlabKind::Floor);
                }
            }
        }
        let model = Model::from_elements(els).unwrap();
        let all = takeoff::lines(&model);
        assert!(!all.iter().any(|l| l.category == "Foundation slabs"
            && model.get(l.element).is_some_and(|e| e.name == "GF Slab")));
    }
}
