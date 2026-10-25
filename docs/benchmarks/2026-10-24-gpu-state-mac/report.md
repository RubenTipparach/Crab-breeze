# Building updates on the GPU, Mac, 2026-10-24

City combat (`--bench defence`), interleaved in one sitting, 2 runs each, windowed at 1920 x 1080
on the M4 Max's 120 Hz display, nothing else running. **Not the RTX 3070.**

- **cpu**: `d98f207`, a hit rebuilds the damaged storeys' meshes (textures, SSAO).
- **gpu-shared**: `building-gpu-state`, meshes built once per building, one shared element state
  buffer, a hit writes its building's range in the render world.

## Findings

1. **Frames are as good as before, and the redraw work is gone.** p99 26.55 against 27.00 ms, p95
   23.73 against 21.81 ms (inside gpu-shared's 5.67 ms spread), worst 50.6 against 43.7 ms. Slow
   `redraw_buildings` frames 12-16 a run against 90-97; what is left is each building's one-time
   build on its first hit.
2. **A first version with materials per building was three times slower and got worse as the fight
   went on** (p50 16.7 ms in the first minute, 60-80 ms ten minutes in; 20.6 fps). Every damaged
   building added about 13 materials, so its draws could not batch with any other building's, in
   every pass. One buffer for all buildings and one material per look fixed it: the frame time
   stays flat at 16.7 ms for the whole fight.

| build | fps | p50 ms | p95 ms | p95 spread | p99 ms | worst ms | slow redraws a run |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| cpu | 60.00 | 16.66 | 21.81 | 1.14 | 27.00 | 43.7 | 90-97 |
| gpu (materials per building) | 20.60 | 58.93 | 81.75 | 18.19 | 84.50 | 370.4 | 14 |
| gpu-shared | 60.00 | 16.68 | 23.73 | 5.67 | 26.55 | 50.6 | 12-16 |

The materials-per-building row is from the sitting before (`output/perf/gpu-state-ab`), against
the same cpu build.

## Limits

- Apple M4 Max at 60 Hz pacing for most of these frames; the RTX 3070 shows how much headroom moved.
- Draw count is next (`building-texture-array`): about 10 meshes per damaged building today.
