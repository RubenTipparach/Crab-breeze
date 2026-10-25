# Proposal: rocket-plumes

## Why

Owner, 2026-10-25: "rocket plumes arent convincing right now". A rocket in flight drops one soft-disc trail puff
every 3 m and a 0.06 s flame dot behind it: a dotted line of grey balls with a flicker. A mech's jets
(`jet_flame`, 60 bursts a second) are a spray of orange discs under each nozzle. Neither has the bright, hard exhaust
core, the hot glow on what is around it, or the thick, continuous smoke a rocket motor leaves.

## What changes

- **An exhaust plume mesh** behind every rocket and under every firing jet nozzle: a cone drawn additively with a
  scrolling-noise shader, a white-hot core at the nozzle going yellow then orange, with shock diamonds on a jet, its
  length from the thrust.
- **A dense smoke trail**: a rocket lays a puff every 0.6 m (now 3 m), from the `fx-sprites` smoke sheet, small and
  bright where it leaves the nozzle, growing to 4 m and greying over 8 s, drifting with the wind. The first 20 m glow
  from the exhaust.
- **Light**: each rocket carries a small moving point light (within the existing cap of 8), so it lights the ground
  and buildings it passes.
- **Jets near the ground** blow dust out in a ring under the mech when within 15 m of the ground.

## Out of scope

Heat shimmer (a screen-space distortion pass), sound, the missile's own model.
