# Design: ambient-occlusion

## 1. What it does

Bevy's SSAO (GTAO-style, `bevy_pbr::ssao`) samples the depth and normal prepasses around each pixel
and scales the indirect diffuse light by how open that point is. It never darkens direct sunlight,
so the effect shows in shade: inside a shelled floor, under slabs, where furniture meets the floor,
between rubble pieces. It requires a depth and a normal prepass (the eye camera already has the
depth prepass for the water) and no MSAA (the camera already uses FXAA).

## 2. Settings (`world.ron` `view`)

- `ssao_radius_m`: 1.5 m, about a desk's depth; Bevy's 0.73 m default suits rooms at human scale,
  and the pilot sees furniture from 10-20 m. 0 switches SSAO off.
- `ssao_quality`: `Medium` (Bevy's levels: Low, Medium, High, Ultra).

## 3. Cost

A normal prepass (one more geometry pass writing normals) plus the SSAO compute passes at
1920 x 1080. Measured, not guessed: the city combat benchmark with and without (`CB_NO_SSAO=1`),
interleaved, on the Mac; the RTX 3070 number comes from a desktop run.

## Open questions

| question | recommendation | status |
| --- | --- | --- |
| TAA with SSAO | FXAA first; add TAA only if the noise shows in a take | provisional |
| Radius | 1.5 m, judged on stills of a shelled floor | provisional |
