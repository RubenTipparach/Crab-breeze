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
| Links | no frontage roads (their ramps and the overpasses could not share the ground with them); a street that meets the highway without crossing it stops 40 m short; the arterials carry on into the farmland as section roads, which cross every highway on bridges |
| House blocks | 118 x 74 m, a 6 m alley, two rows of houses on 13-17 m lots (8-11 x 8-10 m, one or two storeys, pitched roofs), trees in the yards, garages on the alley |
| In the blocks | parks 5 % (10 % near the edge), schools 2.5 %, corner shops on every fourth street |
| Strip | the cells within 190 m of the highway (through), or the two rows of cells along the spur avenue (bypass): a big box (55 %), a gas station (20 %) or a car lot (25 %), each with its car park |
| Highway through | 34 m, an exit at every arterial in town (every fourth street line, about 530 m apart); no houses within 190 m; the verge it clears is planted, a green buffer, not empty land |
| Highway modes | **at grade**: only the arterials cross, each at its exit, the arterial bridged over the highway; **overpasses**: every street with houses on both sides also crosses, on a single-span bridge 8 m over the highway (no piers), flat for 26 m either side of its centre line (longer, and higher, where it must also span an exit's ramps) and down an embankment (a levee) at 8 % each way, with no exit unless it is an arterial; **elevated**: the highway on a 10 m viaduct on piers every 40 m, kept clear of every street that passes under it, car parks under the deck; **bypass**: round the edge, its exits on the section roads. No tunnels: "under the city" is the bypass, bridges and levees (owner, 2026-11-04) |
| Bypass | 34 m round the south at 1.18 times the city's radius, a 26 m spur avenue to the downtown, the strip along the spur and near the interchange |
| Downtown, small | cells of the same grid; compact about 4 x 4 (rounded), main street 8 x 2, two cores 3 x 3 each; towers 40-90 m |
| Downtown, large | compact about 7 x 7 (rounded), main street 13 x 3, two cores 4 x 4; towers 60-260 m; apartment blocks of 3-5 storeys within 650 m |
| Industrial park | blocks of sheds 8-13 m tall at the highway's west end |
| Coast | the sea south of the city, a beach, a small harbour of quay cranes |

### Interchanges

After the owner's chart (2026-11-04: "highways can cut through cities, can go over cities or under cities, because of one
simple thing: EXITS!"). Each exit is laid out in its crossing's own frame (along the highway, along the street) and no
building stands inside its footprint.

| Exit | Where | Ramps | Footprint (along x across, m) |
| --- | --- | --- | --- |
| SPUI (single point urban) | the exit nearest the downtown | four, tight beside the highway, landing on the bridge's span | 540 x 80 |
| Diamond | the other arterials | four ramps to the street, landing square to it 100 m out | 540 x 210 |
| Single quadrant | second from the west | one ramp swinging through one quadrant to the street 165 m out | 540 x 350 |
| Partial cloverleaf | the west end, by the industrial park | two 270 degree loops (radius 42 m) in opposite quadrants, leaving the highway just past the bridge and merging onto the street's embankment heading back toward it; two diamond ramps | 540 x 230 |
| Cloverleaf | the east end | four loops and four outer ramps | 640 x 390 |
| Trumpet | where the spur avenue meets the bypass (a T) | the spur crosses the bypass and ends beyond it in a loop and a ramp | 560 x 320 |

Three-way directional and stack interchanges (where two highways meet) wait for a map with two highways. On a bypass an
exit goes only where the road meets it within about 40 degrees of square; a steeper crossing gets a bridge only.

### Drivable, and proved so

Owner, 2026-11-04: "make sure cars can actually drive on these!", "you need a driving bot or some sort of fill system to
verify geometry+ colliders are valid for driving along them". Every surface a car drives on (street, section road,
highway, viaduct deck, ramp, bridge span, embankment) is recorded as a centre line with heights and a width, and the
mockup's `driveCheck` proves the network, in every variation:

| Rule | Limit |
| --- | --- |
| The fill | every road reachable from the downtown (samples every 6 m; two roads join where one lies on the other within 0.5 m of its height) |
| Clearance | where one road crosses over another, its underside clears the lower by 4.5 m (a deck 1.2 m thick; an embankment is solid) |
| Grade | no road steeper than 8 % |
| Turns | highway radius at least 400 m; ramp radius at least 25 m (a 30 km/h ramp terminal) |
| Ramp ends | every ramp joins another road at both ends |
| Piers | none stands in a lane |
| The bot | 12 trips by shortest path, from random streets to the downtown and to both ends of the highway; every one arrives |

