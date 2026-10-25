# Design: blockout-enemies

## Data

`data/enemies/skins.ron`: `skins: { "<id>": (model: "models/mech_blockouts/<id>.glb", scale: 1.0) }`, validated on
load: every model exists under `assets/`, scale is positive and finite. `EnemySpec` and the defence `raiders` take an
optional `skin` (an id in that list; an unknown id is an error naming it).

## Game (`cb-app::skins`, one plugin)

- At spawn (`spawn_enemy`, the one way enemies enter): the stand-in is built as now; its box meshes are hidden; the skin's
  scene is spawned under the mech's model node (glTF space, the same facing), scaled.
- When the scene has spawned: its `LEG_*` and `ARM_*` nodes are recorded with their side, pair and rest rotation.
- Each frame (presentation, after the mech is posed): every limb still on is set to its rest rotation turned by the
  gait: phase `2 pi gait_cycles + pi (group)`, where group alternates by pair and side and the arms walk as pair -1;
  lift `max(0, sin phase) * gait_swing` about the limb's forward axis (0.22 rad), swing `cos phase * gait_swing` about
  the vertical (0.16 rad over the square root of the pairs); arms pitch 0.18 rad. Tracks have no limbs and do not move.
- `cut_parts`: for a skinned mech, a lost ArmL or ArmR detaches `ARM_L` or `ARM_R`, a lost LegL or LegR the lowest
  numbered `LEG_*L` or `LEG_*R` still on; the limb becomes a dynamic body with the stand-in cut's box collider and
  velocities. The stand-in's own cut bone stops being driven but is not thrown (it is not drawn).

## Cost

Sixteen skins of 4,000 to 9,000 triangles each (`mech_combat.json`, `mech_crabs.json`): ten enemies about 70,000
triangles, against the stand-in's boxes. One extra system walking each skin's limbs (2 to 8) a frame. Measured with
`--bench defence` on the owner's machine (cloud sessions measure nothing).

## Open questions

| Id | Question | Recommendation (provisional) |
| --- | --- | --- |
| E1 | Which skins in the first encounter and the defence waves? | Encounter: Sabre, Hatchet, Tyrant. Waves: combat mechs in wave 1, combat mechs and two-pair crabs in wave 2, crabs with a Tyrant in wave 3 |
| E2 | Hit capsules shaped to each skin, next? | Yes, with the crabs' per-leg parts (`crab-enemies`) |
