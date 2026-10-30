# Tasks: fly-stolen-aircraft

## 1. Write-up
- [ ] 1.1 Proposal, design, spec; `openspec validate --all`
- [ ] 1.2 CLAUDE.md law, sections 8 and 9: the aircraft chase-camera exception

## 2. One vehicle flow
- [ ] 2.1 `VehicleAccess`, vehicle pilot modes, `step_vehicle`; car tests unchanged
- [ ] 2.2 `VehicleView` for `chase_view`; `place_camera` on `Operating`
- [ ] 2.3 `EYE_pilot` and door marks on every aircraft model

## 3. Flight model (`cb_core::flight_model`)
- [ ] 3.1 Atmosphere, fixed-wing forces and moments, gear contact
- [ ] 3.2 Rotorcraft: rotor, torque, tail rotor, ground effect, translational lift
- [ ] 3.3 Damage zones feed surfaces and engines
- [ ] 3.4 Assists: stall protection, auto-trim, hover hold
- [ ] 3.5 `data/aircraft/*.ron`; trim, stall, hover and golden-seed tests

## 4. App
- [ ] 4.1 Input actions for flight; scripted pilot beats
- [ ] 4.2 Flight HUD from the model
- [ ] 4.3 Headless stills; desktop playtest: steal a helicopter, fly, land, get out
