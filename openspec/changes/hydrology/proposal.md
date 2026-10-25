# Hydrology: a crater of mountains, snowmelt rivers, lakes and waterfalls

## Why

The owner, 2026-10-25: "streams from the snow on a mountain flows into form a river then the river
goes down a cliff into a waterfall into a lake or another river", "Smooth out the terrain region",
"Get this integrated into the game", then "too many lakes" and "need a crater like moutain range at
center of islan". The game's rivers were random downhill walks from random high spots; nothing
made them start in snow, meet, or fall over anything.

## What changes

- **A crater range** round the island's middle (`range` in `data/world/world.ron`): smooth ridged
  peaks to about 1.6 km on a 5.5 km rim circle, snow above 900 m, a 60 m cliff band each side of the
  rim (inside, the crater's wall), three passes. The capital stands on the crater floor.
- **Drainage** (`cb-core::world::hydrology`): a 128 m grid over the island's shape, drained by a
  priority flood from the sea, with snowmelt and rain summed downstream. Streams start high in the
  snow, meet and widen into rivers; basins a stream feeds fill as lakes (the ten largest).
- **Rivers, lakes and waterfalls** (`cb-core::world::water`): courses laid on the finished ground
  at 16 m steps, a surface that never rises, a waterfall where a river drops over a cliff (12 m in a
  16 m step, 25 m or more in all, within 64 m), a plunge pool under each river's fall. `water_at`
  answers rivers, lakes and the sea; the carve shapes lake bowls and banks and river channels.
- **Biomes** (`cb-core::world::biome`, from `island-biomes`): moisture drying toward the
  east-south-east; snow, rock, montane forest, grassland, savanna, desert (a quarter of the land),
  beach and wetland, every border a band. The ground colour, grass and trees read it.
- **Drawing** (`cb-app::world`): river strips in 4 km tiles, lake surfaces to their shore, each
  waterfall a sheet on gravity's arc with foam where it lands.
- **Cities and roads**: cities stay off the rim, below 260 m on level land, clear of rivers and
  lakes; a highway is not built over the rim (the cities form a spanning forest if they must).
- **`cb-tools map`**: the island drawn from above, for judging it by eye.
