//! The project's own information: what it is, where, and for whom. One
//! element at most holds it; every field is optional. It fills the
//! sheets' title blocks (under any value a sheet sets itself) and names
//! the project, site and building in IFC exports (A7-5).

use serde::{Deserialize, Serialize};

/// The stages of documentation the Slovak Building Act (25/2025
/// Z. z.) knows, with their codes as file names carry them.
pub const STAGES: [(&str, &str); 5] = [
    ("Stavebný zámer", "SZP"),
    ("Stavebný zámer/Projekt stavby", "SZP/PSP"),
    ("Projekt stavby", "PSP"),
    ("Projekt stavby na ohlásenie", "PSO"),
    ("Vykonávací projekt", "VPP"),
];

/// The code of a stage by its name; none for any other text.
pub fn stage_code(stage: &str) -> Option<&'static str> {
    STAGES
        .iter()
        .find(|(name, _)| *name == stage.trim())
        .map(|s| s.1)
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProjectInfo {
    /// The project's name ("Maple Lane House").
    pub name: String,
    /// The office's job number.
    pub number: String,
    pub client: String,
    /// Who signs the drawings: the chief architect, who drew them and
    /// the designer responsible (the title block's three names).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub architect: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub drawn_by: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub designer: String,
    /// The plots built on ("1838/116") and their cadastral district.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub parcels: String,
    /// The area of the building plot, m², as the cadastre gives it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plot_area: Option<f64>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cadastre: String,
    /// The fire separation distances of the building's sides, m: eight,
    /// north first and on clockwise; nothing where none is given. They
    /// are the designer's, read from the norm's tables.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fire_distances: Vec<f64>,
    /// The norms' tables the fire calculations run on, read from the
    /// designer's parameter file and kept with the project.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fire_params: Vec<crate::fireparams::ParamTable>,
    /// The building's structural system for fire (konštrukčný celok,
    /// STN 92 0201-2): "N" non-combustible, "Za"/"Zb" mixed, "Ha"/"Hb"
    /// combustible; empty is non-combustible.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub fire_system: String,
    /// The fire barriers of mineral wool in an outer insulation that
    /// burns (ETICS of EPS): made automatically unless turned off.
    #[serde(default, skip_serializing_if = "EticsBarriers::is_default")]
    pub etics_barriers: EticsBarriers,
    /// The locality the heat loss and gains take their outside
    /// temperatures from (a row of the climate table among
    /// [`ProjectInfo::fire_params`]); empty is none.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub climate_site: String,
    /// The office's logo for the title block: a picture kept with the
    /// project.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<Logo>,
    /// The office, and its address and contact, at the head of the
    /// title pages.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub office: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub office_contact: String,
    /// The title pages' heading in place of the stage's own
    /// ("Projektová dokumentácia k stavebnému zámeru").
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cover_heading: String,
    /// The documentation's stage, one of [`STAGES`] by its name
    /// ("Projekt stavby").
    pub status: String,
    /// The issue date, as the office writes it.
    pub issue_date: String,
    /// The documentation's revision ("00", "01"...): in the list of
    /// documentation and at the end of exported file names.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub revision: String,
    /// The site's postal address.
    pub address: String,
    pub site_name: String,
    /// Degrees, north positive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    /// Degrees, east positive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    /// The project's zero above sea level, metres.
    pub elevation: f64,
    /// Where the project's origin is in S-JTSK (EPSG:5514): east and
    /// north, metres, both negative in Slovakia (Pilon Map Downloader's
    /// bod_pociatku).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sjtsk: Option<[f64; 2]>,
    /// The level the site is modelled on (the map's terrain, its plan "3D
    /// terén"): not a storey, see [`crate::Model::site_level`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub site_level: Option<crate::ElementId>,
    /// How far true north turns from the drawings' up, degrees anticlockwise.
    pub true_north: f64,
    /// What came down from the map site (Pilon Map Downloader): its
    /// drawings, orthophoto and terrain. They move and turn as one, and
    /// the S-JTSK coordinates with them (see [`crate::mapgroup`]).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub map_group: Vec<crate::ElementId>,
    /// The map drawing in S-JTSK the coordinates are read from: a model
    /// point's S-JTSK east and north are its point in that drawing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub map_ref: Option<crate::ElementId>,
    /// The S-JTSK origin and true north as the map came down, kept when the
    /// ground first moves under the building, so Reset the coordinates can
    /// put it back.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_origin: Option<([f64; 2], f64)>,
    pub building_name: String,
    /// "Dwelling", "Office", "School".
    pub building_use: String,
    /// The project's notepad (MU26): notes and to-dos, markdown, kept with
    /// the project and never printed. Read into [`ProjectInfo::pad`] once.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub notes: String,
    /// The notepad's items (MU27): to-dos and notes, each with its text
    /// (the first line its title) and, if wanted, a link to an element or
    /// a view.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub pad: Vec<PadItem>,
    /// The building codes the project follows: STN, or ČSN.
    #[serde(skip_serializing_if = "crate::norms::NormSet::is_default")]
    pub norms: crate::norms::NormSet,
    /// The project's own profiles (MU27 block 5), for sweeps.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<crate::profiles::NamedProfile>,
    /// How long masonry walls may run between braces, and the columns put
    /// in them (see [`crate::masonry`]).
    #[serde(skip_serializing_if = "crate::masonry::MasonryRules::boxed_default")]
    pub masonry: Box<crate::masonry::MasonryRules>,
}

