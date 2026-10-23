---
name: perf-measure
description: Measure Crab Breeze's frame time and back a performance claim the repository's way. Release build, real time, windowed, nothing else running; fixed scripted scenes (the playtest scripts); old build against new interleaved in one sitting; p50, p95, p99 and the spread between repeats; Bevy diagnostics and Tracy for where the time goes; a checked-in report. Use for any performance question or change that can move frame time, memory or load time ("is it faster", "did this regress", "what does the city cost", "measure fps", "benchmark", "profile", "why is it laggy", "optimise").
metadata:
  author: Crab Breeze (Claude Code), after Pale Blue Dot's and Mining Mike's perf-measure
  version: "1.0"
---

# Measure performance

Crab Breeze is the most detailed mech simulation we can make, so it will be expensive, and it must
stay playable. Every rule below was learned by getting a number wrong in Pale Blue Dot or Mining
Mike. CLAUDE.md section 7 states them as rules; this is how to follow them.

## Targets

Until the owner sets others (survey question `T3`): on the owner's most powerful machine at
1920 x 1080, **p95 frame time within 8.3 ms (120 fps) and p99 within 16.7 ms (60 fps)** in the
cockpit, striding through the densest city block a scene has. A miss is reported as **over**.

## The rules for a claim

1. **The release build** (`cargo build --release`). The dev profile is for iterating, never for
   numbers. Measure with the `profiling` cargo profile when it exists (release with debug info).
2. **Real time, windowed, nothing else running.** No build, no second game, no recording, no OBS.
   One game per device.
3. **The same scene, scripted.** A scene is a playtest script (`data/playtests/`) played by the
   scripted pilot, with a fixed seed and the first 10 seconds thrown away. Never a hand-flown run.
4. **Old against new in the same sitting, interleaved** (A, B, B, A), never against a number
   from another day. Keep the old binary; an old binary runs with the assets from its own commit.
5. **A run that did not do the hard part is invalid**: a stride that never reached the block, a
   fight that never started. Mark it and leave it out.
6. **Report p50, p95, p99, max and the spread between repeats.** A difference smaller than the
   spread is not a result. Trading a spike for throughput is not a fix.
7. **Say what was measured**: machine (CPU, GPU, memory), resolution, build, commit, repeats.
8. **Never call an unmeasured design a speedup.** A design says what it expects to cost; the
   measurement says what it does.
9. **A cloud session measures nothing.** It has no GPU (lavapipe renders on the CPU). It says in
   the PR that frame cost was not measured, and asks a connected session on the owner's hardware
   to measure (the `playtest` skill, "Where it runs").

## Find where the time goes

- **Bevy diagnostics**: `FrameTimeDiagnosticsPlugin`, `EntityCountDiagnosticsPlugin` and
  `SystemInformationDiagnosticsPlugin`, logged by `LogDiagnosticsPlugin` or drawn by the in-game
  debug overlay (an F3 toggle that mirrors a button in the debug menu).
- **Tracy**: build with Bevy's `trace_tracy` feature and connect the Tracy profiler. It shows each
  system, the render graph's passes and GPU timings. The tool the measurement settles on (Tracy or
  `chrome://tracing` through `trace_chrome`) is recorded in `playtest-pipeline`'s design.
- **Switch it off and measure again.** Each expensive system gets a switch (an environment variable
  such as `CB_NO_CROWDS=1`, or a debug menu toggle) so its cost is measured as a variant in the
  same sitting.
- **Usual suspects in a detailed city**: draw calls (merge static geometry, instancing for repeated
  props and crowds), shadow cascades, overdraw on glass and particles, skinned meshes in crowds
  (LODs and animation LOD), asset streaming hitches (load off the main thread, never on a stride),
  and change detection on large queries (query only the components a system needs).

## Report it

1. Write `docs/benchmarks/<YYYY-MM-DD>-<name>/report.md`: `## Findings`, `## Limits`, then the
   table of variants with p50, p95, p99, max and spread.
2. When the result changes the picture (a new baseline, a regression, a cost that moved), publish
   it as an artifact page and link it from the report.
3. Commit the report with the change that moved the numbers.
