# Performance suite, 2026-10-24 01:05

- revision: `2b30a90` (uncommitted changes)
- adapter: Apple M4 Max (Metal); offscreen 1920 x 1080, uncapped (`--bench`)
- first 10.0 s of each run thrown away; 2 run(s) per variant, interleaved
- budgets: p95 within 8.3 ms (120 fps), p99 within 16.7 ms (60 fps); **over** marks a miss. Each cell is the median over the valid runs; spread is the largest minus the smallest p95 between them.

## world

| variant | valid runs | fps | p50 | p95 | p95 spread | p99 | p99.9 | max | render CPU p50 | GPU p50 | entities | peak memory MB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| before | 2/2 | 121.65 | 8.22 | 10.71 **over** | 0.55 | 16.83 **over** | - | 44.93 | - | - | - | 2224.41 |
| current | 2/2 | 403.61 | 2.46 | 2.92 | 0.03 | 3.23 | 3.83 | 74.36 | 0.03 | - | 18999.00 | 815.50 |
| no-shadows | 2/2 | 478.55 | 2.05 | 2.63 | 0.14 | 2.93 | 3.40 | 14.66 | 0.03 | - | 18999.00 | 772.91 |
| no-water-fx | 2/2 | 457.64 | 2.16 | 2.67 | 0.11 | 2.96 | 3.45 | 15.62 | 0.02 | - | 19007.00 | 837.00 |
- before run 0: no frame log (a build from before it): totals from its BENCH line (its fps column is 1000 / p50)
- before run 1: no frame log (a build from before it): totals from its BENCH line (its fps column is 1000 / p50)

Render passes, mean CPU ms per frame (GPU ms where the adapter reports it):

| pass | before | current | no-shadows | no-water-fx |
| --- | ---: | ---: | ---: | ---: |
| clustering | - | 0.01 | 0.01 | 0.01 |
| main_opaque_pass_3d | - | 0.00 | 0.00 | 0.01 |
| main_transmissive_pass_3d | - | 0.00 | 0.00 | - |
| uniform_allocation | - | 0.00 | 0.00 | 0.00 |
| per_view_shadow_pass | - | 0.00 | 0.00 | 0.00 |
| main_transparent_pass_3d | - | 0.00 | 0.00 | 0.00 |
| early_mesh_preprocessing | - | 0.00 | 0.00 | 0.00 |
| bin_unpacking | - | 0.00 | 0.00 | 0.00 |
| upscaling | - | 0.00 | 0.00 | 0.00 |
| tonemapping | - | 0.00 | 0.00 | - |
| auxiliary_views | - | 0.00 | 0.00 | 0.00 |

![world](world.png)

## fight

| variant | valid runs | fps | p50 | p95 | p95 spread | p99 | p99.9 | max | render CPU p50 | GPU p50 | entities | peak memory MB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| before | 2/2 | 119.33 | 8.38 | 10.58 **over** | 0.05 | 11.64 | - | 42.91 | - | - | - | 2287.11 |
| current | 2/2 | 378.00 | 2.53 | 3.89 | 0.09 | 5.20 | 6.46 | 29.39 | 0.03 | - | 16764.00 | 796.66 |
| no-shadows | 2/2 | 456.45 | 2.09 | 2.87 | 0.08 | 4.83 | 5.85 | 28.35 | 0.03 | - | 16769.00 | 773.14 |
| no-water-fx | 2/2 | 436.00 | 2.19 | 2.97 | 0.03 | 5.04 | 5.94 | 28.46 | 0.02 | - | 16758.00 | 768.94 |
- before run 0: no frame log (a build from before it): totals from its BENCH line (its fps column is 1000 / p50)
- before run 1: no frame log (a build from before it): totals from its BENCH line (its fps column is 1000 / p50)

Render passes, mean CPU ms per frame (GPU ms where the adapter reports it):

| pass | before | current | no-shadows | no-water-fx |
| --- | ---: | ---: | ---: | ---: |
| clustering | - | 0.01 | 0.01 | 0.01 |
| main_opaque_pass_3d | - | 0.00 | 0.00 | 0.01 |
| uniform_allocation | - | 0.00 | 0.00 | 0.00 |
| main_transmissive_pass_3d | - | 0.00 | 0.00 | - |
| per_view_shadow_pass | - | 0.00 | 0.00 | 0.00 |
| main_transparent_pass_3d | - | 0.00 | 0.00 | 0.00 |
| early_mesh_preprocessing | - | 0.00 | 0.00 | 0.00 |
| bin_unpacking | - | 0.00 | 0.00 | 0.00 |
| upscaling | - | 0.00 | 0.00 | 0.00 |
| tonemapping | - | 0.00 | 0.00 | - |
| auxiliary_views | - | 0.00 | 0.00 | 0.00 |

![fight](fight.png)

