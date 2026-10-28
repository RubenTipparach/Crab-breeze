# Design: city-frame

## 1. The measurement

`city-stand` (board on a street of the capital, look round from the cockpit for 40 s), RTX 3070, i7-9700F, 1920 x
1080, vsync off, the probe build (`--features hitch_spans perf_probe`, about 3 ms a frame of CPU over a release build:
its GPU times are real, its frame times rank causes only). GPU time is the sum of every render pass's
`elapsed_gpu` in the `perf:` lines over the look; two runs a variant, old and new at both ends of the sitting.

| Variant | Runs | fps | GPU ms | Shadow pass ms | Meshes drawn |
| --- | --- | ---: | ---: | ---: | ---: |
| old (main `0f2e307`) | 2 | 3 to 4 | 230 to 312 | 127 to 171 | 14 651 to 18 756 |
| grass and car rules (this change's code) | 2 | 7 to 9 | 110 to 133 | 63 to 78 | 6 573 to 7 545 |
| + interiors to 80 m | 2 | 6 to 8 | 132 to 133 | 83 to 99 | 6 263 to 7 051 |
| + detailed dressing to 150 m | 2 | 42 to 63 | 7.6 to 24 | 2.2 to 11.8 | 6 181 to 6 298 |
| + the sun's cascades to 300 m | see section 3 | 65 | 9.1 | 2.3 | 5 925 |

Switching one part off at a time (one run each, main `0f2e307`; single runs of this scene spread by up to 2 x, so
only the large differences count): buildings off, GPU 6 ms; the sun's shadows off, 39 ms; grass off, 163 ms against
191; trees off, 171.

## 2. Why

- **Detailed dressing to 250 m was most of the GPU.** Within `dress_radius_m` a building is drawn with its skin,
  trims, parapets and roof props piece by piece (one building's skin mesh was 4.7 MB, about 100 000 vertices), and
  every piece casts the sun's shadow into three cascades. In the capital dozens of buildings stand between 150 and
  250 m, many of them towers; past `dress_radius_m` the same building is its massing and far facade, a few hundred
  triangles. At 150 m the GPU fell from 110 to 133 ms to 8 to 24 ms.
- **Interiors to 250 m** (`rooms_radius_m`) cost the main and prepass passes, not shadows (rooms cast none): 41 ms
  to 31 ms of those two at the old dressing radius. Through a window farther than 80 m a room is a few pixels.
- **Cars**: 1 100 in the capital, 16 meshes each, every one a shadow caster. With `CarDetail` a car is 2 casters and,
  past 400 m, 2 meshes.
- **Grass** to 300 m casting shadows: each tile drawn again into every cascade, about 30 ms of the shadow pass.
- **The cascades' reach** (`live_far_m` 700 m) costs little once the dressing is at 150 m (9.1 against 7.6 ms):
  kept at 700 m for the look.

## 3. The change

| Key | Was | Now |
| --- | ---: | ---: |
| `world.ron` `dressing.dress_radius_m` | 250 | 150 |
| `world.ron` `dressing.rooms_radius_m` | 250 | 250 (kept: the owner's own choice, 2026-10-24, "I would prefer 250 m for furniture"; 80 m was measured only together with dressing at 150 m, so its share is measured next and asked, not changed) |
| `grass.ron` `shadow_lods` (new) | 3 (every level) | 1 (within 25 m) |
| `traffic.ron` `detail` (new) | every part to the traffic's edge | cab insides 30 m, lamps and bumpers 200 m, wheels 400 m, out over the last 15 %; only body and cabin cast shadows |

The dressing and the far look still switch in one frame at 150 m; cross-fading them is `lod-crossfade` task 3.1.

Cost to the look: past 150 m a building shows its massing and its type's far facade (cornices, parapets and roof
props from its own dressing, `building-type-lods`) rather than every modelled piece; rooms stay drawn to 250 m.

## 4. What is left (next)

With the GPU at 8 to 24 ms the frame is the CPU's: in the probe build the render world takes 15 ms a frame
(`render_system` 5.5 ms encoding about 6 000 draws, submitting 3 ms, mesh uploads 7 ms on a fifth of frames with a
worst of 134 ms as buildings come inside 150 m and are dressed). A release build without the probe sets the real
figure; then the draws (cars at 2 meshes to 1 km are about 2 200) and the dressing's uploads are next.

## Open questions

| Id | Question | Recommendation (built while open) |
| --- | --- | --- |
| F1 | Detailed dressing to 150 m: is the far look past 150 m good enough? | Yes; looked at in the game when the owner can. Provisional. |
| F2 | Interiors (rooms and furniture) nearer than the owner's 250 m, if the release measurement shows they still cost the frame? | Ask the owner with the number; 250 m kept until then. |
