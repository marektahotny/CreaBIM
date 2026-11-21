//! The summary report: part B of the documentation of a construction
//! (decree 60/2025 Z. z., annex 17), set out in the decree's nine
//! chapters and filled with what the project knows: who and where from
//! the project information, storeys, areas, heights and rooms from the
//! model, the walls' and floors' build-ups from their types. What the
//! model cannot know stands as `[doplniť]` for the designer to write.
//! The decree is Slovak, so is the report.

use creabim_core::{levels, room, ElementKind, Model, OpeningKind, ScheduleCategory};

/// The mark of what the designer still has to write.
pub const FILL: &str = "[doplniť]";

/// One chapter: its heading and its paragraphs. A paragraph starting
/// with `## ` is a heading under it.
#[derive(Clone, Debug, PartialEq)]
pub struct Chapter {
    pub heading: String,
    pub paragraphs: Vec<String>,
}

/// The figures the report takes from the model.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Figures {
    /// Storeys above and below ground (a storey's floor more than 0.8 m
    /// under the project's zero is below).
    pub above: usize,
    pub below: usize,
    /// What the walls of the first storey above ground enclose, the
    /// walls with it, m² (measured on a fine grid: within a per cent).
    pub built_up: f64,
    /// All rooms' floor areas, m², and each storey's.
    pub floor_area: f64,
    pub by_storey: Vec<(String, f64, usize)>,
    /// The highest wall top over the project's zero, m.
    pub height: f64,
    pub windows: usize,
    pub doors: usize,
    /// The enclosed volume by STN 73 4055, m³: Op = Oz + Os + Ov + Ot.
    pub volume: f64,
    /// Its parts.
    pub cubature: Cubature,
    /// Hard landscape of the site, m².
    pub paved: f64,
    /// The plot's area from the project information, m².
    pub plot: Option<f64>,
    /// The ground at the building against the project's zero, m: its
    /// lowest and highest height at the corners of the ground storey.
    pub ground: Option<(f64, f64)>,
}

impl Figures {
    /// Built-up area over the plot's (index zastavanosti).
    pub fn coverage(&self) -> Option<f64> {
        self.plot.filter(|p| *p > 0.0).map(|p| self.built_up / p)
    }

    /// What of the plot stays green: not built on, not paved.
    pub fn green(&self) -> Option<f64> {
        self.plot
            .filter(|p| *p > 0.0)
            .map(|p| (p - self.built_up - self.paved).max(0.0) / p)
    }
}

/// The ground at the building against the project's zero: the lowest
/// and highest height of the terrain at the ends of the ground storey's
/// walls. None without a terrain under them.
pub fn ground_at_building(model: &Model) -> Option<(f64, f64)> {
    use creabim_core::site::{surface_height, SiteForm};
    let walls: Vec<&creabim_core::Wall> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Wall(w) => Some(w),
            _ => None,
        })
        .collect();
    let ground = levels::sorted(model)
        .into_iter()
        .filter(|(id, ..)| walls.iter().any(|w| w.level == *id))
        .find(|(.., l)| l.elevation >= -0.8)?
        .0;
    let mut range: Option<(f64, f64)> = None;
    for e in model.iter() {
        let ElementKind::Site(s) = &e.kind else {
            continue;
        };
        let SiteForm::Terrain {
            boundary,
            top,
            points,
            ..
        } = &s.form
        else {
            continue;
        };
        let base = model.level(s.level).map_or(0.0, |l| l.elevation);
        for p in walls
            .iter()
            .filter(|w| w.level == ground)
            .flat_map(|w| [w.start, w.end])
        {
            if !creabim_core::polygon::contains(boundary, p) {
                continue;
            }
            let z = base + surface_height(*top, points, p);
            range = Some(range.map_or((z, z), |(lo, hi)| (lo.min(z), hi.max(z))));
        }
    }
    range
}

/// The enclosed volume of the building by STN 73 4055: Op = Oz + Os + Ov
/// + Ot, each measured to the outer faces of the walls (the "šírka" of
/// the norm's figure), m³. The planes between the parts, as the norm's
/// figure draws them:
/// - the plane bounding the foundations: the top of the footings;
/// - the plane bounding the lower part: the floor of the first storey
///   above ground (1. NP), so the lower part is the storeys below ground
///   and what lies between the footings and that floor;
/// - the plane bounding the upper part: the tops of the top storey's
///   walls, on which the roof stands.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cubature {
    /// Oz: the footings, their own volume (the norm's "zrovnaná hrúbka
    /// kubatúry základov" over the footprint).
    pub foundations: f64,
    /// Os: the lower part.
    pub lower: f64,
    /// Ov: the upper part.
    pub upper: f64,
    /// Ot: the roofing, under the roof's upper face over the top
    /// storey's footprint.
    pub roofing: f64,
}

