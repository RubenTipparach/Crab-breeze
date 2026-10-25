# Proposal: building-gpu-state

## Why

Owner, 2026-10-24: "Yea move building updates to gpu". Every hit on a building rebuilds meshes on
the main thread: the damaged storeys' bands (frame, rooms, furniture, one mesh per surface) and,
when it is dressed, its whole skin. Even after the tangent fix, `redraw_buildings` logs 48-150
slow frames a combat run and `dress_near_player` about 100 (`docs/benchmarks/2026-10-24-surfaces-mac/`).
The structure simulation itself is never slow.

## What changes

- A damaged building's frame, rooms, furniture and skin are meshed **once**, every vertex carrying
  the index of the element it hangs on (its column, wall or slab; furniture and partitions the
  slab under them) in the vertex colour's alpha.
- Each such building has a small storage buffer, one word per element: standing or not. A hit
  rewrites that buffer (a few kilobytes), never a mesh.
- The building materials become an extension of `StandardMaterial` whose vertex shaders (main
  pass, and the prepass that also draws shadows and SSAO's normals) read the element's word and
  collapse a broken element's triangles, so a broken part disappears in every pass at once.
- Bands of twelve storeys go: a building's meshes are its whole frame, per surface.

## Out of scope

The structure simulation (stays in `cb-core`, deterministic, on the CPU), the debris and rubble,
far buildings' plain boxes, instancing (`gpu-instancing`), building meshes off the main thread
(a later step if the one-time build shows in a trace).
