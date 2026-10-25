# Proposal: island-biomes

## Why

Owner, 2026-10-24: "We need some biomes for our island, a big mountain range in the middle,
canyons, desert on the side, some beaches, rock formations, a bunch of interesting pbr materials
for the landscape, I want things to look realistic and varied as we fly around this island doing
missions"; "Need tall dense jungle biomes, big roaring waterfalls, and murky swamps"; "Also cliffs
and archipelago".

Today the island is a gentle highland: land 30 m up with up to 90 m of highland and 35 m hills,
151 m at its highest. It is one green-brown everywhere. Missions fly over the same ground the
whole way.

## What changes

The island's shape and its biomes, all from the seed, as pure functions of position (no stored
grid, as today):

- **A mountain range:** a ridged, eroded spine through the middle, 16 km long, running
  north-north-east. Peaks of 1,400-2,200 m reach through the cloud layer (600-1,100 m), with snow
  above 1,600 m. Passes at 500-700 m carry the highways across.
- **The desert:** the range's rain shadow on the east. Dunes, a 250-400 m plateau cut by a canyon
  system 150-250 m deep with layered strata walls, mesas and buttes left standing, and dry
  riverbeds and salt pans.
- **The jungle:** the wet west and south-west lowlands. Tall dense canopy, trees 30-45 m (a mech's
  height and more), rivers through it.
- **Swamps:** flat wetlands at the western river mouths. Murky, shallow water (0.5-4 m, waded:
  `skim-depth`), mangrove and cypress.
- **Rivers and lakes:** rivers from tarns on the range through valley lakes to the sea. No
  waterfalls (owner, 2026-10-25: "No waterfalls").
- **Temperate north:** forest and grassland plains, where most cities stand.
- **Coasts:**
  - beaches and dunes on the south and west;
  - sea cliffs of 60-150 m on the north-east, with stacks and arches;
  - an archipelago of 20-40 islets, 0.2-2 km across, off the south-west, inside the 41 km world.
- **Biome weights** at every point drive the ground materials (`terrain-materials`), the trees
  (`vegetation`), the rock formations (`rock-formations`), the water's murk and where cities may
  stand.

Cities, highways and rivers keep their rules but read the new ground: cities only on gentle ground
below 300 m. The capital moves off the range to the northern plain, so the start is no longer
within 2.5 km of the centre (survey I2).

## Presented by

The three.js mockup `docs/mockups/island.html`, an orbitable and flyable 41 km island at true
scale, before any terrain code (owner, 2026-10-24: "before you put any code into the game, I want
to make sure you are building it right via js mockup").

## Out of scope

Weather beyond clouds, seasons, animals, interiors of caves.
