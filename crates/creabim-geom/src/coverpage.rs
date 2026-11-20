//! The title pages of the documentation, as PilonDesigner draws them
//! (after the office's own): A6 labels for the binders, four to an A4
//! numbered volume 1 to 4, or one over the whole sheet. Each carries
//! the office and its contact, the stage's heading in blue, the
//! building's name on a black band, then on the project's own title
//! page the picture of the building (a 3D view placed in its frame) and
//! on a part's title page its mark and name (D.1 Architektonicko
//! stavebné riešenie), the client, the building, its place and plots,
//! who designed and drew it, the job number and the date. Everything is
//! filled from the project's information, so a title page never needs
//! to be written by hand.
//!
//! Its words are Slovak whatever the window's language: the pages
//! belong to the documentation of a Slovak construction.
//!
//! A sheet is a title page when its `fields["cover"]` names its part:
//! [`PROJECT`] for the project's own, else a key of
//! [`crate::docset::part_order`] (`_ZTI_`); `fields["volumes"]` says
//! how many labels go on it.

use crate::annotation::TextLook;
use crate::paper::{Anchor, Canvas, Stroke};
use creabim_core::project::ProjectInfo;
use creabim_core::Sheet;
use glam::DVec2;

/// The `cover` of the project's own title page.
pub const PROJECT: &str = "project";

/// A label's size, mm: A6.
const LW: f64 = 105.0;
const LH: f64 = 148.5;

/// The parts of the documentation with a title page of their own: the
/// key their sheets' numbers carry, the mark and the name.
pub const PARTS: [(&str, &str, &str); 9] = [
    ("C00_", "C", "Situačné výkresy"),
    ("_ASR_", "D.1", "Architektonicko stavebné riešenie"),
    ("_STA_", "D.2", "Statika"),
    ("_ZTI_", "D.3", "Zdravotechnická inštalácia"),
    ("_VYK_", "D.4", "Vykurovanie"),
    ("_ELI_", "D.5", "Elektroinštalácia a bleskozvod"),
    ("_VZT_", "D.6", "Vzduchotechnika"),
    ("_PLY_", "D.7", "Plynoinštalácia"),
    ("_MPL_", "D.8", "Medicinálne plyny"),
];

/// The part a sheet is the title page of: [`PROJECT`] or a key of
/// [`PARTS`]; none for any other sheet.
pub fn cover_of(sheet: &Sheet) -> Option<&str> {
    sheet
        .fields
        .get("cover")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
}

/// How many labels a title page holds: 1 to 4; four by default, one
/// for each volume, as the office's own.
pub fn volumes(sheet: &Sheet) -> usize {
    sheet
        .fields
        .get("volumes")
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(4)
        .clamp(1, 4)
}

/// The number of the first label on a title page: 5 on the second
/// page of a part's sets, 9 on the third.
pub fn first_volume(sheet: &Sheet) -> usize {
    sheet
        .fields
        .get("first_volume")
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(1)
        .max(1)
}

/// The most sets a part's title pages are made for.
pub const MAX_SETS: usize = 40;

/// The title pages of the part `sheet` is one of, in order of their
/// labels' numbers.
pub fn pages_of(
    model: &creabim_core::Model,
    sheet: creabim_core::ElementId,
) -> Vec<creabim_core::ElementId> {
    use creabim_core::ElementKind;
    let Some(part) = model.get(sheet).and_then(|e| match &e.kind {
        ElementKind::Sheet(s) => cover_of(s).map(str::to_string),
        _ => None,
    }) else {
        return vec![];
    };
    let mut v: Vec<(usize, creabim_core::ElementId)> = model
        .iter()
        .filter_map(|e| match &e.kind {
            ElementKind::Sheet(s) if cover_of(s) == Some(part.as_str()) => {
                Some((first_volume(s), e.id))
            }
            _ => None,
        })
        .collect();
    v.sort();
    v.into_iter().map(|x| x.1).collect()
}

/// How many sets a part's title pages make: the labels on all its pages.
pub fn sets(model: &creabim_core::Model, sheet: creabim_core::ElementId) -> usize {
    pages_of(model, sheet)
        .iter()
        .filter_map(|id| match model.get(*id).map(|e| &e.kind) {
            Some(creabim_core::ElementKind::Sheet(s)) => Some(volumes(s)),
            _ => None,
        })
        .sum()
}

