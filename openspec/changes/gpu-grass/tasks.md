# Tasks: gpu-grass

- [x] 1.1 Tuning in `data/world/grass.ron`, loaded and validated by `GameData` (`cb_core::grass::GrassConfig`), checked against the world (lattice, full-detail ring)
- [x] 1.2 The shared cover rule `WorldPlan::cover_at`, the trees moved onto it (golden seed unchanged); `terrain::cell_height` and `CraterConfig::reach_m` as single copies
- [x] 1.3 Tiles and levels as pure functions (`TileGrid::within`, `GrassConfig::lod_for`), tested
- [x] 1.4 The blade mesh per level (`BladeMesh`), its counts tested against design section 3
- [x] 1.5 The tile bake (`bake_tile`, `TileBake::mask_craters`, `pack`): heights from the lattice, no grass in a city, on a road or in water, thinning on slopes, cleared by craters, tested
- [x] 2.1 `assets/shaders/grass.wesl`: blades built in the vertex shader (Bezier, wind, clumps, edge-on turn, rounded normals, LOD crossfade, pressing), PBR-lit fragment; compiled against Bevy 0.20's `bevy_pbr` with naga in `cargo test`, its uniform's size checked
- [x] 2.2 `cb-app::grass`: the material, one entity a tile, slots in one storage buffer, bakes off the main thread, the pushers, the grass render layer seen only by the pilot's eyes
- [x] 2.3 `CB_NO_GRASS=1` leaves the grass out (benchmark variant)
- [ ] 3.1 Benchmark on the owner's hardware: `--bench world` and `--bench fight` with and without `CB_NO_GRASS=1`, interleaved; report in `docs/benchmarks/`; replace design section 6's estimates
- [ ] 3.2 Playtest: grass seen from the pilot's eyes on foot and from the cockpit, pressed by the mech's feet, cleared by a crater; tune colours against the ground (open question 6)
- [ ] 3.3 Move the requirements into `openspec/specs/gpu-grass/spec.md` once the playtest and the benchmark pass
