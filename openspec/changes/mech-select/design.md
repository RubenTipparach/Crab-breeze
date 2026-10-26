# Design: mech-select

## 1. The roster

`GameData::roster(root) -> Vec<RosterMech>`: every `data/mechs/*.ron` whose `interior` is set, in file-name order,
each with its file (`mechs/<id>.ron`) and display `name`. A mech without an interior is the stand-in or retired.
Tested: it holds the eight roster mechs and the Foreman, and every entry loads (`GameData::load`).

## 2. Starting in a mech

One function starts the game, `launcher::start_game(mode, mech)`: this same program, with the menu's own arguments
passed on, `--play <mode>`, and `--mech <file>` in place of any `--mech` already there. The launcher calls it, and so
does the pause menu's Mech page, which then closes the running game (`AppExit`). One way to start a game, used from
both places (CLAUDE.md 6.1).

| Where | Shows | Picking a mech |
| --- | --- | --- |
| Launcher, Free roam | the roster, then the save check if a free-roam save exists | `start_game("free", mech)` |
| Pause menu in free roam, Mech | the roster, the current mech marked | `start_game("free", mech)`, then the game exits |

## 3. Cost

A restart: the loading screen, as at launch (the world, the mech, its cockpit and boarding). No cost in play.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| MS1 | Swap the mech without a reload (keep where the pilot stands) | Later, its own change: the mech's data, cockpit and boarding become per-mech resources | provisional |
