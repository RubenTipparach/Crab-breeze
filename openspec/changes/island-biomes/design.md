# Design: island-biomes

Sizes are at the 24 m mechs (mech-roster) and the 1.8 m pilot. The world stays 40,960 m square
with land out to about 13 km. All numbers would go in `data/world/world.ron` under `terrain:`.

## 1. Height, in order (each a pure function of (x, z), seeded by label)

1. **Base island:** today's shape (coast noise, shelf, sea floor), land 20 m up.
2. **Range:** distance `d` from a gently curving spine (a quadratic Bezier through the centre, 16
   km, heading 20 degrees east of north).
   - Envelope `exp(-(d / 4,200 m)^2)` along it, tapering at both ends.
   - Height `= envelope x (900 m + 1,300 m x ridged_mf(p))`.
   - `ridged_mf` is a domain-warped ridged multifractal (6 octaves, 5 km base, warp 1.2 km), its
     octaves weighted by the slope so far (Quilez's derivative fBm), so valleys stay smooth and
     peaks sharp: an eroded look without simulation.
   - Peaks reach 1,400-2,200 m.
3. **Passes:** where a highway crosses the spine, the range is pressed down to 500-700 m over a
   1.5 km width.
4. **Rain shadow:** east of the spine (signed distance positive), a plateau rises to 250-400 m,
   edged by an escarpment.
5. **Canyons:** a canyon network (a branching tree of polyline segments from the seed, indexed in
   512 m cells) is cut into the plateau. The depth profile `depth x (1 - smoothstep(0, w, dist))`
   is terraced into strata steps of 12-25 m, giving walls of layered ledges. Depth is 150-250 m,
   floor width 60-200 m.
6. **Mesas and buttes:** on the plateau and the desert flats, a terrace function `floor(h / step) x
   step + smooth` over noise blobs leaves flat-topped tables 300 m-2 km across, 60-180 m high.
7. **Dunes:** on the eastern sands, low ridged noise along the wind (8-25 m, 300 m wavelength).
8. **Cliffs:** on the north-east coast the shelf is replaced by a 60-150 m drop (a sigmoid on the
   coast distance), with stacks and arches as `rock-formations`.
9. **Archipelago:** 20-40 islet blobs (Poisson disc, 1.2 km apart) within 3-9 km off the
   south-west coast, each a small copy of the base island shape (20-80 m high), some with cliffs.
10. **Swamp flats:** within 3 km of the western river mouths, ground below 6 m is flattened toward
    1-2 m above the river water, with pools.
11. **Cities, highways and rivers:** as today, on top. Rivers run downhill from the range.
    Waterfalls are recorded where a river's bed drops more than 25 m within 60 m.

**Cost.** Today a sample is about 70 ns. With the range, canyon index and terraces it is expected at
200-300 ns (measured by the existing `ground_chunks_cost_less_the_coarser_they_are_drawn`).

| Work | Today | Expected |
| --- | --- | --- |
| A ground node (4,489 samples) | 0.3 ms | 0.9-1.3 ms, off the main thread |
| Worker time in flight | 2 ms a second | 5-7 ms a second |
| Body queries (`height_at`, about 10 a step) | trivial | trivial |

## 2. Biomes

Every point gets weights over ten biomes from six fields: height, slope, signed distance from the
spine (west wet, east dry), moisture noise (3 km), distance from the coast, and distance from
rivers.

| Biome | Where | Ground layers (terrain-materials) | Trees and props |
| --- | --- | --- | --- |
| Alpine | above 1,600 m, or steep above 1,000 m | snow, granite, scree | none above 1,800 m; dwarf pines lower |
| Montane forest | the range's flanks 400-1,600 m | forest floor, granite, moss rock | pines, firs |
| Jungle | west and south-west below 400 m, wet | jungle floor, mud, moss rock | 30-45 m canopy trees, palms, ferns, vines |
| Swamp | western river mouths below 6 m | mud, murky water, reeds | mangrove, bald cypress, dead snags |
| Temperate | north plains | grass, dry grass, dirt | oak, birch, hedgerows |
| Savanna | the transition east of the passes | dry grass, red dirt | acacia, scrub |
| Desert | east, the rain shadow | desert sand (dunes), red sandstone, cracked clay | cacti, joshua trees, dry scrub |
| Canyon | inside canyon cuts | red sandstone strata (triplanar), gravel | rock formations, hoodoos |
| Beach | within 120 m of the sea, low | beach sand, wet sand, pebbles | palms, driftwood |
| Cliff | slopes over 45 degrees | limestone or granite by region (triplanar) | sea stacks, arches |

Weights blend over 200-800 m transitions with noise, so borders are never straight lines.

## 3. Water by biome

- The one water material gets a murk input: the vertex colour's third channel, 0 clear to 1
  swamp-brown and opaque at 1.5 m. Rivers pick it up from their biome.
- Swamp pools are shallow and waded (`skim-depth`: skates only past half the mech).
- **Waterfalls:**
  - a curtain mesh down the recorded drop, scrolling foam (Material Maker flow and foam maps);
  - spray particles at the lip and the foot (`combat-fx`'s particles), and a mist volume (a few
    soft sprites);
  - a plunge pool carved below;
  - sound later (`audio`).
  - About 20-40 falls on the island; each is 2 draws and 200 particles within 2 km.

## 4. Cities

Placement keeps its rules, plus: ground below 300 m with slope under 6 degrees across the square.
The capital is the largest site on the northern plain, within 6 km of the centre. The desert gets an
oasis city at a canyon mouth, and the jungle a river city.

## 5. Cost summary (expected; measured on the owner's hardware)

- **Terrain:** sample cost rises about 3-4 times, all off the main thread.
- **Triangles:** unchanged, from the same quadtree.
- **Waterfalls:** under 100 draws and 8,000 particles worst case.
- **Biome weights:** computed with the vertex (8 more noise reads); the trees and rocks read the
  same function.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| W18 | The range | 16 km spine NNE, peaks 1.4-2.2 km through the clouds, passes for highways | proposed (survey W18) |
| W19 | Where the capital goes | the northern plain within 6 km of the centre (the range takes the middle) | proposed (survey W19) |
| W20 | Which side is desert | east (the rain shadow), jungle west and south-west | proposed (survey W20) |
| W21 | Archipelago | 20-40 islets off the south-west, 0.2-2 km across | proposed (survey W21) |
| W23 | Jungle density against frame time | 30-45 m canopy, grown only near the bodies like today's trees; measured | proposed (survey W23) |
| W25 | Waterfalls | 20-40 from the terrain, 40-200 m, spray, mist and plunge pools | proposed (survey W25) |