/// A picture: `width` × `height` pixels, RGBA, stored as text (see
/// `creabim_geom::raster::store_image`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Logo {
    pub width: u32,
    pub height: u32,
    pub rgba: String,
}

/// One item of the project's notepad.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PadItem {
    pub text: String,
    /// A to-do (ticked or not); else a note.
    pub todo: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub done: bool,
    /// What it is about: an element or a view, selected or opened from it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<crate::ElementId>,
}

impl PadItem {
    /// Its first line.
    pub fn title(&self) -> &str {
        self.text.lines().next().unwrap_or("").trim()
    }
}

/// The old markdown notepad as items: `- [ ]` and `- [x]` lines are
/// to-dos, other lines join into notes, a blank line or heading ending one.
pub fn pad_from_markdown(md: &str) -> Vec<PadItem> {
    let mut out: Vec<PadItem> = vec![];
    let mut note: Option<PadItem> = None;
    let flush = |note: &mut Option<PadItem>, out: &mut Vec<PadItem>| {
        if let Some(n) = note.take().filter(|n| !n.text.trim().is_empty()) {
            out.push(n);
        }
    };
    for line in md.lines() {
        let t = line.trim();
        let todo = t.strip_prefix("- [ ] ").map(|r| (false, r)).or_else(|| {
            t.strip_prefix("- [x] ")
                .or(t.strip_prefix("- [X] "))
                .map(|r| (true, r))
        });
        if let Some((done, r)) = todo {
            flush(&mut note, &mut out);
            out.push(PadItem {
                text: r.into(),
                todo: true,
                done,
                link: None,
            });
        } else if t.is_empty() {
            flush(&mut note, &mut out);
        } else {
            let t = t.trim_start_matches('#').trim();
            match &mut note {
                Some(n) => {
                    n.text.push('\n');
                    n.text.push_str(t);
                }
                None => {
                    note = Some(PadItem {
                        text: t.into(),
                        ..Default::default()
                    })
                }
            }
        }
    }
    flush(&mut note, &mut out);
    out
}

/// Items as markdown: to-dos as `- [ ]`/`- [x]` lines, notes as
/// paragraphs, each link named after what it points at.
pub fn pad_to_markdown(
    items: &[&PadItem],
    name_of: &dyn Fn(crate::ElementId) -> Option<String>,
) -> String {
    let mut todos = String::new();
    let mut notes = String::new();
    for i in items {
        let link = i
            .link
            .and_then(name_of)
            .map(|n| format!(" (\u{2192} {n})"))
            .unwrap_or_default();
        if i.todo {
            let mut lines = i.text.lines();
            let first = lines.next().unwrap_or("");
            todos.push_str(&format!(
                "- [{}] {first}{link}\n",
                if i.done { "x" } else { " " }
            ));
            for l in lines {
                todos.push_str(&format!("  {l}\n"));
            }
        } else {
            notes.push_str(&format!("{}{link}\n\n", i.text.trim_end()));
        }
    }
    let mut out = String::new();
    if !todos.is_empty() {
        out.push_str("## To-dos\n\n");
        out.push_str(&todos);
        out.push('\n');
    }
    if !notes.is_empty() {
        out.push_str("## Notes\n\n");
        out.push_str(&notes);
    }
    out.trim_end().to_string() + "\n"
}

