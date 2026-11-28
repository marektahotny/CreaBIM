//! The temperature field of a construction detail (Heating › Thermal
//! field of a detail): a detail called out of a section is cut into its
//! materials ([`creabim_geom::thermalcut`]), its field solved by STN EN
//! ISO 10211 ([`creabim_services::thermfield`]) and checked by STN 73
//! 0540-2 čl. 5.3 ([`creabim_services::thermcheck`]): the lowest inner
//! surface temperature against θsi,80 + Δθsi (table 4) and the
//! temperature factor fRsi against fRsi,N (table 5). The window shows the
//! field in colour with its isotherms and the results as a table.

use super::CreaBimApp;
use crate::sheet::{self, Col};
use creabim_core::fireparams::ParamTable;
use creabim_core::{ElementId, ElementKind, Model};
use creabim_services::thermcheck::{self, Psat};
use creabim_services::thermfield::{
    self, Air, Detail, Env, Field, Material, Mesh, MeshCheck, Region,
};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use std::collections::BTreeMap;

/// What the designer sets, and the last result.
#[derive(Clone, Debug)]
pub struct Thermal {
    pub view: Option<ElementId>,
    /// Inside air θi, °C, and its relative humidity φi, %.
    pub ti: f64,
    pub phi_i: f64,
    /// Outside design temperature θe, °C: the project's locality's (STN
    /// 73 0540-3), until set here; NaN while there is none.
    pub te: f64,
    /// The locality θe was taken from.
    pub site: String,
    /// Rsi 0,25 of STN EN ISO 13788 instead of STN 73 0540-3 table 15.
    pub rsi_iso: bool,
    /// The way of heating, a row of STN 73 0540-2 table 4.
    pub heating: String,
    /// The coldest spot is a corner (kút) rather than a plain surface.
    pub corner: bool,
    /// Mesh: largest cell and the cell at material boundaries, mm.
    pub h_max_mm: f64,
    pub h_min_mm: f64,
    /// Check the mesh by halving every cell (STN EN ISO 10211 7.2.7).
    pub check_mesh: bool,
    /// Ψ measured on the inside or the outside dimensions.
    pub external: bool,
    /// λ given by hand for materials without one, by their name.
    pub lambda: BTreeMap<String, f64>,
    /// The floor's characteristic dimension B (ISO 13370) for a junction
    /// with the ground, m; NaN until given.
    pub b: f64,
    /// Largest cell of a junction with the ground (its soil is tens of
    /// metres), mm.
    pub h_ground_mm: f64,
    pub result: Option<std::sync::Arc<Outcome>>,
    pub error: Option<String>,
}

impl Default for Thermal {
    fn default() -> Self {
        Self {
            view: None,
            ti: 20.0,
            phi_i: 50.0,
            te: f64::NAN,
            site: String::new(),
            rsi_iso: false,
            heating: String::new(),
            corner: true,
            h_max_mm: 20.0,
            h_min_mm: 2.0,
            check_mesh: true,
            external: false,
            lambda: BTreeMap::new(),
            b: f64::NAN,
            h_ground_mm: 250.0,
            result: None,
            error: None,
        }
    }
}

/// A solved detail and its checks.
#[derive(Debug)]
pub struct Outcome {
    pub detail: Detail,
    pub field: Field,
    /// The outlines of the cut pieces, for the drawing.
    pub outlines: Vec<Vec<[f64; 2]>>,
    pub ti: f64,
    pub te: f64,
    pub rsi: thermcheck::SurfaceRs,
    pub rsi_iso: bool,
    /// Heat flow from inside, W/m, and L2D, W/(m·K).
    pub flow: f64,
    pub l2d: f64,
    pub min: Option<thermfield::SurfaceNode>,
    pub frsi: Option<f64>,
    pub frsi_n: Result<f64, String>,
    pub theta_si80: Option<f64>,
    pub delta: Option<f64>,
    pub phi_max: Option<f64>,
    pub mesh: Option<MeshCheck>,
    pub mesh_fine_nodes: usize,
    /// The flanks' U and lengths and Ψ (formula 18), or why not.
    pub psi: Result<(Vec<(thermfield::Flank, f64)>, f64), String>,
    pub external: bool,
    /// The part drawn (the detail's box), lower and upper corner.
    pub view: ([f64; 2], [f64; 2]),
    /// A junction with the ground: Ψg by option B (12.4.3.1).
    pub ground: Option<GroundOut>,
}

/// What Ψg is made of.
#[derive(Debug)]
pub struct GroundOut {
    pub b: f64,
    /// The floor's area and perimeter B was taken from, m², m.
    pub ap: Option<(f64, f64)>,
    pub l2d_a: f64,
    pub hw: f64,
    pub uw: f64,
    /// The model's extent: floor inwards, soil outwards and down, m.
    pub floor: f64,
    pub soil: f64,
}

