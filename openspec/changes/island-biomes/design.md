# Design: island-biomes

Sizes are at the 24 m mechs (mech-roster) and the 1.8 m pilot. The world stays 40,960 m square
with land out to about 13 km. All numbers would go in `data/world/world.ron` under `terrain:`.

## 1. Height, in order (each a pure function of (x, z), seeded by label)

1. **Base island and coast** (owner, 2026-10-24: "The island transition is rough"; the mockup's
   coast was a smooth blob with an even turquoise ring and a ramp into the sea all round):
   - The coastline is ragged at every scale: lobes, then 420 m of 1.1 km noise (headlands and
     bays), 140 m of 320 m noise (coves), and bays proper, notches up to about 2 km deep where an
     along-coast noise runs high.
   - A beach and low coastal strip 120-500 m wide (by noise) at 1-4 m, then the land rises to the
     old 20-50 m and the hills come in over the next 1.2 km.
   - A shelf 250 m-1.6 km wide: wide shelves are lagoons 2-8 m deep, with a reef 1 m under the
     surface near their edge (gaps where a noise is low); narrow ones drop fast. Past the shelf the
     floor falls at 7 %. The sea floor is sand.
   - The north-east sea cliffs are unchanged.
2. **Range:** distance `d` from a gently curving spine (a quadratic Bezier through the centre, 16
   km, heading 20 degrees east of north).
   - Envelope `exp(-(d / 4,200 m)^2)` along it, tapering at both ends.
   - Height `= envelope x (900 m + 1,300 m x ridged_mf(p))`.
   - `ridged_mf` is a domain-warped ridged multifractal (6 octaves, 5 km base, warp 1.2 km), its
     octaves weighted by the slope so far (Quilez's derivative fBm), so valleys stay smooth and
     peaks sharp: an eroded look without simulation.
   - Peaks reach 1,400-2,200 m.
   - Above 250 m the range is 40 % terraced into 210 m cliff bands with short risers, so rivers
     come off it in falls rather than steady slopes.
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

**Moisture decides the climate, height the band.** (Owner, 2026-10-24: "The island transition is
rough"; the mockup showed a straight jungle edge and a grass ring round the whole coast, even where
the desert met the sea.)
- Moisture `m`, 0 desert to 1 rain forest: `1 - smoothstep(-0.8 km, 8.5 km, s')`, where `s'` is
  the signed distance from the spine's line plus 900 m of 2.6 km noise. That is the rain shadow over
  a 9 km band, not a step. It is scaled by `0.72 + 0.36 south` (wetter to the south, `south` a
  13 km smoothstep in z, warped by 1.5 km), plus 0.22 of 3 km noise, plus 0.18 near rivers and
  0.25 in swamps.
- `s'` uses the distance from the nearest segment's line, which stays continuous past the spine's
  ends. The signed distance it replaces jumped from -d to +d across the spine's extended line,
  which drew the straight edge.
- The weights (each a smoothstep over a band; `low` below about 450 m and `mid` from 450 m to 1.6
  km, both wandering 220 m with noise):

| Biome | Weight |
| --- | --- |
| Alpine | above 1,450-1,750 m, or steep above 1,000-1,250 m |
| Montane forest | `mid` where `m` is over 0.25-0.45 |
| Jungle | `low` where `m` is over 0.6-0.8, more to the south |
| Temperate | `low` where `m` is 0.3-0.48 up to 0.68-0.86 (higher in the north) |
| Savanna | `low` (and 0.6 of `mid`) where `m` is 0.07-0.2 up to 0.32-0.46 |
| Desert | where `m` is under 0.17-0.05, below 1,100-1,400 m, all the way to the beach |
| Canyon, beach, cliff, swamp | as before |

- So the dry plateau (500-650 m) is desert, not montane forest, and the desert reaches its beaches.
- **Shares of the land at seed 42** (strongest biome, sampled every 320 m): desert 14.6 %, savanna
  7.0 %, temperate 24.4 %, montane forest 21.2 %, jungle 20.6 %, alpine 1.7 %, canyon 3.2 %, cliff
  4.1 %, beach 2.8 %, swamp 0.4 %. The first moisture band (7 km, desert under 0.22) gave 26 %
  desert, more than the 20 % before it; the owner asked after the desert's ratio, and it was cut
  back.
- On the ground a border is not a crossfade. Each biome's layers carry their own patch noise (140 m,
  35 m and 7 m), and the weights are cubed after it (a height blend), so one biome breaks into the
  other in patches and fingers.