impl Cubature {
    /// Op, the basic enclosed volume.
    pub fn total(&self) -> f64 {
        self.foundations + self.lower + self.upper + self.roofing
    }
}

/// The closed meshes of the footings, their volume summed, m³.
fn footings_volume(model: &Model) -> f64 {
    let footings: Vec<_> = model
        .iter()
        .filter(|e| matches!(e.kind, ElementKind::Footing(_)))
        .map(|e| e.id)
        .collect();
    if footings.is_empty() {
        return 0.0;
    }
    let mut v = 0.0;
    for it in crate::scene::build_scene(model) {
        if !footings.contains(&it.id) || it.ghost {
            continue;
        }
        let m = &it.mesh;
        let p = |i: u32| {
            let q = m.positions[i as usize];
            glam::DVec3::new(q[0] as f64, q[1] as f64, q[2] as f64)
        };
        let mut one = 0.0;
        for t in m.indices.chunks_exact(3) {
            one += p(t[0]).dot(p(t[1]).cross(p(t[2]))) / 6.0;
        }
        v += one.abs();
    }
    v
}

/// The height of the roofs' upper faces over each cell of `fp`, less
/// `z`, summed by the cells' area, m³.
fn roofing_over(model: &Model, fp: &Footprint, z: f64) -> f64 {
    let roofs: Vec<_> = model
        .iter()
        .filter(|e| matches!(e.kind, ElementKind::Roof(_)))
        .map(|e| e.id)
        .collect();
    if roofs.is_empty() {
        return 0.0;
    }
    // the highest upper face over each cell
    let mut top = vec![f64::NEG_INFINITY; fp.inside.len()];
    for it in crate::scene::build_scene(model) {
        if !roofs.contains(&it.id) || it.layer.is_some_and(|l| l.index > 0) {
            continue;
        }
        let m = &it.mesh;
        for t in m.indices.chunks_exact(3) {
            if m.normals[t[0] as usize][2] < 0.2 {
                continue;
            }
            let p = |i: u32| {
                let q = m.positions[i as usize];
                glam::DVec3::new(q[0] as f64, q[1] as f64, q[2] as f64)
            };
            let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
            let d = (b - a).truncate().perp_dot((c - a).truncate());
            if d.abs() < 1e-12 {
                continue;
            }
            let lo = a.truncate().min(b.truncate()).min(c.truncate());
            let hi = a.truncate().max(b.truncate()).max(c.truncate());
            let i0 = ((lo.x - fp.lo.x) / fp.cell).floor().max(0.0) as usize;
            let j0 = ((lo.y - fp.lo.y) / fp.cell).floor().max(0.0) as usize;
            let i1 = (((hi.x - fp.lo.x) / fp.cell).ceil().max(0.0) as usize).min(fp.nx);
            let j1 = (((hi.y - fp.lo.y) / fp.cell).ceil().max(0.0) as usize).min(fp.ny);
            for j in j0..j1 {
                for i in i0..i1 {
                    let k = j * fp.nx + i;
                    if !fp.inside[k] {
                        continue;
                    }
                    let q = fp.lo + glam::DVec2::new(i as f64 + 0.5, j as f64 + 0.5) * fp.cell;
                    // barycentric in plan
                    let u = (b - a).truncate().perp_dot(q - a.truncate()) / d;
                    let v = (q - a.truncate()).perp_dot((c - a).truncate()) / d;
                    let (l1, l2) = (v, u);
                    let l0 = 1.0 - l1 - l2;
                    if l0 < -1e-9 || l1 < -1e-9 || l2 < -1e-9 {
                        continue;
                    }
                    let h = a.z * l0 + b.z * l1 + c.z * l2;
                    top[k] = top[k].max(h);
                }
            }
        }
    }
    top.iter()
        .filter(|h| h.is_finite())
        .map(|h| (h - z).max(0.0))
        .sum::<f64>()
        * fp.cell_area()
}

