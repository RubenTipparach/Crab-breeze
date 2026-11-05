# Proposal: fly-stolen-aircraft

## Why

The owner, 2026-10-30: "The player can steal those and planes as well." In the plan questions the same day: flown
from a **chase camera** (the car exception extended to aircraft), with a **simulated** flight model.

Only cars can be entered today, and the code for that (`PilotMode::{EnteringCar, Driving, ExitingCar}`,
`CarAccess`, `chase_view` reading `TrafficSim`) is car-only. No aircraft anywhere takes player input.

## What changes

- **One vehicle flow** for cars, planes and helicopters: entering, operating and leaving, with the car's behaviour
  kept exactly (CLAUDE.md section 6.1: one implementation).
- **The chase camera for aircraft**, the same rules as the car's (`pilot/chase.rs`), sized to the aircraft. Getting
  in and out stay first person and ease into and out of it with no cut.
- **A simulated flight model** in `cb-core`: fixed-wing (lift, drag, stall, thrust, control surfaces) and rotorcraft
  (collective, cyclic, pedals, rotor thrust, torque, ground effect), per kind in data.
- **Assists** on by default, each one switchable off: stall protection, auto-trim, helicopter hover hold.
- **A flight HUD** whose numbers are the model's own: airspeed, altitude, vertical speed, attitude, throttle, stall
  warning.
- **The law**: CLAUDE.md's first-person law gains the aircraft exception with the owner's words.

## Impact

- `cb_core::pilot` (vehicle modes), a new `cb_core::flight_model`, `data/aircraft/*.ron`, `cb-app::{pilot, traffic,
  air_traffic, interact}`, input action maps, `CLAUDE.md`.
