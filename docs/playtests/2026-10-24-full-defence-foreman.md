# Playtest: the full defence in the Foreman, 2026-10-24

- **Machine:** RTX 3070, i7-9700F, Windows 10; the game's window at 1920 x 1080; quick release build (thin LTO off) of branch `cockpit-mfds`.
- **Take:** `crab-breeze --mech mechs/foreman.ron --showcase defence`, recorded with OBS from `PLAYTEST_START` to `PLAYTEST_END`. First person throughout, uncut, 9 min 51 s.
- **Drive:** [crab-breeze-2026-10-24-full-defence-foreman.mp4](https://drive.google.com/file/d/1U0T4iC63YOIzdSKQFYq4lQtXjG6YQ0mC/view) (H.264, CRF 26, 196 MB), confirmed with the Drive connector.
- **Result:** won. All 41 enemies destroyed: 30 raiders and 11 vehicles. Final city damage: city 1 0.2 %, city 2 1.0 %, city 3 1.9 %.

## What it shows

- The three top-bar MFDs live: MECH (leg compass, damaged parts), FLIGHT (boost, skates, mode, speed, altitude, guns) and MISSION (wave, city, damage, enemies, clock).
- The cabin lit by its own lamps.
- The wider view (85 degrees).
- Bigger tracers.
- Tanks dropped by parachute, helicopters and fighters.

## Takes 1 to 4 (lost), and what changed before the win

| Take | Result | Cause | Change |
| --- | --- | --- | --- |
| 1 | pilot down at about 90 s | the bot fired too rarely | the bot's fire rule loosened to twice the target's size; 2 raiders, not 3, at the pilot at once |
| 2 | pilot down in wave 1 | of 101 hits counted, helicopter rockets (120 mm) beat the leg armour (120 mm) and took a leg | helicopter rockets 100 mm |
| 3 | pilot down at 265 s, 7 of 16 left | 4 tanks a wave wore it down from 450 m | the forces lightened (2 tanks and 1 helicopter in wave 1); tank shells 6 |
| 4 | wave 1 out of time | the bot chased a helicopter circling at 300 m, always "nearest", for seven minutes | the bot weighs aircraft as four times as far: the ground fight first, missiles still at a lock |
| 5 | won | | |

## Rough

- The contact sheet tool dropped its second row of frames (it came out black). The video is whole.
