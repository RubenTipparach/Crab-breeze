# City Layouts: Today and Future

Published: https://claude.ai/artifact/8TEjkZ4V2CpHyj2doztEHf

**What it shows.** Two cities on one ground, on one camera, at true scale, with a 24 m block for the mech. You can see them side by side or one at a time, from the air, at mech height or in plan.

- **Today** is the city generator's own rules and numbers (`cb_core::world::layout`, `data/world/world.ron`).
- **Future** is the proposal in `openspec/changes/city-districts`: a Fargo-like city, after the owner's reference. It comes in variations:
  - the downtown small or large;
  - its shape compact, along one main street, or in two cores;
  - the highway through the middle, or a bypass with a spur avenue;
  - inland or on a coast.

**What made it.** The page itself is `city_layouts.html`: three.js r128 from cdnjs, and every building generated in the page from a fixed seed. Its screenshots are in `docs/screenshots/city-districts/`.
