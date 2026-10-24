# Proposal: combat-fx

## Why

Owner, 2026-10-23: "start authoring some FX for shots, explosions, missiles etc". Today a shot is a
dot redrawn every fixed step (with the default white material: `FxKind::Tracer` has no material of
its own), a muzzle flash is a few orange puffs, an impact is the same few puffs on any surface, and
an explosion is particles and one point light. From the cockpit, 300 m away through the screen
wall, none of it reads as a 35 mm round or a 120 mm shell. Ace Combat (G4) lives on readable
weapons: you see the tracer arc, the flash, the fireball and the smoke column, and you know what
hit what.

## What changes

- **Tracers** are one streak per shot, stretched along its velocity (a 35 mm round is a 9 m streak,
  a 120 mm shell a 14 m one), glowing, faced to the eye about its own axis, and gone the step the
  shot stops. The default-material bug goes with the old per-step dot.
- **Muzzle flash**: a short point light at the muzzle (autocannon 0.05 s, cannon 0.12 s), a
  three-blade star flash mesh, the fire puffs, and for the cannon a muzzle-brake side blast and a
  smoke ring that hangs in the air.
- **Impacts by surface**: on a mech, sparks, a white-hot flash and a smoke wisp; on a building,
  concrete dust, grey chunks that fall, and smoke; on the ground, dirt and dust; on water, a spray
  column and a ring. Heavy shells add a light flash.
- **Explosions in layers**: the light flash (kept), a fireball that grows and cools from white to
  orange to soot over 0.9 s, a fast shockwave shell and a ground dust ring, burning debris that
  arcs out trailing smoke, and a smoke column that rises and spreads for 20 s.
- **Rocket and missile trails** (the rockets themselves are the `rockets` change): a motor flame
  and its light at the tail, and a smoke trail laid every 3 m of flight that hangs for 12 s, so
  you can see where a rocket came from.
- **One effects vocabulary**: every effect above is a named recipe in `data/fx/fx.ron` (layers of
  bursts, lights, meshes, with their numbers), asked for by name with a message (`Fx { recipe,
  at, dir, scale }`). Combat, blasts, the mech and the world stop writing particle numbers inline
  (CLAUDE.md section 6.4).

## Out of scope

Flipbook smoke and fire sprites (survey `G7`: where the sprites come from), decals (scorch marks
on buildings and mechs), screen-space distortion (heat haze), cockpit jolts from near blasts
(survey `G8`), sound.
