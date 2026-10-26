# Map streaming at speed, on the Mac

2026-10-26, on the owner's request: "Let's focus on getting no frames under 60fps, then we'll focus on 90 afterwards.
The biggest hitches is loading and unloading sections of the map it seems." Measured on this Mac (Apple M4 Max); the
owner's target is the RTX 3070 desktop, which the desktop session can rerun (the last section says exactly how).

## Findings

1. **On the M4 Max, the main thread's side of loading and unloading map sections causes no frame over 16.7 ms.**
   Along a fixed route across the west of the island at up to 120 m/s on skates and on the jets, the ground, trees,
   grass and building dressing streams never took 2 ms in a frame, and no main-world system crossed the 4 ms `slow`
   line. 0.07 to 0.14 % of the skating frames were over 16.7 ms. The one streaming cost among them is in the render
   world: uploading new meshes (1 to 9 frames a run, up to 26 ms), which includes the streamed tiles and dressing.
2. **The largest main-thread stall was a sound starting: up to 48 ms.** An ambience bed starts 0 to 30 s into its loop,
   and Bevy's start position decodes and throws away everything before it on the main thread (the game's Vorbis decoder
   cannot seek). Fixed: the bed skips on the audio thread while it is still silent (`ambience::BedLoop`). Frames whose
   main world went over 16.7 ms: 16 before, 3 after; the worst sound start 43 ms before, 14 ms after (a music cue).
