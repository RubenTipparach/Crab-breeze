# Playtest 2026-11-01: the Stratostacean's legs shot at the nozzle; the crater sets its wrecks down

- **Commit:** `8547b34b` (branch `fortress-nuke`, PR #113)
- **Machine:** the owner's Mac (Apple M4 Max), release build, window at 1920 x 1080 (`CB_NATIVE_1080=1`), sound on (`CB_TAKE_SOUND=1`)
- **Script:** `data/playtests/fortress-nuke.ron`, in the Foreman: `./target/release/crab-breeze --playtest fortress-nuke --mech mechs/foreman.ron`
- **Video:** `crab-breeze-2026-11-01-stratostacean-nozzles-crater-wrecks.mp4` in the Drive folder `dungeon-crawler-2026`
  (copied by Drive for Desktop; the Drive connector was not available to confirm it)
- **Result:** `PLAYTEST_RESULT fail`: every beat passed up to `ClearHostiles`, which left two fighters flying

## What changed (design section 13)

The owner saw the bot shooting the empty exhaust under the legs, and the legs taking the damage. The lift thrusters
are now hit as a flat-ended cylinder of the nacelle's own size (15.5 m round, 40 m long), not a capsule whose rounded
ends reached 16.5 m into the exhaust. The nozzle is the nacelle's lower 8 m, and the bot aims at its middle. The nuke
now sets every burnt car on the dug ground down onto the bowl, tilted to it, and burns the trees there away.

## Beats

| Beat | Outcome |
|---|---|
| Board, autocannon, start the mission | passed |
| `BringDownFortress(1500)` | passed: L1-L4 and R1-R4 shot at their nozzles, then HL1-HL3 through their ports, all with the autocannon, down in about 5.5 min |
| `LookAtNearest("nuke")` | passed |
| `JetToNearest("crater")` | passed: the burnt cars lie on the crater floor and up its sides, no tree over the bowl |
| `ClearHostiles(600)` | failed: the ground hostiles went down, and two fighters (5001, 5002) circled 170-2000 m off for the rest of the 600 s; the bot's missiles were spent on the ground targets and its autocannon did not land hits on a crossing fighter |

## Frames

- `docs/screenshots/fortress-down/2026-11-01-nozzles/hull-port.jpg`: under the hull, a port locked
- `docs/screenshots/fortress-down/2026-11-01-nozzles/crater-wrecks-from-air.jpg`: the burning cars on the bowl, seen on the climb out
- `docs/screenshots/fortress-down/2026-11-01-nozzles/crater-floor-wrecks.jpg`: on the crater floor, the wrecks sitting on it
- `docs/screenshots/fortress-down/2026-11-01-nozzles/contact-sheet.jpg`: six frames across the take

## Open

- The fighters at the end: the bot needs either to keep missiles for aircraft, or to lead a crossing fighter with
  the autocannon well enough to hit it. Not changed in this round.
