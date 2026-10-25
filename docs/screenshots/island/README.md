# Island mockup shots

Shots of `docs/mockups/island.html` (with `docs/mockups/island_gen.js`), taken 2026-10-24 in a cloud
session: headless Chromium with SwiftShader (software WebGL), 1600 x 900, the 3D stage stretched to
fill the window, each view loaded fresh from its `#view` hash and shot about 7 s after
`window.__islandReady`. The PNGs are reduced to 256 colours to keep them small. A software renderer
says nothing about frame time, so these shots carry no performance numbers.

`contact.png` has all twelve, four across.

## What was broken, and fixed before these shots

- **Nothing drew.** Every custom shader failed to compile: the log-depth chunk calls
  `isPerspectiveMatrix`, which lives in three.js's `common` chunk, and no vertex shader included it.
  Each vertex shader now includes `<common>`.
- **The loading card never went away.** `.loading` set `display: grid`, which beat the `hidden`
  attribute. `.loading[hidden]` now hides it.
- **Mojibake** (`Â·`, `Â°`, `kmÂ²`) when the page is served as a file: it had no charset. It now
  declares `utf-8`.
- **The sea ended in mid-view** from the overview, a grey band past its edge. The sea plane is now six
  worlds wide (246 km), the far plane 200 km and the sky dome 150 km.
- **Canyon** looked down onto the canyon floor from above, so the cut read as a grey hump. It now
  stands on the rim, 380 m to one side, looking along the cut.
- **Desert** picked its spot with `Math.random`, landed on the escarpment and changed on every load.
  It now searches a fixed 400 m grid for the thickest dune field and frames it from 220 m up.
- **Archipelago** stood at 900 m, inside the cloud layer, behind a cloud. It now stands at 420 m.
- The jungle search is seeded too, so every view is the same on every load.

## Each shot

| Shot | What it shows | What is wrong or stand-in |
|---|---|---|
| `overview.png` | The whole island from 22 km: the range, the eastern desert with its canyon network, the green west, the archipelago to the south-west, cumulus all round | The western jungle/temperate border is a long straight line (the jungle's `south` term is a smoothstep in z). The canyons read as raised red ropes at this distance: they are 125 m deep trenches only one or two 80 m cells wide. |
| `range.png` | The snow range through the clouds from 1.9 km | The terraces (`terrace(range, 210, 0.25)`) show as flat green ledges round grey peaks. Waterfalls are white strips. |
| `canyon.png` | From the rim: striped strata walls, gravel floor, hoodoos | The walls are 80 m triangles, so the strata zig-zag; the game's quadtree is to draw to 1 m. |
| `desert.png` | The dune field from 220 m, canyons beyond | **Finding:** the rain-shadow plateau stands at 500-650 m, so the biome rules give it montane forest (pines on the sand; the HUD says "below montane forest") and the cloud layer (600-1100 m) sits on it: a cloud fills the right third. The dunes (6-22 m high) barely show on the 80 m grid. The design's plateau height or the montane band needs a look. |
| `jungle.png` | Jungle canopy from 70 m, the range behind | Trees are bare green balls every 70 m, no trunks; the game grows dense jungle only near the bodies. |
| `swamp.png` | Swamp flats at a western river mouth, a murky pool, cypress cones | Flat colour; the pools are dark patches. Reads as a stand-in. |
| `falls.png` | A waterfall down a terraced cliff in the range, and the next one below | No river water on the tread between the two falls; the spray puff is small. |
| `cliffs.png` | The north-east sea cliffs from the sea, with sea stacks | The cliffs are a saw-tooth of 80 m triangles. Stacks are plain cylinder block-outs. |
| `islets.png` | The archipelago from 420 m | The sun glint on the sea is large white blotches, not a glitter path. |
| `beach.png` | The south beach from the water, the 24 m mech and the pilot on the sand | Bushes are low-poly balls. |
| `scale.png` | The 24 m mech block-out and the 1.8 m pilot, close | The mech is a box figure, not the digitigrade design. The straight waterline at the right is the 80 m grid crossing sea level on the flat beach, not a bug. |
| `overview_biome.png` | The overview with Biome map on | The biome colours work. The desert's red canyon class shows the network clearly. |

## The clouds in these shots

The clouds are the `cloud-puffs` design as built: lumpy icosphere blobs, lit by the sun, darker at the
base. In close views their silhouettes are visibly faceted (icosphere detail 2) and some edges show a
faint bright rim. They are lit, not hard grey polygons, but they are bright and flat under ACES.

After these shots the owner said (2026-10-24): "I prefer clouds to be billboards or a mix of some cheap
raymarch". That is recorded against W17 in `openspec/changes/cloud-puffs/design.md`; the blobs stay until
that design is revised.
