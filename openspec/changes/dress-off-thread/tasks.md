# Tasks: dress-off-thread

- [x] 1.1 Dressing meshed on the async pool, results added nearest first; damaged or distant buildings drop their jobs
- [x] 1.2 `DressingSettled` waits for jobs in flight
- [x] 2.1 Perf suite before against after, on screen, vsync off (`CB_UNCAPPED=1`), 60 Hz display, two runs
  each, interleaved (`output/perf/20261025-dress`):
  - `dress_near_player` over 4 ms: 232 frames on the world take before, none after.
  - World take: freezes over 50 ms 15 and 17 before, 9 and 8 after; p99 20.5 to 19.4 ms (median of
    runs); p50 8.2 and p95 11.3 ms unchanged.
  - Fight take: inconclusive. The take plays out differently run to run (one "after" run fought on
    through the city twice as long and redrew 20 damaged buildings against about 6), p95 15.5 before,
    16.6 and 19.0 after.
- [ ] 3.1 A playtest walking into a city: boxes give way to dressed buildings with no hitch
