# Proposal: harbours

## Why

The owner, 2026-10-25: "even ships and mariners". The island has a coast, the sea and ten lakes, and nothing on
them. Then, 2026-10-26: "You also forgot boats, I'll take whatever you recommend for that", and "No survey needed
qs preapprove all boat choices". This change covers every boat on the island, and every design choice in it is the
recommendation, pre-approved by the owner on 2026-10-26 in those words (each one is listed in the design's
`## Pre-approved choices`).

## What changes

- **Harbours** on the coast by the coastal cities: a quay on piles out to berth depth, cranes, warehouses and a
  harbour office whose terminal opens the career's jobs board (the air bases' terminal, reused).
- **Sea traffic**: freighters and tankers on sea lanes between the harbours and out past the horizon, fishing boats
  working grounds off each harbour, and ships at anchor in each harbour's roads. Ships follow their lanes, keep clear
  of the coast, the islets and each other, and slow and turn for harbour. Their crews are counted, not drawn
  (`civilians` draws mariners later).
- **Inland water**: rowing boats and motor launches on the lakes, and a ferry from the harbour nearest the
  archipelago out to an islet. The rivers are switched off in the shipped world (owner, 2026-10-25: "delete all
  rivers"), so there are no river boats and no river ferry; they come back with the rivers.
- **Floating**: hulls ride the real water surface and heave, pitch and roll on the same swell the water draws
  (`water_look.ron`); a moving boat throws spray at its bow. The water has no wake or ripple height field yet
  (`water-ripples` is not built), so a boat leaves no wake in the water itself.
- **The fight**: boats are hit like vehicles (one capsule each, the vehicles' armour rule), sink when destroyed and
  leave a wreck on the bottom and an oil slick on the water; a wading or skimming mech shoves small boats aside and
  swamps them when it hits them hard; boats flee from combat; crab mechs go after ships near them.
- **Standing**: harm by the pilot to a crewed boat is reported as a `BoatHarmed` message naming the nearest base, by
  the same rule as cars (W37), civilians (W42) and civil aircraft (W34), for career standing to read.
- **Shipping jobs** at the harbour office and in the mail: escort a freighter into harbour against raiders, sink the
  raiders' gunboats, clear a blockade. Gunboats are enemy vehicles (`invasion-forces`).
- **Models**: block-outs from a Blender generator at true size (freighter, tanker, fishing boat, rowing boat, launch,
  ferry, gunboat, crane, quay), and an orbitable mockup page with the pilot and the 24 m mech beside them.

## Out of scope

Boarding a ship on foot (later, once mariners and ship decks exist), river boats (no rivers), sailing physics,
detailed hull damage, mariners as people (`civilians`), wakes in the water itself (`water-ripples`).
