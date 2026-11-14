# The Fargo-like city in game (step 1)

Headless stills of the capital, seed 42, in a Linux cloud session (lavapipe software Vulkan under Xvfb, about 1 frame
a second), 2026-11-04. openspec change `city-districts`, design section 4, step 1: the Fargo-like town on today's grid.

| Still | What it shows |
| --- | --- |
| `fargo-air.jpg` | the capital from 900 m off: towers at the middle, flats round them, house blocks to the edge, parks |
| `fargo-houses.jpg` | a house block from 55 m up: two rows of houses facing their streets, front yards, drives to the kerb, the pavement ring |
| `fargo-street.jpg` | down a street at eye height: houses on the right, the strip's businesses on the left, people, parked cars |
| `fargo-strip.jpg` | a strip block: businesses with striped car parks on the worn asphalt, paler than the streets |
| `fargo-downtown.jpg` | into the downtown from 30 m up |
| `fargo-highway.jpg` | the highway coming in, on Concrete047A |

Made with the named-view stills (`CB_STILLS_WEATHER=1`, views `fargo-*` in `crates/cb-app/src/capture.rs`):

```sh
cargo build --release -p cb-app
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json CB_STILLS_WEATHER=1 CB_HOUR=11 CB_STILLS_SETTLE_S=50 \
  CB_STILLS_ONLY=fargo-air,fargo-houses,fargo-strip,fargo-downtown,fargo-highway,fargo-street \
  xvfb-run -a -s "-screen 0 1920x1080x24" ./target/release/crab-breeze --stills output/stills-fargo
```

Under the software renderer a view needs `CB_STILLS_SETTLE_S=50` for the houses' near models to stream in; at the
default 8 s they show as their plain far shells. The frame counter in the corner is the software renderer's, not a
measurement.

## Step 2: the highway through the capital on a viaduct, with exits (2026-11-13)

Design section 5. The same headless setup, seed 42, `CB_STILLS_SETTLE_S=40`.

| Still | What it shows |
| --- | --- |
| `fargo-exit.jpg` | an exit from 140 m up: the trunk on its viaduct along the row of car parks, the piers, four one-way ramps down to the frontage streets beside the arterial |
| `fargo-under.jpg` | under the viaduct at eye height: the slab's underside, a pier with its cap beam, the ramps on their fill walls either side |
| `fargo-gore.jpg` | (2026-11-14) above where an off-ramp meets the viaduct: it swings out level to the viaduct's edge, runs beside it with no barrier between, then descends to the street |
| `fargo-ramp.jpg` | at a ramp's landing beside the viaduct: cars waiting at the light where the ramp meets the street |
| `fargo-air-trunk.jpg` | the capital from 900 m off with the trunk across it, its exits and the two highways climbing onto it |

## Downtown shapes (2026-11-13)

Design section 6: each city draws its downtown's shape; the capital is compact.

| Still | What it shows |
| --- | --- |
| `fargo-main-street.jpg` | the first city whose downtown is a main street (in the desert), from the air |
| `fargo-downtown.jpg` | (re-shot 2026-11-14) into the capital's downtown: towers standing apart on their lots, mid-rise blocks round them, streets between (no longer one wall of towers) |
| `fargo-two-cores.jpg` | the first city whose downtown has two cores (it also has a trunk), from the air: two clusters of towers in a ring of flats |

```sh
CB_STILLS_ONLY=fargo-exit,fargo-under,fargo-ramp,fargo-air,fargo-main-street,fargo-two-cores CB_STILLS_SETTLE_S=40 ...
```
