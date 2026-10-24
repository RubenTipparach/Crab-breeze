# Design: building-destruction

- Cells: `nx = ceil(w / 6)`, `nz = ceil(d / 6)`, `ny = storeys`; a bitset per building (a 60-storey
  30 x 30 m tower is 1 500 cells, 188 bytes). Seed 42's 1 226 buildings: about 400 k cells.
- A blast of radius `r` breaks cells whose centre is within `r * 1.2`; a 35 mm round breaks one cell
  in four hits (cell structure in data), a 120 mm shell a 7 m sphere.
- Support: a cell stands if the cell below it stands, or it is on the ground floor. When a hit
  leaves a column of cells unsupported, the cells above fall as chunks: one chunk per storey per
  2 x 2 cell group, given the building's material colour.
- Mesh: per building, quads for every face of an intact cell whose neighbour is gone or outside.
  Rebuilt on change. An intact building is one box (12 triangles).
- Collision: a body overlaps a building only through the intact cells within its height range.

Cost: rebuilding a 1 500-cell mesh is about 0.3 ms; chunks are Avian bodies (debris-physics).

## Open questions

| Id | Question | Recommendation | State |
| --- | --- | --- | --- |
| W9 | How far does destruction go? | every building can be holed and brought down; roads and terrain craters as now | provisional (survey W9) |

## As built (2026-10-24)

- Any shot that reaches a cell breaks the cells within 3.4 m (more for a blast); a mech that is stopped by a wall at over 2.5 m/s smashes the cells in front of it.
- Collision uses standing cells only for damaged buildings; a whole building is still one footprint.
