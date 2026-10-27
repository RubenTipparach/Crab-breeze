# Sky and ground-fog flicker: old against new, 2026-10-27

Owner, 2026-10-26: "the whole sky ios flikering", "also ground color fog flickerewd as well". This is the A/B of
`docs/playtests/sky-fog-flicker-test.md`, section 2, for `openspec/changes/ground-fog` task 5.3, run on the owner's Mac
with the owner's go-ahead.

- **OLD**: `d61a4f3` (before the fix), built in its own worktree. Binary md5 `cd1511cc647f4d2525a382ebae478b23`.
- **NEW**: `c475fbd` (this branch's head when the takes were made: the fix of design section 2b, `EyeWeather` and
  the rain's reach), built in its own worktree. Binary md5 `064b4e699d07e2f084d8bdebd38b7688`.
- **Machine**: Apple M4 Max (16 cores, 40-core GPU, 64 GB), macOS, the built-in Liquid Retina XDR display at
  **120 Hz** (1728 x 1117 points), release builds, windowed. Recorded with OBS 32.2 (ScreenCaptureKit window capture)
  at 60 fps, half size (1676 x 1078), from `PLAYTEST_START` to `PLAYTEST_END`. Each take run on its own under
  `tools/perf/mac_lock.sh`. No frame-time numbers: these are recorded takes.
- **Takes**, all first person, the walk to the mech and the climb in included:
  - `--playtest streaming-route`: skates past city 10, the capital and city 3, then jets at 120 m. OLD ended
    `stuck` on its last jet leg (the old build's jets; 5 km short, the take still has 3 minutes of flight); NEW
    `ok, every beat played`.
  - `--playtest storm-pass` (new, `data/playtests/storm-pass.ron`): task 5.3's "fly low past a storm at speed,
    seated". Jets at 80 m along the west storm's north edge, round its east side, back west under its base and
    through its rain, then out north. Both `ok, every beat played`.
- **Drive** (Drive for Desktop folder `My Drive/dungeon-crawler-2026/`, H.264 CRF 26, preset slow, no audio; the
  storm-pass takes): `crab-breeze-2026-10-27-sky-flicker-old.mp4` (86 MB) and
  `crab-breeze-2026-10-27-sky-flicker-new.mp4` (90 MB). Written to the folder; not confirmed with the Drive connector.
- **Contact sheets** (storm-pass, one frame every 15 s): `2026-10-27-sky-flicker-old_frames.jpg`,
  `2026-10-27-sky-flicker-new_frames.jpg`.

## How the sky was measured

Three crops of the front screen, the same pixels in every take: two of sky either side of the overhead screens
(70 x 120 at (1230, 250) and 75 x 120 at (375, 250)) and one low over the ground for the fog (320 x 40 at (380, 610)).
`ffmpeg ... crop=W:H:X:Y,signalstats,metadata=print` gives `YAVG`, `UAVG` and `VAVG` (of 255) for every frame;
the seated part of each take is used (from 50 s OLD route, 35 s the others).

The raw frame to frame change is mostly the view moving, not the sky: the scripted pilot's head bobs in the seat, the
mech turns, and trees and towers pass through a crop while skating. Every jump over 10 levels that was looked at
was one of those (a pine crossing the right-hand crop at 59.8 s NEW route, the head's bob at 147 s, the city's towers
at 303 s). So two filters, both reported:

- **View still**: a frame pair counts only when a crop over the lower left screen's bezel and the dark cabin behind
  it (110 x 110 at (410, 670), no sky in it) moved under 0.5 of 255, and the sky crops hold no cockpit (`YMIN` 50 or
  more).
- **Whole sky**: flicker of the kind reported moves the whole sky at once, so on still pairs the change counted is the
  smaller of the two sky crops' changes when both move the same way.

### Whole sky, still view (levels of 255, frame to frame)

| Take | Still pairs | Y p50 | Y p95 | Y max | U p95 / max | V p95 / max | pairs over 2 |
| --- | ---: | ---: | ---: | ---: | --- | --- | ---: |
| OLD route | 9,594 | 0.015 | 0.183 | 12.78 | 0.044 / 1.11 | 0.049 / 1.89 | 32 |
| NEW route | 14,131 | 0.019 | 0.187 | 11.72 | 0.049 / 2.08 | 0.058 / 2.96 | 22 |
| OLD storm | 22,343 | 0.015 | 0.177 | 3.21 | 0.039 / 0.25 | 0.039 / 0.31 | 2 |
| NEW storm | 23,704 | 0.015 | 0.164 | 2.25 | 0.049 / 0.32 | 0.048 / 0.24 | 3 |

The route's largest whole-sky jumps (12.8 at 100 s OLD, 11.7 at 88.6 s NEW, and their neighbours) are both crops
filling with pines while skating through the forest by city 1; the NEW route's 9.2 at 302.9 s is the capital's towers
coming into view. The storm-pass's largest (3.2 at 342 s OLD, 2.3 at 367 s NEW, both on the jets past the storm's
west side) are a dark puff drifting across the crop: the strip `2026-10-27-sky-flicker-worst.jpg` (OLD top, NEW
bottom, twelve frames each round the worst pair) shows the sky changing slowly, no frame out of step.

### One sky crop, still view

| Take | Crop | Y p50 | Y p95 | Y max | pairs over 2 |
| --- | --- | ---: | ---: | ---: | ---: |
| OLD storm | right | 0.062 | 0.381 | 16.13 | 5 |
| NEW storm | right | 0.063 | 0.369 | 24.79 | 19 |
| OLD storm | left | 0.048 | 0.362 | 7.26 | 13 |
| NEW storm | left | 0.044 | 0.348 | 19.80 | 10 |
| NEW route | ground fog | 0.041 | 0.233 | 11.44 | 31 |

One crop alone still catches clouds and terrain sliding across it as the mech flies; the maxima are those, the medians
and 95th percentiles are the same in both builds. The ground-fog crop could only be measured on NEW (on OLD the
ground under it is darker than the cockpit filter's floor); its jumps are trees.

### Headless instrument (NEW)

`CB_STILLS_WEATHER=1 CB_STILLS_BURST=20 CB_STILLS_BURST_STEP_M=1`, the eye stepping 1 m a frame (60 m/s at 60 fps):
`under-the-storm` (in the rain, 40 m up, sun share 0.15) and `storm-shadow` (900 m up across the shadow's edge) held
the top third of the frame to 1 level of 255 over 20 frames. (`storm-under-edge` now stands inside a house built at
that spot since the view was placed; its jumps are the walls.)

## Verdict: not reproduced on the Mac, fix unconfirmed

NEW shows no whole-sky flicker: with the view still, the whole sky's colour moves 0.16-0.19 of 255 at the 95th
percentile frame to frame, and its U and V (the colour) never more than 0.32 on the storm pass, the case the owner
flew. **But OLD measures the same** on this Mac (60 fps of a 120 Hz display). The flicker the owner saw did not
appear here in the build that is meant to have it, so this A/B cannot confirm the fix; it shows only that NEW is
steady on the Mac.

The owner's machine is a Windows desktop with an RTX 3070, and the flicker may show only there (another GPU, driver
and display; or it alternated frame to frame and OBS's 60 fps sampling took one phase of it). Task 5.3 stays open
for a desktop run: section 1 of the test procedure by eye, then this A/B. If it shows there, the next suspects are as
task 5.3 lists them: the sun flare's 1x1 occlusion read a frame late (`sun_flare.rs`) and Bevy's atmosphere LUTs at
the eye's height.

A harder attempt to reproduce it on OLD (through the rain shafts and the cloud, low and fast, the sun near the
screen's edge, and the 1 m a frame burst instrument ported onto OLD) was cut short: the Mac's screen saver came up at
about 11:00 and every frame the game drew after it was black, so those runs measured nothing.
