## ADDED Requirements

### Requirement: Every roster mech has its own cockpit interior

Each roster mech's data SHALL name an interior no other roster mech names, and each interior SHALL be built from a
block-out layout in `build_cockpit_blockouts.py`.

#### Scenario: The Driller and the Foreman
- **WHEN** the pilot sits in the Driller and then in the Foreman
- **THEN** the Driller's view is the Octagon's two rows of glass and the Foreman's is the Panorama's one wide screen

#### Scenario: A second mech on the same interior
- **WHEN** a roster mech's data names an interior another roster mech already names
- **THEN** the roster's data test fails, naming both mechs

### Requirement: Every seam between screen rows is clear of the eye line

In every built layout, the rail between two screen rows SHALL sit at least 15 degrees above the eye line at the front
screen's depth, or behind the dash's glare shield.

#### Scenario: The Twin tier's rail
- **WHEN** the pilot sits in the Sapper's Twin tier and looks straight ahead
- **THEN** the rail between its two rings is above the top of the central 30 degrees of the view
