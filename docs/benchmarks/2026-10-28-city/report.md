# The city on the RTX 3070

The night of 2026-10-27 to 28 on the owner's desktop, on the owner's asks: "Optimize city, goal is to get to90 fps with
no dips", "our target is constant 60fps through the whole time", "develop an mcp server if you have to to get perf in
real time". The owner had gone to bed and said Claude may test freely.

## Findings

1. **On main (`0f2e307`) the capital ran at 3 to 5 fps.** Board on a street of the capital and look round from the
   cockpit (`city-stand`): median frame 201 to 245 ms, every frame over 16.7 ms, the GPU about 210 ms a frame, half of
   it the sun's three shadow cascades. Skating through it (`city-cruise`): median 201 to 227 ms.
2. **The buildings near the pilot were nearly all of it.** Switching parts off one at a time, same spot: buildings
   off, GPU 6 ms; the sun's shadows off, 39 ms; grass off, 163 ms; trees off, 171 ms; everything on, 191 ms. Within
   `dress_radius_m` (250 m) every building was drawn piece by piece (window frames, ledges, parapets, roof props:
   one tower's skin about 100 000 vertices), every piece casting shadows into three cascades.
3. **Fixed on `perf-probe` (`openspec/changes/city-frame`):** detailed dressing to 150 m (the GPU fell from 110 to
   190 ms to 8 to 24 ms in the probe build), grass casting shadows only within 25 m, cars' small parts drawn only
   near (cab insides 30 m, lamps and bumpers 200 m, wheels 400 m, dithered out) with only body and cabin casting,
   window glass casting none. Release build: `city-stand` median 27 to 42 ms (from 201 to 245), `city-cruise` 40 to
   72 ms (from 201 to 227). Five to nine times faster, and still not 60 fps.
4. **What is left is mostly video memory, not drawing.** The same scene with the same triangles in view (the probe's
   census: building skins 1.7 million, interiors 1.1 million) took the GPU 8 ms in one run and 66 and 107 ms in the
   next two, at full clock (1 950 MHz) and 100 % busy but drawing 92 W of 220. Windows' GPU counters show why: the
   game held 5.0 GB of dedicated video memory and had **870 MB in shared system memory**, read over PCIe. The card
   has 8 GB; the desktop's other programs (Chrome, Firefox, Discord, Steam, Parsec, BlueStacks and others) held 2.5 GB
   before the game started. When the game fits, the city is near 60 fps (GPU 8 ms, the CPU then the limit); when it
   spills, 10 to 20 fps.
5. **Where the video memory goes** (the probe's census, in the capital): images 2 240 MB, of which the Panorama
   cockpit's eight 4096 px maps are 512 MB (64 MB each, uncompressed, no mipmaps), two generated 4096 px images 171 MB,
   the building texture arrays 149 MB, the sky cube 24 MB; mesh data sent 1 421 MB since the start (some freed);
   plus the shadow maps, the screen's render targets and grass, which the census cannot see.
6. **A 100 ms stutter every 16 s, everywhere: the storm shade.** `clouds_render::shade_sun` rebuilt the sun's storm
   shade map on the main thread every 2 degrees of the sun's travel, 93 to 115 ms each time; it was the worst frame
   of the suburb run. Now built on a worker thread, the old shade kept until the new one is ready (`ac9153a`).
7. **Houses**: twenty brought down (`suburb-destruction`) had 600 debris bodies moving and physics 5 ms a step. The
   limits are now 200 moving and 800 kept within 250 m (was 600, 2 500, 400 m; the owner: "we need to reduce physics
   objects on houses"). Release, old against new interleaved: the worst frame while destroying 59 and 95 ms against
   138 and 144, while settling 49 and 91 against 123 and 106. The medians moved with the video-memory spill and the
   run's place in the sitting (each later run slower) more than with the change, so no median claim is made.
8. **The fixed step doubles when the frame is slow.** In the suburbs `step_traffic` averages 7.7 ms a frame (worst
   35 ms) for about 1 100 cars, physics 5 ms: when the GPU holds a frame to 30 ms the simulation runs two steps in it,
   and the CPU's share doubles. The Mac session is making far cars step less often (`li-traffic-lod`).
9. **Flying flat out across the map** (`fast-flight`, release, about 33 km in 10 min at 60 to 165 m/s): median 14.1
   ms, 33 % of frames under 60 fps. Over open country (the east, x 8 000 to 11 000 m, 300 to 700 m up) the median is
   11.7 to 12.8 ms with 0 to 6 % of frames over 16.7 ms: near the 90 fps line already. The slow stretches are high
   (900 to 1 500 m) with cities in view and low over the western cities: the shadow pass 45 to 57 ms with only 850
   to 1 500 meshes drawn, so many triangles cast, not many meshes.
10. **City destruction** (`city-destruction`, 15 buildings in the capital): 4 to 7 fps throughout, the shadow pass
   73 to 159 ms: the capital's spill as in `city-stand` slow runs, with the debris's meshes added.
11. **Interiors stay at the owner's 250 m** ("I would prefer 250 m for furniture", 2026-10-24): 80 m was measured only
   with the dressing change, so its share is a question for the owner (design F2), not a change.

## What to do next, by size

| Change | Video memory saved | Cost |
| --- | ---: | --- |
| Close the browsers and launchers while playing | about 2.5 GB | none; a workaround, not a fix |
| Cockpit maps compressed (BC7, a quarter of the size) | about 380 MB | an asset step (KTX2) |
| Cockpit maps at 2048 px | about 380 MB | the owner's 4096 px rule for custom textures |
| Every PBR set compressed (BC7) | most of the 2.2 GB of images | the same asset step for all |
| The generated 4096 px images found and halved | up to 128 MB | to be identified |

Then the CPU: in the probe build the render world takes about 15 ms a frame (encoding about 6 000 draws 5.6 ms, mesh
uploads 5.8 ms on a third of frames with a worst of 132 ms as buildings are dressed, submitting 3.3 ms) and the
fixed step 6 ms (traffic 4.2 ms for 1 100 cars). The cockpit's screens cube draws one face of the world out to 400 m
on most frames (`screens_cube_hz` 10, 60 faces a second) and re-filters every frame (2.4 ms of GPU).

## The tools made tonight (`perf-probe`, PR #94)

- `--features perf_probe`: the game answers `127.0.0.1:7878` (`/summary`, `/frames`, `/events`, `/census`): frame
  percentiles, the systems in the slow frames, game events (blasts, buildings falling, debris), entities, physics
  bodies, meshes, triangles by what draws them, image and mesh memory. `tools/perf/probe_mcp.py` serves it as MCP.
- Scripts: `city-stand`, `city-cruise`, `city-destruction`, `suburb-destruction`, `fast-flight`; the pilot's
  `FireAtNearest` beat and the `house` and `farthest-city` destinations.

## How this was measured

RTX 3070 (8 GB), i7-9700F, 32 GB, Windows 10, Vulkan, 1920 x 1080 window on a 60 Hz display, vsync off
(`CB_UNCAPPED=1 CB_NATIVE_1080=1`). Release builds for frame times; the probe build (`hitch_spans`, about 3 ms a frame
slower) for causes and GPU pass times, which are real in either. Old and new interleaved, two runs each; the first 3 s
in the mech dropped. No OBS, no build beside a run. The other desktop programs stayed open (they are the owner's), so
every run shares the card with them: that is finding 4. All tables: `runs.txt`; the census: `census-city-stand.json`.
