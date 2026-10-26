## ADDED Requirements

### Requirement: Fog hides targets from the sensors
A target SHALL be boxed and locked only within ranges shortened by the ground fog along the line from the pilot's eye to it, once less than a threshold share of its light gets through that fog, and a lock SHALL build more slowly in step; the fog SHALL be the one the screens draw, and every boxed target, enemy or civil, and the lock SHALL use the same rule (owner, 2026-10-26, survey F2: "Approve all").

#### Scenario: Thin fog changes nothing
- **WHEN** at least the threshold share of a target's light reaches the eye
- **THEN** it is boxed and locks at the clear-air ranges, as fast as in clear air

#### Scenario: A crab in a fog bank
- **WHEN** a target 800 m away sits behind fog that lets almost none of its light through
- **THEN** it is not boxed and no lock builds on it

#### Scenario: A lock through light fog
- **WHEN** a target is in missile range through fog that lets less than the threshold through
- **THEN** the lock takes longer to complete than in clear air
