# Tasks: building-type-lods

- [x] 1.1 `dressing::far_variant`, `dress_variant` and `massing`, from the dressing's own look and pieces (test: `the_far_look_and_the_dressing_pick_the_same_kind_and_colour`, `massing_keeps_only_what_stands_on_the_roof`)
- [x] 1.2 The far tiles rendered face-on from the dressing's pieces (`cb_core::world::far_facade`) and turned into colour, normal and ORM layers at startup from the near skin's maps (`cb-app/src/far_facades.rs`) (tests: `a_tile_is_the_same_every_time`, `every_far_layer_is_built`, `a_tower_is_mostly_glass_and_a_walk_up_mostly_brick`)
- [x] 1.3 A contact sheet of every variant's tiles (`CB_FAR_SHEET`, `docs/screenshots/building-lods/far-facades-sheet.jpg`), looked at
- [x] 2.1 The far walls on the opaque building material pointed at the far arrays; the massing on the near arrays; both follow the state buffer (`buildings::bind_far_looks`)
- [x] 2.2 The massing per building within `massing_radius_m`, shown and hidden with its far walls (`buildings::show_massing`)
- [x] 2.3 The far walls on every building past the dressing, laid on its cell grid; radii in `data/world/world.ron`
- [x] 2.4 `every_building_type_has_its_far_look`; the far boxes' generic facade texture and tone materials retired
- [x] 3.1 Cloud stills from the overlook, old and new (`docs/playtests/2026-10-26-building-lods-cloud.md`); cockpit stills at set distances still to take on the owner's machine
- [ ] 3.2 Measured on the owner's machine: the combat bench, old against new interleaved
- [ ] 3.3 Playtest: skate toward a city; no type pops colour or window rhythm at 250 m or 1.5 km
