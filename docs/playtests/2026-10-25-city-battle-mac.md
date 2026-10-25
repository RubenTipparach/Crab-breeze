# Showcase take: a city battle, 2026-10-25

`crab-breeze --showcase defence` (seed 42), raiders spawning in the city's streets, recorded with OBS
between PLAYTEST_START and PLAYTEST_END, 3 min 20 s. Frame-time numbers come from separate runs with
no OBS (`docs/benchmarks/`).

- Build: `building-research` at `6235f74` (PR #5 with `main` and the GPU work merged in).
- Machine: Apple M4 Max, macOS; release build; OBS ScreenCaptureKit window capture, 60 fps.
- Result: `PLAYTEST_RESULT lose defence: your mech was destroyed, city 1 2 %, city 2 0 %, city 3 0 %`
- Drive: `dungeon-crawler-2026/crab-breeze-2026-10-25-city-battle.mp4`, https://drive.google.com/file/d/1F3xl-GFXIuvtKjVS_k7RDP_t-0mCZYn3/view
- Contact sheet: `2026-10-25-city-battle-mac_frames.png`

| Beat | What the contact sheet shows |
| --- | --- |
| Stand | seated in the cockpit in city 1, `WAVE 1/3 IN 19 s`, all parts OK |
| Fight | in the streets among the towers, a rocket boost, enemies 16 down to 11, smoke and fire |
| Worn down | arm L 70 %, torso 55 %, then 18 % |
| End | the pilot's mech destroyed at 3:20, city 1 at 2 % damage |

## What looked wrong

- The scripted pilot still loses: it spends long stretches facing a building or the sky. Its wall
  avoidance is a pending desktop PR.
