## ADDED Requirements

### Requirement: Smoke meets the world softly
A blended smoke, dust, steam, mist or trail particle SHALL fade out over a distance before a surface
behind it, so no straight cut shows where it crosses the ground or a wall.

#### Scenario: An explosion's smoke on the ground
- **WHEN** a ground explosion's smoke is seen from the seat at 15 m and at 450 m
- **THEN** no sprite shows a straight edge along the ground

### Requirement: Overlapping smoke keeps its order
Overlapping blended smoke particles SHALL draw so that a nearer one is never covered by a farther one in
a way that flips as the view turns.

#### Scenario: A jet trail crossing the view
- **WHEN** an enemy's jet trail at 120 m is seen from two headings 20 degrees apart
- **THEN** the trail's puffs layer the same way in both frames, with no shimmer between them
