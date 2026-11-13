# Tasks: city-districts

- [x] 1.1 The mockup: today's generator beside the future city, on one camera at true scale
- [x] 1.2 After the owner's notes and the Fargo reference: no river, visible streets, no slab or ring; a highway with the commerce, a downtown beside it, house blocks on drifting grids, farmland
- [x] 1.3 Variations: small and large downtowns, three shapes, a bypass highway, inland or coast; housing scales with a large downtown
- [x] 1.3a One connected street network: every block a cell of one bent grid, frontage roads and interchanges, section roads into the farmland, no empty land between parts of the city
- [x] 1.3b Exits after the owner's chart: SPUI, diamond, single quadrant, partial cloverleaf, cloverleaf and trumpet, and four highway modes (at grade, overpasses on embankments, elevated, bypass)
- [x] 1.3c Drivable: every road surface recorded, a fill, clearances, grades, turns, ramp ends and a driving bot (`driveCheck`), passing in all 14 variations
- [x] 1.4 The owner's pick of the look (CD1): approved
- [x] 1.5 Design the generator change (design section 4: three steps)

## 2. In the game, step 1: the Fargo city on today's grid

- [x] 2.1 `BlockUse` and zoning (downtown, flats, strip, houses, park, school), tuned in `world.ron`
- [x] 2.2 `build_city` by use; the lot placer fills house and strip blocks with drives and car parks onto the city's streets
- [x] 2.3 Drawing: house and strip blocks as a pavement ring with the yards and car parks inside; car parks on `Surface::Lot`; the highway on Concrete047A
- [x] 2.4 Tests: zoning, every city lot off the streets and joined to one, goldens re-pinned
- [x] 2.5 Headless stills of the new cities (`docs/screenshots/city-districts/game/`)

## 3. Step 2: exits

- [ ] 3.1 Decks (`RoadDeck`) and height queries by level: support under the feet, pushed out from under a slab, `road_top_at` leaving out decked highway
- [ ] 3.2 The trunk: its row (`BlockUse::Under`), the two highways re-laid onto it, arterial blocks long enough for a ramp
- [ ] 3.3 Exits: four one-way ramps per arterial, landing at the frontage street's junction
- [ ] 3.4 The lane graph along a highway: nodes at ramps, one-way ramp lanes, shared highway ends, lanes with their own heights
- [ ] 3.5 The drive check (fill, clearance, grade, ramp ends, bot) as `cb-core` tests
- [ ] 3.6 Drawn: decks, piers, ramps with three levels of detail, blended joins (concrete fading into asphalt, ramps welded into streets)
- [ ] 3.7 Headless stills of the viaduct and an exit
- [ ] 3.8 The other modes (overpasses on embankments, the bypass) after the owner has seen the viaduct

## 4. Step 3: drifting grids

- [ ] 4.1 Turned blocks and footprints through collision, facades and interiors

## 5. Left over

- [ ] 5.1 Measure the cost of a large city on the owner's machine
- [ ] 5.2 The game's road colliders pass the same drive check (fill, clearance, grade, turns, ramp ends, bot) in `cb-core` tests
