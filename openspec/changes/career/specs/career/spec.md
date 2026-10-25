## ADDED Requirements

### Requirement: Money moves only through the ledger

The career SHALL change the pilot's money only through its pay and charge functions, and a charge
larger than the money SHALL change nothing.

#### Scenario: A repair you cannot afford
- **WHEN** the pilot has 50 000 credits and a repair costs 80 000
- **THEN** the charge is refused, the money stays 50 000 and the reply says 30 000 is missing

### Requirement: Owned mechs keep their damage

An owned mech SHALL keep its damage, part by part, between sorties and across a save and load.

#### Scenario: Landing hurt
- **WHEN** the pilot lands at a base with the left arm lost and saves
- **THEN** after loading, that mech's left arm is still lost until it is repaired
