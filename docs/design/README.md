# Crab Breeze design hub

## Survey

Open questions for the owner live in the survey page, never in chat (CLAUDE.md section 8):
https://claude.ai/artifact/KndrEC5xE7wBmAd8WM4aTP (source: `docs/design/survey.html`; answers in
its `answers` collection, read with `ArtifactData`). Keep editing this one rather than starting
another.

Question id prefixes: `G` game and feel, `M` the mech, `C` the cockpit, `W` world (cities,
buildings, people), `T` tech and tools, `P` playtesting.

## Changes

| Change | State | Presents |
| --- | --- | --- |
| `playtest-pipeline` | proposed | The scripted pilot, output lines, OBS recording, Drive upload, reports |
| `cargo-workspace` | built | Rust workspace on Bevy 0.20.0: `cb-core`, `cb-app`, `cb-tools`; seeded RNG; data loader |
| `procgen-world` | built, playtest pending | Seeded world: hills, cities with street grids, box buildings 2-60 storeys, highways |
| `mech-rig` | built, playtest pending | Stand-in mech on the rig contract, digitigrade walk, torso twist, glb drop-in |
| `pilot-boarding` | built, playtest pending | Pilot on foot in first person, boarding up the lift into `EYE_pilot`, climbing out |

## Mockups

| Page | Published | Built by |
| --- | --- | --- |
| Crab Breeze Mechs (`docs/mockups/mech_designs.html`): the three chassis candidates at true scale, cockpit view, rig test | https://claude.ai/artifact/B5tX8ycUR2tLBknKyMb8bc | `tools/mockups/gen_mech_designs.py` from `assets/models/mech_*.glb` |
