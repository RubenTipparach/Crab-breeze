# Fargo-like cities in game: cloud stills, 2026-11-04

- **Change:** openspec `city-districts`, design section 4, step 1 (the Fargo-like town on today's grid).
- **Commit:** the head of `claude/great-keller-ng77hm` at this report.
- **Machine:** Linux cloud container, 4 cores, no GPU: lavapipe under Xvfb. This is not a playtest. Cloud sessions
  cannot record and measure nothing (CLAUDE.md sections 7 and 9); the owner's machine plays and records it.

## What was looked at

Six named views of the capital (`fargo-*`), held 50 s each so the near models stream in:
`docs/screenshots/city-districts/game/`.

| View | Outcome |
| --- | --- |
| Air | Towers at the middle, flats round them, house blocks to the edge, parks with trees: the Fargo plan |
| Houses | Two rows of houses back to back, front yards, drives to the kerb, a pavement ring, the ground's grass inside |
| Street | At eye height: houses along the pavement, people walking, parked cars, the strip's businesses opposite |
| Strip | Businesses side by side, each striped car park on the worn asphalt, paler than the dark streets |
| Downtown | Towers and walk-up flats |
| Highway | The highway on Concrete047A, clearly apart from the streets |

## Found and fixed during the run

- At the default 8 s settle the houses showed as their plain far shells: the stills now settle 50 s.
- Car parks tinted lighter on the suburbs' asphalt still read as dark as the streets: they are now their own mesh on the
  worn asphalt set (`Surface::Lot`).

## Not yet

Step 2 (exits: layered roads, ramps, bridges, the drive check in `cb-core`) and step 3 (drifting grids) are designed,
not built. A playtest on the owner's machine (walk the capital in the mech, drive its streets) is the sign-off.