/// Materials of the detail, by name: λ and whether it was given by hand.
pub fn materials_of(
    model: &Model,
    cut: &creabim_geom::thermalcut::ThermalCut,
) -> Vec<(String, Option<f64>)> {
    let mut out: Vec<(String, Option<f64>)> = vec![];
    for p in &cut.pieces {
        let (name, lam) = piece_material(model, p);
        if !out.iter().any(|(n, _)| *n == name) {
            out.push((name, lam));
        }
    }
    out
}

/// A piece's material: its name and λ from the project's materials.
fn piece_material(model: &Model, p: &creabim_geom::thermalcut::Piece) -> (String, Option<f64>) {
    let site = matches!(
        model.get(p.element).map(|e| &e.kind),
        Some(ElementKind::Site(_))
    );
    match p.material.and_then(|m| model.get(m)) {
        Some(e) => {
            let lam = match &e.kind {
                ElementKind::Material(m) => m.lambda,
                _ => None,
            };
            (e.name.clone(), lam)
        }
        None if p.air_gap => (crate::i18n::tr("Air gap"), None),
        None if site => (crate::i18n::tr("Soil"), None),
        // a room's tiling: by its name, its λ given here
        None => match model.get(p.element).map(|e| &e.kind) {
            Some(ElementKind::Room(r)) => {
                let name = r
                    .tiling
                    .as_ref()
                    .map(|t| t.name.clone())
                    .filter(|n| !n.is_empty());
                (
                    name.map_or_else(
                        || crate::i18n::tr("Tiling"),
                        |n| format!("{} – {n}", crate::i18n::tr("Tiling")),
                    ),
                    None,
                )
            }
            _ => (
                model
                    .get(p.element)
                    .map_or_else(String::new, |e| e.name.clone()),
                None,
            ),
        },
    }
}

