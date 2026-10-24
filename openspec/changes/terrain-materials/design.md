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
| W22 | The ground material list | the twenty layers above, Material Maker graphs; CC0 scans (ambientCG) as references only | proposed (survey W22) |
