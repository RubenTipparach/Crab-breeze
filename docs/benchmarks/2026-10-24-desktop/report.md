# RTX 3070 Benchmark

The world, fight and defence takes on the owner's desktop, played in the game's window and presented
through the GPU as a player sees it (CLAUDE.md section 7: no offscreen perf rendering). Two variants,
two runs each, interleaved: the current build, and the same build with no trees drawn
(`CB_NO_TREES=1`).

## Findings

1. **p50 is inside the target on every take**: 6.2-7.9 ms (125-160 fps).
2. **p95 misses 8.3 ms on every take, by 0.2-2.3 ms**: 9.0 ms on the world take, 10.6 ms on the fight,
   10.1 ms on the defence. The trees are not what holds it there: with no trees p95 is 9.0, 10.2 and
   9.4 ms.
3. **The trees are where the spikes come from in combat**: on the defence take p99 drops from 17.0 to
   11.1 ms with no trees, the worst frame from 67 to 23 ms, frames over 50 ms from 1 to 0, and missed
   refreshes from 2.1 % to 0.7 %. They also cost about 180 MB of memory.
4. **The fight take misses the most refreshes** (7.8-9.7 %) with or without trees: the scripted pilot
   spins its view wildly there (a known bot fault, being fixed), so that take's numbers include a
   view that sweeps the whole city several times a second.
5. **The GPU is nearly idle**: about 2 ms of GPU work a frame with trees, 1.1 ms without. The frame
   time is CPU work.

## How this was measured

- RTX 3070, Intel i7-9700F, 32 GB, Windows 10, Vulkan; the game's window at 1920 x 1080, held to the
  display (`CB_BENCH_WINDOWED=1`). Windows reports the RTX 3070's output at 60 Hz, yet the runs
  presented 124-151 frames a second, so presentation was not held to 60 Hz.
- Release build with thin LTO and one codegen unit, from `5e93817` plus uncommitted work (trees grown
  far only round the pilot's eye, fallen trees drawn on their own, crater and tree meshing off the
  main thread, no point light on autocannon flashes, three shadow cascades to 500 m, effects warmed up
  behind the loading screen).
- No OBS, no screenshots, no tracing, no build running beside a run. The owner was away.
- The scripted pilot's view spins wildly at times (a bot fault being fixed); it does so in both variants.
- The defence take ends when the scripted pilot loses (under 3 minutes into wave 1): it measures the
  first wave's fight, not all three waves.
- `python tools/perf/perf_suite.py --exe target/release/crab-breeze.exe --scenarios world,fight,defence --repeats 2 --variant "current" --variant "no-trees||CB_NO_TREES=1"`
