# Showcase take: the world, 2026-10-24

`crab-breeze --showcase` (stand-in mech, seed 42), recorded with OBS between PLAYTEST_START and
PLAYTEST_END, 3 min 25 s. The scripted take writes only input actions; test shells and part hits
come through the F5-F9 test actions until weapons exist.

- Machine: Apple M4 Max (40-core GPU), 64 GB, macOS; release build; 1920 x 1080 window, vsync,
  60 fps capture (OBS ScreenCaptureKit window capture). Frame cost not measured in this take
  (recording runs beside it).
- Result: `PLAYTEST_RESULT ok showcase: boarded, strode, flew, waded, felled trees, shelled, limped`
- Drive: `dungeon-crawler-2026/crab-breeze-2026-10-24-showcase-world.mp4`, https://drive.google.com/file/d/1o4yncCYexhNpEyIBHTBDrIuEq4vP2JEp/view
- Contact sheet: `2026-10-24-showcase-world_frames.png`

| Beat | What it shows |
| --- | --- |
| Walk and board | the pilot walks to the stand-in, Use once, it sits, the climb up past the lift rails |
| Street | stand up and stride down the street |
| Jets | climb on the jets toward a cloud; the view whites out inside it (fly-through fog) |
| Overflight | the island from 600 m: cities, forests, the river, the lakes and the sea sheet beyond |
| River | jets brake the descent into the river; wading with spray and ripple rings on each stride |
| Wood | out of the river into the trees; trunks snap and topple across the view, dust where they land |
| Shells | four test shells: craters stay in the ground, dirt, dust, smoke, sparks, flashes |
| Damage | left arm shot off (wreck tumbles, stump burns), head lost (backup-camera grade), left leg lost: the mech drops onto its stump, the horizon tilts, it limps |

## What looked wrong

- Particles are camera-facing sprites with no soft depth fade: they cut against the ground.
- The sea sheet stops 1 km past the world's edge; from altitude its square edge shows.
- The limp is readable but stiff; the hop needs the stump to plant and push.
- No sound; no cockpit instruments for heat, damage or wading depth yet.
