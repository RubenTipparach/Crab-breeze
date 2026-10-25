# Playtest: destructible buildings, 2026-10-24

The owner said "Go ahead" to playtesting on the Mac. This checks the building integration on
branch `building-research` (changes `structural-destruction`, `building-generation`,
`building-interiors`, `rubble`) in the running game.

- **Machine**: the owner's Mac, Apple M4 Max, release build, the window at 1920 x 1080.
- **Commit**: run from `cbba5c3` plus the fixes listed below, committed together with this report.
- **No OBS take and no Drive upload.** Window screenshots on this Mac came out black. macOS doesn't
  draw a window that is behind another, and the game was started from a terminal. Neither
  `screencapture` nor Bevy's window screenshot shows the game. The stills come from an offscreen
  image the pilot's eye renders into, as the bench does. A recorded take is still owed, on the
  Windows machine with OBS (section 9).

## Runs

1. **`--showcase fight`**: quit at startup with a wgpu validation error. The screens' cube faces
   (`screens_cube.rs`, from `main`) are created without `COPY_SRC` and then copied into the cube.
   Fixed: each face now allows `COPY_SRC`. This isn't building code; it was flagged to the owner.
2. **`--bench fight` with `CB_BENCH_STILLS=1`**: ran to `PLAYTEST_RESULT ok fight`. Every still of
   the tower beat is inside the cockpit, and the screen wall shows nothing of the world. That's the
   cockpit work in progress, not judged here. It couldn't show the buildings.
3. **`--stills` with `CB_STILLS_BUILDINGS=1`** (the capture instrument's new building variant):
   - The pilot stands in the street off an 18-storey building near the start (id 204).
   - Pressing F9 (the debug shell, 6 MJ) at the foot of each column on the facing side brings it
     down.
   - Run three times, tuning in between:

   | Try | What happened | Change |
   | --- | --- | --- |
   | a | 4 shells between the columns: 4 wall panels broke, nothing else (216/216 cells became 212) | The script aimed between columns: aim at each column's foot instead |
   | b | Only 2 elements broke: most F9 presses were lost | The script held F9 for one frame, and frames run faster than the 60 Hz simulation step: press it as the keyboard does (latched until the step takes it) |
   | c | The facing columns broke, the bays above them creaked and peeled off top to bottom: 216 cells became 144, the whole front row of 4 bays x 18 storeys | Dust thinned, falling parts cut per bay instead of 2 x 2 bays |

   - Between a and b, a linear blast falloff was tried. It blew a tank shell's hole three storeys
     tall, so the falloff went back to squared and columns were weakened to 2.0 MJ.

## What it showed (contact sheet `2026-10-24-buildings_frames.png`, try c)

| Still | Seen |
| --- | --- |
| Whole | The building from the street: concrete fins on the column lines, dark floor bands, the window tile |
| Hit | Shells at the facade's foot, dust, the first slabs coming down |
| Creak | Dust rolling out of the ground storey, slabs falling |
| Fall | Storeys coming down through the dust |
| Rubble (12 s) | The front row of bays gone from top to bottom: the floors stand open in a stack, side walls still up |
| Close | Debris chunks and the rubble mound (rubble colour) at the building's foot, the interior behind |

An earlier try (b, from the same spot) showed the open ground storey from 6 m away: columns,
white partitions and furniture inside, floor bands and fins of the storey above.

## Wrong or rough

- Collapse dust still hides most of the fall from 11 m. Thinner now, but it can go thinner again,
  or play the falling chunks first and the cloud on landing.
- Debris chunks are plain dark boxes in the building's tone, now 6 m. They want the Game debris
  pieces (biggest boxes only, broken edges) and the rubble texture.
- The first tries showed the pilot can stand inside a building when placed 45 m back (the street
  is 24 m). A building's box is invisible from inside, but its dressing isn't, so it draws as a
  lattice. Only the capture script placed him there; a walking pilot is pushed out of buildings.
- Not seen: pancaking from a cut high up, and walking on the rubble mound. Both are covered by
  core tests only.

## Fixes made in this playtest (committed with this report)

- `crates/cb-app/src/screens_cube.rs`: the face images allow `COPY_SRC`. The game quit at startup
  without it.
- `crates/cb-core/src/world/structure.rs`, `data/world/world.ron`: columns 2.0 MJ (were 2.5).
- `crates/cb-core/src/world/destruct.rs`: falling parts chunked per bay (6 m), not per 2 x 2 bays.
- `data/fx/fx.ron`, `crates/cb-app/src/buildings.rs`: collapse dust 50 puffs of 4 m for 5 s (was
  120 of 6 m for 9 s), scale at most 2.
- `crates/cb-app/src/capture.rs`: the building variant (`CB_STILLS_BUILDINGS=1`), drawn offscreen.
