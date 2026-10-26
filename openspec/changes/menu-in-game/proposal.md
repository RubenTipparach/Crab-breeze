# Proposal: menu-in-game

## Why

Owner, 2026-10-25, after picking another mech on free roam's pause menu: "oop game just crashed on me, or
restarted?", "why did it relaunch exe?", "main menu is usually in the same game". Two things start a second copy
of the program today:

- **The menu is its own small program** (`launcher.rs`). Picking a mode starts the game as a new process with
  `--play` and the menu closes. That was the cheapest way to have a menu with no world loaded behind it (owner,
  2026-10-25: "when NOT INA GAME, there should be no GAME LOADING, NO WORLD").
- **Picking a mech in free roam restarts the program** (`mech-select`, `MenuButton::PickMech`): the game starts
  itself again with `--mech` and closes. The window vanishes and a new one opens, which looks like a crash.

## What changes

- **One program, one window, from the menu to the game and back.** The game opens on the main menu with no world,
  no mech and no loading behind it, as now. Picking a mode loads the world in the same window behind the loading
  screen. Nothing starts a second process.
- **Quit to menu.** The pause menu's Quit returns to the main menu: the game saves, the world is taken down, and the
  menu shows. Quit on the main menu closes the game.
- **Picking a mech in free roam swaps it in place.** The world stays. The old mech is removed, and the picked one
  stands where it stood, kneeling with its hatch open, with the pilot on foot beside it. Only the new mech's model,
  cockpit and boarding load, behind a short loading screen.
- `--play` and `--mech` stay on the command line for scripted runs and shortcuts. They skip the menu, as now.

Not in this change: swapping mechs in a career (the hangar bays, `mech-shop`, `mech-custom`).
