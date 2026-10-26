# Free-roam performance run

The night of 2026-10-26 on the owner's desktop, on the owner's request: "bot should accept 1 mission, complete it, the
accept a mission as far away as possible. Go there and complete it. Do this 3 times. Well watch the frame graph, ensure
it always stays withing the 60-120 fps range. If it drops below raise some flags and study the conditions that cause
it." The owner was asleep and had said Claude may playtest.

## Findings

1. **Fights and fast skating fell far below 60 fps, and the CPU was why.** On the merged main (`d9fb639`), a fight's
   median frame was 15 to 30 ms and its slowest 5 % 50 to 120 ms; 50 to 85 % of a fight's frames were under 60 fps.
   The GPU took 1 to 1.5 ms of every frame.
2. **One function was most of it: the ground's height.** `Heightfield::height_at` worked every height out from
   scratch (the island's noise, the city flats and the river carve, for three corners), and nothing was kept. Two
   callers paid it hundreds of times a step: where the guns converge (`look_point`, a 2 m march to 1.5 km, 5 to 8 ms
   a step) and every shell in flight (0.43 ms each, about 17 in flight: 7 ms a step).
3. **Fixed: the corners are now kept** (`openspec/changes/terrain-height-cache`): tiles of 16 x 16 corners filled on
   first read, bit for bit the same heights (tested). `look_point` fell to 0.05 ms, a shell to 0.01 ms, the whole
   mech step from 6 to 7 ms to 0.2 ms. The tree streaming stalls (up to 380 ms) went with it: growing a tree tile
   reads the ground too.
4. **After the fix, 97 % of the run is in the 60 to 120 fps band or above.** Fights have 1.5 to 7 % of frames under
   60 fps, the median about 9 ms. What is left is the ordinary CPU cost of a frame (7.5 to 10 ms, 2 000 to 3 000
   meshes drawn, GPU about 1 ms), and stutters while skating at 40 m/s and more out of the first air base: the per
   mesh CPU work the 2026-10-25 report put on buildings (`openspec/changes/gpu-instancing`).
5. **Logging is not a cost**: the same fight with the game's info logging off was the same within the spread.
6. **Two game bugs turned up** (not performance, reported for their own changes):
   - At 88 m/s the mech struck something on a hilltop near (-6049, -7060) and stopped dead, unable to move any way,
     twice, at the same spot. Probably tunnelled into a building's cells, then pushed back every step.
   - The bot's jets carried it over the ridge and down into water at 36 m/s, where it slowed to a crawl.

## Before and after, the free-roam fight

`free-roam-fight` (board, take the nearest job by mail, drive 4.2 km on skates, fight two crabs), the old build and the
new interleaved, three runs each, vsync off (`CB_UNCAPPED=1`), the first 3 s of each dropped.

| | Build | p50 ms | p95 ms | p99 ms | Under 60 fps |
| --- | --- | ---: | ---: | ---: | ---: |
| Drive (3 runs) | old | 8.85 to 9.30 | 26.9 to 29.1 | 38.9 to 42.1 | 15.4 to 18.7 % |
| | new | 8.45 to 8.57 | 18.4 to 20.9 | 27.4 to 27.7 | 5.2 to 5.6 % |
| Fight (3 runs) | old | 10.4 to 16.9 | 27.8 to 82.7 | 43.7 to 196 | 16.0 to 50.7 % |
| | new | 8.97 to 9.14 | 11.9 to 19.1 | 19.5 to 23.7 | 1.5 to 6.8 % |

Frames over 50 ms across the three runs: 839 old, 19 new. Each fight plays out differently (its length, how many
shells fly), so the old build's fight spread is wide; every new run is better than every old one.

Per simulation step (`sections:` log lines, `perf_hud::section`), in the fight:

| Section | Old | New |
| --- | ---: | ---: |
| `look_point` | 5.4 to 7.2 ms | 0.05 to 0.08 ms |
| one shell's flight | 0.43 ms | 0.006 to 0.011 ms |
| `step_mech` (all mechs) | 5.7 to 7.6 ms | 0.1 to 0.2 ms |
| an enemy's brain | 0.2 ms | 0.007 ms |

## The long run

`free-roam-perf`: a new career, the Foreman; by mail from the cockpit, the nearest job, then the farthest, three times.

| Run | Build | Played | p50 ms | p95 ms | p99 ms | Worst | Under 60 fps |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: |
| 1 | old | 1.6 min, stuck at (-6068, -7080) | 8.64 | 38.0 | 75.2 | 289 ms | 17.0 % |
| 4 | new | 2.2 min, stuck past the ridge | 8.56 | 12.3 | 25.5 | 75 ms | 3.1 % |
| 5 | new | 11.2 min, a 10 min fight not won | 9.14 | 12.35 | 19.5 | 108 ms | 1.5 % |

By what the mech was doing in run 4: standing or slow, 0.6 to 0.9 % under 60 fps; 15 to 40 m/s, 0.7 %; 40 m/s and
more, 5.8 % (p95 19 ms); in a fight, 2.6 %. The slowest places were the first 1.5 km out of the air base at
(-10 500, -5 600), at 80 m/s.

Runs 1 and 4 ended on the way to the far job (12 km), not on performance: the island between is a ridge, and the bot
drives straight. The bot now gives up an unreachable job and takes the next farthest (at most three times). In run 5
the only offer open after the first job was 1.5 km away; the bot reached it and fought for the whole 10 min cap
without finishing the last crab (a bot or job problem, not performance: 62 000 frames of that fight had 1.2 % under
60 fps). The six-job loop the owner asked for has not been completed yet: the bot needs a route finder for the long
trips, and a look at why that fight could not be won.

Run 5 by speed: standing or walking (to 15 m/s), 0.4 to 1.0 % under 60 fps; 15 to 40 m/s, 2.4 %; 40 m/s and more, 7.3 %
(p95 21.9 ms). The slowest places are the first stretches out of the air base and the ridge at (-6 500, -7 000).
`redraw_buildings` went over 4 ms 40 times (7.3 ms on average, 25 ms at worst) and frames over 50 ms numbered 32 in
11 min (64 ms on average, 111 ms at worst).

## Next

1. **Skating fast**: the per-mesh CPU cost (buildings, `gpu-instancing`) and `redraw_buildings`; 40 m/s and more is
   the one condition still over 5 % under 60 fps.
2. **The remaining stutters** (32 in 11 min): matched to their frames in `output/perf/free-roam-perf-run5.csv`.
3. **The mech trapped near (-6049, -7060)** at speed: a collision bug, its own change.
4. **The bot's long trips**: a route along the roads so the farthest job is reached, then the full six-job loop.

## How this was measured

- RTX 3070, Intel i7-9700F, 32 GB, Windows 10, Vulkan, the game's window at 1920 x 1080 on a 60 Hz display, vsync off
  (`CB_UNCAPPED=1`: on screen and presented, not waiting for the display, so a frame's cost shows under 16.7 ms).
  Release builds of `perf-free-roam` before (`output/perf/bin/old.exe`) and after (`new.exe`) the cache. No OBS, no
  screenshots, no build beside a run. The owner was asleep.
- Every frame logged by the scripted pilot (`output/perf/<script>-frames.csv`: time, frame ms, beat, position, speed,
  enemies); `tools/perf/free_roam_report.py` and `tools/perf/ablate_report.py` read them.
- Where the time went: a `trace_chrome` build of one fight (18 GB, its span totals kept in
  `output/perf/trace/spans.txt`), then `perf_hud::section` timers in the release build. The traced build is far slower
  than the game and its frame rate is not a result; it only showed which spans to time.
- Switching parts off one at a time (particles, shadows, buildings, clouds, effect lights; two runs each) before the
  fix did not move the fight beyond the spread: the cost was not in drawing.
- The window could not be captured by a screen grab (a Vulkan swapchain); the five-minute checks were made from the
  frame log and the pilot's track instead.
