## ADDED Requirements

### Requirement: People live in the cities and can be greeted
The cities' pavements SHALL have people walking near the pilot, and a person greeted on foot with Use SHALL stop, face the pilot and answer.

#### Scenario: Greeting a passer-by
- **WHEN** the pilot walks up to a person on a pavement and presses Use within 2.5 m
- **THEN** the person stops, turns to the pilot and a line of theirs is shown

#### Scenario: Danger
- **WHEN** a shell lands 30 m from a group of people
- **THEN** they run away from it
