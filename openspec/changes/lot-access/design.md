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

- **The street runs along the row line**, offset toward the city by a front yard (`street_yard_m`, 6 m) plus half
  the street (`street_width_m`, 8 m: two 3.5 m lanes and a 0.5 m gutter each side). It follows the same wander as
  the row, sampled every 4 m.
- **Only where houses stand.** A row's built lots are walked in order; a street piece starts 10 m before the first
  and ends 10 m past the last, and breaks where two neighbours are more than `street_break_m` (60 m) apart.
- **Every piece is joined.** A connector street runs from a piece straight toward the city's square until it meets
  the next row's street or the city's outer street (the city's grid starts and ends with a street, so its edge is
  one). It is tried from the piece's middle, then its ends, then every 20 m between. A connector may not cross a
  lot; a lot it would cross is not built if that is the only way. A piece that can't be joined is dropped with its
  houses. A piece a highway crosses is joined there too, a flat junction.
- **Rows move out to fit**: `suburb_row_spacing_m` from 36 to 40 m (house depth up to 13 m, yard 6, street 8,
  back yards about 6 m each side of the boundary).
- **Levelled like the highways.** The street's height is the ground under it smoothed along its length (a 40 m
  window), and the ground is levelled to it across the street and its 1.5 m shoulders, blending over 6 m, by the
  same rule the highways use (`Ground`'s road levelling), before the lots' pads. A piece steeper than 12 percent
  anywhere is dropped with its houses.

## 3. Driveways

- **Each house kit records its drive** in `pieces.json`: the midpoint and width of the drive's front edge in the
  kit's frame (the ranch's carport slab, the colonial's garage apron, and so on). Kits with no drive get one in the
  mockup: a 3 m concrete strip (gravel for the trailer, adobe and stilt house) beside the house on its open side,
  with the kit's car on it where it already has one. The checker fails a house kit with no drive.
- **The game paves from the kit's drive to the street's near edge**: a straight strip of the drive's width, its
  ends at the pad's height and the street's height, its middle draped on the levelled ground. Where it meets the
  street the curb is cut: the strip widens to the drive's width plus 1.5 m each side over the last 1.5 m.
- **A drive steeper than 12 percent** (`drive_max_grade`) drops its lot; so does a drive that would cross another
  lot.
- **Town houses** on the highway get the same drive to the highway's shoulder.

## 4. Parking lots

- **Businesses stand back** from the highway's shoulder by `parking_depth_m` (12.5 m: a row of 90 degree stalls
  5.5 m deep against the building, and a 7 m two-way aisle) instead of `setback_m` (4 m, kept for houses).
- **The lot** is the kit's front width plus 2 m each side, from the building's front to the shoulder, paved
  asphalt. Stalls are 2.6 m wide (`stall_width_m`), painted white lines 0.1 m wide and 5 m long. One curb cut 7 m
  wide where the lot meets the shoulder, two on a frontage over 30 m (one at each end, so a car drives through).
- **The gas station's apron** already runs in front of its pumps: its lot has no stalls and two cuts, one at each
  end. The drive-in's lane joins the lot.
- **Neighbouring businesses** in a town share one continuous lot when their lots are within 8 m; the gap is
  paved.

## 5. Data and drawing

- `cb_core::world::paving::Paving`, built in world generation after the lots: `streets` (a centre line, width and
  grade per piece, like `Highway`), `slabs` (drives, aprons and parking: a quad of corners each with a height, and
  a surface: asphalt, concrete or gravel) and `stripes` (quads 0.1 m wide, paint). Pure and deterministic.
- `Ground`'s road levelling takes the streets with the highways: one rule for every road's ground.
- `cb-app::world::spawn_roads` adds them to the `roads` mesh (streets and parking on asphalt), with a second mesh
  for the concrete and gravel (the house concrete set, `tile_m` from the materials row), stripes lifted 1 cm over
  the asphalt (no shared planes, no z-fighting) on a white tint of the asphalt texture. Two draws in all, up from one.
- `small_buildings:` in `data/world/world.ron` gains `street_width_m`, `street_yard_m`, `street_break_m`,
  `street_max_grade`, `drive_max_grade`, `parking_depth_m`, `stall_width_m`, each with a code default.

## 6. Costs (estimates; a cloud session measures nothing)

| What | Estimate |
| --- | --- |
| Suburb streets | about 1,570 houses at about 28 m of frontage: 44 km of street, plus about 10 km of connectors; 54 km at 4 m segments, 13,500 quads, 27,000 triangles |
| Drives | one strip and a curb flare per house: about 6 triangles, 9,400 triangles |
| Parking | about 130 businesses, a slab and about 8 stripes each: about 2,400 triangles |
| Draws | the roads mesh grows; one more draw for concrete and gravel |
| Generation | placing streets and connectors, levelling and paving: guessed 10-30 ms on the owner's machine, to be measured |
| Houses lost | lots dropped as unreachable, too steep or in a connector's way: reported in the log and the test, expected under 10 percent |

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
| S7 | How are the suburb streets laid out | along the rows with straight connectors toward the city; cul-de-sacs later | provisional (survey S7) |
| S8 | How big are the car parks | one row of stalls across the business's front | provisional (survey S8) |
| S9 | Parked cars in the car parks | not in this change: the kits' own cars stay where they are | provisional (survey S9) |
| S10 | Do cars drive the suburb streets | not yet: traffic keeps to the highways and city streets | provisional (survey S10) |
