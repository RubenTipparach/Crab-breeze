## ADDED Requirements

### Requirement: Bursts come in a few sizes, all built before play
A particle burst SHALL spawn a whole number of steps of a quarter of its layer's count, and every size of every recipe SHALL be played behind the loading screen, so no effect is built for the first time during play.

#### Scenario: Footsteps at changing speed
- **WHEN** the mech walks and skates at every speed from standing to its top speed
- **THEN** each footstep burst is one of at most nine sizes, each built before `GAME_READY`
