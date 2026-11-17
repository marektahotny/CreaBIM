//! The tools row's tabs (brief 02, 2.1): Architecture (upstream's Model),
//! the building services' disciplines, Annotate, View and Manage each show
//! their own tools and commands, icons only, while Select and Align stay at
//! the left on every tab. A tool picked by its key shows its
//! tab. A tool can be on two tabs where it belongs to both: the callout.

use crate::planview::Tool;

/// One button of a tab.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Entry {
    Tool(Tool),
    /// A command of the menus, by its action id.
    Act {
        id: &'static str,
        label: &'static str,
        icon: &'static str,
        hint: &'static str,
    },
    /// A short menu of commands (see [`menu`]).
    Menu {
        key: &'static str,
        label: &'static str,
        icon: &'static str,
        hint: &'static str,
    },
    /// A divider between groups.
    Sep,
}

/// The first is called Model upstream; this fork names it after its
/// discipline, and puts the building services' disciplines beside it.
pub const TABS: [&str; 10] = [
    "Architecture",
    "Cadastre",
    "Plumbing and gas",
    "Heating",
    "HVAC",
    "Medical gases",
    "Electrical installation",
    "Annotate",
    "View",
    "Manage",
];

/// The tab where annotating starts; the ones before it are disciplines.
pub const ANNOTATE: usize = 7;

/// On every tab, before it.
pub const ALWAYS: [Tool; 2] = [Tool::Select, Tool::Align];

