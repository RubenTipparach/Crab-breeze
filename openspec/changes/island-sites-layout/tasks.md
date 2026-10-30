# Tasks: island-sites-layout

## 1. Write-up and mockup
- [ ] 1.1 Proposal, design, spec; `openspec validate --all`
- [ ] 1.2 Island map mockup (`docs/mockups/island-sites.html`), published and checked in under `docs/artifacts/`

## 2. Placement (`cb_core::world`)
- [ ] 2.1 `sites.airports`, `sites.seaports`, `airport_spacing_m`, `port_coast_apart_deg` in `SitesConfig` and data
- [ ] 2.2 `generate` order: airports, seaports, bases; `plan.sites` holds every site
- [ ] 2.3 `bases.count` 3; tests read the count from the config
- [ ] 2.4 Approach cones: kept clear by sites and capped in cities
- [ ] 2.5 Golden-seed test: seven sites, no overlap, cones clear

## 3. Roads
- [ ] 3.1 `gate` mark in each site template; access road routed to the nearest highway
- [ ] 3.2 Joined into `LaneGraph::build`; `site_trip_share` in `data/world/traffic.ron`

## 4. In the game
- [ ] 4.1 Headless stills of both airports, both ports and a base
- [ ] 4.2 Playtest on the owner's machine
