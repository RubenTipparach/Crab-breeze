# Proposal: mech-select

## Why

Owner, 2026-10-26: "In free play i can select any mech from the menu and it will swap out models (req)". Every
roster mech now has its own cockpit (`cockpit-identity`), but free roam always starts in the mech the command line
names (`--mech`, default the Foreman), and nothing in the game changes it.

## What changes

- **The roster in one place:** `cb_core::GameData::roster` lists every mech in `data/mechs/` that has a cockpit
  interior (the eight roster mechs; not the stand-in, not the retired Breeze), with its name, in a stable order.
  The menu, the pause menu and a test read it; nothing else keeps a list of mechs.
- **On the menu (the launcher):** Free roam opens a mech page listing the roster; picking one starts free roam in
  that mech (`--mech`), after the "this deletes your save" check when there is a save.
- **In the game, free roam's pause menu has a Mech page** listing the roster, the current mech marked. Picking
  another restarts free roam in it: the game starts itself again with that mech, as the launcher starts it, and
  closes, so the loading screen shows while the new mech, its cockpit and its boarding load.

Not in this change: swapping the mech without a reload (the game loads one mech's data, cockpit and boarding at
startup; a live swap is its own change), mechs in a career (the shop and hangars, `mech-shop`, `air-bases`).