/// What tab `tab` holds.
pub fn entries(tab: usize) -> Vec<Entry> {
    use Entry::{Sep, Tool as T};
    let act = |id, label, icon, hint| Entry::Act {
        id,
        label,
        icon,
        hint,
    };
    match tab {
        0 => vec![
            T(Tool::Wall),
            act(
                "tool.curtain",
                "Curtain wall",
                "curtain-wall",
                "The Wall tool drawing a curtain wall type: mullions and glass on a grid",
            ),
            T(Tool::Door),
            T(Tool::Window),
            T(Tool::Opening),
            Sep,
            T(Tool::Slab),
            T(Tool::Shaft),
            T(Tool::Roof),
            T(Tool::Stair),
            T(Tool::Railing),
            Sep,
            T(Tool::Column),
            T(Tool::Beam),
            T(Tool::Footing),
            Sep,
            T(Tool::Object),
            act(
                "catalogue",
                "Catalogue",
                "sofa",
                "Furniture and fixtures to place, and your own from Blender, FreeCAD or glTF",
            ),
            Sep,
            T(Tool::Mass),
            T(Tool::Extrusion),
            T(Tool::Sweep),
            T(Tool::Void),
            T(Tool::Sketch),
            Sep,
            T(Tool::Room),
            T(Tool::RoomSeparator),
            T(Tool::Ceiling),
            Sep,
            T(Tool::Terrain),
            T(Tool::Paving),
            T(Tool::Planting),
        ],
        1 => vec![
            act(
                "discipline:cadastre",
                "Cadastre and site",
                "landmark",
                "Cadastral map, orthophoto and terrain in S-JTSK: the standards and the tools coming",
            ),
            act(
                "project.info",
                "Project information",
                "info",
                "Site, address, latitude and longitude, and the project's codes (STN or ČSN)",
            ),
            act(
                "view.reference",
                "Map underlay",
                "underlay",
                "Trace over a map or a picture: shown faintly under this view (Alt+F2 hides it)",
            ),
            Sep,
            T(Tool::Terrain),
            T(Tool::Paving),
            T(Tool::Planting),
        ],
        2 => vec![
            act(
                "discipline:water",
                "Water supply",
                "droplet",
                "Cold and hot water: the standards (STN EN 806-3, STN 73 6655) and the tools coming",
            ),
            T(Tool::Pipe),
            act(
                "catalogue.cat:bathroom",
                "Sanitary fixtures",
                "fixtures",
                "Basins, WCs, baths and showers from the catalogue",
            ),
            act(
                "catalogue.cat:plumbing",
                "Plumbing fixtures",
                "waves",
                "Floor drains, taps and water heaters from the catalogue",
            ),
            Sep,
            act(
                "discipline:drainage",
                "Drainage",
                "droplets",
                "Waste and soil pipes: the standards (STN EN 12056-2, STN 73 6760) and the tools coming",
            ),
            act(
                "pipe:K",
                "Drain pipe",
                "pipe-fall",
                "The Pipe tool in foul drainage: PP-HT DN 100 falling 2 %",
            ),
            Sep,
            act(
                "discipline:gas",
                "Gas installation",
                "flame",
                "Gas pipes, the main shut-off and the meter: the standards (TPP 704 01) and the tools coming",
            ),
            act(
                "pipe:ZP",
                "Gas pipe",
                "pipe-gas",
                "The Pipe tool in natural gas: HERZ GASPRESS at 2.4 m",
            ),
        ],
        3 => vec![
            act(
                "discipline:heating.radiators",
                "Radiator heating",
                "heater",
                "Heat loss, radiators, pipes and balancing: the standards (STN EN 12831-1, STN EN 12828) and the tools coming",
            ),
            act(
                "discipline:heating.floor",
                "Underfloor heating",
                "layout-grid",
                "Loops, spacing and manifolds: the standards (STN EN 1264) and the tools coming",
            ),
            act(
                "discipline:heating.wall",
                "Wall heating",
                "columns-3",
                "Wall heating panels: the standards (STN EN 1264) and the tools coming",
            ),
            act(
                "discipline:heating.ceiling",
                "Ceiling heating",
                "rows-3",
                "Ceiling heating and cooling panels: the standards (STN EN 1264) and the tools coming",
            ),
            Sep,
            act(
                "pipe:UKP",
                "Heating pipe",
                "pipe",
                "The Pipe tool in heating supply; the return is in the bar's systems",
            ),
            Sep,
            act(
                "catalogue.cat:hvac",
                "Radiators and boilers",
                "thermometer",
                "Radiators, boilers and other heating fixtures from the catalogue",
            ),
        ],
        4 => vec![
            act(
                "discipline:ventilation",
                "Ventilation",
                "wind",
                "Ducts, terminals and units: the standards (STN EN 16798-3) and the tools coming",
            ),
            act(
                "pipe:VZP",
                "Duct",
                "pipe",
                "The Pipe tool in supply air: a Spiro duct Ø160 at 2.7 m; exhaust and rectangular ducts are in the bar",
            ),
            act(
                "catalogue.cat:hvac",
                "Diffusers and fans",
                "fan",
                "Diffusers, grilles, fans and split units from the catalogue",
            ),
        ],
        5 => vec![
            act(
                "discipline:medgas",
                "Medical gases",
                "cylinder",
                "O2, medical air, vacuum, N2O and CO2: the standards (STN EN ISO 7396-1) and the tools coming",
            ),
            act(
                "pipe:O2",
                "Medical gas pipe",
                "pipe",
                "The Pipe tool in oxygen: SANHA copper; AIR, VAC, N2O and CO2 are in the bar's systems",
            ),
        ],
        6 => vec![
            act(
                "discipline:electrical",
                "Electrical installation",
                "zap",
                "Circuits, cables, protection and lightning: the standards (STN 33 2000) and the tools coming",
            ),
            act(
                "catalogue.cat:electrical",
                "Sockets and switches",
                "plug",
                "Sockets, switches, boards and detectors from the catalogue",
            ),
            act(
                "catalogue.cat:lighting",
                "Lights",
                "lightbulb",
                "Ceiling, wall and recessed lights from the catalogue",
            ),
        ],
        ANNOTATE => vec![
            T(Tool::Dimension),
            T(Tool::Angle),
            T(Tool::Radius),
            T(Tool::Diameter),
            T(Tool::ArcLength),
            T(Tool::SpotLevel),
            Sep,
            T(Tool::Tag),
            act(
                "tag.all",
                "Tag all",
                "tags",
                "Tag every door, window, fixture and room in the view that has no tag yet (Shift+T)",
            ),
            T(Tool::Note),
            Sep,
            T(Tool::Detail),
            T(Tool::Callout),
            Sep,
            act(
                "styles",
                "Annotation styles",
                "anno-styles",
                "Named styles for dimensions, notes and tags: text, ends, tag shape, colour",
            ),
            act(
                "marks",
                "Mark numbering",
                "hash",
                "How doors, windows and other elements are numbered; renumber, and check for marks used twice",
            ),
        ],
        8 => vec![
            // levels are what plans are cut from, so they sit with the
            // views that come from them (MU20)
            T(Tool::Level),
            T(Tool::Section),
            T(Tool::Elevation),
            T(Tool::Callout),
            Sep,
            act(
                "crop.add",
                "Crop box",
                "crop-box",
                "Add a crop box to the 3D view: it crops everything outside it",
            ),
            act(
                "view.reference",
                "Reference",
                "underlay",
                "Trace over another view, a picture or a DXF drawing: shown faintly under this view (Alt+F2 hides it)",
            ),
            act("new.sheet", "New sheet", "file-text", "A new drawing sheet"),
            Entry::Menu {
                key: "tab:schedules",
                label: "New schedule",
                icon: "table-2",
                hint: "A schedule of doors, windows, walls, rooms and more",
            },
            Sep,
            act(
                "graphicstyles",
                "Graphic styles",
                "align-left",
                "Style packs: pens by scale, object styles, hatch patterns and view templates",
            ),
            act(
                "templates",
                "Sheet templates",
                "sheet-template",
                "Title blocks for sheets",
            ),
        ],
        _ => vec![
            // the Levels manager is project set-up; the Level tool, which
            // makes a level view, is on the View tab (MU21)
            act(
                "levels",
                "Levels",
                "level",
                "Add, duplicate, rename, move and delete levels",
            ),
            T(Tool::Grid),
            T(Tool::Guide),
            Sep,
            act(
                "project.info",
                "Project information",
                "info",
                "Name, client, address, site and building: they fill the title blocks and the IFC export",
            ),

            act(
                "types.browser",
                "Types",
                "element-types",
                "Wall, curtain wall, door, window, slab, roof, column, beam and object types",
            ),
            act(
                "profiles.browser",
                "Profiles",
                "profile",
                "The profile library: presets and the project's own, for sweeps",
            ),
            act(
                "mat.browser",
                "Materials",
                "palette",
                "Add, edit, delete and assign the project's materials",
            ),
            act(
                "graphicstyles",
                "Style packs",
                "align-left",
                "Pens by scale, object styles, hatch patterns and view templates",
            ),
            act(
                "units",
                "Project units",
                "units",
                "Metric or imperial: how lengths are shown and typed",
            ),
            act(
                "cleanup.stacked",
                "Copies on originals",
                "copy",
                "Find and remove elements sitting exactly on a copy of themselves (left by a Duplicate that never moved)",
            ),
            act(
                "cleanup.overlap",
                "Overlapping walls",
                "walls-overlap",
                "Find walls running along each other and overlapping: the plan merges them, but in 3D they stay two solids. Merge keeps one wall covering both",
            ),
            Sep,
            Entry::Menu {
                key: "tab:ifc",
                label: "IFC",
                icon: "upload",
                hint: "Open an IFC file, or export this project as IFC4 or IFC5",
            },
        ],
    }
}

