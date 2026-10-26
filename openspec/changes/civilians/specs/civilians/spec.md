## ADDED Requirements

### Requirement: People live in the cities and can be greeted
The cities' pavements SHALL have people walking near the pilot, and a person greeted on foot with Use SHALL stop, face the pilot and answer.

#### Scenario: Greeting a passer-by
- **WHEN** the pilot walks up to a person on a pavement and presses Use within 2.5 m
- **THEN** the person stops, turns to the pilot and a line of theirs is shown

#### Scenario: Danger
- **WHEN** a shell lands 30 m from a group of people
- **THEN** they run away from it

### Requirement: People run from danger and the pilot's toll is counted
People SHALL run from rounds, blasts and mechs' feet near them, and a person killed by the pilot's own mech SHALL be counted against the air base nearest where it happened, for the career's standing.

#### Scenario: Crushed under the pilot's mech
- **WHEN** the pilot's mech puts a foot down within 4 m of a person
- **THEN** the person falls, and one death is counted against the nearest base

#### Scenario: The enemy's fire
- **WHEN** an enemy's shell kills a person
- **THEN** the death is counted apart and costs the pilot nothing
