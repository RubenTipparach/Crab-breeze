# Proposal: distant-shadows

## Why

The sun's live shadows stop at 500 m (three cascades, cut from 900 m for performance), so far hills
and ranges stand in flat light. The owner (2026-10-25): "we also need to fix distant terrain shadows
too, somehow cache that distant shadows", "yea distant terrain shadows can be cached", "then when I
walk up to it within like 500m or so then it can render as normally".

## What changes

- **One bake at load:** the island's terrain shadow for the sun, swept along the sun's direction over
  the whole island into a shadow grid.
- **Far ground darkened:** far terrain chunks (drawn coarser than full detail, beyond 700 m) darken
  their colour where the grid says the terrain shades them, so ridges cast long shadows into the
  valleys across the island.
- **Near ground unchanged:** live cascaded shadows, as now.
- **No cost a frame:** the shade is in the far chunks' vertex colours; the GPU does nothing new.
