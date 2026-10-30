# Design: waypoint-nav

## 1. The map image

- **Drawn once, when the world loads**, on the async pool, into a 1024 x 1024 RGBA image. That is 40 m a pixel over
  the 40.96 km island. Drawing it does not touch a frame (CLAUDE.md section 7).
- **The drawer is pure and lives in `cb-core`** (`world::map::paint(plan, px) -> Vec<[u8; 4]>`). It is tested
  headless, and a test pins its colours at the start, at sea and on a highway.
- **What it draws, back to front:**
  - land shaded by height, with a hill shade lit from the north-west;
  - water, with its depth darkening it;
  - highways, 2 px;
  - city streets, 1 px;
  - air bases, airport and seaport footprints, as outlined blocks.
- **Labels are UI text over the image** (city numbers, AIRPORT, SEAPORT, BASE), so they stay sharp at any zoom.
- **Cost:**
  - texture memory: 4 MB, one draw call for the map;
  - each marker or label is one UI node; there are at most a few dozen;
  - drawing: about 1 million height and water samples once, on the pool. It is not timed yet: a cloud session
    measures nothing, so the owner's machine measures it.

## 2. The page

- **Where it is:** a MAP button joins the pause menu's page row, with Resume, Options and Controls. The page is the
  map, 760 px square, with a list beside it:
  - the waypoints in order;
  - each one's distance from the pilot and from the one before;
  - the route's whole length;
  - CLEAR and BACK buttons.
- **Zoom:** the wheel zooms 1x to 8x round the pointer.
- **Pan:** dragging with the middle button, or with the left button held for more than 0.2 s, pans the map. A quick
  left click puts down a waypoint, as the owner asked.
- **Removing a waypoint:** a right click within 12 px of one removes it.
- **Pilot and mission marks:** the pilot's arrow is the pilot's own position and heading, on foot or seated. The
  mission waypoint is the job's own spot (`jobs` waypoint). Both are read from the simulation, never copied
  (CLAUDE.md section 6.1).
- **Tuning** is in `data/hud/nav.ron`, units in the keys:
  - `max_waypoints: 9`;
  - `reach_m: 150.0`;
  - `zoom: (1.0, 8.0)`;
  - `pick_px: 12.0`;
  - the colours.

## 3. Navigating

- **The rule lives in `cb-core::nav`.** `Route { points: Vec<Vec2> }` has `add`, `remove_near`, `clear`,
  `active()` and `step(pilot_xz)`. `step` drops the active point once the pilot is within `reach_m` of it and says
  so, so the app plays the chime. It is unit tested.
- **On screen:** the active point is projected onto the screen at its ground height plus 30 m:
  - a diamond, its number, and its distance (`1.4 KM`, or `850 M` under a kilometre);
  - when it is behind the pilot or off screen, the HUD's edge arrow, the targeting one, reused with a waypoint colour;
  - seated, a tick on the compass tape at its bearing.
- **On foot:** the same marker, without the compass. The HUD shows little else on foot, so the marker is its own
  small overlay. It is the same code with a flag.
- **A mission waypoint** goes through the same marker, in gold, labelled M. It is shown while its job runs.

## Open questions

| Id | Question | Recommendation (provisional until answered) |
|---|---|---|
| WN1 | A route of up to 9 waypoints, or a single one that a new click moves? | A route of 9: plan a trip past the seaport to the airport |
| WN2 | How close counts as reaching a waypoint? | 150 m: about six mech lengths, reached without stopping |
| WN3 | Should the map show enemies, convoys or other units? | No: only places, the pilot and waypoints. A radar shows contacts |
| WN4 | Also a map page on the cockpit MFDs? | Later, in `mfd-menus`, reading the same image and route |
