# Design: rock-formations

| Piece | Sizes | Biome | Per km2 | LOD0 triangles |
| --- | --- | --- | --- | --- |
| Sea stack | 30-120 m tall | cliff coast, archipelago | 2-6 | 20,000 |
| Arch | 60-200 m span | cliff coast, canyons | 0.2-1 | 30,000 |
| Hoodoo, spire | 15-60 m | canyon, desert | 20-60 | 6,000 |
| Boulder | 2-12 m | range, beach, jungle | 50-300 | 1,500 |
| Overhang, ledge | 20-80 m | canyon walls, cliffs | 5-15 | 10,000 |

- **Placement:** a pure function of the 264 m tree tile and the biome weights (the same tiles and
  growth radius as `vegetation`), so a tile grown twice is the same.
- **Drawing:** one shared mesh per piece and LOD, so Bevy batches each into one instanced draw:
  about 15 draws within 2 km.
- **Memory:** about 0.4 MB of instances per body.
- **Cost (expected):** about 1.5 M triangles within 2 km at LOD0-2 (the RTX 3070 target allows it),
  and 15 draws plus 15 per shadow cascade within 300 m.
