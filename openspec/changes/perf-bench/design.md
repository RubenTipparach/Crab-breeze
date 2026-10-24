# Design: perf-bench

- The window stays (small, 640 x 360) for input and events; the one camera targets an offscreen
  `Image` 1920 x 1080 (Rgba8UnormSrgb), so the frame loop runs unthrottled.
- Frame time is `Time<Real>` delta per frame; the first 10 s of the take are thrown away (shader
  compilation, asset upload). Per beat: p50, p95, p99, max.
- Target (CLAUDE.md section 7; the owner's best machine is now an RTX 3070 PC): p95 within 8.3 ms
  and p99 within 16.7 ms in the cockpit. The Mac is not that machine; its numbers are a baseline and
  for finding the expensive systems, read as such.
- Cost of the instrument: one vector push a frame.

## The suite, from Pale Blue Dot (owner, 2026-10-24: "Do a benchmark. Get perf tools from pale blue dot and measure perf")

`tools/perf/perf_suite.py` and `tools/perf/perf_report_html.py` are Pale Blue Dot's suite and
report (revision db25fe2), pointed at `--bench world|fight`. What they measure, per run:

- **Frame wall time**, every frame, from `CB_FRAME_LOG` (a CSV the bench writes): p50, p95, p99,
  p99.9, max, after a 10 s warm-up; and the spread of p95 between repeats.
- **The render passes' time**, from Bevy's `RenderDiagnosticsPlugin`: CPU ms per pass per frame,
  and GPU ms where the adapter has timestamp queries (Vulkan and DX12; on Metal, Bevy 0.20
  records CPU time only, so the owner's Mac reports no GPU column).
- **Entity count** per frame, and the process's **resident memory**, sampled every 0.25 s.
- **Variants** interleaved A, B, then B, A: an old build with its own `data/` (`CB_ROOT`), or the
  same build with a system switched off (`CB_NO_SHADOWS=1`, `CB_NO_WATER_FX=1`).

No screenshot and no recording during a measured run (owner, 2026-10-24: "dont take screenshot on one run or obs, and just do pure perf measurement"): the bench's stills cost about 11 ms a frame each and are off unless `CB_BENCH_STILLS=1`; an OBS take is a separate pass.

**Windowed runs** (owner, 2026-10-24: a benchmark with no window "doesn't tell us anything"): `perf_suite.py --windowed` sets `CB_BENCH_WINDOWED=1`, and the game plays full size (1920 x 1080) centred on the main display, held to its refresh. Those runs report what a player feels: the share of frames that missed a refresh (over 1.5 times the median), and freezes over 50 ms. Offscreen runs still measure what each system costs.

A run is invalid if it crashes, times out, never prints `BENCH all`, or logs fewer than 300 frames.
A build from before the frame log reports totals from its `BENCH all` line.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| T3 | Target | p95 8.3 ms, p99 16.7 ms at 1080p | decided by the owner |
