# Proposal: cargo-workspace

## Why

Every later change needs somewhere to live. CLAUDE.md sections 2 and 6.2 fix the shape: Rust stable,
Bevy pinned exactly, a simulation core with no engine in it, the Bevy game on top, and tools beside
them. Pale Blue Dot (`pbd-core`, `pbd-app`) is the reference split.

## What changes

- A Cargo workspace with `bevy = "=0.20.0"` (survey `T1`, provisional) and three crates:
  `cb-core` (pure simulation, no Bevy), `cb-app` (the game, binary `crab-breeze`) and `cb-tools`
  (command-line tools, starting with `cb-tools validate data/`).
- The two shared foundations every rule leans on, in `cb-core`: a seeded, forkable random stream
  (`SimRng`) and one RON loader that rejects unknown keys and non-finite numbers and names the
  file and field when it fails.
- Workspace lints: warnings are errors under clippy; `cargo fmt` clean.

## Out of scope

Any game feature. The world, the mech and the pilot are `procgen-world`, `mech-rig` and
`pilot-boarding`.
