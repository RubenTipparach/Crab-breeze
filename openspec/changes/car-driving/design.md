# Design: car-driving

## 1. Pilot modes (`cb-core::pilot`)

`PilotMode` gains three modes:
- `EnteringCar(EyePath)` and `ExitingCar(EyePath)`: the same `EyePath` that boarding a mech uses.
  The eye is carried from beside the door to the seat's `EYE_driver` empty over 0.8 s.
- `Driving`: the eye rides `EYE_driver`.

Look input stays the player's throughout, since only the position is carried (CLAUDE.md section 8;
the step-in exception is the mech's alone). Getting in needs:
- the car stopped (below 0.5 m/s);
- the pilot within 1.2 m of its `DOOR_driver` socket, facing it;
- Use pressed.

## 2. The car (`cb-core::car`, pure, fixed step)

Arcade handling (survey J8): a kinematic car, grippy and simple, with suspension heights from the
ground for the bounce:
- **Drive:** speed eases toward the throttle's target at `accel_m_s2` and brakes at `brake_m_s2`, up
  to `top_speed_m_s`; reverse at a third of that. A sound-only engine pitch follows the speed.
- **Steering:** the car turns about its rear axle at `speed / wheelbase x tan(steer)`, the steering
  angle easing to the input at 3 rad/s and its maximum falling with speed, so it never spins out on
  its own.
- **Grip:** full grip, so the car goes where it points; the handbrake (Space) lowers rear grip to
  `handbrake_grip` (0.3) for a slide, and grip returns at 2 per second when it is let go.
- **Suspension:** each wheel is a spring and damper over the ground height (terrain, roads, craters,
  rubble), so kerbs bump and craters drop a wheel.
- **Collisions:**
  - against buildings, the collision grid's standing cells;
  - against cars, an impulse between their boxes;
  - against a mech's legs, the leg capsule. The car stops dead and takes damage from the impact
    speed.
- **Data:** `data/cars/<kind>.ron` holds the mass, top speed, acceleration and braking, wheelbase,
  track, steering limits, handbrake grip, spring and damper rates, and the `EYE_driver` and door
  sockets.
- **Taking a traffic car** (survey J9): Use at a traffic car's driver door stops it (car-traffic's
  car leaves the lane graph and becomes a driven car), and its driver gets out and runs from you to
  the nearest building door. The driver is a person built from the people pipeline
  (blender-humanoid-characters); until people are in the game, a block-out figure stands in.

## 3. Cockpit view

- The interior is modelled for each kind (`tools/blender/build_cars.py`): dashboard, wheel, seats, and
  mirrors with a low-resolution rear view.
- The speedometer needle is a named node turned from `Car::speed_m_s`, the number the simulation
  uses (6.1).
- The steering wheel turns with the steering angle, and the hands follow it (the pilot's arms rig,
  shared with the mech grips).

## 4. Input

The driving actions reuse the mech's axes: throttle and turn from the same WASD actions, the
handbrake on `lift` (Space), and Use to get out. There are no new bindings, so the Controls page gains
a "Driving" group that reads the same fields.

## 5. Cost

| Item | Cost |
| --- | --- |
| One driven car | four wheel queries and a rigid-body step: about 0.02 ms a fixed step |
| Interior | about 25 000 triangles at LOD 0, one PBR atlas per kind (2048 px, about 21 MB) |

## 6. As built (2026-10-26)

- **Pilot** (`cb-core::pilot`): `EnteringCar`, `Driving` and `ExitingCar` as designed, in `step_car`, run
  before the mech's `step`. Reach 1.2 m from `DOOR_driver`, facing within 75 degrees, stopped below 0.5 m/s
  (`data/pilot/pilot.ron`). A moving car is hailed and entered once it stops. Driving, the view turns with
  the car and the mouse looks round from its nose by the seated rule (`coupled_look`). A car wrecked with
  the pilot in it throws them clear beside the door; the pilot has no health on foot yet, so "hurt" is not
  modelled. No save is written while in a car.
- **The car** (`cb-core::car`): as designed; collisions are two circles over the axles against the building
  grid, other cars (an impulse both ways) and mechs (stop dead, as one circle of the mech's body radius until
  the leg capsules are passed in). `wheel_radius_m` joins the car data so the model and the wheels' spin agree.
- **Cockpit view:** block-out interiors only: a dashboard box, `STEER_wheel` (turned at 12 times the front
  wheels' angle) and `NEEDLE_speed` (turned from `CarSpec::speedo_needle_rad`, the simulation's speed). No
  hands, no mirrors, no modelled interior or PBR atlas yet.
- **Taking a traffic car:** its driver runs for the nearest building's footprint as a 1.8 m block-out
  capsule, gone after 40 s or at the wall. Parked cars stand at town kerbs; none at the air bases yet.
- **Controls:** the Controls page lists a Driving group on the walking keys and Space; no new bindings.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| J8 | How cars handle | Arcade: grippy and simple | decided (survey J8) |
| J9 | Which cars you can take | Any car, traffic included; the driver gets out and runs | decided (survey J9) |
