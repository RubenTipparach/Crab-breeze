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

## 4. Placing it in the world (task 3.5)

Today's four bases are block-outs laid by `world::bases`: a rectangle beside a highway, a 1.5 to 2 km runway fitted
beside it, and hangar bays, terminals and a tower drawn by `cb-app/src/air_bases.rs` from `BasesConfig` formulas.
`AirBase` carries them, and about thirty places read it (refits and repairs in the bays, the terminals, air drops,
civilians' trips, ships, lamps, the traffic). The page's base replaces the block-out the airport's way:

1. **A site template.** `SiteKind::MilitaryBase`, loaded from `assets/models/sites/military_base/` beside the airport
   and the seaport. Its buildings are kits (breakable, lit at night, dark when damaged) through the code that draws the
   airport's; its ground and lights are the export's.
2. **Placement.** `place_bases` places the template as `place_airports` places theirs: flat ground for the whole
   2,780 x 950 m footprint, clear of cities, civil sites and every runway's approach cones, its own cones clear of
   everything, its access road from the nearest city. Four of them, the old bases' order kept where they fit.
3. **`AirBase` from the site.** Its rectangle is the footprint, its runway the page's `rwy_a` and `rwy_b` marks (2.4 km;
   B3's shorter fit is dropped: a template has one runway), its hangar bays and terminals the page's three mech hangars,
   exported as marks (`bay_i`, `terminal_i`), not `BasesConfig` formulas. `AirBase::hangars`, `terminals` and
   `structures` keep their signatures and read the marks, so the thirty readers do not change.
4. **Its taxi network** is exported from the page as the airport's is (stands, shelters, pads, the parallel taxiway,
   the holding points), and `air_traffic::fields` reads every site the same way. The generated base layout goes.
5. **The block-out goes.** `air_bases.rs` keeps only what is gameplay (servicing a mech in a bay, the terminals); the
   drawing of hangars, aprons and towers is the site's.

**Cost.** One base is 33 placements of 13 kits and a ground of 16,236 triangles in 20 surfaces, its 17 masts and 349
lamp dots (the export). Four bases: about 130 placements, 65,000 ground triangles; kits are shared, so the texture cost
is one base's. To be measured on the owner's machine with the airport's.

**Risk.** The footprint is about 2.6 times today's base area. Whether four fit the island's flat ground beside the two
airports, two seaports and the cities is measured first (`cb-tools sites 42`); if they do not, the question goes to the
survey (fewer bases, or a smaller variant of the page).

### As built (2026-10-30)

- `SiteKind::MilitaryBase` loads `assets/models/sites/military_base/`; the base page exports its runway ends, its taxi
  network (four links onto the runway, the parallel taxiway, every stand and pad) and its three bays as marks.
- `sites::place_runway_sites` places it four times by the airports' own search, clear of every earlier site and cone.
  Four did not fit within an airport's reach and rise on seed 42 (one did), so a base has its own limits in
  `SitesConfig`: `base_reach_m` 20 km past a city's square and `base_max_rise_m` 150 m of natural rise, levelled by its
  pads. On seed 42 all four fit, turned a quarter (north-south runways); two stand at 384 m and 444 m.
- `AirBase::of_site` reads the base off its site: footprint, first runway, bays (`bay_0` to `bay_2`, a `hangar_m` inside
  each, mouths toward the page's -z), tower; `hangars`, `terminals` and `tower` read them, so their readers are unchanged.
  A block-out base (no template shipped) is as before.
- The traffic reads a site-built base's field from its page's network (`fields::site_field`, shared with the airports);
  the generated base layout stays only for block-outs.
- `air_bases.rs` draws only a site-built base's terminals, turned to its bays; the site draws everything else. Its runway
  is lit by its site's lights.
- Not yet measured on the owner's machine: four bases add about 130 placements and 65,000 ground triangles.

## Open questions

| # | Question | Provisional answer |
| --- | --- | --- |
| B1 | Fighter: fixed wing or swing wing? | Fixed, twin tail |
| B2 | The island's roundel colours | Rust red and bone white |
| B3 | Should the base's runway be the 2.4 km of this design, or keep today's 1.5 to 2 km fit? | 2.4 km: a placed template has one runway (section 4) |
| B4 | If four 2,780 x 950 m bases do not fit the island, fewer bases or a smaller page? | They fit, with a base's own reach and rise limits (as built) |
