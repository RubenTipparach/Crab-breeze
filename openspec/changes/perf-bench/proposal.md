# Proposal: perf-bench

## Why

Owner, 2026-10-24: "playtest to optimize and validate the game is built". On the owner's Mac a
windowed run is pinned to the display's 60 Hz (macOS ignores no-vsync), so frame cost cannot be
read from a windowed run, and Bevy records no GPU timestamps on Metal. A benchmark has to measure
the real cost of a 1920 x 1080 frame, beat by beat, through the same scripted take a playtest plays.

## What changes

- `crab-breeze --bench`: plays the showcase take with the pilot's camera rendering into an
  offscreen 1920 x 1080 image, so no frame waits for a display; every frame's real time is kept per
  beat, and the game prints `BENCH <beat> frames p50 p95 p99 max` lines and a summary, then a JSON
  report in `output/bench/`.
- Each expensive system has a switch (environment variables `CB_NO_TREES`, `CB_NO_SHADOWS`,
  `CB_NO_CLOUDS`, `CB_NO_PARTICLES`), so its cost is measured as a variant in the same sitting.
- Reports go in `docs/benchmarks/<date>-<name>/report.md` (`perf-measure` skill).

## Out of scope

Tracy captures (a later step if a cost needs to be located inside a system).
