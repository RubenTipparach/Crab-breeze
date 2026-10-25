## ADDED Requirements

### Requirement: A purchase the money or the bays cannot cover changes nothing

Buying a mech SHALL charge its price through the career's ledger and park it in a free bay, and a purchase short of
money or with no free bay SHALL be refused with nothing charged and nothing parked.

#### Scenario: Short of money
- **WHEN** the pilot has 1 000 000 credits and buys the Harrow at 1 200 000
- **THEN** the shop says SHORT 200 000 and the money and the bays are unchanged

#### Scenario: Bought in the field
- **WHEN** the pilot is away from every base and buys the Mule with money to spare
- **THEN** the money falls by the Mule's price and the Mule waits in the lowest free bay of the nearest base with one

### Requirement: The shop shows the numbers the game flies the mech by

The shop's sheet SHALL read each mech's height, mass, speed, armour and loadout from the data the game flies it by.

#### Scenario: The Harrow's sheet
- **WHEN** the pilot opens the Harrow's sheet
- **THEN** its height and mass are those in `data/mechs/harrow.ron`
