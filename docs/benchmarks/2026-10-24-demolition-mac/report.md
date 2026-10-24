# Buildings under fire on the Mac, 2026-10-24

Two takes on the owner's Mac (Apple M4 Max, Metal), release builds, run with
`tools/perf/perf_suite.py`, windowed at 1920 x 1080 on the 120 Hz built-in display, nothing else
running. **This is not the RTX 3070 target.**

- **Demolition** (`--bench demolition`, revision `c85d1de`): on foot, shell every column on one face
  of the building nearest the start, watch it fall and settle, walk up to the rubble. 3 runs.
- **City combat** (`--bench defence`, `building-research` with main and `pilot-controls` merged in,
  raiders spawning inside the city): board, then fight the first wave (10 raiders and 6 vehicles)
  in the city's streets. 2 runs, 12 and 7 minutes of fighting.

## How to read the numbers

The window is held to the display's refresh, 120 Hz, so a frame either makes the 8.3 ms slot or
waits for the next at 16.7 ms. A p50 of 16.6 ms therefore means most frames cost between 8.3 and
16.7 ms. (An earlier version of this report said the Mac was capped at 60 fps; the combat runs,
with 29 % of their frames at 5-10 ms, show it is not.)

## Findings

0. **60 fps: yes on average, not on every frame.** City combat averaged 66 fps and the demolition
   59.9 fps. The median frame is 16.2-16.7 ms (60 fps). The slowest 5 % of frames take about 18 ms
   (55 fps), the slowest 1 % 21-27 ms (37-48 fps), and there are freezes: one a run of about 80 ms
   while shelling (once 773 ms), and 21 over 50 ms across the two combat runs. The 120 fps target
   is missed: only 1 % of demolition frames and 29 % of combat frames are under 8.3 ms.
1. **The demolition misses 120 fps almost every frame.** Only 1 % of its frames made the 8.3 ms
   slot. p50 16.65 ms, p95 18.65 ms, p99 20.90 ms (spread 0.12 ms over 3 runs), peak memory
   1222 MB. Over the p95 budget (8.3 ms); the p99 budget (16.7 ms) is missed by 4 ms.
2. **The fall itself is steady**: p95 18.48 ms, p99 19.53 ms, worst frame 23.3 ms over 780 frames.
   The structure step and chunk spawning never logged a slow frame.
3. **Shelling a building freezes the game once**: one frame a run of 78-86 ms, once 773 ms. No
   watched system was slow at that moment; a first-use shader compile or the debris bodies are
   the suspects. Not proven.
4. **City combat sits on the 120 fps line.** 29 % of frames made 8.3 ms, rising from about 20 % in
   the first minutes to 45-50 % later. p50 16.34 ms, p95 18.07 ms, p99 26.75 ms (spread 0.17 ms),
   21 freezes over 50 ms in the two runs, worst 280 ms, peak memory 1459 MB.
5. **`redraw_buildings` is the cost in combat.** Every hit rebuilds that building's mesh on the
   main thread: 1899 slow frames in run 0 (median 8.6 ms, p95 21.4, worst 33.9 ms) and 1324 in
   run 1 (median 10.8, p95 25.6, worst 49.1 ms). That alone fills the 8.3 ms budget.
6. **`dress_near_player` is slow while the city loads** (about 100 slow frames a run, worst 29 ms),
   rarely after.
7. **The building falls differently every run**: 176, 144 and 208 of 216 cells standing, same seed.
   The collapse depends on frame timing, against the determinism rule (CLAUDE.md section 6.3).
8. **The scripted pilot loses in the city.** Run 0: the 600 s clock ran out with 9 of 16 enemies
   left; run 1: the pilot's mech was destroyed. The bot's wall avoidance and unsticking are a
   pending desktop PR; city 1 took 6-8 % damage.

| take | beat | p50 ms | p95 ms | p99 ms | worst ms |
| --- | --- | ---: | ---: | ---: | ---: |
| demolition | whole building | 16.67 | 21.26 | 24.37 | 25.9 |
| demolition | shelling | 16.55 | 20.94 | 46.91 | 772.9 |
| demolition | fall | 16.64 | 18.48 | 19.53 | 23.3 |
| demolition | close | 16.62 | 18.08 | 19.04 | 19.7 |
| city combat | fight, run 0 | 16.16 | 18.08 | 22.36 | 93.3 |
| city combat | fight, run 1 | 16.34 | 17.96 | 27.21 | 126.6 |

## What to fix, in order

1. `redraw_buildings`: rebuild a hit building's mesh off the main thread (the craters already
   mesh off-thread on main), or rebuild only the damaged band.
2. The shelling freeze: find it with a trace on a run with no OBS.
3. Make the collapse step-deterministic, so builds can be compared on the same fall.
4. Then measure on the RTX 3070, windowed: `python3 tools/perf/perf_suite.py --scenarios demolition,defence --repeats 3`.

## Limits

- Apple M4 Max, not the RTX 3070.
- A comparison with shadows, grass and water effects switched off was spoiled by another
  session's `cargo test` taking every CPU core.
- Videos come from separate runs and carry no numbers.
