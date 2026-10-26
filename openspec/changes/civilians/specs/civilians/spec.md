## ADDED Requirements

### Requirement: People live in the cities and can be greeted
The cities' pavements SHALL have people walking near the pilot, and a person greeted on foot with Use SHALL stop, face the pilot and answer.

#### Scenario: Greeting a passer-by
- **WHEN** the pilot walks up to a person on a pavement and presses Use within 2.5 m
- **THEN** the person stops, turns to the pilot and a line of theirs is shown

### Requirement: Use offers every interaction in reach and does the one selected
When more than one interaction is in reach (greeting a person, a terminal, a car, the mech), the prompt SHALL list them,
the mouse wheel SHALL move the selection, and Use SHALL do the selected one only; the greeting SHALL be selected first;
with one in reach Use SHALL do it (survey W40, 2026-10-26: "You should be able to scroll wheel select multiple
options").

#### Scenario: A person beside the stopped mech
- **WHEN** a person stands within 2.5 m in front of the pilot and the stopped mech's rope is in reach
- **THEN** the prompt lists Greet, marked, and Step into the cockpit; Use greets, and after one turn of the wheel Use
  boards

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
