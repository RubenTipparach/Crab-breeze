# Proposal: night-frame

## Why

The owner (2026-10-29, after free roam at midnight): "Night time was really slow. Can you start running optimizations
for night time?". The run's own numbers (bin/plain17.exe, commit b49fa1e1, Sapper, free roam, the clock set to 00:00):

- After the clock went to midnight, 2 271 frames went over the hitch line against 538 before it, the median of those
  22.3 ms against 18.1 ms.
- The perf HUD's lines at night: 44 to 67 fps, 15 to 22.5 ms a frame, while the GPU's own time was 0.9 to 1.7 ms and
  the main thread's systems 6 to 11 ms. So the night's frame is lost on the CPU, outside the game's systems: in
  Bevy's render world (extracting, preparing and queueing the frame).

What the night adds: up to `real_lamps` (96) street lamps within `real_m` (250 m) made real spot lights, the lit
windows, the moon's shadows in place of the sun's, and car lights. Bevy assigns every point and spot light to the
view's clusters on the CPU each frame; `data/fx/fx.ron` already records that 24 effect lights cost about 1 ms of it.

## What changes

1. Measure first (this proposal's instruments): noon against midnight on the same scripted spot, then at midnight
   each suspect switched off alone: the lamps' real lights (`CB_NO_LAMPS=1`, new), the lit windows
   (`CB_NO_WINDOW_LIGHTS=1`, new), the moon's shadows (`CB_NO_SHADOWS=1`), effect lights (`CB_NO_FX_LIGHTS=1`).
2. Fix the largest, by its numbers, in this change's design; measure again old against new.

## Impact

`crates/cb-app/src/lamps.rs` and `buildings.rs` gain an instrument switch each (no change in play). The fix itself is
designed once the numbers are in.
