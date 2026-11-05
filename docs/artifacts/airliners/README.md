# Airliners

Published at https://claude.ai/artifact/9jJVBqZRxVFoXHzhBAcxts (openspec change `airliner-models`).

The propliner (after the DC-3), the four-jet airliner (after the Boeing 720) and the wide-body (after the L-1011
TriStar) at true size beside a 1.8 m pilot and the Foreman: gear up and down, props, fans and control surfaces,
flaps, nav lights, beacons and strobes at night, the cabin cut away, the aisle and the flight deck from inside, the
damage zones pulled apart with their hit capsules, and a click that shoots a zone off (big chunks of the real model,
the section's seats despawned, every piece landing on the ground).

- `index.html` is the published page as built by `python3 tools/mockups/gen_airliners.py` from
  `tools/mockups/airliners_src.html`.
- The data it shows are the models and their manifests in `assets/models/aircraft/` (`propliner`, `airliner`,
  `widebody`, `.glb` and `.json`, built by `tools/blender/build_airliners.py`) and the Foreman block-out; the page
  is published with them beside it as glTF JSON (`docs/mockups/airliners/*.gltf.json`, written by the generator and
  not committed: they are those glbs re-wrapped).
- Shots: `docs/screenshots/airliners/`, by `tools/mockups/shoot_airliners.mjs`.
