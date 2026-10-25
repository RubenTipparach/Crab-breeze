# RTX 3070 Benchmark

The night of 2026-10-25 on the owner's desktop: the world, fight and defence takes, played in the
game's window and presented through the GPU as a player sees it (CLAUDE.md section 7). Two changes
were measured against the build before them (building dressing off the main thread, burst sizes in
steps), and live shadows to 700 m against 500 m. Each pair of variants ran interleaved, two runs each,
in one sitting.

## Findings

1. **With vsync on, what the player gets is 60 fps with drops.** The 60 Hz display holds p50 at
   15.7 ms, and 9-11 % of refreshes are missed on the world and fight takes (p95 31 ms, a frame
   that misses waits for the next refresh). `output/perf/20261025-baseline`.
2. **With vsync off, the frame costs 8-9 ms on the world take, 12-13 ms in the fight and 14 ms in the
   defence.** p95 is 11-13 ms, 15-19 ms and 26 ms. All three are over the 8.3 ms p95 target; the
   2026-10-24 report had p50 6.2-7.9 ms on the same takes.
3. **Shadows and buildings cost about 4 ms of CPU each on the fight take** (p50 12.6 ms; 8.2 ms with
   no shadows, 8.7 ms with no buildings). The GPU stays at 1-7 ms: the cost is the CPU work done for
   every mesh in every view (the main view and three shadow cascades), and each building is its own
   mesh. This is the next thing to change.
4. **Particles are where the fight's missed refreshes come from**: 5.9 % with them, 0.8 % without,
   p95 19.8 against 14.2 ms. Building new effects mid-fight is not it (every size is now built behind
   the loading screen and the spikes stay); it is the live effects.
5. **The building hitch is gone**: dressing a nearby building took 12-15 ms of the main thread, 45 ms
   at worst, one building a frame while moving. Meshed off the main thread now, no frame spends over
   4 ms on it (232 such frames before on the world take), and freezes over 50 ms fell from 15-17 to
   8-9. p50 and p95 did not move: those frames were about 1 % of the take.
6. **Live shadows to 700 m cost the same as to 500 m**, inside the spread on both takes.
7. **One frame of 16.4 s** in a fight run of the unchanged build, at an ordinary moment (no explosion,
   no load, nothing in the log). Not explained; not seen again in five other fight runs.

| Take (vsync off) | Variant | p50 ms | p95 ms | p99 ms | Missed refreshes | Freezes over 50 ms |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| World | before | 9.22 | 12.30 | 23.50 | 3.32 % | 19 |
| World | after | 9.50 | 12.71 | 23.70 | 3.22 % | 22 |
| World | after, shadows to 500 m | 9.36 | 12.73 | 22.12 | 3.44 % | 16 |
| Fight | before | 12.17 | 15.23 | 24.38 | 1.75 % | 26 |
| Fight | after | 12.60 | 17.24 | 27.41 | 3.39 % | 10 |
| Fight | after, shadows to 500 m | 12.53 | 17.99 | 27.38 | 4.11 % | 10 |
| Defence | after (one run) | 14.33 | 25.86 | 35.54 | | |

The fight take plays out differently from run to run (how long the fight lasts, how many buildings
fall), so its spread is wide: 0.2-2.6 ms of p95 between the two runs of one variant.

| Fight take, vsync off | p50 ms | p95 ms | p99 ms | Missed refreshes |
| --- | ---: | ---: | ---: | ---: |
| everything on | 12.60 | 19.81 | 34.19 | 5.90 % |
| no trees | 12.30 | 18.19 | 28.59 | 4.63 % |
| no buildings | 8.73 | 11.88 | 14.13 | 6.35 % |
| no shadows | 8.15 | 12.82 | 15.56 | 9.36 % |
| no cockpit pass | 11.50 | 15.25 | 22.61 | 2.58 % |
| no particles | 11.62 | 14.19 | 16.59 | 0.80 % |

## How this was measured

- RTX 3070, Intel i7-9700F, 32 GB, Windows 10, Vulkan, the game's window at 1920 x 1080 on a 60 Hz
  display. "vsync off" is `CB_UNCAPPED=1`: on screen and presented, not waiting for the display, so
  the cost of a frame shows under 16.7 ms.
- Release builds of `d2aa6ed` (before) and `be48233` (after), the first 10 s of each run thrown
  away. No OBS, no screenshots, no tracing, no build beside a run. The owner was asleep.
- A Chrome-traced build ran once on the fight take to see where time goes. Its 7.7 GB trace slowed
  the game about tenfold and stalled systems that log, so only its shares were used, never its times.
- The defence take now ends 6 minutes in: a raider 90 m up outlived the missiles and the take never
  finished.
- `python tools/perf/perf_suite.py --scenarios fight,world --repeats 2 --variant "before|target/perf-before/crab-breeze.exe|CB_UNCAPPED=1|<main checkout>" --variant "after|target/release/crab-breeze.exe|CB_UNCAPPED=1|<worktree>" --variant "after-500m|...|CB_UNCAPPED=1|<data with live_far_m 500>"`

## Limits

- Two runs a variant: a difference under the spread is not a result, and most of tonight's are.
- The fight take is not repeatable enough to judge a change of 1-2 ms; a fixed fight is needed.
- The defence take has one valid run: the second variant failed to launch (the suite's working
  directory), to be fixed.
