# Tasks: airport-layout

- [x] 0.1 3D mockup for approval: `docs/mockups/airport.html` (`tools/mockups/gen_airport.py`), today against proposed, all four runway sides
- [x] 0.1b Second pass, a whole airport after the references (owner, 2026-10-29: "That airfield looks kinda simple")
- [x] 0.1c Z-fighting gone (every flat layer its own height) and every building breakable in the mockup with the
      small-buildings kit's support rule (owner, 2026-10-29: "more zfighting, also use our breakable framework")
- [x] 0.1d Buildings up to the houses' standard: drawn on the small-buildings kit (bays, framed glazing, doors, floors,
      roofs in panels, interiors) and broken by the street's engine (`docs/mockups/building_engine.js`)
- [ ] 0.2 Owner approves the mockup (no game code before it); AP1 to AP3 in the survey
- [ ] 1.1 The strip: flattened, checked when the runway is placed; `runway_gap_m` 100 m; tests
- [ ] 1.2 `cb_core::world::airfield::layout`: taxiway, links, holds, entries, stands, buildings; tests (inside, no overlaps, clear)
- [ ] 2.1 Runway markings, blast pad chevrons, approach lights, PAPI
- [ ] 2.2 Taxiways, holding positions, stand lines
- [ ] 2.3 Buildings as block-outs: terminal and bridges, glazed tower cab, fire station, fuel farm, car park, helipad, windsock, fence
- [ ] 2.3b Every building a breakable kit set (pieces, colliders, support links) through the small buildings' damage path
- [ ] 2.4 Parked aircraft from the civil stand-ins
- [ ] 3.1 Stills: `CB_STILLS_FX=base` before and after
- [ ] 3.2 Playtest on the owner's machine
