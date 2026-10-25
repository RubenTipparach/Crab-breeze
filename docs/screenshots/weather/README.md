# The weather in the game: lavapipe stills, 2026-10-25

Stills of `cloud-weather` as built on `environment-design`: billboard clouds in seeded weather
systems, rain shafts, cirrus, the storm shade on the sun and rain on the glass. Taken in a cloud
session: the release build, Mesa's software Vulkan (lavapipe) under Xvfb, 1920 x 1080, reduced to
1280 px and 256 colours here. **No frame times:** the software renderer draws about one frame a
second, which says nothing about the game on the owner's RTX 3070 (CLAUDE.md section 7). The
yellow numbers in the corner are lavapipe's.

How: `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json CB_STILLS_WEATHER=1 xvfb-run -s
"-screen 0 1920x1080x24" ./target/release/crab-breeze --stills output/stills-weather`, seed 42.
The instrument (`crates/cb-app/src/capture.rs`, `WeatherStillsPlugin`) holds the pilot's eye at
each named view, waits for the ground and sky to stream in, and saves one still a view;
`CB_STILLS_ONLY=storm,deck` shoots only those. It prints the core's cloud density, rain and sun
share at each eye:

| Still | Where | Core at the eye | What it shows |
| --- | --- | --- | --- |
| `storm.png` | 13 km south of the western storm cell, 160 m up | sun 1.0 | The cell's towers with slate bases, altocumulus to the right, cirrus |
| `under-the-storm.png` | Under the cell, 40 m up, facing away from the sun | rain 0.70, sun 0.15 | The storm's dark base overhead, rain drops on the glass, the ground darkened, the front's towers and rain on the horizon |
| `storm-shadow.png` | 900 m up outside the cell, looking down across its edge | sun 1.0 | The storm's shadow darkening the ground on the right |
| `front.png` | The west coast looking out to the front | sun 0.67 | The front's layered grey band with towers |
| `crater.png` | The crater floor looking west at the rim | sun 1.0 | Cumulus and the storm beyond the rim |
| `desert.png` | The dry east looking west | sun 1.0 | Fair cumulus thinning over the desert, altocumulus |
| `deck.png` | The north coast looking out to sea | sun 1.0 | The stratocumulus deck along the horizon, towering cumulus |
| `streets.png` | The south coast, into the sun | sun 1.0 | Backlit cumulus, grey not black (light scattered inside) |
| `overview.png` | 11 km up, 30 km south | | The weather over the island |

## What changed between takes

The owner looked at the first take (2026-10-25): "clouds are too white, they need some shadowing on
the bottom, and they seem to not sort correctly".

- **Sorting.** The puffs were sorted by the straight-line distance to their centres, which
  misorders puffs off to the side and big puffs beside small ones. They are now sorted by depth
  along the view direction (exact for camera-facing quads), and re-sorted when the view turns 3
  degrees as well as when the eye moves 30 m. A test pins the side-puff case.
- **Undersides.** Each puff darkens toward its own underside and over the last 350 m above its
  system's base, which is cut flat; the sun wraps less (0.35, was 0.6); the base shade is 0.55 (was
  0.4). Light scattered inside a puff (0.2 of the sun) keeps backlit clouds grey: before, they went
  black-brown.
- **The storm shade** was there but weak: a debug checkerboard in place of the map darkened the
  ground in 4 km squares, and the map held 0.4 under the cell, as the core gives, but the sky's
  ambient light washed it out. The cut under a cell is now 85 % (under the front 60 %).

## What is still wrong

- The sea is a flat saturated blue (`water-look`, step 5 of `docs/design/world-look-plan.md`).
- The overview shows a pale square past the world's edge: the sea floor's edge, not the clouds.
- Rain shafts show only as a faint grey band at the horizon; they need a look on the owner's
  machine.
- The ambient light does not darken under a storm, only the sun.
