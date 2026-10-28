# Tasks: people-vat

- [x] 1.1 Packs pinned and fetched outside the repository (`tools/deps/character_packs.json`, `fetch_character_tools.py`)
- [x] 1.2 The body generator (`tools/blender/build_people.py`, `people.json`): two MPFB bodies at 1.75 m, regions and shades, two levels with budgets, UAL clips retargeted, the wave authored, the VAT bake and manifest; `--verify` byte for byte
- [x] 1.3 `cb_core::people_vat`: manifest and `.vat` reading and checks; the shipped bake against its manifest and glbs; the budget refused when over
- [x] 1.4 `cb_core::civilians::anim`: the clip rule, cadence, crossfade, the fall held, start phases; tested
- [x] 2.1 The VAT material and shaders (main and prepass), compiled in `cargo test`
- [x] 2.2 `cb-app::people_vat`: chunk entities, the people buffer, levels by distance, palettes; replaces the stand-in pool and the merged crowd mesh; drivers who leave their cars drawn as people
- [ ] 3.1 Same-session A/B against the stand-ins at the full crowd on a city route; report in `docs/benchmarks/`
- [x] 3.2 In-game stills (walking, sitting, crossing, greeting, at distance; `CB_PEOPLE_STILLS`, `docs/screenshots/people_vat/`) and the orbitable review page with the pilot and a mech for scale (`tools/mockups/gen_people.py`)
- [ ] 3.3 Playtest; then move the requirements into `openspec/specs/`
