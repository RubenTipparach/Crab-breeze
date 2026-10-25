# Proposal: particle-steps

## Why

In the fight take on the owner's RTX 3070 (on screen, vsync off), 5.9 % of 60 Hz refreshes were
missed with particles on and 0.8 % with them off (`CB_NO_PARTICLES=1`, 2026-10-25). The cost is in
spikes, not the average: p95 19.8 ms against 14.2 ms. A burst's particle count is its layer's count
times the effect's density, cut to the room left under the particle budget, and every distinct count
is its own GPU effect, built the first time it plays. Footsteps, foot spray and skate spray set their
density from speed, so a fight kept building new effects mid-combat. The loading-screen warm-up
played each recipe once at density 1 and 220 m away, where small bursts are culled, so many were
never built before play.

## What changes

- A burst spawns a whole number of steps of a quarter of its layer's count (`burst_count`): at most
  a handful of sizes per layer.
- The warm-up plays every recipe at every step from a quarter to twice its count, with the particle
  budget and the range cull lifted behind the loading screen, so every size is built before play.
- What the player sees: bursts a little larger or smaller than before (to the nearest quarter of
  their full count), and no hitch at a recipe's first use.
