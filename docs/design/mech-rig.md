# Mech rig contract

The one description of a Crab Breeze mech skeleton. The engine side (`cb-app`, built on the Mac
session) drives these bones; the art side (`tools/blender/`, built in the cloud session) names its
bones and empties exactly this way. A model that uses another name is wrong, not the engine.
Adapted from Mining Mike's `player.gd` hierarchy (`godot-sandbox/mechs/tools/mech_chassis.py`):
the back knee and the torso that turns on its own are kept, everything is renamed to one scheme.

## Units and axes

- Metres. glTF space: +Y up, the mech faces **+Z** (the glTF convention). `cb-app` turns the
  scene 180 degrees about Y at spawn so it faces Bevy's forward (-Z). Nothing else rotates it.
- The root sits on the ground between the feet. Standing height is about **12 m** to the top of
  the torso (survey `M1`, provisional), pilot eye about **9.6 m**.

## Bones

```
root                      ground, between the feet; locomotion moves this
  pelvis                  hips; bob and sway of the gait
    torso_twist           yaw only: the torso turns independently of the legs (survey G2)
      torso               pitch for aiming; carries the cockpit and arms
        cockpit           the cockpit shell; EYE_pilot is its child
        shoulder_L > upper_arm_L > forearm_L > hand_L
        shoulder_R > upper_arm_R > forearm_R > hand_R
    thigh_L > shin_L > hock_L > foot_L > toe_L     digitigrade: shin_L ends at the back knee (hock)
    thigh_R > shin_R > hock_R > foot_R > toe_R
```

Suffix `_L` is the mech's own left (+X in glTF, because it faces +Z).

## Empties (sockets), all parented to a bone above

| Empty | Parent | What it is |
| --- | --- | --- |
| `EYE_pilot` | `cockpit` | the first-person camera. Looks along +Z. The only camera in the mech. |
| `SEAT_pilot` | `cockpit` | where the pilot body sits |
| `HATCH_entry` | `cockpit` | where boarding ends; the ladder or lift arrives here |
| `LADDER_base` | `root` | where a pilot on foot starts climbing |
| `MUZZLE_L`, `MUZZLE_R` | `hand_L`, `hand_R` | weapon fire points, pointing +Z at rest |
| `JET_L`, `JET_R` | `torso` | flight thrusters, thrust along -Y at rest (survey M3) |
| `WHEEL_L`, `WHEEL_R` | `foot_L`, `foot_R` | skate wheels or treads (survey M4) |

Other nodes a mech may carry, all optional: `ladder` (under `root`; stowed whenever the mech moves),
`spine`, `crest`, `clamp_L`/`clamp_R` (jet cradle arms), `toe_*`, `HALO_*` (a glow sprite: extras
`halo_size_m`, `halo_color`, `halo_opacity`) and `CORE_*` (a reactor core that spins about Y).
The `root` node's extras carry `legSegs` (each leg segment's length in metres and rest angle),
`footH`, `hipY`, `walk` and `idle`: the numbers Mining Mike's pose layer needs. The reference
animation (idle, walk, turn on the spot, rocket skate, hit) is the one in
`tools/mockups/gen_mech_designs.py`, which runs that pose layer on these bones.

Cockpit controls the pilot touches are named nodes too, `CTRL_<name>`, so the game can animate
and hit-test them: `CTRL_stick` (the right grip: flies the mech, twists to turn the torso) and
`CTRL_throttle` (the left grip in its quadrant). The cockpit itself is a separate glb, the sealed
pod (`assets/models/cockpit_breeze.glb`), mounted on the `cockpit` joint so its `EYE_pilot` is the
mech's; its hatch nodes (`HATCH_upper`, `DOOR_L/R`, `FLAP_L/R`, `HATCH_lower`, `STAIR_1..3`,
`CONSOLE_front`, `LATCH_*`, `STRUT_*`, `VENT_*`) are listed with their pivots in its manifest
(`assets/models/cockpit_breeze.json`) and animated by `openspec/changes/mech-boarding`.

## Who owns what

- **Engine (Mac):** the placeholder mech built from boxes on these bones, procedural walk,
  skate and jet poses, the first-person camera on `EYE_pilot`. It must load a glb that follows
  this contract and drive it with no code change.
- **Art (cloud):** Blender generators in `tools/blender/` that export `assets/models/mech_*.glb`
  on this skeleton, with LODs and a manifest, and the review pages in `docs/mockups/`.

Changing a name here is a change to both sides in the same commit.
