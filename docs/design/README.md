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
| `mech-boarding` | proposed | The sealed cockpit and its all-view screen, the mech sitting down, the staged hatch with steam and mist |
| `pilot-boarding` | built, playtest pending | Pilot on foot in first person, boarding up the lift into `EYE_pilot`, climbing out |
| `terrain-water` | built, in the 2026-10-24 world take | Rivers and lakes from the seed, wading drag, ripples, spray and steam |
| `terrain-materials` | proposed | PBR ground layers splatted by slope, water, city and scorch (textures from the art side) |
| `vegetation` | built (procedural trees until models arrive) | Seeded woods, instanced LODs, mechs snapping trees that fall under Avian physics |
| `terrain-deformation` | built | Craters from explosions, dirt bursts, scorch; one ground query |
| `mech-damage` | built | Parts shot off one by one: arms, head (backup cameras), legs (limp) |
| `clouds` | built (no shadows or raindrops yet) | Cheap fly-through cumulus, in-cloud fog, cloud shadows; camera raindrops later |

| `jet-flight` | built | Jets limited by heat, landing on roofs, flame; for flying into clouds |

| `perf-bench` | built | `--bench world|fight`: offscreen 1080p frame time by beat; baseline in docs/benchmarks |
| `weapons` | built | Autocannon and cannon, ballistic shots, hits by part |
| `enemy-mechs` | built | AI pilots through the same controls; the city fight take |
| `building-destruction` | built | Buildings of cells that break and collapse |
| `debris-physics` | built | Avian 0.8 for chunks and wrecks on the drawn ground |

Requests between the Mac and cloud sessions: `docs/design/requests.md`.

## Mockups

| Page | Published | Built by |
| --- | --- | --- |
| Mech Cockpits (`docs/mockups/cockpits.html`): the three interiors (wall, harness, jockey) on their own: sit at the pilot's eye with the world on the screens, open the inner doors, orbit the pod beside a pilot. Was the Breeze Cockpit page | https://claude.ai/artifact/AqMpKdiBjn9e1uQPT1c1aj | `tools/mockups/gen_cockpit_page.py` from `assets/models/cockpit_<variant>/`, published with them as files |
| Crab Breeze Mechs (`docs/mockups/mech_designs.html`): on top, the 24 m low-poly redesign (`tools/blender/build_mech_blockouts.py`, legs against torso measured); below, the detailed 12 m round: Foreman, Stevedore and Ironhorse at true scale; each kneels, opens its chest with steam and drops its ladder on its cockpit interior (wall, harness, jockey); board it in first person or sit in each cockpit | https://claude.ai/artifact/B5tX8ycUR2tLBknKyMb8bc | `tools/mockups/gen_mech_designs.py` from `assets/models/mech_<id>.glb` and `assets/models/cockpit_<variant>/`, published with them as files |
| Alien Crab Mechs (`docs/mockups/alien_mechs.html`): the three enemy crab machines (Skitter 6 m, Snapper 12 m, Big Boy 30 m) at true scale beside the Breeze | https://claude.ai/artifact/DAdKyPWjNyjur74vQaZTA8 | `tools/mockups/gen_alien_mechs.py` from `assets/models/alien_*.glb` (`tools/blender/build_alien_mechs.py`, rig in `docs/design/alien-rig.md`) |