3. **A body new to a place grew its whole ring of trees in one step**: 14 to 16 tiles, 4 to 8 ms on this machine (the
   desktop's tree stalls reached 36 ms). Fixed: the trees within 60 m at once, the rest 4 tiles a step. The worst step
   for the same body: 1.4 to 4.0 ms, the ring complete in 3 to 4 steps. Measured headless in cb-core; the route has no
   enemies, so it does not exercise it.
4. **Every car's transform was written every frame, parked or not.** Fixed: written only when it moved. Transforms
   written a frame: 503 and 586 before, 362 and 377 after. No change in frame time on this machine beyond the spread.
5. **What is left over 16.7 ms here is the Mac's, not the game's streaming:** waits for the GPU (`prepare_windows`: the
   M4 Max drawing 1080p with fog, far buildings and clouds runs over on some frames; the 3070 draws a frame in about
   1 ms), pipeline compiles on first sight of a material (synchronous on macOS only: Bevy compiles them on worker
   threads on Windows and Linux), and mesh uploads in the render world (whether the 3070's driver takes them as slowly
   is for the desktop rerun to show).
6. **The rocket crater bug is fixed** (its own commit, `openspec/changes/terrain-deformation`): the chunk with the
   crater's hole went on screen before the tile that fills it, and a barrage cancelled the tile's meshing job again and
   again. Two blasts on soil: both chunks went up 71 ms after the blasts, each with its tile on the same frame; the dig
   itself costs 0.03 ms.

## The route

`data/playtests/streaming-route.ron`, a new career: board, then flat out on the skates from the first air base
(-10 481, -5 604) to the road at city 10's south-east corner (-9 700, -6 300), down the road past the capital's west
side, along the road to city 3 (-10 000, -2 800); then on the jets at 120 m over the ground south toward city 5
(-9 029, 1 000) and back over the capital (-7 496, -3 575). About 5.5 min, 40 000 frames, most of them at 40 to 120 m/s.
No job and no fight, so every run goes the same way. The jet legs sometimes end "stuck" near (-8 500, 1 300): the mech
stops there in the air, near where the #75 report found a mech trapped. The skating beats (3 to 9) are the comparison.

## Frames over 16.7 ms, by cause

Every frame timed (`--features hitch_spans`, `CB_HITCH_MS=0.1`); the cause is the largest timed system in the frame or,
for a main-world stall, in the frame before (with pipelined rendering a main-world stall shows in the next frame's
length). "Before" is the branch after merging living-island, before the three fixes; "after" is all three.

| Cause | Before (b6) | Before (b7) | After (x1) | Worst |
| --- | ---: | ---: | ---: | --- |
| Waiting for the GPU (`prepare_windows`, Mac only) | 25 | 15 | 30 | 47 ms |
| A sound starting (`play_queued_audio_system`) | 10 | 7 | 2 | 48 ms before, 20 ms after |
| A pipeline compiling (synchronous on macOS only) | 6 | 6 | 6 | 40 ms |
| Mesh uploads (render world) | 9 | 1 | 9 | 26 ms |
| The first particle effects compiling (warm-up, before the take) | 1 | 1 | 1 | 108 ms |
| **All** | **51** | **30** | **48** | |

b7's jet leg ended early (stuck), so it has fewer frames.

## Before and after

| Run | Build | Frames | p50 ms | p95 ms | p99 ms | Max ms | Over 16.7 ms | Skating beats over 16.7 ms | Main world over 16.7 ms |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| b6 | before | 44 973 | 6.81 | 8.94 | 11.24 | 47.4 | 45 (0.10 %) | 0.11 % | 11 |
| b7 | before | 29 178 | 7.13 | 8.76 | 10.56 | 39.3 | 24 (0.08 %) | 0.12 % | 7 |
| a6 | cars, trees | 46 278 | 6.71 | 8.81 | 11.21 | 39.9 | 47 (0.10 %) | 0.14 % | 11 |
| a7 | cars, trees | 42 585 | 6.77 | 8.86 | 11.02 | 40.3 | 52 (0.12 %) | 0.13 % | 11 |
| y1 | cars, trees | 41 608 | 6.89 | 8.90 | 11.05 | 39.6 | 47 (0.11 %) | 0.13 % | 16 |
| x1 | all three | 42 769 | 6.84 | 8.97 | 11.12 | 39.4 | 42 (0.10 %) | 0.07 % | 3 |

Order: b6, a6, a7, b7 in one sitting, then x1 and y1. The first 3 s of each run are dropped. The main world: p50 4.3 to
4.6 ms and p99 7.8 to 8.0 ms in every run; its worst frame 46 to 48 ms before the bed fix (a bed starting), 19 ms after.
Frame time otherwise did not move beyond the spread between runs: on this machine the main world has about 9 ms to spare.

By speed, after (x1): standing 0.2 %, 2 to 15 m/s 0.2 %, 15 to 40 m/s 0.1 %, 40 m/s and more 0.1 % of frames over
16.7 ms.

## Limits

- **A shared machine.** Five other sessions built Crab Breeze on this Mac at the same time (load average 5 to 50). The
  first baseline (before the living-island merge) ran with up to 390 compiler processes beside it and showed 1.6 % of
  frames over 16.7 ms, with systems preempted in 10 ms slices; it is not used. The runs above were made with the load
  average at 4 to 18, interleaved.
- **The display slept** during one set of runs (every frame paced to 60 Hz). Those runs are not used; every run above
  held the display awake (`caffeinate -d -u`).
- **The Mac is not the target.** Its CPU is faster than the desktop's i7-9700F and its GPU slower than the RTX 3070, so
  the desktop's frames over 16.7 ms will be CPU spikes this Mac absorbs. The desktop rerun below settles it.
- A window of 1920 x 1080 pixels (`CB_NATIVE_1080=1`; the Retina display would otherwise draw 3840 x 2160), vsync off
  (`CB_UNCAPPED=1`), built in release with `--features hitch_spans` (Bevy's trace spans on,
  timing every system: its frame times compare with each other, not with a shipped build). GPU time is not available
  on Metal.

## How to rerun it on the RTX 3070

On the desktop, nothing else running, display on, at `li-perf`'s head and at the commit before the three fixes
(`cea7ed6`), each in its own checkout:

```
cargo build --release -p cb-app --features hitch_spans
set CB_UNCAPPED=1
set CB_NATIVE_1080=1
set CB_HITCH_MS=0.1
target\release\crab-breeze.exe --playtest streaming-route > output\perf\route-<tag>.log 2>&1
python tools\perf\free_roam_report.py output\perf\streaming-route-frames.csv
python tools\perf\hitch_causes.py output\perf\route-<tag>.log
```

Rename `output/perf/streaming-route-frames.csv` after each run; interleave old, new, new, old. `CB_HITCH_MS=0.1` logs
every frame (about 40 000 lines); unset, only frames over 16.7 ms are logged. Then the same with
`--playtest free-roam-fight`: a job's enemies are the bodies new to a place that the tree budget is for.

## The tools

- `--features hitch_spans` (`crates/cb-app/src/hitch_spans.rs`): every system span timed into the frame's tally, and
  the transforms moved, entities spawned and meshes and images added, changed or removed counted.
- `perf_hud`: a `hitch:` line for every frame over `CB_HITCH_MS` (16.7 ms by default), or whose main world was.
- `tools/perf/hitch_causes.py`: the `hitch:` lines read back, by cause.
- The `SkateTo` and `JetTo` beats, and `data/playtests/streaming-route.ron`.
