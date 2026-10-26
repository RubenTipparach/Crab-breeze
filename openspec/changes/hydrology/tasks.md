# Tasks

Rivers are switched off in the shipped world (owner, 2026-10-25: "delete all rivers"):
`data/world/world.ron` sets `water.stream_min_flow` to 1e12, so no river, fall or river-fed lake forms.
The code and its tests stay (the world tests run seed 42 at 12.0); 12.0 brings them back.

- [x] Crater range with cliff band and passes in `Shape`
- [x] Drainage grid: priority flood, flow, streams, basins
- [x] Rivers from courses, waterfalls, plunge pools, lakes in `water_at` and the carve
- [x] Biomes from moisture, height and slope; ground colour, grass and trees read them
- [x] Draw river tiles, lakes and waterfalls in `cb-app`
- [x] Cities off the rim and clear of water; highways not over the rim
- [x] `cb-tools map`; seed 42 map in `docs/screenshots/island/game_map_seed42.png`
- [ ] Headless stills of a waterfall and a lake in the running game
- [ ] Playtest on the owner's machine (walk to a fall)
- [ ] Measure frame time with the falls drawn (owner's hardware)
