# Tasks: structural-destruction

## 0. Mockup
- [x] 0.1 The graph, connectivity, load pass, creak, pancake and the element mask in `docs/mockups/buildings.html`
- [x] 0.2 Demolition shots in `docs/screenshots/buildings/`
- [ ] 0.3 Owner tries it (click to fire) and answers W12 and W15

## 1. Core (`cb-core::world::structure`, replacing `destruct.rs`)
- [x] 1.1 Elements and bonds: columns, floor bays and facade panels on the building's cell grid, built on first hit (test: `a_new_frame_stands_with_every_member_at_half_its_capacity`); facade modules and interiors join when `building-generation` and `building-interiors` land
- [x] 1.2 Blasts (test: `a_120mm_shell_holes_a_wall_and_leaves_the_columns`)
- [x] 1.3 Connectivity and islands (tests: `losing_one_ground_column_brings_nothing_else_down`, `losing_every_ground_column_brings_the_building_down`)
- [x] 1.4 Load pass, cantilever rule, creak (tests: `losing_a_whole_ground_side_peels_off_the_bays_above_it`, `the_creak_comes_before_the_fall`, `the_same_hits_break_the_same_elements`)
- [x] 1.5 Falling parts and impact: a fallen part lands after its free-fall time on the highest standing floor under it at 2 x its weight, and keeps falling through bays that give (tests: `a_falling_top_punches_down_through_the_floors_below`, `the_same_collapse_lands_the_same_heaps`)
- [ ] 1.6 Golden demolition: seed 42, the `side` script, the same broken set at the same steps
- [ ] 1.7 Move `building-destruction`'s tests across; archive it as superseded

## 2. App
- [x] 2.0 A damaged building is drawn from its frame (facade panels in its facade material, floors and columns in bare concrete) in bands of 4 storeys; only a band whose standing count changed is rebuilt (tests: `a_band_draws_every_standing_element_of_its_storeys`, `a_broken_element_leaves_its_band`)
- [ ] 2.1 `BuildingMask` extended material, prepass and shadow included (replaces the band rebuild; needs a custom vertex attribute and vertex, prepass and shadow shaders in Bevy 0.20, so it waits until it can be looked at in the running game)
- [ ] 2.2 Piece pool from `FRAC_` cells; Avian for the first 600
- [ ] 2.3 Damage-stage decals and holed variants
- [ ] 2.4 Measured in `perf-bench` during the `side` demolition
- [ ] 2.5 Playtest: bring the tower's podium down from the cockpit; on Drive
