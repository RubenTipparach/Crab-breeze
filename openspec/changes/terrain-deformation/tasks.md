# Tasks: terrain-deformation

- [x] 1.1 Deformation tiles and `height_at` with the delta (test: `height_at_includes_the_crater`)
- [x] 1.2 Crater shape from energy, depth clamp, roads protected (tests: `a_crater_digs_a_bowl_and_raises_a_rim`, `craters_never_dig_below_the_floor`, `a_road_is_scorched_not_dug`)
- [x] 1.3 Ordered events, determinism (test: `the_same_explosions_give_the_same_ground`)
- [x] 2.1 Patch remesh off the main thread, dirt and dust particles, scorch channel
- [x] 2.2 A debug key that drops a shell where the pilot looks, until weapons exist
- [x] 2.3 Measure a 50-crater barrage; playtest
- [x] 3.1 Fix: a rocket hit made a patch of ground vanish for seconds (owner, 2026-10-26: "I fire rockets at the ground ant patch of ground disappears.for a few seconds then is redrawn"). The chunk's holed mesh went up before the crater tile that fills the hole, and a tile dug again had its meshing job cancelled, so under a barrage the tile never finished and the hole stayed open until the firing stopped. Now the holed chunk and its new tiles go up on the same frame (`deform::patch_swap`) and a tile's job is never cancelled (design, "The patch going missing")
