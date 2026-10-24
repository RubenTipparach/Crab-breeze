# Showcase take: the island world, 2026-10-24

`crab-breeze --showcase world` (stand-in mech, seed 42) on the 41 km island, recorded with OBS
between PLAYTEST_START and PLAYTEST_END, 3 min 32 s. The scripted take writes only input actions.

- Commit: `b52bf92` (the island world, the Pale Blue Dot water, the tree crowns, rockets up and
  skates across), owner-approved run (2026-10-24: "Record video, screenshots, upload to gdrive").
- Machine: Apple M4 Max (40-core GPU), 64 GB, macOS; release build; 1920 x 1080 window, vsync,
  60 fps capture (OBS ScreenCaptureKit window capture). Frame cost is not measured in a recorded
  take; the separate benchmark is `docs/benchmarks/2026-10-24-island/`.
- Result: `PLAYTEST_RESULT ok showcase: boarded, strode, flew, waded, felled trees, shelled, limped`
- Drive: `dungeon-crawler-2026/crab-breeze-2026-10-24-island-world.mp4`, https://drive.google.com/file/d/1hLaWvyWcRdxnHSfTJ6OMkgfVn6zPFv2O/view
- Contact sheet: `2026-10-24-island-world_frames.png`

| Beat | What the contact sheet shows |
| --- | --- |
| Street | seated in the cockpit, walking the first city's street; the HUD's heading reads `HDG 140 SE  IN CITY 1` |
| Climb | on the rockets at 848 m over the sea, skates on, the gauges counting both budgets down (rockets 9.7 s left) |
| Overflight | the island's coast, a river and the woods past the towers, `HDG 287 W`, 50 m up |
| Back in the street | stopped, gauges full again, the leg compass straight |

## What looked wrong or is unproven

- The sheet has four frames in a six-slot grid; the take itself is whole (3 min 32 s).
- The climb targets a cloud, and clouds are off in data, so the take climbs to the empty spot
  (848 m) and turns back: the cloud beat shows sky only until the cheap clouds come.
- Not judged yet: the water's new look up close (the river beat), the sea skim, and the drips on
  the glass when surfacing, which this take does not do.
