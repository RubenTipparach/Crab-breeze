# Tasks: seaport-layout

- [x] 0.1 3D mockup for approval: `docs/mockups/seaport.html` (`python3 tools/mockups/gen_seaport.py`), today against
      proposed, every building breakable; the airport's shapes and breakage shared through `tools/mockups/site_kit.js`
- [x] 0.1b Everything breaks, the boats too: built on the small-buildings kit (`docs/mockups/site_buildings_kit.js`), the
      ships and luffing cranes cut from their models, one engine for the street, the airport and the port
      (`docs/mockups/building_engine.js`); a holed ship sinks
- [x] 0.2 Owner approves the mockup (2026-10-29: "Dope, implement all that"); SP1 to SP3 provisional as written
- [x] 1.1 The port's layout: the approved page's own, exported with its pads and marks (`site.json`, design section 3b)
      instead of a second layout in Rust; placed by `cb_core::world::sites::place_seaport`; tests (loads, stands as built,
      placed on seed 42, pads flat, no house on it)
- [ ] 1.2 Berths for the harbour traffic: freighters, tankers, the ferry, trawlers and gunboats moor at their own
      quays; tests
- [x] 2.1 Drawn: breakwaters, quay walls, paving, rail, paint, lights and buoys (`ground.glb` and the lights' kits)
- [x] 2.2 Buildings and cranes as breakable kit sets (the small buildings' damage path), colliders from the pieces
      (design section 3b)
- [x] 2.2b The port placed on the largest coastal city's coast that fits (`place_seaport`), its land, dock and basin
      shaped to its pads; its harbour (`Harbour::of_port`) gives the traffic its berth, mouth and roads and the jobs their
      terminal; its quays walked on as fixed platforms and a wall from the water
- [x] 2.4 Ships as kits that sink in place when holed at the waterline, still broken piece by piece, settling into the
      seabed and gone (`small_buildings::damage::Sinking`)
- [x] 2.5 The traffic's boats under way the same (owner, 2026-10-29: "make sure this damage system works on moving
      ships too"): a kind with a ship kit (freighter, tanker, ferry, fishing boat, launch, tug, gunboat; not the rowing
      boat) is drawn from it, a round breaks its pieces where it is now, holed at its waterline it founders and goes
      down through the kit, gone under the seabed (`Breakable`, `Boat::break_sphere`, `Boat::step_kit`)
- [ ] 2.3 The mech slipway: a walkable ramp into skimming water
- [ ] 3.1 Stills: `CB_STILLS_FX=harbour` before and after
- [ ] 3.2 Playtest on the owner's machine: skim in through the entrance, wade up the slipway
