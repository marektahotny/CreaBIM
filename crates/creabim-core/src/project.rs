//! The project's own information: what it is, where, and for whom. One
//! element at most holds it; every field is optional. It fills the
//! sheets' title blocks (under any value a sheet sets itself) and names
//! the project, site and building in IFC exports (A7-5).

use serde::{Deserialize, Serialize};

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
    /// The office's logo for the title block: a picture kept with the
    /// project.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<Logo>,
    /// "Planning application", "Tender", "Construction".
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
    /// How far true north turns from the drawings' up, degrees anticlockwise.
    pub true_north: f64,
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
