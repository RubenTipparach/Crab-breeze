# Proposal: civil-air-traffic

## Why

The owner asked on 2026-10-26: "Do we have regular air traffic?" We do not. The only aircraft in the
game are the invasion's (`invasion-forces`, `combined-arms`: helicopters, fighters, the crabs' cargo
planes and the crab air fortress). The sky over the island is empty except when it is attacking you.

The owner answered three survey questions the same day:

- **W32:** people, cars and aircraft are all built at once, in parallel (this change is the aircraft;
  `civilians` and `car-traffic` are the other two).
- **W33:** airliners and cargo planes fly between the five air bases; light planes and news or police
  helicopters fly over the cities. They keep clear of a fight, and crabs can shoot them down.
- **W34:** your own fire can hit a civilian aircraft, and it costs you standing with the nearest base.

Related: the owner wants a bought mech air dropped near the player by a cargo aircraft. That is being
built in the career loop (PR #72), which also holds career standing per base. Neither is on `main`, so
this change exposes a hook (a message and a counter) instead of depending on them, and shapes its cargo
plane's flight so the career loop can fly one over a drop point later.

## What changes

- **Airliners and cargo planes fly between the air bases.** You see them roll down a base's apron and
  lift off, climb out, cruise across the island at 900-1200 m over the ground, turn onto a straight-in
  final, descend on a 3 degree glide path, touch down on the far base's apron and brake to a stop.
  - The bases have no runway (`air-bases` draws an apron, hangars, three pads and a tower). Aircraft use
    a strip along the apron's long side instead. It is only about 600 m long, so the take-off roll and
    the braking are shortened to fit (W45).
- **Light planes and helicopters circle the cities** at 180-300 m: news and police helicopters,
  sightseeing light planes.
- **Only near you.** Aircraft are spawned where you cannot see them appear: beyond 10 km in any
  direction, or beyond 3 km and behind you. They are removed beyond 14 km. The island is never
  simulated whole: at most 9 civil aircraft exist at once (W46).
- **They keep clear of a fight.** A route that would pass within 2.5 km of an enemy bends round it. A
  flight whose destination base is in a fight diverts to the nearest base that is not. A helicopter or
  light plane circling a city that comes under attack leaves.
- **They can be shot down.**
  - They are hit through the same shot sweep, capsules and armour rule as the invasion's vehicles.
  - Crab vehicles take shots of opportunity at a civil aircraft that comes into their guns' range and
    cone while the pilot is not their shot.
  - A downed aircraft falls with its momentum and crashes. The crash is an explosion through the blast
    rules (crater, trees, buildings, splash on mechs) and leaves a wreck, laid down as debris through
    the debris rules (bodies near you, a dust puff far away).
- **Your own hits count against you.** Every hit the pilot lands on a civil aircraft is sent as a
  message naming the aircraft, the base nearest where it was hit, and whether it was downed. It is also
  added to a per-base tally the career loop can read when it lands (W34).
- **On the screens they are neutral.** A civil aircraft in range is boxed in white with `CIV` and its
  range, never red. The pick nearest the crosshair and the cycle key skip them, so a missile never locks
  one by itself. They can never be selected or locked: only unguided fire can hit one
  (owner, 2026-10-26, survey W47: "Never: only unguided fire can hit one"). There is no radar scope in the cockpit yet; the boxes and
  the TARGET readout are what exists.
- **Stand-in models at true size.** Labelled block-outs until each is modelled in Blender: a 1980s
  airliner (40 m long, 35 m span), a heavy freighter (70 m long, W48), a light plane (8.3 m) and a
  helicopter (12 m with its rotor). The block-out shapes are one table in `cb-core`, drawn by the game
  and by the mockup page `docs/mockups/civil_aircraft.html`, approved by the owner on 2026-10-26
  ("Approved all of the stand ins"); the models are built after it.
- **Engine sound for the jets only:** a stand-in loop (the mech's jet rumble, pitched down) on airliners
  and freighters. There is no propeller or rotor sound in the library yet (W49).

## Out of scope

- Real models, textures and LODs (next, now the stand-ins are approved).
- A runway at each base (W45 asks).
- The career loop's standing, prices and penalties: this change only reports the hits.
- The mech air drop itself: the career loop flies it with this change's flight plan.
- Air traffic control, radio chatter, airport crowds, taxiing, parked aircraft.
- Blast splash on aircraft (only a direct hit counts for now).