The generator holds itself to the same rules: a street stops short of any structure it would run into (too high to
join, too low to pass under), and a street or section road left cut off is not built, by the same fill. The check
found and drove out: ramps ending short of their street (the owner's screenshot), loops ending backwards, bridge piers
in the lanes, bridge decks with walls down across the highway, a bypass with a corner in it, ramps too steep for their
SPUI, bridges ending in fields, the farmland south of the highway cut off. A copy with the ramps stopped 18 m short of
the street fails it (11 ramp ends in the air).

What it proves is the mockup's road surfaces. Junctions are allowed a 0.5 m step, which the game's generator must weld
into a graded joint; the game's colliders get the same fill, clearance, grade, turn and bot checks, in `cb-core` tests,
when the generator is built.

## 4. In the game

The owner approved the look (2026-11-04: "looks great ... go ahead and start implementing this in game"), so CD1 is
answered. The game today builds every city as one flat square of a straight grid, every block filled with boxes from 60
storeys to 2, and every road has a single height: `road_top_at`, the cars' `OnRoads`, the lane graph's heights and the
terrain under a highway all assume nothing is stacked. The mockup's exits need roads over roads, so the work comes in
three steps, each playable:

| Step | What the player sees | What changes in the code |
| --- | --- | --- |
| 1, now | The Fargo city on today's grid: a small tower downtown, low flats round it, block after block of houses with yards and drives on the city's own streets, the strip of businesses and car parks where the highway comes in, parks and schools; car parks, streets and highways each on their own material | blocks get a use (`BlockUse`); `build_city` builds by use; the suburbs' lot placer also fills house and strip blocks, its drives and car parks reaching the city's streets the way they reach a highway (one implementation); the highway on concrete, car parks on their own surface |
| 2 | Highways through and round the city with real exits (the mockup's six), on bridges and embankments | layered road surfaces: a bridge deck over the road under it, ramps as graded roads whose ground is their embankment, the lane graph joining a highway along its length, every height query told which level a body is on; the mockup's drive check as `cb-core` tests over the generated roads |
| 3 | The grid drifts a few degrees between neighbourhoods | turned blocks and footprints: `Rect` footprints become turned boxes through collision, facades and interiors |

### Step 1 in numbers

| Block use | Where | What stands on it |
| --- | --- | --- |
| Downtown | centre within 0.18 of the city's half size (0.30 for the capital, scaled by size between) | today's buildings, storeys falling from the downtown's tallest to 6 at its edge |
| Flats | centre within 0.45 of the half size | walk-ups of 3 to 5 storeys |
| Strip | within 220 m of where a highway reaches the city | businesses from the roadside kits, side by side along the street, each with its striped car park out to the kerb |
| Houses | the rest | two rows of houses back to back, each facing its street with a drive to the kerb; yards on the ground's own grass, a 4 m pavement round the block |
| Park | 6 % of blocks (as today) | grass and trees |
| School | 3 % of the house blocks | one 2 storey building and a playing field |

The streets, lanes, traffic, lamps and the start point do not change. Every house, business and car park passes the
suburbs' own rules (a drive no steeper than its limit, reaching the road, off the other lots). Car parks are their own
surface (`Surface::Lot`) on the worn asphalt set, paler than the streets; the highway takes the CC0 Concrete047A set
the quays already use.

## 3. Cost

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
| CD1 | Is the Fargo look the right one, and which variations? | Small and large downtowns, all three shapes, both highways | answered: approved (owner, 2026-11-04: "looks great") |
| CD2 | Which cities get which variation? | The capital large; the rest small, the shape and the highway drawn per city | provisional |
| CD5 | Frontage roads: none, as now (the ramps and overpasses need the ground), or added outside the interchanges? | None | provisional |
| CD4 | Which highway mode is the default, and does a city mix them (elevated through the downtown, at grade outside)? | At grade outside the edge, overpasses in town; elevated only for the capital's downtown | provisional |
| CD3 | Water | None but the coast where a city stands on it; no rivers (removed from the game) | provisional |
