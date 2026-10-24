# Design: perf-bench

- The window stays (small, 640 x 360) for input and events; the one camera targets an offscreen
  `Image` 1920 x 1080 (Rgba8UnormSrgb), so the frame loop runs unthrottled.
- Frame time is `Time<Real>` delta per frame; the first 10 s of the take are thrown away (shader
  compilation, asset upload). Per beat: p50, p95, p99, max.
- Target (CLAUDE.md section 7; the owner's best machine is now an RTX 3070 PC): p95 within 8.3 ms
  and p99 within 16.7 ms in the cockpit. The Mac is not that machine; its numbers are a baseline and
  for finding the expensive systems, read as such.
- Cost of the instrument: one vector push a frame.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| T3 | Target | p95 8.3 ms, p99 16.7 ms at 1080p | decided by the owner |
