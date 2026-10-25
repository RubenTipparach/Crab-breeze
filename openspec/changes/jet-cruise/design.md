# Design: jet-cruise

## 1. The rule (`cb-core::mech::locomotion`, a new mode)

`LocomotionMode::Cruise`, entered from Jet with the cruise toggle (`mode_cruise`, key 4) when height
above ground is at least `cruise.min_agl_m` (150 m) and the cruise heat is below `cruise.relight_heat`
(0.25).

Each fixed step in cruise:
- **speed** eases toward `cruise.speed_m_s` (180) at `cruise.accel_m_s2` (8);
- **heading** turns toward the view's yaw at most `cruise.turn_deg_s` (6);
- **height** holds; Space climbs and C descends at `cruise.climb_m_s` (25);
- **heat** rises by `dt / cruise.heat_s` (`heat_s` 240);
- **exit to Jet** on the toggle, on heat reaching 1, on any hit (`took_hit`), or below `min_agl_m`;
  the speed is kept and decays under the jet's drag as usual.

Out of cruise, heat falls by `dt / cruise.cool_s` (60).

The guns hold in cruise (the fire controls do nothing), as they do in free look.

## 2. Data

`cruise:` in each mech's data, defaults in code (all provisional on survey J2):

| Key | Default |
| --- | --- |
| `min_agl_m` | 150 |
| `speed_m_s` | 180 |
| `accel_m_s2` | 8 |
| `turn_deg_s` | 6 |
| `climb_m_s` | 25 |
| `heat_s` | 240 |
| `cool_s` | 60 |
| `relight_heat` | 0.25 |

At 180 m/s, 240 s of cruise covers 43 km: the whole island once, so a job and the way back need a
stop at a base or a cool-down on the ground.

## 3. Cockpit

The FLIGHT MFD shows CRUISE, the speed, and a heat bar from the same `cruise_heat` the rule uses.

## 4. Cost

One more branch in the locomotion step: no measurable CPU. Streaming the world ahead at 180 m/s is
the real cost (tiles, trees, cities ahead of the mech): measured on the 3070 in the first playtest,
with the hitch log (perf_hud) watched for tile loads.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| J2 | How you get to a job | Cruise as above | provisional (survey J2) |
