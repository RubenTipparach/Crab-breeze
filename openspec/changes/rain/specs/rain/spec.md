## ADDED Requirements

### Requirement: Rain falls around the pilot under a storm
Inside a storm's rain shaft, rain streaks SHALL fall around the eye with splashes on the ground near it, and the amount SHALL fade with the distance from the shaft's middle.

#### Scenario: Walking into a storm
- **WHEN** the pilot walks from clear ground into a rain shaft
- **THEN** streaks and splashes grow from none to full over the shaft's rim, and the rain sound rises with them

#### Scenario: Rain everywhere
- **WHEN** Weather is set to Rain everywhere in Options
- **THEN** rain falls around the eye anywhere below the cloud base
