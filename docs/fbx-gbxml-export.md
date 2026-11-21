# FBX and gbXML export

**Súbor › Export › FBX (.fbx) pre vizualizácie** (`creabim-exchange/src/fbx.rs`)

- Binary FBX 7.4 (Twinmotion, Lumion, Enscape, D5, 3ds Max, Cinema 4D, Blender, Unreal, Unity).
- Every element as it shows in 3D (the drawn scene, cutters left out) is one mesh; each of its
  layers or parts (plaster, masonry, insulation, frame, glass) has its own material.
- Tree: project › storey › kind (Steny, Dosky, Okná, Dvere, Strechy, Stĺpy, Nosníky, Schodiská,
  Zábradlia, Základy, Podhľady, Objekty…, Rozvody, Terén); elements of no storey under "Ostatné".
- Materials: the CreaBIM material's name, colour (linear), gloss and transparency (Phong).
- Flat normals, box-mapped UVs in metres (a 1 × 1 m texture tiles at true size).
- Z-up, centimetres (FBX's own unit, `UnitScaleFactor` 1). Each mesh keeps `creabim_id` as a user property.
- CLI: `creabim fbx <model> <out.fbx>`.

**Súbor › Export › Energetický model ako gbXML (.xml)** (`creabim-geom/src/gbxml.rs`)

- gbXML 7.03, validated against `GreenBuildingXML_Ver7.03.xsd`; `SurfaceReferenceLocation="InteriorSurface"`.
- Spaces = rooms (floor at the storey, ceiling at the room's height, inner faces of the walls),
  closed shells; loggias, terraces, balconies and porches are the outside, not spaces.
  Heated rooms `Heated`, the rest (garage, cellar) `Unconditioned`.
- Room sides split by what is beyond the wall: another room → `InteriorWall` (both spaces),
  nothing → `ExteriorWall`, the part under the outside ground → `UndergroundWall`.
- Under a room: the room below → `InteriorFloor`; lowest storey → `SlabOnGrade`
  (`UndergroundSlab` under the ground); else `ExposedFloor`. Over a room with no room above:
  `Roof` with the roof's build-up, or with the slab of the storey above (ceiling under an
  unheated attic). A pitched roof is flat at the room's ceiling.
- Constructions from the walls', slabs' and roofs' build-ups: U and layers (thickness, λ, R).
  Windows: `WindowType` with Uw and g; doors: construction with Ud (from the window/door data).
- Azimuth of each wall from true north; `CADModelAzimuth` = the project's true north.
- Building type from "Účel stavby" (rodinný dom → SingleFamily, bytový dom → MultiFamily, …).
- Known simplification: a room's floor takes the room under its label point (a room partly over a
  loggia is all over the room below).
- CLI: `creabim gbxml <model> <out.xml>` (warnings on stderr).
