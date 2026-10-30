# Waypoint Nav (mockup)

The pause menu's MAP page and the marker it puts in the pilot's view, for the openspec change `waypoint-nav` (owner,
2026-10-30: "we need to add a waypoint nav system in the pause menu. Add a map and let me plop waypoints down to
navigate to").

- Link: https://claude.ai/artifact/QsHAw7s5JknR9QHNFLAzjf
- What it shows: the real seed 42 island (`map_data.json`, from `cargo run -p cb-tools -- mapdata 42 256`). On it:
  - waypoints are put down and taken away, and the map zooms and pans;
  - the route lists each waypoint's distance;
  - a three.js first-person view shows the active waypoint's diamond, distance, compass tick and edge arrow, and the
    route can be walked through.
- What made it: `page.src.html`, built into `index.html` by `python3 tools/mockups/gen_waypoint_nav.py`.
