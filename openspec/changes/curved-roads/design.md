# Curved roads design

## Route

- Grid: the drainage grid, 128 m (`Drainage::ground`, `stream`).
- Step cost: `run * (1 + 6 * (grade / 0.06)^2)`, plus 600 m for a stream cell.
- Out of bounds: ground under sea level plus 2 m, above the higher city plus 45 m, within a lake's
  radius plus 88 m, within 282 m of a third city's square (its blend, the road's band and half a
  cell), outside a box 4 km round the ends.
- Heuristic: straight distance (admissible: every step costs at least its run).
- Prim's tree on straight distances; a pair with no route is refused and not tried again.

## Shape and grade

- Taut: from each kept point, the farthest later one whose straight line crosses only passable
  cells, at most 1.5 km away. Then four Chaikin cuts, resampled at 16 m.
- Grade: land averaged over 240 m each side; level at each city's ground until 282 m clear of its
  square (at least `level_end_m`), easing to the land over 240 m; between, clamped to 6 % both ways
  four times with the level stretches pinned.
- `road_lift_m` 0.15 m (was 0.1): the 8 m ground triangles sag up to 9 cm under a climbing bend.

## Costs

- Generation: 23 A* searches bounded by their boxes, a few ms each.
- `Highway::project` uses a 256 m cell index of its segments: a lookup and a few segment tests.
- Draw: the same one road mesh; seed 42's highways are 69 km (were 71 km straight).

## Open questions

| Id | Question | Provisional |
|---|---|---|
| R1 | Junctions where two roads meet outside a city? | Roads meet only at cities |
