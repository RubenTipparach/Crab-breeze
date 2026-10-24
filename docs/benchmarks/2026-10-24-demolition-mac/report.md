# Demolition benchmark on the Mac, 2026-10-24

`--bench demolition` (structural-destruction task 2.4): on foot, shell every column on one face of
the building nearest the start, watch it fall and settle, walk up to the rubble. Revision `c85d1de`
(`building-research`), release build, run with `tools/perf/perf_suite.py`.

**This is the owner's Mac (Apple M4 Max, Metal), not the RTX 3070 target.** It also turned out to be
held to 60 fps for the whole sitting (below), so it cannot say whether the buildings fit the 8.3 ms
budget. It does say where the freezes and slow systems are.

## Findings

1. **The game held about 60 fps through the whole take, the fall included.** Windowed, 3 valid runs:
   p50 16.65 ms, p95 18.65 ms, p99 20.90 ms, p95 spread 0.12 ms between runs, peak memory 1222 MB.
   The fall beat alone: p50 16.64, p95 18.48, p99 19.53, worst frame 23.3 ms.
2. **Every frame over 50 ms is in the shelling beat**: one a run, 78-86 ms, and once 773 ms (run 0).
   No system the perf HUD watches was slow at those moments (`step_structures` and `spawn_chunks`
   never logged a slow line), so the freeze is outside them: a first-use pipeline compile or the
   rubble's physics bodies are the suspects. Not proven.
3. **`dress_near_player` is the one slow building system.** Over the 3 windowed runs it logged 164
   slow frames (median 7.3 ms, worst 29.0 ms); 130 of them while the world loads before the take,
   34 during it (worst 15.4 ms). `redraw_buildings` logged one slow frame a run (5.1-7.7 ms).
4. **The demolition does not end the same way twice.** Same seed, same script, three outcomes: 176,
   144 and 208 of 216 cells left standing (`PLAYTEST_RESULT`). The collapse depends on frame timing,
   which breaks the determinism rule (CLAUDE.md section 6.3) and makes beat-by-beat comparisons
   between builds unreliable until it is fixed.

| beat | p50 ms | p95 ms | p99 ms | worst ms (any run) |
| --- | ---: | ---: | ---: | ---: |
| whole building | 16.67 | 21.26 | 24.37 | 25.9 |
| shelling | 16.55 | 20.94 | 46.91 | 772.9 |
| fall | 16.64 | 18.48 | 19.53 | 23.3 |
| close (walk up to the rubble) | 16.62 | 18.08 | 19.04 | 19.7 |

Windowed, median over 3 runs, first 10 s thrown away. Offscreen runs (pilot's view rendered to an
image, no vsync) gave the same picture: p50 16.64, p95 18.86, p99 21.83 ms.

## Limits

- **The Mac was capped at 60 fps for the whole sitting.** Main's own `world` take, which measured
  2.46 ms p50 offscreen earlier today, measured 16.67 ms p50 right after these runs. Every build and
  every switch lands on 16.6 ms, so this sitting can't measure cost under 16.7 ms. Likely causes: the
  4K display at 60 Hz, or the display being off or locked (it was black when checked at 17:10).
- **The 8.3 ms verdict needs the RTX 3070.** Run the same suite there, windowed:
  `python3 tools/perf/perf_suite.py --scenarios demolition --repeats 3 --windowed`.
- A comparison with shadows, grass and water effects switched off was spoiled: another session's
  `cargo test` took every CPU core partway through. Its first runs showed no change in p50.
- The plain run asked for was not done separately: the suite's own run logs are the same command
  and carry the `slow:` lines counted above.
