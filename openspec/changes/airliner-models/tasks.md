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
- [x] 3.2 The owner looks at the page (2026-10-30: "implement airplanes!" after asking for a solid lower half, debris,
  bigger chunks, half as many, seats despawning, and pieces landing on the ground)

## 4. The game (after 3.2)

- [x] 4.1 Kinds in `air_traffic.ron` (A1): `propliner` and `widebody` added, `airliner` resized to the 720 model; all three
  fly the routes between the bases (test: `the_shipped_kinds_are_the_ones_the_owner_asked_for_at_true_size`)
- [x] 4.2 The models drawn in place of the stand-in (`cb-app::airliners`): levels of detail by name, the rig from the
  flight (gear below `gear_below_m`, flaps by phase, ailerons and elevators by bank and pitch, props and fans at their
  rpm), the propliner tail down on its wheels, parked aircraft gear down and engines stopped; not yet seen in a playtest
- [x] 4.3 Lamps: nav lights steady, beacons and strobes flashing on the manifest's patterns (offset by aircraft),
  landing lights with the gear down, beacons off when parked (tests: `the_gear_folds_on_its_keys_and_the_strobe_flashes_twice`);
  emission only, no cast light (night rule N3)
- [x] 4.4 Damage zones in `cb_core::air_traffic` (`model`, `Aircraft::hit_part`): capsules per zone from the manifest,
  health by share, the part tree, an engine lost slows it, any other zone or every engine downs it (tests:
  `a_shot_engine_breaks_away_and_the_airliner_flies_on_slower`, `a_shot_through_a_wing_root_brings_the_wing_and_its_engines_away_and_the_airliner_down`,
  `breaking_a_wing_root_takes_the_outer_wing_and_its_engines_with_it`, `the_propliner_stands_tail_down_on_its_models_rest`)
- [x] 4.5 A zone shot away leaves the aircraft as a falling piece of its own mesh that lands and lies (240 s), its
  fit-out despawned, with debris chunks at the hit and the joint; a crash strews every zone so instead of boxes
- [x] 4.6 `cb-tools validate` checks each zoned model: every zone, pivot and lamp node its manifest names, its length,
  span, height and the ground under its gear against its data, its budgets
- [ ] 4.7 Playtest and measure on the owner's machine
