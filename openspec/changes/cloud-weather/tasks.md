# Tasks: cloud-weather

- [x] 0.1 The look in the island mockup: billboards, the six weather systems, rain shafts, storm darkening (`docs/mockups/island.html`, `island_gen.js`), shot in `docs/screenshots/island/`
- [ ] 1.1 `cb_core::world::weather`: the systems as seeded rules producing puffs with a kind, darkness and layer; tuning in `data/world/world.ron` under `weather:` (tests: counts and heights per system, every puff inside its layer, the same seed gives the same sky)
- [ ] 1.2 `Clouds::density_at` reads every kind; rain density under a shaft (test: inside a shaft below its top is raining, outside is not)
- [ ] 2.1 Billboard drawing: one instanced quad per puff, `cloud.wesl` reworked for a sprite (sphere normal, eroded edge), sorted back to front within the draw radius, faded by the depth prepass where they meet the ground
- [ ] 2.2 Rain shafts and cirrus; the storm darkening in the ground and water shaders
- [ ] 2.3 Rain on the glass under a shaft (`cloud-rain`)
- [ ] 3.1 Headless stills (lavapipe): from the ground under a storm, from cockpit height, flying through the deck
- [ ] 3.2 Measured with `CB_NO_CLOUDS` against the same take on the owner's hardware: frame time in fair weather and facing a storm
- [ ] 3.3 Playtest: fly under a storm and into the deck, on Drive
