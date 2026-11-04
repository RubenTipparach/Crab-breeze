# City Layouts: Today and Future

Published: https://claude.ai/artifact/8TEjkZ4V2CpHyj2doztEHf

**What it shows.** Two cities on one ground, on one camera, at true scale, with a 24 m block for the mech. You can see them side by side or one at a time, from the air, at mech height or in plan.

- **Today** is the city generator's own rules and numbers (`cb_core::world::layout`, `data/world/world.ron`).
- **Future** is the proposal in `openspec/changes/city-districts`: a Fargo-like city, after the owner's reference. It comes in variations:
  - the downtown small or large;
  - its shape compact, along one main street, or in two cores;
  - the highway at grade (only arterials cross, at exits), under overpasses (every street bridges over it on embankments), elevated on a viaduct (the streets run on under it), or a bypass with a spur avenue;
  - real exits after the owner's interchange chart: a SPUI by the downtown, diamonds, a single quadrant, a partial cloverleaf and a cloverleaf at the ends, a trumpet where the spur meets the bypass;
  - inland or on a coast.

**What made it.** The page itself is `city_layouts.html`: three.js r128 from cdnjs, and every building generated in the page from a fixed seed. Its screenshots are in `docs/screenshots/city-districts/`.

**Drivable.** The page proves its own roads: `driveCheck` fills the road graph from the downtown (every road reached), checks clearances (4.5 m), grades (8 %), turns (400 m highway, 25 m ramp), ramp ends and piers, and a bot drives 12 trips whose cars you can watch on the page. The result is printed under the Future view (PASS or FAIL).

**Its check.** `check_roads.js` loads the page in headless Chromium and counts the building triangles that stand over the highway in every variation. Road structures (decks, ramps, bridges, piers, embankments) are their own mesh and are not counted. The count must be 0, and the drive check must pass, in all 14 combinations it tries (the four highway modes, both downtowns, all three shapes, the coast). It exits non-zero on any failure. It was up to 1,330 with the bypass before the fix of 2026-11-04 (owner: "buildings in the middle of the highway").
