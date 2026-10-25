# Tasks: island-biomes

## 0. Mockup first
- [ ] 0.1 `docs/mockups/island.html`: the island at true scale with the range, desert and canyons, jungle, swamps, rivers and lakes, beaches, cliffs, archipelago, biome map, clouds; flyable
- [ ] 0.2 Shots (cloud container), owner looks and answers W18-W23

## 1. Core (`cb-core::world::terrain`, `world::biome`)
- [x] 1.1 Range, passes, rain-shadow plateau: the mockup's generator ported line for line to `cb_core::world::island` (noise, random stream, spine, coast, range, plateau), and `terrain::Shape` built on it (owner, 2026-10-25: "I wanted this island"); tests: seed 42 matches the mockup's `baseHeight` at 30 points within 5 cm (`island::tests::seed_42_is_the_mockups_island`), peaks through the clouds, the plateau east of the range
- [x] 1.2 Canyons (383 cuts at seed 42, as the mockup), mesas, dunes, sea cliffs, archipelago (32 islets); swamps as a biome at the western mouths of the game's own rivers (not yet flattened into the ground)
- [x] 1.3 Biome weights: the mockup's ten biomes and moisture (`biome`), feeding the grass, the trees, the ground colour and the map; desert and canyon 25.7 % of the land (`a_quarter_of_the_land_is_desert`)
- [x] 1.4 Cities by the game's rules on the new land (13 at seed 42), the capital within 9 km of the centre on low ground; golden seed updated
- [ ] 1.5 Lakes (partly): the game's drainage lakes (29 at seed 42, highways routed round them); the mockup's tarns at the sources not ported. A waterfall's plunge pool is dug only where the ground can hold its water

## 2. App
- [x] 2.1 Ground drawn with the biome weights: the mockup's surface layers (flat, slope, cliff, wet per biome, canyon strata, patch blend, macro variation) as vertex colours over a neutral grain from the ground texture; the per-pixel layers are `terrain-materials`
- [ ] 2.2 Lakes drawn with the water material; murky swamp water
- [ ] 2.3 Measured; playtest flying a mission route over every biome, on Drive
