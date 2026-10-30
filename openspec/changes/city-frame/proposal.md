# Proposal: city-frame

## Why

The owner (2026-10-28): "Optimize city, goal is to get to90 fps with no dips", "our target is constant 60fps through
the whole time". On the RTX 3070, standing in the capital's densest blocks in the cockpit (`city-stand`), the game runs
at 5 to 20 fps: the GPU takes 50 to 190 ms a frame (`docs/benchmarks/2026-10-28-city/`). Switching parts off one at a
time, same spot, same view:

| Switched off | GPU ms a frame | Shadow pass ms |
| --- | ---: | ---: |
| nothing | 191 | 109 |
| grass | 163 | 73 |
| trees | 171 | 100 |
| buildings | 6 | 0.6 |
| the sun's shadows | 39 | 0 |

So the buildings near the pilot are almost the whole cost, most of it in the three shadow cascades (to 700 m) that draw
them again, and grass to 300 m casting shadows is the next. Every building within 250 m is drawn with its full
interior (`rooms_radius_m`: one building's rooms mesh was 7.3 MB) and its detailed skin (`dress_radius_m`). Cars are
1 100 in the capital, 16 meshes each, every one a shadow caster.

## What changes

1. **Grass**: only the nearest level (within 25 m) casts shadows (`grass.ron` `shadow_lods: 1`, new; 3 is today's).
2. **Cars**: each part gets a draw range, dithered out (`traffic.ron` `detail`: the cab's insides to 30 m, lamps and
   bumpers to 200 m, wheels to 400 m), and only the body and cabin cast shadows.
3. **Building detail** nearer: `dress_radius_m` 250 to 150 m, measured; interiors stay at the owner's 250 m (design F2).
4. **The sun's cascades**: `sun_shade.live_far_m` chosen by measurement; past it the baked shade carries the shadow.
5. One helper for every draw range (`lod_fade`, from `lod-crossfade`).

Each measured old against new in release builds, `city-stand` and `city-cruise` on the 3070.
