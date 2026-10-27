# Design: car-chase-camera

## 1. Where the camera is

Pure rules in `cb_core::pilot::chase` (`ChaseCamera`, tuned by `car_camera` in `data/pilot/pilot.ron`):

| Quantity | Rule | Default |
| --- | --- | --- |
| Orbit point | over the car's middle, its roof plus `pivot_above_roof_m` | roof + 0.6 m |
| Distance | `distance_m + distance_per_length x length` | sedan 8.1 m, bus 16.9 m, semi 22.3 m |
| Pitch | `rest_pitch_deg` plus the player's own, within `pitch_range_deg` | -12 degrees, -60 to 30 |
| Yaw | the car's heading (lagged `heading_lag_s`) plus the player's yaw from the nose | 0.25 s |
| Follow | the orbit point eases after the car (`follow_lag_s`) | 0.08 s |

The camera stands `distance` back along the view from the orbit point (`wanted_eye`). The boom is walked out from
the orbit point in 0.25 m steps; at the first point inside a building's box or within 0.3 m of the ground
(`WorldPlan::solid_at`, the same test the traffic's line of sight uses) the camera stops `clip_margin_m` (0.4 m)
short, never nearer the orbit point than `min_distance_m` (1.5 m) unless the wall is nearer still (`clip`).

## 2. The mouse, and drifting back

The player's yaw from the car's nose (`PilotState::seated_rel_rad`) and pitch are stepped in `step_car`
(`PilotMode::Driving`): the mouse turns them freely, the yaw all the way round (the seated mech view's 90 degree
limit does not apply), the pitch kept inside the range so the mouse never winds up past an end. With the mouse still
for `recenter_after_s` (2 s) and the car faster than `recenter_min_speed_m_s` (2 m/s), both drift back to zero at
`recenter_deg_s` (90 degrees a second): behind the car, at the rest pitch (`ChaseCamera::recenter`). Moving the mouse
stops the drift at once and restarts the wait (`PilotState::look_idle_s`).

This is a change to "never take camera control away from the player" (CLAUDE.md section 8): the view following the
car's heading is what a chase camera is and what the owner asked for; the drift back is the recommendation and is
provisional (DC1).

## 3. Getting in and out

`cb-app::pilot::place_camera` keeps a blend, 0 at the pilot's eyes and 1 at the chase camera, stepped toward 1 while
`Driving` and toward 0 otherwise over `blend_s` (0.6 s) by the same rule that eases the seated field of view
(`fov_blend`), and places the lens between the first-person eye and the chase place with a smoothstep of it
(`blend_view`, position lerped, rotation slerped). Getting in, the eyes are carried to `EYE_driver` in first person
(0.8 s, `car_step_s`), then the camera eases out; getting out, it eases back in while the eyes are carried to the door.
If the car is lost mid-blend, the last chase place is held, so the view never cuts.

## 4. Field of view

`PilotMode::wide_view` is true only in the mech's seat; `pilot::ease_fov` widens the lens on it alone, so driving
keeps the on-foot field of view (owner: "car driving should remain at 75", with the Options offset, `seated-fov`).

## 5. Cost

One boom walk a frame: at most 70 steps for the semi, each a terrain sample and a grid lookup, about 10 microseconds.
Not measured (no change to the drawing).

## 6. As built (2026-10-26)

Built as above. Stills (`CB_STILLS_CARS=drive --stills <dir>`, `cb-app::capture_cars`, drawn offscreen):
`docs/screenshots/cars/drive-*.png`, getting in, easing out, parked, driving, orbited 120 degrees by the mouse, and
getting out. Measured from the stills' log: the camera 7.7 to 9.2 m from a van's middle and 4.8 m over its ground
while driving. The drift back is pinned by tests; the stills' car crashed and stopped before it could be seen in the
game (it drifts back only while the car moves).

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| DC1 | Should the view drift back behind the car by itself? | Yes, after 2 s of a still mouse while moving; parked it stays put | provisional |
| DC2 | The dashboard's speedometer is not seen from outside: show the speed on the HUD while driving? | Yes, a small speed readout low on the screen, from the simulation's speed (`car::speed_readout`) | provisional, built |
| DC3 | The camera clips against buildings as generated, not as damaged: a fallen building still pushes it in | Leave it until it is seen to matter | provisional |
