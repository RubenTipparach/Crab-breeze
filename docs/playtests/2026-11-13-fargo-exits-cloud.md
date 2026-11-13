# 2026-11-13: the highway through the capital, its exits and the downtown shapes (cloud stills)

- Commit: b63071a0 on `claude/great-keller-ng77hm` (PR #116), plus the stills below.
- Machine: a Linux cloud session, headless (lavapipe software Vulkan under Xvfb, about 1 frame a second). Not a
  playtest on the owner's machine and no performance numbers: the frame counter in the corner is the software renderer's.
- Drive link: none (a cloud session cannot record).
- Change: openspec `city-districts`, design sections 5 (the highway through the city) and 6 (downtown shapes).

| Beat | Outcome |
| --- | --- |
| The capital's trunk from the air (`fargo-air-trunk.jpg`) | the viaduct crosses the city along its row of car parks; the two highways climb onto it; its exits' ramps show |
| An exit from 140 m (`fargo-exit.jpg`) | piers under the viaduct, four ramps down to the frontage streets |
| Under the viaduct (`fargo-under.jpg`) | the slab's underside and a pier with its cap beam over the street; the ramps' fill walls either side, their barriers following the grade with no steps (the stepped barriers of the first shoot are fixed) |
| A ramp's landing (`fargo-ramp.jpg`) | traffic waits at the light where the ramp meets the street |
| A main-street downtown (`fargo-main-street.jpg`) and a two-core one (`fargo-two-cores.jpg`) | the shapes show; the two-core city's towers stand in two clusters in a ring of flats |

Found and fixed on the way: the deck tiles were never drawn under Bevy's `VisibilityRange` (their vertices are in
world space, their origin at 0); the game now chooses each tile's level from the eye (`DeckLod`).

Stills: `docs/screenshots/city-districts/game/` (README there).
