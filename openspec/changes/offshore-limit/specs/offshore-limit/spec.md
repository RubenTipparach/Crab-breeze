## ADDED Requirements

### Requirement: The pilot cannot leave far off the coast
The game SHALL fail the active mission, or the free-roam session, with `Loss::OutOfBounds` once the pilot has been
past `min(coast + limit_offshore_m, world edge - edge_margin_m)` for `grace_s`, whatever they are in.

#### Scenario: Fifteen kilometres out
- **WHEN** a mech stands 15.1 km past the coast, more than 1 km inside the world edge, for over a second
- **THEN** the mission fails with `OutOfBounds`, once

#### Scenario: The edge comes first
- **WHEN** a mech is 9 km past the coast and 0.9 km from the world edge
- **THEN** the mission fails with `OutOfBounds`

#### Scenario: A warning before the line
- **WHEN** the pilot is within `warn_m` of the limit
- **THEN** the cockpit alarm sounds and the HUD shows the distance left

### Requirement: A failed pilot respawns in a city
After an `OutOfBounds` loss the game SHALL put the pilot, seated, in their mech, repaired, at a drop site in the
nearest city, with look input the player's throughout.

#### Scenario: Back on land
- **WHEN** the respawn finishes
- **THEN** the mech stands on dry land inside a city and the pilot is seated in it
