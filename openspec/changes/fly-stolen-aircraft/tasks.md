# Tasks: fly-stolen-aircraft

## 1. Write-up
- [x] 1.1 Proposal, design, spec; `openspec validate --all`
- [x] 1.2 CLAUDE.md law, sections 8 and 9: the aircraft chase-camera exception

## 2. One vehicle flow
- [x] 2.1 `CarAccess::vehicle` through the car's pilot modes and `step_car`; car tests unchanged
- [x] 2.2 `chase_view` follows a car or an aircraft; `place_camera` on `Driving`
- [x] 2.3 `EYE_pilot` and door marks on every aircraft model (the freighter, built by another Blender version, keeps the size-based stand-in)

## 3. Flight model (`cb_core::flight_model`)
- [x] 3.1 Atmosphere, fixed-wing forces and moments, gear contact
- [x] 3.2 Rotorcraft: rotor, torque, tail rotor, ground effect, translational lift
- [x] 3.3 Damage zones feed surfaces and engines
- [x] 3.4 Assists: stall protection, auto-trim, hover hold (each a setting on the Options page)
- [x] 3.5 `data/aircraft/*.ron`; trim, stall, hover and golden-seed tests

## 4. App
- [ ] 4.1 Input actions for flight (built); scripted pilot beats (open)
- [x] 4.2 Flight HUD from the model (the speed readout line)
- [ ] 4.3 Headless stills; desktop playtest: steal a helicopter, fly, land, get out
