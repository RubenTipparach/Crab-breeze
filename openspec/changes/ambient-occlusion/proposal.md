# Proposal: ambient-occlusion

## Why

Owner, 2026-10-24, on the textured building parts: "Ssao? Might help". A shelled office floor sits
in shade, lit mostly by the sky's ambient light, so its columns, slabs, walls and furniture meet
with no darkening where they touch and read as flat boxes. Screen-space ambient occlusion darkens
creases and contact points in that indirect light.

## What changes

- Bevy 0.20's `ScreenSpaceAmbientOcclusion` on the pilot's eye camera (the world; the cockpit's
  own camera keeps its baked occlusion), with its radius and quality in data (`world.ron` `view`).
- `CB_NO_SSAO=1` switches it off, as the other render switches, for A/B benchmarks.

## Out of scope

Temporal anti-aliasing (Bevy recommends it with SSAO to hide noise; the camera uses FXAA, and TAA
is its own change if the noise shows), baked occlusion for buildings.
