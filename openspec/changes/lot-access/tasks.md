# Tasks: lot-access

- [x] 1.1 Kit: a `drive` record on every house kit (front-edge midpoint and width), drives added to kits without one; exporter writes it to `pieces.json`, the checker fails a house kit without one
- [x] 1.2 `cb-core`: street pieces along suburb rows, connectors to the next row or the city's outer street, unreachable and too-steep pieces dropped with their houses (tests: `every_suburb_street_joins_a_city_or_a_highway`, `the_same_seed_places_the_same_lots`)
- [x] 1.3 Dropped: the streets are draped on the ground as the pads leave it (design section 2), no levelling rule
- [x] 1.4 `cb-core`: drives from each house to its street or the highway, curb cuts, dropped when too steep or crossing a lot (test: `every_house_has_a_drive_to_a_street_or_a_highway`)
- [x] 1.5 `cb-core`: businesses set back by `parking_depth_m`, their lots, stalls and curb cuts, neighbours' lots joined (test: `every_business_has_a_car_park_with_a_curb_cut_onto_its_highway`)
- [x] 2.1 `cb-app`: the paving drawn with the roads (asphalt, concrete and gravel, stripes lifted 1 cm)
- [x] 2.3 Laid by each ground node on the ground as that node draws it (`cb-app::paving`, `Heightfield::height_at_step`), so far streets no longer hover over coarse ground (owner, 2026-10-28: "suburb roads are hovering"; tests: `height_at_step_lies_on_a_coarse_chunk_s_triangles`, `a_piece_crossing_two_nodes_is_laid_once_between_them`, `paving_rides_on_the_ground_it_is_given`)
- [ ] 2.2 Stills: a suburb street, a drive meeting it, a town's car park (`CB_STILLS_HOUSES`)
- [ ] 3.1 Playtest on the owner's machine: walk a suburb street to the city, drive the mech down one; frame time with and without
- [ ] 3.2 Move the requirements into `openspec/specs/lot-access/spec.md` once the playtest passes
