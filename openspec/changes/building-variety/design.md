# Design: building-variety

## 1. What there is today

`cb_core::world::dressing::look_of` picks a style from the storey count and a roll, then colours,
frame colour and window rhythm per building. Measured on seed 42 within 500 m of the start (393
buildings): 150 curtain-wall towers, 81 ribbon offices, 50 punched-grid offices, 79 walk-ups, 33
warehouses. Materials with normal maps: brick, concrete, roof, wood (`crate::textures`). Glass and
`Mat::Trim` are colour only.

## 2. The new types

| Type | Storeys | Facade grammar | Materials |
|---|---|---|---|
| Brick loft | 7-14 | brick piers on the column lines, wide steel-sash windows (3 x 4 lights), stone base 1 storey, stone cornice | brick, cast stone, steel trim |
| Art Deco stone tower | 15-40 | setbacks at 60 % and 80 % of height, vertical piers every bay, recessed windows with spandrel ornament panels, crown | limestone ashlar, cast stone, bronze trim |
| Precast panel tower | 10-30 | one repeated precast window panel per bay (rounded window opening 1.6 x 1.4 m), exposed slab edges | precast concrete panel |
| Mirror-glass slab | 15-45 | flush curtain wall with no visible mullions, black or bronze spandrels, a granite podium | glass (dark coating), granite |
| Civic or bank block | 3-6 | rusticated base, columns or pilasters on the upper storeys, tall windows, a heavy cornice and parapet | limestone ashlar, granite, bronze trim |
| Industrial shed and parking deck | 2-6 | corrugated cladding with ribbon clerestory, or open decks with spandrel rails and ramps | corrugated metal, concrete |

Each type is a facade grammar in `dressing.rs` beside the existing four, picked through one table
(style to grammar function), so no `match` on style spreads across files.

## 3. Picking a style

`data/world/buildings.ron` (planned by `building-generation` task 1.1 and not yet written: today
`look_of`'s rolls, 0.85, 0.7 and 0.85, are inline constants, against CLAUDE.md section 6.4) gets
`style_weights`: for each distance band from the city centre
(downtown 0-0.35, middle 0.35-0.7, edge 0.7-1 of the city's half size) a weight per type. A
building rolls among the types whose storey range holds its storey count; if none does it keeps
today's choice. Provisional downtown weights: glass tower 25, mirror slab 15, Art Deco 15, precast
15, brick loft 15, office 15. A survey answer (W22) changes only these numbers.

## 4. Materials and their cost

Five new PBR sets at 2048 px (base colour, normal, ORM; 4 bytes a texel; with mips 4/3): 2048 x
2048 x 4 x 3 x 4/3 = 67 MB each, 336 MB for five uncompressed. In BC7 (KTX2, 1 byte a texel) 84 MB.
The trim set at 1024 px: 17 MB uncompressed. The RTX 3070 has 8 GB; the budget question is load
time and the PNG files' size in the repository, so the sets ship as KTX2 BC7 (Bevy reads KTX2 with
zstd, enabled by default).

Triangles: the new grammars are comparable to the existing office and tower (a few thousand
triangles a storey a face at the near LOD); measured on the owner's machine before the change is
called done. Dressing stays within 250 m of the player.

## 5. Normal maps on trim

`Mat::Trim` today is a white material tinted by vertex colour. It becomes a textured material with
a painted-metal and stone trim set (normal map of fine dents and edge wear, tiled at 1 m), still
tinted by vertex colour, so every frame, mullion, sill and cornice picks up relief in the light.

## Open questions

| Id | Question | Recommendation | State |
|---|---|---|---|
| BV1 | Which new types, and in which order? (survey W22) | All six; brick loft and Art Deco first (they put brick and stone downtown) | decided (W22) |
| BV2 | Brick above 6 storeys (survey W18) | Yes, through the brick loft type | W18: "see W12" (whatever looks cool and is efficient): the brick loft carries it |
| BV3 | Texture sets compressed to KTX2 BC7? | Yes, for load time and repository size | provisional |
