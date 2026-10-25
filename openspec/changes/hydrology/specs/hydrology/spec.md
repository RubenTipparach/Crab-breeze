## ADDED Requirements

### Requirement: Rivers start in the snow and meet
Streams SHALL begin where the drained snowmelt and rain pass the stream threshold, and a stream that reaches another river SHALL end on that river's line.

#### Scenario: Seed 42 has snow-fed rivers
- **WHEN** seed 42 is generated with the shipped data
- **THEN** it has rivers, and every river's surface never rises downstream

### Requirement: Rivers fall over cliffs into pools
Where a river's surface drops at least `fall_step_m` per step within four steps for at least `fall_min_height_m`, the world SHALL record a waterfall from a lip to a foot, and a river at least twice a stream's width SHALL have a plunge pool at the foot's level.

#### Scenario: The crater wall has waterfalls
- **WHEN** seed 42 is generated
- **THEN** it has waterfalls, each at least 25 m high

### Requirement: Few lakes, fed by streams
Basin lakes SHALL be fed by a stream and number at most `lake_max_count`; lake water SHALL be reported by `water_at` inside the shore.

#### Scenario: Lake water is at its level
- **WHEN** `water_at` is asked at a lake's centre
- **THEN** the surface is the lake's level

### Requirement: A quarter of the land is desert
The biome rule SHALL make desert the strongest biome on 20 to 30 percent of the land.

#### Scenario: Seed 42's desert share
- **WHEN** biomes are sampled every 160 m over seed 42's land
- **THEN** desert is strongest on 20 to 30 percent of it
