# Tasks: rock-formations

- [x] 0.1 In the island mockup: stacks, arches, hoodoos, boulders at true scale (`docs/mockups/island.html`, `buildRocks`)
- [ ] 1.1 Blender kit with LODs, PBR maps, manifest budgets; contact sheet looked at
- [x] 0.2 In the game, block-out stage (owner, 2026-10-25: "add stacks and arches?"; low poly first): the mockup's placement ported to `cb_core::world::rocks` (70 stacks and 7 arches off the north-east cliffs, stacks round the cliffed islets, up to 500 hoodoos on the canyon floors clear of water, roads and cities), the mockup's displaced kit shapes drawn flat-shaded with strata (`cb-app::rocks`), bodies walking round them (`rocks::push_out`; tests in both crates). Boulders not yet
- [ ] 1.2 Placement per tile by biome (tests: deterministic; none in water or on roads)
- [ ] 1.3 Instanced drawing and colliders; measured
