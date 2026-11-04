## ADDED Requirements

### Requirement: A city grows in districts

A generated city SHALL be made of districts (old town, downtown, courtyard blocks, rowhouses, estate, port and works,
suburbs), each with its own grid angle, block size, street width and block filler, joined by a ring road, avenues and,
where there is one, a river with bridges.

#### Scenario: Two districts side by side
- **WHEN** the pilot stands where downtown meets the courtyard blocks
- **THEN** the streets on each side run at different angles and the buildings differ in height, footprint and form

### Requirement: A block crossed by an avenue is cut, not dropped

A district block that an avenue, the river or the rail would cross SHALL be replaced by the smaller blocks that fit
on either side.

#### Scenario: An avenue through the courtyard blocks
- **WHEN** an avenue runs through a courtyard district
- **THEN** the blocks beside it are smaller and still built, with no gap left where whole blocks would not fit