/// The building's enclosed volume by STN 73 4055 (see [`Cubature`]).
pub fn cubature(model: &Model) -> Cubature {
    let walls: Vec<&creabim_core::Wall> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Wall(w) => Some(w),
            _ => None,
        })
        .collect();
    let storeys: Vec<_> = levels::sorted(model)
        .into_iter()
        .filter(|(id, ..)| walls.iter().any(|w| w.level == *id))
        .collect();
    let mut c = Cubature::default();
    let Some(lowest) = storeys.first() else {
        return c;
    };
    let prints: Vec<Option<Footprint>> = storeys
        .iter()
        .map(|(id, ..)| {
            let here: Vec<&creabim_core::Wall> =
                walls.iter().filter(|w| w.level == *id).copied().collect();
            Footprint::of(&here)
        })
        .collect();
    let area = |i: usize| prints[i].as_ref().map_or(0.0, Footprint::area);
    // the plane bounding the lower part: the floor of 1. NP; the
    // storeys under it are the lower part
    let first_up = storeys
        .iter()
        .position(|(id, ..)| room::storey_number(model, *id) >= 1)
        .unwrap_or(0);
    // the plane bounding the foundations: the footings' tops under the
    // lowest storey
    let z_found = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Footing(f) => Some(f.top(model)),
            _ => None,
        })
        .filter(|z| *z <= lowest.2.elevation + 1e-6)
        .fold(f64::NEG_INFINITY, f64::max);
    let z_found = if z_found.is_finite() {
        z_found
    } else {
        lowest.2.elevation
    };
    c.foundations = footings_volume(model);
    // between the footings and the lowest storey's floor
    c.lower += area(0) * (lowest.2.elevation - z_found).max(0.0);
    let top_index = storeys.len() - 1;
    for (i, (id, _, l)) in storeys.iter().enumerate() {
        let height = if i < top_index {
            storeys[i + 1].2.elevation - l.elevation
        } else {
            walls
                .iter()
                .filter(|w| w.level == *id)
                .map(|w| w.base_offset + w.height)
                .fold(0.0, f64::max)
        };
        let v = area(i) * height.max(0.0);
        if i < first_up {
            c.lower += v;
        } else {
            c.upper += v;
        }
    }
    let top = &storeys[top_index];
    let z_upper = top.2.elevation
        + walls
            .iter()
            .filter(|w| w.level == top.0)
            .map(|w| w.base_offset + w.height)
            .fold(0.0, f64::max);
    if let Some(fp) = &prints[top_index] {
        c.roofing = roofing_over(model, fp, z_upper);
    }
    c
}

pub fn figures(model: &Model) -> Figures {
    let mut f = Figures::default();
    let walls: Vec<&creabim_core::Wall> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Wall(w) => Some(w),
            _ => None,
        })
        .collect();
    let storeys: Vec<_> = levels::sorted(model)
        .into_iter()
        .filter(|(id, ..)| walls.iter().any(|w| w.level == *id))
        .collect();
    for (id, name, l) in &storeys {
        if l.elevation < -0.8 {
            f.below += 1;
        } else {
            f.above += 1;
        }
        let (mut area, mut n) = (0.0, 0);
        for e in model.iter() {
            let ElementKind::Room(r) = &e.kind else {
                continue;
            };
            if r.level != *id {
                continue;
            }
            if let Some(s) = room::shape(model, e.id) {
                area += room::net_area(model, r, &s);
                n += 1;
            }
        }
        f.floor_area += area;
        f.by_storey.push((creabim_core::words::tr(name), area, n));
        let here: Vec<&creabim_core::Wall> =
            walls.iter().filter(|w| w.level == *id).copied().collect();
        for w in &here {
            f.height = f.height.max(l.elevation + w.base_offset + w.height);
        }
    }
    f.built_up = figures_built_up(model);
    f.cubature = cubature(model);
    f.volume = f.cubature.total();
    for e in model.iter() {
        if let ElementKind::Site(s) = &e.kind {
            if let creabim_core::site::SiteForm::Area {
                boundary, holes, ..
            } = &s.form
            {
                let a = |p: &[glam::DVec2]| creabim_core::polygon::signed_area2(p).abs() / 2.0;
                f.paved += a(boundary) - holes.iter().map(|h| a(h)).sum::<f64>();
            }
        }
    }
    f.plot = model.project_info().and_then(|p| p.1.plot_area);
    f.ground = ground_at_building(model);
    for e in model.iter() {
        if let ElementKind::Opening(o) = &e.kind {
            match o.kind {
                OpeningKind::Window => f.windows += 1,
                OpeningKind::Door => f.doors += 1,
                _ => {}
            }
        }
    }
    f
}

/// The area `walls` enclose, themselves with it: the plan is laid on a
/// grid, the walls' cells marked, and what cannot be reached from
/// outside counted. Walls that do not close enclose only themselves.
fn enclosed(walls: &[&creabim_core::Wall]) -> f64 {
    Footprint::of(walls).map_or(0.0, |f| f.area())
}

/// What a storey's walls enclose, themselves with it, as cells of a
/// grid on the plan: its outline at the outer faces of its walls.
pub struct Footprint {
    lo: glam::DVec2,
    cell: f64,
    nx: usize,
    ny: usize,
    inside: Vec<bool>,
}

