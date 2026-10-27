# Houses and small businesses in the game: cloud stills, 2026-10-27

**This is not a playtest.** These are headless stills from a cloud session: Mesa's software Vulkan (lavapipe) under
Xvfb, 4 CPU cores and no GPU. Frames took 2 to 4 s each. **No frame times are claimed here** (CLAUDE.md sections 7
and 9). No OBS, no Drive upload. The pilot never sat in the mech: the stills instrument places and turns the pilot,
and the hit is an `Explosion` written through the game's own blast path. The playtest belongs on the owner's machine:
`./run.sh`, walk from the start towards the edge of the city (the suburbs ring it 40-150 m out).

- Branch `claude/youthful-tesla-zrsjb8`, seed 42, `--mech mechs/foreman.ron`.
- Command: `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json CB_STILLS_HOUSES=1 xvfb-run -a -s "-screen 0
  1920x1080x24" ./target/release/crab-breeze --mech mechs/foreman.ron --stills output/stills-houses`
- Stills (1280 px): `docs/screenshots/houses-game/`. The modelled furniture: `docs/screenshots/furniture/sheet.png`.
- Change: `openspec/changes/small-buildings` section 7.

## What the stills show

| Still | Shows |
| --- | --- |
| `suburb.jpg` | A ranch house (variant a) with its garage and car, colonials and other houses along the row, all facing the city; the far ones at LOD1 |
| `door.jpg` | The front door swung open as the pilot reached it; the hall, the dining table and a bedroom beyond |
| `inside.jpg` | Inside: the modelled table and chairs on the atlas, the chimney breast, the back door, the floor clean (no z-fighting) |
| `town.jpg` | A town on a highway: the corner store, a diner and a gas station along the road |
| `hit.jpg`, `hit-settled.jpg` | A 1 MJ blast on the ranch's front wall: the wall and the roof over it holed, its brick piers standing, the rest of the house whole |

The log: `small buildings: 24 kits, 17 furniture items, 62 surfaces`; `1696 lots spawned` (suburb 1,570, town 112,
roadside 14), spawning in one frame of 7.4 ms on this machine (a measurement of the spawn step alone, not a frame time).

## What was found and fixed on the way

- **A partition met the front door in the middle of its opening**: the ranch's front door opened onto the end of the
  wall between the kitchen and a bedroom. Partitions now stop 1.3 m short of a wall with a door near them (a hall).
- **The bungalow's porch steps were off to one side of its door**, so the rail stood in the way in. A porch's steps
  now line up with the front door.
- **The upper floors had no stairwell**: the colonial's stair climbed into the ceiling. The storey above a stair is
  cut over its run (test `the_colonials_stairs_reach_the_upper_floor`).
- **Ceiling lamps and the beach cottage's umbrella stood on nothing** by the kit's support links (made from below and
  beside), so the first hit on a house would have dropped them all. A piece a whole building leaves unheld is now
  held by what it touches, or the nearest piece (test `a_whole_building_stands_entire`).
- **The floor finish shared the foundation's top plane** (stripes of floor and concrete on every ground floor, in the
  mockup too). The finish now stands 12 mm proud.
- **Neighbouring lots at different heights tilted each other's ground** (one terrain cell is 8 m, and a lattice
  corner between two pads took the nearer one's height). Lots within two cells of each other now stand at the same
  height or are not both built (test `every_lot_is_flat`).
- **Towns and roadside stops could not fit**: the highways are 2.3-5.3 km long, edge to edge, so no point was 2.5 km
  from a city; and the turned footprints were checked against the roads by their axis-aligned box. Towns now keep
  900 m from a city, footprints are checked by their real corners, and a roadside stop tries several places either
  side of its mark.

## Tests

`cargo test -p cb-core --release`: 809 passed (and the small-buildings tests added since: kits load with their
furniture, a pilot walks through the front door of a ranch, a turned ranch, a bungalow and a corner store into the
room, a shut door is a wall, the colonial's stair reaches the upper floor, standing and collapse, a light blast holes
a house and no more, the island's lots: count by zone, clear of water, roads and each other, flat, the same twice).

## Not yet

- Survey S4-S6 are open (montane houses, how near the modelled furniture is drawn, suburb streets).
- The kit's surfaces still draw in their stand-in colours (Material Maker sets, task 1.1); the windows read pale.
- The mech's view of the blocks was not shot (the instrument stands the pilot on foot); a playtest from the cockpit
  is the check.
- Signs are untextured; lamps are not lit at night.
- Frame time of a suburb and a town: on the owner's machine.
