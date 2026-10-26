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
- **Buying** charges the career through its ledger (`career`: the one place money moves) and **air drops the mech
  near the pilot** (owner, 2026-10-26, below): the shop fixes a landing site on good ground 40 to 150 m from where the
  pilot stands, a cargo aircraft flies over and releases it, it comes down under canopies with a last retro burst,
  and it stands there to be walked up to and boarded. Only when no good ground lies within 600 m does it go to a free
  hangar bay instead (the current base's, else the nearest base's with one). A purchase the money cannot cover, or
  one with neither a site nor a free bay, is refused before anything is charged, and the page says why
  (`SHORT 120 000` or `NO FREE BAY`).
- **The same page on the base terminal:** the ready room's catalogue terminal (`air-bases`) shows this SHOP page, so
  there is one shop, not two.
- **Prices are data** (`data/career/catalogue.ron`), inside J4's range (300 000 to 1 200 000 credits).

## The air drop (owner, 2026-10-26)

The owner, 2026-10-26: "when a player orders a mech it should be air dropped close to wherever the player is on a good
are on the ground (not stuck on a building)".

This **supersedes** two earlier answers:

- **Survey J16** ("delivered to the nearest base with a free bay", accepted on its recommendation): a bought mech is
  no longer parked in a bay. It is dropped near the pilot wherever they are, at a base or in the field. Bay delivery
  is kept only as the fallback when no good ground is near.
- **`airfields` section 3** (a cargo aircraft lands at an airfield and rolls the mech out): the aircraft now flies
  over the pilot and drops the mech; it never lands. `airfields` keeps its runways and roster questions.

What the player sees: BUY on the terminal (or the MFD shop) says `AIR DROP INBOUND`, a marker shows where it will
land and how far, an aircraft passes overhead, the mech falls under three canopies, fires its retros just above the
ground, lands on its feet in a ring of dust and footprints, and stands there. Walking up to it, it kneels and lowers
its rope like any owned mech, and the pilot climbs in. If the pilot walks into the landing site while it falls, the
marker warns them off and the canopies steer it aside. It is the career's mech from the moment it is bought, and a
save keeps it where it landed.

## Out of scope

Selling mechs, buying weapons or parts separately (a mech comes with its loadout), repairs (`air-bases`), and the
hangar's bays and doors (`air-bases`).
