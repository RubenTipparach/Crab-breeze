# The Stratostacean in the game: cloud stills, 2026-10-25

The fortress was renamed the Stratostacean on 2026-10-26 (survey H2). On the day of these stills it was called the
Hermit, the name the stills' mail page and the run's log carry; the quotes below use the new name.

**This is not a playtest.** These are headless stills from a cloud session: Mesa's software Vulkan (lavapipe) under
Xvfb, 4 CPU cores and no GPU. Frames took 1 to 3.5 s each. **No frame times are claimed here** (CLAUDE.md sections 7
and 9). No OBS, no Drive upload. The pilot never sat in the mech: the stills instrument places the pilot and breaks
thrusters through the game's own hit path. The playtest belongs on the owner's machine: `./run.sh --mission fortress`,
or `./run.sh`, Free roam, M for the mail, Start on the Stratostacean's message.

- Branch `claude/youthful-tesla-zrsjb8` (main merged in), seed 42, `--mech mechs/foreman.ron`.
- Command: `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json CB_STILLS_FORTRESS=1 xvfb-run -a -s "-screen 0
  1920x1080x24" ./target/release/crab-breeze --mech mechs/foreman.ron --mission fortress --stills output/stills-fortress`
- Stills (1280 px): `docs/screenshots/fortress-game/`.
- Changes: `openspec/changes/crab-airship` section 9 and `openspec/changes/free-roam-missions`.

## What the stills show

| Still | Shows |
| --- | --- |
| `mail.jpg` | Free roam's mail (M): the Stratostacean's message RUNNING with Stop, the city defence's with Start |
| `street.jpg` | The Stratostacean from an open field 1.1 km off: the model exported from the mockup, textured with its designed armour, the eight lift plumes lit. Beyond 700 m its spikes are not drawn (its LOD1) |
| `under.jpg` | From under it, 600 m down: the belly, the drop bays, the beam turrets, eight nozzles glowing |
| `deck.jpg` | The pilot standing on its deck, looking down the runway to the bridge: the spikes drawn up close (LOD0), the deck guns and turrets in red |
| `legs-falling.jpg` | L1, L2 and L3 broken from under them: their legs fall away tumbling, the hull doors open |
| `falling.jpg` | HL1 and HL2 broken: it lists to port, burning, and sinks |
| `crashed.jpg`, `crashed-far.jpg` | After the crash (the wreck itself is hidden behind the city from both places the instrument stands) |

The log of the run (the last of four; 83 to 91 buildings fell in each): `MISSION fortress begins: the Stratostacean over city 1 ... 600 m up`; three `Lift thruster
destroyed`, `the hull doors open`, `the hull thrusters light`; two `Hull thruster destroyed`, `it is falling`; three
`a leg hits the ground and explodes`; status lines `lift port 83 % stbd 100 % ... list 8.3 deg, FALLING` from 568 m down
to 125 m; `MISSION fortress won: the Stratostacean is down, 83 buildings under it brought down`.

## What was found and fixed on the way

- **The defences destroyed the player's empty mech** in the first half minute, before the pilot could reach it.
  They now shoot the pilot's mech only while the pilot sits in it. (Its fighters are the `fighter` unit, whose brain
  still runs at the mech whether or not anyone sits in it, as in the defence: to look at.)
- **The pilot on foot could not stand on anything but the ground**: the on-foot step set the feet to the terrain's
  height, so placed on the deck the pilot dropped 600 m to the street. It now stands on what `support_at` gives
  (ground, roof or platform), as the mech does.
- **Standing still on the deck, the pilot slid 26 m across it**: the step pushed the pilot out of the footprints of
  the buildings and trees 600 m below. Walls now push at the feet's own height and trees only near the ground
  (test `a_pilot_standing_still_on_a_deck_over_the_city_stays_where_they_stand`). After the fix the pilot ended
  exactly where they stood: `(10.0, 49.00, -90.0)` in the fortress's frame after 4.2 s of flight.
- **The mockup's cylinders never got their nacelle wrap**: the panelling code read the type of a copied geometry,
  so every cylinder was panelled as a box. Fixed in the page; the exported model and the published mockup carry the
  designed nacelle armour.
- **Switching missions stopped the game**: starting the defence from the mail turned the Stratostacean off and began the
  defence, then the fortress's drawing system ran once more in the same frame without its mission and Bevy stopped.
  Each of its systems now checks for the mission itself, and the switch runs before drawing. Checked with
  `CB_STILLS_FORTRESS=switch`: `the Stratostacean turned off, the defence running`, clean exit.
- **The spikes now have a level of detail** (owner, 2026-10-25: "spikes should not render until we are up close"):
  `DETAIL_` meshes drawn within 700 m. LOD0 34 372 triangles, LOD1 30 124.

## Noticed, not this work's

About 30 `Entity despawned` warnings in the first seconds of every run, with or without the fortress (the same run
without `--mission fortress` logs the same 31): something on main despawns entities twice. Harmless, worth a look.

## Not yet

The crabs dropped from the bays and the deck guard (task 2.8, survey H6); a mech sliding off a listing deck; the
spine's and bridge's walls do not push yet; wave 3 of the defence bringing the fortress; the Blender model. How hard
the defences hit is untested with a pilot aboard: their numbers are the design's first estimates.