/// The part's title pages made for `n` sets: four labels to an A4,
/// numbered on from page to page (volume 5 on the second page), pages
/// added as copies of the first (its picture too) or taken away.
pub fn set_sets(
    model: &creabim_core::Model,
    sheet: creabim_core::ElementId,
    n: usize,
) -> Option<creabim_core::Transaction> {
    use creabim_core::{Element, ElementKind, Transaction};
    let n = n.clamp(1, MAX_SETS);
    let pages = pages_of(model, sheet);
    let first = model.get(*pages.first()?)?.clone();
    let ElementKind::Sheet(base) = &first.kind else {
        return None;
    };
    let need = n.div_ceil(4);
    let base_number = base.number.clone();
    let mut tx = Transaction::new("Title page sets");
    let mut changed = false;
    for j in 0..need {
        let labels = (n - 4 * j).min(4);
        let want = |s: &mut Sheet| {
            s.fields.insert("volumes".into(), labels.to_string());
            if s.width > s.height {
                (s.width, s.height) = (s.height, s.width);
            }
            if j == 0 {
                s.fields.remove("first_volume");
            } else {
                s.fields
                    .insert("first_volume".into(), (4 * j + 1).to_string());
            }
        };
        match pages.get(j).and_then(|id| model.get(*id)) {
            Some(old) => {
                let mut now = old.clone();
                if let ElementKind::Sheet(s) = &mut now.kind {
                    want(s);
                }
                if now != *old {
                    tx = tx.replace(old.clone(), now);
                    changed = true;
                }
            }
            None => {
                let mut s = base.clone();
                want(&mut s);
                s.number = format!("{base_number}_{:02}", j + 1);
                let page = Element::new(first.name.clone(), ElementKind::Sheet(s));
                let id = page.id;
                tx = tx.insert(page);
                // the picture as on the first page
                for e in model.iter() {
                    if let ElementKind::Viewport(v) = &e.kind {
                        if v.sheet == first.id {
                            let mut v = v.clone();
                            v.sheet = id;
                            tx = tx.insert(Element::new(e.name.clone(), ElementKind::Viewport(v)));
                        }
                    }
                }
                changed = true;
            }
        }
    }
    let extra: Vec<creabim_core::ElementId> = pages.iter().skip(need).copied().collect();
    if !extra.is_empty() {
        tx = tx.remove_all_cascade(model, &extra);
        changed = true;
    }
    changed.then_some(tx)
}

/// The number a part's title page takes: `S01_ZTI_000_TIT`, before the
/// part's drawings; `000_TIT` for the project's own.
pub fn number(part: &str) -> String {
    match part {
        PROJECT => "000_TIT".into(),
        p if p.starts_with('_') => format!("S01{p}000_TIT"),
        p => format!("{p}000_TIT"),
    }
}

/// A title page for `part`: an A4 upright, no title block.
pub fn sheet(part: &str) -> Sheet {
    let mut s = Sheet::a(number(part), "A4");
    s.width = 210.0;
    s.height = 297.0;
    s.title_block = false;
    s.fields.insert("cover".into(), part.into());
    s
}

/// The heading of the title pages: the project's own, else the stage's.
pub fn heading(project: &ProjectInfo) -> String {
    if !project.cover_heading.trim().is_empty() {
        return project.cover_heading.trim().to_string();
    }
    let by_stage = match creabim_core::project::stage_code(&project.status) {
        Some("SZP") => "Projektová dokumentácia k stavebnému zámeru",
        Some("SZP/PSP") => "Projektová dokumentácia k stavebnému zámeru a projekt stavby",
        Some("PSP") => "Projekt stavby",
        Some("PSO") => "Projekt stavby na ohlásenie",
        Some("VPP") => "Vykonávací projekt",
        _ => "",
    };
    if !by_stage.is_empty() {
        return by_stage.into();
    }
    if !project.status.trim().is_empty() {
        return project.status.trim().to_string();
    }
    "Projektová dokumentácia".into()
}

/// A text in two lines at the space nearest its middle, when long.
fn split2(t: &str) -> Vec<String> {
    if t.chars().count() < 30 {
        return vec![t.to_string()];
    }
    let m = t.len() as f64 / 2.0;
    let best = t
        .char_indices()
        .filter(|(_, c)| *c == ' ')
        .map(|(i, _)| i)
        .min_by(|a, b| (*a as f64 - m).abs().total_cmp(&(*b as f64 - m).abs()));
    match best {
        Some(i) => vec![t[..i].to_string(), t[i + 1..].to_string()],
        None => vec![t.to_string()],
    }
}

