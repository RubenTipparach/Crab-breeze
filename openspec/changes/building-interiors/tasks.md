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
- [ ] 3.1 Interiors streamed inside 150 m and on breach, on a worker thread. Drawing is done: a dressed building meshes its rooms on the async pool and shows them within `rooms_radius_m` (250 m, the owner's number); the collision streams inside `collision_radius_m` (task 4.6). Left: building the plans of a breached building off the main thread
- [ ] 3.2 Interior-mapping glass material for far windows
- [ ] 3.3 Pilot collision with floors, partitions, stairs and furniture (on foot, intact and breached: tasks 4.5 and 4.6)
- [ ] 3.4 Playtest: walk into a walk-up, climb its stair, look out of a window; on Drive

## 4. Entrances and stairs (owner, 2026-10-25: "each building needs doors and staairs that i as a player can walk into them")
- [ ] 4.1 The street side of every building, worked out at generation (`WorldPlan::street_sides`; test: every door is reachable from the pavement)
- [ ] 4.2 `Entrance` per building: the door on the street facade, a lobby lined up with it or the corridor, the route to the stair (tests: every building has one, on its street side; nothing on the route)
- [ ] 4.3 The dog-leg stair in the core, ground floor to top storey, and the stairwell in each floor (test: every shipped building's stair has a walkable rise and run)
- [ ] 4.4 The dressing draws the entrance as an opening with its leaf standing open, the stoop, and the floors with their stairwells; the rooms mesh draws the stair
- [ ] 4.5 `Walkable`: one box rule for standing and walls, each box hung on its frame element so a breached building is walkable too; `Scene` uses it for the pilot (test: a pilot walks in off the pavement, through the door and up the stair to the top storey)
- [ ] 4.6 On foot only (owner, 2026-10-26), the app builds `Walkable` for buildings within `collision_radius_m` of the pilot (within `collision_sync_m` at once, the rest on the async pool) and frees them all when the pilot leaves their feet, or one past the margin (tests: the set is chosen by mode and distance; climbing out and boarding switch cleanly)
- [ ] 4.7 Survey W30 (how the door opens) and W31 (warehouse roller doors) answered
- [ ] 4.8 Playtest: walk in off the street, climb to an upper floor, look out of a window (this is 3.4)
