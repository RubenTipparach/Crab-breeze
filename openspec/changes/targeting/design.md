# Design: targeting

## 1. What is a target

Every live mech with an AI pilot (`Enemy`), within `max_range_m` (2 000 m) of the pilot's eye. The
order is stable: by `MechId`. Selection is a `MechId`, so it survives others dying; a selected
target that dies passes to the one nearest the crosshair.

## 2. Selection and lock (`cb-core::targeting`, pure, tested)

- `nearest_to_aim(eye, aim_dir, targets) -> Option<id>`: the smallest angle between the aim and the
  direction to each target's torso (9.8 m over its feet), within `pick_cone_deg` (30 degrees);
  ties by id.
- `next(selected, targets, eye) -> id`: the next target by distance, wrapping.
- `Lock::step(off_deg, range_m, dt) -> bool`: builds while the target is within `lock_range_m`
  (1 200 m) and `lock_cone_deg` (12 degrees) of the aim, falls back at twice the rate otherwise,
  locked at `lock_s` (1.0 s). The cockpit reads `IN RANGE` while the target is inside the range.

## 2a. Missiles (`cb-core::weapons`)

`WeaponSpec` gains `seeker_turn_deg_s` (0 for guns and rockets). A missile is a rocket (no drop)
with a `target: Option<u32>` (the `MechId` locked when it was fired). Each fixed step before it
flies, `home(shot, target_pos, turn_deg_s, dt)` turns its velocity toward the target's torso by at
most `turn * dt`, keeping its speed: a pure function, tested
(`a_missile_turns_toward_its_target_no_faster_than_its_seeker`,
`an_unlocked_missile_flies_straight`). A target that dies leaves the missile flying straight.

| Missile | Speed | Seeker | Interval | Penetration | Damage | Blast |
| --- | --- | --- | --- | --- | --- | --- |
| shoulder missile | 260 m/s | 70 deg/s | 1.2 s | 150 mm | 45 | 2.5 MJ |

At 260 m/s and 70 degrees a second its tightest turn is a 213 m radius, so a raider crossing fast
at close range can slip it; at range it hits. Fire group 2 (`fire_missile`), from `MISSILE_POD`, or
`MUZZLE_R` on a mech without the socket.
- The lead point is `weapons::lead(eye, torso, velocity, muzzle speed, drop)` of the first
  weapon: no second formula.

Tests: `the_target_nearest_the_crosshair_is_picked`, `next_cycles_by_distance_and_wraps`,
`a_lock_builds_in_a_second_in_range`, `no_lock_out_of_range`, `looking_away_breaks_the_lock`.

## 3. Drawing (`cb-app::hud`)

A pool of 32 box nodes (an enemy count above that is not boxed past the 32 nearest). Each frame,
each target's torso is projected with `Camera::world_to_viewport`; behind the camera it is not
boxed. The box's size is the mech's 12 m height projected, clamped to 18-160 px. The lead marker
is a small ring at the projected lead point. The arrow sits on an ellipse inset 60 px from the
screen's edge, pointing along the projected direction.

Data in `data/hud/targeting.ron`: `max_range_m`, `pick_cone_deg`, `lock_cone_deg`, `lock_s`,
`box_px` range.

## 4. Cost

At most 32 projections and node moves a frame: microseconds. No new textures or meshes.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| C9 | The target cycle key | `Q`, next by distance; the middle mouse button picks the target nearest the crosshair | provisional (survey C9) |
| G13 | Missile count | Unlimited for now, one each 1.2 s; a magazine and reloads with the ammunition change | provisional (survey G13) |