- Trees and props pick their biome by its squared weight, so forests mix at the edges instead of
  changing species along a line.

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
  - **The look** (owner, 2026-10-24: "Waterfalls.. ooof don't look great"; the mockup had a flat
    white strip):
    - The curtain follows free fall (owner, 2026-10-24: "It's horizontal velocity is gentl. And
      it should fall in a parabolic curve because gravity"): the water leaves the lip at the
      river's surface speed, 3 m/s, so after falling `y` it is `3 sqrt(2 y / 9.81)` m out from the
      face. That is 19 m at the foot of a 200 m fall and 25 m at 346 m. It is a 25 by 5 grid,
      with rows packed toward the lip where the curve bends, and never under the ground.
    - It is glassy green at the lip, aerating to white by 40 % of the drop. Streaks ride the
      water: their phase is the time to fall that far, `sqrt(2 y / g)`, so they stretch as it
      speeds up. Its edges wander.
    - At the foot, where the curve lands: a foam disc of radius 30 m plus 0.35 of the drop, rings
      spreading from the impact; 8 mist billboards (the cloud sprites) rising a third of the drop;
      spray at the lip and the foot.
    - The ground under a fall is a face: across the fall's 300 m corridor it steps from the lip's
      height to the foot's. On the game's 8 m cells the drop sits in one cell, nearly vertical
      but a stretched pair of triangles (`terrain-features` meshes such faces properly); on the
      mockup's 80 m grid it is one steep cell, so the lower curtain can touch it.

## 4. Cities

Placement keeps its rules, plus: ground below 300 m with slope under 6 degrees across the square.
The capital is the largest site on the northern plain, within 6 km of the centre. The desert gets an
oasis city at a canyon mouth, and the jungle a river city.

The mockup places them (owner, 2026-10-24: "I don't see cities on the island"). At seed 42:
- **22 cities** fit, not 24, once every square has to stand on the main island.
- **The capital** stands 8 km north-east of the centre. With the range down the middle, no flat
  ground under 300 m lies within 6 km, so the 6 km rule cannot be met (survey I9).
- **The oasis** sits 1-3 km past a canyon's seaward end. At the mouths themselves the ground has
  about 185 m of relief across the square, three times what 6 degrees allows.
- **Highways** join the cities in a minimum spanning tree, and a road that crosses the range goes
  over the nearer pass.
- **They read as specks from the air** (owner, 2026-10-24: "I don't see cities on the island"): a
  0.6-1 km square on a 41 km island. The mockup proposes **suburbs**: houses on a 36 m grid aligned
  with the city out to 2.6 times its half-size, thinning outward, on gentle dry ground, with the
  arterials running on through them (`small-buildings` supplies the houses). That makes each city
  2-2.5 km across from the air. It also labels the cities (survey I11).

## 5. Cost summary (expected; measured on the owner's hardware)

- **Terrain:** sample cost rises about 3-4 times, all off the main thread.
- **Triangles:** unchanged, from the same quadtree.
- **Waterfalls:** under 100 draws and 8,000 particles worst case.
- **Biome weights:** computed with the vertex (8 more noise reads); the trees and rocks read the
  same function.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| I1 | The range | 16 km spine NNE, peaks 1.4-2.2 km through the clouds, passes for highways | proposed (survey I1) |
| I2 | Where the capital goes | the northern plain within 6 km of the centre (the range takes the middle) | proposed (survey I2) |
| I3 | Which side is desert | east (the rain shadow), jungle west and south-west | proposed (survey I3) |
| I4 | Archipelago | 20-40 islets off the south-west, 0.2-2 km across | proposed (survey I4) |
| I6 | Jungle density against frame time | 30-45 m canopy, grown only near the bodies like today's trees; measured | proposed (survey I6) |
| I8 | Waterfalls | 20-40 from the terrain, 40-200 m, spray, mist and plunge pools | proposed (survey I8) |
| I9 | The capital, with the range in the middle | the nearest flat site north of the centre, about 8 km out | proposed (survey I9) |
| I11 | Cities from the air | suburbs out to 2.6 times each city's half-size; the cities keep the game's size | proposed (survey I11) |
| I10 | How wide the climate borders are | moisture over a 9 km band (desert 14.6 % of the land); borders in patches, not lines | proposed (survey I10) |
