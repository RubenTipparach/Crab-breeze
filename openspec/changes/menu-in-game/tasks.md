# Tasks: menu-in-game

- [ ] 1.1 `GameState { Menu, Loading, Playing }`; the command line picks the first state
- [ ] 1.2 The launcher's pages become `MenuPlugin` in the game app; `start_game` and the second process removed
- [ ] 2.1 The world plan computed on entering Loading; every world spawn moved from Startup into `WorldSpawn`
- [ ] 2.2 `DespawnOnExit(GameState::Playing)` on every world entity; run resources removed on leaving; a round-trip test
- [ ] 2.3 Simulation and presentation sets gated on `Playing`
- [ ] 3.1 Quit on the pause menu saves and returns to the main menu
- [ ] 3.2 `SwapMech`: the mech swapped in place through `spawn_mech`; the save records it; tested
- [ ] 4.1 Playtest: menu, free roam, swap the mech, quit to menu, start a career, quit; one window throughout
