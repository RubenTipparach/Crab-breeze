# Proposal: terrain-deformation

## Why

Owner, 2026-10-24: "Deformable terrain? Explosions can dig up some dirt." A battlefield should show
the battle: craters where shells landed, furrows where a mech fell, dirt thrown into the air.

## What changes

- A deformation layer over the terrain: a fine height delta (0.5 m grid) on the chunks near the
  action, added to the base heightfield. `height_at` reads base plus delta, so the pilot, the mech,
  water, trees and the drawn ground all agree (one ground query).
- Craters: an explosion of a given energy digs a bowl and raises a rim, with a size from data
  (`crater_radius_m = k * energy_mj^(1/3)`). Clamped so a crater never digs below a floor (bedrock
  at `max_dig_m`), and never deforms city pavement or roads (they crack and scorch instead).
- Dirt: a burst of dirt clods and dust (particles), thrown along the explosion normal; clods land
  as small debris that fade.
- Scorch: craters write the scorch channel `terrain-materials` splats.
- Deterministic: deformation is an ordered list of events (position, energy, id) applied in
  order; the same events give the same ground. Saved with the game.

## Out of scope

Digging tunnels, landslides, mechs digging on purpose, deformation of buildings (a separate
destruction change).
