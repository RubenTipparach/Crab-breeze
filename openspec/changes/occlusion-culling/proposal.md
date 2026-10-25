# Proposal: occlusion-culling

## Why

Owner, 2026-10-24: "we need more work shove to gpu as possible", then "ok go, I want to see
benchmarks and videos of your battles". In a city street most buildings, their rooms and the debris
behind them are hidden by the buildings in front, but every one is still transformed and drawn in
the main pass and the prepass.

## What changes

- Bevy 0.20's GPU occlusion culling (`OcclusionCulling`) on the pilot's eye camera: a two-phase
  depth prepass with a depth pyramid, all on the GPU, skips meshes hidden behind what was drawn last
  frame. Our pipeline already has GPU preprocessing and a depth prepass (water, SSAO), which it
  needs.
- `view.occlusion_culling` in `world.ron` (on), `CB_NO_OCCLUSION=1` to switch it off for A/B runs.
- Kept only if the interleaved city combat benchmark is faster.

## Out of scope

The cockpit camera (it draws the cabin only), shadow views (Bevy culls those by frustum).
