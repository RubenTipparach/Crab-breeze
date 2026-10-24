# Proposal: gpu-instancing

## Why

Owner, 2026-10-24: "Batch meshes", "We need to batch meshes better", "look at swarm repo, we did it
there with lots of entities", "Tightly packed arrays will help with simd", "Or compute shaders".
Measured on the RTX 3070 (the perf readout, `perf-hud`, and a `trace_chrome` trace of the defence
take): the game is CPU-bound throughout. The GPU works 0.4-0.7 ms a frame; frames take 6.6-10.9 ms
uncapped with 5,000-8,000 meshes drawn and 16,000-20,000 entities, and the render world spends its
time per mesh entity per view: material specialization 2.2 ms, command submission and the camera
driver 1.5 ms each, shadow specialize and queue 1.4 ms, material queueing 1.1 ms, visibility about
1 ms, gathering meshes 0.65 ms (with tracing on). Thousands of trees, building cells and the mech
stand-in's boxes are each an entity, and each pays that cost in every view (the eye, the cockpit
pass, four shadow cascades).

## What changes

The swarm-demo way (`RubenTipparach/swarm-demo` at `625596b`, `crates/swarm_app/src/swarm.rs`: "a
million of anything cannot be entities, and the ECS holds the things there are dozens of"):

- **Trees first**: every tree is a row in tightly packed arrays (position, yaw, height, species, tilt
  and fall state as separate `Vec`s, structure of arrays, so the fall and sway updates are SIMD
  friendly), uploaded as one GPU storage buffer per tile when it changes. A compute pass culls them
  (distance and frustum, per view including the shadow cascades) into a compacted visible list and
  writes indirect draw arguments; one indexed indirect draw per species mesh draws them, through a
  custom render command on Bevy's mesh pipeline so they are lit and cast shadows. No entity per
  tree. Felling and falling stay in `cb-core` (the simulation decides); only the changed rows are
  written to the GPU.
- **Buildings next**: the building cells become one mesh per city block, rebuilt only when a cell in
  it breaks (the destruction code already marks dirty buildings).
- **Mechs**: the stand-in's 39 boxes become one merged mesh per bone group; the boxes stay as hit
  shapes only.
- **Measured** before and after each step with the perf readout and `--bench defence`, and reported
  in `docs/benchmarks/`.

## Out of scope

The particles (already on the GPU through hanabi), terrain (already chunked), the Blender mech
models (one skinned mesh each).
