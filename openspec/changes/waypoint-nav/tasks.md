# Tasks: waypoint-nav

## 1. Mockup
- [ ] 1.1 The MAP page and the in-world marker as a three.js page on the real seed 42 island (`cb-tools mapdata`),
      published and checked into `docs/artifacts/waypoint-nav/`

## 2. Core
- [ ] 2.1 `cb_core::world::map::paint`: the island to an RGBA image, tested headless
- [ ] 2.2 `cb_core::nav::Route`: add, remove near, clear, active, step with reach; tested
- [ ] 2.3 `data/hud/nav.ron`, validated on load

## 3. Game
- [ ] 3.1 The MAP page in the pause menu: the image, zoom and pan, labels, pilot and mission marks
- [ ] 3.2 Waypoints put down, removed and cleared on the page; the list beside the map
- [ ] 3.3 The active waypoint's marker, distance, edge arrow and compass tick, seated and on foot; the chime on reaching it
- [ ] 3.4 The mission waypoint through the same marker

## 4. Verify
- [ ] 4.1 Headless stills: the MAP page with a route, and the marker in the world
- [ ] 4.2 Playtest on the owner's machine: plot a route to the seaport and the airport and walk and fly it
