# Tasks: terrain-features

- [ ] 0.1 The island mockup's Cliff view: one canyon wall meshed from the field (surface nets) beside the heightmap
- [ ] 1.1 Biplanar mapping with whiteout normals, height-blended axes, a detail normal and hex tiling in the ground shader (`terrain-materials` section 2)
- [ ] 1.2 Headless stills of a canyon wall before and after; measured in the densest canyon on the owner's hardware
- [ ] 2.1 Steep-cell refinement in the chunk mesher, with the rock and strata offset (tests: only cells over 40 degrees are split; the edges match the neighbours)
- [ ] 3.1 `cb_core::world::features`: the volumes from the seed and the field as a pure function (tests: the field equals the heightmap at a volume's edge; the same seed gives the same field)
- [ ] 3.2 Surface nets mesher with Transvoxel seams (tests: a closed field gives a watertight mesh; no cracks between LODs)
- [ ] 3.3 Drawing: the cubes in the quadtree's place inside a volume; colliders near bodies; the feet ask the field inside a volume
- [ ] 3.4 Playtest: walk a canyon ledge and fly under an overhang and behind a waterfall, on Drive
