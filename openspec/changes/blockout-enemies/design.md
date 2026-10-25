# Design: blockout-enemies

## Data

`data/enemies/skins.ron`: `skins: { "<id>": (model: "models/mech_blockouts/<id>.glb", scale: 1.0) }`, validated on
load: every model exists under `assets/`, scale is positive and finite. `EnemySpec` and the defence `raiders` take an
optional `skin` (an id in that list; an unknown id is an error naming it).

## Game (`cb-app::skins`, one plugin)

- At spawn (`spawn_enemy`, the one way enemies enter): the stand-in is built as now; its box meshes are hidden; the skin's
  scene is spawned under the mech's model node (glTF space, the same facing), scaled.
- When the scene has spawned: its `LEG_*` and `ARM_*` nodes are recorded with their side, pair and rest rotation.
- Each frame (presentation, after the mech is posed), the feet are planted (owner, 2026-10-25: "mechs look like they
  just hover aruond"; the first walk swung a hanging leg about the vertical, which twists it in place). Each walking leg
  has a `<LEG>_foot` node at its lowest point (the block-out builder puts it there; tracks have none and do not walk).
  The foot's offset from the hip across the travel, `d`, gives the swing axis `d x forward` (a hanging leg swings about
  its side axis, a crab's splayed leg about the vertical) and the reach `r = |d|` times the skin's scale.
  - Phase `u = gait_cycles + group`: the sides half a cycle apart, each pair a quarter on, the arms with the first
    pair's other side. A foot is down for 0.55 of the cycle.
  - Half sweep `a = asin(stride_m * 0.55 / (2 r))` (at most 0.6 rad) times `gait_swing`: while down the angle falls
    evenly from `+a` to `-a`, so the foot goes back `2 r sin a`, exactly as far as the body goes forward
    (`gait_cycles` advances by distance over `stride_m`); in the air it comes back, eased, lifted out to the side by up
    to 0.16 rad.
  - A planted foot swung off straight rises by `(R d - d).y`: the skin's body comes down by the most any planted foot
    rose, so its feet keep the ground and it bobs as it walks.
  - Arms pitch 0.18 rad with the step.
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
