# Tasks: airport-layout

- [x] 0.1 3D mockup for approval: `docs/mockups/airport.html` (`tools/mockups/gen_airport.py`), today against proposed, all four runway sides
- [ ] 0.2 Owner approves the mockup (no game code before it)
- [ ] 1.1 The strip: flattened, checked when the runway is placed; `runway_gap_m` 100 m; tests
- [ ] 1.2 `cb_core::world::airfield::layout`: taxiway, links, holds, entries, stands, buildings; tests (inside, no overlaps, clear)
- [ ] 2.1 Runway markings, blast pad chevrons, approach lights, PAPI
- [ ] 2.2 Taxiways, holding positions, stand lines
- [ ] 2.3 Buildings as block-outs: terminal and bridges, glazed tower cab, fire station, fuel farm, car park, helipad, windsock, fence
- [ ] 2.4 Parked aircraft from the civil stand-ins
- [ ] 3.1 Stills: `CB_STILLS_FX=base` before and after
- [ ] 3.2 Playtest on the owner's machine
