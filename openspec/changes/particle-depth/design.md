# Design: particle depth

## 1. The bench (built first)

`CB_STILLS_FX=depth` (in `capture.rs`, beside `jets`): after `GAME_READY` the pilot sits in a parked mech
on open ground beside a building. Each test plays one recipe at a fixed point and holds it while frames
are shot:

| Test | Recipe | Where | Why |
|---|---|---|---|
| ground near | `explosion_ground` smoke layer | 15 m ahead, on the ground | the flat cut, close |
| ground far | the same | 450 m ahead | the flat cut and the shimmer at range |
| wall | `burning` stream | against a building face, 120 m | a sprite crossing a vertical surface |
| trail | `jet_trail` | an enemy flying across at 120 m, 30 m up | overlap order inside one effect |
| skate | `skate_dirt` | a mech skating past at 40 m | dust and the new steam on the ground |
| sky | `explosion_air` | 300 m up | a control: nothing to cut against |

Each test is shot at 0.5 s and 3 s after it starts, and from two headings 20 degrees apart so overlaps
reorder. `tools/fx/depth_sheet.py` stitches A, B and C side by side per test, labelled. The bench runs
headless through lavapipe in a cloud session. It says nothing about frame time.

## 2. Variant B: soft particles in hanabi

Hanabi's render shader (`vfx_render.wgsl`) gets the scene's depth from the depth prepass (the cameras
already carry `DepthPrepass` for the water and the clouds) and multiplies alpha by

    soft = smoothstep(0, soft_m, view_depth(scene) - view_depth(particle))

the clouds' `soft_edge`, with `soft_m` per kind in data (`fx.ron` `soft_m`, default 2 m; 0 turns it off
for sparks and fire, which should stay crisp). Hanabi is vendored under `third_party/bevy_hanabi` at
77630ff with the patch on top and a `PROVENANCE.md`. The workspace `[patch]` points at it.

Cost: one depth texture read per particle pixel. It does not fix the order inside an effect.

## 3. Variant C: sorted smoke billboards

A renderer on the clouds' pattern (`clouds_render`: one mesh of quads, sorted back to front on the CPU
each frame, `soft_edge` against the prepass) draws the blended smoke kinds. Each particle's life
(position, velocity, drag, rise, growth, colour over life) is stepped on the CPU from the same `motion`
and `colour_over_life` tables `particles.rs` holds, so both renderers read one table (section 6.1).
Recipes do not change: a `Burst` of a blended smoke kind goes to this renderer, anything else to hanabi.

Cost: CPU stepping and a sort. At the `max_particles` cap of 5000 that is a sort of up to 5000 keys and
one mesh upload of 20 000 vertices a frame. That needs measuring on the owner's machine before it ships.
It fixes both the cut and the order.

## Open questions

| Id | Question | Recommendation |
|---|---|---|
| PD1 | B or C, from the sheets | C if B still shimmers in the trail and far tests; B if it does not |
| PD2 | `soft_m` per kind | 2 m smoke and dust, 1 m steam and mist, 0 sparks, fire, flame and debris |
