# Design: terrain-materials

## 1. Layers and rules (data/materials/terrain.ron)

| Layer | Rule (weights normalised per vertex) |
| --- | --- |
| grass | slope under 20 degrees, dry |
| dry grass | grass with height noise above 0.6 |
| dirt | slope 20-35 degrees, and paths |
| rock | slope over 35 degrees (triplanar) |
| mud | within 15 m of water, or wetness over 0.5 |
| gravel | riverbeds and highway shoulders |
| scorch | the deformation channel (craters, burns) |
| concrete | city ground |

Weights go in two RGBA vertex attributes (8 layers). Asphalt is the road mesh's own material.

## 2. Shader

A WGSL fragment extension of `StandardMaterial`: sample each layer whose weight is over 0.02
(at most 4 in practice), height-blend, triplanar for rock, detail and macro variation from two
noise octaves to break tiling at range. The texture set is one array texture per map (8 layers x
2048 px, BC7 on desktop, with mips).

## 3. Cost

Texture memory: 4 maps x 8 layers x 2048 px BC7 with mips = about 170 MB. If that is too much for
the budget, 1024 px halves it four times over (about 43 MB). Shader: about 4 layer samples x 4
maps, plus triplanar on slopes (3x the rock samples): measured in the cockpit.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| W5 | Terrain texture resolution | 2048 px tiles over 2 m, 1024 px fallback | provisional (survey W5) |