impl Footprint {
    pub fn area(&self) -> f64 {
        self.inside.iter().filter(|c| **c).count() as f64 * self.cell * self.cell
    }

    /// The middle of every cell inside, with the cell's area.
    pub fn cells(&self) -> impl Iterator<Item = glam::DVec2> + '_ {
        self.inside
            .iter()
            .enumerate()
            .filter(|c| *c.1)
            .map(|(k, _)| {
                self.lo
                    + glam::DVec2::new((k % self.nx) as f64 + 0.5, (k / self.nx) as f64 + 0.5)
                        * self.cell
            })
    }

    pub fn cell_area(&self) -> f64 {
        self.cell * self.cell
    }

    fn of(walls: &[&creabim_core::Wall]) -> Option<Footprint> {
        let first = walls.first()?;
        let (lo, hi) = walls
            .iter()
            .flat_map(|w| [w.start, w.end])
            .fold((first.start, first.start), |(lo, hi), p| {
                (lo.min(p), hi.max(p))
            });
        let thick = walls.iter().map(|w| w.thickness).fold(0.0, f64::max);
        let (lo, hi) = (lo - thick - 0.2, hi + thick + 0.2);
        let size = hi - lo;
        let cell = (size.max_element() / 3000.0).max(0.01);
        let (nx, ny) = (
            (size.x / cell).ceil() as usize + 1,
            (size.y / cell).ceil() as usize + 1,
        );
        // 0 open, 1 wall, 2 reached from outside
        let mut grid = vec![0u8; nx * ny];
        for w in walls {
            let half = w.thickness / 2.0;
            let (a, b) = (w.start, w.end);
            let d = b - a;
            let len2 = d.length_squared().max(1e-12);
            let (wlo, whi) = (a.min(b) - half, a.max(b) + half);
            let ix = |x: f64| (((x - lo.x) / cell).floor().max(0.0) as usize).min(nx - 1);
            let iy = |y: f64| (((y - lo.y) / cell).floor().max(0.0) as usize).min(ny - 1);
            for j in iy(wlo.y)..=iy(whi.y) {
                for i in ix(wlo.x)..=ix(whi.x) {
                    let p = lo + glam::DVec2::new(i as f64 + 0.5, j as f64 + 0.5) * cell;
                    let t = ((p - a).dot(d) / len2).clamp(0.0, 1.0);
                    // square ends: half a thickness along too
                    let q = p - (a + d * t);
                    let along = q.dot(d) / len2.sqrt();
                    let across = (q - d / len2.sqrt() * along).length();
                    if across <= half && along.abs() <= half {
                        grid[j * nx + i] = 1;
                    }
                }
            }
        }
        let mut stack = vec![0usize];
        grid[0] = 2;
        while let Some(k) = stack.pop() {
            let (i, j) = (k % nx, k / nx);
            let mut go = |i: usize, j: usize| {
                let k = j * nx + i;
                if grid[k] == 0 {
                    grid[k] = 2;
                    stack.push(k);
                }
            };
            if i > 0 {
                go(i - 1, j);
            }
            if i + 1 < nx {
                go(i + 1, j);
            }
            if j > 0 {
                go(i, j - 1);
            }
            if j + 1 < ny {
                go(i, j + 1);
            }
        }
        Some(Footprint {
            lo,
            cell,
            nx,
            ny,
            inside: grid.iter().map(|c| *c != 2).collect(),
        })
    }
}

fn m2(v: f64) -> String {
    format!("{v:.2} m²").replace('.', ",")
}

/// The build-ups of the model's layered walls, floors and roofs: each
/// type in use once, "name: layers".
fn build_ups(model: &Model) -> Vec<String> {
    let mut seen = vec![];
    let mut out = vec![];
    for e in model.iter() {
        if !matches!(
            e.kind,
            ElementKind::Wall(_) | ElementKind::Slab(_) | ElementKind::Roof(_)
        ) {
            continue;
        }
        let Some((t, a)) = creabim_core::assembly_of(model, e) else {
            continue;
        };
        if seen.contains(&t.id) {
            continue;
        }
        seen.push(t.id);
        out.push(format!("{}: {}", t.name, a.summary(model)));
    }
    out
}

