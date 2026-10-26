## ADDED Requirements

### Requirement: Harbours on the coast by the coastal cities
The world SHALL have a harbour by each coastal city, up to four, each with a quay out to berth depth, cranes,
warehouses and a harbour office whose terminal opens the career's jobs board, kept clear of the air bases.

#### Scenario: A harbour by a coastal city
- **WHEN** the world for seed 42 is generated
- **THEN** each harbour stands within reach of its city, its berth is at least 10 m deep, and its shore is clear of every air base

#### Scenario: The harbour office's terminal
- **WHEN** the pilot on foot stands at the harbour office's terminal and presses Use
- **THEN** the terminal's screen opens on the career's jobs board

### Requirement: Ships on sea lanes
Freighters and tankers SHALL sail sea lanes between the harbours and out past the horizon, keeping clear of the coast,
the islets and each other, and slowing and turning for harbour; fishing boats SHALL work grounds off their harbours and
ships SHALL lie at anchor in each harbour's roads.

#### Scenario: A lane keeps to open water
- **WHEN** the lanes are laid
- **THEN** every point along every lane between its ends is on water deeper than the lane depth

#### Scenario: A freighter slows for harbour
- **WHEN** a freighter on its lane comes within the approach distance of the berth
- **THEN** it slows to its berthing speed and moors at the berth

### Requirement: Boats on the inland water
Rowing boats and launches SHALL move on the lakes, and a ferry SHALL run between a harbour and an islet when the world
has both; while the world has no rivers there SHALL be no river boats.

#### Scenario: A rowing boat stays on its lake
- **WHEN** a lake boat rows its loop
- **THEN** it stays on water deeper than its draught inside its lake

### Requirement: Boats float on the water
A boat SHALL ride the water surface under it, the sea's or its lake's, and heave, pitch and roll on the water's swell,
less for a longer hull.

#### Scenario: A long hull rides steadier
- **WHEN** a rowing boat and a freighter float on the same swell
- **THEN** the freighter heaves and pitches less than the rowing boat

### Requirement: Boats are hit like vehicles and sink
A boat SHALL be hit by rounds as a vehicle is, take damage only from rounds that beat its armour, and sink when
destroyed, leaving a wreck on the bottom and an oil slick on the water.

#### Scenario: A sunk boat
- **WHEN** a boat's hit points reach zero
- **THEN** it lists and goes down over its sinking time and lies as a wreck on the bottom

### Requirement: A mech shoves and swamps small boats
A wading or skimming mech SHALL push small boats out of its way, and swamp one it meets fast enough.

#### Scenario: Swamped
- **WHEN** a skimming mech runs into a rowing boat faster than the swamping speed
- **THEN** the rowing boat sinks

### Requirement: Boats flee from combat
A boat SHALL run from a nearby explosion, a nearby hostile, or a hit, for a time, and then go back to its route.

#### Scenario: A blast nearby
- **WHEN** an explosion goes off within the flee distance of a fishing boat
- **THEN** the fishing boat turns away from it and runs at its best speed

### Requirement: Crabs attack ships
An enemy crab mech SHALL take a crewed ship near it as its target when the pilot is far away.

#### Scenario: A crab and a freighter
- **WHEN** a crab is within reach of a crewed freighter and the pilot is beyond the crab's pilot distance
- **THEN** the crab fires on the freighter

### Requirement: Harm to crewed boats is reported for standing
Harm by the pilot's own fire, blasts or mech to a crewed boat SHALL be reported, once on first harm and once on sinking,
naming the nearest air base; harm by anyone else, or to an uncrewed boat, SHALL NOT be.

#### Scenario: The pilot's round hits a fishing boat
- **WHEN** a round fired by the pilot's mech damages a crewed fishing boat
- **THEN** a harm report names the nearest air base

#### Scenario: A crab sinks a freighter
- **WHEN** a crab's rounds sink a freighter
- **THEN** no harm report is made

### Requirement: Shipping jobs
The jobs board SHALL offer escort, sink-the-gunboats and blockade jobs sited at the harbours, in the mail and at the
harbour office, with the raiders' gunboats as enemy vehicles.

#### Scenario: An escort job
- **WHEN** the pilot accepts an escort job and reaches its site
- **THEN** a freighter sails for the harbour and the raiders' gunboats come for it, and the job fails if it sinks

### Requirement: Boats spawn and despawn round the pilot
Boats SHALL be real only near the pilot, placed by a seeded timetable, so the same seed and pilot path give the same
boats in the same places.

#### Scenario: Far away
- **WHEN** a boat's timetabled place is beyond the despawn distance of the pilot and it is not sinking or a job's ward
- **THEN** it is not in the world
