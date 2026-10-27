# Testing the sky and fog flicker fix

Owner, 2026-10-26: "the whole sky ios flikering", "also ground color fog flickerewd as well", "you can see
subtle differences in sky color". This is how to check the fix on a desktop. The fix and why it should work are
in `openspec/changes/ground-fog/design.md`, section 2b, and task 5.3 in `openspec/changes/ground-fog/tasks.md`.

## What changed

- **The weather at the eye no longer jumps.** The rain, cloud and clouds' shade at the pilot's eye set the fog over
  the whole view. They were read raw every frame, so passing the edge of a rain shaft, a puff or a cloud's shadow
  at speed swung the colour of the whole sky. They now follow over 1.5 s (`eye_weather_s` in
  `data/world/world.ron`, `ground_fog::EyeWeather`).
- **Rain hazes only 1.5 km of a sight line** (`rain_reach_m`), not the sky's whole 40 km: a trace of rain at
  the eye no longer greys the whole sky.
- **The fog takes the sky's own colour**, so far land and sea meet the sky in one colour at the horizon, darker
  under the clouds' shade (`storm_dim`).

## 1. By eye (5 minutes)

1. Build and run: `./run.sh` (or `cargo run --release -p cb-app`).
2. Board the mech, skate out of the air base, then take to the jets at 100-200 m.
3. Fly at full speed past a storm: under its edge, past its rain shafts, in and out of its shadow, and close
   by low puffs. That is where the flicker showed.
4. **What you should see:** the sky and the fog's colour change slowly (over a second or two) as you pass a
   storm, never frame to frame. The far land and sea fade into the sky at the horizon in one colour, with no
   brown or grey band.
5. **If it still flickers,** take two screenshots a frame apart and note:
   - what is on screen: clouds, a rain shaft, a city coming into view;
   - whether the sun is near the edge of the screen. The sun flare is the next suspect: it reads its occlusion a
     frame late.

## 2. Measured, old against new (30-45 minutes)

Run the same scripted route on the build before the fix and on the build after it, then compare the sky's colour
frame by frame.

1. Build both:
   - OLD: commit `d61a4f3` (before the fix).
   - NEW: this branch's head.

   For example:

   ```sh
   git worktree add ../cb-old d61a4f3
   (cd ../cb-old && cargo build --release -p cb-app)
   cargo build --release -p cb-app
   ```

2. Run the route on each build, recording it with OBS between `PLAYTEST_START` and `PLAYTEST_END` (the
   `playtest` and `obs-record` skills):

   ```sh
   ./target/release/crab-breeze --playtest streaming-route
   ```

   It boards, skates past city 10, the capital and city 3, then jets at 120 m toward city 5 and back over the
   capital.
3. Pick a crop of the front screen that shows only sky above the horizon for most of the route, and check it on a
   contact sheet. Then get the sky's brightness and colour for every frame:

   ```sh
   ffmpeg -i take.mp4 -vf "crop=W:H:X:Y,signalstats,metadata=print:file=sky.txt" -f null -
   ```

   `sky.txt` holds `YAVG` (brightness), `UAVG` and `VAVG` (colour) for each frame.
4. For each take, report:
   - the p50, p95 and maximum of the frame-to-frame change in `YAVG`, `UAVG` and `VAVG`;
   - how many frames change by more than 2 levels (of 255);
   - where on the route the biggest jumps are, by time and whether skating or on the jets;
   - a strip of crop frames round the worst jump.

   Flicker is fast swings, frame to frame or a few frames apart. A slow drift as the light changes is not
   flicker.
5. Upload a share copy of each take to Drive: `crab-breeze-<date>-sky-flicker-old.mp4` and `-new.mp4`.
6. Write the result to `docs/playtests/<date>-sky-flicker.md` with:
   - both commits, the machine and the display's refresh rate;
   - the numbers and the Drive links;
   - the contact sheets;
   - a verdict: gone, reduced or unchanged.

## 3. Headless instrument (cloud sessions, no GPU)

`CB_STILLS_BURST=<n>` shoots n consecutive frames of a weather still view. With `CB_STILLS_BURST_STEP_M=<m>`, the
eye moves m metres along the view between frames (1 m is 60 m/s at 60 frames a second).

```sh
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json CB_STILLS_WEATHER=1 CB_STILLS_BURST=8 \
  CB_STILLS_BURST_STEP_M=1 CB_STILLS_ONLY=storm-under-edge \
  xvfb-run -a -s "-screen 0 1920x1080x24" ./target/release/crab-breeze --stills output/burst
```

On 2026-10-26 the `sea-away` view, held still, measured steady to 0.1 of 255 over 8 frames. The flicker comes
from something that changes as the eye moves, not from the passes themselves.
