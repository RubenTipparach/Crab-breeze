# Design: city-districts

## 1. The mockup

`docs/mockups/city_layouts.html`, three.js r128, each city merged into one mesh with vertex colours (one draw call a
side). The "today" side runs the generator's own rules and numbers from `data/world/world.ron` (block 70-110 m, streets
24 m, arterial 36 m every fourth, lots 18-36 m, setback 1-3 m, sidewalk 4 m, storeys 60 at the centre falling to 2 with
exponent 0.7, fill 0.35-1, 3.5 m a storey, parks 6 %, plazas 8 %, suburb rows 40-150 m out every 36 m, houses 12 m
apart). The "future" side is the proposal below. A 24 m block stands in both for the mech's scale.

| | Today | Future (mockup) |
| --- | --- | --- |
| Footprint | 960 m square | about 2 km across inside the ring, suburbs to 2.5 km |
| Buildings | about 1,000 | about 4,200 |
| Tallest | 147 m | about 190 m |
| Mockup triangles | about 15,000 | about 93,000 |

## 2. The districts

A city is a centre, a river line, an old-town centre, a downtown centre, a park and a ring radius, and a zone function
that says which district a point is in. Each grid district walks its own rotated grid over the city and keeps a block
whose centre and corners are in the district and clear of the river, avenues, ring and rail; a block that does not fit
tries again as four quarter blocks. Each district has a block filler:

| District | Grid angle | Block | Street | Filled with |
| --- | --- | --- | --- | --- |
| Old town | radial | rings 70/130/185/235 m | 9 m lanes | attached houses 3-6 storeys, pitched roofs |
| Downtown | 0.22 rad | 84 x 84 m | 18 m | podium 9-16 m and a tower 40-250 m |
| Courtyard blocks | -0.08 rad | 96 x 78 m | 16 m | perimeter units 14-30 m long, 4-8 storeys, a courtyard |
| Rowhouses | -0.08 rad | 112 x 46 m | 12 m | terraces 5.5-7.5 m wide, 2-3 storeys, gardens |
| Estate | -0.40 rad | 150 x 120 m | 20 m | slabs 55-80 x 13 m and point towers, 9-21 storeys, green |
| Port and works | 0.32 rad | 150 x 110 m | 18 m | sawtooth sheds, tank farms, stacks, container yards |
| Suburbs | curving rings | | 9 m | detached houses each side, gardens |

## 3. Cost

Unmeasured. The future city has about four times the buildings of today's capital over about four times the ground, so
the count per square kilometre is about the same; the cost that moves is the city's size (more of it within the 150 m
dressing radius and the far facade range at once). Measured on the owner's machine before it is built.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| CD1 | Is the future look the right one? | As in the mockup | open (owner's review) |
| CD2 | Does every city get every district? | The capital all; smaller cities a subset by size (old town, courtyard blocks, rowhouses, suburbs; downtown only in the largest few) | provisional |
| CD3 | Do cities get a river? | Where the island's hydrology already runs one near the city; otherwise none | provisional |
