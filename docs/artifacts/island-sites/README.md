# Island sites

Published at https://claude.ai/artifact/DvqkTx4UJqzLXRwenkzUUf (openspec changes `island-sites-layout` and
`offshore-limit`).

The island as the game generates it (seed 42) with the sites the owner asked for on 2026-10-30: two civil airports
placed first, two seaports on different coasts, four military bases where there were five (base 2 added back by the owner), every runway's approach
cones, a provisional ship lane between the ports, and the offshore limit (15 km off the coast or 1 km inside the world's
edge, whichever comes first) with the warning band before it. A table gives, along eight headings, where the coast is,
where the limit falls and which rule sets it. Today's sites can be shown beside the proposal.

- `index.html` is the published page as built by `python3 tools/mockups/gen_island_sites.py` from
  `tools/mockups/island_sites_src.html`.
- The data it shows: the map is `cb-tools map 42` (inlined as a JPEG), and every site and base is where the game now
  places them (`cb-tools sites 42`, since island-sites-layout was built). The Today view is the generator's `TODAY`
  table: what the game built before the change.
- Shots: `docs/screenshots/island-sites/`, by `tools/mockups/shoot_island_sites.mjs`.
