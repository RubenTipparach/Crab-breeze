# Proposal: gpu-grass

## Why

Owner, 2026-10-24: "Add detailed grass, make sure it's all gpu grass. Looks up best practices for
grass on bevy, bench mark it". Today the island's open ground is a flat vertex-coloured mesh with a
grass texture: from the pilot's eye at 1.7 m, and from the cockpit at about 10 m, the ground reads as
carpet. The game is CPU-bound on per-entity render work (`gpu-instancing` measured it), so grass
cannot be entities or CPU instance rows: hundreds of thousands of blades must cost the CPU nothing
per blade.

## What changes

The best practice for real-time grass is Ghost of Tsushima's (GDC 2021, "Procedural Grass in Ghost
of Tsushima"), carried by the open reimplementations (for one, `2Retr0/GodotGrass`):

- **Every blade is built in the vertex shader** from its index: no entity, no transform, no CPU
  instance data per blade. A blade is a curved strip of a few segments narrowing to a tip, bent along
  a quadratic Bezier curve by its lean, stiffness, the wind and anything pressing it.
- **Per-blade variety from a hash** of its cell (height, facing, lean, colour, dry blades), with
  **clumps** that share a height and a facing; **wind** as noise scrolled with the time; **edge-on
  blades turned toward the eye**; **normals rounded** across the width; colour from root to tip.
- **Tiles with levels of detail**: 32 m square tiles round the eye, out to 300 m, each drawing one
  shared blade mesh for its level (16, 4 and 1 blades a square metre; 4, 3 and 2 segments a blade;
  farther blades wider). One material and one mesh per level, so Bevy batches every tile of a level
  into one instanced draw: about three draws for all the grass.
- **One height query** (CLAUDE.md 6.1): the ground's height under a blade is the terrain's own
  lattice, sampled on the CPU through the rule `Heightfield::height_at` uses and read in the shader
  on the same two triangles a cell. No terrain noise in WGSL.
- **Where grass grows** is baked per tile, a metre a cell, off the main thread: bare on roads,
  bridges, buildings and cities (except parks; the rule is shared with the trees), on the beach and
  in water, thinning on steep ground, and cleared where a crater dug the ground.
- **Lit like everything else**: Bevy's PBR lights, shadows received, fog; no shadows cast.
- **Bodies press it flat**: the pilot's feet and every mech's feet, a few spheres in a buffer.
- **Measurable**: `CB_NO_GRASS=1` leaves the grass out, so the benchmark suite compares it as a
  variant. The costs in the design are estimates until that run.

## Capabilities

- New: `gpu-grass`.
- Touched: the trees' placement now asks the shared cover rule (`world::cover`), with the same
  margins as before (the golden seed is unchanged); the terrain's cell interpolation is one function
  (`terrain::cell_height`) the terrain, the dug ground and the grass share; a crater's reach is one
  function (`CraterConfig::reach_m`).

## Out of scope

Grass in the cockpit's screen cube cameras (they light the cabin; grass would double its vertex
cost); grass beyond 300 m (the ground's colour carries it); cutting grass by gunfire; grass that
stays bent after a mech has walked through (no per-blade state); a compute-shader culling pass (see
the design's open questions).
