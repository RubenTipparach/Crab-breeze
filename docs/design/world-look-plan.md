# Making the game's island look like the mockup

What is left to do, in order, so the island in the game looks like the island mockup
(`docs/mockups/island.html`, published at https://claude.ai/artifact/NyHSzekHMnyspZyUrP5rTx):
its clouds and weather, storms with rain, varied ground, cliffs, water, trees and rocks. Written
2026-10-25, after PR #16 (crater, rivers, lakes, waterfalls, biomes) and PR #29 (curved roads)
merged into main.

Every step below is already designed in an OpenSpec change, or says which change it needs. Each
ends in something you can check: headless stills from a cloud session, then a playtest on your
machine.

## Where things stand

| In the mockup | In the game today | Change that brings it |
| --- | --- | --- |
| Billboard clouds in weather systems: a cold front, storm cells with anvils, a stratocumulus deck, cloud streets, altocumulus, cirrus, caps over peaks | Lumpy lit blobs, evenly scattered, 35 % cover, one kind | `cloud-weather` (designed) |
| Rain shafts under storms, the ground and sea darkened under them | Nothing | `cloud-weather` 2.2 |
| Rain on the glass when flying through rain | The drop shader is in the game, fed a fixed 0 | `cloud-rain` |
| Scattering sky, sun disc, aerial haze | Built (`sky-atmosphere`, Bevy's `Atmosphere::earth`) | Done; to retune once clouds change |
| Biomes blended by moisture, height and slope | Built in the core (`world::biome`); drawn as flat vertex colours | `terrain-materials` |
| Textured ground: grass, sand, rock, snow, mud, each with its own patchy look, crossfading with distance | Vertex colours, one grass texture | `terrain-materials` |
| Cliff faces with shape | A 60 m crater wall falls inside one or two 8 m cells: stretched triangles | `terrain-features` steps 1 and 2 |
| Canyons, mesas, dunes in the desert; reefs, sea cliffs, islets; swamps; jungle | Not in the game's generator | `island-biomes` 1.2, adapted to the crater |
| Water with depth colour, glitter, reflected sky | The water material (`water-rendering`), flat-looking | `water-look` |
| Waterfalls | Built: a sheet on gravity's arc, foam where it lands | Spray and mist to add |
| Trees by biome | Pines, oaks and birches everywhere the forest mask says, thinned by biome, drawn to 2 km | `vegetation` follow-up |
| Rock stacks, arches, boulders | None | `rock-formations` |
| Suburbs round cities | None | `island-biomes` (I9, I11) |

## Step 0. One island, not two

**The problem.** The mockup draws its island from its own JavaScript generator
(`docs/mockups/island_gen.js`). The game now generates a different island: a crater of mountains in
the middle, not the mockup's north-south range; rivers from a drainage model, not traced
downhill; curved highways. So "look like the mockup" today compares two different islands, and
every rule (heights, biomes, rivers, weather) exists twice. CLAUDE.md section 6.1 forbids that.

**What to do.**
1. `cb-tools export-island <seed> <dir>`: write the game's island as files the mockup can load:
   a height grid (16-bit PNG, 40 m a sample, 1024 x 1024), the biome weights (two RGBA PNGs at
   the same size), the rivers, lakes, waterfalls, highways and cities as JSON, and (after
   step 2) the weather's puffs and shafts as JSON.
2. The mockup loads those files instead of calling `island_gen.js`, and keeps only its drawing:
   the ground shader, the billboard clouds, the water. `island_gen.js` is retired.
3. The mockup's camera views (overview, crater, canyon, desert, storm, coast and so on) get the
   same names and positions as a list the game's stills capture reads (step 1), so every view can
   be shot in both and laid side by side.

**Done when** the mockup shows the game's crater island and the two match from the overview.

**Cost.** About a day of work. The export is about 10 MB, inlined into the mockup page.

## Step 1. Side-by-side stills, the measuring stick

**What to do.** Add a `--views` mode to the game's stills capture (`crates/cb-app/src/capture.rs`,
already used for the cloud stills): it puts the camera at each named view from step 0, waits for
`GAME_READY` and the ground to load around it, and saves one still each. A script stitches each
game still beside the mockup's shot of the same view into `docs/screenshots/island-parity/`.

**Done when** a contact sheet shows mockup and game for every view. Run it after every step
below; this is how each one is judged.

**Note.** Cloud sessions render these through lavapipe (software): good for looks, never for
frame time.

## Step 2. Clouds as weather (`cloud-weather`)

The biggest visible difference. The design is written (`openspec/changes/cloud-weather/design.md`);
what follows is the work, with what changes for the crater.

**What you will see.** From the ground and the cockpit:
- A cold front lying off the west coast, south-west to north-east: a long grey band, darker along
  its middle, with towers and rain shafts along it, and the sea darkened under it.
- Three storm cells: tall towers up to 6.5-10.5 km with flat anvils blown downwind, dark bases,
  grey rain hanging under them.
- Clear sky over the sea to the south-east, under a high.
- Cumulus over the land by how wet it is: small flat ones over the desert edge, towering ones over
  the wet west.
- Clouds piling against the crater rim's windward (west) side, and caps over its highest peaks.
- Cloud streets in rows over the southern sea, altocumulus high to the south-west, and cirrus
  streaks at 8-9 km.
- No square edge: the clouds run out past the map over the sea.

**What to do.**
1. **Core: `cb_core::world::weather`** (task 1.1). The systems as seeded rules that produce puffs,
   each with a kind, size, darkness and opacity, and the rain shafts and darkened cells. Tuning goes
   in `data/world/world.ron` under `weather:`. Port the mockup's numbers (design section 2), with
   two changes for the crater:
   - Orographic cloud sits on the rim's western arc, 600-3,200 m out from the crest, on ground
     above 500 m.
   - Caps go over rim peaks above about 1,400 m (the crater's peaks reach about 1.7 km, the old
     range's 1.9 km).

   Tests: counts and heights per system, every puff inside its layer, the same seed gives the
   same sky.
2. **`Clouds::density_at` reads every kind** (task 1.2), so flying into the deck or a tower fogs
   the view, and it reports rain under a shaft. Test: inside a shaft below its top is raining,
   outside is not.
3. **Draw billboards** (task 2.1). One instanced quad per puff in a single draw:
   - `cloud.wesl` reworked for a sprite: a sphere normal for the lighting, an eroded edge.
   - Sorted back to front within the 9 km draw radius.
   - Faded where it meets the ground, using the depth prepass.
   - The blob tiles from `cloud-puffs` go.
4. **Rain shafts and cirrus** (task 2.2). Three crossed billboards per shaft with streaks
   scrolling down, and flat quads for cirrus. The storm darkening goes in the ground, water and
   building shaders: one loop over the darkened cells, sun times `1 - 0.6` at the centre, its edge
   wandering.
5. **Rain on the glass** (task 2.3, `cloud-rain`). The existing drop shader's `lens_rain` is fed
   from `density_at`'s rain while the cockpit is under a shaft or in cloud.

**Cost** (design section 3): 3 draws. About 0.1 ms on an RTX 3070 in fair weather, about 0.5 ms
looking through a storm, 0.2 ms of CPU sorting. To be measured on your machine with
`CB_NO_CLOUDS`.

**Open question W25** (in the survey): storm towers as billboards, or a cheap raymarch of the
towers only if they look flat up close. Recommendation: billboards first, judged in a playtest.

## Step 3. Ground materials (`terrain-materials`)

**What you will see.**
- Real surfaces instead of flat colours: grass in clumps, dry savanna grass, sand with ripples,
  red desert rock, scree, snow with wind crust, river mud, beach sand.
- Steep slopes turn to rock, strata bands on canyon and crater walls.
- Patchy up close, smooth from 2 km up, never a visible repeated tile.

**What to do.**
1. Material Maker graphs for about twenty layers (two or three per biome: flat, slope and
   cliff) plus a strata strip, rendered headless in a cloud session
   (`material-maker-headless`), 1024 px PBR sets (base colour, normal, ORM). Contact sheet looked
   at. CC0 sets (ambientCG, Poly Haven) where you ask for them, as for the grass (CLAUDE.md
   section 13).
2. A layer table in `data/materials/terrain.ron`: each biome names its flat, slope and cliff
   layers. Validated on load.
3. The ground shader takes the core's biome weights per vertex: the two strongest biomes and
   their blend, packed into the vertex colour the ground already has.
4. In the shader:
   - Blend the two biomes, each picking a layer by slope.
   - A height blend at the borders, so sand fills between stones rather than fading over them.
   - Biplanar mapping on steep ground.
   - A macro variation map so a field is not one shade.
   - Beyond 3 km, one low-resolution map of the whole island instead of the layers.
5. The vertex colours stay as the fallback (`CB_NO_TERRAIN_MATERIALS=1`) for an A/B.

**Cost.** About 20 x 1024 px PBR sets, roughly 160 MB of texture memory with mips (to be
measured). About 12 texture reads a ground pixel up close, fewer far away.

## Step 4. Cliffs with shape (`terrain-features`)

**What you will see.** The crater wall and the cliff bands as rock faces with ledges and strata,
not stretched triangles. Waterfalls fall down a real face.

**What to do.**
1. Biplanar mapping and the rock material on every cell steeper than 40 degrees (step 3's shader
   does most of this).
2. Steep-cell refinement in the chunk mesher. A cell steeper than 40 degrees is split into 2 m
   cells, with the rock's noise pushing the face in and out along its normal, so the wall gets
   ledges. Neighbouring edges must match, so no cracks.
3. Later, only if overhangs and caves are wanted: surface-nets volumes (`terrain-features`
   step 3).

**Cost.** Steep cells are under 2 % of the island. Refining them at 4 x 4 adds about 30,000
triangles in view near the crater wall.

## Step 5. Water that looks wet (`water-look`)

**What you will see.**
- Sea and lakes with layered small waves.
- The sky and clouds reflected, with glitter toward the sun.
- Shallow water blue-green over sand, deep water dark.
- Rivers moving, and murky green water in swamps.

**What to do.** The `water-look` tasks in order:
1. A baked wave atlas (JONSWAP spectrum, 64 frames).
2. Three wave scales filtered by distance.
3. The sky cube refreshed from the eye, one face a frame.
4. Screen-space reflection with the cube as fallback.
5. GGX glitter over the sun's disc.
6. Scatter.

The lakes and rivers already use the one water material, so they get all of it.

**Waterfalls, added here.**
- A spray particle batch at each foot: one `GPUParticles3D` for every fall, fed a points texture,
  CLAUDE.md batching rule.
- A mist billboard that drifts downwind.
- Foam scrolling down the sheet.

## Step 6. Trees by biome (`vegetation` follow-up)

**What you will see.**
- Dense tall jungle in the wet west.
- Pines on the crater's flanks.
- Broadleaf woods in the temperate land.
- Scattered acacia-like trees on the savanna, scrub and a few dead trees in the desert, palms on
  beaches.
- Woods visible far beyond 2 km.

**What to do.**
1. Species per biome in data (`trees.species` gains a `biomes` list). New species are built in
   Blender with LODs (`blender-hard-surface` for the trunk kit, cards for crowns).
2. GPU-driven trees for distance: the forest mask and biome rules evaluated in a compute shader
   per tile, drawn instanced as impostors from 2 km to 8 km. Only the trees near bodies stay
   entities you can knock down.

**Cost.** Impostors: one draw per species. Hundreds of thousands of quads past 2 km, to be
measured.

## Step 7. Shapes the mockup has and the game does not (`island-biomes` 1.2, `rock-formations`)

Decide first which of these still fit the crater island. That is a survey question, with a
recommendation per item:

| Feature | Recommendation |
| --- | --- |
| Desert plateau with canyons, mesas and dunes (east) | Yes: the desert is already east; the plateau sits east of the crater's outer cliff band |
| North-east sea cliffs | Yes |
| Beach and reef along wide shelves | Yes |
| Archipelago of islets (south-west) | Yes |
| Swamp flats at river mouths in the west | Yes |
| Jungle | Comes from the biome and trees, step 6 |

Then the rock kit:
- Sea stacks, arches, hoodoos and boulder fields from a Blender generator with LODs and PBR.
- Placed per tile by biome, instanced, with colliders near bodies.

## Step 8. Cities and roads

- Suburbs: houses on a grid round each city, from the mockup's proposal (I9, I11).
- Road junctions outside cities: open question R1 in `curved-roads`.

## Order and why

1. Step 0 and step 1 first. Without them every later step is judged by eye against a different
   island.
2. Step 2, clouds and weather: the largest difference, and it changes the light on everything
   else (storm darkening).
3. Step 3, ground materials: the second largest. Every view is mostly ground.
4. Step 4, cliffs: depends on step 3's rock layer.
5. Step 5, water: independent. It can run in parallel with steps 3 and 4 in another session.
6. Step 6, trees, then step 7, shapes and rocks, then step 8.

Each step lands as its own PR from its own OpenSpec change. The change gets a write-up update
first where this plan changed it (the crater adaptation in step 2, the feature list in step 7).
Each PR has:
- the side-by-side sheet from step 1;
- tests for the core rules;
- a note that frame time is measured on your machine, not in the cloud.

A step is done when a playtest on your machine shows it (CLAUDE.md section 9).

## What I need from you

- **Survey answers:**
  - W25: storm towers, billboards or raymarch.
  - Step 7: which mockup features to keep on the crater island.
  - A1-A4: the airfields.

  A blank answer accepts the recommendation.
- **A go per step:** each one is a separate request to build (CLAUDE.md section 4).
