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

## As built (2026-10-30)

What the code does now, and where it differs from the plan above.

- **One vehicle flow, kept by its car names.** Rather than renaming the car flow, `CarAccess` gained a `vehicle`
  (`Vehicle::Car` or `Vehicle::Aircraft`); `PilotMode::{EnteringCar, Driving, ExitingCar}` and `step_car` carry an
  aircraft unchanged (reach, facing, stopped, the eye path, thrown when wrecked), so the car tests run as they were. The
  rename can follow once nothing else is in flight on these files.
- **Which aircraft.** Any aircraft of a kind in `data/aircraft/flight.ron`, standing still on the ground and whole, whose
  door is nearer the pilot than any car's (`stolen_aircraft::find_aircraft`). The door and the seat's eye are the model's
  `DOOR_*` and `EYE_pilot` marks (now read into the manifest); the airliners, the freighter and the block-out kinds have
  no marks yet, so theirs stand on the left a fifth of the way back with the eye ahead, by the kind's size
  (`Aircraft::boarding`; task 2.3 stays open for those models).
- **It stays in the air traffic.** `Traffic::steal` takes it off its stand and out of the traffic's care
  (`Phase::Piloted`); every fixed step the flight model's place is written back (`Traffic::fly`), so it is drawn, hit,
  broken and crashed by the traffic's own code. Flown onto the ground faster than 7 m/s down or banked past 0.6 rad, it
  crashes (`Traffic::crash`) and the pilot is thrown clear. Left on the ground, it stays where the pilot left it.
- **Flight model** (`cb_core::flight_model`): as sections 2 and 3, with these simplifications. The gear is one contact
  (tyre side grip, rolling resistance, brakes, nose-wheel steering that fades with speed, no roll and no nose below level
  on the ground), not three spring-dampers. The rotorcraft's cyclic and pedals command rates the airframe reaches through
  a first-order lag, which is how a stability-augmented helicopter feels; thrust is along the body's up. Pitch, roll and
  yaw damping are one set for every plane. Control authority is given in plain terms per kind (`elevator_alpha_rad`,
  `roll_rate_deg_s`, `rudder_beta_rad`) and the coefficients derived from them. A lost engine's thrust arm yaws toward
  the dead side (`Damage::engine_lost`, the engine's side read from its damage zone's capsules).
- **Data** is one file, `data/aircraft/flight.ron`, planes and rotors keyed as the traffic's kinds (not a file per
  kind); `GameData::load` refuses a kind that is not a traffic kind or is flown as the wrong one, and the tests check
  every model's `rest_m` against its manifest.
- **Input.** The movement keys are the stick (forward pushes the nose down), turn is the rudder or pedals, lift and
  throttle walk the throttle or collective up and down and it stays where it is left, full stop is the brakes. The skate
  toggle (Shift) raises and lowers the gear (never while standing on it), and the weapon wheel steps the flaps through
  up, take-off (half) and landing (full); the readout line shows both. The assists are all on;
  the settings to turn them off are not built.
- **Camera.** The car's chase camera, generalised: `chase_view` follows a car or the stolen aircraft by the access's
  `vehicle`, at a distance from its length. CLAUDE.md's law, sections 8 and 9 carry the exception.
- **HUD.** The speed readout under the view shows airspeed, height over the ground, climb, throttle and the stall
  warning while flying, the step's own numbers.
- **Not built yet:** a stolen fighter's weapons (P1), bailing out (P2), the assist settings, a
  cockpit view (it is flown from the chase camera only), the scripted pilot's flight beats, and the response to a theft
  at a civil airport (F1 of `airfield-traffic`). A theft at a base scrambles it only if the pilot then fires on its
  aircraft. Cost: one flight step per fixed step for the one aircraft flown; not measured.
