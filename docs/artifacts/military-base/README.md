# Military airfield

Published at https://claude.ai/artifact/WGnzu8uTuBXW2o2z28RDdY (openspec change `military-base`).

One of the island's three military bases at the civil airport's detail and true size, beside a 1.8 m pilot and the
Foreman: the 2.4 km runway and its parallel taxiway, eight hardened shelters (two on the alert apron), a cargo apron
and hangar, the helicopter line, the mech hangars, the tower, the fire station, the fuel farm, munitions bunkers
behind earth berms, HQ, barracks and mess, a radar dome, a surveillance radar and two missile sites inside the fence.
The fighter, the military cargo plane, the gunship and the civil helicopter are the game's models; a fighter taxis and
takes off with its gear coming up, a gunship lifts off and circles, a transport lowers its ramp. Day and night, the taxi
routes and the approach cones, and every building breaks by the airport's rule.

- `index.html` is the published page as built by `python3 tools/mockups/gen_military_base.py` from
  `tools/mockups/military_base_src.html`, the site kits and `tools/mockups/airfield_kit.js` (the airport page's
  runway, taxiway and light functions, shared).
- The data it shows are the models and manifests in `assets/models/aircraft/` (`fighter`, `military_cargo`,
  `military_helicopter`, `civil_helicopter`, built by `tools/blender/build_military_aircraft.py`) and the Foreman; the
  page is published with them beside it as glTF JSON (`docs/mockups/military-base/*.gltf.json`, written by the generator
  and not committed: they are those glbs re-wrapped).
- Shots: `docs/screenshots/military-base/`, by `tools/mockups/shoot_military_base.mjs`.
