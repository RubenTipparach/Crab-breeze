# Structural destruction on the CPU: is it cheap enough? (2026-10-24)

A measurement instrument for `structural-destruction` and `rubble` (CLAUDE.md section 4 allows one).
`structbench.rs` ports the mockup's graph, solve and debris step to plain Rust (std only, no Bevy)
and times them. It is not game code.

Run it: `cargo new structbench && cp structbench.rs structbench/src/main.rs && cd structbench && cargo run --release`.

## Conditions, and why this is indicative, not a claim

- Release build, one core, on the owner's Apple M4 Max.
- The owner was using the Mac at the time, so this breaks section 7's "nothing else running". The
  results are indicative only.
- Synthetic towers built like the mockup's: 6 x 6 bays, a 3-storey podium, inset above, with 2
  partitions and 4 furniture items per cell per storey.
- p50, p95 and max are over 400 runs (300 for debris).
- No GPU work is measured here. Cloud and CPU runs measure nothing of the frame (section 7).

## Results (ms)

| What | Elements | Bonds | p50 | p95 | max |
| --- | --- | --- | --- | --- | --- |
| Solve, 19-storey tower, intact | 3 751 | 7 975 | 0.057 | 0.067 | 0.091 |
| Solve, same, every ground column on one side gone | | | 0.062 | 0.072 | 0.116 |
| Solve, 60-storey tower, intact | 10 680 | 22 776 | 0.171 | 0.192 | 0.256 |
| Solve, same, one ground side gone | | | 0.180 | 0.201 | 0.243 |
| Debris step, 600 pieces moving | | | 0.023 | 0.028 | 0.061 |
| Debris step, 2 000 moving | | | 0.078 | 0.090 | 0.149 |
| Debris step, 9 000 moving | | | 0.354 | 0.391 | 0.520 |
| Restack of 9 000 resting pieces | | | 0.277 (once) | | |

Building the graph itself takes 0.5 ms (19 storeys) and 1.1 ms (60 storeys).

## What it says

- The simulation is not the problem. A 60-storey tower solves in under 0.2 ms. The design's limit of
  4 solves per 1/60 s step keeps the worst step under 1 ms, and that holds with room to spare.
- The cost of a collapse is in **what is drawn and simulated as pieces**. The mockup's Full debris
  put 7 556 pieces on the ground from the warehouse. With Game debris (the biggest boxes of each
  member only, 2 parts at most, glass, partitions and furniture straight to dust and heap) it was
  1 387, 5.4 x fewer, and it still reads as a collapse (`docs/screenshots/buildings/warehouse_drop.png`
  against `warehouse_drop_full.png`).
- **Still to measure, on the owner's hardware with the owner's go-ahead** (section 8): the GPU side.
  That is the element-mask material on 25 or more LOD0 buildings with interiors, shadows over them,
  and 2 500 debris instances, in `perf-bench` against the 8.3 ms p95.
