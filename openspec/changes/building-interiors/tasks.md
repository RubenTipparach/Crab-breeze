# Tasks: building-interiors

## 0. Mockup
- [x] 0.1 Floorplans, partitions, cores, stairs and furniture in `docs/mockups/buildings.html`, with the floorplan panel
- [ ] 0.2 Owner looks at the floorplans and answers W11

## 1. Core (`cb-core::world::interior`)
- [ ] 1.1 Floorplan per storey (tests: `every_room_has_a_door`, `the_core_lines_up_on_every_storey`)
- [ ] 1.2 Walls written once per line (test: `no_two_partitions_share_a_plane`)
- [ ] 1.3 Furniture placement (tests: `no_item_blocks_a_door`, `tall_items_never_stand_at_a_window`)
- [ ] 1.4 Golden seed for the block's plans

## 2. Art
- [ ] 2.1 Furniture kit in Blender (`hs_kit`), PBR, LODs, manifest budgets
- [ ] 2.2 Interior-mapping room atlases from staged kit rooms; contact sheet looked at

## 3. App
- [ ] 3.1 Interiors streamed inside 150 m and on breach, on a worker thread
- [ ] 3.2 Interior-mapping glass material for far windows
- [ ] 3.3 Pilot collision with floors, partitions, stairs and furniture
- [ ] 3.4 Playtest: walk into a walk-up, climb its stair, look out of a window; on Drive
