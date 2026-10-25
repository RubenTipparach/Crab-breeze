# Proposal: building-surfaces

## Why

Owner, 2026-10-24, after the city combat take: "The parts on the buildings need textures. All
furniture needs textures. Build.it. this is promising." A building's facade skin is textured
(`building-generation`), but once it is hit the player sees flat colour: the columns and floor
slabs, the room walls, every piece of furniture and the falling chunks are plain tinted boxes.

## What changes

- Eight CC0 PBR sets from ambientCG (colour, normal, packed occlusion-roughness-metal), checked
  seamless, loaded with full mips by `pbr_set` (the CLAUDE.md section 13 CC0 exception, as the
  grass): concrete for columns, slabs, core walls and debris; plaster for room walls; wood, fabric,
  painted metal, plastic, cardboard and leaves for furniture.
- Every furniture box names its surface in the kit (`cb-core::world::interior`), so the furniture
  is drawn one mesh per surface, each with its set; the kit's colours still tint them.
- A building's bands and its whole-building interior split their one "bare" mesh into one mesh per
  surface. Falling chunks use the concrete set, tinted by their building.

## Out of scope

New furniture shapes, decals, glass and facade skins (already textured), the rubble heap on the
ground (it is ground, `rubble`).
