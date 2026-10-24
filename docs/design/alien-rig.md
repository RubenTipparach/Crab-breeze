# Alien crab rig

The one description of an alien crab mech's skeleton. A crab does not fit the biped contract in
`docs/design/mech-rig.md` (no pelvis, torso or cockpit, four to eight legs, two weapon arms where a
crab has claws), so the invaders have this one. The engine side drives these nodes; the art side
(`tools/blender/build_alien_mechs.py`) names its joints and empties exactly this way. A model that
uses another name is wrong, not the engine. Changing a name here is a change to both sides in the
same commit.

No crab carries a claw or a pincer (owner, 2026-10-24: "crab should not have big claw features. It's
claws are guns or swords or missile launchers"). As on Mining Mike's bosses, the arm a crab would
end in a claw ends in a weapon: a cannon, a gun, a sword or a missile rack.

The reference animation (scuttle, fire, idle) is the one in
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
    arm_<S>                         a weapon arm's shoulder: yaw and raise at the hull
      wrist_<S>                     the elbow: bend about Z, fold toward the centre line about Y
        gun_<S>                     a gun or cannon: fires along its +X, recoils along -X
        blade_<S>                   or a sword: lies in its XY plane, swings about Z (+ winds up)
        rack_<S>                    or a missile rack: tubes along its +X
    mandible_<S>                    optional mouthparts under the face, yaw about Y
    mast_<S>                        optional antenna mast, sways about X and Z
    turret_<n>                      optional yaw turret on the hull (the Skitter's sensor turret)
    pod_<S>                         optional missile pod arm (the Skitter), fires along +Z, pitch about X
```

- `<S>` is the side, `L` (+X, the crab's own left) or `R`. `<n>` counts legs front to back from
  1: six legs are `L1 L2 L3 R1 R2 R3`, four legs are `L1 L2 R1 R2`.
- An arm carries exactly one weapon joint, named for its kind. What each crab carries:

  | Crab | Left arm | Right arm | Other weapons |
  | --- | --- | --- | --- |
  | Skitter | `gun_L`, twin-barrel SMG | `gun_R`, twin-barrel SMG | `pod_L`, `pod_R` (four tubes each), `turret_1` (twin guns) |
  | Snapper | `gun_L`, heavy cannon | `blade_R`, sword | |
  | Big Boy | `gun_L`, siege cannon | `rack_R`, twelve-tube missile battery | |

- Legs carry four joints whatever their shape: the Skitter's tall arched legs, the Snapper's
  folded stubs and Big Boy's piston legs all name the same four, so one gait drives every crab.

## Turning a limb, and the side sign

Rotations about Z (raise, knee, a blade's swing) mirror cleanly: a positive femur turn lifts the leg on
both sides. Rotations about Y do not: the right side's joints are the left's turned half a circle
about Y, so a positive yaw swings a left leg backward and a right leg forward. **Multiply every Y
turn by the side sign** (`L` is +1, `R` is -1) to make both sides do the same thing. The root
extras carry each leg's sign.

A recoil is a slide, not a turn: the weapon joint moves back along its own fire axis (the `axis` of
its entry in `weapons`, in the joint's frame) and returns.

## Empties (sockets)

| Empty | Parent | What it is |
| --- | --- | --- |
| `TIP_<S><n>` | `leg_<S><n>_foot` | the foot's ground contact, for foot placement and footprints |
| `MUZZLE_<name>` | the weapon's joint | a fire point; fires along its own **+Z** (`MUZZLE_gun_L`, `MUZZLE_rack_R1` to `MUZZLE_rack_R12`, `MUZZLE_pod_L1`, `MUZZLE_turret_L`) |
| `EJECT_<name>` | a gun's joint | where spent cases leave an SMG (`EJECT_gun_L`) |
| `BLADE_tip_<S>`, `BLADE_mid_<S>` | `blade_<S>` | the sword's tip and the middle of its edge: slash trails and hit tests |
| `SENSOR_<name>` | `body`, `shell` or `turret_<n>` | an eye: where the crab looks from, along +Z (`SENSOR_L`, `SENSOR_main`, `SENSOR_1`) |
| `WEAK_<name>` | `body`, `shell` or a weapon | a lock-on weak point for a set piece (`WEAK_core`, `WEAK_tank_L`, `WEAK_belly`, `WEAK_breech` on Big Boy's siege cannon) |
| `HALO_<n>` | any | a glow sprite: extras `halo_size_m`, `halo_color`, `halo_opacity` (the kit's convention) |

`CORE_<n>` is a reactor core mesh that spins about its Y, as on the human mechs.

## Root extras

The `root` node's extras carry what the reference animation needs, in JSON:

| Key | What |
| --- | --- |
| `rig` | `"alien-crab"`, so a loader can tell this skeleton from the biped one |
| `legs` | `[{id: "L1", side: 1, group: 0}, ...]`: every leg, its side sign and its gait group. Groups alternate (L1, R2, L3 against R1, L2, R3), so half the legs are always planted |
| `arms` | the sides that have a weapon arm |
| `weapons` | `[{joint, kind, muzzles, recoil_m, flash_m, period_s, axis}, ...]`: every weapon. `kind` is `cannon`, `smg`, `blade` or `rack`; `muzzles` its `MUZZLE_*` sockets in firing order; `recoil_m` how far it slides back (metres); `flash_m` its muzzle flash size (metres); `period_s` the review page's fire cycle (seconds); `axis` its fire axis in the joint's frame |
| `masts` | the masts by side |
| `gait` | `hz` (step cycles a second), `swing` (coxa swing, radians), `lift` (femur lift, radians), `scuttle_m` (how far the page's demo scuttles sideways, metres), `bob` (body rise, metres) |
| `idle` | `scan` and `scanPeriod`: the turret's idle sweep, radians and seconds |
| `rest` | every joint's rest Euler (XYZ, radians) |
| `scale` | the kit-to-metres scale the generator used |

## The reference fire cycle

The review page's Fire button runs every entry in `weapons` on its own cycle of `period_s` seconds.
A cannon fires once: a muzzle flash for 0.14 s, a recoil of `recoil_m` in 0.05 s, back over 0.7 s. An
SMG fires a 1.2 s burst at 11 rounds a second, alternating muzzles. A rack ripples its tubes in order
over 1.1 s. A sword winds up 0.55 radians about Z in 0.6 s, slashes down to -1.05 in 0.2 s and comes
back over 1.0 s.

## The reference gait

Each leg's phase is the gait phase plus half a cycle for group 1. In its swing half the femur
lifts by `lift * sin(phase)`, the knee folds by 0.45 of that and the foot turns back by the other
0.55, so the sole stays level; all through the cycle the coxa swings by `side * swing * cos(phase)`.
In the stance half the femur and knee are at rest, which stands on the ground, and the coxa turns
about the vertical, which keeps the sole on it, so no foot ever goes through the ground; the body
only rises, never sinks, for the same reason. A crab scuttles sideways: the root moves along X.

## Who owns what

- **Engine:** spawning, the gait and weapon animation on these names, weapon fire from `MUZZLE_*`,
  lock-on boxes on `WEAK_*`, eyes on `SENSOR_*`.
- **Art (cloud):** `tools/blender/build_alien_mechs.py` on the kit `tools/blender/mech_kit.py`,
  exporting `assets/models/alien_*.glb` and the measured manifest `assets/models/aliens.json`,
  and the review page `docs/mockups/alien_mechs.html`.
