# Tasks: island-biomes

## 0. Mockup first
- [ ] 0.1 `docs/mockups/island.html`: the island at true scale with the range, desert and canyons, jungle, swamps, waterfalls, beaches, cliffs, archipelago, biome map, clouds; flyable
- [ ] 0.2 Shots (cloud container), owner looks and answers W18-W23

## 1. Core (`cb-core::world::terrain`, `world::biome`)
- [ ] 1.1 Range, passes, rain-shadow plateau (tests: peak heights, pass heights, sample cost)
- [ ] 1.2 Canyons and strata, mesas, dunes, cliffs, archipelago, swamp flats
- [ ] 1.3 Biome weights (tests: every biome present on seed 42; borders blend)
- [ ] 1.4 Cities on gentle low ground; capital on the northern plain; golden seed updated
- [ ] 1.5 Waterfalls recorded on steep river drops

## 2. App
- [ ] 2.1 Ground drawn with biome weights for `terrain-materials`
- [ ] 2.2 Waterfalls: curtain, spray, mist, pool; murky swamp water
- [ ] 2.3 Measured; playtest flying a mission route over every biome, on Drive