/// Does tab `tab` show `tool`?
pub fn holds(tab: usize, tool: Tool) -> bool {
    ALWAYS.contains(&tool) || entries(tab).contains(&Entry::Tool(tool))
}

/// The first tab that shows `tool`.
pub fn home_of(tool: Tool) -> usize {
    (0..TABS.len()).find(|t| holds(*t, tool)).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_two_buttons_on_a_tab_share_an_icon() {
        // A5-1: two buttons that look the same read as one
        for (tab, tab_name) in TABS.iter().enumerate() {
            let mut seen: Vec<(&str, String)> = vec![];
            for e in entries(tab) {
                let (icon, name) = match e {
                    Entry::Tool(t) => (crate::app::shell::tool_icon(t), t.label().to_string()),
                    Entry::Act { icon, label, .. } | Entry::Menu { icon, label, .. } => {
                        (icon, label.to_string())
                    }
                    Entry::Sep => continue,
                };
                if let Some((_, other)) = seen.iter().find(|(i, _)| *i == icon) {
                    panic!("{tab_name}: {name} and {other} both use the {icon} icon");
                }
                seen.push((icon, name));
            }
        }
    }

    #[test]
    fn every_tool_is_on_a_tab() {
        for t in Tool::ALL {
            assert!(
                (0..TABS.len()).any(|tab| holds(tab, t)),
                "{} is on no tab",
                t.label()
            );
        }
        assert_eq!(home_of(Tool::Wall), 0);
        assert_eq!(home_of(Tool::Dimension), ANNOTATE);
        assert_eq!(home_of(Tool::Section), ANNOTATE + 1);
        assert_eq!(home_of(Tool::Grid), ANNOTATE + 2);
        // the terrain tools are on Cadastre too, but at home in Architecture
        assert_eq!(home_of(Tool::Terrain), 0);
        assert!(holds(1, Tool::Terrain));
        // the callout is on both of its tabs
        assert!(holds(ANNOTATE, Tool::Callout) && holds(ANNOTATE + 1, Tool::Callout));
        // Select is on every one
        assert!((0..TABS.len()).all(|t| holds(t, Tool::Select)));
    }
}
