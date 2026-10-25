# Proposal: mech-shop

## Why

The owner, 2026-10-25: "make sure to design the mech shop too, you can access via one of the mfds in mouse look
mode", "any mfds can be interactable in mouse look mode, it just brings up the game menu, map, emails, and shop".
Earlier (`air-bases`, `career`): "Air bases allow the player to fix their mechs, buy new mechs and get into different
mechs", "completing missions let you buy more mechs", and survey J14: "all mechs are unlocked, you just dont have
money to buy them".

`air-bases` put the catalogue on a terminal in each base's ready room. The owner wants it in the cockpit, on any MFD.

## What changes

- **SHOP is an MFD page** (`mfd-menus`): pick it from any screen's menu, from the seat, anywhere on the island.
- **What it sells:** the eight roster mechs (Foreman, Ironhorse, Harrow, Roustabout, Driller, Mule, Picket, Sapper),
  every one for sale from the start; money is the only limit (J14).
- **The list:** each mech's name, price and an OWNED mark, the career's money at the top. Pointing at a row and
  clicking opens the mech's sheet.
- **The sheet:** a turning wireframe of the mech drawn from its own glb at true relative scale, its height, mass,
  top speed, armour, cockpit, its signature weapon (`signature-weapons`) and its other guns, then **BUY** with the
  price and the money after it.
- **Buying** charges the career through its ledger (`career`: the one place money moves) and delivers the mech to a
  free hangar bay: the current base's if the pilot is at one, otherwise the nearest base with a free bay, where it is
  waiting on arrival. A purchase that the money or the bays cannot cover is refused before anything is charged, and
  the sheet says why (`SHORT 120 000` or `NO FREE BAY`).
- **The same page on the base terminal:** the ready room's catalogue terminal (`air-bases`) shows this SHOP page, so
  there is one shop, not two.
- **Prices are data** (`data/career/catalogue.ron`), inside J4's range (300 000 to 1 200 000 credits).

## Out of scope

Selling mechs, buying weapons or parts separately (a mech comes with its loadout), repairs (`air-bases`), and the
hangar's bays and doors (`air-bases`).