/// Build the detail and solve it.
pub fn solve(model: &Model, tables: &[ParamTable], s: &Thermal) -> Result<Outcome, String> {
    let t = crate::i18n::tr;
    let view = s
        .view
        .ok_or_else(|| t("Choose a detail called out of a section."))?;
    let cut = creabim_geom::thermalcut::cut(model, view, None)
        .ok_or_else(|| t("The view is not a detail called out of a section."))?;
    if cut.pieces.is_empty() {
        return Err(t("The detail's box cuts nothing."));
    }
    let rsi = thermcheck::surface_rs(tables, s.rsi_iso)
        .ok_or_else(|| t("The surface resistances are not read (STN 73 0540-3 table 15, STN EN ISO 13788): read the norm tables (Manage › Norm tables)."))?;
    let soil = thermcheck::soil_lambda(tables);
    // the materials and their λ: the project's, else given here
    let mut names: Vec<String> = vec![];
    let mut materials: Vec<Material> = vec![];
    let mut missing: Vec<String> = vec![];
    let mut regions = vec![];
    let mut outlines = vec![];
    let mut soil_index = None;
    for p in &cut.pieces {
        let (name, lam) = piece_material(model, p);
        let site = matches!(
            model.get(p.element).map(|e| &e.kind),
            Some(ElementKind::Site(_))
        );
        let lam = s
            .lambda
            .get(&name)
            .copied()
            .or(lam)
            .or(if site { soil } else { None });
        let k = match names.iter().position(|n| *n == name) {
            Some(k) => k,
            None => {
                names.push(name.clone());
                materials.push(Material {
                    name: name.clone(),
                    lambda: lam.unwrap_or(0.0),
                });
                if lam.is_none_or(|l| l <= 0.0) {
                    missing.push(name.clone());
                }
                names.len() - 1
            }
        };
        if site && soil_index.is_none() {
            soil_index = Some(k);
        }
        let poly: Vec<[f64; 2]> = p.points.iter().map(|q| [q.x, q.y]).collect();
        outlines.push(poly.clone());
        regions.push(Region { material: k, poly });
    }
    if !missing.is_empty() {
        return Err(format!("{}: {}", t("λ is missing for"), missing.join(", ")));
    }
    let rect = |lo: glam::DVec2, hi: glam::DVec2| {
        vec![[lo.x, lo.y], [hi.x, lo.y], [hi.x, hi.y], [lo.x, hi.y]]
    };
    // the air over a floor gives its heat down, under a ceiling up
    let inside = Air {
        env: 0,
        rs_over: rsi.down,
        rs_beside: rsi.level,
        rs_under: rsi.up,
    };
    let detail = Detail {
        materials,
        regions,
        envs: vec![
            Env {
                name: t("Inside"),
                theta: s.ti,
            },
            Env {
                name: t("Outside"),
                theta: s.te,
            },
        ],
        edges: vec![],
        air: cut
            .rooms
            .iter()
            .map(|r| (rect(r.lo, r.hi), inside.clone()))
            .collect(),
        outside: Some(Air {
            env: 1,
            rs_over: rsi.e,
            rs_beside: rsi.e,
            rs_under: rsi.e,
        }),
        bounds: Some(([cut.lo.x, cut.lo.y], [cut.hi.x, cut.hi.y])),
        voids: vec![],
    };
    let view = ([cut.lo.x, cut.lo.y], [cut.hi.x, cut.hi.y]);
    if cut.rooms.is_empty() {
        return Err(t(
            "No room is cut in the detail: the inside air is taken from the rooms.",
        ));
    }
    let mesh = Mesh {
        h_max: s.h_max_mm.max(0.1) / 1000.0,
        h_min: s.h_min_mm.max(0.05) / 1000.0,
        ratio: 1.25,
    };
    let dims = if s.external {
        thermfield::Dimensions::External
    } else {
        thermfield::Dimensions::Internal
    };
    // a junction with the ground: the model of 12.4.1 around the box,
    // Ψg by option B (12.4.3.1, formula 23)
    let mut ground = None;
    let mut detail = detail;
    let mut open = None;
    let mut mesh = mesh;
    if let Some(soil) = soil_index {
        // B = A / (0,5·P) of the floor slab on the ground (STN EN ISO
        // 13370, 6.7.1, formula 2), unless given by hand
        let (b, ap) = if s.b.is_nan() {
            let (a, p) = floor_area_perimeter(model, &cut).ok_or_else(|| {
                t("A junction with the ground: no floor slab in the detail to take B from; enter B (STN EN ISO 13370).")
            })?;
            (a / (0.5 * p), Some((a, p)))
        } else {
            (s.b, None)
        };
        if b.is_nan() || b <= 0.0 {
            return Err(t("A junction with the ground: enter the floor's characteristic dimension B (STN EN ISO 13370)."));
        }
        let g = thermcheck::ground_params(tables, soil, b).ok_or_else(|| {
            format!(
                "{}: {}",
                t("Norm tables not read (Manage › Norm tables)"),
                thermcheck::GROUND
            )
        })?;
        let m = detail.ground(&g, 0, 1).map_err(|e| {
            format!(
                "{} ({e})",
                t("The junction with the ground could not be modelled")
            )
        })?;
        ground = Some(GroundOut {
            b,
            ap,
            l2d_a: f64::NAN,
            hw: m.hw,
            uw: m.uw,
            floor: (g.floor_factor * b).min(g.half_max),
            soil: (g.soil_factor * b).min(g.soil_max),
        });
        outlines = m.full.regions[2..].iter().map(|r| r.poly.clone()).collect();
        detail = m.full;
        open = Some(m.open);
        mesh.h_max = s.h_ground_mm.max(s.h_max_mm).max(0.1) / 1000.0;
    }
    let (field, check, fine_nodes) = if s.check_mesh {
        let (f, fine) = detail.solve_checked(&mesh);
        let (fl, fr) = thermcheck::mesh_limits(tables).unwrap_or((f64::NAN, f64::NAN));
        let c = thermfield::mesh_check(&f, &fine, 0, 1, &detail.envs, fl, fr);
        (f, Some(c), fine.nodes())
    } else {
        (detail.solve(&mesh), None, 0)
    };
    let flow = field.flows[0];
    let l2d = field.l2d(0, 1, &detail.envs);
    let min = field.min_surface(0);
    let frsi = min.map(|m| thermfield::frsi(m.theta, s.ti, s.te));
    let psat = Psat::of(tables);
    let psi = match (&mut ground, &open) {
        (Some(g), Some(open)) => {
            let a = open.solve(&mesh);
            g.l2d_a = a.l2d(0, 1, &open.envs);
            Ok((vec![], thermfield::psi_g(l2d, g.hw, g.uw, g.l2d_a)))
        }
        _ => detail
            .psi_parts(0, 1, dims)
            .map(|parts| {
                let ul: Vec<(f64, f64)> = parts.iter().map(|(f, l)| (f.u, *l)).collect();
                let v = thermfield::psi(l2d, &ul);
                (parts, v)
            })
            .map_err(|e| {
                format!(
                    "{} ({e})",
                    t("Ψ needs two flanking elements crossing the box's edges")
                )
            }),
    };
    Ok(Outcome {
        field,
        outlines,
        ti: s.ti,
        te: s.te,
        rsi,
        rsi_iso: s.rsi_iso,
        flow,
        l2d,
        min,
        frsi,
        frsi_n: thermcheck::frsi_n(tables, s.te, s.ti),
        theta_si80: psat.map(|p| p.theta_si80(s.ti, s.phi_i)),
        delta: thermcheck::delta_theta_si(tables, &s.heating, s.corner),
        phi_max: thermcheck::phi_i_max(tables),
        mesh: check,
        mesh_fine_nodes: fine_nodes,
        psi,
        external: s.external,
        detail,
        view,
        ground,
    })
}

/// The floor area and exposed perimeter (STN EN ISO 13370, 6.7.1) of
/// the storey of the floor slab on the ground the detail cuts (the
/// lowest slab among its pieces): its rooms by the outside dimensions,
/// P the outside walls only.
fn floor_area_perimeter(
    model: &Model,
    cut: &creabim_geom::thermalcut::ThermalCut,
) -> Option<(f64, f64)> {
    let low = |p: &creabim_geom::thermalcut::Piece| {
        p.points.iter().map(|q| q.y).fold(f64::INFINITY, f64::min)
    };
    let piece = cut
        .pieces
        .iter()
        .filter(|p| {
            matches!(
                model.get(p.element).map(|e| &e.kind),
                Some(ElementKind::Slab(_))
            )
        })
        .min_by(|a, b| low(a).total_cmp(&low(b)))?;
    let ElementKind::Slab(slab) = &model.get(piece.element)?.kind else {
        return None;
    };
    creabim_services::heatdetail::floor_area_perimeter(model, slab.level)
}

