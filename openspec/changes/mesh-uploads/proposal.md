# Proposal: mesh-uploads

## Why

The owner (2026-10-26): "Let's focus on getting no frames under 60fps, then we'll focus on 90 afterwards. The biggest
hitches is loading and unloading sections of the map it seems." On the RTX 3070 desktop the largest streaming cost is
the render world copying new and changed meshes to the GPU (`allocate_and_free_meshes`): 16.7 ms or more in 245 to 341
frames of each 6.5 min `streaming-route` run, 33 ms or more in 13 to 47, up to 111 ms
(`docs/benchmarks/2026-10-26-streaming-3070/report.md`). The Mac saw 1 to 9 such frames, up to 26 ms. The slow upload
frames add or change only 2 meshes (median; 22 at most), so a few large meshes are the cost. Which meshes is not yet
known.

## What changes

1. **First, a measurement instrument** (CLAUDE.md section 4): the `hitch_spans` build adds, on every frame that adds or
   changes meshes, each mesh's vertex and index bytes to `kb:<name>` (the `Name` of an entity drawing it) and to
   `mesh_kb`; `tools/perf/hitch_causes.py` sums them by name over the frames whose upload took 10 ms or more. That
   names the meshes and their sizes.
2. **Then, once measured, a fix for the meshes it names**, designed here before any code: the candidates and how each
   would be spread or shrunk are in `design.md` section 3. Nothing in the game changes until the measurement picks one.

## Not in this change

The Mac session's fixes (ambience bed seek, tree ring budget, car transforms, the crater patch swap), car wheel
transforms and the jet leg stopping mid-air: the Mac's (`openspec/changes/streaming-hitches`).
