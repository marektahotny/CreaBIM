# Pipes and ducts

Pipes and ducts of building services: water, drainage, gas, heating, cooling, medical gases, flues and ventilation. Each is a straight segment in a **system**, of a **pipe type** (a series such as copper STN EN 1057, HERZ PipeFix or PP-HT) and a **size** in it.

1. Pick the **Pipe** tool, or a pipe button on the *Plumbing and gas*, *Heating*, *HVAC* or *Medical gases* tab, which picks its system too.
2. In the tool options choose the *System*, the *Pipe type* and the *Size*, and the *Height* of the pipe's axis above the floor.
3. Click where the pipe starts, then each next point. Esc or right-click ends the run.

## Systems

Each system has its code, as drawings label it (SV cold water, TV hot water, C circulation, K foul drainage, D rainwater, PV fire water, ZP gas, UKP and UKS heating, CHP and CHS cooling, O2, AIR, VAC, N2O and CO2 medical gases, DYM flue, VZP and VZO ventilation), its colour, and the pipe type, size and height a new pipe takes. Drains fall by their system's slope (2 % foul, 1 % rainwater) as they are drawn, and a run goes on from where its last pipe ended.

## Pipe types

The types are PilonDesigner's catalogue: a system offers the types for what it carries. Each has its sizes with their outside and inside diameters, its roughness for the calculations, its joints and standards, and for manufacturers' series (HERZ, SANHA, GAWAPLAST) their fittings.

## Properties

Select a pipe to change its system, pipe type, size, insulation and the heights of its ends. Another type keeps the outside diameter as near as its series allows, as where a run changes material. A drain shows its fall.

Pipes are drawn in plan in their system's colour, cut, seen below or dashed above the plan's cut, and in 3D as cylinders (a rectangular duct as a box). They export to IFC as IfcPipeSegment or IfcDuctSegment.
