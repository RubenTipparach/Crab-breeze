# Island environment: research and sources

Research behind `openspec/changes/island-biomes`, `rock-formations`, `terrain-materials`,
`cloud-puffs` and `cloud-rain`, 2026-10-24.

## What the island has to do

Missions are flown and strode across the whole 41 km island, Ace Combat style. From 1-2 km up,
the island should read as places: a snow range through the clouds, a red desert of canyons, a dark
jungle, swamps, beaches, cliffs and islets. From the cockpit at 20 m it should look real: rock you
could climb, mud you wade through, sand that ripples.

## Findings

1. **Eroded mountains without simulating erosion.**
   - Plain fBm makes blobby hills.
   - A ridged multifractal (1 - |noise|, squared, octaves weighted by the octave before) makes
     sharp ridgelines.
   - Quilez's derivative-weighted fBm damps detail where the slope is already steep, which reads as
     erosion: smooth valleys, sharp crests.
   - Domain warping bends ridges so they don't line up on the noise grid.
   - All of these stay pure functions of position, which keeps today's quadtree and off-thread
     meshing (`island-world`).
2. **Strata and mesas come from terracing.** Quantising height (`floor(h / step) * step`, softened)
   over a canyon's depth profile gives ledged walls. A strata colour strip indexed by world height
   makes the bands line up along a wall, as in the Far Cry 5 and Horizon terrain talks.
3. **Biomes from a few fields.** Height, slope, moisture and distance to the coast and rivers are
   enough to place biomes believably. A mountain range's lee side is dry, the rain shadow, which
   puts the desert east of the range with no extra rule. Far Cry 5 and Horizon Zero Dawn both drive
   placement from layered rules like these rather than painted maps.
4. **Ground materials: height blending and triplanar.**
   - Blending splat layers by their height maps (sand settling between stones) is what makes
     transitions look real; a linear cross-fade looks like a smear.
   - Triplanar mapping stops cliffs stretching. Normals need the swizzle Golus describes.
5. **Breaking up tiling at range.** Macro variation noise at tens and hundreds of metres, and a far
   colour map baked from the same rules, are the usual answer (Far Cry 5's terrain rendering talk).
   From 2 km up a 4 m tile repeats 500 times across the view without them.
6. **Rock formations need meshes.** Arches, overhangs, stacks and hoodoos can't come from a
   heightfield. They are instanced kit meshes placed by the same biome rules, as Horizon places
   rocks.
7. **Cheap clouds** (built in this PR, `cloud-puffs`):
   - Ace Combat 7 uses Simul's trueSKY volumetric clouds, which is too dear for our budget.
   - Harris and Lastra's lit cloud impostors (GDC 2002) show that shading a cloud's shell by sun
     angle, height and forward scatter reads correctly for a fraction of the cost.
   - We draw lumpy, merged blob meshes lit that way, inside Bevy's fog.
8. **Rain on the glass** reuses Pale Blue Dot's port of Heartfelt's lens drops (already in the
   water lens pass), driven by cloud density (`cloud-rain`).

## Sources

- Inigo Quilez, "Value noise derivatives" (eroded fBm): https://iquilezles.org/articles/morenoise/
- Inigo Quilez, "Domain warping": https://iquilezles.org/articles/warp/
- F. Kenton Musgrave, ridged multifractal terrain, in Ebert et al., *Texturing and Modeling: A
  Procedural Approach*, 3rd ed., Morgan Kaufmann, 2003.
- Etienne Carrier, "Procedural World Generation of Far Cry 5", GDC 2018:
  https://www.gdcvault.com/play/1025557/Procedural-World-Generation-of-Far
- Jeremy Moore, "Terrain Rendering in Far Cry 5", GDC 2018:
  https://www.gdcvault.com/play/1025480/Terrain-Rendering-in-Far-Cry
- Jaap van Muijden, "GPU-Based Run-Time Procedural Placement in Horizon: Zero Dawn", GDC 2017:
  https://www.gdcvault.com/play/1024120/GPU-Based-Run-Time-Procedural
- Andrey Mishkinis, "Advanced Terrain Texture Splatting", Game Developer, 2013:
  https://www.gamedeveloper.com/programming/advanced-terrain-texture-splatting
- Ben Golus, "Normal Mapping for a Triplanar Shader", 2017:
  https://bgolus.medium.com/normal-mapping-for-a-triplanar-shader-10bf39dca05a
- Mark Harris and Anselmo Lastra, "Real-Time Cloud Rendering", Eurographics 2001, and the GDC 2002
  talk on cloud impostors: https://www.markmark.net/clouds/
- Ace Combat 7 and trueSKY (DualShockers):
  https://www.dualshockers.com/ace-combat-7-skies-unknown-trueskys-clouds/
- Heartfelt lens rain (Martijn Steinrucken, Shadertoy ltffzl), via Pale Blue Dot's water lens port.
- Rain shadow, physical geography: https://en.wikipedia.org/wiki/Rain_shadow
- ambientCG (CC0 scans, references for the Material Maker graphs): https://ambientcg.com/
