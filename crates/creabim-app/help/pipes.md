# Pipes and ducts

Pipes and ducts of building services: water, drainage, gas, heating, cooling, medical gases, flues and ventilation. Each is a straight segment in a **system**, of a **pipe type** (a series such as copper STN EN 1057, HERZ PipeFix or PP-HT) and a **size** in it.

1. Pick the **Pipe** tool, or a pipe button on the *Plumbing and gas*, *Heating*, *HVAC* or *Medical gases* tab, which picks its system too.
2. In the tool options choose the *System*, the *Pipe type* and the *Size*, and the *Height* of the pipe's axis above the floor.
3. Click where the pipe starts, then each next point. Esc or right-click ends the run.

## Systems

Each system has its code, as drawings label it (SV cold water, TV hot water, C circulation, K foul drainage, D rainwater, PV fire water, ZP gas, UKP and UKS heating, CHP and CHS cooling, O2, AIR, VAC, N2O and CO2 medical gases, DYM flue, VZP and VZO ventilation), its colour, and the pipe type, size and height a new pipe takes. Drains fall by their system's slope (2 % foul, 1 % rainwater) as they are drawn, and a run goes on from where its last pipe ended.

## Fixtures and their connectors

Sanitary ware, appliances, radiators, manifolds and fan coils from the catalogue have **connectors**, as PilonDesigner places them: a basin its hot and cold water and waste, a WC its cold water and soil pipe, a radiator its heating supply and return, a fan coil its chilled water and condensate. Plan shows each as a small disc in its system's colour, and the object's properties list them with their size and height.

- Start a run on a connector and the tool takes its system and starts at its height.
- End a run on a connector of the same system and the pipe drops or rises to it there, and the run ends.

The catalogue's washbasin, WC, bath, shower, sink, floor drain, outside tap and radiator keep their own 3D models and gain the connectors. The washing machine, dishwasher, bidet, urinal, towel radiator, underfloor heating manifold and the wall, under-window and ceiling cassette fan coils are PilonDesigner's, sized and drawn as it draws them.

## Pipe types

The types are PilonDesigner's catalogue: a system offers the types for what it carries. Each has its sizes with their outside and inside diameters, its roughness for the calculations, its joints and standards, and for manufacturers' series (HERZ, SANHA, GAWAPLAST) their fittings.

## Properties

Select a pipe to change its system, pipe type, size, insulation and the heights of its ends. Another type keeps the outside diameter as near as its series allows, as where a run changes material. A drain shows its fall.

Pipes are drawn in plan in their system's colour, cut, seen below or dashed above the plan's cut, and in 3D as cylinders (a rectangular duct as a box). They export to IFC as IfcPipeSegment or IfcDuctSegment.
