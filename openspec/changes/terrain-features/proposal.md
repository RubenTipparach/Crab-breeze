# Proposal: terrain-features

## Why

Owner, 2026-10-24, looking at the island mockup: "Is there a way to avoid that ugly height map look
by using actual cube marching? Better trippanar shaders?"

The ground is a heightmap (`island-world`): one height per 8 m cell at its finest, drawn by a
quadtree. A heightmap holds one height per (x, z), so it cannot have:
- **Faces.** A 300 m canyon wall or waterfall face falls inside one 8 m cell. It is drawn as a
  pair of triangles stood on end, with no vertex on the face for shape or shading.
- **Overhangs, arches and caves,** which need two heights at one (x, z).
- **Texture on steep ground:** planar mapping from above stretches it down a slope. Triplanar
  (`terrain-materials`) fixes the texture, but not the missing shape.

The island's best places are exactly these: the canyons, the range's cliff bands, the sea cliffs and
the waterfalls.

## What changes

Three steps, cheapest first, each usable alone:

1. **Better triplanar on all ground over 30 degrees** (shader only):
   - biplanar mapping (two taps chosen by the normal, instead of three);
   - normal maps reoriented per axis (whiteout blend), so rock relief lights correctly on a wall;
   - a height-blended axis weight, so the axes meet in rock seams rather than a smear;
   - a detail normal at 1 m;
   - hex-tile stochastic sampling, so a 300 m wall does not repeat.
2. **Refined steep cells:** where a ground cell is steeper than 40 degrees, the chunk mesher splits
   it to 1 m and pushes each new vertex out along the normal by a 3D rock noise (the strata of
   `terrain-materials` decide the ledges). Walls get shape, ledges and gullies. Still one height per
   point, no overhangs.
3. **Feature volumes meshed from a density field:** inside volumes laid over the canyons, cliff
   bands, sea cliffs and waterfall faces, the ground is a signed distance field instead of a
   height: the heightmap's field, combined with carved and added shapes (strata ledges,
   overhangs, arches, sea caves, the waterfall's grotto). It is meshed in 32 m cubes at 1 m (near)
   to 4 m (far) with **surface nets**: marching cubes' dual, fewer and better-shaped triangles,
   with seams between LODs stitched as Transvoxel does. Everything outside the volumes stays a
   heightmap, so roads, rivers, trees and the quadtree are unchanged.

The island mockup gets a **Cliff** view showing step 3 on one canyon wall beside the heightmap.

## Out of scope

Voxel terrain for the whole island (weighed in the design and not recommended), digging tunnels,
and changing `terrain-deformation`'s craters (they stay on the heightmap outside the volumes).
