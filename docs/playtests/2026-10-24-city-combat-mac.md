# Showcase take: combat in the city, 2026-10-24

`crab-breeze --showcase defence` (seed 42) with raiders spawning inside the city
(`spawn_inside_fraction: 0.5`), recorded with OBS between PLAYTEST_START and PLAYTEST_END, 5 min 9 s.
The frame-time numbers are in `docs/benchmarks/2026-10-24-demolition-mac/`, from separate runs with
no OBS.

- Build: `building-research` with main and `pilot-controls` (PR #18) merged in.
- Machine: Apple M4 Max, macOS; release build; OBS ScreenCaptureKit window capture, 60 fps.
- Result: `PLAYTEST_RESULT lose defence: your mech was destroyed, city 1 5 %, city 2 0 %, city 3 0 %`
- Drive: `dungeon-crawler-2026/crab-breeze-2026-10-24-city-combat.mp4`
- Contact sheet: `2026-10-24-city-combat-mac_frames.png`

| Beat | What the contact sheet shows |
| --- | --- |
| Board, stand | seated in the cockpit in city 1's street, `WAVE 1/3 IN 19 s` |
| Fight | the wave in the streets, smoke on the avenue, enemy mechs among the towers, enemies 16 down to 11 |
| Stuck | about 1.5 min with the nose in a broken building's open floors (enemies 15 to 14) |
| End | the pilot's mech destroyed at 5:09, city 1 at 5 % damage |

## What looked wrong

- The scripted pilot pins itself against buildings: the bot's wall avoidance is a pending desktop PR.
- Raiders inside the city work: they spawn in the streets and fight there.
