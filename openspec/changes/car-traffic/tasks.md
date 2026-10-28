# Tasks: car-traffic

- [x] 1.1 Lane graph from streets and highways, deterministic, tested (no dead ends, lanes on the road surface)
- [x] 1.2 `cb-core::traffic`: following, junction right of way, turns, fear, crushing by foot strikes, tested with golden seeds
- [x] 1.4 Traffic lights on the arterials (survey W39, 2026-10-26: "Lights on the arterials"): lit junctions and phases
  in the lane graph, `traffic::light` from `lights` in `traffic.ron`, cars stopping on red, the front-car turn fix;
  tested (no deadlock in ten minutes, cars stop on red); block-out heads drawn (`cb-app::traffic_lights`), not yet
  seen in a playtest
- [x] 1.5 The 1 km ring (owner, 2026-10-26: "car should be 1000m") fixed what it exposed: cars placed only on the
  stretch of a lane in reach (highways filled to half), and a car held at red keeps its place for the room past
  the junction (seed 9 starved a turning car 174 s); tested, golden seed 11 re-recorded
- [x] 1.3 `data/world/traffic.ron` (densities, speeds, IDM constants), validated
- [x] 2.1a The block-outs in `tools/blender/build_cars.py`, checked by `cb-tools validate` and shown on
  `docs/mockups/cars.html`; approved by the owner (2026-10-26: "Approved all of the stand ins", survey W36)
- [ ] 2.1 `tools/blender/build_cars.py`: the 7 kinds modelled in detail, with LODs, crushed variants and PBR sets
  (follow-up to the approved block-outs, 2.1a)
- [x] 2.2 Spawn and despawn out of view; instanced drawing; wheels
  (drawn from the block-outs, whose shared meshes Bevy batches per kind; wheels turn with speed and steer)
- [x] 2.3 Hits and burning wrecks through the weapons' hit rules
- [x] 2.4 Harm to a car with a driver in it, by the player's fire, feet or car, charged to the nearest base's
  standing through `cb_core::civil_standing` (survey W37); tested (`a_car_costs_standing_only_with_a_driver_in_it`)
- [ ] 3.1 Measure downtown traffic on the 3070
  (the simulation alone is measured on the CPU: `cb-tools traffic-bench`, design section 6; frame time is not)
- [ ] 3.2 Playtest: walk and stride through traffic, shell a street
