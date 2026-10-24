# Tasks: cockpit-alarms

## 1. Core

- [ ] 1.1 `cb-core::alarms`: inputs, `AlarmState`, `wave()`, `step()`; tuning struct loaded from `data/cockpit_alarms.ron` and validated
- [ ] 1.2 Tests: part lost strobes then holds; heat ramps from 0.6 and pulses when locked; missile beacon rate rises as it closes; priority order; hit flicker decays in 0.2 s

## 2. App

- [ ] 2.1 `bind_interior` keeps the `LIGHT_*` nodes by role (the lights still removed), `cockpit_alarms` spawns the lamps
- [ ] 2.2 Write the state into the lamps each frame; sparks through `particles` on the cockpit layer
- [ ] 2.3 Missiles: expose the locked target and closing speed of live missiles to the alarm inputs

## 3. Check

- [ ] 3.1 Headless stills: a lost arm, an overheated booster and an incoming missile, from the seat
- [ ] 3.2 A playtest on the owner's machine (section 9), and a frame-time measurement with alarms against without
