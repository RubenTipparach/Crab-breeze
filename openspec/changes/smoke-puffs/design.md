# Design: smoke-puffs

## 1. The puff

One shared blob mesh (an icosphere pushed out by the clouds' `lump` noise; 4 subdivisions, about
500 triangles; detail 3, about 120, beyond 150 m), one `SmokeMaterial` (blended, no prepass, no
shadows). Each puff is an entity with the shared mesh and material, its transform carrying position
and size, and a `MeshTag` indexing a storage buffer of per-puff state (age 0..1, colour, opacity),
so every puff shares one material and batches. The shader is the cloud shader's lighting (sun wrap,
underside, silver lining, soft edge, fog), with the colour, opacity and fade over age from the
buffer. Built on the merged `clouds_render.rs` and `cloud.wesl`: the shared lighting moves into a
WESL module both import (one implementation, CLAUDE.md 6.1).

## 2. Life

Per puff, on the CPU each frame (a few hundred puffs at most): size grows from `size_m` by `grow`,
it rises at `rise_m_s`, drifts with the cloud wind, and fades out over the last `fade` share of
`life_s`. Spent puffs return to a pool (no spawn and despawn churn).

## 3. Who emits

- Explosions: a `Puffs` layer in the explosion recipes in place of their `Smoke` sprites.
- Missiles and rockets: one puff every `spacing_m` along the flight (the `rocket_trail` recipe).
- The pilot's mech and every mech: while lifting on rockets, a puff every `spacing_m` from each
  `JET_*` socket; while skating in the air, from each `WHEEL_*` socket.

## 3a. Trails laid by distance

Owner, 2026-10-25: "use the new smoke trails, and make them larger and more conencted". A trail is laid
by path length, not by frame: a puff every `spacing_m` along the path a source moved, the distance
past the last puff carried over to the next step (`cb_core::fx::lay_along`), and each puff starts
wider than the spacing (`size_m >= spacing_m`, refused otherwise), so the trail is one stream at any
frame rate. A missile's puffs come from how far it has flown (age times speed), so no shot needs an
id. The sprite trails laid one burst per frame, which at four frames a second (a software renderer)
or in a slow frame left clumps 13 m apart.

| trail | spacing | starts | grows | life | puffs alive per source |
| --- | --- | --- | --- | --- | --- |
| jets (one trail from between the two) | 3 m | 6 m | 0.5 m/s, 66 m at 2 min | 120 s | 20 a second at 60 m/s, cut by `max_puffs` |
| missile | 2 m | 3 m | 1.8 m/s | 6 s | 120 a second at 240 m/s, 720 in flight |

Puffs share four lumpy 80-triangle meshes and one material per colour and opacity step (16 steps), so
`max_puffs` (3000) is at most 240,000 triangles; past it the oldest puff is used again.

## 4. Cost

Draw: one batched draw for all puffs in the transparent pass (each sorted on its own; Bevy batches
neighbours sharing mesh and material). Triangles: 500 a near puff; a busy fight's 300 puffs are about
150,000 triangles. Fragment: one blended layer per puff, the clouds' overdraw. Cap `max_puffs`
(600). Measured with city combat against the build before, interleaved.

## Open questions

| question | recommendation | status |
| --- | --- | --- |
| Keep the sprite smoke as well | no: puffs replace the explosions' smoke sprites; fire, sparks and dust stay sprites | provisional |
| Trail length | 6 s for rockets, 3 s for skates | provisional |
