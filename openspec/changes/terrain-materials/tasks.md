# Tasks: terrain-materials

## 0. Mockup
- [ ] 0.1 Island mockup (`docs/mockups/island.html`) shows each biome's layer choice by slope, strata and macro variation with stand-in textures

## 1. Art (cloud session)
- [ ] 1.1 Material Maker graphs for the twenty layers and the strata strip; contact sheet looked at
- [ ] 1.2 Export to `assets/textures/terrain/`, listed in `data/materials/terrain.ron`

## 2. Engine (owner's machine)
- [ ] 2.1 Biome layer table in data, validated (test: `every_biome_names_existing_layers`)
- [ ] 2.2 Biome weights per ground vertex (test: `a_cliff_is_rock_and_a_riverbank_is_mud`)
- [ ] 2.3 Terrain material: two biomes, slope layers, height blend, triplanar, strata, macro, far map
- [ ] 2.4 Measure texture memory and shader cost in the cockpit