/// The report of `model`, chapter by chapter.
pub fn report(model: &Model) -> Vec<Chapter> {
    let info = model
        .project_info()
        .map(|p| p.1.clone())
        .unwrap_or_default();
    let f = figures(model);
    let or_fill = |s: &str| {
        if s.trim().is_empty() {
            FILL.to_string()
        } else {
            s.trim().to_string()
        }
    };
    let line = |label: &str, value: &str| format!("{label}: {}", or_fill(value));
    let name = if info.name.trim().is_empty() {
        &info.building_name
    } else {
        &info.name
    };
    let chapter = |heading: &str, paragraphs: Vec<String>| Chapter {
        heading: heading.to_string(),
        paragraphs,
    };
    let mut out = vec![];

    out.push(chapter(
        "1. Identifikačné údaje",
        vec![
            "## 1.1 Stavba".into(),
            line("ID stavby", ""),
            line("Názov stavby", name),
            line("Miesto stavby", &info.address),
            line("Katastrálne územie", &info.cadastre),
            line("Parcelné čísla stavebných pozemkov", &info.parcels),
            line("List vlastníctva", ""),
            line("Identifikačný kód a typ stavby", ""),
            format!("Charakter stavby: nová stavba / zmena dokončenej stavby {FILL}"),
            format!("Trvalosť stavby: trvalá {FILL}"),
            line("Stupeň dokumentácie", &info.status),
            line("Dátum", &info.issue_date),
            "## 1.2 Stavebník".into(),
            line("Stavebník", &info.client),
            line("Vzťah k pozemkom a k existujúcim stavbám", ""),
            "## 1.3 Projektant".into(),
            line("Generálny projektant", &info.architect),
            line("Zodpovedný projektant", &info.designer),
            line("Vypracoval", &info.drawn_by),
            line("Projektanti jednotlivých častí", ""),
        ],
    ));

    let mut storeys = vec![];
    if f.above > 0 {
        storeys.push(format!("{} nadzemné", f.above));
    }
    if f.below > 0 {
        storeys.push(format!("{} podzemné", f.below));
    }
    out.push(chapter(
        "2. Údaje o stavbe a jej prevádzke",
        vec![
            line("Účel stavby", &info.building_use),
            line("Počet podlaží", &storeys.join(", ")),
            format!("Počet bytov a ich veľkosť: {FILL}"),
            format!("Prevádzka stavby a jej zariadenia: {FILL}"),
        ],
    ));

    out.push(chapter(
        "3. Súlad s územným plánom a s rozhodnutím o stavebnom zámere",
        vec![
            format!("Územnoplánovacia dokumentácia a súlad s jej záväznou časťou: {FILL}"),
            format!(
                "Rozhodnutie o stavebnom zámere (číslo, dátum vydania a právoplatnosti): {FILL}"
            ),
            format!("Splnenie podmienok rozhodnutia a odchýlky od stavebného zámeru: {FILL}"),
        ],
    ));

    out.push(chapter(
        "4. Východiskové podklady a prieskumy",
        vec![
            format!("Geodetické zameranie územia: {FILL}"),
            "Katastrálna mapa a údaje katastra nehnuteľností.".into(),
            format!("Ďalšie podklady a prieskumy (inžinierskogeologický, radónový, vyjadrenia správcov sietí): {FILL}"),
        ],
    ));

    out.push(chapter(
        "5. Členenie stavby",
        vec![
            format!("S01 {}", or_fill(name)),
            format!("Ďalšie stavebné objekty a prevádzkové súbory (prípojky, spevnené plochy, oplotenie): {FILL}"),
            format!("Zmeny oproti stavebnému zámeru a ich odôvodnenie: {FILL}"),
        ],
    ));

    let mut capacity = vec![
        format!(
            "Zastavaná plocha (obrys stien 1. NP z modelu): {}",
            m2(f.built_up)
        ),
        match f.plot {
            Some(p) => format!("Plocha pozemku (z katastra): {}", m2(p)),
            None => format!("Plocha pozemku (z katastra): {FILL}"),
        },
        format!("Spevnené plochy: {}", m2(f.paved)),
        match (f.coverage(), f.green()) {
            (Some(c), Some(g)) => format!(
                "Index zastavanosti: {}, koeficient zelene: {}",
                format!("{c:.2}").replace('.', ","),
                format!("{g:.2}").replace('.', ",")
            ),
            _ => format!("Index zastavanosti a koeficient zelene: {FILL} (chýba plocha pozemku)"),
        },
        format!("Podlahová plocha miestností spolu: {}", m2(f.floor_area)),
    ];
    for (name, area, rooms) in &f.by_storey {
        capacity.push(format!("{name}: {}, miestností: {rooms}", m2(*area)));
    }
    capacity.extend([
        format!(
            "Výška stavby nad ±0,000 (najvyššia koruna steny): {} m",
            format!("{:.2}", f.height).replace('.', ",")
        ),
        format!(
            "±0,000 = {} m n. m.",
            format!("{:.2}", info.elevation).replace('.', ",")
        ),
        match f.ground {
            Some((lo, hi)) => {
                let s = |v: f64| format!("{v:+.2}").replace('.', ",").replace('-', "−");
                if (hi - lo).abs() < 0.005 {
                    format!(
                        "Osadenie stavby: terén pri objekte {} m voči ±0,000 ({} m n. m.)",
                        s(lo),
                        format!("{:.2}", info.elevation + lo).replace('.', ",")
                    )
                } else {
                    format!(
                        "Osadenie stavby: terén pri objekte {} až {} m voči ±0,000 ({} až {} m n. m.)",
                        s(lo),
                        s(hi),
                        format!("{:.2}", info.elevation + lo).replace('.', ","),
                        format!("{:.2}", info.elevation + hi).replace('.', ",")
                    )
                }
            }
            None => format!("Osadenie stavby voči terénu: {FILL} (v modeli nie je terén pod stavbou)"),
        },
        match crate::firezones::fire_height(model) {
            Some(h) => format!(
                "Požiarna výška stavby: {} m (nadzemné požiarne podlažia: {}, podzemné: {}{})",
                format!("{:.2}", h.height).replace('.', ","),
                h.above,
                h.below,
                if h.ground.is_some() {
                    ""
                } else {
                    "; bez terénu v modeli, terén uvažovaný na ±0,000"
                }
            ),
            None => format!("Požiarna výška stavby: {FILL}"),
        },
        {
            let rooms = crate::occupancy::rooms(model);
            match crate::occupancy::table(model) {
                Some(t) if !rooms.is_empty() => {
                    let (zones, total) = crate::occupancy::by_zone(&rooms);
                    let open = rooms.iter().filter(|r| r.persons.is_none()).count();
                    format!(
                        "Obsadenie stavby osobami: {total} osôb v {} požiarnych úsekoch podľa {} ({}{}{})",
                        zones.len(),
                        t.norm,
                        t.edition,
                        if t.changed.is_empty() {
                            ""
                        } else {
                            "; hodnoty tabuľky boli ručne zmenené"
                        },
                        if open == 0 {
                            String::new()
                        } else {
                            format!("; {open} miestností bez určeného počtu")
                        }
                    )
                }
                _ => format!("Obsadenie stavby osobami: {FILL}"),
            }
        },
        {
            let m3 = |v: f64| format!("{v:.1} m³").replace('.', ",");
            let c = &f.cubature;
            format!(
                "Obostavaný priestor podľa STN 73 4055: Op = Oz + Os + Ov + Ot = {} + {} + {} + {} = {} (základy, spodná časť, vrchná časť, zastrešenie)",
                m3(c.foundations),
                m3(c.lower),
                m3(c.upper),
                m3(c.roofing),
                m3(c.total())
            )
        },
        format!(
            "Potreba tepla a energií, pitnej a úžitkovej vody, množstvo odpadových vôd: {FILL}"
        ),
    ]);
    out.push(chapter("6. Kapacitné údaje a bilancie", capacity));

    let mut body = vec![
        format!("Urbanistické a architektonické riešenie stavby a jej okolia: {FILL}"),
        "## Stavebné riešenie".into(),
    ];
    let ups = build_ups(model);
    if ups.is_empty() {
        body.push(format!("Skladby stien, podláh a strechy: {FILL}"));
    } else {
        body.push("Skladby konštrukcií podľa modelu:".into());
        body.extend(ups);
    }
    body.push(format!(
        "Výplne otvorov: okná {} ks, dvere {} ks, podrobne vo výpisoch okien a dverí.",
        f.windows, f.doors
    ));
    body.extend([
        "## Technické vybavenie a napojenie na siete".into(),
        format!("Vodovod, kanalizácia, vykurovanie, vetranie, plyn, elektroinštalácia a ich prípojky: {FILL}"),
        "## Ďalšie údaje".into(),
        format!("Parkovanie a dopravné napojenie: {FILL}"),
        format!("Bezbariérové užívanie: {FILL}"),
        format!("Úpravy terénu a zelene, nakladanie so zrážkovými vodami: {FILL}"),
        format!("Postup výstavby a bezpečnosť a ochrana zdravia pri práci: {FILL}"),
    ]);
    out.push(chapter("7. Popis stavby", body));

    out.push(chapter(
        "8. Vplyv stavby na životné prostredie a chránené záujmy",
        vec![
            format!("Chránené územia a ochranné pásma: {FILL}"),
            format!("Nakladanie s odpadmi počas výstavby a užívania stavby: {FILL}"),
        ],
    ));

    out.push(chapter(
        "9. Predpokladané náklady stavby",
        vec![
            format!("Celkové náklady stavby: {FILL}"),
            format!("Spôsob financovania (pri verejných zdrojoch): {FILL}"),
        ],
    ));
    out
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The report as the files of a Word document (.docx), each with its
/// name in the zip: headings in Word's own heading styles, so the text
/// is edited and its contents listed as in any other.
pub fn docx_files(title: &str, chapters: &[Chapter]) -> Vec<(String, Vec<u8>)> {
    let para = |style: &str, text: &str| {
        let style = if style.is_empty() {
            String::new()
        } else {
            format!("<w:pPr><w:pStyle w:val=\"{style}\"/></w:pPr>")
        };
        format!(
            "<w:p>{style}<w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
            xml(text)
        )
    };
    let mut body = para("Title", title);
    for c in chapters {
        body += &para("Heading1", &c.heading);
        for p in &c.paragraphs {
            body += &match p.strip_prefix("## ") {
                Some(h) => para("Heading2", h),
                None => para("", p),
            };
        }
    }
    let ns = "xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"";
    let head = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>";
    let document = format!(
        "{head}<w:document {ns}><w:body>{body}<w:sectPr><w:pgSz w:w=\"11906\" w:h=\"16838\"/>\
         <w:pgMar w:top=\"1417\" w:right=\"1417\" w:bottom=\"1417\" w:left=\"1417\" \
         w:header=\"708\" w:footer=\"708\" w:gutter=\"0\"/></w:sectPr></w:body></w:document>"
    );
    let style = |id: &str, name: &str, size: u32, bold: bool, before: u32| {
        format!(
            "<w:style w:type=\"paragraph\" w:styleId=\"{id}\"><w:name w:val=\"{name}\"/>\
             <w:basedOn w:val=\"Normal\"/><w:next w:val=\"Normal\"/><w:qFormat/>\
             <w:pPr><w:keepNext/><w:spacing w:before=\"{before}\" w:after=\"120\"/></w:pPr>\
             <w:rPr>{}<w:sz w:val=\"{size}\"/></w:rPr></w:style>",
            if bold { "<w:b/>" } else { "" }
        )
    };
    let styles = format!(
        "{head}<w:styles {ns}><w:docDefaults><w:rPrDefault><w:rPr>\
         <w:rFonts w:ascii=\"Arial\" w:hAnsi=\"Arial\" w:cs=\"Arial\"/><w:sz w:val=\"22\"/>\
         <w:lang w:val=\"sk-SK\"/></w:rPr></w:rPrDefault><w:pPrDefault><w:pPr>\
         <w:spacing w:after=\"100\"/></w:pPr></w:pPrDefault></w:docDefaults>\
         <w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\">\
         <w:name w:val=\"Normal\"/><w:qFormat/></w:style>{}{}{}</w:styles>",
        style("Title", "Title", 36, true, 0),
        style("Heading1", "heading 1", 28, true, 360),
        style("Heading2", "heading 2", 24, true, 200),
    );
    let types = format!(
        "{head}<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
         <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
         <Default Extension=\"xml\" ContentType=\"application/xml\"/>\
         <Override PartName=\"/word/document.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml\"/>\
         <Override PartName=\"/word/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml\"/>\
         </Types>"
    );
    let rel = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
    let rels = format!(
        "{head}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
         <Relationship Id=\"rId1\" Type=\"{rel}/officeDocument\" Target=\"word/document.xml\"/></Relationships>"
    );
    let doc_rels = format!(
        "{head}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
         <Relationship Id=\"rId1\" Type=\"{rel}/styles\" Target=\"styles.xml\"/></Relationships>"
    );
    vec![
        ("[Content_Types].xml".into(), types.into_bytes()),
        ("_rels/.rels".into(), rels.into_bytes()),
        ("word/document.xml".into(), document.into_bytes()),
        ("word/styles.xml".into(), styles.into_bytes()),
        ("word/_rels/document.xml.rels".into(), doc_rels.into_bytes()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_report_has_the_decrees_nine_chapters_filled_from_the_model() {
        let m = creabim_core::sample::sample_house();
        let r = report(&m);
        assert_eq!(r.len(), 9);
        assert!(r[0].heading.starts_with("1. "));
        let f = figures(&m);
        assert!(f.above >= 1 && f.height > 2.0, "{f:?}");
        assert!(f.windows + f.doors > 0);
        // a 6 x 4 m box of 0.3 m walls on their centre lines: 6.3 x 4.3 m
        let p = glam::DVec2::new;
        let l = creabim_core::ElementId::NIL;
        let c = [p(0.0, 0.0), p(6.0, 0.0), p(6.0, 4.0), p(0.0, 4.0)];
        let walls: Vec<creabim_core::Wall> = (0..4)
            .map(|i| creabim_core::Wall::new(l, c[i], c[(i + 1) % 4], 0.3, 3.0))
            .collect();
        let area = enclosed(&walls.iter().collect::<Vec<_>>());
        assert!((area - 6.3 * 4.3).abs() < 0.3, "{area}");
        // what the model cannot know is left to fill
        assert!(r[8].paragraphs.iter().all(|p| p.contains(FILL)));
        let files = docx_files("B Súhrnná správa", &r);
        let doc = files.iter().find(|f| f.0 == "word/document.xml").unwrap();
        let text = String::from_utf8(doc.1.clone()).unwrap();
        assert!(text.contains("Heading1") && text.contains("Identifikačné údaje"));
        assert!(!text.contains("## "));
    }
}

/// The building's figures as a table: built-up area, living and usable
/// area, the parts of the enclosed volume by STN 73 4055 and Op under
/// them.
pub fn figures_schedule(model: &Model) -> crate::schedule::Schedule {
    use crate::schedule::{Column, Row, Schedule};
    use creabim_core::words::tr;
    let units = model.units();
    let c = cubature(model);
    let built = figures_built_up(model);
    let under = crate::roomlegend::Undersides::of(model);
    let (mut living, mut usable) = (0.0, 0.0);
    for e in model.iter() {
        let ElementKind::Room(r) = &e.kind else {
            continue;
        };
        let a = crate::roomlegend::counted_area(model, &under, r);
        if !room::is_open_air(&e.name, r) {
            usable += a;
        }
        if room::counts_as_living(&e.name, r, a) {
            living += a;
        }
    }
    let m3 = |v: f64| format!("{v:.2} m³");
    let row = |what: &str, value: String| Row {
        id: creabim_core::ElementId::NIL,
        group: false,
        cells: vec![tr(what), value],
    };
    let rows = vec![
        row("Built-up area", units.area(built)),
        row("Living area", units.area(living)),
        row("Usable area", units.area(usable)),
        row("Enclosed volume of the foundations Oz", m3(c.foundations)),
        row("Enclosed volume of the lower part Os", m3(c.lower)),
        row("Enclosed volume of the upper part Ov", m3(c.upper)),
        row("Enclosed volume of the roofing Ot", m3(c.roofing)),
    ];
    Schedule {
        category: ScheduleCategory::BuildingFigures,
        columns: vec![
            Column {
                title: "Figure",
                field: None,
                width: 70.0,
                numeric: false,
                align: None,
            },
            Column {
                title: "Value",
                field: None,
                width: 30.0,
                numeric: true,
                align: None,
            },
        ],
        listed: rows.len(),
        rows,
        totals: vec![tr("Enclosed volume Op = Oz + Os + Ov + Ot"), m3(c.total())],
        above_totals: vec![],
    }
}

/// The built-up area: what the walls of the first storey above ground
/// (1. NP) enclose, the walls with it, m².
fn figures_built_up(model: &Model) -> f64 {
    let walls: Vec<&creabim_core::Wall> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Wall(w) => Some(w),
            _ => None,
        })
        .collect();
    let Some(first) = levels::sorted(model)
        .into_iter()
        .filter(|(id, ..)| walls.iter().any(|w| w.level == *id))
        .find(|(id, ..)| room::storey_number(model, *id) >= 1)
    else {
        return 0.0;
    };
    let here: Vec<&creabim_core::Wall> = walls
        .iter()
        .filter(|w| w.level == first.0)
        .copied()
        .collect();
    enclosed(&here)
}

#[cfg(test)]
mod cubature_tests {
    use super::*;

    /// Op is the four parts of STN 73 4055 summed; with footings under
    /// the ground floor the lower part is the band between their tops
    /// and the floor of 1. NP, and the roof stands on the top walls.
    #[test]
    fn the_enclosed_volume_is_its_four_parts() {
        let m = creabim_core::starter::starter_house();
        let c = cubature(&m);
        assert!(
            c.foundations > 1.0 && c.lower > 1.0 && c.upper > 100.0 && c.roofing > 10.0,
            "{c:?}"
        );
        assert!((c.total() - (c.foundations + c.lower + c.upper + c.roofing)).abs() < 1e-9);
        let t = figures_schedule(&m);
        assert_eq!(t.rows.len(), 7);
        assert!(t.totals[1].starts_with(&format!("{:.2}", c.total())));
        // a house without footings or roof: only its storeys
        let s = creabim_core::sample::sample_house();
        let c = cubature(&s);
        assert_eq!((c.foundations, c.lower, c.roofing), (0.0, 0.0, 0.0));
        assert!(c.upper > 0.0);
    }
}
