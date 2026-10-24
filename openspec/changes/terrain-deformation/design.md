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
