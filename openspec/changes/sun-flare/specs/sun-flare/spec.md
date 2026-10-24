## ADDED Requirements

### Requirement: The sun glows
The pilot's eyes camera SHALL bloom its image with Bevy's `Bloom`, its settings read from `data/world/sun_flare.ron`, unless `CB_NO_BLOOM=1` is set. The cockpit camera SHALL NOT bloom, and Bevy UI SHALL NOT be bloomed.

#### Scenario: Looking at the sun
- **WHEN** the pilot looks toward the sun in clear air
- **THEN** the sun disk and the sky round it glow softly, while the HUD's text and lines stay sharp

### Requirement: A lens flare from the sun
The pilot's eyes SHALL show a lens flare when the sun is on screen: a warm core and glow at the sun, a starburst of spikes, an anamorphic horizontal streak, a halo ring and a chain of tinted ghosts with chromatic fringes on the line from the sun through the centre of the view, every colour and size read from `data/world/sun_flare.ron`, unless `CB_NO_FLARE=1` is set.

#### Scenario: The sun in view
- **WHEN** the sun is in view and nothing hides it
- **THEN** the flare is drawn, its ghosts on the line from the sun through the centre of the view

#### Scenario: The sun behind the pilot
- **WHEN** the sun is behind the pilot's eye
- **THEN** no flare is drawn and its pass does not run

### Requirement: The flare fades when the sun is hidden
The flare SHALL fade by the share of 17 depth samples round the sun that are not sky, on the world's depth and on the cockpit camera's depth, so that a building, the mech, terrain or the cockpit's frame hiding the sun fades it out, and an edge crossing the sun fades it in steps no larger than 2/17.

#### Scenario: Behind a building
- **WHEN** the pilot walks until a building covers the sun
- **THEN** the flare fades out as the building's edge crosses the sun, without a pop

#### Scenario: Behind the cockpit's frame
- **WHEN** the seated pilot looks so that a pillar of the screen wall covers the sun
- **THEN** the flare fades out, and returns as the sun comes back into a screen

### Requirement: The flare fades off screen, in cloud and under water
The flare SHALL fade smoothly to nothing as the sun moves past the edge of the view by `offscreen_margin_ndc`, and SHALL fade out while the eye is in cloud as thick as `cloud_full_fade` or under water, following the eye with the time constant `fade_s`.

#### Scenario: Turning away from the sun
- **WHEN** the pilot turns until the sun leaves the view
- **THEN** the ghosts linger briefly past the edge and fade, without a cut

#### Scenario: Diving
- **WHEN** the pilot sinks under a lake while looking toward the sun
- **THEN** the flare fades out within a fraction of a second

### Requirement: The flare's strength follows the sun and the exposure
The flare's strength SHALL be the sun's illuminance times the camera's exposure times `gain`, times the light's colour.

#### Scenario: A brighter exposure
- **WHEN** the camera's exposure doubles
- **THEN** the flare is twice as strong

### Requirement: The flare never moves the view
The flare SHALL only add light to the image; it SHALL NOT read or change any camera's transform.

#### Scenario: Looking round with the flare on
- **WHEN** the pilot looks round with the sun in view
- **THEN** the view turns only with the player's input
