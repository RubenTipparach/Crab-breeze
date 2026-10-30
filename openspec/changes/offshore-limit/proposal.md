# Proposal: offshore-limit

## Why

The owner, 2026-10-30: "Make sure the mech can never leave too far off the coast or it will automatically mission
fail and respawn the player in a city. Even in free roam", then "Offshore limit: 15 km".

The island's coast lies 9 to 15 km from its centre and the world ends 20.48 km out, so 15 km of sea fits only toward
the corners. The owner chose (plan questions, 2026-10-30) "15 km, edge clamps": the limit is 15 km off the coast or
1 km inside the world's edge, whichever comes first. Nothing stops a mech flying or skating into the void today, and
the game has no respawn at all.

## What changes

- **The limit**: a mech (or the pilot in any vehicle) past `min(coast + 15 km, world edge - 1 km)` along its bearing
  loses the mission.
- **A warning first**: from 2 km before the limit a cockpit alarm sounds and the HUD's mission line reads
  `RETURN TO COAST  n.n km`.
- **A new loss**, `Loss::OutOfBounds`, in missions and in free roam alike.
- **Respawn**: the pilot comes back seated in their mech, repaired, at the nearest city's drop site, after the
  mission-failed screen. The game's first respawn, reusable by any later loss.

## Impact

- `cb_core::{mission, missions}`, a new `cb_core::bounds`, a new `respawn` in `cb-app`, the HUD mission line, the
  alarm sound (`cockpit-alarms` provides the sound; this change only plays it).
