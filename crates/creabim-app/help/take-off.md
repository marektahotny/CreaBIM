# Take-off

The **take-off** measures what the model is made of: how many square metres of plaster, cubic metres of concrete, metres of beam. Give a material a **rate** and the take-off prices it. Like a schedule, it is read live from the model, so it changes as you draw.

## What it measures

Walls, slabs, roofs, ceilings, columns, beams and footings each give one line for each material they are built of:

- an element with an [assembly](assemblies) gives a line for each layer (an air gap with no material is left out), as thick as the layer;
- anything else gives one line for its own material.

| Element | Area | Volume | Length |
|---|---|---|---|
| Wall | its face, less its doors and windows | area × thickness | along the wall |
| Slab, ceiling | plan area, less holes | area × thickness | - |
| Roof | the area of its slopes | area × thickness | - |
| Column | - | section × height | its height |
| Beam | - | section × length | its length |
| Footing | its plan area | area × thickness | a strip's length |

Walls and strip footings are measured on their joined outline, so a corner where two walls meet is counted once, and a wall stopping at another's face isn't counted into it.

**Volumes are cleaned up and trimmed.** The volume of walls, slabs, roofs, ceilings, columns, beams, footings and stairs is measured on their 3D solids as the 3D view and the drawings show them: after joins (a slab running through a wall, a column or a system lintel in a wall), after cuts, and with a wall's gable under the roof. A wall's, slab's or roof's area is then that volume over the layer's thickness. While the 3D model is worked out again after a change, the table shows the last measured values and says *measuring the trimmed 3D solids…* in its heading.

The take-off also lists, each under its own category: **doors** and **windows** (one piece each, with their size as area), **objects** (one piece each), **stairs** (the run as length, its plan area, and the concrete as a builder estimates it), **railings** (their length) and **masses** (their volume as modelled, modifiers and scale included).

Quantities are in metres, square metres and cubic metres.

## Rates

A rate is a price for each square metre, cubic metre, metre or piece of a material, in whatever currency you price the project in. Set it:

- in the *Materials* window: *Rate*, a number and what it is *per*; *Clear* takes it away;
- or in the *Take-off* table itself, in the *Rate* and *Per* columns.

A material without a rate is measured but left out of the priced total.

## The Take-off table

*File › New schedule › Take-off* (or *Create › Schedule › Take-off*). It opens as a spreadsheet: a grid, a heading that stays put while the rows scroll, groups with subtotals and the totals at the foot.

- **Every material on its own.** A layered construction (a wall's plaster, masonry, insulation and render; a floor; a roof) gives a row for each of its layers. Materials of the same thickness in one group add up into one row.
- **Group by**, in the table's bar: up to three levels of *Element type* (walls, slabs, roofs…), *Level*, *Construction* (the element's type or assembly) and *Material*. By default the take-off is split by element type; *Level › Element type* lists each level's walls, slabs… on their own. A group shows its volume and cost (a material's group its area and length too); its triangle folds it, *Fold all* / *Unfold all* do all at once.
- **Each element on its own row** lists every layer of every element on a row of its own, with its element, construction, level and layer.
- **Sorting:** a click on a column's heading sorts the rows in each group by it, a second click turns the order round, a third leaves them as they come.
- **Find** keeps only the rows whose material, element, construction or element type holds the text.
- **Showing where it is:** click a row (Ctrl adds more, a group's row takes the whole group) and press **Show in 3D** or **Show in plan**. The elements are selected and framed; a 3D view opens beside the table, or the level's floor plan under it, if none is open. A double-click on a row shows it in 3D.
- **Doors and windows** have a *Perimeter* column (twice width plus height, added up).
- **Window sills**, outer (sheet metal) and inner (boards), are listed under *Outer sills* and *Inner sills*: a row for each size, named with its width (across the wall) and length, with the number of pieces and their length and area. An outer sill reaches 40 mm over the wall's outer face; an inner one 20 mm over the inner face and 20 mm into each jamb. A window's *Outer sill* / *Inner sill* switches leave it out.
- A material's *Rate* and *Per* are edited right in its row.
- The grouping, the element rows and the sorting are saved with the take-off and print the same on a sheet. See [Sheets](sheets).

## To Excel

*File › Export › Schedules to Excel (.xlsx)*, or **Export Excel** on any schedule, writes a workbook: a *Summary*, a sheet for each schedule, then *Materials by element* (the take-off split by element type, every material on its own), *Take-off* (totals by material and by level, priced) and *Take-off lines* (every element's materials, layer by layer, with a filter on each column). Numbers go in as numbers, so the spreadsheet can sum and sort them.

> **Tip:** From the command line, `creabim xlsx house.creabim house.xlsx` writes the same workbook.