/// Colour of temperature `v` between `lo` (blue) and `hi` (red).
fn colour(v: f64, lo: f64, hi: f64) -> Color32 {
    let s = if hi > lo {
        ((v - lo) / (hi - lo)).clamp(0.0, 1.0)
    } else {
        0.5
    } as f32;
    // blue → cyan → green → yellow → red
    let stops = [
        [40, 60, 200],
        [40, 180, 230],
        [70, 190, 90],
        [240, 220, 60],
        [215, 50, 40],
    ];
    let x = s * 4.0;
    let i = (x.floor() as usize).min(3);
    let f = x - i as f32;
    let c: Vec<u8> = (0..3)
        .map(|k| (stops[i][k] as f32 + f * (stops[i + 1][k] as f32 - stops[i][k] as f32)) as u8)
        .collect();
    Color32::from_rgb(c[0], c[1], c[2])
}

/// Draw the field in `rect`: colour by temperature, the pieces' outlines,
/// isotherms every `step` °C, the coldest surface spot marked.
fn draw_field(ui: &mut egui::Ui, o: &Outcome, size: Vec2) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 0.0, ui.visuals().extreme_bg_color);
    let f = &o.field;
    if f.xs.len() < 2 || f.ys.len() < 2 {
        return;
    }
    // the detail's box (a junction with the ground is solved far wider)
    let ([x0, y0], [x1, y1]) = o.view;
    let w = (x1 - x0).max(1e-9);
    let h = (y1 - y0).max(1e-9);
    let pad = 34.0;
    let sc = ((rect.width() - 2.0 * pad) as f64 / w).min((rect.height() - 2.0 * pad) as f64 / h);
    let ox = rect.center().x as f64 - 0.5 * w * sc;
    let oy = rect.center().y as f64 + 0.5 * h * sc;
    let to = |x: f64, y: f64| Pos2::new((ox + (x - x0) * sc) as f32, (oy - (y - y0) * sc) as f32);
    let (lo, hi) = (o.te.min(o.ti), o.te.max(o.ti));
    // the field, its isotherms and outlines only inside the box
    let pv = p.with_clip_rect(Rect::from_two_pos(to(x0, y0), to(x1, y1)).intersect(rect));
    let nx = f.xs.len();
    let mut mesh = egui::Mesh::default();
    for j in 0..f.ys.len() - 1 {
        for i in 0..nx - 1 {
            if f.cells[i + j * (nx - 1)].is_none()
                || f.xs[i + 1] < x0
                || f.xs[i] > x1
                || f.ys[j + 1] < y0
                || f.ys[j] > y1
            {
                continue;
            }
            let v = [
                f.theta[i + j * nx],
                f.theta[i + 1 + j * nx],
                f.theta[i + 1 + (j + 1) * nx],
                f.theta[i + (j + 1) * nx],
            ];
            if v.iter().any(|t| !t.is_finite()) {
                continue;
            }
            let base = mesh.vertices.len() as u32;
            let pts = [
                to(f.xs[i], f.ys[j]),
                to(f.xs[i + 1], f.ys[j]),
                to(f.xs[i + 1], f.ys[j + 1]),
                to(f.xs[i], f.ys[j + 1]),
            ];
            for k in 0..4 {
                mesh.colored_vertex(pts[k], colour(v[k], lo, hi));
            }
            mesh.add_triangle(base, base + 1, base + 2);
            mesh.add_triangle(base, base + 2, base + 3);
        }
    }
    pv.add(egui::Shape::mesh(mesh));
    let iso = Stroke::new(1.0, Color32::from_black_alpha(150));
    let step = if hi - lo > 20.0 { 2.0 } else { 1.0 };
    let mut tv = (lo / step).ceil() * step;
    while tv < hi {
        for [a, b] in thermfield::isotherm(f, tv) {
            pv.line_segment([to(a[0], a[1]), to(b[0], b[1])], iso);
        }
        tv += step;
    }
    // the critical isotherm θsi,80 + Δθsi, in white
    if let (Some(t80), Some(d)) = (o.theta_si80, o.delta) {
        for [a, b] in thermfield::isotherm(f, t80 + d) {
            pv.line_segment(
                [to(a[0], a[1]), to(b[0], b[1])],
                Stroke::new(2.0, Color32::WHITE),
            );
        }
    }
    let edge = Stroke::new(1.2, Color32::from_gray(20));
    for poly in &o.outlines {
        let pts: Vec<Pos2> = poly.iter().map(|q| to(q[0], q[1])).collect();
        pv.add(egui::Shape::closed_line(pts, edge));
    }
    if let Some(m) = o.min {
        let at = to(m.p[0], m.p[1]);
        p.circle_stroke(at, 6.0, Stroke::new(2.5, Color32::from_rgb(160, 0, 160)));
        // the value on a light patch, readable over any colour
        let galley = p.layout_no_wrap(
            format!("θsi,min {} °C", crate::i18n::num(m.theta, 2)),
            egui::FontId::proportional(13.0),
            Color32::from_rgb(110, 0, 110),
        );
        let at_text = at + Vec2::new(10.0, -10.0 - galley.size().y);
        p.rect_filled(
            Rect::from_min_size(at_text, galley.size()).expand(3.0),
            3.0,
            Color32::from_white_alpha(225),
        );
        p.galley(at_text, galley, Color32::from_rgb(110, 0, 110));
    }
    // the scale of colours
    let bar = Rect::from_min_size(
        Pos2::new(rect.right() - 22.0, rect.top() + 10.0),
        Vec2::new(12.0, rect.height() - 20.0),
    );
    let n = 40;
    for k in 0..n {
        let a = k as f32 / n as f32;
        let r = Rect::from_min_max(
            Pos2::new(
                bar.left(),
                bar.bottom() - (a + 1.0 / n as f32) * bar.height(),
            ),
            Pos2::new(bar.right(), bar.bottom() - a * bar.height()),
        );
        p.rect_filled(r, 0.0, colour(lo + a as f64 * (hi - lo), lo, hi));
    }
    for (v, y) in [(hi, bar.top()), (lo, bar.bottom())] {
        p.text(
            Pos2::new(bar.left() - 3.0, y),
            egui::Align2::RIGHT_CENTER,
            format!("{} °C", crate::i18n::num(v, 0)),
            egui::FontId::proportional(12.0),
            ui.visuals().text_color(),
        );
    }
}

