# Design: menu-in-game

## 1. App states

`cb-app` gets one Bevy state, `GameState { Menu, Loading, Playing }`, initialised to `Menu` unless the command line
names a mode (`--play`, `--mission`, `--showcase`, `--bench`, a playtest script), which starts in `Loading`.

- **Menu:** the launcher's pages (Main, Continue, Confirm, the mech roster) become `MenuPlugin` inside the game app,
  with a 2D camera and nothing else. They are built `OnEnter(Menu)` and despawned `OnExit(Menu)`
  (`DespawnOnExit(GameState::Menu)` on the root).
- **Loading:** the world plan is generated, then every world spawn runs, then the existing loading screen waits on
  assets and pipelines as it does now and prints `GAME_READY`, then moves to `Playing`.
- **Playing:** the game as it runs today.

## 2. Moving the world off Startup

21 plugins spawn their part of the world in `Startup` today (`world.rs` roads, buildings, sun, water and trees;
`ground`, `rocks`, `grass`, `mech`, `pilot`, `hud`, `mfd`, `air_bases`, `fx`, `battle_fx`, `ground_marks`, `rope`,
`physics`, `clouds_render`, `water_render`, `sky_light`). Each moves to `OnEnter(GameState::Loading)` in one named
set, `WorldSpawn`. The world plan, computed in `WorldPlugin::build` today, moves to a system at the start of that
set (`cb_core::world::plan`, the same function), so the menu opens without paying for world generation.

Every entity spawned there carries `DespawnOnExit(GameState::Playing)`, so leaving the game takes it all down. A test
counts entities after a Menu, Loading, Playing, Menu round trip: the count is back where it started.

`Update` and `FixedUpdate` work stays where it is, gated as whole sets with `.run_if(in_state(GameState::Playing))`
on the top-level sets (`SimSet` and the presentation sets), not system by system.

Resources the run builds (`Plan`, `Saving`, `Encounters`, the career, the free-roam state) are inserted on entering
`Loading` and removed on leaving `Playing`. Asset handles stay loaded, so a second start is faster than the first.

## 3. Swapping the mech in place

Free roam's Mech page sends `SwapMech(file)`. Its system:

1. reads the picked mech's data (`GameData` already holds every roster mech's spec);
2. remembers the current mech's position and heading, and puts the pilot on foot 25 m behind it if seated;
3. despawns the old mech's entity tree (model, cockpit, boarding, weapons, effects attached to it);
4. spawns the new one through `spawn_mech`, the same function the world spawn uses, at that position, kneeling with
   its hatch open, and shows the loading screen until its model, cockpit and boarding are ready.

The save records the mech file (`FreeRoamSave.mech`), so Continue brings back the swapped mech.

## 4. Cost

- Menu to first frame: one window, the menu only. Measured against today's launcher, the target is the same.
- Loading time: unchanged on a first start. A second start reuses loaded assets and compiled pipelines.
- A mech swap: one mech's glb, cockpit and boarding, about 1-3 s behind the loading screen on the RTX 3070, measured
  in the playtest.
- Per frame: nothing. State gates run once per set.

## 5. As built (2026-10-26)

- **States** (`crates/cb-app/src/state.rs`). A plain run opens in `Menu`; any mode on the command line opens in
  `Loading`, which moves to `Playing` on the first frame. A run cannot open in `Playing` itself: Bevy 0.20 runs the
  first state's `OnEnter` before `PreStartup`, so the world would spawn before the settings and the building state
  are set up. Starting in `Loading` keeps the spawns after every startup schedule, as when they were `Startup`.
- **One helper for the whole sweep** (`GameApp`): `add_world_spawn` (was `Startup`), `add_after_world_spawn` (was
  `PostStartup`, ordered after the spawns) and `add_game_systems` (`First`, `FixedUpdate`, `Update` and `Last`, in
  the `InGame` set, which runs only in `Playing`). Settings and the frame cap stay ungated, so the menu is capped and
  its window settings apply.
- **The menu** (`main_menu.rs`) is the launcher's pages with their own `Camera2d`, both carrying
  `DespawnOnExit(GameState::Menu)`. A pick goes through `begin_run`, the one implementation a `--play` run uses too:
  it inserts `menu::Play`, `career::CareerStart` and `save_game::Saving`, and reloads the game data when the mech
  differs. Continue in free roam flies the mech the save was made in.
- **The mech swap is in place** (`mech::SwapMech`, `swap_mech`), not a teardown and restart: the old mech's tree is
  despawned and the new one spawned through `spawn_mech_entity` where it stood, facing its way. It stands as a fresh
  start stands it, ready to board, rather than kneeling; a pilot who was not on foot is put on foot beside it, at
  the start plan's offset turned to the mech's heading.
- **Not yet:** the world plan is still computed in `WorldPlugin::build` (nothing draws from it before `Playing`), and
  nothing is taken down on leaving `Playing`, so Quit in the game saves and closes the program, as before but never
  relaunching. Quit to the main menu waits on task 2.2.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| MI1 | Where does the swapped mech stand | where the old one stood, kneeling, the pilot on foot beside it | provisional |
| MI2 | Does Quit in the game go to the main menu | yes; Quit on the main menu closes the game | provisional |
| MI3 | Does the separate launcher program stay for anything | no, it is removed | provisional |
