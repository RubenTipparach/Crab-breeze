# Design: military-base

## 1. The site (2,780 x 950 m inside the fence)

Laid along its runway, with the mech area kept from the current base:

| Part | Size | Notes |
| --- | --- | --- |
| Main runway | 2,400 x 45 m | Two thresholds, edge, centre and approach lights, arrester cable at each end |
| Parallel taxiway | 2,400 x 23 m | 180 m from the runway centre line, four links |
| Hardened aircraft shelters | 6, each 26 x 36 x 11.5 m | Concrete arches, blast doors, one fighter each |
| Alert apron | 2 shelters at the runway's departure end | The scramble pair |
| Cargo apron and hangar | 300 x 160 m, hangar 90 x 70 x 24 m | Two cargo stands |
| Helicopter line | 6 pads, 25 m each, and a maintenance hangar | |
| Mech hangar bays | The current base's bays, terminals and repair bay | Kept as built |
| Control tower, fire station | Tower 32 m | Beacon on top |
| Fuel farm | 4 tanks, bunded | |
| Munitions bunkers | 6, behind earth berms | |
| HQ, barracks, mess | Low blocks | Lit windows at night |
| Radar dome, SAM sites | 1 radar, 2 SAM sites | Rotating radar |
| Perimeter | Fence, lights, a gate with a guard post on the access road | |

Built the civil airport's way, so the two are at one fidelity and break by one rule: a three.js site page
(`tools/mockups/military_base_src.html`) lays the site out from the airport's own runway, taxiway, hangar, tower and
light functions (`tools/mockups/airfield_kit.js`, moved out of the airport page so both pages share them) and the
small-buildings kit's buildings (`docs/mockups/site_buildings_kit.js`), and `tools/buildings/export_sites.cjs` turns
the page into game kits, every building a breakable kit through the one kit writer, as it does the airport's. Night
lights are data in the page (masts, glow dots, the runway's lights by `runwayDots`), exported with the site. The
mech hangars are the game's own (`mechHangar`, 44 x 50 x 34 m). Marks for the taxi graph (`airfield-traffic`) are
the routes the page draws.

## 2. The aircraft

| Kind | Length x span x height | Notes | Budget LOD0 / LOD1 / LOD2 |
| --- | --- | --- | --- |
| `fighter` | 19.4 x 13.0 x 5.6 m | Twin engines with afterburner nozzles, twin canted fins, bubble canopy, tricycle gear, wing and belly pylons | 18,000 / 5,000 / 1,500 |
| `military_cargo` | 29.8 x 40.4 x 11.7 m | High wing, four 4-blade turboprops, T-less tail, rear ramp hinged, side doors, gear in sponsons | 22,000 / 6,000 / 2,500 |
| `military_helicopter` | 17.6 x 16.4 (rotor) x 4.4 m | Four-blade main rotor, tail rotor, stub wings with pods, crew doors | 15,000 / 4,000 / 1,200 |
| `civil_helicopter` | 13.0 x 11.0 (rotor) x 3.4 m | Four-blade rotor, skids, cabin of six, two engines | 10,000 / 3,000 / 900 |

Each gets damage zones (wings, tail, engines, rotor, cockpit), a rig (gear, canopy, ramp, rotors, flaps and
control surfaces), lamps (nav, beacon, strobe, formation strips for the fighter), an interior seen through the
glass (the cockpit seats and panel, the cargo hold), an `EYE_pilot` and door marks for stealing. Liveries: grey or
green camouflage for the military with the island's own roundel, a white and colour scheme for the civil helicopter.

## 3. The mockup page

From `tools/mockups/gen_military_base.py`, on the airport page's pattern, the models loaded as on the airliner page: glTF JSON files beside the page, orbit and
zoom, the pilot and the Foreman as yardsticks, toggles per exhibit, gear and rotor and ramp controls, a day/night
switch with the lamps, the taxi graph and approach cones drawn. Published and checked in under
`docs/artifacts/military-base/`.

## Open questions

| # | Question | Provisional answer |
| --- | --- | --- |
| B1 | Fighter: fixed wing or swing wing? | Fixed, twin tail |
| B2 | The island's roundel colours | Rust red and bone white |
| B3 | Should the base's runway be the 2.4 km of this design, or keep today's 1.5 to 2 km fit? | 2.4 km where it fits, else the longest that fits down to 2 km |
