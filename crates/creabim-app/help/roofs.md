# Roofs

## Drawing a roof

1. Pick the **Roof** tool.
2. In the tool options choose its shape and its **Pitch** (slope, in degrees):
   - **Flat**
   - **Mono-pitch**: one slope
   - **Gable**: two slopes meeting at a ridge, with gable ends
   - **Hip**: slopes on every side
3. Draw its outline like a slab: `1` Polygon, `2` Rectangle, `3` Fill room, `4` From sketch. With *Fill room*, the roof it would make shows under the pointer before you click, and the gaps that stop it in red. It runs round curved walls too.

![The starter house: a gable roof, and a flat roof that runs on over the terrace](img/roofs.jpg)

## Changing a roof

In *Properties*:

- the shape and pitch;
- the overhang past the outline;
- **Base offset**: how high the eaves sit above the level, such as 3.00 m to sit on a storey's walls;
- **Eave edge** and **Next edge**: which edge is the eave of a mono-pitch roof, or which edges are gable ends;
- its thickness, material or [assembly](assemblies), and its holes.

The summary line gives the corners, the area including the overhang and the roof's height. *Properties* also says where the eaves end, and warns when a steep pitch and a long overhang bring them below head height over the walls.

## Roofs in plan

A plan that looks down on a roof from above it, such as a roof plan, draws an arrow down each sloping face toward its eave, with the pitch beside it. The arrows come from the roof's shape, so they follow changes to its pitch and edges. A flat roof has none; write its fall in a [note](notes).

## Walls under a roof

Select walls and use *Attach top to roof/slab*: their tops follow the roof's underside, cut to its slope. See [Walls](walls#base-and-top).

> **Note:** Gable and hip roofs need a convex outline (no inward corners). On an L-shaped outline the roof stays flat, and *Properties* says so.

## Gutters and downpipes

The **Gutter** tool hangs an eaves gutter along a roof: click along the roof's eave corner by corner, then click the last point again or press Enter. The gutter keeps to the roof at the offsets set in the top bar: *Offset* from the eave line out to the gutter's back edge in plan, and *Below eave* from the roof's top at the eave down to the gutter's top edge. Pick a half-round or square gutter and its size there too.

The **Downpipe** tool puts a downpipe on the gutter clicked. Move a downpipe to slide it along its gutter, or type *Along the gutter* in *Properties*. The gutter falls to its downpipes by its *Fall* (mm per m): each run falls from a high point, midway between two downpipes or at the gutter's end, to the nearest downpipe.

A downpipe's *Properties* show the plan area of roof that drains to it and the flow from it, Q = r · A · C, with r and C set on the gutter (0,03 l/s·m² and 1,0 by default). The roof plan draws each gutter, its downpipes, their marks and the fall's arrows.

The **Table of sheet-metal items** (a schedule) lists each item once with its mark (K-1, K-2…), its sign, its description, its developed width, its length and its accessories (hooks, end caps, outlets, elbows, clamps), and under it the roof planes' area, the covering and the ridge and hip lengths. Place it on the roof plan's sheet.
