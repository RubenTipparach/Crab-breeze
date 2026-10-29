# Tasks: seaport-layout

- [x] 0.1 3D mockup for approval: `docs/mockups/seaport.html` (`python3 tools/mockups/gen_seaport.py`), today against
      proposed, every building breakable; the airport's shapes and breakage shared through `tools/mockups/site_kit.js`
- [x] 0.1b Everything breaks, the boats too: built on the small-buildings kit (`docs/mockups/site_buildings_kit.js`), the
      ships and luffing cranes cut from their models, one engine for the street, the airport and the port
      (`docs/mockups/building_engine.js`); a holed ship sinks
- [ ] 0.2 Owner approves the mockup (no game code before it); SP1 to SP3 in the survey
- [ ] 1.1 `cb_core::ships::port::layout(harbour, cfg) -> Port`: breakwaters, basin, quays, berths, yards, buildings, from
      the harbour's shore and out direction; tests (inside, no overlaps, berths deep enough, the entrance clear)
- [ ] 1.2 Berths for the harbour traffic: freighters, tankers, the ferry, trawlers and gunboats moor at their own
      quays; tests
- [ ] 2.1 Drawn: breakwaters, quay walls, paving, rail, paint, lights and buoys
- [ ] 2.2 Buildings and cranes as breakable kit sets (the small buildings' damage path), colliders from the pieces
- [ ] 2.3 The mech slipway: a walkable ramp into skimming water
- [ ] 3.1 Stills: `CB_STILLS_FX=harbour` before and after
- [ ] 3.2 Playtest on the owner's machine: skim in through the entrance, wade up the slipway
