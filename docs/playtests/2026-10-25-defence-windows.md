# Showcase take: the three wave city defence on Windows, 2026-10-25

`crab-breeze --showcase defence` (seed 42, the Foreman), the itch build (`itch.bat`, `shipping`
feature), recorded with OBS Game Capture between PLAYTEST_START and PLAYTEST_END, 6 min 9 s.

- Build: `main` at `7dc1e43`, release, `--features shipping`.
- Machine: Intel i9-11900H, NVIDIA RTX 3060 Laptop GPU, Windows 11; 1920x1080 window, OBS NVENC CQP 16, 60 fps.
- Result: `PLAYTEST_RESULT lose defence: out of time after 360 s, city 1 5 %, city 2 0 %, city 3 0 %`
- Drive, full take: `dungeon-crawler-2026/crab-breeze-2026-10-25-defence-take.mp4`, https://drive.google.com/file/d/1gQAxqVqp24IwKw3kRuuN0GHycI-2EpRA/view
- Drive, 56 s trailer cut from it: `dungeon-crawler-2026/crab-breeze-2026-10-25-trailer.mp4`, https://drive.google.com/file/d/1iOrHwaeOeGdNFXe8pfKtKjAdcqKP6yqz/view
- Contact sheet (one frame every 10 s): `2026-10-25-defence-windows_frames.png`

| Beat | What the take shows |
| --- | --- |
| Board | on foot to the Foreman crouched in the street, its chest open; the climb in and the cockpit powering up |
| fight1 | wave 1 (10 raiders, 3 vehicles) in city 1; smoke and a fireball early, then minutes facing facades |
| Finale | from about 5 min 30 s, a street battle: smoke, explosions, debris, a large blast just before the cap |

## What looked wrong

- Only wave 1 was played: the scripted pilot destroyed no raider (`raiders 11/13` rising to `12/13`) and
  the take hit `DEFENCE_TAKE_MAX_S` (360 s) long before the 600 s wave clock. Waves 2 and 3 never started.
- For most of the take the pilot stands still facing building walls (`0.0 m/s mode Walk` in the beat log),
  as in the Mac take of the same day: the pilot needs wall avoidance and a line of sight to its target.
- The sun flare's ghosts drew as solid black hexagons and discs (only their blue rims lit), seen
  in the cockpit as the mech powers up. Cause: `pow()` of a negative base in the ghost rim
  (`sun_flare.wesl` `ghost_body`), undefined in WGSL and NaN on NVIDIA Vulkan/DX12; Metal on the Mac
  happened to get it right. Fixed by squaring by hand (branch `fix/flare-ghost-nan`).
- `Mesh ... contains no vertices` warnings are logged throughout the fight (nothing wrong seen on screen).
