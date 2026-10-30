# Proposal: waypoint-nav

## Why

Owner, 2026-10-30: "we need to add a waypoint nav system in the pause menu. Add a map and let me plop waypoints down to
navigate to". The island is 41 km across and the game has no map. The airport and the seaport sit 11 and 8 km from the
start, and the owner could not find them ("I dont see the new airports or sea ports in game"). A mission's waypoint is
only a distance written on the mail screen ("4.2 KM TO THE WAYPOINT"), with nothing pointing the way.

## What changes

- **A MAP page in the pause menu.** It is a top-down map of the whole island and shows:
  - land shaded by height, the sea, the lakes and rivers, the highways and city streets;
  - the air bases, the airport and the seaport, with their names;
  - the pilot, an arrow pointing the way they face;
  - the accepted mission's waypoint.

  The scroll wheel zooms (1x to 8x) round the pointer, and dragging pans.
- **Plop waypoints.** A left click on the map puts down a waypoint. Waypoints are numbered in the order they are put
  down, and form a route drawn as a line from the pilot through each in turn. A right click on one takes it away.
  CLEAR takes them all.
- **Navigate to them.** The first waypoint not yet reached is the active one. It shows in the world:
  - as a diamond with its number and distance on the screen;
  - as a tick on the cockpit compass;
  - as an arrow at the screen's edge while it is behind the pilot.

  This works seated and on foot. Coming within the reach radius of it (150 m) takes it off and makes the next one
  active, with a chime.
- **A mission's waypoint is shown the same way**, in gold, ahead of the pilot's own: one marker for both. That is
  one implementation.

Not in this change:
- a map on the cockpit's MFDs (a later page, `mfd-menus`);
- enemies or units on the map;
- saving waypoints between sessions (they last until the game quits).
