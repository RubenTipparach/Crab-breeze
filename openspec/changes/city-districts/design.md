# Design: city-districts

## 1. The mockup

`docs/artifacts/city-layouts/city_layouts.html`: three.js r128, each city merged into two meshes with vertex colours.
The ground only receives shadows and the buildings cast them.

**Today** runs the generator's own rules and numbers from `data/world/world.ron`:
- blocks 70-110 m, streets 24 m, a 36 m arterial every fourth street;
- lots 18-36 m, setback 1-3 m, sidewalk 4 m;
- 60 storeys at the centre, falling to 2 with exponent 0.7, fill 0.35-1, 3.5 m a storey;
- parks 6 %, plazas 8 %;
- suburb rows 40-150 m out, 36 m apart, houses 12 m apart.

## 2. The future city

| Part | Rule |
| --- | --- |
| Extent | radius 1,020 m with a ragged edge (+/- 170 m), 1.45 times that with a large downtown |
| Street network | one grid for the whole city (cells 132 x 88 m: 14 m streets, a 22 m arterial every fourth line across and every sixth along), bent by a slow warp of up to about 40 m, so it is close to a grid but not quite and every street runs on to the next (owner: "how do cars drive if the roads dont connect?"); every cell inside the edge is built, so no land lies empty between parts of the city (owner: "a lot of dead space between the sections of the city", "seems like a waste of land") |
| Links | through: frontage roads 30 m each side of the highway, only arterials cross it, an interchange at each; the arterials carry on into the farmland as section roads, which meet the bypass |
| House blocks | 118 x 74 m, a 6 m alley, two rows of houses on 13-17 m lots (8-11 x 8-10 m, one or two storeys, pitched roofs), trees in the yards, garages on the alley |
| In the blocks | parks 5 % (10 % near the edge), schools 2.5 %, corner shops on every fourth street |
| Strip | the cells within 190 m of the highway (through), or the two rows of cells along the spur avenue (bypass): a big box (55 %), a gas station (20 %) or a car lot (25 %), each with its car park |
| Highway through | 34 m, frontage roads 46 m out, an interchange every 450-500 m; no houses within 190 m |
| Bypass | 34 m round the south at 1.18 times the city's radius, a 26 m spur avenue to the downtown, the strip along the spur and near the interchange |
| Downtown, small | cells of the same grid; compact about 4 x 4 (rounded), main street 8 x 2, two cores 3 x 3 each; towers 40-90 m |
| Downtown, large | compact about 7 x 7 (rounded), main street 13 x 3, two cores 4 x 4; towers 60-260 m; apartment blocks of 3-5 storeys within 650 m |
| Industrial park | blocks of sheds 8-13 m tall at the highway's west end |
| Coast | the sea south of the city, a beach, a small harbour of quay cranes |

## 3. Cost

Mockup counts, from its own footer:
- the small compact city: about 2,700 buildings;
- the large compact city: about 5,700 buildings, the tallest 217 m, about 230,000 triangles.

The game's cost is unmeasured. Most of the buildings are houses, which the game already draws as small buildings
(`small-buildings`), and they would be inside the city rather than in a ring round it. The towers are as many as today's
capital or fewer. It is measured on the owner's machine before it is built.

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| CD1 | Is the Fargo look the right one, and which variations? | Small and large downtowns, all three shapes, both highways | open (owner's review) |
| CD2 | Which cities get which variation? | The capital large; the rest small, the shape and the highway drawn per city | provisional |
| CD3 | Water | None but the coast where a city stands on it; no rivers (removed from the game) | provisional |