/// The labels' places on the sheet: the lower left corner and the scale
/// of each.
fn cells(sheet: &Sheet) -> Vec<(DVec2, f64)> {
    let (w, h) = (sheet.width, sheet.height);
    let n = volumes(sheet);
    let fit = |cw: f64, ch: f64, lo: DVec2| {
        let k = (cw / LW).min(ch / LH);
        (lo + DVec2::new((cw - LW * k) / 2.0, (ch - LH * k) / 2.0), k)
    };
    // four to an A4 either way up, a quarter each
    if n == 1 || w.min(h) < 200.0 || w.max(h) < 280.0 {
        return vec![fit(w, h, DVec2::ZERO)];
    }
    let (cw, ch) = (w / 2.0, h / 2.0);
    (0..n)
        .map(|i| {
            // from the top left, across then down
            let lo = DVec2::new((i % 2) as f64 * cw, h - (i / 2 + 1) as f64 * ch);
            fit(cw, ch, lo)
        })
        .collect()
}

/// Where the picture of the building goes on the project's title page,
/// on the sheet: its first label's frame.
pub fn picture_box(sheet: &Sheet) -> (DVec2, DVec2) {
    let (lo, k) = cells(sheet)[0];
    let at = |x: f64, y: f64| lo + DVec2::new(x, LH - y) * k;
    (at(17.4, 83.6), at(87.6, 43.4))
}

/// Where the picture of the building is drawn again on the project's
/// title page: its viewport stands in the first label, the others show
/// it moved by these.
pub fn copies(sheet: &Sheet) -> Vec<DVec2> {
    if cover_of(sheet) != Some(PROJECT) {
        return vec![];
    }
    let cells = cells(sheet);
    cells.iter().skip(1).map(|c| c.0 - cells[0].0).collect()
}

/// The date as the title page writes it: an ISO day as `11 / 2026`.
fn month(date: &str) -> String {
    let parts: Vec<&str> = date.trim().split('-').collect();
    match parts.as_slice() {
        [y, m, _] if y.len() == 4 => match m.parse::<u32>() {
            Ok(m) => format!("{m} / {y}"),
            Err(_) => date.trim().to_string(),
        },
        _ => date.trim().to_string(),
    }
}

/// The values a title page is filled with, by what they are.
pub struct Values {
    pub office: String,
    pub contact: String,
    pub heading: String,
    pub building: String,
    pub client: String,
    pub place: String,
    pub plots: String,
    pub author: String,
    pub designer: String,
    pub drawn: String,
    pub job: String,
    pub date: String,
}

/// What `project` fills a title page with; `name` and `date` stand in
/// for the building's name and the date when the project gives none.
pub fn values(project: &ProjectInfo, name: &str, date: &str) -> Values {
    let t = |s: &str| s.trim().to_string();
    let or = |a: &str, b: &str| if a.trim().is_empty() { t(b) } else { t(a) };
    let plots = [
        t(&project.parcels),
        if project.cadastre.trim().is_empty() {
            String::new()
        } else {
            format!("k. ú. {}", project.cadastre.trim())
        },
    ]
    .into_iter()
    .filter(|s| !s.is_empty())
    .collect::<Vec<_>>()
    .join(", ");
    Values {
        office: or(&project.office, &project.designer),
        contact: t(&project.office_contact),
        heading: heading(project),
        building: or(
            &or(&project.building_name, &project.name),
            &creabim_core::words::tr(name),
        ),
        client: t(&project.client),
        place: or(&project.address, &project.site_name),
        plots,
        author: or(&project.architect, &project.designer),
        designer: t(&project.designer),
        drawn: or(&project.drawn_by, &project.designer),
        job: t(&project.number),
        date: or(&project.issue_date, &month(date)),
    }
}

