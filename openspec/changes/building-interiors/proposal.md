# Proposal: building-interiors

## Why

Owner, 2026-10-24: "research detailed building generation, interiors and floorplans, furniture etc".
A wall blown open by a 120 mm shell should show an office or a flat behind it, and the pilot walks
the city on foot (the law: first person throughout). Empty boxes break both.

Owner, 2026-10-25: "each building needs doors and staairs that i as a player can walk into them".
The floorplans already had a stair core and a door on every room, but only a damaged building drew
them, the stair was an empty concrete box, and the pilot met every intact building as a solid
block. The street has to lead in through a door, and the stair has to climb.

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
- **A street entrance on every building** (owner, 2026-10-25): a door opening in the ground-floor
  facade on the side that faces the street (the facade nearest the block's kerb with nothing built
  in front of it; an inner lot's door opens onto the alley between lots, which reaches the
  pavement). It leads into the corridor, or into a lobby room whose door onto the corridor lines up
  with it, and on through the corridor to the stair core. A walk-up gets its front door up a stone
  stoop, an office or tower its pair of lobby doors, a warehouse a steel personnel door (its roller
  doors stay shut: there is no hall behind them to open into yet).
- **A stair you can climb** in the core, from the ground floor to the top storey: a dog-leg of two
  flights a storey, treads at a walkable rise and run (about 0.175 m and 0.26 m), a half landing,
  a concrete spine wall between the flights and a balustrade on the open side, and a stairwell cut
  in every floor above it.
- **Walkable**: floors, partitions, stairs, the entrance and its door leaves, and furniture over
  the step height are collision for the pilot on foot, built near the pilot on a worker thread, for
  intact and breached buildings alike, and only while the pilot is on foot (owner, 2026-10-26: "when
  on foot I should be able to wlak into the buildings, I think you would generate detailed colliders
  for buildings only when the player is on foot"); in the mech, buildings stay coarse. The
  street door swings open as the pilot walks into it and shuts behind them (survey W30, owner
  2026-10-26: "A door that swings open as you walk into it, and shuts behind you"). Furniture
  can be pushed through by the mech and is debris when hit.
- **Near and far**: real interiors are built within the interior radius (150 m) of the pilot or the
  mech's cockpit, and in any breached building. Beyond that, windows use an interior-mapping shader
  whose room atlas tile comes from the same floorplan.

## Presented by

`docs/mockups/buildings.html`: the storey slider, "Skin off" and the floorplan panel.

## Out of scope

People and crowds, lighting fixtures and night interiors, lifts that move, room doors that swing
(only the street door does: survey W30), roller doors that open (survey W31: they stay shut until
warehouses have a hall inside).
