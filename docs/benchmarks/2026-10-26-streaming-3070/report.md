# Map streaming at speed, on the RTX 3070

2026-10-26, the desktop rerun the Mac asked for in `docs/benchmarks/2026-10-26-streaming-mac/report.md`, toward the
owner's goal: "Let's focus on getting no frames under 60fps, then we'll focus on 90 afterwards. The biggest hitches is
loading and unloading sections of the map it seems." The owner had allowed desktop playtests overnight; the desktop had
had no input for 5 hours when the runs began.

## Findings

1. **On the 3070 desktop, the map streaming does cost whole frames, and the biggest piece is uploading meshes to the
   GPU.** `allocate_and_free_meshes` (the render world copying new and changed meshes into its buffers) took 16.7 ms
   or more in 245 to 341 frames of each 6.5 min route, 33 ms or more in 13 to 47, and up to 111 ms. The Mac saw it
   in 1 to 9 frames, up to 26 ms. Those frames add or change only 2 meshes (median; 22 at most), so a few large
   meshes are the cost, not many small ones. It is the open question of the Mac's report, answered: yes, the 3070's
   driver takes them slowly, and it is the first thing to fix.
2. **The three fixes at `caf0be9` (ambience bed seek, tree ring budget, car transforms) do not move the frame time
   here beyond the spread between runs.** Over 16.7 ms: old 17.4 and 20.8 % of frames, new 19.7 and 16.6 %. The car
   fix does what it says: the median car transforms written a frame fell from 425 and 593 to 307 and 309. But each
   moving car also writes its four wheels (`WHEEL_FL` and the rest), so about 1 500 transforms are still written every
   frame, and `propagate_parent_transforms` takes 2.5 to 2.8 ms on its slow frames.
3. **These numbers come from the `hitch_spans` build, logging every frame, and it is far slower than the game.** The
   median frame here is 12.1 ms; the release build on the same desktop, in free roam out of the same first air
   base, was 9.1 ms (`docs/benchmarks/2026-10-26-free-roam/report.md`, run 5). Timing every system and writing a
   20 MB log a run costs about 3 ms a frame on this machine, so the percentages under 60 fps are not the game's. The
   comparison between old and new, and the ranking of causes, stand; the absolute numbers need a release run without
   the spans (below).
4. **The fight could not be measured: the scripted pilot cannot take a job by mail on `living-island`.** In
   `free-roam-fight` the bot found the offer ("the Nearest offer is job 0, 4247 m away"), found the Accept button and
   clicked it, again and again, and the job was never accepted; the beat failed after 240 s. The career's `accept` rule
   is unchanged and the button exists (otherwise the bot fails after 8 s with "no Accept button on the screen"), so
   the click is not reaching the button: something drawn over the mail page, or taking the pointer, on
   `living-island`. It did not happen on `perf-free-roam` before the merge. Both builds fail the same way. Not
   investigated further: the owner may be back at the desktop, so no more runs.
5. **The jet leg stopped in mid-air again**, in one run of four, near (-8 597, 1 092), 442 m short of beat 10's goal:
   the same place the Mac's runs stopped. The skating beats, the comparison, played in all four.

## The runs

`streaming-route`, interleaved old, new, new, old in one sitting, 13:35 to 14:00. "Old" is `cea7ed6`, "new" `caf0be9`,
each built in its own checkout (`cargo build --release -p cb-app --features hitch_spans`, the two binaries checked to
differ). `CB_UNCAPPED=1 CB_NATIVE_1080=1 CB_HITCH_MS=0.1`. The first 3 s of each run dropped.

| Run | Build | Frames | p50 ms | p95 ms | p99 ms | Max ms | Over 16.7 ms | Over 11.1 ms | Skating beats over 16.7 ms |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| o1 | old | 22 348 | 12.09 | 21.82 | 36.47 | 98.2 | 17.4 % | 59.7 % | 30.0 % |
| n1 | new | 22 626 | 12.13 | 23.40 | 37.31 | 146.3 | 19.7 % | 60.0 % | 32.6 % |
| n2 | new | 22 327 | 12.09 | 21.74 | 36.09 | 125.3 | 16.6 % | 60.1 % | 29.5 % |
| o2 | old | 14 757 | 13.43 | 23.68 | 37.32 | 89.2 | 20.8 % | 73.1 % | 35.5 % |

o2 stopped on the jet leg (finding 5), so it has fewer frames. Standing still (under 2 m/s): 0.3 to 0.4 % over
16.7 ms in every run, p50 10.1 to 10.3 ms. The cost is in moving fast across the map.

## Mesh uploads, per run

| Run | Upload 2 ms or more | 10 ms or more | 16.7 ms or more | 33 ms or more | Worst |
| --- | ---: | ---: | ---: | ---: | ---: |
| o1 | 2 051 | 590 | 317 | 47 | 73.6 ms |
| n1 | 2 114 | 606 | 341 | 41 | 110.7 ms |
| n2 | 2 050 | 589 | 320 | 40 | 100.7 ms |
| o2 | 1 638 | 470 | 245 | 13 | 52.9 ms |

The largest timed cost of a slow frame, all four runs: `renderer::render_system` (the render graph's command
encoding, 4.1 to 4.4 ms mean: the per-mesh CPU cost the #75 report found), then `allocate_and_free_meshes`, then
`step_traffic` (3.0 to 4.2 ms mean on its slow frames, 25 ms worst). `hitch-causes.txt` beside this file is
`tools/perf/hitch_causes.py` on each log.

## Next

1. **Mesh uploads** (claimed by the desktop session): name the meshes in the slow upload frames (ground tiles,
   building dressing, far facades?) and their vertex and index bytes, then write up how to spread or shrink them in an
   OpenSpec change before changing code.
2. **A release run without the spans**, the same route, old and new, for the real frame time once the owner says the
   desktop is free.
3. **The mail click on `living-island`** (finding 4), so the fight can be measured: for the Mac, whose branch it is.
4. **Wheel transforms**: 4 per moving car every frame, about 1 200 a frame; only cars near the pilot need turning
   wheels (traffic is the Mac's).

## How this was measured

RTX 3070, Intel i7-9700F, 32 GB, Windows 10, Vulkan, a 1920 x 1080 window on a 60 Hz display (100 % scale), vsync off.
No OBS, no screenshots, no build beside a run. Logs and frame files in each checkout's `output/perf/` (not committed:
20 MB a log).
