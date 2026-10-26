# Proposal: lod-crossfade

## Why

The owner (2026-10-26): "make sure to use cross fade like in Pale Blue dot for LOD and objects fading out, same with
trees". Today most things in the world pop: a tree tile switches from its far trees to its full ones in one frame at
600 m and vanishes at its draw edge, a building's far look is swapped for its near detail in one frame, cars and
people appear and disappear at the edge of their range, and traffic lights cut off at 700 m. Only the aircraft
(`air_traffic`, three levels) and the fortress's detail already cross-fade.

Pale Blue Dot's method (`Pale-Blue-Dot` at `2cf5ae1`, changes `distance-lod-fade` and `detail-fade`, after keijiro's
CrossFadingLod and Unity's LOD Group "Cross Fade"): across a ring of distance both levels are drawn, each through a
screen-door dither mask, the two masks complementary so every pixel shows exactly one level; the last level dithers
out to nothing over the last stretch of its range. Bevy 0.20 has the same thing built in: `VisibilityRange` with a
start and end margin makes `StandardMaterial` (and any material on Bevy's PBR fragment) dither through
`visibility_range_dither`, with opposite masks for the level fading in and the one fading out, in the main, prepass
and shadow passes.

## What changes

1. **One rule for fading** (CLAUDE.md 6.1): a `lod_fade` helper in `cb-app` that builds a `VisibilityRange` from a
   level's near and far distances and the fade width in data, used by every system below. Aircraft and the fortress
   move onto it.
2. **Trees**: a tile's far and full versions both exist across a 60 m ring at 600 m, the far fading in as the full
   fades out, and every tile dithers out over the last 150 m of its 2 km range instead of the abrupt edge.
3. **Buildings**: the far look and the near detail overlap across a ring, dithered, instead of the far look hidden
   the frame the detail appears.
4. **Cars and people**: they fade in after they appear and fade out before they are dropped (cars over the last
   100 m to 1 km, people over the last 50 m to their keep radius). A car spawned in view (behind cover past 400 m)
   still fades in.
5. **Traffic lights** dither out over their last 100 m to 700 m.
6. **Custom shaders** that draw a faded thing call `visibility_range_dither` themselves (Bevy only does it for its
   own PBR fragment): checked for each, listed in `design.md`.

## Not in this change

The ground's LOD nodes (a quadtree whose levels share edges: cross-fading them is Pale Blue Dot's `distance-lod-fade`
for its terrain, a larger change of its own), grass levels (one instanced draw a level), and the sky flicker and cloud
fog the owner raised the same day (another session).
