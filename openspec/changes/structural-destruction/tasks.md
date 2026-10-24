# Tasks: structural-destruction

## 0. Mockup
- [x] 0.1 The graph, connectivity, load pass, creak, pancake and the element mask in `docs/mockups/buildings.html`
- [x] 0.2 Demolition shots in `docs/screenshots/buildings/`
- [ ] 0.3 Owner tries it (click to fire) and answers W12 and W15

## 1. Core (`cb-core::world::structure`, replacing `destruct.rs`)
- [ ] 1.1 Elements and bonds from `building-generation` (test: `a_new_building_stands_with_every_member_under_half_its_capacity`)
- [ ] 1.2 Blasts (test: `a_120mm_shell_holes_a_wall_and_leaves_the_columns`)
- [ ] 1.3 Connectivity and islands (test: `cutting_a_corner_column_line_drops_nothing_by_itself`)
- [ ] 1.4 Load pass, cantilever rule, creak (test: `taking_a_whole_ground_side_peels_the_bays_above_it`)
- [ ] 1.5 Falling islands and impact (test: `a_falling_storey_punches_through_the_floor_below`)
- [ ] 1.6 Golden demolition: seed 42, the `side` script, the same broken set at the same steps
- [ ] 1.7 Move `building-destruction`'s tests across; archive it as superseded

## 2. App
- [ ] 2.1 `BuildingMask` extended material, prepass and shadow included
- [ ] 2.2 Piece pool from `FRAC_` cells; Avian for the first 600
- [ ] 2.3 Damage-stage decals and holed variants
- [ ] 2.4 Measured in `perf-bench` during the `side` demolition
- [ ] 2.5 Playtest: bring the tower's podium down from the cockpit; on Drive
