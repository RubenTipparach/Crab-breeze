# Design: airport layout

References: `docs/design/references/airports/` (owner, 2026-10-28). Metres throughout; x and z are the ground plane.

## 1. The airfield's frame

A base has a site (`AirBase::rect`, 700 x 500 m) and a runway on one of four sides (`RunwaySide`), `runway_gap_m`
from the site's edge. The **strip** is the rectangle between the site's edge and the runway's far edge, the
runway's length long. It holds the runway, the parallel taxiway and the links. It is flattened to the site's ground
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
| Apron entries | From the taxiway into the site, at 1/3 and 2/3 of the site's runway-side edge | 30 m wide |
| Terminal | Along the low-x edge of the site, between the pads' row and the hangars' row | 40 x 260 m, 14 m high |
| Boarding bridges | From the terminal's face to each stand | 3 m wide, 25 m long |
| Stands | In front of the terminal, 70 m apart | 4 stands, 60 x 60 m |
| Fire station | Front low-x corner | 30 x 20 m, 9 m high, red doors |
| Fuel farm | High-x edge, mid-depth | 3 tanks, 14 m across, 10 m high |
| Car park | Middle, between the pads and the hangars | 120 x 70 m, bays 2.5 x 5 m |
| Helicopter stand | Beside the tower | circle 24 m across, an H |
| Windsock | Front corner by the runway's `a` end | 8 m mast, 3 m cone |
| Fence | Round the site and strip | 2.4 m posts every 5 m, two rails |

When the runway lies behind the hangars (`Back`), the apron entries come in round the hangar row's ends.

## 3. The runway's markings

All painted on the runway's surface (white) or beyond it (yellow chevrons). For a 2000 x 45 m runway:
- **Thresholds:** existing bars, set 60 m in. The 60 m before each is the blast pad, with yellow chevrons.
- **Designators:** the runway's magnetic heading in tens of degrees (bearing from `a` to `b`, +z north), two digits,
  9 m tall, drawn from segments 120 m from each threshold. They read from the approach side.
- **Touchdown zone:** three pairs of bar groups at 150, 300 and 450 m past each threshold.
- **Aiming point:** two broad bars, 45 x 10 m, 400 m past each threshold.
- **Centre line:** the existing 30 m dashes with 20 m gaps.
- **Edge lines:** the existing lines.
- **Approach lights:** a row of posts every 30 m for 300 m out past each end, each with a crossbar of five lamps.
  Each post stands on the natural ground, its lamps at runway height.
- **PAPI:** four boxes, 9 m apart, 15 m outside the left edge at the aiming point.

The lamps are emissive (no point lights: survey N3 keeps night shadows to the player's own lamps).

## 4. Aircraft on the stands

Each stand holds one parked aircraft, drawn from the civil traffic's stand-in shape table so a parked airliner is the
one that flies (section 6.1 of CLAUDE.md). It is static, not in the traffic's cap and not in the hit list yet. Which
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

## Open questions

| Id | Question | Provisional |
|---|---|---|
| AP1 | Gap of 100 m (taxiway 62 m off the runway) against ICAO's 168 m | 100 m, said in section 1 |
| AP2 | Colliders for the new buildings (a mech walks through them today, as through the tower) | Not in this change; with the hangar doors (`air-bases` 2.2) |
| AP3 | Parked aircraft in the hit list and the standing rule | Not in this change |
