## ADDED Requirements

### Requirement: Levels of detail cross-fade and drawn things fade out
Where a thing in the world changes level of detail, the two levels SHALL be drawn across a ring of distance through
complementary screen-door dither masks, and where a thing stops being drawn it SHALL dither out over the last stretch
of its range, never switching in one frame. Every range is built by one helper from distances and fade widths in data.

#### Scenario: A tree tile crosses 600 m
- **WHEN** the eye moves from 630 m to 570 m from a tree tile
- **THEN** across that ring the tile's far trees dither out as its full trees dither in, each pixel showing one, and
  no frame shows the tile switch whole

#### Scenario: A car at the edge of the traffic range
- **WHEN** the pilot drives away from a parked car until it is 1.2 km away
- **THEN** the car dithers out over its last 100 m and is dropped only once fully faded
