# The island world against the old map, 2026-10-24

The suite ported from Pale Blue Dot (`tools/perf/perf_suite.py`, openspec change `perf-bench`) played both benchmark takes on the old build (`2fd1104`, the merged first PR: the 4 km map, every tree an entity, clouds on) and on today's build (`2b30a90` plus the suite: the 41 km island, streamed trees, clouds off, the Pale Blue Dot water). Each was played twice, interleaved, with shadows and the water's underwater and lens passes also switched off in turn. Full tables: `results.md`; the page: `report.html`.

## Findings

1. **Today's build is about 3.3 times faster than the old one** on both takes: the world take p50 2.46 ms against 8.22 ms, p95 2.92 against 10.71; the city fight p50 2.53 against 8.38, p95 3.89 against 10.58. The old build missed the 120 fps target at p95 on both takes and the 60 fps floor at p99 once (16.83 ms); today's build is well inside both (p99 3.23 and 5.20 ms).
2. **Memory fell from about 2.2 GB to 0.8 GB** at peak (resident, sampled every 0.25 s). The likely reasons are the trees (41,571 entities before, about 9,700 drawn now from 14,000 kept near the bodies) and heights computed on demand instead of stored; the suite did not separate them.
3. **What made the old build slow is not proven.** The two builds differ in the map, the trees, the ground's levels of detail and the clouds all at once. This morning's baseline of an earlier commit read 2.52 ms on the world take, so the cost arrived with the commits between (textures, many more trees, clouds) and went with the island. A per-system switch on the old build would settle it.
4. **Shadows cost about 0.4 ms at p50 and up to 1 ms at p95** (world p50 2.46 to 2.05 ms; fight p95 3.89 to 2.87 ms). The fight's p95 gains most: debris and buildings cast more.
5. **The water's underwater and lens passes cost about 0.3 ms at p50** (`CB_NO_WATER_FX=1`: world 2.46 to 2.16 ms, fight 2.53 to 2.19 ms), more than their design's estimate of nothing while dry. The surface material stays on in both variants; the difference is the HDR target, the depth prepass and the transmission copy the optics bring, which are on whenever the camera has them.
6. **The regular 13-14 ms frames are the benchmark's own stills**: one every 4.0 s, exactly the period of the offscreen screenshot the bench saves. They are the instrument, not the game. The one real hitch is a 74 ms frame while boarding in one world run (`world_current_0`, at 12.7 s); the fight's maxima of about 28 ms recur in every variant and are not yet traced.
7. **Bevy batches on the GPU on this Mac**: every run logged "GPU preprocessing is fully supported on this device". The entity count was about 19,000 on the world take and 16,800 in the fight.

## Limits

- **Steam was running throughout**, using one to two CPU cores (it was downloading or updating). Interleaving gives every variant the same disturbance, but absolute numbers may read high.
- **Metal reports no GPU time.** Bevy 0.20 records GPU timestamps only on Vulkan and DX12, and the render passes' CPU spans read about zero on Metal, so the GPU columns are empty and the pass table says nothing about where GPU time goes. The owner's RTX 3070 PC would report it.
- **Offscreen, not windowed.** The bench renders into a 1920 x 1080 image so no frame waits for the display; a windowed run on the Mac is held to vsync. These numbers are how much headroom there is, not the frame rate the owner sees.
- **The old build has no frame log**: its numbers are totals from its own `BENCH all` line (10 s warm-up already thrown away), with no traces, p99.9, entity count or render time.
- **Clouds were not measured.** They are off in today's data; the cheap replacement will be measured as a variant.
- Two runs per variant. p95 spread between runs was 0.03-0.14 ms for today's build and 0.05-0.55 ms for the old one; differences smaller than that are not results.
- Not the owner's RTX 3070 machine.
