# Playtest: the career loop, 2026-10-25

`crab-breeze --playtest career-loop` (run 0, a new career) then `--playtest career-loop --run 1` (Continue), seed 42,
the scripted pilot playing `data/playtests/career-loop.ron` through the player's own inputs: input actions, the Use
and Escape keys, and mouse pointer moves, presses and releases on the menu's buttons. First person throughout: the
walks, the rope climb into the cockpit and the climb out are all in the take, nothing cut. Recorded with OBS
(ScreenCaptureKit window capture of the game window, fitted to OBS's 1280 x 720 canvas) from `PLAYTEST_START` to
`PLAYTEST_END` of each run; the two takes joined, 5 min 10 s.

- Build: `career-loop` at `deff877` (the four career steps, the merge of `origin/main` at `24ef8a8`, and the playtest
  pipeline).
- Machine: Apple M4 Max (16 cores, 40-core GPU, 64 GB), macOS, the built-in Retina display as main; release build,
  windowed (the 1920 x 1080 window clamped by macOS to 1728 x 1079). No frame-time numbers: this was a recorded take.
- Results: `PLAYTEST_RESULT ok career-loop: every beat played` (run 0) and
  `PLAYTEST_RESULT ok career-loop run 1: every beat played` (run 1).
- Drive: `My Drive/dungeon-crawler-2026/crab-breeze-2026-10-25-career-loop.mp4` (H.264, CRF 26, preset slow, 45 MB),
  written to the Drive for Desktop folder. Not confirmed with the Google Drive connector: its session had expired.
- Contact sheet: `2026-10-25-career-loop_frames.png` (20 s, 36 s, 70 s, 135 s, 250 s, 294 s).

## Beats

| # | Beat | Outcome | Take time |
| --- | --- | --- | --- |
| 1 | A new career (`--play career-new`) | Started on air base 1 with 1 000 credits and the Foreman on the apron | 0 s |
| 2 | Walk to the job terminal, sign up | Walked about 175 m to bay 1's terminal, pressed Use; the TERMINAL screen's JOBS tab listed a tier 1 Intercept near City 9 (2 mechs, 37 000 credits); Accept clicked: job 0 under way | 6-39 s |
| 3 | Board, do the job, get paid | Walked back, Use at the rope, climbed into the seat; skated to the site (about 4 km), the job's 2 enemies came in, both destroyed; `JOB 0 won: 37000 credits`, standing with base 2 now 1 | 39-142 s |
| 4 | Walk the mech into a hangar bay | Drove back and stopped in bay 1: `REPAIRED 2 PARTS +2, REARMED +3 RDS, COST 0 CR`, shown on the MECH MFD | 142-258 s |
| 5 | Buy a mech, see tier 3, refit | Climbed out, walked to bay 1's terminal. A disclosed playtest grant of 650 000 credits (in the ledger as `playtest grant`; one job pays 37 000 and the cheapest tier 3 mech is 600 000). SHOP: bought the Sapper, delivered to bay 2, `Tier 3 contracts open` mailed and a tier 3 job offered at once. JOBS: the tier 3 offer is listed. REFIT: bought a 40 mm cannon (60 000) and fitted it to station 1, the right arm: the flown Foreman now carries `40 mm cannon, 120 mm cannon, shoulder missile` | 258-300 s |
| 6 | Quit, then Continue | Escape, Quit clicked: saved 28 000 credits, 2 mechs, on foot at (-10552, -5451). Run 1 (`career-continue`): `restored 28000 credits, 2 mechs, at (-10552, -5451), on foot`, the fit restored (`40 mm cannon` on station 1) | 300-310 s |

## What the contact sheet shows

It is the game, in first person, never black: the walk across the apron toward the hangar bays; the terminal's
JOBS screen with the tier 1 offer and Accept; the Foreman seen from the apron on the walk back to board; the
cockpit with all its MFDs lit during the fight (the job's marker, `35 MM AUTOCANNON` under the crosshair); the
cockpit rolling into bay 1; the REFIT screen with the three stations, the weapons for sale, the armour grade and
the liveries.

## What looked wrong

- The terminal's screen is the menu overlay in the middle of the view, not drawn on the terminal's own screen mesh
  (air-bases task 2.4 says so).
- The first time a terminal opens, its title reads `TERMINAL` without the base and bay; they appear on the next
  redraw (the screen is drawn the frame before the terminal's resource lands).
- The REFIT page is taller than the panel: the livery row is clipped at the bottom, and the `[E] Use the terminal`
  prompt shows through the panel.
- The bay's repair has no gantries, sparks or steam yet; only the MFD says it happened.
- The bought Sapper is in the career and its bay is taken, but it is not drawn in the bay and cannot be boarded yet
  (air-bases 2.5).
- The 40 mm cannon was fitted but not fired in this take (mech-custom 3.1 still open); the weapon models on the mech
  do not change.
- The run needed a grant to afford a tier 3 mech in one take; real earning would take about a dozen tier 2 jobs.
