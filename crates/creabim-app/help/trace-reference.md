# Trace and reference

A **reference** is something shown faintly under (or over) a plan or a section, to draw over or compare against:

- **another view**: the floor below, under the floor you are drawing;
- **a picture**: a scanned plan, a photo of a sketch (PNG, JPEG, TIFF or BMP);
- **a DXF drawing**: a survey or a consultant's plan. A DWG opens once saved as DXF: any CAD program, or the free ODA File Converter, does it offline;
- **a PDF page**: a plan sent as PDF, drawn at its scale 1:N, or a scanned page.

A reference is saved with the project. One reference can show in **several views** (a site survey under every floor). Pictures and drawings stay where they are on disk, as links: *Reload* reads them again after they change.

## Adding one

**Reference** on the *View* tab opens the **Trace and reference** tab of *Properties* for the view in the area you last used. Pick another view from *Add*, or click **Picture…**, **DXF drawing…** or **PDF…**. It is added in place and selected. A picture, DXF or PDF file dropped on a plan or section becomes a reference too.

## Placing it

- Drag the **square grip** in its middle to move it.
- Drag a **corner** to scale it about its middle.
- Drag the **round grip** above it to turn it about its middle (5° steps; `Alt` for any angle).
- Or type *X*, *Y*, *Rotation* and *Scale* in *Properties*.
- **Calibrate…**: click two points on it a known distance apart (a dimensioned wall on a scan), then type that distance. It is scaled to fit; the first point stays put.
- **Align…**: click two points on the reference, then the two points on the model they belong on: it is moved, turned and scaled onto them.
- **Back in place**: where it was drawn, unmoved, unturned, at its own size.
- **Move**, **Rotate** and **Scale** from the toolbar work on references as on any element.

In a section, a reference stands on the section's plane, and its grips and moves stay on that plane. Its lines snap: ends, middles and anywhere along them.

Its dashed outline shows when it is selected or under the pointer, in plans and in 3D; clicking the outline selects it. **Locked** keeps the grips off.

**Copy to view…** puts a separate copy of the same drawing in another view, with its own place, size and settings.

A trace placed in a [Profile view](profiles#the-profile-view) is kept to that profile: **Only in this profile** is ticked, and it is not drawn in plans, sections or 3D. Untick it to choose the views it shows in, as for any other reference.

## A DXF drawing

A DXF is read as PilonDesigner reads it: lines, polylines with arcs, circles, ellipses and splines, texts (TEXT and MTEXT), hatches (their pattern lines, solid fills as outlines), blocks with their arrays, dimensions, points and 3D faces, by layer and in metres from the file's units (millimetres when it gives none). Drawings saved before AutoCAD 2007 are read in their code page, so Slovak and Czech letters (ANSI 1250) come out right.

- **Layers**: tick the layers to show; *All* and *None* switch them together.
- **Texts** show at their size and turn once they are big enough on screen to read.
- **Points**: survey points, small circles marking them and blocks' insertion points snap.
- A drawing in national coordinates (**S-JTSK**, kilometres from the origin) comes to the project's origin, its lower left corner there; move it in *Properties*.
- A **3D survey** shows the range of its heights.

## A picture

A picture comes as wide as its pixels at its **resolution** drawn at **1:100**, as PilonDesigner places a scan: a scan's resolution is read from the file (PNG, JPEG, TIFF), else taken as 150 DPI. In *Properties*, set **Resolution** to the scan's DPI and **Drawing scale** to the scale printed on the drawing, and it comes to size (width = pixels ÷ DPI × N). When neither is known, *Calibrate…* sets the size from a known distance.

## A PDF page

A PDF is read as PilonDesigner reads it: the page's lines, its solid fills in their colour (white areas and the page's background left out) and its texts, at the drawing's **scale 1:N**: a PDF measures paper, so the model is the paper times N. It comes at 1:100; set **Drawing scale** in *Properties* to the scale printed on the drawing, and it grows or shrinks about its middle. A page turned in the PDF is read turned, as it is seen. **Page** picks another page of the file.

Lines, fills and texts are on the layers *PDF*, *PDF výplne* and *PDF texty*, so *Layers* turns them on and off as a DXF's.

A **scanned page** (a picture with no lines) comes as its picture, placed where it lies on the page. Scans saved as JBIG2, CCITT or JPEG 2000 can't be read here: export the page as PNG or JPEG and add that. Check the size with *Calibrate…* when the scale isn't known.

## Clip

**Clip…**: click two opposite corners in the view; only what lies between them shows (lines are cut at the edge). **Whole** shows all of it again.

## How it looks

- **Colour**: red, blue, grey or green, or *its own colours* (a picture's).
- **Strength**: how faint it is.
- **Under** or **Over** the model.
- **Hidden**, or `Alt+F2` to hide or show every reference in the view.

## In 3D

Under **In 3D** in its *Properties*: **Shown** draws the reference in the 3D view too, on its level or its section's plane (a picture shows its image), where it can be picked and snapped to. It can have its **Own strength** there, and be drawn **In perspective** or flat.

References are never in exports, schedules or IFC.
