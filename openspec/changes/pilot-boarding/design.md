# Design: pilot-boarding

Tuning is in `data/pilot/pilot.ron` and `data/input/bindings.ron`.

## 1. Actions

`Actions { move_xy, look_delta_deg, throttle_axis, turn_axis, use_pressed, run, free_look,
throttle_zero, mode_request }`. The input plugin samples devices every frame in `Update` and
accumulates into the action state; the fixed step consumes it, so a mouse movement between steps is
used once. Bindings are key names in data; mouse look is 0.08 degrees a pixel.

## 2. On foot

Walk 1.6 m/s, run 4.5 m/s, reached at 12 m/s2. Feet on `height_at`, body radius 0.35 m pushed out of
buildings, eyes 1.7 m above the feet. Pitch is limited to +-85 degrees.

## 3. Boarding

> Superseded by `mech-boarding` (owner, 2026-10-24: a sealed cockpit, the mech sits, the hatch
> opens, the pilot climbs a stair). The lift below is gone; the path is now stair foot, platform,
> `EYE_pilot`, at 1.2 m/s, after the mech has sat and opened.

- Use is offered within 3.0 m (horizontal) of `LADDER_base`, with the mech's throttle at 0 and
  its speed under 0.2 m/s.
- The path, fixed when boarding starts, is for the pilot's eyes: here, above `LADDER_base` at eye
  height, straight up the lift to `HATCH_entry`'s height, across to `HATCH_entry`, then down into
  `EYE_pilot`. The eyes move along it at 2.0 m/s (a 12 m mech's lift takes about 6 s).
- The pilot keeps mouse look on the way up. On the last leg the view turns to the seat's forward,
  so arriving in the seat is one continuous motion.
- Seated, the camera is `EYE_pilot` with the free-look offset (+-120 degrees yaw, +-70 pitch, eased
  back when Alt is released). Mouse look without Alt aims the torso (`mech-rig`).
- Disembarking reverses the path and leaves the pilot standing at `LADDER_base`, facing the way the
  mech faces.

## 4. The capture instrument

`--stills <dir>` places the pilot in the street 25 m from the mech facing it, writes
`street.png`, boards and writes `cockpit.png` from the seat, sets throttle to 0.6 for 10 s and
writes `cockpit-stride.png`, then logs p50, p95 and p99 frame time over that stride and exits. It
writes only `Actions` (except the starting place), so it is the scripted pilot's first beat in
miniature and moves into `playtest-pipeline` when that lands.

## 5. Cost

One body step and one boarding step a fixed step: microseconds. One camera.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| C1 | How the pilot sees out | sealed pod, all-view screens | decided by the owner, 2026-10-24 (`mech-boarding`) |
| G2 | Piloting depth | free look on Alt, mouse aims the torso | provisional (survey G2) |
