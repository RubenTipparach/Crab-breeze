# Tasks: lot-access

- [ ] 1.1 Kit: a `drive` record on every house kit (front-edge midpoint and width), drives added to kits without one; exporter writes it to `pieces.json`, the checker fails a house kit without one
- [ ] 1.2 `cb-core`: street pieces along suburb rows, connectors to the next row or the city's outer street, unreachable and too-steep pieces dropped with their houses (tests: every piece joined, grade limits, same twice)
- [ ] 1.3 `cb-core`: the streets levelled by `Ground`'s road rule with the highways
- [ ] 1.4 `cb-core`: drives from each house to its street or the highway, curb cuts, dropped when too steep or crossing a lot (test: every house's drive ends on a road's edge)
- [ ] 1.5 `cb-core`: businesses set back by `parking_depth_m`, their lots, stalls and curb cuts, neighbours' lots joined (test: every business's cut lies on the shoulder)
- [ ] 2.1 `cb-app`: the paving drawn with the roads (asphalt, concrete and gravel, stripes lifted 1 cm)
- [ ] 2.2 Stills: a suburb street, a drive meeting it, a town's car park (`CB_STILLS_HOUSES`)
- [ ] 3.1 Playtest on the owner's machine: walk a suburb street to the city, drive the mech down one; frame time with and without
- [ ] 3.2 Move the requirements into `openspec/specs/lot-access/spec.md` once the playtest passes
