# Tasks: island-sites-layout

## 1. Write-up and mockup
- [x] 1.1 Proposal, design, spec; `openspec validate --all`
- [x] 1.2 Island map mockup (`docs/mockups/island-sites.html`), published and checked in under `docs/artifacts/`

## 2. Placement (`cb_core::world`)
- [x] 2.1 `sites.airports`, `sites.seaports`, `airport_spacing_m`, `port_coast_apart_deg` in `SitesConfig` and data
- [x] 2.2 `generate` order: airports, seaports, bases; `plan.sites` holds every site
- [x] 2.3 `bases.count` 4; tests read the count from the config
- [x] 2.4 Approach cones: kept clear by sites and capped in cities
- [x] 2.5 Golden-seed test: seven sites, no overlap, cones clear

## 3. Roads
- [x] 3.1 `gate` mark in each site template; access road routed to the nearest highway
- [x] 3.2 Joined into `LaneGraph::build` at a gate node
- [ ] 3.3 `site_trip_share`: cars choose sites as destinations

## 4. In the game
- [ ] 4.1 Headless stills of both airports, both ports and a base
- [ ] 4.2 Playtest on the owner's machine
