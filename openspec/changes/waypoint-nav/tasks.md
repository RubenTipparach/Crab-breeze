# Tasks: waypoint-nav

## 1. Mockup
- [x] 1.1 The MAP page and the in-world marker as a three.js page on the real seed 42 island (`cb-tools mapdata`),
      published and checked into `docs/artifacts/waypoint-nav/`

## 2. Core
- [x] 2.1 `cb_core::world::map::paint`: the island to an RGBA image, tested headless
- [x] 2.2 `cb_core::nav::Route`: add, remove near, clear, active, step with reach; tested
- [x] 2.3 `data/hud/nav.ron`, validated on load

## 3. Game
- [x] 3.1 The MAP page in the pause menu: the image, zoom and pan, labels, pilot and mission marks
- [x] 3.2 Waypoints put down, removed and cleared on the page; the list beside the map
- [x] 3.3 The active waypoint's marker, distance and edge arrow, seated and on foot (the job marker's `pin` and
      `edge_arrow`); the chime on reaching it (`nav_reached`). No compass tick yet: the HUD has no compass tape, only
      its heading line
- [x] 3.4 The mission waypoint through the same marker

## 4. Verify
- [ ] 4.1 Headless stills: the MAP page with a route, and the marker in the world
- [ ] 4.2 Playtest on the owner's machine: plot a route to the seaport and the airport and walk and fly it
