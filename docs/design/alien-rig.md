# Alien crab rig

The one description of an alien crab mech's skeleton. A crab does not fit the biped contract in
`docs/design/mech-rig.md` (no pelvis, torso or cockpit, six or eight legs, claws instead of
arms), so the invaders have this one. The engine side drives these nodes; the art side
(`tools/blender/build_alien_mechs.py`) names its joints and empties exactly this way. A model that
uses another name is wrong, not the engine. Changing a name here is a change to both sides in the
same commit.

The reference animation (scuttle, claw snap, idle) is the one in
`tools/mockups/gen_alien_mechs.py`, which drives these nodes on the review page
`docs/mockups/alien_mechs.html`.

## Units and axes

- Metres. glTF space: +Y up, the crab faces **+Z**, +X is its own left. As with the human mechs,
  `cb-app` turns the scene 180 degrees about Y at spawn so it faces Bevy's forward (-Z).
- The root sits on the ground under the middle of the body. Every sole touches y = 0 in the rest
  pose: the generator solves each femur's raise so the foot lands exactly on the ground.
- Every limb segment runs along its own joint's **+X**. A joint's rest rotation is its yaw about
  Y and its raise or bend about Z, nothing else, so an animation adds small turns about the same
  two axes.

## Joints

```
root                                ground, under the body; locomotion moves this
  body                              the hull; bob and sway ride here. Never tilted at rest
    shell                           optional: a shell on the hull (Big Boy's lens), tilted at rest
    leg_<S><n>_coxa                 the hip: yaw only (swing the leg forward and back)
      leg_<S><n>_femur              raise about Z (positive lifts the leg)
        leg_<S><n>_tibia            the knee: bend about Z (negative folds it down)
          leg_<S><n>_foot           keeps the sole level: at rest its Z turn cancels femur + tibia
            TIP_<S><n>              empty on the ground under the sole: the foot's contact point
    arm_<S>                         a claw's shoulder: yaw and raise at the hull
      wrist_<S>                     the elbow: bend about Z, fold toward the centre line about Y
        claw_<S>                    the hand: palm and the fixed finger
          pincer_<S>                the movable finger: open and shut about Z
    mandible_<S>                    optional mouthparts under the face, yaw about Y
    mast_<S>                        optional antenna mast, sways about X and Z
    turret_<n>                      optional yaw turret on the hull (the Skitter's sensor turret)
    pod_<S>                         optional missile pod arm (the Skitter), pitch about X
```

- `<S>` is the side, `L` (+X, the crab's own left) or `R`. `<n>` counts legs front to back from
  1: six legs are `L1 L2 L3 R1 R2 R3`, four legs are `L1 L2 R1 R2`.
- Legs carry four joints whatever their shape: the Skitter's tall arched legs, the Snapper's
  folded stubs and Big Boy's piston legs all name the same four, so one gait drives every crab.

## Turning a limb, and the side sign

Rotations about Z (raise, knee, pincer) mirror cleanly: a positive femur turn lifts the leg on
both sides. Rotations about Y do not: the right side's joints are the left's turned half a circle
about Y, so a positive yaw swings a left leg backward and a right leg forward. **Multiply every Y
turn by the side sign** (`L` is +1, `R` is -1) to make both sides do the same thing. The root
extras carry each leg's sign.

Each design's pincer opens the way its claw is drawn, given as `snap.dir` in the root extras:
`pincer_<S>` turns by `dir * open` about Z.

## Empties (sockets)

| Empty | Parent | What it is |
| --- | --- | --- |
| `TIP_<S><n>` | `leg_<S><n>_foot` | the foot's ground contact, for foot placement and footprints |
| `MUZZLE_<name>` | the weapon's joint | a fire point; fires along its own **+Z** (`MUZZLE_claw_L`, `MUZZLE_pod_L1`, `MUZZLE_turret_L`) |
| `SENSOR_<name>` | `body`, `shell` or `turret_<n>` | an eye: where the crab looks from, along +Z (`SENSOR_L`, `SENSOR_main`, `SENSOR_1`) |
| `WEAK_<name>` | `body` or `shell` | a lock-on weak point for a set piece (`WEAK_core`, `WEAK_tank_L`, `WEAK_belly`) |
| `HALO_<n>` | any | a glow sprite: extras `halo_size_m`, `halo_color`, `halo_opacity` (the kit's convention) |

`CORE_<n>` is a reactor core mesh that spins about its Y, as on the human mechs.

## Root extras

The `root` node's extras carry what the reference animation needs, in JSON:

| Key | What |
| --- | --- |
| `rig` | `"alien-crab"`, so a loader can tell this skeleton from the biped one |
| `legs` | `[{id: "L1", side: 1, group: 0}, ...]`: every leg, its side sign and its gait group. Groups alternate (L1, R2, L3 against R1, L2, R3), so half the legs are always planted |
| `claws` | the sides that have a claw chain |
| `masts` | the masts by side |
| `gait` | `hz` (step cycles a second), `swing` (coxa swing, radians), `lift` (femur lift, radians), `scuttle_m` (how far the page's demo scuttles sideways, metres), `bob` (body rise, metres) |
| `snap` | `hz` (claw snaps a second), `open` (radians), `dir` (which way the pincer opens) |
| `idle` | `scan` and `scanPeriod`: the turret's idle sweep, radians and seconds |
| `rest` | every joint's rest Euler (XYZ, radians) |
| `scale` | the kit-to-metres scale the generator used |

## The reference gait

Each leg's phase is the gait phase plus half a cycle for group 1. In its swing half the femur
lifts by `lift * sin(phase)`, the knee folds by 0.45 of that and the foot turns back by the other
0.55, so the sole stays level; all through the cycle the coxa swings by `side * swing * cos(phase)`.
In the stance half the femur and knee are at rest, which stands on the ground, and the coxa turns
about the vertical, which keeps the sole on it, so no foot ever goes through the ground; the body
only rises, never sinks, for the same reason. A crab scuttles sideways: the root moves along X.

## Who owns what

- **Engine:** spawning, the gait and claw animation on these names, weapon fire from `MUZZLE_*`,
  lock-on boxes on `WEAK_*`, eyes on `SENSOR_*`.
- **Art (cloud):** `tools/blender/build_alien_mechs.py` on the kit `tools/blender/mech_kit.py`,
  exporting `assets/models/alien_*.glb` and the measured manifest `assets/models/aliens.json`,
  and the review page `docs/mockups/alien_mechs.html`.
