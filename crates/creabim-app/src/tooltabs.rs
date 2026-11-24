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
            act(
                "joinery:edit",
                "Window and door editor",
                "layout-grid",
                "Split the selected window or door by mullions and transoms and set how each field opens",
            ),
            Sep,
            T(Tool::Slab),
            T(Tool::Shaft),
            T(Tool::Roof),
            T(Tool::Gutter),
            T(Tool::Downpipe),
            T(Tool::Capping),
            T(Tool::ChimneyFlashing),
            T(Tool::RoofOutlet),
            T(Tool::Stair),
            T(Tool::Railing),
            act(
                "railing:check",
                "Check railings",
                "problem",
                "Railings and window sills against STN 74 3305: where a railing is needed, how high, the gaps in its infill, the windows whose sill is too low over a fall",
            ),
            Sep,
            T(Tool::Column),
            T(Tool::Beam),
            Entry::Menu {
                key: "tab:footing",
                label: "Footing",
                icon: "footing",
                hint: "Foundations: a strip footing under walls, a pad footing or a bored pile under columns (on their axis) or on its own",
            },
            Entry::Menu {
                key: "tab:masonry",
                label: "Masonry",
                icon: "brick-wall",
                hint: "System lintels (PORFIX, Porotherm) put by hand in the storey's masonry plan, RC columns in walls, and the check of how far masonry walls run between braces",
            },
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
            act(
                "map:open",
                "Map underlays",
                "globe",
                "The map site in a window (Pilon Map Downloader): pick the area, and the cadastre, orthophoto and terrain come into the plan, the project placed in S-JTSK",
            ),
            act(
                "map:zip",
                "Map package",
                "map",
                "A ZIP from Pilon Map Downloader: the cadastre and site plan, the orthophoto under it, the terrain, and where the project lies in S-JTSK and above sea level",
            ),
            act(
                "catalogue.cat:site",
                "Site and connections",
                "box",
                "The water meter shaft with its assembly, Wavin Tegra 425 and 600 inspection chambers, the electricity meter pillar and the gas meter box, from the catalogue",
            ),
            act(
                "site:soakaway",
                "Soakaway",
                "droplets",
                "A rainwater soakaway of DRENBLOK blocks in 1 to 5 layers with its filter shaft and overflow, sized after DWA-A 138 from the connected area, the ground's permeability and the rain series",
            ),
            act(
                "site:retention",
                "Retention tank",
                "droplet",
                "A rainwater retention tank of DRENBLOK blocks wrapped in foil (EKODREN RN), its filter shaft ahead and a throttled outlet to the recipient, sized after DWA-A 117 from the connected area, the allowed outflow and the rain series; with its parts list",
            ),
            act(
                "site:network",
                "Utility network",
                "spline",
                "Draw a utility network in the site plan: water, sewers, gas, power, lighting, telecom, heating, drainage. Colour, line type, DXF layer and code by its kind; proposed heavy, existing thin, cancelled crossed; set in Properties",
            ),
            act(
                "site:place",
                "Place the building",
                "move",
                "Lays the site under the building: click a side of the place drawn on the map near its corner, then the building's matching side near its corner. The map, terrain and what was drawn on them move and turn as one; the building and the floor plans stay as drawn",
            ),
            act(
                "site:exact",
                "Place the building exactly",
                "ruler",
                "Sets the building on the plot by typed distances: from the side boundary its side runs along, and from the rear boundary (askew too) at the nearest corner. The map moves under the building; the building and the floor plans stay as drawn",
            ),
            act(
                "site:reset",
                "Reset the coordinates",
                "refresh-cw",
                "Puts the map back where it came down: every drawing in S-JTSK straight on its own coordinates, the project's S-JTSK origin at ±0,000, the orthophoto, terrain and what was drawn on the map with them. The building stays; Ctrl+Z undoes it",
            ),
            act(
                "site:symbols",
                "Site symbols and networks",
                "map-pin",
                "The site plan's networks (6.152–6.732, 8.021) and point symbols (hydrant, valve, manhole, gully, overflow, gas sniffer, outdoor lamps 6.120–6.580, the entrances to the building and the plot) after the map symbol tables. What is placed goes into the site plan's legend",
            ),
            act(
                "site:coords",
                "S-JTSK coordinates",
                "crosshair",
                "A coordinate mark: click a point (a plot's corner, a cadastre line's bend, the building's corner), then where the text goes; it writes the point's Y and X in S-JTSK, as the cadastre does",
            ),
            Sep,
            T(Tool::Terrain),
            act(
                "terrain:points",
                "Terrain from points",
                "mountain",
                "A terrain through surveyed points (XYZ, TXT, PTS, CSV; S-JTSK too): its outline round them, each a spot height",
            ),
            act(
                "earthworks:schedule",
                "Earthworks",
                "calc",
                "The schedule of earthworks: topsoil, the pit, platforms' cut and fill, trenches, back-fill and haul, with KROS codes. Draw pits and platforms with the Terrain tool",
            ),
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
                "calc:water",
                "Calculate water supply",
                "calc",
                "Size the cold and hot water pipes drawn to the fixtures (STN EN 806-3) and check the pressure",
            ),
            act(
                "calc:circulation",
                "Calculate circulation",
                "refresh-cw",
                "Size the hot water circulation from the heat its loops keep up (7 W/m in risers, 10 W/m along), find the pump and what each loop's balancing valve takes (STN EN 806-2)",
            ),
            act(
                "valves:auto",
                "Propose valves",
                "wrench",
                "Add the valves the pipes drawn still need: the meter assembly, shut-offs and angle valves on water (STN EN 806-2), vent heads and cleanouts on waste (STN EN 12056-2), radiator valves and shut-offs on heating, the main shut-off and cocks on gas",
            ),
            act(
                "routes:check",
                "Check routes",
                "problem",
                "Pipes and ducts through beams and lintels (a level pipe can be led under), gas nearer other pipes than 20 mm (TPH 03 2012 F.5.7), passages into another fire zone",
            ),
            act(
                "calc:fittings",
                "Fittings schedule",
                "table-2",
                "The elbows, tees, branches and reducers the pipes need where they meet, counted by system",
            ),
            act(
                "catalogue.cat:bathroom",
                "Sanitary fixtures",
                "fixtures",
                "Basins, WCs, baths and showers from the catalogue",
            ),
            act(
                "calc:fixtures",
                "Legend of fixtures",
                "file-text",
                "The sanitary fixtures placed, each type with its mark, trap and connection, as the legend of a plumbing drawing",
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
            act(
                "calc:drain",
                "Calculate drainage",
                "calc-drain",
                "Size the waste pipes and stacks from the fixtures on them: Qww = K·√ΣDU, horizontal pipes half full on their fall, stacks by their capacity (STN EN 12056-2)",
            ),
            act(
                "pipe:D",
                "Rainwater pipe",
                "roof-flat",
                "The Pipe tool in rainwater drainage: PP-HT DN 100 falling 1 %",
            ),
            act(
                "calc:storm",
                "Calculate storm water",
                "gauge",
                "Size the rainwater stacks and drains from the roof drains on them and their areas (STN EN 12056-3)",
            ),
            Sep,
            act(
                "pipe:PV",
                "Fire water pipe",
                "git-commit-horizontal",
                "The Pipe tool in the fire main to the hose systems",
            ),
            act(
                "calc:fire",
                "Calculate fire main",
                "book-check",
                "Size the fire main for the two worst hose systems at once and find the pressure needed at the inlet (STN 92 0400)",
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
            act(
                "calc:gas",
                "Calculate gas",
                "calc-gas",
                "Size the gas pipes from the appliances on them: reduced flow, 100 Pa to the farthest appliance, the meter and the cabinet (TPP 704 01)",
            ),
            Sep,
            act(
                "viewmode:services",
                "Services drawing",
                "layers-2",
                "The open view for a services drawing: the building in halftone outlines, pipes, ducts and fixtures in full over it",
            ),
            act(
                "viewmode:architecture",
                "Architecture drawing",
                "pencil",
                "The open view as the architect draws it: hatches, dimensions, tags and notes",
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
            act(
                "calc:heating",
                "Calculate heating pipes",
                "calc-heating",
                "Size the supply and return to the radiators, find the critical circuit and the pump, and preset the HERZ valves (TS-98-V, STRÖMAX) to balance the circuits",
            ),
            act(
                "valves:auto",
                "Propose valves",
                "wrench",
                "Radiator valves, shut-offs and a strainer, air vents at the high points and drain cocks at the low points of the heating and cooling pipes (STN EN 12828)",
            ),
            act(
                "routes:check",
                "Check routes",
                "problem",
                "Pipes and ducts through beams and lintels (a level pipe can be led under), gas nearer other pipes than 20 mm (TPH 03 2012 F.5.7), passages into another fire zone",
            ),
            act(
                "calc:herzdp",
                "HERZ differential pressure regulator",
                "circle-dot-dashed",
                "A HERZ 4002, 4007 or F 4007 regulator for a circuit's flow and loss: its setting, the STRÖMAX partner valve and the pump's head, as HERZ's design tool works them out",
            ),
            Sep,
            act(
                "calc:heatloss",
                "Heat loss",
                "gauge",
                "The design heat loss of every room (STN EN 12831-1): through walls, windows, floor and roof and by ventilation, and the building's total",
            ),
            act(
                "calc:floorheat",
                "Design underfloor heating",
                "spline",
                "Give rooms their underfloor loops from their heat loss: spacing, loops of 100 m at most, flows and what to set at the manifold (STN EN 1264)",
            ),
            act(
                "calc:radiators",
                "Size radiators",
                "table-2",
                "Radiators for every room from its heat loss, less what its underfloor heating gives: type, length, output and flow (STN EN 442)",
            ),
            act(
                "heat:source",
                "Heat source",
                "flame",
                "A hot water cylinder, a gas condensing boiler or a heat pump from the range: sized by the heat load, a heat pump by its bivalence point, then placed",
            ),
            act(
                "catalogue.cat:hvac",
                "Radiators and boilers",
                "thermometer",
                "Radiators, boilers and other heating fixtures from the catalogue",
            ),
            Sep,
            act(
                "viewmode:services",
                "Services drawing",
                "layers-2",
                "The open view for a services drawing: the building in halftone outlines, pipes, ducts and fixtures in full over it",
            ),
            act(
                "viewmode:architecture",
                "Architecture drawing",
                "pencil",
                "The open view as the architect draws it: hatches, dimensions, tags and notes",
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
            Sep,
            act(
                "calc:air",
                "Air balance",
                "refresh-cw",
                "The air every room needs, supply and exhaust through its diffusers and grilles, and the flow of the air handling unit (STN EN 16798-1)",
            ),
            act(
                "calc:recuperation",
                "Heat recovery design",
                "thermometer",
                "Designs ventilation with heat recovery in one step: the ATREA DUPLEX unit, distribution boxes, flexible ducts Ø75 through the doors to a plate valve in every room, room units in the outer wall for the rooms marked so; pressure loss, transfer under the doors and the take-off VZT – recuperation with prices",
            ),
            act(
                "calc:ducts",
                "Duct sizing",
                "gauge",
                "The air every duct carries, its velocity as drawn and the diameter for the velocity allowed; sets the proposed diameters on the round ducts",
            ),
            act(
                "routes:check",
                "Check routes",
                "problem",
                "Pipes and ducts through beams and lintels (a level pipe can be led under), gas nearer other pipes than 20 mm (TPH 03 2012 F.5.7), passages into another fire zone",
            ),
            act(
                "calc:cooling",
                "Cooling load",
                "sun",
                "The cooling load of every room in summer (STN 73 0548): sun through the windows by where they face, walls, roof, people and appliances, and the fan-coil that covers it",
            ),
            Sep,
            act(
                "viewmode:services",
                "Services drawing",
                "layers-2",
                "The open view for a services drawing: the building in halftone outlines, pipes, ducts and fixtures in full over it",
            ),
            act(
                "viewmode:architecture",
                "Architecture drawing",
                "pencil",
                "The open view as the architect draws it: hatches, dimensions, tags and notes",
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
            act(
                "calc:medgas",
                "Calculate medical gases",
                "gauge",
                "Size the O2, AIR and VAC pipelines from their outlets by the kind of room they are in (Q1 + (n − 1)·Δ), at most 20 kPa to the worst outlet (STN EN ISO 7396-1)",
            ),
            act(
                "routes:check",
                "Check routes",
                "problem",
                "Pipes and ducts through beams and lintels (a level pipe can be led under), gas nearer other pipes than 20 mm (TPH 03 2012 F.5.7), passages into another fire zone",
            ),
            act(
                "med:unit",
                "Supply unit",
                "heart-pulse",
                "A wall rail NR-H3 or a ceiling bridge SM-P 5.4 made to order: its gas outlets, sockets, data and nurse call, then placed",
            ),
            act(
                "med:box",
                "Valve box",
                "columns-3",
                "A zone valve box for 1 to 4 gases made to order: flush or on the wall, the gas out on top or at the bottom, Cu 18 to 42, its valves, NIST inlet and alarm switch, then placed",
            ),
            act(
                "catalogue.cat:medical",
                "Medical units",
                "fixtures",
                "The usual wall rail and ceiling bridge from the catalogue",
            ),
            Sep,
            act(
                "viewmode:services",
                "Services drawing",
                "layers-2",
                "The open view for a services drawing: the building in halftone outlines, pipes, ducts and fixtures in full over it",
            ),
            act(
                "viewmode:architecture",
                "Architecture drawing",
                "pencil",
                "The open view as the architect draws it: hatches, dimensions, tags and notes",
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
            Sep,
            act(
                "calc:circuits",
                "Circuits",
                "circuits",
                "The installation's circuits: devices and their power, breakers, cables, phases, RCDs, the check of each and the building's power balance; proposes circuits for devices that have none",
            ),
            act(
                "calc:circuit",
                "Circuit check",
                "calc",
                "A circuit's design current, voltage drop, protection IB ≤ In ≤ Iz and fault loop, with the smallest breaker and cable that pass (STN 33 2000-5-52)",
            ),
        ],
        ANNOTATE => vec![
            T(Tool::Line),
            T(Tool::Polyline),
            T(Tool::Arc),
            T(Tool::Curve),
            Sep,
            T(Tool::Dimension),
            T(Tool::Angle),
            T(Tool::Slope),
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
            T(Tool::Text),
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
            Sep,
            act(
                "viewmode:services",
                "Services drawing",
                "layers-2",
                "The open view for a services drawing: the building in halftone outlines, pipes, ducts and fixtures in full over it",
            ),
            act(
                "viewmode:architecture",
                "Architecture drawing",
                "pencil",
                "The open view as the architect draws it: hatches, dimensions, tags and notes",
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
            act(
                "viewmode:architecture",
                "Architecture drawing",
                "pencil",
                "The open view as the architect draws it: hatches, dimensions, tags and notes",
            ),
            act(
                "viewmode:blind",
                "Blind drawing",
                "square-dashed",
                "The open view in outlines only: no dimensions, tags, notes, room names or wall fills, to hand to the other professions",
            ),
            act(
                "viewmode:services",
                "Services drawing",
                "layers-2",
                "The open view for a services drawing: the building in halftone outlines, pipes, ducts and fixtures in full over it",
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
                "norms:tables",
                "Norm tables",
                "book-open",
                "The tables of the norms (fire safety, heat loss, cooling, climate, railings, stairs) in one place: read a folder of table files at once, see which tables each check needs and which are missing, read it again when they change",
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
    // the Footing tool is behind its menu of kinds
    let menu = |key: &str| {
        entries(tab)
            .iter()
            .any(|e| matches!(e, Entry::Menu { key: k, .. } if *k == key))
    };
    ALWAYS.contains(&tool)
        || entries(tab).contains(&Entry::Tool(tool))
        || (tool == Tool::Footing && menu("tab:footing"))
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
                // an icon icons.js lacks draws as an empty button
                assert!(
                    include_str!("../ui/icons.js").contains(&format!("\"{icon}\":")),
                    "{tab_name}: {name} names {icon}, which icons.js lacks"
                );
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
