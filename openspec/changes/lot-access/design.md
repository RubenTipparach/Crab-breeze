# Design: lot-access

Owner, 2026-10-27: "driveways need to connect with roads for it to be realistic, same with businesses they need
parking lots that connect with road".

## 1. What the player sees

On foot in a suburb: a street in front of every row of houses, each house with a drive from its garage or carport
down to the street, a lowered curb where they meet. Walking the street leads, by side streets, to the city's own
streets or to a highway. In a town or at a roadside stop: each diner, store, garage or motel has a striped car park
in front of it, opening onto the highway through a curb cut; the gas station's apron runs out to the road at both
ends. From the cockpit (about 20 m up) the suburbs read as streets lined with houses, not houses in a field.

## 2. Suburb streets

Today (`lots::suburbs`) each city has rows of houses on a wandering round rectangle 40-150 m from its square, rows
about 36 m apart, each house facing the city, its front on the row line.

- **The street runs along the row line** (the kits' front edge, where their drive slabs end), offset toward the city
  by a verge (`street_verge_m`, 2 m: room for a house turned up to 8 degrees off the row) plus half the street
  (`street_width_m`, 8 m: two 3.5 m lanes and a 0.5 m gutter each side). It follows the same wander as the row,
  sampled every 4 m. The kits' own front yards (drive slab, walk and lawn, up to 12 m deep) are the yard.
- **Only where houses stand.** A row's built lots are walked in order; a street piece starts 10 m before the first
  and ends 10 m past the last, and breaks where two neighbours are more than `street_break_m` (60 m) apart.
- **Every piece is joined.** A connector street runs from a piece straight toward the city's square until it meets
  an inner row's street, the city's square (its grid starts and ends with a street, so its edge is one) or a
  highway. It is tried from the piece's middle, then its ends, then every 4 m along it (`connector_every_m`), so it
  finds a gap between the inner row's houses. A connector may not cross a building, water or an air base. A piece
  that can't be joined is dropped with its houses.
- **Rows stay 36 m apart**: the streets fit between the rows as they are (measured: of about 1,400 suburb houses
  placed, 11 lacked a connector and 49 street samples ran into water or a building).
- **Draped, not levelled.** As built, a street lies on the ground as the lots' pads leave it, drawn
  `street_lift_m` (0.13 m) over it in 4 m pieces, the drives and car parks in 2 m pieces: the terrain's lattice is
  8 m, so the paving follows it. A highway (0.15 m) wins where they cross. Levelling the ground under the streets,
  as the highways are, was the plan; draping gives the same look with no new ground rule, and residential streets
  follow the land.

## 3. Driveways

- **Each house kit records its drive** in the manifest (`access: { drive: [x, width_m] }`): its slab runs to the
  kit's front edge (12 m) centred on `x`. The kits with no drive got one in the mockup (the L-plan ranch, two
  colonials, the three bungalows, the adobe, stilt and beach houses, the trailer): a 3 m concrete strip beside the
  house on its open side, with a car on it. The checker fails a house kit with no drive, or a drive with no paved
  slab across its width at the front edge (the real glb, not the record).
- **The game paves on from the kit's slab to the street**: a straight strip of the drive's width, rising from
  `drive_lift_m` (0.06 m, the slab's top) to just under the street. Where it meets the street the curb is cut: the
  strip widens by 1.5 m each side over its last 1.5 m.
- **A drive that rises more than 20 percent** (`drive_max_grade`) over its length and the pad's 10 m blend drops
  its lot; so does a drive that would cross another building, or reach no street within 24 m (`drive_reach_m`).
- **Town houses** on the highway get the same drive to the highway's shoulder.

## 4. Parking lots

- **Each business kit records where its car park starts** (`access: { lot: z, stalls }`): in front of the building
  (the diner, the corner store, the motel, the Quonset workshop, whose cars stood on grass), or at the end of the
  kit's own apron (the gas station, the garage, the strip mall, the drive-in).
- **The car park** runs from there across the kit's width to `parking_depth_m` (12.5 m: a row of stalls and a 7 m
  aisle) past the kit's front edge, asphalt, `lot_lift_m` (0.05 m) over the ground. The businesses already stood
  16 m off the highway's paved edge (its 12 m flat shoulder and the 4 m setback), so nothing moved. Stall lines are
  2.6 m apart (`stall_width_m`), 0.1 m wide and 5 m long (`stall_depth_m`), 1 cm over the asphalt.
