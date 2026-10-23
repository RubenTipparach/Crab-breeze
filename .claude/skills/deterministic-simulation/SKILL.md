---
name: deterministic-simulation
description: Rules for deterministic, seed-driven Crab Breeze code - fixed time steps in seconds, seeded and forked RNG, stable ordering of entities, finite-value guards, pure resolvers and golden-seed tests. Use whenever code involves randomness, simulation time, ordering of entities, procedural generation (cities, crowds, damage), replays, scripted playtests or anything that must give the same result twice.
metadata:
  author: Crab Breeze (Claude Code), after Falchion's deterministic-simulation and Star Crew's determinism rules
  version: "1.0"
---

# Deterministic simulation

A scripted playtest must play the same way twice, a benchmark scene must load the same city twice,
and a bug seen once must be reproducible. Therefore:

1. **Fixed steps in seconds.** The simulation advances in `FixedUpdate` by a fixed `dt` in seconds,
   never by frame count and never by wall clock. Core functions take `dt_s` as a parameter.
2. **No ambient randomness.** Never `rand::random()` or `thread_rng()` in simulation code. Use the
   project's seeded RNG, seeded from the session seed, the entity's stable id and the purpose.
3. **Fork, don't share.** Each subsystem derives its own stream (`rng.fork("crowds")`), so adding a
   feature does not shift every later roll.
4. **Stable order.** A `HashMap` is fine for keyed lookup, but never let hash order or ECS
   iteration order choose an outcome, an id, a save layout or a network message order. Sort by a
   stable id first.
5. **Stable ids.** Entities that matter to the simulation carry a stable id component; Bevy's
   `Entity` is not stable across runs or saves.
6. **Finite values only.** Reject NaN and infinity where numbers enter (data, input, physics
   results, saves) with a logged error, never let them spread.
7. **Pure resolvers.** Damage, ballistics and generation take `(state, inputs, seed)` and return a
   result, without hidden reads of time, globals or the renderer.
8. **Golden seeds.** A test that says "seed 42 builds this block with these 37 buildings" catches
   accidental nondeterminism. A scripted playtest that ends in a different place on the same seed
   is a bug.

Physics engines may not be bit-deterministic across machines; the design that picks one (an
OpenSpec change) says what is promised and what is not.
