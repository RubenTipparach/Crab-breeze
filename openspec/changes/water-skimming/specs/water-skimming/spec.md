## ADDED Requirements

### Requirement: Skimming on skate power
A mech at the surface of water SHALL ride the surface and not sink while it has skate power, unless the pilot holds Ctrl.

#### Scenario: Skating onto deep water
- **WHEN** a skating mech at speed runs from the shore onto water 30 m deep
- **THEN** it stays at the surface and keeps its speed

### Requirement: Rockets hold the mech up when the skates are empty
On deep water with the skate budget empty, the rockets SHALL fire by themselves to hold the mech at the surface until their budget is empty too.

#### Scenario: Skates run dry at sea
- **WHEN** the skates lock out while the mech is skimming deep water
- **THEN** the mech stays at the surface on its rockets and the rocket budget burns

### Requirement: Sinking and recharging
With both budgets empty, or with Ctrl held, a mech on deep water SHALL sink to the bottom and walk there slowly while its budgets recharge, and SHALL rise again on its rockets when the pilot holds boost.

#### Scenario: Ctrl forces a sink
- **WHEN** the pilot holds Ctrl while skimming water 30 m deep
- **THEN** the mech sinks to the bottom

#### Scenario: Recharged on the bottom
- **WHEN** a sunk mech waits on the bottom for 15 s and the pilot then holds boost
- **THEN** the mech rises to the surface

### Requirement: Ctrl dives from the air
Holding Ctrl in the air SHALL make the mech fall faster than it falls without it.

#### Scenario: Diving
- **WHEN** a mech 300 m up falls for 3 s with Ctrl held and another falls for 3 s without
- **THEN** the diving mech has fallen further