- **Curb cuts**, 7 m wide (`cut_width_m`), run from the car park's far edge on to the highway's paved edge: one in
  the middle, or one at each end of a car park wider than 30 m (`two_cuts_over_m`).
- **The gas station's apron** already runs in front of its pumps: its car park has no stalls (`stalls: false`).
- **Neighbouring car parks** are not joined: a 6 m strip of grass stays between them (`town_gap_m`). Alternate ones
  sit 2 mm apart in height so two that meet on a bend never share a plane.

## 5. Data and drawing

- `cb_core::world::small_buildings::paving::Paving`, built with the lots (`lots::place`) and kept beside them
  (`Lots::paving`): `streets` (a centre line, width and lift) and `slabs` (four corners, each with its own lift, and a
  surface: asphalt, concrete or paint), with an index for `paved(p, r)`. Pure and deterministic.
- **Nothing grows on it**: `cover_at` treats paving like a highway (no grass or trees within the road clearance), and
  `surface_at` reads it as paved underfoot.
- `cb-app::paving` drapes it on the ground as each ground node draws it (2026-10-28, owner: "suburb roads are
  hovering"): past 700 m a node draws the ground on 16 m cells, then 32 m, which cut under the tops of dunes, and
  paving draped on the full detail floated over it by up to 2.7 m at step 2 and 4.3 m at step 4 (seed 42, measured
  along every street every 2 m; none at step 1). So each node lays the pieces whose middle lies in it on
  `Heightfield::height_at_step` for its own step, built on the worker with its ground mesh and drawn as its children,
  shown and hidden with it. Streets and car parks on the suburbs' asphalt (Asphalt026C), drives on the kerbs'
  concrete, stall lines with the lane marks' material. A connector sits 5 mm under the street it joins, a drive and a
  curb cut 1 cm under theirs: no shared planes, no z-fighting.
- `small_buildings:` keys (code defaults, `data/world/world.ron` sets none): `street_width_m`, `street_verge_m`,
  `street_break_m`, `connector_every_m`, `street_lift_m`, `drive_reach_m`, `drive_max_grade`, `drive_lift_m`,
  `parking_depth_m`, `stall_width_m`, `stall_depth_m`, `lot_lift_m`, `cut_width_m`, `two_cuts_over_m`.

## 6. Costs (estimates; a cloud session measures nothing)

| What | As built on seed 42 (counted; times are not measured here) |
| --- | --- |
| Suburb streets | 758 pieces and connectors, 85 km; draped in 4 m pieces 2 cells across: about 42,000 quads, 85,000 triangles, split among the ground nodes |
| Drives | 1,326 houses, two strips each in 2 m cells: about 8 quads, 16 triangles a house, about 21,000 triangles |
| Car parks | about 70 businesses, each a car park of about 60 cells, a cut and about 6 stall lines: about 10,000 triangles |
| Draws | up to three a ground node with paving on it (asphalt, concrete, paint), sharing the roads' materials; a node holds a few hundred to a few thousand of the 120,000 triangles |
| Generation | a street sample, connector and drive test per house: to be measured on the owner's machine |
| Houses lost | suburbs 1,281 (1,570 before; the kits grew 3 m wider where they gained a side drive, and about 230 houses had a drive too steep or no connector) |

## 7. Checks

- Every built house has a drive whose far end lies on a street or highway's paved edge.
- Every street piece is joined to a city street or a highway through streets (a graph search over pieces).
- Every business has a parking lot whose curb cut lies on the highway's shoulder.
- No paving crosses another lot's footprint; no drive or street is steeper than its limit.
- No stripe shares a plane with the asphalt under it.
- The same seed gives the same streets twice.
- In the game: stills of a suburb street, a drive meeting it and a town's car park (cloud), then a playtest.

## Open questions

| # | Question | Provisional answer | Who decides |
| --- | --- | --- | --- |
| S6 | Do suburbs get their own streets | Yes (owner, 2026-10-27, the request above) | answered |
| S7 | How are the suburb streets laid out | along the rows with straight connectors toward the city; cul-de-sacs later | provisional (survey S7); built this way |
| S8 | How big are the car parks | one row of stalls across the business's front | provisional (survey S8); built this way |
| S9 | Parked cars in the car parks | not in this change: the kits' own cars stay where they are | provisional (survey S9) |
| S10 | Do cars drive the suburb streets | not yet: traffic keeps to the highways and city streets | provisional (survey S10) |