impl CreaBimApp {
    /// `calc:thermalfield` from the Heating tab: open the window on the
    /// detail in view (and solve it at once when it is one).
    pub(super) fn thermal_action(&mut self, head: &str, arg: &str) -> bool {
        if head != "calc" || arg != "thermalfield" {
            return false;
        }
        let mut s = self.thermal.take().unwrap_or_default();
        let model = self.doc.model();
        if let Some(v) = self.active_view.filter(|v| {
            creabim_geom::thermalcut::is_cuttable(model, *v)
                && model
                    .get(*v)
                    .is_some_and(|e| matches!(&e.kind, ElementKind::View(v) if v.callout.is_some()))
        }) {
            s.view = Some(v);
        }
        if s.heating.is_empty() {
            let tables = thermcheck::tables_of(model);
            s.heating = thermcheck::heating_modes(&tables)
                .first()
                .map(|m| m.0.clone())
                .unwrap_or_default();
        }
        if s.te.is_nan() {
            let name = model
                .project_info()
                .map(|(_, p)| p.climate_site.clone())
                .unwrap_or_default();
            if let Some(site) = self
                .climate_sites()
                .into_iter()
                .find(|c| c.name == name && !name.is_empty())
            {
                s.te = site.te;
                s.site = site.name;
            }
        }
        // a headless screenshot: B of a junction with the ground
        if let Some(b) = std::env::var("CREABIM_SHOT_THERMAL_B")
            .ok()
            .and_then(|v| creabim_core::fireparams::number(&v))
        {
            s.b = b;
        }
        // a headless screenshot: λ given as "name=value;…"
        if let Ok(l) = std::env::var("CREABIM_SHOT_THERMAL_LAMBDA") {
            for (k, v) in l.split(';').filter_map(|kv| kv.split_once('=')) {
                if let Some(v) = creabim_core::fireparams::number(v) {
                    s.lambda.insert(k.to_string(), v);
                }
            }
        }
        if s.view.is_some() {
            let tables = thermcheck::tables_of(model);
            match solve(model, &tables, &s) {
                Ok(o) => {
                    s.result = Some(std::sync::Arc::new(o));
                    s.error = None;
                }
                Err(e) => s.error = Some(e),
            }
        }
        self.thermal = Some(s);
        true
    }

