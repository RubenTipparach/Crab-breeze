# Design: frame-budget

## 1. The gate

`cb_core::perf::FrameBudget` is the rule: a run passes when it saw at least one frame and none was over
`FRAME_BUDGET_MS` (1000/60 ms); a frame whose length is not a finite, non-negative number counts as over. The game
(`crates/cb-app/src/perf_gate.rs`) adds `Time<Real>`'s frame length in `Last` from the frame after `GAME_READY` on (the
frame `GAME_READY` is seen on still measures loading), logs each frame over as `PERF_GATE over:`, and on `AppExit`
prints `PERF_GATE pass|fail: <over> of <frames> frames after GAME_READY over 16.7 ms, worst <ms>, <n> pipelines
compiled after GAME_READY | over: #frame ms, ...`. With `--perf-gate` it also sends `AppExit` code 3 when it failed
(Bevy exits with the first error among the exit messages). Cost: one comparison a frame.

The render world's pipeline cache is walked once a frame from its first unfinished entry; a pipeline that reaches
`Ok` or `Err` after `GAME_READY` is logged with its label and shaders. Cost: a few hundred enum checks a frame.

## Open questions

| Id | Question | Recommendation (built while open) |
| --- | --- | --- |
| B1 | Is 16.7 ms judged on the uncapped frame (vsync off), as the perf runs measure it? | Yes: vsync would hide a frame that would have been late. Provisional. |
