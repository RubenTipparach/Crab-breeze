# Design: mech-rig

All tuning below is the default in `data/mechs/placeholder.ron`.

## 1. Spaces

- **Model space** is the contract's glTF space: +Y up, the mech faces +Z, its left is +X. The
  engine puts every mech under a `model` entity turned 180 degrees about Y, so model +Z is the
  world's forward (Bevy's -Z). Nothing else rotates it.
- **Bind**: once the mech's hierarchy exists (boxes at once, a glb when its scene spawns), one
  system finds every contract bone and socket by `Name`, records each bone's rest local transform
  and rest model-space rotation, and measures the leg segments from the rest positions. A missing
  bone stops the bind with an error naming the bone and the file; the mech stays unbound.
- **Drive**: the gait produces, per bone, a model-space rotation `D` relative to rest. A bone's new
  local rotation is `(D_parent * R_parent_rest)^-1 * D * R_rest`, where `D_parent` comes from the
  nearest driven ancestor. This is correct whatever rest orientation Blender gives each bone, so a
  glb's bones need no particular roll. Translations stay at rest except the pelvis (bob and sway).

## 2. Locomotion (fixed step, `cb-core::mech::locomotion`)

| Input | Effect | Tuning |
| --- | --- | --- |
| Throttle up/down (W/S) | moves the throttle lever, -0.4 to 1.0 | 0.6 a second |
| Throttle zero (X) | lever to 0 | |
| Turn (A/D) | turns the legs | 30 degrees a second |
| Mouse | torso twist and pitch targets | twist +-100 degrees at 70 degrees a second; pitch -20 to +25 degrees at 40 degrees a second |

Ground speed chases `throttle * 7.0 m/s` at 2.5 m/s2 accelerating and 4.0 m/s2 braking. The root
follows the ground (`height_at`) and is pushed out of buildings with a 3 m radius (the world's one
collision query).

## 3. Walk (`cb-core::mech::gait`)

- The gait phase advances by `|v| / L(v)` cycles a second, with stride `L(v) = max(2.0, 9.0 * |v| /
  7.0)` m per cycle. Turning in place advances it as if moving at `|omega| * 2.5 m`.
- Left foot: stance for half the cycle, sliding back from `+L/4` to `-L/4` under the hip; swing for
  the other half, forward along a sine arc 0.9 m high. The right foot is half a cycle behind.
  Amplitude and lift scale with `smoothstep(0, 0.6 m/s, |v|)` so the feet settle together at rest.
- The pelvis drops 0.25 m twice a cycle and sways 0.15 m toward the stance foot.
- **Digitigrade leg solver**: the hock (back-knee) segment keeps its rest angle, so the solver
  places the hock joint behind the foot target, then solves thigh and shin as a two-bone chain in
  the leg's plane, bending the knee the way the rest pose bends it. A target out of reach is pulled
  in to 99.5 % of the chain. The foot keeps its rest angle (flat).

## 4. Modes

`LocomotionMode::{Walk, Skate, Jet}`. Only `Walk` has a model; asking for `Skate` or `Jet` keeps
walking and logs that the mode is not built yet (the input binding exists so muscle memory does not
change when they arrive). The `WHEEL_*` and `JET_*` sockets are bound and kept for them.

## 5. The stand-in

12 m to the top of the torso, eye 9.6 m, hips 5.9 m, feet 3.2 m apart. Thigh 2.8 m, shin 2.6 m,
hock 1.1 m. About 40 boxes in 6 tones; the cockpit is a frame of pillars, a roof with a hatch gap, a
dash and a seat around `EYE_pilot`, so the city is seen through a frame as it will be through the
real canopy (survey `C1`).

## 6. A glb drops in

`model: Some("models/mech_<name>.glb")` in the mech's data loads `Scene 0` of the glb under the
same `model` entity. The bind and drive code is the same; segment lengths come from the glb's own
rest pose. No code changes.

## 7. Cost

Per fixed step: one locomotion step, microseconds. Per frame: one pose of about 30 bones, under
0.05 ms. The stand-in is about 40 boxes (480 triangles) in under 10 batched draws.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| M1 | Height | 12 m | provisional (survey M1) |
| M2 | Legs | two, digitigrade | provisional (survey M2) |
| M3 | Jet limits | fuel and heat; mode reserved, not built | open (survey M3) |
| M4 | Skating | wheels in the feet; mode reserved, not built | open (survey M4) |
| M5 | Mode switch | automatic with a manual switch; only walk exists | open (survey M5) |
| G2 | Piloting depth | throttle lever, legs and torso independent | provisional (survey G2) |
