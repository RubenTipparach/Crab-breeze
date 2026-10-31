# Playtest 2026-10-31: fortress-nuke (the Stratostacean, its nuclear crash the finale)

- **Script:** `data/playtests/fortress-nuke.ron`, in the Foreman: `CB_TAKE_SOUND=1 ./target/release/crab-breeze --playtest fortress-nuke --mech mechs/foreman.ron`
- **Commit:** `b27ec5de` on `fortress-nuke` (PR #113).
- **Machine:** the owner's MacBook Pro (Apple M4 Max), 1920 x 1080 (`CB_NATIVE_1080=1`).
- **Recording:** OBS window capture with desktop sound, from `PLAYTEST_START` to `PLAYTEST_END`. The sound measured -19.8 dB mean and -0.5 dB peak.
- **Drive:** `dungeon-crawler-2026/crab-breeze-2026-10-31-stratostacean-nuke.mp4` (6 min 50 s, H.264 `-crf 26 -preset slow`). It is cut at the ejection, which comes 52 s into the watch.
- **Contact sheet:** `docs/screenshots/fortress-down/2026-10-31-nuke-finale/sheet.png`.

## Beats

| Beat | Outcome |
|---|---|
| Wait, Board, Wait, SelectWeapon("autocannon"), StartMission("Fortress"), Wait | played |
| LookAtNearest("fortress", 4) | looked at the Stratostacean's deck |
| BringDownFortress(900) | the Stratostacean came down in 327 s. Its crash went nuclear: a crater 800 m across and 100 m deep, and 403 buildings and houses flattened |
| LookAtNearest("nuke", 60) | the pilot held the mushroom cloud in view from 1536 m. 52 s in, a fighter shot out the Foreman's torso and the pilot ejected |

`PLAYTEST_RESULT ok fortress-nuke run 0: every beat played`.

## Earlier attempts the same morning

| Take | Script | What happened |
|---|---|---|
| 1 | fortress-down | The carrier came down. The pilot turned at once to clear the hostiles, so the cloud was never watched, and the Foreman was lost about 70 s after the nuke. The take was stopped there. |
| 2 | fortress-nuke | The Foreman spent about four minutes 250-290 m off the weaving carrier, never closing to its 120 m missile range. It was lost under the carrier before the carrier fell. |
| 3 | fortress-nuke | This take. |

## Found

- **Silent takes:** every take before these recorded silence, because the owner's `settings.ron` holds both volumes at 0. `CB_TAKE_SOUND=1` now plays a take at full volume, in memory only.
- **The mech-lost alert:** `PLAYTEST_ALERT` only fired while the pilot was seated. The seat throws the pilot out on the frame the mech is destroyed, so take 3's loss went unreported. It is now checked whatever the pilot's mode.
- **The cloud from the cockpit:** the cockpit's overhead screens hide the top of the rising cloud. The view holds a point at 45 % of the cap's height.
- **The bot:** it can hang 250-290 m off a weaving carrier for minutes without closing to missile range, and the carrier's escorts can kill it while it watches.

## The take under section 12's rules (every leg, then half the hull)

- **Commit:** `1918638d` on `fortress-nuke`.
- **Drive:** `dungeon-crawler-2026/crab-breeze-2026-10-31-stratostacean-legs-and-hull.mp4` (13 min 50 s, cut just
  before the mech was lost).
- **Sound:** -19.5 dB mean, 0 dB peak.
- **Contact sheet:** `docs/screenshots/fortress-down/2026-10-31-legs-and-hull/sheet.png`.

| Beat | Outcome |
|---|---|
| BringDownFortress(1500) | All eight legs, then three hull ports, on tapped jets the whole fight (420-470 m up at the legs, about 205 m up under the ports); down in 355 s |
| LookAtNearest("nuke", 25) | The fireball and the cloud, from the cockpit |
| ClearHostiles(480) | Seven of the ten taken out: all six crab mechs, then a fighter. A fighter destroyed the Foreman at 829 s with three fighters left. `PLAYTEST_ALERT` ended the take |

The unrecorded run before it (try 8) won every beat: the carrier down in 360 s, then every hostile taken out in
183 s.

Tries 1-7 found:

| Problem | Fix |
|---|---|
| Held jets, low flight | Tapped jets near the carrier |
| The hover settling 60 m behind the moving carrier | Its goal led by 4 s |
| Missiles launched 180 m under a port arriving too shallow | 400 m under, fired only within 15 deg of vertical |
| Hull ports at 900 hp needing 15 of 40 missiles | 600 hp, open question H8, provisional |
| The straggler hunt re-picking its target every frame | One target at a time, the ground first |
