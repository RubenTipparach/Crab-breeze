# Tasks: gpu-instancing

- [x] 0.1 Measure: the perf readout and a trace of the defence take (CPU-bound, per-mesh render work)
- [x] 1.0 Trees merged per tile, one mesh per species; falling trees drawn on their own (measured: entities halved, frame time unchanged)
- [ ] 1.1 Trees as structure-of-arrays in `cb-core`, dirty rows (test: `a_falling_tree_updates_only_its_own_row`)
- [ ] 1.2 The instance buffer, the culling compute pass, indirect draws in the opaque, prepass and shadow phases
- [ ] 1.3 Remove the tree entities; measure before and after
- [ ] 2.1 Buildings: one mesh per block with per-vertex element indices and a per-building element-state buffer read in the vertex stage (no rebuild on damage); measure
- [ ] 3.1 Mech stand-ins: one merged mesh per bone group; measure
- [ ] 4.1 Denser, farther woods (T6); report in `docs/benchmarks/`
