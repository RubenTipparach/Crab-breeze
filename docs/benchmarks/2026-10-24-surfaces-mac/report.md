# Textured buildings and SSAO on the Mac, 2026-10-24

City combat (`--bench defence`, raiders spawning in the streets), three builds interleaved in one
sitting, 2 runs each, windowed at 1920 x 1080 on the M4 Max's 120 Hz display, nothing else
running. **Not the RTX 3070.**

- **before**: `building-research` at `2e00b49`, flat-colour building parts.
- **new**: the textured parts and furniture (`building-surfaces`), tangents written directly,
  debris physics only within 400 m (`debris-physics`), SSAO off (`CB_NO_SSAO=1`).
- **new-ssao**: the same with SSAO on (`ambient-occlusion`, 1.5 m, medium).

## Findings

1. **The textured build is no slower than the flat one, and steadier.** p50 16.67 against 16.64 ms,
   p99 30.53 against 35.30 ms, freezes over 50 ms 0 against 8, worst frame 44 against 68 ms.
2. **The redraw cost fell about tenfold.** Slow `redraw_buildings` frames: 742-1266 a run before,
   48-150 after. Most of it was MikkTSpace tangent generation, which the old build already ran on
   the skin meshes; the boxes' tangents are now written straight from their planar UVs (test:
   `planar_tangents_match_mikktspace_on_a_box`).
3. **An earlier textured build without that fix was much worse**: p95 38.6 against 23.3 ms and 494
   freezes against 9 (same sitting style, 2 runs each), because every hit ran MikkTSpace on up to
   8 meshes per band.
4. **SSAO costs nothing measurable here**: new-ssao p95 21.45 ms (spread 0.21), p99 25.00 ms,
   against new's p95 26.82 ms (spread 5.70) and p99 30.53 ms; the difference is inside new's own
   spread, so it is not a speedup either. One 373 ms freeze in a new-ssao run, at the wave's start.
5. **Still over the 120 fps target**: every build's p50 sits at 16.6 ms on this display.

| build | fps | p50 ms | p95 ms | p95 spread | p99 ms | worst ms | freezes > 50 ms | peak MB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| before | 59.83 | 16.64 | 24.26 | 0.59 | 35.30 | 67.9 | 8 | 1370 |
| new | 60.00 | 16.67 | 26.82 | 5.70 | 30.53 | 44.1 | 0 | 1380 |
| new-ssao | 59.87 | 16.63 | 21.45 | 0.21 | 25.00 | 373.5 | 2 | 1471 |

## Limits

- Apple M4 Max, not the RTX 3070. The scripted pilot's fight differs run to run (its mech was
  destroyed in every run, city damage 2-6 %), so two runs a build is a small sample.
- Peak process memory: new 1380 MB against before 1370 MB, inside run noise; new-ssao 1471 MB
  (the normal prepass and SSAO buffers). The texture sets are about 134 MB on the GPU (design
  estimate), not measured here.
