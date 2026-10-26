# Tasks: harbours

## 1. Rules (cb-core, tested)

- [x] 1.1 `data/world/ships.ron` loaded and validated with the other data files
- [x] 1.2 Harbours: coastal cities, the berth bearing, clear of the bases; tested on seed 42
- [x] 1.3 Sea lanes: A* over open water, coast and islet clearance, string pulling; tested
- [x] 1.4 Boats: route following, coast probes, spacing, slowing for harbour, mooring, fishing grounds, lake loops, the ferry; tested
- [x] 1.5 Floating on the water's surface and its swell; tested
- [x] 1.6 Hits by the vehicles' armour rule (`units::armour_hit`), sinking, wrecks and slicks; tested
- [x] 1.7 Shoving and swamping by a wading or skimming mech; tested
- [x] 1.8 Fleeing from blasts, hostiles and hits; tested
- [x] 1.9 Crabs taking a ship as their target; tested
- [x] 1.10 The standing rule and its once-only reports; tested
- [x] 1.11 Shipping jobs: three kinds sited at harbours, the ward's loss fails the job; tested
- [x] 1.12 Gunboats as vehicles: on the water, clear of the shallows; tested
- [x] 1.13 The timetable: spawn and despawn round the pilot, seeded; tested

## 2. In the game (cb-app)

Built and unit-tested; not yet run in the game (no playtest on this branch).

- [x] 2.1 Harbours drawn: quay (walkable), cranes, warehouses, office and its terminal
- [x] 2.2 Sea traffic, lake boats and the ferry drawn and stepped, floating, with bow spray
- [x] 2.3 Boats in the hit list; sinking, wrecks and slicks drawn
- [x] 2.4 Mechs shove and swamp boats; boats flee; crabs go after ships
- [x] 2.5 `BoatHarmed` written for the lead to wire into standing
- [x] 2.6 Shipping jobs started: gunboats, the escort's freighter, the blockade

## 3. Models and pages

- [x] 3.1 `tools/blender/build_boats.py`: freighter, tanker, fishing boat, rowing boat, launch, ferry, gunboat, crane, quay at true size, with the manifest and budgets
- [x] 3.2 `cb-tools validate` checks the boat models against their data
- [ ] 3.3 The orbitable mockup page (`tools/mockups/gen_boats.py`), not published

## 4. Proof

- [x] 4.1 Step cost measured (`cb-tools ships`)
- [ ] 4.2 Playtest: skim out to a freighter and escort it into harbour (the lead's session)
