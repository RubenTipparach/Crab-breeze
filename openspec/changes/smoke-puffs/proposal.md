# Proposal: smoke-puffs

## Why

Owner, 2026-10-25: "I like how the cloud renders ... can we render smoke like that? especially stuff
from explosions?", "I still cant see the players thruster trails from using rockets and skates", "use
the same cloud rendering thing for that smoke, same with missiles too". Smoke today is bevy_hanabi
billboard sprites: flat, unlit by the sun, and big blended clouds reorder and flicker when their
bursts merge. The pilot's own jet flame is a 0.25 s flame at nozzles behind and below the seat, and
skating in the air throws nothing, so from the cockpit no trail is ever seen.

## What changes

- Smoke as lit 3D puffs drawn the way the clouds are (`cloud-puffs`: a lumpy blob mesh, the scene's
  sun wrapped round it, a darker underside, a silver lining toward the sun, a soft silhouette, Bevy's
  fog), each puff its own entity so it sorts on its own and never reorders with others.
- A puff grows, rises, drifts with the wind and fades over its life; its colour comes from the
  recipe (grey-black explosion smoke, white rocket exhaust, brown dust).
- Explosion smoke, missile trails, the rockets' exhaust while lifting and the skates' thrust in the
  air leave puffs: trails that linger several seconds, so the pilot sees them looking round, down,
  or on the side screens, and sees other mechs' trails.
- A `Puffs` effect layer in `fx.ron` (count or spacing, size, growth, rise, life, colour), played by
  the same recipes as the sprites.

## Out of scope

Volumetric (raymarched) smoke, puffs colliding with anything, smoke shadows.
