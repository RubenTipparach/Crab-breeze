# Design: lod-crossfade

## 1. Bevy's dithered visibility range

`bevy::camera::visibility::VisibilityRange { start_margin, end_margin, use_aabb }`: an entity is fully drawn between
`start_margin.end` and `end_margin.start` from the camera, dithers in across `start_margin` and out across
`end_margin`, and is culled outside. The mask is a 4 x 4 ordered pattern fixed to the screen; a level fading in uses
the complement of the one fading out, so two levels given touching margins (`end_margin` of the near one equal to
`start_margin` of the far one) share every pixel of the ring between them, each pixel showing one. It is applied in
the forward, prepass and shadow passes by `pbr_functions::visibility_range_dither` when the mesh has a range
(`VISIBILITY_RANGE_DITHER`). Pale Blue Dot's own shader (`bayer4`, `planet_surface.wgsl`) is the same pattern, written
by hand because its planet is not drawn with Bevy's PBR.

Cost: inside a ring both levels are drawn, half their pixels each discarded. With rings 10 % of a level's span the
extra draws are the meshes in the rings only. `discard` turns off early depth for those draws; measured on the 3070
when the owner frees the desktop, not claimed.

## 2. One helper

`cb-app::lod_fade`:

```rust
/// A level drawn from `near_m` to `far_m`, fading in over `fade_in_m` before `near_m` and out over `fade_out_m`
/// before `far_m` (a first level has no fade in, a last level fades to nothing).
pub fn range(near_m: f32, far_m: f32, fade_in_m: f32, fade_out_m: f32) -> VisibilityRange
/// The ranges of consecutive levels split at `splits_m`, each pair cross-fading over `ring_m`, the last fading out
/// over `out_m` before `far_m`.
pub fn levels(splits_m: &[f32], far_m: f32, ring_m: f32, out_m: f32) -> Vec<VisibilityRange>
```

Tested headless: consecutive levels' margins touch; a range is never inverted; zero widths give Bevy's abrupt range.
Fade widths live in data with the ranges they belong to (`fade_m`, CLAUDE.md 6.4).

## 3. Each thing

| Thing | Today | With this change | Material |
| --- | --- | --- | --- |
| Tree tiles, full and far (`world.rs`, `TREE_FAR_M` 600 m, slack 60 m) | the tile's mesh rebuilt as the other version at the line: a pop | both versions kept across the 60 m slack, full ending and far starting there; the old one removed once the eye is past the ring | `StandardMaterial`: dithered by Bevy |
| Tree tiles' edge (2 km, `VisibilityRange::abrupt`) | cut | out over the last 150 m | same |
| Buildings' far look and near detail (`buildings.rs`) | far hidden the frame the detail is spawned | the far look keeps an end margin and the detail parts a start margin over a 40 m ring; hidden only past it | far: `StandardMaterial`; near: `ExtendedMaterial` `ArrayState` / `ElementState`, whose fragment shader must call `visibility_range_dither` (checked in task 3.2) |
| Cars (`traffic.rs`, 1 to 1.2 km) | appear and vanish | fade in over 50 m after appearing; out over the last 100 m before 1.2 km, dropped past it | glb `StandardMaterial` |
| People (`civilians.rs`, 400 m spawn, 600 m keep) | appear and vanish | out over the last 50 m before 600 m; in over 30 m | glb, skinned: `StandardMaterial` |
| Traffic lights (700 m, abrupt) | cut | out over the last 100 m | `StandardMaterial` |
| Aircraft (three levels, 10 % rings) | already cross-fades | onto `lod_fade::levels`, same numbers | |
| The fortress's detail (out over 100 m) | already fades | onto `lod_fade::range`, same numbers | |

A thing that appears inside its fade-in ring (a car spawned behind cover at 400 m) needs a range measured from where
it is, so fading in is by distance only where the eye approaches it; a car spawned in place fades in over 0.5 s by a
per-entity start margin set from its current distance and moved out to 0 over that time. To be confirmed in task 2.3
that Bevy re-reads a changed `VisibilityRange` every frame; otherwise a car spawns only outside its fade-in ring.

## 4. Order

1. The helper and its tests; aircraft and the fortress onto it (no visible change).
2. Trees (the owner named them), then cars, people and traffic lights (data and one range each).
3. Buildings, after checking their extended materials dither.
4. Measured on the 3070, release build, `streaming-route` before and after interleaved, and a look at the fade rings
   in the game.

## Open questions

| Id | Question | Recommendation (built while open) |
| --- | --- | --- |
| X1 | Fade widths: trees 60 m ring and 150 m out, buildings 40 m, cars 100 m out, people 50 m out? | Yes; tuned on sight. Provisional. |
| X2 | The ground's quadtree levels cross-faded too, as Pale Blue Dot's terrain? | Its own change after this one. Provisional. |
