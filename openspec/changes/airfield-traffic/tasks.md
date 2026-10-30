# Tasks: airfield-traffic

## 1. Write-up
- [x] 1.1 Proposal, design, spec; `openspec validate --all`

## 2. Sites
- [x] 2.1 Stand, taxi, hold, runway and pad marks in the airport template, and a laid-out network per base (service marks still open)
- [x] 2.2 `TaxiGraph`, runway as one resource; tests (edge reservation still open)

## 3. Civil traffic
- [x] 3.1 New phases (stand, pushback, taxi, hold, line up, vacate); parked aircraft as starting state
- [x] 3.2 Routes between civil airports (off-map routes still open)
- [x] 3.3 Helicopter pads and phases (city helipads still open)
- [ ] 3.4 Near and far simulation
- [ ] 3.5 Ground support vehicles: models, service lanes, turnaround visits

## 4. Military traffic
- [x] 4.1 Military kinds from `military-base`
- [x] 4.2 Patrols, supply runs, helicopter patrols
- [x] 4.3 Hostility and scramble, through `cb_core::units` brains: over the pilot a scrambled fighter or gunship is handed to the units (`base_fighter`, `base_gunship`) and attacks with their weapons, drawn as its own model

## 5. Stealing
- [x] 5.1 `Traffic::steal` and stand release for aircraft
- [ ] 5.2 Headless stills; desktop playtest of an airport and a base
