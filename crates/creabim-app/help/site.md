# Site: terrain, hard landscape and planting

The **Site** tools (*Create › Site*) model the ground around the building: the terrain, paved and gravelled areas, and trees and shrubs. Cars and people come from the catalogue.

## Terrain

1. Pick the **Terrain** tool.
2. Set its *Depth* in the tool options.
3. Draw the ground's outline like a slab: `1` Polygon, `2` Rectangle, `3` Fill room or `4` From sketch.

New terrain is flat at its level and covered in *Grass* from the material library. To shape it, add **spot heights** in *Properties*. Click *Add spot height*, then type its X and Y position and its height above the level. The surface runs through every spot height and slopes between them. The outline's corners stay at the terrain's *Top* unless you put a spot height on them, so one spot height in the middle makes a mound and spot heights on the corners tilt the whole site. Set *Contours every* to draw contour lines in plans, for example every 250 mm. Spot heights show in plans as small crosses.

### Terrain from surveyed points

**Terrain from points** on the *Cadastre* tab makes a terrain from a surveyor's file, as PilonDesigner does with `teren.xyz`: XYZ, TXT, PTS, CSV or ASC, a point per line as `x y z` (spaces, tabs, semicolons or commas; a leading point number and header lines are skipped).

- Its outline goes round the outermost points, and the surface passes through every point. A large survey is thinned to about 1500 points on a grid, the one nearest each cell's middle kept.
- Points in **S-JTSK** (Y and X over 50 km) are turned to east and north, and their lower left whole metre comes to the project's origin.
- Heights above sea level (Bpv) come to the project's ±0,000: its *Elevation* in *Project information* when set, else the lowest point's whole metre. The status line says where the origin and ±0,000 are.
- A 3D DXF survey (points, 3D polylines, a TIN of 3D faces) makes one the same way: **Terrain from its heights** in the reference's *Properties*, from the layers shown, where the reference is placed.
- Contours are drawn every 0.25, 0.5 or 1 m, by how much the ground rises. Only the first 40 spot heights are listed in *Properties*.

### Map underlays

**Map underlays** on the *Cadastre* tab opens Pilon Map Downloader's site (mapy.pilondesign.eu) in a window of the app, as PilonDesigner does. Find the place, pick the area and what to take, and send it with the map's **Vložiť do Revitu** button: the app answers the map as Revit's Pilon Map Link does. The site plan, the orthophoto and the terrain come into the plan that is open, and the project is placed in S-JTSK, as with a map package below. The files are kept in a folder beside the project's file.

The app listens for the map on this computer only, and answers the map's own site only.

### A map package

**Map package** on the *Cadastre* tab puts in a ZIP from Pilon Map Downloader, as PilonDesigner does: pick the ZIP with a plan open, and its files are unpacked into a folder of the same name beside it.

- `situacia.dxf` (the cadastre, buildings, networks and contours) becomes a reference in the plan, in metres, where the map puts it.
- `ortofoto.jpg` with its world file `ortofoto.jgw` becomes a picture reference under the model, placed and sized by the world file. The terrain's fill covers it in the plan: untick *Terrain* under the view's categories to see it.
- `teren.xyz` becomes a terrain, as with **Terrain from points**, its heights to the project's ±0,000.
- `bod_pociatku.txt` gives where the project's origin is in S-JTSK (shown in *Project information* as **S-JTSK origin**) and its ±0,000 above sea level (Bpv), which becomes the project's *Elevation* when it has none yet.

When the package has both local and S-JTSK files, the local ones are taken; S-JTSK ones are brought to the origin from `bod_pociatku.txt`. Putting a package in again adds its references and terrain again: remove the old ones.

### Earthworks

The **Terrain** tool also draws earthworks, as PilonDesigner's terrain edits: pick **Building pit** or **Platform** in its bar, set the bottom (or height) above the level and the slopes 1:n, and draw the outline like a slab.

- A **building pit** is dug to its bottom with its sides sloping up to the ground, and back-filled once the building stands: the ground stays as it was. Plans draw its bottom, the slopes' top edge dashed and hachures down the slopes.
- A **platform** levels the ground at its height with slopes down or up to the ground round it. It changes the terrain for good: the 3D view and the contours follow it.
- In *Properties*: the kind, bottom or height, slopes and the topsoil stripped first.

**Earthworks** on the *Cadastre* tab opens the schedule of earthworks: topsoil, the cut of platforms, the pit, trenches under strip footings and shafts under pad footings, moving the spoil, back-fill (the pit less the building's underground), fill and the surplus to a tip, each with its KROS/Cenkros code as a guide. Volumes are the ground before less after on its triangulated surface, in cells of 0.5 m. *Copy for a spreadsheet* copies it with semicolons, for Excel or Cenkros.

## Hard landscape

**Hard landscape** areas are slab-like layers on the ground, each with its own material and surface pattern.

1. Pick the **Hard landscape** tool.
2. In the tool options, pick what to draw:
   - **Paving**: 600 × 400 concrete flags.
   - **Driveway**: block paving laid in stretcher bond.
   - **Gravel**: stippled.
   - **Decking**: boards.
   - **Asphalt**: no pattern.
3. Draw the area's outline like a slab.

The picks are only starting points. In *Properties*, change the area's *Top*, *Depth* and **Pattern**:

- **Units**: their length, width, staggered courses and angle.
- **Lines**: their spacing and angle.
- **Stipple**: its spacing.

Plans draw the pattern over the area, and 3D shows the area in its material. An area drawn inside flat terrain sits flush: the ground leaves room for it.

## Planting

1. Pick the **Planting** tool.
2. In the tool options, pick a **Tree**, **Conifer**, **Palm** or **Shrub**, and its *Canopy* (the spread of its crown) and *Height*.
3. Click to plant. Keep clicking to plant more.

In plan, each plant is a symbol as wide as its canopy: a scalloped crown with its trunk for a tree, a ringed star for a conifer, fronds for a palm, a smaller scalloped outline for a shrub. In 3D, sections and elevations it is a simple form: a trunk under a round crown, a cone, a tall trunk with a head of fronds, or a low mound. Give it a material to change the colour of its crown.

## Cars and people

*Create › Site › Cars and people…* opens the [Catalogue](furniture) at **Entourage**: a car, a person and a child, for scale. Place them like any other object.

## Reshaping

Terrain and hard landscape areas have the same grips as a slab: drag a corner or an edge, and open them in [Edit mode](edit-mode) (`Enter`) to add or remove corners or change an area's holes. Move plants with the [Move](move) tool.

## Exchange and visibility

Terrain exports to IFC as *IfcGeographicElement* (TERRAIN), plants as *IfcGeographicElement*, and hard landscape as *IfcCivilElement*. The IFCX format reads them back as the same editable elements. Views can hide them through the *Terrain and hard landscape* and *Planting* categories (see [Visibility](visibility)).
