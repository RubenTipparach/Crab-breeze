# Proposal: gpu-rubble

## Why

Owner, 2026-10-24: "Small rubble as gpu particles great idea". A breaking building throws only
big physics chunks (a storey's slab or column per 6 m bay, `debris-physics`) and flat dust and
rubble sprites; there is no small solid debris. Making the small pieces Avian bodies would cost
CPU for every one; the GPU can throw thousands.

## What changes

- A new effect layer, `Chunks`: small 3D concrete pieces drawn as mesh particles by `bevy_hanabi`
  (a few rough box meshes, each piece a random one, size and spin), thrown on the GPU, falling
  under gravity, landing on the ground height where they were thrown and resting there until they
  fade. No CPU work per piece.
- Every chunk that breaks off a building sheds them: near the pilot beside its physics body, far
  away with its dust puff (the far puff gains them, so a distant collapse throws solid debris too).
  How many grows with the chunk's volume, capped per effect.
- The numbers (count per cubic metre, cap, size, speed, life, rest time) live in `fx.ron`.

## Out of scope

Pieces colliding with buildings, mechs or each other (they only land on the ground height of the
place they were thrown), lit pieces (hanabi particles are unlit; they are tinted concrete), pieces
the player can push.
