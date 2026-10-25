# Tasks: particle-steps

- [x] 1.1 `burst_count` in steps, tested (quarter steps, whole steps under the budget, a footstep at any speed is one of a few sizes)
- [x] 2.1 Warm-up at every step, budget and range lifted while loading
- [x] 3.1 Perf suite before against after, on screen, vsync off, 60 Hz display, two runs each, interleaved
  (`output/perf/20261025-round2`, with dress-off-thread in the same build): no measured change. Fight
  p95 15.2 before, 17.2 after (spread 0.2 and 1.7 ms; the take plays out differently run to run);
  world p95 12.3 before, 12.7 after (spread 1.0 and 0.3 ms). The spikes `CB_NO_PARTICLES=1` removes
  (5.9 to 0.8 % missed refreshes) are not from building new effects: they stay with every size built
  behind the loading screen. Kept for the warm-up's fix (small bursts were culled by range and never
  built before play) and fewer GPU effects; the cost of live effects is the next thing to measure.
