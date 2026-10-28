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
6. **Interiors stay at the owner's 250 m** ("I would prefer 250 m for furniture", 2026-10-24): 80 m was measured only
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
