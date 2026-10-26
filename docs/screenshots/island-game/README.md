# The mockup's island in the game: lavapipe stills, 2026-10-25

The owner, 2026-10-25: "I don't like our crappy island ... I wanted this island ... can you update
our terrain and textures and everything to use the cool island with different biomes???", "Of
course we still want cool roads and rivers". The game now builds the island mockup's land
(`docs/mockups/island.html`, `island_gen.js`) from seed 42: `cb_core::world::island` is its
generator ported line for line, checked against it run in Node at 30 points (within 5 cm). The
game's own drainage rivers, lakes and waterfalls, cities and curved highways are laid on it, the
mockup's ten biomes colour the ground (its surface layers per vertex), and the weather sits on it.

Headless stills from a cloud session: the release build, Mesa's lavapipe under Xvfb, 1920 x 1080,
reduced to 1280 px and 256 colours. **No frame times** (CLAUDE.md section 7). Shot with
`CB_STILLS_WEATHER=1 ... crab-breeze --stills <dir>`, whose views are the mockup's named views
worked out from its generator, so each still lies beside the mockup's shot of the same view
(`side-by-side.png`; the mockup's shots are `docs/screenshots/island/`).

| File | What it shows |
| --- | --- |
| `map_seed42.png` | `cb-tools map 42`: the game's island from above by biome (the mockup's biome colours), rivers, lakes, waterfalls (red), cities and highways |
| `side-by-side.png` | Game (left) and mockup (right): overview, range, desert, canyon, jungle, cliffs, islets |
| `range.png` | From 1.9 km west of the range: the eye is inside orographic cloud (density 0.6), so the range is fogged |
| `desert.png`, `canyon.png` | The plateau, its sand and the red sandstone strata of the canyon walls, as the mockup draws them |
| `jungle.png` | The wet west: forest, with cloud banked on the range's windward flank in front of the eye on the right |
| `storm.png` | Grassland and a river under the storm's towers, a waterfall coming off the range |
| `under-the-storm.png` | Under the storm: rain on the glass, its dark base, forest and a city |
| `islets.png`, `cliffs.png`, `coast.png`, `overview.png`, `weather.png`, `storm-shadow.png` | The archipelago, the north-east coast, the east coast, the island from 22 km up |

Seed 42 now: 13 cities (were 24 on the crater island), 3,609 buildings, 12 highways (41 km), 64
rivers (217 km), 29 lakes, 111 waterfalls, 498 km2 of land, highest ground 2,252 m; desert and
canyon 25.7 % of the land. The world builds in about 1.3 s in the test profile (was 0.24 s): the
mockup's land costs a few microseconds a sample.

## What is still wrong

- **Trees** grow only near the bodies (the game's rule), so from the air the jungle and the range's
  forests read as ground colour, not the mockup's dense canopy (`vegetation` follow-up, step 6).
- **The ground's layers are per vertex**: at 8 m a vertex, the mockup's fine detail (pebbles,
  dune ripples) is not there; that is the per-pixel shader of `terrain-materials`.
- **Rocks, stacks and arches** of the mockup are not in the game (`rock-formations`).
- **Swamps** are a biome at the western river mouths, not yet flattened into wet flats.
- **111 waterfalls**: the range's terraced bands make the game's rivers fall often. The mockup has
  none (owner: "No waterfalls"), the crater island had them by design. A question for the owner.
- The sea is a flat blue (`water-look`), and past the world's edge a pale square shows from high up.
