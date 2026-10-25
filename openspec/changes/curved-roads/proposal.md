# Curved roads: highways routed over the island

## Why

The owner, 2026-10-25: "roads shouldnt be straight like that", "roads between cities I mean", "see
godot-sandbox repo of mine that has curved roads". Highways were straight lines between cities,
graded on an S-curve, cut through hills and blocked by the crater's rim.

## What changes

- A highway is a routed centre line with a grade per point (`Highway::path`, `grade_m`), found the
  way Mining Mike's road crew surveys its corridor (godot-sandbox `scripts/terrain.gd`,
  `road_corridor.gd`, revision 950a46d): A* over the 128 m drainage cells, paying for distance and
  the square of the grade, a stream crossing costing a bridge, the sea, lakes, other cities and
  ground past the road's climb out of bounds.
- The cell path is pulled taut between sight lines (straights of at most 1.5 km), rounded by four
  Chaikin cuts and resampled every 16 m.
- The grade follows the land averaged over 240 m, held level at each city's ground until the road
  is clear of its blend band, and within a 6 % climb between.
- City flats are applied after road grading, so a square stays flat beside a road.
- Bridges are found along every road segment near a river. The drawn road follows the curve.
- The crater's passes are wider and wander, so their walls are not straight cuts.