    /// The window, while it is open.
    pub(super) fn thermal_window(&mut self, ctx: &egui::Context) {
        let Some(mut s) = self.thermal.clone() else {
            return;
        };
        let t = crate::i18n::tr;
        let num = crate::i18n::num;
        let model = self.doc.model();
        let tables = thermcheck::tables_of(model);
        let missing: Vec<&str> = thermcheck::TABLES
            .iter()
            .copied()
            .filter(|id| !tables.iter().any(|t| t.id == *id))
            .collect();
        let details: Vec<(ElementId, String)> = model
            .iter()
            .filter(|e| {
                matches!(&e.kind, ElementKind::View(v) if v.callout.is_some())
                    && creabim_geom::thermalcut::is_cuttable(model, e.id)
            })
            .map(|e| (e.id, e.name.clone()))
            .collect();
        let mut open = true;
        let mut calc = false;
        // the whole window: the field and its table side by side
        let area = ctx.content_rect();
        let mats = s
            .view
            .and_then(|v| creabim_geom::thermalcut::cut(model, v, None))
            .map(|c| materials_of(model, &c))
            .unwrap_or_default();
        let modes = thermcheck::heating_modes(&tables);
        super::shell::window(t("Thermal field of a detail"))
            .id(egui::Id::new("thermal.field"))
            .open(&mut open)
            .default_pos(area.center() - Vec2::new(590.0, 380.0))
            .default_size(crate::app::shell::at_least(Vec2::new(1180.0, 760.0)))
            .constrain_to(area)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().id_salt("thermal.scroll").auto_shrink([false, false]).show(ui, |ui| {
                if !missing.is_empty() {
                    ui.colored_label(
                        ui.visuals().warn_fg_color,
                        format!("{}: {}", t("Norm tables not read (Manage › Norm tables)"), missing.join(", ")),
                    );
                }
                ui.horizontal_wrapped(|ui| {
                    ui.label(t("Detail"));
                    let shown = s.view.and_then(|v| details.iter().find(|d| d.0 == v)).map_or(t("choose"), |d| d.1.clone());
                    egui::ComboBox::from_id_salt("thermal.view").selected_text(shown).width(220.0).show_ui(ui, |ui| {
                        for (id, name) in &details {
                            ui.selectable_value(&mut s.view, Some(*id), name);
                        }
                    });
                    if details.is_empty() {
                        ui.weak(t("Draw a callout on a section first: its box is the detail (cut planes at least 3 thicknesses from the junction, STN 73 0540-2 čl. 5.3)."));
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label("θi");
                    ui.add(egui::DragValue::new(&mut s.ti).speed(0.1).suffix(" °C"));
                    ui.label("φi");
                    ui.add(egui::DragValue::new(&mut s.phi_i).speed(0.5).range(0.0..=100.0).suffix(" %"));
                    ui.label("θe");
                    if s.te.is_nan() {
                        if ui.button(t("enter")).clicked() {
                            s.te = 0.0;
                        }
                    } else if ui.add(egui::DragValue::new(&mut s.te).speed(0.1).suffix(" °C")).changed() {
                        s.site.clear();
                    }
                    if !s.site.is_empty() {
                        ui.weak(format!("({}, STN 73 0540-3)", s.site));
                    }
                    ui.separator();
                    ui.label("Rsi");
                    egui::ComboBox::from_id_salt("thermal.rsi")
                        .selected_text(if s.rsi_iso { t("0,25 (STN EN ISO 13788)") } else { t("0,10 / 0,13 / 0,17 (STN 73 0540-3)") })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut s.rsi_iso, false, t("0,10 / 0,13 / 0,17 (STN 73 0540-3)"));
                            ui.selectable_value(&mut s.rsi_iso, true, t("0,25 (STN EN ISO 13788)"));
                        });
                    ui.separator();
                    ui.label("Ψ");
                    egui::ComboBox::from_id_salt("thermal.dims")
                        .selected_text(if s.external { t("outside dimensions") } else { t("inside dimensions") })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut s.external, false, t("inside dimensions"));
                            ui.selectable_value(&mut s.external, true, t("outside dimensions"));
                        });
                    ui.separator();
                    ui.label(t("Ground: B")).on_hover_text(t("The floor's characteristic dimension B = A / (0,5·P) by STN EN ISO 13370 (2), for a junction with the ground; above 8 m the model is the same (12.4.1)."));
                    if s.b.is_nan() {
                        ui.weak(t("from the storey"));
                        if ui.button(t("enter")).clicked() {
                            s.b = s.result.as_ref().and_then(|o| o.ground.as_ref()).map_or(8.0, |g| g.b);
                        }
                    } else {
                        ui.add(egui::DragValue::new(&mut s.b).speed(0.05).range(0.5..=200.0).suffix(" m"));
                        if ui.button(t("from the storey")).clicked() {
                            s.b = f64::NAN;
                        }
                    }
                    ui.label(t("cell in the soil"));
                    ui.add(egui::DragValue::new(&mut s.h_ground_mm).speed(1.0).range(20.0..=2000.0).prefix("max ").suffix(" mm"));
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(t("Way of heating"));
                    let shown = modes.iter().find(|m| m.0 == s.heating).map_or(t("choose"), |m| m.1.clone());
                    egui::ComboBox::from_id_salt("thermal.heating").selected_text(shown).width(300.0).show_ui(ui, |ui| {
                        for (k, text) in &modes {
                            ui.selectable_value(&mut s.heating, k.clone(), text);
                        }
                    });
                    ui.checkbox(&mut s.corner, t("Corner of the junction (Δθsi for a corner)"));
                    ui.separator();
                    ui.label(t("Mesh"));
                    ui.add(egui::DragValue::new(&mut s.h_min_mm).speed(0.1).range(0.1..=50.0).prefix("min ").suffix(" mm"));
                    ui.add(egui::DragValue::new(&mut s.h_max_mm).speed(0.5).range(1.0..=200.0).prefix("max ").suffix(" mm"));
                    ui.checkbox(&mut s.check_mesh, t("Check by halving (7.2.7)"));
                    if ui.button(t("Calculate")).clicked() {
                        calc = true;
                    }
                });
                if mats.iter().any(|m| m.1.is_none()) || !s.lambda.is_empty() {
                    egui::CollapsingHeader::new(t("λ of materials without a value")).id_salt("thermal.lambda").default_open(mats.iter().any(|m| m.1.is_none() && !s.lambda.contains_key(&m.0))).show(ui, |ui| {
                        egui::Grid::new("thermal.lambda.grid").num_columns(2).show(ui, |ui| {
                            for (name, lam) in &mats {
                                if lam.is_some() && !s.lambda.contains_key(name) {
                                    continue;
                                }
                                ui.label(name);
                                let mut v = s.lambda.get(name).copied().or(*lam).unwrap_or(0.0);
                                if ui.add(egui::DragValue::new(&mut v).speed(0.001).range(0.0..=400.0).max_decimals(4).suffix(" W/(m·K)")).changed() {
                                    s.lambda.insert(name.clone(), v);
                                }
                                ui.end_row();
                            }
                        });
                        ui.weak(t("Design λ by STN 73 0540-3 table 16 or the maker's declared value converted by STN EN ISO 10456; an unventilated air gap as λ = d/Rg."));
                    });
                }
                if let Some(e) = &s.error {
                    ui.colored_label(ui.visuals().error_fg_color, e);
                }
                let Some(o) = s.result.clone() else {
                    return;
                };
                // side by side where the window is wide, else one under the other
                let w = ui.available_width();
                if w >= 760.0 {
                    ui.horizontal_top(|ui| {
                        let h = (ui.available_height() - 10.0).clamp(300.0, 620.0);
                        draw_field(ui, &o, Vec2::new(w * 0.45, h));
                        ui.vertical(|ui| results_table(ui, &o, &s));
                    });
                } else {
                    draw_field(ui, &o, Vec2::new(w, 340.0));
                    ui.add_space(6.0);
                    results_table(ui, &o, &s);
                }
                let _ = num;
                });
            });
        if calc {
            match solve(self.doc.model(), &tables, &s) {
                Ok(o) => {
                    s.result = Some(std::sync::Arc::new(o));
                    s.error = None;
                }
                Err(e) => {
                    s.result = None;
                    s.error = Some(e);
                }
            }
        }
        self.thermal = open.then_some(s);
    }
}

