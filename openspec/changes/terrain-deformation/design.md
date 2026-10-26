# Design: terrain-deformation

## 1. Layer

A sparse map of 64 m deformation tiles, each a 129 x 129 grid of f32 height deltas (0.5 m spacing,
66 kB). A tile is created on first hit. `height_at` = base triangle height + bilinear delta. The
terrain mesh for a touched chunk is rebuilt at 0.5 m resolution inside touched tiles (a finer
patch stitched into the 8 m chunk with skirts), off the main thread, ready the next frame.

## 2. Crater shape

Radius `R = 2.2 m * E^(1/3)` with `E` in megajoules (a 120 mm shell, about 4 MJ, gives 3.5 m),
depth `0.35 R`, rim height `0.08 R` out to `1.6 R`. Profile: a cosine bowl plus a Gaussian rim.
Overlapping craters add, clamped to `max_dig_m` (data, 6 m) below the original ground.

## 3. Cost

A crater touches at most four tiles: about 2 x 10^4 cells, under 0.3 ms on one core, plus the
patch remesh (async). Memory: 66 kB per touched tile; a heavy fight touching 200 tiles is 13 MB.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| W7 | How much can the ground be dug up? | craters up to 6 m deep, roads and pavement crack and scorch but keep their level | provisional (survey W7) |

## As built (2026-10-24)

- Tuning is the `craters:` section of `data/world/world.ron`. Shells come from the F9 test key or the scripted pilot (`DebugCommand::Shell`).
- The patch remesh runs on the main thread (one tile, 16 k quads, a few ms); off-thread is still to do.

## The patch going missing (2026-10-26)

Owner, 2026-10-26: "I fire rockets at the ground ant patch of ground disappears.for a few seconds then is redrawn",
then, on whether to keep the system: "We might use deformation again in the future so don't remove the system. I'm have
actually leave it in, just fix the patch going missing". Deformation stays as built; the redraw was wrong in two ways.

1. **The hole went up before what fills it.** A blast marks its chunk stale; the chunk is rebuilt (off the main
   thread) with a hole where each crater tile lies, and the tile is meshed in its own job. Whichever finished first
   went on screen. The chunk's job is the smaller (64 x 64 quads against the tile's 128 x 128) and both wait in the
   worker pool behind every streaming job (grass, trees, dressing), so the hole could go up first and the ground under
   the crater stood open until the tile's job came back.
2. **A barrage kept the tile from ever finishing.** A tile dug again while being meshed dropped its job (dropping a
   Bevy task cancels it) and started over. Rockets landing on one spot every fraction of a second cancelled the job
   each time, so the hole stayed open until the firing stopped, then the tile came back: "a few seconds then is
   redrawn".

The fix, in `ground.rs`, with the rule in `cb_core::world::deform::patch_swap`:

- A chunk's finished mesh waits beside the one on screen, with the list of tiles it leaves holes for. It goes up only
  when every one of those tiles is on screen already or has its mesh finished, and those tiles go up on the same
  frame. Until then the old chunk (no hole, no crater) stays drawn.
- A finished tile already on screen (a crater dug again) is swapped at once: it covers the same hole.
- A tile dug again while being meshed is marked and meshed once more when the job in hand is done. No job is ever
  cancelled, so a barrage shows its craters as they are dug, a job's length behind.
- The blast itself does no meshing on the main thread; its cost on the frame is digging the deltas and marking the
  chunk and collider stale: 0.03 ms a blast on the M4 Max (`explode: dig` section, two blasts of 6 and 25 MJ). The
  398 ms the desktop's trace put on `blast::explode` was not the dig.
- Measured on the M4 Max (`CB_STILLS_FX=soil`, two blasts on open soil): both chunks went up 71 ms after the blasts,
  each with its new tile on the same frame (the `craters:` log line). What the frames after a first blast do cost is
  the effect's pipelines compiling (26 to 28 ms, two frames, once a run) and the new meshes' upload (about 10 ms a
  frame for six frames, `allocate_and_free_meshes`).
