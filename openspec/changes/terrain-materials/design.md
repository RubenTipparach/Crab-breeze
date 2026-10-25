# Design: terrain-materials

## 1. The layers (data/materials/terrain.ron)

Twenty layers in one array texture per map. Each is a Material Maker graph (section 10 of
CLAUDE.md), 2048 px over its tile size, with mips.

| # | Layer | Tile m | Look | Biomes |
| --- | --- | --- | --- | --- |
| 0 | grass | 2 | green turf, clover, bare patches | temperate |
| 1 | dry grass | 2 | straw, tufts, dust | savanna, temperate |
| 2 | forest floor | 2 | needles, leaf litter, roots | montane forest |
| 3 | jungle floor | 2 | wet broad leaves, moss, dark soil | jungle |
| 4 | mud | 2 | wet brown, puddles (low roughness in hollows) | rivers, swamp |
| 5 | swamp muck | 3 | black-green ooze, rotting reeds, algae | swamp |
| 6 | beach sand | 3 | pale fine sand, small shells | beach |
| 7 | wet sand | 3 | darker, glossy, ripple marks | beach (tide band) |
| 8 | pebbles | 2 | rounded stones, grey and ochre | beach, riverbeds |
| 9 | desert sand | 4 | orange sand with wind ripples | desert |
| 10 | cracked clay | 3 | dry mud polygons, curled edges | salt pans, dry riverbeds |
| 11 | red sandstone | 4 | cross-bedded red and buff rock | canyon, mesas (triplanar, strata) |
| 12 | granite | 4 | grey speckled rock, fractures, lichen | alpine, cliff |
| 13 | limestone | 4 | pale layered rock, karst pits | sea cliffs, archipelago |
| 14 | scree | 3 | loose angular rock fragments | below cliffs, alpine |
| 15 | snow | 4 | wind-packed snow, sastrugi, sparkle | alpine |
| 16 | moss rock | 3 | rock under moss and ferns | jungle, montane |
| 17 | gravel | 2 | road shoulders, river gravel | everywhere near roads |
| 18 | red dirt | 2 | laterite, tyre-rut cracks | savanna, desert edges |
| 19 | scorch | 2 | burnt soil, ash, glassy spots | craters (terrain-deformation) |

Each biome names three layers by slope (`island-biomes` section 2), plus a wet layer near water:

```ron
desert: (flat: 9, slope: 18, cliff: 11, wet: 10),
alpine: (flat: 15, slope: 14, cliff: 12, wet: 14),
```

## 2. Shader

A `StandardMaterial` extension (WESL), per pixel:

1. Read the vertex's biome weights (three vec4 attributes, ten biomes), take the strongest two.
2. Each of the two picks a layer from the slope: flat under 20 degrees, slope 20-35, cliff over 35,
   blended over 5 degrees, and its wet layer near water. That is at most 4 layers.
3. Sample each layer's three maps (planar from above, or triplanar for cliffs: 3 taps a map).
   Blend by weight times height (the alpha channel), so stones poke through sand instead of
   cross-fading.
4. **Strata:** sandstone, limestone and granite take a tint from a 256 px strata strip indexed by
   world height (warped by 30 m noise), so bands run level along a whole canyon wall.
5. **Macro variation:** two noise octaves (40 m and 400 m) shift hue, brightness and roughness.
   Beyond 600 m a far colour map takes over (one 4096 px map of the whole island at 10 m a texel,
   baked from the same rules), so nothing tiles from altitude.
6. Scorch from the deformation channel, then PBR lighting and fog as now.

## 2b. MegaSplat-style options (owner, 2026-10-25: "Mega splats?")

MegaSplat (Jason Booth's Unity terrain shader) blends terrain textures differently from a weight per
layer. Each vertex or texel holds **two texture indices and a blend weight** into one texture array
of up to 256 textures. Each pair is blended by the textures' height maps, so one breaks through the
other in a natural line. **Texture clusters** pick between two or three variants of a material by
noise, so a 2 km plain does not repeat. What it would bring here:

| Idea | What it fixes | Cost | Take |
| --- | --- | --- | --- |
| All twenty layers in one texture array (BC7) | one material, one bind group, any layer per pixel | none beyond today's memory | yes |
| Texture clusters, 2-3 variants a layer | tiling on wide plains and dunes | about 1.5 times the layer memory (about 510 MB at 2048 px; the far-only layers at 1024 px bring it back near 400 MB); 1 more sample a layer | yes, for the five widest layers (grass, dry grass, desert sand, jungle floor, forest floor) |
| A per-pixel "procedural splat": the top two layers chosen per pixel from the vertex weights plus 30 m and 6 m noise, then height-blended | borders that break up below the 8 m vertex spacing, and only two layers sampled a pixel (cheaper than the four of section 2) | 6-8 samples a pixel instead of 12-36 | yes, replaces section 2's strongest-two-biomes step |
| A baked splat map (indices per texel), painted by the rules, streamed as a clipmap round the eye (4,096 px at 1 m, 64 MB) | hand-tuned detail (roads' verges, paths, scorch) at 1 m | a compute pass when the eye moves 500 m, 64 MB | later, if painting by hand is wanted |

How the mockup's transitions map to it: the moisture field sets the weights over a 9 km band; up
close the noise picks the two layers per pixel (patches and fingers); from the air the blend is a
plain crossfade, so a border reads as a gradient.

## 3. Cost (expected; measured in the cockpit on the owner's machine)

- **Texture memory:**
  - 3 maps x 20 layers x 2048 px, BC7 with mips: about 340 MB;
  - the far map: 16 MB;
  - 1024 px for the far-only layers would save about 120 MB.
  - An RTX 3070 has 8 GB.
- **Samples:**
  - typical (two layers, planar): 6 samples;
  - worst (four layers, triplanar): 36 samples;
  - plus noise and the far map.
  - Cliffs are a small share of the screen. Measured in the densest scene; if over budget, the
    second biome drops out past 300 m.
- **Vertex data:** 48 bytes more a vertex (three vec4 of biome weights). That is 216 KB more per
  ground node of 4,489 vertices.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| W5 | Terrain texture resolution | 2048 px tiles, 1024 px for far-only layers | provisional (survey W5) |
| T10 | MegaSplat-style texturing | one texture array, clusters on the five widest layers, a per-pixel procedural splat of the top two layers; a baked splat clipmap later | proposed (survey T10) |
| I5 | The ground material list | the twenty layers above, Material Maker graphs; CC0 scans (ambientCG) as references only | proposed (survey I5) |
