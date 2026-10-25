## ADDED Requirements

### Requirement: Repairs are paid part by part and stop when the money runs out

A mech stopped in a repair bay SHALL be repaired one part at a time in a fixed order, each part
charged through the career when it completes, and the repair SHALL stop at the first part the career
cannot pay for.

#### Scenario: Short of money
- **WHEN** the torso and both arms are damaged and the pilot can pay for only the torso
- **THEN** the torso is repaired and charged, and both arms stay damaged

### Requirement: A bought mech is parked in a free bay

Buying a mech SHALL charge its price and park it at full structure in the lowest free bay of the
current base, and SHALL be refused, with nothing charged, when no bay is free.

#### Scenario: Hangar full
- **WHEN** all six bays of the current base hold mechs and the pilot buys another
- **THEN** nothing is charged and the catalogue says no bay is free

#### Scenario: Job pay buys a mech
- **WHEN** the pilot wins a job, its pay brings the money to at least the Driller's price, and they buy the Driller
- **THEN** the money falls by the Driller's price and the Driller stands in a free bay

### Requirement: Switching mechs is boarding

Boarding a parked mech SHALL make it the piloted mech, and the mech left behind SHALL stay parked in
its bay with its damage.

#### Scenario: Taking the Driller out
- **WHEN** the pilot climbs out of the Foreman in bay 1 and boards the Driller in bay 2
- **THEN** the Driller is the piloted mech and the Foreman stands in bay 1 with its damage
