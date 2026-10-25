# Tasks: distant-shadows

- [x] 1.1 `SunShade` bake and lookup, tested: it agrees with a ray toward the sun at 2989 of 2989 clear-cut points; baked in 445 ms (release) from a corner grid; with the sun 43 degrees up about 0.8% of the island is in terrain shade
- [x] 2.1 Far chunks darkened by it; the sun's direction shared
- [x] 2.2 Cascades to 700 m, measured against 500 m (`live_far_m` in data), on screen, vsync off, 60 Hz,
  two runs each, interleaved (`output/perf/20261025-round2`): no measured difference. Fight p95 17.2
  (700 m) and 18.0 ms (500 m), spreads 1.7 and 2.6 ms; world p95 12.7 and 12.7 ms. All shadows off
  (`CB_NO_SHADOWS=1`) saves about 4.5 ms of p50 on the fight take; that cost is in the cascades
  near the eye, not their far end.
- [ ] 3.1 Stills: far ridges casting shade
