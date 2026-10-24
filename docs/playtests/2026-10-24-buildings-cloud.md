# Buildings in the game against the mockup: cloud stills, 2026-10-24

**This is not a playtest.** It is headless stills from a cloud session: Mesa's software Vulkan
(lavapipe) under Xvfb, 4 CPU cores, no GPU. Frames took 1.3 to 2.4 s each. **No frame times are
claimed here** (CLAUDE.md sections 7 and 9). No OBS, no Drive upload.

- Branch `building-research`, tested at `c14d1ec`; the stills in this report are from the build at
  `f3bfb8a` (the glass and sky light change), the "before" glass from the build at `63c4380`.
- Command: `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json CB_STILLS_BUILDINGS=1 xvfb-run -s
  "-screen 0 1920x1080x24" ./target/release/crab-breeze --stills output/stills-buildings`, seed 42.
- Mockup: `NODE_PATH=$(npm root -g) node tools/mockups/shoot_buildings.mjs docs/mockups/buildings.html
  output/mockup-shots` (Chromium, software WebGL). Its `*_floorplan` shots time out; the main shots
  were taken.
- Stills (1280 px) and sheets: `docs/screenshots/buildings-game/`.

## Checks

| Check | Result |
|---|---|
| `cargo fmt --all -- --check` | failed on `c14d1ec` (one block in `capture.rs`); formatted, passes |
| `cargo clippy --all-targets -- -D warnings` | passes (Rust 1.99.1; 1.97.0 was too old for Bevy 0.20's 1.97.1 floor) |
| `cargo test --workspace` | 221 pass (217 at `c14d1ec`, all passing; 4 new for `sky_light`) |
| `openspec validate --all` | 41 pass |
| Dash check | passes (now skips `.hdr` files, binary pixel data) |
| `cargo build --release -p cb-app` | builds |

Nothing in the building code failed its tests. The instrument needed four fixes to work on a slow
renderer, all in `crates/cb-app/src/capture.rs` and one flag in `buildings.rs`:

1. **Wait for `GAME_READY`** before the first beat (it began after 6 s whatever the loading screen
   said).
2. **The fall's shots** fired only on a frame within 1/30 s after their time, which 1.3 s frames
   step over: `building-rubble-close` and others were missing. Each is now taken once its time
   has passed.
3. **The game's clock.** The beats ran in real seconds, and Bevy's virtual clock advances at most
   0.25 s a frame, so the simulation ran about five times slower than the script and the "fall"
   shot showed a building still standing. The building beats now run on the virtual clock.
4. **Dressing.** Buildings are dressed one a frame within 250 m. With 3 s a shot (2 frames) the
   first tour shots showed plain boxes with punched windows. Each tour shot now waits until no
   building near the pilot is waiting to be dressed (`DressingSettled`), then 3 s.
5. **F9 on every column.** The aim was picked by time, so long frames skipped columns: only 4 of
   216 cells broke. Each column is now shelled in turn, on an aim held since the frame before.

## The street tour

The game's tour picks the nearest walk-up (building 160, 6 storeys), office (148, 14 storeys) and
tower (208, 25 storeys) to the mech, and shoots each from across the street, then the tower from
7.9 m up. The mockup is its own block (seed 42 of its own layout), so the sheets compare **kinds of
building**, not the same building. Game left, mockup right:

- `compare-walkup-street.jpg`: game walk-up against the mockup's street of walk-ups and warehouses
- `compare-office-street.jpg`: game office street against the mockup's roofs view
- `compare-tower-street.jpg`: game tower against the mockup's tower cutaway
- `compare-cockpit-height.jpg`: game tower from 7.9 m against the mockup's cockpit-height view

### Point by point

| Point | Game | Against the mockup |
|---|---|---|
| Sash windows | Walk-up: two-pane sashes in punched openings, a row per storey | Match in kind. The mockup's are larger, with visible glazing bars and frames |
| Sills and lintels | Sills read as a thin light line; lintels barely show | Weaker. The mockup's stone sills and lintels read from across the street |
| Cornice, parapet | No cornice seen on the walk-up; the roof line is a flat edge | **Missing** in the shots. The mockup's walk-ups have a stone cornice |
| Fire escapes | None seen | **Missing.** They are the mockup's strongest walk-up feature |
| Ribbon windows with fins (office) | Concrete grid of piers and spandrel bands with windows between | Close in kind; the 0.8 m fins do not read at this angle |
| Curtain wall (tower) | Mullions every 1.5 m, dark spandrel bands at each slab, rooms behind | Match. Now with tinted, reflective glass (below) |
| Steel sash (warehouse) | No warehouse near the start, none shot | **Not seen** |
| Brick | Brick texture on the walk-up | Duller and flatter than the mockup's, under the haze |
| Concrete | Grey board-formed texture on the office | Reads, faintly |
| Wood | The walk-up's door | Match |
| Roof, roof props | Not visible: no shot looks down on a roof | **Not seen.** Needs a shot from above (the mockup's water tanks, HVAC, masts) |
| See-through glass, rooms behind | Rooms with furniture behind every curtain wall and office window | Match |
| Doors | A wooden door on the walk-up; a canopy entrance on the tower | Match |
| Shopfronts and awnings | Glazed shopfronts with flat coloured awnings | In kind; the mockup's are white, sloped and deeper |
| Variety | One brick building in four shots; downtown is all grey concrete and glass | **Weak** (below: "Where the brick is") |
| Z-fighting, floating pieces | Thin light vertical strips across the walk-up's brick at irregular places, possibly downpipes or frame members showing through the skin; to check close up. In `building-rubble-close`, a debris slab overhangs the camera | Needs a closer look on a GPU machine |
| Light | A grey-blue haze (the distance fog) over everything; soft shadows | The mockup is clear and bright, with hard shadows |

### Where the brick is (owner, 2026-10-24: "why aren't you utilizing brick buildings")

Measured on seed 42 with a throwaway test: 6 100 buildings, 2 903 walk-ups and warehouses (brick,
48 %), 1 883 offices, 1 314 towers. In the capital, where the game starts: 447 buildings, 180
brick (40 %). Brick is used, but **only on buildings of 6 storeys or fewer**
(`dressing::style_of`), and height falls with distance from the centre (24 to 60 storeys
downtown, exponent 0.7), so brick sits in a ring at the city's edge and downtown, where you
start, has none.

The style itself is drawn per building (`dressing::look_of`), and measured with a throwaway test
over the same seed it gives, within 500 m of the start (393 buildings): 150 curtain-wall towers
(38 %), 81 ribbon-window offices, 50 punched-grid concrete offices, 79 brick walk-ups and 33 brick
warehouses (28 % brick). Over the world: 1 655 towers, 1 148 + 730 offices, 2 057 walk-ups, 510
warehouses. Five facade kinds in all, each varied by colour (6 brick tints, 4 concrete, 4 glass,
3 mullion), frame colour and window rhythm; brick, concrete, roofing and wood carry normal maps,
glass and trim (frames, mullions, sills) do not. The owner: "are we using all kinds of
buildings? all i see in screenshots is the one glass type, there should be many other tpyes, with
buyildings that have normal maps". Five kinds is too few, and downtown is dominated by one of
them: OpenSpec change `building-variety` proposes more. Survey W18 recommends picking the material per building: brick on about 40 % of
7-14 storey buildings (pre-war lofts and offices) and brick podiums under some towers.

## The glass (owner, 2026-10-24)

"That glass needs better reflections/materials to be reflective, cache some cube maps?",
https://polyhaven.com/hdris, "usually glass is tinted", "so it reflects more light, and interiors
appear darker than the world around it". OpenSpec change `glass-reflections`:

- **Why it looked like clear plastic**: no environment map on the world camera (nothing to
  reflect but the sun) and alpha blending, which scaled the reflection down to 35 %.
- **Built**: a Poly Haven sky (CC0) baked once to a cube, filtered on the GPU once and cached on the
  eye camera; glass premultiplied (reflection added whole), tinted blue-green, alpha 0.7 (the rooms
  show at 30 %), reflectance 0.9 (about 13 % head-on, toward a mirror at grazing angles).
- `glass-before-after-tower.jpg` and `glass-before-after-walkup.jpg` (before left, after right):
  the tower's glass now carries the sky's clouds, and the rooms behind sit darker than the street.
  `building-whole.jpg` shows the clouds along the tower from its foot.
- **Not yet**: the reference photos' towers mirror the buildings across the street. The sky cube
  reflects only sky. Cached street cube maps at each crossing are designed (survey W21).
- **Second pass, mirror glass** (owner: "glass still doesnt look right, too transparent", "it should
  go harder on the reflections", "in real life, thered be less light on the inside of the
  building, so the reflectivity shuold be way stronger"). Plain glass in Bevy reflects at most
  16 % head-on, so the pane now has a reflective coating (Bevy's `metallic` 0.9 over a blue-grey
  tint: about 45 % head-on) and lets 12 % of the room through (alpha 0.88), as a room lit only
  through its windows would look. `glass-mirror-before-after-tower.jpg` and
  `-office.jpg` (first pass left, mirror right): the curtain walls now read as mirrors full of
  cloud, the office windows reflect the sky, the rooms are a faint shadow behind. The tour stills
  and the four comparison sheets are from this build (the demolition stills are from the first
  pass; the glass does not change the fall).
- **Seen in the mirror pass**: light triangular shards along the towers' side edges, in every run
  since the first; to look at on a GPU machine.
- Two runs went wrong on the way and are left out: a 20-frame cache (run 4) was taken before the
  filter pipelines compiled and lit the world from a black map (street brightness 49 against
  109); the cache now waits for frames the render world saw filtered.

## The demolition

Building 204 (18 storeys, seed 42) at (2348, 547); the pilot 11 m off its -z face shells the foot
of each column on that face. `demolition-sheet.jpg`: whole, hit, creak, fall, rubble at
12 s, rubble close.

| When | Cells standing | Frame elements |
|---|---|---|
| Before | 216 / 216 | (frame not built yet) |
| Shelling | 214, 213, 201, 183 | 824, 819, 798, 766 |
| Falling | 172, 154, 146 | 739, 661, 617 |
| Settled (to the end) | 144 / 216 | 590 |

The same numbers came out in two runs (deterministic). A third of the building (72 cells) came
down: the facing side's bays peeled off, as structural destruction designs; the rest stands.

- **Hit**: orange shell bursts, sparks and dust; slabs coming off the face. Matches the mockup's
  look of a hit.
- **Creak and fall**: a wall of brown dust fills the street; slabs fall through it.
- **Rubble at 12 s**: the dust has thinned; the surviving bays stand stripped to slabs, rails and
  frame.
- **Rubble close**: a pile of large slabs and a brown heap. The debris is flat grey, untextured
  boxes: the mockup's rubble carries the building's materials. One slab overhangs the camera.

## What to look at next, on a GPU machine

1. The sky light and glass in motion; measure the frame with and without it
   (`glass-reflections` task 1.5).
2. The stray vertical strips on the walk-up's brick, close up.
3. A shot from above to check roofs, parapets and props.
4. Fire escapes and cornices on walk-ups, and warehouses near the start, both missing here.
5. Textures on falling debris.
