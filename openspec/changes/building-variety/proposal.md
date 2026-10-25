# Proposal: building-variety

## Why

Owner, 2026-10-24, on the building stills: "are we using all kinds of buildings? all i see in
screenshots is the one glass type, there should be many other tpyes, with buyildings that have
normal maps", and before it: "why aren't you utilizing brick buildings".

Measured on seed 42 (`docs/playtests/2026-10-24-buildings-cloud.md`): the game has five facade
kinds (brick walk-up, brick warehouse, ribbon-window office, punched-grid concrete office,
curtain-wall tower). Within 500 m of the start, 38 % of buildings are curtain-wall towers, and
brick only ever goes on buildings of 6 storeys or fewer, so downtown has none. Brick, concrete,
roofing and wood carry normal maps; glass and every trim piece (frames, mullions, sills, cornices)
are flat colour. From the street the city reads as one glass type.

## What changes

- **Six more building types** from the 1980s alt-history city (survey G1), each a facade grammar
  and a material set: pre-war brick loft (7-14 storeys), Art Deco stone setback tower (15-40), 1970s
  precast concrete panel tower (10-30), dark mirror-glass slab (15-45), stone civic or bank block
  (3-6), metal-clad industrial shed and parking deck (2-6). Eleven kinds in all.
- **Style is picked per building from weights in data**, by distance band from the city centre
  (downtown, middle, edge), limited to the storey range each type can carry. Height still comes
  from the layout. Downtown gets brick lofts, stone towers and precast towers beside the glass.
- **Normal maps on every facade surface**: five new PBR material sets (limestone ashlar, cast stone
  and terracotta, precast concrete panel, corrugated metal, polished granite base) from Material
  Maker graphs, and the trim pieces (frames, mullions, sills, cornices, spandrels) get a shared
  painted-metal and stone trim set with normals instead of flat colour.
- The four existing styles keep their grammar; the warehouse and walk-up pick up the stone trim.

## Out of scope

Interiors per type (`building-interiors`), destruction per material (`structural-destruction`
already breaks any element), landmarks and one-off buildings.
