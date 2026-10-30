# Island sites

Published at https://claude.ai/artifact/DvqkTx4UJqzLXRwenkzUUf (openspec changes `island-sites-layout` and
`offshore-limit`).

The island as the game generates it (seed 42) with the sites the owner asked for on 2026-10-30: two civil airports
placed first, two seaports on different coasts, three military bases where there were five, every runway's approach
cones, a provisional ship lane between the ports, and the offshore limit (15 km off the coast or 1 km inside the world's
edge, whichever comes first) with the warning band before it. A table gives, along eight headings, where the coast is,
where the limit falls and which rule sets it. Today's sites can be shown beside the proposal.

- `index.html` is the published page as built by `python3 tools/mockups/gen_island_sites.py` from
  `tools/mockups/island_sites_src.html`.
- The data it shows: the map is `cb-tools map 42` (inlined as a JPEG), today's sites and bases are `cb-tools sites 42`,
  and the proposed second airport and seaport are the generator script's `PROPOSED` table, placed by eye on the map.
- Shots: `docs/screenshots/island-sites/`, by `tools/mockups/shoot_island_sites.mjs`.
