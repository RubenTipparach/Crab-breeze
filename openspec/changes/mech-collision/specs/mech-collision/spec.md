## ADDED Requirements

### Requirement: A mech is solid to the pilot on foot

The pilot on foot SHALL be pushed out of every mech's body, posed as the mech stands that step, by the shortest way, as
out of a building's wall; the pilot SHALL never stand inside a mech's plating.

#### Scenario: Walking into a kneeling mech's foot
- **WHEN** the pilot walks straight at the foot of a mech kneeling to be boarded
- **THEN** the pilot stops at the foot's face and the view never enters it

#### Scenario: Room under a standing mech
- **WHEN** the pilot walks between the legs of a standing mech where nothing is lower than 1.9 m over the ground
- **THEN** the pilot walks on, unblocked

### Requirement: Every mech can still be boarded

In every roster mech's kneel, the pilot's body at the rope's foot SHALL touch none of the mech's boxes.

#### Scenario: Reaching the rope
- **WHEN** a roster mech kneels and the pilot walks to the rope's foot
- **THEN** the pilot is not pushed away from it and can step in

### Requirement: A mech's body is measured off its model

Each roster mech's boxes SHALL be generated from its glb by `tools/mechs/gen_mech_data.py`, and `cb-tools validate`
SHALL refuse a mech file whose boxes no longer match its glb.

#### Scenario: A rebuilt mech
- **WHEN** a mech's glb is rebuilt with a bigger foot and its data is not regenerated
- **THEN** `cb-tools validate` fails, naming the mech file and the bone
