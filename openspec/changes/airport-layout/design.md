# Design: airport layout

## 0. Second pass: a whole airport (2026-10-29)

The owner on the first mockup: "That airfield looks kinda simple. What about more complex runway structures. Airplane
hangars, landing pads for helicopters, boarding tunnels. Multiple terminals support. Look at pictures of actual airports,
look at the references I gave you". The Crab Breeze Airport page (`docs/mockups/airport.html`) now lays out a whole
airfield, 2.3 x 1.35 km, after the references. Sections 1 to 5 below are the first pass, kept for the single-site
numbers they still hold (markings, PAPI, approach lights, holds).

| Area | What | Size |
|---|---|---|
| Runways | Main 09/27, and a secondary 14/32 crossing it (the RAF airfield's pair) | 2000 x 45 m; 1170 x 30 m |
| Taxiways | Parallel the main's length; three links with holding points; two rapid exits at 30 degrees | 23 m wide |
| Terminal 1 | Linear, glazed to the apron, curbside canopy; four gates | 320 x 40 m, 16 m high |
| Terminal 2 | A hall and a pier with gates both sides | hall 280 x 65 m, pier 200 x 30 m; six gates |
| Jet bridges | One a gate: a rotunda on its column, a tunnel in three telescoping sections, a cab at the aircraft's front left door, a drive column with wheels | ten |
| Stands | Lead-in line, stop bar, red safety line round the parked aircraft | per aircraft |
| Cargo | A terminal with six doors, the freighter on its stand | 180 x 65 m |
| De-icing pad | A stop ring and two spray booms, by the 27 end | 80 x 80 m |
| Maintenance | Two arched hangars, doors slid back, an airliner and the freighter inside | 110 x 80 m, 30 m high |
| Heliport | Three helipads (a dashed square, a ring, an H), a helicopter hangar, two helicopters | pads 30 m |
| General aviation | Two rows of 24 T-hangars, six light planes | 290 x 24 m a row |
| Tower | A shaft over an operations block, an eight-sided glazed cab, the aerodrome beacon turning green and white | 71 m to the beacon |
| Navigation | PAPI and approach lights at every runway end, an ILS localiser array and a glide path mast, windsocks, a surveillance radar | |
| Military side | The mech base as the game draws it (hangars, pads, a glazed base tower), its link and hold, four dispersal hard standings, the operational readiness platform, arrester gear at both ends of the main | the 700 x 500 m site |
| Services | A midfield fire station with four bays, a fuel farm of four tanks in a bund | |
| Landside | The road, curbs, a four-deck car park and a surface car park; the fence runs inside the road | |

The game's base today is one 700 x 500 m site, so this layout cannot sit at every base: AP1.


References: `docs/design/references/airports/` (owner, 2026-10-28). Metres throughout; x and z are the ground plane.

## 1. The airfield's frame

A base has a site (`AirBase::rect`, 700 x 500 m) and a runway on one of four sides (`RunwaySide`), `runway_gap_m`
from the site's edge. The **strip** is the rectangle from the site's edge to 40 m past the runway's far edge, and from
90 m before one end of the runway to 90 m past the other (a runway end safety area, so the fence never crosses the
runway). It holds the runway, the parallel taxiway and the links. It is flattened to the site's ground
(it replaces the runway and taxiway rectangles in the flattening list). It is sampled dry, above the sea, within
`runway_max_cut_m` and clear of the cities, highways and rivers when the runway is placed (`runway_fits`).

`runway_gap_m` goes from 60 to 100 m:
- taxiway edge to runway edge: 100 - 23 - 15 = 62 m;
- taxiway edge to the site: 15 m.

That is short of ICAO's 168 m for a code E runway, and is said here. A gap that size would put the strip past most
base sites on seed 42, and nothing in the game measures it.

## 2. The layout (`cb_core::world::airfield::layout(base, cfg) -> Airfield`)

Pure and deterministic; the one place every piece's footprint is decided. The game draws it and the tests read it.

| Piece | Where | Size |
|---|---|---|
| Parallel taxiway | In the strip, 15 m off the site's edge, the runway's whole length | 23 m wide |
| Links | From the taxiway to the runway at each end and the middle | 23 m wide |
| Holding positions | On each link, 50 m before the runway's edge | across the link |
| Apron entries | From the taxiway into the site: the first two of 1/3, 2/3, 1/6, 5/6 and 1/2 along the site's runway-side edge whose way 60 m in is clear of every building, stand, hangar and the tower | 30 m wide |
| Terminal | Along the low-x edge of the site, 5 m in | 30 x 260 m, 14 m high, glazed toward the stands |
| Boarding bridges | From the terminal's face to each stand, 4 to 7 m up on a leg | 3 m wide, 25 m long |
| Stands | In front of the terminal, 85 m apart | 3 stands, 80 x 80 m (the 70 m freighter fits) |
| Fire station | Front low-x corner | 30 x 25 m, 9 m high, three red doors toward the apron |
| Fuel farm | High-x edge, 30 m in, mid-depth | 3 tanks, 14 m across, 10 m high |
| Car park | The back low-x corner, past the terminal's end (clear of where mechs walk) | 120 x 70 m, bays 2.5 x 5 m |
| Helicopter stand | 55 m toward low x from the tower | circle 24 m across, an H, a parked helicopter |
| Windsock | 200 m along from the runway's `a` end, 25 m off its edge toward the taxiway | 8 m mast, 3 m sleeve |
| Fence | Round the site and strip together, 3 m inside their outline | 2.4 m posts every 10 m, two rails |

When the runway lies behind the hangars (`Back`), the apron entries come in round the hangar row's ends.

## 3. The runway's markings

All painted on the runway's surface (white) or beyond it (yellow chevrons). For a 2000 x 45 m runway:
- **Thresholds:** existing bars, set 60 m in. The 60 m before each is the blast pad, with yellow chevrons.
- **Designators:** the runway's magnetic heading in tens of degrees (bearing from `a` to `b`, +z north), two digits,
  9 m tall, drawn from segments 120 m from each threshold. They read from the approach side.
- **Touchdown zone:** bar groups either side of the centre line at 150, 300, 600 and 750 m past each threshold (3, 3,
  2 and 1 bars), each 22.5 m long; a group past the runway's middle is left off.
- **Aiming point:** two broad bars, 50 x 4.5 m, 400 m past each threshold.
- **Centre line:** the existing 30 m dashes with 20 m gaps.
- **Edge lines:** the existing lines.
- **Approach lights:** a post every 30 m for 300 m out past each end, each with a crossbar of five lamps (wider every
  third post).
  Each post stands on the natural ground, its lamps at runway height.
- **PAPI:** four boxes, 9 m apart, from 15 m outside the runway's far edge (away from the taxiway) at the aiming point.

The lamps are emissive (no point lights: survey N3 keeps night shadows to the player's own lamps).

## 4. Aircraft on the stands

Each stand holds one parked aircraft, spawned by the civil traffic's own `spawn_aircraft` (its model where it has one,
else its block-out stand-in), so a parked airliner is the one that flies (section 6.1 of CLAUDE.md). It is static, not in the traffic's cap and not in the hit list yet. Which
kinds stand where is seeded from the base's id: airliners, freighters, light planes.

## 5. Cost

Per base, block-out geometry:
- runway markings, about 600 quads;
- approach lights, 22 posts with 5 lamps each;
- fence, about 1100 posts;
- buildings, about 60 blocks;
- 4 parked aircraft at the stand-ins' size.

About 25 000 triangles a base, in 6 draws (concrete, asphalt, white paint, yellow paint, lamps, aircraft). Nothing per
frame. Not measured: the cloud session has no GPU.

## 5b. Junctions and the crossing

A taxiway meets a runway or the parallel taxiway on curved fillets (25 m radius, 12 m at the runway's end links so none
runs past an end), its end tucked 30 m under what it meets, so no square corner shows (owner, 2026-10-29: "need proper
merge"). Where the secondary runway passes under the main one, its markings stop at the main runway's edges, as at a real
crossing, so no paint lies in the main runway's surface (owner, 2026-10-29: "zfighting").

## 6. Every building breaks

Owner, 2026-10-29: "use our breakable framework, all buildings are destructible". Every airport building (both
terminals and the pier, the jet bridges, cargo, the hangars, the heliport and T-hangars, the tower, the car park, the
radar, the fire station, the fuel tanks, the de-icing booms, the mech base's hangars and tower) is made of pieces, and the
small-buildings kit's support rule decides what stands after a hit (`SmallBuildings.supportLinks` and `standing`, the
rule `cb_core::small_buildings::damage` mirrors): a piece stands while it reaches the ground through the pieces under it,
or through at most `MAX_SIDE` (3) pieces beside it. A piece lost from under others drops them.

- **Built like the houses** (owner, 2026-10-29: "Airport is not up to standard of like suburban houses"). Every building
  is drawn on the small-buildings kit's Builder by `docs/mockups/site_buildings_kit.js`, with the kit's own parts: walls
  in structural bays of at most 6 m with real openings (glazed curtain walls, ribbon windows, entrances, roll-up and
  hangar doors, each framed and glazed), floors and roofs in 5 m panels on interior columns, parapets and copings,
  stairs, check-in counters, gate seating, desks, workbenches and cars, signs from the kit's sign maker. Angled members
  (a jet bridge's tunnel, a hangar's arch, a boom) are oriented plates, each a piece that falls as one tilted chunk.
- **One engine.** `docs/mockups/building_engine.js` is the street's engine, taken out of the small-buildings page so the
  street, the airport and the seaport share it: the generated PBR textures, merged meshes (one draw a material), the shot
  (pieces blown out, big ones broken into chunks), the creak and fall of what is left hanging, a building that loses half
  its load-bearing walls coming down whole, the rubble settling in piles at the angle of repose, dust, fire and flashes.
- **Built to the rule.** Every piece is held as built: `MOCKUP.unheld()` reports none, the node check of every building
  type in the kit reports none.
- **Fitted out** (owner: "terminals are empty in airport, more furniitures!", "samer with control tower"): the terminals
  have check-in islands with baggage belts and queue posts, a baggage hall with carousels and trolleys, waiting seats,
  and upstairs gate lounges, a food court, shops with shelves and coolers and departure boards; the pier its seats,
  kiosks and booths; the tower's cab its consoles round the glass and the supervisor's desk, its base a reception,
  briefing room, offices, equipment room and crew room, its shaft landings and stairs; sheds their racking, pallets and
  forklifts.
- **Measured on the mockup** (block-out art, no GPU here): 34 buildings, 31 048 pieces, 1 200 740 triangles in 29 draws
  (one a material); about 700 000 of the triangles are furniture. The largest building is Terminal 2, 7 201 pieces. Support links are computed once at load, per building.
  A demolition charge at Terminal 1's face takes out 99 pieces; what they held falls.
- **In the game** each building becomes a breakable kit set (the small buildings' path: `kit.rs` pieces, `damage.rs`),
  with its colliders from the same pieces, so AP2 is answered: solid and breakable from the start.

## 7. The game build (2026-10-29, owner: "Dope, implement all that")

The game takes the approved page itself, never a second copy of it (CLAUDE.md section 6.1):

- **Export.** `tools/buildings/export_sites.cjs` runs the page's own script in node (its DOM, renderer and controls
  stubbed, its geometry real three.js r147), takes every building from the page's `put` and the paving, paint, lights
  and fence from the group the page builds. Each distinct building is written as a breakable kit by the houses' own
  writer (`tools/buildings/kit_export.cjs`, shared with `export_small_buildings.cjs`), the ground as one static
  `ground.glb`, and `assets/models/sites/airport/site.json` lists every placement, the ground's solids and the pad the
  airfield is flattened to. The civil export leaves out the page's military side: the air bases are the military
  airfields (AP1).
- **Placing.** `cb_core::world::sites::place_airport` tries the airfield on rings round each city, largest first, at
  16 bearings and every quarter turn, on dry land that rises at most 45 m across it, clear of the cities (300 m), the
  bases and their runways (600 m), the highways and the rivers (`sites:` in `data/world/world.ron`). Its pad levels the
  ground to its buildings' floor (the paving lies 5 to 24 cm over it), the houses keep off it.
- **Drawing and breaking.** Every placement is a lot of the `Airport` zone: drawn, dressed, furnished and broken
  exactly as a house (`cb-app::small_buildings`, `small_buildings::damage`), its glb read from its kit's folder. A
  building hundreds of metres long has its level-of-detail distances measured from its edge, not its middle.
- **Measured** (block-out art, no GPU here): 24 kits for 30 buildings. Terminal 2 draws 96 134 triangles of shell
  and 350 724 of block furniture within 100 m; Terminal 1 84 758 and about 300 000. Each kit's budget is its
  measurement, with the reason, in the exporter; the site's files are 120 MB. The ground: 16 666 triangles in 17
  surfaces. Not measured on a GPU: the cloud session has none.

## Mockup

`docs/mockups/airport.html` (`python3 tools/mockups/gen_airport.py`): the base today against the proposed airfield,
the runway on each of its four sides, from above, on final, from the cockpit and on foot, at true size beside the
Foreman and a pilot. Approved before any game code is written (CLAUDE.md section 4).

## Open questions

In the survey, section AP (`docs/design/survey.html`). Built on the recommendations while open (provisional).

| Id | Question | Provisional |
|---|---|---|
| AP1 | The airport (2.3 x 1.35 km) cannot be every base: which bases become what? | One airport by the largest city, four military airfields |
| AP2 | Colliders for the new buildings | Answered (owner, 2026-10-29: "all buildings are destructible"): solid and breakable from the start, section 6 |
| AP3 | Parked aircraft and jet bridges shootable | Later, with the civil standing rules |
