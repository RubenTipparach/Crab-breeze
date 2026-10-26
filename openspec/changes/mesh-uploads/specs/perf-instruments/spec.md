## ADDED Requirements

### Requirement: A hitch_spans build names the meshes each frame sends to the GPU
A build with the `hitch_spans` feature SHALL add, for every mesh added or changed in a frame, its vertex and index
bytes in KiB to the frame's counts as `kb:<name>` (the `Name` of an entity drawing it, `?` when none does) and to
`mesh_kb`, so a `hitch:` line says which meshes the next upload carries and how large they are.

#### Scenario: A tree tile goes on screen
- **WHEN** a tree tile's mesh is added in a frame of a `hitch_spans` build
- **THEN** that frame's `hitch:` line carries `kb:tree tile` with the mesh's size in KiB, and `mesh_kb` at least as
  large (measured on the 3070's `streaming-route`)