/// Draw `sheet` as a title page, when it is one; false for any other
/// sheet, drawn as usual.
pub fn draw(
    c: &mut dyn Canvas,
    sheet: &Sheet,
    project: &ProjectInfo,
    name: &str,
    date: &str,
) -> bool {
    let Some(part) = cover_of(sheet) else {
        return false;
    };
    let v = values(project, name, date);
    let own = PARTS.iter().find(|p| p.0 == part);
    const BLUE: [f32; 3] = [0.043, 0.239, 0.569];
    let rows: [Option<(&str, &str, bool)>; 10] = [
        Some(("Investor a vlastník", &v.client, false)),
        Some(("Stavba", &v.building, true)),
        Some(("Miesto stavby", &v.place, true)),
        Some(("Číslo parcely", &v.plots, true)),
        None,
        Some(("Autor", &v.author, false)),
        Some(("Zodpovedný", &v.designer, false)),
        Some(("Vypracoval", &v.drawn, false)),
        None,
        Some(("Zákazkové číslo", &v.job, false)),
    ];
    for (i, (lo, k)) in cells(sheet).into_iter().enumerate() {
        // the label's own millimetres, y down from its top as on paper
        let at = |x: f64, y: f64| lo + DVec2::new(x, LH - y) * k;
        let text = |c: &mut dyn Canvas,
                    x: f64,
                    base: f64,
                    t: &str,
                    h: f64,
                    anchor: Anchor,
                    room: f64,
                    bold: bool,
                    rgb: Option<[f32; 3]>| {
            if t.trim().is_empty() {
                return;
            }
            // the letters' height by the font's size; a text too long
            // for its room is written smaller
            let mut size = h * 0.72;
            let wide = crate::paper::rough_width(t, size);
            if room > 0.0 && wide > room {
                size *= room / wide;
            }
            let look = TextLook {
                size: size * k,
                rgb,
                bold,
                ..TextLook::default()
            };
            c.styled_text(at(x, base - size / 2.0), t, anchor, 0.0, &look);
        };
        let (start, mid, end) = (Anchor::Start, Anchor::Middle, Anchor::End);
        c.rect(
            at(5.0, 143.5),
            at(100.0, 5.0),
            Stroke::solid(1.4 * k / 2.0, 0.0),
        );
        text(c, 52.5, 13.5, &v.office, 4.6, mid, 86.0, false, None);
        c.line(&[at(8.0, 15.6), at(97.0, 15.6)], Stroke::solid(0.15, 0.0));
        text(c, 52.5, 21.0, &v.contact, 3.4, mid, 88.0, false, None);
        let head = split2(&v.heading);
        let mut y = if head.len() > 1 { 28.0 } else { 31.0 };
        for l in &head {
            text(c, 52.5, y, l, 4.3, mid, 88.0, false, Some(BLUE));
            y += 6.0;
        }
        let band = [at(9.0, 43.0), at(96.0, 43.0), at(96.0, 36.5), at(9.0, 36.5)];
        c.fill(&band, 0.0);
        let building = if v.building.is_empty() {
            "Stavba".to_string()
        } else {
            v.building.clone()
        };
        text(
            c,
            52.5,
            41.6,
            &building,
            3.8,
            mid,
            82.0,
            false,
            Some([1.0, 1.0, 1.0]),
        );
        match own {
            Some((_, mark, title)) => {
                // a part's page: its mark and name, the documentation of
                // the building objects over the parts D
                text(c, 52.5, 58.0, mark, 12.0, mid, 0.0, true, None);
                for (j, l) in split2(title).iter().enumerate() {
                    text(
                        c,
                        52.5,
                        68.0 + j as f64 * 6.5,
                        l,
                        5.0,
                        mid,
                        84.0,
                        true,
                        None,
                    );
                }
                if mark.starts_with("D.") {
                    let d = format!("D {}", "Dokumentácia stavebných objektov");
                    text(c, 52.5, 81.5, &d, 2.8, mid, 84.0, false, None);
                }
            }
            None => {
                // the project's own: the frame the picture is placed in
                c.rect(at(17.0, 84.0), at(88.0, 43.0), Stroke::solid(0.4 * k, 0.0));
            }
        }
        let mut y = 89.5;
        for r in rows {
            let Some((label, value, bold)) = r else {
                y += 1.6;
                continue;
            };
            text(c, 11.0, y, label, 2.9, start, 23.0, bold, None);
            text(c, 35.0, y, ":", 2.9, start, 0.0, false, None);
            text(c, 38.5, y, value, 2.9, start, 56.0, bold, None);
            y += 4.3;
        }
        text(c, 83.0, 129.0, "Zväzok č. :", 2.9, end, 0.0, false, None);
        text(
            c,
            92.0,
            131.0,
            &(first_volume(sheet) + i).to_string(),
            7.5,
            start,
            0.0,
            false,
            Some([0.114, 0.306, 0.847]),
        );
        text(c, 52.5, 140.5, &v.date, 3.4, mid, 0.0, false, None);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paper::SvgCanvas;

    fn project() -> ProjectInfo {
        ProjectInfo {
            client: "Jozef Matúš".into(),
            building_name: "Rodinný dom".into(),
            address: "Bratislava".into(),
            parcels: "1838/116".into(),
            cadastre: "Rača".into(),
            designer: "Ing. arch. Marek Tahotný".into(),
            number: "27/2025".into(),
            status: "Projekt stavby".into(),
            office: "Ateliér T".into(),
            ..Default::default()
        }
    }

    #[test]
    fn a_title_page_is_filled_from_the_project() {
        let s = sheet("_ZTI_");
        assert_eq!(s.number, "S01_ZTI_000_TIT");
        assert_eq!(
            crate::docset::part_of(&s.number),
            Some("D.3 Plumbing (ZTI)")
        );
        assert_eq!(volumes(&s), 4);
        let mut c = SvgCanvas::new(DVec2::new(s.width, s.height));
        assert!(draw(&mut c, &s, &project(), "file", "2026-11-19"));
        let svg = c.finish();
        for want in [
            "Jozef Matúš",
            "Rodinný dom",
            "Bratislava",
            "1838/116, k. ú. Rača",
            "Ing. arch. Marek Tahotný",
            "27/2025",
            "Projekt stavby",
            "Ateliér T",
            "D.3",
            "11 / 2026",
        ] {
            assert!(svg.contains(want), "{want}");
        }
        // four labels, numbered 1 to 4
        assert_eq!(svg.matches("27/2025").count(), 4);
        assert!(svg.contains(">4<"));
        // the heading may be the office's own
        let mut p = project();
        p.cover_heading = "Dokumentácia pre stavebné povolenie".into();
        let mut c = SvgCanvas::new(DVec2::new(s.width, s.height));
        draw(&mut c, &s, &p, "", "");
        assert!(c.finish().contains("povolenie"));
    }

    #[test]
    fn the_projects_own_page_holds_one_label_with_a_picture_frame() {
        let mut s = sheet(PROJECT);
        assert_eq!(volumes(&s), 4);
        assert_eq!(copies(&s).len(), 3);
        s.fields.insert("volumes".into(), "1".into());
        assert!(copies(&s).is_empty());
        let (lo, hi) = picture_box(&s);
        assert!(lo.x > 0.0 && hi.x < 210.0 && lo.y > 100.0 && hi.y < 297.0);
        assert!((hi.x - lo.x) > 130.0);
        let mut c = SvgCanvas::new(DVec2::new(s.width, s.height));
        assert!(draw(&mut c, &s, &project(), "", ""));
        // any other sheet is no title page
        let mut c = SvgCanvas::new(DVec2::new(420.0, 297.0));
        assert!(!draw(
            &mut c,
            &Sheet::a("S01_ASR_001_N01", "A3"),
            &project(),
            "",
            ""
        ));
        assert_eq!(
            split2("Projektová dokumentácia k stavebnému zámeru").len(),
            2
        );
        assert_eq!(heading(&ProjectInfo::default()), "Projektová dokumentácia");
    }
}

#[cfg(test)]
mod look {
    use super::*;
    #[test]
    #[ignore]
    fn write_pages() {
        let out = std::env::var("COVER_OUT").unwrap();
        let mut m = creabim_core::sample::sample_house();
        let p = ProjectInfo {
            client: "Jozef Matúš".into(),
            building_name: "Rodinný dom Matúš".into(),
            address: "Bratislava – Rača".into(),
            parcels: "1838/116".into(),
            cadastre: "Rača".into(),
            designer: "Ing. arch. Marek Tahotný".into(),
            status: "Projekt stavby".into(),
            number: "27/2025".into(),
            issue_date: "11 / 2026".into(),
            office: "Ateliér T".into(),
            office_contact: "Hlavná 1, 811 01 Bratislava, tel. 0900 000 000".into(),
            ..Default::default()
        };
        let tx = match m.project_info().map(|x| x.0) {
            Some(id) => {
                let old = m.get(id).unwrap().clone();
                let mut e = old.clone();
                e.kind = creabim_core::ElementKind::Project(p);
                creabim_core::Transaction::new("i").replace(old, e)
            }
            None => creabim_core::Transaction::new("i").insert(creabim_core::Element::new(
                "Project",
                creabim_core::ElementKind::Project(p),
            )),
        };
        tx.apply(&mut m).unwrap();
        let (tx, _) = crate::docset::drawing_set(&m).unwrap();
        tx.apply(&mut m).unwrap();
        let info = crate::sheet::SheetInfo {
            project: "x".into(),
            date: "2026-11-19".into(),
        };
        for e in m.iter() {
            if let creabim_core::ElementKind::Sheet(s) = &e.kind {
                if super::cover_of(s).is_some() || s.number.ends_with("N01") {
                    let svg = crate::sheet::sheet_svg(&m, e.id, &info).unwrap();
                    std::fs::write(format!("{out}/{}.svg", s.number), svg).unwrap();
                }
            }
        }
    }
}

#[cfg(test)]
mod sets_tests {
    use super::*;
    use creabim_core::{Element, ElementKind, Model, Transaction, View, Viewport};

    fn pages(m: &Model, part: &str) -> Vec<(String, usize, usize)> {
        let mut v: Vec<_> = m
            .iter()
            .filter_map(|e| match &e.kind {
                ElementKind::Sheet(s) if cover_of(s) == Some(part) => {
                    Some((s.number.clone(), first_volume(s), volumes(s)))
                }
                _ => None,
            })
            .collect();
        v.sort();
        v
    }

    /// More than four sets go on further pages numbered on; fewer take
    /// them away again; a page turned on its side stands up again.
    #[test]
    fn sets_beyond_four_go_on_further_pages_numbered_on() {
        let mut m = Model::new();
        let mut s = sheet(PROJECT);
        (s.width, s.height) = (297.0, 210.0);
        let page = Element::new("Titulná strana", ElementKind::Sheet(s));
        let id = page.id;
        let view = Element::new("3D", ElementKind::View(View::axonometric(35.0, 30.0)));
        let vp = Viewport {
            sheet: id,
            view: view.id,
            center: DVec2::new(100.0, 200.0),
            target: DVec2::ZERO,
            crop: None,
            ink: Default::default(),
        };
        Transaction::new("t")
            .insert(page)
            .insert(view)
            .insert(Element::new("Viewport", ElementKind::Viewport(vp)))
            .apply(&mut m)
            .unwrap();
        assert_eq!(sets(&m, id), 4);
        set_sets(&m, id, 7).unwrap().apply(&mut m).unwrap();
        assert_eq!(
            pages(&m, PROJECT),
            [
                ("000_TIT".to_string(), 1, 4),
                ("000_TIT_02".to_string(), 5, 3)
            ]
        );
        assert_eq!(sets(&m, id), 7);
        // the second page has the picture too, and says volume 5 to 7
        let viewports = m
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Viewport(_)))
            .count();
        assert_eq!(viewports, 2);
        let second = m
            .iter()
            .find_map(|e| match &e.kind {
                ElementKind::Sheet(s) if s.number == "000_TIT_02" => Some(s.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!((second.width, second.height), (210.0, 297.0));
        let mut c = crate::paper::SvgCanvas::new(DVec2::new(210.0, 297.0));
        draw(&mut c, &second, &ProjectInfo::default(), "", "");
        let svg = c.finish();
        assert!(svg.contains(">5<") && svg.contains(">7<") && !svg.contains(">8<"));
        // back to two: one page again
        set_sets(&m, id, 2).unwrap().apply(&mut m).unwrap();
        assert_eq!(pages(&m, PROJECT), [("000_TIT".to_string(), 1, 2)]);
        let viewports = m
            .iter()
            .filter(|e| matches!(e.kind, ElementKind::Viewport(_)))
            .count();
        assert_eq!(viewports, 1);
        // and four again
        set_sets(&m, id, 4).unwrap().apply(&mut m).unwrap();
        assert_eq!(pages(&m, PROJECT), [("000_TIT".to_string(), 1, 4)]);
        assert!(set_sets(&m, id, 4).is_none());
    }
}
