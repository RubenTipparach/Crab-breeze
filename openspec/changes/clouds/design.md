# Design: clouds

## 1. Layer (cb-core::world::clouds)

Clusters by Poisson-disc on the world at `cloud_spacing_m` (default 700 m) with coverage noise
(`coverage` 0.4 default): each cluster 3-8 ellipsoids, 80-300 m across, flat bottoms at cloud
base. Pure; `cloud_density_at(p) -> f32` is the one query (fogging, shadows, radar later).

## 2. Drawing

- Puffs: each ellipsoid filled with 4-8 camera-facing quads with a soft round texture and a normal
  baked from a sphere, lit by the sun (wrap lighting) and the sky colour; depth-faded against the
  scene; drawn in one instanced transparent pass, sorted by cluster.
- Inside: `DistanceFog` on the camera is driven each frame from `cloud_density_at(eye)`: visibility
  from 3 km down to 40 m, colour toward lit white; a 50-quad wisp emitter around the camera
  streams past at the flight speed.
- Shadows: a 2 km tiling cloud shadow texture scrolled with the wind, multiplied into the sun in
  the terrain and building materials.

## 3. Cost (expected)

About 300 clusters x 30 quads = 9 k quads, 1 transparent draw, the overdraw measured from the
ground and from inside a cloud (the worst case). Fog: free. Shadow texture: one sample per pixel.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| W8 | Clouds | scattered cumulus at 600-1100 m, about 40 % cover, fly-through fog | provisional (survey W8) |
| T4 | Camera raindrops from Pale Blue Dot | port its lens-droplet effect when rain arrives; bring its source into reach | open (survey T4) |

## As built (2026-10-24)

- Puffs are 7 unlit soft billboards each (288 puffs for seed 42, about 2 000 quads); fly-through fog is the camera's distance fog driven by `density_at` at the eye. Cloud shadows and camera raindrops are still to do.
