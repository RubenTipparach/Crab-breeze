# Proposal: building-interiors

## Why

Owner, 2026-10-24: "research detailed building generation, interiors and floorplans, furniture etc".
A wall blown open by a 120 mm shell should show an office or a flat behind it, and the pilot walks
the city on foot (the law: first person throughout). Empty boxes break both.

## What changes

- **Floorplans per storey**, generated in `cb-core` from the building's footprint and the storey's
  use (retail, lobby, office, flat, industrial):
  - A core (stairs, two lifts) in one cell every storey shares, so the stairs line up.
  - A double-loaded corridor through the core along the long axis. Single-loaded, with the stair at
    the end away from the street, when the plan is under 13 m deep or too small for a core.
  - Rooms fill the bands either side, split on the 3 m facade bay lines. Flats subdivide into
    kitchen, bath, living room and bedroom.
  - Every room has a door on a corridor or into another room of its flat. Partitions are elements
    (`structural-destruction`) resting on the floor below.
- **Furniture placed by rules** (Star Crew's `crew_rooms.py` checks, in Rust):
  - Each room type lists items in order with a count range.
  - Items anchor to a wall, the room's centre or a grid (desk pods, warehouse racks), with a clearance
    strip in front.
  - They keep out of door zones and column footprints. Tall items never stand in front of a window.
  - 1980s kit: wood-panel desks with beige CRTs, filing cabinets, sofas, CRT televisions, kitchen runs,
    pallet racks, lathes.
- **Walkable**: floors, partitions, stairs and doors are collision for the pilot on foot. Furniture
  can be pushed through by the mech and is debris when hit.
- **Near and far**: real interiors are built within the interior radius (150 m) of the pilot or the
  mech's cockpit, and in any breached building. Beyond that, windows use an interior-mapping shader
  whose room atlas tile comes from the same floorplan.

## Presented by

`docs/mockups/buildings.html`: the storey slider, "Skin off" and the floorplan panel.

## Out of scope

People and crowds, lighting fixtures and night interiors, doors that open, lifts that move.
