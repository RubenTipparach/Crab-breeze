# Showcase take: a city battle in the Foreman, 2026-10-25

`crab-breeze --mech mechs/foreman.ron --showcase defence` (seed 42), raiders spawning in the city's
streets, recorded with OBS between PLAYTEST_START and PLAYTEST_END, 10 min 39 s. Frame-time numbers
come from separate runs with no OBS (`docs/benchmarks/`).

- Build: `building-research` at `c468b31` (PR #5 with the re-baked cab of PR #26 merged in).
- Machine: Apple M4 Max, macOS; release build; OBS ScreenCaptureKit window capture, 60 fps.
- Result: `PLAYTEST_RESULT lose defence: wave 1 was not stopped in time, city 1 7 %, city 2 0 %, city 3 0 %`
- Drive: `dungeon-crawler-2026/crab-breeze-2026-10-25-foreman-battle.mp4`, https://drive.google.com/file/d/182JxyPrayyRTn5Y5jTV_Xids3KbZjfw4/view
- Contact sheet: `2026-10-25-foreman-battle-mac_frames.png`; stills: `2026-10-25-foreman-battle-mac_stills.jpg`

| Beat | What the take shows |
| --- | --- |
| Board | on foot to the Foreman crouched in the street, its chest open; the climb into the seat |
| Cockpit | the panorama cockpit: front screen, overhead MFDs, damage diagram, radar, weapons list, gauges, both sticks |
| Fight | the streets of city 1, a target locked, explosions on facades, enemies 16 down to 9; 61-84 fps on the readout |
| Stuck | from about 4 min, six minutes facing a shot-open office building; the wave clock runs out |

## What looked wrong

- The scripted pilot pins itself on a building (wall avoidance is a pending desktop PR).
- The damage diagram MFD is labelled `CHASSIS BREEZE` in the Foreman.
