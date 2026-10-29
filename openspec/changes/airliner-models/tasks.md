# Tasks: airliner-models

## 1. Write-up and reference

- [x] 1.1 Blueprints: NASA public-domain three-views, `docs/reference/airliner-blueprints/` (owner, 2026-10-29: "approve all")
- [x] 1.2 Proposal, design, spec (this change); `openspec validate --all`

## 2. Models (`tools/blender/`)

- [x] 2.1 `aircraft_kit.py`: the freighter's primitives moved into one kit; the freighter rebuilds byte-identical
- [x] 2.2 `build_airliners.py`: propliner, airliner, widebody; hollow fuselage, window holes, wings, tail, engines, props
- [x] 2.3 Rig: gear legs, doors, props, fans, control surfaces and flaps as pivots; the manifest's `rig`
- [x] 2.4 Lights: nav, beacons, strobes, landing, logo; `LAMP_*` empties
- [x] 2.5 Interiors: flight deck, floors, seats, bins, bulkheads, galleys
- [x] 2.6 Damage zones: cuts at the joints, capped with `torn`, capsules and the part tree in the manifest
- [x] 2.7 LOD0-2 within budget, PBR materials, manifests, byte-reproducible

## 3. Mockup

- [x] 3.1 `docs/mockups/airliners.html` from `tools/mockups/gen_airliners.py`: true scale, pilot and Foreman, gear,
  props, lights day and night, interior cutaway, zones pulled apart; published, linked in `docs/design/README.md`
- [ ] 3.2 The owner looks at the page

## 4. The game (after 3.2)

- [ ] 4.1 Kinds in `air_traffic.ron` (A1), routes for the propliner and the wide-body
- [ ] 4.2 Draw the models in place of the stand-in; LODs; gear, flaps, props and surfaces from the flight
- [ ] 4.3 Lights in the game, with the night rules
- [ ] 4.4 Damage zones in `cb_core::air_traffic`: capsules, health, the part tree, effects on the flight; tests
- [ ] 4.5 Detached zones and the wreck as debris of their own meshes
- [ ] 4.6 `cb-tools validate` checks each model against its data
- [ ] 4.7 Playtest and measure on the owner's machine
