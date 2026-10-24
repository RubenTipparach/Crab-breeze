# Proposal: building-destruction

## Why

Owner, 2026-10-24: "Fully destructible environment ... all physics drive". Buildings are solid boxes
that nothing can harm. A mech fight in a city should leave holes in walls, floors falling in, and
towers coming down.

## What changes

- Every building is a grid of cells: about 6 m by 6 m by one storey (3.5 m). A cell is intact or
  gone. Shells and blasts break the cells they reach; a mech walking into a wall at speed breaks
  the cells it hits (and slows).
- Collapse: when the cells holding up a storey are mostly gone (under 35 % of a storey's footprint
  left under a part of the building), everything above that part falls: it becomes physics debris
  (`debris-physics`) in storey-sized chunks, which breaks into smaller chunks when it lands.
- The building is drawn from its intact cells (rebuilt when cells break, faces between intact
  cells skipped), and collision uses the intact cells at the body's height, so a mech can walk
  through a hole it blew.
- Deterministic in `cb-core`: the same hits break the same cells.

## Out of scope

Interiors, glass, rubble you can walk on as collision (debris is not a wall yet), fire spreading.
