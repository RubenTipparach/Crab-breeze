# Tasks: mech-collision

- [ ] 1.1 `gen_mech_data.py`: each bone's boxes (voxelise, flood, merge, drop small) into `data/mechs/<id>.ron` `body`
- [ ] 1.2 The rope's foot clear in the kneel, checked in the generator for every roster mech
- [ ] 1.3 `cb-core` `MechConfig.body` (validated: finite, min under max, a contract bone); `cb-tools validate` against the glb
- [ ] 1.4 `cb-core` `world::body`: slab clip, convex hull, `push_out_of_convex`; tests read as sentences
- [ ] 1.5 `world::Scene.bodies`; the pilot's walk pushed out of mechs; tests: walking into a foot stops at its face,
      walking under a standing thigh passes, the rope's foot is reachable
- [ ] 1.6 `cb-app`: posed boxes from each nearby mech's bones in the fixed step
- [ ] 1.7 Headless still: the pilot walked into the kneeling Foreman's foot stops outside it
- [ ] 1.8 Playtest on the owner's machine: walk round every roster mech, kneeling and standing, and board it
