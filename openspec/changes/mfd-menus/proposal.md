# Proposal: mfd-menus

## Why

The owner, 2026-10-25: "make sure to design the mech shop too, you can access via one of the mfds in mouse look
mode", then "actually any mfds can be interactable in mouse look mode, it just brings up the game menu, map, emails,
and shop", and "just make them have some outline that shows the player can interact with them when mouse over".

Today, holding free look (Left Ctrl) and clicking while looking at an MFD steps it to its next readout page
(`cockpit-mfds`). Nothing shows which screen the click will land on, and the screens only cycle readouts. The mail
(`mission-jobs`), the map and the shop (`mech-shop`) need a place in the cockpit, and the game menu (`game-menus`)
should be reachable from the seat without leaving first person (the law).

## What changes

- **An outline on the screen you point at.** In free look, the MFD under the crosshair (within reach, 3 m) gets a lit
  amber outline round its bezel, so the player sees it can be pressed. It goes when the crosshair leaves it or free
  look is let go. Every one of the six screens, in every cockpit (`cockpit-parity`).
- **Clicking any MFD opens its menu on that screen.** The menu lists the readout pages (MECH, FLIGHT, MISSION,
  WEAPONS, TARGET) and four more: **MAP**, **MAIL**, **SHOP** and **MENU**. The row under the crosshair is lit; a
  click picks it; the screen then shows that page until it is opened again. BACK, or Escape, closes the menu or
  app and gives the screen back the page it showed before it was opened (owner, 2026-10-25: "clicking back should
  return to mech view and restore the mfd to its original screen").
- **MAP** is a new page: the island from above with the cities, roads, air bases, the mech's position and heading,
  the job's site and known enemies, zoomed by clicking + and - rows.
- **MAIL** is `mission-jobs`' page; **SHOP** is `mech-shop`'s; **MENU** opens the game's pause menu
  (`game-menus`: resume, options, controls, quit), the same menu Escape opens. One page each, one implementation:
  the base terminals and the Escape menu show the same ones.
- **Pointing is the crosshair.** In free look the mouse turns the head (`seated-look`), so the view's centre is the
  pointer: the screen hit and the spot on it come from the same ray. Nothing takes the view from the player.

## Out of scope

The shop's catalogue and rules (`mech-shop`), the mail's jobs (`mission-jobs`), the menus' own pages
(`game-menus`). Touch-screen sounds are named for `audio`.
