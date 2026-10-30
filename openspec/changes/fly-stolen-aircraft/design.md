# Design: fly-stolen-aircraft

## 1. One vehicle flow

`CarAccess` becomes `VehicleAccess { id, kind: Car | Plane | Heli, door, eye, centre, heading_rad, speed_m_s,
wrecked, size_m }`. `PilotMode::{EnteringCar, Driving, ExitingCar}` become `EnteringVehicle(EyePath)`,
`Operating`, `ExitingVehicle(EyePath)` with the vehicle's kind beside them. `step_car` becomes `step_vehicle`; its
rules (reach, facing, stopped, the eye path, thrown when wrecked) are unchanged. A car keeps every behaviour it has;
the car tests run unchanged against the new names.

`place_camera` blends to the chase view when the mode is `Operating`, and `chase_view` reads the vehicle's pivot,
length and height from a `VehicleView` resource each vehicle system fills, not from `TrafficSim`.

Entering an aircraft: only on the ground and stopped (under 1 m/s), by its door (airliners: the forward left door
via the stairs or jet bridge if one stands there; fighters: a ladder; helicopters: the cabin door). The eye path ends
at the aircraft's `EYE_pilot` empty (added to every aircraft model).

## 2. Fixed-wing model

State: position, velocity, attitude (quaternion), angular rates, throttle, flap and gear state. Per fixed step
(1/60 s):

- Angle of attack `a` and sideslip `b` from the air-relative velocity in the body frame.
- Lift `L = q S CL(a)`, `CL` linear (`cl_alpha` per radian, from `cl0`) up to `a_stall`, then falling to
  `cl_post_stall` over 6 degrees. Flaps add `cl_flap` and shift `a_stall` down 2 degrees.
- Drag `D = q S (cd0 + k CL^2 + cd_gear + cd_flap)`, `q = 0.5 rho V^2`, `rho` from a standard atmosphere.
- Thrust `T = throttle * thrust_n * engines_alive / engines`, falling with altitude by density ratio.
- Moments from the control surfaces (`cm_elevator`, `cl_aileron`, `cn_rudder`) and stability derivatives
  (`cm_alpha`, `cl_beta`, `cn_beta`) and damping (`cm_q`, `cl_p`, `cn_r`), against the kind's inertia.
- Ground: the landing gear as three spring-damper points with brakes and nose-wheel steering, from the manifest's
  gear positions and `rest_m`.

Damage: a broken zone removes its surface (an aileron, a tailplane: its derivative goes to zero and its drag is
added) and a lost engine its thrust, through the zone tree already in `air_traffic::model`.

## 3. Rotorcraft model

Rotor thrust `T = collective * thrust_max_n * ground_effect(h / rotor_d)`, along the rotor disc's normal, tilted by
cyclic (up to 10 degrees). Torque from collective is balanced by the tail rotor's pedals; a lost tail rotor leaves
it unbalanced (the aircraft spins). Translational lift adds up to 15 % thrust between 8 and 20 m/s. Drag from a
body drag area. Skids or wheels as ground points.

## 4. Assists

- **Stall protection** (planes): the elevator is limited so `a` stays 2 degrees under `a_stall`.
- **Auto-trim** (planes): holds the pitch attitude when the stick is centred.
- **Hover hold** (helicopters): with the cyclic centred, cyclic is driven to null ground speed and collective to
  hold height.

Each is a setting, on by default (survey G2 and G5: deep piloting with assists that can be turned off). An assist
writes the same control values the player would; the model never sees a shortcut.

## 5. Input

Actions in data: `pitch`, `roll`, `yaw`, `throttle` / `collective`, `flaps`, `gear`, `brake`, mapped from the same
devices as the mech (mouse and keyboard, gamepad). The scripted pilot writes the same actions.

## 6. The camera and the law

The chase camera follows the aircraft's velocity-aligned heading, with distance from the aircraft's length
(`distance_for`). The mouse orbits it freely; after 2 s still it drifts back behind (the car's provisional DC1). Field
of view 75 degrees. CLAUDE.md's law block gains: "Flying a stolen aircraft is third person too (owner, 2026-10-30,
plan questions: chase camera)", and sections 8 and 9 the same.

## 7. Data and tests

`data/aircraft/<kind>.ron` per flyable kind: mass, inertia, wing area, span, the coefficients above, thrust, rotor
values, with units in the keys. Starting numbers from public data for the real aircraft each model is after.

Tests: lift equals weight at the trim speed for each kind within 2 %; stall at `a_stall` (CL peaks there); a hover
holds within 0.5 m over 30 s with hover hold on; the same input gives the same path (golden seed); a lost engine
yaws the plane toward it.

## Open questions

| # | Question | Provisional answer |
| --- | --- | --- |
| P1 | Can the mech's weapons be used from a stolen aircraft, or do fighters fire their own? | A stolen fighter fires its own guns and missiles; others are unarmed |
| P2 | Can the pilot bail out in the air? | Yes: a parachute down, first person |
| P3 | Fuel | Ignored, as the mech's is (survey M3) |
