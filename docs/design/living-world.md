# The living world: career, bases, cars, people, ships

The owner, 2026-10-25: "build out career mode, hanger bays, cars civilians, air bases and airports and
even ships and mariners. I need a living breathing world where you can walk up and greet people, steal
cars, sign up for jobs at air bases, do missions and get money. Then spend money to build, repair or
customize your mech. Unlock new mechs so you can take on riskier endevaors."

This page is the order the pieces are built in and which OpenSpec change holds each one. Every piece is
first person throughout (the law in `CLAUDE.md`), walked, driven or flown, never cut to a menu.

## The loop the player plays

1. Start free roam or a career from the menu (`launcher`); Continue picks up where the last game was
   saved (`save-game`).
2. Walk the streets: people go about the city, and you can walk up and greet them (`civilians`).
   Cars drive the roads; open a parked one, or pull a driver out of a stopped one, and drive it
   (`car-traffic`, `car-driving`).
3. Drive or fly to an air base. Sign up for jobs at its ready-room terminal or in the mail
   (`air-bases`, `mission-jobs`). Harbours on the coast offer shipping work: escort a freighter,
   sink raiders' boats (`harbours`).
4. Do the job in your mech and get paid (`mission-jobs`, `career`).
5. Back at a base: repair in the repair bay, rearm, buy parts and refit or repaint the mech in a
   hangar bay (`air-bases`, `mech-custom`), or buy a new mech at an airfield (`airfields`,
   `mech-shop`).
6. Owning a stronger mech unlocks riskier, better-paid jobs (`career-progression`).

## Build order

Each step is a playable game on its own, and the next one builds on it.

| Step | Change | What the player gets | State |
| --- | --- | --- | --- |
| 1 | `save-game` | Continue restores where you were: position, mech, damage, ammunition, money, the job under way | built: autosave and Continue work in free roam |
| 2 | `air-bases` | Five bases: landing pads, hangar bays, a repair bay (free repair and rearm), the job terminal | block-out built: placed, drawn, free repair and rearm in any bay |
| 3 | `airfields`, `mech-shop` | Buy mechs with job pay; they are delivered by air to an airfield | written |
| 4 | `career-progression` | Jobs in risk tiers; a tier opens when you own a mech that can take it | written |
| 5 | `mech-custom` | Refit weapons and armour, repaint, in a hangar bay, for money | written |
| 6 | `car-traffic`, `car-driving` | Traffic on the roads; get in a parked car or take one from its driver, and drive | written |
| 7 | `civilians` | People on the pavements, in parks and at the bases; walk up and greet them | written |
| 8 | `harbours` | Harbours, ships at sea and their crews; shipping jobs | written |

Rules that hold throughout: free roam starts with no enemies (owner, 2026-10-25: "its CALLED FREE
ROAM"); enemies come only for a job you accepted, when you reach its site; the menu runs with no world
loaded (`launcher`).
