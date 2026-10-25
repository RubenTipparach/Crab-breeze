# Proposal: structural-destruction

## Why

Owner, 2026-10-24: "I want buildings to look nice, and be fully destructible", "fast and efficient
breakage of structures".

`building-destruction` (built) breaks 6 m cells and brings a building down only when a storey has
under 35 % left. A hit building reads as voxels, collapse has no cause the player can see, and every
hit remeshes the building.

## What changes

This change replaces the cell model with a structural graph on the same 6 m grid.

- **Elements**: every piece `building-generation` and `building-interiors` make is one element. That
  covers columns, floor bays, wall panels, glass, parapets, partitions, cores, furniture and props.
  Each element has a mass, hit points in megajoules and the elements it rests on.
- **Blasts** use the crater rule's radius (`2.2 m x E^(1/3)`, `terrain-deformation`) and take
  `E x (1 - d/R)^2` from each element within reach. Glass shatters at 0.02 MJ, a partition goes at
  0.15 MJ, a column at 2.5 MJ.
- **What falls**: connectivity from the ground. Anything not standing on something that stands falls
  as one rigid island. A floor bay with nothing under it may hang from a directly held neighbour for
  one hop.
- **What gives**: a top-down load pass.
  - Floors put their weight on the corner columns whose column line still reaches the ground. Columns
    pass it down.
  - Capacity is 2 x the intact load. A floor bay held on one edge only is a cantilever at 45 %.
  - An overloaded member creaks for 0.5-1.6 s (dust, a groan, the stress shown on the cockpit's
    target readout), then fails.
- **Falling islands** load the floors they land on at 2 x their weight, so storeys pancake. They
  shatter into the module's pre-fractured cells when they stop.
- **Fast**:
  - A solve is O(elements) for one building, on change only.
  - A break flips one bit in the building's element mask. The vertex shader drops that element's
    triangles, so no remesh and no new draw.
  - Pieces come from a fixed pool.
- **Deterministic** in `cb-core`: the same hits break the same elements at the same steps, and islands
  fall by a kinematic model in the core. Avian (`debris-physics`) only moves the loose pieces, which
  are presentation.
- **Mech contact**: a mech walking into a wall at over 2.5 m/s breaks what it touches (as today),
  now element by element, and slows by the mass it broke.

## Presented by

`docs/mockups/buildings.html`: click to fire, the demolition buttons and "Stress view". Shots
`hole`, `corner_stress`, `side_*`, `warehouse_drop` and `volley` in `docs/screenshots/buildings/`.

## Replaces

The cell grid of `building-destruction` (its requirement "Buildings break by cells" is not in
`openspec/specs/` yet). When this lands, `building-destruction` is archived as superseded and its tests
move here.

## Out of scope

Rubble on the ground (`rubble`), fire, rebar and cracked-but-standing states beyond the creak (a later
change, as THE FINALS' gradual destruction), destruction over the network.
