## ADDED Requirements

### Requirement: A shotgun throws a cone of pellets
A weapon of kind `pellets` SHALL fire `count` projectiles per pull, spread evenly over the disc of the cone of half-angle `spread_deg` around the aim, drawn from the seeded simulation RNG.

#### Scenario: Point blank and long range
- **WHEN** a shotgun of 12 pellets and 1.5 degrees fires 100 times at a still torso of 3 m radius from 100 m, and 100 times from 400 m
- **THEN** every pellet strikes the torso at 100 m, and at 400 m fewer than 2 pellets a shot strike it on average

### Requirement: A railgun charges before it fires
A weapon of kind `slug` SHALL fire its shot `charge_s` after the trigger is pressed, along the aim at that moment, and SHALL fire nothing and add no heat if the trigger is released before then.

#### Scenario: An early release
- **WHEN** the pilot presses the railgun's trigger and releases it 0.6 s later, with a 1.0 s charge
- **THEN** no slug is fired and the weapon heat bank is unchanged

### Requirement: A laser burns while it is held
A weapon of kind `beam` SHALL, every fixed step its group is held, trace the aim up to `range_m` and apply its damage per second times the step to the first thing it meets, if its penetration beats that part's armour.

#### Scenario: Two seconds on a torso
- **WHEN** a 60-per-second laser with 120 mm penetration is held on a torso of 100 mm armour for 2 s
- **THEN** the torso loses 120 structure points

### Requirement: A plasma ball bursts where it stops
A weapon of kind `plasma` SHALL fly straight with no drop and SHALL explode with its blast energy on contact or after flying `burst_m`, whichever comes first.

#### Scenario: A miss in open air
- **WHEN** a plasma ball with `burst_m` 650 is fired into empty sky
- **THEN** it explodes 650 m from the muzzle with its 4.0 MJ blast

### Requirement: Heat weapons lock when the bank is full
The railgun, the laser and the plasma cannon SHALL add their heat to the mech's weapon heat bank, and while the bank is locked none of them SHALL fire.

#### Scenario: A rail shot into a hot bank
- **WHEN** the weapon heat bank stands at 0.8 and the railgun (0.35 a shot) fires
- **THEN** the bank fills and locks, and neither the railgun, the laser nor the plasma cannon fires until the bank is back under its relight mark of 0.4
