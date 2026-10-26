# Tasks: menu-in-game

- [x] 1.1 `GameState { Menu, Loading, Playing }`; the command line picks the first state (`state.rs`)
- [x] 1.2 The launcher's pages become `MainMenuPlugin` in the game app; `start_game` and the second process removed
- [ ] 2.1 The world plan computed on entering Loading; every world spawn moved from Startup into `WorldSpawn`
  (the spawns are moved; the plan is still computed in `WorldPlugin::build`, see design section 2)
- [ ] 2.2 `DespawnOnExit(GameState::Playing)` on every world entity; run resources removed on leaving; a round-trip test
- [x] 2.3 Simulation and presentation sets gated on `Playing` (`InGame`)
- [ ] 3.1 Quit on the pause menu saves and returns to the main menu (it saves and closes the game for now: 2.2 first)
- [x] 3.2 `SwapMech`: the mech swapped in place through `spawn_mech_entity`; the save records it
  (`Saving.mech`, and Continue reads the save's mech); not yet seen in a playtest
- [ ] 4.1 Playtest: menu, free roam, swap the mech, quit to menu, start a career, quit; one window throughout
