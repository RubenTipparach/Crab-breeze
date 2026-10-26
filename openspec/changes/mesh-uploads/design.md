# Design: mesh-uploads

## 1. What is known

From the four `streaming-route` runs on the 3070 (`docs/benchmarks/2026-10-26-streaming-3070/`):

| | Per 6.5 min run |
| --- | --- |
| Upload frames of 2 ms or more | 1 638 to 2 114 |
| 16.7 ms or more | 245 to 341 |
| 33 ms or more | 13 to 47 |
| Worst | 53 to 111 ms |
| Meshes added or changed in an upload frame of 10 ms or more | median 2, most 22 |

One example: frame 8 323 of run n1, 76.8 ms, 57.1 ms of it the upload, for 4 meshes added, 2 changed and 5 removed.
Standing still the frame stays under 16.7 ms (0.3 % over); the cost comes with moving fast across the map.

## 2. The instrument

- `hitch_spans::count_assets` (Last, `--features hitch_spans` only): for each `AssetEvent::Added` and `Modified` of a
  `Mesh`, the mesh's bytes (every vertex attribute's bytes plus its indices, as the render world uploads them) go to
  the frame's counts as `kb:<name>` and `mesh_kb`, in KiB. The name is that of an entity whose `Mesh3d` holds the mesh;
  `?` when none does yet. The names are found by one pass over the `Mesh3d` entities, only on a frame that sends
  meshes.
- `tools/perf/hitch_causes.py` prints, over the frames whose upload took 10 ms or more, the `kb:` counts of that frame
  and the frame before (the render world uploads what the main world added a frame earlier), by name: frames, KiB in
  all, KiB a frame.
- Its cost: nothing in a build without the feature. With it, a walk over the drawn meshes (a few thousand) on frames
  that send meshes, about 0.1 ms.

## 3. The candidates, to be confirmed or ruled out by the measurement

| Mesh | Size | How often | Fix if it is the cost |
| --- | --- | --- | --- |
| Tree tiles within 600 m (full detail; an oak 1 602 vertices, a birch 810, a pine 186; position, normal, colour, UV: 48 bytes a vertex) | a tile of a few hundred trees: several MB | up to 2 a frame (`TREE_TILES_SHOWN_PER_FRAME`), and every tile crossing the 600 m line is rebuilt | a byte budget a frame instead of a tile count; split a tile's mesh by species or by quarter; draw full-detail trees as instances of one mesh per species (no upload at all) |
| Ground LOD nodes (`chunk_cells` 64; 85 meshes, 739 840 triangles in all) | about 8 700 triangles a node, a few hundred KB | up to 6 a frame (`builds_per_frame`) | fewer a frame when they are large |
| Building far facades and dressing | to be measured | as cities come into range | to be measured |

The fix is chosen from what the instrument names, measured old against new on the 3070 (CLAUDE.md section 7), and
written into this design before code.

## Open questions

| Id | Question | Recommendation (built while open) |
| --- | --- | --- |
| U1 | Is a frame budget in bytes, rather than a mesh count, the rule for every streamed mesh? | Yes, if the measurement shows sizes vary by 10 times or more. Provisional. |
