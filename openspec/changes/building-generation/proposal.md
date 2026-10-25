# Proposal: building-generation

## Why

Owner, 2026-10-23: "We're going to model buildings, cities, people, and the mech in high detail."
Owner, 2026-10-24: "I want buildings to look nice, and be fully destructible."

Today every building is one grey box (`procgen-world`). A 12 m mech walks past them at cockpit height
and flies over their roofs, so facades and roofs are what the player looks at most. They need
storeys, window bays, storefronts, cornices, fire escapes and roof clutter, at true scale.

## What changes

- A building is generated from its lot by a split grammar in `cb-core`. Each step forks the seed by
  label (`building/<id>`, `plan/<storey>`, `roof`).
  - lot to mass (footprint on the 6 m structural grid, podium and setbacks)
  - mass to storeys (ground storey 4.2-5.5 m, upper storeys 3.3-4.5 m by archetype)
  - each storey to facade panels, one per structural bay per side
  - each panel to modules (punched windows with sills and lintels, ribbon windows with fins, curtain
    wall with mullions and spandrels, industrial steel sash, storefronts with sign bands and awnings)
  - roofs to parapets, copings and cornices, bulkheads over the core, and props placed by rejection
    sampling clear of the parapet (water tanks, HVAC, vents, ducts, dishes, masts, chimneys, TV
    aerials, skylights)
- **Archetypes are data** (`data/world/buildings.ron`): storey ranges and heights, skin, cladding,
  column inset, roof prop list, use per storey. Four to start, all from the 1980s alt-history setting
  (survey G1): brick walk-up, brutalist office, curtain-wall tower, brick warehouse.
- **Every piece belongs to a structural element** (column, floor bay, wall panel, glass, parapet,
  prop), the graph `structural-destruction` breaks. Generation and destruction share one grid and
  one element list.
- **Modules are a Blender kit** (`tools/blender/build_building_kit.py` on `hs_kit`'s CSG method).
  Each module is a glb in metres with LODs, pre-fractured cells and a manifest with triangle budgets.
- **Textures**: Material Maker for bulk materials (brick, concrete, stucco, roofing, asphalt).
  Star Crew's face-on Cycles render of modelled facade bays, trims and roof cells for far LODs (survey
  `W14`). Full PBR sets (section 10).
- **Drawing**: one mesh per material per building at LOD0, tagged by element. LOD1 is the facade as
  flat quads with the rendered bay maps and relief parallax. LOD2 is the box with a facade atlas.

## Presented by

`docs/mockups/buildings.html` (shots in `docs/screenshots/buildings/`). Research:
`docs/analysis/buildings-research.md`.

## Out of scope

Interiors and furniture (`building-interiors`), breaking (`structural-destruction`), rubble
(`rubble`), organic street layouts (survey W1), people.