impl ProjectInfo {
    /// The revision as it is written: "00" until one is given.
    pub fn revision_or_first(&self) -> String {
        match self.revision.trim() {
            "" => "00".to_string(),
            r => r.to_string(),
        }
    }

    /// The title-block fields this fills, by the names title blocks use:
    /// only those with a value.
    pub fn fields(&self) -> Vec<(&'static str, String)> {
        [
            ("job", &self.name),
            ("project", &self.name),
            ("project_number", &self.number),
            ("client", &self.client),
            ("status", &self.status),
            ("issue_date", &self.issue_date),
            ("address", &self.address),
            ("site", &self.site_name),
            ("building", &self.building_name),
        ]
        .into_iter()
        .filter(|(_, v)| !v.trim().is_empty())
        .map(|(k, v)| (k, v.trim().to_string()))
        .collect()
    }

    /// Nothing filled in.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// The change that makes `info` the project's information: one undo
/// step, nothing when it is already so.
pub fn set_project(model: &crate::Model, info: ProjectInfo) -> crate::Transaction {
    let tx = crate::Transaction::new("Project information");
    match model.project_info() {
        Some((_, p)) if *p == info => tx,
        Some((id, _)) => {
            let Some(e) = model.get(id) else { return tx };
            let mut after = e.clone();
            after.kind = crate::ElementKind::Project(info);
            tx.replace(e.clone(), after)
        }
        None => tx.insert(crate::Element::new(
            "Project information",
            crate::ElementKind::Project(info),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_filled_fields_reach_the_title_blocks() {
        let p = ProjectInfo {
            name: "Maple Lane House".into(),
            client: "  ".into(),
            ..Default::default()
        };
        let f = p.fields();
        assert!(f.contains(&("job", "Maple Lane House".to_string())));
        assert!(f.contains(&("project", "Maple Lane House".to_string())));
        assert!(!f.iter().any(|(k, _)| *k == "client"));
        assert!(ProjectInfo::default().is_empty());
    }

    #[test]
    fn the_old_notepad_reads_as_items_and_writes_back_as_markdown() {
        let md = "- [x] Start a new project\n- [ ] testing a new note\n\nA note\nits second line";
        let items = pad_from_markdown(md);
        assert_eq!(items.len(), 3);
        assert!(items[0].todo && items[0].done);
        assert_eq!(items[2].title(), "A note");
        let refs: Vec<&PadItem> = items.iter().collect();
        let back = pad_to_markdown(&refs, &|_| None);
        assert!(back.contains("- [x] Start a new project"));
        assert!(back.contains("## Notes"));
        assert_eq!(
            pad_from_markdown(&back).iter().filter(|i| i.todo).count(),
            2
        );
    }

    #[test]
    fn setting_it_twice_keeps_one_element() {
        let mut doc = crate::document::Document::new();
        let a = ProjectInfo {
            name: "A".into(),
            ..Default::default()
        };
        doc.commit(set_project(doc.model(), a.clone())).unwrap();
        let b = ProjectInfo {
            name: "B".into(),
            ..a.clone()
        };
        doc.commit(set_project(doc.model(), b)).unwrap();
        let n = doc
            .model()
            .iter()
            .filter(|e| matches!(e.kind, crate::ElementKind::Project(_)))
            .count();
        assert_eq!(n, 1);
        assert_eq!(doc.model().project_info().unwrap().1.name, "B");
        assert!(
            set_project(doc.model(), doc.model().project_info().unwrap().1.clone())
                .ops
                .is_empty()
        );
    }
}

/// Every site plan turned `degrees` further anticlockwise on screen and
/// paper (project north against true north, as Revit has it): the model
/// stays as it was drawn, so the floor plans keep their way up (George:
/// turning north turned his building in the 1st floor plan).
pub fn turn_site_plans(model: &crate::Model, degrees: f64) -> crate::Transaction {
    let mut tx = crate::Transaction::new("Rotate project north");
    for e in model.iter() {
        let crate::ElementKind::View(v) = &e.kind else {
            continue;
        };
        if !v.graphics.range.is_site() || !matches!(v.kind, crate::ViewKind::Plan { .. }) {
            continue;
        }
        let t = (v.turn + degrees + 180.0).rem_euclid(360.0) - 180.0;
        let t = (t * 1e9).round() / 1e9;
        let mut now = e.clone();
        if let crate::ElementKind::View(w) = &mut now.kind {
            w.turn = if t.abs() < 1e-9 { 0.0 } else { t };
        }
        if now != *e {
            tx = tx.replace(e.clone(), now);
        }
    }
    tx
}

/// The project's north turned by `degrees` anticlockwise (PilonDesigner's
/// "Otočiť projektový sever"): the whole model, the building and what is
/// round it (terrain, parcels, map underlays), turns about the origin, so
/// that the plans draw turned; true north stays where it is on the ground
/// ([`ProjectInfo::true_north`] turns with the model), the origin stays
/// on its S-JTSK point and every height stays as it is.
pub fn rotate_north(model: &crate::Model, degrees: f64) -> crate::Transaction {
    use crate::{ElementKind, ViewKind};
    let turn = crate::transform::Xform::Rotate {
        center: glam::DVec2::ZERO,
        degrees,
    };
    let rot = glam::DVec2::from_angle(degrees.to_radians());
    let in_plan = |view: crate::ElementId| {
        matches!(
            model.get(view).map(|e| &e.kind),
            Some(ElementKind::View(v)) if matches!(v.kind, ViewKind::Plan { .. })
        )
    };
    let mut tx = crate::Transaction::new("Rotate project north");
    let mut has_project = false;
    for e in model.iter() {
        let mut now = e.clone();
        match &mut now.kind {
            // these go with what they are in, or lie on paper
            ElementKind::Opening(_)
            | ElementKind::Valve(_)
            | ElementKind::Circuit(_)
            | ElementKind::Level(_)
            | ElementKind::PaperMark(_) => continue,
            ElementKind::Project(p) => {
                has_project = true;
                let n = (p.true_north + degrees + 180.0).rem_euclid(360.0) - 180.0;
                p.true_north = (n * 1e6).round() / 1e6;
            }
            // what is written in a plan turns with the plan
            ElementKind::Tag(t) if in_plan(t.view) => {
                t.offset = rot.rotate(t.offset);
                t.elbow = t.elbow.map(|p| rot.rotate(p));
                t.end = t.end.map(|p| rot.rotate(p));
            }
            ElementKind::Note(n) if in_plan(n.view) => {
                n.at = rot.rotate(n.at);
                n.elbow = n.elbow.map(|p| rot.rotate(p));
                n.leader = n.leader.map(|mut p| {
                    if p.host.is_none() {
                        p.local = rot.rotate(p.local);
                    }
                    p
                });
            }
            ElementKind::Dimension(d) if in_plan(d.view) => {
                // across and up the drawing swap on a quarter turn; at
                // any other angle the dimension follows what it measures
                use crate::annotation::DimAxis;
                let quarters = (degrees / 90.0).round();
                let square = (degrees - quarters * 90.0).abs() < 1e-6;
                d.axis = match (d.axis, square, quarters.rem_euclid(2.0) == 1.0) {
                    (DimAxis::Aligned, ..) => DimAxis::Aligned,
                    (a, true, false) => a,
                    (DimAxis::Horizontal, true, true) => DimAxis::Vertical,
                    (DimAxis::Vertical, true, true) => DimAxis::Horizontal,
                    (_, false, _) => DimAxis::Aligned,
                };
                for p in d.points_mut() {
                    if p.host.is_none() {
                        p.local = rot.rotate(p.local);
                    }
                }
                if let Some(c) = &mut d.center {
                    if c.host.is_none() {
                        c.local = rot.rotate(c.local);
                    }
                }
            }
            _ => match crate::transform::transformed(model, e, &turn) {
                Some(n) => now = n,
                None => continue,
            },
        }
        if now != *e {
            tx = tx.replace(e.clone(), now);
        }
    }
    if !has_project {
        tx = tx.insert(crate::Element::new(
            "Project",
            ElementKind::Project(ProjectInfo {
                true_north: degrees,
                ..Default::default()
            }),
        ));
    }
    tx
}

#[cfg(test)]
mod north_tests {
    use crate::{Document, ElementKind};

    /// A map's parcel boundary (a line of a DXF reference) picked: the
    /// map turns with the model, so the boundary ends up level.
    #[test]
    fn a_reference_line_turned_level_turns_the_map_with_it() {
        use crate::{Element, RefSource, Reference, View};
        use glam::DVec2;
        let mut doc = Document::from_model(crate::sample::sample_house());
        let level = doc.model().levels().next().unwrap().0.id;
        let view = Element::new("Situácia", ElementKind::View(View::plan(level)));
        let vid = view.id;
        let mut r = Reference::new(
            RefSource::Dxf {
                path: "situacia.dxf".into(),
                unit: Some(1.0),
            },
            vec![vid],
        );
        r.in_place([DVec2::new(-10.0, -10.0), DVec2::new(30.0, 30.0)]);
        r.at += DVec2::new(5.0, 3.0);
        let e = Element::new("Map", ElementKind::Reference(r));
        let rid = e.id;
        doc.commit(crate::Transaction::new("Map").insert(view).insert(e))
            .unwrap();
        let line = |doc: &Document| {
            let ElementKind::Reference(r) = &doc.model().get(rid).unwrap().kind else {
                panic!()
            };
            (
                r.place(DVec2::new(0.0, 0.0)),
                r.place(DVec2::new(10.0, 3.0)),
            )
        };
        let (a, b) = line(&doc);
        let th = (b - a).to_angle().to_degrees();
        let by = (th / 90.0).round() * 90.0 - th;
        doc.commit(super::rotate_north(doc.model(), by)).unwrap();
        let (a, b) = line(&doc);
        assert!((a.y - b.y).abs() < 1e-9, "{a} {b}");
    }

    #[test]
    fn the_project_turns_and_true_north_stays_on_the_ground() {
        let mut doc = Document::from_model(crate::sample::sample_house());
        let walls = |doc: &Document| -> Vec<(glam::DVec2, glam::DVec2, f64)> {
            doc.model()
                .iter()
                .filter_map(|e| match &e.kind {
                    ElementKind::Wall(w) => Some((w.start, w.end, w.height)),
                    _ => None,
                })
                .collect()
        };
        let before = walls(&doc);
        let north = doc
            .model()
            .project_info()
            .map_or(0.0, |(_, p)| p.true_north);
        doc.commit(super::rotate_north(doc.model(), 30.0)).unwrap();
        let rot = glam::DVec2::from_angle(30f64.to_radians());
        for (b, a) in before.iter().zip(walls(&doc)) {
            assert!((rot.rotate(b.0) - a.0).length() < 1e-9);
            assert!((rot.rotate(b.1) - a.1).length() < 1e-9);
            assert_eq!(b.2, a.2, "heights stay");
        }
        let now = doc.model().project_info().unwrap().1.true_north;
        assert!((now - north - 30.0).abs() < 1e-9);
        // and back
        doc.commit(super::rotate_north(doc.model(), -30.0)).unwrap();
        for (b, a) in before.iter().zip(walls(&doc)) {
            assert!((b.0 - a.0).length() < 1e-9 && (b.1 - a.1).length() < 1e-9);
        }
    }
}

#[cfg(test)]
mod stage_tests {
    use super::*;

    #[test]
    fn a_stage_has_its_code_and_other_text_none() {
        assert_eq!(stage_code("Projekt stavby"), Some("PSP"));
        assert_eq!(stage_code(" Vykonávací projekt "), Some("VPP"));
        assert_eq!(stage_code("Planning application"), None);
    }
}

/// How the fire barriers of an ETICS of combustible insulation are made
/// (Weber ETICS manual s. 16-17, after STN 73 0802): bands of mineral
/// wool in the insulation layer of outer walls, at the plinth and about
/// the floors, as the building's fire height asks.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EticsBarriers {
    /// No barriers made (the designer draws them some other way).
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub off: bool,
    /// The top of the plinth barrier over the ground, m (most 0.6).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plinth_top: Option<f64>,
    /// A barrier's height, m (0.2; 0.15 to 0.4).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f64>,
    /// Their material; none takes the base's façade mineral wool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub material: Option<crate::ElementId>,
    /// A barrier at every floor, not only the first (up to 7 m) the
    /// table asks of buildings up to 12 m.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub every_floor: bool,
    /// Bands over the openings of each storey, 500 mm past their sides,
    /// instead of round the building about the floors.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub over_openings: bool,
    /// No parapet band at the top of the walls under a roof that burns.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub no_parapet: bool,
}

impl EticsBarriers {
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }
    pub fn plinth_top(&self) -> f64 {
        self.plinth_top.unwrap_or(0.6)
    }
    pub fn width(&self) -> f64 {
        self.width.unwrap_or(0.2)
    }
}
