# Baseline: the world take and the city fight, 2026-10-24

`crab-breeze --bench world|fight` (openspec change `perf-bench`): the scripted take plays with the
pilot's camera rendering into an offscreen 1920 x 1080 image, so no frame waits for the display
(the Mac's windowed runs are pinned to vsync). First 10 s thrown away. Release build, nothing else
running, Apple M4 Max (40-core GPU), 64 GB, macOS. One run each; repeats and old-against-new
interleaving are still to do, so differences under about 0.3 ms are not results.

## Findings

- **Well inside the target on this Mac**: the world take p95 3.0 ms, p99 3.3 ms; the city fight
  p95 4.1 ms, p99 5.1 ms (target p95 8.3 ms, p99 16.7 ms). The owner's target machine is now an
  RTX 3070 PC; it has not been measured.
- **The fight costs about 1 ms more at p95** than the street (particles, tracers, Avian debris,
  building remeshes).
- **Shadows cost about 0.5-0.7 ms** (world take p50 2.52 with, 1.83 ms without).
- **Hitches**: max frames of 15-35 ms appear in most beats. Suspects, to be confirmed with Tracy or
  switch-off variants: crater tile remeshing on the main thread, building remeshing on the main
  thread, first use of a material or pipeline (shader compile), and Avian trimesh rebuilds when a
  crater re-meshes a terrain chunk.

## Limits

- One run per variant; no spread measured yet.
- The offscreen stills the bench saves stop updating late in a run (the frame-time numbers do not
  depend on them; the windowed recording shows the take moving throughout). To investigate.
- Not the owner's RTX 3070 machine.

## Results (ms)

| Variant | Frames | p50 | p95 | p99 | max |
| --- | --- | --- | --- | --- | --- |
| world take | 78 310 | 2.52 | 2.96 | 3.30 | 29.29 |
| world take, `CB_NO_SHADOWS=1` | 103 865 | 1.83 | 2.45 | 2.75 | 14.74 |
| city fight | 33 969 | 2.87 | 4.14 | 5.13 | 22.79 |

| World beat | p50 | p95 | p99 | max |
| --- | --- | --- | --- | --- |
| board | 2.58 | 2.93 | 3.65 | 29.18 |
| street | 2.67 | 2.98 | 3.69 | 17.39 |
| jets | 2.43 | 2.91 | 3.21 | 29.29 |
| descent | 2.24 | 2.69 | 2.92 | 25.42 |
| wade | 2.31 | 2.80 | 3.01 | 8.00 |
| wood | 2.76 | 3.06 | 3.68 | 27.62 |
| shells | 2.57 | 2.97 | 3.65 | 6.31 |
| damage | 2.70 | 3.02 | 3.67 | 16.94 |

JSON: `world.json`, `world-no-shadows.json`, `fight.json` beside this report.
