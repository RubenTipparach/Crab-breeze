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
