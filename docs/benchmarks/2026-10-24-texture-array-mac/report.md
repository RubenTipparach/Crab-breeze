# Texture arrays against shared materials, Mac, 2026-10-24

City combat (`--bench defence`), interleaved, 2 runs each, windowed at 1920 x 1080 on the M4 Max,
nothing else running. **Not the RTX 3070.**

- **shared**: `c58a98b`, building-gpu-state, about 10 meshes per damaged building, one shared
  material per look.
- **arrays**: building-texture-array, 4 meshes per damaged building (frame, facade, skin, glass),
  two shared materials.

## Findings

1. **No measurable difference on the Mac.** p95 23.56 against 25.40 ms and p99 25.13 against 26.86 ms
   favour the arrays, but the spread between runs is 3.40 and 7.37 ms: smaller than the spread is
   not a result (CLAUDE.md section 7). Paired by fight length (the scripted fight ran 14,000-18,000
   or 37,000 frames), the arrays were better in one pair (12.8 % of frames over 20 ms against
   18.0 %) and worse in the other (5.3 % against 0.9 %).
2. **The look matches**: stills of the same street and office with the separate materials and with
   the arrays (`docs/screenshots/building-texture-array/`).
3. The Mac shows frames held to 60 Hz most of the time, where fewer draws do not show; the draw
   saving is for a CPU-bound render, as on the RTX 3070.

| build | fps | p50 ms | p95 ms | p95 spread | p99 ms | worst ms | freezes > 50 ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| shared | 60.00 | 16.68 | 25.40 | 7.37 | 26.86 | 50.3 | 1 |
| arrays | 60.00 | 16.69 | 23.56 | 3.40 | 25.13 | 43.5 | 0 |

## Decision

Not merged into `building-research` (owner, 2026-10-24: "As long as it looks good and perf
improves"): kept on branch `building-texture-array` until a run on the RTX 3070 shows a difference
larger than its spread.
