## ADDED Requirements

### Requirement: A bed starts without decoding on the main thread
An ambience bed SHALL start at its chosen point in the loop without decoding the skipped audio on the main thread; it
is silent while it catches up and then plays exactly the samples from that point, looping from the recording's
beginning.

#### Scenario: A bed starts 0.5 s in
- **WHEN** a bed starts 0.5 s into its recording
- **THEN** it gives silence for a sixteenth of the skipped samples, then the same samples Bevy's start position gave
  (test `a_bed_plays_from_its_start_after_a_silent_catch_up_as_bevys_start_position_did`)
