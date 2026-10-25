# Showcase take: the demolition, 2026-10-24

`crab-breeze --showcase demolition` (seed 42), recorded with OBS between PLAYTEST_START and
PLAYTEST_END, 31.5 s. The scripted take writes only input actions. The frame-time numbers are in
`docs/benchmarks/2026-10-24-demolition-mac/`, from separate runs with no OBS.

- Commit: `c85d1de` (`building-research`), owner-approved run (2026-10-24: "Can you record video?").
- Machine: Apple M4 Max, macOS; release build; OBS ScreenCaptureKit window capture, 60 fps.
- Result: `PLAYTEST_RESULT ok demolition: building 204 cells=144/216 frame_elements=590`
- Drive: `dungeon-crawler-2026/crab-breeze-2026-10-24-demolition.mp4`,
  https://drive.google.com/file/d/1D1ktYjDBEJ04VrsXpjU17mZC36SIxTkT/view
- Contact sheet: `2026-10-24-demolition-mac_frames.png`

| Beat | What the contact sheet shows |
| --- | --- |
| Settle | on foot in the street, the stand-in mech ahead, towers either side (the take waits 12 s after loading) |
| Whole | looking up at the glass tower, intact |
| Shelling | shells bursting on the face, debris and fire, dust rising |
| Fall | the tower comes down into a dust cloud, slabs falling, the frame standing open floor by floor |
| Close | at the rubble pile, slabs and broken frame up close |

## What looked wrong or is unproven

- The first third of the take is the settle wait, a still street: worth trimming for a showcase.
- The rubble slabs are flat untextured boxes, and the dust cloud washes the view to one beige.
- The collapse differs run to run (144 cells standing here, 176 and 208 in the benchmark runs).
