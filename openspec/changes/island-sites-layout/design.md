# Design: island-sites-layout

## 1. Order of placement

`world::generate` today: cities, highways, `bases::place_bases`, plateaus, `place_airport_counted`, `place_seaport`.
New order, each step keeping clear of everything before it:

1. Cities and highways (unchanged).
2. **Civil airports**, `sites.airports` of them (2). `place_airport_counted` runs once per airport with the earlier
   airports added to its `Clear`. Two airports keep at least `airport_spacing_m` (8,000 m) between centres, and each
   runway's approach cone (below) stays off the other airport's footprint.
3. **Seaports**, `sites.seaports` of them (2). `place_seaport` runs once per port; a later port must sit on a shore
   whose outward normal is at least `port_coast_apart_deg` (90 degrees) from every earlier port's, so they are on
   different coasts.
4. **Military bases**, `bases.count` of them (4), placed by `place_bases` with every civil site in its keep-clear.
5. Plateaus (unchanged).

If a site does not fit, the generator logs which one and why (`Rejections`, as the airport does now) and the world
is still made; a test on the shipped seed holds that all seven fit.

## 2. Approach cones

A runway's approach is a trapezoid off each end: 3,000 m long, the runway's width at the threshold opening to 600 m,
rising at 3 degrees (a real ILS glide path). No other runway's footprint, no tower, and no building taller than the
cone's height at that point may stand in it. Cities already keep 300 m from site footprints; inside a cone a city's
tall blocks are capped by the cone (the city generator reads the cones as a height limit).

## 3. Access roads

Each site template carries a `gate` mark. After the highways are laid, a road is routed from the gate to the nearest
highway point by the highway router's own cost (slope, water, cities), 2 lanes, joined into `LaneGraph::build`. Road
traffic then reaches every site, and cars pick airports and ports as destinations at a small share
(`traffic.site_trip_share`, 0.05).

## 4. Cost

Placement is generation-time only: seven placers instead of seven-with-five-bases, under 50 ms more on the shipped
seed (measured in the test). Two more site templates are drawn (an airport is about 2,000 draw calls at full detail
with its parked aircraft), but sites stream with their tiles, so the frame cost is that of the nearest site only.

## Open questions

| # | Question | Provisional answer |
| --- | --- | --- |
| L1 | Do both civil airports use the one airport template, or should the second be a smaller regional field? | One template for both; a regional variant later if the owner wants it |
| L2 | Which base goes? | Base 4, crowded between bases 1 and 3; base 2 stays beside civil airport 1 (owner, 2026-10-30), its runway 2.85 km south of and parallel to the airport's, so their approach cones do not cross |
