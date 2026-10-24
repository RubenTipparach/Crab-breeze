# Tasks: building-interiors

## 0. Mockup
- [x] 0.1 Floorplans, partitions, cores, stairs and furniture in `docs/mockups/buildings.html`, with the floorplan panel
- [ ] 0.2 Owner looks at the floorplans and answers W11

## 1. Core (`cb-core::world::interior`)
- [x] 1.1 Floorplan per storey on the plan's boxes, `cb-core::world::interior` (tests: `every_room_has_a_door`, `walk_ups_hold_flats_and_towers_hold_offices`, `the_same_storey_is_planned_the_same`); the core sits in the middle cell of every storey
- [x] 1.2 Walls written once per line where bands meet; flats under 4.6 m stay one room, wider ones open off the kitchen
- [x] 1.3 Furniture placement (tests: `no_item_blocks_a_door`, `walls_and_items_rest_on_the_floor_below`); tall items never go on a facade wall
- [ ] 1.4 Golden seed for the block's plans

## 2. Art
- [ ] 2.1 Furniture kit in Blender (`hs_kit`), PBR, LODs, manifest budgets
- [ ] 2.2 Interior-mapping room atlases from staged kit rooms; contact sheet looked at

## 3. App
- [x] 3.0 Interiors drawn in a damaged building's band meshes (no extra entities), each wall and item while the floor bay under it stands; 12 storeys plan in 0.4 ms (2 400 boxes, release, indicative)
- [ ] 3.1 Interiors streamed inside 150 m and on breach, on a worker thread (today they appear only once a building is breached, which is when they can be seen: buildings are opaque)
- [ ] 3.2 Interior-mapping glass material for far windows
- [ ] 3.3 Pilot collision with floors, partitions, stairs and furniture
- [ ] 3.4 Playtest: walk into a walk-up, climb its stair, look out of a window; on Drive
