# Design: cloud-puffs

## 1. Options weighed against "good looking, correct lighting, cheap"

| Option | Looks | Lighting | Cost | Taken |
| --- | --- | --- | --- | --- |
| Sprite stacks (the old way) | soft | baked, wrong toward the sun | heavy overdraw: 7 large blended quads a puff | no (owner turned it off) |
| Ray-marched volumes (Horizon's Nubis, trueSKY in Ace Combat 7) | best | correct by construction | 1-3 ms even at quarter resolution with reprojection; a custom render pass | no (budget) |
| Impostor atlases from Blender volumetrics (Harris and Lastra, GDC 2002) | good from a distance | baked per view angle | cheap, but a bake pipeline and flat when flown over | later, for far clouds |
| **Lumpy blob meshes with a cloud shader** | soft-edged cumulus | sun wrap, base shading, silver lining, sky ambient, live sun | one blended layer per blob, a few hundred thousand triangles | **yes** |

## 2. Mesh

- A puff is an icosphere at subdivision 4 (252 vertices, about 500 triangles).
- Vertex `d` (unit direction) moves to `centre + d * radii * (0.78 + 0.45 * lump(d, puff))`.
- `lump` is three octaves of a stable trigonometric hash (0..1), the same shape every time.
- Anything below `max(cloud base, centre - 0.75 * radius)` is lifted to it: a flat base.
- Smooth normals are recomputed from the pushed positions.
- A tile's puffs are merged into one mesh in world space at time zero, about 20 puffs and 10,000
  triangles per tile.

## 3. Shading (linear HDR, per pixel)

- `diffuse = clamp((n.l + wrap) / (1 + wrap))`, with `wrap` 0.6.
- `body = mix(1 - base_shade, 1, smoothstep(0, 0.8, h))`, where `h` is height through the layer
  and `base_shade` is 0.45.
- `silver = pow(max(v.l, 0), 6) * (1 - |n.v|)^2 * silver_strength`, with strength 1.6.
- `colour = sun * albedo / pi * (diffuse * body + silver) + ambient * albedo * mix(0.55, 1, h)`,
  with albedo 0.9.
- `alpha = smoothstep(0, edge_softness, |n.v|) * smoothstep(0.4 * near_fade, near_fade, distance)`,
  with softness 0.45 and near fade 160 m.
- Bevy's fog is applied last (aerial haze; inside a cloud, the cloud fog).

All six numbers live in `data/world/world.ron` under `clouds:` (`look_*`).

## 4. Cost (expected; measured with `CB_NO_CLOUDS` on the owner's hardware)

| Item | Value |
| --- | --- |
| Entities | about 30 tiles in reach (2 spawned a frame, nearest first) |
| Triangles | about 300,000 in reach, a third in view |
| Draws | one material, one batched draw per tile in view |
| Overdraw | about one blended layer per blob; two to three where blobs of a cluster overlap |
| CPU | 30 transform writes a frame (wind), one uniform write |

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| W8 | Clouds | scattered cumulus at 600-1100 m, about 35 % cover, fly-through fog | provisional (survey W8) |
| W17 | How clouds are made | lit blob meshes now; far impostors later if the haze needs more | replaced by `cloud-weather` (owner, 2026-10-24: "I prefer clouds to be billboards or a mix of some cheap raymarch") |
| I7 | Cloud shadows on the ground | a scrolling 2 km shadow texture, one sample a pixel | proposed (survey I7) |
