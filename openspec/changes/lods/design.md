# Design: lods

## Building the levels

In the Blender generators (`build_mechs.py`, `build_mech_blockouts.py`, `build_alien_mechs.py`), after a model is
finished and checked, one shared step (`tools/blender/lods.py`, the one implementation) makes, for every mesh node:

| Level | How | Target share of LOD0 | Small parts (largest side under) |
| --- | --- | --- | --- |
| LOD0 | as built | 1 | kept |
| LOD1 | Decimate (collapse, symmetric, triangulated) after a planar dissolve at 2 degrees | 0.35 | 0.25 m dropped |
| LOD2 | the same at a lower ratio | 0.10 | 0.8 m dropped |
| LOD3 | the part's convex hull, decimated to at most 24 triangles | under 0.03 | 2.0 m dropped |

Each level is a sibling mesh node under the same parent, named `<node>_LOD<n>`, sharing its materials, so it rides the
same bone. UVs and normals come through the decimation; the plating's normal map hides most of the loss at LOD1 and
LOD2. The manifest records each level's triangles; the build fails a model whose level is over budget.

## Budgets (triangles per model)

| Class | LOD0 | LOD1 | LOD2 | LOD3 |
| --- | --- | --- | --- | --- |
| Detailed mech (24 m) | 30,000 | 10,000 | 3,000 | 600 |
| Crab mech (detailed) | 30,000 | 10,000 | 3,000 | 600 |
| Big Boy (30 m) | 40,000 | 14,000 | 4,000 | 800 |
| Skitter, Snapper | 12,000 | 4,000 | 1,200 | 300 |
| Block-out stand-in | 10,000 | 4,000 | 1,200 | 300 |

## Switching

`cb-app` puts a `VisibilityRange` on every LOD mesh entity when a model is bound (the binder already walks the parts).
Distances are in model heights `h`, so a level changes at about the same size on screen (1080 lines, 60 degrees
vertical field of view: a model `h` tall at `8h` is about 125 px tall):

| Level | Shown from | to | Crossfade |
| --- | --- | --- | --- |
| LOD0 | 0 | 6h | 0.5h |
| LOD1 | 6h | 18h | 1h |
| LOD2 | 18h | 50h | 3h |
| LOD3 | 50h | 150h | 10h; beyond, radar only |

For a 24 m mech: full detail inside 144 m, LOD1 to 432 m, LOD2 to 1.2 km, LOD3 to 3.6 km. Tuning in
`data/lods.ron`, units in the keys. The pilot's mech, and any model the camera is inside, keep LOD0.

## Cost

- Twelve 24 m enemies spread between 150 m and 1.5 km: about 12 x 3,000 = 36,000 triangles, against 12 x 24,000 =
  288,000 at full detail. Draw calls do not fall (a part is still a draw); `gpu-instancing` is the change for that.
- Crossfade is a dithered pass over two levels for the margin's width: twice the triangles of the smaller level for
  that band only.
- Disk: about 0.5 times LOD0's size again per model.
- To be measured on the owner's machine: the densest fight, LODs on against off, p50, p95 and p99 (CLAUDE.md section 7).

## Open questions

| Id | Question | Recommendation (provisional) |
| --- | --- | --- |
| L1 | Switch distances in model heights, as above? | Yes |
| L2 | LOD3 as a convex hull per part, or one merged hull for the whole model (one draw)? | Per part, so a lost limb still drops its LOD3 |
| L3 | Do shadows follow the level the camera sees, or always use LOD2? | LOD2 for shadows beyond LOD0's range |
| L4 | Buildings and city blocks next, in their own change? | Yes, after the combatants |
