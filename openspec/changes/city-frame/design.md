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

## 5. Destruction, with the card's memory free (2026-10-28)

With the desktop's other programs closed (the card 0.8 GB used before the game), release build, 4K and 2K textures
interleaved two runs each: standing in the capital, median 13.6 ms, 1 to 4 % of frames over 16.7 ms, whichever the
textures; bringing down 15 buildings there (`city-destruction`), median 19.5 to 21.8 ms and 81 to 93 % of frames over
16.7 ms while shooting, worst 92 to 103 ms, and 16.5 to 18.5 ms while the debris settles; ten houses in the suburb
(`suburb-house`), 17.4 to 17.7 ms and 66 to 68 % over while shooting. Texture size moved none of it beyond the spread.

The first time a building is hit, `buildings::redraw_buildings` swaps its whole-building box for its frame and, if it
is dressed, re-meshes its skin with the frame's slots: `frame_meshes` and `dressing_meshes`, on the main thread, in that
frame (10 to 25 ms each in the logs). Later hits only write the building's words in the shared state buffer.

**Measured, and not the cause:** the probe on `city-destruction` (bot fixed) puts `redraw_buildings` under 2 ms in every
slow frame. The long frames (100 to 600 ms) are late shader pipelines: 107 compiled after `GAME_READY`, each frame that
long coming right after one in the log. `perf_gate` now logs each late pipeline's shader defs: standard PBR in six
vertex layouts (normals with or without UVs, tangents and colours) and three blends, seen by the eye, the shadow
cascades and the cockpit cube's faces, and the building frame and glass shaders. **Fixed:** `shader_warmup` lays a
zoo of those in front of the eye behind the loading screen (a box per layout and blend, a building piece in skin and
in glass), so they compile before `GAME_READY`; measured old against new (`docs/benchmarks/2026-10-28-city/`).

**Measured, no effect:** the warm-up. `city-destruction`, release, old against new interleaved two runs each: late
pipelines 27 and 39 with it, 52 and 23 without (the driver's own shader cache makes the count vary); the median while
shooting 18.9 to 19.3 ms against 19.3 to 19.5, frames over 50 ms 15 and 19 against 16 and 19. Kept (it moves compiles
into loading and costs nothing in play), claimed as nothing.

**What the long frames are:** the probe's frames over 50 ms while shooting name `allocate_and_free_meshes` (the render
world copying new and changed meshes into its buffers) in 30 of 51, 26 ms on average in them: a hit building's new
frame, and if dressed its skin, glass and rooms re-meshed with the frame's slots, several MB at once, and the debris.
**Tried and undone:** Bevy's mesh slabs (a mesh of 1 MiB or more in a buffer of its own, slabs to 64 MiB, growing 2 x),
the same build with and without: standing 16.9 ms against 13.8, shooting 23.1 ms against 19.9, frames over 50 ms 21
against 16. Meshes in buffers of their own cannot be drawn together, so it cost every frame and did not touch the
spikes. **Next:** upload less on a building's first hit (its rooms are re-meshed only to carry the frame's slots).

**Not done (the async frame build below):** the two meshes built on the async compute pool
from copies of the building's frame, site and dressing inputs; the whole-building box stays drawn until both are
done, then the swap happens in one frame as now. The state buffer is written at once either way, so damage is not
delayed, only its look on a building's first hit, by the few frames the build takes. Cost: one task a newly hit
building, the same work off the main thread.

## Open questions

| Id | Question | Recommendation (built while open) |
| --- | --- | --- |
| F1 | Detailed dressing to 150 m: is the far look past 150 m good enough? | Yes; looked at in the game when the owner can. Provisional. |
| F2 | Interiors (rooms and furniture) nearer than the owner's 250 m, if the release measurement shows they still cost the frame? | Ask the owner with the number; 250 m kept until then. |
