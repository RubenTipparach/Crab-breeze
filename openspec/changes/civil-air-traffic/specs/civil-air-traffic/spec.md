## ADDED Requirements

### Requirement: Airliners and freighters fly between the air bases
Route aircraft SHALL take off along a strip on a base's apron, climb out, cruise toward another base, join the strip's
extended centre line, descend on the glide path and land on that strip, stopping before it ends; they SHALL keep at least
the route clearance over the ground on the way, climbing in a circle when the ground ahead is higher than a straight climb
can clear.

#### Scenario: A flight across the island
- **WHEN** an airliner takes off from one base of the shipped seed-42 island for the next
- **THEN** it lifts off before its strip ends, never comes within half the route clearance of the ground en route, and
  touches down lined up on the destination base's strip and stops on it

#### Scenario: Arriving too high
- **WHEN** a route aircraft reaches the threshold more than 20 m above the strip or off its centre line
- **THEN** it goes round again instead of landing

### Requirement: Light planes and helicopters circle the cities
City aircraft SHALL circle a city's centre at their radius and height over the ground.

#### Scenario: A news helicopter over a city
- **WHEN** a helicopter has circled its city for a minute
- **THEN** it stays within a quarter of its radius of its circle and within 15 m of its height

### Requirement: Civil traffic exists only near the pilot and appears unseen
Civil aircraft SHALL be spawned only beyond `spawn_far_m` of the pilot, or beyond `spawn_near_m` and outside the pilot's
view cone; SHALL be removed beyond `despawn_m`; SHALL never number more than each kind's `max_near`; and SHALL be chosen
from seeded randomness, so a seed flies the same traffic.

#### Scenario: Looking north at the start
- **WHEN** traffic runs for ten minutes with the pilot looking one way
- **THEN** every aircraft appeared where the pilot could not see it appear, and all four kinds flew

#### Scenario: Leaving the island
- **WHEN** the pilot is 60 km out to sea
- **THEN** no civil aircraft remains

### Requirement: Civil aircraft keep clear of a fight
Every civil aircraft SHALL keep out of the circle of `keep_clear_m` round every live enemy; a route whose destination base is
in a fight SHALL divert to the nearest base that is not; a city aircraft whose city is in a fight SHALL leave; and none SHALL
be spawned in a fight, to a base in one or to circle a city in one.

#### Scenario: A fight on the way
- **WHEN** a fight lies on an airliner's straight way to its base
- **THEN** it bends round the fight without entering it and still lands

#### Scenario: The destination is attacked
- **WHEN** a freighter's destination base comes under attack
- **THEN** it diverts and lands at a quiet base

### Requirement: Civil aircraft can be shot down, by crabs or by the pilot
Civil aircraft SHALL be hit through the shot sweep and the single-value armour rule the invasion's vehicles use; a downed
aircraft SHALL fall with its momentum, explode on the ground through the blast rules and leave its wreck as debris; an
invasion vehicle SHALL take a shot of opportunity at a civil aircraft in its guns' range and cone when the pilot is not its
shot.

#### Scenario: A shot too weak for the armour
- **WHEN** a round that does not beat a light plane's armour hits it
- **THEN** it takes no damage

#### Scenario: Shot down at cruise
- **WHEN** an airliner at 1 000 m is destroyed
- **THEN** it falls, carries on several hundred metres the way it was flying, and crashes

### Requirement: The pilot's hits on civil aircraft are reported per base
Every hit on a civil aircraft SHALL be sent as a message naming the aircraft, the air base nearest the hit, whether it was
downed and whether the pilot fired it; the pilot's hits SHALL be tallied per base.

#### Scenario: Two hits near base 2
- **WHEN** the pilot hits an aircraft twice near base 2, downing it, and once near base 0
- **THEN** base 2's tally reads two hits and one downed, and base 0's one hit

### Requirement: Civil aircraft are never selected or locked
Targeting SHALL never select a civil aircraft, by the pick nearest the crosshair, by the cycle key or by a selection kept
from an earlier step, so no missile lock SHALL build on one; only unguided fire SHALL hit one (owner, 2026-10-26, survey
W47). The cockpit SHALL box civil aircraft in a neutral colour marked `CIV`, as information.

#### Scenario: An airliner on the crosshair, an enemy beside it
- **WHEN** the crosshair rests on an airliner with an enemy mech a few degrees off
- **THEN** the enemy is selected, and pressing the cycle key goes to the enemy, never the airliner

#### Scenario: An airliner alone on the crosshair
- **WHEN** the crosshair is held on an airliner with no enemy in range, with or without the cycle key
- **THEN** nothing is selected and no lock builds
