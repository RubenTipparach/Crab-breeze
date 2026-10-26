## ADDED Requirements

### Requirement: Roads and pavements are drawn with CC0 PBR sets
City streets SHALL be drawn with the clean asphalt set, arterials, highways and the air bases' runways and taxiways with the worn asphalt set, and pavements with the paving slab set; each set SHALL be a full PBR set (colour, normal, occlusion-roughness-metal) loaded with a full mip chain, its tile size read from `road_surfaces` in the world data.

#### Scenario: A city street on foot
- **WHEN** the pilot stands in a city street
- **THEN** the asphalt shows its aggregate at real scale, lit through its normal and roughness maps, and the pavement shows its slabs

### Requirement: The road's tone varies over tens of metres
A road's colour SHALL be its kind's tone times a factor from smooth noise of its world position, at most `tint_amount` from 1, continuous across the road, so a tile's repeat does not line up from the cockpit.

#### Scenario: Two points far apart on one street
- **WHEN** the tint is read at two points 100 m apart
- **THEN** both are within `tint_amount` of the tone, and two points 10 cm apart differ by almost nothing
