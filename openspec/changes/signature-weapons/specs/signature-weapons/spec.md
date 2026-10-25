## ADDED Requirements

### Requirement: Every roster mech carries its own signature weapon

Each roster mech SHALL carry, in its loadout data, the signature weapon its model carries (the Foreman's boom
cannon, the Driller's cutting lance, the Harrow's twin racks, the Ironhorse's steam cannon, the Mule's rocket ripple,
the Picket's rail rifle, the Roustabout's breaching gun, the Sapper's siege mortar), and no two roster mechs SHALL
carry the same signature.

#### Scenario: Boarding the Harrow
- **WHEN** the pilot sits in the Harrow and opens the WEAPONS page
- **THEN** the first station is the twin racks, starred, with its locks held out of twelve

### Requirement: A burst fires its rounds from one pull

A weapon with a `burst` SHALL fire its count of shots from one pull, the given gap apart, each along the aim at its
own moment, and its interval SHALL run from the last shot.

#### Scenario: The Foreman's boom cannon
- **WHEN** the pilot pulls the boom cannon's trigger once
- **THEN** three shells leave 0.2 s apart and the next burst is ready 4.0 s after the third

### Requirement: A lobbing gun lands on the aim point

A weapon that lobs SHALL lay itself at 45 degrees and set its muzzle speed for the horizontal range to the aim point,
within its range limits, so that on flat ground its shell lands within 1 m of the aim point.

#### Scenario: The Sapper's mortar over a tower
- **WHEN** the aim point is 700 m away behind a 120 m building
- **THEN** the shell climbs to 175 m and lands within 1 m of the aim point 12 s later
