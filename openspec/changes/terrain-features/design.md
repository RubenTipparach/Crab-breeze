# Design: terrain-features

## 1. Options

| Option | Fixes | Cost | Taken |
| --- | --- | --- | --- |
| Planar texture (today) | nothing on steep ground | - | no |
| Triplanar (`terrain-materials`) | the stretched texture | 3 taps a map on cliffs | improved to biplanar (1) |
| Refine steep cells (2) | shape and ledges on walls | more triangles on steep ground only | yes |
| Density field in feature volumes (3) | faces, overhangs, arches, caves | a mesher, off the main thread; colliders from meshes | yes |
| Voxel terrain everywhere | everything | 41 km by 2.3 km of height: every system that asks "how high is the ground" (rivers, roads, trees, the mech's feet, craters) becomes a 3D query; LOD seams over the whole island | no |
| Rock meshes on cliffs (`rock-formations`) | hero overhangs and arches | instanced, cheap | kept, alongside (3) |

## 2. Biplanar triplanar (step 1)

Per layer, on ground over 30 degrees:
- Take the two largest components of the normal and sample only those two projections (Quilez,
  biplanar mapping). That is 2 taps a map instead of 3, about a third fewer samples on cliffs.
- Blend weight: `w_i = max(|n_i| - 0.5, 0)^8`, multiplied by the layer's height map, so the
  projections meet along rock seams rather than smearing into each other.
- Normals: the tangent-space normal is reoriented per axis with the whiteout blend before mixing.
- A detail normal at 1 m on rock layers.
- Hex-tile stochastic sampling (Mikkelsen 2022): 3 taps of the colour map, so a 300 m wall does
  not repeat.

Worst case, four rock layers biplanar with hex tiling: 4 x (2 x 3 + 2) = 32 samples, against 36 for
plain triplanar today.

## 3. Refined steep cells (step 2)

- **Where:** in the chunk mesher, a cell whose slope is over 40 degrees, within 700 m (step 1) is
  split to 1 m: 64 more vertices along the face, 128 triangles a cell.
- **Shape:** each new vertex moves along the cell's normal by `rock(p) = 6 m x ridged(p / 18 m) +
  strata_ledge(y)`, where the ledge is the same 9-25 m strata table as the colour.
- **Cost:** the island's cells over 40 degrees within 700 m are expected at about 1-3 % of the
  cells in view (to be measured). That is 30,000-90,000 more triangles, and one more pass over steep cells when a
  chunk is built (off the main thread).
- **The height queries** (feet, rivers, trees) keep reading the heightmap, so a refined face is
  drawn, not walked on. Faces over 40 degrees cannot be walked anyway.

## 4. Feature volumes (step 3)

- **Volumes:** boxes in world space, from the seed, laid along:
  - each canyon segment: 300 m wide, from the floor to the rim plus 20 m;
  - each cliff band of the range, where the terrace riser is over 60 m;
  - the north-east sea cliffs: 200 m deep, from the sea floor to the top;
  - each waterfall face: 120 m across, from the foot to the lip.

  Expected at seed 42: about 400 boxes covering about 3 % of the land (to be counted).
- **The field:** `d(p) = p.y - height(p.xz)`, the heightmap as a field, then:
  - strata ledges: `smooth_min` with level slabs every 9-25 m, pushed in and out 2-8 m by noise;
  - overhangs: `smooth_max` with an undercut where a softer stratum sits under a harder one;
  - arches and sea caves: subtraction of capsules from `rock-formations`'s placements;
  - the waterfall grotto: a half-cylinder behind each curtain, 0.3 times the drop deep;
  - rock surface: 3D ridged noise, 3 m at 20 m and 0.8 m at 4 m.

  Each is a pure function of position, so the same seed gives the same cliffs.
- **Meshing:** 32 m cubes, voxel size 1 m within 300 m, 2 m within 700 m, 4 m beyond. Surface nets
  place one vertex a cell at the mean of its edge crossings. That gives about 2 triangles a square
  metre of surface at 1 m, versus marching cubes' 3-5 with more slivers. A cube is meshed on a
  worker thread in about 1-2 ms (32,768 field samples at about 40 ns). LOD seams are stitched
  with Transvoxel's transition cells, so no cracks show between 1 m and 2 m cubes.
- **Joining the heightmap:** the heightmap cells inside a volume are not drawn. The volume's field
  equals the heightmap's at its edges (the shapes fade out over the last 20 m), so the surfaces
  meet.
- **Collision:** a cube's mesh becomes a trimesh collider when a body is within 200 m. The mech's
  feet ask the field instead of the heightmap inside a volume, so a ledge can be stood on.
- **Shading:** the same ground material, step 1's biplanar, the biome weights read from the
  heightmap's point below.

**Cost (expected; measured on the owner's hardware):**

| Item | Value |
| --- | --- |
| Cubes within 700 m in a canyon | about 150 |
| Triangles drawn | about 300,000-600,000 near, falling with LOD |
| Memory | about 40 MB of meshes for the cubes kept |
| Meshing | about 2 ms a cube, off the main thread; flying at 150 m/s meets about 10 new cubes a second |
| Draws | one per cube in view (60-100), batched by material |

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| T8 | How far to go past the heightmap | all three steps, in order; no whole-island voxels | proposed (survey T8) |
| T9 | Surface nets or marching cubes | surface nets (fewer, better triangles); marching cubes only if sharp edges look soft | proposed (survey T9) |
