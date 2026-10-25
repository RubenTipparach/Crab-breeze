# Proposal: building-texture-array

## Why

Owner, 2026-10-24: "Can you combine materials to make draws cheaper?", then "As long as it looks
good and perf improves". A damaged building is drawn as about ten meshes, one per material (facade,
eight surfaces, the skin's materials), each a draw in the main pass, the prepass and four shadow
cascades.

## What changes

- The opaque building looks (the eight surfaces, the skin's brick, concrete, roof, wood and trim,
  the facade tones' plain panel) become layers of three texture arrays: colour, normal, packed
  occlusion-roughness-metal. Each vertex carries its layer in its second UV channel.
- One opaque building material reads the arrays (its own fragment shader, which builds Bevy's PBR
  input and hands it to Bevy's lighting, so sun, shadows, SSAO and fog are unchanged), so a
  building's opaque parts are one mesh and one draw per pass; glass stays its own mesh and material.
- Done only if a still of the same demolition looks the same and the interleaved combat benchmark
  is faster (the owner's condition).

## Out of scope

Glass (transparent, its own pass), far buildings' boxes, merging buildings into one mesh.
