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
## 5. As built (2026-10-30)

- **The second airport** takes the flattest fitting ground, not the first: on seed 42 the only ground within the
  first airport's 45 m rise is 3.6 km from it, inside the 8 km spacing. So a later airport is tried out to
  `later_airport_reach_m` (16 km) past each city, beside the city farthest from the airports already down, and every
  candidate whose natural rise is within `later_airport_max_rise_m` (90 m) is kept; the flattest whose cones clear is
  taken and levelled like any site. A nine-point pass (corners, edges' middles, middle) turns down a candidate off dry
  land or too steep before the full sampling, and a spot tried round two cities is tried once: placing every site
  costs nothing measurable on top of generation (`cb-tools sites 42`: 1 m 45 s before, 1 m 40 s after, release).
- **Seed 42**: airport 1 (-5780, 7686) where it was; airport 2 (1722, -9163) in the north, 59 m of rise levelled;
  seaport 1 (-11319, 3714) on the west coast where it was; seaport 2 (122, -12518) on the north coast; bases 0, 1, 2
  where they were; base 3 (405, -11029), moved 1 km to clear the new sites. The approved page drew the second
  airport and seaport in the east by eye; the generator's rules put them in the north.
- **Approach cones** are `world::approach`, one implementation the sites and the bases share: an airport turns down a
  spot whose cones cover a base, another site or a building rising into them, or that stands under an earlier cone;
  the bases keep out of the civil cones through `KeepClear::cones`.
- **Access roads**: the gate is the footprint's point nearest the site's city; `roads::lay_access` routes and grades
  the road as a highway from the city's edge (`lay_highway` and it are one `lay_road`), its second city
  `roads::SITE_END`, and the lane graph ends it at a gate node of its own, where cars turn round. Every access road
  is lit like a highway. Cars do not yet pick sites as destinations (`site_trip_share` is left for traffic's own work).

## Open questions

| # | Question | Provisional answer |
| --- | --- | --- |
| L1 | Do both civil airports use the one airport template, or should the second be a smaller regional field? | One template for both; a regional variant later if the owner wants it |
| L2 | Which base goes? | Base 4, crowded between bases 1 and 3; base 2 stays beside civil airport 1 (owner, 2026-10-30), its runway 2.85 km south of and parallel to the airport's, so their approach cones do not cross |
