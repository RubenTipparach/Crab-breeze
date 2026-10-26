## ADDED Requirements

### Requirement: A purchase the money cannot cover changes nothing

Buying a mech SHALL charge its price through the career's ledger, and a purchase short of money, or with neither a
landing site nor a free bay, SHALL be refused with nothing charged and nothing delivered.

#### Scenario: Short of money
- **WHEN** the pilot has 1 000 000 credits and buys the Harrow at 1 200 000
- **THEN** the shop says SHORT 200 000 and the money, the mechs and the bays are unchanged

#### Scenario: Nowhere to put it
- **WHEN** no good ground lies within the search and every bay of every base is full
- **THEN** the shop says NO FREE BAY and nothing is charged

### Requirement: A bought mech is air dropped near the pilot

A bought mech SHALL be air dropped on good ground near where the pilot is when they buy it (owner, 2026-10-26: "when
a player orders a mech it should be air dropped close to wherever the player is on a good are on the ground (not stuck
on a building)"), superseding survey J16's bay delivery. The site SHALL lie inside the ring round the pilot whenever
the ring holds good ground, and SHALL never overlap a building, a hangar or a rock with the mech's footprint, lie in
water deeper than the mech's skim depth, or lie on ground steeper than the drop's limit. The same order SHALL always
give the same site.

#### Scenario: Bought in a dense city block
- **WHEN** the pilot stands among towers and buys a mech
- **THEN** the site is clear of every building's footprint by the mech's footprint and the clearance, on a street or
  plaza wide enough to take it

#### Scenario: Bought on open ground
- **WHEN** the pilot stands on flat open ground and buys a mech
- **THEN** the site is between 40 and 150 m from the pilot

#### Scenario: No good ground near
- **WHEN** nothing within 600 m of the pilot is good ground
- **THEN** the mech goes to a free hangar bay instead, as before

### Requirement: The drop lands on its feet, off the pilot, and can be boarded

The dropped mech SHALL come down at its site on its feet through the mech's own landing, SHALL not land on the pilot
(the site moves aside while the pilot stands in it), and SHALL then stand as an owned mech that the pilot boards by
walking up to it. Saving SHALL keep it where it landed.

#### Scenario: The pilot walks into the site
- **WHEN** the pilot stands at the site while the mech falls
- **THEN** the site moves to good ground just clear of the pilot and the mech lands there

#### Scenario: Continue after the drop
- **WHEN** the career is saved after the mech landed and continued
- **THEN** the mech stands where it landed and the pilot can walk up to it and board it

### Requirement: The shop shows the numbers the game flies the mech by

The shop's sheet SHALL read each mech's height, mass, speed, armour and loadout from the data the game flies it by.

#### Scenario: The Harrow's sheet
- **WHEN** the pilot opens the Harrow's sheet
- **THEN** its height and mass are those in `data/mechs/harrow.ron`
