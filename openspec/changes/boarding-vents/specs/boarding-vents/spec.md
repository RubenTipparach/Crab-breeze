## ADDED Requirements

### Requirement: Every mech vents steam and mist as it opens and closes

Every roster mech's boarding timeline SHALL carry the steam and mist vents of its own model (its glb's `VENT_*`
empties and its interior's), written by the generator from the models' extras, and the game SHALL play a vent while
the boarding clock moves through its window, and while it rests there only if the vent holds.

#### Scenario: The seals hiss as the chest opens
- **WHEN** the Harrow sits and its boarding clock passes through its seals' window
- **THEN** steam streams from each of its `VENT_seam_*` empties

#### Scenario: A sealed mech stays quiet
- **WHEN** a mech rests sat and sealed with its clock inside the seals' window
- **THEN** no seal vent emits

#### Scenario: Mist lingers while it stands open
- **WHEN** the chest is fully open and the clock rests at `ready_s`
- **THEN** mist keeps rolling from the sill's vents and the cabin's floor vents

### Requirement: Only the mech the pilot boards vents

Survey M12, decided by the owner on 2026-10-26 ("Approve all": "Only the mech you board vents"). The game SHALL stream
boarding vents only from the mech the pilot is boarding, sitting in or leaving; every other mech, owned or enemy,
SHALL stay quiet whatever its own boarding clock says. No enemy pilot boards a mech.

#### Scenario: An owned mech parked nearby stays quiet
- **WHEN** the pilot boards the Foreman while an owned Mule stands sat and open beside it
- **THEN** steam and mist stream from the Foreman's vents only

### Requirement: A stage that only vents does not change when the mech is sealed

The timeline time of the sat and sealed mech SHALL be the start of the first stage that moves a node, no earlier
than the end of the sit, whatever vent-only stages come before it.

#### Scenario: The legs vent from the start
- **WHEN** a timeline has a vent-only stage at 0 s and its first chest motion at 3.55 s
- **THEN** the sat and sealed time is 3.55 s
