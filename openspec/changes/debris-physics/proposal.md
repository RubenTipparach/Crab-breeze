# Proposal: debris-physics

## Why

Owner, 2026-10-24: "carnage all physics drive" and survey T2 (Avian). Falling building chunks,
mech wrecks and felled trees should tumble, collide and pile up under real rigid-body physics.

## What changes

- Avian 3D (`avian3d = 0.8.0`, for Bevy 0.20) in `cb-app`: the terrain (a heightfield collider per
  chunk, plus crater tiles), buildings (static box colliders from intact cells near debris), and
  dynamic bodies for building chunks, wrecks and felled trees.
- Debris is presentation, not simulation: the authoritative world (cells, trees, ground, mechs) is
  `cb-core`'s and deterministic; Avian bodies decide only where pieces come to rest. (Avian is not
  bit-deterministic across machines; this is said here, as `deterministic-simulation` requires.)
- Budget: at most 600 dynamic bodies; the oldest resting ones freeze into static scenery past that.

## Out of scope

Debris as walls for mechs, physics-driven mech movement.
