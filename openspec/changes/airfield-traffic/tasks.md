# Tasks: airfield-traffic

## 1. Write-up
- [x] 1.1 Proposal, design, spec; `openspec validate --all`

## 2. Sites
- [ ] 2.1 Stand, taxi, hold, runway, pad and service marks in the airport and base templates
- [ ] 2.2 `TaxiGraph::of_site`, edge reservation, runway as one resource; tests

## 3. Civil traffic
- [ ] 3.1 New phases (stand, pushback, taxi, hold, line up, vacate); parked aircraft as starting state
- [ ] 3.2 Routes between civil airports and off-map
- [ ] 3.3 Helicopter pads and phases; city helipads
- [ ] 3.4 Near and far simulation
- [ ] 3.5 Ground support vehicles: models, service lanes, turnaround visits

## 4. Military traffic
- [ ] 4.1 Military kinds from `military-base`
- [ ] 4.2 Patrols, supply runs, helicopter patrols
- [ ] 4.3 Hostility and scramble from the alert apron, through `cb_core::units` brains

## 5. Stealing
- [ ] 5.1 `take` and stand release for aircraft
- [ ] 5.2 Headless stills; desktop playtest of an airport and a base
