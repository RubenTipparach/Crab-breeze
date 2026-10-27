# Proposal: frame-budget

## Why

Owner, 2026-10-26: "Let's focus on getting no frames under 60fps, then we'll focus on 90", "ensure that when you run
perf, everything is always above 60 fps". CLAUDE.md section 7 makes it a rule: after `GAME_READY`, no frame of a
release build may take longer than 16.7 ms on the scripted perf routes (`streaming-route`, `free-roam-fight`). The
Mac's last run (`docs/benchmarks/2026-10-26-streaming-mac/report.md`) had 48 frames over it: waits on the GPU, mesh
uploads, pipeline compiles, music starts and a particle compile. The desktop's had hundreds of upload frames.

## What changes

1. **A gate**: the game judges every frame after `GAME_READY` against 16.7 ms (`cb_core::perf::FrameBudget`) and prints
   one `PERF_GATE pass|fail` line on the way out; `--perf-gate` makes it exit non-zero on a frame over, and
   `tools/perf/perf_gate.py` plays the routes and reports. Pipelines that compile after `GAME_READY` are logged
   (`pipeline late:`) and counted on the same line.
2. **Each cause of a frame over budget fixed at its cause**, biggest first, measured old against new on the Mac, with
   the desktop's rerun commands in `docs/benchmarks/2026-10-26-60fps-mac/report.md`. Each fix is written into
   `design.md` as it lands.

## Not in this change

What a tree or a building looks like: a fix that would change the look is asked first.
