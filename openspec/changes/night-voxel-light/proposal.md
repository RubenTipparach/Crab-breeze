# Proposal: night-voxel-light

## Why

Owner, 2026-10-29: "at night time, can we do voxel lighting that bakes vertex colors instead of fake glass
lighting?"

At night today (`day-night-cycle`, `docs/screenshots/2026-10-28-city-lights-at-night.png`) the city is black
walls with bright window rectangles on them:
- A lit window is emission drawn in the glass and nothing else. It lights no wall, no street and no car.
- A street lamp lights its surroundings only while it is one of the 96 nearest within 250 m, as a real spot
  light. Past that it is a glowing head and a glow sprite that light nothing. From the air the streets are
  dotted lines over black ground.

The light has a source but lands on nothing, which is the fake look the owner means.

## What changes

The light from every street lamp and every lit window is baked into the city as it streams in, by a voxel flood
fill, and stored on each vertex of the static world:
- **Voxelise.** Each city is turned into 2 m voxels: building volumes and the ground are solid, the rest is air.
- **Flood fill.** Light spreads out from every lamp head and every lit facade through the air. It turns corners
  and pools in the streets, and it stops at walls, so it never leaks through a building.
- **Bake.** Every vertex of the buildings, houses, ground, paving, poles and rocks reads the light at its own
  position. The reading is stored as two numbers in a new vertex attribute: lamp light and window light.
- **Draw.** Each surface's shader adds that light to its colour after the PBR lighting. The lamp half follows the
  lamps' dusk switch; the window half follows the share of windows lit at the hour (`WindowLights::lit_share`).

What it gives the player:
- Orange pools under every lamp in every city, from the street and from the air.
- Facades washed by the light of their own windows and of the windows across the street.
- Light that stops at the building that blocks it.
- No new draws and no per-frame lighting cost for any of it.

Two parts are owned by `day-night-cycle` and change with this one:
- **Tier 1, the real spot lights.** The pool keeps lighting moving things (cars, people, the mech) and gives wet
  road its glints. It shrinks to `real_lamps` 32 within 120 m (survey N9, provisional).
- **Survey N4's "pools only within 250 m".** Pools become everywhere, because they are baked (survey N11,
  provisional).

## Out of scope

- Car headlights, the player's vehicle lamps and anything else that moves. These stay real lights, as in
  `day-night-cycle` section 7.
- Daylight bounce and sky occlusion. The same voxels could carry them later, as their own change.
- Interiors behind lit windows (`building-interiors`).
- Far facades (the far building impostors from `building-type-lods`). They keep their own window rule.
