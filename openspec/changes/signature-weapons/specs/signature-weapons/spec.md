## ADDED Requirements

### Requirement: Every roster mech carries its own signature weapon

Each roster mech SHALL carry, in its loadout data, the signature weapon its model carries (the Foreman's boom
cannon, the Driller's cutting lance, the Harrow's twin racks, the Ironhorse's steam cannon, the Mule's rocket ripple,
the Picket's rail rifle, the Roustabout's breaching gun, the Sapper's grenade launcher), and no two roster mechs SHALL
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

A weapon that lobs SHALL lay itself at its fixed elevation and set its muzzle speed for the range and height to the
aim point, within its range limits, so that its shell lands within 1 m of the aim point.

#### Scenario: The Sapper's grenade launcher over a wall
- **WHEN** the aim point is 300 m away, behind a 24 m wall standing 50 m in front of it
- **THEN** the grenade climbs to about 43 m, clears the wall, and lands within 1 m of the aim point about 6 s later