/// The results and checks as a table.
fn results_table(ui: &mut egui::Ui, o: &Outcome, s: &Thermal) {
    let t = crate::i18n::tr;
    let num = crate::i18n::num;
    let yes_no = |ok: bool| if ok { t("meets") } else { t("does not meet") };
    let mut rows: Vec<(String, String, String, String)> = vec![];
    rows.push((t("Inside air θi"), "°C".into(), num(o.ti, 1), String::new()));
    rows.push((
        t("Outside air θe"),
        "°C".into(),
        num(o.te, 1),
        String::new(),
    ));
    rows.push((
        t("Surface resistance Rsi (up / level / down)"),
        "m²·K/W".into(),
        format!(
            "{} / {} / {}",
            num(o.rsi.up, 2),
            num(o.rsi.level, 2),
            num(o.rsi.down, 2)
        ),
        if o.rsi_iso {
            "STN EN ISO 13788, 4.4.1".into()
        } else {
            "STN 73 0540-3, tab. 15".into()
        },
    ));
    rows.push((
        t("Surface resistance Rse"),
        "m²·K/W".into(),
        num(o.rsi.e, 2),
        "STN 73 0540-3, tab. 15".into(),
    ));
    rows.push((
        t("Heat flow from inside Φ"),
        "W/m".into(),
        num(o.flow, 3),
        "STN EN ISO 10211 (15)".into(),
    ));
    rows.push((
        t("Thermal coupling coefficient L2D"),
        "W/(m·K)".into(),
        num(o.l2d, 4),
        "STN EN ISO 10211 (15)".into(),
    ));
    if let (Some(g), Ok((_, psi))) = (&o.ground, &o.psi) {
        if let Some((a, p)) = g.ap {
            rows.push((
                t("Floor on the ground: area A / exposed perimeter P"),
                String::new(),
                format!("{} m² / {} m", num(a, 2), num(p, 2)),
                t("rooms by outside dimensions, P outside walls only (ISO 13370, 6.7.1)"),
            ));
        }
        rows.push((
            t("Floor's characteristic dimension B"),
            "m".into(),
            num(g.b, 2),
            if g.ap.is_some() {
                "B = A / (0,5·P), STN EN ISO 13370 (2)".into()
            } else {
                t("given by hand, STN EN ISO 13370")
            },
        ));
        rows.push((
            t("Model: floor inwards / soil outwards and down"),
            "m".into(),
            format!("{} / {}", num(g.floor, 2), num(g.soil, 2)),
            "STN EN ISO 10211, 12.4.1, tab. 5".into(),
        ));
        rows.push((
            t("L2D,a without the wall"),
            "W/(m·K)".into(),
            num(g.l2d_a, 4),
            "STN EN ISO 10211, 12.4.3.1".into(),
        ));
        rows.push((
            "Uw · hw".into(),
            String::new(),
            format!("{} W/(m²·K) · {} m", num(g.uw, 4), num(g.hw, 3)),
            "STN EN ISO 10211, 12.4.3.1".into(),
        ));
        rows.push((
            t("Linear thermal transmittance Ψg"),
            "W/(m·K)".into(),
            num(*psi, 4),
            t("option B, inside dimensions, STN EN ISO 10211 (23)"),
        ));
    } else {
        match &o.psi {
            Ok((parts, psi)) => {
                let dims = if o.external {
                    t("outside dimensions")
                } else {
                    t("inside dimensions")
                };
                for (k, (f, l)) in parts.iter().enumerate() {
                    rows.push((
                        format!("U{} · l{}", k + 1, k + 1),
                        String::new(),
                        format!("{} W/(m²·K) · {} m", num(f.u, 4), num(*l, 3)),
                        "STN EN ISO 10211, 12.3".into(),
                    ));
                }
                rows.push((
                    t("Linear thermal transmittance Ψ"),
                    "W/(m·K)".into(),
                    num(*psi, 4),
                    format!("{dims}, STN EN ISO 10211 (18)"),
                ));
            }
            Err(e) => rows.push((
                t("Linear thermal transmittance Ψ"),
                String::new(),
                "–".into(),
                e.clone(),
            )),
        }
    }
    if let Some(m) = o.min {
        rows.push((
            t("Lowest inner surface temperature θsi,min"),
            "°C".into(),
            num(m.theta, 2),
            String::new(),
        ));
    }
    if let Some(f) = o.frsi {
        rows.push((
            t("Temperature factor fRsi"),
            "–".into(),
            num(f, 3),
            "STN EN ISO 10211 (32)".into(),
        ));
    }
    match &o.frsi_n {
        Ok(n) => {
            rows.push((
                t("Required fRsi,N"),
                "–".into(),
                num(*n, 3),
                "STN 73 0540-2, tab. 5".into(),
            ));
            if let Some(f) = o.frsi {
                rows.push((
                    t("fRsi ≥ fRsi,N (7)"),
                    String::new(),
                    yes_no(f >= *n - 1e-12),
                    String::new(),
                ));
            }
        }
        Err(e) => rows.push((
            t("Required fRsi,N"),
            "–".into(),
            "–".into(),
            format!("{} ({e})", t("outside table 5")),
        )),
    }
    if let (Some(t80), Some(d)) = (o.theta_si80, o.delta) {
        rows.push((
            t("Critical surface temperature θsi,80"),
            "°C".into(),
            num(t80, 2),
            format!("φi {} %, STN 73 0540-3 (10)", num(s.phi_i, 0)),
        ));
        rows.push((
            t("Safety margin Δθsi"),
            "K".into(),
            num(d, 1),
            "STN 73 0540-2, tab. 4".into(),
        ));
        rows.push((
            t("Required θsi,N = θsi,80 + Δθsi"),
            "°C".into(),
            num(t80 + d, 2),
            "STN 73 0540-2 (6)".into(),
        ));
        if let Some(m) = o.min {
            let note = match o.phi_max {
                Some(p) if s.phi_i > p => t("φi over 80 %: not proven by calculation (čl. 5.3.1)"),
                _ => String::new(),
            };
            rows.push((
                t("θsi,min ≥ θsi,N (6)"),
                String::new(),
                yes_no(m.theta >= t80 + d - 1e-12),
                note,
            ));
        }
    }
    if let Some(c) = o.mesh {
        rows.push((
            t("Mesh check: heat flow change"),
            "%".into(),
            num(100.0 * c.flow_change, 2),
            format!("{} {}", t("nodes, halved:"), o.mesh_fine_nodes),
        ));
        rows.push((
            t("Mesh check: fRsi change"),
            "–".into(),
            num(c.frsi_change, 4),
            yes_no(c.ok),
        ));
    }
    rows.push((
        t("Nodes"),
        String::new(),
        o.field.nodes().to_string(),
        format!("{} {:.1e}", t("balance (C.2 e)"), o.field.balance),
    ));
    for m in &o.detail.materials {
        rows.push((
            format!("λ – {}", m.name),
            "W/(m·K)".into(),
            num(m.lambda, 3),
            String::new(),
        ));
    }
    let cols = [
        Col::text("Physical quantity"),
        Col::num("Value", ""),
        Col::text("Source"),
    ];
    sheet::show(
        ui,
        "thermal.results",
        &cols,
        rows.len(),
        520.0,
        |row, i| {
            let r = &rows[i];
            row.text(r.0.clone());
            row.strong(if r.1.is_empty() || r.1 == "–" {
                r.2.clone()
            } else {
                format!("{} {}", r.2, r.1)
            });
            row.weak(r.3.clone());
        },
        None,
    );
}
