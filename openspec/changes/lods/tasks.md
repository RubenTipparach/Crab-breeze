# Tasks: lods

## 1. Build

- [ ] 1.1 `tools/blender/lods.py`: per-node LOD1 to LOD3 (planar dissolve, collapse decimate, hull), small parts dropped by size
- [ ] 1.2 Call it from `build_mechs.py`, `build_mech_blockouts.py` and `build_alien_mechs.py`; triangles per level in the manifests
- [ ] 1.3 Budgets per class; the build fails a level over budget

## 2. Game

- [ ] 2.1 `data/lods.ron`: level ranges and margins in model heights; validated on load
- [ ] 2.2 Bind: a `VisibilityRange` on every `_LOD<n>` entity from the model's height; the pilot's mech keeps LOD0
- [ ] 2.3 `cb-tools validate`: every shipped combatant glb has all four levels within budget

## 3. Check

- [ ] 3.1 Headless stills of one mech at each level's distance, and the crossfade
- [ ] 3.2 Frame time on the owner's machine in the densest fight, LODs on against off
