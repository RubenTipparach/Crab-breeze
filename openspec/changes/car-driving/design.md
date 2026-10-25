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

A rigid body on four wheels, simulated in the plane with suspension heights from the ground:
- **Engine:** a torque curve over rpm, a 4-speed automatic (shift at 85 % of redline, down at 40 %),
  and a final drive.
- **Tyres:** a simplified Pacejka (magic formula) lateral force from slip angle and longitudinal force
  from slip ratio, scaled by each wheel's load. Weight transfers front and back under braking and
  side to side in turns, so a car that brakes hard into a corner slides.
- **Steering:** the steering angle eases to the input at 2.5 rad/s, with the maximum angle falling
  with speed (assist, can be turned off: survey G5).
- **Suspension:** each wheel is a spring and damper over the ground height (terrain, roads, craters,
  rubble), so kerbs bump and craters drop a wheel.
- **Collisions:**
  - against buildings, the collision grid's standing cells;
  - against cars, an impulse between their boxes;
  - against a mech's legs, the leg capsule. The car stops dead and takes damage from the impact
    speed.
- **Data:** `data/cars/<kind>.ron` holds the mass, torque curve, gear ratios, wheelbase, track, tyre
  constants, spring and damper rates, and the `EYE_driver` and door sockets.

## 3. Cockpit view

- The interior is modelled for each kind (`tools/blender/build_cars.py`): dashboard, wheel, seats, and
  mirrors with a low-resolution rear view.
- The speedometer and tachometer needles are named nodes turned from `Car::speed_m_s` and
  `Car::rpm`: the numbers the simulation uses (6.1).
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

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| J8 | How cars handle | Simulated (engine, gears, grip, weight) with assists you can turn off | provisional (survey J8) |
| J9 | Which cars you can take | Parked cars in towns and at the bases; not traffic cars with drivers | provisional (survey J9) |
