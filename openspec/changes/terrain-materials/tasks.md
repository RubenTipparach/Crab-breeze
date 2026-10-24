# Tasks: terrain-materials

## 1. Art (cloud session)
- [ ] 1.1 Material Maker graphs for the nine sets; contact sheet looked at
- [ ] 1.2 Export to `assets/textures/terrain/`, listed in `data/materials/terrain.ron`

## 2. Engine (owner's machine)
- [ ] 2.1 Layer rules in data, validated (test: `terrain_weights_sum_to_one`)
- [ ] 2.2 Splat weights per vertex from the rules (test: `a_cliff_is_rock_and_a_riverbank_is_mud`)
- [ ] 2.3 Terrain material, triplanar rock, height blend
- [ ] 2.4 Measure texture memory and shader cost in the cockpit
